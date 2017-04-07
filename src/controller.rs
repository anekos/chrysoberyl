
use std::fs::File;
use std::sync::mpsc::Sender;
use std::thread::spawn;

use operation::Operation;
use operation_utils::read_operations;



pub struct Controllers {
    pub inputs: Vec<String>,
    pub fragiles: Vec<String>
}


impl Controllers {
    pub fn new() -> Controllers {
        Controllers {
            inputs: vec![],
            fragiles: vec![]
        }
    }
}



pub fn register(tx: Sender<Operation>, controllers: &Controllers) {
    for path in &controllers.inputs {
        file_controller(tx.clone(), path.clone());
    }
    for path in &controllers.fragiles {
        fragile_controller(tx.clone(), path.clone());
    }

    stdin_controller(tx.clone());
}


fn fragile_controller(tx: Sender<Operation>, filepath: String) {
    spawn(move || {
        while let Ok(file) = File::open(&filepath) {
            puts_event!("fragile_controller", "state" => "open");
            read_operations(file, tx.clone());
            puts_event!("fragile_controller", "state" => "close");
        }
        puts_error!("at" => "fragile_controller", "reason" => "Could not open file", "for" => filepath);
    });
}

pub fn file_controller(tx: Sender<Operation>, filepath: String) {
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
