//! Reusable inspector field compositions. Values and edits remain owned by callers.
use dioxus::prelude::*;

use crate::ui::{
    checkbox::{Checkbox, CheckboxState},
    collapsible::{Collapsible, CollapsibleContent, CollapsibleTrigger},
    field::{
        Field, FieldContent, FieldDescription, FieldError, FieldGroup, FieldLabel, FieldVariant,
    },
    input::{Input, InputType},
    input_group::{
        InputGroup, InputGroupAddon, InputGroupAddonAlign, InputGroupInput, InputGroupText,
    },
};

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
) -> Element {
    rsx! {
        Field { invalid: error.is_some(), disabled,
            FieldLabel { html_for: id.clone(), "{label}" }
            InputGroup {
                InputGroupInput { id, value, r#type: InputType::Number, disabled, oninput }
                if let Some(unit) = unit {
                    InputGroupAddon { align: InputGroupAddonAlign::InlineEnd,
                        InputGroupText { "{unit}" }
                    }
                }
            }
            if let Some(description) = description { FieldDescription { "{description}" } }
            if let Some(error) = error { FieldError { "{error}" } }
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
    #[props(default)] description: Option<String>,
    #[props(default)] error: Option<String>,
    #[props(default)] disabled: bool,
) -> Element {
    rsx! {
        Field { invalid: error.is_some(), disabled,
            FieldLabel { html_for: id.clone(), "{label}" }
            Input { id, value, disabled, oninput }
            if let Some(description) = description { FieldDescription { "{description}" } }
            if let Some(error) = error { FieldError { "{error}" } }
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
) -> Element {
    rsx! {
        Field { variant: FieldVariant::Horizontal, invalid: error.is_some(), disabled,
            Checkbox { id: id.clone(), checked: Some(checked), disabled, on_checked_change }
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
        Collapsible { open: Some(open), on_open_change,
            CollapsibleTrigger { "{title}" }
            CollapsibleContent { FieldGroup { {children} } }
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
            }
        }
    }

    #[test]
    fn compositions_render_on_host() {
        let mut dom = VirtualDom::new(fixture);
        dom.rebuild_in_place();
    }
}
