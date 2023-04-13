use anyhow::Result;
use jvm_hprof::{heap_dump::SubRecord, parse_hprof, HeapDumpSegment, RecordTag};
use rusqlite::{params, Connection};
use std::{env, fs};

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    let path = args[1].as_str();
    let db_path = args[2].as_str();
    println!("Read: {path}");
    println!("Write: {path}");
    let conn = Connection::open(db_path)?;
    build_schema(&conn)?;
    parse_records(fs::File::open(path)?, &conn)?;
    Ok(())
}

fn build_schema(conn: &Connection) -> Result<()> {
    conn.execute_batch(include_str!("schema.sql"))?;
    Ok(())
}

fn parse_records(file: fs::File, conn: &Connection) -> Result<()> {
    let memmap = unsafe { memmap::MmapOptions::new().map(&file) }.unwrap();
    let hprof = parse_hprof(&memmap[..]).unwrap();
    let mut record_count = 0;
    let mut dump_count = 0;
    let mut instance_count = 0;
    for record in hprof.records_iter() {
        let record = record.unwrap();
        record_count += 1;
        match record.tag() {
            RecordTag::HeapDumpSegment => {
                dump_count += 1;
                instance_count +=
                    parse_dump_records(&record.as_heap_dump_segment().unwrap().unwrap())
            }
            RecordTag::LoadClass => {
                let class = record.as_load_class().unwrap().unwrap();
                conn.execute(
                    "insert into class (id, serial, stack_trace_serial, name_id) values (?1, ?2, ?3, ?4)",
                    params![
                        class.class_obj_id().id(),
                        class.class_serial().num(),
                        class.stack_trace_serial().num(),
                        class.class_name_id().id(),
                    ],
                )?;
            }
            RecordTag::Utf8 => {
                let name = record.as_utf_8().unwrap().unwrap();
                conn.execute(
                    "insert into name (id, text) values (?1, ?2)",
                    params![name.name_id().id(), name.text(),],
                )?;
            }
            _ => {}
        }
    }
    println!("Records: {record_count}");
    println!("Dumps: {dump_count}");
    println!("Instances: {instance_count}");
    Ok(())
}

fn parse_dump_records(record: &HeapDumpSegment) -> i32 {
    let mut count = 0;
    for sub in record.sub_records() {
        let sub = sub.unwrap();
        match sub {
            SubRecord::Instance(instance) => {
                count += 1;
                instance.obj_id();
            }
            _ => {}
        }
    }
    count
}
