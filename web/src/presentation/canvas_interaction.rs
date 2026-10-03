//! The Layout canvas admits one owner-backed placement interaction at a time.
//!
//! This is intentionally limited to the existing mirrored-pair and controller
//! placement workflows; it is not a general gesture framework.
use std::{cell::Cell, rc::Rc};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum CanvasInteractionOwner {
    MirroredPair,
    PartPlacement,
    OutlinePerimeter,
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
}
