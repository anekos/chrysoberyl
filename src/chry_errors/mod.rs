
use std::fmt::{Display, Formatter, Result as FmtResult};
use std::io;

use cairo;
use failure::{Backtrace, Context, Fail};


pub type AppResult<T> = Result<T, Error>;
pub type AppResultU = Result<(), Error>;


macro_rules! chry_error {
    ($message:expr) => {
        {
            use errors::ErrorKind;
            ErrorKind::Standard($message)
        }
    };
    ($message:expr $(,$args:expr)*) => {
        {
            use crate::errors::ErrorKind;
            ErrorKind::Standard(format!($message, $($args),*))
        }
    }
}


#[derive(Fail, Debug, Clone)]
pub enum ErrorKind {
    Io,
    Cairo,
    File(&'static str, String),
    Fixed(&'static str),
    InvalidValue(String),
    NotSupported(&'static str),
    Parse(String),
    Standard(String),
    UndefinedOperation(String),
}

#[derive(Debug)]
pub struct Error {
    inner: Context<ErrorKind>,
}


impl Display for ErrorKind {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        use self::ErrorKind::*;

        match *self {
            Io => write!(f, "IO Error"),
            Cairo => write!(f, "Cairo Error"),
            File(e, ref file) => write!(f, "{}: {}", e, file),
            Fixed(e) => write!(f, "{}", e),
            InvalidValue(ref e) => write!(f, "Invalid value: {}", e),
            NotSupported(e) => write!(f, "Not supported: {}", e),
            Parse(ref e) => write!(f, "Parsing error: {}", e),
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

impl Error {
    pub fn new(inner: Context<ErrorKind>) -> Error {
        Error { inner }
    }

    pub fn kind(&self) -> &ErrorKind {
        self.inner.get_context()
    }
}

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

impl From<cairo::IoError> for Error {
    fn from(error: cairo::IoError) -> Self {
        Error {
            inner: ErrorKind::Standard(d!(error))
        }
    }
}

impl From<cairo::Status> for Error {
    fn from(status: cairo::Status) -> Self {
        Error {
            inner: ErrorKind::Standard(d!(status))
        }
    }
}

impl From<&'static str> for Error {
    fn from(error: &'static str) -> Self {
        Error {
            inner: ErrorKind::Fixed(error)
        }
    }
}
