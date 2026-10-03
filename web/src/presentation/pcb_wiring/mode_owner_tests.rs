use super::apply::{BoardWiringApplyActions, BoardWiringApplyFeedback, use_board_wiring_apply};
use super::mode::{
    BoardWiringModeActions, BoardWiringModeEditRequest, BoardWiringModeFeedback,
    CurrentSnapshotBlocker, current_snapshot_probe, use_board_wiring_mode_edits,
};
use super::pins::{
    PcbWiringPinActions, PcbWiringPinEditRequest, PcbWiringPinFeedback, use_pcb_wiring_pin_edits,
};
use super::*;
use boardstudio_application::{
    AcceptedSnapshot, Completion, Durability, Effect, Event, Lifecycle, OperationId, ReadModel,
    SaveResult, Scope, Session, SessionEpoch, SnapshotToken, TerminalOutcome,
};
use boardstudio_core::{
    electrical::{ElectricalDiagnostic, ElectricalMode, ElectricalPlan, ElectricalPlanRequest},
    model::*,
};
use dioxus::prelude::*;
use std::{
    cell::{Cell, RefCell},
    collections::BTreeMap,
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
    latest_pins: Rc<RefCell<Option<PcbWiringPinActions>>>,
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
        source.clone(),
        resolution,
    );
    let pin_actions = use_pcb_wiring_pin_edits(
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
    *probe.latest_pins.borrow_mut() = Some(pin_actions);
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
    document.definitions.push(test_definition(
        "switch-definition",
        PartKind::Switch,
        BTreeMap::from([
            ("row".into(), vec!["1".into()]),
            ("column".into(), vec!["2".into()]),
        ]),
        Some(MatrixTerminals {
            row: "row".into(),
            column: "column".into(),
        }),
        None,
    ));
    document.definitions.push(test_definition(
        "diode-definition",
        PartKind::Passive,
        BTreeMap::from([
            ("anode".into(), vec!["A".into()]),
            ("cathode".into(), vec!["K".into()]),
        ]),
        None,
        None,
    ));
    document.definitions.push(test_definition(
        "mcu-definition",
        PartKind::Controller,
        [
            "GND", "P1", "P2", "P3", "P4", "P5", "P6", "P7", "P8", "P9", "P10", "P14", "P15",
            "P16", "P18", "P19", "P20", "P21",
        ]
        .into_iter()
        .map(|pin| (pin.to_owned(), vec![pin.to_owned()]))
        .collect(),
        None,
        Some("ceoloide/mcu_nice_nano"),
    ));
    let electrical_parts = [
        test_part("matrix/m/r0c0", "switch-definition"),
        test_part("matrix/m/r0c0/diode", "diode-definition"),
        test_part("mcu-left", "mcu-definition"),
    ];
    document.boards.push(Board {
        id: "left".into(),
        name: "Left".into(),
        outline_ids: vec![],
        part_ids: electrical_parts
            .iter()
            .map(|part| part.id.clone())
            .collect(),
        net_ids: vec![],
        thickness: 1.6,
        traces: vec![],
        vias: vec![],
    });
    document.parts.extend(electrical_parts);
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
    document.matrices.push(Matrix {
        id: "m".into(),
        name: None,
        rows: 1,
        columns: 1,
        pitch: Vec2 { x: 19.0, y: 19.0 },
        origin: Vec2::default(),
        definition_id: "switch-definition".into(),
        part_ids: vec!["matrix/m/r0c0".into(), "matrix/m/r0c0/diode".into()],
        board_id: Some("left".into()),
        mirror: None,
        rotation: None,
        edge_gap: None,
        diode_direction: Some(DiodeDirection::Row2col),
        row_offsets: vec![],
        column_offsets: vec![],
        column_staggers: vec![],
        column_splays: vec![],
        column_origins: vec![],
        cells: vec![MatrixCell {
            row: 0,
            column: 0,
            enabled: true,
            definition_id: Some("switch-definition".into()),
            variant: None,
            offset: None,
            rotation: None,
            assemblies: vec![],
            assemblies_local: None,
        }],
    });
    document
}

fn test_part(id: &str, definition_id: &str) -> Part {
    Part {
        id: id.into(),
        definition_id: definition_id.into(),
        reference: id.into(),
        pose: Pose2 {
            at: Vec2::default(),
            rotation: 0.0,
        },
        side: Side::Front,
        locked: None,
        keycap: None,
        outline: None,
        properties: None,
        generator_parameters: None,
    }
}

fn test_definition(
    id: &str,
    kind: PartKind,
    terminals: BTreeMap<String, Vec<String>>,
    matrix_terminals: Option<MatrixTerminals>,
    generator_source: Option<&str>,
) -> PartDefinition {
    let pads = terminals
        .values()
        .flatten()
        .map(|id| Pad {
            id: id.clone(),
            number: id.clone(),
            at: Vec2::default(),
            size: Vec2 { x: 1.0, y: 1.0 },
            shape: PadShape::Circle,
            drill: None,
            plated: Some(true),
            side: None,
            rotation: None,
            net_id: None,
        })
        .collect();
    PartDefinition {
        hardware_profile: None,
        input_profile: None,
        id: id.into(),
        name: id.into(),
        kind,
        keycap: None,
        envelope_source: None,
        kicad_source: None,
        terminals,
        matrix_terminals,
        envelope_notice: None,
        courtyard: vec![],
        pads,
        models: None,
        generator: generator_source.map(|source| PartGenerator {
            source: source.into(),
            version: "test".into(),
            parameters: BTreeMap::new(),
        }),
        mechanical_profile: None,
    }
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

fn resolved_plan(document: &ProjectDoc) -> Rc<ElectricalPlan> {
    let mode = document
        .hardware
        .as_ref()
        .and_then(|hardware| {
            hardware
                .boards
                .iter()
                .find(|entry| entry.board_id == "left")
        })
        .map_or(ElectricalMode::Matrix, |configuration| configuration.mode);
    resolved_plan_with_mode(document, mode)
}

fn resolved_plan_with_mode(document: &ProjectDoc, mode: ElectricalMode) -> Rc<ElectricalPlan> {
    Rc::new(boardstudio_core::electrical::resolve(
        ElectricalPlanRequest {
            instance_id: None,
            document: document.clone(),
            mode,
            locks: Default::default(),
            controller_profile: Some("ceoloide/mcu_nice_nano".into()),
            board_id: Some("left".into()),
            controller_part_id: Some("mcu-left".into()),
        },
    ))
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
        document: std::sync::Arc::new(document()),
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
            plan: resolved_plan(&document()),
        })),
        latest: Rc::default(),
        latest_apply: Rc::default(),
        latest_pins: Rc::default(),
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

fn pin_request(probe: &Probe, assignment_id: &str, pin: Option<String>) -> PcbWiringPinEditRequest {
    PcbWiringPinEditRequest {
        identity: probe
            .latest_pins
            .borrow()
            .as_ref()
            .unwrap()
            .identity
            .clone()
            .unwrap(),
        assignment_id: assignment_id.to_owned(),
        pin,
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
fn mounted_pin_controls_edit_only_selected_board_locks_and_settle_after_save() {
    let (probe, mut dom) = mounted();
    let mut starting = document();
    let mut configuration = ElectricalBoardConfiguration {
        board_id: "left".into(),
        controller_part_id: Some("mcu-left".into()),
        mode: ElectricalMode::Matrix,
        locks: BTreeMap::from([("unrelated".into(), "P10".into())]),
        key_bindings: BTreeMap::from([("matrix/m/r0c0".into(), "A".into())]),
        ..Default::default()
    };
    configuration
        .assignments
        .insert("kept".into(), "P14".into());
    starting.hardware = Some(HardwareConfiguration {
        boards: vec![
            configuration,
            ElectricalBoardConfiguration {
                board_id: "right".into(),
                locks: BTreeMap::from([("right-lock".into(), "P3".into())]),
                ..Default::default()
            },
        ],
        ..Default::default()
    });
    *probe.runtime.model.borrow_mut() = model(starting.clone(), 1);
    *probe.resolution.borrow_mut() = PcbWiringResolution::Current {
        identity: probe.source.borrow().identity.clone(),
        plan: resolved_plan(&starting),
    };
    tick(&probe, &mut dom);

    let actions = probe.latest_pins.borrow().as_ref().unwrap().clone();
    assert!(actions.editable);
    let plan = resolved_plan(&starting);
    let selected_pin = plan
        .free_pins
        .first()
        .expect("fixture has a free pin")
        .clone();
    actions
        .on_change
        .call(pin_request(&probe, "row/0", Some(selected_pin.clone())));
    let (operation, proposal) = submitted_pin_edit(&probe);
    assert_eq!(
        proposal.hardware.as_ref().unwrap().boards[0].locks["row/0"],
        selected_pin
    );
    assert_eq!(
        proposal.hardware.as_ref().unwrap().boards[0].locks["unrelated"],
        "P10"
    );
    assert_eq!(
        proposal.hardware.as_ref().unwrap().boards[0].assignments["kept"],
        "P14"
    );
    assert_eq!(
        proposal.hardware.as_ref().unwrap().boards[0].key_bindings["matrix/m/r0c0"],
        "A"
    );
    assert_eq!(
        proposal.hardware.as_ref().unwrap().boards[1].locks["right-lock"],
        "P3"
    );
    assert_eq!(proposal.boards, starting.boards);

    let mut saved = proposal.clone();
    saved.revision = 1;
    *probe.runtime.model.borrow_mut() = model(saved.clone(), 2);
    *probe.source.borrow_mut() = source(2, 1, None, 5);
    *probe.resolution.borrow_mut() = PcbWiringResolution::Current {
        identity: probe.source.borrow().identity.clone(),
        plan: resolved_plan(&saved),
    };
    assert!(probe.runtime.settle(operation, TerminalOutcome::Completed));
    tick(&probe, &mut dom);
    assert!(matches!(
        probe
            .latest_pins
            .borrow()
            .as_ref()
            .unwrap()
            .feedback
            .as_ref()
            .unwrap()
            .state,
        PcbWiringPinFeedback::Saved
    ));

    probe.runtime.events.borrow_mut().clear();
    let actions = probe.latest_pins.borrow().as_ref().unwrap().clone();
    actions.on_change.call(pin_request(&probe, "row/0", None));
    let (unlock_operation, unlocked) = submitted_pin_edit(&probe);
    assert!(
        !unlocked.hardware.as_ref().unwrap().boards[0]
            .locks
            .contains_key("row/0")
    );
    assert_eq!(
        unlocked.hardware.as_ref().unwrap().boards[0].locks["unrelated"],
        "P10"
    );
    assert_eq!(
        unlocked.hardware.as_ref().unwrap().boards[1].locks["right-lock"],
        "P3"
    );
    assert!(
        probe
            .runtime
            .settle(unlock_operation, TerminalOutcome::Completed)
    );
    let mut unlocked_saved = unlocked.clone();
    unlocked_saved.revision = 2;
    *probe.runtime.model.borrow_mut() = model(unlocked_saved.clone(), 3);
    *probe.source.borrow_mut() = source(3, 2, None, 5);
    *probe.resolution.borrow_mut() = PcbWiringResolution::Current {
        identity: probe.source.borrow().identity.clone(),
        plan: resolved_plan(&unlocked_saved),
    };
    tick(&probe, &mut dom);
    assert!(matches!(
        probe
            .latest_pins
            .borrow()
            .as_ref()
            .unwrap()
            .feedback
            .as_ref()
            .unwrap()
            .state,
        PcbWiringPinFeedback::Saved
    ));
}

fn submitted_pin_edit(probe: &Probe) -> (OperationId, ProjectDoc) {
    let events = probe.runtime.events.borrow();
    let [
        Event::Edit {
            operation_id,
            command,
        },
    ] = events.as_slice()
    else {
        panic!("one assignment pin action must submit one Edit")
    };
    assert_eq!(command.target_ids, vec!["left"]);
    let EditOperation::ReplaceDocument { document } = &command.operation else {
        panic!("assignment pin changes must use the existing ReplaceDocument edit")
    };
    (*operation_id, (**document).clone())
}

#[test]
fn production_snapshot_probe_distinguishes_import_settlement_from_other_admission_gates() {
    let (probe, mut dom) = mounted();
    let identity = probe
        .latest
        .borrow()
        .as_ref()
        .unwrap()
        .identity
        .clone()
        .unwrap();
    let saved = probe.runtime.model.borrow().clone();
    let check = |workspace, current| {
        current_snapshot_probe(&probe.runtime, &identity, workspace, 5, current)
    };
    assert!(check("PCB", true).is_ok());
    assert_eq!(
        check("Layout", true).unwrap_err(),
        CurrentSnapshotBlocker::Workspace
    );
    assert_eq!(
        check("PCB", false).unwrap_err(),
        CurrentSnapshotBlocker::InstanceSelection
    );

    let mut opening = saved.clone();
    opening.lifecycle = Lifecycle::Opening;
    *probe.runtime.model.borrow_mut() = opening;
    assert_eq!(
        check("PCB", true).unwrap_err(),
        CurrentSnapshotBlocker::Lifecycle
    );
    *probe.runtime.model.borrow_mut() = saved.clone();

    let mut preview = saved.clone();
    preview.display_preview = preview
        .accepted
        .as_ref()
        .map(|accepted| accepted.scene.clone());
    *probe.runtime.model.borrow_mut() = preview;
    assert_eq!(
        check("PCB", true).unwrap_err(),
        CurrentSnapshotBlocker::Preview
    );

    *probe.runtime.model.borrow_mut() = saved;
    probe.workspace.set("Layout");
    tick(&probe, &mut dom);
    assert!(!probe.latest_pins.borrow().as_ref().unwrap().editable);
    probe.workspace.set("PCB");
    tick(&probe, &mut dom);
    assert!(probe.latest_pins.borrow().as_ref().unwrap().editable);
}

#[test]
fn mounted_pin_owner_rejects_unavailable_pins_and_retained_selection_actions() {
    let (probe, mut dom) = mounted();
    let actions = probe.latest_pins.borrow().as_ref().unwrap().clone();
    assert!(actions.editable);
    let identity = actions.identity.clone().unwrap();
    actions.on_change.call(PcbWiringPinEditRequest {
        identity: identity.clone(),
        assignment_id: "row/0".into(),
        pin: Some("not-in-current-plan".into()),
    });
    assert!(probe.runtime.events.borrow().is_empty());

    let free_pin = resolved_plan(&document()).free_pins[0].clone();
    let retained = PcbWiringPinEditRequest {
        identity,
        assignment_id: "row/0".into(),
        pin: Some(free_pin),
    };
    probe.runtime.model.borrow_mut().selected_part_ids = vec!["matrix/m/r0c0".into()];
    tick(&probe, &mut dom);
    assert!(!probe.latest_pins.borrow().as_ref().unwrap().editable);
    actions.on_change.call(retained);
    assert!(probe.runtime.events.borrow().is_empty());
}

#[test]
fn assignment_projection_uses_current_mode_rows_locks_and_only_free_pin_choices() {
    let mut document = document();
    let mut configuration = ElectricalBoardConfiguration {
        board_id: "left".into(),
        ..Default::default()
    };
    configuration.locks.insert("row/0".into(), "P2".into());
    document.hardware = Some(HardwareConfiguration {
        boards: vec![configuration],
        ..Default::default()
    });
    let mut source = source(1, 0, None, 5);
    source.document = std::sync::Arc::new(document.clone());
    let matrix = resolved_plan_with_mode(&document, ElectricalMode::Matrix);
    let rows = super::pins::assignments(&source, &matrix);
    let row = rows.iter().find(|row| row.id == "row/0").unwrap();
    assert_eq!(row.label, "Row 1");
    assert!(row.locked);
    let choices = super::pins::pin_choices(row, &matrix);
    assert_eq!(choices.first(), row.value.as_ref());
    assert!(matrix.free_pins.iter().all(|pin| choices.contains(pin)));
    assert_eq!(choices.len(), 1 + matrix.free_pins.len());

    let direct = resolved_plan_with_mode(&document, ElectricalMode::Direct);
    let direct_rows = super::pins::assignments(&source, &direct);
    assert!(direct_rows.iter().any(|row| row.id == "matrix/m/r0c0"));
    assert!(!direct_rows.iter().any(|row| row.id == "row/0"));
}

#[test]
fn mounted_owner_refreshes_from_a_real_session_open_and_board_navigation() {
    let (probe, mut dom) = mounted();
    let mut session = Session::new();
    let mut core = boardstudio_core::CoreEngine::new();
    let open_effects = session.submit(Event::Open {
        operation_id: OperationId(70),
        document: document(),
    });
    *probe.runtime.model.borrow_mut() = session.read_model().clone();
    tick(&probe, &mut dom);
    assert!(!probe.latest_pins.borrow().as_ref().unwrap().editable);

    let mut persist_effects = Vec::new();
    for effect in open_effects {
        if let Effect::Core {
            request_id,
            executor_epoch,
            request,
            ..
        } = effect
        {
            persist_effects.extend(session.complete(Completion::Core {
                request_id,
                executor_epoch,
                reply: Box::new(core.handle(*request)),
            }));
        }
    }
    let opening_snapshot = session.read_model().accepted.as_ref();
    if let Some(snapshot) = opening_snapshot {
        let opening_scope = Scope {
            session_epoch: snapshot.session_epoch,
            document_id: snapshot.document.id.clone(),
            board_id: "left".into(),
            instance_id: None,
        };
        probe.runtime.set_scope(Some(opening_scope.clone()));
        *probe.runtime.model.borrow_mut() = session.read_model().clone();
        let identity = crate::pcb_wiring_mode_operation::BoardWiringModeIdentity {
            plan: WiringPlanIdentity {
                scope: opening_scope.clone(),
                token: snapshot.token,
                revision: snapshot.document.revision,
                executor_epoch: probe.runtime.electrical_preview_executor_epoch(),
            },
            ui_scope: opening_scope,
            selected_part_id: None,
            scope_generation: 5,
        };
        let blocker =
            current_snapshot_probe(&probe.runtime, &identity, "PCB", 5, true).unwrap_err();
        assert!(matches!(
            blocker,
            CurrentSnapshotBlocker::Lifecycle | CurrentSnapshotBlocker::Durability
        ));
    }

    for effect in persist_effects {
        if let Effect::Persist {
            save_attempt_id, ..
        } = effect
        {
            session.complete(Completion::Persist {
                save_attempt_id,
                result: SaveResult::Committed,
            });
        }
    }
    session.submit(Event::Navigate {
        operation_id: OperationId(71),
        board_id: "left".into(),
        instance_id: None,
    });
    let accepted = session.read_model().accepted.as_ref().unwrap().clone();
    let ui_scope = session.scope().expect("opened board is navigable");
    probe.runtime.set_scope(Some(ui_scope.clone()));
    *probe.runtime.model.borrow_mut() = session.read_model().clone();
    let identity = WiringPlanIdentity {
        scope: Scope {
            instance_id: None,
            ..ui_scope.clone()
        },
        token: accepted.token,
        revision: accepted.document.revision,
        executor_epoch: probe.runtime.electrical_preview_executor_epoch(),
    };
    *probe.source.borrow_mut() = super::PcbWiringSource {
        identity: identity.clone(),
        ui_scope: ui_scope.clone(),
        scope_generation: 5,
        active_part_id: None,
        document: accepted.document.clone(),
    };
    *probe.resolution.borrow_mut() = PcbWiringResolution::Current {
        identity: identity.clone(),
        plan: resolved_plan(&accepted.document),
    };
    let mode_identity = crate::pcb_wiring_mode_operation::BoardWiringModeIdentity {
        plan: identity,
        ui_scope,
        selected_part_id: None,
        scope_generation: 5,
    };
    assert!(current_snapshot_probe(&probe.runtime, &mode_identity, "PCB", 5, true).is_ok());
    tick(&probe, &mut dom);
    assert!(probe.latest_pins.borrow().as_ref().unwrap().editable);
}

#[test]
fn mounted_apply_owner_submits_one_exact_current_plan_and_settles_after_saved_revision() {
    let (probe, mut dom) = mounted();
    let actions = probe.latest_apply.borrow().as_ref().unwrap().clone();
    assert!(actions.editable);
    let expected_plan = resolved_plan(&document());
    assert_eq!(expected_plan.mode, ElectricalMode::Matrix);
    assert!(
        expected_plan
            .diagnostics
            .iter()
            .all(|diagnostic| diagnostic.severity != "error")
    );
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
    assert_eq!(proposal.nets, expected_plan.nets);
    assert_eq!(
        proposal.boards[0].net_ids,
        expected_plan
            .nets
            .iter()
            .map(|net| net.id.clone())
            .collect::<Vec<_>>()
    );
    assert_eq!(
        proposal.hardware.as_ref().unwrap().boards[0].mode,
        ElectricalMode::Matrix
    );
    let operation = *operation_id;
    let mut saved = (**proposal).clone();
    drop(events);
    saved.revision = 1;
    *probe.runtime.model.borrow_mut() = model(saved.clone(), 2);
    *probe.source.borrow_mut() = source(2, 1, None, 5);
    *probe.resolution.borrow_mut() = PcbWiringResolution::Current {
        identity: probe.source.borrow().identity.clone(),
        plan: resolved_plan(&saved),
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
fn mounted_apply_owner_rejects_mode_mismatched_or_unavailable_current_plan() {
    for state in ["mode-mismatch", "error", "pending", "failed"] {
        let (probe, mut dom) = mounted();
        let identity = probe.source.borrow().identity.clone();
        match state {
            "mode-mismatch" => {
                let accepted = document();
                let direct_plan = resolved_plan_with_mode(&accepted, ElectricalMode::Direct);
                assert!(
                    direct_plan
                        .diagnostics
                        .iter()
                        .all(|diagnostic| diagnostic.severity != "error")
                );
                *probe.runtime.model.borrow_mut() = model(accepted, 1);
                *probe.resolution.borrow_mut() = PcbWiringResolution::Current {
                    identity,
                    plan: direct_plan,
                };
            }
            "error" => {
                let mut plan = (*resolved_plan(&document())).clone();
                plan.diagnostics.push(ElectricalDiagnostic {
                    code: "test-error".into(),
                    severity: "error".into(),
                    message: "plan must be resolved before applying".into(),
                    key_id: None,
                });
                *probe.resolution.borrow_mut() = PcbWiringResolution::Current {
                    identity,
                    plan: Rc::new(plan),
                };
            }
            "pending" => {
                *probe.resolution.borrow_mut() = PcbWiringResolution::Pending { identity };
            }
            "failed" => {
                *probe.resolution.borrow_mut() = PcbWiringResolution::Failed {
                    identity,
                    message: "resolver failed".into(),
                };
            }
            _ => unreachable!(),
        }
        tick(&probe, &mut dom);
        let actions = probe.latest_apply.borrow().as_ref().unwrap().clone();
        assert!(!actions.editable, "{state} plans must disable Apply");
        actions.on_apply.call(actions.identity.unwrap());
        assert!(
            probe.runtime.events.borrow().is_empty(),
            "{state} plan submitted an edit"
        );
    }
}

#[test]
fn mounted_apply_owner_rejects_retained_action_after_context_changes() {
    let (probe, mut dom) = mounted();
    let old_actions = probe.latest_apply.borrow().as_ref().unwrap().clone();
    probe.runtime.model.borrow_mut().selected_part_ids = vec!["matrix/m/r0c0".into()];
    *probe.source.borrow_mut() = source(1, 0, Some("matrix/m/r0c0"), 5);
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
        plan: resolved_plan(&proposal),
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
    probe.runtime.model.borrow_mut().selected_part_ids = vec!["matrix/m/r0c0".into()];
    *probe.source.borrow_mut() = source(1, 0, Some("matrix/m/r0c0"), 5);
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
    *probe.runtime.model.borrow_mut() = model(updated.clone(), 2);
    *probe.source.borrow_mut() = source(2, 1, None, 5);
    *probe.resolution.borrow_mut() = PcbWiringResolution::Current {
        identity: probe.source.borrow().identity.clone(),
        plan: resolved_plan(&updated),
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
