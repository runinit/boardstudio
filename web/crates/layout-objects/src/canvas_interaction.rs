//! Mounted Layout interaction state shared by the workbench and SVG tools.

use boardstudio_web_layout_core::interaction::CanvasInteraction;
pub use boardstudio_web_layout_core::interaction::{
    CanvasEvent, CanvasKey, CanvasMode, InteractionEffect, MarqueeBounds,
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
    marquee: Signal<Option<MarqueeBounds>>,
}

pub fn use_layout_interaction() -> LayoutInteraction {
    LayoutInteraction {
        state: use_signal(CanvasInteraction::default),
        cancel: use_signal(|| None),
        gesture: use_signal(|| None),
        control: use_signal(|| false),
        version: use_signal(|| 0),
        marquee: use_signal(|| None),
    }
}

impl LayoutInteraction {
    pub fn marquee(self) -> Option<MarqueeBounds> {
        (self.marquee)()
    }

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
        for effect in &effects {
            if let InteractionEffect::PreviewMarquee(bounds) = effect {
                self.marquee.set(Some(*bounds));
            }
        }
        if previous.is_some() && self.current().is_none() {
            if previous == Some(CanvasMode::Marquee) {
                self.marquee.set(None);
            }
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
