//! The Layout canvas rail: labelled handle tokens over the selected matrix, row, column, key or
//! independent part. One overlay renders the rail and routes pointer drags through
//! [`LayoutInteraction`] into `CanvasInteraction`; pure gesture math lives in
//! `layout_core::rail_gesture`.
use super::TreeContext;
pub use super::layout_rail_tool::{LayoutTransformTool, ToolActivation, activate_transform_tool};
use super::layout_toolbar::{GestureSnapInputs, LayoutSnapSettings};
use crate::canvas_interaction::{
    ActiveRailDrag, CanvasEvent, GestureSeed, InteractionEffect, LayoutInteraction, RailBodyDrag,
    RailDragKind, RailDragSeed, client_point, focus_svg, release_capture,
};
use crate::matrix_relations;
use crate::matrix_transform_operation::{
    ArrowKey, MatrixDragAnchor, PlacementParameter, SceneBasis, column_angle, handle_rail,
    matrix_drag_anchor_pose, matrix_drag_transport,
};
use crate::outline_snapping::Origin;
use crate::runtime::Runtime;
use boardstudio_application::{AcceptedSnapshot, EditResolver, Event, Resolution};
use boardstudio_core_kernel::matrix::layout::evaluated_part_pose;
use boardstudio_model::model::{
    EditOperation, Matrix, MatrixScene, MatrixSplayAffect, Position, Vec2,
};
use boardstudio_web_layout_core::{
    rail_drag::drag_session_is_ready,
    rail_gesture::{
        PartAxis, RailGesture, RailInput, RailNudge, RailPreview, anchor_key_base_rotation,
        part_edit_guard,
    },
};
use boardstudio_web_ui_model::state::{FieldPreview, FieldPreviewSource, FieldPreviewState};
use boardstudio_web_ui_model::transform_feedback::{
    FeedbackDelta, FeedbackEvent, FeedbackState, advance_feedback,
};
use dioxus::prelude::*;
use dioxus_web::WebEventExt;
use std::{cell::RefCell, rc::Rc};
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::spawn_local;
use web_sys::SvgElement;

#[derive(Clone)]
pub struct LayoutTransformRuntime(pub Rc<Runtime>);

impl PartialEq for LayoutTransformRuntime {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

/// What the rail draws over: a matrix selection with its evaluated scene, or one independent part.
#[derive(Clone, PartialEq)]
pub enum RailTarget {
    Matrix {
        matrix: Rc<Matrix>,
        projection: Rc<MatrixScene>,
        context: TreeContext,
    },
    Part {
        part_id: String,
        snap: GestureSnapInputs,
        on_focus_half: EventHandler<()>,
    },
}

/// One token's presentation and what it starts.
#[derive(Clone)]
struct Token {
    tool: LayoutTransformTool,
    label: String,
    value: String,
    title: Option<String>,
    glyph_degrees: f64,
    guide: Option<(f64, f64)>,
    disabled: bool,
    nudge: Option<RailNudge>,
}

const GLYPH_ARROWS: &str = "M-5 0h10M-3 -2l-2 2 2 2M3 -2l2 2-2 2";
const GLYPH_FOUR: &str = "M0 -3.2v6.4M-3.2 0h6.4M-1.4 -1.8 0 -3.2l1.4 1.4M-1.4 1.8 0 3.2l1.4 -1.4M-1.8 -1.4-3.2 0l1.4 1.4M1.8 -1.4 3.2 0l-1.4 1.4";
const GLYPH_SPLAY: &str = "M-5 4A9 9 0 0 1 4 -5M1.5 -6.5 4 -5l-1.3 2.4M-5 4h2";
const GLYPH_BEND: &str = "M-4 5C-4 0-1-4 4-4M1.5-6 4-4 1.5-2";
const GLYPH_ROTATE: &str = "M4.5 -2A5 5 0 1 1 -2 -4.6M4.6 -5v3h-3";
pub(super) const GLYPH_ANCHOR: &str =
    "M0-2.5V5M-3-.5h6M-4.5 2.2a4.5 4.5 0 0 0 9 0M0-5.6a1.5 1.5 0 1 0 .01 0";

/// The token glyph path; the one-dimensional tools draw the axis arrow rotated by `degrees`.
fn glyph(tool: LayoutTransformTool, quick: bool, degrees: f64, stroke: &str) -> Element {
    let style = format!(
        "fill:none;stroke:{stroke};stroke-width:1.5px;stroke-linecap:round;stroke-linejoin:round;vector-effect:non-scaling-stroke"
    );
    use LayoutTransformTool as T;
    match (tool, quick) {
        (T::Stagger | T::Offset, false) | (T::Spread, _) => {
            rsx! { g { transform: "rotate({degrees})", path { d: "{GLYPH_ARROWS}", style: "{style}" } } }
        }
        (T::Stagger | T::Offset, true) | (T::Shift | T::Move, _) => {
            rsx! { g { transform: "scale(1.5)", path { d: "{GLYPH_FOUR}", style: "{style}" } } }
        }
        (T::Bend, _) => rsx! { path { d: "{GLYPH_BEND}", style: "{style}" } },
        (T::Splay, _) => rsx! { path { d: "{GLYPH_SPLAY}", style: "{style}" } },
        (T::Rotate, _) => rsx! { path { d: "{GLYPH_ROTATE}", style: "{style}" } },
        (T::Anchor, _) => rsx! { path { d: "{GLYPH_ANCHOR}", style: "{style}" } },
        (T::Origin, _) => rsx! {},
    }
}

/// A token's (fill, glyph stroke): primary while its drag is live, the selection colours while
/// hovered or focused, otherwise transparent with a muted glyph.
pub(super) fn token_paint(active: bool, lit: bool) -> (&'static str, &'static str) {
    if active {
        ("var(--primary)", "var(--primary-foreground)")
    } else if lit {
        ("var(--selected)", "var(--selected-foreground)")
    } else {
        ("transparent", "var(--muted-foreground)")
    }
}

/// The 18 px tile, its taller hit area, the glyph and the native title.
pub(super) fn token_face(fill: &str, glyph: Element, title: Option<&str>) -> Element {
    rsx! {
        rect { x: "-9", y: "-9", width: "18", height: "18", rx: "4", style: "fill:{fill};stroke:none" }
        rect { x: "-10", y: "-12", width: "20", height: "24", style: "fill:transparent;stroke:none;pointer-events:all" }
        {glyph}
        if let Some(title) = title { title { "{title}" } }
    }
}

/// World units per screen pixel of the canvas surface.
pub(super) fn world_per_screen(
    surface: &Rc<RefCell<Option<SvgElement>>>,
    width: f64,
    height: f64,
) -> f64 {
    surface.borrow().as_ref().map_or(1.0, |svg| {
        let rect = svg.get_bounding_client_rect();
        if rect.width() > 0.0 {
            (width / rect.width()).max(height / rect.height().max(1.0))
        } else {
            1.0
        }
    })
}

/// The rail centred over `points` (horizontally on their extent, just above the topmost).
pub(super) fn rail_over_points(
    points: &[Vec2],
    count: usize,
    px: f64,
) -> Option<handle_rail::WorldRailLayout> {
    if points.is_empty() {
        return None;
    }
    let min = points.iter().map(|p| p.x).fold(f64::INFINITY, f64::min);
    let max = points.iter().map(|p| p.x).fold(f64::NEG_INFINITY, f64::max);
    let top = points.iter().map(|p| p.y).fold(f64::NEG_INFINITY, f64::max);
    handle_rail::rail_world((min + max) / 2.0, top, count, px)
}

/// The rail's card backdrop, drawn once behind the tokens.
pub(super) fn rail_backdrop(
    layout: &handle_rail::WorldRailLayout,
    count: usize,
    px: f64,
) -> Element {
    let width = handle_rail::rail_width(count);
    let (cx, cy) = (
        layout.rail.x + layout.rail.w / 2.0,
        layout.rail.y + layout.rail.h / 2.0,
    );
    rsx! {
        g { "aria-hidden": "true", transform: "translate({cx} {cy}) scale({px} {-px})", style: "pointer-events:none",
            rect { x: "{-width / 2.0}", y: "-11", width: "{width}", height: "22", rx: "6",
                style: "fill:var(--card);stroke:var(--border-strong);stroke-width:1px;vector-effect:non-scaling-stroke;filter:drop-shadow(0 2px 4px rgb(24 35 49 / 12%))" }
        }
    }
}

type SeedMaker = Rc<dyn Fn(&AcceptedSnapshot, Vec2, bool, String, &str) -> Option<GestureSeed>>;

#[derive(Clone)]
struct RailView {
    runtime: Rc<Runtime>,
    surface: Rc<RefCell<Option<SvgElement>>>,
    interaction: LayoutInteraction,
    feedback: Signal<FeedbackState>,
    popup_at: Signal<Option<(f64, f64)>>,
    toast_gen: Signal<u64>,
    guide: Signal<Option<Vec2>>,
    view: (f64, f64, f64, f64),
    px: f64,
    pitch: Vec2,
    snap: LayoutSnapSettings,
    origins: Rc<Vec<Origin>>,
    part_snap: Option<GestureSnapInputs>,
}

/// Raise one commit toast and arm its 1.8 s expiry; the timer only touches live signals.
pub(super) fn raise_toast(
    feedback: Signal<FeedbackState>,
    mut generation: Signal<u64>,
    value: String,
    delta: FeedbackDelta,
) {
    feedback_event(feedback, FeedbackEvent::Commit { value, delta });
    generation.set(generation() + 1);
    let seen = generation();
    let mut feedback = feedback;
    spawn_local(async move {
        gloo_timers::future::TimeoutFuture::new(1800).await;
        if generation.try_peek().is_ok_and(|current| *current == seen)
            && let Ok(mut state) = feedback.try_write()
        {
            *state = advance_feedback(&state, FeedbackEvent::Expire);
        }
    });
}

pub(super) fn feedback_event(mut feedback: Signal<FeedbackState>, event: FeedbackEvent) {
    let next = advance_feedback(&feedback(), event);
    feedback.set(next);
}

fn world_point(view: &RailView, pointer: &web_sys::PointerEvent) -> Option<(Vec2, f64)> {
    let (x, y, w, h) = view.view;
    crate::presentation::coordinates_with_scale(&view.surface, pointer, x, y, w, h)
}

fn rail_input<'a>(
    view: &'a RailView,
    start_client: (f64, f64),
    pointer: &web_sys::PointerEvent,
    point: Vec2,
) -> Option<RailInput<'a>> {
    let rect = view.surface.borrow().as_ref()?.get_bounding_client_rect();
    if rect.width() <= 0.0 || rect.height() <= 0.0 {
        return None;
    }
    let (_, _, w, h) = view.view;
    let tolerance = w / rect.width() * 8.0;
    Some(RailInput {
        point,
        world_delta: Vec2 {
            x: (f64::from(pointer.client_x()) - start_client.0) * w / rect.width(),
            y: -(f64::from(pointer.client_y()) - start_client.1) * h / rect.height(),
        },
        alt: pointer.alt_key(),
        pitch: view.part_snap.map_or(view.pitch, |snap| snap.pitch),
        snap_fraction: view
            .part_snap
            .map_or(view.snap.snap_fraction, |snap| snap.snap_fraction),
        geometry_snap: view
            .part_snap
            .map_or(view.snap.geometry_snap, |snap| snap.geometry_snap),
        origins: &view.origins,
        tolerance,
    })
}

fn clear_rail_preview(runtime: &Rc<Runtime>, active: &ActiveRailDrag) {
    runtime.submit(Event::ClearPreview {
        operation_id: runtime.operation(),
        token: active.token,
        revision: active.revision,
        board_id: active.board_id.clone(),
        transaction_id: active.transaction_id.clone(),
    });
}

fn clear_rail_chrome(view: &RailView) {
    let (mut popup_at, mut guide) = (view.popup_at, view.guide);
    if let Ok(mut popup) = popup_at.try_write() {
        *popup = None;
    }
    if let Ok(mut point) = guide.try_write() {
        *point = None;
    }
}

fn cancel_active_rail_drag(view: &RailView, active: &ActiveRailDrag) {
    match &active.kind {
        RailDragKind::Body(body) if body.last_sample.is_some() => {
            view.runtime.submit(Event::GestureCancel {
                pointer_id: i64::from(active.pointer_id),
            });
        }
        _ if active.last_preview.is_some() => clear_rail_preview(&view.runtime, active),
        _ => {}
    }
    release_capture(&view.surface, active.pointer_id);
    clear_rail_chrome(view);
    let mut feedback = view.feedback;
    if let Ok(mut state) = feedback.try_write() {
        *state = advance_feedback(&state, FeedbackEvent::Dismiss);
    }
}

fn abandon_rail_drag(interaction: LayoutInteraction) {
    if let Some(active) = interaction.rail_drag() {
        interaction.handle(CanvasEvent::PointerCancel(active.pointer_id));
    }
}

fn start_rail_drag(view: &RailView, pointer: &web_sys::PointerEvent, make: &SeedMaker) {
    if pointer.button() != 0 || view.interaction.current().is_some() {
        return;
    }
    let model = view.runtime.model();
    let (Some(snapshot), Some(scope)) = (model.accepted.as_ref(), view.runtime.scope()) else {
        return;
    };
    if !drag_session_is_ready(&model, Some(snapshot.document.revision)) {
        return;
    }
    let id = pointer.pointer_id();
    let client = client_point(pointer);
    // The latch below trusts `control_down` as well as this event, and the
    // handle stops propagation, so stale modifiers must be synced first.
    view.interaction.sync_control(pointer.ctrl_key());
    let quick_offset = pointer.ctrl_key() || view.interaction.peek_control_down();
    let Some((point, _)) = world_point(view, pointer) else {
        return;
    };
    let transaction_id = format!("layout-transform-{}", view.runtime.operation().0);
    let Some(seed) = make(
        snapshot,
        point,
        quick_offset,
        transaction_id,
        &scope.board_id,
    ) else {
        return;
    };
    let mode = seed.mode();
    let Some(capture) = view.surface.borrow().clone() else {
        return;
    };
    if view.interaction.handle(CanvasEvent::PointerDown {
        id,
        client,
        control: pointer.ctrl_key(),
        seed: Some(seed),
    }) != [InteractionEffect::CapturePointer(id)]
    {
        return;
    }
    if capture.set_pointer_capture(id).is_err() {
        view.interaction.end(mode);
        return;
    }
    let handle = pointer
        .target()
        .and_then(|target| target.dyn_into::<web_sys::Element>().ok())
        .and_then(|target| target.closest("[data-transform-handle]").ok().flatten())
        .and_then(|target| target.dyn_into::<SvgElement>().ok());
    pointer.prevent_default();
    pointer.stop_propagation();
    focus_svg(handle.as_ref().unwrap_or(&capture));
    let cancel_view = view.clone();
    view.interaction.on_cancel(move || {
        if let Some(canceled) = cancel_view.interaction.take_canceled_rail_drag() {
            cancel_active_rail_drag(&cancel_view, &canceled);
        }
    });
    let pointer_view = view.clone();
    view.interaction.on_pointer(move |pointer, released| {
        if released {
            finish_rail_drag(&pointer_view, pointer);
        } else {
            move_rail_drag(&pointer_view, pointer);
        }
    });
    let mut popup_at = view.popup_at;
    popup_at.set(Some((point.x + 18.0 * view.px, point.y - 18.0 * view.px)));
}

fn sample_rail_gesture(
    view: &RailView,
    active: &ActiveRailDrag,
    pointer: &web_sys::PointerEvent,
) -> Option<(Vec2, RailPreview)> {
    let RailDragKind::Gesture(gesture) = &active.kind else {
        return None;
    };
    let (point, _) = world_point(view, pointer)?;
    let preview = gesture.sample(&rail_input(view, active.start_client, pointer, point)?)?;
    Some((point, preview))
}

fn move_rail_drag(view: &RailView, pointer: &web_sys::PointerEvent) {
    let Some(active) = view
        .interaction
        .rail_drag()
        .filter(|active| active.pointer_id == pointer.pointer_id())
    else {
        return;
    };
    pointer.prevent_default();
    pointer.stop_propagation();
    if let RailDragKind::Body(body) = &active.kind {
        body_rail_moved(view, pointer, &active, body, false);
        return;
    }
    let Some((point, preview)) = sample_rail_gesture(view, &active, pointer) else {
        return;
    };
    let target_id = match &active.kind {
        RailDragKind::Gesture(gesture) => gesture.target_id().to_owned(),
        RailDragKind::Body(_) => String::new(),
    };
    view.interaction
        .note_rail_preview(Some((preview.value.clone(), preview.delta.clone())));
    let (mut guide, mut popup_at) = (view.guide, view.popup_at);
    guide.set(preview.guide);
    for effect in view.interaction.handle(CanvasEvent::PointerMove {
        id: pointer.pointer_id(),
        client: client_point(pointer),
        operation: Some(preview.operation),
    }) {
        if let InteractionEffect::Preview(operation) = effect {
            view.runtime.submit(Event::PreviewEdit {
                operation_id: view.runtime.operation(),
                transaction_id: active.transaction_id.clone(),
                target_ids: vec![target_id.clone()],
                operation,
            });
        }
    }
    feedback_event(
        view.feedback,
        FeedbackEvent::Drag {
            name: active.tool_label.to_owned(),
            value: preview.value,
            delta: preview.delta,
        },
    );
    popup_at.set(Some((point.x + 18.0 * view.px, point.y - 18.0 * view.px)));
}

/// A detached part's Move drives the session body-drag gesture past the shipped threshold.
fn body_rail_moved(
    view: &RailView,
    pointer: &web_sys::PointerEvent,
    active: &ActiveRailDrag,
    body: &RailBodyDrag,
    finishing: bool,
) {
    let client = client_point(pointer);
    if client == active.start_client && body.last_sample.is_none() {
        return;
    }
    let world = world_point(view, pointer);
    let sample = world.map(|(_, scale)| {
        (
            Vec2 {
                x: body.start_at.x + (client.0 - active.start_client.0) / scale,
                y: body.start_at.y - (client.1 - active.start_client.1) / scale,
            },
            scale,
            pointer.alt_key(),
        )
    });
    let Some((at, scale, alt)) = sample.or(body.last_sample.filter(|_| finishing)) else {
        return;
    };
    let pointer_id = i64::from(active.pointer_id);
    let part = vec![Position {
        id: body.part_id.clone(),
        at,
    }];
    let snap = view.part_snap.unwrap_or(GestureSnapInputs {
        pitch: view.pitch,
        snap_fraction: view.snap.snap_fraction,
        geometry_snap: view.snap.geometry_snap,
        gap: None,
    });
    if body.last_sample.is_none() {
        view.runtime.submit(Event::GestureBegin {
            operation_id: view.runtime.operation(),
            pointer_id,
            target_ids: vec![body.part_id.clone()],
            transaction_id: active.transaction_id.clone(),
            start: vec![Position {
                id: body.part_id.clone(),
                at: body.start_at,
            }],
            pitch: snap.pitch,
            snap_fraction: snap.snap_fraction,
            geometry_snap: snap.geometry_snap,
            gap: snap.gap,
            pixels_per_world: Some(scale),
            alt,
        });
        let admitted = view.runtime.model().gesture.as_ref().is_some_and(|g| {
            g.pointer_id == pointer_id && g.target_ids == vec![body.part_id.clone()]
        });
        if !admitted {
            view.interaction
                .handle(CanvasEvent::PointerCancel(active.pointer_id));
            return;
        }
    }
    view.interaction
        .note_rail_body_sample(Some((at, scale, alt)));
    if finishing {
        view.runtime.submit(Event::GestureEnd {
            pointer_id,
            final_positions: part,
            pixels_per_world: Some(scale),
            alt,
        });
    } else {
        view.runtime.submit(Event::GestureSample {
            pointer_id,
            positions: part,
            pixels_per_world: Some(scale),
            alt,
        });
    }
    let delta = FeedbackDelta::Mm2(at.x - body.start_at.x, at.y - body.start_at.y);
    let value = format!("X {:.2}, Y {:.2} mm", at.x, at.y);
    let mut popup_at = view.popup_at;
    if finishing {
        raise_toast(
            view.feedback,
            view.toast_gen,
            format!("{} {value}", active.tool_label),
            delta,
        );
    } else if let Some((point, _)) = world {
        feedback_event(
            view.feedback,
            FeedbackEvent::Drag {
                name: active.tool_label.to_owned(),
                value,
                delta,
            },
        );
        popup_at.set(Some((point.x + 18.0 * view.px, point.y - 18.0 * view.px)));
    }
}

fn commit_rail_edit(
    runtime: &Rc<Runtime>,
    target: &str,
    transaction_id: &str,
    expected_revision: Option<u64>,
    operation: EditOperation,
) {
    let (target, transaction_id) = (target.to_owned(), transaction_id.to_owned());
    let result = runtime.commit(
        "layout-transform-drag",
        EditResolver::new(
            "layout-transform-drag",
            move |accepted: &AcceptedSnapshot| {
                if expected_revision.is_some_and(|rev| accepted.document.revision != rev) {
                    return Resolution::Retire(
                        "The layout changed before this drag landed.".into(),
                    );
                }
                Resolution::submit_with_transaction_id(
                    vec![target.clone()],
                    transaction_id.clone(),
                    operation.clone(),
                )
            },
        ),
    );
    report_rail_commit(runtime, result, false);
}

/// Report a drag commit (pointer drops never set `unlaid`; keyboard nudges do).
fn report_rail_commit(runtime: &Runtime, result: Result<AcceptedSnapshot, String>, unlaid: bool) {
    if unlaid {
        runtime.report("The matrix is anchored to a key that is not laid out.");
    } else if let Err(message) = result {
        runtime.report(message);
    }
}

fn finish_rail_drag(view: &RailView, pointer: &web_sys::PointerEvent) {
    let Some(active) = view
        .interaction
        .rail_drag()
        .filter(|active| active.pointer_id == pointer.pointer_id())
    else {
        return;
    };
    pointer.prevent_default();
    pointer.stop_propagation();
    if let RailDragKind::Body(body) = &active.kind {
        body_rail_moved(view, pointer, &active, body, true);
        view.interaction.handle(CanvasEvent::PointerUp {
            id: pointer.pointer_id(),
            operation: None,
        });
        release_capture(&view.surface, active.pointer_id);
        clear_rail_chrome(view);
        return;
    }
    let sampled = sample_rail_gesture(view, &active, pointer);
    let effective_preview = sampled
        .as_ref()
        .map(|(_, preview)| (preview.value.clone(), preview.delta.clone()))
        .or_else(|| active.last_preview.clone());
    let target = match &active.kind {
        RailDragKind::Gesture(gesture) => gesture.target_id().to_owned(),
        RailDragKind::Body(_) => String::new(),
    };
    let finish = view
        .interaction
        .handle(CanvasEvent::PointerUp {
            id: pointer.pointer_id(),
            operation: sampled.map(|(_, preview)| preview.operation),
        })
        .into_iter()
        .find(|effect| matches!(effect, InteractionEffect::Commit { .. }));
    match (finish, effective_preview) {
        (
            Some(InteractionEffect::Commit {
                operation,
                needs_preview,
            }),
            Some((value, delta)),
        ) if !delta.is_zero() => {
            if needs_preview {
                view.runtime.submit(Event::PreviewEdit {
                    operation_id: view.runtime.operation(),
                    transaction_id: active.transaction_id.clone(),
                    target_ids: vec![target.clone()],
                    operation: operation.clone(),
                });
            }
            commit_rail_edit(
                &view.runtime,
                &target,
                &active.transaction_id,
                Some(active.revision),
                operation,
            );
            raise_toast(
                view.feedback,
                view.toast_gen,
                format!("{} {value}", active.tool_label),
                delta,
            );
        }
        (_, preview) => {
            clear_rail_preview(&view.runtime, &active);
            if let Some((value, _)) = preview {
                feedback_event(
                    view.feedback,
                    FeedbackEvent::Choose {
                        name: active.tool_label.to_owned(),
                        value,
                    },
                );
            }
        }
    }
    release_capture(&view.surface, active.pointer_id);
    clear_rail_chrome(view);
}

// Token construction

fn context_column(context: &TreeContext) -> Option<u32> {
    match context {
        TreeContext::Column { column, .. } | TreeContext::Key { column, .. } => Some(*column),
        TreeContext::Component { column, .. } => *column,
        _ => None,
    }
}

/// The anchor frame a matrix drag inverse-solves through; `Err` when the matrix is anchored to
/// a key that is not laid out, so the drag never solves through a stale frame.
fn matrix_drag_anchor(
    accepted: &AcceptedSnapshot,
    matrix: &Matrix,
) -> Result<Option<MatrixDragAnchor>, ()> {
    let Some(anchor) = matrix.anchor.as_ref() else {
        return Ok(None);
    };
    let key_pose = matrix_drag_anchor_pose(&accepted.scene.matrix_scenes, anchor).ok_or(())?;
    Ok(Some(MatrixDragAnchor {
        key_pose,
        follow: anchor.follow,
        transport: matrix_drag_transport(&accepted.document, &matrix.id),
    }))
}

/// The Offset-like parameter a Shift or Bend token edits, or `None` when the context has no row/key.
fn carried(context: &TreeContext, bend: bool) -> Option<PlacementParameter> {
    match (context, bend) {
        (TreeContext::Row { row, .. }, false) => Some(PlacementParameter::RowShift { row: *row }),
        (TreeContext::Key { row, column, .. }, false) => Some(PlacementParameter::KeyShift {
            row: *row,
            column: *column,
        }),
        (TreeContext::Row { row, .. }, true) => Some(PlacementParameter::RowBend { row: *row }),
        (TreeContext::Key { row, column, .. }, true) => Some(PlacementParameter::KeyBend {
            row: *row,
            column: *column,
        }),
        _ => None,
    }
}

fn rail_seed(
    tool: LayoutTransformTool,
    kind: RailDragKind,
    matrix: bool,
    accepted: &AcceptedSnapshot,
    transaction_id: String,
    board_id: &str,
) -> GestureSeed {
    let seed = RailDragSeed {
        transaction_id,
        tool_label: tool.label(),
        kind,
        token: accepted.token,
        revision: accepted.document.revision,
        board_id: board_id.to_owned(),
    };
    if matrix {
        GestureSeed::MatrixRail(seed)
    } else {
        GestureSeed::PartRail(seed)
    }
}

/// The matrix tokens for `tools`, each with the gesture seed builder it starts.
fn matrix_tokens(
    tools: &[LayoutTransformTool],
    matrix: &Rc<Matrix>,
    projection: &Rc<MatrixScene>,
    context: &TreeContext,
    affect: &MatrixSplayAffect,
    control: bool,
) -> Vec<(Token, Option<SeedMaker>)> {
    use LayoutTransformTool as T;
    let column = context_column(context).unwrap_or(0);
    let row = match context {
        TreeContext::Row { row, .. } => Some(*row),
        _ => None,
    };
    let basis = projection
        .columns
        .iter()
        .find(|basis| basis.column == column);
    let stagger_axis = basis.map_or(Vec2 { x: 0.0, y: 1.0 }, |basis| basis.stagger_axis);
    let axis_degrees = (-stagger_axis.y).atan2(stagger_axis.x).to_degrees();
    let offset_parameter = row.map_or(PlacementParameter::ColumnOffset { column }, |row| {
        PlacementParameter::RowOffset { row }
    });
    let value_of = |parameter| {
        boardstudio_web_layout_core::rail_gesture::format_placement_value(
            boardstudio_web_layout_core::rail_gesture::placement_current(matrix, parameter),
        )
    };
    let placement = |tool: T, parameter: PlacementParameter| -> SeedMaker {
        let (matrix, projection) = (matrix.clone(), projection.clone());
        Rc::new(move |accepted, _, _, tx, board_id| {
            let basis = SceneBasis::for_parameter(&projection, parameter)?;
            Some(rail_seed(
                tool,
                RailDragKind::Gesture(RailGesture::Placement {
                    matrix: (*matrix).clone(),
                    parameter,
                    basis,
                }),
                true,
                accepted,
                tx,
                board_id,
            ))
        })
    };
    struct MatrixTokenSpec {
        label: &'static str,
        value: String,
        title: Option<String>,
        guide: Option<(f64, f64)>,
        nudge: Option<RailNudge>,
        disabled: bool,
        maker: SeedMaker,
    }

    let mut tokens = Vec::new();
    for &tool in tools {
        // One armed token row: the label/value/title chrome plus its guide, nudge,
        // availability and start maker. Every tool below shares this shape.
        let make_token = |spec: MatrixTokenSpec| {
            Some((
                Token {
                    tool,
                    label: spec.label.to_owned(),
                    value: spec.value,
                    title: spec.title,
                    glyph_degrees: axis_degrees,
                    guide: spec.guide,
                    nudge: spec.nudge,
                    disabled: spec.disabled,
                },
                Some(spec.maker),
            ))
        };
        let entry = match tool {
            T::Stagger => {
                let parameter = PlacementParameter::Stagger { column };
                let offset = PlacementParameter::ColumnOffset { column };
                // Ctrl latched at pointer-down turns the whole drag into a free column offset.
                let (quick, normal) = (placement(tool, offset), placement(tool, parameter));
                let maker: SeedMaker = Rc::new(move |accepted, point, ctrl, tx, board_id| {
                    if ctrl {
                        quick(accepted, point, ctrl, tx, board_id)
                    } else {
                        normal(accepted, point, ctrl, tx, board_id)
                    }
                });
                make_token(MatrixTokenSpec {
                    label: if control {
                        "Drag to offset column"
                    } else {
                        "Drag to stagger"
                    },
                    value: value_of(if control { offset } else { parameter }),
                    title: None,
                    guide: Some((stagger_axis.x, stagger_axis.y)),
                    nudge: (!control && row.is_none()).then_some(RailNudge::Placement(parameter)),
                    disabled: false,
                    maker,
                })
            }
            // Column 0 follows the pitch, so it has no Spread step to drag.
            T::Spread if column == 0 && row.is_none() => None,
            T::Spread => {
                let parameter = PlacementParameter::ColumnSpread { column };
                make_token(MatrixTokenSpec {
                    label: "Drag to spread",
                    value: value_of(parameter),
                    title: Some(format!("Drag to spread · {}", value_of(parameter))),
                    guide: Some((1.0, 0.0)),
                    nudge: Some(RailNudge::Placement(parameter)),
                    disabled: false,
                    maker: placement(tool, parameter),
                })
            }
            T::Offset => make_token(MatrixTokenSpec {
                label: if row.is_some() {
                    "Drag to offset row"
                } else {
                    "Drag to offset column"
                },
                value: value_of(offset_parameter),
                title: None,
                guide: Some((stagger_axis.x, stagger_axis.y)),
                nudge: Some(RailNudge::Placement(offset_parameter)),
                disabled: false,
                maker: placement(tool, offset_parameter),
            }),
            T::Shift => carried(context, false).and_then(|parameter| {
                make_token(MatrixTokenSpec {
                    label: if row.is_some() {
                        "Drag to shift row"
                    } else {
                        "Drag to shift key"
                    },
                    value: value_of(parameter),
                    title: None,
                    guide: None,
                    nudge: Some(RailNudge::Placement(parameter)),
                    disabled: false,
                    maker: placement(tool, parameter),
                })
            }),
            T::Bend => carried(context, true).and_then(|parameter| {
                make_token(MatrixTokenSpec {
                    label: if row.is_some() {
                        "Drag to bend row"
                    } else {
                        "Drag to bend key"
                    },
                    value: value_of(parameter),
                    title: None,
                    guide: None,
                    nudge: Some(RailNudge::Placement(parameter)),
                    disabled: false,
                    maker: placement(tool, parameter),
                })
            }),
            // Origin stays a point on the canvas (`origin_handle`) and never joins the rail.
            T::Origin => None,
            T::Splay => basis.cloned().and_then(|basis| {
                let (matrix, affect_for_start) = (matrix.clone(), affect.clone());
                let maker: SeedMaker = Rc::new(move |accepted, point, ctrl, tx, board_id| {
                    let origin = basis.splay_origin;
                    Some(rail_seed(
                        tool,
                        RailDragKind::Gesture(RailGesture::Splay {
                            matrix: (*matrix).clone(),
                            column,
                            origin,
                            start_angle: (point.y - origin.y).atan2(point.x - origin.x),
                            swing: ctrl,
                            affect: affect_for_start.clone(),
                            column_base: basis.base,
                        }),
                        true,
                        accepted,
                        tx,
                        board_id,
                    ))
                });
                let (splay, stagger_rotation) = (basis.splay_angle, basis.rotation);
                let angle = column_angle(splay, stagger_rotation, affect);
                make_token(MatrixTokenSpec {
                    label: "Drag to splay",
                    value: format!("{angle:.1}°"),
                    title: Some(format!("Drag to splay · {angle:.0}°")),
                    guide: None,
                    nudge: Some(RailNudge::SplayAngle {
                        column,
                        affect: affect.clone(),
                    }),
                    disabled: false,
                    maker,
                })
            }),
            T::Move | T::Rotate => {
                let (matrix, rotate) = (matrix.clone(), tool == T::Rotate);
                let followed = matrix
                    .anchor
                    .as_ref()
                    .is_none_or(|anchor| anchor.follow.rotation);
                let (center, value) = (
                    projection
                        .cells
                        .iter()
                        .find(|cell| cell.enabled)
                        .or(projection.cells.first())
                        .map_or(matrix.origin, |cell| cell.pose.at),
                    if rotate {
                        format!("{:.2} degrees", matrix.rotation.unwrap_or(0.0))
                    } else {
                        format!("X {:.2} mm, Y {:.2} mm", matrix.origin.x, matrix.origin.y)
                    },
                );
                let maker: SeedMaker = Rc::new(move |accepted, point, _, tx, board_id| {
                    let anchor = matrix_drag_anchor(accepted, &matrix).ok()?;
                    let gesture = if rotate {
                        RailGesture::MatrixRotate {
                            matrix: (*matrix).clone(),
                            anchor,
                            center,
                            start_angle: (point.y - center.y).atan2(point.x - center.x),
                        }
                    } else {
                        RailGesture::MatrixMove {
                            matrix: (*matrix).clone(),
                            anchor,
                        }
                    };
                    Some(rail_seed(
                        tool,
                        RailDragKind::Gesture(gesture),
                        true,
                        accepted,
                        tx,
                        board_id,
                    ))
                });
                let nudge = if rotate {
                    RailNudge::MatrixRotate
                } else {
                    RailNudge::MatrixMove { anchor: None }
                };
                make_token(MatrixTokenSpec {
                    label: if rotate {
                        "Drag to rotate matrix"
                    } else {
                        "Drag to move matrix"
                    },
                    value,
                    title: None,
                    guide: None,
                    nudge: Some(nudge),
                    disabled: rotate && !followed,
                    maker,
                })
            }
            T::Anchor => None,
        };
        tokens.extend(entry);
    }
    tokens
}

/// The independent part's tokens: Move and Rotate edit its local Offset and Rotation (or pose),
/// Spread and Stagger the values carried from its anchor key.
fn part_tokens(
    tools: &[LayoutTransformTool],
    part_id: &str,
    accepted: &AcceptedSnapshot,
) -> (Vec<(Token, Option<SeedMaker>)>, Vec2, f64) {
    use LayoutTransformTool as T;
    let document = &accepted.document;
    let Some(part) = document.parts.iter().find(|part| part.id == part_id) else {
        return (Vec::new(), Vec2::default(), 0.0);
    };
    let evaluated = evaluated_part_pose(document, part);
    let anchor = part.anchor.clone();
    let base = anchor
        .as_ref()
        .and_then(|anchor| anchor_key_base_rotation(document, anchor))
        .unwrap_or(0.0);
    let (spread, stagger, offset, rotation) = anchor
        .as_ref()
        .map_or((0.0, 0.0, Vec2::default(), evaluated.rotation), |a| {
            (a.spread, a.stagger, a.offset, a.rotation)
        });
    let mut tokens = Vec::new();
    for &tool in tools {
        let pos = if anchor.is_some() {
            offset
        } else {
            evaluated.at
        };
        let (label, value, title, glyph_degrees, guide, axis) = match tool {
            T::Move => (
                "Drag to move part",
                format!("X {:.2}, Y {:.2} mm", pos.x, pos.y),
                true,
                0.0,
                None,
                PartAxis::Offset,
            ),
            T::Rotate => (
                "Drag to rotate part",
                format!("{rotation:.1}°"),
                true,
                0.0,
                None,
                PartAxis::Rotation,
            ),
            T::Spread => (
                "Drag to spread",
                format!("{spread:.2} mm"),
                true,
                (-base.to_radians().sin())
                    .atan2(base.to_radians().cos())
                    .to_degrees(),
                Some((base.to_radians().cos(), base.to_radians().sin())),
                PartAxis::Spread,
            ),
            T::Stagger => (
                "Drag to stagger",
                format!("{stagger:.2} mm"),
                false,
                base.to_radians()
                    .cos()
                    .atan2(-base.to_radians().sin())
                    .to_degrees(),
                Some((-base.to_radians().sin(), base.to_radians().cos())),
                PartAxis::Stagger,
            ),
            _ => continue,
        };
        let (id, part_anchor) = (part_id.to_owned(), anchor.clone());
        let maker: SeedMaker = Rc::new(move |accepted, point, _, tx, board_id| {
            let document = &accepted.document;
            let part = document.parts.iter().find(|part| part.id == id)?;
            if part_edit_guard(document, &id, part_anchor.is_some()).is_some() {
                return None;
            }
            let evaluated = evaluated_part_pose(document, part);
            let sweep_start = (point.y - evaluated.at.y).atan2(point.x - evaluated.at.x);
            let kind = match (&part.anchor, axis) {
                (None, PartAxis::Offset) => RailDragKind::Body(RailBodyDrag {
                    part_id: id.clone(),
                    start_at: evaluated.at,
                    last_sample: None,
                }),
                (None, _) => RailDragKind::Gesture(RailGesture::PartRotate {
                    part_id: id.clone(),
                    center: evaluated.at,
                    start_angle: sweep_start,
                    start_rotation: evaluated.rotation,
                }),
                (Some(anchor), axis) => RailDragKind::Gesture(RailGesture::PartValue {
                    part_id: id.clone(),
                    axis,
                    anchor: anchor.clone(),
                    base_rotation: anchor_key_base_rotation(document, anchor).unwrap_or(0.0),
                    center: evaluated.at,
                    start_angle: sweep_start,
                }),
            };
            Some(rail_seed(tool, kind, false, accepted, tx, board_id))
        });
        let token = Token {
            tool,
            label: label.to_owned(),
            value: value.clone(),
            title: title.then(|| format!("{label} · {value}")),
            glyph_degrees,
            guide,
            disabled: false,
            nudge: None,
        };
        tokens.push((token, Some(maker)));
    }
    (
        tokens,
        evaluated.at,
        part_courtyard_top(document, &part.definition_id),
    )
}

/// The top of a part's courtyard outline, shared with the anchor pick.
pub(super) fn part_courtyard_top(
    document: &boardstudio_model::model::ProjectDoc,
    definition_id: &str,
) -> f64 {
    document
        .definitions
        .iter()
        .find(|definition| definition.id == definition_id)
        .map(|definition| {
            definition
                .courtyard
                .iter()
                .map(|point| point.y)
                .fold(f64::NEG_INFINITY, f64::max)
        })
        .filter(|top| top.is_finite())
        .unwrap_or(0.0)
}

// ---------------------------------------------------------------------------------------------
// Overlay

#[component]
pub fn LayoutRailOverlay(
    runtime: LayoutTransformRuntime,
    surface: Rc<RefCell<Option<SvgElement>>>,
    target: RailTarget,
    tools: Vec<LayoutTransformTool>,
    snap_settings: LayoutSnapSettings,
    #[props(into)] snap_origins: Rc<Vec<Origin>>,
    splay_affect: Signal<MatrixSplayAffect>,
    #[props(default)] surface_size: Option<Signal<(f64, f64)>>,
    view_x: f64,
    view_y: f64,
    width: f64,
    height: f64,
) -> Element {
    let _ = surface_size.map(|size| size());
    let runtime = runtime.0;
    let interaction = use_context::<LayoutInteraction>();
    let _ = interaction.version();
    let control = interaction.control_down();
    let feedback = use_signal(FeedbackState::default);
    let popup_at = use_signal(|| None::<(f64, f64)>);
    let toast_gen = use_signal(|| 0_u64);
    let guide = use_signal(|| None::<Vec2>);
    let hovered = use_signal(|| None::<usize>);
    let px = world_per_screen(&surface, width, height);
    let accepted = runtime.model().accepted;
    let (pitch, part_snap) = match &target {
        RailTarget::Matrix { matrix, .. } => (matrix.pitch, None),
        RailTarget::Part { snap, .. } => (snap.pitch, Some(*snap)),
    };
    let rail_view = RailView {
        runtime: runtime.clone(),
        surface: surface.clone(),
        interaction,
        feedback,
        popup_at,
        toast_gen,
        guide,
        view: (view_x, view_y, width, height),
        px,
        pitch,
        snap: snap_settings,
        origins: snap_origins,
        part_snap,
    };
    use_drop(move || {
        abandon_rail_drag(interaction);
    });
    let context_key = format!(
        "{:?}",
        match &target {
            RailTarget::Matrix { context, .. } => Some(context.clone()),
            RailTarget::Part { part_id, .. } => Some(TreeContext::Component {
                part_id: Some(part_id.clone()),
                matrix_id: None,
                row: None,
                column: None,
                assembly_id: None
            }),
        }
    );
    use_effect(use_reactive((&context_key, &tools), move |_| {
        abandon_rail_drag(interaction);
    }));

    let Some(accepted) = accepted else {
        return rsx! {};
    };
    type RailSceneBuild = (
        Vec<(Token, Option<SeedMaker>)>,
        Vec<Vec2>,
        Option<(Rc<Matrix>, Rc<MatrixScene>, TreeContext)>,
    );
    let (tokens, points, matrix_ctx): RailSceneBuild = match &target {
        RailTarget::Matrix {
            matrix,
            projection,
            context,
        } => {
            let tokens = matrix_tokens(
                &tools,
                matrix,
                projection,
                context,
                &splay_affect(),
                control,
            );
            let cells = projection.cells.iter().filter(|cell| cell.enabled);
            let points: Vec<Vec2> = match context {
                TreeContext::Column { column, .. } => cells
                    .filter(|c| c.column == *column)
                    .map(|c| c.pose.at)
                    .collect(),
                TreeContext::Row { row, .. } => {
                    cells.filter(|c| c.row == *row).map(|c| c.pose.at).collect()
                }
                TreeContext::Key { row, column, .. } => cells
                    .filter(|c| c.row == *row && c.column == *column)
                    .map(|c| c.pose.at)
                    .collect(),
                _ => cells.map(|c| c.pose.at).collect(),
            };
            let fallback = projection
                .cells
                .iter()
                .find(|cell| cell.enabled)
                .or(projection.cells.first())
                .map_or(matrix.origin, |cell| cell.pose.at);
            (
                tokens,
                if points.is_empty() && matches!(context, TreeContext::Matrix { .. }) {
                    vec![fallback]
                } else {
                    points
                },
                Some((matrix.clone(), projection.clone(), context.clone())),
            )
        }
        RailTarget::Part { part_id, .. } => {
            let (tokens, at, courtyard_top) = part_tokens(&tools, part_id, &accepted);
            (
                tokens,
                vec![Vec2 {
                    x: at.x,
                    y: at.y + courtyard_top,
                }],
                None,
            )
        }
    };
    let slots = tools
        .iter()
        .position(|tool| *tool == LayoutTransformTool::Anchor)
        .map_or(tokens.len(), |_| tokens.len() + 1);
    let layout =
        rail_over_points(&points, slots.max(1), px).filter(|_| !tokens.is_empty() || slots > 0);
    let origin_point = matrix_ctx
        .as_ref()
        .filter(|_| {
            tools
                .iter()
                .any(|t| matches!(t, LayoutTransformTool::Splay | LayoutTransformTool::Origin))
        })
        .and_then(|(_, projection, context)| {
            let column = context_column(context)?;
            projection
                .columns
                .iter()
                .find(|basis| basis.column == column)
                .map(|basis| basis.splay_origin)
        });
    let Some(layout) = layout else { return rsx! {} };
    let on_focus_half = match &target {
        RailTarget::Part { on_focus_half, .. } => Some(*on_focus_half),
        _ => None,
    };
    let active_drag = interaction.rail_drag();
    let feedback_state = feedback();
    let preview = try_use_context::<FieldPreview>();
    let preview_source = try_use_context::<Option<FieldPreviewSource>>().flatten();
    let affect_now = splay_affect();
    let show_preview = {
        let (context, matrix_ctx) = (
            matrix_ctx.as_ref().map(|(_, _, c)| c.clone()),
            matrix_ctx.is_some(),
        );
        move |tool: LayoutTransformTool| {
            let (Some(target), Some(source), Some(context), true) =
                (preview, preview_source.clone(), context.clone(), matrix_ctx)
            else {
                return;
            };
            if let Some(field) = tool.preview_field(&context)
                && let Some(facts) = matrix_relations::field_preview(
                    &source.document,
                    &source.scene.matrix_scenes,
                    &context,
                    field,
                    affect_now.clone(),
                )
            {
                target.show(FieldPreviewState {
                    parts: facts.parts,
                    hint: facts.hint,
                });
            }
        }
    };
    let hide_preview = move || {
        if let Some(target) = preview {
            target.clear();
        }
    };

    rsx! {
        if slots > 0 {
            {rail_backdrop(&layout, slots, px)}
        }
        if let Some(point) = guide() {
            g { class: "pointer-events-none fill-none stroke-primary stroke-[0.45] [&_circle]:fill-card", "aria-hidden": "true",
                circle { cx: "{point.x}", cy: "{point.y}", r: "{width / 170.0}" }
            }
        }
        for (index, (token, maker)) in tokens.into_iter().enumerate() {
            {
                let (tx, ty) = layout.token_centers.get(index).copied().unwrap_or((0.0, 0.0));
                let dragging = active_drag.as_ref().is_some_and(|active| active.tool_label == token.tool.label());
                let lit = hovered() == Some(index);
                let (fill, stroke) = token_paint(dragging, lit);
                let quick = control && matches!(token.tool, LayoutTransformTool::Stagger | LayoutTransformTool::Offset);
                let face = token_face(fill, glyph(token.tool, quick, token.glyph_degrees, stroke), token.title.as_deref());
                let description = format!("{}{}", token.value, if token.tool == LayoutTransformTool::Stagger { ". Hold Ctrl while dragging to offset the column." } else { "" });
                let opacity = if active_drag.as_ref().is_some_and(|active| active.tool_label != token.tool.label()) { 0.35 } else { 1.0 };
                let (rail_view, maker, token_for_key) = (rail_view.clone(), maker.unwrap_or_else(|| Rc::new(|_, _, _, _, _| None)), token.clone());
                let (show, hide) = (show_preview.clone(), hide_preview);
                let (matrix_for_key, projection_for_key) = (matrix_ctx.clone(), matrix_ctx.clone());
                let (mut hovered, name, value) = (hovered, token.tool.label().to_owned(), token.value.clone());
                let (name_focus, value_focus) = (name.clone(), value.clone());
                rsx! {
                    if token.guide.is_some() && (lit || dragging) {
                        {axis_guide((tx, ty), token.guide.unwrap_or((0.0, 1.0)), px, if dragging { "0.8" } else { "0.35" })}
                    }
                    g {
                        "data-transform-handle": "true", class: "outline-none focus-visible:stroke-foreground",
                        transform: "translate({tx} {ty}) scale({px} {-px})", role: "button", tabindex: "0", "aria-label": "{token.label}",
                        "aria-description": "{description}", "aria-disabled": "{token.disabled}",
                        "data-quick-offset": "{quick}",
                        style: "cursor:grab;touch-action:none;opacity:{opacity}",
                        onpointerdown: { let (rail_view, maker, on_focus_half) = (rail_view.clone(), maker.clone(), on_focus_half); move |event: PointerEvent| {
                            if let Some(on_focus_half) = on_focus_half { on_focus_half.call(()); }
                            if token_for_key.disabled { return; }
                            if let Some(pointer) = event.data().try_as_web_event() { start_rail_drag(&rail_view, &pointer, &maker); }
                        } },
                        onkeydown: {
                            let rail_view = rail_view.clone();
                            move |event: KeyboardEvent| {
                                let Some(key) = event.data().try_as_web_event() else { return };
                                let arrow = match key.key().as_str() {
                                    "ArrowLeft" => ArrowKey::Left,
                                    "ArrowRight" => ArrowKey::Right,
                                    "ArrowUp" => ArrowKey::Up,
                                    "ArrowDown" => ArrowKey::Down,
                                    _ => return,
                                };
                                let (Some((matrix, projection, _)), Some(nudge)) = (matrix_for_key.as_ref().or(projection_for_key.as_ref()), token_for_key.nudge.as_ref()) else { return };
                                if rail_view.interaction.current().is_some() || !boardstudio_web_layout_core::rail_drag::nudge_session_is_ready(&rail_view.runtime.model()) { return; }
                                // Preflight decides whether the key belongs to this token; the resolver below recomputes from the current accepted snapshot.
                                let preflight_nudge = match nudge {
                                    RailNudge::MatrixMove { .. } => {
                                        let Some(accepted) = rail_view.runtime.model().accepted else { return };
                                        match matrix_drag_anchor(&accepted, matrix) {
                                            Ok(anchor) => RailNudge::MatrixMove { anchor },
                                            Err(()) => {
                                                rail_view.runtime.report("The matrix is anchored to a key that is not laid out.");
                                                return;
                                            }
                                        }
                                    }
                                    other => other.clone(),
                                };
                                let big = key.shift_key();
                                if preflight_nudge.operation(matrix, projection, arrow, big).is_none() {
                                    return;
                                }
                                key.prevent_default();
                                key.stop_propagation();
                                let matrix_id = matrix.id.clone();
                                let nudge = nudge.clone();
                                let unlaid = Rc::new(std::cell::Cell::new(false));
                                let unlaid_for_resolver = unlaid.clone();
                                let result = rail_view.runtime.commit(
                                    "layout-transform-drag",
                                    EditResolver::new("layout-transform-drag", move |accepted: &AcceptedSnapshot| {
                                        let Some(matrix) = accepted.document.matrices.iter().find(|m| m.id == matrix_id) else {
                                            return Resolution::Retire("The matrix is no longer present.".into());
                                        };
                                        let Some(scene) = accepted.scene.matrix_scenes.iter().find(|s| s.matrix_id == matrix_id) else {
                                            return Resolution::Retire("The matrix scene is not available.".into());
                                        };
                                        // An anchored matrix steps through its anchor frame; one whose anchor key is not laid out refuses.
                                        let nudge = match &nudge {
                                            RailNudge::MatrixMove { .. } => match matrix_drag_anchor(accepted, matrix) {
                                                Ok(anchor) => RailNudge::MatrixMove { anchor },
                                                Err(()) => {
                                                    unlaid_for_resolver.set(true);
                                                    return Resolution::Retire(
                                                        "The matrix is anchored to a key that is not laid out.".into(),
                                                    );
                                                }
                                            },
                                            other => other.clone(),
                                        };
                                        let Some(operation) = nudge.operation(matrix, scene, arrow, big) else {
                                            return Resolution::Retire("The nudge no longer applies.".into());
                                        };
                                        Resolution::submit_with_transaction_id(
                                            vec![matrix_id.clone()],
                                            String::new(),
                                            operation,
                                        )
                                    }),
                                );
                                report_rail_commit(&rail_view.runtime, result, unlaid.get());
                            }
                        },
                        onpointerenter: { let (mut feedback, mut popup_at, name, value, show) = (rail_view.feedback, rail_view.popup_at, name.clone(), value.clone(), show.clone()); move |_| { hovered.set(Some(index)); show(token.tool); popup_at.set(None); feedback_event(feedback, FeedbackEvent::Choose { name: name.clone(), value: value.clone() }); let _ = &mut feedback; } },
                        onpointerleave: { let feedback = rail_view.feedback; move |_| { hovered.set(None); hide(); feedback_event(feedback, FeedbackEvent::Dismiss); } },
                        onfocus: { let (feedback, mut popup_at) = (rail_view.feedback, rail_view.popup_at); move |_| { show(token.tool); popup_at.set(None); feedback_event(feedback, FeedbackEvent::Choose { name: name_focus.clone(), value: value_focus.clone() }); } },
                        onblur: { let feedback = rail_view.feedback; move |_| { hide(); feedback_event(feedback, FeedbackEvent::Dismiss); } },
                        {face}
                    }
                }
            }
        }
        if let (Some(origin), Some((matrix, projection, context))) = (origin_point, matrix_ctx.clone()) {
            {origin_handle(&rail_view, &matrix, &projection, &context, origin, px)}
        }
        {super::layout_rail_chrome::feedback_chrome(&feedback_state, true, popup_at(), Some(&layout), px, (view_x, view_y, width))}
    }
}

/// The dashed guide through a lit one-dimensional token along its drag axis.
fn axis_guide((x, y): (f64, f64), (axis_x, axis_y): (f64, f64), px: f64, opacity: &str) -> Element {
    let length = axis_x.hypot(axis_y).max(1e-9);
    let (dx, dy) = (axis_x / length * 17.0 * px, axis_y / length * 17.0 * px);
    rsx! {
        line { x1: "{x - dx}", y1: "{y - dy}", x2: "{x + dx}", y2: "{y + dy}", class: "pointer-events-none",
            style: "fill:none;stroke:var(--primary);stroke-width:1px;stroke-dasharray:3 3;opacity:{opacity};pointer-events:none;vector-effect:non-scaling-stroke" }
    }
}

/// The splay-origin point drawn on the canvas for the Splay and Origin tools.
fn origin_handle(
    rail_view: &RailView,
    matrix: &Rc<Matrix>,
    projection: &Rc<MatrixScene>,
    context: &TreeContext,
    origin: Vec2,
    px: f64,
) -> Element {
    let column = context_column(context).unwrap_or(0);
    let basis = projection
        .columns
        .iter()
        .find(|basis| basis.column == column)
        .cloned();
    let maker: SeedMaker = {
        let matrix = matrix.clone();
        Rc::new(move |accepted, _, ctrl, tx, board_id| {
            let basis = basis.clone()?;
            Some(rail_seed(
                LayoutTransformTool::Origin,
                RailDragKind::Gesture(RailGesture::Origin {
                    matrix: (*matrix).clone(),
                    column,
                    swing: ctrl,
                    column_base: basis.base,
                    start_origin: basis.splay_origin,
                }),
                true,
                accepted,
                tx,
                board_id,
            ))
        })
    };
    let rail_view = rail_view.clone();
    rsx! {
        g { "data-transform-handle": "true", class: "outline-none focus-visible:stroke-foreground",
            transform: "translate({origin.x} {origin.y}) scale({px} {-px})", role: "button", tabindex: "0", "aria-label": "Move splay origin",
            "aria-description": "Hold Ctrl while dragging to swing the origin without moving the keys.",
            style: "cursor:grab;touch-action:none",
            onpointerdown: move |event: PointerEvent| {
                if let Some(pointer) = event.data().try_as_web_event() {
                    start_rail_drag(&rail_view, &pointer, &maker);
                }
            },
            rect { x: "-12", y: "-12", width: "24", height: "24", style: "fill:transparent;stroke:none;pointer-events:all" }
            circle { r: "3", style: "fill:var(--card);stroke:var(--primary);stroke-width:1.5px;vector-effect:non-scaling-stroke" }
            path { d: "M-6 0h12M0 -6v12", style: "fill:none;stroke:var(--primary);stroke-width:1px;vector-effect:non-scaling-stroke" }
            title { "Move splay origin" }
        }
    }
}
