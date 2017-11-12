
extern crate gtk;
extern crate gdk;
extern crate glib;
#[macro_use] extern crate closet;

#[macro_use]#[allow(unused_macros)] mod macro_utils;

use gdk::EventKey;
use glib::Type;
use gtk::prelude::*;
use gtk::{EntryCompletion, ListStore, Value};


const WINDOW_CLASS: &'static str = concat!(env!("CARGO_PKG_NAME"), "-completer");
static OPERATIONS: &'static str = include_str!("static/operations.txt");



fn main() {
    gtk::init().unwrap();

    let window = make_gui();
    window.show_all();

    gtk::main();
}


fn make_gui() -> gtk::Window {
    let window = gtk::Window::new(gtk::WindowType::Toplevel);
    window.set_position(gtk::WindowPosition::Center);
    window.set_wmclass(WINDOW_CLASS, WINDOW_CLASS);
    window.set_title("chrysoberyl-shell");
    window.resize(500, 1);

    let entry_completion = tap!(entry_completion = EntryCompletion::new(), {
        let store = ListStore::new(&[Type::String]);
        entry_completion.set_model(&store);
        entry_completion.set_text_column(0);
        update_completion(&store);
    });

    let entry = gtk::Entry::new();
    entry.set_completion(&entry_completion);

    window.add(&entry);

    window.connect_delete_event(|_, _| on_delete());
    window.connect_key_press_event(clone_army!([entry] move |_, key| on_key_press(&entry, key)));

    window
}


fn append_completion_entry(store: &ListStore, entry: &str) {
    let iter = store.append();
    let value = Value::from(entry);
    store.set_value(&iter, 0, &value);
}

fn update_completion(store: &ListStore) {
    for it in OPERATIONS.split(" ") {
        append_completion_entry(store, it);
    }
}


fn on_key_press(entry: &gtk::Entry, event_key: &EventKey) -> Inhibit {
    use gdk::enums::key;

    let keyval = event_key.as_ref().keyval;
    match keyval {
        key::Return => if let Some(text) = entry.get_text() {
            println!("{}", text);
        },
        key::Escape => (),
        _ => return Inhibit(false)
    }

    gtk::main_quit();
    Inhibit(false)
}

fn on_delete() -> Inhibit {
    gtk::main_quit();
    Inhibit(false)
}
