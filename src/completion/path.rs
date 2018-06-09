
use std::env::{current_dir, home_dir};

use shellexpand_wrapper as sh;



pub fn get_candidates(path: &str, directory_only: bool, prefix: &str, result: &mut Vec<String>) {
    let path = &path[min!(prefix.len(), path.len())..];
    let path = sh::expand_to_pathbuf(path);
    let entries = if path.iter().count() == 0 {
        if_let_ok!(dir = current_dir(), |_| ());
        dir.read_dir()
    } else if path.is_dir() {
        path.read_dir()
    } else if let Some(dir) = path.parent() {
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
                    result.push(format!("{}~{}", prefix, &path[home.len()..]));
                } else {
                    result.push(format!("{}{}", prefix, path));
                }
            }
        }
    }
}
