use super::apply::{BoardWiringApplyActions, BoardWiringApplyFeedback, use_board_wiring_apply};
use super::mode::{
    BoardWiringModeActions, BoardWiringModeEditRequest, BoardWiringModeFeedback,
    use_board_wiring_mode_edits,
};
use super::*;
use boardstudio_application::{
    AcceptedSnapshot, Durability, Event, Lifecycle, OperationId, ReadModel, Scope, SessionEpoch,
    SnapshotToken, TerminalOutcome,
};
use boardstudio_core::{
    electrical::{ElectricalMode, ElectricalPlan},
    model::{
        Board, EditOperation, Net, Part, Pose2, ProjectDoc, Readiness, SceneDelta, Side, Vec2,
    },
};
use dioxus::prelude::*;
use std::{
    cell::{Cell, RefCell},
    future::Future,
    rc::Rc,
    task::{Context, Waker},
};

#[derive(Clone)]
struct Probe {
    runtime: Rc<crate::runtime::Runtime>,
    source: Rc<RefCell<PcbWiringSource>>,
    resolution: Rc<RefCell<PcbWiringResolution>>,
    latest: Rc<RefCell<Option<BoardWiringModeActions>>>,
    latest_apply: Rc<RefCell<Option<BoardWiringApplyActions>>>,
    version: Rc<Cell<u64>>,
    generation: Rc<Cell<u64>>,
    workspace: Rc<Cell<&'static str>>,
    active: Rc<Cell<bool>>,
}

fn host() -> Element {
    let probe = use_context::<Probe>();
    let mut version = use_signal(|| probe.version.get());
    let mut generation = use_signal(|| probe.generation.get());
    let mut workspace = use_signal(|| probe.workspace.get());
    let mut resolution = use_signal(|| probe.resolution.borrow().clone());
    if *version.peek() != probe.version.get() {
        version.set(probe.version.get());
    }
    if *generation.peek() != probe.generation.get() {
        generation.set(probe.generation.get());
    }
    if *workspace.peek() != probe.workspace.get() {
        workspace.set(probe.workspace.get());
    }
    if *resolution.peek() != *probe.resolution.borrow() {
        resolution.set(probe.resolution.borrow().clone());
    }
    let source = Some(probe.source.borrow().clone());
    let is_current: Rc<dyn Fn() -> bool> = {
        let probe = probe.clone();
        Rc::new(move || probe.active.get())
    };
    let actions = use_board_wiring_mode_edits(
        probe.runtime.clone(),
        version,
        workspace,
        generation,
        is_current,
        source.clone(),
        resolution,
    );
    let apply_actions = use_board_wiring_apply(
        probe.runtime.clone(),
        version,
        workspace,
        generation,
        {
            let probe = probe.clone();
            Rc::new(move || probe.active.get())
        },
        source,
        resolution,
    );
    *probe.latest.borrow_mut() = Some(actions);
    *probe.latest_apply.borrow_mut() = Some(apply_actions);
    rsx! { div { "mode owner test host" } }
}

fn accepted(document: ProjectDoc, token: u64) -> AcceptedSnapshot {
    let revision = document.revision;
    AcceptedSnapshot {
        token: SnapshotToken(token),
        session_epoch: SessionEpoch(3),
        document: std::sync::Arc::new(document),
        scene: std::sync::Arc::new(SceneDelta {
            module_scenes: vec![],
            revision,
            transaction_id: "accepted-mode-test".into(),
            changed_ids: vec![],
            transforms: vec![],
            matrix_scenes: vec![],
            contours: vec![],
            board_contours: vec![],
            board_readiness: vec![],
            board_outline_scenes: vec![],
            finding_markers: vec![],
            findings: vec![],
            readiness: Readiness {
                layout: true,
                outline: true,
                pcb: true,
                case_ready: false,
            },
        }),
    }
}

fn document() -> ProjectDoc {
    let mut document = ProjectDoc::empty("project", "Mode test");
    document.boards.push(Board {
        id: "left".into(),
        name: "Left".into(),
        outline_ids: vec![],
        part_ids: vec!["switch".into()],
        net_ids: vec![],
        thickness: 1.6,
        traces: vec![],
        vias: vec![],
    });
    document.parts.push(Part {
        id: "switch".into(),
        definition_id: "switch-definition".into(),
        reference: "SW1".into(),
        pose: Pose2 {
            at: Vec2 { x: 0.0, y: 0.0 },
            rotation: 0.0,
        },
        side: Side::Front,
        locked: None,
        keycap: None,
        outline: None,
        properties: None,
        generator_parameters: None,
    });
    document.boards.push(Board {
        id: "right".into(),
        name: "Right".into(),
        outline_ids: vec![],
        part_ids: vec![],
        net_ids: vec![],
        thickness: 1.6,
        traces: vec![],
        vias: vec![],
    });
    document
}

fn scope() -> Scope {
    Scope {
        session_epoch: SessionEpoch(3),
        document_id: "project".into(),
        board_id: "left".into(),
        instance_id: None,
    }
}

fn model(document: ProjectDoc, token: u64) -> ReadModel {
    let revision = document.revision;
    ReadModel {
        lifecycle: Lifecycle::Ready,
        durability: Durability::Saved { revision },
        accepted: Some(accepted(document, token)),
        selected_part_ids: vec![],
        active_board_id: "left".into(),
        active_instance_id: None,
        ..Default::default()
    }
}

fn resolved_plan(revision: u64) -> Rc<ElectricalPlan> {
    Rc::new(ElectricalPlan {
        instance_id: None,
        jumpers: vec![],
        module_aliases: Default::default(),
        mode: ElectricalMode::Direct,
        assignments: vec![],
        row_pins: vec![],
        column_pins: vec![],
        diagnostics: vec![],
        fingerprint: "fixture-current-plan".into(),
        board_id: Some("left".into()),
        controller_part_id: None,
        revision,
        controller_profile: None,
        free_pins: vec![],
        nets: vec![Net {
            id: "generated/electrical/left/direct/0".into(),
            name: "SW1".into(),
            pins: vec![],
        }],
        diode_direction: "".into(),
        peripherals: vec![],
        peripheral_pins: Default::default(),
        peripheral_terminals: Default::default(),
    })
}

fn source(
    token: u64,
    revision: u64,
    selected_part_id: Option<&str>,
    generation: u64,
) -> PcbWiringSource {
    let ui_scope = scope();
    let identity = FirmwarePlanIdentity {
        scope: Scope {
            instance_id: None,
            ..ui_scope.clone()
        },
        token: SnapshotToken(token),
        revision,
        executor_epoch: 4,
    };
    super::PcbWiringSource {
        identity,
        ui_scope,
        scope_generation: generation,
        active_part_id: selected_part_id.map(str::to_owned),
    }
}

fn mounted() -> (Probe, VirtualDom) {
    let plan_identity = source(1, 0, None, 5).identity;
    let runtime = crate::runtime::Runtime::new(model(document(), 1), scope());
    let probe = Probe {
        runtime,
        source: Rc::new(RefCell::new(source(1, 0, None, 5))),
        resolution: Rc::new(RefCell::new(PcbWiringResolution::Current {
            identity: plan_identity,
            plan: resolved_plan(0),
        })),
        latest: Rc::default(),
        latest_apply: Rc::default(),
        version: Rc::new(Cell::new(0)),
        generation: Rc::new(Cell::new(5)),
        workspace: Rc::new(Cell::new("PCB")),
        active: Rc::new(Cell::new(true)),
    };
    let mut dom = VirtualDom::new(host);
    dom.provide_root_context(probe.clone());
    dom.rebuild_to_vec();
    flush(&mut dom);
    (probe, dom)
}

fn flush(dom: &mut VirtualDom) {
    dom.mark_dirty(ScopeId::APP);
    for _ in 0..6 {
        dom.render_immediate_to_vec();
        let mut work = std::pin::pin!(dom.wait_for_work());
        let _ = Future::poll(work.as_mut(), &mut Context::from_waker(Waker::noop()));
    }
}

fn tick(probe: &Probe, dom: &mut VirtualDom) {
    probe.version.set(probe.version.get() + 1);
    flush(dom);
}

fn request(probe: &Probe, mode: ElectricalMode) -> BoardWiringModeEditRequest {
    BoardWiringModeEditRequest {
        identity: probe
            .latest
            .borrow()
            .as_ref()
            .unwrap()
            .identity
            .clone()
            .unwrap(),
        mode,
    }
}

fn submitted(probe: &Probe) -> (OperationId, ProjectDoc) {
    let events = probe.runtime.events.borrow();
    let [
        Event::Edit {
            operation_id,
            command,
        },
    ] = events.as_slice()
    else {
        panic!("one accepted mode choice must submit one Edit")
    };
    let EditOperation::ReplaceDocument { document } = &command.operation else {
        panic!("mode choice must use the existing ReplaceDocument edit")
    };
    (*operation_id, (**document).clone())
}

#[test]
fn mounted_apply_owner_submits_one_exact_current_plan_and_settles_after_saved_revision() {
    let (probe, mut dom) = mounted();
    let actions = probe.latest_apply.borrow().as_ref().unwrap().clone();
    assert!(actions.editable);
    let identity = actions.identity.clone().unwrap();
    actions.on_apply.call(identity);
    let events = probe.runtime.events.borrow();
    let [
        Event::Edit {
            operation_id,
            command,
        },
    ] = events.as_slice()
    else {
        panic!("one current plan application must submit one Edit")
    };
    assert_eq!(command.target_ids, vec!["left"]);
    assert_eq!(command.base_revision, 0);
    let EditOperation::ReplaceDocument { document: proposal } = &command.operation else {
        panic!("applying a plan must use the existing ReplaceDocument edit")
    };
    assert_eq!(proposal.nets.len(), 1);
    assert_eq!(proposal.nets[0].id, "generated/electrical/left/direct/0");
    assert_eq!(
        proposal.boards[0].net_ids,
        vec!["generated/electrical/left/direct/0"]
    );
    assert_eq!(
        proposal.hardware.as_ref().unwrap().boards[0].mode,
        ElectricalMode::Direct
    );
    let operation = *operation_id;
    let mut saved = (**proposal).clone();
    drop(events);
    saved.revision = 1;
    *probe.runtime.model.borrow_mut() = model(saved, 2);
    *probe.source.borrow_mut() = source(2, 1, None, 5);
    *probe.resolution.borrow_mut() = PcbWiringResolution::Current {
        identity: probe.source.borrow().identity.clone(),
        plan: resolved_plan(1),
    };
    assert!(probe.runtime.settle(operation, TerminalOutcome::Completed));
    tick(&probe, &mut dom);
    assert!(matches!(
        probe
            .latest_apply
            .borrow()
            .as_ref()
            .unwrap()
            .feedback
            .as_ref()
            .unwrap()
            .state,
        BoardWiringApplyFeedback::Saved
    ));
}

#[test]
fn mounted_apply_owner_rejects_retained_action_after_context_changes() {
    let (probe, mut dom) = mounted();
    let old_actions = probe.latest_apply.borrow().as_ref().unwrap().clone();
    probe.runtime.model.borrow_mut().selected_part_ids = vec!["switch".into()];
    *probe.source.borrow_mut() = source(1, 0, Some("switch"), 5);
    flush(&mut dom);
    old_actions.on_apply.call(old_actions.identity.unwrap());
    assert!(probe.runtime.events.borrow().is_empty());
}

#[test]
fn mounted_apply_owner_keeps_accepted_document_when_session_rejects_edit() {
    let (probe, mut dom) = mounted();
    let actions = probe.latest_apply.borrow().as_ref().unwrap().clone();
    actions.on_apply.call(actions.identity.clone().unwrap());
    let events = probe.runtime.events.borrow();
    let [Event::Edit { operation_id, .. }] = events.as_slice() else {
        panic!("current plan must be submitted before Session settlement")
    };
    let operation = *operation_id;
    drop(events);
    assert!(probe.runtime.settle(
        operation,
        TerminalOutcome::Rejected("stale revision".into())
    ));
    tick(&probe, &mut dom);
    assert_eq!(
        probe
            .runtime
            .model
            .borrow()
            .accepted
            .as_ref()
            .unwrap()
            .document
            .revision,
        0
    );
    assert!(matches!(
        probe
            .latest_apply
            .borrow()
            .as_ref()
            .unwrap()
            .feedback
            .as_ref()
            .unwrap()
            .state,
        BoardWiringApplyFeedback::Failed(_)
    ));
}

#[test]
fn mounted_owner_submits_one_current_edit_and_retains_saved_feedback_after_revision_advance() {
    let (probe, mut dom) = mounted();
    assert!(probe.latest.borrow().as_ref().unwrap().editable);
    let action = probe.latest.borrow().as_ref().unwrap().on_change;
    action.call(request(&probe, ElectricalMode::Direct));
    let (operation, mut proposal) = submitted(&probe);
    assert_eq!(
        proposal.hardware.as_ref().unwrap().boards[0].mode,
        ElectricalMode::Direct
    );

    proposal.revision = 1;
    *probe.runtime.model.borrow_mut() = model(proposal.clone(), 2);
    *probe.source.borrow_mut() = source(2, 1, None, 5);
    *probe.resolution.borrow_mut() = PcbWiringResolution::Current {
        identity: probe.source.borrow().identity.clone(),
        plan: resolved_plan(1),
    };
    assert!(probe.runtime.settle(operation, TerminalOutcome::Completed));
    tick(&probe, &mut dom);

    let actions = probe.latest.borrow();
    assert!(matches!(
        actions.as_ref().unwrap().feedback.as_ref().unwrap().state,
        BoardWiringModeFeedback::Saved
    ));
    assert_eq!(
        actions.as_ref().unwrap().feedback.as_ref().unwrap().target,
        actions
            .as_ref()
            .unwrap()
            .identity
            .as_ref()
            .unwrap()
            .feedback_target()
    );
    assert_eq!(
        probe
            .runtime
            .model
            .borrow()
            .accepted
            .as_ref()
            .unwrap()
            .document
            .hardware
            .as_ref()
            .unwrap()
            .boards[0]
            .mode,
        ElectricalMode::Direct
    );
}

#[test]
fn mounted_owner_rejects_a_retained_control_after_selection_changes() {
    let (probe, mut dom) = mounted();
    let old_actions = probe.latest.borrow().as_ref().unwrap().clone();
    let old_request = BoardWiringModeEditRequest {
        identity: old_actions.identity.clone().unwrap(),
        mode: ElectricalMode::Direct,
    };
    probe.runtime.model.borrow_mut().selected_part_ids = vec!["switch".into()];
    *probe.source.borrow_mut() = source(1, 0, Some("switch"), 5);
    flush(&mut dom);
    old_actions.on_change.call(old_request);
    assert!(probe.runtime.events.borrow().is_empty());
}

#[test]
fn mounted_owner_rejects_retained_controls_after_each_source_identity_change() {
    // Each iteration starts from the legal, current board-context fixture and then changes
    // exactly one admission dimension while retaining the old callback.
    for changed in [
        "project",
        "session",
        "board",
        "revision",
        "workspace",
        "generation",
    ] {
        let (probe, mut dom) = mounted();
        let old_actions = probe.latest.borrow().as_ref().unwrap().clone();
        let old_request = BoardWiringModeEditRequest {
            identity: old_actions.identity.clone().unwrap(),
            mode: ElectricalMode::Direct,
        };
        match changed {
            "project" => {
                let mut model = probe.runtime.model.borrow_mut();
                model.accepted.as_mut().unwrap().document = std::sync::Arc::new({
                    let mut document = document();
                    document.id = "other-project".into();
                    document
                });
                probe.runtime.set_scope(Some(Scope {
                    document_id: "other-project".into(),
                    ..scope()
                }));
            }
            "session" => {
                probe
                    .runtime
                    .model
                    .borrow_mut()
                    .accepted
                    .as_mut()
                    .unwrap()
                    .session_epoch = SessionEpoch(4);
                probe.runtime.set_scope(Some(Scope {
                    session_epoch: SessionEpoch(4),
                    ..scope()
                }));
            }
            "board" => {
                probe.runtime.model.borrow_mut().active_board_id = "right".into();
                probe.runtime.set_scope(Some(Scope {
                    board_id: "right".into(),
                    ..scope()
                }));
            }
            "revision" => {
                let mut model = probe.runtime.model.borrow_mut();
                let accepted = model.accepted.as_mut().unwrap();
                let mut document = (*accepted.document).clone();
                document.revision = 1;
                accepted.document = std::sync::Arc::new(document);
                accepted.token = SnapshotToken(2);
                model.durability = Durability::Saved { revision: 1 };
            }
            "workspace" => probe.workspace.set("Layout"),
            "generation" => {
                probe.generation.set(6);
            }
            _ => unreachable!(),
        }
        flush(&mut dom);
        old_actions.on_change.call(old_request);
        assert!(
            probe.runtime.events.borrow().is_empty(),
            "retained callback submitted after {changed} changed"
        );
    }
}

#[test]
fn failed_feedback_is_hidden_after_accepted_plan_identity_advances() {
    let (probe, mut dom) = mounted();
    probe
        .latest
        .borrow()
        .as_ref()
        .unwrap()
        .on_change
        .call(request(&probe, ElectricalMode::Direct));
    let (operation, _) = submitted(&probe);
    assert!(
        probe
            .runtime
            .settle(operation, TerminalOutcome::Rejected("stale".into()))
    );
    tick(&probe, &mut dom);
    assert!(matches!(
        probe
            .latest
            .borrow()
            .as_ref()
            .unwrap()
            .feedback
            .as_ref()
            .unwrap()
            .state,
        BoardWiringModeFeedback::Failed(_)
    ));

    let mut updated = document();
    updated.revision = 1;
    *probe.runtime.model.borrow_mut() = model(updated, 2);
    *probe.source.borrow_mut() = source(2, 1, None, 5);
    *probe.resolution.borrow_mut() = PcbWiringResolution::Current {
        identity: probe.source.borrow().identity.clone(),
        plan: resolved_plan(1),
    };
    tick(&probe, &mut dom);
    assert!(probe.latest.borrow().as_ref().unwrap().feedback.is_none());
}

#[test]
fn mounted_owner_failures_keep_the_accepted_mode_and_settle_the_exact_observer() {
    for outcome in [
        TerminalOutcome::Rejected("rejected".into()),
        TerminalOutcome::ExecutorFailed("executor".into()),
        TerminalOutcome::PersistenceFailed("disk".into()),
        TerminalOutcome::Cancelled,
    ] {
        let (probe, mut dom) = mounted();
        probe
            .latest
            .borrow()
            .as_ref()
            .unwrap()
            .on_change
            .call(request(&probe, ElectricalMode::Direct));
        let (operation, _) = submitted(&probe);
        assert!(probe.runtime.settle(operation, outcome));
        tick(&probe, &mut dom);
        let accepted = probe.runtime.model.borrow();
        let accepted = accepted.accepted.as_ref().unwrap();
        let mode = accepted
            .document
            .hardware
            .as_ref()
            .and_then(|hardware| {
                hardware
                    .boards
                    .iter()
                    .find(|board| board.board_id == "left")
            })
            .map_or(ElectricalMode::Matrix, |board| board.mode);
        assert_eq!(mode, ElectricalMode::Matrix);
        assert!(matches!(
            probe
                .latest
                .borrow()
                .as_ref()
                .unwrap()
                .feedback
                .as_ref()
                .unwrap()
                .state,
            BoardWiringModeFeedback::Failed(_)
        ));
    }
}
