
extern crate argparse;
extern crate ctrlc;
extern crate gdk;
extern crate gdk_pixbuf;
extern crate gtk;
extern crate hyper;
extern crate hyper_native_tls;
extern crate image_utils;
extern crate url;
extern crate cairo;
extern crate libc;
extern crate rand;
#[macro_use] extern crate closet;

#[macro_use]
mod utils;
mod app;
mod controller;
mod entry;
mod events;
mod fragile_input;
mod http_cache;
mod index_pointer;
mod log;
mod operation;
mod options;
mod path;

use gtk::prelude::*;
use gtk::{Image, Window};
use argparse::{ArgumentParser, List, Collect, StoreTrue, Store};
use std::thread::{sleep};
use std::time::Duration;

use operation::Operation;
use entry::EntryContainerOptions;



fn main() {
    use Operation::*;

    unsafe {
        log::puts1("PID", libc::getpid());
    }

    let mut files: Vec<String> = vec![];
    let mut inputs: Vec<String> = vec![];
    let mut fragiles: Vec<String> = vec![];
    let mut expand: bool = false;
    let mut min_width: u32 = 0;
    let mut min_height: u32 = 0;
    let mut shuffle: bool = false;

    {
        let mut ap = ArgumentParser::new();

        ap.set_description("Controllable Image Viewer");

        ap.refer(&mut inputs).add_option(&["--input", "-i"], Collect, "Controller files");
        ap.refer(&mut fragiles).add_option(&["--fragile-input", "-f"], Collect, "Chrysoberyl makes the `fifo` file whth given path");
        ap.refer(&mut expand).add_option(&["--expand", "-e"], StoreTrue, "`Expand` first file");
        ap.refer(&mut shuffle).add_option(&["--shuffle", "-z"], StoreTrue, "Shuffle file list");
        ap.refer(&mut min_width).add_option(&["--min-width", "-W"], Store, "Minimum width");
        ap.refer(&mut min_height).add_option(&["--min-height", "-H"], Store, "Minimum height");
        ap.refer(&mut files).add_argument("images", List, "Image files or URLs");

        ap.parse_args_or_exit();
    }

    let (window, image) = setup();

    let (mut app, rx) = app::App::new(
        EntryContainerOptions { min_width: min_width, min_height: min_height },
        files,
        fragiles.clone(),
        window.clone(),
        image.clone()
        );
    let tx = app.tx.clone();

    window.connect_key_press_event(clone_army!([tx] move |_, key| events::on_key_press(tx.clone(), key)));
    window.connect_configure_event(clone_army!([tx] move |_, _| events::on_configure(tx.clone())));

    for path in inputs {
        controller::run_file_controller(tx.clone(), path);
    }
    for path in fragiles {
        controller::run_file_controller(tx.clone(), path);
    }
    controller::run_stdin_controller(tx.clone());

    window.show_all();

    app.operate(&First);
    if expand { app.operate(&Expand); }
    if shuffle { app.operate_multi(&[Shuffle, First]); }

    loop {
        while gtk::events_pending() {
            gtk::main_iteration();
        }

        for op in rx.try_iter() {
            app.operate(&op);
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
