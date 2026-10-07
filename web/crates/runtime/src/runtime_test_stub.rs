//! Native test Runtime: Session and Core are real; browser effects use in-memory ports.
#[cfg(any(test, feature = "test-support"))]
use crate::gate_driver::{GateDriver, OneShotBehavior};
use boardstudio_application::{
    Completion, Effect, Event, OperationId, ReadModel, RequestId, SaveResult, Scope, Session,
};
use boardstudio_core::{
    CoreEngine,
    model::{CoreRequest, ProjectDoc},
};
use std::{
    cell::{Cell, RefCell},
    collections::{BTreeMap, VecDeque},
    rc::Rc,
};

type ParkedCore = (
    RequestId,
    boardstudio_application::ExecutorEpoch,
    Box<CoreRequest>,
);

/// A synchronous adapter for the browser Runtime's session-driven edit behavior.
///
/// The public surface deliberately matches what native-mounted workspace tests need.
/// A submitted event runs through Session, CoreEngine and memory saves until it settles or
/// reaches a one-shot test gate.
pub struct Runtime {
    session: RefCell<Session>,
    engine: RefCell<CoreEngine>,
    pub(crate) outcomes: crate::operation_outcomes::OperationOutcomes,
    next_operation: Cell<u64>,
    saves: RefCell<BTreeMap<String, ProjectDoc>>,
    #[cfg(any(test, feature = "test-support"))]
    gates: GateDriver,
    #[cfg(any(test, feature = "test-support"))]
    core_entered: Cell<bool>,
    #[cfg(any(test, feature = "test-support"))]
    save_entered: Cell<bool>,
    #[cfg(any(test, feature = "test-support"))]
    parked_core: RefCell<Option<ParkedCore>>,
    #[cfg(any(test, feature = "test-support"))]
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
            engine: RefCell::new(CoreEngine::new()),
            outcomes: Default::default(),
            next_operation: Cell::new(1),
            saves: RefCell::new(BTreeMap::new()),
            #[cfg(any(test, feature = "test-support"))]
            gates: GateDriver::default(),
            #[cfg(any(test, feature = "test-support"))]
            core_entered: Cell::new(false),
            #[cfg(any(test, feature = "test-support"))]
            save_entered: Cell::new(false),
            #[cfg(any(test, feature = "test-support"))]
            parked_core: RefCell::new(None),
            #[cfg(any(test, feature = "test-support"))]
            parked_save: RefCell::new(None),
            events: EventLog::new(),
        })
    }

    pub fn model(&self) -> ReadModel {
        self.session.borrow().read_model().clone()
    }

    pub fn scope(&self) -> Option<Scope> {
        self.session.borrow().scope()
    }

    pub fn electrical_preview_executor_epoch(&self) -> u64 {
        self.session.borrow().core_executor_epoch().0
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
        self.outcomes.observe(operation)
    }

    pub fn submit(&self, event: Event) {
        self.events.push(event.clone());
        let effects = self.session.borrow_mut().submit(event);
        self.drive(effects.into());
    }

    #[cfg(any(test, feature = "test-support"))]
    pub fn hold_next_core(&self) {
        self.core_entered.set(false);
        self.gates.hold_next_core();
    }

    #[cfg(any(test, feature = "test-support"))]
    pub fn fail_next_core(&self, reason: impl Into<String>) {
        self.gates.fail_next_core(reason);
    }

    #[cfg(any(test, feature = "test-support"))]
    pub fn hold_next_save(&self) {
        self.save_entered.set(false);
        self.gates.hold_next_save();
    }

    #[cfg(any(test, feature = "test-support"))]
    pub fn fail_next_save(&self, reason: impl Into<String>) {
        self.gates.fail_next_save(reason);
    }

    /// Whether the held Core request has reached the in-memory adapter.
    #[cfg(any(test, feature = "test-support"))]
    pub fn core_entered(&self) -> bool {
        self.core_entered.get()
    }

    /// Whether the held save has reached the in-memory adapter.
    #[cfg(any(test, feature = "test-support"))]
    pub fn save_entered(&self) -> bool {
        self.save_entered.get()
    }

    #[cfg(any(test, feature = "test-support"))]
    pub fn release_core(&self) {
        let Some((request_id, executor_epoch, request)) = self.parked_core.borrow_mut().take()
        else {
            panic!("no parked Core request");
        };
        self.core_entered.set(false);
        let reply = self.engine.borrow_mut().handle(*request);
        let effects = self.session.borrow_mut().complete(Completion::Core {
            request_id,
            executor_epoch,
            reply: Box::new(reply),
        });
        self.drive(effects.into());
    }

    #[cfg(any(test, feature = "test-support"))]
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
        let effects = self.session.borrow_mut().complete(Completion::Persist {
            save_attempt_id,
            result: SaveResult::Committed,
        });
        self.drive(effects.into());
    }

    fn drive(&self, mut effects: VecDeque<Effect>) {
        while let Some(effect) = effects.pop_front() {
            match effect {
                Effect::Core {
                    request_id,
                    executor_epoch,
                    request,
                    ..
                } => effects.extend(self.run_core(request_id, executor_epoch, request)),
                Effect::Persist {
                    save_attempt_id,
                    document,
                    ..
                } => effects.extend(self.run_save(save_attempt_id, document)),
                Effect::Settled {
                    operation_id,
                    outcome,
                    landing,
                } => {
                    self.outcomes
                        .settle_with_landing(operation_id, outcome, landing);
                }
                Effect::RestartCoreExecutor { executor_epoch } => {
                    effects.extend(
                        self.session
                            .borrow_mut()
                            .complete(Completion::ExecutorRestarted { executor_epoch }),
                    );
                }
                _ => {}
            }
        }
    }
}

impl Runtime {
    #[cfg(any(test, feature = "test-support"))]
    fn run_core(
        &self,
        request_id: RequestId,
        executor_epoch: boardstudio_application::ExecutorEpoch,
        request: Box<CoreRequest>,
    ) -> VecDeque<Effect> {
        match self.gates.take_core() {
            Some(OneShotBehavior::Fail(reason)) => self
                .session
                .borrow_mut()
                .complete(Completion::CoreFailed {
                    request_id,
                    executor_epoch,
                    reason,
                })
                .into(),
            Some(OneShotBehavior::Hold) => {
                self.core_entered.set(true);
                *self.parked_core.borrow_mut() = Some((request_id, executor_epoch, request));
                VecDeque::new()
            }
            None => {
                let reply = self.engine.borrow_mut().handle(*request);
                self.session
                    .borrow_mut()
                    .complete(Completion::Core {
                        request_id,
                        executor_epoch,
                        reply: Box::new(reply),
                    })
                    .into()
            }
        }
    }

    #[cfg(not(any(test, feature = "test-support")))]
    fn run_core(
        &self,
        request_id: RequestId,
        executor_epoch: boardstudio_application::ExecutorEpoch,
        request: Box<CoreRequest>,
    ) -> VecDeque<Effect> {
        let reply = self.engine.borrow_mut().handle(*request);
        self.session
            .borrow_mut()
            .complete(Completion::Core {
                request_id,
                executor_epoch,
                reply: Box::new(reply),
            })
            .into()
    }

    #[cfg(any(test, feature = "test-support"))]
    fn run_save(
        &self,
        save_attempt_id: boardstudio_application::SaveAttemptId,
        document: std::sync::Arc<ProjectDoc>,
    ) -> VecDeque<Effect> {
        match self.gates.take_save() {
            Some(OneShotBehavior::Fail(reason)) => self
                .session
                .borrow_mut()
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

    #[cfg(not(any(test, feature = "test-support")))]
    fn run_save(
        &self,
        save_attempt_id: boardstudio_application::SaveAttemptId,
        document: std::sync::Arc<ProjectDoc>,
    ) -> VecDeque<Effect> {
        self.commit_save(save_attempt_id, &document)
    }

    fn commit_save(
        &self,
        save_attempt_id: boardstudio_application::SaveAttemptId,
        document: &ProjectDoc,
    ) -> VecDeque<Effect> {
        self.saves
            .borrow_mut()
            .insert(document.id.clone(), document.clone());
        self.session
            .borrow_mut()
            .complete(Completion::Persist {
                save_attempt_id,
                result: SaveResult::Committed,
            })
            .into()
    }
}
