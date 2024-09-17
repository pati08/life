use super::{DataPersistError, PlatformWorkerError};
use serde::{Deserialize, Serialize};
use std::{
    fs::File,
    io::{Read, Seek, Write},
    marker::PhantomData,
    sync::{
        mpsc::{self, Receiver, SyncSender},
        RwLock,
    },
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

pub struct PlatformWorker<Args: Send, Res: Send> {
    tx: SyncSender<Message<Args>>,
    rx: Receiver<Res>,
    computing: bool,
}

impl<Args: Send + 'static, Res: Send + 'static> PlatformWorker<Args, Res> {
    pub fn new<F: Fn(Args) -> Res + Send + 'static>(fun: F) -> Self {
        let (proc_tx, proc_rx) = mpsc::sync_channel(0);
        let (res_tx, res_rx) = mpsc::sync_channel(1);
        let _handle = std::thread::spawn(move || {
            while let Ok(Message::Process(data)) = proc_rx.recv() {
                if res_tx.send(fun(data)).is_err() {
                    break;
                };
            }
        });
        Self {
            tx: proc_tx,
            rx: res_rx,
            computing: false,
        }
    }
    /// Send some data over to be processed
    pub fn send(&mut self, data: Args) -> Result<bool, PlatformWorkerError> {
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
    pub fn results(&mut self) -> Result<Option<Res>, PlatformWorkerError> {
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
    // This function is not currently used but may become useful in the future.
    /// Blocks until results are available.
    pub fn wait_results(&mut self) -> Result<Res, PlatformWorkerError> {
        let res = self
            .rx
            .recv()
            .map_err(|_| PlatformWorkerError::Disconnected);
        if res.is_ok() {
            self.computing = false;
        }
        res
    }
    pub fn computing(&self) -> bool {
        self.computing
    }
}

enum Message<Args> {
    Stop,
    Process(Args),
}

// Tell the other thread to stop when this is dropped.
impl<Args: Send, Res: Send> Drop for PlatformWorker<Args, Res> {
    fn drop(&mut self) {
        let _ = self.tx.send(Message::Stop);
    }
}
