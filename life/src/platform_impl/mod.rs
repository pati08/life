#[cfg(not(target_arch = "wasm32"))]
mod native;
#[cfg(not(target_arch = "wasm32"))]
pub use native::*;

#[cfg(target_arch = "wasm32")]
mod web;
use serde::{Deserialize, Serialize};
#[cfg(target_arch = "wasm32")]
pub use web::*;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum DataPersistError {
    #[error("Native filesystem or io error: {0:?}")]
    DataNative(#[from] std::io::Error),
    #[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
    #[error("Web data persistence error.")]
    DataWeb,
    #[error("JSON/Serde error")]
    Json(#[from] serde_json::Error),
}

impl<T> DataHandle<T>
where
    T: Serialize + for<'de> Deserialize<'de>,
{
    pub fn update<F: FnOnce(&mut Option<T>)>(
        &mut self,
        with: F,
    ) -> Result<(), DataPersistError> {
        let mut data = self.get()?;
        with(&mut data);
        self.maybe_set(&data)?;
        Ok(())
    }
    fn maybe_set(&mut self, v: &Option<T>) -> Result<bool, DataPersistError> {
        if let Some(ref v) = v {
            self.set(v)?;
            Ok(true)
        } else {
            Ok(false)
        }
    }
}

#[derive(Error, Debug)]
pub enum PlatformWorkerError {
    #[error("Disconnected")]
    Disconnected,
    #[error("Failed to post message to web worker")]
    MessagePostFailed,
    #[error("Failed spawning worker or thread")]
    SpawnFailed,
}
