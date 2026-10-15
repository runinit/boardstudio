//! Shared workspace navigation; document trees keep their own selection and visibility owners.
use boardstudio_web_ui_shared::controls::BoardButton;
use dioxus::prelude::*;

#[component]
pub(super) fn WorkspaceObjectNavigation(mut workspace: Signal<&'static str>) -> Element {
    use_effect(move || {
        let _ = workspace();
        let _ = document::eval(
            r#"document.querySelector('.m1-workflow-navigation [aria-current="page"]')?.scrollIntoView({block:'nearest', inline:'nearest'});"#,
        );
    });
    rsx! {
        nav { class: "m1-workflow-navigation", aria_label: "Workspaces",
            for (name, glyph) in [
                ("Layout", "M2 2h5v5H2zM9 2h5v5H9zM2 9h5v5H2zM9 9h5v5H9z"),
                ("PCB", "M2 2h12v12H2zM5 5h6v6H5zM1 5h4M11 11h4"),
                ("Keymap", "M2 3h12v10H2zM4 6h2M7 6h2M10 6h2M4 10h8"),
                ("Keycaps", "M4 3h8l2 10H2ZM2 13v2h12v-2M5 6h6"),
                ("Case", "m2 5 6-3 6 3v7l-6 3-6-3ZM2 5l6 3 6-3M8 8v7"),
                ("Parts", "M2 5h12v9H2zM5 5V2h6v3M6 9h4"),
            ] {
                BoardButton {
                    key: "{name}", "id": "m1-object-workspace-{name}",
                    class: "m1-workspace-tree-link", aria_label: "Open {name} workspace",
                    "aria-controls": "m1-workspace-panel",
                    "aria-current": if workspace() == name { "page" },
                    onclick: move |_| workspace.set(name),
                    svg { view_box: "0 0 16 16", fill: "none", stroke: "currentColor", stroke_width: "1.3", "aria-hidden": "true", path { d: glyph } }
                    span { "{name}" }
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
