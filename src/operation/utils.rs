
use std::path::{Path, PathBuf};

use shellexpand;



pub fn pathbuf(s: &str) -> PathBuf {
    Path::new(s).to_path_buf()
}

pub fn expand(s: &str) -> Result<String, String> {
    shellexpand::full(&s).map_err(|it| s!(it)).map(|it| it.into_owned())
}

pub fn expand_to_pathbuf(s: &str) -> Result<PathBuf, String> {
    expand(s).map(|it| Path::new(&it).to_path_buf())
}
