//! Parts-owned Objects, canvas placeholder and Inspector composition.
use super::parts::{PartsInspectorPanel, PartsLibraryPanel, PartsPreviewWorkspace};
use super::parts::{PartsQuery, PartsSelection};
use super::workspace_composition::PlaceholderInput;
use boardstudio_application::{AcceptedSnapshot, Scope};
use dioxus::prelude::*;

pub(super) struct ObjectsInput {
    pub(super) snapshot: AcceptedSnapshot,
    pub(super) scope: Option<Scope>,
    pub(super) scope_generation: Signal<u64>,
    pub(super) workspace: Signal<&'static str>,
    pub(super) query: PartsQuery,
    pub(super) selected: PartsSelection,
    pub(super) on_select: EventHandler<()>,
}

pub(super) struct InspectorInput {
    pub(super) snapshot: AcceptedSnapshot,
    pub(super) scope: Option<Scope>,
    pub(super) query: PartsQuery,
    pub(super) selected: PartsSelection,
    pub(super) selected_context: Signal<Option<super::objects::ScopedTreeContext>>,
    pub(super) on_place_controller: EventHandler<String>,
    pub(super) on_place_component: EventHandler<super::part_placement::ComponentPlacementAction>,
    pub(super) controller_placement_enabled: bool,
    pub(super) placement_busy: bool,
    pub(super) placement_error: Option<String>,
    pub(super) layout_target: Signal<Option<String>>,
    pub(super) on_place_assembly: EventHandler<super::objects::MatrixPlacementSource>,
    pub(super) on_board_placed: EventHandler<()>,
    pub(super) on_open_module_placement: EventHandler<String>,
    pub(super) on_module_attached: EventHandler<super::parts::AttachedModuleNavigation>,
}

pub(super) struct CanvasInput {
    pub(super) controller_back: Option<EventHandler<()>>,
    pub(super) snapshot: AcceptedSnapshot,
    pub(super) scope: Option<Scope>,
    pub(super) query: PartsQuery,
    pub(super) selected: PartsSelection,
}

pub(super) fn objects(input: ObjectsInput) -> Element {
    rsx! {
        PartsLibraryPanel {
            snapshot: input.snapshot,
            scope: input.scope,
            scope_generation: input.scope_generation,
            workspace: input.workspace,
            query: input.query,
            selected: input.selected,
            on_select: input.on_select,
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

pub(super) fn preview(input: CanvasInput) -> Element {
    rsx! {
        if let Some(on_back) = input.controller_back {
            button { class: "m1-parts-controller-back", type: "button", onclick: move |_| on_back.call(()), "Back to PCB" }
        }
        PartsPreviewWorkspace {
            snapshot: input.snapshot,
            scope: input.scope,
            query: input.query,
            selected: input.selected,
        }
    }
}

pub(super) fn inspector(input: InspectorInput) -> Element {
    rsx! {
        PartsInspectorPanel {
            snapshot: input.snapshot,
            scope: input.scope,
            query: input.query,
            selected: input.selected,
            selected_context: input.selected_context,
            on_place_controller: input.on_place_controller,
            on_place_component: input.on_place_component,
            controller_placement_enabled: input.controller_placement_enabled,
            placement_busy: input.placement_busy,
            placement_error: input.placement_error,
            layout_target: input.layout_target,
            on_place_assembly: input.on_place_assembly,
            on_board_placed: input.on_board_placed,
            on_open_module_placement: input.on_open_module_placement,
            on_module_attached: input.on_module_attached,
        }
    }
}
