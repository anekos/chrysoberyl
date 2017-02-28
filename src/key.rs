
use gdk;


#[derive(Debug, Clone, PartialEq)]
pub struct KeyData {
    pub code: u32,
    pub shift: bool,
    pub ctrl: bool,
    pub alt: bool
}


impl KeyData {
    pub fn new(key: &gdk::EventKey) -> KeyData{
        KeyData {
            code: key.as_ref().keyval,
            shift: false,
            ctrl: false,
            alt: false,
        }
    }

    pub fn text(&self) -> String {
        gdk::keyval_name(self.code).unwrap_or(format!("{}", self.code))
    }
}
