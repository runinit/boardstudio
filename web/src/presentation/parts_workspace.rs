//! Parts-owned Objects, canvas placeholder and Inspector composition.
use super::parts::{
    PartsInspectorPanel, PartsLibraryPanel, PartsPreviewWorkspace, PartsQuery, PartsSelection,
};
use boardstudio_application::Scope;
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

pub(super) struct PartsFrame {
    pub(crate) matrix_placement_busy: bool,
    pub(crate) on_place_assembly: EventHandler<super::objects::MatrixPlacementSource>,
    pub(crate) on_open_module_placement: EventHandler<String>,
    pub(crate) on_module_attached: EventHandler<super::parts::AttachedModuleNavigation>,
}

#[component]
pub(super) fn PartsWorkspace() -> Element {
    use super::workspace_composition::{WorkspaceSurfaces, publish_surfaces, use_frame};
    use boardstudio_web_ui_model::editor::EditorContext;

    let frame = use_frame();
    let parts = frame.parts.as_ref().expect("Parts workspace frame");
    let editor = use_context::<EditorContext>();
    let state = use_context::<PartsWorkspaceState>();
    let layout = frame.status.layout;
    let placement = &frame.part_placement;
    let scope = frame.owner.scope.clone();
    let snapshot = &frame.snapshot;
    let objects = rsx! {
        PartsLibraryPanel {
            snapshot: snapshot.clone(),
            scope: scope.clone(),
            scope_generation: editor.scope_generation,
            workspace: editor.workspace,
            query: state.query(),
            selected: state.selected(),
            on_select: move |_| {
                let mut panels = editor.panels;
                panels.objects_open.set(false);
                panels.inspector_open.set(true);
            },
        }
    };
    let inspector = rsx! {
        PartsInspectorPanel {
            snapshot: snapshot.clone(),
            scope: scope.clone(),
            query: state.query(),
            selected: state.selected(),
            selected_context: frame.objects.selected_context,
            on_place_controller: placement.on_place_controller,
            on_place_component: placement.on_place_component,
            controller_placement_enabled: placement.controller_placement_enabled,
            placement_busy: placement.busy || parts.matrix_placement_busy,
            placement_error: frame.placement_error.clone(),
            layout_target: frame.layout_target,
            on_place_assembly: parts.on_place_assembly,
            on_board_placed: move |_| layout.selection_kind().set(super::objects::LayoutSelectionKind::Part),
            on_open_module_placement: parts.on_open_module_placement,
            on_module_attached: parts.on_module_attached,
        }
    };
    let canvas = rsx! {
        if let Some(on_back) = placement.controller_back {
            Button { variant: ButtonVariant::Secondary, onclick: move |_| on_back.call(()), "Back to PCB" }
        }
        PartsPreviewWorkspace { snapshot: snapshot.clone(), scope, query: state.query(), selected: state.selected() }
    };
    publish_surfaces(
        use_context(),
        Workspace::Parts,
        WorkspaceSurfaces {
            objects,
            inspector,
            canvas,
            toolbar: rsx! {},
            status: rsx! {},
            layers: rsx! {},
        },
    );
    rsx! {}
}
