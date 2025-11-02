use std::ops;
use strum::{EnumIter, IntoEnumIterator};

#[derive(PartialEq, Eq, Hash, Debug, Clone, Copy, Ord, PartialOrd)]
pub struct Hex {
    pub q: i32,
    pub r: i32,
    pub h: i32,
}

impl Hex {
    pub fn s(&self) -> i32 {
        self.q + self.r
    }
}

impl ops::Add<Hex> for Hex {
    type Output = Hex;

    fn add(self, rhs: Hex) -> Self::Output {
        Hex {
            q: self.q + rhs.q,
            r: self.r + rhs.r,
            h: self.h + rhs.h,
        }
    }
}

impl ops::Sub<Hex> for Hex {
    type Output = Hex;
    fn sub(self, rhs: Hex) -> Self::Output {
        Hex {
            q: self.q - rhs.q,
            r: self.r - rhs.r,
            h: self.h - rhs.h,
        }
    }
}

impl ops::Add<&Hex> for &Hex {
    type Output = Hex;

    fn add(self, rhs: &Hex) -> Self::Output {
        *self + *rhs
    }
}

impl ops::Sub<&Hex> for &Hex {
    type Output = Hex;

    fn sub(self, rhs: &Hex) -> Self::Output {
        *self - *rhs
    }
}

impl Direction {
    pub fn vector(&self) -> Hex {
        match *self {
            Direction::UpLeft => Hex { q: 0, r: -1, h: 0 },
            Direction::UpRight => Hex { q: 1, r: -1, h: 0 },
            Direction::Right => Hex { q: 1, r: 0, h: 0 },
            Direction::DownRight => Hex { q: 0, r: 1, h: 0 },
            Direction::DownLeft => Hex { q: -1, r: 1, h: 0 },
            Direction::Left => Hex { q: -1, r: 0, h: 0 },
        }
    }
}

/// Calculate the straight line distance between two hexes ignoring height
pub fn flat_distance(lhs: &Hex, rhs: &Hex) -> i32 {
    let vec = lhs - rhs;
    (vec.q.abs() + vec.r.abs() + vec.s().abs()) / 2
}

pub fn neighbors(hex: &Hex) -> impl Iterator<Item = Hex> {
    Direction::iter().map(|d| neighbor(hex, &d))
}

pub fn neighbor(hex: &Hex, direction: &Direction) -> Hex {
    hex + &direction.vector()
}

pub fn is_adjacent(lhs: &Hex, rhs: &Hex) -> bool {
    flat_distance(lhs, rhs) == 1
}

//THIS HAS TO GO IN A CIRCLE
#[derive(PartialEq, Eq, Hash, Debug, EnumIter, Clone, Copy)]
pub enum Direction {
    UpLeft,
    UpRight,
    Right,
    DownRight,
    DownLeft,
    Left,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_distance_identity() {
        assert_eq!(0, flat_distance(&Hex{q: 0, r: 0, h: 0}, &Hex{q: 0, r: 0, h: 0}))
    }

    #[test]
    fn test_distance_ones() {
        assert_eq!(1, flat_distance(&Hex{q: 0, r: 0, h: 0}, &Hex{q: 0, r: 1, h: 0}));
        assert_eq!(1, flat_distance(&Hex{q: 0, r: 0, h: 0}, &Hex{q: 1, r: 0, h: 0}));
        assert_eq!(1, flat_distance(&Hex{q: 0, r: 0, h: 0}, &Hex{q: -1, r: 0, h: 0}));
        assert_eq!(1, flat_distance(&Hex{q: 0, r: 0, h: 0}, &Hex{q: 0, r: -1, h: 0}));
        assert_eq!(1, flat_distance(&Hex{q: 0, r: 0, h: 0}, &Hex{q: 1, r: -1, h: 0}));
        assert_eq!(1, flat_distance(&Hex{q: 0, r: 0, h: 0}, &Hex{q: -1, r: 1, h: 0}));
    }
}