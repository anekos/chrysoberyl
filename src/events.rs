
use std::sync::mpsc::Sender;

use gdk::EventButton;
use gtk::prelude::*;
use gtk::{Inhibit, main_quit};

use app;
use key::KeyData;
use operation::Operation;



pub fn register(gui: app::Gui, tx: Sender<Operation>) {
    gui.window.connect_key_press_event(clone_army!([tx] move |_, key| on_key_press(tx.clone(), KeyData::new(key))));
    gui.window.connect_configure_event(clone_army!([tx] move |_, _| on_configure(tx.clone())));
    gui.window.connect_button_press_event(clone_army!([tx] move |_, button| on_button_press(tx.clone(), button)));
    gui.window.connect_delete_event(|_, _| on_delete());

}


fn on_key_press(tx: Sender<Operation>, key: KeyData) -> Inhibit {
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

fn on_button_press(tx: Sender<Operation>, button: &EventButton) -> Inhibit {
    tx.send(Operation::Button(button.get_button())).unwrap();
    Inhibit(true)
}

fn on_configure(tx: Sender<Operation>) -> bool {
    tx.send(Operation::Refresh).unwrap();
    false
}

fn on_delete() -> Inhibit {
    main_quit();
    Inhibit(false)
}
