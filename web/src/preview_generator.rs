//! Runtime client for the private, packaged Ergogen preview module worker.

use futures_channel::oneshot;
use js_sys::Reflect;
use serde_json::Value;
use std::{cell::RefCell, collections::BTreeMap, rc::Rc};
use wasm_bindgen::{JsCast, closure::Closure};
use web_sys::{ErrorEvent, Event, MessageEvent, Worker, WorkerOptions, WorkerType};

type Reply = Result<Value, String>;
type Pending = BTreeMap<u64, oneshot::Sender<Reply>>;

pub(crate) struct PreviewGeneratorClient {
    worker: Worker,
    pending: Rc<RefCell<Pending>>,
    message: Closure<dyn FnMut(MessageEvent)>,
    error: Closure<dyn FnMut(Event)>,
    message_error: Closure<dyn FnMut(Event)>,
}

impl PreviewGeneratorClient {
    pub(crate) fn new(url: &str) -> Result<Self, String> {
        let options = WorkerOptions::new();
        options.set_type(WorkerType::Module);
        let worker = Worker::new_with_options(url, &options).map_err(js_error)?;
        let pending: Rc<RefCell<Pending>> = Rc::new(RefCell::new(BTreeMap::new()));

        let receiving = pending.clone();
        let message = Closure::<dyn FnMut(MessageEvent)>::new(move |event: MessageEvent| {
            let data = event.data();
            let Ok(id) = Reflect::get(&data, &"request_id".into()) else {
                return;
            };
            let Some(id) = id.as_f64().filter(|id| id.is_finite() && *id >= 1.0) else {
                return;
            };
            let id = id as u64;
            if let Some(sender) = receiving.borrow_mut().remove(&id) {
                let result = serde_wasm_bindgen::from_value::<Value>(data)
                    .map_err(|error| format!("Preview worker reply could not be decoded: {error}"));
                let _ = sender.send(result);
            }
        });
        let failed = pending.clone();
        let error = Closure::<dyn FnMut(Event)>::new(move |event: Event| {
            let reason = event
                .dyn_ref::<ErrorEvent>()
                .map(|error| error.message())
                .filter(|message| !message.is_empty())
                .unwrap_or_else(|| "preview generator worker failed".to_owned());
            fail_all(&failed, reason);
        });
        let failed = pending.clone();
        let message_error = Closure::<dyn FnMut(Event)>::new(move |_| {
            fail_all(
                &failed,
                "preview generator worker reply could not be decoded".into(),
            );
        });

        worker.set_onmessage(Some(message.as_ref().unchecked_ref()));
        worker.set_onerror(Some(error.as_ref().unchecked_ref()));
        worker.set_onmessageerror(Some(message_error.as_ref().unchecked_ref()));
        Ok(Self {
            worker,
            pending,
            message,
            error,
            message_error,
        })
    }

    pub(crate) async fn generate(&self, request_id: u64, request: &Value) -> Result<Value, String> {
        if request_id == 0 || request_id > 9_007_199_254_740_991 {
            return Err("Preview worker request identity is outside the safe integer range".into());
        }
        let (sender, receiver) = oneshot::channel();
        {
            let mut pending = self.pending.borrow_mut();
            if pending.contains_key(&request_id) {
                return Err("Preview worker request identity is already pending".into());
            }
            pending.insert(request_id, sender);
        }
        let message = match serde_wasm_bindgen::to_value(request) {
            Ok(message) => message,
            Err(error) => {
                self.pending.borrow_mut().remove(&request_id);
                return Err(format!(
                    "Preview worker request could not be encoded: {error}"
                ));
            }
        };
        if let Err(error) = self.worker.post_message(&message) {
            self.pending.borrow_mut().remove(&request_id);
            return Err(js_error(error));
        }
        receiver
            .await
            .map_err(|_| "Preview worker reply caller was abandoned".to_owned())?
    }
}

impl Drop for PreviewGeneratorClient {
    fn drop(&mut self) {
        self.worker.set_onmessage(None);
        self.worker.set_onerror(None);
        self.worker.set_onmessageerror(None);
        self.worker.terminate();
        fail_all(
            &self.pending,
            "preview generator worker was closed".to_owned(),
        );
        let _ = &self.message;
        let _ = &self.error;
        let _ = &self.message_error;
    }
}

fn fail_all(pending: &Rc<RefCell<Pending>>, reason: String) {
    for (_, sender) in std::mem::take(&mut *pending.borrow_mut()) {
        let _ = sender.send(Err(reason.clone()));
    }
}

fn js_error(value: wasm_bindgen::JsValue) -> String {
    value
        .as_string()
        .or_else(|| {
            Reflect::get(&value, &"message".into())
                .ok()
                .and_then(|message| message.as_string())
        })
        .unwrap_or_else(|| "preview generator browser operation failed".to_owned())
}
