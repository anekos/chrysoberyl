
use std::fmt::Display;
use std::fs::File;
use std::io::Read;
use std::rc::Rc;

use cairo::{Context, ImageSurface, Format};
use gdk_pixbuf::{Pixbuf, PixbufAnimation, PixbufLoader};

use color::Color;
use entry::{Entry, EntryContent};
use gtk_utils::new_pixbuf_from_surface;
use poppler::PopplerDocument;
use size::{FitTo, Size};
use state::ScalingMethod;
use utils::path_to_str;



const FONT_SIZE: f64 = 12.0;
const PADDING: f64 = 5.0;



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


pub fn get_pixbuf(entry: &Entry, cell: &Size, fit: &FitTo, scaling: &ScalingMethod) -> Result<Pixbuf, Error> {
    use self::EntryContent::*;

    match (*entry).content {
        File(ref path) | Http(ref path, _) =>
            make_scaled_from_file(path_to_str(path), cell, fit, scaling),
        Archive(_, ref entry) =>
            make_scaled(&*entry.content.as_slice(), cell, fit, scaling),
        Pdf(_, ref document, index) =>
            make_scaled_from_pdf(document, index, cell, fit)
    }
}


pub fn get_pixbuf_animation(entry: &Entry) -> Result<PixbufAnimation, Error> {
    use self::EntryContent::*;

    match (*entry).content {
        File(ref path) | Http(ref path, _) =>
            PixbufAnimation::new_from_file(path_to_str(path)),
        Archive(_, ref entry) => {
            let loader = PixbufLoader::new();
            loader.loader_write(&*entry.content.as_slice()).map(|_| {
                loader.close().unwrap();
                loader.get_animation().unwrap()
            })
        }
        _ => not_implemented!(),
    } .map_err(Error::new)
}

fn make_scaled(buffer: &[u8], cell: &Size, fit: &FitTo, scaling: &ScalingMethod) -> Result<Pixbuf, Error> {
    let loader = PixbufLoader::new();
    loader.loader_write(buffer).map_err(Error::new).and_then(|_| {
        if loader.close().is_err() {
            return Err(Error::new("Invalid image data"))
        }
        if let Some(source) = loader.get_pixbuf() {
            let (scale, fitted, _) = Size::from_pixbuf(&source).fit(cell, fit);
            let scaled = unsafe { Pixbuf::new(0, true, 8, fitted.width, fitted.height).unwrap() };
            source.scale(&scaled, 0, 0, fitted.width, fitted.height, 0.0, 0.0, scale, scale, scaling.0);
            Ok(scaled)
        } else {
            Err(Error::new("Invalid image"))
        }
    })
}

fn make_scaled_from_file(path: &str, cell: &Size, fit: &FitTo, scaling: &ScalingMethod) -> Result<Pixbuf, Error> {
    File::open(path).map_err(Error::new).and_then(|mut file| {
        let mut buffer: Vec<u8> = vec![];
        file.read_to_end(&mut buffer).map_err(Error::new).and_then(|_| {
            make_scaled(buffer.as_slice(), cell, fit, scaling)
        })
    })
}

fn make_scaled_from_pdf(document: &Rc<PopplerDocument>, index: usize, cell: &Size, fit: &FitTo) -> Result<Pixbuf, Error> {
    Ok(document.nth_page(index).get_pixbuf(cell, fit))
}

