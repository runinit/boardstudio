//! Reusable inspector field compositions. Values and edits remain owned by callers.
//!
//! Single source for the Parts and Keycaps form rows: label/input/error structure
//! lives here, owners keep drafts, validation, blur commits and settlement.
use dioxus::prelude::*;

use crate::ui::{
    checkbox::{Checkbox, CheckboxState},
    collapsible::{Collapsible, CollapsibleContent, CollapsibleTrigger},
    field::{
        Field, FieldContent, FieldDescription, FieldError, FieldGroup, FieldLabel, FieldVariant,
    },
    input::InputType,
    input_group::{
        InputGroup, InputGroupAddon, InputGroupAddonAlign, InputGroupInput, InputGroupText,
    },
    select::{Select, SelectOption},
};

fn field_message_id(id: &str) -> String {
    format!("{id}-message")
}

/// A controlled numeric text value, optionally accompanied by a unit.
#[component]
pub fn NumericField(
    id: String,
    label: String,
    value: String,
    oninput: EventHandler<FormEvent>,
    #[props(default)] unit: Option<String>,
    #[props(default)] description: Option<String>,
    #[props(default)] error: Option<String>,
    #[props(default)] disabled: bool,
    #[props(default)] aria_label: Option<String>,
    #[props(default)] placeholder: Option<String>,
    #[props(default)] min: Option<String>,
    #[props(default)] max: Option<String>,
    #[props(default = "any".to_owned())] step: String,
    #[props(default)] readonly: bool,
    #[props(default)] onblur: EventHandler<FocusEvent>,
    #[props(default)] onkeydown: EventHandler<KeyboardEvent>,
) -> Element {
    let message_id = field_message_id(&id);
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
                    readonly,
                    placeholder,
                    min,
                    max,
                    step,
                    aria_label,
                    aria_invalid: has_error,
                    aria_describedby: described_by,
                    oninput: move |event: FormEvent| oninput.call(event),
                    attributes: vec![
                        dioxus::events::onblur(move |event: FocusEvent| onblur.call(event)),
                        dioxus::events::onkeydown(move |event: KeyboardEvent| onkeydown.call(event)),
                    ],
                }
                if let Some(unit) = unit {
                    InputGroupAddon { align: InputGroupAddonAlign::InlineEnd,
                        InputGroupText { "{unit}" }
                    }
                }
            }
            if let Some(description) = description { FieldDescription { "{description}" } }
            if let Some(error) = error {
                div { id: message_id, FieldError { "{error}" } }
            }
        }
    }
}

/// A controlled single-line text field.
#[component]
pub fn TextField(
    id: String,
    label: String,
    value: String,
    oninput: EventHandler<FormEvent>,
    #[props(default)] r#type: InputType,
    #[props(default)] description: Option<String>,
    #[props(default)] error: Option<String>,
    #[props(default)] disabled: bool,
    #[props(default)] aria_label: Option<String>,
    #[props(default)] placeholder: Option<String>,
    #[props(default)] readonly: bool,
    #[props(default)] maxlength: Option<String>,
    #[props(default)] onblur: EventHandler<FocusEvent>,
    #[props(default)] onkeydown: EventHandler<KeyboardEvent>,
) -> Element {
    let message_id = field_message_id(&id);
    let has_error = error.is_some();
    let described_by = has_error.then(|| message_id.clone());
    rsx! {
        Field { invalid: has_error, disabled, class: "gap-1 min-w-0",
            FieldLabel { html_for: id.clone(), "{label}" }
            InputGroup {
                InputGroupInput {
                    id: id.clone(),
                    r#type,
                    value: "{value}",
                    disabled,
                    readonly,
                    placeholder,
                    aria_label,
                    aria_invalid: has_error,
                    aria_describedby: described_by,
                    oninput: move |event: FormEvent| oninput.call(event),
                    attributes: {
                        let mut extra = vec![
                            dioxus::events::onblur(move |event: FocusEvent| onblur.call(event)),
                            dioxus::events::onkeydown(move |event: KeyboardEvent| onkeydown.call(event)),
                        ];
                        if let Some(maxlength) = maxlength.clone() {
                            extra.push(Attribute::new("maxlength", maxlength.as_str(), None, false));
                        }
                        extra
                    },
                }
            }
            if let Some(description) = description { FieldDescription { "{description}" } }
            if let Some(error) = error {
                div { id: message_id, FieldError { "{error}" } }
            }
        }
    }
}

/// A fixed short-list select row (twelve or fewer options). Longer lists belong
/// in `choice_field::ChoiceField`.
#[component]
pub fn SelectField(
    label: String,
    value: String,
    options: Vec<(String, String)>,
    on_select: EventHandler<String>,
    #[props(default)] aria_label: Option<String>,
    #[props(default)] disabled: bool,
) -> Element {
    let id = format!("inspector-select-{}", dioxus::core::current_scope_id().0);
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

/// A controlled multiline text composition in the shared Field structure.
#[component]
pub fn TextareaField(
    label: String,
    value: String,
    oninput: EventHandler<FormEvent>,
    #[props(default)] aria_label: Option<String>,
    #[props(default)] disabled: bool,
    #[props(default = 3)] rows: u8,
    #[props(default = true)] spellcheck: bool,
    #[props(default)] placeholder: Option<String>,
    #[props(default)] error: Option<String>,
    #[props(default)] description: Option<String>,
) -> Element {
    let id = format!("inspector-textarea-{}", dioxus::core::current_scope_id().0);
    let message_id = field_message_id(&id);
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
                placeholder,
                aria_invalid: has_error,
                aria_describedby: described_by,
                oninput: move |event: FormEvent| oninput.call(event),
            }
            if let Some(description) = description { FieldDescription { "{description}" } }
            if let Some(error) = error {
                div { id: message_id, FieldError { "{error}" } }
            }
        }
    }
}

/// A controlled checkbox and its associated label and help text.
#[component]
pub fn CheckboxField(
    id: String,
    label: String,
    checked: CheckboxState,
    on_checked_change: EventHandler<CheckboxState>,
    #[props(default)] description: Option<String>,
    #[props(default)] error: Option<String>,
    #[props(default)] disabled: bool,
    #[props(default)] aria_label: Option<String>,
) -> Element {
    rsx! {
        Field { variant: FieldVariant::Horizontal, invalid: error.is_some(), disabled,
            Checkbox { id: id.clone(), aria_label, checked: Some(checked), disabled, on_checked_change }
            FieldContent {
                FieldLabel { html_for: id, "{label}" }
                if let Some(description) = description { FieldDescription { "{description}" } }
                if let Some(error) = error { FieldError { "{error}" } }
            }
        }
    }
}

/// A titled section whose open state is owned by the caller.
#[component]
pub fn InspectorSection(
    title: String,
    open: bool,
    on_open_change: EventHandler<bool>,
    children: Element,
) -> Element {
    rsx! {
        Collapsible { keep_mounted: true, open: Some(open), on_open_change,
            CollapsibleTrigger { class: "max-[960px]:min-h-control-touch [@media(pointer:coarse)]:min-h-control-touch", "{title}" }
            CollapsibleContent { class: "data-[open=false]:hidden", FieldGroup { {children} } }
        }
    }
}

/// Three-column grid for X/Y/Z axis rows shared by model editors.
#[component]
pub fn AxisGrid(children: Element) -> Element {
    rsx! { div { class: "grid grid-cols-3 gap-1.5 min-w-0", {children} } }
}

/// One numeric axis input in the shared Field structure.
#[component]
pub fn AxisField(
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
        "inspector-axis-{}-{}",
        axis.to_lowercase(),
        dioxus::core::current_scope_id().0
    );
    let message_id = field_message_id(&id);
    let has_error = error.is_some();
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

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;

    fn fixture() -> Element {
        rsx! {
            InspectorSection { title: "Geometry", open: true, on_open_change: |_| {},
                NumericField { id: "width", label: "Width", value: "3", unit: Some("mm".into()), oninput: |_| {} }
                TextField { id: "name", label: "Name", value: "Part", oninput: |_| {} }
                CheckboxField { id: "enabled", label: "Enabled", checked: CheckboxState::Checked, on_checked_change: |_| {} }
                SelectField { label: "Side", value: "front", options: vec![("front".into(), "Front".into())], on_select: |_| {} }
                TextareaField { label: "Notes", value: "n", oninput: |_| {} }
                AxisGrid {
                    AxisField { axis: "X", value: "1", oninput: |_| {} }
                }
            }
        }
    }

    #[test]
    fn compositions_render_on_host() {
        let mut dom = VirtualDom::new(fixture);
        dom.rebuild_in_place();
    }
}
