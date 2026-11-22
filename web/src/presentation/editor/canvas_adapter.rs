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
                let at = snap_placement(
                    &runtime,
                    &adapter,
                    &active,
                    &snap_settings.read(),
                    point,
                    pointer.alt_key(),
                );
                placement.on_move.call(at);
            } else {
                pointer.prevent_default();
                pointer.stop_propagation();
            }
            return;
        }
        let Some(mut current) = live_drag(
            &drag,
            &svg,
            &runtime,
            &adapter,
            &owner,
            workspace,
            &render_scope,
            render_generation,
            pointer.pointer_id(),
        ) else {
            return;
        };
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
            let current_model = runtime.model();
            if current_model.gesture.is_some() {
                selection::cancel_scoped_drag(&runtime, &drag, &svg, Some(&current.scope));
                return;
            }
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
                selection::cancel_scoped_drag(&runtime, &drag, &svg, Some(&current.scope));
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

fn snap_placement(
    runtime: &Runtime,
    adapter: &SelectionAdapter,
    active: &part_placement::ActivePartPlacement,
    settings: &objects::LayoutSnapSettings,
    point: Vec2,
    free: bool,
) -> Vec2 {
    let model = runtime.model();
    if model.accepted.is_none() {
        return point;
    }
    part_placement::snap_placement_at(
        &active.snap_document,
        &active.owner.board_id,
        &active.pending,
        point,
        part_placement::PlacementSnapOptions {
            snap_fraction: settings.snap_fraction,
            geometry_snap: settings.geometry_snap,
            gap: part_placement::placement_gap(settings),
            free,
            pitch: matrix_snap_parameters(&model, &active.owner.scope, adapter, None).0,
        },
    )
}

/// Resolve the captured pointer against the render that owns it before touching the edit session.
#[allow(clippy::too_many_arguments)]
fn live_drag(
    drag: &Rc<RefCell<Option<Drag>>>,
    svg: &Rc<RefCell<Option<SvgElement>>>,
    runtime: &Rc<Runtime>,
    adapter: &SelectionAdapter,
    owner: &LayoutOwnerIdentity,
    workspace: Signal<&'static str>,
    render_scope: &Scope,
    generation: u64,
    pointer: i32,
) -> Option<Drag> {
    let current = drag.borrow().clone().filter(|current| {
        current.pointer == i64::from(pointer)
            && &current.scope == render_scope
            && current.generation == generation
    })?;
    if !canvas_owner_is_current(runtime, workspace, adapter, owner)
        || runtime.scope().as_ref() != Some(&current.scope)
        || (adapter.generation)() != current.generation
        || (current.active && !current.pan && !selection::owns_session_gesture(runtime, &current))
    {
        selection::cancel_scoped_drag(runtime, drag, svg, Some(render_scope));
        return None;
    }
    Some(current)
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
            if part_placement::pointer_release_commits(pointer.button())
                && let Some(active) = placement.projection.as_ref()
                && let Some(point) = coordinates(&svg, &pointer, view_x, view_y, width, height)
            {
                let at = snap_placement(
                    &runtime,
                    &adapter,
                    active,
                    &snap_settings.read(),
                    point,
                    pointer.alt_key(),
                );
                placement.on_commit.call(at);
            }
            return;
        }
        let Some(current) = live_drag(
            &drag,
            &svg,
            &runtime,
            &adapter,
            &owner,
            workspace,
            &render_scope,
            render_generation,
            pointer.pointer_id(),
        ) else {
            return;
        };
        let mut interaction_version = interaction_version;
        interaction_version += 1;
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
