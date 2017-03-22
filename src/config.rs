
use std::fs::File;
use std::sync::mpsc::Sender;
use std::io::{BufReader, BufRead};

use app_path;
use operation::Operation;



static DEFAULT_CONFIG: &'static str = "
@map h @first
@map j @next
@map k @previous
@map l @last
@map q @quit
@map z @shuffle
@map e @expand
@map E @expand --recursive
@map R @refresh
@map i @toggle information
@map v @views
@map r @toggle reverse
@map q @quit

@map --mouse-button 1 @next
@map --mouse-button 3 @previous
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
