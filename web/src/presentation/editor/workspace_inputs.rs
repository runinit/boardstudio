use super::super::*;

pub(super) struct WorkspaceEventWiringInput {
    pub runtime: Rc<Runtime>,
    pub adapter: SelectionAdapter,
    pub workspace: Signal<&'static str>,
    pub objects_open: Signal<bool>,
    pub inspect_open: Signal<bool>,
    pub inspector_panel_settings: Signal<PanelSettings>,
    pub setup_state: setup_workspace::SetupWorkspaceState,
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
    pub retry_save: EventHandler<()>,
    pub download_pending: EventHandler<()>,
    pub recover_saved: EventHandler<()>,
    pub canvas_end_pointer: EventHandler<PointerEvent>,
    pub canvas_cancel_pointer: EventHandler<PointerEvent>,
    pub canvas_keyboard: EventHandler<KeyboardEvent>,
    pub canvas_wheel: EventHandler<WheelEvent>,
}

#[allow(clippy::too_many_arguments)]
pub(super) fn use_workspace_event_wiring(
    input: WorkspaceEventWiringInput,
    workspace_routing: &mut WorkspaceRoutingSlots,
    select_tree: impl FnMut(objects::TreeSelectRequest) + 'static,
    navigate: impl FnMut((Scope, String, Option<String>)) + 'static,
    nudge_tree: impl FnMut(objects::TreeNudgeRequest) + 'static,
    on_parts_select: impl FnMut(()) + 'static,
    on_keymap_select: impl FnMut(String) + 'static,
    on_keycaps_select: impl FnMut(String) + 'static,
    on_keycaps_finding: impl FnMut(keycaps_fit::FindingNavigationRequest) + 'static,
    on_pcb_empty_hit: impl FnMut(PointerEvent) + 'static,
    on_pcb_part_hit: impl FnMut(pcb_scene::PcbPartHit) + 'static,
    on_mounted_module_select: impl FnMut(String) + 'static,
    on_pcb_part_pointer_down: impl FnMut(pcb_scene::PcbPartPointerDown) + 'static,
    on_pcb_wiring_edit_board: impl FnMut(()) + 'static,
    on_layout_selection_kind: impl FnMut(objects::LayoutSelectionKind) + 'static,
    on_layout_snap_intent: impl FnMut(objects::LayoutSnapIntent) + 'static,
    on_pcb_selection_kind: impl FnMut(objects::LayoutSelectionKind) + 'static,
    on_pcb_snap_intent: impl FnMut(objects::LayoutSnapIntent) + 'static,
    on_case_action: impl FnMut(case_workspace::TreeAction) + 'static,
    on_case_display: impl FnMut(case_workspace::DisplayRequest) + 'static,
    on_keymap_layer: impl FnMut(String) + 'static,
    on_keymap_export: impl FnMut(()) + 'static,
    on_show_configured_board: impl FnMut(String) + 'static,
    end_pointer: impl FnMut(PointerEvent) + 'static,
    cancel_pointer: impl FnMut(PointerEvent) + 'static,
    keyboard: impl FnMut(KeyboardEvent) + 'static,
    wheel: impl FnMut(WheelEvent) + 'static,
) -> (
    WorkspaceEventWiring,
    workspace_composition::SharedObjectsInput,
) {
    let WorkspaceEventWiringInput {
        runtime,
        adapter,
        workspace,
        objects_open,
        inspect_open,
        inspector_panel_settings,
        setup_state,
        layer_visibility,
        version,
    } = input;
    let undo = runtime.clone();
    let redo = runtime.clone();
    let retry = runtime.clone();
    let recover = runtime.clone();
    workspace_routing.select_tree.replace(Box::new(select_tree));
    workspace_routing.navigate.replace(Box::new(navigate));
    let nudge_tree = EventHandler::new(nudge_tree);
    let parts_select = EventHandler::new(on_parts_select);
    let keymap_select = EventHandler::new(on_keymap_select);
    let keycaps_select = EventHandler::new(on_keycaps_select);
    let keycaps_finding = EventHandler::new(on_keycaps_finding);
    let pcb_empty_hit = EventHandler::new(on_pcb_empty_hit);
    let pcb_part_hit = EventHandler::new(on_pcb_part_hit);
    let mounted_module_select = EventHandler::new(on_mounted_module_select);
    let pcb_part_pointer_down = EventHandler::new(on_pcb_part_pointer_down);
    let pcb_wiring_edit_board = EventHandler::new(on_pcb_wiring_edit_board);
    let on_layout_selection_kind_handler = EventHandler::new(on_layout_selection_kind);
    let layout_snap_intent = EventHandler::new(on_layout_snap_intent);
    let pcb_selection_kind = EventHandler::new(on_pcb_selection_kind);
    let pcb_snap_intent = EventHandler::new(on_pcb_snap_intent);
    let case_action = EventHandler::new(on_case_action);
    let case_display = EventHandler::new(on_case_display);
    let keymap_layer = EventHandler::new(on_keymap_layer);
    let keymap_export = EventHandler::new(on_keymap_export);
    let show_configured_board = EventHandler::new(on_show_configured_board);
    let open_geometry_scripts = {
        let mut workspace = workspace;
        let mut objects_open = objects_open;
        let mut inspect_open = inspect_open;
        let inspector_settings = inspector_panel_settings;
        EventHandler::new(move |_| {
            workspace.set("Layout");
            objects_open.set(false);
            inspect_open.set(true);
            setup_state.set_geometry_scripts_open(true);
            pin_inspector_on_desktop(inspector_settings);
        })
    };
    let toggle_footprints = EventHandler::new(move |_| {
        let mut footprints = layer_visibility.footprints;
        footprints.set(!footprints());
    });
    let retry_save = {
        let retry = retry.clone();
        EventHandler::new(move |_| {
            retry.submit(Event::RetrySave {
                operation_id: retry.operation(),
            });
        })
    };
    let download_pending = {
        let pending = runtime.clone();
        EventHandler::new(move |_| pending.download_pending_edits())
    };
    let recover_saved = {
        let recover = recover.clone();
        EventHandler::new(move |_| recover.recover_saved())
    };
    let canvas_end_pointer = EventHandler::new(end_pointer);
    let canvas_cancel_pointer = EventHandler::new(cancel_pointer);
    let canvas_keyboard = EventHandler::new(keyboard);
    let canvas_wheel = EventHandler::new(wheel);
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
        on_open_geometry_scripts: open_geometry_scripts,
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
            retry_save,
            download_pending,
            recover_saved,
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
            pcb_shared_objects.on_open_geometry_scripts = pcb_add_layout_open_handler(
                runtime.clone(),
                workspace,
                adapter.clone(),
                pcb_add_owner.clone(),
                layout_assembly_3d,
                pcb_shared_objects.on_open_geometry_scripts,
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

pub(super) struct OnLayoutViewModeArgs {
    pub runtime: Rc<Runtime>,
    pub adapter: SelectionAdapter,
    pub layout_owner: LayoutOwnerIdentity,
    pub workspace: Signal<&'static str>,
    pub layout_assembly_3d: Signal<bool>,
    pub layout_state: layout_workspace::LayoutWorkspaceState,
    pub drag: Rc<RefCell<Option<Drag>>>,
    pub svg: Rc<RefCell<Option<SvgElement>>>,
    pub render_scope: Scope,
    pub interaction_version: Signal<u64>,
    pub part_placement: part_placement::PartPlacementMount,
    pub matrix_placement: objects::MatrixPlacementMount,
    pub canvas_interaction: CanvasInteractionArbiter,
    pub mirrored_pair: objects::MirroredPairMount,
    pub matrix_setup: objects::MatrixSetupMount,
}

pub(super) fn on_layout_view_mode(input: OnLayoutViewModeArgs) -> EventHandler<bool> {
    let OnLayoutViewModeArgs {
        runtime,
        adapter,
        layout_owner,
        workspace,
        layout_assembly_3d,
        layout_state,
        drag,
        svg,
        render_scope,
        interaction_version,
        part_placement,
        matrix_placement,
        canvas_interaction,
        mirrored_pair,
        matrix_setup,
    } = input;
    layout_view_mode_handler(
        {
            let runtime = runtime.clone();
            let adapter = adapter.clone();
            let owner = layout_owner.clone();
            move || layout_owner_is_current(&runtime, workspace, &adapter, &owner)
        },
        layout_assembly_3d,
        {
            move |assembly_3d| {
                layout_state.set_assembly_3d(assembly_3d);
            }
        },
        LayoutPlacementCancellation {
            parts: part_placement.clone(),
            matrices: matrix_placement.clone(),
            interactions: canvas_interaction.clone(),
        },
        {
            let runtime = runtime.clone();
            let drag = drag.clone();
            let svg = svg.clone();
            let render_scope = render_scope.clone();
            move || {
                selection::cancel_scoped_drag(&runtime, &drag, &svg, Some(&render_scope));
                let mut interaction_version = interaction_version;
                interaction_version += 1;
            }
        },
        {
            let mirrored_pair = mirrored_pair.clone();
            let matrix_setup = matrix_setup.clone();
            let matrix_placement = matrix_placement.clone();
            move || {
                if let Some(active) = mirrored_pair.placement.as_ref() {
                    mirrored_pair.on_cancel.call(active.owner.clone());
                } else if let Some(form) = mirrored_pair.form.as_ref() {
                    mirrored_pair.on_cancel.call(form.owner.clone());
                }
                if let Some(projection) = matrix_setup.projection.as_ref() {
                    matrix_setup.on_cancel.call(projection.owner.clone());
                }
                if let Some(owner) = matrix_placement.cancel_owner.clone() {
                    matrix_placement.on_cancel.call(owner);
                }
            }
        },
    )
}

// GROUPING: allow until the Editor rewrite lands; remove then.
#[allow(dead_code)]
pub(super) struct WsWiring {
    pub wiring: WorkspaceEventWiring,
    pub pcb: PcbWiringBindings,
}

// GROUPING: allow until the Editor rewrite lands; remove then.
#[allow(dead_code)]
// Unnameable handler tuple cannot be factored; many params are inherent to the call.
#[allow(clippy::type_complexity, clippy::too_many_arguments)]
pub(super) fn use_workspace_wiring<
    'a,
    S1,
    S2,
    S3,
    S4,
    S5,
    S6,
    S7,
    S8,
    S9,
    S10,
    S11,
    S12,
    S13,
    S14,
    S15,
>(
    setup_a: &'a super::setup_shell::SetupA,
    setup_b: &super::setup_shell::SetupB,
    snapshot: &'a AcceptedSnapshot,
    render_scope: &Scope,
    matrix_setup: &objects::MatrixSetupMount,
    workspace_routing: &mut WorkspaceRoutingSlots,
    instance_scope_pending: bool,
    cam: &super::camera::CamNamed,
    sel: (
        super::selection_handlers::SelHandlersNamed,
        S1,
        S2,
        S3,
        S4,
        S5,
        S6,
        S7,
        S8,
        S9,
        S10,
        S11,
        S12,
        S13,
        S14,
        S15,
    ),
    navigate: impl FnMut((Scope, String, Option<String>)) + Clone + 'static,
    on_keycaps_finding: impl FnMut(keycaps_fit::FindingNavigationRequest) + 'static,
    on_show_configured_board: impl FnMut(String) + 'static,
    on_pcb_part_pointer_down: impl FnMut(pcb_scene::PcbPartPointerDown) + 'static,
    on_pcb_empty_hit: impl FnMut(PointerEvent) + 'static,
    on_parts_select: impl FnMut(()) + 'static,
    end_pointer: impl FnMut(PointerEvent) + Clone + 'static,
    cancel_pointer: impl FnMut(PointerEvent) + Clone + 'static,
    keyboard: impl FnMut(KeyboardEvent) + Clone + 'static,
    wheel: impl FnMut(WheelEvent) + Clone + 'static,
) -> (
    WsWiring,
    workspace_composition::WorkspaceObjectsInput<'a>,
    super::selection_handlers::SelHandlersNamed,
)
where
    S1: FnMut(objects::TreeSelectRequest) + 'static,
    S2: FnMut(objects::LayoutSelectionKind) + 'static,
    S3: FnMut(objects::LayoutSnapIntent) + 'static,
    S4: FnMut(objects::LayoutSelectionKind) + 'static,
    S5: FnMut(objects::LayoutSnapIntent) + 'static,
    S6: FnMut(String) + Clone + 'static,
    S7: FnMut(String) + Clone + 'static,
    S8: FnMut(pcb_scene::PcbPartHit) + 'static,
    S9: FnMut(String) + 'static,
    S10: FnMut(()) + 'static,
    S11: FnMut(case_workspace::TreeAction) + 'static,
    S12: FnMut(case_workspace::DisplayRequest) + 'static,
    S13: FnMut(String) + 'static,
    S14: FnMut(()) + 'static,
    S15: FnMut(objects::TreeNudgeRequest) + 'static,
{
    let (
        sel_named,
        select_tree,
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
    ) = sel;
    let runtime = &setup_a.runtime;
    let adapter = &setup_a.adapter;
    let workspace = setup_a.workspace;
    let objects_open = setup_a.objects_open;
    let inspect_open = setup_a.inspect_open;
    let inspector_panel_settings = setup_a.inspector_panel_settings;
    let setup_state = setup_a.setup_state;
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
    let layer_actions = &setup_b.layer_actions;
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
            retry_save,
            download_pending,
            recover_saved,
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
            objects_open,
            inspect_open,
            inspector_panel_settings,
            setup_state,
            layer_visibility,
            version,
        },
        &mut *workspace_routing,
        select_tree,
        navigate,
        nudge_tree,
        on_parts_select,
        on_keymap_select.clone(),
        on_keycaps_select.clone(),
        on_keycaps_finding,
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
        on_show_configured_board,
        end_pointer.clone(),
        cancel_pointer.clone(),
        keyboard.clone(),
        wheel.clone(),
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
                retry_save,
                download_pending,
                recover_saved,
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
        },
        objects_input,
        sel_named,
    )
}

// GROUPING: allow until the Editor rewrite lands; remove then.
#[allow(dead_code)]
pub(super) struct WsInputs {
    pub on_layout_view_mode: EventHandler<bool>,
    pub toolbar_input: workspace_composition::WorkspaceToolbarInput,
    pub canvas_input: Option<workspace_composition::WorkspaceCanvasInput>,
    pub on_component_inspector_action: EventHandler<inspector::LayoutComponentInspectorAction>,
    pub on_companion_placement_action: EventHandler<inspector::CompanionPlacementAction>,
}

// GROUPING: allow until the Editor rewrite lands; remove then.
#[allow(dead_code)]
#[allow(clippy::too_many_arguments, clippy::ptr_arg)] // &Vec keeps .clone() verbatim.
pub(super) fn use_workspace_inputs(
    setup_a: &super::setup_shell::SetupA,
    setup_b: &super::setup_shell::SetupB,
    cam: &super::camera::CamNamed,
    canvas: &super::canvas_state::CanvasNamed,
    wir: &WsWiring,
    snapshot: &AcceptedSnapshot,
    render_scope: &Scope,
    matrices: &Vec<&boardstudio_core::model::Matrix>,
    matrix_setup: &objects::MatrixSetupMount,
    instance_scope_pending: bool,
    start_pan: impl FnMut(PointerEvent) + Clone + 'static,
    move_pointer: impl FnMut(PointerEvent) + Clone + 'static,
    key_up: impl FnMut(KeyboardEvent) + Clone + 'static,
) -> WsInputs {
    let runtime = &setup_a.runtime;
    let adapter = &setup_a.adapter;
    let layout_owner = &setup_a.layout_owner;
    let workspace = setup_a.workspace;
    let layout_assembly_3d = setup_a.layout_assembly_3d;
    let layout_state = setup_a.layout_state;
    let drag = &cam.placement.drag;
    let svg = &cam.svg_surface.svg;
    let interaction_version = cam.placement.interaction_version;
    let part_placement = &cam.placement.part_placement;
    let matrix_placement = &setup_a.matrix_placement;
    let canvas_interaction = &setup_a.canvas_interaction;
    let mirrored_pair = &setup_a.mirrored_pair;
    let tree_cell_anchor = &setup_a.tree_cell_anchor;
    let accepted_canvas_source = &cam.scene_bounds.accepted_canvas_source;
    let model = &setup_a.model;
    let selected_tree_context = &canvas.selection.selected_tree_context;
    let active_workspace = setup_a.active_workspace;
    let show_position_inspector = canvas.inspector.show_position_inspector;
    let matrix_transform_inspector = &setup_a.matrix_transform_inspector;
    let objects_open = setup_a.objects_open;
    let inspect_open = setup_a.inspect_open;
    let inspector_panel_settings = setup_a.inspector_panel_settings;
    let layout_transform_tool = setup_a.layout_transform_tool;
    let on_layout_selection_kind_handler = wir.wiring.on_layout_selection_kind_handler;
    let layer_visibility = setup_a.layer_visibility;
    let layout_snap_settings = setup_a.layout_snap_settings;
    let layout_command_menu = setup_a.layout_command_menu;
    let toggle_footprints = wir.wiring.toggle_footprints;
    let retry_save = wir.wiring.retry_save;
    let download_pending = wir.wiring.download_pending;
    let recover_saved = wir.wiring.recover_saved;
    let layout_selection_kind = setup_a.layout_selection_kind;
    let component_inspector = &canvas.inspector.component_inspector;
    let pcb_selection_kind = wir.wiring.pcb_selection_kind;
    let pcb_snap_intent = wir.wiring.pcb_snap_intent;
    let layout_transform_target = &canvas.selection.layout_transform_target;
    let layout_align = &setup_a.layout_align;
    let layout_snap_intent = wir.wiring.layout_snap_intent;
    let pcb_align = &setup_a.pcb_align;
    let mount = canvas.inspector.mount;
    let canvas_end_pointer = wir.wiring.canvas_end_pointer;
    let canvas_cancel_pointer = wir.wiring.canvas_cancel_pointer;
    let canvas_keyboard = wir.wiring.canvas_keyboard;
    let canvas_wheel = wir.wiring.canvas_wheel;
    let view_box = &cam.scene_bounds.view_box;
    let render_generation = setup_a.render_generation;
    let keymap_view = &setup_b.keymap_view;
    let keymap_contours = &setup_b.keymap_contours;
    let keycaps_view = &setup_b.keycaps_view;
    let keycaps_contours = &setup_b.keycaps_contours;
    let keymap_select = wir.wiring.keymap_select;
    let keycaps_select = wir.wiring.keycaps_select;
    let mechanical_settings = &setup_b.mechanical_settings;
    let current_scope = &setup_a.current_scope;
    let parts_query = setup_a.parts_query;
    let parts_selection = setup_a.parts_selection;
    let pcb_empty_hit = wir.wiring.pcb_empty_hit;
    let pcb_part_hit = wir.wiring.pcb_part_hit;
    let pcb_part_pointer_down = wir.wiring.pcb_part_pointer_down;
    let mounted_module_select = wir.wiring.mounted_module_select;
    let layout_component_inspector_lifetime = &setup_a.layout_component_inspector_lifetime;
    let on_layout_view_mode = on_layout_view_mode(OnLayoutViewModeArgs {
        runtime: runtime.clone(),
        adapter: adapter.clone(),
        layout_owner: layout_owner.clone(),
        workspace,
        layout_assembly_3d,
        layout_state,
        drag: drag.clone(),
        svg: svg.clone(),
        render_scope: render_scope.clone(),
        interaction_version,
        part_placement: part_placement.clone(),
        matrix_placement: matrix_placement.clone(),
        canvas_interaction: canvas_interaction.clone(),
        mirrored_pair: mirrored_pair.clone(),
        matrix_setup: matrix_setup.clone(),
    });
    let toolbar_selection_availability = super::toolbar_inputs::use_toolbar_selection_availability(
        tree_cell_anchor.clone(),
        layout_owner.clone(),
        accepted_canvas_source.clone(),
        snapshot,
        model,
        selected_tree_context.clone(),
        active_workspace,
    );
    let toolbar_input =
        super::toolbar_inputs::toolbar_input(super::toolbar_inputs::ToolbarInputArgs {
            active_workspace,
            toolbar_selection_availability,
            selected_tree_context: selected_tree_context.clone(),
            show_position_inspector,
            matrix_transform_inspector: matrix_transform_inspector.clone(),
            runtime: runtime.clone(),
            adapter: adapter.clone(),
            workspace,
            objects_open,
            inspect_open,
            inspector_panel_settings,
            layout_owner: layout_owner.clone(),
            layout_transform_tool,
            on_layout_selection_kind_handler,
            matrices: matrices.clone(),
            layer_visibility,
            model,
            layout_snap_settings,
            layout_command_menu,
            layout_assembly_3d,
            on_layout_view_mode,
            toggle_footprints,
            retry_save,
            download_pending,
            recover_saved,
            layout_selection_kind,
            component_inspector: component_inspector.clone(),
            pcb_selection_kind,
            pcb_snap_intent,
            layout_transform_target: layout_transform_target.clone(),
            layout_align: layout_align.clone(),
            layout_snap_intent,
            pcb_align: pcb_align.clone(),
        });
    let canvas_input = super::canvas_inspector_inputs::canvas_input(
        super::canvas_inspector_inputs::CanvasInputArgs {
            mount,
            end_pointer: canvas_end_pointer,
            cancel_pointer: canvas_cancel_pointer,
            keyboard: canvas_keyboard,
            wheel: canvas_wheel,
            snapshot,
            render_scope: render_scope.clone(),
            view_box: view_box.clone(),
            model,
            render_generation,
            keymap_view: keymap_view.clone(),
            keymap_contours: keymap_contours.clone(),
            keycaps_view: keycaps_view.clone(),
            keycaps_contours: keycaps_contours.clone(),
            keymap_select,
            keycaps_select,
            mechanical_settings: mechanical_settings.clone(),
            instance_scope_pending,
            part_placement: part_placement.clone(),
            current_scope: current_scope.clone(),
            parts_query,
            parts_selection,
            workspace,
            active_workspace,
            pcb_empty_hit,
            pcb_part_hit,
            pcb_part_pointer_down,
            mounted_module_select,
        },
        start_pan.clone(),
        move_pointer.clone(),
        key_up.clone(),
    );
    let on_component_inspector_action =
        super::canvas_inspector_inputs::on_component_inspector_action_handler(
            runtime.clone(),
            adapter.clone(),
            layout_owner.clone(),
            layout_component_inspector_lifetime.clone(),
            workspace,
            inspect_open,
            layout_state,
        );
    let on_companion_placement_action =
        super::canvas_inspector_inputs::on_companion_placement_action_handler(
            runtime.clone(),
            adapter.clone(),
            workspace,
            layout_state,
        );
    WsInputs {
        on_layout_view_mode,
        toolbar_input,
        canvas_input,
        on_component_inspector_action,
        on_companion_placement_action,
    }
}
