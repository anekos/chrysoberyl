
extern crate gdk_sys;

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
        1 => context.translate(f64!(page.height), 0.0),
        2 => context.translate(f64!(page.width), f64!(page.height)),
        3 => context.translate(0.0, f64!(page.width)),
        _ => panic!("WTF"),
    };

    if rotation > 0 {
        context.rotate(PI / 2.0 * f64!(rotation));
    }
}
