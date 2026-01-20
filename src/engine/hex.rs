use std::ops;
use strum::{EnumIter, IntoEnumIterator};

#[derive(PartialEq, Eq, Hash, Debug, Clone, Copy, Ord, PartialOrd, Default)]
pub struct Hex {
    pub q: i32,
    pub r: i32,
    pub h: i32,
}

#[repr(i32)]
#[derive(Copy, Clone, EnumIter, Debug)]
pub enum RotationDegrees {
    Sixty = 60,
    OneTwenty = 120,
    OneEighty = 180,
    TwoForty = 240,
    ThreeHundred = 300,
    ThreeSixty = 360,
}

impl RotationDegrees {
    fn as_int(&self) -> i32 {
        *self as i32
    }
}

impl Hex {
    pub fn s(&self) -> i32 {
        -self.q - self.r
    }

    pub fn base_level(&self) -> Hex {
        Hex { h: 0, ..*self }
    }

    pub fn rotated_by(&self, degrees: RotationDegrees) -> Hex {
        // To rotate 60 degrees clockwise you multiply q, r, and s by negative one and shift the coordinate
        // one to the left. Repeat the process on the result to go another 60 deg.
        let deg = degrees.as_int();
        let num_rotations = deg / 60;
        let multiplier = if num_rotations % 2 == 1 { -1 } else { 1 };
        match num_rotations % 3 {
            0 => Hex {
                q: self.q * multiplier,
                r: self.r * multiplier,
                h: self.h,
            },
            1 => Hex {
                q: self.r * multiplier,
                r: self.s() * multiplier,
                h: self.h,
            },
            2 => Hex {
                q: self.s() * multiplier,
                r: self.q * multiplier,
                h: self.h,
            },
            res => panic!(
                "A positive number mod 3 must be between 0 and 2, got {res} for {num_rotations} % 3"
            ),
        }
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
        assert_eq!(
            0,
            flat_distance(&Hex { q: 0, r: 0, h: 0 }, &Hex { q: 0, r: 0, h: 0 })
        )
    }

    #[test]
    fn test_distance_ones() {
        assert_eq!(
            1,
            flat_distance(&Hex { q: 0, r: 0, h: 0 }, &Hex { q: 0, r: 1, h: 0 })
        );
        assert_eq!(
            1,
            flat_distance(&Hex { q: 0, r: 0, h: 0 }, &Hex { q: 1, r: 0, h: 0 })
        );
        assert_eq!(
            1,
            flat_distance(&Hex { q: 0, r: 0, h: 0 }, &Hex { q: -1, r: 0, h: 0 })
        );
        assert_eq!(
            1,
            flat_distance(&Hex { q: 0, r: 0, h: 0 }, &Hex { q: 0, r: -1, h: 0 })
        );
        assert_eq!(
            1,
            flat_distance(&Hex { q: 0, r: 0, h: 0 }, &Hex { q: 1, r: -1, h: 0 })
        );
        assert_eq!(
            1,
            flat_distance(&Hex { q: 0, r: 0, h: 0 }, &Hex { q: -1, r: 1, h: 0 })
        );
    }

    #[test]
    fn test_s() {
        // Verify the following equality:
        // q + r + s == 0
        assert_eq!(-2, Hex { q: 1, r: 1, h: 0 }.s());
        assert_eq!(-1, Hex { q: 0, r: 1, h: 0 }.s());
        assert_eq!(-1, Hex { q: 1, r: 0, h: 0 }.s());
        assert_eq!(1, Hex { q: -1, r: 0, h: 0 }.s());
    }

    #[test]
    fn test_neighbor() {
        pretty_assertions::assert_eq!(
            neighbor(&Hex { q: 0, r: 0, h: 0 }, &Direction::UpLeft),
            Hex { q: 0, r: -1, h: 0 }
        )
    }
}
