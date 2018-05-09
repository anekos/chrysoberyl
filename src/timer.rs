
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Sender, channel};
use std::thread::{spawn, sleep};
use std::time::Duration;

use uuid::Uuid;
use uuid_to_pokemon::uuid_to_pokemon;

use operation::Operation;



pub struct TimerManager {
    pub table: HashMap<String, Timer>,
    app_tx: Sender<Operation>,
}

pub struct Timer {
    pub interval: Duration,
    pub operation: Vec<String>,
    pub repeat: Option<usize>,
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

    pub fn register(&mut self, name: Option<String>, op: Vec<String>, interval: Duration, repeat: Option<usize>) {
        let name = name.unwrap_or_else(|| {
            let id = Uuid::new_v4();
            s!(uuid_to_pokemon(id))
        });
        let timer = Timer::new(name.clone(), op, self.app_tx.clone(), interval, repeat);
        if let Some(old) = self.table.insert(name, timer) {
            if old.is_live() {
                old.tx.send(TimerOperation::Kill).unwrap();
            }
        }
    }

    pub fn unregister(&mut self, name: &str) {
        match self.table.remove(name) {
            Some(timer) => {
                timer.tx.send(TimerOperation::Kill).unwrap();
            }
            None => {
                puts_error!(chry_error!("timer `{}` is not found", name), "at" => "timer/kill");
            }
        }
    }
}


impl Timer {
    pub fn new(name: String, operation: Vec<String>, app_tx: Sender<Operation>, interval: Duration, repeat: Option<usize>) -> Timer {
        let (tx, rx) = channel();
        let live = Arc::new(AtomicBool::new(true));

        sleep_and_fire(interval, &tx);

        spawn(clone_army!([live, tx, operation] move || {
            use self::TimerOperation::*;

            let mut repeat = repeat;

            while let Ok(top) = rx.recv() {
                match top {
                    Kill => break,
                    Fire => {
                        puts_event!("timer/fire", "name" => name);
                        match Operation::parse_from_vec(&operation) {
                            Ok(op) => app_tx.send(op).unwrap(),
                            Err(err) => puts_error!(err, "at" => "timer/fire"),
                        }
                        if let Some(repeat) = repeat.as_mut() {
                            *repeat -= 1;
                        }
                        if repeat == Some(0) {
                            break;
                        } else {
                            sleep_and_fire(interval, &tx);
                        }
                    }
                }
            }

            live.store(false, Ordering::SeqCst);
        }));

        Timer { tx, live, operation, interval, repeat }
    }

    pub fn is_live(&self) -> bool {
        self.live.load(Ordering::SeqCst)
    }
}


fn sleep_and_fire(duration: Duration, tx: &Sender<TimerOperation>) {
    spawn(clone_army!([tx] move || {
        sleep(duration);
        let _ = tx.send(TimerOperation::Fire);
    }));
}
