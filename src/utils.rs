
use std::fmt::Display;
use std::path::Path;
use std::time::Duration;



pub fn path_to_str<T: AsRef<Path>>(path: &T) -> &str {
    path.as_ref().to_str().unwrap()
}

pub fn s<T: Display>(x: &T) -> String {
    format!("{}", x)
}

pub fn mangle<T>(_: T) -> () {
    ()
}

pub fn duration_to_string(t: Duration) -> String {
    let msec: u64 = t.as_secs() * 1000 + t.subsec_nanos() as u64 / 1000000;

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
