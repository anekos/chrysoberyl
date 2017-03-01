
use std::thread::spawn;
use std::sync::mpsc::Sender;
use std::fs::File;

use operation::Operation;



pub fn run_file_controller(tx: Sender<Operation>, filepath: String) {
    use std::io::{BufReader, BufRead};

    spawn(move || {
        match File::open(&filepath) {
            Ok(file) => {
                let file = BufReader::new(file);
                for line in file.lines() {
                    let line = line.unwrap();
                    tx.send(from_string(&line)).unwrap();
                }
            }
            Err(err) => puts_error!("at" => "file_controller", "reason" => err)
        }
    });
}

pub fn run_stdin_controller(tx: Sender<Operation>) {
    use std::io;
    use std::io::BufRead;

    spawn(move || {
        let stdin = io::stdin();
        for line in stdin.lock().lines() {
            let line = line.unwrap();
            tx.send(from_string(&line)).unwrap();
        }
    });
}


fn from_string(s: &str) -> Operation {
    use std::str::FromStr;

    Operation::from_str(s).unwrap_or(Operation::Push(s.to_owned()))
}
