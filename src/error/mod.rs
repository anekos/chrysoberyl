
use std::sync::mpsc::{Sender, channel};
use std::sync::Arc;
use std::sync::Mutex;
use std::thread::spawn;

use operation::Operation;



pub struct Error(pub String);

enum ErrorChannel {
    Push(Error),
    Register(Sender<Operation>),
}


lazy_static! {
    static ref ERROR_CHANNEL: Arc<Mutex<Sender<ErrorChannel>>> = {
        Arc::new(Mutex::new(main()))
    };
}


macro_rules! puts_error {
    ( $message:expr $(,$name:expr => $value:expr)* ) => {
        {
            use error;
            error::push(error::Error(o!($message)));
            puts!("event" => "error", "message" => $message $(, $name => $value)*)
        }
    }
}

pub fn register(op_tx: Sender<Operation>) {
    let tx = (*ERROR_CHANNEL).lock().unwrap();
    tx.send(ErrorChannel::Register(op_tx)).unwrap()
}

pub fn push(error: Error) {
    let tx = (*ERROR_CHANNEL).lock().unwrap();
    tx.send(ErrorChannel::Push(error)).unwrap()
}


fn main() -> Sender<ErrorChannel> {
    use self::ErrorChannel::*;

    let (tx, rx) = channel();
    let mut targets: Vec<Sender<Operation>> = vec![];

    spawn(move || {
        while let Ok(ec) = rx.recv() {
            match ec {
                Push(error) => {
                    for target in &targets {
                        target.send(Operation::Error(error.0.clone())).unwrap()
                    }
                },
                Register(tx) => targets.push(tx),
            }
        }
    });

    tx
}
