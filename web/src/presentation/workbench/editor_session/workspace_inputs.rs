use super::super::*;
use boardstudio_web_ui_model::{Workspace, editor::EditorContext};

pub(in crate::presentation) struct WsWiring {
    pub mounted_module_select: EventHandler<String>,
    pub layout_snap_intent: EventHandler<objects::LayoutSnapIntent>,
    pub on_browse_parts: EventHandler<()>,
    pub on_empty_board_matrix: Callback<()>,
    pub shared_objects: workspace_composition::SharedObjectsInput,
    pub on_open_module_placement: EventHandler<String>,
    pub on_module_attached: EventHandler<parts::AttachedModuleNavigation>,
}

pub(super) fn use_workspace_wiring(
    state: &super::session_state::SessionState,
    matrix_setup: &objects::MatrixSetupMount,
    mut workspace_routing: WorkspaceRoutingSlots,
    sel: super::selection_handlers::SelHandlers,
    select_tree: impl FnMut(objects::TreeSelectRequest) + 'static,
    navigate: super::findings_navigation::NavigateFn,
) -> WsWiring {
    let snapshot = state.snapshot;
    let render_scope = state.render_scope;
    let super::selection_handlers::SelHandlers {
        on_layout_snap_intent,
        on_mounted_module_select,
        nudge_tree,
        on_open_module_placement,
        on_module_attached,
    } = sel;
    let editor = use_context::<EditorContext>();
    let runtime = &editor.runtime();
    let adapter = &state.adapter;
    let workspace = editor.workspace;
    let layout_assembly_3d = state.layout_state.assembly_3d();
    let render_generation = state.render_generation;
    let active_workspace = state.active_workspace;
    let objects_panel_settings = state.objects_panel_settings;
    workspace_routing.select_tree.replace(Box::new(select_tree));
    workspace_routing.navigate.replace(navigate);
    let shared_objects = workspace_composition::SharedObjectsInput {
        selected_context: adapter.selected_context,
        on_select: workspace_routing.select_tree,
        on_navigate: workspace_routing.navigate,
        on_nudge: nudge_tree,
        board_setup: objects::use_board_setup(
            runtime.clone(),
            editor.version,
            workspace,
            adapter.generation,
            workspace_routing.navigate,
        ),
    };
    let panels = editor.panels;
    let scope_generation = adapter.generation;
    let on_browse_parts = EventHandler::new(move |_| {
        browse_parts_workspace(
            workspace,
            panels.objects_open,
            objects_panel_settings,
            is_compact_viewport(),
        );
    });
    let on_empty_board_matrix = use_callback({
        let runtime = runtime.clone();
        let owner_scope = render_scope.clone();
        let owner_token = snapshot.token;
        let owner_revision = snapshot.document.revision;
        let generation = scope_generation;
        let mut workspace = workspace;
        let on_open = matrix_setup.on_open;
        move |_| {
            let model = runtime.model();
            if workspace() != active_workspace
                || generation() != render_generation
                || runtime.scope().as_ref() != Some(&owner_scope)
                || model.lifecycle != Lifecycle::Ready
                || !matches!(model.durability, Durability::Saved { revision } | Durability::Saving { revision } if revision == owner_revision)
                || model.gesture.is_some()
                || model.display_preview.is_some()
                || !model.accepted.as_ref().is_some_and(|accepted| {
                    accepted.token == owner_token && accepted.document.revision == owner_revision
                })
            {
                return;
            }
            workspace.set(Workspace::Layout);
            let mut layout_assembly_3d = layout_assembly_3d;
            layout_assembly_3d.set(false);
            on_open.call(());
        }
    });
    WsWiring {
        mounted_module_select: on_mounted_module_select,
        layout_snap_intent: on_layout_snap_intent,
        on_browse_parts,
        on_empty_board_matrix,
        shared_objects,
        on_open_module_placement,
        on_module_attached,
    }
}
