
use std::error::Error;
use std::io::{BufReader, BufRead, Read};
use std::sync::mpsc::Sender;

use expandable::Expandable;
use operation::Operation;

pub mod fifo;
pub mod file;
pub mod stdin;
pub mod unix_socket;


#[derive(Clone)]
pub enum Source {
    Fifo(Expandable),
    File(Expandable),
    UnixSocket(Expandable, bool),
}


pub fn register(tx: Sender<Operation>, source: Source) -> Result<(), Box<Error>> {
    use self::Source::*;

    match source {
        Fifo(path) => fifo::register(tx, &path.expand()),
        File(path) => file::register(tx, &path.expand()),
        UnixSocket(path, true) => unix_socket::register_as_binary(tx, &path.expand())?,
        UnixSocket(path, _) => unix_socket::register(tx, &path.expand())?,
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
