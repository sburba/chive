use crate::engine::game::Game;
use crate::engine::hive::{Color, Hive, HiveParseError};
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use thiserror::Error;

/// Custom error type for save/load operations
#[derive(Debug, Error)]
pub enum SaveGameError {
    #[error("Failed to create directory '{0}': {1}")]
    CreateDirError(String, #[source] io::Error),

    #[error("Failed to create file '{0}': {1}")]
    CreateFileError(String, #[source] io::Error),

    #[error("Failed to write to file '{0}': {1}")]
    WriteFileError(String, #[source] io::Error),

    #[error("Failed to read from file '{0}': {1}")]
    ReadFileError(String, #[source] io::Error),

    #[error("System time error while generating filename: {0}")]
    TimeError(#[from] std::time::SystemTimeError),

    #[error("Failed to parse active player: {0}")]
    ParseColorError(String),

    #[error("Failed to parse game")]
    ParseGameError(#[from] HiveParseError),
}

pub fn save_game(game: &Game, directory_path: impl AsRef<Path>) -> Result<PathBuf, SaveGameError> {
    let dir_path = directory_path.as_ref();

    // Ensure directory exists
    fs::create_dir_all(dir_path)
        .map_err(|e| SaveGameError::CreateDirError(dir_path.display().to_string(), e))?;

    let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
    let mut filename = format!("save_{}.txt", timestamp);
    let mut file_path = dir_path.join(&filename);

    // Avoid conflicts
    let mut counter = 1;
    while file_path.exists() {
        filename = format!("save_{}_({}).txt", timestamp, counter);
        file_path = dir_path.join(&filename);
        counter += 1;
    }

    // Write file: first line = active player, rest = game state
    let mut file = File::create(&file_path)
        .map_err(|e| SaveGameError::CreateFileError(file_path.display().to_string(), e))?;
    let contents = format!("ActivePlayer: {}\n{}", game.active_player, game.hive);
    file.write_all(contents.as_bytes())
        .map_err(|e| SaveGameError::WriteFileError(file_path.display().to_string(), e))?;

    Ok(file_path)
}

pub fn load_game(file_path: impl AsRef<Path>) -> Result<Game, SaveGameError> {
    let path = file_path.as_ref();
    let mut contents = String::new();

    File::open(path)
        .map_err(|e| SaveGameError::ReadFileError(path.display().to_string(), e))?
        .read_to_string(&mut contents)
        .map_err(|e| SaveGameError::ReadFileError(path.display().to_string(), e))?;

    let mut lines = contents.lines();

    // Parse first line for active player
    let first_line = lines
        .next()
        .ok_or_else(|| SaveGameError::ParseColorError("Missing active player line".to_string()))?;
    let color_str = first_line
        .strip_prefix("ActivePlayer:")
        .ok_or_else(|| {
            SaveGameError::ParseColorError("Invalid active player line format".to_string())
        })?
        .trim();
    let active_player = color_str
        .parse::<Color>()
        .map_err(|e| SaveGameError::ParseColorError(e.to_string()))?;

    // Remaining lines form the game state
    let game_data: String = lines.collect::<Vec<_>>().join("\n");
    let hive: Hive = game_data.parse()?;
    let game = Game::from_hive(hive, active_player);

    Ok(game)
}

pub fn list_save_games(directory_path: impl AsRef<Path>) -> Result<Vec<String>, SaveGameError> {
    let dir_path = directory_path.as_ref();

    let mut saves = Vec::new();

    let entries = fs::read_dir(dir_path)
        .map_err(|e| SaveGameError::ReadFileError(dir_path.display().to_string(), e))?;

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_file()
            && let Some(ext) = path.extension()
            && ext == "txt"
        {
            saves.push(path.file_name().unwrap().display().to_string());
        }
    }

    // Optional: sort by modified time or name
    saves.sort();

    Ok(saves)
}
