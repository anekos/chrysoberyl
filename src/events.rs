
use gtk::{Inhibit, main_quit};
use gdk::EventKey;
use std::sync::mpsc::Sender;

use operation::Operation;




pub fn on_key_press(tx: Sender<Operation>, key: &EventKey) -> Inhibit {
    use operation::Operation::*;
    use options::AppOptionName as opt;

    if let Some(operation) = match key.as_ref().keyval {
        101 => Some(Expand),                // e
        104 | 102 => Some(First),           // f | h
        106 => Some(Next),                  // j
        107 => Some(Previous),              // k
        108 => Some(Last),                  // l
        113 => Some(Exit),                  // q
        114 => Some(Refresh),               // r
        105 => Some(Toggle(opt::ShowText)), // i
        122 => Some(Shuffle),               // z
        key => if 48 <= key && key <= 57 {
            Some(Count((key - 48) as u8))
        } else {
            Some(Key(key))
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
