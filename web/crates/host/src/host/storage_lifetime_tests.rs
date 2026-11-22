use super::*;
use wasm_bindgen_test::*;

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
        load_value(&database_name, ASSET_STORE, "key".into())
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
        load_value(&database_name, ASSET_STORE, "key".into())
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
async fn stale_tab_save_preserves_the_other_tabs_document() {
    let name = format!("boardstudio-conflict-{}", js_sys::Date::now());
    let first = BrowserStore::new(&name).unwrap();
    let second = BrowserStore::new(&name).unwrap();
    let mut document = boardstudio_model::model::ProjectDoc::empty("conflict", "Original");
    document.id = "conflict".into();
    first
        .save_document(&document, &Default::default())
        .await
        .unwrap();
    let mut stale = second
        .load_document(document.id.clone())
        .await
        .unwrap()
        .unwrap();
    document.name = "First tab".into();
    document.revision = 1;
    first
        .save_document(&document, &Default::default())
        .await
        .unwrap();
    stale.name = "Second tab".into();
    stale.revision = 1;
    assert!(
        second
            .save_document(&stale, &Default::default())
            .await
            .is_err(),
        "a stale tab must not replace equal-revision content"
    );
    assert_eq!(
        first
            .load_document(document.id.clone())
            .await
            .unwrap()
            .unwrap()
            .name,
        "First tab"
    );
}

#[wasm_bindgen_test]
async fn concurrent_same_token_saves_leave_exactly_one_winner() {
    let name = format!("boardstudio-same-token-{}", js_sys::Date::now());
    let first = BrowserStore::new(&name).unwrap();
    let second = BrowserStore::new(&name).unwrap();
    let mut document = boardstudio_model::model::ProjectDoc::empty("race", "Original");
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
