
use std::fmt;
use std::str::FromStr;
use std::default::Default;

use gdk::{self, EventButton, EventKey, EventScroll, ScrollDirection, ModifierType};

use errors::ChryError;
use size::Region;



#[derive(Eq, PartialEq, Hash, Clone, Debug)]
pub struct Key(pub String);

#[derive(Eq, PartialEq, Hash, Clone, Debug, Copy)]
pub struct Coord {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

pub type KeySequence = Vec<Key>;


impl Default for Key {
    fn default() -> Key {
        Key(o!("nop"))
    }
}

impl fmt::Display for Key {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl FromStr for Key {
    type Err = ChryError;

    fn from_str(src: &str) -> Result<Self, ChryError> {
        Ok(Key(o!(src)))
    }
}

impl<'a> From<&'a EventButton> for Key {
    fn from(ev: &'a EventButton) -> Self {
        let mut key = get_modifiers_text(ev.get_state(), false);
        key.push_str(&format!("button-{}", ev.get_button()));
        Key(key)
    }
}

impl<'a> From<&'a EventKey> for Key {
    fn from(ev: &'a EventKey) -> Self {
        let keyval = ev.as_ref().keyval;
        let mut key = get_modifiers_text(ev.get_state(), true);
        key.push_str(&gdk::keyval_name(keyval).unwrap_or_else(|| s!(keyval)));
        Key(key)
    }
}

impl<'a> From<&'a EventScroll> for Key {
    fn from(ev: &'a EventScroll) -> Self {
        let mut key = get_modifiers_text(ev.get_state(), false);
        key.push_str(&get_direction_text(&ev.get_direction()));
        Key(key)
    }
}

fn get_modifiers_text(state: ModifierType, ignore_shift: bool) -> String {
    let mut result = o!("");
    if state.contains(ModifierType::CONTROL_MASK) { result.push_str("C-"); }
    if state.contains(ModifierType::HYPER_MASK) { result.push_str("H-"); }
    if state.contains(ModifierType::META_MASK) { result.push_str("M-"); }
    if state.contains(ModifierType::MOD1_MASK) { result.push_str("A-"); }
    if state.contains(ModifierType::SUPER_MASK) { result.push_str("U-"); }
    if state.contains(ModifierType::SHIFT_MASK) && !ignore_shift { result.push_str("S-"); }
    result
}

fn get_direction_text(direction: &ScrollDirection) -> String {
    use self::ScrollDirection::*;

    let name = match *direction {
        Up => "up",
        Down => "down",
        Left => "left",
        Right => "right",
        Smooth => "smooth",
        __Unknown(n) => return format!("scroll-x{}", n)
    };
    format!("scroll-{}", name)
}


impl Coord {
    fn relative_x(&self) -> f32 {
        self.x as f32 / self.width as f32
    }

    fn relative_y(&self) -> f32 {
        self.y as f32 / self.height as f32
    }

    pub fn is_valid(&self) -> bool {
        !self.relative_x().is_nan() && !self.relative_y().is_nan()
    }

    pub fn on_region(&self, region: &Region) -> bool {
        let x = f64::from(self.relative_x());
        let y = f64::from(self.relative_y());
        region.left <= x && x <= region.right && y <= region.top && y <= region.bottom
    }
}

impl Default for Coord {
    fn default() -> Self {
        Coord { x: 0, y: 0, width: 0, height: 0 }
    }
}

impl fmt::Display for Coord {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{:1.2}x{:1.2}",
            self.x as f32 / self.width as f32,
            self.y as f32 / self.height as f32)
    }
}


pub fn new_key_sequence(s: &str) -> KeySequence {
    s.split(',').map(|it| Key(o!(it))).collect()
}

pub fn key_sequence_to_string(seq: &[Key]) -> String {
    let mut result = o!("");
    let len = seq.len();
    for (index, it) in seq.iter().enumerate() {
        if len - 1 == index { // Detect zero length KeySequence
            result.push_str(&it.0)
        } else {
            result.push_str(&format!("{},", it))
        }
    }
    result
}
