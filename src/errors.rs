
use std::io;
use std::sync::mpsc::SendError;

use cairo;
use failure::Fail;



pub type AppResult<T> = Result<T, AppError>;
pub type AppResultU = Result<(), AppError>;



#[derive(Fail, Debug)]
pub enum AppError {
    #[fail(display = "{}: {}", 0, 1)]
    File(&'static str, String),
    #[fail(display = "{}", 0)]
    Fixed(&'static str),
    #[fail(display = "Invalid value: {}", 0)]
    InvalidValue(String),
    #[fail(display = "Invalid value: {} ({})", 0, 1)]
    InvalidValueWithReason(String, String),
    #[fail(display = "IO Error: {}", 0)]
    Io(io::Error),
    #[fail(display = "`{}` is not supported", 0)]
    NotSupported(&'static str),
    #[fail(display = "{}", 0)]
    OperationParser(ParsingError),
    #[fail(display = "Overflow")]
    Overflow,
    #[fail(display = "Not a number: {}", 0)]
    ParseInt(std::num::ParseIntError),
    #[fail(display = "Channel send error")]
    SendError,
    #[fail(display = "{}", 0)]
    Standard(String),
    #[fail(display = "Timer `{}` is not found", 0)]
    TimerNotFound(String),
    #[fail(display = "`{}` is undefined operation", 0)]
    UndefinedOperation(String),
}


#[derive(Fail, Debug, PartialEq)]
pub enum ParsingError {
    #[fail(display = "`{}` is not operation", 0)]
    NotOperation(String),
    #[fail(display = "`{}` is invalid argument", 0)]
    InvalidArgument(String),
    #[fail(display = "{}", 0)]
    Fixed(&'static str),
    #[fail(display = "Too few arguments")]
    TooFewArguments,
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


macro_rules! define_error {
    ($source:ty, $kind:ident) => {
        impl From<$source> for AppError {
            fn from(error: $source) -> AppError {
                AppError::$kind(error)
            }
        }
    }
}

macro_rules! define_std_error {
    ($source:ty) => {
        impl From<$source> for AppError {
            fn from(error: $source) -> AppError {
                AppError::Standard(s!(error))
            }
        }
    }
}


define_error!(io::Error, Io);
define_error!(std::num::ParseIntError, ParseInt);
define_error!(ParsingError, OperationParser);

define_std_error!(String);
define_std_error!(apng_encoder::apng::errors::Error);
define_std_error!(cairo::IoError);
define_std_error!(css_color_parser::ColorParseError);
define_std_error!(curl::Error);
define_std_error!(glib::error::Error);
define_std_error!(mrusty::MrubyError);
define_std_error!(std::env::VarError);
define_std_error!(std::string::FromUtf8Error);
define_std_error!(url::ParseError);

impl<T> From<SendError<T>> for AppError {
    fn from(_error: SendError<T>) -> Self {
        AppError::SendError
    }
}

impl From<&'static str> for AppError {
    fn from(error: &'static str) -> Self {
        AppError::Fixed(error)
    }
}

impl From<cairo::Status> for AppError {
    fn from(error: cairo::Status) -> AppError {
        AppError::Standard(d!(error))
    }
}
