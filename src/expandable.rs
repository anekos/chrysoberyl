
use std::default::Default;
use std::path::{PathBuf, Path};
use std::str::FromStr;
use std::string::ToString;

use shellexpand_wrapper as sh;

use app_path;
use util::path::path_to_string;



#[derive(Clone, Debug, PartialEq)]
pub enum Expandable {
    Expanded(String),
    Unexpanded(String),
}


impl ToString for Expandable {
    fn to_string(&self) -> String {
        use self::Expandable::*;

        match *self {
            Expanded(ref path) => o!(path),
            Unexpanded(ref path) => sh::expand(path),
        }
    }
}

impl FromStr for Expandable {
    type Err = String;

    fn from_str(src: &str) -> Result<Self, String> {
        Ok(Expandable::new(o!(src)))
    }
}

impl Default for Expandable {
    fn default() -> Expandable {
        Expandable::new(o!(""))
    }
}

impl Expandable {
    pub fn new(path: String) -> Self {
        Expandable::Unexpanded(path)
    }

    pub fn expanded(path: String) -> Self {
        Expandable::Expanded(path)
    }
    pub fn expand(&self) -> PathBuf {
        use self::Expandable::*;

        match *self {
            Expanded(ref path) => Path::new(path).to_path_buf(),
            Unexpanded(ref path) => sh::expand_to_pathbuf(path),
        }
    }

    pub fn search_path(&self, path_list: &app_path::PathList) -> PathBuf {
        let base = self.expand();
        app_path::search_path(&base, path_list)
    }
}


pub fn expand_all(xs: &[Expandable], search_path: bool, path_list: &app_path::PathList) -> Vec<String> {
    xs.iter().enumerate().map(|(index, it)| {
        if search_path && index == 0 {
            path_to_string(&app_path::search_path(&it.expand(), path_list))
        } else {
            it.to_string()
        }
    }).collect()
}
