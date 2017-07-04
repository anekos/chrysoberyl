
use std::fmt;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Sender, channel};
use std::thread::spawn;

use logger;
use option::OptionValue;
use shellexpand_wrapper as sh;
use utils::path_to_str;



pub struct File {
    current: Option<(logger::Handle, PathBuf)>,
}


impl File {
    pub fn new() -> Self {
        File { current: None }
    }

    pub fn unregister(&mut self) {
        if let Some((handle, _)) = self.current {
            logger::unregister(handle);
        }
        self.current = None;
    }

    pub fn register<T: AsRef<Path>>(&mut self, path: &T) -> Result<(), String> {
        self.unregister();

        match register(path) {
            Ok(tx) => {
                self.current = Some((logger::register(tx), path.as_ref().to_path_buf()));
                Ok(())
            }
            Err(error) => Err(error)
        }
    }
}

impl OptionValue for File {
    fn set(&mut self, path: &str) -> Result<(), String> {
        self.register(&sh::expand_to_pathbuf(path))
    }

    fn unset(&mut self) -> Result<(), String> {
        self.unregister();
        Ok(())
    }
}

impl fmt::Display for File {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if let Some((_, ref path)) = self.current {
            write!(f, "{}", path_to_str(path))
        } else {
            write!(f, "")
        }
    }
}


pub fn register<T: AsRef<Path>>(path: &T) -> Result<Sender<String>, String> {
    OpenOptions::new()
        .read(false)
        .write(true)
        .append(true)
        .create(true)
        .open(path).map(|mut file| {
        let (tx, rx) = channel::<String>();

        spawn(move || {
            while let Ok(s) = rx.recv() {
                file.write_fmt(format_args!("{}\n", s)).unwrap();
            }
        });

        tx
    }).map_err(|it| s!(it))
}
