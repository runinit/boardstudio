//! Keycaps-owned workspace surface composition.
use super::objects;
use super::keycaps_scene::{
    KeycapsCanvas, KeycapsKeyList, KeycapsMatrixList, KeycapsSelectedSummary, KeycapsView,
};
use super::workspace_composition::{CanvasEventHandlers, SharedObjectsInput};
use boardstudio_core::model::Contour;
use dioxus::prelude::*;
use std::{collections::BTreeSet, rc::Rc};

pub(super) struct CanvasInput {
    pub(super) view: Option<Rc<KeycapsView>>,
    pub(super) contours: Rc<[Contour]>,
    pub(super) view_box: String,
    pub(super) selected_ids: BTreeSet<String>,
    pub(super) selected_key_id: Option<String>,
    pub(super) handlers: CanvasEventHandlers,
    pub(super) on_select_key: EventHandler<String>,
}

pub(super) struct InspectorInput {
    pub(super) view: Option<Rc<KeycapsView>>,
    pub(super) selected_key_id: Option<String>,
}

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

pub(super) fn canvas(input: CanvasInput) -> Element {
    let handlers = input.handlers;
    if let Some(view) = input.view {
        rsx! {
            section { class: "m1-keycaps-workspace", "aria-label": "Keycaps workspace",
                div { class: "m1-keycaps-controls",
                    KeycapsKeyList {
                        view: view.clone(),
                        selected_key_id: input.selected_key_id.clone(),
                        on_select_key: input.on_select_key,
                    }
                    KeycapsMatrixList { view: view.clone() }
                }
                svg {
                    class: "m1-canvas m1-keycaps-canvas",
                    view_box: "{input.view_box}",
                    preserve_aspect_ratio: "xMidYMid meet",
                    tabindex: "0",
                    role: "group",
                    "aria-label": "Physical keycaps; select a key with click, Enter, or Space, hold Space and drag to pan, or use the mouse wheel to zoom",
                    onmounted: handlers.mount,
                    onpointerdown: handlers.start_pan,
                    onpointermove: handlers.move_pointer,
                    onpointerup: handlers.end_pointer,
                    onpointercancel: handlers.cancel_pointer,
                    onlostpointercapture: handlers.cancel_pointer,
                    onkeydown: handlers.keyboard,
                    onkeyup: handlers.key_up,
                    onwheel: handlers.wheel,
                    g { transform: "scale(1,-1)",
                        KeycapsCanvas {
                            view,
                            contours: input.contours,
                            selected_ids: input.selected_ids,
                            on_select_key: input.on_select_key,
                        }
                    }
                }
            }
        }
    } else {
        rsx! {
            p { class: "m1-keycaps-empty-note", role: "status", "Keycaps are unavailable for the current board." }
        }
    }
}

pub(super) fn inspector(input: InspectorInput) -> Element {
    if let Some(view) = input.view {
        rsx! {
            KeycapsSelectedSummary {
                view,
                selected_key_id: input.selected_key_id,
            }
        }
    } else {
        rsx! {
            section { class: "m1-keycaps-selected-summary", "aria-label": "Selected keycap",
                h2 { "Keycaps unavailable" }
                p { "The current board has no Keycaps projection." }
            }
        }
    }
}
