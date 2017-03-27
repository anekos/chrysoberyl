
use std::str::FromStr;

use gdk;

use utils::s;



#[derive(Clone, Debug, PartialEq)]
pub struct Value {
    value: f64
}

#[derive(Clone, Debug, PartialEq)]
pub struct RGB {
    pub red: f64,
    pub green: f64,
    pub blue: f64
}


impl Value {
    pub fn new(value: f64) -> Value {
        Value { value: value }
    }

    pub fn max() -> Value {
        Value::new(1.0)
    }
}

impl FromStr for Value {
    type Err = String;

    fn from_str(src: &str) -> Result<Value, String> {
        src.parse().map(|it: u8| {
            Value::new(it as f64 / 255.0)
        }).or_else(|_| {
            src.parse().map(Value::new)
        }).map_err(s)
    }
}

impl RGB {
    pub fn new(red: Value, green: Value, blue: Value) -> RGB {
        RGB { red: red.value, green: green.value, blue: blue.value }
    }

    pub fn gdk_rgba(&self) -> gdk::RGBA {
        gdk::RGBA {
            red: self.red,
            green: self.green,
            blue: self.blue,
            alpha: 1.0
        }
    }
}
