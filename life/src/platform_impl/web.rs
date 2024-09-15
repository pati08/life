use std::marker::PhantomData;

use super::{DataPersistError, PlatformWorkerError};
use js_sys::Uint8Array;
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

use std::sync::mpsc;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{MessageEvent, Worker as WebWorker};

// Adjusted PlatformWorker struct without type generics
pub struct PlatformWorker<Args: Serialize, Res> {
    rx: mpsc::Receiver<Res>,
    worker: WebWorker,
    computing: bool,
    _phantom_data: PhantomData<Args>,
}

impl<
        Args: for<'de> Deserialize<'de> + 'static + Serialize,
        Res: for<'de> Deserialize<'de> + 'static,
    > PlatformWorker<Args, Res>
{
    // Accepts a JavaScript worker file path and a closure that will be used inside the worker
    pub fn new() -> Self {
        let worker =
            WebWorker::new(worker_url).expect("Failed to create web worker");

        let (res_tx, res_rx) = mpsc::sync_channel(1);

        // Worker message handler to receive results from the worker
        let closure = Closure::wrap(Box::new(move |event: MessageEvent| {
            let result = event.data();
            let data =
                bincode::deserialize(&Uint8Array::from(result).to_vec()[..])
                    .unwrap();
            res_tx.send(data).expect("Failed to send result");
        }) as Box<dyn FnMut(_)>);

        worker.set_onmessage(Some(closure.as_ref().unchecked_ref()));
        closure.forget(); // Prevent closure from being dropped

        Self {
            rx: res_rx,
            worker,
            computing: false,
            _phantom_data: PhantomData,
        }
    }

    /// Send data to be processed by the worker
    pub fn send(&mut self, data: Args) -> Result<bool, PlatformWorkerError> {
        if self.computing {
            return Ok(false); // Already computing
        }

        let Ok(data) = bincode::serialize(&Message::Process(data)) else {
            return Err(PlatformWorkerError::SerFailed);
        };

        if self.worker.post_message(&JsValue::from(data)).is_err() {
            Err(PlatformWorkerError::MessagePostFailed)
        } else {
            self.computing = true;
            Ok(true)
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

// Simple message enum to manage worker messages
#[derive(Deserialize, Serialize)]
enum Message<Args, Res> {
    Stop,
    Process(Args),
    Load(Closure<dyn Fn(Args) -> Res>),
}

// Automatically stop the worker when the struct is dropped
impl<Args: Serialize, Res> Drop for PlatformWorker<Args, Res> {
    fn drop(&mut self) {
        let js_val =
            JsValue::from(bincode::serialize(&Message::<Args>::Stop).unwrap());
        let _ = self.worker.post_message(&js_val);
        self.worker.terminate();
    }
}
