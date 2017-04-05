
use std::fs::File;
use std::sync::mpsc::Sender;
use std::io::{BufReader, BufRead};

use app_path;
use operation::Operation;



pub static DEFAULT_CONFIG: &'static str = "
@map key h @first
@map key j @next
@map key k @previous
@map key l @last
@map key q @quit
@map key z @shuffle
@map key e @expand
@map key E @expand --recursive
@map key R @refresh
@map key i @toggle status-bar
@map key v @views
@map key V @views --rows
@map key r @toggle reverse
@map key q @quit

@map mouse 1 @next
@map mouse 2 @cherenkov --radius 0.02 --color purple --spokes 50
@map mouse 3 @previous
";


pub fn load_config(tx: Sender<Operation>) {
    let filepath = app_path::config_file();
    if let Ok(file) = File::open(&filepath) {
        puts_event!("config_file", "state" => "open");
        let file = BufReader::new(file);
        for line in file.lines() {
            let line = line.unwrap();
            tx.send(Operation::from_str_force(&line)).unwrap();
        }
        puts_event!("config_file", "state" => "close");
    } else {
        for line in DEFAULT_CONFIG.lines() {
            tx.send(Operation::from_str_force(line)).unwrap();
        }
    }
}
