
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
use std::thread::{sleep, spawn};
use std::time::Duration;

use operation::Operation;



#[derive(Clone)]
pub struct LazySender {
    serial: Arc<Mutex<u64>>,
    delay: Duration,
    tx: Sender<Operation>,
}


impl LazySender {
    pub fn new(tx: Sender<Operation>, delay: Duration) -> LazySender {
        LazySender { serial: Arc::new(Mutex::new(0)), tx: tx, delay: delay }
    }

    pub fn request(&mut self, item: Operation)  {
        let mut serial = self.serial.lock().unwrap();
        *serial += 1;

        let tx = self.tx.clone();
        let delay = self.delay.clone();
        let current_serial = self.serial.clone();
        let serial = *serial;

        spawn(move || {
            sleep(delay);
            let current_serial = current_serial.lock().unwrap();
            if *current_serial == serial {
                tx.send(item).unwrap();
            }
        });
    }
}
