use super::super::OutlineVersionInspector;
use super::super::objects::{ScopedTreeContext, TreeContext};
use super::*;
use boardstudio_application::{Event, TerminalOutcome};
use boardstudio_core::model::{
    Board, BoardOutline, OutlineProvenance, OutlineSettings, OutlineSnapshot, OutlineVersion,
    ProjectDoc,
};
use std::{
    cell::{Cell, RefCell},
    future::Future,
    rc::Rc,
    task::{Context, Waker},
};
use wasm_bindgen_test::wasm_bindgen_test;

wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

#[derive(Clone)]
struct Probe {
    runtime: Rc<crate::runtime::Runtime>,
    version: Rc<Cell<u64>>,
    generation: Rc<Cell<u64>>,
    workspace: Rc<Cell<&'static str>>,
    latest: Rc<RefCell<Option<OutlineInspectorProjection>>>,
    context: Rc<RefCell<TreeContext>>,
    activation: Rc<RefCell<Option<EventHandler<OutlineAction>>>>,
}
impl Probe {
    fn projection(&self) -> OutlineInspectorProjection {
        self.latest.borrow().as_ref().unwrap().clone()
    }

    fn watch_next(&self) -> boardstudio_web_runtime::operation_outcomes::OutcomeSlot {
        // The hook reserves one operation for its dispatch-time plan seed, then EditTicket
        // allocates the operation that owns this action. Reserve the seed here so the
        // observed ID remains deterministic across asynchronous Core/Session execution.
        let seed = self.runtime.operation();
        self.runtime
            .observe_operation(boardstudio_application::OperationId(seed.0 + 2))
    }

    fn copy(&self) {
        let projection = self.projection();
        projection.on_action.call(projection.copy_action());
    }

    fn delete(&self) {
        let projection = self.projection();
        projection
            .on_action
            .call(projection.delete_action().unwrap());
    }
}

fn host() -> Element {
    let probe = use_context::<Probe>();
    let mut version = use_signal(|| 0u64);
    use_context_provider(|| version);
    let mut generation = use_signal(|| 1u64);
    let mut workspace = use_signal(|| "Layout");
    let mut selected = use_signal(|| None::<ScopedTreeContext>);
    if *version.peek() != probe.version.get() {
        version.set(probe.version.get());
    }
    if *generation.peek() != probe.generation.get() {
        generation.set(probe.generation.get());
    }
    if *workspace.peek() != probe.workspace.get() {
        workspace.set(probe.workspace.get());
    }
    let next = probe.runtime.scope().map(|scope| ScopedTreeContext {
        scope,
        context: probe.context.borrow().clone(),
    });
    if *selected.peek() != next {
        selected.set(next);
    }
    let (projection, activation) =
        use_outline_lifecycle(probe.runtime.clone(), selected, workspace, generation);
    *probe.activation.borrow_mut() = Some(activation);
    *probe.latest.borrow_mut() = projection.clone();
    rsx! { div { if let Some(projection) = projection { OutlineVersionInspector { projection } } } }
}

fn flush(probe: &Probe, dom: &mut VirtualDom) {
    probe.version.set(probe.version.get() + 1);
    dom.mark_dirty(ScopeId::APP);
    for _ in 0..6 {
        dom.render_immediate_to_vec();
        let mut work = std::pin::pin!(dom.wait_for_work());
        let _ = work.as_mut().poll(&mut Context::from_waker(Waker::noop()));
    }
}

fn document() -> ProjectDoc {
    let mut document = ProjectDoc::empty("outline-hook-tests", "Outline hook tests");
    document.boards.push(Board {
        id: "board".into(),
        name: "Board".into(),
        outline_ids: vec![],
        part_ids: vec![],
        net_ids: vec![],
        thickness: 1.6,
        traces: vec![],
        vias: vec![],
    });
    install_version(&mut document, "fixed");
    document
}

async fn mounted() -> (Probe, VirtualDom) {
    let runtime = crate::runtime::project_name_test_support::new_runtime();
    crate::runtime::project_name_test_support::open_document(&runtime, document()).await;
    let probe = Probe {
        runtime,
        version: Rc::default(),
        generation: Rc::new(Cell::new(1)),
        workspace: Rc::new(Cell::new("Layout")),
        latest: Rc::default(),
        activation: Rc::default(),
        context: Rc::new(RefCell::new(TreeContext::Outline {
            board_id: "board".into(),
        })),
    };
    let mut dom = VirtualDom::new(host);
    dom.provide_root_context(probe.clone());
    dom.rebuild_to_vec();
    flush(&probe, &mut dom);
    (probe, dom)
}

fn install_version(document: &mut ProjectDoc, id: &str) {
    document.board_outlines = vec![BoardOutline {
        board_id: "board".into(),
        active_version_id: Some(id.into()),
        generated_last_valid: None,
        versions: vec![OutlineVersion {
            id: id.into(),
            name: "Edited outline 1".into(),
            source: OutlineProvenance {
                revision: 0,
                version_id: None,
            },
            geometry: OutlineSnapshot {
                features: vec![],
                settings: OutlineSettings::default(),
                expected_regions: 1,
                bridges: vec![],
                protected_gaps: vec![],
            },
        }],
    }];
}

async fn accept(probe: &Probe) {
    crate::runtime::project_name_test_support::run_pending(&probe.runtime).await;
}

async fn replace_document(probe: &Probe, document: ProjectDoc) {
    probe.runtime.submit(Event::Open {
        operation_id: probe.runtime.operation(),
        document,
    });
    accept(probe).await;
}

async fn recover_document(probe: &Probe, document: ProjectDoc) {
    probe.runtime.submit(Event::RecoverWithDocument {
        operation_id: probe.runtime.operation(),
        document,
    });
    accept(probe).await;
}

async fn versioned() -> (Probe, VirtualDom) {
    let (probe, mut dom) = mounted().await;
    let mut document = probe
        .runtime
        .model()
        .accepted
        .unwrap()
        .document
        .as_ref()
        .clone();
    install_version(&mut document, "fixed");
    replace_document(&probe, document).await;
    *probe.context.borrow_mut() = TreeContext::OutlineVersion {
        board_id: "board".into(),
        version_id: None,
    };
    flush(&probe, &mut dom);
    (probe, dom)
}

fn generated_action(probe: &Probe) -> OutlineAction {
    let model = probe.runtime.model();
    OutlineAction::for_tree(
        model.accepted.as_ref().unwrap(),
        &probe.runtime.scope().unwrap(),
        probe.generation.get(),
        &probe.context.borrow(),
    )
    .unwrap()
}

fn activate_generated(probe: &Probe) {
    probe
        .activation
        .borrow()
        .as_ref()
        .unwrap()
        .call(generated_action(probe));
}

#[wasm_bindgen_test]
async fn stale_rendered_copy_generation_is_rejected_after_callback_refresh() {
    let (probe, mut dom) = mounted().await;
    let old = probe.projection();
    let action = old.copy_action();
    probe.generation.set(2);
    flush(&probe, &mut dom);
    old.on_action.call(action);
    accept(&probe).await;
    flush(&probe, &mut dom);
    assert_eq!(
        probe
            .runtime
            .model()
            .accepted
            .unwrap()
            .document
            .board_outlines[0]
            .versions
            .len(),
        1,
        "old rendered Copy must not create a version"
    );
}

#[wasm_bindgen_test]
async fn stale_rendered_delete_generation_is_rejected_after_callback_refresh() {
    let (probe, mut dom) = versioned().await;
    let old = probe.projection();
    let action = old.delete_action().unwrap();
    probe.generation.set(2);
    flush(&probe, &mut dom);
    old.on_action.call(action);
    accept(&probe).await;
    assert_eq!(
        probe
            .runtime
            .model()
            .accepted
            .unwrap()
            .document
            .board_outlines[0]
            .versions
            .len(),
        1
    );
}

#[wasm_bindgen_test]
async fn close_drains_active_edit_then_reopen_retires_its_owner() {
    let (probe, mut dom) = mounted().await;
    let (entered, release) =
        crate::runtime::project_name_test_support::gate_next_core_reply(&probe.runtime);
    let slot = probe.watch_next();
    probe.copy();
    let runtime = probe.runtime.clone();
    let (done_tx, done_rx) = futures_channel::oneshot::channel();
    wasm_bindgen_futures::spawn_local(async move {
        crate::runtime::project_name_test_support::run_pending(&runtime).await;
        let _ = done_tx.send(());
    });
    entered.await.unwrap();

    probe.runtime.submit(Event::Close {
        operation_id: probe.runtime.operation(),
    });
    release.send(()).unwrap();
    done_rx.await.unwrap();
    assert_eq!(*slot.borrow(), Some(TerminalOutcome::Completed));
    assert_eq!(
        probe.runtime.model().lifecycle,
        boardstudio_application::Lifecycle::Closed
    );

    // A closed Session cannot accept another Open. The test adapter installs a fresh real
    // Session and opens the replacement through Core, just as a new editor session does.
    let replacement = document();
    crate::runtime::project_name_test_support::open_document(&probe.runtime, replacement).await;
    flush(&probe, &mut dom);
    assert!(
        probe.projection().enabled,
        "the new Session accepts and presents its opened document"
    );
}

#[wasm_bindgen_test]
async fn admitted_outcome_survives_editor_unmount_until_terminal() {
    let (probe, dom) = mounted().await;
    let (entered, release) =
        crate::runtime::project_name_test_support::gate_next_core_reply(&probe.runtime);
    let slot = probe.watch_next();
    probe.copy();
    let runtime = probe.runtime.clone();
    let (done_tx, done_rx) = futures_channel::oneshot::channel();
    wasm_bindgen_futures::spawn_local(async move {
        crate::runtime::project_name_test_support::run_pending(&runtime).await;
        let _ = done_tx.send(());
    });
    entered.await.unwrap();
    assert!(slot.borrow().is_none());
    drop(dom);
    release.send(()).unwrap();
    done_rx.await.unwrap();
    assert_eq!(*slot.borrow(), Some(TerminalOutcome::Completed));
}

#[wasm_bindgen_test]
async fn hidden_inspector_retains_exact_success_until_same_owner_returns() {
    let (probe, mut dom) = mounted().await;
    let slot = probe.watch_next();
    probe.copy();
    accept(&probe).await;
    assert_eq!(*slot.borrow(), Some(TerminalOutcome::Completed));
    probe.workspace.set("Case");
    flush(&probe, &mut dom);
    assert!(probe.latest.borrow().is_none());
    probe.workspace.set("Layout");
    flush(&probe, &mut dom);
    assert_eq!(probe.projection().feedback.unwrap().state, "saved");
    assert!(probe.projection().enabled);
}

#[wasm_bindgen_test]
async fn exact_rejection_allows_fresh_operation_retry() {
    let (probe, mut dom) = mounted().await;
    crate::runtime::project_name_test_support::fail_next_persist(&probe.runtime, "disk full");
    let rejected = probe.watch_next();
    probe.copy();
    accept(&probe).await;
    assert!(matches!(
        *rejected.borrow(),
        Some(TerminalOutcome::PersistenceFailed(_))
    ));
    assert_eq!(
        probe.runtime.model().lifecycle,
        boardstudio_application::Lifecycle::RecoveryRequired,
        "a failed durable write requires recovery before another edit"
    );
    flush(&probe, &mut dom);
    assert_eq!(probe.projection().feedback.unwrap().state, "failed");

    // The old hand-settled harness could retry immediately after inventing a persistence
    // failure. Real Session keeps edits blocked until the saved document is reopened.
    recover_document(&probe, document()).await;
    assert_eq!(
        probe.runtime.model().lifecycle,
        boardstudio_application::Lifecycle::Ready
    );
    flush(&probe, &mut dom);
    let retried = probe.watch_next();
    probe.copy();
    accept(&probe).await;
    assert_eq!(*retried.borrow(), Some(TerminalOutcome::Completed));
    let saved = probe.runtime.model().accepted.unwrap().document;
    assert!(
        !saved.board_outlines.is_empty(),
        "retry lands through Core and persistence"
    );
}

#[wasm_bindgen_test]
async fn completed_old_epoch_retires_before_replacement_readiness() {
    let (probe, mut dom) = mounted().await;
    let (entered, release) =
        crate::runtime::project_name_test_support::gate_next_core_reply(&probe.runtime);
    let slot = probe.watch_next();
    probe.copy();
    let runtime = probe.runtime.clone();
    let (done_tx, done_rx) = futures_channel::oneshot::channel();
    wasm_bindgen_futures::spawn_local(async move {
        crate::runtime::project_name_test_support::run_pending(&runtime).await;
        let _ = done_tx.send(());
    });
    entered.await.unwrap();
    let mut replacement = document();
    replacement.name = "Next epoch".into();
    replace_document(&probe, replacement).await;
    release.send(()).unwrap();
    done_rx.await.unwrap();
    accept(&probe).await;
    flush(&probe, &mut dom);
    assert_eq!(*slot.borrow(), Some(TerminalOutcome::Completed));
    assert!(probe.projection().enabled);
}

#[wasm_bindgen_test]
async fn completed_current_source_in_recovery_retires_with_failure() {
    let (probe, mut dom) = mounted().await;
    crate::runtime::project_name_test_support::fail_next_persist(&probe.runtime, "disk full");
    let slot = probe.watch_next();
    probe.copy();
    accept(&probe).await;
    assert!(matches!(
        *slot.borrow(),
        Some(TerminalOutcome::PersistenceFailed(_))
    ));
    flush(&probe, &mut dom);
    assert_eq!(
        probe.runtime.model().lifecycle,
        boardstudio_application::Lifecycle::RecoveryRequired
    );
    let feedback = probe.projection().feedback.unwrap();
    assert_eq!(feedback.state, "failed");
    assert!(feedback.message.unwrap().contains("disk full"));
}

#[wasm_bindgen_test]
async fn different_context_hides_inspector_and_rejects_retained_copy() {
    let (probe, mut dom) = mounted().await;
    let old = probe.projection();
    let request = old.copy_action();
    let before = probe.runtime.model().accepted.unwrap().document;
    *probe.context.borrow_mut() = TreeContext::Component {
        part_id: Some("part".into()),
        matrix_id: None,
        row: None,
        column: None,
        assembly_id: None,
    };
    flush(&probe, &mut dom);
    assert!(probe.latest.borrow().is_none());
    old.on_action.call(request);
    accept(&probe).await;
    assert!(
        std::sync::Arc::ptr_eq(&before, &probe.runtime.model().accepted.unwrap().document),
        "a retained action from the prior tree context cannot edit the document"
    );
}

#[wasm_bindgen_test]
async fn activation_requires_saved_source() {
    let (probe, mut dom) = versioned().await;
    crate::runtime::project_name_test_support::fail_next_persist(&probe.runtime, "disk full");
    probe.delete();
    accept(&probe).await;
    flush(&probe, &mut dom);
    let before = probe.runtime.model().accepted.unwrap().document;
    activate_generated(&probe);
    accept(&probe).await;
    assert!(std::sync::Arc::ptr_eq(
        &before,
        &probe.runtime.model().accepted.unwrap().document
    ));
}

#[wasm_bindgen_test]
async fn activation_rejects_stale_rendered_token() {
    let (probe, mut dom) = versioned().await;
    let old = generated_action(&probe);
    let replacement = probe
        .runtime
        .model()
        .accepted
        .unwrap()
        .document
        .as_ref()
        .clone();
    replace_document(&probe, replacement).await;
    flush(&probe, &mut dom);
    assert!(!old.is_current(&probe.runtime, probe.generation.get()));
    let before = probe.runtime.model().accepted.unwrap().document;
    probe.activation.borrow().as_ref().unwrap().call(old);
    accept(&probe).await;
    assert!(
        std::sync::Arc::ptr_eq(&before, &probe.runtime.model().accepted.unwrap().document,),
        "stale token must not create an accepted revision"
    );
}

#[wasm_bindgen_test]
async fn activation_retains_observed_slot_after_editor_unmount() {
    let (probe, dom) = versioned().await;
    let (entered, release) =
        crate::runtime::project_name_test_support::gate_next_core_reply(&probe.runtime);
    let slot = probe.watch_next();
    activate_generated(&probe);
    let runtime = probe.runtime.clone();
    let (done_tx, done_rx) = futures_channel::oneshot::channel();
    wasm_bindgen_futures::spawn_local(async move {
        crate::runtime::project_name_test_support::run_pending(&runtime).await;
        let _ = done_tx.send(());
    });
    entered.await.unwrap();
    drop(dom);
    release.send(()).unwrap();
    done_rx.await.unwrap();
    assert_eq!(*slot.borrow(), Some(TerminalOutcome::Completed));
}

#[wasm_bindgen_test]
async fn activation_rejects_stale_generation_at_refreshed_dispatch() {
    let (probe, mut dom) = versioned().await;
    let old = generated_action(&probe);
    probe.generation.set(2);
    flush(&probe, &mut dom);
    assert!(!old.is_current(&probe.runtime, probe.generation.get()));
    let before = probe.runtime.model().accepted.unwrap().document;
    probe.activation.borrow().as_ref().unwrap().call(old);
    accept(&probe).await;
    assert!(
        std::sync::Arc::ptr_eq(&before, &probe.runtime.model().accepted.unwrap().document,),
        "stale generation must not create an accepted revision"
    );
}

#[wasm_bindgen_test]
async fn activation_settles_exact_generated_target_while_hidden() {
    let (probe, mut dom) = versioned().await;
    let slot = probe.watch_next();
    activate_generated(&probe);
    accept(&probe).await;
    assert_eq!(*slot.borrow(), Some(TerminalOutcome::Completed));
    assert_eq!(
        probe
            .runtime
            .model()
            .accepted
            .unwrap()
            .document
            .board_outlines[0]
            .active_version_id
            .as_deref(),
        None,
        "activation targets the generated outline on this board"
    );
    probe.workspace.set("Case");
    flush(&probe, &mut dom);
    assert!(probe.latest.borrow().is_none());
    probe.workspace.set("Layout");
    flush(&probe, &mut dom);
    assert_eq!(probe.projection().feedback.unwrap().state, "saved");
    let revision = probe.runtime.model().accepted.unwrap().document.revision;
    activate_generated(&probe);
    accept(&probe).await;
    assert_eq!(
        probe.runtime.model().accepted.unwrap().document.revision,
        revision,
        "already active Generated does not add history"
    );
}

#[wasm_bindgen_test]
async fn activation_checks_target_membership_and_selected_version() {
    let (probe, mut dom) = versioned().await;
    *probe.context.borrow_mut() = TreeContext::OutlineVersion {
        board_id: "board".into(),
        version_id: Some("missing".into()),
    };
    flush(&probe, &mut dom);
    activate_generated(&probe);
    accept(&probe).await;
    assert_eq!(
        probe
            .runtime
            .model()
            .accepted
            .unwrap()
            .document
            .board_outlines[0]
            .active_version_id
            .as_deref(),
        Some("fixed")
    );
    let stale = generated_action(&probe);
    *probe.context.borrow_mut() = TreeContext::OutlineVersion {
        board_id: "board".into(),
        version_id: Some("fixed".into()),
    };
    flush(&probe, &mut dom);
    probe.activation.borrow().as_ref().unwrap().call(stale);
    accept(&probe).await;
    let revision = probe.runtime.model().accepted.unwrap().document.revision;
    activate_generated(&probe);
    accept(&probe).await;
    assert_eq!(
        probe
            .runtime
            .model()
            .accepted
            .unwrap()
            .document
            .board_outlines[0]
            .active_version_id
            .as_deref(),
        Some("fixed")
    );
    assert_eq!(
        probe.runtime.model().accepted.unwrap().document.revision,
        revision,
        "already active fixed version is a no-op"
    );
}
