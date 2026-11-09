//! Shared one-shot test behavior for in-process Core and persistence adapters.

use std::cell::RefCell;

/// The next call either fails, or is held until its adapter's test gate is released.
#[derive(Debug)]
pub(crate) enum OneShotBehavior {
    Fail(String),
    Hold,
}

/// Stores one-shot Core and save behavior without prescribing how an adapter waits.
/// The browser adapter waits asynchronously; the native Runtime parks the effect and
/// resumes it from its explicit `release_*` methods.
#[derive(Default)]
pub(crate) struct GateDriver {
    core: RefCell<Option<OneShotBehavior>>,
    save: RefCell<Option<OneShotBehavior>>,
}

impl GateDriver {
    pub(crate) fn fail_next_core(&self, reason: impl Into<String>) {
        *self.core.borrow_mut() = Some(OneShotBehavior::Fail(reason.into()));
    }

    pub(crate) fn hold_next_core(&self) {
        *self.core.borrow_mut() = Some(OneShotBehavior::Hold);
    }

    pub(crate) fn take_core(&self) -> Option<OneShotBehavior> {
        self.core.borrow_mut().take()
    }

    pub(crate) fn fail_next_save(&self, reason: impl Into<String>) {
        *self.save.borrow_mut() = Some(OneShotBehavior::Fail(reason.into()));
    }

    pub(crate) fn hold_next_save(&self) {
        *self.save.borrow_mut() = Some(OneShotBehavior::Hold);
    }

    pub(crate) fn take_save(&self) -> Option<OneShotBehavior> {
        self.save.borrow_mut().take()
    }
}
