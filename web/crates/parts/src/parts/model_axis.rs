//! X/Y/Z presentation shared by the assembly and component model editors.
//! Each owner keeps its own drafts, validation and commits.
use boardstudio_web_ui_shared::controls::BoardNumberField;
use dioxus::prelude::*;

#[component]
pub(super) fn ModelAxisGrid(children: Element) -> Element {
    rsx! { div { class: "m1-parts-assembly-transform m1-model-axes", {children} } }
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
    rsx! {
        BoardNumberField {
            label: axis, value, aria_label, disabled, min, error,
            step: "0.1",
            oninput, onblur, onkeydown,
        }
    }
}
