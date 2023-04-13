use anyhow::Result;
use jvm_hprof::{heap_dump::SubRecord, parse_hprof, HeapDumpSegment, RecordTag};
use rusqlite::{params, Connection, Transaction};
use std::{env, fs};

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    let path = args[1].as_str();
    let db_path = args[2].as_str();
    println!("Read: {path}");
    println!("Write: {path}");
    let mut conn = Connection::open(db_path)?;
    build_schema(&conn)?;
    parse_records(fs::File::open(path)?, &mut conn)?;
    Ok(())
}

fn build_schema(conn: &Connection) -> Result<()> {
    conn.execute_batch(include_str!("schema.sql"))?;
    Ok(())
}

fn parse_records(file: fs::File, conn: &mut Connection) -> Result<()> {
    let tx = conn.transaction()?;
    let memmap = unsafe { memmap::MmapOptions::new().map(&file) }.unwrap();
    let hprof = parse_hprof(&memmap[..]).unwrap();
    let mut record_count = 0;
    let mut dump_count = 0;
    let mut instance_count = 0;
    let mut class_count = 0;
    let mut name_count = 0;
    for record in hprof.records_iter() {
        let record = record.unwrap();
        record_count += 1;
        match record.tag() {
            RecordTag::HeapDumpSegment => {
                dump_count += 1;
                instance_count +=
                    parse_dump_records(&record.as_heap_dump_segment().unwrap().unwrap(), &tx)?;
            }
            RecordTag::LoadClass => {
                class_count += 1;
                let class = record.as_load_class().unwrap().unwrap();
                tx.execute(
                    "insert into load_class(serial, obj_id, stack_trace_serial, name_id) values(?1, ?2, ?3, ?4)",
                    params![
                        class.class_serial().num(),
                        class.class_obj_id().id(),
                        class.stack_trace_serial().num(),
                        class.class_name_id().id(),
                    ],
                )?;
            }
            RecordTag::Utf8 => {
                name_count += 1;
                let name = record.as_utf_8().unwrap().unwrap();
                tx.execute(
                    "insert into name(name_id, text) values(?1, ?2)",
                    params![name.name_id().id(), name.text(),],
                )?;
            }
            _ => {}
        }
    }
    tx.commit()?;
    println!("Records: {record_count}");
    println!("Classes: {class_count}");
    println!("Dumps: {dump_count}");
    println!("Names: {name_count}");
    println!("Instances: {instance_count}");
    Ok(())
}

fn parse_dump_records(record: &HeapDumpSegment, tx: &Transaction) -> Result<i32> {
    let mut count = 0;
    for sub in record.sub_records() {
        let sub = sub.unwrap();
        match sub {
            SubRecord::Class(class) => {
                tx.execute(
                    "insert into class(obj_id, stack_trace_serial, super_obj_id, instance_size) values(?1, ?2, ?3, ?4)",
                    params![
                        class.obj_id().id(),
                        class.stack_trace_serial().num(),
                        class.super_class_obj_id().map(|sup| sup.id()),
                        class.instance_size_bytes(),
                    ],
                )?;
            }
            SubRecord::Instance(instance) => {
                count += 1;
                tx.execute(
                    "insert into instance(obj_id, stack_trace_serial, class_obj_id) values(?1, ?2, ?3)",
                    params![
                        instance.obj_id().id(),
                        instance.stack_trace_serial().num(),
                        instance.class_obj_id().id(),
                    ],
                )?;
            }
            _ => {}
        }
    }
    Ok(count)
}
