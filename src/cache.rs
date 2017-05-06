
use std::clone::Clone;
use std::cmp::Eq;
use std::collections::HashMap;
use std::hash::Hash;
use std::sync::{Arc, Mutex};



#[derive(Clone)]
pub struct Cache<K, V> {
    limit: usize,
    entries: Arc<Mutex<HashMap<K, V>>>
}


impl<K, V> Cache<K, V> where K: Hash + Eq, V: Clone {
    pub fn new() -> Cache<K, V> {
        Cache {
            limit: 10,
            entries: Arc::new(Mutex::new(HashMap::new()))
        }
    }

    pub fn push(&mut self, key: K, value: V) {
        let mut entries = self.entries.lock().unwrap();
        entries.insert(key, value);
    }

    pub fn get(&self, key: &K) -> Option<V> {
        let entries = self.entries.lock().unwrap();
        entries.get(key).cloned()
    }

    pub fn contains(&self, key: &K) -> bool {
        let entries = self.entries.lock().unwrap();
        entries.contains_key(key)
    }
}
