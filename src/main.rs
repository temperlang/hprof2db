use anyhow::Result;
use jvm_hprof::{
    heap_dump::{
        Class, FieldDescriptor, FieldType, FieldValue, PrimitiveArray, PrimitiveArrayType,
        SubRecord, Instance,
    },
    parse_hprof, HeapDumpSegment, Id, IdSize, RecordTag,
};
use rusqlite::{params, Connection, Statement};
use std::{collections::HashMap, env, fs};

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    let path = args[1].as_str();
    let db_path = args[2].as_str();
    println!("Read: {path}");
    println!("Write: {db_path}");
    let mut conn = Connection::open(db_path)?;
    build_schema(&conn)?;
    parse_records(fs::File::open(path)?, &mut conn)?;
    println!("Index"); // faster after insert
    conn.execute_batch(include_str!("index.sql"))?;
    Ok(())
}

fn build_schema(conn: &Connection) -> Result<()> {
    conn.execute_batch(include_str!("schema.sql"))?;
    conn.execute_batch("pragma synchronous = off")?; // maybe faster
    Ok(())
}

struct Statements<'conn> {
    insert_class: Statement<'conn>,
    insert_field_info: Statement<'conn>,
    insert_field_value: Statement<'conn>,
    insert_header: Statement<'conn>,
    insert_instance: Statement<'conn>,
    insert_load_class: Statement<'conn>,
    insert_name: Statement<'conn>,
    insert_obj_array: Statement<'conn>,
    insert_primitive_array: Statement<'conn>,
}

struct ClassInfo {
    id: Id,
    super_id: Option<Id>,
    fields: Vec<FieldDescriptor>,
}

struct Context<'conn> {
    class_infos: HashMap<Id, ClassInfo>,
    id_size: IdSize,
    statements: Statements<'conn>,
}

fn parse_records(file: fs::File, conn: &mut Connection) -> Result<()> {
    let tx = conn.transaction()?;
    let mut statements = Statements {
        insert_class: tx.prepare("insert into class(obj_id, stack_trace_serial, super_obj_id, instance_size) values(?1, ?2, ?3, ?4)")?,
        insert_field_info: tx.prepare("insert into field_info(class_obj_id, ind, name_id, type_id) values(?1, ?2, ?3, ?4)")?,
        insert_field_value: tx.prepare("insert into field_value(instance_obj_id, class_obj_id, ind, float, int, obj_id) values(?1, ?2, ?3, ?4, ?5, ?6)")?,
        insert_header: tx.prepare("insert into header(label, id_size, timestamp) values(?1, ?2, ?3)")?,
        insert_instance: tx.prepare("insert into instance(obj_id, stack_trace_serial, class_obj_id) values(?1, ?2, ?3)")?,
        insert_load_class: tx.prepare("insert into load_class(serial, obj_id, stack_trace_serial, name_id) values(?1, ?2, ?3, ?4)")?,
        insert_name: tx.prepare("insert into name(name_id, text) values(?1, ?2)")?,
        insert_obj_array: tx.prepare("insert into obj_array(obj_id, stack_trace_serial, class_obj_id, length) values(?1, ?2, ?3, ?4)")?,
        insert_primitive_array: tx.prepare("insert into primitive_array(obj_id, stack_trace_serial, type_id, length, text) values(?1, ?2, ?3, ?4, ?5)")?,
    };
    let memmap = unsafe { memmap::MmapOptions::new().map(&file) }.unwrap();
    let hprof = parse_hprof(&memmap[..]).unwrap();
    let mut record_count = 0;
    let mut dump_count = 0;
    let mut instance_count = 0;
    let mut class_count = 0;
    let mut name_count = 0;
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
        statements,
    };
    // TODO Update object type size to id_size?
    // TODO Infer sizes using calculations?
    for record in hprof.records_iter() {
        let record = record.unwrap();
        record_count += 1;
        match record.tag() {
            RecordTag::HeapDump | RecordTag::HeapDumpSegment => {
                dump_count += 1;
                instance_count += parse_dump_records(
                    &record.as_heap_dump_segment().unwrap().unwrap(),
                    &mut context,
                )?;
            }
            RecordTag::LoadClass => {
                class_count += 1;
                let class = record.as_load_class().unwrap().unwrap();
                context.statements.insert_load_class.execute(params![
                    class.class_serial().num(),
                    class.class_obj_id().id(),
                    class.stack_trace_serial().num(),
                    class.class_name_id().id(),
                ])?;
            }
            RecordTag::Utf8 => {
                name_count += 1;
                let name = record.as_utf_8().unwrap().unwrap();
                context
                    .statements
                    .insert_name
                    .execute(params![name.name_id().id(), name.text()])?;
            }
            _ => {}
        }
    }
    drop(context);
    tx.commit()?;
    println!("Records: {record_count}");
    println!("Classes: {class_count}");
    println!("Dumps: {dump_count}");
    println!("Names: {name_count}");
    println!("Instances: {instance_count}");
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
                // for thing in array.elements(id_size) {
                //     let thing = thing.unwrap().unwrap().id();
                // }
                context.statements.insert_obj_array.execute(params![
                    array.obj_id().id(),
                    array.stack_trace_serial().num(),
                    array.array_class_obj_id().id(),
                    array.elements(context.id_size).count(),
                ])?;
            }
            SubRecord::PrimitiveArray(array) => {
                context.statements.insert_primitive_array.execute(params![
                    array.obj_id().id(),
                    array.stack_trace_serial().num(),
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
    let class_info = ClassInfo {
        id: class.obj_id(),
        super_id: class.super_class_obj_id(),
        fields: class
            .instance_field_descriptors()
            .map(|field| field.unwrap())
            .collect(),
    };
    for (i, descriptor) in class_info.fields.iter().enumerate() {
        context.statements.insert_field_info.execute(params![
            class_info.id.id(),
            i,
            descriptor.name_id().id(),
            field_type_id(descriptor.field_type()),
        ])?;
        // TODO Duplicate supertype fields?
    }
    context.class_infos.insert(class.obj_id(), class_info);
    context.statements.insert_class.execute(params![
        class.obj_id().id(),
        class.stack_trace_serial().num(),
        class.super_class_obj_id().map(|sup| sup.id()),
        class.instance_size_bytes(),
    ])?;
    Ok(())
}

fn process_instance(instance: Instance, context: &mut Context) -> Result<()> {
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
            let (float, int, obj) = field_value_tuple(value);
            context.statements.insert_field_value.execute(params![
                instance.obj_id().id(),
                class.id.id(),
                i,
                float,
                int,
                obj,
            ])?;
        }
        class_info = class.super_id.map(|id| &context.class_infos[&id]);
    }
    context.statements.insert_instance.execute(params![
        instance.obj_id().id(),
        instance.stack_trace_serial().num(),
        instance.class_obj_id().id(),
    ])?;
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

fn field_value_tuple(value: FieldValue) -> (Option<f64>, Option<i64>, Option<u64>) {
    match value {
        FieldValue::ObjectId(obj) => (None, None, obj.map(|o| o.id())),
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
    if array.primitive_type() != PrimitiveArrayType::Char {
        return Ok(None)
    }
    let chars: Vec<u16> = array.chars().unwrap().map(|c| c.unwrap()).collect();
    Ok(Some(String::from_utf16(&chars)?))
}
