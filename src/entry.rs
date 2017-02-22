
use std::path::PathBuf;
use std::io;
use std::fmt;

use index_pointer::IndexPointer;
use log;



pub struct EntryContainer {
    files: Vec<PathBuf>,
    pub pointer: IndexPointer,
}


impl EntryContainer {
    pub fn new() -> EntryContainer {
        EntryContainer { files: vec![], pointer: IndexPointer::new() }
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
            log::error(format!("Invalid path: {:?}", file));
        }
    }

    pub fn current(&self) -> Option<(PathBuf, usize)> {
        self.pointer.current.and_then(|index| {
            self.files.get(index).map(|it| (it.clone(), index))
        })
    }

    pub fn current_file(&self) -> Option<PathBuf> {
        self.current().map(|(file, _)| file)
    }

    pub fn expand(&mut self, n: usize) {
        let result = self.current().and_then(|(file, index)| {
            let dir = n_parents(file.clone(), n);
            expand(dir.to_path_buf()).ok().and_then(|mut middle| {
                middle.sort();
                let (left, right) = self.files.split_at(index);
                let mut result = vec![];
                result.extend_from_slice(left);
                result.extend_from_slice(middle.as_slice());
                result.extend(right.iter().skip(1).map(|it| it.clone()));
                Some((result, file))
            })
        });

        if let Some((expanded, file)) = result {
            self.files.clear();
            self.files.extend_from_slice(expanded.as_slice());
            self.set_current(file);
        }
    }

    fn set_current(&mut self, entry: PathBuf) {
        if let Some(index) = self.files.iter().position(|it| *it == entry) {
            self.pointer.current = Some(index);
        }
    }

    fn push_file(&mut self, file: PathBuf) {
        let path = file.canonicalize().unwrap();
        if !self.files.contains(&path) {
            self.files.push(path);
        }
    }

    fn push_directory(&mut self, dir: PathBuf) {
        through!([expanded = expand(dir)] {
            for file in expanded {
                self.push(file);
            }
        });
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



fn expand(dir: PathBuf) -> Result<Vec<PathBuf>, io::Error> {
    let mut result = vec![];

    through!([dir = dir.read_dir()] {
        for entry in dir {
            through!([entry = entry] {
                let path = entry.path();
                if path.is_file() && is_image(&path) {
                    result.push(path)
                } else if path.is_dir() {
                    through!([expanded = expand(path)] {
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

fn n_parents(path: PathBuf, n: usize) -> PathBuf {
    if n > 100 {
        return n_parents(path, 100)
    }

    if n > 0 {
        if let Some(parent) = path.clone().parent() {
            return n_parents(parent.to_path_buf(), n - 1);
        }
    }

    path
}
