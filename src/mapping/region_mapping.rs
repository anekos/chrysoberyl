
use std::collections::HashMap;

use crate::key::Key;



pub struct RegionMapping {
    pub table: HashMap<Key, Vec<String>>
}


impl RegionMapping {
    pub fn new() -> Self {
        RegionMapping { table: HashMap::new() }
    }

    pub fn register(&mut self, button: Key, operation: Vec<String>) {
        self.table.insert(button, operation);
    }

    pub fn unregister(&mut self, button: &Key) {
        self.table.remove(button);
    }

    pub fn matched(&self, button: &Key) -> Option<Vec<String>> {
        self.table.get(button).cloned()
    }
}
