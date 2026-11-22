use super::*;
use boardstudio_core_kernel::document_format::CURRENT_VERSION;
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
async fn version_3_closure_project_is_refused_without_rewriting_saved_strings() {
    let database_name = format!("boardstudio-idb-v3-closure-{}", js_sys::Date::now());
    let store = BrowserStore::new(&database_name).unwrap();
    let mut document = ProjectDoc::empty("closure", "Version 3 closure");
    document.format_version = 3;
    let mut definition = boardstudio_core_kernel::generators::catalogue()
        .unwrap()
        .into_iter()
        .find(|d| d.generator.as_ref().unwrap().source == "ceoloide/mounting_hole_npth")
        .unwrap();
    definition.id = "assembly-closure/definition/board/12.50000/-7.25000".into();
    let parameters = &mut definition.generator.as_mut().unwrap().parameters;
    parameters.insert("hole_drill".into(), serde_json::json!("6.6"));
    parameters.insert("hole_size".into(), serde_json::json!("6.6"));
    document.parts.push(
        serde_json::from_value(serde_json::json!({
            "id":"closure-hole", "definitionId":definition.id,"reference":"H1","side":"front",
            "pose":{"at":{"x":12.5,"y":-7.25},"rotation":0},
            "generatorParameters":{"hole_size":"6.6"}
        }))
        .unwrap(),
    );
    document.definitions.push(definition);
    store
        .save_document(&document, &Default::default())
        .await
        .unwrap();

    // The stored document really is the old shape before it is loaded.
    let stored =
        |value: JsValue| serde_wasm_bindgen::from_value::<serde_json::Value>(value).unwrap();
    let before = stored(
        load_value(&database_name, PROJECT_STORE, "closure".into())
            .await
            .unwrap()
            .unwrap(),
    );
    assert_eq!(before["formatVersion"], 3);
    assert_eq!(
        before["definitions"][0]["generator"]["parameters"]["hole_drill"],
        "6.6"
    );

    let error = store.load_document("closure".into()).await.unwrap_err();
    assert!(
        error.0.contains("format 3") && error.0.contains("No conversion"),
        "{error:?}"
    );
    assert_eq!(store.list_projects().await.unwrap().unreadable.len(), 1);
    let after = stored(
        load_value(&database_name, PROJECT_STORE, "closure".into())
            .await
            .unwrap()
            .unwrap(),
    );
    assert_eq!(
        after, before,
        "list/load must not convert or stamp the record"
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
async fn legacy_generator_project_is_refused_with_saved_data_and_assets_intact() {
    let database_name = format!("boardstudio-idb-migration-{}", js_sys::Date::now());
    let store = BrowserStore::new(&database_name).unwrap();
    let mut document = ProjectDoc::empty("legacy", "Legacy generator");
    document.format_version = 1;
    let mut definition = boardstudio_core_kernel::generators::catalogue()
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
    let before = serde_wasm_bindgen::from_value::<serde_json::Value>(
        load_value(&database_name, PROJECT_STORE, "legacy".into())
            .await
            .unwrap()
            .unwrap(),
    )
    .unwrap();
    let listing = store.list_projects().await.unwrap();
    assert!(listing.documents.is_empty());
    assert_eq!(listing.unreadable.len(), 1);
    let error = store.load_document("legacy".into()).await.unwrap_err();
    assert!(error.0.contains("No conversion"), "{error:?}");
    let after = serde_wasm_bindgen::from_value::<serde_json::Value>(
        load_value(&database_name, PROJECT_STORE, "legacy".into())
            .await
            .unwrap()
            .unwrap(),
    )
    .unwrap();
    assert_eq!(after, before);
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
    boardstudio_core_mechanical::mechanical::features::validate(&loaded)
        .expect("the reopened project's pair is still consistent");

    window()
        .unwrap()
        .indexed_db()
        .unwrap()
        .unwrap()
        .delete_database(&database_name)
        .unwrap();
}

/// A paired document whose linked half reserves opening identities, so the reservation
/// has something to outlive.
fn reserved_pair_document() -> ProjectDoc {
    let mut document = ProjectDoc::empty("reserved-pair", "Reserved pair");
    document.boards.push(
        serde_json::from_value(serde_json::json!({
            "id": "board", "name": "Board", "outlineIds": [], "partIds": [],
            "netIds": [], "thickness": 1.6, "traces": [], "vias": []
        }))
        .unwrap(),
    );
    let instance = |id: &str, half: &str, flipped: bool| {
        serde_json::json!({
            "id": id, "name": id, "boardId": "board", "half": half,
            "role": "peripheral", "flipped": flipped,
            "mechanical": {
                "boardId": "board", "method": "printed", "mount": "tray",
                "plateThickness": 1.5, "plateFoamThickness": 0.5, "pcbThickness": 1.6,
                "bottomFoamThickness": 2.0, "batteryHeight": 0.0, "bottomThickness": 3.0,
                "plateToPcb": 3.5, "wallThickness": 2.0, "clearance": 0.3,
                "mounts": [], "closureMounts": [], "profiles": [],
                "nextOpeningIdentity": 7,
                "openings": [{
                    "id": "opening-6",
                    "vertices": [
                        { "id": "vertex-3", "point": { "x": 0.0, "y": 0.0 } },
                        { "id": "vertex-4", "point": { "x": 5.0, "y": 0.0 } },
                        { "id": "vertex-5", "point": { "x": 5.0, "y": 5.0 } }
                    ],
                    "z": 1.0, "height": 2.0, "nextVertexIdentity": 6
                }]
            },
            "constructionOwnership": "pair"
        })
    };
    document.hardware = Some(
        serde_json::from_value(serde_json::json!({
            "topology": "split", "transport": "none",
            "instances": [instance("left", "left", false), instance("right", "right", true)],
            "boards": [],
            "sharedConstruction": {
                "boardId": "board", "method": "printed", "mount": "tray",
                "plateThickness": 1.5, "plateFoamThickness": 0.5, "pcbThickness": 1.6,
                "bottomFoamThickness": 2.0, "batteryHeight": 0.0, "bottomThickness": 3.0,
                "plateToPcb": 3.5, "wallThickness": 2.0, "clearance": 0.3,
                "mounts": [], "closureMounts": [], "profiles": []
            },
            "pairs": [{
                "id": "pair-left-right",
                "members": ["left", "right"],
                "mapping": { "at": { "x": 0.0, "y": 0.0 }, "rotation": 0.0 },
                "construction": {
                    "method": "printed", "mount": "tray", "integratedPlateFrame": false,
                    "bottomStyle": "shell", "middleFrame": false,
                    "plateThickness": 1.5, "plateFoamThickness": 0.5,
                    "bottomFoamThickness": 2.0, "bottomThickness": 3.0,
                    "plateToPcb": 3.5, "wallThickness": 2.0, "clearance": 0.3
                },
                "features": []
            }]
        }))
        .unwrap(),
    );
    document
}

/// Apply one typed mechanical edit to a document through the real Core operation.
fn applied(
    document: &ProjectDoc,
    base_revision: u64,
    target_ids: Vec<String>,
    edit: boardstudio_core_mechanical::mechanical::change::MechanicalEdit,
) -> ProjectDoc {
    use boardstudio_model::model::{CoreReply, CoreRequest, EditCommand, EditOperation, EditPhase};
    let mut engine = boardstudio_core::CoreEngine::new();
    match engine.handle(CoreRequest::Open {
        id: "storage-open".into(),
        document: document.clone(),
    }) {
        CoreReply::Scene { .. } => {}
        other => panic!("Core did not open the document: {other:?}"),
    }
    let command = EditCommand {
        base_revision,
        transaction_id: "storage".into(),
        phase: EditPhase::Commit,
        target_ids,
        operation: EditOperation::ApplyMechanicalEdit {
            edit: Box::new(edit),
        },
    };
    match engine.handle(CoreRequest::Edit {
        id: "storage-edit".into(),
        command,
    }) {
        CoreReply::Scene { document, .. } => *document,
        other => panic!("Core did not apply the edit: {other:?}"),
    }
}

/// A disabled half's identity reservation outlives its stack through the browser store.
///
/// The confirmed paired behaviour is unlink-and-disable: one Disable clears the pair and
/// removes only that half's stack, and the reservations it had made move onto the
/// instance. Enabling it again must not reuse an identity a queued intent may name, and
/// the reservation has to survive a save and reopen for that promise to hold.
#[wasm_bindgen_test]
async fn a_disabled_pairs_identity_reservation_survives_indexeddb() {
    use boardstudio_core_mechanical::mechanical::change::{
        MechanicalChange, MechanicalEdit, MechanicalTarget,
    };
    let database_name = format!("boardstudio-idb-reservation-{}", js_sys::Date::now());
    let store = BrowserStore::new(&database_name).unwrap();
    let document = reserved_pair_document();

    let edit = |change: MechanicalChange| MechanicalEdit {
        target: MechanicalTarget::Instance {
            instance_id: "left".into(),
        },
        change,
        mounting_hole: None,
    };
    let disabled = applied(
        &document,
        document.revision,
        vec!["left".into()],
        edit(MechanicalChange::Disable),
    );
    assert!(
        disabled
            .hardware
            .as_ref()
            .expect("hardware")
            .pairs
            .is_empty(),
        "the paired Disable clears the link"
    );
    let enabled = applied(
        &disabled,
        disabled.revision,
        vec!["left".into()],
        edit(MechanicalChange::Enable),
    );

    let left = enabled
        .hardware
        .as_ref()
        .expect("hardware")
        .instances
        .iter()
        .find(|instance| instance.id == "left")
        .expect("the enabled half");
    let reservation = left
        .mechanical_identity_reservation
        .expect("the reservation outlived the disabled stack");
    assert!(
        reservation.next_opening_identity >= 7,
        "the highest reserved opening identity is kept, got {reservation:?}"
    );
    assert!(
        left.mechanical.is_some(),
        "Enable restored the half's own stack"
    );
    // The reservation is what stops reuse: a later addition cannot be handed an
    // identity the disabled half already used.
    let after_add = applied(
        &enabled,
        enabled.revision,
        vec!["left".into()],
        edit(MechanicalChange::AddOpening),
    );
    let added = after_add
        .hardware
        .as_ref()
        .expect("hardware")
        .instances
        .iter()
        .find(|instance| instance.id == "left")
        .expect("the half")
        .mechanical
        .as_ref()
        .expect("the half's configuration")
        .openings
        .as_ref()
        .expect("openings")
        .last()
        .expect("the added opening");
    let added_number = added
        .id
        .trim_start_matches("opening-")
        .parse::<u64>()
        .expect("opening ids carry their number");
    assert!(
        added_number >= reservation.next_opening_identity,
        "an addition after re-enabling cannot reuse a reserved identity, got {}",
        added.id
    );
    assert_ne!(
        added.id, "opening-6",
        "the removed opening identity is not reused"
    );
    assert!(
        enabled
            .hardware
            .as_ref()
            .expect("hardware")
            .pairs
            .is_empty(),
        "enabling the half infers no relink"
    );

    store
        .save_document(&enabled, &Default::default())
        .await
        .unwrap();
    let loaded = store
        .load_document("reserved-pair".into())
        .await
        .unwrap()
        .expect("the saved project loads back");
    assert_eq!(
        loaded.hardware, enabled.hardware,
        "the reservation and the enabled stack round-trip through the browser store"
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
async fn stored_v5_mirrored_halves_are_refused_without_mutating_browser_storage() {
    let database_name = format!("boardstudio-idb-halves-{}", js_sys::Date::now());
    let store = BrowserStore::new(&database_name).unwrap();
    let mut document = ProjectDoc::empty("halves", "Mirrored halves");
    document.format_version = 5;
    document.hardware = Some(
        serde_json::from_value(serde_json::json!({
            "pairs": [{
                "id": "pair", "members": ["left", "right"],
                "mapping": { "at": { "x": 200.0, "y": 0.0 }, "rotation": 0.0 },
                "construction": { "method": "printed", "mount": "tray", "integratedPlateFrame": false,
                    "plateThickness": 1.3, "plateFoamThickness": 2.0, "bottomFoamThickness": 2.0,
                    "bottomThickness": 3.0, "plateToPcb": 2.2, "wallThickness": 2.0, "clearance": 0.3 },
                "constructionSource": "left",
                "halves": {
                    "sourceBoardId": "left-board", "mirrorBoardId": "right-board",
                    "correspondences": [{ "kind": "part", "sourceId": "l-key", "mirrorId": "r-key" }],
                    "exceptions": [{ "kind": "part", "mirrorId": "r-extra", "state": "omitted" }]
                }
            }]
        }))
        .unwrap(),
    );
    store
        .save_document(&document, &Default::default())
        .await
        .unwrap();

    let raw_before_load = serde_wasm_bindgen::from_value::<serde_json::Value>(
        load_value(&database_name, PROJECT_STORE, "halves".into())
            .await
            .unwrap()
            .unwrap(),
    )
    .unwrap();
    assert_eq!(raw_before_load["formatVersion"], 5);

    let error = store.load_document("halves".into()).await.unwrap_err();
    assert!(
        error.0.contains("format 5") && error.0.contains("No conversion"),
        "{error:?}"
    );
    let listing = store.list_projects().await.unwrap();
    assert!(listing.documents.is_empty());
    assert_eq!(listing.unreadable.len(), 1);
    let raw = serde_wasm_bindgen::from_value::<serde_json::Value>(
        load_value(&database_name, PROJECT_STORE, "halves".into())
            .await
            .unwrap()
            .unwrap(),
    )
    .unwrap();
    assert_eq!(
        raw, raw_before_load,
        "unsupported mirrored halves remain byte-for-value intact"
    );
}

#[wasm_bindgen_test]
async fn framed_constraint_endpoints_and_relations_round_trip_through_indexeddb() {
    let database_name = format!("boardstudio-idb-framed-{}", js_sys::Date::now());
    let store = BrowserStore::new(&database_name).unwrap();
    let mut document = ProjectDoc::empty("framed", "Framed constraints");
    document.constraints = serde_json::from_value(serde_json::json!([
        {"kind":"framed", "id":"c/offset",
         "source":{"kind":"part", "partId":"left/source"},
         "target":{"kind":"linked-mirror", "pairId":"pair", "sourcePartId":"left/target", "mirrorPartId":"right/target"},
         "relation":{"kind":"offset", "offset":{"x":5.0,"y":-2.0},"rotation":15.0}},
        {"kind":"framed", "id":"c/part-offset",
         "source":{"kind":"linked-mirror", "pairId":"pair", "sourcePartId":"left/source", "mirrorPartId":"right/source"},
         "target":{"kind":"part", "partId":"local/target"},
         "relation":{"kind":"part-offset", "offset":{"x":-3.0,"y":8.0},"rotation":-22.0}},
        {"kind":"framed", "id":"c/mirror-line",
         "source":{"kind":"part", "partId":"local/source"},
         "target":{"kind":"part", "partId":"local/target-2"},
         "relation":{"kind":"mirror-line", "at":{"x":2.25,"y":-5.5},"angle":37.0}}
    ]))
    .unwrap();
    assert_eq!(document.format_version, CURRENT_VERSION);
    store
        .save_document(&document, &Default::default())
        .await
        .unwrap();

    let loaded = store.load_document("framed".into()).await.unwrap().unwrap();

    assert_eq!(loaded.constraints, document.constraints);
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
async fn mixed_format_records_are_listed_without_conversion_and_raw_json_is_preserved() {
    use serde_json::json;
    let name = format!("boardstudio-format-gate-{}", js_sys::Date::now());
    let store = BrowserStore::new(&name).unwrap();
    let healthy = ProjectDoc::empty("healthy", "Healthy");
    store
        .save_document(&healthy, &Default::default())
        .await
        .unwrap();
    let mut cases = Vec::new();
    for version in 0..CURRENT_VERSION {
        cases.push((
            Some(json!(version)),
            if version == 0 {
                "invalid formatVersion"
            } else {
                "No conversion"
            },
        ));
    }
    cases.extend([
        (None, "legacy"),
        (Some(json!(CURRENT_VERSION + 1)), "compatible newer build"),
        (Some(json!("7")), "invalid formatVersion"),
        (Some(json!(null)), "invalid formatVersion"),
        (Some(json!(-1)), "invalid formatVersion"),
        (Some(json!(7.5)), "invalid formatVersion"),
    ]);
    let database = open_database(&name).await.unwrap();
    let transaction = database
        .transaction_with_str_and_mode(PROJECT_STORE, IdbTransactionMode::Readwrite)
        .unwrap();
    let completion = transaction_completion(&transaction, Rc::default());
    let object_store = transaction.object_store(PROJECT_STORE).unwrap();
    let mut originals = Vec::new();
    for (index, (version, explanation)) in cases.into_iter().enumerate() {
        let id = format!("unsupported-{index}");
        let mut value = serde_json::to_value(ProjectDoc::empty(&id, "Unsupported")).unwrap();
        match version {
            Some(version) => value["formatVersion"] = version,
            None => {
                value.as_object_mut().unwrap().remove("formatVersion");
            }
        }
        object_store
            .put(&js_sys::JSON::parse(&value.to_string()).unwrap())
            .unwrap();
        originals.push((id, value, explanation));
    }
    completion.await.unwrap();
    database.close();
    let listing = store.list_projects().await.unwrap();
    assert_eq!(listing.documents, vec![healthy]);
    assert_eq!(listing.unreadable.len(), originals.len());
    for (id, original, explanation) in originals {
        let error = store.load_document(id.clone()).await.unwrap_err();
        assert!(error.0.contains(explanation), "{id}: {error:?}");
        let (raw, _) = store
            .raw_project(&id.clone().into())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&raw).unwrap(),
            original
        );
        let stored = load_value(&name, PROJECT_STORE, id).await.unwrap().unwrap();
        assert_eq!(
            serde_wasm_bindgen::from_value::<serde_json::Value>(stored).unwrap(),
            original
        );
    }
}

#[wasm_bindgen_test]
async fn foreign_format_identifier_is_unsupported_and_preserved_on_list_and_load() {
    let name = format!("boardstudio-foreign-format-{}", js_sys::Date::now());
    let store = BrowserStore::new(&name).unwrap();
    let healthy = ProjectDoc::empty("healthy", "Healthy");
    store
        .save_document(&healthy, &Default::default())
        .await
        .unwrap();
    let mut value = serde_json::to_value(ProjectDoc::empty("foreign", "Foreign")).unwrap();
    value["format"] = serde_json::json!("boardstudio/v1");
    let database = open_database(&name).await.unwrap();
    let transaction = database
        .transaction_with_str_and_mode(PROJECT_STORE, IdbTransactionMode::Readwrite)
        .unwrap();
    let completion = transaction_completion(&transaction, Rc::default());
    transaction
        .object_store(PROJECT_STORE)
        .unwrap()
        .put(&js_sys::JSON::parse(&value.to_string()).unwrap())
        .unwrap();
    completion.await.unwrap();
    database.close();
    let listing = store.list_projects().await.unwrap();
    assert_eq!(listing.documents, vec![healthy]);
    assert_eq!(listing.unreadable.len(), 1);
    assert!(
        listing.unreadable[0]
            .error
            .contains("Unsupported document format")
    );
    let error = store.load_document("foreign".into()).await.unwrap_err();
    assert!(error.0.contains("Unsupported document format"), "{error:?}");
    let stored = load_value(&name, PROJECT_STORE, "foreign".into())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        serde_wasm_bindgen::from_value::<serde_json::Value>(stored).unwrap(),
        value
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

#[path = "storage_lifetime_tests/cleanup_tests.rs"]
mod cleanup_tests;
