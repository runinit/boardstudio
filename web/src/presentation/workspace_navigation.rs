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
    use boardstudio_web_ui_shared::controls::{BoardToolbar, BoardToolbarButton};
    use wasm_bindgen::JsCast;
    rsx! {
        BoardToolbar { label,
            for (index, (name, selector, activate)) in actions.into_iter().enumerate() {
                BoardToolbarButton { index, disabled, aria_label: name, on_activate: move |_| {
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
