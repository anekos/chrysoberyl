
use std::str::FromStr;

use gdk_pixbuf::Pixbuf;

use option;



#[derive(Clone, PartialEq, Eq)]
pub struct Size {
    pub width: i32,
    pub height: i32
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

    pub fn floated(&self) -> (f64, f64) {
        (self.width as f64, self.height as f64)
    }

    pub fn scaled(&self, scale: f64) -> Size {
        Size {
            width: (self.width as f64 * scale) as i32,
            height: (self.height as f64 * scale) as i32,
        }
    }

    /** (scale, fitted_size, delta) **/
    pub fn fit(&self, cell: &Size, to: &FitTo) -> (f64, Size, Size) {
        use self::FitTo::*;

        let (scale, fitted) = match *to {
            Original => self.fit_to_original(),
            OriginalOrCell => self.fit_to_original_or_cell(cell),
            Cell => self.fit_to_cell(cell),
            Width => self.fit_to_width(cell),
            Height => self.fit_to_height(cell),
        };

        let delta = Size::new(max!(fitted.width - cell.width, 0), max!(fitted.height - cell.height, 0));
        (scale, fitted, delta)
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
