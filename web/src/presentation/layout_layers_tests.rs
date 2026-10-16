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

use boardstudio_web_ui_shared::test_wait::wait_until;

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
    wait_until("the scene shows its keycap and switch", || {
        count(&root, ".m1-keycap-overlay") == 1 && count(&root, ".m1-scene-part .m1-part") == 1
    })
    .await;
    assert_eq!(count(&root, ".m1-keycap-overlay"), 1);
    assert_eq!(count(&root, ".m1-scene-part .m1-part"), 1);
    assert_eq!(
        count(
            &root,
            "#m1-objects-panel #m1-layers-trigger[aria-expanded='false']"
        ),
        1,
        "Layout layers must start collapsed at the persistent bottom edge"
    );
    assert_eq!(
        count(&root, "#m1-layers-trigger[aria-label='Canvas visibility']"),
        1,
        "Layout visibility controls are distinct from authored and PCB layers"
    );
    assert_eq!(
        count(&root, ".m1-workspace-content .m1-layers"),
        0,
        "the viewer must not duplicate the docked visibility controls"
    );
    click(&root, "#m1-layers-trigger");
    wait_until("the layers menu offers Hide Switches", || {
        count(&root, "button[aria-label='Hide Switches']") == 1
    })
    .await;
    click(&root, "button[aria-label='Hide Switches']");
    wait_until(
        "hiding switches removes the switch and offers Show Switches",
        || {
            count(&root, ".m1-scene-part .m1-part") == 0
                && count(&root, "button[aria-label='Show Switches']") == 1
        },
    )
    .await;
    assert_eq!(
        count(&root, ".m1-keycap-overlay"),
        1,
        "hiding switches must preserve visible keycaps"
    );
    assert_eq!(count(&root, ".m1-scene-part .m1-part"), 0);
    assert_eq!(count(&root, "button[aria-label='Show Switches']"), 1);
    click(&root, "button[aria-label='Hide Keycaps']");
    wait_until(
        "hiding keycaps removes the keycaps and the scene part",
        || count(&root, ".m1-keycap-overlay") == 0 && count(&root, ".m1-scene-part") == 0,
    )
    .await;
    assert_eq!(count(&root, ".m1-keycap-overlay"), 0);
    assert_eq!(count(&root, ".m1-scene-part"), 0);
    click(&root, "button[aria-label='Show Switches']");
    wait_until(
        "showing switches restores the switch without keycaps",
        || count(&root, ".m1-scene-part .m1-part") == 1 && count(&root, ".m1-keycap-overlay") == 0,
    )
    .await;
    assert_eq!(count(&root, ".m1-scene-part .m1-part"), 1);
    assert_eq!(count(&root, ".m1-keycap-overlay"), 0);
    assert_eq!(
        runtime.model().accepted.as_ref().unwrap().document.revision,
        revision
    );
    root.remove();
}

fn finding_row_containing(root: &web_sys::Element, phrase: &str) -> Option<web_sys::Element> {
    let rows = root.query_selector_all("li.m1-layout-finding").unwrap();
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
        count(&root, ".m1-scene-part .m1-part") > 0
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
    click(&root, "#m1-layout-findings-trigger");
    wait_until("the footer opens the Info-only findings panel", || {
        root.query_selector(".m1-layout-findings")
            .unwrap()
            .is_some()
            && root
                .text_content()
                .unwrap_or_default()
                .contains("Layout findings: 3")
            && finding_row_containing(&root, &pin_notice).is_some()
    })
    .await;
    let text = root.text_content().unwrap_or_default();
    assert!(text.contains(&pin_notice));
    assert!(text.contains(&net_notice));
    assert!(text.contains(&registration_notice));
    let pin_row = finding_row_containing(&root, &pin_notice).expect("pin notice row is rendered");
    assert!(
        pin_row
            .query_selector("button.wb-finding-action")
            .unwrap()
            .is_none(),
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
        transaction_id: "ticket160-browser-cancel-preview".into(),
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
