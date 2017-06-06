
use std::fs::File;
use std::io:: Read;
use std::path::Path;
use std::sync::mpsc::Sender;

use config::DEFAULT_CONFIG;
use operation::Operation;



pub fn load(tx: &Sender<Operation>, source: &str) {
    let lines: Vec<&str> = source.lines().collect();

    puts_event!("input/script/open");
    for line in lines {
        match Operation::parse(line) {
            Ok(Operation::Load(ref file)) =>
                load_from_file(tx, file),
            Ok(Operation::LoadDefault) =>
                load(tx, DEFAULT_CONFIG),
            Ok(op) =>
                tx.send(op).unwrap(),
            Err(err) =>
                puts_error!("at" => "input/script", "reason" => s!(err), "for" => o!(line)),
        }
    }
    puts_event!("input/script/close");
}

pub fn load_from_file(tx: &Sender<Operation>, file: &Path) {
    let mut source = o!("");
    match File::open(file).and_then(|mut file| file.read_to_string(&mut source)) {
        Ok(_) => load(tx, &source),
        Err(err) => puts_error!("at" => o!("on_load"), "reason" => s!(err)),
    }
}
