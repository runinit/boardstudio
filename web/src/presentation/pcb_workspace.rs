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
    pub(super) open_menu: Signal<Option<objects::LayoutCommandMenu>>,
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
    pub(super) on_module_select: EventHandler<String>,
}

pub(super) struct ObjectsInput {
    pub(super) shared: SharedObjectsInput,
    pub(super) matrix_setup: objects::MatrixSetupMount,
    pub(super) mirrored_pair: objects::MirroredPairMount,
    pub(super) on_place_component: EventHandler<super::part_placement::ComponentPlacementAction>,
    pub(super) layout_target: Signal<Option<String>>,
    pub(super) parts_query: super::parts::PartsQuery,
    pub(super) on_browse_parts: EventHandler<()>,
    pub(super) placement_error: Option<String>,
}

pub(super) fn objects(input: ObjectsInput) -> Element {
    rsx! {
        objects::Objects {
            selected_context: input.shared.selected_context,
            on_select: input.shared.on_select,
            on_navigate: input.shared.on_navigate,
            on_nudge: input.shared.on_nudge,
            on_open_geometry_scripts: input.shared.on_open_geometry_scripts,
            board_setup: Some(input.shared.board_setup),
            matrix_setup: Some(input.matrix_setup),
            mirrored_pair: Some(input.mirrored_pair),
            pair_created: None,
            on_place_component: Some(input.on_place_component),
            layout_target: Some(input.layout_target),
            parts_query: Some(input.parts_query),
            on_browse_parts: Some(input.on_browse_parts),
            placement_error: input.placement_error,
            matrix_inspector: None,
        }
    }
}

pub(super) fn toolbar(input: ToolbarInput) -> Element {
    rsx! {
        div { class: "m1-canvas-toolbar",
            objects::LayoutCommandPill {
                command_label: input.command_label,
                menu_owner_key: input.menu_owner_key,
                open_menu: input.open_menu,
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
                        on_module_select: input.on_module_select,
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
