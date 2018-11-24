
use std::collections::HashMap;
use std::sync::{Arc, Mutex, Condvar};

use cache::Cache;
use cherenkov::{Cherenkoved, Modifier};
use entry::image::Imaging;
use entry::{Entry, Key, self};
use image::ImageBuffer;



const SIZE_LIMIT: usize = 3;


#[derive(Clone)]
pub struct Stage {
    cache: Cache<Key, Result<ImageBuffer, String>>,
    fetching: Arc<(Mutex<HashMap<Key, bool>>, Condvar)>,
}


#[derive(Clone)]
pub struct ImageCache {
    limit: usize,
    cherenkoved: Arc<Mutex<Cherenkoved>>,
    stages: Cache<Imaging, Stage>, /* String for display the error */
}


impl Stage {
    pub fn len(&self) -> usize {
        self.cache.len()
    }
}

impl ImageCache {
    pub fn new(limit: usize) -> ImageCache {
        ImageCache {
            stages: Cache::new(SIZE_LIMIT),
            cherenkoved: Arc::new(Mutex::new(Cherenkoved::new())),
            limit,
        }
    }

    pub fn update_limit(&mut self, limit: usize) {
        self.limit = limit;
        self.stages.each_mut(move |it| it.cache.update_limit(limit));
    }

    pub fn clear(&mut self) {
        self.stages.each(|stage| {
            // Cancel current fetchings
            let &(ref fetching, ref cond) = &*stage.fetching;
            let mut fetching = fetching.lock().unwrap();
            for it in fetching.values_mut() {
                *it = false;
            }
            cond.notify_all();
        });

        self.stages.clear();
    }

    pub fn clear_entry(&mut self, imaging: &Imaging, key: &Key) -> bool {
        let mut stage = self.get_stage(imaging);
        stage.cache.clear_entry(key)
    }

    pub fn mark_fetching(&mut self, imaging: &Imaging, key: Key) -> bool {
        trace!("image_cache/mark_fetching: key={:?}", key);

        let stage = self.get_stage(imaging);

        let &(ref fetching, _) = &*stage.fetching;
        let mut fetching = fetching.lock().unwrap();
        if stage.cache.contains(&key) || fetching.contains_key(&key) {
            false
        } else {
            fetching.insert(key, true);
            true
        }
    }

    pub fn push(&mut self, imaging: &Imaging, key: &Key, image_buffer: Result<ImageBuffer, String>) {
        trace!("image_cache/push: key={:?}", key);

        let mut stage = self.get_stage(imaging);

        let do_push = {
            let &(ref fetching, ref cond) = &*stage.fetching;
            let mut fetching = fetching.lock().unwrap();
            let result = fetching.remove(key) == Some(true);
            cond.notify_all();
            result
        };
        if do_push {
            stage.cache.push(key.clone(), image_buffer);
        }
    }

    pub fn get_image_buffer(&mut self, entry: &Entry, imaging: &Imaging) -> Result<ImageBuffer, String> {
        {
            let mut cherenkoved = self.cherenkoved.lock().unwrap();
            cherenkoved.get_image_buffer(entry, imaging).map(|it| it.map_err(|it| s!(it)))
        }.unwrap_or_else(|| {
            let stage = self.get_stage(imaging);

            let &(ref fetching, ref cond) = &*stage.fetching;
            let mut fetching = fetching.lock().unwrap();
            while fetching.get(&entry.key) == Some(&true) {
                fetching = cond.wait(fetching).unwrap();
            }

            stage.cache.get_or_update(entry.key.clone(), move |_| {
                entry::image::get_image_buffer(entry, imaging).map_err(|it| s!(it))
            })
        })
    }

    pub fn len(&self) -> Vec<usize> {
        let mut result = vec![];
        self.stages.each(|it| {
            result.push(it.len());
        });
        result
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

    fn get_stage(&mut self, imaging: &Imaging) -> Stage {
        let limit = self.limit;
        self.stages.get_or_update(imaging.clone(), move |_| {
            Stage { cache: Cache::new(limit), fetching: Arc::new((Mutex::new(HashMap::new()), Condvar::new())) }
        })
    }
}
