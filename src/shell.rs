
use std::borrow::Cow;
use std::process::{Command, ChildStdout, Stdio};
use std::sync::mpsc::Sender;
use std::thread::spawn;

use onig::Regex;
use operation::Operation;
use operation_utils::read_operations;
use shell_escape::escape;



pub fn call(async: bool, command_name: &str, arguments: &Vec<String>, info: Vec<(String, String)>, tx: Option<Sender<Operation>>) {
    let mut command = Command::new("bash");
    let mut command_line = command_name.to_owned();

    for argument in arguments {
        command_line.push(' ');
        if is_variable(argument) {
            command_line.push_str(&format!(r#""{}""#, argument));
        } else {
            let argument = Cow::from(argument.to_owned());
            command_line.push_str(&escape(argument).into_owned());
        }
    }

    command.arg("-c").arg(command_line);

    for (key, value) in info {
        command.env(format!("Chrysoberyl_{}", key).to_uppercase(), value);
    }

    let child = command.stdout(Stdio::piped()).spawn().expect(&*format!("Failed to run: {}", command_name));
    if async {
        spawn(move || read_from_stdout(child.stdout, tx));
    } else {
        read_from_stdout(child.stdout, tx);
    }
}

fn read_from_stdout(stdout: Option<ChildStdout>, tx: Option<Sender<Operation>>) {
    if let Some(stdout) = stdout {
        if let Some(tx) = tx {
            read_operations(stdout, tx);
        }
    }
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
