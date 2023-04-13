use anyhow::Result;
use jvm_hprof::{heap_dump::SubRecord, parse_hprof, HeapDumpSegment, RecordTag};
use rusqlite::Connection;
use std::{env, fs};

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    let path = args[1].as_str();
    println!("Reading: {path}");
    let conn = Connection::open_in_memory()?;
    build_schema(&conn)?;
    parse_records(fs::File::open(path)?);
    Ok(())
}

fn build_schema(conn: &Connection) -> Result<()> {
    conn.execute(
        "
        create table instance (
            id integer primary key,
            class_id integer
        )
        ",
        (),
    )?;
    Ok(())
}

fn parse_records(file: fs::File) {
    let memmap = unsafe { memmap::MmapOptions::new().map(&file) }.unwrap();
    let hprof = parse_hprof(&memmap[..]).unwrap();
    let mut record_count = 0;
    let mut dump_count = 0;
    let mut instance_count = 0;
    hprof.records_iter().map(|r| r.unwrap()).for_each(|record| {
        record_count += 1;
        match record.tag() {
            RecordTag::HeapDumpSegment => {
                dump_count += 1;
                instance_count +=
                    parse_dump_records(&record.as_heap_dump_segment().unwrap().unwrap())
            }
            _ => {}
        }
    });
    println!("Records: {record_count}");
    println!("Dumps: {dump_count}");
    println!("Instances: {instance_count}");
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
