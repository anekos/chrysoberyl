
use std::ffi::CString;
use std::fs::File;
use std::io::Error;
use std::sync::mpsc::Sender;
use std::thread::spawn;

use libc;

use operation::Operation;
use operation_utils::read_operations;
use termination;



pub fn new_fragile_input(tx: Sender<Operation>, path: &str) {
    let res = unsafe {
        let mode = 0o600;
        let cstr = CString::new(path.as_bytes());
        libc::mkfifo(cstr.unwrap().as_ptr(), mode as libc::mode_t)
    };

    if res != 0 {
        panic!("Could not mkfifo {:?} {}", path, Error::last_os_error().raw_os_error().unwrap());
    }

    termination::register(termination::Process::Delete(path.to_owned()));

    let path = o!(path);
    spawn(move || {
        while let Ok(file) = File::open(&path) {
            puts_event!("fragile_controller", "state" => "open");
            read_operations(file, tx.clone());
            puts_event!("fragile_controller", "state" => "close");
        }
        puts_error!("at" => "fragile_controller", "reason" => "Could not open file", "for" => path);
    });
}
