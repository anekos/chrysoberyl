
extern crate argparse;
extern crate ctrlc;
extern crate gdk;
extern crate gdk_pixbuf;
extern crate gtk;
extern crate hyper;
extern crate hyper_native_tls;
extern crate immeta;
extern crate url;
extern crate cairo;
extern crate libc;
extern crate rand;
#[macro_use] extern crate closet;
extern crate env_logger;
#[macro_use] extern crate log;

#[macro_use] mod utils;
#[macro_use] mod output;
mod app;
mod controller;
mod entry;
mod events;
mod fragile_input;
mod http_cache;
mod index_pointer;
mod operation;
mod options;
mod path;
mod key;

use gtk::prelude::*;
use gtk::{Image, Window};
use argparse::{ArgumentParser, List, Collect, Store, StoreTrue, StoreOption};
use std::thread::{sleep};
use std::time::Duration;

use entry::EntryContainerOptions;
use key::KeyData;



fn main() {
    env_logger::init().unwrap();

    unsafe {
        puts!("PID", libc::getpid());
    }

    let mut files: Vec<String> = vec![];
    let mut inputs: Vec<String> = vec![];
    let mut fragiles: Vec<String> = vec![];
    let mut expand: bool = false;
    let mut expand_recursive: bool = false;
    let mut min_width: Option<u32> = None;
    let mut min_height: Option<u32> = None;
    let mut shuffle: bool = false;
    let mut max_http_threads: u8 = 3;

    {
        let mut ap = ArgumentParser::new();

        ap.set_description("Controllable Image Viewer");

        ap.refer(&mut inputs).add_option(&["--input", "-i"], Collect, "Controller files");
        ap.refer(&mut fragiles).add_option(&["--fragile-input", "-f"], Collect, "Chrysoberyl makes the `fifo` file whth given path");
        ap.refer(&mut expand).add_option(&["--expand", "-e"], StoreTrue, "`Expand` first file");
        ap.refer(&mut expand_recursive).add_option(&["--expand-recursive", "-E"], StoreTrue, "`Expand` first file");
        ap.refer(&mut shuffle).add_option(&["--shuffle", "-z"], StoreTrue, "Shuffle file list");
        ap.refer(&mut min_width).add_option(&["--min-width", "-W"], StoreOption, "Minimum width");
        ap.refer(&mut min_height).add_option(&["--min-height", "-H"], StoreOption, "Minimum height");
        ap.refer(&mut max_http_threads).add_option(&["--max-http-threads", "-t"], Store, "Maximum number of HTTP Threads");
        ap.refer(&mut files).add_argument("images", List, "Image files or URLs");

        ap.parse_args_or_exit();
    }

    let (window, image) = setup();

    let (mut app, rx) = app::App::new(
        EntryContainerOptions { min_width: min_width, min_height: min_height },
        max_http_threads,
        expand,
        expand_recursive,
        shuffle,
        files,
        fragiles.clone(),
        window.clone(),
        image.clone()
        );
    let tx = app.tx.clone();

    window.connect_key_press_event(clone_army!([tx] move |_, key| events::on_key_press(tx.clone(), KeyData::new(key))));
    window.connect_configure_event(clone_army!([tx] move |_, _| events::on_configure(tx.clone())));
    window.connect_button_press_event(clone_army!([tx] move |_, button| events::on_button_press(tx.clone(), button)));

    for path in inputs {
        controller::run_file_controller(tx.clone(), path);
    }
    for path in fragiles {
        controller::run_file_controller(tx.clone(), path);
    }
    controller::run_stdin_controller(tx.clone());

    window.show_all();

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
