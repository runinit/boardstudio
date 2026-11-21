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

async fn quota_latched_runtime(active_name: &str) -> Rc<Runtime> {
    let runtime = test_support::new_runtime();
    test_support::install(
        &runtime,
        Session::new(CoreEngine::validate_open),
        CoreEngine::new(),
    );
    let active = ProjectDoc::empty("quota-delete-active", active_name);
    runtime.submit(Event::Open {
        operation_id: runtime.operation(),
        document: active.clone(),
    });
    test_support::run_pending(&runtime).await;
    let other = ProjectDoc::empty("quota-delete-other", "Other board");
    runtime
        .store
        .save_document(&other, &BTreeMap::new())
        .await
        .unwrap();
    test_support::fail_next_persist(&runtime, "QuotaExceededError: origin storage is full");
    let mut edited = active.clone();
    edited.name = "Oversized failed edit".into();
    test_support::replace_document(&runtime, "quota-delete-edit", &active.id, edited).await;
    assert_eq!(runtime.model().lifecycle, Lifecycle::RecoveryRequired);
    assert!(
        runtime.has_pending_edits(),
        "the aborted save latches pending edits"
    );
    runtime
}

#[wasm_bindgen_test]
async fn quota_delete_non_active_skips_recovery_writes_and_keeps_pending_for_explicit_retry() {
    let runtime = quota_latched_runtime("Active board").await;
    // Persistent quota: every save aborts, not just the latched one.
    test_support::fail_next_persist(&runtime, "QuotaExceededError: origin storage is full");
    let active_id = "quota-delete-active";
    let mut pending_target = test_support::track_next_persist_target(&runtime);
    let confirm = ConfirmChoice::new();
    confirm.answer.set(true);
    let result = runtime
        .delete_saved_project("quota-delete-other".into())
        .await;
    assert!(
        result.is_ok(),
        "quota delete must succeed where the save aborted: {result:?}"
    );
    assert_eq!(
        confirm.calls.get(),
        1,
        "one explicit discard-confirm gates the recovery delete"
    );
    test_support::run_pending(&runtime).await;
    assert!(
        pending_target.try_recv().unwrap().is_none(),
        "no recovery or replacement write may run before the single-key delete"
    );
    assert!(
        runtime
            .store
            .load_document("quota-delete-other".into())
            .await
            .unwrap()
            .is_none(),
        "the other keyboard is gone"
    );
    assert_eq!(
        runtime
            .store
            .load_document(active_id.into())
            .await
            .unwrap()
            .unwrap()
            .name,
        "Active board",
        "the durable active copy is untouched"
    );
    assert_eq!(runtime.model().accepted.unwrap().document.id, active_id);
    assert_eq!(runtime.model().lifecycle, Lifecycle::RecoveryRequired);
    assert!(
        runtime.has_pending_edits(),
        "the pending doc stays available for explicit retry or download"
    );
    let remaining = runtime.store.list_documents().await.unwrap();
    assert!(
        remaining
            .iter()
            .all(|document| document.id != "quota-delete-fresh"),
        "no replacement keyboard is created by the delete"
    );
}

#[wasm_bindgen_test]
async fn quota_delete_active_removes_only_the_saved_copy() {
    let runtime = quota_latched_runtime("Active board").await;
    // Persistent quota: every save aborts, not just the latched one.
    test_support::fail_next_persist(&runtime, "QuotaExceededError: origin storage is full");
    let active_id = "quota-delete-active";
    let confirm = ConfirmChoice::new();
    confirm.answer.set(true);
    let result = runtime.delete_saved_project(active_id.into()).await;
    assert!(
        result.is_ok(),
        "quota delete of the active keyboard must succeed: {result:?}"
    );
    test_support::run_pending(&runtime).await;
    assert!(
        runtime
            .store
            .load_document(active_id.into())
            .await
            .unwrap()
            .is_none(),
        "the active saved copy is gone"
    );
    assert!(
        runtime
            .store
            .load_document("quota-delete-other".into())
            .await
            .unwrap()
            .is_some(),
        "the other keyboard is untouched"
    );
    assert_eq!(
        runtime.model().accepted.unwrap().document.id,
        active_id,
        "the accepted in-memory keyboard stays for the explicit next step"
    );
    assert_eq!(runtime.model().lifecycle, Lifecycle::RecoveryRequired);
    assert!(
        !runtime.saved_tokens.borrow().contains_key(active_id),
        "the saved token is dropped with the record"
    );
}

#[wasm_bindgen_test]
async fn quota_create_new_discards_pending_without_committing_oversized_doc() {
    let runtime = quota_latched_runtime("Active board").await;
    test_support::fail_next_persist(&runtime, "QuotaExceededError: origin storage is full");
    let confirm = ConfirmChoice::new();
    confirm.answer.set(true);
    let (id, outcome) = runtime
        .create_new_keyboard()
        .expect("discard-confirmed create-new must replace");
    assert_eq!(confirm.calls.get(), 1);
    assert!(
        !runtime.has_pending_edits(),
        "the explicit discard clears the latched pending doc immediately"
    );
    test_support::run_pending(&runtime).await;
    assert!(
        matches!(
            &*outcome.borrow(),
            Some(TerminalOutcome::PersistenceFailed(_))
        ),
        "the fresh save fails explicitly under quota instead of retrying silently"
    );
    assert_eq!(
        runtime.model().accepted.unwrap().document.id,
        "quota-delete-active",
        "under quota the accepted keyboard stays until a save succeeds"
    );
    let durable = runtime
        .store
        .load_document("quota-delete-active".into())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        durable.name, "Active board",
        "the oversized doc is never committed"
    );
    assert_eq!(durable.revision, 0);
    // The escape hatch retains only the fresh small doc: an explicit retry
    // (quota simulation spent) commits and installs it, proving the
    // oversized candidate was discarded rather than retried.
    runtime.submit(Event::RetrySave {
        operation_id: runtime.operation(),
    });
    test_support::run_pending(&runtime).await;
    assert_eq!(
        runtime.model().accepted.unwrap().document.id,
        id,
        "explicit retry installs the fresh keyboard, not the oversized edit"
    );
    assert!(
        runtime
            .store
            .load_document(id.clone())
            .await
            .unwrap()
            .is_some(),
        "the fresh keyboard is durably saved"
    );
}
