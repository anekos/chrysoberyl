
use std::fs::File;
use std::io::Read;

use gdk_pixbuf::{Pixbuf, PixbufLoader};
use immeta::markers::Gif;
use immeta::{self, GenericMetadata};

use entry::{Entry, EntryContent};
use poppler::PopplerDocument;
use size::Size;
use state::DrawingState;
use utils::path_to_str;
use image::{ImageBuffer, StaticImageBuffer, AnimationBuffer};



type Error = String;


pub fn is_animation(entry: &Entry) -> bool {
    if let Some(img) = get_meta(entry) {
        if let Ok(img) = img {
            if let Ok(gif) = img.into::<Gif>() {
                if gif.is_animated() {
                    return true
                }
            }
        }
    }
    false
}

pub fn get_image_buffer(entry: &Entry, cell: &Size, drawing: &DrawingState) -> Result<ImageBuffer, Error> {
    if is_animation(entry) {
        get_animation_buffer(entry).map(ImageBuffer::Animation)
    } else {
        get_static_image_buffer(entry, cell, drawing).map(ImageBuffer::Static)
    }
}


pub fn get_static_image_buffer(entry: &Entry, cell: &Size, drawing: &DrawingState) -> Result<StaticImageBuffer, Error> {
    use self::EntryContent::*;

    match (*entry).content {
        File(ref path) | Http(ref path, _) =>
            make_scaled_from_file(path_to_str(path), cell, drawing),
        Archive(_, ref entry) =>
            make_scaled(&*entry.content.as_slice(), cell, drawing),
        Pdf(ref path, index) =>
            Ok(make_scaled_from_pdf(path_to_str(path), index, cell, drawing))
    }
}


pub fn get_animation_buffer(entry: &Entry) -> Result<AnimationBuffer, Error> {
    use self::EntryContent::*;

    match (*entry).content {
        File(ref path) | Http(ref path, _) =>
            AnimationBuffer::new_from_file(path),
        Archive(_, ref entry) =>
            Ok(AnimationBuffer::new_from_slice(&*entry.content)),
        _ => not_implemented!(),
    }
}

fn make_scaled(buffer: &[u8], cell: &Size, drawing: &DrawingState) -> Result<StaticImageBuffer, Error> {
    let loader = PixbufLoader::new();
    loader.loader_write(buffer).map_err(|it| s!(it)).and_then(|_| {
        if loader.close().is_err() {
            return Err(o!("Invalid image data"))
        }
        if let Some(source) = loader.get_pixbuf() {
            let original = Size::from_pixbuf(&source);
            let (scale, fitted) = original.fit(cell, &drawing.fit_to);
            let scaled = unsafe { Pixbuf::new(0, true, 8, fitted.width, fitted.height).unwrap() };
            source.scale(&scaled, 0, 0, fitted.width, fitted.height, 0.0, 0.0, scale, scale, drawing.scaling.0);
            Ok(StaticImageBuffer::new_from_pixbuf(&scaled))
        } else {
            Err(o!("Invalid image"))
        }
    })
}

fn make_scaled_from_file(path: &str, cell: &Size, drawing: &DrawingState) -> Result<StaticImageBuffer, Error> {
    File::open(path).map_err(|it| s!(it)).and_then(|mut file| {
        let mut buffer: Vec<u8> = vec![];
        file.read_to_end(&mut buffer).map_err(|it| s!(it)).and_then(|_| {
            make_scaled(buffer.as_slice(), cell, drawing)
        })
    })
}

fn make_scaled_from_pdf(pdf_path: &str, index: usize, cell: &Size, drawing: &DrawingState) -> StaticImageBuffer {
    let document = PopplerDocument::new_from_file(&pdf_path);
    StaticImageBuffer::new_from_pixbuf(&document.nth_page(index).get_pixbuf(cell, drawing))
}

fn get_meta(entry: &Entry) -> Option<Result<GenericMetadata, immeta::Error>> {
    use self::EntryContent::*;

    match (*entry).content {
        File(ref path) | Http(ref path, _) =>
            Some(immeta::load_from_file(&path)),
        Archive(_, ref entry) =>
            Some(immeta::load_from_buf(&entry.content)),
        Pdf(_,  _) =>
            None
    }
}

