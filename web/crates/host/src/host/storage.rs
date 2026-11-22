mod database;
mod startup;
use database::CachedDatabase;
#[cfg(test)]
use database::open_database;
use crate::persistence_contract::{sha256_bytes, validate_exact_json_integers};
use boardstudio_model::model::ProjectDoc;
use futures_channel::oneshot;
use js_sys::{Array, Reflect, Uint8Array};
use serde::Serialize;
pub use startup::StorageSession;
use std::{
    cell::RefCell,
    collections::{BTreeMap, BTreeSet},
    fmt,
    rc::Rc,
};
use wasm_bindgen::{JsCast, JsValue, prelude::*};
use wasm_bindgen_futures::spawn_local;
use web_sys::{
    Event, IdbRequest, IdbTransaction, IdbTransactionMode,
    Window,
};

// IndexedDB event handlers must outlive a cancelled caller.
async fn storage_task<T: 'static, E: From<PersistError> + 'static>(
    task: impl std::future::Future<Output = Result<T, E>> + 'static,
) -> Result<T, E> {
    let (send, receive) = oneshot::channel();
    spawn_local(async move {
        let _ = send.send(task.await);
    });
    receive
        .await
        .map_err(|_| PersistError("storage task was abandoned".into()))?
}

const PROJECT_STORE: &str = "projects";
const ASSET_STORE: &str = "assets";
const PROJECT_TOKEN_STORE: &str = "project_tokens";
const ACTIVE_PROJECT_KEY: &str = "boardstudio-m1-active-project";
pub const CONFLICT: &str =
    "This keyboard was changed or deleted in another tab. Your pending edits have been preserved.";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StorageToken(String);

impl StorageToken {
    pub fn for_document(document: &ProjectDoc) -> Result<Self, PersistError> {
        Self::from_stored(serialize_document(document)?)
    }

    fn new_nonce() -> Result<Self, PersistError> {
        crate::browser::browser_uuid()
            .map(|uuid| Self(format!("nonce:{uuid}")))
            .map_err(PersistError)
    }

    fn from_stored(value: JsValue) -> Result<Self, PersistError> {
        // IndexedDB stores JS numbers: 2.0 becomes 2, and -0.0 becomes 0 on JSON decode.
        Self::from_value(serde_wasm_bindgen::from_value(value).map_err(js_decode_error)?)
    }

    fn from_value(mut value: serde_json::Value) -> Result<Self, PersistError> {
        value.sort_all_objects();
        let bytes = serde_json::to_vec(&value).map_err(|error| PersistError(error.to_string()))?;
        Ok(Self(sha256_bytes(&bytes)))
    }
}

#[derive(Clone, Debug)]
pub struct SavedProject {
    pub document: ProjectDoc,
    pub token: StorageToken,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SaveError {
    Conflict,
    Storage(String),
}

impl fmt::Display for SaveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Conflict => f.write_str(CONFLICT),
            Self::Storage(message) => f.write_str(message),
        }
    }
}

impl From<PersistError> for SaveError {
    fn from(error: PersistError) -> Self {
        Self::Storage(error.0)
    }
}

type RequestHandler = Closure<dyn FnMut(Event)>;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PersistError(pub String);

impl fmt::Display for PersistError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for PersistError {}

#[derive(Clone, Debug)]
pub struct AssetBytes {
    pub sha256: String,
    pub bytes: Vec<u8>,
}

#[derive(Clone, Debug, Default)]
pub struct ProjectListing {
    pub documents: Vec<ProjectDoc>,
    pub unreadable: Vec<UnreadableProject>,
}

#[derive(Clone, Debug)]
pub struct UnreadableProject {
    pub key: JsValue,
    pub label: String,
    pub error: String,
    pub token: Option<StorageToken>,
}

#[derive(Clone, Debug)]
pub struct BrowserStore {
    database_name: String,
    database: Rc<RefCell<Option<CachedDatabase>>>,
    observed: Rc<RefCell<BTreeMap<String, StorageToken>>>,
}

impl BrowserStore {
    pub fn new(database_name: impl Into<String>) -> Result<Self, PersistError> {
        let database_name = database_name.into();
        if database_name.is_empty() || database_name == "boardstudio-v2" {
            return Err(PersistError(
                "M1 storage requires a database separate from the React reference".into(),
            ));
        }
        Ok(Self {
            database_name,
            database: Rc::default(),
            observed: Rc::default(),
        })
    }

    pub fn scoped(scope: &str) -> Result<Self, PersistError> {
        if scope.is_empty()
            || !scope
                .chars()
                .all(|character| character.is_ascii_alphanumeric() || character == '-')
        {
            return Err(PersistError(
                "storage scope must use letters, digits or hyphens".into(),
            ));
        }
        Self::new(format!("boardstudio-m1-{scope}"))
    }

    pub async fn save_document(
        &self,
        document: &ProjectDoc,
        assets: &std::collections::BTreeMap<String, Vec<u8>>,
    ) -> Result<(), PersistError> {
        let expected = self.observed.borrow().get(&document.id).cloned();
        self.save_document_expected(document, assets, expected.as_ref())
            .await
            .map(|_| ())
            .map_err(|error| PersistError(error.to_string()))
    }

    pub async fn save_document_expected(
        &self,
        document: &ProjectDoc,
        assets: &BTreeMap<String, Vec<u8>>,
        expected: Option<&StorageToken>,
    ) -> Result<StorageToken, SaveError> {
        let assets = assets
            .iter()
            .map(|(sha256, bytes)| AssetBytes {
                sha256: sha256.clone(),
                bytes: bytes.clone(),
            })
            .collect();
        let stored = serialize_document(document)?;
        let token = StorageToken::new_nonce()?;
        let required_assets = document.assets.iter().map(|asset| asset.sha256.clone()).collect();
        let key = JsValue::from_str(&document.id);
        let expected = expected.cloned();
        let store = self.clone();
        storage_task(async move {
            store.change_project(
                key.clone(), Some(expected), Some((stored, Some(token.clone()))),
                required_assets, assets,
            ).await?;
            let project_id = key.as_string().expect("document id is a string");
            if let Ok(window) = window()
                && let Ok(Some(storage)) = window.local_storage()
            {
                let _ = storage.set_item(&store.active_key(), &project_id);
            }
            store.observe_token(project_id, token.clone());
            Ok(token)
        })
        .await
    }

    pub async fn load_document(
        &self,
        project_id: String,
    ) -> Result<Option<ProjectDoc>, PersistError> {
        let saved = self.load_document_with_token(project_id).await?;
        if let Some(saved) = &saved {
            self.observe_token(saved.document.id.clone(), saved.token.clone());
        }
        Ok(saved.map(|saved| saved.document))
    }

    pub async fn load_document_with_token(
        &self,
        project_id: String,
    ) -> Result<Option<SavedProject>, PersistError> {
        let store = self.clone();
        storage_task(async move {
            let Some((stored, nonce)) = store.read_project(project_id.into()).await? else {
                return Ok(None);
            };
            let json = serde_wasm_bindgen::from_value(stored).map_err(js_decode_error)?;
            let token = project_token(nonce, &json)?;
            let document = decode_project_json(json)?;
            Ok(Some(SavedProject { document, token }))
        })
        .await
    }

    pub async fn list_documents(&self) -> Result<Vec<ProjectDoc>, PersistError> {
        Ok(self.list_projects().await?.documents)
    }

    pub async fn list_projects(&self) -> Result<ProjectListing, PersistError> {
        let store = self.clone();
        storage_task(async move {
            let transaction = store.transaction(
                &[PROJECT_STORE, PROJECT_TOKEN_STORE], IdbTransactionMode::Readonly,
            ).await?;
            let completion = transaction_completion(&transaction, Rc::default());
            let projects = transaction.object_store(PROJECT_STORE).map_err(js_error)?;
            let tokens = transaction.object_store(PROJECT_TOKEN_STORE).map_err(js_error)?;
            let keys = request_result(projects.get_all_keys().map_err(js_error)?);
            let values = request_result(projects.get_all().map_err(js_error)?);
            let token_keys = request_result(tokens.get_all_keys().map_err(js_error)?);
            let token_values = request_result(tokens.get_all().map_err(js_error)?);
            let (keys, values) = (keys.await?, values.await?);
            let nonces: BTreeMap<_, _> = Array::from(&token_keys.await?).iter()
                .zip(Array::from(&token_values.await?).iter())
                .filter_map(|(key, value)| key.as_string().map(|key| (key, value)))
                .collect();
            completion.await?;
            let mut listing = ProjectListing::default();
            for (key, stored) in Array::from(&keys).iter().zip(Array::from(&values).iter()) {
                let nonce = key.as_string().and_then(|id| nonces.get(&id).cloned())
                    .unwrap_or(JsValue::UNDEFINED);
                let json = serde_wasm_bindgen::from_value(stored).map_err(invalid_project);
                let token = json.as_ref().ok().and_then(|json| project_token(nonce, json).ok());
                match json.and_then(decode_project_json) {
                    Ok(document) => {
                        if let Some(token) = token {
                            // A library refresh must not advance the baseline of an active editor.
                            store.observed.borrow_mut().entry(document.id.clone()).or_insert(token);
                        }
                        listing.documents.push(document);
                    }
                    Err(error) => listing.unreadable.push(UnreadableProject {
                        label: key.as_string().unwrap_or_else(|| format!("{key:?}")),
                        key, error: error.0, token,
                    }),
                }
            }
            Ok(listing)
        })
        .await
    }

    pub async fn raw_project(
        &self,
        key: &JsValue,
    ) -> Result<Option<(String, StorageToken)>, PersistError> {
        let store = self.clone();
        let key = key.clone();
        storage_task(async move {
            let Some((stored, nonce)) = store.read_project(key).await? else {
                return Ok(None);
            };
            let json = serde_wasm_bindgen::from_value(stored).map_err(js_decode_error)?;
            let token = project_token(nonce, &json)?;
            let json = serde_json::to_string_pretty(&json)
                .map_err(|error| PersistError(error.to_string()))?;
            Ok(Some((json, token)))
        }).await
    }

    async fn read_project(&self, key: JsValue) -> Result<Option<(JsValue, JsValue)>, PersistError> {
        let transaction = self.transaction(
            &[PROJECT_STORE, PROJECT_TOKEN_STORE], IdbTransactionMode::Readonly,
        ).await?;
        let projects = transaction.object_store(PROJECT_STORE).map_err(js_error)?;
        let tokens = transaction.object_store(PROJECT_TOKEN_STORE).map_err(js_error)?;
        let completion = transaction_completion(&transaction, Rc::default());
        let project = request_result(projects.get(&key).map_err(js_error)?);
        let token = request_result(tokens.get(&key).map_err(js_error)?);
        let (project, token) = (project.await?, token.await?);
        completion.await?;
        Ok((!project.is_undefined()).then_some((project, token)))
    }

    pub async fn delete_unreadable_by_key(
        &self,
        record: &UnreadableProject,
    ) -> Result<(), PersistError> {
        self.delete_key(record.key.clone(), None).await
    }

    pub async fn save_asset(&self, asset: AssetBytes) -> Result<(), PersistError> {
        verify_asset(&asset)?;
        let store = self.clone();
        storage_task(async move {
            let transaction = store.transaction(&[ASSET_STORE], IdbTransactionMode::Readwrite).await?;
            let completion = transaction_completion(&transaction, Rc::default());
            transaction.object_store(ASSET_STORE).map_err(js_error)?
                .put_with_key(&Uint8Array::from(asset.bytes.as_slice()), &asset.sha256.into())
                .map_err(js_error)?;
            completion.await
        }).await
    }

    pub async fn load_asset(&self, sha256: String) -> Result<Option<Uint8Array>, PersistError> {
        let store = self.clone();
        storage_task(async move {
            load_value(&store, ASSET_STORE, JsValue::from_str(&sha256))
                .await
                .and_then(|value| {
                    value
                        .map(|value| {
                            if !value.is_instance_of::<Uint8Array>() {
                                return Err(PersistError(
                                    "stored asset is not a Uint8Array".into(),
                                ));
                            }
                            let bytes = Uint8Array::new(&value);
                            if sha256_bytes(&bytes.to_vec()) != sha256 {
                                return Err(PersistError(format!(
                                    "stored asset hash mismatch: {sha256}"
                                )));
                            }
                            Ok(bytes)
                        })
                        .transpose()
                })
        })
        .await
    }

    pub async fn delete_project(&self, project_id: String) -> Result<(), PersistError> {
        let expected = self.observed.borrow().get(&project_id).cloned();
        self.delete_project_expected(project_id, expected.as_ref())
            .await
    }

    /// Store one raw record without validation, so tests can fabricate entries
    /// the typed writers never produce (for example values JSON cannot
    /// represent, which decode to unreadable records with no token).
    #[cfg(any(test, feature = "test-support"))]
    pub async fn put_raw_record_for_test(&self, value: JsValue) -> Result<(), PersistError> {
        let store = self.clone();
        storage_task(async move {
            let key = Reflect::get(&value, &"id".into()).map_err(js_error)?;
            let result = store.change_project(key, None, Some((value, None)), vec![], vec![]).await;
            result.map_err(|error| PersistError(error.to_string()))
        }).await
    }

    pub async fn delete_project_expected(
        &self,
        project_id: String,
        expected: Option<&StorageToken>,
    ) -> Result<(), PersistError> {
        self.delete_key(project_id.into(), Some(expected.cloned())).await
    }

    // The outer option selects a checked deletion; the inner option is the expected token.
    async fn delete_key(
        &self,
        key: JsValue,
        expected: Option<Option<StorageToken>>,
    ) -> Result<(), PersistError> {
        let store = self.clone();
        storage_task(async move {
            store.change_project(key.clone(), expected, None, vec![], vec![]).await
                .map_err(|error| PersistError(error.to_string()))?;
            if let Some(project_id) = key.as_string() {
                store.observed.borrow_mut().remove(&project_id);
                let active_key = store.active_key();
                if let Ok(window) = window()
                    && let Ok(Some(storage)) = window.local_storage()
                    && storage.get_item(&active_key).ok().flatten().as_deref() == Some(&project_id)
                {
                    let _ = storage.remove_item(&active_key);
                }
            }
            Ok(())
        }).await
    }

    pub fn active_project_id(&self, fallback: &str) -> Result<String, PersistError> {
        let storage = window()?
            .local_storage()
            .map_err(js_error)?
            .ok_or_else(|| PersistError("local storage is unavailable".into()))?;
        Ok(storage
            .get_item(&self.active_key())
            .map_err(js_error)?
            .unwrap_or_else(|| fallback.to_owned()))
    }

    pub fn observed_token(&self, id: &str) -> Option<StorageToken> {
        self.observed.borrow().get(id).cloned()
    }

    pub fn observe_token(&self, id: String, token: StorageToken) {
        self.observed.borrow_mut().insert(id, token);
    }

    fn active_key(&self) -> String {
        format!("{ACTIVE_PROJECT_KEY}:{}", self.database_name)
    }
}

impl BrowserStore {
    async fn change_project(
        &self,
        key: JsValue,
        expected: Option<Option<StorageToken>>,
        write: Option<(JsValue, Option<StorageToken>)>,
        required_assets: Vec<String>,
        new_assets: Vec<AssetBytes>,
    ) -> Result<(), SaveError> {
        for asset in &new_assets {
            verify_asset(asset)?;
        }
        let mut names = vec![PROJECT_STORE, PROJECT_TOKEN_STORE];
        if expected.is_some() && write.is_some() {
            names.push(ASSET_STORE);
        }
        let transaction = self.transaction(&names, IdbTransactionMode::Readwrite).await?;
        let failure = Rc::new(RefCell::new(None));
        let completion = transaction_completion(&transaction, failure.clone());
        let projects = transaction.object_store(PROJECT_STORE).map_err(js_error)?;
        let tokens = transaction.object_store(PROJECT_TOKEN_STORE).map_err(js_error)?;
        let new_token = write.as_ref()
            .and_then(|(_, token)| token.as_ref())
            .map(|token| JsValue::from_str(&token.0));
        let checked = expected.is_some();
        let check_request = checked.then(|| tokens.get(&key).map_err(js_error)).transpose()?;
        let project_key = key.clone();
        let token_store = tokens.clone();
        let project_store = projects.clone();
        let apply = move || -> Result<(), PersistError> {
            if let Some((document, _)) = write {
                project_store.put(&document).map_err(js_error)?;
            } else {
                project_store.delete(&project_key).map_err(js_error)?;
            }
            if let Some(token) = new_token {
                token_store.put_with_key(&token, &project_key).map_err(js_error)?;
            } else {
                token_store.delete(&project_key).map_err(js_error)?;
            }
            Ok(())
        };
        let mut handlers = Vec::new();
        let fallback = Rc::new(RefCell::new(None));
        let conflict = Rc::new(std::cell::Cell::new(false));
        let enqueue = (|| -> Result<(), PersistError> {
            if let (Some(check), Some(expected)) = (check_request, expected) {
                let stale = conflict.clone();
                let finish = move |actual| {
                    if actual != expected {
                        stale.set(true);
                        return Err(PersistError(CONFLICT.into()));
                    }
                    apply()
                };
                let legacy_handler = fallback.clone();
                let tx = transaction.clone();
                let failed = failure.clone();
                handlers.push(on_success(check, &transaction, &failure, move |value| {
                    if !value.is_undefined() {
                        return finish(decode_nonce(value)?);
                    }
                    let legacy = projects.get(&key).map_err(js_error)?;
                    *legacy_handler.borrow_mut() = Some(on_success(legacy, &tx, &failed, move |value| {
                        let actual = (!value.is_undefined())
                            .then(|| StorageToken::from_stored(value))
                            .transpose()?;
                        finish(actual)
                    }));
                    Ok(())
                }));
            } else {
                apply()?;
            }
            if names.contains(&ASSET_STORE) {
                let assets = transaction.object_store(ASSET_STORE).map_err(js_error)?;
                let mut refs: BTreeSet<_> = required_assets.into_iter().collect();
                for asset in new_assets {
                    refs.remove(&asset.sha256);
                    assets.put_with_key(
                        &Uint8Array::from(asset.bytes.as_slice()), &asset.sha256.into(),
                    ).map_err(js_error)?;
                }
                for hash in refs {
                    let request = assets.get(&JsValue::from_str(&hash)).map_err(js_error)?;
                    handlers.push(on_success(request, &transaction, &failure, move |value| {
                        if !value.is_instance_of::<Uint8Array>() {
                            return Err(PersistError(format!("required asset is missing or invalid: {hash}")));
                        }
                        Ok(())
                    }));
                }
            }
            Ok(())
        })();
        if enqueue.is_err() {
            let _ = transaction.abort();
        }
        let committed = completion.await;
        drop((handlers, fallback));
        enqueue?;
        if conflict.get() {
            return Err(SaveError::Conflict);
        }
        Ok(committed?)
    }
}

// Queue dependent requests from the success event while the transaction is active.
fn on_success(
    request: IdbRequest,
    transaction: &IdbTransaction,
    failure: &Rc<RefCell<Option<String>>>,
    handle: impl FnOnce(JsValue) -> Result<(), PersistError> + 'static,
) -> RequestHandler {
    let transaction = transaction.clone();
    let failure = failure.clone();
    let check = request.clone();
    let mut handle = Some(handle);
    let callback = Closure::<dyn FnMut(Event)>::new(move |_| {
        let Some(handle) = handle.take() else { return };
        if let Err(error) = check.result().map_err(js_error).and_then(handle) {
            *failure.borrow_mut() = Some(error.0);
            let _ = transaction.abort();
        }
    });
    request.set_onsuccess(Some(callback.as_ref().unchecked_ref()));
    callback
}

fn decode_nonce(value: JsValue) -> Result<Option<StorageToken>, PersistError> {
    if value.is_undefined() {
        return Ok(None);
    }
    value.as_string().map(StorageToken).map(Some)
        .ok_or_else(|| PersistError("stored project token is not a string".into()))
}

fn project_token(nonce: JsValue, json: &serde_json::Value) -> Result<StorageToken, PersistError> {
    match decode_nonce(nonce)? {
        Some(token) => Ok(token),
        None => StorageToken::from_value(json.clone()),
    }
}

fn serialize_document(document: &ProjectDoc) -> Result<JsValue, PersistError> {
    let value = serde_json::to_value(document).map_err(|error| PersistError(error.to_string()))?;
    validate_exact_json_integers(&value).map_err(PersistError)?;
    value.serialize(&serde_wasm_bindgen::Serializer::json_compatible()).map_err(js_decode_error)
}

fn invalid_project(error: impl fmt::Display) -> PersistError {
    PersistError(format!("stored project has invalid representation: {error}"))
}

fn decode_document(value: JsValue) -> Result<ProjectDoc, PersistError> {
    decode_project_json(serde_wasm_bindgen::from_value(value).map_err(invalid_project)?)
}

fn decode_project_json(value: serde_json::Value) -> Result<ProjectDoc, PersistError> {
    boardstudio_core_kernel::document_format::require_current_value(&value).map_err(invalid_project)?;
    serde_json::from_value(value).map_err(invalid_project)
}

async fn load_value(
    store: &BrowserStore,
    store_name: &str,
    key: JsValue,
) -> Result<Option<JsValue>, PersistError> {
    let transaction = store.transaction(&[store_name], IdbTransactionMode::Readonly).await?;
    let completion = transaction_completion(&transaction, Rc::default());
    let store = transaction.object_store(store_name).map_err(js_error)?;
    let value = request_result(store.get(&key).map_err(js_error)?).await?;
    completion.await?;
    Ok((!value.is_undefined()).then_some(value))
}

fn request_result(
    request: IdbRequest,
) -> impl std::future::Future<Output = Result<JsValue, PersistError>> {
    let (sender, receiver) = oneshot::channel();
    let sender = Rc::new(RefCell::new(Some(sender)));
    let success_sender = sender.clone();
    let success_request = request.clone();
    let on_success = Closure::<dyn FnMut(Event)>::new(move |_| {
        let result = success_request.result().map_err(js_error);
        if let Some(sender) = success_sender.borrow_mut().take() {
            let _ = sender.send(result);
        }
    });
    let error_sender = sender;
    let error_request = request.clone();
    let on_error = Closure::<dyn FnMut(Event)>::new(move |_| {
        let error = error_request
            .error()
            .ok()
            .flatten()
            .map(|error| PersistError(error.message()))
            .unwrap_or_else(|| PersistError("IndexedDB request failed".into()));
        if let Some(sender) = error_sender.borrow_mut().take() {
            let _ = sender.send(Err(error));
        }
    });
    request.set_onsuccess(Some(on_success.as_ref().unchecked_ref()));
    request.set_onerror(Some(on_error.as_ref().unchecked_ref()));
    async move {
        let result = receiver
            .await
            .map_err(|_| PersistError("IndexedDB request callback was abandoned".into()))?;
        let _ = (&on_success, &on_error);
        result
    }
}

struct TransactionCallbacks {
    transaction: IdbTransaction,
    on_complete: Closure<dyn FnMut(Event)>,
    on_abort: Closure<dyn FnMut(Event)>,
}

impl Drop for TransactionCallbacks {
    fn drop(&mut self) {
        self.transaction.set_oncomplete(None);
        self.transaction.set_onabort(None);
    }
}

fn transaction_completion(
    transaction: &IdbTransaction,
    failure_reason: Rc<RefCell<Option<String>>>,
) -> impl std::future::Future<Output = Result<(), PersistError>> {
    let (sender, receiver) = oneshot::channel();
    let sender = Rc::new(RefCell::new(Some(sender)));
    let complete_sender = sender.clone();
    let on_complete = Closure::<dyn FnMut(Event)>::new(move |_| {
        if let Some(sender) = complete_sender.borrow_mut().take() {
            let _ = sender.send(Ok(()));
        }
    });
    let aborted_transaction = transaction.clone();
    let on_abort = Closure::<dyn FnMut(Event)>::new(move |_| {
        let reason = failure_reason.borrow_mut().take().unwrap_or_else(|| {
            aborted_transaction.error().map_or_else(
                || "IndexedDB transaction aborted".into(),
                |error| {
                    format!(
                        "IndexedDB transaction aborted: {}: {}",
                        error.name(),
                        error.message()
                    )
                },
            )
        });
        if let Some(sender) = sender.borrow_mut().take() {
            let _ = sender.send(Err(PersistError(reason)));
        }
    });
    // A request error can be handled without aborting. Only complete/abort are terminal.
    // Own registration outside the async block so even an unpolled future detaches on drop.
    let callbacks = TransactionCallbacks {
        transaction: transaction.clone(),
        on_complete,
        on_abort,
    };
    transaction.set_oncomplete(Some(callbacks.on_complete.as_ref().unchecked_ref()));
    transaction.set_onabort(Some(callbacks.on_abort.as_ref().unchecked_ref()));
    async move {
        let result = receiver
            .await
            .map_err(|_| PersistError("IndexedDB transaction callback was abandoned".into()))?;
        drop(callbacks);
        result
    }
}

fn verify_asset(asset: &AssetBytes) -> Result<(), PersistError> {
    if sha256_bytes(&asset.bytes) != asset.sha256 {
        return Err(PersistError(format!(
            "asset bytes do not match SHA-256 key {}",
            asset.sha256
        )));
    }
    Ok(())
}

fn window() -> Result<Window, PersistError> {
    web_sys::window().ok_or_else(|| PersistError("window is unavailable".into()))
}

fn js_error(value: JsValue) -> PersistError {
    PersistError(crate::browser::js_error(value))
}

#[cfg(all(test, target_arch = "wasm32"))]
#[path = "storage_lifetime_tests.rs"]
mod lifetime_tests;

fn js_decode_error(error: serde_wasm_bindgen::Error) -> PersistError {
    PersistError(error.to_string())
}
