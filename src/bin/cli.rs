use chive::engine::game::Game;
use chive::engine::hive::{Color, Hive};

use chive::engine::ai::Ai;
use std::time::Duration;

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
    let pondering_time = Duration::from_secs(10);
    let mut ai = Ai::new(pondering_time, pondering_time * 3);
    let mut game = start;
    while let Ok(turn) = ai.choose_turn(&game) {
        game = game.with_turn_applied(turn);
        println!("{}", game.hive);
    }
    println!("{}", game.hive);
}
