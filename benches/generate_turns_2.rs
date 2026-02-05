use gungraun::Callgrind;
use chive::engine::game::Game;
use gungraun::{library_benchmark, library_benchmark_group, main, EventKind, LibraryBenchmarkConfig};
use std::hint::black_box;

const EARLY_GAME: &'static str = r#"
.  A  .
 .  Q  .
.  q  a
"#;

const COMPLEX_GAME: &'static str = r#"
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

const HIGH_DENSITY_GAME: &'static str = r#"
.  A  G  S  .
 B  Q  M  L  .
.  q  a  b  g
 s  p  l  m  .
.  .  P  .  .
"#;

const BEETLE_STACK_GAME: &'static str = r#"
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


fn load_game(game: &str) -> Game {
    Game::from_map_str(game).unwrap()
}

#[library_benchmark(setup=load_game)]
#[bench::early(EARLY_GAME)]
#[bench::complex(COMPLEX_GAME)]
#[bench::high_density(HIGH_DENSITY_GAME)]
#[bench::beetle_stack(BEETLE_STACK_GAME)]
fn bench_turns(game: Game) -> usize {
    black_box(game.turns().count())
}

library_benchmark_group!(
    name = bench_turns_group;
    benchmarks = bench_turns
);

main!(
    config = LibraryBenchmarkConfig::default()
        .tool(Callgrind::default()
            .soft_limits([(EventKind::Ir, 5.0)])
        );
    library_benchmark_groups = bench_turns_group
);
