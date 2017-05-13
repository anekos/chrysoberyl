
use std::str::FromStr;

use gdk_pixbuf::InterpType;

use color::Color;
use option::*;
use size::FitTo;
use state::{ScalingMethod, StatusFormat};



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

    fn cycle(&mut self) -> Result {
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
    fn cycle(&mut self) -> Result {
        *self += 1;
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

    fn cycle(&mut self) -> Result {
        use self::InterpType::*;

        match self.0 {
            Hyper => self.0 = Bilinear,
            Bilinear => self.0 = Nearest,
            Nearest => self.0 = Tiles,
            Tiles => self.0 = Hyper,
        }
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

    fn cycle(&mut self) -> Result {
        use self::FitTo::*;

        *self = match *self {
            Cell => OriginalOrCell,
            OriginalOrCell => Original,
            Original => Width,
            Width => Height,
            Height => Cell,
        };
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
