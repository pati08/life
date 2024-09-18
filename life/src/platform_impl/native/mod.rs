use super::{DataPersistError, PlatformWorkerError};
use serde::{Deserialize, Serialize};
use std::{
    fs::File,
    io::{Read, Seek, Write},
    marker::PhantomData,
    sync::{mpsc, RwLock},
};

type DPResult<T> = Result<T, DataPersistError>;

pub struct DataHandle<T>
where
    T: Serialize + for<'de> Deserialize<'de>,
{
    file: RwLock<File>,
    _phantom_data: PhantomData<T>,
}

impl<T> DataHandle<T>
where
    T: Serialize + for<'de> Deserialize<'de>,
{
    pub fn new(id: &str) -> DPResult<Self> {
        let file = std::fs::OpenOptions::new()
            .write(true)
            .read(true)
            .create(true)
            .open(format!("{id}.json"))?
            .into();
        Ok(Self {
            file,
            _phantom_data: PhantomData,
        })
    }
    pub fn set(&mut self, to: &T) -> DPResult<()> {
        let serialized = serde_json::to_string_pretty(to)?;
        let mut file = self.file.write().unwrap();
        file.set_len(0)?;
        file.seek(std::io::SeekFrom::Start(0))?;
        file.write_all(serialized.as_bytes())?;
        Ok(())
    }
    // Because of the kind of data that will be stored, I decided not to
    // cache the current data in the struct. That's because it will be saves,
    // which are large and will be read (and updated) only occasionally.
    pub fn get(&self) -> DPResult<Option<T>> {
        let mut buf = String::new();
        let mut file = self.file.write().unwrap();
        file.seek(std::io::SeekFrom::Start(0))?;
        file.read_to_string(&mut buf)?;
        if buf.is_empty() {
            Ok(None)
        } else {
            let val = serde_json::from_str(&buf)?;
            Ok(Some(val))
        }
    }
}

use super::{Message, PlatformWorker};

#[allow(clippy::unnecessary_wraps)]
pub fn new_plat_worker<
    Args: Send + 'static,
    Res: Send + 'static,
    F: Fn(Args) -> Res + Send + 'static,
>(
    fun: F,
) -> Result<PlatformWorker<Args, Res>, PlatformWorkerError> {
    let (proc_tx, proc_rx) = mpsc::sync_channel(0);
    let (res_tx, res_rx) = mpsc::sync_channel(1);
    let _handle = std::thread::spawn(move || {
        while let Ok(Message::Process(data)) = proc_rx.recv() {
            if res_tx.send(fun(data)).is_err() {
                break;
            };
        }
    });
    Ok(PlatformWorker {
        tx: proc_tx,
        rx: res_rx,
        computing: false,
    })
}
