use anyhow::Result;
use jvm_hprof::{
    heap_dump::{
        Class, FieldDescriptor, FieldType, FieldValue, Instance, PrimitiveArray,
        PrimitiveArrayType, SubRecord,
    },
    parse_hprof, HeapDumpSegment, Id, IdSize, RecordTag,
};
use rusqlite::{params, Connection, Statement, Transaction};
use std::{collections::HashMap, env, fs, time::SystemTime};
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    let path = args[1].as_str();
    let db_path = args[2].as_str();
    println!("Read: {path}");
    println!("Write: {db_path}");
    // Do first pass to map ids.
    println!("{} Map", now());
    let mapping = map_ids(fs::File::open(path)?)?;
    // And second pass to insert data.
    println!("{} Fill", now());
    let mut conn = Connection::open(db_path)?;
    build_schema(&conn)?;
    parse_records(fs::File::open(path)?, &mut conn, &mapping)?;
    // Index after insert to faster overall.
    println!("{} Index", now());
    conn.execute_batch(include_str!("index.sql"))?;
    println!("{} Vacuum", now());
    conn.execute_batch("vacuum")?;
    println!("{} Done", now());
    Ok(())
}

fn now() -> String {
    let dt: OffsetDateTime = SystemTime::now().into();
    dt.format(&Rfc3339).unwrap()
}

fn build_schema(conn: &Connection) -> Result<()> {
    conn.execute_batch(include_str!("schema.sql"))?;
    conn.execute_batch("pragma synchronous = off")?; // maybe faster
    Ok(())
}

struct Statements<'conn> {
    insert_class: Statement<'conn>,
    insert_field: Statement<'conn>,
    insert_field_value: Statement<'conn>,
    insert_header: Statement<'conn>,
    insert_hprof_obj_id: Statement<'conn>,
    insert_instance: Statement<'conn>,
    insert_name: Statement<'conn>,
    insert_obj_array: Statement<'conn>,
    insert_obj_array_item: Statement<'conn>,
    insert_primitive_array: Statement<'conn>,
}

struct ClassInfo {
    id: i64,
    fields: Vec<FieldDescriptor>,
    field_ids: Vec<i64>,
    instance_size: i64,
    name_id: i64,
    super_id: Option<Id>,
}

struct Mapping {
    class_ids: HashMap<Id, i64>,
    // Can get too many instances to be worth premapping.
    // instance_ids: HashMap<Id, i64>,
    name_ids: HashMap<Id, i64>,
}

fn ensure_id(map: &mut HashMap<Id, i64>, id: Id) {
    if !map.contains_key(&id) {
        map.insert(id, (map.len() + 1).try_into().unwrap());
    }
}

fn insert_id(map: &mut HashMap<Id, i64>, id: Id) {
    map.insert(id, (map.len() + 1).try_into().unwrap());
}

struct Context<'conn, 'mapping> {
    class_infos: HashMap<Id, ClassInfo>,
    id_size: IdSize,
    instance_id: i64,
    mapping: &'mapping Mapping,
    statements: Statements<'conn>,
    tx: &'conn Transaction<'conn>,
}

impl<'conn, 'mapping> Context<'conn, 'mapping> {
    fn next_instance_id(&mut self, obj_id: Id) -> Result<i64> {
        self.statements
            .insert_hprof_obj_id
            .execute(params![obj_id.id()])?;
        self.instance_id = self.tx.last_insert_rowid();
        Ok(self.instance_id)
    }
}

fn insert_class(statements: &mut Statements, mapping: &Mapping, info: &ClassInfo) -> Result<()> {
    statements.insert_class.execute(params![
        info.id,
        info.name_id,
        info.super_id.map(|sup| mapping.class_ids[&sup]),
        info.instance_size,
    ])?;
    Ok(())
}

fn map_ids(file: fs::File) -> Result<Mapping> {
    let memmap = unsafe { memmap::MmapOptions::new().map(&file) }.unwrap();
    let hprof = parse_hprof(&memmap[..]).unwrap();
    let mut mapping = Mapping {
        class_ids: HashMap::new(),
        // instance_ids: HashMap::new(),
        name_ids: HashMap::new(),
    };
    for record in hprof.records_iter() {
        let record = record.unwrap();
        match record.tag() {
            RecordTag::HeapDump | RecordTag::HeapDumpSegment => {
                let record = record.as_heap_dump_segment().unwrap().unwrap();
                for sub in record.sub_records() {
                    let sub = sub.unwrap();
                    match sub {
                        SubRecord::Class(class) => {
                            ensure_id(&mut mapping.class_ids, class.obj_id());
                        }
                        _ => {}
                    }
                }
            }
            RecordTag::LoadClass => {
                let class = record.as_load_class().unwrap().unwrap();
                ensure_id(&mut mapping.class_ids, class.class_obj_id());
            }
            RecordTag::Utf8 => {
                let name = record.as_utf_8().unwrap().unwrap();
                insert_id(&mut mapping.name_ids, name.name_id());
            }
            _ => {}
        }
    }
    println!("Classes: {}", mapping.class_ids.len());
    println!("Names: {}", mapping.name_ids.len());
    Ok(mapping)
}

fn parse_records(file: fs::File, conn: &mut Connection, mapping: &Mapping) -> Result<()> {
    let tx = conn.transaction()?;
    let mut statements = Statements {
        insert_class: tx.prepare(
            "insert into class(id, name_id, super_id, instance_size) values(?1, ?2, ?3, ?4)",
        )?,
        insert_field: tx
            .prepare("insert into field(class_id, name_id, ind, type_id) values(?1, ?2, ?3, ?4)")?,
        insert_field_value: tx
            .prepare("insert into field_value(instance_id, field_id, obj_id) values(?1, ?2, ?3)")?,
        insert_header: tx
            .prepare("insert into header(label, id_size, timestamp) values(?1, ?2, ?3)")?,
        insert_hprof_obj_id: tx.prepare("insert into hprof_obj_id(hprof_obj_id) values(?1)")?,
        insert_instance: tx.prepare("insert into instance(id, class_id) values(?1, ?2)")?,
        insert_name: tx.prepare("insert into name(text) values(?1)")?,
        insert_obj_array: tx
            .prepare("insert into obj_array(id, class_id, length) values(?1, ?2, ?3)")?,
        insert_obj_array_item: tx
            .prepare("insert into obj_array_item(array_id, ind, obj_id) values(?1, ?2, ?3)")?,
        insert_primitive_array: tx.prepare(
            "insert into primitive_array(id, type_id, length, text) values(?1, ?2, ?3, ?4)",
        )?,
    };
    let memmap = unsafe { memmap::MmapOptions::new().map(&file) }.unwrap();
    let hprof = parse_hprof(&memmap[..]).unwrap();
    let header = hprof.header();
    statements.insert_header.execute(params![
        header.label().unwrap(),
        match header.id_size() {
            IdSize::U32 => 4,
            IdSize::U64 => 8,
        },
        header.timestamp_millis(),
    ])?;
    let mut context = Context {
        class_infos: HashMap::new(),
        id_size: header.id_size(),
        instance_id: 0,
        mapping: &mapping,
        statements,
        tx: &tx,
    };
    // TODO Update object type size to id_size?
    // TODO Infer sizes using calculations?
    for record in hprof.records_iter() {
        let record = record.unwrap();
        match record.tag() {
            RecordTag::HeapDump | RecordTag::HeapDumpSegment => {
                parse_dump_records(
                    &record.as_heap_dump_segment().unwrap().unwrap(),
                    &mut context,
                )?;
            }
            RecordTag::LoadClass => {
                let class = record.as_load_class().unwrap().unwrap();
                match context.class_infos.get_mut(&class.class_obj_id()) {
                    Some(info) => {
                        if info.name_id == 0 {
                            info.name_id = mapping.name_ids[&class.class_name_id()];
                        }
                        if info.instance_size >= 0 {
                            insert_class(&mut context.statements, context.mapping, &info)?;
                        }
                    }
                    None => {
                        context.class_infos.insert(
                            class.class_obj_id(),
                            ClassInfo {
                                id: mapping.class_ids[&class.class_obj_id()],
                                fields: vec![],
                                field_ids: vec![],
                                instance_size: -1,
                                name_id: mapping.name_ids[&class.class_name_id()],
                                super_id: None,
                            },
                        );
                    }
                }
            }
            RecordTag::Utf8 => {
                let name = record.as_utf_8().unwrap().unwrap();
                context
                    .statements
                    .insert_name
                    .execute(params![name.text()])?;
            }
            _ => {}
        }
    }
    println!("Instances: {}", context.instance_id);
    drop(context);
    tx.commit()?;
    Ok(())
}

fn parse_dump_records(record: &HeapDumpSegment, context: &mut Context) -> Result<i32> {
    let mut count = 0;
    for sub in record.sub_records() {
        let sub = sub.unwrap();
        match sub {
            SubRecord::Class(class) => process_class(class, context)?,
            SubRecord::Instance(instance) => {
                count += 1;
                process_instance(instance, context)?;
            }
            SubRecord::ObjectArray(array) => {
                let id = context.next_instance_id(array.obj_id())?;
                let items = array.elements(context.id_size);
                context.statements.insert_obj_array.execute(params![
                    id,
                    context.class_infos[&array.array_class_obj_id()].id,
                    items.count(),
                ])?;
                for (i, obj_id) in array.elements(context.id_size).enumerate() {
                    context.statements.insert_obj_array_item.execute(params![
                        id,
                        i,
                        obj_id.unwrap().map(|o| o.id()),
                    ])?;
                }
            }
            SubRecord::PrimitiveArray(array) => {
                let id = context.next_instance_id(array.obj_id())?;
                context.statements.insert_primitive_array.execute(params![
                    id,
                    primitive_array_type_id(array.primitive_type()),
                    primitive_array_length(&array),
                    primitive_array_text(&array)?,
                ])?;
            }
            _ => {}
        }
    }
    Ok(count)
}

fn process_class(class: Class, context: &mut Context) -> Result<()> {
    match context.class_infos.get_mut(&class.obj_id()) {
        Some(info) => {
            info.super_id = class.super_class_obj_id();
            info.instance_size = class.instance_size_bytes().into();
            insert_class(&mut context.statements, context.mapping, &info)?;
        }
        None => {
            context.class_infos.insert(
                class.obj_id(),
                ClassInfo {
                    id: (context.class_infos.len() + 1).try_into().unwrap(),
                    fields: vec![],
                    field_ids: vec![],
                    instance_size: class.instance_size_bytes().into(),
                    name_id: -1,
                    super_id: None,
                },
            );
        }
    };
    let info = context.class_infos.get_mut(&class.obj_id()).unwrap();
    info.fields = class
        .instance_field_descriptors()
        .map(|field| field.unwrap())
        .collect();
    for (i, descriptor) in info.fields.iter().enumerate() {
        context.statements.insert_field.execute(params![
            info.id,
            context.mapping.name_ids[&descriptor.name_id()],
            i,
            field_type_id(descriptor.field_type()),
        ])?;
        info.field_ids.push(context.tx.last_insert_rowid());
    }
    Ok(())
}

fn process_instance(instance: Instance, context: &mut Context) -> Result<()> {
    let id = context.next_instance_id(instance.obj_id())?;
    context.statements.insert_instance.execute(params![
        id,
        context.class_infos[&instance.class_obj_id()].id,
    ])?;
    let mut class_info = Some(&context.class_infos[&instance.class_obj_id()]);
    let mut input = *instance.fields();
    while class_info.is_some() {
        let class = class_info.unwrap();
        for (i, field) in class.fields.iter().enumerate() {
            let (next, value) = field
                .field_type()
                .parse_value(input, context.id_size)
                .unwrap();
            input = next;
            let (_float, _int, obj) = field_value_tuple(value);
            if obj.is_some() {
                context.statements.insert_field_value.execute(params![
                    id,
                    class.field_ids[i],
                    obj,
                ])?;
            }
        }
        class_info = class.super_id.map(|id| &context.class_infos[&id]);
    }
    Ok(())
}

fn field_type_id(id: FieldType) -> i32 {
    match id {
        FieldType::ObjectId => 2,
        FieldType::Boolean => 4,
        FieldType::Char => 5,
        FieldType::Float => 6,
        FieldType::Double => 7,
        FieldType::Byte => 8,
        FieldType::Short => 9,
        FieldType::Int => 10,
        FieldType::Long => 11,
    }
}

fn field_value_tuple(value: FieldValue) -> (Option<f64>, Option<i64>, Option<i64>) {
    match value {
        FieldValue::ObjectId(obj) => (None, None, obj.map(|o| o.id().try_into().unwrap())),
        FieldValue::Boolean(b) => (None, Some(b.into()), None),
        FieldValue::Char(c) => (None, Some(c.into()), None),
        FieldValue::Float(f) => (Some(f.into()), None, None),
        FieldValue::Double(d) => (Some(d), None, None),
        FieldValue::Byte(b) => (None, Some(b.into()), None),
        FieldValue::Short(s) => (None, Some(s.into()), None),
        FieldValue::Int(i) => (None, Some(i.into()), None),
        FieldValue::Long(l) => (None, Some(l), None),
    }
}

fn primitive_array_type_id(id: PrimitiveArrayType) -> i32 {
    match id {
        PrimitiveArrayType::Boolean => 4,
        PrimitiveArrayType::Char => 5,
        PrimitiveArrayType::Float => 6,
        PrimitiveArrayType::Double => 7,
        PrimitiveArrayType::Byte => 8,
        PrimitiveArrayType::Short => 9,
        PrimitiveArrayType::Int => 10,
        PrimitiveArrayType::Long => 11,
    }
}

fn primitive_array_length(array: &PrimitiveArray) -> usize {
    match array.primitive_type() {
        PrimitiveArrayType::Boolean => array.booleans().unwrap().count(),
        PrimitiveArrayType::Char => array.chars().unwrap().count(),
        PrimitiveArrayType::Float => array.floats().unwrap().count(),
        PrimitiveArrayType::Double => array.doubles().unwrap().count(),
        PrimitiveArrayType::Byte => array.bytes().unwrap().count(),
        PrimitiveArrayType::Short => array.shorts().unwrap().count(),
        PrimitiveArrayType::Int => array.ints().unwrap().count(),
        PrimitiveArrayType::Long => array.longs().unwrap().count(),
    }
}

fn primitive_array_text(array: &PrimitiveArray) -> Result<Option<String>> {
    let text = match array.primitive_type() {
        PrimitiveArrayType::Byte => {
            let bytes: Vec<u8> = array
                .bytes()
                .unwrap()
                .map(|c| unsafe { std::mem::transmute(c.unwrap()) })
                .collect();
            match String::from_utf8(bytes) {
                Ok(text) => text,
                Err(_) => return Ok(None),
            }
        }
        PrimitiveArrayType::Char => {
            let chars: Vec<u16> = array.chars().unwrap().map(|c| c.unwrap()).collect();
            String::from_utf16(&chars)?
        }
        _ => return Ok(None),
    };
    Ok(Some(text))
}
