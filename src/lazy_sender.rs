
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
use std::thread::{sleep, spawn};
use std::time::{Duration, Instant};

use crate::operation::Operation;



#[derive(Clone)]
pub struct LazySender {
    current: Arc<Mutex<Option<(Instant, Operation)>>>,
    delay: Duration,
    tx: Sender<Operation>,
}


impl LazySender {
    pub fn new(tx: Sender<Operation>, delay: Duration) -> LazySender {
        LazySender { current: Arc::new(Mutex::new(None)), tx, delay }
    }

    pub fn cancel(&self) {
        let mut current = self.current.lock().unwrap();
        *current = None;
    }

    pub fn initialize(&self, op: Operation)  {
        let current = self.current.lock().unwrap();
        if current.is_none() {
            self.tx.send(op).unwrap();
        }
    }

    pub fn request(&self, op: Operation)  {
        let mut current = self.current.lock().unwrap();
        let expired_at = Instant::now() + self.delay;

        if current.is_some() {
            *current = Some((expired_at, op));
            return
        }

        *current = Some((expired_at, op));

        let tx = self.tx.clone();
        let delay = self.delay;
        let current = Arc::clone(&self.current);

        spawn(move || {
            let mut delay = delay;

            loop {
                sleep(delay);

                {
                    let mut current = current.lock().unwrap();

                    if let Some((expired_at, op)) = current.as_ref() {
                        let now = Instant::now();
                        if *expired_at <= now {
                            tx.send(op.clone()).unwrap();
                        } else {
                            delay = *expired_at - now;
                            continue;
                        }
                    }
                    *current = None;
                    break;
                }
            }
        });
    }

    pub fn set_delay(&mut self, duration: Duration) {
        self.delay = duration;
    }
}
