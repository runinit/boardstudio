use super::super::*;
use crate::presentation::editor::selection_handlers::on_snap_intent_handler;
use boardstudio_web_ui_model::Workspace;

pub(super) struct SelectTreeInput {
    pub runtime: Rc<Runtime>,
    pub adapter: SelectionAdapter,
    pub owner: LayoutOwnerIdentity,
    pub tree_cell_anchor: Rc<RefCell<Option<OwnedTreeCellAnchor>>>,
    pub generation: u64,
    pub workspace: Signal<Workspace>,
    pub inspector_settings: Signal<PanelSettings>,
    pub objects_open: Signal<bool>,
    pub inspect_open: Signal<bool>,
    pub layout_state: layout_workspace::LayoutWorkspaceState,
    pub outline_ui: outline::OutlineUiState,
}

pub(super) fn select_tree_handler(
    input: SelectTreeInput,
) -> impl FnMut(objects::TreeSelectRequest) + 'static {
    let SelectTreeInput {
        runtime,
        adapter,
        owner,
        tree_cell_anchor,
        generation,
        mut workspace,
        inspector_settings,
        mut objects_open,
        mut inspect_open,
        layout_state,
        outline_ui,
    } = input;
    move |request: objects::TreeSelectRequest| {
        if request
            .outline_action
            .as_ref()
            .is_some_and(|action| !outline::outline_action_is_current(action, &runtime, generation))
            || (adapter.generation)() != generation
            || runtime.scope().as_ref() != Some(&request.scope)
            || !selection::context_is_current(&runtime.model(), &request.scope, &request.context)
        {
            return;
        }
        let (close_objects, open_inspect) = match &request.context {
            objects::TreeContext::Board { .. } => (false, false),
            objects::TreeContext::MountedModule { .. }
            | objects::TreeContext::Outline { .. }
            | objects::TreeContext::OutlineVersion { .. }
            | objects::TreeContext::Bridge { .. } => (true, true),
            objects::TreeContext::LayoutGroup { .. } => (true, false),
            objects::TreeContext::Matrix { .. }
            | objects::TreeContext::Row { .. }
            | objects::TreeContext::Column { .. }
            | objects::TreeContext::Key { .. }
            | objects::TreeContext::Component { .. } => (true, true),
        };
        let outline_route = matches!(
            &request.context,
            objects::TreeContext::Outline { .. }
                | objects::TreeContext::OutlineVersion { .. }
                | objects::TreeContext::Bridge { .. }
        );
        let bridge_id = match &request.context {
            objects::TreeContext::Bridge { bridge_id, .. } => Some(bridge_id.clone()),
            _ => None,
        };
        let activation = request.outline_action.clone();
        let scope = request.scope.clone();
        layout_state.set_selection_kind_from_context(&request.context);
        update_tree_cell_anchor(
            &tree_cell_anchor,
            &owner,
            &runtime.model(),
            Some(&request.context),
        );
        selection::submit_context(&runtime, &adapter, request.into());
        if (adapter.generation)() != generation || runtime.scope().as_ref() != Some(&scope) {
            return;
        }
        if let Some(action) = activation {
            workspace.set(Workspace::Layout);
            outline::commit_outline_action(
                &runtime,
                outline_ui,
                adapter.selected_context,
                workspace,
                adapter.generation,
                generation,
                action,
            );
        }
        if outline_route {
            workspace.set(Workspace::Layout);
            fit_selected_bridge(&runtime, bridge_id.as_deref());
            pin_inspector_on_desktop(inspector_settings);
        }
        if close_objects {
            objects_open.set(false);
        }
        if open_inspect {
            inspect_open.set(true);
        }
    }
}

pub(super) struct SelHandlers {
    pub on_layout_snap_intent: EventHandler<objects::LayoutSnapIntent>,
    pub on_mounted_module_select: EventHandler<String>,
    pub nudge_tree: EventHandler<objects::TreeNudgeRequest>,
    pub on_open_module_placement: EventHandler<String>,
    pub on_module_attached: EventHandler<parts::AttachedModuleNavigation>,
}

pub(super) fn build_selection_handlers(
    state: &super::session_state::SessionState,
    snapshot: &AcceptedSnapshot,
    render_scope: &Scope,
) -> (
    SelHandlers,
    impl FnMut(objects::TreeSelectRequest) + 'static,
) {
    let editor = use_context::<boardstudio_web_ui_model::editor::EditorContext>();
    let runtime = &editor.runtime();
    let adapter = &state.adapter;
    let workspace = editor.workspace;
    let render_generation = state.render_generation;
    let inspect_open = editor.panels.inspector_open;
    let tree_cell_anchor = &state.tree_cell_anchor;
    let objects_open = editor.panels.objects_open;
    let instance_selection = editor.instance_selection;
    let layout_owner = &state.view.layout_owner;
    let inspector_panel_settings = state.inspector_panel_settings;
    let parts_selection = state.view.parts_selection;
    let layout_state = state.layout_state;
    let outline_ui = state.view.outline_ui;
    let layout_snap_settings = state.layout_state.snap_settings();
    let objects_panel_settings = state.objects_panel_settings;
    let parts_selection_generation = state.view.parts_selection_generation;
    let parts_state = state.parts_state;
    let select_tree = select_tree_handler(SelectTreeInput {
        runtime: runtime.clone(),
        adapter: adapter.clone(),
        owner: layout_owner.clone(),
        tree_cell_anchor: tree_cell_anchor.clone(),
        generation: render_generation,
        workspace,
        inspector_settings: inspector_panel_settings,
        objects_open,
        inspect_open,
        layout_state,
        outline_ui,
    });
    let on_layout_snap_intent = on_snap_intent_handler(
        runtime.clone(),
        adapter.clone(),
        layout_owner.clone(),
        layout_snap_settings,
        workspace,
        Workspace::Layout,
    );
    let on_mounted_module_select = super::action_handlers::on_mounted_module_select_handler(
        super::action_handlers::OnMountedModuleSelectInput {
            runtime: runtime.clone(),
            adapter: adapter.clone(),
            scope: render_scope.clone(),
            generation: render_generation,
            token: snapshot.token,
            revision: snapshot.document.revision,
            inspect_open,
            workspace,
            instance_selection,
            parts_state,
            objects_open,
            objects_panel_settings,
        },
    );
    let on_open_module_placement =
        crate::presentation::parts_workspace::on_open_module_placement_handler(
            crate::presentation::parts_workspace::OnOpenModulePlacementInput {
                runtime: runtime.clone(),
                adapter: adapter.clone(),
                scope: render_scope.clone(),
                generation: render_generation,
                token: snapshot.token,
                revision: snapshot.document.revision,
                workspace,
                objects_open,
                inspect_open,
                instance_selection,
                parts_selection,
            },
        );
    let on_module_attached = crate::presentation::parts_workspace::on_module_attached_handler(
        crate::presentation::parts_workspace::OnModuleAttachedInput {
            runtime: runtime.clone(),
            adapter: adapter.clone(),
            workspace,
            objects_open,
            inspect_open,
            parts_selection_generation,
            parts_selection,
            instance_selection,
        },
    );
    let nudge_tree = super::action_handlers::nudge_tree_handler(
        runtime.clone(),
        adapter.clone(),
        render_generation,
        inspect_open,
    );
    (
        SelHandlers {
            on_layout_snap_intent,
            on_mounted_module_select,
            nudge_tree,
            on_open_module_placement,
            on_module_attached,
        },
        select_tree,
    )
}
