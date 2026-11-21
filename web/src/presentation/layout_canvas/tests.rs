use super::*;
use boardstudio_core::model::{MatrixSceneCell, Transform};
use boardstudio_core::model::{ProjectDoc, SceneDelta};
use serde_json::json;
use std::sync::Arc;
use wasm_bindgen::JsCast;
use wasm_bindgen_test::wasm_bindgen_test;

#[wasm_bindgen_test]
fn explicit_offset_and_origin_activate_the_named_canvas_handle() {
    use super::super::objects::{LayoutTransformTool as Tool, TreeContext};
    let column = TreeContext::Column {
        matrix_id: "matrix".into(),
        column: 0,
    };
    assert_eq!(
        transform_tools(&column, Some(Tool::Offset), true, None),
        vec![Tool::Offset, Tool::Splay]
    );
    assert_eq!(
        transform_tools(&column, Some(Tool::Origin), true, None),
        vec![Tool::Stagger, Tool::Origin]
    );
    assert_eq!(
        transform_tools(&column, None, true, None),
        vec![Tool::Stagger, Tool::Splay]
    );
    // Ticket 05: a Row keeps its shipped Offset next to Shift and Bend;
    // a Key offers Shift and Bend.
    assert_eq!(
        transform_tools(
            &TreeContext::Row {
                matrix_id: "matrix".into(),
                row: 0
            },
            Some(Tool::Stagger),
            true,
            None,
        ),
        vec![Tool::Offset, Tool::Shift, Tool::Bend]
    );
    assert_eq!(
        transform_tools(
            &TreeContext::Row {
                matrix_id: "matrix".into(),
                row: 0
            },
            None,
            true,
            None,
        ),
        vec![Tool::Offset, Tool::Shift, Tool::Bend]
    );
    assert_eq!(
        transform_tools(
            &TreeContext::Key {
                matrix_id: "matrix".into(),
                row: 0,
                column: 1,
            },
            None,
            true,
            None,
        ),
        vec![Tool::Shift, Tool::Bend]
    );
    assert_eq!(
        transform_tools(
            &TreeContext::Matrix {
                matrix_id: "matrix".into()
            },
            None,
            true,
            None,
        ),
        vec![Tool::Move, Tool::Rotate, Tool::Anchor]
    );
    assert_eq!(
        transform_tools(
            &TreeContext::Matrix {
                matrix_id: "matrix".into()
            },
            Some(Tool::Move),
            true,
            None,
        ),
        vec![Tool::Move, Tool::Rotate, Tool::Anchor]
    );
    assert_eq!(
        transform_tools(
            &TreeContext::Matrix {
                matrix_id: "matrix".into()
            },
            Some(Tool::Move),
            false,
            None,
        ),
        vec![Tool::Move]
    );
    // Ticket 07: an independent part gets its own rail from the mount
    // site's facts; without facts (a member or companion) nothing draws.
    use boardstudio_web_layout::part_rail::PartRailFacts;
    let part = TreeContext::Component {
        part_id: Some("mcu".into()),
        matrix_id: None,
        row: None,
        column: None,
        assembly_id: None,
    };
    let anchored = || {
        Some(PartRailFacts {
            anchored: true,
            anchor_column: 1,
            locked: false,
            member_or_companion: false,
        })
    };
    assert_eq!(
        transform_tools(&part, None, true, anchored()),
        vec![
            Tool::Move,
            Tool::Rotate,
            Tool::Spread,
            Tool::Stagger,
            Tool::Anchor,
        ]
    );
    assert_eq!(
        transform_tools(
            &part,
            None,
            true,
            Some(PartRailFacts {
                anchored: true,
                anchor_column: 0,
                locked: false,
                member_or_companion: false,
            }),
        ),
        vec![Tool::Move, Tool::Rotate, Tool::Stagger, Tool::Anchor]
    );
    assert_eq!(
        transform_tools(
            &part,
            None,
            true,
            Some(PartRailFacts {
                anchored: false,
                anchor_column: 0,
                locked: false,
                member_or_companion: false,
            }),
        ),
        vec![Tool::Move, Tool::Rotate, Tool::Anchor]
    );
    assert_eq!(
        transform_tools(
            &part,
            None,
            true,
            Some(PartRailFacts {
                anchored: true,
                anchor_column: 1,
                locked: true,
                member_or_companion: false,
            }),
        ),
        Vec::new()
    );
    assert_eq!(transform_tools(&part, None, true, None), Vec::new());
    // A matrix member or companion names itself in the facts and draws no
    // part rail.
    assert_eq!(
        transform_tools(
            &part,
            None,
            true,
            Some(PartRailFacts {
                anchored: true,
                anchor_column: 1,
                locked: false,
                member_or_companion: true,
            }),
        ),
        Vec::new()
    );
    // A matrix member keeps its matrix's key tools, never the part rail.
    let member_key = TreeContext::Key {
        matrix_id: "matrix".into(),
        row: 0,
        column: 1,
    };
    assert_eq!(
        transform_tools(&member_key, None, true, anchored()),
        vec![Tool::Shift, Tool::Bend]
    );
}

/// Ticket 23: the Handles toggle is view-only. Hidden handles drop every
/// passive overlay but keep the active tool's required handle; with no
/// active tool nothing draws.
#[wasm_bindgen_test]
fn hidden_handles_keep_only_the_active_tool() {
    use super::super::objects::{LayoutTransformTool as Tool, TreeContext};
    let column = TreeContext::Column {
        matrix_id: "matrix".into(),
        column: 1,
    };
    let row = TreeContext::Row {
        matrix_id: "matrix".into(),
        row: 0,
    };
    let key = TreeContext::Key {
        matrix_id: "matrix".into(),
        row: 0,
        column: 1,
    };
    assert_eq!(transform_tools(&column, None, false, None), Vec::new());
    assert_eq!(transform_tools(&row, None, false, None), Vec::new());
    assert_eq!(transform_tools(&key, None, false, None), Vec::new());
    assert_eq!(
        transform_tools(&column, Some(Tool::Stagger), false, None),
        vec![Tool::Stagger]
    );
    assert_eq!(
        transform_tools(&column, Some(Tool::Spread), false, None),
        vec![Tool::Spread]
    );
    assert_eq!(
        transform_tools(&column, Some(Tool::Origin), false, None),
        vec![Tool::Origin]
    );
    assert_eq!(
        transform_tools(&row, Some(Tool::Offset), false, None),
        vec![Tool::Offset]
    );
    assert_eq!(
        transform_tools(&row, Some(Tool::Shift), false, None),
        vec![Tool::Shift]
    );
    assert_eq!(
        transform_tools(&row, Some(Tool::Bend), false, None),
        vec![Tool::Bend]
    );
    // A Row selection never draws a Stagger handle, even hidden.
    assert_eq!(
        transform_tools(&row, Some(Tool::Stagger), false, None),
        Vec::new()
    );
    assert_eq!(
        transform_tools(&key, Some(Tool::Shift), false, None),
        vec![Tool::Shift]
    );
    assert_eq!(
        transform_tools(&key, Some(Tool::Bend), false, None),
        vec![Tool::Bend]
    );
    // A Key selection never draws a Stagger handle, even hidden.
    assert_eq!(
        transform_tools(&key, Some(Tool::Stagger), false, None),
        Vec::new()
    );
    // Ticket 07: a hidden part rail keeps only its active tool.
    use boardstudio_web_layout::part_rail::PartRailFacts;
    let part = TreeContext::Component {
        part_id: Some("mcu".into()),
        matrix_id: None,
        row: None,
        column: None,
        assembly_id: None,
    };
    let anchored = || {
        Some(PartRailFacts {
            anchored: true,
            anchor_column: 1,
            locked: false,
            member_or_companion: false,
        })
    };
    let detached = || {
        Some(PartRailFacts {
            anchored: false,
            anchor_column: 0,
            locked: false,
            member_or_companion: false,
        })
    };
    assert_eq!(transform_tools(&part, None, false, anchored()), Vec::new());
    assert_eq!(
        transform_tools(&part, Some(Tool::Spread), false, anchored()),
        vec![Tool::Spread]
    );
    assert_eq!(
        transform_tools(&part, Some(Tool::Move), false, detached()),
        vec![Tool::Move]
    );
    // Spread is not in the detached rail, even hidden.
    assert_eq!(
        transform_tools(&part, Some(Tool::Spread), false, detached()),
        Vec::new()
    );
}

fn fixture() -> (Arc<ProjectDoc>, Arc<SceneDelta>) {
    let mut document = ProjectDoc::empty("canvas", "Canvas");
    document.parts.push(
        serde_json::from_value(json!({
            "id": "key", "definitionId": "switch", "reference": "SW1",
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
    let mut stationary = document.parts[0].clone();
    stationary.id = "stationary".into();
    stationary.pose.at.x = 30.0;
    document.parts.push(stationary);
    document.boards.push(serde_json::from_value(json!({
        "id": "board", "name": "Board", "partIds": ["key", "stationary"], "outlineIds": [], "netIds": [], "thickness": 1.6
    })).unwrap());
    let scene = serde_json::from_value(json!({
        "revision": 0, "transactionId": "fixture", "changedIds": [], "transforms": [],
        "matrixScenes": [], "contours": [], "boardContours": [], "boardReadiness": [], "findings": [],
        "readiness": {"layout": true, "outline": false, "pcb": false, "case": false}
    })).unwrap();
    (Arc::new(document), Arc::new(scene))
}

fn render_fixture() -> Element {
    let mut highlight = use_signal(|| None::<Vec<String>>);
    use_context_provider(|| super::super::RelationHighlight(highlight));
    let (document, initial_scene) = use_hook(fixture);
    let mut accepted_document = use_signal(|| document.clone());
    let mut displayed = use_signal(|| initial_scene.clone());
    let mut preview = use_signal(|| false);
    let mut generation = use_signal(|| 0_usize);
    let mut observed = use_signal(|| 0_usize);
    let mut selected = use_signal(Vec::<String>::new);
    let mut keys = use_signal(|| true);
    let captured_generation = generation();
    rsx! {
        button { id: "canvas-unrelated", onclick: move |_| generation += 1, "Unrelated update" }
        button { id: "canvas-highlight", onclick: move |_| highlight.set(Some(vec!["key".into()])), "Highlight relation" }
        button { id: "canvas-preview", onclick: move |_| {
            let mut next = (*displayed()).clone();
            next.transforms = vec![Transform { id: "key".into(), pose: Pose2 { at: Vec2 { x: 9.0, y: 4.0 }, rotation: 0.0 } }];
            displayed.set(Arc::new(next));
            preview.set(true);
        }, "Preview" }
        button { id: "canvas-preview-off", onclick: move |_| {
            displayed.set(initial_scene.clone());
            preview.set(false);
        }, "Clear preview" }
        button { id: "canvas-select", onclick: move |_| selected.set(vec!["key".into()]), "Select" }
        button { id: "canvas-accepted", onclick: move |_| {
            let mut next = accepted_document();
            Arc::make_mut(&mut next).parts[0].keycap = Some(Vec2 { x: 30.0, y: 18.0 });
            accepted_document.set(next);
        }, "Accepted geometry" }
        button { id: "canvas-hide", onclick: move |_| keys.set(false), "Hide" }
        output { id: "canvas-observed", "{observed()}" }
        svg {
            Scene {
                source: SceneSource { document: accepted_document(), scene: displayed(), board_id: "board".into() },
                preview_active: preview(),
                accepted_scene: None,
                selected_ids: selected(), selected_context: None,
                view_width_world: 200.0, surface_width_px: 800.0,
                keys_visible: keys(), components_visible: true, keycaps_visible: keys(), footprints_visible: false,
                on_matrix_pointer_down: |_| {},
                on_part_pointer_down: move |_| observed.set(captured_generation),
            }
        }
    }
}

#[wasm_bindgen_test]
async fn canvas_skips_unrelated_updates_but_refreshes_callbacks_preview_selection_and_layers() {
    let document = web_sys::window().unwrap().document().unwrap();
    let root = document.create_element("div").unwrap();
    document.body().unwrap().append_child(&root).unwrap();
    dioxus_web::launch::launch_virtual_dom(
        VirtualDom::new(render_fixture),
        dioxus_web::Config::new().rootnode(root.clone().into()),
    );
    gloo_timers::future::TimeoutFuture::new(5).await;
    let renders = SCENE_RENDERS.with(|count| count.get());
    let click = |selector: &str| {
        root.query_selector(selector)
            .unwrap()
            .unwrap()
            .dyn_into::<web_sys::HtmlElement>()
            .unwrap()
            .click()
    };
    click("#canvas-unrelated");
    gloo_timers::future::TimeoutFuture::new(5).await;
    assert_eq!(
        SCENE_RENDERS.with(|count| count.get()),
        renders,
        "unchanged canvas inputs must not rebuild the scene"
    );
    let pointer = js_sys::eval("new PointerEvent('pointerdown', {bubbles: true})")
        .unwrap()
        .dyn_into::<web_sys::Event>()
        .unwrap();
    root.query_selector("[data-geometry='part-hit-area']")
        .unwrap()
        .unwrap()
        .dispatch_event(&pointer)
        .unwrap();
    gloo_timers::future::TimeoutFuture::new(5).await;
    assert_eq!(
        root.query_selector("#canvas-observed")
            .unwrap()
            .unwrap()
            .text_content()
            .as_deref(),
        Some("1"),
        "memoized canvas callbacks must use the latest parent capture"
    );
    click("#canvas-highlight");
    gloo_timers::future::TimeoutFuture::new(5).await;
    assert!(
        root.query_selector("[data-geometry='part-outline'][data-relation-highlight='true']")
            .unwrap()
            .is_some()
    );
    assert_eq!(
        SCENE_RENDERS.with(|count| count.get()),
        renders,
        "relation children remain reactive without rebuilding the canvas"
    );
    let part_renders = PART_CONTENT_RENDERS.with(|count| count.get());
    click("#canvas-preview");
    gloo_timers::future::TimeoutFuture::new(5).await;
    assert_eq!(
        PART_CONTENT_RENDERS.with(|count| count.get()),
        part_renders,
        "pose-only previews must not rebuild unchanged part graphics"
    );
    assert_eq!(
        root.query_selector("[data-geometry='scene-part']")
            .unwrap()
            .unwrap()
            .get_attribute("transform")
            .as_deref(),
        Some("translate(9,4) rotate(0) ")
    );
    click("#canvas-select");
    gloo_timers::future::TimeoutFuture::new(5).await;
    assert!(
        root.query_selector("[data-geometry='keycap'][data-selected='true']")
            .unwrap()
            .is_some()
    );
    click("#canvas-accepted");
    gloo_timers::future::TimeoutFuture::new(5).await;
    assert_eq!(
        root.query_selector("[data-geometry='part-hit-area']")
            .unwrap()
            .unwrap()
            .get_attribute("width")
            .as_deref(),
        Some("30"),
        "accepted geometry must refresh cached graphics"
    );
    click("#canvas-hide");
    gloo_timers::future::TimeoutFuture::new(5).await;
    assert!(
        root.query_selector("[data-geometry='scene-part']")
            .unwrap()
            .is_none()
    );
    root.remove();
}

/// Ticket 07: the selected part's outline turns accent while unselected
/// part outlines stay violet.
#[wasm_bindgen_test]
async fn selected_part_outline_turns_accent_while_unselected_stays_violet() {
    let document = web_sys::window().unwrap().document().unwrap();
    let root = document.create_element("div").unwrap();
    document.body().unwrap().append_child(&root).unwrap();
    dioxus_web::launch::launch_virtual_dom(
        VirtualDom::new(render_fixture),
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
    click("#canvas-select");
    gloo_timers::future::TimeoutFuture::new(5).await;
    let outlines = root
        .query_selector_all("[data-geometry='part-outline']")
        .unwrap();
    assert_eq!(outlines.length(), 2, "both parts draw outlines");
    let selected = root
        .query_selector_all("[data-geometry='part-outline'][data-selected='true']")
        .unwrap();
    assert_eq!(
        selected.length(),
        1,
        "exactly the selected part turns accent"
    );
    root.remove();
}

/// Ticket 23 slice 9, RED: a transform-drag preview marks the effective
/// poses amber until the preview clears.
#[wasm_bindgen_test]
async fn transform_preview_marks_the_scene_amber_until_it_clears() {
    let document = web_sys::window().unwrap().document().unwrap();
    let root = document.create_element("div").unwrap();
    document.body().unwrap().append_child(&root).unwrap();
    dioxus_web::launch::launch_virtual_dom(
        VirtualDom::new(render_fixture),
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
        root.query_selector("[data-scene-preview='true']")
            .unwrap()
            .is_none(),
        "the settled scene carries no preview mark"
    );
    click("#canvas-preview");
    gloo_timers::future::TimeoutFuture::new(5).await;
    assert!(
        root.query_selector("[data-scene-preview='true']")
            .unwrap()
            .is_some(),
        "the preview marks its effective poses amber"
    );
    click("#canvas-preview-off");
    gloo_timers::future::TimeoutFuture::new(5).await;
    assert!(
        root.query_selector("[data-scene-preview='true']")
            .unwrap()
            .is_none(),
        "clearing the preview restores the scene"
    );
    root.remove();
}

#[wasm_bindgen_test]
fn projection_indexes_visible_parts_enabled_matrix_keycaps_and_first_transform() {
    let visible: Vec<Part> = vec![
        serde_json::from_value(json!({
            "id": "key-1", "definitionId": "switch", "reference": "SW1",
            "pose": {"at": {"x": 0.0, "y": 0.0}, "rotation": 0.0}, "side": "front"
        }))
        .unwrap(),
    ];
    let matrix: Matrix = serde_json::from_value(json!({
        "id": "matrix", "rows": 1, "columns": 1, "pitch": {"x": 19.0, "y": 20.0},
        "origin": {"x": 0.0, "y": 0.0}, "definitionId": "switch", "partIds": ["key-1"],
        "boardId": "board", "edgeGap": {"x": 2.0, "y": 3.0}, "cells": []
    }))
    .unwrap();
    let matrix_scene = MatrixScene {
        matrix_id: "matrix".into(),
        cells: vec![
            MatrixSceneCell {
                shift_frame: Default::default(),
                offset_frame: Default::default(),
                row_shift: Vec2 { x: 0.0, y: 0.0 },
                key_shift: Vec2 { x: 0.0, y: 0.0 },
                row_bend: 0.0,
                key_bend: 0.0,
                cumulative_shift: Vec2 { x: 0.0, y: 0.0 },
                cumulative_bend: 0.0,
                row: 0,
                column: 0,
                enabled: true,
                member_id: Some("key-1".into()),
                pose: Pose2 {
                    at: Vec2 { x: 0.0, y: 0.0 },
                    rotation: 0.0,
                },
            },
            MatrixSceneCell {
                shift_frame: Default::default(),
                offset_frame: Default::default(),
                row_shift: Vec2 { x: 0.0, y: 0.0 },
                key_shift: Vec2 { x: 0.0, y: 0.0 },
                row_bend: 0.0,
                key_bend: 0.0,
                cumulative_shift: Vec2 { x: 0.0, y: 0.0 },
                cumulative_bend: 0.0,
                row: 0,
                column: 1,
                enabled: false,
                member_id: Some("disabled".into()),
                pose: Pose2 {
                    at: Vec2 { x: 0.0, y: 0.0 },
                    rotation: 0.0,
                },
            },
        ],
        columns: vec![],
        companions: vec![],
    };
    let matrix_scenes = BTreeMap::from([("matrix", &matrix_scene)]);
    let matrices = [&matrix];
    let transforms = [
        Transform {
            id: "key-1".into(),
            pose: Pose2 {
                at: Vec2 { x: 5.0, y: 6.0 },
                rotation: 7.0,
            },
        },
        Transform {
            id: "key-1".into(),
            pose: Pose2 {
                at: Vec2 { x: 8.0, y: 9.0 },
                rotation: 10.0,
            },
        },
    ];
    let visible_refs: Vec<_> = visible.iter().collect();
    let projection = project(&visible_refs, &matrices, &matrix_scenes, &transforms);

    assert!(projection.visible_by_id.contains_key("key-1"));
    assert_eq!(
        projection.matrix_keycaps.get("key-1"),
        Some(&Vec2 { x: 17.0, y: 17.0 })
    );
    assert!(!projection.matrix_keycaps.contains_key("disabled"));
    assert_eq!(
        projection.part_transforms.get("key-1").unwrap().at,
        Vec2 { x: 5.0, y: 6.0 }
    );
}
