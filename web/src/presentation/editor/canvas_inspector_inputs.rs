use super::super::*;

pub(super) struct CanvasInputArgs<'a> {
    pub mount: EventHandler<MountedEvent>,
    pub end_pointer: EventHandler<PointerEvent>,
    pub cancel_pointer: EventHandler<PointerEvent>,
    pub keyboard: EventHandler<KeyboardEvent>,
    pub pcb_empty_hit: EventHandler<PointerEvent>,
    pub pcb_part_hit: EventHandler<pcb_scene::PcbPartHit>,
    pub pcb_part_pointer_down: EventHandler<pcb_scene::PcbPartPointerDown>,
    pub mounted_module_select: EventHandler<String>,
    pub wheel: EventHandler<WheelEvent>,
    pub snapshot: &'a AcceptedSnapshot,
    pub render_scope: Scope,
    pub view_box: String,
    pub model: &'a ReadModel,
    pub render_generation: u64,
    pub keymap_view: Option<Rc<keymap::KeymapView>>,
    pub keymap_contours: Option<Rc<[Contour]>>,
    pub keycaps_view: Option<Rc<keycaps_scene::KeycapsView>>,
    pub keycaps_contours: Option<Rc<[Contour]>>,
    pub keymap_select: EventHandler<String>,
    pub keycaps_select: EventHandler<String>,
    pub mechanical_settings: mechanical_settings_mount::MechanicalSettingsMount,
    pub instance_scope_pending: bool,
    pub part_placement: part_placement::PartPlacementMount,
    pub current_scope: Option<Scope>,
    pub parts_query: parts::PartsQuery,
    pub parts_selection: parts::PartsSelection,
    pub workspace: Signal<&'static str>,
    pub active_workspace: &'static str,
}

pub(super) fn canvas_input(
    input: CanvasInputArgs<'_>,
    start_pan: impl FnMut(PointerEvent) + 'static,
    move_pointer: impl FnMut(PointerEvent) + 'static,
    key_up: impl FnMut(KeyboardEvent) + 'static,
) -> Option<workspace_composition::WorkspaceCanvasInput> {
    let CanvasInputArgs {
        mount,
        end_pointer,
        cancel_pointer,
        keyboard,
        wheel,
        snapshot,
        render_scope,
        view_box,
        model,
        render_generation,
        keymap_view,
        keymap_contours,
        keycaps_view,
        keycaps_contours,
        keymap_select,
        keycaps_select,
        mechanical_settings,
        instance_scope_pending,
        part_placement,
        current_scope,
        parts_query,
        parts_selection,
        workspace,
        active_workspace,
        pcb_empty_hit,
        pcb_part_hit,
        pcb_part_pointer_down,
        mounted_module_select,
    } = input;
    let canvas_handlers = workspace_composition::CanvasEventHandlers {
        mount,
        start_pan: EventHandler::new(start_pan),
        move_pointer: EventHandler::new(move_pointer),
        end_pointer,
        cancel_pointer,
        keyboard,
        key_up: EventHandler::new(key_up),
        wheel,
    };
    match active_workspace {
        "PCB" => Some(workspace_composition::WorkspaceCanvasInput::Pcb(Box::new(
            pcb_workspace::CanvasInput {
                snapshot: snapshot.clone(),
                scope: render_scope.clone(),
                view_box: view_box.clone(),
                selected_ids: model.selected_part_ids.clone(),
                generation: render_generation,
                handlers: canvas_handlers,
                on_empty_hit: pcb_empty_hit,
                on_part_hit: pcb_part_hit,
                on_part_pointer_down: pcb_part_pointer_down,
                on_module_select: mounted_module_select,
            },
        ))),
        "Keymap" => Some(workspace_composition::WorkspaceCanvasInput::Keymap(
            Box::new(keymap_workspace::CanvasInput {
                view: keymap_view.clone(),
                contours: keymap_contours
                    .clone()
                    .unwrap_or_else(|| Rc::<[Contour]>::from(Vec::new())),
                view_box: view_box.clone(),
                selected_ids: model.selected_part_ids.iter().cloned().collect(),
                handlers: canvas_handlers,
                on_select_key: keymap_select,
            }),
        )),
        "Keycaps" => Some(workspace_composition::WorkspaceCanvasInput::Keycaps(
            Box::new(keycaps_workspace::CanvasInput {
                view: keycaps_view.clone(),
                contours: keycaps_contours
                    .clone()
                    .unwrap_or_else(|| Rc::<[Contour]>::from(Vec::new())),
                view_box: view_box.clone(),
                selected_ids: model.selected_part_ids.iter().cloned().collect(),
                handlers: canvas_handlers,
                on_select_key: keycaps_select,
            }),
        )),
        "Case" => Some(workspace_composition::WorkspaceCanvasInput::Case(Box::new(
            case_workspace::CanvasInput {
                generation_ready: mechanical_settings.generation_ready,
                instance_scope_pending,
                mechanical_settings: mechanical_settings.props.clone(),
            },
        ))),
        "Parts" => Some(workspace_composition::WorkspaceCanvasInput::Parts(
            parts_workspace::CanvasInput {
                controller_back: part_placement.controller_back,
                snapshot: snapshot.clone(),
                scope: current_scope.clone(),
                query: parts_query,
                selected: parts_selection,
            },
        )),
        "Layout" | "Export" => None,
        _ => Some(workspace_composition::WorkspaceCanvasInput::Other(
            workspace_composition::PlaceholderInput {
                workspace,
                name: active_workspace,
                message: "Parts library editing is not available yet in the Rust interface.",
            },
        )),
    }
}

pub(super) fn on_component_inspector_action_handler(
    runtime: Rc<Runtime>,
    adapter: SelectionAdapter,
    layout_owner: LayoutOwnerIdentity,
    lifetime: Rc<inspector::LayoutComponentInspectorLifetime>,
    workspace: Signal<&'static str>,
    inspect_open: Signal<bool>,
    layout_state: layout_workspace::LayoutWorkspaceState,
) -> EventHandler<inspector::LayoutComponentInspectorAction> {
    let component_inspector_pending_edits = layout_state.component_inspector_edits();
    EventHandler::new({
        let runtime = runtime.clone();
        let adapter = adapter.clone();
        let owner = layout_owner.clone();
        move |action: inspector::LayoutComponentInspectorAction| {
            dispatch_layout_component_inspector_action(
                &runtime,
                &adapter,
                &owner,
                &lifetime,
                (workspace, inspect_open),
                component_inspector_pending_edits,
                action,
            )
        }
    })
}

pub(super) fn on_companion_placement_action_handler(
    runtime: Rc<Runtime>,
    adapter: SelectionAdapter,
    workspace: Signal<&'static str>,
    layout_state: layout_workspace::LayoutWorkspaceState,
) -> EventHandler<inspector::CompanionPlacementAction> {
    let (_companion_placement_pending, mut companion_placement_error, companion_placement_edits) =
        layout_state.companion_placement_controls();
    EventHandler::new({
        let runtime = runtime.clone();
        let adapter = adapter.clone();
        let owner_workspace = workspace;
        move |action: inspector::CompanionPlacementAction| {
            if !companion_placement_owner_is_current(
                &runtime,
                owner_workspace,
                &adapter,
                &action.owner,
                &action.pair_id,
                &action.targets,
            ) {
                companion_placement_error.set(Some(
                    "The selection changed before this edit ran; placement was not changed.".into(),
                ));
                return;
            }
            companion_placement_error.set(None);
            submit_layout_component_edit(
                &runtime,
                companion_placement_edits,
                layout_component_edits::InspectorField::CompanionPlacement,
                layout_component_edits::companion_placement_resolver(
                    action.pair_id,
                    action.targets,
                    action.mode,
                    action.tweak,
                ),
                None,
            );
        }
    })
}

pub(super) struct KeymapInspectorArgs<'a> {
    pub keymap_view: Option<Rc<keymap::KeymapView>>,
    pub render_scope: Scope,
    pub keymap_layer_id: Signal<String>,
    pub model: &'a ReadModel,
    pub keymap_layer: EventHandler<String>,
    pub keymap_export: EventHandler<()>,
    pub binding_actions: keymap::BindingActions,
    pub macro_actions: keymap::MacroActions,
    pub layer_actions: keymap::LayerActions,
    pub keymap_select: EventHandler<String>,
    pub snapshot: &'a AcceptedSnapshot,
    pub scope_generation: Signal<u64>,
}

pub(super) fn keymap_inspector_input(
    input: KeymapInspectorArgs<'_>,
) -> workspace_composition::WorkspaceInspectorInput {
    let KeymapInspectorArgs {
        keymap_view,
        render_scope,
        keymap_layer_id,
        model,
        keymap_layer,
        keymap_export,
        binding_actions,
        macro_actions,
        keymap_select,
        snapshot,
        scope_generation,
        layer_actions,
    } = input;
    workspace_composition::WorkspaceInspectorInput::Keymap(Box::new(
        keymap_workspace::InspectorInput {
            view: keymap_view.clone(),
            scope: render_scope.clone(),
            layer_actions,
            active_layer_id: keymap_layer_id(),
            selected_key_id: model.selected_part_ids.first().cloned(),
            on_layer: keymap_layer,
            on_export: keymap_export,
            firmware_export_enabled: keymap_view
                .as_ref()
                .is_some_and(|view| !view.keys.is_empty())
                || binding_actions
                    .encoder_projection
                    .as_ref()
                    .is_some_and(|projection| !projection.rows.is_empty()),
            on_select_key: keymap_select,
            binding_actions,
            macro_actions,
            admission_token: snapshot.token,
            admission_revision: snapshot.document.revision,
            scope_generation: scope_generation(),
        },
    ))
}

pub(super) struct CaseInspectorArgs<'a> {
    pub physical_setup_mount: pcb_physical_setup::PhysicalSetupMount,
    pub mechanical_settings: mechanical_settings_mount::MechanicalSettingsMount,
    pub instance_scope_pending: bool,
    pub current_scope: Option<Scope>,
    pub case_scene: Option<Rc<crate::runtime::CadScene>>,
    pub case_selection: case_viewer::CaseSelection,
    pub case_selected_layer_id: String,
    pub case_selected_context: Option<objects::ScopedTreeContext>,
    pub model: &'a ReadModel,
    pub selected_tree_context: Option<objects::ScopedTreeContext>,
    pub show_configured_board: EventHandler<String>,
    pub case_display: EventHandler<case_workspace::DisplayRequest>,
}

pub(super) fn case_inspector_input(
    input: CaseInspectorArgs<'_>,
) -> workspace_composition::WorkspaceInspectorInput {
    let CaseInspectorArgs {
        physical_setup_mount,
        mechanical_settings,
        instance_scope_pending,
        current_scope,
        case_scene,
        case_selection,
        case_selected_layer_id,
        case_selected_context,
        model,
        selected_tree_context,
        show_configured_board,
        case_display,
    } = input;
    workspace_composition::WorkspaceInspectorInput::Case(Box::new(case_workspace::InspectorInput {
        physical_setup: physical_setup_mount.clone(),
        mechanical_settings: mechanical_settings.clone(),
        instance_scope_pending,
        scope: current_scope.clone(),
        scene: case_scene,
        case_selection,
        selected_layer_id: case_selected_layer_id,
        selected_context: case_selected_context,
        selected_part_summary: case_workspace::selected_part_summary(
            model,
            selected_tree_context.as_ref(),
        ),
        on_show_configured_board: show_configured_board,
        on_display: case_display,
    }))
}

pub(super) struct PartsInspectorArgs<'a> {
    pub snapshot: &'a AcceptedSnapshot,
    pub current_scope: Option<Scope>,
    pub parts_query: parts::PartsQuery,
    pub parts_selection: parts::PartsSelection,
    pub selected_context: Signal<Option<objects::ScopedTreeContext>>,
    pub part_placement: part_placement::PartPlacementMount,
    pub matrix_placement: objects::MatrixPlacementMount,
    pub layout_target: Signal<Option<String>>,
    pub layout_selection_kind: Signal<objects::LayoutSelectionKind>,
    pub on_place_matrix_assembly: EventHandler<objects::MatrixPlacementSource>,
    pub on_open_module_placement: EventHandler<String>,
    pub on_module_attached: EventHandler<parts::AttachedModuleNavigation>,
}

pub(super) fn parts_inspector_input(
    input: PartsInspectorArgs<'_>,
) -> workspace_composition::WorkspaceInspectorInput {
    let PartsInspectorArgs {
        snapshot,
        current_scope,
        parts_query,
        parts_selection,
        selected_context,
        part_placement,
        matrix_placement,
        layout_target,
        on_place_matrix_assembly,
        on_open_module_placement,
        on_module_attached,
        layout_selection_kind,
    } = input;
    workspace_composition::WorkspaceInspectorInput::Parts(Box::new(
        parts_workspace::InspectorInput {
            snapshot: snapshot.clone(),
            scope: current_scope.clone(),
            query: parts_query,
            selected: parts_selection,
            selected_context,
            on_place_controller: part_placement.on_place_controller,
            on_place_component: part_placement.on_place_component,
            controller_placement_enabled: part_placement.controller_placement_enabled,
            placement_busy: part_placement.busy || matrix_placement.busy,
            placement_error: matrix_placement
                .error
                .clone()
                .or_else(|| part_placement.error.clone()),
            layout_target,
            on_place_assembly: on_place_matrix_assembly,
            on_board_placed: EventHandler::new(move |_| {
                let mut selection_kind = layout_selection_kind;
                selection_kind.set(objects::LayoutSelectionKind::Part);
            }),
            on_open_module_placement,
            on_module_attached,
        },
    ))
}

pub(super) struct PcbInspectorArgs<'a> {
    pub selected_tree_context: Option<objects::ScopedTreeContext>,
    pub render_scope: Scope,
    pub runtime: Rc<Runtime>,
    pub snapshot: &'a AcceptedSnapshot,
    pub selected_context: Signal<Option<objects::ScopedTreeContext>>,
    pub part_placement: part_placement::PartPlacementMount,
    pub pcb_wiring_source: Option<pcb_wiring::PcbWiringSource>,
    pub render_generation: u64,
    pub resolution: pcb_wiring::PcbWiringResolution,
    pub firmware_feedback: Option<pcb_wiring::FirmwarePositionFeedback>,
    pub firmware_editable: bool,
    pub firmware_on_edit: EventHandler<pcb_wiring::FirmwarePositionEditRequest>,
    pub part_net_actions: pcb_wiring::PartNetActions,
    pub part_input_actions: pcb_wiring::PartInputActions,
    pub pcb_wiring_edit_board: EventHandler<()>,
    pub mode_actions: pcb_wiring::BoardWiringModeActions,
    pub pin_actions: pcb_wiring::PcbWiringPinActions,
    pub apply_actions: pcb_wiring::BoardWiringApplyActions,
    pub on_resolve: EventHandler<()>,
    pub protected_remap_actions: pcb_wiring::ProtectedRemapActions,
}

pub(super) fn pcb_inspector_input(
    input: PcbInspectorArgs<'_>,
) -> workspace_composition::WorkspaceInspectorInput {
    let PcbInspectorArgs {
        selected_tree_context,
        render_scope,
        runtime,
        snapshot,
        selected_context,
        part_placement,
        pcb_wiring_source,
        render_generation,
        resolution,
        firmware_feedback,
        firmware_editable,
        firmware_on_edit,
        part_net_actions,
        part_input_actions,
        pcb_wiring_edit_board,
        mode_actions,
        pin_actions,
        apply_actions,
        protected_remap_actions,
        on_resolve,
    } = input;
    let selected_module = selected_tree_context.as_ref().and_then(|selected| {
        if selected.scope != render_scope {
            return None;
        }
        match &selected.context {
            objects::TreeContext::MountedModule {
                board_id,
                module_id,
            } if board_id == &render_scope.board_id => Some(module_id.clone()),
            _ => None,
        }
    });
    if let Some(module_id) = selected_module {
        workspace_composition::WorkspaceInspectorInput::PcbModule(Box::new(
            pcb_module_inspector::InspectorInput {
                runtime: runtime.clone(),
                snapshot: snapshot.clone(),
                scope: render_scope.clone(),
                module_id,
                selected_context,
                on_place_component: part_placement.on_place_component,
            },
        ))
    } else {
        workspace_composition::WorkspaceInspectorInput::Pcb(pcb_wiring_source.map(|source| {
            let firmware_position_projection =
                pcb_wiring::firmware_position_projection(&source, render_generation, &resolution);
            let firmware_feedback = firmware_feedback.filter(|feedback| {
                feedback.target.is_visible(
                    &source.ui_scope,
                    render_generation,
                    &firmware_position_projection,
                )
            });
            let firmware_controls = rsx! {
                firmware_positions::FirmwareKeymapPanel {
                    projection: firmware_position_projection.clone(),
                    feedback: firmware_feedback.clone(),
                    editable: firmware_editable,
                    on_change: firmware_on_edit,
                }
            };
            Box::new(pcb_wiring::PcbWiringInspectorProps {
                board_overview: true,
                source,
                resolution,
                firmware_positions: firmware_position_projection,
                firmware_feedback,
                firmware_controls,
                part_net_actions: part_net_actions.clone(),
                part_input_actions: part_input_actions.clone(),
                on_firmware_edit: firmware_on_edit,
                on_resolve,
                on_choose_controller: part_placement.on_choose_controller,
                on_edit_board_wiring: pcb_wiring_edit_board,
                mode_actions: mode_actions.clone(),
                pin_actions: pin_actions.clone(),
                apply_actions: apply_actions.clone(),
                protected_remap_actions: protected_remap_actions.clone(),
            })
        }))
    }
}

pub(super) struct KeycapsInspectorArgs<'a> {
    pub model: &'a ReadModel,
    pub current_scope: Option<Scope>,
    pub runtime: Rc<Runtime>,
    pub document: std::sync::Arc<boardstudio_core::model::ProjectDoc>,
    pub keycaps_view: Option<Rc<keycaps_scene::KeycapsView>>,
    pub keycaps_settings_actions: Option<keycaps_settings::KeycapsSettingsActions>,
    pub keycaps_select: EventHandler<String>,
    pub keycaps_fit_state: keycaps_fit::KeycapsFitActions,
    pub keycaps_finding: EventHandler<keycaps_fit::FindingNavigationRequest>,
}

pub(super) fn keycaps_inspector_input(
    input: KeycapsInspectorArgs<'_>,
) -> workspace_composition::WorkspaceInspectorInput {
    let KeycapsInspectorArgs {
        model,
        current_scope,
        runtime,
        document,
        keycaps_view,
        keycaps_settings_actions,
        keycaps_select,
        keycaps_fit_state,
        keycaps_finding,
    } = input;
    let selected_key_id = model.selected_part_ids.first().cloned();
    let mechanical_layer_ids = current_scope
        .as_ref()
        .zip(model.accepted.as_ref())
        .and_then(|(scope, snapshot)| current_case_layer_ids(&runtime, scope, snapshot))
        .unwrap_or_else(|| Rc::from([]));
    let settings_editor = keycaps_view.as_deref().and_then(|view| {
        let selected =
            keycaps_settings::project_selected_key(&document, view, selected_key_id.as_deref())?;
        keycaps_settings_actions
            .clone()
            .map(|actions| (selected, actions))
    });
    workspace_composition::WorkspaceInspectorInput::Keycaps(Box::new(
        keycaps_workspace::InspectorInput {
            view: keycaps_view.clone(),
            document: Rc::new((*document).clone()),
            selected_key_id,
            on_select_key: keycaps_select,
            settings_editor,
            settings_actions: keycaps_settings_actions.clone(),
            fit_state: keycaps_fit_state.state.clone(),
            mechanical_layer_ids,
            fit_retry: keycaps_fit_state.on_retry,
            fit_navigate: keycaps_finding,
        },
    ))
}

pub(super) struct LayoutInspectorArgs<'a> {
    pub layout_state: layout_workspace::LayoutWorkspaceState,
    pub geometry_scripts_open: Signal<bool>,
    pub setup_state: setup_workspace::SetupWorkspaceState,
    pub context_summary: &'a Option<context_summary::ContextSummary>,
    pub board_inspector_projection: Option<board_inspector::BoardInspectorProjection>,
    pub show_position_inspector: bool,
    pub component_inspector: Option<inspector::LayoutComponentInspectorProjection>,
    pub companion_placement: Option<inspector::CompanionPlacementProjection>,
    pub matrix_inspector: objects::MatrixInspectorMount,
    pub key_size: objects::KeySizeMount,
    pub matrix_transform_inspector: objects::MatrixTransformInspectorMount,
    pub pending_anchor_pick: Signal<Option<objects::MatrixTransformInspectorOwner>>,
    pub pending_splay_origin_pick: Signal<Option<objects::MatrixTransformInspectorOwner>>,
    pub selected_tree_context: Option<objects::ScopedTreeContext>,
    pub runtime: Rc<Runtime>,
    pub model: &'a ReadModel,
    pub select_tree: EventHandler<objects::TreeSelectRequest>,
    pub outline_inspector: Option<outline_lifecycle::OutlineInspectorProjection>,
    pub layout_findings: boardstudio_web_layout::layout_findings_state::LayoutFindingsState,
    pub layout_owner: LayoutOwnerIdentity,
    pub board_inspector: board_inspector::BoardInspectorMount,
    pub document: std::sync::Arc<boardstudio_core::model::ProjectDoc>,
    pub findings: Vec<LayoutFinding>,
    pub active_workspace: &'static str,
    pub render_scope: Scope,
    pub snapshot: &'a AcceptedSnapshot,
    pub render_generation: u64,
    pub on_close_layout_findings: Callback<(), ()>,
    pub on_layout_finding: Callback<layout_findings::Request, ()>,
    pub on_return_from_layout_finding: Callback<(), ()>,
    pub on_companion_placement_action: EventHandler<inspector::CompanionPlacementAction>,
    pub on_component_inspector_action: EventHandler<inspector::LayoutComponentInspectorAction>,
}

pub(super) fn layout_inspector_input(
    input: LayoutInspectorArgs<'_>,
) -> workspace_composition::WorkspaceInspectorInput {
    let LayoutInspectorArgs {
        layout_state,
        geometry_scripts_open,
        setup_state,
        context_summary,
        board_inspector_projection,
        show_position_inspector,
        component_inspector,
        companion_placement,
        matrix_inspector,
        key_size,
        matrix_transform_inspector,
        pending_anchor_pick,
        pending_splay_origin_pick,
        selected_tree_context,
        model,
        select_tree,
        outline_inspector,
        layout_findings,
        layout_owner,
        board_inspector,
        document,
        findings,
        active_workspace,
        render_scope,
        snapshot,
        render_generation,
        on_close_layout_findings,
        on_layout_finding,
        on_return_from_layout_finding,
        on_companion_placement_action,
        on_component_inspector_action,
        runtime,
    } = input;
    let component_inspector_pending_edits = layout_state.component_inspector_edits();
    let (
        component_inspector_x,
        component_inspector_y,
        component_inspector_margin,
        component_inspector_error,
    ) = layout_state.component_inspector_drafts();
    let (
        component_inspector_x_failure,
        component_inspector_y_failure,
        component_inspector_margin_failure,
    ) = layout_state.component_inspector_field_failures();
    let (
        component_inspector_layout_pending,
        component_inspector_constraint_pending,
        component_inspector_remove_pending,
        component_inspector_mirrored_half_pending,
    ) = layout_state.component_inspector_pending_controls();
    let (companion_placement_pending, companion_placement_error, companion_placement_edits) =
        layout_state.companion_placement_controls();
    workspace_composition::WorkspaceInspectorInput::Layout(Box::new(
        layout_workspace::InspectorInput {
            geometry_scripts_open: geometry_scripts_open(),
            on_close_geometry_scripts: EventHandler::new(move |()| {
                setup_state.set_geometry_scripts_open(false)
            }),
            context_title: context_summary
                .as_ref()
                .map(|summary| summary.title.clone())
                .or_else(|| {
                    board_inspector_projection
                        .as_ref()
                        .map(|board| board.board_name.clone())
                }),
            context_detail: context_summary
                .as_ref()
                .and_then(|summary| summary.detail.clone())
                .or_else(|| {
                    board_inspector_projection
                        .as_ref()
                        .map(|_| "Layout".to_owned())
                }),
            show_position_inspector,
            component_inspector,
            component_inspector_pending_edits,
            component_inspector_x,
            component_inspector_y,
            component_inspector_margin,
            component_inspector_error,
            component_inspector_x_failure,
            component_inspector_y_failure,
            component_inspector_margin_failure,
            component_inspector_layout_pending,
            component_inspector_constraint_pending,
            component_inspector_remove_pending,
            component_inspector_mirrored_half_pending,
            companion_placement,
            companion_placement_pending,
            companion_placement_error,
            companion_placement_edits,
            on_companion_placement_action,
            on_component_inspector_action,
            matrix_inspector,
            key_size,
            matrix_transform_inspector: matrix_transform_inspector.clone(),
            on_pick_splay_origin: {
                let mount = matrix_transform_inspector.clone();
                let mut pending = pending_splay_origin_pick;
                let mut anchor_pending = pending_anchor_pick;
                EventHandler::new(move |()| {
                    let Some(projection) = mount.projection.as_ref() else {
                        return;
                    };
                    if matches!(
                        &projection.owner.context,
                        objects::TreeContext::Column { .. }
                    ) && mount.editable
                    {
                        anchor_pending.set(None);
                        pending.set(Some(projection.owner.clone()));
                    }
                })
            },
            on_pick_anchor: {
                let mount = matrix_transform_inspector.clone();
                let mut pending = pending_anchor_pick;
                let mut splay_pending = pending_splay_origin_pick;
                EventHandler::new(move |()| {
                    let Some(projection) = mount.projection.as_ref() else {
                        return;
                    };
                    if matches!(
                        &projection.owner.context,
                        objects::TreeContext::Matrix { .. }
                    ) && mount.editable
                    {
                        splay_pending.set(None);
                        mount.clear_aggregate_pick();
                        pending.set(Some(projection.owner.clone()));
                    }
                })
            },
            anchor_pick_pending: pending_anchor_pick().is_some(),
            on_cancel_anchor_pick: {
                let mount = matrix_transform_inspector.clone();
                let mut pending = pending_anchor_pick;
                EventHandler::new(move |()| {
                    mount.clear_aggregate_pick();
                    pending.set(None);
                })
            },
            splay_origin_pick_pending: pending_splay_origin_pick().is_some(),
            on_cancel_splay_origin_pick: {
                let mut pending = pending_splay_origin_pick;
                EventHandler::new(move |()| pending.set(None))
            },
            inspector_tab: layout_state.inspector_tab(),
            matrix_relations: matrix_context_relations(model, selected_tree_context.as_ref()),
            relation_target: {
                let runtime = runtime.clone();
                Rc::new(move |request| relation_target_request(&runtime, request))
            },
            relation_scope: selected_tree_context
                .as_ref()
                .map(|selected| selected.scope.clone()),
            on_navigate_anchor_part: {
                let runtime = runtime.clone();
                let scope = selected_tree_context
                    .as_ref()
                    .map(|selected| selected.scope.clone());
                let select = select_tree;
                EventHandler::new(move |context: objects::TreeContext| {
                    let Some(scope) = scope.clone() else {
                        return;
                    };
                    let request = objects::TreeSelectRequest {
                        scope,
                        context,
                        mode: boardstudio_application::SelectionMode::Replace,
                        outline_action: None,
                    };
                    // A part key removed since the section was projected:
                    // do nothing; the next accepted revision re-projects.
                    let Some(request) = relation_target_request(&runtime, request) else {
                        return;
                    };
                    select.call(request);
                })
            },
            on_select_context: select_tree,
            outline_inspector: outline_inspector.clone().map(Box::new),
            findings_return_available: (layout_findings.return_target)().as_ref().is_some_and(
                |target| {
                    layout_finding_return_is_current(
                        model,
                        &layout_owner,
                        selected_tree_context.as_ref(),
                        target,
                    )
                },
            ),
            on_findings_return: on_return_from_layout_finding,
            board_inspector_mount: board_inspector,
            findings_page: Some(layout_findings::InspectorMount {
                open: (layout_findings.open)(),
                document: Rc::new(document.as_ref().clone()),
                findings,
                source: layout_findings::Source {
                    workspace: active_workspace,
                    scope: render_scope.clone(),
                    token: snapshot.token,
                    revision: snapshot.document.revision,
                    generation: render_generation,
                },
                on_close: on_close_layout_findings,
                on_navigate: on_layout_finding,
            }),
        },
    ))
}
