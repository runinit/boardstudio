//! Workbench-wide keyboard actions that belong to presentation navigation.
use crate::runtime::Runtime;
use boardstudio_application::Event;
use dioxus::prelude::KeyboardEvent;
use dioxus_web::WebEventExt;
use std::rc::Rc;
use wasm_bindgen::JsCast;

/// Mirror the React workbench's history shortcuts for focused shell controls as well as canvas.
/// Canvas-specific handling runs first and marks its event prevented, so it remains the owner
/// when the canvas itself is focused and this boundary does not submit the action twice.
pub(crate) fn handle_history_shortcut(runtime: Rc<Runtime>, event: KeyboardEvent) {
    let Some(raw) = event.data().try_as_web_event() else {
        return;
    };
    if raw.default_prevented() || is_typing_target(&raw) {
        return;
    }
    if !raw.ctrl_key() && !raw.meta_key() {
        return;
    }
    let key = raw.key();
    let operation = match key.to_ascii_lowercase().as_str() {
        "z" if raw.shift_key() => Event::Redo {
            operation_id: runtime.operation(),
        },
        "z" => Event::Undo {
            operation_id: runtime.operation(),
        },
        "y" => Event::Redo {
            operation_id: runtime.operation(),
        },
        _ => return,
    };
    event.prevent_default();
    runtime.submit(operation);
}

fn is_typing_target(event: &web_sys::KeyboardEvent) -> bool {
    event
        .target()
        .and_then(|target| target.dyn_into::<web_sys::HtmlElement>().ok())
        .is_some_and(|target| matches!(target.tag_name().as_str(), "INPUT" | "TEXTAREA" | "SELECT"))
}
