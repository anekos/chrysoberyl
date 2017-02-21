
use gtk::{Inhibit, main_quit};
use gdk::EventKey;
use std::sync::mpsc::Sender;

use operation::Operation;




pub fn on_key_press(tx: Sender<Operation>, key: &EventKey) -> Inhibit {
    use operation::Operation::*;
    use options::AppOptionName as opt;

    if let Some(operation) = match key.as_ref().keyval {
        101 => Some(Expand),
        104 | 102 => Some(First),
        106 => Some(Next),
        107 => Some(Previous),
        108 => Some(Last),
        113 => Some(Exit),
        114 => Some(Refresh),
        105 => Some(Toggle(opt::ShowText)),
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
