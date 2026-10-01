use boardstudio_p1_cad::{Action, Identity, Lifecycle, Payload, Reply, Request};
use js_sys::{Array, Object, Reflect, Uint8Array};
use std::{cell::RefCell, collections::BTreeMap, rc::Rc};
use wasm_bindgen::{JsCast, prelude::*};
use web_sys::{Event, MessageEvent, Worker, WorkerOptions, WorkerType};

type MeshBuffers = (Rc<Vec<u8>>, Rc<Vec<u8>>);
pub type Outcome = Result<(Reply, Vec<Rc<Vec<u8>>>, bool), String>;
struct State {
    callers: Lifecycle,
    ready: bool,
    rejected: usize,
    results: BTreeMap<Identity, Outcome>,
    meshes: BTreeMap<String, MeshBuffers>,
}
impl State {
    fn fail(&mut self, cause: &str) {
        for id in self.callers.fail() {
            self.results.insert(id, Err(cause.into()));
        }
    }
}
pub struct Client {
    worker: Worker,
    state: Rc<RefCell<State>>,
    message: Closure<dyn FnMut(MessageEvent)>,
    error: Closure<dyn FnMut(Event)>,
}
impl Client {
    pub fn new(url: &str, epoch: u32) -> Result<Self, JsValue> {
        let options = WorkerOptions::new();
        options.set_type(WorkerType::Module);
        let worker = Worker::new_with_options(url, &options)?;
        let state = Rc::new(RefCell::new(State {
            callers: Lifecycle::new(epoch),
            ready: false,
            rejected: 0,
            results: BTreeMap::new(),
            meshes: BTreeMap::new(),
        }));
        let receive = state.clone();
        let message = Closure::<dyn FnMut(MessageEvent)>::new(move |event: MessageEvent| {
            let data = event.data();
            let mut state = receive.borrow_mut();
            if data.as_string().as_deref() == Some("ready") {
                state.ready = true;
                return;
            }
            let decoded = Reflect::get(&data, &"frame".into())
                .ok()
                .and_then(|frame| frame.as_string())
                .and_then(|frame| serde_json::from_str::<Reply>(&frame).ok());
            if let Some(reply) = decoded {
                let buffers: Vec<Rc<Vec<u8>>> = Reflect::get(&data, &"buffers".into())
                    .ok()
                    .map(|value| {
                        Array::from(&value)
                            .iter()
                            .map(|buffer| Rc::new(Uint8Array::new(&buffer).to_vec()))
                            .collect()
                    })
                    .unwrap_or_default();
                if let Payload::Preview { delta, .. } = &reply.payload
                    && buffers.len() != delta.changed.len() * 2
                {
                    state.fail("invalid preview buffers");
                    return;
                }
                let complete = match &reply.payload {
                    Payload::Preview { delta, .. } => state.callers.finish_preview(reply.id, delta),
                    _ => state.callers.finish(reply.id),
                };
                if let Ok(current) = complete {
                    if let Payload::Preview { delta, .. } = &reply.payload {
                        state.meshes.retain(|id, _| delta.ids.contains(id));
                        for (body, pair) in
                            delta.changed.iter().zip(buffers.as_chunks::<2>().0.iter())
                        {
                            state
                                .meshes
                                .insert(body.id.clone(), (pair[0].clone(), pair[1].clone()));
                        }
                    }
                    state
                        .results
                        .insert(reply.id, Ok((reply, buffers, current)));
                    return;
                }
            }
            state.rejected += 1;
        });
        let failed = state.clone();
        let error = Closure::<dyn FnMut(Event)>::new(move |_: Event| {
            failed
                .borrow_mut()
                .fail("worker error or message decoding failure")
        });
        worker.set_onmessage(Some(message.as_ref().unchecked_ref()));
        worker.set_onerror(Some(error.as_ref().unchecked_ref()));
        worker.set_onmessageerror(Some(error.as_ref().unchecked_ref()));
        Ok(Self {
            worker,
            state,
            message,
            error,
        })
    }
    pub fn send(
        &self,
        id: Identity,
        action: Action,
        bytes: Option<&Uint8Array>,
    ) -> Result<(), String> {
        self.state.borrow_mut().callers.begin(id)?;
        let result = (|| -> Result<(), JsValue> {
            let object = Object::new();
            Reflect::set(
                &object,
                &"frame".into(),
                &JsValue::from_str(
                    &serde_json::to_string(&Request { id, action })
                        .map_err(|error| JsValue::from_str(&error.to_string()))?,
                ),
            )?;
            if let Some(bytes) = bytes {
                let buffer = bytes.buffer();
                Reflect::set(&object, &"buffer".into(), &buffer)?;
                let transfer = Array::new();
                transfer.push(&buffer);
                self.worker.post_message_with_transfer(&object, &transfer)
            } else {
                self.worker.post_message(&object)
            }
        })();
        if let Err(error) = result {
            self.close();
            return Err(format!("post failure: {error:?}"));
        }
        Ok(())
    }
    pub fn cancel(&self, target: Identity) -> Result<(), String> {
        let object = Object::new();
        let frame = serde_json::to_string(&Request {
            id: target,
            action: Action::Cancel { target },
        })
        .map_err(|e| e.to_string())?;
        Reflect::set(&object, &"frame".into(), &frame.into()).map_err(|e| format!("{e:?}"))?;
        self.worker
            .post_message(&object)
            .map_err(|e| format!("{e:?}"))
    }
    pub async fn wait(&self, id: Identity) -> Outcome {
        for _ in 0..3000 {
            if let Some(result) = self.state.borrow_mut().results.remove(&id) {
                return result;
            }
            gloo_timers::future::TimeoutFuture::new(5).await;
        }
        self.state
            .borrow_mut()
            .fail("probe deadline exceeded; no replay");
        self.state
            .borrow_mut()
            .results
            .remove(&id)
            .unwrap_or_else(|| Err("missing caller".into()))
    }
    pub fn close(&self) {
        self.worker.terminate();
        self.state.borrow_mut().fail("worker closed");
    }
    pub fn mesh(&self, id: &str) -> Option<MeshBuffers> {
        self.state.borrow().meshes.get(id).cloned()
    }
    pub fn body_key(&self, id: &str) -> Option<String> {
        self.state.borrow().callers.body_key(id).map(str::to_owned)
    }
    pub fn cache_base(&self) -> u32 {
        self.state.borrow().callers.cache_base()
    }
    pub fn rejected(&self) -> usize {
        self.state.borrow().rejected
    }
    pub fn pending(&self) -> usize {
        self.state.borrow().callers.pending()
    }
    pub fn ready(&self) -> bool {
        self.state.borrow().ready
    }
}
impl Drop for Client {
    fn drop(&mut self) {
        self.close();
        self.worker.set_onmessage(None);
        self.worker.set_onerror(None);
        self.worker.set_onmessageerror(None);
        // Retain callback owners until after their browser registrations are cleared.
        let _ = (&self.message, &self.error);
    }
}
