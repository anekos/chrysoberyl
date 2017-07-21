
pub struct Paginator {
    pub position: Option<usize>,
    pub fly_leaves: usize,
}

pub struct Sight {
    pub start: usize,
    pub end: usize,
}

#[derive(Debug)]
pub struct Paging {
    pub len: usize,
    pub count: usize,
    pub wrap: bool,
    pub sight_size: usize,
    pub ignore_site: bool,
}

trait Position {
    fn realize(&self, fly_leaves: usize) -> usize;
}

// pub struct Pseudo(usize);
pub struct Real(usize);


impl Paginator {
    pub fn new() -> Self {
        Paginator {
            position: None,
            fly_leaves: 0,
        }
    }

    pub fn real_sight(&self, sight_size: usize) -> Option<Sight> {
        self.real_sight_with(sight_size, 0)
    }

    pub fn real_position(&self, sight_size: usize) -> Option<usize> {
        self.real_sight(sight_size).map(|it| it.start)
    }

    pub fn real_sight_with(&self, sight_size: usize, delta: usize) -> Option<Sight> {
        if delta < self.fly_leaves {
            return None;
        }

        self.position.map(|position| {
            Sight::new_with_size(position + delta - self.fly_leaves, sight_size)
        })
    }

    pub fn update_with_real(&mut self, position: usize) -> bool {
        self.update(&Real(position), 0)
    }

    pub fn update_with_pseudo(&mut self, position: usize, fly_leaves: usize) -> bool {
        self.update(&Real(position), fly_leaves)
    }

    pub fn reset(&mut self) {
        self.position = None;
    }

    fn update<T: Position>(&mut self, position: &T, fly_leaves: usize) -> bool {
        self.fly_leaves = fly_leaves;

        let realized = position.realize(self.fly_leaves);
        if self.position == Some(realized) {
            return false;
        }

        self.position = Some(realized);
        true
    }

    pub fn force_nth(&mut self, position: usize) {
        self.position = Some(position);
    }

    pub fn first(&mut self, paging: &Paging) -> bool {
        if paging.len < 1 {
            return false;
        }

        info!("paging: {:?}", paging);

        let new_position = paging.count - 1;
        let m = paging.sighted(1);
        let m_cont = (paging.len + (m - 1)) / m;

        if new_position < m_cont {
            self.update_with_real(new_position * m)
        } else {
            self.update_with_real((m_cont - 1) * m)
        }
    }

    pub fn last(&mut self, paging: &Paging) -> bool {
        if paging.len < 1 {
            return false
        }

        let counted = paging.count - 1;
        let m = paging.sighted(1);
        let m_cont = (paging.len + (m - 1)) / m;

        if counted < m_cont {
            self.update_with_real((m_cont - counted - 1) * m)
        } else {
            self.update_with_real(0)
        }
    }

    pub fn next(&mut self, paging: &Paging) -> bool {
        if paging.len <= 1 {
            return false
        }

        if_let_some!(position = self.position, false);

        let fly_leaves = self.fly_leaves;
        let sight_size = paging.sighted(1);
        let count = paging.count;

        let m_cont = (paging.len + fly_leaves + (sight_size - 1)) / sight_size;
        let m_cur = position / sight_size;
        let m_pad = position % sight_size;

        let m_position = m_cur + count;

        let new_position = m_pad + m_position * sight_size - fly_leaves;

        if paging.len <= new_position {
            if paging.wrap {
                return self.update_with_real((m_cur + count - m_cont) % m_cont * sight_size);
            }
        } else {
            return self.update_with_real(new_position);
        }

        false
    }

    pub fn previous(&mut self, paging: &Paging) -> bool {
        if paging.len <= 1 {
            return false
        }

        if_let_some!(position = self.position, false);

        let sight_size = paging.sighted(1);
        let count = paging.count;

        let m_cont = (paging.len + (sight_size - 1)) / sight_size;
        let m_cur = position / sight_size;
        let m_pad = position % sight_size;

        if count <= m_cur {
            return self.update_with_real((m_pad + (m_cur - count) * sight_size));
        }

        let fly_leaves = paging.sighted_count() - position;
        if fly_leaves < paging.sight_size {
            return self.update_with_pseudo(0, fly_leaves);
        }

        if paging.wrap {
            let delta = (count - m_cur) % m_cont + fly_leaves;
            if delta != 0 {
                return self.update_with_real((m_cont - delta) * sight_size);
            }
        }

        false
    }

    pub fn show_page(&mut self, paging: &Paging) -> bool {
        if paging.len <= paging.count {
            return false;
        }

        if_let_some!(position = self.position, false);

        let m_pad = position % paging.sight_size;
        self.update_with_real((paging.count - m_pad) / paging.sight_size * paging.sight_size + m_pad);
        true
    }
}


impl Sight {
    pub fn new_with_size(start: usize, size: usize) -> Self {
        Sight { start: start, end: start + size - 1 }
    }
}


impl Position for Real {
    fn realize(&self, _: usize) -> usize {
        self.0
    }
}


// impl Position for Pseudo {
//     fn realize(&self, fly_leaves: usize) -> usize {
//         self.0 + fly_leaves
//     }
// }


impl Paging {
    fn sighted(&self, base: usize) -> usize {
        if self.ignore_site {
            base
        } else {
            base * self.sight_size
        }
    }

    fn sighted_count(&self) -> usize {
        if self.ignore_site {
            self.count
        } else {
            self.count * self.sight_size
        }
    }
}



#[cfg(test)]
mod test_paginator {
    use paginator::*;

    fn call<T>(mut paginator: Paginator, paging: Paging, updater: T) -> Option<usize>
    where T: FnOnce(&mut Paginator, &Paging) -> bool {
        updater(&mut paginator, &paging);
        paginator.position
    }

    #[test]
    fn test_next() {
        fn next(paginator: Paginator, paging: Paging) -> Option<usize> {
            call(paginator, paging, Paginator::next)
        }

        assert_eq!(
            next(
                Paginator { position: Some(0), fly_leaves: 0 },
                Paging { len: 2, sight_size: 1, count: 1, wrap: false, ignore_site: false }),
            Some(1));

        assert_eq!(
            next(
                Paginator { position: Some(97), fly_leaves: 0 },
                Paging { len: 195, sight_size: 2, count: 1, wrap: false, ignore_site: false }),
            Some(99));

        assert_eq!(
            next(
                Paginator { position: Some(98), fly_leaves: 0 },
                Paging { len: 195, sight_size: 2, count: 1, wrap: false, ignore_site: false }),
            Some(100));

        assert_eq!(
            next(
                Paginator { position: Some(98), fly_leaves: 0 },
                Paging { len: 195, sight_size: 3, count: 1, wrap: false, ignore_site: false }),
            Some(101));

        assert_eq!(
            next(
                Paginator { position: Some(98), fly_leaves: 0 },
                Paging { len: 195, sight_size: 3, count: 2, wrap: false, ignore_site: false }),
            Some(104));

        // Empty or Just 1

        assert_eq!(
            next(
                Paginator { position: Some(0), fly_leaves: 0 },
                Paging { len: 0, sight_size: 1, count: 1, wrap: false, ignore_site: false }),
            Some(0)); // Invalid pattern

        assert_eq!(
            next(
                Paginator { position: Some(0), fly_leaves: 0 },
                Paging { len: 1, sight_size: 1, count: 1, wrap: false, ignore_site: false }),
            Some(0));

        // // wrap

        assert_eq!(
            next(
                Paginator { position: Some(1), fly_leaves: 0 },
                Paging { len: 2, sight_size: 1, count: 1, wrap: true, ignore_site: false }),
            Some(0));

        assert_eq!(
            next(
                Paginator { position: Some(5), fly_leaves: 0 },
                Paging { len: 10, sight_size: 1, count: 5, wrap: true, ignore_site: false }),
            Some(0));

        assert_eq!(
            next(
                Paginator { position: Some(7), fly_leaves: 0 },
                Paging { len: 10, sight_size: 1, count: 5, wrap: true, ignore_site: false }),
            Some(2));

        assert_eq!(
            next(
                Paginator { position: Some(7), fly_leaves: 0 },
                Paging { len: 10, sight_size: 1, count: 15, wrap: true, ignore_site: false }),
            Some(2));

        // wrap and sight

        assert_eq!(
            next(
                Paginator { position: Some(9), fly_leaves: 0 },
                Paging { len: 11, sight_size: 3, count: 1, wrap: true, ignore_site: false }),
            Some(0));

        assert_eq!(
            next(
                Paginator { position: Some(9), fly_leaves: 0 },
                Paging { len: 11, sight_size: 4, count: 1, wrap: true, ignore_site: false }),
            Some(0));

        // fly_leaves

        assert_eq!(
            next(
                Paginator { position: Some(0), fly_leaves: 3 },
                Paging { len: 11, sight_size: 4, count: 1, wrap: true, ignore_site: false }),
            Some(1));
    }

    fn test_previous() {
        fn prev(paginator: Paginator, paging: Paging) -> Option<usize> {
            call(paginator, paging, Paginator::previous)
        }

        assert_eq!(
            prev(
                Paginator { position: Some(0), fly_leaves: 0 },
                Paging { len: 1, sight_size: 1, count: 1, wrap: false, ignore_site: false }),
            Some(0));

        assert_eq!(
            prev(
                Paginator { position: Some(1), fly_leaves: 0 },
                Paging { len: 2, sight_size: 1, count: 1, wrap: false, ignore_site: false }),
            Some(0));

        assert_eq!(
            prev(
                Paginator { position: Some(1), fly_leaves: 0 },
                Paging { len: 20, sight_size: 1, count: 1, wrap: false, ignore_site: false }),
            Some(0));

        // Empty or Just 1

        assert_eq!(
            prev(
                Paginator { position: Some(0), fly_leaves: 0 },
                Paging { len: 1, sight_size: 1, count: 1, wrap: false, ignore_site: false }),
            Some(0));

        assert_eq!(
            prev(
                Paginator { position: Some(0), fly_leaves: 0 },
                Paging { len: 1, sight_size: 1, count: 1, wrap: true, ignore_site: false }),
            Some(0));

        // wrap

        assert_eq!(
            prev(
                Paginator { position: Some(0), fly_leaves: 0 },
                Paging { len: 2, sight_size: 1, count: 1, wrap: true, ignore_site: false }),
            Some(1));

        assert_eq!(
            prev(
                Paginator { position: Some(0), fly_leaves: 0 },
                Paging { len: 2, sight_size: 1, count: 2, wrap: true, ignore_site: false }),
            Some(0));

        assert_eq!(
            prev(
                Paginator { position: Some(5), fly_leaves: 0 },
                Paging { len: 10, sight_size: 1, count: 5, wrap: true, ignore_site: false }),
            Some(0));

        assert_eq!(
            prev(
                Paginator { position: Some(5), fly_leaves: 0 },
                Paging { len: 10, sight_size: 1, count: 6, wrap: true, ignore_site: false }),
            Some(9));

        assert_eq!(
            prev(
                Paginator { position: Some(5), fly_leaves: 0 },
                Paging { len: 10, sight_size: 1, count: 8, wrap: true, ignore_site: false }),
            Some(7));

        // wrap and multiply

        assert_eq!(
            prev(
                Paginator { position: Some(0), fly_leaves: 0 },
                Paging { len: 10, sight_size: 3, count: 1, wrap: true, ignore_site: false }),
            Some(9));

        assert_eq!(
            prev(
                Paginator { position: Some(1), fly_leaves: 0 },
                Paging { len: 10, sight_size: 3, count: 2, wrap: true, ignore_site: false }),
            Some(6));

        // fly_leaves

        assert_eq!(
            prev(
                Paginator { position: Some(1), fly_leaves: 3 },
                Paging { len: 10, sight_size: 4, count: 1, wrap: false, ignore_site: false }),
            Some(0));

        // wrap and multiply and fly_leaves

        assert_eq!(
            prev(
                Paginator { position: Some(1), fly_leaves: 0 },
                Paging { len: 10, sight_size: 3, count: 1, wrap: true, ignore_site: false }),
            Some(0));

        assert_eq!(
            prev(
                Paginator { position: Some(1), fly_leaves: 0 },
                Paging { len: 10, sight_size: 3, count: 2, wrap: true, ignore_site: false }),
            Some(9));
    }

    #[test]
    fn test_show_page() {
        fn show_page(paginator: Paginator, paging: Paging) -> Option<usize> {
            call(paginator, paging, Paginator::show_page)
        }

        assert_eq!(
            show_page(
                Paginator { position: Some(0), fly_leaves: 0 },
                Paging { len: 10, sight_size: 1, count: 5, wrap: false, ignore_site: false }),
            Some(5));

        assert_eq!(
            show_page(
                Paginator { position: Some(0), fly_leaves: 0 },
                Paging { len: 10, sight_size: 3, count: 5, wrap: false, ignore_site: false }),
            Some(3));

        assert_eq!(
            show_page(
                Paginator { position: Some(0), fly_leaves: 0 },
                Paging { len: 10, sight_size: 3, count: 4, wrap: false, ignore_site: false }),
            Some(3));

        assert_eq!(
            show_page(
                Paginator { position: Some(1), fly_leaves: 0 },
                Paging { len: 10, sight_size: 3, count: 4, wrap: false, ignore_site: false }),
            Some(4));

        assert_eq!(
            show_page(
                Paginator { position: Some(1), fly_leaves: 0 },
                Paging { len: 10, sight_size: 3, count: 5, wrap: false, ignore_site: false }),
            Some(4));

        assert_eq!(
            show_page(
                Paginator { position: Some(1), fly_leaves: 0 },
                Paging { len: 10, sight_size: 4, count: 4, wrap: false, ignore_site: false }),
            Some(1));
    }
}
