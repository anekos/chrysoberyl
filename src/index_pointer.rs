

pub struct IndexPointer {
    pub current: Option<usize>,
    count: Option<usize>,
}


impl IndexPointer {
    pub fn new() -> IndexPointer {
        IndexPointer { current: None, count: None }
    }

    pub fn push_counting_number(&mut self, n: u8) {
        if let Some(current) = self.count {
            self.count = Some(current * 10 + n as usize);
        } else if n > 0 {
            self.count = Some(n as usize);
        }
    }

    pub fn first(&mut self, container_size: usize) -> Option<usize> {
        if container_size < 1 {
            return None
        }

        let delta = self.counted();
        let result = if delta <= container_size {
            delta - 1
        } else {
            container_size - 1
        };
        self.update(result)
    }

    pub fn last(&mut self, container_size: usize) -> Option<usize> {
        if container_size < 1 {
            return None
        }

        let delta = self.counted();
        let result = if delta <= container_size {
            container_size + 1 - delta
        } else {
            0
        };
        self.update(result)
    }

    pub fn next(&mut self, container_size: usize) -> Option<usize> {
        if container_size < 1 {
            return None
        }

        self.current.and_then(|current| {
            let mut result = current + self.counted();
            if container_size <= result {
                result = container_size - 1
            }
            self.update(result)
        })
    }

    pub fn previous(&mut self) -> Option<usize> {
        self.current.and_then(|current| {
            let delta = self.counted();

            let result = if delta <= current {
                current - delta
            } else {
                0
            };
            self.update(result)
        })
    }

    fn update(&mut self, new_index: usize) -> Option<usize> {
        if Some(new_index) == self.current {
            None
        } else {
            self.current = Some(new_index);
            Some(new_index)
        }
    }

    fn counted(&mut self) -> usize {
        let result = self.count.unwrap_or(1);
        self.count = None;
        result
    }
}
