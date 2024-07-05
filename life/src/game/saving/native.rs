use super::{DataStorage, SaveGame};
use std::{
    fs::File,
    io::{Read, Write},
    path::PathBuf,
};

pub struct NativeFs<T>
where
    T: serde::Serialize + for<'a> serde::Deserialize<'a> + Default,
{
    file: File,
    data: T,
}

impl<T> DataStorage for NativeFs<T>
where
    T: serde::Serialize + for<'a> serde::Deserialize<'a> + Default + Clone,
{
    type Data = T;
    type Error = anyhow::Error;
    fn new(identifier: &str) -> Result<(NativeFs<T>, T), anyhow::Error> {
        let filename = format!("{}.json", identifier);
        let existing_data: (T, String) = File::open(&filename)
            .ok()
            .and_then(|mut v| {
                let mut buf = String::new();
                v.read_to_string(&mut buf).ok()?;
                serde_json::from_str(&buf).ok().map(|v| (v, buf))
            })
            .unwrap_or((T::default(), serde_json::to_string_pretty(&T::default())?));

        let mut file = File::create(&filename)?;
        file.write_all(existing_data.1.as_bytes())?;

        Ok((
            NativeFs {
                file,
                data: existing_data.0.clone(),
            },
            existing_data.0,
        ))
    }
    fn get(&self) -> &T {
        &self.data
    }
    fn set(&mut self, data: T) {
        self.data = data;
    }
    fn finish(&mut self) -> Result<(), anyhow::Error> {
        self.file
            .write(serde_json::to_string_pretty(&self.data)?.as_bytes())?;
        Ok(())
    }
}
