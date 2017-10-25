
use std::fmt;
use std::str::FromStr;
use std::default::Default;

use gdk;

use gdk::{EventButton, EventKey};
use errors::ChryError;



#[derive(Eq, PartialEq, Hash, Clone, Debug)]
pub struct Key(pub String);

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


fn get_modifiers_text(state: gdk::ModifierType) -> String {
    let mut result = o!("");
    if state.contains(gdk::CONTROL_MASK) { result.push_str("C-"); }
    if state.contains(gdk::HYPER_MASK) { result.push_str("H-"); }
    if state.contains(gdk::META_MASK) { result.push_str("M-"); }
    if state.contains(gdk::MOD1_MASK) { result.push_str("A-"); }
    if state.contains(gdk::SUPER_MASK) { result.push_str("S-"); }
    result
}
