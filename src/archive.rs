
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
use natord;

use operation::{Operation, QueuedOperation};
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


pub fn fetch_entries(path: &PathBuf, encodings: &[EncodingRef], tx: Sender<Operation>, mut sorting_buffer: SortingBuffer<QueuedOperation>, force: bool) {
    let from_index: HashMap<usize, (usize, String)> = {
        #[derive(Clone, Debug)]
        struct IndexWithName {
            index: usize,
            name: String,
        }

        impl Ord for IndexWithName {
            fn cmp(&self, other: &IndexWithName) -> Ordering {
                natord::compare(&self.name, &other.name)
            }
        }

        impl PartialOrd for IndexWithName {
            fn partial_cmp(&self, other: &IndexWithName) -> Option<Ordering> {
                Some(self.cmp(other))
            }
        }

        impl Eq for IndexWithName {}

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

    let ticket = sorting_buffer.reserve_n(from_index.len());

    spawn(clone_army!([path] move || {
        let mut builder = Builder::new();
        builder.support_format(ReadFormat::All).ok();
        builder.support_filter(ReadFilter::All).ok();

        let mut reader = builder.open_file(&path).unwrap();

        let mut buffer = sorting_buffer;
        let mut index = 0;

        while let Some(_) = reader.next_header() {
            if let Some(serial_name) = from_index.get(&index) {
                let (serial, ref name) = *serial_name;

                let mut content = vec![];
                loop {
                    if let Ok(block) = reader.read_block() {
                        if let Some(block) = block {
                            content.extend_from_slice(block);
                        } else if content.is_empty() {
                            panic!("Empty content in archive");
                        } else {
                            buffer.push(
                                ticket + serial,
                                QueuedOperation::PushArchiveEntry(
                                    path.clone(),
                                    ArchiveEntry { name: (*name).to_owned(), index: index, content: Arc::new(content) },
                                    force));
                            break;
                        }
                    } else {
                        buffer.skip(ticket + serial);
                        break;
                    }
                }

                if serial < 10 || serial % 20 == 0 {
                    tx.send(Operation::Pull).unwrap();
                }
            }

            index += 1;
        }

        tx.send(Operation::Pull).unwrap();
    }));
}


fn get_filename(entry: &Entry, index: usize, encodings: &[EncodingRef]) -> String {
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
    use std::fs::File;
    use std::io::Read;

    let mut builder = Builder::new();
    builder.support_format(ReadFormat::All).ok();
    builder.support_filter(ReadFilter::All).ok();

    let mut reader = builder.open_file("test-files/maru-sankaku-sikaku.zip").unwrap();
    reader.next_header();

    assert_eq!(reader.entry().pathname(), "maru.png");

    {
        let in_zip = reader.read_block().unwrap().unwrap();
        let mut raw = vec![];
        File::open("test-files/raw/maru.png").unwrap().read_to_end(&mut raw).unwrap();
        assert_eq!(in_zip, raw.as_slice());
    }

    reader.next_header();

    assert_eq!(reader.entry().pathname(), "sankaku.png");

    {
        let in_zip = reader.read_block().unwrap().unwrap();
        let mut raw = vec![];
        File::open("test-files/raw/sankaku.png").unwrap().read_to_end(&mut raw).unwrap();
        assert_eq!(in_zip, raw.as_slice());
    }

}
