
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
extern crate cmdline_parser;
extern crate shell_escape;
#[macro_use] extern crate lazy_static;

#[macro_use] mod utils;
#[macro_use] mod output;
mod types;
mod app;
mod controller;
mod entry;
mod events;
mod fragile_input;
mod http_cache;
mod index_pointer;
mod operation;
mod options;
mod key;
mod sorting_buffer;
mod termination;

use gtk::prelude::*;
use gtk::{Image, Window};
use argparse::{ArgumentParser, List, Collect, Store, StoreTrue, StoreOption};
use std::thread::{sleep};
use std::time::Duration;
use std::sync::mpsc::Receiver;

use entry::EntryContainerOptions;
use key::KeyData;
use types::*;
use operation::Operation;
use options::AppOptions;



fn main() {
    env_logger::init().unwrap();

    unsafe {
        puts_event!("info", "name" => "pid", "value" => libc::getpid());
    }

    let (window, image) = setup();

    let (mut app, rx, inputs, fragiles, commands) = parse_arguments(&window, image);

    let tx = app.tx.clone();

    window.connect_key_press_event(clone_army!([tx] move |_, key| events::on_key_press(tx.clone(), KeyData::new(key))));
    window.connect_configure_event(clone_army!([tx] move |_, _| events::on_configure(tx.clone())));
    window.connect_button_press_event(clone_army!([tx] move |_, button| events::on_button_press(tx.clone(), button)));

    for path in inputs {
        controller::run_file_controller(tx.clone(), path);
    }
    for path in fragiles {
        controller::run_fifo_controller(tx.clone(), path);
    }
    for path in commands {
        controller::run_command_controller(tx.clone(), path);
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


fn parse_arguments(window: &Window, image: Image) -> (app::App, Receiver<Operation>, Vec<String>, Vec<String>, Vec<String>) {
    let mut files: Vec<String> = vec![];
    let mut inputs: Vec<String> = vec![];
    let mut fragiles: Vec<String> = vec![];
    let mut commands: Vec<String> = vec![];
    let mut expand: bool = false;
    let mut expand_recursive: bool = false;
    let mut shuffle: bool = false;
    let mut max_http_threads: u8 = 3;
    let mut eco = EntryContainerOptions::new();
    let mut app_options = AppOptions::new();

    {
        let mut width: Option<ImageSize> = None;
        let mut height: Option<ImageSize> = None;

        {

            let mut ap = ArgumentParser::new();

            ap.set_description("Controllable Image Viewer");

            // Controllers
            ap.refer(&mut inputs).add_option(&["--input", "-i"], Collect, "Controller files");
            ap.refer(&mut fragiles).add_option(&["--fragile", "-f"], Collect, "Chrysoberyl makes `fifo` controller file");
            ap.refer(&mut commands).add_option(&["--command", "-c"], Collect, "Controller command");
            // Listing
            ap.refer(&mut expand).add_option(&["--expand", "-e"], StoreTrue, "`Expand` first file");
            ap.refer(&mut expand_recursive).add_option(&["--expand-recursive", "-E"], StoreTrue, "`Expand` first file");
            ap.refer(&mut shuffle).add_option(&["--shuffle", "-z"], StoreTrue, "Shuffle file list");
            // Filter
            ap.refer(&mut eco.min_width).add_option(&["--min-width", "-w"], StoreOption, "Minimum width");
            ap.refer(&mut eco.min_height).add_option(&["--min-height", "-h"], StoreOption, "Minimum height");
            ap.refer(&mut eco.max_width).add_option(&["--max-width", "-W"], StoreOption, "Maximum width");
            ap.refer(&mut eco.max_height).add_option(&["--max-height", "-H"], StoreOption, "Maximum height");
            ap.refer(&mut width).add_option(&["--width"], StoreOption, "Width");
            ap.refer(&mut height).add_option(&["--height"], StoreOption, "Height");
            // Options
            ap.refer(&mut app_options.show_text).add_option(&["--show-info"], StoreTrue, "Show information bar on window bottom");
            // Limitation
            ap.refer(&mut max_http_threads).add_option(&["--max-http-threads", "-t"], Store, "Maximum number of HTTP Threads");
            // Files
            ap.refer(&mut files).add_argument("images", List, "Image files or URLs");

            ap.parse_args_or_exit();
        }

        if let Some(width) = width { eco.min_width = Some(width); eco.max_width = Some(width); }
        if let Some(height) = height { eco.min_height = Some(height); eco.max_height = Some(height); }
    }


    let (app, rx) = app::App::new(eco, max_http_threads, expand, expand_recursive, shuffle, files, fragiles.clone(), window.clone(), image, app_options);

    (app, rx, inputs, fragiles, commands)
}
