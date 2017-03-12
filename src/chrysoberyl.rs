
use std::env::home_dir;
use std::fs::File;
use std::io::{BufReader, BufRead};
use std::sync::mpsc::{Sender, Receiver, channel};
use std::thread::{sleep};
use std::time::{Duration, Instant};

use argparse::{ArgumentParser, List, Collect, Store, StoreTrue, StoreOption};
use encoding::EncodingRef;
use encoding::label::encoding_from_whatwg_label;
use env_logger;
use gtk::prelude::*;
use gtk::{self, Image, Window};
use libc;

use app;
use controller;
use entry::EntryContainerOptions;
use events;
use key::KeyData;
use operation::Operation;
use options::AppOptions;



pub fn main() {
    env_logger::init().unwrap();

    unsafe {
        puts_event!("info", "name" => "pid", "value" => libc::getpid());
    }

    let (window, image) = setup();

    let (mut app, rx, inputs, fragiles, commands) = parse_arguments(&window, image);

    let tx = app.tx.clone();
    let (primary_tx, primary_rx) = channel();

    window.connect_key_press_event(clone_army!([primary_tx] move |_, key| events::on_key_press(primary_tx.clone(), KeyData::new(key))));
    window.connect_configure_event(clone_army!([primary_tx] move |_, _| events::on_configure(primary_tx.clone())));
    window.connect_button_press_event(clone_army!([primary_tx] move |_, button| events::on_button_press(primary_tx.clone(), button)));

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

    'outer: loop {
        while gtk::events_pending() {
            gtk::main_iteration();
        }

        for op in primary_rx.try_iter() {
            println!("primary: {:?}", op);
            app.operate(&op);
        }

        let t = Instant::now();

        for op in rx.try_iter() {
            app.operate(&op);
            if t.elapsed() > Duration::from_millis(10) {
                continue 'outer;
            }
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
    let mut encodings: Vec<String> = vec![];

    {
        let mut width: Option<u32> = None;
        let mut height: Option<u32> = None;

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
            ap.refer(&mut eco.ratio).add_option(&["--ratio", "-R"], StoreOption, "Width / Height");
            ap.refer(&mut width).add_option(&["--width"], StoreOption, "Width");
            ap.refer(&mut height).add_option(&["--height"], StoreOption, "Height");
            // Options
            ap.refer(&mut app_options.show_text).add_option(&["--show-info"], StoreTrue, "Show information bar on window bottom");
            // Limitation
            ap.refer(&mut max_http_threads).add_option(&["--max-http-threads", "-t"], Store, "Maximum number of HTTP Threads");
            // Archive
            ap.refer(&mut encodings).add_option(&["--encoding", "--enc"], Collect, "Character encoding for filename in archives");
            // Files
            ap.refer(&mut files).add_argument("images", List, "Image files or URLs");

            ap.parse_args_or_exit();
        }

        if let Some(width) = width { eco.min_width = Some(width); eco.max_width = Some(width); }
        if let Some(height) = height { eco.min_height = Some(height); eco.max_height = Some(height); }
    }

    let encodings = parse_encodings(&encodings);

    let (app, rx) = app::App::new(eco, max_http_threads, expand, expand_recursive, shuffle, files, fragiles.clone(), window.clone(), image, encodings, app_options);

    load_config(app.tx.clone());

    (app, rx, inputs, fragiles, commands)
}


fn parse_encodings(names: &Vec<String>) -> Vec<EncodingRef> {
    let mut result = vec![];

    for name in names.iter() {
        if let Some(encoding) = encoding_from_whatwg_label(name) {
            result.push(encoding);
        } else {
            puts_error!("invalid_encoding_name" => name);
        }
    }

    result
}


fn load_config(tx: Sender<Operation>) {
    use operation::Operation::*;
    use mapping::Input;
    use options::AppOptionName::*;

    let filepath = {
        let mut path = home_dir().unwrap();
        path.push(".config");
        path.push("chrysoberyl");
        path.push("rc.conf");
        path
    };

    if let Ok(file) = File::open(&filepath) {
        puts_event!("config_file", "state" => "open");
        let file = BufReader::new(file);
        for line in file.lines() {
            let line = line.unwrap();
            tx.send(Operation::from_str_force(&line)).unwrap();
        }
        puts_event!("config_file", "state" => "close");
    } else {
        tx.send(Map(Input::key("h"), Box::new(First))).unwrap();
        tx.send(Map(Input::key("j"), Box::new(Next))).unwrap();
        tx.send(Map(Input::key("k"), Box::new(Previous))).unwrap();
        tx.send(Map(Input::key("l"), Box::new(Last))).unwrap();
        tx.send(Map(Input::key("q"), Box::new(Quit))).unwrap();
        tx.send(Map(Input::key("z"), Box::new(Shuffle(false)))).unwrap();
        tx.send(Map(Input::key("e"), Box::new(Expand(None)))).unwrap();
        tx.send(Map(Input::key("E"), Box::new(ExpandRecursive(None)))).unwrap();
        tx.send(Map(Input::key("i"), Box::new(Toggle(ShowText)))).unwrap();
        tx.send(Map(Input::key("r"), Box::new(Refresh))).unwrap();
    }
}
