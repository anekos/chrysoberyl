

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

        let position = self.counted() - 1;
        let m = self.fix(1, multiply);
        let m_cont = (container_size + (m - 1)) / m;

        if position < m_cont {
            self.update(position * m)
        } else {
            self.update((m_cont - 1) * m)
        }
    }

    pub fn last(&mut self, container_size: usize, multiply: bool) -> bool {
        if container_size < 1 {
            return false
        }

        let counted = self.counted() - 1;
        let m = self.fix(1, multiply);
        let m_cont = (container_size + (m - 1)) / m;

        if counted < m_cont {
            self.update((m_cont - counted - 1) * m)
        } else {
            self.update(0)
        }
    }

    pub fn next(&mut self, container_size: usize, multiply: bool, wrap: bool) -> bool {
        if container_size < self.multiplier {
            return false
        }

        if let Some(current) = self.current {
            if let Some(position) = calculate_next(current, container_size, self.fix(1, multiply), self.counted(), wrap) {
                return self.update(position);
            }
        }

        false
    }

    pub fn previous(&mut self, container_size: usize, multiply: bool, wrap: bool) -> bool {
        if let Some(current) = self.current {
            if let Some(position) = calculate_previous(current, container_size, self.fix(1, multiply), self.counted(), wrap) {
                return self.update(position);
            }
        }

        false
    }

    pub fn show_found(&mut self, target: usize, multiply: bool) -> bool {
        if let Some(current) = self.current {
            let multiply = self.fix(1, multiply);
            self.update(calculate_first_for(current, target, multiply))
        } else {
            self.update(target)
        }
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


fn calculate_previous(current: usize, container_size: usize, multiply: usize, counted: usize, wrap: bool) -> Option<usize> {
    if container_size <= 1 {
        return None
    }

    let m_cont = (container_size + (multiply - 1)) / multiply;
    let m_cur = current / multiply;
    let m_pad = current % multiply;

    if counted <= m_cur {
        return Some(m_pad + (m_cur - counted) * multiply);
    } else if wrap {
        let delta = (counted - m_cur) % m_cont;
        // println!("");
        // println!("current: {}, container_size: {}, multiply: {}, counted: {}, wrap: {}", current, container_size, multiply, counted, wrap);
        // println!("m_cont: {}, m_cur: {}, m_pad: {}, counted: {}, delta: {}", m_cont, m_cur, m_pad, counted, delta);
        return if delta == 0 {
            None
        } else {
            Some((m_cont - delta) * multiply)
        };
    }

    None
}

fn calculate_next(current: usize, container_size: usize, multiply: usize, counted: usize, wrap: bool) -> Option<usize> {
    if container_size <= 1 {
        return None
    }

    let m_cont = (container_size + (multiply - 1)) / multiply;
    let m_cur = current / multiply;
    let m_pad = current % multiply;

    let m_position = m_cur + counted;
    let position = m_pad + m_position * multiply;
    if container_size <= position {
        if wrap {
            return Some((m_cur + counted - m_cont) % m_cont * multiply);
        }
    } else {
        return Some(position);
    }

    None
}

fn calculate_first_for(current: usize, target: usize, multiply: usize) -> usize {
    let m_pad = current % multiply;
    target / multiply * multiply + m_pad
}



#[cfg(test)]#[test]
fn test_calculate_next() {
    // current, container_size, multiply, counted, wrap

    assert_eq!(calculate_next(0, 2, 1, 1, false), Some(1));
    assert_eq!(calculate_next(97, 195, 2, 1, false), Some(99));
    assert_eq!(calculate_next(98, 195, 2, 1, false), Some(100));
    assert_eq!(calculate_next(98, 195, 3, 1, false), Some(101));
    assert_eq!(calculate_next(98, 195, 3, 2, false), Some(104));
    // Empty or Just 1
    assert_eq!(calculate_next(0, 0, 1, 1, false), None);
    assert_eq!(calculate_next(0, 1, 1, 1, false), None);
    // wrap
    assert_eq!(calculate_next(1, 2, 1, 1, true), Some(0));
    assert_eq!(calculate_next(5, 10, 1, 5, true), Some(0));
    assert_eq!(calculate_next(7, 10, 1, 5, true), Some(2));
    assert_eq!(calculate_next(7, 10, 1, 15, true), Some(2));
    // wrap and multiply
    assert_eq!(calculate_next(9, 11, 3, 1, true), Some(0));
    assert_eq!(calculate_next(9, 11, 4, 1, true), Some(0));
}

#[cfg(test)]#[test]
fn test_calculate_previous() {
    // current, container_size, multiply, counted, wrap

    assert_eq!(calculate_previous(0, 1, 1, 1, false), None);
    assert_eq!(calculate_previous(1, 2, 1, 1, false), Some(0));
    assert_eq!(calculate_previous(1, 20, 1, 1, false), Some(0));
    // Empty or Just 1
    assert_eq!(calculate_previous(0, 0, 1, 1, false), None);
    assert_eq!(calculate_previous(0, 1, 1, 1, true), None);
    // wrap
    assert_eq!(calculate_previous(0, 2, 1, 1, true), Some(1));
    assert_eq!(calculate_previous(0, 2, 1, 2, true), None);
    assert_eq!(calculate_previous(5, 10, 1, 5, true), Some(0));
    assert_eq!(calculate_previous(5, 10, 1, 6, true), Some(9));
    assert_eq!(calculate_previous(5, 10, 1, 8, true), Some(7));
    // wrap and multiply
    assert_eq!(calculate_previous(0, 10, 3, 1, true), Some(9));
    assert_eq!(calculate_previous(1, 10, 3, 1, true), Some(9));
    assert_eq!(calculate_previous(1, 10, 3, 2, true), Some(6));
}

#[cfg(test)]#[test]
fn test_calculate_first_for() {
    // current, target, multiply

    assert_eq!(calculate_first_for(0, 5, 1), 5);
    assert_eq!(calculate_first_for(0, 5, 3), 3);
    assert_eq!(calculate_first_for(0, 4, 3), 3);
    assert_eq!(calculate_first_for(1, 4, 3), 4);
    assert_eq!(calculate_first_for(1, 5, 3), 4);
}
