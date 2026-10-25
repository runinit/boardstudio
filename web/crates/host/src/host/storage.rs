mod startup;
use crate::persistence_contract::{sha256_bytes, validate_exact_json_integers};
use boardstudio_core::model::ProjectDoc;
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
    Event, IdbDatabase, IdbObjectStoreParameters, IdbOpenDbRequest, IdbRequest, IdbTransaction,
    IdbTransactionMode, Window,
};

const PROJECT_STORE: &str = "projects";
const ASSET_STORE: &str = "assets";
const ACTIVE_PROJECT_KEY: &str = "boardstudio-m1-active-project";
const CONFLICT: &str =
    "This keyboard was changed or deleted in another tab. Your pending edits have been preserved.";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StorageToken(String);

impl StorageToken {
    pub fn for_document(document: &ProjectDoc) -> Result<Self, PersistError> {
        let value =
            serde_json::to_value(document).map_err(|error| PersistError(error.to_string()))?;
        validate_exact_json_integers(&value).map_err(PersistError)?;
        let stored = value
            .serialize(&serde_wasm_bindgen::Serializer::json_compatible())
            .map_err(js_decode_error)?;
        Self::from_stored(stored)
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

type RequestHandlers = Vec<Closure<dyn FnMut(Event)>>;

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
        self.save_assets_expected(
            document,
            assets
                .iter()
                .map(|(sha256, bytes)| AssetBytes {
                    sha256: sha256.clone(),
                    bytes: bytes.clone(),
                })
                .collect(),
            expected,
        )
        .await
    }

    async fn save_assets_expected(
        &self,
        document: &ProjectDoc,
        assets: Vec<AssetBytes>,
        expected: Option<&StorageToken>,
    ) -> Result<StorageToken, SaveError> {
        let json = serde_json::to_value(document)
            .map_err(|error| SaveError::Storage(error.to_string()))?;
        validate_exact_json_integers(&json).map_err(SaveError::Storage)?;
        let js_document = json
            .serialize(&serde_wasm_bindgen::Serializer::json_compatible())
            .map_err(|error| SaveError::Storage(error.to_string()))?;
        let token = StorageToken::from_stored(js_document.clone())?;
        self.persist_project(
            js_document,
            document
                .assets
                .iter()
                .map(|asset| asset.sha256.clone())
                .collect(),
            assets,
            expected.cloned(),
        )
        .await?;
        self.observed
            .borrow_mut()
            .insert(document.id.clone(), token.clone());
        Ok(token)
    }

    async fn persist_project(
        &self,
        document: JsValue,
        required_assets: Vec<String>,
        new_assets: Vec<AssetBytes>,
        expected: Option<StorageToken>,
    ) -> Result<(), SaveError> {
        let database_name = self.database_name.clone();
        let active_key = self.active_key();
        let (sender, receiver) = oneshot::channel();
        spawn_local(async move {
            let result = persist_project(
                &database_name,
                &active_key,
                document,
                required_assets,
                new_assets,
                expected,
            )
            .await;
            let _ = sender.send(result);
        });
        receiver
            .await
            .map_err(|_| SaveError::Storage("save completion task was abandoned".into()))?
    }

    /// Decode migrations in memory; persistence is always guarded by the original token.
    pub async fn load_document(
        &self,
        project_id: String,
    ) -> Result<Option<ProjectDoc>, PersistError> {
        let saved = self.load_document_with_token(project_id).await?;
        if let Some(saved) = &saved {
            self.observed
                .borrow_mut()
                .insert(saved.document.id.clone(), saved.token.clone());
        }
        Ok(saved.map(|saved| saved.document))
    }

    pub async fn load_document_with_token(
        &self,
        project_id: String,
    ) -> Result<Option<SavedProject>, PersistError> {
        let database_name = self.database_name.clone();
        let (sender, receiver) = oneshot::channel();
        spawn_local(async move {
            let result = load_value(&database_name, PROJECT_STORE, project_id)
                .await
                .and_then(|value| {
                    value
                        .map(|value| {
                            let json = serde_wasm_bindgen::from_value(value.clone())
                                .map_err(js_decode_error)?;
                            let token = StorageToken::from_value(json)?;
                            let (document, _) = decode_document(value)?;
                            Ok(SavedProject { document, token })
                        })
                        .transpose()
                });
            let _ = sender.send(result);
        });
        receiver
            .await
            .map_err(|_| PersistError("project load task was abandoned".into()))?
    }

    pub async fn list_documents(&self) -> Result<Vec<ProjectDoc>, PersistError> {
        Ok(self.list_projects().await?.documents)
    }

    pub async fn list_projects(&self) -> Result<ProjectListing, PersistError> {
        let database_name = self.database_name.clone();
        let observed = self.observed.clone();
        let (sender, receiver) = oneshot::channel();
        spawn_local(async move {
            let result = list_project_values(&database_name).await.map(|records| {
                let mut listing = ProjectListing::default();
                for (key, value) in records {
                    match decode_document(value.clone()) {
                        Ok((document, _)) => {
                            if let Ok(json) = serde_wasm_bindgen::from_value(value)
                                && let Ok(token) = StorageToken::from_value(json)
                            {
                                observed.borrow_mut().insert(document.id.clone(), token);
                            }
                            listing.documents.push(document);
                        }
                        Err(error) => {
                            let token = serde_wasm_bindgen::from_value(value)
                                .ok()
                                .and_then(|value| StorageToken::from_value(value).ok());
                            let label = key.as_string().unwrap_or_else(|| format!("{key:?}"));
                            listing.unreadable.push(UnreadableProject {
                                key,
                                label,
                                error: error.0,
                                token,
                            });
                        }
                    }
                }
                listing
            });
            let _ = sender.send(result);
        });
        receiver
            .await
            .map_err(|_| PersistError("project list task was abandoned".into()))?
    }

    /// Export the raw representation without trying to migrate it.
    pub async fn raw_project(
        &self,
        key: &JsValue,
    ) -> Result<Option<(String, StorageToken)>, PersistError> {
        let store = self.clone();
        let key = key.clone();
        let (sender, receiver) = oneshot::channel();
        spawn_local(async move {
            let _ = sender.send(store.raw_project_inner(&key).await);
        });
        receiver
            .await
            .map_err(|_| PersistError("raw project load task was abandoned".into()))?
    }

    async fn raw_project_inner(
        &self,
        key: &JsValue,
    ) -> Result<Option<(String, StorageToken)>, PersistError> {
        let database = open_database(&self.database_name).await?;
        let transaction = database
            .transaction_with_str_and_mode(PROJECT_STORE, IdbTransactionMode::Readonly)
            .map_err(js_error)?;
        let completion = transaction_completion(&transaction, Rc::default());
        let request = transaction
            .object_store(PROJECT_STORE)
            .map_err(js_error)?
            .get(key)
            .map_err(js_error)?;
        let result = request_result(request).await;
        let committed = completion.await;
        database.close();
        committed?;
        let value = result?;
        if value.is_undefined() {
            return Ok(None);
        }
        let json: serde_json::Value =
            serde_wasm_bindgen::from_value(value).map_err(js_decode_error)?;
        let token = StorageToken::from_value(json.clone())?;
        Ok(Some((
            serde_json::to_string_pretty(&json).map_err(|error| PersistError(error.to_string()))?,
            token,
        )))
    }

    /// Permanently remove one unreadable record by key without comparing a
    /// token. Only call after the explicit "permanently delete what we cannot
    /// read" confirm: unlike [`Self::delete_unreadable`], this does not refuse
    /// records whose value JSON cannot represent (those leave no token).
    /// Skips every read and follow-up write; a small delete must succeed where
    /// a full origin makes saves abort.
    pub async fn delete_unreadable_by_key(
        &self,
        record: &UnreadableProject,
    ) -> Result<(), PersistError> {
        let database_name = self.database_name.clone();
        let key = record.key.clone();
        let (sender, receiver) = oneshot::channel();
        let observed = self.observed.clone();
        spawn_local(async move {
            let result = delete_key_inner(&database_name, &key).await;
            if result.is_ok()
                && let Some(id) = key.as_string()
            {
                observed.borrow_mut().remove(&id);
            }
            let _ = sender.send(result);
        });
        receiver
            .await
            .map_err(|_| PersistError("unreadable project delete task was abandoned".into()))?
    }

    pub async fn delete_unreadable(&self, record: &UnreadableProject) -> Result<(), PersistError> {
        let expected = record.token.as_ref().ok_or_else(|| {
            PersistError("This record cannot be safely compared; it was kept.".into())
        })?;
        let database_name = self.database_name.clone();
        let key = record.key.clone();
        let expected = expected.clone();
        let (sender, receiver) = oneshot::channel();
        let observed = self.observed.clone();
        spawn_local(async move {
            let result = delete_record(&database_name, &key, Some(expected)).await;
            if result.is_ok()
                && let Some(id) = key.as_string()
            {
                observed.borrow_mut().remove(&id);
            }
            let _ = sender.send(result);
        });
        receiver
            .await
            .map_err(|_| PersistError("unreadable project delete task was abandoned".into()))?
    }

    pub async fn save_asset(&self, asset: AssetBytes) -> Result<(), PersistError> {
        verify_asset(&asset)?;
        let database_name = self.database_name.clone();
        let (sender, receiver) = oneshot::channel();
        spawn_local(async move {
            let result = save_asset_inner(&database_name, asset).await;
            let _ = sender.send(result);
        });
        receiver
            .await
            .map_err(|_| PersistError("asset save task was abandoned".into()))?
    }

    pub async fn load_asset(&self, sha256: String) -> Result<Option<Uint8Array>, PersistError> {
        let database_name = self.database_name.clone();
        let (sender, receiver) = oneshot::channel();
        spawn_local(async move {
            let result = load_value(&database_name, ASSET_STORE, sha256.clone())
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
                });
            let _ = sender.send(result);
        });
        receiver
            .await
            .map_err(|_| PersistError("asset load task was abandoned".into()))?
    }

    pub async fn delete_project(&self, project_id: String) -> Result<(), PersistError> {
        let expected = self.observed.borrow().get(&project_id).cloned();
        self.delete_project_expected(project_id, expected.as_ref())
            .await
    }

    /// Permanently remove one saved project by key without comparing a token.
    /// Only call after an explicit user discard/delete confirm: unlike the
    /// guarded deletes, this does not refuse when another tab changed the
    /// record. Skips every read and follow-up write; a small delete must
    /// succeed where a full origin makes saves abort. Deleting a missing key
    /// is a no-op.
    pub async fn delete_project_by_key(&self, project_id: String) -> Result<(), PersistError> {
        let database_name = self.database_name.clone();
        let active_key = self.active_key();
        let key = JsValue::from_str(&project_id);
        let (sender, receiver) = oneshot::channel();
        let observed = self.observed.clone();
        spawn_local(async move {
            let result = delete_key_inner(&database_name, &key).await;
            if result.is_ok() {
                observed.borrow_mut().remove(&project_id);
            }
            if let Ok(window) = window()
                && let Ok(Some(storage)) = window.local_storage()
                && storage
                    .get_item(&active_key)
                    .ok()
                    .flatten()
                    .as_deref()
                    == Some(project_id.as_str())
            {
                let _ = storage.remove_item(&active_key);
            }
            let _ = sender.send(result);
        });
        receiver
            .await
            .map_err(|_| PersistError("project delete task was abandoned".into()))?
    }

    /// Store one raw record without validation, so tests can fabricate entries
    /// the typed writers never produce (for example values JSON cannot
    /// represent, which decode to unreadable records with no token).
    #[cfg(any(test, feature = "test-support"))]
    pub async fn put_raw_record_for_test(&self, value: JsValue) -> Result<(), PersistError> {
        let database_name = self.database_name.clone();
        let (sender, receiver) = oneshot::channel();
        spawn_local(async move {
            let result = put_raw_record_inner(&database_name, value).await;
            let _ = sender.send(result);
        });
        receiver
            .await
            .map_err(|_| PersistError("raw record put task was abandoned".into()))?
    }

    pub async fn delete_project_expected(
        &self,
        project_id: String,
        expected: Option<&StorageToken>,
    ) -> Result<(), PersistError> {
        let database_name = self.database_name.clone();
        let active_key = self.active_key();
        let expected = expected.cloned();
        let (sender, receiver) = oneshot::channel();
        let observed = self.observed.clone();
        spawn_local(async move {
            let result =
                delete_project_inner(&database_name, &active_key, &project_id, expected).await;
            if result.is_ok() {
                observed.borrow_mut().remove(&project_id);
            }
            let _ = sender.send(result);
        });
        receiver
            .await
            .map_err(|_| PersistError("project delete task was abandoned".into()))?
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

    fn active_key(&self) -> String {
        format!("{ACTIVE_PROJECT_KEY}:{}", self.database_name)
    }
}

async fn persist_project(
    database_name: &str,
    active_key: &str,
    document: JsValue,
    required_assets: Vec<String>,
    new_assets: Vec<AssetBytes>,
    expected: Option<StorageToken>,
) -> Result<(), SaveError> {
    let project_id = Reflect::get(&document, &"id".into())
        .map_err(js_error)?
        .as_string()
        .ok_or_else(|| PersistError("project document has no string id".into()))?;
    for asset in &new_assets {
        verify_asset(asset)?;
    }
    let refs: BTreeSet<String> = required_assets.into_iter().collect();
    let incoming: BTreeSet<String> = new_assets
        .iter()
        .map(|asset| asset.sha256.clone())
        .collect();
    let database = open_database(database_name).await?;
    let names = Array::new();
    names.push(&JsValue::from_str(PROJECT_STORE));
    names.push(&JsValue::from_str(ASSET_STORE));
    let transaction = database
        .transaction_with_str_sequence_and_mode(&names, IdbTransactionMode::Readwrite)
        .map_err(js_error)?;
    let missing_asset = Rc::new(RefCell::new(None::<String>));
    let conflict = Rc::new(std::cell::Cell::new(false));
    let completion = transaction_completion(&transaction, missing_asset.clone());

    let operation = (|| -> Result<RequestHandlers, PersistError> {
        let projects = transaction.object_store(PROJECT_STORE).map_err(js_error)?;
        let assets = transaction.object_store(ASSET_STORE).map_err(js_error)?;
        let request = projects
            .get(&JsValue::from_str(&project_id))
            .map_err(js_error)?;
        let check = request.clone();
        let failed = missing_asset.clone();
        let tx = transaction.clone();
        let stale = conflict.clone();
        let on_success = Closure::<dyn FnMut(Event)>::new(move |_| {
            let result = (|| {
                let stored = check.result().map_err(js_error)?;
                let token = if stored.is_undefined() {
                    None
                } else {
                    let json = serde_wasm_bindgen::from_value(stored).map_err(js_decode_error)?;
                    Some(StorageToken::from_value(json)?)
                };
                if token != expected {
                    stale.set(true);
                    return Err(PersistError(CONFLICT.into()));
                }
                projects.put(&document).map_err(js_error)?;
                Ok(())
            })();
            if let Err(error) = result {
                *failed.borrow_mut() = Some(error.0);
                let _ = tx.abort();
            }
        });
        request.set_onsuccess(Some(on_success.as_ref().unchecked_ref()));

        for asset in &new_assets {
            let value = Uint8Array::from(asset.bytes.as_slice());
            assets
                .put_with_key(&value, &JsValue::from_str(&asset.sha256))
                .map_err(js_error)?;
        }

        let mut checks = vec![on_success];
        for hash in refs.difference(&incoming) {
            let request = assets.get(&JsValue::from_str(hash)).map_err(js_error)?;
            let failed = missing_asset.clone();
            let transaction = transaction.clone();
            let hash = hash.clone();
            let check_request = request.clone();
            let on_success = Closure::<dyn FnMut(Event)>::new(move |_| {
                let absent = check_request
                    .result()
                    .map(|value| {
                        value.is_undefined()
                            || value.is_null()
                            || !value.is_instance_of::<Uint8Array>()
                    })
                    .unwrap_or(true);
                if absent {
                    *failed.borrow_mut() =
                        Some(format!("required asset is missing or invalid: {hash}"));
                    let _ = transaction.abort();
                }
            });
            request.set_onsuccess(Some(on_success.as_ref().unchecked_ref()));
            checks.push(on_success);
        }
        Ok(checks)
    })();
    let checks = match operation {
        Ok(checks) => checks,
        Err(error) => {
            let _ = transaction.abort();
            database.close();
            return Err(error.into());
        }
    };
    let result = completion.await;
    let _ = checks;
    database.close();
    if conflict.get() {
        return Err(SaveError::Conflict);
    }
    result?;
    if let Ok(window) = window()
        && let Ok(Some(storage)) = window.local_storage()
    {
        let _ = storage.set_item(active_key, &project_id);
    }
    Ok(())
}

async fn save_asset_inner(database_name: &str, asset: AssetBytes) -> Result<(), PersistError> {
    let database = open_database(database_name).await?;
    let transaction = database
        .transaction_with_str_and_mode(ASSET_STORE, IdbTransactionMode::Readwrite)
        .map_err(js_error)?;
    let completion = transaction_completion(&transaction, Rc::new(RefCell::new(None)));
    let store = transaction.object_store(ASSET_STORE).map_err(js_error)?;
    let bytes = Uint8Array::from(asset.bytes.as_slice());
    store
        .put_with_key(&bytes, &JsValue::from_str(&asset.sha256))
        .map_err(js_error)?;
    let result = completion.await;
    database.close();
    result
}

async fn delete_project_inner(
    database_name: &str,
    active_key: &str,
    project_id: &str,
    expected: Option<StorageToken>,
) -> Result<(), PersistError> {
    delete_record(database_name, &JsValue::from_str(project_id), expected).await?;
    if let Ok(window) = window()
        && let Ok(Some(storage)) = window.local_storage()
        && storage.get_item(active_key).ok().flatten().as_deref() == Some(project_id)
    {
        let _ = storage.remove_item(active_key);
    }
    Ok(())
}

async fn delete_record(
    database_name: &str,
    key: &JsValue,
    expected: Option<StorageToken>,
) -> Result<(), PersistError> {
    let database = open_database(database_name).await?;
    let transaction = database
        .transaction_with_str_and_mode(PROJECT_STORE, IdbTransactionMode::Readwrite)
        .map_err(js_error)?;
    let failure = Rc::new(RefCell::new(None));
    let completion = transaction_completion(&transaction, failure.clone());
    let store = transaction.object_store(PROJECT_STORE).map_err(js_error)?;
    let key = key.clone();
    let request = store.get(&key).map_err(js_error)?;
    let check = request.clone();
    let tx = transaction.clone();
    let on_success = Closure::<dyn FnMut(Event)>::new(move |_| {
        let result = (|| {
            let value = check.result().map_err(js_error)?;
            let token = if value.is_undefined() {
                None
            } else {
                Some(StorageToken::from_value(
                    serde_wasm_bindgen::from_value(value).map_err(js_decode_error)?,
                )?)
            };
            if token != expected {
                return Err(PersistError(CONFLICT.into()));
            }
            store.delete(&key).map_err(js_error)?;
            Ok(())
        })();
        if let Err(error) = result {
            *failure.borrow_mut() = Some(error.0);
            let _ = tx.abort();
        }
    });
    request.set_onsuccess(Some(on_success.as_ref().unchecked_ref()));
    let result = completion.await;
    database.close();
    result?;
    drop(on_success);
    Ok(())
}

/// Remove one project-store record without reading or comparing it. Deleting
/// a missing key succeeds as a no-op.
async fn delete_key_inner(database_name: &str, key: &JsValue) -> Result<(), PersistError> {
    let database = open_database(database_name).await?;
    let transaction = database
        .transaction_with_str_and_mode(PROJECT_STORE, IdbTransactionMode::Readwrite)
        .map_err(js_error)?;
    let completion = transaction_completion(&transaction, Rc::new(RefCell::new(None)));
    let store = transaction.object_store(PROJECT_STORE).map_err(js_error)?;
    store.delete(key).map_err(js_error)?;
    let result = completion.await;
    database.close();
    result
}

#[cfg(any(test, feature = "test-support"))]
async fn put_raw_record_inner(
    database_name: &str,
    value: JsValue,
) -> Result<(), PersistError> {
    let database = open_database(database_name).await?;
    let transaction = database
        .transaction_with_str_and_mode(PROJECT_STORE, IdbTransactionMode::Readwrite)
        .map_err(js_error)?;
    let completion = transaction_completion(&transaction, Rc::new(RefCell::new(None)));
    let store = transaction.object_store(PROJECT_STORE).map_err(js_error)?;
    let request = store.put(&value).map_err(js_error)?;
    let result = request_result(request).await;
    let committed = completion.await;
    database.close();
    result?;
    committed
}

/// Decode a stored project, migrating older versions first. Returns whether it migrated.
fn decode_document(value: JsValue) -> Result<(ProjectDoc, bool), PersistError> {
    let invalid = |error: String| {
        PersistError(format!(
            "stored project has invalid representation: {error}"
        ))
    };
    let value: serde_json::Value =
        serde_wasm_bindgen::from_value(value).map_err(|error| invalid(error.to_string()))?;
    let (value, migrated) = boardstudio_core::migrate::migrate_value(value).map_err(invalid)?;
    let document = serde_json::from_value(value).map_err(|error| invalid(error.to_string()))?;
    Ok((document, migrated))
}

async fn load_value(
    database_name: &str,
    store_name: &str,
    key: String,
) -> Result<Option<JsValue>, PersistError> {
    let database = open_database(database_name).await?;
    let transaction = database
        .transaction_with_str_and_mode(store_name, IdbTransactionMode::Readonly)
        .map_err(js_error)?;
    let completion = transaction_completion(&transaction, Rc::new(RefCell::new(None)));
    let store = transaction.object_store(store_name).map_err(js_error)?;
    let request = store.get(&JsValue::from_str(&key)).map_err(js_error)?;
    let value = request_result(request).await;
    let committed = completion.await;
    database.close();
    let value = value?;
    committed?;
    Ok((!value.is_undefined()).then_some(value))
}

async fn list_project_values(database_name: &str) -> Result<Vec<(JsValue, JsValue)>, PersistError> {
    let database = open_database(database_name).await?;
    let transaction = database
        .transaction_with_str_and_mode(PROJECT_STORE, IdbTransactionMode::Readonly)
        .map_err(js_error)?;
    let completion = transaction_completion(&transaction, Rc::default());
    let store = transaction.object_store(PROJECT_STORE).map_err(js_error)?;
    let keys = request_result(store.get_all_keys().map_err(js_error)?);
    let values = request_result(store.get_all().map_err(js_error)?);
    let keys = keys.await;
    let values = values.await;
    let committed = completion.await;
    database.close();
    committed?;
    Ok(Array::from(&keys?)
        .iter()
        .zip(Array::from(&values?).iter())
        .collect())
}

async fn open_database(name: &str) -> Result<IdbDatabase, PersistError> {
    let factory = window()?
        .indexed_db()
        .map_err(js_error)?
        .ok_or_else(|| PersistError("IndexedDB is unavailable".into()))?;
    let request = factory.open_with_u32(name, 1).map_err(js_error)?;
    let upgrade_request = request.clone();
    let on_upgrade = Closure::<dyn FnMut(Event)>::new(move |_| {
        let Ok(database) = upgrade_request
            .result()
            .and_then(|value| value.dyn_into::<IdbDatabase>())
        else {
            return;
        };
        let params = IdbObjectStoreParameters::new();
        params.set_key_path_opt_str(Some("id"));
        if database
            .create_object_store_with_optional_parameters(PROJECT_STORE, &params)
            .is_err()
        {
            wasm_bindgen::throw_str("could not create project store");
        }
        if database.create_object_store(ASSET_STORE).is_err() {
            wasm_bindgen::throw_str("could not create asset store");
        }
    });
    request.set_onupgradeneeded(Some(on_upgrade.as_ref().unchecked_ref()));
    let database = open_request_result(request).await?;
    let _ = on_upgrade;
    Ok(database)
}

async fn open_request_result(request: IdbOpenDbRequest) -> Result<IdbDatabase, PersistError> {
    let (sender, receiver) = oneshot::channel();
    let sender = Rc::new(RefCell::new(Some(sender)));
    let success_sender = sender.clone();
    let success_request = request.clone();
    let on_success = Closure::<dyn FnMut(Event)>::new(move |_| {
        let result = success_request
            .result()
            .and_then(|value| value.dyn_into::<IdbDatabase>())
            .map_err(js_error);
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
            .unwrap_or_else(|| PersistError("IndexedDB open failed".into()));
        if let Some(sender) = error_sender.borrow_mut().take() {
            let _ = sender.send(Err(error));
        }
    });
    request.set_onsuccess(Some(on_success.as_ref().unchecked_ref()));
    request.set_onerror(Some(on_error.as_ref().unchecked_ref()));
    let result = receiver
        .await
        .map_err(|_| PersistError("IndexedDB open callback was abandoned".into()))?;
    let _ = (&on_success, &on_error);
    result
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
    let message = value
        .as_string()
        .or_else(|| {
            Reflect::get(&value, &"message".into())
                .ok()
                .and_then(|message| message.as_string())
        })
        .unwrap_or_else(|| format!("{value:?}"));
    PersistError(message)
}

#[cfg(all(test, target_arch = "wasm32"))]
#[path = "storage_lifetime_tests.rs"]
mod lifetime_tests;

fn js_decode_error(error: serde_wasm_bindgen::Error) -> PersistError {
    PersistError(error.to_string())
}
