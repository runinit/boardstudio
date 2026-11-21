//! The matrix rail overlay: one token (or the splay origin point) per mounted tool.
use super::*;

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
    let control = use_signal(|| false);
    let _control_listener = use_hook(move || {
        Rc::new(
            if matches!(
                tool,
                LayoutTransformTool::Stagger
                    | LayoutTransformTool::Splay
                    | LayoutTransformTool::Origin
            ) {
                CtrlModifierListener::install(control)
            } else {
                None
            },
        )
    });
    let _ = surface_size.map(|size| size());
    // Ticket 03: the rail peers share one svg handle per canvas. The key is
    // the shared handle's identity, so every overlay of one selection reads
    // the same mount set however the mounts interleave.
    let svg_key: usize = Rc::as_ptr(&svg.0) as usize;
    use_rail_mount(Rail::Matrix, svg_key, tool);
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
    let runtime_inner = runtime.0.clone();
    let arbiter_for_drop = arbiter.clone();
    let drag_for_drop = drag.clone();
    let mut guide_for_drop = snap_guide;
    use_drop(move || {
        cancel_transform_drag(
            &runtime_inner,
            &drag_for_drop,
            &arbiter_for_drop,
            &mut guide_for_drop,
            quick_glyph,
            swing_glyph,
            svg_key,
        );
    });
    use_document_escape({
        let runtime = runtime.0.clone();
        let drag = drag.clone();
        let arbiter = arbiter.clone();
        let mut feedback = feedback;
        let mut popup_at = popup_at;
        let mut guide = snap_guide;
        move |event| {
            // Another visible handle may own the live gesture. Its listener cancels it.
            if escape_targets_text_entry(event)
                || (drag.borrow().is_none() && arbiter.current().is_some())
            {
                return;
            }
            event.prevent_default();
            // Ticket 06: Escape clears the chosen tool and its popup.
            feedback.set(advance_feedback(&feedback(), FeedbackEvent::Dismiss));
            popup_at.set(None);
            cancel_transform_drag(
                &runtime,
                &drag,
                &arbiter,
                &mut guide,
                quick_glyph,
                swing_glyph,
                svg_key,
            );
            on_finish.call(());
        }
    });
    use_stale_drag_cancel(
        &drag,
        &owner,
        &context,
        tool,
        selected_context,
        scope_generation,
        {
            let runtime = runtime.0.clone();
            let drag = drag.clone();
            let arbiter = arbiter.clone();
            let mut guide = snap_guide;
            move || {
                cancel_transform_drag(
                    &runtime,
                    &drag,
                    &arbiter,
                    &mut guide,
                    quick_glyph,
                    swing_glyph,
                    svg_key,
                )
            }
        },
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
    let rail_tools = matrix_rail_tools(svg_key, &context);
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
    let matrix_for_spread = matrix.clone();
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
    let svg_for_start = svg.0.clone();
    let drag_for_start = drag.clone();
    let arbiter_for_start = arbiter.clone();
    let guide_for_start = snap_guide;
    let popup_at_for_start = popup_at;
    let start_drag = Rc::new(move |event: PointerEvent, gesture: TransformGesture| {
        let mut guide_for_start = guide_for_start;
        let mut quick_for_start = quick_glyph;
        let mut swing_for_start = swing_glyph;
        let svg_key_for_start = svg_key;
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
        let Some(capture) = pointer
            .target()
            .and_then(|target| target.dyn_into::<web_sys::Element>().ok())
            .and_then(|target| {
                target
                    .closest("[role='button'][tabindex='0']")
                    .ok()
                    .flatten()
            })
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
            pointer_id: pointer.pointer_id(),
            capture,
            matrix: matrix_for_start.clone(),
            gesture,
            transaction_id: format!("layout-transform-{}", operation.0),
            owner: current_owner.clone(),
            context: current_context.clone(),
            tool,
            basis,
            origin_base,
            start_origin,
            start_client_x: f64::from(pointer.client_x()),
            start_client_y: f64::from(pointer.client_y()),
            pending: None,
            preview_submitted: false,
        });
        guide_for_start.set(None);
        // Ticket 06: the popup follows the pointer from pointer-down, 18 px
        // off it like the prototype, until the drop re-docks or clears it.
        let mut popup_at_for_start = popup_at_for_start;
        popup_at_for_start.set(Some((
            start_point.x + 18.0 * world_per_screen,
            start_point.y - 18.0 * world_per_screen,
        )));
        Rail::Matrix.set_active(svg_key_for_start, Some(tool));
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
    let svg_for_move = svg.0.clone();
    let drag_for_move = drag.clone();
    let arbiter_for_move = arbiter.clone();
    let guide_for_move = snap_guide;
    let feedback_for_move = feedback;
    let popup_at_for_move = popup_at;
    let snap_settings_for_move = snap_settings.clone();
    let snap_origins_for_move = snap_origins.clone();
    let move_drag = Rc::new(move |event: PointerEvent| {
        let mut guide_for_move = guide_for_move;
        let mut feedback_for_move = feedback_for_move;
        let mut popup_at_for_move = popup_at_for_move;
        let Some(pointer) = event.data().try_as_web_event() else {
            return;
        };
        let Some(mut active) = drag_for_move
            .borrow()
            .clone()
            .filter(|drag| drag.pointer_id == pointer.pointer_id())
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
            cancel_transform_drag(
                &runtime_for_move,
                &drag_for_move,
                &arbiter_for_move,
                &mut guide_for_move,
                quick_glyph,
                swing_glyph,
                svg_key,
            );
            return;
        }
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
            && active.pending.as_ref() != Some(&operation)
        {
            submit_transform_edit(&runtime_for_move, &active, operation.clone());
            active.preview_submitted = true;
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
            active.pending = Some(operation);
            *drag_for_move.borrow_mut() = Some(active);
        }
    });

    let runtime_for_end = runtime.0.clone();
    let svg_for_end = svg.0.clone();
    let drag_for_end = drag.clone();
    let arbiter_for_end = arbiter.clone();
    let guide_for_end = snap_guide;
    let feedback_for_end = feedback;
    let popup_at_for_end = popup_at;
    let toast_gen_for_end = toast_gen;
    let end_drag = Rc::new(move |event: PointerEvent| {
        let mut guide_for_end = guide_for_end;
        let mut feedback_for_end = feedback_for_end;
        let mut popup_at_for_end = popup_at_for_end;
        let mut toast_gen_for_end = toast_gen_for_end;
        let svg_key_for_end = svg_key;
        let Some(pointer) = event.data().try_as_web_event() else {
            return;
        };
        let Some(mut active) = drag_for_end
            .borrow()
            .clone()
            .filter(|drag| drag.pointer_id == pointer.pointer_id())
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
            cancel_transform_drag(
                &runtime_for_end,
                &drag_for_end,
                &arbiter_for_end,
                &mut guide_for_end,
                quick_glyph,
                swing_glyph,
                svg_key,
            );
            return;
        }
        if pointer.type_() != "pointerup" {
            cancel_transform_drag(
                &runtime_for_end,
                &drag_for_end,
                &arbiter_for_end,
                &mut guide_for_end,
                quick_glyph,
                swing_glyph,
                svg_key,
            );
            return;
        }
        let point =
            crate::presentation::coordinates(&svg_for_end, &pointer, view_x, view_y, width, height);
        let operation = point
            .and_then(|point| {
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
            })
            .or_else(|| active.pending.clone());
        // The live value behind the toast and the re-docked popup.
        let live_value = operation
            .as_ref()
            .and_then(|operation| placement_target(operation, &active.matrix))
            .map(|(_, end)| format_placement_value(end))
            .unwrap_or_default();
        if let Some(operation) = operation {
            if active.pending.as_ref() != Some(&operation) {
                submit_transform_edit(&runtime_for_end, &active, operation.clone());
                active.preview_submitted = true;
            }
            if !same_transform_as_start(&active, &operation) {
                commit_transform_drag(&runtime_for_end, &active, &operation);
                // Ticket 06: a committed placement drag raises the value
                // toast as one Undo step. The toast writes nothing: it is a
                // signal the 1.8 s timer clears.
                if let Some(TransformDragResult::Placement { delta, .. }) =
                    transform_drag_result(&active, &operation)
                {
                    feedback_for_end.set(advance_feedback(
                        &feedback_for_end(),
                        FeedbackEvent::Commit {
                            value: format!("{} {live_value}", feedback_tool_name(tool)),
                            delta: placement_feedback_delta(delta),
                        },
                    ));
                    let seen = {
                        toast_gen_for_end.set(toast_gen_for_end() + 1);
                        toast_gen_for_end()
                    };
                    arm_commit_toast_expiry(feedback_for_end, toast_gen_for_end, seen);
                }
            } else if active.preview_submitted {
                clear_transform_preview(&runtime_for_end, &active);
                feedback_for_end.set(advance_feedback(
                    &feedback_for_end(),
                    FeedbackEvent::Choose {
                        name: feedback_tool_name(tool).to_owned(),
                        value: live_value.clone(),
                    },
                ));
            }
        } else if active.preview_submitted {
            clear_transform_preview(&runtime_for_end, &active);
            feedback_for_end.set(advance_feedback(
                &feedback_for_end(),
                FeedbackEvent::Choose {
                    name: feedback_tool_name(tool).to_owned(),
                    value: live_value,
                },
            ));
        }
        popup_at_for_end.set(None);
        guide_for_end.set(None);
        Rail::Matrix.set_active(svg_key_for_end, None);
        let mut quick_for_end = quick_glyph;
        quick_for_end.set(false);
        let mut swing_for_end = swing_glyph;
        swing_for_end.set(false);
        // Releasing capture fires `lostpointercapture`; retire the drag before it can cancel the drop.
        drag_for_end.borrow_mut().take();
        if active.capture.has_pointer_capture(active.pointer_id) {
            let _ = active.capture.release_pointer_capture(active.pointer_id);
        }
        arbiter_for_end.release(CanvasInteractionOwner::MatrixTransform);
    });

    let runtime_for_cancel = runtime.0.clone();
    let drag_for_cancel = drag.clone();
    let arbiter_for_cancel = arbiter.clone();
    let guide_for_cancel = snap_guide;
    let feedback_for_cancel = feedback;
    let popup_at_for_cancel = popup_at;
    let cancel_drag = Rc::new(move |event: PointerEvent| {
        let mut guide_for_cancel = guide_for_cancel;
        let mut feedback_for_cancel = feedback_for_cancel;
        let mut popup_at_for_cancel = popup_at_for_cancel;
        let Some(pointer) = event.data().try_as_web_event() else {
            return;
        };
        if drag_for_cancel
            .borrow()
            .as_ref()
            .is_some_and(|drag| drag.pointer_id == pointer.pointer_id())
        {
            pointer.prevent_default();
            pointer.stop_propagation();
            // Ticket 06: a cancelled drag takes its popup with it.
            feedback_for_cancel.set(advance_feedback(
                &feedback_for_cancel(),
                FeedbackEvent::Dismiss,
            ));
            popup_at_for_cancel.set(None);
            cancel_transform_drag(
                &runtime_for_cancel,
                &drag_for_cancel,
                &arbiter_for_cancel,
                &mut guide_for_cancel,
                quick_glyph,
                swing_glyph,
                svg_key,
            );
        }
    });

    let runtime_for_key = runtime.0.clone();
    let drag_for_key = drag.clone();
    let arbiter_for_key = arbiter.clone();
    let guide_for_key = snap_guide;
    let feedback_for_key = feedback;
    let popup_at_for_key = popup_at;
    let on_key_down = Rc::new(move |event: KeyboardEvent| {
        let mut guide_for_key = guide_for_key;
        let mut feedback_for_key = feedback_for_key;
        let mut popup_at_for_key = popup_at_for_key;
        let Some(key) = event.data().try_as_web_event() else {
            return;
        };
        if key.key() == "Escape" {
            key.prevent_default();
            key.stop_propagation();
            // Ticket 06: Escape clears the chosen tool and its popup.
            feedback_for_key.set(advance_feedback(
                &feedback_for_key(),
                FeedbackEvent::Dismiss,
            ));
            popup_at_for_key.set(None);
            cancel_transform_drag(
                &runtime_for_key,
                &drag_for_key,
                &arbiter_for_key,
                &mut guide_for_key,
                quick_glyph,
                swing_glyph,
                svg_key,
            );
            on_finish.call(());
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
        // previous steps produced.
        let resolver = transform_nudge_resolver(matrix_for_key.id.clone(), nudge, delta);
        let mut edits = PendingEdits::default();
        edits.begin(
            &runtime_for_key,
            (),
            "layout-transform-nudge",
            Some("transform".into()),
            resolver,
        );
        let runtime = runtime_for_key.clone();
        let owner = owner_for_key.clone();
        let context = context_for_key.clone();
        spawn_local(async move {
            loop {
                let is_live = transform_nudge_admitted(
                    &runtime,
                    workspace,
                    scope_generation,
                    selected_context,
                    &owner,
                    &context,
                );
                if !edits.settle(is_live).is_empty() {
                    break;
                }
                gloo_timers::future::TimeoutFuture::new(16).await;
            }
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
    let stagger_axis = projection
        .columns
        .iter()
        .find(|basis| basis.column == selected_column)
        .map_or(Vec2 { x: 0.0, y: 1.0 }, |basis| basis.stagger_axis);
    let spread_now = match placement_current(
        &matrix_for_spread,
        PlacementParameter::ColumnSpread {
            column: selected_column,
        },
    ) {
        PlacementValue::Spread(value) => value,
        _ => matrix_for_spread.pitch.x,
    };
    let handle_label = match (tool, &context) {
        (LayoutTransformTool::Stagger, _) if control() || quick_glyph() => "Drag to offset column",
        (LayoutTransformTool::Offset, TreeContext::Row { .. }) => "Drag to offset row",
        (LayoutTransformTool::Offset, _) => "Drag to offset column",
        (LayoutTransformTool::Shift, TreeContext::Row { .. }) => "Drag to shift row",
        (LayoutTransformTool::Shift, TreeContext::Key { .. }) => "Drag to shift key",
        (LayoutTransformTool::Shift, _) => "Drag to shift",
        (LayoutTransformTool::Bend, TreeContext::Row { .. }) => "Drag to bend row",
        (LayoutTransformTool::Bend, TreeContext::Key { .. }) => "Drag to bend key",
        (LayoutTransformTool::Bend, _) => "Drag to bend",
        (LayoutTransformTool::Spread, _) => "Drag to spread",
        _ => "Drag to stagger",
    };
    let handle_value = match tool {
        LayoutTransformTool::Offset => {
            format!("X {:.2} mm, Y {:.2} mm", offset_now.x, offset_now.y)
        }
        LayoutTransformTool::Shift => {
            format!("X {:.2} mm, Y {:.2} mm", shift_now.x, shift_now.y)
        }
        LayoutTransformTool::Bend => format!("{bend_now:.1}°"),
        LayoutTransformTool::Spread => format!("{spread_now:.2} mm"),
        _ if control() || quick_glyph() => {
            format!("X {:.2} mm, Y {:.2} mm", offset_now.x, offset_now.y)
        }
        _ => format!("{stagger_now:.2} mm"),
    };
    let quick_offset_shown = quick_glyph() || control() || tool == LayoutTransformTool::Offset;
    let active_peer = Rail::Matrix.active(svg_key);
    let dragging_here = drag.borrow().is_some();
    // A live drag on a peer dims this token to 35%; the active token keeps
    // full opacity. Siblings re-read the shared mount set whenever the
    // Editor re-renders, which every preview submit triggers.
    let rail_dimmed = active_peer.is_some_and(|peer| peer != tool) && rail_index.is_some();
    let token_active = dragging_here;
    let token_lit = hovered() || focused() || token_active;
    // Screen-frame glyph rotation for the column axis: the px groups below
    // are y-down, so the world axis appears mirrored vertically.
    let glyph_degrees = (-stagger_axis.y).atan2(stagger_axis.x).to_degrees();
    let (token_fill, glyph_stroke) = token_paint(token_active, hovered() || focused());
    let token_opacity = if rail_dimmed { "0.35" } else { "1" };
    let guide_opacity = if token_active { "0.8" } else { "0.35" };
    // Splay, Move and Rotate keep hardcoded labels: the gesture distinguishes
    // them, and the mounted drag tests query these strings.
    let token_aria_label: String = match (tool, &context) {
        (LayoutTransformTool::Splay, _) => "Drag to splay".to_owned(),
        (LayoutTransformTool::Move, _) => "Drag to move matrix".to_owned(),
        (LayoutTransformTool::Rotate, _) => "Drag to rotate matrix".to_owned(),
        (LayoutTransformTool::Anchor, _) => "Pick anchor".to_owned(),
        (LayoutTransformTool::Origin, _) => "Move splay origin".to_owned(),
        (LayoutTransformTool::Shift, TreeContext::Row { .. }) => "Drag to shift row".to_owned(),
        (LayoutTransformTool::Shift, TreeContext::Key { .. }) => "Drag to shift key".to_owned(),
        (LayoutTransformTool::Shift, _) => "Drag to shift".to_owned(),
        (LayoutTransformTool::Bend, TreeContext::Row { .. }) => "Drag to bend row".to_owned(),
        (LayoutTransformTool::Bend, TreeContext::Key { .. }) => "Drag to bend key".to_owned(),
        (LayoutTransformTool::Bend, _) => "Drag to bend".to_owned(),
        _ => handle_label.to_owned(),
    };
    let token_description: String = match tool {
        LayoutTransformTool::Spread => handle_value.clone(),
        LayoutTransformTool::Shift | LayoutTransformTool::Bend => handle_value.clone(),
        LayoutTransformTool::Stagger | LayoutTransformTool::Offset => {
            format!("{handle_value}. Hold Ctrl while dragging to offset the column.")
        }
        LayoutTransformTool::Splay => {
            "Hold Ctrl while dragging to swing the origin instead.".to_owned()
        }
        LayoutTransformTool::Move => {
            format!("X {:.2} mm, Y {:.2} mm", matrix.origin.x, matrix.origin.y)
        }
        LayoutTransformTool::Rotate => {
            if matrix_rotation_followed {
                format!("{:.2} degrees", matrix.rotation.unwrap_or(0.0))
            } else {
                "Follow rotation is off, so turning changes nothing.".to_owned()
            }
        }
        LayoutTransformTool::Origin => {
            "Hold Ctrl while dragging to swing the origin without moving the keys.".to_owned()
        }
        LayoutTransformTool::Anchor => {
            "Click a violet ring to anchor here. Esc leaves pick mode.".to_owned()
        }
    };
    let token_title: Option<String> = match tool {
        LayoutTransformTool::Spread => Some(format!("Drag to spread · {handle_value}")),
        LayoutTransformTool::Splay => Some(format!("Drag to splay · {angle_degrees:.0}°")),
        LayoutTransformTool::Move => Some(format!(
            "Drag to move matrix · X {:.2} mm, Y {:.2} mm",
            matrix.origin.x, matrix.origin.y
        )),
        LayoutTransformTool::Rotate if matrix_rotation_followed => Some(format!(
            "Drag to rotate matrix · {:.2} degrees",
            matrix.rotation.unwrap_or(0.0)
        )),
        LayoutTransformTool::Rotate => {
            Some("Rotate matrix (unavailable: Follow rotation is off)".to_owned())
        }
        LayoutTransformTool::Origin => Some("Move splay origin".to_owned()),
        LayoutTransformTool::Anchor => Some("Pick anchor".to_owned()),
        LayoutTransformTool::Stagger
        | LayoutTransformTool::Offset
        | LayoutTransformTool::Shift
        | LayoutTransformTool::Bend => None,
    };
    // The token centre in world units, when this overlay draws a rail token.
    // Splay also needs its origin basis: without it the drag has no frame.
    let token_center: Option<(f64, f64)> = rail_index.and_then(|index| {
        rail_layout
            .as_ref()
            .and_then(|layout| layout.token_centers.get(index).copied())
    });
    // Shift and Bend need their row or key parameter: on any other context
    // the token stays hidden rather than editing the wrong field.
    let token_shown = token_center.is_some()
        && (tool != LayoutTransformTool::Splay || origin_point.is_some())
        && (tool != LayoutTransformTool::Shift || shift_parameter.is_some())
        && (tool != LayoutTransformTool::Bend || bend_parameter.is_some());
    // The axis guide's direction for the one-dimensional tokens, in world units.
    let guide_axis = match tool {
        LayoutTransformTool::Spread => (1.0, 0.0),
        _ => (stagger_axis.x, stagger_axis.y),
    };
    let px = world_per_screen;
    // The token drags its own gesture: Splay needs the origin frame the
    // shared `gesture` leaves to the Origin handle.
    let token_gesture = if tool == LayoutTransformTool::Splay {
        origin_point
            .map(|origin| TransformGesture::Splay {
                column: selected_column,
                start_angle: 0.0,
                origin,
                swing: false,
            })
            .unwrap_or(gesture)
    } else {
        gesture
    };
    // Ticket 06: the popup and toast content behind the chrome below.
    let feedback_state = feedback();
    rsx! {
        if let Some(guide) = snap_guide() {
            g { class: "pointer-events-none fill-none stroke-primary stroke-[0.45] [&_circle]:fill-card", "aria-hidden": "true",
                circle { cx: "{guide.x}", cy: "{guide.y}", r: "{width / 170.0}" }
            }
        }
        if token_shown {
            {
                let (tok_x, tok_y) = token_center.unwrap_or((0.0, 0.0));
                let token_style =
                    format!("cursor:grab;opacity:{token_opacity};touch-action:none");
                let glyph = rail_token_glyph(tool, quick_offset_shown, glyph_degrees, glyph_stroke);
                let face = token_face(token_fill, glyph, token_title.as_deref());
                let show_hover = show_preview.clone();
                let hide_hover = release_preview.clone();
                let show_focus = show_preview.clone();
                let hide_focus = release_preview.clone();
                rsx! {
                    if let (Some(0), Some(layout)) = (rail_index, rail_layout.as_ref()) {
                        {rail_backdrop(layout, rail_tools.len(), px)}
                    }
                    if tool == LayoutTransformTool::Rotate && !matrix_rotation_followed {
                        g {
                            "data-transform-handle": "true", class: "outline-none focus-visible:stroke-foreground",
                            transform: "translate({tok_x} {tok_y}) scale({px} {-px})",
                            role: "button", tabindex: "0", "aria-label": "{token_aria_label}",
                            "aria-disabled": "true",
                            "aria-description": "{token_description}",
                            style: "{token_style}",
                            {face}
                        }
                    } else {
                        g {
                            "data-transform-handle": "true", class: "outline-none focus-visible:stroke-foreground",
                            transform: "translate({tok_x} {tok_y}) scale({px} {-px})",
                            role: "button", tabindex: "0", "aria-label": "{token_aria_label}",
                            "aria-description": "{token_description}",
                            "data-quick-offset": "{quick_offset_shown}",
                            "data-swing": "{swing_glyph()}",
                            style: "{token_style}",
                            onpointerdown: { let start = start_drag.clone(); move |event| start(event, token_gesture) },
                            onpointermove: { let handler = move_drag.clone(); move |event| handler(event) },
                            onpointerup: { let handler = end_drag.clone(); move |event| handler(event) },
                            onpointercancel: { let handler = cancel_drag.clone(); move |event| handler(event) },
                            onlostpointercapture: { let handler = cancel_drag.clone(); move |event| handler(event) },
                            onkeydown: { let handler = on_key_down.clone(); move |event| handler(event) },
                            // Ticket 06: hovering or focusing a token docks the
                            // value popup above the rail; leaving clears it.
                            // The content comes from the pure reducer, so the
                            // chrome never writes.
                            onpointerenter: { let mut hovered = hovered; let mut feedback = feedback; let mut popup_at = popup_at; let name = feedback_tool_name(tool).to_owned(); let value = handle_value.clone(); move |_| { hovered.set(true); show_hover(); popup_at.set(None); feedback.set(advance_feedback(&feedback(), FeedbackEvent::Choose { name: name.clone(), value: value.clone() })); } },
                            onpointerleave: { let mut hovered = hovered; let mut feedback = feedback; move |_| { hovered.set(false); hide_hover(); feedback.set(advance_feedback(&feedback(), FeedbackEvent::Dismiss)); } },
                            onfocus: { let mut focused = focused; let mut feedback = feedback; let mut popup_at = popup_at; let name = feedback_tool_name(tool).to_owned(); let value = handle_value.clone(); move |_| { focused.set(true); show_focus(); popup_at.set(None); feedback.set(advance_feedback(&feedback(), FeedbackEvent::Choose { name: name.clone(), value: value.clone() })); } },
                            onblur: { let mut focused = focused; let mut feedback = feedback; move |_| { focused.set(false); hide_focus(); feedback.set(advance_feedback(&feedback(), FeedbackEvent::Dismiss)); } },
                            {face}
                        }
                    }
                    if token_lit
                        && matches!(
                            tool,
                            LayoutTransformTool::Spread
                                | LayoutTransformTool::Stagger
                                | LayoutTransformTool::Offset
                        )
                    {
                        {axis_guide((tok_x, tok_y), guide_axis, world_per_screen, guide_opacity)}
                    }
                }
            }
        }
        if tool == LayoutTransformTool::Splay && token_lit {
            if let Some(origin) = origin_point {
                if let Some(arc_path) = arc.as_ref() {
                    path {
                        class: "pointer-events-none",
                        d: "M{origin.x} {origin.y} L{angle_point.x} {angle_point.y}",
                        style: "fill:none;stroke:var(--primary);stroke-width:1px;stroke-dasharray:3 3;opacity:{guide_opacity};pointer-events:none;vector-effect:non-scaling-stroke",
                    }
                    path {
                        class: "pointer-events-none",
                        d: "{arc_path}",
                        style: "fill:none;stroke:var(--primary);stroke-width:1px;stroke-dasharray:3 3;opacity:{guide_opacity};pointer-events:none;vector-effect:non-scaling-stroke",
                    }
                }
            }
        }
        if matches!(tool, LayoutTransformTool::Splay | LayoutTransformTool::Origin) {
            if let Some(origin) = origin_point {
                g {
                    "data-transform-handle": "true", class: "outline-none focus-visible:stroke-foreground",
                    transform: "translate({origin.x} {origin.y}) scale({px} {-px})",
                    role: "button", tabindex: "0", "aria-label": "Move splay origin",
                    "aria-description": "Hold Ctrl while dragging to swing the origin without moving the keys.",
                    "data-swing": "{swing_glyph()}",
                    style: "cursor:grab;touch-action:none",
                    onpointerdown: { let start = start_drag.clone(); move |event| start(event, gesture) },
                    onpointermove: { let handler = move_drag.clone(); move |event| handler(event) },
                    onpointerup: { let handler = end_drag.clone(); move |event| handler(event) },
                    onpointercancel: { let handler = cancel_drag.clone(); move |event| handler(event) },
                    onlostpointercapture: { let handler = cancel_drag.clone(); move |event| handler(event) },
                    onkeydown: { let handler = on_key_down.clone(); move |event| handler(event) },
                    rect {
                        x: "-12",
                        y: "-12",
                        width: "24",
                        height: "24",
                        style: "fill:transparent;stroke:none;pointer-events:all",
                    }
                    circle {
                        r: "3",
                        style: "fill:var(--card);stroke:var(--primary);stroke-width:1.5px;vector-effect:non-scaling-stroke",
                    }
                    path {
                        d: "M-6 0h12M0 -6v12",
                        style: "fill:none;stroke:var(--primary);stroke-width:1px;vector-effect:non-scaling-stroke",
                    }
                    title { "Move splay origin" }
                }
            }
        }
        // Ticket 06: the value popup — docked 8 px above the rail while a
        // tool is hovered or focused, 18 px off the pointer while dragging —
        // and the commit toast at the top centre of the canvas.
        {feedback_chrome(&feedback_state, true, popup_at(), rail_layout.as_ref(), px, (view_x, view_y, width))}
    }
}

// Presentation-only angular annotation: the accepted Core scene supplies origin and angle.
pub(super) fn splay_arc(origin: Vec2, end: Vec2, degrees: f64) -> String {
    let dx = end.x - origin.x;
    let dy = end.y - origin.y;
    let radius = dx.hypot(dy);
    let angle = dy.atan2(dx);
    let sweep_degrees = degrees.clamp(-359.9, 359.9);
    let start = angle - sweep_degrees.to_radians();
    let x = origin.x + radius * start.cos();
    let y = origin.y + radius * start.sin();
    let large = u8::from(sweep_degrees.abs() > 180.0);
    let sweep = u8::from(sweep_degrees >= 0.0);
    format!(
        "M{x} {y} A{radius} {radius} 0 {large} {sweep} {} {}",
        end.x, end.y
    )
}

pub(super) fn angle_handle_point(
    projection: &MatrixScene,
    column: u32,
    pitch_y: f64,
    points: &[Vec2],
) -> Vec2 {
    if points.is_empty() {
        let origin = projection
            .columns
            .iter()
            .find(|basis| basis.column == column)
            .map_or(Vec2::default(), |basis| basis.splay_origin);
        return Vec2 {
            x: origin.x,
            y: origin.y + 20.0,
        };
    }
    let x = points
        .iter()
        .map(|point| point.x)
        .fold(f64::INFINITY, f64::min)
        - pitch_y * 0.9;
    let y = points
        .iter()
        .map(|point| point.y)
        .fold(f64::NEG_INFINITY, f64::max)
        + pitch_y * 0.9;
    Vec2 { x, y }
}

// The modifier changes presentation immediately; pointer-down latches the edit intent.
pub(super) struct CtrlModifierListener {
    pub(super) document: web_sys::Document,
    pub(super) window: web_sys::Window,
    pub(super) down: Closure<dyn FnMut(web_sys::KeyboardEvent)>,
    pub(super) up: Closure<dyn FnMut(web_sys::KeyboardEvent)>,
    pub(super) blur: Closure<dyn FnMut(web_sys::Event)>,
}
impl CtrlModifierListener {
    pub(super) fn install(mut pressed: Signal<bool>) -> Option<Self> {
        let window = web_sys::window()?;
        let document = window.document()?;
        let down = Closure::wrap(Box::new(move |event: web_sys::KeyboardEvent| {
            if event.key() == "Control" {
                pressed.set(true);
            }
        }) as Box<dyn FnMut(_)>);
        let up = Closure::wrap(Box::new(move |event: web_sys::KeyboardEvent| {
            if event.key() == "Control" {
                pressed.set(false);
            }
        }) as Box<dyn FnMut(_)>);
        let blur = Closure::wrap(
            Box::new(move |_: web_sys::Event| pressed.set(false)) as Box<dyn FnMut(_)>
        );
        let listener = Self {
            document,
            window,
            down,
            up,
            blur,
        };
        listener
            .document
            .add_event_listener_with_callback("keydown", listener.down.as_ref().unchecked_ref())
            .ok()?;
        listener
            .document
            .add_event_listener_with_callback("keyup", listener.up.as_ref().unchecked_ref())
            .ok()?;
        listener
            .window
            .add_event_listener_with_callback("blur", listener.blur.as_ref().unchecked_ref())
            .ok()?;
        Some(listener)
    }
}
impl Drop for CtrlModifierListener {
    fn drop(&mut self) {
        let _ = self
            .document
            .remove_event_listener_with_callback("keydown", self.down.as_ref().unchecked_ref());
        let _ = self
            .document
            .remove_event_listener_with_callback("keyup", self.up.as_ref().unchecked_ref());
        let _ = self
            .window
            .remove_event_listener_with_callback("blur", self.blur.as_ref().unchecked_ref());
    }
}
