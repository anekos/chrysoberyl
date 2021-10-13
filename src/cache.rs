
use std::clone::Clone;
use std::cmp::Eq;
use std::hash::Hash;
use std::sync::{Arc, Mutex};

use lru_cache::LruCache;



#[derive(Clone)]
pub struct Cache<K: Hash + Eq, V> {
    entries: Arc<Mutex<LruCache<K, V>>>
}


impl<K, V> Cache<K, V> where K: Hash + Eq + Clone, V: Clone {
    pub fn new(limit: usize) -> Cache<K, V> {
        Cache {
            entries: Arc::new(Mutex::new(LruCache::new(limit)))
        }
    }

    pub fn each<F>(&self, mut block: F) where F: FnMut(&V) {
        let entries = self.entries.lock().unwrap();
        for (_, entry) in &*entries {
            block(entry)
        }
    }

    pub fn each_mut<F>(&mut self, block: F) where F: Fn(&mut V) {
        let mut entries = self.entries.lock().unwrap();
        for (_, entry) in entries.iter_mut() {
            block(entry)
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

    pub fn clear_entry(&mut self, key: &K) -> bool {
        let mut entries = self.entries.lock().unwrap();
        entries.remove(key).is_some()
    }

    pub fn push(&mut self, key: K, value: V) {
        let mut entries = self.entries.lock().unwrap();
        entries.insert(key, value);
    }

    #[allow(dead_code)]
    pub fn get(&self, key: &K) -> Option<V> {
        let mut entries = self.entries.lock().unwrap();
        entries.get_mut(key).cloned()
    }

    pub fn get_or_update<F>(&self, key: K, updater: F) -> V
    where F: FnOnce(&K) -> V {
        let mut entries = self.entries.lock().unwrap();
        if let Some(found) = entries.get_mut(&key) {
            return found.clone()
        }
        let new = updater(&key);
        entries.insert(key, new.clone());
        new
    }

    pub fn contains(&self, key: &K) -> bool {
        let mut entries = self.entries.lock().unwrap();
        entries.contains_key(key)
    }

    pub fn len(&self) -> usize {
        let entries = self.entries.lock().unwrap();
        entries.len()
    }
}
