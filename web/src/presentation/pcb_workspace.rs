//! PCB-owned workspace surface composition.
use super::objects;
use super::pcb_scene::{PcbPartHit, PcbScene};
use super::workspace_composition::{CanvasEventHandlers, SharedObjectsInput};
use boardstudio_application::{AcceptedSnapshot, Scope};
use dioxus::prelude::*;

pub(super) struct CanvasInput {
    pub(super) snapshot: AcceptedSnapshot,
    pub(super) scope: Scope,
    pub(super) view_box: String,
    pub(super) selected_ids: Vec<String>,
    pub(super) generation: u64,
    pub(super) handlers: CanvasEventHandlers,
    pub(super) on_empty_hit: EventHandler<MouseEvent>,
    pub(super) on_part_hit: EventHandler<PcbPartHit>,
}

pub(super) fn objects(input: SharedObjectsInput) -> Element {
    rsx! {
        objects::Objects {
            selected_context: input.selected_context,
            on_select: input.on_select,
            on_navigate: input.on_navigate,
            on_nudge: input.on_nudge,
            matrix_setup: None,
            mirrored_pair: None,
            pair_created: None,
        }
    }
}

pub(super) fn toolbar() -> Element {
    rsx! {}
}

pub(super) fn canvas(input: CanvasInput) -> Element {
    let handlers = input.handlers;
    rsx! {
        section { class: "m1-pcb-workspace", "aria-label": "PCB workspace",
            svg {
                class: "m1-canvas m1-pcb-canvas",
                view_box: "{input.view_box}",
                preserve_aspect_ratio: "xMidYMid meet",
                tabindex: "0",
                role: "group",
                "aria-label": "PCB layout; select a part with click, Enter, or Space, hold Space and drag to pan, or use the mouse wheel to zoom",
                onmounted: handlers.mount,
                onpointerdown: handlers.start_pan,
                onpointermove: handlers.move_pointer,
                onpointerup: handlers.end_pointer,
                onpointercancel: handlers.cancel_pointer,
                onlostpointercapture: handlers.cancel_pointer,
                onkeydown: handlers.keyboard,
                onkeyup: handlers.key_up,
                onwheel: handlers.wheel,
                onclick: input.on_empty_hit,
                g { transform: "scale(1,-1)",
                    PcbScene {
                        snapshot: input.snapshot,
                        scope: input.scope,
                        selected_ids: input.selected_ids,
                        generation: input.generation,
                        on_part_hit: input.on_part_hit,
                    }
                }
            }
        }
    }
}

pub(super) fn inspector() -> Element {
    rsx! {}
}
