
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

    pub fn is_empty(&self) -> bool {
        self.files.is_empty()
    }

    pub fn push(&mut self, file: PathBuf) {
        if file.is_dir() {
            self.push_directory(file);
        } else if file.is_file() {
            self.files.push(file.canonicalize().unwrap());
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

    pub fn expand(&mut self) {
        let result = self.current().and_then(|(file, index)| {
            file.clone().parent().and_then(|dir| {
                expand(dir.to_path_buf()).ok().and_then(|mut middle| {
                    middle.sort();
                    let (left, right) = self.files.split_at(index);
                    let mut result = vec![];
                    result.extend_from_slice(left);
                    result.extend_from_slice(middle.as_slice());
                    result.extend(right.iter().skip(1).map(|it| it.clone()));
                    Some((result, file))
                })
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
    let name = dir.file_name().unwrap();

    through!([dir = dir.read_dir()] {
        for entry in dir {
            through!([entry = entry, ft = entry.file_type()] {
                let path = entry.path();
                if ft.is_file() && is_image(&path) {
                    result.push(path)
                } else if ft.is_dir() {
                    if name != entry.file_name() {
                        through!([expanded = expand(entry.path())] {
                            result.extend(expanded)
                        });
                    }
                } else if ft.is_symlink() {
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
