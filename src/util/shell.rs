
use std::borrow::Cow;
use std::path::PathBuf;

use shell_escape;
use util::path::path_to_str;



pub fn escape_pathbuf(path: &PathBuf) -> String {
    escape(path_to_str(path))
}

pub fn escape(s: &str) -> String {
    let s = Cow::from(o!(s));
    shell_escape::escape(s).into_owned()
}
