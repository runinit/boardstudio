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
async fn cancelled_transaction_observer_detaches_registered_handlers() {
    let database_name = format!("boardstudio-idb-cancelled-{}", js_sys::Date::now());
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
    transaction
        .object_store(ASSET_STORE)
        .unwrap()
        .put_with_key(&JsValue::from_str("value"), &JsValue::from_str("key"))
        .unwrap();
    drop(completion);
    gloo_timers::future::TimeoutFuture::new(50).await;
    window
        .remove_event_listener_with_callback("error", on_error.as_ref().unchecked_ref())
        .unwrap();
    database.close();
    window
        .indexed_db()
        .unwrap()
        .unwrap()
        .delete_database(&database_name)
        .unwrap();
    assert!(
        failures.borrow().is_empty(),
        "dropping an unpolled observer must detach its callbacks: {:?}",
        failures.borrow()
    );
}

#[wasm_bindgen_test]
async fn handled_request_error_does_not_report_failed_transaction() {
    let failures = Rc::new(RefCell::new(Vec::<String>::new()));
    let captured = failures.clone();
    let capture_error =
        Closure::<dyn FnMut(web_sys::ErrorEvent)>::new(move |event: web_sys::ErrorEvent| {
            captured.borrow_mut().push(event.message());
            event.prevent_default();
        });
    let window = window().unwrap();
    window
        .add_event_listener_with_callback("error", capture_error.as_ref().unchecked_ref())
        .unwrap();
    let database_name = format!("boardstudio-idb-handled-error-{}", js_sys::Date::now());
    let database = open_database(&database_name).await.unwrap();
    let transaction = database
        .transaction_with_str_and_mode(ASSET_STORE, IdbTransactionMode::Readwrite)
        .unwrap();
    let completion = transaction_completion(&transaction, Rc::new(RefCell::new(None)));
    let store = transaction.object_store(ASSET_STORE).unwrap();
    let key = JsValue::from_str("duplicate-key");
    store
        .add_with_key(&JsValue::from_str("first"), &key)
        .unwrap();
    let duplicate = store
        .add_with_key(&JsValue::from_str("second"), &key)
        .unwrap();
    let on_error = Closure::<dyn FnMut(Event)>::new(move |event: Event| event.prevent_default());
    duplicate.set_onerror(Some(on_error.as_ref().unchecked_ref()));
    let result = completion.await;
    gloo_timers::future::TimeoutFuture::new(50).await;
    window
        .remove_event_listener_with_callback("error", capture_error.as_ref().unchecked_ref())
        .unwrap();
    duplicate.set_onerror(None);
    // Retain the request handler until after its event has returned.
    drop(on_error);
    database.close();
    window
        .indexed_db()
        .unwrap()
        .unwrap()
        .delete_database(&database_name)
        .unwrap();
    assert_eq!(
        result,
        Ok(()),
        "a handled request error does not abort a transaction"
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
async fn legacy_generator_project_migrates_and_is_written_back_with_assets_intact() {
    let database_name = format!("boardstudio-idb-migration-{}", js_sys::Date::now());
    let store = BrowserStore::new(&database_name).unwrap();
    let mut document = ProjectDoc::empty("legacy", "Legacy generator");
    document.format_version = 1;
    let mut definition = boardstudio_core::generators::catalogue()
        .unwrap()
        .into_iter()
        .find(|d| d.generator.as_ref().unwrap().source == "ceoloide/mounting_hole_npth")
        .unwrap();
    definition.id = "ergogen:ceoloide/mounting_hole_npth".into();
    definition
        .generator
        .as_mut()
        .unwrap()
        .parameters
        .insert("hole_size".into(), serde_json::json!("3.5"));
    document.parts.push(
        serde_json::from_value(serde_json::json!({
            "id":"hole", "definitionId":definition.id,"reference":"H1","side":"back",
            "pose":{"at":{"x":0,"y":0},"rotation":37}
        }))
        .unwrap(),
    );
    document.definitions.push(definition);
    let bytes = b"persisted model bytes".to_vec();
    let hash = sha256_bytes(&bytes);
    document.assets.push(
        serde_json::from_value(serde_json::json!({
            "id":"ergogen:model:vendor/model.step","name":"model.step","mediaType":"model/step",
            "sha256":hash,"source":"bundled Ergogen library"
        }))
        .unwrap(),
    );
    store
        .save_document(
            &document,
            &std::collections::BTreeMap::from([(hash.clone(), bytes.clone())]),
        )
        .await
        .unwrap();
    let listed = store.list_documents().await.unwrap();
    assert_eq!(listed[0].format_version, 3);
    let loaded = store.load_document("legacy".into()).await.unwrap().unwrap();
    assert_eq!(loaded.format_version, 3);
    assert_eq!(
        loaded.parts[0].definition_id,
        "generator:ceoloide/mounting_hole_npth"
    );
    assert_eq!(
        loaded.parts[0].generator_parameters.as_ref().unwrap()["side"],
        "B"
    );
    assert_eq!(
        loaded.definitions[0].generator.as_ref().unwrap().parameters["hole_size"],
        3.5
    );
    boardstudio_core::generators::render_forms(&loaded.definitions[0], Some(&loaded.parts[0]))
        .unwrap();
    let raw = load_value(&database_name, PROJECT_STORE, "legacy".into())
        .await
        .unwrap()
        .unwrap();
    let raw: serde_json::Value = serde_wasm_bindgen::from_value(raw).unwrap();
    assert_eq!(raw["formatVersion"], 3, "migration writes back immediately");
    assert!(!raw.to_string().contains("ergogen"));
    assert_eq!(
        store.load_asset(hash).await.unwrap().unwrap().to_vec(),
        bytes
    );
    // A fresh project saves and reopens through the same boundary.
    let fresh = ProjectDoc::empty("fresh", "Fresh project");
    store
        .save_document(&fresh, &Default::default())
        .await
        .unwrap();
    assert_eq!(
        store.load_document("fresh".into()).await.unwrap().unwrap(),
        fresh
    );
    window()
        .unwrap()
        .indexed_db()
        .unwrap()
        .unwrap()
        .delete_database(&database_name)
        .unwrap();
}

/// A pair-bearing project round-trips through the real browser store with its pair
/// identity, member states, geometry and identity reservations intact.
///
/// The stored shape is the same boundary the editor uses for save/reopen, so a pair's
/// linked status, per-feature exceptions, construction ownership and the opening/vertex
/// reservations all have to survive it rather than only a serde round trip.
#[wasm_bindgen_test]
async fn a_mechanical_pair_round_trips_through_indexeddb() {
    let database_name = format!("boardstudio-idb-pair-{}", js_sys::Date::now());
    let store = BrowserStore::new(&database_name).unwrap();
    let mut document = ProjectDoc::empty("pair-project", "Pair project");
    document.boards.push(
        serde_json::from_value(serde_json::json!({
            "id": "board", "name": "Board", "outlineIds": [], "partIds": [],
            "netIds": [], "thickness": 1.6, "traces": [], "vias": []
        }))
        .unwrap(),
    );
    document.mechanical = Some(
        serde_json::from_value(serde_json::json!({
            "boardId": "board", "method": "printed", "mount": "tray",
            "plateThickness": 1.5, "plateFoamThickness": 0.5, "pcbThickness": 1.6,
            "bottomFoamThickness": 2.0, "batteryHeight": 0.0, "bottomThickness": 3.0,
            "plateToPcb": 3.5, "wallThickness": 2.0, "clearance": 0.3,
            "mounts": [], "closureMounts": [], "profiles": [],
            "nextOpeningIdentity": 7,
            "openings": [{
                "id": "opening-4",
                "vertices": [
                    { "id": "vertex-3", "point": { "x": 0.0, "y": 0.0 } },
                    { "id": "vertex-4", "point": { "x": 5.0, "y": 0.0 } },
                    { "id": "vertex-5", "point": { "x": 5.0, "y": 5.0 } }
                ],
                "z": 1.0, "height": 2.0, "nextVertexIdentity": 6
            }]
        }))
        .unwrap(),
    );
    document.hardware = Some(
        serde_json::from_value(serde_json::json!({
            "topology": "split", "transport": "none",
            "instances": [
                {
                    "id": "left", "name": "Left", "boardId": "board", "half": "left",
                    "role": "central", "flipped": false,
                    "mechanical": {
                        "boardId": "board", "method": "printed", "mount": "tray",
                        "plateThickness": 1.5, "plateFoamThickness": 0.5, "pcbThickness": 1.6,
                        "bottomFoamThickness": 2.0, "batteryHeight": 0.0, "bottomThickness": 3.0,
                        "plateToPcb": 3.5, "wallThickness": 2.0, "clearance": 0.3,
                        "mounts": [{
                            "id": "mount-left", "at": { "x": 2.0, "y": 3.0 },
                            "kind": "hole", "holeDiameter": 2.2
                        }],
                        "closureMounts": [], "profiles": []
                    },
                    "constructionOwnership": "pair"
                },
                {
                    "id": "right", "name": "Right", "boardId": "board", "half": "right",
                    "role": "peripheral", "flipped": true,
                    "mechanical": {
                        "boardId": "board", "method": "printed", "mount": "tray",
                        "plateThickness": 1.5, "plateFoamThickness": 0.5, "pcbThickness": 1.6,
                        "bottomFoamThickness": 2.0, "batteryHeight": 0.0, "bottomThickness": 3.0,
                        "plateToPcb": 3.5, "wallThickness": 2.0, "clearance": 0.3,
                        "mounts": [], "closureMounts": [], "profiles": []
                    },
                    "constructionOwnership": "pair"
                },
                {
                    "id": "third", "name": "Third", "boardId": "board", "half": "third",
                    "role": "peripheral", "flipped": false,
                    "constructionOwnership": "legacy"
                }
            ],
            "boards": [],
            "sharedConstruction": {
                "boardId": "board",
                "method": "printed", "mount": "tray", "plateThickness": 1.5,
                "plateFoamThickness": 0.5, "pcbThickness": 1.6, "batteryHeight": 0.0,
                "bottomFoamThickness": 2.0,
                "bottomThickness": 3.0, "plateToPcb": 3.5, "wallThickness": 2.0, "clearance": 0.3,
                "mounts": [], "closureMounts": [], "profiles": []
            },
            "pairs": [{
                "id": "pair-left-right",
                "members": ["left", "right"],
                "mapping": { "at": { "x": 30.0, "y": -5.0 }, "rotation": 180.0 },
                "construction": {
                    "method": "printed", "mount": "tray", "integratedPlateFrame": false,
                    "bottomStyle": "shell", "middleFrame": false,
                    "plateThickness": 1.5, "plateFoamThickness": 0.5,
                    "bottomFoamThickness": 2.0, "bottomThickness": 3.0,
                    "plateToPcb": 3.5, "wallThickness": 2.0, "clearance": 0.3
                },
                "features": [{
                    "id": "mount-left",
                    "kind": "mount",
                    "author": "left",
                    "value": {
                        "kind": "mount",
                        "mount": {
                            "id": "mount-left", "at": { "x": 2.0, "y": 3.0 },
                            "kind": "hole", "holeDiameter": 2.2
                        }
                    },
                    "members": [
                        { "instanceId": "left", "featureId": "mount-left", "state": "linked" },
                        { "instanceId": "right", "featureId": "mount-right", "state": "omitted" }
                    ]
                }]
            }]
        }))
        .unwrap(),
    );

    let saved = document.clone();
    store
        .save_document(&document, &Default::default())
        .await
        .unwrap();
    let loaded = store
        .load_document("pair-project".into())
        .await
        .unwrap()
        .expect("the saved pair project loads back");

    assert_eq!(
        loaded.hardware, saved.hardware,
        "pair identity, mapping, member configurations, construction ownership, \
         linked/omitted states and geometry round-trip through the browser store"
    );
    assert_eq!(
        loaded.mechanical, saved.mechanical,
        "opening/vertex identity reservations round-trip through the browser store"
    );
    boardstudio_core::mechanical::features::validate(&loaded)
        .expect("the reopened project's pair is still consistent");

    window()
        .unwrap()
        .indexed_db()
        .unwrap()
        .unwrap()
        .delete_database(&database_name)
        .unwrap();
}
