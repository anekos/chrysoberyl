
use std::env;
use std::io::Write;
use std::io::{BufReader, BufRead};
use std::process::Command;
use std::sync::mpsc:: Sender;
use std::fs::File;

use cmdline_parser::Parser;
use config::{ConfigSource, config_lines};
use mkstemp::TempFile;
use operation::Operation;



pub fn start_edit(tx: &Sender<Operation>, editor_command: Option<String>, config_sources: Vec<ConfigSource>) {
    let mut temp_file = {
        let mut temp = env::temp_dir();
        temp.push("chrysoberyl.XXXXXX");
        TempFile::new(temp.to_str().unwrap(), true).unwrap()
    };

    {
        for source in config_sources {
            let lines = config_lines(&source);
            for line in lines {
                temp_file.write_all(format!("# {}\n", line).as_bytes()).unwrap();
            }
        }
    }

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

    let file = BufReader::new(File::open(&temp_file.path()).unwrap());
    for line in file.lines() {
        let line = line.unwrap();
        tx.send(Operation::from_str_force(&line)).unwrap();
    }
}
