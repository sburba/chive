use crate::hex::Hex;
use crate::parse::HexMapParseError::{InvalidHexContents, MissingLayerNumber};
use std::cmp::{max, min};
use std::collections::HashMap;
use std::num::ParseIntError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum HexMapParseError {
    #[error("Invalid Layer Number")]
    InvalidLayerNumber(#[from] ParseIntError),
    #[error("Got Layer without a corresponding number")]
    MissingLayerNumber,
    #[error("Hex contents can only be a single character, got: {contents}")]
    InvalidHexContents { contents: String },
}

#[derive(Debug, Ord, PartialOrd, Eq, PartialEq)]
struct OddrCoordinate {
    row: i32,
    col: i32,
    height: i32,
}

fn hex_to_oddr(hex: &Hex) -> OddrCoordinate {
    let parity = hex.r & 1;
    let col = hex.q + (hex.r - parity) / 2;
    let row = hex.r;

    OddrCoordinate {
        col,
        row,
        height: hex.h,
    }
}

fn oddr_to_hex(oddr: &OddrCoordinate) -> Hex {
    let parity = oddr.row & 1;
    let q = oddr.col - (oddr.row - parity) / 2;
    let r = oddr.row;
    Hex {
        q,
        r,
        h: oddr.height,
    }
}

pub fn parse_hex_map_string(s: &str) -> Result<HashMap<Hex, String>, HexMapParseError> {
    let mut map = HashMap::new();
    let rows = s.split("\n").map(|row| row.split_whitespace());
    let mut height = 0;
    let mut row_num = 0;
    for row in rows {
        let mut token_iter = row.into_iter();
        let mut col_num = 0;
        let mut should_increment_row = false;
        while let Some(token) = token_iter.next() {
            match token {
                "Layer" => {
                    height = token_iter.next().ok_or(MissingLayerNumber)?.parse()?;
                    row_num = 0;
                }
                "." => {
                    should_increment_row = true;
                }
                token if token.len() == 1 => {
                    should_increment_row = true;
                    let hex = oddr_to_hex(&OddrCoordinate {
                        row: row_num,
                        col: col_num,
                        height,
                    });
                    map.insert(hex, token.to_string());
                }
                contents => {
                    return Err(InvalidHexContents {
                        contents: contents.to_string(),
                    });
                }
            }
            col_num += 1;
        }
        if should_increment_row {
            row_num += 1;
        }
    }
    Ok(map)
}

#[derive(Default)]
struct HexMapDimensions {
    row_min: i32,
    row_max: i32,
    col_min: i32,
    col_max: i32,
    height_min: i32,
    height_max: i32,
}

pub fn hex_map_to_string(hex_map: &HashMap<Hex, String>) -> String {
    if hex_map.is_empty() {
        return "<empty>".to_owned();
    }

    if hex_map.len() == 1 {
        return hex_map.iter().next().unwrap().1.clone();
    }

    let dimensions = hex_map
        .iter()
        .fold(Default::default(), |dims: HexMapDimensions, (hex, _)| {
            let oddr = hex_to_oddr(hex);
            HexMapDimensions {
                row_min: min(dims.row_min, oddr.row),
                row_max: max(dims.row_max, oddr.row),
                col_min: min(dims.col_min, oddr.col),
                col_max: max(dims.col_max, oddr.col),
                height_min: min(dims.height_min, oddr.height),
                height_max: max(dims.height_max, oddr.height),
            }
        });

    let mut map_str = String::new();
    for height in dimensions.height_min..=dimensions.height_max {
        if dimensions.height_max != 0 {
            map_str.push_str(&format!("\nLayer {height}\n"));
        }
        for row in dimensions.row_min..=dimensions.row_max {
            // Indent every odd row. Use binary and instead of mod so that it works for negative
            // numbers. For the purpose of this function, zero is even.
            if row & 1 == 1 {
                map_str.push(' ')
            }
            for col in dimensions.col_min..=dimensions.col_max {
                let default = ".".to_string();
                let token = hex_map
                    .get(&oddr_to_hex(&OddrCoordinate { row, col, height }))
                    .unwrap_or(&default);
                map_str.push_str(&format!(" {} ", token));
            }
            map_str.push('\n')
        }
    }

    map_str
}
