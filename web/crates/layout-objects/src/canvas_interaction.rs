//! Mounted Layout interaction state shared by the workbench and SVG tools.

use crate::objects::GestureSnapInputs;
use crate::runtime::Runtime;
use boardstudio_application::{Event, Scope};
use boardstudio_model::model::{Position, Vec2};
pub use boardstudio_web_layout_core::interaction::{
    ActiveOutlineDrag, ActivePan, ActivePartDrag, ActiveRailDrag, CanvasEvent, CanvasInteraction,
    CanvasKey, CanvasMode, GestureSeed, InteractionEffect, OutlineHandleSeed, RailBodyDrag,
    RailDragKind, RailDragSeed,
};
use boardstudio_web_layout_core::outline_snapping;
use boardstudio_web_ui_model::selection::ScopedDrag;
use boardstudio_web_ui_model::transform_feedback::FeedbackDelta;
use dioxus::prelude::*;
use std::rc::Rc;
use web_sys::PointerEvent;

type PointerGesture = Rc<dyn Fn(&PointerEvent, bool)>;

#[derive(Clone, Copy, PartialEq)]
pub struct LayoutInteraction {
    state: Signal<CanvasInteraction>,
    cancel: Signal<Option<Rc<dyn Fn()>>>,
    canceled_drag: Signal<Option<ScopedDrag>>,
    canceled_rail_drag: Signal<Option<ActiveRailDrag>>,
    canceled_outline_drag: Signal<Option<ActiveOutlineDrag>>,
    gesture: Signal<Option<PointerGesture>>,
    control: Signal<bool>,
    version: Signal<u64>,
}

pub fn use_layout_interaction() -> LayoutInteraction {
    LayoutInteraction {
        state: use_signal(CanvasInteraction::default),
        cancel: use_signal(|| None),
        canceled_drag: use_signal(|| None),
        canceled_rail_drag: use_signal(|| None),
        canceled_outline_drag: use_signal(|| None),
        gesture: use_signal(|| None),
        control: use_signal(|| false),
        version: use_signal(|| 0),
    }
}

fn active_scoped_drag(state: &CanvasInteraction) -> Option<ScopedDrag> {
    state
        .part_drag()
        .map(|drag| ScopedDrag {
            pointer_id: drag.pointer_id,
            scope: drag.scope,
            gesture_generation: drag.gesture_generation,
            active: drag.active,
            pan: false,
        })
        .or_else(|| {
            state.pan().map(|pan| ScopedDrag {
                pointer_id: pan.pointer_id,
                scope: pan.scope,
                gesture_generation: None,
                active: true,
                pan: true,
            })
        })
}

impl LayoutInteraction {
    pub fn version(self) -> u64 {
        (self.version)()
    }

    pub fn current(self) -> Option<CanvasMode> {
        self.state.peek().mode()
    }

    pub fn pointer(self) -> Option<i32> {
        self.state.peek().pointer()
    }

    pub fn space_down(self) -> bool {
        self.state.peek().space_down
    }

    pub fn control_down(self) -> bool {
        (self.control)()
    }

    pub fn peek_control_down(self) -> bool {
        self.state.peek().control_down
    }

    pub fn quick_offset(self) -> bool {
        self.state.peek().quick_offset()
    }

    pub fn begin(self, mode: CanvasMode) -> bool {
        if self.current().is_some() {
            return false;
        }
        self.handle(CanvasEvent::Begin(mode));
        self.current() == Some(mode)
    }

    pub fn pan(self) -> Option<ActivePan> {
        self.state.peek().pan()
    }

    pub fn part_drag(self) -> Option<ActivePartDrag> {
        self.state.peek().part_drag()
    }

    pub fn rail_drag(self) -> Option<ActiveRailDrag> {
        self.state.peek().rail_drag()
    }

    pub fn outline_drag(self) -> Option<ActiveOutlineDrag> {
        self.state.peek().outline_drag()
    }

    pub fn set_part_drag_gesture_generation(mut self, generation: Option<u64>) {
        self.state
            .write()
            .set_part_drag_gesture_generation(generation);
    }

    pub fn note_part_drag_sample(mut self, sample: Option<(Vec2, f64, bool)>) {
        self.state.write().note_part_drag_sample(sample);
    }

    pub fn note_rail_preview(mut self, preview: Option<(String, FeedbackDelta)>) {
        self.state.write().note_rail_preview(preview);
    }

    pub fn note_rail_body_sample(mut self, sample: Option<(Vec2, f64, bool)>) {
        self.state.write().note_rail_body_sample(sample);
    }

    pub fn sample_outline_drag(
        mut self,
        world: Vec2,
        paths: &[Vec<Vec2>],
        origins: &[outline_snapping::Origin],
        options: outline_snapping::Options,
    ) {
        self.state
            .write()
            .sample_outline_drag(world, paths, origins, options);
    }

    pub fn note_outline_preview_submitted(mut self) {
        self.state.write().note_outline_preview_submitted();
    }

    pub fn switch_pick(self, mode: CanvasMode) -> bool {
        if let Some(current) = self.current()
            && current.is_pick()
            && current != mode
        {
            self.handle(CanvasEvent::ScopeChanged);
        }
        self.begin(mode)
    }

    pub fn sync_control(mut self, down: bool) {
        if *self.control.peek() != down && self.state.write().sync_control(down) {
            self.control.set(down);
        }
    }

    pub fn capture_drag(
        self,
        id: i32,
        client: (f64, f64),
        control: bool,
        seed: GestureSeed,
        capture: &web_sys::SvgElement,
        focus: Option<&web_sys::SvgElement>,
    ) -> bool {
        // Modifiers can be stale when callers stop propagation before workbench handlers see them.
        self.sync_control(control);
        let mode = seed.mode();
        if self.handle(CanvasEvent::PointerDown {
            id,
            client,
            control,
            seed: Some(seed),
        }) != [InteractionEffect::CapturePointer(id)]
        {
            return false;
        }
        if capture.set_pointer_capture(id).is_err() {
            self.end(mode);
            return false;
        }
        focus_svg(focus.unwrap_or(capture));
        true
    }

    pub fn end(self, mode: CanvasMode) {
        if self.current() == Some(mode) {
            self.handle(CanvasEvent::End(mode));
        }
    }

    pub fn cancel(self, id: i32) {
        self.handle(CanvasEvent::PointerCancel(id));
    }

    pub fn pointer_up(self, id: i32) {
        self.handle(CanvasEvent::PointerUp {
            id,
            operation: None,
        });
    }

    pub fn pointer_down(self, id: i32, client: (f64, f64), control: bool) {
        self.handle(CanvasEvent::PointerDown {
            id,
            client,
            control,
            seed: None,
        });
    }

    pub fn on_cancel(mut self, cancel: impl Fn() + 'static) {
        self.cancel.set(Some(Rc::new(cancel)));
    }

    pub fn take_canceled_drag(mut self) -> Option<ScopedDrag> {
        self.canceled_drag.write().take()
    }

    pub fn take_canceled_rail_drag(mut self) -> Option<ActiveRailDrag> {
        self.canceled_rail_drag.write().take()
    }

    pub fn take_canceled_outline_drag(mut self) -> Option<ActiveOutlineDrag> {
        self.canceled_outline_drag.write().take()
    }

    pub fn cancel_scoped(self, expected_scope: &Scope) {
        let pointer_id = self
            .part_drag()
            .filter(|drag| drag.scope == *expected_scope)
            .map(|drag| drag.pointer_id)
            .or_else(|| {
                self.pan()
                    .filter(|pan| pan.scope == *expected_scope)
                    .map(|pan| pan.pointer_id)
            });
        if let Some(pointer_id) = pointer_id {
            self.handle(CanvasEvent::PointerCancel(pointer_id));
        }
    }

    pub fn on_pointer(mut self, gesture: impl Fn(&PointerEvent, bool) + 'static) {
        self.gesture.set(Some(Rc::new(gesture)));
    }

    /// Captured rail and outline input is delivered by the mounted SVG.
    pub fn route_pointer(self, pointer: &PointerEvent, released: bool) -> bool {
        let gesture = self.gesture.peek().clone();
        let Some(gesture) = gesture else { return false };
        if self.pointer() == Some(pointer.pointer_id()) {
            gesture(pointer, released);
        }
        true
    }

    pub fn handle(mut self, event: CanvasEvent) -> Vec<InteractionEffect> {
        let previous = self.current();
        let (effects, control, active_drag, active_rail, active_outline) = {
            let mut state = self.state.write();
            let active_drag = active_scoped_drag(&state);
            let active_rail = state.rail_drag();
            let active_outline = state.outline_drag();
            let effects = state.handle(event);
            (
                effects,
                state.control_down,
                active_drag,
                active_rail,
                active_outline,
            )
        };
        if *self.control.peek() != control {
            self.control.set(control);
        }
        if previous.is_some() && self.current().is_none() {
            self.version += 1;
            self.gesture.write().take();
            let cancel = self.cancel.write().take();
            if effects
                .iter()
                .any(|effect| matches!(effect, InteractionEffect::Cancel(_)))
                && let Some(cancel) = cancel
            {
                *self.canceled_drag.write() = active_drag;
                *self.canceled_rail_drag.write() = active_rail;
                *self.canceled_outline_drag.write() = active_outline;
                cancel();
                self.canceled_drag.write().take();
                self.canceled_rail_drag.write().take();
                self.canceled_outline_drag.write().take();
            }
        }
        effects
    }
}

pub fn client_point(pointer: &web_sys::PointerEvent) -> (f64, f64) {
    (f64::from(pointer.client_x()), f64::from(pointer.client_y()))
}

pub fn focus_svg(surface: &web_sys::SvgElement) {
    let options = web_sys::FocusOptions::new();
    options.set_prevent_scroll(true);
    let _ = surface.focus_with_options(&options);
}

pub fn release_capture(surface: &std::cell::RefCell<Option<web_sys::SvgElement>>, pointer_id: i32) {
    if let Some(capture) = surface.borrow().as_ref()
        && capture.has_pointer_capture(pointer_id)
    {
        let _ = capture.release_pointer_capture(pointer_id);
    }
}

pub fn begin_body_drag(
    runtime: &Rc<Runtime>,
    interaction: LayoutInteraction,
    pointer_id: i32,
    tx_id: Option<&str>,
    start: &[Position],
    snap: GestureSnapInputs,
    (scale, alt): (f64, bool),
) -> Option<u64> {
    let target_ids: Vec<_> = start.iter().map(|p| p.id.clone()).collect();
    let op = runtime.operation();
    let transaction_id = tx_id.map_or_else(|| format!("drag-{}", op.0), str::to_owned);
    runtime.submit(Event::GestureBegin {
        operation_id: op,
        pointer_id: i64::from(pointer_id),
        target_ids: target_ids.clone(),
        transaction_id,
        start: start.to_vec(),
        pitch: snap.pitch,
        snap_fraction: snap.snap_fraction,
        geometry_snap: snap.geometry_snap,
        gap: snap.gap,
        pixels_per_world: Some(scale),
        alt,
    });
    let gesture_gen = runtime
        .model()
        .gesture
        .filter(|g| g.pointer_id == i64::from(pointer_id) && g.target_ids == target_ids)
        .map(|g| g.generation);
    if gesture_gen.is_none() {
        interaction.handle(CanvasEvent::PointerCancel(pointer_id));
    }
    gesture_gen
}

pub fn submit_gesture_step(
    runtime: &Rc<Runtime>,
    pointer_id: i32,
    positions: Vec<Position>,
    scale: f64,
    alt: bool,
    end: bool,
) {
    let pointer_id = i64::from(pointer_id);
    runtime.submit(if end {
        Event::GestureEnd {
            pointer_id,
            final_positions: positions,
            pixels_per_world: Some(scale),
            alt,
        }
    } else {
        Event::GestureSample {
            pointer_id,
            positions,
            pixels_per_world: Some(scale),
            alt,
        }
    });
}
