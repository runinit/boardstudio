//! Parts-editor field compositions (021): the mapped Field + InputGroup shape for the
//! definition, pad, assembly, model, profile and generator editors.
//!
//! Each owner keeps its own drafts, validation, blur commits and key handling;
//! these helpers only own the label/input/error structure. An explicit
//! `aria_label` owns the accessible name (existing tests address fields by it);
//! otherwise the visible label names the field through `label[for]`.
use boardstudio_web_ui_shared::choice_field::{ChoiceField, ChoiceFieldLayout};
use boardstudio_web_ui_shared::ui::{
    field::{Field, FieldError, FieldLabel},
    input::InputType,
    input_group::{InputGroup, InputGroupInput},
    select::{Select, SelectOption},
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
    let id = editor_field_id("parts-editor-text");
    let message_id = format!("{id}-message");
    let has_error = error.is_some();
    let described_by = has_error.then(|| message_id.clone());
    rsx! {
        Field { invalid: has_error, disabled, class: "gap-1 min-w-0",
            FieldLabel { html_for: id.clone(), "{label}" }
            InputGroup {
                InputGroupInput {
                    id: id.clone(),
                    r#type: InputType::Text,
                    value: "{value}",
                    disabled,
                    readonly,
                    placeholder,
                    aria_label,
                    aria_invalid: has_error,
                    aria_describedby: described_by,
                    oninput: move |event: FormEvent| oninput.call(event),
                    attributes: vec![
                        dioxus::events::onblur(move |event: FocusEvent| onblur.call(event)),
                        dioxus::events::onkeydown(move |event: KeyboardEvent| onkeydown.call(event)),
                    ],
                }
            }
            if let Some(message) = error.as_deref() {
                div { id: message_id, FieldError { "{message}" } }
            }
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
    let id = editor_field_id("parts-editor-number");
    let message_id = format!("{id}-message");
    let has_error = error.is_some();
    let described_by = has_error.then(|| message_id.clone());
    rsx! {
        Field { invalid: has_error, disabled, class: "gap-1 min-w-0",
            FieldLabel { html_for: id.clone(), "{label}" }
            InputGroup {
                InputGroupInput {
                    id: id.clone(),
                    r#type: InputType::Number,
                    value: "{value}",
                    disabled,
                    min,
                    max,
                    step,
                    placeholder,
                    aria_label,
                    aria_invalid: has_error,
                    aria_describedby: described_by,
                    oninput: move |event: FormEvent| oninput.call(event),
                    attributes: vec![
                        dioxus::events::onblur(move |event: FocusEvent| onblur.call(event)),
                        dioxus::events::onkeydown(move |event: KeyboardEvent| onkeydown.call(event)),
                    ],
                }
            }
            if let Some(message) = error.as_deref() {
                div { id: message_id, FieldError { "{message}" } }
            }
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
    let id = editor_field_id("parts-editor-select");
    let current = use_memo(use_reactive((&value,), |(value,)| Some(value)));
    let name = aria_label.unwrap_or_else(|| label.clone());
    rsx! {
        Field { class: "gap-1.5", disabled,
            FieldLabel { html_for: id.clone(), "{label}" }
            Select::<String> {
                class: "max-[960px]:[&_[data-slot=select-trigger]]:min-h-control-touch [@media(pointer:coarse)]:[&_[data-slot=select-trigger]]:min-h-control-touch",
                id: id.clone(),
                aria_label: name,
                value: Some(current.into()),
                disabled,
                on_value_change: move |next: Option<String>| {
                    if let Some(next) = next {
                        on_select.call(next);
                    }
                },
                for (index, (option_value, option_text)) in options.into_iter().enumerate() {
                    SelectOption::<String> { class: "max-[960px]:min-h-control-touch [@media(pointer:coarse)]:min-h-control-touch", index, value: option_value, text_value: option_text.clone(), "{option_text}" }
                }
            }
        }
    }
}

/// Multiline text composition for the parts editors: Field + label + a native
/// textarea in theme styling. No copied Textarea exists in `ui/`; the mapping
/// sends textareas here and the legacy CSS has no global textarea rule, so a
/// themed native control inside the Field structure is the faithful target.
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
    let id = editor_field_id("parts-editor-textarea");
    let message_id = format!("{id}-message");
    let has_error = error.is_some();
    let described_by = has_error.then(|| message_id.clone());
    rsx! {
        Field { invalid: has_error, disabled, class: "gap-1.5 min-w-0",
            FieldLabel { html_for: id.clone(), "{label}" }
            textarea {
                id: id.clone(),
                class: "text-foreground placeholder:text-muted-foreground selection:bg-primary selection:text-primary-foreground flex min-h-control w-full min-w-0 rounded-md bg-field px-3 py-2 text-base transition-[color,box-shadow] outline-none focus-visible:ring-ring/50 focus-visible:ring-2 disabled:pointer-events-none disabled:cursor-not-allowed disabled:opacity-50 aria-invalid:ring-destructive/20 aria-invalid:border-destructive",
                rows: rows as i64,
                spellcheck,
                value: "{value}",
                disabled,
                aria_label,
                aria_invalid: has_error,
                aria_describedby: described_by,
                oninput: move |event: FormEvent| oninput.call(event),
            }
            if let Some(message) = error.as_deref() {
                div { id: message_id, FieldError { "{message}" } }
            }
        }
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
