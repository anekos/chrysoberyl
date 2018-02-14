
use std::error::Error;

use cairo::{Context, ImageSurface, Format};
use gdk::Atom;
use gdk::prelude::ContextExt;
use gdk_pixbuf::Pixbuf;
use gtk::{Clipboard, ClipboardExt};

use entry::Meta;
use errors::ChryError;
use expandable::Expandable;
use operation::{Operation, ClipboardSelection};



pub fn get_operations(selection: &ClipboardSelection, meta: Option<Meta>, force: bool) -> Result<Vec<Operation>, Box<Error>> {
    let cb = from_selection(selection);

    if let Some(pixbuf) = cb.wait_for_image() {
        let buffer = from_pixbuf(&pixbuf)?;
        return Ok(vec![Operation::PushMemory(buffer, meta)]);
    }

    {
        let uris = cb.wait_for_uris();
        if !uris.is_empty() {
            return Ok(uris.into_iter().map(|uri| Operation::PushURL(uri, meta.clone(), force, None)).collect())
        }
    }

    if let Some(text) = cb.wait_for_text() {
        return Ok(text.lines().into_iter().map(|line| Operation::Push(Expandable::expanded(o!(line)), meta.clone(), force)).collect())
    }

    Err(ChryError::Fixed("Invalid clipboard"))?
}

pub fn store(selection: &ClipboardSelection, pixbuf: &Pixbuf) {
    let cb = from_selection(selection);
    cb.set_image(pixbuf);
}

fn from_pixbuf(pixbuf: &Pixbuf) -> Result<Vec<u8>, Box<Error>> {
    let (width, height) = (pixbuf.get_width(), pixbuf.get_height());
    let surface = ImageSurface::create(Format::ARgb32, width, height).unwrap();
    let context = Context::new(&surface);
    context.set_source_pixbuf(pixbuf, 0.0, 0.0);
    context.paint();
    let mut vec = Vec::<u8>::new();
    surface.write_to_png(&mut vec).map_err(ChryError::from)?;
    Ok(vec)
}

fn from_selection(selection: &ClipboardSelection) -> Clipboard {
    use self::ClipboardSelection::*;

    let name = match *selection {
        Primary => "PRIMARY",
        Secondary => "SECONDARY",
        Clipboard => "CLIPBOARD",
    };

    self::Clipboard::get(&Atom::intern(name))
}
