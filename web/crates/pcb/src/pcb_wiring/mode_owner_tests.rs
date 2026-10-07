use super::apply::{BoardWiringApplyActions, BoardWiringApplyFeedback, use_board_wiring_apply};
use super::mode::{
    BoardWiringModeActions, BoardWiringModeEditRequest, BoardWiringModeFeedback,
    CurrentSnapshotBlocker, current_snapshot_probe, use_board_wiring_mode_edits,
};
use super::pins::{
    PcbWiringPinActions, PcbWiringPinEditRequest, PcbWiringPinFeedback, use_pcb_wiring_pin_edits,
};
use super::remap::{ProtectedRemapActions, ProtectedRemapFeedback, use_protected_remap_review};
use super::*;
use boardstudio_application::{Event, OperationId, Resolution, Scope, SessionEpoch, SnapshotToken};
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
    latest_remap: Rc<RefCell<Option<ProtectedRemapActions>>>,
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
        source.clone(),
        resolution,
    );
    let remap_actions = use_protected_remap_review(
        probe.runtime.clone(),
        version,
        workspace,
        generation,
        {
            let probe = probe.clone();
            Rc::new(move || probe.active.get())
        },
        source,
    );
    *probe.latest.borrow_mut() = Some(actions);
    *probe.latest_apply.borrow_mut() = Some(apply_actions);
    *probe.latest_pins.borrow_mut() = Some(pin_actions);
    *probe.latest_remap.borrow_mut() = Some(remap_actions);
    rsx! { div { "mode owner test host" } }
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

fn replace_document(runtime: &Rc<crate::runtime::Runtime>, document: ProjectDoc) {
    let target_id = document.id.clone();
    let resolver =
        boardstudio_application::EditResolver::new("native-test-document-replacement", move |_| {
            boardstudio_application::Resolution::submit(
                vec![target_id.clone()],
                EditOperation::ReplaceDocument {
                    document: Box::new(document.clone()),
                },
            )
        });
    let ticket = boardstudio_web_runtime::edit_ticket::EditTicket::begin(
        runtime,
        "native-test-document-replacement",
        None,
        resolver,
    );
    assert!(matches!(
        ticket.settlement(true),
        boardstudio_web_runtime::edit_ticket::Settlement::Landed { .. }
    ));
}

fn select_part(runtime: &crate::runtime::Runtime, part_id: &str) {
    runtime.submit(Event::SelectParts {
        operation_id: runtime.operation(),
        mode: boardstudio_application::SelectionMode::Replace,
        part_ids: vec![part_id.into()],
        range_part_ids: Vec::new(),
    });
}

fn navigate(runtime: &crate::runtime::Runtime, board_id: &str) {
    runtime.submit(Event::Navigate {
        operation_id: runtime.operation(),
        board_id: board_id.into(),
        instance_id: None,
    });
}

fn assert_no_edit_submitted(runtime: &crate::runtime::Runtime) {
    assert_eq!(mutation_count(runtime), 0);
}

fn mutation_count(runtime: &crate::runtime::Runtime) -> usize {
    runtime
        .events
        .borrow()
        .iter()
        .filter(|event| {
            matches!(
                event,
                Event::ResolveEdit { .. } | Event::ReviewElectricalRemap { .. }
            )
        })
        .count()
}

fn refreshed_source(
    runtime: &crate::runtime::Runtime,
    selected_part_id: Option<&str>,
    generation: u64,
) -> PcbWiringSource {
    let accepted = runtime
        .model()
        .accepted
        .expect("Session has an accepted project");
    let ui_scope = runtime.scope().expect("Session has an active board");
    PcbWiringSource {
        identity: FirmwarePlanIdentity {
            scope: Scope {
                instance_id: None,
                ..ui_scope.clone()
            },
            token: accepted.token,
            revision: accepted.document.revision,
            executor_epoch: runtime.electrical_preview_executor_epoch(),
        },
        ui_scope,
        scope_generation: generation,
        active_part_id: selected_part_id.map(str::to_owned),
        document: accepted.document,
    }
}

fn mounted() -> (Probe, VirtualDom) {
    mounted_with_document(document())
}

fn mounted_with_document(document: ProjectDoc) -> (Probe, VirtualDom) {
    let runtime = crate::runtime::Runtime::new();
    runtime.submit(Event::Open {
        operation_id: runtime.operation(),
        document: document.clone(),
    });
    runtime.submit(Event::Navigate {
        operation_id: runtime.operation(),
        board_id: "left".into(),
        instance_id: None,
    });
    let accepted = runtime.model().accepted.expect("fixture opens in Session");
    let ui_scope = runtime.scope().expect("fixture board is active");
    let mut source = source(accepted.token.0, accepted.document.revision, None, 5);
    source.identity = FirmwarePlanIdentity {
        scope: Scope {
            instance_id: None,
            ..ui_scope.clone()
        },
        token: accepted.token,
        revision: accepted.document.revision,
        executor_epoch: runtime.electrical_preview_executor_epoch(),
    };
    source.ui_scope = ui_scope;
    source.document = accepted.document.clone();
    let plan_identity = source.identity.clone();
    let probe = Probe {
        runtime,
        source: Rc::new(RefCell::new(source)),
        resolution: Rc::new(RefCell::new(PcbWiringResolution::Current {
            identity: plan_identity,
            plan: resolved_plan(&document),
        })),
        latest: Rc::default(),
        latest_apply: Rc::default(),
        latest_pins: Rc::default(),
        latest_remap: Rc::default(),
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
    let Event::ResolveEdit {
        operation_id,
        resolver,
        ..
    } = events
        .iter()
        .rev()
        .find(|event| matches!(event, Event::ResolveEdit { .. }))
        .expect("edit submitted")
    else {
        panic!("one accepted mode choice must submit one resolved edit")
    };
    let accepted = probe.runtime.model().accepted.clone().unwrap();
    let Resolution::Submit(command) = resolver.resolve(&accepted) else {
        panic!("mode choice must resolve to a command")
    };
    let EditOperation::ReplaceDocument { document } = &command.operation else {
        panic!("mode choice must use the existing ReplaceDocument edit")
    };
    (*operation_id, (**document).clone())
}

fn submitted_remap(probe: &Probe) -> (OperationId, u64, String, String) {
    let events = probe.runtime.events.borrow();
    let Event::ReviewElectricalRemap {
        operation_id,
        base_revision,
        board_id,
        expected_fingerprint,
    } = events
        .iter()
        .rev()
        .find(|event| matches!(event, Event::ReviewElectricalRemap { .. }))
        .expect("remap submitted")
    else {
        panic!("one protected-remap review must submit one Session review event")
    };
    (
        *operation_id,
        *base_revision,
        board_id.clone(),
        expected_fingerprint.clone(),
    )
}

fn protected_document() -> ProjectDoc {
    let mut document = document();
    let handoff_revision = document.revision.saturating_sub(1);
    let boards = &mut document
        .hardware
        .get_or_insert_with(Default::default)
        .boards;
    let configuration_index = boards
        .iter()
        .position(|configuration| configuration.board_id == "left");
    let configuration = if let Some(index) = configuration_index {
        &mut boards[index]
    } else {
        boards.push(ElectricalBoardConfiguration {
            board_id: "left".into(),
            ..Default::default()
        });
        boards.last_mut().unwrap()
    };
    configuration.protected_handoff = Some(ElectricalHandoffBaseline {
        fingerprint: "protected-fixture-fingerprint".into(),
        revision: handoff_revision,
        assignments: [("matrix/m/r0c0".into(), "P1".into())].into(),
    });
    document
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

    probe.runtime.hold_next_core();
    let mut opening_document = document();
    opening_document.id = "opening-project".into();
    probe.runtime.submit(Event::Open {
        operation_id: probe.runtime.operation(),
        document: opening_document,
    });
    assert_eq!(
        check("PCB", true).unwrap_err(),
        CurrentSnapshotBlocker::Lifecycle
    );
    probe.runtime.release_core();

    let (preview_probe, _preview_dom) = mounted();
    let preview_identity = preview_probe
        .latest
        .borrow()
        .as_ref()
        .unwrap()
        .identity
        .clone()
        .unwrap();
    preview_probe.runtime.submit(Event::PreviewEdit {
        operation_id: preview_probe.runtime.operation(),
        transaction_id: "owner-probe-preview".into(),
        target_ids: vec!["matrix/m/r0c0".into()],
        operation: EditOperation::MoveParts {
            positions: vec![Position {
                id: "matrix/m/r0c0".into(),
                at: Vec2 { x: 1.0, y: 0.0 },
            }],
        },
    });
    assert_eq!(
        current_snapshot_probe(&preview_probe.runtime, &preview_identity, "PCB", 5, true)
            .unwrap_err(),
        CurrentSnapshotBlocker::Preview
    );

    probe.workspace.set("Layout");
    tick(&probe, &mut dom);
    assert!(!probe.latest_pins.borrow().as_ref().unwrap().editable);
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
    assert_no_edit_submitted(&probe.runtime);

    let free_pin = resolved_plan(&document()).free_pins[0].clone();
    let retained = PcbWiringPinEditRequest {
        identity,
        assignment_id: "row/0".into(),
        pin: Some(free_pin),
    };
    select_part(&probe.runtime, "matrix/m/r0c0");
    tick(&probe, &mut dom);
    assert!(!probe.latest_pins.borrow().as_ref().unwrap().editable);
    actions.on_change.call(retained);
    assert_no_edit_submitted(&probe.runtime);
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
    assert!(probe.latest_pins.borrow().as_ref().unwrap().editable);

    probe.runtime.hold_next_core();
    probe.runtime.submit(Event::Open {
        operation_id: probe.runtime.operation(),
        document: document(),
    });
    tick(&probe, &mut dom);
    assert!(!probe.latest_pins.borrow().as_ref().unwrap().editable);
    probe.runtime.release_core();
    navigate(&probe.runtime, "left");

    navigate(&probe.runtime, "right");
    tick(&probe, &mut dom);
    assert!(!probe.latest_pins.borrow().as_ref().unwrap().editable);

    navigate(&probe.runtime, "left");
    let accepted = probe.runtime.model().accepted.unwrap();
    let ui_scope = probe.runtime.scope().unwrap();
    let identity = FirmwarePlanIdentity {
        scope: Scope {
            instance_id: None,
            ..ui_scope.clone()
        },
        token: accepted.token,
        revision: accepted.document.revision,
        executor_epoch: probe.runtime.electrical_preview_executor_epoch(),
    };
    let current_source = PcbWiringSource {
        identity: identity.clone(),
        ui_scope,
        scope_generation: 5,
        active_part_id: None,
        document: accepted.document.clone(),
    };
    *probe.source.borrow_mut() = current_source;
    *probe.resolution.borrow_mut() = PcbWiringResolution::Current {
        identity,
        plan: resolved_plan(&accepted.document),
    };
    tick(&probe, &mut dom);
    assert!(probe.latest_pins.borrow().as_ref().unwrap().editable);
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
                replace_document(&probe.runtime, accepted);
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
        let mutations_before = mutation_count(&probe.runtime);
        actions.on_apply.call(actions.identity.unwrap());
        assert_eq!(mutation_count(&probe.runtime), mutations_before);
    }
}

#[test]
fn mounted_apply_owner_rejects_retained_action_after_context_changes() {
    let (probe, mut dom) = mounted();
    let old_actions = probe.latest_apply.borrow().as_ref().unwrap().clone();
    select_part(&probe.runtime, "matrix/m/r0c0");
    *probe.source.borrow_mut() = source(1, 0, Some("matrix/m/r0c0"), 5);
    flush(&mut dom);
    old_actions.on_apply.call(old_actions.identity.unwrap());
    assert_no_edit_submitted(&probe.runtime);
}

#[test]
fn mounted_owner_rejects_a_retained_control_after_selection_changes() {
    let (probe, mut dom) = mounted();
    let old_actions = probe.latest.borrow().as_ref().unwrap().clone();
    let old_request = BoardWiringModeEditRequest {
        identity: old_actions.identity.clone().unwrap(),
        mode: ElectricalMode::Direct,
    };
    select_part(&probe.runtime, "matrix/m/r0c0");
    *probe.source.borrow_mut() = source(1, 0, Some("matrix/m/r0c0"), 5);
    flush(&mut dom);
    old_actions.on_change.call(old_request);
    assert_no_edit_submitted(&probe.runtime);
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
                let mut changed_document = document();
                changed_document.id = "other-project".into();
                probe.runtime.submit(Event::Open {
                    operation_id: probe.runtime.operation(),
                    document: changed_document,
                });
            }
            "session" => {
                probe.runtime.submit(Event::Open {
                    operation_id: probe.runtime.operation(),
                    document: document(),
                });
            }
            "board" => navigate(&probe.runtime, "right"),
            "revision" => {
                let mut changed_document =
                    (*probe.runtime.model().accepted.unwrap().document).clone();
                changed_document.name.push_str(" revised");
                replace_document(&probe.runtime, changed_document);
            }
            "workspace" => probe.workspace.set("Layout"),
            "generation" => {
                probe.generation.set(6);
            }
            _ => unreachable!(),
        }
        flush(&mut dom);
        let mutations_before = mutation_count(&probe.runtime);
        old_actions.on_change.call(old_request);
        assert_eq!(mutation_count(&probe.runtime), mutations_before);
    }
}

#[test]
fn mounted_protected_remap_review_submits_one_exact_edit_and_settles_saved_revision() {
    let document = protected_document();
    let (probe, mut dom) = mounted_with_document(document.clone());
    let actions = probe.latest_remap.borrow().as_ref().unwrap().clone();
    let accepted_before_review = probe.runtime.model().accepted.unwrap().document;
    assert_eq!(
        actions.handoff_revision,
        Some(document.revision.saturating_sub(1))
    );
    assert!(actions.editable);
    let identity = actions.identity.clone().unwrap();
    actions.on_review.call(identity);

    let (_operation, base_revision, board_id, fingerprint) = submitted_remap(&probe);
    assert_eq!(base_revision, document.revision);
    assert_eq!(board_id, "left");
    assert_eq!(fingerprint, "protected-fixture-fingerprint");
    let mut proposal = crate::pcb_wiring_remap_operation::propose_review_remap(
        &accepted_before_review,
        &board_id,
        &fingerprint,
    )
    .unwrap();
    proposal.revision += 1;
    let left = proposal
        .hardware
        .as_ref()
        .unwrap()
        .boards
        .iter()
        .find(|configuration| configuration.board_id == "left")
        .unwrap();
    assert!(left.protected_handoff.is_none());
    assert_eq!(
        left.locks,
        document.hardware.as_ref().unwrap().boards[0].locks
    );

    let accepted = probe.runtime.model().accepted.unwrap();
    assert_eq!(accepted.document.as_ref(), &proposal);
    let next_source = refreshed_source(&probe.runtime, None, 5);
    *probe.source.borrow_mut() = next_source;
    tick(&probe, &mut dom);

    let settled = probe.latest_remap.borrow();
    let actions = settled.as_ref().unwrap();
    assert_eq!(actions.handoff_revision, None);
    assert!(matches!(
        actions.feedback.as_ref().unwrap().state,
        ProtectedRemapFeedback::Saved
    ));
}

#[test]
fn mounted_protected_remap_review_rejects_a_retained_fingerprint_after_refresh() {
    let document = protected_document();
    let (probe, mut dom) = mounted_with_document(document.clone());
    let old_actions = probe.latest_remap.borrow().as_ref().unwrap().clone();
    let old_identity = old_actions.identity.clone().unwrap();

    let mut changed = document;
    changed
        .hardware
        .as_mut()
        .unwrap()
        .boards
        .iter_mut()
        .find(|configuration| configuration.board_id == "left")
        .unwrap()
        .protected_handoff
        .as_mut()
        .unwrap()
        .fingerprint = "new-fingerprint".into();
    replace_document(&probe.runtime, changed);
    let current_source = refreshed_source(&probe.runtime, None, 5);
    *probe.source.borrow_mut() = current_source;
    tick(&probe, &mut dom);

    let mutations_before = mutation_count(&probe.runtime);
    old_actions.on_review.call(old_identity);
    assert_eq!(mutation_count(&probe.runtime), mutations_before);
}

#[test]
fn mounted_protected_remap_failures_preserve_the_accepted_handoff() {
    for fail_save in [false, true] {
        let document = protected_document();
        let (probe, mut dom) = mounted_with_document(document.clone());
        if fail_save {
            probe.runtime.fail_next_save("disk");
        } else {
            probe.runtime.fail_next_core("executor");
        }
        probe
            .latest_remap
            .borrow()
            .as_ref()
            .unwrap()
            .on_review
            .call(
                probe
                    .latest_remap
                    .borrow()
                    .as_ref()
                    .unwrap()
                    .identity
                    .clone()
                    .unwrap(),
            );
        let _ = submitted_remap(&probe);
        tick(&probe, &mut dom);
        let accepted = probe.runtime.model();
        assert_eq!(
            accepted
                .accepted
                .as_ref()
                .unwrap()
                .document
                .hardware
                .as_ref()
                .unwrap()
                .boards[0]
                .protected_handoff
                .as_ref()
                .unwrap()
                .fingerprint,
            "protected-fixture-fingerprint"
        );
        assert!(matches!(
            probe
                .latest_remap
                .borrow()
                .as_ref()
                .unwrap()
                .feedback
                .as_ref()
                .unwrap()
                .state,
            ProtectedRemapFeedback::Failed(_)
        ));
    }
}
