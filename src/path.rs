

use std::path::PathBuf;



pub fn to_string(path: &PathBuf) -> String {
    path.to_str().unwrap().to_string()
}
