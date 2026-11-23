use super::super::*;
use boardstudio_web_ui_model::Workspace;

pub(in crate::presentation) struct LayoutPointer {
    pub matrix_pointer_down: EventHandler<layout_canvas::MatrixPointerDown>,
    pub part_pointer_down: EventHandler<layout_canvas::PartPointerDown>,
}

/// Layout's canvas pointer-down handlers, built inside `LayoutWorkspace` from
/// the frame and contexts instead of being forwarded through the session.
pub(in crate::presentation) fn build_canvas_pointer(
    frame: &workspace_composition::WorkspaceFrame,
) -> LayoutPointer {
    let editor = use_context::<boardstudio_web_ui_model::editor::EditorContext>();
    let adapter = use_context::<SelectionAdapter>();
    let interaction = use_context::<workspace_composition::WorkspaceInteraction>();
    let layout_state = use_context::<layout_workspace::LayoutWorkspaceState>();
    let services = frame.layout_services();
    let runtime = editor.runtime();
    let render_scope = frame.scope.clone();
    let render_generation = frame.owner.generation;
    let matrix_pointer_down = matrix_pointer_down_handler(MatrixPointerDownInput {
        runtime: runtime.clone(),
        adapter: adapter.clone(),
        svg: frame.svg.clone(),
        scope: render_scope.clone(),
        generation: render_generation,
        range_ids: services.visible_ids.clone(),
        selection_kind: layout_state.selection_kind(),
        owner: frame.owner.clone(),
        tree_cell_anchor: interaction.tree_cell_anchor.clone(),
        pending_splay_pick: layout_state.pending_splay_origin_pick(),
        pending_anchor_pick: layout_state.pending_anchor_pick(),
        transform_inspector: services.matrix_transform_inspector.clone(),
        split_focus: layout_state.split_focus(),
        workspace: editor.workspace,
        view_x: frame.camera.view_x,
        view_y: frame.camera.view_y,
        width: frame.camera.width,
        height: frame.camera.height,
    });
    let part_pointer_down = part_pointer_down_handler(PartPointerDownInput {
        runtime,
        svg: frame.svg.clone(),
        drag: interaction.navigation.drag_interaction(),
        navigation: interaction.navigation.clone(),
        adapter,
        render_scope,
        generation: render_generation,
        range_ids: services.visible_ids.clone(),
        selection_kind: layout_state.selection_kind(),
        owner: frame.owner.clone(),
        tree_cell_anchor: interaction.tree_cell_anchor.clone(),
        pending_splay_pick: layout_state.pending_splay_origin_pick(),
        transform_inspector: services.matrix_transform_inspector.clone(),
        split_focus: layout_state.split_focus(),
        workspace: editor.workspace,
        view_x: frame.camera.view_x,
        view_y: frame.camera.view_y,
        width: frame.camera.width,
        height: frame.camera.height,
    });
    let interaction = use_context::<LayoutInteraction>();
    let runtime = editor.runtime();
    let part_pointer_down = EventHandler::new(move |request: layout_canvas::PartPointerDown| {
        if interaction.current() == Some(CanvasMode::PickingMatrixAnchor) {
            if let Some(objects::TreeContext::Key {
                matrix_id,
                row,
                column,
            }) = runtime.with_model(|model| objects::context_for_part(model, &request.part_id))
            {
                matrix_pointer_down.call(layout_canvas::MatrixPointerDown {
                    event: request.event,
                    matrix_id,
                    row,
                    column,
                    part_id: Some(request.part_id),
                });
            }
        } else {
            part_pointer_down.call(request);
        }
    });
    LayoutPointer {
        matrix_pointer_down,
        part_pointer_down,
    }
}

pub(in crate::presentation) struct MatrixPointerDownInput {
    pub runtime: Rc<Runtime>,
    pub adapter: SelectionAdapter,
    pub svg: Rc<RefCell<Option<SvgElement>>>,
    pub scope: Scope,
    pub generation: u64,
    pub range_ids: Rc<Vec<String>>,
    pub selection_kind: Signal<objects::LayoutSelectionKind>,
    pub owner: LayoutOwnerIdentity,
    pub tree_cell_anchor: Rc<RefCell<Option<OwnedTreeCellAnchor>>>,
    pub pending_splay_pick: Signal<Option<objects::MatrixTransformInspectorOwner>>,
    pub pending_anchor_pick: Signal<Option<objects::MatrixTransformInspectorOwner>>,
    pub transform_inspector: objects::MatrixTransformInspectorMount,
    pub split_focus: Signal<boardstudio_web_ui_model::split_focus::SplitHalf>,
    pub workspace: Signal<Workspace>,
    pub view_x: f64,
    pub view_y: f64,
    pub width: f64,
    pub height: f64,
}

pub(in crate::presentation) fn matrix_pointer_down_handler(
    input: MatrixPointerDownInput,
) -> EventHandler<layout_canvas::MatrixPointerDown> {
    let canvas_interaction = use_context::<LayoutInteraction>();
    let MatrixPointerDownInput {
        runtime,
        adapter,
        svg,
        scope,
        generation,
        range_ids,
        selection_kind,
        owner,
        tree_cell_anchor,
        mut pending_splay_pick,
        mut pending_anchor_pick,
        transform_inspector,
        mut split_focus,
        workspace,
        view_x,
        view_y,
        width,
        height,
    } = input;
    EventHandler::new(move |request: layout_canvas::MatrixPointerDown| {
        let Some(pointer) = request.event.data().try_as_web_event() else {
            return;
        };
        canvas_interaction.sync_control(pointer.ctrl_key());
        if pointer.button() != 0
            || (canvas_interaction.space_down() && canvas_interaction.current().is_none())
            || !matches!(
                canvas_interaction.current(),
                None | Some(CanvasMode::PickingSplayOrigin | CanvasMode::PickingMatrixAnchor)
            )
        {
            return;
        }
        if pending_splay_pick.peek().is_some()
            && let Some(point) = coordinates(&svg, &pointer, view_x, view_y, width, height)
        {
            prevent_and_focus(&pointer, &svg);
            canvas_interaction.end(CanvasMode::PickingSplayOrigin);
            apply_pending_splay_origin_pick(
                &runtime,
                workspace,
                &adapter,
                &owner,
                PendingSplayOriginPick {
                    pending: &mut pending_splay_pick,
                    inspector: &transform_inspector,
                    point,
                    swing: pointer.ctrl_key(),
                },
            );
            return;
        }
        if pending_anchor_pick.peek().is_some() {
            // Key-identity hits retarget the anchor; the world point is
            // irrelevant, so no coordinate resolution can fail here. An
            // aggregate-armed pick appends to the pending parts instead and
            // stays armed for the next key.
            prevent_and_focus(&pointer, &svg);
            let aggregate_armed = transform_inspector.projection().is_some_and(
                |projection| {
                    matches!(
                        &projection.fields,
                        boardstudio_web_layout::matrix_transform_operation::MatrixTransformFields::Matrix {
                            anchor: Some(anchor),
                            ..
                        } if anchor.aggregate.is_some()
                    )
                },
            );
            if aggregate_armed {
                if !transform_inspector.pick_aggregate_part(
                    request.matrix_id.clone(),
                    request.row,
                    request.column,
                ) {
                    runtime.report("Matrix selection changed. Pick the anchor parts again.");
                }
                return;
            }
            canvas_interaction.end(CanvasMode::PickingMatrixAnchor);
            apply_pending_anchor_pick(
                &runtime,
                workspace,
                &adapter,
                &owner,
                PendingAnchorPick {
                    pending: &mut pending_anchor_pick,
                    inspector: &transform_inspector,
                    matrix_id: request.matrix_id.clone(),
                    row: request.row,
                    column: request.column,
                },
            );
            return;
        }
        prevent_and_focus(&pointer, &svg);
        let current = runtime.model();
        if !layout_owner_is_current(&runtime, workspace, &adapter, &owner) {
            return;
        }
        let Some(hit_context) =
            objects::context_for_cell(&current, &request.matrix_id, request.row, request.column)
        else {
            return;
        };
        // Ticket 14: clicking the dimmed half on the canvas moves the
        // view-only editing focus and still performs normal selection.
        if let Some(accepted) = current.accepted.as_ref() {
            let resolved = boardstudio_application::resolved_document(&accepted.document);
            if let Some(half) =
                boardstudio_web_ui_model::split_focus::context_half(resolved.as_ref(), &hit_context)
                && half != split_focus()
            {
                split_focus.set(half);
            }
        }
        update_tree_cell_anchor(&tree_cell_anchor, &owner, &current, Some(&hit_context));
        let retained = tree_cell_anchor_for_owner(&tree_cell_anchor, &owner);
        let context = objects::context_for_selection_kind(
            &current,
            &hit_context,
            selection_kind(),
            retained.as_ref(),
        )
        .map(|projection| projection.context)
        .unwrap_or_else(|| hit_context.clone());
        let mode = if pointer.shift_key() {
            SelectionMode::Range
        } else if pointer.ctrl_key() || pointer.meta_key() {
            SelectionMode::Toggle
        } else {
            SelectionMode::Replace
        };
        if let Some(target_part_id) = request.part_id {
            selection::submit_matrix_cell_selection(
                &runtime,
                &adapter,
                &scope,
                generation,
                selection::MatrixCellSelection {
                    matrix_id: request.matrix_id,
                    target_part_id,
                    hit_context: &hit_context,
                    context,
                    mode,
                },
            );
        } else {
            selection::submit_canvas_selection(
                &runtime,
                &adapter,
                &scope,
                generation,
                context,
                mode,
                if mode == SelectionMode::Range {
                    range_ids.as_ref().clone()
                } else {
                    Vec::new()
                },
            );
        }
    })
}

pub(in crate::presentation) struct PartPointerDownInput {
    pub runtime: Rc<Runtime>,
    pub svg: Rc<RefCell<Option<SvgElement>>>,
    pub drag: Rc<RefCell<Option<Drag>>>,
    pub navigation: CanvasNavigationState,
    pub adapter: SelectionAdapter,
    pub render_scope: Scope,
    pub generation: u64,
    pub range_ids: Rc<Vec<String>>,
    pub selection_kind: Signal<objects::LayoutSelectionKind>,
    pub owner: LayoutOwnerIdentity,
    pub tree_cell_anchor: Rc<RefCell<Option<OwnedTreeCellAnchor>>>,
    pub pending_splay_pick: Signal<Option<objects::MatrixTransformInspectorOwner>>,
    pub transform_inspector: objects::MatrixTransformInspectorMount,
    pub split_focus: Signal<boardstudio_web_ui_model::split_focus::SplitHalf>,
    pub workspace: Signal<Workspace>,
    pub view_x: f64,
    pub view_y: f64,
    pub width: f64,
    pub height: f64,
}

pub(in crate::presentation) fn part_pointer_down_handler(
    input: PartPointerDownInput,
) -> EventHandler<layout_canvas::PartPointerDown> {
    let canvas_interaction = use_context::<LayoutInteraction>();
    let PartPointerDownInput {
        runtime,
        svg,
        drag,
        navigation,
        adapter,
        render_scope,
        generation,
        range_ids,
        selection_kind,
        owner,
        tree_cell_anchor,
        mut pending_splay_pick,
        transform_inspector,
        mut split_focus,
        workspace,
        view_x,
        view_y,
        width,
        height,
    } = input;
    let mut selected_context = adapter.selected_context;
    EventHandler::new(move |request: layout_canvas::PartPointerDown| {
        let id = request.part_id;
        let Some(pointer) = request.event.data().try_as_web_event() else {
            return;
        };
        canvas_interaction.sync_control(pointer.ctrl_key());
        if pointer.button() != 0 {
            return;
        }
        if !matches!(
            canvas_interaction.current(),
            None | Some(CanvasMode::PickingSplayOrigin)
        ) {
            return;
        }
        if runtime.scope().as_ref() != Some(&render_scope) || (adapter.generation)() != generation {
            return;
        }
        if drag.borrow().is_some() || runtime.model().gesture.is_some() {
            return;
        }
        prevent_and_focus(&pointer, &svg);
        if pending_splay_pick.peek().is_some()
            && let Some(point) = coordinates(&svg, &pointer, view_x, view_y, width, height)
        {
            canvas_interaction.end(CanvasMode::PickingSplayOrigin);
            apply_pending_splay_origin_pick(
                &runtime,
                workspace,
                &adapter,
                &owner,
                PendingSplayOriginPick {
                    pending: &mut pending_splay_pick,
                    inspector: &transform_inspector,
                    point,
                    swing: pointer.ctrl_key(),
                },
            );
            return;
        }
        if canvas_interaction.space_down() && canvas_interaction.current().is_none() {
            workbench::editor_session::canvas_adapter::start_pan(
                &runtime,
                &navigation,
                &svg,
                &render_scope,
                generation,
                &pointer,
                canvas_interaction,
            );
            return;
        }
        if canvas_interaction.current().is_some() {
            return;
        }
        let Some(origin) = coordinates(&svg, &pointer, view_x, view_y, width, height) else {
            if let Some(surface) = svg.borrow().as_ref() {
                let _ = surface.release_pointer_capture(pointer.pointer_id());
            }
            return;
        };
        let mode = if pointer.shift_key() {
            SelectionMode::Range
        } else if pointer.ctrl_key() || pointer.meta_key() {
            SelectionMode::Toggle
        } else {
            SelectionMode::Replace
        };
        let current = runtime.model();
        if !layout_owner_is_current(&runtime, workspace, &adapter, &owner) {
            return;
        }
        let Some(hit_context) = objects::context_for_part(&current, &id) else {
            return;
        };
        // Ticket 14: clicking the dimmed half on the canvas moves the
        // view-only editing focus and still performs normal selection.
        if let Some(accepted) = current.accepted.as_ref() {
            let resolved = boardstudio_application::resolved_document(&accepted.document);
            if let Some(half) =
                boardstudio_web_ui_model::split_focus::context_half(resolved.as_ref(), &hit_context)
                && half != split_focus()
            {
                split_focus.set(half);
            }
        }
        update_tree_cell_anchor(&tree_cell_anchor, &owner, &current, Some(&hit_context));
        let retained = tree_cell_anchor_for_owner(&tree_cell_anchor, &owner);
        let selection_projection = objects::context_for_selection_kind(
            &current,
            &hit_context,
            selection_kind(),
            retained.as_ref(),
        );
        let target_ids = selection_projection
            .as_ref()
            .map(|projection| projection.part_ids.clone())
            .unwrap_or_else(|| {
                selection::resolve_context(&current, &hit_context).unwrap_or_default()
            });
        let context = selection_projection
            .map(|projection| projection.context)
            .unwrap_or_else(|| hit_context.clone());
        if let objects::TreeContext::Key {
            matrix_id,
            row,
            column,
        } = &hit_context
            && id == format!("matrix/{matrix_id}/r{row}c{column}")
        {
            selection::submit_matrix_cell_selection(
                &runtime,
                &adapter,
                &render_scope,
                generation,
                selection::MatrixCellSelection {
                    matrix_id: matrix_id.clone(),
                    target_part_id: id.clone(),
                    hit_context: &hit_context,
                    context: context.clone(),
                    mode,
                },
            );
        } else if !current.selected_part_ids.contains(&id)
            || mode != SelectionMode::Replace
            || target_ids != current.selected_part_ids
        {
            selection::submit_canvas_selection(
                &runtime,
                &adapter,
                &render_scope,
                generation,
                context.clone(),
                mode,
                if mode == SelectionMode::Range {
                    range_ids.as_ref().clone()
                } else {
                    Vec::new()
                },
            );
        } else {
            let anchor_valid = (adapter.anchor_scope)().as_ref() == Some(&render_scope)
                && current.selection_anchor_id.as_ref() == Some(&id)
                && selection::eligible_live_ids(&current).contains(&id);
            if !anchor_valid {
                selection::submit_canvas_selection(
                    &runtime,
                    &adapter,
                    &render_scope,
                    generation,
                    context,
                    SelectionMode::Add,
                    Vec::new(),
                );
            } else {
                selected_context.set(Some(objects::ScopedTreeContext {
                    scope: render_scope.clone(),
                    context,
                }));
            }
        }
        let current = runtime.model();
        if !current.selected_part_ids.contains(&id) || mode != SelectionMode::Replace {
            return;
        }
        let Some(snapshot) = current.accepted else {
            return;
        };
        let positions: Vec<_> = snapshot
            .document
            .parts
            .iter()
            .filter(|part| current.selected_part_ids.contains(&part.id))
            .map(|part| Position {
                id: part.id.clone(),
                at: part.pose.at,
            })
            .collect();
        if positions.is_empty() {
            return;
        }
        if !workbench::editor_session::canvas_adapter::capture_canvas_drag(
            canvas_interaction,
            workbench::editor_session::canvas_adapter::CanvasDragStart {
                mode: CanvasMode::DraggingParts,
                id: pointer.pointer_id(),
                client: client_point(&pointer),
                control: pointer.ctrl_key(),
            },
            &runtime,
            &drag,
            &svg,
        ) {
            return;
        }
        *drag.borrow_mut() = Some(Drag {
            pointer: i64::from(pointer.pointer_id()),
            scope: render_scope.clone(),
            generation,
            gesture_generation: None,
            origin,
            client_x: f64::from(pointer.client_x()),
            client_y: f64::from(pointer.client_y()),
            positions,
            last_sample: None,
            active: false,
            pan: false,
            camera: Vec2::default(),
        });
    })
}

fn prevent_and_focus(pointer: &web_sys::PointerEvent, svg: &Rc<RefCell<Option<SvgElement>>>) {
    pointer.prevent_default();
    pointer.stop_propagation();
    if let Some(surface) = svg.borrow().as_ref() {
        focus_svg(surface);
    }
}
