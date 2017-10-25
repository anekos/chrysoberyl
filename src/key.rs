
use std::fmt;
use std::str::FromStr;
use std::default::Default;

use gdk;

use gdk::{EventButton, EventKey};
use errors::ChryError;



#[derive(Eq, PartialEq, Hash, Clone, Debug)]
pub struct Key(pub String);

#[derive(Eq, PartialEq, Hash, Clone, Debug, Copy)]
pub struct Coord {
    pub x: i32,
    pub y: i32,
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
        let mut key = get_modifiers_text(ev.get_state());
        key.push_str(&format!("button-{}", ev.get_button()));
        Key(key)
    }
}

impl<'a> From<&'a EventKey> for Key {
    fn from(ev: &'a EventKey) -> Self {
        let keyval = ev.as_ref().keyval;
        let mut key = get_modifiers_text(ev.get_state());
        key.push_str(&gdk::keyval_name(keyval).unwrap_or_else(|| s!(keyval)));
        Key(key)
    }
}

fn get_modifiers_text(state: gdk::ModifierType) -> String {
    let mut result = o!("");
    if state.contains(gdk::CONTROL_MASK) { result.push_str("C-"); }
    if state.contains(gdk::HYPER_MASK) { result.push_str("H-"); }
    if state.contains(gdk::META_MASK) { result.push_str("M-"); }
    if state.contains(gdk::MOD1_MASK) { result.push_str("A-"); }
    if state.contains(gdk::SUPER_MASK) { result.push_str("S-"); }
    result
}


impl Default for Coord {
    fn default() -> Self {
        Coord { x: 0, y: 0 }
    }
}

impl fmt::Display for Coord {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}x{}", self.x, self.y)
    }
}


pub fn new_key_sequence(s: &str) -> KeySequence {
    s.split(',').map(|it| Key(o!(it))).collect()
}
