//! X/Y/Z presentation shared by the assembly and component model editors.
//! Each owner keeps its own drafts, validation and commits.
use boardstudio_web_ui_shared::ui::{
    field::{Field, FieldError, FieldLabel},
    input::InputType,
    input_group::{InputGroup, InputGroupInput},
};
use dioxus::prelude::*;

#[component]
pub(super) fn ModelAxisGrid(children: Element) -> Element {
    rsx! { div { class: "grid grid-cols-3 gap-1.5 min-w-0", {children} } }
}

#[component]
pub(super) fn ModelAxisField(
    axis: &'static str,
    value: String,
    #[props(default)] aria_label: Option<String>,
    #[props(default)] disabled: bool,
    #[props(default)] min: Option<String>,
    #[props(default)] error: Option<String>,
    #[props(default)] oninput: EventHandler<FormEvent>,
    #[props(default)] onblur: EventHandler<FocusEvent>,
    #[props(default)] onkeydown: EventHandler<KeyboardEvent>,
) -> Element {
    let id = format!(
        "parts-model-axis-{}-{}",
        axis.to_lowercase(),
        dioxus::core::current_scope_id().0
    );
    let message_id = format!("{id}-message");
    let has_error = error.is_some();
    // An explicit aria-label owns the accessible name; otherwise the visible
    // axis label names the field through label[for].
    let input_label = aria_label.clone();
    let described_by = has_error.then(|| message_id.clone());
    rsx! {
        Field { invalid: has_error, disabled, class: "gap-1 min-w-0",
            FieldLabel { html_for: id.clone(), "{axis}" }
            InputGroup {
                InputGroupInput {
                    id: id.clone(),
                    r#type: InputType::Number,
                    step: "0.1",
                    min,
                    value: "{value}",
                    disabled,
                    aria_label: input_label,
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
