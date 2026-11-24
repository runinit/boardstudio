//! Mounted Layout interaction state shared by the workbench and SVG tools.

use boardstudio_application::Scope;
use boardstudio_model::model::{Position, Vec2};
pub use boardstudio_web_layout_core::interaction::{
    ActiveMarquee, ActivePan, ActivePartDrag, CanvasEvent, CanvasInteraction, CanvasKey,
    CanvasMode, InteractionEffect, MatrixPlacementPayload, MirroredPairPayload,
    PartPlacementPayload,
};
use dioxus::prelude::*;
use std::rc::Rc;
use web_sys::PointerEvent;

type PointerGesture = Rc<dyn Fn(&PointerEvent, bool)>;

#[derive(Clone, Copy, PartialEq)]
pub struct LayoutInteraction {
    state: Signal<CanvasInteraction>,
    cancel: Signal<Option<Rc<dyn Fn()>>>,
    gesture: Signal<Option<PointerGesture>>,
    control: Signal<bool>,
    version: Signal<u64>,
}

pub fn use_layout_interaction() -> LayoutInteraction {
    LayoutInteraction {
        state: use_signal(CanvasInteraction::default),
        cancel: use_signal(|| None),
        gesture: use_signal(|| None),
        control: use_signal(|| false),
        version: use_signal(|| 0),
    }
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

    fn begin_with(self, event: CanvasEvent, mode: CanvasMode) -> bool {
        if self.current().is_some() {
            return false;
        }
        self.handle(event);
        self.current() == Some(mode)
    }

    /// Arms part placement with the session payload; the canvas routes moves
    /// until the session commits or cancels.
    pub fn begin_part_placement(self, payload: PartPlacementPayload) -> bool {
        self.begin_with(
            CanvasEvent::BeginPartPlacement(payload),
            CanvasMode::PlacingPart,
        )
    }

    pub fn begin_matrix_placement(self, payload: MatrixPlacementPayload) -> bool {
        self.begin_with(
            CanvasEvent::BeginMatrixPlacement(payload),
            CanvasMode::PlacingMatrix,
        )
    }

    pub fn begin_mirrored_pair_placement(self, payload: MirroredPairPayload) -> bool {
        self.begin_with(
            CanvasEvent::BeginMirroredPairPlacement(payload),
            CanvasMode::PlacingMirroredPair,
        )
    }

    pub fn seed_pan(self, scope: Scope, generation: u64, start_camera: Vec2) {
        self.handle(CanvasEvent::SeedPan {
            scope,
            generation,
            start_camera,
        });
    }

    pub fn seed_part_drag(
        self,
        scope: Scope,
        generation: u64,
        origin_mm: Vec2,
        positions: Vec<Position>,
    ) {
        self.handle(CanvasEvent::SeedPartDrag {
            scope,
            generation,
            origin_mm,
            positions,
        });
    }

    pub fn seed_marquee(self, additive: bool) {
        self.handle(CanvasEvent::SeedMarquee { additive });
    }

    pub fn pan(self) -> Option<ActivePan> {
        self.state.peek().pan()
    }

    pub fn part_drag(self) -> Option<ActivePartDrag> {
        self.state.peek().part_drag()
    }

    pub fn marquee(self) -> Option<ActiveMarquee> {
        self.state.peek().marquee()
    }

    pub fn placing_part(self) -> Option<PartPlacementPayload> {
        self.state.peek().placing_part()
    }

    pub fn placing_matrix(self) -> Option<MatrixPlacementPayload> {
        self.state.peek().placing_matrix()
    }

    pub fn placing_pair(self) -> Option<MirroredPairPayload> {
        self.state.peek().placing_pair()
    }

    pub fn set_part_drag_gesture_generation(mut self, generation: Option<u64>) {
        self.state
            .write()
            .set_part_drag_gesture_generation(generation);
    }

    pub fn note_part_drag_sample(mut self, sample: Option<(Vec2, f64, bool)>) {
        self.state.write().note_part_drag_sample(sample);
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

    pub fn end(self, mode: CanvasMode) {
        if self.current() == Some(mode) {
            self.handle(CanvasEvent::End(mode));
        }
    }

    pub fn on_cancel(mut self, cancel: impl Fn() + 'static) {
        self.cancel.set(Some(Rc::new(cancel)));
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
        let (effects, control) = {
            let mut state = self.state.write();
            let effects = state.handle(event);
            (effects, state.control_down)
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
                cancel();
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
