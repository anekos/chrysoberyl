
use gtk::{Inhibit, main_quit};
use gdk::EventKey;
use std::sync::mpsc::Sender;

use operation::Operation;
use key;




pub fn on_key_press(tx: Sender<Operation>, key: &EventKey) -> Inhibit {
    use operation::Operation::*;
    use options::AppOptionName as opt;

    let keyval = key.as_ref().keyval;
    if let Some(operation) = match key::to_name(keyval).as_str() {
        "e" => Some(Expand),                // e
        "f" | "h" => Some(First),           // f | h
        "j" => Some(Next),                  // j
        "k" => Some(Previous),              // k
        "l" => Some(Last),                  // l
        "q" => Some(Exit),                  // q
        "r" => Some(Refresh),               // r
        "i" => Some(Toggle(opt::ShowText)), // i
        "z" => Some(Shuffle),               // z
        _ => if 48 <= keyval && keyval <= 57 {
            Some(Count((keyval - 48) as u8))
        } else {
            Some(Key(keyval))
        }
    } {
        tx.send(operation).unwrap();
    }

    Inhibit(false)
}

pub fn on_configure(tx: Sender<Operation>) -> bool {
    tx.send(Operation::Refresh).unwrap();
    false
}

pub fn on_delete() -> Inhibit {
    main_quit();
    Inhibit(false)
}
