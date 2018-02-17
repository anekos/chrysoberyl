
use std::path::{Path, PathBuf};
use std::env::{self, home_dir};

use shellexpand;



pub fn expand_env(s: &str) -> String {
    shellexpand::env_with_context_no_errors(&s, context).into_owned()
}

pub fn expand(s: &str) -> String {
    shellexpand::full_with_context_no_errors(&s, home_dir, context).into_owned()
}

pub fn expand_to_pathbuf(s: &str) -> PathBuf {
    Path::new(&expand(s)).to_path_buf()
}

fn context(name: &str) -> Option<String> {
    match env::var(name) {
        Ok(v) => Some(v),
        _ => Some(o!(""))
    }
}
