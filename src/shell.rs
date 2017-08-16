
use std::io::{BufReader, BufRead, Read};
use std::process::{Command, Stdio, Child};
use std::sync::mpsc::Sender;
use std::thread::spawn;

use operation::Operation;
use utils::join;



pub fn call(async: bool, command_line: &[String], stdin: Option<String>, tx: Option<Sender<Operation>>) {
    if async {
        let command_line = command_line.to_vec();
        spawn(move || run(tx, &command_line, stdin));
    } else {
        run(tx, command_line, stdin);
    }
}

fn run(tx: Option<Sender<Operation>>, command_line: &[String], stdin: Option<String>) {
    let (command_name, args) = command_line.split_first().expect("WTF: Empty command line");

    let mut command = Command::new(command_name);
    command
        .args(args);
    command
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    command.stdin(if stdin.is_some() { Stdio::piped() } else { Stdio::null() });

    let child = command.spawn().unwrap();

    // let terminator = termination::Process::Kill(child.id());
    // termination::register(terminator.clone());
    //
    puts_event!("shell/open");
    if process_stdout(tx, child, stdin) {
        puts_event!("shell/close");
    } else {
        puts_error!("at" => "shell", "for" => join(command_line, ','));
    }

    // termination::unregister(&terminator);
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
