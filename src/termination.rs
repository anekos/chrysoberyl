
use std::fs::remove_file;
use std::path::PathBuf;
use std::process::exit;
use std::sync::{Arc, Mutex};

use ctrlc;
use libc;



#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Process {
    Kill(u32),
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

pub fn unregister(process: &Process) {
    let mut list = (*PROCESS_LIST).lock().unwrap();

    if let Some(pos) = list.iter().position(|x| *x == *process) {
        list.remove(pos);
        debug!("unregister: {:?}", *list);
    }
}

pub fn execute() {
    use self::Process::*;

    let list = (*PROCESS_LIST).lock().unwrap();

    for process in list.iter() {
        debug!("execute: {:?}", process);
        match *process {
            Kill(pid) => unsafe {
                libc::kill(-(pid as i32), libc::SIGTERM);
            },
            Delete(ref path) => {
                let _ = remove_file(path);
            }
        }
    }

    exit(0);
}
