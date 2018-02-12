
use std::error::Error;
use std::fs::{create_dir_all, File, OpenOptions};
use std::io::{Read, Write};
use std::path::Path;

use app_path;



pub static DEFAULT_CONFIG: &'static str = include_str!("static/default.chry");


pub fn get_config_source<T: AsRef<Path>>(path: Option<&T>) -> String {
    let default = app_path::config_file();
    let path = path.map(|it| it.as_ref()).unwrap_or_else(|| default.as_path());
    read_config(&path).map_err(|err| puts_error!(err, "file" => p!(path))).unwrap_or_else(|_| {
        let _ = create_default(); // Ignore any errors
        o!(DEFAULT_CONFIG)
    })
}

fn read_config<T: AsRef<Path>>(path: &T) -> Result<String, Box<Error>> {
    let mut file = File::open(path)?;
    let mut out = o!("");
    file.read_to_string(&mut out)?;
    Ok(out)
}

fn create_default() -> Result<(), Box<Error>> {
    let file = app_path::config_file();
    let dir = file.parent().ok_or("WTF: Unexpected config path")?;
    create_dir_all(dir)?;
    let mut file = OpenOptions::new().read(true).write(false).append(false).create(false).open(&file)?;
    file.write_all(DEFAULT_CONFIG.as_bytes())?;
    Ok(())
}
