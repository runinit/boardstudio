//! Canvas overlays for editing and previewing outlines.
use boardstudio_web_layout_core::rail_gesture::grid_step;
use boardstudio_web_ui_shared::inspector_fields::NumericField;

use super::{
    OutlineAction, OutlineActionEnvelope, OutlineActionKind, OutlineDrawTool,
    OutlineInspectorProjection, OutlinePointTarget, commit_outline_action,
    preview_outline_perimeter,
};
use crate::canvas_interaction::{
    ActiveOutlineDrag, CanvasEvent, CanvasKey, CanvasMode, GestureSeed, LayoutInteraction,
    OutlineHandleSeed, client_point, focus_svg, release_capture,
};
use crate::runtime::Runtime;
use boardstudio_application::Event;
use boardstudio_model::model::{EditPhase, Operation, OutlineFeature, Vec2, polygon_signed_area};
use dioxus::prelude::*;
use dioxus_web::WebEventExt;
use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::JsCast;
use web_sys::SvgElement;

#[derive(Clone)]
struct PointSnapInputs {
    paths: Vec<Vec<Vec2>>,
    origins: Rc<Vec<crate::presentation::outline_snapping::Origin>>,
    grid: Vec2,
    geometry_snap: bool,
}

#[derive(Clone)]
pub struct OutlineRuntimeHandle(pub Rc<Runtime>);

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

fn snap_options(
    svg: &Rc<RefCell<Option<SvgElement>>>,
    width: f64,
    inputs: &PointSnapInputs,
    free: bool,
) -> crate::presentation::outline_snapping::Options {
    let pixel_width = svg
        .borrow()
        .as_ref()
        .map(|surface| surface.get_bounding_client_rect().width())
        .unwrap_or(1.0)
        .max(1.0);
    crate::presentation::outline_snapping::Options {
        grid: inputs.grid,
        tolerance: width / pixel_width * 7.0,
        enabled: inputs.geometry_snap,
        free,
    }
}

fn clear_outline_preview_and_chrome(
    runtime: &Rc<Runtime>,
    active: &ActiveOutlineDrag,
    mut preview: Signal<Option<Vec<Vec2>>>,
    mut guides: Signal<Option<crate::presentation::outline_snapping::Snap>>,
) {
    if active.preview_submitted {
        runtime.submit(Event::ClearPreview {
            operation_id: runtime.operation(),
            token: active.envelope.token,
            revision: active.envelope.revision,
            board_id: active.envelope.board_id.clone(),
            transaction_id: active.transaction_id.clone(),
        });
    }
    if let Ok(mut state) = preview.try_write() {
        *state = None;
    }
    if let Ok(mut state) = guides.try_write() {
        *state = None;
    }
}

fn perimeter_action(
    envelope: OutlineActionEnvelope,
    target: OutlinePointTarget,
    points: Vec<Vec2>,
    phase: EditPhase,
    transaction_id: String,
) -> OutlineAction {
    OutlineAction {
        envelope,
        kind: OutlineActionKind::EditPerimeter {
            target,
            points,
            phase,
            transaction_id,
        },
    }
}

#[derive(Clone)]
struct PointOverlayCtx {
    arbiter: LayoutInteraction,
    runtime: Rc<Runtime>,
    projection: OutlineInspectorProjection,
    svg: Rc<RefCell<Option<SvgElement>>>,
    snap: PointSnapInputs,
    view: (f64, f64, f64, f64),
    preview: Signal<Option<Vec<Vec2>>>,
    guides: Signal<Option<crate::presentation::outline_snapping::Snap>>,
}

fn move_outline_point(ctx: &PointOverlayCtx, pointer: &web_sys::PointerEvent) {
    let Some(at) = crate::coordinates(
        &ctx.svg, pointer, ctx.view.0, ctx.view.1, ctx.view.2, ctx.view.3,
    ) else {
        return;
    };
    let options = snap_options(&ctx.svg, ctx.view.2, &ctx.snap, pointer.alt_key());
    ctx.arbiter
        .sample_outline_drag(at, &ctx.snap.paths, &ctx.snap.origins, options);
    let Some(active) = ctx
        .arbiter
        .outline_drag()
        .filter(|drag| drag.pointer_id == pointer.pointer_id())
    else {
        return;
    };
    let (mut preview, mut guides) = (ctx.preview, ctx.guides);
    preview.set(Some(active.preview_points.clone()));
    guides.set(active.snap.clone());
    let operation = active
        .moved
        .then(|| {
            let tx = active.transaction_id.clone();
            preview_outline_perimeter(
                &ctx.runtime,
                ctx.projection.ui,
                &perimeter_action(
                    active.envelope.clone(),
                    active.target.clone(),
                    active.pending.clone(),
                    EditPhase::Preview,
                    tx.clone(),
                ),
                &tx,
            )
        })
        .flatten();
    if operation.is_some() {
        ctx.arbiter.note_outline_preview_submitted();
    }
    ctx.arbiter.handle(CanvasEvent::PointerMove {
        id: pointer.pointer_id(),
        client: client_point(pointer),
        operation,
    });
}

fn finish_outline_point(
    ctx: &PointOverlayCtx,
    pointer: &web_sys::PointerEvent,
    mut active: ActiveOutlineDrag,
) {
    if (active.moved || active.start_client != client_point(pointer))
        && let Some(at) = crate::coordinates(
            &ctx.svg, pointer, ctx.view.0, ctx.view.1, ctx.view.2, ctx.view.3,
        )
    {
        let options = snap_options(&ctx.svg, ctx.view.2, &ctx.snap, pointer.alt_key());
        ctx.arbiter
            .sample_outline_drag(at, &ctx.snap.paths, &ctx.snap.origins, options);
        if let Some(updated) = ctx.arbiter.outline_drag() {
            active = updated;
        }
    }
    ctx.arbiter.handle(CanvasEvent::PointerUp {
        id: pointer.pointer_id(),
        operation: None,
    });
    release_capture(&ctx.svg, pointer.pointer_id());
    let (mut preview, mut guides) = (ctx.preview, ctx.guides);
    preview.set(None);
    guides.set(None);
    if active.moved {
        let generation = active.envelope.generation;
        commit_outline_action(
            &ctx.runtime,
            ctx.projection.ui,
            ctx.projection.selected_context,
            ctx.projection.workspace,
            ctx.projection.scope_generation,
            generation,
            perimeter_action(
                active.envelope,
                active.target,
                active.pending,
                EditPhase::Commit,
                active.transaction_id,
            ),
        );
    } else if active.preview_submitted {
        ctx.runtime.submit(Event::ClearPreview {
            operation_id: ctx.runtime.operation(),
            token: active.envelope.token,
            revision: active.envelope.revision,
            board_id: active.envelope.board_id,
            transaction_id: active.transaction_id,
        });
    }
}

#[component]
pub fn OutlinePointCanvasOverlay(
    projection: OutlineInspectorProjection,
    runtime: OutlineRuntimeHandle,
    svg: Rc<RefCell<Option<SvgElement>>>,
    view_x: f64,
    view_y: f64,
    width: f64,
    height: f64,
    snap_settings: crate::objects::LayoutSnapSettings,
    pitch: Vec2,
    origins: Vec<crate::presentation::outline_snapping::Origin>,
) -> Element {
    let arbiter = use_context::<LayoutInteraction>();
    let _ = arbiter.version();
    let preview = use_signal(|| None::<Vec<Vec2>>);
    let guides = use_signal(|| None::<crate::presentation::outline_snapping::Snap>);
    let token = projection.token;
    let revision = projection.revision;
    let drop_board_id = projection.board_id.clone();
    use_drop(move || {
        if let Some(active) = arbiter.outline_drag()
            && active.envelope.board_id == drop_board_id
            && active.envelope.token == token
            && active.envelope.revision == revision
        {
            arbiter.handle(CanvasEvent::PointerCancel(active.pointer_id));
        }
    });
    let Some(perimeter) = projection
        .perimeter
        .as_ref()
        .filter(|_| (projection.ui.editing_points)())
    else {
        return rsx! {};
    };
    if perimeter.canvas_points.len() < 3 {
        return rsx! {};
    }
    let canvas_ui = projection.ui.canvas_ui;
    let saved_points = perimeter.points.clone();
    let canvas_points = perimeter.canvas_points.clone();
    let target = perimeter.target.clone();
    let anchor = perimeter.anchor;
    let enabled = projection.enabled;
    let envelope = projection.action_envelope();
    let grid = grid_step(pitch, snap_settings.snap_fraction).unwrap_or_default();
    let snap_inputs = PointSnapInputs {
        paths: projection.snap_paths.clone(),
        origins: Rc::new(origins),
        grid,
        geometry_snap: snap_settings.geometry_snap,
    };

    let ctx = PointOverlayCtx {
        arbiter,
        runtime: runtime.0.clone(),
        projection: projection.clone(),
        svg: svg.clone(),
        snap: snap_inputs,
        view: (view_x, view_y, width, height),
        preview,
        guides,
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
        g { "aria-label": "Outline perimeter points",
            if let Some(snap) = guides().as_ref().filter(|snap| !snap.guides.is_empty()) {
                g { "aria-label": "Outline alignment guides",
                    for (index, guide) in snap.guides.iter().enumerate() {
                        line {
                            key: "guide-{index}-{guide.id}",
                            class: "pointer-events-none stroke-selected-foreground stroke-[0.3] [stroke-dasharray:1_0.7]",
                            x1: "{guide.from.x - guide.direction.x * width * 2.0}",
                            y1: "{guide.from.y - guide.direction.y * width * 2.0}",
                            x2: "{guide.from.x + guide.direction.x * width * 2.0}",
                            y2: "{guide.from.y + guide.direction.y * width * 2.0}",
                            "data-guide": "{guide.label}"
                        }
                        text {
                            class: "pointer-events-none fill-selected-foreground stroke-canvas stroke-[0.25] text-[2.6px] [paint-order:stroke]",
                            transform: "translate({snap.at.x - handle_radius} {snap.at.y - handle_radius * (2.0 + index as f64)}) scale(1,-1)",
                            text_anchor: "end",
                            "{guide.label}"
                        }
                    }
                }
            }
            polyline {
                class: "pointer-events-none fill-none stroke-primary stroke-[0.65] [stroke-dasharray:1.2_0.6]",
                points: "{rendered_point_string}",
            }
            for (index, point) in rendered_points.iter().copied().enumerate() {
                {
                    let is_selected = canvas_ui.read().selected_point == index;
                    let mut point_selected = canvas_ui;
                    let point_id = format!("outline-point-{index}");
                    let point_label = format!("Outline point {}", index + 1);
                    let down_ctx = ctx.clone();
                    let (down_points, down_canvas_points, down_target, down_envelope) = (
                        saved_points.clone(),
                        canvas_points.clone(),
                        target.clone(),
                        envelope.clone(),
                    );
                    let motion_ctx = ctx.clone();
                    let pointer_motion = Rc::new(move |pointer: &web_sys::PointerEvent, released: bool| {
                        let Some(active) = motion_ctx
                            .arbiter
                            .outline_drag()
                            .filter(|drag| drag.pointer_id == pointer.pointer_id())
                        else {
                            return;
                        };
                        pointer.stop_propagation();
                        if released {
                            finish_outline_point(&motion_ctx, pointer, active);
                        } else {
                            move_outline_point(&motion_ctx, pointer);
                        }
                    });
                    let key_ctx = ctx.clone();
                    let (key_target, key_points) = (target.clone(), saved_points.clone());
                    rsx! {
                        circle {
                            key: "{point_id}",
                            class: if is_selected { "cursor-grab fill-primary stroke-card stroke-[1.2] [pointer-events:all] focus-visible:stroke-primary focus-visible:stroke-[1.7]" } else { "cursor-grab fill-card stroke-primary stroke-[0.9] [pointer-events:all] focus-visible:stroke-primary focus-visible:stroke-[1.7]" },
                            role: "button",
                            tabindex: "0",
                            "aria-label": "{point_label}",
                            "aria-pressed": "{is_selected}",
                            cx: "{point.x}", cy: "{point.y}", r: "{handle_radius}",
                            onfocus: move |_| point_selected.write().selected_point = index,
                            onpointerdown: move |event: PointerEvent| {
                                let Some(pointer) = event.data().try_as_web_event() else { return; };
                                if !enabled || pointer.button() != 0 || down_ctx.arbiter.current().is_some() { return; }
                                let Some(handle) = pointer.target().and_then(|target| target.dyn_into::<SvgElement>().ok())
                                    .filter(|target| target.get_attribute("role").as_deref() == Some("button")) else {
                                    return;
                                };
                                let id = pointer.pointer_id();
                                let seed = GestureSeed::OutlineHandle(OutlineHandleSeed {
                                    point_index: index,
                                    points: down_points.clone(),
                                    preview_points: down_canvas_points.clone(),
                                    original_world: down_canvas_points[index],
                                    target: down_target.clone(),
                                    anchor,
                                    envelope: down_envelope.clone(),
                                    transaction_id: format!("outline-drag-{}", down_ctx.runtime.operation().0),
                                });
                                if !down_ctx.arbiter.handle(CanvasEvent::PointerDown {
                                    id, client: client_point(&pointer), control: pointer.ctrl_key(), seed: Some(seed),
                                }).is_empty()
                                    && down_ctx.svg.borrow().as_ref().is_some_and(|surface| surface.set_pointer_capture(id).is_ok())
                                {
                                    pointer.prevent_default();
                                    focus_svg(&handle);
                                    point_selected.write().selected_point = index;
                                    let cancel_ctx = down_ctx.clone();
                                    down_ctx.arbiter.on_cancel(move || {
                                        if let Some(canceled) = cancel_ctx.arbiter.take_canceled_outline_drag() {
                                            release_capture(&cancel_ctx.svg, canceled.pointer_id);
                                            clear_outline_preview_and_chrome(&cancel_ctx.runtime, &canceled, cancel_ctx.preview, cancel_ctx.guides);
                                        }
                                    });
                                    let motion = pointer_motion.clone();
                                    down_ctx.arbiter.on_pointer(move |pointer, released| motion(pointer, released));
                                } else {
                                    down_ctx.arbiter.end(CanvasMode::DraggingOutlineHandle);
                                }
                            },
                            onkeydown: move |event: KeyboardEvent| {
                                let Some(key) = event.data().try_as_web_event() else { return; };
                                if key.key() == "Escape" && key_ctx.arbiter.outline_drag().is_some() {
                                    key.prevent_default(); key.stop_propagation();
                                    key_ctx.arbiter.handle(CanvasEvent::KeyDown {
                                        key: CanvasKey::Escape, layout: true, canvas: true, typing: false, prevented: false,
                                    });
                                    return;
                                }
                                if key.key() == "Escape" {
                                    key.prevent_default(); key.stop_propagation();
                                    if key_ctx.projection.selected_context.read().as_ref().is_some_and(|sel| {
                                        sel.scope == key_ctx.projection.scope
                                            && matches!(&sel.context, crate::objects::TreeContext::Outline { board_id } if board_id == &key_ctx.projection.board_id)
                                    }) {
                                        let mut editing_points = key_ctx.projection.ui.editing_points;
                                        editing_points.set(false);
                                    }
                                    return;
                                }
                                if key_ctx.arbiter.current().is_some() || !enabled { return; }
                                let mut points = key_points.clone();
                                let step_x = grid.x.max(0.1) * if key.shift_key() { 10.0 } else { 1.0 };
                                let step_y = grid.y.max(0.1) * if key.shift_key() { 10.0 } else { 1.0 };
                                let world = anchor.map_or(point, |a| a.world(key_points[index]));
                                let (dx, dy) = match key.key().as_str() {
                                    "ArrowLeft" => (-step_x, 0.0), "ArrowRight" => (step_x, 0.0),
                                    "ArrowUp" => (0.0, step_y), "ArrowDown" => (0.0, -step_y),
                                    _ if key.key() == "Delete" || key.key() == "Backspace" => {
                                        key.prevent_default(); key.stop_propagation();
                                        if points.len() <= 3 { return; }
                                        points.remove(index); point_selected.write().selected_point = index.saturating_sub(1);
                                        key_ctx.projection.dispatch(OutlineActionKind::EditPerimeter {
                                            target: key_target.clone(), points, phase: EditPhase::Commit,
                                            transaction_id: format!("outline-delete-{}-{}", key_ctx.runtime.operation().0, index),
                                        });
                                        return;
                                    }
                                    _ => return,
                                };
                                key.prevent_default(); key.stop_propagation();
                                let next = Vec2 { x: world.x + dx, y: world.y + dy };
                                points[index] = anchor.map_or(next, |a| a.local(next));
                                key_ctx.projection.dispatch(OutlineActionKind::EditPerimeter {
                                    target: key_target.clone(), points, phase: EditPhase::Commit,
                                    transaction_id: format!("outline-nudge-{}-{}", key_ctx.runtime.operation().0, index),
                                });
                            }
                        }
                        text { class: "pointer-events-none fill-foreground stroke-card stroke-[0.3] text-[2.5px] [paint-order:stroke]", x: "{point.x + 2.0}", y: "{point.y + 2.0}", "aria-hidden": "true", "{index + 1}" }
                    }
                }
            }
        }
    }
}

#[component]
pub fn OutlineDraftCanvasOverlay(
    projection: OutlineInspectorProjection,
    runtime: OutlineRuntimeHandle,
    svg: Rc<RefCell<Option<SvgElement>>>,
    view_x: f64,
    view_y: f64,
    width: f64,
    height: f64,
    snap_settings: crate::objects::LayoutSnapSettings,
    pitch: Vec2,
    origins: Vec<crate::presentation::outline_snapping::Origin>,
) -> Element {
    let arbiter = use_context::<LayoutInteraction>();
    let Some(tool) = (projection.ui.drawing_operation)() else {
        return rsx! {};
    };
    let minimum_points = if tool == OutlineDrawTool::Connect {
        2
    } else {
        3
    };
    let mut drawing_operation = projection.ui.drawing_operation;
    let mut canvas_ui = projection.ui.canvas_ui;
    let paths = projection.snap_paths.clone();
    let origins = Rc::new(origins);
    let grid = grid_step(pitch, snap_settings.snap_fraction).unwrap_or_default();
    let geometry_snap = snap_settings.geometry_snap;
    let draft = canvas_ui.read().drawing_points.clone();
    let point_string = draft
        .iter()
        .map(|point| format!("{},{}", point.x, point.y))
        .collect::<Vec<_>>()
        .join(" ");
    let runtime = runtime.0.clone();
    let enabled = projection.enabled;
    let board_id = projection.board_id.clone();
    let revision = projection.revision;
    let finish_projection = projection.clone();
    let finish = {
        let mut canvas_ui = canvas_ui;
        let runtime = runtime.clone();
        move || {
            let draft = canvas_ui.read().drawing_points.clone();
            if draft.len() < minimum_points
                || matches!(tool, OutlineDrawTool::Polygon(_))
                    && polygon_signed_area(&draft).abs() < 1e-6
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
                    finish_projection.dispatch(OutlineActionKind::AddFeature {
                        context: finish_projection.selection_context.clone(),
                        feature,
                    });
                }
                OutlineDrawTool::Connect => {
                    finish_projection.dispatch(OutlineActionKind::AddConnection { points: draft });
                }
            }
            canvas_ui.write().drawing_points.clear();
            drawing_operation.set(None);
        }
    };
    rsx! {
        g { class: "[&_rect]:cursor-crosshair [&_rect]:[pointer-events:all] [&_rect:focus-visible]:outline-none [&_rect:focus-visible]:stroke-primary [&_rect:focus-visible]:stroke-[0.4]", "aria-label": match tool { OutlineDrawTool::Polygon(Operation::Add) => "Draw addition", OutlineDrawTool::Polygon(Operation::Subtract) => "Draw cutout", OutlineDrawTool::Connect => "Connect points" },
            rect {
                transform: "scale(1,-1)",
                x: "{view_x}", y: "{view_y}", width: "{width}", height: "{height}",
                fill: "transparent", tabindex: "0", role: "application",
                "aria-label": "Outline drawing canvas. Click to add points. Enter finishes; Escape cancels.",
                onpointerdown: move |event: PointerEvent| {
                    let Some(pointer) = event.data().try_as_web_event() else { return; };
                    if !enabled || pointer.button() != 0 || arbiter.current().is_some() { return; }
                    let Some(at) = crate::coordinates(&svg, &pointer, view_x, view_y, width, height) else { return; };
                    let path_points = canvas_ui.read().drawing_points.clone();
                    let context = crate::presentation::outline_snapping::Context { anchor: path_points.last().copied(), previous: path_points.iter().rev().nth(1).copied(), exclude: None, neighbor: None };
                    let px = svg.borrow().as_ref().map(|surface| surface.get_bounding_client_rect().width()).unwrap_or(1.0).max(1.0);
                    let tolerance = width / px * 7.0;
                    let mut snapped = crate::presentation::outline_snapping::snap_outline_point(at, context, &paths, crate::presentation::outline_snapping::Options { grid, tolerance, enabled: geometry_snap, free: pointer.alt_key() }, None);
                    if !pointer.alt_key() && geometry_snap && snapped.guides.is_empty()
                        && let Some((at, _)) = crate::presentation::outline_snapping::snap_origin(at, &origins, tolerance)
                    {
                        snapped.at = at;
                    }
                    if !path_points.iter().any(|point| (point.x-snapped.at.x).abs() < 1e-7 && (point.y-snapped.at.y).abs() < 1e-7) {
                        canvas_ui.write().drawing_points.push(snapped.at);
                    }
                    pointer.prevent_default();
                    if let Some(target) = pointer.target().and_then(|target| target.dyn_into::<SvgElement>().ok()) {
                        focus_svg(&target);
                    }
                },
                onkeydown: {
                    let mut finish = finish.clone();
                    let mut canvas_ui = canvas_ui;
                    let mut drawing_operation = drawing_operation;
                    move |event: KeyboardEvent| {
                        let Some(key) = event.data().try_as_web_event() else { return; };
                        match key.key().as_str() {
                            "Enter" => { key.prevent_default(); key.stop_propagation(); finish(); },
                            "Escape" => { key.prevent_default(); key.stop_propagation(); canvas_ui.write().drawing_points.clear(); drawing_operation.set(None); },
                            "Backspace" | "Delete" => { key.prevent_default(); key.stop_propagation(); canvas_ui.write().drawing_points.pop(); },
                            _ => {}
                        }
                    }
                },
            }
            if !draft.is_empty() {
                polyline { class: "pointer-events-none fill-none stroke-primary stroke-[0.65] [stroke-dasharray:1.5_1]", points: "{point_string}" }
                for (index, point) in draft.iter().enumerate() {
                    circle { key: "outline-draft-point-{index}", class: "pointer-events-none fill-primary stroke-card stroke-[0.35]", cx: "{point.x}", cy: "{point.y}", r: "1.5" }
                }
            }
        }
    }
}

#[component]
pub(super) fn OutlineDimension(
    label: &'static str,
    value: f64,
    minimum: f64,
    editable: bool,
    on_commit: EventHandler<f64>,
) -> Element {
    rsx! {
        NumericField {
            label: label.to_owned(),
            value,
            min: minimum,
            unit: "mm",
            step: 0.1,
            disabled_reason: (!editable).then(|| "Editing is unavailable".to_owned()),
            on_commit,
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
    rsx! {
        NumericField {
            label,
            value,
            unit: "mm",
            step: 0.1,
            disabled_reason: (!editable).then(|| "Editing is unavailable".to_owned()),
            on_commit,
        }
    }
}
