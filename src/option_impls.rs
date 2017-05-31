
use std::str::FromStr;

use gdk_pixbuf::InterpType;

use color::Color;
use option::*;
use size::FitTo;
use state::{ScalingMethod, StatusFormat, TitleFormat, RegionFunction};



impl OptionValue for bool {
    fn is_enabled(&self) -> StdResult<bool, String> {
        Ok(*self)
    }

    fn enable(&mut self) -> Result {
        *self = true;
        Ok(())
    }

    fn disable(&mut self) -> Result {
        *self = false;
        Ok(())
    }

    fn cycle(&mut self, _: bool) -> Result {
        self.toggle()
    }

    fn set(&mut self, value: &str) -> Result {
        *self = match value {
            "true" | "yes" | "on" | "1" => true,
            "false" | "no" | "off" | "0" => false,
            _ => return Err(format!("Invalid value: {}", value))
        };
        Ok(())
    }
}


impl OptionValue for usize {
    fn cycle(&mut self, reverse: bool) -> Result {
        if reverse {
            if *self != 0 {
                *self -= 1;
            }
        } else {
            *self += 1;
        }
        Ok(())
    }

    fn set(&mut self, value: &str) -> Result {
        value.parse().map(|value| {
            *self = value;
        }).map_err(|it| s!(it))
    }
}


impl OptionValue for Color {
    // CSS Color names
    // fn cycle(&mut self) -> Result {
    //     *self += 1;
    //     Ok(())
    // }

    fn set(&mut self, value: &str) -> Result {
        value.parse().map(|value| {
            *self = value;
        })
    }
}


impl FromStr for ScalingMethod {
    type Err = String;

    fn from_str(src: &str) -> StdResult<ScalingMethod, String> {
        match src {
            "n" | "nearest" => Ok(InterpType::Nearest),
            "t" | "tiles" => Ok(InterpType::Tiles),
            "b" | "bilinear" => Ok(InterpType::Bilinear),
            "h" | "hyper" => Ok(InterpType::Hyper),
            _ => Err(format!("Invalid scaling method name: {}", src))
        } .map(ScalingMethod)
    }
}

impl OptionValue for ScalingMethod {
    fn set(&mut self, value: &str) -> Result {
        value.parse().map(|value| {
            *self = value;
            ()
        })
    }

    fn cycle(&mut self, reverse: bool) -> Result {
        use self::InterpType::*;
        self.0 = cycled(self.0, &[Bilinear, Nearest, Tiles, Hyper], reverse);
        Ok(())
    }
}


impl FromStr for FitTo {
    type Err = String;

    fn from_str(src: &str) -> StdResult<Self, String> {
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

impl OptionValue for FitTo {
    fn set(&mut self, value: &str) -> Result {
        value.parse().map(|value| {
            *self = value;
            ()
        })
    }

    fn cycle(&mut self, reverse: bool) -> Result {
        use self::FitTo::*;
        *self = cycled(*self, &[Cell, OriginalOrCell, Original, Width, Height], reverse);
        Ok(())
    }
}


impl OptionValue for StatusFormat {
    fn set(&mut self, value: &str) -> Result {
        *self = StatusFormat(o!(value));
        Ok(())
    }

    fn unset(&mut self) -> Result {
        *self = StatusFormat::default();
        Ok(())
    }
}


impl OptionValue for TitleFormat {
    fn set(&mut self, value: &str) -> Result {
        *self = TitleFormat(o!(value));
        Ok(())
    }

    fn unset(&mut self) -> Result {
        *self = TitleFormat::default();
        Ok(())
    }
}


impl FromStr for RegionFunction {
    type Err = String;

    fn from_str(src: &str) -> StdResult<Self, String> {
        use self::RegionFunction::*;

        match src {
            "c" | "clip" => Ok(Clip),
            "f" | "fill" => Ok(Fill),
            _ => Err(format!("Invalid region function name: {}", src))
        }
    }
}

impl OptionValue for RegionFunction {
    fn set(&mut self, value: &str) -> Result {
        value.parse().map(|value| {
            *self = value;
            ()
        })
    }

    fn cycle(&mut self, reverse: bool) -> Result {
        use self::RegionFunction::*;
        *self = cycled(*self, &[Clip, Fill], reverse);
        Ok(())
    }
}


pub fn cycled<T>(current: T, order: &[T], reverse: bool) -> T
where T: Eq + Copy {
    let i = order.iter().position(|it| *it == current).expect("Invalid value");
    if reverse {
        if i == 0 {
            order.last().cloned().unwrap()
        } else {
            order[i - 1]
        }
    } else {
        order.get(i + 1).cloned().unwrap_or_else(|| order[0])
    }
}
