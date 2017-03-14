
use std::cmp::Ordering;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::mpsc::Sender;
use std::thread::spawn;

use encoding::types::EncodingRef;
use libarchive::archive::{ReadFilter, ReadFormat, Entry, FileType};
use libarchive::reader::Builder;
use libarchive::reader::Reader;

use operation::Operation;
use sorting_buffer::SortingBuffer;
use validation::is_valid_image_filename;



#[derive(Eq, Clone, Debug)]
pub struct ArchiveEntry {
    pub index: usize,
    pub name: String,
    pub content: Arc<Vec<u8>>
}


impl Ord for ArchiveEntry {
    fn cmp(&self, other: &ArchiveEntry) -> Ordering {
        self.name.cmp(&other.name)
    }
}

impl PartialOrd for ArchiveEntry {
    fn partial_cmp(&self, other: &ArchiveEntry) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for ArchiveEntry {
    fn eq(&self, other: &ArchiveEntry) -> bool {
        self.name == other.name
    }
}

impl Hash for ArchiveEntry {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.name.hash(state);
    }
}


pub fn fetch_entries(path: &PathBuf, encodings: &Vec<EncodingRef>, tx: Sender<Operation>) {
    let from_index: HashMap<usize, (usize, String)> = {
        #[derive(Eq, Clone, Debug, PartialOrd)]
        struct IndexWithName {
            index: usize,
            name: String,
        }

        impl Ord for IndexWithName {
            fn cmp(&self, other: &IndexWithName) -> Ordering {
                self.name.cmp(&other.name)
            }
        }

        impl PartialEq for IndexWithName {
            fn eq(&self, other: &IndexWithName) -> bool {
                self.name == other.name
            }
        }

        let mut candidates = {
            let mut result = vec![];

            let mut builder = Builder::new();
            builder.support_format(ReadFormat::All).ok();
            builder.support_filter(ReadFilter::All).ok();

            let mut reader = builder.open_file(&path).unwrap();
            let mut index = 0;

            while let Some(entry) = reader.next_header() {
                let name = get_filename(entry, index, encodings);
                match entry.filetype() {
                    FileType::RegularFile if is_valid_image_filename(&name) => {
                        result.push(IndexWithName { index: index, name: name });
                    }
                    _ => ()
                }
                index += 1;
            }

            result
        };

        candidates.sort();

        let mut result = HashMap::new();
        for (serial, candidate) in candidates.iter().enumerate() {
            result.insert(candidate.index, (serial, candidate.name.clone()));
        }

        result
    };

    spawn(clone_army!([path] move || {
        let mut builder = Builder::new();
        builder.support_format(ReadFormat::All).ok();
        builder.support_filter(ReadFilter::All).ok();

        let mut reader = builder.open_file(&path).unwrap();

        let mut buffer = SortingBuffer::new(0);
        let mut index = 0;

        while let Some(_) = reader.next_header() {
            if let Some(serial_name) = from_index.get(&index) {
                let (serial, ref name) = *serial_name;

                let mut content = vec![];
                loop {
                    if let Ok(block) = reader.read_block() {
                        if let Some(block) = block {
                            content.extend_from_slice(block);
                            continue;
                        } else if content.is_empty() {
                            panic!("Empty content in archive");
                        } else {
                            buffer.push(serial, ArchiveEntry { name: (*name).to_owned(), index: index, content: Arc::new(content) });
                        }
                    } else {
                        buffer.skip(serial);
                    }
                    break;
                }
            }

            while let Some(entry) = buffer.pull() {
                tx.send(Operation::PushArchiveEntry(path.clone(), entry.clone())).unwrap();
            }

            index += 1;
        }
    }));
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
