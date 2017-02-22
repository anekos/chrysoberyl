
use std::ffi::CString;
use std::io::Error;
use libc;



pub fn new_fragile_input(path: &str) {
    let res = unsafe {
        let mode = 0o600;
        libc::mkfifo(CString::new(path.as_bytes()).unwrap().as_ptr(), mode as libc::mode_t)
    };

    if res != 0 {
        panic!("Could not mkfifo {:?} {}", path, Error::last_os_error().raw_os_error().unwrap());
    }
}
