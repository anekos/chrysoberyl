
use std::error;
use std::fs::File;
use std::io::Read;
use std::path::Path;

use cairo::{Context, ImageSurface, Format};
use gdk::prelude::ContextExt;
use gdk_pixbuf::PixbufLoader;
use immeta::markers::Gif;
use immeta::{self, GenericMetadata};

use entry::{Entry, EntryContent};
use errors::ChryError;
use gtk_utils::{new_pixbuf_from_surface, context_rotate};
use image::{ImageBuffer, StaticImageBuffer, AnimationBuffer};
use poppler::PopplerDocument;
use size::Size;
use state::DrawingState;
use util::path::path_to_str;



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

pub fn get_image_buffer(entry: &Entry, cell: &Size, drawing: &DrawingState) -> Result<ImageBuffer, Box<error::Error>> {
    if drawing.animation && is_animation(entry) {
        Ok(get_animation_buffer(entry).map(ImageBuffer::Animation)?)
    } else {
        get_static_image_buffer(entry, cell, drawing).map(ImageBuffer::Static)
    }
}


pub fn get_static_image_buffer(entry: &Entry, cell: &Size, drawing: &DrawingState) -> Result<StaticImageBuffer, Box<error::Error>> {
    use self::EntryContent::*;

    match (*entry).content {
        Image(ref path) =>
            make_scaled_from_file(path_to_str(path), cell, drawing),
        Archive(_, ref entry) =>
            make_scaled(&*entry.content.as_slice(), cell, drawing),
        Memory(ref content, _) =>
            make_scaled(content, cell, drawing),
        Pdf(ref path, index) =>
            Ok(make_scaled_from_pdf(&**path, index, cell, drawing))
    }
}


pub fn get_animation_buffer(entry: &Entry) -> Result<AnimationBuffer, Box<error::Error>> {
    use self::EntryContent::*;

    match (*entry).content {
        Image(ref path) =>
            Ok(AnimationBuffer::new_from_file(path)?),
        Archive(_, ref entry) =>
            Ok(AnimationBuffer::new_from_slice(&*entry.content)),
        _ => Err(Box::new(ChryError::Fixed("Not implemented: get_animation_buffer"))),
    }
}

fn make_scaled(buffer: &[u8], cell: &Size, drawing: &DrawingState) -> Result<StaticImageBuffer, Box<error::Error>> {
    let loader = PixbufLoader::new();
    loader.loader_write(buffer)?;

    if loader.close().is_err() {
        return Err(Box::new(ChryError::Fixed("Invalid image data")))
    }

    let source = loader.get_pixbuf().ok_or_else(|| Box::new(ChryError::Fixed("Invalid image")))?;
    let original = Size::from_pixbuf(&source);
    let (scale, fitted, clipped_region) = original.rotate(drawing.rotation).fit_with_clipping(cell, drawing);

    let result = {
        let surface = ImageSurface::create(Format::ARgb32, fitted.width, fitted.height).unwrap();
        let context = Context::new(&surface);
        context.scale(scale, scale);
        if let Some(r) = clipped_region {
            context.translate(-r.left as f64, -r.top as f64);
            context.rectangle(r.left as f64, r.top as f64, r.right as f64, r.bottom as f64);
            context.clip();
        }
        context_rotate(&context, &original, drawing.rotation);
        context.set_source_pixbuf(&source, 0.0, 0.0);
        context.paint();
        new_pixbuf_from_surface(&surface)
    };

    Ok(StaticImageBuffer::new_from_pixbuf(&result, Some(original)))
}

fn make_scaled_from_file(path: &str, cell: &Size, drawing: &DrawingState) -> Result<StaticImageBuffer, Box<error::Error>> {
    let mut file = File::open(path)?;
    let mut buffer: Vec<u8> = vec![];
    let _ = file.read_to_end(&mut buffer)?;
    make_scaled(buffer.as_slice(), cell, drawing)
}

fn make_scaled_from_pdf<T: AsRef<Path>>(pdf_path: &T, index: usize, cell: &Size, drawing: &DrawingState) -> StaticImageBuffer {
    let document = PopplerDocument::new_from_file(pdf_path);
    let page = document.nth_page(index);
    let pixbuf = page.get_pixbuf(cell, drawing);
    let size = page.get_size();
    StaticImageBuffer::new_from_pixbuf(&pixbuf, Some(size))
}

fn get_meta(entry: &Entry) -> Option<Result<GenericMetadata, immeta::Error>> {
    use self::EntryContent::*;

    match (*entry).content {
        Image(ref path) =>
            Some(immeta::load_from_file(&path)),
        Archive(_, ref entry) =>
            Some(immeta::load_from_buf(&entry.content)),
        Memory(ref content, _) =>
            Some(immeta::load_from_buf(content)),
        Pdf(_,  _) =>
            None
    }
}

