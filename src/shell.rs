
use std::collections::HashMap;
use std::env;
use std::io::{BufReader, BufRead, Read};
use std::process::{Command, Stdio, Child};
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
use std::thread::spawn;

use errors::ChryError;
use operation::Operation;
use termination;
use util::string::join;



type Envs = HashMap<String, String>;
type Entries = Arc<Mutex<HashMap<u32, Process>>>;

#[derive(Default)]
pub struct Process {
    pub command_line: Vec<String>,
}

struct Finalizer {
    entries: Entries,
    pid: u32,
    process: termination::Process,
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

    pub fn call(&mut self, async: bool, command_line: &[String], stdin: Option<String>, as_binary: bool, read_operations: bool) {
        let tx = if as_binary || read_operations {
            Some(self.tx.clone())
        } else {
            None
        };
        call(self.entries.clone(), async, command_line, stdin, as_binary, tx);
    }

    pub fn each<F>(&self, mut block: F) where F: FnMut((&u32, &Process)) -> () {
        let entries = self.entries.lock().unwrap();
        for pair in &*entries {
            block(pair)
        }
    }
}


impl  Finalizer {
    fn new(entries: Entries, pid: u32, command_line: Vec<String>) -> Finalizer {
        {
            let mut entries = entries.lock().unwrap();
            entries.insert(pid, Process { command_line });
        }

        let process = termination::Process::Kill(pid);
        termination::register(process.clone());

        Finalizer { entries, pid, process }
    }

    fn finalize(self) {
        let mut entries = self.entries.lock().unwrap();
        entries.remove(&self.pid);
        termination::unregister(&self.process);
    }
}


fn call(entries: Entries, async: bool, command_line: &[String], stdin: Option<String>, as_binary: bool, tx: Option<Sender<Operation>>) {
    let envs = if async { Some(get_envs()) } else { None };

    if async {
        let command_line = command_line.to_vec();
        spawn(move || run(entries, tx, envs, &command_line, stdin, as_binary));
    } else {
        run(entries, tx, envs, command_line, stdin, as_binary);
    }
}

fn run(entries: Entries, tx: Option<Sender<Operation>>, envs: Option<Envs>, command_line: &[String], stdin: Option<String>, as_binary: bool) {
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
    match process_stdout(tx, child, stdin, as_binary) {
        Ok(_) => puts_event!("shell/close"),
        Err(err) => puts_error!(err, "at" => "shell", "for" => join(command_line, ',')),
    }

    finalizer.finalize();
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

fn get_envs() -> Envs {
    let mut result = HashMap::new();

    for (key, value) in env::vars_os() {
        if let (Ok(key), Ok(value)) = (key.into_string(), value.into_string()) {
            result.insert(key, value);
        }
    }

    result
}
