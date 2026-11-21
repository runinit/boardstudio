use super::*;
use boardstudio_model::model::{EditCommand, EditOperation, EditPhase};
use wasm_bindgen_test::wasm_bindgen_test;

#[wasm_bindgen_test]
async fn conflicting_runtime_preserves_pending_document_and_assets_for_download() {
    let first = project_name_test_support::new_runtime();
    project_name_test_support::install(
        &first,
        Session::new(boardstudio_core::CoreEngine::validate_open),
        boardstudio_core::CoreEngine::new(),
    );
    let mut document = in_process_support::board_document();
    document.id = format!("runtime-conflict-{}", js_sys::Date::now());
    document.name = "Original".into();
    first.submit(Event::Open {
        operation_id: first.operation(),
        document: document.clone(),
    });
    project_name_test_support::run_pending(&first).await;
    let second =
        Runtime::new_with_store(false, first.store.clone(), CoreServices::engine(), None).unwrap();
    project_name_test_support::install(
        &second,
        Session::new(boardstudio_core::CoreEngine::validate_open),
        boardstudio_core::CoreEngine::new(),
    );
    let done = project_name_test_support::track_open_saved(&second);
    second.open_saved(document.id.clone());
    done.await.unwrap();
    project_name_test_support::run_pending(&second).await;
    let stale = second.model().accepted.unwrap();
    let accepted = first.model().accepted.unwrap();
    let mut winner = (*accepted.document).clone();
    winner.name = "First tab saved".into();
    project_name_test_support::submit_fixed_command(
        &first,
        EditCommand {
            base_revision: accepted.document.revision,
            transaction_id: "first-tab".into(),
            phase: EditPhase::Commit,
            target_ids: Vec::new(),
            operation: EditOperation::ReplaceDocument {
                document: Box::new(winner),
            },
        },
    );
    project_name_test_support::run_pending(&first).await;
    let mut pending = (*stale.document).clone();
    pending.name = "Second tab pending".into();
    let bytes = b"model retained after conflict".to_vec();
    let hash: String = Sha256::digest(&bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    pending.assets.push(
        serde_json::from_value(serde_json::json!({
            "id":"pending-model", "name":"pending.step", "mediaType":"model/step", "sha256":hash
        }))
        .unwrap(),
    );
    second
        .assets
        .borrow_mut()
        .insert(hash.clone(), bytes.clone());
    project_name_test_support::submit_fixed_command(
        &second,
        EditCommand {
            base_revision: stale.document.revision,
            transaction_id: "second-tab".into(),
            phase: EditPhase::Commit,
            target_ids: Vec::new(),
            operation: EditOperation::ReplaceDocument {
                document: Box::new(pending),
            },
        },
    );
    project_name_test_support::run_pending(&second).await;
    assert_eq!(second.model().lifecycle, Lifecycle::RecoveryRequired);
    assert!(second.has_save_conflict());
    assert_eq!(second.model().accepted.unwrap().document.name, "Original");
    assert_eq!(
        first
            .store
            .load_document(document.id.clone())
            .await
            .unwrap()
            .unwrap()
            .name,
        "First tab saved"
    );
    second.submit(Event::RetrySave {
        operation_id: second.operation(),
    });
    project_name_test_support::run_pending(&second).await;
    assert!(
        second.has_save_conflict(),
        "retry cannot adopt the other tab's token"
    );
    let window = web_sys::window().unwrap();
    let old_confirm = Reflect::get(&window, &"confirm".into()).unwrap();
    let cancel = Closure::<dyn FnMut(JsValue) -> bool>::new(|_| false);
    Reflect::set(&window, &"confirm".into(), cancel.as_ref()).unwrap();
    let done = project_name_test_support::track_open_saved(&second);
    second.open_saved(document.id.clone());
    done.await.unwrap();
    assert_eq!(
        second.delete_saved_project("another-keyboard".into()).await,
        Err("Pending edits were kept; no keyboard was deleted.".into())
    );
    Reflect::set(&window, &"confirm".into(), &old_confirm).unwrap();
    drop(cancel);
    assert_eq!(second.model().lifecycle, Lifecycle::RecoveryRequired);
    second.submit(Event::RetrySave {
        operation_id: second.operation(),
    });
    project_name_test_support::run_pending(&second).await;
    assert!(
        second.has_save_conflict(),
        "cancelled recovery must not adopt a new storage token"
    );
    let (captured, archive) = second.pending_archive().await.unwrap();
    assert_eq!(captured.name, "Second tab pending");
    let (reply, buffers) =
        boardstudio_core::archive::request("{\"kind\":\"unpack-project\"}", &[archive]);
    let ArchiveReply::Unpacked {
        project_json,
        assets,
    } = serde_json::from_str(&reply).unwrap()
    else {
        panic!("pending copy was not a valid archive: {reply}")
    };
    let unpacked: ProjectDoc = serde_json::from_str(&project_json).unwrap();
    assert_eq!(unpacked.name, "Second tab pending");
    assert_eq!(buffers[assets[0].buffer_index as usize], bytes);
    assert!(
        second.store.load_asset(hash).await.unwrap().is_none(),
        "the conflicting asset write was rolled back"
    );
}

#[wasm_bindgen_test]
async fn archive_exactly_at_size_limit_reaches_normal_validation() {
    let window = web_sys::window().unwrap();
    let original = Reflect::get(&window, &"confirm".into()).unwrap();
    let confirm = Closure::<dyn FnMut(JsValue) -> bool>::new(|_| true);
    Reflect::set(&window, &"confirm".into(), confirm.as_ref()).unwrap();
    let runtime = project_name_test_support::new_runtime();
    project_name_test_support::open_document(&runtime, in_process_support::board_document()).await;
    let imported = ProjectDoc::empty("archive-at-limit", "At limit");
    let (mut entered, release) =
        project_name_test_support::gate_import_archive(&runtime, &imported);
    let done = project_name_test_support::track_import_file(&runtime);
    let file =
        web_sys::File::new_with_u8_array_sequence(&Array::new(), "at-limit.boardstudio").unwrap();
    let descriptor = Object::new();
    Reflect::set(
        &descriptor,
        &"value".into(),
        &JsValue::from_f64(128.0 * 1024.0 * 1024.0),
    )
    .unwrap();
    Object::define_property(&Object::from(file.clone()), &"size".into(), &descriptor);
    runtime.import_file(file);
    let mut admitted = false;
    for _ in 0..200 {
        if matches!(entered.try_recv(), Ok(Some(()))) {
            admitted = true;
            break;
        }
        TimeoutFuture::new(1).await;
    }
    assert!(
        admitted,
        "exactly-at-limit files must reach Core: {}",
        runtime.status()
    );
    release.send(()).unwrap();
    done.await.unwrap();
    Reflect::set(&window, &"confirm".into(), &original).unwrap();
    project_name_test_support::run_pending(&runtime).await;
    assert_eq!(
        runtime.model().accepted.unwrap().document.id,
        "archive-at-limit"
    );
}

#[wasm_bindgen_test]
async fn oversized_import_is_rejected_before_opening_or_reading() {
    let runtime = project_name_test_support::new_runtime();
    project_name_test_support::open_document(&runtime, in_process_support::board_document()).await;
    let before = runtime.model().accepted.unwrap();
    let file =
        web_sys::File::new_with_u8_array_sequence(&Array::new(), "oversized.boardstudio").unwrap();
    let descriptor = Object::new();
    Reflect::set(
        &descriptor,
        &"value".into(),
        &JsValue::from_f64(128.0 * 1024.0 * 1024.0 + 1.0),
    )
    .unwrap();
    Object::define_property(&Object::from(file.clone()), &"size".into(), &descriptor);
    runtime.import_file(file);
    assert!(
        runtime.status().contains("128 MiB"),
        "oversized files must be rejected synchronously: {}",
        runtime.status()
    );
    assert_eq!(runtime.model().accepted.unwrap().token, before.token);
}
