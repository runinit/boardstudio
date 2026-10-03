//! The Layout canvas admits one owner-backed placement interaction at a time.
//!
//! This arbitrates the Layout canvas' owner-backed pointer workflows. It is not
//! a general gesture framework.
use std::{cell::Cell, rc::Rc};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum CanvasInteractionOwner {
    MirroredPair,
    MatrixPlacement,
    PartPlacement,
    OutlinePerimeter,
    MatrixTransform,
}

#[derive(Clone, Default)]
pub(super) struct CanvasInteractionArbiter {
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
    pub(super) fn try_acquire(&self, owner: CanvasInteractionOwner) -> bool {
        if self.owner.get().is_some() {
            return false;
        }
        self.owner.set(Some(owner));
        true
    }

    pub(super) fn is_owner(&self, owner: CanvasInteractionOwner) -> bool {
        self.owner.get() == Some(owner)
    }

    pub(super) fn current(&self) -> Option<CanvasInteractionOwner> {
        self.owner.get()
    }

    pub(super) fn release(&self, owner: CanvasInteractionOwner) {
        if self.is_owner(owner) {
            self.owner.set(None);
        }
    }
}

/// Whether a pending Layout part drag has real client-pointer movement.
/// Keep this tied to pointer coordinates so a canvas
/// resize after pointer-down cannot manufacture movement in world space.
pub(super) fn pending_part_drag_threshold_reached(
    start_client: (f64, f64),
    current_client: (f64, f64),
) -> bool {
    let dx = current_client.0 - start_client.0;
    let dy = current_client.1 - start_client.1;
    dx != 0.0 || dy != 0.0
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
}
