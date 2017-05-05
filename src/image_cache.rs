
use gdk_pixbuf::{Pixbuf, Colorspace};

use entry::{Entry, Key};
use cache::Cache;
use size::Size;
use image_buffer::{ImageData, ImageBuffer, get_pixbuf_animation};



#[derive(Clone)]
pub struct ImageCache {
    cache: Cache<Key, Result<CacheEntry, String>>
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
        }
    }

    pub fn push(&mut self, key: Key, image: Result<(Pixbuf, Size), String>) {
        self.cache.push(
            key,
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
            }))
    }

    pub fn push_animation(&mut self, key: Key) {
        self.cache.push(key, Ok(CacheEntry::Animation))
    }

    pub fn get(&self, entry: &Entry) -> Option<Result<ImageData, String>> {
        self.cache.get(&entry.key).map(|found| {
            found.and_then(|found| found.get_image_data(entry))
        })
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
