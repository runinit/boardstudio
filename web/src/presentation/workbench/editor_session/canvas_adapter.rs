use super::super::*;
use crate::presentation::layout_canvas::{MatrixPointerDown, PartPointerDown};
use crate::presentation::layout_workspace::LayoutWorkspaceState;
use crate::presentation::owner_checks::canvas_owner_is_current;
use crate::presentation::pending_picks::{
    PendingAnchorPick, PendingSplayOriginPick, apply_pending_anchor_pick,
    apply_pending_splay_origin_pick,
};
use crate::presentation::tree_cell_anchor::{
    OwnedTreeCellAnchor, tree_cell_anchor_for_owner, update_tree_cell_anchor,
};
use crate::presentation::view_helpers::matrix_snap_parameters;
use crate::presentation::workspace_composition::{WorkspaceFrame, WorkspaceInteraction};
use boardstudio_web_layout::canvas_interaction::{ActivePan, ActivePartDrag};
use boardstudio_web_ui_model::Workspace;

pub(super) struct MovePointerInput {
    pub runtime: Rc<Runtime>,
    pub svg: Rc<RefCell<Option<SvgElement>>>,
    pub adapter: SelectionAdapter,
    pub render_scope: Scope,
    pub owner: LayoutOwnerIdentity,
    pub snap_settings: Signal<objects::LayoutSnapSettings>,
    pub placement: part_placement::PartPlacementSession,
    pub matrix_placement: objects::MatrixPlacementSession,
    pub tree_cell_anchor: Rc<RefCell<Option<OwnedTreeCellAnchor>>>,
    pub mirrored_pair: objects::MirroredPairSession,
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
        let Some(LivePointerGesture::Parts(current)) = live_pointer_gesture(
            canvas_interaction,
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
                pointer_id: i64::from(current.pointer_id),
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
                gesture.pointer_id == i64::from(current.pointer_id)
                    && gesture.target_ids == target_ids
            });
            if gesture.as_ref().is_none_or(|_| {
                runtime.scope().as_ref() != Some(&current.scope)
                    || (adapter.generation)() != current.generation
            }) {
                canvas_interaction.handle(CanvasEvent::PointerCancel(pointer.pointer_id()));
                return;
            }
            canvas_interaction
                .set_part_drag_gesture_generation(gesture.map(|gesture| gesture.generation));
        }
        let alt = pointer.alt_key();
        canvas_interaction.note_part_drag_sample(Some((point, scale, alt)));
        runtime.submit(Event::GestureSample {
            pointer_id: i64::from(current.pointer_id),
            positions: moved_positions(&current.positions, current.origin_mm, point),
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

enum LivePointerGesture {
    Pan(ActivePan),
    Parts(ActivePartDrag),
}

#[allow(clippy::too_many_arguments)]
fn live_pointer_gesture(
    interaction: LayoutInteraction,
    runtime: &Rc<Runtime>,
    adapter: &SelectionAdapter,
    owner: &LayoutOwnerIdentity,
    workspace: Signal<Workspace>,
    render_scope: &Scope,
    generation: u64,
    pointer: i32,
) -> Option<LivePointerGesture> {
    let live = canvas_owner_is_current(runtime, workspace, adapter, owner)
        && runtime
            .scope()
            .as_ref()
            .is_some_and(|scope| scope == render_scope)
        && (adapter.generation)() == generation;
    if let Some(pan) = interaction.pan().filter(|pan| {
        pan.pointer_id == pointer && pan.scope == *render_scope && pan.generation == generation
    }) {
        if !live {
            interaction.handle(CanvasEvent::PointerCancel(pointer));
            return None;
        }
        return Some(LivePointerGesture::Pan(pan));
    }
    let current = interaction.part_drag().filter(|current| {
        current.pointer_id == pointer
            && current.scope == *render_scope
            && current.generation == generation
    })?;
    if !live
        || (current.active
            && !selection::owns_session_gesture(
                runtime,
                i64::from(current.pointer_id),
                &current.scope,
                current.gesture_generation,
            ))
    {
        interaction.handle(CanvasEvent::PointerCancel(pointer));
        return None;
    }
    Some(LivePointerGesture::Parts(current))
}

pub(super) struct EndPointerInput {
    pub navigation: CanvasNavigationState,
    pub runtime: Rc<Runtime>,
    pub svg: Rc<RefCell<Option<SvgElement>>>,
    pub adapter: SelectionAdapter,
    pub owner: LayoutOwnerIdentity,
    pub placement: part_placement::PartPlacementSession,
    pub matrix_placement: objects::MatrixPlacementSession,
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
        let current = match live_pointer_gesture(
            canvas_interaction,
            &runtime,
            &adapter,
            &owner,
            workspace,
            &render_scope,
            render_generation,
            pointer.pointer_id(),
        ) {
            None => return,
            Some(LivePointerGesture::Pan(pan)) => {
                canvas_interaction.handle(CanvasEvent::PointerUp {
                    id: pointer.pointer_id(),
                    operation: None,
                });
                let mut interaction_version = interaction_version;
                interaction_version += 1;
                navigation.finish_pan(
                    &runtime,
                    &svg,
                    pan.start_camera,
                    pan.origin_px,
                    (f64::from(pointer.client_x()), f64::from(pointer.client_y())),
                    (width, height),
                    pointer.pointer_id(),
                );
                return;
            }
            Some(LivePointerGesture::Parts(current)) => current,
        };
        canvas_interaction.handle(CanvasEvent::PointerUp {
            id: pointer.pointer_id(),
            operation: None,
        });
        let mut interaction_version = interaction_version;
        interaction_version += 1;
        if current.active
            && let Some((point, scale, alt)) =
                coordinates_with_scale(&svg, &pointer, view_x, view_y, width, height)
                    .map(|(point, scale)| (point, scale, pointer.alt_key()))
                    .or(current.last_sample)
        {
            runtime.submit(Event::GestureEnd {
                pointer_id: i64::from(current.pointer_id),
                final_positions: moved_positions(&current.positions, current.origin_mm, point),
                pixels_per_world: Some(scale),
                alt,
            });
        } else if current.active {
            selection::cancel_drag_if_owned(
                &runtime,
                i64::from(current.pointer_id),
                &current.scope,
                current.gesture_generation,
            );
        }
        if let Some(element) = svg.borrow().as_ref() {
            let _ = element.release_pointer_capture(pointer.pointer_id());
        }
    }
}

pub(super) fn cancel_pointer_handler(
    interaction_version: Signal<u64>,
    lost_capture: bool,
) -> impl FnMut(PointerEvent) + Clone + 'static {
    let input = use_context::<layout_input::LayoutInput>();
    move |event: PointerEvent| {
        let Some(pointer) = event.data().try_as_web_event() else {
            return;
        };
        let id = pointer.pointer_id();
        let effects = input.dispatch(if lost_capture {
            CanvasEvent::LostPointerCapture(id)
        } else {
            CanvasEvent::PointerCancel(id)
        });
        if effects
            .iter()
            .any(|effect| matches!(effect, InteractionEffect::Cancel(_)))
        {
            let mut interaction_version = interaction_version;
            interaction_version += 1;
        }
    }
}

pub(in crate::presentation) struct CanvasDragStart {
    pub id: i32,
    pub client: (f64, f64),
    pub control: bool,
    pub seed: GestureSeed,
}

pub(in crate::presentation) fn capture_pan(
    interaction: LayoutInteraction,
    runtime: &Rc<Runtime>,
    svg: &Rc<RefCell<Option<SvgElement>>>,
    scope: &Scope,
    generation: u64,
    pointer: &web_sys::PointerEvent,
) -> bool {
    capture_canvas_drag(
        interaction,
        CanvasDragStart {
            id: pointer.pointer_id(),
            client: client_point(pointer),
            control: pointer.ctrl_key(),
            seed: GestureSeed::Pan {
                scope: scope.clone(),
                generation,
                start_camera: runtime.with_model(|model| model.camera.center),
            },
        },
        runtime,
        svg,
    )
}

#[allow(clippy::too_many_arguments)]
fn resolve_canvas_hit(
    runtime: &Rc<Runtime>,
    adapter: &SelectionAdapter,
    render_scope: &Scope,
    generation: u64,
    owner: &LayoutOwnerIdentity,
    workspace: Signal<Workspace>,
    tree_cell_anchor: &Rc<RefCell<Option<OwnedTreeCellAnchor>>>,
    selection_kind: Signal<objects::LayoutSelectionKind>,
    split_focus: Signal<boardstudio_web_ui_model::split_focus::SplitHalf>,
    pointer: &web_sys::PointerEvent,
    hit_context: &objects::TreeContext,
    current: &boardstudio_application::ReadModel,
) -> Option<(objects::TreeContext, Vec<String>, SelectionMode)> {
    if !selection::layout_owner_is_current(runtime, workspace, adapter, owner)
        || runtime.scope().as_ref() != Some(render_scope)
        || (adapter.generation)() != generation
    {
        return None;
    }
    if let Some(accepted) = current.accepted.as_ref() {
        let resolved = boardstudio_application::resolved_document(&accepted.document);
        if let Some(half) =
            boardstudio_web_ui_model::split_focus::context_half(resolved.as_ref(), hit_context)
        {
            let mut split_focus = split_focus;
            if half != split_focus() {
                split_focus.set(half);
            }
        }
    }
    update_tree_cell_anchor(tree_cell_anchor, owner, current, Some(hit_context));
    let retained = tree_cell_anchor_for_owner(tree_cell_anchor, owner);
    let projection = objects::context_for_selection_kind(
        current,
        hit_context,
        selection_kind(),
        retained.as_ref(),
    );
    let target_ids = projection
        .as_ref()
        .map(|projection| projection.part_ids.clone())
        .unwrap_or_else(|| selection::resolve_context(current, hit_context).unwrap_or_default());
    let context = projection
        .map(|projection| projection.context)
        .unwrap_or_else(|| hit_context.clone());
    let mode = if pointer.shift_key() {
        SelectionMode::Range
    } else if pointer.ctrl_key() || pointer.meta_key() {
        SelectionMode::Toggle
    } else {
        SelectionMode::Replace
    };
    Some((context, target_ids, mode))
}

pub(in crate::presentation) fn capture_canvas_drag(
    interaction: LayoutInteraction,
    start: CanvasDragStart,
    runtime: &Rc<Runtime>,
    svg: &Rc<RefCell<Option<SvgElement>>>,
) -> bool {
    let CanvasDragStart {
        id,
        client,
        control,
        seed,
    } = start;
    // Some callers stop propagation before the workbench can sync modifiers.
    interaction.sync_control(control);
    let mode = seed.mode();
    let seed_scope = match &seed {
        GestureSeed::Pan { scope, .. } | GestureSeed::PartDrag { scope, .. } => Some(scope.clone()),
        GestureSeed::MatrixRail(_) | GestureSeed::PartRail(_) | GestureSeed::OutlineHandle(_) => {
            None
        }
    };
    if interaction.handle(CanvasEvent::PointerDown {
        id,
        client,
        control,
        seed: Some(seed),
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
    let (runtime, svg) = (runtime.clone(), svg.clone());
    interaction.on_cancel(move || {
        if let Some(drag) = interaction.take_canceled_drag() {
            selection::cancel_scoped_drag(&runtime, &svg, seed_scope.as_ref(), &drag);
        }
    });
    true
}

pub(super) fn moved_positions(
    positions: &[Position],
    origin_mm: Vec2,
    point: Vec2,
) -> Vec<Position> {
    positions
        .iter()
        .map(|position| Position {
            id: position.id.clone(),
            at: Vec2 {
                x: position.at.x + point.x - origin_mm.x,
                y: position.at.y + point.y - origin_mm.y,
            },
        })
        .collect()
}

pub(super) struct PointerBundle {
    pub move_pointer: EventHandler<PointerEvent>,
    pub end_pointer: EventHandler<PointerEvent>,
    pub cancel_pointer: EventHandler<PointerEvent>,
    pub lost_capture: EventHandler<PointerEvent>,
}

pub(super) fn use_pointer_bundle(
    state: &super::session_state::SessionState,
    cam: &super::camera::CamNamed,
    render_scope: &Scope,
) -> PointerBundle {
    let editor = use_context::<boardstudio_web_ui_model::editor::EditorContext>();
    let canvas_navigation = &state.canvas_navigation;
    let runtime = &editor.runtime();
    let svg = &cam.svg_surface.svg;
    let adapter = &state.adapter;
    let layout_owner = &state.view.layout_owner;
    let layout_snap_settings = state.layout_state.snap_settings();
    let part_placement = &cam.placement.part_placement;
    let matrix_placement = &state.matrix_pair.matrix_placement;
    let tree_cell_anchor = &state.tree_cell_anchor;
    let mirrored_pair = &state.matrix_pair.mirrored_pair;
    let workspace = editor.workspace;
    let view_x = cam.scene_bounds.view_x;
    let view_y = cam.scene_bounds.view_y;
    let width = cam.scene_bounds.width;
    let height = cam.scene_bounds.height;
    let render_generation = state.render_generation;
    let interaction_version = cam.placement.interaction_version;
    let canvas_interaction = use_context::<LayoutInteraction>();
    let move_pointer = move_pointer_handler(MovePointerInput {
        runtime: runtime.clone(),
        svg: svg.clone(),
        adapter: adapter.clone(),
        render_scope: render_scope.clone(),
        owner: layout_owner.clone(),
        snap_settings: layout_snap_settings,
        placement: part_placement.clone(),
        matrix_placement: matrix_placement.clone(),
        tree_cell_anchor: tree_cell_anchor.clone(),
        mirrored_pair: mirrored_pair.clone(),
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
        adapter: adapter.clone(),
        owner: layout_owner.clone(),
        placement: part_placement.clone(),
        matrix_placement: matrix_placement.clone(),
        snap_settings: layout_snap_settings,
        render_scope: render_scope.clone(),
        workspace,
        view_x,
        view_y,
        width,
        height,
        render_generation,
        interaction_version,
    });
    let active_workspace = state.active_workspace;
    let previous_workspace = use_hook(move || Rc::new(Cell::new(active_workspace)));
    let render_scope_for_cleanup = render_scope.clone();
    use_effect(use_reactive((&active_workspace,), move |(current,)| {
        if previous_workspace.replace(current) == Workspace::Pcb && current != Workspace::Pcb {
            canvas_interaction.cancel_scoped(&render_scope_for_cleanup);
        }
    }));
    let cancel_pointer = |lost| cancel_pointer_handler(interaction_version, lost);
    PointerBundle {
        move_pointer: EventHandler::new(move_pointer),
        end_pointer: EventHandler::new(end_pointer),
        cancel_pointer: EventHandler::new(cancel_pointer(false)),
        lost_capture: EventHandler::new(cancel_pointer(true)),
    }
}

pub(in crate::presentation) struct LayoutPointer {
    pub matrix_pointer_down: EventHandler<MatrixPointerDown>,
    pub part_pointer_down: EventHandler<PartPointerDown>,
}

pub(in crate::presentation) fn build_canvas_pointer(frame: &WorkspaceFrame) -> LayoutPointer {
    let editor = use_context::<boardstudio_web_ui_model::editor::EditorContext>();
    let adapter = use_context::<SelectionAdapter>();
    let interaction = use_context::<WorkspaceInteraction>();
    let layout_state = use_context::<LayoutWorkspaceState>();
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
    let part_pointer_down = EventHandler::new(move |request: PartPointerDown| {
        if interaction.current() == Some(CanvasMode::PickingMatrixAnchor) {
            if let Some(objects::TreeContext::Key {
                matrix_id,
                row,
                column,
            }) = runtime.with_model(|model| objects::context_for_part(model, &request.part_id))
            {
                matrix_pointer_down.call(MatrixPointerDown {
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
) -> EventHandler<MatrixPointerDown> {
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
        split_focus,
        workspace,
        view_x,
        view_y,
        width,
        height,
    } = input;
    EventHandler::new(move |request: MatrixPointerDown| {
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
        let Some(hit_context) =
            objects::context_for_cell(&current, &request.matrix_id, request.row, request.column)
        else {
            return;
        };
        let Some((context, _, mode)) = resolve_canvas_hit(
            &runtime,
            &adapter,
            &scope,
            generation,
            &owner,
            workspace,
            &tree_cell_anchor,
            selection_kind,
            split_focus,
            &pointer,
            &hit_context,
            &current,
        ) else {
            return;
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
) -> EventHandler<PartPointerDown> {
    let canvas_interaction = use_context::<LayoutInteraction>();
    let PartPointerDownInput {
        runtime,
        svg,
        adapter,
        render_scope,
        generation,
        range_ids,
        selection_kind,
        owner,
        tree_cell_anchor,
        mut pending_splay_pick,
        transform_inspector,
        split_focus,
        workspace,
        view_x,
        view_y,
        width,
        height,
    } = input;
    let mut selected_context = adapter.selected_context;
    EventHandler::new(move |request: PartPointerDown| {
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
        if runtime.model().gesture.is_some() {
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
            capture_pan(
                canvas_interaction,
                &runtime,
                &svg,
                &render_scope,
                generation,
                &pointer,
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
        let current = runtime.model();
        let Some(hit_context) = objects::context_for_part(&current, &id) else {
            return;
        };
        let Some((context, target_ids, mode)) = resolve_canvas_hit(
            &runtime,
            &adapter,
            &render_scope,
            generation,
            &owner,
            workspace,
            &tree_cell_anchor,
            selection_kind,
            split_focus,
            &pointer,
            &hit_context,
            &current,
        ) else {
            return;
        };
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
        capture_canvas_drag(
            canvas_interaction,
            CanvasDragStart {
                id: pointer.pointer_id(),
                client: client_point(&pointer),
                control: pointer.ctrl_key(),
                seed: GestureSeed::PartDrag {
                    scope: render_scope.clone(),
                    generation,
                    origin_mm: origin,
                    positions,
                },
            },
            &runtime,
            &svg,
        );
    })
}

fn prevent_and_focus(pointer: &web_sys::PointerEvent, svg: &Rc<RefCell<Option<SvgElement>>>) {
    pointer.prevent_default();
    pointer.stop_propagation();
    if let Some(surface) = svg.borrow().as_ref() {
        focus_svg(surface);
    }
}

pub(super) struct StartPanInput {
    pub render_scope: Scope,
    pub render_generation: u64,
    pub runtime: Rc<Runtime>,
    pub svg: Rc<RefCell<Option<SvgElement>>>,
    pub adapter: SelectionAdapter,
    pub mirrored_pair: objects::MirroredPairSession,
    pub pending_splay_pick: Signal<Option<objects::MatrixTransformInspectorOwner>>,
    pub transform_inspector: objects::MatrixTransformInspectorMount,
    pub layout_owner_for_pick: LayoutOwnerIdentity,
    pub workspace: Signal<Workspace>,
    pub view_x: f64,
    pub view_y: f64,
    pub width: f64,
    pub height: f64,
}

pub(super) fn start_pan_handler(
    input: StartPanInput,
) -> impl FnMut(PointerEvent) + Clone + 'static {
    let canvas_interaction = use_context::<LayoutInteraction>();
    let StartPanInput {
        render_scope,
        render_generation,
        runtime,
        svg,
        adapter,
        mirrored_pair,
        mut pending_splay_pick,
        transform_inspector,
        layout_owner_for_pick,
        workspace,
        view_x,
        view_y,
        width,
        height,
    } = input;
    move |event: PointerEvent| {
        let Some(pointer) = event.data().try_as_web_event() else {
            return;
        };
        if pointer.default_prevented() || pointer.button() != 0 {
            return;
        }
        canvas_interaction.sync_control(pointer.ctrl_key());
        let active_workspace = workspace();
        if active_workspace != Workspace::Layout {
            if matches!(
                active_workspace,
                Workspace::Pcb | Workspace::Keymap | Workspace::Keycaps
            ) && canvas_interaction.space_down()
                && canvas_interaction.current().is_none()
                && runtime.scope().as_ref() == Some(&render_scope)
                && (adapter.generation)() == render_generation
                && runtime.with_model(|model| model.gesture.is_none())
                && capture_pan(
                    canvas_interaction,
                    &runtime,
                    &svg,
                    &render_scope,
                    render_generation,
                    &pointer,
                )
            {
                pointer.prevent_default();
            }
            return;
        }
        let id = pointer.pointer_id();
        if canvas_interaction
            .pointer()
            .is_some_and(|owner| owner != id)
        {
            return;
        }
        match canvas_interaction.current() {
            Some(CanvasMode::PlacingMirroredPair) => {
                canvas_interaction.handle(CanvasEvent::PointerDown {
                    id,
                    client: client_point(&pointer),
                    control: pointer.ctrl_key(),
                    seed: None,
                });
                if let Some(placement) = mirrored_pair.placement.as_ref() {
                    let owner = placement.owner.clone();
                    if runtime.scope().as_ref() != Some(&owner.scope)
                        || (adapter.generation)() != owner.scope_generation
                    {
                        mirrored_pair.on_cancel.call(owner);
                    } else if let Some(center) =
                        coordinates(&svg, &pointer, view_x, view_y, width, height)
                    {
                        mirrored_pair
                            .on_commit
                            .call(objects::MirroredPairMove { owner, center });
                    }
                }
            }
            Some(CanvasMode::PickingSplayOrigin) => {
                if let Some(point) = coordinates(&svg, &pointer, view_x, view_y, width, height) {
                    canvas_interaction.end(CanvasMode::PickingSplayOrigin);
                    apply_pending_splay_origin_pick(
                        &runtime,
                        workspace,
                        &adapter,
                        &layout_owner_for_pick,
                        PendingSplayOriginPick {
                            pending: &mut pending_splay_pick,
                            inspector: &transform_inspector,
                            point,
                            swing: pointer.ctrl_key(),
                        },
                    );
                }
            }
            Some(_) => {
                canvas_interaction.handle(CanvasEvent::PointerDown {
                    id: pointer.pointer_id(),
                    client: client_point(&pointer),
                    control: pointer.ctrl_key(),
                    seed: None,
                });
            }
            None => {
                if !selection::layout_owner_is_current(
                    &runtime,
                    workspace,
                    &adapter,
                    &layout_owner_for_pick,
                ) || runtime.with_model(|model| model.gesture.is_some())
                {
                    return;
                }
                if !canvas_interaction.space_down() {
                    return;
                }
                if capture_pan(
                    canvas_interaction,
                    &runtime,
                    &svg,
                    &render_scope,
                    render_generation,
                    &pointer,
                ) {
                    pointer.prevent_default();
                }
            }
        }
        pointer.prevent_default();
    }
}

pub(super) struct WheelInput {
    pub runtime: Rc<Runtime>,
    pub svg: Rc<RefCell<Option<SvgElement>>>,
    pub scope: Scope,
    pub adapter: SelectionAdapter,
    pub render_generation: u64,
    pub view_x: f64,
    pub view_y: f64,
    pub width: f64,
    pub height: f64,
    pub keymap_scale_ratio: f64,
    pub min_x: f64,
    pub max_x: f64,
    pub min_y: f64,
    pub max_y: f64,
}

pub(super) fn wheel_handler(input: WheelInput) -> impl FnMut(WheelEvent) + Clone + 'static {
    let WheelInput {
        runtime,
        svg,
        scope,
        adapter,
        render_generation,
        view_x,
        view_y,
        width,
        height,
        keymap_scale_ratio,
        min_x,
        max_x,
        min_y,
        max_y,
    } = input;
    // Wheel bursts (trackpads emit dozens of ticks per gesture) coalesce
    // into one zoom application per animation frame: ticks only add to
    // this burst, and the single rAF flush submits once. Without this,
    // every tick ran Runtime::submit -> changed()/notify() plus a DOM
    // rect read, re-rendering the full scene per tick (Issue 2).
    let pending = Rc::new(RefCell::new(layout_camera::WheelZoomCoalescer::new()));
    move |event: WheelEvent| {
        if runtime.scope().as_ref() != Some(&scope) || (adapter.generation)() != render_generation {
            return;
        }
        let Some(wheel) = event.data().try_as_web_event() else {
            return;
        };
        wheel.prevent_default();
        let surface = svg.borrow();
        let Some(element) = surface.as_ref() else {
            return;
        };
        let rect = element.get_bounding_client_rect();
        if rect.width() <= 0.0 || rect.height() <= 0.0 {
            return;
        }
        let Some(location) = pointer_location(
            &rect,
            wheel.client_x(),
            wheel.client_y(),
            view_x,
            view_y,
            width,
            height,
        ) else {
            return;
        };
        let Some(window) = web_sys::window() else {
            return;
        };
        let schedule_flush = pending.borrow_mut().push(
            wheel.delta_y(),
            layout_camera::WheelAnchor {
                world: location.world,
                x_fraction: location.x_fraction,
                y_fraction: location.y_fraction,
            },
        );
        if !schedule_flush {
            // A flush is already queued for this frame; this tick rides it.
            return;
        }
        let flush_pending = pending.clone();
        let flush_runtime = runtime.clone();
        let flush_scope = scope.clone();
        let flush_generation = adapter.generation;
        // keymap_scale_ratio is 1.0 outside Keymap, so Layout and Keymap
        // share the reference 0.25..4 limits.
        let flush = move || {
            let start_zoom = flush_runtime.model().camera.zoom;
            // Replay the burst tick-by-tick through the clamp: a burst
            // that hits a limit mid-burst then backs off ends exactly
            // where sequential application ends.
            let final_zoom = flush_pending
                .borrow()
                .coalesced_zoom(start_zoom, keymap_scale_ratio);
            let Some(batch) = flush_pending.borrow_mut().take_batch() else {
                return;
            };
            let Some(final_zoom) = final_zoom else {
                return;
            };
            if final_zoom == start_zoom {
                return;
            }
            if flush_runtime.scope().as_ref() != Some(&flush_scope)
                || (flush_generation)() != render_generation
            {
                return;
            }
            CanvasNavigationState::apply_zoom_target(
                &flush_runtime,
                boardstudio_web_ui_shared::canvas_navigation::ZoomTarget {
                    center: CanvasNavigationState::zoom_center_at(
                        (min_x, max_x, min_y, max_y),
                        PointerLocation {
                            world: batch.anchor.world,
                            x_fraction: batch.anchor.x_fraction,
                            y_fraction: batch.anchor.y_fraction,
                        },
                        final_zoom,
                    ),
                    zoom: final_zoom,
                },
            );
        };
        let callback = Closure::once_into_js(flush);
        let _ = window.request_animation_frame(callback.unchecked_ref());
    }
}
