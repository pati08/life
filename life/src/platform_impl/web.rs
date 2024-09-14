use std::marker::PhantomData;

use super::DataPersistError;
use serde::{Deserialize, Serialize};
use web_sys::Storage;

type DPResult<T> = Result<T, DataPersistError>;

pub struct DataHandle<T>
where
    T: Serialize + for<'de> Deserialize<'de>,
{
    storage: Storage,
    id: String,
    _phantom_data: PhantomData<T>,
}

impl<T> DataHandle<T>
where
    T: Serialize + for<'de> Deserialize<'de>,
{
    pub fn new(id: &str) -> DPResult<Self> {
        let storage = web_sys::window()
            .ok_or(DataPersistError::DataWeb)?
            .local_storage()
            .map_err(|_| DataPersistError::DataWeb)?
            .ok_or(DataPersistError::DataWeb)?;
        let id = id.to_owned();
        Ok(Self {
            storage,
            id,
            _phantom_data: PhantomData,
        })
    }
    pub fn set(&mut self, to: &T) -> DPResult<()> {
        let serialized = serde_json::to_string_pretty(to)?;
        self.storage
            .set_item(&self.id, &serialized)
            .map_err(|_| DataPersistError::DataWeb)?;
        Ok(())
    }
    pub fn get(&self) -> DPResult<Option<T>> {
        let Some(data) = self
            .storage
            .get_item(&self.id)
            .map_err(|_| DataPersistError::DataWeb)?
        else {
            return Ok(None);
        };
        if data.is_empty() {
            Ok(None)
        } else {
            let val = serde_json::from_str(&data)?;
            Ok(Some(val))
        }
    }
}
