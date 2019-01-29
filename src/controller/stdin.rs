use std::io::{self, Read};
use std::path::PathBuf;
use std::sync::mpsc::Sender;
use std::thread::spawn;

use atty;
use rustyline::Editor;

use crate::controller::process;
use crate::joiner::Joiner;
use crate::operation::Operation;
use crate::util;



pub fn register(tx: Sender<Operation>, mut history_file: Option<PathBuf>) {
    use std::io;
    use std::io::BufRead;

    let mut readline = Editor::<()>::new();

    if let Some(history_file) = history_file.as_ref() {
        let _ = readline.load_history(history_file);
    }

    spawn(move || {
        let stdin = io::stdin();
        let mut joiner = Joiner::new();
        if atty::is(atty::Stream::Stdin) {
            puts_event!("input/stdin/open", "type" => "readline");
            while let Ok(line) = readline.readline("") {
                if let Some(line) = joiner.push(&line) {
                    if process(&tx, &*line, "input/stdin") {
                        if let Err(error) = util::file::write_line(&line, &history_file) {
                            puts_error!(error, "at" => "input/stdin/write_line");
                            history_file = None; // Do not retry
                        }
                    }
                }
            }
        } else {
            puts_event!("input/stdin/open", "type" => "standard");
            for line in stdin.lock().lines() {
                let line = line.unwrap();
                if let Some(line) = joiner.push(&line) {
                    process(&tx, &*line, "input/stdin");
                }
            }
        }
        puts_event!("input/stdin/close");
    });
}

pub fn register_as_binary(tx: Sender<Operation>) {
    spawn(move || {
        let stdin = io::stdin();
        let mut stdin = stdin.lock();
        let mut buf = vec![];
        stdin.read_to_end(&mut buf).unwrap();
        tx.send(Operation::PushMemory(buf, None, false)).unwrap();
    });
}
