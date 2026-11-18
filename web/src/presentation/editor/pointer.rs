use super::super::*;

pub(super) struct MovePointerInput {
    pub navigation: CanvasNavigationState,
    pub runtime: Rc<Runtime>,
    pub svg: Rc<RefCell<Option<SvgElement>>>,
    pub drag: Rc<RefCell<Option<Drag>>>,
    pub adapter: SelectionAdapter,
    pub render_scope: Scope,
    pub owner: LayoutOwnerIdentity,
    pub snap_settings: Signal<objects::LayoutSnapSettings>,
    pub placement: part_placement::PartPlacementMount,
    pub matrix_placement: objects::MatrixPlacementMount,
    pub tree_cell_anchor: Rc<RefCell<Option<OwnedTreeCellAnchor>>>,
    pub mirrored_pair: objects::MirroredPairMount,
    pub canvas_interaction: CanvasInteractionArbiter,
    pub workspace: Signal<&'static str>,
    pub view_x: f64,
    pub view_y: f64,
    pub width: f64,
    pub height: f64,
    pub render_generation: u64,
}

pub(super) fn move_pointer_handler(
    input: MovePointerInput,
) -> impl FnMut(PointerEvent) + Clone + 'static {
    let MovePointerInput {
        navigation,
        runtime,
        svg,
        drag,
        adapter,
        render_scope,
        owner,
        snap_settings,
        placement,
        matrix_placement,
        tree_cell_anchor,
        mirrored_pair,
        canvas_interaction,
        workspace,
        view_x,
        view_y,
        width,
        height,
        render_generation,
    } = input;
    move |event: PointerEvent| {
        let Some(pointer) = event.data().try_as_web_event() else {
            return;
        };
        if canvas_interaction.is_owner(CanvasInteractionOwner::MatrixPlacement) {
            let Some(active) = matrix_placement.placement.as_ref() else {
                pointer.prevent_default();
                pointer.stop_propagation();
                return;
            };
            let owner = active.owner.clone();
            if workspace() != "Layout"
                || runtime.scope().as_ref() != Some(&owner.scope)
                || (adapter.generation)() != owner.scope_generation
            {
                matrix_placement.on_cancel.call(owner);
                return;
            }
            if let Some(center) = coordinates(&svg, &pointer, view_x, view_y, width, height) {
                matrix_placement
                    .on_move
                    .call(objects::MatrixPlacementMove { owner, center });
            }
            return;
        }
        if canvas_interaction.is_owner(CanvasInteractionOwner::MirroredPair) {
            let Some(placement) = mirrored_pair.placement.as_ref() else {
                pointer.prevent_default();
                pointer.stop_propagation();
                return;
            };
            let pair_owner = placement.owner.clone();
            if workspace() != "Layout"
                || runtime.scope().as_ref() != Some(&pair_owner.scope)
                || (adapter.generation)() != pair_owner.scope_generation
            {
                mirrored_pair.on_cancel.call(pair_owner);
                return;
            }
            if let Some(center) = coordinates(&svg, &pointer, view_x, view_y, width, height) {
                mirrored_pair.on_move.call(objects::MirroredPairMove {
                    owner: pair_owner,
                    center,
                });
            }
            return;
        }
        if canvas_interaction.is_owner(CanvasInteractionOwner::PartPlacement) {
            if let Some(active) = placement.projection.clone()
                && let Some(point) = coordinates(&svg, &pointer, view_x, view_y, width, height)
            {
                let model = runtime.model();
                let at = model.accepted.as_ref().map_or(point, |_snapshot| {
                    part_placement::snap_placement_at(
                        &active.snap_document,
                        &active.owner.board_id,
                        &active.pending,
                        point,
                        {
                            let settings = snap_settings.read();
                            part_placement::PlacementSnapOptions {
                                snap_fraction: settings.snap_fraction,
                                geometry_snap: settings.geometry_snap,
                                gap: part_placement::placement_gap(&settings),
                                free: pointer.alt_key(),
                                // Ticket 22: component placement snaps in the selected
                                // matrix pitch (19.05 mm only when none is selected).
                                pitch: matrix_snap_parameters(
                                    &model,
                                    &active.owner.scope,
                                    &adapter,
                                    None,
                                )
                                .0,
                            }
                        },
                    )
                });
                placement.on_move.call(at);
            } else {
                pointer.prevent_default();
                pointer.stop_propagation();
            }
            return;
        }
        let Some(mut current) = drag
            .borrow()
            .clone()
            .filter(|d| d.pointer == i64::from(pointer.pointer_id()))
        else {
            return;
        };
        if current.scope != render_scope || current.generation != render_generation {
            return;
        }
        if !canvas_owner_is_current(&runtime, workspace, &adapter, &owner) {
            selection::cancel_drag_if_owned(&runtime, &current);
            drag.borrow_mut().take();
            if let Some(element) = svg.borrow().as_ref() {
                let _ = element.release_pointer_capture(pointer.pointer_id());
            }
            return;
        }
        if runtime.scope().as_ref() != Some(&current.scope)
            || (adapter.generation)() != current.generation
        {
            if let Some(element) = svg.borrow().as_ref() {
                let _ = element.release_pointer_capture(pointer.pointer_id());
            }
            let mut stored = drag.borrow_mut();
            if stored.as_ref().is_some_and(|stored| {
                stored.pointer == current.pointer
                    && stored.scope == current.scope
                    && stored.generation == current.generation
            }) {
                stored.take();
            }
            return;
        }
        if current.active && !current.pan && !selection::owns_session_gesture(&runtime, &current) {
            if let Some(element) = svg.borrow().as_ref() {
                let _ = element.release_pointer_capture(pointer.pointer_id());
            }
            drag.borrow_mut().take();
            return;
        }
        if navigation.consume_pan_pointer_move(current.pointer, &current.scope, current.generation)
        {
            return;
        }
        let Some((point, scale)) =
            coordinates_with_scale(&svg, &pointer, view_x, view_y, width, height)
        else {
            return;
        };
        if !current.active {
            if !canvas_interaction::pending_part_drag_threshold_reached(
                (current.client_x, current.client_y),
                (f64::from(pointer.client_x()), f64::from(pointer.client_y())),
            ) {
                return;
            }
            if runtime.model().gesture.is_some() {
                drag.borrow_mut().take();
                if let Some(element) = svg.borrow().as_ref() {
                    let _ = element.release_pointer_capture(pointer.pointer_id());
                }
                return;
            }
            if !canvas_owner_is_current(&runtime, workspace, &adapter, &owner)
                || owner.scope.as_ref() != Some(&current.scope)
            {
                drag.borrow_mut().take();
                if let Some(element) = svg.borrow().as_ref() {
                    let _ = element.release_pointer_capture(pointer.pointer_id());
                }
                return;
            }
            let current_model = runtime.model();
            let retained_cell = tree_cell_anchor_for_owner(&tree_cell_anchor, &owner);
            let (matrix_pitch, matrix_gap) = matrix_snap_parameters(
                &current_model,
                &current.scope,
                &adapter,
                retained_cell.as_ref(),
            );
            let settings = snap_settings.read().clone();
            let snap = objects::gesture_snap_inputs(&settings, matrix_pitch, matrix_gap);
            let target_ids: Vec<_> = current
                .positions
                .iter()
                .map(|position| position.id.clone())
                .collect();
            let operation = runtime.operation();
            runtime.submit(Event::GestureBegin {
                operation_id: operation,
                pointer_id: current.pointer,
                target_ids: target_ids.clone(),
                transaction_id: format!("drag-{}", operation.0),
                start: current.positions.clone(),
                pitch: snap.pitch,
                snap_fraction: snap.snap_fraction,
                geometry_snap: snap.geometry_snap,
                gap: snap.gap,
                pixels_per_world: Some(scale),
                alt: pointer.alt_key(),
            });
            let fresh = runtime.model();
            let gesture = fresh.gesture.filter(|gesture| {
                gesture.pointer_id == current.pointer && gesture.target_ids == target_ids
            });
            if gesture.as_ref().is_none_or(|_| {
                runtime.scope().as_ref() != Some(&current.scope)
                    || (adapter.generation)() != current.generation
            }) {
                drag.borrow_mut().take();
                if let Some(element) = svg.borrow().as_ref() {
                    let _ = element.release_pointer_capture(pointer.pointer_id());
                }
                return;
            }
            current.active = true;
            current.gesture_generation = gesture.map(|gesture| gesture.generation);
            *drag.borrow_mut() = Some(current.clone());
        }
        runtime.submit(Event::GestureSample {
            pointer_id: current.pointer,
            positions: moved(&current, point),
            pixels_per_world: Some(scale),
            alt: pointer.alt_key(),
        });
    }
}

pub(super) struct EndPointerInput {
    pub navigation: CanvasNavigationState,
    pub runtime: Rc<Runtime>,
    pub svg: Rc<RefCell<Option<SvgElement>>>,
    pub drag: Rc<RefCell<Option<Drag>>>,
    pub adapter: SelectionAdapter,
    pub owner: LayoutOwnerIdentity,
    pub placement: part_placement::PartPlacementMount,
    pub matrix_placement: objects::MatrixPlacementMount,
    pub snap_settings: Signal<objects::LayoutSnapSettings>,
    pub render_scope: Scope,
    pub canvas_interaction: CanvasInteractionArbiter,
    pub workspace: Signal<&'static str>,
    pub view_x: f64,
    pub view_y: f64,
    pub width: f64,
    pub height: f64,
    pub render_generation: u64,
    pub interaction_version: Signal<u64>,
}

pub(super) fn end_pointer_handler(
    input: EndPointerInput,
) -> impl FnMut(PointerEvent) + Clone + 'static {
    let EndPointerInput {
        navigation,
        runtime,
        svg,
        drag,
        adapter,
        owner,
        placement,
        matrix_placement,
        snap_settings,
        render_scope,
        canvas_interaction,
        workspace,
        view_x,
        view_y,
        width,
        height,
        render_generation,
        interaction_version,
    } = input;
    move |event: PointerEvent| {
        let Some(pointer) = event.data().try_as_web_event() else {
            return;
        };
        if canvas_interaction.is_owner(CanvasInteractionOwner::MatrixPlacement) {
            pointer.prevent_default();
            pointer.stop_propagation();
            if pointer.button() == 0
                && let Some(active) = matrix_placement.placement.as_ref()
            {
                let owner = active.owner.clone();
                if workspace() != "Layout"
                    || runtime.scope().as_ref() != Some(&owner.scope)
                    || (adapter.generation)() != owner.scope_generation
                {
                    matrix_placement.on_cancel.call(owner);
                } else if let Some(center) =
                    coordinates(&svg, &pointer, view_x, view_y, width, height)
                {
                    matrix_placement
                        .on_commit
                        .call(objects::MatrixPlacementMove { owner, center });
                }
            }
            return;
        }
        if canvas_interaction.is_owner(CanvasInteractionOwner::MirroredPair) {
            pointer.prevent_default();
            pointer.stop_propagation();
            return;
        }
        if canvas_interaction.is_owner(CanvasInteractionOwner::PartPlacement) {
            pointer.prevent_default();
            pointer.stop_propagation();
            if placement.projection.is_none()
                || !part_placement::pointer_release_commits(pointer.button())
            {
                return;
            }
            if let Some(point) = coordinates(&svg, &pointer, view_x, view_y, width, height) {
                let model = runtime.model();
                let at = model.accepted.as_ref().map_or(point, |_snapshot| {
                    placement.projection.as_ref().map_or(point, |active| {
                        part_placement::snap_placement_at(
                            &active.snap_document,
                            &active.owner.board_id,
                            &active.pending,
                            point,
                            {
                                let settings = snap_settings.read();
                                part_placement::PlacementSnapOptions {
                                    snap_fraction: settings.snap_fraction,
                                    geometry_snap: settings.geometry_snap,
                                    gap: part_placement::placement_gap(&settings),
                                    free: pointer.alt_key(),
                                    // Ticket 22: component placement snaps in the selected
                                    // matrix pitch (19.05 mm only when none is selected).
                                    pitch: matrix_snap_parameters(
                                        &model,
                                        &active.owner.scope,
                                        &adapter,
                                        None,
                                    )
                                    .0,
                                }
                            },
                        )
                    })
                });
                placement.on_commit.call(at);
            }
            return;
        }
        let Some(current) = drag
            .borrow()
            .clone()
            .filter(|d| d.pointer == i64::from(pointer.pointer_id()))
        else {
            return;
        };
        if !canvas_owner_is_current(&runtime, workspace, &adapter, &owner) {
            selection::cancel_drag_if_owned(&runtime, &current);
            drag.borrow_mut().take();
            if let Some(element) = svg.borrow().as_ref() {
                let _ = element.release_pointer_capture(pointer.pointer_id());
            }
            return;
        }
        let mut interaction_version = interaction_version;
        interaction_version += 1;
        if current.scope != render_scope || current.generation != render_generation {
            return;
        }
        if runtime.scope().as_ref() != Some(&current.scope)
            || (adapter.generation)() != current.generation
        {
            if let Some(element) = svg.borrow().as_ref() {
                let _ = element.release_pointer_capture(pointer.pointer_id());
            }
            let mut stored = drag.borrow_mut();
            if stored.as_ref().is_some_and(|stored| {
                stored.pointer == current.pointer
                    && stored.scope == current.scope
                    && stored.generation == current.generation
            }) {
                stored.take();
            }
            return;
        }
        if current.active && !current.pan && !selection::owns_session_gesture(&runtime, &current) {
            if let Some(element) = svg.borrow().as_ref() {
                let _ = element.release_pointer_capture(pointer.pointer_id());
            }
            drag.borrow_mut().take();
            return;
        }
        if current.pan {
            navigation.finish_pan(
                &runtime,
                &svg,
                pointer.pointer_id(),
                (f64::from(pointer.client_x()), f64::from(pointer.client_y())),
                (width, height),
            );
            return;
        }
        if current.active
            && let Some((point, scale)) =
                coordinates_with_scale(&svg, &pointer, view_x, view_y, width, height)
        {
            runtime.submit(Event::GestureEnd {
                pointer_id: current.pointer,
                final_positions: moved(&current, point),
                pixels_per_world: Some(scale),
                alt: pointer.alt_key(),
            });
        } else if current.active {
            selection::cancel_drag_if_owned(&runtime, &current);
        }
        drag.borrow_mut().take();
        if let Some(element) = svg.borrow().as_ref() {
            let _ = element.release_pointer_capture(pointer.pointer_id());
        }
    }
}

pub(super) struct CancelPointerInput {
    pub runtime: Rc<Runtime>,
    pub svg: Rc<RefCell<Option<SvgElement>>>,
    pub drag: Rc<RefCell<Option<Drag>>>,
    pub placement: part_placement::PartPlacementMount,
    pub matrix_placement: objects::MatrixPlacementMount,
    pub render_scope: Scope,
    pub canvas_interaction: CanvasInteractionArbiter,
    pub render_generation: u64,
    pub interaction_version: Signal<u64>,
}

pub(super) fn cancel_pointer_handler(
    input: CancelPointerInput,
) -> impl FnMut(PointerEvent) + Clone + 'static {
    let CancelPointerInput {
        runtime,
        svg,
        drag,
        placement,
        matrix_placement,
        render_scope,
        canvas_interaction,
        render_generation,
        interaction_version,
    } = input;
    move |event: PointerEvent| {
        match canvas_interaction.current() {
            Some(CanvasInteractionOwner::MatrixPlacement) => {
                if let Some(owner) = matrix_placement.cancel_owner.clone() {
                    matrix_placement.on_cancel.call(owner);
                }
                return;
            }
            Some(CanvasInteractionOwner::MirroredPair) => return,
            Some(CanvasInteractionOwner::PartPlacement) => {
                if placement.projection.is_some() {
                    placement.on_cancel.call(());
                }
                return;
            }
            Some(CanvasInteractionOwner::OutlinePerimeter) => return,
            Some(CanvasInteractionOwner::MatrixTransform) => return,
            None => {}
        }
        let pointer_id = event
            .data()
            .try_as_web_event()
            .map(|event| event.pointer_id());
        let Some(current) = drag.borrow().clone().filter(|current| {
            current.scope == render_scope
                && current.generation == render_generation
                && pointer_id.is_none_or(|pointer| current.pointer == i64::from(pointer))
        }) else {
            return;
        };
        let mut interaction_version = interaction_version;
        interaction_version += 1;
        drag.borrow_mut().take();
        if let Some(element) = svg.borrow().as_ref() {
            let _ = element.release_pointer_capture(current.pointer as i32);
        }
        if current.scope == render_scope
            && current.generation == render_generation
            && runtime.scope().as_ref() == Some(&current.scope)
            && current.active
            && !current.pan
        {
            selection::cancel_drag_if_owned(&runtime, &current);
        }
    }
}

pub(super) fn use_pointer_workspace_effect(
    active_workspace: &'static str,
    runtime: Rc<Runtime>,
    drag: Rc<RefCell<Option<Drag>>>,
    svg: Rc<RefCell<Option<SvgElement>>>,
    space_down: Rc<Cell<bool>>,
    render_scope: Scope,
    interaction_version: Signal<u64>,
) {
    use_effect(use_reactive((&active_workspace,), move |_| {
        space_down.set(false);
        selection::cancel_scoped_drag(&runtime, &drag, &svg, Some(&render_scope));
        let mut interaction_version = interaction_version;
        interaction_version += 1;
    }));
}

pub(super) struct MatrixPointerDownInput {
    pub runtime: Rc<Runtime>,
    pub adapter: SelectionAdapter,
    pub svg: Rc<RefCell<Option<SvgElement>>>,
    pub space_down: Rc<Cell<bool>>,
    pub scope: Scope,
    pub generation: u64,
    pub range_ids: Rc<Vec<String>>,
    pub selection_kind: Signal<objects::LayoutSelectionKind>,
    pub owner: LayoutOwnerIdentity,
    pub tree_cell_anchor: Rc<RefCell<Option<OwnedTreeCellAnchor>>>,
    pub pending_splay_pick: Signal<Option<objects::MatrixTransformInspectorOwner>>,
    pub pending_anchor_pick: Signal<Option<objects::MatrixTransformInspectorOwner>>,
    pub transform_inspector: objects::MatrixTransformInspectorMount,
    pub canvas_interaction: CanvasInteractionArbiter,
    pub mirrored_pair: objects::MirroredPairMount,
    pub split_focus: Signal<boardstudio_web_ui_model::split_focus::SplitHalf>,
    pub workspace: Signal<&'static str>,
    pub view_x: f64,
    pub view_y: f64,
    pub width: f64,
    pub height: f64,
}

pub(super) fn matrix_pointer_down_handler(
    input: MatrixPointerDownInput,
) -> EventHandler<layout_canvas::MatrixPointerDown> {
    let MatrixPointerDownInput {
        runtime,
        adapter,
        svg,
        space_down,
        scope,
        generation,
        range_ids,
        selection_kind,
        owner,
        tree_cell_anchor,
        mut pending_splay_pick,
        mut pending_anchor_pick,
        transform_inspector,
        canvas_interaction,
        mirrored_pair,
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
        if canvas_interaction.is_owner(CanvasInteractionOwner::MirroredPair)
            && mirrored_pair.placement.is_some()
        {
            return;
        }
        if canvas_interaction.current().is_some() {
            pointer.prevent_default();
            pointer.stop_propagation();
            return;
        }
        if pointer.button() != 0 || space_down.get() {
            return;
        }
        if pending_splay_pick.peek().is_some()
            && let Some(point) = coordinates(&svg, &pointer, view_x, view_y, width, height)
        {
            pointer.prevent_default();
            pointer.stop_propagation();
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
            pointer.prevent_default();
            pointer.stop_propagation();
            let aggregate_armed = transform_inspector.projection.as_ref().is_some_and(
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
        pointer.prevent_default();
        pointer.stop_propagation();
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
        if let Some(accepted) = current.accepted.as_ref()
            && let Some(half) = boardstudio_web_ui_model::split_focus::context_half(
                &accepted.document,
                &hit_context,
            )
            && half != split_focus()
        {
            split_focus.set(half);
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

pub(super) struct PartPointerDownInput {
    pub runtime: Rc<Runtime>,
    pub svg: Rc<RefCell<Option<SvgElement>>>,
    pub drag: Rc<RefCell<Option<Drag>>>,
    pub space_down: Rc<Cell<bool>>,
    pub canvas_interaction: CanvasInteractionArbiter,
    pub navigation: CanvasNavigationState,
    pub adapter: SelectionAdapter,
    pub render_scope: Scope,
    pub generation: u64,
    pub range_ids: Rc<Vec<String>>,
    pub selection_kind: Signal<objects::LayoutSelectionKind>,
    pub owner: LayoutOwnerIdentity,
    pub tree_cell_anchor: Rc<RefCell<Option<OwnedTreeCellAnchor>>>,
    pub mirrored_pair: objects::MirroredPairMount,
    pub pending_splay_pick: Signal<Option<objects::MatrixTransformInspectorOwner>>,
    pub transform_inspector: objects::MatrixTransformInspectorMount,
    pub split_focus: Signal<boardstudio_web_ui_model::split_focus::SplitHalf>,
    pub workspace: Signal<&'static str>,
    pub view_x: f64,
    pub view_y: f64,
    pub width: f64,
    pub height: f64,
}

pub(super) fn part_pointer_down_handler(
    input: PartPointerDownInput,
) -> EventHandler<layout_canvas::PartPointerDown> {
    let PartPointerDownInput {
        runtime,
        svg,
        drag,
        space_down,
        canvas_interaction,
        navigation,
        adapter,
        render_scope,
        generation,
        range_ids,
        selection_kind,
        owner,
        tree_cell_anchor,
        mirrored_pair,
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
        if pointer.button() != 0 {
            return;
        }
        if canvas_interaction.is_owner(CanvasInteractionOwner::MirroredPair)
            && mirrored_pair.placement.is_some()
        {
            return;
        }
        if canvas_interaction.current().is_some() {
            pointer.prevent_default();
            pointer.stop_propagation();
            return;
        }
        if runtime.scope().as_ref() != Some(&render_scope) || (adapter.generation)() != generation {
            return;
        }
        if drag.borrow().is_some() || runtime.model().gesture.is_some() {
            return;
        }
        pointer.prevent_default();
        pointer.stop_propagation();
        if pending_splay_pick.peek().is_some()
            && let Some(point) = coordinates(&svg, &pointer, view_x, view_y, width, height)
        {
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
        if space_down.get() {
            if let Some(svg) = svg.borrow().as_ref() {
                let _ = svg.set_pointer_capture(pointer.pointer_id());
                let options = web_sys::FocusOptions::new();
                options.set_prevent_scroll(true);
                let _ = svg.focus_with_options(&options);
            }
            navigation.start_space_pan(boardstudio_web_ui_shared::canvas_navigation::PanStart {
                pointer_id: i64::from(pointer.pointer_id()),
                scope: render_scope.clone(),
                generation,
                client: (f64::from(pointer.client_x()), f64::from(pointer.client_y())),
                camera: runtime.model().camera.center,
            });
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
        if let Some(accepted) = current.accepted.as_ref()
            && let Some(half) = boardstudio_web_ui_model::split_focus::context_half(
                &accepted.document,
                &hit_context,
            )
            && half != split_focus()
        {
            split_focus.set(half);
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
        if let Some(svg) = svg.borrow().as_ref() {
            let _ = svg.set_pointer_capture(pointer.pointer_id());
            let options = web_sys::FocusOptions::new();
            options.set_prevent_scroll(true);
            let _ = svg.focus_with_options(&options);
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
            active: false,
            pan: false,
            camera: Vec2::default(),
        });
    })
}

pub(super) fn moved(drag: &Drag, point: Vec2) -> Vec<Position> {
    drag.positions
        .iter()
        .map(|p| Position {
            id: p.id.clone(),
            at: Vec2 {
                x: p.at.x + point.x - drag.origin.x,
                y: p.at.y + point.y - drag.origin.y,
            },
        })
        .collect()
}

// GROUPING: allow until the Editor rewrite lands; remove then.
#[allow(dead_code)]
pub(super) fn use_pointer_bundle(
    setup_a: &super::setup_shell::SetupA,
    cam: &super::camera::CamNamed,
    render_scope: &Scope,
) -> (
    impl FnMut(PointerEvent) + Clone + 'static,
    impl FnMut(PointerEvent) + Clone + 'static,
    impl FnMut(PointerEvent) + Clone + 'static,
) {
    let canvas_navigation = &setup_a.canvas_navigation;
    let runtime = &setup_a.runtime;
    let svg = &cam.svg_surface.svg;
    let drag = &cam.placement.drag;
    let adapter = &setup_a.adapter;
    let layout_owner = &setup_a.layout_owner;
    let layout_snap_settings = setup_a.layout_snap_settings;
    let part_placement = &cam.placement.part_placement;
    let matrix_placement = &setup_a.matrix_placement;
    let tree_cell_anchor = &setup_a.tree_cell_anchor;
    let mirrored_pair = &setup_a.mirrored_pair;
    let canvas_interaction = &setup_a.canvas_interaction;
    let workspace = setup_a.workspace;
    let view_x = cam.scene_bounds.view_x;
    let view_y = cam.scene_bounds.view_y;
    let width = cam.scene_bounds.width;
    let height = cam.scene_bounds.height;
    let render_generation = setup_a.render_generation;
    let interaction_version = cam.placement.interaction_version;
    let active_workspace = setup_a.active_workspace;
    let space_down = &cam.placement.space_down;
    let move_pointer = move_pointer_handler(MovePointerInput {
        navigation: canvas_navigation.clone(),
        runtime: runtime.clone(),
        svg: svg.clone(),
        drag: drag.clone(),
        adapter: adapter.clone(),
        render_scope: render_scope.clone(),
        owner: layout_owner.clone(),
        snap_settings: layout_snap_settings,
        placement: part_placement.clone(),
        matrix_placement: matrix_placement.clone(),
        tree_cell_anchor: tree_cell_anchor.clone(),
        mirrored_pair: mirrored_pair.clone(),
        canvas_interaction: canvas_interaction.clone(),
        workspace,
        view_x,
        view_y,
        width,
        height,
        render_generation,
    });
    let end_pointer = end_pointer_handler(EndPointerInput {
        navigation: canvas_navigation.clone(),
        runtime: runtime.clone(),
        svg: svg.clone(),
        drag: drag.clone(),
        adapter: adapter.clone(),
        owner: layout_owner.clone(),
        placement: part_placement.clone(),
        matrix_placement: matrix_placement.clone(),
        snap_settings: layout_snap_settings,
        render_scope: render_scope.clone(),
        canvas_interaction: canvas_interaction.clone(),
        workspace,
        view_x,
        view_y,
        width,
        height,
        render_generation,
        interaction_version,
    });
    let cancel_pointer = cancel_pointer_handler(CancelPointerInput {
        runtime: runtime.clone(),
        svg: svg.clone(),
        drag: drag.clone(),
        placement: part_placement.clone(),
        matrix_placement: matrix_placement.clone(),
        render_scope: render_scope.clone(),
        canvas_interaction: canvas_interaction.clone(),
        render_generation,
        interaction_version,
    });
    use_pointer_workspace_effect(
        active_workspace,
        runtime.clone(),
        drag.clone(),
        svg.clone(),
        space_down.clone(),
        render_scope.clone(),
        interaction_version,
    );
    (move_pointer, end_pointer, cancel_pointer)
}
