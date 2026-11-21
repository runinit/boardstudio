//! Parts-editor field compositions: thin owners over the shared inspector molecules.
//!
//! Each owner keeps its own drafts, validation, blur commits and key handling;
//! label/input/error structure lives once in `ui-shared::inspector_fields`.
use boardstudio_web_ui_shared::choice_field::{ChoiceField, ChoiceFieldLayout};
use boardstudio_web_ui_shared::inspector_fields::{
    NumericField, SelectField, TextField, TextareaField,
};
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

#[component]
pub(crate) fn EditorNumberField(
    label: String,
    value: String,
    #[props(default)] aria_label: Option<String>,
    #[props(default)] disabled: bool,
    #[props(default)] min: Option<String>,
    #[props(default)] max: Option<String>,
    #[props(default = "any".to_owned())] step: String,
    #[props(default)] placeholder: Option<String>,
    #[props(default)] error: Option<String>,
    #[props(default)] oninput: EventHandler<FormEvent>,
    #[props(default)] onblur: EventHandler<FocusEvent>,
    #[props(default)] onkeydown: EventHandler<KeyboardEvent>,
) -> Element {
    rsx! {
        NumericField {
            id: editor_field_id("parts-editor-number"),
            label,
            value,
            aria_label,
            disabled,
            min,
            max,
            step,
            placeholder,
            error,
            oninput,
            onblur,
            onkeydown,
        }
    }
}

/// Short fixed-list select (the 12-option rule) for the parts editors: Field +
/// label + Select. The owner keeps the accepted value and commits through
/// `on_select`; an empty-value option renders the unchosen row.
#[component]
pub(crate) fn EditorSelect(
    label: String,
    value: String,
    options: Vec<(String, String)>,
    #[props(default)] aria_label: Option<String>,
    #[props(default)] disabled: bool,
    on_select: EventHandler<String>,
) -> Element {
    rsx! {
        SelectField { label, value, options, aria_label, disabled, on_select }
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

/// Long-or-short choice field for the parts editors: `Select` through 12
/// options, `Combobox` above (the 12-option rule). The owner keeps the
/// accepted value and commits through `on_select`.
#[component]
pub(crate) fn EditorChoiceField(
    label: String,
    value: String,
    options: Vec<(String, String)>,
    #[props(default)] aria_label: Option<String>,
    #[props(default)] disabled: bool,
    on_select: EventHandler<String>,
) -> Element {
    rsx! {
        ChoiceField { label, value, options, aria_label, disabled, layout: ChoiceFieldLayout::WrappedTight,
            on_value_change: move |next: Option<String>| if let Some(next) = next { on_select.call(next) },
        }
    }
}
