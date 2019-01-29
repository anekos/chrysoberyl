
use cairo::{Context, ImageSurface, Format};
use gdk::Atom;
use gdk::prelude::ContextExt;
use gdk_pixbuf::{Pixbuf, PixbufExt};
use gtk::{Clipboard, ClipboardExt};

use crate::entry::Meta;
use crate::errors::{AppResult, ErrorKind};
use crate::expandable::Expandable;
use crate::operation::{Operation, ClipboardSelection};



pub fn get_operations(selection: ClipboardSelection, as_operation: bool, meta: Option<Meta>, force: bool, show: bool) -> AppResult<Vec<Operation>> {
    let cb = from_selection(selection);

    if let Some(pixbuf) = cb.wait_for_image() {
        let buffer = from_pixbuf(&pixbuf)?;
        return Ok(vec![Operation::PushMemory(buffer, meta, show)]);
    }

    {
        let uris = cb.wait_for_uris();
        if !uris.is_empty() {
            return Ok(uris.into_iter().enumerate().map(|(index, uri)| Operation::PushURL(uri, meta.clone(), force, show && index == 0, None)).collect())
        }
    }

    if let Some(text) = cb.wait_for_text() {
        let lines = text.lines();
        return Ok(lines.enumerate().flat_map(|(index, line)| {
            if as_operation {
                Operation::parse_fuzziness(line).map_err(|err| {
                    puts_error!(err, "operation" => o!(line), "at" => "clipboard/get_operations");
                })
            } else {
                Ok(Operation::Push(Expandable::expanded(o!(line)), meta.clone(), force, show && index == 0))
            }
        }).collect());
    }

    Err(ErrorKind::Fixed("Invalid clipboard"))?
}

pub fn store(selection: ClipboardSelection, pixbuf: &Pixbuf) {
    let cb = from_selection(selection);
    cb.set_image(pixbuf);
}

fn from_pixbuf(pixbuf: &Pixbuf) -> AppResult<Vec<u8>> {
    let (width, height) = (pixbuf.get_width(), pixbuf.get_height());
    let surface = ImageSurface::create(Format::ARgb32, width, height).unwrap();
    let context = Context::new(&surface);
    context.set_source_pixbuf(pixbuf, 0.0, 0.0);
    context.paint();
    let mut vec = Vec::<u8>::new();
    surface.write_to_png(&mut vec)?;
    Ok(vec)
}

fn from_selection(selection: ClipboardSelection) -> Clipboard {
    use self::ClipboardSelection::*;

    let name = match selection {
        Primary => "PRIMARY",
        Secondary => "SECONDARY",
        Clipboard => "CLIPBOARD",
    };

    self::Clipboard::get(&Atom::intern(name))
}
