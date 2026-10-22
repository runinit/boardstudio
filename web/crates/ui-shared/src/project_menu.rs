//! Project menu pieces shared by the shell's menu and the Library page.
use boardstudio_application::Durability;
use boardstudio_web_ui_model::state::{AppearanceState, PreferenceStorageWarning, ThemeState};
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

/// The Appearance and Theme choices (ticket 010 moves both into the header).
#[component]
pub fn ThemePicker() -> Element {
    let mut appearance = use_context::<AppearanceState>().0;
    let mut theme = use_context::<ThemeState>().0;
    let preference_warning = use_context::<PreferenceStorageWarning>().0;
    rsx! {
        crate::controls::BoardSelectField {
            class: "m1-theme-picker",
            label: "Appearance",
            aria_label: "Color theme",
            value: appearance().to_string(),
            onchange: move |event: FormEvent| {
                let preference = match event.value().as_str() { "light" => "light", "dark" => "dark", _ => "system" };
                crate::panels::write_appearance_preference(preference_warning, preference);
                appearance.set(preference);
            },
            option { value: "system", "System" }
            option { value: "light", "Light" }
            option { value: "dark", "Dark" }
        }
        crate::controls::BoardSelectField {
            class: "m1-theme-picker",
            label: "Theme",
            aria_label: "Workbench theme",
            value: theme().to_string(),
            onchange: move |event: FormEvent| {
                let preference = if event.value() == "shadcn" { "shadcn" } else { "original" };
                crate::panels::write_workbench_theme_preference(preference_warning, preference);
                theme.set(preference);
            },
            option { value: "original", "Original" }
            option { value: "shadcn", "shadcn" }
        }
    }
}

pub fn durability_label(durability: &Durability) -> &'static str {
    match durability {
        Durability::Saved { .. } => "Saved",
        Durability::Saving { .. } => "Saving…",
        Durability::Failed { .. } => "Save failed",
        _ => "Pending",
    }
}
