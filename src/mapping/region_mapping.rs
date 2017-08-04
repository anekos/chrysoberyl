
use std::collections::HashMap;



pub struct RegionMapping {
    pub table: HashMap<u32, Vec<String>>
}


impl RegionMapping {
    pub fn new() -> Self {
        RegionMapping { table: HashMap::new() }
    }

    pub fn register(&mut self, button: u32, operation: Vec<String>) {
        self.table.insert(button, operation);
    }

    pub fn unregister(&mut self, button: &u32) {
        self.table.remove(button);
    }

    pub fn matched(&self, button: u32) -> Option<Vec<String>> {
        self.table.get(&button).cloned()
    }
}
