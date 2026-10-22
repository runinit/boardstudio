//! Header workspace navigation; document trees keep their own selection and visibility owners.
use boardstudio_web_ui_shared::ui::button::{Button, ButtonVariant};
use dioxus::prelude::*;

#[component]
pub(super) fn WorkspaceObjectNavigation(mut workspace: Signal<&'static str>) -> Element {
    rsx! {
        nav { class: "header-workspaces", aria_label: "Workspaces",
            for name in ["Layout", "PCB", "Keymap", "Keycaps", "Case", "Parts"] {
                Button {
                    key: "{name}", id: "m1-workspace-{name}",
                    variant: ButtonVariant::Ghost,
                    aria_label: "Open {name} workspace",
                    aria_current: (workspace() == name).then_some("page".to_string()),
                    onclick: move |_| workspace.set(name),
                    "{name}"
                }
            }
        }
    }
}

/// Focus shortcuts reveal the existing Inspector; they never write document state.
#[component]
pub(super) fn InspectorActions(
    label: String,
    workspace: Signal<&'static str>,
    mut inspect_open: Signal<bool>,
    inspector_settings: Signal<super::PanelSettings>,
    generation: Signal<u64>,
    actions: Vec<(&'static str, &'static str, bool)>,
    disabled: bool,
) -> Element {
    use boardstudio_web_ui_shared::ui::toolbar::{Toolbar, ToolbarButton};
    use wasm_bindgen::JsCast;
    rsx! {
        Toolbar { aria_label: label,
            for (index, (name, selector, activate)) in actions.into_iter().enumerate() {
                ToolbarButton { index, disabled, aria_label: name, on_click: move |_| {
                    let captured_workspace = workspace();
                    let captured_generation = generation();
                    super::pin_inspector_on_desktop(inspector_settings);
                    inspect_open.set(true);
                    spawn(async move {
                        gloo_timers::future::TimeoutFuture::new(0).await;
                        if workspace() != captured_workspace || generation() != captured_generation { return; }
                        let Some(document) = web_sys::window().and_then(|window| window.document()) else { return; };
                        let Some(element) = document.query_selector(selector).ok().flatten() else { return; };
                        let mut ancestor = element.parent_element();
                        while let Some(parent) = ancestor {
                            if parent.tag_name().eq_ignore_ascii_case("details") { let _ = parent.set_attribute("open", ""); }
                            ancestor = parent.parent_element();
                        }
                        if let Ok(element) = element.dyn_into::<web_sys::HtmlElement>() {
                            if activate { element.click(); }
                            let _ = element.focus();
                        }
                    });
                }, "{name}" }
            }
        }
    }
}

#[cfg(all(test, target_arch = "wasm32"))]
mod tests {
    use super::*;
    use boardstudio_web_ui_shared::{
        panels::PanelSettings,
        test_role::by_role_name,
        test_wait::wait_until,
        ui::{field::FieldLabel, input_group::InputGroupInput},
    };
    use wasm_bindgen::JsCast;
    use wasm_bindgen_test::*;

    fn fixture() -> Element {
        let workspace = use_signal(|| "Layout");
        let inspect_open = use_signal(|| false);
        let settings = use_signal(PanelSettings::default_pinned);
        let generation = use_signal(|| 0u64);
        rsx! {
            div {
                InspectorActions {
                    label: "Inspector shortcuts", workspace, inspect_open,
                    inspector_settings: settings, generation,
                    actions: vec![("Edit local draft", "#shell-local-draft", false)],
                    disabled: false,
                }
                details {
                    summary { "Draft section" }
                    FieldLabel { html_for: "shell-local-draft", "Local draft" }
                    InputGroupInput { id: "shell-local-draft", value: "uncommitted" }
                }
                output { role: "status", aria_label: "Inspector revealed", "{inspect_open()}" }
            }
        }
    }

    #[wasm_bindgen_test]
    async fn inspector_shortcut_reveals_and_focuses_the_mounted_draft_owner() {
        let document = web_sys::window().unwrap().document().unwrap();
        let root = document.create_element("div").unwrap();
        document.body().unwrap().append_child(&root).unwrap();
        dioxus_web::launch::launch_virtual_dom(
            VirtualDom::new(fixture),
            dioxus_web::Config::new().rootnode(root.clone().into()),
        );
        wait_until("the Inspector shortcut mounts", || {
            by_role_name(&root, "button", "Edit local draft").is_some()
        })
        .await;
        let field = by_role_name(&root, "textbox", "Local draft")
            .unwrap()
            .dyn_into::<web_sys::HtmlInputElement>()
            .unwrap();
        let shortcut = by_role_name(&root, "button", "Edit local draft").unwrap();
        assert_eq!(
            shortcut.get_attribute("data-slot").as_deref(),
            Some("toolbar-button")
        );
        shortcut.click();
        wait_until(
            "the shortcut reveals and focuses the mounted owner field",
            || document.active_element() == Some(field.clone().into()),
        )
        .await;
        assert_eq!(field.value(), "uncommitted");
        assert!(
            field
                .closest("details")
                .unwrap()
                .unwrap()
                .has_attribute("open")
        );
        root.remove();
    }
}
