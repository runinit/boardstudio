use crate::persistence_contract::{sha256_bytes, validate_exact_json_integers};
use boardstudio_core::model::ProjectDoc;
use futures_channel::oneshot;
use js_sys::{Array, Reflect, Uint8Array};
use serde::Serialize;
use std::{cell::RefCell, collections::BTreeSet, fmt, rc::Rc};
use wasm_bindgen::{JsCast, JsValue, prelude::*};
use wasm_bindgen_futures::spawn_local;
use web_sys::{
    Event, IdbDatabase, IdbObjectStoreParameters, IdbOpenDbRequest, IdbRequest, IdbTransaction,
    IdbTransactionMode, Window,
};

const PROJECT_STORE: &str = "projects";
const ASSET_STORE: &str = "assets";
const ACTIVE_PROJECT_KEY: &str = "boardstudio-m1-active-project";
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

#[derive(Clone, Debug)]
pub struct BrowserStore {
    database_name: String,
}

impl BrowserStore {
    pub fn new(database_name: impl Into<String>) -> Result<Self, PersistError> {
        let database_name = database_name.into();
        if database_name.is_empty() || database_name == "boardstudio-v2" {
            return Err(PersistError(
                "M1 storage requires a database separate from the React reference".into(),
            ));
        }
        Ok(Self { database_name })
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
        let json = serde_json::to_value(document)
            .map_err(|error| PersistError(format!("could not encode project document: {error}")))?;
        validate_exact_json_integers(&json).map_err(PersistError)?;
        let js_document = json
            .serialize(&serde_wasm_bindgen::Serializer::json_compatible())
            .map_err(|error| {
                PersistError(format!("project is not IndexedDB-compatible: {error}"))
            })?;
        let project_id = document.id.clone();
        let required_assets = document
            .assets
            .iter()
            .map(|asset| asset.sha256.clone())
            .collect();
        let new_assets = assets
            .iter()
            .map(|(sha256, bytes)| AssetBytes {
                sha256: sha256.clone(),
                bytes: bytes.clone(),
            })
            .collect();
        self.persist_project(project_id, js_document, required_assets, new_assets, false)
            .await
    }

    pub async fn save_document_with_assets(
        &self,
        document: &ProjectDoc,
        assets: Vec<AssetBytes>,
    ) -> Result<(), PersistError> {
        let json = serde_json::to_value(document)
            .map_err(|error| PersistError(format!("could not encode project document: {error}")))?;
        validate_exact_json_integers(&json).map_err(PersistError)?;
        let js_document = json
            .serialize(&serde_wasm_bindgen::Serializer::json_compatible())
            .map_err(|error| {
                PersistError(format!("project is not IndexedDB-compatible: {error}"))
            })?;
        let project_id = document.id.clone();
        let required_assets = document
            .assets
            .iter()
            .map(|asset| asset.sha256.clone())
            .collect();
        self.persist_project(project_id, js_document, required_assets, assets, false)
            .await
    }

    #[cfg(feature = "test-harness")]
    pub async fn abort_document_save_for_test(
        &self,
        document: &ProjectDoc,
        assets: Vec<AssetBytes>,
    ) -> Result<(), PersistError> {
        let json = serde_json::to_value(document)
            .map_err(|error| PersistError(format!("could not encode project document: {error}")))?;
        validate_exact_json_integers(&json).map_err(PersistError)?;
        let js_document = json
            .serialize(&serde_wasm_bindgen::Serializer::json_compatible())
            .map_err(|error| {
                PersistError(format!("project is not IndexedDB-compatible: {error}"))
            })?;
        let required_assets = document
            .assets
            .iter()
            .map(|asset| asset.sha256.clone())
            .collect();
        self.persist_project(
            document.id.clone(),
            js_document,
            required_assets,
            assets,
            true,
        )
        .await
    }

    async fn persist_project(
        &self,
        project_id: String,
        document: JsValue,
        required_assets: Vec<String>,
        new_assets: Vec<AssetBytes>,
        force_abort: bool,
    ) -> Result<(), PersistError> {
        let database_name = self.database_name.clone();
        let active_key = self.active_key();
        let (sender, receiver) = oneshot::channel();
        spawn_local(async move {
            let result = persist_project(
                &database_name,
                &active_key,
                project_id,
                document,
                required_assets,
                new_assets,
                force_abort,
            )
            .await;
            let _ = sender.send(result);
        });
        receiver
            .await
            .map_err(|_| PersistError("save completion task was abandoned".into()))?
    }

    pub async fn load_document(
        &self,
        project_id: String,
    ) -> Result<Option<ProjectDoc>, PersistError> {
        let database_name = self.database_name.clone();
        let (sender, receiver) = oneshot::channel();
        spawn_local(async move {
            let result = load_value(&database_name, PROJECT_STORE, project_id)
                .await
                .and_then(|value| {
                    value
                        .map(|value| {
                            serde_wasm_bindgen::from_value(value).map_err(|error| {
                                PersistError(format!(
                                    "stored project has invalid representation: {error}"
                                ))
                            })
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
        let database_name = self.database_name.clone();
        let (sender, receiver) = oneshot::channel();
        spawn_local(async move {
            let result = list_values(&database_name, PROJECT_STORE)
                .await
                .and_then(|values| {
                    values
                        .into_iter()
                        .map(|value| {
                            serde_wasm_bindgen::from_value(value).map_err(|error| {
                                PersistError(format!(
                                    "stored project has invalid representation: {error}"
                                ))
                            })
                        })
                        .collect()
                });
            let _ = sender.send(result);
        });
        receiver
            .await
            .map_err(|_| PersistError("project list task was abandoned".into()))?
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
        let database_name = self.database_name.clone();
        let active_key = self.active_key();
        let (sender, receiver) = oneshot::channel();
        spawn_local(async move {
            let result = delete_project_inner(&database_name, &active_key, &project_id).await;
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

    fn active_key(&self) -> String {
        format!("{ACTIVE_PROJECT_KEY}:{}", self.database_name)
    }
}

async fn persist_project(
    database_name: &str,
    active_key: &str,
    project_id: String,
    document: JsValue,
    required_assets: Vec<String>,
    new_assets: Vec<AssetBytes>,
    force_abort: bool,
) -> Result<(), PersistError> {
    let document_id = Reflect::get(&document, &"id".into())
        .map_err(js_error)?
        .as_string()
        .ok_or_else(|| PersistError("project document has no string id".into()))?;
    if document_id != project_id {
        return Err(PersistError(
            "project id does not match document object".into(),
        ));
    }
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
    let completion = transaction_completion(&transaction, missing_asset.clone());

    let operation = (|| -> Result<RequestHandlers, PersistError> {
        let projects = transaction.object_store(PROJECT_STORE).map_err(js_error)?;
        let assets = transaction.object_store(ASSET_STORE).map_err(js_error)?;
        projects.put(&document).map_err(js_error)?;

        for asset in &new_assets {
            let value = Uint8Array::from(asset.bytes.as_slice());
            assets
                .put_with_key(&value, &JsValue::from_str(&asset.sha256))
                .map_err(js_error)?;
        }

        let mut checks = Vec::new();
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
            return Err(error);
        }
    };
    if force_abort {
        *missing_asset.borrow_mut() = Some("test-injected IndexedDB transaction abort".into());
        let _ = transaction.abort();
    }
    let result = completion.await;
    let _ = checks;
    database.close();
    result?;
    if let Some(storage) = window()?.local_storage().map_err(js_error)? {
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
) -> Result<(), PersistError> {
    let database = open_database(database_name).await?;
    let transaction = database
        .transaction_with_str_and_mode(PROJECT_STORE, IdbTransactionMode::Readwrite)
        .map_err(js_error)?;
    let completion = transaction_completion(&transaction, Rc::new(RefCell::new(None)));
    let store = transaction.object_store(PROJECT_STORE).map_err(js_error)?;
    store
        .delete(&JsValue::from_str(project_id))
        .map_err(js_error)?;
    let result = completion.await;
    database.close();
    result?;
    if let Some(storage) = window()?.local_storage().map_err(js_error)?
        && storage.get_item(active_key).map_err(js_error)?.as_deref() == Some(project_id)
    {
        let _ = storage.remove_item(active_key);
    }
    Ok(())
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

async fn list_values(database_name: &str, store_name: &str) -> Result<Vec<JsValue>, PersistError> {
    let database = open_database(database_name).await?;
    let transaction = database
        .transaction_with_str_and_mode(store_name, IdbTransactionMode::Readonly)
        .map_err(js_error)?;
    let completion = transaction_completion(&transaction, Rc::new(RefCell::new(None)));
    let store = transaction.object_store(store_name).map_err(js_error)?;
    let request = store.get_all().map_err(js_error)?;
    let value = request_result(request).await;
    let committed = completion.await;
    database.close();
    let value = value?;
    committed?;
    Ok(Array::from(&value).iter().collect())
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
    let abort_sender = sender.clone();
    let abort_failure_reason = failure_reason.clone();
    let on_abort = Closure::<dyn FnMut(Event)>::new(move |_| {
        let reason = abort_failure_reason
            .borrow_mut()
            .take()
            .unwrap_or_else(|| "IndexedDB transaction aborted".into());
        if let Some(sender) = abort_sender.borrow_mut().take() {
            let _ = sender.send(Err(PersistError(reason)));
        }
    });
    let error_sender = sender;
    let on_error = Closure::<dyn FnMut(Event)>::new(move |_| {
        if let Some(sender) = error_sender.borrow_mut().take() {
            let _ = sender.send(Err(PersistError("IndexedDB transaction failed".into())));
        }
    });
    transaction.set_oncomplete(Some(on_complete.as_ref().unchecked_ref()));
    transaction.set_onabort(Some(on_abort.as_ref().unchecked_ref()));
    transaction.set_onerror(Some(on_error.as_ref().unchecked_ref()));
    async move {
        let result = receiver
            .await
            .map_err(|_| PersistError("IndexedDB transaction callback was abandoned".into()))?;
        let _ = (&on_complete, &on_abort, &on_error, failure_reason);
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
