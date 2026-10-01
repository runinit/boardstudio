use crate::{Action, BodyStamp, Delta, Identity, Payload, Reply, Request};
use js_sys::{Array, ArrayBuffer, Float32Array, Function, Object, Reflect, Uint8Array};
use std::{
    cell::{Cell, RefCell},
    collections::{BTreeMap, BTreeSet, VecDeque},
    rc::Rc,
};
use wasm_bindgen::{JsCast, prelude::*};
use wasm_bindgen_futures::{JsFuture, spawn_local};
use web_sys::{DedicatedWorkerGlobalScope, MessageEvent};

#[wasm_bindgen(raw_module = "./cad/boardstudio_cadrum_wasm.js")]
extern "C" {
    #[wasm_bindgen(catch, js_name = "default")]
    fn initialize_cad() -> Result<js_sys::Promise, JsValue>;
    #[wasm_bindgen(catch)]
    fn preview_body(input: JsValue, key: &str, progress: &Function) -> Result<JsValue, JsValue>;
    #[wasm_bindgen(catch)]
    fn export_cached_assembly(input: JsValue, keys: JsValue) -> Result<JsValue, JsValue>;
    #[wasm_bindgen(catch)]
    fn read_step_model(bytes: Uint8Array) -> Result<JsValue, JsValue>;
}

#[derive(Default)]
struct State {
    queue: VecDeque<(Request, Option<JsValue>)>,
    busy: bool,
    active: Option<Identity>,
    cancelled: BTreeSet<Identity>,
    base: u32,
    index: BTreeMap<String, BodyStamp>,
    transferred_bytes: u64,
    detached: bool,
    progress: Rc<Cell<u32>>,
}

#[wasm_bindgen]
pub async fn start_worker() -> Result<(), JsValue> {
    JsFuture::from(initialize_cad()?).await?;
    let scope: DedicatedWorkerGlobalScope = js_sys::global().dyn_into()?;
    let state = Rc::new(RefCell::new(State {
        detached: true,
        ..State::default()
    }));
    let target = scope.clone();
    let callback = Closure::<dyn FnMut(MessageEvent)>::new(move |event: MessageEvent| {
        let result = (|| -> Result<(), JsValue> {
            let data = event.data();
            let frame = Reflect::get(&data, &"frame".into())?
                .as_string()
                .ok_or_else(|| error("missing JSON text frame"))?;
            let request: Request =
                serde_json::from_str(&frame).map_err(|e| error(&e.to_string()))?;
            if matches!(request.action, Action::Crash) {
                wasm_bindgen::throw_str("intentional P1 CAD worker crash");
            }
            if let Action::Cancel { target: cancelled } = request.action {
                let mut s = state.borrow_mut();
                if s.active == Some(cancelled) || s.queue.iter().any(|(r, _)| r.id == cancelled) {
                    s.cancelled.insert(cancelled);
                }
                return Ok(());
            }
            let buffer = Reflect::get(&data, &"buffer".into())
                .ok()
                .filter(|b| !b.is_undefined());
            let start = {
                let mut s = state.borrow_mut();
                s.queue.push_back((request, buffer));
                let start = !s.busy;
                s.busy = true;
                start
            };
            if start {
                let state = state.clone();
                let scope = target.clone();
                spawn_local(async move {
                    loop {
                        let job = state.borrow_mut().queue.pop_front();
                        let Some((request, buffer)) = job else {
                            state.borrow_mut().busy = false;
                            break;
                        };
                        let id = request.id;
                        state.borrow_mut().active = Some(id);
                        if let Err(cause) = execute(&scope, &state, request, buffer).await
                            && let Err(post_error) = post(
                                &scope,
                                &state,
                                Reply {
                                    id,
                                    payload: Payload::Error(format!("{cause:?}")),
                                },
                                Vec::new(),
                            )
                        {
                            wasm_bindgen::throw_val(post_error);
                        }
                        let mut s = state.borrow_mut();
                        s.active = None;
                        s.cancelled.remove(&id);
                    }
                });
            }
            Ok(())
        })();
        if let Err(cause) = result {
            wasm_bindgen::throw_val(cause);
        }
    });
    scope.set_onmessage(Some(callback.as_ref().unchecked_ref()));
    callback.forget(); // Dedicated-worker registration ends when this owned worker terminates.
    scope.post_message(&"ready".into())
}

async fn execute(
    scope: &DedicatedWorkerGlobalScope,
    state: &Rc<RefCell<State>>,
    request: Request,
    buffer: Option<JsValue>,
) -> Result<(), JsValue> {
    if state.borrow().cancelled.contains(&request.id) {
        return post(
            scope,
            state,
            Reply {
                id: request.id,
                payload: Payload::Cancelled,
            },
            Vec::new(),
        );
    }
    let mut buffers = Vec::new();
    let payload = match request.action {
        Action::Preview { prepared } => {
            let input = js_sys::JSON::parse(&prepared)?;
            let revision = revision(&prepared)?;
            let bodies = Array::from(&Reflect::get(&input, &"bodies".into())?);
            if bodies.length() == 0 {
                return Err(error("Case assembly requires at least one body"));
            }
            let mut index = BTreeMap::new();
            let mut changed = Vec::new();
            let mut ids = Vec::new();
            for body in bodies.iter() {
                let stamp = stamp(&body)?;
                ids.push(stamp.id.clone());
                if state.borrow().index.get(&stamp.id) != Some(&stamp) {
                    let progress = state.borrow().progress.clone();
                    let callback = Closure::<dyn FnMut(JsValue, JsValue)>::new(move |_, _| {
                        progress.set(progress.get() + 1)
                    });
                    let mesh = preview_body(body, &stamp.key, callback.as_ref().unchecked_ref())?;
                    buffers.push(
                        Reflect::get(&mesh, &"positions".into())?
                            .dyn_into::<Float32Array>()?
                            .buffer()
                            .into(),
                    );
                    buffers.push(
                        Reflect::get(&mesh, &"normals".into())?
                            .dyn_into::<Float32Array>()?
                            .buffer()
                            .into(),
                    );
                    changed.push(stamp.clone());
                    // An explicit cooperative boundary; synchronous OCCT cannot be interrupted.
                    gloo_timers::future::TimeoutFuture::new(0).await;
                    if state.borrow().cancelled.contains(&request.id) {
                        return post(
                            scope,
                            state,
                            Reply {
                                id: request.id,
                                payload: Payload::Cancelled,
                            },
                            Vec::new(),
                        );
                    }
                }
                index.insert(stamp.id.clone(), stamp);
            }
            let base = state.borrow().base;
            let next = base
                .checked_add(1)
                .ok_or_else(|| error("cache lifetime exhausted"))?;
            let reply = Reply {
                id: request.id,
                payload: Payload::Preview {
                    revision,
                    delta: Delta {
                        base,
                        next,
                        ids,
                        changed,
                    },
                },
            };
            post(scope, state, reply, buffers)?;
            let mut s = state.borrow_mut();
            s.index = index;
            s.base = next;
            return Ok(());
        }
        Action::Export { prepared } => {
            let input = js_sys::JSON::parse(&prepared)?;
            let bodies = Array::from(&Reflect::get(&input, &"bodies".into())?);
            let keys = Array::new();
            for body in bodies.iter() {
                keys.push(&stamp(&body)?.key.into());
            }
            let result = export_cached_assembly(input, keys.into())?;
            if Reflect::get(&result, &"revision".into())?.as_f64()
                != Some(revision(&prepared)? as f64)
            {
                return Err(error("provider export revision mismatch"));
            }
            buffers.push(
                Reflect::get(&result, &"step".into())?
                    .dyn_into::<Uint8Array>()?
                    .buffer()
                    .into(),
            );
            let mesh = Reflect::get(&result, &"mesh".into())?;
            buffers.push(
                Reflect::get(&mesh, &"positions".into())?
                    .dyn_into::<Float32Array>()?
                    .buffer()
                    .into(),
            );
            buffers.push(
                Reflect::get(&mesh, &"normals".into())?
                    .dyn_into::<Float32Array>()?
                    .buffer()
                    .into(),
            );
            Payload::Export {
                revision: revision(&prepared)?,
            }
        }
        Action::ReadStep => {
            let bytes = Uint8Array::new(&buffer.ok_or_else(|| error("missing STEP transfer"))?);
            let model = read_step_model(bytes)?;
            let bounds = Reflect::get(&model, &"bounds".into())?;
            Payload::Model {
                min: triple(&Reflect::get(&bounds, &"min".into())?)?,
                max: triple(&Reflect::get(&bounds, &"max".into())?)?,
            }
        }
        Action::Status => {
            let s = state.borrow();
            Payload::Status {
                detached: s.detached,
                transferred_bytes: s.transferred_bytes,
            }
        }
        Action::Cancel { .. } => return Err(error("control should bypass job queue")),
        Action::Crash => return Err(error("crash should be dispatched as worker error")),
    };
    post(
        scope,
        state,
        Reply {
            id: request.id,
            payload,
        },
        buffers,
    )
}

fn error(message: &str) -> JsValue {
    JsValue::from_str(message)
}
fn revision(input: &str) -> Result<u64, JsValue> {
    let json: serde_json::Value = serde_json::from_str(input).map_err(|e| error(&e.to_string()))?;
    json["revision"]
        .as_u64()
        .ok_or_else(|| error("missing prepared revision"))
}
fn triple(value: &JsValue) -> Result<[f64; 3], JsValue> {
    let a = Array::from(value);
    Ok([
        a.get(0).as_f64().ok_or_else(|| error("invalid bounds"))?,
        a.get(1).as_f64().ok_or_else(|| error("invalid bounds"))?,
        a.get(2).as_f64().ok_or_else(|| error("invalid bounds"))?,
    ])
}
fn stamp(input: &JsValue) -> Result<BodyStamp, JsValue> {
    let body = Reflect::get(input, &"body".into())?;
    let id = Reflect::get(&body, &"id".into())?
        .as_string()
        .ok_or_else(|| error("body id missing"))?;
    let name = Reflect::get(&body, &"name".into())?
        .as_string()
        .ok_or_else(|| error("body name missing"))?;
    // Preserve the existing bodyKey contract, including JS numeric formatting and property order.
    let geometry = Object::assign(&Object::new(), &Object::from(body));
    Reflect::delete_property(&geometry, &"id".into())?;
    Reflect::delete_property(&geometry, &"name".into())?;
    let value = Object::new();
    Reflect::set(&value, &"body".into(), &geometry)?;
    Reflect::set(
        &value,
        &"regions".into(),
        &Reflect::get(input, &"regions".into())?,
    )?;
    let key = format!(
        "final-mesh:v1:0.1:0.5:{}",
        js_sys::JSON::stringify(&value)?
            .as_string()
            .ok_or_else(|| error("body key failed"))?
    );
    Ok(BodyStamp { id, name, key })
}
fn post(
    scope: &DedicatedWorkerGlobalScope,
    state: &Rc<RefCell<State>>,
    reply: Reply,
    buffers: Vec<JsValue>,
) -> Result<(), JsValue> {
    let object = Object::new();
    let frame = serde_json::to_string(&reply).map_err(|e| error(&e.to_string()))?;
    Reflect::set(&object, &"frame".into(), &frame.into())?;
    let transfer = Array::new();
    let mut bytes = 0;
    for buffer in &buffers {
        bytes += buffer.unchecked_ref::<ArrayBuffer>().byte_length() as u64;
        transfer.push(buffer);
    }
    Reflect::set(&object, &"buffers".into(), &transfer)?;
    scope.post_message_with_transfer(&object, &transfer)?;
    let mut s = state.borrow_mut();
    s.transferred_bytes += bytes;
    s.detached &= buffers
        .iter()
        .all(|b| b.unchecked_ref::<ArrayBuffer>().byte_length() == 0);
    Ok(())
}
