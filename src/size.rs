
use std::ops::Add;
use std::str::FromStr;

use gdk_pixbuf::{Pixbuf, PixbufAnimation, PixbufAnimationExt};

use option;
use state::DrawingOption;
use utils::feq;



#[derive(Clone, PartialEq, Eq, Copy, Debug)]
pub struct Size {
    pub width: i32,
    pub height: i32,
}

#[derive(Clone, Debug, Copy)]
pub struct Region {
    pub left: f64,
    pub top: f64,
    pub right: f64,
    pub bottom: f64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FitTo {
    Original,
    OriginalOrCell,
    Width,
    Height,
    Cell,
}


const FERROR: f64 = 0.000001;


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

    pub fn clipped(&self, region: &Region) -> (Size, Region) {
        let (w, h) = self.floated();
        let clipped_size = Size::new(
            (w * (region.right - region.left)) as i32,
            (h * (region.bottom - region.top)) as i32);
        let clipped_region = Region::new(
            w * region.left,
            h * region.top,
            w * region.right,
            h * region.bottom);
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

    pub fn fit_with_clipping(&self, cell_size: &Size, drawing: &DrawingOption) -> (f64, Size, Option<Region>) {
        if let Some(ref clip) = drawing.clipping {
            let (clipped_size, clipped_region) = self.clipped(clip);
            let (scale, fitted) = clipped_size.fit(cell_size, &drawing.fit_to);
            (scale, fitted, Some(clipped_region))
        } else {
            let (scale, size) = self.fit(cell_size, &drawing.fit_to);
            (scale, size, None)
        }
    }

    fn fit_to_original(&self) -> (f64, Size) {
        (1.0, *self)
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


impl Region {
    pub fn new(left: f64, top: f64, right: f64, bottom: f64) -> Region {
        Region {
            left: min!(left, right),
            top: min!(top, bottom),
            right: max!(left, right),
            bottom: max!(top, bottom),
        }
    }

    pub fn width(&self) -> f64 {
        self.right - self.left
    }

    pub fn height(&self) -> f64 {
        self.bottom - self.top
    }
}


impl Add for Region {
    type Output = Region;

    fn add(self, inner: Region) -> Region {
        Region::new(
            self.left + inner.left * self.width(),
            self.top + inner.top * self.height(),
            self.left + inner.right * self.width(),
            self.top + inner.bottom * self.height())
    }
}


impl Default for Region {
    fn default() -> Self {
        Region::new(0.0, 0.0, 1.0, 1.0)
    }
}

impl PartialEq for Region {
    fn eq(&self, other: &Region) -> bool {
        feq(self.left, other.left, FERROR) && feq(self.top, other.top, FERROR) && feq(self.right, other.right, FERROR) && feq(self.bottom, other.bottom, FERROR)
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
