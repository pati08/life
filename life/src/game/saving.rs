use super::GameState;
use ordered_float::OrderedFloat;
use serde::{Deserialize, Serialize};
use std::{fs::File, hash::Hash, io::Read, path::PathBuf};
use vec2::Vector2;

/// A representation of a game save file. The saves are stored in memory unless
/// written to disk via `SaveFile::write_to_disk`.
pub struct SaveFile {
    /// A vector of the saves
    saves: Vec<SaveGame>,
    /// A write-only file handle for the saves file
    file: File,
    /// Whether or not the save file is newly created
    is_new: bool,
}

impl SaveFile {
    fn new_from_disk(filepath: PathBuf) -> Result<Self, anyhow::Error> {
        // let existing_data: Vec<SaveGame> = {
        //     File::open(filepath).map(|f| f.read_to_string())
        // }
        let data: Vec<SaveGame> = {
            let mut buf = String::new();
            File::open(&filepath)?.read_to_string(&mut buf)?;
            serde_json::from_str(&buf)?
        };
        let file = File::create(filepath)?;
        Ok(Self {
            saves: data,
            file,
            is_new: false,
        })
    }

    fn new_and_new_file(filepath: PathBuf) -> Result<Self, anyhow::Error> {
        let file = File::create_new(filepath)?;
        Ok(Self {
            saves: Vec::new(),
            file,
            is_new: false,
        })
    }

    pub fn new(filepath: PathBuf) -> Result<Self, anyhow::Error> {
        if let Ok(v) = Self::new_and_new_file(filepath.clone()) {
            Ok(v)
        } else {
            Self::new_from_disk(filepath)
        }
    }

    pub fn write_to_disk(self) -> Result<(), serde_json::Error> {
        serde_json::to_writer_pretty(self.file, &self.saves)?;
        Ok(())
    }

    pub fn add_save(&mut self, save: SaveGame) {
        self.saves.push(save);
    }

    pub fn delete_save(&mut self, index: usize) -> bool {
        if self.saves.len() > index {
            self.saves.remove(index);
            true
        } else {
            false
        }
    }
}

#[derive(Serialize, Deserialize, Hash)]
pub struct SaveGame {
    living_cells: Vec<Vector2<i32>>,
    grid_size: OrderedFloat<f32>,
    pan_position: Vector2<OrderedFloat<f64>>,
    created: chrono::DateTime<chrono::Local>,
    name: String,
}

impl SaveGame {
    pub fn new(game_state: &GameState, name: String) -> Self {
        Self {
            living_cells: game_state.living_cells.iter().cloned().collect(),
            grid_size: OrderedFloat::from(game_state.grid_size),
            pan_position: Vector2::new(
                OrderedFloat::from(game_state.pan_position.x),
                OrderedFloat::from(game_state.pan_position.y),
            ),
            created: chrono::Local::now(),
            name,
        }
    }
}
