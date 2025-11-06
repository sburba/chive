use std::cmp::{max, min};
use crate::engine::hex::Hex;

#[derive(Debug, Ord, PartialOrd, Eq, PartialEq, Default, Copy, Clone)]
pub struct RowCol {
    pub row: i32,
    pub col: i32,
    pub height: i32,
}

impl RowCol {
    pub fn to_hex(&self) -> Hex {
        let parity = self.row & 1;
        let q = self.col - (self.row - parity) / 2;
        let r = self.row;
        Hex {
            q,
            r,
            h: self.height,
        }
    }

    pub fn from_hex(hex: &Hex) -> RowCol {
        let parity = hex.r & 1;
        let col = hex.q + (hex.r - parity) / 2;
        let row = hex.r;

        RowCol {
            col,
            row,
            height: hex.h,
        }
    }
}

impl From<&Hex> for RowCol {
    fn from(value: &Hex) -> Self {
        RowCol::from_hex(value)
    }
}

impl Into<Hex> for RowCol {
    fn into(self) -> Hex {
        self.to_hex()
    }
}

#[derive(Default)]
pub struct RowColDimensions {
    pub row_min: i32,
    pub row_max: i32,
    pub col_min: i32,
    pub col_max: i32,
    pub height_min: i32,
    pub height_max: i32,
}

impl RowColDimensions {
    pub fn width(&self) -> i32 {
        self.col_max - self.col_min + 1
    }

    pub fn height(&self) -> i32 {
        self.row_max - self.row_min + 1
    }
}

pub fn dimensions<'a>(hexes: impl Iterator<Item=&'a Hex>) -> RowColDimensions {
    hexes.fold(Default::default(), |dims: RowColDimensions, hex| {
        let oddr = RowCol::from_hex(hex);
        RowColDimensions {
            row_min: min(dims.row_min, oddr.row),
            row_max: max(dims.row_max, oddr.row),
            col_min: min(dims.col_min, oddr.col),
            col_max: max(dims.col_max, oddr.col),
            height_min: min(dims.height_min, oddr.height),
            height_max: max(dims.height_max, oddr.height),
        }
    })
}