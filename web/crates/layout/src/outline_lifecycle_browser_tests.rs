use super::*;
use boardstudio_application::{
    AcceptedSnapshot, Durability, Lifecycle, ReadModel, Scope, SessionEpoch, SnapshotToken,
};
use boardstudio_core::model::{
    Board, BoardContours, BoardOutline, Contour, CoreReply, CoreRequest, EditOperation,
    OutlineConnection, OutlineControlPoint, OutlineFeature, OutlineProvenance, OutlineSnapshot,
    OutlineVersion, Part, PartDefinition, PartKind, Pose2, ProjectDoc, Readiness, SceneDelta, Side,
    Vec2,
};
use std::sync::Arc;
use std::{cell::RefCell, rc::Rc};
use wasm_bindgen::JsCast;
use wasm_bindgen_test::wasm_bindgen_test;

wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
fn outline_version_id_avoids_ids_saved_before_runtime_restart() {
    assert_eq!(
        unique_outline_version_id(1, ["outline-version-1".to_owned()]),
        "outline-version-1-2",
        "the first operation after reopening must not reuse a saved version ID"
    );
    assert_eq!(
        unique_outline_version_id(
            1,
            [
                "outline-version-1".to_owned(),
                "outline-version-1-2".to_owned(),
            ],
        ),
        "outline-version-1-3",
        "collisions choose the next deterministic unused suffix"
    );
}

#[wasm_bindgen_test]
fn outline_connection_id_avoids_ids_saved_before_runtime_restart() {
    assert_eq!(
        unique_outline_entity_id("outline-connection", 1, ["outline-connection-1".to_owned()],),
        "outline-connection-1-2",
        "a new Generated connection must not reuse an ID saved before Runtime restarted"
    );
    assert_eq!(
        unique_outline_entity_id(
            "outline-connection",
            1,
            [
                "outline-connection-1".to_owned(),
                "outline-connection-1-2".to_owned(),
            ],
        ),
        "outline-connection-1-3",
        "collisions choose the next deterministic unused suffix"
    );
}

#[derive(Clone)]
struct DimensionProbe {
    committed: Rc<RefCell<Vec<f64>>>,
}

fn mounted_dimension_host() -> Element {
    let probe = use_context::<DimensionProbe>();
    let committed = probe.committed.clone();
    let on_commit = use_callback(move |value: f64| committed.borrow_mut().push(value));
    rsx! {
        div { id: "outline-dimension-test-root",
            OutlineDimension {
                label: "Outline margin",
                value: 4.0,
                minimum: 0.0,
                editable: true,
                on_commit,
            }
        }
    }
}

fn coordinate_history_host() -> Element {
    let mut accepted = use_signal(|| 0.0);
    rsx! {
        OutlineCoordinate { label: "Point 1 X mm".to_owned(), value: accepted(), editable: true, on_commit: |_| {} }
        OutlineCoordinate { label: "Point 1 Y mm".to_owned(), value: 10.0, editable: true, on_commit: |_| {} }
        button { id: "coordinate-accepted", onclick: move |_| accepted.set(1.5), "Accept coordinate" }
        button { id: "coordinate-undo", onclick: move |_| accepted.set(0.0), "Undo coordinate" }
    }
}

#[wasm_bindgen_test]
async fn coordinate_undo_restores_accepted_value_without_reviving_a_submitted_draft() {
    let document = web_sys::window().unwrap().document().unwrap();
    let root = document.create_element("div").unwrap();
    root.set_id("coordinate-history-mount");
    document.body().unwrap().append_child(&root).unwrap();
    dioxus_web::launch::launch_virtual_dom(
        VirtualDom::new(coordinate_history_host),
        dioxus_web::Config::new().rootnode(root.clone().into()),
    );
    settle_dimension().await;
    let x = root
        .query_selector("input[aria-label='Point 1 X mm']")
        .unwrap()
        .unwrap()
        .dyn_into::<web_sys::HtmlInputElement>()
        .unwrap();
    let y = root
        .query_selector("input[aria-label='Point 1 Y mm']")
        .unwrap()
        .unwrap()
        .dyn_into::<web_sys::HtmlInputElement>()
        .unwrap();
    send_input(&y, "");
    x.focus().unwrap();
    send_input(&x, "1.5");
    send_key(&x, "Enter");
    root.query_selector("#coordinate-accepted")
        .unwrap()
        .unwrap()
        .dyn_into::<web_sys::HtmlElement>()
        .unwrap()
        .click();
    settle_dimension().await;
    assert_eq!(x.value(), "1.5");
    assert_eq!(
        y.value(),
        "",
        "another coordinate's dirty draft must remain local"
    );
    root.query_selector("#coordinate-undo")
        .unwrap()
        .unwrap()
        .dyn_into::<web_sys::HtmlElement>()
        .unwrap()
        .click();
    settle_dimension().await;
    assert_eq!(
        x.value(),
        "0",
        "Undo must show the restored accepted coordinate"
    );
    assert_eq!(
        y.value(),
        "",
        "Undo on X must preserve the unrelated Y draft"
    );
    root.remove();
}

fn mount_dimension() -> (DimensionProbe, web_sys::Element) {
    let probe = DimensionProbe {
        committed: Rc::default(),
    };
    let document = web_sys::window().unwrap().document().unwrap();
    let root = document.create_element("div").unwrap();
    root.set_id("outline-dimension-mount");
    document.body().unwrap().append_child(&root).unwrap();
    let dom = VirtualDom::new(mounted_dimension_host);
    dom.provide_root_context(probe.clone());
    dioxus_web::launch::launch_virtual_dom(
        dom,
        dioxus_web::Config::new().rootnode(root.clone().into()),
    );
    (probe, root)
}

async fn settle_dimension() {
    gloo_timers::future::TimeoutFuture::new(35).await;
}

fn dimension_input() -> web_sys::HtmlInputElement {
    web_sys::window()
        .unwrap()
        .document()
        .unwrap()
        .query_selector(
            "#outline-dimension-mount #outline-dimension-test-root input[aria-label='Outline margin']",
        )
        .unwrap()
        .unwrap()
        .dyn_into::<web_sys::HtmlInputElement>()
        .unwrap()
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

#[wasm_bindgen_test]
async fn outline_dimension_enter_commits_and_escape_restores_accepted_value() {
    let (probe, root) = mount_dimension();
    settle_dimension().await;
    let input = dimension_input();

    input.focus().unwrap();
    send_input(&input, "5.25");
    send_key(&input, "Enter");
    settle_dimension().await;
    assert_eq!(probe.committed.borrow().as_slice(), &[5.25]);

    input.focus().unwrap();
    send_input(&input, "9");
    send_key(&input, "Escape");
    settle_dimension().await;
    assert_eq!(input.value(), "4");
    input.blur().unwrap();
    settle_dimension().await;
    assert_eq!(
        probe.committed.borrow().as_slice(),
        &[5.25],
        "Escape restores the accepted baseline without submitting"
    );
    root.remove();
}

#[derive(Clone)]
struct InspectorProbe {
    runtime: Rc<crate::runtime::Runtime>,
    scope: Scope,
    connection_action: Rc<RefCell<Option<(Rc<OutlineActionContext>, EventHandler<OutlineAction>)>>>,
    selected_context: Rc<RefCell<Option<Signal<Option<super::super::objects::ScopedTreeContext>>>>>,
}

impl InspectorProbe {
    fn add_connection(&self, points: Vec<Vec2>) {
        let action = self
            .connection_action
            .borrow()
            .as_ref()
            .map(|(context, handler)| (context.add_connection(points), handler.clone()))
            .expect("mounted Inspector publishes its production action owner");
        action.1.call(action.0);
    }
}

fn mounted_outline_inspector_host() -> Element {
    let probe = use_context::<InspectorProbe>();
    let version = use_signal(|| 0u64);
    use_context_provider(|| version);
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
        *probe.connection_action.borrow_mut() = Some((
            projection.action_context.clone(),
            projection.on_action.clone(),
        ));
    }
    let _ = version();
    let board_projection = board_inspector.projection;
    let board_rename = board_inspector.on_rename;
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
        style { {include_str!("../../assets/m1.css")} }
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
        if let Some(projection) = board_projection {
            super::super::board_inspector::BoardInspector {
                projection,
                on_rename: board_rename,
            }
        }
    }
}

fn mounted_outline_inspector() -> (InspectorProbe, web_sys::Element) {
    mounted_outline_inspector_with_version(None)
}

fn mounted_outline_inspector_with_version(
    version: Option<boardstudio_core::model::OutlineVersion>,
) -> (InspectorProbe, web_sys::Element) {
    let scope = Scope {
        session_epoch: SessionEpoch(5),
        document_id: "outline-inspector-doc".into(),
        board_id: "outline-board".into(),
        instance_id: None,
    };
    let mut document = ProjectDoc::empty("outline-inspector-doc", "Outline fixture");
    document.revision = 9;
    document.boards.push(Board {
        id: scope.board_id.clone(),
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
            .push(boardstudio_core::model::BoardOutline {
                board_id: scope.board_id.clone(),
                active_version_id: Some(version.id.clone()),
                versions: vec![version],
                generated_last_valid: None,
            });
    }
    let snapshot = AcceptedSnapshot {
        token: SnapshotToken(13),
        session_epoch: scope.session_epoch,
        document: Arc::new(document),
        scene: Arc::new(SceneDelta {
            module_scenes: vec![],
            revision: 9,
            transaction_id: "outline-inspector-fixture".into(),
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
    };
    let model = ReadModel {
        lifecycle: Lifecycle::Ready,
        durability: Durability::Saved { revision: 9 },
        accepted: Some(snapshot),
        active_board_id: scope.board_id.clone(),
        ..ReadModel::default()
    };
    let runtime = crate::runtime::Runtime::new().expect("browser Runtime initializes");
    runtime.set_layout_component_inspector_test_state(model, Some(scope.clone()));
    let probe = InspectorProbe {
        runtime,
        scope,
        connection_action: Rc::default(),
        selected_context: Rc::default(),
    };
    let document = web_sys::window().unwrap().document().unwrap();
    let root = document.create_element("div").unwrap();
    root.set_id("outline-inspector-mount");
    document.body().unwrap().append_child(&root).unwrap();
    let dom = VirtualDom::new(mounted_outline_inspector_host);
    dom.provide_root_context(probe.clone());
    dioxus_web::launch::launch_virtual_dom(
        dom,
        dioxus_web::Config::new().rootnode(root.clone().into()),
    );
    (probe, root)
}

#[wasm_bindgen_test]
async fn mounted_outline_perimeter_escape_returns_to_board_inspector() {
    let (probe, root) = mounted_polygon_outline_inspector(true);
    settle_dimension().await;
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

    let open = document
        .query_selector("#outline-points-inspector-mount button.m1-outline-action")
        .unwrap()
        .unwrap()
        .dyn_into::<web_sys::HtmlElement>()
        .unwrap();
    open.click();
    settle_dimension().await;
    let done = document
        .query_selector("#outline-points-inspector-mount .m1-outline-point-editor button")
        .unwrap()
        .unwrap()
        .dyn_into::<web_sys::HtmlElement>()
        .unwrap();
    done.focus().unwrap();
    let init = web_sys::KeyboardEventInit::new();
    init.set_key("Escape");
    init.set_bubbles(true);
    done.dispatch_event(
        &web_sys::KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &init).unwrap(),
    )
    .unwrap();
    settle_dimension().await;

    assert!(
        matches!(
            selection.read().as_ref().map(|selected| &selected.context),
            Some(super::super::objects::TreeContext::Board { board_id }) if board_id == &probe.scope.board_id
        ),
        "Escape from the focused Perimeter Done button returns selection to the board inspector"
    );
    assert!(
        document
            .query_selector("#outline-points-inspector-mount .m1-outline-point-editor")
            .unwrap()
            .is_none(),
        "the nested Perimeter inspector is no longer mounted after returning to Board"
    );
    let board_name = document
        .query_selector("#outline-points-inspector-mount input[aria-label='Board name']")
        .unwrap()
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
    let (probe, root) = mounted_polygon_outline_inspector(true);
    settle_dimension().await;
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

    document
        .query_selector("#outline-points-inspector-mount button.m1-outline-action")
        .unwrap()
        .expect("fixed outline exposes Edit perimeter points")
        .dyn_into::<web_sys::HtmlElement>()
        .unwrap()
        .click();
    settle_dimension().await;

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
    settle_dimension().await;

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
            .query_selector("#outline-points-inspector-mount .m1-outline-point-editor")
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
        document
            .query_selector("#outline-points-inspector-mount button.m1-outline-action")
            .unwrap()
            .is_some(),
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
        Vec2 { x: -8.2375, y: -13.0 },
        Vec2 { x: 13.0, y: -13.0 },
        Vec2 { x: 13.0, y: 13.0 },
    ];
    let (probe, root) = mounted_polygon_outline_inspector_with_points(true, points);
    settle_dimension().await;
    let document = web_sys::window().unwrap().document().unwrap();
    let _ = probe.runtime.take_layout_component_inspector_test_events();

    document
        .query_selector("#outline-points-inspector-mount button.m1-outline-action")
        .unwrap()
        .expect("fixed outline exposes Edit perimeter points")
        .dyn_into::<web_sys::HtmlElement>()
        .unwrap()
        .click();
    settle_dimension().await;

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
    settle_dimension().await;
    assert_eq!(
        captured.captured.get(),
        None,
        "the mounted production pointerup handler finished the point interaction"
    );
    let pointer_events = probe.runtime.take_layout_component_inspector_test_events();
    assert!(
        pointer_events.is_empty(),
        "a pointer interaction without pointer movement must not submit an outline edit; got {pointer_events:?}"
    );

    dispatch_test_pointer_at(&point, "pointerdown", 1, client_x, client_y);
    assert_eq!(captured.captured.get(), Some(17));
    dispatch_test_pointer_at(&point, "pointerup", 0, client_x + 4, client_y);
    settle_dimension().await;
    assert_eq!(captured.captured.get(), None);
    let displacement_events = probe.runtime.take_layout_component_inspector_test_events();
    assert!(
        matches!(displacement_events.as_slice(), [boardstudio_application::Event::Edit { command, .. }]
            if command.phase == boardstudio_core::model::EditPhase::Commit),
        "a pointer-up displaced from pointer-down still commits the final sample when no pointermove was delivered; got {displacement_events:?}"
    );

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

    assert!(probe.runtime.take_layout_component_inspector_test_events().is_empty());
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
    let has_capture = Closure::<dyn FnMut(i32) -> bool>::new(move |id| {
        has_capture_state.get() == Some(id)
    });
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

fn mounted_polygon_outline_inspector(fixed: bool) -> (InspectorProbe, web_sys::Element) {
    mounted_polygon_outline_inspector_with_connections(fixed, vec![])
}

fn mounted_polygon_outline_inspector_with_connections(
    fixed: bool,
    connections: Vec<OutlineConnection>,
) -> (InspectorProbe, web_sys::Element) {
    let points = vec![
        boardstudio_core::model::Vec2 { x: 0.0, y: 0.0 },
        boardstudio_core::model::Vec2 { x: 20.0, y: 0.0 },
        boardstudio_core::model::Vec2 { x: 20.0, y: 20.0 },
        boardstudio_core::model::Vec2 { x: 0.0, y: 20.0 },
    ];
    mounted_polygon_outline_inspector_with_points_and_connections(fixed, points, connections)
}

fn mounted_polygon_outline_inspector_with_points(
    fixed: bool,
    points: Vec<Vec2>,
) -> (InspectorProbe, web_sys::Element) {
    mounted_polygon_outline_inspector_with_points_and_connections(fixed, points, vec![])
}

fn mounted_polygon_outline_inspector_with_points_and_connections(
    fixed: bool,
    points: Vec<Vec2>,
    connections: Vec<OutlineConnection>,
) -> (InspectorProbe, web_sys::Element) {
    let scope = Scope {
        session_epoch: SessionEpoch(6),
        document_id: "outline-points-doc".into(),
        board_id: "outline-points-board".into(),
        instance_id: None,
    };
    let mut document = ProjectDoc::empty("outline-points-doc", "Outline points fixture");
    document.revision = 11;
    let has_envelope_part = !connections.is_empty();
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
            properties: None,
            generator_parameters: None,
        });
    }
    document.boards.push(Board {
        id: scope.board_id.clone(),
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
            board_id: scope.board_id.clone(),
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
                        operation: boardstudio_core::model::Operation::Add,
                    }],
                    settings: boardstudio_core::model::OutlineSettings::default(),
                    expected_regions: 1,
                    bridges: vec![],
                    protected_gaps: vec![],
                },
            }],
            generated_last_valid: None,
        });
    } else {
        document.outline.push(OutlineFeature::PartEnvelope {
            connections,
            settings: boardstudio_core::model::OutlineSettings::default(),
            id: "generated-outline".into(),
            part_ids: has_envelope_part
                .then(|| "k1".to_owned())
                .into_iter()
                .collect(),
            margin: 4.0,
            operation: boardstudio_core::model::Operation::Add,
        });
    }
    let snapshot = AcceptedSnapshot {
        token: SnapshotToken(21),
        session_epoch: scope.session_epoch,
        document: Arc::new(document),
        scene: Arc::new(SceneDelta {
            module_scenes: vec![],
            revision: 11,
            transaction_id: "outline-points-fixture".into(),
            changed_ids: vec![],
            transforms: vec![],
            matrix_scenes: vec![],
            contours: vec![],
            board_contours: vec![BoardContours {
                board_id: scope.board_id.clone(),
                contours: vec![Contour {
                    points: points.clone(),
                    hole: false,
                }],
            }],
            board_readiness: vec![],
            board_outline_scenes: vec![boardstudio_core::model::BoardOutlineScene {
                board_id: scope.board_id.clone(),
                source_contours: vec![Contour {
                    points,
                    hole: false,
                }],
                bridges: vec![],
                gaps: vec![],
            }],
            finding_markers: vec![],
            findings: vec![],
            readiness: Readiness {
                layout: true,
                outline: true,
                pcb: true,
                case_ready: false,
            },
        }),
    };
    let model = ReadModel {
        lifecycle: Lifecycle::Ready,
        durability: Durability::Saved { revision: 11 },
        accepted: Some(snapshot),
        active_board_id: scope.board_id.clone(),
        ..ReadModel::default()
    };
    let runtime = crate::runtime::Runtime::new().expect("browser Runtime initializes");
    runtime.set_layout_component_inspector_test_state(model, Some(scope.clone()));
    let probe = InspectorProbe {
        runtime,
        scope,
        connection_action: Rc::default(),
        selected_context: Rc::default(),
    };
    let document = web_sys::window().unwrap().document().unwrap();
    let root = document.create_element("div").unwrap();
    root.set_id("outline-points-inspector-mount");
    document.body().unwrap().append_child(&root).unwrap();
    let dom = VirtualDom::new(mounted_outline_inspector_host);
    dom.provide_root_context(probe.clone());
    dioxus_web::launch::launch_virtual_dom(
        dom,
        dioxus_web::Config::new().rootnode(root.clone().into()),
    );
    (probe, root)
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
    let (probe, root) = mounted_polygon_outline_inspector_with_connections(false, vec![existing]);
    settle_dimension().await;
    probe.add_connection(vec![Vec2 { x: -6.0, y: 0.0 }, Vec2 { x: -16.0, y: 0.0 }]);
    settle_dimension().await;

    let events = probe.runtime.take_layout_component_inspector_test_events();
    let command = match events.as_slice() {
        [boardstudio_application::Event::Edit { command, .. }] => command.clone(),
        _ => panic!("adding a Generated connection submits one edit"),
    };
    let EditOperation::SetOutline { feature } = &command.operation else {
        panic!("Generated connection edits the accepted PartEnvelope")
    };
    let OutlineFeature::PartEnvelope { connections, .. } = feature else {
        panic!("the connection edit retains a PartEnvelope")
    };
    assert_eq!(
        connections
            .iter()
            .map(|connection| connection.id.as_str())
            .collect::<Vec<_>>(),
        ["outline-connection-1", "outline-connection-1-2"]
    );

    let mut core = boardstudio_core::CoreEngine::new();
    let baseline = probe.runtime.model().accepted.unwrap();
    assert!(matches!(
        core.handle(CoreRequest::Open {
            id: "open-outline-connection-fixture".into(),
            document: baseline.document.as_ref().clone(),
        }),
        CoreReply::Scene { .. }
    ));
    let reply = core.handle(CoreRequest::Edit {
        id: "add-outline-connection-after-reopen".into(),
        command,
    });
    let CoreReply::Scene {
        scene, document, ..
    } = reply
    else {
        panic!("unique saved connection IDs must remain a valid accepted Core edit")
    };
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
        "outline-connection-1-2",
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
fn fixed_perimeter_finds_the_first_polygon_after_primitive_features() {
    let (probe, root) = mounted_polygon_outline_inspector(true);
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
    use boardstudio_core::model::{
        CornerStyle, OutlineProvenance, OutlineSettings, OutlineSnapshot, OutlineVersion,
    };
    let (probe, root) = mounted_outline_inspector_with_version(Some(OutlineVersion {
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
    }));
    settle_dimension().await;
    let select = root
        .query_selector("select[aria-label='Active outline']")
        .unwrap()
        .unwrap();
    assert_eq!(
        js_sys::Reflect::get(&select, &"value".into())
            .unwrap()
            .as_string()
            .as_deref(),
        Some("saved-outline"),
        "initially mounted saved fixed version must not display Generated"
    );
    let size = root
        .query_selector("input[aria-label='Chamfer size']")
        .unwrap()
        .unwrap()
        .dyn_into::<web_sys::HtmlInputElement>()
        .unwrap();
    assert_eq!(size.value(), "7.25");
    assert!(
        probe
            .runtime
            .take_layout_component_inspector_test_events()
            .is_empty()
    );
    root.remove();
}

#[wasm_bindgen_test]
async fn mounted_outline_inspector_generates_through_the_production_owner() {
    let (probe, root) = mounted_outline_inspector();
    settle_dimension().await;
    let button = web_sys::window()
        .unwrap()
        .document()
        .unwrap()
        .query_selector("#outline-inspector-mount button.m1-outline-action")
        .unwrap()
        .unwrap()
        .dyn_into::<web_sys::HtmlElement>()
        .unwrap();
    button.click();
    settle_dimension().await;
    let events = probe.runtime.take_layout_component_inspector_test_events();
    let operation = match events.as_slice() {
        [boardstudio_application::Event::Edit { operation_id, .. }] => *operation_id,
        _ => panic!("the production owner should submit exactly one edit"),
    };
    let created = match events.as_slice() {
        [boardstudio_application::Event::Edit { command, .. }] => {
            let boardstudio_core::model::EditOperation::ReplaceDocument { document } =
                &command.operation
            else {
                panic!("automatic outline creation uses the existing document edit")
            };
            document
                .boards
                .iter()
                .find(|board| board.id == probe.scope.board_id)
                .is_some_and(|board| {
                    board.outline_ids.iter().any(|id| {
                        document.outline.iter().any(|feature| {
                            feature.id() == id
                                && matches!(
                                    feature,
                                    boardstudio_core::model::OutlineFeature::PartEnvelope { .. }
                                )
                        })
                    })
                })
        }
        _ => false,
    };
    assert!(
        created,
        "the mounted Inspector emits a generated feature edit"
    );
    assert!(
        probe
            .runtime
            .settle_layout_component_inspector_test_operation(
                operation,
                boardstudio_application::TerminalOutcome::Rejected("fixture rejection".into()),
            )
    );
    settle_dimension().await;
    root.remove();
}

#[wasm_bindgen_test]
async fn mounted_generated_perimeter_insert_creates_one_fixed_copy_with_edited_points() {
    let (probe, root) = mounted_polygon_outline_inspector(false);
    settle_dimension().await;
    let document = web_sys::window().unwrap().document().unwrap();
    let open = document
        .query_selector("#outline-points-inspector-mount button.m1-outline-action")
        .unwrap()
        .unwrap()
        .dyn_into::<web_sys::HtmlElement>()
        .unwrap();
    open.click();
    settle_dimension().await;
    let insert = document
        .query_selector("#outline-points-inspector-mount button.m1-outline-insert-point")
        .unwrap()
        .unwrap()
        .dyn_into::<web_sys::HtmlElement>()
        .unwrap();
    insert.click();
    settle_dimension().await;

    let events = probe.runtime.take_layout_component_inspector_test_events();
    let operation = match events.as_slice() {
        [boardstudio_application::Event::Edit { command, .. }] => &command.operation,
        _ => panic!("inserting a Generated outline point submits one edit"),
    };
    let boardstudio_core::model::EditOperation::CopyOutline {
        board_id,
        version_id,
        name,
        edit: Some(edit),
        feature: None,
    } = operation
    else {
        panic!("the first Generated edit creates a fixed copy with its point edit")
    };
    assert_eq!(board_id, &probe.scope.board_id);
    assert_eq!(name, "Edited outline 1");
    assert!(version_id.starts_with("outline-version-"));
    assert_eq!(edit.contour, 0);
    assert_eq!(edit.points.len(), 5);
    assert_eq!(edit.points[1].x, 10.0);
    assert_eq!(edit.points[1].y, 0.0);
    root.remove();
}

#[wasm_bindgen_test]
async fn mounted_fixed_perimeter_coordinate_enter_updates_only_the_active_feature() {
    let (probe, root) = mounted_polygon_outline_inspector(true);
    settle_dimension().await;
    let document = web_sys::window().unwrap().document().unwrap();
    let open = document
        .query_selector("#outline-points-inspector-mount button.m1-outline-action")
        .unwrap()
        .unwrap()
        .dyn_into::<web_sys::HtmlElement>()
        .unwrap();
    open.click();
    settle_dimension().await;
    let input = document
        .query_selector("#outline-points-inspector-mount input[aria-label='Point 1 X mm']")
        .unwrap()
        .unwrap()
        .dyn_into::<web_sys::HtmlInputElement>()
        .unwrap();
    input.focus().unwrap();
    send_input(&input, "9.5");
    send_key(&input, "Escape");
    settle_dimension().await;
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

    let events = probe.runtime.take_layout_component_inspector_test_events();
    let operation = match events.as_slice() {
        [boardstudio_application::Event::Edit { command, .. }] => &command.operation,
        _ => panic!("a finite coordinate Enter submits one edit"),
    };
    let boardstudio_core::model::EditOperation::SetOutline {
        feature:
            OutlineFeature::Polygon {
                id,
                points,
                operation: boardstudio_core::model::Operation::Add,
                ..
            },
    } = operation
    else {
        panic!("a fixed edit updates its accepted outline feature")
    };
    assert_eq!(id, "fixed-contour");
    assert_eq!(points.len(), 4);
    assert_eq!(points[0].x, 1.5);
    assert_eq!(points[0].y, 0.0);
    root.remove();
}
