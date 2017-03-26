
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::mpsc::{channel, Sender, Receiver};

use encoding::types::EncodingRef;
use gdk_pixbuf::{Pixbuf, PixbufAnimation, PixbufLoader};
use gtk::prelude::*;
use gtk::Image;
use gtk;
use immeta::markers::Gif;
use immeta::{self, GenericMetadata};
use rand::{self, ThreadRng};
use rand::distributions::{IndependentSample, Range};

use entry::{Entry,EntryContainer, EntryContainerOptions};
use utils::path_to_str;



pub fn get_pixbuf(entry: &Entry, width: i32, height: i32) -> Result<Pixbuf, gtk::Error> {
    use gdk_pixbuf::InterpType;

    match *entry {
        Entry::File(ref path) | Entry::Http(ref path, _) =>
            Pixbuf::new_from_file_at_scale(path_to_str(path), width, height, true),
        Entry::Archive(_, ref entry) => {
            let loader = PixbufLoader::new();
            loader.loader_write(&*entry.content.as_slice()).map(|_| {
                loader.close().unwrap();
                let source = loader.get_pixbuf().unwrap();
                let (scale, out_width, out_height) = calculate_scale(&source, width, height);
                let scaled = unsafe { Pixbuf::new(0, false, 8, out_width, out_height).unwrap() };
                source.scale(&scaled, 0, 0, out_width, out_height, 0.0, 0.0, scale, scale, InterpType::Bilinear);
                scaled
            })
        }
    }
}

pub fn get_pixbuf_animation(entry: &Entry) -> Result<PixbufAnimation, gtk::Error> {
    match *entry {
        Entry::File(ref path) | Entry::Http(ref path, _) =>
            PixbufAnimation::new_from_file(path_to_str(path)),
        Entry::Archive(_, ref entry) => {
            let loader = PixbufLoader::new();
            loader.loader_write(&*entry.content.as_slice()).map(|_| {
                loader.close().unwrap();
                loader.get_animation().unwrap()
            })
        }
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
