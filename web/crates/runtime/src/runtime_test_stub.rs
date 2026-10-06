//! Minimal native-test Runtime surface for mounted Editor-owner regression tests.
use boardstudio_application::{Event, OperationId, ReadModel, Scope, TerminalOutcome};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

pub struct Runtime {
    pub model: RefCell<ReadModel>,
    scope: RefCell<Option<Scope>>,
    executor_epoch: Cell<u64>,
    next_operation: Cell<u64>,
    pub events: RefCell<Vec<Event>>,
    pub outcomes: crate::operation_outcomes::OperationOutcomes,
}

impl Runtime {
    pub fn new(model: ReadModel, scope: Scope) -> Rc<Self> {
        Rc::new(Self {
            model: RefCell::new(model),
            scope: RefCell::new(Some(scope)),
            executor_epoch: Cell::new(4),
            next_operation: Cell::new(1),
            events: RefCell::new(Vec::new()),
            outcomes: Default::default(),
        })
    }

    pub fn model(&self) -> ReadModel {
        self.model.borrow().clone()
    }

    pub fn scope(&self) -> Option<Scope> {
        self.scope.borrow().clone()
    }

    pub fn set_scope(&self, scope: Option<Scope>) {
        *self.scope.borrow_mut() = scope;
    }

    pub fn electrical_preview_executor_epoch(&self) -> u64 {
        self.executor_epoch.get()
    }

    pub fn operation(&self) -> OperationId {
        let next = self.next_operation.get();
        self.next_operation.set(next + 1);
        OperationId(next)
    }

    pub fn observe_operation(
        &self,
        operation: OperationId,
    ) -> crate::operation_outcomes::OutcomeSlot {
        self.outcomes.observe(operation)
    }

    pub fn submit(&self, event: Event) {
        self.events.borrow_mut().push(event);
    }

    pub fn settle(&self, operation: OperationId, outcome: TerminalOutcome) -> bool {
        self.outcomes.settle(operation, outcome)
    }
}
