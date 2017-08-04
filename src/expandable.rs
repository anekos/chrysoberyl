
use std::path::{PathBuf, Path};
use std::str::FromStr;
use std::string::ToString;

use shellexpand_wrapper as sh;

use app_path::config_file;
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
    pub fn to_path_buf(&self) -> PathBuf {
        sh::expand_to_pathbuf(&self.0)
    }
}


pub fn expand_all(xs: &[Expandable], search_path: bool) -> Vec<String> {
    xs.iter().enumerate().map(|(index, it)| {
        if search_path && index == 0 {

            let path = it.to_path_buf();

            let mut conf = config_file(Some("script"));
            conf.push(path.clone());
            if conf.exists() {
                return path_to_string(&conf)
            }

            let mut share = Path::new("/usr/share/chrysoberyl/script").to_path_buf();
            share.push(path);
            if share.exists() {
                return path_to_string(&share)
            }
        }

        it.to_string()
    }).collect()
}
