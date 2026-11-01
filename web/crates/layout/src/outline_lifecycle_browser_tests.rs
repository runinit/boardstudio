use super::*;
use boardstudio_application::Scope;
use boardstudio_core::model::{
    Board, BoardOutline, OutlineConnection, OutlineControlPoint, OutlineFeature, OutlineProvenance,
    OutlineSnapshot, OutlineVersion, Part, PartDefinition, PartKind, Pose2, ProjectDoc, Side, Vec2,
};
use boardstudio_web_ui_shared::test_wait::wait_until;
use std::{cell::RefCell, rc::Rc, sync::Arc};
use wasm_bindgen::JsCast;
use wasm_bindgen_test::wasm_bindgen_test;

wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

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
    wait_until("the coordinate inputs mount", || {
        root.query_selector("input[aria-label='Point 1 X mm']")
            .unwrap()
            .is_some()
    })
    .await;
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
    wait_until("Undo shows the restored accepted coordinate", || {
        x.value() == "0"
    })
    .await;
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

/// Drive the in-process Session and Core until every queued edit has settled.
async fn accept(probe: &InspectorProbe) {
    for _ in 0..6 {
        crate::runtime::project_name_test_support::run_pending(&probe.runtime).await;
        gloo_timers::future::TimeoutFuture::new(10).await;
    }
}

pub(super) async fn undo(runtime: &Rc<crate::runtime::Runtime>) {
    runtime.submit(boardstudio_application::Event::Undo {
        operation_id: runtime.operation(),
    });
    crate::runtime::project_name_test_support::run_pending(runtime).await;
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

fn accepted_document(probe: &InspectorProbe) -> boardstudio_core::model::ProjectDoc {
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
    wait_until("the outline margin input mounts", || {
        web_sys::window()
            .unwrap()
            .document()
            .unwrap()
            .query_selector(
                "#outline-dimension-mount #outline-dimension-test-root input[aria-label='Outline margin']",
            )
            .unwrap()
            .is_some()
    })
    .await;
    let input = dimension_input();

    input.focus().unwrap();
    send_input(&input, "5.25");
    send_key(&input, "Enter");
    wait_until("Enter commits 5.25", || {
        probe.committed.borrow().as_slice() == [5.25]
    })
    .await;
    assert_eq!(probe.committed.borrow().as_slice(), &[5.25]);

    input.focus().unwrap();
    send_input(&input, "9");
    send_key(&input, "Escape");
    wait_until("Escape restores the accepted baseline", || {
        input.value() == "4"
    })
    .await;
    assert_eq!(input.value(), "4");
    input.blur().unwrap();
    // Negative check (no commit): one event-loop turn, recorded as unproven in the Outcome.
    yield_turn().await;
    assert_eq!(
        probe.committed.borrow().as_slice(),
        &[5.25],
        "Escape restores the accepted baseline without submitting"
    );
    root.remove();
}

type OutlineConnectionAction = (Rc<OutlineActionContext>, EventHandler<OutlineAction>);

fn gap_world_points(document: &ProjectDoc, gap: &boardstudio_core::model::OutlineGap) -> Vec<Vec2> {
    gap.points
        .iter()
        .map(|point| {
            let Some(id) = &point.part_id else {
                return point.at;
            };
            let part = document.parts.iter().find(|part| &part.id == id).unwrap();
            let x = if part.side == Side::Back {
                -point.at.x
            } else {
                point.at.x
            };
            let (sin, cos) = part.pose.rotation.to_radians().sin_cos();
            Vec2 {
                x: part.pose.at.x + x * cos - point.at.y * sin,
                y: part.pose.at.y + x * sin + point.at.y * cos,
            }
        })
        .collect()
}

fn connected_mirrored_gap_document() -> ProjectDoc {
    use boardstudio_core::model::{CoreReply, CoreRequest};
    use serde_json::json;
    let mut input: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../core/tests/fixtures/synthetic-outline.json"
    ))
    .unwrap();
    for key in ["parts", "matrices", "layouts", "outline"] {
        input[key] = json!([]);
    }
    input["boards"][0]["partIds"] = json!([]);
    input["boards"][0]["outlineIds"] = json!([]);
    let mut engine = boardstudio_core::CoreEngine::new();
    let reply = engine.handle(CoreRequest::Open {
        id: "gap-fixture".into(),
        document: serde_json::from_value(input).unwrap(),
    });
    let CoreReply::Scene { document, .. } = reply else {
        panic!("the synthetic gap board opens: {reply:?}");
    };
    let apply = |engine: &mut boardstudio_core::CoreEngine, document: &ProjectDoc, operation| {
        let reply = engine.request(&json!({"id":"gap-fixture", "kind":"edit", "command":{
            "baseRevision":document.revision,"transactionId":format!("gap-{}",document.revision),
            "phase":"commit","targetIds":[],"operation":operation
        }}).to_string());
        let CoreReply::Scene {
            document, scene, ..
        } = serde_json::from_str(&reply).unwrap()
        else {
            panic!("fixture edit is accepted: {reply}");
        };
        (*document, scene)
    };
    let (document, _) = apply(
        &mut engine,
        &document,
        json!({"kind":"create-mirrored-pair",
            "matrix":{"id":"left-matrix","rows":1,"columns":2,"pitch":{"x":24.,"y":19.},
                "origin":{"x":-36.,"y":0.},"columnLayout": [{"offset": {"x": 0.0, "y": 0.0}}, {"offset": {"x": 0.0, "y": 10.0}}],
        "definitionId":"synthetic/switch","boardId":"main","partIds":[]},
            "left":{"id":"left-layout","name":"Left","boardId":"main","matrixIds":["left-matrix"],"partIds":[]},
            "right":{"id":"right-layout","name":"Right","boardId":"main","matrixIds":["right-matrix"],"partIds":[],
                "mirrorLink":{"sourceId":"left-layout","axisX":0.}}
        }),
    );
    let (document, _) = apply(
        &mut engine,
        &document,
        json!({"kind":"apply-outline-edit","edit":{
            "target":{"kind":"board","boardId":"main"},"change":{"create-generated":{
                "margin":1.,"part_ids":document.boards[0].part_ids,
                "settings":{"corners":"fillet","size":2.,"bridgeWidth":10.,
                    "repair":{"enabled":true,"maximumGapSpan":20.,"minimumConnectionWidth":2.,"edgeClearance":0.}}
            }},"seed":141,"reviewed":null
        }}),
    );
    let (mut document, mut scene) = apply(
        &mut engine,
        &document,
        json!({"kind":"apply-outline-edit","edit":{
            "target":{"kind":"board","boardId":"main"},"change":{"add-connection":{"connection":{
                "id":"cross-split","width":6.,"points":[{"at":{"x":-4.,"y":20.}},{"at":{"x":4.,"y":20.}}]
            }}},"seed":144,"reviewed":null
        }}),
    );
    for (index, side) in [-1., 1.].into_iter().enumerate() {
        let gap = scene
            .board_outline_scenes
            .iter()
            .find(|board| board.board_id == "main")
            .unwrap()
            .gaps
            .iter()
            .find(|gap| {
                gap_world_points(&document, gap)
                    .iter()
                    .all(|point| point.x * side > 20.)
            })
            .expect("each half has a distinct recess")
            .clone();
        (document, scene) = apply(
            &mut engine,
            &document,
            json!({"kind":"apply-outline-edit","edit":{
                "target":{"kind":"board","boardId":"main"},"change":{"set-protected-gap":{
                    "gap_id":gap.id,"points":gap.points
                }},"seed":150 + index,"reviewed":null
            }}),
        );
    }
    document
}

#[wasm_bindgen_test]
async fn connected_mirrored_outline_gap_controls_edit_distinct_regions() {
    for side in [-1., 1.] {
        let fixture = connected_mirrored_gap_document();
        let runtime = crate::runtime::project_name_test_support::new_runtime();
        crate::runtime::project_name_test_support::open_document(&runtime, fixture).await;
        let probe = InspectorProbe {
            scope: runtime.scope().unwrap(),
            runtime,
            connection_action: Rc::default(),
            selected_context: Rc::default(),
        };
        let document = web_sys::window().unwrap().document().unwrap();
        let root = document.create_element("div").unwrap();
        root.set_id("connected-gap-inspector");
        document.body().unwrap().append_child(&root).unwrap();
        let dom = VirtualDom::new(mounted_outline_inspector_host);
        dom.provide_root_context(probe.clone());
        dioxus_web::launch::launch_virtual_dom(
            dom,
            dioxus_web::Config::new().rootnode(root.clone().into()),
        );
        wait_until("the connected outline Inspector mounts", || {
            probe.connection_action.borrow().is_some()
        })
        .await;
        let before = probe.runtime.model().accepted.unwrap();
        assert_eq!(before.document.boards.len(), 1);
        assert_eq!(before.document.boards[0].outline_ids.len(), 1);
        let contours = |snapshot: &boardstudio_application::AcceptedSnapshot| {
            snapshot
                .scene
                .board_contours
                .iter()
                .find(|board| board.board_id == "main")
                .unwrap()
                .contours
                .clone()
        };
        assert_eq!(
            contours(&before)
                .iter()
                .filter(|contour| !contour.hole)
                .count(),
            1
        );
        let gaps = &before
            .scene
            .board_outline_scenes
            .iter()
            .find(|board| board.board_id == "main")
            .unwrap()
            .gaps;
        let selected = gaps
            .iter()
            .position(|gap| {
                gap.protected
                    && gap_world_points(&before.document, gap)
                        .iter()
                        .all(|point| point.x * side > 20.)
            })
            .unwrap();
        let other = gaps
            .iter()
            .find(|gap| {
                gap.protected
                    && gap_world_points(&before.document, gap)
                        .iter()
                        .all(|point| point.x * side < -20.)
            })
            .unwrap();
        let selected_id = gaps[selected].id.clone();
        let other_id = other.id.clone();
        assert_ne!(selected_id, other_id);
        assert_eq!(gaps[selected].feature_id, other.feature_id);
        assert!(
            gaps[selected]
                .protected_ids
                .iter()
                .all(|id| !other.protected_ids.contains(id))
        );
        let controls = root
            .query_selector_all("input[aria-label^='Keep gap']")
            .unwrap();
        assert_eq!(controls.length() as usize, gaps.len());
        let input = controls
            .item(selected as u32)
            .unwrap()
            .dyn_into::<web_sys::HtmlInputElement>()
            .unwrap();
        assert!(input.checked());
        input.set_checked(false);
        let event = web_sys::EventInit::new();
        event.set_bubbles(true);
        input
            .dispatch_event(&web_sys::Event::new_with_event_init_dict("change", &event).unwrap())
            .unwrap();
        accept(&probe).await;
        wait_until("the gap edit lands through the production resolver", || {
            probe
                .runtime
                .model()
                .accepted
                .as_ref()
                .unwrap()
                .document
                .revision
                > before.document.revision
        })
        .await;
        let after = probe.runtime.model().accepted.unwrap();
        let after_gaps = &after
            .scene
            .board_outline_scenes
            .iter()
            .find(|board| board.board_id == "main")
            .unwrap()
            .gaps;
        assert!(
            !after_gaps
                .iter()
                .find(|gap| gap.id == selected_id)
                .unwrap()
                .protected
        );
        assert!(
            after_gaps
                .iter()
                .find(|gap| gap.id == other_id)
                .unwrap()
                .protected
        );
        assert_eq!(
            contours(&after)
                .iter()
                .filter(|contour| !contour.hole)
                .count(),
            1
        );
        let half_points = |snapshot: &boardstudio_application::AcceptedSnapshot, side: f64| {
            let mut points: Vec<_> = contours(snapshot)
                .into_iter()
                .flat_map(|contour| contour.points)
                .filter(|point| point.x * side > 20.)
                .collect();
            points.sort_by(|a, b| a.x.total_cmp(&b.x).then(a.y.total_cmp(&b.y)));
            points
        };
        assert_ne!(half_points(&before, side), half_points(&after, side));
        assert_eq!(half_points(&before, -side), half_points(&after, -side));
        web_sys::console::log_1(&serde_json::json!({"ticket":141,"boardId":"main", "side":side,
            "selectedGapId":selected_id,"otherGapId":other_id,"featureId":gaps[selected].feature_id,
            "selectedWorldPoints":gap_world_points(&before.document, &gaps[selected]),
            "otherWorldPoints":gap_world_points(&before.document, other),
            "beforeGaps":gaps,"afterGaps":after_gaps,"beforeContours":contours(&before),"afterContours":contours(&after)
        }).to_string().into());
        root.remove();
    }
}

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
        style { {include_str!("../../../assets/m1.css")} }
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

async fn mounted_outline_inspector() -> (InspectorProbe, web_sys::Element) {
    mounted_outline_inspector_with_version(None).await
}

async fn mounted_outline_inspector_with_version(
    version: Option<boardstudio_core::model::OutlineVersion>,
) -> (InspectorProbe, web_sys::Element) {
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
            .push(boardstudio_core::model::BoardOutline {
                board_id: board_id.clone(),
                active_version_id: Some(version.id.clone()),
                versions: vec![version],
                generated_last_valid: None,
                retired_version_ids: vec![],
            });
    }
    let runtime = crate::runtime::project_name_test_support::new_runtime();
    crate::runtime::project_name_test_support::open_document(&runtime, document).await;
    let scope = runtime
        .scope()
        .expect("the opened outline fixture has a scope");
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
    let (probe, root) = mounted_polygon_outline_inspector(true).await;
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

    let open = document
        .query_selector("#outline-points-inspector-mount button.m1-outline-action")
        .unwrap()
        .unwrap()
        .dyn_into::<web_sys::HtmlElement>()
        .unwrap();
    open.click();
    wait_until("Edit perimeter points renders the point editor", || {
        document
            .query_selector("#outline-points-inspector-mount .m1-outline-point-editor button")
            .unwrap()
            .is_some()
    })
    .await;
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
    // Selection changes before the nested editor unmounts and before focus moves, so the wait
    // covers all three: the assertions below read the unmounted editor and the focused field.
    wait_until(
        "Escape from the focused Perimeter Done button returns selection to the board inspector",
        || {
            matches!(
                selection.read().as_ref().map(|selected| &selected.context),
                Some(super::super::objects::TreeContext::Board { board_id }) if board_id == &probe.scope.board_id
            ) && document
                .query_selector("#outline-points-inspector-mount .m1-outline-point-editor")
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
    let (probe, root) = mounted_polygon_outline_inspector(true).await;
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

    document
        .query_selector("#outline-points-inspector-mount button.m1-outline-action")
        .unwrap()
        .expect("fixed outline exposes Edit perimeter points")
        .dyn_into::<web_sys::HtmlElement>()
        .unwrap()
        .click();
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
            .query_selector("#outline-points-inspector-mount .m1-outline-point-editor")
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
        Vec2 {
            x: -8.2375,
            y: -13.0,
        },
        Vec2 { x: 13.0, y: -13.0 },
        Vec2 { x: 13.0, y: 13.0 },
    ];
    let (probe, root) = mounted_polygon_outline_inspector_with_points(true, points).await;
    wait_until("the fixed outline exposes Edit perimeter points", || {
        web_sys::window()
            .unwrap()
            .document()
            .unwrap()
            .query_selector("#outline-points-inspector-mount button.m1-outline-action")
            .unwrap()
            .is_some()
    })
    .await;
    let document = web_sys::window().unwrap().document().unwrap();
    let revision_before = accepted_revision(&probe);

    document
        .query_selector("#outline-points-inspector-mount button.m1-outline-action")
        .unwrap()
        .expect("fixed outline exposes Edit perimeter points")
        .dyn_into::<web_sys::HtmlElement>()
        .unwrap()
        .click();
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

async fn mounted_polygon_outline_inspector(fixed: bool) -> (InspectorProbe, web_sys::Element) {
    mounted_polygon_outline_inspector_with_connections(fixed, vec![]).await
}

async fn mounted_polygon_outline_inspector_with_connections(
    fixed: bool,
    connections: Vec<OutlineConnection>,
) -> (InspectorProbe, web_sys::Element) {
    let points = vec![
        boardstudio_core::model::Vec2 { x: 0.0, y: 0.0 },
        boardstudio_core::model::Vec2 { x: 20.0, y: 0.0 },
        boardstudio_core::model::Vec2 { x: 20.0, y: 20.0 },
        boardstudio_core::model::Vec2 { x: 0.0, y: 20.0 },
    ];
    mounted_polygon_outline_inspector_with_points_and_connections(fixed, points, connections).await
}

async fn mounted_polygon_outline_inspector_with_points(
    fixed: bool,
    points: Vec<Vec2>,
) -> (InspectorProbe, web_sys::Element) {
    mounted_polygon_outline_inspector_with_points_and_connections(fixed, points, vec![]).await
}

async fn mounted_polygon_outline_inspector_with_points_and_connections(
    fixed: bool,
    points: Vec<Vec2>,
    connections: Vec<OutlineConnection>,
) -> (InspectorProbe, web_sys::Element) {
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
                        operation: boardstudio_core::model::Operation::Add,
                    }],
                    settings: boardstudio_core::model::OutlineSettings::default(),
                    expected_regions: 1,
                    bridges: vec![],
                    protected_gaps: vec![],
                },
                ownership: boardstudio_core::model::OutlineOwnership::Fixed,
            }],
            generated_last_valid: None,
            retired_version_ids: vec![],
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
    let runtime = crate::runtime::project_name_test_support::new_runtime();
    crate::runtime::project_name_test_support::open_document(&runtime, document).await;
    let scope = runtime
        .scope()
        .expect("the opened outline fixture has a scope");
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
    let (probe, root) =
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
    let (probe, root) = mounted_polygon_outline_inspector(true).await;
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
    let (_probe, root) = mounted_outline_inspector_with_version(Some(OutlineVersion {
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
        ownership: boardstudio_core::model::OutlineOwnership::Fixed,
    }))
    .await;
    wait_until("the Active outline select mounts", || {
        root.query_selector("select[aria-label='Active outline']")
            .unwrap()
            .is_some()
    })
    .await;
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
    root.remove();
}

#[wasm_bindgen_test]
async fn mounted_outline_inspector_generates_through_the_production_owner() {
    let (probe, root) = mounted_outline_inspector().await;
    wait_until("the Generate action mounts", || {
        web_sys::window()
            .unwrap()
            .document()
            .unwrap()
            .query_selector("#outline-inspector-mount button.m1-outline-action")
            .unwrap()
            .is_some()
    })
    .await;
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
                            boardstudio_core::model::OutlineFeature::PartEnvelope { .. }
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
    button.click();
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
    let (probe, root) = mounted_polygon_outline_inspector(false).await;
    let document = web_sys::window().unwrap().document().unwrap();
    wait_until(
        "the generated perimeter exposes its point editor action",
        || {
            document
                .query_selector("#outline-points-inspector-mount button.m1-outline-action")
                .unwrap()
                .is_some()
        },
    )
    .await;
    let open = document
        .query_selector("#outline-points-inspector-mount button.m1-outline-action")
        .unwrap()
        .unwrap()
        .dyn_into::<web_sys::HtmlElement>()
        .unwrap();
    open.click();
    wait_until("Insert point renders in the point editor", || {
        document
            .query_selector("#outline-points-inspector-mount button.m1-outline-insert-point")
            .unwrap()
            .is_some()
    })
    .await;
    let insert = document
        .query_selector("#outline-points-inspector-mount button.m1-outline-insert-point")
        .unwrap()
        .unwrap()
        .dyn_into::<web_sys::HtmlElement>()
        .unwrap();
    insert.click();
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
    let (probe, root) = mounted_polygon_outline_inspector(true).await;
    let document = web_sys::window().unwrap().document().unwrap();
    wait_until(
        "the fixed perimeter exposes its point editor action",
        || {
            document
                .query_selector("#outline-points-inspector-mount button.m1-outline-action")
                .unwrap()
                .is_some()
        },
    )
    .await;
    let open = document
        .query_selector("#outline-points-inspector-mount button.m1-outline-action")
        .unwrap()
        .unwrap()
        .dyn_into::<web_sys::HtmlElement>()
        .unwrap();
    open.click();
    wait_until("the point editor renders Point 1 X", || {
        document
            .query_selector("#outline-points-inspector-mount input[aria-label='Point 1 X mm']")
            .unwrap()
            .is_some()
    })
    .await;
    let input = document
        .query_selector("#outline-points-inspector-mount input[aria-label='Point 1 X mm']")
        .unwrap()
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

/// Ownership presentation: the Inspector names Core's persisted ownership, shows a
/// linked refinement's located finding and affected exports, and never presents a
/// legacy fixed version as linked (ADR-0002/0003).
mod ownership_presentation {
    use super::*;
    use boardstudio_core::model::{OutlineOwnership, OutlineSettings};

    const BOARD: &str = "ownership-board";

    fn polygon(id: &str, x: f64) -> OutlineFeature {
        OutlineFeature::Polygon {
            anchor_part_id: None,
            id: id.into(),
            points: vec![
                Vec2 { x, y: 0.0 },
                Vec2 {
                    x: x + 20.0,
                    y: 0.0,
                },
                Vec2 {
                    x: x + 20.0,
                    y: 20.0,
                },
                Vec2 { x, y: 20.0 },
            ],
            operation: boardstudio_core::model::Operation::Add,
        }
    }

    fn version(
        id: &str,
        name: &str,
        ownership: OutlineOwnership,
        features: Vec<OutlineFeature>,
    ) -> OutlineVersion {
        OutlineVersion {
            id: id.into(),
            name: name.into(),
            source: OutlineProvenance {
                revision: 0,
                version_id: None,
            },
            geometry: OutlineSnapshot {
                features,
                settings: OutlineSettings::default(),
                expected_regions: 1,
                bridges: vec![],
                protected_gaps: vec![],
            },
            ownership,
        }
    }

    /// A linked refinement that resolves from its own authored contour, plus a legacy
    /// fixed version (written before ownership existed, so its serde default is Fixed).
    async fn mounted(
        versions: Vec<OutlineVersion>,
        active: &str,
    ) -> (InspectorProbe, web_sys::Element) {
        let mut document = ProjectDoc::empty("ownership-doc", "Ownership fixture");
        document.revision = 5;
        document.boards.push(Board {
            id: BOARD.into(),
            name: "Board".into(),
            outline_ids: vec![],
            part_ids: vec![],
            net_ids: vec![],
            thickness: 1.6,
            traces: vec![],
            vias: vec![],
        });
        document.board_outlines.push(BoardOutline {
            board_id: BOARD.into(),
            active_version_id: Some(active.into()),
            versions,
            generated_last_valid: None,
            retired_version_ids: vec![],
        });
        let runtime = crate::runtime::project_name_test_support::new_runtime();
        crate::runtime::project_name_test_support::open_document(&runtime, document).await;
        let scope = runtime.scope().expect("the fixture has a scope");
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

    fn option_labels(root: &web_sys::Element) -> Vec<String> {
        let nodes = root
            .query_selector_all("select[aria-label='Active outline'] option")
            .unwrap();
        (0..nodes.length())
            .filter_map(|index| nodes.get(index))
            .map(|node| node.text_content().unwrap_or_default())
            .collect()
    }

    fn input(root: &web_sys::Element, selector: &str) -> web_sys::HtmlInputElement {
        root.query_selector(selector)
            .unwrap()
            .unwrap_or_else(|| panic!("{selector} is mounted"))
            .dyn_into::<web_sys::HtmlInputElement>()
            .unwrap()
    }

    /// The DOM `disabled` property, so a `<select>` and an `<input>` are read the same way.
    fn is_disabled(element: &web_sys::Element) -> bool {
        js_sys::Reflect::get(element, &"disabled".into())
            .ok()
            .and_then(|value| value.as_bool())
            .unwrap_or(false)
    }

    fn element(root: &web_sys::Element, selector: &str) -> web_sys::Element {
        root.query_selector(selector)
            .unwrap()
            .unwrap_or_else(|| panic!("{selector} is mounted"))
    }

    fn button(root: &web_sys::Element, selector: &str) -> web_sys::HtmlElement {
        root.query_selector(selector)
            .unwrap()
            .unwrap_or_else(|| panic!("{selector} is mounted"))
            .dyn_into::<web_sys::HtmlElement>()
            .unwrap()
    }

    #[wasm_bindgen_test]
    async fn mounted_linked_and_legacy_fixed_versions_are_distinct_ownership_choices() {
        let (_probe, root) = mounted(
            vec![
                version(
                    "linked-v1",
                    "Linked 1",
                    OutlineOwnership::Linked,
                    vec![polygon("linked-contour", 0.0)],
                ),
                version(
                    "legacy-fixed",
                    "Edited outline 1",
                    OutlineOwnership::Fixed,
                    vec![polygon("legacy-contour", 60.0)],
                ),
            ],
            "linked-v1",
        )
        .await;
        settle_dimension().await;

        let labels = option_labels(&root);
        assert!(
            labels.iter().any(|label| label.contains("Generated")),
            "the permanent Generated outline stays a distinct choice: {labels:?}"
        );
        assert!(
            labels
                .iter()
                .any(|label| label == "Linked refinement \u{b7} Linked 1"),
            "a persisted linked refinement is named as linked: {labels:?}"
        );
        assert!(
            labels
                .iter()
                .any(|label| label == "Edited outline \u{b7} Edited outline 1"),
            "a legacy fixed version is presented as an edited outline, never as linked: {labels:?}"
        );
        // Settings stay editable while a linked refinement is selected.
        assert!(
            !is_disabled(&element(&root, "select[aria-label='Outline corners']")),
            "the linked refinement's settings remain editable"
        );
        assert!(
            !input(&root, "input[aria-label='Outline version name']").disabled(),
            "the linked refinement's own choices stay editable"
        );
        root.remove();
    }

    #[wasm_bindgen_test]
    async fn mounted_linked_refinement_edits_its_own_envelope_settings_and_a_fixed_copy_has_none() {
        let envelope = |id: &str, margin: f64| OutlineFeature::PartEnvelope {
            connections: vec![],
            settings: OutlineSettings::default(),
            id: id.into(),
            part_ids: vec!["missing-part".into()],
            margin,
            operation: boardstudio_core::model::Operation::Add,
        };
        let (_probe, root) = mounted(
            vec![
                version(
                    "linked-v1",
                    "Linked 1",
                    OutlineOwnership::Linked,
                    vec![envelope("linked-envelope", 4.0)],
                ),
                version(
                    "fixed-v1",
                    "Edited outline 1",
                    OutlineOwnership::Fixed,
                    vec![polygon("fixed-contour", 60.0)],
                ),
            ],
            "linked-v1",
        )
        .await;
        settle_dimension().await;

        // The linked refinement shows and edits its own margin, bridge width and cleanup,
        // so a source change never forces a Freeze just to adjust them.
        let margin = input(&root, "input[aria-label='Outline margin']");
        assert_eq!(margin.value().parse::<f64>().unwrap(), 4.0);
        assert!(
            !margin.disabled(),
            "the linked refinement's margin is editable"
        );
        for selector in [
            "input[aria-label='Bridge width']",
            "input[aria-label='Automatic gap cleanup']",
            "input[aria-label='Maximum gap span']",
        ] {
            assert!(
                !input(&root, selector).disabled(),
                "{selector} is editable on a linked refinement"
            );
        }
        root.remove();

        // A fixed edited copy has no envelope, so those controls are not offered.
        let (_probe, root) = mounted(
            vec![version(
                "fixed-v1",
                "Edited outline 1",
                OutlineOwnership::Fixed,
                vec![polygon("fixed-contour", 60.0)],
            )],
            "fixed-v1",
        )
        .await;
        settle_dimension().await;
        for selector in [
            "input[aria-label='Outline margin']",
            "input[aria-label='Bridge width']",
            "input[aria-label='Automatic gap cleanup']",
            "input[aria-label='Maximum gap span']",
        ] {
            assert!(
                root.query_selector(selector).unwrap().is_none(),
                "{selector} is not offered on a fixed copy"
            );
        }
        root.remove();
    }

    #[wasm_bindgen_test]
    async fn mounted_freeze_creates_one_fixed_copy_retaining_the_link_and_one_undo_restores_both() {
        let (probe, root) = mounted(
            vec![version(
                "linked-v1",
                "Linked 1",
                OutlineOwnership::Linked,
                vec![polygon("linked-contour", 0.0)],
            )],
            "linked-v1",
        )
        .await;
        settle_dimension().await;
        let revision_before = accepted_revision(&probe);
        let freeze = button(&root, "button.m1-outline-freeze");
        freeze.click();
        accept(&probe).await;

        let accepted = accepted_document(&probe);
        let state = &accepted.board_outlines[0];
        assert_eq!(
            state.versions.len(),
            2,
            "one action creates exactly one copy"
        );
        let linked = state
            .versions
            .iter()
            .find(|version| version.id == "linked-v1")
            .expect("the linked refinement is retained");
        assert_eq!(
            linked.ownership,
            OutlineOwnership::Linked,
            "Freeze keeps the linked refinement linked"
        );
        let frozen = state
            .versions
            .iter()
            .find(|version| version.id != "linked-v1")
            .expect("the frozen copy exists");
        assert_eq!(frozen.ownership, OutlineOwnership::Fixed);
        assert!(
            frozen.name.starts_with("Edited outline "),
            "the copy is named as an edited outline: {}",
            frozen.name
        );
        assert_eq!(
            state.active_version_id.as_deref(),
            Some(frozen.id.as_str()),
            "the frozen copy becomes the selected version"
        );
        assert!(
            accepted.revision > revision_before,
            "Freeze is one accepted revision"
        );
        settle_dimension().await;
        let labels = option_labels(&root);
        assert!(
            labels
                .iter()
                .any(|label| label == "Linked refinement \u{b7} Linked 1"),
            "the linked refinement stays visibly available after the freeze copy: {labels:?}"
        );
        assert!(
            root.query_selector(".m1-outline-problem")
                .unwrap()
                .is_none(),
            "a resolvable linked refinement reports no problem"
        );

        // One Undo restores both the document state and the selection.
        undo(&probe.runtime).await;
        accept(&probe).await;
        let restored = accepted_document(&probe);
        assert_eq!(
            restored.board_outlines[0].versions.len(),
            1,
            "one Undo removes only the freeze copy"
        );
        assert_eq!(
            restored.board_outlines[0].active_version_id.as_deref(),
            Some("linked-v1"),
            "one Undo restores the selected version as well"
        );
        root.remove();
    }

    #[wasm_bindgen_test]
    async fn mounted_linked_refinement_that_cannot_resolve_shows_its_located_finding() {
        // The selected refinement owns a generated envelope whose included components are
        // gone, so it cannot resolve and Core keeps its last valid shape.
        let (_probe, root) = mounted(
            vec![version(
                "linked-v1",
                "Linked 1",
                OutlineOwnership::Linked,
                vec![OutlineFeature::PartEnvelope {
                    connections: vec![],
                    settings: OutlineSettings::default(),
                    id: "linked-envelope".into(),
                    part_ids: vec!["missing-part".into()],
                    margin: 4.0,
                    operation: boardstudio_core::model::Operation::Add,
                }],
            )],
            "linked-v1",
        )
        .await;
        settle_dimension().await;

        let finding_id = format!("board:{BOARD}:outline:linked:linked-v1");
        let problem = root
            .query_selector(&format!(".m1-outline-problem[data-finding-id='{finding_id}']"))
            .unwrap()
            .unwrap_or_else(|| {
                panic!(
                    "the located finding is presented by its id contract, not by message text; body: {}",
                    root.inner_html()
                )
            });
        assert_eq!(
            problem.get_attribute("role").as_deref(),
            Some("alert"),
            "the located finding is announced"
        );
        let text = problem.text_content().unwrap_or_default();
        assert!(
            text.contains("cannot resolve"),
            "Core's message is shown as-is: {text}"
        );
        assert!(
            text.contains("PCB"),
            "the affected export is named (only the affected exports are disabled): {text}"
        );
        assert!(
            text.contains("Saving and further editing stay available"),
            "saving and editing stay available: {text}"
        );
        // The linked choice stays selectable and its settings stay editable.
        let labels = option_labels(&root);
        assert!(
            labels
                .iter()
                .any(|label| label == "Linked refinement \u{b7} Linked 1"),
            "a broken refinement remains an available choice: {labels:?}"
        );
        assert!(
            !input(&root, "input[aria-label='Outline version name']").disabled(),
            "editing stays possible while the refinement cannot resolve"
        );
        root.remove();
    }
}

/// Product-context gestures on a generated outline: the panel's own controls must land
/// through real Session/Core, never a silent dead click.
mod product_context {
    use super::*;

    fn click_by_text(root: &web_sys::Element, selector: &str, text: &str) -> bool {
        let nodes = root.query_selector_all(selector).unwrap();
        for index in 0..nodes.length() {
            let Some(node) = nodes.get(index) else {
                continue;
            };
            let Some(element) = node.dyn_ref::<web_sys::HtmlElement>() else {
                continue;
            };
            if node.text_content().as_deref().map(str::trim) == Some(text) {
                element.click();
                return true;
            }
        }
        false
    }

    fn versions(runtime: &Rc<crate::runtime::Runtime>, board_id: &str) -> usize {
        runtime
            .model()
            .accepted
            .expect("accepted")
            .document
            .board_outlines
            .iter()
            .find(|state| state.board_id == board_id)
            .map_or(0, |state| state.versions.len())
    }

    fn alert_text(root: &web_sys::Element) -> Option<String> {
        root.query_selector("[role='alert']")
            .ok()
            .flatten()
            .and_then(|node| node.text_content())
    }

    fn ownership_labels(root: &web_sys::Element) -> Vec<String> {
        let nodes = root
            .query_selector_all("select[aria-label='Active outline'] option")
            .unwrap();
        (0..nodes.length())
            .filter_map(|index| nodes.get(index))
            .map(|node| node.text_content().unwrap_or_default())
            .collect()
    }

    #[wasm_bindgen_test]
    async fn mounted_refine_outline_creates_a_linked_refinement_with_one_undo() {
        let (probe, root) = mounted_polygon_outline_inspector(false).await;
        settle_dimension().await;
        assert!(
            click_by_text(
                &root,
                "#outline-points-inspector-mount .m1-outline-actions button",
                "Refine outline"
            ),
            "the Refine outline control is reachable in the generated context; body: {}",
            root.inner_html()
        );
        accept(&probe).await;
        settle_dimension().await;
        let accepted = accepted_document(&probe);
        let state = accepted
            .board_outlines
            .iter()
            .find(|state| state.board_id == probe.scope.board_id)
            .expect("the refinement creates board outline state");
        assert_eq!(state.versions.len(), 1, "one action creates one refinement");
        let created = &state.versions[0];
        assert_eq!(
            created.ownership,
            boardstudio_core::model::OutlineOwnership::Linked,
            "accepting the alternative creates a linked refinement"
        );
        assert_eq!(
            state.active_version_id.as_deref(),
            Some(created.id.as_str()),
            "the created refinement is selected"
        );
        let labels = ownership_labels(&root);
        assert!(
            labels
                .iter()
                .any(|label| label.starts_with("Linked refinement")),
            "the refinement is presented as linked: {labels:?}"
        );
        let name_input = root
            .query_selector("input[aria-label='Outline version name']")
            .unwrap()
            .expect("the version name control is mounted for the selected refinement")
            .dyn_into::<web_sys::HtmlInputElement>()
            .unwrap();
        assert!(
            !name_input.disabled(),
            "its settings stay editable while linked"
        );

        // One Undo removes the refinement and restores the Generated selection.
        undo(&probe.runtime).await;
        accept(&probe).await;
        settle_dimension().await;
        let undone = accepted_document(&probe);
        let state = undone
            .board_outlines
            .iter()
            .find(|state| state.board_id == probe.scope.board_id);
        assert!(
            state.is_none_or(|state| state.versions.is_empty()),
            "one Undo removes the refinement"
        );
        assert!(
            undone
                .board_outlines
                .iter()
                .all(|state| state.active_version_id.is_none()),
            "one Undo restores the Generated selection"
        );
        root.remove();
    }

    #[wasm_bindgen_test]
    async fn mounted_copy_outline_on_a_generated_outline_creates_one_version() {
        let (probe, root) = mounted_polygon_outline_inspector(false).await;
        settle_dimension().await;
        assert!(
            click_by_text(
                &root,
                "#outline-points-inspector-mount .m1-outline-actions button",
                "Copy outline"
            ),
            "the Copy outline control is reachable"
        );
        accept(&probe).await;
        settle_dimension().await;
        let count = versions(&probe.runtime, &probe.scope.board_id);
        assert!(
            count == 1,
            "Copy outline lands one version (got {count}); alert={:?}",
            alert_text(&root)
        );
        root.remove();
    }
}
