//! Mounted tests for the matrix transform Inspector over the real in-process Session and
//! Core. The probe host mounts the Inspector beside a stub matrix canvas that selects (and,
//! while an anchor pick is armed, picks) keys the way the Layout canvas does.
use super::*;
use crate::presentation::objects::matrix_edit_test_support as fixture;
use crate::presentation::objects::{
    ScopedTreeContext, TreeContext, use_workspace_matrix_transform,
};
use crate::runtime::Runtime;
use boardstudio_application::{AcceptedSnapshot, Event, SelectionMode};
use boardstudio_model::model::{
    AggregateAnchor, AggregateMethod, AnchorPart, Board, Layout, LayoutMirrorLink, Matrix,
    MatrixAnchor, MatrixAssembly, MirroredCorrespondence, MirroredException,
    MirroredExceptionState, MirroredItemKind, PartDefinition, PartKind, Pose2, ProjectDoc, Vec2,
};
use boardstudio_web_test_runtime::project_name_test_support as support;
use boardstudio_web_ui_shared::test_mount::MountedFixture;
use boardstudio_web_ui_shared::{test_role::by_role_name, test_wait::wait_until};
use dioxus_web::WebEventExt;
use fixture::{commit_value, key_part, matrix_cell, query, query_all, text_of, type_value};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
use wasm_bindgen::JsCast;
use wasm_bindgen_test::wasm_bindgen_test;
use web_sys::HtmlInputElement;

wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

#[path = "matrix_transform_inspector_tests/anchor.rs"]
mod anchor;

const ROOT_ID: &str = "matrix-transform-inspector-test";
const KEY: &str = "key-definition";

#[derive(Clone)]
struct Probe {
    canvas_controls: bool,
    runtime: Rc<Runtime>,
    selected: ScopedTreeContext,
    version: Rc<RefCell<Option<Signal<u64>>>>,
    selected_context: Rc<RefCell<Option<Signal<Option<ScopedTreeContext>>>>>,
    adapter: Rc<RefCell<Option<crate::presentation::selection::SelectionAdapter>>>,
    mount: Rc<RefCell<Option<MatrixTransformInspectorMount>>>,
    render_count: Rc<Cell<u64>>,
    navigated: Rc<RefCell<Vec<TreeContext>>>,
}

impl Probe {
    fn accepted(&self) -> AcceptedSnapshot {
        self.runtime.model().accepted.expect("an accepted document")
    }

    fn revision(&self) -> u64 {
        self.accepted().document.revision
    }

    fn matrix(&self, id: &str) -> Matrix {
        fixture::matrix_named(&self.runtime, id)
    }

    fn anchor(&self, matrix_id: &str) -> MatrixAnchor {
        self.matrix(matrix_id).anchor.expect("the anchor survives")
    }

    fn aggregate(&self) -> AggregateAnchor {
        self.anchor("matrix-b")
            .aggregate
            .expect("the anchor stays aggregate")
    }

    fn mount(&self) -> MatrixTransformInspectorMount {
        self.mount
            .borrow()
            .clone()
            .expect("the Inspector mount exists")
    }

    /// The selected context the host currently holds.
    fn selection(&self) -> Option<TreeContext> {
        let selection = *self.selected_context.borrow().as_ref().unwrap();
        selection
            .read()
            .as_ref()
            .map(|selected| selected.context.clone())
    }

    /// The laid-out pose of key (`row`, `column`) of `matrix_id`.
    fn scene_pose(&self, matrix_id: &str, row: u32, column: u32) -> Pose2 {
        self.accepted()
            .scene
            .matrix_scenes
            .iter()
            .find(|scene| scene.matrix_id == matrix_id)
            .and_then(|scene| {
                scene
                    .cells
                    .iter()
                    .find(|cell| cell.row == row && cell.column == column)
                    .map(|cell| cell.pose)
            })
            .expect("the key is laid out")
    }

    fn key_at(&self, row: u32, column: u32) -> Vec2 {
        self.scene_pose("matrix", row, column).at
    }

    /// The accepted cell (0, `column`) of the fixture matrix.
    fn cell(&self, column: u32) -> boardstudio_model::model::MatrixCell {
        self.matrix("matrix")
            .cells
            .into_iter()
            .find(|cell| cell.row == 0 && cell.column == column)
            .expect("the matrix cell exists")
    }

    /// Bump the host's Editor version and wait for the render that follows.
    async fn settle(&self) {
        wait_until("matrix transform Inspector mounts", || {
            self.version.borrow().is_some()
        })
        .await;
        let before = self.render_count.get();
        let mut version = self.version.borrow().expect("Editor version signal");
        version += 1;
        wait_until(
            "matrix transform Inspector renders the current state",
            || self.render_count.get() > before,
        )
        .await;
    }

    /// Drive the Session, then re-render the host.
    async fn run(&self) {
        support::run_pending(&self.runtime).await;
        self.settle().await;
    }

    async fn undo(&self) {
        self.runtime.submit(Event::Undo {
            operation_id: self.runtime.operation(),
        });
        self.run().await;
    }

    async fn redo(&self) {
        self.runtime.submit(Event::Redo {
            operation_id: self.runtime.operation(),
        });
        self.run().await;
    }
}

/// The mounted probe root; dropping it removes the root and unmounts the host.
struct Root {
    element: web_sys::Element,
    _mounted: MountedFixture,
}

impl std::ops::Deref for Root {
    type Target = web_sys::Element;
    fn deref(&self) -> &web_sys::Element {
        &self.element
    }
}

impl Drop for Root {
    fn drop(&mut self) {
        self.element.remove();
    }
}

/// Removes the test host's document `keydown` listener when the fixture unmounts.
struct DocumentKeydownGuard {
    document: web_sys::Document,
    callback: wasm_bindgen::closure::Closure<dyn FnMut(web_sys::KeyboardEvent)>,
}

impl Drop for DocumentKeydownGuard {
    fn drop(&mut self) {
        let _ = self
            .document
            .remove_event_listener_with_callback("keydown", self.callback.as_ref().unchecked_ref());
    }
}

/// The matrix canvas stub: three key hit targets on row 0 that pick an aggregate part while
/// the anchor pick is armed and otherwise submit the canvas key selection.
fn canvas_key_target(probe: &Probe, column: u32, armed: Signal<bool>) -> Element {
    let target_part_id = format!("matrix/matrix/r0c{column}");
    let adapter = probe.adapter.borrow().as_ref().unwrap().clone();
    let probe = probe.clone();
    rsx! {
        g {
            "data-part-id": "{target_part_id}",
            onpointerdown: move |event: PointerEvent| {
                if armed() {
                    probe.mount().pick_aggregate_part("matrix".into(), 0, column);
                    return;
                }
                let Some(pointer) = event.data().try_as_web_event() else { return; };
                let model = probe.runtime.model();
                let hit_context = TreeContext::Key { matrix_id: "matrix".into(), row: 0, column };
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
                    &probe.runtime,
                    &adapter,
                    &probe.selected.scope,
                    (adapter.generation)(),
                    crate::presentation::selection::MatrixCellSelection {
                        matrix_id: "matrix".into(),
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
    let splay_affect = use_signal(|| boardstudio_model::model::MatrixSplayAffect::Column);
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
    // The ticket-23 armed-pick pattern, mirrored for the test host: arming is host state, the
    // draft lives on the mount, and Esc clears both.
    let mut armed = use_signal(|| false);
    let mut esc_armed = armed;
    let esc_probe = probe.clone();
    // `use_hook` clones its state every render, so share the guard: the stored clone keeps the
    // listener until unmount drops it.
    let _keydown_guard: Option<Rc<DocumentKeydownGuard>> = use_hook(move || {
        let document = web_sys::window().and_then(|window| window.document())?;
        let callback =
            wasm_bindgen::closure::Closure::wrap(Box::new(move |event: web_sys::KeyboardEvent| {
                // The guard removes this listener on unmount; ignore keys once the VDOM
                // dropped its signals.
                if event.key() != "Escape"
                    || !esc_armed.try_peek().map(|armed| *armed).unwrap_or(false)
                    || esc_armed
                        .try_write()
                        .map(|mut armed| *armed = false)
                        .is_err()
                {
                    return;
                }
                if let Some(mount) = esc_probe.mount.borrow().as_ref() {
                    mount.clear_aggregate_pick();
                }
            })
                as Box<dyn FnMut(web_sys::KeyboardEvent)>);
        let _ =
            document.add_event_listener_with_callback("keydown", callback.as_ref().unchecked_ref());
        Some(Rc::new(DocumentKeydownGuard { document, callback }))
    });
    let mut canvas_tool = use_signal(|| Some(super::super::LayoutTransformTool::Stagger));
    rsx! {
        svg { id: "{ROOT_ID}-matrix-canvas",
            for column in 0..3u32 {
                {canvas_key_target(&probe, column, armed)}
            }
        }
        if probe.canvas_controls {
            div { id: "canvas-geometry-test",
                super::super::LayoutCanvasControls {
                    mount: mount.clone(),
                    tools: super::super::LayoutTransformMenuMount {
                        properties_available: true, column_available: true, row_available: true,
                        pointer_tools_visible: true, pointer_tools_available: true,
                        active_tool: canvas_tool(), matrix_rotate_followed: None,
                        on_selection_kind: EventHandler::new(|_| {}),
                        on_show_properties: EventHandler::new(|_| {}),
                        on_transform_tool: EventHandler::new(move |tool| canvas_tool.set(Some(tool))),
                    },
                }
            }
        }
        div { id: "inspector-geometry-test",
            MatrixTransformInspector {
                mount,
                on_pick_splay_origin: |_| {},
                on_pick_anchor: EventHandler::new(move |()| armed.set(true)),
                anchor_pick_pending: armed(),
                on_cancel_anchor_pick: {
                    let probe = probe.clone();
                    EventHandler::new(move |_| {
                        if let Some(mount) = probe.mount.borrow().as_ref() {
                            mount.clear_aggregate_pick();
                        }
                        armed.set(false);
                    })
                },
                on_navigate_anchor_part: EventHandler::new(move |context: TreeContext| {
                    probe.navigated.borrow_mut().push(context);
                }),
            }
        }
    }
}

/// Mount the transform Inspector over `runtime` with the `context` selection, and settle it.
async fn mount_runtime(
    runtime: Rc<Runtime>,
    context: TreeContext,
    canvas_controls: bool,
) -> (Probe, Root) {
    let scope = runtime.scope().expect("the opened project has a scope");
    let probe = Probe {
        canvas_controls,
        runtime,
        selected: ScopedTreeContext { scope, context },
        version: Rc::default(),
        selected_context: Rc::default(),
        adapter: Rc::default(),
        mount: Rc::default(),
        render_count: Rc::default(),
        navigated: Rc::default(),
    };
    let dom = VirtualDom::new(matrix_transform_host);
    dom.provide_root_context(probe.clone());
    let (element, mounted) = fixture::mount_root(dom);
    element.set_id(ROOT_ID);
    let root = Root {
        element,
        _mounted: mounted,
    };
    probe.settle().await;
    (probe, root)
}

async fn select_parts(runtime: &Rc<Runtime>, selected_ids: &[&str]) {
    runtime.submit(Event::SelectParts {
        operation_id: runtime.operation(),
        part_ids: selected_ids.iter().map(|id| (*id).to_owned()).collect(),
        range_part_ids: Vec::new(),
        mode: SelectionMode::Replace,
    });
    support::run_pending(runtime).await;
}

/// Open `document` through the real Session and Core, select `selected_ids` the way the
/// canvas does, and mount the transform Inspector over that runtime.
async fn mount_with(
    document: ProjectDoc,
    context: TreeContext,
    selected_ids: &[&str],
    canvas_controls: bool,
) -> (Probe, Root) {
    let runtime = support::new_runtime();
    support::open_document(&runtime, document).await;
    select_parts(&runtime, selected_ids).await;
    mount_runtime(runtime, context, canvas_controls).await
}

async fn mount_probe(
    document: ProjectDoc,
    context: TreeContext,
    selected_ids: &[&str],
) -> (Probe, Root) {
    mount_with(document, context, selected_ids, false).await
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

fn layout(id: &str, matrix_ids: &[&str]) -> Layout {
    Layout {
        id: id.into(),
        name: "Main".into(),
        board_id: "board".into(),
        matrix_ids: matrix_ids.iter().map(|id| (*id).into()).collect(),
        part_ids: vec![],
        mirror_link: None,
    }
}

fn column_context(column: u32) -> TreeContext {
    TreeContext::Column {
        matrix_id: "matrix".into(),
        column,
    }
}

fn row_context(row: u32) -> TreeContext {
    TreeContext::Row {
        matrix_id: "matrix".into(),
        row,
    }
}

fn key_context(matrix_id: &str, row: u32, column: u32) -> TreeContext {
    TreeContext::Key {
        matrix_id: matrix_id.into(),
        row,
        column,
    }
}

fn matrix_context(matrix_id: &str) -> TreeContext {
    TreeContext::Matrix {
        matrix_id: matrix_id.into(),
    }
}

/// A one-key matrix document with an attached LED. The test opens it through the real Session
/// and Core, which generate the scene the Inspector reads.
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
        definition(KEY, "Key"),
        definition("assembly-current/definition/led", "Current LED snapshot"),
        definition("assembly-other/definition/led", "Unrelated LED snapshot"),
        definition("replacement-definition", "Replacement LED"),
    ];
    document
        .parts
        .push(key_part("key-part", "K1", KEY, Vec2::default()));
    let mut matrix =
        fixture::single_key_matrix("matrix", "key-part", KEY, "board", 19.0, Vec2::default());
    matrix.name = Some("Keys".into());
    matrix.cells[0].assemblies = vec![MatrixAssembly {
        id: "led".into(),
        definition_id: "assembly-current/definition/led".into(),
        offset: Vec2::default(),
        rotation: None,
        side: None,
    }];
    document.matrices.push(matrix);
    (document, key_context("matrix", 0, 0))
}

/// Add key r0c`column` to the fixture matrix with a local offset, at `x` on the board.
fn push_key(document: &mut ProjectDoc, column: u32, offset: Vec2, x: f64) {
    let key_id = format!("matrix/matrix/r0c{column}");
    document.boards[0].part_ids.push(key_id.clone());
    let matrix = &mut document.matrices[0];
    matrix.columns = column + 1;
    matrix.part_ids.push(key_id.clone());
    let mut cell = matrix_cell(0, column, KEY);
    cell.offset = Some(offset);
    cell.assemblies_local = Some(true);
    matrix.cells.push(cell);
    let reference = format!("K{}", column + 1);
    document
        .parts
        .push(key_part(&key_id, &reference, KEY, Vec2 { x, y: 0.0 }));
}

fn two_key_fixture(include_standalone: bool) -> (ProjectDoc, TreeContext) {
    let (mut document, selected) = fixture();
    let first_key = "matrix/matrix/r0c0";
    document.boards[0].part_ids[0] = first_key.into();
    document.matrices[0].part_ids[0] = first_key.into();
    document.parts[0].id = first_key.into();
    push_key(&mut document, 1, Vec2 { x: 6.0, y: 7.0 }, 19.0);
    if include_standalone {
        document.boards[0].part_ids.push("standalone-part".into());
        let at = Vec2 { x: 40.0, y: 0.0 };
        document
            .parts
            .push(key_part("standalone-part", "U1", KEY, at));
    }
    (document, selected)
}

fn three_key_fixture() -> (ProjectDoc, TreeContext) {
    let (mut document, selected) = two_key_fixture(false);
    push_key(&mut document, 2, Vec2 { x: 12.0, y: 0.0 }, 38.0);
    (document, selected)
}

/// Append a 1x1 matrix `id` with its key part `matrix/<id>/r0c0` at `at`, anchored by
/// `anchor`.
fn push_single_key_matrix(
    document: &mut ProjectDoc,
    id: &str,
    reference: &str,
    at: Vec2,
    anchor: Option<MatrixAnchor>,
) {
    let key_id = format!("matrix/{id}/r0c0");
    document.boards[0].part_ids.push(key_id.clone());
    document.parts.push(key_part(&key_id, reference, KEY, at));
    let mut matrix = fixture::single_key_matrix(id, &key_id, KEY, "board", 19.0, Vec2::default());
    matrix.anchor = anchor;
    document.matrices.push(matrix);
}

/// A second 1-key matrix anchored to the first key, with rotation Follow on or off. Both
/// matrices join one Layout, which anchor validation requires.
fn anchored_matrix_fixture(rotation_followed: bool) -> (ProjectDoc, TreeContext) {
    let (mut document, _) = fixture();
    document
        .layouts
        .push(layout("layout", &["matrix", "matrix-b"]));
    let anchor = fixture::key_anchor("matrix", rotation_followed);
    let at = Vec2 { x: 60.0, y: 0.0 };
    push_single_key_matrix(&mut document, "matrix-b", "K2", at, Some(anchor));
    document.matrices[1].name = Some("Anchored".into());
    (document, matrix_context("matrix-b"))
}

/// A second row under the default 1x1 key fixture, so carried row edits have a later key to
/// move. The matrix frame stays identity: no splay, no mirror, origin zero, so world deltas
/// equal matrix deltas exactly.
fn two_row_fixture() -> (ProjectDoc, TreeContext) {
    let (mut document, selected) = fixture();
    let second_key = "matrix/matrix/r1c0";
    document.matrices[0].rows = 2;
    document.matrices[0].part_ids.push(second_key.into());
    document.boards[0].part_ids.push(second_key.into());
    document
        .parts
        .push(key_part(second_key, "K2", KEY, Vec2::default()));
    (document, selected)
}

/// A second column over the two-row fixture, so a key Shift has a same-column later key to
/// carry to and a neighbouring column that must stay still.
fn shift_grid_fixture() -> (ProjectDoc, TreeContext) {
    let (mut document, selected) = two_row_fixture();
    document.matrices[0].columns = 2;
    for key in ["matrix/matrix/r0c1", "matrix/matrix/r1c1"] {
        document.matrices[0].part_ids.push(key.into());
        document.boards[0].part_ids.push(key.into());
        let reference = format!("K{key}");
        document
            .parts
            .push(key_part(key, &reference, KEY, Vec2::default()));
    }
    (document, selected)
}

fn numeric_field(root: &web_sys::Element, label: &str) -> HtmlInputElement {
    by_role_name(root, "spinbutton", label)
        .unwrap_or_else(|| panic!("the {label} field is mounted"))
        .unchecked_into()
}

fn field_number(root: &web_sys::Element, label: &str) -> f64 {
    numeric_field(root, label)
        .value()
        .parse()
        .unwrap_or_else(|_| panic!("the {label} field holds a number"))
}

fn set_numeric_field(root: &web_sys::Element, label: &str, value: &str) {
    commit_value(&numeric_field(root, label), value);
}

/// Commit `value` into the `label` field and drive the edit through the Session.
async fn edit_field(probe: &Probe, root: &web_sys::Element, label: &str, value: &str) {
    set_numeric_field(root, label, value);
    probe.run().await;
}

fn button_named(root: &web_sys::Element, text: &str) -> Option<web_sys::HtmlElement> {
    query_all(root, "button")
        .into_iter()
        .find(|button| button.text_content().as_deref() == Some(text))
        .map(|button| button.unchecked_into())
}

fn click_button(root: &web_sys::Element, text: &str) {
    button_named(root, text)
        .unwrap_or_else(|| panic!("the Inspector offers {text}"))
        .click();
}

fn has_exact_text(root: &web_sys::Element, expected: &str) -> bool {
    query_all(root, "*")
        .iter()
        .any(|element| element.text_content().as_deref() == Some(expected))
}

fn alert_text(root: &web_sys::Element) -> Vec<String> {
    query_all(root, "[role='alert']")
        .iter()
        .filter_map(|alert| alert.text_content())
        .collect()
}

/// Select `option` the way the primitive selects: a complete pointer gesture, not a synthetic
/// click.
fn choose_option(option: &web_sys::Element) {
    for phase in ["pointerdown", "pointerup"] {
        let pointer = web_sys::PointerEvent::new(phase).unwrap();
        pointer.init_event_with_bubbles_and_cancelable(phase, true, true);
        option.dispatch_event(&pointer).unwrap();
    }
}

fn close(left: Vec2, right: Vec2) -> bool {
    (left.x - right.x).abs() < 1e-9 && (left.y - right.y).abs() < 1e-9
}

fn moved_by(before: Vec2, after: Vec2, dx: f64, dy: f64) -> bool {
    close(
        Vec2 {
            x: before.x + dx,
            y: before.y + dy,
        },
        after,
    )
}

#[wasm_bindgen_test]
async fn additive_canvas_selection_keeps_key_properties_for_same_matrix_keys() {
    let (document, context) = two_key_fixture(false);
    let (probe, root) = mount_probe(document, context, &["matrix/matrix/r0c0"]).await;
    let adapter = probe.adapter.borrow().as_ref().unwrap().clone();
    let hit_context = key_context("matrix", 0, 1);
    crate::presentation::selection::submit_matrix_cell_selection(
        &probe.runtime,
        &adapter,
        &probe.selected.scope,
        1,
        crate::presentation::selection::MatrixCellSelection {
            matrix_id: "matrix".into(),
            target_part_id: "matrix/matrix/r0c1".into(),
            hit_context: &hit_context,
            context: TreeContext::Component {
                part_id: Some("matrix/matrix/r0c1".into()),
                matrix_id: None,
                row: None,
                column: None,
                assembly_id: None,
            },
            mode: SelectionMode::Toggle,
        },
    )
    .expect("live canvas key hit submits additive selection");
    probe.settle().await;
    assert_eq!(
        probe.selection(),
        Some(hit_context),
        "a same-matrix multi-key selection keeps its current key as the Inspector context"
    );
    assert_eq!(
        numeric_field(&root, "Local X").value(),
        "6",
        "the key-local Inspector follows the most recently toggled key"
    );
    assert!(
        by_role_name(&root, "combobox", "Key Assembly").is_some(),
        "the mounted key properties retain Key Assembly controls"
    );
    let all_enabled = |enabled: bool| {
        probe
            .matrix("matrix")
            .cells
            .iter()
            .all(|cell| cell.enabled == enabled)
    };
    // Bulk disable, bulk enable, then one Undo and one Redo each reverse the whole bulk edit.
    by_role_name(&root, "checkbox", "Key enabled")
        .unwrap()
        .click();
    probe.settle().await;
    support::run_pending(&probe.runtime).await;
    wait_until("disabling a multi-key selection disables every key", || {
        all_enabled(false)
    })
    .await;
    probe.settle().await;
    by_role_name(&root, "checkbox", "Key enabled")
        .unwrap()
        .click();
    support::run_pending(&probe.runtime).await;
    wait_until("the same selection re-enables every disabled key", || {
        all_enabled(true)
    })
    .await;
    probe.undo().await;
    wait_until("one Undo reverses the entire bulk enable", || {
        all_enabled(false)
    })
    .await;
    probe.redo().await;
    wait_until("one Redo restores the entire bulk enable", || {
        all_enabled(true)
    })
    .await;
}

#[wasm_bindgen_test]
async fn additive_mixed_component_selection_keeps_first_part_group_context() {
    let (document, context) = two_key_fixture(true);
    let (probe, root) = mount_probe(document, context, &["matrix/matrix/r0c0"]).await;
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
        &probe.selected.scope,
        1,
        part_context,
        SelectionMode::Toggle,
        Vec::new(),
    )
    .expect("live canvas key hit submits additive selection");
    probe.settle().await;
    assert!(
        matches!(
            probe.selection(),
            Some(TreeContext::Component { part_id: Some(part_id), .. }) if part_id == "matrix/matrix/r0c0"
        ),
        "mixed matrix-key and standalone selection keeps the first selected part as its group anchor"
    );
    assert!(
        by_role_name(&root, "spinbutton", "Local X").is_none(),
        "mixed selection does not show a key-local Inspector"
    );
}

#[wasm_bindgen_test]
async fn toggle_removal_keeps_the_inspector_and_commands_aimed_at_remaining_keys() {
    let (document, context) = three_key_fixture();
    let (probe, root) = mount_probe(document, context, &["matrix/matrix/r0c0"]).await;
    let ctrl_click = async |column: u32| {
        let target = query(
            &root,
            &format!("g[data-part-id='matrix/matrix/r0c{column}']"),
        )
        .expect("mounted matrix canvas contains the key hit target");
        fixture::dispatch_pointer(&target, "pointerdown", true, false, 1, (0, 0));
        probe.settle().await;
    };
    ctrl_click(1).await;
    ctrl_click(2).await;
    // Removing C leaves A and B selected; C is still a live key, so the Inspector must be
    // re-anchored to a member that remains selected. Removing B then leaves A.
    for (column, removed) in [(2, "C"), (1, "B")] {
        ctrl_click(column).await;
        assert_eq!(
            probe.selection(),
            Some(key_context("matrix", 0, 0)),
            "removing {removed} must keep the key Inspector on the remaining selected A"
        );
    }
    // Its edit command changes A's cell rather than the deselected B cell.
    set_numeric_field(&root, "Local X", "5");
    support::run_pending(&probe.runtime).await;
    wait_until("the remaining key accepts its local X edit", || {
        probe.cell(0).offset.map(|offset| offset.x) == Some(5.0)
    })
    .await;
    assert_eq!(
        probe.cell(1).offset.map(|offset| offset.x),
        Some(6.0),
        "the deselected B cell is unchanged"
    );
}

#[wasm_bindgen_test]
async fn pending_reset_disables_its_button_and_rejects_a_retained_duplicate() {
    let (mut document, context) = fixture();
    document.matrices[0].cells[0].offset = Some(Vec2 { x: 3.0, y: 0.0 });
    let (probe, root) = mount_probe(document, context, &["key-part"]).await;
    let retained = probe.mount();
    let projection = retained.projection.as_ref().unwrap();
    let button = by_role_name(&root, "button", "Reset local transform").unwrap();
    let input = numeric_field(&root, "Local X");
    let (entered, release) = support::gate_next_core_reply(&probe.runtime);
    button.click();
    support::drive_pending(&probe.runtime);
    entered.await.expect("the reset reaches Core");
    probe.settle().await;
    let disabled_while_pending = button.has_attribute("disabled");
    assert!(
        !input.disabled(),
        "field edits stay available while an action is pending"
    );
    commit_value(&input, "5");
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
    let ready = || {
        matches!(
            probe.runtime.model().lifecycle,
            boardstudio_application::Lifecycle::Ready
        )
    };
    for _ in 0..100 {
        support::run_pending(&probe.runtime).await;
        if ready() {
            break;
        }
        probe.settle().await;
    }
    assert!(
        ready(),
        "the reset and queued field edit finish through the real Session"
    );
    probe.settle().await;
    assert!(
        disabled_while_pending,
        "the reset button disables while its action is pending"
    );
    assert_eq!(
        probe.cell(0).offset,
        Some(Vec2 { x: 5.0, y: 0.0 }),
        "a retained duplicate reset must not undo the queued field edit"
    );
    let live_button = by_role_name(&root, "button", "Reset local transform").unwrap();
    assert!(
        !live_button.has_attribute("disabled"),
        "the live reset button is available after settlement; captured connected: {}, lifecycle: {:?}",
        button.is_connected(),
        probe.runtime.model().lifecycle,
    );
    assert!(query(&root, "[role='alert']").is_none());
}

/// A failed numeric edit keeps a newer typed draft, or (unchanged) restores the accepted
/// text while keeping its inline error.
#[wasm_bindgen_test]
async fn failed_numeric_edit_keeps_a_newer_draft_or_its_inline_error() {
    const FAILURE: &str = "The transform change could not be applied: injected transform failure";
    for retype in [true, false] {
        let (document, context) = fixture();
        let (probe, root) = mount_probe(document, context, &["key-part"]).await;
        let input = numeric_field(&root, "Local X");
        support::fail_next_core_reply(&probe.runtime, "injected transform failure");
        commit_value(&input, "5.00");
        if retype {
            // Type again before the failed Core reply is observed by the mounted panel.
            type_value(&input, "7.00");
        }
        probe.run().await;
        assert_eq!(text_of(&root, "[role='alert']"), FAILURE);
        if retype {
            assert_eq!(
                input.value(),
                "7.00",
                "failure must not restore older submitted text"
            );
            continue;
        }
        assert_eq!(
            input.value(),
            "0",
            "the unchanged failed draft restores accepted text"
        );
        assert_eq!(input.get_attribute("aria-invalid").as_deref(), Some("true"));
        let document = web_sys::window().unwrap().document().unwrap();
        let descriptions: Vec<_> = input
            .get_attribute("aria-describedby")
            .expect("units and failure describe the field")
            .split_whitespace()
            .map(|id| {
                document
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
                .any(|text| text.contains("injected transform failure")),
            "restoring the accepted projection must not clear the helper-owned failure"
        );
    }
}

#[wasm_bindgen_test]
async fn mounted_attached_component_choices_are_current_item_scoped_and_emit_the_selected_replacement()
 {
    let (document, context) = fixture();
    let (probe, root) = mount_probe(document, context, &["key-part"]).await;
    let selector =
        by_role_name(&root, "combobox", "Replace led").expect("actual attached LED chooser");
    selector.focus().unwrap();
    fixture::press_key(&selector, "ArrowDown");
    wait_until("the attached component options open", || {
        by_role_name(&root, "option", "Current LED snapshot").is_some()
    })
    .await;
    assert!(
        by_role_name(&root, "option", "Replacement LED").is_some(),
        "include the actual replacement definition"
    );
    assert!(
        by_role_name(&root, "option", "Unrelated LED snapshot").is_none(),
        "exclude an unrelated assembly snapshot"
    );
    choose_option(&by_role_name(&root, "option", "Replacement LED").unwrap());
    probe.run().await;
    assert!(
        probe
            .cell(0)
            .assemblies
            .iter()
            .any(|assembly| assembly.id == "led"
                && assembly.definition_id == "replacement-definition"),
        "the accepted key carries the selected replacement"
    );
}

#[wasm_bindgen_test]
async fn deleting_selected_keys_removes_cells_and_assemblies_and_undo_restores_them() {
    let (document, context) = two_key_fixture(false);
    let selected = ["matrix/matrix/r0c0", "matrix/matrix/r0c1"];
    let (probe, root) = mount_probe(document, context, &selected).await;
    by_role_name(&root, "button", "Delete selected keys")
        .unwrap()
        .click();
    probe.run().await;
    let accepted = probe.accepted();
    assert!(
        accepted.scene.matrix_scenes[0].cells.is_empty(),
        "deleted cells have no snapping/ghost positions"
    );
    assert!(
        accepted.document.parts.is_empty(),
        "switches and attached assemblies are removed together"
    );
    probe.undo().await;
    assert_eq!(
        probe.accepted().scene.matrix_scenes[0].cells.len(),
        2,
        "one Undo restores the complete selection"
    );
}

/// Begin one edit ticket that submits `operation` built against the accepted snapshot.
fn begin_ticket(
    runtime: &Rc<Runtime>,
    label: &'static str,
    operation: impl Fn(&AcceptedSnapshot) -> boardstudio_model::model::EditOperation + 'static,
) -> boardstudio_web_runtime::edit_ticket::EditTicket {
    boardstudio_web_runtime::edit_ticket::EditTicket::begin(
        runtime,
        label,
        None,
        boardstudio_application::EditResolver::new(label, move |accepted| {
            boardstudio_application::Resolution::submit(vec!["matrix".into()], operation(accepted))
        }),
    )
}

fn landed(ticket: &boardstudio_web_runtime::edit_ticket::EditTicket) -> bool {
    matches!(
        ticket.settlement(true),
        boardstudio_web_runtime::edit_ticket::Settlement::Landed { .. }
    )
}

#[wasm_bindgen_test]
async fn queued_key_edit_retires_after_column_deletion_instead_of_editing_reindexed_survivor() {
    let (document, context) = two_key_fixture(false);
    let (probe, root) = mount_probe(document, context, &["matrix/matrix/r0c0"]).await;
    let (entered, release) = support::gate_next_core_reply(&probe.runtime);
    let removal = begin_ticket(&probe.runtime, "delete-column-test", |_| {
        boardstudio_model::model::EditOperation::RemoveMatrixColumn {
            matrix_id: "matrix".into(),
            column: 0,
        }
    });
    support::drive_pending(&probe.runtime);
    entered
        .await
        .expect("deletion reaches Core while old key remains accepted");
    set_numeric_field(&root, "Local X", "77");
    release.send(()).expect("release removal");
    probe.run().await;
    assert!(landed(&removal));
    assert_eq!(
        probe.cell(0).offset.unwrap().x,
        6.0,
        "the queued deleted-key edit cannot alter the survivor reindexed into its coordinate"
    );
}

#[wasm_bindgen_test]
async fn mixed_enabled_selection_displays_unchecked_and_enables_every_selected_cell() {
    let (document, context) = two_key_fixture(false);
    let selected = ["matrix/matrix/r0c0", "matrix/matrix/r0c1"];
    let (probe, root) = mount_probe(document, context, &selected).await;
    let disabled = begin_ticket(&probe.runtime, "disable-one-test", |accepted| {
        let mut matrix = accepted.document.matrices[0].clone();
        matrix.cells[1].enabled = false;
        boardstudio_model::model::EditOperation::SetMatrix {
            matrix,
            definitions: None,
        }
    });
    support::run_pending(&probe.runtime).await;
    assert!(landed(&disabled));
    probe.settle().await;
    let enabled = by_role_name(&root, "checkbox", "Key enabled").unwrap();
    assert_eq!(
        enabled.get_attribute("aria-checked").as_deref(),
        Some("false"),
        "mixed selection does not imply every key is enabled"
    );
    enabled.click();
    probe.run().await;
    assert!(probe.matrix("matrix").cells.iter().all(|cell| cell.enabled));
}

#[wasm_bindgen_test]
async fn row_and_column_delete_controls_submit_structural_removal() {
    for (context, label, remaining_columns) in [
        (column_context(1), "Delete column", Some(1)),
        (row_context(0), "Delete row", None),
    ] {
        let (document, _) = two_key_fixture(false);
        let (probe, root) = mount_probe(document, context, &[]).await;
        let button = by_role_name(&root, "button", label).unwrap();
        button.click();
        button.click();
        probe.run().await;
        let accepted = probe.accepted();
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
    }
}

fn angled_column_fixture(splay: f64, rotation: f64) -> ProjectDoc {
    let (mut document, _) = two_key_fixture(false);
    document.matrices[0]
        .column_layout
        .resize_with(1, Default::default);
    document.matrices[0].column_layout[0].splay = splay;
    document.matrices[0].column_layout[0].rotation = rotation;
    document
}

#[wasm_bindgen_test]
async fn column_angle_scope_switches_display_and_discards_the_other_fields_draft() {
    let (probe, root) = mount_probe(angled_column_fixture(7.0, 10.0), column_context(0), &[]).await;
    let input: HtmlInputElement = query(&root, "input[aria-label='Splay']")
        .unwrap()
        .unchecked_into();
    assert_eq!(input.value(), "10", "This column displays Column rotation");
    let before = probe.accepted().document;
    type_value(&input, "99");
    probe.settle().await;
    let mut affect = probe.mount().splay_affect;
    affect.set(boardstudio_model::model::MatrixSplayAffect::Following);
    probe.settle().await;
    assert_eq!(
        input.value(),
        "7",
        "switching fields resets the dirty draft"
    );
    assert_eq!(probe.accepted().document, before);
    affect.set(boardstudio_model::model::MatrixSplayAffect::Column);
    probe.settle().await;
    assert_eq!(input.value(), "10");
    edit_field(&probe, &root, "Splay", "0").await;
    let record = probe.matrix("matrix").column_layout[0];
    assert_eq!((record.rotation, record.splay), (0.0, 7.0));
}

#[wasm_bindgen_test]
async fn column_angle_equality_uses_the_requested_field_in_the_accepted_snapshot() {
    use super::super::matrix_transform_controller::transform_edit_resolver;
    use boardstudio_application::Resolution;
    use boardstudio_model::model::MatrixSplayAffect;
    let runtime = support::new_runtime();
    support::open_document(&runtime, angled_column_fixture(0.0, 10.0)).await;
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

#[wasm_bindgen_test]
async fn inspector_stagger_lands_as_a_placement_edit_that_moves_later_columns_with_one_undo() {
    let (document, _) = two_key_fixture(false);
    let (probe, root) = mount_probe(document, column_context(0), &[]).await;
    assert!(has_exact_text(&root, "Column 1"));
    let second_column_y = || probe.key_at(0, 1).y;
    // The fixture's key poses were never laid out by Core; the first edit syncs them, so
    // measure the second change from that synced state.
    edit_field(&probe, &root, "Stagger", "1").await;
    let before = second_column_y();
    let revision = probe.revision();
    edit_field(&probe, &root, "Stagger", "4").await;
    assert_eq!(
        probe.matrix("matrix").column_layout[0].stagger,
        4.0,
        "the Inspector value is the matrix's column stagger"
    );
    let axis = probe.accepted().scene.matrix_scenes[0].columns[0].stagger_axis;
    let moved = second_column_y() - before;
    assert!(
        (moved - 3.0 * axis.y).abs() < 1e-9 && axis.y != 0.0,
        "the later column moved 3 mm along Core's stagger axis ({moved} vs {})",
        3.0 * axis.y
    );
    assert_eq!(probe.revision(), revision + 1, "one accepted edit");
    probe.undo().await;
    assert_eq!(
        probe.matrix("matrix").column_layout[0].stagger,
        1.0,
        "one Undo reverts the whole second stagger edit"
    );
    assert!((second_column_y() - before).abs() < 1e-9);
}

fn offset_column_fixture(splay: f64) -> ProjectDoc {
    let (mut document, _) = two_key_fixture(false);
    let column_layout = &mut document.matrices[0].column_layout;
    column_layout.resize_with(1, Default::default);
    column_layout[0].offset = Vec2 { x: 0.0, y: -9.525 };
    column_layout[0].splay = splay;
    document
}

#[wasm_bindgen_test]
async fn convert_to_stagger_appears_only_for_a_nonzero_offset_y_and_is_one_undo_step() {
    // No offset Y: no action.
    let (document, _) = two_key_fixture(false);
    let (_probe, root) = mount_probe(document, column_context(0), &[]).await;
    assert!(button_named(&root, "Convert to stagger").is_none());
    drop(root);

    let (probe, root) = mount_probe(offset_column_fixture(0.0), column_context(0), &[]).await;
    let button = button_named(&root, "Convert to stagger").expect("the action is mounted");
    assert!(!button.has_attribute("disabled"));
    let revision = probe.revision();
    button.click();
    probe.run().await;
    let matrix = probe.matrix("matrix");
    assert_eq!(probe.revision(), revision + 1, "one accepted edit");
    let staggers = |matrix: &Matrix| {
        matrix
            .column_layout
            .iter()
            .map(|record| record.stagger)
            .collect::<Vec<_>>()
    };
    assert_eq!(staggers(&matrix), vec![-9.525, 9.525]);
    assert_eq!(matrix.column_layout[0].offset.y, 0.0);
    // The probe host re-renders only when its version signal moves.
    probe.settle().await;
    assert_eq!(field_number(&root, "Stagger"), -9.525);
    assert_eq!(field_number(&root, "Offset Y"), 0.0);
    assert!(
        button_named(&root, "Convert to stagger").is_none(),
        "nothing left to convert"
    );
    probe.undo().await;
    let undone = probe.matrix("matrix");
    assert_eq!(undone.column_layout[0].offset.y, -9.525);
    assert!(staggers(&undone).iter().all(|value| *value == 0.0));
}

#[wasm_bindgen_test]
async fn convert_to_stagger_is_disabled_with_core_reason_on_a_splayed_column() {
    let (probe, root) = mount_probe(offset_column_fixture(10.0), column_context(0), &[]).await;
    let button = button_named(&root, "Convert to stagger").expect("the action is mounted");
    assert!(
        button.has_attribute("disabled"),
        "a splayed column cannot convert"
    );
    let reason =
        boardstudio_model::model::convert_offset_available(&probe.matrix("matrix"), 0).unwrap_err();
    assert!(has_exact_text(&root, &reason));
    assert_eq!(
        reason,
        "Converting requires zero Splay, Column rotation and carried Bend"
    );
}

#[wasm_bindgen_test]
async fn collapsing_column_sections_preserves_drafts_nodes_and_edit_owner() {
    let (document, _) = two_key_fixture(false);
    let (probe, root) = mount_probe(document, column_context(0), &[]).await;
    let accepted = probe.revision();
    let position = by_role_name(&root, "button", "Position and rotation")
        .expect("position section is a mounted disclosure");
    let splay = by_role_name(&root, "button", "Splay and origin")
        .expect("splay section is a mounted disclosure");
    let expanded = |button: &web_sys::HtmlElement| button.get_attribute("aria-expanded");
    assert_eq!(
        expanded(&splay).as_deref(),
        Some("false"),
        "specialist origin controls start collapsed"
    );
    splay.click();
    let input = numeric_field(&root, "Stagger");
    type_value(&input, "");
    probe.settle().await;
    position.click();
    wait_until("position section collapses", || {
        expanded(&position).as_deref() == Some("false")
    })
    .await;
    assert_eq!(
        expanded(&splay).as_deref(),
        Some("true"),
        "sections can be collapsed independently"
    );
    probe.settle().await;
    assert_eq!(
        expanded(&position).as_deref(),
        Some("false"),
        "unrelated renders retain collapsed state"
    );
    assert!(
        input.is_same_node(
            by_role_name(&root, "spinbutton", "Stagger")
                .as_ref()
                .map(|node| node.as_ref())
        ),
        "collapse never remounts field ownership"
    );
    position.click();
    probe.settle().await;
    assert_eq!(input.value(), "");
    assert_eq!(
        probe.revision(),
        accepted,
        "toggling sections never submits edits"
    );
}

#[wasm_bindgen_test]
async fn canvas_dimensions_preserve_inspector_drafts_and_share_undo() {
    let (document, _) = two_key_fixture(false);
    let (probe, root) = mount_with(document, column_context(0), &[], true).await;
    let inspector = query(&root, "#inspector-geometry-test").unwrap();
    let canvas = query(&root, "#canvas-geometry-test").unwrap();
    let draft = numeric_field(&inspector, "Stagger");
    type_value(&draft, "");
    probe.settle().await;
    let before = probe.revision();
    edit_field(&probe, &canvas, "Stagger", "4").await;
    assert_eq!(probe.matrix("matrix").column_layout[0].stagger, 4.0);
    assert_eq!(probe.revision(), before + 1);
    assert_eq!(
        draft.value(),
        "",
        "canvas edits preserve an unfinished inspector draft"
    );
    probe.undo().await;
    let stagger = probe
        .matrix("matrix")
        .column_layout
        .first()
        .map_or(0.0, |record| record.stagger);
    assert_eq!(stagger, 0.0);
    assert_eq!(draft.value(), "");
}

/// What a carried row or key field case edits, and where its value lands.
#[derive(Clone, Copy, Debug)]
enum Carried {
    RowShift,
    KeyShift,
    RowBend,
    KeyBend,
}

/// Ticket 23: the Row and Key Inspectors name the carried Shift and Bend fields. Setting one
/// moves (Shift X) or turns (Bend) the edited row or key and the later keys it carries to
/// through Core's running frame as one accepted edit; a key's neighbouring column stays
/// still, and one Undo restores every key exactly.
#[wasm_bindgen_test]
async fn carried_row_and_key_fields_move_their_keys_through_cores_frame_with_one_undo() {
    for (case, label, value) in [
        (Carried::RowShift, "Shift X", 4.0),
        (Carried::KeyShift, "Shift X", 3.0),
        (Carried::RowBend, "Bend", 10.0),
        (Carried::KeyBend, "Bend", 10.0),
    ] {
        let (document, key) = match case {
            Carried::RowShift | Carried::RowBend => two_row_fixture(),
            Carried::KeyShift | Carried::KeyBend => shift_grid_fixture(),
        };
        let context = match case {
            Carried::RowShift | Carried::RowBend => row_context(0),
            Carried::KeyShift | Carried::KeyBend => key,
        };
        let (probe, root) = mount_probe(document, context, &[]).await;
        let keys: &[(u32, u32)] = match case {
            Carried::RowShift | Carried::RowBend => &[(0, 0), (1, 0)],
            Carried::KeyShift | Carried::KeyBend => &[(0, 0), (1, 0), (0, 1), (1, 1)],
        };
        let poses = || {
            keys.iter()
                .map(|&(row, column)| probe.scene_pose("matrix", row, column))
                .collect::<Vec<_>>()
        };
        let before = poses();
        let revision = probe.revision();
        edit_field(&probe, &root, label, &value.to_string()).await;
        // The stored field as (x, y) for a Shift and (degrees, 0) for a Bend.
        let stored = || {
            let matrix = probe.matrix("matrix");
            let row = matrix.row_layout.first().copied().unwrap_or_default();
            let cell = matrix
                .cells
                .into_iter()
                .find(|cell| cell.row == 0 && cell.column == 0)
                .expect("the edited key keeps its cell");
            match case {
                Carried::RowShift => (row.shift.x, row.shift.y),
                Carried::KeyShift => cell.shift.map_or((0.0, 0.0), |shift| (shift.x, shift.y)),
                Carried::RowBend => (row.bend, 0.0),
                Carried::KeyBend => (cell.bend.unwrap_or_default(), 0.0),
            }
        };
        assert_eq!(
            stored(),
            (value, 0.0),
            "{case:?}: the Inspector value is the carried field"
        );
        assert_eq!(
            probe.revision(),
            revision + 1,
            "{case:?}: one accepted edit"
        );
        for ((&(row, column), from), to) in keys.iter().zip(&before).zip(poses()) {
            // Column 0 holds the edited row or key and everything it carries to.
            let carried = if column == 0 { value } else { 0.0 };
            let ok = match case {
                Carried::RowShift | Carried::KeyShift => moved_by(from.at, to.at, carried, 0.0),
                Carried::RowBend | Carried::KeyBend => (to.rotation - carried).abs() < 1e-9,
            };
            assert!(
                ok,
                "{case:?}: key ({row}, {column}) carries {carried}, got {to:?}"
            );
        }
        probe.undo().await;
        assert_eq!(
            stored(),
            (0.0, 0.0),
            "{case:?}: one Undo clears the carried field"
        );
        for ((&(row, column), from), to) in keys.iter().zip(&before).zip(poses()) {
            assert!(
                close(from.at, to.at) && (from.rotation - to.rotation).abs() < 1e-9,
                "{case:?}: one Undo restores key ({row}, {column}) exactly"
            );
        }
    }
}

/// Ticket 23: the Column Inspector names the inherited Spread step and the Row Inspector the
/// Padding step. Setting one moves its own column or row past the 19 mm pitch as one accepted
/// edit without moving column or row 0, and one Undo restores both exactly.
#[wasm_bindgen_test]
async fn spread_and_padding_move_only_their_own_column_or_row_with_one_undo() {
    for (label, spread) in [("Spread", true), ("Padding", false)] {
        let (document, context, moved_key) = if spread {
            (two_key_fixture(false).0, column_context(1), (0, 1))
        } else {
            (two_row_fixture().0, row_context(1), (1, 0))
        };
        let (probe, root) = mount_probe(document, context, &[]).await;
        let (moved, still) = (
            || probe.key_at(moved_key.0, moved_key.1),
            || probe.key_at(0, 0),
        );
        let (before_moved, before_still) = (moved(), still());
        let revision = probe.revision();
        edit_field(&probe, &root, label, "24").await;
        let matrix = probe.matrix("matrix");
        let stored = if spread {
            matrix.column_layout.get(1).and_then(|record| record.spread)
        } else {
            matrix.row_layout.get(1).and_then(|record| record.padding)
        };
        assert_eq!(
            stored,
            Some(24.0),
            "the Inspector value is the explicit {label}"
        );
        assert_eq!(probe.revision(), revision + 1, "one accepted edit");
        let (dx, dy) = if spread { (5.0, 0.0) } else { (0.0, 5.0) };
        assert!(
            moved_by(before_moved, moved(), dx, dy),
            "its own {label} steps out the full value past the 19 mm pitch"
        );
        assert!(
            close(before_still, still()),
            "index 0 does not follow a later {label}"
        );
        probe.undo().await;
        assert!(
            close(before_moved, moved()) && close(before_still, still()),
            "one Undo restores both exactly"
        );
    }
}

/// Ticket 23: column 0 has no Spread step and row 0 no Padding step, and the Inspector says
/// why instead of leaving a silent gap.
#[wasm_bindgen_test]
async fn index_zero_hides_spread_and_padding_with_an_accessible_reason() {
    for (document, context, label, reason) in [
        (
            two_key_fixture(false).0,
            column_context(0),
            "Spread",
            "Spread starts at column 1. Column 0 steps by pitch.",
        ),
        (
            two_row_fixture().0,
            row_context(0),
            "Padding",
            "Padding starts at row 1. Row 0 sits at the column base.",
        ),
    ] {
        let (_probe, root) = mount_probe(document, context, &[]).await;
        assert!(
            query(&root, &format!("input[aria-label='{label}']")).is_none(),
            "index 0 has no {label} field"
        );
        assert_eq!(
            text_of(&root, "[role=note]"),
            reason,
            "the reason names the rule"
        );
    }
}

/// Ticket 23: a default Spread reads the pitch with a pitch badge; an explicit Spread reads
/// its own value with no badge.
#[wasm_bindgen_test]
async fn default_spread_shows_the_pitch_badge_and_explicit_spread_does_not() {
    let (document, _) = two_key_fixture(false);
    let (probe, root) = mount_probe(document, column_context(1), &[]).await;
    let italic = || {
        query(&root, "input[aria-label='Spread']")
            .expect("column 1 has a Spread field")
            .get_attribute("class")
            .unwrap_or_default()
            .split_whitespace()
            .any(|class| class == "italic")
    };
    assert_eq!(
        numeric_field(&root, "Spread").value(),
        "19",
        "the default Spread reads the pitch"
    );
    assert!(
        italic(),
        "the inherited pitch is visually distinguished from an explicit value"
    );
    assert!(
        query(&root, "[data-pitch-badge]").is_some(),
        "the default carries a pitch badge"
    );
    edit_field(&probe, &root, "Spread", "24").await;
    assert!(
        query(&root, "[data-pitch-badge]").is_none(),
        "an explicit Spread drops the badge"
    );
    assert!(!italic(), "an explicit value is not styled as inherited");
}

/// Ticket 23: Carried and This-only sections group the named fields with scope explanations.
#[wasm_bindgen_test]
async fn carried_and_this_only_sections_group_the_named_fields() {
    let (document, _) = two_key_fixture(false);
    let (_probe, root) = mount_probe(document, column_context(1), &[]).await;
    let headings = query_all(&root, "summary, h3")
        .iter()
        .filter_map(|heading| heading.text_content())
        .collect::<Vec<_>>();
    for section in ["Carried", "This column only"] {
        assert!(
            headings.iter().any(|heading| heading == section),
            "{section} is mounted"
        );
    }
    for field in ["Spread", "Stagger"] {
        assert!(
            query(&root, &format!("input[aria-label='{field}']")).is_some(),
            "{field} stays a named field"
        );
    }
}

/// Ticket 23: a committed edit names its changed field and promises one Undo entry; the line
/// clears when the next edit submits.
#[wasm_bindgen_test]
async fn committed_shift_names_the_changed_field_with_one_undo() {
    let (document, _) = two_row_fixture();
    let (probe, root) = mount_probe(document, row_context(0), &[]).await;
    let status = || query(&root, "[data-matrix-change-status]");
    assert!(
        status().is_none(),
        "no change is announced before the first commit"
    );
    edit_field(&probe, &root, "Shift X", "4").await;
    assert_eq!(
        status().and_then(|status| status.text_content()).as_deref(),
        Some("Changed Shift X · one Undo entry"),
        "the commit names the changed field"
    );
    set_numeric_field(&root, "Shift X", "5");
    probe.settle().await;
    assert!(
        status().is_none(),
        "a newer submit retires the previous announcement"
    );
    probe.run().await;
}
