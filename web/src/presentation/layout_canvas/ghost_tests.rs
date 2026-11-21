use super::*;
use boardstudio_core::model::SceneDelta;
use serde_json::json;
use std::sync::Arc;
use wasm_bindgen::JsCast;
use wasm_bindgen_test::wasm_bindgen_test;

fn ghost_fixture() -> (Arc<ProjectDoc>, Arc<SceneDelta>, Arc<SceneDelta>) {
    let mut document = ProjectDoc::empty("canvas", "Canvas");
    document.parts.push(
        serde_json::from_value(json!({
            "id": "matrix/m/r0c0", "definitionId": "switch", "reference": "SW1",
            "pose": {"at": {"x": 0.0, "y": 0.0}, "rotation": 0.0}, "side": "front",
            "keycap": {"x": 18.0, "y": 18.0}
        }))
        .unwrap(),
    );
    document.definitions.push(
        serde_json::from_value(json!({
            "id": "switch", "name": "Switch", "kind": "switch", "courtyard": [], "pads": []
        }))
        .unwrap(),
    );
    document.boards.push(
        serde_json::from_value(json!({
            "id": "board", "name": "Board", "partIds": ["matrix/m/r0c0"],
            "outlineIds": [], "netIds": [], "thickness": 1.6
        }))
        .unwrap(),
    );
    document.matrices.push(
        serde_json::from_value(json!({
            "id": "m", "rows": 1, "columns": 1,
            "pitch": {"x": 19.05, "y": 19.05}, "origin": {"x": 0.0, "y": 0.0},
            "definitionId": "switch", "partIds": ["matrix/m/r0c0"], "boardId": "board",
            "cells": [{"row": 0, "column": 0, "enabled": true, "definitionId": "switch", "assemblies": []}]
        }))
        .unwrap(),
    );
    let document = Arc::new(document);
    let cell = |x: f64, y: f64| {
        json!({
            "row": 0, "column": 0, "enabled": true, "memberId": "matrix/m/r0c0",
            "pose": {"at": {"x": x, "y": y}, "rotation": 0.0},
            "shiftFrame": {"origin": {"x": 0.0, "y": 0.0}, "rotation": 0.0, "axisX": {"x": 1.0, "y": 0.0}, "axisY": {"x": 0.0, "y": 1.0}},
            "offsetFrame": {"origin": {"x": 0.0, "y": 0.0}, "rotation": 0.0, "axisX": {"x": 1.0, "y": 0.0}, "axisY": {"x": 0.0, "y": 1.0}}
        })
    };
    let scene_with = |x: f64, y: f64| -> Arc<SceneDelta> {
        Arc::new(
            serde_json::from_value(json!({
                "revision": 0, "transactionId": "fixture", "changedIds": [], "transforms": [],
                "matrixScenes": [{
                    "matrixId": "m", "cells": [cell(x, y)],
                    "columns": [{
                        "column": 0, "base": {"x": 0.0, "y": 0.0}, "rotation": 0.0,
                        "relativeOrigin": {"x": 0.0, "y": 0.0},
                        "splayOrigin": {"x": 0.0, "y": 0.0}, "splayAngle": 0.0, "customOrigin": false,
                        "axisX": {"x": 1.0, "y": 0.0}, "axisY": {"x": 0.0, "y": 1.0},
                        "staggerAxis": {"x": 0.0, "y": 1.0}
                    }]
                }],
                "contours": [], "boardContours": [], "boardReadiness": [], "findings": [],
                "readiness": {"layout": true, "outline": false, "pcb": false, "case": false}
            }))
            .unwrap(),
        )
    };
    (document, scene_with(0.0, 0.0), scene_with(9.0, 4.0))
}

fn render_ghost_fixture() -> Element {
    let highlight = use_signal(|| None::<Vec<String>>);
    use_context_provider(|| super::super::RelationHighlight(highlight));
    let (document, initial_scene, preview_scene) = use_hook(ghost_fixture);
    let accepted_document = use_signal(|| document.clone());
    let mut displayed = use_signal(|| initial_scene.clone());
    rsx! {
        button { id: "canvas-ghost-preview", onclick: move |_| {
            displayed.set(preview_scene.clone());
        }, "Preview" }
        button { id: "canvas-ghost-off", onclick: move |_| {
            displayed.set(initial_scene.clone());
        }, "Clear preview" }
        svg {
            Scene {
                source: SceneSource { document: accepted_document(), scene: displayed(), board_id: "board".into() },
                preview_active: false,
                accepted_scene: Some(initial_scene.clone()),
                selected_ids: Vec::new(), selected_context: None,
                view_width_world: 200.0, surface_width_px: 800.0,
                keys_visible: true, components_visible: true, keycaps_visible: true, footprints_visible: false,
                on_matrix_pointer_down: |_| {}, on_part_pointer_down: |_| {},
            }
        }
    }
}

/// Ticket 23 slice 10, RED: a transform preview ghosts the pre-drag pose
/// of every moved matrix key until the preview clears.
#[wasm_bindgen_test]
async fn transform_preview_ghosts_the_pre_drag_pose() {
    let document = web_sys::window().unwrap().document().unwrap();
    let root = document.create_element("div").unwrap();
    document.body().unwrap().append_child(&root).unwrap();
    dioxus_web::launch::launch_virtual_dom(
        VirtualDom::new(render_ghost_fixture),
        dioxus_web::Config::new().rootnode(root.clone().into()),
    );
    gloo_timers::future::TimeoutFuture::new(5).await;
    let click = |selector: &str| {
        root.query_selector(selector)
            .unwrap()
            .unwrap()
            .dyn_into::<web_sys::HtmlElement>()
            .unwrap()
            .click()
    };
    assert!(
        root.query_selector("[data-ghost='scene']")
            .unwrap()
            .is_none(),
        "the settled scene carries no ghosts"
    );
    click("#canvas-ghost-preview");
    gloo_timers::future::TimeoutFuture::new(5).await;
    let ghost = root
        .query_selector("[data-ghost='scene']")
        .unwrap()
        .expect("the preview ghosts the pre-drag pose");
    assert!(
        ghost
            .get_attribute("transform")
            .as_deref()
            .unwrap_or_default()
            .contains("translate(0 0)"),
        "the ghost sits on the pre-drag pose"
    );
    click("#canvas-ghost-off");
    gloo_timers::future::TimeoutFuture::new(5).await;
    assert!(
        root.query_selector("[data-ghost='scene']")
            .unwrap()
            .is_none(),
        "clearing the preview retires its ghosts"
    );
    root.remove();
}
