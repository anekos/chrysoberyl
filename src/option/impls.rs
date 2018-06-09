
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::time::Duration;

use cairo;
use num::Integer;

use color::Color;
use gui::{Position, Screen};
use option::*;
use resolution;
use size::FitTo;
use state::{Alignment, AutoPaging, MaskOperator};

use option::common;



impl OptionValue for bool {
    fn is_enabled(&self) -> Result<bool, ChryError> {
        Ok(*self)
    }

    fn enable(&mut self) -> Result<(), ChryError> {
        *self = true;
        Ok(())
    }

    fn disable(&mut self) -> Result<(), ChryError> {
        *self = false;
        Ok(())
    }

    fn cycle(&mut self, _: bool, n: usize) -> Result<(), ChryError> {
        if n.is_odd() {
            self.toggle()
        } else {
            Ok(())
        }
    }

    fn set(&mut self, value: &str) -> Result<(), ChryError> {
        common::parse_bool(value).map(|value| {
            *self = value;
        })
    }
}

impl OptionValue for Option<PathBuf> {
    fn set(&mut self, value: &str) -> Result<(), ChryError> {
        *self = Some(Path::new(value).to_path_buf());
        Ok(())
    }

    fn unset(&mut self) -> Result<(), ChryError> {
        *self = None;
        Ok(())
    }
}

impl OptionValue for Duration {
    fn set(&mut self, value: &str) -> Result<(), ChryError> {
        value.parse().map(|value: f64| {
            *self = Duration::from_millis((value * 1000.0) as u64);
        }).map_err(|it| ChryError::Standard(format!("Invalid value: {} ({})", value, it)))
    }
}


macro_rules! def_uint {
    ($type:ty) => {
        impl OptionValue for $type {
            fn cycle(&mut self, reverse: bool, n: usize) -> Result<(), ChryError> {
                for _ in 0 .. n {
                    if reverse {
                        if *self != 0 {
                            *self -= 1;
                        }
                    } else if *self < <$type>::max_value() {
                        *self += 1;
                    } else {
                        *self = 0
                    }
                }

                Ok(())
            }

            fn unset(&mut self) -> Result<(), ChryError> {
                *self = 0;
                Ok(())
            }

            fn set(&mut self, value: &str) -> Result<(), ChryError> {
                value.parse().map(|value| {
                    *self = value;
                }).map_err(|it| ChryError::Standard(format!("Invalid value: {} ({})", value, it)))
            }

            fn increment(&mut self, delta: usize) -> Result<(), ChryError> {
                if_let_some!(modified = self.checked_add(delta as $type), Err(ChryError::Fixed("Overflow")));
                *self = modified;
                Ok(())
            }

            fn decrement(&mut self, delta: usize) -> Result<(), ChryError> {
                if_let_some!(modified = self.checked_sub(delta as $type), Err(ChryError::Fixed("Overflow")));
                *self = modified;
                Ok(())
            }
        }
    }
}

macro_rules! def_opt_uint {
    ($type:ty) => {
        impl OptionValue for Option<$type> {
            fn cycle(&mut self, reverse: bool, n: usize) -> Result<(), ChryError> {
                if_let_some!(v = self.as_mut(), Ok(()));

                for _ in 0 .. n {
                    if reverse {
                        if *v != 0 {
                            *v -= 1;
                        }
                    } else {
                        *v += 1;
                    }
                }

                Ok(())
            }

            fn unset(&mut self) -> Result<(), ChryError> {
                *self = None;
                Ok(())
            }

            fn set(&mut self, value: &str) -> Result<(), ChryError> {
                value.parse().map(|value| {
                    *self = Some(value);
                }).map_err(|it| ChryError::InvalidValue(s!(it)))
            }

            fn increment(&mut self, delta: usize) -> Result<(), ChryError> {
                if_let_some!(current = *self, Ok(()));
                if_let_some!(modified = current.checked_add(delta as $type), Err(ChryError::Fixed("Overflow")));
                *self = Some(modified);
                Ok(())
            }

            fn decrement(&mut self, delta: usize) -> Result<(), ChryError> {
                if_let_some!(current = *self, Ok(()));
                if_let_some!(modified = current.checked_sub(delta as $type), Err(ChryError::Fixed("Overflow")));
                *self = Some(modified);
                Ok(())
            }
        }
    }
}

def_opt_uint!(usize);
def_opt_uint!(u64);
def_opt_uint!(u32);
def_uint!(usize);
def_uint!(u8);


impl FromStr for AutoPaging {
    type Err = ChryError;

    fn from_str(src: &str) -> Result<Self, ChryError> {
        use self::AutoPaging::*;

        common::parse_bool(src).map(|it| {
            if it { AutoPaging::Always } else { AutoPaging::Smart }
        }).or_else(|_| {
            let result = match src {
                "always" | "a" => Always,
                "smart" | "s" => Smart,
                _ => return Err(ChryError::InvalidValue(o!(src)))
            };
            Ok(result)
        })
    }
}

impl OptionValue for AutoPaging {
    fn is_enabled(&self) -> Result<bool, ChryError> {
        Ok(self.enabled())
    }

    fn enable(&mut self) -> Result<(), ChryError> {
        *self = AutoPaging::Always;
        Ok(())
    }

    fn disable(&mut self) -> Result<(), ChryError> {
        *self = AutoPaging::DoNot;
        Ok(())
    }

    fn cycle(&mut self, reverse: bool, n: usize) -> Result<(), ChryError> {
        use self::AutoPaging::*;
        *self = cycled(*self, &[DoNot, Always, Smart], reverse, n);
        Ok(())
    }

    fn set(&mut self, value: &str) -> Result<(), ChryError> {
        value.parse().map(|value| {
            *self = value;
            ()
        })
    }
}

impl OptionValue for Color {
    // CSS Color names
    // fn cycle(&mut self) -> Result<(), ChryError> {
    //     *self += 1;
    //     Ok(())
    // }

    fn set(&mut self, value: &str) -> Result<(), ChryError> {
        value.parse().map(|value| {
            *self = value;
        })
    }
}


impl FromStr for FitTo {
    type Err = ChryError;

    fn from_str(src: &str) -> Result<Self, ChryError> {
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
                if let Ok((w, h)) = resolution::from(src) {
                    return Ok(Fixed(w as i32, h as i32));
                }
                if src.ends_with('%') {
                    if let Ok(scale) = src[.. src.len() - 1].parse() {
                        return Ok(Scale(scale))
                    }
                }
                return Err(ChryError::InvalidValue(o!(src)))
            }
        };
        Ok(result)
    }
}

impl OptionValue for FitTo {
    fn set(&mut self, value: &str) -> Result<(), ChryError> {
        value.parse().map(|value| {
            *self = value;
            ()
        })
    }

    fn set_from_count(&mut self, value: Option<usize>) -> Result<(), ChryError> {
        self.set_scale(value.unwrap_or(100));
        Ok(())
    }

    fn cycle(&mut self, reverse: bool, n: usize) -> Result<(), ChryError> {
        use self::FitTo::*;
        *self = cycled(*self, &[Cell, OriginalOrCell, Original, Width, Height], reverse, n);
        Ok(())
    }

    fn increment(&mut self, delta: usize) -> Result<(), ChryError> {
        let value = get_scale(self).checked_add(delta).unwrap_or(<usize>::max_value());
        self.set_scale(value);
        Ok(())
    }

    fn decrement(&mut self, delta: usize) -> Result<(), ChryError> {
        let value = get_scale(self).checked_sub(delta).unwrap_or(<usize>::min_value());
        self.set_scale(value);
        Ok(())
    }
}

fn get_scale(fit_to: &FitTo) -> usize {
    match *fit_to {
        FitTo::Scale(scale) => scale,
        _ => 100,
    }
}


impl OptionValue for Position {
    fn set(&mut self, value: &str) -> Result<(), ChryError> {
        value.parse().map(|value| {
            *self = value;
            ()
        })
    }
}


impl FromStr for MaskOperator {
    type Err = ChryError;

    fn from_str(src: &str) -> Result<Self, ChryError> {
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
            _ => return Err(ChryError::InvalidValue(o!(src))),
        };

        Ok(MaskOperator(result))
    }
}

impl OptionValue for MaskOperator {
    fn set(&mut self, value: &str) -> Result<(), ChryError> {
        value.parse().map(|value| {
            *self = value;
            ()
        })
    }

    fn cycle(&mut self, reverse: bool, n: usize) -> Result<(), ChryError> {
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
        ], reverse, n);

        Ok(())
    }
}


pub fn cycled<T>(current: T, order: &[T], reverse: bool, n: usize) -> T
where T: PartialEq + Copy {
    let len = order.len();
    let n = n % len;
    let i = order.iter().position(|it| *it == current).expect("Invalid value");
    if reverse {
        if n <= i {
            order[i - n]
        } else {
            order[len - (n - i)]
        }
    } else {
        let new = i + n;
        if new < len {
            order[new]
        } else {
            order[new - len]
        }
    }
}


impl FromStr for Alignment {
    type Err = ChryError;

    fn from_str(src: &str) -> Result<Self, ChryError> {
        use gtk::Align::*;

        let align = match src {
            "left" | "l" | "start" => Start,
            "right" | "r" | "end" => End,
            "center" | "c" => Center,
            _ => return Err(ChryError::InvalidValue(o!(src))),
        };
        Ok(Alignment(align))
    }
}

impl OptionValue for Alignment {
    fn cycle(&mut self, reverse: bool, n: usize) -> Result<(), ChryError> {
        use gtk::Align::*;
        *self = cycled(*self, &[Alignment(Start), Alignment(Center), Alignment(End)], reverse, n);
        Ok(())
    }

    fn set(&mut self, value: &str) -> Result<(), ChryError> {
        value.parse().map(|value| {
            *self = value;
            ()
        })
    }
}


impl OptionValue for Screen {
    fn cycle(&mut self, reverse: bool, n: usize) -> Result<(), ChryError> {
        use self::Screen::*;
        *self = cycled(*self, &[Main, CommandLine, LogView], reverse, n);
        Ok(())
    }

    fn set(&mut self, value: &str) -> Result<(), ChryError> {
        value.parse().map(|value| {
            *self = value;
            ()
        })
    }
}

impl FromStr for Screen {
    type Err = ChryError;

    fn from_str(src: &str) -> Result<Self, ChryError> {
        let screen = match src {
            "main" => Screen::Main,
            "log-view" => Screen::LogView,
            "command-line" => Screen::CommandLine,
            _ => return Err(ChryError::InvalidValue(o!(src))),
        };
        Ok(screen)
    }
}
