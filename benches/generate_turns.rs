use chive::engine::game::Game;
use criterion::{Criterion, criterion_group, criterion_main};

fn complex_game() -> Game {
    let map_str = r#"
        Layer 0
        .  .  .  A  .  .
         .  s  b  a  .  .
        .  G  Q  B  .  .
         .  m  q  g  S  .
        .  .  L  P  .  .
         .  .  M  p  .  .
        Layer 1
        .  .  .  .  .  .
         .  .  .  .  .  .
        .  .  .  b  .  .
         .  .  .  .  .  .
        .  .  .  .  .  .
        "#;

    Game::from_map_str(map_str).unwrap()
}

fn mid_game() -> Game {
    let map_str = r#"
        Layer 0
        .  A  .
         .  Q  .
        .  q  a
        "#;
    Game::from_map_str(map_str).unwrap()
}

fn high_density_game() -> Game {
    // A very crowded board where many pieces are blocked or have many neighbors to check
    let map_str = r#"
        Layer 0
        .  A  G  S  .
         B  Q  M  L  .
        .  q  a  b  g
         s  p  l  m  .
        .  .  P  .  .
        "#;
    Game::from_map_str(map_str).unwrap()
}

fn beetle_stack_game() -> Game {
    // Beetles stacked on top of each other and other pieces
    let map_str = r#"
        Layer 0
        .  B  .
         Q  q  .
        .  .  .
        Layer 1
        .  b  .
         .  B  .
        .  .  .
        Layer 2
        .  .  .
         .  b  .
        .  .  .
        "#;
    Game::from_map_str(map_str).unwrap()
}

fn bench_generate_turns(c: &mut Criterion) {
    let mut group = c.benchmark_group("generate_turns");

    let games = [
        ("complex", complex_game()),
        ("mid", mid_game()),
        ("high_density", high_density_game()),
        ("beetle_stack", beetle_stack_game()),
    ];

    for (name, game) in games.iter() {
        group.bench_with_input(format!("turns {}", name), game, |b, g| {
            b.iter(|| {
                g.turns().collect::<Vec<_>>()
            })
        });
    }

    group.finish();
}

criterion_group!(benches, bench_generate_turns);
criterion_main!(benches);
