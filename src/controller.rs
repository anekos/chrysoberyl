
use std::error::Error;
use std::fs::File;
use std::sync::mpsc::Sender;
use std::thread::spawn;

use atty;
use readline;

use errors::ChryError;
use operation::Operation;
use operation_utils::read_operations;



pub fn register_file(tx: Sender<Operation>, filepath: String) {
    spawn(move || {
        if let Ok(file) = File::open(&filepath) {
            puts_event!("input/file/open");
            read_operations("file", file, &tx);
            puts_event!("input/file/close");
        } else {
            puts_error!(ChryError::from("Could not open file"), "at" => "input/file", "for" => filepath);
        }
    });
}

pub fn register_stdin(tx: Sender<Operation>) {
    use std::io;
    use std::io::BufRead;

    fn process(tx: &Sender<Operation>, line: &str) {
        match Operation::parse_fuzziness(line) {
            Ok(op) => tx.send(op).unwrap(),
            Err(err) => puts_error!(err, "at" => "input/stdin", "for" => line)
        }
    }

    spawn(move || {
        let stdin = io::stdin();
        if atty::is(atty::Stream::Stdin) {
            puts_event!("input/stdin/open", "type" => "readline");
            while let Ok(ref line) = readline::readline("") {
                let _ = readline::add_history(line);
                process(&tx, line);
            }
        } else {
            puts_event!("input/stdin/open", "type" => "standard");
            for line in stdin.lock().lines() {
                let line = line.unwrap();
                process(&tx, &*line);
            }
        }
        puts_event!("input/stdin/close");
    });
}
