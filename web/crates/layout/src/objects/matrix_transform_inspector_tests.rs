use super::*;
use crate::presentation::objects::{
    ScopedTreeContext, TreeContext, use_workspace_matrix_transform,
};
use crate::runtime::Runtime;
use crate::runtime::project_name_test_support as support;
use boardstudio_application::{Event, SelectionMode};
use boardstudio_core::model::{
    Board, Matrix, MatrixAssembly, MatrixCell, Part, PartDefinition, PartKind, Pose2, ProjectDoc,
    Side, Vec2,
};
use dioxus_web::WebEventExt;
use std::{cell::RefCell, rc::Rc};
use wasm_bindgen::JsCast;
use wasm_bindgen_test::wasm_bindgen_test;
use web_sys::{Event as DomEvent, HtmlInputElement};

wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

#[derive(Clone)]
struct Probe {
    runtime: Rc<Runtime>,
    selected: ScopedTreeContext,
    root_id: &'static str,
    version: Rc<RefCell<Option<Signal<u64>>>>,
    selected_context: Rc<RefCell<Option<Signal<Option<ScopedTreeContext>>>>>,
    adapter: Rc<RefCell<Option<crate::presentation::selection::SelectionAdapter>>>,
    mount: Rc<RefCell<Option<MatrixTransformInspectorMount>>>,
}

#[component]
fn matrix_transform_host() -> Element {
    let probe = use_context::<Probe>();
    let version = use_signal(|| 0u64);
    *probe.version.borrow_mut() = Some(version);
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
    *probe.mount.borrow_mut() = Some(mount.clone());
    let matrix_id = "matrix".to_owned();
    let canvas_adapter = probe.adapter.borrow().as_ref().unwrap().clone();
    let canvas_runtime = probe.runtime.clone();
    let canvas_scope = probe.selected.scope.clone();
    rsx! {
        svg { id: "{probe.root_id}-matrix-canvas",
            for column in 0..3u32 {
                {
                    let target_part_id = format!("matrix/matrix/r0c{column}");
                    let target_matrix = matrix_id.clone();
                    let scope = canvas_scope.clone();
                    let adapter = canvas_adapter.clone();
                    let runtime = canvas_runtime.clone();
                    rsx! {
                        g {
                            "data-part-id": "{target_part_id}",
                            onpointerdown: move |event: PointerEvent| {
                                let Some(pointer) = event.data().try_as_web_event() else { return; };
                                let model = runtime.model();
                                let hit_context = TreeContext::Key {
                                    matrix_id: target_matrix.clone(),
                                    row: 0,
                                    column,
                                };
                                let Some(projection) = crate::presentation::objects::context_for_selection_kind(
                                    &model,
                                    &hit_context,
                                    crate::presentation::objects::LayoutSelectionKind::Key,
                                    None,
                                ) else { return; };
                                let mode = if pointer.shift_key() {
                                    SelectionMode::Range
                                } else if pointer.ctrl_key() || pointer.meta_key() {
                                    SelectionMode::Toggle
                                } else {
                                    SelectionMode::Replace
                                };
                                crate::presentation::selection::submit_matrix_cell_selection(
                                    &runtime,
                                    &adapter,
                                    &scope,
                                    (adapter.generation)(),
                                    crate::presentation::selection::MatrixCellSelection {
                                        matrix_id: target_matrix.clone(),
                                        target_part_id: target_part_id.clone(),
                                        hit_context: &hit_context,
                                        context: projection.context,
                                        mode,
                                    },
                                );
                            },
                            rect { width: "18", height: "18" }
                        }
                    }
                }
            }
        }
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

/// A one-key matrix document. The test opens it through the real Session and Core, which
/// generate the scene the Inspector reads.
fn fixture() -> (ProjectDoc, TreeContext) {
    let mut document = ProjectDoc::empty("matrix-transform-mounted", "Matrix transform fixture");
    document.boards.push(Board {
        id: "board".into(),
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
        board_id: Some("board".into()),
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
            deleted: false,
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
    (
        document,
        TreeContext::Key {
            matrix_id: "matrix".into(),
            row: 0,
            column: 0,
        },
    )
}

fn two_key_fixture(include_standalone: bool) -> (ProjectDoc, TreeContext) {
    let (mut document, selected) = fixture();
    let first_key = "matrix/matrix/r0c0";
    let second_key = "matrix/matrix/r0c1";
    document.boards[0].part_ids[0] = first_key.into();
    document.boards[0].part_ids.push(second_key.into());
    let matrix = &mut document.matrices[0];
    matrix.columns = 2;
    matrix.part_ids[0] = first_key.into();
    matrix.part_ids.push(second_key.into());
    matrix.cells.push(MatrixCell {
        deleted: false,
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
    (document, selected)
}

fn three_key_fixture() -> (ProjectDoc, TreeContext) {
    let (mut document, selected) = two_key_fixture(false);
    let third_key = "matrix/matrix/r0c2";
    document.boards[0].part_ids.push(third_key.into());
    let matrix = &mut document.matrices[0];
    matrix.columns = 3;
    matrix.part_ids.push(third_key.into());
    matrix.cells.push(MatrixCell {
        deleted: false,
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
    (document, selected)
}

/// Open `document` through the real Session and Core, select `selected_ids` the way the
/// canvas does, and mount the transform Inspector over that runtime.
async fn mount_selection_probe(
    root_id: &'static str,
    document: ProjectDoc,
    context: TreeContext,
    selected_ids: &[&str],
) -> (Probe, web_sys::Element) {
    let runtime = support::new_runtime();
    support::open_document(&runtime, document).await;
    runtime.submit(Event::SelectParts {
        operation_id: runtime.operation(),
        part_ids: selected_ids.iter().map(|id| (*id).to_owned()).collect(),
        range_part_ids: Vec::new(),
        mode: SelectionMode::Replace,
    });
    support::run_pending(&runtime).await;
    let scope = runtime.scope().expect("the opened project has a scope");
    let probe = Probe {
        runtime,
        selected: ScopedTreeContext { scope, context },
        root_id,
        version: Rc::default(),
        selected_context: Rc::default(),
        adapter: Rc::default(),
        mount: Rc::default(),
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

fn accepted_cell(probe: &Probe, column: u32) -> boardstudio_core::model::MatrixCell {
    probe
        .runtime
        .model()
        .accepted
        .expect("an accepted document")
        .document
        .matrices
        .iter()
        .find(|matrix| matrix.id == "matrix")
        .and_then(|matrix| {
            matrix
                .cells
                .iter()
                .find(|cell| cell.row == 0 && cell.column == column)
                .cloned()
        })
        .expect("the matrix cell exists")
}

#[wasm_bindgen_test]
async fn additive_canvas_selection_keeps_key_properties_for_same_matrix_keys() {
    let (document, context) = two_key_fixture(false);
    let (probe, root) = mount_selection_probe(
        "matrix-transform-multiselect-mounted-test",
        document,
        context,
        &["matrix/matrix/r0c0"],
    )
    .await;
    let selected = probe.selected.clone();
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
    let enabled = document
        .query_selector(
            "#matrix-transform-multiselect-mounted-test input[aria-label='Key enabled']",
        )
        .unwrap()
        .unwrap()
        .dyn_into::<HtmlInputElement>()
        .unwrap();
    enabled.set_checked(false);
    let event = web_sys::EventInit::new();
    event.set_bubbles(true);
    enabled
        .dispatch_event(&web_sys::Event::new_with_event_init_dict("change", &event).unwrap())
        .unwrap();
    settle().await;
    support::run_pending(&probe.runtime).await;
    settle().await;
    let accepted = probe.runtime.model().accepted.unwrap();
    assert!(
        accepted.document.matrices[0]
            .cells
            .iter()
            .all(|cell| !cell.enabled),
        "disabling a multi-key selection disables every selected key"
    );
    let mut version = probe.version.borrow().expect("Editor version signal");
    let next = version.peek().saturating_add(1);
    version.set(next);
    settle().await;
    let enabled = root
        .query_selector("input[aria-label='Key enabled']")
        .unwrap()
        .unwrap()
        .dyn_into::<HtmlInputElement>()
        .unwrap();
    enabled.set_checked(true);
    enabled
        .dispatch_event(&web_sys::Event::new_with_event_init_dict("change", &event).unwrap())
        .unwrap();
    support::run_pending(&probe.runtime).await;
    settle().await;
    assert!(
        probe.runtime.model().accepted.unwrap().document.matrices[0]
            .cells
            .iter()
            .all(|cell| cell.enabled),
        "the same selection re-enables every disabled key"
    );
    probe.runtime.submit(Event::Undo {
        operation_id: probe.runtime.operation(),
    });
    support::run_pending(&probe.runtime).await;
    settle().await;
    assert!(
        probe.runtime.model().accepted.unwrap().document.matrices[0]
            .cells
            .iter()
            .all(|cell| !cell.enabled),
        "one Undo reverses the entire bulk enable"
    );
    probe.runtime.submit(Event::Redo {
        operation_id: probe.runtime.operation(),
    });
    support::run_pending(&probe.runtime).await;
    settle().await;
    assert!(
        probe.runtime.model().accepted.unwrap().document.matrices[0]
            .cells
            .iter()
            .all(|cell| cell.enabled),
        "one Redo restores the entire bulk enable"
    );
    root.remove();
}

#[wasm_bindgen_test]
async fn additive_mixed_component_selection_keeps_first_part_group_context() {
    let (document, context) = two_key_fixture(true);
    let (probe, root) = mount_selection_probe(
        "matrix-transform-multiselect-mounted-test",
        document,
        context,
        &["matrix/matrix/r0c0"],
    )
    .await;
    let selected = probe.selected.clone();
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
    let (document, context) = three_key_fixture();
    let (probe, root) = mount_selection_probe(
        "matrix-transform-multiselect-mounted-test",
        document,
        context,
        &["matrix/matrix/r0c0"],
    )
    .await;
    settle().await;
    let ctrl_click = |column: u32| {
        let document = web_sys::window().unwrap().document().unwrap();
        let target = document
            .query_selector(&format!(
                "#matrix-transform-multiselect-mounted-test-matrix-canvas g[data-part-id='matrix/matrix/r0c{column}']"
            ))
            .unwrap()
            .expect("mounted matrix canvas contains the key hit target");
        let constructor = js_sys::Reflect::get(&js_sys::global(), &"PointerEvent".into())
            .unwrap()
            .dyn_into::<js_sys::Function>()
            .unwrap();
        let arguments = js_sys::Array::new();
        arguments.push(&"pointerdown".into());
        let init = js_sys::Object::new();
        js_sys::Reflect::set(init.as_ref(), &"bubbles".into(), &true.into()).unwrap();
        js_sys::Reflect::set(init.as_ref(), &"ctrlKey".into(), &true.into()).unwrap();
        arguments.push(init.as_ref());
        let pointer = js_sys::Reflect::construct(&constructor, &arguments)
            .unwrap()
            .dyn_into::<web_sys::PointerEvent>()
            .unwrap();
        target.dispatch_event(&pointer).unwrap();
    };

    ctrl_click(1);
    settle().await;
    ctrl_click(2);
    settle().await;

    // Removing C leaves A and B selected; C is still a live key, so the Inspector
    // must be re-anchored to a member that remains selected.
    ctrl_click(2);
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
        "removing C must move the key Inspector to a remaining selected member"
    );

    // Removing B leaves A. Keep the key-specific Inspector and verify its edit
    // command changes A's cell rather than the deselected B cell.
    ctrl_click(1);
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
    support::run_pending(&probe.runtime).await;
    settle().await;
    assert_eq!(
        accepted_cell(&probe, 0).offset.map(|offset| offset.x),
        Some(5.0),
        "the remaining key control edits A"
    );
    assert_eq!(
        accepted_cell(&probe, 1).offset.map(|offset| offset.x),
        Some(6.0),
        "the deselected B cell is unchanged"
    );
    root.remove();
}

async fn settle() {
    gloo_timers::future::TimeoutFuture::new(60).await;
}

#[wasm_bindgen_test]
async fn pending_reset_disables_its_button_and_rejects_a_retained_duplicate() {
    let (mut document, context) = fixture();
    document.matrices[0].cells[0].offset = Some(Vec2 { x: 3.0, y: 0.0 });
    let (probe, root) = mount_selection_probe(
        "matrix-transform-pending-reset-test",
        document,
        context,
        &["key-part"],
    )
    .await;
    settle().await;
    let retained = probe.mount.borrow().as_ref().unwrap().clone();
    let projection = retained.projection.as_ref().unwrap();
    let button = root.query_selector_all("button").unwrap();
    let button = (0..button.length())
        .filter_map(|index| button.item(index))
        .find(|node| node.text_content().as_deref() == Some("Reset local transform"))
        .expect("the reset action is mounted")
        .dyn_into::<web_sys::HtmlElement>()
        .unwrap();
    let input = root
        .query_selector("input[aria-label='Local X']")
        .unwrap()
        .expect("the key offset field is mounted")
        .dyn_into::<HtmlInputElement>()
        .unwrap();
    let (entered, release) = support::gate_next_core_reply(&probe.runtime);
    button.click();
    support::drive_pending(&probe.runtime);
    entered.await.expect("the reset reaches Core");
    settle().await;
    let disabled_while_pending = button.has_attribute("disabled");
    assert!(
        !input.disabled(),
        "field edits stay available while an action is pending"
    );

    input.set_value("5");
    let event = DomEvent::new("input").unwrap();
    event.init_event_with_bubbles("input", true);
    input.dispatch_event(&event).unwrap();
    let enter = web_sys::KeyboardEventInit::new();
    enter.set_bubbles(true);
    enter.set_key("Enter");
    input
        .dispatch_event(
            &web_sys::KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &enter).unwrap(),
        )
        .unwrap();
    let mut sequence = retained.request_sequence;
    let request_id = sequence().checked_add(1).unwrap();
    sequence.set(request_id);
    retained.on_edit.call(MatrixTransformRequest {
        owner: projection.owner.clone(),
        request_id,
        snapshot_token: projection.snapshot_token,
        revision: projection.revision,
        field: MatrixTransformField::KeyTransformReset,
        baseline: MatrixTransformValue::CellTransform {
            offset: Vec2 { x: 3.0, y: 0.0 },
            rotation: 0.0,
        },
        value: MatrixTransformValue::CellTransform {
            offset: Vec2::default(),
            rotation: 0.0,
        },
        splay_affect: (retained.splay_affect)(),
        draft: None,
        failure: None,
        submitted_text: None,
        one_shot: true,
    });
    release.send(()).expect("release the held reset");
    // The gated task runs through drive_pending, so run_pending alone does not await it.
    // Refresh this host's version only after the real Session has finished both edits.
    for _ in 0..100 {
        support::run_pending(&probe.runtime).await;
        if matches!(
            probe.runtime.model().lifecycle,
            boardstudio_application::Lifecycle::Ready
        ) {
            break;
        }
        settle().await;
    }
    assert!(
        matches!(
            probe.runtime.model().lifecycle,
            boardstudio_application::Lifecycle::Ready
        ),
        "the reset and queued field edit finish through the real Session"
    );
    let mut version = probe.version.borrow().expect("Editor version signal");
    let next = version.peek().saturating_add(1);
    version.set(next);
    settle().await;

    assert!(
        disabled_while_pending,
        "the reset button disables while its action is pending"
    );
    assert_eq!(
        accepted_cell(&probe, 0).offset,
        Some(Vec2 { x: 5.0, y: 0.0 }),
        "a retained duplicate reset must not undo the queued field edit"
    );
    let live_buttons = root.query_selector_all("button").unwrap();
    let live_button = (0..live_buttons.length())
        .filter_map(|index| live_buttons.item(index))
        .find(|node| node.text_content().as_deref() == Some("Reset local transform"))
        .expect("the live reset action remains mounted")
        .dyn_into::<web_sys::HtmlElement>()
        .unwrap();
    assert!(
        !live_button.has_attribute("disabled"),
        "the live reset button is available after settlement; captured connected: {}, lifecycle: {:?}",
        button.is_connected(),
        probe.runtime.model().lifecycle,
    );
    assert!(root.query_selector("[role='alert']").unwrap().is_none());
    root.remove();
}

#[wasm_bindgen_test]
async fn failed_numeric_edit_keeps_a_newer_typed_draft() {
    let (document, context) = fixture();
    let (probe, root) = mount_selection_probe(
        "matrix-transform-failed-draft-test",
        document,
        context,
        &["key-part"],
    )
    .await;
    settle().await;

    let input = root
        .query_selector("input[aria-label='Local X']")
        .unwrap()
        .expect("the key offset field is mounted")
        .dyn_into::<HtmlInputElement>()
        .unwrap();
    support::fail_next_core_reply(&probe.runtime, "injected transform failure");

    input.set_value("5.00");
    let first_input = DomEvent::new("input").unwrap();
    first_input.init_event_with_bubbles("input", true);
    input.dispatch_event(&first_input).unwrap();
    let enter = web_sys::KeyboardEventInit::new();
    enter.set_bubbles(true);
    enter.set_key("Enter");
    input
        .dispatch_event(
            &web_sys::KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &enter).unwrap(),
        )
        .unwrap();

    // Type again before the failed Core reply is observed by the mounted panel.
    input.set_value("7.00");
    let newer_input = DomEvent::new("input").unwrap();
    newer_input.init_event_with_bubbles("input", true);
    input.dispatch_event(&newer_input).unwrap();
    support::run_pending(&probe.runtime).await;
    let mut version = probe.version.borrow().expect("Editor version signal");
    let next = version.peek().saturating_add(1);
    version.set(next);
    settle().await;

    assert_eq!(
        input.value(),
        "7.00",
        "failure must not restore older submitted text"
    );
    assert_eq!(
        root.query_selector("[role='alert']")
            .unwrap()
            .and_then(|element| element.text_content()),
        Some("The transform change could not be applied: injected transform failure".into())
    );
    root.remove();
}

#[wasm_bindgen_test]
async fn failed_numeric_edit_keeps_its_inline_error_after_restoring_the_draft() {
    let (document, context) = fixture();
    let (probe, root) = mount_selection_probe(
        "matrix-transform-restored-failure-test",
        document,
        context,
        &["key-part"],
    )
    .await;
    settle().await;

    let input = root
        .query_selector("input[aria-label='Local X']")
        .unwrap()
        .expect("the key offset field is mounted")
        .dyn_into::<HtmlInputElement>()
        .unwrap();
    support::fail_next_core_reply(&probe.runtime, "injected transform failure");
    input.set_value("5.00");
    let event = DomEvent::new("input").unwrap();
    event.init_event_with_bubbles("input", true);
    input.dispatch_event(&event).unwrap();
    let enter = web_sys::KeyboardEventInit::new();
    enter.set_bubbles(true);
    enter.set_key("Enter");
    input
        .dispatch_event(
            &web_sys::KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &enter).unwrap(),
        )
        .unwrap();

    support::run_pending(&probe.runtime).await;
    let mut version = probe.version.borrow().expect("Editor version signal");
    let next = version.peek().saturating_add(1);
    version.set(next);
    settle().await;
    assert_eq!(
        input.value(),
        "0",
        "the unchanged failed draft restores accepted text"
    );
    assert_eq!(
        root.query_selector("[role='alert']")
            .unwrap()
            .and_then(|element| element.text_content()),
        Some("The transform change could not be applied: injected transform failure".into()),
        "restoring the accepted projection must not clear the helper-owned failure"
    );
    root.remove();
}

#[wasm_bindgen_test]
async fn hidden_transform_inspector_retires_a_held_failure_before_remount() {
    let (document, context) = fixture();
    let (probe, root) = mount_selection_probe(
        "matrix-transform-hidden-failure-test",
        document,
        context,
        &["key-part"],
    )
    .await;
    settle().await;

    let input = root
        .query_selector("input[aria-label='Local X']")
        .unwrap()
        .expect("the key offset field is mounted")
        .dyn_into::<HtmlInputElement>()
        .unwrap();
    support::fail_next_core_reply(&probe.runtime, "held transform failure");
    input.set_value("5");
    let event = DomEvent::new("input").unwrap();
    event.init_event_with_bubbles("input", true);
    input.dispatch_event(&event).unwrap();
    let enter = web_sys::KeyboardEventInit::new();
    enter.set_bubbles(true);
    enter.set_key("Enter");
    input
        .dispatch_event(
            &web_sys::KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &enter).unwrap(),
        )
        .unwrap();

    // Hide the actual inspector while its Core result is still held. The controller's
    // inspector-mounted marker must retire the request even though the selected key stays.
    let mut selected_signal = probe.selected_context.borrow().as_ref().unwrap().clone();
    selected_signal.set(None);
    settle().await;
    support::run_pending(&probe.runtime).await;
    settle().await;
    selected_signal.set(Some(probe.selected.clone()));
    settle().await;

    assert!(
        root.query_selector("[role='alert']").unwrap().is_none(),
        "a held failure from the hidden owner must not surface after remount"
    );
    root.remove();
}

#[wasm_bindgen_test]
async fn mounted_attached_component_choices_are_current_item_scoped_and_emit_the_selected_replacement()
 {
    let (document, context) = fixture();
    let (probe, root) = mount_selection_probe(
        "matrix-transform-inspector-mounted-test",
        document,
        context,
        &["key-part"],
    )
    .await;
    let document = web_sys::window().unwrap().document().unwrap();
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
    support::run_pending(&probe.runtime).await;
    settle().await;
    assert!(
        accepted_cell(&probe, 0)
            .assemblies
            .iter()
            .any(|assembly| assembly.id == "led"
                && assembly.definition_id == "replacement-definition"),
        "the accepted key carries the selected replacement"
    );
    root.remove();
}

#[wasm_bindgen_test]
async fn deleting_selected_keys_removes_cells_and_assemblies_and_undo_restores_them() {
    let (document, context) = two_key_fixture(false);
    let (probe, root) = mount_selection_probe(
        "matrix-transform-delete-selected-test",
        document,
        context,
        &["matrix/matrix/r0c0", "matrix/matrix/r0c1"],
    )
    .await;
    settle().await;
    let delete = root.query_selector_all("button").unwrap();
    let delete = (0..delete.length())
        .filter_map(|index| delete.item(index))
        .find(|node| node.text_content().as_deref() == Some("Delete selected keys"))
        .expect("key properties provide actual deletion distinct from the enabled toggle")
        .dyn_into::<web_sys::HtmlElement>()
        .unwrap();
    delete.click();
    support::run_pending(&probe.runtime).await;
    settle().await;
    let model = probe.runtime.model();
    let scene = &model.accepted.as_ref().unwrap().scene;
    assert!(
        scene.matrix_scenes[0].cells.is_empty(),
        "deleted cells have no snapping/ghost positions"
    );
    assert!(
        model.accepted.as_ref().unwrap().document.parts.is_empty(),
        "switches and attached assemblies are removed together"
    );
    probe.runtime.submit(Event::Undo {
        operation_id: probe.runtime.operation(),
    });
    support::run_pending(&probe.runtime).await;
    settle().await;
    assert_eq!(
        probe.runtime.model().accepted.unwrap().scene.matrix_scenes[0]
            .cells
            .len(),
        2,
        "one Undo restores the complete selection"
    );
    root.remove();
}

#[wasm_bindgen_test]
async fn queued_key_edit_retires_after_column_deletion_instead_of_editing_reindexed_survivor() {
    let (document, context) = two_key_fixture(false);
    let (probe, root) = mount_selection_probe(
        "matrix-transform-queued-delete-test",
        document,
        context,
        &["matrix/matrix/r0c0"],
    )
    .await;
    settle().await;
    let (entered, release) = support::gate_next_core_reply(&probe.runtime);
    let removal = boardstudio_web_runtime::edit_ticket::EditTicket::begin(
        &probe.runtime,
        "delete-column-test",
        None,
        boardstudio_application::EditResolver::new("delete-column-test", |_| {
            boardstudio_application::Resolution::submit(
                vec!["matrix".into()],
                boardstudio_core::model::EditOperation::RemoveMatrixColumn {
                    matrix_id: "matrix".into(),
                    column: 0,
                },
            )
        }),
    );
    support::drive_pending(&probe.runtime);
    entered
        .await
        .expect("deletion reaches Core while old key remains accepted");
    let input = root
        .query_selector("input[aria-label='Local X']")
        .unwrap()
        .unwrap()
        .dyn_into::<HtmlInputElement>()
        .unwrap();
    input.set_value("77");
    let event = web_sys::EventInit::new();
    event.set_bubbles(true);
    input
        .dispatch_event(&web_sys::Event::new_with_event_init_dict("input", &event).unwrap())
        .unwrap();
    let enter = web_sys::KeyboardEventInit::new();
    enter.set_bubbles(true);
    enter.set_key("Enter");
    input
        .dispatch_event(
            &web_sys::KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &enter).unwrap(),
        )
        .unwrap();
    release.send(()).expect("release removal");
    support::run_pending(&probe.runtime).await;
    settle().await;
    assert!(matches!(
        removal.settlement(true),
        boardstudio_web_runtime::edit_ticket::Settlement::Landed { .. }
    ));
    assert_eq!(
        accepted_cell(&probe, 0).offset.unwrap().x,
        6.0,
        "the queued deleted-key edit cannot alter the survivor reindexed into its coordinate"
    );
    root.remove();
}

#[wasm_bindgen_test]
async fn mixed_enabled_selection_displays_unchecked_and_enables_every_selected_cell() {
    let (document, context) = two_key_fixture(false);
    let (probe, root) = mount_selection_probe(
        "matrix-transform-mixed-enabled-test",
        document,
        context,
        &["matrix/matrix/r0c0", "matrix/matrix/r0c1"],
    )
    .await;
    settle().await;
    let disabled = boardstudio_web_runtime::edit_ticket::EditTicket::begin(
        &probe.runtime,
        "disable-one-test",
        None,
        boardstudio_application::EditResolver::new("disable-one-test", |accepted| {
            let mut matrix = accepted.document.matrices[0].clone();
            matrix.cells[1].enabled = false;
            boardstudio_application::Resolution::submit(
                vec![matrix.id.clone()],
                boardstudio_core::model::EditOperation::SetMatrix {
                    matrix,
                    definitions: None,
                },
            )
        }),
    );
    support::run_pending(&probe.runtime).await;
    assert!(matches!(
        disabled.settlement(true),
        boardstudio_web_runtime::edit_ticket::Settlement::Landed { .. }
    ));
    let mut version = probe.version.borrow().expect("Editor version signal");
    let next = version.peek().saturating_add(1);
    version.set(next);
    settle().await;
    let enabled = root
        .query_selector("input[aria-label='Key enabled']")
        .unwrap()
        .unwrap()
        .dyn_into::<HtmlInputElement>()
        .unwrap();
    assert!(
        !enabled.checked(),
        "mixed selection does not imply every key is enabled"
    );
    enabled.set_checked(true);
    let event = web_sys::EventInit::new();
    event.set_bubbles(true);
    enabled
        .dispatch_event(&web_sys::Event::new_with_event_init_dict("change", &event).unwrap())
        .unwrap();
    support::run_pending(&probe.runtime).await;
    settle().await;
    assert!(
        probe.runtime.model().accepted.unwrap().document.matrices[0]
            .cells
            .iter()
            .all(|cell| cell.enabled)
    );
    root.remove();
}

#[wasm_bindgen_test]
async fn row_and_column_delete_controls_submit_structural_removal() {
    for (context, label, remaining_columns) in [
        (
            TreeContext::Column {
                matrix_id: "matrix".into(),
                column: 1,
            },
            "Delete column",
            Some(1),
        ),
        (
            TreeContext::Row {
                matrix_id: "matrix".into(),
                row: 0,
            },
            "Delete row",
            None,
        ),
    ] {
        let (document, _) = two_key_fixture(false);
        let (probe, root) =
            mount_selection_probe("matrix-transform-axis-delete-test", document, context, &[])
                .await;
        settle().await;
        let buttons = root.query_selector_all("button").unwrap();
        let button = (0..buttons.length())
            .filter_map(|index| buttons.item(index))
            .find(|node| node.text_content().as_deref() == Some(label))
            .expect("axis deletion control")
            .dyn_into::<web_sys::HtmlElement>()
            .unwrap();
        button.click();
        button.click();
        support::run_pending(&probe.runtime).await;
        settle().await;
        let accepted = probe.runtime.model().accepted.unwrap();
        if let Some(columns) = remaining_columns {
            assert_eq!(
                accepted.document.matrices[0].columns, columns,
                "one action removes one column despite repeated click"
            );
            assert_eq!(accepted.scene.matrix_scenes[0].cells.len(), 1);
        } else {
            assert!(
                accepted.document.matrices.is_empty(),
                "removing the final row removes its matrix"
            );
            assert!(accepted.document.parts.is_empty());
        }
        root.remove();
    }
}

#[wasm_bindgen_test]
async fn explicitly_reselecting_same_disabled_key_discards_previous_bulk_selection() {
    let (document, _) = two_key_fixture(false);
    let context = TreeContext::Key {
        matrix_id: "matrix".into(),
        row: 0,
        column: 1,
    };
    let (probe, root) = mount_selection_probe(
        "matrix-transform-disabled-reselect-test",
        document,
        context.clone(),
        &["matrix/matrix/r0c0", "matrix/matrix/r0c1"],
    )
    .await;
    settle().await;
    let event = web_sys::EventInit::new();
    event.set_bubbles(true);
    let enabled = root
        .query_selector("input[aria-label='Key enabled']")
        .unwrap()
        .unwrap()
        .dyn_into::<HtmlInputElement>()
        .unwrap();
    enabled.set_checked(false);
    enabled
        .dispatch_event(&web_sys::Event::new_with_event_init_dict("change", &event).unwrap())
        .unwrap();
    support::run_pending(&probe.runtime).await;
    let mut version = probe.version.borrow().expect("Editor version signal");
    let next = version.peek().saturating_add(1);
    version.set(next);
    settle().await;
    let adapter = probe.adapter.borrow().as_ref().unwrap().clone();
    crate::presentation::selection::submit_context(
        &probe.runtime,
        &adapter,
        crate::presentation::selection::ContextRequest {
            scope: probe.selected.scope.clone(),
            context,
            mode: SelectionMode::Replace,
        },
    );
    settle().await;
    let enabled = root
        .query_selector("input[aria-label='Key enabled']")
        .unwrap()
        .unwrap()
        .dyn_into::<HtmlInputElement>()
        .unwrap();
    enabled.set_checked(true);
    enabled
        .dispatch_event(&web_sys::Event::new_with_event_init_dict("change", &event).unwrap())
        .unwrap();
    support::run_pending(&probe.runtime).await;
    settle().await;
    assert!(
        !accepted_cell(&probe, 0).enabled,
        "reselected single key cannot revive the old bulk target"
    );
    assert!(accepted_cell(&probe, 1).enabled);
    root.remove();
}
