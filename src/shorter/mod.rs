
use std::env::home_dir;
use std::path::{PathBuf, Path};

use utils::path_to_string;

#[cfg(test)] mod test;



pub fn shorten<T: AsRef<Path>>(path: &T, max: usize) -> String {
    let mut path = path.as_ref().to_path_buf();

    if let Some(home) = home_dir() {
        if path.starts_with(&home) {
            let mut s = path_to_string(&path);
            s.drain(0..path_to_string(&home).len());
            path = Path::new(&format!("~{}", s)).to_path_buf()
        }
    }

    while max < len(&path) {
        if let Some(short) = pop_front(&path) {
            path = short;
        } else {
            break;
        }
    }

    path_to_string(&path)
}


fn pop_front<T: AsRef<Path>>(path: &T) -> Option<PathBuf> {
    let mut cs = path.as_ref().components();
    let result = cs.next().map(|_| cs.as_path().to_path_buf());
    cs.next().and_then(|_| result)
}


fn len<T: AsRef<Path>>(path: &T) -> usize {
    path.as_ref().to_str().map(|it| it.len()).unwrap_or(0)
}
