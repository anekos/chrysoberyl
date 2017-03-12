
use std::cmp::Ordering;
use std::collections::{HashSet, HashMap};
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

    let mut candidates = Vec::new();

    let mut builder = Builder::new();
    builder.support_format(ReadFormat::All).ok();
    builder.support_filter(ReadFilter::All).ok();

    let mut reader = builder.open_file(&path).unwrap();
    let mut index = 0;
    let mut targets = HashSet::new();

    while let Some(entry) = reader.next_header() {
        let name = get_filename(entry, index, encodings);

        match entry.filetype() {
            FileType::RegularFile if is_valid_image_filename(&name) => {
                targets.insert(index);
                candidates.push(ArchiveEntry { name: name.to_owned(), index: index });
            }
            _ => ()
        }

        index += 1;

    }

    candidates.sort();

    let mut index_to_serial: HashMap<usize, usize> = HashMap::new();
    for (serial, candidate) in candidates.iter().enumerate() {
        index_to_serial.insert(candidate.index, serial);
    }

    spawn(clone_army!([path] move || {
        let mut builder = Builder::new();
        builder.support_format(ReadFormat::All).ok();
        builder.support_filter(ReadFilter::All).ok();

        let mut reader = builder.open_file(&path).unwrap();

        let mut buffer = SortingBuffer::new(0);
        let mut candidates = candidates.iter();
        let mut index = 0;

        while let Some(_) = reader.next_header() {
            if targets.contains(&index) {
                let candidate = candidates.next().unwrap();

                let mut content = vec![];
                loop {
                    if let Ok(block) = reader.read_block() {
                        if let Some(block) = block {
                            content.extend_from_slice(block);
                            continue;
                        } else if content.is_empty() {
                            panic!("Empty content in archive");
                        } else {
                            buffer.push(*index_to_serial.get(&candidate.index).unwrap(), (candidate, content));
                        }
                    } else {
                        buffer.skip(*index_to_serial.get(&candidate.index).unwrap());
                    }
                    break;
                }
            }

            while let Some((entry, buffer)) = buffer.pull() {
                tx.send(Operation::PushArchiveEntry(path.clone(), entry.clone(), Arc::new(buffer))).unwrap();
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
