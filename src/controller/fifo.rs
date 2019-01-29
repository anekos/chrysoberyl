
use std::ffi::CString;
use std::fs::File;
use std::io;
use std::path::Path;
use std::sync::mpsc::Sender;
use std::thread::spawn;

use libc;

use crate::chainer;
use crate::errors::ErrorKind;
use crate::operation::Operation;

use crate::controller::process_lines;



pub fn register<T: AsRef<Path>>(tx: Sender<Operation>, path: &T) {
    let res = unsafe {
        let mode = 0o600;
        let cstr = CString::new(path.as_ref().to_str().unwrap().as_bytes());
        libc::mkfifo(cstr.unwrap().as_ptr(), mode as libc::mode_t)
    };

    if res != 0 {
        puts_error!(
            chry_error!("Could not mkfifo {:?} {}", path.as_ref(), io::Error::last_os_error().raw_os_error().unwrap()),
            "at" => "fragile_controller",
            "for" => d!(path.as_ref()));
        return
    }

    chainer::register(chainer::Target::File(path.as_ref().to_path_buf()));

    let path = path.as_ref().to_path_buf();
    spawn(move || {
        while let Ok(file) = File::open(&path) {
            puts_event!("input/fragile/open");
            process_lines(&tx, file, "input/fragile");
            puts_event!("input/fragile/close");
        }
        puts_error!(ErrorKind::Fixed("Could not open file"), "at" => "fragile_controller", "for" => p!(&path));
    });
}
