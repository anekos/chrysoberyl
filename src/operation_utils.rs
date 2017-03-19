
use std::sync::mpsc::Sender;
use std::io::{BufReader, BufRead, Read};

use operation::Operation;



pub fn read_operations<T: Read>(source: T, tx: Sender<Operation>) {

    for line in BufReader::new(source).lines() {
        let line = line.unwrap();
        tx.send(Operation::from_str_force(&line)).unwrap();
    }
}
