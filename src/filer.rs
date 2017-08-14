
use std::ffi::OsStr;
use std::fs::{self, File, create_dir_all};
use std::io::Write;
use std::path::{Path, PathBuf};

use size::Size;
use utils::{s, mangle};


#[derive(Clone, Debug, PartialEq, Copy)]
pub enum IfExist {
    Overwrite,
    NewFileName,
    Fail
}


#[derive(Clone, Debug, PartialEq)]
pub struct FileOperation {
    action: FileOperationAction,
    destination_directory: PathBuf,
    if_exist: IfExist,
    pub size: Option<Size>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum FileOperationAction {
    Copy,
    Move,
}



impl FileOperation {
    pub fn new_move(destination_directory: PathBuf, if_exist: IfExist, size: Option<Size>) -> FileOperation {
        FileOperation::new(FileOperationAction::Move, destination_directory, if_exist, size)
    }

    pub fn new_copy(destination_directory: PathBuf, if_exist: IfExist, size: Option<Size>) -> FileOperation {
        FileOperation::new(FileOperationAction::Copy, destination_directory, if_exist, size)
    }

    fn new(action: FileOperationAction, destination_directory: PathBuf, if_exist: IfExist, size: Option<Size>) -> FileOperation {
        FileOperation { action: action, destination_directory: destination_directory, if_exist: if_exist, size: size }
    }

    pub fn execute(&self, source: &PathBuf) -> Result<(), String> {
        use self::FileOperationAction::*;

        match self.action {
            Copy => {
                destination_path(source, &self.destination_directory, &self.if_exist).and_then(|dest| {
                    fs::copy(source, dest).map_err(|it| s(&it)).map(mangle)
                })
            }
            Move => {
                destination_path(source, &self.destination_directory, &self.if_exist).and_then(|dest| {
                    fs::rename(source, dest).map_err(|it| s(&it)).map(mangle)
                })
            }
        }
    }

    pub fn execute_with_buffer(&self, source: &[u8], source_name: &PathBuf) -> Result<(), String> {
        destination_path(source_name, &self.destination_directory, &self.if_exist).and_then(|dest| {
            File::create(dest).map_err(|it| s(&it)).and_then(|mut file| {
                file.write_all(source).map_err(|it| s(&it))
            })
        })
    }
}


fn destination_path(source: &PathBuf, destination_directory: &PathBuf, if_exist: &IfExist) -> Result<PathBuf, String> {
    use self::IfExist::*;

    let file_name = source.file_name().unwrap();
    let mut path = destination_directory.clone();

    if !path.exists() {
        if let Err(error) = create_dir_all(&path) {
            return Err(s!(error));
        }
    }

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
                path = destination_directory.clone();
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
