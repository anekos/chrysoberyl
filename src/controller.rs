
use std::fs::File;
use std::process::{Command, Stdio};
use std::sync::mpsc::Sender;
use std::thread::spawn;

use operation::Operation;
use operation_utils::read_operations;
use termination;



pub struct Controllers {
    pub inputs: Vec<String>,
    pub fragiles: Vec<String>,
    pub commands: Vec<String>
}


impl Controllers {
    pub fn new() -> Controllers {
        Controllers {
            inputs: vec![],
            fragiles: vec![],
            commands: vec![],
        }
    }
}



pub fn register(tx: Sender<Operation>, controllers: &Controllers) {
    for path in &controllers.inputs {
        file_controller(tx.clone(), path.clone());
    }
    for path in &controllers.fragiles {
        fifo_controller(tx.clone(), path.clone());
    }
    for path in &controllers.commands {
        command_controller(tx.clone(), path.clone());
    }

    stdin_controller(tx.clone());
}


fn fifo_controller(tx: Sender<Operation>, filepath: String) {
    spawn(move || {
        while let Ok(file) = File::open(&filepath) {
            puts_event!("fifo_controller", "state" => "open");
            read_operations(file, tx.clone());
            puts_event!("fifo_controller", "state" => "close");
        }
        puts_error!("at" => "fifo_controller", "reason" => "Could not open file", "for" => filepath);
    });
}

fn file_controller(tx: Sender<Operation>, filepath: String) {
    spawn(move || {
        if let Ok(file) = File::open(&filepath) {
            puts_event!("file_controller", "state" => "open");
            read_operations(file, tx.clone());
            puts_event!("file_controller", "state" => "close");
        } else {
            puts_error!("at" => "file_controller", "reason" => "Could not open file", "for" => filepath);
        }
    });
}

fn stdin_controller(tx: Sender<Operation>) {
    use std::io;
    use std::io::BufRead;

    spawn(move || {
        let stdin = io::stdin();
        puts_event!("stdin_controller", "state" => "open");
        for line in stdin.lock().lines() {
            let line = line.unwrap();
            tx.send(Operation::from_str_force(&line)).unwrap();
        }
        puts_event!("stdin_controller", "state" => "close");
    });
}


fn command_controller(tx: Sender<Operation>, command: String) {
    use std::io::{BufReader, BufRead};

    spawn(move || {
        let child = Command::new("setsid")
            .arg("bash")
            .arg("-c")
            .arg(&command)
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn().unwrap();

        puts_event!("command_controller", "state" => "open");

        termination::register(termination::Process::Kill(child.id()));

        if let Some(stdout) = child.stdout {
            for line in BufReader::new(stdout).lines() {
                let line = line.unwrap();
                tx.send(Operation::from_str_force(&line)).unwrap();
            }
            puts_event!("command_controller", "state" => "close");
        } else {
            puts_error!("at" => "command_controller", "for" => command);
        }
    });
}
