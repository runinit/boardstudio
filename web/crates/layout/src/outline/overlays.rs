//! Canvas overlays for editing and previewing outlines.

use super::hook::{OutlineActionContext, OutlineDrawTool, OutlineInspectorProjection};
use super::planner::{OutlinePointTarget, PerimeterAnchor};
use crate::canvas_interaction::{CanvasInteractionArbiter, CanvasInteractionOwner};
use crate::runtime::Runtime;
use boardstudio_application::Event;
use boardstudio_core::model::{EditPhase, Operation, OutlineFeature, Vec2};
use dioxus::prelude::*;
use dioxus_web::WebEventExt;
use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::JsCast;
use web_sys::SvgElement;

struct PointDrag {
    pointer_id: i32,
    start_client: (i32, i32),
    point_index: usize,
    points: Vec<Vec2>,
    pending: Vec<Vec2>,
    preview_points: Vec<Vec2>,
    original_world: Vec2,
    target: OutlinePointTarget,
    anchor: Option<PerimeterAnchor>,
    action_context: Rc<OutlineActionContext>,
    transaction_id: String,
    capture: SvgElement,
    snap: Option<crate::presentation::outline_snapping::Snap>,
    preview_submitted: bool,
    moved: bool,
}

#[derive(Clone)]
struct PointSnapInputs {
    paths: Vec<Vec<Vec2>>,
    origins: Rc<Vec<crate::presentation::outline_snapping::Origin>>,
    grid: Vec2,
    geometry_snap: bool,
    svg: Rc<RefCell<Option<SvgElement>>>,
    width: f64,
}

#[derive(Clone)]
pub struct OutlineRuntimeHandle(Rc<Runtime>);

impl OutlineRuntimeHandle {
    pub fn new(runtime: Rc<Runtime>) -> Self {
        Self(runtime)
    }
}

impl PartialEq for OutlineRuntimeHandle {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

#[component]
pub fn OutlinePointCanvasOverlay(
    projection: OutlineInspectorProjection,
    runtime: OutlineRuntimeHandle,
    arbiter: CanvasInteractionArbiter,
    svg: Rc<RefCell<Option<SvgElement>>>,
    view_x: f64,
    view_y: f64,
    width: f64,
    height: f64,
    snap_settings: crate::objects::LayoutSnapSettings,
    pitch: Vec2,
    origins: Vec<crate::presentation::outline_snapping::Origin>,
) -> Element {
    let drag = use_hook(|| Rc::new(RefCell::new(None::<PointDrag>)));
    let mut preview = use_signal(|| None::<Vec<Vec2>>);
    let mut guides = use_signal(|| None::<crate::presentation::outline_snapping::Snap>);
    let token = projection.token;
    let revision = projection.revision;
    let clear_board_id = projection.board_id.clone();
    let drop_drag = drag.clone();
    let drop_runtime = runtime.0.clone();
    let drop_arbiter = arbiter.clone();
    let drop_board_id = clear_board_id.clone();
    use_drop(move || {
        if let Some(active) = drop_drag.borrow_mut().take() {
            drop_arbiter.release(CanvasInteractionOwner::OutlinePerimeter);
            if active.capture.has_pointer_capture(active.pointer_id) {
                let _ = active.capture.release_pointer_capture(active.pointer_id);
            }
            if active.preview_submitted {
                drop_runtime.submit(Event::ClearPreview {
                    operation_id: drop_runtime.operation(),
                    token,
                    revision,
                    board_id: drop_board_id,
                    transaction_id: active.transaction_id,
                });
            }
        }
    });
    let Some(perimeter) = projection
        .perimeter
        .as_ref()
        .filter(|_| (projection.editing_points)())
    else {
        return rsx! {};
    };
    if perimeter.canvas_points.len() < 3 {
        return rsx! {};
    }
    let on_action = projection.on_action;
    let action_context = projection.action_context.clone();
    let selected_point = projection.selected_point;
    let saved_points = perimeter.points.clone();
    let canvas_points = perimeter.canvas_points.clone();
    let target = perimeter.target.clone();
    let anchor = perimeter.anchor;
    let snap_paths = projection.snap_paths.clone();
    let origins = Rc::new(origins);
    let grid = outline_grid(snap_settings.snap_fraction, pitch);
    let geometry_snap = snap_settings.geometry_snap;
    let enabled = projection.enabled;
    let snap_inputs = PointSnapInputs {
        paths: snap_paths.clone(),
        origins: origins.clone(),
        grid,
        geometry_snap,
        svg: svg.clone(),
        width,
    };

    let finish_drag = {
        let drag = drag.clone();
        let runtime = runtime.0.clone();
        let arbiter = arbiter.clone();
        let snap_inputs = snap_inputs.clone();
        move |commit: bool, final_at: Option<Vec2>, final_displaced: bool, free: bool| {
            let Some(mut active) = drag.borrow_mut().take() else {
                return;
            };
            if (active.moved || final_displaced)
                && let Some(at) = final_at
            {
                apply_point_sample(&mut active, at, &snap_inputs, free);
            }
            preview.set(None);
            guides.set(None);
            arbiter.release(CanvasInteractionOwner::OutlinePerimeter);
            if active.capture.has_pointer_capture(active.pointer_id) {
                let _ = active.capture.release_pointer_capture(active.pointer_id);
            }
            if commit && active.moved {
                on_action.call(active.action_context.edit_perimeter(
                    active.target,
                    active.pending,
                    EditPhase::Commit,
                    active.transaction_id,
                ));
            } else if active.preview_submitted {
                runtime.submit(Event::ClearPreview {
                    operation_id: runtime.operation(),
                    token,
                    revision,
                    board_id: clear_board_id.clone(),
                    transaction_id: active.transaction_id,
                });
            }
        }
    };

    let rendered_points = preview().unwrap_or(canvas_points.clone());
    let screen_width = svg
        .borrow()
        .as_ref()
        .map(|surface| surface.get_bounding_client_rect().width())
        .filter(|value| *value > 0.0)
        .unwrap_or(800.0);
    let handle_radius = width / screen_width * 12.0;
    let rendered_point_string = rendered_points
        .iter()
        .chain(rendered_points.first())
        .map(|point| format!("{},{}", point.x, point.y))
        .collect::<Vec<_>>()
        .join(" ");
    rsx! {
        g { class: "m1-outline-point-controls", "aria-label": "Outline perimeter points",
            if let Some(snap) = guides().as_ref().filter(|snap| !snap.guides.is_empty()) {
                g { class: "m1-outline-snap-guides", "aria-label": "Outline alignment guides",
                    for (index, guide) in snap.guides.iter().enumerate() {
                        line {
                            key: "guide-{index}-{guide.id}",
                            class: "m1-outline-snap-guide",
                            x1: "{guide.from.x - guide.direction.x * width * 2.0}",
                            y1: "{guide.from.y - guide.direction.y * width * 2.0}",
                            x2: "{guide.from.x + guide.direction.x * width * 2.0}",
                            y2: "{guide.from.y + guide.direction.y * width * 2.0}",
                            "data-guide": "{guide.label}"
                        }
                        text {
                            class: "m1-outline-snap-label",
                            transform: "translate({snap.at.x - handle_radius} {snap.at.y - handle_radius * (2.0 + index as f64)}) scale(1,-1)",
                            text_anchor: "end",
                            "{guide.label}"
                        }
                    }
                }
            }
            polyline {
                class: "m1-outline-control-path",
                points: "{rendered_point_string}",
            }
            for (index, point) in rendered_points.iter().copied().enumerate() {
                {
                    let is_selected = selected_point() == index;
                    let point_drag = drag.clone();
                    let point_preview = preview;
                    let point_guides = guides;
                    let point_action = on_action;
                    let point_context = action_context.clone();
                    let point_runtime = runtime.0.clone();
                    let point_arbiter = arbiter.clone();
                    let point_target = target.clone();
                    let point_points = saved_points.clone();
                    let point_canvas_points = canvas_points.clone();
                    let point_anchor = anchor;
                    let point_snap_inputs = snap_inputs.clone();
                    let point_svg = svg.clone();
                    let mut point_selected = selected_point;
                    let point_id = format!("outline-point-{index}");
                    let point_label = format!("Outline point {}", index + 1);
                    let point_x = point.x;
                    let point_y = point.y;
                    let point_commit = finish_drag.clone();
                    let mut pointerup_commit = point_commit.clone();
                    let mut key_commit = point_commit.clone();
                    let mut cancel_commit = point_commit.clone();
                    let mut lost_capture_commit = point_commit.clone();
                    let down_drag = point_drag.clone();
                    let down_arbiter = point_arbiter.clone();
                    let mut down_selected = point_selected;
                    let down_runtime = point_runtime.clone();
                    let down_points = point_points.clone();
                    let down_canvas_points = point_canvas_points.clone();
                    let down_target = point_target.clone();
                    let down_context = point_context.clone();
                    let move_drag = point_drag.clone();
                    let move_svg = point_svg.clone();
                    let move_snap_inputs = point_snap_inputs.clone();
                    let mut move_preview = point_preview;
                    let mut move_guides = point_guides;
                    let move_action = point_action;
                    let up_drag = point_drag.clone();
                    let up_svg = point_svg.clone();
                    let key_drag = point_drag.clone();
                    let key_points = point_points.clone();
                    let key_anchor = point_anchor;
                    let key_context = point_context.clone();
                    let key_target = point_target.clone();
                    let key_runtime = point_runtime.clone();
                    let mut key_selected = point_selected;
                    let key_action = point_action;
                    let mut escape_editing_points = projection.editing_points;
                    let escape_context = projection.selected_context;
                    let escape_scope = projection.scope.clone();
                    let escape_board_id = projection.board_id.clone();
                    rsx! {
                        circle {
                            key: "{point_id}",
                            class: if is_selected { "m1-outline-point-handle is-selected" } else { "m1-outline-point-handle" },
                            role: "button",
                            tabindex: "0",
                            "aria-label": "{point_label}",
                            "aria-pressed": "{is_selected}",
                            cx: "{point_x}", cy: "{point_y}", r: "{handle_radius}",
                            onfocus: move |_| point_selected.set(index),
                            onpointerdown: move |event: PointerEvent| {
                                let Some(pointer) = event.data().try_as_web_event() else { return; };
                                pointer.stop_propagation();
                                if !enabled || pointer.button() != 0 || !down_arbiter.try_acquire(CanvasInteractionOwner::OutlinePerimeter) { return; }
                                let Some(capture) = pointer.target().and_then(|target| target.dyn_into::<SvgElement>().ok()).filter(|target| target.get_attribute("class").is_some_and(|classes| classes.split_whitespace().any(|class| class == "m1-outline-point-handle"))) else { down_arbiter.release(CanvasInteractionOwner::OutlinePerimeter); return; };
                                if capture.set_pointer_capture(pointer.pointer_id()).is_err() || !capture.has_pointer_capture(pointer.pointer_id()) {
                                    down_arbiter.release(CanvasInteractionOwner::OutlinePerimeter);
                                    return;
                                }
                                pointer.prevent_default();
                                down_selected.set(index);
                                let transaction_id = format!("outline-drag-{}", down_runtime.operation().0);
                                *down_drag.borrow_mut() = Some(PointDrag {
                                    pointer_id: pointer.pointer_id(), point_index: index,
                                    start_client: (pointer.client_x(), pointer.client_y()),
                                    points: down_points.clone(), pending: down_points.clone(), preview_points: down_canvas_points.clone(), original_world: down_canvas_points[index],
                                    target: down_target.clone(), anchor: point_anchor, action_context: down_context.clone(), transaction_id,
                                    capture, snap: None, preview_submitted: false, moved: false,
                                });
                            },
                            onpointermove: move |event: PointerEvent| {
                                let Some(pointer) = event.data().try_as_web_event() else { return; };
                                let mut active = move_drag.borrow_mut();
                                let Some(active) = active.as_mut().filter(|active| active.pointer_id == pointer.pointer_id()) else { return; };
                                pointer.stop_propagation();
                                let Some(at) = crate::coordinates(&move_svg, &pointer, view_x, view_y, width, height) else { return; };
                                apply_point_sample(active, at, &move_snap_inputs, pointer.alt_key());
                                let preview_world = active.preview_points.clone();
                                let pending = active.pending.clone();
                                let context = active.action_context.clone();
                                let target = active.target.clone();
                                let transaction = active.transaction_id.clone();
                                let changed = active.moved;
                                if changed {
                                    active.preview_submitted = true;
                                }
                                let snap = active.snap.clone();
                                let _ = active;
                                move_preview.set(Some(preview_world)); move_guides.set(snap);
                                if changed {
                                    move_action.call(context.edit_perimeter(target, pending, EditPhase::Preview, transaction));
                                }
                            },
                            onpointerup: move |event: PointerEvent| {
                                let Some(pointer) = event.data().try_as_web_event() else { return; };
                                let final_displaced = up_drag.borrow().as_ref().is_some_and(|drag| {
                                    drag.pointer_id == pointer.pointer_id()
                                        && drag.start_client != (pointer.client_x(), pointer.client_y())
                                });
                                if !up_drag.borrow().as_ref().is_some_and(|drag| drag.pointer_id == pointer.pointer_id()) { return; }
                                pointer.stop_propagation();
                                let at = crate::coordinates(&up_svg, &pointer, view_x, view_y, width, height);
                                pointerup_commit(true, at, final_displaced, pointer.alt_key());
                            },
                            onpointercancel: move |_| cancel_commit(false, None, false, false),
                            onlostpointercapture: move |_| lost_capture_commit(false, None, false, false),
                            onkeydown: move |event: KeyboardEvent| {
                                let Some(key) = event.data().try_as_web_event() else { return; };
                                if key.key() == "Escape" && key_drag.borrow().is_some() {
                                    key.prevent_default(); key.stop_propagation(); key_commit(false, None, false, false); return;
                                }
                                if key.key() == "Escape" {
                                    key.prevent_default();
                                    key.stop_propagation();
                                    let Some(selected) = escape_context.read().clone() else { return; };
                                    if selected.scope != escape_scope
                                        || !matches!(
                                            &selected.context,
                                            crate::objects::TreeContext::Outline { board_id }
                                                if board_id == &escape_board_id
                                        )
                                    {
                                        return;
                                    }
                                    escape_editing_points.set(false);
                                    return;
                                }
                                if key_drag.borrow().is_some() || !enabled { return; }
                                let mut points = key_points.clone();
                                let step_x = grid.x.max(0.1) * if key.shift_key() { 10.0 } else { 1.0 };
                                let step_y = grid.y.max(0.1) * if key.shift_key() { 10.0 } else { 1.0 };
                                let world = key_anchor.map_or(Vec2 { x: point_x, y: point_y }, |a| a.world(key_points[index]));
                                let (dx, dy) = match key.key().as_str() {
                                    "ArrowLeft" => (-step_x, 0.0), "ArrowRight" => (step_x, 0.0),
                                    "ArrowUp" => (0.0, step_y), "ArrowDown" => (0.0, -step_y),
                                    _ if key.key() == "Delete" || key.key() == "Backspace" => {
                                        key.prevent_default(); key.stop_propagation();
                                        if points.len() <= 3 { return; }
                                        points.remove(index); key_selected.set(index.saturating_sub(1));
                                        key_action.call(key_context.edit_perimeter(key_target.clone(), points, EditPhase::Commit, format!("outline-delete-{}-{}", key_runtime.operation().0, index)));
                                        return;
                                    },
                                    _ => return,
                                };
                                key.prevent_default(); key.stop_propagation();
                                let next = Vec2 { x: world.x + dx, y: world.y + dy };
                                points[index] = key_anchor.map_or(next, |a| a.local(next));
                                key_action.call(key_context.edit_perimeter(key_target.clone(), points, EditPhase::Commit, format!("outline-nudge-{}-{}", key_runtime.operation().0, index)));
                            }
                        }
                        text { class: "m1-outline-point-label", x: "{point_x + 2.0}", y: "{point_y + 2.0}", "aria-hidden": "true", "{index + 1}" }
                    }
                }
            }
        }
    }
}

pub(super) fn outline_grid(fraction: f64, pitch: Vec2) -> Vec2 {
    let spacing = |axis: f64| {
        if fraction > 0.0 {
            axis * fraction
        } else if fraction < 0.0 {
            -fraction
        } else {
            0.0
        }
    };
    Vec2 {
        x: spacing(pitch.x),
        y: spacing(pitch.y),
    }
}

pub(super) fn polygon_area(points: &[Vec2]) -> f64 {
    points
        .iter()
        .enumerate()
        .map(|(index, point)| {
            let next = points[(index + 1) % points.len()];
            point.x * next.y - next.x * point.y
        })
        .sum::<f64>()
        * 0.5
}

#[component]
pub fn OutlineDraftCanvasOverlay(
    projection: OutlineInspectorProjection,
    runtime: OutlineRuntimeHandle,
    arbiter: CanvasInteractionArbiter,
    svg: Rc<RefCell<Option<SvgElement>>>,
    view_x: f64,
    view_y: f64,
    width: f64,
    height: f64,
    snap_settings: crate::objects::LayoutSnapSettings,
    pitch: Vec2,
    origins: Vec<crate::presentation::outline_snapping::Origin>,
) -> Element {
    let Some(tool) = (projection.drawing_operation)() else {
        return rsx! {};
    };
    let minimum_points = if tool == OutlineDrawTool::Connect {
        2
    } else {
        3
    };
    let mut drawing_operation = projection.drawing_operation;
    let points = projection.drawing_points;
    let grid = outline_grid(snap_settings.snap_fraction, pitch);
    let paths = projection.snap_paths.clone();
    let origins = Rc::new(origins);
    let geometry_snap = snap_settings.geometry_snap;
    let draft = points.read().clone();
    let point_string = draft
        .iter()
        .map(|point| format!("{},{}", point.x, point.y))
        .collect::<Vec<_>>()
        .join(" ");
    let runtime = runtime.0.clone();
    let action_context = projection.action_context.clone();
    let on_action = projection.on_action;
    let enabled = projection.enabled;
    let board_id = projection.board_id.clone();
    let revision = projection.revision;
    let finish = {
        let mut points = points;
        let runtime = runtime.clone();
        move || {
            let draft = points.read().clone();
            if draft.len() < minimum_points
                || matches!(tool, OutlineDrawTool::Polygon(_)) && polygon_area(&draft).abs() < 1e-6
            {
                return;
            }
            match tool {
                OutlineDrawTool::Polygon(operation) => {
                    let feature = OutlineFeature::Polygon {
                        id: format!(
                            "outline-manual-{board_id}-{}-{}",
                            revision,
                            runtime.operation().0
                        ),
                        points: draft,
                        anchor_part_id: None,
                        operation,
                    };
                    on_action.call(action_context.add_feature(feature));
                }
                OutlineDrawTool::Connect => on_action.call(action_context.add_connection(draft)),
            }
            points.set(Vec::new());
            drawing_operation.set(None);
        }
    };
    rsx! {
        g { class: "m1-outline-draft-controls", "aria-label": match tool { OutlineDrawTool::Polygon(Operation::Add) => "Draw addition", OutlineDrawTool::Polygon(Operation::Subtract) => "Draw cutout", OutlineDrawTool::Connect => "Connect points" },
            rect {
                transform: "scale(1,-1)",
                x: "{view_x}", y: "{view_y}", width: "{width}", height: "{height}",
                fill: "transparent", tabindex: "0", role: "application",
                "aria-label": "Outline drawing canvas. Click to add points. Enter finishes; Escape cancels.",
                onpointerdown: move |event: PointerEvent| {
                    let mut points = points;
                    let Some(pointer) = event.data().try_as_web_event() else { return; };
                    pointer.stop_propagation();
                    if !enabled || pointer.button() != 0 || !arbiter.try_acquire(CanvasInteractionOwner::OutlinePerimeter) { return; }
                    let Some(at) = crate::coordinates(&svg, &pointer, view_x, view_y, width, height) else { arbiter.release(CanvasInteractionOwner::OutlinePerimeter); return; };
                    let path_points = points.read().clone();
                    let context = crate::presentation::outline_snapping::Context { anchor: path_points.last().copied(), previous: path_points.iter().rev().nth(1).copied(), exclude: None, neighbor: None };
                    let px = svg.borrow().as_ref().map(|surface| surface.get_bounding_client_rect().width()).unwrap_or(1.0).max(1.0);
                    let tolerance = width / px * 7.0;
                    let mut snapped = crate::presentation::outline_snapping::snap_outline_point(at, context, &paths, crate::presentation::outline_snapping::Options { grid, tolerance, enabled: geometry_snap, free: pointer.alt_key() }, None);
                    if !pointer.alt_key() && geometry_snap && snapped.guides.is_empty()
                        && let Some((at, _)) = crate::presentation::outline_snapping::snap_origin(at, &origins, tolerance)
                    {
                        snapped.at = at;
                    }
                    if !path_points.iter().any(|point| (point.x-snapped.at.x).abs() < 1e-7 && (point.y-snapped.at.y).abs() < 1e-7) { points.set(path_points.into_iter().chain([snapped.at]).collect()); }
                    arbiter.release(CanvasInteractionOwner::OutlinePerimeter);
                    pointer.prevent_default();
                },
                onkeydown: {
                    let mut finish = finish.clone();
                    let mut points = points;
                    let mut drawing_operation = drawing_operation;
                    move |event: KeyboardEvent| {
                        let Some(key) = event.data().try_as_web_event() else { return; };
                        match key.key().as_str() {
                            "Enter" => { key.prevent_default(); key.stop_propagation(); finish(); },
                            "Escape" => { key.prevent_default(); key.stop_propagation(); points.set(Vec::new()); drawing_operation.set(None); },
                            "Backspace" | "Delete" => { key.prevent_default(); key.stop_propagation(); let mut next = points.read().clone(); next.pop(); points.set(next); },
                            _ => {}
                        }
                    }
                },
            }
            if !draft.is_empty() {
                polyline { class: "m1-outline-draft-path", points: "{point_string}" }
                for (index, point) in draft.iter().enumerate() {
                    circle { key: "outline-draft-point-{index}", class: "m1-outline-draft-point", cx: "{point.x}", cy: "{point.y}", r: "1.5" }
                }
            }
        }
    }
}

fn apply_point_sample(drag: &mut PointDrag, world: Vec2, inputs: &PointSnapInputs, free: bool) {
    let count = drag.points.len();
    let index = drag.point_index;
    let context = crate::presentation::outline_snapping::Context {
        anchor: Some(drag.preview_points[(index + count - 1) % count]),
        previous: Some(drag.original_world),
        exclude: Some(drag.original_world),
        neighbor: Some(drag.preview_points[(index + 1) % count]),
    };
    let pixel_width = inputs
        .svg
        .borrow()
        .as_ref()
        .map(|surface| surface.get_bounding_client_rect().width())
        .unwrap_or(1.0)
        .max(1.0);
    let tolerance = inputs.width / pixel_width * 7.0;
    let mut result = crate::presentation::outline_snapping::snap_outline_point(
        world,
        context,
        &inputs.paths,
        crate::presentation::outline_snapping::Options {
            grid: inputs.grid,
            tolerance,
            enabled: inputs.geometry_snap,
            free,
        },
        drag.snap.as_ref(),
    );
    if !free
        && inputs.geometry_snap
        && result.guides.is_empty()
        && let Some((at, _)) =
            crate::presentation::outline_snapping::snap_origin(world, &inputs.origins, tolerance)
    {
        result.at = at;
    }
    let mut preview_points = drag.preview_points.clone();
    preview_points[index] = result.at;
    let mut pending = drag.points.clone();
    pending[index] = drag
        .anchor
        .map_or(result.at, |anchor| anchor.local(result.at));
    drag.moved = pending != drag.points;
    drag.pending = pending;
    drag.preview_points = preview_points;
    drag.snap = Some(result);
}

#[component]
pub(super) fn OutlineDimension(
    label: &'static str,
    value: f64,
    minimum: f64,
    editable: bool,
    on_commit: EventHandler<f64>,
) -> Element {
    let mut field = use_signal(|| (value, value.to_string()));
    use_effect(use_reactive!(|value| {
        if field.peek().0 != value {
            field.set((value, value.to_string()));
        }
    }));
    let draft = if field().0 == value {
        field().1
    } else {
        value.to_string()
    };
    let parsed = draft.trim().parse::<f64>();
    let valid = !draft.trim().is_empty()
        && parsed.is_ok_and(|parsed| parsed.is_finite() && parsed >= minimum);
    let error = !valid;
    rsx! {
        label { class: "m1-outline-field",
            span { "{label}" }
            span { class: "m1-outline-number",
                input {
                    r#type: "number",
                    step: "0.1",
                    min: "{minimum}",
                    value: "{draft}",
                    disabled: !editable,
                    aria_label: label,
                    aria_invalid: error,
                    oninput: move |event: FormEvent| field.set((value, event.value())),
                    onblur: move |_| {
                        if field().0 != value {
                            field.set((value, value.to_string()));
                        } else if let Ok(next) = field().1.trim().parse::<f64>()
                            && next.is_finite() && next >= minimum && next != value
                        { on_commit.call(next); }
                    },
                    onkeydown: move |event: KeyboardEvent| match event.data().key().to_string().as_str() {
                        "Enter" => {
                            event.prevent_default();
                            if let Some(input) = event.data().try_as_web_event()
                                .and_then(|event| event.target())
                                .and_then(|target| target.dyn_into::<web_sys::HtmlInputElement>().ok())
                            { let _ = input.blur(); }
                        }
                        "Escape" => {
                            event.prevent_default();
                            let has_draft = field().1 != value.to_string();
                            field.set((value, value.to_string()));
                            if has_draft { event.stop_propagation(); }
                        }
                        _ => {}
                    }
                }
                small { "mm" }
            }
            if error { small { role: "alert", "Enter a finite value greater than or equal to {minimum} mm." } }
        }
    }
}

#[component]
pub(super) fn OutlineCoordinate(
    label: String,
    value: f64,
    editable: bool,
    on_commit: EventHandler<f64>,
) -> Element {
    let mut field = use_signal(|| (value, value.to_string()));
    use_effect(use_reactive!(|value| {
        if field.peek().0 != value {
            field.set((value, value.to_string()));
        }
    }));
    let draft = if field().0 == value {
        field().1
    } else {
        value.to_string()
    };
    let valid = !draft.trim().is_empty()
        && draft
            .trim()
            .parse::<f64>()
            .is_ok_and(|parsed| parsed.is_finite());
    rsx! {
        label { class: "m1-outline-field",
            span { "{label}" }
            span { class: "m1-outline-number",
                input {
                    r#type: "number",
                    step: "0.1",
                    value: "{draft}",
                    disabled: !editable,
                    aria_label: "{label}",
                    aria_invalid: !valid,
                    oninput: move |event: FormEvent| field.set((value, event.value())),
                    onblur: move |_| {
                        if field().0 != value {
                            field.set((value, value.to_string()));
                        } else if let Ok(next) = field().1.trim().parse::<f64>()
                            && next.is_finite() && next != value
                        { on_commit.call(next); }
                    },
                    onkeydown: move |event: KeyboardEvent| match event.data().key().to_string().as_str() {
                        "Enter" => {
                            event.prevent_default();
                            if let Some(input) = event.data().try_as_web_event()
                                .and_then(|event| event.target())
                                .and_then(|target| target.dyn_into::<web_sys::HtmlInputElement>().ok())
                            { let _ = input.blur(); }
                        }
                        "Escape" => {
                            event.prevent_default();
                            let has_draft = field().1 != value.to_string();
                            field.set((value, value.to_string()));
                            if has_draft { event.stop_propagation(); }
                        }
                        _ => {}
                    }
                }
                span { "mm" }
            }
        }
    }
}
