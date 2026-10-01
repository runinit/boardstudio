use boardstudio_core::{CoreEngine, archive};
use js_sys::{Array, Object, Reflect, Uint8Array};
use serde::Deserialize;
use serde_json::Value;
use wasm_bindgen::{JsCast, prelude::*};
use web_sys::{DedicatedWorkerGlobalScope, MessageEvent};

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
enum WorkerMessage {
    Core {
        request_id: String,
        executor_epoch: String,
        frame: String,
    },
    Archive {
        request_id: String,
        executor_epoch: String,
        metadata: String,
        buffers: Vec<u32>,
    },
}

#[wasm_bindgen]
pub fn start_core_worker() -> Result<(), JsValue> {
    let scope: DedicatedWorkerGlobalScope = js_sys::global().dyn_into()?;
    let target = scope.clone();
    let mut engine = CoreEngine::new();
    let callback = Closure::<dyn FnMut(MessageEvent)>::new(move |event: MessageEvent| {
        if let Err(error) = dispatch(&target, &mut engine, event.data()) {
            wasm_bindgen::throw_val(error);
        }
    });
    scope.set_onmessage(Some(callback.as_ref().unchecked_ref()));
    callback.forget();
    scope.post_message(&JsValue::from_str("boardstudio-core-ready"))
}

fn dispatch(
    scope: &DedicatedWorkerGlobalScope,
    engine: &mut CoreEngine,
    data: JsValue,
) -> Result<(), JsValue> {
    let message_json = js_sys::JSON::stringify(&data)?
        .as_string()
        .ok_or_else(|| JsValue::from_str("worker message could not be encoded"))?;
    let message: WorkerMessage = serde_json::from_str(&message_json)
        .map_err(|error| JsValue::from_str(&error.to_string()))?;
    match message {
        WorkerMessage::Core {
            request_id,
            executor_epoch,
            frame,
        } => {
            let request_value: Value = serde_json::from_str(&frame)
                .map_err(|error| JsValue::from_str(&error.to_string()))?;
            let request_frame_id = request_value
                .get("id")
                .and_then(Value::as_str)
                .ok_or_else(|| JsValue::from_str("core request has no string id"))?;
            if request_frame_id != request_id {
                return post_error(
                    scope,
                    &request_id,
                    executor_epoch,
                    "core request id mismatch",
                );
            }
            let reply_json = match serde_json::from_str(&frame) {
                Ok(request) => serde_json::to_string(&engine.handle(request))
                    .map_err(|error| JsValue::from_str(&error.to_string()))?,
                Err(error) => {
                    let reply = serde_json::json!({
                        "kind": "error",
                        "id": request_id,
                        "message": error.to_string(),
                        "revision": 0
                    });
                    serde_json::to_string(&reply)
                        .map_err(|error| JsValue::from_str(&error.to_string()))?
                }
            };
            let reply_id = serde_json::from_str::<Value>(&reply_json)
                .ok()
                .and_then(|value| value.get("id").and_then(Value::as_str).map(str::to_owned));
            if reply_id.as_deref() != Some(request_id.as_str()) {
                return post_error(scope, &request_id, executor_epoch, "core reply id mismatch");
            }
            let response = Object::new();
            Reflect::set(&response, &"kind".into(), &"core".into())?;
            Reflect::set(&response, &"request_id".into(), &request_id.into())?;
            Reflect::set(
                &response,
                &"executor_epoch".into(),
                &JsValue::from_str(&executor_epoch),
            )?;
            Reflect::set(&response, &"frame".into(), &reply_json.into())?;
            scope.post_message(&response)
        }
        WorkerMessage::Archive {
            request_id,
            executor_epoch,
            metadata,
            buffers,
        } => {
            let inputs = buffers
                .iter()
                .map(|index| {
                    Reflect::get(&data, &format!("buffer_{index}").into())
                        .map(|value| Uint8Array::new(&value).to_vec())
                })
                .collect::<Result<Vec<_>, _>>()?;
            let (reply, output_bytes) = archive::request(&metadata, &inputs);
            let response = Object::new();
            let output = Array::new();
            let transfers = Array::new();
            for bytes in output_bytes {
                let typed = Uint8Array::from(bytes.as_slice());
                let buffer = typed.buffer();
                output.push(&buffer);
                transfers.push(&buffer);
            }
            Reflect::set(&response, &"kind".into(), &"archive".into())?;
            Reflect::set(&response, &"request_id".into(), &request_id.into())?;
            Reflect::set(
                &response,
                &"executor_epoch".into(),
                &JsValue::from_str(&executor_epoch),
            )?;
            Reflect::set(&response, &"frame".into(), &reply.into())?;
            Reflect::set(&response, &"buffers".into(), &output)?;
            scope.post_message_with_transfer(&response, &transfers)
        }
    }
}

fn post_error(
    scope: &DedicatedWorkerGlobalScope,
    request_id: &str,
    executor_epoch: String,
    message: &str,
) -> Result<(), JsValue> {
    let response = Object::new();
    Reflect::set(&response, &"request_id".into(), &request_id.into())?;
    Reflect::set(
        &response,
        &"executor_epoch".into(),
        &JsValue::from_str(&executor_epoch),
    )?;
    Reflect::set(&response, &"error".into(), &message.into())?;
    scope.post_message(&response)
}
