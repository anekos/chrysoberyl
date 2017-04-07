
use std::sync::mpsc::Sender;

use gdk::{EventButton, EventKey};
use gtk::prelude::*;
use gtk::Inhibit;

use gui::Gui;
use mapping::Input;
use operation::Operation;



pub fn register(gui: &Gui, tx: &Sender<Operation>) {
    gui.window.connect_key_press_event(clone_army!([tx] move |_, key| on_key_press(&tx, key)));
    gui.window.connect_configure_event(clone_army!([tx] move |_, _| on_configure(&tx)));
    gui.window.connect_button_press_event(clone_army!([tx] move |_, button| on_button_press(&tx, button)));
    gui.window.connect_delete_event(clone_army!([tx] move |_, _| on_delete(&tx)));
}


fn on_key_press(tx: &Sender<Operation>, key: &EventKey) -> Inhibit {
    let keyval = key.as_ref().keyval;
    tx.send({
        if 48 <= keyval && keyval <= 57 {
            Operation::CountDigit((keyval - 48) as u8)
        } else {
            Operation::Input(Input::key_from_event_key(key))
        }
    }).unwrap();

    Inhibit(false)
}

fn on_button_press(tx: &Sender<Operation>, button: &EventButton) -> Inhibit {
    let (x, y) = button.get_position();
    tx.send(
        Operation::Input(
            Input::mouse_button(x as i32, y as i32, button.get_button()))).unwrap();
    Inhibit(true)
}

fn on_configure(tx: &Sender<Operation>) -> bool {
    tx.send(Operation::Refresh).unwrap();
    false
}

fn on_delete(tx: &Sender<Operation>) -> Inhibit {
    tx.send(Operation::Quit).unwrap();
    Inhibit(false)
}
