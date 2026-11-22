//! The matrix rail overlay: one token (or the splay origin point) per mounted tool.
use super::*;

mod helpers;
mod token;
use helpers::{angle_handle_point, splay_arc};
use token::{MatrixTokenView, matrix_token_chrome};

#[component]
pub fn LayoutTransformToolOverlay(
    runtime: LayoutTransformRuntime,
    svg: LayoutTransformSvg,
    arbiter: CanvasInteractionArbiter,
    owner: LayoutOwnerIdentity,
    selected_context: Signal<Option<ScopedTreeContext>>,
    scope_generation: Signal<u64>,
    workspace: Signal<&'static str>,
    #[props(into)] matrix: Rc<Matrix>,
    #[props(into)] projection: Rc<MatrixScene>,
    context: TreeContext,
    tool: LayoutTransformTool,
    #[props(default)] surface_size: Option<Signal<(f64, f64)>>,
    snap_settings: super::super::layout_toolbar::LayoutSnapSettings,
    #[props(into)] snap_origins: Rc<Vec<crate::outline_snapping::Origin>>,
    splay_affect: Signal<MatrixSplayAffect>,
    view_x: f64,
    view_y: f64,
    width: f64,
    height: f64,
    on_finish: EventHandler<()>,
) -> Element {
    // W3-C: the rail peers share one canvas-owned svg handle. Every overlay of
    // one selection clones the same handle, so all read the same mount set
    // and key listener however the mounts interleave; a second canvas owns a
    // second handle.
    let control = use_canvas_control(
        &svg,
        matches!(
            tool,
            LayoutTransformTool::Stagger | LayoutTransformTool::Splay | LayoutTransformTool::Origin
        ),
    );
    let _ = surface_size.map(|size| size());
    let svg_handle = svg.clone();
    use_rail_mount(Rail::Matrix, svg_handle.clone(), tool);
    let drag = use_hook(|| Rc::new(RefCell::new(None::<TransformDrag>)));
    let snap_guide = use_signal(|| None::<Vec2>);
    // Ticket 03: hover (or keyboard focus) on a rail token previews, in
    // amber, the keys its drag would move — the same transient view state
    // the Inspector number fields show. It never writes.
    let hovered = use_signal(|| false);
    // Keyboard focus previews like hover, matching the Inspector fields.
    let focused = use_signal(|| false);
    // Ticket 06: the value popup and commit toast content (pure reducer;
    // rendering only), the popup's top-left in world units (`None` docks it
    // above the rail), and the toast generation the 1.8 s timer checks.
    let feedback = use_signal(FeedbackState::default);
    let popup_at = use_signal(|| None::<(f64, f64)>);
    let toast_gen = use_signal(|| 0_u64);
    // Single-ticket commits: drags and nudges queue here and settle on the
    // production `version` signal (durable toast on landing, `report` on
    // failure, silent on retirement). No keyed map or polling loop is kept.
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
    let preview = try_use_context::<FieldPreview>();
    let preview_source = try_use_context::<Option<FieldPreviewSource>>().flatten();
    let shown = use_hook(|| Rc::new(RefCell::new(Vec::<String>::new())));
    let show_preview = {
        let shown = shown.clone();
        let context = context.clone();
        let affect = splay_affect;
        move || {
            let (Some(target), Some(source)) = (preview, preview_source.clone()) else {
                return;
            };
            let Some(field) = preview_field_for_tool(tool, &context) else {
                return;
            };
            let Some(facts) = matrix_relations::field_preview(
                &source.document,
                &source.scene.matrix_scenes,
                &context,
                field,
                affect(),
            ) else {
                return;
            };
            *shown.borrow_mut() = facts.parts.clone();
            target.show(FieldPreviewState {
                parts: facts.parts,
                hint: facts.hint,
            });
        }
    };
    let release_preview = {
        let shown = shown.clone();
        move || {
            if let Some(target) = preview {
                target.release(&shown.borrow());
            }
        }
    };
    use_drop({
        let shown = shown.clone();
        move || {
            if let Some(target) = preview {
                target.release(&shown.borrow());
            }
        }
    });
    // True from a Ctrl pointer-down until release: the handle shows the offset glyph.
    let quick_glyph = use_signal(|| false);
    // True from a Ctrl pointer-down on a Splay or Origin handle until release.
    let swing_glyph = use_signal(|| false);
    let cancel = {
        let runtime = runtime.0.clone();
        let arbiter = arbiter.clone();
        let drag = drag.clone();
        let svg = svg_handle.clone();
        move || {
            cancel_transform_drag(
                &runtime,
                &drag,
                &arbiter,
                snap_guide,
                quick_glyph,
                swing_glyph,
                &svg,
            );
        }
    };
    let escape_drag = use_rail_drag_cancel(
        &drag,
        &owner,
        &context,
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

    let mut selected_points = {
        let model = runtime.0.model();
        model.accepted.as_ref().map_or_else(Vec::new, |snapshot| {
            model
                .selected_part_ids
                .iter()
                .filter_map(|id| {
                    snapshot
                        .document
                        .parts
                        .iter()
                        .find(|part| &part.id == id)
                        .map(|part| part.pose.at)
                })
                .collect::<Vec<_>>()
        })
    };
    match &context {
        TreeContext::Column { column, .. } => {
            selected_points = projection
                .cells
                .iter()
                .filter(|cell| cell.enabled && cell.column == *column)
                .map(|cell| cell.pose.at)
                .collect()
        }
        TreeContext::Row { row, .. } => {
            selected_points = projection
                .cells
                .iter()
                .filter(|cell| cell.enabled && cell.row == *row)
                .map(|cell| cell.pose.at)
                .collect()
        }
        _ => {}
    }
    // The matrix Move/Rotate handles share one anchor point on the drawn
    // matrix: the first enabled key's pose. The rotation center captured at
    // pointer-down uses the same point from the live scene, so the two cannot
    // drift apart.
    let matrix_handle_point = projection
        .cells
        .iter()
        .find(|cell| cell.enabled)
        .or(projection.cells.first())
        .map(|cell| cell.pose.at)
        .unwrap_or(matrix.origin);
    let matrix_rotation_followed = matrix
        .anchor
        .as_ref()
        .map(|anchor| anchor.follow.rotation)
        .unwrap_or(true);
    let selected_column = context_column(&context).unwrap_or(0);
    let handle_scale = handle_scale(&svg, width, height);
    let world_per_screen = handle_scale / 3.0;
    let rail_tools = matrix_rail_tools(&svg_handle, &context);
    let rail_index = rail_tools.iter().position(|peer| *peer == tool);
    // The Origin point stays on the canvas at the splay origin; it never
    // joins the rail.
    let origin_point = if matches!(
        tool,
        LayoutTransformTool::Splay | LayoutTransformTool::Origin
    ) {
        projection
            .columns
            .iter()
            .find(|basis| basis.column == selected_column)
            .map(|basis| basis.splay_origin)
    } else {
        None
    };
    // The rail centres on the selection: the column or row cells for a
    // Column/Row selection, every enabled cell for a Matrix.
    let rail_points: Vec<Vec2> = match &context {
        TreeContext::Column { column, .. } => projection
            .cells
            .iter()
            .filter(|cell| cell.enabled && cell.column == *column)
            .map(|cell| cell.pose.at)
            .collect(),
        TreeContext::Row { row, .. } => projection
            .cells
            .iter()
            .filter(|cell| cell.enabled && cell.row == *row)
            .map(|cell| cell.pose.at)
            .collect(),
        TreeContext::Key { row, column, .. } => projection
            .cells
            .iter()
            .filter(|cell| cell.enabled && cell.row == *row && cell.column == *column)
            .map(|cell| cell.pose.at)
            .collect(),
        TreeContext::Matrix { .. } => projection
            .cells
            .iter()
            .filter(|cell| cell.enabled)
            .map(|cell| cell.pose.at)
            .collect(),
        _ => Vec::new(),
    };
    // A Matrix with no enabled cell still draws its rail at the shared
    // handle point, so Move and Rotate never lose their tokens.
    let rail_points = if rail_points.is_empty() && matches!(context, TreeContext::Matrix { .. }) {
        vec![matrix_handle_point]
    } else {
        rail_points
    };
    let rail_layout =
        rail_index.and_then(|_| rail_over_points(&rail_points, rail_tools.len(), world_per_screen));
    if rail_layout.is_none() && origin_point.is_none() {
        return rsx! {};
    }
    let fallback_angle_point = angle_handle_point(
        &projection,
        selected_column,
        matrix.pitch.y,
        &selected_points,
    );
    let angle_point = if selected_points.is_empty() {
        fallback_angle_point
    } else {
        Vec2 {
            x: selected_points.iter().map(|p| p.x).sum::<f64>() / selected_points.len() as f64,
            y: selected_points
                .iter()
                .map(|p| p.y)
                .fold(f64::INFINITY, f64::min)
                - matrix.pitch.y / 2.0
                - 18.0 * handle_scale,
        }
    };
    let angle_degrees = projection
        .columns
        .iter()
        .find(|basis| basis.column == selected_column)
        .map_or(0.0, |basis| {
            column_angle(basis.splay_angle, basis.rotation, &splay_affect())
        });
    let arc = origin_point.map(|origin| splay_arc(origin, angle_point, angle_degrees));
    let current_owner = owner.clone();
    let current_context = context.clone();
    let matrix_for_key = matrix.clone();
    let offset_parameter = match context_row(&context) {
        Some(row) => PlacementParameter::RowOffset { row },
        None => PlacementParameter::ColumnOffset {
            column: selected_column,
        },
    };
    let offset_now = match placement_current(&matrix, offset_parameter) {
        PlacementValue::Offset(offset) => offset,
        _ => Vec2::default(),
    };
    // Ticket 05: a row or key selection carries its own shift and bend; any
    // other context leaves them absent and the tokens stay hidden.
    let shift_parameter = shift_parameter(&context);
    let shift_now = shift_parameter
        .and_then(|parameter| match placement_current(&matrix, parameter) {
            PlacementValue::Shift(shift) => Some(shift),
            _ => None,
        })
        .unwrap_or_default();
    let bend_parameter = bend_parameter(&context);
    let bend_now = bend_parameter
        .and_then(|parameter| match placement_current(&matrix, parameter) {
            PlacementValue::Bend(bend) => Some(bend),
            _ => None,
        })
        .unwrap_or(0.0);
    let stagger_now = matrix
        .column_layout
        .get(selected_column as usize)
        .map(|record| &record.stagger)
        .copied()
        .unwrap_or(0.0);
    let projection_for_key = projection.clone();
    let projection_for_start = projection.clone();
    let matrix_for_start = matrix.clone();
    let owner_for_key = owner.clone();
    let context_for_key = context.clone();
    let runtime_for_start = runtime.0.clone();
    let svg_for_start = svg.surface().clone();
    let drag_for_start = drag.clone();
    let arbiter_for_start = arbiter.clone();
    let guide_for_start = snap_guide;
    let popup_at_for_start = popup_at;
    let svg_handle_for_start_outer = svg_handle.clone();
    let start_drag = Rc::new(move |event: PointerEvent, gesture: TransformGesture| {
        let mut guide_for_start = guide_for_start;
        let mut quick_for_start = quick_glyph;
        let mut swing_for_start = swing_glyph;
        let svg_handle_for_start = svg_handle_for_start_outer.clone();
        let Some(pointer) = event.data().try_as_web_event() else {
            return;
        };
        // The modifier is read here and nowhere else, so a drag never switches parameter.
        let gesture = match gesture {
            TransformGesture::Stagger { column, .. } => TransformGesture::Stagger {
                column,
                quick_offset: pointer.ctrl_key(),
            },
            TransformGesture::Splay {
                column,
                start_angle,
                origin,
                ..
            } => TransformGesture::Splay {
                column,
                start_angle,
                origin,
                swing: pointer.ctrl_key(),
            },
            TransformGesture::Origin { column, .. } => TransformGesture::Origin {
                column,
                swing: pointer.ctrl_key(),
            },
            TransformGesture::MatrixMove { .. } => {
                // Capture the anchor context now; the preview and the commit
                // compose this drag against whatever the matrix becomes. An
                // anchored matrix whose anchor target is not laid out refuses
                // the drag before it acquires anything.
                let Some(accepted) = runtime_for_start.model().accepted.clone() else {
                    return;
                };
                let Ok(anchor) = matrix_drag_anchor(&accepted, &matrix_for_start) else {
                    return;
                };
                TransformGesture::MatrixMove { anchor }
            }
            TransformGesture::MatrixRotate { .. } => {
                let Some(accepted) = runtime_for_start.model().accepted.clone() else {
                    return;
                };
                let Ok(anchor) = matrix_drag_anchor(&accepted, &matrix_for_start) else {
                    return;
                };
                TransformGesture::MatrixRotate {
                    anchor,
                    center: Vec2::default(),
                    start_angle: 0.0,
                    start_rotation: 0.0,
                }
            }
            other => other,
        };
        let basis = gesture
            .parameter()
            .and_then(|parameter| SceneBasis::for_parameter(&projection_for_start, parameter));
        if pointer.button() != 0 || drag_for_start.borrow().is_some() {
            return;
        }
        if !transform_owner_is_current(
            &runtime_for_start,
            workspace,
            scope_generation,
            selected_context,
            &current_owner,
            &current_context,
        ) || context_matrix_id(&current_context) != Some(matrix_for_start.id.as_str())
            || !transform_start_is_editable(&runtime_for_start, &current_owner, &matrix_for_start)
            || !arbiter_for_start.try_acquire(CanvasInteractionOwner::MatrixTransform)
        {
            return;
        }
        let Some(capture) = capture_rail_pointer(
            &pointer,
            "[role='button'][tabindex='0']",
            &arbiter_for_start,
        ) else {
            return;
        };
        let angle_origin = match gesture {
            TransformGesture::Splay { origin, .. } => origin,
            _ => Vec2::default(),
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
        let start_angle = (start_point.y - angle_origin.y).atan2(start_point.x - angle_origin.x);
        let gesture = match gesture {
            TransformGesture::Splay {
                column,
                origin,
                swing,
                ..
            } => TransformGesture::Splay {
                column,
                origin,
                start_angle,
                swing,
            },
            TransformGesture::MatrixRotate { anchor, .. } => {
                // The rotation center is the shared handle point: the first
                // enabled key's pose from the drag's own scene.
                let center = projection_for_start
                    .cells
                    .iter()
                    .find(|cell| cell.enabled)
                    .or(projection_for_start.cells.first())
                    .map(|cell| cell.pose.at)
                    .unwrap_or(matrix_for_start.origin);
                TransformGesture::MatrixRotate {
                    anchor,
                    center,
                    start_angle: (start_point.y - center.y).atan2(start_point.x - center.x),
                    start_rotation: matrix_for_start.rotation.unwrap_or(0.0),
                }
            }
            other => other,
        };
        let operation = runtime_for_start.operation();
        let swing_basis = match gesture {
            TransformGesture::Splay { column, .. } | TransformGesture::Origin { column, .. } => {
                projection_for_start
                    .columns
                    .iter()
                    .find(|basis| basis.column == column)
            }
            _ => None,
        };
        let origin_base = swing_basis.map(|basis| basis.base);
        let start_origin = swing_basis.map(|basis| basis.splay_origin);
        *drag_for_start.borrow_mut() = Some(TransformDrag {
            rail: RailDrag {
                pointer_id: pointer.pointer_id(),
                capture,
                transaction_id: format!("layout-transform-{}", operation.0),
                owner: current_owner.clone(),
                context: current_context.clone(),
                tool,
                interaction: RailDrag::interaction(pointer.pointer_id()),
            },
            matrix: matrix_for_start.clone(),
            gesture,
            basis,
            origin_base,
            start_origin,
            start_client_x: f64::from(pointer.client_x()),
            start_client_y: f64::from(pointer.client_y()),
        });
        guide_for_start.set(None);
        // Ticket 06: the popup follows the pointer from pointer-down, 18 px
        // off it like the prototype, until the drop re-docks or clears it.
        let mut popup_at_for_start = popup_at_for_start;
        popup_at_for_start.set(Some((
            start_point.x + 18.0 * world_per_screen,
            start_point.y - 18.0 * world_per_screen,
        )));
        set_rail_active(Rail::Matrix, &svg_handle_for_start, Some(tool));
        quick_for_start.set(matches!(
            gesture,
            TransformGesture::Stagger {
                quick_offset: true,
                ..
            }
        ));
        swing_for_start.set(matches!(
            gesture,
            TransformGesture::Splay { swing: true, .. }
                | TransformGesture::Origin { swing: true, .. }
        ));
    });

    let runtime_for_move = runtime.0.clone();
    let svg_for_move = svg.surface().clone();
    let drag_for_move = drag.clone();
    let move_event = drag_event.clone();
    let guide_for_move = snap_guide;
    let feedback_for_move = feedback;
    let popup_at_for_move = popup_at;
    let snap_settings_for_move = snap_settings.clone();
    let snap_origins_for_move = snap_origins.clone();
    let move_drag = Rc::new(move |event: PointerEvent| {
        let mut guide_for_move = guide_for_move;
        let mut feedback_for_move = feedback_for_move;
        let mut popup_at_for_move = popup_at_for_move;
        let Some((pointer, mut active)) = move_event(event, false) else {
            return;
        };
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
        let operation = sample_transform(
            &active,
            &pointer,
            point,
            TransformSampleContext {
                view_width: width,
                view_height: height,
                settings: &snap_settings_for_move,
                origins: &snap_origins_for_move,
                affect: splay_affect(),
                svg: &svg_for_move,
                guide: &mut guide_for_move,
            },
        );
        if let Some(operation) = operation
            && active
                .rail
                .interaction
                .sample(pointer.pointer_id(), &operation)
        {
            active
                .rail
                .submit_preview(&runtime_for_move, &active.matrix.id, operation.clone());
            // Ticket 06: the popup follows the pointer with the live value
            // and delta; a non-placement gesture keeps its docked content.
            if let Some(TransformDragResult::Placement { delta, .. }) =
                transform_drag_result(&active, &operation)
            {
                let value = placement_target(&operation, &active.matrix)
                    .map(|(_, end)| format_placement_value(end))
                    .unwrap_or_default();
                feedback_for_move.set(advance_feedback(
                    &feedback_for_move(),
                    FeedbackEvent::Drag {
                        name: feedback_tool_name(tool).to_owned(),
                        value,
                        delta: placement_feedback_delta(delta),
                    },
                ));
            }
            popup_at_for_move.set(Some((
                point.x + 18.0 * world_per_screen,
                point.y - 18.0 * world_per_screen,
            )));
            *drag_for_move.borrow_mut() = Some(active);
        }
    });

    let runtime_for_end = runtime.0.clone();
    let svg_for_end = svg.surface().clone();
    let drag_for_end = drag.clone();
    let arbiter_for_end = arbiter.clone();
    let guide_for_end = snap_guide;
    let feedback_for_end = feedback;
    let popup_at_for_end = popup_at;
    let pending_for_end = pending;
    let svg_handle_for_end_outer = svg_handle.clone();
    let end_drag = Rc::new(move |event: PointerEvent| {
        let mut guide_for_end = guide_for_end;
        let mut feedback_for_end = feedback_for_end;
        let mut popup_at_for_end = popup_at_for_end;
        let mut pending_for_end = pending_for_end;
        let svg_handle_for_end = svg_handle_for_end_outer.clone();
        let Some((pointer, mut active)) = drag_event(event, true) else {
            return;
        };
        let point =
            crate::presentation::coordinates(&svg_for_end, &pointer, view_x, view_y, width, height);
        let operation = point.and_then(|point| {
            sample_transform(
                &active,
                &pointer,
                point,
                TransformSampleContext {
                    view_width: width,
                    view_height: height,
                    settings: &snap_settings,
                    origins: &snap_origins,
                    affect: splay_affect(),
                    svg: &svg_for_end,
                    guide: &mut guide_for_end,
                },
            )
        });
        if let Some(sample) = active
            .rail
            .interaction
            .finish(pointer.pointer_id(), operation)
        {
            let operation = sample.operation;
            if sample.needs_preview {
                active
                    .rail
                    .submit_preview(&runtime_for_end, &active.matrix.id, operation.clone());
            }
            let live_value = || {
                placement_target(&operation, &active.matrix)
                    .map(|(_, end)| format_placement_value(end))
                    .unwrap_or_default()
            };
            if same_transform_as_start(&active, &operation) {
                active.rail.clear_preview(&runtime_for_end);
                feedback_for_end.set(advance_feedback(
                    &feedback_for_end(),
                    FeedbackEvent::Choose {
                        name: feedback_tool_name(tool).to_owned(),
                        value: live_value(),
                    },
                ));
            } else if let Some(result) = transform_drag_result(&active, &operation) {
                // Queued drags compose through the resolver (ADR-0005).
                let toast = match &result {
                    TransformDragResult::Placement { delta, .. } => Some(PendingTransformToast {
                        tool_name: feedback_tool_name(tool),
                        value: live_value(),
                        delta: placement_feedback_delta(*delta),
                    }),
                    _ => None,
                };
                let ticket = runtime_for_end.resolve_edit(
                    "layout-transform-drag",
                    Some("transform".into()),
                    transform_drag_resolver(
                        active.matrix.id.clone(),
                        active.rail.transaction_id.clone(),
                        result,
                    ),
                );
                pending_for_end.write().push(PendingTransformCommit {
                    ticket,
                    owner: active.rail.owner.clone(),
                    context: active.rail.context.clone(),
                    toast,
                });
            }
        }
        popup_at_for_end.set(None);
        guide_for_end.set(None);
        set_rail_active(Rail::Matrix, &svg_handle_for_end, None);
        let mut quick_for_end = quick_glyph;
        quick_for_end.set(false);
        let mut swing_for_end = swing_glyph;
        swing_for_end.set(false);
        // Releasing capture fires `lostpointercapture`; retire the drag before it can cancel the drop.
        drag_for_end.borrow_mut().take();
        active.rail.release(&arbiter_for_end);
    });

    let cancel_drag = {
        let drag = drag.clone();
        Rc::new(move |event: PointerEvent| {
            cancel_rail_pointer(event, &drag, feedback, popup_at, &cancel);
        })
    };

    let runtime_for_key = runtime.0.clone();
    let on_key_down = Rc::new(move |event: KeyboardEvent| {
        let mut pending = pending;
        let Some(key) = event.data().try_as_web_event() else {
            return;
        };
        if key.key() == "Escape" {
            key.prevent_default();
            key.stop_propagation();
            escape_drag();
            return;
        }
        if !matches!(
            key.key().as_str(),
            "ArrowLeft" | "ArrowRight" | "ArrowUp" | "ArrowDown"
        ) || !transform_nudge_admitted(
            &runtime_for_key,
            workspace,
            scope_generation,
            selected_context,
            &owner_for_key,
            &context_for_key,
        ) {
            return;
        }
        let Some(target) = key
            .target()
            .and_then(|target| target.dyn_into::<web_sys::Element>().ok())
            .and_then(|target| {
                target
                    .closest("[role='button'][tabindex='0']")
                    .ok()
                    .flatten()
            })
        else {
            return;
        };
        let step = if key.shift_key() { 1.0 } else { 0.1 };
        let delta = Vec2 {
            x: match key.key().as_str() {
                "ArrowLeft" => -step,
                "ArrowRight" => step,
                _ => 0.0,
            },
            y: match key.key().as_str() {
                "ArrowDown" => -step,
                "ArrowUp" => step,
                _ => 0.0,
            },
        };
        let arrow = match key.key().as_str() {
            "ArrowLeft" => ArrowKey::Left,
            "ArrowRight" => ArrowKey::Right,
            "ArrowUp" => ArrowKey::Up,
            _ => ArrowKey::Down,
        };
        let nudge = match target.get_attribute("aria-label").as_deref() {
            Some("Drag to spread") => {
                let parameter = PlacementParameter::ColumnSpread {
                    column: selected_column,
                };
                let big = key.shift_key();
                // The pair across the spread axis is not consumed; column 0 has
                // no Spread handle, so its basis never reaches this mapping.
                let Some(basis) = SceneBasis::for_parameter(&projection_for_key, parameter) else {
                    return;
                };
                if nudge_value(
                    &basis,
                    parameter,
                    placement_current(&matrix_for_key, parameter),
                    arrow,
                    big,
                )
                .is_none()
                {
                    return;
                }
                TransformNudge::Spread {
                    column: selected_column,
                    key: arrow,
                    big,
                }
            }
            Some("Drag to stagger") => {
                let parameter = PlacementParameter::Stagger {
                    column: selected_column,
                };
                let big = key.shift_key();
                // The pair across the stagger axis is not consumed.
                let Some(basis) = SceneBasis::for_parameter(&projection_for_key, parameter) else {
                    return;
                };
                if context_row(&context_for_key).is_some()
                    || nudge_value(
                        &basis,
                        parameter,
                        placement_current(&matrix_for_key, parameter),
                        arrow,
                        big,
                    )
                    .is_none()
                {
                    return;
                }
                TransformNudge::Stagger {
                    column: selected_column,
                    key: arrow,
                    big,
                }
            }
            Some(label @ ("Drag to offset row" | "Drag to offset column")) => {
                let parameter = match (label, context_row(&context_for_key)) {
                    ("Drag to offset row", Some(row)) => PlacementParameter::RowOffset { row },
                    ("Drag to offset column", None) => PlacementParameter::ColumnOffset {
                        column: selected_column,
                    },
                    _ => return,
                };
                TransformNudge::Offset {
                    parameter,
                    key: arrow,
                    big: key.shift_key(),
                }
            }
            // Ticket 05: Shift nudges step through the carried-shift frame
            // like an offset; Bend nudges turn the running frame by degrees,
            // mirroring the Splay angle nudge (Shift steps five).
            Some("Drag to shift row") => {
                let Some(row) = context_row(&context_for_key) else {
                    return;
                };
                TransformNudge::Offset {
                    parameter: PlacementParameter::RowShift { row },
                    key: arrow,
                    big: key.shift_key(),
                }
            }
            Some("Drag to shift key") => {
                let Some((row, column)) = context_key(&context_for_key) else {
                    return;
                };
                TransformNudge::Offset {
                    parameter: PlacementParameter::KeyShift { row, column },
                    key: arrow,
                    big: key.shift_key(),
                }
            }
            Some(label @ ("Drag to bend row" | "Drag to bend key")) if delta.x != 0.0 => {
                let parameter = match (label, &context_for_key) {
                    ("Drag to bend row", TreeContext::Row { row, .. }) => {
                        PlacementParameter::RowBend { row: *row }
                    }
                    ("Drag to bend key", TreeContext::Key { row, column, .. }) => {
                        PlacementParameter::KeyBend {
                            row: *row,
                            column: *column,
                        }
                    }
                    _ => return,
                };
                TransformNudge::Bend {
                    parameter,
                    degrees: delta.x.signum() * if key.shift_key() { 5.0 } else { 1.0 },
                }
            }
            Some("Move splay origin") => TransformNudge::SplayOrigin {
                column: selected_column,
            },
            Some("Drag to splay") if delta.x != 0.0 => TransformNudge::SplayAngle {
                column: selected_column,
                degrees: delta.x.signum() * if key.shift_key() { 5.0 } else { 1.0 },
                affect: splay_affect(),
            },
            Some("Drag to move matrix") => TransformNudge::MatrixMove,
            Some("Drag to rotate matrix") if delta.x != 0.0 => TransformNudge::MatrixRotate {
                degrees: delta.x.signum() * if key.shift_key() { 5.0 } else { 1.0 },
            },
            _ => return,
        };
        key.prevent_default();
        key.stop_propagation();
        // A held key queues one step per repeat; each resolves against the matrix the
        // previous steps produced (ADR-0005). The ticket settles on `version`:
        // failures report, retirement is silent, focus stays on the token.
        let resolver = transform_nudge_resolver(matrix_for_key.id.clone(), nudge, delta);
        let ticket = runtime_for_key.resolve_edit(
            "layout-transform-nudge",
            Some("transform".into()),
            resolver,
        );
        pending.write().push(PendingTransformCommit {
            ticket,
            owner: owner_for_key.clone(),
            context: context_for_key.clone(),
            toast: None,
        });
    });

    let gesture = match tool {
        LayoutTransformTool::Stagger => TransformGesture::Stagger {
            column: selected_column,
            quick_offset: false,
        },
        LayoutTransformTool::Spread => TransformGesture::Spread {
            column: selected_column,
        },
        LayoutTransformTool::Offset => TransformGesture::Offset {
            parameter: offset_parameter,
        },
        // A Shift drag edits the row's or key's carried shift through the
        // Offset gesture's shift axes; a Bend drag turns the running frame.
        // Without a row or key the tokens stay hidden, so the offsets below
        // never start a drag.
        LayoutTransformTool::Shift => TransformGesture::Offset {
            parameter: shift_parameter.unwrap_or(offset_parameter),
        },
        LayoutTransformTool::Bend => TransformGesture::Bend {
            parameter: bend_parameter.unwrap_or(offset_parameter),
        },
        LayoutTransformTool::Splay | LayoutTransformTool::Origin => TransformGesture::Origin {
            column: selected_column,
            swing: false,
        },
        LayoutTransformTool::Move => TransformGesture::MatrixMove { anchor: None },
        LayoutTransformTool::Rotate => TransformGesture::MatrixRotate {
            anchor: None,
            center: Vec2::default(),
            start_angle: 0.0,
            start_rotation: 0.0,
        },
        // Ticket 08: the Anchor token never mounts this drag overlay (the
        // pick overlay owns it), so no gesture starts here.
        LayoutTransformTool::Anchor => TransformGesture::MatrixMove { anchor: None },
    };
    matrix_token_chrome(MatrixTokenView {
        tool,
        context: context.clone(),
        projection: projection.clone(),
        matrix: matrix.clone(),
        selected_column,
        matrix_rotation_followed,
        offset_now,
        shift_now,
        bend_now,
        stagger_now,
        control,
        quick_glyph,
        swing_glyph,
        hovered,
        focused,
        feedback,
        popup_at,
        snap_guide,
        drag: drag.clone(),
        svg: svg_handle.clone(),
        rail_index,
        rail_layout,
        rail_tools,
        world_per_screen,
        angle_degrees,
        origin_point,
        arc,
        angle_point,
        shift_parameter,
        bend_parameter,
        gesture,
        start_drag,
        move_drag,
        end_drag,
        cancel_drag,
        on_key_down,
        show_preview,
        release_preview,
        view_x,
        view_y,
        width,
    })
}
