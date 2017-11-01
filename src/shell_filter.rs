
use std::io::{Write, BufReader, BufRead, BufWriter};
use std::process::{Command, Stdio};
use std::sync::mpsc::{Sender, channel};
use std::thread::spawn;

use logger;
use operation::Operation;



pub fn start(command_line: Vec<String>, tx: Sender<Operation>) {
    spawn(move || main(command_line, tx));
}

fn main(command_line: Vec<String>, tx: Sender<Operation>) {
    let mut command = Command::new("setsid");
    command.args(command_line);
    command.stdin(Stdio::piped());
    command.stdout(Stdio::piped());

    let child = command.spawn().unwrap();

    let stdin = child.stdin.unwrap();
    let stdout = child.stdout.unwrap();

    let stdout_handle = spawn(move || {
        let reader = BufReader::new(stdout);
        for line in reader.lines() {
            if let Ok(line) = line {
                match Operation::parse(&line) {
                    Ok(op) =>
                        tx.send(op).unwrap(),
                    Err(err) =>
                        puts_error!(err, "at" => "filter", "for" => &line),
                }
            }
        }
    });

    let (tx, rx) = channel();
    let output_handle = logger::register(tx);

    let stdin_handle = spawn(move || {
        let mut writer = BufWriter::new(stdin);
        while let Ok(s) = rx.recv() {
            if writer.write_fmt(format_args!("{}\n", s)).is_err() || writer.flush().is_err() {
                return;
            }
        }
    });

    stdout_handle.join().unwrap();
    stdin_handle.join().unwrap();

    logger::unregister(output_handle);
}
