//! X/Y/Z presentation shared by the assembly and component model editors.
//! Structure lives once in `ui-shared::inspector_fields`; owners keep drafts,
//! validation and commits.
use boardstudio_web_ui_shared::inspector_fields::{AxisField, AxisGrid};
use dioxus::prelude::*;

#[component]
pub(super) fn ModelAxisGrid(children: Element) -> Element {
    rsx! { AxisGrid { {children} } }
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
        AxisField { axis, value, aria_label, disabled, min, error, oninput, onblur, onkeydown }
    }
}
