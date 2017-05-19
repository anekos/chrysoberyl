
use std::sync::{Arc, Mutex};
use std::collections::HashMap;


pub type Ticket = usize;


#[derive(Clone)]
pub struct SortingBuffer<T> {
    reserved: Arc<Mutex<Ticket>>,
    shipped: Arc<Mutex<Ticket>>,
    buffer: Arc<Mutex<HashMap<Ticket, Option<T>>>>
}


impl<T> SortingBuffer<T> {
    pub fn new() -> SortingBuffer<T> {
        SortingBuffer {
            reserved: Arc::new(Mutex::new(0)),
            shipped: Arc::new(Mutex::new(1)),
            buffer: Arc::new(Mutex::new(HashMap::new()))
        }
    }

    pub fn reserve(&mut self) -> Ticket {
        let mut reserved = self.reserved.lock().unwrap();
        *reserved += 1;
        *reserved
    }

    pub fn push(&mut self, ticket: Ticket, entry: T) {
        let mut buffer = self.buffer.lock().unwrap();
        buffer.insert(ticket, Some(entry));
    }

    pub fn push_without_reserve(&mut self, entry: T) {
        let mut reserved = self.reserved.lock().unwrap();
        let mut buffer = self.buffer.lock().unwrap();
        *reserved += 1;
        buffer.insert(*reserved, Some(entry));
    }

    pub fn skip(&mut self, ticket: Ticket) {
        let mut buffer = self.buffer.lock().unwrap();
        buffer.insert(ticket, None);
    }

    pub fn pull(&mut self) -> Option<T> {
        let mut shipped = self.shipped.lock().unwrap();
        let mut buffer = self.buffer.lock().unwrap();

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

    pub fn len(&self) -> usize {
        let buffer = self.buffer.lock().unwrap();
        buffer.len()
    }
}
