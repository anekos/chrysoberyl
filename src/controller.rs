
use std::error::Error;
use std::fs::{OpenOptions, File, create_dir_all};
use std::io::{Write, BufReader, BufRead};
use std::path::PathBuf;
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

pub fn register_stdin(tx: Sender<Operation>, mut history_file: Option<PathBuf>) {
    use std::io;
    use std::io::BufRead;

    if let Err(ref error) = setup_readline(&history_file) {
        puts_error!(error, "at" => "input/stdin/setup_readline");
        history_file = None;
    }

    spawn(move || {
        let stdin = io::stdin();
        if atty::is(atty::Stream::Stdin) {
            puts_event!("input/stdin/open", "type" => "readline");
            while let Ok(line) = readline::readline("") {
                let _ = readline::add_history(&*line);
                if process(&tx, &*line) {
                    if let Err(error) = write_line(&line, &history_file) {
                        puts_error!(error, "at" => "input/stdin/write_line");
                        history_file = None; // Do not retry
                    }
                }
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

fn process(tx: &Sender<Operation>, line: &str) -> bool {
    match Operation::parse_fuzziness(line) {
        Ok(op) => {
            tx.send(op).unwrap();
            true
        }
        Err(err) => {
            puts_error!(err, "at" => "input/stdin", "for" => line);
            false
        }
    }
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

fn write_line(line: &str, file: &Option<PathBuf>) -> Result<(), Box<Error>> {
    if_let_some!(file = file.as_ref(), Ok(()));
    let mut file = OpenOptions::new().read(false).write(true).append(true).create(true).open(file)?;
    write!(file, "{}\n", line)?;
    Ok(())
}
