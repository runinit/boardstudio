use super::super::*;

pub(super) struct WorkspaceEventWiringInput {
    pub runtime: Rc<Runtime>,
    pub adapter: SelectionAdapter,
    pub workspace: Signal<&'static str>,
    pub layer_visibility: LayerVisibility,
    pub version: Signal<u64>,
}

pub(super) struct WorkspaceEventWiring {
    pub undo: Rc<Runtime>,
    pub redo: Rc<Runtime>,
    pub parts_select: EventHandler<()>,
    pub keymap_select: EventHandler<String>,
    pub keycaps_select: EventHandler<String>,
    pub keycaps_finding: EventHandler<keycaps_fit::FindingNavigationRequest>,
    pub pcb_empty_hit: EventHandler<PointerEvent>,
    pub pcb_part_hit: EventHandler<pcb_scene::PcbPartHit>,
    pub mounted_module_select: EventHandler<String>,
    pub pcb_part_pointer_down: EventHandler<pcb_scene::PcbPartPointerDown>,
    pub pcb_wiring_edit_board: EventHandler<()>,
    pub on_layout_selection_kind_handler: EventHandler<objects::LayoutSelectionKind>,
    pub layout_snap_intent: EventHandler<objects::LayoutSnapIntent>,
    pub pcb_selection_kind: EventHandler<objects::LayoutSelectionKind>,
    pub pcb_snap_intent: EventHandler<objects::LayoutSnapIntent>,
    pub case_action: EventHandler<case_workspace::TreeAction>,
    pub case_display: EventHandler<case_workspace::DisplayRequest>,
    pub keymap_layer: EventHandler<String>,
    pub keymap_export: EventHandler<()>,
    pub show_configured_board: EventHandler<String>,
    pub toggle_footprints: EventHandler<()>,
    pub canvas_end_pointer: EventHandler<PointerEvent>,
    pub canvas_cancel_pointer: EventHandler<PointerEvent>,
    pub canvas_keyboard: EventHandler<KeyboardEvent>,
    pub canvas_wheel: EventHandler<WheelEvent>,
}

pub(super) struct WiringHandlers {
    pub nudge_tree: EventHandler<objects::TreeNudgeRequest>,
    pub on_parts_select: EventHandler<()>,
    pub on_keymap_select: EventHandler<String>,
    pub on_keycaps_select: EventHandler<String>,
    pub on_keycaps_finding: EventHandler<keycaps_fit::FindingNavigationRequest>,
    pub on_pcb_empty_hit: EventHandler<PointerEvent>,
    pub on_pcb_part_hit: EventHandler<pcb_scene::PcbPartHit>,
    pub on_mounted_module_select: EventHandler<String>,
    pub on_pcb_part_pointer_down: EventHandler<pcb_scene::PcbPartPointerDown>,
    pub on_pcb_wiring_edit_board: EventHandler<()>,
    pub on_layout_selection_kind: EventHandler<objects::LayoutSelectionKind>,
    pub on_layout_snap_intent: EventHandler<objects::LayoutSnapIntent>,
    pub on_pcb_selection_kind: EventHandler<objects::LayoutSelectionKind>,
    pub on_pcb_snap_intent: EventHandler<objects::LayoutSnapIntent>,
    pub on_case_action: EventHandler<case_workspace::TreeAction>,
    pub on_case_display: EventHandler<case_workspace::DisplayRequest>,
    pub on_keymap_layer: EventHandler<String>,
    pub on_keymap_export: EventHandler<()>,
    pub on_show_configured_board: EventHandler<String>,
    pub end_pointer: EventHandler<PointerEvent>,
    pub cancel_pointer: EventHandler<PointerEvent>,
    pub keyboard: EventHandler<KeyboardEvent>,
    pub wheel: EventHandler<WheelEvent>,
}

pub(super) fn use_workspace_event_wiring(
    input: WorkspaceEventWiringInput,
    workspace_routing: &mut WorkspaceRoutingSlots,
    select_tree: impl FnMut(objects::TreeSelectRequest) + 'static,
    navigate: super::findings_navigation::NavigateFn,
    handlers: WiringHandlers,
) -> (
    WorkspaceEventWiring,
    workspace_composition::SharedObjectsInput,
) {
    let WorkspaceEventWiringInput {
        runtime,
        adapter,
        workspace,
        layer_visibility,
        version,
    } = input;
    let undo = runtime.clone();
    let redo = runtime.clone();
    workspace_routing.select_tree.replace(Box::new(select_tree));
    workspace_routing.navigate.replace(navigate);
    let WiringHandlers {
        nudge_tree,
        on_parts_select: parts_select,
        on_keymap_select: keymap_select,
        on_keycaps_select: keycaps_select,
        on_keycaps_finding: keycaps_finding,
        on_pcb_empty_hit: pcb_empty_hit,
        on_pcb_part_hit: pcb_part_hit,
        on_mounted_module_select: mounted_module_select,
        on_pcb_part_pointer_down: pcb_part_pointer_down,
        on_pcb_wiring_edit_board: pcb_wiring_edit_board,
        on_layout_selection_kind: on_layout_selection_kind_handler,
        on_layout_snap_intent: layout_snap_intent,
        on_pcb_selection_kind: pcb_selection_kind,
        on_pcb_snap_intent: pcb_snap_intent,
        on_case_action: case_action,
        on_case_display: case_display,
        on_keymap_layer: keymap_layer,
        on_keymap_export: keymap_export,
        on_show_configured_board: show_configured_board,
        end_pointer,
        cancel_pointer,
        keyboard,
        wheel,
    } = handlers;
    let toggle_footprints = EventHandler::new(move |_| {
        let mut footprints = layer_visibility.footprints;
        footprints.set(!footprints());
    });
    let canvas_end_pointer = end_pointer;
    let canvas_cancel_pointer = cancel_pointer;
    let canvas_keyboard = keyboard;
    let canvas_wheel = wheel;
    let board_setup = objects::use_board_setup(
        runtime.clone(),
        version,
        workspace,
        adapter.generation,
        workspace_routing.navigate,
    );
    let shared_objects = workspace_composition::SharedObjectsInput {
        selected_context: adapter.selected_context,
        on_select: workspace_routing.select_tree,
        on_navigate: workspace_routing.navigate,
        on_nudge: nudge_tree,
        board_setup,
    };
    (
        WorkspaceEventWiring {
            undo,
            redo,
            parts_select,
            keymap_select,
            keycaps_select,
            keycaps_finding,
            pcb_empty_hit,
            pcb_part_hit,
            mounted_module_select,
            pcb_part_pointer_down,
            pcb_wiring_edit_board,
            on_layout_selection_kind_handler,
            layout_snap_intent,
            pcb_selection_kind,
            pcb_snap_intent,
            case_action,
            case_display,
            keymap_layer,
            keymap_export,
            show_configured_board,
            toggle_footprints,
            canvas_end_pointer,
            canvas_cancel_pointer,
            canvas_keyboard,
            canvas_wheel,
        },
        shared_objects,
    )
}

pub(super) struct PcbWiringBindingsInput<'a> {
    pub runtime: Rc<Runtime>,
    pub current_scope: Option<Scope>,
    pub model: &'a ReadModel,
    pub instance_selection: InstanceSelection,
    pub snapshot: &'a AcceptedSnapshot,
    pub adapter: SelectionAdapter,
    pub version: Signal<u64>,
    pub workspace: Signal<&'static str>,
    pub matrix_setup: objects::MatrixSetupMount,
    pub layout_assembly_3d: Signal<bool>,
    pub render_scope: Scope,
    pub render_generation: u64,
    pub active_workspace: &'static str,
    pub case_selection: case_viewer::CaseSelection,
    pub objects_open: Signal<bool>,
    pub objects_panel_settings: Signal<PanelSettings>,
    pub resolution_signal: Signal<pcb_wiring::PcbWiringResolution>,
}

pub(super) struct PcbWiringBindings {
    pub case_scene: Option<Rc<crate::runtime::CadScene>>,
    pub pcb_wiring_source: Option<pcb_wiring::PcbWiringSource>,
    pub pcb_wiring_mode_actions: pcb_wiring::BoardWiringModeActions,
    pub pcb_wiring_pin_actions: pcb_wiring::PcbWiringPinActions,
    pub pcb_wiring_protected_remap_actions: pcb_wiring::ProtectedRemapActions,
    pub pcb_wiring_apply_actions: pcb_wiring::BoardWiringApplyActions,
    pub zmk_firmware_export_panel: zmk_firmware_export::ZmkFirmwareExportPanelInput,
    pub case_selected_body_id: Option<String>,
    pub case_selected_layer_id: String,
    pub case_selected_context: Option<objects::ScopedTreeContext>,
    pub on_browse_parts: EventHandler<()>,
    pub on_empty_board_matrix: Callback<(), ()>,
}

pub(super) fn use_pcb_wiring_bindings(input: PcbWiringBindingsInput<'_>) -> PcbWiringBindings {
    let PcbWiringBindingsInput {
        runtime,
        current_scope,
        model,
        instance_selection,
        snapshot,
        adapter,
        version,
        workspace,
        matrix_setup,
        layout_assembly_3d,
        render_scope,
        render_generation,
        active_workspace,
        case_selection,
        objects_open,
        objects_panel_settings,
        resolution_signal,
    } = input;
    let case_scene = case_workspace::workspace_display_scene(runtime.cad_scene(), &render_scope);
    let pcb_wiring_source = current_scope
        .as_ref()
        .filter(|scope| {
            active_board_scope_matches(model, scope) && instance_selection.is_current(model)
        })
        .and_then(|scope| {
            let mut board_scope = scope.clone();
            board_scope.instance_id = None;
            pcb_wiring::PcbWiringSource::new(
                snapshot,
                &board_scope,
                scope,
                model.selected_part_ids.first().map(String::as_str),
                runtime.electrical_preview_executor_epoch(),
                (adapter.generation)(),
            )
        });
    let pcb_wiring_mode_actions = pcb_wiring::use_board_wiring_mode_edits(
        runtime.clone(),
        version,
        workspace,
        adapter.generation,
        {
            let runtime = runtime.clone();
            Rc::new(move || instance_selection.is_current(&runtime.model()))
        },
        pcb_wiring_source.clone(),
        resolution_signal,
    );
    let pcb_wiring_pin_actions = pcb_wiring::use_pcb_wiring_pin_edits(
        runtime.clone(),
        version,
        workspace,
        adapter.generation,
        {
            let runtime = runtime.clone();
            Rc::new(move || instance_selection.is_current(&runtime.model()))
        },
        pcb_wiring_source.clone(),
        resolution_signal,
    );
    let pcb_wiring_protected_remap_actions = pcb_wiring::use_protected_remap_review(
        runtime.clone(),
        version,
        workspace,
        adapter.generation,
        {
            let runtime = runtime.clone();
            Rc::new(move || instance_selection.is_current(&runtime.model()))
        },
        pcb_wiring_source.clone(),
    );
    let pcb_wiring_apply_actions = pcb_wiring::use_board_wiring_apply(
        runtime.clone(),
        version,
        workspace,
        adapter.generation,
        {
            let runtime = runtime.clone();
            Rc::new(move || instance_selection.is_current(&runtime.model()))
        },
        pcb_wiring_source.clone(),
        resolution_signal,
    );
    let zmk_firmware_export_panel = use_export_panel_input(
        runtime.clone(),
        workspace,
        adapter.generation,
        pcb_wiring_source.clone(),
        resolution_signal,
        instance_selection,
    );
    let case_selected_body_id = case_selection
        .body
        .read()
        .as_ref()
        .filter(|selection| selection.scope == render_scope)
        .map(|selection| selection.body_id.clone());
    let case_selected_layer_id = case_selection.layer_id(&render_scope);
    let case_selected_context = adapter
        .selected_context
        .read()
        .clone()
        .filter(|selected| selected.scope == render_scope);
    let on_browse_parts = EventHandler::new(move |_| {
        browse_parts_workspace(
            workspace,
            objects_open,
            objects_panel_settings,
            is_compact_viewport(),
        );
    });
    let on_empty_board_matrix = use_callback({
        let runtime = runtime.clone();
        let owner_scope = render_scope.clone();
        let owner_token = snapshot.token;
        let owner_revision = snapshot.document.revision;
        let generation = adapter.generation;
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
            workspace.set("Layout");
            let mut layout_assembly_3d = layout_assembly_3d;
            layout_assembly_3d.set(false);
            on_open.call(());
        }
    });
    PcbWiringBindings {
        case_scene,
        pcb_wiring_source,
        pcb_wiring_mode_actions,
        pcb_wiring_pin_actions,
        pcb_wiring_protected_remap_actions,
        pcb_wiring_apply_actions,
        zmk_firmware_export_panel,
        case_selected_body_id,
        case_selected_layer_id,
        case_selected_context,
        on_browse_parts,
        on_empty_board_matrix,
    }
}

pub(super) struct ObjectsInputArgs<'a, 'b> {
    pub active_workspace: &'static str,
    pub runtime: Rc<Runtime>,
    pub render_generation: u64,
    pub canvas_interaction: CanvasInteractionArbiter,
    pub render_scope: Scope,
    pub snapshot: &'a AcceptedSnapshot,
    pub workspace: Signal<&'static str>,
    pub adapter: SelectionAdapter,
    pub layout_assembly_3d: Signal<bool>,
    pub shared_objects: workspace_composition::SharedObjectsInput,
    pub matrix_setup: objects::MatrixSetupMount,
    pub part_placement: part_placement::PartPlacementMount,
    pub layout_target: Signal<Option<String>>,
    pub parts_query: parts::PartsQuery,
    pub on_browse_parts: EventHandler<()>,
    pub matrix_placement: objects::MatrixPlacementMount,
    pub keymap_view: Option<Rc<keymap::KeymapView>>,
    pub keymap_layer_id: Signal<String>,
    pub model: &'a ReadModel,
    pub layer_actions: &'b keymap::LayerActions,
    pub keymap_layer: EventHandler<String>,
    pub keymap_select: EventHandler<String>,
    pub keycaps_view: Option<Rc<keycaps_scene::KeycapsView>>,
    pub keycaps_settings_actions: Option<keycaps_settings::KeycapsSettingsActions>,
    pub case_scene: Option<Rc<crate::runtime::CadScene>>,
    pub current_scope: Option<Scope>,
    pub instance_scope_pending: bool,
    pub case_selected_body_id: Option<String>,
    pub case_selected_layer_id: String,
    pub case_selection: case_viewer::CaseSelection,
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
    input: ObjectsInputArgs<'a, '_>,
) -> workspace_composition::WorkspaceObjectsInput<'a> {
    let ObjectsInputArgs {
        active_workspace,
        runtime,
        render_generation,
        canvas_interaction,
        render_scope,
        snapshot,
        workspace,
        adapter,
        layout_assembly_3d,
        shared_objects,
        matrix_setup,
        part_placement,
        layout_target,
        parts_query,
        on_browse_parts,
        matrix_placement,
        keymap_view,
        keymap_layer_id,
        model,
        layer_actions,
        keymap_layer,
        keymap_select,
        keycaps_view,
        keycaps_settings_actions,
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
        "PCB" => {
            let pcb_add_owner = LayoutOwnerIdentity {
                scope: Some(render_scope.clone()),
                token: Some(snapshot.token),
                revision: Some(snapshot.document.revision),
                generation: render_generation,
                workspace: "PCB",
            };
            let pcb_add_is_current =
                pcb_add_layout_owner_is_current(&runtime, workspace, &adapter, &pcb_add_owner);
            let mut pcb_shared_objects = shared_objects;
            pcb_shared_objects.on_select = pcb_add_outline_select_handler(
                runtime.clone(),
                workspace,
                adapter.clone(),
                pcb_add_owner.clone(),
                layout_assembly_3d,
                pcb_shared_objects.on_select,
            );
            let mut pcb_matrix_setup = matrix_setup.clone();
            pcb_matrix_setup.can_open = pcb_add_is_current
                && pcb_matrix_setup.projection.is_none()
                && canvas_interaction.current().is_none();
            pcb_matrix_setup.on_open = pcb_add_layout_open_handler(
                runtime.clone(),
                workspace,
                adapter.clone(),
                pcb_add_owner.clone(),
                layout_assembly_3d,
                matrix_setup.on_open,
            );
            workspace_composition::WorkspaceObjectsInput::Pcb(Box::new(
                pcb_workspace::ObjectsInput {
                    shared: pcb_shared_objects,
                    matrix_setup: pcb_matrix_setup,
                    on_place_component: part_placement.on_place_component,
                    layout_target,
                    parts_query,
                    on_browse_parts,
                    placement_error: matrix_placement
                        .error
                        .clone()
                        .or_else(|| part_placement.error.clone()),
                },
            ))
        }
        "Keymap" => workspace_composition::WorkspaceObjectsInput::Keymap(
            keymap_workspace::NavigationInput {
                view: keymap_view.clone(),
                scope: render_scope.clone(),
                active_layer_id: keymap_layer_id(),
                selected_key_id: model.selected_part_ids.first().cloned(),
                layer_actions: keymap::LayerActions {
                    enabled: layer_actions.enabled,
                    feedback: layer_actions.feedback.clone(),
                    on_operation: layer_actions.on_operation,
                },
                on_layer: keymap_layer,
                on_select_key: keymap_select,
            },
        ),
        "Keycaps" => workspace_composition::WorkspaceObjectsInput::Keycaps(
            keycaps_workspace::NavigationInput {
                shared: shared_objects,
                view: keycaps_view.clone(),
                settings_actions: keycaps_settings_actions.clone(),
            },
        ),
        "Case" => workspace_composition::WorkspaceObjectsInput::Case(Box::new(
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
        "Parts" => {
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
    pub wiring: WorkspaceEventWiring,
    pub pcb: PcbWiringBindings,
    pub on_open_module_placement: EventHandler<String>,
    pub on_module_attached: EventHandler<parts::AttachedModuleNavigation>,
}

pub(super) struct WiringInput<'a, 'b> {
    pub setup_a: &'a super::setup_bundles::SetupA,
    pub setup_b: &'b super::setup_bundles::SetupB,
    pub snapshot: &'a AcceptedSnapshot,
    pub render_scope: &'b Scope,
    pub matrix_setup: &'b objects::MatrixSetupMount,
    pub workspace_routing: &'b mut WorkspaceRoutingSlots,
    pub instance_scope_pending: bool,
    pub layer_actions: &'b keymap::LayerActions,
    pub cam: &'b super::camera::CamNamed,
    pub sel: super::selection_handlers::SelHandlers,
    pub nav: &'b super::findings_navigation::NavNamed,
    pub pk: super::keyboard::PkNamed,
    pub ops: EventHandler<()>,
}

pub(super) fn use_workspace_wiring<'a, 'b>(
    input: WiringInput<'a, 'b>,
    select_tree: impl FnMut(objects::TreeSelectRequest) + 'static,
    navigate: super::findings_navigation::NavigateFn,
    end_pointer: impl FnMut(PointerEvent) + Clone + 'static,
    cancel_pointer: impl FnMut(PointerEvent) + Clone + 'static,
    keyboard: impl FnMut(KeyboardEvent) + Clone + 'static,
    wheel: impl FnMut(WheelEvent) + Clone + 'static,
) -> (WsWiring, workspace_composition::WorkspaceObjectsInput<'a>) {
    let WiringInput {
        setup_a,
        setup_b,
        snapshot,
        render_scope,
        matrix_setup,
        workspace_routing,
        instance_scope_pending,
        layer_actions,
        cam,
        sel,
        nav,
        pk,
        ops,
    } = input;
    let super::selection_handlers::SelHandlers {
        on_layout_selection_kind,
        on_layout_snap_intent,
        on_pcb_selection_kind,
        on_pcb_snap_intent,
        on_keymap_select,
        on_keycaps_select,
        on_pcb_part_hit,
        on_mounted_module_select,
        on_pcb_wiring_edit_board,
        on_case_action,
        on_case_display,
        on_keymap_layer,
        on_keymap_export,
        nudge_tree,
        on_open_module_placement,
        on_module_attached,
    } = sel;
    let super::keyboard::PkNamed {
        on_pcb_part_pointer_down,
        on_pcb_empty_hit,
    } = pk;
    let handlers = WiringHandlers {
        nudge_tree,
        on_parts_select: ops,
        on_keymap_select,
        on_keycaps_select,
        on_keycaps_finding: nav.keycaps_finding,
        on_pcb_empty_hit,
        on_pcb_part_hit,
        on_mounted_module_select,
        on_pcb_part_pointer_down,
        on_pcb_wiring_edit_board,
        on_layout_selection_kind,
        on_layout_snap_intent,
        on_pcb_selection_kind,
        on_pcb_snap_intent,
        on_case_action,
        on_case_display,
        on_keymap_layer,
        on_keymap_export,
        on_show_configured_board: nav.show_configured_board,
        end_pointer: EventHandler::new(end_pointer.clone()),
        cancel_pointer: EventHandler::new(cancel_pointer.clone()),
        keyboard: EventHandler::new(keyboard.clone()),
        wheel: EventHandler::new(wheel.clone()),
    };
    let runtime = &setup_a.runtime;
    let adapter = &setup_a.adapter;
    let workspace = setup_a.workspace;
    let objects_open = setup_a.objects_open;
    let layer_visibility = setup_a.layer_visibility;
    let version = setup_a.version;
    let current_scope = &setup_a.current_scope;
    let model = &setup_a.model;
    let instance_selection = setup_a.instance_selection;
    let layout_assembly_3d = setup_a.layout_assembly_3d;
    let render_generation = setup_a.render_generation;
    let active_workspace = setup_a.active_workspace;
    let case_selection = setup_a.case_selection;
    let objects_panel_settings = setup_a.objects_panel_settings;
    let resolution_signal = setup_b.resolution_signal;
    let canvas_interaction = &setup_a.canvas_interaction;
    let part_placement = &cam.placement.part_placement;
    let layout_target = setup_a.layout_target;
    let parts_query = setup_a.parts_query;
    let matrix_placement = &setup_a.matrix_placement;
    let keymap_view = &setup_b.keymap_view;
    let keymap_layer_id = setup_a.keymap_layer_id;
    let keycaps_view = &setup_b.keycaps_view;
    let keycaps_settings_actions = &setup_b.keycaps_settings_actions;
    let case_tree_expanded = setup_a.case_tree_expanded;
    let pair_created_selection = setup_a.pair_created_selection;
    let layout_state = setup_a.layout_state;
    let parts_selection = setup_a.parts_selection;
    let matrix_inspector = &setup_a.matrix_inspector;
    let (
        WorkspaceEventWiring {
            undo,
            redo,
            parts_select,
            keymap_select,
            keycaps_select,
            keycaps_finding,
            pcb_empty_hit,
            pcb_part_hit,
            mounted_module_select,
            pcb_part_pointer_down,
            pcb_wiring_edit_board,
            on_layout_selection_kind_handler,
            layout_snap_intent,
            pcb_selection_kind,
            pcb_snap_intent,
            case_action,
            case_display,
            keymap_layer,
            keymap_export,
            show_configured_board,
            toggle_footprints,
            canvas_end_pointer,
            canvas_cancel_pointer,
            canvas_keyboard,
            canvas_wheel,
        },
        shared_objects,
    ) = use_workspace_event_wiring(
        WorkspaceEventWiringInput {
            runtime: runtime.clone(),
            adapter: adapter.clone(),
            workspace,
            layer_visibility,
            version,
        },
        &mut *workspace_routing,
        select_tree,
        navigate,
        handlers,
    );
    let PcbWiringBindings {
        case_scene,
        pcb_wiring_source,
        pcb_wiring_mode_actions,
        pcb_wiring_pin_actions,
        pcb_wiring_protected_remap_actions,
        pcb_wiring_apply_actions,
        zmk_firmware_export_panel,
        case_selected_body_id,
        case_selected_layer_id,
        case_selected_context,
        on_browse_parts,
        on_empty_board_matrix,
    } = use_pcb_wiring_bindings(PcbWiringBindingsInput {
        runtime: runtime.clone(),
        current_scope: current_scope.clone(),
        model,
        instance_selection,
        snapshot,
        adapter: adapter.clone(),
        version,
        workspace,
        matrix_setup: matrix_setup.clone(),
        layout_assembly_3d,
        render_scope: render_scope.clone(),
        render_generation,
        active_workspace,
        case_selection,
        objects_open,
        objects_panel_settings,
        resolution_signal,
    });
    let objects_input = objects_input(ObjectsInputArgs {
        active_workspace,
        runtime: runtime.clone(),
        render_generation,
        canvas_interaction: canvas_interaction.clone(),
        render_scope: render_scope.clone(),
        snapshot,
        workspace,
        adapter: adapter.clone(),
        layout_assembly_3d,
        shared_objects,
        matrix_setup: matrix_setup.clone(),
        part_placement: part_placement.clone(),
        layout_target,
        parts_query,
        on_browse_parts,
        matrix_placement: matrix_placement.clone(),
        keymap_view: keymap_view.clone(),
        keymap_layer_id,
        model,
        layer_actions,
        keymap_layer,
        keymap_select,
        keycaps_view: keycaps_view.clone(),
        keycaps_settings_actions: keycaps_settings_actions.clone(),
        case_scene: case_scene.clone(),
        current_scope: current_scope.clone(),
        instance_scope_pending,
        case_selected_body_id: case_selected_body_id.clone(),
        case_selected_layer_id: case_selected_layer_id.clone(),
        case_selection,
        case_tree_expanded,
        case_action,
        case_display,
        pair_created_selection,
        layout_state,
        workspace_routing: *workspace_routing,
        parts_selection,
        parts_select,
        matrix_inspector: matrix_inspector.clone(),
    });
    (
        WsWiring {
            wiring: WorkspaceEventWiring {
                undo,
                redo,
                parts_select,
                keymap_select,
                keycaps_select,
                keycaps_finding,
                pcb_empty_hit,
                pcb_part_hit,
                mounted_module_select,
                pcb_part_pointer_down,
                pcb_wiring_edit_board,
                on_layout_selection_kind_handler,
                layout_snap_intent,
                pcb_selection_kind,
                pcb_snap_intent,
                case_action,
                case_display,
                keymap_layer,
                keymap_export,
                show_configured_board,
                toggle_footprints,
                canvas_end_pointer,
                canvas_cancel_pointer,
                canvas_keyboard,
                canvas_wheel,
            },
            pcb: PcbWiringBindings {
                case_scene,
                pcb_wiring_source,
                pcb_wiring_mode_actions,
                pcb_wiring_pin_actions,
                pcb_wiring_protected_remap_actions,
                pcb_wiring_apply_actions,
                zmk_firmware_export_panel,
                case_selected_body_id,
                case_selected_layer_id,
                case_selected_context,
                on_browse_parts,
                on_empty_board_matrix,
            },
            on_open_module_placement,
            on_module_attached,
        },
        objects_input,
    )
}
