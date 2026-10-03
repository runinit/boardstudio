//! PCB-owned workspace surface composition.
use super::objects;
use super::pcb_layers::PcbLayerControls;
use super::pcb_scene::{PcbPartHit, PcbPartPointerDown, PcbScene};
use super::workspace_composition::{CanvasEventHandlers, SharedObjectsInput};
use boardstudio_application::{AcceptedSnapshot, Scope};
use dioxus::prelude::*;

pub(super) struct ToolbarInput {
    pub(super) command_label: String,
    pub(super) menu_owner_key: String,
    pub(super) selection_kind: objects::LayoutSelectionKind,
    pub(super) snap_settings: objects::LayoutSnapSettings,
    pub(super) transform: objects::LayoutTransformMenuMount,
    pub(super) align: objects::LayoutAlignMount,
    pub(super) on_selection_kind: EventHandler<objects::LayoutSelectionKind>,
    pub(super) on_snap_intent: EventHandler<objects::LayoutSnapIntent>,
}

pub(super) struct CanvasInput {
    pub(super) snapshot: AcceptedSnapshot,
    pub(super) scope: Scope,
    pub(super) view_box: String,
    pub(super) selected_ids: Vec<String>,
    pub(super) generation: u64,
    pub(super) handlers: CanvasEventHandlers,
    pub(super) on_empty_hit: EventHandler<PointerEvent>,
    pub(super) on_part_hit: EventHandler<PcbPartHit>,
    pub(super) on_part_pointer_down: EventHandler<PcbPartPointerDown>,
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
            on_place_component: None,
            layout_target: None,
            parts_query: None,
            on_browse_parts: None,
            placement_error: None,
        }
    }
}

pub(super) fn toolbar(input: ToolbarInput) -> Element {
    rsx! {
        div { class: "m1-canvas-toolbar",
            objects::LayoutCommandPill {
                command_label: input.command_label,
                menu_owner_key: input.menu_owner_key,
                transform: input.transform,
                align: input.align,
                selection_kind: input.selection_kind,
                snap_settings: input.snap_settings,
                on_selection_kind: input.on_selection_kind,
                on_snap_intent: input.on_snap_intent,
            }
        }
    }
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
                onpointerdown: move |event: PointerEvent| {
                    handlers.start_pan.call(event.clone());
                    input.on_empty_hit.call(event);
                },
                onpointermove: handlers.move_pointer,
                onpointerup: handlers.end_pointer,
                onpointercancel: handlers.cancel_pointer,
                onlostpointercapture: handlers.cancel_pointer,
                onkeydown: handlers.keyboard,
                onkeyup: handlers.key_up,
                onwheel: handlers.wheel,
                g { transform: "scale(1,-1)",
                    PcbScene {
                        snapshot: input.snapshot.clone(),
                        scope: input.scope.clone(),
                        selected_ids: input.selected_ids,
                        generation: input.generation,
                        on_part_hit: input.on_part_hit,
                        on_part_pointer_down: input.on_part_pointer_down,
                    }
                }
            }
            PcbLayerControls {
                snapshot: input.snapshot.clone(),
                scope: input.scope.clone(),
            }
        }
    }
}

pub(super) fn inspector() -> Element {
    rsx! {}
}
