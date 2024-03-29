
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::time::Duration;

use cairo;
use num::Integer;

use crate::cherenkov::Operator;
use crate::color::Color;
use crate::errors::{AppResult, AppResultU, AppError};
use crate::gui::{Position, Screen};
use crate::option::*;
use crate::resolution;
use crate::size::FitTo;
use crate::state::{Alignment, AutoPaging};

use crate::option::common;



impl OptionValue for bool {
    fn is_enabled(&self) -> AppResult<bool> {
        Ok(*self)
    }

    fn enable(&mut self) -> AppResultU {
        *self = true;
        Ok(())
    }

    fn disable(&mut self) -> AppResultU {
        *self = false;
        Ok(())
    }

    fn cycle(&mut self, _: bool, n: usize, _: &[String]) -> AppResultU {
        if n.is_odd() {
            self.toggle()
        } else {
            Ok(())
        }
    }

    fn set(&mut self, value: &str) -> AppResultU {
        common::parse_bool(value).map(|value| {
            *self = value;
        })
    }
}

impl OptionValue for Option<PathBuf> {
    fn set(&mut self, value: &str) -> AppResultU {
        *self = Some(Path::new(value).to_path_buf());
        Ok(())
    }

    fn unset(&mut self) -> AppResultU {
        *self = None;
        Ok(())
    }
}

impl OptionValue for Duration {
    fn set(&mut self, value: &str) -> AppResultU {
        value.parse().map(|value: f64| {
            *self = Duration::from_millis((value * 1000.0) as u64);
        }).map_err(|it| AppError::InvalidValueWithReason(s!(value), s!(it)))?;
        Ok(())
    }
}


macro_rules! def_uint_cycle {
    ($type:ty, $target:ident, $candidates:expr, $reverse:expr, $n:expr) => {
        if !$candidates.is_empty() {
            let mut cs = vec![];
            let mut set_first_value = true;
            for candidate in $candidates {
                if let Ok(v) = candidate.parse() {
                    if v == *$target {
                        set_first_value = false;
                    }
                    cs.push(v);
                } else {
                    return Err(AppError::InvalidValue(o!(candidate)));
                }
            }
            if set_first_value {
                *$target = cs[0];
                return Ok(())
            } else {
                return set_cycled($target, cs.as_slice(), $reverse, $n, &[])
            }
        }

        *$target = cycle_uint!($type, $reverse, $n, $target);
    }
}

macro_rules! def_uint {
    ($type:ty) => {
        impl OptionValue for $type {
            fn cycle(&mut self, reverse: bool, n: usize, candidates: &[String]) -> AppResultU {
                def_uint_cycle!($type, self, candidates, reverse, n);
                Ok(())
            }

            fn unset(&mut self) -> AppResultU {
                *self = 0;
                Ok(())
            }

            fn set(&mut self, value: &str) -> AppResultU {
                value.parse().map(|value| {
                    *self = value;
                }).map_err(|it| AppError::InvalidValueWithReason(s!(value), s!(it)))?;
                Ok(())
            }

            fn increment(&mut self, delta: usize) -> AppResultU {
                if_let_some!(modified = self.checked_add(delta as $type), Err(AppError::Overflow));
                *self = modified;
                Ok(())
            }

            fn decrement(&mut self, delta: usize) -> AppResultU {
                if_let_some!(modified = self.checked_sub(delta as $type), Err(AppError::Overflow));
                *self = modified;
                Ok(())
            }
        }
    }
}

macro_rules! def_opt_uint {
    ($type:ty) => {
        impl OptionValue for Option<$type> {
            fn cycle(&mut self, reverse: bool, n: usize, candidates: &[String]) -> AppResultU {
                if_let_some!(v = self.as_mut(), Ok(()));
                def_uint_cycle!($type, v, candidates, reverse, n);
                Ok(())
            }

            fn unset(&mut self) -> AppResultU {
                *self = None;
                Ok(())
            }

            fn set(&mut self, value: &str) -> AppResultU {
                value.parse().map(|value| {
                    *self = Some(value);
                }).map_err(|it| AppError::InvalidValue(s!(it)))?;
                Ok(())
            }

            fn increment(&mut self, delta: usize) -> AppResultU {
                if_let_some!(current = *self, Ok(()));
                if_let_some!(modified = current.checked_add(delta as $type), Err(AppError::Overflow));
                *self = Some(modified);
                Ok(())
            }

            fn decrement(&mut self, delta: usize) -> AppResultU {
                if_let_some!(current = *self, Ok(()));
                if_let_some!(modified = current.checked_sub(delta as $type), Err(AppError::Overflow));
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
    type Err = AppError;

    fn from_str(src: &str) -> AppResult<Self> {
        use self::AutoPaging::*;

        common::parse_bool(src).map(|it| {
            if it { AutoPaging::Always } else { AutoPaging::Smart }
        }).or_else(|_| {
            let result = match src {
                "always" | "a" => Always,
                "smart" | "s" => Smart,
                _ => return Err(AppError::InvalidValue(o!(src)))
            };
            Ok(result)
        })
    }
}

impl OptionValue for AutoPaging {
    fn is_enabled(&self) -> AppResult<bool> {
        Ok(self.enabled())
    }

    fn enable(&mut self) -> AppResultU {
        *self = AutoPaging::Always;
        Ok(())
    }

    fn disable(&mut self) -> AppResultU {
        *self = AutoPaging::DoNot;
        Ok(())
    }

    fn cycle(&mut self, reverse: bool, n: usize, candidates: &[String]) -> AppResultU {
        use self::AutoPaging::*;
        set_cycled(self, &[DoNot, Always, Smart], reverse, n, candidates)
    }

    fn set(&mut self, value: &str) -> AppResultU {
        value.parse().map(|value| {
            *self = value;
        })
    }
}

impl OptionValue for Color {
    // CSS Color names
    // fn cycle(&mut self) -> Result<(), ChryError> {
    //     *self += 1;
    //     Ok(())
    // }

    fn set(&mut self, value: &str) -> AppResultU {
        value.parse().map(|value| {
            *self = value;
        })
    }
}


impl FromStr for FitTo {
    type Err = AppError;

    fn from_str(src: &str) -> AppResult<Self> {
        use self::FitTo::*;

        let result = match src {
            "original" => Original,
            "original-or-cell" | "cell-or-original" => OriginalOrCell,
            "cell" => Cell,
            "crop" => Crop,
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
                if let Some(stripped) = src.strip_suffix('%') {
                    if let Ok(scale) = stripped.parse() {
                        return Ok(Scale(scale))
                    }
                }
                return Err(AppError::InvalidValue(o!(src)))
            }
        };
        Ok(result)
    }
}

impl OptionValue for FitTo {
    fn set(&mut self, value: &str) -> AppResultU {
        value.parse().map(|value| {
            *self = value;
        })
    }

    fn set_from_count(&mut self, value: Option<usize>) -> AppResultU {
        self.set_scale(value.unwrap_or(100));
        Ok(())
    }

    fn cycle(&mut self, reverse: bool, n: usize, candidates: &[String]) -> AppResultU {
        use self::FitTo::*;
        set_cycled(self, &[Cell, OriginalOrCell, Original, Width, Height], reverse, n, candidates)
    }

    fn increment(&mut self, delta: usize) -> AppResultU {
        let value = get_scale(self).saturating_add(delta);
        self.set_scale(value);
        Ok(())
    }

    fn decrement(&mut self, delta: usize) -> AppResultU {
        let value = get_scale(self).saturating_sub(delta);
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
    fn set(&mut self, value: &str) -> AppResultU {
        value.parse().map(|value| {
            *self = value;
        })
    }
}


impl OptionValue for Operator {
    fn set(&mut self, value: &str) -> AppResultU {
        value.parse().map(|value| {
            *self = value;
        })
    }

    fn cycle(&mut self, reverse: bool, n: usize, candidates: &[String]) -> AppResultU {
        use self::cairo::Operator::*;

        set_cycled(self, &[
            Operator(Clear),
            Operator(Source),
            Operator(Over),
            Operator(In),
            Operator(Out),
            Operator(Atop),
            Operator(Dest),
            Operator(DestOver),
            Operator(DestIn),
            Operator(DestOut),
            Operator(DestAtop),
            Operator(Xor),
            Operator(Add),
            Operator(Saturate),
            Operator(Multiply),
            Operator(Screen),
            Operator(Overlay),
            Operator(Darken),
            Operator(Lighten),
            Operator(ColorDodge),
            Operator(ColorBurn),
            Operator(HardLight),
            Operator(SoftLight),
            Operator(Difference),
            Operator(Exclusion),
            Operator(HslHue),
            Operator(HslSaturation),
            Operator(HslColor),
            Operator(HslLuminosity),
        ], reverse, n, candidates)
    }
}


pub fn set_cycled<T>(current: &mut T, order: &[T], reverse: bool, n: usize, candidates: &[String]) -> AppResultU
where T: PartialEq + Copy + FromStr {
    if candidates.is_empty() {
        *current = cycled(*current, order, reverse, n);
        return Ok(());
    }

    let mut cs = vec![];
    let mut return_first = true;
    for candidate in candidates {
        if let Ok(candidate) = candidate.parse() {
            if candidate == *current {
                return_first = false;
            }
            cs.push(candidate);
        } else {
            return Err(AppError::InvalidValue(o!(candidate)));
        }
    }

    if return_first {
        *current = cs[0];
        return Ok(());
    }

    *current = cycled(*current, &cs, reverse, n);
    Ok(())
}

fn cycled<T>(current: T, order: &[T], reverse: bool, n: usize) -> T
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
    type Err = AppError;

    fn from_str(src: &str) -> AppResult<Self> {
        use gtk::Align::*;

        let align = match src {
            "left" | "l" | "start" => Start,
            "right" | "r" | "end" => End,
            "center" | "c" => Center,
            _ => return Err(AppError::InvalidValue(o!(src))),
        };
        Ok(Alignment(align))
    }
}

impl OptionValue for Alignment {
    fn cycle(&mut self, reverse: bool, n: usize, candidates: &[String]) -> AppResultU {
        use gtk::Align::*;
        set_cycled(self, &[Alignment(Start), Alignment(Center), Alignment(End)], reverse, n, candidates)
    }

    fn set(&mut self, value: &str) -> AppResultU {
        value.parse().map(|value| {
            *self = value;
        })
    }
}


impl OptionValue for Screen {
    fn cycle(&mut self, reverse: bool, n: usize, candidates: &[String]) -> AppResultU {
        use self::Screen::*;
        set_cycled(self, &[Main, CommandLine, LogView], reverse, n, candidates)
    }

    fn unset(&mut self) -> AppResultU {
        *self = Screen::Main;
        Ok(())
    }

    fn set(&mut self, value: &str) -> AppResultU {
        value.parse().map(|value| {
            *self = value;
        })
    }
}

impl FromStr for Screen {
    type Err = AppError;

    fn from_str(src: &str) -> AppResult<Self> {
        let screen = match src {
            "main" => Screen::Main,
            "log-view" => Screen::LogView,
            "command-line" => Screen::CommandLine,
            "ui" => Screen::UserUI,
            _ => return Err(AppError::InvalidValue(o!(src))),
        };
        Ok(screen)
    }
}
