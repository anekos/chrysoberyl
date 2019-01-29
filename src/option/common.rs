
use crate::errors::{AppResult, Error as AppError, ErrorKind};



pub fn parse_bool(value: &str) -> AppResult<bool>{
    match value {
        "true" | "yes" | "on" | "1" => Ok(true),
        "false" | "no" | "off" | "0" => Ok(false),
        _ => Err(AppError::from(ErrorKind::InvalidValue(o!(value))))
    }
}

pub fn bool_to_str(value: bool) -> &'static str {
    if value {
        "yes"
    } else {
        "no"
    }
}
