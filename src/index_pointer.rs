

pub struct IndexPointer {
    pub current: Option<usize>,
    count: Option<usize>,
}


impl IndexPointer {
    pub fn new() -> IndexPointer {
        IndexPointer { current: None, count: None }
    }

    pub fn set_count(&mut self, count: Option<usize>) {
        self.count = count;
    }

    pub fn push_count_digit(&mut self, n: u8) {
        if let Some(current) = self.count {
            self.set_count(Some(current * 10 + n as usize));
        } else if n > 0 {
            self.set_count(Some(n as usize));
        }
    }

    pub fn first(&mut self, container_size: usize) -> bool {
        if container_size < 1 {
            return false
        }

        let delta = self.counted();
        let result = if delta <= container_size {
            delta - 1
        } else {
            container_size - 1
        };
        self.update(result)
    }

    pub fn last(&mut self, container_size: usize) -> bool {
        if container_size < 1 {
            return false
        }

        let delta = self.counted();
        let result = if delta <= container_size {
            container_size - delta
        } else {
            0
        };
        self.update(result)
    }

    pub fn next(&mut self, container_size: usize) -> bool {
        if container_size < 1 {
            return false
        }

        if let Some(current) = self.current {
            let mut result = current + self.counted();
            if container_size <= result {
                result = container_size - 1
            }
            self.update(result)
        } else {
            false
        }
    }

    pub fn previous(&mut self) -> bool {
        if let Some(current) = self.current {
            let delta = self.counted();

            let result = if delta <= current {
                current - delta
            } else {
                0
            };
            self.update(result)
        } else {
            false
        }
    }

    pub fn counted(&mut self) -> usize {
        let result = self.count.unwrap_or(1);
        self.count = None;
        result
    }

    fn update(&mut self, new_index: usize) -> bool {
        if Some(new_index) == self.current {
            false
        } else {
            self.current = Some(new_index);
            true
        }
    }
}
