use crate::AppError::AiError;
use chive::engine::ai::PiecesAroundQueenAndAvailableMoves;
use chive::engine::bug::Bug;
use chive::engine::game::{Game, GameResult, Turn};
use chive::engine::hive::{Color, Tile};
use chive::engine::row_col;
use chive::engine::row_col::{RowCol, RowColDimensions};
use itertools::Itertools;
use minimax::{IterativeOptions, ParallelOptions, ParallelSearch, Strategy};
use ratatui::crossterm::event;
use ratatui::crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::prelude::Direction;
use ratatui::style::Stylize;
use ratatui::text::{Line, Span, Text};
use ratatui::{DefaultTerminal, Frame};
use rustc_hash::FxHashSet;
use std::io;
use std::time::Duration;
use thiserror::Error;

struct App {
    game: Game,
    ai: ParallelSearch<PiecesAroundQueenAndAvailableMoves>,
    cursor_pos: RowCol,
    player_color: Color,
    selected_piece: Option<RowCol>,
    active_height: i32,
}

#[derive(Error, Debug)]
pub enum AppError {
    #[error("Failed to interact with terminal")]
    IoError(#[from] io::Error),
    #[error("AI Failed to find a valid move")]
    AiError(String),
}

impl App {
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

    fn game_result(&self) -> Option<String> {
        match self.game.game_result() {
            GameResult::None => None,
            GameResult::Draw => Some(format!("Draw!\n{}", self.game.hive)),
            GameResult::Winner { color } => Some(format!("{} Won!\n{}", color, self.game.hive)),
        }
    }

    fn run(mut self, mut terminal: DefaultTerminal) -> Result<String, AppError> {
        loop {
            if let Some(result) = self.game_result() {
                return Ok(result);
            }
            terminal.draw(|frame| self.draw(frame))?;
            if self.game.active_player != self.player_color {
                if let Some(turn) = self.ai.choose_move(&self.game) {
                    self.game = self.game.with_turn_applied(turn);
                    if let Some(result) = self.game_result() {
                        return Ok(result);
                    }
                } else {
                    self.ai.set_timeout(Duration::from_secs(10));
                    let turn = self.ai.choose_move(&self.game).unwrap();
                    self.ai.set_timeout(Duration::from_millis(15));
                    self.game = self.game.with_turn_applied(turn);
                    if let Some(result) = self.game_result() {
                        return Ok(result);
                    }
                }
                terminal.draw(|frame| self.draw(frame))?;
            }
            let dims = self.board_dimensions();
            if let Some(key) = event::read()?.as_key_press_event() {
                match key {
                    KeyEvent {
                        code: KeyCode::Left,
                        ..
                    } => {
                        self.cursor_pos.col =
                            (self.cursor_pos.col - 1).clamp(dims.col_min, dims.col_max);
                    }
                    KeyEvent {
                        code: KeyCode::Right,
                        ..
                    } => {
                        self.cursor_pos.col =
                            (self.cursor_pos.col + 1).clamp(dims.col_min, dims.col_max);
                    }
                    KeyEvent {
                        code: KeyCode::Up, ..
                    } => {
                        self.cursor_pos.row =
                            (self.cursor_pos.row - 1).clamp(dims.row_min, dims.row_max);
                    }
                    KeyEvent {
                        code: KeyCode::Down,
                        ..
                    } => {
                        self.cursor_pos.row =
                            (self.cursor_pos.row + 1).clamp(dims.row_min, dims.row_max);
                    }
                    KeyEvent {
                        code: KeyCode::Esc, ..
                    } => {
                        self.selected_piece = None;
                    }
                    KeyEvent {
                        code: KeyCode::Enter,
                        ..
                    } => {
                        if !self.selected_piece.is_some() {
                            self.selected_piece = Some(self.cursor_pos)
                        } else {
                            if self.selected_piece == Some(self.cursor_pos) {
                                self.selected_piece = None;
                            } else {
                                let turn = Turn::Move {
                                    from: self.selected_piece.unwrap().to_hex(),
                                    to: self.cursor_pos.to_hex(),
                                };
                                if self.game.turn_is_valid(turn) {
                                    self.game = self.game.with_turn_applied(turn);
                                    self.selected_piece = None
                                }
                            }
                        }
                    }
                    KeyEvent {
                        code: KeyCode::PageUp,
                        ..
                    } => {
                        self.active_height = (self.active_height + 1).clamp(0, dims.height_max);
                        self.cursor_pos = RowCol {
                            height: self.active_height,
                            ..self.cursor_pos
                        };
                    }
                    KeyEvent {
                        code: KeyCode::PageDown,
                        ..
                    } => {
                        self.active_height = (self.active_height - 1).clamp(0, dims.height_max);
                        self.cursor_pos = RowCol {
                            height: self.active_height,
                            ..self.cursor_pos
                        };
                    }
                    KeyEvent {
                        code: KeyCode::End, ..
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

        let white_pieces = self
            .game
            .white_reserve
            .iter()
            .map(|b| b.to_string())
            .join(", ");
        let black_pieces = self
            .game
            .black_reserve
            .iter()
            .map(|b| b.to_string())
            .join(", ");
        frame.render_widget(
            Text::raw(format!("White Reserve: {white_pieces}")),
            layout[0],
        );
        frame.render_widget(
            Text::raw(format!("Black Reserve: {black_pieces}")),
            layout[1],
        );
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
        frame.render_widget(stack_text, layout[2]);
        self.draw_map(frame, &layout[3])
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

        let starred_locations = if let Some(piece) = self.selected_piece {
            self.game
                .valid_destinations_for_piece(&piece.to_hex())
                .map(|hex| RowCol::from_hex(&hex))
                .collect()
        } else {
            FxHashSet::default()
        };

        let default = Text::raw(".");
        for (i, cell) in cells.enumerate() {
            let visual_row = (i as i32 / board_dimensions.width()) - 1;
            let visual_col = (i as i32 % board_dimensions.width()) - 1;
            let row = map_dimensions.row_min + visual_row;
            let col = map_dimensions.col_min + visual_col;
            let row_col = RowCol {
                row,
                col,
                height: self.active_height,
            };
            let hex = row_col.to_hex();

            if self.cursor_pos == row_col {
                frame.set_cursor_position(cell)
            }

            if starred_locations.contains(&row_col) {
                frame.render_widget("*", cell);
            } else {
                let mut text = self
                    .game
                    .hive
                    .map
                    .get(&hex)
                    .map(|t| {
                        if t.color == Color::White {
                            Text::raw(t.to_string()).black().on_white()
                        } else {
                            Text::raw(t.to_string()).white().on_black()
                        }
                    })
                    .unwrap_or(default.clone());
                if Some(row_col) == self.selected_piece {
                    text = text.slow_blink();
                }
                if self.game.hive.stack_height(&hex) > 1 {
                    text = text.underlined()
                }
                frame.render_widget(text, cell);
            }
        }
    }
}

fn main() {
    let terminal = ratatui::init();
    let mut strategy = ParallelSearch::new(
        PiecesAroundQueenAndAvailableMoves {
            piece_around_queen_value: 100,
            available_move_value: 1,
        },
        IterativeOptions::new(),
        ParallelOptions::new().with_background_pondering(),
    );
    strategy.set_timeout(Duration::from_millis(15));
    let app = App {
        game: Default::default(),
        ai: strategy,
        cursor_pos: Default::default(),
        player_color: Default::default(),
        selected_piece: None,
        active_height: 0,
    };
    let result = app.run(terminal);
    ratatui::restore();
    match result {
        Ok(final_board_state) => {
            println!("{}", final_board_state);
        }
        Err(AiError(final_board_state)) => {
            println!("{}", final_board_state);
        }
        _ => {
            println!("{:?}", result)
        }
    }
}
