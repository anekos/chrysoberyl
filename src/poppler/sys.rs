
extern crate cairo_sys;
extern crate glib_sys;

use libc::{c_int, c_char, c_double, c_void, c_uint};
use self::glib_sys::{gboolean, GList};

#[repr(C)]
pub struct document_t(c_void);
#[repr(C)]
pub struct page_t(c_void);
#[repr(C)]
pub struct page_index_iter_t(c_void);
#[repr(C)]
pub struct action_t {
    pub action_type: c_char, // only for 2
    pub title: *const c_char,
    pub dest: *const dest_t,
}
#[repr(C)]
pub struct dest_t {
    pub dest_type: c_char,
    pub page: c_int,
    pub left: c_double,
    pub bottom: c_double,
    pub right: c_double,
    pub top: c_double,
    pub zoom: c_double,
    pub named_dest: *const c_char,
    pub change_left: c_uint,
    pub change_top: c_uint,
    pub change_zoom: c_uint,
}
#[repr(C)]#[derive(Debug)]
pub struct rectangle_t {
  pub x1: c_double, /* gdouble */
  pub y1: c_double, /* gdouble */
  pub x2: c_double, /* gdouble */
  pub y2: c_double, /* gdouble */
}


#[link(name = "poppler")]
extern "C" {
    pub fn poppler_document_new_from_file(uri: *const c_char, password: *const c_char, error: *const *const glib_sys::GError) -> *const document_t;
    pub fn poppler_document_get_n_pages(doc: *const document_t) -> c_int;
    pub fn poppler_document_get_page(doc: *const document_t, index: c_int) -> *const page_t;

    pub fn poppler_page_render(page: *const page_t, cairo: *const cairo_sys::cairo_t);
    pub fn poppler_page_get_size(page: *const page_t, width: *mut c_double, height: *mut c_double);
    pub fn poppler_page_find_text(page: *const page_t, text: *const c_char) -> *mut GList;

    pub fn poppler_index_iter_new(doc: *const document_t) -> *const page_index_iter_t;
    pub fn poppler_index_iter_free(iter: *const page_index_iter_t);
    pub fn poppler_index_iter_get_child(iter: *const page_index_iter_t) -> *const page_index_iter_t;
    // pub fn poppler_index_iter_is_open(iter: *const page_index_iter_t) -> gboolean;
    pub fn poppler_index_iter_get_action(iter: *const page_index_iter_t) -> *const action_t;
    pub fn poppler_index_iter_next(iter: *const page_index_iter_t) -> gboolean;

    pub fn poppler_action_free(action: *const action_t);

    // pub fn poppler_document_find_dest(doc: *const document_t, link_name: *const c_char) -> *mut dest_t;
    // pub fn poppler_dest_free (dest: *const dest_t);
}
