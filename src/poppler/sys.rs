
extern crate cairo_sys;
extern crate glib;
extern crate glib_sys;
extern crate gobject_sys;
extern crate libc;

use libc::{c_int, c_char, c_double, c_void};



#[repr(C)]
pub struct document_t(c_void);
#[repr(C)]
pub struct page_t(c_void);


#[link(name = "poppler")]
extern "C" {
    pub fn poppler_document_new_from_file(uri: *const c_char, password: *const c_char, error: *mut *mut glib_sys::GError) -> *mut document_t;
    pub fn poppler_document_get_n_pages(doc: *const document_t) -> c_int;
    pub fn poppler_document_get_page(doc: *const document_t, index: c_int) -> *mut page_t;

    pub fn poppler_page_render(page: *const page_t, cairo: *mut cairo_sys::cairo_t);
    pub fn poppler_page_get_size(page: *const page_t, width: *mut c_double, height: *mut c_double);
}
