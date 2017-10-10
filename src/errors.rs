
use std::error;
use std::fmt;
use std::io;

use cairo;


pub type BoxedError = Box<error::Error>;


macro_rules! chry_error {
    ($message:expr) => {
        {
            use errors::ChryError;
            ChryError::Standard($message)
        }
    };
    ($message:expr $(,$args:expr)*) => {
        {
            use errors::ChryError;
            ChryError::Standard(format!($message, $($args),*))
        }
    }
}


#[derive(Debug, Clone)]
pub enum ChryError {
    Standard(String),
    Parse(String),
    Fixed(&'static str),
    NotSupported(&'static str),
    InvalidValue(String),
}


impl fmt::Display for ChryError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use self::ChryError::*;

        match *self {
            Standard(ref e) => write!(f, "{}", e),
            Fixed(e) => write!(f, "Invalid value: {}", e),
            NotSupported(e) => write!(f, "Not supported: {}", e),
            Parse(ref e) => write!(f, "Parsing error: {}", e),
            InvalidValue(ref e) => write!(f, "Invalid value: {}", e),
        }
    }
}


impl error::Error for ChryError {
    fn description(&self) -> &str {
        use self::ChryError::*;

        match *self {
            Standard(_) => "error",
            Fixed(e) => e,
            NotSupported(_) => "Not supported",
            Parse(_) => "Parsing error",
            InvalidValue(_) => "Invalid value",
        }
    }

    fn cause(&self) -> Option<&error::Error> {
        None
    }
}

impl From<io::Error> for ChryError {
    fn from(error: io::Error) -> Self {
        ChryError::Standard(s!(error))
    }
}

impl From<cairo::IoError> for ChryError {
    fn from(error: cairo::IoError) -> Self {
        ChryError::Standard(d!(error))
    }
}

impl From<cairo::Status> for ChryError {
    fn from(status: cairo::Status) -> Self {
        ChryError::Standard(d!(status))
    }
}

impl From<&'static str> for ChryError {
    fn from(error: &'static str) -> Self {
        ChryError::Fixed(error)
    }
}
