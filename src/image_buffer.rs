
use std::fmt::Display;
use std::fs::File;
use std::io::Read;
use std::rc::Rc;

use cairo::{Context, ImageSurface, Format};
use gdk_pixbuf::{Pixbuf, PixbufAnimation, PixbufLoader};
use immeta::markers::Gif;
use immeta::{self, GenericMetadata};

use color::Color;
use entry::{Entry, EntryContent};
use gtk_utils::new_pixbuf_from_surface;
use poppler::PopplerDocument;
use size::Size;
use state::{DrawingOption};
use utils::path_to_str;



const FONT_SIZE: f64 = 12.0;
const PADDING: f64 = 5.0;


#[derive(Clone)]
pub struct ImageData {
    pub size: Size,
    pub buffer: ImageBuffer,
}


#[derive(Clone)]
pub enum ImageBuffer {
    Static(Pixbuf),
    Animation(PixbufAnimation),
}


pub struct Error {
    pub error: String,
}


impl Error {
    pub fn new<T: Display>(error: T) -> Error {
        Error { error: s!(error) }
    }

    pub fn get_pixbuf(&self, cell: &Size, fg: &Color, bg: &Color) -> Pixbuf {
        let text = &self.error;

        let surface = ImageSurface::create(Format::ARgb32, cell.width, cell.height);

        let (width, height) = cell.floated();

        let context = Context::new(&surface);

        context.set_font_size(FONT_SIZE);
        let extents = context.text_extents(text);

        let (x, y) = (width / 2.0 - extents.width / 2.0, height / 2.0 - extents.height / 2.0);

        let bg = bg.gdk_rgba();
        context.set_source_rgba(bg.red, bg.green, bg.blue, bg.alpha);
        context.rectangle(
            x - PADDING,
            y - extents.height - PADDING,
            extents.width + PADDING * 2.0,
            extents.height + PADDING * 2.0);
        context.fill();

        context.move_to(x, y);
        let fg = fg.gdk_rgba();
        context.set_source_rgba(fg.red, fg.green, fg.blue, fg.alpha);
        context.show_text(text);

        puts_error!("at" => "show_image", "reason" => text);

        new_pixbuf_from_surface(&surface)
    }
}


pub fn get_image_data(entry: &Entry, cell: &Size, drawing: &DrawingOption) -> Result<ImageData, Error> {
    if let Some(img) = get_meta(entry) {
        if let Ok(img) = img {
            if let Ok(gif) = img.into::<Gif>() {
                if gif.is_animated() {
                    return get_pixbuf_animation(entry)
                }
            }
        }
    }
    get_pixbuf(entry, cell, drawing)
}


fn get_pixbuf(entry: &Entry, cell: &Size, drawing: &DrawingOption) -> Result<ImageData, Error> {
    use self::EntryContent::*;

    match (*entry).content {
        File(ref path) | Http(ref path, _) =>
            make_scaled_from_file(path_to_str(path), cell, drawing),
        Archive(_, ref entry) =>
            make_scaled(&*entry.content.as_slice(), cell, drawing),
        Pdf(_, ref document, index) =>
            make_scaled_from_pdf(document, index, cell, drawing)
    }
}


fn get_pixbuf_animation(entry: &Entry) -> Result<ImageData, Error> {
    use self::EntryContent::*;

    match (*entry).content {
        File(ref path) | Http(ref path, _) => {
            PixbufAnimation::new_from_file(path_to_str(path)).map(|pixbuf| {
                let size = Size::from_pixbuf_animation(&pixbuf);
                let buffer = ImageBuffer::Animation(pixbuf);
                ImageData { buffer: buffer, size: size }
            })
        }
        Archive(_, ref entry) => {
            let loader = PixbufLoader::new();
            loader.loader_write(&*entry.content.as_slice()).map(|_| {
                loader.close().unwrap();
                let buf = loader.get_animation().unwrap();
                ImageData {
                    size: Size::from_pixbuf_animation(&buf),
                    buffer: ImageBuffer::Animation(buf)
                }
            })
        }
        _ => not_implemented!(),
    } .map_err(Error::new)
}

fn make_scaled(buffer: &[u8], cell: &Size, drawing: &DrawingOption) -> Result<ImageData, Error> {
    let loader = PixbufLoader::new();
    loader.loader_write(buffer).map_err(Error::new).and_then(|_| {
        if loader.close().is_err() {
            return Err(Error::new("Invalid image data"))
        }
        if let Some(source) = loader.get_pixbuf() {
            let original = Size::from_pixbuf(&source);
            let (scale, fitted) = original.fit(cell, &drawing.fit_to);
            let scaled = unsafe { Pixbuf::new(0, true, 8, fitted.width, fitted.height).unwrap() };
            source.scale(&scaled, 0, 0, fitted.width, fitted.height, 0.0, 0.0, scale, scale, drawing.scaling.0);
            Ok(ImageData { size: original, buffer: ImageBuffer::Static(scaled) })
        } else {
            Err(Error::new("Invalid image"))
        }
    })
}

fn make_scaled_from_file(path: &str, cell: &Size, drawing: &DrawingOption) -> Result<ImageData, Error> {
    File::open(path).map_err(Error::new).and_then(|mut file| {
        let mut buffer: Vec<u8> = vec![];
        file.read_to_end(&mut buffer).map_err(Error::new).and_then(|_| {
            make_scaled(buffer.as_slice(), cell, drawing)
        })
    })
}

fn make_scaled_from_pdf(document: &Rc<PopplerDocument>, index: usize, cell: &Size, drawing: &DrawingOption) -> Result<ImageData, Error> {
    let pixbuf = document.nth_page(index).get_pixbuf(cell, drawing);
    let size = Size::from_pixbuf(&pixbuf);
    let buffer = ImageBuffer::Static(pixbuf);
    Ok(ImageData { buffer: buffer, size: size })
}

fn get_meta(entry: &Entry) -> Option<Result<GenericMetadata, immeta::Error>> {
    use self::EntryContent::*;

    match (*entry).content {
        File(ref path) | Http(ref path, _) =>
            Some(immeta::load_from_file(&path)),
        Archive(_, ref entry) =>
            Some(immeta::load_from_buf(&entry.content)),
        Pdf(_, _, _) =>
            None
    }
}

