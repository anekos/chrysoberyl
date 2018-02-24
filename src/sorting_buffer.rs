
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::collections::HashMap;
use std::mem::swap;


pub type Ticket = usize;


#[derive(Clone)]
pub struct SortingBuffer<T> {
    reserved: Arc<AtomicUsize>,
    stable: Arc<AtomicBool>,
    shipped: Arc<Mutex<Ticket>>,
    buffer: Arc<Mutex<HashMap<Ticket, Option<T>>>>,
    unstable_buffer: Arc<Mutex<Vec<T>>>,
}


impl<T> SortingBuffer<T> {
    #[cfg_attr(feature = "cargo-clippy", allow(mutex_atomic))]
    pub fn new() -> SortingBuffer<T> {
        SortingBuffer {
            reserved: Arc::new(AtomicUsize::new(0)),
            stable: Arc::new(AtomicBool::new(true)),
            shipped: Arc::new(Mutex::new(1)),
            buffer: Arc::new(Mutex::new(HashMap::new())),
            unstable_buffer: Arc::new(Mutex::new(vec!())),
        }
    }

    pub fn reserve(&mut self) -> Ticket {
        reserve_n(&self.reserved, 1)
    }

    pub fn reserve_n(&mut self, n: usize) -> Ticket {
        reserve_n(&self.reserved, n)
    }

    pub fn push(&mut self, ticket: Ticket, entry: T) {
        let stable = self.stable.load(Ordering::Acquire);
        if stable {
            self.buffer.lock().unwrap().insert(ticket, Some(entry));
        } else {
            self.unstable_buffer.lock().unwrap().push(entry);
            self.skip(ticket);
        }
    }

    pub fn skip(&mut self, ticket: Ticket) {
        self.buffer.lock().unwrap().insert(ticket, None);
    }

    pub fn push_with_reserve(&mut self, entry: T) -> Vec<T> {
        let ticket = self.reserve();
        let mut buffer = self.buffer.lock().unwrap();
        buffer.insert(ticket, Some(entry));

        let mut shipped = self.shipped.lock().unwrap();

        let mut result = vec![];
        pull_all(&mut result, &mut buffer, &mut shipped);
        result
    }

    // pub fn skip_with_reserve(&mut self, ticket: Ticket);

    pub fn pull_all(&mut self) -> Vec<T> {
        let mut shipped = self.shipped.lock().unwrap();
        let mut buffer = self.buffer.lock().unwrap();
        let mut unstable_buffer = self.unstable_buffer.lock().unwrap();

        let mut result = vec![];
        swap(&mut result, &mut unstable_buffer);

        pull_all(&mut result, &mut buffer, &mut shipped);
        result
    }

    pub fn len(&self) -> usize {
        let buffer = self.buffer.lock().unwrap();
        buffer.len()
    }

    pub fn set_stability(&self, stable: bool) {
        self.stable.store(stable, Ordering::Release);
    }
}


fn reserve_n(reserved: &AtomicUsize, n: usize) -> Ticket {
    reserved.fetch_add(n, Ordering::AcqRel) + 1
}

fn pull_all<T>(result: &mut Vec<T>, buffer: &mut HashMap<Ticket, Option<T>>, shipped: &mut Ticket) {
    while !buffer.is_empty() {
        match buffer.remove(&*shipped) {
            None =>
                return,
            Some(next) => {
                *shipped += 1;
                if let Some(next) = next {
                    result.push(next);
                }
            },
        }
    }
}
