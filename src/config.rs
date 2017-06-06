
use std::fs::File;
use std::io::Read;

use app_path;



pub static DEFAULT_CONFIG: &'static str = include_str!("../res/config/default.conf");


pub fn get_config_source() -> String {
    let file = app_path::config_file(None);
    let mut out = o!("");
    match File::open(file).and_then(|mut file| file.read_to_string(&mut out)) {
        Ok(_) => out,
        Err(_) => o!(DEFAULT_CONFIG),
    }
}
