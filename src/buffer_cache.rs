
use std::collections::HashMap;
use std::hash::Hash;
use std::sync::Arc;
use std::sync::mpsc::{Sender, channel};
use std::thread::spawn;


pub struct BufferCache<T: Hash + Eq + Send> {
    pub tx: Sender<Operation<T>>
}


pub enum Operation<T: Hash + Eq + Send> {
    Fill(T, Vec<u8>),
    Tell(T, Sender<Arc<Vec<u8>>>)
}


impl<T: 'static + Hash + Eq + Send> BufferCache<T> {
    pub fn new() -> BufferCache<T> {
        BufferCache {
            tx: collector()
        }
    }

    pub fn get(&self, key: T) -> Arc<Vec<u8>> {
        let (tx, rx) = channel();
        self.tx.send(Operation::Tell(key, tx)).unwrap();
        rx.recv().unwrap()
    }

}


fn collector<T: 'static + Hash + Eq + Send>() -> Sender<Operation<T>> {
    let (tx, rx) = channel();

    spawn(move || {
        use self::Operation::*;

        let mut cache: HashMap<T, Arc<Vec<u8>>> = HashMap::new();
        let mut tell_to: Option<(T, Sender<Arc<Vec<u8>>>)> = None;

        while let Ok(op) = rx.recv() {
            match op {
                Fill(key, buffer) => {
                    let entry = Arc::new(buffer);
                    if let Some((ref tt_key, ref tx)) = tell_to {
                        if key == *tt_key {
                            tx.send(entry.clone()).unwrap();
                        }
                    }
                    cache.insert(key, entry);
                }
                Tell(key, tx) => {
                    if let Some(buffer) = cache.get(&key) {
                        tx.send(buffer.clone()).unwrap();
                    } else {
                        if let Some(_) = tell_to {
                            panic!("tell_to has already been registered")
                        } else {
                            tell_to = Some((key, tx));
                        }
                    }
                }
            }
        }

    });

    tx
}
