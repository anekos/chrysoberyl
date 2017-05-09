
use std::cmp::{PartialEq, PartialOrd, Ord, Ordering};
use std::collections::HashMap;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::io;
use std::path::{PathBuf, Path};
use std::rc::Rc;
use std::sync::Arc;


use immeta;
use rand::{thread_rng, Rng, ThreadRng};

use archive::ArchiveEntry;
use index_pointer::IndexPointer;
use utils::path_to_str;
use validation::is_valid_image_filename;
use poppler::PopplerDocument;



pub struct EntryContainer {
    files: Vec<Rc<Entry>>,
    file_indices: HashMap<Rc<Entry>, usize>,
    options: EntryContainerOptions,
    rng: ThreadRng,
}

pub struct EntryContainerOptions {
    pub min_width: Option<u32>,
    pub min_height: Option<u32>,
    pub max_width: Option<u32>,
    pub max_height: Option<u32>,
    pub ratio: Option<f32>, // width / height
}

#[derive(Debug, Eq, Clone)]
pub struct Entry {
    pub key: Key,
    pub content: EntryContent,
    pub meta: Meta
}

#[derive(Debug, Eq, PartialEq, Hash, Clone, PartialOrd, Ord)]
pub enum EntryContent {
    File(PathBuf),
    Http(PathBuf, String),
    Archive(Arc<PathBuf>, ArchiveEntry),
    Pdf(Arc<PathBuf>, usize)
}

pub type Meta = Arc<Vec<MetaEntry>>;
pub type MetaSlice = [MetaEntry];

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

pub type Key = (char, String, usize);


impl Entry {
    pub fn new(content: EntryContent, meta: Meta) -> Entry {
        Entry { key: content.key(), content: content, meta: meta }
    }

    pub fn new_without_meta(content: EntryContent) -> Entry {
        Entry { key: content.key(), content: content, meta: new_meta(&[]) }
    }

    pub fn archive_name(&self) -> &str {
        &self.key.1
    }
}

impl Ord for Entry {
    fn cmp(&self, other: &Entry) -> Ordering {
        self.key.cmp(&other.key)
    }
}

impl PartialEq for Entry {
    fn eq(&self, other: &Entry) -> bool {
        self.key.eq(&other.key)
    }
}

impl PartialOrd for Entry {
    fn partial_cmp(&self, other: &Entry) -> Option<Ordering> {
        self.key.partial_cmp(&other.key)
    }
}

impl Hash for Entry {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.key.hash(state);
    }
}

impl EntryContent {
    fn key(&self) -> Key {
        use self::EntryContent::*;

        match *self {
            File(ref path) =>
                ('f', path_to_str(path).to_owned(), 1),
            Http(_, ref url) =>
                ('h', url.clone(), 1),
            Archive(ref path, ref entry) =>
                ('a', path_to_str(path).to_owned(), entry.index),
            Pdf(ref path, index) =>
                ('p', path_to_str(path).to_owned(), index),
        }
    }
}


impl EntryContainer {
    pub fn new(options: EntryContainerOptions) -> EntryContainer {
        EntryContainer {
            files: vec![],
            file_indices: HashMap::new(),
            rng: thread_rng(),
            options: options
        }
    }

    pub fn clear(&mut self, pointer: &mut IndexPointer) {
        self.files.clear();
        pointer.current = None;
    }

    pub fn len(&self) -> usize {
        self.files.len()
    }

    pub fn current(&self, pointer: &IndexPointer) -> Option<(Entry, usize)> {
        pointer.current.and_then(|index| {
            self.files.get(index).map(|it: &Rc<Entry>| {
                ((**it).clone(), index)
            })
        })
    }

    pub fn current_with(&self, pointer: &IndexPointer, delta: usize) -> Option<(Entry, usize)> {
        pointer.current_with(delta).and_then(|index| {
            self.files.get(index).map(|it: &Rc<Entry>| {
                ((**it).clone(), index)
            })
        })
    }

    pub fn current_entry(&self, pointer: &IndexPointer) -> Option<Entry> {
        self.current(pointer).map(|(entry, _)| entry)
    }

    pub fn current_for_file(&self, pointer: &IndexPointer) -> Option<(PathBuf, usize, Entry)> {
        self.current(pointer).and_then(|(entry, index)| {
            match entry.content {
                EntryContent::File(ref path) => Some((path.clone(), index, entry.clone())),
                _ => None
            }
        })
    }

    pub fn to_displays(&self) -> Vec<String> {
        self.files.iter().map(|it: &Rc<Entry>| (**it).display_path()).collect()
    }

    pub fn expand(&mut self, pointer: &mut IndexPointer, dir: Option<PathBuf>, n: u8, recursive: u8) {
        let result =
            if let Some((file, index, current_entry)) = self.current_for_file(pointer) {
                let dir = n_parents(file.clone(), n);
                expand(&dir.to_path_buf(), recursive).ok().and_then(|middle| {
                    let mut middle: Vec<Rc<Entry>> = middle.into_iter().map(|it| {
                        Entry::new(EntryContent::File(it), current_entry.meta.clone())
                    }).filter(|entry| {
                        current_entry == *entry || (!self.is_duplicated(entry) && self.is_valid_image(entry))
                    }).map(Rc::new).collect();

                    middle.sort();

                    let (left, right) = self.files.split_at(index);

                    let mut result = vec![];
                    result.extend_from_slice(left);
                    result.extend_from_slice(middle.as_slice());
                    result.extend_from_slice(&right[1..]);

                    Some((result, Some(current_entry)))
                })
            } else if let Some(dir) = dir {
                let dir = n_parents(dir, n - 1);
                expand(&dir.to_path_buf(), recursive).ok().map(|files| {
                    let mut result = self.files.clone();
                    let mut tail: Vec<Rc<Entry>> = files.into_iter().map(|it| {
                        Entry::new(EntryContent::File(it), new_meta(&[]))
                    }).filter(|entry| {
                        !self.is_duplicated(entry) && self.is_valid_image(entry)
                    }).map(Rc::new).collect();
                    tail.sort();
                    result.extend_from_slice(tail.as_slice());
                    (result, None)
                })
            } else {
                None
            };

        if let Some((expanded, file)) = result {
            self.files.clear();
            self.files.extend_from_slice(expanded.as_slice());
            self.reset_indices();
            if let Some(file) = file {
                self.set_current(pointer, file);
            } else  {
                pointer.first(1, false);
            }
        }
    }

    pub fn shuffle(&mut self, pointer: &mut IndexPointer, fix_current: bool) {
        let current_entry = self.current_entry(pointer);
        let mut source = self.files.clone();
        let mut buffer = source.as_mut_slice();
        self.rng.shuffle(&mut buffer);
        self.files.clear();
        self.files.extend_from_slice(buffer);
        self.reset_indices();

        if fix_current {
            if let Some(current_entry) = current_entry {
                self.set_current(pointer, current_entry);
                return
            }
        }
        pointer.first(1, false);
    }

    pub fn sort(&mut self, pointer: &mut IndexPointer) {
        let current_entry = self.current_entry(pointer);
        self.files.sort();
        self.reset_indices();
        if let Some(current_entry) = current_entry {
            self.set_current(pointer, current_entry);
        }
    }

    pub fn find_next_archive(&self, pointer: &IndexPointer, mut count: usize) -> Option<usize> {
        self.current(pointer).map(|(entry, base_index)| {
            let mut current_archive = entry.archive_name().to_owned();
            for (index, it) in self.files.iter().enumerate().skip(base_index + 1) {
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

    pub fn find_previous_archive(&self, pointer: &IndexPointer, mut count: usize) -> Option<usize> {
        self.current(pointer).map(|(entry, base_index)| {
            let current_archive = entry.archive_name().to_owned();
            let mut previous_archive: Option<&str> = None;
            let mut previous_index = None;
            for (index, it) in self.files.iter().enumerate().rev().skip(self.files.len() - base_index + 1) {
                if let Some(prev) = previous_archive {
                    if it.archive_name() == prev {
                        previous_index = Some(index)
                    } else if count == 1 {
                        break;
                    } else {
                        count -= 1;
                        previous_archive = Some(it.archive_name());
                    }
                } else {
                    if it.archive_name() != current_archive {
                        previous_archive = Some(it.archive_name())
                    }
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

    fn reset_indices(&mut self) {
        self.file_indices.clear();
        for (index, file) in self.files.iter().enumerate() {
            self.file_indices.insert(file.clone(), index);
        }
    }

    fn set_current(&mut self, pointer: &mut IndexPointer, entry: Entry) {
        if let Some(index) = self.file_indices.get(&entry) {
            pointer.current = Some(*index);
        }
    }

    fn push_entry(&mut self, pointer: &mut IndexPointer, entry: Entry) -> bool {
        let entry = Rc::new(entry);

        if self.is_valid_image(&entry) && !self.is_duplicated(&entry) {
            self.file_indices.insert(entry.clone(), self.files.len());
            self.files.push(entry);
            self.files.len() == 1 && pointer.first(1, false)
        } else {
            false
        }
    }

    pub fn push_path(&mut self, pointer: &mut IndexPointer, file: &PathBuf, meta: &MetaSlice) -> bool {
        if file.is_dir() {
            self.push_directory(pointer, file, meta)
        } else if file.is_file() {
            self.push_file(pointer, file, meta)
        } else {
            puts_error!("at" => "push", "reason" => "Invalid path", "for" => path_to_str(file));
            false
        }
    }

    pub fn push_http_cache(&mut self, pointer: &mut IndexPointer, file: &PathBuf, url: &str, meta: &MetaSlice) -> bool {
        let path = file.canonicalize().expect("canonicalize");
        self.push_entry(
            pointer,
            Entry::new(EntryContent::Http(path, url.to_owned()), new_meta(meta)))
    }

    pub fn push_archive_entry(&mut self, pointer: &mut IndexPointer, archive_path: &PathBuf, entry: &ArchiveEntry) -> bool {
        self.push_entry(
            pointer,
            Entry::new_without_meta(
                EntryContent::Archive(
                    Arc::new(archive_path.clone()),
                    entry.clone())))
    }

    pub fn push_pdf(&mut self, pointer: &mut IndexPointer, pdf_path: &PathBuf, meta: &MetaSlice) -> bool {
        let document = PopplerDocument::new_from_file(&pdf_path);
        let n_pages = document.n_pages();
        let mut result = false;
        for index in 0 .. n_pages {
            let content = EntryContent::Pdf(Arc::new(pdf_path.clone()), index);
            result |= self.push_entry(pointer, Entry::new(content, new_meta(meta)));
        }
        result
    }

    pub fn search(&self, key: &SearchKey) -> Option<usize> {
        self.files.iter().position(|it| key.matches(it))
    }

    fn push_file(&mut self, pointer: &mut IndexPointer, file: &PathBuf, meta: &MetaSlice) -> bool {
        let path = file.canonicalize().expect("canonicalize");
        self.push_entry(
            pointer,
            Entry::new(EntryContent::File(path), new_meta(meta)))
    }

    fn push_directory(&mut self, pointer: &mut IndexPointer, dir: &PathBuf, meta: &MetaSlice) -> bool {
        let mut changed = false;

        through!([expanded = expand(dir, <u8>::max_value())] {
            for file in expanded {
                changed |= self.push_file(pointer, &file, meta);
            }
        });

        pointer.first(1, false);
        changed
    }

    fn is_duplicated(&self, entry: &Entry) -> bool {
        self.file_indices.contains_key(entry)
    }

    fn is_valid_image(&self, entry: &Entry) -> bool {
        use self::EntryContent::*;

        match (*entry).content {
            File(ref path) | Http(ref path, _) => self.is_valid_image_file(path),
            Archive(_, _) | Pdf(_,  _) => true, // FIXME archive
        }
    }

    fn is_valid_image_file(&self, path: &PathBuf) -> bool {
        let opt = &self.options;

        if !opt.needs_image_info() && is_valid_image_filename(path){
            return true;
        }

        debug!("&is_valid_image(&path): path = {:?}", path);

        if let Ok(img) = immeta::load_from_file(&path) {
            let dim = img.dimensions();

            let min_w = opt.min_width.map(|it| it <= dim.width).unwrap_or(true);
            let min_h = opt.min_height.map(|it| it <= dim.height).unwrap_or(true);
            let max_w = opt.max_width.map(|it| dim.width <= it).unwrap_or(true);
            let max_h = opt.max_height.map(|it| dim.height <= it).unwrap_or(true);
            let ratio = opt.ratio_matches(dim.width, dim.height);

            min_w && min_h && max_w && max_h && ratio
        } else {
            false
        }
    }
}


impl fmt::Display for EntryContainer {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for entry in &self.files {
            writeln!(f, "{:?}", entry).unwrap();
        }
        Ok(())
    }
}


impl EntryContainerOptions {
    pub fn new() -> EntryContainerOptions {
        EntryContainerOptions { min_width: None, min_height: None, max_width: None, max_height: None, ratio: None }
    }

    fn needs_image_info(&self) -> bool {
        self.min_width.is_some() || self.min_height.is_some() || self.max_width.is_some() || self.max_height.is_some() || self.ratio.is_some()
    }

    fn ratio_matches(&self, width: u32, height: u32) -> bool {
        if let Some(ratio) = self.ratio {
            (ratio - (width as f32 / height as f32)).abs() < 0.001
        } else {
            true
        }
    }
}


impl Entry {
    pub fn display_path(&self) -> String {
        use self::EntryContent::*;

        match (*self).content {
            File(ref path) => path_to_str(path).to_owned(),
            Http(_, ref url) => url.clone(),
            Archive(ref archive_path, ref entry) => format!("{}@{}", entry.name, path_to_str(&*archive_path)),
            Pdf(ref pdf_path, ref index) => format!("{}@{}", index, path_to_str(&*pdf_path)),
        }
    }
}


pub fn new_meta(meta: &MetaSlice) -> Meta {
    Arc::new(o!(meta))
}

pub fn new_empty_meta() -> Meta {
    Arc::new(vec![])
}

pub fn new_meta_from_vec(meta: Vec<MetaEntry>) -> Meta {
    Arc::new(meta)
}



impl MetaEntry {
    pub fn new_without_value(key: String) -> MetaEntry {
        MetaEntry { key: key, value: o!("true") }
    }
}


impl SearchKey {
    pub fn matches(&self, entry: &Entry) -> bool {
        if let Some(index) = self.index {
            Self::matches_with_path_and_index(&entry.content, &self.path, index)
        } else {
            Self::matches_with_path(&entry.content, &self.path)
        }
    }

    fn matches_with_path(entry: &EntryContent, key: &str) -> bool {
        use self::EntryContent::*;

        match *entry {
            Http(_, ref url) =>
                url == key,
            File(ref path) =>
                Path::new(key) == path,
            Archive(ref path, _) | Pdf(ref path, _) =>
                Path::new(key) == **path,
        }
    }

    fn matches_with_path_and_index(entry: &EntryContent, key_path: &str, key_index: usize) -> bool {
        use self::EntryContent::*;

        match *entry {
            Http(_, ref url) =>
                url == key_path,
            File(ref path) =>
                Path::new(key_path) == path,
            Archive(ref path, ref entry) =>
                Path::new(key_path) == **path && key_index == entry.index,
            Pdf(ref path, index) =>
                Path::new(key_path) == **path && key_index == index,
        }
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

fn expand(dir: &PathBuf, recursive: u8) -> Result<Vec<PathBuf>, io::Error> {
    let mut result = vec![];

    through!([dir = dir.read_dir()] {
        for entry in dir {
            through!([entry = entry] {
                let path = entry.path();
                if path.is_file() {
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
