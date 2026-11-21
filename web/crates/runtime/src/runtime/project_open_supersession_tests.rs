use super::*;
use crate::runtime::project_name_test_support as test_support;
use boardstudio_core::CoreEngine;
use wasm_bindgen_test::wasm_bindgen_test;

struct ConfirmChoice {
    original: JsValue,
    calls: Rc<Cell<u32>>,
    answer: Rc<Cell<bool>>,
    _callback: Closure<dyn FnMut(JsValue) -> bool>,
}

impl ConfirmChoice {
    fn new() -> Self {
        let window = web_sys::window().unwrap();
        let original = Reflect::get(&window, &"confirm".into()).unwrap();
        let calls = Rc::new(Cell::new(0));
        let answer = Rc::new(Cell::new(false));
        let count = calls.clone();
        let result = answer.clone();
        let callback = Closure::<dyn FnMut(JsValue) -> bool>::new(move |_| {
            count.set(count.get() + 1);
            result.get()
        });
        Reflect::set(&window, &"confirm".into(), callback.as_ref()).unwrap();
        Self {
            original,
            calls,
            answer,
            _callback: callback,
        }
    }
}

impl Drop for ConfirmChoice {
    fn drop(&mut self) {
        Reflect::set(
            &web_sys::window().unwrap(),
            &"confirm".into(),
            &self.original,
        )
        .unwrap();
    }
}

#[wasm_bindgen_test]
async fn unknown_outcome_recovery_still_confirms_discard_before_new() {
    let runtime = test_support::new_runtime();
    test_support::install(
        &runtime,
        Session::new(CoreEngine::validate_open),
        CoreEngine::new(),
    );
    let current = ProjectDoc::empty("unknown-current", "Current");
    runtime.submit(Event::Open {
        operation_id: runtime.operation(),
        document: current.clone(),
    });
    test_support::run_pending(&runtime).await;
    assert_eq!(*runtime.model().accepted.unwrap().document, current);
    test_support::fail_next_core_reply(&runtime, "unknown worker outcome");
    let mut edited = current.clone();
    edited.name = "Unknown edit".into();
    test_support::replace_document(&runtime, "unknown-before-new", &current.id, edited).await;
    assert_eq!(runtime.model().lifecycle, Lifecycle::RecoveryRequired);
    assert!(!runtime.has_pending_edits());
    let before = runtime.model();
    let confirm = ConfirmChoice::new();
    assert!(
        runtime.create_new_keyboard().is_err(),
        "an unknown outcome does not authorize discarding local drafts"
    );
    assert_eq!(confirm.calls.get(), 1);
    assert_eq!(runtime.model(), before);
    assert_eq!(
        runtime.store.list_projects().await.unwrap().documents,
        vec![current.clone()]
    );
    assert_eq!(
        runtime
            .store
            .load_document(current.id.clone())
            .await
            .unwrap(),
        Some(current)
    );
}

#[wasm_bindgen_test]
async fn new_project_cancel_keeps_drafts_and_publication_waits_for_persistence() {
    let runtime = test_support::new_runtime();
    test_support::install(
        &runtime,
        Session::new(CoreEngine::validate_open),
        CoreEngine::new(),
    );
    let current = ProjectDoc::empty("new-keeps-current", "Current");
    runtime.submit(Event::Open {
        operation_id: runtime.operation(),
        document: current.clone(),
    });
    test_support::run_pending(&runtime).await;
    assert_eq!(*runtime.model().accepted.unwrap().document, current);
    let mut edited = current.clone();
    edited.name = "Current with history".into();
    let current = (*test_support::replace_document(&runtime, "before-new", &current.id, edited)
        .await
        .document)
        .clone();
    assert_eq!(current.revision, 1);
    runtime.submit(Event::GestureBegin {
        operation_id: runtime.operation(),
        pointer_id: 19,
        target_ids: Vec::new(),
        transaction_id: "retained-draft".into(),
        start: Vec::new(),
        pitch: boardstudio_model::model::Vec2 { x: 19.05, y: 19.05 },
        snap_fraction: 0.0,
        geometry_snap: false,
        gap: None,
        pixels_per_world: None,
        alt: false,
    });
    test_support::run_pending(&runtime).await;
    let before = runtime.model();
    assert!(before.gesture.is_some());
    let confirm = ConfirmChoice::new();
    assert!(
        runtime.create_new_keyboard().is_err(),
        "cancel must keep the current draft"
    );
    assert_eq!(confirm.calls.get(), 1);
    assert_eq!(runtime.model(), before);
    assert_eq!(
        runtime.store.list_projects().await.unwrap().documents,
        vec![current.clone()]
    );
    confirm.answer.set(true);
    let (entered, release) = test_support::gate_next_persist(&runtime);
    let (id, outcome) = runtime.create_new_keyboard().unwrap();
    test_support::drive_pending(&runtime);
    entered.await.unwrap();
    assert_eq!(runtime.model().accepted, before.accepted);
    assert_ne!(id, current.id);
    assert!(
        runtime
            .store
            .load_document(id.clone())
            .await
            .unwrap()
            .is_none()
    );
    release.send(()).unwrap();
    test_support::run_pending(&runtime).await;
    assert_eq!(*outcome.borrow(), Some(TerminalOutcome::Completed));
    let accepted = runtime.model().accepted.unwrap();
    assert_eq!(accepted.document.id, id);
    assert_ne!(
        accepted.session_epoch,
        before.accepted.unwrap().session_epoch
    );
    assert!(runtime.model().gesture.is_none());
    assert_eq!(
        runtime
            .store
            .load_document(current.id.clone())
            .await
            .unwrap(),
        Some(current)
    );
    let undo = test_support::observe_next(&runtime);
    runtime.submit(Event::Undo {
        operation_id: runtime.operation(),
    });
    test_support::run_pending(&runtime).await;
    assert!(matches!(
        &*undo.borrow(),
        Some(TerminalOutcome::Rejected(_))
    ));
    assert_eq!(runtime.model().accepted.unwrap(), accepted);
}

#[wasm_bindgen_test]
async fn unsupported_saved_target_is_validated_before_discard_confirmation() {
    let runtime = test_support::new_runtime();
    test_support::install(
        &runtime,
        Session::new(CoreEngine::validate_open),
        CoreEngine::new(),
    );
    let current = ProjectDoc::empty("recover-pending", "Current");
    runtime.submit(Event::Open {
        operation_id: runtime.operation(),
        document: current.clone(),
    });
    test_support::run_pending(&runtime).await;
    assert_eq!(*runtime.model().accepted.unwrap().document, current);
    test_support::fail_next_persist(&runtime, "quota");
    let mut pending = current.clone();
    pending.name = "Keep my failed edit".into();
    test_support::replace_document(&runtime, "pending-recovery", &current.id, pending).await;
    assert_eq!(runtime.model().lifecycle, Lifecycle::RecoveryRequired);
    let before = runtime.model();
    let retained = runtime.pending_document.borrow().clone().unwrap();
    let mut old = ProjectDoc::empty("recover-unsupported", "Older");
    old.format_version = 6;
    runtime
        .store
        .save_document(&old, &BTreeMap::new())
        .await
        .unwrap();
    let confirm = ConfirmChoice::new();
    let done = test_support::track_open_saved(&runtime);
    runtime.open_saved(old.id);
    assert_eq!(
        confirm.calls.get(),
        0,
        "invalid targets must not ask to discard work"
    );
    done.await.unwrap();
    test_support::run_pending(&runtime).await;
    assert_eq!(confirm.calls.get(), 0);
    assert_eq!(runtime.model(), before);
    assert!(Arc::ptr_eq(
        runtime.pending_document.borrow().as_ref().unwrap(),
        &retained
    ));
    assert!(runtime.status().contains("No conversion"));
    runtime.submit(Event::RetrySave {
        operation_id: runtime.operation(),
    });
    test_support::run_pending(&runtime).await;
    assert_eq!(
        runtime.model().accepted.unwrap().document.name,
        "Keep my failed edit"
    );
}

#[wasm_bindgen_test]
async fn unsupported_saved_open_preserves_current_work_storage_and_undo() {
    let runtime = test_support::new_runtime();
    test_support::install(
        &runtime,
        Session::new(CoreEngine::validate_open),
        CoreEngine::new(),
    );
    let current = ProjectDoc::empty("recovery-current", "Before edit");
    runtime.submit(Event::Open {
        operation_id: runtime.operation(),
        document: current.clone(),
    });
    test_support::run_pending(&runtime).await;
    assert_eq!(*runtime.model().accepted.unwrap().document, current);
    let mut edited = current.clone();
    edited.name = "Accepted edit".into();
    test_support::replace_document(&runtime, "recovery-edit", &current.id, edited).await;
    let before = runtime.model().accepted.unwrap();
    let scope = runtime.scope();
    let mut older = ProjectDoc::empty("recovery-older", "Older saved keyboard");
    older.format_version = 6;
    let payload = b"preserved unsupported-project payload".to_vec();
    let hash = sha256_hex(&payload);
    older.assets.push(boardstudio_model::model::Asset {
        id: "older-asset".into(),
        name: "retained.bin".into(),
        media_type: "application/octet-stream".into(),
        sha256: hash.clone(),
        license: None,
        source: None,
    });
    runtime
        .store
        .save_document(&older, &BTreeMap::from([(hash.clone(), payload.clone())]))
        .await
        .unwrap();
    let raw_before = runtime
        .store
        .raw_project(&older.id.clone().into())
        .await
        .unwrap()
        .unwrap()
        .0;
    let listing = runtime.store.list_projects().await.unwrap();
    assert_eq!(listing.documents, vec![(*before.document).clone()]);
    assert_eq!(listing.unreadable.len(), 1);
    assert!(listing.unreadable[0].error.contains("format 6"));
    let done = test_support::track_open_saved(&runtime);
    runtime.open_saved(older.id.clone());
    done.await.unwrap();
    test_support::run_pending(&runtime).await;
    assert!(runtime.status().contains("No conversion"));
    assert_eq!(runtime.model().accepted.unwrap(), before);
    assert_eq!(runtime.scope(), scope);
    assert_eq!(
        runtime
            .store
            .load_asset(hash.clone())
            .await
            .unwrap()
            .map(|bytes| bytes.to_vec()),
        Some(payload.clone())
    );
    assert_eq!(
        runtime
            .store
            .raw_project(&older.id.clone().into())
            .await
            .unwrap()
            .unwrap()
            .0,
        raw_before
    );
    assert_eq!(
        runtime
            .store
            .load_document(current.id.clone())
            .await
            .unwrap(),
        Some((*before.document).clone())
    );
    let confirm = ConfirmChoice::new();
    let incoming_bytes = b"must not be partially imported".to_vec();
    let incoming_hash = sha256_hex(&incoming_bytes);
    for (version, revision, reason) in [
        (6, 0, "No conversion"),
        (7, u64::MAX, "revision is exhausted"),
    ] {
        let mut candidate = older.clone();
        candidate.id = format!("refused-import-{version}");
        candidate.format_version = version;
        candidate.revision = revision;
        candidate.assets[0].sha256 = incoming_hash.clone();
        let metadata = serde_json::to_string(&ArchiveRequest::PackFiles {
            entries: vec![
                ArchiveEntry {
                    path: "project.json".into(),
                    buffer_index: 0,
                },
                ArchiveEntry {
                    path: format!("assets/{incoming_hash}"),
                    buffer_index: 1,
                },
                ArchiveEntry {
                    path: "archive.json".into(),
                    buffer_index: 2,
                },
            ],
        })
        .unwrap();
        let (reply, packed) = boardstudio_core::archive::request(
            &metadata,
            &[
                serde_json::to_vec(&candidate).unwrap(),
                incoming_bytes.clone(),
                b"{\"author\":\"retained metadata\"}".to_vec(),
            ],
        );
        assert!(matches!(
            serde_json::from_str::<ArchiveReply>(&reply).unwrap(),
            ArchiveReply::Packed
        ));
        let file = web_sys::File::new_with_u8_array_sequence(
            &Array::of1(&Uint8Array::from(packed[0].as_slice()).into()),
            "refused.boardstudio",
        )
        .unwrap();
        let pending_assets = runtime.assets.borrow().clone();
        let model = runtime.model();
        let done = test_support::track_import_file(&runtime);
        runtime.import_file(file.clone());
        done.await.unwrap();
        test_support::run_pending(&runtime).await;
        assert!(runtime.status().contains(reason), "{}", runtime.status());
        assert_eq!(runtime.model(), model);
        assert_eq!(*runtime.assets.borrow(), pending_assets);
        assert_eq!(confirm.calls.get(), 0);
        assert!(
            runtime
                .store
                .load_document(candidate.id)
                .await
                .unwrap()
                .is_none()
        );
        assert!(
            runtime
                .store
                .load_asset(incoming_hash.clone())
                .await
                .unwrap()
                .is_none()
        );
        assert_eq!(
            Uint8Array::new(&JsFuture::from(file.array_buffer()).await.unwrap()).to_vec(),
            packed[0]
        );
    }
    runtime.submit(Event::Undo {
        operation_id: runtime.operation(),
    });
    test_support::run_pending(&runtime).await;
    assert_eq!(
        runtime.model().accepted.unwrap().document.name,
        "Before edit"
    );
    assert_eq!(
        runtime
            .store
            .load_asset(hash.clone())
            .await
            .unwrap()
            .map(|bytes| bytes.to_vec()),
        Some(payload)
    );
    assert_eq!(
        runtime
            .store
            .raw_project(&older.id.into())
            .await
            .unwrap()
            .unwrap()
            .0,
        raw_before
    );
}

async fn fixture() -> (Rc<Runtime>, ProjectDoc, ProjectDoc) {
    let runtime = test_support::new_runtime();
    let suffix = new_project_id().expect("browser fixture id is available");
    let (session, accepted, _) = in_process_support::opened_session();
    let current = (*accepted.document).clone();
    let mut stale_saved = current.clone();
    stale_saved.id = format!("stale-saved-{suffix}");
    stale_saved.name = "Stale saved project".into();
    let mut newer_saved = current.clone();
    newer_saved.id = format!("newer-saved-{suffix}");
    newer_saved.name = "Newer saved project".into();
    runtime
        .store
        .save_document(&stale_saved, &BTreeMap::new())
        .await
        .expect("stale saved fixture persists");
    runtime
        .store
        .save_document(&newer_saved, &BTreeMap::new())
        .await
        .expect("newer saved fixture persists");
    test_support::install(&runtime, session, CoreEngine::new());
    (runtime, stale_saved, newer_saved)
}

fn archive_file() -> web_sys::File {
    let bytes = Uint8Array::from(&b"controlled archive input"[..]);
    web_sys::File::new_with_u8_array_sequence(
        &Array::of1(&bytes.into()),
        "controlled-import.boardstudio",
    )
    .expect("archive File fixture constructs")
}

fn imported_document(base: &ProjectDoc) -> ProjectDoc {
    let mut document = base.clone();
    document.id = format!(
        "imported-{}",
        new_project_id().expect("browser fixture id is available")
    );
    document.name = "Imported newer project".into();
    document
}

async fn wait_until_current_is_durable(runtime: &Rc<Runtime>, expected: &ProjectDoc) {
    for _ in 0..100 {
        test_support::run_pending(runtime).await;
        if runtime
            .model()
            .accepted
            .as_ref()
            .is_some_and(|accepted| accepted.document.id == expected.id)
        {
            assert_eq!(
                runtime.store.active_project_id("").unwrap(),
                expected.id,
                "the newer project is durably current"
            );
            assert_eq!(
                runtime
                    .store
                    .load_document(expected.id.clone())
                    .await
                    .unwrap(),
                Some(expected.clone()),
                "the newer project remains persisted"
            );
            return;
        }
        TimeoutFuture::new(1).await;
    }
    panic!("newer project did not become accepted and durable");
}

#[wasm_bindgen_test]
async fn delayed_import_cannot_replace_a_newer_saved_project_open() {
    let confirm = ConfirmChoice::new();
    confirm.answer.set(true);
    let (runtime, _, newer) = fixture().await;
    let imported = imported_document(&newer);
    let (archive_entered, release_archive) = test_support::gate_import_archive(&runtime, &imported);
    let import_done = test_support::track_import_file(&runtime);
    runtime.import_file(archive_file());
    archive_entered
        .await
        .expect("import reached unpack reply boundary");

    let open_done = test_support::track_open_saved(&runtime);
    runtime.open_saved(newer.id.clone());
    wait_until_current_is_durable(&runtime, &newer).await;
    open_done.await.expect("saved open route completed");

    release_archive.send(()).expect("release delayed import");
    import_done.await.expect("stale import route completed");
    assert_eq!(
        runtime.model().accepted.as_ref().unwrap().document.id,
        newer.id,
        "late archive output cannot replace the newer accepted project"
    );
    assert_eq!(runtime.store.active_project_id("").unwrap(), newer.id);
    assert!(
        runtime
            .store
            .load_document(imported.id)
            .await
            .unwrap()
            .is_none()
    );
}

#[wasm_bindgen_test]
async fn delayed_saved_open_cannot_replace_a_newer_import() {
    let confirm = ConfirmChoice::new();
    confirm.answer.set(true);
    let (runtime, stale_saved, current) = fixture().await;
    let imported = imported_document(&current);
    let (open_entered, release_open) = test_support::gate_open_saved_result(&runtime);
    let open_done = test_support::track_open_saved(&runtime);
    runtime.open_saved(stale_saved.id.clone());
    open_entered
        .await
        .expect("saved document read reached pre-admission boundary");

    let (archive_entered, release_archive) = test_support::gate_import_archive(&runtime, &imported);
    let import_done = test_support::track_import_file(&runtime);
    runtime.import_file(archive_file());
    archive_entered
        .await
        .expect("import reached unpack reply boundary");
    release_archive.send(()).expect("release newer import");
    wait_until_current_is_durable(&runtime, &imported).await;
    import_done.await.expect("new import route completed");

    release_open.send(()).expect("release stale saved open");
    open_done.await.expect("stale saved open route completed");
    assert_eq!(
        runtime.model().accepted.as_ref().unwrap().document.id,
        imported.id,
        "late saved-open output cannot replace the newer accepted import"
    );
    assert_eq!(runtime.store.active_project_id("").unwrap(), imported.id);
    assert_eq!(
        runtime
            .store
            .load_document(stale_saved.id.clone())
            .await
            .unwrap(),
        Some(stale_saved),
        "the superseded saved project remains in the library"
    );
}
