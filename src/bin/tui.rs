use crate::AppError::AiError;
use chive::engine::ai::Ai;
use chive::engine::bug::Bug;
use chive::engine::game::{Game, GameResult, Turn};
use chive::engine::hex::Hex;
use chive::engine::hive::{Color, Tile};
use chive::engine::row_col::{RowCol, RowColDimensions};
use chive::engine::save_game::{list_save_games, load_game, save_game};
use chive::engine::{ai, row_col};
use clap::Parser;
use itertools::Itertools;
use ratatui::crossterm::event;
use ratatui::crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::prelude::Direction;
use ratatui::style::Stylize;
use ratatui::text::{Line, Span};
use ratatui::{DefaultTerminal, Frame};
use rustc_hash::FxHashSet;
use std::cmp::max;
use std::io;
use std::path::PathBuf;
use std::time::Duration;
use thiserror::Error;

struct App {
    game: Game,
    ai: Ai,
    cursor_pos: RowCol,
    player_color: Color,
    selected_pos: Option<RowCol>,
    last_ai_move_pos: Option<RowCol>,
}

#[derive(Error, Debug)]
pub enum AppError {
    #[error("Failed to interact with terminal")]
    IoError(#[from] io::Error),
    #[error("AI Failed to find a valid move")]
    AiError(#[from] ai::AiError),
}

fn tile_to_span<'a>(tile: Tile) -> Span<'a> {
    if tile.color == Color::White {
        Span::from(tile.to_string()).black().on_white()
    } else {
        Span::from(tile.to_string()).white().on_black()
    }
}

enum Dir {
    Left,
    Right,
    Up,
    Down,
}

// Add left to right, wrapping the value around to stay within min and max
fn wrapping_add(left: i32, right: i32, min: i32, max: i32) -> i32 {
    let range = max - min + 1;
    min + (left - min + right).rem_euclid(range)
}

impl App {
    fn last_affected_row_col(&self, turn: &Turn) -> Option<RowCol> {
        match turn {
            Turn::Placement { hex, tile: _ } => Some(RowCol::from_hex(hex)),
            Turn::Move { from: _, to } => Some(RowCol::from_hex(to)),
            Turn::Skip => self.last_ai_move_pos,
        }
    }
    fn board_dimensions(&self) -> RowColDimensions {
        let map_dimensions = row_col::dimensions(self.game.hive.to_hex_map().keys());
        RowColDimensions {
            row_min: map_dimensions.row_min - 1,
            row_max: map_dimensions.row_max + 1,
            col_min: map_dimensions.col_min - 1,
            col_max: map_dimensions.col_max + 1,
            height_min: 0,
            height_max: map_dimensions.height_max + 1,
        }
    }

    fn board_string(&self) -> String {
        self.game.hive.to_string()
    }

    fn game(&self) -> Game {
        self.game.clone()
    }

    fn game_result(&self) -> Option<String> {
        match self.game.game_result() {
            GameResult::None => None,
            GameResult::Draw => Some(format!("Draw!\n{}", self.game.hive)),
            GameResult::Winner { color } => Some(format!("{} Won!\n{}", color, self.game.hive)),
        }
    }

    fn move_cursor(&mut self, dir: Dir) {
        let dims = self.board_dimensions();
        match dir {
            Dir::Left => {
                self.cursor_pos.col =
                    wrapping_add(self.cursor_pos.col, -1, dims.col_min, dims.col_max);
            }
            Dir::Right => {
                self.cursor_pos.col =
                    wrapping_add(self.cursor_pos.col, 1, dims.col_min, dims.col_max);
            }
            Dir::Up => {
                self.cursor_pos.row =
                    wrapping_add(self.cursor_pos.row, -1, dims.row_min, dims.row_max);
            }
            Dir::Down => {
                self.cursor_pos.row =
                    wrapping_add(self.cursor_pos.row, 1, dims.row_min, dims.row_max);
            }
        }
    }

    fn run(&mut self, mut terminal: DefaultTerminal) -> Result<String, AppError> {
        loop {
            if let Some(result) = self.game_result() {
                return Ok(result);
            }
            terminal.draw(|frame| self.draw(frame))?;
            if self.game.active_player != self.player_color {
                let turn = self.ai.choose_turn(&self.game)?;
                self.last_ai_move_pos = self.last_affected_row_col(&turn);
                self.game = self.game.with_turn_applied(turn);
                if let Some(result) = self.game_result() {
                    return Ok(result);
                }
                terminal.draw(|frame| self.draw(frame))?;
            }

            if let Some(key) = event::read()?.as_key_press_event() {
                match key {
                    KeyEvent {
                        code: KeyCode::Left | KeyCode::Char('h'),
                        ..
                    } => self.move_cursor(Dir::Left),
                    KeyEvent {
                        code: KeyCode::Right | KeyCode::Char('l'),
                        ..
                    } => self.move_cursor(Dir::Right),
                    KeyEvent {
                        code: KeyCode::Up | KeyCode::Char('k'),
                        ..
                    } => self.move_cursor(Dir::Up),
                    KeyEvent {
                        code: KeyCode::Down | KeyCode::Char('j'),
                        ..
                    } => {
                        self.move_cursor(Dir::Down);
                    }
                    KeyEvent {
                        code: KeyCode::Esc, ..
                    } => self.selected_pos = None,
                    KeyEvent {
                        code: KeyCode::Enter,
                        ..
                    } => {
                        if self.selected_pos.is_none() {
                            self.selected_pos = self
                                .game
                                .hive
                                .topmost_occupied_hex(&self.cursor_pos.to_hex())
                                .filter(|hex| {
                                    self.game
                                        .hive
                                        .tile_at(hex)
                                        .is_some_and(|tile| tile.color == self.player_color)
                                })
                                .map(|hex: Hex| RowCol::from_hex(&hex))
                        } else if self.selected_pos == Some(self.cursor_pos) {
                            self.selected_pos = None;
                        } else {
                            let turn = Turn::Move {
                                from: self.selected_pos.unwrap().to_hex(),
                                to: self
                                    .game
                                    .hive
                                    .bottommost_unoccupied_hex(&self.cursor_pos.to_hex()),
                            };
                            if self.game.turn_is_valid(turn) {
                                self.game = self.game.with_turn_applied(turn);
                                self.selected_pos = None
                            }
                        }
                    }
                    KeyEvent {
                        code: KeyCode::F(1),
                        ..
                    } => return Ok(self.game.hive.to_string()),
                    KeyEvent {
                        code: KeyCode::Char(char),
                        ..
                    } => {
                        if self.game.active_player != self.player_color {
                            continue;
                        }

                        if let Ok(bug) = char.to_string().to_uppercase().parse::<Bug>() {
                            let turn = Turn::Placement {
                                hex: self.cursor_pos.to_hex(),
                                tile: Tile {
                                    bug,
                                    color: self.player_color,
                                },
                            };
                            if self.game.turn_is_valid(turn) {
                                self.game = self.game.with_turn_applied(turn);
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    fn draw(&self, frame: &mut Frame) {
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Min(3),
            ])
            .split(frame.area());

        self.draw_reserve(Color::White, frame, layout[0]);
        self.draw_reserve(Color::Black, frame, layout[1]);
        self.draw_stack(frame, layout[2]);
        self.draw_map(frame, &layout[3])
    }

    fn draw_reserve(&self, color: Color, frame: &mut Frame, area: Rect) {
        let (reserve, name) = if color == Color::White {
            (&self.game.white_reserve, "White")
        } else {
            (&self.game.black_reserve, "Black")
        };

        #[allow(unstable_name_collisions)]
        let pieces = reserve
            .iter()
            .map(|b| tile_to_span(Tile { bug: *b, color }))
            .intersperse(Span::from(", "));
        let reserve: Vec<Span> = [Span::from(format!("{name} Reserve: "))]
            .into_iter()
            .chain(pieces)
            .collect();
        frame.render_widget(Line::from(reserve), area);
    }

    fn draw_stack(&self, frame: &mut Frame, area: Rect) {
        let cursor_hex_pos = self.cursor_pos.to_hex();

        let mut spans: Vec<Span> = vec![Span::raw("Stack: ")];
        for (i, tile) in self.game.hive.stack_at(&cursor_hex_pos).enumerate() {
            if tile.color == Color::White {
                spans.push(Span::raw(tile.to_string()).black().on_white())
            } else {
                spans.push(Span::raw(tile.to_string()).white().on_black())
            }

            if i % 2 == 0 {
                spans.push(Span::raw(" "));
            }
        }
        let stack_text = Line::from(spans);
        frame.render_widget(stack_text, area);
    }

    fn draw_map(&self, frame: &mut Frame, area: &Rect) {
        let hex_map = self.game.hive.to_hex_map();
        let map_dimensions = row_col::dimensions(hex_map.keys());
        let board_dimensions = self.board_dimensions();
        let col_constraints = (0..board_dimensions.width()).map(|_| Constraint::Length(1));
        let row_constraints = (0..board_dimensions.height()).map(|_| Constraint::Length(1));
        let odd_horizontal = Layout::horizontal(col_constraints.clone()).spacing(2);
        let even_horizontal = Layout::horizontal(col_constraints)
            .spacing(2)
            .horizontal_margin(1);
        let vertical = Layout::vertical(row_constraints);
        let odd_first = board_dimensions.row_min & 1 == 1;

        let cells = area
            .layout_vec(&vertical)
            .into_iter()
            .enumerate()
            .flat_map(|(i, row)| {
                if (odd_first && i & 1 == 1) || !odd_first && i & 1 != 1 {
                    row.layout_vec(&odd_horizontal)
                } else {
                    row.layout_vec(&even_horizontal)
                }
            });

        let valid_move_positions = if let Some(piece) = self.selected_pos {
            self.game
                .valid_destinations_for_piece(&piece.to_hex())
                .map(|hex| RowCol::from_hex(&Hex { h: 0, ..hex }))
                .collect()
        } else {
            FxHashSet::default()
        };

        let default = Span::from(".");
        for (i, cell) in cells.enumerate() {
            let visual_row = (i as i32 / board_dimensions.width()) - 1;
            let visual_col = (i as i32 % board_dimensions.width()) - 1;
            let row = map_dimensions.row_min + visual_row;
            let col = map_dimensions.col_min + visual_col;
            let row_col = RowCol {
                row,
                col,
                height: 0,
            };
            let hex = row_col.to_hex();

            if self.cursor_pos == row_col {
                frame.set_cursor_position(cell)
            }

            let mut text = self
                .game
                .hive
                .top_tile_at(&hex)
                .map(tile_to_span)
                .unwrap_or(default.clone());
            if Some(row_col) == self.selected_pos {
                text = text.slow_blink();
            }
            if self.game.hive.stack_height(&hex) > 1 {
                text = text.underlined()
            }
            if valid_move_positions.contains(&row_col) {
                text = text.on_green();
            } else if Some(row_col) == self.last_ai_move_pos {
                text = text.on_magenta()
            }
            frame.render_widget(text, cell);
        }
    }
}

/// Play hive against the computer
///
/// - Arrow keys to move around
///
/// - First letter of the bug to place a bug
///
/// - Enter to select tile, enter again to move piece to cursor
///
/// - Escape to deselect
///
/// - f1 to quit
#[derive(Debug, Parser)]
pub struct Config {
    #[clap(value_parser = humantime::parse_duration, default_value = "5s")]
    #[arg(short, long)]
    pondering_time: Duration,

    #[clap(default_value = "chive-saves")]
    #[arg(long)]
    save_directory: PathBuf,

    #[arg(short = 's', long)]
    load_save_file: Option<PathBuf>,

    #[arg(short, long)]
    list_saves: bool,

    #[clap(default_value = "white")]
    #[arg(short = 'c', long)]
    player_color: Color,
}

fn main() {
    let args = Config::parse();
    if args.list_saves {
        let saves = list_save_games(args.save_directory).unwrap();
        println!("{}", saves.iter().join("\n"));
        return;
    }

    let game = if let Some(save) = args.load_save_file {
        load_game(
            [args.save_directory.clone(), save]
                .iter()
                .collect::<PathBuf>(),
        )
        .unwrap()
    } else {
        Default::default()
    };

    let terminal = ratatui::init();
    let pondering_time = args.pondering_time;
    let mut app = App {
        game,
        ai: Ai::new(
            pondering_time,
            max(pondering_time * 3, Duration::from_secs(5)),
        ),
        cursor_pos: Default::default(),
        player_color: args.player_color,
        selected_pos: None,
        last_ai_move_pos: None,
    };
    let result = app.run(terminal);
    ratatui::restore();
    match result {
        Ok(final_board_state) => {
            println!("{}", final_board_state);
            let game_path = save_game(&app.game(), args.save_directory).unwrap();
            println!("Saved game to {}", game_path.display());
        }
        Err(AiError(_)) => {
            println!("AI Failed to find move in time :(");
            println!("{}", app.board_string());
            let game_path = save_game(&app.game(), args.save_directory).unwrap();
            println!("Saved game to {}", game_path.display());
        }
        _ => {
            println!("{:?}", result);
            println!("{}", app.board_string());
            let game_path = save_game(&app.game(), args.save_directory).unwrap();
            println!("Saved game to {}", game_path.display());
        }
    }
}
