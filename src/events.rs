
use std::default::Default;
use std::fmt;
use std::str::FromStr;



#[derive(PartialEq, Hash, Clone, Debug, Eq)]
pub enum EventName {
    AtFirst,
    AtLast,
    DownloadAll,
    Error,
    Initialize,
    InvalidAll,
    Quit,
    ResizeWindow,
    ShowImage,
    Spawn,
    Void,
    User(String),
}


impl FromStr for EventName {
    type Err = String;

    fn from_str(src: &str) -> Result<Self, String> {
        use self::EventName::*;

        match src {
            "at-first" => Ok(AtFirst),
            "at-last" => Ok(AtLast),
            "download-all" => Ok(DownloadAll),
            "error" => Ok(Error),
            "initialize" | "init" => Ok(Initialize),
            "invalid-all" => Ok(InvalidAll),
            "quit" => Ok(Quit),
            "resize-window" | "resize" => Ok(ResizeWindow),
            "show-image" => Ok(ShowImage),
            "spawn" => Ok(Spawn),
            "void" => Ok(Void),
            _ => Ok(User(o!(src)))
        }
    }
}

impl fmt::Display for EventName {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use self::EventName::*;

        write!(f, "{}", {
            match *self {
                AtFirst => "at-first",
                AtLast => "at-last",
                DownloadAll => "download-all",
                Error => "error",
                Initialize => "initialize",
                InvalidAll => "invalid-all",
                Quit => "quit",
                ResizeWindow => "resize-window",
                ShowImage => "show-image",
                Spawn => "spawn",
                Void => "void",
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
