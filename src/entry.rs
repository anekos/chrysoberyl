
use std::collections::HashMap;
use std::fmt;
use std::io;
use std::path::PathBuf;
use std::rc::Rc;

use immeta;
use rand::{thread_rng, Rng, ThreadRng};

use archive::ArchiveEntry;
use index_pointer::IndexPointer;
use utils::path_to_str;
use validation::is_valid_image_filename;



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

#[derive(Debug, Eq, PartialEq, Hash, Clone, PartialOrd, Ord)]
pub enum Entry {
    File(PathBuf),
    Http(PathBuf, String),
    Archive(Rc<PathBuf>, ArchiveEntry)
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
            match entry {
                Entry::File(ref path) => Some((path.clone(), index, entry.clone())),
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
                    let mut middle: Vec<Rc<Entry>> = middle.into_iter().map(|path| Entry::File(path)).filter(|entry| {
                        current_entry == *entry || (!self.is_duplicated(entry) && self.is_valid_image(entry))
                    }).map(|it| Rc::new(it)).collect();

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
                    let mut tail: Vec<Rc<Entry>> = files.into_iter().map(|path| Entry::File(path)).filter(|entry| {
                        !self.is_duplicated(entry) && self.is_valid_image(entry)
                    }).map(|it| Rc::new(it)).collect();
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
                pointer.first(1);
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
        pointer.first(1);
    }

    pub fn sort(&mut self, pointer: &mut IndexPointer) {
        let current_entry = self.current_entry(pointer);
        self.files.sort();
        self.reset_indices();
        if let Some(current_entry) = current_entry {
            self.set_current(pointer, current_entry);
        }
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
            self.files.len() == 1 && pointer.first(1)
        } else {
            false
        }
    }

    pub fn push_path(&mut self, pointer: &mut IndexPointer, file: &PathBuf) -> bool {
        if file.is_dir() {
            self.push_directory(pointer, file)
        } else if file.is_file() {
            self.push_file(pointer, &file)
        } else {
            puts_error!("at" => "push", "reason" => "Invalid path", "for" => path_to_str(&file));
            false
        }
    }

    pub fn push_http_cache(&mut self, pointer: &mut IndexPointer, file: &PathBuf, url: &str) -> bool {
        let path = file.canonicalize().expect("canonicalize");
        self.push_entry(pointer, Entry::Http(path, url.to_owned()))
    }

    pub fn push_archive_entry(&mut self, pointer: &mut IndexPointer, archive_path: &PathBuf, entry: &ArchiveEntry) -> bool {
        self.push_entry(pointer, Entry::Archive(Rc::new(archive_path.clone()), entry.clone()))
    }

    fn push_file(&mut self, pointer: &mut IndexPointer, file: &PathBuf) -> bool {
        let path = file.canonicalize().expect("canonicalize");
        self.push_entry(pointer, Entry::File(path))
    }

    fn push_directory(&mut self, pointer: &mut IndexPointer, dir: &PathBuf) -> bool {
        let mut changed = false;

        through!([expanded = expand(dir, <u8>::max_value())] {
            for file in expanded {
                changed |= self.push_file(pointer, &file);
            }
        });

        pointer.first(1);
        changed
    }

    fn is_duplicated(&self, entry: &Entry) -> bool {
        self.file_indices.contains_key(entry)
    }

    fn is_valid_image(&self, entry: &Entry) -> bool {
        use self::Entry::*;

        match *entry {
            File(ref path) => self.is_valid_image_file(path),
            Http(ref path, _) => self.is_valid_image_file(path),
            Archive(_, _) => true // FIXME ??
        }
    }

    fn is_valid_image_file(&self, path: &PathBuf) -> bool {
        let opt = &self.options;

        if !is_valid_image_filename(path) {
            return false;
        }

        if !opt.needs_image_info() {
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
        for entry in self.files.iter() {
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
        use self::Entry::*;

        match *self {
            File(ref path) => path_to_str(path).to_owned(),
            Http(_, ref url) => url.clone(),
            Archive(ref archive_path, ref entry) => format!("{}@{}", entry.name, path_to_str(&*archive_path))
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
