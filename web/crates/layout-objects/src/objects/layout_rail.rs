//! The Layout canvas rail: labelled handle tokens over the selected matrix, row, column, key or
//! independent part. One overlay owns every drag; the pure gesture math lives in
//! `layout_core::rail_gesture` and the pointer protocol in [`Ctl`].
use super::layout_selection_kind::LayoutSelectionKind;
use super::layout_toolbar::{GestureSnapInputs, LayoutSnapSettings};
use super::TreeContext;
use crate::matrix_relations;
use crate::matrix_transform_operation::{
    ArrowKey, MatrixTransformField, PlacementParameter, SceneBasis, column_angle, handle_rail, matrix_drag_anchor_pose,
    matrix_drag_transport, MatrixDragAnchor,
};
use crate::outline_snapping::Origin;
use crate::presentation::canvas_interaction::{CanvasInteractionArbiter, CanvasInteractionOwner};
use crate::runtime::Runtime;
use boardstudio_application::{AcceptedSnapshot, EditResolver, Event, Resolution, SnapshotToken};
use boardstudio_core_kernel::matrix::layout::evaluated_part_pose;
use boardstudio_model::model::{EditOperation, Matrix, MatrixScene, MatrixSplayAffect, Position, Vec2};
use boardstudio_web_layout_core::{
    interaction::CanvasInteraction,
    rail_drag::drag_session_is_ready,
    rail_gesture::{
        PartAxis, RailGesture, RailInput, RailNudge, RailPreview, anchor_key_base_rotation, part_edit_guard,
    },
};
use boardstudio_web_ui_model::state::{FieldPreview, FieldPreviewSource, FieldPreviewState};
use boardstudio_web_ui_model::transform_feedback::{FeedbackDelta, FeedbackEvent, FeedbackState, advance_feedback};
use dioxus::prelude::*;
use dioxus_web::WebEventExt;
use std::{cell::RefCell, rc::Rc};
use wasm_bindgen::{JsCast, closure::Closure};
use wasm_bindgen_futures::spawn_local;
use web_sys::SvgElement;

#[derive(Clone)]
pub struct LayoutTransformRuntime(pub Rc<Runtime>);

impl PartialEq for LayoutTransformRuntime {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

/// The rail tools. Matrix selections offer the placement tools; an independent part offers Move,
/// Rotate, Spread and Stagger; both end in the Anchor pick token.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LayoutTransformTool {
    Stagger,
    Spread,
    Offset,
    /// Carries one row or key sideways through its running frame.
    Shift,
    /// Turns one row's or one key's running frame, in degrees.
    Bend,
    Splay,
    Origin,
    /// Moves the selected matrix (or independent part) through its Origin.
    Move,
    /// Turns the selected matrix (or independent part).
    Rotate,
    /// Arms canvas Anchor pick mode; the token toggles and never starts a drag.
    Anchor,
}

impl From<crate::part_rail::PartRailTool> for LayoutTransformTool {
    fn from(tool: crate::part_rail::PartRailTool) -> Self {
        use crate::part_rail::PartRailTool as P;
        match tool {
            P::Move => Self::Move,
            P::Rotate => Self::Rotate,
            P::Spread => Self::Spread,
            P::Stagger => Self::Stagger,
            P::Anchor => Self::Anchor,
        }
    }
}

impl LayoutTransformTool {
    pub(super) const MENU: [(Self, &'static str); 9] = [
        (Self::Stagger, "Stagger"),
        (Self::Spread, "Spread"),
        (Self::Offset, "Offset"),
        (Self::Shift, "Shift"),
        (Self::Bend, "Bend"),
        (Self::Splay, "Splay"),
        (Self::Origin, "Origin"),
        (Self::Move, "Move"),
        (Self::Rotate, "Rotate"),
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Stagger => "Stagger",
            Self::Spread => "Spread",
            Self::Offset => "Offset",
            Self::Shift => "Shift",
            Self::Bend => "Bend",
            Self::Splay => "Splay",
            Self::Origin => "Origin",
            Self::Move => "Move",
            Self::Rotate => "Rotate",
            Self::Anchor => "Anchor",
        }
    }

    /// The Inspector field whose amber preview shows this token's movers, if the tool moves any.
    fn preview_field(self, context: &TreeContext) -> Option<MatrixTransformField> {
        use MatrixTransformField as F;
        match (self, context) {
            (Self::Stagger, TreeContext::Column { .. }) => Some(F::ColumnStagger),
            (Self::Spread, _) => Some(F::ColumnSpread),
            (Self::Splay, _) => Some(F::ColumnSplay),
            (Self::Offset, TreeContext::Row { .. }) => Some(F::RowOffsetX),
            (Self::Offset, _) => Some(F::ColumnOffsetX),
            (Self::Shift, TreeContext::Row { .. }) => Some(F::RowShiftX),
            (Self::Shift, TreeContext::Key { .. }) => Some(F::KeyShiftX),
            (Self::Bend, TreeContext::Row { .. }) => Some(F::RowBend),
            (Self::Bend, TreeContext::Key { .. }) => Some(F::KeyBend),
            (Self::Move, _) => Some(F::OriginX),
            (Self::Rotate, _) => Some(F::MatrixRotation),
            _ => None,
        }
    }
}

/// What choosing a menu tool does: the tool that is active afterwards and the selection kind to
/// switch to. Choosing the active tool again turns it off. Offset keeps a Row selection; Shift and
/// Bend keep a Row or Key; Move, Rotate and Anchor keep the selection; everything else becomes the
/// selection's Column.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ToolActivation {
    pub active: Option<LayoutTransformTool>,
    pub selection_kind: Option<LayoutSelectionKind>,
}

pub fn activate_transform_tool(active: Option<LayoutTransformTool>, tool: LayoutTransformTool, context: &TreeContext) -> ToolActivation {
    use LayoutSelectionKind as Kind;
    use LayoutTransformTool as Tool;
    if active == Some(tool) {
        return ToolActivation { active: None, selection_kind: None };
    }
    let selection_kind = match (tool, context) {
        (Tool::Move | Tool::Rotate | Tool::Anchor, _) => None,
        (Tool::Offset | Tool::Shift | Tool::Bend, TreeContext::Row { .. }) => Some(Kind::Row),
        (Tool::Shift | Tool::Bend, TreeContext::Key { .. }) => Some(Kind::Key),
        _ => Some(Kind::Column),
    };
    ToolActivation { active: Some(tool), selection_kind }
}

/// What the rail draws over: a matrix selection with its evaluated scene, or one independent part.
#[derive(Clone, PartialEq)]
pub enum RailTarget {
    Matrix { matrix: Rc<Matrix>, projection: Rc<MatrixScene>, context: TreeContext },
    Part { part_id: String, snap: GestureSnapInputs, on_focus_half: EventHandler<()> },
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
pub(super) const GLYPH_ANCHOR: &str = "M0-2.5V5M-3-.5h6M-4.5 2.2a4.5 4.5 0 0 0 9 0M0-5.6a1.5 1.5 0 1 0 .01 0";

/// The token glyph path; the one-dimensional tools draw the axis arrow rotated by `degrees`.
fn glyph(tool: LayoutTransformTool, quick: bool, degrees: f64, stroke: &str) -> Element {
    let style = format!("fill:none;stroke:{stroke};stroke-width:1.5px;stroke-linecap:round;stroke-linejoin:round;vector-effect:non-scaling-stroke");
    use LayoutTransformTool as T;
    match (tool, quick) {
        (T::Stagger | T::Offset, false) | (T::Spread, _) => rsx! { g { transform: "rotate({degrees})", path { d: "{GLYPH_ARROWS}", style: "{style}" } } },
        (T::Stagger | T::Offset, true) | (T::Shift | T::Move, _) => rsx! { g { transform: "scale(1.5)", path { d: "{GLYPH_FOUR}", style: "{style}" } } },
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
pub(super) fn world_per_screen(surface: &Rc<RefCell<Option<SvgElement>>>, width: f64, height: f64) -> f64 {
    surface.borrow().as_ref().map_or(1.0, |svg| {
        let rect = svg.get_bounding_client_rect();
        if rect.width() > 0.0 { (width / rect.width()).max(height / rect.height().max(1.0)) } else { 1.0 }
    })
}

/// The rail centred over `points` (horizontally on their extent, just above the topmost).
pub(super) fn rail_over_points(points: &[Vec2], count: usize, px: f64) -> Option<handle_rail::WorldRailLayout> {
    if points.is_empty() {
        return None;
    }
    let min = points.iter().map(|p| p.x).fold(f64::INFINITY, f64::min);
    let max = points.iter().map(|p| p.x).fold(f64::NEG_INFINITY, f64::max);
    let top = points.iter().map(|p| p.y).fold(f64::NEG_INFINITY, f64::max);
    handle_rail::rail_world((min + max) / 2.0, top, count, px)
}

/// The rail's card backdrop, drawn once behind the tokens.
pub(super) fn rail_backdrop(layout: &handle_rail::WorldRailLayout, count: usize, px: f64) -> Element {
    let width = handle_rail::rail_width(count);
    let (cx, cy) = (layout.rail.x + layout.rail.w / 2.0, layout.rail.y + layout.rail.h / 2.0);
    rsx! {
        g { "aria-hidden": "true", transform: "translate({cx} {cy}) scale({px} {-px})", style: "pointer-events:none",
            rect { x: "{-width / 2.0}", y: "-11", width: "{width}", height: "22", rx: "6",
                style: "fill:var(--card);stroke:var(--border-strong);stroke-width:1px;vector-effect:non-scaling-stroke;filter:drop-shadow(0 2px 4px rgb(24 35 49 / 12%))" }
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Pointer protocol

struct Active {
    pointer_id: i32,
    capture: SvgElement,
    transaction_id: String,
    tool: LayoutTransformTool,
    gesture: Option<RailGesture>,
    /// A detached part's Move drives the session body-drag gesture instead of a preview.
    body: Option<Body>,
    interaction: CanvasInteraction,
    token: SnapshotToken,
    revision: u64,
    board_id: String,
    start_client: (f64, f64),
}

struct Body {
    part_id: String,
    start_at: Vec2,
    began: bool,
}

/// How a pointer-down starts, once the accepted snapshot and start point are known.
enum Start {
    Rail(RailGesture),
    Body(Body),
}

type Slot = Rc<RefCell<Option<Active>>>;
type Maker = Rc<dyn Fn(&AcceptedSnapshot, Vec2, bool) -> Option<Start>>;

#[derive(Clone)]
struct Ctl {
    runtime: Rc<Runtime>,
    surface: Rc<RefCell<Option<SvgElement>>>,
    arbiter: CanvasInteractionArbiter,
    drag: Slot,
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
pub(super) fn raise_toast(feedback: Signal<FeedbackState>, mut generation: Signal<u64>, value: String, delta: FeedbackDelta) {
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

impl Ctl {
    fn world(&self, pointer: &web_sys::PointerEvent) -> Option<(Vec2, f64)> {
        let (x, y, w, h) = self.view;
        crate::presentation::coordinates_with_scale(&self.surface, pointer, x, y, w, h)
    }

    fn input<'a>(&'a self, active: &Active, pointer: &web_sys::PointerEvent, point: Vec2) -> Option<RailInput<'a>> {
        let rect = self.surface.borrow().as_ref()?.get_bounding_client_rect();
        if rect.width() <= 0.0 || rect.height() <= 0.0 {
            return None;
        }
        let (_, _, w, h) = self.view;
        let tolerance = w / rect.width() * 8.0;
        Some(RailInput {
            point,
            world_delta: Vec2 {
                x: (f64::from(pointer.client_x()) - active.start_client.0) * w / rect.width(),
                y: -(f64::from(pointer.client_y()) - active.start_client.1) * h / rect.height(),
            },
            alt: pointer.alt_key(),
            pitch: self.part_snap.map_or(self.pitch, |snap| snap.pitch),
            snap_fraction: self.part_snap.map_or(self.snap.snap_fraction, |snap| snap.snap_fraction),
            geometry_snap: self.part_snap.map_or(self.snap.geometry_snap, |snap| snap.geometry_snap),
            origins: &self.origins,
            tolerance,
        })
    }

    fn clear_preview(&self, active: &Active) {
        self.runtime.submit(Event::ClearPreview {
            operation_id: self.runtime.operation(),
            token: active.token,
            revision: active.revision,
            board_id: active.board_id.clone(),
            transaction_id: active.transaction_id.clone(),
        });
    }

    /// Release the pointer and the canvas. The drag leaves its slot first: releasing capture
    /// dispatches `lostpointercapture`.
    fn release(&self, active: &Active) {
        if active.capture.has_pointer_capture(active.pointer_id) {
            let _ = active.capture.release_pointer_capture(active.pointer_id);
        }
        self.arbiter.release(CanvasInteractionOwner::MatrixTransform);
    }

    /// Escape, lost capture, a selection change or unmount: clear the preview, land nothing.
    fn abandon(&self) -> bool {
        let Some(mut active) = self.drag.borrow_mut().take() else { return false };
        match &active.body {
            Some(body) if body.began => self.runtime.submit(Event::GestureCancel { pointer_id: i64::from(active.pointer_id) }),
            _ => {
                if active.interaction.cancel() {
                    self.clear_preview(&active);
                }
            }
        }
        self.release(&active);
        let (mut popup_at, mut guide) = (self.popup_at, self.guide);
        popup_at.set(None);
        guide.set(None);
        feedback_event(self.feedback, FeedbackEvent::Dismiss);
        true
    }

    fn start(&self, pointer: &web_sys::PointerEvent, tool: LayoutTransformTool, make: &Maker) {
        if pointer.button() != 0 || self.drag.borrow().is_some() {
            return;
        }
        let model = self.runtime.model();
        let (Some(snapshot), Some(scope)) = (model.accepted.as_ref(), self.runtime.scope()) else { return };
        if !drag_session_is_ready(&model, Some(snapshot.document.revision)) || !self.arbiter.try_acquire(CanvasInteractionOwner::MatrixTransform) {
            return;
        }
        let capture = pointer
            .target()
            .and_then(|target| target.dyn_into::<web_sys::Element>().ok())
            .and_then(|target| target.closest("[data-transform-handle]").ok().flatten())
            .and_then(|target| target.dyn_into::<SvgElement>().ok())
            .filter(|capture| capture.set_pointer_capture(pointer.pointer_id()).is_ok());
        let started = capture.as_ref().and_then(|_| self.world(pointer)).and_then(|(point, _)| make(snapshot, point, pointer.ctrl_key()).map(|start| (start, point)));
        let (Some(capture), Some((start, point))) = (capture, started) else {
            if let Some(capture) = pointer.target().and_then(|t| t.dyn_into::<SvgElement>().ok()) {
                let _ = capture.release_pointer_capture(pointer.pointer_id());
            }
            self.arbiter.release(CanvasInteractionOwner::MatrixTransform);
            return;
        };
        pointer.prevent_default();
        pointer.stop_propagation();
        let mut interaction = CanvasInteraction::default();
        interaction.begin_transform(pointer.pointer_id());
        let (gesture, body) = match start {
            Start::Rail(gesture) => (Some(gesture), None),
            Start::Body(body) => (None, Some(body)),
        };
        *self.drag.borrow_mut() = Some(Active {
            pointer_id: pointer.pointer_id(),
            capture,
            transaction_id: format!("layout-transform-{}", self.runtime.operation().0),
            tool,
            gesture,
            body,
            interaction,
            token: snapshot.token,
            revision: snapshot.document.revision,
            board_id: scope.board_id,
            start_client: (f64::from(pointer.client_x()), f64::from(pointer.client_y())),
        });
        let mut popup_at = self.popup_at;
        popup_at.set(Some((point.x + 18.0 * self.px, point.y - 18.0 * self.px)));
    }

    /// The preview or body-drag sample for one pointer event; `None` while nothing is proposed.
    fn sample(&self, active: &mut Active, pointer: &web_sys::PointerEvent) -> Option<(Vec2, RailPreview)> {
        let (point, _) = self.world(pointer)?;
        let preview = match (&active.gesture, &active.body) {
            (Some(gesture), _) => gesture.sample(&self.input(active, pointer, point)?)?,
            _ => return None,
        };
        Some((point, preview))
    }

    fn moved(&self, pointer: &web_sys::PointerEvent) {
        let mut slot = self.drag.borrow_mut();
        let Some(active) = slot.as_mut().filter(|active| active.pointer_id == pointer.pointer_id()) else { return };
        pointer.prevent_default();
        pointer.stop_propagation();
        if active.body.is_some() {
            drop(slot);
            return self.body_moved(pointer, false);
        }
        let Some((point, preview)) = self.sample(active, pointer) else { return };
        let (mut guide, mut popup_at) = (self.guide, self.popup_at);
        guide.set(preview.guide);
        if active.interaction.sample(pointer.pointer_id(), &preview.operation) {
            self.runtime.submit(Event::PreviewEdit {
                operation_id: self.runtime.operation(),
                transaction_id: active.transaction_id.clone(),
                target_ids: vec![active.gesture.as_ref().map(|g| g.target_id().to_owned()).unwrap_or_default()],
                operation: preview.operation,
            });
        }
        feedback_event(self.feedback, FeedbackEvent::Drag { name: active.tool.label().to_owned(), value: preview.value, delta: preview.delta });
        popup_at.set(Some((point.x + 18.0 * self.px, point.y - 18.0 * self.px)));
    }

    /// A detached part's Move drives the session body-drag gesture past the shipped threshold.
    fn body_moved(&self, pointer: &web_sys::PointerEvent, finishing: bool) {
        let mut slot = self.drag.borrow_mut();
        let Some(active) = slot.as_mut() else { return };
        let Some((point, scale)) = self.world(pointer) else { return };
        let Some(body) = active.body.as_mut() else { return };
        let moved = crate::presentation::canvas_interaction::pending_part_drag_threshold_reached(
            active.start_client,
            (f64::from(pointer.client_x()), f64::from(pointer.client_y())),
        );
        if !moved && !body.began {
            return;
        }
        let at = Vec2 {
            x: body.start_at.x + (f64::from(pointer.client_x()) - active.start_client.0) / scale,
            y: body.start_at.y - (f64::from(pointer.client_y()) - active.start_client.1) / scale,
        };
        let pointer_id = i64::from(active.pointer_id);
        let part = vec![Position { id: body.part_id.clone(), at }];
        let snap = self.part_snap.unwrap_or(GestureSnapInputs { pitch: self.pitch, snap_fraction: self.snap.snap_fraction, geometry_snap: self.snap.geometry_snap, gap: None });
        if !body.began {
            self.runtime.submit(Event::GestureBegin {
                operation_id: self.runtime.operation(),
                pointer_id,
                target_ids: vec![body.part_id.clone()],
                transaction_id: active.transaction_id.clone(),
                start: vec![Position { id: body.part_id.clone(), at: body.start_at }],
                pitch: snap.pitch,
                snap_fraction: snap.snap_fraction,
                geometry_snap: snap.geometry_snap,
                gap: snap.gap,
                pixels_per_world: Some(scale),
                alt: pointer.alt_key(),
            });
            let admitted = self.runtime.model().gesture.as_ref().is_some_and(|g| g.pointer_id == pointer_id && g.target_ids == vec![body.part_id.clone()]);
            if !admitted {
                drop(slot);
                self.abandon();
                return;
            }
            body.began = true;
        }
        if finishing {
            self.runtime.submit(Event::GestureEnd { pointer_id, final_positions: part, pixels_per_world: Some(scale), alt: pointer.alt_key() });
        } else {
            self.runtime.submit(Event::GestureSample { pointer_id, positions: part, pixels_per_world: Some(scale), alt: pointer.alt_key() });
        }
        let delta = FeedbackDelta::Mm2(at.x - body.start_at.x, at.y - body.start_at.y);
        let value = format!("X {:.2}, Y {:.2} mm", at.x, at.y);
        let (tool, mut popup_at) = (active.tool, self.popup_at);
        drop(slot);
        if finishing {
            self.toast(tool, value, delta);
        } else {
            feedback_event(self.feedback, FeedbackEvent::Drag { name: tool.label().to_owned(), value, delta });
            popup_at.set(Some((point.x + 18.0 * self.px, point.y - 18.0 * self.px)));
        }
    }

    fn toast(&self, tool: LayoutTransformTool, value: String, delta: FeedbackDelta) {
        raise_toast(self.feedback, self.toast_gen, format!("{} {value}", tool.label()), delta);
    }

    fn commit(&self, target: &str, transaction_id: &str, operation: EditOperation) {
        let (target, transaction_id) = (target.to_owned(), transaction_id.to_owned());
        let result = self.runtime.commit(
            "layout-transform-drag",
            EditResolver::new("layout-transform-drag", move |_: &AcceptedSnapshot| {
                Resolution::submit_with_transaction_id(vec![target.clone()], transaction_id.clone(), operation.clone())
            }),
        );
        if let Err(message) = result {
            self.runtime.report(message);
        }
    }

    fn finished(&self, pointer: &web_sys::PointerEvent) {
        if self.drag.borrow().as_ref().is_none_or(|active| active.pointer_id != pointer.pointer_id()) {
            return;
        }
        pointer.prevent_default();
        pointer.stop_propagation();
        if self.drag.borrow().as_ref().is_some_and(|active| active.body.is_some()) {
            self.body_moved(pointer, true);
            let active = self.drag.borrow_mut().take();
            if let Some(active) = active {
                self.release(&active);
            }
            self.clear_chrome();
            return;
        }
        let Some(mut active) = self.drag.borrow_mut().take() else { return };
        let sampled = self.sample(&mut active, pointer);
        let target = active.gesture.as_ref().map(|g| g.target_id().to_owned()).unwrap_or_default();
        let tool = active.tool;
        let finish = active.interaction.finish(pointer.pointer_id(), sampled.as_ref().map(|(_, preview)| preview.operation.clone()));
        match (finish, sampled) {
            (Some(drop), Some((_, preview))) if !preview.delta.is_zero() => {
                if drop.needs_preview {
                    self.runtime.submit(Event::PreviewEdit { operation_id: self.runtime.operation(), transaction_id: active.transaction_id.clone(), target_ids: vec![target.clone()], operation: drop.operation.clone() });
                }
                self.commit(&target, &active.transaction_id, drop.operation);
                self.toast(tool, preview.value, preview.delta);
            }
            (_, preview) => {
                self.clear_preview(&active);
                if let Some((_, preview)) = preview {
                    feedback_event(self.feedback, FeedbackEvent::Choose { name: tool.label().to_owned(), value: preview.value });
                }
            }
        }
        self.release(&active);
        self.clear_chrome();
    }

    fn clear_chrome(&self) {
        let (mut popup_at, mut guide) = (self.popup_at, self.guide);
        popup_at.set(None);
        guide.set(None);
    }
}

/// The document key listener one overlay keeps while mounted: Control (quick offset), Escape.
struct KeyListener {
    document: web_sys::Document,
    window: web_sys::Window,
    down: Closure<dyn FnMut(web_sys::KeyboardEvent)>,
    up: Closure<dyn FnMut(web_sys::KeyboardEvent)>,
    blur: Closure<dyn FnMut(web_sys::Event)>,
}

impl KeyListener {
    fn install(mut control: Signal<bool>, on_escape: impl Fn(&web_sys::KeyboardEvent) -> bool + 'static) -> Option<Self> {
        let window = web_sys::window()?;
        let document = window.document()?;
        let down = Closure::wrap(Box::new(move |event: web_sys::KeyboardEvent| match event.key().as_str() {
            "Control" => { let _ = control.try_write().map(|mut c| *c = true); }
            "Escape" if on_escape(&event) => event.prevent_default(),
            _ => {}
        }) as Box<dyn FnMut(_)>);
        let up = Closure::wrap(Box::new(move |event: web_sys::KeyboardEvent| {
            if event.key() == "Control" {
                let _ = control.try_write().map(|mut c| *c = false);
            }
        }) as Box<dyn FnMut(_)>);
        let blur = Closure::wrap(Box::new(move |_: web_sys::Event| { let _ = control.try_write().map(|mut c| *c = false); }) as Box<dyn FnMut(_)>);
        document.add_event_listener_with_callback("keydown", down.as_ref().unchecked_ref()).ok()?;
        document.add_event_listener_with_callback("keyup", up.as_ref().unchecked_ref()).ok()?;
        window.add_event_listener_with_callback("blur", blur.as_ref().unchecked_ref()).ok()?;
        Some(Self { document, window, down, up, blur })
    }
}

impl Drop for KeyListener {
    fn drop(&mut self) {
        let _ = self.document.remove_event_listener_with_callback("keydown", self.down.as_ref().unchecked_ref());
        let _ = self.document.remove_event_listener_with_callback("keyup", self.up.as_ref().unchecked_ref());
        let _ = self.window.remove_event_listener_with_callback("blur", self.blur.as_ref().unchecked_ref());
    }
}

fn escape_targets_text_entry(event: &web_sys::KeyboardEvent) -> bool {
    event
        .target()
        .and_then(|target| target.dyn_into::<web_sys::Element>().ok())
        .is_some_and(|target| target.closest("input, textarea, select, [contenteditable='true']").ok().flatten().is_some())
}

// ---------------------------------------------------------------------------------------------
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
fn matrix_drag_anchor(accepted: &AcceptedSnapshot, matrix: &Matrix) -> Result<Option<MatrixDragAnchor>, ()> {
    let Some(anchor) = matrix.anchor.as_ref() else { return Ok(None) };
    let key_pose = matrix_drag_anchor_pose(&accepted.scene.matrix_scenes, anchor).ok_or(())?;
    Ok(Some(MatrixDragAnchor { key_pose, follow: anchor.follow, transport: matrix_drag_transport(&accepted.document, &matrix.id) }))
}

/// The Offset-like parameter a Shift or Bend token edits, or `None` when the context has no row/key.
fn carried(context: &TreeContext, bend: bool) -> Option<PlacementParameter> {
    match (context, bend) {
        (TreeContext::Row { row, .. }, false) => Some(PlacementParameter::RowShift { row: *row }),
        (TreeContext::Key { row, column, .. }, false) => Some(PlacementParameter::KeyShift { row: *row, column: *column }),
        (TreeContext::Row { row, .. }, true) => Some(PlacementParameter::RowBend { row: *row }),
        (TreeContext::Key { row, column, .. }, true) => Some(PlacementParameter::KeyBend { row: *row, column: *column }),
        _ => None,
    }
}

/// The matrix tokens for `tools`, each with the gesture it starts.
fn matrix_tokens(
    tools: &[LayoutTransformTool],
    matrix: &Rc<Matrix>,
    projection: &Rc<MatrixScene>,
    context: &TreeContext,
    affect: &MatrixSplayAffect,
    control: bool,
) -> Vec<(Token, Option<Maker>)> {
    use LayoutTransformTool as T;
    use crate::matrix_transform_operation::PlacementValue as V;
    let column = context_column(context).unwrap_or(0);
    let row = match context {
        TreeContext::Row { row, .. } => Some(*row),
        _ => None,
    };
    let basis = projection.columns.iter().find(|basis| basis.column == column);
    let stagger_axis = basis.map_or(Vec2 { x: 0.0, y: 1.0 }, |basis| basis.stagger_axis);
    let axis_degrees = (-stagger_axis.y).atan2(stagger_axis.x).to_degrees();
    let offset_parameter = row.map_or(PlacementParameter::ColumnOffset { column }, |row| PlacementParameter::RowOffset { row });
    let value_of = |parameter| boardstudio_web_layout_core::rail_gesture::format_placement_value(boardstudio_web_layout_core::rail_gesture::placement_current(matrix, parameter));
    let placement = |parameter: PlacementParameter| -> Maker {
        let (matrix, projection) = (matrix.clone(), projection.clone());
        Rc::new(move |_, _, _| {
            let basis = SceneBasis::for_parameter(&projection, parameter)?;
            Some(Start::Rail(RailGesture::Placement { matrix: (*matrix).clone(), parameter, basis }))
        })
    };
    let mut tokens = Vec::new();
    for &tool in tools {
        let token = |label: &str, value: String, title: Option<String>| Token { tool, label: label.to_owned(), value, title, glyph_degrees: axis_degrees, guide: None, disabled: false, nudge: None };
        let entry = match tool {
            T::Stagger if control => Some((Token { guide: Some((stagger_axis.x, stagger_axis.y)), nudge: None, ..token("Drag to offset column", value_of(PlacementParameter::ColumnOffset { column }), None) }, Some(placement(PlacementParameter::ColumnOffset { column })))),
            T::Stagger => {
                let parameter = PlacementParameter::Stagger { column };
                Some((Token { guide: Some((stagger_axis.x, stagger_axis.y)), nudge: row.is_none().then_some(RailNudge::Placement(parameter)), ..token("Drag to stagger", value_of(parameter), None) }, {
                    // Ctrl latched at pointer-down turns the whole drag into a free column offset.
                    let (quick, normal) = (placement(PlacementParameter::ColumnOffset { column }), placement(parameter));
                    Some(Rc::new(move |accepted: &AcceptedSnapshot, point, ctrl| if ctrl { quick(accepted, point, ctrl) } else { normal(accepted, point, ctrl) }) as Maker)
                }))
            }
            T::Spread => {
                let parameter = PlacementParameter::ColumnSpread { column };
                Some((Token { guide: Some((1.0, 0.0)), nudge: Some(RailNudge::Placement(parameter)), ..token("Drag to spread", value_of(parameter), Some(format!("Drag to spread · {}", value_of(parameter)))) }, Some(placement(parameter))))
            }
            T::Offset => Some((Token { guide: Some((stagger_axis.x, stagger_axis.y)), nudge: Some(RailNudge::Placement(offset_parameter)), ..token(if row.is_some() { "Drag to offset row" } else { "Drag to offset column" }, value_of(offset_parameter), None) }, Some(placement(offset_parameter)))),
            T::Shift => carried(context, false).map(|parameter| (Token { nudge: Some(RailNudge::Placement(parameter)), ..token(if row.is_some() { "Drag to shift row" } else { "Drag to shift key" }, value_of(parameter), None) }, Some(placement(parameter)))),
            T::Bend => carried(context, true).map(|parameter| (Token { nudge: Some(RailNudge::Placement(parameter)), ..token(if row.is_some() { "Drag to bend row" } else { "Drag to bend key" }, value_of(parameter), None) }, Some(placement(parameter)))),
            T::Splay | T::Origin => basis.cloned().map(|basis| {
                let (matrix, affect_for_start) = (matrix.clone(), affect.clone());
                let maker: Maker = Rc::new(move |_, point, ctrl| {
                    let origin = basis.splay_origin;
                    Some(Start::Rail(if tool == T::Splay {
                        RailGesture::Splay { matrix: (*matrix).clone(), column, origin, start_angle: (point.y - origin.y).atan2(point.x - origin.x), swing: ctrl, affect: affect_for_start.clone(), column_base: basis.base }
                    } else {
                        RailGesture::Origin { matrix: (*matrix).clone(), column, swing: ctrl, column_base: basis.base, start_origin: origin }
                    }))
                });
                let (splay, stagger_rotation) = (basis.splay_angle, basis.rotation);
                let angle = column_angle(splay, stagger_rotation, affect);
                (Token { nudge: Some(RailNudge::SplayAngle { column, affect: affect.clone() }), ..token(if tool == T::Splay { "Drag to splay" } else { "Move splay origin" }, format!("{angle:.1}°"), Some(format!("Drag to splay · {angle:.0}°"))) }, Some(maker))
            }),
            T::Move | T::Rotate => {
                let (matrix, rotate) = (matrix.clone(), tool == T::Rotate);
                let followed = matrix.anchor.as_ref().is_none_or(|anchor| anchor.follow.rotation);
                let (center, value) = (
                    projection.cells.iter().find(|cell| cell.enabled).or(projection.cells.first()).map_or(matrix.origin, |cell| cell.pose.at),
                    if rotate { format!("{:.2} degrees", matrix.rotation.unwrap_or(0.0)) } else { format!("X {:.2} mm, Y {:.2} mm", matrix.origin.x, matrix.origin.y) },
                );
                let maker: Maker = Rc::new(move |accepted, point, _| {
                    let anchor = matrix_drag_anchor(accepted, &matrix).ok()?;
                    Some(Start::Rail(if rotate {
                        RailGesture::MatrixRotate { matrix: (*matrix).clone(), anchor, center, start_angle: (point.y - center.y).atan2(point.x - center.x) }
                    } else {
                        RailGesture::MatrixMove { matrix: (*matrix).clone(), anchor }
                    }))
                });
                let nudge = if rotate { RailNudge::MatrixRotate } else { RailNudge::MatrixMove { anchor: None } };
                Some((Token { disabled: rotate && !followed, nudge: Some(nudge), ..token(if rotate { "Drag to rotate matrix" } else { "Drag to move matrix" }, value, None) }, Some(maker)))
            }
            T::Anchor => None,
        };
        let _ = V::Stagger(0.0);
        tokens.extend(entry);
    }
    tokens
}

/// The independent part's tokens: Move and Rotate edit its local Offset and Rotation (or pose),
/// Spread and Stagger the values carried from its anchor key.
fn part_tokens(tools: &[LayoutTransformTool], part_id: &str, accepted: &AcceptedSnapshot) -> (Vec<(Token, Option<Maker>)>, Vec2, f64) {
    use LayoutTransformTool as T;
    let document = &accepted.document;
    let Some(part) = document.parts.iter().find(|part| part.id == part_id) else { return (Vec::new(), Vec2::default(), 0.0) };
    let evaluated = evaluated_part_pose(document, part);
    let anchor = part.anchor.clone();
    let base = anchor.as_ref().and_then(|anchor| anchor_key_base_rotation(document, anchor)).unwrap_or(0.0);
    let (spread, stagger, offset, rotation) = anchor.as_ref().map_or((0.0, 0.0, Vec2::default(), evaluated.rotation), |a| (a.spread, a.stagger, a.offset, a.rotation));
    let mut tokens = Vec::new();
    for &tool in tools {
        let (label, value, title, glyph_degrees, guide, axis) = match tool {
            T::Move => ("Drag to move part", format!("X {:.2}, Y {:.2} mm", if anchor.is_some() { offset } else { evaluated.at }.x, if anchor.is_some() { offset } else { evaluated.at }.y), true, 0.0, None, PartAxis::Offset),
            T::Rotate => ("Drag to rotate part", format!("{rotation:.1}°"), true, 0.0, None, PartAxis::Rotation),
            T::Spread => ("Drag to spread", format!("{spread:.2} mm"), true, (-base.to_radians().sin()).atan2(base.to_radians().cos()).to_degrees(), Some((base.to_radians().cos(), base.to_radians().sin())), PartAxis::Spread),
            T::Stagger => ("Drag to stagger", format!("{stagger:.2} mm"), false, base.to_radians().cos().atan2(-base.to_radians().sin()).to_degrees(), Some((-base.to_radians().sin(), base.to_radians().cos())), PartAxis::Stagger),
            _ => continue,
        };
        let (id, part_anchor) = (part_id.to_owned(), anchor.clone());
        let maker: Maker = Rc::new(move |accepted, point, _| {
            let document = &accepted.document;
            let part = document.parts.iter().find(|part| part.id == id)?;
            if part_edit_guard(document, &id, part_anchor.is_some()).is_some() {
                return None;
            }
            let evaluated = evaluated_part_pose(document, part);
            let sweep_start = (point.y - evaluated.at.y).atan2(point.x - evaluated.at.x);
            Some(match (&part.anchor, axis) {
                (None, PartAxis::Offset) => Start::Body(Body { part_id: id.clone(), start_at: evaluated.at, began: false }),
                (None, _) => Start::Rail(RailGesture::PartRotate { part_id: id.clone(), center: evaluated.at, start_angle: sweep_start, start_rotation: evaluated.rotation }),
                (Some(anchor), axis) => Start::Rail(RailGesture::PartValue { part_id: id.clone(), axis, anchor: anchor.clone(), base_rotation: anchor_key_base_rotation(document, anchor).unwrap_or(0.0), center: evaluated.at, start_angle: sweep_start }),
            })
        });
        let token = Token { tool, label: label.to_owned(), value: value.clone(), title: title.then(|| format!("{label} · {value}")), glyph_degrees, guide, disabled: false, nudge: None };
        tokens.push((token, Some(maker)));
    }
    (tokens, evaluated.at, part_courtyard_top(document, &part.definition_id))
}

fn part_courtyard_top(document: &boardstudio_model::model::ProjectDoc, definition_id: &str) -> f64 {
    document
        .definitions
        .iter()
        .find(|definition| definition.id == definition_id)
        .map(|definition| definition.courtyard.iter().map(|point| point.y).fold(f64::NEG_INFINITY, f64::max))
        .filter(|top| top.is_finite())
        .unwrap_or(0.0)
}

impl From<RailGesture> for Start {
    fn from(gesture: RailGesture) -> Self {
        Self::Rail(gesture)
    }
}

// ---------------------------------------------------------------------------------------------
// Overlay

#[component]
pub fn LayoutRailOverlay(
    runtime: LayoutTransformRuntime,
    surface: Rc<RefCell<Option<SvgElement>>>,
    arbiter: CanvasInteractionArbiter,
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
    on_finish: EventHandler<()>,
) -> Element {
    let _ = surface_size.map(|size| size());
    let runtime = runtime.0;
    let control = use_signal(|| false);
    let feedback = use_signal(FeedbackState::default);
    let popup_at = use_signal(|| None::<(f64, f64)>);
    let toast_gen = use_signal(|| 0_u64);
    let guide = use_signal(|| None::<Vec2>);
    let hovered = use_signal(|| None::<usize>);
    let drag: Slot = use_hook(|| Rc::new(RefCell::new(None)));
    let px = world_per_screen(&surface, width, height);
    let (accepted, part_context) = (runtime.model().accepted, match &target {
        RailTarget::Part { part_id, .. } => Some(part_id.clone()),
        RailTarget::Matrix { .. } => None,
    });
    let (pitch, part_snap) = match &target {
        RailTarget::Matrix { matrix, .. } => (matrix.pitch, None),
        RailTarget::Part { snap, .. } => (snap.pitch, Some(*snap)),
    };
    let ctl = Ctl {
        runtime: runtime.clone(),
        surface: surface.clone(),
        arbiter: arbiter.clone(),
        drag: drag.clone(),
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
    use_hook({
        let ctl = ctl.clone();
        move || {
            Rc::new(KeyListener::install(control, move |event| {
                let live = ctl.drag.borrow().is_some();
                if escape_targets_text_entry(event) || !(live || ctl.arbiter.current().is_none()) {
                    return false;
                }
                ctl.abandon();
                on_finish.call(());
                true
            }))
        }
    });
    use_drop({
        let ctl = ctl.clone();
        move || {
            ctl.abandon();
        }
    });
    let context_key = format!("{:?}", match &target {
        RailTarget::Matrix { context, .. } => Some(context.clone()),
        RailTarget::Part { part_id, .. } => Some(TreeContext::Component { part_id: Some(part_id.clone()), matrix_id: None, row: None, column: None, assembly_id: None }),
    });
    use_effect(use_reactive((&context_key, &tools), {
        let ctl = ctl.clone();
        move |_| {
            ctl.abandon();
        }
    }));

    let Some(accepted) = accepted else { return rsx! {} };
    let (tokens, points, matrix_ctx): (Vec<(Token, Option<Maker>)>, Vec<Vec2>, Option<(Rc<Matrix>, Rc<MatrixScene>, TreeContext)>) = match &target {
        RailTarget::Matrix { matrix, projection, context } => {
            let tokens = matrix_tokens(&tools, matrix, projection, context, &splay_affect(), control());
            let cells = projection.cells.iter().filter(|cell| cell.enabled);
            let points: Vec<Vec2> = match context {
                TreeContext::Column { column, .. } => cells.filter(|c| c.column == *column).map(|c| c.pose.at).collect(),
                TreeContext::Row { row, .. } => cells.filter(|c| c.row == *row).map(|c| c.pose.at).collect(),
                TreeContext::Key { row, column, .. } => cells.filter(|c| c.row == *row && c.column == *column).map(|c| c.pose.at).collect(),
                _ => cells.map(|c| c.pose.at).collect(),
            };
            let fallback = projection.cells.iter().find(|cell| cell.enabled).or(projection.cells.first()).map_or(matrix.origin, |cell| cell.pose.at);
            (tokens, if points.is_empty() && matches!(context, TreeContext::Matrix { .. }) { vec![fallback] } else { points }, Some((matrix.clone(), projection.clone(), context.clone())))
        }
        RailTarget::Part { part_id, .. } => {
            let (tokens, at, courtyard_top) = part_tokens(&tools, part_id, &accepted);
            (tokens, vec![Vec2 { x: at.x, y: at.y + courtyard_top }], None)
        }
    };
    let slots = tools.iter().position(|tool| *tool == LayoutTransformTool::Anchor).map_or(tokens.len(), |_| tokens.len() + 1);
    let layout = rail_over_points(&points, slots.max(1), px).filter(|_| !tokens.is_empty() || slots > 0);
    let origin_point = matrix_ctx.as_ref().filter(|_| tools.iter().any(|t| matches!(t, LayoutTransformTool::Splay | LayoutTransformTool::Origin))).and_then(|(_, projection, context)| {
        let column = context_column(context)?;
        projection.columns.iter().find(|basis| basis.column == column).map(|basis| basis.splay_origin)
    });
    let Some(layout) = layout else { return rsx! {} };
    let on_focus_half = match &target {
        RailTarget::Part { on_focus_half, .. } => Some(*on_focus_half),
        _ => None,
    };
    let _ = part_context;
    let feedback_state = feedback();
    let preview = try_use_context::<FieldPreview>();
    let preview_source = try_use_context::<Option<FieldPreviewSource>>().flatten();
    let affect_now = splay_affect();
    let show_preview = {
        let (context, matrix_ctx) = (matrix_ctx.as_ref().map(|(_, _, c)| c.clone()), matrix_ctx.is_some());
        move |tool: LayoutTransformTool| {
            let (Some(target), Some(source), Some(context), true) = (preview, preview_source.clone(), context.clone(), matrix_ctx) else { return };
            if let Some(field) = tool.preview_field(&context)
                && let Some(facts) = matrix_relations::field_preview(&source.document, &source.scene.matrix_scenes, &context, field, affect_now.clone())
            {
                target.show(FieldPreviewState { parts: facts.parts, hint: facts.hint });
            }
        }
    };
    let hide_preview = move || {
        if let Some(target) = preview {
            target.clear();
        }
    };

    rsx! {
        {rail_backdrop(&layout, slots, px)}
        if let Some(point) = guide() {
            g { class: "pointer-events-none fill-none stroke-primary stroke-[0.45] [&_circle]:fill-card", "aria-hidden": "true",
                circle { cx: "{point.x}", cy: "{point.y}", r: "{width / 170.0}" }
            }
        }
        for (index, (token, maker)) in tokens.into_iter().enumerate() {
            {
                let (tx, ty) = layout.token_centers.get(index).copied().unwrap_or((0.0, 0.0));
                let dragging = ctl.drag.borrow().as_ref().is_some_and(|active| active.tool == token.tool);
                let lit = hovered() == Some(index);
                let (fill, stroke) = token_paint(dragging, lit);
                let quick = control() && matches!(token.tool, LayoutTransformTool::Stagger | LayoutTransformTool::Offset);
                let face = token_face(fill, glyph(token.tool, quick, token.glyph_degrees, stroke), token.title.as_deref());
                let description = format!("{}{}", token.value, if token.tool == LayoutTransformTool::Stagger { ". Hold Ctrl while dragging to offset the column." } else { "" });
                let opacity = if ctl.drag.borrow().as_ref().is_some_and(|active| active.tool != token.tool) { 0.35 } else { 1.0 };
                let (ctl, maker, token_for_key) = (ctl.clone(), maker.unwrap_or_else(|| Rc::new(|_, _, _| None)), token.clone());
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
                        onpointerdown: { let (ctl, maker, on_focus_half) = (ctl.clone(), maker.clone(), on_focus_half); move |event: PointerEvent| {
                            if let Some(on_focus_half) = on_focus_half { on_focus_half.call(()); }
                            if token_for_key.disabled { return; }
                            if let Some(pointer) = event.data().try_as_web_event() { ctl.start(&pointer, token_for_key.tool, &maker); }
                        } },
                        onpointermove: { let ctl = ctl.clone(); move |event: PointerEvent| { if let Some(pointer) = event.data().try_as_web_event() { ctl.moved(&pointer); } } },
                        onpointerup: { let ctl = ctl.clone(); move |event: PointerEvent| { if let Some(pointer) = event.data().try_as_web_event() { ctl.finished(&pointer); } } },
                        onpointercancel: { let ctl = ctl.clone(); move |_| { ctl.abandon(); } },
                        onlostpointercapture: { let ctl = ctl.clone(); move |_| { ctl.abandon(); } },
                        onkeydown: {
                            let ctl = ctl.clone();
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
                                if !boardstudio_web_layout_core::rail_drag::nudge_session_is_ready(&ctl.runtime.model()) { return; }
                                if let Some(operation) = nudge.operation(matrix, projection, arrow, key.shift_key()) {
                                    key.prevent_default();
                                    key.stop_propagation();
                                    // The step applies to the matrix accepted when it runs; focus stays on the token.
                                    ctl.commit(&matrix.id, "", operation);
                                }
                            }
                        },
                        onpointerenter: { let (mut feedback, mut popup_at, name, value, show) = (ctl.feedback, ctl.popup_at, name.clone(), value.clone(), show.clone()); move |_| { hovered.set(Some(index)); show(token.tool); popup_at.set(None); feedback_event(feedback, FeedbackEvent::Choose { name: name.clone(), value: value.clone() }); let _ = &mut feedback; } },
                        onpointerleave: { let feedback = ctl.feedback; move |_| { hovered.set(None); hide(); feedback_event(feedback, FeedbackEvent::Dismiss); } },
                        onfocus: { let (feedback, mut popup_at) = (ctl.feedback, ctl.popup_at); move |_| { show(token.tool); popup_at.set(None); feedback_event(feedback, FeedbackEvent::Choose { name: name_focus.clone(), value: value_focus.clone() }); } },
                        onblur: { let feedback = ctl.feedback; move |_| { hide(); feedback_event(feedback, FeedbackEvent::Dismiss); } },
                        {face}
                    }
                }
            }
        }
        if let (Some(origin), Some((matrix, projection, context))) = (origin_point, matrix_ctx.clone()) {
            {origin_handle(&ctl, &matrix, &projection, &context, origin, px)}
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
fn origin_handle(ctl: &Ctl, matrix: &Rc<Matrix>, projection: &Rc<MatrixScene>, context: &TreeContext, origin: Vec2, px: f64) -> Element {
    let column = context_column(context).unwrap_or(0);
    let basis = projection.columns.iter().find(|basis| basis.column == column).cloned();
    let maker: Maker = {
        let matrix = matrix.clone();
        Rc::new(move |_, _, ctrl| {
            let basis = basis.clone()?;
            Some(Start::Rail(RailGesture::Origin { matrix: (*matrix).clone(), column, swing: ctrl, column_base: basis.base, start_origin: basis.splay_origin }))
        })
    };
    let (start, moved, finished, cancelled, lost) = (ctl.clone(), ctl.clone(), ctl.clone(), ctl.clone(), ctl.clone());
    rsx! {
        g { "data-transform-handle": "true", class: "outline-none focus-visible:stroke-foreground",
            transform: "translate({origin.x} {origin.y}) scale({px} {-px})", role: "button", tabindex: "0", "aria-label": "Move splay origin",
            "aria-description": "Hold Ctrl while dragging to swing the origin without moving the keys.",
            style: "cursor:grab;touch-action:none",
            onpointerdown: move |event: PointerEvent| { if let Some(pointer) = event.data().try_as_web_event() { start.start(&pointer, LayoutTransformTool::Origin, &maker); } },
            onpointermove: move |event: PointerEvent| { if let Some(pointer) = event.data().try_as_web_event() { moved.moved(&pointer); } },
            onpointerup: move |event: PointerEvent| { if let Some(pointer) = event.data().try_as_web_event() { finished.finished(&pointer); } },
            onpointercancel: move |_| { cancelled.abandon(); },
            onlostpointercapture: move |_| { lost.abandon(); },
            rect { x: "-12", y: "-12", width: "24", height: "24", style: "fill:transparent;stroke:none;pointer-events:all" }
            circle { r: "3", style: "fill:var(--card);stroke:var(--primary);stroke-width:1.5px;vector-effect:non-scaling-stroke" }
            path { d: "M-6 0h12M0 -6v12", style: "fill:none;stroke:var(--primary);stroke-width:1px;vector-effect:non-scaling-stroke" }
            title { "Move splay origin" }
        }
    }
}
