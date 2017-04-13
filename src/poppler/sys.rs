
extern crate libc;
extern crate gobject_sys;
extern crate cairo_sys;

#[allow(unused_imports)]
use libc::{c_int, c_char, c_uchar, c_float, c_uint, c_double, c_short, c_ushort, c_long, c_ulong, c_void, size_t, ssize_t, time_t, FILE};

use self::cairo_sys::cairo_t;
use glib;


#[repr(C)]
pub struct document_t(c_void);
#[repr(C)]
pub struct page_t(c_void);


#[link(name = "poppler")]
extern "C" {
    pub fn poppler_document_new_from_file(uri: *const c_char, password: *const c_char, error: *mut *mut glib::Error) -> *mut document_t;
    pub fn poppler_document_get_n_pages(doc: *const document_t) -> c_int;
    pub fn poppler_document_get_page(doc: *const document_t, index: c_int) -> *mut page_t;

    pub fn poppler_page_render(page: *const page_t, cairo: *mut cairo_t);
    pub fn poppler_page_get_size(page: *const page_t, width: *mut c_double, height: *mut c_double);
}


#[cfg(test)]#[test]
fn test_call() {
    use std::ptr::{null, null_mut};
    use std::ffi::CString;
    use std::mem::transmute;
    use self::gobject_sys::GObject;

    let p = CString::new("file:///tmp/xmosh/foo.pdf").unwrap();
    unsafe {
        let doc = poppler_document_new_from_file(p.as_ptr(), null(), null_mut());
        let n_pages = poppler_document_get_n_pages(doc);

        let doc = transmute::<*mut document_t, *mut GObject>(doc);
        // let doc = transmute::<u64, *mut GObject>(1111111);
        gobject_sys::g_object_unref(doc);

        assert_eq!(n_pages, 264);
    }
}
