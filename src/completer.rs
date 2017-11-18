
extern crate gtk;

use gdk::EventKey;
use glib::Type;
use gtk::prelude::*;
use gtk::{EntryCompletion, ListStore, Value};


const WINDOW_CLASS: &'static str = concat!(env!("CARGO_PKG_NAME"), "-completer");
static OPERATIONS: &'static str = include_str!("static/operations.txt");
static OPTIONS: &'static str = include_str!("static/options.txt");



pub fn main() {
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

    let completion = tap!(completion = EntryCompletion::new(), {
        let store = ListStore::new(&[Type::String]);
        completion.set_model(&store);
        completion.set_text_column(0);
        completion.set_inline_completion(true);
        completion.set_inline_selection(true);
        completion.set_popup_single_match(false);
        completion.set_popup_completion(true);
        update_completion(&store);
    });

    let entry = gtk::Entry::new();
    entry.set_completion(&completion);

    window.add(&entry);

    window.connect_delete_event(|_, _| on_delete());
    entry.connect_key_press_event(clone_army!([entry] move |_, key| on_key_press(&entry, key)));
    window
}


fn append_completion_entry(store: &ListStore, entry: &str) {
    let iter = store.append();
    let value = Value::from(entry);
    store.set_value(&iter, 0, &value);
}

fn update_completion(store: &ListStore) {
    for it in OPERATIONS.split('\n').filter(|it| 0 < it.len()) {
        match &it[1..] {
            "set" | "set-by-count" | "increase" | "decrease" | "unset" | "enable" | "disable" | "cycle" | "toggle" => {
                for option in OPTIONS.split('\n') {
                    append_completion_entry(store, &format!("{} {}", it, option));
                }
            },
            _ =>
                append_completion_entry(store, it),
        }
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
        key::Tab => {
            if let Some(text) = entry.get_text() {
                let mut p = text.len() as i32;
                entry.insert_text(" ", &mut p);
                entry.set_position(p - 1);
            } else {
                println!("empty");
            }
            return Inhibit(false);
        },
        _ => return Inhibit(false),
    }

    gtk::main_quit();
    Inhibit(false)
}

fn on_delete() -> Inhibit {
    gtk::main_quit();
    Inhibit(false)
}
