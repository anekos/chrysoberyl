
extern crate gdk_sys;
extern crate glib;
extern crate gobject_sys;

#[cfg(feature = "poppler_lock")] use std::sync::{Arc, Mutex};
use std::ffi::CString;
use std::mem::transmute;
use std::path::Path;
use std::ptr::{null, null_mut};

use cairo::{Context, ImageSurface, Format};
use cairo;
use gdk_pixbuf::Pixbuf;
use glib::translate::ToGlibPtr;
use libc::{c_int, c_double};

use gtk_utils::new_pixbuf_from_surface;
use size::Size;
use state::DrawingState;

mod sys;


#[cfg(feature = "poppler_lock")]
lazy_static! {
    static ref LOCK: Arc<Mutex<usize>> = {
        #[cfg_attr(feature = "cargo-clippy", allow(mutex_atomic))]
        Arc::new(Mutex::new(0))
    };
}


#[derive(Debug, Eq, PartialEq, Hash, Clone, PartialOrd, Ord)]
pub struct PopplerDocument(*mut sys::document_t);
pub struct PopplerPage(*mut sys::page_t);



impl PopplerDocument {
    pub fn new_from_file<T: AsRef<Path>>(filepath: T) -> PopplerDocument {
        let filepath = filepath.as_ref().to_str().unwrap();
        let filepath = format!("file://{}", filepath);
        let filepath = CString::new(filepath).unwrap();
        let raw = unsafe {
            time!("poppler/new_from_file" => sys::poppler_document_new_from_file(filepath.as_ptr(), null(), null_mut()))
        };
        PopplerDocument(raw)
    }

    pub fn n_pages(&self) -> usize {
        unsafe {
            sys::poppler_document_get_n_pages(self.0) as usize
        }
    }

    pub fn nth_page(&self, index: usize) -> PopplerPage {
        let page = unsafe {
            time!("nth_page" => sys::poppler_document_get_page(self.0, index as c_int))
        };
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
        #[cfg(feature = "poppler_lock")]
        let mut count = (*LOCK).lock().unwrap();
        #[cfg(feature = "poppler_lock")]
        trace!("render/start: {:?}", *count);

        let context = context.as_ref().to_glib_none().0;
        unsafe { sys::poppler_page_render(self.0, context) };

        #[cfg(feature = "poppler_lock")]
        {
            trace!("render/end: {:?}", *count);
            *count += 1;
        }
    }

    pub fn get_size(&self) -> Size {
        let (mut width, mut height): (c_double, c_double) = (0.0, 0.0);
        unsafe { sys::poppler_page_get_size(self.0, &mut width, &mut height) };
        Size::new(width as i32, height as i32)
    }

    pub fn get_pixbuf(&self, cell: &Size, drawing: &DrawingState) -> Pixbuf {
        let page = self.get_size();

        let (scale, fitted, clipped_region) = page.fit_with_clipping(cell, drawing);
        let surface = ImageSurface::create(Format::ARgb32, fitted.width, fitted.height);

        {
            let context = Context::new(&surface);
            context.scale(scale, scale);
            context.set_source_rgb(1.0, 1.0, 1.0);
            if let Some(r) = clipped_region {
                context.translate(-r.left as f64, -r.top as f64);
                context.rectangle(r.left as f64, r.top as f64, r.right as f64, r.bottom as f64);
                context.clip();
            }
            context.paint();
            self.render(&context);
        }

        new_pixbuf_from_surface(&surface)
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
