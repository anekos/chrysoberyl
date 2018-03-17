use std::error::Error;
use std::fs::{File, create_dir_all};
use std::io::{self, Read, BufReader, BufRead};
use std::path::PathBuf;
use std::sync::mpsc::Sender;
use std::thread::spawn;

use atty;
use readline;

use controller::process;
use joiner::Joiner;
use operation::Operation;
use util;



pub fn register(tx: Sender<Operation>, mut history_file: Option<PathBuf>) {
    use std::io;
    use std::io::BufRead;

    if let Err(ref error) = setup_readline(&history_file) {
        puts_error!(error, "at" => "input/stdin/setup_readline");
        history_file = None;
    }

    spawn(move || {
        let stdin = io::stdin();
        let mut joiner = Joiner::new();
        if atty::is(atty::Stream::Stdin) {
            puts_event!("input/stdin/open", "type" => "readline");
            while let Ok(line) = readline::readline("") {
                if let Some(line) = joiner.push(&line) {
                    let _ = readline::add_history(&*line);
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
        tx.send(Operation::PushMemory(buf, None)).unwrap();
    });
}

fn setup_readline(file: &Option<PathBuf>) -> Result<(), Box<Error>> {
    if let Some(file) = file.as_ref() {
        if let Some(dir) = file.parent() {
            create_dir_all(dir)?;
        }
        if !file.exists() {
            return Ok(())
        }
        let file = File::open(file)?;
        let reader = BufReader::new(file);
        for line in reader.lines() {
            readline::add_history(&line?)?;
        }
    }

    Ok(())
}
