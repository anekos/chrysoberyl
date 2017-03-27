
use cairo::{Context, ImageSurface, Format};
use gdk_pixbuf::{Pixbuf, PixbufAnimation, PixbufLoader};
use gtk::Image;
use gtk;

use entry::Entry;
use utils::path_to_str;



const FONT_SIZE: f64 = 12.0;
const PADDING: f64 = 5.0;
const ALPHA: f64 = 1.0;



pub struct Error {
    pub surface: ImageSurface,
    pub error: gtk::Error,
}


impl Error {
    pub fn new(width: i32, height: i32, error: gtk::Error) -> Error {
        let message = write_message(width, height, &s!(error));
        Error { error: error, surface: message }
    }

    pub fn show(&self, image: &Image) {
        image.set_from_surface(&self.surface);
        puts_error!("at" => "show_image", "reason" => s!(self.error));
    }
}



pub fn get_pixbuf(entry: &Entry, width: i32, height: i32) -> Result<Pixbuf, Error> {
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
    } .map_err(|err| Error::new(width, height, err))
}

pub fn get_pixbuf_animation(entry: &Entry, width: i32, height: i32) -> Result<PixbufAnimation, Error> {
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
    } .map_err(|err| Error::new(width, height, err))
}

fn write_message(width: i32, height: i32, text: &str) -> ImageSurface {
    let surface = ImageSurface::create(Format::ARgb32, width, height);

    let (width, height) = (width as f64, height as f64);

    let context = Context::new(&surface);

    context.set_font_size(FONT_SIZE);
    let extents = context.text_extents(text);

    let (x, y) = (width / 2.0 - extents.width / 2.0, height / 2.0 - extents.height / 2.0);

    context.set_source_rgba(1.0, 0.00, 0.00, ALPHA);
    context.rectangle(
        x - PADDING,
        y - extents.height - PADDING,
        extents.width + PADDING * 2.0,
        extents.height + PADDING * 2.0);
    context.fill();

    context.move_to(x, y);
    context.set_source_rgba(1.0, 1.0, 1.0, ALPHA);
    context.show_text(text);

    surface
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