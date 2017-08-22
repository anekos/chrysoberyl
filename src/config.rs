
use std::fs::{create_dir_all, File};
use std::io::{Read, Write};

use app_path;



pub static DEFAULT_CONFIG: &'static str = include_str!("static/default.chry");


pub fn get_config_source() -> String {
    let file = app_path::config_file(None);
    let mut out = o!("");
    match File::open(&file).and_then(|mut file| file.read_to_string(&mut out)) {
        Ok(_) => out,
        Err(_) => {
            let dir = file.parent().unwrap();
            let _ = create_dir_all(dir).and_then(|_| {
                File::create(&file).and_then(|mut file| {
                    file.write_all(DEFAULT_CONFIG.as_bytes())
                })
            });
            o!(DEFAULT_CONFIG)
        }
    }
}
