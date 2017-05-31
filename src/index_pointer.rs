

#[derive(Clone)]
pub struct IndexPointer {
    pub current: Option<usize>,
    count: Option<usize>,
    multiplier: usize,
}

pub struct Save {
    count: Option<usize>
}


impl IndexPointer {
    pub fn new() -> IndexPointer {
        IndexPointer { current: None, count: None, multiplier: 1 }
    }

    pub fn new_with_index(index: usize) -> IndexPointer {
        IndexPointer { current: Some(index), count: None, multiplier: 1 }
    }

    pub fn save(&self) -> Save {
        Save { count: self.count }
    }

    pub fn restore(&mut self, save: &Save) {
        self.count = save.count;
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

    pub fn next(&mut self, container_size: usize, multiply: bool, wrap: bool) -> bool {
        if container_size < self.multiplier {
            return false
        }

        if let Some(current) = self.current {
            let counted = self.counted();
            let m = self.fix(1, multiply);
            let m_cont = (container_size + (m - 1)) / m;
            let m_cur = current / m;

            let position = m_cur + counted;
            if m_cont <= position {
                if wrap {
                    return self.update((position - m_cont) % m_cont * m);
                }
            } else {
                return self.update(position * m);
            }
        }

        false
    }

    pub fn previous(&mut self, container_size: usize, multiply: bool, wrap: bool) -> bool {
        if let Some(current) = self.current {
            let counted = self.counted();
            let m = self.fix(1, multiply);
            let m_cont = (container_size + (m - 1)) / m;
            let m_cur = current / m;

            if counted <= m_cur {
                if wrap {
                    return self.update((m_cur - counted) * m);
                }
            } else {
                return self.update((m_cont - (counted - m_cur) % m_cont) * m);
            }
        }

        false
    }

    pub fn counted(&mut self) -> usize {
        let result = self.count.unwrap_or(1);
        self.count = None;
        result
    }

    fn fix(&self, x: usize, multiply: bool) -> usize {
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
