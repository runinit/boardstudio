use super::super::*;
use boardstudio_web_ui_model::Workspace;

pub(super) struct MovePointerInput {
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
    pub workspace: Signal<Workspace>,
    pub view_x: f64,
    pub view_y: f64,
    pub width: f64,
    pub height: f64,
    pub render_generation: u64,
}

pub(super) fn move_pointer_handler(
    input: MovePointerInput,
) -> impl FnMut(PointerEvent) + Clone + 'static {
    let canvas_interaction = use_context::<LayoutInteraction>();
    let MovePointerInput {
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
        canvas_interaction.sync_control(pointer.ctrl_key());
        if canvas_interaction.route_pointer(&pointer, false) {
            return;
        }
        if canvas_interaction
            .pointer()
            .is_some_and(|id| id != pointer.pointer_id())
        {
            return;
        }
        let mode = canvas_interaction.current();
        if mode == Some(CanvasMode::PlacingMatrix) {
            let Some(active) = matrix_placement.placement.as_ref() else {
                return;
            };
            let owner = active.owner.clone();
            if workspace() != Workspace::Layout
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
        if mode == Some(CanvasMode::PlacingMirroredPair) {
            let Some(placement) = mirrored_pair.placement.as_ref() else {
                return;
            };
            let pair_owner = placement.owner.clone();
            if workspace() != Workspace::Layout
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
        if mode == Some(CanvasMode::PlacingPart) {
            if let Some(active) = placement.projection.as_ref()
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
                placement.on_move.call(at);
            }
            return;
        }
        let Some(mut current) = live_drag(
            canvas_interaction,
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
        if current.pan {
            return;
        }
        let Some((point, scale)) =
            coordinates_with_scale(&svg, &pointer, view_x, view_y, width, height)
        else {
            return;
        };
        if !current.active {
            if !canvas_interaction
                .handle(CanvasEvent::PointerMove {
                    id: pointer.pointer_id(),
                    client: client_point(&pointer),
                    operation: None,
                })
                .contains(&InteractionEffect::StartPartDrag)
            {
                return;
            }
            let current_model = runtime.model();
            if current_model.gesture.is_some() {
                canvas_interaction.handle(CanvasEvent::PointerCancel(pointer.pointer_id()));
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
                canvas_interaction.handle(CanvasEvent::PointerCancel(pointer.pointer_id()));
                selection::cancel_scoped_drag(&runtime, &drag, &svg, Some(&current.scope));
                return;
            }
            current.active = true;
            current.gesture_generation = gesture.map(|gesture| gesture.generation);
            *drag.borrow_mut() = Some(current.clone());
        }
        let alt = pointer.alt_key();
        if let Some(stored) = drag.borrow_mut().as_mut() {
            stored.last_sample = Some((point, scale, alt));
        }
        runtime.submit(Event::GestureSample {
            pointer_id: current.pointer,
            positions: moved(&current, point),
            pixels_per_world: Some(scale),
            alt,
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
    interaction: LayoutInteraction,
    drag: &Rc<RefCell<Option<Drag>>>,
    svg: &Rc<RefCell<Option<SvgElement>>>,
    runtime: &Rc<Runtime>,
    adapter: &SelectionAdapter,
    owner: &LayoutOwnerIdentity,
    workspace: Signal<Workspace>,
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
        interaction.handle(CanvasEvent::PointerCancel(pointer));
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
    pub workspace: Signal<Workspace>,
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
    let canvas_interaction = use_context::<LayoutInteraction>();
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
        canvas_interaction.sync_control(pointer.ctrl_key());
        if canvas_interaction.route_pointer(&pointer, true) {
            return;
        }
        if canvas_interaction
            .pointer()
            .is_some_and(|id| id != pointer.pointer_id())
        {
            return;
        }
        let mode = canvas_interaction.current();
        if matches!(
            mode,
            Some(
                CanvasMode::PlacingPart
                    | CanvasMode::PlacingMatrix
                    | CanvasMode::PlacingMirroredPair
            )
        ) {
            if canvas_interaction.pointer() != Some(pointer.pointer_id()) {
                return;
            }
            canvas_interaction.handle(CanvasEvent::PointerUp {
                id: pointer.pointer_id(),
                operation: None,
            });
        }
        if mode == Some(CanvasMode::PlacingMatrix) {
            pointer.prevent_default();
            pointer.stop_propagation();
            if pointer.button() == 0
                && let Some(active) = matrix_placement.placement.as_ref()
            {
                let owner = active.owner.clone();
                if workspace() != Workspace::Layout
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
        if mode == Some(CanvasMode::PlacingMirroredPair) {
            pointer.prevent_default();
            pointer.stop_propagation();
            return;
        }
        if mode == Some(CanvasMode::PlacingPart) {
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
            canvas_interaction,
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
        canvas_interaction.handle(CanvasEvent::PointerUp {
            id: pointer.pointer_id(),
            operation: None,
        });
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
            && let Some((point, scale, alt)) =
                coordinates_with_scale(&svg, &pointer, view_x, view_y, width, height)
                    .map(|(point, scale)| (point, scale, pointer.alt_key()))
                    .or(current.last_sample)
        {
            runtime.submit(Event::GestureEnd {
                pointer_id: current.pointer,
                final_positions: moved(&current, point),
                pixels_per_world: Some(scale),
                alt,
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

pub(super) fn cancel_pointer_handler(
    runtime: Rc<Runtime>,
    svg: Rc<RefCell<Option<SvgElement>>>,
    drag: Rc<RefCell<Option<Drag>>>,
    interaction_version: Signal<u64>,
    lost_capture: bool,
) -> impl FnMut(PointerEvent) + Clone + 'static {
    let input = use_context::<layout_input::LayoutInput>();
    move |event: PointerEvent| {
        let Some(pointer) = event.data().try_as_web_event() else {
            return;
        };
        let id = pointer.pointer_id();
        input.dispatch(if lost_capture {
            CanvasEvent::LostPointerCapture(id)
        } else {
            CanvasEvent::PointerCancel(id)
        });
        if drag
            .borrow()
            .as_ref()
            .is_some_and(|drag| drag.pointer == i64::from(id))
        {
            selection::cancel_scoped_drag(&runtime, &drag, &svg, None);
            let mut interaction_version = interaction_version;
            interaction_version += 1;
        }
    }
}

pub(in crate::presentation) fn capture_canvas_drag(
    interaction: LayoutInteraction,
    mode: CanvasMode,
    id: i32,
    client: (f64, f64),
    control: bool,
    runtime: &Rc<Runtime>,
    drag: &Rc<RefCell<Option<Drag>>>,
    svg: &Rc<RefCell<Option<SvgElement>>>,
) -> bool {
    if interaction.handle(CanvasEvent::PointerDown {
        mode,
        id,
        client,
        control,
    }) != [InteractionEffect::CapturePointer(id)]
    {
        return false;
    }
    if let Some(surface) = svg.borrow().as_ref()
        && surface.set_pointer_capture(id).is_ok()
    {
        focus_svg(surface);
    } else {
        interaction.end(mode);
        return false;
    }
    let (runtime, drag, svg) = (runtime.clone(), drag.clone(), svg.clone());
    interaction.on_cancel(move || selection::cancel_scoped_drag(&runtime, &drag, &svg, None));
    true
}

pub(in crate::presentation) fn start_pan(
    runtime: &Rc<Runtime>,
    navigation: &CanvasNavigationState,
    svg: &Rc<RefCell<Option<SvgElement>>>,
    scope: &Scope,
    generation: u64,
    pointer: &web_sys::PointerEvent,
    interaction: LayoutInteraction,
) {
    let client = client_point(pointer);
    if capture_canvas_drag(
        interaction,
        CanvasMode::Panning,
        pointer.pointer_id(),
        client,
        pointer.ctrl_key(),
        runtime,
        &navigation.drag_interaction(),
        svg,
    ) {
        navigation.start_space_pan(boardstudio_web_ui_shared::canvas_navigation::PanStart {
            pointer_id: i64::from(pointer.pointer_id()),
            scope: scope.clone(),
            generation,
            client,
            camera: runtime.with_model(|model| model.camera.center),
        });
    }
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
