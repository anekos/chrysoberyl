
use std::clone::Clone;
use std::cmp::Eq;
use std::hash::Hash;
use std::sync::{Arc, Mutex};

use lru_cache::LruCache;



#[derive(Clone)]
pub struct Cache<K: Hash + Eq, V> {
    entries: Arc<Mutex<LruCache<K, V>>>
}


impl<K, V> Cache<K, V> where K: Hash + Eq, V: Clone {
    pub fn new(limit: usize) -> Cache<K, V> {
        Cache {
            entries: Arc::new(Mutex::new(LruCache::new(limit)))
        }
    }

    pub fn update_limit(&mut self, limit: usize) {
        let mut entries = self.entries.lock().unwrap();
        entries.set_capacity(limit);
    }

    pub fn clear(&mut self) {
        let mut entries = self.entries.lock().unwrap();
        entries.clear();
    }

    pub fn push(&mut self, key: K, value: V) {
        let mut entries = self.entries.lock().unwrap();
        entries.insert(key, value);
    }

    pub fn get(&self, key: &K) -> Option<V> {
        let mut entries = self.entries.lock().unwrap();
        entries.get_mut(key).map(|it| it.clone())
    }

    pub fn contains(&self, key: &K) -> bool {
        let mut entries = self.entries.lock().unwrap();
        entries.contains_key(key)
    }
}
