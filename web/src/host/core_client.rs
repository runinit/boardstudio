use crate::core_protocol::encode_core_request;
use boardstudio_core::model::{ArtifactReply, ArtifactRequest, CoreReply, CoreRequest};
use futures_channel::oneshot;
use js_sys::{Array, Object, Reflect, Uint8Array};
use serde_json::Value;
use std::{cell::RefCell, collections::BTreeMap, fmt, rc::Rc};
use wasm_bindgen::{JsCast, prelude::*};
use web_sys::{ErrorEvent, Event, MessageEvent, Worker, WorkerOptions, WorkerType};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HostError(pub String);

impl fmt::Display for HostError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for HostError {}

#[derive(Clone, Debug)]
pub struct ArchiveResult {
    pub metadata: String,
    pub buffers: Vec<Uint8Array>,
}

enum WorkerResult {
    Core(Box<CoreReply>),
    Artifact(Box<ArtifactReply>),
    Archive(ArchiveResult),
}

type PendingResult = Result<WorkerResult, HostError>;
struct Pending {
    executor_epoch: String,
    kind: String,
    sender: oneshot::Sender<PendingResult>,
}

#[derive(Default)]
struct State {
    ready: bool,
    closed: bool,
    ready_waiters: Vec<oneshot::Sender<Result<(), HostError>>>,
    pending: BTreeMap<String, Pending>,
}

pub struct CoreWorker {
    worker: Worker,
    state: Rc<RefCell<State>>,
    message: Closure<dyn FnMut(MessageEvent)>,
    error: Closure<dyn FnMut(Event)>,
    message_error: Closure<dyn FnMut(Event)>,
}

impl CoreWorker {
    pub fn new(url: &str) -> Result<Self, HostError> {
        let options = WorkerOptions::new();
        options.set_type(WorkerType::Module);
        let worker =
            Worker::new_with_options(url, &options).map_err(|error| HostError(js_error(error)))?;
        let state = Rc::new(RefCell::new(State::default()));

        let receiving = state.clone();
        let message = Closure::<dyn FnMut(MessageEvent)>::new(move |event: MessageEvent| {
            receive_message(&receiving, event.data());
        });
        let failed = state.clone();
        let error = Closure::<dyn FnMut(Event)>::new(move |event: Event| {
            let reason = event
                .dyn_ref::<ErrorEvent>()
                .map(|error| error.message())
                .filter(|message| !message.is_empty())
                .unwrap_or_else(|| "core worker failed".to_owned());
            fail_all(&failed, HostError(reason));
        });
        let failed = state.clone();
        let message_error = Closure::<dyn FnMut(Event)>::new(move |_| {
            fail_all(
                &failed,
                HostError("core worker message could not be decoded".into()),
            );
        });

        worker.set_onmessage(Some(message.as_ref().unchecked_ref()));
        worker.set_onerror(Some(error.as_ref().unchecked_ref()));
        worker.set_onmessageerror(Some(message_error.as_ref().unchecked_ref()));

        Ok(Self {
            worker,
            state,
            message,
            error,
            message_error,
        })
    }

    pub async fn ready(&self) -> Result<(), HostError> {
        let (sender, receiver) = oneshot::channel();
        {
            let mut state = self.state.borrow_mut();
            if state.ready {
                return Ok(());
            }
            if state.closed {
                return Err(HostError("core worker is closed".into()));
            }
            state.ready_waiters.push(sender);
        }
        receiver
            .await
            .map_err(|_| HostError("core worker readiness was abandoned".into()))?
    }

    pub async fn request(
        &self,
        request_id: &str,
        executor_epoch: &str,
        request: &CoreRequest,
    ) -> Result<CoreReply, HostError> {
        self.ready().await?;
        let frame = encode_core_request(request)
            .map_err(|error| HostError(format!("could not encode core request: {error}")))?;
        let receiver = self.dispatch(request_id, executor_epoch, "core", |object| {
            Reflect::set(object, &"frame".into(), &JsValue::from_str(&frame))
                .map(|_| ())
                .map_err(|error| HostError(js_error(error)))
        })?;
        let response = receiver
            .await
            .map_err(|_| HostError("core worker reply caller was abandoned".into()))??;
        match response {
            WorkerResult::Core(reply) => Ok(*reply),
            WorkerResult::Artifact(_) | WorkerResult::Archive(_) => {
                Err(HostError("worker returned archive for core request".into()))
            }
        }
    }

    pub async fn archive(
        &self,
        request_id: &str,
        executor_epoch: &str,
        metadata: &str,
        buffers: Vec<Uint8Array>,
    ) -> Result<ArchiveResult, HostError> {
        self.ready().await?;
        let indices: Vec<u32> = (0..buffers.len())
            .map(|index| {
                u32::try_from(index).map_err(|_| HostError("too many archive buffers".into()))
            })
            .collect::<Result<_, _>>()?;
        let transfer_buffers: Vec<JsValue> = buffers
            .iter()
            .enumerate()
            .map(|(index, bytes)| transfer_view(bytes, index))
            .collect::<Result<_, _>>()?;
        let receiver = self.dispatch(request_id, executor_epoch, "archive", |object| {
            Reflect::set(object, &"metadata".into(), &JsValue::from_str(metadata))
                .map_err(|error| HostError(js_error(error)))?;
            let index_array = Array::new();
            let transfer_array = Array::new();
            for (index, (number, buffer)) in indices.iter().zip(&transfer_buffers).enumerate() {
                index_array.push(&JsValue::from_f64(*number as f64));
                Reflect::set(object, &format!("buffer_{index}").into(), buffer)
                    .map_err(|error| HostError(js_error(error)))?;
                transfer_array.push(buffer);
            }
            Reflect::set(object, &"buffers".into(), &index_array)
                .map_err(|error| HostError(js_error(error)))?;
            Reflect::set(object, &"transfer".into(), &transfer_array)
                .map_err(|error| HostError(js_error(error)))?;
            Ok(())
        })?;
        let response = receiver
            .await
            .map_err(|_| HostError("archive worker reply caller was abandoned".into()))??;
        match response {
            WorkerResult::Archive(reply) => Ok(reply),
            WorkerResult::Core(_) | WorkerResult::Artifact(_) => Err(HostError(
                "worker returned core reply for archive request".into(),
            )),
        }
    }

    pub async fn artifact(
        &self,
        request_id: &str,
        executor_epoch: &str,
        request: &ArtifactRequest,
    ) -> Result<ArtifactReply, HostError> {
        self.ready().await?;
        let frame = serde_json::to_string(request)
            .map_err(|error| HostError(format!("could not encode artifact request: {error}")))?;
        let frame_id = serde_json::from_str::<Value>(&frame)
            .ok()
            .and_then(|value| value.get("id").and_then(Value::as_str).map(str::to_owned));
        if frame_id.as_deref() != Some(request_id) {
            return Err(HostError(
                "artifact request id does not match worker request id".into(),
            ));
        }
        let receiver = self.dispatch(request_id, executor_epoch, "artifact", |object| {
            Reflect::set(object, &"frame".into(), &JsValue::from_str(&frame))
                .map(|_| ())
                .map_err(|error| HostError(js_error(error)))
        })?;
        let response = receiver
            .await
            .map_err(|_| HostError("artifact worker reply caller was abandoned".into()))??;
        match response {
            WorkerResult::Artifact(reply) => Ok(*reply),
            WorkerResult::Core(_) | WorkerResult::Archive(_) => Err(HostError(
                "worker returned a non-artifact reply for artifact request".into(),
            )),
        }
    }

    pub fn close(&self) {
        self.worker.terminate();
        fail_all(&self.state, HostError("core worker closed".into()));
    }

    fn dispatch(
        &self,
        request_id: &str,
        executor_epoch: &str,
        kind: &str,
        fill: impl FnOnce(&Object) -> Result<(), HostError>,
    ) -> Result<oneshot::Receiver<PendingResult>, HostError> {
        if request_id.is_empty() {
            return Err(HostError("worker request id must not be empty".into()));
        }
        let (sender, receiver) = oneshot::channel();
        {
            let mut state = self.state.borrow_mut();
            if state.closed {
                return Err(HostError("core worker is closed".into()));
            }
            if state.pending.contains_key(request_id) {
                return Err(HostError("duplicate in-flight worker request id".into()));
            }
            state.pending.insert(
                request_id.to_owned(),
                Pending {
                    executor_epoch: executor_epoch.to_owned(),
                    kind: kind.to_owned(),
                    sender,
                },
            );
        }
        let object = Object::new();
        let result = (|| -> Result<(), HostError> {
            Reflect::set(&object, &"kind".into(), &JsValue::from_str(kind))
                .map_err(|error| HostError(js_error(error)))?;
            Reflect::set(
                &object,
                &"request_id".into(),
                &JsValue::from_str(request_id),
            )
            .map_err(|error| HostError(js_error(error)))?;
            Reflect::set(
                &object,
                &"executor_epoch".into(),
                &JsValue::from_str(executor_epoch),
            )
            .map_err(|error| HostError(js_error(error)))?;
            fill(&object)?;
            let transfer = Reflect::get(&object, &"transfer".into())
                .map_err(|error| HostError(js_error(error)))?;
            if transfer.is_undefined() {
                self.worker
                    .post_message(&object)
                    .map_err(|error| HostError(js_error(error)))
            } else {
                self.worker
                    .post_message_with_transfer(&object, &Array::from(&transfer))
                    .map_err(|error| HostError(js_error(error)))
            }
        })();
        if let Err(error) = result {
            if let Some(pending) = self.state.borrow_mut().pending.remove(request_id) {
                let _ = pending.sender.send(Err(error.clone()));
            }
            return Err(error);
        }
        Ok(receiver)
    }
}

impl Drop for CoreWorker {
    fn drop(&mut self) {
        self.close();
        self.worker.set_onmessage(None);
        self.worker.set_onerror(None);
        self.worker.set_onmessageerror(None);
        let _ = (&self.message, &self.error, &self.message_error);
    }
}

fn receive_message(state: &Rc<RefCell<State>>, data: JsValue) {
    if data.as_string().as_deref() == Some("boardstudio-core-ready") {
        let mut state = state.borrow_mut();
        if state.closed {
            return;
        }
        state.ready = true;
        for waiter in state.ready_waiters.drain(..) {
            let _ = waiter.send(Ok(()));
        }
        return;
    }
    let request_id = Reflect::get(&data, &"request_id".into())
        .ok()
        .and_then(|value| value.as_string());
    let Some(request_id) = request_id else {
        return;
    };
    let epoch = Reflect::get(&data, &"executor_epoch".into())
        .ok()
        .and_then(|value| value.as_string());
    let Some(epoch) = epoch else {
        return;
    };
    let pending = state.borrow_mut().pending.remove(&request_id);
    let Some(pending) = pending else {
        return;
    };
    let kind = Reflect::get(&data, &"kind".into())
        .ok()
        .and_then(|value| value.as_string());
    let outcome = if epoch != pending.executor_epoch {
        Err(HostError("worker reply has a stale executor epoch".into()))
    } else if kind.as_deref().is_some_and(|kind| kind != pending.kind) {
        Err(HostError("worker reply kind does not match request".into()))
    } else if let Ok(error) = Reflect::get(&data, &"error".into()) {
        if let Some(error) = error.as_string() {
            Err(HostError(error))
        } else {
            decode_reply(data, &request_id)
        }
    } else {
        decode_reply(data, &request_id)
    };
    let _ = pending.sender.send(outcome);
}

fn decode_reply(data: JsValue, expected_id: &str) -> PendingResult {
    let frame = Reflect::get(&data, &"frame".into())
        .map_err(|error| HostError(js_error(error)))?
        .as_string()
        .ok_or_else(|| HostError("worker reply is missing its text frame".into()))?;
    let kind = Reflect::get(&data, &"kind".into())
        .ok()
        .and_then(|value| value.as_string());
    if kind.as_deref() == Some("archive") {
        let buffers =
            Reflect::get(&data, &"buffers".into()).map_err(|error| HostError(js_error(error)))?;
        let buffers = Array::from(&buffers)
            .iter()
            .map(|buffer| Uint8Array::new(&buffer))
            .collect();
        return Ok(WorkerResult::Archive(ArchiveResult {
            metadata: frame,
            buffers,
        }));
    }
    if kind.as_deref() == Some("artifact") {
        let reply: ArtifactReply = serde_json::from_str(&frame)
            .map_err(|error| HostError(format!("invalid artifact reply: {error}")))?;
        let value: Value = serde_json::from_str(&frame)
            .map_err(|error| HostError(format!("invalid artifact reply identity: {error}")))?;
        if value.get("id").and_then(Value::as_str) != Some(expected_id) {
            return Err(HostError("artifact reply id does not match request".into()));
        }
        return Ok(WorkerResult::Artifact(Box::new(reply)));
    }
    let reply: CoreReply = serde_json::from_str(&frame)
        .map_err(|error| HostError(format!("invalid core reply: {error}")))?;
    let value: Value = serde_json::from_str(&frame)
        .map_err(|error| HostError(format!("invalid core reply identity: {error}")))?;
    if value.get("id").and_then(Value::as_str) != Some(expected_id) {
        return Err(HostError("core reply id does not match request".into()));
    }
    Ok(WorkerResult::Core(Box::new(reply)))
}

fn fail_all(state: &Rc<RefCell<State>>, error: HostError) {
    let mut state = state.borrow_mut();
    state.closed = true;
    for waiter in state.ready_waiters.drain(..) {
        let _ = waiter.send(Err(error.clone()));
    }
    for (_, pending) in std::mem::take(&mut state.pending) {
        let _ = pending.sender.send(Err(error.clone()));
    }
}

fn transfer_view(bytes: &Uint8Array, index: usize) -> Result<JsValue, HostError> {
    let buffer = bytes.buffer();
    let byte_length = bytes.byte_length();
    let offset = bytes.byte_offset();
    if offset == 0 && buffer.byte_length() == byte_length {
        Ok(buffer.into())
    } else {
        let copy = Uint8Array::new_with_length(byte_length);
        copy.set(bytes, 0);
        let owned = copy.buffer();
        if owned.byte_length() != byte_length {
            return Err(HostError(format!("could not copy archive buffer {index}")));
        }
        Ok(owned.into())
    }
}

fn js_error(value: JsValue) -> String {
    value
        .as_string()
        .or_else(|| {
            Reflect::get(&value, &"message".into())
                .ok()
                .and_then(|message| message.as_string())
        })
        .unwrap_or_else(|| format!("{value:?}"))
}
