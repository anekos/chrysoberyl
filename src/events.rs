
use std::default::Default;
use std::fmt;
use std::str::FromStr;



#[derive(PartialEq, Hash, Clone, Debug, Eq)]
pub enum EventName {
    Void,
    Initialize,
    Quit,
    ResizeWindow,
    ShowImage,
    InvalidAll,
    User(String),
}


impl FromStr for EventName {
    type Err = String;

    fn from_str(src: &str) -> Result<Self, String> {
        use self::EventName::*;

        match src {
            "void" =>
                Ok(Void),
            "initialize" | "init" =>
                Ok(Initialize),
            "quit" =>
                Ok(Quit),
            "resize-window" | "resize" =>
                Ok(ResizeWindow),
            "show-image" =>
                Ok(ShowImage),
            "invalid-all" =>
                Ok(InvalidAll),
            _ => Ok(User(o!(src)))
        }
    }
}

impl fmt::Display for EventName {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use self::EventName::*;

        write!(f, "{}", {
            match *self {
                Void => "void",
                Initialize => "initialize",
                Quit => "quit",
                ResizeWindow => "resize-window",
                ShowImage => "show-image",
                InvalidAll => "invalid-all",
                User(ref name) => name,
            }
        })
    }
}

impl Default for EventName {
    fn default() -> Self {
        EventName::Void
    }
}
