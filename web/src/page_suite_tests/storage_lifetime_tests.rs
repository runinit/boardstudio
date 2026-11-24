use boardstudio_core::model::ProjectDoc;
use boardstudio_web_host::host::test_hooks::{
    ASSET_STORE, PROJECT_STORE, has_cached_database, load_value, open_database, request_result,
    serialize_document, storage_token_str, transaction_completion,
};
use boardstudio_web_host::host::{BrowserStore, PersistError, SaveError, StorageToken};
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;
use wasm_bindgen::{JsCast, JsValue, closure::Closure};
use wasm_bindgen_test::*;
use web_sys::{Event, IdbTransactionMode, window};

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
async fn transaction_request_error_does_not_call_dropped_abort_handler() {
    let database_name = format!("boardstudio-idb-lifetime-{}", js_sys::Date::now());
    let database = open_database(&database_name).await.unwrap();
    let failures = Rc::new(RefCell::new(Vec::<String>::new()));
    let captured = failures.clone();
    let on_error =
        Closure::<dyn FnMut(web_sys::ErrorEvent)>::new(move |event: web_sys::ErrorEvent| {
            captured.borrow_mut().push(event.message());
            event.prevent_default();
        });
    let window = window().unwrap();
    window
        .add_event_listener_with_callback("error", on_error.as_ref().unchecked_ref())
        .unwrap();
    let transaction = database
        .transaction_with_str_and_mode(ASSET_STORE, IdbTransactionMode::Readwrite)
        .unwrap();
    let completion = transaction_completion(&transaction, Rc::new(RefCell::new(None)));
    let store = transaction.object_store(ASSET_STORE).unwrap();
    let key = JsValue::from_str("duplicate-key");
    store
        .add_with_key(&JsValue::from_str("first"), &key)
        .unwrap();
    store
        .add_with_key(&JsValue::from_str("second"), &key)
        .unwrap();
    let result = completion.await;
    gloo_timers::future::TimeoutFuture::new(50).await;
    window
        .remove_event_listener_with_callback("error", on_error.as_ref().unchecked_ref())
        .unwrap();
    database.close();
    let _ = window
        .indexed_db()
        .unwrap()
        .unwrap()
        .delete_database(&database_name)
        .unwrap();
    assert!(
        result.is_err(),
        "real duplicate IndexedDB key must abort the write"
    );
    assert!(
        result.unwrap_err().0.contains("ConstraintError"),
        "preserve the native abort cause"
    );
    assert!(
        failures.borrow().is_empty(),
        "transaction error followed by abort must retain/clear handlers through terminal: {:?}",
        failures.borrow()
    );
}

#[wasm_bindgen_test]
async fn normal_completion_is_saved_and_detaches_handlers() {
    let database_name = format!("boardstudio-idb-complete-{}", js_sys::Date::now());
    let database = open_database(&database_name).await.unwrap();
    let transaction = database
        .transaction_with_str_and_mode(ASSET_STORE, IdbTransactionMode::Readwrite)
        .unwrap();
    let completion = transaction_completion(&transaction, Rc::new(RefCell::new(None)));
    transaction
        .object_store(ASSET_STORE)
        .unwrap()
        .put_with_key(&JsValue::from_str("saved"), &JsValue::from_str("key"))
        .unwrap();
    assert_eq!(completion.await, Ok(()));
    assert!(transaction.oncomplete().is_none());
    assert!(transaction.onabort().is_none());
    database.close();
    assert_eq!(
        load_value(
            &BrowserStore::new(&database_name).unwrap(),
            ASSET_STORE,
            "key".into()
        )
        .await
        .unwrap()
        .unwrap()
        .as_string()
        .as_deref(),
        Some("saved")
    );
    window()
        .unwrap()
        .indexed_db()
        .unwrap()
        .unwrap()
        .delete_database(&database_name)
        .unwrap();
}

#[wasm_bindgen_test]
async fn explicit_abort_preserves_caller_reason_and_rolls_back() {
    let database_name = format!("boardstudio-idb-abort-{}", js_sys::Date::now());
    let database = open_database(&database_name).await.unwrap();
    let transaction = database
        .transaction_with_str_and_mode(ASSET_STORE, IdbTransactionMode::Readwrite)
        .unwrap();
    let completion = transaction_completion(
        &transaction,
        Rc::new(RefCell::new(Some("required asset is missing".into()))),
    );
    transaction
        .object_store(ASSET_STORE)
        .unwrap()
        .put_with_key(&JsValue::from_str("not saved"), &JsValue::from_str("key"))
        .unwrap();
    transaction.abort().unwrap();
    assert_eq!(
        completion.await,
        Err(PersistError("required asset is missing".into()))
    );
    assert!(transaction.oncomplete().is_none());
    assert!(transaction.onabort().is_none());
    database.close();
    assert!(
        load_value(
            &BrowserStore::new(&database_name).unwrap(),
            ASSET_STORE,
            "key".into()
        )
        .await
        .unwrap()
        .is_none()
    );
    window()
        .unwrap()
        .indexed_db()
        .unwrap()
        .unwrap()
        .delete_database(&database_name)
        .unwrap();
}

#[wasm_bindgen_test]
async fn v1_records_use_legacy_tokens_and_cached_connections_allow_database_deletion() {
    let name = format!("boardstudio-v1-{}", js_sys::Date::now());
    let factory = window().unwrap().indexed_db().unwrap().unwrap();
    let request = factory.open_with_u32(&name, 1).unwrap();
    let upgrade = request.clone();
    let create = Closure::<dyn FnMut(Event)>::new(move |_| {
        let db: web_sys::IdbDatabase = upgrade.result().unwrap().unchecked_into();
        let params = web_sys::IdbObjectStoreParameters::new();
        params.set_key_path_opt_str(Some("id"));
        db.create_object_store_with_optional_parameters(PROJECT_STORE, &params)
            .unwrap();
        db.create_object_store(ASSET_STORE).unwrap();
    });
    request.set_onupgradeneeded(Some(create.as_ref().unchecked_ref()));
    let legacy: web_sys::IdbDatabase = request_result(request.unchecked_into())
        .await
        .unwrap()
        .unchecked_into();
    let mut document = ProjectDoc::empty("conflict", "Original");
    let tx = legacy
        .transaction_with_str_and_mode(PROJECT_STORE, IdbTransactionMode::Readwrite)
        .unwrap();
    let committed = transaction_completion(&tx, Rc::default());
    tx.object_store(PROJECT_STORE)
        .unwrap()
        .put(&serialize_document(&document).unwrap())
        .unwrap();
    committed.await.unwrap();
    legacy.close();

    let first = BrowserStore::new(&name).unwrap();
    let second = BrowserStore::new(&name).unwrap();
    let stale = second
        .load_document(document.id.clone())
        .await
        .unwrap()
        .unwrap();
    let legacy_token = StorageToken::for_document(&document).unwrap();
    assert_eq!(
        second.observed_token(&document.id),
        Some(legacy_token.clone())
    );
    assert_eq!(
        first
            .save_document_expected(&stale, &BTreeMap::new(), None)
            .await,
        Err(SaveError::Conflict)
    );

    document.name = "First tab".into();
    document.revision = 1;
    let nonce = first
        .save_document_expected(&document, &BTreeMap::new(), Some(&legacy_token))
        .await
        .unwrap();
    assert!(storage_token_str(&nonce).starts_with("nonce:"));
    assert_eq!(second.list_documents().await.unwrap(), [document.clone()]);
    assert_eq!(
        second
            .save_document_expected(
                &stale,
                &BTreeMap::new(),
                second.observed_token(&document.id).as_ref()
            )
            .await,
        Err(SaveError::Conflict)
    );
    assert_eq!(
        first.load_document(document.id.clone()).await.unwrap(),
        Some(document.clone())
    );
    assert!(has_cached_database(&first) && has_cached_database(&second));
    request_result(factory.delete_database(&name).unwrap().unchecked_into())
        .await
        .unwrap();
    assert!(!has_cached_database(&first) && !has_cached_database(&second));
    assert_eq!(first.load_document(document.id).await.unwrap(), None);
}

#[wasm_bindgen_test]
async fn concurrent_same_token_saves_leave_exactly_one_winner() {
    let name = format!("boardstudio-same-token-{}", js_sys::Date::now());
    let first = BrowserStore::new(&name).unwrap();
    let second = BrowserStore::new(&name).unwrap();
    let mut document = boardstudio_core::model::ProjectDoc::empty("race", "Original");
    document.id = "race".into();
    // Both writers observe the same stored token before either one writes.
    let token = first
        .save_document_expected(&document, &Default::default(), None)
        .await
        .unwrap();
    let mut first_write = document.clone();
    first_write.name = "First writer".into();
    first_write.revision = 1;
    let mut second_write = document.clone();
    second_write.name = "Second writer".into();
    second_write.revision = 1;
    // Both saves carry the same expected token. IndexedDB serializes the two
    // readwrite transactions, so exactly one can win the token check.
    let first_result = first
        .save_document_expected(&first_write, &Default::default(), Some(&token))
        .await;
    let second_result = second
        .save_document_expected(&second_write, &Default::default(), Some(&token))
        .await;
    let winner = match (&first_result, &second_result) {
        (Ok(_), Err(SaveError::Conflict)) => &first_write,
        (Err(SaveError::Conflict), Ok(_)) => &second_write,
        _ => panic!(
            "exactly one same-token save must win, got {first_result:?} and {second_result:?}"
        ),
    };
    assert_eq!(
        first
            .load_document(document.id.clone())
            .await
            .unwrap()
            .unwrap(),
        *winner,
        "the stored bytes must equal the winning writer's document"
    );
}

#[wasm_bindgen_test]
async fn unreadable_project_does_not_hide_healthy_projects() {
    let store =
        BrowserStore::new(format!("boardstudio-partial-list-{}", js_sys::Date::now())).unwrap();
    let healthy = ProjectDoc::empty("healthy", "Healthy");
    let mut future = ProjectDoc::empty("future", "Future version");
    future.format_version = 999;
    store
        .save_document(&healthy, &Default::default())
        .await
        .unwrap();
    store
        .save_document(&future, &Default::default())
        .await
        .unwrap();
    let documents = store
        .list_documents()
        .await
        .expect("one unreadable record must not fail the list");
    assert_eq!(documents.len(), 1);
    assert_eq!(documents[0].id, "healthy");
}
