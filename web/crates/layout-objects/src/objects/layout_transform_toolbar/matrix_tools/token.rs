//! The matrix rail token, splay-origin handle and feedback chrome of the transform overlay.
use super::*;

pub(super) struct MatrixTokenView<Show, Hide> {
    pub(super) tool: LayoutTransformTool,
    pub(super) context: TreeContext,
    pub(super) projection: Rc<MatrixScene>,
    pub(super) matrix: Rc<Matrix>,
    pub(super) selected_column: u32,
    pub(super) matrix_rotation_followed: bool,
    pub(super) offset_now: Vec2,
    pub(super) shift_now: Vec2,
    pub(super) bend_now: f64,
    pub(super) stagger_now: f64,
    pub(super) control: Signal<bool>,
    pub(super) quick_glyph: Signal<bool>,
    pub(super) swing_glyph: Signal<bool>,
    pub(super) hovered: Signal<bool>,
    pub(super) focused: Signal<bool>,
    pub(super) feedback: Signal<FeedbackState>,
    pub(super) popup_at: Signal<Option<(f64, f64)>>,
    pub(super) snap_guide: Signal<Option<Vec2>>,
    pub(super) drag: Rc<RefCell<Option<TransformDrag>>>,
    pub(super) svg: LayoutTransformSvg,
    pub(super) rail_index: Option<usize>,
    pub(super) rail_layout: Option<crate::matrix_transform_operation::handle_rail::WorldRailLayout>,
    pub(super) rail_tools: Vec<LayoutTransformTool>,
    pub(super) world_per_screen: f64,
    pub(super) angle_degrees: f64,
    pub(super) origin_point: Option<Vec2>,
    pub(super) arc: Option<String>,
    pub(super) angle_point: Vec2,
    pub(super) shift_parameter: Option<PlacementParameter>,
    pub(super) bend_parameter: Option<PlacementParameter>,
    pub(super) gesture: TransformGesture,
    pub(super) start_drag: Rc<dyn Fn(PointerEvent, TransformGesture)>,
    pub(super) move_drag: Rc<dyn Fn(PointerEvent)>,
    pub(super) end_drag: Rc<dyn Fn(PointerEvent)>,
    pub(super) cancel_drag: Rc<dyn Fn(PointerEvent)>,
    pub(super) on_key_down: Rc<dyn Fn(KeyboardEvent)>,
    pub(super) show_preview: Show,
    pub(super) release_preview: Hide,
    pub(super) view_x: f64,
    pub(super) view_y: f64,
    pub(super) width: f64,
}

pub(super) fn matrix_token_chrome<Show, Hide>(view: MatrixTokenView<Show, Hide>) -> Element
where
    Show: Fn() + Clone + 'static,
    Hide: Fn() + Clone + 'static,
{
    let MatrixTokenView {
        tool,
        context,
        projection,
        matrix,
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
        drag,
        svg,
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
    } = view;
    let stagger_axis = projection
        .columns
        .iter()
        .find(|basis| basis.column == selected_column)
        .map_or(Vec2 { x: 0.0, y: 1.0 }, |basis| basis.stagger_axis);
    let spread_now = match placement_current(
        &matrix,
        PlacementParameter::ColumnSpread {
            column: selected_column,
        },
    ) {
        PlacementValue::Spread(value) => value,
        _ => matrix.pitch.x,
    };
    // Splay, Move and Rotate keep hardcoded labels: the gesture distinguishes
    // them, and the mounted drag tests query these strings.
    let token_aria_label: String = match (tool, &context) {
        (LayoutTransformTool::Splay, _) => "Drag to splay",
        (LayoutTransformTool::Move, _) => "Drag to move matrix",
        (LayoutTransformTool::Rotate, _) => "Drag to rotate matrix",
        (LayoutTransformTool::Anchor, _) => "Pick anchor",
        (LayoutTransformTool::Origin, _) => "Move splay origin",
        (LayoutTransformTool::Shift, TreeContext::Row { .. }) => "Drag to shift row",
        (LayoutTransformTool::Shift, TreeContext::Key { .. }) => "Drag to shift key",
        (LayoutTransformTool::Shift, _) => "Drag to shift",
        (LayoutTransformTool::Bend, TreeContext::Row { .. }) => "Drag to bend row",
        (LayoutTransformTool::Bend, TreeContext::Key { .. }) => "Drag to bend key",
        (LayoutTransformTool::Bend, _) => "Drag to bend",
        (LayoutTransformTool::Stagger, _) if control() || quick_glyph() => "Drag to offset column",
        (LayoutTransformTool::Offset, TreeContext::Row { .. }) => "Drag to offset row",
        (LayoutTransformTool::Offset, _) => "Drag to offset column",
        (LayoutTransformTool::Spread, _) => "Drag to spread",
        _ => "Drag to stagger",
    }
    .to_owned();
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
    let active_peer = rail_active(Rail::Matrix, &svg);
    let dragging_here = drag.borrow().is_some();
    let rail_dimmed = rail_dimmed(active_peer, tool, rail_index);
    let token_active = dragging_here;
    let token_lit = hovered() || focused() || token_active;
    // Screen-frame glyph rotation for the column axis: the px groups below
    // are y-down, so the world axis appears mirrored vertically.
    let glyph_degrees = (-stagger_axis.y).atan2(stagger_axis.x).to_degrees();
    let (token_fill, glyph_stroke) = token_paint(token_active, hovered() || focused());
    let token_opacity = token_opacity(rail_dimmed);
    let guide_opacity = guide_opacity(token_active);
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
                let token_style = rail_token_style(token_opacity);
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
                    if token_lit && matches!(tool, LayoutTransformTool::Spread | LayoutTransformTool::Stagger | LayoutTransformTool::Offset)
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
