
use gdk::RGBA;
use css_color_parser::Color;



pub fn tupled(color: &Color) -> (f64, f64, f64) {
    (to_f(color.r), to_f(color.g), to_f(color.b))
}

pub fn gdk_rgba(color: &Color) -> RGBA {
    RGBA {
        red: to_f(color.r),
        green: to_f(color.g),
        blue: to_f(color.b),
        alpha: color.a as f64
    }
}

fn to_f(v: u8) -> f64 {
    v as f64 / 255.0
}

