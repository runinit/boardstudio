//! Shared one-shot test behavior for persistence adapters.

use std::cell::RefCell;

/// The next call either fails, or is held until its adapter's test gate is released.
#[derive(Debug)]
pub enum OneShotBehavior {
    Fail(String),
    Hold,
}

/// Stores one-shot save behavior without prescribing how an adapter waits.
/// The browser adapter waits asynchronously; the native Runtime parks the effect and
/// resumes it from its explicit `release_*` methods.
#[derive(Default)]
pub struct GateDriver {
    save: RefCell<Option<OneShotBehavior>>,
}

impl GateDriver {
    pub fn fail_next_save(&self, reason: impl Into<String>) {
        *self.save.borrow_mut() = Some(OneShotBehavior::Fail(reason.into()));
    }

    pub fn hold_next_save(&self) {
        *self.save.borrow_mut() = Some(OneShotBehavior::Hold);
    }

    pub fn take_save(&self) -> Option<OneShotBehavior> {
        self.save.borrow_mut().take()
    }
}
