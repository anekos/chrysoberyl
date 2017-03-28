
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
        // src.parse().map(|it: gdk::RGBA| {
        //     RGB::new(it.red, it.green, it.blue)
        // }).map_err(|_| "Invalid color text".to_owned())
        src.parse().map(|it: u8| {
            Value::new(it as f64 / 255.0)
        }).or_else(|_| {
            src.parse().map(Value::new)
        }).map_err(s)
    }
}

impl RGB {
    pub fn new(red: f64, green: f64, blue: f64) -> RGB {
        RGB { red: red, green: green, blue: blue }
    }

    pub fn from_values(red: Value, green: Value, blue: Value) -> RGB {
        RGB::new(red.value, green.value, blue.value)
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
