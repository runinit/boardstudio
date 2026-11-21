//! Startup asset cleanup and storage-token lifetime tests.
use super::*;

struct ClientInventoryFixture {
    container: web_sys::ServiceWorkerContainer,
    previous: JsValue,
    _post: Closure<dyn FnMut(JsValue, JsValue)>,
}

impl ClientInventoryFixture {
    fn new(alone: bool) -> Self {
        let container = window().unwrap().navigator().service_worker();
        let object = js_sys::Object::from(container.clone());
        let previous = js_sys::Object::get_own_property_descriptor(&object, &"controller".into());
        let worker = js_sys::Object::new();
        let post = Closure::<dyn FnMut(JsValue, JsValue)>::new(
            move |message: JsValue, transfers: JsValue| {
                assert_eq!(
                    message.as_string().as_deref(),
                    Some(crate::offline::STORAGE_WINDOW_QUERY)
                );
                let port = Array::from(&transfers)
                    .get(0)
                    .dyn_into::<web_sys::MessagePort>()
                    .unwrap();
                port.post_message(&JsValue::from_str(if alone {
                    crate::offline::STORAGE_WINDOW_ALONE
                } else {
                    "defer"
                }))
                .unwrap();
            },
        );
        Reflect::set(&worker, &"postMessage".into(), post.as_ref()).unwrap();
        let descriptor = js_sys::Object::new();
        Reflect::set(&descriptor, &"value".into(), &worker).unwrap();
        Reflect::set(&descriptor, &"configurable".into(), &JsValue::TRUE).unwrap();
        js_sys::Object::define_property(&object, &"controller".into(), &descriptor);
        Self {
            container,
            previous,
            _post: post,
        }
    }
}

impl Drop for ClientInventoryFixture {
    fn drop(&mut self) {
        let object = js_sys::Object::from(self.container.clone());
        if self.previous.is_undefined() {
            Reflect::delete_property(&object, &"controller".into()).unwrap();
        } else {
            js_sys::Object::define_property(
                &object,
                &"controller".into(),
                &js_sys::Object::from(self.previous.clone()),
            );
        }
    }
}

#[wasm_bindgen_test]
async fn startup_reclaims_only_unused_assets_after_editors_close() {
    let _inventory = ClientInventoryFixture::new(true);
    let name = format!("boardstudio-collect-{}", js_sys::Date::now());
    let store = BrowserStore::new(&name).unwrap();
    let (lease, error) = store.start_session().await.unwrap();
    assert!(error.is_none());
    assert!(lease.is_some());
    let shared_bytes = b"shared model".to_vec();
    let unused_bytes = b"deleted model".to_vec();
    let shared = sha256_bytes(&shared_bytes);
    let unused = sha256_bytes(&unused_bytes);
    let mut document = ProjectDoc::empty("retained", "Retained");
    document.assets.push(
        serde_json::from_value(serde_json::json!({
            "id":"shared", "name":"shared.step", "mediaType":"model/step", "sha256":shared
        }))
        .unwrap(),
    );
    store
        .save_document(
            &document,
            &BTreeMap::from([(shared.clone(), shared_bytes.clone())]),
        )
        .await
        .unwrap();
    let mut deleted = ProjectDoc::empty("deleted", "Deleted");
    deleted.assets = document.assets.clone();
    deleted.assets.push(
        serde_json::from_value(serde_json::json!({
            "id":"unused", "name":"unused.step", "mediaType":"model/step", "sha256":unused
        }))
        .unwrap(),
    );
    store
        .save_document(&deleted, &BTreeMap::from([(unused.clone(), unused_bytes)]))
        .await
        .unwrap();
    store.delete_project(deleted.id).await.unwrap();
    let second = BrowserStore::new(&name).unwrap();
    let (other_lease, error) = second.start_session().await.unwrap();
    assert!(error.is_none());
    assert!(
        store.load_asset(unused.clone()).await.unwrap().is_some(),
        "an open editor may still need its Undo assets"
    );
    drop(other_lease);
    drop(lease);
    // Web Locks releases after its callback promise settles, as when a closed tab drains.
    gloo_timers::future::TimeoutFuture::new(10).await;
    let (last_lease, error) = second.start_session().await.unwrap();
    assert!(error.is_none());
    assert!(second.load_asset(unused).await.unwrap().is_none());
    assert_eq!(
        second.load_asset(shared).await.unwrap().unwrap().to_vec(),
        shared_bytes
    );
    drop(last_lease);
}

#[wasm_bindgen_test]
async fn startup_keeps_assets_when_a_legacy_window_is_present() {
    let _inventory = ClientInventoryFixture::new(false);
    let store =
        BrowserStore::new(format!("boardstudio-legacy-window-{}", js_sys::Date::now())).unwrap();
    let bytes = b"legacy tab undo".to_vec();
    let hash = sha256_bytes(&bytes);
    store
        .save_asset(AssetBytes {
            sha256: hash.clone(),
            bytes,
        })
        .await
        .unwrap();
    let (_lease, error) = store.start_session().await.unwrap();
    assert!(error.is_none());
    assert!(store.load_asset(hash).await.unwrap().is_some());
}

#[wasm_bindgen_test]
async fn unreadable_project_aborts_cleanup_and_can_be_recovered_or_deleted() {
    let _inventory = ClientInventoryFixture::new(true);
    let store = BrowserStore::new(format!(
        "boardstudio-unreadable-cleanup-{}",
        js_sys::Date::now()
    ))
    .unwrap();
    let bytes = b"unknown future reference".to_vec();
    let hash = sha256_bytes(&bytes);
    store
        .save_asset(AssetBytes {
            sha256: hash.clone(),
            bytes,
        })
        .await
        .unwrap();
    let mut future = ProjectDoc::empty("future", "Future");
    future.format_version = 999;
    store
        .save_document(&future, &Default::default())
        .await
        .unwrap();
    let (_lease, error) = store.start_session().await.unwrap();
    assert!(error.is_some());
    assert!(
        store.load_asset(hash).await.unwrap().is_some(),
        "cleanup must roll back on unknown references"
    );
    let listing = store.list_projects().await.unwrap();
    assert_eq!(listing.unreadable.len(), 1);
    let record = &listing.unreadable[0];
    let (json, _) = store.raw_project(&record.key).await.unwrap().unwrap();
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&json).unwrap()["formatVersion"],
        999
    );
    store.delete_unreadable(record).await.unwrap();
    assert!(store.list_projects().await.unwrap().unreadable.is_empty());
}

#[wasm_bindgen_test]
async fn conflict_rolls_back_assets_and_retries_keep_the_original_expectation() {
    let name = format!("boardstudio-conflict-rollback-{}", js_sys::Date::now());
    let first = BrowserStore::new(&name).unwrap();
    let second = BrowserStore::new(&name).unwrap();
    let mut document = ProjectDoc::empty("conflict", "Original");
    let original = first
        .save_document_expected(&document, &Default::default(), None)
        .await
        .unwrap();
    let stale = second
        .load_document_with_token(document.id.clone())
        .await
        .unwrap()
        .unwrap();
    document.name = "Winner".into();
    first
        .save_document_expected(&document, &Default::default(), Some(&original))
        .await
        .unwrap();
    let mut pending = stale.document.clone();
    pending.name = "Pending".into();
    let bytes = b"pending model".to_vec();
    let hash = sha256_bytes(&bytes);
    pending.assets.push(serde_json::from_value(serde_json::json!({"id":"pending", "name":"pending.step", "mediaType":"model/step", "sha256":hash})).unwrap());
    let assets = BTreeMap::from([(hash.clone(), bytes)]);
    for _ in 0..2 {
        assert_eq!(
            second
                .save_document_expected(&pending, &assets, Some(&stale.token))
                .await,
            Err(SaveError::Conflict)
        );
        assert!(second.load_asset(hash.clone()).await.unwrap().is_none());
    }
    assert!(
        second
            .delete_project_expected(document.id.clone(), Some(&stale.token))
            .await
            .is_err()
    );
    assert_eq!(
        first
            .load_document(document.id)
            .await
            .unwrap()
            .unwrap()
            .name,
        "Winner"
    );
}

struct DisabledPreferenceFixture {
    window: web_sys::Window,
    previous: JsValue,
}

impl DisabledPreferenceFixture {
    fn new() -> Self {
        let window = window().unwrap();
        let object = js_sys::Object::from(window.clone());
        let previous = js_sys::Object::get_own_property_descriptor(&object, &"localStorage".into());
        let descriptor = js_sys::Object::new();
        let getter = js_sys::Function::new_no_args(
            "throw new DOMException('Preferences disabled', 'SecurityError')",
        );
        Reflect::set(&descriptor, &"get".into(), &getter).unwrap();
        Reflect::set(&descriptor, &"configurable".into(), &JsValue::TRUE).unwrap();
        js_sys::Object::define_property(&object, &"localStorage".into(), &descriptor);
        Self { window, previous }
    }
}

impl Drop for DisabledPreferenceFixture {
    fn drop(&mut self) {
        let object = js_sys::Object::from(self.window.clone());
        if self.previous.is_undefined() {
            Reflect::delete_property(&object, &"localStorage".into()).unwrap();
        } else {
            js_sys::Object::define_property(
                &object,
                &"localStorage".into(),
                &js_sys::Object::from(self.previous.clone()),
            );
        }
    }
}

#[wasm_bindgen_test]
async fn preference_failure_after_commit_does_not_poison_save_tokens() {
    let store = BrowserStore::new(format!(
        "boardstudio-no-preferences-{}",
        js_sys::Date::now()
    ))
    .unwrap();
    let _preferences = DisabledPreferenceFixture::new();
    let mut document = ProjectDoc::empty("committed", "Original");
    let token = store
        .save_document_expected(&document, &Default::default(), None)
        .await
        .unwrap();
    document.name = "Next edit".into();
    let token = store
        .save_document_expected(&document, &Default::default(), Some(&token))
        .await
        .unwrap();
    assert_eq!(
        store
            .load_document(document.id.clone())
            .await
            .unwrap()
            .unwrap()
            .name,
        "Next edit"
    );
    store
        .delete_project_expected(document.id.clone(), Some(&token))
        .await
        .unwrap();
    assert!(store.load_document(document.id).await.unwrap().is_none());
}

#[wasm_bindgen_test]
async fn a_deleted_project_can_be_imported_again_without_a_stale_local_token() {
    let store = BrowserStore::new(format!("boardstudio-reimport-{}", js_sys::Date::now())).unwrap();
    let mut document = ProjectDoc::empty("reimport", "Original");
    store
        .save_document(&document, &Default::default())
        .await
        .unwrap();
    store.delete_project(document.id.clone()).await.unwrap();
    document.name = "Imported again".into();
    store
        .save_document(&document, &Default::default())
        .await
        .unwrap();
    assert_eq!(
        store
            .load_document(document.id.clone())
            .await
            .unwrap()
            .unwrap(),
        document
    );
}

#[wasm_bindgen_test]
async fn storage_tokens_survive_javascript_integer_float_normalization() {
    let store =
        BrowserStore::new(format!("boardstudio-number-token-{}", js_sys::Date::now())).unwrap();
    let mut document = ProjectDoc::empty("numeric", "Original");
    document.boards.push(serde_json::from_value(serde_json::json!({
        "id":"board", "name":"Board", "outlineIds":[], "partIds":[], "netIds":[], "thickness":2.0
    })).unwrap());
    document
        .outline
        .push(boardstudio_model::model::OutlineFeature::Rect {
            id: "numeric-outline".into(),
            rotation: Some(-0.0),
            anchor_part_id: None,
            center: boardstudio_model::model::Vec2 { x: -0.0, y: 1.0 },
            size: boardstudio_model::model::Vec2 { x: 20.0, y: 20.0 },
            radius: 0.0,
            operation: boardstudio_model::model::Operation::Add,
        });
    let token = store
        .save_document_expected(&document, &Default::default(), None)
        .await
        .unwrap();
    let loaded = store
        .load_document_with_token(document.id.clone())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        loaded.token, token,
        "the save token must describe the actual stored JavaScript value"
    );
    assert_eq!(StorageToken::for_document(&document).unwrap(), token);
    document.name = "Renamed".into();
    store
        .save_document_expected(&document, &Default::default(), Some(&token))
        .await
        .unwrap();
    assert_eq!(
        store
            .load_document(document.id)
            .await
            .unwrap()
            .unwrap()
            .name,
        "Renamed"
    );
}

/// Quota-recovery RED: an unreadable record whose value JSON cannot represent
/// (out-of-range BigInt) leaves no comparable token, so the guarded delete
/// refuses it. The token-independent single-key delete must remove it anyway.
#[wasm_bindgen_test]
async fn tokenless_unreadable_record_deletes_by_key_where_guarded_delete_refuses() {
    let database_name = format!("boardstudio-idb-tokenless-{}", js_sys::Date::now());
    let database = open_database(&database_name).await.unwrap();
    let raw = js_sys::Object::new();
    Reflect::set(
        &raw,
        &"id".into(),
        &JsValue::from_str("unreadable-tokenless"),
    )
    .unwrap();
    let huge = js_sys::BigInt::new(&JsValue::from_str("1267650600228229401496703205376")).unwrap();
    Reflect::set(&raw, &"marker".into(), &huge).unwrap();
    let transaction = database
        .transaction_with_str_and_mode(PROJECT_STORE, IdbTransactionMode::Readwrite)
        .map_err(js_error)
        .unwrap();
    let completion = transaction_completion(&transaction, Rc::new(RefCell::new(None)));
    let projects = transaction
        .object_store(PROJECT_STORE)
        .map_err(js_error)
        .unwrap();
    request_result(projects.put(&raw.into()).map_err(js_error).unwrap())
        .await
        .unwrap();
    completion.await.unwrap();
    database.close();

    let store = BrowserStore::new(database_name.clone()).unwrap();
    let listing = store.list_projects().await.unwrap();
    assert!(listing.documents.is_empty());
    assert_eq!(listing.unreadable.len(), 1);
    let record = &listing.unreadable[0];
    assert_eq!(record.label, "unreadable-tokenless");
    assert!(
        record.token.is_none(),
        "an out-of-range BigInt leaves no comparable token"
    );
    assert!(
        store.delete_unreadable(record).await.is_err(),
        "the guarded delete refuses tokenless records; that refusal is why the bypass exists"
    );
    store
        .delete_unreadable_by_key(record)
        .await
        .expect("token-independent single-key delete must remove the unreadable record");
    let listing = store.list_projects().await.unwrap();
    assert!(listing.documents.is_empty());
    assert!(
        listing.unreadable.is_empty(),
        "the unreadable record is gone after the by-key delete"
    );
    let _ = window()
        .unwrap()
        .indexed_db()
        .unwrap()
        .unwrap()
        .delete_database(&database_name)
        .unwrap();
}

/// The quota-recovery single-key project delete removes exactly one record,
/// needs no observed token, and is idempotent for missing keys.
#[wasm_bindgen_test]
async fn delete_project_by_key_removes_one_record_without_a_token() {
    let database_name = format!("boardstudio-idb-by-key-{}", js_sys::Date::now());
    let store = BrowserStore::new(database_name.clone()).unwrap();
    let kept = boardstudio_model::model::ProjectDoc::empty("by-key-kept", "Kept board");
    let removed = boardstudio_model::model::ProjectDoc::empty("by-key-removed", "Removed board");
    store
        .save_document(&kept, &Default::default())
        .await
        .unwrap();
    store
        .save_document(&removed, &Default::default())
        .await
        .unwrap();
    store
        .delete_project_by_key(removed.id.clone())
        .await
        .unwrap();
    assert!(
        store
            .load_document(removed.id.clone())
            .await
            .unwrap()
            .is_none(),
        "the deleted key is gone"
    );
    assert_eq!(
        store
            .load_document(kept.id.clone())
            .await
            .unwrap()
            .unwrap()
            .name,
        "Kept board"
    );
    assert!(
        store.observed_token(&removed.id).is_none(),
        "the observed token is dropped with the record"
    );
    store
        .delete_project_by_key("missing-key".into())
        .await
        .expect("deleting a missing key stays a no-op");
    let _ = window()
        .unwrap()
        .indexed_db()
        .unwrap()
        .unwrap()
        .delete_database(&database_name)
        .unwrap();
}
