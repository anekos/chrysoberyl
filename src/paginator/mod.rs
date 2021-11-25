
pub mod values;
#[cfg(test)] mod test;

use self::values::*;



#[derive(Debug, PartialEq)]
pub struct Paginator {
    fly_leaves: FlyLeaves,
    len: usize,
    level: Option<Level>, /* NOT index */
    sight_size: SightSize,
}

#[derive(Debug, PartialEq)]
pub struct Paging {
    pub count: usize,
    pub ignore_sight: bool,
    pub wrap: bool,
}

#[derive(Debug, PartialEq)]
pub struct Condition {
    pub len: usize,
    pub sight_size: usize,
}


impl Paginator {
    pub fn new() -> Self {
        Paginator {
            level: None,
            fly_leaves: FlyLeaves(0),
            len: 0,
            sight_size: SightSize(1)
        }
    }

    pub fn pseudo_len(&self) -> PseudoLen {
        PseudoLen(self.len + self.fly_leaves.0)
    }

    pub fn fly_leaves(&self) -> usize {
        self.fly_leaves.0
    }

    pub fn position(&self) -> Option<Position> {
        self.level.map(|level| level.to_position(self.sight_size))
    }

    fn backs(&self) -> usize {
        self.pseudo_len().0 - self.sight_size.0 * self.level.unwrap_or_default().0 - 1
    }

    /* the nubmer of levels */
    fn levels(&self) -> usize {
        if self.len == 0 {
            0
        } else if self.sight_size.0 <= 1 {
            self.pseudo_len().0
        } else {
            (self.pseudo_len().0 - 1) / self.sight_size.0 + 1
        }
    }

    pub fn reset_level(&mut self) -> bool {
        if 0 < self.levels() {
            self.update_level(Level(0))
        } else {
            false
        }
    }

    pub fn current_index(&self) -> Option<usize> {
        self.current_index_with(0)
    }

    pub fn current_index_with(&self, delta: isize) -> Option<usize> {
        let new_index = self.position()
            .and_then(|position| position.checked_add(delta))
            .and_then(|position| position.to_index(self.fly_leaves));

        if let Some(index) = new_index {
            if index.0 < self.len {
                return Some(index.0);
            }
        }
        None
    }

    pub fn at_first(&self) -> bool {
        self.level.map(|it| it.0 == 0).unwrap_or(false)
    }

    pub fn at_last(&self) -> bool {
        let levels = self.levels();
        0 < levels && self.level.map(|it| it.0 == levels - 1).unwrap_or(false)
    }

    pub fn reset(&mut self) {
        self.fly_leaves = FlyLeaves(0);
        self.level = None;
    }

    pub fn update_condition(&mut self, condition: &Condition) {
        self.len = condition.len;
        self.sight_size = SightSize(condition.sight_size);
        self.fly_leaves = FlyLeaves(min!(self.sight_size.0 - 1, self.fly_leaves.0));
    }

    pub fn first(&mut self, paging: &Paging) -> bool {
        if self.len == 0 {
            return false;
        }

        let count = paging.count;
        let levels = self.levels();
        let sight_size = self.sight_size;

        if paging.ignore_sight {
            let old_fly_leaves = self.fly_leaves;
            let new_index = Index(min!(count, self.len) - 1);
            self.fly_leaves = FlyLeaves((sight_size.0 - (new_index.0 % sight_size.0)) % sight_size.0);
            let new_level = new_index.with_fly_leaves(self.fly_leaves).to_level(self.sight_size);
            return self.update_level(new_level) || old_fly_leaves != self.fly_leaves;
        }

        let new_level = if count <= levels {
             count - 1
        } else if paging.wrap {
            (count - 1) % levels
        } else {
             levels - 1
        };

        self.update_level(Level(new_level))
    }

    pub fn last(&mut self, paging: &Paging) -> bool {
        if self.len == 0 {
            return false;
        }

        let count = paging.count;
        let levels = self.levels();
        let sight_size = self.sight_size;

        if paging.ignore_sight {
            let old_fly_leaves = self.fly_leaves;
            let new_index = Index(self.len - min!(self.len, count));
            self.fly_leaves = FlyLeaves((sight_size.0 - (new_index.0 % sight_size.0)) % sight_size.0);
            let new_level = new_index.with_fly_leaves(self.fly_leaves).to_level(self.sight_size);
            return self.update_level(new_level) || old_fly_leaves != self.fly_leaves;
        }

        let new_level = if count <= levels {
             levels - count
        } else if paging.wrap {
            levels - (count - 1) % levels - 1
        } else {
             0
        };

        self.update_level(Level(new_level))
    }

    pub fn next(&mut self, paging: &Paging) -> bool {
        if self.len == 0 {
            return false;
        }

        if !paging.ignore_sight {
            return self.increase_level(paging.count, paging.wrap);
        }

        let delta = min!(paging.count, self.backs());

        let level = delta / self.sight_size.0;
        let fly_leaves = delta % self.sight_size.0;

        let level_updated = self.increase_level(level, paging.wrap);
        let fly_leaves_updated = self.decrease_fly_leaves(fly_leaves);

        level_updated || fly_leaves_updated
    }

    pub fn previous(&mut self, paging: &Paging) -> bool {
        if self.len == 0 {
            return false;
        }

        if !paging.ignore_sight {
            return self.decrease_level(paging.count, paging.wrap);
        }

        let delta = min!(
            paging.count,
            (self.sight_size.0 - self.fly_leaves.0 - 1) + self.current_index().unwrap_or_default());
        let level = delta / self.sight_size.0;
        let fly_leaves = delta % self.sight_size.0;

        let level_updated = self.decrease_level(level, paging.wrap);
        let fly_leaves_updated = self.increase_fly_leaves(fly_leaves);

        level_updated || fly_leaves_updated
    }

    pub fn show(&mut self, paging: &Paging) -> bool {
        self.update_index(Index(paging.count - 1))
    }

    pub fn set_fly_leaves(&mut self, n: usize) -> bool {
        use std::cmp::Ordering::*;

        let n = n % self.sight_size.0;

        match self.fly_leaves.0.cmp(&n) {
            Less => {
                let d = n - self.fly_leaves.0;
                self.increase_fly_leaves(d)
            },
            Equal => false,
            Greater => {
                let d = self.fly_leaves.0 - n;
                self.decrease_fly_leaves(d)
            }
        }
    }

    fn increase_level(&mut self, delta: usize, wrap: bool) -> bool {
        let current = self.level.unwrap_or_default().0;
        let levels = self.levels();

        let new_level = if wrap || current + delta < levels {
            (current + delta) % levels
        } else {
            levels - 1
        };

        self.update_level(Level(new_level))
    }

    fn decrease_level(&mut self, delta: usize, wrap: bool) -> bool {
        let current = self.level.unwrap_or_default().0;
        let levels = self.levels();

        let new_level = if delta <= current {
            current - delta
        } else if wrap {
            let delta = (delta - current) % levels;
            (levels - delta) % levels
        } else {
            0
        };

        self.update_level(Level(new_level))
    }

    fn update_level(&mut self, new_level: Level) -> bool {
        let new_level = Some(new_level);
        let updated = new_level != self.level;
        self.level = new_level;
        updated
    }

    fn increase_fly_leaves(&mut self, delta: usize) -> bool {
        let old_fly_leaves = self.fly_leaves.0;
        let new_fly_leaves = (self.fly_leaves.0 + delta) % self.sight_size.0;

        if old_fly_leaves == new_fly_leaves {
            return false;
        }

        self.fly_leaves = FlyLeaves(new_fly_leaves);

        if new_fly_leaves == 0 {
            self.decrease_level(1, false);
        }

        true
    }

    fn decrease_fly_leaves(&mut self, delta: usize) -> bool {
        let delta = delta % self.sight_size.0;

        let old_fly_leaves = self.fly_leaves.0;
        let new_fly_leaves = if delta <= self.fly_leaves.0 {
            self.fly_leaves.0 - delta
        } else {
            self.sight_size.0 - (delta - self.fly_leaves.0)
        };

        if new_fly_leaves == old_fly_leaves {
            return false;
        }

        self.fly_leaves = FlyLeaves(new_fly_leaves);

        if old_fly_leaves < new_fly_leaves && (old_fly_leaves == 0 || (new_fly_leaves - old_fly_leaves) < old_fly_leaves) {
            self.increase_level(1, false);
        }

        true
    }

    pub fn update_index(&mut self, index: Index) -> bool {
        if self.len == 0 {
            return false;
        }

        let new_level = index.with_fly_leaves(self.fly_leaves).to_level(self.sight_size);
        let levels = self.levels();
        self.update_level(Level(min!(levels - 1, new_level.0)))
    }
}
