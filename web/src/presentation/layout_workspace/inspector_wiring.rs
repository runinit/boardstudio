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
    editor: boardstudio_web_ui_model::editor::EditorContext,
    state: &super::super::workbench::editor_session::session_state::SessionState,
    canvas: &CanvasNamed,
    callbacks: &super::super::workbench::editor_session::findings_navigation::LayoutFindingCallbacks,
    bindings: &super::toolbar_wiring::WsInputs,
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

// --- Inspector projections and the combined canvas bundle ---

pub(in crate::presentation) struct InspectorProjections {
    pub component_inspector: Option<inspector::LayoutComponentInspectorProjection>,
    pub companion_placement: Option<inspector::CompanionPlacementProjection>,
    pub board_inspector_projection: Option<board_inspector::BoardInspectorProjection>,
    pub show_position_inspector: bool,
    pub context_summary: Option<context_summary::ContextSummary>,
    pub mount: EventHandler<MountedEvent>,
    pub canvas_aria_label: &'static str,
}

pub(in crate::presentation) fn use_inspector_projections(
    state: &super::super::workbench::editor_session::session_state::SessionState,
    cam: &super::super::workbench::editor_session::camera::CamNamed,
    selected_tree_context: Option<objects::ScopedTreeContext>,
) -> InspectorProjections {
    let editor = use_context::<boardstudio_web_ui_model::editor::EditorContext>();
    let model = &state.view.model;
    let active_workspace = state.active_workspace;
    let layout_component_inspector_lifetime = state.layout_component_inspector_lifetime.clone();
    let adapter = state.adapter.clone();
    let accepted_token = state.view.layout_owner.token;
    let current_scope = state.view.current_scope.clone();
    let render_generation = state.render_generation;
    let board_inspector = state.board_inspector.clone();
    let matrix_inspector = state.matrix_inspector.clone();
    let key_size = state.key_size.clone();
    let matrix_transform_inspector = state.matrix_transform_inspector.clone();
    let outline_inspector = state.view.outline_inspector.clone();
    let zoom_surface_size = cam.svg_surface.zoom_surface_size;
    let runtime = editor.runtime();
    let svg = cam.svg_surface.svg.clone();
    let part_placement = cam.placement.part_placement.clone();
    let matrix_placement = state.matrix_pair.matrix_placement.clone();
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
            let resolved = selection::resolve_context(model, &selected.context);
            resolved.is_some_and(|ids| {
                ids.len() == 1 && model.selected_part_ids.as_slice() == ids.as_slice()
            }) && matches!(&selected.context, objects::TreeContext::Component { .. })
        });
    let context_summary = selected_tree_context
        .as_ref()
        .and_then(|selected| context_summary::summarize(model, &selected.context));
    let mount = EventHandler::new(CanvasNavigationState::mount_handler(
        zoom_surface_size,
        runtime.clone(),
        svg.clone(),
        part_placement.projection.is_some(),
    ));
    let placement_active = part_placement.projection.is_some()
        || part_placement.busy
        || matrix_placement.placement.is_some()
        || matrix_placement.busy;
    let canvas_aria_label = if matrix_placement.placement.is_some() || matrix_placement.busy {
        "Matrix placement canvas. Move the pointer or use arrow keys; click or press Enter to place, Escape to cancel."
    } else {
        "Keyboard layout; drag components, hold Shift for range selection, hold Space and drag to pan, or use position controls"
    };
    let focus_svg = svg.clone();
    use_effect(use_reactive!(|placement_active| {
        if !placement_active {
            return;
        }
        if let Some(element) = focus_svg.borrow().as_ref() {
            let options = web_sys::FocusOptions::new();
            options.set_prevent_scroll(true);
            let _ = element.focus_with_options(&options);
        }
    }));
    InspectorProjections {
        component_inspector,
        companion_placement,
        board_inspector_projection,
        show_position_inspector,
        context_summary,
        mount,
        canvas_aria_label,
    }
}

pub(in crate::presentation) struct CanvasNamed {
    pub selection: CanvasSelectionState,
    pub inspector: InspectorProjections,
}

pub(in crate::presentation) fn use_canvas_bundle(
    state: &super::super::workbench::editor_session::session_state::SessionState,
    views: &super::super::workbench::editor_session::setup_workspaces::Views,
    cam: &super::super::workbench::editor_session::camera::CamNamed,
    snapshot: &AcceptedSnapshot,
    render_scope: &Scope,
) -> CanvasNamed {
    let selection = use_canvas_selection_state(state, views, cam, snapshot, render_scope);
    let inspector = use_inspector_projections(state, cam, selection.selected_tree_context.clone());
    CanvasNamed {
        selection,
        inspector,
    }
}

/// Everything the Layout surfaces need from the session bundles, assembled
/// in one place so the editor session only routes by workspace.
pub(in crate::presentation) struct LayoutPayloadInput<'a> {
    pub editor: boardstudio_web_ui_model::editor::EditorContext,
    pub state: &'a super::super::workbench::editor_session::session_state::SessionState<'a>,
    pub cam: &'a super::super::workbench::editor_session::camera::CamNamed,
    pub canvas: &'a CanvasNamed,
    pub callbacks:
        &'a super::super::workbench::editor_session::findings_navigation::LayoutFindingCallbacks,
    pub inp: &'a super::toolbar_wiring::WsInputs,
    pub guide: &'a super::super::workbench::editor_session::setup_guide_state::GuideNamed,
    pub wir: &'a super::super::workbench::editor_session::workspace_inputs::WsWiring,
    pub matrix_setup: &'a objects::MatrixSetupMount,
    pub snapshot: &'a AcceptedSnapshot,
    pub select_tree: EventHandler<objects::TreeSelectRequest>,
    pub split_focused_half: Option<boardstudio_web_ui_model::split_focus::SplitHalf>,
    pub ptr: &'a super::super::workbench::editor_session::pointer_bundles::PtrNamed,
}

pub(in crate::presentation) fn layout_payload(
    input: LayoutPayloadInput<'_>,
) -> super::super::workspace_composition::WorkspacePayload {
    let LayoutPayloadInput {
        editor,
        state,
        cam,
        canvas,
        callbacks,
        inp,
        guide,
        wir,
        matrix_setup,
        snapshot,
        select_tree,
        split_focused_half,
        ptr,
    } = input;
    super::super::workspace_composition::WorkspacePayload::Layout {
        objects: super::ObjectsInput {
            shared: wir.shared_objects.clone(),
            matrix_setup: matrix_setup.clone(),
            matrix_inspector: state.matrix_inspector.clone(),
            pair_created: state.matrix_pair.pair_created_selection,
            on_place_component: cam.placement.part_placement.on_place_component,
            layout_target: state.view.layout_target,
            parts_query: state.view.parts_query,
            on_browse_parts: wir.on_browse_parts,
            placement_error: state
                .matrix_pair
                .matrix_placement
                .error
                .clone()
                .or_else(|| cam.placement.part_placement.error.clone()),
            add_object_menu_open: state.layout_state.add_object_menu_open(),
        },
        inspector: layout_inspector_input(editor, state, canvas, callbacks, inp, select_tree),
        toolbar: inp.toolbar_input.clone(),
        canvas_aria_label: canvas.inspector.canvas_aria_label,
        canvas_grid: guide.canvas_grid,
        outline_inspector: state.view.outline_inspector.clone(),
        outline_overlay_key: guide.outline_overlay_key.clone(),
        outline_snap_settings: guide.outline_snap_settings.clone(),
        outline_pitch: guide.outline_pitch,
        outline_snap_origins: guide.outline_snap_origins.clone(),
        selected_bridge: super::scene::selected_bridge(
            snapshot,
            canvas.selection.selected_tree_context.as_ref(),
        ),
        matrix_placement: state.matrix_pair.matrix_placement.clone(),
        mirrored_pair: state.matrix_pair.mirrored_pair.clone(),
        split_focused_half,
        spacing_ghost_prop: canvas.selection.spacing_ghost_prop.clone(),
        selected_tree_context: canvas.selection.selected_tree_context.clone(),
        canvas_anchor_badge: canvas.selection.canvas_anchor_badge.clone(),
        matrix_pointer_down: ptr.matrix_pointer_down,
        part_pointer_down: ptr.part_pointer_down,
        selection_geometry_anchor: guide.selection_geometry_anchor,
        canvas_transform_menu: Some(inp.toolbar_input.transform.clone()),
        matrix_transform_inspector: state.matrix_transform_inspector.clone(),
        drag: cam.placement.drag.clone(),
        layout_transform_target: canvas.selection.layout_transform_target.clone(),
        part_rail_selection: canvas.selection.part_rail_selection.clone(),
        part_rail_snap: canvas.selection.part_rail_snap,
    }
}
