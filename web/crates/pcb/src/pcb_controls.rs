//! Private compositions of the shared controls for PCB inspectors.
use boardstudio_web_ui_shared::inspector_fields::TextField;
use boardstudio_web_ui_shared::ui::button::{Button, ButtonVariant};
use boardstudio_web_ui_shared::ui::input::InputType;
use dioxus::prelude::*;
use wasm_bindgen::JsCast;

/// Text input for staged forms that accept a blank value.
#[component]
pub(crate) fn PcbInput(
    label: String,
    aria_label: String,
    value: String,
    r#type: InputType,
    #[props(default)] placeholder: Option<String>,
    #[props(default)] disabled: bool,
    oninput: EventHandler<FormEvent>,
) -> Element {
    let id = format!("pcb-input-{}", dioxus::core::current_scope_id().0);
    let aria_label = Some(aria_label);
    rsx! {
        TextField { id, label, value, r#type, aria_label, placeholder, disabled, oninput }
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
