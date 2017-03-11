
use std::path::Path;
use std::rc::Rc;
use libarchive::reader::Builder;
use libarchive::archive::{ReadFilter, ReadFormat, Entry};
use libarchive::reader::Reader;
use encoding::types::EncodingRef;



pub struct ArchiveEntry {
    pub name: String,
    pub content: Rc<Vec<u8>>
}



pub fn read_entries<T: AsRef<Path>>(path: T, encodings: &Vec<EncodingRef>) -> Vec<ArchiveEntry> {
    let mut result = Vec::new();

    let mut builder = Builder::new();
    builder.support_format(ReadFormat::All).ok();
    builder.support_filter(ReadFilter::All).ok();

    let mut reader = builder.open_file(path).unwrap();
    let mut index = 0;

    while let Some(ref name) = reader.next_header().map(|entry| get_filename(entry, index, encodings)) {
        let mut content = vec![];
        loop {
            if let Ok(block) = reader.read_block() {
                if let Some(block) = block {
                    content.extend_from_slice(block);
                    continue;
                } else if !content.is_empty() {
                    index +=1;
                    result.push(ArchiveEntry {
                        name: name.to_owned(),
                        content: Rc::new(content)
                    });
                }
            }
            break;
        }
    }

    result
}


fn get_filename(entry: &Entry, index: usize, encodings: &Vec<EncodingRef>) -> String {
    use libarchive3_sys::ffi;
    use std::ffi::CStr;
    use encoding::Encoding;
    use encoding::DecoderTrap::{Strict, Ignore};
    use encoding::all::ASCII;

    let c_str: &CStr = unsafe { CStr::from_ptr(ffi::archive_entry_pathname(entry.entry())) };
    let buf: &[u8] = c_str.to_bytes();

    for encoding in encodings {
        if let Ok(result) = encoding.decode(buf, Strict) {
            return result;
        }
    }

    ASCII.decode(buf, Ignore).unwrap_or_else(|_| format!("{:4}", index))
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
