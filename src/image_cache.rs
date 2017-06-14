
use std::collections::HashMap;
use std::sync::{Arc, Mutex, Condvar};

use cache::Cache;
use cherenkov::{Cherenkoved, Che};
use entry::{Entry, Key};
use entry_image;
use image::{ImageBuffer};
use size::Size;
use state::DrawingState;



#[derive(Clone)]
pub struct ImageCache {
    cherenkoved: Arc<Mutex<Cherenkoved>>,
    cache: Cache<Key, Result<ImageBuffer, String>>,
    fetching: Arc<(Mutex<HashMap<Key, bool>>, Condvar)>,
}


impl ImageCache {
    pub fn new(limit: usize) -> ImageCache {
        ImageCache {
            cherenkoved: Arc::new(Mutex::new(Cherenkoved::new())),
            cache: Cache::new(limit),
            fetching: Arc::new((Mutex::new(HashMap::new()), Condvar::new())),
        }
    }

    pub fn update_limit(&mut self, limit: usize) {
        self.cache.update_limit(limit);
    }

    pub fn clear(&mut self) {
        self.cache.clear();

        // Cancel current fetchings
        let &(ref fetching, ref cond) = &*self.fetching;
        let mut fetching = fetching.lock().unwrap();
        for it in fetching.values_mut() {
            *it = false;
        }
        cond.notify_all();
    }

    pub fn mark_fetching(&mut self, key: Key) -> bool {
        trace!("image_cache/fetching: key={:?}", key);

        let &(ref fetching, _) = &*self.fetching;
        let mut fetching = fetching.lock().unwrap();
        if self.cache.contains(&key) || fetching.contains_key(&key) {
            false
        } else {
            fetching.insert(key, true);
            true
        }
    }

    pub fn push(&mut self, key: Key, image_buffer: Result<ImageBuffer, String>) {
        trace!("image_cache/push: key={:?}", key);

        let &(ref fetching, ref cond) = &*self.fetching;
        let mut fetching = fetching.lock().unwrap();
        if fetching.remove(&key) == Some(true) {
            self.cache.push(key.clone(), image_buffer);
        }
        cond.notify_all();
    }

    pub fn get_image_buffer(&mut self, entry: &Entry, cell_size: &Size, drawing: &DrawingState) -> Result<ImageBuffer, String> {
        {
            let mut cherenkoved = self.cherenkoved.lock().unwrap();
            cherenkoved.get_image_buffer(entry, cell_size, drawing)
        }.unwrap_or_else(|| {
            self.wait(&entry.key);
            self.cache.get_or_update(entry.key.clone(), move |_| {
                entry_image::get_image_buffer(entry, cell_size, drawing)
            })
        })
    }

    pub fn cherenkov(&mut self, entry: &Entry, cell_size: &Size, che: &Che, drawing: &DrawingState) {
        let mut cherenkoved = self.cherenkoved.lock().unwrap();
        cherenkoved.cherenkov(entry, cell_size, che, drawing)
    }

    pub fn uncherenkov(&mut self, entry: &Entry) {
        let mut cherenkoved = self.cherenkoved.lock().unwrap();
        cherenkoved.remove(entry)
    }

    fn wait(&mut self, key: &Key) {
        trace!("image_cache/wait/start: key={:?}", key);

        let &(ref fetching, ref cond) = &*self.fetching;
        let mut fetching = fetching.lock().unwrap();
        while fetching.get(key) == Some(&true) {
            fetching = cond.wait(fetching).unwrap();
        }
        trace!("image_cache/wait/end: key={:?}", key);
    }
}
