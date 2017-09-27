
use std::error::Error;
use std::io::{BufReader, BufRead, Read};
use std::sync::mpsc::Sender;

use operation::Operation;



pub fn read_operations<T: Read>(at: &str, source: T, tx: &Sender<Operation>) {
    for line in BufReader::new(source).lines() {
        let line = line.unwrap();
        match Operation::parse_fuzziness(&line) {
            Ok(op) => tx.send(op).unwrap(),
            Err(err) => puts_error!(err, "at" => at, "for" => &line)
        }
    }
}
