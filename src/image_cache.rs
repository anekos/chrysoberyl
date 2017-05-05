
use gdk_pixbuf::{Pixbuf, Colorspace};

use entry::Key;
use cache::Cache;



pub struct ImageCache {
    cache: Cache<Key, Entry>
}

#[derive(Clone)]
pub struct Entry {
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
        }
    }

    pub fn push(&mut self, key: Key, pixbuf: &Pixbuf) {
        self.cache.push(
            key,
            Entry {
                pixels: unsafe { pixbuf.get_pixels().to_vec() },
                colorspace: pixbuf.get_colorspace(),
                bits_per_sample: pixbuf.get_bits_per_sample(),
                has_alpha: pixbuf.get_has_alpha(),
                width: pixbuf.get_width(),
                height: pixbuf.get_height(),
                rowstride: pixbuf.get_rowstride(),
            })
    }

    pub fn get(&self, key: &Key) -> Option<Pixbuf> {
        self.cache.get(key).map(|entry| {
            Pixbuf::new_from_vec(
                entry.pixels.clone(),
                entry.colorspace,
                entry.has_alpha,
                entry.bits_per_sample,
                entry.width,
                entry.height,
                entry.rowstride)
        })
    }
}
