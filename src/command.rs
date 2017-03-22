
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};

use utils::{s, mangle};


#[derive(Clone, Debug, PartialEq, Copy)]
pub enum IfExist {
    Overwrite,
    NewFileName,
    Fail
}


#[derive(Clone, Debug, PartialEq)]
pub enum Command {
    Copy(PathBuf, IfExist),
    Move(PathBuf, IfExist),
}



impl Command {
    pub fn execute(&self, source: &PathBuf) -> Result<(), String> {
        use self::Command::*;

        match *self {
            Copy(ref destination, ref if_exist) => {
                destination_path(source, destination, if_exist).and_then(|dest| {
                    fs::copy(source, dest).map_err(s).map(mangle)
                })
            }
            Move(ref destination, ref if_exist) => {
                destination_path(source, destination, if_exist).and_then(|dest| {
                    fs::rename(source, dest).map_err(s).map(mangle)
                })
            }
        }
    }
}


fn destination_path(source: &PathBuf, destination: &PathBuf, if_exist: &IfExist) -> Result<PathBuf, String> {
    use self::IfExist::*;

    let file_name = source.file_name().unwrap();
    let mut path = destination.clone();

    path.push(file_name);

    match *if_exist {
        Fail if path.exists() => Err(format!("File already exists: {:?}", path)),
        Fail | Overwrite  => Ok(path),
        NewFileName => {
            let mut suffix = 0;
            let stem = os(Path::new(file_name).file_stem().unwrap());
            let ext = Path::new(file_name).extension().map(os);
            while path.exists() {
                suffix += 1;
                path = destination.clone();
                path.push({
                    if let Some(ext) = ext {
                        format!("{}_{}.{}", stem, suffix, ext)
                    } else {
                        format!("{}_{}", stem, suffix)
                    }
                });
            }
            Ok(path)
        }
    }
}


fn os(x: &OsStr) -> &str {
    x.to_str().unwrap()
}
