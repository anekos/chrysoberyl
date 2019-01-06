
use std::fs::File;
use std::path::Path;
use std::sync::mpsc::Sender;
use std::thread::spawn;

use crate::operation::Operation;

use crate::controller::process_lines;



pub fn register<T: AsRef<Path>>(tx: Sender<Operation>, filepath: T) {
    with_error!(at = "input/file", {
        let file = File::open(filepath.as_ref())?;
        spawn(move || {
            puts_event!("input/file/open");
            process_lines(&tx, file, "file");
            puts_event!("input/file/close");
        });
    });
}
