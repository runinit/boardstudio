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
    rsx! {
        style { {include_str!("../../assets/workbench-base.css")} }
        style { {include_str!("../../assets/tailwind.css")} }
        style { {include_str!("../../assets/shell.css")} }
        style { {include_str!("../../assets/layout-toolbar.css")} }
        div { style: "display:flex;flex-direction:column;width:1000px;height:720px",
            super::notifications::NotificationProvider { Editor {} }
        }
    }
}

use boardstudio_web_ui_shared::{test_role::by_role_name, test_wait::wait_until};
use boardstudio_web_ui_shared::ui::command::use_command_registry;
use std::{cell::RefCell, rc::Rc};

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
    click(&root, "View");
    wait_until("the View section reveals the layer controls", || {
        root.query_selector("[data-workbench-section='view'][data-open='true']")
            .unwrap()
            .is_some()
            && by_role_name(&root, "button", "Canvas visibility").is_some()
    })
    .await;
    let layer_trigger = by_role_name(&root, "button", "Canvas visibility").unwrap();
    assert_eq!(
        layer_trigger.get_attribute("aria-expanded").as_deref(),
        Some("false"),
        "Layout layers start collapsed in View"
    );
    assert!(
        layer_trigger
            .closest("[data-workbench-section='view']")
            .unwrap()
            .is_some(),
        "Layout visibility controls stay with the View section"
    );
    assert_eq!(
        count(&root, "button[aria-label='Canvas visibility']"),
        1,
        "the View visibility controls are not duplicated"
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

fn finding_row_containing(root: &web_sys::Element, phrase: &str) -> Option<web_sys::Element> {
    let panel = root
        .query_selector("[aria-label='Layout findings']")
        .unwrap()?;
    let rows = panel.query_selector_all("li").unwrap();
    (0..rows.length())
        .filter_map(|index| rows.item(index))
        .filter_map(|row| row.dyn_into::<web_sys::Element>().ok())
        .find(|row| row.text_content().unwrap_or_default().contains(phrase))
}

#[wasm_bindgen_test]
async fn paired_rejoin_preview_findings_reach_editor_footer_panel_and_clear_before_commit() {
    let mut fixture: serde_json::Value = serde_json::from_str(include_str!(
        "layout_layers_fixtures/paired-rejoin-preview.json"
    ))
    .unwrap();
    fixture["document"]["formatVersion"] = boardstudio_core::migrate::CURRENT_VERSION.into();
    // The captured public fixture has two unrelated source diagnostics. Complete the reset
    // switch definition with its real pad/keycap geometry so this Preview is Info-only.
    let reset = fixture["document"]["definitions"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|definition| definition["id"] == "synthetic/reset")
        .expect("captured public fixture contains the reset definition");
    reset["keycap"] = serde_json::json!({"x": 6.0, "y": 6.0});
    reset["pads"] = serde_json::json!([{
        "id": "pad-1",
        "number": "1",
        "at": {"x": 0.0, "y": 0.0},
        "size": {"x": 1.0, "y": 1.0},
        "shape": "circle"
    }]);

    let document: ProjectDoc = serde_json::from_value(fixture["document"].clone()).unwrap();
    let operation: boardstudio_core::model::EditOperation =
        serde_json::from_value(fixture["operation"].clone()).unwrap();
    let target_ids: Vec<String> = fixture["targetIds"]
        .as_array()
        .unwrap()
        .iter()
        .map(|id| id.as_str().unwrap().to_owned())
        .collect();
    let mirror_net_id = fixture["netId"].as_str().unwrap().to_owned();

    let runtime = support::new_runtime();
    support::open_document(&runtime, document).await;
    support::navigate(&runtime, "right").await;
    let accepted = runtime.model().accepted.clone().unwrap();
    let accepted_document = accepted.document.clone();
    let accepted_revision = accepted_document.revision;
    assert!(accepted.scene.findings.iter().all(|finding| !matches!(
        finding.severity,
        boardstudio_core::model::Severity::Error | boardstudio_core::model::Severity::Warning
    )));
    let accepted_findings_count =
        keycaps_fit::presented_findings(&accepted.scene.findings, &accepted_document).len();
    assert_eq!(accepted_findings_count, 0, "the accepted scene is clean");

    let dom_document = web_sys::window().unwrap().document().unwrap();
    let root = dom_document.create_element("div").unwrap();
    dom_document.body().unwrap().append_child(&root).unwrap();
    let dom = VirtualDom::new(host);
    dom.provide_root_context(runtime.clone());
    dioxus_web::launch::launch_virtual_dom(
        dom,
        dioxus_web::Config::new().rootnode(root.clone().into()),
    );
    wait_until("the Editor shows the paired right board", || {
        count(
            &root,
            "[data-geometry='scene-part'] [data-geometry='part-outline']",
        ) > 0
    })
    .await;
    assert!(
        root.text_content()
            .unwrap_or_default()
            .contains("Layout findings: 0")
    );

    let transaction_id = "ticket160-browser-rejoin-preview";
    runtime.submit(Event::PreviewEdit {
        operation_id: runtime.operation(),
        transaction_id: transaction_id.into(),
        target_ids: target_ids.clone(),
        operation: operation.clone(),
    });
    support::run_pending(&runtime).await;
    wait_until("Core Preview reaches the active Editor scene", || {
        runtime
            .model()
            .display_preview
            .as_ref()
            .is_some_and(|scene| {
                scene
                    .findings
                    .iter()
                    .any(|finding| finding.id.starts_with("preview:electrical:pin:"))
            })
    })
    .await;

    let preview = runtime.model().display_preview.clone().unwrap();
    assert!(
        preview
            .findings
            .iter()
            .all(|finding| { finding.severity == boardstudio_core::model::Severity::Info })
    );
    assert_eq!(
        preview
            .findings
            .iter()
            .filter(|finding| finding.id.starts_with("preview:electrical:"))
            .count(),
        3,
        "the paired Rejoin Preview carries pin, net, and registration notices"
    );
    let preview_findings_count =
        keycaps_fit::presented_findings(&preview.findings, &accepted_document).len();
    assert_eq!(preview_findings_count, 3, "the active Preview is Info-only");
    assert_eq!(preview_findings_count, accepted_findings_count + 3);
    assert_eq!(
        runtime.model().accepted.as_ref().unwrap().document,
        accepted_document,
        "Preview leaves the accepted document untouched"
    );
    assert_eq!(
        runtime.model().accepted.as_ref().unwrap().document.revision,
        accepted_revision,
        "Preview leaves the accepted revision untouched"
    );

    let pin_notice = format!(
        "Preview removes pad pad-1 on right-U1 (right/U1) from net OMIT_CONTROLLER ({mirror_net_id})."
    );
    let net_notice = format!("Preview removes net OMIT_CONTROLLER ({mirror_net_id}).");
    let registration_notice = format!(
        "Preview removes net OMIT_CONTROLLER ({mirror_net_id}) from board Synthetic left (mirror) (right)."
    );
    click(
        &root,
        &format!("Layout findings: {}", accepted_findings_count + 3),
    );
    wait_until("the footer opens the Info-only findings panel", || {
        root.query_selector("[aria-label='Layout findings']")
            .unwrap()
            .is_some()
            && root
                .text_content()
                .unwrap_or_default()
                .contains("Layout findings: 3")
            && finding_row_containing(&root, &pin_notice).is_some()
    })
    .await;
    let finding = finding_row_containing(&root, &pin_notice).unwrap();
    finding.remove_attribute("class").unwrap();
    assert!(
        finding_row_containing(&root, &pin_notice).is_some(),
        "finding lookup does not depend on styling classes"
    );
    let text = root.text_content().unwrap_or_default();
    assert!(text.contains(&pin_notice));
    assert!(text.contains(&net_notice));
    assert!(text.contains(&registration_notice));
    let pin_row = finding_row_containing(&root, &pin_notice).expect("pin notice row is rendered");
    assert!(
        by_role_name(&pin_row, "button", "Select affected geometry").is_none(),
        "the preview electrical notice has no navigation action"
    );

    runtime.submit(Event::ClearPreview {
        operation_id: runtime.operation(),
        token: accepted.token,
        revision: accepted_revision,
        board_id: "right".into(),
        transaction_id: transaction_id.into(),
    });
    support::run_pending(&runtime).await;
    assert!(
        runtime.model().display_preview.is_none(),
        "ClearPreview releases the Session preview before rendering"
    );
    wait_until(
        "clearing Preview removes its rows and restores the empty count",
        || {
            runtime.model().display_preview.is_none()
                && finding_row_containing(&root, &pin_notice).is_none()
                && root
                    .text_content()
                    .unwrap_or_default()
                    .contains("Layout findings: 0")
        },
    )
    .await;
    assert!(runtime.model().display_preview.is_none());
    assert!(finding_row_containing(&root, &net_notice).is_none());
    assert!(finding_row_containing(&root, &registration_notice).is_none());

    runtime.submit(Event::PreviewEdit {
        operation_id: runtime.operation(),
        transaction_id: "ticket160-browser-cancel".into(),
        target_ids,
        operation,
    });
    support::run_pending(&runtime).await;
    wait_until(
        "a second Preview displays the loss notices before cancellation",
        || {
            finding_row_containing(&root, &pin_notice).is_some()
                && finding_row_containing(&root, &net_notice).is_some()
                && finding_row_containing(&root, &registration_notice).is_some()
        },
    )
    .await;
    let controller = accepted_document
        .parts
        .iter()
        .find(|part| part.id == "right/U1")
        .unwrap();
    runtime.submit(Event::GestureBegin {
        operation_id: runtime.operation(),
        pointer_id: 160,
        target_ids: vec![controller.id.clone()],
        transaction_id: "ticket160-browser-cancel".into(),
        start: vec![boardstudio_core::model::Position {
            id: controller.id.clone(),
            at: controller.pose.at,
        }],
        pitch: boardstudio_core::model::Vec2 { x: 19.0, y: 19.0 },
        snap_fraction: 0.0,
        geometry_snap: false,
        gap: None,
        pixels_per_world: None,
        alt: true,
    });
    assert!(runtime.model().display_preview.is_some());
    runtime.submit(Event::GestureCancel { pointer_id: 160 });
    support::run_pending(&runtime).await;
    wait_until("gesture cancellation removes every Preview notice", || {
        runtime.model().display_preview.is_none()
            && finding_row_containing(&root, &pin_notice).is_none()
            && finding_row_containing(&root, &net_notice).is_none()
            && finding_row_containing(&root, &registration_notice).is_none()
            && root
                .text_content()
                .unwrap_or_default()
                .contains("Layout findings: 0")
    })
    .await;
    assert_eq!(
        runtime.model().accepted.as_ref().unwrap().document,
        accepted_document
    );
    root.remove();
}

#[wasm_bindgen_test]
async fn layout_panels_overlay_the_same_canvas_without_a_legacy_dock() {
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
    wait_until("the Layout canvas and its panel utilities mount", || {
        root.query_selector("#m1-workspace-panel svg[role='group']").unwrap().is_some()
            && by_role_name(&root, "button", "Objects").is_some()
            && by_role_name(&root, "button", "Inspect").is_some()
    })
    .await;
    let canvas = root
        .query_selector("#m1-workspace-panel svg[role='group']")
        .unwrap()
        .unwrap();
    let rect = canvas.get_bounding_client_rect();
    let before = (rect.x(), rect.y(), rect.width(), rect.height());
    assert!(before.2 > 0.0 && before.3 > 0.0, "production CSS gives the canvas bounds");
    let view_box = canvas.get_attribute("viewBox");
    assert_eq!(count(&root, "#m1-objects-panel, #m1-inspector-panel"), 0);
    let assert_canvas_unchanged = |root: &web_sys::Element| {
        let current = root
            .query_selector("#m1-workspace-panel svg[role='group']")
            .unwrap()
            .unwrap();
        let rect = current.get_bounding_client_rect();
        assert!(canvas.is_same_node(Some(current.as_ref())), "panel visibility must not remount the canvas");
        assert_eq!((rect.x(), rect.y(), rect.width(), rect.height()), before);
        assert_eq!(current.get_attribute("viewBox"), view_box);
        assert_eq!(runtime.model().accepted.as_ref().unwrap().document.revision, revision);
    };
    click(&root, "Objects");
    wait_until("actual Layout Objects content opens", || {
        root.query_selector("#workbench-objects:not([hidden]) #m1-add-object-trigger")
            .unwrap()
            .is_some()
    })
    .await;
    assert_canvas_unchanged(&root);
    click(&root, "Inspect");
    wait_until("Inspect replaces Objects", || {
        root.query_selector("#workbench-inspector:not([hidden])").unwrap().is_some()
            && root.query_selector("#workbench-objects[hidden][inert]").unwrap().is_some()
    })
    .await;
    assert_canvas_unchanged(&root);
    click(&root, "Inspect");
    wait_until("both Layout panels close", || {
        root.query_selector("#workbench-inspector[hidden][inert]").unwrap().is_some()
    })
    .await;
    assert_canvas_unchanged(&root);
    root.remove();
}

#[wasm_bindgen_test]
async fn actual_layout_tool_row_has_seven_entry_points_and_stable_canvas() {
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
    wait_until("the Layout canvas and its panel utilities mount", || {
        root.query_selector("#m1-workspace-panel svg[role='group']").unwrap().is_some()
            && by_role_name(&root, "button", "Objects").is_some()
            && by_role_name(&root, "button", "Inspect").is_some()
    })
    .await;
    web_sys::window().unwrap().scroll_to_with_x_and_y(0.0, 0.0);
    assert_eq!(count(&root, ".workbench-tool-row"), 1, "one bounded tool row");
    let row = root.query_selector(".workbench-tool-row").unwrap().unwrap();
    let height = row.get_bounding_client_rect().height();
    assert!(
        (height - 42.0).abs() < 0.5,
        "row stays 42px high: {height}"
    );
    let window = web_sys::window().unwrap();
    assert_eq!(count(&root, "#workbench-objects:not([hidden])"), 0);
    assert_eq!(count(&root, "#workbench-inspector:not([hidden])"), 0);
    for name in ["Select", "Transform", "Align", "Snap", "View", "Objects", "Inspect"] {
        let button = by_role_name(&root, "button", name)
            .unwrap_or_else(|| panic!("{name} entry is rendered"));
        let rect = button.get_bounding_client_rect();
        assert!(
            rect.width() > 0.0 && rect.height() > 0.0,
            "{name} has bounds"
        );
        assert_eq!(
            count(&root, &format!("button[aria-label=\"{name}\"]")),
            1,
            "{name} has no duplicate entry"
        );
        assert!(button.is_connected(), "{name} is mounted");
        assert!(
            button.closest("[hidden]").unwrap().is_none(),
            "{name} has no hidden ancestor"
        );
        assert!(
            button.closest("[inert]").unwrap().is_none(),
            "{name} has no inert ancestor"
        );
        assert!(!button.has_attribute("disabled"), "{name} is enabled");
        let pointer_events = window
            .get_computed_style(&button)
            .unwrap()
            .unwrap()
            .get_property_value("pointer-events")
            .unwrap();
        assert_ne!(pointer_events, "none", "{name} accepts pointer hits");
    }
    let canvas = root
        .query_selector("#m1-workspace-panel svg[role='group']")
        .unwrap()
        .unwrap();
    let rect = canvas.get_bounding_client_rect();
    let before = (rect.x(), rect.y(), rect.width(), rect.height());
    assert!(before.2 > 0.0 && before.3 > 0.0, "production CSS gives the canvas bounds");
    let view_box = canvas.get_attribute("viewBox");
    let assert_canvas_unchanged = |root: &web_sys::Element| {
        let current = root
            .query_selector("#m1-workspace-panel svg[role='group']")
            .unwrap()
            .unwrap();
        let rect = current.get_bounding_client_rect();
        assert!(canvas.is_same_node(Some(current.as_ref())), "panel/section visibility must not remount the canvas");
        assert_eq!((rect.x(), rect.y(), rect.width(), rect.height()), before);
        assert_eq!(current.get_attribute("viewBox"), view_box);
        assert_eq!(runtime.model().accepted.as_ref().unwrap().document.revision, revision);
    };
    for (name, key) in [
        ("Select", "select"),
        ("Transform", "transform"),
        ("Align", "align"),
        ("Snap", "snap"),
        ("View", "view"),
    ] {
        click(&root, name);
        let what = format!("{name} section opens alone");
        wait_until(&what, || {
            root.query_selector(&format!("[data-workbench-section='{key}'][data-open='true']"))
                .unwrap()
                .is_some()
                && count(&root, "[data-workbench-section][data-open='true']") == 1
        })
        .await;
        assert_canvas_unchanged(&root);
    }
    click(&root, "Objects");
    wait_until("Objects opens", || {
        root.query_selector("#workbench-objects:not([hidden])")
            .unwrap()
            .is_some()
    })
    .await;
    assert_canvas_unchanged(&root);
    click(&root, "Inspect");
    wait_until("Inspect replaces Objects", || {
        root.query_selector("#workbench-inspector:not([hidden])").unwrap().is_some()
            && root.query_selector("#workbench-objects[hidden][inert]").unwrap().is_some()
    })
    .await;
    assert_canvas_unchanged(&root);
    click(&root, "Inspect");
    wait_until("both Layout panels close", || {
        root.query_selector("#workbench-inspector[hidden][inert]").unwrap().is_some()
    })
    .await;
    assert_canvas_unchanged(&root);
    root.remove();
}

#[wasm_bindgen_test]
async fn view_mode_and_visibility_toggles_keep_pressed_state_and_revision() {
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
    wait_until("the Layout canvas and its panel utilities mount", || {
        root.query_selector("#m1-workspace-panel svg[role='group']")
            .unwrap()
            .is_some()
            && by_role_name(&root, "button", "Objects").is_some()
            && by_role_name(&root, "button", "Inspect").is_some()
    })
    .await;
    click(&root, "View");
    wait_until("the View section reveals the view controls", || {
        root.query_selector("[data-workbench-section='view'][data-open='true']")
            .unwrap()
            .is_some()
            && by_role_name(&root, "group", "Design view").is_some()
            && by_role_name(&root, "button", "2D").is_some()
            && by_role_name(&root, "button", "3D assembly").is_some()
            && by_role_name(&root, "button", "Footprints").is_some()
            && by_role_name(&root, "button", "Canvas handles").is_some()
            && by_role_name(&root, "button", "Canvas visibility").is_some()
    })
    .await;
    let pressed_of = |name: &str| -> (String, Option<String>) {
        let button = by_role_name(&root, "button", name)
            .unwrap_or_else(|| panic!("{name} is rendered"));
        assert!(
            !button.has_attribute("disabled"),
            "{name} stays enabled for the view-only toggle"
        );
        (
            button.get_attribute("aria-pressed").unwrap_or_default(),
            button.get_attribute("data-state"),
        )
    };
    let (pressed, state) = pressed_of("2D");
    assert_eq!(pressed, "true", "2D starts pressed");
    assert_eq!(state.as_deref(), Some("on"), "2D pressed uses data-state on");
    assert_eq!(pressed_of("3D assembly").0, "false", "3D starts unpressed");
    assert_eq!(
        pressed_of("Footprints").0,
        "false",
        "Footprints starts unpressed"
    );
    let (pressed, state) = pressed_of("Canvas handles");
    assert_eq!(pressed, "true", "handles start pressed");
    assert_eq!(
        state.as_deref(),
        Some("on"),
        "handles pressed uses data-state on"
    );
    assert_eq!(
        runtime.model().accepted.as_ref().unwrap().document.revision,
        revision
    );
    click(&root, "3D assembly");
    wait_until("3D assembly mode presses 3D and hides View-only controls", || {
        by_role_name(&root, "button", "3D assembly").is_some_and(|button| {
            button.get_attribute("aria-pressed").as_deref() == Some("true")
        }) && by_role_name(&root, "button", "2D").is_some_and(|button| {
            button.get_attribute("aria-pressed").as_deref() == Some("false")
        }) && by_role_name(&root, "button", "Footprints").is_none()
            && by_role_name(&root, "button", "Canvas handles").is_none()
            && by_role_name(&root, "button", "Canvas visibility").is_none()
    })
    .await;
    assert_eq!(
        count(&root, ".workbench-tool-row"),
        1,
        "3D keeps one bounded row"
    );
    assert_eq!(pressed_of("3D assembly").0, "true");
    assert_eq!(pressed_of("2D").0, "false");
    assert_eq!(
        runtime.model().accepted.as_ref().unwrap().document.revision,
        revision
    );
    click(&root, "2D");
    wait_until("2D mode restores View controls", || {
        by_role_name(&root, "button", "2D").is_some_and(|button| {
            button.get_attribute("aria-pressed").as_deref() == Some("true")
        }) && by_role_name(&root, "button", "Footprints").is_some()
            && by_role_name(&root, "button", "Canvas handles").is_some()
            && by_role_name(&root, "button", "Canvas visibility").is_some()
    })
    .await;
    let (pressed, state) = pressed_of("2D");
    assert_eq!(pressed, "true");
    assert_eq!(state.as_deref(), Some("on"));
    assert_eq!(pressed_of("3D assembly").0, "false");
    assert_eq!(
        runtime.model().accepted.as_ref().unwrap().document.revision,
        revision
    );
    if root
        .query_selector("[data-workbench-section='view'][data-open='true']")
        .unwrap()
        .is_none()
    {
        click(&root, "View");
    }
    wait_until("View section is open for visibility toggles", || {
        root.query_selector("[data-workbench-section='view'][data-open='true']")
            .unwrap()
            .is_some()
    })
    .await;
    click(&root, "Footprints");
    wait_until("Footprints presses without moving 2D or handles", || {
        by_role_name(&root, "button", "Footprints").is_some_and(|button| {
            button.get_attribute("aria-pressed").as_deref() == Some("true")
        })
    })
    .await;
    let (pressed, state) = pressed_of("Footprints");
    assert_eq!(pressed, "true");
    assert_eq!(state.as_deref(), Some("on"));
    assert_eq!(
        pressed_of("2D").0,
        "true",
        "Footprints toggle keeps 2D pressed"
    );
    assert_eq!(
        pressed_of("Canvas handles").0,
        "true",
        "Footprints toggle keeps handles pressed"
    );
    assert_eq!(
        runtime.model().accepted.as_ref().unwrap().document.revision,
        revision
    );
    click(&root, "Canvas handles");
    wait_until("handles unpress without moving 2D or Footprints", || {
        by_role_name(&root, "button", "Canvas handles").is_some_and(|button| {
            button.get_attribute("aria-pressed").as_deref() == Some("false")
        })
    })
    .await;
    assert_eq!(pressed_of("Canvas handles").0, "false");
    assert_eq!(
        pressed_of("2D").0,
        "true",
        "handles toggle keeps 2D pressed"
    );
    assert_eq!(
        pressed_of("Footprints").0,
        "true",
        "handles toggle keeps Footprints pressed"
    );
    assert_eq!(
        runtime.model().accepted.as_ref().unwrap().document.revision,
        revision
    );
    root.remove();
}

struct LegacyPanelStorageGuard {
    left: Option<String>,
    right: Option<String>,
}

impl LegacyPanelStorageGuard {
    fn seed_pinned_extremes() -> Self {
        let storage = web_sys::window()
            .unwrap()
            .local_storage()
            .unwrap()
            .unwrap();
        let left = storage.get_item("boardstudio:v2:panel:left").unwrap();
        let right = storage.get_item("boardstudio:v2:panel:right").unwrap();
        storage
            .set_item("boardstudio:v2:panel:left", r#"{"mode":"pinned","width":420}"#)
            .unwrap();
        storage
            .set_item(
                "boardstudio:v2:panel:right",
                r#"{"mode":"pinned","width":480}"#,
            )
            .unwrap();
        Self { left, right }
    }

    fn restore(key: &str, previous: &Option<String>) {
        let storage = web_sys::window()
            .unwrap()
            .local_storage()
            .unwrap()
            .unwrap();
        match previous {
            Some(value) => storage.set_item(key, value).unwrap(),
            None => storage.remove_item(key).unwrap(),
        }
    }
}

impl Drop for LegacyPanelStorageGuard {
    fn drop(&mut self) {
        Self::restore("boardstudio:v2:panel:left", &self.left);
        Self::restore("boardstudio:v2:panel:right", &self.right);
    }
}

#[derive(Clone, Copy)]
struct CommandsHandles {
    workspace: Signal<&'static str>,
    compact: CompactPanelState,
}

fn commands_host() -> Element {
    let capture = use_context::<Rc<RefCell<Option<CommandsHandles>>>>();
    let registry = use_command_registry();
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
    let compact = CompactPanelState {
        objects_open,
        inspector_open,
    };
    use_context_provider(|| compact);
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
    *capture.borrow_mut() = Some(CommandsHandles { workspace, compact });
    let view_commands = registry.commands();
    let objects_command = view_commands
        .iter()
        .find(|command| command.group == "View" && command.label == "Show/hide Objects")
        .cloned();
    let inspector_command = view_commands
        .iter()
        .find(|command| command.group == "View" && command.label == "Show/hide Inspector")
        .cloned();
    let status = format!(
        "{}{}",
        objects_command.is_some() as u8,
        inspector_command.is_some() as u8
    );
    rsx! {
        style { {include_str!("../../assets/workbench-base.css")} }
        style { {include_str!("../../assets/tailwind.css")} }
        style { {include_str!("../../assets/shell.css")} }
        style { {include_str!("../../assets/layout-toolbar.css")} }
        div { style: "display:flex;flex-direction:column;width:1000px;height:720px",
            super::notifications::NotificationProvider { Editor {} }
            input { id: "caller-retained-input", aria_label: "Caller-owned retained input" }
            output { id: "view-commands-status", aria_label: "Registered View commands", "{status}" }
            button {
                id: "invoke-show-hide-objects",
                aria_label: "Invoke Show/hide Objects",
                disabled: objects_command.is_none(),
                onclick: move |_| {
                    if let Some(command) = objects_command.clone() {
                        assert!(command.reason.is_none(), "Show/hide Objects stays enabled");
                        command.on_select.call(());
                    }
                },
                "Invoke Show/hide Objects"
            }
            button {
                id: "invoke-show-hide-inspector",
                aria_label: "Invoke Show/hide Inspector",
                disabled: inspector_command.is_none(),
                onclick: move |_| {
                    if let Some(command) = inspector_command.clone() {
                        assert!(command.reason.is_none(), "Show/hide Inspector stays enabled");
                        command.on_select.call(());
                    }
                },
                "Invoke Show/hide Inspector"
            }
        }
    }
}

fn invoke_view_command(root: &web_sys::Element, label: &str) {
    click(root, label);
}

#[wasm_bindgen_test]
async fn show_hide_commands_route_to_each_workspace_current_host() {
    let _storage = LegacyPanelStorageGuard::seed_pinned_extremes();
    let runtime = support::new_runtime();
    support::open_document(&runtime, document()).await;
    let revision = runtime.model().accepted.as_ref().unwrap().document.revision;
    let window = web_sys::window().unwrap();
    let document = window.document().unwrap();
    let root = document.create_element("div").unwrap();
    document.body().unwrap().append_child(&root).unwrap();
    let capture = Rc::new(RefCell::new(None::<CommandsHandles>));
    let dom = VirtualDom::new(commands_host);
    dom.provide_root_context(runtime.clone());
    dom.provide_root_context(capture.clone());
    dioxus_web::launch::launch_virtual_dom(
        dom,
        dioxus_web::Config::new().rootnode(root.clone().into()),
    );
    wait_until("the Layout canvas and its panel utilities mount", || {
        root.query_selector("#m1-workspace-panel svg[role='group']")
            .unwrap()
            .is_some()
            && by_role_name(&root, "button", "Objects").is_some()
            && by_role_name(&root, "button", "Inspect").is_some()
    })
    .await;
    wait_until("the View commands register", || {
        root.query_selector("#view-commands-status")
            .unwrap()
            .is_some_and(|status| status.text_content().as_deref() == Some("11"))
    })
    .await;
    let mut handles = capture
        .borrow()
        .as_ref()
        .copied()
        .expect("commands host captured");
    let canvas = root
        .query_selector("#m1-workspace-panel svg[role='group']")
        .unwrap()
        .unwrap();
    let rect = canvas.get_bounding_client_rect();
    let before = (rect.x(), rect.y(), rect.width(), rect.height());
    let view_box = canvas.get_attribute("viewBox");
    assert_eq!(count(&root, "#m1-objects-panel, #m1-inspector-panel"), 0);
    let compact_signals = || {
        let borrowed = capture.borrow();
        let handles = borrowed.as_ref().unwrap();
        (
            *handles.compact.objects_open.peek(),
            *handles.compact.inspector_open.peek(),
        )
    };
    let assert_canvas_unchanged = |root: &web_sys::Element| {
        let current = root
            .query_selector("#m1-workspace-panel svg[role='group']")
            .unwrap()
            .unwrap();
        let rect = current.get_bounding_client_rect();
        assert!(
            canvas.is_same_node(Some(current.as_ref())),
            "commands must not remount the canvas"
        );
        assert_eq!((rect.x(), rect.y(), rect.width(), rect.height()), before);
        assert_eq!(current.get_attribute("viewBox"), view_box);
        assert_eq!(
            runtime.model().accepted.as_ref().unwrap().document.revision,
            revision
        );
    };
    let legacy_before = compact_signals();
    let invoke_objects = by_role_name(&root, "button", "Invoke Show/hide Objects").unwrap();
    invoke_objects.focus().unwrap();
    invoke_view_command(&root, "Invoke Show/hide Objects");
    wait_until("Show/hide Objects opens the Layout panel", || {
        by_role_name(&root, "button", "Objects").is_some_and(|button| {
            button.get_attribute("aria-expanded").as_deref() == Some("true")
        })
    })
    .await;
    let panel = root
        .query_selector("#workbench-objects:not([hidden])")
        .unwrap()
        .expect("Objects panel visible");
    let width = panel.get_bounding_client_rect().width();
    assert!(
        (width - 320.0).abs() < 2.0,
        "Layout Objects ignores the seeded 420px legacy width: {width}"
    );
    assert_eq!(
        count(&root, "#m1-objects-panel, #m1-inspector-panel"),
        0,
        "Layout renders no legacy dock"
    );
    assert_eq!(
        compact_signals(),
        legacy_before,
        "Layout command leaves legacy compact state alone"
    );
    assert_eq!(
        document.active_element().as_ref(),
        Some(invoke_objects.as_ref()),
        "command-opened Objects keeps focus when content has no search target"
    );
    assert_canvas_unchanged(&root);
    let close_objects = by_role_name(&root, "button", "Close Objects").unwrap();
    close_objects.focus().unwrap();
    assert_eq!(
        document.active_element().as_ref(),
        Some(close_objects.as_ref()),
        "precondition: focus moves inside the open panel"
    );
    invoke_view_command(&root, "Invoke Show/hide Objects");
    wait_until("Show/hide Objects closes the Layout panel", || {
        by_role_name(&root, "button", "Objects").is_some_and(|button| {
            button.get_attribute("aria-expanded").as_deref() == Some("false")
        })
    })
    .await;
    let objects = by_role_name(&root, "button", "Objects").unwrap();
    assert_eq!(
        document.active_element().as_ref(),
        Some(objects.as_ref()),
        "command close returns focus to its utility"
    );
    assert!(
        root.query_selector("#workbench-objects[hidden][inert]")
            .unwrap()
            .is_some()
    );
    assert_canvas_unchanged(&root);
    invoke_view_command(&root, "Invoke Show/hide Inspector");
    wait_until("Show/hide Inspector opens the Layout panel", || {
        by_role_name(&root, "button", "Inspect").is_some_and(|button| {
            button.get_attribute("aria-expanded").as_deref() == Some("true")
        })
    })
    .await;
    let panel = root
        .query_selector("#workbench-inspector:not([hidden])")
        .unwrap()
        .expect("Inspector panel visible");
    let width = panel.get_bounding_client_rect().width();
    assert!(
        (width - 400.0).abs() < 2.0,
        "Layout Inspector ignores the seeded 480px legacy width: {width}"
    );
    assert_eq!(
        count(&root, "#m1-objects-panel, #m1-inspector-panel"),
        0,
        "Layout renders no legacy dock"
    );
    assert_eq!(
        compact_signals(),
        legacy_before,
        "Layout command leaves legacy compact state alone"
    );
    let heading = root
        .query_selector("#workbench-inspector h2")
        .unwrap()
        .unwrap();
    assert_eq!(
        document.active_element().as_ref(),
        Some(heading.as_ref()),
        "command-opened Inspector focuses its heading"
    );
    assert_canvas_unchanged(&root);
    invoke_view_command(&root, "Invoke Show/hide Inspector");
    wait_until("Show/hide Inspector closes the Layout panel", || {
        by_role_name(&root, "button", "Inspect").is_some_and(|button| {
            button.get_attribute("aria-expanded").as_deref() == Some("false")
        })
    })
    .await;
    let inspect = by_role_name(&root, "button", "Inspect").unwrap();
    assert_eq!(
        document.active_element().as_ref(),
        Some(inspect.as_ref()),
        "command close returns focus to its utility"
    );
    let desktop = !super::is_compact_viewport();
    for workspace in ["Case", "PCB"] {
        handles.workspace.set(workspace);
        let label = format!("{workspace} mounts its legacy host without a workbench panel");
        wait_until(&label, || {
                root.query_selector("#m1-objects-panel").unwrap().is_some()
                    && root.query_selector("#workbench-objects").unwrap().is_none()
                    && root.query_selector("#workbench-inspector").unwrap().is_none()
        })
        .await;
        let label = format!("{workspace} View commands register");
        wait_until(&label, || {
            root.query_selector("#view-commands-status")
                .unwrap()
                .is_some_and(|status| status.text_content().as_deref() == Some("11"))
        })
        .await;
        assert_eq!(
            count(
                &root,
                "#workbench-objects:not([hidden]), #workbench-inspector:not([hidden])"
            ),
            0,
            "{workspace} shows no ghost Layout panel"
        );
        if desktop {
            let mode_of = |selector: &str| {
                root.query_selector(selector)
                    .unwrap()
                    .unwrap()
                    .get_attribute("data-mode")
            };
            assert_eq!(
                mode_of("#m1-objects-panel").as_deref(),
                Some("pinned"),
                "{workspace} starts from the seeded legacy mode"
            );
            let dock = root.query_selector("#m1-objects-panel").unwrap().unwrap();
            let dock_width = dock.get_bounding_client_rect().width();
            assert!(
                (dock_width - 420.0).abs() < 3.0,
                "{workspace} legacy dock honors the seeded 420px width: {dock_width}"
            );
            invoke_view_command(&root, "Invoke Show/hide Objects");
            let label = format!("{workspace} Objects collapses its legacy dock");
            wait_until(&label, || {
                mode_of("#m1-objects-panel").as_deref() == Some("collapsed")
            })
            .await;
            assert_eq!(
                count(
                    &root,
                    "#workbench-objects:not([hidden]), #workbench-inspector:not([hidden])"
                ),
                0,
                "{workspace} command opens no ghost Layout panel"
            );
            invoke_view_command(&root, "Invoke Show/hide Objects");
            let label = format!("{workspace} Objects restores its legacy dock");
            wait_until(&label, || {
                mode_of("#m1-objects-panel").as_deref() == Some("pinned")
            })
            .await;
            invoke_view_command(&root, "Invoke Show/hide Inspector");
            let label = format!("{workspace} Inspector collapses its legacy dock");
            wait_until(
                &label,
                || mode_of("#m1-inspector-panel").as_deref() == Some("collapsed"),
            )
            .await;
            assert_eq!(
                count(
                    &root,
                    "#workbench-objects:not([hidden]), #workbench-inspector:not([hidden])"
                ),
                0,
                "{workspace} command opens no ghost Layout panel"
            );
            invoke_view_command(&root, "Invoke Show/hide Inspector");
            let label = format!("{workspace} Inspector restores its legacy dock");
            wait_until(
                &label,
                || mode_of("#m1-inspector-panel").as_deref() == Some("pinned"),
            )
            .await;
        } else {
            let (objects_open, _) = compact_signals();
            invoke_view_command(&root, "Invoke Show/hide Objects");
            let label = format!("{workspace} Objects answers on its compact state");
            wait_until(&label, || compact_signals() == (!objects_open, false))
            .await;
            assert_eq!(
                count(
                    &root,
                    "#workbench-objects:not([hidden]), #workbench-inspector:not([hidden])"
                ),
                0,
                "{workspace} command opens no ghost Layout panel"
            );
            let (_, inspector_open) = compact_signals();
            invoke_view_command(&root, "Invoke Show/hide Inspector");
            let label = format!("{workspace} Inspector answers on its compact state");
            wait_until(
                &label,
                || compact_signals() == (false, !inspector_open),
            )
            .await;
            assert_eq!(
                count(
                    &root,
                    "#workbench-objects:not([hidden]), #workbench-inspector:not([hidden])"
                ),
                0,
                "{workspace} command opens no ghost Layout panel"
            );
        }
    }
    handles.workspace.set("Layout");
    wait_until("Layout utilities mount again", || {
        by_role_name(&root, "button", "Objects").is_some()
            && by_role_name(&root, "button", "Inspect").is_some()
    })
    .await;
    assert_eq!(
        by_role_name(&root, "button", "Objects")
            .unwrap()
            .get_attribute("aria-expanded")
            .as_deref(),
        Some("false"),
        "returning to Layout reopens no panel"
    );
    assert_eq!(
        by_role_name(&root, "button", "Inspect")
            .unwrap()
            .get_attribute("aria-expanded")
            .as_deref(),
        Some("false"),
        "returning to Layout reopens no panel"
    );
    assert_eq!(
        runtime.model().accepted.as_ref().unwrap().document.revision,
        revision
    );
    root.remove();
}

async fn settle_frames() {
    for _ in 0..2 {
        let promise = js_sys::Promise::new(&mut |resolve, _| {
            web_sys::window()
                .unwrap()
                .request_animation_frame(&resolve)
                .unwrap();
        });
        let _ = wasm_bindgen_futures::JsFuture::from(promise).await;
    }
}

fn mount_commands_host(
    runtime: &std::rc::Rc<Runtime>,
    root: &web_sys::Element,
) -> Rc<RefCell<Option<CommandsHandles>>> {
    let capture = Rc::new(RefCell::new(None::<CommandsHandles>));
    let dom = VirtualDom::new(commands_host);
    dom.provide_root_context(runtime.clone());
    dom.provide_root_context(capture.clone());
    dioxus_web::launch::launch_virtual_dom(
        dom,
        dioxus_web::Config::new().rootnode(root.clone().into()),
    );
    capture
}

async fn wait_for_commands_host(root: &web_sys::Element) {
    wait_until("the Layout canvas and its panel utilities mount", || {
        root.query_selector("#m1-workspace-panel svg[role='group']")
            .unwrap()
            .is_some()
            && by_role_name(&root, "button", "Objects").is_some()
            && by_role_name(&root, "button", "Inspect").is_some()
    })
    .await;
    wait_until("the View commands register", || {
        root.query_selector("#view-commands-status")
            .unwrap()
            .is_some_and(|status| status.text_content().as_deref() == Some("11"))
    })
    .await;
}

#[wasm_bindgen_test]
async fn accepted_route_clears_host_focus_and_callbacks_while_unchanged_route_keeps_surface() {
    let runtime = support::new_runtime();
    support::open_document(&runtime, document()).await;
    let revision = runtime.model().accepted.as_ref().unwrap().document.revision;
    let token = runtime.model().accepted.as_ref().unwrap().token;
    let window = web_sys::window().unwrap();
    let document = window.document().unwrap();
    let root = document.create_element("div").unwrap();
    document.body().unwrap().append_child(&root).unwrap();
    let capture = mount_commands_host(&runtime, &root);
    wait_for_commands_host(&root).await;
    let mut handles = capture
        .borrow()
        .as_ref()
        .copied()
        .expect("commands host captured");
    click(&root, "Objects");
    wait_until("Objects opens", || {
        by_role_name(&root, "button", "Objects")
            .is_some_and(|button| button.get_attribute("aria-expanded").as_deref() == Some("true"))
    })
    .await;
    handles.workspace.set("Layout");
    settle_frames().await;
    assert!(
        by_role_name(&root, "button", "Objects")
            .is_some_and(|button| button.get_attribute("aria-expanded").as_deref() == Some("true")),
        "an unchanged route leaves the open surface alone"
    );
    assert_eq!(
        runtime.model().accepted.as_ref().unwrap().token,
        token,
        "no route means no new session"
    );
    let heading = root
        .query_selector("#workbench-objects h2")
        .unwrap()
        .unwrap()
        .dyn_into::<web_sys::HtmlElement>()
        .unwrap();
    heading.focus().unwrap();
    assert_eq!(
        document.active_element().as_ref(),
        Some(heading.as_ref()),
        "precondition: the open host holds focus"
    );
    handles.workspace.set("Case");
    wait_until("Case mounts its legacy host without a workbench panel", || {
        root.query_selector("#m1-objects-panel").unwrap().is_some()
            && root.query_selector("#workbench-objects").unwrap().is_none()
            && root.query_selector("#workbench-inspector").unwrap().is_none()
    })
    .await;
    assert!(!heading.is_connected(), "accepted route detaches the old panel");
    assert_ne!(
        document.active_element().as_ref(),
        Some(heading.as_ref()),
        "accepted route clears host focus"
    );
    assert!(
        document
            .active_element()
            .as_ref()
            .is_none_or(|active| active.is_connected()),
        "focus never strands in detached host content"
    );
    assert_eq!(*handles.workspace.peek(), "Case");
    assert_eq!(
        runtime.model().accepted.as_ref().unwrap().token,
        token,
        "the route change carries no edit and no shell-made cancellation"
    );
    assert_eq!(
        runtime.model().accepted.as_ref().unwrap().document.revision,
        revision
    );
    let mode_of = |selector: &str| {
        root.query_selector(selector)
            .unwrap()
            .unwrap()
            .get_attribute("data-mode")
    };
    let escape = web_sys::KeyboardEventInit::new();
    escape.set_key("Escape");
    escape.set_bubbles(true);
    escape.set_cancelable(true);
    document
        .body()
        .unwrap()
        .dispatch_event(&web_sys::KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &escape).unwrap())
        .unwrap();
    settle_frames().await;
    assert_eq!(
        count(
            &root,
            "#workbench-objects:not([hidden]), #workbench-inspector:not([hidden])"
        ),
        0,
        "old host Escape callbacks stay dead after the route"
    );
    if !super::is_compact_viewport() {
        assert_eq!(
            mode_of("#m1-objects-panel").as_deref(),
            Some("pinned"),
            "old host callbacks cannot disturb the new legacy dock"
        );
    }
    handles.workspace.set("Layout");
    wait_until("Layout utilities mount again", || {
        by_role_name(&root, "button", "Objects").is_some()
            && by_role_name(&root, "button", "Inspect").is_some()
    })
    .await;
    assert_eq!(
        by_role_name(&root, "button", "Objects")
            .unwrap()
            .get_attribute("aria-expanded")
            .as_deref(),
        Some("false"),
        "returning after the accepted route reopens no panel"
    );
    root.remove();
}

#[wasm_bindgen_test]
async fn caller_owned_input_survives_round_trip_and_session_replace_clears_old_host() {
    let runtime = support::new_runtime();
    support::open_document(&runtime, document()).await;
    let revision = runtime.model().accepted.as_ref().unwrap().document.revision;
    let mut replacement = document();
    replacement.id = "layout-layers-replaced".into();
    let window = web_sys::window().unwrap();
    let document = window.document().unwrap();
    let root = document.create_element("div").unwrap();
    document.body().unwrap().append_child(&root).unwrap();
    let capture = mount_commands_host(&runtime, &root);
    wait_for_commands_host(&root).await;
    let mut handles = capture
        .borrow()
        .as_ref()
        .copied()
        .expect("commands host captured");
    let field = root
        .query_selector("#caller-retained-input")
        .unwrap()
        .unwrap()
        .dyn_into::<web_sys::HtmlInputElement>()
        .unwrap();
    field.set_value("kept across the round trip");
    field.focus().unwrap();
    click(&root, "Objects");
    wait_until("Objects opens", || {
        by_role_name(&root, "button", "Objects")
            .is_some_and(|button| button.get_attribute("aria-expanded").as_deref() == Some("true"))
    })
    .await;
    handles.workspace.set("Case");
    wait_until("Case mounts its legacy host without a workbench panel", || {
        root.query_selector("#m1-objects-panel").unwrap().is_some()
            && root.query_selector("#workbench-objects").unwrap().is_none()
    })
    .await;
    assert_eq!(
        field.value(),
        "kept across the round trip",
        "caller-owned input survives the workspace round trip"
    );
    assert!(field.is_connected());
    handles.workspace.set("Layout");
    wait_for_commands_host(&root).await;
    assert_eq!(
        field.value(),
        "kept across the round trip",
        "caller-owned input survives the return"
    );
    assert_eq!(
        by_role_name(&root, "button", "Objects")
            .unwrap()
            .get_attribute("aria-expanded")
            .as_deref(),
        Some("false"),
        "returning reopens no panel"
    );
    assert_eq!(
        by_role_name(&root, "button", "Inspect")
            .unwrap()
            .get_attribute("aria-expanded")
            .as_deref(),
        Some("false"),
        "returning reopens no panel"
    );
    click(&root, "Objects");
    wait_until("Objects opens before the session replace", || {
        by_role_name(&root, "button", "Objects")
            .is_some_and(|button| button.get_attribute("aria-expanded").as_deref() == Some("true"))
    })
    .await;
    let heading = root
        .query_selector("#workbench-objects h2")
        .unwrap()
        .unwrap()
        .dyn_into::<web_sys::HtmlElement>()
        .unwrap();
    heading.focus().unwrap();
    support::open_document(&runtime, replacement).await;
    wait_until("the replacement session lands", || {
        runtime.model().accepted.as_ref().is_some_and(|snapshot| {
            snapshot.document.id == "layout-layers-replaced"
        })
    })
    .await;
    wait_until("the replacement session closes the old panel", || {
        root.query_selector("#workbench-objects[hidden][inert]")
            .unwrap()
            .is_some()
    })
    .await;
    assert_ne!(
        document.active_element().as_ref(),
        Some(heading.as_ref()),
        "the replaced session keeps no old focus authority"
    );
    assert!(
        document
            .active_element()
            .as_ref()
            .is_none_or(|active| active.is_connected()),
        "focus never strands in replaced host content"
    );
    assert_eq!(
        field.value(),
        "kept across the round trip",
        "caller-owned input survives the session replace"
    );
    field.focus().unwrap();
    let field_element: web_sys::Element = field.clone().into();
    assert_eq!(
        document.active_element().as_ref(),
        Some(&field_element),
        "caller-owned input stays usable after the replace"
    );
    let search = root
        .query_selector("#workbench-objects input[type=search], #workbench-objects h2")
        .unwrap()
        .unwrap();
    let escape = web_sys::KeyboardEventInit::new();
    escape.set_key("Escape");
    escape.set_bubbles(true);
    escape.set_cancelable(true);
    search
        .dispatch_event(&web_sys::KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &escape).unwrap())
        .unwrap();
    settle_frames().await;
    assert_eq!(
        by_role_name(&root, "button", "Objects")
            .unwrap()
            .get_attribute("aria-expanded")
            .as_deref(),
        Some("false"),
        "old host callbacks cannot reopen after the replace"
    );
    click(&root, "Objects");
    wait_until("the new session still opens its panel", || {
        by_role_name(&root, "button", "Objects")
            .is_some_and(|button| button.get_attribute("aria-expanded").as_deref() == Some("true"))
    })
    .await;
    assert_eq!(
        runtime.model().accepted.as_ref().unwrap().document.revision,
        revision,
        "session replace performs no edit"
    );
    root.remove();
}
