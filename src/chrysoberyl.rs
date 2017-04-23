
use std::sync::mpsc::Receiver;
use std::thread::{sleep};
use std::time::{Duration, Instant};

use argparse::{ArgumentParser, List, Collect, Store, StoreTrue, StoreOption, Print, StoreConst};
use encoding::EncodingRef;
use encoding::label::encoding_from_whatwg_label;
use env_logger;
use gtk;

use app;
use app_path;
use config;
use entry::EntryContainerOptions;
use gui::Gui;
use operation::Operation;



pub fn main() {
    env_logger::init().unwrap();

    // {
    //     use poppler_sys::*;
    //
    //     use std::ptr::{null, null_mut};
    //     use std::ffi::CString;
    //
    //     println!("test");
    //
    //     let p = CString::new("file://tmp/xmosh/foo.pdf").unwrap();
    //     unsafe {
    //         let res = poppler_document_new_from_file(p.as_ptr(), null(), null_mut());
    //         println!("{:?}", res);
    //     }
    // }

    let gui = Gui::new();

    let (mut app, primary_rx, secondary_rx) = parse_arguments(gui.clone());

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


fn parse_arguments(gui: Gui) -> (app::App, Receiver<Operation>, Receiver<Operation>) {
    use state::*;

    let mut eco = EntryContainerOptions::new();
    let mut states = States::new();
    let mut encodings: Vec<String> = vec![];
    let mut initial = app::Initial::new();

    {
        let mut width: Option<u32> = None;
        let mut height: Option<u32> = None;

        let path = format!(
            "Configuration: {}\nCache: {}",
            app_path::config_file().to_str().unwrap(),
            app_path::cache_dir("/").to_str().unwrap());

        {

            let mut ap = ArgumentParser::new();

            ap.set_description("Controllable image viewer");

            // Initial
            ap.refer(&mut initial.expand)
                .add_option(&["--expand", "-e"], StoreTrue, "`Expand` first file");
            ap.refer(&mut initial.expand_recursive)
                .add_option(&["--expand-recursive", "-E"], StoreTrue, "`Expand` first file");
            ap.refer(&mut initial.shuffle)
                .add_option(&["--shuffle", "-z"], StoreTrue, "Shuffle file list");
            ap.refer(&mut initial.http_threads)
                .add_option(&["--max-http-threads", "-t"], Store, "Maximum number of HTTP Threads");
            ap.refer(&mut encodings)
                .add_option(&["--encoding", "--enc"], Collect, "Character encoding for filename in archives");
            ap.refer(&mut initial.files)
                .add_argument("images", List, "Image files or URLs");

            // Controllers
            ap.refer(&mut initial.controllers.inputs)
                .add_option(&["--input", "-i"], Collect, "Controller files")
                .metavar("FILEPATH");
            ap.refer(&mut initial.operations)
                .add_option(&["--operation", "-o"], Collect, "Execute operations at start")
                .metavar("OPERATION");

            // Options
            ap.refer(&mut states.status_bar)
                .add_option(&["--status-bar"], StoreConst(StatusBarValue::Enabled), "Show status bar");
            ap.refer(&mut states.reverse)
                .add_option(&["--reverse"], StoreConst(ReverseValue::Enabled), "Reverse in multi view");
            ap.refer(&mut states.view.center_alignment)
                .add_option(&["--center"], StoreConst(CenterAlignmentValue::Enabled), "Center alignment in multi view");

            // Container
            ap.refer(&mut eco.min_width)
                .add_option(&["--min-width", "-w"], StoreOption, "Minimum width")
                .metavar("PX");
            ap.refer(&mut eco.min_height)
                .add_option(&["--min-height", "-h"], StoreOption, "Minimum height")
                .metavar("PX");
            ap.refer(&mut eco.max_width)
                .add_option(&["--max-width", "-W"], StoreOption, "Maximum width")
                .metavar("PX");
            ap.refer(&mut eco.max_height)
                .add_option(&["--max-height", "-H"], StoreOption, "Maximum height")
                .metavar("PX");
            ap.refer(&mut eco.ratio)
                .add_option(&["--ratio", "-R"], StoreOption, "Width / Height");
            ap.refer(&mut width)
                .add_option(&["--width"], StoreOption, "Width")
                .metavar("PX");
            ap.refer(&mut height)
                .add_option(&["--height"], StoreOption, "Height")
                .metavar("PX");

            ap.add_option(&["-V", "--version"], Print(env!("CARGO_PKG_VERSION").to_string()), "Show version");

            ap.add_option(&["--print-default"], Print(o!(config::DEFAULT_CONFIG)), "Print default config");
            ap.add_option(&["--print-path"], Print(path), "Print application files path");

            ap.parse_args_or_exit();
        }

        if let Some(width) = width { eco.min_width = Some(width); eco.max_width = Some(width); }
        if let Some(height) = height { eco.min_height = Some(height); eco.max_height = Some(height); }
    }

    initial.encodings = parse_encodings(&encodings);

    let (app, primary_rx, rx) = app::App::new(initial, states, gui, eco);

    config::load_config(&app.tx, &config::ConfigSource::User);

    app.tx.send(Operation::Initialized).unwrap();

    (app, primary_rx, rx)
}


fn parse_encodings(names: &[String]) -> Vec<EncodingRef> {
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
