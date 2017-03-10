
use gtk;
use gdk_pixbuf::{Pixbuf, PixbufAnimation, PixbufLoader};
use immeta::{self, GenericMetadata};

use entry::Entry;
use utils::path_to_str;



pub fn get_pixbuf_animation(entry: &Entry) -> Result<PixbufAnimation, gtk::Error> {
    match *entry {
        Entry::File(ref path) => PixbufAnimation::new_from_file(path_to_str(path)),
        Entry::Http(ref path, _) => PixbufAnimation::new_from_file(path_to_str(path)),
        Entry::Archive(_, _, ref content) => {
            let loader = PixbufLoader::new();
            loader.loader_write(&*content.as_slice()).map(|_| {
                loader.close().unwrap();
                loader.get_animation().unwrap()
            })
        }
    }
}

pub fn get_pixbuf(entry: &Entry, width: i32, height: i32) -> Result<Pixbuf, gtk::Error> {
    use gdk_pixbuf::InterpType;

    match *entry {
        Entry::File(ref path) => Pixbuf::new_from_file_at_scale(path_to_str(path), width, height, true),
        Entry::Http(ref path, _) => Pixbuf::new_from_file_at_scale(path_to_str(path), width, height, true),
        Entry::Archive(_, _, ref content) => {
            let loader = PixbufLoader::new();
            let pixbuf = loader.loader_write(&(*content).clone().as_slice()).map(|_| {
                loader.close().unwrap();
                let source = loader.get_pixbuf().unwrap();
                let (scale, out_width, out_height) = calculate_scale(&source, width, height);
                let mut scaled = unsafe { Pixbuf::new(0, false, 8, out_width, out_height).unwrap() };
                source.scale(&mut scaled, 0, 0, out_width, out_height, 0.0, 0.0, scale, scale, InterpType::Bilinear);
                scaled
            });
            pixbuf
        }
    }
}

pub fn get_meta(entry: &Entry) -> Result<GenericMetadata, immeta::Error> {
    match *entry {
        Entry::File(ref path) => immeta::load_from_file(&path),
        Entry::Http(ref path, _) => immeta::load_from_file(&path),
        Entry::Archive(_, _, ref content) => immeta::load_from_buf(&content),
    }
}

fn calculate_scale(pixbuf: &Pixbuf, max_width: i32, max_height: i32) -> (f64, i32, i32) {
    let (in_width, in_height) = (pixbuf.get_width(), pixbuf.get_height());
    let mut scale = max_width as f64 / in_width as f64;
    let mut out_height = (in_height as f64 * scale) as i32;
    if out_height > max_height {
        scale = max_height as f64 / in_height as f64;
        out_height = (in_height as f64 * scale) as i32;
    }
    (scale, (in_width as f64 * scale) as i32, out_height)
}
