//! Queued PCB edits exercise the real Session/Core and saving adapter.
use super::*;
use crate::runtime::{Runtime, project_name_test_support as support};
use boardstudio_application::{EditResolver, Event, Resolution};
use boardstudio_core::{
    electrical::{self, ElectricalPlanRequest},
    model::*,
};
use std::{cell::RefCell, collections::BTreeMap, rc::Rc};
use wasm_bindgen::JsCast;
use wasm_bindgen_test::*;

#[derive(Clone)]
struct Probe {
    runtime: Rc<Runtime>,
    setup: Rc<RefCell<Option<crate::pcb_physical_setup::PhysicalSetupMount>>>,
    mode: Rc<RefCell<Option<BoardWiringModeActions>>>,
    pins: Rc<RefCell<Option<PcbWiringPinActions>>>,
    apply: Rc<RefCell<Option<BoardWiringApplyActions>>>,
    nets: Rc<RefCell<Option<PartNetActions>>>,
}

fn plan(document: &ProjectDoc) -> Rc<ElectricalPlan> {
    let config = document.hardware.as_ref().and_then(|hardware| {
        hardware
            .boards
            .iter()
            .find(|board| board.board_id == "left")
    });
    Rc::new(electrical::resolve(ElectricalPlanRequest {
        document: document.clone(),
        instance_id: None,
        mode: config.map_or(ElectricalMode::Matrix, |config| config.mode),
        locks: config.map_or_else(Default::default, |config| config.locks.clone()),
        controller_profile: Some("ceoloide/mcu_nice_nano".into()),
        controller_part_id: Some("mcu-left".into()),
        board_id: Some("left".into()),
    }))
}

fn host() -> Element {
    let probe = use_context::<Probe>();
    let runtime = probe.runtime.clone();
    let version = use_signal(|| 0u64);
    use_context_provider(|| version);
    let observed = runtime.clone();
    use_hook(move || {
        observed.subscribe(Rc::new(move || {
            let mut version = version;
            version += 1;
        }))
    });
    let _ = version();
    let workspace = use_signal(|| "PCB");
    let generation = use_signal(|| 0u64);
    let accepted = runtime.model().accepted.unwrap();
    let scope = runtime.scope().unwrap();
    let identity = WiringPlanIdentity {
        scope: scope.clone(),
        token: accepted.token,
        revision: accepted.document.revision,
        executor_epoch: runtime.electrical_preview_executor_epoch(),
    };
    let source = Some(PcbWiringSource {
        identity: identity.clone(),
        ui_scope: scope.clone(),
        scope_generation: 0,
        active_part_id: runtime.model().selected_part_ids.first().cloned(),
        document: accepted.document.clone(),
    });
    let current = PcbWiringResolution::Current {
        identity,
        plan: plan(&accepted.document),
    };
    let mut resolution = use_signal(|| current.clone());
    if *resolution.peek() != current {
        resolution.set(current);
    }
    *probe.mode.borrow_mut() = Some(use_board_wiring_mode_edits(
        runtime.clone(),
        version,
        workspace,
        generation,
        Rc::new(|| true),
        source.clone(),
        resolution,
    ));
    *probe.pins.borrow_mut() = Some(use_pcb_wiring_pin_edits(
        runtime.clone(),
        version,
        workspace,
        generation,
        Rc::new(|| true),
        source.clone(),
        resolution,
    ));
    *probe.apply.borrow_mut() = Some(use_board_wiring_apply(
        runtime.clone(),
        version,
        workspace,
        generation,
        Rc::new(|| true),
        source,
        resolution,
    ));
    *probe.nets.borrow_mut() = Some(use_pcb_part_net_edits(
        runtime,
        version,
        workspace,
        generation,
        Rc::new(|| true),
    ));
    let setup_runtime = probe.runtime.clone();
    let current_runtime = setup_runtime.clone();
    let preference = use_signal(|| None);
    *probe.setup.borrow_mut() = Some(crate::pcb_physical_setup::use_controller(
        setup_runtime,
        version,
        generation,
        Rc::new(|| true),
        crate::InstanceSelection(preference),
        Rc::new(move |owner, strict| {
            let model = current_runtime.model();
            model.accepted.as_ref().is_some_and(|accepted| {
                owner.session_epoch == accepted.session_epoch
                    && owner.document_id == accepted.document.id
                    && owner.board_id == model.active_board_id
                    && owner.instance_id == model.active_instance_id
                    && owner.generation == 0
                    && (!strict
                        || owner.token == accepted.token
                            && owner.revision == accepted.document.revision)
            })
        }),
        Rc::new(|document, intent| {
            Box::pin(async move {
                boardstudio_web_catalogue::catalogue::prepare_physical_setup_proposal(
                    &document, intent,
                )
            })
        }),
    ));
    let actions = probe.mode.borrow().as_ref().unwrap().clone();
    let shown_mode = actions
        .identity
        .as_ref()
        .and_then(mode::pending_mode)
        .unwrap_or_else(|| plan(&accepted.document).mode);
    let mode_value = if shown_mode == ElectricalMode::Direct {
        "direct"
    } else {
        "matrix"
    };
    rsx! { div { select { id: "queued-mode", value: mode_value, disabled: !actions.editable,
        option { value: "matrix", "Matrix" } option { value: "direct", "Direct GPIO" }
    } } }
}

async fn mounted() -> (Probe, web_sys::Element) {
    mounted_with_document(document()).await
}
async fn mounted_with_document(document_fixture: ProjectDoc) -> (Probe, web_sys::Element) {
    let runtime = support::new_runtime();
    support::open_document(&runtime, document_fixture).await;
    runtime.submit(Event::Navigate {
        operation_id: runtime.operation(),
        board_id: "left".into(),
        instance_id: None,
    });
    let probe = Probe {
        runtime,
        setup: Rc::new(RefCell::new(None)),
        mode: Rc::new(RefCell::new(None)),
        pins: Rc::new(RefCell::new(None)),
        apply: Rc::new(RefCell::new(None)),
        nets: Rc::new(RefCell::new(None)),
    };
    let document = web_sys::window().unwrap().document().unwrap();
    let root = document.create_element("div").unwrap();
    document.body().unwrap().append_child(&root).unwrap();
    let dom = VirtualDom::new(host);
    dom.provide_root_context(probe.clone());
    dioxus_web::launch::launch_virtual_dom(
        dom,
        dioxus_web::Config::new().rootnode(root.clone().into()),
    );
    rendered().await;
    (probe, root)
}
async fn rendered() {
    gloo_timers::future::TimeoutFuture::new(30).await;
}
async fn settle(runtime: &Rc<Runtime>) {
    for _ in 0..12 {
        support::run_pending(runtime).await;
        rendered().await;
    }
}
fn configuration(runtime: &Runtime) -> ElectricalBoardConfiguration {
    runtime
        .model()
        .accepted
        .unwrap()
        .document
        .hardware
        .as_ref()
        .unwrap()
        .boards
        .iter()
        .find(|board| board.board_id == "left")
        .unwrap()
        .clone()
}

#[wasm_bindgen_test]
async fn wiring_mode_and_peripheral_pin_lock_queue_and_undo_in_order() {
    let (probe, root) = mounted().await;
    let runtime = &probe.runtime;
    let accepted = runtime.model().accepted.unwrap();
    let first_plan = plan(&accepted.document);
    let assignment = first_plan
        .peripheral_terminals
        .keys()
        .next()
        .unwrap()
        .clone();
    let direct = crate::pcb_wiring_mode_operation::propose_mode(
        &accepted.document,
        "left",
        ElectricalMode::Direct,
    )
    .unwrap();
    let direct_plan = plan(&direct);
    let pin = first_plan
        .free_pins
        .iter()
        .find(|pin| direct_plan.free_pins.contains(pin))
        .unwrap()
        .clone();
    let (entered, release) = support::gate_next_core_reply(runtime);
    let actions = probe.mode.borrow().as_ref().unwrap().clone();
    actions.on_change.call(BoardWiringModeEditRequest {
        identity: actions.identity.unwrap(),
        mode: ElectricalMode::Direct,
    });
    support::drive_pending(runtime);
    entered.await.unwrap();
    rendered().await;
    let displayed = root
        .query_selector("#queued-mode")
        .unwrap()
        .unwrap()
        .dyn_into::<web_sys::HtmlSelectElement>()
        .unwrap();
    assert_eq!(displayed.value(), "direct");
    assert!(!displayed.disabled());
    let actions = probe.pins.borrow().as_ref().unwrap().clone();
    assert!(actions.editable);
    actions.on_change.call(PcbWiringPinEditRequest {
        identity: actions.identity.unwrap(),
        assignment_id: assignment.clone(),
        pin: Some(pin.clone()),
    });
    release.send(()).unwrap();
    settle(runtime).await;
    let config = configuration(runtime);
    assert_eq!(config.mode, ElectricalMode::Direct);
    assert_eq!(config.locks.get(&assignment), Some(&pin));
    runtime.submit(Event::Undo {
        operation_id: runtime.operation(),
    });
    settle(runtime).await;
    let config = configuration(runtime);
    assert_eq!(config.mode, ElectricalMode::Direct);
    assert!(!config.locks.contains_key(&assignment));
    runtime.submit(Event::Undo {
        operation_id: runtime.operation(),
    });
    settle(runtime).await;
    assert!(
        runtime
            .model()
            .accepted
            .unwrap()
            .document
            .hardware
            .as_ref()
            .is_none_or(|hardware| hardware.boards.is_empty())
    );
    root.remove();
}

#[wasm_bindgen_test]
async fn part_net_assignment_queued_behind_layout_preserves_both_edits_and_undo() {
    let (probe, root) = mounted().await;
    let runtime = &probe.runtime;
    runtime.submit(Event::SelectParts {
        operation_id: runtime.operation(),
        part_ids: vec!["connector".into()],
        range_part_ids: vec![],
        mode: boardstudio_application::SelectionMode::Replace,
    });
    rendered().await;
    let (entered, release) = support::gate_next_core_reply(runtime);
    runtime.submit(Event::ResolveEdit {
        operation_id: runtime.operation(),
        label: "layout-fixture".into(),
        resolver: EditResolver::new(
            "layout-fixture",
            |accepted: &boardstudio_application::AcceptedSnapshot| {
                let mut document = (*accepted.document).clone();
                document
                    .parts
                    .iter_mut()
                    .find(|part| part.id == "connector")
                    .unwrap()
                    .pose
                    .at
                    .x = 40.0;
                Resolution::Submit(EditCommand {
                    base_revision: accepted.document.revision,
                    transaction_id: String::new(),
                    phase: EditPhase::Commit,
                    target_ids: vec!["connector".into()],
                    operation: EditOperation::ReplaceDocument {
                        document: Box::new(document),
                    },
                })
            },
        ),
    });
    support::drive_pending(runtime);
    entered.await.unwrap();
    rendered().await;
    let actions = probe.nets.borrow().as_ref().unwrap().clone();
    actions.on_edit.call(PartNetEditRequest {
        identity: actions.identity.unwrap(),
        action: PartNetEditAction::AssignPads {
            pad_ids: vec!["1".into()],
            net_id: Some("net".into()),
        },
    });
    release.send(()).unwrap();
    settle(runtime).await;
    let doc = runtime.model().accepted.unwrap().document;
    assert_eq!(
        doc.parts
            .iter()
            .find(|part| part.id == "connector")
            .unwrap()
            .pose
            .at
            .x,
        40.0
    );
    assert!(
        doc.nets[0]
            .pins
            .iter()
            .any(|pin| pin.part_id == "connector")
    );
    runtime.submit(Event::Undo {
        operation_id: runtime.operation(),
    });
    settle(runtime).await;
    assert!(
        runtime.model().accepted.unwrap().document.nets[0]
            .pins
            .is_empty()
    );
    assert_eq!(
        runtime
            .model()
            .accepted
            .unwrap()
            .document
            .parts
            .iter()
            .find(|part| part.id == "connector")
            .unwrap()
            .pose
            .at
            .x,
        40.0
    );
    runtime.submit(Event::Undo {
        operation_id: runtime.operation(),
    });
    settle(runtime).await;
    assert_eq!(
        runtime
            .model()
            .accepted
            .unwrap()
            .document
            .parts
            .iter()
            .find(|part| part.id == "connector")
            .unwrap()
            .pose
            .at
            .x,
        0.0
    );
    root.remove();
}

#[wasm_bindgen_test]
async fn plan_queued_behind_part_deletion_is_refused_and_does_not_restore_the_part() {
    let mut fixture = document();
    fixture.parts.retain(|part| part.id != "connector");
    fixture.boards[0].part_ids.retain(|id| id != "connector");
    let (probe, root) = mounted_with_document(fixture).await;
    let runtime = &probe.runtime;
    let (entered, release) = support::gate_next_core_reply(runtime);
    support::submit_fixed_command(
        runtime,
        EditCommand {
            base_revision: runtime.model().accepted.unwrap().document.revision,
            transaction_id: "delete-controller".into(),
            phase: EditPhase::Commit,
            target_ids: vec!["mcu-left".into()],
            operation: EditOperation::RemoveParts {
                ids: vec!["mcu-left".into()],
            },
        },
    );
    support::drive_pending(runtime);
    entered.await.unwrap();
    rendered().await;
    let actions = probe.apply.borrow().as_ref().unwrap().clone();
    assert!(
        actions.editable,
        "{:?}",
        plan(&runtime.model().accepted.unwrap().document).diagnostics
    );
    actions.on_apply.call(actions.identity.unwrap());
    rendered().await;
    assert!(!probe.apply.borrow().as_ref().unwrap().editable);
    assert!(probe.mode.borrow().as_ref().unwrap().editable);
    release.send(()).unwrap();
    settle(runtime).await;
    assert!(
        !runtime
            .model()
            .accepted
            .unwrap()
            .document
            .parts
            .iter()
            .any(|part| part.id == "mcu-left")
    );
    assert!(
        matches!(&probe.apply.borrow().as_ref().unwrap().feedback.as_ref().unwrap().state,
        BoardWiringApplyFeedback::Failed(message) if message.contains("deleted"))
    );
    runtime.submit(Event::Undo {
        operation_id: runtime.operation(),
    });
    settle(runtime).await;
    assert!(
        runtime
            .model()
            .accepted
            .unwrap()
            .document
            .parts
            .iter()
            .any(|part| part.id == "mcu-left")
    );
    root.remove();
}

#[wasm_bindgen_test]
async fn failed_field_ticket_restores_accepted_value_and_keeps_feedback_across_another_edit() {
    let (probe, root) = mounted().await;
    let runtime = &probe.runtime;
    support::fail_next_core_reply(runtime, "injected wiring refusal");
    let actions = probe.mode.borrow().as_ref().unwrap().clone();
    actions.on_change.call(BoardWiringModeEditRequest {
        identity: actions.identity.unwrap(),
        mode: ElectricalMode::Direct,
    });
    settle(runtime).await;
    assert!(
        runtime
            .model()
            .accepted
            .unwrap()
            .document
            .hardware
            .as_ref()
            .is_none_or(|hardware| hardware.boards.is_empty())
    );
    assert!(
        matches!(&probe.mode.borrow().as_ref().unwrap().feedback.as_ref().unwrap().state,
        BoardWiringModeFeedback::Failed(message) if message == "The wiring mode change could not be applied: injected wiring refusal")
    );
    runtime.submit(Event::ResolveEdit {
        operation_id: runtime.operation(),
        label: "unrelated-name".into(),
        resolver: EditResolver::new(
            "unrelated-name",
            |accepted: &boardstudio_application::AcceptedSnapshot| {
                let mut document = (*accepted.document).clone();
                document.name = "Later name".into();
                Resolution::Submit(EditCommand {
                    base_revision: accepted.document.revision,
                    transaction_id: String::new(),
                    phase: EditPhase::Commit,
                    target_ids: vec![],
                    operation: EditOperation::ReplaceDocument {
                        document: Box::new(document),
                    },
                })
            },
        ),
    });
    settle(runtime).await;
    assert!(matches!(
        &probe
            .mode
            .borrow()
            .as_ref()
            .unwrap()
            .feedback
            .as_ref()
            .unwrap()
            .state,
        BoardWiringModeFeedback::Failed(_)
    ));
    runtime.submit(Event::SelectParts {
        operation_id: runtime.operation(),
        part_ids: vec!["connector".into()],
        range_part_ids: vec![],
        mode: boardstudio_application::SelectionMode::Replace,
    });
    settle(runtime).await;
    assert!(probe.mode.borrow().as_ref().unwrap().feedback.is_none());
    root.remove();
}

#[wasm_bindgen_test]
async fn physical_topology_queued_behind_another_edit_preserves_it_and_undo() {
    let (probe, root) = mounted().await;
    let runtime = &probe.runtime;
    let (entered, release) = support::gate_next_core_reply(runtime);
    runtime.submit(Event::ResolveEdit {
        operation_id: runtime.operation(),
        label: "layout-name".into(),
        resolver: EditResolver::new(
            "layout-name",
            |accepted: &boardstudio_application::AcceptedSnapshot| {
                let mut document = (*accepted.document).clone();
                document.name = "Layout name".into();
                Resolution::Submit(EditCommand {
                    base_revision: accepted.document.revision,
                    transaction_id: String::new(),
                    phase: EditPhase::Commit,
                    target_ids: vec![],
                    operation: EditOperation::ReplaceDocument {
                        document: Box::new(document),
                    },
                })
            },
        ),
    });
    support::drive_pending(runtime);
    entered.await.unwrap();
    rendered().await;
    probe
        .setup
        .borrow()
        .as_ref()
        .unwrap()
        .submit(crate::pcb_physical_setup::PhysicalSetupIntent::ProjectTopology(true));
    rendered().await;
    assert!(probe.setup.borrow().as_ref().unwrap().projection.busy);
    assert!(probe.mode.borrow().as_ref().unwrap().editable);
    release.send(()).unwrap();
    settle(runtime).await;
    let accepted = runtime.model().accepted.unwrap();
    assert_eq!(accepted.document.name, "Layout name");
    assert_eq!(
        accepted.document.hardware.as_ref().unwrap().topology,
        HardwareTopology::Split
    );
    runtime.submit(Event::Undo {
        operation_id: runtime.operation(),
    });
    settle(runtime).await;
    let accepted = runtime.model().accepted.unwrap();
    assert_eq!(accepted.document.name, "Layout name");
    assert!(
        accepted
            .document
            .hardware
            .as_ref()
            .is_none_or(|hardware| hardware.instances.is_empty())
    );
    runtime.submit(Event::Undo {
        operation_id: runtime.operation(),
    });
    settle(runtime).await;
    assert_eq!(runtime.model().accepted.unwrap().document.name, "Mode test");
    root.remove();
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
    let mut encoder = test_definition(
        "encoder",
        PartKind::Utility,
        BTreeMap::from([
            ("A".into(), vec!["A".into()]),
            ("B".into(), vec!["B".into()]),
            ("C".into(), vec!["C".into()]),
        ]),
        None,
        Some("ceoloide/rotary_encoder_ec11_ec12"),
    );
    encoder.generator.as_mut().unwrap().parameters.insert(
        "include_momentary_switch_pads".into(),
        serde_json::json!(false),
    );
    document.definitions.push(encoder);
    document.parts.push(test_part("encoder", "encoder"));
    document.definitions.push(test_definition(
        "connector",
        PartKind::Passive,
        BTreeMap::from([("signal".into(), vec!["1".into()])]),
        None,
        None,
    ));
    document.parts.push(test_part("connector", "connector"));
    document.boards[0]
        .part_ids
        .extend(["encoder".into(), "connector".into()]);
    document.nets.push(Net {
        id: "net".into(),
        name: "Signal".into(),
        pins: vec![],
    });
    document.boards[0].net_ids.push("net".into());
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
