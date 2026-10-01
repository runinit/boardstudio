use crate::{Action, Identity, Reply, Request, ResultPayload, execute};
use boardstudio_core::CoreEngine;
use js_sys::{Array, Object, Reflect};
use wasm_bindgen::{JsCast, prelude::*};
use web_sys::{DedicatedWorkerGlobalScope, MessageEvent};

#[wasm_bindgen]
pub fn start_worker() -> Result<(), JsValue> {
    let scope: DedicatedWorkerGlobalScope = js_sys::global().dyn_into()?;
    let target = scope.clone();
    let mut engine = CoreEngine::new();
    let callback = Closure::<dyn FnMut(MessageEvent)>::new(move |event: MessageEvent| {
        if let Err(error) = dispatch(&target, &mut engine, event.data()) {
            wasm_bindgen::throw_val(error);
        }
    });
    scope.set_onmessage(Some(callback.as_ref().unchecked_ref()));
    // This registration lives exactly as long as this dedicated worker.
    callback.forget();
    scope.post_message(&JsValue::from_str("ready"))
}

fn dispatch(
    scope: &DedicatedWorkerGlobalScope,
    engine: &mut CoreEngine,
    data: JsValue,
) -> Result<(), JsValue> {
    let frame = Reflect::get(&data, &"frame".into())?
        .as_string()
        .ok_or_else(|| JsValue::from_str("request frame must be JSON text"))?;
    let request: Request =
        serde_json::from_str(&frame).map_err(|error| JsValue::from_str(&error.to_string()))?;
    if matches!(request.action, Action::Crash) {
        wasm_bindgen::throw_str("intentional P1 worker crash");
    }
    if matches!(request.action, Action::Noise) {
        scope.post_message(&JsValue::from_str("invalid unsolicited payload"))?;
        post(
            scope,
            Reply {
                id: Identity {
                    epoch: request.id.epoch.wrapping_sub(1),
                    operation: request.id.operation,
                },
                payload: ResultPayload::Ack,
            },
            None,
        )?;
        post(
            scope,
            Reply {
                id: Identity {
                    epoch: request.id.epoch,
                    operation: request.id.operation.wrapping_add(1000),
                },
                payload: ResultPayload::Ack,
            },
            None,
        )?;
    }
    let buffer = if matches!(request.action, Action::Echo) {
        Some(Reflect::get(&data, &"buffer".into())?)
    } else {
        None
    };
    let reply =
        execute(engine, request).ok_or_else(|| JsValue::from_str("missing execution reply"))?;
    post(scope, reply, buffer)
}

fn post(
    scope: &DedicatedWorkerGlobalScope,
    reply: Reply,
    buffer: Option<JsValue>,
) -> Result<(), JsValue> {
    let message = Object::new();
    Reflect::set(
        &message,
        &"frame".into(),
        &JsValue::from_str(
            &serde_json::to_string(&reply)
                .map_err(|error| JsValue::from_str(&error.to_string()))?,
        ),
    )?;
    if let Some(buffer) = buffer {
        Reflect::set(&message, &"buffer".into(), &buffer)?;
        let transfer = Array::new();
        transfer.push(&buffer);
        scope.post_message_with_transfer(&message, &transfer)
    } else {
        scope.post_message(&message)
    }
}
