
use std::fs::create_dir_all;
use std::path::{PathBuf, Path};

use app_dirs::*;


const APP_INFO: AppInfo = AppInfo { name: "chrysoberyl", author: "anekos" };
pub const DEFAULT_SESSION_FILENAME: &'static str = "default";



pub fn cache_dir(path: &str) -> PathBuf {
    let dir = get_app_dir(AppDataType::UserCache, &APP_INFO, path).unwrap();
    if !dir.exists() {
        create_dir_all(&dir).unwrap();
    }
    dir
}

pub fn config_dir() -> PathBuf {
     get_app_root(AppDataType::UserConfig, &APP_INFO).unwrap()
}

pub fn config_file(filename: Option<&str>) -> PathBuf {
    let file = get_app_dir(AppDataType::UserConfig, &APP_INFO, filename.unwrap_or("config.chry")).unwrap();
    {
        let dir = file.parent().unwrap();
        if !dir.exists() {
            create_dir_all(&dir).unwrap();
        }
    }
    file
}

pub fn search_path<T: AsRef<Path>>(filename: &T) -> PathBuf {
    let path = filename.as_ref().to_path_buf();

    let mut conf = config_dir();
    conf.push(path.clone());
    if conf.exists() {
        return conf;
    }

    let mut share = Path::new("/usr/share/chrysoberyl").to_path_buf();
    share.push(path.clone());
    if share.exists() {
        return share;
    }

    path
}
