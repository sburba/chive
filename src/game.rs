use crate::bug::Bug;
use crate::game::Turn::{Move, Placement};
use crate::hex::{Hex, is_adjacent, neighbors};
use crate::hive::{Color, Hive, Tile};
use crate::pathfinding::move_would_break_hive;
use itertools::Itertools;
use std::collections::{HashMap, HashSet};

#[derive(Clone)]
pub struct Game {
    pub hive: Hive,
    pub white_reserve: Vec<Bug>,
    pub black_reserve: Vec<Bug>,
    pub active_player: Color,
}

#[derive(Debug, Eq, PartialEq, Clone, Copy, Ord, PartialOrd, Hash)]
pub enum Turn {
    Placement { hex: Hex, tile: Tile },
    Move { from: Hex, to: Hex },
}

pub enum GameResult {
    None,
    Draw,
    Winner { color: Color },
}

impl Game {
    pub fn with_turn_applied(&self, turn: Turn) -> Game {
        let mut new_map = self.hive.map.clone();
        match turn {
            Placement { tile, hex } => {
                let mut new_reserve = self.active_reserve().clone();
                if tile.color != self.active_player {
                    panic!("Cannot apply {turn:?}, is not the active player")
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
                //TODO: Handle the case that the opposing player can not move
                Game {
                    hive: Hive { map: new_map },
                    white_reserve,
                    black_reserve,
                    active_player: self.active_player.opposite(),
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
                //TODO: Handle the case that the opposing player can not move
                Game {
                    hive: Hive { map: new_map },
                    white_reserve: self.white_reserve.clone(),
                    black_reserve: self.black_reserve.clone(),
                    active_player: self.active_player.opposite(),
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

        if losing_colors.len() == 0 {
            return GameResult::None;
        }
        if losing_colors.len() == 2 {
            return GameResult::Draw;
        }

        GameResult::Winner {
            color: *losing_colors.first().unwrap(),
        }
    }

    fn active_reserve(&self) -> &Vec<Bug> {
        match self.active_player {
            Color::Black => &self.black_reserve,
            Color::White => &self.white_reserve,
        }
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

        valid_turns
    }

    fn valid_moves(&self) -> Vec<Turn> {
        let mut valid_turns: Vec<Turn> = Vec::new();
        if self.active_reserve().contains(&Bug::Queen) {
            return Vec::new();
        }
        let toplevel_pieces = self.hive.map.iter().filter(|(hex, _)| self.hive.stack_height(hex) - 1 == hex.h);
        for (hex, tile) in toplevel_pieces {
            if tile.color == self.active_player {
                match tile.bug {
                    Bug::Beetle => {
                        let bottom_level = Hex { h: 0, ..*hex };
                        let allowed_slides = self.allowed_slides(hex, None);
                        let allowed_mounts =
                            self.hive.occupied_neighbors_at_same_level(&bottom_level);
                        let possible_moves =
                            allowed_slides
                                .chain(allowed_mounts)
                                .map(|possible_move| Hex {
                                    h: self.hive.stack_height(&possible_move),
                                    ..possible_move
                                });

                        let unique_moves: HashSet<Hex> = HashSet::from_iter(possible_moves);

                        let allowed_moves = unique_moves
                            .into_iter()
                            .filter(|possible_move| {
                                !move_would_break_hive(&self.hive, hex, possible_move)
                            })
                            .map(|valid_move| Turn::Move {
                                from: *hex,
                                to: valid_move,
                            });
                        valid_turns.extend(allowed_moves);
                    }
                    Bug::Queen => {
                        let allowed_moves = self
                            .allowed_slides(hex, None)
                            .into_iter()
                            .filter(|possible_move| {
                                !move_would_break_hive(&self.hive, hex, possible_move)
                            })
                            .map(|valid_move| Turn::Move {
                                from: *hex,
                                to: valid_move,
                            });
                        valid_turns.extend(allowed_moves);
                    }
                    Bug::Grasshopper => {
                        let allowed_moves = self
                            .allowed_jumps(hex)
                            .filter(|possible_move| {
                                !move_would_break_hive(&self.hive, hex, possible_move)
                            })
                            .map(|valid_jump| Move {
                                from: *hex,
                                to: valid_jump,
                            });
                        valid_turns.extend(allowed_moves)
                    }
                    Bug::Ant => {
                        let allowed_moves = self.allowed_ant_destinations(hex).map(|slide| Move {
                            from: *hex,
                            to: slide,
                        });
                        valid_turns.extend(allowed_moves)
                    }
                    Bug::Spider => {
                        let allowed_moves = self
                            .allowed_spider_destinations(hex)
                            .map(|to| Move { from: *hex, to });
                        valid_turns.extend(allowed_moves)
                    }
                }
            }
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

        let mut placement_allowed: HashMap<Hex, bool> = HashMap::new();
        let mut valid_turns: Vec<Turn> = Vec::new();
        let reserve =
            if active_player_reserve.len() <= 8 && active_player_reserve.contains(&Bug::Queen) {
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

    fn allowed_jumps(&self, hex: &Hex) -> impl Iterator<Item = Hex> {
        let mut allowed_jumps = vec![];
        let occupied_neighbors = self.hive.occupied_neighbors_at_same_level(hex);
        for neighbor in occupied_neighbors {
            let direction = neighbor - *hex;
            let unoccupied_spot = self.hive.next_unoccupied_spot_in_direction(hex, &direction);
            allowed_jumps.push(unoccupied_spot);
        }

        allowed_jumps.into_iter()
    }

    fn allowed_spider_destinations(&self, start: &Hex) -> impl Iterator<Item = Hex> {
        let mut paths: Vec<Vec<Hex>> = vec![vec![*start]];

        for i in 1..=3 {
            let mut new_paths: Vec<Vec<Hex>> = vec![];
            for path in paths {
                let hex = path.last().unwrap();
                for dest in self.allowed_slides(&hex, Some(start)) {
                    if !self
                        .hive
                        .occupied_neighbors_at_same_level(&hex)
                        .filter(|neighbor| neighbor != start)
                        .any(|neighbor| is_adjacent(&dest, &neighbor))
                    {
                        continue;
                    }

                    if i == 1 && move_would_break_hive(&self.hive, start, &dest) {
                        continue;
                    }

                    if !path.contains(&dest) {
                        let mut new_path = path.clone();
                        new_path.push(dest);
                        new_paths.push(new_path);
                    }
                }
            }
            paths = new_paths;
        }

        let mut unique_destinations: HashSet<Hex> = HashSet::new();
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
        let mut allowed_moves = HashSet::new();
        let mut frontier: Vec<Hex> = vec![];
        frontier.push(current);

        let first_move = true;
        while !frontier.is_empty() {
            current = frontier.pop().unwrap();
            for dest in self.allowed_slides(&current, None) {
                // If the destination isn't connected to anything, then it's not a valid move
                if !self
                    .hive
                    .occupied_neighbors_at_same_level(&dest)
                    .any(|_| true)
                {
                    continue;
                }
                // The ant can only break the hive on its first move as long as it is adjacent to
                // something at each step. I think?!?!?!
                if first_move && move_would_break_hive(&self.hive, start, &dest) {
                    continue;
                }
                if *start != dest && allowed_moves.insert(dest) {
                    frontier.push(dest);
                }
            }
        }

        allowed_moves.into_iter()
    }

    fn allowed_slides(
        &self,
        hex: &Hex,
        ignore_hex: Option<&Hex>,
    ) -> impl Iterator<Item = Hex> + use<> {
        let neighbors: Vec<Hex> = self.hive.neighbors_at_same_level(hex).collect();

        let mut empty_seen = 0;
        let mut allowed_slides: HashSet<Hex> = HashSet::new();
        for (i, hex) in neighbors.iter().enumerate() {
            if self.hive.is_occupied(&hex) && Some(hex) != ignore_hex {
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
        if !self.hive.is_occupied(&first) && !self.hive.is_occupied(last) {
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
                    .map_or(false, |tile| tile.color == *color)
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::{hex_map_to_string, parse_hex_map_string};
    use Turn::Move;
    use Turn::Placement;

    fn turns_to_string(hex_map: &HashMap<Hex, String>, moves: Vec<Turn>) -> String {
        let mut turns_map = hex_map.clone();
        for mv in moves {
            match mv {
                Placement { hex, tile: _ } => {
                    turns_map.insert(hex, "*".to_owned());
                }
                Move { from: _, to } => {
                    turns_map.insert(to, "*".to_owned());
                }
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

        let hex_map: HashMap<Hex, String> = placements_map
            .into_iter()
            .filter(|(_, token)| *token != "*")
            .collect();
        let hive = Hive::from_hex_map(&hex_map).unwrap();

        let game = Game {
            hive,
            white_reserve: vec![Bug::Queen],
            black_reserve: vec![],
            active_player: Color::White,
        };

        let mut actual_placements: Vec<Turn> = game
            .valid_turns()
            .into_iter()
            .filter(|turn| match turn {
                Placement { .. } => true,
                Move { .. } => false,
            })
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

        let hex_map: HashMap<Hex, String> = moves_map
            .into_iter()
            .filter(|(_, token)| *token != "*")
            .collect();
        let hive = Hive::from_hex_map(&hex_map).unwrap();

        let game = Game {
            hive,
            white_reserve: vec![],
            black_reserve: vec![],
            active_player: Color::White,
        };

        let mut actual_turns = game.valid_turns();

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
        assert_placements(r#"
            .  a  .
             .  B  *
            .  *  *
        "#)
    }

    #[test]
    fn test_placement_with_multiple_layers() {
        assert_placements(r#"
        Layer 0
            .  a  .
             .  B  .
            .  .  .
        Layer 1
            .  a  .
             .  b  .
            .  .  .
        "#)
    }

    #[test]
    fn test_placement_not_allowed_above_layer_0() {
        assert_placements(r#"
        Layer 0
            .  a  .
             .  b  .
            .  .  a
        Layer 1
            .  .  .
             .  B  .
            .  .  .
        "#)
    }

    #[test]
    fn test_placement_uses_top_layer_for_hex_color() {
        assert_placements(r#"
        Layer 0
            .  a  .
             .  b  *
            .  *  *
        Layer 1
            .  .  .
             .  B  .
            .  .  .
        "#)
    }


    #[test]
    fn test_queen_cannot_move_out_from_under_beetle() {
        assert_moves(r#"
        Layer 0
            .  a  .
             .  Q  .
            .  .  .
        Layer 1
            .  .  .
             .  b  .
            .  .  .
        "#)
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
    fn test_spider_does_not_turbofuck_hive() {
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
}
