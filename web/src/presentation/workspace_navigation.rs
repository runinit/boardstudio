//! Header workspace navigation; document trees keep their own selection and visibility owners.
use boardstudio_web_ui_model::Workspace;
use boardstudio_web_ui_shared::ui::{
    button::{Button, ButtonVariant},
    toolbar::{Toolbar, ToolbarButton},
};
use dioxus::prelude::*;
use wasm_bindgen::JsCast;

#[component]
pub(super) fn WorkspaceObjectNavigation(mut workspace: Signal<Workspace>) -> Element {
    rsx! {
        nav { class: "header-workspaces", aria_label: "Workspaces",
            for name in Workspace::TABS {
                Button {
                    key: "{name}", id: "m1-workspace-{name}", variant: ButtonVariant::Ghost,
                    aria_label: "Open {name} workspace",
                    aria_current: (workspace() == name).then_some("page".to_string()),
                    onclick: move |_| workspace.set(name), "{name}"
                }
            }
        }
    }
}

#[component]
pub(super) fn InspectorActions(
    label: String,
    workspace: Signal<Workspace>,
    mut inspect_open: Signal<bool>,
    inspector_settings: Signal<super::PanelSettings>,
    generation: Signal<u64>,
    actions: Vec<(&'static str, &'static str, bool)>,
    disabled: bool,
) -> Element {
    rsx! {
        Toolbar { aria_label: label, class: "max-w-full flex-wrap justify-center",
            for (index, (name, selector, activate)) in actions.into_iter().enumerate() {
                ToolbarButton { index, disabled, aria_label: name, on_click: move |_| {
                    let (ws, g) = (workspace(), generation());
                    super::pin_inspector_on_desktop(inspector_settings);
                    inspect_open.set(true);
                    spawn(async move {
                        gloo_timers::future::TimeoutFuture::new(0).await;
                        if workspace() != ws || generation() != g { return; }
                        let Some(el) = web_sys::window().and_then(|w| w.document()).and_then(|d| d.query_selector(selector).ok().flatten()) else { return; };
                        let mut anc = el.parent_element();
                        let mut triggers = Vec::new();
                        while let Some(p) = anc {
                            if p.tag_name().eq_ignore_ascii_case("details") { let _ = p.set_attribute("open", ""); }
                            if p.get_attribute("data-slot").as_deref() == Some("collapsible-content")
                                && let Some(o) = p.parent_element()
                                && let Ok(Some(t)) = o.query_selector(":scope > [data-slot='collapsible-trigger'][aria-expanded='false']")
                                && let Ok(t) = t.dyn_into::<web_sys::HtmlElement>() { triggers.push(t); }
                            anc = p.parent_element();
                        }
                        for t in triggers.into_iter().rev() {
                            if workspace() != ws || generation() != g || !el.is_connected() { return; }
                            t.click();
                            gloo_timers::future::TimeoutFuture::new(0).await;
                        }
                        if workspace() == ws && generation() == g && el.is_connected()
                            && let Ok(el) = el.dyn_into::<web_sys::HtmlElement>()
                        {
                            if activate { el.click(); }
                            let _ = el.focus();
                        }
                    });
                }, "{name}" }
            }
        }
    }
}
