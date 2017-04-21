
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


pub static DEFAULT_CONFIG: &'static str = "
# move
@map key h @first
@map key j @next
@map key k @previous
@map key l @last
@map key H @first --ignore-views
@map key J @next --ignore-views
@map key K @previous --ignore-views
@map key L @last --ignore-views

# option
@map key a @toggle auto-paging
@map key i @toggle status-bar
@map key f @toggle fit
@map key r @toggle reverse
@map key c @toggle center
@map key v @views
@map key V @views --rows

# entries
@map key z @shuffle
@map key e @expand
@map key E @expand --recursive
@map key R @refresh
@map key X @clear

# misc
@map key q @quit
@map key q @quit
@map key Escape @count

@map mouse 1 @next
@map mouse 2 @cherenkov --radius 0.02 --color purple --spokes 50
@map mouse 3 @previous
";


pub fn load_config(tx: &Sender<Operation>, config_source: &ConfigSource) {
    let lines = config_lines(config_source);

    puts_event!("input/config/open");
    for line in lines {
        match Operation::parse(&line) {
            Ok(op) => tx.send(op).unwrap(),
            Err(err) => puts_error!("at" => "input/config", "reason" => s!(err), "for" => &line),
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
            if let Ok(mut file) = File::open(app_path::config_file()) {
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
