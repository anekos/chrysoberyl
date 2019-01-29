
use std::fs::File;
use std::io::Read;
use std::path::Path;

use cairo::{Context, ImageSurface, Format};
use gdk::prelude::ContextExt;
use gdk_pixbuf::{PixbufLoader, PixbufLoaderExt};
use immeta::markers::Gif;
use immeta::{self, GenericMetadata};

use crate::entry::EntryContent;
use crate::errors::{AppResult, ErrorKind};
use crate::gtk_utils::{new_pixbuf_from_surface, context_flip, context_rotate};
use crate::image::{ImageBuffer, StaticImageBuffer, AnimationBuffer};
use crate::poppler::PopplerDocument;
use crate::size::Size;
use crate::state::Drawing;
use crate::util::path::path_to_str;



#[derive(Clone, Default, Eq, Hash, PartialEq)]
pub struct Imaging {
    pub cell_size: Size,
    pub drawing: Drawing,
}

impl Imaging {
    pub fn new(cell_size: Size, drawing: Drawing) -> Imaging {
        Imaging { cell_size, drawing }
    }
}

pub fn get_image_buffer(entry_content: &EntryContent, imaging: &Imaging) -> AppResult<ImageBuffer> {
    if imaging.drawing.animation && is_animation(entry_content) {
        Ok(get_animation_buffer(entry_content).map(ImageBuffer::Animation)?)
    } else {
        get_static_image_buffer(entry_content, imaging).map(ImageBuffer::Static)
    }
}


pub fn get_static_image_buffer(entry_content: &EntryContent, imaging: &Imaging) -> AppResult<StaticImageBuffer> {
    use self::EntryContent::*;

    match *entry_content {
        Image(ref path) =>
            make_scaled_from_file(path_to_str(path), &imaging),
        Archive(_, ref entry) =>
            make_scaled(&*entry.content.as_slice(), &imaging),
        Memory(ref content, _) =>
            make_scaled(content, &imaging),
        Pdf(ref path, index) =>
            Ok(make_scaled_from_pdf(&**path, index, &imaging)),
        Message(ref message) =>
            Err(ErrorKind::Standard(o!(message)))?,
    }
}


pub fn get_animation_buffer(entry_content: &EntryContent) -> AppResult<AnimationBuffer> {
    use self::EntryContent::*;

    match *entry_content {
        Image(ref path) =>
            Ok(AnimationBuffer::new_from_file(path)?),
        Archive(_, ref entry) =>
            Ok(AnimationBuffer::new_from_slice(&*entry.content)),
        _ => Err(ErrorKind::Fixed("Not implemented: get_animation_buffer"))?,
    }
}


fn is_animation(entry_content: &EntryContent) -> bool {
    if let Some(img) = get_meta(entry_content) {
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

fn make_scaled(buffer: &[u8], imaging: &Imaging) -> AppResult<StaticImageBuffer> {
    let loader = PixbufLoader::new();
    loader.write(buffer)?;

    if loader.close().is_err() {
        return Err(ErrorKind::Fixed("Invalid image data"))?
    }

    let source = loader.get_pixbuf().ok_or_else(|| ErrorKind::Fixed("Invalid image"))?;
    let original = Size::from_pixbuf(&source);
    let (scale, fitted, clipped_region) = original.rotate(imaging.drawing.rotation).fit_with_clipping(imaging.cell_size, &imaging.drawing);

    let result = {
        let surface = ImageSurface::create(Format::ARgb32, fitted.width, fitted.height).unwrap();
        let context = Context::new(&surface);
        context.scale(scale, scale);
        if let Some(r) = clipped_region {
            context.translate(-r.left as f64, -r.top as f64);
            context.rectangle(r.left as f64, r.top as f64, r.right as f64, r.bottom as f64);
            context.clip();
        }
        context_rotate(&context, original, imaging.drawing.rotation);
        context_flip(&context, original, imaging.drawing.horizontal_flip, imaging.drawing.vertical_flip);
        context.set_source_pixbuf(&source, 0.0, 0.0);
        context.paint();
        new_pixbuf_from_surface(&surface)
    };

    Ok(StaticImageBuffer::new_from_pixbuf(&result, Some(original)))
}

fn make_scaled_from_file(path: &str, imaging: &Imaging) -> AppResult<StaticImageBuffer> {
    let mut file = File::open(path)?;
    let mut buffer: Vec<u8> = vec![];
    let _ = file.read_to_end(&mut buffer)?;
    make_scaled(buffer.as_slice(), imaging)
}

fn make_scaled_from_pdf<T: AsRef<Path>>(pdf_path: &T, index: usize, imaging: &Imaging) -> StaticImageBuffer {
    let document = PopplerDocument::new_from_file(pdf_path);
    let page = document.nth_page(index);
    let pixbuf = page.get_pixbuf(imaging.cell_size, &imaging.drawing);
    let size = page.get_size();
    StaticImageBuffer::new_from_pixbuf(&pixbuf, Some(size))
}

fn get_meta(entry_content: &EntryContent) -> Option<Result<GenericMetadata, immeta::Error>> {
    use self::EntryContent::*;

    match *entry_content {
        Image(ref path) =>
            Some(immeta::load_from_file(&path)),
        Archive(_, ref entry) =>
            Some(immeta::load_from_buf(&entry.content)),
        Memory(ref content, _) =>
            Some(immeta::load_from_buf(content)),
        Pdf(_,  _) | Message(_) =>
            None
    }
}

