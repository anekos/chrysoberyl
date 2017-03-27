
use std::fs::create_dir_all;
use std::path::PathBuf;

use app_dirs::*;


const APP_INFO: AppInfo = AppInfo { name: "chrysoberyl", author: "anekos" };



pub fn cache_dir(path: &str) -> PathBuf {
    let dir = get_app_dir(AppDataType::UserCache, &APP_INFO, path).unwrap();
    if !dir.exists() {
        create_dir_all(&dir).unwrap();
    }
    dir
}

pub fn config_file() -> PathBuf {
    let file = get_app_dir(AppDataType::UserConfig, &APP_INFO, "rc.conf").unwrap();
    {
        let dir = file.parent().unwrap();
        if !dir.exists() {
            create_dir_all(&dir).unwrap();
        }
    }
    file
}