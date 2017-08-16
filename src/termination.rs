
use std::fs::remove_file;
use std::path::PathBuf;
use std::process::exit;
use std::sync::{Arc, Mutex};

use ctrlc;



#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Process {
    Delete(PathBuf)
}



lazy_static! {
    static ref PROCESS_LIST: Arc<Mutex<Vec<Process>>> = {
        ctrlc::set_handler(execute).unwrap();
        Arc::new(Mutex::new(vec![]))
    };
}

pub fn register(process: Process) {
    let mut list = (*PROCESS_LIST).lock().unwrap();
    list.push(process);
    debug!("register: {:?}", *list);
}


pub fn execute() {
    use self::Process::*;

    let list = (*PROCESS_LIST).lock().unwrap();

    for process in list.iter() {
        debug!("execute: {:?}", process);
        match *process {
            Delete(ref path) => {
                let _ = remove_file(path);
            }
        }
    }

    exit(0);
}
