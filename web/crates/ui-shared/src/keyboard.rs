//! Shared keyboard-event guards for canvas-adjacent controls.
use dioxus::prelude::KeyboardEvent;
use dioxus_web::WebEventExt;

/// Keep Space activation on controls from starting canvas pan: native control
/// activation must not become the canvas's Space-pan shortcut.
pub fn stop_space_pan(event: KeyboardEvent) {
    if event.key().to_string() == " " {
        event.stop_propagation();
        if let Some(raw) = event.data().try_as_web_event() {
            raw.stop_propagation();
        }
    }
}
