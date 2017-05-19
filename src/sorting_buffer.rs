
use std::collections::HashMap;



pub struct SortingBuffer<T> {
    next_serial: usize,
    buffer: HashMap<usize, Option<T>>
}


impl<T> SortingBuffer<T> {
    pub fn new(next_serial: usize) -> SortingBuffer<T> {
        SortingBuffer { next_serial: next_serial, buffer: HashMap::new() }
    }

    pub fn push(&mut self, serial: usize, entry: T) {
        self.buffer.insert(serial, Some(entry));
    }

    pub fn skip(&mut self, serial: usize) {
        self.buffer.insert(serial, None);
    }

    pub fn pull(&mut self) -> Option<T> {
        while !self.buffer.is_empty() {

            let result = self.buffer.remove(&self.next_serial);
            if result.is_none() {
                return None
            }

            self.next_serial += 1;
            if let Some(result) = result {
                return result
            }
        }

        None
    }
}
