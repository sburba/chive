use minimax::{Evaluation, Evaluator, Winner};
use rustc_hash::FxHashMap;
use crate::engine::bug::Bug;
use crate::engine::game::{Game, GameResult, Turn};

pub struct HiveGame;

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
pub struct PiecesAroundQueenAndAvailableMoves {
    pub piece_around_queen_cost: i16,
    pub available_move_cost: i16,
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