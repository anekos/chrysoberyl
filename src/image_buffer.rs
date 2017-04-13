
use std::fmt::Display;
use std::fs::File;
use std::io::Read;

use cairo::{Context, ImageSurface, Format};
use gdk_pixbuf::{Pixbuf, PixbufAnimation, PixbufLoader};
use gtk::Image;
use css_color_parser::Color;

use color::gdk_rgba;
use entry::{Entry, EntryContent};
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

    pub fn show(&self, image: &Image, width: i32, height: i32, fg: &Color, bg: &Color) {
        let text = &self.error;

        let surface = ImageSurface::create(Format::ARgb32, width, height);

        let (width, height) = (width as f64, height as f64);

        let context = Context::new(&surface);

        context.set_font_size(FONT_SIZE);
        let extents = context.text_extents(text);

        let (x, y) = (width / 2.0 - extents.width / 2.0, height / 2.0 - extents.height / 2.0);

        let bg = gdk_rgba(bg);
        context.set_source_rgba(bg.red, bg.green, bg.blue, bg.alpha);
        context.rectangle(
            x - PADDING,
            y - extents.height - PADDING,
            extents.width + PADDING * 2.0,
            extents.height + PADDING * 2.0);
        context.fill();

        context.move_to(x, y);
        let fg = gdk_rgba(fg);
        context.set_source_rgba(fg.red, fg.green, fg.blue, fg.alpha);
        context.show_text(text);

        image.set_from_surface(&surface);

        puts_error!("at" => "show_image", "reason" => text);
    }
}


pub fn get_pixbuf(entry: &Entry, width: i32, height: i32, fit: bool) -> Result<Pixbuf, Error> {
    use self::EntryContent::*;

    match (*entry).content {
        File(ref path) | Http(ref path, _) =>
            make_scaled_from_file(path_to_str(path), width, height, fit),
        Archive(_, ref entry) =>
            make_scaled(&*entry.content.as_slice(), width, height, fit),
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
    } .map_err(Error::new)
}

fn calculate_scale(pixbuf: &Pixbuf, max_width: i32, max_height: i32, fit: bool) -> (f64, i32, i32) {
    let (in_width, in_height) = (pixbuf.get_width(), pixbuf.get_height());

    if !fit && in_width <= max_width && in_height <= max_height {
        return (1.0, in_width, in_height)
    }

    let mut scale = max_width as f64 / in_width as f64;
    let mut out_height = (in_height as f64 * scale) as i32;
    if out_height > max_height {
        scale = max_height as f64 / in_height as f64;
        out_height = (in_height as f64 * scale) as i32;
    }
    (scale, (in_width as f64 * scale) as i32, out_height)
}

fn make_scaled(buffer: &[u8], max_width: i32, max_height: i32, fit: bool) -> Result<Pixbuf, Error> {
    use gdk_pixbuf::InterpType;

    let loader = PixbufLoader::new();
    loader.loader_write(buffer).map_err(Error::new).and_then(|_| {
        if loader.close().is_err() {
            return Err(Error::new("Invalid image data"))
        }
        if let Some(source) = loader.get_pixbuf() {
            let (scale, out_width, out_height) = calculate_scale(&source, max_width, max_height, fit);
            let scaled = unsafe { Pixbuf::new(0, true, 8, out_width, out_height).unwrap() };
            source.scale(&scaled, 0, 0, out_width, out_height, 0.0, 0.0, scale, scale, InterpType::Bilinear);
            Ok(scaled)
        } else {
            Err(Error::new("Invalid image"))
        }
    })
}

fn make_scaled_from_file(path: &str, max_width: i32, max_height: i32, fit: bool) -> Result<Pixbuf, Error> {
    File::open(path).map_err(Error::new).and_then(|mut file| {
        let mut buffer: Vec<u8> = vec![];
        file.read_to_end(&mut buffer).map_err(Error::new).and_then(|size| {
            make_scaled(buffer.as_slice(), max_width, max_height, fit)
        })
    })
}
