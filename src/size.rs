
use std::cmp::Ordering;
use std::fmt;
use std::ops::Add;
use std::str::FromStr;

use gdk_pixbuf::{Pixbuf, PixbufExt};

use resolution;
use state::DrawingState;
use util::num::feq;



#[derive(Debug)]
pub struct Coord {
    pub x: f64,
    pub y: f64,
}

#[derive(Eq, PartialEq, Hash, Clone, Debug, Copy)]
pub struct CoordPx {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

#[derive(Clone, PartialEq, Eq, Copy, Debug, Default, Ord)]
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

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum FitTo {
    Original,
    OriginalOrCell,
    Width,
    Height,
    Cell,
    Fixed(i32, i32),
    Scale(usize),
}


const FERROR: f64 = 0.000_001;
const MINIMUM_SCALE: usize = 10;
const MAXIMUM_SCALE: usize = 1000;


impl Coord {
    pub fn on_region(&self, region: &Region) -> bool {
        region.left <= self.x && self.x <= region.right && region.top <= self.y && self.y <= region.bottom
    }
}


impl CoordPx {
    fn relative_x(&self) -> f32 {
        self.x as f32 / self.width as f32
    }

    fn relative_y(&self) -> f32 {
        self.y as f32 / self.height as f32
    }

    pub fn is_valid(&self) -> bool {
        !self.relative_x().is_nan() && !self.relative_y().is_nan()
    }
}

impl Default for CoordPx {
    fn default() -> Self {
        CoordPx { x: 0, y: 0, width: 0, height: 0 }
    }
}

impl fmt::Display for CoordPx {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{:1.2}x{:1.2}",
            self.x as f32 / self.width as f32,
            self.y as f32 / self.height as f32)
    }
}


impl Size {
    pub fn new(width: i32, height: i32) -> Size {
        Size { width, height }
    }

    pub fn from_pixbuf(pixbuf: &Pixbuf) -> Size {
        Size {
            width: pixbuf.get_width(),
            height: pixbuf.get_height(),
        }
    }

    // pub fn from_pixbuf_animation(pixbuf: &PixbufAnimation) -> Size {
    //     Size {
    //         width: pixbuf.get_width(),
    //         height: pixbuf.get_height(),
    //     }
    // }

    pub fn floated(&self) -> (f64, f64) {
        (f64!(self.width), f64!(self.height))
    }

    pub fn rotate(&self, n: u8) -> Self {
        if n % 2 == 1 {
            Size { width: self.height, height: self.width }
        } else {
            *self
        }
    }

    pub fn scaled(&self, scale: f64) -> Size {
        Size {
            width: (f64!(self.width) * scale) as i32,
            height: (f64!(self.height) * scale) as i32,
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
            Fixed(w, h) => self.fit_to_fixed(w, h),
            Scale(scale) => self.fit_to_scaled(scale),
        };

        (scale, fitted)
    }

    pub fn fit_with_clipping(&self, cell_size: &Size, drawing: &DrawingState) -> (f64, Size, Option<Region>) {
        if let Some(ref clip) = drawing.clipping {
            let (clipped_size, clipped_region) = self.clipped(clip);
            let (scale, fitted) = clipped_size.fit(cell_size, &drawing.fit_to);
            (scale, fitted, Some(clipped_region))
        } else {
            let (scale, size) = self.fit(cell_size, &drawing.fit_to);
            (scale, size, None)
        }
    }

    pub fn dimensions(&self) -> i32 {
        self.width * self.height
    }

    pub fn ratio(&self) -> (i32, i32) {
        let divisor = gcd(self.width, self.height);
        (self.width / divisor, self.height / divisor)
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
        let mut scale = f64!(cell.width) / f64!(self.width);
        let result_height = (f64!(self.height) * scale) as i32;
        if result_height > cell.height {
            scale = f64!(cell.height) / f64!(self.height);
        }
        (scale, self.scaled(scale))
    }

    fn fit_to_width(&self, cell: &Size) -> (f64, Size) {
        let scale = f64!(cell.width) / f64!(self.width);
        (scale, self.scaled(scale))
    }

    fn fit_to_height(&self, cell: &Size) -> (f64, Size) {
        let scale = f64!(cell.height) / f64!(self.height);
        (scale, self.scaled(scale))
    }

    pub fn fit_to_fixed(&self, w: i32, h: i32) -> (f64, Size) {
        let mut scale = f64!(w) / f64!(self.width);
        let result_height = (f64!(self.height) * scale) as i32;
        if result_height > h {
            scale = f64!(h) / f64!(self.height);
        }
        (scale, self.scaled(scale))
    }

    fn fit_to_scaled(&self, scale: usize) -> (f64, Size) {
        let scale = scale as f64 / 100.0;
        (scale, self.scaled(scale))
    }
}

impl PartialOrd for Size {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.dimensions().partial_cmp(&other.dimensions())
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

    pub fn full() -> Region {
        Region::new(0.0, 0.0, 1.0, 1.0)
    }

    pub fn width(&self) -> f64 {
        self.right - self.left
    }

    pub fn height(&self) -> f64 {
        self.bottom - self.top
    }

    pub fn centroids(&self) -> (f64, f64) {
        (self.left - self.width() / 2.0, self.top - self.height() / 2.0)
    }

    #[cfg_attr(feature = "cargo-clippy", allow(many_single_char_names))]
    pub fn contains(&self, x: i32, y: i32, width: i32, height: i32) -> bool {
        let (l, r, t, b) = self.absolute(width, height);
        (l <= x && x <= r && t <= y && y <= b)
    }

    pub fn absolute(&self, width: i32, height: i32) -> (i32, i32, i32, i32) {
        let l = (f64!(width) * self.left) as i32;
        let r = (f64!(width) * self.right) as i32;
        let t = (f64!(height) * self.top) as i32;
        let b = (f64!(height) * self.bottom) as i32;
        (l, r, t, b)
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

impl FromStr for Region {
    type Err = String;

    fn from_str(src: &str) -> Result<Self, String> {
        let err = Err(o!("Invalid format (e.g. 0.0x0.0-1.0x1.0)"));

        let hyphen: Vec<&str> = src.split_terminator('-').collect();
        if hyphen.len() != 2 {
            return err;
        }

        let xs_from: Vec<&str> = hyphen[0].split_terminator('x').collect();
        let xs_to: Vec<&str> = hyphen[1].split_terminator('x').collect();

        if xs_from.len() != 2 || xs_to.len() != 2 {
            return err
        }

        if let (Ok(left), Ok(top), Ok(right), Ok(bottom)) = (xs_from[0].parse(), xs_from[1].parse(), xs_to[0].parse(), xs_to[1].parse()) {
            Ok(Self::new(left, top, right, bottom))
        } else {
            err
        }
    }
}

impl fmt::Display for Region {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:1.2}x{:1.2}-{:1.2}x{:1.2}", self.left, self.top, self.right, self.bottom)
    }
}


impl FitTo {
    pub fn is_scrollable(&self) -> bool {
        *self != FitTo::Cell
    }

    pub fn set_scale(&mut self, value: usize) {
        *self = FitTo::Scale(clamp!(MINIMUM_SCALE, value, MAXIMUM_SCALE));
    }
}

impl FromStr for Size {
    type Err = String;

    fn from_str(src: &str) -> Result<Self, String> {
        let size: Vec<&str> = src.split_terminator('x').collect();
        if size.len() == 2 {
            if let (Ok(w), Ok(h)) = (size[0].parse(), size[1].parse()) {
                return Ok(Size::new(w, h));
            }
        }
        if let Ok((w, h)) = resolution::from(src) {
            return Ok(Size::new(w as i32, h as i32));
        }

        Err(format!("Invalid size format: {}", src))
    }
}


fn gcd(x: i32, y: i32) -> i32 {
    if y == 0 {
        x
    } else {
        gcd(y, x % y)
    }
}
