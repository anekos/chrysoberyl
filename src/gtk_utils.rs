
extern crate gdk_sys;
extern crate glib;
extern crate gobject_sys;

use cairo::ImageSurface;
use gdk_pixbuf::Pixbuf;
use glib::translate::*;
use glib::translate::ToGlibPtr;



pub fn new_pixbuf_from_surface(surface: &ImageSurface) -> Pixbuf {
    let (width, height) = (surface.get_width(), surface.get_height());

    unsafe {
        let surface = surface.as_ref().to_glib_none().0;
        from_glib_full(gdk_sys::gdk_pixbuf_get_from_surface(surface, 0, 0, width, height))
    }
}
