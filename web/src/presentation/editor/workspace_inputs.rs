use super::super::*;
use boardstudio_web_ui_model::Workspace;

pub(super) struct ObjectsInputArgs<'a> {
    pub active_workspace: Workspace,
    pub snapshot: &'a AcceptedSnapshot,
    pub workspace: Signal<Workspace>,
    pub adapter: SelectionAdapter,
    pub shared_objects: workspace_composition::SharedObjectsInput,
    pub matrix_setup: objects::MatrixSetupMount,
    pub part_placement: part_placement::PartPlacementMount,
    pub layout_target: Signal<Option<String>>,
    pub parts_query: parts::PartsQuery,
    pub on_browse_parts: EventHandler<()>,
    pub matrix_placement: objects::MatrixPlacementMount,
    pub model: &'a ReadModel,
    pub case_scene: Option<Rc<crate::runtime::CadScene>>,
    pub current_scope: Option<Scope>,
    pub instance_scope_pending: bool,
    pub case_selected_body_id: Option<String>,
    pub case_selected_layer_id: String,
    pub case_selection: shared_viewer::CaseSelection,
    pub case_tree_expanded: Signal<BTreeSet<String>>,
    pub case_action: EventHandler<case_workspace::TreeAction>,
    pub case_display: EventHandler<case_workspace::DisplayRequest>,
    pub pair_created_selection: Signal<Option<objects::MirroredPairCreated>>,
    pub layout_state: layout_workspace::LayoutWorkspaceState,
    pub workspace_routing: WorkspaceRoutingSlots,
    pub parts_selection: parts::PartsSelection,
    pub parts_select: EventHandler<()>,
    pub matrix_inspector: objects::MatrixInspectorMount,
}

pub(super) fn objects_input<'a>(
    input: ObjectsInputArgs<'a>,
) -> workspace_composition::WorkspaceObjectsInput<'a> {
    let ObjectsInputArgs {
        active_workspace,
        snapshot,
        workspace,
        adapter,
        shared_objects,
        matrix_setup,
        part_placement,
        layout_target,
        parts_query,
        on_browse_parts,
        matrix_placement,
        model,
        case_scene,
        current_scope,
        instance_scope_pending,
        case_selected_body_id,
        case_selected_layer_id,
        case_selection,
        case_tree_expanded,
        case_action,
        case_display,
        pair_created_selection,
        layout_state,
        workspace_routing,
        parts_selection,
        parts_select,
        matrix_inspector,
    } = input;
    match active_workspace {
        Workspace::Case => workspace_composition::WorkspaceObjectsInput::Case(Box::new(
            case_workspace::ObjectsInput {
                model,
                scope: current_scope.clone(),
                instance_scope_pending,
                scene: case_scene.clone(),
                selected_context: adapter.selected_context,
                selected_body_id: case_selected_body_id.clone(),
                selected_layer_id: case_selected_layer_id.clone(),
                case_selection,
                expanded: case_tree_expanded,
                on_action: case_action,
                on_select: workspace_routing.select_tree,
                on_display: case_display,
            },
        )),
        Workspace::Parts => {
            workspace_composition::WorkspaceObjectsInput::Parts(parts_workspace::ObjectsInput {
                snapshot: snapshot.clone(),
                scope: current_scope.clone(),
                scope_generation: adapter.generation,
                workspace,
                query: parts_query,
                selected: parts_selection,
                on_select: parts_select,
            })
        }
        _ => workspace_composition::WorkspaceObjectsInput::Layout(Box::new(
            layout_workspace::ObjectsInput {
                shared: shared_objects,
                matrix_setup: matrix_setup.clone(),
                matrix_inspector: matrix_inspector.clone(),
                pair_created: pair_created_selection,
                on_place_component: part_placement.on_place_component,
                layout_target,
                parts_query,
                on_browse_parts,
                placement_error: matrix_placement
                    .error
                    .clone()
                    .or_else(|| part_placement.error.clone()),
                add_object_menu_open: layout_state.add_object_menu_open(),
            },
        )),
    }
}

pub(super) struct WsWiring {
    pub mounted_module_select: EventHandler<String>,
    pub on_layout_selection_kind_handler: EventHandler<objects::LayoutSelectionKind>,
    pub layout_snap_intent: EventHandler<objects::LayoutSnapIntent>,
    pub case_display: EventHandler<case_workspace::DisplayRequest>,
    pub show_configured_board: EventHandler<String>,
    pub toggle_footprints: EventHandler<()>,
    pub case_scene: Option<Rc<crate::runtime::CadScene>>,
    pub case_selected_layer_id: String,
    pub case_selected_context: Option<objects::ScopedTreeContext>,
    pub on_browse_parts: EventHandler<()>,
    pub on_empty_board_matrix: Callback<()>,
    pub shared_objects: workspace_composition::SharedObjectsInput,
    pub on_open_module_placement: EventHandler<String>,
    pub on_module_attached: EventHandler<parts::AttachedModuleNavigation>,
}

pub(super) struct WiringInput<'a, 'b> {
    pub setup_a: &'a super::setup_bundles::SetupA,
    pub snapshot: &'a AcceptedSnapshot,
    pub render_scope: &'b Scope,
    pub matrix_setup: &'b objects::MatrixSetupMount,
    pub workspace_routing: &'b mut WorkspaceRoutingSlots,
    pub instance_scope_pending: bool,
    pub cam: &'b super::camera::CamNamed,
    pub sel: super::selection_handlers::SelHandlers,
    pub nav: &'b super::findings_navigation::NavNamed,
    pub ops: EventHandler<()>,
}

pub(super) fn use_workspace_wiring<'a, 'b>(
    input: WiringInput<'a, 'b>,
    select_tree: impl FnMut(objects::TreeSelectRequest) + 'static,
    navigate: super::findings_navigation::NavigateFn,
) -> (
    WsWiring,
    Option<workspace_composition::WorkspaceObjectsInput<'a>>,
) {
    let WiringInput {
        setup_a,
        snapshot,
        render_scope,
        matrix_setup,
        workspace_routing,
        instance_scope_pending,
        cam,
        sel,
        nav,
        ops,
    } = input;
    let super::selection_handlers::SelHandlers {
        on_layout_selection_kind,
        on_layout_snap_intent,
        on_mounted_module_select,
        on_case_action,
        on_case_display,
        nudge_tree,
        on_open_module_placement,
        on_module_attached,
    } = sel;
    let runtime = &setup_a.editor.runtime();
    let adapter = &setup_a.adapter;
    let workspace = setup_a.editor.workspace;
    let current_scope = &setup_a.view.current_scope;
    let model = &setup_a.view.model;
    let layout_assembly_3d = setup_a.layout_assembly_3d;
    let render_generation = setup_a.render_generation;
    let active_workspace = setup_a.active_workspace;
    let case_selection = setup_a.case_selection;
    let objects_panel_settings = setup_a.objects_panel_settings;
    let part_placement = &cam.placement.part_placement;
    let layout_target = setup_a.view.layout_target;
    let parts_query = setup_a.view.parts_query;
    let matrix_placement = &setup_a.matrix_pair.matrix_placement;
    let case_tree_expanded = setup_a.case_tree_expanded;
    let pair_created_selection = setup_a.matrix_pair.pair_created_selection;
    let layout_state = setup_a.layout_state;
    let parts_selection = setup_a.view.parts_selection;
    let matrix_inspector = &setup_a.matrix_inspector;
    let editor = setup_a.editor;
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
    let case_scene = case_workspace::workspace_display_scene(runtime.cad_scene(), render_scope);
    let case_selected_body_id = case_selection
        .body
        .read()
        .as_ref()
        .filter(|selection| &selection.scope == render_scope)
        .map(|selection| selection.body_id.clone());
    let case_selected_layer_id = case_selection.layer_id(render_scope);
    let case_selected_context = adapter
        .selected_context
        .read()
        .clone()
        .filter(|selected| &selected.scope == render_scope);
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
                || model.durability
                    != (Durability::Saved {
                        revision: owner_revision,
                    })
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
    let objects_input = (!workspace_composition::owns_surfaces(active_workspace)).then(|| {
        objects_input(ObjectsInputArgs {
            active_workspace,
            snapshot,
            workspace,
            adapter: adapter.clone(),
            shared_objects: shared_objects.clone(),
            matrix_setup: matrix_setup.clone(),
            part_placement: part_placement.clone(),
            layout_target,
            parts_query,
            on_browse_parts,
            matrix_placement: matrix_placement.clone(),
            model,
            case_scene: case_scene.clone(),
            current_scope: current_scope.clone(),
            instance_scope_pending,
            case_selected_body_id: case_selected_body_id.clone(),
            case_selected_layer_id: case_selected_layer_id.clone(),
            case_selection,
            case_tree_expanded,
            case_action: on_case_action,
            case_display: on_case_display,
            pair_created_selection,
            layout_state,
            workspace_routing: *workspace_routing,
            parts_selection,
            parts_select: ops,
            matrix_inspector: matrix_inspector.clone(),
        })
    });
    (
        WsWiring {
            mounted_module_select: on_mounted_module_select,
            on_layout_selection_kind_handler: on_layout_selection_kind,
            layout_snap_intent: on_layout_snap_intent,
            case_display: on_case_display,
            show_configured_board: nav.show_configured_board,
            toggle_footprints: super::super::workspace_composition::toggle_footprints(
                editor.layers,
            ),
            case_scene,
            case_selected_layer_id,
            case_selected_context,
            on_browse_parts,
            on_empty_board_matrix,
            shared_objects,
            on_open_module_placement,
            on_module_attached,
        },
        objects_input,
    )
}
