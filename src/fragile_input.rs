
use std::ffi::CString;
use std::fs::File;
use std::io::Error;
use std::sync::mpsc::Sender;
use std::thread::spawn;
use std::path::Path;

use libc;

use operation::Operation;
use operation_utils::read_operations;
use termination;



pub fn new_fragile_input<T: AsRef<Path>>(tx: Sender<Operation>, path: &T) {
    let res = unsafe {
        let mode = 0o600;
        let cstr = CString::new(path.as_ref().to_str().unwrap().as_bytes());
        libc::mkfifo(cstr.unwrap().as_ptr(), mode as libc::mode_t)
    };

    if res != 0 {
        puts_error!(
            "at" => "fragile_controller",
            "reason" => format!("Could not mkfifo {:?} {}", path.as_ref(), Error::last_os_error().raw_os_error().unwrap()),
            "for" => d!(path.as_ref()));
        return
    }

    termination::register(termination::Process::Delete(path.as_ref().to_path_buf()));

    let path = path.as_ref().to_path_buf();
    spawn(move || {
        while let Ok(file) = File::open(&path) {
            puts_event!("input/fragile/open");
            read_operations("fragile", file, &tx);
            puts_event!("input/fragile/close");
        }
        puts_error!("at" => "fragile_controller", "reason" => "Could not open file", "for" => d!(path));
    });
}
