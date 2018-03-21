
use std::mem::swap;

use regex::Regex;



pub struct Joiner {
    buffer: String,
    pattern: Regex,
}


impl Joiner {
    pub fn new() -> Self {
        Joiner { buffer: o!(""), pattern: Regex::new(r"\\+\z").unwrap()}
    }

    pub fn push(&mut self, line: &str) -> Option<String> {
        if let Some(found) = self.pattern.find(line) {
            if (found.end() - found.start()) % 2 == 1 {
                self.buffer.push_str(&line[0..line.len() - 1]);
                return None;
            }
        }

        let mut result = o!("");
        self.buffer.push_str(line);
        swap(&mut result, &mut self.buffer);
        Some(result)
    }
}
