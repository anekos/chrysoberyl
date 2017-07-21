
use std::sync::mpsc::Receiver;
use std::thread::sleep;
use std::time::{Duration, Instant};

use argparse::{ArgumentParser, List, Collect, Store, StoreTrue, Print};
use encoding::EncodingRef;
use encoding::label::encoding_from_whatwg_label;
use env_logger;
use gtk;

use app;
use app_path;
use script;
use config;
use gui::Gui;
use operation::Operation;



pub fn main() {
    use self::Operation::UpdateUI;

    env_logger::init().unwrap();

    put_features();

    let gui = Gui::new();

    let (mut app, primary_rx, secondary_rx) = parse_arguments(gui.clone());

    'outer: loop {
        while gtk::events_pending() {
            gtk::main_iteration();
        }

        for op in primary_rx.try_iter() {
            match op {
                UpdateUI => continue 'outer,
                op => app.operate(op),
            }
        }

        let t = Instant::now();

        for op in secondary_rx.try_iter() {
            match op {
                UpdateUI => continue 'outer,
                op => app.operate(op),
            }
            if t.elapsed() > Duration::from_millis(10) {
                continue 'outer;
            }
        }

        sleep(Duration::from_millis(10));
    }
}


fn parse_arguments(gui: Gui) -> (app::App, Receiver<Operation>, Receiver<Operation>) {
    use state::*;

    let mut states = States::default();
    let mut encodings: Vec<String> = vec![];
    let mut initial = app::Initial::new();

    {
        let path = format!(
            "Configuration: {}\nCache: {}",
            app_path::config_file(None).to_str().unwrap(),
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
                .add_option(&["--status-bar"], StoreTrue, "Show status bar");
            ap.refer(&mut states.reverse)
                .add_option(&["--reverse"], StoreTrue, "Reverse in multi view");
            ap.refer(&mut states.view.center_alignment)
                .add_option(&["--center"], StoreTrue, "Center alignment in multi view");

            ap.add_option(&["-V", "--version"], Print(env!("CARGO_PKG_VERSION").to_string()), "Show version");

            ap.add_option(&["--print-default"], Print(o!(config::DEFAULT_CONFIG)), "Print default config");
            ap.add_option(&["--print-path"], Print(path), "Print application files path");

            ap.parse_args_or_exit();
        }
    }

    initial.encodings = parse_encodings(&encodings);

    let (app, primary_rx, rx) = app::App::new(initial, states, gui);

    script::load(&app.tx, &config::get_config_source());

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

fn put_features() {
    if cfg!(feature = "poppler_lock") {
        info!("main: features=[+poppler_lock]");
    } else {
        info!("main: features=[-poppler_lock]");
    }
}
