
use std::error::Error;
use std::fs::OpenOptions;
use std::io::{BufRead, BufReader, Read, Write};
use std::path::{PathBuf, Path};



pub fn write_line(line: &str, file: &Option<PathBuf>) -> Result<(), Box<Error>> {
    if_let_some!(file = file.as_ref(), Ok(()));
    let mut file = OpenOptions::new().read(false).write(true).append(true).create(true).open(file)?;
    writeln!(file, "{}", line)?;
    Ok(())
}

pub fn read_lines<T: AsRef<Path>>(file: T) -> Result<Vec<String>, Box<Error>> {
    let file = OpenOptions::new().read(true).write(false).append(false).create(false).open(file.as_ref())?;
    let file = BufReader::new(file);
    let mut result = vec![];
    for line in file.lines() {
        result.push(line?);
    }
    Ok(result)
}

pub fn read_string<T: AsRef<Path>>(file: T) -> Result<String, Box<Error>> {
    let file = OpenOptions::new().read(true).write(false).append(false).create(false).open(file.as_ref())?;
    let mut file = BufReader::new(file);
    let mut result = o!("");
    file.read_to_string(&mut result)?;
    Ok(result)
}
