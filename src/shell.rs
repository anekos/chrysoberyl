
use std::fmt::Write;
use std::io::{BufReader, BufRead, Read};
use std::process::{Command, Stdio, Child};
use std::sync::mpsc::Sender;
use std::thread::spawn;

use operation::Operation;
use termination;



pub fn call(async: bool, command_line: &[String], stdin: Option<String>, tx: Option<Sender<Operation>>) {
    if async {
        let command_line = command_line.to_vec();
        spawn(move || run(tx, &command_line, stdin));
    } else {
        run(tx, command_line, stdin);
    }
}

fn run(tx: Option<Sender<Operation>>, command_line: &[String], stdin: Option<String>) {
    let mut command = Command::new("setsid");
    command
        .args(command_line);
    command
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    if stdin.is_some() {
        command.stdin(Stdio::piped());
    }

    let child = command.spawn().unwrap();

    termination::register(termination::Process::Kill(child.id()));

    puts_event!("shell/open");
    if process_stdout(tx, child, stdin) {
        puts_event!("shell/close");
    } else {
        puts_error!("at" => "shell", "for" => join(command_line));
    }
}

fn process_stdout(tx: Option<Sender<Operation>>, child: Child, stdin: Option<String>) -> bool {
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
                    Err(err) => puts_error!("at" => "shell_stdout", "reason" => err, "for" => &line)
                }
            }
        } else {
            return false
        }
    } else {
        let stderr = child.stderr;
        spawn(move || pass("stderr", stderr));
        pass("stdout", child.stdout);
    }
    true
}

fn pass<T: Read + Send>(source: &str, out: Option<T>) {
    if let Some(out) = out {
        for line in BufReader::new(out).lines() {
            let line = line.unwrap();
            puts_event!(format!("shell/{}", source), "line" => line);
        }
    }
}

fn join(xs: &[String]) -> String {
    let mut result = o!("");
    for x in xs {
        write!(result, "{},", x).unwrap();
    }
    result.pop();
    result
}
