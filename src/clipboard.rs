
use std::error::Error;

use cairo::{Context, ImageSurface, Format};
use gdk::Atom;
use gdk::prelude::ContextExt;
use gdk_pixbuf::{Pixbuf, PixbufAnimationExt};
use gtk::{Clipboard, ClipboardExt};

use entry::Meta;
use errors::ChryError;
use operation::{Operation, ClipboardSelection};



pub fn get_operation(selection: &ClipboardSelection, meta: Option<Meta>) -> Result<Operation, Box<Error>> {
    let selection = from_selection(selection);
    let cb = Clipboard::get(&selection);

    if let Some(pixbuf) = cb.wait_for_image() {
        let buffer = from_pixbuf(&pixbuf)?;
        Ok(Operation::PushMemory(buffer, meta))
    } else {
        Err(ChryError::Fixed("Invalid clipboard"))?
    }
}

fn from_pixbuf(pixbuf: &Pixbuf) -> Result<Vec<u8>, Box<Error>> {

    let (width, height) = (pixbuf.get_width(), pixbuf.get_height());
    let surface = ImageSurface::create(Format::ARgb32, width, height).unwrap();
    let context = Context::new(&surface);
    context.set_source_pixbuf(&pixbuf, 0.0, 0.0);
    context.paint();
    let mut vec = Vec::<u8>::new();
    surface.write_to_png(&mut vec).map_err(ChryError::from)?;

    Ok(vec)
}

fn from_selection(selection: &ClipboardSelection) -> Atom {
    use self::ClipboardSelection::*;

    let name = match *selection {
        Primary => "PRIMARY",
        Secondary => "SECONDARY",
        Clipboard => "CLIPBOARD",
    };

    Atom::intern(name)
}
