
use std::path::PathBuf;
use std::io;

use index_pointer::IndexPointer;



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
        self.files.push(file);
    }

    pub fn current(&self) -> Option<(PathBuf, usize)> {
        if let Some(index) = self.pointer.current {
            self.files.get(index).map(|it| (it.clone(), index))
        } else {
            None
        }
    }

    pub fn current_file(&self) -> Option<PathBuf> {
        self.current().map(|(file, _)| file)
    }

    pub fn expand(&mut self) {
        let expanded = self.current().and_then(|(file, index)| {
            file.parent().and_then(|dir| {
                expand(dir.to_path_buf()).ok().and_then(|middle| {
                    let (left, right) = self.files.split_at(index);
                    let mut result = vec![];
                    result.extend_from_slice(left);
                    result.extend_from_slice(middle.as_slice());
                    result.extend_from_slice(right);
                    Some(result)
                })
            })
        });

        if let Some(expanded) = expanded {
            self.files.clear();
            self.files.extend_from_slice(expanded.as_slice());
        }
    }
}


fn expand(dir: PathBuf) -> Result<Vec<PathBuf>, io::Error> {
    let mut result = vec![];
    let name = dir.file_name().unwrap();

    through!([dir = dir.read_dir()] {
        for entry in dir {
            through!([entry = entry, ft = entry.file_type()] {
                if ft.is_file() {
                    result.push(entry.path())
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
