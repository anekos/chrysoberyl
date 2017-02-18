
extern crate gdk;
extern crate gdk_pixbuf;
extern crate gtk;
extern crate hyper;
extern crate mktemp;

mod http_cache;
mod index_pointer;
mod app;

use gtk::prelude::*;
use gtk::{Image, Window};
use std::env::args;
use std::sync::mpsc::Sender;
use std::thread::{sleep, spawn};
use std::time::Duration;

use app::Operation;



fn main() {
    use Operation::*;

    let (window, image) = setup();

    let files: Vec<String> = args().skip(1).collect();

    let (mut app, rx) = app::App::new(files, window.clone(), image.clone());
    let tx = app.tx.clone();

    {
        let tx = tx.clone();
        window.connect_key_press_event(move |_, key| on_key_press(tx.clone(), key));
    }

    {
        let tx = tx.clone();
        window.connect_configure_event(move |_, _| on_configure(tx.clone()));
    }

    {
        let tx = tx.clone();
        stdin_reader(tx);
    }

    window.show_all();

    tx.send(First).unwrap();

    loop {
        while gtk::events_pending() {
            gtk::main_iteration();
        }

        for op in rx.try_iter() {
            app.operate(op);
        }
        sleep(Duration::from_millis(10));
    }
}


fn on_configure(tx: Sender<Operation>) -> bool {
    tx.send(Operation::Refresh).unwrap();
    false
}


fn on_key_press(tx: Sender<Operation>, key: &gdk::EventKey) -> gtk::Inhibit {
    use Operation::*;

    if let Some(operation) = match key.as_ref().keyval {
        104 | 102 => Some(First),
        106 => Some(Next),
        107 => Some(Previous),
        108 => Some(Last),
        113 => Some(Exit),
        114 => Some(Refresh),
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


fn stdin_reader(tx: Sender<Operation>) {
    use std::io;
    use std::io::BufRead;

    spawn(move || {
        let stdin = io::stdin();
        for line in stdin.lock().lines() {
            let line = line.unwrap();
            if line.starts_with("http://") {
                tx.send(Operation::PushURL(line)).unwrap();
            } else {
                tx.send(Operation::PushFile(line)).unwrap();
            }
             // tx.send(Operation::Last).unwrap();
        }
    });
}


fn setup() -> (Window, Image) {
    gtk::init().unwrap();

    let window = gtk::Window::new(gtk::WindowType::Toplevel);

    window.set_title("Chrysoberyl");
    window.set_border_width(0);
    window.set_position(gtk::WindowPosition::Center);

    window.connect_delete_event(|_, _| {
        gtk::main_quit();
        Inhibit(false)
    });

    let image = Image::new_from_pixbuf(None);
    window.add(&image);

    (window, image)
}
