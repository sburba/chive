mod bug;
mod game;
mod hex;
mod hive;
mod parse;
mod pathfinding;
mod zobrist;

use crate::bug::Bug;
use crate::game::{Game, GameResult, Turn};
use crate::hive::{Color, Hive};
use minimax::{Evaluation, Evaluator, IterativeOptions, ParallelOptions, Strategy, Winner};
use std::time::Duration;
use rustc_hash::FxHashMap;

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

    fn zobrist_hash(state: &Self::S) -> u64 {
        state.zobrist_hash.value()
    }
}

#[derive(Clone)]
struct PiecesAroundQueenAndAvailableMoves {
    piece_around_queen_cost: i16,
    available_move_cost: i16,
}

impl Evaluator for PiecesAroundQueenAndAvailableMoves {
    type G = HiveGame;

    fn evaluate(&self, s: &<Self::G as minimax::Game>::S) -> Evaluation {
        let statuses: FxHashMap<_, _> = s
            .hive
            .map
            .iter()
            .filter(|(_, tile)| tile.bug == Bug::Queen)
            .map(|(hex, tile)| {
                (
                    tile.color,
                    s.hive.occupied_neighbors_at_same_level(hex).count() as i16,
                )
            })
            .collect();

        let inactive_player_pieces_around_queen =
            *statuses.get(&s.active_player.opposite()).unwrap_or(&0);
        let active_player_pieces_around_queen = *statuses.get(&s.active_player).unwrap_or(&0);
        let active_player_available_moves = s.valid_turns().len() as i16;
        (inactive_player_pieces_around_queen - active_player_pieces_around_queen)
            * self.piece_around_queen_cost
            + active_player_available_moves * self.available_move_cost
    }
}

fn main() {
    let hive: Hive = r#"
            .  .  .  .
           .  .  .  .
            .  .  .  .
           .  .  .  .
        "#
    .parse()
    .unwrap();
    let start = Game::from_hive(hive, Color::White);

    println!("{}", start.hive);
    let mut strategy = minimax::ParallelSearch::new(
        PiecesAroundQueenAndAvailableMoves {
            piece_around_queen_cost: 100,
            available_move_cost: 1,
        },
        IterativeOptions::new(),
        ParallelOptions::new(),
    );
    strategy.set_timeout(Duration::from_secs(10));
    let mut game = start;
    for _ in 0..3 {
        game = game.with_turn_applied(strategy.choose_move(&game).unwrap())
    }
    // while let Some(best_move) = strategy.choose_move(&game) {
    //     game = game.with_turn_applied(best_move);
    // }
    println!("{:?}", game.hive);
}
