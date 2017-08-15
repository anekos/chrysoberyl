
use std::str::FromStr;

use cairo;
use gdk_pixbuf::InterpType;

use color::Color;
use option::*;
use resolution;
use size::FitTo;
use state::{ScalingMethod, StatusFormat, TitleFormat, MaskOperator};



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


macro_rules! def_opt_uint {
    ($type:ty) => {
        impl OptionValue for Option<$type> {
            fn cycle(&mut self, reverse: bool) -> Result {
                if_let_some!(v = self.as_mut(), Ok(()));

                if reverse {
                    if *v != 0 {
                        *v -= 1;
                    }
                } else {
                    *v += 1;
                }

                Ok(())
            }

            fn unset(&mut self) -> Result {
                *self = None;
                Ok(())
            }

            fn set(&mut self, value: &str) -> Result {
                value.parse().map(|value| {
                    *self = Some(value);
                }).map_err(|it| s!(it))
            }
        }
    }
}

def_opt_uint!(usize);
def_opt_uint!(u64);
def_opt_uint!(u32);


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
            _ => {
                let size: Vec<&str> = src.split_terminator('x').collect();
                if size.len() == 2 {
                    if let (Ok(w), Ok(h)) = (size[0].parse(), size[1].parse()) {
                        return Ok(Fixed(w, h));
                    }
                }
                if let Ok((w, h)) = resolution::from(src.as_bytes().to_vec()) {
                    return Ok(Fixed(w as i32, h as i32));
                }
                return Err(format!("Invalid target name: {}", src))
            }
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


impl FromStr for MaskOperator {
    type Err = String;

    fn from_str(src: &str) -> StdResult<Self, String> {
        use self::cairo::Operator::*;

        let result = match src {
            "clear" => Clear,
            "source" => Source,
            "over" => Over,
            "in" => In,
            "out" => Out,
            "atop" => Atop,
            "dest" => Dest,
            "dest-over" => DestOver,
            "dest-in" => DestIn,
            "dest-out" => DestOut,
            "dest-atop" => DestAtop,
            "xor" => Xor,
            "add" => Add,
            "saturate" => Saturate,
            "multiply" => Multiply,
            "screen" => Screen,
            "overlay" => Overlay,
            "darken" => Darken,
            "lighten" => Lighten,
            "color-dodge" => ColorDodge,
            "color-burn" => ColorBurn,
            "hard-light" => HardLight,
            "soft-light" => SoftLight,
            "difference" => Difference,
            "exclusion" => Exclusion,
            "hsl-hue" => HslHue,
            "hsl-saturation" => HslSaturation,
            "hsl-color" => HslColor,
            "hsl-luminosity" => HslLuminosity,
            _ => return Err(format!("invalid mask operator: {}", src)),
        };

        Ok(MaskOperator(result))
    }
}

impl OptionValue for MaskOperator {
    fn set(&mut self, value: &str) -> Result {
        value.parse().map(|value| {
            *self = value;
            ()
        })
    }

    fn cycle(&mut self, reverse: bool) -> Result {
        use self::cairo::Operator::*;

        self.0 = cycled(self.0, &[
            Clear,
            Source,
            Over,
            In,
            Out,
            Atop,
            Dest,
            DestOver,
            DestIn,
            DestOut,
            DestAtop,
            Xor,
            Add,
            Saturate,
            Multiply,
            Screen,
            Overlay,
            Darken,
            Lighten,
            ColorDodge,
            ColorBurn,
            HardLight,
            SoftLight,
            Difference,
            Exclusion,
            HslHue,
            HslSaturation,
            HslColor,
            HslLuminosity,
        ], reverse);

        Ok(())
    }
}


pub fn cycled<T>(current: T, order: &[T], reverse: bool) -> T
where T: PartialEq + Copy {
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
