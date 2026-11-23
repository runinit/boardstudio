//! Parts-owned Objects, canvas placeholder and Inspector composition.
use super::parts::{
    PartsInspectorPanel, PartsLibraryPanel, PartsPreviewWorkspace, PartsQuery, PartsSelection,
};
use boardstudio_application::{AcceptedSnapshot, Scope};
use boardstudio_web_ui_model::Workspace;
use boardstudio_web_ui_shared::ui::button::{Button, ButtonVariant};
use dioxus::prelude::*;

#[derive(Clone, Copy)]
pub(super) struct PartsWorkspaceState {
    query: PartsQuery,
    selected: PartsSelection,
    assembly_selection: Signal<Option<super::parts::MatrixPresetId>>,
    assembly_orientation: Signal<super::parts::SwitchOrientation>,
    selection_generation: Signal<u64>,
    preview_activation: Signal<u64>,
    generator_draft: Signal<Option<super::parts::GeneratorPreviewDraft>>,
}

pub(super) fn use_parts_workspace_state() -> PartsWorkspaceState {
    let state = PartsWorkspaceState {
        query: use_signal(String::new),
        selected: use_signal(|| None),
        assembly_selection: use_signal(|| None),
        assembly_orientation: use_signal(|| super::parts::SwitchOrientation::South),
        selection_generation: use_signal(|| 0),
        preview_activation: use_signal(|| 0),
        generator_draft: use_signal(|| None),
    };
    use_context_provider(|| super::parts::PartsAssemblySelection(state.assembly_selection));
    use_context_provider(|| super::parts::PartsAssemblyOrientation(state.assembly_orientation));
    use_context_provider(|| super::parts::PartsSelectionGeneration(state.selection_generation));
    use_context_provider(|| super::parts::PartsPreviewActivation(state.preview_activation));
    use_context_provider(|| super::parts::GeneratorDraftStore(state.generator_draft));
    state
}

impl PartsWorkspaceState {
    pub(super) fn query(self) -> PartsQuery {
        self.query
    }
    pub(super) fn selected(self) -> PartsSelection {
        self.selected
    }
    pub(super) fn assembly_orientation(self) -> Signal<super::parts::SwitchOrientation> {
        self.assembly_orientation
    }
    pub(super) fn selection_generation(self) -> Signal<u64> {
        self.selection_generation
    }
    pub(super) fn select_mounted_module(mut self, scope: Scope, definition_id: &str) {
        self.selected
            .set(Some((Some(scope), format!("module:{definition_id}"))));
        self.selection_generation
            .with_mut(|g| *g = g.wrapping_add(1));
        self.preview_activation.with_mut(|a| *a = a.wrapping_add(1));
    }
}

pub(super) struct ObjectsInput {
    pub(super) snapshot: AcceptedSnapshot,
    pub(super) scope: Option<Scope>,
    pub(super) scope_generation: Signal<u64>,
    pub(super) workspace: Signal<Workspace>,
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

pub(super) fn objects(i: ObjectsInput) -> Element {
    rsx! { PartsLibraryPanel { snapshot: i.snapshot, scope: i.scope, scope_generation: i.scope_generation, workspace: i.workspace, query: i.query, selected: i.selected, on_select: i.on_select } }
}
pub(super) fn toolbar() -> Element {
    rsx! {}
}
pub(super) fn preview(i: CanvasInput) -> Element {
    rsx! {
        if let Some(on_back) = i.controller_back { Button { variant: ButtonVariant::Secondary, onclick: move |_| on_back.call(()), "Back to PCB" } }
        PartsPreviewWorkspace { snapshot: i.snapshot, scope: i.scope, query: i.query, selected: i.selected }
    }
}
pub(super) fn inspector(i: InspectorInput) -> Element {
    rsx! {
        PartsInspectorPanel {
            snapshot: i.snapshot, scope: i.scope, query: i.query, selected: i.selected, selected_context: i.selected_context,
            on_place_controller: i.on_place_controller, on_place_component: i.on_place_component,
            controller_placement_enabled: i.controller_placement_enabled, placement_busy: i.placement_busy,
            placement_error: i.placement_error, layout_target: i.layout_target, on_place_assembly: i.on_place_assembly,
            on_board_placed: i.on_board_placed, on_open_module_placement: i.on_open_module_placement, on_module_attached: i.on_module_attached,
        }
    }
}
