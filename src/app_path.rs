
use std::default::Default;
use std::fmt;
use std::fs::create_dir_all;
use std::path::{PathBuf, Path};

use app_dirs::*;

use errors::ChryError;
use option::OptionValue;
use shellexpand_wrapper as sh;
use util::path::path_to_str;



const APP_INFO: AppInfo = AppInfo { name: "chrysoberyl", author: "anekos" };


pub struct PathList {
    pub entries: Vec<PathBuf>
}


pub fn cache_dir(path: &str) -> PathBuf {
    let dir = get_app_dir(AppDataType::UserCache, &APP_INFO, path).unwrap();
    if !dir.exists() {
        create_dir_all(&dir).unwrap();
    }
    dir
}

fn config_dir() -> PathBuf {
     get_app_root(AppDataType::UserConfig, &APP_INFO).unwrap()
}

pub fn config_file() -> PathBuf {
    let file = get_app_dir(AppDataType::UserConfig, &APP_INFO, "config.chry").unwrap();
    {
        let dir = file.parent().unwrap();
        if !dir.exists() {
            create_dir_all(&dir).unwrap();
        }
    }
    file
}

pub fn search_path<T: AsRef<Path>>(filename: &T, path_list: &PathList) -> PathBuf {
    for path in &path_list.entries {
        let mut path = path.clone();
        path.push(filename);
        if path.exists() {
            return path;
        }
    }

    Path::new(filename.as_ref()).to_path_buf()
}

pub fn entry_history() -> PathBuf {
    let mut path = cache_dir("history");
    path.push("entry.log");
    path
}


impl OptionValue for PathList {
    fn unset(&mut self) -> Result<(), ChryError> {
        *self = PathList::default();
        Ok(())
    }

    fn set(&mut self, value: &str) -> Result<(), ChryError> {
        self.entries = value.split(':').map(sh::expand_to_pathbuf).collect();
        Ok(())
    }
}

impl Default for PathList {
    fn default() -> Self {
        let mut entries = vec![];
        entries.push(config_dir());
        entries.push(Path::new("/usr/share/chrysoberyl").to_path_buf());
        if let Ok(entry) = get_app_root(AppDataType::UserCache, &APP_INFO) {
            entries.push(entry);
        }
        PathList { entries }
    }
}

impl fmt::Display for PathList {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let last = self.entries.len() - 1;
        for (i, entry) in self.entries.iter().enumerate() {
            let result = write!(f, "{}{}", path_to_str(entry), if i == last { "" } else { ":" });
            if result.is_err() {
                return result;
            }
        }
        Ok(())
    }
}
