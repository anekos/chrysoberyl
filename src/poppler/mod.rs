
extern crate gio_sys;
extern crate glib_sys;
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
use self::glib_sys::g_list_free;
use self::gio_sys::{g_file_new_for_path, GFile};
use self::gobject_sys::{GObject, g_object_unref};

use color::Color;
use gtk_utils::{new_pixbuf_from_surface, context_rotate};
use size::{Size, Region};
use state::Drawing;

mod sys;
mod util;
pub mod index;



#[cfg(feature = "poppler_lock")]
lazy_static! {
    static ref LOCK: Arc<Mutex<usize>> = {
        #[cfg_attr(feature = "cargo-clippy", allow(mutex_atomic))]
        Arc::new(Mutex::new(0))
    };
}


#[derive(Debug, Eq, PartialEq, Hash, PartialOrd, Ord)]
pub struct PopplerDocument(*const sys::document_t);

pub struct PopplerPage(*const sys::page_t);

pub struct File(*const GFile);

#[derive(Debug)]
pub struct Link {
    pub page: usize,
    pub region: Region,
}


impl PopplerDocument {
    pub fn new_from_file<T: AsRef<Path>>(filepath: T) -> PopplerDocument {
        let raw = unsafe {
            let file = File::new(filepath);
            time!("poppler/new_from_file" => sys::poppler_document_new_from_gfile(file.0, null(), null(), null_mut()))
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

    pub fn index(&self) -> index::Index {
        unsafe {
            let iter = sys::poppler_index_iter_new(self.0);
            let result = index::Index::new(iter);
            sys::poppler_index_iter_free(iter);
            result
        }
    }
}

impl Drop for PopplerDocument {
    fn drop(&mut self) {
        unsafe {
            let ptr = transmute::<*const sys::document_t, *mut GObject>(self.0);
            g_object_unref(ptr);
        }
    }
}

impl PopplerPage {
    #[cfg_attr(feature = "cargo-clippy", allow(many_single_char_names))]
    pub fn render(&self, context: &cairo::Context, link_color: Option<&Color>) {
        #[cfg(feature = "poppler_lock")]
        let mut count = (*LOCK).lock().unwrap();
        #[cfg(feature = "poppler_lock")]
        trace!("render/start: {:?}", *count);

        unsafe {
            let context = context.to_glib_none().0;
            sys::poppler_page_render(self.0, context);
        };

        if let Some(color) = link_color.and_then(Color::option) {
            let size = self.get_size();
            let (r, g, b, a) = color.tupled4();
            context.set_source_rgba(r, g, b, a);
            for link in self.get_links() {
                let (l, r, t, b) = link.region.absolute(size.width, size.height);
                let (l, r, w, h) = map!(f64, l, t, r - l, b - t);
                context.rectangle(l, r, w, h);
                context.fill();
            }
        }

        #[cfg(feature = "poppler_lock")]
        {
            trace!("render/end: {:?}", *count);
            *count += 1;
        }
    }

    pub fn get_png_data(&self, size: &Option<Size>) -> Vec<u8> {
        let page = self.get_size();
        let (scale, fitted) = match *size {
            Some(size) => page.fit_to_fixed(size.width, size.height),
            None => (1.0, page),
        };
        let surface = ImageSurface::create(Format::ARgb32, fitted.width, fitted.height).unwrap();
        let context = Context::new(&surface);
        context.scale(scale, scale);
        self.render(&context, None);
        let mut result = vec![];
        surface.write_to_png(&mut result).expect("get_png_data");
        result
    }

    pub fn get_size(&self) -> Size {
        let (mut width, mut height): (c_double, c_double) = (0.0, 0.0);
        unsafe { sys::poppler_page_get_size(self.0, &mut width, &mut height) };
        Size::new(width as i32, height as i32)
    }

    pub fn get_pixbuf(&self, cell: &Size, drawing: &Drawing) -> Pixbuf {
        let page = self.get_size();

        let (scale, fitted, clipped_region) = page.rotate(drawing.rotation).fit_with_clipping(cell, drawing);
        let surface = ImageSurface::create(Format::ARgb32, fitted.width, fitted.height).unwrap();

        {
            let context = Context::new(&surface);
            context.scale(scale, scale);
            context.set_source_rgb(1.0, 1.0, 1.0);
            if let Some(r) = clipped_region {
                context.translate(-r.left as f64, -r.top as f64);
                context.rectangle(r.left as f64, r.top as f64, r.right as f64, r.bottom as f64);
                context.clip();
            }
            context_rotate(&context, &page, drawing.rotation);
            context.paint();
            self.render(&context, Some(&drawing.link_color));
        }

        new_pixbuf_from_surface(&surface)
    }

    pub fn find_text(&self, text: &str) -> Vec<Region> {
        unsafe {
            let cstr = CString::new(text.as_bytes()).unwrap();
            let listed = sys::poppler_page_find_text(self.0, cstr.as_ptr());

            if listed.is_null() {
                return vec![];
            }

            let size = self.get_size();

            tap!(g_list_map!(it: *const sys::rectangle_t = listed => util::new_region_on(it, &size)), g_list_free(listed))
        }
    }

    pub fn get_links(&self) -> Vec<Link> {
        let mut result = vec![];

        unsafe {
            let listed = sys::poppler_page_get_link_mapping(self.0);

            if listed.is_null() {
                return result;
            }

            let size = self.get_size();

            g_list_for!(
                data: *const sys::link_mapping_t = listed =>
                if let Some(action) = util::extract_action(&*data.action) {
                    let page = action.page;
                    let region = util::new_region_on(&data.area, &size);
                    result.push(Link { page, region });
                });

            sys::poppler_page_free_link_mapping(listed);

            result
        }

    }
}

impl Drop for PopplerPage {
    fn drop(&mut self) {
        unsafe {
            let ptr = transmute::<*const sys::page_t, *mut GObject>(self.0);
            g_object_unref(ptr);
        }
    }
}


impl File {
    pub fn new<T: AsRef<Path>>(filepath: T) -> File {
        let filepath = filepath.as_ref().to_str().unwrap();
        let filepath = CString::new(filepath).unwrap();
        let g_file = unsafe {
            g_file_new_for_path(filepath.into_raw())
        };
        File(g_file)
    }
}

impl Drop for File {
    fn drop(&mut self) {
        unsafe {
            let ptr = transmute::<*const GFile, *mut GObject>(self.0);
            g_object_unref(ptr);
        }
    }
}
