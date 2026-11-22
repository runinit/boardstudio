use super::super::{
    CompactPanelState, Editor, ExportReturnWorkspace, LayerVisibility, PreferenceStorageWarning,
    ResolvedAppearance, SelectionAdapter, SetupGuideRequest, ThemeState, WorkspaceState, objects,
    selection,
};
use super::*;
use boardstudio_application::{Event, Session};
use boardstudio_core::CoreEngine;
use boardstudio_web_test_runtime::project_name_test_support as support;
use boardstudio_web_ui_shared::test_wait::wait_until;
use serde_json::json;
use std::cell::RefCell;
use std::collections::BTreeSet;
use std::rc::Rc;
use wasm_bindgen::JsCast;
use wasm_bindgen_test::wasm_bindgen_test;

/// A split pair with a detached diode part: left/right 1x1 switch
/// matrices under a mirror link, plus `d1` placed from the left half
/// origin so the canvas draws the dim label, the axis note, the ring
/// and a part name together.
fn chrome_fixture() -> (Arc<ProjectDoc>, Arc<SceneDelta>) {
    let mut document = ProjectDoc::empty("chrome", "Chrome");
    document.parts.push(
        serde_json::from_value(json!({
            "id": "matrix/left/r0c0", "definitionId": "switch", "reference": "SW1",
            "pose": {"at": {"x": 0.0, "y": 0.0}, "rotation": 0.0}, "side": "front",
            "keycap": {"x": 18.0, "y": 18.0}
        }))
        .unwrap(),
    );
    document.matrices.push(
        serde_json::from_value(json!({
            "id": "left", "rows": 1, "columns": 1,
            "pitch": {"x": 19.05, "y": 19.05}, "origin": {"x": -30.0, "y": 0.0},
            "definitionId": "switch", "partIds": ["matrix/left/r0c0"], "boardId": "board",
            "cells": [{"row": 0, "column": 0, "enabled": true, "definitionId": "switch", "assemblies": []}]
        }))
        .unwrap(),
    );
    document.parts.push(
        serde_json::from_value(json!({
            "id": "d1", "definitionId": "diode", "reference": "D1",
            "pose": {"at": {"x": -20.0, "y": 10.0}, "rotation": 0.0}, "side": "front",
            "keycap": null
        }))
        .unwrap(),
    );
    document.definitions.push(
        serde_json::from_value(json!({
            "id": "switch", "name": "Switch", "kind": "switch", "courtyard": [], "pads": []
        }))
        .unwrap(),
    );
    document.definitions.push(
        serde_json::from_value(json!({
            "id": "diode", "name": "Diode", "kind": "passive",
            "courtyard": [
                {"x": -7.0, "y": -7.0}, {"x": 7.0, "y": -7.0},
                {"x": 7.0, "y": 7.0}, {"x": -7.0, "y": 7.0}
            ],
            "pads": []
        }))
        .unwrap(),
    );
    document.boards.push(
        serde_json::from_value(json!({
            "id": "board", "name": "Board",
            "partIds": ["matrix/left/r0c0", "d1"],
            "outlineIds": [], "netIds": [], "thickness": 1.6
        }))
        .unwrap(),
    );
    document.layouts.push(
        serde_json::from_value(json!({
            "id": "left", "name": "Left half", "boardId": "board",
            "matrixIds": ["left"], "partIds": ["d1"]
        }))
        .unwrap(),
    );
    document.layouts.push(
        serde_json::from_value(json!({
            "id": "right", "name": "Right half", "boardId": "board",
            "matrixIds": [], "partIds": []
        }))
        .unwrap(),
    );
    document.mirrored_pairs = serde_json::from_value(json!([{
        "id": "pair",
        "source": {"kind": "sameBoardLayout", "id": "left"},
        "destination": {"kind": "sameBoardLayout", "id": "right"},
        "mapping": {"at": {"x": 0.0, "y": 0.0}, "rotation": 0.0},
        "overrides": []
    }]))
    .unwrap();
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
    let scene = Arc::new(
        serde_json::from_value(json!({
            "revision": 0, "transactionId": "fixture", "changedIds": [], "transforms": [],
            "matrixScenes": [
                {"matrixId": "left", "cells": [cell("left", -30.0)], "columns": []},
                {"matrixId": "mirror_70616972_left", "cells": [cell("mirror_70616972_left", 30.0)], "columns": []}
            ],
            "contours": [], "boardContours": [], "boardReadiness": [], "findings": [],
            "readiness": {"layout": true, "outline": false, "pcb": false, "case": false}
        }))
        .unwrap(),
    );
    (document, scene)
}

fn chrome_badge() -> anchor_badge::AnchorBadge {
    anchor_badge::AnchorBadge {
        anchor: Vec2 { x: -30.0, y: 0.0 },
        selection: Vec2 { x: -11.0, y: 0.0 },
        highlight: Vec::new(),
        label: "Column 1 · spread, stagger, splay carry".to_owned(),
        tied: true,
    }
}

#[derive(Props, Clone, PartialEq)]
struct ChromeCaseProps {
    document: Arc<ProjectDoc>,
    scene: Arc<SceneDelta>,
    ghost: SpacingGhost,
    badge: anchor_badge::AnchorBadge,
    view_width: f64,
}

#[component]
fn RenderChromeCase(props: ChromeCaseProps) -> Element {
    let highlight = use_signal(|| None::<Vec<String>>);
    use_context_provider(|| super::super::RelationHighlight(highlight));
    // A real viewport mapping: 800x600 CSS px over the view box, so the
    // browser maps world units to screen pixels and getBBox reads true
    // on-screen sizes.
    let view_box = format!("0 0 {} {}", props.view_width, props.view_width * 0.75);
    rsx! {
        svg { width: "800", height: "600", view_box: "{view_box}",
            Scene {
                source: SceneSource { document: props.document, scene: props.scene, board_id: "board".into() },
                preview_active: false,
                accepted_scene: None,
                spacing_ghost: Some(props.ghost),
                selected_ids: Vec::new(), selected_context: None,
                view_width_world: props.view_width, surface_width_px: 800.0,
                anchor_badge: Some(props.badge),
                focused_half: Some(SplitHalf::Left),
                keys_visible: true, components_visible: true, keycaps_visible: true, footprints_visible: false,
                on_matrix_pointer_down: |_| {}, on_part_pointer_down: |_| {},
            }
        }
    }
}

async fn mount_chrome(view_width: f64) -> web_sys::Element {
    let (document, scene) = chrome_fixture();
    let document = match boardstudio_application::resolved_document(&document) {
        std::borrow::Cow::Borrowed(_) => document,
        std::borrow::Cow::Owned(resolved) => Arc::new(resolved),
    };
    let ghost = SpacingGhost {
        scene: scene.clone(),
        delta_mm: 10.0,
        axis_x: 0.0,
    };
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
            RenderChromeCase,
            ChromeCaseProps {
                document,
                scene,
                ghost,
                badge: chrome_badge(),
                view_width,
            },
        ),
        dioxus_web::Config::new().rootnode(root.clone().into()),
    );
    wait_until("the fixed-size chrome renders", || {
        root.query_selector("[data-focus-label='split']")
            .unwrap()
            .is_some()
    })
    .await;
    root
}

/// World units behind an inline `font-size:…px` style. Browsers may
/// serialize the declaration with whitespace or a trailing semicolon.
fn style_world_size(root: &web_sys::Element, selector: &str) -> f64 {
    let node = root
        .query_selector(selector)
        .unwrap()
        .unwrap_or_else(|| panic!("missing {selector}"));
    let style = node
        .dyn_into::<web_sys::SvgElement>()
        .unwrap()
        .get_attribute("style")
        .unwrap_or_default();
    style
        .split(';')
        .find_map(|declaration| {
            let (name, value) = declaration.split_once(':')?;
            (name.trim() == "font-size").then(|| value.trim().to_owned())
        })
        .and_then(|value| value.trim_end_matches("px").parse().ok())
        .unwrap_or_else(|| panic!("no font-size in {selector} style {style:?}"))
}

/// World units per screen pixel for the mounted surface width.
fn on_screen(world: f64, view_width: f64) -> f64 {
    world / (view_width / 800.0)
}

/// True on-screen height in CSS px through the browser's own view-box
/// mapping (all transforms included), independent of the helper under
/// test.
fn bbox_height(root: &web_sys::Element, selector: &str) -> f64 {
    root.query_selector(selector)
        .unwrap()
        .unwrap_or_else(|| panic!("missing {selector}"))
        .get_bounding_client_rect()
        .height()
}

/// Ticket 02 RED: one scene at two zoom levels keeps every technical
/// label at 11 px on screen. It fails while labels use world units.
#[wasm_bindgen_test]
async fn labels_hold_eleven_px_across_zooms() {
    let mut real_heights: Vec<Vec<f64>> = Vec::new();
    for view_width in [200.0, 100.0] {
        let root = mount_chrome(view_width).await;
        for selector in [
            "[data-focus-label='split']",
            "[data-ghost-note='spacing']",
            "[data-part-label]",
        ] {
            let px = on_screen(style_world_size(&root, selector), view_width);
            assert!(
                (px - 11.0).abs() < 0.5,
                "{selector} reads {px:.2} px on screen at view width {view_width}, want 11"
            );
        }
        real_heights.push(
            [
                "[data-focus-label='split']",
                "[data-ghost-note='spacing']",
                "[data-part-label]",
            ]
            .iter()
            .map(|selector| bbox_height(&root, selector))
            .collect(),
        );
        assert!(
            root.query_selector("[data-anchor-chrome='label']")
                .unwrap()
                .is_none(),
            "the canvas carries no badge label at view width {view_width}; it lives in the readout"
        );
        root.remove();
    }
    // True pixels through the browser mapping: the same label measures
    // the same height at both zooms.
    for (wide, close) in real_heights[0].iter().zip(real_heights[1].iter()) {
        assert!(
            (wide - close).abs() < 0.5,
            "label measures {wide:.2} px at one zoom and {close:.2} px at the other"
        );
    }
}

/// Ticket 02: the anchor ring holds 7 px on screen at both zooms.
#[wasm_bindgen_test]
async fn anchor_ring_holds_seven_px_across_zooms() {
    for view_width in [200.0, 100.0] {
        let root = mount_chrome(view_width).await;
        let ring = root
            .query_selector("[data-anchor-chrome='ring']")
            .unwrap()
            .expect("the anchor ring renders");
        let radius: f64 = ring
            .get_attribute("r")
            .unwrap_or_default()
            .parse()
            .expect("the ring radius parses");
        let diameter = on_screen(2.0 * radius, view_width);
        assert!(
            (diameter - 7.0).abs() < 0.5,
            "the ring reads {diameter:.2} px across at view width {view_width}, want 7"
        );
        // True pixels through the browser mapping.
        let real = bbox_height(&root, "[data-anchor-chrome='ring']");
        assert!(
            (real - 7.0).abs() < 0.5,
            "the ring measures {real:.2} px across at view width {view_width}, want 7"
        );
        root.remove();
    }
}

#[derive(Clone)]
struct ChromeProbe {
    runtime: Rc<crate::runtime::Runtime>,
    adapter: Rc<RefCell<Option<SelectionAdapter>>>,
}

impl ChromeProbe {
    fn select(&self, context: objects::TreeContext) {
        selection::submit_context(
            &self.runtime,
            &self.adapter.borrow().clone().expect("mounted adapter"),
            selection::ContextRequest {
                scope: self.runtime.scope().expect("opened project scope"),
                context,
                mode: boardstudio_application::SelectionMode::Replace,
            },
        );
    }
}

fn chrome_host() -> Element {
    let probe = use_context::<ChromeProbe>();
    let workspace = use_signal(|| "Layout");
    use_context_provider(|| WorkspaceState(workspace));
    use_context_provider(|| ExportReturnWorkspace(workspace));
    let version = use_signal(|| 0u64);
    use_context_provider(|| version);
    let created = use_signal(|| None::<SetupGuideRequest>);
    use_context_provider(|| created);
    let objects_open = use_signal(|| true);
    let inspector_open = use_signal(|| true);
    use_context_provider(|| CompactPanelState {
        objects_open,
        inspector_open,
    });
    let warning = use_signal(|| false);
    use_context_provider(|| PreferenceStorageWarning(warning));
    let theme = use_signal(|| "light");
    use_context_provider(|| ThemeState(theme));
    let resolved = use_memo(|| "light");
    use_context_provider(|| ResolvedAppearance(resolved));
    let selected = use_signal(|| None);
    let anchor = use_signal(|| None);
    let generation = use_signal(|| 0u64);
    let adapter = SelectionAdapter::new(selected, anchor, generation);
    *probe.adapter.borrow_mut() = Some(adapter.clone());
    use_context_provider(|| adapter);
    let hidden = use_signal(BTreeSet::new);
    let modules_hidden = use_signal(BTreeSet::new);
    let footprints = use_signal(|| false);
    use_context_provider(|| LayerVisibility {
        hidden,
        modules_hidden,
        footprints,
    });
    boardstudio_web_ui_model::state::use_test_case_generation_state();
    rsx! { boardstudio_web_ui_shared::ui::toast::ToastProvider { Editor {} } }
}

/// Mount the real Layout editor on the chrome document (empty matrix
/// scenes: only the detached diode part draws).
async fn mount_chrome_editor(root_id: &'static str) -> ChromeProbe {
    let (document, _) = chrome_fixture();
    let runtime = support::new_runtime();
    support::install(
        &runtime,
        Session::new(CoreEngine::validate_open),
        CoreEngine::new(),
    );
    support::install_memory_persistence(&runtime);
    runtime.submit(Event::Open {
        operation_id: runtime.operation(),
        document: (*document).clone(),
    });
    support::run_pending(&runtime).await;
    let probe = ChromeProbe {
        runtime: runtime.clone(),
        adapter: Rc::default(),
    };
    let dom_document = web_sys::window().unwrap().document().unwrap();
    let root = dom_document.create_element("div").unwrap();
    root.set_id(root_id);
    dom_document.body().unwrap().append_child(&root).unwrap();
    let dom = VirtualDom::new(chrome_host);
    dom.provide_root_context(runtime.clone());
    dom.provide_root_context(probe.clone());
    dioxus_web::launch::launch_virtual_dom(
        dom,
        dioxus_web::Config::new().rootnode(root.clone().into()),
    );
    wait_until("the Layout canvas draws the diode part", || {
        dom_document
            .query_selector(&format!("#{root_id} [data-geometry='scene-part']"))
            .unwrap()
            .is_some()
    })
    .await;
    probe
}

fn readout(root_id: &'static str) -> String {
    web_sys::window()
        .unwrap()
        .document()
        .unwrap()
        .query_selector(&format!("#{root_id} [data-canvas-anchor-readout]"))
        .unwrap()
        .map(|node| node.text_content().unwrap_or_default())
        .unwrap_or_default()
}

/// Ticket 02: a part selection names the half origin in the readout, and
/// clearing the selection clears the readout with the ring.
#[wasm_bindgen_test]
async fn part_selection_reads_placed_from_and_clears() {
    let root_id = "chrome-part-readout";
    let probe = mount_chrome_editor(root_id).await;
    probe.select(objects::TreeContext::Component {
        part_id: Some("d1".into()),
        matrix_id: None,
        row: None,
        column: None,
        assembly_id: None,
    });
    wait_until("the readout names the part anchor", || {
        readout(root_id).contains("Left half origin · default")
    })
    .await;
    assert_eq!(
        readout(root_id),
        "Placed from Left half origin · default",
        "the readout carries the full Placed from text"
    );
    let dom_document = web_sys::window().unwrap().document().unwrap();
    assert!(
        dom_document
            .query_selector(&format!("#{root_id} [data-anchor-chrome='ring']"))
            .unwrap()
            .is_some(),
        "the 7 px ring marks the anchor while the readout stands"
    );
    probe.select(objects::TreeContext::Board {
        board_id: "board".into(),
    });
    wait_until("clearing the selection clears the readout", || {
        readout(root_id).is_empty()
    })
    .await;
    assert!(
        dom_document
            .query_selector(&format!("#{root_id} [data-anchor-chrome='badge']"))
            .unwrap()
            .is_none(),
        "clearing the selection clears the ring with the readout"
    );
}
