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
    // Single-ticket part commits settle on the production `version` signal
    // (durable toast on landing, `report` on failure, silent on retirement).
    // No keyed map or polling loop is kept.
    let pending = use_signal(Vec::<PendingTransformCommit>::new);
    use_settled_transform_commits(
        runtime.0.clone(),
        workspace,
        scope_generation,
        selected_context,
        pending,
        feedback,
        toast_gen,
    );
    let part_context = TreeContext::Component {
        part_id: Some(part_id.clone()),
        matrix_id: None,
        row: None,
        column: None,
        assembly_id: None,
    };
    let cancel = {
        let runtime = runtime.0.clone();
        let drag = drag.clone();
        let arbiter = arbiter.clone();
        let svg = svg_handle.clone();
        move || {
            cancel_part_drag(&runtime, &drag, &arbiter, &svg);
        }
    };
    let escape_drag = use_rail_drag_cancel(
        &drag,
        &owner,
        &part_context,
        tool,
        selected_context,
        scope_generation,
        &svg_handle,
        &arbiter,
        feedback,
        popup_at,
        on_finish,
        cancel.clone(),
    );
    let drag_event = rail_drag_events(
        &drag,
        &runtime.0,
        workspace,
        scope_generation,
        selected_context,
        tool,
        cancel.clone(),
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
    let rail_dimmed = rail_dimmed(active_peer, tool, rail_index);
    let token_active = dragging_here;
    let token_lit = hovered() || focused() || token_active;
    let (token_fill, glyph_stroke) = token_paint(token_active, hovered() || focused());
    let token_opacity = token_opacity(rail_dimmed);
    let guide_opacity = guide_opacity(token_active);
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
        let Some(capture) =
            capture_rail_pointer(&pointer, "[data-transform-handle]", &arbiter_for_start)
        else {
            return;
        };
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
            rail: RailDrag {
                pointer_id: pointer.pointer_id(),
                capture,
                transaction_id: format!("layout-part-rail-{}", operation.0),
                owner: owner_for_start.clone(),
                context: context_for_start.clone(),
                tool,
                preview: DragPreview::default(),
            },
            part_id: part_for_start.clone(),
            kind,
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
        });
        let mut popup_at_for_start = popup_at;
        popup_at_for_start.set(Some((
            start_point.x + 18.0 * world_per_screen,
            start_point.y - 18.0 * world_per_screen,
        )));
        set_rail_active(Rail::Part, &svg_handle_for_start, Some(tool));
    });

    let surface = svg.surface().clone();
    let pointer_sample = move |active: &PartDrag, pointer: &web_sys::PointerEvent| {
        let (point, scale) = crate::presentation::coordinates_with_scale(
            &surface, pointer, view_x, view_y, width, height,
        )?;
        let rect = surface.borrow().as_ref()?.get_bounding_client_rect();
        if rect.width() <= 0.0 || rect.height() <= 0.0 {
            return None;
        }
        // Local anchor edits use the viewport axes. The Session body gesture uses its
        // uniform pixel scale instead, as it does when started from the part itself.
        let world_delta = Vec2 {
            x: (f64::from(pointer.client_x()) - active.start_client_x) * width / rect.width(),
            y: -(f64::from(pointer.client_y()) - active.start_client_y) * height / rect.height(),
        };
        Some((point, scale, world_delta))
    };
    let runtime_for_move = runtime.0.clone();
    let drag_for_move = drag.clone();
    let move_event = drag_event.clone();
    let sample_for_move = pointer_sample.clone();
    let cancel_for_move = cancel.clone();
    let move_drag = Rc::new(move |event: PointerEvent| {
        let Some((pointer, mut active)) = move_event(event, false) else {
            return;
        };
        let Some((point, scale, world_delta)) = sample_for_move(&active, &pointer) else {
            return;
        };
        let alt = pointer.alt_key();
        let (value, delta) = if active.kind == PartRailDragKind::DetachedMove {
            // A click stays out of the Session gesture, including a release without a move.
            if !crate::presentation::canvas_interaction::pending_part_drag_threshold_reached(
                (active.start_client_x, active.start_client_y),
                (f64::from(pointer.client_x()), f64::from(pointer.client_y())),
            ) {
                return;
            }
            let pointer_id = i64::from(active.rail.pointer_id);
            let at = detached_part_position(&active, &pointer, scale);
            if !active.gesture_begun {
                runtime_for_move.submit(Event::GestureBegin {
                    operation_id: runtime_for_move.operation(),
                    pointer_id,
                    target_ids: vec![active.part_id.clone()],
                    transaction_id: active.rail.transaction_id.clone(),
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
                    gesture.pointer_id == pointer_id
                        && gesture.target_ids == vec![active.part_id.clone()]
                });
                if !admitted {
                    cancel_for_move();
                    return;
                }
                active.gesture_begun = true;
            }
            runtime_for_move.submit(Event::GestureSample {
                pointer_id,
                positions: vec![Position {
                    id: active.part_id.clone(),
                    at,
                }],
                pixels_per_world: Some(scale),
                alt,
            });
            detached_part_feedback(active.start_at, at)
        } else {
            let Some(operation) = sample_part_preview(
                &active,
                world_delta,
                point,
                alt,
                snap.pitch,
                snap.snap_fraction,
            ) else {
                return;
            };
            let Some((delta, _, value)) = part_preview_result(&active, &operation) else {
                return;
            };
            if !active.rail.preview.sample(&operation) {
                return;
            }
            active
                .rail
                .submit_preview(&runtime_for_move, &active.part_id, operation);
            (value, delta)
        };
        let (mut feedback, mut popup_at) = (feedback, popup_at);
        feedback.set(advance_feedback(
            &feedback(),
            FeedbackEvent::Drag {
                name: feedback_tool_name(tool).to_owned(),
                value,
                delta,
            },
        ));
        popup_at.set(Some((point.x + 18.0 * px, point.y - 18.0 * px)));
        *drag_for_move.borrow_mut() = Some(active);
    });

    let runtime_for_end = runtime.0.clone();
    let svg_for_end = svg_handle.clone();
    let drag_for_end = drag.clone();
    let arbiter_for_end = arbiter.clone();
    let cancel_for_end = cancel.clone();
    let end_drag = Rc::new(move |event: PointerEvent| {
        let Some((pointer, active)) = drag_event(event, true) else {
            return;
        };
        let (mut feedback, mut popup_at, mut pending) = (feedback, popup_at, pending);
        let tool_name = feedback_tool_name(tool);
        let alt = pointer.alt_key();
        let follow_up = if active.kind == PartRailDragKind::DetachedMove {
            if active.gesture_begun {
                let Some((_, scale, _)) = pointer_sample(&active, &pointer) else {
                    cancel_for_end();
                    return;
                };
                let at = detached_part_position(&active, &pointer, scale);
                runtime_for_end.submit(Event::GestureEnd {
                    pointer_id: i64::from(active.rail.pointer_id),
                    final_positions: vec![Position {
                        id: active.part_id.clone(),
                        at,
                    }],
                    pixels_per_world: Some(scale),
                    alt,
                });
                let (value, delta) = detached_part_feedback(active.start_at, at);
                if delta.is_zero() {
                    Some(FeedbackEvent::Choose {
                        name: tool_name.to_owned(),
                        value,
                    })
                } else {
                    raise_commit_toast(feedback, toast_gen, format!("{tool_name} {value}"), delta);
                    None
                }
            } else {
                Some(FeedbackEvent::Dismiss)
            }
        } else {
            let operation =
                pointer_sample(&active, &pointer).and_then(|(point, _, world_delta)| {
                    sample_part_preview(
                        &active,
                        world_delta,
                        point,
                        alt,
                        snap.pitch,
                        snap.snap_fraction,
                    )
                });
            active.rail.preview.finish(operation).and_then(|sample| {
                let (feedback_delta, delta, value) =
                    part_preview_result(&active, &sample.operation)?;
                if sample.submit_preview {
                    active
                        .rail
                        .submit_preview(&runtime_for_end, &active.part_id, sample.operation);
                }
                if feedback_delta.is_zero() {
                    if active.rail.preview.has_preview() {
                        active.rail.clear_preview(&runtime_for_end);
                    }
                    return Some(FeedbackEvent::Choose {
                        name: tool_name.to_owned(),
                        value,
                    });
                }
                let part_id = active.part_id.clone();
                let transaction_id = active.rail.transaction_id.clone();
                let resolver = match (active.kind, delta) {
                    (PartRailDragKind::DetachedRotate, PartAxisDelta::Rotation(delta)) => {
                        part_detached_rotate_resolver(part_id, transaction_id, delta)
                    }
                    (_, delta) => part_anchor_drag_resolver(part_id, transaction_id, delta),
                };
                let ticket = runtime_for_end.resolve_edit(
                    "layout-part-rail-drag",
                    Some("transform".into()),
                    resolver,
                );
                pending.write().push(PendingTransformCommit {
                    ticket,
                    owner: active.rail.owner.clone(),
                    context: active.rail.context.clone(),
                    toast: Some(PendingTransformToast {
                        tool_name,
                        value,
                        delta: feedback_delta,
                    }),
                });
                None
            })
        };
        if let Some(event) = follow_up {
            feedback.set(advance_feedback(&feedback(), event));
        }
        popup_at.set(None);
        set_rail_active(Rail::Part, &svg_for_end, None);
        drag_for_end.borrow_mut().take();
        active.rail.release(&arbiter_for_end);
    });

    let cancel_drag = Rc::new(move |event: PointerEvent| {
        cancel_rail_pointer(event, &drag, feedback, popup_at, &cancel);
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

    let token_style = rail_token_style(token_opacity);
    let glyph = rail_token_glyph(tool, false, glyph_degrees, glyph_stroke);
    let face = token_face(token_fill, glyph, token_title.as_deref());
    let show_hover_name = feedback_tool_name(tool).to_owned();
    let show_hover_value = handle_value.clone();
    let show_focus_name = feedback_tool_name(tool).to_owned();
    let show_focus_value = handle_value.clone();
    rsx! {
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

fn detached_part_position(drag: &PartDrag, pointer: &web_sys::PointerEvent, scale: f64) -> Vec2 {
    Vec2 {
        x: drag.start_at.x + (f64::from(pointer.client_x()) - drag.start_client_x) / scale,
        y: drag.start_at.y - (f64::from(pointer.client_y()) - drag.start_client_y) / scale,
    }
}

fn detached_part_feedback(start: Vec2, at: Vec2) -> (String, FeedbackDelta) {
    (
        format!("X {:.2}, Y {:.2} mm", at.x, at.y),
        FeedbackDelta::Mm2(at.x - start.x, at.y - start.y),
    )
}
