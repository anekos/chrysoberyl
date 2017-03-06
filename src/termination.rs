
use std::fs::remove_file;
use std::sync::{Arc, Mutex};
use std::process::exit;
use ctrlc;
use libc;



#[derive(Clone, Debug)]
pub enum Process {
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
            Delete(ref path) => {
                let _ = remove_file(path);
            }
        }
    }

    unsafe {
        let pid = libc::getpid();
        libc::kill(-(pid as i32), libc::SIGTERM);
    }

    exit(0);
}
