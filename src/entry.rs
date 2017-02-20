

use index_pointer::IndexPointer;



pub struct EntryContainer {
    files: Vec<String>,
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

    pub fn push(&mut self, file: String) {
        self.files.push(file);
    }

    pub fn current(&self) -> Option<(String, usize)> {
        if let Some(index) = self.pointer.current {
            self.files.get(index).map(|it| (it.to_owned(), index))
        } else {
            None
        }
    }

    pub fn current_file(&self) -> Option<String> {
        self.current().map(|(file, _)| file)
    }
}
