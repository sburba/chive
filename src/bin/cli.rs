use chive::engine::game::Game;
use chive::engine::hive::{Color, Hive};

use minimax::{IterativeOptions, ParallelOptions, Strategy};
use std::time::Duration;
use chive::engine::ai::PiecesAroundQueenAndAvailableMoves;

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
            piece_around_queen_value: 100,
            available_move_value: 1,
        },
        IterativeOptions::new(),
        ParallelOptions::new(),
    );
    strategy.set_timeout(Duration::from_secs(10));
    let mut game = start;
    while let Some(best_move) = strategy.choose_move(&game) {
        game = game.with_turn_applied(best_move);
        println!("{}", game.hive);
    }
    println!("{}", game.hive);
}
