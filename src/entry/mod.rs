
use std::cmp::{PartialEq, PartialOrd, Ord, Ordering};
use std::error;
use std::fs::File;
use std::hash::{Hash, Hasher};
use std::io::{self, Read};
use std::ops;
use std::path::{PathBuf, Path};
use std::slice;
use std::sync::Arc;

use natord;
use url::Url;

use app::info::AppInfo;
use archive::ArchiveEntry;
use entry::filter::expression::Expr as FilterExpr;
use errors::ChryError;
use file_extension::{is_valid_image_filename};
use filterable_vec::{FilterableVec, Pred, Compare};
use shorter::*;
use util::path::path_to_str;

pub mod image;
pub mod filter;
pub mod info;

use self::info::EntryInfo;



type FilterPred = Pred<Entry, AppInfo>;

pub struct EntryContainer {
    serial: Serial,
    entries: FilterableVec<Entry, AppInfo>,
}

#[derive(Clone, Copy, PartialEq)]
pub struct Serial(usize);

pub struct Entry {
    pub serial: Serial,
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
    Memory(Vec<u8>, String),
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

pub type Key = (EntryType, String, usize); /* usize = 0 origin page number */

#[derive(Clone, Debug, PartialEq, Eq, Hash, Copy)]
pub enum EntryType {
    Invalid,
    PDF,
    Image,
    Archive,
    Memory,
}


impl Entry {
    fn new(serial: Serial, content: EntryContent, meta: Option<Meta>, url: Option<String>) -> Entry {
        let key = content.key(url.clone());

        let info = EntryInfo::new(&content, &key.1, key.2 + 1);

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

    pub fn page_number(&self) -> usize {
        self.key.2 + 1
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
                 0),
            Archive(ref path, ref entry) =>
                (EntryType::Archive,
                 url.unwrap_or_else(|| path_to_str(&**path).to_owned()),
                 entry.index),
            Memory(_, ref hash) =>
                (EntryType::Memory,
                 url.unwrap_or_else(|| format!("{}.png", hash)),
                 0),
            Pdf(ref path, index) =>
                (EntryType::PDF,
                 url.unwrap_or_else(|| path_to_str(&**path).to_owned()),
                 index),
        }
    }

    pub fn local_file_path(&self) -> Option<PathBuf> {
        use self::EntryContent::*;

        match *self {
            Archive(ref path, _) | Pdf(ref path, _) =>
                Some(path.to_path_buf()),
            Image(ref path) =>
                Some(path.to_path_buf()),
            Memory(_, _) =>
                None
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

    pub fn iter(&self) -> slice::Iter<Arc<Entry>> {
        self.entries.iter()
    }

    pub fn clear(&mut self) {
        self.entries.clear();
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn real_len(&self) -> usize {
        self.entries.real_len()
    }

    pub fn nth(&self, index: usize) -> Option<Arc<Entry>> {
        self.entries.get(index)
    }

    pub fn validate_nth(&mut self, index: usize, expr: FilterExpr, app_info: &AppInfo) -> Option<bool> {
        let pred: FilterPred = Box::new(move |entry, app_info| expr.evaluate(entry, app_info));
        self.entries.validate_nth(index, app_info, &pred)
    }

    pub fn expand(&mut self, app_info: &AppInfo, center: Option<(PathBuf, usize, Arc<Entry>)>, dir: Option<PathBuf>, n: u8, recursive: u8) -> bool {
        let result =
            if let Some((file, index, current_entry)) = center {
                let dir = n_parents(file.clone(), n);
                expand(&dir.to_path_buf(), recursive).ok().and_then(|middle| {
                    let serial = self.new_serials(middle.len());

                    let mut middle: Vec<Arc<Entry>> = middle.into_iter().enumerate().map(|(index, it)| {
                        let serial = if it == file { current_entry.serial } else { serial + index };
                        Entry::new_local(serial, EntryContent::Image(it), current_entry.meta.clone())
                    }).filter(|entry| {
                        *current_entry == *entry || (!self.is_duplicated(entry) && self.is_valid_image(entry))
                    }).map(Arc::new).collect();

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
                    let mut tail: Vec<Arc<Entry>> = files.into_iter().enumerate().map(|(index, it)| {
                        Entry::new_local(serial + index, EntryContent::Image(it), None)
                    }).filter(|entry| {
                        !self.is_duplicated(entry) && self.is_valid_image(entry)
                    }).map(Arc::new).collect();
                    tail.sort();
                    result.extend_from_slice(tail.as_slice());
                    result
                })
            } else {
                None
            };

        if let Some(expanded) = result {
            self.entries.clear();
            self.entries.extend_from_slice(app_info, expanded.as_slice());
            self.entries.filter(app_info, None);
            true
        } else {
            false
        }
    }

    pub fn shuffle(&mut self, app_info: &AppInfo) {
        self.entries.shuffle(app_info)
    }

    pub fn sort(&mut self, app_info: &AppInfo) -> Option<usize> {
        self.entries.sort(app_info)
    }

    pub fn sort_by(&mut self, app_info: &AppInfo, compare: &Compare<Entry>) -> Option<usize> {
        self.entries.sort_by(app_info, compare)
    }

    pub fn find_page_in_archive(&self, current: usize, page_number: usize) -> Option<usize> {
        if_let_some!(base = self.entries.get(current), None);

        let len = self.entries.len();

        for i in (0..current).rev() {
            let it = self.entries.get(i).unwrap();
            if it.archive_name() == base.archive_name() {
                if it.page_number() == page_number {
                    return Some(i);
                }
            } else {
                break;
            }
        }

        for i in current..len {
            let it = self.entries.get(i).unwrap();
            if it.archive_name() == base.archive_name() {
                if it.page_number() == page_number {
                    return Some(i);
                }
            } else {
                break;
            }
        }
        None
    }

    pub fn find_next_archive(&self, current: Option<(Arc<Entry>, usize)>, mut count: usize) -> Option<usize> {
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

    pub fn find_previous_archive(&self, current: Option<(Arc<Entry>, usize)>, mut count: usize) -> Option<usize> {
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

    fn push_entry(&mut self, app_info: &AppInfo, entry: Entry, force: bool) {
        let entry = Arc::new(entry);

        if force || !self.is_duplicated(&entry) {
            self.entries.push(app_info, &entry);
        }
    }

    pub fn push_archive_entry(&mut self, app_info: &AppInfo, archive_path: &PathBuf, entry: &ArchiveEntry, meta: Option<Meta>, force: bool, url: Option<String>) {
        let serial = self.new_serial();
        self.push_entry(
            app_info,
            Entry::new(
                serial,
                EntryContent::Archive(Arc::new(archive_path.clone()), entry.clone()),
                meta,
                url),
            force);
    }

    pub fn push_pdf_entry(&mut self, app_info: &AppInfo, pdf_path: &Arc<PathBuf>, index: usize, meta: Option<Meta>, force: bool, url: Option<String>) {
        let content = EntryContent::Pdf(Arc::clone(pdf_path), index);
        let serial = self.new_serial();
        self.push_entry(app_info, Entry::new(serial, content, meta, url), force);
    }

    pub fn search(&self, key: &SearchKey) -> Option<usize> {
        self.entries.iter().position(|it| key.matches(it))
    }

    pub fn search_by_serial(&self, serial: Serial) -> Option<usize> {
        self.entries.iter().position(|it| it.serial == serial)
    }

    pub fn push_memory(&mut self, app_info: &AppInfo, content: Vec<u8>, meta: Option<Meta>, force: bool, url: Option<String>) -> Result<(), Box<error::Error>> {
        use sha2::{Sha256, Digest};

        let mut hasher = Sha256::default();

        hasher.input(&content);

        let mut hash = String::new();
        for b in hasher.result().as_ref() {
            hash.push_str(&format!("{:2x}", b));
        }

        let serial = self.new_serial();
        self.push_entry(
            app_info,
            Entry::new(serial, EntryContent::Memory(content, hash), meta, url),
            force);
        Ok(())
    }

    pub fn push_image(&mut self, app_info: &AppInfo, file: &PathBuf, meta: Option<Meta>, force: bool, expand_level: Option<u8>, url: Option<String>) -> Result<(), Box<error::Error>> {
        use std::os::unix::fs::FileTypeExt;

        if let Ok(metadata) = file.metadata() {
            if metadata.file_type().is_fifo() {
                let mut content = vec![];
                let mut file = File::open(file)?;
                file.read_to_end(&mut content)?;
                return self.push_memory(app_info, content, meta, force, url);
            }
        }

        let file = file.canonicalize().map_err(|_| ChryError::File("Could not canonicalize", d!(file)))?;

        if let Some(expand_level) = expand_level {
            if let Some(dir) = file.parent() {
                let files = expand(dir, expand_level)?;
                for file in files {
                    self.push_image(app_info, &file, meta.clone(), force, None, None)?;
                }
            }
        }

        let serial = self.new_serial();
        self.push_entry(
            app_info,
            Entry::new(serial, EntryContent::Image(file), meta, url),
            force);
        Ok(())
    }

    pub fn push_directory(&mut self, app_info: &AppInfo, dir: &PathBuf, meta: &Option<Meta>, force: bool) -> Result<(), Box<error::Error>> {
        through!([expanded = expand(dir, <u8>::max_value())] {
            let mut expanded = expanded;
            expanded.sort_by(|a, b| natord::compare(path_to_str(a), path_to_str(b)));
            for file in expanded {
                self.push_image(app_info, &file, meta.clone(), force, None, None)?;
            }
        });
        Ok(())
    }

    fn is_duplicated(&self, entry: &Entry) -> bool {
        self.entries.contains(entry)
    }

    fn is_valid_image(&self, entry: &Entry) -> bool {
        use self::EntryContent::*;

        match (*entry).content {
            Image(ref path) => is_valid_image_filename(path),
            Archive(_, _) | Pdf(_,  _) | Memory(_, _) => true, // FIXME archive
        }
    }

    pub fn update_filter(&mut self, app_info: &AppInfo, dynamic: bool, current_index: Option<usize>, pred: Option<FilterPred>) -> Option<usize> {
        self.entries.update_filter(app_info, dynamic, current_index, pred)
    }

    pub fn delete(&mut self, app_info: &AppInfo, current_index: Option<usize>, pred: FilterPred) -> Option<usize> {
        self.entries.delete(app_info, current_index, pred)
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

    pub fn abbrev_path(&self, max: usize) -> String {
        if let Some(ref url) = self.url {
            Url::parse(&**url).as_ref().map(|it| shorten_url(it, max)).unwrap_or_else(|_| (**url).clone())
        } else {
            shorten_path(&Path::new(&self.key.1), max)
        }
    }

    pub fn page_filename(&self) -> PathBuf {
        use self::EntryContent::*;

        fn gen<T: AsRef<Path>>(filename: &T, index: usize, fixed_ext: Option<&str>) -> String {
            let stem = filename.as_ref().file_stem().and_then(|it| it.to_str()).unwrap();
            let ext = fixed_ext.unwrap_or_else(|| filename.as_ref().extension().and_then(|it| it.to_str()).unwrap());
            format!("{}.{:04}.{}", stem, index + 1, ext)
        }

        let result = &match self.content {
            Archive(_, ArchiveEntry { index, ref name, ..}) =>
                gen(&Path::new(name), index, None),
            Pdf(ref path, index) =>
                gen(&**path, index, Some("png")),
            _ => self.display_path(),
        };

        Path::new(result).to_path_buf()
    }
}


pub fn new_opt_meta(entries: Vec<MetaEntry>) -> Option<Meta> {
    if entries.is_empty() {
        None
    } else {
        Some(Arc::new(entries))
    }
}


pub fn compare_key(a: &Key, b: &Key) -> Ordering {
    let name = natord::compare(&a.1, &b.1);
    if name == Ordering::Equal {
        a.2.cmp(&b.2)
    } else {
        name
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

    pub fn from_key(key: &Key) -> Self {
        SearchKey { path: key.1.clone(), index: Some(key.2) }
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
