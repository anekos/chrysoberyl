
use std::borrow::Cow;
use std::sync::mpsc::{Sender, channel};
use std::sync::{Arc, Mutex};
use std::thread::spawn;

use shell_escape::escape;



lazy_static! {
    static ref OUTPUT_INSTANCE: Arc<Mutex<Output>> = {
        let mut out = Output { txs: vec![] };
        out.register(run_stdout_output());
        Arc::new(Mutex::new(out))
    };
}


#[derive(Clone)]
pub struct Output {
    txs: Vec<Sender<String>>
}


impl Output {
    pub fn puts(&self, data: &[(String, String)]) {
        self.puts_each_channel(generate_text(data));
    }

    fn puts_each_channel(&self, text: String) {
        for tx in &self.txs {
            tx.send(text.clone()).unwrap();
        }
    }

    fn register(&mut self, tx: Sender<String>) {
        self.txs.push(tx);
    }
}



pub fn puts(data: &[(String, String)]) {
    let out = (*OUTPUT_INSTANCE).lock().unwrap();
    out.puts(data);
}


macro_rules! puts {
    ( $($name:expr => $value:expr),* ) => {
        {
            use output;
            output::puts(&[
                $( ($name.to_owned(), $value.to_owned()) ),*
            ])
        }
    }
}

macro_rules! puts_event {
    ( $event:expr  $(,$name:expr => $value:expr)* ) => {
        puts!("event" => $event $(, $name => $value)*)
    }
}


macro_rules! puts_error {
    ( $($name:expr => $value:expr),* ) => {
        puts!("event" => "error" $(, $name => $value)*)
    }
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
