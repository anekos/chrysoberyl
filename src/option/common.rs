
use errors::ChryError;



pub fn parse_bool(value: &str) -> Result<bool, ChryError>{
    match value {
        "true" | "yes" | "on" | "1" => Ok(true),
        "false" | "no" | "off" | "0" => Ok(false),
        _ => Err(ChryError::InvalidValue(o!(value)))
    }
}

pub fn bool_to_str(value: bool) -> &'static str {
    if value {
        "yes"
    } else {
        "no"
    }
}
