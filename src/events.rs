
use std::cell::Cell;
use std::sync::Arc;
use std::sync::mpsc::Sender;
use std::time::Duration;

use gdk::{EventButton, EventKey, EventConfigure};
use gtk::prelude::*;
use gtk::Inhibit;

use lazy_sender::LazySender;
use gui::Gui;
use mapping::Input;
use operation::Operation;
use size::Region;
use utils::feq;


type ArcPressedAt = Arc<Cell<(f64, f64)>>;
type ArcLastWindowSize = Arc<Cell<(u32, u32)>>;


pub fn register(gui: &Gui, tx: &Sender<Operation>) {
    let pressed_at = Arc::new(Cell::new((0.0, 0.0)));
    let last_window_size = Arc::new(Cell::new((0u32, 0u32)));
    let sender = LazySender::new(tx.clone(), Duration::from_millis(200));

    gui.window.connect_key_press_event(clone_army!([tx] move |_, key| on_key_press(&tx, key)));
    gui.window.connect_configure_event(clone_army!([last_window_size, sender] move |_, configure| on_configure(sender.clone(), configure, last_window_size.clone())));
    gui.window.connect_delete_event(clone_army!([tx] move |_, _| on_delete(&tx)));
    gui.window.connect_button_press_event(clone_army!([pressed_at] move |_, button| on_button_press(button, pressed_at.clone())));
    gui.window.connect_button_release_event(clone_army!([tx] move |_, button| on_button_release(&tx, button, pressed_at.clone())));
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

fn on_button_press(button: &EventButton, pressed_at: ArcPressedAt) -> Inhibit {
    let (x, y) = button.get_position();
    (*pressed_at).set((x, y));
    Inhibit(true)
}

fn on_button_release(tx: &Sender<Operation>, button: &EventButton, pressed_at: ArcPressedAt) -> Inhibit {
    let (x, y) = button.get_position();
    let (px, py) = (*pressed_at).get();
    if feq(x, px, 10.0) && feq(y, py, 10.0) {
        tx.send(
            Operation::Input(
                Input::mouse_button(x as i32, y as i32, button.get_button()))).unwrap();
    } else {
        tx.send( Operation::Clip(Region::new(px, py, x, y))).unwrap();
    }
    Inhibit(true)
}

fn on_configure(mut sender: LazySender, configure: &EventConfigure, last_window_size: ArcLastWindowSize) -> bool {
    let (w, h) = configure.get_size();
    let (lw, lh) = last_window_size.get();

    if lw != w || lh != h {
        sender.request(Operation::WindowResized);
        (*last_window_size).set((w, h));
    }
    false
}

fn on_delete(tx: &Sender<Operation>) -> Inhibit {
    tx.send(Operation::Quit).unwrap();
    Inhibit(false)
}
