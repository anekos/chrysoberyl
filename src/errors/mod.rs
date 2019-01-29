
use std::fmt::{Display, Formatter, Result as FmtResult};
use std::io;
use std::num::ParseIntError;
use std::sync::mpsc::SendError;

use cairo;
use failure::{Backtrace, Context, Fail};

use crate::operation::ParsingError;



pub type AppResult<T> = Result<T, Error>;
pub type AppResultU = Result<(), Error>;


macro_rules! chry_error {
    ($message:expr) => {
        {
            use errors::ErrorKind;
            crate::errors::Error::from(ErrorKind::Standard($message))
        }
    };
    ($message:expr $(,$args:expr)*) => {
        {
            use crate::errors::ErrorKind;
            crate::errors::Error::from(ErrorKind::Standard(format!($message, $($args),*)))
        }
    }
}

#[derive(Fail, Debug, Clone)]
pub enum ErrorKind {
    File(&'static str, String),
    Fixed(&'static str),
    InvalidValue(String),
    Io,
    NotSupported(&'static str),
    Parse(String),
    ParseInt,
    ParseOperation,
    SendError,
    Standard(String),
    UndefinedOperation(String),
    Library,
}

#[derive(Debug)]
pub struct Error {
    inner: Context<ErrorKind>,
}


impl Display for ErrorKind {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        use self::ErrorKind::*;

        match *self {
            File(e, ref file) => write!(f, "{}: {}", e, file),
            Fixed(e) => write!(f, "{}", e),
            InvalidValue(ref e) => write!(f, "Invalid value: {}", e),
            Io => write!(f, "IO Error"),
            Library => write!(f, "Library error"),
            NotSupported(e) => write!(f, "Not supported: {}", e),
            Parse(ref e) => write!(f, "Parsing error: {}", e),
            ParseInt => write!(f, "Integer parsing error"),
            ParseOperation => write!(f, "Operation parsing error"),
            SendError => write!(f, "Channel send error"),
            Standard(ref e) => write!(f, "{}", e),
            UndefinedOperation(ref name) => write!(f, "Undefined operation: @{}", name),
        }
    }
}


impl Fail for Error {
    fn cause(&self) -> Option<&Fail> {
        self.inner.cause()
    }

    fn backtrace(&self) -> Option<&Backtrace> {
        self.inner.backtrace()
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        Display::fmt(&self.inner, f)
    }
}

// impl Error {
//     pub fn new(inner: Context<ErrorKind>) -> Error {
//         Error { inner }
//     }
//
//     pub fn kind(&self) -> &ErrorKind {
//         self.inner.get_context()
//     }
// }

impl From<ErrorKind> for Error {
    fn from(kind: ErrorKind) -> Error {
        Error {
            inner: Context::new(kind),
        }
    }
}

impl From<Context<ErrorKind>> for Error {
    fn from(inner: Context<ErrorKind>) -> Error {
        Error { inner }
    }
}

impl From<io::Error> for Error {
    fn from(error: io::Error) -> Error {
        Error {
            inner: error.context(ErrorKind::Io),
        }
    }
}

impl<T> From<SendError<T>> for Error {
    fn from(_error: SendError<T>) -> Self {
        Error::from(ErrorKind::SendError)
    }
}

impl From<cairo::Status> for Error {
    fn from(status: cairo::Status) -> Self {
        Error::from(ErrorKind::Standard(d!(status)))
    }
}

impl From<&'static str> for Error {
    fn from(error: &'static str) -> Self {
        Error::from(ErrorKind::Fixed(error))
    }
}

impl From<String> for Error {
    fn from(error: String) -> Self {
        Error::from(ErrorKind::Standard(error))
    }
}

impl From<ParseIntError> for Error {
    fn from(error: ParseIntError) -> Error {
        Error {
            inner: error.context(ErrorKind::ParseInt),
        }
    }
}

impl From<ParsingError> for Error {
    fn from(error: ParsingError) -> Error {
        Error {
            inner: error.context(ErrorKind::ParseOperation),
        }
    }
}

macro_rules! define_library_error {
    ($type:ty) => {
        impl From<$type> for Error {
            fn from(error: $type) -> Error {
                Error {
                    inner: error.context(ErrorKind::Library),
                }
            }
        }
    }
}

define_library_error!(css_color_parser::ColorParseError);
define_library_error!(curl::Error);
define_library_error!(glib::error::Error);
define_library_error!(mrusty::MrubyError);
define_library_error!(std::env::VarError);
define_library_error!(std::string::FromUtf8Error);
define_library_error!(url::ParseError);
define_library_error!(cairo::IoError);
