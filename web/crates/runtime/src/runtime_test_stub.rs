//! Native test Runtime: Session and Core are real; browser effects use in-memory ports.
use crate::gate_driver::{GateDriver, OneShotBehavior};
use boardstudio_application::{
    Completion, Effect, Event, Landing, OperationId, ReadModel, SaveResult, Scope, Session,
    TerminalOutcome,
};
use boardstudio_model::model::ProjectDoc;
use std::{
    cell::{Cell, RefCell},
    collections::{BTreeMap, VecDeque},
    rc::Rc,
};

/// A synchronous adapter for the browser Runtime's session-driven edit behavior.
///
/// The public surface deliberately matches what native-mounted workspace tests need.
/// A submitted event runs through Session, CoreEngine and memory saves until it settles or
/// reaches a one-shot test gate.
pub struct Runtime {
    session: RefCell<Session>,
    pub(crate) operation_outcomes: crate::operation_outcomes::OperationOutcomes,
    next_operation: Cell<u64>,
    saves: RefCell<BTreeMap<String, ProjectDoc>>,
    gates: GateDriver,
    save_entered: Cell<bool>,
    parked_save: RefCell<Option<(boardstudio_application::SaveAttemptId, ProjectDoc)>>,
    /// Events submitted through this Runtime, retained for read-only test assertions.
    pub events: EventLog,
}

/// A read-only view of submitted events. Mutation stays inside the Runtime driver.
pub struct EventLog(RefCell<Vec<Event>>);

impl EventLog {
    fn new() -> Self {
        Self(RefCell::new(Vec::new()))
    }

    pub fn borrow(&self) -> std::cell::Ref<'_, Vec<Event>> {
        self.0.borrow()
    }

    fn push(&self, event: Event) {
        self.0.borrow_mut().push(event);
    }
}

impl Runtime {
    pub fn new() -> Rc<Self> {
        Rc::new(Self {
            session: RefCell::new(Session::new()),
            operation_outcomes: Default::default(),
            next_operation: Cell::new(1),
            saves: RefCell::new(BTreeMap::new()),
            gates: GateDriver::default(),
            save_entered: Cell::new(false),
            parked_save: RefCell::new(None),
            events: EventLog::new(),
        })
    }

    pub fn model(&self) -> ReadModel {
        self.with_model(Clone::clone)
    }

    pub fn with_model<R>(&self, f: impl FnOnce(&ReadModel) -> R) -> R {
        f(self.session.borrow().read_model())
    }

    pub fn scope(&self) -> Option<Scope> {
        self.session.borrow().scope()
    }

    pub fn operation(&self) -> OperationId {
        let operation = OperationId(self.next_operation.get());
        self.next_operation.set(
            self.next_operation
                .get()
                .checked_add(1)
                .expect("operation identity exhausted"),
        );
        operation
    }

    pub fn observe_operation(
        &self,
        operation: OperationId,
    ) -> crate::operation_outcomes::OutcomeSlot {
        self.operation_outcomes.observe(operation)
    }

    pub fn commit(
        &self,
        label: &str,
        resolver: boardstudio_application::EditResolver,
    ) -> Result<boardstudio_application::AcceptedSnapshot, String> {
        let operation_id = self.operation();
        let outcome = self.submit_observed(
            Event::ResolveEdit {
                operation_id,
                label: label.into(),
                resolver,
            },
            Some(operation_id),
        );
        crate::operation_outcomes::commit_result(outcome.map(|(outcome, _)| outcome), || {
            self.with_model(|model| model.accepted.clone())
        })
    }

    pub fn submit(&self, event: Event) {
        self.submit_observed(event, None);
    }

    pub(crate) fn submit_observed(
        &self,
        event: Event,
        observe: Option<OperationId>,
    ) -> Option<(TerminalOutcome, Option<Landing>)> {
        self.events.push(event.clone());
        let previous_scope = self.scope();
        let effects = self.session.borrow_mut().submit(event);
        if self.scope() != previous_scope {
            self.operation_outcomes.retire_edit_scope();
        }
        let result = crate::operation_outcomes::observed_outcome(&effects, observe);
        self.drive(effects.into());
        result
    }

    pub fn hold_next_save(&self) {
        self.save_entered.set(false);
        self.gates.hold_next_save();
    }

    pub fn fail_next_save(&self, reason: impl Into<String>) {
        self.gates.fail_next_save(reason);
    }

    /// Whether the held save has reached the in-memory adapter.
    pub fn save_entered(&self) -> bool {
        self.save_entered.get()
    }

    pub fn release_save(&self) {
        let Some((save_attempt_id, document)) = self.parked_save.borrow_mut().take() else {
            panic!("no parked save");
        };
        self.save_entered.set(false);
        self.saves
            .borrow_mut()
            .insert(document.id.clone(), document);
        self.finish_save(save_attempt_id);
    }

    pub fn saved_document(&self, project_id: &str) -> Option<ProjectDoc> {
        self.saves.borrow().get(project_id).cloned()
    }

    fn finish_save(&self, save_attempt_id: boardstudio_application::SaveAttemptId) {
        let effects = self.complete(Completion::Persist {
            save_attempt_id,
            result: SaveResult::Committed,
        });
        self.drive(effects.into());
    }

    fn complete(&self, event: Completion) -> Vec<Effect> {
        let previous_scope = self.scope();
        let effects = self.session.borrow_mut().complete(event);
        if self.scope() != previous_scope {
            self.operation_outcomes.retire_edit_scope();
        }
        effects
    }

    fn drive(&self, mut effects: VecDeque<Effect>) {
        while let Some(effect) = effects.pop_front() {
            match effect {
                Effect::Persist {
                    save_attempt_id,
                    document,
                    ..
                } => effects.extend(self.run_save(save_attempt_id, document)),
                Effect::Settled {
                    operation_id,
                    outcome,
                    ..
                } => {
                    self.operation_outcomes.settle(operation_id, outcome);
                }
                _ => {}
            }
        }
    }
}

impl Runtime {
    fn run_save(
        &self,
        save_attempt_id: boardstudio_application::SaveAttemptId,
        document: std::sync::Arc<ProjectDoc>,
    ) -> VecDeque<Effect> {
        match self.gates.take_save() {
            Some(OneShotBehavior::Fail(reason)) => self
                .complete(Completion::Persist {
                    save_attempt_id,
                    result: SaveResult::Aborted(reason),
                })
                .into(),
            Some(OneShotBehavior::Hold) => {
                self.save_entered.set(true);
                *self.parked_save.borrow_mut() = Some((save_attempt_id, (*document).clone()));
                VecDeque::new()
            }
            None => self.commit_save(save_attempt_id, &document),
        }
    }

    fn commit_save(
        &self,
        save_attempt_id: boardstudio_application::SaveAttemptId,
        document: &ProjectDoc,
    ) -> VecDeque<Effect> {
        self.saves
            .borrow_mut()
            .insert(document.id.clone(), document.clone());
        self.complete(Completion::Persist {
            save_attempt_id,
            result: SaveResult::Committed,
        })
        .into()
    }
}
