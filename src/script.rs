
use std::fs::File;
use std::io:: Read;
use std::path::Path;
use std::sync::mpsc::Sender;

use app_path::PathList;
use config::DEFAULT_CONFIG;
use operation::Operation;
use utils::path_to_string;



pub fn load(tx: &Sender<Operation>, source: &str, path_list: &PathList) {
    puts_event!("script/open");
    load_from_str(tx, source, path_list);
    puts_event!("script/close");
}

pub fn load_from_file(tx: &Sender<Operation>, file: &Path, path_list: &PathList) {
    puts_event!("script/open", "file" => path_to_string(&file));
    let mut source = o!("");
    match File::open(file).and_then(|mut file| file.read_to_string(&mut source)) {
        Ok(_) => load_from_str(tx, &source, path_list),
        Err(err) => puts_error!(s!(err), "at" => o!("on_load")),
    }
    puts_event!("script/close", "file" => path_to_string(&file));
}

fn load_from_str(tx: &Sender<Operation>, source: &str, path_list: &PathList) {
    let lines: Vec<&str> = source.lines().collect();

    for line in lines {
        match Operation::parse(line) {
            Ok(op) =>
                process(tx, op, path_list),
            Err(err) =>
                puts_error!(s!(err), "at" => "script/line", "for" => o!(line)),
        }
    }
}

fn process(tx: &Sender<Operation>, operation: Operation, path_list: &PathList) {
    match operation {
        Operation::Load(ref file, search_path) => {
            let path = if search_path { file.search_path(path_list) } else { file.expand() };
            load_from_file(tx, &path, path_list);
        },
        Operation::LoadDefault =>
            load(tx, DEFAULT_CONFIG, path_list),
        op =>
            tx.send(op).unwrap(),
    }
}
