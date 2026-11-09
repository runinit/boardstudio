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
    on_value_change: EventHandler<Option<String>>,
) -> Element {
    let id = format!("choice-field-{}", dioxus::core::current_scope_id().0);
    let current = use_memo(use_reactive((&value,), |(value,)| Some(value)));
    let name = aria_label.unwrap_or_else(|| label.clone());
    let narrow = layout == ChoiceFieldLayout::StackedNarrow;
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
    let option_class = if narrow {
        ""
    } else {
        "max-[960px]:min-h-control-touch [@media(pointer:coarse)]:min-h-control-touch"
    };
    if options.len() > 12 {
        let placeholder = if narrow { label.clone() } else { String::new() };
        let combo = rsx! {
            Combobox::<String> {
                class: combo_class, aria_label: name.clone(), list_aria_label: name,
                placeholder, value: Some(current.into()), disabled, on_value_change,
                for (index, (value, text)) in options.into_iter().enumerate() {
                    ComboboxOption::<String> { class: option_class, index, value, text_value: text.clone(), "{text}" }
                }
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
        rsx! {
            Field { class: field_class, disabled: field_disabled,
                FieldLabel { html_for: id.clone(), "{label}" }
                Select::<String> {
                    class: select_class, id, aria_label: name, value: Some(current.into()), disabled,
                    on_value_change,
                    for (index, (value, text)) in options.into_iter().enumerate() {
                        SelectOption::<String> { class: option_class, index, value, text_value: text.clone(), "{text}" }
                    }
                }
            }
        }
    }
}
