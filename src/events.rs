
use std::default::Default;
use std::fmt;
use std::str::FromStr;



#[derive(PartialEq, Hash, Clone, Debug, Eq)]
pub enum EventName {
    AtFirst,
    AtLast,
    DownloadAll,
    Error,
    FileChanged,
    Initialize,
    InvalidAll,
    MappedInput,
    Quit,
    ResizeWindow,
    ShowImage,
    ShowImagePre,
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
            "file-changed" => Ok(FileChanged),
            "initialize" | "init" => Ok(Initialize),
            "invalid-all" => Ok(InvalidAll),
            "mapped-input" => Ok(MappedInput),
            "quit" => Ok(Quit),
            "resize-window" | "resize" => Ok(ResizeWindow),
            "show-image" => Ok(ShowImage),
            "show-image-pre" => Ok(ShowImagePre),
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
                FileChanged => "file-changed",
                Initialize => "initialize",
                InvalidAll => "invalid-all",
                MappedInput => "mapped-input",
                Quit => "quit",
                ResizeWindow => "resize-window",
                ShowImagePre => "show-image-pre",
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
