
use std::borrow::Cow;
use std::collections::HashMap;
use std::sync::mpsc::{Sender, channel};
use std::sync::{Arc, Mutex};
use std::thread::spawn;

use shell_escape::escape;

pub mod file;
#[macro_use] pub mod macros;



lazy_static! {
    pub static ref OUTPUT_INSTANCE: Arc<Mutex<Output>> = {
        let mut out = Output { txs: HashMap::new(), handle: 0 };
        out.register(run_stdout_output());
        Arc::new(Mutex::new(out))
    };
}


type Handle = u64;

#[derive(Clone)]
pub struct Output {
    handle: Handle,
    txs: HashMap<Handle, Sender<String>>,
}


impl Output {
    pub fn puts(&mut self, data: &[(String, String)]) {
        self.puts_each_channel(generate_text(data));
    }

    pub fn register(&mut self, tx: Sender<String>) -> Handle {
        self.handle += 1;
        self.txs.insert(self.handle, tx);
        self.handle
    }

    pub fn unregister(&mut self, handle: Handle) {
        self.txs.remove(&handle);
    }

    fn puts_each_channel(&mut self, text: String) {
        let mut removes: Vec<Handle> = vec![];
        for (handle, tx) in &self.txs {
            if tx.send(text.clone()).is_err() {
                removes.push(*handle);
            }
        }
        for handle in removes {
            self.unregister(handle);
        }
    }
}


pub fn puts(data: &[(String, String)]) {
    let mut out = (*OUTPUT_INSTANCE).lock().unwrap();
    out.puts(data);
}

pub fn register(tx: Sender<String>) -> Handle {
    let mut out = (*OUTPUT_INSTANCE).lock().unwrap();
    out.register(tx)
}

pub fn unregister(handle: Handle) {
    let mut out = (*OUTPUT_INSTANCE).lock().unwrap();
    out.unregister(handle);
}

fn generate_text(data: &[(String, String)]) -> String {
    let mut result = "".to_owned();

    for (index, pair) in data.iter().enumerate() {
        let (ref key, ref value) = *pair;
        let value = Cow::from(value.to_owned());
        if index == 0 {
            result += "O=O";
        }
        result += &format!(" {}={}", key, escape(value));
    }

    result
}

fn run_stdout_output() -> Sender<String> {
    let (tx, rx) = channel();

    spawn(move || {
        while let Ok(s) = rx.recv() {
            println!("{}", s);
        }
    });

    tx
}
