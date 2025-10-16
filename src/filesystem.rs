use std::fs::File;
use std::io::prelude::*;
use std::path::PathBuf;

pub fn write_to_file(filename: &PathBuf, data: &[u8]) -> std::io::Result<()> {
    let mut f = File::create(filename)?;
    f.write_all(data)?;
    Ok(())
}

pub fn read_from_file(filename: &PathBuf) -> std::io::Result<Vec<u8>> {
    let mut f = File::open(filename)?;
    let mut data = vec![];
    f.read_to_end(&mut data)?;
    Ok(data)
}
