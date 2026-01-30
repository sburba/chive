use crate::engine::bug::Bug;
use crate::engine::game::Turn::{Move, Placement};
use crate::engine::hex::{Hex, is_adjacent, neighbors};
use crate::engine::hive::{Color, Hive, HiveParseError, Tile};
use crate::engine::parse::{HexMapParseError, parse_hex_map_string};
use crate::engine::pathfinding::move_would_break_hive;
use crate::engine::zobrist::{ZobristHash, ZobristTable};
use Turn::Skip;
use itertools::{Either, Itertools};
use rustc_hash::{FxHashMap, FxHashSet};
use std::cmp::max;
use std::iter;
use thiserror::Error;

#[derive(Clone)]
pub struct Game {
    pub hive: Hive,
    pub zobrist_table: &'static ZobristTable,
    pub zobrist_hash: ZobristHash,
    pub white_reserve: Vec<Bug>,
    pub black_reserve: Vec<Bug>,
    pub active_player: Color,
    pub immobilized_piece: Option<Hex>,
    pub last_turn: Option<Turn>,
}

#[derive(Debug, Eq, PartialEq, Clone, Copy, Ord, PartialOrd, Hash)]
pub enum Turn {
    Placement {
        hex: Hex,
        tile: Tile,
    },
    Move {
        from: Hex,
        to: Hex,
        freezes_piece: bool,
    },
    Skip,
}

#[derive(Debug)]
pub enum GameResult {
    None,
    Draw,
    Winner { color: Color },
}

const DEFAULT_RESERVE: [Bug; 14] = [
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
    Bug::Pillbug,
];

fn default_reserve() -> Vec<Bug> {
    Vec::from(DEFAULT_RESERVE)
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
            last_turn: None,
            immobilized_piece: None,
            zobrist_table: ZobristTable::get(),
            zobrist_hash: Default::default(),
        }
    }
}

#[derive(Error, Debug)]
pub enum GameParseError {
    #[error("Invalid hex map string")]
    InvalidMap(#[from] HexMapParseError),
    #[error("Invalid hive configuration")]
    InvalidHive(#[from] HiveParseError),
}

impl Game {
    pub fn turn_is_valid(&self, turn: Turn) -> bool {
        //TODO: This is a really slow way to implement this
        self.turns().contains(&turn)
    }

    pub fn from_map_str(map: &str) -> Result<Game, GameParseError> {
        let hex_map = parse_hex_map_string(map)?;
        let hive = Hive::from_hex_map(&hex_map)?;
        Ok(Self::from_hive(hive, Color::White))
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
            last_turn: None,
            immobilized_piece: None,
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
                    immobilized_piece: None,
                    last_turn: Some(turn),
                    active_player: self.active_player.opposite(),
                    zobrist_table: self.zobrist_table,
                    zobrist_hash: new_zobrist_hash,
                }
            }
            Move {
                from,
                to,
                freezes_piece,
            } => {
                debug_assert!(
                    self.hive.is_occupied(&from),
                    "There isn't a piece at {:?}",
                    from
                );
                debug_assert!(!self.hive.is_occupied(&to), "There is a piece at {:?}", to);

                let tile = new_map.remove(&from).unwrap();
                debug_assert!(
                    tile.color == self.active_player || freezes_piece,
                    "Only the pillbug can move a piece of the opposing player, and that should freeze the piece"
                );

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
                    last_turn: Some(turn),
                    immobilized_piece: if freezes_piece { Some(to) } else { None },
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
                    last_turn: Some(turn),
                    immobilized_piece: None,
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
        self.moves().into_iter().filter_map(|turn| match turn {
            Move {
                from,
                to,
                freezes_piece: false,
            } if from == *hex => Some(to),
            _ => None,
        })
    }

    pub fn turns(&self) -> impl Iterator<Item = Turn> {
        let active_player_reserve = if self.active_player == Color::Black {
            &self.black_reserve
        } else {
            &self.white_reserve
        };

        let mut turns = self
            .placements(active_player_reserve)
            .into_iter()
            .chain(self.moves())
            .peekable();

        // If there are no valid turns, you must skip
        if turns.peek().is_none() {
            Either::Left(iter::once(Skip))
        } else {
            Either::Right(turns)
        }
    }

    fn placements<'a>(
        &'a self,
        active_player_reserve: &'a Vec<Bug>,
    ) -> Box<dyn Iterator<Item = Turn> + 'a> {
        if active_player_reserve.is_empty() {
            return Box::new(iter::empty());
        }

        if self.hive.map.is_empty() {
            return Box::new(
                active_player_reserve
                    .iter()
                    .filter(|bug| **bug != Bug::Queen)
                    .unique()
                    .map(|bug| Placement {
                        hex: Hex { q: 0, r: 0, h: 0 },
                        tile: Tile {
                            bug: *bug,
                            color: self.active_player,
                        },
                    }),
            );
        }

        if self.hive.map.len() == 1 {
            let only_occupied_hex = self.hive.map.iter().next().unwrap().0;

            return Box::new(
                active_player_reserve
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
                    }),
            );
        }

        let mut placement_allowed: FxHashMap<Hex, bool> = FxHashMap::default();
        let mut valid_turns: Vec<Turn> = Vec::new();
        // If you haven't played your queen by turn 4, you must play your queen
        let is_turn_four = active_player_reserve.len() <= DEFAULT_RESERVE.len() - 3;
        let reserve = if is_turn_four && active_player_reserve.contains(&Bug::Queen) {
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
                        let turns = reserve.iter().map(|bug| Placement {
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

        Box::new(valid_turns.into_iter())
    }

    fn moves(&self) -> impl Iterator<Item = Turn> {
        if self.active_reserve().contains(&Bug::Queen) {
            return Either::Left(iter::empty());
        }

        Either::Right(
            self.hive
                .toplevel_pieces()
                .filter(|(_, tile)| tile.color == self.active_player)
                .flat_map(|(hex, tile)| self.moves_for_tile(tile.bug, hex)),
        )
    }

    pub fn moves_for_piece<'a>(&'a self, hex: &'a Hex) -> impl Iterator<Item = Turn> {
        // If you haven't placed your queen yet you're not allowed to move.
        // Only the top piece in a stack is allowed to move
        if self.active_reserve().contains(&Bug::Queen) || self.hive.stack_height(hex) != hex.h + 1 {
            return Either::Left(iter::empty());
        }

        let tile = self.hive.tile_at(hex).unwrap();
        Either::Right(self.moves_for_tile(tile.bug, hex))
    }

    fn moves_for_tile<'a>(&'a self, bug: Bug, hex: &'a Hex) -> Box<dyn Iterator<Item = Turn> + 'a> {
        match bug {
            Bug::Beetle => Box::new(self.beetle_moves(&hex)),
            Bug::Queen => Box::new(self.queen_moves(&hex)),
            Bug::Grasshopper => Box::new(self.grasshopper_moves(&hex)),
            Bug::Ant => Box::new(self.ant_moves(&hex)),
            Bug::Spider => Box::new(self.spider_moves(&hex)),
            Bug::Ladybug => Box::new(self.ladybug_moves(&hex)),
            Bug::Mosquito => Box::new(self.mosquito_moves(&hex)),
            Bug::Pillbug => Box::new(self.pillbug_moves(&hex)),
        }
    }

    fn pillbug_moves(&self, pillbug_hex: &Hex) -> impl Iterator<Item = Turn> {
        // The Pillbug moves one space at a time, but it also has a special ability it may use
        // instead of moving.
        // The special ability allows the Pillbug to move an adjacent piece (friend or enemy) up to
        // two spaces; up onto itself and then down into another empty space adjacent to itself.
        //
        // Exceptions:
        //  * The Pillbug may not move the piece which was just moved by the other player
        //  * The Pillbug may not move any piece in a stack of pieces
        //  * The Pillbug may not move a piece if it splits the hive (violating the One Hive Rule)
        //  * The Pillbug may not move a piece through a narrow gap of stacked pieces (violating the
        //    Freedom to Move Rule)
        //
        //  Furthermore, any piece moved by the Pillbug may not be moved at all (directly or via
        //  Pillbug action) on the next player's turn.
        //  The Mosquito can mimic either the movement or special ability of the Pillbug, even when
        //  the Pillbug is immobile.

        let direct_moves = if self.immobilized_piece == Some(*pillbug_hex) {
            Either::Left(iter::empty())
        } else {
            Either::Right(self.queen_moves(pillbug_hex))
        };

        let mut special_ability_moves: Vec<Turn> = vec![];
        let free_spaces: Vec<_> = self.hive.unoccupied_neighbors(&pillbug_hex).collect();
        let above_pillbug = Hex {
            h: 1,
            ..*pillbug_hex
        };
        let piece_moved_last_turn = match self.last_turn {
            Some(Move { to, .. }) => Some(to),
            _ => None,
        };

        for neighbor in self.hive.topmost_occupied_neighbors(pillbug_hex) {
            // Cannot move a piece that is in a stack
            if neighbor.h != 0 {
                continue;
            }
            // Cannot move a piece that just moved
            if Some(neighbor) == piece_moved_last_turn {
                continue;
            }

            // Verify that the move onto the pillbug is not blocked
            if !self.slide_is_allowed(&Hex { h: 1, ..neighbor }, &above_pillbug) {
                continue;
            }

            // The only move that could break the hive is the move up onto the pillbug, so we
            // only check that one
            if move_would_break_hive(&self.hive, &neighbor, &above_pillbug) {
                continue;
            }

            // Can move every neighbor to every unoccupied space
            for free_space in free_spaces.iter() {
                // Verify that the move down from the pillbug is not blocked
                let above_free_space = Hex {
                    h: 1,
                    ..*free_space
                };
                if !self.slide_is_allowed(&above_pillbug, &above_free_space) {
                    continue;
                }
                special_ability_moves.push(Move {
                    from: neighbor,
                    to: *free_space,
                    freezes_piece: true,
                })
            }
        }

        direct_moves.chain(special_ability_moves)
    }

    fn grasshopper_moves(&self, from: &Hex) -> impl Iterator<Item = Turn> {
        // Grasshopper either cannot move at all or can make all moves, so just check for hive
        // breakage once at the start
        if move_would_break_hive(&self.hive, from, &Hex{h: 100, ..*from}) {
            return Either::Left(iter::empty())
        }

        let mut allowed_jumps = vec![];
        let occupied_neighbors = self.hive.occupied_neighbors_at_same_level(from);
        for neighbor in occupied_neighbors {
            let direction = neighbor - *from;
            let unoccupied_spot = self
                .hive
                .next_unoccupied_spot_in_direction(from, &direction);
            allowed_jumps.push(unoccupied_spot);
        }
        Either::Right(allowed_jumps
            .into_iter()
            .map(|to| Move {
                from: *from,
                to,
                freezes_piece: false,
            }))
    }

    fn queen_moves(&self, from: &Hex) -> impl Iterator<Item = Turn> {
        if self.immobilized_piece == Some(*from) {
            return Either::Left(iter::empty());
        }

        Either::Right(
            self.allowed_slides(from, Some(from))
                .filter(|possible_move| !move_would_break_hive(&self.hive, from, possible_move))
                .map(|to| Move {
                    from: *from,
                    to,
                    freezes_piece: false,
                }),
        )
    }

    fn beetle_moves(&self, from: &Hex) -> impl Iterator<Item = Turn> {
        if self.immobilized_piece == Some(*from) {
            return Either::Left(iter::empty());
        }

        Either::Right(
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
                .map(|to| Move {
                    from: *from,
                    to,
                    freezes_piece: false,
                }),
        )
    }

    fn ladybug_moves(&self, from: &Hex) -> impl Iterator<Item = Turn> {
        if self.immobilized_piece == Some(*from) {
            return Either::Left(iter::empty());
        }

        let mut paths: Vec<Vec<Hex>> = vec![vec![*from]];
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
                        .filter(|dest| dest.base_level() != *from)
                        .filter(|dest| {
                            self.slide_is_allowed(
                                &Hex {
                                    h: dest.h,
                                    ..*current
                                },
                                dest,
                            )
                        })
                        .filter(|dest| !(i == 1 && move_would_break_hive(&self.hive, from, dest)))
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

        Either::Right(unique_destinations.into_iter().map(|to| Move {
            from: *from,
            to,
            freezes_piece: false,
        }))
    }

    fn spider_moves(&self, from: &Hex) -> impl Iterator<Item = Turn> {
        if self.immobilized_piece == Some(*from) {
            return Either::Left(iter::empty());
        }
        let mut paths: Vec<Vec<Hex>> = vec![vec![*from]];
        let mut new_paths: Vec<Vec<Hex>> = vec![];

        for i in 1..=3 {
            let first_move = i == 1;
            for path in paths.iter() {
                let current = path.last().unwrap();
                for dest in self.allowed_slides(current, Some(from)) {
                    if path.contains(&dest) {
                        continue;
                    }
                    // The spider can only break the hive on its first move as long as it is adjacent to
                    // something at each step. I think?!?!?!
                    if first_move && move_would_break_hive(&self.hive, current, &dest)
                        || !first_move
                            && self.slide_would_separate_self_from_hive(current, &dest, from)
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
        Either::Right(unique_destinations.into_iter().map(|to| Move {
            from: *from,
            to,
            freezes_piece: false,
        }))
    }

    fn ant_moves(&self, from: &Hex) -> impl Iterator<Item = Turn> {
        if self.immobilized_piece == Some(*from) {
            return Either::Left(iter::empty());
        }

        let mut current = *from;
        let mut allowed_moves = FxHashSet::default();
        let mut frontier: Vec<Hex> = vec![];
        frontier.push(current);

        let mut first_move = true;
        while !frontier.is_empty() {
            current = frontier.pop().unwrap();
            for dest in self.allowed_slides(&current, Some(from)) {
                if allowed_moves.contains(&dest) || *from == dest {
                    continue;
                }
                // The ant can only break the hive on its first move as long as it is adjacent to
                // something at each step. I think?!?!?!
                if first_move && move_would_break_hive(&self.hive, &current, &dest)
                    || !first_move
                        && self.slide_would_separate_self_from_hive(&current, &dest, from)
                {
                    continue;
                }
                allowed_moves.insert(dest);
                frontier.push(dest);
            }
            first_move = false;
        }

        Either::Right(allowed_moves.into_iter().map(|to| Move {
            from: *from,
            to,
            freezes_piece: false,
        }))
    }

    fn mosquito_moves(&self, start: &Hex) -> impl Iterator<Item = Turn> {
        let immobilized = self.immobilized_piece == Some(*start);

        let adjacent_bugs: Vec<_> = self
            .hive
            .topmost_occupied_neighbors(start)
            .map(|hex| self.hive.map.get(&hex).unwrap().bug)
            // Not allowed to copy other mosquitos
            .filter(|bug| *bug != Bug::Mosquito)
            // If immobilized, can only copy the pillbug push moves
            .filter(|bug| !immobilized || *bug == Bug::Pillbug)
            .collect();

        let mut turns: FxHashSet<Turn> = FxHashSet::default();
        for bug in adjacent_bugs {
            turns.extend(self.moves_for_tile(bug, start))
        }

        turns.into_iter()
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
        // are blocking the slide. For example in this board:
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
        let mut allowed_slides: Vec<Hex> = vec![];
        let mut previous_added = false;

        for (i, hex) in neighbors.iter().enumerate() {
            if self.hive.is_occupied(hex) && Some(hex) != ignore_hex {
                empty_seen = 0;
            } else {
                if empty_seen > 0 {
                    allowed_slides.push(*hex);
                    if !previous_added {
                        allowed_slides.push(neighbors[i - 1]);
                    }
                    previous_added = true;
                } else {
                    previous_added = false;
                }
                empty_seen += 1;
            }
        }

        let first = &neighbors[0];
        let second = &neighbors[1];
        let last = &neighbors[5];
        if !self.hive.is_occupied(first) && !self.hive.is_occupied(&last) {
            if !previous_added {
                allowed_slides.push(*last);
            }
            if self.hive.is_occupied(second) {
                allowed_slides.push(*first);
            }
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
                Move { to, .. } => {
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
            .turns()
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

    fn assert_pillbug_pushes(moves: &str) {
        let mut moves_map = parse_hex_map_string(moves).unwrap();
        let push_target_hex = moves_map
            .iter()
            .find(|(_, char)| *char == "&")
            .map(|(hex, _)| *hex)
            .unwrap();

        moves_map.insert(push_target_hex, "a".into());

        _assert_moves(&moves_map, push_target_hex, true)
    }

    fn assert_moves(moves: &str) {
        let moves_map = parse_hex_map_string(moves).unwrap();
        let (from, _) = moves_map
            .iter()
            .find(|(_, token)| token.chars().next().unwrap().is_uppercase())
            .unwrap();

        _assert_moves(&moves_map, *from, false)
    }

    fn _assert_moves(moves_map: &FxHashMap<Hex, String>, for_hex: Hex, freezes_piece: bool) {
        let mut expected_moves: Vec<Turn> = moves_map
            .iter()
            .filter(|(_, token)| *token == "*")
            .map(|(hex, _)| Move {
                from: for_hex,
                to: *hex,
                freezes_piece,
            })
            .collect();

        let hex_map: FxHashMap<Hex, String> = moves_map
            .into_iter()
            .filter(|(_, token)| *token != "*")
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect();
        let hive = Hive::from_hex_map(&hex_map).unwrap();
        let game = Game::from_hive_with_reserves(hive, Color::White, vec![], vec![]);

        let mut actual_moves: Vec<Turn> = game.turns().collect();

        actual_moves.retain(|turn| match turn {
            Move { from, .. } => *from == for_hex,
            _ => false,
        });

        expected_moves.sort();
        actual_moves.sort();

        if expected_moves != actual_moves {
            let expected_moves_map = turns_to_string(&hex_map, expected_moves);
            let actual_moves_map = turns_to_string(&hex_map, actual_moves);
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
    fn test_must_place_queen_by_turn_four() {
        let hex_map = parse_hex_map_string(
            r#"
            .  A  .
             A  A  .
            .  .  .
        "#,
        )
        .unwrap();

        let hive = Hive::from_hex_map(&hex_map).unwrap();
        let game = Game::from_hive(hive, Color::White);

        let moves = game.turns();
        let (queen_placements, non_queen_placements): (Vec<Turn>, Vec<Turn>) =
            moves.partition(|mv| {
                matches!(
                    mv,
                    Placement {
                        tile: Tile {
                            bug: Bug::Queen,
                            ..
                        },
                        ..
                    }
                )
            });

        assert!(queen_placements.len() > 0);
        assert_eq!(non_queen_placements.len(), 0);
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
    fn test_queen_move_can_escape_semicircle_in_top() {
        assert_moves(
            r#"
            .  *  *
             a  Q  a
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

    #[test]
    fn test_pillbug_can_use_special_ability() {
        assert_pillbug_pushes(
            r#"
        .  *  *
         *  P  *
        .  &  *
        "#,
        );
    }

    #[test]
    fn test_pillbug_can_slide() {
        assert_moves(
            r#"
            .  .  .
             .  q  *
            .  *  P
        "#,
        )
    }

    #[test]
    fn test_pillbug_cannot_pull_through_blocked_gap() {
        assert_pillbug_pushes(
            r#"
        Layer 0
        .  .  .
         .  P  a
        .  a  &
        Layer 1
        .  .  .
         .  .  b
        .  b  .
        "#,
        )
    }

    #[test]
    fn test_pillbug_cannot_push_through_blocked_gap() {
        assert_pillbug_pushes(
            r#"
        Layer 0
        .  .  a
         a  P  *
        .  *  &
        Layer 1
        .  .  b
         b  .  .
        .  .  .
        "#,
        )
    }

    #[test]
    fn test_pillbug_cannot_move_piece_that_just_moved() {
        let hex_map = parse_hex_map_string(
            r#"
        .  .  .
         q  Q  .
        .  .  P
        "#,
        )
        .unwrap();
        let hive = Hive::from_hex_map(&hex_map).unwrap();
        let mut game = Game::from_hive(hive, Color::Black);

        // Do this move
        // .  .  .
        //  _  Q  .
        // .  q  P
        game = game.with_turn_applied(Move {
            from: Hex { q: 0, r: 1, h: 0 },
            to: Hex { q: 0, r: 2, h: 0 },
            freezes_piece: false,
        });

        // Find all the moves that move the black queen (at q: 0, r: 2)
        let moves = game
            .pillbug_moves(&Hex { q: 1, r: 2, h: 0 })
            .filter(|turn| match turn {
                Move {
                    from: Hex { q: 0, r: 2, h: 0 },
                    ..
                } => true,
                _ => false,
            });

        // There shouldn't be any
        assert_eq!(moves.count(), 0);
    }

    #[test]
    fn test_cannot_move_piece_just_moved_by_pillbug() {
        let hex_map = parse_hex_map_string(
            r#"
        .  .  .
         .  P  .
        .  q  Q
        "#,
        )
        .unwrap();

        let hive = Hive::from_hex_map(&hex_map).unwrap();
        let mut game = Game::from_hive(hive, Color::White);

        // Move black queen from bottom left of pillbug to top right:
        // .  .  q
        //  .  P  .
        // .  _  Q
        game = game.with_turn_applied(Move {
            from: Hex { q: 0, r: 2, h: 0 },
            to: Hex { q: 2, r: 0, h: 0 },
            freezes_piece: true,
        });

        // Black should not be able to move anything
        assert_eq!(game.moves().count(), 0);
    }

    #[test]
    fn test_pillbug_can_use_ability_when_frozen() {
        let hex_map = parse_hex_map_string(
            r#"
            .  Q  .
             p  q  .
            .  P  .
            "#,
        )
        .unwrap();

        let hive = Hive::from_hex_map(&hex_map).unwrap();
        let mut game = Game::from_hive(hive, Color::White);

        // Use white pillbug to move black pillbug, freezing it
        // .  Q  .
        //  _  q  .
        // .  P  p
        game = game.with_turn_applied(Move {
            from: Hex { q: 0, r: 1, h: 0 },
            to: Hex { q: 1, r: 2, h: 0 },
            freezes_piece: true,
        });

        // Use black pillbug to move white pillbug, even though the black pillbug is frozen
        // .  Q  .
        //  .  q  P
        // .  _  p
        assert!(game.turn_is_valid(Move {
            from: Hex { q: 0, r: 2, h: 0 },
            to: Hex { q: 2, r: 1, h: 0 },
            freezes_piece: true
        }))
    }

    #[test]
    fn test_mosquito_can_use_pillbug_ability_even_if_pillbug_is_frozen() {
        let hex_map = parse_hex_map_string(
            r#"
        .  P  .
         p  q  Q
        .  .  M
        "#,
        )
        .unwrap();

        let hive = Hive::from_hex_map(&hex_map).unwrap();
        let mut game = Game::from_hive(hive, Color::Black);

        // Use black pillbug to move white pillbug to the hex next to the white mosquito
        // .  _  .
        //  p  q  Q
        // .  P  M
        game = game.with_turn_applied(Move {
            from: Hex { q: 1, r: 0, h: 0 },
            to: Hex { q: 0, r: 2, h: 0 },
            freezes_piece: true,
        });

        // Make sure we can move the white pillbug to the other side of the white mosquito by having
        // the white mosquito copy the white pillbug's ability
        // .  .  .  .
        //  p  q  _  .
        // .  P  M  Q
        assert!(game.turn_is_valid(Move {
            from: Hex { q: 2, r: 1, h: 0 },
            to: Hex { q: 2, r: 2, h: 0 },
            freezes_piece: true,
        }));
    }
}
