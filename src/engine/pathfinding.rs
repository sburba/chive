use crate::engine::hex;
use crate::engine::hex::{is_adjacent, Hex};
use crate::engine::hive::Hive;
use crate::engine::pathfinding::PathfindingError::HexNotPopulated;
use rustc_hash::FxHashSet;
use std::cmp::Ordering;
use std::collections::BinaryHeap;
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
    // You can't break the hive by moving from any layer but the bottom layer
    if from.h != 0 {
        return false;
    }

    //TODO: I don't think this logic should be here, it's too specific to each type of movement
    let is_slide = from.h == 0 && to.h == 0 && is_adjacent(from, to);
    if is_slide
        && !hive
            .occupied_neighbors_at_same_level(from)
            .any(|neighbor| is_adjacent(&neighbor, to))
    {
        return true;
    }

    let mut connected_pieces = FxHashSet::default();
    let mut neighbors = hive.occupied_neighbors_at_same_level(from);
    let first = neighbors.next().unwrap();

    

    neighbors.any(|neighbor| {
        !pieces_are_connected(hive, &first, &neighbor, from, &mut connected_pieces).unwrap()
    })
}

#[derive(Error, Debug)]
pub enum PathfindingError {
    #[error("Affected hex {hex:?} must contain a tile")]
    HexNotPopulated { hex: Hex },
}

fn pieces_are_connected(
    hive: &Hive,
    left: &Hex,
    right: &Hex,
    hex_to_avoid: &Hex,
    already_connected_pieces: &mut FxHashSet<Hex>,
) -> Result<bool, PathfindingError> {
    let left_hex_populated = hive.map.contains_key(left);
    let right_hex_populated = hive.map.contains_key(right);
    if !left_hex_populated || !right_hex_populated {
        return Err(HexNotPopulated {
            hex: if !left_hex_populated { *left } else { *right },
        });
    }

    let start = left;
    let end = Hex { h: 0, ..*right };

    let mut frontier = BinaryHeap::new();
    let start_location = PathLocation {
        hex: *start,
        priority: 0,
    };

    frontier.push(start_location);
    let mut hexes_seen = FxHashSet::default();
    hexes_seen.insert(*start);

    while !frontier.is_empty() {
        let current = frontier.pop().unwrap();

        if current.hex == end
            || is_adjacent(&current.hex, &end)
            || already_connected_pieces.contains(&current.hex)
        {
            already_connected_pieces.extend(hexes_seen);
            return Ok(true);
        }

        for next in hive.occupied_neighbors_at_same_level(&current.hex) {
            if next == *hex_to_avoid {
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
    Ok(false)
}
