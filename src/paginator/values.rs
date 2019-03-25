use std::ops;



#[derive(PartialEq, Debug, Clone, Copy)]
pub struct Level(pub usize);

#[derive(PartialEq, Debug, Clone, Copy)]
pub struct Index(pub usize);

#[derive(PartialEq, Debug, Clone, Copy)]
pub struct Position(pub usize);

#[derive(PartialEq, Debug, Clone, Copy)]
pub struct SightSize(pub usize);

#[derive(PartialEq, Debug, Clone, Copy)]
pub struct FlyLeaves(pub usize);

#[derive(PartialEq, Debug, Clone, Copy)]
pub struct PseudoLen(pub usize);



impl Index {
    pub fn with_fly_leaves(self, fly_leaves: FlyLeaves) -> Position {
        Position(fly_leaves.0 + self.0)
    }

    #[cfg(test)]
    pub fn with_sight_size(&self, sight_size: SightSize) -> FlyLeaves {
        FlyLeaves(rev_mod(self.0, sight_size.0))
    }
}


impl Level {
    pub fn to_position(self, sight_size: SightSize) -> Position {
        Position(self.0 * sight_size.0)
    }
}

impl Default for Level {
    fn default() -> Self {
        Level(0)
    }
}

impl ops::Add<usize> for Level {
    type Output = Self;

    fn add(self, rhs: usize) -> Level {
        Level(self.0 + rhs)
    }
}

impl ops::Add<isize> for Level {
    type Output = Self;

    fn add(self, rhs: isize) -> Level {
        Level((self.0 as isize + rhs) as usize)
    }
}

impl ops::Sub<isize> for Level {
    type Output = Self;

    fn sub(self, rhs: isize) -> Level {
        Level((self.0 as isize - rhs) as usize)
    }
}

impl ops::Sub<usize> for Level {
    type Output = Self;

    fn sub(self, rhs: usize) -> Level {
        Level(self.0 - rhs)
    }
}


impl Position {
    pub fn checked_add(self, rhs: isize) -> Option<Self> {
        ((self.0 as isize).checked_add(rhs)).map(|it| Position(it as usize))
    }

    pub fn to_index(self, fly_leaves: FlyLeaves) -> Option<Index> {
        if self.0 < fly_leaves.0 {
            None
        } else {
            Some(Index(self.0 - fly_leaves.0))
        }
    }

    pub fn to_level(self, sight_size: SightSize) -> Level {
        Level(self.0 / sight_size.0)
    }
}

impl ops::Add<usize> for Position {
    type Output = Self;

    fn add(self, rhs: usize) -> Self {
        Position(self.0 + rhs)
    }
}


#[cfg(test)]
fn rev_mod(lhs: usize, rhs: usize) -> usize {
    (rhs - lhs % rhs) % rhs
}
