//! The Layout canvas rail: labelled handle tokens over the selected matrix, row, column, key or
//! independent part. One overlay renders the rail and routes pointer drags through
//! [`LayoutInteraction`] into `CanvasInteraction`; pure gesture math lives in
//! `layout_core::rail_gesture`.
use super::TreeContext;
pub use super::layout_rail_tool::{LayoutTransformTool, ToolActivation, activate_transform_tool};
use super::layout_toolbar::{GestureSnapInputs, LayoutSnapSettings};
use crate::canvas_interaction::{
    ActiveRailDrag, CanvasEvent, CanvasMode, GestureSeed, InteractionEffect, LayoutInteraction,
    RailBodyDrag, RailDragKind, RailDragSeed, begin_body_drag, client_point, release_capture,
    submit_gesture_step,
};
use crate::matrix_relations;
use crate::matrix_transform_operation::{
    ArrowKey, MatrixDragAnchor, PlacementParameter as PP, SceneBasis, column_angle, handle_rail,
    matrix_drag_anchor_pose, matrix_drag_transport,
};
use crate::outline_snapping::Origin;
use crate::runtime::Runtime;
use boardstudio_application::{AcceptedSnapshot, EditResolver, Event, Resolution};
use boardstudio_core_kernel::matrix::layout::evaluated_part_pose;
use boardstudio_model::model::{
    EditOperation, Matrix, MatrixScene, MatrixSplayAffect, Position, ProjectDoc, Vec2,
};
use boardstudio_web_layout_core::rail_drag::{drag_session_is_ready, nudge_session_is_ready};
use boardstudio_web_layout_core::rail_gesture::{
    PartAxis, RailGesture, RailInput, RailNudge, anchor_key_base_rotation, format_placement_value,
    part_edit_guard, placement_current,
};
use boardstudio_web_ui_model::state::{FieldPreview, FieldPreviewSource, FieldPreviewState};
use boardstudio_web_ui_model::transform_feedback::{
    FeedbackDelta, FeedbackEvent, FeedbackState, advance_feedback,
};
use dioxus::prelude::*;
use dioxus_web::WebEventExt;
use std::rc::Rc;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::spawn_local;
use web_sys::SvgElement;

use LayoutTransformTool as T;
pub(super) type SurfaceRef = Rc<std::cell::RefCell<Option<SvgElement>>>;
const ANCHOR_ERR: &str = "The matrix is anchored to a key that is not laid out.";
const GLYPH_ARROWS: &str = "M-5 0h10M-3 -2l-2 2 2 2M3 -2l2 2-2 2";
const GLYPH_FOUR: &str = "M0 -3.2v6.4M-3.2 0h6.4M-1.4 -1.8 0 -3.2l1.4 1.4M-1.4 1.8 0 3.2l1.4 -1.4M-1.8 -1.4-3.2 0l1.4 1.4M1.8 -1.4 3.2 0l-1.4 1.4";
const GLYPH_SPLAY: &str = "M-5 4A9 9 0 0 1 4 -5M1.5 -6.5 4 -5l-1.3 2.4M-5 4h2";
const GLYPH_BEND: &str = "M-4 5C-4 0-1-4 4-4M1.5-6 4-4 1.5-2";
const GLYPH_ROTATE: &str = "M4.5 -2A5 5 0 1 1 -2 -4.6M4.6 -5v3h-3";
pub(super) const GLYPH_ANCHOR: &str =
    "M0-2.5V5M-3-.5h6M-4.5 2.2a4.5 4.5 0 0 0 9 0M0-5.6a1.5 1.5 0 1 0 .01 0";

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

#[derive(Clone)]
struct Token(T, &'static str, String, f64, Option<(f64, f64)>, bool);

#[derive(Clone, Default)]
struct RailChrome {
    popup_at: Option<(f64, f64)>,
    guide: Option<Vec2>,
    hovered: Option<usize>,
}

fn glyph(tool: T, quick: bool, deg: f64, stroke: &str) -> Element {
    let s = format!(
        "fill:none;stroke:{stroke};stroke-width:1.5px;stroke-linecap:round;stroke-linejoin:round;vector-effect:non-scaling-stroke"
    );
    match (tool, quick) {
        (T::Stagger | T::Offset, false) | (T::Spread, _) => {
            rsx! { g { transform: "rotate({deg})", path { d: "{GLYPH_ARROWS}", style: "{s}" } } }
        }
        (T::Stagger | T::Offset, true) | (T::Shift | T::Move, _) => {
            rsx! { g { transform: "scale(1.5)", path { d: "{GLYPH_FOUR}", style: "{s}" } } }
        }
        (T::Bend, _) => rsx! { path { d: "{GLYPH_BEND}", style: "{s}" } },
        (T::Splay, _) => rsx! { path { d: "{GLYPH_SPLAY}", style: "{s}" } },
        (T::Rotate, _) => rsx! { path { d: "{GLYPH_ROTATE}", style: "{s}" } },
        (T::Anchor, _) => rsx! { path { d: "{GLYPH_ANCHOR}", style: "{s}" } },
        (T::Origin, _) => rsx! {},
    }
}

pub(super) fn token_paint(active: bool, lit: bool) -> (&'static str, &'static str) {
    match (active, lit) {
        (true, _) => ("var(--primary)", "var(--primary-foreground)"),
        (false, true) => ("var(--selected)", "var(--selected-foreground)"),
        _ => ("transparent", "var(--muted-foreground)"),
    }
}

pub(super) fn token_face(fill: &str, glyph: Element, title: Option<&str>) -> Element {
    rsx! {
        rect { x: "-9", y: "-9", width: "18", height: "18", rx: "4", style: "fill:{fill};stroke:none" }
        rect { x: "-10", y: "-12", width: "20", height: "24", style: "fill:transparent;stroke:none;pointer-events:all" }
        {glyph}
        if let Some(title) = title { title { "{title}" } }
    }
}

pub(super) fn world_per_screen(surface: &SurfaceRef, width: f64, height: f64) -> f64 {
    surface.borrow().as_ref().map_or(1.0, |svg| {
        let r = svg.get_bounding_client_rect();
        if r.width() > 0.0 {
            (width / r.width()).max(height / r.height().max(1.0))
        } else {
            1.0
        }
    })
}

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

pub(super) fn rail_backdrop(
    layout: &handle_rail::WorldRailLayout,
    count: usize,
    px: f64,
) -> Element {
    let (w, cx, cy) = (
        handle_rail::rail_width(count),
        layout.rail.x + layout.rail.w / 2.0,
        layout.rail.y + layout.rail.h / 2.0,
    );
    let st = "fill:var(--card);stroke:var(--border-strong);stroke-width:1px;vector-effect:non-scaling-stroke;filter:drop-shadow(0 2px 4px rgb(24 35 49 / 12%))";
    rsx! { g { "aria-hidden": "true", transform: "translate({cx} {cy}) scale({px} {-px})", style: "pointer-events:none", rect { x: "{-w / 2.0}", y: "-11", width: "{w}", height: "22", rx: "6", style: "{st}" } } }
}

#[derive(Clone)]
struct RailView {
    runtime: Rc<Runtime>,
    surface: SurfaceRef,
    interaction: LayoutInteraction,
    target: RailTarget,
    feedback: Signal<FeedbackState>,
    chrome: Signal<RailChrome>,
    toast_gen: Signal<u64>,
    view: (f64, f64, f64, f64),
    px: f64,
    snap: GestureSnapInputs,
    origins: Rc<Vec<Origin>>,
    preview: Option<FieldPreview>,
    preview_source: Option<FieldPreviewSource>,
    affect: MatrixSplayAffect,
}

impl RailView {
    fn world_pt(&self, ptr: &web_sys::PointerEvent) -> Option<(Vec2, f64)> {
        let (x, y, w, h) = self.view;
        crate::presentation::coordinates_with_scale(&self.surface, ptr, x, y, w, h)
    }
    fn set_popup(&self, pt: Vec2) {
        let mut ch = self.chrome;
        if let Ok(mut c) = ch.try_write() {
            c.popup_at = Some((pt.x + 18.0 * self.px, pt.y - 18.0 * self.px));
        }
    }
    fn end_drag(&self, id: i32) {
        // Drag leaves CanvasInteraction state before release_capture dispatches lostpointercapture.
        release_capture(&self.surface, id);
        let mut ch = self.chrome;
        if let Ok(mut c) = ch.try_write() {
            c.popup_at = None;
            c.guide = None;
        }
    }
    fn rail_input<'a>(
        &'a self,
        st: (f64, f64),
        ptr: &web_sys::PointerEvent,
        point: Vec2,
    ) -> Option<RailInput<'a>> {
        let r = self.surface.borrow().as_ref()?.get_bounding_client_rect();
        if r.width() <= 0.0 || r.height() <= 0.0 {
            return None;
        }
        let (_, _, w, h) = self.view;
        let dx = (f64::from(ptr.client_x()) - st.0) * w / r.width();
        let dy = -(f64::from(ptr.client_y()) - st.1) * h / r.height();
        let (world_delta, alt, tolerance) =
            (Vec2 { x: dx, y: dy }, ptr.alt_key(), w / r.width() * 8.0);
        let (pitch, snap_fraction, geometry_snap) = (
            self.snap.pitch,
            self.snap.snap_fraction,
            self.snap.geometry_snap,
        );
        Some(RailInput {
            point,
            world_delta,
            alt,
            pitch,
            snap_fraction,
            geometry_snap,
            origins: &self.origins,
            tolerance,
        })
    }
    fn preview_edit(&self, tx: &str, target: &str, operation: EditOperation) {
        let (operation_id, transaction_id, target_ids) = (
            self.runtime.operation(),
            tx.to_owned(),
            vec![target.to_owned()],
        );
        self.runtime.submit(Event::PreviewEdit {
            operation_id,
            transaction_id,
            target_ids,
            operation,
        });
    }
    fn choose_token(&self, hovered: Option<usize>, tool: T, name: &str, val: &str) {
        let mut ch = self.chrome;
        {
            let mut c = ch.write();
            if hovered.is_some() {
                c.hovered = hovered;
            }
            c.popup_at = None;
        }
        if let (Some(t), Some(s), RailTarget::Matrix { context, .. }) =
            (self.preview, self.preview_source.as_ref(), &self.target)
            && let Some(field) = tool.preview_field(context)
            && let Some(f) = matrix_relations::field_preview(
                &s.document,
                &s.scene.matrix_scenes,
                context,
                field,
                self.affect.clone(),
            )
        {
            t.show(FieldPreviewState {
                parts: f.parts,
                hint: f.hint,
            });
        }
        feedback_event(
            self.feedback,
            FeedbackEvent::Choose {
                name: name.to_owned(),
                value: val.to_owned(),
            },
        );
    }
    fn blur_token(&self, unhover: bool) {
        let mut ch = self.chrome;
        if unhover {
            ch.write().hovered = None;
        }
        if let Some(t) = self.preview {
            t.clear();
        }
        feedback_event(self.feedback, FeedbackEvent::Dismiss);
    }
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
    let mut fb = feedback;
    spawn_local(async move {
        gloo_timers::future::TimeoutFuture::new(1800).await;
        if generation.try_peek().is_ok_and(|c| *c == seen)
            && let Ok(mut st) = fb.try_write()
        {
            *st = advance_feedback(&st, FeedbackEvent::Expire);
        }
    });
}

pub(super) fn feedback_event(mut feedback: Signal<FeedbackState>, event: FeedbackEvent) {
    let next = advance_feedback(&feedback(), event);
    feedback.set(next);
}

fn drag_feedback(v: &RailView, lbl: &str, value: String, delta: FeedbackDelta, finish: bool) {
    if finish {
        raise_toast(v.feedback, v.toast_gen, format!("{lbl} {value}"), delta);
        return;
    }
    feedback_event(
        v.feedback,
        FeedbackEvent::Drag {
            name: lbl.to_owned(),
            value,
            delta,
        },
    );
}

fn clear_rail_preview(rt: &Rc<Runtime>, a: &ActiveRailDrag) {
    let (operation_id, token, revision) = (rt.operation(), a.token, a.revision);
    let (board_id, transaction_id) = (a.board_id.clone(), a.transaction_id.clone());
    rt.submit(Event::ClearPreview {
        operation_id,
        token,
        revision,
        board_id,
        transaction_id,
    });
}

fn cancel_active_rail_drag(view: &RailView, active: &ActiveRailDrag) {
    match &active.kind {
        RailDragKind::Body(b) if b.last_sample.is_some() => {
            view.runtime.submit(Event::GestureCancel {
                pointer_id: i64::from(active.pointer_id),
            });
        }
        _ if active.last_preview.is_some() => clear_rail_preview(&view.runtime, active),
        _ => {}
    }
    // The drag must leave CanvasInteraction state before release_capture runs because
    // release_pointer_capture dispatches lostpointercapture (CanvasInteraction transitions
    // out of Dragging*Rail before on_cancel invokes this callback).
    view.end_drag(active.pointer_id);
    let mut fb = view.feedback;
    if let Ok(mut st) = fb.try_write() {
        *st = advance_feedback(&st, FeedbackEvent::Dismiss);
    }
}

fn abandon_rail_drag(interaction: LayoutInteraction, expected: CanvasMode) {
    if interaction.current() == Some(expected)
        && let Some(active) = interaction.rail_drag()
    {
        interaction.cancel(active.pointer_id);
    }
}

fn start_rail_drag(view: &RailView, ptr: &web_sys::PointerEvent, tool: T) {
    if ptr.button() != 0 || view.interaction.current().is_some() {
        return;
    }
    let model = view.runtime.model();
    // Sync Ctrl before reading quick-offset: handles stop propagation, so the latched state may be stale.
    view.interaction.sync_control(ptr.ctrl_key());
    let quick = ptr.ctrl_key() || view.interaction.peek_control_down();
    if let (Some(snap), Some(scope)) = (model.accepted.as_ref(), view.runtime.scope())
        && drag_session_is_ready(&model, Some(snap.document.revision))
        && let (Some((pt, _)), Some(cap)) = (view.world_pt(ptr), view.surface.borrow().clone())
        && let Some((kind, is_mtx)) = tool_seed(&view.target, tool, &view.affect, snap, pt, quick)
    {
        let transaction_id = format!("layout-transform-{}", view.runtime.operation().0);
        let (tool_label, token, revision, board_id) = (
            tool.label(),
            snap.token,
            snap.document.revision,
            scope.board_id,
        );
        let raw = RailDragSeed {
            transaction_id,
            tool_label,
            kind,
            token,
            revision,
            board_id,
        };
        let seed = if is_mtx {
            GestureSeed::MatrixRail(raw)
        } else {
            GestureSeed::PartRail(raw)
        };
        let h = ptr
            .target()
            .and_then(|t| t.dyn_into::<web_sys::Element>().ok())
            .and_then(|e| e.closest("[data-transform-handle]").ok().flatten())
            .and_then(|e| e.dyn_into::<SvgElement>().ok());
        if view.interaction.capture_drag(
            ptr.pointer_id(),
            client_point(ptr),
            ptr.ctrl_key(),
            seed,
            &cap,
            h.as_ref(),
        ) {
            ptr.prevent_default();
            ptr.stop_propagation();
            let (cv, pv) = (view.clone(), view.clone());
            view.interaction.on_cancel(move || {
                if let Some(c) = cv.interaction.take_canceled_rail_drag() {
                    cancel_active_rail_drag(&cv, &c);
                }
            });
            view.interaction
                .on_pointer(move |p, rel| handle_rail_pointer(&pv, p, rel));
            view.set_popup(pt);
        }
    }
}

fn handle_rail_pointer(view: &RailView, ptr: &web_sys::PointerEvent, released: bool) {
    let id = ptr.pointer_id();
    let Some(active) = view.interaction.rail_drag().filter(|a| a.pointer_id == id) else {
        return;
    };
    ptr.prevent_default();
    ptr.stop_propagation();
    if let RailDragKind::Body(body) = &active.kind {
        body_rail_moved(view, ptr, &active, body, released);
        if released {
            view.interaction.pointer_up(id);
            view.end_drag(id);
        }
        return;
    }
    let RailDragKind::Gesture(gesture) = &active.kind else {
        return;
    };
    let smp = view.world_pt(ptr).and_then(|(pt, _)| {
        Some((
            pt,
            gesture.sample(&view.rail_input(active.start_client, ptr, pt)?)?,
        ))
    });
    let (target, tx, lbl) = (
        gesture.target_id(),
        &active.transaction_id,
        active.tool_label,
    );
    if !released {
        if let Some((pt, p)) = smp {
            view.interaction
                .note_rail_preview(Some((p.value.clone(), p.delta.clone())));
            let mut ch = view.chrome;
            ch.write().guide = p.guide;
            view.set_popup(pt);
            for eff in view.interaction.handle(CanvasEvent::PointerMove {
                id,
                client: client_point(ptr),
                operation: Some(p.operation),
            }) {
                if let InteractionEffect::Preview(op) = eff {
                    view.preview_edit(tx, target, op);
                }
            }
            drag_feedback(view, lbl, p.value, p.delta, false);
        }
        return;
    }
    let eff = smp
        .as_ref()
        .map(|(_, p)| (p.value.clone(), p.delta.clone()))
        .or_else(|| active.last_preview.clone());
    let commit = view
        .interaction
        .handle(CanvasEvent::PointerUp {
            id,
            operation: smp.map(|(_, p)| p.operation),
        })
        .into_iter()
        .find(|e| matches!(e, InteractionEffect::Commit { .. }));
    if let (
        Some(InteractionEffect::Commit {
            operation: op,
            needs_preview,
        }),
        Some((v, dl)),
    ) = (commit, &eff)
        && !dl.is_zero()
    {
        if needs_preview {
            view.preview_edit(tx, target, op.clone());
        }
        commit_rail_edit(&view.runtime, target, tx, Some(active.revision), op);
        drag_feedback(view, lbl, v.clone(), dl.clone(), true);
    } else {
        clear_rail_preview(&view.runtime, &active);
        if let Some((v, _)) = eff {
            feedback_event(
                view.feedback,
                FeedbackEvent::Choose {
                    name: lbl.to_owned(),
                    value: v,
                },
            );
        }
    }
    view.end_drag(id);
}

/// A detached part's Move drives the session body-drag gesture past the shipped threshold.
fn body_rail_moved(
    view: &RailView,
    ptr: &web_sys::PointerEvent,
    active: &ActiveRailDrag,
    body: &RailBodyDrag,
    finishing: bool,
) {
    let client = client_point(ptr);
    if client == active.start_client && body.last_sample.is_none() {
        return;
    }
    let world = view.world_pt(ptr);
    let sample = world.map(|(_, sc)| {
        let (dx, dy) = (
            (client.0 - active.start_client.0) / sc,
            (client.1 - active.start_client.1) / sc,
        );
        (
            Vec2 {
                x: body.start_at.x + dx,
                y: body.start_at.y - dy,
            },
            sc,
            ptr.alt_key(),
        )
    });
    if let Some((at, sc, alt)) = sample.or(body.last_sample.filter(|_| finishing)) {
        let (rt, inter, id) = (&view.runtime, view.interaction, active.pointer_id);
        let mut pos = [Position {
            id: body.part_id.clone(),
            at: body.start_at,
        }];
        let tx = Some(active.transaction_id.as_str());
        if body.last_sample.is_none()
            && begin_body_drag(rt, inter, id, tx, &pos, view.snap, (sc, alt)).is_none()
        {
            return;
        }
        inter.note_rail_body_sample(Some((at, sc, alt)));
        pos[0].at = at;
        submit_gesture_step(rt, id, pos.to_vec(), sc, alt, finishing);
        let (delta, val) = (
            FeedbackDelta::Mm2(at.x - body.start_at.x, at.y - body.start_at.y),
            format!("X {:.2}, Y {:.2} mm", at.x, at.y),
        );
        if finishing {
            drag_feedback(view, active.tool_label, val, delta, true);
        } else if let Some((pt, _)) = world {
            drag_feedback(view, active.tool_label, val, delta, false);
            view.set_popup(pt);
        }
    }
}

fn commit_rail_edit(
    rt: &Rc<Runtime>,
    target_id: &str,
    tx: &str,
    expected_rev: Option<u64>,
    op: EditOperation,
) {
    let (id, tx) = (target_id.to_owned(), tx.to_owned());
    let res = EditResolver::new("layout-transform-drag", move |acc| {
        if expected_rev.is_some_and(|r| acc.document.revision != r) {
            return Resolution::Retire("The layout changed before this drag landed.".into());
        }
        Resolution::submit_with_transaction_id(vec![id.clone()], tx.clone(), op.clone())
    });
    if let Err(err) = rt.commit("layout-transform-drag", res) {
        rt.report(err);
    }
}

fn context_column(context: &TreeContext) -> Option<u32> {
    match context {
        TreeContext::Column { column, .. } | TreeContext::Key { column, .. } => Some(*column),
        TreeContext::Component { column, .. } => *column,
        _ => None,
    }
}

fn focus_pt(m: &Matrix, p: &MatrixScene) -> Vec2 {
    p.cells
        .iter()
        .find(|c| c.enabled)
        .or(p.cells.first())
        .map_or(m.origin, |c| c.pose.at)
}

/// The anchor frame a matrix drag inverse-solves through; `Err` when the matrix is anchored to
/// a key that is not laid out, so the drag never solves through a stale frame.
fn matrix_drag_anchor(acc: &AcceptedSnapshot, m: &Matrix) -> Result<Option<MatrixDragAnchor>, ()> {
    let Some(a) = m.anchor.as_ref() else {
        return Ok(None);
    };
    let key_pose = matrix_drag_anchor_pose(&acc.scene.matrix_scenes, a).ok_or(())?;
    let transport = matrix_drag_transport(&acc.document, &m.id);
    Ok(Some(MatrixDragAnchor {
        key_pose,
        follow: a.follow,
        transport,
    }))
}

fn matrix_placement(
    tool: T,
    context: &TreeContext,
    ctrl: bool,
) -> Option<(PP, &'static str, bool, Option<bool>)> {
    let column = context_column(context).unwrap_or(0);
    let (row, key) = match context {
        TreeContext::Row { row, .. } => (Some(*row), None),
        TreeContext::Key { row, column, .. } => (None, Some((*row, *column))),
        _ => (None, None),
    };
    let (p, lbl) = match (tool, row, key) {
        (T::Stagger, ..) if ctrl => (PP::ColumnOffset { column }, "Drag to offset column"),
        (T::Stagger, ..) => (PP::Stagger { column }, "Drag to stagger"),
        // Column 0 follows the pitch, so it has no Spread step to drag.
        (T::Spread, None, _) if column == 0 => return None,
        (T::Spread, ..) => (PP::ColumnSpread { column }, "Drag to spread"),
        (T::Offset, Some(row), _) => (PP::RowOffset { row }, "Drag to offset row"),
        (T::Offset, None, _) => (PP::ColumnOffset { column }, "Drag to offset column"),
        (T::Shift, Some(row), _) => (PP::RowShift { row }, "Drag to shift row"),
        (T::Shift, None, Some((row, column))) => {
            (PP::KeyShift { row, column }, "Drag to shift key")
        }
        (T::Bend, Some(row), _) => (PP::RowBend { row }, "Drag to bend row"),
        (T::Bend, None, Some((row, column))) => (PP::KeyBend { row, column }, "Drag to bend key"),
        _ => return None,
    };
    let nudge = tool != T::Stagger || (!ctrl && row.is_none());
    let guide = match tool {
        T::Stagger | T::Offset => Some(true),
        T::Spread => Some(false),
        _ => None,
    };
    Some((p, lbl, nudge, guide))
}

fn tool_seed(
    target: &RailTarget,
    tool: T,
    aff: &MatrixSplayAffect,
    accepted: &AcceptedSnapshot,
    pt: Vec2,
    ctrl: bool,
) -> Option<(RailDragKind, bool)> {
    if let RailTarget::Part { part_id, .. } = target {
        let doc = &accepted.document;
        let part = doc.parts.iter().find(|p| p.id == *part_id)?;
        if part_edit_guard(doc, part_id, part.anchor.is_some()).is_some() {
            return None;
        }
        let ev = evaluated_part_pose(doc, part);
        let (center, start_angle, start_rotation) =
            (ev.at, (pt.y - ev.at.y).atan2(pt.x - ev.at.x), ev.rotation);
        let axis = match tool {
            T::Move => PartAxis::Offset,
            T::Rotate => PartAxis::Rotation,
            T::Spread => PartAxis::Spread,
            T::Stagger => PartAxis::Stagger,
            _ => return None,
        };
        let part_id = part_id.clone();
        let kind = match (&part.anchor, axis) {
            (None, PartAxis::Offset) => RailDragKind::Body(RailBodyDrag {
                part_id,
                start_at: center,
                last_sample: None,
            }),
            (None, _) => RailDragKind::Gesture(RailGesture::PartRotate {
                part_id,
                center,
                start_angle,
                start_rotation,
            }),
            (Some(anc), axis) => {
                let (anchor, base_rotation) = (
                    anc.clone(),
                    anchor_key_base_rotation(doc, anc).unwrap_or(0.0),
                );
                RailDragKind::Gesture(RailGesture::PartValue {
                    part_id,
                    axis,
                    anchor,
                    base_rotation,
                    center,
                    start_angle,
                })
            }
        };
        return Some((kind, false));
    }
    let RailTarget::Matrix {
        matrix: mat,
        projection: proj,
        context,
    } = target
    else {
        return None;
    };
    let (matrix, column) = ((**mat).clone(), context_column(context).unwrap_or(0));
    let g = if let Some((parameter, ..)) = matrix_placement(tool, context, ctrl) {
        RailGesture::Placement {
            matrix,
            parameter,
            basis: SceneBasis::for_parameter(proj, parameter)?,
        }
    } else if matches!(tool, T::Splay | T::Origin) {
        let b = proj.columns.iter().find(|c| c.column == column)?;
        let (origin, column_base, swing, affect) = (b.splay_origin, b.base, ctrl, aff.clone());
        let start_angle = (pt.y - origin.y).atan2(pt.x - origin.x);
        if tool == T::Origin {
            RailGesture::Origin {
                matrix,
                column,
                swing,
                column_base,
                start_origin: origin,
            }
        } else {
            RailGesture::Splay {
                matrix,
                column,
                origin,
                start_angle,
                swing,
                affect,
                column_base,
            }
        }
    } else if matches!(tool, T::Move | T::Rotate) {
        let (anchor, center) = (
            matrix_drag_anchor(accepted, &matrix).ok()?,
            focus_pt(&matrix, proj),
        );
        let start_angle = (pt.y - center.y).atan2(pt.x - center.x);
        if tool == T::Rotate {
            RailGesture::MatrixRotate {
                matrix,
                anchor,
                center,
                start_angle,
            }
        } else {
            RailGesture::MatrixMove { matrix, anchor }
        }
    } else {
        return None;
    };
    Some((RailDragKind::Gesture(g), true))
}

fn matrix_tokens(
    tools: &[T],
    m: &Matrix,
    proj: &MatrixScene,
    ctx: &TreeContext,
    aff: &MatrixSplayAffect,
    ctrl: bool,
) -> Vec<Token> {
    let basis = proj
        .columns
        .iter()
        .find(|b| b.column == context_column(ctx).unwrap_or(0));
    let axis = basis.map_or(Vec2 { x: 0.0, y: 1.0 }, |b| b.stagger_axis);
    let (ax, deg) = ((axis.x, axis.y), (-axis.y).atan2(axis.x).to_degrees());
    let mut out = Vec::new();
    for &tool in tools {
        // Column 0 follows the pitch, so it has no Spread step; Origin stays on the canvas (`origin_handle`).
        let (mut dis, mut g) = (false, None);
        let (lbl, val) = if let Some((p, l, _, gk)) = matrix_placement(tool, ctx, ctrl) {
            g = gk.map(|a| if a { ax } else { (1.0, 0.0) });
            (l, format_placement_value(placement_current(m, p)))
        } else if let (T::Splay, Some(b)) = (tool, basis) {
            let ang = column_angle(b.splay_angle, b.rotation, aff);
            ("Drag to splay", format!("{ang:.0}°"))
        } else if tool == T::Move {
            (
                "Drag to move matrix",
                format!("X {:.2} mm, Y {:.2} mm", m.origin.x, m.origin.y),
            )
        } else if tool == T::Rotate {
            dis = m.anchor.as_ref().is_some_and(|a| !a.follow.rotation);
            (
                "Drag to rotate matrix",
                format!("{:.2} degrees", m.rotation.unwrap_or(0.0)),
            )
        } else {
            continue;
        };
        out.push(Token(tool, lbl, val, deg, g, dis));
    }
    out
}

fn part_tokens(tools: &[T], part_id: &str, accepted: &AcceptedSnapshot) -> (Vec<Token>, Vec2, f64) {
    let doc = &accepted.document;
    let Some(part) = doc.parts.iter().find(|p| p.id == part_id) else {
        return (Vec::new(), Vec2::default(), 0.0);
    };
    let ev = evaluated_part_pose(doc, part);
    let base = part
        .anchor
        .as_ref()
        .and_then(|a| anchor_key_base_rotation(doc, a))
        .unwrap_or(0.0);
    let (sin, cos) = base.to_radians().sin_cos();
    let (sp, st, off, rot) = part
        .anchor
        .as_ref()
        .map_or((0.0, 0.0, Vec2::default(), ev.rotation), |a| {
            (a.spread, a.stagger, a.offset, a.rotation)
        });
    let pos = if part.anchor.is_some() { off } else { ev.at };
    let (d_sp, d_st) = ((-sin).atan2(cos).to_degrees(), cos.atan2(-sin).to_degrees());
    let mut out = Vec::new();
    for &tool in tools {
        let (l, v, d, g) = match tool {
            T::Move => (
                "Drag to move part",
                format!("X {:.2}, Y {:.2} mm", pos.x, pos.y),
                0.0,
                None,
            ),
            T::Rotate => ("Drag to rotate part", format!("{rot:.1}°"), 0.0, None),
            T::Spread => (
                "Drag to spread",
                format!("{sp:.2} mm"),
                d_sp,
                Some((cos, sin)),
            ),
            T::Stagger => (
                "Drag to stagger",
                format!("{st:.2} mm"),
                d_st,
                Some((-sin, cos)),
            ),
            _ => continue,
        };
        out.push(Token(tool, l, v, d, g, false));
    }
    (out, ev.at, part_courtyard_top(doc, &part.definition_id))
}

pub(super) fn part_courtyard_top(doc: &ProjectDoc, def_id: &str) -> f64 {
    doc.definitions
        .iter()
        .find(|d| d.id == def_id)
        .and_then(|d| d.courtyard.iter().map(|p| p.y).reduce(f64::max))
        .filter(|t| t.is_finite())
        .unwrap_or(0.0)
}

fn matrix_target_points(m: &Matrix, p: &MatrixScene, ctx: &TreeContext) -> Vec<Vec2> {
    let cell_ok = |c: &&boardstudio_model::model::MatrixSceneCell| {
        c.enabled
            && match ctx {
                TreeContext::Column { column, .. } => c.column == *column,
                TreeContext::Row { row, .. } => c.row == *row,
                TreeContext::Key { row, column, .. } => c.row == *row && c.column == *column,
                _ => true,
            }
    };
    let mut pts: Vec<_> = p.cells.iter().filter(cell_ok).map(|c| c.pose.at).collect();
    if pts.is_empty() && matches!(ctx, TreeContext::Matrix { .. }) {
        pts.push(focus_pt(m, p));
    }
    pts
}

fn render_rail_token(
    view: &RailView,
    active: Option<&'static str>,
    hovered: Option<usize>,
    control: bool,
    idx: usize,
    tok: Token,
    (tx, ty): (f64, f64),
) -> Element {
    let Token(tool, lbl, val_f, deg, guide, dis) = tok;
    let (drag, lit) = (active == Some(tool.label()), hovered == Some(idx));
    let (fill, stroke) = token_paint(drag, lit);
    let quick = control && matches!(tool, T::Stagger | T::Offset);
    let titled = match &view.target {
        RailTarget::Matrix { .. } => matches!(tool, T::Spread | T::Splay),
        RailTarget::Part { .. } => tool != T::Stagger,
    };
    let title = titled.then(|| format!("{lbl} · {val_f}"));
    let face = token_face(fill, glyph(tool, quick, deg, stroke), title.as_deref());
    let desc = if tool == T::Stagger {
        format!("{val_f}. Hold Ctrl while dragging to offset the column.")
    } else {
        val_f.clone()
    };
    let op = if active.is_some() && !drag { 0.35 } else { 1.0 };
    let (v1, v2, v3, v4, v5, v6) = (
        view.clone(),
        view.clone(),
        view.clone(),
        view.clone(),
        view.clone(),
        view.clone(),
    );
    let (val, px) = (val_f.clone(), view.px);
    rsx! {
        if let Some(dir) = guide.filter(|_| lit || drag) { {axis_guide((tx, ty), dir, px, if drag { "0.8" } else { "0.35" })} }
        g { "data-transform-handle": "true", class: "outline-none focus-visible:stroke-foreground",
            transform: "translate({tx} {ty}) scale({px} {-px})", role: "button", tabindex: "0",
            "aria-label": "{lbl}", "aria-description": "{desc}", "aria-disabled": "{dis}",
            "data-quick-offset": "{quick}", style: "cursor:grab;touch-action:none;opacity:{op}",
            onpointerdown: move |e: PointerEvent| {
                if let RailTarget::Part { on_focus_half, .. } = &v1.target { on_focus_half.call(()); }
                if !dis && let Some(p) = e.data().try_as_web_event() { start_rail_drag(&v1, &p, tool); }
            },
            onkeydown: move |e: KeyboardEvent| handle_token_keydown(&v2, tool, &e),
            onpointerenter: move |_| v3.choose_token(Some(idx), tool, tool.label(), &val),
            onpointerleave: move |_| v4.blur_token(true),
            onfocus: move |_| v5.choose_token(None, tool, tool.label(), &val_f),
            onblur: move |_| v6.blur_token(false),
            {face}
        }
    }
}

#[component]
pub fn LayoutRailOverlay(
    runtime: LayoutTransformRuntime,
    surface: SurfaceRef,
    target: RailTarget,
    tools: Vec<T>,
    snap_settings: LayoutSnapSettings,
    #[props(into)] snap_origins: Rc<Vec<Origin>>,
    splay_affect: Signal<MatrixSplayAffect>,
    #[props(default)] surface_size: Option<Signal<(f64, f64)>>,
    view_x: f64,
    view_y: f64,
    width: f64,
    height: f64,
) -> Element {
    let _ = surface_size.map(|s| s());
    let runtime = runtime.0;
    let interaction = use_context::<LayoutInteraction>();
    let _ = interaction.version();
    let control = interaction.control_down();
    let (feedback, toast_gen, chrome) = (
        use_signal(FeedbackState::default),
        use_signal(|| 0_u64),
        use_signal(RailChrome::default),
    );
    let (px, accepted) = (
        world_per_screen(&surface, width, height),
        runtime.model().accepted,
    );
    let (snap_fraction, geometry_snap) = (snap_settings.snap_fraction, snap_settings.geometry_snap);
    let (mode, snap) = match &target {
        RailTarget::Matrix { matrix, .. } => (
            CanvasMode::DraggingMatrixRail,
            GestureSnapInputs {
                pitch: matrix.pitch,
                snap_fraction,
                geometry_snap,
                gap: None,
            },
        ),
        RailTarget::Part { snap, .. } => (CanvasMode::DraggingPartRail, *snap),
    };
    let preview_source = try_use_context::<Option<FieldPreviewSource>>().flatten();
    let rv = RailView {
        runtime: runtime.clone(),
        surface,
        interaction,
        target: target.clone(),
        feedback,
        chrome,
        toast_gen,
        view: (view_x, view_y, width, height),
        px,
        snap,
        origins: snap_origins,
        preview: try_use_context::<FieldPreview>(),
        preview_source,
        affect: splay_affect(),
    };
    use_drop(move || abandon_rail_drag(interaction, mode));
    let ctx_key = match &target {
        RailTarget::Matrix { context, .. } => format!("{context:?}"),
        RailTarget::Part { part_id, .. } => format!("part:{part_id}"),
    };
    use_effect(use_reactive((&ctx_key, &tools), move |_| {
        abandon_rail_drag(interaction, mode)
    }));
    let Some(accepted) = accepted else {
        return rsx! {};
    };
    let (tokens, points, origin_pt) = match &target {
        RailTarget::Matrix {
            matrix,
            projection,
            context,
        } => {
            let col = context_column(context);
            let orig = tools
                .iter()
                .any(|t| matches!(t, T::Splay | T::Origin))
                .then(|| {
                    projection
                        .columns
                        .iter()
                        .find(|b| Some(b.column) == col)
                        .map(|b| b.splay_origin)
                })
                .flatten();
            (
                matrix_tokens(&tools, matrix, projection, context, &rv.affect, control),
                matrix_target_points(matrix, projection, context),
                orig,
            )
        }
        RailTarget::Part { part_id, .. } => {
            let (t, at, top) = part_tokens(&tools, part_id, &accepted);
            (
                t,
                vec![Vec2 {
                    x: at.x,
                    y: at.y + top,
                }],
                None,
            )
        }
    };
    let slots = tokens.len() + usize::from(tools.contains(&T::Anchor));
    let Some(layout) =
        rail_over_points(&points, slots.max(1), px).filter(|_| !tokens.is_empty() || slots > 0)
    else {
        return rsx! {};
    };
    let (fb_st, ch_st, active) = (
        feedback(),
        chrome(),
        interaction.rail_drag().map(|a| a.tool_label),
    );
    rsx! {
        if slots > 0 { {rail_backdrop(&layout, slots, px)} }
        if let Some(pt) = ch_st.guide {
            g { class: "pointer-events-none fill-none stroke-primary stroke-[0.45] [&_circle]:fill-card", "aria-hidden": "true", circle { cx: "{pt.x}", cy: "{pt.y}", r: "{width / 170.0}" } }
        }
        for (idx, tok) in tokens.into_iter().enumerate() {
            {render_rail_token(&rv, active, ch_st.hovered, control, idx, tok, layout.token_centers.get(idx).copied().unwrap_or((0.0, 0.0)))}
        }
        if let Some(origin) = origin_pt { {origin_handle(&rv, origin, px)} }
        {super::layout_rail_chrome::feedback_chrome(&fb_st, true, ch_st.popup_at, Some(&layout), px, (view_x, view_y, width))}
    }
}

fn resolve_nudge(acc: &AcceptedSnapshot, m: &Matrix, n: &RailNudge) -> Result<RailNudge, ()> {
    match n {
        RailNudge::MatrixMove { .. } => {
            matrix_drag_anchor(acc, m).map(|anchor| RailNudge::MatrixMove { anchor })
        }
        other => Ok(other.clone()),
    }
}

fn handle_token_keydown(rv: &RailView, tool: T, event: &KeyboardEvent) {
    let Some(key) = event.data().try_as_web_event() else {
        return;
    };
    let arrow = match key.key().as_str() {
        "ArrowLeft" => ArrowKey::Left,
        "ArrowRight" => ArrowKey::Right,
        "ArrowUp" => ArrowKey::Up,
        "ArrowDown" => ArrowKey::Down,
        _ => return,
    };
    let RailTarget::Matrix {
        matrix,
        projection,
        context,
    } = &rv.target
    else {
        return;
    };
    if rv.interaction.current().is_some() || !nudge_session_is_ready(&rv.runtime.model()) {
        return;
    }
    let nudge = if let Some((p, _, true, _)) = matrix_placement(tool, context, false) {
        if tool == T::Stagger && rv.interaction.control_down() {
            return;
        }
        RailNudge::Placement(p)
    } else {
        let col = context_column(context).unwrap_or(0);
        match tool {
            T::Splay => RailNudge::SplayAngle {
                column: col,
                affect: rv.affect.clone(),
            },
            T::Move => RailNudge::MatrixMove { anchor: None },
            T::Rotate => RailNudge::MatrixRotate,
            _ => return,
        }
    };
    let Some(accepted) = rv.runtime.model().accepted else {
        return;
    };
    // Preflight decides whether the key belongs to this token; the resolver below recomputes from
    // the current accepted snapshot, refusing anchored matrices whose anchor key is not laid out.
    let Ok(pre) = resolve_nudge(&accepted, matrix, &nudge) else {
        rv.runtime.report(ANCHOR_ERR);
        return;
    };
    let big = key.shift_key();
    if pre.operation(matrix, projection, arrow, big).is_none() {
        return;
    }
    key.prevent_default();
    key.stop_propagation();
    let matrix_id = matrix.id.clone();
    let unlaid = Rc::new(std::cell::Cell::new(false));
    let flag = unlaid.clone();
    let res = EditResolver::new("layout-transform-drag", move |acc| {
        let Some(m) = acc.document.matrices.iter().find(|m| m.id == matrix_id) else {
            return Resolution::Retire("The matrix is no longer present.".into());
        };
        let Some(sc) = acc
            .scene
            .matrix_scenes
            .iter()
            .find(|s| s.matrix_id == matrix_id)
        else {
            return Resolution::Retire("The matrix scene is not available.".into());
        };
        let Ok(n) = resolve_nudge(acc, m, &nudge) else {
            flag.set(true);
            return Resolution::Retire(ANCHOR_ERR.into());
        };
        let Some(op) = n.operation(m, sc, arrow, big) else {
            return Resolution::Retire("The nudge no longer applies.".into());
        };
        Resolution::submit_with_transaction_id(vec![matrix_id.clone()], String::new(), op)
    });
    if let Err(err) = rv.runtime.commit("layout-transform-drag", res) {
        rv.runtime.report(if unlaid.get() {
            ANCHOR_ERR.to_owned()
        } else {
            err
        });
    }
}

fn axis_guide((x, y): (f64, f64), (ax, ay): (f64, f64), px: f64, opacity: &str) -> Element {
    let len = ax.hypot(ay).max(1e-9);
    let (dx, dy) = (ax / len * 17.0 * px, ay / len * 17.0 * px);
    let st = format!(
        "fill:none;stroke:var(--primary);stroke-width:1px;stroke-dasharray:3 3;opacity:{opacity};pointer-events:none;vector-effect:non-scaling-stroke"
    );
    rsx! { line { x1: "{x - dx}", y1: "{y - dy}", x2: "{x + dx}", y2: "{y + dy}", class: "pointer-events-none", style: "{st}" } }
}

fn origin_handle(rail_view: &RailView, origin: Vec2, px: f64) -> Element {
    let rv = rail_view.clone();
    let desc = "Hold Ctrl while dragging to swing the origin without moving the keys.";
    let stroke =
        "fill:none;stroke:var(--primary);stroke-width:1px;vector-effect:non-scaling-stroke";
    let circ = "fill:var(--card);stroke:var(--primary);stroke-width:1.5px;vector-effect:non-scaling-stroke";
    rsx! {
        g { "data-transform-handle": "true", class: "outline-none focus-visible:stroke-foreground",
            transform: "translate({origin.x} {origin.y}) scale({px} {-px})", role: "button", tabindex: "0",
            "aria-label": "Move splay origin", "aria-description": "{desc}", style: "cursor:grab;touch-action:none",
            onpointerdown: move |e: PointerEvent| if let Some(p) = e.data().try_as_web_event() { start_rail_drag(&rv, &p, T::Origin); },
            rect { x: "-12", y: "-12", width: "24", height: "24", style: "fill:transparent;stroke:none;pointer-events:all" }
            circle { r: "3", style: "{circ}" }
            path { d: "M-6 0h12M0 -6v12", style: "{stroke}" }
            title { "Move splay origin" }
        }
    }
}
