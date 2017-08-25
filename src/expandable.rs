
use std::path::PathBuf;
use std::str::FromStr;
use std::string::ToString;

use shellexpand_wrapper as sh;

use app_path;
use utils::path_to_string;



#[derive(Clone, Debug, PartialEq)]
pub struct Expandable(pub String);


impl ToString for Expandable {
    fn to_string(&self) -> String {
        sh::expand(&self.0)
    }
}

impl FromStr for Expandable {
    type Err = String;

    fn from_str(src: &str) -> Result<Self, String> {
        Ok(Expandable(o!(src)))
    }
}

impl Expandable {
    pub fn expand(&self) -> PathBuf {
        sh::expand_to_pathbuf(&self.0)
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
