use crate::engine::bug::Bug;
use crate::engine::game::{Game, GameResult, Turn};
use minimax::{
    Evaluation, Evaluator, IterativeOptions, ParallelOptions, ParallelSearch, Strategy, Winner,
};
use rustc_hash::FxHashMap;
use std::time::Duration;
use strum::Display;
use thiserror::Error;
use AiError::RanOutOfTime;

#[derive(Error, Debug, Display)]
pub enum AiError {
    RanOutOfTime,
}

pub struct Ai {
    default_pondering_time: Duration,
    max_pondering_time: Duration,
    strategy: ParallelSearch<PiecesAroundQueenAndAvailableMoves>,
}

impl Ai {
    pub fn new(default_pondering_time: Duration, max_pondering_time: Duration) -> Ai {
        Ai {
            default_pondering_time,
            max_pondering_time,
            strategy: ParallelSearch::new(
                PiecesAroundQueenAndAvailableMoves {
                    piece_around_queen_value: 100,
                    available_move_value: 1,
                },
                IterativeOptions::new(),
                ParallelOptions::new(),
            ),
        }
    }

    pub fn choose_turn(&mut self, game: &Game) -> Result<Turn, AiError> {
        self.strategy.set_timeout(self.default_pondering_time);
        if let Some(turn) = self.strategy.choose_move(game) {
            Ok(turn)
        } else {
            self.strategy
                .set_timeout(self.max_pondering_time - self.default_pondering_time);
            self.strategy.choose_move(game).ok_or(RanOutOfTime)
        }
    }
}

struct HiveGame;

impl minimax::Game for HiveGame {
    type S = Game;
    type M = Turn;

    fn generate_moves(state: &Self::S, moves: &mut Vec<Self::M>) {
        moves.extend(state.turns())
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
    pub piece_around_queen_value: i16,
    pub available_move_value: i16,
}

impl Default for PiecesAroundQueenAndAvailableMoves {
    fn default() -> Self {
        Self {
            piece_around_queen_value: 100,
            available_move_value: 1,
        }
    }
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
        let active_player_available_moves = s.turns().count() as i16;
        (inactive_player_pieces_around_queen - active_player_pieces_around_queen)
            * self.piece_around_queen_value
            + active_player_available_moves * self.available_move_value
    }
}
