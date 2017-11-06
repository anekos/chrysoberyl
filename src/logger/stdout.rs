
use std::fmt;
use std::io::{self, Write};
use std::sync::mpsc::{Sender, channel};
use std::thread::spawn;

use errors::ChryError;
use logger;
use option::OptionValue;

use option::common;



pub struct StdOut {
    current: Option<logger::Handle>,
    sender: Sender<String>,
}


impl StdOut {
    pub fn new() -> Self {
        StdOut {
            current: None,
            sender: run_stdout_output(),
        }
    }

    pub fn unregister(&mut self) {
        if let Some(handle) = self.current {
            logger::unregister(handle);
        }
        self.current = None;
    }

    pub fn register(&mut self) {
        self.unregister();
        let tx = self.sender.clone();
        self.current = Some(logger::register(tx));
    }
}

impl OptionValue for StdOut {
    fn is_enabled(&self) -> Result<bool, ChryError> {
        Ok(self.current.is_some())
    }

    fn enable(&mut self) -> Result<(), ChryError> {
        self.register();
        Ok(())
    }

    fn disable(&mut self) -> Result<(), ChryError> {
        self.unregister();
        Ok(())
    }

    fn cycle(&mut self, _: bool) -> Result<(), ChryError> {
        self.toggle()
    }

    fn set(&mut self, path: &str) -> Result<(), ChryError> {
        common::parse_bool(path).map(|value| {
            if value {
                self.register();
            } else {
                self.unregister();
            }
        })
    }

    fn unset(&mut self) -> Result<(), ChryError> {
        self.unregister();
        Ok(())
    }
}

impl fmt::Display for StdOut {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", common::bool_to_str(self.current.is_some()))
    }
}

fn run_stdout_output() -> Sender<String> {
    let (tx, rx) = channel();

    spawn(move || {
        let stdout = io::stdout();
        while let Ok(s) = rx.recv() {
            let mut stdout = stdout.lock();
            let _ = stdout.write_fmt(format_args!("{}\n", s));
        }
    });

    tx
}
