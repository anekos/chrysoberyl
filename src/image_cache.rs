
use std::collections::HashSet;
use std::sync::{Arc, Mutex, Condvar};

use gdk_pixbuf::{Pixbuf, Colorspace};

use entry::{Entry, Key};
use cache::Cache;
use size::Size;
use image_buffer::{ImageData, ImageBuffer, get_pixbuf_animation};



#[derive(Clone)]
pub struct ImageCache {
    cache: Cache<Key, Result<CacheEntry, String>>,
    fetching: Arc<(Mutex<HashSet<Key>>, Condvar)>,
}

#[derive(Clone)]
pub enum CacheEntry {
    Static(StaticImage),
    Animation,
}

#[derive(Clone)]
pub struct StaticImage {
    original: Size,
    pixels: Vec<u8>,
    colorspace: Colorspace,
    has_alpha: bool,
    bits_per_sample: i32,
    width: i32,
    height: i32,
    rowstride: i32,
}


impl ImageCache {
    pub fn new() -> ImageCache {
        ImageCache {
            cache: Cache::new(),
            fetching: Arc::new((Mutex::new(HashSet::new()), Condvar::new())),
        }
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
    where F: FnOnce(Entry) -> Result<(Pixbuf, Size), String> {
        let key = entry.key.clone();

        let &(ref fetching, ref cond) = &*self.fetching;

        trace!("image_cache/fetcher: key={:?}", key);
        let image = fetcher(entry);
        self.cache.push(
            key.clone(),
            image.map(|(pixbuf, original)| {
                CacheEntry::Static(
                    StaticImage {
                        original: original,
                        pixels: unsafe { pixbuf.get_pixels().to_vec() },
                        colorspace: pixbuf.get_colorspace(),
                        bits_per_sample: pixbuf.get_bits_per_sample(),
                        has_alpha: pixbuf.get_has_alpha(),
                        width: pixbuf.get_width(),
                        height: pixbuf.get_height(),
                        rowstride: pixbuf.get_rowstride(),
                    })
            }));

        {
            trace!("image_cache/finished/static: key={:?}", key);
            let mut fetching = fetching.lock().unwrap();
            fetching.remove(&key);
            cond.notify_all();
        }
    }

    pub fn push_animation(&mut self, key: Key) {
        let &(ref fetching, ref cond) = &*self.fetching;

        self.cache.push(key.clone(), Ok(CacheEntry::Animation));

        trace!("image_cache/finished/animation: key={:?}", key);
        let mut fetching = fetching.lock().unwrap();
        fetching.remove(&key);
        cond.notify_all();
    }

    pub fn get(&mut self, entry: &Entry) -> Option<Result<ImageData, String>> {
        self.wait(&entry.key);
        self.cache.get(&entry.key).map(|found| {
            found.and_then(|found| found.get_image_data(entry))
        })
    }

    pub fn wait(&mut self, key: &Key) {
        trace!("image_cache/wait/start: key={:?}", key);
        let &(ref fetching, ref cond) = &*self.fetching;
        let mut fetching = fetching.lock().unwrap();
        while fetching.contains(key) {
            fetching = cond.wait(fetching).unwrap();
        }
        trace!("image_cache/wait/end: key={:?}", key);
    }
}

impl CacheEntry {
    pub fn get_image_data(&self, entry: &Entry) -> Result<ImageData, String> {
        match *self {
            CacheEntry::Static(ref it) =>
                Ok(ImageData {
                    size: it.original,
                    buffer: ImageBuffer::Static(
                        Pixbuf::new_from_vec(
                            it.pixels.clone(),
                            it.colorspace,
                            it.has_alpha,
                            it.bits_per_sample,
                            it.width,
                            it.height,
                            it.rowstride))
                }),
            CacheEntry::Animation =>
                get_pixbuf_animation(entry)
        }
    }
}
