//! Shared short/long choice field composition. Owners retain accepted values and edits.
use crate::ui::{
    combobox::{Combobox, ComboboxOption},
    field::{Field, FieldLabel},
    label::Label,
    select::{Select, SelectOption},
};
use dioxus::prelude::*;

/// Existing field layouts and touch breakpoints for the shared choice control.
#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub enum ChoiceFieldLayout {
    /// Normal short field; long-field label wraps the input (960px touch breakpoint).
    #[default]
    Wrapped,
    /// As Wrapped, with tighter spacing and disabled styling on the short field.
    WrappedTight,
    /// Separate label and placeholder for the long field (760px touch breakpoint).
    StackedNarrow,
    /// As StackedNarrow, with touch sizing on the options as well as the trigger.
    StackedNarrowTouchOptions,
}

/// Option identity and placeholder behavior for a controlled choice field.
#[derive(Clone, Default, PartialEq, Eq)]
pub enum ChoiceFieldMode {
    /// Zero-based options in positional mount order.
    #[default]
    Standard,
    /// Key by position and reserve Select index zero for an optional empty-value choice.
    Indexed { placeholder: Option<String> },
    /// Key by value and keep the prompt as the controlled selection. The caller handles
    /// each choice as an action.
    Prompt { placeholder: String },
}

/// Select through twelve options; above that, a filterable Combobox.
#[component]
pub fn ChoiceField(
    label: String,
    value: String,
    options: Vec<(String, String)>,
    #[props(default)] aria_label: Option<String>,
    #[props(default)] disabled: bool,
    #[props(default)] layout: ChoiceFieldLayout,
    #[props(default)] mode: ChoiceFieldMode,
    on_value_change: EventHandler<Option<String>>,
) -> Element {
    let id = format!("choice-field-{}", dioxus::core::current_scope_id().0);
    let long = options.len() > 12;
    let selected = match mode {
        ChoiceFieldMode::Prompt { .. } if long => None,
        ChoiceFieldMode::Prompt { .. } => Some(String::new()),
        _ => Some(value),
    };
    let current = use_memo(use_reactive((&selected,), |(selected,)| selected));
    let name = aria_label.unwrap_or_else(|| label.clone());
    let narrow = matches!(
        layout,
        ChoiceFieldLayout::StackedNarrow | ChoiceFieldLayout::StackedNarrowTouchOptions
    );
    let select_class = if narrow {
        "max-[760px]:[&_[data-slot=select-trigger]]:min-h-control-touch [@media(pointer:coarse)]:[&_[data-slot=select-trigger]]:min-h-control-touch"
    } else {
        "max-[960px]:[&_[data-slot=select-trigger]]:min-h-control-touch [@media(pointer:coarse)]:[&_[data-slot=select-trigger]]:min-h-control-touch"
    };
    let combo_class = if narrow {
        "max-[760px]:[&_[data-slot=combobox-input]]:min-h-control-touch [@media(pointer:coarse)]:[&_[data-slot=combobox-input]]:min-h-control-touch"
    } else {
        "max-[960px]:[&_[data-slot=combobox-input]]:min-h-control-touch [@media(pointer:coarse)]:[&_[data-slot=combobox-input]]:min-h-control-touch"
    };
    let option_class = match layout {
        ChoiceFieldLayout::StackedNarrowTouchOptions => {
            "max-[760px]:min-h-control-touch [@media(pointer:coarse)]:min-h-control-touch"
        }
        ChoiceFieldLayout::StackedNarrow => "",
        _ => "max-[960px]:min-h-control-touch [@media(pointer:coarse)]:min-h-control-touch",
    };
    let placeholder = match &mode {
        ChoiceFieldMode::Standard => None,
        ChoiceFieldMode::Indexed { placeholder } => placeholder.clone(),
        ChoiceFieldMode::Prompt { placeholder } => Some(placeholder.clone()),
    };
    let options = options.into_iter().enumerate();
    if long {
        let placeholder = placeholder.unwrap_or_else(|| match mode {
            ChoiceFieldMode::Standard if narrow => label.clone(),
            _ => String::new(),
        });
        let options = options.map(|(index, (value, text))| {
            let key = mode.option_key(index, &value);
            if let Some(key) = key {
                rsx! { ComboboxOption::<String> { key: "{key}", class: option_class, index, value, text_value: text.clone(), "{text}" } }
            } else {
                rsx! { ComboboxOption::<String> { class: option_class, index, value, text_value: text.clone(), "{text}" } }
            }
        });
        let combo = rsx! {
            Combobox::<String> {
                class: combo_class, aria_label: name.clone(), list_aria_label: name,
                placeholder, value: Some(current.into()), disabled, on_value_change,
                {options}
            }
        };
        if narrow {
            rsx! { Field { class: "gap-1.5", disabled, FieldLabel { "{label}" } {combo} } }
        } else {
            rsx! { Field { Label { class: "flex-col items-stretch gap-3", "{label}" {combo} } } }
        }
    } else {
        let field_class = if layout == ChoiceFieldLayout::Wrapped {
            ""
        } else {
            "gap-1.5"
        };
        let field_disabled = disabled && layout != ChoiceFieldLayout::Wrapped;
        let offset = usize::from(mode != ChoiceFieldMode::Standard);
        let options = options.map(|(index, (value, text))| {
            let key = mode.option_key(index, &value);
            let index = index + offset;
            if let Some(key) = key {
                rsx! { SelectOption::<String> { key: "{key}", class: option_class, index, value, text_value: text.clone(), "{text}" } }
            } else {
                rsx! { SelectOption::<String> { class: option_class, index, value, text_value: text.clone(), "{text}" } }
            }
        });
        rsx! {
            Field { class: field_class, disabled: field_disabled,
                FieldLabel { html_for: id.clone(), "{label}" }
                Select::<String> {
                    class: select_class, id, aria_label: name, value: Some(current.into()), disabled,
                    on_value_change,
                    if let Some(text) = placeholder {
                        SelectOption::<String> { class: option_class, index: 0usize, value: String::new(), text_value: text.clone(), "{text}" }
                    }
                    {options}
                }
            }
        }
    }
}

impl ChoiceFieldMode {
    fn option_key(&self, index: usize, value: &str) -> Option<String> {
        match self {
            Self::Standard => None,
            Self::Indexed { .. } => Some(index.to_string()),
            Self::Prompt { .. } => Some(value.to_owned()),
        }
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;

    fn fixture() -> Element {
        rsx! {
            ChoiceField {
                label: "Choice", value: "a", options: vec![("a".into(), "A".into())],
                on_value_change: |_| {},
            }
        }
    }

    #[test]
    fn choice_field_renders_on_host() {
        let mut dom = VirtualDom::new(fixture);
        dom.rebuild_in_place();
    }
}
