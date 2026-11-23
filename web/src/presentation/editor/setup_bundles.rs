use super::super::*;
use super::EditorContext;
use boardstudio_web_ui_model::Workspace;

pub(super) struct SetupA {
    pub editor: EditorContext,
    pub objects_panel_settings: Signal<PanelSettings>,
    pub inspector_panel_settings: Signal<PanelSettings>,
    pub setup_state: setup_workspace::SetupWorkspaceState,
    pub guide_preferences: Signal<Option<setup_guide::SetupGuidePreferences>>,
    pub guide_name_draft: Signal<Option<(String, String)>>,
    pub guide_name_failure: Signal<Option<String>>,
    pub guide_name_edit: SetupProjectNameEdit,
    pub adapter: SelectionAdapter,
    pub layout_component_inspector_lifetime: Rc<inspector::LayoutComponentInspectorLifetime>,
    pub observed_version: u64,
    pub case_workspace_state: case_workspace::CaseWorkspaceState,
    pub case_selection: shared_viewer::CaseSelection,
    pub case_tree_expanded: Signal<std::collections::BTreeSet<String>>,
    pub layout_state: layout_workspace::LayoutWorkspaceState,
    pub canvas_navigation: CanvasNavigationState,
    pub active_workspace: Workspace,
    pub requested_workspace_panel: Signal<Option<Workspace>>,
    pub render_generation: u64,
    pub layout_selection_kind: Signal<objects::LayoutSelectionKind>,
    pub split_focus: Signal<boardstudio_web_ui_model::split_focus::SplitHalf>,
    pub spacing_ghost_state: Signal<Option<spacing_ghost::SpacingGhostState>>,
    pub layout_snap_settings: Signal<objects::LayoutSnapSettings>,
    pub layout_command_menu: Signal<Option<objects::LayoutCommandMenu>>,
    pub layout_transform_tool: Signal<Option<objects::LayoutTransformTool>>,
    pub layout_assembly_3d: Signal<bool>,
    pub tree_cell_anchor: Rc<RefCell<Option<OwnedTreeCellAnchor>>>,
    pub matrix_inspector: objects::MatrixInspectorMount,
    pub board_inspector: board_inspector::BoardInspectorMount,
    pub key_size: objects::KeySizeMount,
    pub matrix_splay_affect: Signal<boardstudio_core::model::MatrixSplayAffect>,
    pub matrix_transform_inspector: objects::MatrixTransformInspectorMount,
    pub pending_splay_origin_pick: Signal<Option<objects::MatrixTransformInspectorOwner>>,
    pub pending_anchor_pick: Signal<Option<objects::MatrixTransformInspectorOwner>>,
    pub layout_align: objects::LayoutAlignMount,
    pub pcb_align: objects::LayoutAlignMount,
    pub parts_state: parts_workspace::PartsWorkspaceState,
    pub canvas_interaction: CanvasInteractionArbiter,
    pub matrix_pair: super::setup_shell::MatrixPairWiring,
    pub view: super::setup_shell::EditorContexts,
}

pub(super) struct SetupB {
    pub keycaps_state: keycaps_workspace::KeycapsWorkspaceState,
    pub focused_keycaps_finding: Signal<Option<keycaps_finding_marker::FocusedFinding>>,
    pub keycaps_navigation_alive: Rc<Cell<bool>>,
    pub keycaps_fit_state: keycaps_fit::KeycapsFitActions,
    pub keycaps_settings_actions: Option<keycaps_settings::KeycapsSettingsActions>,
    pub resolution_signal: Signal<pcb_wiring::PcbWiringResolution>,
    pub resolution: pcb_wiring::PcbWiringResolution,
    pub on_resolve: EventHandler<()>,
    pub firmware_feedback: Option<pcb_wiring::FirmwarePositionFeedback>,
    pub firmware_editable: bool,
    pub firmware_on_edit: EventHandler<pcb_wiring::FirmwarePositionEditRequest>,
    pub part_net_actions: pcb_wiring::PartNetActions,
    pub part_input_actions: pcb_wiring::PartInputActions,
    pub physical_setup_mount: pcb_physical_setup::PhysicalSetupMount,
    pub keymap_view: Option<Rc<keymap::KeymapView>>,
    pub keymap_contours: Option<Rc<[Contour]>>,
    pub keycaps_view: Option<Rc<keycaps_scene::KeycapsView>>,
    pub keycaps_contours: Option<Rc<[Contour]>>,
    pub mechanical_settings: mechanical_settings::MechanicalSettingsState,
}

pub(super) fn use_editor_setup_shell(
    editor: EditorContext,
) -> (
    SetupA,
    WorkspaceRoutingSlots,
    objects::MatrixSetupMount,
    EventHandler<()>,
) {
    boardstudio_web_ui_shared::layer_dock::use_layer_dock_provider();
    parts::use_assembly_action_provider();
    let workspace_routing = use_hook(|| WorkspaceRoutingSlots {
        select_tree: EventHandler::new(|_: objects::TreeSelectRequest| {}),
        navigate: EventHandler::new(|_: (Scope, String, Option<String>)| {}),
    });
    let runtime = editor.runtime();
    let CompactPanelState {
        objects_open,
        inspector_open: inspect_open,
    } = editor.panels;
    let preference_warning = use_context::<PreferenceStorageWarning>().0;
    let objects_panel_settings = use_panel_settings(PanelSide::Objects, preference_warning);
    let inspector_panel_settings = use_panel_settings(PanelSide::Inspector, preference_warning);
    let created_request_signal = use_context::<Signal<Option<SetupGuideRequest>>>();
    let setup_state = setup_workspace::use_setup_workspace_state(
        runtime.clone(),
        created_request_signal,
        objects_open,
        inspect_open,
        objects_panel_settings,
        inspector_panel_settings,
    );
    let guide_preferences = setup_state.guide_preferences();
    let guide_name_edit = use_setup_project_name(runtime.clone());
    let guide_name_draft = guide_name_edit.draft;
    let guide_name_failure = guide_name_edit.failure;
    let adapter = use_context::<SelectionAdapter>();
    let layout_component_inspector_lifetime =
        use_hook(|| Rc::new(inspector::LayoutComponentInspectorLifetime::default()));
    let observed_version = (editor.version)();
    let case_workspace_state = case_workspace::use_case_workspace_state();
    let case_selection = case_workspace_state.selection();
    let case_tree_expanded = case_workspace_state.tree_expanded();
    let workspace = editor.workspace;
    let layout_state =
        layout_workspace::use_layout_workspace_state(adapter.selected_context, workspace);
    let canvas_navigation = use_canvas_navigation_state(workspace);
    let active_workspace = workspace();
    let requested_workspace_panel =
        panels::use_workspace_panel_defaults(active_workspace, objects_open, inspect_open);
    let render_generation = (editor.scope_generation)();
    let layout_selection_kind = layout_state.selection_kind();
    let split_focus = layout_state.split_focus();
    let super::setup_shell::LayoutViewState {
        spacing_ghost_state,
        layout_snap_settings,
        layout_command_menu,
        layout_transform_tool,
        layout_assembly_3d,
        tree_cell_anchor,
        matrix_inspector,
        board_inspector,
        key_size,
        matrix_splay_affect,
        matrix_transform_inspector,
        pending_splay_origin_pick,
        pending_anchor_pick,
        layout_align,
        pcb_align,
        mut matrix_setup,
        parts_state,
        parts_assembly_orientation,
        canvas_interaction,
    } = super::setup_shell::use_layout_view_state(editor, &adapter, layout_state);
    let matrix_pair =
        super::setup_shell::use_matrix_pair_wiring(super::setup_shell::MatrixPairWiringInput {
            editor,
            adapter: adapter.clone(),
            layout_assembly_3d,
            canvas_interaction: canvas_interaction.clone(),
            layout_state,
            inspector_panel_settings,
            parts_assembly_orientation,
            matrix_setup: &mut matrix_setup,
        });
    let (view, on_parts_select) = super::setup_shell::use_editor_contexts(
        editor,
        &adapter,
        parts_state,
        &tree_cell_anchor,
        render_generation,
        active_workspace,
    );
    (
        SetupA {
            editor,
            objects_panel_settings,
            inspector_panel_settings,
            setup_state,
            guide_preferences,
            guide_name_draft,
            guide_name_failure,
            guide_name_edit,
            adapter,
            layout_component_inspector_lifetime,
            observed_version,
            case_workspace_state,
            case_selection,
            case_tree_expanded,
            layout_state,
            canvas_navigation,
            active_workspace,
            requested_workspace_panel,
            render_generation,
            layout_selection_kind,
            split_focus,
            spacing_ghost_state,
            layout_snap_settings,
            layout_command_menu,
            layout_transform_tool,
            layout_assembly_3d,
            tree_cell_anchor,
            matrix_inspector,
            board_inspector,
            key_size,
            matrix_splay_affect,
            matrix_transform_inspector,
            pending_splay_origin_pick,
            pending_anchor_pick,
            layout_align,
            pcb_align,
            parts_state,
            canvas_interaction,
            matrix_pair,
            view,
        },
        workspace_routing,
        matrix_setup,
        on_parts_select,
    )
}

pub(super) fn use_setup_workspaces(
    setup_a: &SetupA,
) -> (
    SetupB,
    keymap::LayerActions,
    keymap::BindingActions,
    keymap::MacroActions,
) {
    let model = &setup_a.view.model;
    let current_scope = &setup_a.view.current_scope;
    let runtime = &setup_a.editor.runtime();
    let workspace = setup_a.editor.workspace;
    let version = setup_a.editor.version;
    let adapter = &setup_a.adapter;
    let instance_selection = setup_a.editor.instance_selection;
    let layout_owner = &setup_a.view.layout_owner;
    let guide_preferences = setup_a.guide_preferences;
    let keymap_layer_id = setup_a.view.keymap_layer_id;
    let accepted_token = setup_a.view.accepted_token;
    let inspect_open = setup_a.editor.panels.inspector_open;
    let case_selection = setup_a.case_selection;
    let super::setup_workspaces::KeycapsPcbState {
        active_board_id,
        keycaps_state,
        focused_keycaps_finding,
        keycaps_navigation_alive,
        layer_source,
        keycaps_fit_state,
        keycaps_settings_actions,
        resolution_signal,
        resolution,
        on_resolve,
        firmware_feedback,
        firmware_editable,
        firmware_on_edit,
        part_net_actions,
        part_input_actions,
    } = super::setup_workspaces::use_keycaps_pcb_state(super::setup_workspaces::KeycapsPcbInput {
        editor: setup_a.editor,
        model,
        current_scope: current_scope.clone(),
        layout_owner: layout_owner.clone(),
    });
    let physical_setup_mount = super::setup_workspaces::use_physical_setup(
        runtime.clone(),
        guide_preferences,
        version,
        adapter.clone(),
        instance_selection,
        workspace,
    );
    let super::setup_workspaces::KeymapProjections {
        layer_actions,
        binding_actions,
        keymap_view,
        keymap_contours,
        keycaps_view,
        keycaps_contours,
        macro_actions,
    } = super::setup_workspaces::use_keymap_projections(
        super::setup_workspaces::KeymapProjectionsInput {
            runtime: runtime.clone(),
            layer_source,
            keymap_layer_id,
            workspace,
            generation: adapter.generation,
            accepted_token,
            current_scope: current_scope.clone(),
            active_board_id: active_board_id.clone(),
            instance_selection,
            resolution_signal,
        },
    );
    let on_show_mechanical_finding = super::setup_workspaces::on_show_mechanical_finding_handler(
        super::setup_workspaces::OnShowMechanicalFindingArgs {
            runtime: runtime.clone(),
            current_scope: current_scope.clone(),
            model,
            adapter_generation: adapter.generation,
            workspace,
            inspect_open,
            case_selection,
            instance_selection,
        },
    );
    let mechanical_settings = mechanical_settings::use_mechanical_settings(
        runtime.clone(),
        workspace,
        instance_selection,
        case_selection,
        on_show_mechanical_finding,
    );
    (
        SetupB {
            keycaps_state,
            focused_keycaps_finding,
            keycaps_navigation_alive,
            keycaps_fit_state,
            keycaps_settings_actions,
            resolution_signal,
            resolution,
            on_resolve,
            firmware_feedback,
            firmware_editable,
            firmware_on_edit,
            part_net_actions,
            part_input_actions,
            physical_setup_mount,
            keymap_view,
            keymap_contours,
            keycaps_view,
            keycaps_contours,
            mechanical_settings,
        },
        layer_actions,
        binding_actions,
        macro_actions,
    )
}
