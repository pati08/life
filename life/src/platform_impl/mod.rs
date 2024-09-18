use std::sync::mpsc::{self, Receiver, SyncSender};

#[cfg(not(target_arch = "wasm32"))]
mod native;
#[cfg(not(target_arch = "wasm32"))]
pub use native::{new_plat_worker, DataHandle};

#[cfg(target_arch = "wasm32")]
mod web;
use serde::{Deserialize, Serialize};
#[cfg(target_arch = "wasm32")]
pub use web::{new_plat_worker, DataHandle};

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

#[allow(dead_code)] // Some are platform-specific
#[derive(Error, Debug)]
pub enum PlatformWorkerError {
    #[error("Disconnected")]
    Disconnected,
    #[error("Failed to post message to web worker")]
    MessagePostFailed,
    #[error("Failed spawning worker or thread")]
    SpawnFailed,
}

enum Message<Args> {
    Stop,
    Process(Args),
}

pub trait ComputeWorker<Args: Send, Res: Send> {
    fn send(&mut self, data: Args) -> Result<bool, PlatformWorkerError>;
    fn results(&mut self) -> Result<Option<Res>, PlatformWorkerError>;
    fn computing(&self) -> bool;
}

pub struct PlatformWorker<Args: Send, Res: Send> {
    tx: SyncSender<Message<Args>>,
    rx: Receiver<Res>,
    computing: bool,
}

impl<Args: Send + 'static, Res: Send + 'static> PlatformWorker<Args, Res> {
    pub fn new<F: Fn(Args) -> Res + Send + 'static>(
        fun: F,
    ) -> Result<Self, PlatformWorkerError> {
        new_plat_worker(fun)
    }
}

impl<Args: Send + 'static, Res: Send + 'static> ComputeWorker<Args, Res>
    for PlatformWorker<Args, Res>
{
    /// Send some data over to be processed
    fn send(&mut self, data: Args) -> Result<bool, PlatformWorkerError> {
        match self.tx.try_send(Message::Process(data)) {
            Ok(()) => {
                self.computing = true;
                Ok(true)
            }
            Err(mpsc::TrySendError::Full(_data)) => Ok(false),
            Err(mpsc::TrySendError::Disconnected(_data)) => {
                Err(PlatformWorkerError::Disconnected)
            }
        }
    }
    /// Get results if they are available, but return immediately if not.
    fn results(&mut self) -> Result<Option<Res>, PlatformWorkerError> {
        match self.rx.try_recv() {
            Ok(res) => {
                self.computing = false;
                Ok(Some(res))
            }
            Err(mpsc::TryRecvError::Disconnected) => {
                Err(PlatformWorkerError::Disconnected)
            }
            Err(mpsc::TryRecvError::Empty) => Ok(None),
        }
    }
    fn computing(&self) -> bool {
        self.computing
    }
}

// Tell the other thread to stop when this is dropped.
impl<Args: Send, Res: Send> Drop for PlatformWorker<Args, Res> {
    fn drop(&mut self) {
        let _ = self.tx.send(Message::Stop);
    }
}
