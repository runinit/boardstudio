use super::super::objects::{ScopedTreeContext, TreeContext};
use super::*;
use crate::runtime;
use boardstudio_core::model::{
    BoardOutline, OutlineProvenance, OutlineSettings, OutlineSnapshot, OutlineVersion,
};
use std::{
    cell::{Cell, RefCell},
    future::Future,
    task::{Context, Waker},
};

#[derive(Clone)]
struct Probe {
    runtime: Rc<runtime::Runtime>,
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
    fn edits(&self) -> usize {
        self.runtime
            .events
            .borrow()
            .iter()
            .filter(|event| matches!(event, Event::Edit { .. }))
            .count()
    }
    fn copy(&self) {
        let projection = self.projection();
        projection.on_action.call(projection.copy_action());
    }
    fn terminal(&self, outcome: TerminalOutcome) {
        let operation = self
            .runtime
            .events
            .borrow()
            .iter()
            .find_map(|event| match event {
                Event::Edit { operation_id, .. } => Some(*operation_id),
                _ => None,
            })
            .unwrap();
        assert!(
            self.runtime.outcomes.settle(operation, outcome),
            "exact admitted observer must survive Editor unmount"
        );
        crate::poll_detached();
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
fn mounted() -> (Probe, VirtualDom) {
    crate::TASKS.with_borrow_mut(Vec::clear);
    let probe = Probe {
        runtime: runtime::Runtime::new(),
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
fn install_version(model: &mut boardstudio_application::ReadModel, id: &str) {
    let doc = std::sync::Arc::make_mut(&mut model.accepted.as_mut().unwrap().document);
    doc.board_outlines = vec![BoardOutline {
        board_id: "board".into(),
        active_version_id: Some(id.into()),
        generated_last_valid: None,
        versions: vec![OutlineVersion {
            id: id.into(),
            name: "Edited outline 1".into(),
            source: OutlineProvenance {
                revision: 10,
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
#[test]
fn stale_rendered_copy_generation_is_rejected_after_callback_refresh() {
    let (probe, mut dom) = mounted();
    let old = probe.projection();
    let action = old.copy_action();
    probe.generation.set(2);
    flush(&probe, &mut dom);
    old.on_action.call(action);
    flush(&probe, &mut dom);
    assert_eq!(
        probe.edits(),
        0,
        "old rendered Copy must not acquire the refreshed callback generation"
    );
}
#[test]
fn stale_rendered_delete_generation_is_rejected_after_callback_refresh() {
    let (probe, mut dom) = mounted();
    install_version(&mut probe.runtime.model.borrow_mut(), "fixed");
    *probe.context.borrow_mut() = TreeContext::OutlineVersion {
        board_id: "board".into(),
        version_id: Some("fixed".into()),
    };
    flush(&probe, &mut dom);
    let old = probe.projection();
    let action = old.delete_action().unwrap();
    probe.generation.set(2);
    flush(&probe, &mut dom);
    old.on_action.call(action);
    flush(&probe, &mut dom);
    assert_eq!(
        probe.edits(),
        0,
        "old rendered Delete must not acquire the refreshed callback generation"
    );
}
#[test]
fn completed_without_accepted_snapshot_retires_before_reopen() {
    let (probe, mut dom) = mounted();
    probe.copy();
    *probe.runtime.model.borrow_mut() = Default::default();
    probe.terminal(TerminalOutcome::Completed);
    flush(&probe, &mut dom);
    *probe.runtime.model.borrow_mut() = runtime::model("A", 1, 10);
    flush(&probe, &mut dom);
    assert!(
        probe.projection().enabled,
        "closed exact terminal cannot block a newly accepted document"
    );
}
#[test]
fn admitted_outcome_survives_editor_unmount_until_terminal() {
    let (probe, dom) = mounted();
    probe.copy();
    assert_eq!(probe.edits(), 1);
    drop(dom);
    probe.terminal(TerminalOutcome::Completed);
}
#[test]
fn hidden_inspector_retains_exact_success_until_same_owner_returns() {
    let (probe, mut dom) = mounted();
    probe.copy();
    let id = probe
        .runtime
        .events
        .borrow()
        .iter()
        .find_map(|event| match event {
            Event::Edit { command, .. } => match &command.operation {
                EditOperation::CopyOutline { version_id, .. } => Some(version_id.clone()),
                _ => None,
            },
            _ => None,
        })
        .unwrap();
    probe.workspace.set("Case");
    flush(&probe, &mut dom);
    let mut accepted = runtime::model("A", 2, 11);
    install_version(&mut accepted, &id);
    *probe.runtime.model.borrow_mut() = accepted;
    probe.terminal(TerminalOutcome::Completed);
    flush(&probe, &mut dom);
    assert!(probe.latest.borrow().is_none());
    probe.workspace.set("Layout");
    flush(&probe, &mut dom);
    assert_eq!(probe.projection().feedback.unwrap().state, "saved");
    assert!(probe.projection().enabled);
}
#[test]
fn exact_rejection_allows_fresh_operation_retry() {
    let (probe, mut dom) = mounted();
    probe.copy();
    probe.terminal(TerminalOutcome::Rejected("invalid".into()));
    flush(&probe, &mut dom);
    assert_eq!(probe.projection().feedback.unwrap().state, "rejected");
    probe.copy();
    let operations: Vec<_> = probe
        .runtime
        .events
        .borrow()
        .iter()
        .filter_map(|event| match event {
            Event::Edit { operation_id, .. } => Some(*operation_id),
            _ => None,
        })
        .collect();
    assert_eq!(operations.len(), 2);
    assert_ne!(operations[0], operations[1]);
}

#[test]
fn completed_old_epoch_retires_before_replacement_readiness() {
    let (probe, mut dom) = mounted();
    probe.copy();
    let mut replacement = runtime::model("A", 1, 1);
    replacement.accepted.as_mut().unwrap().session_epoch = boardstudio_application::SessionEpoch(2);
    replacement.lifecycle = Lifecycle::Opening;
    *probe.runtime.model.borrow_mut() = replacement;
    probe.terminal(TerminalOutcome::Completed);
    flush(&probe, &mut dom);
    assert_eq!(
        probe.runtime.observed.borrow()[0].strong_count(),
        0,
        "exact old-epoch terminal must retire before evaluating replacement readiness"
    );
}

#[test]
fn completed_current_source_in_recovery_retires_with_failure() {
    let (probe, mut dom) = mounted();
    probe.copy();
    probe.runtime.model.borrow_mut().lifecycle = Lifecycle::RecoveryRequired;
    probe.terminal(TerminalOutcome::Completed);
    flush(&probe, &mut dom);
    assert_eq!(
        probe.projection().feedback.unwrap().state,
        "recovery-required"
    );
    assert_eq!(probe.runtime.observed.borrow()[0].strong_count(), 0);
}

#[test]
fn different_context_hides_inspector_and_rejects_retained_copy() {
    let (probe, mut dom) = mounted();
    let old = probe.projection();
    let request = old.copy_action();
    *probe.context.borrow_mut() = TreeContext::Component {
        part_id: "part".into(),
    };
    flush(&probe, &mut dom);
    assert!(probe.latest.borrow().is_none());
    old.on_action.call(request);
    assert_eq!(probe.edits(), 0);
}

fn activation_fixture() -> (Probe, VirtualDom) {
    let (probe, mut dom) = mounted();
    install_version(&mut probe.runtime.model.borrow_mut(), "fixed");
    *probe.context.borrow_mut() = TreeContext::OutlineVersion {
        board_id: "board".into(),
        version_id: None,
    };
    flush(&probe, &mut dom);
    (probe, dom)
}
fn generated_action(probe: &Probe) -> OutlineAction {
    OutlineAction::for_tree(
        probe.runtime.model().accepted.as_ref().unwrap(),
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

#[test]
fn activation_requires_saved_source() {
    let (probe, mut dom) = activation_fixture();
    probe.runtime.model.borrow_mut().lifecycle = Lifecycle::RecoveryRequired;
    flush(&probe, &mut dom);
    activate_generated(&probe);
    assert_eq!(probe.edits(), 0);
}
#[test]
fn activation_rejects_stale_rendered_token() {
    let (probe, mut dom) = activation_fixture();
    let old = generated_action(&probe);
    let mut replacement = runtime::model("A", 2, 11);
    install_version(&mut replacement, "fixed");
    *probe.runtime.model.borrow_mut() = replacement;
    flush(&probe, &mut dom);
    assert!(
        !old.is_current(&probe.runtime, probe.generation.get()),
        "root admission rejects before changing tree selection"
    );
    probe.activation.borrow().as_ref().unwrap().call(old);
    assert_eq!(probe.edits(), 0);
}
#[test]
fn activation_retains_observed_slot_after_editor_unmount() {
    let (probe, dom) = activation_fixture();
    activate_generated(&probe);
    assert_eq!(probe.edits(), 1);
    drop(dom);
    probe.terminal(TerminalOutcome::Completed);
}

#[test]
fn activation_rejects_stale_generation_at_refreshed_dispatch() {
    let (probe, mut dom) = activation_fixture();
    let old = generated_action(&probe);
    probe.generation.set(2);
    flush(&probe, &mut dom);
    assert!(
        !old.is_current(&probe.runtime, probe.generation.get()),
        "root admission rejects before changing tree selection"
    );
    probe.activation.borrow().as_ref().unwrap().call(old);
    assert_eq!(probe.edits(), 0);
}

#[test]
fn activation_settles_exact_generated_target_while_hidden() {
    let (probe, mut dom) = activation_fixture();
    activate_generated(&probe);
    activate_generated(&probe);
    assert_eq!(
        probe.edits(),
        1,
        "one observed pending activation at a time"
    );
    assert!(
        matches!(&probe.runtime.events.borrow()[0], Event::Edit { command, .. } if matches!(&command.operation, EditOperation::SelectOutline { board_id, version_id: None } if board_id == "board"))
    );
    probe.workspace.set("Case");
    flush(&probe, &mut dom);
    let mut accepted = runtime::model("A", 2, 11);
    install_version(&mut accepted, "fixed");
    std::sync::Arc::make_mut(&mut accepted.accepted.as_mut().unwrap().document).board_outlines[0]
        .active_version_id = None;
    *probe.runtime.model.borrow_mut() = accepted;
    probe.terminal(TerminalOutcome::Completed);
    flush(&probe, &mut dom);
    assert!(probe.latest.borrow().is_none());
    probe.workspace.set("Layout");
    flush(&probe, &mut dom);
    assert_eq!(probe.projection().feedback.unwrap().state, "saved");
    activate_generated(&probe);
    assert_eq!(
        probe.edits(),
        1,
        "already active Generated does not add history"
    );
}

#[test]
fn activation_checks_target_membership_and_selected_version() {
    let (probe, mut dom) = activation_fixture();
    *probe.context.borrow_mut() = TreeContext::OutlineVersion {
        board_id: "board".into(),
        version_id: Some("missing".into()),
    };
    flush(&probe, &mut dom);
    activate_generated(&probe);
    assert_eq!(probe.edits(), 0, "missing accepted version cannot activate");
    let stale = generated_action(&probe);
    *probe.context.borrow_mut() = TreeContext::OutlineVersion {
        board_id: "board".into(),
        version_id: Some("fixed".into()),
    };
    flush(&probe, &mut dom);
    probe.activation.borrow().as_ref().unwrap().call(stale);
    assert_eq!(
        probe.edits(),
        0,
        "event target must match the selected version"
    );
    activate_generated(&probe);
    assert_eq!(probe.edits(), 0, "already active fixed version is a no-op");
}
