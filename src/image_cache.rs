
use std::collections::HashSet;
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
    fetching: Arc<(Mutex<HashSet<Key>>, Condvar)>,
}


impl ImageCache {
    pub fn new(limit: usize) -> ImageCache {
        ImageCache {
            cherenkoved: Arc::new(Mutex::new(Cherenkoved::new())),
            cache: Cache::new(limit),
            fetching: Arc::new((Mutex::new(HashSet::new()), Condvar::new())),
        }
    }

    pub fn update_limit(&mut self, limit: usize) {
        self.cache.update_limit(limit);
    }

    pub fn clear(&mut self) {
        self.cache.clear();
        // TODO Remove current fetchings
    }

    pub fn fetching(&mut self, key: Key) -> bool {
        trace!("image_cache/fetching: key={:?}", key);

        let &(ref fetching, _) = &*self.fetching;
        let mut fetching = fetching.lock().unwrap();
        if self.cache.contains(&key) || fetching.contains(&key) {
            false
        } else {
            fetching.insert(key);
            true
        }
    }

    pub fn push<F>(&mut self, entry: Entry, fetcher: F)
    where F: FnOnce(Entry) -> Result<ImageBuffer, String> {
        trace!("image_cache/push: key={:?}", entry.key);

        let key = entry.key.clone();
        let &(ref fetching, ref cond) = &*self.fetching;

        let image = time!("image_cache/fetcher" => fetcher(entry));

        self.cache.push(key.clone(), image);

        {
            trace!("image_cache/finished/static: key={:?}", key);
            let mut fetching = fetching.lock().unwrap();
            fetching.remove(&key);
            cond.notify_all();
        }
    }

    pub fn get_image_buffer(&mut self, entry: &Entry, cell_size: &Size, drawing: &DrawingState) -> Result<ImageBuffer, String> {
        {
            let mut cherenkoved = self.cherenkoved.lock().unwrap();
            cherenkoved.get_image_buffer(entry, cell_size, drawing)
        }.unwrap_or_else(|| {
            self.wait(&entry.key);
            self.cache.get(&entry.key).unwrap_or_else(|| {
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
        while fetching.contains(key) {
            fetching = cond.wait(fetching).unwrap();
        }
        trace!("image_cache/wait/end: key={:?}", key);
    }
}
