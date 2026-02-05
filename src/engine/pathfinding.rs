use crate::engine::hex::Hex;
use crate::engine::hive::Hive;
use itertools::Itertools;
use rustc_hash::{FxHashMap, FxHashSet};
use std::cmp::min;

#[derive(Default)]
struct ArticulationSearchState {
    visited: FxHashSet<Hex>,
    depth: FxHashMap<Hex, i32>,
    low: FxHashMap<Hex, i32>,
    articulation_points: FxHashSet<Hex>,
}

pub fn articulation_points(hive: &Hive) -> FxHashSet<Hex> {
    if hive.map.is_empty() {
        return FxHashSet::default();
    }

    let mut state = ArticulationSearchState::default();
    // Only bottom level tiles can be articulation points, so we only examine the bottom level of
    // the hive
    let start = hive.map.keys().find_or_first(|hex| hex.h == 0).unwrap();
    state.visited.insert(*start);
    state.depth.insert(*start, 0);
    state.low.insert(*start, 0);

    let mut root_children_count = 0;
    for child in hive.occupied_neighbors_at_same_level(start) {
        if !state.visited.contains(&child) {
            root_children_count += 1;
            _find_articulation_points(&hive, &child, start, 1, &mut state);
        }
    }

    // If root has two or more children, it's an articulation point
    if root_children_count >= 2 {
        state.articulation_points.insert(*start);
    }

    state.articulation_points
}

fn _find_articulation_points(
    hive: &Hive,
    current: &Hex,
    parent: &Hex,
    depth: i32,
    mut state: &mut ArticulationSearchState,
) {
    state.depth.insert(*current, depth);
    state.low.insert(*current, depth);
    state.visited.insert(*current);

    for child in hive.occupied_neighbors_at_same_level(current) {
        if !state.visited.contains(&child) {
            // We haven't seen this child yet, calculate its low value
            _find_articulation_points(&hive, &child, current, depth + 1, &mut state);
            state
                .low
                .insert(*current, min(state.low[current], state.low[&child]));

            if state.low[&child] >= state.depth[current] {
                state.articulation_points.insert(*current);
            }
        } else if child != *parent {
            state
                .low
                .insert(*current, min(state.low[current], state.depth[&child]));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::parse::{hex_map_to_string, parse_hex_map_string};

    fn assert_articulation_points(board: &str) {
        let board_map = parse_hex_map_string(board).unwrap();

        // Extract expected articulation points (marked with *)
        // * means "this position should be an articulation point"
        let mut expected_points: FxHashSet<Hex> = FxHashSet::default();
        let mut hex_map: FxHashMap<Hex, String> = FxHashMap::default();

        for (hex, token) in board_map.iter() {
            if *token == "*" {
                // This position is marked as an articulation point
                expected_points.insert(*hex);
                // Use any valid bug type (we'll use 'a' for ant)
                hex_map.insert(*hex, "a".to_string());
            } else {
                hex_map.insert(*hex, token.clone());
            }
        }
        let hive = Hive::from_hex_map(&hex_map).unwrap();

        let actual_points: FxHashSet<Hex> = articulation_points(&hive).into_iter().collect();

        if expected_points != actual_points {
            // Create visualization maps for debugging
            let mut expected_map = hex_map.clone();
            for hex in expected_points.iter() {
                expected_map.insert(*hex, "*".to_owned());
            }

            let mut actual_map = hex_map.clone();
            for hex in actual_points.iter() {
                actual_map.insert(*hex, "*".to_owned());
            }

            pretty_assertions::assert_eq!(
                hex_map_to_string(&expected_map),
                hex_map_to_string(&actual_map)
            );
        }
    }

    #[test]
    fn test_linear_chain() {
        // In a linear chain, middle pieces are articulation points
        assert_articulation_points(
            r#"
            a  *  *  a
            "#,
        );
    }

    #[test]
    fn test_single_articulation_point_star() {
        // Star topology: center piece connects to three branches
        assert_articulation_points(
            r#"
            .  a  .
             .  *  a
            .  a  .
            "#,
        );
    }

    #[test]
    fn test_no_articulation_points_cycle() {
        // Hexagonal cycle has no articulation points
        assert_articulation_points(
            r#"
            .  a  a
             a  .  a
            .  a  a
            "#,
        );
    }

    #[test]
    fn test_multiple_articulation_points() {
        // Complex branching structure with multiple articulation points
        assert_articulation_points(
            r#"
            a  *  *  a
             .  .  a  a
            "#,
        );
    }

    #[test]
    fn test_bridge_structure() {
        // Two clusters connected by a bridge
        assert_articulation_points(
            r#"
            a  a  .  .
             a  *  .  .
            .  .  *  a
             .  .  a  a
            "#,
        );
    }

    #[test]
    fn test_root_with_multiple_children() {
        // Root node with multiple disconnected subtrees
        assert_articulation_points(
            r#"
            a  *  a
            "#,
        );
    }

    #[test]
    fn test_single_piece() {
        // Single piece has no articulation points
        assert_articulation_points(
            r#"
            a
            "#,
        );
    }

    #[test]
    fn test_two_pieces() {
        // Two connected pieces - no articulation points
        assert_articulation_points(
            r#"
            a  a
            "#,
        );
    }

    #[test]
    fn test_t_shape() {
        // T-shape: center is articulation point
        assert_articulation_points(
            r#"
            .  a  .
             a  *  a
            .  a  .
            "#,
        );
    }

    #[test]
    fn test_complex_hive() {
        // More realistic hive structure
        assert_articulation_points(
            r#"
            .  a  a  .
             a  a  *  a
            .  a  a  .
            "#,
        );
    }
}
