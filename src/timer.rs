
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Sender, channel};
use std::thread::{spawn, sleep};
use std::time::Duration;

use operation::Operation;



pub struct TimerManager {
    table: HashMap<String, Timer>,
    app_tx: Sender<Operation>,
}

pub struct Timer {
    tx: Sender<TimerOperation>,
    live: Arc<AtomicBool>,
}

#[derive(Clone, Copy)]
pub enum TimerOperation {
    Kill,
    Fire,
}


impl TimerManager {
    pub fn new(app_tx: Sender<Operation>) -> TimerManager {
        TimerManager {
            table: HashMap::new(),
            app_tx: app_tx,
        }
    }

    pub fn register(&mut self, name: String, op: Vec<String>, interval: Duration) {
        let timer = Timer::new(
            name.clone(),
            op,
            self.app_tx.clone(),
            interval);
        if let Some(old) = self.table.insert(name, timer) {
            if old.live.load(Ordering::SeqCst) {
                old.tx.send(TimerOperation::Kill).unwrap();
            }
        }
    }
}


impl Timer {
    pub fn new(name: String, op: Vec<String>, app_tx: Sender<Operation>, interval: Duration) -> Timer {
        let (tx, rx) = channel();
        let live = Arc::new(AtomicBool::new(true));

        sleep_and_fire(interval, &tx);

        spawn(clone_army!([live, tx] move || {
            use self::TimerOperation::*;

            while let Ok(top) = rx.recv() {
                match top {
                    Kill => {
                        live.store(false, Ordering::SeqCst);
                        return;
                    }
                    Fire => {
                        puts_event!("timer/fire", "name" => name);
                        match Operation::parse_from_vec(&op) {
                            Ok(op) => app_tx.send(op).unwrap(),
                            Err(err) => puts_error!("at" => "timer/fire", "reason" => err),
                        }
                        sleep_and_fire(interval, &tx);
                    }
                }
            }
        }));

        Timer { tx: tx, live: live }
    }
}


fn sleep_and_fire(duration: Duration, tx: &Sender<TimerOperation>) {
    spawn(clone_army!([tx] move || {
        sleep(duration);
        tx.send(TimerOperation::Fire).unwrap();
    }));
}
