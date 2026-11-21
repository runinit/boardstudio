//! Mounted browser tests for the transform toolbar, its menus and the rail overlays. This
//! module is the shared harness: one host per overlay kind, mounted the way the production
//! canvas mounts them, over the real in-process Session and Core.
use super::*;
use crate::presentation::objects::matrix_edit_test_support as fixture;
use boardstudio_web_test_runtime::project_name_test_support as support;
use boardstudio_web_ui_shared::test_mount::MountedFixture;
use boardstudio_web_ui_shared::test_role::by_role_name;
use boardstudio_web_ui_shared::test_wait::wait_until;
use fixture::{
    dispatch_pointer, pump_past, pump_preview, pump_until, query, query_all, revision, text_of,
    wait_until_editable, world_to_client,
};
use wasm_bindgen_test::wasm_bindgen_test;

wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

mod anchor_pick;
mod matrix_rail;
mod menu;
mod nudge;
mod part_rail;

/// A test-owned cell a host publishes one of its signals through.
#[derive(Clone)]
struct Slot<T>(Rc<RefCell<Option<T>>>);

impl<T> Default for Slot<T> {
    fn default() -> Self {
        Self(Rc::new(RefCell::new(None)))
    }
}

impl<T> PartialEq for Slot<T> {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

impl<T: Copy> Slot<T> {
    fn get(&self) -> T {
        self.0.borrow().expect("the host published its signal")
    }
}

/// Let the overlays re-render after a dispatched event.
async fn frame() {
    gloo_timers::future::TimeoutFuture::new(60).await;
}

fn owner_of(runtime: &Rc<Runtime>) -> LayoutOwnerIdentity {
    let model = runtime.model();
    let accepted = model.accepted.as_ref().expect("an accepted document");
    LayoutOwnerIdentity {
        scope: runtime.scope(),
        token: Some(accepted.token),
        revision: Some(accepted.document.revision),
        generation: 0,
        workspace: "Layout",
    }
}

fn scoped(owner: &LayoutOwnerIdentity, context: TreeContext) -> Option<ScopedTreeContext> {
    Some(ScopedTreeContext {
        scope: owner.scope.clone().expect("a scope"),
        context,
    })
}

fn part_context(part_id: &str) -> TreeContext {
    TreeContext::Component {
        part_id: Some(part_id.into()),
        matrix_id: None,
        row: None,
        column: None,
        assembly_id: None,
    }
}

fn column(column: u32) -> TreeContext {
    TreeContext::Column {
        matrix_id: fixture::MATRIX_ID.into(),
        column,
    }
}

fn row(row: u32) -> TreeContext {
    TreeContext::Row {
        matrix_id: fixture::MATRIX_ID.into(),
        row,
    }
}

fn key(row: u32, column: u32) -> TreeContext {
    TreeContext::Key {
        matrix_id: fixture::MATRIX_ID.into(),
        row,
        column,
    }
}

fn full_follow() -> boardstudio_model::model::Follow {
    boardstudio_model::model::Follow {
        x: true,
        y: true,
        rotation: true,
    }
}

fn dep_context() -> TreeContext {
    TreeContext::Matrix {
        matrix_id: "dep".into(),
    }
}

/// A second 1x1 matrix `dep` anchored to the fixture matrix's first key, with a live key
/// part, for the matrix Move, Rotate and Anchor pick tools. Both matrices join one Layout,
/// which anchor validation requires.
fn anchored_drag_document(follow: boardstudio_model::model::Follow) -> ProjectDoc {
    use boardstudio_model::model::{ColumnLayout, Layout};
    let mut document = fixture::matrix_document();
    document.layouts.push(Layout {
        id: "layout".into(),
        name: "Main".into(),
        board_id: fixture::BOARD_ID.into(),
        matrix_ids: vec![fixture::MATRIX_ID.into(), "dep".into()],
        part_ids: vec![],
        mirror_link: None,
    });
    let origin = Vec2 { x: 0.0, y: -19.05 };
    add_single_key_matrix(&mut document, "dep", "K3", origin);
    let dep = document.matrices.last_mut().expect("the dep matrix");
    dep.name = Some("Dependent".into());
    dep.column_layout = vec![ColumnLayout::default()];
    dep.anchor = Some(fixture::key_anchor(fixture::MATRIX_ID, follow.rotation));
    if let Some(anchor) = dep.anchor.as_mut() {
        anchor.follow = follow;
    }
    document
}

/// Append a 1x1 matrix `id` with its key part `matrix/<id>/r0c0` at `origin`.
fn add_single_key_matrix(document: &mut ProjectDoc, id: &str, reference: &str, origin: Vec2) {
    let key_id = format!("matrix/{id}/r0c0");
    document.boards[0].part_ids.push(key_id.clone());
    document
        .parts
        .push(fixture::key_part(&key_id, reference, "switch:base", origin));
    document.matrices.push(fixture::single_key_matrix(
        id,
        &key_id,
        "switch:base",
        fixture::BOARD_ID,
        19.05,
        origin,
    ));
}

async fn open_anchored_drag_runtime(follow: boardstudio_model::model::Follow) -> Rc<Runtime> {
    let runtime = support::new_runtime();
    support::open_document(&runtime, anchored_drag_document(follow)).await;
    wait_until_editable(&runtime).await;
    runtime
}

/// The accepted matrix and its scene, as the canvas hands them to an overlay.
fn matrix_and_scene(runtime: &Rc<Runtime>, matrix_id: &str) -> (Matrix, MatrixScene) {
    let accepted = runtime.model().accepted.expect("an accepted document");
    let scene = accepted
        .scene
        .matrix_scenes
        .iter()
        .find(|scene| scene.matrix_id == matrix_id)
        .cloned()
        .expect("the matrix scene");
    (fixture::matrix_named(runtime, matrix_id), scene)
}

/// The Editor's transient amber preview contexts, over the accepted document.
fn provide_preview_contexts(runtime: &Rc<Runtime>) -> Signal<Option<FieldPreviewState>> {
    let preview_state = use_signal(|| None::<FieldPreviewState>);
    use_context_provider(|| FieldPreview(preview_state));
    let accepted = runtime.model().accepted.expect("an accepted document");
    use_context_provider(|| {
        Some(FieldPreviewSource {
            document: accepted.document.clone(),
            scene: accepted.scene.clone(),
        })
    });
    preview_state
}

/// Ticket 03: the matrix rail host mounts one overlay per tool into one shared svg, like the
/// production rail, so the registry lays out the real peer set. It provides the Editor's
/// amber preview contexts and publishes its visibility, so a test can unmount the overlays
/// exactly like a production scope switch.
#[derive(Props, Clone, PartialEq)]
struct MatrixRailHostProps {
    runtime: LayoutTransformRuntime,
    svg: LayoutTransformSvg,
    matrix: Matrix,
    projection: MatrixScene,
    owner: LayoutOwnerIdentity,
    context: TreeContext,
    tools: Vec<LayoutTransformTool>,
    preview: Slot<Signal<Option<FieldPreviewState>>>,
    show: Slot<Signal<bool>>,
}

#[component]
fn matrix_rail_host(props: MatrixRailHostProps) -> Element {
    let show = use_signal(|| true);
    props.show.0.borrow_mut().get_or_insert(show);
    let preview_state = provide_preview_contexts(&props.runtime.0);
    props.preview.0.borrow_mut().get_or_insert(preview_state);
    let selected_context = use_signal(|| scoped(&props.owner, props.context.clone()));
    let scope_generation = use_signal(|| props.owner.generation);
    let workspace = use_signal(|| "Layout");
    let splay_affect = use_signal(|| MatrixSplayAffect::Column);
    let arbiter = use_hook(CanvasInteractionArbiter::default);
    rsx! {
        if show() {
            svg { id: "overlay-host-svg", view_box: "0 0 100 100", width: "400", height: "400",
                for tool in props.tools.clone() {
                    LayoutTransformToolOverlay {
                        key: "{tool:?}",
                        runtime: props.runtime.clone(),
                        svg: props.svg.clone(),
                        arbiter: arbiter.clone(),
                        owner: props.owner.clone(),
                        selected_context,
                        scope_generation,
                        workspace,
                        matrix: props.matrix.clone(),
                        projection: props.projection.clone(),
                        context: props.context.clone(),
                        tool,
                        snap_settings: super::super::layout_toolbar::LayoutSnapSettings::default(),
                        snap_origins: Vec::new(),
                        splay_affect,
                        view_x: 0.0,
                        view_y: 0.0,
                        width: 100.0,
                        height: 100.0,
                        on_finish: EventHandler::new(|()| {}),
                    }
                }
            }
        }
    }
}

struct MountedRail {
    root: web_sys::Element,
    preview: Slot<Signal<Option<FieldPreviewState>>>,
    show: Signal<bool>,
    _mounted: MountedFixture,
}

impl MountedRail {
    /// The first drawn handle: the token, or the splay origin point without one.
    fn handle(&self) -> web_sys::Element {
        query(&self.root, "[data-transform-handle]").expect("the tool draws a handle")
    }

    fn token(&self, label: &str) -> web_sys::Element {
        query(&self.root, &format!("[aria-label='{label}']"))
            .unwrap_or_else(|| panic!("the {label} token draws"))
    }

    /// World point to client pixels on the overlay svg.
    fn client(&self) -> impl Fn(f64, f64) -> (i32, i32) + use<> {
        world_to_client(&query(&self.root, "svg").expect("the overlay svg"))
    }

    /// The amber preview's movers, when one is showing.
    fn preview_parts(&self) -> Option<Vec<String>> {
        self.preview
            .get()
            .read()
            .as_ref()
            .map(|state| state.parts.clone())
    }
}

impl Drop for MountedRail {
    fn drop(&mut self) {
        self.root.remove();
    }
}

/// Mount `tools` for `matrix_id` with the `context` selection, sharing one svg handle.
async fn mount_matrix_rail(
    runtime: &Rc<Runtime>,
    matrix_id: &str,
    context: TreeContext,
    tools: Vec<LayoutTransformTool>,
) -> MountedRail {
    let (matrix, projection) = matrix_and_scene(runtime, matrix_id);
    let svg = LayoutTransformSvg::new(Rc::new(RefCell::new(None)));
    let props = MatrixRailHostProps {
        runtime: LayoutTransformRuntime(runtime.clone()),
        svg: svg.clone(),
        matrix,
        projection,
        owner: owner_of(runtime),
        context,
        tools,
        preview: Slot::default(),
        show: Slot::default(),
    };
    let (preview, show) = (props.preview.clone(), props.show.clone());
    let (root, mounted) = fixture::mount_root(VirtualDom::new_with_props(matrix_rail_host, props));
    attach_svg(&root, &svg).await;
    MountedRail {
        root,
        preview,
        show: show.get(),
        _mounted: mounted,
    }
}

/// One matrix tool for a column 1 selection of the fixture matrix.
async fn mount_column_tool(runtime: &Rc<Runtime>, tool: LayoutTransformTool) -> MountedRail {
    mount_matrix_rail(runtime, fixture::MATRIX_ID, column(1), vec![tool]).await
}

/// Point the shared svg handle at the mounted svg and stub pointer capture on every handle.
async fn attach_svg(root: &web_sys::Element, svg: &LayoutTransformSvg) {
    wait_until("the overlay host svg mounts", || {
        query(root, "#overlay-host-svg").is_some()
    })
    .await;
    *svg.surface().borrow_mut() =
        query(root, "#overlay-host-svg").and_then(|element| element.dyn_into::<SvgElement>().ok());
    fixture::stub_pointer_capture(root, "[data-transform-handle]");
}

/// Ticket 07: the part rail host mounts one overlay per tool for one part, like the
/// production mount site: a Component context for the part, the anchor matrix pitch for
/// drag snap, and a counter for the half-focus callback.
#[derive(Props, Clone, PartialEq)]
struct PartRailHostProps {
    runtime: LayoutTransformRuntime,
    svg: LayoutTransformSvg,
    owner: LayoutOwnerIdentity,
    part_id: String,
    tools: Vec<LayoutTransformTool>,
    snap: GestureSnapInputs,
    focus_half: Rc<RefCell<u32>>,
}

#[component]
fn part_rail_host(props: PartRailHostProps) -> Element {
    let selected_context = use_signal(|| scoped(&props.owner, part_context(&props.part_id)));
    let scope_generation = use_signal(|| props.owner.generation);
    let workspace = use_signal(|| "Layout");
    let surface_size = use_signal(|| (800.0, 600.0));
    let arbiter = use_hook(CanvasInteractionArbiter::default);
    rsx! {
        svg { id: "overlay-host-svg", view_box: "0 0 100 100", width: "400", height: "400",
            for tool in props.tools.clone() {
                LayoutPartToolOverlay {
                    key: "{tool:?}",
                    runtime: props.runtime.clone(),
                    svg: props.svg.clone(),
                    arbiter: arbiter.clone(),
                    owner: props.owner.clone(),
                    selected_context,
                    scope_generation,
                    workspace,
                    surface_size: Some(surface_size),
                    part_id: props.part_id.clone(),
                    tool,
                    snap: props.snap,
                    view_x: 0.0,
                    view_y: 0.0,
                    width: 100.0,
                    height: 100.0,
                    on_finish: EventHandler::new(|()| {}),
                    on_focus_half: {
                        let focus_half = props.focus_half.clone();
                        EventHandler::new(move |()| *focus_half.borrow_mut() += 1)
                    },
                }
            }
        }
    }
}

struct MountedPartRail {
    root: web_sys::Element,
    focus_half: Rc<RefCell<u32>>,
    _mounted: MountedFixture,
}

impl MountedPartRail {
    /// The first drawn token, if any: locked parts, anchors without the tool and matrix
    /// members draw nothing.
    fn handle(&self) -> Option<web_sys::Element> {
        query(&self.root, "[data-transform-handle]")
    }

    fn client(&self) -> impl Fn(f64, f64) -> (i32, i32) + use<> {
        world_to_client(&query(&self.root, "svg").expect("the overlay svg"))
    }
}

impl Drop for MountedPartRail {
    fn drop(&mut self) {
        self.root.remove();
    }
}

fn part_snap() -> GestureSnapInputs {
    super::super::layout_toolbar::gesture_snap_inputs(
        &super::super::layout_toolbar::LayoutSnapSettings::default(),
        Some(Vec2 { x: 19.05, y: 19.05 }),
        None,
    )
}

async fn mount_part_rail(
    runtime: &Rc<Runtime>,
    part_id: &str,
    tools: Vec<LayoutTransformTool>,
    snap: GestureSnapInputs,
) -> MountedPartRail {
    let svg = LayoutTransformSvg::new(Rc::new(RefCell::new(None)));
    let focus_half = Rc::new(RefCell::new(0_u32));
    let props = PartRailHostProps {
        runtime: LayoutTransformRuntime(runtime.clone()),
        svg: svg.clone(),
        owner: owner_of(runtime),
        part_id: part_id.to_owned(),
        tools,
        snap,
        focus_half: focus_half.clone(),
    };
    let (root, mounted) = fixture::mount_root(VirtualDom::new_with_props(part_rail_host, props));
    attach_svg(&root, &svg).await;
    MountedPartRail {
        root,
        focus_half,
        _mounted: mounted,
    }
}

/// One part tool with the default part snap.
async fn mount_part_tool(
    runtime: &Rc<Runtime>,
    part_id: &str,
    tool: LayoutTransformTool,
) -> MountedPartRail {
    mount_part_rail(runtime, part_id, vec![tool], part_snap()).await
}

/// Ticket 08: the pick host mounts one Anchor overlay with the production preview context,
/// so rings position on evaluated scenes.
#[derive(Props, Clone, PartialEq)]
struct PickHostProps {
    runtime: LayoutTransformRuntime,
    svg: LayoutTransformSvg,
    owner: LayoutOwnerIdentity,
    pick_owner: AnchorPickOwner,
}

#[component]
fn pick_host(props: PickHostProps) -> Element {
    let selected_context = use_signal(|| scoped(&props.owner, props.pick_owner.context()));
    let scope_generation = use_signal(|| props.owner.generation);
    let workspace = use_signal(|| "Layout");
    let surface_size = use_signal(|| (800.0, 600.0));
    let arbiter = use_hook(CanvasInteractionArbiter::default);
    provide_preview_contexts(&props.runtime.0);
    rsx! {
        svg { id: "overlay-host-svg", view_box: "0 0 100 100", width: "400", height: "400",
            LayoutAnchorPickOverlay {
                runtime: props.runtime.clone(),
                svg: props.svg.clone(),
                arbiter,
                owner: props.owner.clone(),
                selected_context,
                scope_generation,
                workspace,
                surface_size: Some(surface_size),
                pick_owner: props.pick_owner.clone(),
                view_x: 0.0,
                view_y: 0.0,
                width: 100.0,
                height: 100.0,
            }
        }
    }
}

/// The pick owner for the accepted matrix `matrix_id`, as the canvas builds it.
fn matrix_pick_owner(runtime: &Rc<Runtime>, matrix_id: &str) -> AnchorPickOwner {
    let (matrix, projection) = matrix_and_scene(runtime, matrix_id);
    AnchorPickOwner::Matrix {
        matrix: Rc::new(matrix),
        projection: Rc::new(projection),
        context: TreeContext::Matrix {
            matrix_id: matrix_id.into(),
        },
    }
}

async fn mount_pick(
    runtime: &Rc<Runtime>,
    pick_owner: AnchorPickOwner,
) -> (web_sys::Element, MountedFixture) {
    let svg = LayoutTransformSvg::new(Rc::new(RefCell::new(None)));
    let props = PickHostProps {
        runtime: LayoutTransformRuntime(runtime.clone()),
        svg: svg.clone(),
        owner: owner_of(runtime),
        pick_owner,
    };
    let (root, mounted) = fixture::mount_root(VirtualDom::new_with_props(pick_host, props));
    attach_svg(&root, &svg).await;
    (root, mounted)
}

/// One synthetic pointer drag: down at `from` (with Ctrl held only for the down), one move
/// and the release at `to` (with Alt on both), then drive the runtime until the edit lands.
/// The dispatched events are script-created, so `isTrusted` is always false; real trusted
/// pointer proof lives in the packaged-page mouse smoke (`check.py drag-smoke`).
async fn drag_commit(
    runtime: &Rc<Runtime>,
    handle: &web_sys::Element,
    from: (i32, i32),
    to: (i32, i32),
    ctrl: bool,
    alt: bool,
) {
    let before = revision(runtime);
    dispatch_pointer(handle, "pointerdown", ctrl, false, 1, from);
    frame().await;
    dispatch_pointer(handle, "pointermove", false, alt, 1, to);
    frame().await;
    dispatch_pointer(handle, "pointerup", false, alt, 0, to);
    pump_past(runtime, before).await;
    assert_eq!(revision(runtime), before + 1, "one accepted edit");
}

/// The signed degrees before the first degree mark in a popup or toast text, e.g. `45.0°`.
fn degrees_in(text: &str) -> f64 {
    let end = text.find('°').expect("the text names degrees");
    let start = text[..end]
        .rfind(|cell: char| {
            !(cell.is_ascii_digit() || cell == '.' || cell == '-' || cell == '+' || cell == '−')
        })
        .map(|index| index + 1)
        .unwrap_or(0);
    text[start..end].parse().expect("the text names degrees")
}

/// The commit toast's text, once the commit has settled and raised it.
async fn toast_text(runtime: &Rc<Runtime>, root: &web_sys::Element) -> String {
    pump_until(runtime, "the commit raises its toast", || {
        query(root, "[data-feedback-toast]").is_some()
    })
    .await;
    text_of(root, "[data-feedback-toast]")
}
