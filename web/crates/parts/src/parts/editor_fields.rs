//! Parts-editor field compositions: thin owners over the shared inspector molecules.
//!
//! Each owner keeps its own drafts, validation, blur commits and key handling;
//! label/input/error structure lives once in `ui-shared::inspector_fields`.
use boardstudio_web_ui_shared::inspector_fields::{TextField, TextareaField};
use dioxus::prelude::*;

fn editor_field_id(prefix: &str) -> String {
    format!("{prefix}-{}", dioxus::core::current_scope_id().0)
}

#[component]
pub(crate) fn EditorTextField(
    label: String,
    value: String,
    #[props(default)] aria_label: Option<String>,
    #[props(default)] disabled: bool,
    #[props(default)] readonly: bool,
    #[props(default)] placeholder: Option<String>,
    #[props(default)] error: Option<String>,
    #[props(default)] oninput: EventHandler<FormEvent>,
    #[props(default)] onblur: EventHandler<FocusEvent>,
    #[props(default)] onkeydown: EventHandler<KeyboardEvent>,
) -> Element {
    rsx! {
        TextField {
            id: editor_field_id("parts-editor-text"),
            label,
            value,
            aria_label,
            disabled,
            readonly,
            placeholder,
            error,
            oninput,
            onblur,
            onkeydown,
        }
    }
}

/// Multiline text composition for the parts editors: shared Field structure with
/// a themed native control. No copied Textarea exists in `ui/`; the mapping
/// sends textareas here.
#[component]
pub(crate) fn EditorTextarea(
    label: String,
    value: String,
    #[props(default)] aria_label: Option<String>,
    #[props(default)] disabled: bool,
    #[props(default = 3)] rows: u8,
    #[props(default = true)] spellcheck: bool,
    #[props(default)] error: Option<String>,
    #[props(default)] oninput: EventHandler<FormEvent>,
) -> Element {
    rsx! {
        TextareaField { label, value, aria_label, disabled, rows, spellcheck, error, oninput }
    }
}
