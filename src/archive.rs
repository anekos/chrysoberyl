

use std::path::Path;
use std::fs::File;
use std::io::{Write, Read};
use libarchive::reader::{FileReader, Builder};
use libarchive::archive::{self, ReadFilter, ReadFormat, Entry};
use libarchive::reader::{self, Reader};



pub struct ArchiveEntry {
    name: String,
    index: usize,
    content: Vec<u8>
}



pub fn read_entries<T: AsRef<Path>>(path: T) -> Vec<ArchiveEntry> {
    let mut result = Vec::new();

    let mut builder = Builder::new();
    builder.support_format(ReadFormat::All).ok();
    builder.support_filter(ReadFilter::All).ok();

    let mut reader = builder.open_file(path).unwrap();
    let mut index = 0;

    while let Some(name) = reader.next_header().map(|entry| entry.pathname().to_owned()) {
        result.push(ArchiveEntry {
            name: name,
            index: index,
            content: reader.read_block().unwrap().unwrap().to_vec()
        });
        index += 1;
    }

    result
}


#[cfg(test)]#[test]
fn test_open_archive() {
    let mut builder = Builder::new();
    builder.support_format(ReadFormat::All).ok();
    builder.support_filter(ReadFilter::All).ok();

    let mut reader = builder.open_file("test/maru-sankaku-sikaku.zip").unwrap();
    reader.next_header();

    {
        let mut entry = reader.entry();
        assert_eq!(entry.pathname(), "maru.png");
    }

    {
        let in_zip = reader.read_block().unwrap().unwrap();
        let mut raw = vec![];
        File::open("test/raw/maru.png").unwrap().read_to_end(&mut raw).unwrap();
        assert_eq!(in_zip, raw.as_slice());
    }

    reader.next_header();

    {
        let mut entry = reader.entry();
        assert_eq!(entry.pathname(), "sankaku.png");
    }

    {
        let in_zip = reader.read_block().unwrap().unwrap();
        let mut raw = vec![];
        File::open("test/raw/sankaku.png").unwrap().read_to_end(&mut raw).unwrap();
        assert_eq!(in_zip, raw.as_slice());
    }

}
