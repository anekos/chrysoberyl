
use std::str::FromStr;

use gdk_pixbuf::Pixbuf;



#[derive(Clone, PartialEq, Eq)]
pub struct Size {
    pub width: i32,
    pub height: i32
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FitTo {
    Original,
    Width,
    Height,
    Cell,
}


impl Size {
    pub fn new(width: i32, height: i32) -> Size {
        Size { width: width, height: height }
    }

    pub fn from_pixbuf(pixbuf: &Pixbuf) -> Size {
        Size {
            width: pixbuf.get_width(),
            height: pixbuf.get_height(),
        }
    }

    pub fn floated(&self) -> (f64, f64) {
        (self.width as f64, self.height as f64)
    }

    pub fn scaled(&self, scale: f64) -> Size {
        Size {
            width: (self.width as f64 * scale) as i32,
            height: (self.height as f64 * scale) as i32,
        }
    }

    pub fn fit(&self, cell: &Size, to: &FitTo) -> (f64, Size) {
        let mut scale = cell.width as f64 / self.width as f64;
        let result_height = (self.height as f64 * scale) as i32;
        if result_height > cell.height {
            scale = cell.height as f64 / self.height as f64;
        }
        if *to == FitTo::Original && 1.0 <= scale {
            (1.0, self.clone())
        } else {
            (scale, self.scaled(scale))
        }
    }
}


impl FromStr for FitTo {
    type Err = String;

    fn from_str(src: &str) -> Result<FitTo, String> {
        use self::FitTo::*;

        let result = match src {
            "original" => Original,
            "cell" => Cell,
            "width" => Width,
            "height" => Height,
            _ => return Err(format!("Invalid target name: {}", src))
        };
        Ok(result)
    }
}
