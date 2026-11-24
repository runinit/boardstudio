use super::super::*;
use super::scene::{CanvasSelectionState, use_canvas_selection_state};
use boardstudio_web_ui_model::Workspace;

pub(in crate::presentation) fn on_component_inspector_action_handler(
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

pub(in crate::presentation) fn on_companion_placement_action_handler(
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

pub(in crate::presentation) fn layout_inspector_input(
    frame: &super::super::workspace_composition::WorkspaceFrame,
    canvas: &CanvasNamed,
    mounts: &super::LayoutMounts,
    bindings: &super::toolbar_wiring::WsInputs,
    runtime: Rc<Runtime>,
) -> layout_workspace::InspectorInput {
    let services = frame.layout_services();
    let layout_state = frame.status.layout;
    let model = &frame.model;
    let layout_owner = &frame.owner;
    let layout_findings = services.layout_findings;
    let render_scope = &frame.scope;
    let snapshot = &frame.snapshot;
    let select_tree = frame.objects.on_select;
    let selected_tree_context = &canvas.selection.selected_tree_context;
    let board_inspector_projection = &canvas.inspector.board_inspector_projection;
    let matrix_transform_inspector = &services.matrix_transform_inspector;
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
    layout_workspace::InspectorInput {
        context_title: canvas
            .inspector
            .context_summary
            .as_ref()
            .map(|summary| summary.title.clone())
            .or_else(|| {
                board_inspector_projection
                    .as_ref()
                    .map(|board| board.board_name.clone())
            }),
        context_detail: canvas
            .inspector
            .context_summary
            .as_ref()
            .and_then(|summary| summary.detail.clone())
            .or_else(|| {
                board_inspector_projection
                    .as_ref()
                    .map(|_| "Layout".to_owned())
            }),
        show_position_inspector: canvas.inspector.show_position_inspector,
        component_inspector: canvas.inspector.component_inspector.clone(),
        component_inspector_x,
        component_inspector_y,
        component_inspector_margin,
        component_inspector_error,
        component_inspector_x_failure,
        component_inspector_y_failure,
        component_inspector_margin_failure,
        companion_placement: canvas.inspector.companion_placement.clone(),
        companion_placement_error,
        on_companion_placement_action: bindings.on_companion_placement_action,
        on_component_inspector_action: bindings.on_component_inspector_action,
        matrix_inspector: services.matrix_inspector.clone(),
        key_size: mounts.key_size.clone(),
        matrix_transform_inspector: services.matrix_transform_inspector.clone(),
        on_pick_splay_origin: {
            let mount = matrix_transform_inspector.clone();
            let mut pending = layout_state.pending_splay_origin_pick();
            let interaction = consume_context::<LayoutInteraction>();
            EventHandler::new(move |()| {
                let Some(projection) = mount.projection() else {
                    return;
                };
                if matches!(
                    &projection.owner.context,
                    objects::TreeContext::Column { .. }
                ) && mount.editable()
                    && interaction.switch_pick(CanvasMode::PickingSplayOrigin)
                {
                    pending.set(Some(projection.owner.clone()));
                    interaction.on_cancel(move || {
                        let mut pending = pending;
                        pending.set(None);
                    });
                }
            })
        },
        on_pick_anchor: {
            let mount = matrix_transform_inspector.clone();
            let mut pending = layout_state.pending_anchor_pick();
            let interaction = consume_context::<LayoutInteraction>();
            EventHandler::new(move |()| {
                let Some(projection) = mount.projection() else {
                    return;
                };
                if matches!(
                    &projection.owner.context,
                    objects::TreeContext::Matrix { .. }
                ) && mount.editable()
                    && interaction.switch_pick(CanvasMode::PickingMatrixAnchor)
                {
                    mount.clear_aggregate_pick();
                    pending.set(Some(projection.owner.clone()));
                    let mount = mount.clone();
                    interaction.on_cancel(move || {
                        let mut pending = pending;
                        mount.clear_aggregate_pick();
                        pending.set(None);
                    });
                }
            })
        },
        anchor_pick_pending: layout_state.pending_anchor_pick()().is_some(),
        on_cancel_anchor_pick: {
            let interaction = consume_context::<LayoutInteraction>();
            let mount = matrix_transform_inspector.clone();
            let mut pending = layout_state.pending_anchor_pick();
            EventHandler::new(move |()| {
                interaction.end(CanvasMode::PickingMatrixAnchor);
                mount.clear_aggregate_pick();
                pending.set(None);
            })
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
        outline_inspector: services.outline_inspector.clone().map(Box::new),
        findings_return_available: (layout_findings.return_target)().as_ref().is_some_and(
            |target| {
                layout_finding_return_is_current(
                    model,
                    layout_owner,
                    selected_tree_context.as_ref(),
                    target,
                )
            },
        ),
        on_findings_return: services.on_return_from_layout_finding,
        board_inspector_mount: mounts.board_inspector.clone(),
        findings_page: Some(layout_findings::InspectorMount {
            open: (layout_findings.open)(),
            document: Rc::new(frame.document.as_ref().clone()),
            findings: layout_findings_for_display(
                &frame.snapshot.scene.findings,
                frame
                    .model
                    .display_preview
                    .as_ref()
                    .map(|preview| preview.findings.as_slice()),
            ),
            source: layout_findings::Source {
                workspace: Workspace::Layout,
                scope: render_scope.clone(),
                token: snapshot.token,
                revision: snapshot.document.revision,
                generation: frame.owner.generation,
            },
            on_close: services.on_close_layout_findings,
            on_navigate: services.on_layout_finding,
        }),
    }
}

// --- Inspector projections and the combined canvas bundle ---

pub(in crate::presentation) struct InspectorProjections {
    pub component_inspector: Option<inspector::LayoutComponentInspectorProjection>,
    pub companion_placement: Option<inspector::CompanionPlacementProjection>,
    pub board_inspector_projection: Option<board_inspector::BoardInspectorProjection>,
    pub show_position_inspector: bool,
    pub context_summary: Option<context_summary::ContextSummary>,
    pub canvas_aria_label: &'static str,
}

pub(in crate::presentation) fn use_inspector_projections(
    frame: &super::super::workspace_composition::WorkspaceFrame,
    mounts: &super::LayoutMounts,
    selected_tree_context: Option<objects::ScopedTreeContext>,
    layout_component_inspector_lifetime: Rc<inspector::LayoutComponentInspectorLifetime>,
) -> InspectorProjections {
    let editor = use_context::<boardstudio_web_ui_model::editor::EditorContext>();
    let model = &frame.model;
    let active_workspace = Workspace::Layout;
    let adapter = use_context::<SelectionAdapter>();
    let accepted_token = frame.owner.token;
    let current_scope = frame.owner.scope.clone();
    let render_generation = frame.owner.generation;
    let board_inspector = mounts.board_inspector.clone();
    let matrix_inspector = frame.layout_services().matrix_inspector.clone();
    let key_size = mounts.key_size.clone();
    let matrix_transform_inspector = frame.layout_services().matrix_transform_inspector.clone();
    let outline_inspector = frame.layout_services().outline_inspector.clone();
    let runtime = editor.runtime();
    let matrix_placement = frame.layout_services().matrix_placement.clone();
    let component_inspector_key = layout_component_inspector_owner_key(
        model,
        active_workspace,
        selected_tree_context.as_ref(),
    );
    let component_inspector_generation =
        layout_component_inspector_lifetime.update(component_inspector_key);
    let component_inspector_scope_generation = (adapter.generation)();
    let component_inspector_identity = (
        accepted_token,
        current_scope.clone(),
        model.active_board_id.clone(),
        model.active_instance_id.clone(),
        model.selected_part_ids.clone(),
        selected_tree_context.clone(),
        component_inspector_generation,
        component_inspector_scope_generation,
        active_workspace,
    );
    let component_inspector = use_memo(use_reactive((&component_inspector_identity,), {
        let runtime = runtime.clone();
        move |(identity,)| {
            let (
                token,
                _scope,
                active_board_id,
                active_instance_id,
                selected_part_ids,
                selected,
                context_generation,
                scope_generation,
                workspace,
            ) = identity;
            if workspace != Workspace::Layout {
                return None;
            }
            let model = runtime.model();
            if model.accepted.as_ref().map(|snapshot| snapshot.token) != token
                || model.active_board_id != active_board_id
                || model.active_instance_id != active_instance_id
                || model.selected_part_ids != selected_part_ids
            {
                return None;
            }
            layout_component_inspector_projection(
                &model,
                selected.as_ref(),
                context_generation,
                scope_generation,
            )
        }
    }))
    .read()
    .clone();
    let companion_placement = (active_workspace == Workspace::Layout)
        .then(|| {
            companion_placement_projection(
                model,
                selected_tree_context.as_ref(),
                render_generation,
                render_generation,
            )
        })
        .flatten();
    let board_inspector_projection = board_inspector.projection.clone().filter(|_| {
        component_inspector.is_none()
            && !matrix_inspector.is_active()
            && key_size.projection.is_none()
            && !matrix_transform_inspector.is_active()
            && outline_inspector.is_none()
    });
    let show_position_inspector = board_inspector_projection.is_none()
        && component_inspector.is_none()
        && selected_tree_context.as_ref().is_none_or(|selected| {
            super::super::workspace_composition::selection_is_single_component(
                model,
                &selected.context,
            )
        });
    let context_summary = selected_tree_context
        .as_ref()
        .and_then(|selected| context_summary::summarize(model, &selected.context));
    let canvas_aria_label = if matrix_placement.placement.is_some() || matrix_placement.busy {
        "Matrix placement canvas. Move the pointer or use arrow keys; click or press Enter to place, Escape to cancel."
    } else {
        "Keyboard layout; drag components, hold Shift for range selection, hold Space and drag to pan, or use position controls"
    };
    InspectorProjections {
        component_inspector,
        companion_placement,
        board_inspector_projection,
        show_position_inspector,
        context_summary,
        canvas_aria_label,
    }
}

pub(in crate::presentation) struct CanvasNamed {
    pub selection: CanvasSelectionState,
    pub inspector: InspectorProjections,
}

pub(in crate::presentation) fn use_canvas_bundle(
    frame: &super::super::workspace_composition::WorkspaceFrame,
    mounts: &super::LayoutMounts,
    lifetime: Rc<inspector::LayoutComponentInspectorLifetime>,
) -> CanvasNamed {
    let selection = use_canvas_selection_state(frame);
    let inspector = use_inspector_projections(
        frame,
        mounts,
        selection.selected_tree_context.clone(),
        lifetime,
    );
    CanvasNamed {
        selection,
        inspector,
    }
}
