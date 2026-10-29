//! Ticket 18: ghost of the previous spacing, mounted against the real Editor,
//! Session and Core. A committed edit that widens the spacing raises the
//! dashed ghost of the previous spacing with its delta note and grows the
//! view; narrowing ghosts the other way and shrinks it. Selection, the next
//! edit, undo and redo retire the ghost, and a drag preview leaves the view
//! box still.
use super::*;
use crate::runtime::project_name_test_support as support;
use boardstudio_core::model::{
    Board, EditOperation, Layout, LayoutMirrorLink, MatrixPlacementChange, MirroredCorrespondence,
    MirroredItemKind, ProjectDoc, Vec2,
};
use boardstudio_web_ui_model::spacing_ghost::spacing_note;
use boardstudio_web_ui_shared::test_wait::wait_until;
use wasm_bindgen::JsCast;
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

fn split_document() -> ProjectDoc {
    let mut document = ProjectDoc::empty("split-ghost", "Split ghost");
    document.boards.push(Board {
        id: "board".into(),
        name: "Board".into(),
        outline_ids: vec![],
        part_ids: vec!["matrix/left/r0c0".into(), "matrix/right/r0c0".into()],
        net_ids: vec![],
        thickness: 1.6,
        traces: vec![],
        vias: vec![],
    });
    document.definitions.push(
        serde_json::from_value(serde_json::json!({
            "id": "switch:base", "name": "Switch", "kind": "switch",
            "courtyard": [{"x": -7, "y": -7}, {"x": 7, "y": -7}, {"x": 7, "y": 7}],
            "pads": [], "keycap": {"x": 18, "y": 18}
        }))
        .unwrap(),
    );
    for (id, reference, x) in [
        ("matrix/left/r0c0", "SW1", -29.0),
        ("matrix/right/r0c0", "SW2", 29.0),
    ] {
        document.parts.push(
            serde_json::from_value(serde_json::json!({
                "id": id, "definitionId": "switch:base", "reference": reference,
                "pose": {"at": {"x": x, "y": 0.0}, "rotation": 0}, "side": "front"
            }))
            .unwrap(),
        );
    }
    // Half-frame origins for a 40 mm stored gap about axis 0: the halves sit
    // at ∓20, so local ∓9 puts the keys at board ∓29 (key edges ∓20). Parts
    // stay board-world, matching ticket 10's Core fixtures.
    for (id, origin_x, part) in [
        ("left", -9.0, "matrix/left/r0c0"),
        ("right", 9.0, "matrix/right/r0c0"),
    ] {
        document.matrices.push(
            serde_json::from_value(serde_json::json!({
                "id": id, "rows": 1, "columns": 1,
                "pitch": {"x": 19.05, "y": 19.05}, "origin": {"x": origin_x, "y": 0.0},
                "edgeGap": {"x": 1.0, "y": 1.0},
                "definitionId": "switch:base", "partIds": [part], "boardId": "board",
                "cells": [{"row": 0, "column": 0, "enabled": true, "assemblies": []}]
            }))
            .unwrap(),
        );
    }
    document.layouts.push(Layout {
        id: "source-layout".into(),
        name: "Left half".into(),
        board_id: "board".into(),
        matrix_ids: vec!["left".into()],
        part_ids: vec![],
        mirror_link: None,
    });
    document.layouts.push(Layout {
        id: "paired-layout".into(),
        name: "Right half".into(),
        board_id: "board".into(),
        matrix_ids: vec!["right".into()],
        part_ids: vec![],
        mirror_link: Some(LayoutMirrorLink {
            correspondences: vec![MirroredCorrespondence {
                companion_mode: None,
                companion_tweak: None,
                kind: MirroredItemKind::Matrix,
                source_id: "left".into(),
                mirror_id: "right".into(),
            }],
            exceptions: vec![],
            source_id: "source-layout".into(),
            axis_x: 0.0,
            split_gap: Some(40.0),
            spacing: Some(40.0),
        }),
    });
    document
}

fn host() -> Element {
    let workspace = use_signal(|| "Layout");
    use_context_provider(|| WorkspaceState(workspace));
    use_context_provider(|| ExportReturnWorkspace(workspace));
    let version = use_signal(|| 0u64);
    use_context_provider(|| version);
    let runtime = use_context::<Rc<Runtime>>();
    use_hook(move || {
        runtime.subscribe(Rc::new(move || {
            let mut version = version;
            version += 1;
        }))
    });
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
    use_context_provider(|| ResolvedTheme(resolved));
    let selected = use_signal(|| None);
    let anchor = use_signal(|| None);
    let generation = use_signal(|| 0u64);
    use_context_provider(|| SelectionAdapter::new(selected, anchor, generation));
    let hidden = use_signal(BTreeSet::new);
    let modules_hidden = use_signal(BTreeSet::new);
    let footprints = use_signal(|| false);
    use_context_provider(|| LayerVisibility {
        hidden,
        modules_hidden,
        footprints,
    });
    boardstudio_web_ui_model::state::use_test_case_generation_state();
    rsx! {
        Editor {}
    }
}

fn count(root: &web_sys::Element, selector: &str) -> u32 {
    root.query_selector_all(selector).unwrap().length()
}

fn click(root: &web_sys::Element, selector: &str) {
    root.query_selector(selector)
        .unwrap()
        .unwrap_or_else(|| panic!("missing {selector}"))
        .dyn_into::<HtmlElement>()
        .unwrap()
        .click();
}

fn ghost_note(root: &web_sys::Element) -> String {
    root.query_selector(".m1-spacing-ghost-note")
        .unwrap()
        .map(|note| note.text_content().unwrap_or_default())
        .unwrap_or_default()
}

fn canvas_svg(root: &web_sys::Element) -> web_sys::SvgElement {
    root.query_selector("svg.m1-canvas")
        .unwrap()
        .expect("the Layout canvas renders")
        .dyn_into::<web_sys::SvgElement>()
        .expect("the canvas is an SVG element")
}

fn view_box_width(root: &web_sys::Element) -> f64 {
    canvas_svg(root)
        .get_attribute("viewBox")
        .unwrap_or_default()
        .split_whitespace()
        .nth(2)
        .and_then(|width| width.parse::<f64>().ok())
        .unwrap_or(0.0)
}

/// The view-box width the canvas must show for the live accepted state: the
/// accepted content bounds, expanded to the live surface aspect, over the
/// live camera zoom. Mirrors the Editor's own derivation exactly, so equality
/// proves the view tracks the content with no camera drift.
fn expected_view_width(root: &web_sys::Element, runtime: &Rc<Runtime>) -> f64 {
    let rect = canvas_svg(root).get_bounding_client_rect();
    let surface = (rect.width(), rect.height());
    let model = runtime.model();
    let snapshot = model.accepted.as_ref().expect("a board is open");
    let bounds = keycaps_fit::layout_canvas_bounds(&snapshot.document, &snapshot.scene, "board")
        .expect("the split board has bounds");
    let aspect = keycaps_fit::aspect_bounds(bounds, surface);
    (aspect.1 - aspect.0).max(50.0) / model.camera.zoom
}

fn assert_view_tracks_content(root: &web_sys::Element, runtime: &Rc<Runtime>, stage: &str) {
    let shown = view_box_width(root);
    let expected = expected_view_width(root, runtime);
    assert!(
        (shown - expected).abs() < 1e-6,
        "the view tracks the content {stage}: shows {shown}, content needs {expected}"
    );
}

fn effective_spacing(document: &ProjectDoc) -> f64 {
    document
        .layouts
        .iter()
        .find_map(|layout| layout.mirror_link.as_ref())
        .and_then(|link| link.spacing.or(link.split_gap))
        .expect("the split pair carries a spacing")
}

#[wasm_bindgen_test]
async fn spacing_change_ghosts_refits_and_retires() {
    let runtime = support::new_runtime();
    support::open_document(&runtime, split_document()).await;
    let dom_document = web_sys::window().unwrap().document().unwrap();
    let root = dom_document.create_element("div").unwrap();
    dom_document.body().unwrap().append_child(&root).unwrap();
    let dom = VirtualDom::new(host);
    dom.provide_root_context(runtime.clone());
    dioxus_web::launch::launch_virtual_dom(
        dom,
        dioxus_web::Config::new().rootnode(root.clone().into()),
    );

    wait_until("both split halves render", || {
        count(&root, ".m1-matrix-key") == 2
    })
    .await;
    assert_eq!(count(&root, ".m1-spacing-ghost"), 0);
    // The settled view already tracks the settled content exactly.
    assert_view_tracks_content(&root, &runtime, "on open");
    let settled_width = view_box_width(&root);
    assert!(settled_width > 0.0, "the canvas has a view box");
    let revision = runtime.model().accepted.as_ref().unwrap().document.revision;

    // Widen: push the left half toward the axis so the outlines would touch.
    // The adaptive pass widens the spacing instead, in the same Undo entry.
    let before = runtime.model().accepted.as_ref().unwrap().document.clone();
    let mut widened = (*before).clone();
    widened
        .matrices
        .iter_mut()
        .find(|matrix| matrix.id == "left")
        .expect("the left half exists")
        .origin
        .x = 15.0;
    support::replace_document(&runtime, "ghost-widen", "split-ghost", widened).await;
    let after_widen = runtime.model().accepted.as_ref().unwrap().document.clone();
    let widened_by = effective_spacing(&after_widen) - effective_spacing(&before);
    assert!(
        widened_by > 0.5,
        "the setup must widen the spacing, moved {widened_by:?}"
    );
    let expected_wide_note = spacing_note(widened_by);
    wait_until("the widening ghost reads on the axis", || {
        count(&root, ".m1-spacing-ghost") == 2 && ghost_note(&root) == expected_wide_note
    })
    .await;
    // The view refits to the widened content: the box tracks the new bounds
    // exactly, and it moved.
    assert_view_tracks_content(&root, &runtime, "after the widening commit");
    let wide_width = view_box_width(&root);
    assert!(
        (wide_width - settled_width).abs() > 1e-9,
        "the view refits after the spacing commit: {settled_width} -> {wide_width}"
    );

    // A drag preview leaves the view box still and preempts the ghost note.
    runtime.submit(Event::PreviewEdit {
        operation_id: runtime.operation(),
        transaction_id: "ghost-drag".into(),
        target_ids: vec!["left".into()],
        operation: EditOperation::SetMatrixPlacement {
            matrix_id: "left".into(),
            change: MatrixPlacementChange::KeyShift {
                row: 0,
                column: 0,
                value: Vec2 { x: 5.0, y: 0.0 },
            },
        },
    });
    support::run_pending(&runtime).await;
    wait_until("the drag preview lands", || {
        runtime.model().display_preview.is_some()
    })
    .await;
    assert!(
        (view_box_width(&root) - wide_width).abs() < 1e-9,
        "the view stays still while dragging"
    );
    assert!(
        ghost_note(&root).is_empty(),
        "the preview hides the spacing note while it is up"
    );
    let accepted = runtime.model().accepted.as_ref().unwrap().clone();
    runtime.submit(Event::ClearPreview {
        operation_id: runtime.operation(),
        token: accepted.token,
        revision: accepted.document.revision,
        board_id: "board".into(),
        transaction_id: "ghost-drag".into(),
    });
    support::run_pending(&runtime).await;
    wait_until("the spacing ghost returns after the preview clears", || {
        count(&root, ".m1-spacing-ghost") == 2 && ghost_note(&root) == expected_wide_note
    })
    .await;
    // Tree selection retires the ghost without touching the document.
    click(&root, "[id='m1-object-tree-half:source-layout']");
    wait_until("selection retires the ghost", || {
        count(&root, ".m1-spacing-ghost") == 0 && ghost_note(&root).is_empty()
    })
    .await;
    assert_eq!(
        runtime.model().accepted.as_ref().unwrap().document.revision,
        revision + 1,
        "selection is view-only: no new revision"
    );
    // Narrow back to the stored floor: the ghost reads the other way and the
    // view shrinks with the content.
    let moved = runtime.model().accepted.as_ref().unwrap().document.clone();
    let mut narrowed = (*moved).clone();
    narrowed
        .matrices
        .iter_mut()
        .find(|matrix| matrix.id == "left")
        .expect("the left half exists")
        .origin
        .x = -9.0;
    // The stored gap is untouched by the edit; only the evaluated spacing moves.
    let stored = narrowed
        .layouts
        .iter()
        .find_map(|layout| layout.mirror_link.as_ref().and_then(|link| link.split_gap))
        .expect("the stored gap survives");
    support::replace_document(&runtime, "ghost-narrow", "split-ghost", narrowed).await;
    let after_narrow = runtime.model().accepted.as_ref().unwrap().document.clone();
    let narrowed_by = effective_spacing(&after_narrow) - effective_spacing(&moved);
    assert!(narrowed_by < -0.5, "the setup must narrow the spacing");
    assert!(
        (effective_spacing(&after_narrow) - stored).abs() < 1e-6,
        "narrowing stops at the stored floor"
    );
    let expected_narrow_note = spacing_note(narrowed_by);
    assert!(
        expected_narrow_note.starts_with("−"),
        "narrowing notes use the prototype minus"
    );
    wait_until("the narrowing ghost reads on the axis", || {
        count(&root, ".m1-spacing-ghost") == 2 && ghost_note(&root) == expected_narrow_note
    })
    .await;
    // The view refits the other way too: it tracks the narrowed content
    // exactly, and the box moved back.
    assert_view_tracks_content(&root, &runtime, "after the narrowing commit");
    assert!(
        (view_box_width(&root) - wide_width).abs() > 1e-9,
        "the view refits after the narrowing commit"
    );
    // Undo retires the ghost instead of re-raising it; redo keeps it retired.
    let narrowed_revision = runtime.model().accepted.as_ref().unwrap().document.revision;
    runtime.submit(Event::Undo {
        operation_id: runtime.operation(),
    });
    support::run_pending(&runtime).await;
    wait_until("undo lands and retires the ghost", || {
        runtime.model().accepted.as_ref().unwrap().document.revision == narrowed_revision + 1
            && count(&root, ".m1-spacing-ghost") == 0
            && ghost_note(&root).is_empty()
    })
    .await;
    // The view refits after undo as well: the restored content drives the box.
    assert_view_tracks_content(&root, &runtime, "after undo");
    runtime.submit(Event::Redo {
        operation_id: runtime.operation(),
    });
    support::run_pending(&runtime).await;
    wait_until("redo lands", || {
        runtime.model().accepted.as_ref().unwrap().document.revision == narrowed_revision + 2
    })
    .await;
    gloo_timers::future::TimeoutFuture::new(20).await;
    assert_eq!(
        count(&root, ".m1-spacing-ghost"),
        0,
        "redo keeps the ghost retired"
    );
    assert!(
        ghost_note(&root).is_empty(),
        "redo keeps the axis note retired"
    );
    root.remove();
}
