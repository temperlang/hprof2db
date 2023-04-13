use jvm_hprof::parse_hprof;
use std::{env, fs, io};

fn main() -> io::Result<()> {
    let args: Vec<String> = env::args().collect();
    let path = args[1].as_str();
    println!("Hello, {path}!");
    count_records(fs::File::open(path)?);
    Ok(())
}

fn count_records(file: fs::File) {
    let memmap = unsafe { memmap::MmapOptions::new().map(&file) }.unwrap();
    let hprof = parse_hprof(&memmap[..]).unwrap();
    hprof
        .records_iter()
        .map(|r| r.unwrap())
        .for_each(|record| match record.tag() {
            _ => (),
        });
}
