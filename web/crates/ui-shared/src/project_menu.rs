//! Project menu pieces shared by the shell's menu and the Library page.
use boardstudio_application::Durability;
use boardstudio_web_ui_model::state::{PreferenceStorageWarning, ThemeState};
use dioxus::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::HtmlElement;

pub fn close_project_menu() {
    let Some(document) = web_sys::window().and_then(|window| window.document()) else {
        return;
    };
    let Some(menu) = document
        .query_selector("details.m1-project-menu")
        .ok()
        .flatten()
    else {
        return;
    };
    let _ = menu.remove_attribute("open");
    if let Some(summary) = menu
        .query_selector("summary")
        .ok()
        .flatten()
        .and_then(|element| element.dyn_into::<HtmlElement>().ok())
    {
        let _ = summary.focus();
    }
}

#[component]
pub fn ThemePicker() -> Element {
    let mut theme = use_context::<ThemeState>().0;
    let preference_warning = use_context::<PreferenceStorageWarning>().0;
    rsx! { label { class: "m1-theme-picker", "Appearance"
        select { "aria-label": "Color theme", value: "{theme()}", onchange: move |event: FormEvent| {
            let preference = match event.value().as_str() { "light" => "light", "dark" => "dark", _ => "system" };
            crate::panels::write_theme_preference(preference_warning, preference);
            theme.set(preference);
        },
            option { value: "system", "System" }
            option { value: "light", "Light" }
            option { value: "dark", "Dark" }
        }
    } }
}

pub fn durability_label(durability: &Durability) -> &'static str {
    match durability {
        Durability::Saved { .. } => "Saved",
        Durability::Saving { .. } => "Saving…",
        Durability::Failed { .. } => "Save failed",
        _ => "Pending",
    }
}
