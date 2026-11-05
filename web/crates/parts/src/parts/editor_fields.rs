//! Parts-editor field compositions (021): the mapped Field + InputGroup shape for the
//! definition, pad, assembly, model, profile and generator editors.
//!
//! Each owner keeps its own drafts, validation, blur commits and key handling;
//! these helpers only own the label/input/error structure. An explicit
//! `aria_label` owns the accessible name (existing tests address fields by it);
//! otherwise the visible label names the field through `label[for]`.
use boardstudio_web_ui_shared::ui::{
    field::{Field, FieldError, FieldLabel},
    input::InputType,
    input_group::{InputGroup, InputGroupInput},
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
