use boardstudio_application::{Event, TerminalOutcome};
use boardstudio_core::model::{
    Board, BoardOutline, OutlineProvenance, OutlineSettings, OutlineSnapshot, OutlineVersion,
    ProjectDoc,
};
use boardstudio_web_layout::outline::{
    OutlineAction, OutlineInspectorProjection, OutlineVersionInspector, outline_action_is_current,
    use_outline_lifecycle,
};
use boardstudio_web_ui_model::tree::{ScopedTreeContext, TreeContext};
use dioxus::prelude::*;
use std::{
    cell::{Cell, RefCell},
    future::Future,
    rc::Rc,
    task::{Context, Waker},
};
use wasm_bindgen_test::wasm_bindgen_test;

#[derive(Clone)]
struct Probe {
    runtime: Rc<crate::runtime::Runtime>,
    version: Rc<Cell<u64>>,
    generation: Rc<Cell<u64>>,
    workspace: Rc<Cell<&'static str>>,
    latest: Rc<RefCell<Option<OutlineInspectorProjection>>>,
    context: Rc<RefCell<TreeContext>>,
    activation: Rc<RefCell<Option<EventHandler<OutlineAction>>>>,
    inspector_visible: Rc<Cell<bool>>,
}
impl Probe {
    fn projection(&self) -> OutlineInspectorProjection {
        self.latest.borrow().as_ref().unwrap().clone()
    }

    fn watch_next(&self) -> boardstudio_web_runtime::operation_outcomes::OutcomeSlot {
        // The hook reserves one operation for its dispatch-time plan seed, then EditOutcome
        // allocates the operation that owns this action. Reserve the seed here so the
        // observed ID remains deterministic across asynchronous Core/Session execution.
        let seed = self.runtime.operation();
        self.runtime
            .observe_operation(boardstudio_application::OperationId(seed.0 + 2))
    }

    fn copy_action(&self, projection: &OutlineInspectorProjection) -> OutlineAction {
        let accepted = self.runtime.model().accepted.unwrap();
        OutlineAction::Copy {
            scope: projection.scope.clone(),
            token: accepted.token,
            revision: accepted.document.revision,
            generation: self.generation.get(),
            board_id: projection.board_id.clone(),
        }
    }

    fn delete(&self) {
        let projection = self.projection();
        let accepted = self.runtime.model().accepted.unwrap();
        projection.on_action.call(OutlineAction::Delete {
            scope: projection.scope,
            token: accepted.token,
            revision: accepted.document.revision,
            generation: self.generation.get(),
            board_id: projection.board_id,
            version_id: projection.active_version_id.unwrap(),
        });
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
    let show_inspector = probe.inspector_visible.get();
    rsx! { div { if show_inspector { if let Some(projection) = projection { OutlineVersionInspector { projection } } } } }
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
    let runtime = boardstudio_web_test_runtime::project_name_test_support::new_runtime();
    boardstudio_web_test_runtime::project_name_test_support::open_document(&runtime, document())
        .await;
    let probe = Probe {
        runtime,
        version: Rc::default(),
        generation: Rc::new(Cell::new(1)),
        workspace: Rc::new(Cell::new("Layout")),
        latest: Rc::default(),
        activation: Rc::default(),
        inspector_visible: Rc::new(Cell::new(true)),
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
        retired_version_ids: vec![],
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
            ownership: boardstudio_core::model::OutlineOwnership::Fixed,
        }],
    }];
}

async fn accept(probe: &Probe) {
    boardstudio_web_test_runtime::project_name_test_support::run_pending(&probe.runtime).await;
}

async fn replace_document(probe: &Probe, document: ProjectDoc) {
    probe.runtime.submit(Event::Open {
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
    let action = probe.copy_action(&old);
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
async fn different_context_hides_inspector_and_rejects_retained_copy() {
    let (probe, mut dom) = mounted().await;
    let old = probe.projection();
    let request = probe.copy_action(&old);
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
    boardstudio_web_test_runtime::project_name_test_support::fail_next_persist(
        &probe.runtime,
        "disk full",
    );
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
    assert!(!outline_action_is_current(
        &old,
        &probe.runtime,
        probe.generation.get()
    ));
    let before = probe.runtime.model().accepted.unwrap().document;
    probe.activation.borrow().as_ref().unwrap().call(old);
    accept(&probe).await;
    assert!(
        std::sync::Arc::ptr_eq(&before, &probe.runtime.model().accepted.unwrap().document,),
        "stale token must not create an accepted revision"
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
    assert!(probe.projection().feedback.is_none());
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
