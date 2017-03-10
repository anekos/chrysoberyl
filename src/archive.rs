
use std::path::Path;
use std::rc::Rc;
use libarchive::reader::Builder;
use libarchive::archive::{ReadFilter, ReadFormat, Entry};
use libarchive::reader::Reader;



pub struct ArchiveEntry {
    pub name: String,
    pub content: Rc<Vec<u8>>
}



pub fn read_entries<T: AsRef<Path>>(path: T) -> Vec<ArchiveEntry> {
    let mut result = Vec::new();

    let mut builder = Builder::new();
    builder.support_format(ReadFormat::All).ok();
    builder.support_filter(ReadFilter::All).ok();

    let mut reader = builder.open_file(path).unwrap();

    while let Some(ref name) = reader.next_header().map(|it| it.pathname().to_owned()) {
        let mut content = vec![];
        loop {
            if let Ok(block) = reader.read_block() {
                if let Some(block) = block {
                    content.extend_from_slice(block);
                    continue;
                } else if !content.is_empty() {
                    result.push(ArchiveEntry {
                        name: name.to_owned(),
                        content: Rc::new(content)
                    });
                }
            }
            break;
        }
    }

    println!("t7");
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
