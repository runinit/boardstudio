use super::*;
use boardstudio_application::{
    AcceptedSnapshot, Durability, Lifecycle, ReadModel, Scope, SessionEpoch, SnapshotToken,
};
use boardstudio_core::model::{
    Board, BoardContours, BoardOutline, Contour, OutlineFeature, OutlineProvenance,
    OutlineSnapshot, OutlineVersion, ProjectDoc, Readiness, SceneDelta,
};
use std::sync::Arc;
use std::{cell::RefCell, rc::Rc};
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
    let (projection, _) =
        use_outline_lifecycle(probe.runtime.clone(), selected, workspace, generation);
    let _ = version();
    rsx! {
        style { {include_str!("../../assets/m1.css")} }
        if let Some(projection) = projection {
            OutlineVersionInspector { projection }
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
    let probe = InspectorProbe { runtime, scope };
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

fn mounted_polygon_outline_inspector(fixed: bool) -> (InspectorProbe, web_sys::Element) {
    let scope = Scope {
        session_epoch: SessionEpoch(6),
        document_id: "outline-points-doc".into(),
        board_id: "outline-points-board".into(),
        instance_id: None,
    };
    let points = vec![
        boardstudio_core::model::Vec2 { x: 0.0, y: 0.0 },
        boardstudio_core::model::Vec2 { x: 20.0, y: 0.0 },
        boardstudio_core::model::Vec2 { x: 20.0, y: 20.0 },
        boardstudio_core::model::Vec2 { x: 0.0, y: 20.0 },
    ];
    let mut document = ProjectDoc::empty("outline-points-doc", "Outline points fixture");
    document.revision = 11;
    document.boards.push(Board {
        id: scope.board_id.clone(),
        name: "Board".into(),
        outline_ids: if fixed {
            vec![]
        } else {
            vec!["generated-outline".into()]
        },
        part_ids: vec![],
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
            connections: vec![],
            settings: boardstudio_core::model::OutlineSettings::default(),
            id: "generated-outline".into(),
            part_ids: vec![],
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
    let probe = InspectorProbe { runtime, scope };
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
    let perimeter = editable_perimeter(&snapshot, &probe.scope.board_id, Some("fixed-outline-v1"))
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
        editable_perimeter(&snapshot, &probe.scope.board_id, Some("fixed-outline-v1")).is_none()
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
