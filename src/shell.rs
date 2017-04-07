
use std::borrow::Cow;
use std::io::{BufReader, BufRead};
use std::process::{Command, Stdio, Child};
use std::sync::mpsc::Sender;
use std::thread::spawn;

use onig::Regex;
use operation::Operation;
use shell_escape::escape;
use termination;



pub fn call(async: bool, command_line: &[String], tx: Option<Sender<Operation>>) {
    let command_line = {
        let mut result = o!("");
        for argument in command_line {
            result.push(' ');
            if is_variable(argument) {
                result.push_str(&format!(r#""{}""#, argument));
            } else {
                let argument = Cow::from(argument.to_owned());
                result.push_str(&escape(argument).into_owned());
            }
        }
        result
    };

    if async {
        spawn(move || run(tx, command_line));
    } else {
        run(tx, command_line);
    }
}

fn run(tx: Option<Sender<Operation>>, command_line: String) {
    let mut command = Command::new("setsid");
    command
        .args(&["bash", "-c"])
        .arg(&command_line);
    command
        .stdout(Stdio::piped())
        .stderr(Stdio::null());

    let child = command.spawn().unwrap();

    termination::register(termination::Process::Kill(child.id()));

    puts_event!("shell", "state" => "open");
    if process_stdout(tx, child) {
        puts_event!("shell", "state" => "close");
    } else {
        puts_error!("at" => "shell", "for" => command_line);
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

fn is_variable(s: &str) -> bool {
    Regex::new(r#"^\$[a-zA-Z_][a-zA-Z_0-9]*$"#).unwrap().is_match(s)
}


#[cfg(test)]#[test]
fn test_is_variable() {
    assert!(!is_variable("hoge"));
    assert!(is_variable("$file"));
    assert!(is_variable("$CHRYSOBERYL_FILE"));
    assert!(!is_variable("$CHRYSOBERYL    FILE"));
}
