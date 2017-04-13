
extern crate gobject_sys;

use std::ptr::{null, null_mut};
use std::ffi::CString;
use std::path::Path;
use std::mem::transmute;

use cairo;
use self::gobject_sys::GObject;
use glib::translate::ToGlibPtr;

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
}

impl Drop for PopplerDocument {
    fn drop(&mut self) {
        unsafe {
            let ptr = transmute::<*mut sys::document_t, *mut GObject>(self.0);
            gobject_sys::g_object_unref(ptr);
        }
    }
}

impl PopplerPage {
    pub fn render(&self, context: &mut cairo::Context) {
        let context = context.as_ref().to_glib_none().0;
        unsafe { sys::poppler_page_render(self.0, context) };
    }
}

impl Drop for PopplerPage {
    fn drop(&mut self) {
        unsafe {
            let ptr = transmute::<*mut sys::page_t, *mut GObject>(self.0);
            gobject_sys::g_object_unref(ptr);
        }
    }
}
