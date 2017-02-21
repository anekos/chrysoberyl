
use std::path::PathBuf;

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
}
