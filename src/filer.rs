
use std::convert::AsRef;
use std::ffi::OsStr;
use std::fs::{self, File, create_dir_all};
use std::io::Write;
use std::path::{Path, PathBuf};

use crate::errors::*;
use crate::size::Size;
use crate::util::common::mangle;


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
    destination_file: Option<String>,
    if_exist: IfExist,
    pub size: Option<Size>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum FileOperationAction {
    Copy,
    Move,
}



impl FileOperation {
    pub fn new_move(destination_directory: PathBuf, destination_file: Option<String>, if_exist: IfExist, size: Option<Size>) -> FileOperation {
        FileOperation::new(FileOperationAction::Move, destination_directory, destination_file, if_exist, size)
    }

    pub fn new_copy(destination_directory: PathBuf, destination_file: Option<String>, if_exist: IfExist, size: Option<Size>) -> FileOperation {
        FileOperation::new(FileOperationAction::Copy, destination_directory, destination_file, if_exist, size)
    }

    fn new(action: FileOperationAction, destination_directory: PathBuf, destination_file: Option<String>, if_exist: IfExist, size: Option<Size>) -> FileOperation {
        FileOperation { action, destination_directory, destination_file, if_exist, size, }
    }

    pub fn execute(&self, source: &PathBuf) -> AppResultU {
        use self::FileOperationAction::*;

        match self.action {
            Copy => {
                let dest = destination_path(source, &self.destination_directory, &self.destination_file, self.if_exist)?;
                fs::copy(source, dest).map(mangle)?;
                Ok(())
            }
            Move => {
                let dest = destination_path(source, &self.destination_directory, &self.destination_file, self.if_exist)?;
                fs::rename(source, dest).map(mangle)?;
                Ok(())
            }
        }
    }

    pub fn execute_with_buffer(&self, source: &[u8], source_name: &PathBuf) -> AppResultU {
        let dest = destination_path(source_name, &self.destination_directory, &self.destination_file, self.if_exist)?;
        let mut file = File::create(dest)?;
        file.write_all(source)?;
        Ok(())
    }
}


fn destination_path(source: &PathBuf, destination_directory: &PathBuf, file_name: &Option<String>, if_exist: IfExist) -> AppResult<PathBuf> {
    use self::IfExist::*;

    let file_name = file_name.as_ref().map(AsRef::as_ref).unwrap_or_else(|| source.file_name().unwrap());
    let mut path = destination_directory.clone();

    if !path.exists() {
        create_dir_all(&path)?;
    }

    path.push(file_name);

    match if_exist {
        Fail if path.exists() => Err(AppError::File("Already exists", d!(path))),
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
