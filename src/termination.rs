
use std::fs::remove_file;
use std::process::exit;
use std::sync::{Arc, Mutex};

use ctrlc;
use libc;



#[derive(Clone, Debug)]
pub enum Process {
    Kill(u32),
    Delete(String)
}



lazy_static! {
    static ref PROCESS_LIST: Arc<Mutex<Vec<Process>>> = {
        ctrlc::set_handler(move || execute());
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
