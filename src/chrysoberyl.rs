
use std::env::home_dir;
use std::fs::File;
use std::io::{BufReader, BufRead};
use std::sync::mpsc::{Sender, Receiver};
use std::thread::{sleep};
use std::time::{Duration, Instant};

use argparse::{ArgumentParser, List, Collect, Store, StoreTrue, StoreOption, Print};
use encoding::EncodingRef;
use encoding::label::encoding_from_whatwg_label;
use env_logger;
use gtk::prelude::*;
use gtk::{self, Image, Label};
use libc;

use app;
use entry::EntryContainerOptions;
use operation::Operation;
use options::AppOptions;



pub fn main() {
    env_logger::init().unwrap();

    let gui = setup_gui();
    let (mut app, primary_rx, secondary_rx) = parse_arguments(gui.clone());

    unsafe {
        puts_event!("info", "name" => "pid", "value" => libc::getpid());
    }

    'outer: loop {
        while gtk::events_pending() {
            gtk::main_iteration();
        }

        for op in primary_rx.try_iter() {
            app.operate(&op);
        }

        let t = Instant::now();

        for op in secondary_rx.try_iter() {
            app.operate(&op);
            if t.elapsed() > Duration::from_millis(10) {
                continue 'outer;
            }
        }

        sleep(Duration::from_millis(10));
    }
}


fn setup_gui() -> app::Gui {
    use gtk::Orientation;

    gtk::init().unwrap();

    let window = gtk::Window::new(gtk::WindowType::Toplevel);

    window.set_title("Chrysoberyl");
    window.set_border_width(0);
    window.set_position(gtk::WindowPosition::Center);

    let vbox = gtk::Box::new(Orientation::Vertical, 0);

    let image = Image::new_from_pixbuf(None);

    let label = Label::new(Some(&format!("Chrysoberyl v{}", env!("CARGO_PKG_VERSION"))));

    vbox.pack_end(&label, false, false, 0);
    vbox.pack_end(&image, true, true, 0);
    window.add(&vbox);

    vbox.show();
    image.show();
    window.show();

    app::Gui {
        window: window,
        image: image,
        label: label
    }
}


fn parse_arguments(gui: app::Gui) -> (app::App, Receiver<Operation>, Receiver<Operation>) {
    let mut eco = EntryContainerOptions::new();
    let mut app_options = AppOptions::new();
    let mut encodings: Vec<String> = vec![];
    let mut initial = app::Initial::new();

    {
        let mut width: Option<u32> = None;
        let mut height: Option<u32> = None;

        {

            let mut ap = ArgumentParser::new();

            ap.set_description("Controllable Image Viewer");

            // Initial
            ap.refer(&mut initial.expand).add_option(&["--expand", "-e"], StoreTrue, "`Expand` first file");
            ap.refer(&mut initial.expand_recursive).add_option(&["--expand-recursive", "-E"], StoreTrue, "`Expand` first file");
            ap.refer(&mut initial.shuffle).add_option(&["--shuffle", "-z"], StoreTrue, "Shuffle file list");
            ap.refer(&mut initial.http_threads).add_option(&["--max-http-threads", "-t"], Store, "Maximum number of HTTP Threads");
            ap.refer(&mut encodings).add_option(&["--encoding", "--enc"], Collect, "Character encoding for filename in archives");
            ap.refer(&mut initial.files).add_argument("images", List, "Image files or URLs");
            // Controllers
            ap.refer(&mut initial.controllers.inputs).add_option(&["--input", "-i"], Collect, "Controller files");
            ap.refer(&mut initial.controllers.commands).add_option(&["--command", "-c"], Collect, "Controller command");
            ap.refer(&mut initial.controllers.fragiles).add_option(&["--fragile", "-f"], Collect, "Chrysoberyl makes `fifo` controller file");
            // Options
            ap.refer(&mut app_options.show_text).add_option(&["--show-info"], StoreTrue, "Show information bar on window bottom");
            // Container
            ap.refer(&mut eco.min_width).add_option(&["--min-width", "-w"], StoreOption, "Minimum width");
            ap.refer(&mut eco.min_height).add_option(&["--min-height", "-h"], StoreOption, "Minimum height");
            ap.refer(&mut eco.max_width).add_option(&["--max-width", "-W"], StoreOption, "Maximum width");
            ap.refer(&mut eco.max_height).add_option(&["--max-height", "-H"], StoreOption, "Maximum height");
            ap.refer(&mut eco.ratio).add_option(&["--ratio", "-R"], StoreOption, "Width / Height");
            ap.refer(&mut width).add_option(&["--width"], StoreOption, "Width");
            ap.refer(&mut height).add_option(&["--height"], StoreOption, "Height");

            ap.add_option(&["-V", "--version"], Print(env!("CARGO_PKG_VERSION").to_string()), "Show version");

            ap.parse_args_or_exit();
        }

        if let Some(width) = width { eco.min_width = Some(width); eco.max_width = Some(width); }
        if let Some(height) = height { eco.min_height = Some(height); eco.max_height = Some(height); }
    }

    initial.encodings = parse_encodings(&encodings);

    let (app, primary_rx, rx) = app::App::new(initial, app_options, gui, eco);

    load_config(app.tx.clone());

    (app, primary_rx, rx)
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
