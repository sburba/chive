use crate::engine::hex::Hex;
use crate::engine::parse::HexMapParseError::{InvalidHexContents, MissingLayerNumber};
use crate::engine::row_col;
use crate::engine::row_col::RowCol;
use std::num::ParseIntError;
use rustc_hash::FxHashMap;
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

pub fn parse_hex_map_string(s: &str) -> Result<FxHashMap<Hex, String>, HexMapParseError> {
    let mut map : FxHashMap<Hex, String> = FxHashMap::default();
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
                    let hex = RowCol {
                        row: row_num,
                        col: col_num,
                        height,
                    }.to_hex();
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

pub fn hex_map_to_string(hex_map: &FxHashMap<Hex, String>) -> String {
    if hex_map.is_empty() {
        return "<empty>".to_owned();
    }

    if hex_map.len() == 1 {
        return hex_map.iter().next().unwrap().1.clone();
    }

    let dimensions = row_col::dimensions(hex_map.keys());

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
                    .get(&RowCol { row, col, height }.to_hex())
                    .unwrap_or(&default);
                map_str.push_str(&format!(" {} ", token));
            }
            map_str.push('\n')
        }
    }

    map_str
}
