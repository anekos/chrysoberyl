
use std::borrow::Cow;
use std::path::Path;

use shell_escape;
use crate::util::path::path_to_str;



pub fn escape_pathbuf<T: AsRef<Path>>(path: &T) -> String {
    escape(path_to_str(path))
}

pub fn escape(s: &str) -> String {
    let s = Cow::from(o!(s));
    shell_escape::escape(s).into_owned()
}
