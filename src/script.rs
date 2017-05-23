
use std::fs::File;
use std::io::{self, Read};
use std::path::PathBuf;
use std::sync::mpsc::Sender;

use app_path;
use operation::Operation;


#[derive(Clone, Debug, PartialEq)]
pub enum ScriptSource {
    File(PathBuf),
    Config(ConfigSource)
}

#[derive(Clone, Debug, PartialEq, Copy)]
pub enum ConfigSource {
    Default,
    User,
    DefaultSession,
}


pub static DEFAULT_CONFIG: &'static str = include_str!("../res/config/default.conf");


pub fn load(tx: &Sender<Operation>, script_source: &ScriptSource) {
    match script_lines(script_source) {
        Ok(lines) => {
            puts_event!("input/script/open");
            for line in lines {
                match Operation::parse(&line) {
                    Ok(Operation::Load(ref source)) =>
                        load(tx, source),
                    Ok(op) =>
                        tx.send(op).unwrap(),
                    Err(err) =>
                        puts_error!("at" => "input/script", "reason" => s!(err), "for" => &line),
                }
            }
            puts_event!("input/script/close");
        }
        Err(err) =>
            puts_error!("at" => "load", "reason" => o!(err)),
    }

}


pub fn script_lines(script_source: &ScriptSource) -> Result<Vec<String>, String> {
    fn load_default() -> Vec<String> {
        DEFAULT_CONFIG.lines().map(|it| o!(it)).collect()
    }

    fn load_from_file(path: &PathBuf) -> Result<Vec<String>, io::Error> {
        File::open(path).map(|mut file| {
            let mut source = o!("");
            file.read_to_string(&mut source).unwrap();
            source.lines().map(|it| o!(it)).collect()
        })
    }

    match *script_source {
        ScriptSource::File(ref path) =>
            load_from_file(path).map_err(|it| s!(it)),
        ScriptSource::Config(ConfigSource::User) =>
            Ok(load_from_file(&app_path::config_file(None)).unwrap_or_else(|_| load_default())),
        ScriptSource::Config(ConfigSource::DefaultSession) =>
            Ok(load_from_file(&app_path::config_file(Some(app_path::DEFAULT_SESSION_FILENAME))).unwrap_or_else(|_| load_default())),
        ScriptSource::Config(ConfigSource::Default) =>
            Ok(load_default())
    }
}
