
use std::process::exit;
use std::sync::mpsc::Receiver;
use std::thread::sleep;
use std::time::{Duration, Instant};

use env_logger;
use gtk;

use app;
use command_line;
use events::EventName;
use operation::Operation;



pub fn main() {
    use self::Operation::UpdateUI;

    env_logger::init().unwrap();

    put_features();

    let (mut app, primary_rx, secondary_rx) = parse_arguments();

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


fn parse_arguments() -> (app::App, Receiver<Operation>, Receiver<Operation>) {
    if_let_ok!(initial = command_line::parse_args(), |err| {
        println!("{}", err);
        exit(1);
    });

    let (app, primary_rx, rx) = app::App::new(initial);

    app.tx.send(EventName::Initialize.operation()).unwrap();

    (app, primary_rx, rx)
}


fn put_features() {
    if cfg!(feature = "poppler_lock") {
        info!("main: features=[+poppler_lock]");
    } else {
        info!("main: features=[-poppler_lock]");
    }
}
