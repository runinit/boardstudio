//! Exercise the actual Layout canvas and its layer controls together.
use super::*;
use boardstudio_core::model::{Board, ProjectDoc};
use boardstudio_web_test_runtime::project_name_test_support as support;
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
    use_context_provider(|| AppearanceState(theme));
    let resolved = use_memo(|| "light");
    use_context_provider(|| ResolvedAppearance(resolved));
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
    rsx! { super::notifications::NotificationProvider { Editor {} } }
}

use boardstudio_web_ui_shared::{test_role::by_role_name, test_wait::wait_until};

fn count(root: &web_sys::Element, selector: &str) -> u32 {
    root.query_selector_all(selector).unwrap().length()
}

fn click(root: &web_sys::Element, name: &str) {
    by_role_name(root, "button", name)
        .unwrap_or_else(|| panic!("Button {name} is rendered"))
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
    wait_until("the scene shows its keycap and switch", || {
        count(&root, "[data-geometry='keycap']") == 1
            && count(
                &root,
                "[data-geometry='scene-part'] [data-geometry='part-outline']",
            ) == 1
    })
    .await;
    assert_eq!(count(&root, "[data-geometry='keycap']"), 1);
    assert_eq!(
        count(
            &root,
            "[data-geometry='scene-part'] [data-geometry='part-outline']"
        ),
        1
    );
    let layer_trigger = by_role_name(&root, "button", "Canvas visibility").unwrap();
    assert_eq!(
        layer_trigger.get_attribute("aria-expanded").as_deref(),
        Some("false"),
        "Layout layers start collapsed at the persistent bottom edge"
    );
    assert!(
        layer_trigger
            .closest("#m1-objects-panel")
            .unwrap()
            .is_some(),
        "Layout visibility controls stay in the Objects dock"
    );
    assert_eq!(
        count(
            &root,
            "[aria-label='Layout workspace'] [aria-label='Canvas visibility']"
        ),
        0,
        "the viewer must not duplicate the docked visibility controls"
    );
    click(&root, "Canvas visibility");
    wait_until("the layers menu offers Hide Switches", || {
        by_role_name(&root, "button", "Hide Switches").is_some()
    })
    .await;
    click(&root, "Hide Switches");
    wait_until(
        "hiding switches removes the switch and offers Show Switches",
        || {
            count(
                &root,
                "[data-geometry='scene-part'] [data-geometry='part-outline']",
            ) == 0
                && by_role_name(&root, "button", "Show Switches").is_some()
        },
    )
    .await;
    assert_eq!(
        count(&root, "[data-geometry='keycap']"),
        1,
        "hiding switches must preserve visible keycaps"
    );
    assert_eq!(
        count(
            &root,
            "[data-geometry='scene-part'] [data-geometry='part-outline']"
        ),
        0
    );
    assert!(by_role_name(&root, "button", "Show Switches").is_some());
    click(&root, "Hide Keycaps");
    wait_until(
        "hiding keycaps removes the keycaps and the scene part",
        || {
            count(&root, "[data-geometry='keycap']") == 0
                && count(&root, "[data-geometry='scene-part']") == 0
        },
    )
    .await;
    assert_eq!(count(&root, "[data-geometry='keycap']"), 0);
    assert_eq!(count(&root, "[data-geometry='scene-part']"), 0);
    click(&root, "Show Switches");
    wait_until(
        "showing switches restores the switch without keycaps",
        || {
            count(
                &root,
                "[data-geometry='scene-part'] [data-geometry='part-outline']",
            ) == 1
                && count(&root, "[data-geometry='keycap']") == 0
        },
    )
    .await;
    assert_eq!(
        count(
            &root,
            "[data-geometry='scene-part'] [data-geometry='part-outline']"
        ),
        1
    );
    assert_eq!(count(&root, "[data-geometry='keycap']"), 0);
    assert_eq!(
        runtime.model().accepted.as_ref().unwrap().document.revision,
        revision
    );
    root.remove();
}
