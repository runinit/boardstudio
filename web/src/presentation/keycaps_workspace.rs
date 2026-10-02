//! Keycaps-owned workspace surface composition.
use super::objects;
use super::workspace_composition::{PlaceholderInput, SharedObjectsInput};
use dioxus::prelude::*;

pub(super) fn objects(input: SharedObjectsInput) -> Element {
    rsx! {
        objects::Objects {
            selected_context: input.selected_context,
            on_select: input.on_select,
            on_navigate: input.on_navigate,
            on_nudge: input.on_nudge,
        }
    }
}

pub(super) fn toolbar() -> Element {
    rsx! {}
}

pub(super) fn canvas(mut input: PlaceholderInput) -> Element {
    rsx! {
        section { class: "m1-placeholder-workspace",
            h1 { "{input.name}" }
            p { "{input.message}" }
            button { onclick: move |_| input.workspace.set("Layout"), "Back to Layout" }
        }
    }
}

pub(super) fn inspector() -> Element {
    rsx! {}
}
