
use std::collections::HashMap;
use std::env;
use std::io::{BufReader, BufRead, Read};
use std::process::{Command, Stdio, Child};
use std::sync::mpsc::Sender;
use std::thread::spawn;

use errors::ChryError;
use operation::Operation;
use util::string::join;



type Envs = HashMap<String, String>;

pub fn call(async: bool, command_line: &[String], stdin: Option<String>, tx: Option<Sender<Operation>>) {
    let envs = if async { Some(store_envs()) } else { None };

    if async {
        let command_line = command_line.to_vec();
        spawn(move || run(tx, envs, &command_line, stdin));
    } else {
        run(tx, envs, command_line, stdin);
    }
}

fn run(tx: Option<Sender<Operation>>, envs: Option<Envs>, command_line: &[String], stdin: Option<String>) {
    let (command_name, args) = command_line.split_first().expect("WTF: Empty command line");

    let mut command = Command::new(command_name);
    command
        .args(args);
    command
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    command.stdin(if stdin.is_some() { Stdio::piped() } else { Stdio::null() });

    if let Some(envs) = envs {
        command.env_clear().envs(envs);
    }

    let child = command.spawn().unwrap();

    // let terminator = termination::Process::Kill(child.id());
    // termination::register(terminator.clone());
    //
    puts_event!("shell/open");
    match process_stdout(tx, child, stdin) {
        Ok(_) => puts_event!("shell/close"),
        Err(err) => puts_error!(err, "at" => "shell", "for" => join(command_line, ',')),
    }
    // termination::unregister(&terminator);
}

fn process_stdout(tx: Option<Sender<Operation>>, child: Child, stdin: Option<String>) -> Result<(), ChryError> {
    use std::io::Write;

    if let Some(stdin) = stdin {
        child.stdin.unwrap().write_all(stdin.as_bytes()).unwrap();
    }

    if let Some(tx) = tx {
        let stderr = child.stderr;
        spawn(move || pass("stderr", stderr));
        if let Some(stdout) = child.stdout {
            for line in BufReader::new(stdout).lines() {
                let line = line.unwrap();
                puts_event!("shell/stdout", "line" => line);
                match Operation::parse_fuzziness(&line) {
                    Ok(op) => tx.send(op).unwrap(),
                    Err(err) => puts_error!(err, "at" => "shell_stdout", "for" => &line)
                }
            }
        } else {
            return Err("Could not get stdout")?;
        }
    } else {
        let stderr = child.stderr;
        spawn(move || pass("stderr", stderr));
        pass("stdout", child.stdout);
    }
    Ok(())
}

fn pass<T: Read + Send>(source: &str, out: Option<T>) {
    if let Some(out) = out {
        for line in BufReader::new(out).lines() {
            let line = line.unwrap();
            puts_event!(format!("shell/{}", source), "line" => line);
        }
    }
}

fn store_envs() -> Envs {
    let mut result = HashMap::new();

    for (key, value) in env::vars_os() {
        if let (Ok(key), Ok(value)) = (key.into_string(), value.into_string()) {
            result.insert(key, value);
        }
    }

    result
}
