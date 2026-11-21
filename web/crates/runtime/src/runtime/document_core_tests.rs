use super::*;
use crate::edit_ticket::{EditTicket, Settlement};
use boardstudio_application::{EditResolver, Resolution};
use boardstudio_model::model::{EditOperation, MatrixPlacementChange};
use wasm_bindgen_test::wasm_bindgen_test;

fn runtime() -> Rc<Runtime> {
    let mut engine = boardstudio_core::CoreEngine::new();
    runtime_with_core(Box::new(move |request| engine.handle(request)))
}

fn runtime_with_core(core: DocumentCore) -> Rc<Runtime> {
    let runtime = Runtime::new_with_restoration(false, CoreServices::engine(), Some(core)).unwrap();
    // A dead worker must not prevent Open, document history or matrix projection.
    runtime.core.borrow().close();
    runtime
}

pub(super) mod profile;

async fn wait_for_outcome(outcome: &crate::operation_outcomes::OutcomeSlot) -> TerminalOutcome {
    for _ in 0..200 {
        if let Some(outcome) = outcome.borrow().clone() {
            return outcome;
        }
        TimeoutFuture::new(1).await;
    }
    panic!("operation did not settle");
}

fn submit(
    runtime: &Rc<Runtime>,
    event: impl FnOnce(OperationId) -> Event,
) -> crate::operation_outcomes::OutcomeSlot {
    let operation = runtime.operation();
    let outcome = runtime.observe_operation(operation);
    runtime.submit(event(operation));
    outcome
}

fn open(runtime: &Rc<Runtime>, id: &str) -> crate::operation_outcomes::OutcomeSlot {
    submit(runtime, |operation_id| Event::Open {
        operation_id,
        document: ProjectDoc::empty(id, "Original"),
    })
}

fn rename(runtime: &Rc<Runtime>, name: &'static str) -> EditTicket {
    EditTicket::begin(
        runtime,
        "project name",
        None,
        EditResolver::rename_project("rename", name.into()),
    )
}

fn stagger(runtime: &Rc<Runtime>, matrix: &'static str, value: f64) -> EditTicket {
    EditTicket::begin(
        runtime,
        "column stagger",
        None,
        EditResolver::new("stagger", move |_| {
            Resolution::submit(
                vec![matrix.into()],
                EditOperation::SetMatrixPlacement {
                    matrix_id: matrix.into(),
                    change: MatrixPlacementChange::ColumnStagger { column: 0, value },
                },
            )
        }),
    )
}

#[wasm_bindgen_test]
async fn page_core_commits_before_return_and_lands_in_save_order() {
    let runtime = runtime();
    assert_eq!(
        wait_for_outcome(&open(&runtime, "first")).await,
        TerminalOutcome::Completed
    );
    let initial = runtime.model().accepted.unwrap();
    let revision = initial.document.revision;

    let first = rename(&runtime, "First edit");
    assert_eq!(
        runtime.model().durability,
        Durability::Saving {
            revision: revision + 1
        },
        "Core has prepared the save before the normal edit submission returns"
    );
    assert_eq!(first.settlement(true), Settlement::Pending);
    assert_eq!(runtime.model().accepted.unwrap().token, initial.token);

    let rejected = stagger(&runtime, "missing", 2.0);
    let second = rename(&runtime, "Second edit");
    assert_eq!(runtime.model().accepted.unwrap().document.name, "Original");

    let undo = submit(&runtime, |operation_id| Event::Undo { operation_id });
    let first_outcome = runtime.observe_operation(first.operation());
    let rejected_outcome = runtime.observe_operation(rejected.operation());
    let second_outcome = runtime.observe_operation(second.operation());
    runtime.subscribe(Rc::new({
        let undo = undo.clone();
        move || {
            if rejected_outcome.borrow().is_some() {
                assert_eq!(*first_outcome.borrow(), Some(TerminalOutcome::Completed));
            }
            if second_outcome.borrow().is_some() {
                assert!(matches!(
                    *rejected_outcome.borrow(),
                    Some(TerminalOutcome::Rejected(_))
                ));
            }
            if undo.borrow().is_some() {
                assert_eq!(*second_outcome.borrow(), Some(TerminalOutcome::Completed));
            }
        }
    }));
    assert_eq!(wait_for_outcome(&undo).await, TerminalOutcome::Completed);
    runtime.unsubscribe();
    assert_eq!(
        first.settlement(true),
        Settlement::Landed {
            revision: revision + 1
        }
    );
    assert!(
        matches!(rejected.settlement(true), Settlement::Failed { message } if message.contains("Matrix not found"))
    );
    assert_eq!(
        second.settlement(true),
        Settlement::Landed {
            revision: revision + 2
        }
    );
    let undone = runtime.model().accepted.unwrap();
    assert_eq!(undone.document.name, "First edit");
    assert_eq!(undone.session_epoch, initial.session_epoch);
    assert_eq!(
        runtime
            .store
            .load_document("first".into())
            .await
            .unwrap()
            .unwrap()
            .name,
        "First edit"
    );
    assert!(
        runtime
            .project_matrices(undone.document.revision, vec![])
            .await
            .unwrap()
            .is_empty()
    );

    let redo = submit(&runtime, |operation_id| Event::Redo { operation_id });
    assert_eq!(wait_for_outcome(&redo).await, TerminalOutcome::Completed);
    let redone = runtime.model().accepted.unwrap();
    assert_eq!(redone.document.name, "Second edit");
    assert_eq!(redone.session_epoch, initial.session_epoch);
    assert!(
        runtime
            .project_matrices(undone.document.revision, vec![])
            .await
            .is_err()
    );
}

#[wasm_bindgen_test]
async fn page_core_open_retires_old_edits_and_resets_history() {
    let runtime = runtime();
    assert_eq!(
        wait_for_outcome(&open(&runtime, "first")).await,
        TerminalOutcome::Completed
    );
    let epoch = runtime.model().accepted.unwrap().session_epoch;
    let first_edit = rename(&runtime, "Waiting on save");
    let replacement = open(&runtime, "replacement");
    let obsolete = rename(&runtime, "Must not cross Open");
    assert_eq!(
        wait_for_outcome(&replacement).await,
        TerminalOutcome::Completed
    );
    let accepted = runtime.model().accepted.unwrap();
    assert_ne!(accepted.session_epoch, epoch);
    assert_eq!(accepted.document.id, "replacement");
    assert_eq!(accepted.document.name, "Original");
    assert_eq!(obsolete.settlement(true), Settlement::Retired);
    assert_eq!(first_edit.settlement(true), Settlement::Retired);
    assert_eq!(
        runtime
            .store
            .load_document("first".into())
            .await
            .unwrap()
            .unwrap()
            .name,
        "Waiting on save"
    );

    let undo = submit(&runtime, |operation_id| Event::Undo { operation_id });
    assert!(matches!(
        wait_for_outcome(&undo).await,
        TerminalOutcome::Rejected(_)
    ));
    assert_eq!(runtime.model().accepted.unwrap().token, accepted.token);
}

#[wasm_bindgen_test]
async fn page_core_save_conflict_keeps_the_accepted_snapshot_until_recovery() {
    let runtime = runtime();
    assert_eq!(
        wait_for_outcome(&open(&runtime, "conflict")).await,
        TerminalOutcome::Completed
    );
    let initial = runtime.model().accepted.unwrap();
    let load = || runtime.store.load_document_with_token("conflict".into());
    let saved = load().await.unwrap().unwrap();
    let mut competing = saved.document;
    competing.name = "Saved in another tab".into();
    competing.revision += 1;
    runtime
        .store
        .save_document_expected(&competing, &BTreeMap::new(), Some(&saved.token))
        .await
        .unwrap();

    let failed = rename(&runtime, "Conflicting edit");
    let outcome = runtime.observe_operation(failed.operation());
    assert!(matches!(
        wait_for_outcome(&outcome).await,
        TerminalOutcome::PersistenceFailed(_)
    ));
    assert_eq!(runtime.model().accepted.unwrap().token, initial.token);
    assert_eq!(runtime.model().lifecycle, Lifecycle::RecoveryRequired);
    assert!(runtime.has_save_conflict());
    let saved = load().await.unwrap().unwrap();
    assert_eq!(saved.document.name, competing.name);

    let recovered = submit(&runtime, |operation_id| Event::RecoverWithDocument {
        operation_id,
        document: runtime.remember_saved(saved),
    });
    assert_eq!(
        wait_for_outcome(&recovered).await,
        TerminalOutcome::Completed
    );
    let accepted = runtime.model().accepted.unwrap();
    assert_eq!(accepted.document.name, competing.name);
    assert_ne!(accepted.session_epoch, initial.session_epoch);
    assert!(!runtime.has_save_conflict());
    let undo = submit(&runtime, |operation_id| Event::Undo { operation_id });
    assert!(matches!(
        wait_for_outcome(&undo).await,
        TerminalOutcome::Rejected(_)
    ));
}

#[wasm_bindgen_test]
async fn page_core_stagger_prepares_before_persistence_runs() {
    use futures_channel::oneshot;

    struct HeldSave {
        underlying: Rc<dyn DocumentPersistence>,
        started: Cell<Option<oneshot::Sender<()>>>,
        release: Cell<Option<oneshot::Receiver<()>>>,
    }

    impl DocumentPersistence for HeldSave {
        fn save_document<'a>(
            &'a self,
            document: &'a ProjectDoc,
            assets: &'a BTreeMap<String, Vec<u8>>,
            expected: Option<&'a StorageToken>,
        ) -> Pin<Box<dyn Future<Output = Result<StorageToken, SaveError>> + 'a>> {
            let started = self.started.take().expect("one edit is being saved");
            let release = self.release.take().unwrap();
            started.send(()).unwrap();
            Box::pin(async move {
                release.await.unwrap();
                self.underlying
                    .save_document(document, assets, expected)
                    .await
            })
        }
    }

    let runtime = runtime();
    let mut document = profile::fixture(true);
    document.id.push_str("-held-save");
    let id = document.id.clone();
    let opened = submit(&runtime, |operation_id| Event::Open {
        operation_id,
        document,
    });
    assert_eq!(wait_for_outcome(&opened).await, TerminalOutcome::Completed);
    let initial = runtime.model().accepted.unwrap();
    let revision = initial.document.revision + 1;

    let (started, mut entered) = oneshot::channel();
    let (release, held) = oneshot::channel();
    let underlying = runtime.persistence.borrow().clone();
    let persistence = Rc::new(HeldSave {
        underlying,
        started: Cell::new(Some(started)),
        release: Cell::new(Some(held)),
    });
    *runtime.persistence.borrow_mut() = persistence.clone();

    let ticket = stagger(&runtime, "matrix", 3.125);
    assert_eq!(entered.try_recv().unwrap(), None);
    assert_eq!(runtime.model().durability, Durability::Saving { revision });
    assert_eq!(runtime.model().accepted.as_ref(), Some(&initial));
    assert_eq!(ticket.settlement(true), Settlement::Pending);

    entered.await.unwrap();
    assert_eq!(runtime.model().accepted.as_ref(), Some(&initial));
    assert_eq!(ticket.settlement(true), Settlement::Pending);
    let stored = runtime
        .store
        .load_document(id.clone())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(&stored, initial.document.as_ref());

    release.send(()).unwrap();
    let outcome = runtime.observe_operation(ticket.operation());
    assert_eq!(wait_for_outcome(&outcome).await, TerminalOutcome::Completed);
    let accepted = runtime.model().accepted.unwrap();
    assert_eq!(
        accepted.document.matrices[0].column_layout[0].stagger,
        3.125
    );
    assert_eq!(accepted.scene.revision, revision);
    assert_ne!(accepted.token, initial.token);
    assert_eq!(ticket.settlement(true), Settlement::Landed { revision });
    let stored = runtime.store.load_document(id).await.unwrap().unwrap();
    assert_eq!(&stored, accepted.document.as_ref());
    *runtime.persistence.borrow_mut() = persistence.underlying.clone();
}

#[wasm_bindgen_test]
async fn page_core_measures_rename_and_stagger_submission() {
    let runtime = runtime();
    let document: ProjectDoc = serde_json::from_str(include_str!(
        "../../../../../core/tests/fixtures/synthetic-outline.json"
    ))
    .unwrap();
    let opened = submit(&runtime, |operation_id| Event::Open {
        operation_id,
        document,
    });
    assert_eq!(wait_for_outcome(&opened).await, TerminalOutcome::Completed);

    let performance =
        Reflect::get(web_sys::window().unwrap().as_ref(), &"performance".into()).unwrap();
    let now: Function = Reflect::get(&performance, &"now".into())
        .unwrap()
        .dyn_into()
        .unwrap();
    let now = || now.call0(&performance).unwrap().as_f64().unwrap();
    for label in ["rename", "column stagger"] {
        let mut samples = Vec::new();
        for i in 0..36 {
            let revision = runtime.model().accepted.unwrap().document.revision + 1;
            let start = now();
            let ticket = if label == "rename" {
                rename(&runtime, ["First name", "Second name"][i % 2])
            } else {
                stagger(&runtime, "matrix", [2.0, 4.0][i % 2])
            };
            let elapsed = now() - start;
            assert_eq!(runtime.model().durability, Durability::Saving { revision });
            let outcome = runtime.observe_operation(ticket.operation());
            assert_eq!(wait_for_outcome(&outcome).await, TerminalOutcome::Completed);
            assert_eq!(ticket.settlement(true), Settlement::Landed { revision });
            if i >= 6 {
                samples.push(elapsed);
            }
        }
        samples.sort_by(f64::total_cmp);
        web_sys::console::log_1(
            &format!(
                "{label}: n=30, p50={:.3} ms, p95={:.3} ms",
                samples[14], samples[28]
            )
            .into(),
        );
    }
}
