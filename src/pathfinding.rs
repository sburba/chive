use crate::hex;
use crate::hex::{is_adjacent, Hex};
use crate::hive::Hive;
use crate::pathfinding::PathfindingError::HexNotPopulated;
use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashSet};
use thiserror::Error;

#[derive(Eq, PartialEq, Hash, Debug, Clone, Copy)]
struct PathLocation {
    hex: Hex,
    priority: i32,
}

impl Ord for PathLocation {
    fn cmp(&self, other: &Self) -> Ordering {
        if self.priority < other.priority {
            Ordering::Greater
        } else if self.priority > other.priority {
            Ordering::Less
        } else {
            Ordering::Equal
        }
    }
}

/// Inverted order based on priority so that BinaryHeap is a MinHeap instead of a MaxHeap
impl PartialOrd for PathLocation {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

pub fn move_would_break_hive(hive: &Hive, from: &Hex, to: &Hex) -> bool {
    let mut connected_pieces = HashSet::new();

    // You can't break the hive by moving from any layer but the bottom layer
    if from.h != 0 {
        return false;
    }

    //TODO: I don't think this logic should be here, it's too specific to each type of movement
    let is_slide = from.h == 0 && to.h == 0 && is_adjacent(from, to);
    if is_slide && !hive.occupied_neighbors_at_same_level(from).any(|neighbor| is_adjacent(&neighbor, to)) {
        return true;
    }

    for hex in hive.occupied_neighbors_at_same_level(from) {
        if move_would_disconnect_piece(hive, from, to, &hex, &mut connected_pieces).unwrap() {
            return true;
        }
    }

    false
}

#[derive(Error, Debug)]
pub enum PathfindingError {
    #[error("Affected hex {hex:?} must contain a tile")]
    HexNotPopulated { hex: Hex },
}
fn move_would_disconnect_piece(
    hive: &Hive,
    from: &Hex,
    to: &Hex,
    affected_piece: &Hex,
    already_connected_pieces: &mut HashSet<Hex>,
) -> Result<bool, PathfindingError> {
    if !hive.map.contains_key(affected_piece) {
        return Err(HexNotPopulated {
            hex: *affected_piece,
        });
    }

    let hex_to_avoid = if from.h == 0 { Some(*from) } else { None };
    let end = Hex {h: 0, ..*to};

    let mut frontier = BinaryHeap::new();
    let start_location = PathLocation {
        hex: *affected_piece,
        priority: 0,
    };
    frontier.push(start_location);
    let mut hexes_seen = HashSet::new();
    hexes_seen.insert(*affected_piece);

    while !frontier.is_empty() {
        let current = frontier.pop().unwrap();

        if current.hex == end
            || is_adjacent(&current.hex, &end)
            || already_connected_pieces.contains(&current.hex)
        {
            already_connected_pieces.extend(hexes_seen);
            return Ok(false);
        }

        for next in hive.occupied_neighbors_at_same_level(&current.hex) {
            if Some(next) == hex_to_avoid {
                continue;
            }
            if !hexes_seen.contains(&next) {
                hexes_seen.insert(next);
                frontier.push(PathLocation {
                    hex: next,
                    priority: hex::flat_distance(&next, &end),
                })
            }
        }
    }
    Ok(true)
}
