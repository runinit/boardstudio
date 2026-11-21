//! Shared choice-row molecule for the Matrix Inspector form.
//!
//! The preset, orientation, and per-field choice rows all render the same
//! `Field` + `FieldLabel` + `Select` composition with the same
//! `layout-choice-{scope}-{label}` id scheme, differing only in label, choices,
//! and change handling. One component keeps the ids, classes, `data-slot`, and
//! ARIA attributes identical across rows.

use super::*;

#[derive(Props, Clone, PartialEq)]
pub(super) struct InspectorChoiceRowProps {
    pub id: String,
    pub label: String,
    pub aria_label: String,
    #[props(default)]
    pub field_class: Option<String>,
    pub choices: Vec<(String, String, bool)>,
    pub selected: String,
    pub disabled: bool,
    #[props(default)]
    pub invalid: bool,
    #[props(default)]
    pub describedby: Option<String>,
    pub on_value_change: EventHandler<Option<String>>,
}

#[component]
pub(super) fn InspectorChoiceRow(props: InspectorChoiceRowProps) -> Element {
    let selected = use_memo(use_reactive((&props.selected,), |(selected,)| {
        Some(selected.clone())
    }));
    rsx! {
        Field {
            class: props.field_class.clone().unwrap_or_default(),
            invalid: props.invalid,
            FieldLabel { html_for: props.id.clone(), "{props.label}" }
            Select::<String> {
                id: props.id.clone(),
                aria_label: props.aria_label.clone(),
                aria_invalid: props.invalid,
                aria_describedby: props.describedby.clone(),
                value: Some(selected.into()),
                disabled: props.disabled,
                on_value_change: props.on_value_change,
                for (index, (value, label, disabled)) in props.choices.iter().enumerate() {
                    SelectOption::<String> {
                        index,
                        value: value.clone(),
                        text_value: label.clone(),
                        disabled: *disabled,
                        "{label}"
                    }
                }
            }
        }
    }
}
