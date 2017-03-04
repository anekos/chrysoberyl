
use std::thread::spawn;
use std::sync::mpsc::Sender;
use std::fs::File;
use std::process::{Command, Stdio};

use operation::Operation;



pub fn run_fifo_controller(tx: Sender<Operation>, filepath: String) {
    use std::io::{BufReader, BufRead};

    spawn(move || {
        while let Ok(file) = File::open(&filepath) {
            puts_event!("fifo_controller", "state" => "open");
            let file = BufReader::new(file);
            for line in file.lines() {
                let line = line.unwrap();
                tx.send(from_string(&line)).unwrap();
            }
            puts_event!("fifo_controller", "state" => "close");
        }
        puts_error!("at" => "file_controller", "reason" => "Could not open file", "for" => filepath);
    });
}

pub fn run_file_controller(tx: Sender<Operation>, filepath: String) {
    use std::io::{BufReader, BufRead};

    spawn(move || {
        if let Ok(file) = File::open(&filepath) {
            puts_event!("file_controller", "state" => "open");
            let file = BufReader::new(file);
            for line in file.lines() {
                let line = line.unwrap();
                tx.send(from_string(&line)).unwrap();
            }
            puts_event!("file_controller", "state" => "close");
        } else {
            puts_error!("at" => "file_controller", "reason" => "Could not open file", "for" => filepath);
        }
    });
}

pub fn run_stdin_controller(tx: Sender<Operation>) {
    use std::io;
    use std::io::BufRead;

    spawn(move || {
        let stdin = io::stdin();
        puts_event!("stdin_controller", "state" => "open");
        for line in stdin.lock().lines() {
            let line = line.unwrap();
            tx.send(from_string(&line)).unwrap();
        }
        puts_event!("stdin_controller", "state" => "close");
    });
}


pub fn run_command_controller(tx: Sender<Operation>, command: String) {
    use std::io::{BufReader, BufRead};

    spawn(move || {
        let child = Command::new(&command).stdout(Stdio::piped()).spawn().unwrap();
        puts_event!("command_controller", "state" => "open");
        if let Some(stdout) = child.stdout {
            for line in BufReader::new(stdout).lines() {
                let line = line.unwrap();
                tx.send(from_string(&line)).unwrap();
            }
            puts_event!("command_controller", "state" => "close");
        } else {
            puts_error!("at" => "command_controller", "for" => command);
        }
    });
}

fn from_string(s: &str) -> Operation {
    use std::str::FromStr;

    Operation::from_str(s).unwrap_or(Operation::Push(s.to_owned()))
}
