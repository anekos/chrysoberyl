
use std::fs::File;
use std::io::Read;
use std::sync::mpsc::Sender;

use app_path;
use operation::Operation;


#[derive(Clone, Debug, PartialEq, Copy)]
pub enum ConfigSource {
    Default,
    User,
}


pub static DEFAULT_CONFIG: &'static str = include_str!("../res/config/default.conf");


pub fn load_config(tx: &Sender<Operation>, config_source: &ConfigSource) {
    let lines = config_lines(config_source);

    puts_event!("input/config/open");
    for line in lines {
        match Operation::parse(&line) {
            Ok(Operation::LoadConfig(ref source)) =>
                load_config(tx, source),
            Ok(op) =>
                tx.send(op).unwrap(),
            Err(err) =>
                puts_error!("at" => "input/config", "reason" => s!(err), "for" => &line),
        }
    }
    puts_event!("input/config/close");
}


pub fn config_lines(config_source: &ConfigSource) -> Vec<String> {
    fn load_default() -> Vec<String> {
        DEFAULT_CONFIG.lines().map(|it| o!(it)).collect()
    }

    match *config_source {
        ConfigSource::User =>
            if let Ok(mut file) = File::open(app_path::config_file(None)) {
                let mut source = o!("");
                file.read_to_string(&mut source).unwrap();
                source.lines().map(|it| o!(it)).collect()
            } else {
                load_default()
            },
        ConfigSource::Default =>
            load_default()
    }
}
