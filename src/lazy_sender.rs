
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
use std::thread::{sleep, spawn};
use std::time::{Duration, Instant};

use operation::Operation;



#[derive(Clone)]
pub struct LazySender {
    current: Arc<Mutex<Option<(Instant, Operation)>>>,
    delay: Duration,
    tx: Sender<Operation>,
}


impl LazySender {
    pub fn new(tx: Sender<Operation>, delay: Duration) -> LazySender {
        LazySender { current: Arc::new(Mutex::new(None)), tx: tx, delay: delay }
    }

    pub fn request(&mut self, op: Operation)  {
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
                    let current = current.lock().unwrap();

                    if let Some((expired_at, ref op)) = *current {
                        let now = Instant::now();
                        if expired_at <= now {
                            tx.send(op.clone()).unwrap();
                            break;
                        } else {
                            delay = expired_at - now;
                        }
                    }
                }
            }

            let mut current = current.lock().unwrap();
            *current = None;
        });
    }
}
