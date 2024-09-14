use crate::platform_impl::DataHandle;
use rustc_hash::FxHashSet;
use serde::{Deserialize, Serialize};
use vec2::Vector2;

/// A representation of a game save file. The saves are stored in memory unless
/// written to disk via `SaveFile::write_to_disk`.
/// TODO: Remove the unwrap and make these return results
pub struct SaveData {
    inner: DataHandle<Vec<SaveGame>>,
}

impl SaveData {
    pub fn new() -> Result<Self, anyhow::Error> {
        let inner = DataHandle::new("saves")?;
        Ok(Self { inner })
    }

    /// Add a game save to the file.
    pub fn add_save(&mut self, save: SaveGame) {
        self.inner
            .update(move |saves| {
                if let Some(saves) = saves {
                    saves.push(save);
                } else {
                    saves.replace(vec![save]);
                }
            })
            .unwrap();
    }

    /// Delete a save from the file at a given index. This is safe to perform on
    /// an index that is out of bounds. The function returns whether or not it
    /// removed a save.
    pub fn delete_save(&mut self, index: usize) -> bool {
        let saves = self.inner.get().unwrap();
        if let Some(mut saves) = saves
            && saves.len() > index
        {
            saves.remove(index);
            self.inner.set(&saves).unwrap();
            true
        } else {
            false
        }
    }

    /// Get an iterator over the game saves the file contains
    pub fn saves_iter(&self) -> impl Iterator<Item = SaveGame> {
        self.inner.get().unwrap().unwrap_or_default().into_iter()
    }
    /// Get the number of stored saves
    pub fn save_count(&self) -> usize {
        self.inner.get().unwrap().unwrap_or_default().len()
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
/// A record of a game that can be restored.
pub struct SaveGame {
    living_cells: Vec<Vector2<i32>>,
    grid_size: f32,
    pan_position: Vector2<f64>,
    pub created: chrono::DateTime<chrono::Local>,
    pub name: String,
}

impl SaveGame {
    pub fn new(game_state: &super::State, name: String) -> Self {
        Self {
            living_cells: game_state.living_cells.iter().copied().collect(),
            grid_size: game_state.grid_size,
            pan_position: game_state.pan_position,
            created: chrono::Local::now(),
            name,
        }
    }
    pub fn living_cells(&self) -> FxHashSet<Vector2<i32>> {
        self.living_cells.iter().copied().collect()
    }
    pub fn pan_position(&self) -> Vector2<f64> {
        self.pan_position
    }
    pub fn grid_size(&self) -> f32 {
        self.grid_size
    }
}
