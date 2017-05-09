
use std::fs::File;
use std::io::Read;
use std::path::Path;

use gdk_pixbuf::{Pixbuf, PixbufAnimation, Colorspace, PixbufLoader};

use size::Size;



#[derive(Clone)]
pub struct ImageData {
    pub size: Size,
    pub buffer: ImageBuffer,
}


#[derive(Clone)]
pub enum ImageBuffer {
    Static(StaticImageBuffer),
    Animation(AnimationBuffer),
}


#[derive(Clone)]
pub struct StaticImageBuffer {
    original_size: Size,
    pixels: Vec<u8>,
    colorspace: Colorspace,
    has_alpha: bool,
    bits_per_sample: i32,
    width: i32,
    height: i32,
    rowstride: i32,
}

#[derive(Clone)]
pub struct AnimationBuffer {
    source: Vec<u8>,
}


impl StaticImageBuffer {
    pub fn new_from_pixbuf(pixbuf: &Pixbuf) -> StaticImageBuffer {
        StaticImageBuffer {
            original_size: Size::from_pixbuf(pixbuf),
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
    pub fn new_from_file<T: AsRef<Path>>(path: T) -> Result<AnimationBuffer, String> {
        File::open(path).and_then(|mut file| {
            let mut buffer = vec![];
            file.read_to_end(&mut buffer).map(|_| AnimationBuffer { source: buffer })
        }).map_err(|it| s!(it))
    }

    pub fn new_from_slice(source: &[u8]) -> AnimationBuffer {
        AnimationBuffer { source: source.to_vec() }
    }

    pub fn get_pixbuf_animation(&self) -> Result<PixbufAnimation, String> {
        let loader = PixbufLoader::new();
        loader.loader_write(&*self.source.as_slice()).map(|_| {
            loader.close().unwrap();
            loader.get_animation().unwrap()
        }).map_err(|it| s!(it))
    }
}
