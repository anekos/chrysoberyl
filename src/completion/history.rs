
use std::convert::AsRef;

use crate::app_path;
use crate::util::file::{read_lines, write_line};



pub struct History {
    items: Vec<String>,
    next_at: usize,
}


impl History {
    pub fn new() -> Self {
        let path = app_path::entry_history();
        if let Ok(items) = read_lines(&path) {
            return History { items, next_at: 0 }
        }
        History { items: vec![], next_at: 0 }
    }

    pub fn reset(&mut self) {
        self.next_at = 0;
    }

    pub fn forward(&mut self) -> Option<&str> {
        if self.items.is_empty() {
            return None;
        }

        let n = self.next_at;

        if self.next_at == 0 {
            self.next_at = self.items.len() - 1;
        } else {
            self.next_at -= 1;
        }

        self.nth(n)
    }

    pub fn backward(&mut self) -> Option<&str> {
        if self.items.is_empty() {
            return None;
        }

        let n = self.next_at;

        if self.next_at < self.items.len() - 1 {
            self.next_at += 1;
        } else {
            self.next_at = 0;
        }

        self.nth(n)
    }

    pub fn push(&mut self, line: String) {
        if let Some(last) = self.items.last() {
            if line == *last {
                return;
            }
        }
        let _ = write_line(&line, &Some(app_path::entry_history()));
        self.items.push(line);
    }

    fn nth(&self, n: usize) -> Option<&str> {
        self.items.get(self.items.len() - n - 1).map(AsRef::as_ref)
    }

}
