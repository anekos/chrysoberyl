
use std::collections::HashMap;
use std::sync::{Arc, Mutex, Condvar};

use cache::Cache;
use cherenkov::{Cherenkoved, Modifier};
use entry::{Entry, Key, self};
use image::{ImageBuffer};
use size::Size;
use state::Drawing;



#[derive(Clone)]
pub struct ImageCache {
    cherenkoved: Arc<Mutex<Cherenkoved>>,
    cache: Cache<Key, Result<ImageBuffer, String>>, /* String for display the error */
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

    pub fn clear_entry(&mut self, key: &Key) -> bool {
        self.cache.clear_entry(key)
    }

    pub fn mark_fetching(&mut self, key: Key) -> bool {
        trace!("image_cache/mark_fetching: key={:?}", key);

        let &(ref fetching, _) = &*self.fetching;
        let mut fetching = fetching.lock().unwrap();
        if self.cache.contains(&key) || fetching.contains_key(&key) {
            false
        } else {
            fetching.insert(key, true);
            true
        }
    }

    pub fn push(&mut self, key: &Key, image_buffer: Result<ImageBuffer, String>) {
        trace!("image_cache/push: key={:?}", key);

        let &(ref fetching, ref cond) = &*self.fetching;
        let mut fetching = fetching.lock().unwrap();
        if fetching.remove(key) == Some(true) {
            self.cache.push(key.clone(), image_buffer);
        }
        cond.notify_all();
    }

    pub fn get_image_buffer(&mut self, entry: &Entry, cell_size: &Size, drawing: &Drawing) -> Result<ImageBuffer, String> {
        {
            let mut cherenkoved = self.cherenkoved.lock().unwrap();
            cherenkoved.get_image_buffer(entry, cell_size, drawing).map(|it| it.map_err(|it| s!(it)))
        }.unwrap_or_else(|| {
            self.wait(&entry.key);
            self.cache.get_or_update(entry.key.clone(), move |_| {
                entry::image::get_image_buffer(entry, cell_size, drawing).map_err(|it| s!(it))
            })
        })
    }

    pub fn cherenkov1(&mut self, entry: &Entry, cell_size: &Size, modifier: Modifier, drawing: &Drawing) {
        let mut cherenkoved = self.cherenkoved.lock().unwrap();
        cherenkoved.cherenkov1(entry, cell_size, modifier, drawing)
    }

    pub fn cherenkov(&mut self, entry: &Entry, cell_size: &Size, modifiers: &[Modifier], drawing: &Drawing) {
        let mut cherenkoved = self.cherenkoved.lock().unwrap();
        cherenkoved.cherenkov(entry, cell_size, modifiers, drawing)
    }

    pub fn cherenkov_reset(&mut self, entry: &Entry) {
        let mut cherenkoved = self.cherenkoved.lock().unwrap();
        cherenkoved.reset(entry);
    }

    pub fn uncherenkov(&mut self, key: &Key) {
        let mut cherenkoved = self.cherenkoved.lock().unwrap();
        cherenkoved.remove(key)
    }

    pub fn undo_cherenkov(&mut self, key: &Key, count: usize) {
        let mut cherenkoved = self.cherenkoved.lock().unwrap();
        cherenkoved.undo(key, count)
    }

    pub fn clear_entry_search_highlights(&mut self, entry: &Entry) -> bool {
        let mut cherenkoved = self.cherenkoved.lock().unwrap();
        cherenkoved.clear_entry_search_highlights(entry)
    }

    pub fn clear_search_highlights(&mut self) -> bool {
        let mut cherenkoved = self.cherenkoved.lock().unwrap();
        cherenkoved.clear_search_highlights()
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
