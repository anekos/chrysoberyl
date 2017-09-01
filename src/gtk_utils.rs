
extern crate gdk_sys;
extern crate glib;
extern crate gobject_sys;

use std::f64::consts::PI;

use cairo::{ImageSurface, Context};
use gdk_pixbuf::Pixbuf;
use glib::translate::*;
use glib::translate::ToGlibPtr;

use size::Size;



pub fn new_pixbuf_from_surface(surface: &ImageSurface) -> Pixbuf {
    let (width, height) = (surface.get_width(), surface.get_height());

    unsafe {
        let surface = surface.as_ref().to_glib_none().0;
        from_glib_full(gdk_sys::gdk_pixbuf_get_from_surface(surface, 0, 0, width, height))
    }
}

pub fn context_rotate(context: &Context, page: &Size, rotation: u8) {
    let rotation = rotation % 4;

    match rotation {
        0 => (),
        1 => context.translate(page.height as f64, 0.0),
        2 => context.translate(page.width as f64, page.height as f64),
        3 => context.translate(0.0, page.width as f64),
        _ => panic!("WTF"),
    };

    if rotation > 0 {
        context.rotate(PI / 2.0 * rotation as f64);
    }
}
