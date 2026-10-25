//! Ticket 14: the unfocused half dims in Layout only; a canvas click moves the
//! view-only focus while tree selection, workspace switches and spacing edits
//! keep it. Mounted against the real Editor, Session and Core.
use super::*;
use crate::runtime::project_name_test_support as support;
use boardstudio_core::model::{
    Board, Layout, LayoutMirrorLink, MirroredCorrespondence, MirroredItemKind, ProjectDoc,
};
use boardstudio_web_ui_shared::test_wait::wait_until;
use wasm_bindgen::JsCast;
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

fn split_document() -> ProjectDoc {
    let mut document = ProjectDoc::empty("split-focus", "Split focus");
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
        ("matrix/left/r0c0", "SW1", -30.0),
        ("matrix/right/r0c0", "SW2", 30.0),
    ] {
        document.parts.push(
            serde_json::from_value(serde_json::json!({
                "id": id, "definitionId": "switch:base", "reference": reference,
                "pose": {"at": {"x": x, "y": 0.0}, "rotation": 0}, "side": "front"
            }))
            .unwrap(),
        );
    }
    for (id, origin_x, part) in [
        ("left", -30.0, "matrix/left/r0c0"),
        ("right", 30.0, "matrix/right/r0c0"),
    ] {
        document.matrices.push(
            serde_json::from_value(serde_json::json!({
                "id": id, "rows": 1, "columns": 1,
                "pitch": {"x": 19, "y": 19}, "origin": {"x": origin_x, "y": 0.0},
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
            split_gap: None,
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
    // The header workspace tabs live above `Editor` and write this same
    // signal; these buttons drive the identical switch for the test.
    let mut workspace_switch = workspace;
    rsx! {
        button { id: "split-focus-go-pcb", onclick: move |_| workspace_switch.set("PCB"), "Go PCB" }
        button { id: "split-focus-go-layout", onclick: move |_| workspace_switch.set("Layout"), "Go Layout" }
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

fn canvas_pointer_down(root: &web_sys::Element, selector: &str) {
    let target = root
        .query_selector(selector)
        .unwrap()
        .unwrap_or_else(|| panic!("missing {selector}"));
    let pointer = js_sys::eval("new PointerEvent('pointerdown', {bubbles: true})")
        .unwrap()
        .dyn_into::<web_sys::Event>()
        .unwrap();
    target.dispatch_event(&pointer).unwrap();
}

fn dim_label(root: &web_sys::Element) -> String {
    root.query_selector(".m1-split-focus-label")
        .unwrap()
        .map(|label| label.text_content().unwrap_or_default())
        .unwrap_or_default()
}

#[wasm_bindgen_test]
async fn dimmed_half_click_focuses_and_tree_pcb_roundtrip_keeps_focus() {
    let runtime = support::new_runtime();
    support::open_document(&runtime, split_document()).await;
    let revision = runtime.model().accepted.as_ref().unwrap().document.revision;
    let dom_document = web_sys::window().unwrap().document().unwrap();
    let root = dom_document.create_element("div").unwrap();
    dom_document.body().unwrap().append_child(&root).unwrap();
    let dom = VirtualDom::new(host);
    dom.provide_root_context(runtime.clone());
    dioxus_web::launch::launch_virtual_dom(
        dom,
        dioxus_web::Config::new().rootnode(root.clone().into()),
    );

    // Focus starts on the source half: the right half dims with variant B's
    // grey dashed outline and inline click-to-edit label.
    wait_until("the right half dims with a click-to-edit label", || {
        count(&root, ".m1-matrix-key.is-dimmed") == 1
            && dim_label(&root).contains("Right half · linked mirror · click to edit here")
    })
    .await;

    // Clicking the dimmed half on the canvas focuses it and still selects.
    canvas_pointer_down(&root, ".m1-matrix-key[data-matrix-id='right']");
    wait_until(
        "the canvas click focuses and selects the right half",
        || {
            dim_label(&root).contains("Left half · linked mirror · click to edit here")
                && runtime.model().selected_part_ids == vec!["matrix/right/r0c0".to_owned()]
        },
    )
    .await;
    assert_eq!(count(&root, ".m1-matrix-key.is-dimmed"), 1);

    // Tree selection never moves focus.
    click(&root, "[id='m1-object-tree-half:source-layout']");
    wait_until("the tree selection applies", || {
        runtime.model().selected_part_ids == vec!["matrix/left/r0c0".to_owned()]
    })
    .await;
    assert!(
        dim_label(&root).contains("Left half · linked mirror · click to edit here"),
        "tree selection must not move focus"
    );

    // PCB never dims.
    click(&root, "#split-focus-go-pcb");
    wait_until("the PCB workspace shows", || {
        root.query_selector("#m1-workspace-panel[aria-label='PCB workspace']")
            .unwrap()
            .is_some()
    })
    .await;
    assert_eq!(count(&root, ".m1-split-focus-label"), 0);
    assert_eq!(count(&root, ".m1-matrix-key.is-dimmed"), 0);

    // Switching back keeps focus.
    click(&root, "#split-focus-go-layout");
    wait_until("returning to Layout keeps focus on the right half", || {
        dim_label(&root).contains("Left half · linked mirror · click to edit here")
    })
    .await;

    assert_eq!(
        runtime.model().accepted.as_ref().unwrap().document.revision,
        revision,
        "focus is view-only: no document revision and no Undo entry"
    );

    // A committed document revision (what spacing changes ride on) keeps focus.
    support::replace_document(
        &runtime,
        "split-focus-revision",
        "split-focus",
        split_document(),
    )
    .await;
    wait_until(
        "the revision lands and focus is still on the right half",
        || {
            runtime.model().accepted.as_ref().unwrap().document.revision == revision + 1
                && count(&root, ".m1-matrix-key") == 2
                && dim_label(&root).contains("Left half · linked mirror · click to edit here")
        },
    )
    .await;
    // The tree selection survived the revision on the dimmed half, so the
    // dimmed key correctly shows selection instead of the dim outline.
    // Re-select the focused half on the canvas: the unfocused key must dim.
    canvas_pointer_down(&root, ".m1-matrix-key[data-matrix-id='right']");
    wait_until("the unfocused half dims again after the revision", || {
        runtime.model().selected_part_ids == vec!["matrix/right/r0c0".to_owned()]
            && count(&root, ".m1-matrix-key.is-dimmed") == 1
            && dim_label(&root).contains("Left half · linked mirror · click to edit here")
    })
    .await;
    root.remove();
}
