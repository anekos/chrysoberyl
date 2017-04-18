
extern crate gdk_sys;
extern crate glib;
extern crate gobject_sys;

use std::ptr::{null, null_mut};
use std::ffi::CString;
use std::path::Path;
use std::mem::transmute;

use cairo::{Context, ImageSurface, Format};
use cairo;
use gdk_pixbuf::Pixbuf;
use glib::translate::*;
use glib::translate::ToGlibPtr;
use libc::{c_int, c_double};

use size::Size;

mod sys;



#[derive(Debug, Eq, PartialEq, Hash, Clone, PartialOrd, Ord)]
pub struct PopplerDocument(*mut sys::document_t);
pub struct PopplerPage(*mut sys::page_t);



impl PopplerDocument {
    pub fn new_from_file<T: AsRef<Path>>(filepath: T) -> PopplerDocument {
        let filepath = filepath.as_ref().to_str().unwrap();
        let filepath = format!("file://{}", filepath);
        let filepath = CString::new(filepath).unwrap();
        let raw = unsafe { sys::poppler_document_new_from_file(filepath.as_ptr(), null(), null_mut()) };
        PopplerDocument(raw)
    }

    pub fn n_pages(&self) -> usize {
        unsafe {
            sys::poppler_document_get_n_pages(self.0) as usize
        }
    }

    pub fn nth_page(&self, index: usize) -> PopplerPage {
        let page = unsafe { sys::poppler_document_get_page(self.0, index as c_int) };
        PopplerPage(page)
    }
}

impl Drop for PopplerDocument {
    fn drop(&mut self) {
        unsafe {
            let ptr = transmute::<*mut sys::document_t, *mut gobject_sys::GObject>(self.0);
            gobject_sys::g_object_unref(ptr);
        }
    }
}

impl PopplerPage {
    pub fn render(&self, context: &cairo::Context) {
        let context = context.as_ref().to_glib_none().0;
        unsafe { sys::poppler_page_render(self.0, context) };
    }

    pub fn get_size(&self) -> Size {
        let (mut width, mut height): (c_double, c_double) = (0.0, 0.0);
        unsafe { sys::poppler_page_get_size(self.0, &mut width, &mut height) };
        Size::new(width as i32, height as i32)
    }

    pub fn get_pixbuf(&self, cell: &Size) -> Pixbuf {
        let page = self.get_size();

        let (scale, fitted) = page.fit(cell);
        let surface = ImageSurface::create(Format::ARgb32, fitted.width, fitted.height);

        {
            let context = Context::new(&surface);
            context.scale(scale, scale);
            context.set_source_rgb(1.0, 1.0, 1.0);
            context.paint();
            self.render(&context);
        }

        let (width, height) = (surface.get_width(), surface.get_height());

        unsafe {
            let surface = surface.as_ref().to_glib_none().0;
            from_glib_full(gdk_sys::gdk_pixbuf_get_from_surface(surface, 0, 0, width, height))
        }
    }
}

impl Drop for PopplerPage {
    fn drop(&mut self) {
        unsafe {
            let ptr = transmute::<*mut sys::page_t, *mut gobject_sys::GObject>(self.0);
            gobject_sys::g_object_unref(ptr);
        }
    }
}
