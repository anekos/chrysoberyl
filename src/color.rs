
use std::fmt;
use std::str::FromStr;

use css_color_parser::Color as CssColor;
use rand::{thread_rng, Rng};

use crate::errors::{AppResult, AppError};



#[derive(Debug, Clone, PartialEq, Eq, Copy, Hash)]
pub struct Color {
    r: u8,
    g: u8,
    b: u8,
    a: u8,
}


impl Color {
    pub fn black() -> Color {
        Color::new(0, 0, 0)
    }

    pub fn new(r: u8, g: u8, b: u8) -> Color {
        Color { r, g, b, a: 255 }
    }

    pub fn new4(r: u8, g: u8, b: u8, a: u8) -> Color {
        Color { r, g, b, a }
    }

    pub fn new_random() -> Color {
        let mut rng = thread_rng();
        Color::new(rng.gen(), rng.gen(), rng.gen())
    }

    pub fn new_from_css_color(css_color: CssColor) -> Color {
        Color::new4(css_color.r, css_color.g, css_color.b, min!(css_color.a * 255.0, 255.0) as u8)
    }

    pub fn tupled3(self) -> (f64, f64, f64) {
        (to_f(self.r), to_f(self.g), to_f(self.b))
    }

    pub fn tupled4(self) -> (f64, f64, f64, f64) {
        (to_f(self.r), to_f(self.g), to_f(self.b), to_f(self.a))
    }

    pub fn option(self) -> Option<Color> {
        if self.a == 0 {
            None
        } else {
            Some(self)
        }

    }
}


impl FromStr for Color {
    type Err = AppError;

    fn from_str(src: &str) -> AppResult<Color> {
        match src {
            "random" => Ok(Color::new_random()),
            _ => Ok(src.parse().map(Color::new_from_css_color)?)
        }
    }
}

impl fmt::Display for Color {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "rgba({}, {}, {}, {})", self.r, self.g, self.b, f32!(self.a) / 255.0 )
    }
}



fn to_f(v: u8) -> f64 {
    f64!(v) / 255.0
}

