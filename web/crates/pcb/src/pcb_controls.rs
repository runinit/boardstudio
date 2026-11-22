//! Private compositions of the shared controls for PCB inspectors.
use boardstudio_web_ui_shared::inspector_fields::{
    CheckboxField, DraftNumericField as NumericField, TextField,
};
use boardstudio_web_ui_shared::ui::button::{Button, ButtonVariant};
use boardstudio_web_ui_shared::ui::checkbox::CheckboxState;
use boardstudio_web_ui_shared::ui::collapsible::{
    Collapsible, CollapsibleContent, CollapsibleTrigger,
};
use boardstudio_web_ui_shared::ui::combobox::{Combobox, ComboboxOption};
use boardstudio_web_ui_shared::ui::field::{Field, FieldLabel};
use boardstudio_web_ui_shared::ui::input::InputType;
use boardstudio_web_ui_shared::ui::select::{Select, SelectOption};
use dioxus::prelude::*;
use wasm_bindgen::JsCast;

#[component]
pub(crate) fn PcbSection(
    label: String,
    #[props(default)] class: String,
    #[props(default)] default_open: bool,
    children: Element,
) -> Element {
    let mut open = use_signal(|| default_open);
    rsx! {
        Collapsible { class, open: open(), on_open_change: move |next| open.set(next), keep_mounted: true,
            CollapsibleTrigger { class: "w-full p-3 bg-muted text-foreground max-[960px]:min-h-control-touch [@media(pointer:coarse)]:min-h-control-touch", "{label}" }
            CollapsibleContent { hidden: !open(), inert: (!open()).then_some(""), {children} }
        }
    }
}

/// Each choice carries its value, visible name and disabled state.
#[component]
pub(crate) fn PcbChoice(
    label: String,
    aria_label: String,
    value: String,
    options: Vec<(String, String, bool)>,
    #[props(default)] id: Option<String>,
    #[props(default)] disabled: bool,
    #[props(default)] searchable: bool,
    #[props(default)] class: String,
    #[props(default)] label_class: String,
    on_change: EventHandler<String>,
) -> Element {
    let id = id.unwrap_or_else(|| format!("pcb-choice-{}", dioxus::core::current_scope_id().0));
    let selected = use_memo(use_reactive((&value,), |(value,)| Some(value)));
    rsx! {
        Field { class: "gap-1.5 {class}",
            FieldLabel { class: label_class, html_for: id.clone(), "{label}" }
            if searchable {
                Combobox::<String> { id, aria_label: aria_label.clone(), list_aria_label: aria_label, value: Some(selected.into()), disabled,
                    class: "max-[960px]:[&_[data-slot=combobox-input]]:min-h-control-touch [@media(pointer:coarse)]:[&_[data-slot=combobox-input]]:min-h-control-touch",
                    on_value_change: move |value: Option<String>| if let Some(value) = value { on_change.call(value); },
                    for (index, (value, label, disabled)) in options.into_iter().enumerate() {
                        ComboboxOption::<String> { index, value, disabled, text_value: label.clone(), class: "max-[960px]:min-h-control-touch [@media(pointer:coarse)]:min-h-control-touch", "{label}" }
                    }
                }
            } else {
                Select::<String> { id, aria_label, value: Some(selected.into()), disabled,
                    class: "w-full max-[960px]:[&_[data-slot=select-trigger]]:min-h-control-touch [@media(pointer:coarse)]:[&_[data-slot=select-trigger]]:min-h-control-touch",
                    on_value_change: move |value: Option<String>| if let Some(value) = value { on_change.call(value); },
                    for (index, (value, label, disabled)) in options.into_iter().enumerate() {
                        SelectOption::<String> { index, value, disabled, text_value: label.clone(), class: "max-[960px]:min-h-control-touch [@media(pointer:coarse)]:min-h-control-touch", "{label}" }
                    }
                }
            }
        }
    }
}

/// Number and text rows delegate to the shared inspector fields; a missing step keeps
/// the browser default of 1.
#[component]
pub(crate) fn PcbInput(
    label: String,
    aria_label: String,
    value: String,
    r#type: InputType,
    #[props(default)] step: Option<String>,
    #[props(default)] min: Option<String>,
    #[props(default)] placeholder: Option<String>,
    #[props(default)] disabled: bool,
    oninput: EventHandler<FormEvent>,
) -> Element {
    let id = format!("pcb-input-{}", dioxus::core::current_scope_id().0);
    let aria_label = Some(aria_label);
    rsx! {
        if r#type == InputType::Number {
            NumericField { id, label, value, aria_label, placeholder, disabled, min, step: step.unwrap_or_else(|| "1".to_owned()), oninput }
        } else {
            TextField { id, label, value, r#type, aria_label, placeholder, disabled, oninput }
        }
    }
}

#[component]
pub(crate) fn PcbCheck(
    label: String,
    checked: bool,
    #[props(default)] disabled: bool,
    on_change: EventHandler<bool>,
) -> Element {
    let id = format!("pcb-check-{}", dioxus::core::current_scope_id().0);
    rsx! {
        CheckboxField {
            id,
            aria_label: Some(label.clone()),
            label,
            checked: if checked { CheckboxState::Checked } else { CheckboxState::Unchecked },
            disabled,
            on_checked_change: move |value| on_change.call(value == CheckboxState::Checked),
        }
    }
}

/// Native file inputs stay hidden; a named shared button opens the browser picker.
#[component]
pub(crate) fn PcbFile(
    label: String,
    #[props(default)] accept: Option<String>,
    #[props(default)] directory: bool,
    #[props(default)] disabled: bool,
    onchange: EventHandler<FormEvent>,
) -> Element {
    let id = format!("pcb-file-{}", dioxus::core::current_scope_id().0);
    let trigger_id = id.clone();
    rsx! {
        Button { variant: ButtonVariant::Secondary, class: "max-[960px]:min-h-control-touch [@media(pointer:coarse)]:min-h-control-touch", disabled,
            onclick: move |_| {
                if let Some(input) = web_sys::window().and_then(|window| window.document()).and_then(|document| document.get_element_by_id(&trigger_id)).and_then(|element| element.dyn_into::<web_sys::HtmlElement>().ok()) { input.click(); }
            }, "{label}"
        }
        input { id, r#type: "file", hidden: true, accept, multiple: directory, "webkitdirectory": directory.then_some(""), disabled, onchange }
    }
}
