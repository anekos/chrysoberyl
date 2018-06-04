
use std::env::{current_dir, home_dir};
use std::path::Path;



pub fn get_candidates<T: AsRef<Path>>(path: T, directory_only: bool, result: &mut Vec<String>) {
    let entries = if path.as_ref().iter().count() == 0 {
        if_let_ok!(dir = current_dir(), |_| ());
        dir.read_dir()
    } else if path.as_ref().is_dir() {
        path.as_ref().read_dir()
    } else if let Some(dir) = path.as_ref().parent() {
        if dir.is_dir() {
            dir.read_dir()
        } else {
            if_let_ok!(dir = current_dir(), |_| ());
            dir.read_dir()
        }
    } else {
        if_let_ok!(dir = current_dir(), |_| ());
        dir.read_dir()
    };

    if_let_ok!(entries = entries, |_| ());
    if_let_some!(home = home_dir(), ());
    if_let_some!(home = home.to_str(), ());

    for entry in entries {
        if let Ok(entry) = entry {
            if directory_only {
                if let Ok(file_type) = entry.file_type() {
                    if file_type.is_file() {
                        continue;
                    }
                }
            }
            if let Some(path) = entry.path().to_str() {
                if path.starts_with(home) {
                    result.push(format!("~{}", &path[home.len()..]));
                } else {
                    result.push(o!(path));
                }
            }
        }
    }
}
