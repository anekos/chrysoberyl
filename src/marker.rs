
use std::collections::HashMap;

use entry::SearchKey;



pub struct Marker {
    store: HashMap<String, SearchKey>,
}

impl Marker {
    pub fn new() -> Self {
        Marker { store: HashMap::new() }
    }

    pub fn get(&self, name: &str) -> Option<&SearchKey> {
        self.store.get(name)
    }

    pub fn set(&mut self, name: String, search_key: SearchKey) {
        self.store.insert(name, search_key);
    }
}
