//! Ticket 07: the part rail overlay for one independent part tool.
use super::*;

/// Ticket 07: the rail token for one independent part tool. Move and Rotate
/// edit the part's local Offset and Rotation; Spread and Stagger edit the
/// values carried from its anchor key. A detached part moves through the
/// shipped part-drag gesture; its Rotate sweep previews the popup and commits
/// one `SetPartRotation`.
#[component]
#[allow(clippy::too_many_arguments)]
pub fn LayoutPartToolOverlay(
    runtime: LayoutTransformRuntime,
    svg: LayoutTransformSvg,
    arbiter: CanvasInteractionArbiter,
    owner: LayoutOwnerIdentity,
    selected_context: Signal<Option<ScopedTreeContext>>,
    scope_generation: Signal<u64>,
    workspace: Signal<&'static str>,
    #[props(default)] surface_size: Option<Signal<(f64, f64)>>,
    part_id: String,
    tool: LayoutTransformTool,
    snap: GestureSnapInputs,
    view_x: f64,
    view_y: f64,
    width: f64,
    height: f64,
    on_finish: EventHandler<()>,
    on_focus_half: EventHandler<()>,
) -> Element {
    let _ = surface_size.map(|size| size());
    // W3-C: canvas-owned rail sets; siblings share the handle, a second canvas owns its own.
    let svg_handle = svg.clone();
    use_rail_mount(Rail::Part, svg_handle.clone(), tool);
    let drag = use_hook(|| Rc::new(RefCell::new(None::<PartDrag>)));
    let hovered = use_signal(|| false);
    let focused = use_signal(|| false);
    // Ticket 06, reused: the value popup and commit toast content (pure
    // reducer; rendering only), the popup's top-left in world units (`None`
    // docks it above the rail), and the toast generation the 1.8 s timer
    // checks.
    let feedback = use_signal(FeedbackState::default);
    let popup_at = use_signal(|| None::<(f64, f64)>);
    let toast_gen = use_signal(|| 0_u64);
    // Parent `version` via context drives settlement (see matrix overlay).
    let parent_version = try_use_context::<Signal<u64>>();
    let local_version = use_signal(|| 0_u64);
    let version = parent_version.unwrap_or(local_version);
    // Single-ticket part commits settle on `version` (durable toast on
    // landing, `report` on failure, silent on retirement). No keyed map.
    let mut pending = use_signal(Vec::<PendingTransformCommit>::new);
    use_effect(use_reactive((&version(),), {
        let runtime = runtime.0.clone();
        let mut pending = pending;
        let mut feedback = feedback;
        let mut toast_gen = toast_gen;
        move |_| {
            settle_transform_commits(
                &runtime,
                workspace,
                scope_generation,
                selected_context,
                &mut pending,
                &mut feedback,
                &mut toast_gen,
            );
        }
    }));
    let runtime_inner = runtime.0.clone();
    let drag_for_drop = drag.clone();
    let arbiter_for_drop = arbiter.clone();
    let svg_for_drop_cancel = svg_handle.clone();
    use_drop(move || {
        cancel_part_drag(
            &runtime_inner,
            &drag_for_drop,
            &arbiter_for_drop,
            &svg_for_drop_cancel,
        );
    });
    // Escape, on the document or on the focused token, clears the drag and its popup.
    let svg_for_escape_drag = svg_handle.clone();
    let escape_drag = {
        let runtime = runtime.0.clone();
        let drag = drag.clone();
        let arbiter = arbiter.clone();
        let svg_for_escape_drag = svg_for_escape_drag.clone();
        Rc::new(move || {
            let (mut feedback, mut popup_at) = (feedback, popup_at);
            feedback.set(advance_feedback(&feedback(), FeedbackEvent::Dismiss));
            popup_at.set(None);
            cancel_part_drag(&runtime, &drag, &arbiter, &svg_for_escape_drag);
            on_finish.call(());
        })
    };
    use_document_escape({
        let drag = drag.clone();
        let arbiter = arbiter.clone();
        let escape_drag = escape_drag.clone();
        move |event| {
            if escape_targets_text_entry(event)
                || (drag.borrow().is_none() && arbiter.current().is_some())
            {
                return;
            }
            event.prevent_default();
            escape_drag();
        }
    });
    let part_context = TreeContext::Component {
        part_id: Some(part_id.clone()),
        matrix_id: None,
        row: None,
        column: None,
        assembly_id: None,
    };
    use_stale_drag_cancel(
        &drag,
        &owner,
        &part_context,
        tool,
        selected_context,
        scope_generation,
        {
            let runtime = runtime.0.clone();
            let drag = drag.clone();
            let arbiter = arbiter.clone();
            let svg_for_stale = svg_handle.clone();
            move || {
                cancel_part_drag(&runtime, &drag, &arbiter, &svg_for_stale);
            }
        },
    );

    // The live part behind this token: a missing, locked, member or
    // companion part hides its token instead of editing the wrong field.
    // A detached part keeps its stored pose; the anchor snapshot stays empty
    // and only Move and Rotate reach the drag.
    let live = {
        let model = runtime.0.model();
        model.accepted.as_ref().and_then(|snapshot| {
            let document = &snapshot.document;
            let part = document.parts.iter().find(|part| part.id == part_id)?;
            if part.locked == Some(true) {
                return None;
            }
            if document
                .matrices
                .iter()
                .any(|matrix| matrix.part_ids.contains(&part.id))
            {
                return None;
            }
            let anchor = part.anchor.clone();
            part_drag_kind(tool, anchor.is_some())?;
            let evaluated = evaluated_part_pose(document, part);
            let courtyard_top = part_courtyard_top(document, &part.definition_id);
            let base_rotation = anchor
                .as_ref()
                .and_then(|anchor| anchor_key_base_rotation(document, anchor))
                .unwrap_or(0.0);
            Some((anchor, evaluated, courtyard_top, base_rotation))
        })
    };
    let Some((anchor, evaluated, courtyard_top, base_rotation)) = live else {
        return rsx! {};
    };
    let world_per_screen = handle_scale(&svg, width, height) / 3.0;
    let rail_tools = rail_peers(Rail::Part, &svg_handle);
    let rail_index = rail_tools.iter().position(|peer| *peer == tool);
    // The rail centres on the part: its evaluated centre with the top edge
    // of its courtyard, matching the matrix rail's centred row.
    let rail_top = Vec2 {
        x: evaluated.at.x,
        y: evaluated.at.y + courtyard_top,
    };
    let rail_layout =
        rail_index.and_then(|_| rail_over_points(&[rail_top], rail_tools.len(), world_per_screen));
    let Some(rail_layout) = rail_layout else {
        return rsx! {};
    };
    let (start_spread, start_stagger, start_offset, start_rotation) = anchor
        .as_ref()
        .map(|anchor| {
            (
                anchor.spread,
                anchor.stagger,
                anchor.offset,
                anchor.rotation,
            )
        })
        .unwrap_or((0.0, 0.0, Vec2::default(), evaluated.rotation));
    let handle_value = match tool {
        LayoutTransformTool::Spread => {
            format!("{:.2} mm", start_spread)
        }
        LayoutTransformTool::Stagger => {
            format!("{:.2} mm", start_stagger)
        }
        LayoutTransformTool::Rotate => format!("{start_rotation:.1}°"),
        // An anchored Move edits the local Offset; a detached Move edits the
        // board position, so each shows the value its drag writes.
        LayoutTransformTool::Move => {
            let at = if anchor.is_some() {
                start_offset
            } else {
                evaluated.at
            };
            format!("X {:.2}, Y {:.2} mm", at.x, at.y)
        }
        _ => format!("{start_stagger:.2} mm"),
    };
    let token_aria_label: String = match tool {
        LayoutTransformTool::Move => "Drag to move part".to_owned(),
        LayoutTransformTool::Rotate => "Drag to rotate part".to_owned(),
        LayoutTransformTool::Spread => "Drag to spread".to_owned(),
        _ => "Drag to stagger".to_owned(),
    };
    let token_description: String = match tool {
        LayoutTransformTool::Rotate => format!("{start_rotation:.2} degrees"),
        _ => handle_value.clone(),
    };
    let token_title: Option<String> = match tool {
        LayoutTransformTool::Move => Some(format!("Drag to move part · {handle_value}")),
        LayoutTransformTool::Rotate => {
            Some(format!("Drag to rotate part · {start_rotation:.2} degrees"))
        }
        LayoutTransformTool::Spread => Some(format!("Drag to spread · {handle_value}")),
        _ => None,
    };
    // Screen-frame glyph rotation for the one-dimensional carried axes: the
    // px groups below are y-down, so the world axis appears mirrored
    // vertically, exactly like the matrix tokens.
    let glyph_degrees = match tool {
        LayoutTransformTool::Spread => (-base_rotation.to_radians().sin())
            .atan2(base_rotation.to_radians().cos())
            .to_degrees(),
        LayoutTransformTool::Stagger => (base_rotation.to_radians().cos())
            .atan2(-base_rotation.to_radians().sin())
            .to_degrees(),
        _ => 0.0,
    };
    let active_peer = rail_active(Rail::Part, &svg_handle);
    let dragging_here = drag.borrow().is_some();
    // A live drag on a peer dims this token to 35%; the active token keeps
    // full opacity. Siblings re-read the shared mount set whenever the
    // Editor re-renders, which each preview submit triggers.
    let rail_dimmed = active_peer.is_some_and(|peer| peer != tool) && rail_index.is_some();
    let token_active = dragging_here;
    let token_lit = hovered() || focused() || token_active;
    let (token_fill, glyph_stroke) = token_paint(token_active, hovered() || focused());
    let token_opacity = if rail_dimmed { "0.35" } else { "1" };
    let guide_opacity = if token_active { "0.8" } else { "0.35" };
    let guide_axis = match tool {
        LayoutTransformTool::Spread => {
            let radians = base_rotation.to_radians();
            (radians.cos(), radians.sin())
        }
        _ => {
            let radians = base_rotation.to_radians();
            (-radians.sin(), radians.cos())
        }
    };
    let px = world_per_screen;
    let token_center: (f64, f64) = rail_index
        .and_then(|index| rail_layout.token_centers.get(index).copied())
        .unwrap_or((rail_top.x, rail_top.y));
    let feedback_state = feedback();

    let runtime_for_start = runtime.0.clone();
    let svg_for_start = svg.surface().clone();
    let drag_for_start = drag.clone();
    let arbiter_for_start = arbiter.clone();
    let context_for_start = part_context.clone();
    let owner_for_start = owner.clone();
    let part_for_start = part_id.clone();
    let svg_handle_for_start = svg_handle.clone();
    let start_drag = Rc::new(move |event: PointerEvent| {
        let Some(pointer) = event.data().try_as_web_event() else {
            return;
        };
        // A token press focuses the part's half, like a body click, so a
        // drag on the dimmed half keeps editing that half.
        on_focus_half.call(());
        if pointer.button() != 0 || drag_for_start.borrow().is_some() {
            return;
        }
        if !transform_owner_is_current(
            &runtime_for_start,
            workspace,
            scope_generation,
            selected_context,
            &owner_for_start,
            &context_for_start,
        ) || !part_start_is_editable(&runtime_for_start, &owner_for_start, &part_for_start)
            || runtime_for_start.model().gesture.is_some()
            || !arbiter_for_start.try_acquire(CanvasInteractionOwner::MatrixTransform)
        {
            return;
        }
        let Some(accepted) = runtime_for_start.model().accepted.clone() else {
            arbiter_for_start.release(CanvasInteractionOwner::MatrixTransform);
            return;
        };
        let document = &accepted.document;
        let Some(part) = document.parts.iter().find(|part| part.id == part_for_start) else {
            arbiter_for_start.release(CanvasInteractionOwner::MatrixTransform);
            return;
        };
        if part.locked == Some(true)
            || document
                .matrices
                .iter()
                .any(|matrix| matrix.part_ids.contains(&part.id))
        {
            arbiter_for_start.release(CanvasInteractionOwner::MatrixTransform);
            return;
        }
        let anchor = part.anchor.clone();
        let Some(kind) = part_drag_kind(tool, anchor.is_some()) else {
            arbiter_for_start.release(CanvasInteractionOwner::MatrixTransform);
            return;
        };
        if let PartRailDragKind::AnchorValue(_) = kind
            && part_anchor_drag_guard(document, &part_for_start).is_some()
        {
            arbiter_for_start.release(CanvasInteractionOwner::MatrixTransform);
            return;
        }
        let Some(capture) = pointer
            .target()
            .and_then(|target| target.dyn_into::<web_sys::Element>().ok())
            .and_then(|target| target.closest("[data-transform-handle]").ok().flatten())
            .and_then(|target| target.dyn_into::<SvgElement>().ok())
        else {
            arbiter_for_start.release(CanvasInteractionOwner::MatrixTransform);
            return;
        };
        if capture.set_pointer_capture(pointer.pointer_id()).is_err() {
            arbiter_for_start.release(CanvasInteractionOwner::MatrixTransform);
            return;
        }
        pointer.prevent_default();
        pointer.stop_propagation();
        let focus_options = web_sys::FocusOptions::new();
        focus_options.set_prevent_scroll(true);
        let _ = capture.focus_with_options(&focus_options);
        let Some(start_point) = crate::presentation::coordinates(
            &svg_for_start,
            &pointer,
            view_x,
            view_y,
            width,
            height,
        ) else {
            let _ = capture.release_pointer_capture(pointer.pointer_id());
            arbiter_for_start.release(CanvasInteractionOwner::MatrixTransform);
            return;
        };
        let evaluated = evaluated_part_pose(document, part);
        let base_rotation = anchor
            .as_ref()
            .and_then(|anchor| anchor_key_base_rotation(document, anchor))
            .unwrap_or(0.0);
        let splay = anchor.as_ref().map_or(0.0, |anchor| anchor.splay);
        let (start_spread, start_stagger, start_offset, start_rotation) = anchor
            .as_ref()
            .map(|anchor| {
                (
                    anchor.spread,
                    anchor.stagger,
                    anchor.offset,
                    anchor.rotation,
                )
            })
            .unwrap_or((0.0, 0.0, Vec2::default(), evaluated.rotation));
        let operation = runtime_for_start.operation();
        *drag_for_start.borrow_mut() = Some(PartDrag {
            pointer_id: i64::from(pointer.pointer_id()),
            capture,
            part_id: part_for_start.clone(),
            kind,
            tool,
            transaction_id: format!("layout-part-rail-{}", operation.0),
            owner: owner_for_start.clone(),
            context: context_for_start.clone(),
            anchor,
            start_spread,
            start_stagger,
            start_offset,
            start_rotation,
            start_at: evaluated.at,
            base_rotation,
            frame_rotation: base_rotation + splay,
            center: evaluated.at,
            start_angle: (start_point.y - evaluated.at.y).atan2(start_point.x - evaluated.at.x),
            start_client_x: f64::from(pointer.client_x()),
            start_client_y: f64::from(pointer.client_y()),
            gesture_begun: false,
            preview_submitted: false,
            pending: None,
        });
        let mut popup_at_for_start = popup_at;
        popup_at_for_start.set(Some((
            start_point.x + 18.0 * world_per_screen,
            start_point.y - 18.0 * world_per_screen,
        )));
        set_rail_active(Rail::Part, &svg_handle_for_start, Some(tool));
    });

    let runtime_for_move = runtime.0.clone();
    let svg_for_move = svg.surface().clone();
    let svg_for_move_cancel = svg_handle.clone();
    let drag_for_move = drag.clone();
    let arbiter_for_move = arbiter.clone();
    let feedback_for_move = feedback;
    let popup_at_for_move = popup_at;
    let move_drag = Rc::new(move |event: PointerEvent| {
        let mut feedback_for_move = feedback_for_move;
        let mut popup_at_for_move = popup_at_for_move;
        let Some(pointer) = event.data().try_as_web_event() else {
            return;
        };
        let pointer_id = i64::from(pointer.pointer_id());
        let Some(mut active) = drag_for_move
            .borrow()
            .clone()
            .filter(|drag| drag.pointer_id == pointer_id)
        else {
            return;
        };
        pointer.prevent_default();
        pointer.stop_propagation();
        if !transform_owner_is_current(
            &runtime_for_move,
            workspace,
            scope_generation,
            selected_context,
            &active.owner,
            &active.context,
        ) || active.tool != tool
        {
            cancel_part_drag(
                &runtime_for_move,
                &drag_for_move,
                &arbiter_for_move,
                &svg_for_move_cancel,
            );
            return;
        }
        let Some(rect) = svg_for_move
            .borrow()
            .as_ref()
            .map(|surface| surface.get_bounding_client_rect())
        else {
            return;
        };
        if rect.width() <= 0.0 || rect.height() <= 0.0 {
            return;
        }
        let Some((_, scale)) = crate::presentation::coordinates_with_scale(
            &svg_for_move,
            &pointer,
            view_x,
            view_y,
            width,
            height,
        ) else {
            return;
        };
        let world_delta = Vec2 {
            x: (f64::from(pointer.client_x()) - active.start_client_x) * width / rect.width(),
            y: -(f64::from(pointer.client_y()) - active.start_client_y) * height / rect.height(),
        };
        let alt = pointer.alt_key();
        let tool_name = feedback_tool_name(tool).to_owned();
        match active.kind {
            PartRailDragKind::AnchorValue(axis) => {
                let Some(point) = crate::presentation::coordinates(
                    &svg_for_move,
                    &pointer,
                    view_x,
                    view_y,
                    width,
                    height,
                ) else {
                    return;
                };
                let Some((operation, delta_feedback, _, value_text)) = sample_anchor_value(
                    &active,
                    axis,
                    world_delta,
                    point,
                    alt,
                    snap.pitch,
                    snap.snap_fraction,
                ) else {
                    return;
                };
                if active.pending.as_ref() != Some(&operation) {
                    submit_part_preview(&runtime_for_move, &active, operation.clone());
                    active.preview_submitted = true;
                    // The popup follows the pointer with the live value and
                    // delta, like the matrix rail.
                    feedback_for_move.set(advance_feedback(
                        &feedback_for_move(),
                        FeedbackEvent::Drag {
                            name: tool_name,
                            value: value_text,
                            delta: delta_feedback,
                        },
                    ));
                    popup_at_for_move.set(Some((point.x + 18.0 * px, point.y - 18.0 * px)));
                    active.pending = Some(operation);
                    *drag_for_move.borrow_mut() = Some(active);
                }
            }
            PartRailDragKind::DetachedMove => {
                // The shipped body threshold keeps clicks out of the session
                // gesture; only real motion begins it.
                if !crate::presentation::canvas_interaction::pending_part_drag_threshold_reached(
                    (active.start_client_x, active.start_client_y),
                    (f64::from(pointer.client_x()), f64::from(pointer.client_y())),
                ) {
                    return;
                }
                let at = Vec2 {
                    x: active.start_at.x
                        + (f64::from(pointer.client_x()) - active.start_client_x) / scale,
                    y: active.start_at.y
                        - (f64::from(pointer.client_y()) - active.start_client_y) / scale,
                };
                let positions = vec![Position {
                    id: active.part_id.clone(),
                    at,
                }];
                if !active.gesture_begun {
                    let operation = runtime_for_move.operation();
                    runtime_for_move.submit(Event::GestureBegin {
                        operation_id: operation,
                        pointer_id: active.pointer_id,
                        target_ids: vec![active.part_id.clone()],
                        transaction_id: active.transaction_id.clone(),
                        start: vec![Position {
                            id: active.part_id.clone(),
                            at: active.start_at,
                        }],
                        pitch: snap.pitch,
                        snap_fraction: snap.snap_fraction,
                        geometry_snap: snap.geometry_snap,
                        gap: snap.gap,
                        pixels_per_world: Some(scale),
                        alt,
                    });
                    let fresh = runtime_for_move.model();
                    let admitted = fresh.gesture.as_ref().is_some_and(|gesture| {
                        gesture.pointer_id == active.pointer_id
                            && gesture.target_ids == vec![active.part_id.clone()]
                    });
                    if !admitted {
                        cancel_part_drag(
                            &runtime_for_move,
                            &drag_for_move,
                            &arbiter_for_move,
                            &svg_for_move_cancel,
                        );
                        return;
                    }
                    active.gesture_begun = true;
                }
                runtime_for_move.submit(Event::GestureSample {
                    pointer_id: active.pointer_id,
                    positions: positions.clone(),
                    pixels_per_world: Some(scale),
                    alt,
                });
                feedback_for_move.set(advance_feedback(
                    &feedback_for_move(),
                    FeedbackEvent::Drag {
                        name: tool_name,
                        value: format!("X {:.2}, Y {:.2} mm", at.x, at.y),
                        delta: FeedbackDelta::Mm2(
                            at.x - active.start_at.x,
                            at.y - active.start_at.y,
                        ),
                    },
                ));
                if let Some(point) = crate::presentation::coordinates(
                    &svg_for_move,
                    &pointer,
                    view_x,
                    view_y,
                    width,
                    height,
                ) {
                    popup_at_for_move.set(Some((point.x + 18.0 * px, point.y - 18.0 * px)));
                }
                *drag_for_move.borrow_mut() = Some(active);
            }
            PartRailDragKind::DetachedRotate => {
                let Some(point) = crate::presentation::coordinates(
                    &svg_for_move,
                    &pointer,
                    view_x,
                    view_y,
                    width,
                    height,
                ) else {
                    return;
                };
                let angle = (point.y - active.center.y).atan2(point.x - active.center.x)
                    - active.start_angle;
                let swept = angle.sin().atan2(angle.cos()).to_degrees();
                let value = part_rotate_value(active.start_rotation, swept, alt);
                let delta = value - active.start_rotation;
                let operation = EditOperation::SetPartRotation {
                    part_id: active.part_id.clone(),
                    rotation: value,
                };
                if active.pending.as_ref() != Some(&operation) {
                    submit_part_preview(&runtime_for_move, &active, operation.clone());
                    active.preview_submitted = true;
                    feedback_for_move.set(advance_feedback(
                        &feedback_for_move(),
                        FeedbackEvent::Drag {
                            name: tool_name,
                            value: format!("{value:.1}°"),
                            delta: FeedbackDelta::Deg(delta),
                        },
                    ));
                    popup_at_for_move.set(Some((point.x + 18.0 * px, point.y - 18.0 * px)));
                    active.pending = Some(operation);
                    *drag_for_move.borrow_mut() = Some(active);
                }
            }
        }
    });

    let runtime_for_end = runtime.0.clone();
    let svg_for_end = svg.surface().clone();
    let svg_handle_for_end = svg_handle.clone();
    let drag_for_end = drag.clone();
    let arbiter_for_end = arbiter.clone();
    let feedback_for_end = feedback;
    let popup_at_for_end = popup_at;
    let mut pending_for_end = pending;
    let mut toast_gen_for_end = toast_gen;
    let handle_value_for_end = handle_value.clone();
    let end_drag = Rc::new(move |event: PointerEvent| {
        let mut feedback_for_end = feedback_for_end;
        let mut popup_at_for_end = popup_at_for_end;
        let mut pending_for_end = pending_for_end;
        let mut toast_gen_for_end = toast_gen_for_end;
        let Some(pointer) = event.data().try_as_web_event() else {
            return;
        };
        let pointer_id = i64::from(pointer.pointer_id());
        let Some(active) = drag_for_end
            .borrow()
            .clone()
            .filter(|drag| drag.pointer_id == pointer_id)
        else {
            return;
        };
        pointer.prevent_default();
        pointer.stop_propagation();
        if !transform_owner_is_current(
            &runtime_for_end,
            workspace,
            scope_generation,
            selected_context,
            &active.owner,
            &active.context,
        ) || active.tool != tool
        {
            cancel_part_drag(
                &runtime_for_end,
                &drag_for_end,
                &arbiter_for_end,
                &svg_handle_for_end,
            );
            return;
        }
        if pointer.type_() != "pointerup" {
            cancel_part_drag(
                &runtime_for_end,
                &drag_for_end,
                &arbiter_for_end,
                &svg_handle_for_end,
            );
            return;
        }
        let alt = pointer.alt_key();
        let tool_name = feedback_tool_name(tool).to_owned();
        // Landing one revision, with a toast unless the drag netted nothing.
        let mut landed = |feedback: FeedbackDelta, value_text: String| {
            if feedback.is_zero() {
                return false;
            }
            feedback_for_end.set(advance_feedback(
                &feedback_for_end(),
                FeedbackEvent::Commit {
                    value: format!("{tool_name} {value_text}"),
                    delta: feedback,
                },
            ));
            let seen = {
                toast_gen_for_end.set(toast_gen_for_end() + 1);
                toast_gen_for_end()
            };
            arm_commit_toast_expiry(feedback_for_end, toast_gen_for_end, seen);
            true
        };
        match active.kind {
            PartRailDragKind::AnchorValue(axis) => {
                let sample = crate::presentation::coordinates(
                    &svg_for_end,
                    &pointer,
                    view_x,
                    view_y,
                    width,
                    height,
                )
                .and_then(|point| {
                    let rect = svg_for_end
                        .borrow()
                        .as_ref()
                        .map(|surface| surface.get_bounding_client_rect())?;
                    if rect.width() <= 0.0 || rect.height() <= 0.0 {
                        return None;
                    }
                    let world_delta = Vec2 {
                        x: (f64::from(pointer.client_x()) - active.start_client_x) * width
                            / rect.width(),
                        y: -(f64::from(pointer.client_y()) - active.start_client_y) * height
                            / rect.height(),
                    };
                    sample_anchor_value(
                        &active,
                        axis,
                        world_delta,
                        point,
                        alt,
                        snap.pitch,
                        snap.snap_fraction,
                    )
                })
                .or_else(|| {
                    active.pending.as_ref().and_then(|operation| {
                        let EditOperation::SetPartAnchorValues { change, .. } = operation else {
                            return None;
                        };
                        let (feedback, delta, value_text) = match (axis, change) {
                            (PartAnchorAxis::Spread, PartAnchorValueChange::Spread(value)) => (
                                FeedbackDelta::Mm(value - active.start_spread),
                                PartAxisDelta::Spread(value - active.start_spread),
                                format_placement_value(PlacementValue::Spread(*value)),
                            ),
                            (PartAnchorAxis::Stagger, PartAnchorValueChange::Stagger(value)) => (
                                FeedbackDelta::Mm(value - active.start_stagger),
                                PartAxisDelta::Stagger(value - active.start_stagger),
                                format_placement_value(PlacementValue::Stagger(*value)),
                            ),
                            (PartAnchorAxis::Offset, PartAnchorValueChange::Offset(value)) => (
                                FeedbackDelta::Mm2(
                                    value.x - active.start_offset.x,
                                    value.y - active.start_offset.y,
                                ),
                                PartAxisDelta::Offset(Vec2 {
                                    x: value.x - active.start_offset.x,
                                    y: value.y - active.start_offset.y,
                                }),
                                format_placement_value(PlacementValue::Offset(*value)),
                            ),
                            (PartAnchorAxis::Rotation, PartAnchorValueChange::Rotation(value)) => (
                                FeedbackDelta::Deg(value - active.start_rotation),
                                PartAxisDelta::Rotation(value - active.start_rotation),
                                format!("{value:.1}°"),
                            ),
                            _ => return None,
                        };
                        Some((operation.clone(), feedback, delta, value_text))
                    })
                });
                if let Some((operation, feedback, delta, value_text)) = sample {
                    if active.pending.as_ref() != Some(&operation) {
                        submit_part_preview(&runtime_for_end, &active, operation);
                    }
                    if feedback.is_zero() {
                        clear_part_preview(&runtime_for_end, &active);
                        feedback_for_end.set(advance_feedback(
                            &feedback_for_end(),
                            FeedbackEvent::Choose {
                                name: tool_name,
                                value: value_text,
                            },
                        ));
                    } else {
                        let ticket = commit_part_anchor_drag(&runtime_for_end, &active, delta);
                        pending_for_end.write().push(PendingTransformCommit {
                            ticket,
                            owner: active.owner.clone(),
                            context: active.context.clone(),
                            toast: Some(PendingTransformToast {
                                tool_name: feedback_tool_name(tool),
                                value: value_text,
                                delta: feedback,
                            }),
                        });
                    }
                } else if active.preview_submitted {
                    clear_part_preview(&runtime_for_end, &active);
                    feedback_for_end.set(advance_feedback(
                        &feedback_for_end(),
                        FeedbackEvent::Choose {
                            name: tool_name,
                            value: handle_value_for_end.clone(),
                        },
                    ));
                }
            }
            PartRailDragKind::DetachedMove => {
                if active.gesture_begun {
                    let Some((_, scale)) = crate::presentation::coordinates_with_scale(
                        &svg_for_end,
                        &pointer,
                        view_x,
                        view_y,
                        width,
                        height,
                    ) else {
                        cancel_part_drag(
                            &runtime_for_end,
                            &drag_for_end,
                            &arbiter_for_end,
                            &svg_handle_for_end,
                        );
                        return;
                    };
                    let positions = vec![Position {
                        id: active.part_id.clone(),
                        at: Vec2 {
                            x: active.start_at.x
                                + (f64::from(pointer.client_x()) - active.start_client_x) / scale,
                            y: active.start_at.y
                                - (f64::from(pointer.client_y()) - active.start_client_y) / scale,
                        },
                    }];
                    runtime_for_end.submit(Event::GestureEnd {
                        pointer_id: active.pointer_id,
                        final_positions: positions.clone(),
                        pixels_per_world: Some(scale),
                        alt,
                    });
                    let at = &positions[0].at;
                    let delta =
                        FeedbackDelta::Mm2(at.x - active.start_at.x, at.y - active.start_at.y);
                    let value_text = format!("X {:.2}, Y {:.2} mm", at.x, at.y);
                    if !landed(delta, value_text.clone()) {
                        feedback_for_end.set(advance_feedback(
                            &feedback_for_end(),
                            FeedbackEvent::Choose {
                                name: tool_name,
                                value: value_text,
                            },
                        ));
                    }
                } else {
                    feedback_for_end.set(advance_feedback(
                        &feedback_for_end(),
                        FeedbackEvent::Dismiss,
                    ));
                }
            }
            PartRailDragKind::DetachedRotate => {
                let sample = crate::presentation::coordinates(
                    &svg_for_end,
                    &pointer,
                    view_x,
                    view_y,
                    width,
                    height,
                )
                .map(|point| {
                    let angle = (point.y - active.center.y).atan2(point.x - active.center.x)
                        - active.start_angle;
                    let swept = angle.sin().atan2(angle.cos()).to_degrees();
                    let value = part_rotate_value(active.start_rotation, swept, alt);
                    let delta = value - active.start_rotation;
                    (
                        EditOperation::SetPartRotation {
                            part_id: active.part_id.clone(),
                            rotation: value,
                        },
                        FeedbackDelta::Deg(delta),
                        delta,
                        format!("{value:.1}°"),
                    )
                })
                .or_else(|| {
                    active.pending.as_ref().and_then(|operation| {
                        let EditOperation::SetPartRotation { rotation, .. } = operation else {
                            return None;
                        };
                        let delta = rotation - active.start_rotation;
                        Some((
                            operation.clone(),
                            FeedbackDelta::Deg(delta),
                            delta,
                            format!("{rotation:.1}°"),
                        ))
                    })
                });
                if let Some((operation, feedback, delta, value_text)) = sample {
                    if active.pending.as_ref() != Some(&operation) {
                        submit_part_preview(&runtime_for_end, &active, operation);
                    }
                    if feedback.is_zero() {
                        clear_part_preview(&runtime_for_end, &active);
                        feedback_for_end.set(advance_feedback(
                            &feedback_for_end(),
                            FeedbackEvent::Choose {
                                name: tool_name,
                                value: value_text,
                            },
                        ));
                    } else {
                        let ticket = commit_part_detached_rotate(&runtime_for_end, &active, delta);
                        pending_for_end.write().push(PendingTransformCommit {
                            ticket,
                            owner: active.owner.clone(),
                            context: active.context.clone(),
                            toast: Some(PendingTransformToast {
                                tool_name: feedback_tool_name(tool),
                                value: value_text,
                                delta: feedback,
                            }),
                        });
                    }
                } else if active.preview_submitted {
                    clear_part_preview(&runtime_for_end, &active);
                    feedback_for_end.set(advance_feedback(
                        &feedback_for_end(),
                        FeedbackEvent::Choose {
                            name: tool_name,
                            value: handle_value_for_end.clone(),
                        },
                    ));
                }
            }
        }
        popup_at_for_end.set(None);
        set_rail_active(Rail::Part, &svg_handle_for_end, None);
        drag_for_end.borrow_mut().take();
        if active.capture.has_pointer_capture(pointer.pointer_id()) {
            let _ = active.capture.release_pointer_capture(pointer.pointer_id());
        }
        arbiter_for_end.release(CanvasInteractionOwner::MatrixTransform);
    });

    let runtime_for_cancel = runtime.0.clone();
    let drag_for_cancel = drag.clone();
    let arbiter_for_cancel = arbiter.clone();
    let feedback_for_cancel = feedback;
    let popup_at_for_cancel = popup_at;
    let svg_for_cancel = svg_handle.clone();
    let cancel_drag = Rc::new(move |event: PointerEvent| {
        let mut feedback_for_cancel = feedback_for_cancel;
        let mut popup_at_for_cancel = popup_at_for_cancel;
        let Some(pointer) = event.data().try_as_web_event() else {
            return;
        };
        if drag_for_cancel
            .borrow()
            .as_ref()
            .is_some_and(|drag| drag.pointer_id == i64::from(pointer.pointer_id()))
        {
            pointer.prevent_default();
            pointer.stop_propagation();
            // A cancelled drag takes its popup with it.
            feedback_for_cancel.set(advance_feedback(
                &feedback_for_cancel(),
                FeedbackEvent::Dismiss,
            ));
            popup_at_for_cancel.set(None);
            cancel_part_drag(
                &runtime_for_cancel,
                &drag_for_cancel,
                &arbiter_for_cancel,
                &svg_for_cancel,
            );
        }
    });

    let on_key_down = Rc::new(move |event: KeyboardEvent| {
        let Some(key) = event.data().try_as_web_event() else {
            return;
        };
        // Escape clears the drag and its popup. Arrow keys do not nudge
        // parts: the ticket carries no part nudge, so the rail leaves them
        // to the canvas.
        if key.key() != "Escape" {
            return;
        }
        key.prevent_default();
        key.stop_propagation();
        escape_drag();
    });

    let token_style = format!("cursor:grab;opacity:{token_opacity};touch-action:none");
    let glyph = rail_token_glyph(tool, false, glyph_degrees, glyph_stroke);
    let face = token_face(token_fill, glyph, token_title.as_deref());
    let show_hover_name = feedback_tool_name(tool).to_owned();
    let show_hover_value = handle_value.clone();
    let show_focus_name = feedback_tool_name(tool).to_owned();
    let show_focus_value = handle_value.clone();
    // Named mounted control for parent integration: present while a part
    // commit is queued.
    let pending_commits = pending.read().len();
    rsx! {
        if pending_commits > 0 {
            g {
                "data-transform-pending": "true",
                "data-transform-tool": "{tool:?}",
                style: "display:none",
                "aria-hidden": "true",
            }
        }
        if token_lit
            && matches!(
                tool,
                LayoutTransformTool::Spread | LayoutTransformTool::Stagger
            )
        {
            {axis_guide(token_center, guide_axis, world_per_screen, guide_opacity)}
        }
        {
            let start = start_drag.clone();
            let move_handler = move_drag.clone();
            let end_handler = end_drag.clone();
            let cancel_handler = cancel_drag.clone();
            let lost_capture_handler = cancel_drag.clone();
            let key_handler = on_key_down.clone();
            rsx! {
                if rail_index == Some(0) {
                    {rail_backdrop(&rail_layout, rail_tools.len(), px)}
                }
                g {
                    "data-transform-handle": "true", class: "outline-none focus-visible:stroke-foreground",
                    transform: "translate({token_center.0} {token_center.1}) scale({px} {-px})",
                    role: "button", tabindex: "0", "aria-label": "{token_aria_label}",
                    "aria-description": "{token_description}",
                    style: "{token_style}",
                    onpointerdown: move |event| start(event),
                    onpointermove: move |event| move_handler(event),
                    onpointerup: move |event| end_handler(event),
                    onpointercancel: move |event| cancel_handler(event),
                    onlostpointercapture: move |event| lost_capture_handler(event),
                    onkeydown: move |event| key_handler(event),
                    // Hovering or focusing a token docks the value popup
                    // above the rail; leaving clears it. The content comes
                    // from the pure reducer, so the chrome never writes.
                    onpointerenter: { let mut hovered = hovered; let mut feedback = feedback; let mut popup_at = popup_at; move |_| { hovered.set(true); popup_at.set(None); feedback.set(advance_feedback(&feedback(), FeedbackEvent::Choose { name: show_hover_name.clone(), value: show_hover_value.clone() })); } },
                    onpointerleave: { let mut hovered = hovered; let mut feedback = feedback; move |_| { hovered.set(false); feedback.set(advance_feedback(&feedback(), FeedbackEvent::Dismiss)); } },
                    onfocus: { let mut focused = focused; let mut feedback = feedback; let mut popup_at = popup_at; move |_| { focused.set(true); popup_at.set(None); feedback.set(advance_feedback(&feedback(), FeedbackEvent::Choose { name: show_focus_name.clone(), value: show_focus_value.clone() })); } },
                    onblur: { let mut focused = focused; let mut feedback = feedback; move |_| { focused.set(false); feedback.set(advance_feedback(&feedback(), FeedbackEvent::Dismiss)); } },
                    {face}
                }
            }
        }
        // The value popup — docked 8 px above the rail while a token is
        // hovered or focused, 18 px off the pointer while dragging — and the
        // commit toast at the top centre of the canvas.
        {feedback_chrome(&feedback_state, true, popup_at(), Some(&rail_layout), px, (view_x, view_y, width))}
    }
}
