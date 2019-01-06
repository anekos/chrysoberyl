
use std::collections::VecDeque;
use std::sync::mpsc::{Sender, channel};
use std::sync::{Arc, Mutex};
use std::thread::spawn;

use crate::logger;



const LIMIT: usize = 1000;
type Buffer = Arc<Mutex<VecDeque<String>>>;


pub struct Memory {
    pub buffer: Buffer,
}


impl Memory {
    pub fn new() -> Self {
        let buffer = Arc::new(Mutex::new(VecDeque::with_capacity(LIMIT)));
        let tx = main(buffer.clone());
        logger::register(tx);
        Memory { buffer }
    }
}

fn main(buffer: Buffer) -> Sender<String> {
    let (tx, rx) = channel();

    spawn(move || {
        while let Ok(s) = rx.recv() {
            let mut buffer = buffer.lock().unwrap();
            buffer.push_back(s);
        }
    });

    tx
}
