
use std::fmt;
use std::fs::{OpenOptions, create_dir_all};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Sender, channel};
use std::thread::spawn;

use crate::errors::{AppResult, AppResultU};
use crate::logger;
use crate::option::OptionValue;
use crate::shellexpand_wrapper as sh;
use crate::util::path::path_to_str;



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

    pub fn register<T: AsRef<Path>>(&mut self, path: &T) -> AppResultU {
        self.unregister();

        register(path).map(|tx| {
            self.current = Some((logger::register(tx), path.as_ref().to_path_buf()));
        })
    }
}

impl OptionValue for File {
    fn set(&mut self, path: &str) -> AppResultU {
        self.register(&sh::expand_to_pathbuf(path))
    }

    fn unset(&mut self) -> AppResultU {
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


pub fn register<T: AsRef<Path>>(path: &T) -> AppResult<Sender<String>> {
    if let Some(parent) = path.as_ref().parent() {
        create_dir_all(parent).unwrap();
    }

    let mut file = OpenOptions::new().read(false).write(true).append(true).create(true).open(path)?;

    let (tx, rx) = channel::<String>();

    spawn(move || {
        while let Ok(s) = rx.recv() {
            file.write_fmt(format_args!("{}\n", s)).unwrap();
        }
    });

    Ok(tx)
}
