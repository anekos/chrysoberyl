
use std::collections::HashMap;
use std::env;
use std::io::{BufReader, BufRead, Read};
use std::process::{Command, Stdio, Child};
use std::sync::mpsc::Sender;
use std::thread::spawn;

use errors::ChryError;
use operation::Operation;
use termination;
use util::string::join;



type Envs = HashMap<String, String>;

pub fn call(async: bool, command_line: &[String], stdin: Option<String>, as_binary: bool, tx: Option<Sender<Operation>>) {
    let envs = if async { Some(store_envs()) } else { None };

    if async {
        let command_line = command_line.to_vec();
        spawn(move || run(tx, envs, &command_line, stdin, as_binary));
    } else {
        run(tx, envs, command_line, stdin, as_binary);
    }
}

fn run(tx: Option<Sender<Operation>>, envs: Option<Envs>, command_line: &[String], stdin: Option<String>, as_binary: bool) {
    let mut command = Command::new("setsid");
    command
        .args(command_line);
    command
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    command.stdin(if stdin.is_some() { Stdio::piped() } else { Stdio::null() });

    if let Some(envs) = envs {
        command.env_clear().envs(envs);
    }

    let child = command.spawn().unwrap();

    let terminator = termination::Process::Kill(child.id());
    termination::register(terminator.clone());

    puts_event!("shell/open");
    match process_stdout(tx, child, stdin, as_binary) {
        Ok(_) => puts_event!("shell/close"),
        Err(err) => puts_error!(err, "at" => "shell", "for" => join(command_line, ',')),
    }

    termination::unregister(&terminator);
}

fn process_stdout(tx: Option<Sender<Operation>>, child: Child, stdin: Option<String>, as_binary: bool) -> Result<(), ChryError> {
    use std::io::Write;

    if let Some(stdin) = stdin {
        child.stdin.unwrap().write_all(stdin.as_bytes()).unwrap();
    }

    if let Some(tx) = tx {
        let stderr = child.stderr;
        spawn(move || pass("stderr", stderr));
        if let Some(mut stdout) = child.stdout {
            if as_binary {
                let mut buffer = vec![];
                match stdout.read_to_end(&mut buffer) {
                    Ok(_) => tx.send(Operation::PushMemory(buffer, None, false)).unwrap(),
                    Err(err) => puts_error!(err, "at" => "shell_stdout/as_binary"),
                }
            } else {
                for line in BufReader::new(stdout).lines() {
                    let line = line.unwrap();
                    puts_event!("shell/stdout", "line" => line);
                    match Operation::parse_fuzziness(&line) {
                        Ok(op) => tx.send(op).unwrap(),
                        Err(err) => puts_error!(err, "at" => "shell_stdout", "for" => &line),
                    }
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
