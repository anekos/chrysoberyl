

pub struct IndexPointer {
    pub current: Option<usize>,
    count: Option<usize>,
    multiplier: usize,
}


impl IndexPointer {
    pub fn new() -> IndexPointer {
        IndexPointer { current: None, count: None, multiplier: 1 }
    }

    pub fn current_with(&self, delta: usize) -> Option<usize> {
        self.current.map(|current| current + delta)
    }

    pub fn set_count(&mut self, count: Option<usize>) {
        self.count = count;
    }

    /**
     * If `count` is not None, overwrite `self.count`
     */
    pub fn with_count(&mut self, count: Option<usize>) -> &mut IndexPointer {
        if count.is_some() {
            self.count = count;
        }
        self
    }

    pub fn set_multiplier(&mut self, x: usize) {
        if x == 0 {
            panic!("Invalid multiplier: {}", x);
        }
        self.multiplier = x;
    }

    pub fn push_count_digit(&mut self, n: u8) {
        if let Some(current) = self.count {
            self.set_count(Some(current * 10 + n as usize));
        } else if n > 0 {
            self.set_count(Some(n as usize));
        }
    }

    pub fn first(&mut self, container_size: usize, multiply: bool) -> bool {
        if container_size < 1 {
            return false
        }

        let counted = self.counted();
        let delta = self.fix(counted - 1, multiply);
        let result = if delta < container_size {
            delta
        } else {
            container_size - 1
        };
        self.update(result)
    }

    pub fn last(&mut self, container_size: usize, multiply: bool) -> bool {
        if container_size < 1 {
            return false
        }

        let counted = self.counted();
        let delta = self.fix(counted, multiply);
        let result = if delta <= container_size {
            container_size - delta
        } else {
            0
        };
        self.update(result)
    }

    pub fn next(&mut self, container_size: usize, multiply: bool) -> bool {
        if container_size < self.multiplier {
            return false
        }

        if let Some(current) = self.current {
            let counted = self.counted();
            let mut result = current + self.fix(counted, multiply);
            if container_size <= result {
                result = if multiply { 
                    let d = current % self.multiplier;
                    let result = (container_size - d) / self.multiplier * self.multiplier + d;
                    if container_size <= result { container_size - self.multiplier } else { result }
                } else { container_size - 1 }
            }
            self.update(result)
        } else {
            false
        }
    }

    pub fn previous(&mut self, multiply: bool) -> bool {
        if let Some(current) = self.current {
            let counted = self.counted();
            let delta = self.fix(counted, multiply);

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

    pub fn fix(&self, x: usize, multiply: bool) -> usize {
        if multiply {
            x * self.multiplier
        } else {
            x
        }
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
