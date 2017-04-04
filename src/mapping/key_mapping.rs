
use std::collections::HashMap;

use operation::Operation;


pub struct KeyMapping {
    table: HashMap<String, Operation>
}


impl KeyMapping {
    pub fn new() -> KeyMapping {
        KeyMapping { table: HashMap::new() }
    }

    pub fn register(&mut self, key: String, operation: Operation) {
        self.table.insert(key, operation);
    }

    pub fn matched(&self, key: &str) -> Option<Operation> {
        self.table.get(key).cloned()
    }
}
