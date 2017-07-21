
#[derive(Clone)]
pub struct Counter {
    value: Option<usize>
}


impl Counter {
    pub fn new() -> Self {
        Counter { value: None }
    }

    pub fn set(&mut self, value: Option<usize>) {
        self.value = value;
    }

    pub fn pop(&mut self) -> usize {
        if_let_some!(result = self.value, 1);
        self.set(None);
        result
    }

    pub fn push_digit(&mut self, digit: u8) {
        if let Some(value) = self.value {
            self.set(Some(value * 10 + digit as usize));
        } else if digit > 0 {
            self.set(Some(digit as usize));
        }
    }

    /**
     * If `count` is not None, overwrite `self.count`
     */
    pub fn overwrite(&mut self, value: Option<usize>) -> &mut Counter {
        if value.is_some() {
            self.value = value;
        }
        self
    }
}
