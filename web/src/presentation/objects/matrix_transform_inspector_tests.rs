use super::*;
use crate::presentation::objects::{
    ScopedTreeContext, TreeContext, use_workspace_matrix_transform,
};
use crate::runtime::Runtime;
use boardstudio_application::{
    AcceptedSnapshot, Durability, Event, Lifecycle, ReadModel, Scope, SelectionMode, SessionEpoch,
    SnapshotToken,
};
use boardstudio_core::model::{
    Board, EditOperation, Matrix, MatrixAssembly, MatrixCell, MatrixScene, MatrixSceneCell, Part,
    PartDefinition, PartKind, Pose2, ProjectDoc, Readiness, SceneDelta, Side, Vec2,
};
use std::{cell::RefCell, rc::Rc, sync::Arc};
use wasm_bindgen::JsCast;
use wasm_bindgen_test::wasm_bindgen_test;

wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

#[derive(Clone)]
struct Probe {
    runtime: Rc<Runtime>,
    selected: ScopedTreeContext,
    root_id: &'static str,
    selected_context: Rc<RefCell<Option<Signal<Option<ScopedTreeContext>>>>>,
    adapter: Rc<RefCell<Option<crate::presentation::selection::SelectionAdapter>>>,
}

#[component]
fn matrix_transform_host() -> Element {
    let probe = use_context::<Probe>();
    let version = use_signal(|| 0u64);
    let selected_context = use_signal(|| Some(probe.selected.clone()));
    *probe.selected_context.borrow_mut() = Some(selected_context);
    let workspace = use_signal(|| "Layout");
    let scope_generation = use_signal(|| 1u64);
    let anchor_scope = use_signal(|| None);
    *probe.adapter.borrow_mut() = Some(crate::presentation::selection::SelectionAdapter::new(
        selected_context,
        anchor_scope,
        scope_generation,
    ));
    let splay_affect = use_signal(|| boardstudio_core::model::MatrixSplayAffect::Column);
    let mount = use_workspace_matrix_transform(
        probe.runtime.clone(),
        version,
        selected_context,
        workspace,
        scope_generation,
        splay_affect,
        "Layout",
    );
    rsx! {
        MatrixTransformInspector { mount, on_pick_splay_origin: |_| {} }
    }
}

fn definition(id: &str, name: &str) -> PartDefinition {
    PartDefinition {
        hardware_profile: None,
        input_profile: None,
        id: id.into(),
        name: name.into(),
        kind: PartKind::Controller,
        keycap: None,
        envelope_source: None,
        kicad_source: None,
        terminals: Default::default(),
        matrix_terminals: None,
        envelope_notice: None,
        courtyard: vec![],
        pads: vec![],
        models: None,
        generator: None,
        mechanical_profile: None,
    }
}

fn fixture() -> (ReadModel, ScopedTreeContext) {
    let scope = Scope {
        session_epoch: SessionEpoch(81),
        document_id: "matrix-transform-mounted".into(),
        board_id: "board".into(),
        instance_id: None,
    };
    let mut document = ProjectDoc::empty(&scope.document_id, "Matrix transform fixture");
    document.revision = 12;
    document.boards.push(Board {
        id: scope.board_id.clone(),
        name: "Board".into(),
        outline_ids: vec![],
        part_ids: vec!["key-part".into()],
        net_ids: vec![],
        thickness: 1.6,
        traces: vec![],
        vias: vec![],
    });
    document.definitions = vec![
        definition("key-definition", "Key"),
        definition("assembly-current/definition/led", "Current LED snapshot"),
        definition("assembly-other/definition/led", "Unrelated LED snapshot"),
        definition("replacement-definition", "Replacement LED"),
    ];
    document.parts.push(Part {
        keycap: None,
        outline: None,
        id: "key-part".into(),
        definition_id: "key-definition".into(),
        reference: "K1".into(),
        pose: Pose2 {
            at: Vec2::default(),
            rotation: 0.0,
        },
        side: Side::Front,
        locked: Some(false),
        properties: None,
        generator_parameters: None,
    });
    document.matrices.push(Matrix {
        id: "matrix".into(),
        name: Some("Keys".into()),
        rows: 1,
        columns: 1,
        pitch: Vec2 { x: 19.0, y: 19.0 },
        origin: Vec2::default(),
        definition_id: "key-definition".into(),
        part_ids: vec!["key-part".into()],
        board_id: Some(scope.board_id.clone()),
        mirror: None,
        rotation: None,
        edge_gap: None,
        diode_direction: None,
        row_offsets: vec![],
        column_offsets: vec![],
        column_staggers: vec![],
        column_splays: vec![],
        column_origins: vec![],
        cells: vec![MatrixCell {
            row: 0,
            column: 0,
            enabled: true,
            definition_id: Some("key-definition".into()),
            variant: None,
            offset: None,
            rotation: None,
            assemblies: vec![MatrixAssembly {
                id: "led".into(),
                definition_id: "assembly-current/definition/led".into(),
                offset: Vec2::default(),
                rotation: None,
                side: None,
            }],
            assemblies_local: None,
        }],
    });
    let scene = SceneDelta {
        module_scenes: vec![],
        revision: 12,
        transaction_id: "matrix-transform-mounted-fixture".into(),
        changed_ids: vec![],
        transforms: vec![],
        matrix_scenes: vec![MatrixScene {
            matrix_id: "matrix".into(),
            cells: vec![MatrixSceneCell {
                row: 0,
                column: 0,
                enabled: true,
                member_id: Some("key-part".into()),
                pose: Pose2 {
                    at: Vec2::default(),
                    rotation: 0.0,
                },
            }],
            columns: vec![],
        }],
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
    };
    let snapshot = AcceptedSnapshot {
        token: SnapshotToken(13),
        session_epoch: scope.session_epoch,
        document: Arc::new(document),
        scene: Arc::new(scene),
    };
    let model = ReadModel {
        lifecycle: Lifecycle::Ready,
        durability: Durability::Saved { revision: 12 },
        accepted: Some(snapshot),
        active_board_id: scope.board_id.clone(),
        ..ReadModel::default()
    };
    let selected = ScopedTreeContext {
        scope,
        context: TreeContext::Key {
            matrix_id: "matrix".into(),
            row: 0,
            column: 0,
        },
    };
    (model, selected)
}

fn two_key_fixture(include_standalone: bool) -> (ReadModel, ScopedTreeContext) {
    let (mut model, mut selected) = fixture();
    let snapshot = model.accepted.as_mut().expect("fixture snapshot");
    let document = Arc::make_mut(&mut snapshot.document);
    let first_key = "matrix/matrix/r0c0";
    let second_key = "matrix/matrix/r0c1";
    document.boards[0].part_ids[0] = first_key.into();
    document.boards[0].part_ids.push(second_key.into());
    let matrix = &mut document.matrices[0];
    matrix.columns = 2;
    matrix.part_ids[0] = first_key.into();
    matrix.part_ids.push(second_key.into());
    matrix.cells.push(MatrixCell {
        row: 0,
        column: 1,
        enabled: true,
        definition_id: Some("key-definition".into()),
        variant: None,
        offset: Some(Vec2 { x: 6.0, y: 7.0 }),
        rotation: None,
        assemblies: vec![],
        assemblies_local: Some(true),
    });
    document.parts.push(Part {
        keycap: None,
        outline: None,
        id: second_key.into(),
        definition_id: "key-definition".into(),
        reference: "K2".into(),
        pose: Pose2 {
            at: Vec2 { x: 19.0, y: 0.0 },
            rotation: 0.0,
        },
        side: Side::Front,
        locked: Some(false),
        properties: None,
        generator_parameters: None,
    });
    document.parts[0].id = first_key.into();
    let scene = Arc::make_mut(&mut snapshot.scene);
    scene.matrix_scenes[0].cells[0].member_id = Some(first_key.into());
    scene.matrix_scenes[0].cells.push(MatrixSceneCell {
        row: 0,
        column: 1,
        enabled: true,
        member_id: Some(second_key.into()),
        pose: Pose2 {
            at: Vec2 { x: 19.0, y: 0.0 },
            rotation: 0.0,
        },
    });
    model.selected_part_ids = vec![first_key.into()];
    model.selection_anchor_id = Some(first_key.into());
    selected.context = TreeContext::Key {
        matrix_id: "matrix".into(),
        row: 0,
        column: 0,
    };
    if include_standalone {
        document.boards[0].part_ids.push("standalone-part".into());
        document.parts.push(Part {
            keycap: None,
            outline: None,
            id: "standalone-part".into(),
            definition_id: "key-definition".into(),
            reference: "U1".into(),
            pose: Pose2 {
                at: Vec2 { x: 40.0, y: 0.0 },
                rotation: 0.0,
            },
            side: Side::Front,
            locked: Some(false),
            properties: None,
            generator_parameters: None,
        });
    }
    (model, selected)
}

fn three_key_fixture() -> (ReadModel, ScopedTreeContext) {
    let (mut model, selected) = two_key_fixture(false);
    let snapshot = model.accepted.as_mut().expect("fixture snapshot");
    let document = Arc::make_mut(&mut snapshot.document);
    let third_key = "matrix/matrix/r0c2";
    document.boards[0].part_ids.push(third_key.into());
    let matrix = &mut document.matrices[0];
    matrix.columns = 3;
    matrix.part_ids.push(third_key.into());
    matrix.cells.push(MatrixCell {
        row: 0,
        column: 2,
        enabled: true,
        definition_id: Some("key-definition".into()),
        variant: None,
        offset: Some(Vec2 { x: 12.0, y: 0.0 }),
        rotation: None,
        assemblies: vec![],
        assemblies_local: Some(true),
    });
    document.parts.push(Part {
        keycap: None,
        outline: None,
        id: third_key.into(),
        definition_id: "key-definition".into(),
        reference: "K3".into(),
        pose: Pose2 {
            at: Vec2 { x: 38.0, y: 0.0 },
            rotation: 0.0,
        },
        side: Side::Front,
        locked: Some(false),
        properties: None,
        generator_parameters: None,
    });
    Arc::make_mut(&mut snapshot.scene).matrix_scenes[0]
        .cells
        .push(MatrixSceneCell {
            row: 0,
            column: 2,
            enabled: true,
            member_id: Some(third_key.into()),
            pose: Pose2 {
                at: Vec2 { x: 38.0, y: 0.0 },
                rotation: 0.0,
            },
        });
    (model, selected)
}

fn mount_selection_probe(
    model: ReadModel,
    selected: ScopedTreeContext,
) -> (Probe, web_sys::Element) {
    let runtime = Runtime::new().expect("browser Runtime initializes");
    runtime.set_layout_component_inspector_test_state(model, Some(selected.scope.clone()));
    let probe = Probe {
        runtime,
        selected,
        root_id: "matrix-transform-multiselect-mounted-test",
        selected_context: Rc::default(),
        adapter: Rc::default(),
    };
    let document = web_sys::window().unwrap().document().unwrap();
    let root = document.create_element("div").unwrap();
    root.set_id(probe.root_id);
    document.body().unwrap().append_child(&root).unwrap();
    let dom = VirtualDom::new(matrix_transform_host);
    dom.provide_root_context(probe.clone());
    dioxus_web::launch::launch_virtual_dom(
        dom,
        dioxus_web::Config::new().rootnode(root.clone().into()),
    );
    (probe, root)
}

#[wasm_bindgen_test]
async fn additive_canvas_selection_keeps_key_properties_for_same_matrix_keys() {
    let (model, selected) = two_key_fixture(false);
    let (probe, root) = mount_selection_probe(model, selected.clone());
    settle().await;
    let adapter = probe.adapter.borrow().as_ref().unwrap().clone();
    let hit_context = TreeContext::Key {
        matrix_id: "matrix".into(),
        row: 0,
        column: 1,
    };
    let part_context = TreeContext::Component {
        part_id: Some("matrix/matrix/r0c1".into()),
        matrix_id: None,
        row: None,
        column: None,
        assembly_id: None,
    };
    crate::presentation::selection::submit_matrix_cell_selection(
        &probe.runtime,
        &adapter,
        &selected.scope,
        1,
        crate::presentation::selection::MatrixCellSelection {
            matrix_id: "matrix".into(),
            target_part_id: "matrix/matrix/r0c1".into(),
            hit_context: &hit_context,
            context: part_context,
            mode: SelectionMode::Toggle,
        },
    )
    .expect("live canvas key hit submits additive selection");
    settle().await;

    let selection = *probe.selected_context.borrow().as_ref().unwrap();
    assert!(
        matches!(
            selection.read().as_ref().map(|selected| &selected.context),
            Some(TreeContext::Key {
                matrix_id,
                row: 0,
                column: 1,
            }) if matrix_id == "matrix"
        ),
        "a same-matrix multi-key selection keeps its current key as the Inspector context"
    );
    let document = web_sys::window().unwrap().document().unwrap();
    let local_x = document
        .query_selector("#matrix-transform-multiselect-mounted-test input[aria-label='Local X']")
        .unwrap();
    assert!(
        local_x.is_some(),
        "the mounted key-local Inspector remains visible"
    );
    assert_eq!(
        local_x
            .unwrap()
            .dyn_into::<web_sys::HtmlInputElement>()
            .unwrap()
            .value(),
        "6",
        "the Inspector follows the most recently toggled key"
    );
    assert!(
        document
            .query_selector(
                "#matrix-transform-multiselect-mounted-test select[aria-label='Key Assembly']"
            )
            .unwrap()
            .is_some(),
        "the mounted key properties retain Key Assembly controls"
    );
    root.remove();
}

#[wasm_bindgen_test]
async fn additive_mixed_component_selection_keeps_first_part_group_context() {
    let (model, selected) = two_key_fixture(true);
    let (probe, root) = mount_selection_probe(model, selected.clone());
    settle().await;
    let adapter = probe.adapter.borrow().as_ref().unwrap().clone();
    let part_context = TreeContext::Component {
        part_id: Some("standalone-part".into()),
        matrix_id: None,
        row: None,
        column: None,
        assembly_id: None,
    };
    crate::presentation::selection::submit_canvas_selection(
        &probe.runtime,
        &adapter,
        &selected.scope,
        1,
        part_context,
        SelectionMode::Toggle,
        Vec::new(),
    )
    .expect("live canvas key hit submits additive selection");
    settle().await;

    let selection = *probe.selected_context.borrow().as_ref().unwrap();
    assert!(
        matches!(
            selection.read().as_ref().map(|selected| &selected.context),
            Some(TreeContext::Component { part_id: Some(part_id), .. }) if part_id == "matrix/matrix/r0c0"
        ),
        "mixed matrix-key and standalone selection keeps the first selected part as its group anchor"
    );
    let document = web_sys::window().unwrap().document().unwrap();
    assert!(
        document
            .query_selector(
                "#matrix-transform-multiselect-mounted-test input[aria-label='Local X']"
            )
            .unwrap()
            .is_none(),
        "mixed selection does not show a key-local Inspector"
    );
    root.remove();
}

#[wasm_bindgen_test]
async fn toggle_removal_keeps_the_inspector_and_commands_aimed_at_remaining_keys() {
    let (mut model, selected) = three_key_fixture();
    let (probe, root) = mount_selection_probe(model.clone(), selected.clone());
    settle().await;
    let adapter = probe.adapter.borrow().as_ref().unwrap().clone();
    let submit_toggle = |row: u32, column: u32, context: TreeContext| {
        let hit_context = TreeContext::Key {
            matrix_id: "matrix".into(),
            row,
            column,
        };
        crate::presentation::selection::submit_matrix_cell_selection(
            &probe.runtime,
            &adapter,
            &selected.scope,
            1,
            crate::presentation::selection::MatrixCellSelection {
                matrix_id: "matrix".into(),
                target_part_id: format!("matrix/matrix/r{row}c{column}"),
                hit_context: &hit_context,
                context,
                mode: SelectionMode::Toggle,
            },
        )
        .expect("live matrix key toggles selection");
    };
    let component_context = |column| TreeContext::Component {
        part_id: Some(format!("matrix/matrix/r0c{column}")),
        matrix_id: None,
        row: None,
        column: None,
        assembly_id: None,
    };

    submit_toggle(0, 1, component_context(1));
    model.selected_part_ids = vec!["matrix/matrix/r0c0".into(), "matrix/matrix/r0c1".into()];
    probe
        .runtime
        .set_layout_component_inspector_test_state(model.clone(), Some(selected.scope.clone()));
    submit_toggle(0, 2, component_context(2));
    model.selected_part_ids.push("matrix/matrix/r0c2".into());
    probe
        .runtime
        .set_layout_component_inspector_test_state(model.clone(), Some(selected.scope.clone()));

    // Removing C leaves A and B selected; C is still a live key, so the Inspector
    // must be re-anchored to a member that remains selected.
    submit_toggle(0, 2, component_context(2));
    model.selected_part_ids.pop();
    probe
        .runtime
        .set_layout_component_inspector_test_state(model.clone(), Some(selected.scope.clone()));
    let selected_context = *probe.selected_context.borrow().as_ref().unwrap();
    assert!(
        matches!(
            selected_context
                .read()
                .as_ref()
                .map(|selected| &selected.context),
            Some(TreeContext::Key {
                row: 0,
                column: 0,
                ..
            })
        ),
        "removing C must move the key Inspector to a remaining selected member"
    );

    // Removing B leaves A. Keep the key-specific Inspector and verify its edit
    // command changes A's cell rather than the deselected B cell.
    submit_toggle(0, 1, component_context(1));
    model.selected_part_ids.pop();
    probe
        .runtime
        .set_layout_component_inspector_test_state(model, Some(selected.scope.clone()));
    settle().await;
    let selected_context = *probe.selected_context.borrow().as_ref().unwrap();
    assert!(
        matches!(
            selected_context
                .read()
                .as_ref()
                .map(|selected| &selected.context),
            Some(TreeContext::Key {
                row: 0,
                column: 0,
                ..
            })
        ),
        "removing B must keep the remaining A key Inspector mounted"
    );
    let document = web_sys::window().unwrap().document().unwrap();
    let local_x = document
        .query_selector("#matrix-transform-multiselect-mounted-test input[aria-label='Local X']")
        .unwrap()
        .expect("remaining selected key keeps its local transform field")
        .dyn_into::<web_sys::HtmlInputElement>()
        .unwrap();
    let _ = probe.runtime.take_layout_component_inspector_test_events();
    local_x.set_value("5");
    let input = web_sys::EventInit::new();
    input.set_bubbles(true);
    local_x
        .dispatch_event(&web_sys::Event::new_with_event_init_dict("input", &input).unwrap())
        .unwrap();
    let enter = web_sys::KeyboardEventInit::new();
    enter.set_bubbles(true);
    enter.set_key("Enter");
    local_x
        .dispatch_event(
            &web_sys::KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &enter).unwrap(),
        )
        .unwrap();
    settle().await;
    let events = probe.runtime.take_layout_component_inspector_test_events();
    assert!(
        matches!(
            events.as_slice(),
            [Event::Edit { command, .. }]
                if matches!(&command.operation, EditOperation::SetMatrix { matrix, .. }
                    if matrix.cells.iter().find(|cell| cell.row == 0 && cell.column == 0)
                        .is_some_and(|cell| cell.offset.is_some_and(|offset| offset.x == 5.0))
                        && matrix.cells.iter().find(|cell| cell.row == 0 && cell.column == 1)
                            .is_some_and(|cell| cell.offset.is_some_and(|offset| offset.x == 6.0)))
        ),
        "the remaining key control must target A and leave B unchanged: {events:?}"
    );
    root.remove();
}

async fn settle() {
    gloo_timers::future::TimeoutFuture::new(60).await;
}

#[wasm_bindgen_test]
async fn mounted_attached_component_choices_are_current_item_scoped_and_emit_the_selected_replacement()
 {
    let (model, selected) = fixture();
    let runtime = Runtime::new().expect("browser Runtime initializes");
    runtime.set_layout_component_inspector_test_state(model, Some(selected.scope.clone()));
    let probe = Probe {
        runtime,
        selected,
        root_id: "matrix-transform-inspector-mounted-test",
        selected_context: Rc::default(),
        adapter: Rc::default(),
    };
    let document = web_sys::window().unwrap().document().unwrap();
    let root = document.create_element("div").unwrap();
    root.set_id(probe.root_id);
    document.body().unwrap().append_child(&root).unwrap();
    let mut dom = VirtualDom::new(matrix_transform_host);
    dom.provide_root_context(probe.clone());
    dioxus_web::launch::launch_virtual_dom(
        dom,
        dioxus_web::Config::new().rootnode(root.clone().into()),
    );
    settle().await;

    let selector = document
        .query_selector(&format!(
            "#{} select[aria-label='Replace led']",
            probe.root_id
        ))
        .unwrap()
        .expect("the mounted Inspector renders the attached LED selector");
    assert!(
        selector
            .query_selector("option[value='assembly-current/definition/led']")
            .unwrap()
            .is_some(),
        "the selector must retain this attached item's current snapshot"
    );
    assert!(
        selector
            .query_selector("option[value='replacement-definition']")
            .unwrap()
            .is_some(),
        "the choice list must include the actual replacement definition"
    );
    assert!(
        selector
            .query_selector("option[value='assembly-other/definition/led']")
            .unwrap()
            .is_none(),
        "the choice list must exclude an unrelated assembly snapshot"
    );

    js_sys::Reflect::set(
        selector.as_ref(),
        &"value".into(),
        &"replacement-definition".into(),
    )
    .unwrap();
    let event = web_sys::EventInit::new();
    event.set_bubbles(true);
    selector
        .dispatch_event(&web_sys::Event::new_with_event_init_dict("change", &event).unwrap())
        .unwrap();
    settle().await;
    let events = probe.runtime.take_layout_component_inspector_test_events();
    assert!(
        matches!(
            events.as_slice(),
            [Event::Edit { command, .. }]
                if matches!(&command.operation, EditOperation::SetMatrix { matrix, .. }
                    if matrix.id == "matrix" && matrix.cells.iter().find(|cell| cell.row == 0 && cell.column == 0)
                        .is_some_and(|cell| cell.assemblies.iter().any(|assembly|
                            assembly.id == "led" && assembly.definition_id == "replacement-definition")))
        ),
        "the mounted controller must submit the selected replacement against the current key: {events:?}"
    );
    root.remove();
}
