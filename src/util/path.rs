
use std::path::Path;



pub fn path_to_str<T: AsRef<Path>>(path: &T) -> &str {
    path.as_ref().to_str().unwrap()
}

pub fn path_to_string<T: AsRef<Path>>(path: &T) -> String {
    path.as_ref().to_str().unwrap().to_owned()
}
