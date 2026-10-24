use super::*;
use crate::presentation::objects::{
    ScopedTreeContext, TreeContext, use_workspace_matrix_transform,
};
use crate::runtime::Runtime;
use crate::runtime::project_name_test_support as support;
use boardstudio_application::{Event, SelectionMode};
use boardstudio_core::model::{
    Board, Follow, Layout, Matrix, MatrixAnchor, MatrixAssembly, MatrixCell, Part, PartDefinition,
    PartKind, Pose2, ProjectDoc, Side, Vec2,
};
use boardstudio_web_ui_shared::test_wait::wait_until;
use dioxus_web::WebEventExt;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
use wasm_bindgen::JsCast;
use wasm_bindgen_test::wasm_bindgen_test;
use web_sys::{Event as DomEvent, HtmlInputElement};

wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

#[derive(Clone)]
struct Probe {
    canvas_controls: bool,
    runtime: Rc<Runtime>,
    selected: ScopedTreeContext,
    root_id: &'static str,
    version: Rc<RefCell<Option<Signal<u64>>>>,
    selected_context: Rc<RefCell<Option<Signal<Option<ScopedTreeContext>>>>>,
    adapter: Rc<RefCell<Option<crate::presentation::selection::SelectionAdapter>>>,
    mount: Rc<RefCell<Option<MatrixTransformInspectorMount>>>,
    render_count: Rc<Cell<u64>>,
}

#[component]
fn matrix_transform_host() -> Element {
    let probe = use_context::<Probe>();
    probe.render_count.set(probe.render_count.get() + 1);
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
    let mut canvas_tool = use_signal(|| Some(super::super::LayoutTransformTool::Stagger));
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
        if probe.canvas_controls {
            div { id: "canvas-geometry-test",
                super::super::LayoutCanvasControls {
                    mount: mount.clone(),
                    tools: super::super::LayoutTransformMenuMount {
                        properties_available: true, column_available: true, row_available: true,
                        pointer_tools_visible: true, pointer_tools_available: true,
                        active_tool: canvas_tool(), on_selection_kind: EventHandler::new(|_| {}),
                        on_show_properties: EventHandler::new(|_| {}),
                        on_transform_tool: EventHandler::new(move |tool| canvas_tool.set(Some(tool))),
                    },
                }
            }
        }
        div { id: "inspector-geometry-test",
            MatrixTransformInspector { mount, on_pick_splay_origin: |_| {} }
        }
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
        row_layout: vec![],
        column_layout: vec![],
        anchor: None,
        cells: vec![MatrixCell {
            shift: None,
            bend: None,
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
        shift: None,
        bend: None,
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
        shift: None,
        bend: None,
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

/// A second 1-key matrix anchored to the first key, with rotation Follow on or
/// off. The anchor target is the fixture's own first key. Both matrices join
/// one Layout, which anchor validation requires.
fn anchored_matrix_fixture(rotation_followed: bool) -> (ProjectDoc, TreeContext) {
    let (mut document, _) = fixture();
    document.layouts.push(Layout {
        id: "layout".into(),
        name: "Main".into(),
        board_id: "board".into(),
        matrix_ids: vec!["matrix".into(), "matrix-b".into()],
        part_ids: vec![],
        mirror_link: None,
    });
    let key_id = "matrix/matrix-b/r0c0";
    document.boards[0].part_ids.push(key_id.into());
    document.parts.push(Part {
        keycap: None,
        outline: None,
        id: key_id.into(),
        definition_id: "key-definition".into(),
        reference: "K2".into(),
        pose: Pose2 {
            at: Vec2 { x: 60.0, y: 0.0 },
            rotation: 0.0,
        },
        side: Side::Front,
        locked: Some(false),
        properties: None,
        generator_parameters: None,
    });
    document.matrices.push(Matrix {
        id: "matrix-b".into(),
        name: Some("Anchored".into()),
        rows: 1,
        columns: 1,
        pitch: Vec2 { x: 19.0, y: 19.0 },
        origin: Vec2::default(),
        definition_id: "key-definition".into(),
        part_ids: vec![key_id.into()],
        board_id: Some("board".into()),
        mirror: None,
        rotation: None,
        edge_gap: None,
        diode_direction: None,
        row_layout: vec![],
        column_layout: vec![],
        anchor: Some(MatrixAnchor {
            matrix_id: "matrix".into(),
            row: 0,
            column: 0,
            follow: Follow {
                x: true,
                y: true,
                rotation: rotation_followed,
            },
        }),
        cells: vec![MatrixCell {
            shift: None,
            bend: None,
            deleted: false,
            row: 0,
            column: 0,
            enabled: true,
            definition_id: Some("key-definition".into()),
            variant: None,
            offset: None,
            rotation: None,
            assemblies: vec![],
            assemblies_local: None,
        }],
    });
    (
        document,
        TreeContext::Matrix {
            matrix_id: "matrix-b".into(),
        },
    )
}

/// Open `document` through the real Session and Core, select `selected_ids` the way the
/// canvas does, and mount the transform Inspector over that runtime.
async fn mount_selection_probe(
    root_id: &'static str,
    document: ProjectDoc,
    context: TreeContext,
    selected_ids: &[&str],
) -> (Probe, web_sys::Element) {
    mount_selection_probe_with_canvas(root_id, document, context, selected_ids, false).await
}

async fn mount_selection_probe_with_canvas(
    root_id: &'static str,
    document: ProjectDoc,
    context: TreeContext,
    selected_ids: &[&str],
    canvas_controls: bool,
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
        canvas_controls,
        runtime,
        selected: ScopedTreeContext { scope, context },
        root_id,
        version: Rc::default(),
        selected_context: Rc::default(),
        adapter: Rc::default(),
        mount: Rc::default(),
        render_count: Rc::default(),
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
    settle(&probe).await;
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
    settle(&probe).await;

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
    settle(&probe).await;
    support::run_pending(&probe.runtime).await;
    wait_until("the selected keys accept their disabled state", || {
        probe
            .runtime
            .model()
            .accepted
            .as_ref()
            .is_some_and(|snapshot| {
                snapshot.document.matrices[0]
                    .cells
                    .iter()
                    .all(|cell| !cell.enabled)
            })
    })
    .await;
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
    settle(&probe).await;
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
    wait_until("the same selection accepts its enabled state", || {
        probe
            .runtime
            .model()
            .accepted
            .as_ref()
            .is_some_and(|snapshot| {
                snapshot.document.matrices[0]
                    .cells
                    .iter()
                    .all(|cell| cell.enabled)
            })
    })
    .await;
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
    wait_until("Undo restores the disabled selection", || {
        probe
            .runtime
            .model()
            .accepted
            .as_ref()
            .is_some_and(|snapshot| {
                snapshot.document.matrices[0]
                    .cells
                    .iter()
                    .all(|cell| !cell.enabled)
            })
    })
    .await;
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
    wait_until("Redo restores the enabled selection", || {
        probe
            .runtime
            .model()
            .accepted
            .as_ref()
            .is_some_and(|snapshot| {
                snapshot.document.matrices[0]
                    .cells
                    .iter()
                    .all(|cell| cell.enabled)
            })
    })
    .await;
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
    settle(&probe).await;
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
    settle(&probe).await;

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
    settle(&probe).await;
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
    settle(&probe).await;
    ctrl_click(2);
    settle(&probe).await;

    // Removing C leaves A and B selected; C is still a live key, so the Inspector
    // must be re-anchored to a member that remains selected.
    ctrl_click(2);
    settle(&probe).await;
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
    settle(&probe).await;
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
    wait_until("the remaining key accepts its local X edit", || {
        accepted_cell(&probe, 0).offset.map(|offset| offset.x) == Some(5.0)
    })
    .await;
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

async fn settle(probe: &Probe) {
    wait_until("matrix transform Inspector mounts", || {
        probe.version.borrow().is_some()
    })
    .await;
    let before = probe.render_count.get();
    let mut version = probe
        .version
        .borrow()
        .as_ref()
        .copied()
        .expect("Editor version signal");
    version += 1;
    wait_until(
        "matrix transform Inspector renders the current state",
        || probe.render_count.get() > before,
    )
    .await;
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
    settle(&probe).await;
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
    settle(&probe).await;
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
        settle(&probe).await;
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
    settle(&probe).await;

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
    settle(&probe).await;

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
    settle(&probe).await;

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
    settle(&probe).await;

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
    settle(&probe).await;
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
    assert_eq!(input.get_attribute("aria-invalid").as_deref(), Some("true"));
    let descriptions = input
        .get_attribute("aria-describedby")
        .expect("units and failure describe the field");
    let descriptions: Vec<_> = descriptions
        .split_whitespace()
        .map(|id| {
            web_sys::window()
                .unwrap()
                .document()
                .unwrap()
                .get_element_by_id(id)
                .expect("description is mounted")
                .text_content()
                .unwrap()
        })
        .collect();
    assert!(descriptions.iter().any(|text| text == "mm"));
    assert!(
        descriptions
            .iter()
            .any(|text| text.contains("injected transform failure"))
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
    settle(&probe).await;

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
    settle(&probe).await;
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
    settle(&probe).await;
    let delete = root.query_selector_all("button").unwrap();
    let delete = (0..delete.length())
        .filter_map(|index| delete.item(index))
        .find(|node| node.text_content().as_deref() == Some("Delete selected keys"))
        .expect("key properties provide actual deletion distinct from the enabled toggle")
        .dyn_into::<web_sys::HtmlElement>()
        .unwrap();
    delete.click();
    support::run_pending(&probe.runtime).await;
    settle(&probe).await;
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
    settle(&probe).await;
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
    settle(&probe).await;
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
    settle(&probe).await;
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
    settle(&probe).await;
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
    settle(&probe).await;
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
    settle(&probe).await;
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
        settle(&probe).await;
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
        settle(&probe).await;
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

fn column_context(column: u32) -> TreeContext {
    TreeContext::Column {
        matrix_id: "matrix".into(),
        column,
    }
}

#[wasm_bindgen_test]
async fn column_angle_scope_switches_display_and_discards_the_other_fields_draft() {
    let (mut document, _) = two_key_fixture(false);
    document.matrices[0]
        .column_layout
        .resize_with(1, Default::default);
    document.matrices[0].column_layout[0].splay = 7.0;
    document.matrices[0].column_layout[0].rotation = 10.0;
    let (probe, root) =
        mount_selection_probe("column-angle-scope-test", document, column_context(0), &[]).await;
    settle(&probe).await;
    let input = root
        .query_selector("input[aria-label='Splay']")
        .unwrap()
        .unwrap()
        .dyn_into::<HtmlInputElement>()
        .unwrap();
    assert_eq!(input.value(), "10", "This column displays Column rotation");
    let before = probe.runtime.model().accepted.unwrap().document;
    input.set_value("99");
    let typed = DomEvent::new("input").unwrap();
    typed.init_event_with_bubbles("input", true);
    input.dispatch_event(&typed).unwrap();
    settle(&probe).await;
    let mut affect = probe.mount.borrow().as_ref().unwrap().splay_affect;
    affect.set(boardstudio_core::model::MatrixSplayAffect::Following);
    settle(&probe).await;
    assert_eq!(
        input.value(),
        "7",
        "switching fields resets the dirty draft"
    );
    assert_eq!(probe.runtime.model().accepted.unwrap().document, before);
    affect.set(boardstudio_core::model::MatrixSplayAffect::Column);
    settle(&probe).await;
    assert_eq!(input.value(), "10");
    set_numeric_field(&root, "Splay", "0");
    support::run_pending(&probe.runtime).await;
    settle(&probe).await;
    let accepted = probe.runtime.model().accepted.unwrap();
    assert_eq!(accepted.document.matrices[0].column_layout[0].rotation, 0.0);
    assert_eq!(accepted.document.matrices[0].column_layout[0].splay, 7.0);
    root.remove();
}

#[wasm_bindgen_test]
async fn column_angle_equality_uses_the_requested_field_in_the_accepted_snapshot() {
    use super::super::matrix_transform_controller::transform_edit_resolver;
    use boardstudio_application::Resolution;
    use boardstudio_core::model::MatrixSplayAffect;
    let (mut document, _) = two_key_fixture(false);
    document.matrices[0]
        .column_layout
        .resize_with(1, Default::default);
    document.matrices[0].column_layout[0].rotation = 10.0;
    let runtime = support::new_runtime();
    support::open_document(&runtime, document).await;
    let accepted = runtime.model().accepted.unwrap();
    let resolve = |value, affect| {
        transform_edit_resolver(
            "matrix".into(),
            "board".into(),
            column_context(0),
            MatrixTransformField::ColumnSplay,
            MatrixTransformValue::Number(value),
            affect,
            vec![],
        )
        .resolve(&accepted)
    };
    assert!(
        matches!(
            resolve(10.0, MatrixSplayAffect::Column),
            Resolution::Unchanged
        ),
        "requesting the current rotation is a no-op even when Splay differs"
    );
    assert!(
        matches!(
            resolve(0.0, MatrixSplayAffect::Column),
            Resolution::Submit(_)
        ),
        "zero Splay must not retire a request to clear nonzero rotation"
    );
    assert!(matches!(
        resolve(0.0, MatrixSplayAffect::Following),
        Resolution::Unchanged
    ));
}

fn set_numeric_field(root: &web_sys::Element, label: &str, value: &str) {
    let input = root
        .query_selector(&format!("input[aria-label='{label}']"))
        .unwrap()
        .unwrap_or_else(|| panic!("the {label} field is mounted"))
        .dyn_into::<HtmlInputElement>()
        .unwrap();
    input.set_value(value);
    let typed = DomEvent::new("input").unwrap();
    typed.init_event_with_bubbles("input", true);
    input.dispatch_event(&typed).unwrap();
    let enter = web_sys::KeyboardEventInit::new();
    enter.set_bubbles(true);
    enter.set_key("Enter");
    input
        .dispatch_event(
            &web_sys::KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &enter).unwrap(),
        )
        .unwrap();
}

fn second_column_y(runtime: &Rc<Runtime>) -> f64 {
    runtime.model().accepted.unwrap().scene.matrix_scenes[0]
        .cells
        .iter()
        .find(|cell| cell.column == 1)
        .expect("the second column has a key")
        .pose
        .at
        .y
}

#[wasm_bindgen_test]
async fn inspector_stagger_lands_as_a_placement_edit_that_moves_later_columns_with_one_undo() {
    let (document, _) = two_key_fixture(false);
    let (probe, root) = mount_selection_probe(
        "matrix-transform-stagger-test",
        document,
        column_context(0),
        &[],
    )
    .await;
    settle(&probe).await;
    let contexts = root.query_selector_all(".m1-matrix-field-context").unwrap();
    let label = (0..contexts.length())
        .filter_map(|index| contexts.item(index))
        .find_map(|node| {
            let text = node.text_content()?;
            text.starts_with("Column").then_some(text)
        });
    assert_eq!(label.as_deref(), Some("Column 1"));
    // The fixture's key poses were never laid out by Core; the first edit syncs them, so
    // measure the second change from that synced state.
    set_numeric_field(&root, "Stagger", "1");
    support::run_pending(&probe.runtime).await;
    settle(&probe).await;
    let before = second_column_y(&probe.runtime);
    let revision = probe.runtime.model().accepted.unwrap().document.revision;

    set_numeric_field(&root, "Stagger", "4");
    support::run_pending(&probe.runtime).await;
    settle(&probe).await;

    let accepted = probe.runtime.model().accepted.unwrap();
    assert_eq!(
        accepted.document.matrices[0]
            .column_layout
            .first()
            .map(|record| &record.stagger)
            .copied(),
        Some(4.0),
        "the Inspector value is the matrix's column stagger"
    );
    let axis = accepted.scene.matrix_scenes[0].columns[0].stagger_axis;
    let moved = second_column_y(&probe.runtime) - before;
    assert!(
        (moved - 3.0 * axis.y).abs() < 1e-9 && axis.y != 0.0,
        "the later column moved 3 mm along Core's stagger axis ({moved} vs {})",
        3.0 * axis.y
    );
    assert_eq!(
        accepted.document.revision,
        revision + 1,
        "one accepted edit"
    );

    probe.runtime.submit(Event::Undo {
        operation_id: probe.runtime.operation(),
    });
    support::run_pending(&probe.runtime).await;
    settle(&probe).await;
    let undone = probe.runtime.model().accepted.unwrap();
    assert_eq!(
        undone.document.matrices[0]
            .column_layout
            .first()
            .map(|record| &record.stagger)
            .copied(),
        Some(1.0),
        "one Undo reverts the whole second stagger edit"
    );
    assert!((second_column_y(&probe.runtime) - before).abs() < 1e-9);
    root.remove();
}

fn button_named(root: &web_sys::Element, label: &str) -> Option<web_sys::HtmlElement> {
    let buttons = root.query_selector_all("button").unwrap();
    (0..buttons.length())
        .filter_map(|index| buttons.item(index))
        .find(|node| node.text_content().as_deref() == Some(label))
        .map(|node| node.dyn_into::<web_sys::HtmlElement>().unwrap())
}

fn field_number(root: &web_sys::Element, label: &str) -> f64 {
    root.query_selector(&format!("input[aria-label='{label}']"))
        .unwrap()
        .unwrap_or_else(|| panic!("the {label} field is mounted"))
        .dyn_into::<HtmlInputElement>()
        .unwrap()
        .value()
        .parse()
        .unwrap_or_else(|_| panic!("the {label} field holds a number"))
}

#[wasm_bindgen_test]
async fn convert_to_stagger_appears_only_for_a_nonzero_offset_y_and_is_one_undo_step() {
    // No offset Y: no action.
    let (document, _) = two_key_fixture(false);
    let (probe, root) = mount_selection_probe(
        "matrix-transform-convert-hidden-test",
        document,
        column_context(0),
        &[],
    )
    .await;
    settle(&probe).await;
    assert!(button_named(&root, "Convert to stagger").is_none());
    root.remove();

    let (mut document, _) = two_key_fixture(false);
    {
        let values = vec![Vec2 { x: 0.0, y: -9.525 }];
        let column_layout = &mut document.matrices[0].column_layout;
        column_layout.resize_with(values.len().max(column_layout.len()), Default::default);
        for record in &mut document.matrices[0].column_layout {
            record.offset = Vec2::default();
        }
        for (record, value) in document.matrices[0].column_layout.iter_mut().zip(values) {
            record.offset = value;
        }
    }
    let (probe, root) = mount_selection_probe(
        "matrix-transform-convert-test",
        document,
        column_context(0),
        &[],
    )
    .await;
    settle(&probe).await;
    let button = button_named(&root, "Convert to stagger").expect("the action is mounted");
    assert!(!button.has_attribute("disabled"));
    let revision = probe.runtime.model().accepted.unwrap().document.revision;
    button.click();
    support::run_pending(&probe.runtime).await;
    settle(&probe).await;

    let accepted = probe.runtime.model().accepted.unwrap();
    let matrix = &accepted.document.matrices[0];
    assert_eq!(
        accepted.document.revision,
        revision + 1,
        "one accepted edit"
    );
    assert_eq!(
        matrix
            .column_layout
            .iter()
            .map(|record| record.stagger)
            .collect::<Vec<_>>(),
        vec![-9.525, 9.525]
    );
    assert_eq!(matrix.column_layout[0].offset.y, 0.0);
    // The probe host re-renders only when its version signal moves.
    let mut version = probe.version.borrow().expect("Editor version signal");
    let next = version.peek().saturating_add(1);
    version.set(next);
    settle(&probe).await;
    assert_eq!(field_number(&root, "Stagger"), -9.525);
    assert_eq!(field_number(&root, "Offset Y"), 0.0);
    assert!(
        button_named(&root, "Convert to stagger").is_none(),
        "nothing left to convert"
    );

    probe.runtime.submit(Event::Undo {
        operation_id: probe.runtime.operation(),
    });
    support::run_pending(&probe.runtime).await;
    settle(&probe).await;
    let undone = probe.runtime.model().accepted.unwrap();
    assert_eq!(
        undone.document.matrices[0].column_layout[0].offset.y,
        -9.525
    );
    assert!(
        undone.document.matrices[0]
            .column_layout
            .iter()
            .map(|record| &record.stagger)
            .all(|value| *value == 0.0)
    );
    root.remove();
}

#[wasm_bindgen_test]
async fn convert_to_stagger_is_disabled_with_core_reason_on_a_splayed_column() {
    let (mut document, _) = two_key_fixture(false);
    {
        let values = vec![Vec2 { x: 0.0, y: -9.525 }];
        let column_layout = &mut document.matrices[0].column_layout;
        column_layout.resize_with(values.len().max(column_layout.len()), Default::default);
        for record in &mut document.matrices[0].column_layout {
            record.offset = Vec2::default();
        }
        for (record, value) in document.matrices[0].column_layout.iter_mut().zip(values) {
            record.offset = value;
        }
    }
    {
        let values = vec![10.0];
        let column_layout = &mut document.matrices[0].column_layout;
        column_layout.resize_with(values.len().max(column_layout.len()), Default::default);
        for record in &mut document.matrices[0].column_layout {
            record.splay = 0.0;
        }
        for (record, value) in document.matrices[0].column_layout.iter_mut().zip(values) {
            record.splay = value;
        }
    }
    let (probe, root) = mount_selection_probe(
        "matrix-transform-convert-splayed-test",
        document,
        column_context(0),
        &[],
    )
    .await;
    settle(&probe).await;
    let button = button_named(&root, "Convert to stagger").expect("the action is mounted");
    assert!(
        button.has_attribute("disabled"),
        "a splayed column cannot convert"
    );
    let reason = boardstudio_core::model::convert_offset_available(
        &probe.runtime.model().accepted.unwrap().document.matrices[0],
        0,
    )
    .unwrap_err();
    let shown = root
        .query_selector(".m1-matrix-transform-unavailable")
        .unwrap()
        .expect("the reason is shown")
        .text_content();
    assert_eq!(shown.as_deref(), Some(reason.as_str()));
    assert_eq!(
        reason,
        "Converting requires zero Splay, Column rotation and carried Bend"
    );
    root.remove();
}

#[wasm_bindgen_test]
async fn collapsing_column_sections_preserves_drafts_nodes_and_edit_owner() {
    let (document, _) = two_key_fixture(false);
    let (probe, root) = mount_selection_probe(
        "matrix-transform-disclosure-test",
        document,
        column_context(0),
        &[],
    )
    .await;
    settle(&probe).await;
    let accepted = probe.runtime.model().accepted.unwrap().document.revision;
    let position = root
        .query_selector("details[aria-label='Position and rotation']")
        .unwrap()
        .expect("position section is a mounted disclosure");
    let splay = root
        .query_selector("details[aria-label='Splay and origin']")
        .unwrap()
        .expect("splay section is a mounted disclosure");
    assert!(
        !splay.has_attribute("open"),
        "specialist origin controls start collapsed"
    );
    splay
        .query_selector("summary")
        .unwrap()
        .unwrap()
        .dyn_into::<web_sys::HtmlElement>()
        .unwrap()
        .click();
    let input = root
        .query_selector("input[aria-label='Stagger']")
        .unwrap()
        .unwrap()
        .dyn_into::<HtmlInputElement>()
        .unwrap();
    input.set_value("");
    let event = DomEvent::new("input").unwrap();
    event.init_event_with_bubbles("input", true);
    input.dispatch_event(&event).unwrap();
    settle(&probe).await;
    position
        .query_selector("summary")
        .unwrap()
        .unwrap()
        .dyn_into::<web_sys::HtmlElement>()
        .unwrap()
        .click();
    assert!(!position.has_attribute("open"));
    assert!(
        splay.has_attribute("open"),
        "sections can be collapsed independently"
    );
    let mut version = probe.version.borrow().expect("Editor version signal");
    let next = version.peek().saturating_add(1);
    version.set(next);
    settle(&probe).await;
    assert!(
        !position.has_attribute("open"),
        "unrelated renders retain collapsed state"
    );
    assert!(
        input.is_same_node(
            root.query_selector("input[aria-label='Stagger']")
                .unwrap()
                .as_ref()
                .map(|node| node.as_ref())
        ),
        "collapse never remounts field ownership"
    );
    position
        .query_selector("summary")
        .unwrap()
        .unwrap()
        .dyn_into::<web_sys::HtmlElement>()
        .unwrap()
        .click();
    settle(&probe).await;
    assert_eq!(input.value(), "");
    assert_eq!(
        probe.runtime.model().accepted.unwrap().document.revision,
        accepted,
        "toggling sections never submits edits"
    );
    root.remove();
}

#[wasm_bindgen_test]
async fn canvas_dimensions_preserve_inspector_drafts_and_share_undo() {
    let (document, _) = two_key_fixture(false);
    let (probe, root) = mount_selection_probe_with_canvas(
        "canvas-independent-drafts",
        document,
        column_context(0),
        &[],
        true,
    )
    .await;
    settle(&probe).await;
    let inspector = root
        .query_selector("#inspector-geometry-test")
        .unwrap()
        .unwrap();
    let canvas = root
        .query_selector("#canvas-geometry-test")
        .unwrap()
        .unwrap();
    let draft = inspector
        .query_selector("input[aria-label='Stagger']")
        .unwrap()
        .unwrap()
        .dyn_into::<HtmlInputElement>()
        .unwrap();
    draft.set_value("");
    let input = DomEvent::new("input").unwrap();
    input.init_event_with_bubbles("input", true);
    draft.dispatch_event(&input).unwrap();
    settle(&probe).await;
    let before = probe.runtime.model().accepted.unwrap().document.revision;
    set_numeric_field(&canvas, "Stagger", "4");
    support::run_pending(&probe.runtime).await;
    settle(&probe).await;
    let accepted = probe.runtime.model().accepted.unwrap();
    assert_eq!(accepted.document.matrices[0].column_layout[0].stagger, 4.0);
    assert_eq!(accepted.document.revision, before + 1);
    assert_eq!(
        draft.value(),
        "",
        "canvas edits preserve an unfinished inspector draft"
    );
    probe.runtime.submit(Event::Undo {
        operation_id: probe.runtime.operation(),
    });
    support::run_pending(&probe.runtime).await;
    settle(&probe).await;
    assert_eq!(
        probe.runtime.model().accepted.unwrap().document.matrices[0]
            .column_layout
            .first()
            .map(|record| &record.stagger)
            .copied()
            .unwrap_or(0.0),
        0.0
    );
    assert_eq!(draft.value(), "");
    root.remove();
}

/// A second row under the default 1x1 key fixture, so carried row edits have a
/// later key to move. The matrix frame stays identity: no splay, no mirror,
/// origin zero, so world deltas equal matrix deltas exactly.
fn two_row_fixture() -> (ProjectDoc, TreeContext) {
    let (mut document, selected) = fixture();
    let second_key = "matrix/matrix/r1c0";
    let matrix = &mut document.matrices[0];
    matrix.rows = 2;
    matrix.part_ids.push(second_key.into());
    document.boards[0].part_ids.push(second_key.into());
    document.parts.push(Part {
        keycap: None,
        outline: None,
        id: second_key.into(),
        definition_id: "key-definition".into(),
        reference: "K2".into(),
        pose: Pose2 {
            at: Vec2::default(),
            rotation: 0.0,
        },
        side: Side::Front,
        locked: Some(false),
        properties: None,
        generator_parameters: None,
    });
    (document, selected)
}

/// A second column over the two-row fixture, so a key Shift has a same-column
/// later key to carry to and a neighbouring column that must stay still.
fn shift_grid_fixture() -> (ProjectDoc, TreeContext) {
    let (mut document, selected) = two_row_fixture();
    let matrix = &mut document.matrices[0];
    matrix.columns = 2;
    for key in ["matrix/matrix/r0c1", "matrix/matrix/r1c1"] {
        matrix.part_ids.push(key.into());
        document.boards[0].part_ids.push(key.into());
        document.parts.push(Part {
            keycap: None,
            outline: None,
            id: key.into(),
            definition_id: "key-definition".into(),
            reference: format!("K{key}"),
            pose: Pose2 {
                at: Vec2::default(),
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

fn row_context(row: u32) -> TreeContext {
    TreeContext::Row {
        matrix_id: "matrix".into(),
        row,
    }
}

fn scene_key_at(probe: &Probe, row: u32, column: u32) -> Vec2 {
    probe
        .runtime
        .model()
        .accepted
        .expect("an accepted document")
        .scene
        .matrix_scenes
        .iter()
        .find(|scene| scene.matrix_id == "matrix")
        .and_then(|scene| {
            scene
                .cells
                .iter()
                .find(|cell| cell.row == row && cell.column == column)
        })
        .expect("the key is laid out")
        .pose
        .at
}

/// Ticket 23 first slice: the Row Inspector names the carried Shift fields.
/// Setting Shift X moves the later-row key through Core's running frame as one
/// accepted edit, and one Undo restores both rows exactly.
#[wasm_bindgen_test]
async fn inspector_row_shift_moves_later_row_keys_with_one_undo() {
    let (document, _) = two_row_fixture();
    let (probe, root) = mount_selection_probe(
        "matrix-transform-row-shift-test",
        document,
        row_context(0),
        &[],
    )
    .await;
    settle(&probe).await;
    let before_later = scene_key_at(&probe, 1, 0);
    let before_own = scene_key_at(&probe, 0, 0);
    let revision = probe.runtime.model().accepted.unwrap().document.revision;

    set_numeric_field(&root, "Shift X", "4");
    support::run_pending(&probe.runtime).await;
    settle(&probe).await;

    let accepted = probe.runtime.model().accepted.unwrap();
    assert_eq!(
        accepted.document.matrices[0]
            .row_layout
            .first()
            .map(|record| &record.shift)
            .copied()
            .unwrap_or_default(),
        Vec2 { x: 4.0, y: 0.0 },
        "the Inspector value is the matrix row's carried Shift"
    );
    assert_eq!(
        accepted.document.revision,
        revision + 1,
        "one accepted edit"
    );
    let later = scene_key_at(&probe, 1, 0);
    assert!(
        (later.x - before_later.x - 4.0).abs() < 1e-9 && (later.y - before_later.y).abs() < 1e-9,
        "the later-row key moves the full Shift through Core's running frame"
    );
    let own = scene_key_at(&probe, 0, 0);
    assert!(
        (own.x - before_own.x - 4.0).abs() < 1e-9 && (own.y - before_own.y).abs() < 1e-9,
        "the edited row's own key carries its Shift too"
    );

    probe.runtime.submit(Event::Undo {
        operation_id: probe.runtime.operation(),
    });
    support::run_pending(&probe.runtime).await;
    settle(&probe).await;
    let undone = probe.runtime.model().accepted.unwrap();
    assert_eq!(
        undone.document.matrices[0]
            .row_layout
            .first()
            .map(|record| &record.shift)
            .copied()
            .unwrap_or_default(),
        Vec2::default()
    );
    let restored_later = scene_key_at(&probe, 1, 0);
    let restored_own = scene_key_at(&probe, 0, 0);
    assert!(
        (restored_later.x - before_later.x).abs() < 1e-9
            && (restored_later.y - before_later.y).abs() < 1e-9
            && (restored_own.x - before_own.x).abs() < 1e-9
            && (restored_own.y - before_own.y).abs() < 1e-9,
        "one Undo restores both rows exactly"
    );
    root.remove();
}

/// Ticket 23 first slice: the Key Inspector names this-column Shift fields.
/// Setting Shift X moves the edited key and carries down its own column through
/// Core's running frame as one accepted edit; the neighbouring column stays
/// still, and one Undo restores all four keys exactly.
#[wasm_bindgen_test]
async fn inspector_key_shift_carries_down_its_column_with_one_undo() {
    let (document, selected) = shift_grid_fixture();
    let (probe, root) =
        mount_selection_probe("matrix-transform-key-shift-test", document, selected, &[]).await;
    settle(&probe).await;
    let before_moved = scene_key_at(&probe, 0, 0);
    let before_carried = scene_key_at(&probe, 1, 0);
    let before_next_own = scene_key_at(&probe, 0, 1);
    let before_next_later = scene_key_at(&probe, 1, 1);
    let revision = probe.runtime.model().accepted.unwrap().document.revision;

    set_numeric_field(&root, "Shift X", "3");
    support::run_pending(&probe.runtime).await;
    settle(&probe).await;

    let accepted = probe.runtime.model().accepted.unwrap();
    let cell = accepted.document.matrices[0]
        .cells
        .iter()
        .find(|cell| cell.row == 0 && cell.column == 0)
        .expect("the edited key keeps its cell");
    assert_eq!(
        cell.shift.unwrap_or_default(),
        Vec2 { x: 3.0, y: 0.0 },
        "the Inspector value is the key's own Shift"
    );
    assert_eq!(
        accepted.document.revision,
        revision + 1,
        "one accepted edit"
    );
    let moved = scene_key_at(&probe, 0, 0);
    assert!(
        (moved.x - before_moved.x - 3.0).abs() < 1e-9 && (moved.y - before_moved.y).abs() < 1e-9,
        "the edited key moves its Shift through Core's frame"
    );
    let carried = scene_key_at(&probe, 1, 0);
    assert!(
        (carried.x - before_carried.x - 3.0).abs() < 1e-9
            && (carried.y - before_carried.y).abs() < 1e-9,
        "a key Shift carries down its own column"
    );
    let next_own = scene_key_at(&probe, 0, 1);
    let next_later = scene_key_at(&probe, 1, 1);
    assert!(
        (next_own.x - before_next_own.x).abs() < 1e-9
            && (next_own.y - before_next_own.y).abs() < 1e-9
            && (next_later.x - before_next_later.x).abs() < 1e-9
            && (next_later.y - before_next_later.y).abs() < 1e-9,
        "the neighbouring column does not follow a key Shift"
    );

    probe.runtime.submit(Event::Undo {
        operation_id: probe.runtime.operation(),
    });
    support::run_pending(&probe.runtime).await;
    settle(&probe).await;
    for (row, column, before) in [
        (0, 0, before_moved),
        (1, 0, before_carried),
        (0, 1, before_next_own),
        (1, 1, before_next_later),
    ] {
        let restored = scene_key_at(&probe, row, column);
        assert!(
            (restored.x - before.x).abs() < 1e-9 && (restored.y - before.y).abs() < 1e-9,
            "one Undo restores key ({row}, {column}) exactly"
        );
    }
    root.remove();
}

fn scene_key_rotation(probe: &Probe, row: u32, column: u32) -> f64 {
    probe
        .runtime
        .model()
        .accepted
        .expect("an accepted document")
        .scene
        .matrix_scenes
        .iter()
        .find(|scene| scene.matrix_id == "matrix")
        .and_then(|scene| {
            scene
                .cells
                .iter()
                .find(|cell| cell.row == row && cell.column == column)
        })
        .expect("the key is laid out")
        .pose
        .rotation
}

/// Ticket 23, RED: the Row Inspector names the carried Bend field. Setting
/// Bend turns the later-row keys through Core's running frame as one accepted
/// edit, and one Undo restores both rows exactly.
#[wasm_bindgen_test]
async fn inspector_row_bend_turns_later_row_keys_with_one_undo() {
    let (document, _) = two_row_fixture();
    let (probe, root) = mount_selection_probe(
        "matrix-transform-row-bend-test",
        document,
        row_context(0),
        &[],
    )
    .await;
    settle(&probe).await;
    let revision = probe.runtime.model().accepted.unwrap().document.revision;

    set_numeric_field(&root, "Bend", "10");
    support::run_pending(&probe.runtime).await;
    settle(&probe).await;

    let accepted = probe.runtime.model().accepted.unwrap();
    assert_eq!(
        accepted.document.matrices[0]
            .row_layout
            .first()
            .map(|record| &record.bend)
            .copied()
            .unwrap_or_default(),
        10.0,
        "the Inspector value is the matrix row's carried Bend"
    );
    assert_eq!(
        accepted.document.revision,
        revision + 1,
        "one accepted edit"
    );
    assert!(
        (scene_key_rotation(&probe, 0, 0) - 10.0).abs() < 1e-9
            && (scene_key_rotation(&probe, 1, 0) - 10.0).abs() < 1e-9,
        "the edited row and its later row turn the full Bend"
    );

    probe.runtime.submit(Event::Undo {
        operation_id: probe.runtime.operation(),
    });
    support::run_pending(&probe.runtime).await;
    settle(&probe).await;
    assert!(
        scene_key_rotation(&probe, 0, 0).abs() < 1e-9
            && scene_key_rotation(&probe, 1, 0).abs() < 1e-9,
        "one Undo restores both rows exactly"
    );
    root.remove();
}

/// Ticket 23, RED: the Key Inspector names this-column Bend. Setting Bend
/// turns the edited key and carries down its own column as one accepted edit;
/// the neighbouring column stays still, and one Undo restores all four keys.
#[wasm_bindgen_test]
async fn inspector_key_bend_carries_down_its_column_with_one_undo() {
    let (document, selected) = shift_grid_fixture();
    let (probe, root) =
        mount_selection_probe("matrix-transform-key-bend-test", document, selected, &[]).await;
    settle(&probe).await;
    let revision = probe.runtime.model().accepted.unwrap().document.revision;

    set_numeric_field(&root, "Bend", "10");
    support::run_pending(&probe.runtime).await;
    settle(&probe).await;

    let accepted = probe.runtime.model().accepted.unwrap();
    let cell = accepted.document.matrices[0]
        .cells
        .iter()
        .find(|cell| cell.row == 0 && cell.column == 0)
        .expect("the edited key keeps its cell");
    assert_eq!(
        cell.bend.unwrap_or_default(),
        10.0,
        "the Inspector value is the key's own Bend"
    );
    assert_eq!(
        accepted.document.revision,
        revision + 1,
        "one accepted edit"
    );
    assert!(
        (scene_key_rotation(&probe, 0, 0) - 10.0).abs() < 1e-9
            && (scene_key_rotation(&probe, 1, 0) - 10.0).abs() < 1e-9,
        "a key Bend turns its key and carries down its column"
    );
    assert!(
        scene_key_rotation(&probe, 0, 1).abs() < 1e-9
            && scene_key_rotation(&probe, 1, 1).abs() < 1e-9,
        "the neighbouring column does not follow a key Bend"
    );

    probe.runtime.submit(Event::Undo {
        operation_id: probe.runtime.operation(),
    });
    support::run_pending(&probe.runtime).await;
    settle(&probe).await;
    for (row, column) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
        assert!(
            scene_key_rotation(&probe, row, column).abs() < 1e-9,
            "one Undo restores key ({row}, {column}) exactly"
        );
    }
    root.remove();
}

/// Ticket 23, RED: the Column Inspector names the inherited Spread field.
/// Setting Spread moves the later columns through Core as one accepted edit,
/// and one Undo restores both columns exactly.
#[wasm_bindgen_test]
async fn inspector_spread_moves_later_columns_with_one_undo() {
    let (document, _) = two_key_fixture(false);
    let (probe, root) = mount_selection_probe(
        "matrix-transform-spread-test",
        document,
        column_context(1),
        &[],
    )
    .await;
    settle(&probe).await;
    let before_moved = scene_key_at(&probe, 0, 1);
    let before_still = scene_key_at(&probe, 0, 0);
    let revision = probe.runtime.model().accepted.unwrap().document.revision;

    set_numeric_field(&root, "Spread", "24");
    support::run_pending(&probe.runtime).await;
    settle(&probe).await;

    let accepted = probe.runtime.model().accepted.unwrap();
    assert_eq!(
        accepted.document.matrices[0]
            .column_layout
            .get(1)
            .and_then(|record| record.spread),
        Some(24.0),
        "the Inspector value is the column's explicit Spread"
    );
    assert_eq!(
        accepted.document.revision,
        revision + 1,
        "one accepted edit"
    );
    let moved = scene_key_at(&probe, 0, 1);
    assert!(
        (moved.x - before_moved.x - 5.0).abs() < 1e-9 && (moved.y - before_moved.y).abs() < 1e-9,
        "column 1 steps out the full Spread past the 19 mm pitch"
    );
    let still = scene_key_at(&probe, 0, 0);
    assert!(
        (still.x - before_still.x).abs() < 1e-9 && (still.y - before_still.y).abs() < 1e-9,
        "column 0 does not follow a later Spread"
    );

    probe.runtime.submit(Event::Undo {
        operation_id: probe.runtime.operation(),
    });
    support::run_pending(&probe.runtime).await;
    settle(&probe).await;
    let restored_moved = scene_key_at(&probe, 0, 1);
    let restored_still = scene_key_at(&probe, 0, 0);
    assert!(
        (restored_moved.x - before_moved.x).abs() < 1e-9
            && (restored_moved.y - before_moved.y).abs() < 1e-9
            && (restored_still.x - before_still.x).abs() < 1e-9
            && (restored_still.y - before_still.y).abs() < 1e-9,
        "one Undo restores both columns exactly"
    );
    root.remove();
}

/// Ticket 23, RED: the Row Inspector names the Padding step field. Setting
/// Padding on row 1 moves that row's keys through Core as one accepted edit
/// without moving row 0, and one Undo restores both rows exactly.
#[wasm_bindgen_test]
async fn inspector_padding_moves_only_its_own_row_with_one_undo() {
    let (document, _) = two_row_fixture();
    let (probe, root) = mount_selection_probe(
        "matrix-transform-padding-test",
        document,
        row_context(1),
        &[],
    )
    .await;
    settle(&probe).await;
    let before_moved = scene_key_at(&probe, 1, 0);
    let before_still = scene_key_at(&probe, 0, 0);
    let revision = probe.runtime.model().accepted.unwrap().document.revision;

    set_numeric_field(&root, "Padding", "24");
    support::run_pending(&probe.runtime).await;
    settle(&probe).await;

    let accepted = probe.runtime.model().accepted.unwrap();
    assert_eq!(
        accepted.document.matrices[0]
            .row_layout
            .get(1)
            .and_then(|record| record.padding),
        Some(24.0),
        "the Inspector value is the row's explicit Padding"
    );
    assert_eq!(
        accepted.document.revision,
        revision + 1,
        "one accepted edit"
    );
    let moved = scene_key_at(&probe, 1, 0);
    assert!(
        (moved.y - before_moved.y - 5.0).abs() < 1e-9 && (moved.x - before_moved.x).abs() < 1e-9,
        "row 1 steps out the full Padding past the 19 mm pitch"
    );
    let still = scene_key_at(&probe, 0, 0);
    assert!(
        (still.x - before_still.x).abs() < 1e-9 && (still.y - before_still.y).abs() < 1e-9,
        "row 0 does not follow a later Padding"
    );

    probe.runtime.submit(Event::Undo {
        operation_id: probe.runtime.operation(),
    });
    support::run_pending(&probe.runtime).await;
    settle(&probe).await;
    let restored_moved = scene_key_at(&probe, 1, 0);
    let restored_still = scene_key_at(&probe, 0, 0);
    assert!(
        (restored_moved.x - before_moved.x).abs() < 1e-9
            && (restored_moved.y - before_moved.y).abs() < 1e-9
            && (restored_still.x - before_still.x).abs() < 1e-9
            && (restored_still.y - before_still.y).abs() < 1e-9,
        "one Undo restores both rows exactly"
    );
    root.remove();
}

fn section_named(root: &web_sys::Element, name: &str) -> Option<web_sys::Element> {
    let headings = root.query_selector_all("summary, h3").unwrap();
    (0..headings.length())
        .filter_map(|index| headings.item(index))
        .find(|node| node.text_content().as_deref() == Some(name))
        .and_then(|node| node.dyn_into::<web_sys::Element>().ok())
}

/// Ticket 23: column 0 has no Spread step, and the Inspector says why instead
/// of leaving a silent gap.
#[wasm_bindgen_test]
async fn column_zero_hides_spread_with_an_accessible_reason() {
    let (document, _) = two_key_fixture(false);
    let (probe, root) = mount_selection_probe(
        "matrix-transform-no-spread-test",
        document,
        column_context(0),
        &[],
    )
    .await;
    settle(&probe).await;
    assert!(
        root.query_selector("input[aria-label='Spread']")
            .unwrap()
            .is_none(),
        "column 0 has no Spread field"
    );
    let note = root
        .query_selector(".m1-matrix-transform-note")
        .unwrap()
        .expect("the structural rule is explained in place");
    assert_eq!(
        note.text_content().as_deref(),
        Some("Spread starts at column 1. Column 0 steps by pitch."),
        "the reason names the rule"
    );
    root.remove();
}

/// Ticket 23: row 0 has no Padding step, and the Inspector says why.
#[wasm_bindgen_test]
async fn row_zero_hides_padding_with_an_accessible_reason() {
    let (document, _) = two_row_fixture();
    let (probe, root) = mount_selection_probe(
        "matrix-transform-no-padding-test",
        document,
        row_context(0),
        &[],
    )
    .await;
    settle(&probe).await;
    assert!(
        root.query_selector("input[aria-label='Padding']")
            .unwrap()
            .is_none(),
        "row 0 has no Padding field"
    );
    let note = root
        .query_selector(".m1-matrix-transform-note")
        .unwrap()
        .expect("the structural rule is explained in place");
    assert_eq!(
        note.text_content().as_deref(),
        Some("Padding starts at row 1. Row 0 sits at the column base."),
        "the reason names the rule"
    );
    root.remove();
}

/// Ticket 23: a default Spread reads the pitch with a pitch badge; an explicit
/// Spread reads its own value with no badge.
#[wasm_bindgen_test]
async fn default_spread_shows_the_pitch_badge_and_explicit_spread_does_not() {
    let (document, _) = two_key_fixture(false);
    let (probe, root) = mount_selection_probe(
        "matrix-transform-spread-badge-test",
        document,
        column_context(1),
        &[],
    )
    .await;
    settle(&probe).await;
    let field = root
        .query_selector("input[aria-label='Spread']")
        .unwrap()
        .expect("column 1 has a Spread field")
        .dyn_into::<HtmlInputElement>()
        .unwrap();
    assert_eq!(field.value(), "19", "the default Spread reads the pitch");
    assert!(
        root.query_selector(".m1-pitch-badge").unwrap().is_some(),
        "the default Spread carries a pitch badge"
    );

    set_numeric_field(&root, "Spread", "24");
    support::run_pending(&probe.runtime).await;
    settle(&probe).await;
    assert!(
        root.query_selector(".m1-pitch-badge").unwrap().is_none(),
        "an explicit Spread drops the pitch badge"
    );
    root.remove();
}

/// Ticket 23: Carried and This-only sections group the named fields with
/// scope explanations.
#[wasm_bindgen_test]
async fn carried_and_this_only_sections_group_the_named_fields() {
    let (document, _) = two_key_fixture(false);
    let (probe, root) = mount_selection_probe(
        "matrix-transform-sections-test",
        document,
        column_context(1),
        &[],
    )
    .await;
    settle(&probe).await;
    assert!(
        section_named(&root, "Carried").is_some(),
        "the carried section is mounted"
    );
    assert!(
        section_named(&root, "This column only").is_some(),
        "the this-only section is mounted"
    );
    assert!(
        root.query_selector("input[aria-label='Spread']")
            .unwrap()
            .is_some(),
        "Spread stays a named field"
    );
    assert!(
        root.query_selector("input[aria-label='Stagger']")
            .unwrap()
            .is_some(),
        "Stagger stays a named field"
    );
    root.remove();
}

/// Ticket 23: a committed edit names its changed field and promises one Undo
/// entry; the line clears when the next edit submits.
#[wasm_bindgen_test]
async fn committed_shift_names_the_changed_field_with_one_undo() {
    let (document, _) = two_row_fixture();
    let (probe, root) = mount_selection_probe(
        "matrix-transform-status-test",
        document,
        row_context(0),
        &[],
    )
    .await;
    settle(&probe).await;
    assert!(
        root.query_selector(".m1-matrix-change-status")
            .unwrap()
            .is_none(),
        "no change is announced before the first commit"
    );

    set_numeric_field(&root, "Shift X", "4");
    support::run_pending(&probe.runtime).await;
    settle(&probe).await;
    let status = root
        .query_selector(".m1-matrix-change-status")
        .unwrap()
        .expect("the commit names its change");
    assert_eq!(
        status.text_content().as_deref(),
        Some("Changed Shift X · one Undo entry"),
        "the status names the changed field"
    );

    set_numeric_field(&root, "Shift X", "5");
    settle(&probe).await;
    assert!(
        root.query_selector(".m1-matrix-change-status")
            .unwrap()
            .is_none(),
        "a newer submit retires the previous announcement"
    );
    support::run_pending(&probe.runtime).await;
    settle(&probe).await;
    root.remove();
}

/// Ticket 23 slice 7a, RED: an anchored matrix that does not follow rotation
/// disables its Rotation field with an accessible reason instead of letting an
/// edit hit Core's anchored-frame refusal.
#[wasm_bindgen_test]
async fn unfollowed_rotation_disables_the_rotation_field_with_a_reason() {
    let (document, context) = anchored_matrix_fixture(false);
    let (probe, root) = mount_selection_probe(
        "matrix-transform-rotation-follow-test",
        document,
        context,
        &[],
    )
    .await;
    settle(&probe).await;
    let field = root
        .query_selector("input[aria-label='Rotation']")
        .unwrap()
        .expect("the Matrix section names Rotation")
        .dyn_into::<HtmlInputElement>()
        .unwrap();
    assert!(
        field.disabled(),
        "Rotation is unavailable without rotation Follow"
    );
    let note = root
        .query_selector(".m1-matrix-transform-note")
        .unwrap()
        .expect("the Follow rule is explained in place");
    assert_eq!(
        note.text_content().as_deref(),
        Some("Rotation is not followed, so turning it changes nothing."),
        "the reason names the Follow rule"
    );
    root.remove();
}

/// Ticket 23 slice 7a, RED control: a followed rotation stays editable.
#[wasm_bindgen_test]
async fn followed_rotation_keeps_the_rotation_field_editable() {
    let (document, context) = anchored_matrix_fixture(true);
    let (probe, root) = mount_selection_probe(
        "matrix-transform-rotation-follow-control-test",
        document,
        context,
        &[],
    )
    .await;
    settle(&probe).await;
    let field = root
        .query_selector("input[aria-label='Rotation']")
        .unwrap()
        .expect("the Matrix section names Rotation")
        .dyn_into::<HtmlInputElement>()
        .unwrap();
    assert!(
        !field.disabled(),
        "Rotation stays editable with rotation Follow"
    );
    root.remove();
}
