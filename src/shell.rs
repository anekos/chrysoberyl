
use std::collections::HashMap;
use std::env;
use std::io::{BufReader, BufRead, Read};
use std::process::{Command, Stdio, Child};
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
use std::thread::spawn;

use crate::chainer;
use crate::errors::ChryError;
use crate::expandable::Expandable;
use crate::operation::{Operation, ReadAs};
use crate::session::StatusText;
use crate::util::shell::escape;
use crate::util::string::join;



type Envs = HashMap<String, String>;
type Entries = Arc<Mutex<HashMap<u32, Process>>>;

#[derive(Default)]
pub struct Process {
    pub command_line: Vec<String>,
}

struct Finalizer {
    entries: Entries,
    pid: u32,
    target: chainer::Target,
}

pub struct ProcessManager {
    entries: Entries,
    tx: Sender<Operation>,
}


impl ProcessManager {
    pub fn new(tx: Sender<Operation>) -> Self {
        ProcessManager {
            entries: Entries::default(),
            tx,
        }
    }

    pub fn call(&mut self, r#async: bool, command_line: &[String], stdin: Option<String>, read_as: ReadAs) {
        let tx = if read_as != ReadAs::Ignore {
            Some(self.tx.clone())
        } else {
            None
        };
        call(self.entries.clone(), r#async, command_line, stdin, read_as, tx);
    }
}

impl StatusText for ProcessManager {
    fn write_status_text(&self, out: &mut String) {
        let entries = self.entries.lock().unwrap();
        for (pid, process) in &*entries {
            sprint!(out, "process: pid={}", pid);
            for it in &process.command_line {
                sprint!(out, " {}", escape(it));
            }
            sprintln!(out, "");
        }
    }
}


impl  Finalizer {
    fn new(entries: Entries, pid: u32, command_line: Vec<String>) -> Finalizer {
        {
            let mut entries = entries.lock().unwrap();
            entries.insert(pid, Process { command_line });
        }

        let target = chainer::Target::Process(pid);
        chainer::register(target.clone());

        Finalizer { entries, pid, target }
    }

    fn finalize(self) {
        let mut entries = self.entries.lock().unwrap();
        entries.remove(&self.pid);
        chainer::unregister(&self.target);
    }
}


fn call(entries: Entries, r#async: bool, command_line: &[String], stdin: Option<String>, read_as: ReadAs, tx: Option<Sender<Operation>>) {
    let envs = if r#async { Some(get_envs()) } else { None };

    if r#async {
        let command_line = command_line.to_vec();
        spawn(move || run(entries, tx, envs, &command_line, stdin, read_as));
    } else {
        run(entries, tx, envs, command_line, stdin, read_as);
    }
}

fn run(entries: Entries, tx: Option<Sender<Operation>>, envs: Option<Envs>, command_line: &[String], stdin: Option<String>, read_as: ReadAs) {
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

    let finalizer = Finalizer::new(entries, child.id(), command_line.to_vec());

    puts_event!("shell/open");
    match process_stdout(tx, child, stdin, read_as) {
        Ok(_) => puts_event!("shell/close"),
        Err(err) => puts_error!(err, "at" => "shell", "for" => join(command_line, ',')),
    }

    finalizer.finalize();
}

fn process_stdout(tx: Option<Sender<Operation>>, child: Child, stdin: Option<String>, read_as: ReadAs) -> Result<(), ChryError> {
    use std::io::Write;

    if let Some(stdin) = stdin {
        child.stdin.unwrap().write_all(stdin.as_bytes()).unwrap();
    }

    if let Some(tx) = tx {
        let stderr = child.stderr;
        spawn(move || pass("stderr", stderr));
        if let Some(mut stdout) = child.stdout {
            match read_as {
                ReadAs::Binary => {
                    let mut buffer = vec![];
                    match stdout.read_to_end(&mut buffer) {
                        Ok(_) => tx.send(Operation::PushMemory(buffer, None, false)).unwrap(),
                        Err(err) => puts_error!(err, "at" => "shell_stdout/as_binary"),
                    }
                },
                ReadAs::Operations => {
                    for line in BufReader::new(stdout).lines() {
                        let line = line.unwrap();
                        match Operation::parse_fuzziness(&line) {
                            Ok(op) => tx.send(op).unwrap(),
                            Err(err) => puts_error!(err, "at" => "shell_stdout", "for" => &line),
                        }
                    }
                },
                ReadAs::Paths => {
                    for line in BufReader::new(stdout).lines() {
                        let line = line.unwrap();
                        tx.send(Operation::Push(Expandable::new(line), None, false, false)).unwrap();
                    }
                },
                ReadAs::Ignore => panic!("WTF: read_as == Ignore"),
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

fn get_envs() -> Envs {
    let mut result = HashMap::new();

    for (key, value) in env::vars_os() {
        if let (Ok(key), Ok(value)) = (key.into_string(), value.into_string()) {
            result.insert(key, value);
        }
    }

    result
}
