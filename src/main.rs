mod bug;
mod game;
mod hex;
mod hive;
mod parse;
mod pathfinding;

use crate::bug::Bug;
use crate::game::{Game, GameResult, Turn};
use crate::hive::Color;
use hex::Hex;
use hive::Hive;
use minimax::{Evaluation, Evaluator, IterativeOptions, ParallelOptions, Strategy, Winner};
use std::collections::HashMap;

struct Point {
    x: i32,
    y: i32,
}

const SIZE: f64 = 50f64;
const THREE_HALVES: f64 = 3f64 / 2f64;
const SQRT_3: f64 = 1.732050807568877293527446341505872367_f64;
const HALF_SQRT_3: f64 = SQRT_3 / 2f64;

fn cube_coordinate_to_point(cube: Hex) -> Point {
    let x = THREE_HALVES * cube.q as f64;
    let y = HALF_SQRT_3 * cube.q as f64 + SQRT_3 * cube.r as f64;
    Point {
        x: (x * SIZE) as i32,
        y: (y * SIZE) as i32,
    }
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

struct HiveGame;

impl minimax::Game for HiveGame {
    type S = Game;
    type M = Turn;

    fn generate_moves(state: &Self::S, moves: &mut Vec<Self::M>) {
        moves.extend(state.valid_turns())
    }

    fn apply(state: &mut Self::S, m: Self::M) -> Option<Self::S> {
        Some(state.with_turn_applied(m))
    }

    fn get_winner(state: &Self::S) -> Option<Winner> {
        match state.game_result() {
            GameResult::None => None,
            GameResult::Draw => Some(Winner::Draw),
            GameResult::Winner { color } => {
                if color == state.active_player {
                    Some(Winner::PlayerToMove)
                } else {
                    Some(Winner::PlayerJustMoved)
                }
            }
        }
    }

    fn zobrist_hash(_state: &Self::S) -> u64 {
        todo!()
    }
}

#[derive(Clone)]
struct NumberOfPiecesAroundQueen;
impl Evaluator for NumberOfPiecesAroundQueen {
    type G = HiveGame;

    fn evaluate(&self, s: &<Self::G as minimax::Game>::S) -> Evaluation {
        let statuses: HashMap<_, _> = s
            .hive
            .map
            .iter()
            .filter(|(_, tile)| tile.bug == Bug::Queen)
            .map(|(hex, tile)| (tile.color, s.hive.occupied_neighbors(hex).count() as i16))
            .collect();

        let inactive_player_pieces_around_queen = *statuses.get(&s.active_player.opposite()).unwrap_or(&0);
        let active_player_pieces_around_queen = *statuses.get(&s.active_player).unwrap_or(&0);
        let active_player_available_moves = s.valid_turns().len() as i16;
        let score = (inactive_player_pieces_around_queen - active_player_pieces_around_queen) * 100 + active_player_available_moves;
        score
    }
}

fn main() {
    // let start = Game {
    //     hive: Hive {
    //         map: HashMap::new(),
    //     },
    //     white_reserve: vec![
    //         Bug::Queen,
    //         Bug::Ant,
    //         Bug::Ant,
    //         Bug::Ant,
    //         Bug::Beetle,
    //         Bug::Beetle,
    //         Bug::Grasshopper,
    //         Bug::Grasshopper,
    //         Bug::Grasshopper,
    //         Bug::Spider,
    //         Bug::Spider,
    //     ],
    //     black_reserve: vec![
    //         Bug::Queen,
    //         Bug::Ant,
    //         Bug::Spider,
    //         Bug::Spider,
    //     ],
    //     active_player: Color::White,
    // };
    // let mut strategy = minimax::ParallelSearch::new(
    //     NumberOfPiecesAroundQueen {},
    //     IterativeOptions::new(),
    //     ParallelOptions::new(),
    // );
    let start = Game {
        hive: r#"
            .  .  .  .
           .  .  .  .
            .  .  .  .
           .  .  .  .
        "#
        .parse()
        .unwrap(),
        white_reserve: vec![
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
        ],
        black_reserve: vec![
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
        ],
        active_player: Color::White,
    };
    println!("{}", start.hive.to_string());
    let test = NumberOfPiecesAroundQueen {};
    let mut strategy = minimax::Negamax::new(NumberOfPiecesAroundQueen, 4);
    let mut game = start;
    let mut i = 1;
    while let Some(best_move) = strategy.choose_move(&game) {
        println!("Turn {}", i);
        println!("Score {}", test.evaluate(&game));
        game = game.with_turn_applied(best_move);
        println!("Move {:?}", best_move);
        println!("{}", game.hive);
        println!("=================");
        i = i + 1;
    }
    println!("{:?}", game.hive);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hex::{Direction, flat_distance, neighbor};
    use pretty_assertions::assert_eq;

    #[test]
    fn test_distance() {
        assert_eq!(
            flat_distance(&Hex { q: 0, r: 0, h: 0 }, &Hex { q: -1, r: 0, h: 0 }),
            1
        )
    }

    #[test]
    fn test_neighbor() {
        assert_eq!(
            neighbor(&Hex { q: 0, r: 0, h: 0 }, &Direction::UpLeft),
            Hex { q: 0, r: -1, h: 0 }
        )
    }

    #[test]
    fn test_shortest_path() {}
}
