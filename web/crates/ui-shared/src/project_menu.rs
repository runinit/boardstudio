//! Project menu pieces shared by the shell's menu and the Library page.
use crate::ui::dropdown_menu::{
    DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuLabel, DropdownMenuSeparator,
    DropdownMenuTrigger,
};
use crate::ui::icons::{Check, ChevronDown};
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

/// Browser preferences; the shell may open Settings from the Project menu too.
#[component]
pub fn ThemePicker(#[props(default)] open: Option<Signal<bool>>) -> Element {
    let fallback_open = use_signal(|| false);
    let mut open = open.unwrap_or(fallback_open);
    let mut appearance = use_context::<AppearanceState>().0;
    let mut theme = use_context::<ThemeState>().0;
    let preference_warning = use_context::<PreferenceStorageWarning>().0;
    rsx! {
        DropdownMenu { open: open(), on_open_change: move |next| open.set(next),
            DropdownMenuTrigger { aria_label: "Settings", "Theme" ChevronDown {} }
            DropdownMenuContent { class: "header-settings-menu",
                div { role: "group", aria_label: "Appearance",
                    DropdownMenuLabel { "Appearance" }
                    for (index, (value, label)) in [("system", "System"), ("light", "Light"), ("dark", "Dark")].into_iter().enumerate() {
                        DropdownMenuItem { value, index, role: "menuitemradio", aria_checked: appearance() == value,
                            on_select: move |value| {
                                crate::panels::write_appearance_preference(preference_warning, value);
                                appearance.set(value);
                            },
                            span { class: "inline-flex size-4", if appearance() == value { Check {} } }
                            "{label}"
                        }
                    }
                }
                DropdownMenuSeparator {}
                div { role: "group", aria_label: "Theme",
                    DropdownMenuLabel { "Theme" }
                    for (index, (value, label)) in [("original", "Original"), ("shadcn", "shadcn")].into_iter().enumerate() {
                        DropdownMenuItem { value, index: index + 3, role: "menuitemradio", aria_checked: theme() == value,
                            on_select: move |value| {
                                crate::panels::write_workbench_theme_preference(preference_warning, value);
                                theme.set(value);
                            },
                            span { class: "inline-flex size-4", if theme() == value { Check {} } }
                            "{label}"
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
#[path = "header_control_tests.rs"]
mod header_control_tests;

pub fn durability_label(durability: &Durability) -> &'static str {
    match durability {
        Durability::Saved { .. } => "Saved",
        Durability::Saving { .. } => "Saving…",
        Durability::Failed { .. } => "Save failed",
        _ => "Pending",
    }
}
