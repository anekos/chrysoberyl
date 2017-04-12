
use std::path::{Path, PathBuf};

use shellexpand;



pub fn pathbuf(s: &str) -> PathBuf {
    Path::new(s).to_path_buf()
}

pub fn expand(s: &str) -> String {
    shellexpand::full(&s).unwrap().into_owned()
}

pub fn expand_to_pathbuf(s: &str) -> PathBuf {
    Path::new(&expand(s)).to_path_buf()
}
