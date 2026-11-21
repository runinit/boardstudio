use super::*;
use boardstudio_core::model::SceneDelta;
use serde_json::json;
use std::sync::Arc;
use wasm_bindgen::JsCast;
use wasm_bindgen_test::wasm_bindgen_test;

fn spacing_fixture() -> (Arc<ProjectDoc>, Arc<SceneDelta>, Arc<SceneDelta>) {
    let mut document = ProjectDoc::empty("canvas", "Canvas");
    for (half, part_id) in [("left", "matrix/left/r0c0"), ("right", "matrix/right/r0c0")] {
        document.parts.push(
            serde_json::from_value(json!({
                "id": part_id, "definitionId": "switch", "reference": "SW1",
                "pose": {"at": {"x": 0.0, "y": 0.0}, "rotation": 0.0}, "side": "front",
                "keycap": {"x": 18.0, "y": 18.0}
            }))
            .unwrap(),
        );
        document.matrices.push(
            serde_json::from_value(json!({
                "id": half, "rows": 1, "columns": 1,
                "pitch": {"x": 19.05, "y": 19.05}, "origin": {"x": 0.0, "y": 0.0},
                "definitionId": "switch", "partIds": [part_id], "boardId": "board",
                "cells": [{"row": 0, "column": 0, "enabled": true, "definitionId": "switch", "assemblies": []}]
            }))
            .unwrap(),
        );
    }
    document.definitions.push(
        serde_json::from_value(json!({
            "id": "switch", "name": "Switch", "kind": "switch", "courtyard": [], "pads": []
        }))
        .unwrap(),
    );
    document.boards.push(
        serde_json::from_value(json!({
            "id": "board", "name": "Board",
            "partIds": ["matrix/left/r0c0", "matrix/right/r0c0"],
            "outlineIds": [], "netIds": [], "thickness": 1.6
        }))
        .unwrap(),
    );
    let document = Arc::new(document);
    let cell = |matrix: &str, x: f64| {
        json!({
            "row": 0, "column": 0, "enabled": true,
            "memberId": format!("matrix/{matrix}/r0c0"),
            "pose": {"at": {"x": x, "y": 0.0}, "rotation": 0.0},
            "shiftFrame": {"origin": {"x": 0.0, "y": 0.0}, "rotation": 0.0, "axisX": {"x": 1.0, "y": 0.0}, "axisY": {"x": 0.0, "y": 1.0}},
            "offsetFrame": {"origin": {"x": 0.0, "y": 0.0}, "rotation": 0.0, "axisX": {"x": 1.0, "y": 0.0}, "axisY": {"x": 0.0, "y": 1.0}}
        })
    };
    let scene_with = |half_gap: f64| -> Arc<SceneDelta> {
        Arc::new(
            serde_json::from_value(json!({
                "revision": 0, "transactionId": "fixture", "changedIds": [], "transforms": [],
                "matrixScenes": [
                    {"matrixId": "left", "cells": [cell("left", -half_gap)],
                     "columns": [{"column": 0, "base": {"x": 0.0, "y": 0.0}, "rotation": 0.0,
                                  "relativeOrigin": {"x": 0.0, "y": 0.0},
                                  "splayOrigin": {"x": 0.0, "y": 0.0}, "splayAngle": 0.0, "customOrigin": false,
                                  "axisX": {"x": 1.0, "y": 0.0}, "axisY": {"x": 0.0, "y": 1.0},
                                  "staggerAxis": {"x": 0.0, "y": 1.0}}]},
                    {"matrixId": "right", "cells": [cell("right", half_gap)],
                     "columns": [{"column": 0, "base": {"x": 0.0, "y": 0.0}, "rotation": 0.0,
                                  "relativeOrigin": {"x": 0.0, "y": 0.0},
                                  "splayOrigin": {"x": 0.0, "y": 0.0}, "splayAngle": 0.0, "customOrigin": false,
                                  "axisX": {"x": 1.0, "y": 0.0}, "axisY": {"x": 0.0, "y": 1.0},
                                  "staggerAxis": {"x": 0.0, "y": 1.0}}]}
                ],
                "contours": [], "boardContours": [], "boardReadiness": [], "findings": [],
                "readiness": {"layout": true, "outline": false, "pcb": false, "case": false}
            }))
            .unwrap(),
        )
    };
    // Narrow spacing (keys at ±30) and the widened spacing (keys at ±35).
    (document, scene_with(30.0), scene_with(35.0))
}

#[derive(Props, Clone, PartialEq)]
struct SpacingCaseProps {
    document: Arc<ProjectDoc>,
    displayed: Arc<SceneDelta>,
    accepted: Option<Arc<SceneDelta>>,
    ghost: Option<SpacingGhost>,
    preview: bool,
}

#[component]
fn RenderSpacingCase(props: SpacingCaseProps) -> Element {
    let highlight = use_signal(|| None::<Vec<String>>);
    use_context_provider(|| super::super::RelationHighlight(highlight));
    rsx! {
        svg {
            Scene {
                source: SceneSource { document: props.document, scene: props.displayed, board_id: "board".into() },
                preview_active: props.preview,
                accepted_scene: props.accepted,
                spacing_ghost: props.ghost,
                selected_ids: Vec::new(), selected_context: None,
                view_width_world: 200.0, surface_width_px: 800.0,
                keys_visible: true, components_visible: true, keycaps_visible: true, footprints_visible: false,
                on_matrix_pointer_down: |_| {}, on_part_pointer_down: |_| {},
            }
        }
    }
}

async fn mount_case(
    displayed: Arc<SceneDelta>,
    accepted: Option<Arc<SceneDelta>>,
    ghost: Option<SpacingGhost>,
    preview: bool,
) -> web_sys::Element {
    let (document, _, _) = spacing_fixture();
    let root = web_sys::window()
        .unwrap()
        .document()
        .unwrap()
        .create_element("div")
        .unwrap();
    web_sys::window()
        .unwrap()
        .document()
        .unwrap()
        .body()
        .unwrap()
        .append_child(&root)
        .unwrap();
    dioxus_web::launch::launch_virtual_dom(
        VirtualDom::new_with_props(
            RenderSpacingCase,
            SpacingCaseProps {
                document,
                displayed,
                accepted,
                ghost,
                preview,
            },
        ),
        dioxus_web::Config::new().rootnode(root.clone().into()),
    );
    gloo_timers::future::TimeoutFuture::new(5).await;
    root
}

fn ghost_transforms(root: &web_sys::Element) -> Vec<String> {
    let nodes = root.query_selector_all("[data-ghost='spacing']").unwrap();
    (0..nodes.length())
        .filter_map(|index| {
            nodes
                .item(index)?
                .dyn_into::<web_sys::SvgElement>()
                .ok()?
                .get_attribute("transform")
        })
        .collect()
}

/// RED: widening draws the previous (narrow) keys dashed with a +N note.
#[wasm_bindgen_test]
async fn widening_ghosts_the_narrow_keys_with_a_plus_note() {
    let (_, narrow, wide) = spacing_fixture();
    let root = mount_case(
        wide,
        None,
        Some(SpacingGhost {
            scene: narrow,
            delta_mm: 10.0,
            axis_x: 0.0,
        }),
        false,
    )
    .await;
    let transforms = ghost_transforms(&root);
    assert_eq!(
        transforms.len(),
        2,
        "both halves ghost at the previous spacing"
    );
    assert!(
        transforms
            .iter()
            .any(|transform| transform.contains("translate(-30 0)")),
        "the left ghost sits on its narrow pose, got {transforms:?}"
    );
    assert!(
        transforms
            .iter()
            .any(|transform| transform.contains("translate(30 0)")),
        "the right ghost sits on its narrow pose, got {transforms:?}"
    );
    let note = root
        .query_selector("[data-ghost-note='spacing']")
        .unwrap()
        .expect("the axis carries the widening delta");
    assert_eq!(note.text_content().as_deref(), Some("+10.0 mm"));
    // Ticket 02: the note sits a fixed 14 px screen gap above the top
    // key edge: 9.025 half key plus 3.5 world units at this zoom.
    assert!(
        note.get_attribute("transform")
            .as_deref()
            .unwrap_or_default()
            .contains("translate(0 12.5"),
        "the note sits on the axis above the keys"
    );
    root.remove();
}

/// RED: narrowing draws the previous (wide) keys dashed with a −N note.
#[wasm_bindgen_test]
async fn narrowing_ghosts_the_wide_keys_with_a_minus_note() {
    let (_, narrow, wide) = spacing_fixture();
    let root = mount_case(
        narrow,
        None,
        Some(SpacingGhost {
            scene: wide,
            delta_mm: -6.0,
            axis_x: 0.0,
        }),
        false,
    )
    .await;
    let transforms = ghost_transforms(&root);
    assert_eq!(transforms.len(), 2, "both halves ghost at the wide poses");
    assert!(
        transforms
            .iter()
            .any(|transform| transform.contains("translate(-35 0)")),
        "the left ghost sits on its wide pose, got {transforms:?}"
    );
    let note = root
        .query_selector("[data-ghost-note='spacing']")
        .unwrap()
        .expect("the axis carries the narrowing delta");
    assert_eq!(note.text_content().as_deref(), Some("−6.0 mm"));
    root.remove();
}

/// RED: a settled scene with no ghost prop draws no ghost elements.
#[wasm_bindgen_test]
async fn settled_scene_without_ghost_draws_nothing() {
    let (_, _, wide) = spacing_fixture();
    let root = mount_case(wide, None, None, false).await;
    assert!(
        root.query_selector_all("[data-ghost='spacing']")
            .unwrap()
            .length()
            == 0,
        "no ghost prop means no ghost rects"
    );
    assert!(
        root.query_selector("[data-ghost-note='spacing']")
            .unwrap()
            .is_none(),
        "no ghost prop means no axis note"
    );
    root.remove();
}

/// RED: a drag preview keeps the amber preview ghosts and hides the
/// spacing ghost and its note.
#[wasm_bindgen_test]
async fn drag_preview_preempts_the_spacing_ghost() {
    let (_, narrow, wide) = spacing_fixture();
    let root = mount_case(
        wide,
        Some(narrow.clone()),
        Some(SpacingGhost {
            scene: narrow,
            delta_mm: 10.0,
            axis_x: 0.0,
        }),
        true,
    )
    .await;
    assert!(
        root.query_selector("[data-ghost='scene']")
            .unwrap()
            .is_some(),
        "the preview still ghosts the pre-drag pose"
    );
    assert!(
        root.query_selector_all("[data-ghost='spacing']")
            .unwrap()
            .length()
            == 0,
        "the spacing ghost hides while a preview is up"
    );
    assert!(
        root.query_selector("[data-ghost-note='spacing']")
            .unwrap()
            .is_none(),
        "the axis note hides while a preview is up"
    );
    root.remove();
}
