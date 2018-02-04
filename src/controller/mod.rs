
use std::error::Error;
use std::io::{BufReader, BufRead, Read};
use std::sync::mpsc::Sender;

use operation::Operation;

pub mod fifo;
pub mod file;
pub mod stdin;
pub mod unix_socket;


#[derive(Clone)]
pub enum Source {
    Fifo(String),
    File(String),
    UnixSocket(String, bool),
}


pub fn register(tx: Sender<Operation>, source: Source) -> Result<(), Box<Error>> {
    use self::Source::*;

    match source {
        Fifo(path) => fifo::register(tx, &path),
        File(path) => file::register(tx, &path),
        UnixSocket(path, true) => unix_socket::register_as_file(tx, &path)?,
        UnixSocket(path, _) => unix_socket::register(tx, &path)?,
    }
    Ok(())
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
