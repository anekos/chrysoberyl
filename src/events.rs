
use std::sync::mpsc::Sender;

use gdk::EventButton;
use gtk::{Inhibit, main_quit};

use key::KeyData;
use operation::Operation;



pub fn on_key_press(tx: Sender<Operation>, key: KeyData) -> Inhibit {
    use operation::Operation::*;

    let keyval = key.code;
    tx.send({
        if 48 <= keyval && keyval <= 57 {
            Count((keyval - 48) as u8)
        } else {
            Key(key)
        }
    }).unwrap();

    Inhibit(false)
}

pub fn on_button_press(tx: Sender<Operation>, button: &EventButton) -> Inhibit {
    tx.send(Operation::Button(button.get_button())).unwrap();
    Inhibit(true)
}

pub fn on_configure(tx: Sender<Operation>) -> bool {
    tx.send(Operation::Refresh).unwrap();
    false
}

pub fn on_delete() -> Inhibit {
    main_quit();
    Inhibit(false)
}
