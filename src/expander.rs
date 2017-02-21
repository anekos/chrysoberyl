

use std::path::PathBuf;



pub fn expand(filepath: PathBuf) -> Vec<PathBuf> {
    let mut result = vec![];

    if let Some(dir) = filepath.parent() {
        for entry in dir.read_dir().unwrap() {
            if let Ok(entry) = entry {
                result.push(entry.path());
            }
        }
    }

    result
}
