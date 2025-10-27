use crate::bug::{Bug, BugParseError};
use crate::hex::{Hex, neighbors};
use crate::parse::{HexMapParseError, hex_map_to_string, parse_hex_map_string};
use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::str::FromStr;
use thiserror::Error;

#[derive(Debug, Clone, Eq, PartialEq, Copy, Ord, PartialOrd, Hash)]
pub enum Color {
    Black,
    White,
}

impl Display for Color {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Color::Black => {
                write!(f, "B")
            }
            Color::White => {
                write!(f, "W")
            }
        }
    }
}

impl Color {
    pub fn opposite(&self) -> Color {
        match self {
            Color::Black => Color::White,
            Color::White => Color::Black,
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Copy, Ord, PartialOrd, Hash)]
pub struct Tile {
    pub bug: Bug,
    pub color: Color,
}

impl Display for Tile {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if self.color == Color::White {
            write!(f, "{}", self.bug.to_string().to_uppercase())
        } else {
            write!(f, "{}", self.bug.to_string().to_lowercase())
        }
    }
}

#[derive(Debug, Clone)]
pub struct Hive {
    pub map: HashMap<Hex, Tile>,
}

impl Hive {
    pub fn from_hex_map(hex_map: &HashMap<Hex, String>) -> Result<Hive, HiveParseError> {
        let mut map: HashMap<Hex, Tile> = HashMap::new();
        for (hex, token) in hex_map {
            if token == "." {
                continue;
            }

            let bug = token.to_uppercase().parse()?;
            let token_char = token.chars().next().unwrap();
            let color = if token_char.is_uppercase() {
                Color::White
            } else {
                Color::Black
            };
            map.insert(*hex, Tile { bug, color });
        }
        Ok(Hive { map })
    }

    pub fn to_hex_map(&self) -> HashMap<Hex, String> {
        self.map
            .iter()
            .map(|(hex, tile)| (*hex, tile.to_string()))
            .collect()
    }

    pub fn stack_height(&self, hex: &Hex) -> i32 {
        let mut height = 0;
        while self.map.contains_key(&Hex { h: height, ..*hex }) {
            height += 1;
        }
        height
    }

    pub fn topmost_occupied_hex(&self, hex: &Hex) -> Option<Hex> {
        let stack_height = self.stack_height(hex);
        if stack_height > 0 {
            Some(Hex {
                h: stack_height - 1,
                ..*hex
            })
        } else {
            None
        }
    }

    pub fn neighbors_at_same_level(&self, hex: &Hex) -> impl Iterator<Item = Hex> {
        neighbors(hex)
    }

    pub fn occupied_neighbors_at_same_level(&self, hex: &Hex) -> impl Iterator<Item = Hex> {
        neighbors(hex).filter(|h| self.map.contains_key(h))
    }

    pub fn topmost_occupied_neighbors(&self, hex: &Hex) -> impl Iterator<Item = Hex> {
        self.neighbors_at_same_level(hex)
            .filter_map(|hex| self.topmost_occupied_hex(&hex))
    }

    pub fn unoccupied_neighbors(&self, hex: &Hex) -> impl Iterator<Item = Hex> {
        neighbors(hex).filter(|neighbor| !self.map.contains_key(neighbor))
    }

    pub fn is_occupied(&self, hex: &Hex) -> bool {
        self.map.contains_key(hex)
    }

    pub fn next_unoccupied_spot_in_direction(&self, hex: &Hex, direction: &Hex) -> Hex {
        let mut current: Hex = *hex;
        while self.map.contains_key(&current) {
            current = current + *direction;
        }
        current
    }
}

impl Display for Hive {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let hex_map: HashMap<Hex, String> =
            self.map.iter().map(|(h, t)| (*h, t.to_string())).collect();
        write!(f, "{}", hex_map_to_string(&hex_map))
    }
}

#[derive(Error, Debug)]
pub enum HiveParseError {
    #[error("Invalid Hex Map")]
    InvalidMap(#[from] HexMapParseError),
    #[error("Invalid bug type")]
    InvalidBugType(#[from] BugParseError),
}

impl FromStr for Hive {
    type Err = HiveParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let hex_map = parse_hex_map_string(s)?;
        Hive::from_hex_map(&hex_map)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn test_parse_map() {
        let map_str = r#"
    Layer 0
      G  .  .
     .  B  a
      a  q  .

    Layer 1
      .  .  .
     .  .  .
      b  .  .
    "#;

        let hive: Hive = map_str.parse().unwrap();

        assert_eq!(
            normalize_whitespace(&hive.to_string()),
            normalize_whitespace(map_str)
        )
    }

    fn normalize_whitespace(s: &str) -> String {
        s.trim()
            .lines()
            .map(|line| line.split_whitespace().collect::<Vec<_>>().join(" "))
            .collect::<Vec<_>>()
            .join("\n")
    }
}
