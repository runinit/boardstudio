use std::{
    cell::{Cell, RefCell},
    collections::BTreeMap,
    rc::Rc,
};

use boardstudio_core::{CoreEngine, archive, model::*};
use js_sys::{Array, Function, JSON, Object, Promise, Reflect, Uint8Array};
use serde::Serialize;
use serde_json::Value;
use wasm_bindgen::{JsCast, JsValue, closure::Closure, prelude::wasm_bindgen};
use wasm_bindgen_futures::JsFuture;

const DB_NAME: &str = "boardstudio-p3-durability";
const PROJECT_STORE: &str = "projects";
const ASSET_STORE: &str = "assets";
const ACTIVE_PROJECT_KEY: &str = "boardstudio-p3-durability-active-project";

struct PendingSave {
    operation_id: String,
    project_json: String,
    assets: BTreeMap<String, Vec<u8>>,
    committed_revision: u64,
}

struct ProbeSession {
    engine: CoreEngine,
    original_document: ProjectDoc,
    assets: BTreeMap<String, Vec<u8>>,
    pending: Option<PendingSave>,
    commit_count: u32,
    save_attempts: u32,
}

thread_local! {
    static SESSION: RefCell<Option<ProbeSession>> = const { RefCell::new(None) };
}

fn js_error(message: impl AsRef<str>) -> JsValue {
    js_sys::Error::new(message.as_ref()).into()
}

fn property(value: &JsValue, key: &str) -> Result<JsValue, JsValue> {
    Reflect::get(value, &JsValue::from_str(key))
}

fn call(value: &JsValue, key: &str, args: &[JsValue]) -> Result<JsValue, JsValue> {
    let function = property(value, key)?.dyn_into::<Function>()?;
    let arguments = Array::new();
    for arg in args {
        arguments.push(arg);
    }
    function.apply(value, &arguments)
}

fn json_value(value: &Value) -> Result<JsValue, JsValue> {
    value
        .serialize(&serde_wasm_bindgen::Serializer::json_compatible())
        .map_err(|error| js_error(error.to_string()))
}

async fn request_result(request: &JsValue) -> Result<JsValue, JsValue> {
    let request = request.clone();
    let promise = Promise::new(&mut |resolve, reject| {
        let success_request = request.clone();
        let success = Closure::once_into_js(move |_event: JsValue| {
            let result = Reflect::get(&success_request, &JsValue::from_str("result"))
                .unwrap_or(JsValue::UNDEFINED);
            let _ = resolve.call1(&JsValue::UNDEFINED, &result);
        });
        let error_request = request.clone();
        let failure = Closure::once_into_js(move |_event: JsValue| {
            let reason = Reflect::get(&error_request, &JsValue::from_str("error"))
                .unwrap_or_else(|_| js_error("IndexedDB request failed"));
            let _ = reject.call1(&JsValue::UNDEFINED, &reason);
        });
        let _ = Reflect::set(&request, &JsValue::from_str("onsuccess"), &success);
        let _ = Reflect::set(&request, &JsValue::from_str("onerror"), &failure);
    });
    JsFuture::from(promise).await
}

async fn transaction_result(
    transaction: &JsValue,
    successful_requests: Rc<Cell<u32>>,
) -> Result<u32, JsValue> {
    let transaction = transaction.clone();
    let promise = Promise::new(&mut |resolve, reject| {
        let complete_requests = successful_requests.clone();
        let success = Closure::once_into_js(move |_event: JsValue| {
            let _ = resolve.call1(
                &JsValue::UNDEFINED,
                &JsValue::from_f64(complete_requests.get() as f64),
            );
        });
        let failure_tx = transaction.clone();
        let abort_requests = successful_requests.clone();
        let failure = Closure::once_into_js(move |_event: JsValue| {
            let error = js_sys::Error::new("IndexedDB transaction abort event");
            let event_name = Reflect::get(&failure_tx, &JsValue::from_str("error"))
                .ok()
                .and_then(|value| property(&value, "name").ok())
                .and_then(|value| value.as_string())
                .unwrap_or_else(|| "AbortError".into());
            error.set_name(&event_name);
            let _ = Reflect::set(
                &error,
                &JsValue::from_str("transactionEvent"),
                &JsValue::from_str("abort"),
            );
            let _ = Reflect::set(
                &error,
                &JsValue::from_str("requestsSucceeded"),
                &JsValue::from_f64(abort_requests.get() as f64),
            );
            let _ = reject.call1(&JsValue::UNDEFINED, &error);
        });
        let _ = Reflect::set(&transaction, &JsValue::from_str("oncomplete"), &success);
        let _ = Reflect::set(&transaction, &JsValue::from_str("onabort"), &failure);
    });
    let result = JsFuture::from(promise).await?;
    Ok(result.as_f64().unwrap_or_default() as u32)
}

fn observe_write_request(
    request: &JsValue,
    successful_requests: &Rc<Cell<u32>>,
    abort_transaction: Option<JsValue>,
) -> Result<(), JsValue> {
    let successful_requests = successful_requests.clone();
    let success = Closure::once_into_js(move |_event: JsValue| {
        successful_requests.set(successful_requests.get() + 1);
        if let Some(transaction) = abort_transaction {
            let _ = call(&transaction, "abort", &[]);
        }
    });
    Reflect::set(request, &JsValue::from_str("onsuccess"), &success)?;
    Ok(())
}

async fn open_database() -> Result<JsValue, JsValue> {
    let indexed_db = property(&js_sys::global(), "indexedDB")?;
    if indexed_db.is_null() || indexed_db.is_undefined() {
        return Err(js_error("IndexedDB is unavailable in this context"));
    }
    let request = call(
        &indexed_db,
        "open",
        &[JsValue::from_str(DB_NAME), JsValue::from_f64(1.0)],
    )?;
    let open_request = request.clone();
    let upgrade = Closure::once_into_js(move |_event: JsValue| {
        let result = Reflect::get(&open_request, &JsValue::from_str("result"));
        if let Ok(database) = result {
            let options = Object::new();
            let _ = Reflect::set(
                &options,
                &JsValue::from_str("keyPath"),
                &JsValue::from_str("id"),
            );
            let _ = call(
                &database,
                "createObjectStore",
                &[JsValue::from_str(PROJECT_STORE), options.into()],
            );
            let _ = call(
                &database,
                "createObjectStore",
                &[JsValue::from_str(ASSET_STORE)],
            );
        }
    });
    Reflect::set(&request, &JsValue::from_str("onupgradeneeded"), &upgrade)?;
    request_result(&request).await
}

fn begin_transaction(database: &JsValue, stores: &[&str], mode: &str) -> Result<JsValue, JsValue> {
    let names = Array::new();
    for store in stores {
        names.push(&JsValue::from_str(store));
    }
    call(
        database,
        "transaction",
        &[names.into(), JsValue::from_str(mode)],
    )
}

fn object_store(transaction: &JsValue, store: &str) -> Result<JsValue, JsValue> {
    call(transaction, "objectStore", &[JsValue::from_str(store)])
}

async fn save_document_and_assets(
    document: &JsValue,
    assets: &BTreeMap<String, Vec<u8>>,
    abort_after_project_request: bool,
) -> Result<u32, JsValue> {
    let database = open_database().await?;
    let transaction = begin_transaction(&database, &[PROJECT_STORE, ASSET_STORE], "readwrite")?;
    let successful_requests = Rc::new(Cell::new(0));
    let project_store = object_store(&transaction, PROJECT_STORE)?;
    let asset_store = object_store(&transaction, ASSET_STORE)?;
    let project_request = call(&project_store, "put", std::slice::from_ref(document))?;
    observe_write_request(
        &project_request,
        &successful_requests,
        abort_after_project_request.then(|| transaction.clone()),
    )?;
    for (hash, bytes) in assets {
        let value = Uint8Array::from(bytes.as_slice());
        let request = call(
            &asset_store,
            "put",
            &[value.into(), JsValue::from_str(hash)],
        )?;
        observe_write_request(&request, &successful_requests, None)?;
    }
    let outcome = transaction_result(&transaction, successful_requests).await;
    let _ = call(&database, "close", &[]);
    let requests_succeeded = outcome?;
    if let Ok(storage) = property(&js_sys::global(), "localStorage") {
        let id = property(document, "id")?;
        call(
            &storage,
            "setItem",
            &[JsValue::from_str(ACTIVE_PROJECT_KEY), id],
        )?;
    }
    Ok(requests_succeeded)
}

async fn load_value(store_name: &str, key: &str) -> Result<JsValue, JsValue> {
    let database = open_database().await?;
    let transaction = begin_transaction(&database, &[store_name], "readonly")?;
    let store = object_store(&transaction, store_name)?;
    let request = call(&store, "get", &[JsValue::from_str(key)])?;
    let result = request_result(&request).await;
    let _ = call(&database, "close", &[]);
    result
}

fn unpack_bytes(bytes: &[u8]) -> Result<(String, BTreeMap<String, Vec<u8>>), JsValue> {
    let metadata = r#"{"kind":"unpack-project"}"#;
    let (reply_json, outputs) = archive::request(metadata, &[bytes.to_vec()]);
    let reply: Value =
        serde_json::from_str(&reply_json).map_err(|error| js_error(error.to_string()))?;
    if reply["kind"] != "unpacked" {
        return Err(js_error(
            reply["message"]
                .as_str()
                .unwrap_or("Archive validation failed"),
        ));
    }
    let project_json = reply["projectJson"]
        .as_str()
        .ok_or_else(|| js_error("Archive reply omitted project JSON"))?
        .to_owned();
    let mut assets = BTreeMap::new();
    for asset in reply["assets"]
        .as_array()
        .ok_or_else(|| js_error("Archive reply omitted assets"))?
    {
        let hash = asset["sha256"]
            .as_str()
            .ok_or_else(|| js_error("Archive asset omitted hash"))?;
        let index = asset["bufferIndex"]
            .as_u64()
            .ok_or_else(|| js_error("Archive asset omitted buffer index"))?
            as usize;
        let bytes = outputs
            .get(index)
            .ok_or_else(|| js_error("Archive reply omitted asset bytes"))?;
        assets.insert(hash.to_owned(), bytes.clone());
    }
    Ok((project_json, assets))
}

#[wasm_bindgen]
pub async fn import_fixture() -> Result<JsValue, JsValue> {
    let fetch = property(&js_sys::global(), "fetch")?.dyn_into::<Function>()?;
    let response = JsFuture::from(
        fetch
            .call1(
                &js_sys::global(),
                &JsValue::from_str("fixtures/reviung41-original.boardstudio"),
            )?
            .dyn_into::<Promise>()?,
    )
    .await?;
    if !property(&response, "ok")?.as_bool().unwrap_or(false) {
        return Err(js_error(
            "The copied REVIUNG41 archive could not be fetched",
        ));
    }
    let buffer =
        JsFuture::from(call(&response, "arrayBuffer", &[])?.dyn_into::<Promise>()?).await?;
    let archive_bytes = Uint8Array::new(&buffer).to_vec();
    let (project_json, assets) = unpack_bytes(&archive_bytes)?;
    let project: ProjectDoc =
        serde_json::from_str(&project_json).map_err(|error| js_error(error.to_string()))?;
    if project.name != "REVIUNG41"
        || project.assets.len() != assets.len()
        || project.parts.len() != 85
    {
        return Err(js_error(
            "The copied project does not match the expected REVIUNG41 fixture",
        ));
    }
    let document = json_value(
        &serde_json::from_str::<Value>(&project_json)
            .map_err(|error| js_error(error.to_string()))?,
    )?;
    save_document_and_assets(&document, &assets, false).await?;

    let mut engine = CoreEngine::new();
    let opened = engine.handle(CoreRequest::Open {
        id: "p3-open-reviung41".into(),
        document: project.clone(),
    });
    if !matches!(opened, CoreReply::Scene { .. }) {
        return Err(js_error(
            "The copied REVIUNG41 document did not open in CoreEngine",
        ));
    }
    SESSION.with(|slot| {
        *slot.borrow_mut() = Some(ProbeSession {
            engine,
            original_document: project,
            assets: assets.clone(),
            pending: None,
            commit_count: 0,
            save_attempts: 0,
        });
    });
    Ok(summary(&document, &assets))
}

fn summary(document: &JsValue, assets: &BTreeMap<String, Vec<u8>>) -> JsValue {
    let result = Object::new();
    let _ = Reflect::set(
        &result,
        &JsValue::from_str("id"),
        &property(document, "id").unwrap_or(JsValue::UNDEFINED),
    );
    let _ = Reflect::set(
        &result,
        &JsValue::from_str("name"),
        &property(document, "name").unwrap_or(JsValue::UNDEFINED),
    );
    let _ = Reflect::set(
        &result,
        &JsValue::from_str("partCount"),
        &JsValue::from_f64(
            property(document, "parts")
                .map(|parts| Array::from(&parts).length() as f64)
                .unwrap_or_default(),
        ),
    );
    let _ = Reflect::set(
        &result,
        &JsValue::from_str("assetCount"),
        &JsValue::from_f64(assets.len() as f64),
    );
    result.into()
}

#[wasm_bindgen]
pub async fn load_stored_project() -> Result<JsValue, JsValue> {
    let storage = property(&js_sys::global(), "localStorage")?;
    let active_id = call(
        &storage,
        "getItem",
        &[JsValue::from_str(ACTIVE_PROJECT_KEY)],
    )?;
    let Some(active_id) = active_id.as_string() else {
        return Err(js_error("No active copied project is stored"));
    };
    let document = load_value(PROJECT_STORE, &active_id).await?;
    if document.is_undefined() {
        return Err(js_error(
            "The active copied project is missing from IndexedDB",
        ));
    }
    let assets = Array::from(&property(&document, "assets")?);
    let mut verified = Vec::new();
    for asset in assets.iter() {
        let hash = property(&asset, "sha256")?
            .as_string()
            .ok_or_else(|| js_error("Stored asset has no hash"))?;
        let bytes = load_value(ASSET_STORE, &hash).await?;
        if bytes.is_undefined() {
            return Err(js_error(format!("Missing stored asset {hash}")));
        }
        verified.push(hash);
    }
    let checked = Object::new();
    Reflect::set(&checked, &JsValue::from_str("document"), &document)?;
    Reflect::set(
        &checked,
        &JsValue::from_str("assets"),
        &JsValue::from_str(&verified.join(",")),
    )?;
    Reflect::set(
        &checked,
        &JsValue::from_str("assetCount"),
        &JsValue::from_f64(verified.len() as f64),
    )?;
    Ok(checked.into())
}

#[wasm_bindgen]
pub fn commit_one_edit() -> Result<JsValue, JsValue> {
    SESSION.with(|slot| {
        let mut slot = slot.borrow_mut();
        let session = slot
            .as_mut()
            .ok_or_else(|| js_error("No imported CoreEngine session"))?;
        if session.pending.is_some() {
            return Err(js_error(
                "A committed snapshot is already awaiting save recovery",
            ));
        }
        let part = session
            .original_document
            .parts
            .first()
            .ok_or_else(|| js_error("REVIUNG41 has no parts"))?;
        let id = part.id.clone();
        let target = Vec2 {
            x: part.pose.at.x + 1.0,
            y: part.pose.at.y,
        };
        let reply = session.engine.handle(CoreRequest::Edit {
            id: "p3-single-edit".into(),
            command: EditCommand {
                base_revision: session.original_document.revision,
                transaction_id: "p3-durability-edit-1".into(),
                phase: EditPhase::Commit,
                target_ids: vec![id.clone()],
                operation: EditOperation::MoveParts {
                    positions: vec![Position { id, at: target }],
                },
            },
        });
        let CoreReply::Scene { document, .. } = reply else {
            return Err(js_error(format!("CoreEngine edit failed: {reply:?}")));
        };
        session.commit_count += 1;
        let value = serde_json::to_value(&document).map_err(|error| js_error(error.to_string()))?;
        let js_document = json_value(&value)?;
        let project_json = JSON::stringify(&js_document)?
            .as_string()
            .ok_or_else(|| js_error("Committed snapshot did not serialize"))?;
        let operation_id = "p3-durability-edit-1".to_owned();
        session.pending = Some(PendingSave {
            operation_id,
            project_json,
            assets: session.assets.clone(),
            committed_revision: document.revision,
        });
        Ok(summary(&js_document, &session.assets))
    })
}

#[wasm_bindgen]
pub async fn save_pending(force_abort: bool) -> Result<JsValue, JsValue> {
    let (project_json, assets, operation_id, attempt, revision, commits) =
        SESSION.with(|slot| {
            let mut slot = slot.borrow_mut();
            let session = slot
                .as_mut()
                .ok_or_else(|| js_error("No imported CoreEngine session"))?;
            let pending = session
                .pending
                .as_ref()
                .ok_or_else(|| js_error("There is no retained committed snapshot to save"))?;
            session.save_attempts += 1;
            Ok::<_, JsValue>((
                pending.project_json.clone(),
                pending.assets.clone(),
                pending.operation_id.clone(),
                session.save_attempts,
                pending.committed_revision,
                session.commit_count,
            ))
        })?;
    let document = JSON::parse(&project_json)?;
    let requests_succeeded = save_document_and_assets(&document, &assets, force_abort).await?;
    Ok(save_receipt(
        &operation_id,
        attempt,
        revision,
        commits,
        requests_succeeded,
    ))
}

#[wasm_bindgen]
pub async fn retry_pending() -> Result<JsValue, JsValue> {
    save_pending(false).await
}

fn save_receipt(
    operation_id: &str,
    attempt: u32,
    revision: u64,
    commits: u32,
    requests_succeeded: u32,
) -> JsValue {
    let result = Object::new();
    let _ = Reflect::set(
        &result,
        &JsValue::from_str("operationId"),
        &JsValue::from_str(operation_id),
    );
    let _ = Reflect::set(
        &result,
        &JsValue::from_str("saveAttempt"),
        &JsValue::from_f64(attempt as f64),
    );
    let _ = Reflect::set(
        &result,
        &JsValue::from_str("revision"),
        &JsValue::from_f64(revision as f64),
    );
    let _ = Reflect::set(
        &result,
        &JsValue::from_str("engineCommitCount"),
        &JsValue::from_f64(commits as f64),
    );
    let _ = Reflect::set(
        &result,
        &JsValue::from_str("transactionEvent"),
        &JsValue::from_str("complete"),
    );
    let _ = Reflect::set(
        &result,
        &JsValue::from_str("requestsSucceeded"),
        &JsValue::from_f64(requests_succeeded as f64),
    );
    result.into()
}

#[wasm_bindgen]
pub fn undo_once() -> Result<JsValue, JsValue> {
    SESSION.with(|slot| {
        let mut slot = slot.borrow_mut();
        let session = slot
            .as_mut()
            .ok_or_else(|| js_error("No imported CoreEngine session"))?;
        let reply = session.engine.handle(CoreRequest::Undo {
            id: "p3-check-undo".into(),
        });
        let CoreReply::Scene { document, .. } = reply else {
            return Err(js_error(format!("Undo failed: {reply:?}")));
        };
        let result = Object::new();
        let _ = Reflect::set(
            &result,
            &JsValue::from_str("revision"),
            &JsValue::from_f64(document.revision as f64),
        );
        let _ = Reflect::set(
            &result,
            &JsValue::from_str("partX"),
            &JsValue::from_f64(document.parts[0].pose.at.x),
        );
        Ok(result.into())
    })
}

#[wasm_bindgen]
pub async fn archive_stored_project_roundtrip() -> Result<JsValue, JsValue> {
    let stored = load_stored_project().await?;
    let document = property(&stored, "document")?;
    let project_json = JSON::stringify(&document)?
        .as_string()
        .ok_or_else(|| js_error("Stored project did not serialize"))?;
    let assets = Array::from(&property(&document, "assets")?);
    let mut records = BTreeMap::new();
    for asset in assets.iter() {
        let hash = property(&asset, "sha256")?
            .as_string()
            .ok_or_else(|| js_error("Stored asset has no hash"))?;
        let bytes_value = load_value(ASSET_STORE, &hash).await?;
        let bytes = Uint8Array::new(&bytes_value).to_vec();
        records.insert(hash, bytes);
    }
    let entries: Vec<_> = records.keys().enumerate().map(|(index, hash)| serde_json::json!({ "path": format!("assets/{hash}"), "bufferIndex": index })).collect();
    let metadata = serde_json::json!({ "kind": "pack-project", "projectJson": project_json, "assets": entries }).to_string();
    let input_buffers: Vec<_> = records.values().cloned().collect();
    let (packed_reply, packed) = archive::request(&metadata, &input_buffers);
    let packed_reply: Value =
        serde_json::from_str(&packed_reply).map_err(|error| js_error(error.to_string()))?;
    if packed_reply["kind"] != "packed" {
        return Err(js_error(
            packed_reply["message"]
                .as_str()
                .unwrap_or("Rust pack failed"),
        ));
    }
    let archive_bytes = packed
        .first()
        .ok_or_else(|| js_error("Rust pack omitted archive bytes"))?;
    let (reopened_json, reopened_assets) = unpack_bytes(archive_bytes)?;
    if serde_json::from_str::<Value>(&reopened_json).map_err(|error| js_error(error.to_string()))?
        != serde_json::from_str::<Value>(&project_json)
            .map_err(|error| js_error(error.to_string()))?
        || reopened_assets != records
    {
        return Err(js_error(
            "Stored project/assets differ after Rust archive roundtrip",
        ));
    }
    let output = Object::new();
    Reflect::set(
        &output,
        &JsValue::from_str("projectId"),
        &property(&document, "id")?,
    )?;
    Reflect::set(
        &output,
        &JsValue::from_str("assetCount"),
        &JsValue::from_f64(reopened_assets.len() as f64),
    )?;
    Reflect::set(
        &output,
        &JsValue::from_str("hashes"),
        &JsValue::from_str(
            &reopened_assets
                .keys()
                .cloned()
                .collect::<Vec<_>>()
                .join(","),
        ),
    )?;
    Ok(output.into())
}
