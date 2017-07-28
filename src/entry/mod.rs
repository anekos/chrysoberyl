
use std::cmp::{PartialEq, PartialOrd, Ord, Ordering};
use std::hash::{Hash, Hasher};
use std::io;
use std::ops;
use std::path::{PathBuf, Path};
use std::rc::Rc;
use std::slice;
use std::sync::Arc;

use natord;

use archive::ArchiveEntry;
use entry::filter::expression::Expr as FilterExpr;
use file_extension::{is_valid_image_filename};
use filterable_vec::FilterableVec;
use utils::path_to_str;

pub mod image;
pub mod filter;
pub mod info;

use self::info::EntryInfo;



pub struct EntryContainer {
    serial: Serial,
    entries: FilterableVec<Entry>,
}

#[derive(Clone, Copy, PartialEq)]
struct Serial(usize);

#[derive(Clone)]
pub struct Entry {
    serial: Serial,
    pub key: Key,
    pub content: EntryContent,
    pub meta: Option<Meta>,
    pub info: info::EntryInfo,
    pub url: Option<Arc<String>>,
}

#[derive(Clone)]
pub enum EntryContent {
    Image(PathBuf),
    Archive(Arc<PathBuf>, ArchiveEntry),
    Pdf(Arc<PathBuf>, usize),
}

pub type Meta = Arc<Vec<MetaEntry>>;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MetaEntry {
    pub key: String,
    pub value: String
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SearchKey {
    pub path: String,
    pub index: Option<usize>
}

pub type Key = (EntryType, String, usize);

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum EntryType {
    Invalid,
    PDF,
    Image,
    Archive,
}


impl Entry {
    fn new(serial: Serial, content: EntryContent, meta: Option<Meta>, url: Option<String>) -> Entry {
        let key = content.key(url.clone());

        let info = EntryInfo::new(&content, &key.1, key.2);

        Entry {
            serial: serial,
            key: key,
            content: content,
            meta: meta,
            info: info,
            url: url.map(Arc::new),
        }
    }

    fn new_local(serial: Serial, content: EntryContent, meta: Option<Meta>) -> Entry {
        Entry::new(serial, content, meta, None)
    }

    pub fn archive_name(&self) -> &str {
        &self.key.1
    }
}

impl Ord for Entry {
    fn cmp(&self, other: &Entry) -> Ordering {
        compare_key(&self.key, &other.key)
    }
}

impl PartialOrd for Entry {
    fn partial_cmp(&self, other: &Entry) -> Option<Ordering> {
        Some(compare_key(&self.key, &other.key))
    }
}

impl Eq for Entry {}

impl PartialEq for Entry {
    fn eq(&self, other: &Entry) -> bool {
        self.key.eq(&other.key)
    }
}

impl Hash for Entry {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.key.hash(state);
    }
}

impl EntryContent {
    fn key(&self, url: Option<String>) -> Key {
        use self::EntryContent::*;

        match *self {
            Image(ref path) =>
                (EntryType::Image,
                 url.unwrap_or_else(|| path_to_str(path).to_owned()),
                 1),
            Archive(ref path, ref entry) =>
                (EntryType::Archive,
                 url.unwrap_or_else(|| path_to_str(&**path).to_owned()),
                 entry.index),
            Pdf(ref path, index) =>
                (EntryType::PDF,
                 url.unwrap_or_else(|| path_to_str(&**path).to_owned()),
                 index),
        }
    }
}


impl EntryContainer {
    pub fn new() -> EntryContainer {
        EntryContainer {
            serial: Serial(0),
            entries: FilterableVec::new(),
        }
    }

    fn new_serial(&mut self) -> Serial {
        self.serial.0 += 1;
        self.serial
    }

    fn new_serials(&mut self, n: usize) -> Serial {
        let result = self.serial + 1;
        self.serial.0 += n;
        result
    }

    pub fn iter(&self) -> slice::Iter<Rc<Entry>> {
        self.entries.iter()
    }

    pub fn clear(&mut self) {
        self.entries.clear();
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn nth(&self, index: usize) -> Option<Entry> {
        self.entries.get(index).map(|it: &Rc<Entry>| {
            (**it).clone()
        })
    }

    pub fn to_displays(&self) -> Vec<String> {
        self.entries.iter().map(|it: &Rc<Entry>| (**it).display_path()).collect()
    }

    pub fn validate_nth(&mut self, index: usize, expr: FilterExpr) -> Option<bool> {
        self.entries.validate_nth(index, Box::new(move |ref mut entry| expr.evaluate(entry)))
    }

    pub fn expand(&mut self, center: Option<(PathBuf, usize, Entry)>, dir: Option<PathBuf>, n: u8, recursive: u8) -> bool {
        let result =
            if let Some((file, index, current_entry)) = center {
                let dir = n_parents(file.clone(), n);
                expand(&dir.to_path_buf(), recursive).ok().and_then(|middle| {
                    let serial = self.new_serials(middle.len());

                    let mut middle: Vec<Rc<Entry>> = middle.into_iter().enumerate().map(|(index, it)| {
                        Entry::new_local(serial + index, EntryContent::Image(it), current_entry.meta.clone())
                    }).filter(|entry| {
                        current_entry == *entry || (!self.is_duplicated(entry) && self.is_valid_image(entry))
                    }).map(Rc::new).collect();

                    middle.sort();

                    let (left, right) = self.entries.split_at(index);

                    let mut result = vec![];
                    result.extend_from_slice(left);
                    result.extend_from_slice(middle.as_slice());
                    result.extend_from_slice(&right[1..]);

                    Some(result)
                })
            } else if let Some(dir) = dir {
                let dir = n_parents(dir, n - 1);
                expand(&dir.to_path_buf(), recursive).ok().map(|files| {
                    let serial = self.new_serials(files.len());

                    let mut result = self.entries.clone_filtered();
                    let mut tail: Vec<Rc<Entry>> = files.into_iter().enumerate().map(|(index, it)| {
                        Entry::new_local(serial + index, EntryContent::Image(it), None)
                    }).filter(|entry| {
                        !self.is_duplicated(entry) && self.is_valid_image(entry)
                    }).map(Rc::new).collect();
                    tail.sort();
                    result.extend_from_slice(tail.as_slice());
                    result
                })
            } else {
                None
            };

        if let Some(expanded) = result {
            self.entries.clear();
            self.entries.extend_from_slice(expanded.as_slice());
            true
        } else {
            false
        }
    }

    pub fn shuffle(&mut self, current_index: Option<usize>) -> Option<usize> {
        let serial_before = current_index.and_then(|idx| self.nth(idx).map(|it| it.serial));
        self.entries.shuffle();
        serial_before.and_then(|it| self.search_by_serial(it))
    }

    pub fn sort(&mut self, current_index: Option<usize>) -> Option<usize> {
        self.entries.sort(current_index)
    }

    pub fn find_next_archive(&self, current: Option<(Entry, usize)>, mut count: usize) -> Option<usize> {
        current.map(|(entry, base_index)| {
            let mut current_archive = entry.archive_name().to_owned();
            for (index, it) in self.entries.iter().enumerate().skip(base_index + 1) {
                if it.archive_name() != current_archive {
                    if count == 1 {
                        return Some(index)
                    } else {
                        count -= 1;
                        current_archive = it.archive_name().to_owned();
                    }
                }
            }
            None
        }).unwrap_or_else(|| {
            if self.len() == 0 {
                None
            } else {
                Some(1)
            }
        })
    }

    pub fn find_nth_archive(&self, mut count: usize, reverse: bool) -> Option<usize> {
        let len = self.len();

        if len == 0 {
            return None;
        } else if !reverse {
            return self.find_next_archive(None, count);
        }

        self.entries.get(len - 1).map(|entry| {
            let mut previous_archive = entry.archive_name();
            let mut previous_index = len - 1;
            for (index, it) in self.entries.iter().enumerate().rev() {
                if it.archive_name() != previous_archive {
                    if count == 1 {
                        break;
                    }
                    count -= 1;
                    previous_archive = it.archive_name();
                } else {
                    previous_index = index;
                }
            }
            previous_index
        })
    }

    pub fn find_previous_archive(&self, current: Option<(Entry, usize)>, mut count: usize) -> Option<usize> {
        current.map(|(entry, base_index)| {
            let current_archive = entry.archive_name().to_owned();
            let mut previous_archive: Option<&str> = None;
            let mut previous_index = None;
            for (index, it) in self.entries.iter().enumerate().rev().skip(self.entries.len() - base_index + 1) {
                if let Some(prev) = previous_archive {
                    if it.archive_name() == prev {
                        previous_index = Some(index)
                    } else if count == 1 {
                        break;
                    } else {
                        count -= 1;
                        previous_archive = Some(it.archive_name());
                    }
                } else if it.archive_name() != current_archive {
                    previous_archive = Some(it.archive_name())
                }
            }
            previous_index
        }).unwrap_or_else(|| {
            if self.len() == 0 {
                None
            } else {
                Some(1)
            }
        })
    }

    pub fn get_entry_index(&self, entry: &Entry) -> Option<usize> {
        self.entries.get_index(entry)
    }

    fn push_entry(&mut self, entry: Entry, force: bool) {
        let entry = Rc::new(entry);

        if self.is_valid_image(&entry) && (force || !self.is_duplicated(&entry)) {
            self.entries.push(entry);
        }
    }

    pub fn push_archive_entry(&mut self, archive_path: &PathBuf, entry: &ArchiveEntry, meta: Option<Meta>, force: bool, url: Option<String>) {
        let serial = self.new_serial();
        self.push_entry(
            Entry::new(
                serial,
                EntryContent::Archive(Arc::new(archive_path.clone()), entry.clone()),
                meta,
                url),
            force);
    }

    pub fn push_pdf_entry(&mut self, pdf_path: Arc<PathBuf>, index: usize, meta: Option<Meta>, force: bool, url: Option<String>) {
        let content = EntryContent::Pdf(pdf_path.clone(), index);
        let serial = self.new_serial();
        self.push_entry(Entry::new(serial, content, meta, url), force);
    }

    pub fn search(&self, key: &SearchKey) -> Option<usize> {
        self.entries.iter().position(|it| key.matches(it))
    }

    fn search_by_serial(&self, serial: Serial) -> Option<usize> {
        self.entries.iter().position(|it| it.serial == serial)
    }

    pub fn push_image(&mut self, file: &PathBuf, meta: Option<Meta>, force: bool, expand_level: Option<u8>, url: Option<String>) {
        if_let_some!(file = file.canonicalize().ok(), {
            puts_error!("at" => "push_image", "reason" => o!("Failed to canonicalize"), "for" => path_to_str(&file));
        });

        if let Some(expand_level) = expand_level {
            if let Some(dir) = file.parent() {
                match expand(dir, expand_level) {
                    Ok(files) => {
                        for file in files {
                            self.push_image(&file, meta.clone(), force, None, None);
                        }
                    },
                    Err(err) => {
                        puts_error!("at" => "push_image", "reason" => s!(err), "for" => path_to_str(&file));
                        return;
                    }
                }
            }
        }

        let serial = self.new_serial();
        self.push_entry(
            Entry::new(serial, EntryContent::Image(file), meta, url),
            force);
    }

    pub fn push_directory(&mut self, dir: &PathBuf, meta: Option<Meta>, force: bool) {
        through!([expanded = expand(dir, <u8>::max_value())] {
            let mut expanded = expanded;
            expanded.sort_by(|a, b| natord::compare(path_to_str(a), path_to_str(b)));
            for file in expanded {
                self.push_image(&file, meta.clone(), force, None, None);
            }
        });
    }

    fn is_duplicated(&self, entry: &Entry) -> bool {
        self.entries.contains(entry)
    }

    fn is_valid_image(&self, entry: &Entry) -> bool {
        use self::EntryContent::*;

        match (*entry).content {
            Image(ref path) => is_valid_image_filename(path),
            Archive(_, _) | Pdf(_,  _) => true, // FIXME archive
        }
    }

    pub fn update_filter(&mut self, dynamic: bool, current_index: Option<usize>, pred: Option<Box<FnMut(&mut Entry) -> bool>>) -> Option<usize> {
        self.entries.update_filter(dynamic, current_index, pred)
    }
}


impl Entry {
    pub fn display_path(&self) -> String {
        if let Some(ref url) = self.url {
            (**url).clone()
        } else {
            self.key.1.clone()
        }
    }
}


pub fn new_opt_meta(entries: Vec<MetaEntry>) -> Option<Meta> {
    if entries.is_empty() {
        None
    } else {
        Some(Arc::new(entries))
    }
}


impl MetaEntry {
    pub fn new_without_value(key: String) -> MetaEntry {
        MetaEntry { key: key, value: o!("true") }
    }
}


impl SearchKey {
    pub fn matches(&self, entry: &Entry) -> bool {
        if let Some(index) = self.index {
            entry.key.1 == self.path && entry.key.2 == index
        } else {
            entry.key.1 == self.path
        }
    }
}


impl EntryType {
    pub fn is_container(&self) -> bool {
        use self::EntryType::*;

        match *self {
            PDF | Archive => true,
            _ => false,
        }
    }
}


impl ops::Add<usize> for Serial {
    type Output = Self;

    fn add(self, n: usize) -> Self {
        Serial(n + self.0)
    }
}


fn n_parents(path: PathBuf, n: u8) -> PathBuf {
    if n > 0 {
        if let Some(parent) = path.clone().parent() {
            return n_parents(parent.to_path_buf(), n - 1);
        }
    }

    path
}

fn expand(dir: &Path, recursive: u8) -> Result<Vec<PathBuf>, io::Error> {
    let mut result = vec![];

    through!([dir = dir.read_dir()] {
        for entry in dir {
            through!([entry = entry] {
                let path = entry.path();
                if path.is_file() && is_valid_image_filename(&path) {
                    result.push(path)
                } else if recursive > 0 && path.is_dir() {
                    through!([expanded = expand(&path, recursive - 1)] {
                        result.extend(expanded)
                    });
                }
            })
        }
    });

    Ok(result)
}

fn compare_key(a: &Key, b: &Key) -> Ordering {
    let name = natord::compare(&a.1, &b.1);
    if name == Ordering::Equal {
        a.2.cmp(&b.2)
    } else {
        name
    }
}
