
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
# scroll or move
@map key h @scroll --size 0.8 left  @previous
@map key j @scroll --size 0.8 down  @next
@map key k @scroll --size 0.8 up    @previous
@map key l @scroll --size 0.8 right @next

# move
@map key asciicircum @first
@map key dollar      @last
@map key g           @first --ignore-views
@map key G           @last  --ignore-views

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
