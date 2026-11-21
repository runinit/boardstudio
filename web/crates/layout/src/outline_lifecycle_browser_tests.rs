use super::*;
use boardstudio_application::Scope;
use boardstudio_model::model::{
    Board, BoardOutline, OutlineConnection, OutlineControlPoint, OutlineFeature, OutlineProvenance,
    OutlineSnapshot, OutlineVersion, Part, PartDefinition, PartKind, Pose2, ProjectDoc, Side, Vec2,
};
use boardstudio_web_ui_shared::test_mount::MountedFixture;
use boardstudio_web_ui_shared::{test_role::by_role_name, test_wait::wait_until};
use std::{cell::RefCell, rc::Rc, sync::Arc};
use wasm_bindgen::JsCast;
use wasm_bindgen_test::wasm_bindgen_test;

wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

/// The shared browser document.
fn browser_document() -> web_sys::Document {
    web_sys::window().unwrap().document().unwrap()
}

/// A mounted root element by id (the inspector hosts use fixed mount ids).
fn root_by_id(id: &str) -> web_sys::Element {
    browser_document().get_element_by_id(id).unwrap()
}

fn points_root() -> web_sys::Element {
    root_by_id("outline-points-inspector-mount")
}

fn inspector_root() -> web_sys::Element {
    root_by_id("outline-inspector-mount")
}

/// Create (but do not mount) a root div with the given id.
fn new_root(id: &str) -> web_sys::Element {
    let document = browser_document();
    let root = document.create_element("div").unwrap();
    root.set_id(id);
    document.body().unwrap().append_child(&root).unwrap();
    root
}

/// Mount `dom` (with `probe` as root context) under a fresh root div.
fn mount_probe<T: Clone + 'static>(
    id: &str,
    dom: VirtualDom,
    probe: &T,
) -> (web_sys::Element, MountedFixture) {
    let root = new_root(id);
    dom.provide_root_context(probe.clone());
    let mounted = MountedFixture::new(root.clone(), dom);
    (root, mounted)
}

/// Open `document` in a fresh runtime and build its inspector probe.
async fn open_inspector_probe(document: ProjectDoc) -> InspectorProbe {
    let runtime = boardstudio_web_test_runtime::project_name_test_support::new_runtime();
    boardstudio_web_test_runtime::project_name_test_support::open_document(&runtime, document)
        .await;
    let scope = runtime.scope().expect("the opened fixture has a scope");
    InspectorProbe {
        runtime,
        scope,
        connection_action: Rc::default(),
        selected_context: Rc::default(),
    }
}

/// A button by accessible name, as a clickable HTML element.
fn button(root: &web_sys::Element, name: &str) -> web_sys::HtmlElement {
    by_role_name(root, "button", name)
        .unwrap_or_else(|| panic!("button {name} is mounted"))
        .dyn_into::<web_sys::HtmlElement>()
        .unwrap()
}

fn click_button(root: &web_sys::Element, name: &str) {
    button(root, name).click();
}

/// Wait until a button with `name` is mounted under `root`.
async fn wait_for_button(root: &web_sys::Element, description: &'static str, name: &str) {
    let root = root.clone();
    wait_until(description, move || {
        by_role_name(&root, "button", name).is_some()
    })
    .await;
}

/// Wait for and open the perimeter point editor on the points inspector.
async fn open_perimeter_editor() {
    wait_for_button(
        &points_root(),
        "the perimeter exposes its point editor action",
        "Edit perimeter points",
    )
    .await;
    click_button(&points_root(), "Edit perimeter points");
}

async fn active_outline_option_labels(root: &web_sys::Element) -> Vec<String> {
    let choice = by_role_name(root, "button", "Active outline").unwrap();
    choice.focus().unwrap();
    let init = web_sys::KeyboardEventInit::new();
    init.set_key("ArrowDown");
    init.set_bubbles(true);
    init.set_cancelable(true);
    choice
        .dispatch_event(
            &web_sys::KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &init).unwrap(),
        )
        .unwrap();
    wait_until("the actual Active outline list opens", || {
        choice.get_attribute("aria-expanded").as_deref() == Some("true")
            && by_role_name(root, "listbox", "Active outline").is_some()
    })
    .await;
    let list = by_role_name(root, "listbox", "Active outline").unwrap();
    let options = list.query_selector_all("[role=option]").unwrap();
    let labels = (0..options.length())
        .filter_map(|index| options.item(index))
        .map(|option| option.text_content().unwrap_or_default().trim().to_owned())
        .collect();
    init.set_key("Escape");
    web_sys::window()
        .unwrap()
        .document()
        .unwrap()
        .active_element()
        .unwrap()
        .dispatch_event(
            &web_sys::KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &init).unwrap(),
        )
        .unwrap();
    wait_until("the outline choices close and return focus", || {
        choice.get_attribute("aria-expanded").as_deref() == Some("false")
            && web_sys::window()
                .unwrap()
                .document()
                .unwrap()
                .active_element()
                .as_ref()
                .is_some_and(|active| active.is_same_node(Some(choice.as_ref())))
    })
    .await;
    labels
}

/// Drive the in-process Session and Core until every queued edit has settled.
async fn accept(probe: &InspectorProbe) {
    for _ in 0..6 {
        boardstudio_web_test_runtime::project_name_test_support::run_pending(&probe.runtime).await;
        gloo_timers::future::TimeoutFuture::new(10).await;
    }
}

pub(super) async fn undo(runtime: &Rc<crate::runtime::Runtime>) {
    runtime.submit(boardstudio_application::Event::Undo {
        operation_id: runtime.operation(),
    });
    boardstudio_web_test_runtime::project_name_test_support::run_pending(runtime).await;
}

fn accepted_revision(probe: &InspectorProbe) -> u64 {
    probe
        .runtime
        .model()
        .accepted
        .expect("an accepted document")
        .document
        .revision
}

fn accepted_document(probe: &InspectorProbe) -> boardstudio_model::model::ProjectDoc {
    probe
        .runtime
        .model()
        .accepted
        .expect("an accepted document")
        .document
        .as_ref()
        .clone()
}

async fn settle_dimension() {
    gloo_timers::future::TimeoutFuture::new(35).await;
}

/// One event-loop turn before a negative check. It waits for no condition, so a negative
/// assertion after it is recorded as render-unproven in the Outcome.
async fn yield_turn() {
    gloo_timers::future::TimeoutFuture::new(0).await;
}

fn send_input(input: &web_sys::HtmlInputElement, value: &str) {
    input.set_value(value);
    let init = web_sys::EventInit::new();
    init.set_bubbles(true);
    input
        .dispatch_event(&web_sys::Event::new_with_event_init_dict("input", &init).unwrap())
        .unwrap();
}

fn send_key(input: &web_sys::HtmlInputElement, key: &str) {
    let init = web_sys::KeyboardEventInit::new();
    init.set_key(key);
    init.set_bubbles(true);
    input
        .dispatch_event(
            &web_sys::KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &init).unwrap(),
        )
        .unwrap();
}

type OutlineConnectionAction = (Rc<OutlineActionContext>, EventHandler<OutlineAction>);

#[derive(Clone)]
struct InspectorProbe {
    runtime: Rc<crate::runtime::Runtime>,
    scope: Scope,
    connection_action: Rc<RefCell<Option<OutlineConnectionAction>>>,
    selected_context: Rc<RefCell<Option<Signal<Option<super::super::objects::ScopedTreeContext>>>>>,
}

impl InspectorProbe {
    fn add_connection(&self, points: Vec<Vec2>) {
        let action = self
            .connection_action
            .borrow()
            .as_ref()
            .map(|(context, handler)| (context.add_connection(points), *handler))
            .expect("mounted Inspector publishes its production action owner");
        action.1.call(action.0);
    }
}

fn mounted_outline_inspector_host() -> Element {
    let probe = use_context::<InspectorProbe>();
    let version = use_signal(|| 0u64);
    use_context_provider(|| version);
    use_hook({
        let runtime = probe.runtime.clone();
        move || {
            runtime.subscribe(Rc::new(move || {
                let mut version = version;
                version += 1;
            }));
        }
    });
    let workspace = use_signal(|| "Layout");
    let generation = use_signal(|| 1u64);
    let selected = use_signal(|| {
        Some(super::super::objects::ScopedTreeContext {
            scope: probe.scope.clone(),
            context: super::super::objects::TreeContext::Outline {
                board_id: probe.scope.board_id.clone(),
            },
        })
    });
    *probe.selected_context.borrow_mut() = Some(selected);
    let (projection, _) =
        use_outline_lifecycle(probe.runtime.clone(), selected, workspace, generation);
    let board_inspector = super::super::board_inspector::use_board_inspector(
        probe.runtime.clone(),
        selected,
        workspace,
        generation,
    );
    if let Some(projection) = projection.as_ref() {
        *probe.connection_action.borrow_mut() =
            Some((projection.action_context.clone(), projection.on_action));
    }
    let _ = version();
    let point_canvas_svg = use_hook(|| Rc::new(RefCell::new(None::<web_sys::SvgElement>)));
    let point_canvas_arbiter =
        use_hook(super::super::canvas_interaction::CanvasInteractionArbiter::default);
    let mounted_point_canvas_svg = point_canvas_svg.clone();
    let mount_point_canvas = move |event: MountedEvent| {
        if let Some(svg) = event
            .data()
            .try_as_web_event()
            .and_then(|event| event.dyn_into::<web_sys::SvgElement>().ok())
        {
            *mounted_point_canvas_svg.borrow_mut() = Some(svg);
        }
    };
    rsx! {
        style { {include_str!("../../../assets/workbench-base.css")} }
        svg { id: "outline-point-canvas-test", view_box: "-20 -20 40 40", onmounted: mount_point_canvas,
            if let Some(projection) = projection.clone() {
                super::super::outline_lifecycle::OutlinePointCanvasOverlay {
                    projection,
                    runtime: super::super::outline_lifecycle::OutlineRuntimeHandle::new(probe.runtime.clone()),
                    arbiter: point_canvas_arbiter.clone(),
                    svg: point_canvas_svg.clone(),
                    view_x: -20.0,
                    view_y: -20.0,
                    width: 40.0,
                    height: 40.0,
                    snap_settings: super::super::objects::LayoutSnapSettings::default(),
                    pitch: Vec2 { x: 19.05, y: 19.05 },
                    origins: vec![],
                }
            }
        }
        if let Some(projection) = projection {
            OutlineVersionInspector { projection }
        }
        if board_inspector.projection.is_some() {
            super::super::board_inspector::BoardInspector {
                mount: board_inspector,
            }
        }
    }
}

async fn mounted_outline_inspector() -> (InspectorProbe, web_sys::Element, MountedFixture) {
    mounted_outline_inspector_with_version(None).await
}

#[wasm_bindgen_test]
async fn outline_actions_keep_their_layout_without_legacy_classes() {
    let (_probe, root, mut mounted) = mounted_outline_inspector().await;
    wait_for_button(&root, "outline actions mount", "Copy outline").await;
    let document = web_sys::window().unwrap().document().unwrap();
    let stylesheet = document.create_element("style").unwrap();
    stylesheet.set_text_content(Some(include_str!("../../../assets/tailwind.css")));
    document.head().unwrap().append_child(&stylesheet).unwrap();
    mounted.own_node(stylesheet.clone());
    let action = by_role_name(&root, "button", "Copy outline").unwrap();
    let row = action.parent_element().unwrap();
    let classes = row.get_attribute("class").unwrap_or_default();
    row.set_attribute(
        "class",
        &classes
            .split_whitespace()
            .filter(|class| !class.starts_with("m1-"))
            .collect::<Vec<_>>()
            .join(" "),
    )
    .unwrap();
    let window = web_sys::window().unwrap();
    let row_style = window.get_computed_style(&row).unwrap().unwrap();
    let action_style = window.get_computed_style(&action).unwrap().unwrap();
    assert_eq!(row_style.get_property_value("display").unwrap(), "flex");
    assert_eq!(row_style.get_property_value("gap").unwrap(), "8px");
    assert_eq!(row_style.get_property_value("margin-top").unwrap(), "12px");
    assert_eq!(
        action_style.get_property_value("min-height").unwrap(),
        "36px"
    );
    let heading = root
        .query_selector("[aria-label='Board outline'] h2")
        .unwrap()
        .unwrap();
    let heading_row = heading.parent_element().unwrap();
    let classes = heading_row.get_attribute("class").unwrap_or_default();
    heading_row
        .set_attribute(
            "class",
            &classes
                .split_whitespace()
                .filter(|class| !class.starts_with("m1-"))
                .collect::<Vec<_>>()
                .join(" "),
        )
        .unwrap();
    let heading_row_style = window.get_computed_style(&heading_row).unwrap().unwrap();
    let heading_style = window.get_computed_style(&heading).unwrap().unwrap();
    assert_eq!(
        heading_row_style.get_property_value("display").unwrap(),
        "flex"
    );
    assert_eq!(
        heading_style.get_property_value("margin-top").unwrap(),
        "0px"
    );
    assert_eq!(
        heading_style.get_property_value("font-size").unwrap(),
        "16px"
    );
    stylesheet.remove();
    root.remove();
}

async fn mounted_outline_inspector_with_version(
    version: Option<boardstudio_model::model::OutlineVersion>,
) -> (InspectorProbe, web_sys::Element, MountedFixture) {
    let board_id = "outline-board".to_string();
    let mut document = ProjectDoc::empty("outline-inspector-doc", "Outline fixture");
    document.revision = 9;
    document.boards.push(Board {
        id: board_id.clone(),
        name: "Board".into(),
        outline_ids: vec![],
        part_ids: vec![],
        net_ids: vec![],
        thickness: 1.6,
        traces: vec![],
        vias: vec![],
    });
    if let Some(version) = version {
        document
            .board_outlines
            .push(boardstudio_model::model::BoardOutline {
                board_id: board_id.clone(),
                active_version_id: Some(version.id.clone()),
                versions: vec![version],
                generated_last_valid: None,
                retired_version_ids: vec![],
            });
    }
    let probe = open_inspector_probe(document).await;
    let (root, mounted) = mount_probe(
        "outline-inspector-mount",
        VirtualDom::new(mounted_outline_inspector_host),
        &probe,
    );
    (probe, root, mounted)
}

#[wasm_bindgen_test]
async fn mounted_outline_perimeter_escape_returns_to_board_inspector() {
    let (probe, root, _mounted) = mounted_polygon_outline_inspector(true).await;
    wait_until("the mounted Outline publishes its selected context", || {
        probe.selected_context.borrow().is_some()
    })
    .await;
    let document = web_sys::window().unwrap().document().unwrap();
    let selection = *probe
        .selected_context
        .borrow()
        .as_ref()
        .expect("mounted lifecycle exposes its actual selected-context owner");
    assert!(matches!(
        selection.read().as_ref().map(|selected| &selected.context),
        Some(super::super::objects::TreeContext::Outline { .. })
    ));

    click_button(&points_root(), "Edit perimeter points");
    wait_for_button(
        &points_root(),
        "Edit perimeter points renders the point editor",
        "Done",
    )
    .await;
    let done = button(&points_root(), "Done");
    done.focus().unwrap();
    let init = web_sys::KeyboardEventInit::new();
    init.set_key("Escape");
    init.set_bubbles(true);
    done.dispatch_event(
        &web_sys::KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &init).unwrap(),
    )
    .unwrap();
    // Selection changes before the nested editor unmounts and before focus moves, so the wait
    // covers all three: the assertions below read the unmounted editor and the focused field.
    wait_until(
        "Escape from the focused Perimeter Done button returns selection to the board inspector",
        || {
            matches!(
                selection.read().as_ref().map(|selected| &selected.context),
                Some(super::super::objects::TreeContext::Board { board_id }) if board_id == &probe.scope.board_id
            ) && document
                .query_selector("#outline-points-inspector-mount [aria-label='Perimeter']")
                .unwrap()
                .is_none()
                && document
                    .active_element()
                    .and_then(|active| active.get_attribute("aria-label"))
                    .as_deref()
                    == Some("Board name")
        },
    )
    .await;
    assert!(
        matches!(
            selection.read().as_ref().map(|selected| &selected.context),
            Some(super::super::objects::TreeContext::Board { board_id }) if board_id == &probe.scope.board_id
        ),
        "Escape from the focused Perimeter Done button returns selection to the board inspector"
    );
    assert!(
        document
            .query_selector("#outline-points-inspector-mount [aria-label='Perimeter']")
            .unwrap()
            .is_none(),
        "the nested Perimeter inspector is no longer mounted after returning to Board"
    );
    let board_name = by_role_name(&points_root(), "textbox", "Board name")
        .expect("the returned Board Inspector exposes its name field");
    assert_eq!(
        document
            .active_element()
            .and_then(|active| active.get_attribute("aria-label"))
            .as_deref(),
        Some("Board name"),
        "Escape transfers focus into the newly mounted Board Inspector"
    );
    drop(board_name);
    root.remove();
}

#[wasm_bindgen_test]
async fn mounted_fixed_outline_canvas_point_escape_returns_to_board_without_editing() {
    let (probe, root, _mounted) = mounted_polygon_outline_inspector(true).await;
    wait_until("the mounted Outline publishes its selected context", || {
        probe.selected_context.borrow().is_some()
    })
    .await;
    let document = web_sys::window().unwrap().document().unwrap();
    let selected_context = *probe
        .selected_context
        .borrow()
        .as_ref()
        .expect("mounted outline lifecycle exposes its selected-context owner");
    let accepted_before = probe
        .runtime
        .model()
        .accepted
        .expect("fixture retains the accepted fixed outline");
    let accepted_document_before = accepted_before.document.clone();

    click_button(&points_root(), "Edit perimeter points");
    wait_until("the canvas point handle mounts", || {
        document
            .query_selector("#outline-point-canvas-test circle[aria-label='Outline point 2']")
            .unwrap()
            .is_some()
    })
    .await;

    let point = document
        .query_selector("#outline-point-canvas-test circle[aria-label='Outline point 2']")
        .unwrap()
        .expect("the production canvas overlay mounts its focusable point 2 circle")
        .dyn_into::<web_sys::SvgElement>()
        .unwrap();
    point.focus().unwrap();
    assert_eq!(
        document
            .active_element()
            .and_then(|active| active.get_attribute("aria-label"))
            .as_deref(),
        Some("Outline point 2"),
        "the actual canvas point circle owns keyboard focus before Escape"
    );

    let escape = web_sys::KeyboardEventInit::new();
    escape.set_key("Escape");
    escape.set_bubbles(true);
    point
        .dispatch_event(
            &web_sys::KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &escape).unwrap(),
        )
        .unwrap();
    wait_until("Escape closes the canvas point editor", || {
        document
            .query_selector("#outline-points-inspector-mount [aria-label='Perimeter']")
            .unwrap()
            .is_none()
            && document
                .query_selector("#outline-point-canvas-test circle[aria-label='Outline point 2']")
                .unwrap()
                .is_none()
    })
    .await;

    assert!(
        matches!(
            selected_context
                .read()
                .as_ref()
                .map(|selected| &selected.context),
            Some(super::super::objects::TreeContext::Outline { board_id })
                if board_id == &probe.scope.board_id
        ),
        "canvas Escape closes only point editing and preserves the selected outline context"
    );
    assert!(
        document
            .query_selector("#outline-points-inspector-mount [aria-label='Perimeter']")
            .unwrap()
            .is_none(),
        "Escape from the canvas point exits perimeter editing"
    );
    assert!(
        document
            .query_selector("#outline-point-canvas-test circle[aria-label='Outline point 2']")
            .unwrap()
            .is_none(),
        "the focused point handle unmounts after returning to the outline inspector"
    );
    assert!(
        by_role_name(&points_root(), "button", "Edit perimeter points").is_some(),
        "the normal Board outline controls remain mounted after point editing closes"
    );
    assert_eq!(
        document
            .active_element()
            .map(|active| active.tag_name())
            .as_deref(),
        Some("BODY"),
        "canvas-owned Escape leaves focus on BODY as the reference does"
    );
    let accepted_after = probe
        .runtime
        .model()
        .accepted
        .expect("accepted snapshot remains available after navigation");
    assert_eq!(
        accepted_after.document.as_ref(),
        accepted_document_before.as_ref(),
        "navigation Escape does not edit the accepted fixed outline"
    );
    root.remove();
}

#[wasm_bindgen_test]
async fn mounted_fixed_outline_point_click_and_escape_preserve_fractional_geometry() {
    let points = vec![
        Vec2 { x: -13.0, y: 13.0 },
        Vec2 {
            x: -8.2375,
            y: -13.0,
        },
        Vec2 { x: 13.0, y: -13.0 },
        Vec2 { x: 13.0, y: 13.0 },
    ];
    let (probe, root, _mounted) = mounted_polygon_outline_inspector_with_points(true, points).await;
    open_perimeter_editor().await;
    let document = web_sys::window().unwrap().document().unwrap();
    let revision_before = accepted_revision(&probe);

    wait_until("the perimeter point handles mount", || {
        document
            .query_selector("#outline-point-canvas-test circle[aria-label='Outline point 2']")
            .unwrap()
            .is_some()
    })
    .await;

    let point = document
        .query_selector("#outline-point-canvas-test circle[aria-label='Outline point 2']")
        .unwrap()
        .expect("the production canvas overlay mounts its focusable point 2 circle")
        .dyn_into::<web_sys::SvgElement>()
        .unwrap();
    point.focus().unwrap();
    let captured = install_test_pointer_capture(&point);
    let rect = point.get_bounding_client_rect();
    let client_x = (rect.x() + rect.width() / 2.0).round() as i32;
    let client_y = (rect.y() + rect.height() / 2.0).round() as i32;
    dispatch_test_pointer_at(&point, "pointerdown", 1, client_x, client_y);
    assert_eq!(
        captured.captured.get(),
        Some(17),
        "the mounted production pointerdown handler started a point interaction"
    );
    dispatch_test_pointer_at(&point, "pointerup", 0, client_x, client_y);
    wait_until(
        "the mounted production pointerup handler finishes the point interaction",
        || captured.captured.get().is_none(),
    )
    .await;
    assert_eq!(
        captured.captured.get(),
        None,
        "the mounted production pointerup handler finished the point interaction"
    );
    accept(&probe).await;
    assert_eq!(
        accepted_revision(&probe),
        revision_before,
        "a pointer interaction without pointer movement must not edit the outline"
    );

    dispatch_test_pointer_at(&point, "pointerdown", 1, client_x, client_y);
    assert_eq!(captured.captured.get(), Some(17));
    dispatch_test_pointer_at(&point, "pointerup", 0, client_x + 4, client_y);
    wait_until(
        "the displaced pointerup finishes the point interaction",
        || captured.captured.get().is_none(),
    )
    .await;
    assert_eq!(captured.captured.get(), None);
    accept(&probe).await;
    assert_eq!(
        accepted_revision(&probe),
        revision_before + 1,
        "a pointer-up displaced from pointer-down still commits the final sample when no pointermove was delivered"
    );
    let revision_after_commit = accepted_revision(&probe);

    let shift = web_sys::KeyboardEventInit::new();
    shift.set_key("Shift");
    shift.set_bubbles(true);
    point
        .dispatch_event(
            &web_sys::KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &shift).unwrap(),
        )
        .unwrap();
    point
        .dispatch_event(
            &web_sys::KeyboardEvent::new_with_keyboard_event_init_dict("keyup", &shift).unwrap(),
        )
        .unwrap();
    let escape = web_sys::KeyboardEventInit::new();
    escape.set_key("Escape");
    escape.set_bubbles(true);
    point
        .dispatch_event(
            &web_sys::KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &escape).unwrap(),
        )
        .unwrap();
    settle_dimension().await;

    accept(&probe).await;
    assert_eq!(
        accepted_revision(&probe),
        revision_after_commit,
        "Shift and Escape do not edit the outline"
    );
    root.remove();
}

struct TestPointerCapture {
    captured: Rc<std::cell::Cell<Option<i32>>>,
    _set_capture: wasm_bindgen::closure::Closure<dyn FnMut(i32)>,
    _has_capture: wasm_bindgen::closure::Closure<dyn FnMut(i32) -> bool>,
    _release_capture: wasm_bindgen::closure::Closure<dyn FnMut(i32)>,
}

fn install_test_pointer_capture(target: &web_sys::SvgElement) -> TestPointerCapture {
    use wasm_bindgen::closure::Closure;

    let captured = Rc::new(std::cell::Cell::new(None::<i32>));
    let set_capture_state = captured.clone();
    let set_capture = Closure::<dyn FnMut(i32)>::new(move |id| {
        set_capture_state.set(Some(id));
    });
    let has_capture_state = captured.clone();
    let has_capture =
        Closure::<dyn FnMut(i32) -> bool>::new(move |id| has_capture_state.get() == Some(id));
    let release_capture_state = captured.clone();
    let release_capture = Closure::<dyn FnMut(i32)>::new(move |id| {
        if release_capture_state.get() == Some(id) {
            release_capture_state.set(None);
        }
    });
    let target_value: &wasm_bindgen::JsValue = target.as_ref();
    js_sys::Reflect::set(
        target_value,
        &"setPointerCapture".into(),
        set_capture.as_ref(),
    )
    .unwrap();
    js_sys::Reflect::set(
        target_value,
        &"hasPointerCapture".into(),
        has_capture.as_ref(),
    )
    .unwrap();
    js_sys::Reflect::set(
        target_value,
        &"releasePointerCapture".into(),
        release_capture.as_ref(),
    )
    .unwrap();
    TestPointerCapture {
        captured,
        _set_capture: set_capture,
        _has_capture: has_capture,
        _release_capture: release_capture,
    }
}

fn dispatch_test_pointer_at(
    target: &web_sys::SvgElement,
    kind: &str,
    buttons: u16,
    client_x: i32,
    client_y: i32,
) {
    let pointer_id = 17;
    let init = js_sys::Object::new();
    for (name, value) in [
        ("bubbles", wasm_bindgen::JsValue::TRUE),
        ("cancelable", wasm_bindgen::JsValue::TRUE),
        ("pointerId", pointer_id.into()),
        ("pointerType", "mouse".into()),
        ("isPrimary", wasm_bindgen::JsValue::TRUE),
        ("button", 0.into()),
        ("buttons", buttons.into()),
        ("clientX", client_x.into()),
        ("clientY", client_y.into()),
    ] {
        js_sys::Reflect::set(init.as_ref(), &name.into(), &value).unwrap();
    }
    let pointer_constructor = js_sys::Reflect::get(&js_sys::global(), &"PointerEvent".into())
        .unwrap()
        .dyn_into::<js_sys::Function>()
        .unwrap();
    let arguments = js_sys::Array::new();
    arguments.push(&kind.into());
    arguments.push(init.as_ref());
    let event = js_sys::Reflect::construct(&pointer_constructor, &arguments)
        .unwrap()
        .dyn_into::<web_sys::PointerEvent>()
        .unwrap();
    target.dispatch_event(&event).unwrap();
}

async fn mounted_polygon_outline_inspector(
    fixed: bool,
) -> (InspectorProbe, web_sys::Element, MountedFixture) {
    mounted_polygon_outline_inspector_with_connections(fixed, vec![]).await
}

async fn mounted_polygon_outline_inspector_with_connections(
    fixed: bool,
    connections: Vec<OutlineConnection>,
) -> (InspectorProbe, web_sys::Element, MountedFixture) {
    let points = vec![
        boardstudio_model::model::Vec2 { x: 0.0, y: 0.0 },
        boardstudio_model::model::Vec2 { x: 20.0, y: 0.0 },
        boardstudio_model::model::Vec2 { x: 20.0, y: 20.0 },
        boardstudio_model::model::Vec2 { x: 0.0, y: 20.0 },
    ];
    mounted_polygon_outline_inspector_with_points_and_connections(fixed, points, connections).await
}

async fn mounted_polygon_outline_inspector_with_points(
    fixed: bool,
    points: Vec<Vec2>,
) -> (InspectorProbe, web_sys::Element, MountedFixture) {
    mounted_polygon_outline_inspector_with_points_and_connections(fixed, points, vec![]).await
}

async fn mounted_polygon_outline_inspector_with_points_and_connections(
    fixed: bool,
    points: Vec<Vec2>,
    connections: Vec<OutlineConnection>,
) -> (InspectorProbe, web_sys::Element, MountedFixture) {
    let board_id = "outline-points-board".to_string();
    let mut document = ProjectDoc::empty("outline-points-doc", "Outline points fixture");
    document.revision = 11;
    let has_envelope_part = !connections.is_empty() || !fixed;
    if has_envelope_part {
        document.definitions.push(PartDefinition {
            hardware_profile: None,
            input_profile: None,
            mechanical_profile: None,
            id: "switch".into(),
            name: "Switch".into(),
            kind: PartKind::Switch,
            keycap: None,
            envelope_source: None,
            kicad_source: None,
            terminals: Default::default(),
            matrix_terminals: None,
            envelope_notice: None,
            generator: None,
            courtyard: vec![
                Vec2 { x: -3.0, y: -3.0 },
                Vec2 { x: 3.0, y: -3.0 },
                Vec2 { x: 3.0, y: 3.0 },
                Vec2 { x: -3.0, y: 3.0 },
            ],
            pads: vec![],
            models: None,
        });
        document.parts.push(Part {
            id: "k1".into(),
            definition_id: "switch".into(),
            reference: "SW1".into(),
            pose: Pose2 {
                at: Vec2::default(),
                rotation: 0.0,
            },
            side: Side::Front,
            locked: None,
            keycap: None,
            outline: None,
            anchor: None,
            properties: None,
            generator_parameters: None,
        });
    }
    document.boards.push(Board {
        id: board_id.clone(),
        name: "Board".into(),
        outline_ids: if fixed {
            vec![]
        } else {
            vec!["generated-outline".into()]
        },
        part_ids: has_envelope_part
            .then(|| "k1".to_owned())
            .into_iter()
            .collect(),
        net_ids: vec![],
        thickness: 1.6,
        traces: vec![],
        vias: vec![],
    });
    if fixed {
        document.board_outlines.push(BoardOutline {
            board_id: board_id.clone(),
            active_version_id: Some("fixed-outline-v1".into()),
            versions: vec![OutlineVersion {
                id: "fixed-outline-v1".into(),
                name: "Edited outline 1".into(),
                source: OutlineProvenance {
                    revision: 10,
                    version_id: None,
                },
                geometry: OutlineSnapshot {
                    features: vec![OutlineFeature::Polygon {
                        anchor_part_id: None,
                        id: "fixed-contour".into(),
                        points: points.clone(),
                        operation: boardstudio_model::model::Operation::Add,
                    }],
                    settings: boardstudio_model::model::OutlineSettings::default(),
                    expected_regions: 1,
                    bridges: vec![],
                    protected_gaps: vec![],
                },
                ownership: boardstudio_model::model::OutlineOwnership::Fixed,
            }],
            generated_last_valid: None,
            retired_version_ids: vec![],
        });
    } else {
        document.outline.push(OutlineFeature::PartEnvelope {
            connections,
            settings: boardstudio_model::model::OutlineSettings::default(),
            id: "generated-outline".into(),
            part_ids: has_envelope_part
                .then(|| "k1".to_owned())
                .into_iter()
                .collect(),
            margin: 4.0,
            operation: boardstudio_model::model::Operation::Add,
        });
    }
    let probe = open_inspector_probe(document).await;
    let (root, mounted) = mount_probe(
        "outline-points-inspector-mount",
        VirtualDom::new(mounted_outline_inspector_host),
        &probe,
    );
    (probe, root, mounted)
}

#[wasm_bindgen_test]
async fn mounted_generated_connection_after_reopen_preserves_unique_ids_and_valid_geometry() {
    let existing = OutlineConnection {
        id: "outline-connection-1".into(),
        width: 10.0,
        points: vec![
            OutlineControlPoint {
                at: Vec2 { x: 6.0, y: 0.0 },
                part_id: None,
            },
            OutlineControlPoint {
                at: Vec2 { x: 16.0, y: 0.0 },
                part_id: None,
            },
        ],
    };
    let (probe, root, _mounted) =
        mounted_polygon_outline_inspector_with_connections(false, vec![existing]).await;
    settle_dimension().await;
    probe.add_connection(vec![Vec2 { x: -6.0, y: 0.0 }, Vec2 { x: -16.0, y: 0.0 }]);
    settle_dimension().await;

    accept(&probe).await;
    let document = accepted_document(&probe);
    let scene = probe
        .runtime
        .model()
        .accepted
        .expect("accepted")
        .scene
        .clone();
    let feature = document
        .outline
        .iter()
        .find(|candidate| candidate.id() == "generated-outline")
        .expect("the generated outline stays accepted");
    let OutlineFeature::PartEnvelope { connections, .. } = feature else {
        panic!("the connection edit retains a PartEnvelope")
    };
    assert_eq!(connections.len(), 2);
    assert_eq!(connections[0].id, "outline-connection-1");
    assert_ne!(
        connections[1].id, connections[0].id,
        "the added connection gets a distinct identity"
    );
    let new_connection_id = connections[1].id.clone();
    assert!(
        scene
            .findings
            .iter()
            .all(|finding| finding.id != "outline:generated-outline:invalid"),
        "the accepted generated outline must not gain a duplicate-ID validation finding"
    );
    assert!(
        scene
            .board_contours
            .iter()
            .find(|contours| contours.board_id == probe.scope.board_id)
            .is_some_and(|contours| !contours.contours.is_empty())
    );

    let accepted = document
        .outline
        .iter()
        .find(|candidate| candidate.id() == "generated-outline")
        .unwrap();
    let OutlineFeature::PartEnvelope {
        connections: accepted_connections,
        ..
    } = accepted
    else {
        unreachable!()
    };
    let moved_old = move_connection_point(
        accepted,
        "outline-connection-1",
        0,
        Vec2 { x: 7.0, y: 1.0 },
        &document.parts,
    );
    let moved_new = move_connection_point(
        accepted,
        &new_connection_id,
        0,
        Vec2 { x: -7.0, y: 1.0 },
        &document.parts,
    );
    let OutlineFeature::PartEnvelope {
        connections: moved_old_connections,
        ..
    } = moved_old
    else {
        unreachable!()
    };
    let OutlineFeature::PartEnvelope {
        connections: moved_new_connections,
        ..
    } = moved_new
    else {
        unreachable!()
    };
    assert_eq!(accepted_connections.len(), 2);
    assert_eq!(moved_old_connections[0].points[0].at.x, 7.0);
    assert_eq!(moved_old_connections[1].points[0].at.x, -6.0);
    assert_eq!(moved_new_connections[0].points[0].at.x, 6.0);
    assert_eq!(moved_new_connections[1].points[0].at.x, -7.0);
    root.remove();
}

#[wasm_bindgen_test]
async fn fixed_perimeter_finds_the_first_polygon_after_primitive_features() {
    let (probe, root, _mounted) = mounted_polygon_outline_inspector(true).await;
    let mut snapshot = probe.runtime.model().accepted.unwrap();
    let document = Arc::make_mut(&mut snapshot.document);
    let features = &mut document.board_outlines[0].versions[0].geometry.features;
    let polygon = features[0].clone();
    features.insert(
        0,
        OutlineFeature::Rect {
            id: "fixed-rectangle".into(),
            rotation: None,
            anchor_part_id: None,
            center: Vec2 { x: 10.0, y: 10.0 },
            size: Vec2 { x: 2.0, y: 2.0 },
            radius: 0.0,
            operation: Operation::Add,
        },
    );
    features.insert(
        1,
        OutlineFeature::PartEnvelope {
            id: "fixed-envelope".into(),
            connections: vec![],
            settings: OutlineSettings::default(),
            part_ids: vec![],
            margin: 4.0,
            operation: Operation::Add,
        },
    );
    let mut later_polygon = polygon.clone();
    if let OutlineFeature::Polygon { id, .. } = &mut later_polygon {
        *id = "later-polygon".into();
    }
    features.push(later_polygon);
    let perimeter = editable_perimeter(
        &snapshot,
        &probe.scope.board_id,
        Some("fixed-outline-v1"),
        None,
    )
    .expect("primitive features must not hide the first editable polygon");
    assert!(
        matches!(perimeter.target, OutlinePointTarget::Fixed { feature_id, .. } if feature_id == "fixed-contour")
    );
    let OutlineFeature::Polygon { points, .. } = polygon else {
        unreachable!()
    };
    assert_eq!(perimeter.points, points);
    Arc::make_mut(&mut snapshot.document).board_outlines[0].versions[0]
        .geometry
        .features
        .retain(|feature| !matches!(feature, OutlineFeature::Polygon { .. }));
    assert!(
        editable_perimeter(
            &snapshot,
            &probe.scope.board_id,
            Some("fixed-outline-v1"),
            None,
        )
        .is_none()
    );
    root.remove();
}

#[wasm_bindgen_test]
async fn mounted_reopened_fixed_outline_selects_its_saved_version() {
    use boardstudio_model::model::{
        CornerStyle, OutlineProvenance, OutlineSettings, OutlineSnapshot, OutlineVersion,
    };
    let (_probe, root, _mounted) = mounted_outline_inspector_with_version(Some(OutlineVersion {
        id: "saved-outline".into(),
        name: "QA Outline".into(),
        source: OutlineProvenance {
            revision: 3,
            version_id: None,
        },
        geometry: OutlineSnapshot {
            features: vec![],
            settings: OutlineSettings {
                corners: CornerStyle::Chamfer,
                size: 7.25,
                ..OutlineSettings::default()
            },
            expected_regions: 1,
            bridges: vec![],
            protected_gaps: vec![],
        },
        ownership: boardstudio_model::model::OutlineOwnership::Fixed,
    }))
    .await;
    wait_for_button(&root, "the Active outline select mounts", "Active outline").await;
    let select = by_role_name(&root, "button", "Active outline").unwrap();
    assert_eq!(
        select.text_content().as_deref(),
        Some("Edited outline · QA Outline"),
        "initially mounted saved fixed version must not display Generated"
    );
    let size = by_role_name(&root, "spinbutton", "Chamfer size")
        .unwrap()
        .dyn_into::<web_sys::HtmlInputElement>()
        .unwrap();
    assert_eq!(size.value(), "7.25");
    root.remove();
}

#[wasm_bindgen_test]
async fn mounted_outline_inspector_generates_through_the_production_owner() {
    let (probe, root, _mounted) = mounted_outline_inspector().await;
    wait_for_button(
        &inspector_root(),
        "the Generate action mounts",
        "Generate automatic outline",
    )
    .await;
    // Hold the mounted button: the second click below reuses this element, whose
    // handler settles Unchanged once the outline exists.
    let generate = button(&inspector_root(), "Generate automatic outline");
    generate.click();
    settle_dimension().await;
    accept(&probe).await;
    let document = accepted_document(&probe);
    let created = document
        .boards
        .iter()
        .find(|board| board.id == probe.scope.board_id)
        .is_some_and(|board| {
            board.outline_ids.iter().any(|id| {
                document.outline.iter().any(|feature| {
                    feature.id() == id
                        && matches!(
                            feature,
                            boardstudio_model::model::OutlineFeature::PartEnvelope { .. }
                        )
                })
            })
        });
    assert!(
        created,
        "the mounted Inspector's generate action leaves a generated feature accepted"
    );
    // One accepted edit registered the envelope and the board membership together.
    let revision_after_generate = accepted_revision(&probe);
    assert_eq!(
        document.outline.len(),
        1,
        "Generate authors exactly one Generated envelope"
    );
    assert_eq!(
        document.boards[0].outline_ids.len(),
        1,
        "the same accepted edit registers the board membership"
    );

    // A second Generate is Unchanged: no new envelope, no revision step.
    generate.click();
    settle_dimension().await;
    accept(&probe).await;
    let repeated = accepted_document(&probe);
    assert_eq!(
        repeated.outline.len(),
        1,
        "a second Generate adds no second Generated envelope"
    );
    assert_eq!(
        repeated.boards[0].outline_ids.len(),
        1,
        "a second Generate adds no second membership"
    );
    assert_eq!(
        accepted_revision(&probe),
        revision_after_generate,
        "an already-generated outline settles Unchanged without a revision step"
    );

    // One Undo removes the envelope and its membership together.
    undo(&probe.runtime).await;
    accept(&probe).await;
    let undone = accepted_document(&probe);
    assert!(
        undone.outline.is_empty(),
        "one Undo removes the Generated envelope"
    );
    assert!(
        undone.boards[0].outline_ids.is_empty(),
        "the same Undo removes the board membership"
    );
    root.remove();
}

#[wasm_bindgen_test]
async fn mounted_generated_perimeter_insert_creates_one_fixed_copy_with_edited_points() {
    let (probe, root, _mounted) = mounted_polygon_outline_inspector(false).await;
    open_perimeter_editor().await;
    wait_for_button(
        &points_root(),
        "Insert point renders in the point editor",
        "Insert after 1",
    )
    .await;
    click_button(&points_root(), "Insert after 1");
    // The insert's effect is the accepted revision, which the accept drain below reads.
    settle_dimension().await;

    accept(&probe).await;
    let source_points = probe
        .runtime
        .model()
        .accepted
        .map(|snapshot| snapshot.scene.board_outline_scenes.clone())
        .and_then(|scenes| {
            scenes
                .iter()
                .find(|scene| scene.board_id == probe.scope.board_id)
                .and_then(|scene| {
                    scene
                        .source_contours
                        .first()
                        .map(|contour| contour.points.len())
                })
        });
    let document = accepted_document(&probe);
    let outline = document
        .board_outlines
        .iter()
        .find(|outline| outline.board_id == probe.scope.board_id)
        .expect("the first Generated edit creates a fixed copy");
    let version = outline
        .versions
        .iter()
        .find(|version| Some(&version.id) == outline.active_version_id.as_ref())
        .expect("the fixed copy is active");
    assert_eq!(version.name, "Edited outline 1");
    assert!(version.id.starts_with("outline-version-"));
    let OutlineFeature::Polygon { points, .. } = &version.geometry.features[0] else {
        panic!("the fixed copy holds the edited polygon")
    };
    assert_eq!(
        Some(points.len()),
        source_points.map(|count| count + 1),
        "one inserted point on the accepted contour"
    );
    root.remove();
}

#[wasm_bindgen_test]
async fn mounted_fixed_perimeter_coordinate_enter_updates_only_the_active_feature() {
    let (probe, root, _mounted) = mounted_polygon_outline_inspector(true).await;
    open_perimeter_editor().await;
    wait_until("the point editor renders Point 1 X", || {
        by_role_name(&points_root(), "spinbutton", "Point 1 X mm").is_some()
    })
    .await;
    let input = by_role_name(&points_root(), "spinbutton", "Point 1 X mm")
        .unwrap()
        .dyn_into::<web_sys::HtmlInputElement>()
        .unwrap();
    input.focus().unwrap();
    send_input(&input, "9.5");
    send_key(&input, "Escape");
    wait_until("Escape restores the accepted X coordinate", || {
        input.value() == "0"
    })
    .await;
    assert_eq!(input.value(), "0");
    let selected = *probe.selected_context.borrow().as_ref().unwrap();
    assert!(
        matches!(
            selected.read().as_ref().map(|selected| &selected.context),
            Some(super::super::objects::TreeContext::Outline { .. })
        ),
        "Escape first discards the coordinate draft without leaving the Perimeter inspector"
    );

    input.focus().unwrap();
    send_input(&input, "1.5");
    send_key(&input, "Enter");
    settle_dimension().await;

    accept(&probe).await;
    let document = accepted_document(&probe);
    let feature = document.board_outlines[0].versions[0]
        .geometry
        .features
        .iter()
        .find(|feature| feature.id() == "fixed-contour")
        .expect("the fixed contour stays accepted");
    let OutlineFeature::Polygon { points, .. } = feature else {
        panic!("a fixed edit updates its accepted outline feature")
    };
    assert_eq!(points.len(), 4);
    assert_eq!(points[0].x, 1.5);
    assert_eq!(points[0].y, 0.0);
    root.remove();
}

#[path = "outline_lifecycle_browser_tests/dimension_tests.rs"]
mod dimension_tests;
#[path = "outline_lifecycle_browser_tests/gap_tests.rs"]
mod gap_tests;
#[path = "outline_lifecycle_browser_tests/ownership_presentation.rs"]
mod ownership_presentation;
#[path = "outline_lifecycle_browser_tests/product_context.rs"]
mod product_context;
