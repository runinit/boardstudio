//! Minimal native-test Runtime surface for mounted Editor-owner regression tests.
use boardstudio_application::{Event, OperationId, ReadModel, Scope, TerminalOutcome};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

pub(crate) struct Runtime {
    pub(crate) model: RefCell<ReadModel>,
    scope: RefCell<Option<Scope>>,
    executor_epoch: Cell<u64>,
    next_operation: Cell<u64>,
    pub(crate) events: RefCell<Vec<Event>>,
    pub(crate) outcomes: crate::operation_outcomes::OperationOutcomes,
}

impl Runtime {
    pub(crate) fn new(model: ReadModel, scope: Scope) -> Rc<Self> {
        Rc::new(Self {
            model: RefCell::new(model),
            scope: RefCell::new(Some(scope)),
            executor_epoch: Cell::new(4),
            next_operation: Cell::new(1),
            events: RefCell::new(Vec::new()),
            outcomes: Default::default(),
        })
    }

    pub(crate) fn model(&self) -> ReadModel {
        self.model.borrow().clone()
    }

    pub(crate) fn scope(&self) -> Option<Scope> {
        self.scope.borrow().clone()
    }

    pub(crate) fn electrical_preview_executor_epoch(&self) -> u64 {
        self.executor_epoch.get()
    }

    pub(crate) fn operation(&self) -> OperationId {
        let next = self.next_operation.get();
        self.next_operation.set(next + 1);
        OperationId(next)
    }

    pub(crate) fn observe_operation(
        &self,
        operation: OperationId,
    ) -> crate::operation_outcomes::OutcomeSlot {
        self.outcomes.observe(operation)
    }

    pub(crate) fn submit(&self, event: Event) {
        self.events.borrow_mut().push(event);
    }

    pub(crate) fn settle(&self, operation: OperationId, outcome: TerminalOutcome) -> bool {
        self.outcomes.settle(operation, outcome)
    }
}
