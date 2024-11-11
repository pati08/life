use std::marker::PhantomData;

use super::{DataPersistError, PlatformWorkerError};
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsValue;
use web_sys::DedicatedWorkerGlobalScope;
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
use web_sys::Worker;

use super::{Message, PlatformWorker};

pub fn new_plat_worker<
    Args: Send + 'static,
    Res: Send + 'static,
    F: Fn(Args) -> Res + Send + 'static,
>(
    fun: F,
) -> Result<PlatformWorker<Args, Res>, PlatformWorkerError> {
    let Ok(worker) = Worker::new("/worker.js") else {
        return Err(PlatformWorkerError::SpawnFailed);
    };
    let (proc_tx, proc_rx) = mpsc::sync_channel::<Message<Args>>(0);
    let (res_tx, res_rx) = mpsc::sync_channel(1);

    let array = js_sys::Array::new();
    array.push(&wasm_bindgen::module());
    array.push(&wasm_bindgen::memory());
    worker
        .post_message(&array)
        .map_err(|_e| PlatformWorkerError::MessagePostFailed)?;
    let work_func = move || {
        while let Ok(Message::Process(data)) = proc_rx.recv() {
            if res_tx.send(fun(data)).is_err() {
                break;
            }
        }
    };
    let work = Box::new(Work {
        func: Box::new(work_func),
    });
    let ptr = Box::into_raw(work);
    if worker.post_message(&JsValue::from(ptr as u32)).is_err() {
        unsafe {
            drop(Box::from_raw(ptr));
        }
        panic!("Failed to post message to web worker");
    }

    Ok(PlatformWorker {
        tx: proc_tx,
        rx: res_rx,
        computing: false,
    })
}

struct Work {
    func: Box<dyn FnOnce() + Send>,
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
pub fn child_entry_point(ptr: u32) -> Result<(), JsValue> {
    let ptr = unsafe { Box::from_raw(ptr as *mut Work) };
    let global =
        js_sys::global().unchecked_into::<DedicatedWorkerGlobalScope>();
    (ptr.func)();
    global.post_message(&JsValue::undefined())?;
    Ok(())
}
