//! Private compositions of the shared controls for PCB inspectors.
use boardstudio_web_ui_shared::ui::collapsible::{
    Collapsible, CollapsibleContent, CollapsibleTrigger,
};
use boardstudio_web_ui_shared::ui::combobox::{Combobox, ComboboxOption};
use boardstudio_web_ui_shared::ui::field::{Field, FieldLabel};
use boardstudio_web_ui_shared::ui::input::InputType;
use boardstudio_web_ui_shared::ui::input_group::{InputGroup, InputGroupInput};
use boardstudio_web_ui_shared::ui::select::{Select, SelectOption};
use dioxus::prelude::*;

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
    #[props(default)] disabled: bool,
    #[props(default)] searchable: bool,
    #[props(default)] class: String,
    #[props(default)] label_class: String,
    on_change: EventHandler<String>,
) -> Element {
    let id = format!("pcb-choice-{}", dioxus::core::current_scope_id().0);
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
    #[props(default)] class: String,
    oninput: EventHandler<FormEvent>,
) -> Element {
    let id = format!("pcb-input-{}", dioxus::core::current_scope_id().0);
    rsx! {
        Field { class: "gap-1.5 {class}", FieldLabel { html_for: id.clone(), "{label}" }
            InputGroup { InputGroupInput { id, aria_label, value, r#type, step, min, placeholder, disabled,
                class: "max-[960px]:min-h-control-touch [@media(pointer:coarse)]:min-h-control-touch",
                oninput: move |event: FormEvent| oninput.call(event),
            } }
        }
    }
}
