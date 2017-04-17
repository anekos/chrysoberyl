
use std::collections::HashMap;

use operation::Operation;


pub struct KeyMapping {
    table: HashMap<String, Vec<String>>
}


impl KeyMapping {
    pub fn new() -> KeyMapping {
        KeyMapping { table: HashMap::new() }
    }

    pub fn register(&mut self, key: String, operation: &[String]) {
        self.table.insert(key, operation.to_vec());
    }

    pub fn matched(&self, key: &str) -> Option<Result<Operation, String>> {
        self.table.get(key).cloned().map(|op| {
            Operation::parse_from_vec(&op)
        })
    }
}
