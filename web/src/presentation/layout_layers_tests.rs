//! Exercise the actual Layout canvas and its layer controls together.
use super::*;
use crate::runtime::project_name_test_support as support;
use boardstudio_core::model::{Board, ProjectDoc};
use wasm_bindgen_test::*;

fn document() -> ProjectDoc {
    let mut document = ProjectDoc::empty("layout-layers", "Layers");
    document.boards.push(Board {
        id: "board".into(),
        name: "Board".into(),
        outline_ids: vec![],
        part_ids: vec!["matrix/matrix/r0c0".into()],
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
    document.parts.push(
        serde_json::from_value(serde_json::json!({
            "id": "matrix/matrix/r0c0", "definitionId": "switch:base", "reference": "SW1",
            "pose": {"at": {"x": 0, "y": 0}, "rotation": 0}, "side": "front"
        }))
        .unwrap(),
    );
    document.matrices.push(
        serde_json::from_value(serde_json::json!({
            "id": "matrix", "rows": 1, "columns": 1,
            "pitch": {"x": 19, "y": 19}, "origin": {"x": 0, "y": 0},
            "definitionId": "switch:base", "partIds": ["matrix/matrix/r0c0"], "boardId": "board",
            "cells": [{"row": 0, "column": 0, "enabled": true, "assemblies": []}]
        }))
        .unwrap(),
    );
    document
}

fn host() -> Element {
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
    rsx! { Editor {} }
}

async fn settle() {
    gloo_timers::future::TimeoutFuture::new(80).await;
}

fn count(root: &web_sys::Element, selector: &str) -> u32 {
    root.query_selector_all(selector).unwrap().length()
}

fn click(root: &web_sys::Element, selector: &str) {
    root.query_selector(selector)
        .unwrap()
        .expect(selector)
        .dyn_into::<HtmlElement>()
        .unwrap()
        .click();
}

#[wasm_bindgen_test]
async fn switch_visibility_keeps_keycaps_independent() {
    let runtime = support::new_runtime();
    support::open_document(&runtime, document()).await;
    let revision = runtime.model().accepted.as_ref().unwrap().document.revision;
    let document = web_sys::window().unwrap().document().unwrap();
    let root = document.create_element("div").unwrap();
    document.body().unwrap().append_child(&root).unwrap();
    let dom = VirtualDom::new(host);
    dom.provide_root_context(runtime.clone());
    dioxus_web::launch::launch_virtual_dom(
        dom,
        dioxus_web::Config::new().rootnode(root.clone().into()),
    );
    settle().await;
    assert_eq!(count(&root, ".m1-keycap-overlay"), 1);
    assert_eq!(count(&root, ".m1-scene-part .m1-part"), 1);
    click(&root, "#m1-layers-trigger");
    settle().await;
    click(&root, "button[aria-label='Hide Switches']");
    settle().await;
    assert_eq!(
        count(&root, ".m1-keycap-overlay"),
        1,
        "hiding switches must preserve visible keycaps"
    );
    assert_eq!(count(&root, ".m1-scene-part .m1-part"), 0);
    assert_eq!(count(&root, "button[aria-label='Show Switches']"), 1);
    click(&root, "button[aria-label='Hide Keycaps']");
    settle().await;
    assert_eq!(count(&root, ".m1-keycap-overlay"), 0);
    assert_eq!(count(&root, ".m1-scene-part"), 0);
    click(&root, "button[aria-label='Show Switches']");
    settle().await;
    assert_eq!(count(&root, ".m1-scene-part .m1-part"), 1);
    assert_eq!(count(&root, ".m1-keycap-overlay"), 0);
    assert_eq!(
        runtime.model().accepted.as_ref().unwrap().document.revision,
        revision
    );
    root.remove();
}
