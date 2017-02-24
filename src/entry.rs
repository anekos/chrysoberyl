
use std::rc::Rc;
use std::path::PathBuf;
use std::collections::HashMap;
use std::io;
use std::fmt;
use rand::{thread_rng, Rng, ThreadRng};
use immeta;

use index_pointer::IndexPointer;
use output;



pub struct EntryContainer {
    files: Vec<Rc<PathBuf>>,
    file_indices: HashMap<Rc<PathBuf>, usize>,
    options: EntryContainerOptions,
    rng: ThreadRng,
    pub pointer: IndexPointer,
}

#[derive(Debug)]
pub struct EntryContainerOptions {
    pub min_width: Option<u32>,
    pub min_height: Option<u32>,
}


impl EntryContainer {
    pub fn new(options: EntryContainerOptions) -> EntryContainer {
        EntryContainer {
            files: vec![],
            pointer: IndexPointer::new(),
            file_indices: HashMap::new(),
            rng: thread_rng(),
            options: options
        }
    }

    pub fn len(&self) -> usize {
        self.files.len()
    }

    pub fn push(&mut self, file: PathBuf) {
        if file.is_dir() {
            self.push_directory(file);
        } else if file.is_file() {
            self.push_file(file);
        } else {
            output::error(format!("Invalid path: {:?}", file));
        }
    }

    pub fn current(&self) -> Option<(PathBuf, usize)> {
        self.pointer.current.and_then(|index| {
            self.files.get(index).map(|it: &Rc<PathBuf>| {
                ((**it).clone(), index)
            })
        })
    }

    pub fn current_file(&self) -> Option<PathBuf> {
        self.current().map(|(file, _)| file)
    }

    pub fn expand(&mut self, n: u8, recursive: u8) {
        let result = self.current().and_then(|(file, index)| {
            let dir = n_parents(file.clone(), n);
            expand(dir.to_path_buf(), recursive).ok().and_then(|middle| {
                let mut middle: Vec<Rc<PathBuf>> = {
                    middle.into_iter().filter(|path| {
                        *path == file || self.is_valid_image(path)
                    }).map(|path| {
                        Rc::new(path)
                    }).collect()
                };

                middle.sort();

                let (left, right) = self.files.split_at(index);

                let mut result = vec![];
                result.extend_from_slice(left);
                result.extend_from_slice(middle.as_slice());
                result.extend_from_slice(&right[1..]);

                Some((result, file))
            })
        });

        if let Some((expanded, file)) = result {
            self.files.clear();
            self.files.extend_from_slice(expanded.as_slice());
            self.reset_indices();
            self.set_current(file);
        }
    }

    pub fn shuffle(&mut self) {
        let mut source = self.files.clone();
        let mut buffer = source.as_mut_slice();
        self.rng.shuffle(&mut buffer);
        self.files.clear();
        self.files.extend_from_slice(buffer);
        self.reset_indices();
    }

    fn reset_indices(&mut self) {
        self.file_indices.clear();
        for (index, file) in self.files.iter().enumerate() {
            self.file_indices.insert(file.clone(), index);
        }
    }

    fn set_current(&mut self, entry: PathBuf) {
        if let Some(index) = self.file_indices.get(&entry) {
            self.pointer.current = Some(*index);
        }
    }

    fn push_file(&mut self, file: PathBuf) {
        let path = Rc::new(file.canonicalize().expect("canonicalize"));

        if self.is_valid_image(&path) {
            self.file_indices.insert(path.clone(), self.files.len());
            self.files.push(path);
        }
    }

    fn push_directory(&mut self, dir: PathBuf) {
        through!([expanded = expand(dir, <u8>::max_value())] {
            for file in expanded {
                self.push(file);
            }
        });
    }

    fn is_valid_image(&self, path: &PathBuf) -> bool {
        let opt = &self.options;

        if self.file_indices.contains_key(path) {
            return false;
        }

        if opt.min_width.is_none() && opt.min_height.is_none() {
            return true;
        }

        debug!("&is_valid_image(&path): path = {:?}", path);

        if let Ok(img) = immeta::load_from_file(&path) {
            let w = opt.min_width.map(|it| it < img.dimensions().width).unwrap_or(true);
            let h = opt.min_height.map(|it| it < img.dimensions().height).unwrap_or(true);
            w && h
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



fn expand(dir: PathBuf, recursive: u8) -> Result<Vec<PathBuf>, io::Error> {
    let mut result = vec![];

    through!([dir = dir.read_dir()] {
        for entry in dir {
            through!([entry = entry] {
                let path = entry.path();
                if path.is_file() && is_image(&path) {
                    result.push(path)
                } else if recursive > 0 && path.is_dir() {
                    through!([expanded = expand(path, recursive - 1)] {
                        result.extend(expanded)
                    });
                }
            })
        }
    });

    Ok(result)
}

fn is_image(path: &PathBuf) -> bool {
    let image_extensions: Vec<&str> = vec!["jpeg", "jpg", "png", "gif"];
    path.extension().map(|extension| {
        let extension: &str = &extension.to_str().unwrap().to_lowercase();
        image_extensions.contains(&extension)
    }).unwrap_or(false)
}

fn n_parents(path: PathBuf, n: u8) -> PathBuf {
    if n > 0 {
        if let Some(parent) = path.clone().parent() {
            return n_parents(parent.to_path_buf(), n - 1);
        }
    }

    path
}
