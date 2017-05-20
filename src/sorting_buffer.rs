
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::collections::HashMap;


pub type Ticket = usize;


#[derive(Clone)]
pub struct SortingBuffer<T> {
    reserved: Arc<AtomicUsize>,
    shipped: Arc<Mutex<Ticket>>,
    buffer: Arc<Mutex<HashMap<Ticket, Option<T>>>>
}


impl<T> SortingBuffer<T> {
    #[cfg_attr(feature = "cargo-clippy", allow(mutex_atomic))]
    pub fn new() -> SortingBuffer<T> {
        SortingBuffer {
            reserved: Arc::new(AtomicUsize::new(0)),
            shipped: Arc::new(Mutex::new(1)),
            buffer: Arc::new(Mutex::new(HashMap::new()))
        }
    }

    pub fn reserve(&mut self) -> Ticket {
        reserve_n(&self.reserved, 1)
    }

    pub fn reserve_n(&mut self, n: usize) -> Ticket {
        reserve_n(&self.reserved, n)
    }

    pub fn push(&mut self, ticket: Ticket, entry: T) {
        let mut buffer = self.buffer.lock().unwrap();
        buffer.insert(ticket, Some(entry));
    }

    pub fn push_without_reserve(&mut self, entry: T) -> Vec<T> {
        let ticket = self.reserve();
        let mut buffer = self.buffer.lock().unwrap();
        buffer.insert(ticket, Some(entry));

        let mut shipped = self.shipped.lock().unwrap();
        pull_all(&mut buffer, &mut shipped)
    }

    fn push_n_without_reserve(&mut self, entries: Vec<T>) -> Vec<T> {
        let mut buffer = self.buffer.lock().unwrap();
        let ticket = reserve_n(&self.reserved, buffer.len());

        for (index, entry) in entries.into_iter().enumerate() {
            buffer.insert(ticket + index, Some(entry));
        }

        let mut shipped = self.shipped.lock().unwrap();
        pull_all(&mut buffer, &mut shipped)
    }

    pub fn skip(&mut self, ticket: Ticket) {
        let mut buffer = self.buffer.lock().unwrap();
        buffer.insert(ticket, None);
    }

    pub fn pull(&mut self) -> Option<T> {
        let mut shipped = self.shipped.lock().unwrap();
        let mut buffer = self.buffer.lock().unwrap();

        pull(&mut buffer, &mut shipped)
    }

    pub fn pull_all(&mut self) -> Vec<T> {
        let mut shipped = self.shipped.lock().unwrap();
        let mut buffer = self.buffer.lock().unwrap();

        pull_all(&mut buffer, &mut shipped)
    }

    pub fn len(&self) -> usize {
        let buffer = self.buffer.lock().unwrap();
        buffer.len()
    }
}


fn reserve_n(reserved: &AtomicUsize, n: usize) -> Ticket {
    reserved.fetch_add(n, Ordering::Relaxed) + 1
}

fn pull<T>(buffer: &mut HashMap<Ticket, Option<T>>, shipped: &mut Ticket) -> Option<T> {
    while !buffer.is_empty() {
        let result = buffer.remove(&*shipped);
        if result.is_none() {
            return None
        }
        *shipped += 1;
        if let Some(result) = result {
            return result
        }
    }

    None
}

fn pull_all<T>(buffer: &mut HashMap<Ticket, Option<T>>, shipped: &mut Ticket) -> Vec<T> {
    let mut result = vec![];
    while let Some(it) = pull(buffer, shipped) {
        result.push(it);
    }
    result
}
