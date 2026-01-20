use crate::engine::hex::Hex;
use crate::engine::parse::HexMapParseError::{InvalidHexContents, MissingLayerNumber};
use crate::engine::row_col;
use crate::engine::row_col::RowCol;
use itertools::Itertools;
use rustc_hash::FxHashMap;
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

pub fn parse_hex_map_string(s: &str) -> Result<FxHashMap<Hex, String>, HexMapParseError> {
    let mut map: FxHashMap<Hex, String> = FxHashMap::default();

    let rows = s.split("\n").map(|row| row.split_whitespace());

    let mut starting_row_num = 0;
    let first_two_lines: Vec<&str> = s
        .split("\n")
        .filter(|row| !row.is_empty() && !row.contains("Layer"))
        .take(2)
        .collect();
    if first_two_lines.len() >= 2 {
        let first_line_whitespace_count = first_two_lines[0]
            .chars()
            .find_position(|char| !char::is_whitespace(*char));
        let second_line_whitespace_count = first_two_lines[1]
            .chars()
            .find_position(|char| !char::is_whitespace(*char));
        if first_line_whitespace_count > second_line_whitespace_count {
            starting_row_num = 1;
        }
    }

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
                    row_num = starting_row_num;
                }
                "." => {
                    should_increment_row = true;
                }
                token if token.chars().count() == 1 => {
                    should_increment_row = true;
                    let hex = RowCol {
                        row: row_num,
                        col: col_num,
                        height,
                    }
                    .to_hex();
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

#[cfg(test)]
mod test {
    use super::*;
    use crate::engine::canonicalizer::canonicalize;
    use proptest::prelude::*;

    fn hex_strategy() -> impl Strategy<Value = Hex> {
        (-5..=5, -5..=5, 0..=2).prop_map(|(q, r, h)| Hex { q, r, h })
    }

    fn hex_map_strategy() -> impl Strategy<Value = FxHashMap<Hex, String>> {
        prop::collection::hash_map(hex_strategy(), r"[a-zA-Z]", 1..=42)
            .prop_map(|map| map.into_iter().collect())
    }

    #[test]
    fn parses_empty_map() {
        let map = r#"
        .  .  .
         .  .  .
        .  .  .
        "#;
        assert_eq!(FxHashMap::default(), parse_hex_map_string(map).unwrap());
    }

    #[test]
    fn parses_map() {
        let map = r#"
        Layer 0
        .  a  .
         m  Q  r
        .  .  .
        Layer 1
        .  B  .
         .  b  .
        .  .  .
        "#;

        assert_eq!(
            FxHashMap::from_iter([
                (Hex { q: 1, r: 0, h: 0 }, "a".into()),
                (Hex { q: 0, r: 1, h: 0 }, "m".into()),
                (Hex { q: 1, r: 1, h: 0 }, "Q".into()),
                (Hex { q: 2, r: 1, h: 0 }, "r".into()),
                (Hex { q: 1, r: 0, h: 1 }, "B".into()),
                (Hex { q: 1, r: 1, h: 1 }, "b".into()),
            ]),
            parse_hex_map_string(map).unwrap()
        );
    }

    #[test]
    fn indentation_order_does_not_affect_relative_hex_positions() {
        let indent_first_row_map = r#"
        Layer 0
         .  a  .
        m  Q  r
         .  .  .
        Layer 1
         .  B  .
        .  b  .
         .  .  .
        "#;
        let indent_second_row_map = r#"
        Layer 0
        .  .  .
         .  a  .
        m  Q  r
         .  .  .
        Layer 1
        .  .  .
         .  B  .
        .  b  .
         .  .  .
        "#;

        pretty_assertions::assert_str_eq!(
            hex_map_to_string(&parse_hex_map_string(indent_first_row_map).unwrap()),
            hex_map_to_string(&parse_hex_map_string(indent_second_row_map).unwrap())
        )
    }

    proptest! {
        #[test]
        fn parse_doesnt_crash(s in r"[\PC*]") {
            let _ = parse_hex_map_string(&s);
        }

        #[test]
        fn is_reversable(map in hex_map_strategy()) {
            let canonicalized_map = canonicalize(&map);
            let first_string = hex_map_to_string(&canonicalized_map);
            let second_canonicalized_map = canonicalize(&parse_hex_map_string(&first_string).unwrap());
            let second_string = hex_map_to_string(&second_canonicalized_map);
            assert_eq!(
                first_string,
                second_string
            );
        }
    }
}
