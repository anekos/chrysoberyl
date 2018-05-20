
use std::fs::File;
use std::io::{Error as IoError, Read};
use std::path::Path;

use gdk_pixbuf::{Pixbuf, PixbufExt, PixbufAnimation, Colorspace, PixbufLoader, PixbufLoaderExt};
use glib;
use immeta;

use size::Size;



#[derive(Clone)]
pub enum ImageBuffer {
    Static(StaticImageBuffer),
    Animation(AnimationBuffer),
}


#[derive(Clone)]
pub struct StaticImageBuffer {
    pixels: Vec<u8>,
    colorspace: Colorspace,
    has_alpha: bool,
    bits_per_sample: i32,
    width: i32,
    height: i32,
    rowstride: i32,
    pub original_size: Option<Size>,
}

#[derive(Clone)]
pub struct AnimationBuffer {
    source: Vec<u8>,
}


impl ImageBuffer {
    pub fn get_original_size(&self) -> Option<Size> {
        use self::ImageBuffer::*;

        match *self {
            Static(ref image) =>
                image.original_size,
            Animation(ref image) =>
                image.get_original_size().ok(),
        }
    }

    pub fn get_fit_size(&self) -> Option<Size> {
        use self::ImageBuffer::*;

        match *self {
            Static(ref image) =>
                Some(Size::new(image.width, image.height)),
            Animation(_) =>
                None,
        }
    }
}


impl StaticImageBuffer {
    pub fn new_from_pixbuf(pixbuf: &Pixbuf, original_size: Option<Size>) -> StaticImageBuffer {
        StaticImageBuffer {
            original_size,
            pixels: unsafe { pixbuf.get_pixels().to_vec() },
            colorspace: pixbuf.get_colorspace(),
            bits_per_sample: pixbuf.get_bits_per_sample(),
            has_alpha: pixbuf.get_has_alpha(),
            width: pixbuf.get_width(),
            height: pixbuf.get_height(),
            rowstride: pixbuf.get_rowstride(),
        }
    }

    pub fn get_pixbuf(&self) -> Pixbuf {
        Pixbuf::new_from_vec(
            self.pixels.clone(),
            self.colorspace,
            self.has_alpha,
            self.bits_per_sample,
            self.width,
            self.height,
            self.rowstride)
    }
}


impl AnimationBuffer {
    pub fn new_from_file<T: AsRef<Path>>(path: T) -> Result<AnimationBuffer, IoError> {
        let mut file = File::open(path)?;
        let mut buffer = vec![];
        file.read_to_end(&mut buffer).map(|_| AnimationBuffer { source: buffer })
    }

    pub fn new_from_slice(source: &[u8]) -> AnimationBuffer {
        AnimationBuffer { source: source.to_vec() }
    }

    pub fn get_pixbuf_animation(&self) -> Result<PixbufAnimation, glib::Error> {
        let loader = PixbufLoader::new();
        loader.write(&*self.source.as_slice()).map(|_| {
            loader.close().unwrap();
            loader.get_animation().unwrap()
        })
    }

    pub fn get_original_size(&self) -> Result<Size, immeta::Error> {
        immeta::load_from_buf(&self.source).map(|img| {
            let dim = img.dimensions();
            Size::new(dim.width as i32, dim.height as i32)
        })
    }
}
