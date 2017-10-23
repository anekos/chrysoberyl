
use std::default::Default;
use std::fmt;
use std::str::FromStr;

use gdk;



#[derive(PartialEq, Eq, Hash, Debug, Clone, Copy)]
pub struct ScrollDirection(pub gdk::ScrollDirection);


impl fmt::Display for ScrollDirection {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use gdk::ScrollDirection as D;
        let t = match self.0 {
            D::Up => "up",
            D::Down => "down",
            D::Left => "left",
            D::Right => "right",
            D::Smooth => "smooth",
            D::__Unknown(n) => return write!(f, "x_{}", n)
        };
        write!(f, "{}", t)
    }
}

impl FromStr for ScrollDirection {
    type Err = String;

    fn from_str(src: &str) -> Result<Self, String> {
        use gdk::ScrollDirection as D;

        let d = match src {
            "up" | "u" => D::Up,
            "down" | "d" => D::Down,
            "left" | "l" => D::Left,
            "right" | "r" => D::Right,
            _ => return Err(format!("Invalid direction: {}", src)),
        };
        Ok(ScrollDirection(d))
    }
}

impl Default for ScrollDirection {
    fn default() -> Self {
        ScrollDirection(gdk::ScrollDirection::Up)
    }
}
