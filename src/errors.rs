
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


#[derive(Debug)]
pub enum ChryError {
    Standard(String),
    Parse(String),
    Fix(&'static str),
    NotSupported(&'static str),
    InvalidValue(String),
}


impl fmt::Display for ChryError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use self::ChryError::*;

        match *self {
            Standard(ref e) => write!(f, "{}", e)
        }
    }
}


impl error::Error for ChryError {
    fn description(&self) -> &str {
        use self::ChryError::*;

        match *self {
            Standard(ref e) => e
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
