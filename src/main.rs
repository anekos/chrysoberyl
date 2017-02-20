
extern crate argparse;
extern crate gdk;
extern crate gdk_pixbuf;
extern crate gtk;
extern crate hyper;
extern crate hyper_native_tls;
extern crate url;
extern crate cairo;
extern crate libc;
#[macro_use] extern crate closet;

mod app;
mod controller;
mod entry;
mod events;
mod http_cache;
mod index_pointer;
mod log;
mod operation;
mod options;

use gtk::prelude::*;
use gtk::{Image, Window};
use argparse::{ArgumentParser, List, Collect};
use std::thread::{sleep};
use std::time::Duration;

use operation::Operation;



fn main() {
    use Operation::*;

    unsafe {
        log::puts1("PID", libc::getpid());
    }

    let mut files: Vec<String> = vec![];
    let mut inputs: Vec<String> = vec![];

    {
        let mut ap = ArgumentParser::new();

        ap.set_description("Controllable Image Viewer");

        ap.refer(&mut inputs).add_option(&["--input", "-i"], Collect, "Controller files");
        ap.refer(&mut files).add_argument("images", List, "Image files or URLs");

        ap.parse_args_or_exit();
    }

    let (window, image) = setup();

    let (mut app, rx) = app::App::new(files, window.clone(), image.clone());
    let tx = app.tx.clone();

    window.connect_key_press_event(clone_army!([tx] move |_, key| events::on_key_press(tx.clone(), key)));
    window.connect_configure_event(clone_army!([tx] move |_, _| events::on_configure(tx.clone())));

    for input in inputs {
        controller::run_file_controller(tx.clone(), input);
    }
    controller::run_stdin_controller(tx.clone());

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


fn setup() -> (Window, Image) {
    gtk::init().unwrap();

    let window = gtk::Window::new(gtk::WindowType::Toplevel);

    window.set_title("Chrysoberyl");
    window.set_border_width(0);
    window.set_position(gtk::WindowPosition::Center);

    window.connect_delete_event(|_, _| events::on_delete());

    let image = Image::new_from_pixbuf(None);
    window.add(&image);

    (window, image)
}
