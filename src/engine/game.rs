use crate::engine::bug::Bug;
use crate::engine::game::Turn::{Move, Placement};
use crate::engine::hex::{Hex, is_adjacent, neighbors};
use crate::engine::hive::{Color, Hive, Tile};
use crate::engine::pathfinding::move_would_break_hive;
use crate::engine::zobrist::{ZobristHash, ZobristTable};
use Turn::Skip;
use itertools::Itertools;
use rustc_hash::{FxHashMap, FxHashSet};
use std::cmp::max;

#[derive(Clone)]
pub struct Game {
    pub hive: Hive,
    pub zobrist_table: &'static ZobristTable,
    pub zobrist_hash: ZobristHash,
    pub white_reserve: Vec<Bug>,
    pub black_reserve: Vec<Bug>,
    pub active_player: Color,
}

#[derive(Debug, Eq, PartialEq, Clone, Copy, Ord, PartialOrd, Hash)]
pub enum Turn {
    Placement { hex: Hex, tile: Tile },
    Move { from: Hex, to: Hex },
    Skip,
}

#[derive(Debug)]
pub enum GameResult {
    None,
    Draw,
    Winner { color: Color },
}

const DEFAULT_RESERVE_SIZE: usize = 13;

fn default_reserve() -> Vec<Bug> {
    vec![
        Bug::Queen,
        Bug::Ant,
        Bug::Ant,
        Bug::Ant,
        Bug::Beetle,
        Bug::Beetle,
        Bug::Grasshopper,
        Bug::Grasshopper,
        Bug::Grasshopper,
        Bug::Spider,
        Bug::Spider,
        Bug::Ladybug,
        Bug::Mosquito,
    ]
}

impl Default for Game {
    fn default() -> Self {
        Game {
            hive: Hive {
                map: Default::default(),
            },
            white_reserve: default_reserve(),
            black_reserve: default_reserve(),
            active_player: Color::White,
            zobrist_table: ZobristTable::get(),
            zobrist_hash: Default::default(),
        }
    }
}

impl Game {
    pub fn turn_is_valid(&self, turn: Turn) -> bool {
        //TODO: This is a really slow way to implement this
        self.valid_turns().contains(&turn)
    }

    pub fn from_hive(hive: Hive, active_player: Color) -> Game {
        let mut white_reserve = default_reserve();
        let mut black_reserve = default_reserve();
        for (_, tile) in hive.map.iter() {
            if tile.color == Color::White {
                let index = white_reserve.iter().position(|b| *b == tile.bug).unwrap();
                white_reserve.remove(index);
            } else {
                let index = black_reserve.iter().position(|b| *b == tile.bug).unwrap();
                black_reserve.remove(index);
            }
        }

        Self::from_hive_with_reserves(hive, active_player, white_reserve, black_reserve)
    }

    pub fn from_hive_with_reserves(
        hive: Hive,
        active_player: Color,
        white_reserve: Vec<Bug>,
        black_reserve: Vec<Bug>,
    ) -> Game {
        let zobrist_table = ZobristTable::get();
        let zobrist_hash = zobrist_table.hash(&hive, active_player);
        Game {
            hive,
            white_reserve,
            black_reserve,
            zobrist_table,
            zobrist_hash,
            active_player,
        }
    }

    pub fn with_turn_applied(&self, turn: Turn) -> Game {
        let mut new_map = self.hive.map.clone();
        match turn {
            Placement { tile, hex } => {
                let mut new_reserve = self.active_reserve().clone();
                if tile.color != self.active_player {
                    panic!(
                        "Cannot apply {turn:?}, {} is not the active player",
                        tile.color
                    )
                }

                let bug_index = self
                    .active_reserve()
                    .iter()
                    .position(|bug| bug == &tile.bug);
                match bug_index {
                    None => {
                        panic!()
                    }
                    Some(index) => {
                        new_reserve.remove(index);
                    }
                }

                if self.hive.is_occupied(&hex) {
                    panic!()
                }

                let white_reserve;
                let black_reserve;

                if self.active_player == Color::White {
                    white_reserve = new_reserve;
                    black_reserve = self.black_reserve.clone();
                } else {
                    white_reserve = self.white_reserve.clone();
                    black_reserve = new_reserve;
                }

                new_map.insert(hex, tile);
                let new_zobrist_hash = self
                    .zobrist_hash
                    .with_added_tile(self.zobrist_table, &hex, &tile)
                    .with_turn_change(self.zobrist_table);

                Game {
                    hive: Hive { map: new_map },
                    white_reserve,
                    black_reserve,
                    active_player: self.active_player.opposite(),
                    zobrist_table: self.zobrist_table,
                    zobrist_hash: new_zobrist_hash,
                }
            }
            Move { from, to } => {
                if !self.hive.is_occupied(&from) || self.hive.is_occupied(&to) {
                    panic!()
                }

                let tile = new_map.remove(&from).unwrap();
                if tile.color != self.active_player {
                    panic!()
                }

                new_map.insert(to, tile);
                let new_zobrist_hash = self
                    .zobrist_hash
                    .with_removed_tile(self.zobrist_table, &from, &tile)
                    .with_added_tile(self.zobrist_table, &to, &tile)
                    .with_turn_change(self.zobrist_table);

                Game {
                    hive: Hive { map: new_map },
                    white_reserve: self.white_reserve.clone(),
                    black_reserve: self.black_reserve.clone(),
                    active_player: self.active_player.opposite(),
                    zobrist_table: self.zobrist_table,
                    zobrist_hash: new_zobrist_hash,
                }
            }
            Skip => {
                let new_zobrist_hash = self.zobrist_hash ^ self.zobrist_table.black_to_move;
                Game {
                    hive: self.hive.clone(),
                    white_reserve: self.white_reserve.clone(),
                    black_reserve: self.black_reserve.clone(),
                    active_player: self.active_player.opposite(),
                    zobrist_table: self.zobrist_table,
                    zobrist_hash: new_zobrist_hash,
                }
            }
        }
    }

    pub fn game_result(&self) -> GameResult {
        let losing_colors: Vec<Color> = self
            .hive
            .map
            .iter()
            .filter(|(hex, t)| {
                t.bug == Bug::Queen && self.hive.occupied_neighbors_at_same_level(hex).count() == 6
            })
            .map(|(_, t)| t.color)
            .collect();

        if losing_colors.is_empty() {
            return GameResult::None;
        }
        if losing_colors.len() == 2 {
            return GameResult::Draw;
        }

        GameResult::Winner {
            color: losing_colors.first().unwrap().opposite(),
        }
    }

    fn active_reserve(&self) -> &Vec<Bug> {
        match self.active_player {
            Color::Black => &self.black_reserve,
            Color::White => &self.white_reserve,
        }
    }

    pub fn valid_destinations_for_piece(&self, hex: &Hex) -> impl Iterator<Item = Hex> {
        //TODO: This is a slow way to do this
        self.valid_moves()
            .into_iter()
            .filter_map(|turn| match turn {
                Move { from, to } if from == *hex => Some(to),
                _ => None,
            })
    }

    pub fn valid_turns(&self) -> Vec<Turn> {
        let mut valid_turns: Vec<Turn> = vec![];
        let active_player_reserve = if self.active_player == Color::Black {
            &self.black_reserve
        } else {
            &self.white_reserve
        };

        // Find all valid placements
        valid_turns.extend(self.valid_placements(active_player_reserve));

        // Find all valid moves
        valid_turns.extend(self.valid_moves());

        if valid_turns.is_empty() {
            valid_turns.push(Skip)
        }
        valid_turns
    }

    fn valid_placements(&self, active_player_reserve: &Vec<Bug>) -> Vec<Turn> {
        if active_player_reserve.is_empty() {
            return vec![];
        }

        if self.hive.map.is_empty() {
            return active_player_reserve
                .iter()
                .filter(|bug| **bug != Bug::Queen)
                .unique()
                .map(|bug| Placement {
                    hex: Hex { q: 0, r: 0, h: 0 },
                    tile: Tile {
                        bug: *bug,
                        color: self.active_player,
                    },
                })
                .collect();
        }

        if self.hive.map.len() == 1 {
            let only_occupied_hex = self.hive.map.iter().next().unwrap().0;

            return active_player_reserve
                .iter()
                .filter(|bug| **bug != Bug::Queen)
                .flat_map(|bug| {
                    neighbors(only_occupied_hex).map(|hex| Placement {
                        hex,
                        tile: Tile {
                            bug: *bug,
                            color: self.active_player,
                        },
                    })
                })
                .collect();
        }

        let mut placement_allowed: FxHashMap<Hex, bool> = FxHashMap::default();
        let mut valid_turns: Vec<Turn> = Vec::new();
        let reserve = if active_player_reserve.len() <= DEFAULT_RESERVE_SIZE - 3
            && active_player_reserve.contains(&Bug::Queen)
        {
            &vec![Bug::Queen]
        } else {
            active_player_reserve
        };

        for (hex, tile) in self.hive.map.iter() {
            if tile.color == self.active_player {
                for neighbor in self.hive.unoccupied_neighbors(&Hex { h: 0, ..*hex }) {
                    let allowed = *placement_allowed.entry(neighbor).or_insert_with(|| {
                        !self.is_adjacent_to_color(&neighbor, &self.active_player.opposite())
                    });
                    if allowed {
                        let turns = reserve.iter().map(|bug| Turn::Placement {
                            hex: neighbor,
                            tile: Tile {
                                bug: *bug,
                                color: self.active_player,
                            },
                        });

                        valid_turns.extend(turns);
                    }
                }
            }
        }

        valid_turns
    }
    fn valid_moves(&self) -> Vec<Turn> {
        let mut valid_turns: Vec<Turn> = Vec::new();
        if self.active_reserve().contains(&Bug::Queen) {
            return Vec::new();
        }
        for (hex, tile) in self.hive.toplevel_pieces() {
            if tile.color == self.active_player {
                let allowed_turns = self
                    .allowed_destinations(tile.bug, hex)
                    .map(|to| Move { from: *hex, to });
                valid_turns.extend(allowed_turns)
            }
        }

        valid_turns
    }

    fn allowed_destinations<'a>(
        &'a self,
        bug: Bug,
        hex: &'a Hex,
    ) -> Box<dyn Iterator<Item = Hex> + 'a> {
        match bug {
            Bug::Beetle => Box::new(self.allowed_beetle_destinations(&hex)),
            Bug::Queen => Box::new(self.allowed_queen_destinations(&hex)),
            Bug::Grasshopper => Box::new(self.allowed_grasshopper_destinations(&hex)),
            Bug::Ant => Box::new(self.allowed_ant_destinations(&hex)),
            Bug::Spider => Box::new(self.allowed_spider_destinations(&hex)),
            Bug::Ladybug => Box::new(self.allowed_ladybug_destinations(&hex)),
            Bug::Mosquito => Box::new(self.allowed_mosquito_destinations(&hex)),
        }
    }

    fn allowed_grasshopper_destinations(&self, hex: &Hex) -> impl Iterator<Item = Hex> {
        let mut allowed_jumps = vec![];
        let occupied_neighbors = self.hive.occupied_neighbors_at_same_level(hex);
        for neighbor in occupied_neighbors {
            let direction = neighbor - *hex;
            let unoccupied_spot = self.hive.next_unoccupied_spot_in_direction(hex, &direction);
            allowed_jumps.push(unoccupied_spot);
        }
        allowed_jumps
            .into_iter()
            .filter(|possible_move| !move_would_break_hive(&self.hive, hex, possible_move))
    }

    fn allowed_queen_destinations(&self, hex: &Hex) -> impl Iterator<Item = Hex> {
        self.allowed_slides(hex, Some(hex))
            .filter(|possible_move| !move_would_break_hive(&self.hive, hex, possible_move))
    }

    fn allowed_beetle_destinations(&self, from: &Hex) -> impl Iterator<Item = Hex> {
        neighbors(from)
            .filter_map(|neighbor| {
                let to_height = self.hive.stack_height(&neighbor);
                // If we're moving up, we need to check if we're blocked from the level above
                // If we're moving down, we need to check if we're blocked at our level
                let slide_check_height = max(from.h, to_height);
                if self.slide_is_allowed(
                    &Hex {
                        h: slide_check_height,
                        ..*from
                    },
                    &Hex {
                        h: slide_check_height,
                        ..neighbor
                    },
                ) {
                    Some(Hex {
                        h: to_height,
                        ..neighbor
                    })
                } else {
                    None
                }
            })
            .filter(|possible_move| !move_would_break_hive(&self.hive, from, possible_move))
    }

    fn allowed_ladybug_destinations(&self, start: &Hex) -> impl Iterator<Item = Hex> {
        let mut paths: Vec<Vec<Hex>> = vec![vec![*start]];
        let mut new_paths: Vec<Vec<Hex>> = vec![];

        for i in 1..=3 {
            let last_move = i == 3;
            for path in paths.iter() {
                let current = path.last().unwrap();
                let dests: Vec<Hex> = if last_move {
                    self.hive
                        .unoccupied_neighbors(&Hex { h: 0, ..*current })
                        .filter(|dest| {
                            self.slide_is_allowed(
                                current,
                                &Hex {
                                    h: current.h,
                                    ..*dest
                                },
                            )
                        })
                        .collect()
                } else {
                    self.hive
                        .topmost_occupied_neighbors(current)
                        .map(|dest| Hex {
                            h: dest.h + 1,
                            ..dest
                        })
                        .filter(|dest| dest.base_level() != *start)
                        .filter(|dest| {
                            self.slide_is_allowed(
                                &Hex {
                                    h: dest.h,
                                    ..*current
                                },
                                dest,
                            )
                        })
                        .filter(|dest| !(i == 1 && move_would_break_hive(&self.hive, start, dest)))
                        .collect()
                };

                for dest in dests {
                    let mut new_path = path.clone();
                    new_path.push(dest);
                    new_paths.push(new_path);
                }
            }
            // Allow us to re-use new_paths without allocating new memory
            // the old value of paths is no longer needed
            std::mem::swap(&mut paths, &mut new_paths);
            new_paths.clear();
        }

        let unique_destinations: FxHashSet<Hex> = FxHashSet::from_iter(
            paths
                .into_iter()
                .filter(|path| path.len() == 4)
                .map(|path| *path.last().unwrap()),
        );
        unique_destinations.into_iter()
    }

    fn allowed_spider_destinations(&self, start: &Hex) -> impl Iterator<Item = Hex> {
        let mut paths: Vec<Vec<Hex>> = vec![vec![*start]];
        let mut new_paths: Vec<Vec<Hex>> = vec![];

        for i in 1..=3 {
            let first_move = i == 1;
            for path in paths.iter() {
                let current = path.last().unwrap();
                for dest in self.allowed_slides(current, Some(start)) {
                    if path.contains(&dest) {
                        continue;
                    }
                    // The spider can only break the hive on its first move as long as it is adjacent to
                    // something at each step. I think?!?!?!
                    if first_move && move_would_break_hive(&self.hive, current, &dest)
                        || !first_move
                            && self.slide_would_separate_self_from_hive(current, &dest, start)
                    {
                        continue;
                    }

                    let mut new_path = path.clone();
                    new_path.push(dest);
                    new_paths.push(new_path);
                }
            }

            // Allow us to re-use new_paths without allocating new memory
            // the old value of paths is no longer needed
            std::mem::swap(&mut paths, &mut new_paths);
            new_paths.clear();
        }

        let mut unique_destinations: FxHashSet<Hex> = FxHashSet::default();
        unique_destinations.extend(
            paths
                .into_iter()
                .filter(|path| path.len() == 4)
                .map(|path| *path.last().unwrap()),
        );
        unique_destinations.into_iter()
    }

    fn allowed_ant_destinations(&self, start: &Hex) -> impl Iterator<Item = Hex> {
        let mut current = *start;
        let mut allowed_moves = FxHashSet::default();
        let mut frontier: Vec<Hex> = vec![];
        frontier.push(current);

        let mut first_move = true;
        while !frontier.is_empty() {
            current = frontier.pop().unwrap();
            for dest in self.allowed_slides(&current, Some(start)) {
                if allowed_moves.contains(&dest) || *start == dest {
                    continue;
                }
                // The ant can only break the hive on its first move as long as it is adjacent to
                // something at each step. I think?!?!?!
                if first_move && move_would_break_hive(&self.hive, &current, &dest)
                    || !first_move
                        && self.slide_would_separate_self_from_hive(&current, &dest, start)
                {
                    continue;
                }
                allowed_moves.insert(dest);
                frontier.push(dest);
            }
            first_move = false;
        }

        allowed_moves.into_iter()
    }

    fn allowed_mosquito_destinations<'a>(&'a self, hex: &'a Hex) -> impl Iterator<Item = Hex> + 'a {
        self.hive
            .topmost_occupied_neighbors(hex)
            .map(|hex| self.hive.map.get(&hex).unwrap().bug)
            // Not allowed to copy other mosquitos
            .filter(|bug| *bug != Bug::Mosquito)
            .flat_map(|bug| self.allowed_destinations(bug, hex))
    }

    fn slide_would_separate_self_from_hive(&self, from: &Hex, to: &Hex, ignore_hex: &Hex) -> bool {
        !self
            .hive
            .occupied_neighbors_at_same_level(from)
            .any(|neighbor| neighbor != *ignore_hex && is_adjacent(&neighbor, to))
    }

    fn slide_is_allowed(&self, from: &Hex, to: &Hex) -> bool {
        assert_eq!(from.h, to.h, "Slides must stay at the same height");

        // To test if a slide is allowed, we need to check if the two adjacent tiles to the slide
        // are blocking the slide for example in this board:
        // .  .  1
        //  .  Q  d
        // .  .  2
        // To check if Q can move to position d, we need to check spaces 1 and 2. If both are
        // filled, Q cannot move there.
        let mov = to - from;
        let counter_clockwise_neighbor = from
            + &Hex {
                q: -mov.s(),
                r: -mov.q,
                h: 0,
            };
        let clockwise_neighbor = from
            + &Hex {
                q: -mov.r,
                r: -mov.s(),
                h: 0,
            };

        !self.hive.is_occupied(&clockwise_neighbor)
            || !self.hive.is_occupied(&counter_clockwise_neighbor)
    }

    fn allowed_slides(
        &self,
        hex: &Hex,
        ignore_hex: Option<&Hex>,
    ) -> impl Iterator<Item = Hex> + use<> {
        let neighbors: Vec<Hex> = self.hive.neighbors_at_same_level(hex).collect();

        let mut empty_seen = 0;
        let mut allowed_slides: FxHashSet<Hex> = FxHashSet::default();
        for (i, hex) in neighbors.iter().enumerate() {
            if self.hive.is_occupied(hex) && Some(hex) != ignore_hex {
                empty_seen = 0;
            } else {
                if empty_seen > 0 {
                    allowed_slides.insert(*hex);
                    allowed_slides.insert(neighbors[i - 1]);
                }
                empty_seen += 1;
            }
        }

        let first = neighbors.first().unwrap();
        let last = neighbors.last().unwrap();
        if !self.hive.is_occupied(first) && !self.hive.is_occupied(last) {
            allowed_slides.insert(*first);
            allowed_slides.insert(*last);
        }

        allowed_slides.into_iter()
    }

    fn is_adjacent_to_color(&self, hex: &Hex, color: &Color) -> bool {
        self.hive
            .topmost_occupied_neighbors(hex)
            .any(|adjacent_hex| {
                self.hive
                    .map
                    .get(&adjacent_hex)
                    .is_some_and(|tile| tile.color == *color)
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::parse::{hex_map_to_string, parse_hex_map_string};
    use Turn::Move;
    use Turn::Placement;

    fn turns_to_string(hex_map: &FxHashMap<Hex, String>, turns: Vec<Turn>) -> String {
        let mut turns_map = hex_map.clone();
        for turn in turns {
            match turn {
                Placement { hex, tile: _ } => {
                    turns_map.insert(hex, "*".to_owned());
                }
                Move { from: _, to } => {
                    turns_map.insert(to, "*".to_owned());
                }
                Skip => {}
            }
        }
        hex_map_to_string(&turns_map)
    }

    fn assert_placements(placements: &str) {
        let placements_map = parse_hex_map_string(placements).unwrap();
        let mut expected_placements: Vec<Turn> = placements_map
            .iter()
            .filter(|(_, token)| *token == "*")
            .map(|(hex, _)| Placement {
                hex: *hex,
                tile: Tile {
                    bug: Bug::Queen,
                    color: Color::White,
                },
            })
            .collect();

        let hex_map: FxHashMap<Hex, String> = placements_map
            .into_iter()
            .filter(|(_, token)| *token != "*")
            .collect();
        let hive = Hive::from_hex_map(&hex_map).unwrap();
        let game = Game::from_hive(hive, Color::White);

        let mut actual_placements: Vec<Turn> = game
            .valid_turns()
            .into_iter()
            .filter(|turn| matches!(turn, Placement { .. }))
            .collect();

        expected_placements.sort();
        actual_placements.sort();

        if expected_placements != actual_placements {
            let actual_placements_map = turns_to_string(&hex_map, actual_placements);
            let expected_placements_map = turns_to_string(&hex_map, expected_placements);

            pretty_assertions::assert_eq!(expected_placements_map, actual_placements_map);
        }
    }

    fn assert_moves(moves: &str) {
        assert_moves_for_hex(moves, None)
    }

    fn assert_moves_for_hex(moves: &str, only_for_hex: Option<Hex>) {
        let moves_map = parse_hex_map_string(moves).unwrap();
        let (from, _) = moves_map
            .iter()
            .find(|(_, token)| token.chars().next().unwrap().is_uppercase())
            .unwrap();

        let mut expected_turns: Vec<Turn> = moves_map
            .iter()
            .filter(|(_, token)| *token == "*")
            .map(|(hex, _)| Move {
                from: *from,
                to: *hex,
            })
            .collect();

        let hex_map: FxHashMap<Hex, String> = moves_map
            .into_iter()
            .filter(|(_, token)| *token != "*")
            .collect();
        let hive = Hive::from_hex_map(&hex_map).unwrap();
        let game = Game::from_hive_with_reserves(hive, Color::White, vec![], vec![]);

        let mut actual_turns: Vec<Turn> = game.valid_turns();

        if let Some(hex) = only_for_hex {
            actual_turns.retain(|turn| match turn {
                Move { from, .. } => *from == hex,
                _ => false,
            });
        }

        expected_turns.sort();
        actual_turns.sort();

        if expected_turns != actual_turns {
            let expected_moves_map = turns_to_string(&hex_map, expected_turns);
            let actual_moves_map = turns_to_string(&hex_map, actual_turns);
            pretty_assertions::assert_str_eq!(expected_moves_map, actual_moves_map)
        }
    }

    #[test]
    fn test_placement() {
        assert_placements(
            r#"
            .  a  .
             .  B  *
            .  *  *
        "#,
        )
    }

    #[test]
    fn test_placement_with_multiple_layers() {
        assert_placements(
            r#"
        Layer 0
            .  a  .
             .  B  .
            .  .  .
        Layer 1
            .  a  .
             .  b  .
            .  .  .
        "#,
        )
    }

    #[test]
    fn test_placement_not_allowed_above_layer_0() {
        assert_placements(
            r#"
        Layer 0
            .  a  .
             .  b  .
            .  .  a
        Layer 1
            .  .  .
             .  B  .
            .  .  .
        "#,
        )
    }

    #[test]
    fn test_placement_uses_top_layer_for_hex_color() {
        assert_placements(
            r#"
        Layer 0
            .  a  .
             .  b  *
            .  *  *
        Layer 1
            .  .  .
             .  B  .
            .  .  .
        "#,
        )
    }

    #[test]
    fn test_queen_cannot_move_out_from_under_beetle() {
        assert_moves(
            r#"
        Layer 0
            .  a  .
             .  Q  .
            .  .  .
        Layer 1
            .  .  .
             .  b  .
            .  .  .
        "#,
        )
    }

    #[test]
    fn test_queen_move_slide() {
        assert_moves(
            r#"
            .  a  *
             *  Q  .
            .  .  .
        "#,
        );
    }

    #[test]
    fn test_queen_move_does_not_break_hive() {
        assert_moves(
            r#"
            .  a  .
             .  Q  a
            .  .  .
        "#,
        );
    }

    #[test]
    fn test_queen_move_escapes_semicircle() {
        assert_moves(
            r#"
            .  a  *
             b  Q  *
            .  b  a
        "#,
        );
    }

    #[test]
    fn test_queen_move_does_not_temporarily_break_hive() {
        assert_moves(
            r#"
            .  b  b  .
             q  *  a  .
            .  .  Q  *
        "#,
        );
    }

    #[test]
    fn test_queen_move_can_escape_semicircle_in_top_left() {
        assert_moves(
            r#"
            .  *  a
             *  Q  a
            .  b  b
        "#,
        );
    }

    #[test]
    fn test_beetle_move_slide() {
        assert_moves(
            r#"
        Layer 0
            .  a  *
             *  B  .
            .  .  .
        Layer 1
            .  *  .
             .  .  .
            .  .  .
        "#,
        );
    }

    #[test]
    fn test_beetle_can_slide_down() {
        assert_moves(
            r#"
            Layer 0
            .  *  a
             a  q  *
            .  *  *
            Layer 1
            .  .  *
             *  B  .
            .  .  .
            "#,
        );
    }

    #[test]
    fn test_beetle_move_from_higher_layer() {
        assert_moves(
            r#"
        Layer 0
            .  a  *
             *  a  *
            .  *  *
        Layer 1
            .  *  .
             .  B  .
            .  .  .
        "#,
        );
    }

    #[test]
    fn test_beetle_move_does_not_break_hive() {
        assert_moves(
            r#"
        Layer 0
            .  a  .
             .  B  a
            .  b  .
        "#,
        );
    }

    #[test]
    fn test_beetle_move_can_slide_or_mount() {
        assert_moves(
            r#"
        Layer 0
            *  a  .
             B  b  .
            *  a  .
        Layer 1
            .  *  .
           .  *  .
            .  *  .
        "#,
        );
    }

    #[test]
    fn test_grasshopper_move_jumps() {
        assert_moves(
            r#"
            .  *  .
             .  a  .
            *  a  G
        "#,
        );
    }

    #[test]
    fn test_ant_move_unlimited_slides() {
        assert_moves(
            r#"
            .  A  *
             *  q  *
            .  *  *
        "#,
        );
    }

    #[test]
    fn test_valid_spider_moves() {
        assert_moves(
            r#"
            .  S  .
             .  b  .
            .  .  *
        "#,
        );
    }

    #[test]
    fn test_spider_moves_finds_multiple_paths_to_same_location() {
        assert_moves(
            r#"
            *  *  *  *
             a  .  a  .
            b  .  S  g
             g  g  a  .
        "#,
        );
    }

    #[test]
    fn test_grasshopper_does_not_break_hive() {
        assert_moves(
            r#"
            .  a  .
             .  G  .
            .  .  a
        "#,
        );
    }

    #[test]
    fn test_spider_cannot_make_illegal_slides() {
        assert_moves(
            r#"
            q  .
             a  .
            .  q
             S  .
            .  q
             a  .
        "#,
        );
    }

    #[test]
    fn test_spider_cannot_temporarily_break_hive() {
        assert_moves(
            r#"
            .  a  q
             .  .  a
            s  S  a
        "#,
        );
    }

    #[test]
    fn test_ant_cannot_temporarily_break_hive() {
        assert_moves(
            r#"
            .  a  q
             .  .  a
            s  A  a
        "#,
        );
    }

    #[test]
    fn test_beetle_cannot_temporarily_break_hive() {
        assert_moves(
            r#"
            .  .  .
             .  a  a
            .  .  Q
             .  .  a
        "#,
        );
    }

    #[test]
    fn test_beetle_cannot_break_slide_rules_when_mounting() {
        assert_moves(
            r#"
            Layer 0
            .  a  a
             a  B  *
            .  *  .
            Layer 1
            .  .  b
             b  .  .
            .  .  .
            Layer 2
            .  .  *
             *  .  .
            .  .  .
            "#,
        )
    }

    #[test]
    fn test_ladybug_movement() {
        assert_moves(
            r#"
            .  *  *
             *  a  *
            .  a  *
             .  L  .
            "#,
        );
    }

    #[test]
    fn test_ladybug_can_traverse_any_height() {
        assert_moves(
            r#"
            Layer 0
            .  *  *
             *  a  *
            .  a  *
             .  L  .
            Layer 1
            .  .  .
             .  B  .
            .  .  .
             .  .  .
            Layer 2
            .  .  .
             .  b  .
            .  .  .
             .  .  .
            "#,
        );
    }

    #[test]
    fn test_ladybug_cant_make_illegal_slides() {
        assert_moves(
            r#"
            Layer 0
            .  .  a  .
             q  a  *  .
            .  *  a  .
             .  .  L  .
            Layer 1
            .  .  b  .
             b  .  .  .
            .  .  .  .
             .  .  .  .
            "#,
        );
    }

    #[test]
    fn test_ladybug_cant_break_hive() {
        assert_moves(
            r#"
            .  .  .  .
             .  a  .  .
            .  .  a  .
             .  .  L  .
            .  .  .  a
            "#,
        );
    }

    #[test]
    fn test_ant_can_go_in_and_out_of_pocket() {
        assert_moves(
            r#"
            .  *  *  *
             *  a  a  A
            .  *  *  a  *
             .  *  a  *
            .  .  *  *
        "#,
        );
    }

    #[test]
    fn test_mosquito_can_copy_queen() {
        assert_moves(
            r#"
        .  .  .
         .  q  *
        .  *  M
        "#,
        )
    }

    #[test]
    fn test_mosquito_can_copy_multiple_abilities() {
        assert_moves(
            r#"
        .  *  .  *
         .  q  g  .
        .  *  M  *
        "#,
        )
    }

    #[test]
    fn test_mosquito_cannot_copy_another_mosquito() {
        assert_moves(
            r#"
        .  q  .
         .  m  .
        .  .  M
        "#,
        );
    }
}
