
extern crate gdk_sys;

use std::f64::consts::PI;

use cairo::{ImageSurface, Context, Matrix, MatrixTrait};
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

pub fn context_flip(context: &Context, size: Size, horizontal: bool, vertical: bool) {
    if !(horizontal || vertical) {
        return;
    }
    let (mut xx, yx, xy, mut yy, mut x0, mut y0)  = (1.0, 0.0, 0.0, 1.0, 0.0, 0.0);
    if horizontal {
        yy = -1.0;
        y0 = f64!(size.height);
    }
    if vertical {
        xx = -1.0;
        x0 = f64!(size.width);
    }
    context.transform(Matrix::new(xx, yx, xy, yy, x0, y0));
}

pub fn context_rotate(context: &Context, page: Size, rotation: u8) {
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

macro_rules! g_list_for {
    ( $val:ident : $type:ty = $list:expr => $body:expr ) => {
        {
            let mut current = $list;
            while !current.is_null() {
                let $val = &*((*current).data as $type);
                $body;
                current = (*current).next;
            }
        }

    }
}

macro_rules! g_list_map {
    ( $val:ident : $type:ty = $list:expr => $body:expr ) => {
        {
            let mut result = vec![];
            g_list_for!($val: $type = $list => {
                let entry = $body;
                result.push(entry);
            });
            result
        }

    }
}

macro_rules! widget_case_clause {
    ($var:ident, $object:ident) => {
        ()
    };

    ($var:ident, $object:ident, $type:tt, $body:expr $(,$rtype:tt, $rbody:expr)*) => {
        let widget = &*($object as *const glib::Object as *const gtk::Widget);
        let ty = widget.get_path().get_object_type();
        if ty.is_a(&($type::static_type())) {
            let $var = &*($object as *const glib::Object as *const $type);
            $body;
        } else {
            widget_case_clause!($var, $object $(, $rtype, $rbody)*);
        }
    }
}

macro_rules! widget_case {
    ($var:ident = $object:expr, { $type:tt => $body:expr, $($rtype:tt => $rbody:expr,)* }) => {
        {
            let object = $object;
            widget_case_clause!($var, object, $type, $body $(, $rtype, $rbody)*);
        }
    }
}
