use std::{cell::Cell, rc::Rc};

/// Owner token for detached standard-fit Core requests. Checking this token is
/// the first operation after an await and before touching editor-owned Signals.
#[derive(Clone)]
pub(crate) struct PartsStandardProfileLifetime(Rc<Cell<bool>>);

impl PartsStandardProfileLifetime {
    pub(crate) fn new() -> Self {
        Self(Rc::new(Cell::new(true)))
    }

    pub(crate) fn retire(&self) {
        self.0.set(false);
    }

    pub(crate) fn run_if_mounted<T>(&self, continuation: impl FnOnce() -> T) -> Option<T> {
        self.0.get().then(continuation)
    }
}
