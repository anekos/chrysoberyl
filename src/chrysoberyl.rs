
use std::collections::HashMap;
use std::process::exit;
use std::sync::mpsc::Receiver;
use std::thread::sleep;
use std::time::{Duration, Instant};

use log::info;

use crate::app;
use crate::command_line;
use crate::events::EventName;
use crate::operation::Operation;



pub fn main() {
    use self::Operation::UpdateUI;

    env_logger::init();

    put_features();

    let (mut app, primary_rx, secondary_rx) = parse_arguments();

    let mut idle = None;
    let mut idle_fired = false;

    'outer: loop {
        while gtk::events_pending() {
            gtk::main_iteration();
        }

        for op in primary_rx.try_iter() {
            idle = None;
            match op {
                UpdateUI => continue 'outer,
                op => app.operate(op, None),
            }
        }

        let t = Instant::now();

        for op in secondary_rx.try_iter() {
            idle = None;
            match op {
                UpdateUI => continue 'outer,
                op => app.operate(op, None),
            }
            if Duration::from_millis(10) < t.elapsed() {
                continue 'outer;
            }
        }

        if let Some(idle) = idle {
            let now = Instant::now();
            if !idle_fired && app.states.idle_time < now.duration_since(idle) {
                app.operate(Operation::AppEvent(EventName::Idle, HashMap::new()), None);
                idle_fired = true;
            }
        } else {
            idle = Some(Instant::now());
            idle_fired = false;
        }

        sleep(Duration::from_millis(10));
    }
}


fn parse_arguments() -> (app::App, Receiver<Operation>, Receiver<Operation>) {
    if_let_ok!(initial = command_line::parse_args(), |err| {
        println!("{}", err);
        exit(1);
    });

    let (app, primary_rx, rx) = app::App::build(initial);

    (app, primary_rx, rx)
}


fn put_features() {
    if cfg!(feature = "poppler_lock") {
        info!("main: features=[+poppler_lock]");
    } else {
        info!("main: features=[-poppler_lock]");
    }
}
