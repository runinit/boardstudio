use super::super::*;
use boardstudio_web_ui_model::Workspace;

pub(super) struct CanvasInputArgs<'a> {
    pub snapshot: &'a AcceptedSnapshot,
    pub mechanical_settings: mechanical_settings::MechanicalSettingsState,
    pub instance_scope_pending: bool,
    pub part_placement: part_placement::PartPlacementMount,
    pub current_scope: Option<Scope>,
    pub parts_query: parts::PartsQuery,
    pub parts_selection: parts::PartsSelection,
    pub active_workspace: Workspace,
}

pub(super) fn canvas_input(
    input: CanvasInputArgs<'_>,
) -> Option<workspace_composition::WorkspaceCanvasInput> {
    let CanvasInputArgs {
        snapshot,
        mechanical_settings,
        instance_scope_pending,
        part_placement,
        current_scope,
        parts_query,
        parts_selection,
        active_workspace,
    } = input;
    match active_workspace {
        Workspace::Case => Some(workspace_composition::WorkspaceCanvasInput::Case(Box::new(
            case_workspace::CanvasInput {
                generation_ready: mechanical_settings.generation_ready,
                instance_scope_pending,
                mechanical_settings: mechanical_settings.props.clone(),
            },
        ))),
        Workspace::Parts => Some(workspace_composition::WorkspaceCanvasInput::Parts(
            parts_workspace::CanvasInput {
                controller_back: part_placement.controller_back,
                snapshot: snapshot.clone(),
                scope: current_scope.clone(),
                query: parts_query,
                selected: parts_selection,
            },
        )),
        _ => None,
    }
}

pub(super) fn on_component_inspector_action_handler(
    runtime: Rc<Runtime>,
    adapter: SelectionAdapter,
    layout_owner: LayoutOwnerIdentity,
    lifetime: Rc<inspector::LayoutComponentInspectorLifetime>,
    workspace: Signal<Workspace>,
    inspect_open: Signal<bool>,
    layout_state: layout_workspace::LayoutWorkspaceState,
) -> EventHandler<inspector::LayoutComponentInspectorAction> {
    let (x, y, margin, error) = layout_state.component_inspector_drafts();
    let (x_failure, y_failure, margin_failure) = layout_state.component_inspector_field_failures();
    let views = layout_component_edits::ComponentEditViews {
        x: (x, x_failure),
        y: (y, y_failure),
        margin: (margin, margin_failure),
        error,
    };
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
                views,
                action,
            )
        }
    })
}

pub(super) fn on_companion_placement_action_handler(
    runtime: Rc<Runtime>,
    adapter: SelectionAdapter,
    workspace: Signal<Workspace>,
    layout_state: layout_workspace::LayoutWorkspaceState,
) -> EventHandler<inspector::CompanionPlacementAction> {
    let mut companion_placement_error = layout_state.companion_placement_error();
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
            let result = runtime.commit(
                "layout-companion-placement",
                layout_component_edits::companion_placement_resolver(
                    action.pair_id,
                    action.targets,
                    action.mode,
                    action.tweak,
                ),
            );
            companion_placement_error.set(result.err());
        }
    })
}

pub(super) struct CaseInspectorArgs<'a> {
    pub physical_setup_mount: pcb_physical_setup::PhysicalSetupMount,
    pub mechanical_settings: mechanical_settings::MechanicalSettingsState,
    pub instance_scope_pending: bool,
    pub current_scope: Option<Scope>,
    pub case_scene: Option<Rc<crate::runtime::CadScene>>,
    pub case_selection: shared_viewer::CaseSelection,
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

pub(super) struct LayoutInspectorArgs<'a> {
    pub layout_state: layout_workspace::LayoutWorkspaceState,
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
    pub active_workspace: Workspace,
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
    let companion_placement_error = layout_state.companion_placement_error();
    workspace_composition::WorkspaceInspectorInput::Layout(Box::new(
        layout_workspace::InspectorInput {
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
            component_inspector_x,
            component_inspector_y,
            component_inspector_margin,
            component_inspector_error,
            component_inspector_x_failure,
            component_inspector_y_failure,
            component_inspector_margin_failure,
            companion_placement,
            companion_placement_error,
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
                    let Some(projection) = mount.projection() else {
                        return;
                    };
                    if matches!(
                        &projection.owner.context,
                        objects::TreeContext::Column { .. }
                    ) && mount.editable()
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
                    let Some(projection) = mount.projection() else {
                        return;
                    };
                    if matches!(
                        &projection.owner.context,
                        objects::TreeContext::Matrix { .. }
                    ) && mount.editable()
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
