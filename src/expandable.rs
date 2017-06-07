
use std::path::PathBuf;
use std::string::ToString;

use shellexpand_wrapper as sh;



#[derive(Clone, Debug, PartialEq)]
pub struct Expandable(pub String);


impl ToString for Expandable {
    fn to_string(&self) -> String {
        sh::expand(&self.0)
    }
}

impl Expandable {
    pub fn to_path_buf(&self) -> PathBuf {
        sh::expand_to_pathbuf(&self.0)
    }
}


pub fn expand_all(xs: &[Expandable]) -> Vec<String> {
    xs.iter().map(|it| it.to_string()).collect()
}
