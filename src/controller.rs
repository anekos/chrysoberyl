
use std::fs::File;
use std::sync::mpsc::Sender;
use std::thread::spawn;

use operation::Operation;
use operation_utils::read_operations;



pub struct Controllers {
    pub inputs: Vec<String>
}


impl Controllers {
    pub fn new() -> Controllers {
        Controllers {
            inputs: vec![],
        }
    }
}



pub fn register(tx: &Sender<Operation>, controllers: &Controllers) {
    for path in &controllers.inputs {
        file_controller(tx.clone(), path.clone());
    }

    stdin_controller(tx.clone());
}


pub fn file_controller(tx: Sender<Operation>, filepath: String) {
    spawn(move || {
        if let Ok(file) = File::open(&filepath) {
            puts_event!("input/file/open");
            read_operations("file", file, &tx);
            puts_event!("input/file/close");
        } else {
            puts_error!("at" => "input/file", "reason" => "Could not open file", "for" => filepath);
        }
    });
}

fn stdin_controller(tx: Sender<Operation>) {
    use std::io;
    use std::io::BufRead;

    spawn(move || {
        let stdin = io::stdin();
        puts_event!("input/stdin/open");
        for line in stdin.lock().lines() {
            let line = line.unwrap();
            match Operation::parse_fuzziness(&line) {
                Ok(op) => tx.send(op).unwrap(),
                Err(err) => puts_error!("at" => "input/stdin", "reason" => err, "for" => &line)
            }
        }
        puts_event!("input/stdin/close");
    });
}
