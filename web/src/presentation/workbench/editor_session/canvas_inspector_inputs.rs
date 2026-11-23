use super::super::*;
use boardstudio_web_ui_model::Workspace;

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

pub(super) fn layout_inspector_input(
    editor: boardstudio_web_ui_model::editor::EditorContext,
    state: &super::session_state::SessionState,
    canvas: &super::canvas_state::CanvasNamed,
    callbacks: &super::findings_navigation::LayoutFindingCallbacks,
    bindings: &super::toolbar_inputs::WsInputs,
    select_tree: EventHandler<objects::TreeSelectRequest>,
) -> layout_workspace::InspectorInput {
    let layout_state = state.layout_state;
    let model = &state.view.model;
    let document = &state.views.document;
    let layout_owner = &state.view.layout_owner;
    let layout_findings = state.view.layout_findings;
    let render_scope = state.render_scope;
    let snapshot = state.snapshot;
    let runtime = editor.runtime();
    let selected_tree_context = &canvas.selection.selected_tree_context;
    let board_inspector_projection = &canvas.inspector.board_inspector_projection;
    let matrix_transform_inspector = &state.matrix_transform_inspector;
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
        matrix_inspector: state.matrix_inspector.clone(),
        key_size: state.key_size.clone(),
        matrix_transform_inspector: state.matrix_transform_inspector.clone(),
        on_pick_splay_origin: {
            let mount = matrix_transform_inspector.clone();
            let mut pending = layout_state.pending_splay_origin_pick();
            let mut anchor_pending = layout_state.pending_anchor_pick();
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
            let mut pending = layout_state.pending_anchor_pick();
            let mut splay_pending = layout_state.pending_splay_origin_pick();
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
        anchor_pick_pending: layout_state.pending_anchor_pick()().is_some(),
        on_cancel_anchor_pick: {
            let mount = matrix_transform_inspector.clone();
            let mut pending = layout_state.pending_anchor_pick();
            EventHandler::new(move |()| {
                mount.clear_aggregate_pick();
                pending.set(None);
            })
        },
        splay_origin_pick_pending: layout_state.pending_splay_origin_pick()().is_some(),
        on_cancel_splay_origin_pick: {
            let mut pending = layout_state.pending_splay_origin_pick();
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
        outline_inspector: state.view.outline_inspector.clone().map(Box::new),
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
        on_findings_return: callbacks.on_return_from_layout_finding,
        board_inspector_mount: state.board_inspector.clone(),
        findings_page: Some(layout_findings::InspectorMount {
            open: (layout_findings.open)(),
            document: Rc::new(document.as_ref().clone()),
            findings: state.views.findings.clone(),
            source: layout_findings::Source {
                workspace: Workspace::Layout,
                scope: render_scope.clone(),
                token: snapshot.token,
                revision: snapshot.document.revision,
                generation: state.render_generation,
            },
            on_close: callbacks.on_close_layout_findings,
            on_navigate: callbacks.on_layout_finding,
        }),
    }
}
