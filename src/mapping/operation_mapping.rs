
use std::collections::HashMap;



type OperationCode = Vec<String>;

pub struct OperationMapping {
    table: HashMap<String, OperationCode>,
}


impl OperationMapping {
    pub fn new() -> Self {
        OperationMapping { table: HashMap::new() }
    }

    pub fn register(&mut self, name: String, operation: OperationCode) {
        self.table.insert(name, operation);
    }

    pub fn unregister(&mut self, name: &str) {
        self.table.remove(name);
    }

    pub fn matched(&self, name: &str) -> Option<Vec<String>> {
        self.table.get(name).cloned()
    }
}
