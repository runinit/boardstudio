//! PCB-owned workspace surface composition.
use super::objects;
use super::pcb_scene::{PcbPartHit, PcbPartPointerDown, PcbScene};
use super::workspace_composition::{CanvasEventHandlers, SharedObjectsInput};
use boardstudio_application::{AcceptedSnapshot, Scope};
use boardstudio_web_ui_shared::inspector_fields::InspectorSection;
use boardstudio_web_ui_shared::ui::toggle_group::{ToggleGroup, ToggleItem};
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
    pub(super) has_selection_context: bool,
    pub(super) show_relationships: bool,
    pub(super) on_show_relationships: EventHandler<()>,
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
    pub(super) on_place_component: EventHandler<super::part_placement::ComponentPlacementAction>,
    pub(super) layout_target: Signal<Option<String>>,
    pub(super) parts_query: super::parts::PartsQuery,
    pub(super) on_browse_parts: EventHandler<()>,
    pub(super) placement_error: Option<String>,
}

pub(super) fn objects(i: ObjectsInput) -> Element {
    rsx! {
        div {
            objects::Objects {
                selected_context: i.shared.selected_context, on_select: i.shared.on_select, on_navigate: i.shared.on_navigate,
                on_nudge: i.shared.on_nudge, board_setup: Some(i.shared.board_setup), matrix_setup: Some(i.matrix_setup),
                pair_created: None, on_place_component: Some(i.on_place_component), layout_target: Some(i.layout_target),
                parts_query: Some(i.parts_query), on_browse_parts: Some(i.on_browse_parts), placement_error: i.placement_error,
                matrix_inspector: None, add_object_menu_open: None,
            }
        }
    }
}

pub(super) fn toolbar(i: ToolbarInput) -> Element {
    rsx! {
        div { class: "pointer-events-none absolute inset-0 z-[12] [container:layout-toolbar/inline-size]", "data-workspace-toolbar": "pcb",
            div { class: "pointer-events-none absolute inset-x-3 top-3 max-[760px]:inset-x-2 max-[760px]:top-2",
                objects::LayoutCommandPill {
                    command_label: i.command_label, menu_owner_key: i.menu_owner_key, open_menu: i.open_menu,
                    transform: i.transform, align: i.align, selection_kind: i.selection_kind, snap_settings: i.snap_settings,
                    on_selection_kind: i.on_selection_kind, on_snap_intent: i.on_snap_intent,
                    has_selection_context: i.has_selection_context, show_relationships: i.show_relationships,
                    on_show_relationships: i.on_show_relationships,
                }
            }
        }
    }
}

pub(super) fn canvas(i: CanvasInput) -> Element {
    let h = i.handlers;
    rsx! {
        section { class: "flex min-h-0 min-w-0 flex-1", "aria-label": "PCB workspace",
            svg {
                class: "block h-full min-h-[250px] min-w-0 w-full flex-1 touch-none bg-canvas max-[760px]:min-h-0",
                view_box: "{i.view_box}", preserve_aspect_ratio: "xMidYMid meet", tabindex: "0", role: "group",
                "aria-label": "PCB layout; select a part with click, Enter, or Space, hold Space and drag to pan, or use the mouse wheel to zoom",
                onmounted: h.mount,
                onpointerdown: move |e: PointerEvent| { h.start_pan.call(e.clone()); i.on_empty_hit.call(e); },
                onpointermove: h.move_pointer, onpointerup: h.end_pointer, onpointercancel: h.cancel_pointer,
                onlostpointercapture: h.cancel_pointer, onkeydown: h.keyboard, onkeyup: h.key_up, onwheel: h.wheel,
                g { transform: "scale(1,-1)",
                    PcbScene {
                        snapshot: i.snapshot, scope: i.scope, selected_ids: i.selected_ids, generation: i.generation,
                        on_part_hit: i.on_part_hit, on_part_pointer_down: i.on_part_pointer_down, on_module_select: i.on_module_select,
                    }
                }
            }
        }
    }
}

pub(super) fn inspector() -> Element {
    rsx! {}
}

#[component]
pub(super) fn PcbInspector(selected: bool, children: Element) -> Element {
    let mut wiring = use_signal(|| true);
    rsx! {
        section { aria_label: "PCB inspector",
            ToggleGroup { class: "mb-4 flex gap-0 border-b border-border", aria_label: "PCB inspector mode", horizontal: true,
                pressed: Some([if wiring() { 1usize } else { 0usize }].into_iter().collect()),
                on_pressed_change: move |p: std::collections::HashSet<usize>| {
                    if p.contains(&1) { wiring.set(true); } else if p.contains(&0) && selected { wiring.set(false); }
                },
                ToggleItem { index: 0usize, disabled: !selected, "Selection" }
                ToggleItem { index: 1usize, "Routing & nets" }
            }
            div { hidden: wiring(), inert: wiring().then_some(""),
                InspectorSection { title: "Position", default_open: true, super::inspector::Inspector {} }
            }
            div { hidden: !wiring(), inert: (!wiring()).then_some(""), {children} }
        }
    }
}
