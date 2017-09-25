
use std::env;
use std::error::Error;
use std::fs::{File, remove_file};
use std::io::Write;
use std::io::{BufReader, BufRead};
use std::process::Command;
use std::sync::mpsc:: Sender;

use cmdline_parser::Parser;
use mkstemp::TempFile;
use operation::Operation;



pub fn start_edit(tx: &Sender<Operation>, editor_command: Option<String>, default_text: &str) {
    let mut temp_file = {
        let mut temp = env::temp_dir();
        temp.push("chrysoberyl.XXXXXX");
        TempFile::new(temp.to_str().unwrap(), false).unwrap()
    };

    temp_file.write_all(default_text.as_bytes()).unwrap();

    let (command_name, args) = {
        let editor = editor_command.unwrap_or_else(|| {
            env::var("EDITOR").unwrap_or_else(|_| o!("gvim --nofork"))
        });
        let command_line: Vec<String> = Parser::new(&editor).map(|(_, it)| it).collect();
        let (name, args) = command_line.split_first().unwrap();
        (name.clone(), args.to_vec())
    };

    let mut command = Command::new(command_name);
    command.args(&args);
    command.arg(temp_file.path());
    command.status().expect("Failed to execute process");

    {
        let file = BufReader::new(File::open(&temp_file.path()).unwrap());
        for line in file.lines() {
            let line = line.unwrap();
            match Operation::parse_fuzziness(&line) {
                Ok(op) => tx.send(op).unwrap(),
                Err(err) => puts_error!(err, "at" => "editor", "for" => &line)
            }
        }
    }

    remove_file(temp_file.path()).expect(&format!("Could not remove temporary file: {:?}", temp_file.path()));
}
