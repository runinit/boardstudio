use super::*;
use boardstudio_application::{Scope, SessionEpoch};
use boardstudio_web_ui_shared::test_wait::wait_until;
use wasm_bindgen::JsCast;
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

#[component]
fn state_host() -> Element {
    let mut selected_context = use_signal(|| None::<objects::ScopedTreeContext>);
    let mut workspace = use_signal(|| "Layout");
    let state = use_layout_workspace_state(selected_context, workspace);
    let mut inspector_tab = state.inspector_tab();
    let selection_kind = state.selection_kind();
    let mut pending_pick = state.pending_splay_origin_pick();
    let mut pending_anchor_pick = state.pending_anchor_pick();
    let mut active_tool = state.transform_tool();
    let owner = objects::MatrixTransformInspectorOwner {
        editor_instance_id: 1,
        workspace: "Layout",
        context_generation: 1,
        scope_generation: 1,
        scope: Scope {
            session_epoch: SessionEpoch(1),
            document_id: "layout-state".into(),
            board_id: "board".into(),
            instance_id: None,
        },
        matrix_id: "matrix".into(),
        context: objects::TreeContext::Matrix {
            matrix_id: "matrix".into(),
        },
    };
    let selection_owner = owner.clone();
    let pick_owner = owner.clone();
    let anchor_pick_owner = owner.clone();
    let selection_kind_label = match selection_kind() {
        objects::LayoutSelectionKind::Matrix => "Matrix",
        objects::LayoutSelectionKind::Row => "Row",
        objects::LayoutSelectionKind::Column => "Column",
        objects::LayoutSelectionKind::Key => "Key",
        objects::LayoutSelectionKind::Part => "Part",
    };
    let inspector_tab_label = match inspector_tab() {
        LayoutInspectorTab::Properties => "Properties",
        LayoutInspectorTab::Relations => "Relations",
    };
    let selection_present = selected_context().is_some();
    rsx! {
                Button {
                    variant: ButtonVariant::Secondary,
                    id: "layout-state-relations",
                    onclick: move |_| inspector_tab.set(LayoutInspectorTab::Relations),
                    "Relations"
                }
                Button { variant: ButtonVariant::Secondary,
    id: "layout-state-select",
    onclick: move |_| {
                            let context = objects::TreeContext::Matrix { matrix_id: "matrix".into() };
                            selected_context.set(Some(objects::ScopedTreeContext {
                                scope: selection_owner.scope.clone(),
                                context: context.clone(),
                            }));
                            state.set_selection_kind_from_context(&context);
                        },
                    "Select matrix"
                 }
                Button {
                    variant: ButtonVariant::Secondary,
                    id: "layout-state-arm-pick",
                    onclick: move |_| pending_pick.set(Some(pick_owner.clone())),
                    "Arm origin pick"
                }
                Button {
                    variant: ButtonVariant::Secondary,
                    id: "layout-state-arm-anchor-pick",
                    onclick: move |_| pending_anchor_pick.set(Some(anchor_pick_owner.clone())),
                    "Arm anchor pick"
                }
                Button {
                    variant: ButtonVariant::Secondary,
                    id: "layout-state-enter-3d",
                    onclick: move |_| state.set_assembly_3d(true),
                    "Enter 3D"
                }
                Button {
                    variant: ButtonVariant::Secondary,
                    id: "layout-state-tool",
                    onclick: move |_| active_tool.set(Some(objects::LayoutTransformTool::Stagger)),
                    "Activate tool"
                }
                Button {
                    variant: ButtonVariant::Secondary,
                    id: "layout-state-switch",
                    onclick: move |_| workspace.set("Keycaps"),
                    "Switch workspace"
                }
                output { id: "layout-state-tab", "{inspector_tab_label}" }
                output { id: "layout-state-selection-kind", "{selection_kind_label}" }
                output { id: "layout-state-selection-present", "{selection_present}" }
                output { id: "layout-state-pending", "{pending_pick().is_some()}" }
                output { id: "layout-state-anchor-pending", "{pending_anchor_pick().is_some()}" }
                output { id: "layout-state-tool-active", "{active_tool().is_some()}" }
            }
}

fn click(document: &web_sys::Document, id: &str) {
    document
        .get_element_by_id(id)
        .unwrap()
        .dyn_into::<web_sys::HtmlElement>()
        .unwrap()
        .click();
}

#[wasm_bindgen_test]
async fn handle_resets_layout_interactions_when_selection_or_owner_changes() {
    let document = web_sys::window().unwrap().document().unwrap();
    let root = document.create_element("div").unwrap();
    document.body().unwrap().append_child(&root).unwrap();
    let dom = VirtualDom::new(state_host);
    dioxus_web::launch::launch_virtual_dom(
        dom,
        dioxus_web::Config::new().rootnode(root.clone().into()),
    );
    wait_until("Layout workspace state controls mount", || {
        root.query_selector("#layout-state-select")
            .is_ok_and(|button| button.is_some())
    })
    .await;

    click(&document, "layout-state-select");
    wait_until("Layout selection state becomes present", || {
        root.query_selector("#layout-state-selection-present")
            .ok()
            .flatten()
            .is_some_and(|selected| selected.text_content().as_deref() == Some("true"))
    })
    .await;
    assert_eq!(
        document
            .get_element_by_id("layout-state-selection-kind")
            .unwrap()
            .text_content()
            .as_deref(),
        Some("Matrix")
    );
    click(&document, "layout-state-arm-pick");
    wait_until("the origin pick becomes armed", || {
        root.query_selector("#layout-state-pending")
            .ok()
            .flatten()
            .is_some_and(|pending| pending.text_content().as_deref() == Some("true"))
    })
    .await;
    click(&document, "layout-state-arm-anchor-pick");
    wait_until("the anchor pick becomes armed", || {
        root.query_selector("#layout-state-anchor-pending")
            .ok()
            .flatten()
            .is_some_and(|pending| pending.text_content().as_deref() == Some("true"))
    })
    .await;
    click(&document, "layout-state-enter-3d");
    wait_until("entering 3D clears the origin pick", || {
        root.query_selector("#layout-state-pending")
            .ok()
            .flatten()
            .is_some_and(|pending| pending.text_content().as_deref() == Some("false"))
    })
    .await;
    wait_until("entering 3D clears the anchor pick", || {
        root.query_selector("#layout-state-anchor-pending")
            .ok()
            .flatten()
            .is_some_and(|pending| pending.text_content().as_deref() == Some("false"))
    })
    .await;
    assert_eq!(
        document
            .get_element_by_id("layout-state-pending")
            .unwrap()
            .text_content()
            .as_deref(),
        Some("false")
    );
    assert_eq!(
        document
            .get_element_by_id("layout-state-anchor-pending")
            .unwrap()
            .text_content()
            .as_deref(),
        Some("false")
    );
    click(&document, "layout-state-tool");
    wait_until("the transform tool becomes active", || {
        root.query_selector("#layout-state-tool-active")
            .ok()
            .flatten()
            .is_some_and(|active| active.text_content().as_deref() == Some("true"))
    })
    .await;
    click(&document, "layout-state-switch");
    wait_until("leaving Layout clears the transform tool", || {
        root.query_selector("#layout-state-tool-active")
            .ok()
            .flatten()
            .is_some_and(|active| active.text_content().as_deref() == Some("false"))
    })
    .await;
    assert_eq!(
        document
            .get_element_by_id("layout-state-tool-active")
            .unwrap()
            .text_content()
            .as_deref(),
        Some("false")
    );
    root.remove();
}
