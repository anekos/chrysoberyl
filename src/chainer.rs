
/**
 * Chain file and process to chrysoberyl.
 * At exiting, Chrysoberyl terminate the process or delete the file.
 */

use std::fs::remove_file;
use std::path::PathBuf;
use std::process::exit;
use std::sync::{Arc, Mutex};

use ctrlc;
use lazy_static::lazy_static;
use libc;
use log::debug;



#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Target {
    Process(u32),
    File(PathBuf)
}



lazy_static! {
    static ref PROCESS_LIST: Arc<Mutex<Vec<Target>>> = {
        ctrlc::set_handler(execute).unwrap();
        Arc::new(Mutex::new(vec![]))
    };
}

pub fn register(target: Target) {
    let mut list = (*PROCESS_LIST).lock().unwrap();
    list.push(target);
    debug!("register: {:?}", *list);
}

pub fn unregister(target: &Target) {
    let mut list = (*PROCESS_LIST).lock().unwrap();

    if let Some(pos) = list.iter().position(|x| *x == *target) {
        list.remove(pos);
        debug!("unregister: {:?}", *list);
    }
}

pub fn execute() {
    use self::Target::*;

    let list = (*PROCESS_LIST).lock().unwrap();

    for target in list.iter() {
        debug!("execute: {:?}", target);
        match *target {
            Process(pid) => unsafe {
                libc::kill(-(pid as i32), libc::SIGTERM);
            },
            File(ref path) => {
                let _ = remove_file(path);
            }
        }
    }

    exit(0);
}
