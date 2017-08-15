
use std::fs::File;
use std::io:: Read;
use std::path::Path;
use std::sync::mpsc::Sender;

use config::DEFAULT_CONFIG;
use operation::Operation;
use utils::path_to_string;



pub fn load(tx: &Sender<Operation>, source: &str) {
    puts_event!("script/open");
    load_from_str(tx, source);
    puts_event!("script/close");
}

pub fn load_from_file(tx: &Sender<Operation>, file: &Path) {
    puts_event!("script/open", "file" => path_to_string(&file));
    let mut source = o!("");
    match File::open(file).and_then(|mut file| file.read_to_string(&mut source)) {
        Ok(_) => load_from_str(tx, &source),
        Err(err) => puts_error!("at" => o!("on_load"), "reason" => s!(err)),
    }
    puts_event!("script/close", "file" => path_to_string(&file));
}

fn load_from_str(tx: &Sender<Operation>, source: &str) {
    let lines: Vec<&str> = source.lines().collect();

    for line in lines {
        match Operation::parse(line) {
            Ok(op) =>
                process(tx, op),
            Err(err) =>
                puts_error!("at" => "script/line", "reason" => s!(err), "for" => o!(line)),
        }
    }
}

fn process(tx: &Sender<Operation>, operation: Operation) {
    match operation {
        Operation::Load(ref file, search_path) => {
            let path = if search_path { file.search_path() } else { file.expand() };
            load_from_file(tx, &path);
        },
        Operation::LoadDefault =>
            load(tx, DEFAULT_CONFIG),
        op =>
            tx.send(op).unwrap(),
    }
}
