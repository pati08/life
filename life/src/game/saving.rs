use super::GameState;
use serde::{Deserialize, Serialize};
use std::{fs::File, io::Read, path::PathBuf};
use vec2::Vector2;

/// A representation of a game save file. The saves are stored in memory unless
/// written to disk via `SaveFile::write_to_disk`.
pub struct SaveFile {
    /// A vector of the saves
    saves: Vec<SaveGame>,
    /// A write-only file handle for the saves file
    file: File,
}

impl SaveFile {
    /// Create a new `SaveFile` by reading a file from disk. Returns an error
    /// if the file does not exist.
    fn new_from_disk(filepath: PathBuf) -> Result<Self, anyhow::Error> {
        let data: Vec<SaveGame> = {
            let mut buf = String::new();
            File::open(&filepath)?.read_to_string(&mut buf)?;
            serde_json::from_str(&buf)?
        };
        let file = File::create(filepath)?;
        Ok(Self { saves: data, file })
    }

    /// Create a new `SaveFile` by creating a new file on the disk. Returns an
    /// error if the file already exists.
    fn new_and_new_file(filepath: PathBuf) -> Result<Self, anyhow::Error> {
        let file = File::create_new(filepath)?;
        Ok(Self {
            saves: Vec::new(),
            file,
        })
    }

    /// Creates a new `SaveFile`. Uses the existing file on disk if it exists
    /// or otherwise create a new one.
    pub fn new(filepath: PathBuf) -> Result<Self, anyhow::Error> {
        if let Ok(v) = Self::new_and_new_file(filepath.clone()) {
            Ok(v)
        } else {
            Self::new_from_disk(filepath)
        }
    }

    /// Write the savefile to the disk.
    pub fn write_to_disk(self) -> Result<(), serde_json::Error> {
        serde_json::to_writer_pretty(self.file, &self.saves)?;
        Ok(())
    }

    /// Add a game save to the file.
    pub fn add_save(&mut self, save: SaveGame) {
        self.saves.push(save);
    }

    /// Delete a save from the file at a given index. This is safe to perform on
    /// an index that is out of bounds. The function returns whether or not it
    /// removed a save.
    pub fn delete_save(&mut self, index: usize) -> bool {
        if self.saves.len() > index {
            self.saves.remove(index);
            true
        } else {
            false
        }
    }

    /// Get an iterator over the game saves the file contains
    pub fn saves_iter(&self) -> impl Iterator<Item = &SaveGame> {
        self.saves.iter()
    }

    /// Get a reference to the save at a particular index
    pub fn save_at(&self, index: usize) -> Option<&SaveGame> {
        self.saves.get(index)
    }
}

#[derive(Serialize, Deserialize)]
/// A record of a game that can be restored.
pub struct SaveGame {
    living_cells: Vec<Vector2<i32>>,
    grid_size: f32,
    pan_position: Vector2<f64>,
    created: chrono::DateTime<chrono::Local>,
    name: String,
}

impl SaveGame {
    pub fn new(game_state: &GameState, name: String) -> Self {
        Self {
            living_cells: game_state.living_cells.iter().cloned().collect(),
            grid_size: game_state.grid_size,
            pan_position: game_state.pan_position,
            created: chrono::Local::now(),
            name,
        }
    }
}
