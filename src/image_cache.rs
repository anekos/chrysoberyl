
use std::collections::HashMap;
use std::sync::{Arc, Mutex, Condvar};

use cache::Cache;
use cherenkov::{Cherenkoved, Modifier};
use entry::image::Imaging;
use entry::{Entry, Key, self};
use image::ImageBuffer;
use size::Size;



const SIZE_LIMIT: usize = 3;


#[derive(Clone)]
pub struct ImageCache {
    limit: usize,
    cherenkoved: Arc<Mutex<Cherenkoved>>,
    cache: Cache<Size, Cache<Key, Result<ImageBuffer, String>>>, /* String for display the error */
    fetching: Arc<(Mutex<HashMap<Key, bool>>, Condvar)>,
}


impl ImageCache {
    pub fn new(limit: usize) -> ImageCache {
        ImageCache {
            cache: Cache::new(SIZE_LIMIT),
            cherenkoved: Arc::new(Mutex::new(Cherenkoved::new())),
            fetching: Arc::new((Mutex::new(HashMap::new()), Condvar::new())),
            limit,
        }
    }

    pub fn update_limit(&mut self, limit: usize) {
        self.limit = limit;
        self.cache.each(move |it| it.update_limit(limit));
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

    pub fn clear_entry(&mut self, cell_size: Size, key: &Key) -> bool {
        let mut cache = self.get_sized_cache(cell_size);
        cache.clear_entry(key)
    }

    pub fn mark_fetching(&mut self, cell_size: Size, key: Key) -> bool {
        trace!("image_cache/mark_fetching: key={:?}", key);

        let contains = {
            let mut cache = self.get_sized_cache(cell_size);
            cache.contains(&key)
        };

        let &(ref fetching, _) = &*self.fetching;
        let mut fetching = fetching.lock().unwrap();
        if contains || fetching.contains_key(&key) {
            false
        } else {
            fetching.insert(key, true);
            true
        }
    }

    pub fn push(&mut self, cell_size: Size, key: &Key, image_buffer: Result<ImageBuffer, String>) {
        trace!("image_cache/push: key={:?}", key);

        let do_push = {
            let &(ref fetching, ref cond) = &*self.fetching;
            let mut fetching = fetching.lock().unwrap();
            let result = fetching.remove(key) == Some(true);
            cond.notify_all();
            result
        };
        if do_push {
            let mut cache = self.get_sized_cache(cell_size);
            cache.push(key.clone(), image_buffer);
        }
    }

    pub fn get_image_buffer(&mut self, entry: &Entry, imaging: &Imaging) -> Result<ImageBuffer, String> {
        {
            let mut cherenkoved = self.cherenkoved.lock().unwrap();
            cherenkoved.get_image_buffer(entry, imaging).map(|it| it.map_err(|it| s!(it)))
        }.unwrap_or_else(|| {
            self.wait(&entry.key);
            let cache = self.get_sized_cache(imaging.cell_size);
            cache.get_or_update(entry.key.clone(), move |_| {
                entry::image::get_image_buffer(entry, imaging).map_err(|it| s!(it))
            })
        })
    }

    pub fn cherenkov1(&mut self, entry: &Entry, imaging: &Imaging, modifier: Modifier) {
        let mut cherenkoved = self.cherenkoved.lock().unwrap();
        cherenkoved.cherenkov1(entry, imaging, modifier)
    }

    pub fn cherenkov(&mut self, entry: &Entry, imaging: &Imaging, modifiers: &[Modifier]) {
        let mut cherenkoved = self.cherenkoved.lock().unwrap();
        cherenkoved.cherenkov(entry, imaging, modifiers)
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

    fn get_sized_cache(&mut self, cell_size: Size) -> Cache<Key, Result<ImageBuffer, String>> {
        let limit = self.limit;
        self.cache.get_or_update(cell_size, move |_| {
            Cache::new(limit)
        })
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
