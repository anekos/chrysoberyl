
use std::fmt::{Display, Write};
use std::ops::Range;
use std::path::Path;
use std::time::Duration;



pub fn path_to_str<T: AsRef<Path>>(path: &T) -> &str {
    path.as_ref().to_str().unwrap()
}

pub fn path_to_string<T: AsRef<Path>>(path: &T) -> String {
    path.as_ref().to_str().unwrap().to_owned()
}

pub fn s<T: Display>(x: &T) -> String {
    format!("{}", x)
}

pub fn mangle<T>(_: T) -> () {
    ()
}

pub fn duration_to_string(t: Duration) -> String {
    let msec: u64 = t.as_secs() * 1000 + t.subsec_nanos() as u64 / 1_000_000;

    if 60 * 1000 <= msec {
        format!("{} min {} sec", msec / 60 / 1000, msec % (60 * 1000) / 1000)
    } else {
        format!("{} sec", msec as f64 / 1000.0)
    }
}

#[inline]
pub fn feq(x: f64, y: f64, error: f64) -> bool {
    (x - y).abs() < error
}

pub fn join(xs: &[String], sep: char) -> String {
    let mut result = o!("");
    for x in xs {
        write!(result, "{}{}", x, sep).unwrap();
    }
    result.pop();
    result
}

pub fn range_contains<T: PartialOrd>(range: &Range<T>, index: &T) -> bool {
    range.start <= *index && *index <= range.end
}
