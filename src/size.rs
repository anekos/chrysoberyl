
use std::str::FromStr;

use gdk_pixbuf::{Pixbuf, PixbufAnimation, PixbufAnimationExt};

use option;



#[derive(Clone, PartialEq, Eq)]
pub struct Size {
    pub width: i32,
    pub height: i32,
}

#[derive(Clone)]
pub struct Region<T> {
    pub left: T,
    pub top: T,
    pub right: T,
    pub bottom: T,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FitTo {
    Original,
    OriginalOrCell,
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

    pub fn from_pixbuf_animation(pixbuf: &PixbufAnimation) -> Size {
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

    pub fn clipped(&self, region: Region<f64>) -> (Size, Region<i32>) {
        let (w, h) = self.floated();
        let clipped_size = Size::new(
            (w * (region.right - region.left)) as i32,
            (h * (region.bottom - region.top)) as i32);
        let clipped_region = Region::new(
            (w * region.left) as i32,
            (h * region.top) as i32,
            (w * region.right) as i32,
            (h * region.bottom) as i32);
        (clipped_size, clipped_region)
    }

    /** returns (scale, fitted_size, delta) **/
    pub fn fit(&self, cell_size: &Size, fit_to: &FitTo) -> (f64, Size) {
        use self::FitTo::*;

        let (scale, fitted) = match *fit_to {
            Original => self.fit_to_original(),
            OriginalOrCell => self.fit_to_original_or_cell(cell_size),
            Cell => self.fit_to_cell(cell_size),
            Width => self.fit_to_width(cell_size),
            Height => self.fit_to_height(cell_size),
        };

        (scale, fitted)
    }

    pub fn fit_with_clipping(&self, cell_size: &Size, fit_to: &FitTo, clip: Option<Region<f64>>) -> (f64, Size, Option<Region<i32>>) {
        if let Some(clip) = clip {
            let (clipped_size, clipped_region) = self.clipped(clip);
            let (scale, fitted) = clipped_size.fit(cell_size, fit_to);
            (scale, fitted, Some(clipped_region))
        } else {
            let (scale, size) = self.fit(cell_size, fit_to);
            (scale, size, None)
        }
    }

    fn fit_to_original(&self) -> (f64, Size) {
        (1.0, self.clone())
    }

    fn fit_to_original_or_cell(&self, cell: &Size) -> (f64, Size) {
        let (scale, fitted) = self.fit_to_cell(cell);
        if 1.0 <= scale {
            self.fit_to_original()
        } else {
            (scale, fitted)
        }
    }

    fn fit_to_cell(&self, cell: &Size) -> (f64, Size) {
        let mut scale = cell.width as f64 / self.width as f64;
        let result_height = (self.height as f64 * scale) as i32;
        if result_height > cell.height {
            scale = cell.height as f64 / self.height as f64;
        }
        (scale, self.scaled(scale))
    }

    fn fit_to_width(&self, cell: &Size) -> (f64, Size) {
        let scale = cell.width as f64 / self.width as f64;
        (scale, self.scaled(scale))
    }

    fn fit_to_height(&self, cell: &Size) -> (f64, Size) {
        let scale = cell.height as f64 / self.height as f64;
        (scale, self.scaled(scale))
    }
}


impl<T> Region<T> {
    pub fn new(left: T, top: T, right: T, bottom: T) -> Region<T> {
        Region { left: left, top: top, right: right, bottom: bottom }
    }
}


impl FitTo {
    pub fn is_scrollable(&self) -> bool {
        *self != FitTo::Cell
    }
}

impl FromStr for FitTo {
    type Err = String;

    fn from_str(src: &str) -> Result<FitTo, String> {
        use self::FitTo::*;

        let result = match src {
            "original" => Original,
            "original-or-cell" | "cell-or-original" => OriginalOrCell,
            "cell" => Cell,
            "width" => Width,
            "height" => Height,
            _ => return Err(format!("Invalid target name: {}", src))
        };
        Ok(result)
    }
}

const FIT_TO_DEFAULT: &'static [FitTo] = &[FitTo::Original, FitTo::Cell, FitTo::OriginalOrCell, FitTo::Width, FitTo::Height];

impl option::OptionValue for FitTo {
    fn default_series<'a>() -> &'a [FitTo] {
        FIT_TO_DEFAULT
    }

    fn to_char(&self) -> char {
        use self::FitTo::*;

        match *self {
            Original => 'O',
            OriginalOrCell => 'o',
            Width => 'w',
            Height => 'h',
            Cell => 'c',
        }
    }
}
