use super::*;
use crate::runtime::project_name_test_support as test_support;
use boardstudio_application::RequestId;
use boardstudio_model::model::{EditCommand, EditOperation, EditPhase};
use wasm_bindgen_test::wasm_bindgen_test;

/// A runtime whose Core requests run through a fresh in-process engine and whose saves
/// land in the adapter's memory store.
fn installed_runtime() -> Rc<Runtime> {
    let runtime = test_support::new_runtime();
    test_support::install(
        &runtime,
        Session::new(boardstudio_core::CoreEngine::validate_open),
        boardstudio_core::CoreEngine::new(),
    );
    test_support::install_memory_persistence(&runtime);
    runtime
}

/// Open the fixture document through the runtime itself, so every test exercises the
/// installed adapter from open through save.
async fn submit_open(runtime: &Rc<Runtime>, name: &str) -> AcceptedSnapshot {
    let slot = test_support::observe_next(runtime);
    runtime.submit(Event::Open {
        operation_id: runtime.operation(),
        document: ProjectDoc::empty("adapter-test", name),
    });
    test_support::run_pending(runtime).await;
    assert_eq!(
        wait_outcome(&slot).await,
        TerminalOutcome::Completed,
        "the fixture open is accepted"
    );
    runtime.model().accepted.unwrap()
}

fn rename_edit(accepted: &AcceptedSnapshot, name: &str) -> EditCommand {
    let mut document = (*accepted.document).clone();
    document.name = name.into();
    EditCommand {
        base_revision: accepted.document.revision,
        transaction_id: format!("m1-adapter-test-{}", name),
        phase: EditPhase::Commit,
        target_ids: vec![document.id.clone()],
        operation: EditOperation::ReplaceDocument {
            document: Box::new(document),
        },
    }
}

fn submit_edit(
    runtime: &Rc<Runtime>,
    accepted: &AcceptedSnapshot,
    name: &str,
) -> crate::operation_outcomes::OutcomeSlot {
    let slot = test_support::observe_next(runtime);
    test_support::submit_fixed_command(runtime, rename_edit(accepted, name));
    slot
}

async fn wait_outcome(slot: &crate::operation_outcomes::OutcomeSlot) -> TerminalOutcome {
    for _ in 0..200 {
        if let Some(outcome) = slot.borrow().clone() {
            return outcome;
        }
        TimeoutFuture::new(1).await;
    }
    panic!("operation outcome never settled");
}

#[wasm_bindgen_test]
async fn installed_adapter_accepts_an_edit_and_saves_a_memory_copy() {
    let runtime = installed_runtime();
    let accepted = submit_open(&runtime, "Adapter test").await;
    let slot = submit_edit(&runtime, &accepted, "Renamed in memory");
    test_support::run_pending(&runtime).await;
    assert_eq!(wait_outcome(&slot).await, TerminalOutcome::Completed);
    let current = runtime.model().accepted.unwrap();
    assert_eq!(current.document.name, "Renamed in memory");
    assert_eq!(current.document.revision, accepted.document.revision + 1);
    let saved = test_support::saved_document(&runtime, "adapter-test")
        .expect("the completed edit leaves a saved copy in memory");
    assert_eq!(saved.name, "Renamed in memory");
}

#[wasm_bindgen_test]
async fn gated_save_holds_saving_until_released_then_reports_saved() {
    let runtime = installed_runtime();
    let accepted = submit_open(&runtime, "Adapter test").await;
    let (entered, release) = test_support::gate_next_persist(&runtime);
    let slot = submit_edit(&runtime, &accepted, "Gated save");
    test_support::drive_pending(&runtime);
    entered
        .await
        .expect("the gated save reaches the persistence port");
    assert!(
        matches!(runtime.model().durability, Durability::Saving { .. }),
        "the edit stays pending in Saving while the save is held"
    );
    release.send(()).expect("release the held save");
    assert_eq!(wait_outcome(&slot).await, TerminalOutcome::Completed);
    let current = runtime.model().accepted.unwrap();
    assert_eq!(current.document.name, "Gated save");
    assert_eq!(
        runtime.model().durability,
        Durability::Saved {
            revision: current.document.revision
        }
    );
    assert_eq!(
        test_support::saved_document(&runtime, "adapter-test")
            .expect("released save lands in memory")
            .name,
        "Gated save"
    );
}

#[wasm_bindgen_test]
async fn failed_save_requires_recovery_and_keeps_the_accepted_document() {
    let runtime = installed_runtime();
    let accepted = submit_open(&runtime, "Adapter test").await;
    test_support::fail_next_persist(&runtime, "injected durable write failure");
    let slot = submit_edit(&runtime, &accepted, "Must not commit");
    test_support::run_pending(&runtime).await;
    assert_eq!(
        wait_outcome(&slot).await,
        TerminalOutcome::PersistenceFailed("injected durable write failure".into())
    );
    assert_eq!(runtime.model().lifecycle, Lifecycle::RecoveryRequired);
    assert_eq!(
        runtime.model().durability,
        Durability::Failed {
            revision: accepted.document.revision + 1,
            reason: "injected durable write failure".into(),
        }
    );
    assert_eq!(
        runtime.model().accepted.unwrap().document.name,
        "Adapter test",
        "the accepted document keeps the pre-edit value"
    );
    assert_eq!(
        test_support::saved_document(&runtime, "adapter-test")
            .expect("the open left a saved copy in memory")
            .name,
        "Adapter test",
        "the failed edit's save never replaced the accepted copy"
    );
}

#[wasm_bindgen_test]
async fn gated_core_reply_keeps_the_edit_pending_until_released() {
    let runtime = installed_runtime();
    let accepted = submit_open(&runtime, "Adapter test").await;
    let (entered, release) = test_support::gate_next_core_reply(&runtime);
    let slot = submit_edit(&runtime, &accepted, "Parked rename");
    test_support::drive_pending(&runtime);
    entered
        .await
        .expect("the edit request reaches the in-process executor");
    assert_eq!(
        runtime.model().accepted.unwrap().document.revision,
        accepted.document.revision,
        "nothing is accepted while the reply is held"
    );
    release.send(()).expect("release the held reply");
    assert_eq!(wait_outcome(&slot).await, TerminalOutcome::Completed);
    assert_eq!(
        runtime.model().accepted.unwrap().document.name,
        "Parked rename"
    );
}

#[wasm_bindgen_test]
async fn restart_builds_a_fresh_engine_and_stale_replies_are_ignored() {
    let runtime = installed_runtime();
    let accepted = submit_open(&runtime, "Adapter test").await;
    let old_core = test_support::in_process_core(&runtime);
    let old_epoch = runtime.session.borrow().core_executor_epoch();
    test_support::fail_next_core_reply(&runtime, "injected core failure");
    let slot = submit_edit(&runtime, &accepted, "Never applied");
    test_support::run_pending(&runtime).await;
    assert_eq!(
        wait_outcome(&slot).await,
        TerminalOutcome::ExecutorFailed("injected core failure".into())
    );
    assert_eq!(runtime.model().lifecycle, Lifecycle::RecoveryRequired);
    let new_core = test_support::in_process_core(&runtime);
    assert!(
        !Rc::ptr_eq(&old_core, &new_core),
        "the executor restart builds a fresh in-process engine"
    );
    assert!(old_core.is_closed());
    assert!(!new_core.is_closed());
    let current_epoch = runtime.session.borrow().core_executor_epoch();
    assert_ne!(current_epoch, old_epoch, "the executor epoch advanced");
    runtime.complete(Completion::Core {
        request_id: RequestId(2),
        executor_epoch: old_epoch,
        reply: Box::new(CoreReply::Error {
            id: "m1-2".into(),
            message: "late reply from the replaced executor".into(),
            revision: 0,
        }),
    });
    assert_eq!(
        runtime.model().accepted.unwrap().document.name,
        "Adapter test",
        "a late reply from the replaced executor changes nothing"
    );
    assert_eq!(runtime.model().lifecycle, Lifecycle::RecoveryRequired);
}
