
use std::io::{BufReader, BufRead, Read};
use std::sync::mpsc::Sender;

use operation::Operation;

mod fifo;
pub mod file;
pub mod stdin;


#[derive(Clone)]
pub enum Source {
    Fifo(String),
    File(String),
}


pub fn register(tx: Sender<Operation>, source: Source) {
    use self::Source::*;

    match source {
        Fifo(path) => fifo::register(tx, &path),
        File(path) => file::register(tx, &path),
    }
}


fn process(tx: &Sender<Operation>, line: &str, at: &'static str) -> bool {
    match Operation::parse_fuzziness(line) {
        Ok(op) => {
            tx.send(op).unwrap();
            true
        }
        Err(err) => {
            puts_error!(err, "at" => at, "for" => line);
            false
        }
    }
}

fn process_lines<T: Read>(tx: &Sender<Operation>, source: T, at: &'static str) {
    for line in BufReader::new(source).lines() {
        let line = line.unwrap();
        process(tx, &line, at);
    }
}
