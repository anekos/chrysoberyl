
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Sender, channel};
use std::thread::{spawn, sleep};
use std::time::Duration;

use closet::clone_army;
use uuid::Uuid;
use uuid_to_pokemon::uuid_to_pokemon;

use crate::errors::{AppResult, AppResultU, AppError};
use crate::operation::Operation;



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
    Fire,
    Kill,
    Wakeup,
}


impl TimerManager {
    pub fn new(app_tx: Sender<Operation>) -> TimerManager {
        TimerManager {
            table: HashMap::new(),
            app_tx,
        }
    }

    pub fn register(&mut self, name: Option<String>, op: Vec<String>, interval: Duration, repeat: Option<usize>, r#async: bool) -> AppResultU {
        let name = name.unwrap_or_else(new_name);
        let timer = Timer::build(name.clone(), op, self.app_tx.clone(), interval, repeat, r#async)?;
        if let Some(old) = self.table.insert(name, timer) {
            if old.is_live() {
                old.tx.send(TimerOperation::Kill).unwrap();
            }
        }
        Ok(())
    }

    pub fn unregister(&mut self, name: &str) -> AppResultU {
        match self.table.remove(name) {
            Some(timer) => {
                timer.tx.send(TimerOperation::Kill).unwrap();
                Ok(())
            }
            None => {
                Err(AppError::TimerNotFound(s!(name)))
            }
        }
    }

    pub fn wakeup(&self, name: &str) {
        self.table[name].wakeup();
    }
}


impl Timer {
    pub fn build(name: String, operation: Vec<String>, app_tx: Sender<Operation>, interval: Duration, repeat: Option<usize>, r#async: bool) -> AppResult<Timer> {
        let (tx, rx) = channel();
        let live = Arc::new(AtomicBool::new(true));

        sleep_and_fire(interval, &tx);

        let op = Operation::parse_from_vec(&operation)?;

        spawn(clone_army!([live, tx] move || {
            use self::TimerOperation::*;

            let mut repeat = repeat;

            while let Ok(top) = rx.recv() {
                match top {
                    Kill => break,
                    Fire => {
                        puts_event!("timer/fire", "name" => name);
                        app_tx.send(op.clone()).unwrap();
                        if let Some(repeat) = repeat.as_mut() {
                            *repeat -= 1;
                        }
                        if repeat == Some(0) {
                            break;
                        }
                        if r#async {
                            sleep_and_fire(interval, &tx);
                        } else {
                            app_tx.send(Operation::WakeupTimer(name.clone())).unwrap();
                        }
                    },
                    Wakeup =>
                        sleep_and_fire(interval, &tx),
                }
            }

            live.store(false, Ordering::SeqCst);
        }));

        Ok(Timer { tx, live, operation, interval, repeat })
    }

    pub fn is_live(&self) -> bool {
        self.live.load(Ordering::SeqCst)
    }

    pub fn wakeup(&self) {
        self.tx.send(TimerOperation::Wakeup).unwrap();
    }
}


fn sleep_and_fire(duration: Duration, tx: &Sender<TimerOperation>) {
    spawn(clone_army!([tx] move || {
        sleep(duration);
        let _ = tx.send(TimerOperation::Fire);
    }));
}

fn new_name() -> String {
    let id = Uuid::new_v4();
    s!(uuid_to_pokemon(id)).replace(' ', "-").to_lowercase()
}
