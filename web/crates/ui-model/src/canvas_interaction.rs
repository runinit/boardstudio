//! The Layout canvas admits one owner-backed placement interaction at a time.
//!
//! This arbitrates the Layout canvas' owner-backed pointer workflows. It is not
//! a general gesture framework.
use std::{cell::Cell, rc::Rc};
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::{JsCast, closure::Closure};
#[cfg(target_arch = "wasm32")]
use web_sys::{HtmlElement, KeyboardEvent, Window};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CanvasInteractionOwner {
    MirroredPair,
    MatrixPlacement,
    PartPlacement,
    OutlinePerimeter,
    MatrixTransform,
}

#[derive(Clone, Default)]
pub struct CanvasInteractionArbiter {
    owner: Rc<Cell<Option<CanvasInteractionOwner>>>,
}

impl PartialEq for CanvasInteractionArbiter {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.owner, &other.owner)
    }
}

impl CanvasInteractionArbiter {
    /// Atomically claim the canvas for a new interaction. Repeated or competing
    /// starts are rejected synchronously, before either workflow mutates state.
    pub fn try_acquire(&self, owner: CanvasInteractionOwner) -> bool {
        if self.owner.get().is_some() {
            return false;
        }
        self.owner.set(Some(owner));
        true
    }

    pub fn is_owner(&self, owner: CanvasInteractionOwner) -> bool {
        self.owner.get() == Some(owner)
    }

    pub fn current(&self) -> Option<CanvasInteractionOwner> {
        self.owner.get()
    }

    pub fn release(&self, owner: CanvasInteractionOwner) {
        if self.is_owner(owner) {
            self.owner.set(None);
        }
    }
}

/// Whether a pending Layout part drag has real client-pointer movement.
/// Keep this tied to pointer coordinates so a canvas
/// resize after pointer-down cannot manufacture movement in world space.
pub fn pending_part_drag_threshold_reached(
    start_client: (f64, f64),
    current_client: (f64, f64),
) -> bool {
    let dx = current_client.0 - start_client.0;
    let dy = current_client.1 - start_client.1;
    dx != 0.0 || dy != 0.0
}

/// Whether a Layout-level keyboard event should enable Space-drag panning.
/// This is intentionally independent of canvas focus so pointer-over-canvas
/// Space gestures work before the SVG has received a click or keyboard focus.
pub fn layout_space_pan_keydown(key: &str, code: &str, typing_target: bool) -> bool {
    !typing_target && (key == " " || code == "Space")
}

#[cfg(target_arch = "wasm32")]
pub struct LayoutSpacePanWindowListener {
    window: Window,
    keydown: Closure<dyn FnMut(KeyboardEvent)>,
    keyup: Closure<dyn FnMut(KeyboardEvent)>,
}

#[cfg(target_arch = "wasm32")]
impl LayoutSpacePanWindowListener {
    pub fn install(is_layout: Rc<dyn Fn() -> bool>, space_down: Rc<Cell<bool>>) -> Option<Self> {
        let window = web_sys::window()?;
        let keydown_space = space_down.clone();
        let keydown = Closure::wrap(Box::new(move |event: KeyboardEvent| {
            let typing_target = event
                .target()
                .and_then(|target| target.dyn_into::<HtmlElement>().ok())
                .is_some_and(|target| {
                    matches!(target.tag_name().as_str(), "INPUT" | "TEXTAREA" | "SELECT")
                });
            if is_layout() && layout_space_pan_keydown(&event.key(), &event.code(), typing_target) {
                keydown_space.set(true);
                event.prevent_default();
            }
        }) as Box<dyn FnMut(_)>);
        let keyup_space = space_down;
        let keyup = Closure::wrap(Box::new(move |event: KeyboardEvent| {
            if event.key() == " " || event.code() == "Space" {
                keyup_space.set(false);
            }
        }) as Box<dyn FnMut(_)>);
        window
            .add_event_listener_with_callback("keydown", keydown.as_ref().unchecked_ref())
            .ok()?;
        if window
            .add_event_listener_with_callback("keyup", keyup.as_ref().unchecked_ref())
            .is_err()
        {
            let _ = window
                .remove_event_listener_with_callback("keydown", keydown.as_ref().unchecked_ref());
            return None;
        }
        Some(Self {
            window,
            keydown,
            keyup,
        })
    }
}

#[cfg(target_arch = "wasm32")]
impl Drop for LayoutSpacePanWindowListener {
    fn drop(&mut self) {
        let _ = self
            .window
            .remove_event_listener_with_callback("keydown", self.keydown.as_ref().unchecked_ref());
        let _ = self
            .window
            .remove_event_listener_with_callback("keyup", self.keyup.as_ref().unchecked_ref());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[wasm_bindgen_test::wasm_bindgen_test]
    fn either_start_order_admits_only_the_first_canvas_owner() {
        for (first, second) in [
            (
                CanvasInteractionOwner::MirroredPair,
                CanvasInteractionOwner::PartPlacement,
            ),
            (
                CanvasInteractionOwner::PartPlacement,
                CanvasInteractionOwner::MirroredPair,
            ),
            (
                CanvasInteractionOwner::MatrixTransform,
                CanvasInteractionOwner::OutlinePerimeter,
            ),
            (
                CanvasInteractionOwner::OutlinePerimeter,
                CanvasInteractionOwner::MatrixTransform,
            ),
        ] {
            let arbiter = CanvasInteractionArbiter::default();
            assert!(arbiter.try_acquire(first));
            assert!(!arbiter.try_acquire(second));
            assert_eq!(arbiter.current(), Some(first));
            arbiter.release(second);
            assert_eq!(arbiter.current(), Some(first));
            arbiter.release(first);
            assert_eq!(arbiter.current(), None);
        }
    }

    #[wasm_bindgen_test::wasm_bindgen_test]
    fn pending_part_drag_requires_real_client_motion() {
        let start = (100.0, 200.0);

        // Even a one-pixel movement must be admitted, matching the reference.
        assert!(pending_part_drag_threshold_reached(start, (101.0, 200.0)));
        assert!(pending_part_drag_threshold_reached(start, (104.0, 200.0)));

        // If panel reflow changes the SVG/world mapping while the pointer stays
        // at the same client coordinates, the pending click must remain still.
        assert!(!pending_part_drag_threshold_reached(start, start));
    }

    #[wasm_bindgen_test::wasm_bindgen_test]
    fn layout_space_pan_uses_global_space_key_without_a_focus_precondition() {
        assert!(layout_space_pan_keydown(" ", "Space", false));
        assert!(layout_space_pan_keydown("Space", "Space", false));
        assert!(!layout_space_pan_keydown(" ", "Space", true));
        assert!(!layout_space_pan_keydown("x", "KeyX", false));

        let space_down = Rc::new(Cell::new(false));
        let is_layout = Rc::new(Cell::new(true));
        let listener = LayoutSpacePanWindowListener::install(
            {
                let is_layout = is_layout.clone();
                Rc::new(move || is_layout.get())
            },
            space_down.clone(),
        )
        .expect("window listeners install");
        let window = web_sys::window().expect("window");
        let init = web_sys::KeyboardEventInit::new();
        init.set_key(" ");
        init.set_code("Space");
        init.set_bubbles(true);
        init.set_cancelable(true);
        let down = web_sys::KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &init)
            .expect("Space keydown");
        window.dispatch_event(&down).unwrap();
        assert!(space_down.get(), "window Space keydown enables pan");
        assert!(down.default_prevented(), "Space does not scroll the page");

        let up = web_sys::KeyboardEvent::new_with_keyboard_event_init_dict("keyup", &init)
            .expect("Space keyup");
        window.dispatch_event(&up).unwrap();
        assert!(!space_down.get(), "window Space keyup releases pan");

        is_layout.set(false);
        space_down.set(false);
        window.dispatch_event(&down).unwrap();
        assert!(
            !space_down.get(),
            "other workspaces do not acquire Layout pan"
        );
        drop(listener);
    }
}
