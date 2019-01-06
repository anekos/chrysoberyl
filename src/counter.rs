
use crate::errors::ChryError;



#[derive(Clone, Default)]
pub struct Counter {
    value: Option<usize>,
    stack: Vec<Option<usize>>,
}


impl Counter {
    /**
     * If `count` is not None, overwrite `self.count`
     */
    pub fn overwrite(&mut self, value: Option<usize>) -> &mut Counter {
        if value.is_some() {
            self.value = value;
        }
        self
    }

    pub fn peek(&self) -> usize {
        self.value.unwrap_or(1)
    }

    pub fn pop(&mut self) -> Result<(), ChryError> {
        if let Some(item) = self.stack.pop() {
            self.value = item;
            Ok(())
        } else {
            Err(ChryError::Fixed("Empty stack"))
        }
    }

    pub fn push(&mut self) {
        self.stack.push(self.value.take());
    }

    pub fn push_digit(&mut self, digit: u8) {
        if let Some(value) = self.value {
            self.set(Some(value * 10 + digit as usize));
        } else if digit > 0 {
            self.set(Some(digit as usize));
        }
    }

    pub fn set(&mut self, value: Option<usize>) {
        self.value = value;
    }

    pub fn take(&mut self) -> usize {
        if_let_some!(result = self.value, 1);
        self.set(None);
        result
    }

    pub fn take_option(&mut self) -> Option<usize> {
        if_let_some!(result = self.value, None);
        self.set(None);
        Some(result)
    }
}
