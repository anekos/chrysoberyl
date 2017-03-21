
use std::fs::File;
use std::sync::mpsc::Sender;
use std::io::{BufReader, BufRead};

use app_path;
use operation::Operation;



static DEFAULT_CONFIG: &'static str = "
@map keyboard h @first
@map keyboard j @next
@map keyboard k @previous
@map keyboard l @last
@map keyboard q @quit
@map keyboard z @shuffle
@map keyboard e @expand
@map keyboard E @expand --recursive
@map keyboard R @refresh
@map keyboard i @toggle information
@map keyboard v @views
@map keyboard r @toggle reverse
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
            tx.send(Operation::from_str_force(&line)).unwrap();
        }
    }
}
