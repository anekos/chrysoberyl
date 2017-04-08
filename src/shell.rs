
use std::fmt::Write;
use std::io::{BufReader, BufRead};
use std::process::{Command, Stdio, Child};
use std::sync::mpsc::Sender;
use std::thread::spawn;

use operation::Operation;
use termination;



pub fn call(async: bool, command_line: &Vec<String>, tx: Option<Sender<Operation>>) {
    if async {
        let command_line = command_line.clone();
        spawn(move || run(tx, &command_line));
    } else {
        run(tx, command_line);
    }
}

fn run(tx: Option<Sender<Operation>>, command_line: &Vec<String>) {
    let mut command = Command::new("setsid");
    command
        .args(command_line);
    command
        .stdout(Stdio::piped())
        .stderr(Stdio::null());

    let child = command.spawn().unwrap();

    termination::register(termination::Process::Kill(child.id()));

    puts_event!("shell", "state" => "open");
    if process_stdout(tx, child) {
        puts_event!("shell", "state" => "close");
    } else {
        puts_error!("at" => "shell", "for" => join(command_line));
    }
}

fn process_stdout(tx: Option<Sender<Operation>>, mut child: Child) -> bool {
    if let Some(tx) = tx {
        if let Some(stdout) = child.stdout {
            for line in BufReader::new(stdout).lines() {
                let line = line.unwrap();
                tx.send(Operation::from_str_force(&line)).unwrap();
            }
        } else {
            return false
        }
    } else {
        child.wait().unwrap();
    }
    true
}

fn join(xs: &Vec<String>) -> String {
    let mut result = o!("");
    for x in xs {
        write!(result, "{},", x).unwrap();
    }
    result.pop();
    result
}
