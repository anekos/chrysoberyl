
use std::io::Read;
use std::os::unix::net::UnixListener;
use std::path::Path;
use std::sync::mpsc::Sender;
use std::thread::spawn;

use crate::chainer;
use crate::controller::process;
use crate::errors::AppResultU;
use crate::operation::Operation;



pub fn register<T: AsRef<Path>>(tx: Sender<Operation>, path: T) -> AppResultU {
    let listener = UnixListener::bind(path.as_ref())?;

    chainer::register(chainer::Target::File(path.as_ref().to_path_buf()));

    spawn(move || {
        with_error!(at = "controller/unix_socket", {
            for stream in listener.incoming() {
                let mut stream = stream?;
                let mut buffer = o!("");
                stream.read_to_string(&mut buffer)?;
                for line in buffer.lines() {
                    process(&tx, line, "controller/unix_socket");
                }
            }
        });
    });

    Ok(())
}

pub fn register_as_binary<T: AsRef<Path>>(tx: Sender<Operation>, path: T) -> AppResultU {
    let listener = UnixListener::bind(path.as_ref())?;

    chainer::register(chainer::Target::File(path.as_ref().to_path_buf()));

    spawn(move || {
        with_error!(at = "controller/unix_socket", {
            for stream in listener.incoming() {
                let mut stream = stream?;
                let mut buffer = vec![];
                stream.read_to_end(&mut buffer)?;
                tx.send(Operation::PushMemory(buffer, None, false))?;
            }
        });
    });

    Ok(())
}
