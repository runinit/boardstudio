//! One edit ticket for every panel: submit a pending edit as intent and read what happened
//! to it. The ticket owns the operation identity, observation, the outcome-to-settlement
//! mapping, the standard failure wording and the owner-liveness gate, so controllers repeat
//! none of it.
//!
//! The module compiles natively (no Dioxus, no browser handles); its tests run under
//! `cargo test -p boardstudio-web-runtime` against a real `Session` and `CoreEngine`.
//!
//! Controllers use it like this:
//!
//! ```ignore
//! // When the user commits a draft, begin a ticket for it:
//! self.ticket = Some(EditTicket::begin(
//!     runtime.as_ref(), "layout-inspector", Some("layout".into()), resolver));
//! // On each render, read the settlement and answer owner liveness:
//! match self.ticket.as_ref().unwrap().settlement(owner_is_live) {
//!     Settlement::Pending => { /* keep showing the draft value */ }
//!     Settlement::Landed { .. } => { /* show the accepted value */ }
//!     Settlement::Failed { message } => { /* show the accepted value plus message */ }
//!     Settlement::Retired => { self.ticket = None; /* show nothing */ }
//! }
//! ```
use crate::operation_outcomes::{LandingSlot, OutcomeSlot};
use boardstudio_application::{
    DOCUMENT_SESSION_CHANGED, EditResolver, Event, Landing, OperationId, TerminalOutcome,
};

/// How a pending edit settled, as the controller should present it.
#[derive(Clone, Debug, PartialEq)]
pub enum Settlement {
    /// The edit has not settled yet: keep showing the draft value. One-shot controls
    /// disable themselves while a ticket answers [`EditTicket::is_pending`] (ADR-0005
    /// amendment).
    Pending,
    /// The edit is part of the accepted document at this revision.
    Landed { revision: u64 },
    /// The edit did not land: show the accepted value again with this message inline.
    /// Nothing retries automatically.
    Failed { message: String },
    /// The owner is gone, or the session moved on before the edit ran: show nothing and
    /// drop the ticket.
    Retired,
}

/// The submit/observe port a ticket drives. Two adapters exist: the browser Runtime
/// (wasm) and, for native tests, the in-process driver in [`tests`].
pub trait EditTicketPort {
    fn allocate_operation(&self) -> OperationId;
    fn observe(&self, operation: OperationId) -> (OutcomeSlot, LandingSlot);
    fn submit(&self, event: Event);
}

pub struct EditTicket {
    operation: OperationId,
    outcome: OutcomeSlot,
    landing: LandingSlot,
    feature: Option<String>,
}

impl Clone for EditTicket {
    fn clone(&self) -> Self {
        // The observation slots are shared: a clone observes the same settlement.
        Self {
            operation: self.operation,
            outcome: self.outcome.clone(),
            landing: self.landing.clone(),
            feature: self.feature.clone(),
        }
    }
}

impl EditTicket {
    /// Begin a pending edit for `resolver`. The outcome and landing are observed before
    /// the event is submitted, so a synchronous settlement cannot be missed. `feature` is
    /// the noun used in failure wording (for example `"layout"`).
    pub fn begin(
        port: &dyn EditTicketPort,
        label: &str,
        feature: Option<String>,
        resolver: EditResolver,
    ) -> Self {
        let operation = port.allocate_operation();
        let (outcome, landing) = port.observe(operation);
        port.submit(Event::ResolveEdit {
            operation_id: operation,
            label: label.to_owned(),
            resolver,
        });
        Self {
            operation,
            outcome,
            landing,
            feature,
        }
    }

    /// The operation the edit runs under.
    pub fn operation(&self) -> OperationId {
        self.operation
    }

    /// Whether the edit has not settled yet. One-shot actions disable their control while
    /// this answers true.
    pub fn is_pending(&self) -> bool {
        self.outcome.borrow().is_none()
    }

    /// Read the settlement. `owner_is_live` answers whether the ticket's owner (the
    /// selection, panel or project the edit belongs to) is still current; a departed owner
    /// retires the ticket whatever the outcome.
    pub fn settlement(&self, owner_is_live: bool) -> Settlement {
        if !owner_is_live {
            return Settlement::Retired;
        }
        let Some(outcome) = self.outcome.borrow().clone() else {
            return Settlement::Pending;
        };
        let landing = *self.landing.borrow();
        settlement_of(outcome, landing, self.feature.as_deref())
    }
}

/// The one mapping from a terminal outcome and its landing to a settlement.
fn settlement_of(
    outcome: TerminalOutcome,
    landing: Option<Landing>,
    feature: Option<&str>,
) -> Settlement {
    let subject = feature.unwrap_or("edit");
    match outcome {
        TerminalOutcome::Completed => match landing {
            Some(landing) => Settlement::Landed {
                revision: landing.revision,
            },
            None => Settlement::Failed {
                message: format!(
                    "The {subject} change could not be confirmed where it landed. The accepted value was restored."
                ),
            },
        },
        TerminalOutcome::Rejected(reason) if reason == DOCUMENT_SESSION_CHANGED => {
            Settlement::Retired
        }
        TerminalOutcome::Rejected(reason) => Settlement::Failed {
            message: format!("The {subject} change was refused: {reason}"),
        },
        TerminalOutcome::BlockedByRecovery(reason) => Settlement::Failed {
            message: format!(
                "The {subject} change was refused while saving is in recovery: {reason}"
            ),
        },
        TerminalOutcome::PersistenceFailed(reason) => Settlement::Failed {
            message: format!(
                "The {subject} completed, but the accepted document did not save: {reason}. Retry after recovery."
            ),
        },
        TerminalOutcome::ExecutorFailed(reason) => Settlement::Failed {
            message: format!("The {subject} change could not be applied: {reason}"),
        },
        TerminalOutcome::Superseded | TerminalOutcome::Cancelled | TerminalOutcome::Closed => {
            Settlement::Retired
        }
    }
}

mod runtime_port {
    use super::*;
    use crate::runtime::Runtime;

    impl EditTicketPort for std::rc::Rc<Runtime> {
        fn allocate_operation(&self) -> OperationId {
            Runtime::operation(self)
        }

        fn observe(&self, operation: OperationId) -> (OutcomeSlot, LandingSlot) {
            #[cfg(target_arch = "wasm32")]
            {
                self.observe_operation_with_landing(operation)
            }
            #[cfg(not(target_arch = "wasm32"))]
            {
                self.outcomes.observe_with_landing(operation)
            }
        }

        fn submit(&self, event: Event) {
            Runtime::submit(self, event)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use boardstudio_application::{
        Completion, Effect, ExecutorEpoch, RequestId, Resolution, SaveAttemptId, SaveResult,
        Session,
    };
    use boardstudio_core::model::{
        CoreRequest, Part, PartDefinition, PartKind, Pose2, ProjectDoc, Side, Vec2,
    };
    use std::cell::{Cell, RefCell};
    use std::collections::{BTreeMap, VecDeque};
    use std::rc::Rc;

    /// Drives a real `Session` and `CoreEngine` in-process, keeping saves in memory, with
    /// gates that park the next Core reply or save so tests can observe Pending.
    struct NativeEditDriver {
        session: RefCell<Session>,
        engine: RefCell<boardstudio_core::CoreEngine>,
        outcomes: crate::operation_outcomes::OperationOutcomes,
        next_operation: Cell<u64>,
        saves: RefCell<BTreeMap<String, ProjectDoc>>,
        hold_next_core: Cell<bool>,
        hold_next_save: Cell<bool>,
        fail_next_core: RefCell<Option<String>>,
        fail_next_save: RefCell<Option<String>>,
        parked_core: RefCell<Option<(RequestId, ExecutorEpoch, Box<CoreRequest>)>>,
        parked_save: RefCell<Option<SaveAttemptId>>,
    }

    impl NativeEditDriver {
        fn new() -> Self {
            Self {
                session: RefCell::new(Session::new()),
                engine: RefCell::new(boardstudio_core::CoreEngine::new()),
                outcomes: Default::default(),
                next_operation: Cell::new(1),
                saves: RefCell::new(BTreeMap::new()),
                hold_next_core: Cell::new(false),
                hold_next_save: Cell::new(false),
                fail_next_core: RefCell::new(None),
                fail_next_save: RefCell::new(None),
                parked_core: RefCell::new(None),
                parked_save: RefCell::new(None),
            }
        }

        fn hold_next_core(&self) {
            self.hold_next_core.set(true);
        }

        fn hold_next_save(&self) {
            self.hold_next_save.set(true);
        }

        fn fail_next_core(&self, reason: &str) {
            *self.fail_next_core.borrow_mut() = Some(reason.into());
        }

        fn fail_next_save(&self, reason: &str) {
            *self.fail_next_save.borrow_mut() = Some(reason.into());
        }

        fn release_core(&self) {
            let Some((request_id, executor_epoch, request)) = self.parked_core.borrow_mut().take()
            else {
                panic!("no parked core request");
            };
            let reply = self.engine.borrow_mut().handle(*request);
            let effects = self.session.borrow_mut().complete(Completion::Core {
                request_id,
                executor_epoch,
                reply: Box::new(reply),
            });
            self.drive(effects.into());
        }

        fn release_save(&self) {
            let Some(save_attempt_id) = self.parked_save.borrow_mut().take() else {
                panic!("no parked save");
            };
            self.finish_save(save_attempt_id);
        }

        fn finish_save(&self, save_attempt_id: SaveAttemptId) {
            let result = match self.fail_next_save.borrow_mut().take() {
                Some(reason) => SaveResult::Aborted(reason),
                None => SaveResult::Committed,
            };
            let effects = self.session.borrow_mut().complete(Completion::Persist {
                save_attempt_id,
                result,
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
                    } => {
                        if let Some(reason) = self.fail_next_core.borrow_mut().take() {
                            effects.extend(self.session.borrow_mut().complete(
                                Completion::CoreFailed {
                                    request_id,
                                    executor_epoch,
                                    reason,
                                },
                            ));
                            continue;
                        }
                        if self.hold_next_core.get() {
                            self.hold_next_core.set(false);
                            *self.parked_core.borrow_mut() =
                                Some((request_id, executor_epoch, request));
                            continue;
                        }
                        let reply = self.engine.borrow_mut().handle(*request);
                        effects.extend(self.session.borrow_mut().complete(Completion::Core {
                            request_id,
                            executor_epoch,
                            reply: Box::new(reply),
                        }));
                    }
                    Effect::Persist {
                        save_attempt_id,
                        document,
                        ..
                    } => {
                        if let Some(reason) = self.fail_next_save.borrow_mut().take() {
                            effects.extend(self.session.borrow_mut().complete(
                                Completion::Persist {
                                    save_attempt_id,
                                    result: SaveResult::Aborted(reason),
                                },
                            ));
                            continue;
                        }
                        if self.hold_next_save.get() {
                            self.hold_next_save.set(false);
                            *self.parked_save.borrow_mut() = Some(save_attempt_id);
                            continue;
                        }
                        self.saves
                            .borrow_mut()
                            .insert(document.id.clone(), (*document).clone());
                        self.finish_save(save_attempt_id);
                    }
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

        fn read_model(&self) -> boardstudio_application::ReadModel {
            self.session.borrow().read_model().clone()
        }

        fn saved_document(&self, id: &str) -> Option<ProjectDoc> {
            self.saves.borrow().get(id).cloned()
        }

        fn open_fixture(&self, id: &str, name: &str) {
            self.submit(Event::Open {
                operation_id: self.allocate_operation(),
                document: fixture_document(id, name),
            });
        }
    }

    impl EditTicketPort for NativeEditDriver {
        fn allocate_operation(&self) -> OperationId {
            let operation = OperationId(self.next_operation.get());
            self.next_operation.set(self.next_operation.get() + 1);
            operation
        }

        fn observe(&self, operation: OperationId) -> (OutcomeSlot, LandingSlot) {
            self.outcomes.observe_with_landing(operation)
        }

        fn submit(&self, event: Event) {
            let effects = self.session.borrow_mut().submit(event);
            self.drive(effects.into());
        }
    }

    fn fixture_document(id: &str, name: &str) -> ProjectDoc {
        let mut document = ProjectDoc::empty(id, name);
        document.definitions.push(PartDefinition {
            mechanical_profile: None,
            id: "key".into(),
            name: "Key".into(),
            kind: PartKind::Switch,
            courtyard: vec![],
            pads: vec![],
            models: None,
            hardware_profile: None,
            input_profile: None,
            keycap: Some(Vec2 { x: 18.0, y: 18.0 }),
            envelope_source: None,
            kicad_source: None,
            terminals: Default::default(),
            matrix_terminals: None,
            envelope_notice: None,
            generator: None,
        });
        document.parts.push(Part {
            id: "key".into(),
            definition_id: "key".into(),
            reference: "SW1".into(),
            pose: Pose2 {
                at: Vec2 { x: 0.0, y: 0.0 },
                rotation: 0.0,
            },
            side: Side::Front,
            locked: None,
            keycap: None,
            outline: None,
            properties: None,
            generator_parameters: None,
        });
        document
    }

    fn rename_resolver(name: &'static str) -> EditResolver {
        EditResolver::new(
            "layout-inspector",
            move |accepted: &boardstudio_application::AcceptedSnapshot| {
                let mut document = (*accepted.document).clone();
                document.name = name.into();
                Resolution::Submit(boardstudio_core::model::EditCommand {
                    base_revision: accepted.document.revision,
                    transaction_id: String::new(),
                    phase: boardstudio_core::model::EditPhase::Commit,
                    target_ids: vec![document.id.clone()],
                    operation: boardstudio_core::model::EditOperation::ReplaceDocument {
                        document: Box::new(document),
                    },
                })
            },
        )
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn native_runtime_owner_observes_the_real_sessions_landing() {
        let driver = NativeEditDriver::new();
        driver.open_fixture("ticket-test", "Ticket test");
        let runtime = crate::runtime::Runtime::new(
            driver.read_model(),
            driver.session.borrow().scope().unwrap(),
        );
        runtime.operation(); // The driver's Open used operation 1.
        let ticket = EditTicket::begin(
            &runtime,
            "native-owner",
            Some("field".into()),
            rename_resolver("Renamed"),
        );
        assert_eq!(ticket.settlement(true), Settlement::Pending);
        let (outcome, landing) = driver.observe(ticket.operation());
        driver.submit(runtime.events.borrow_mut().remove(0));
        runtime.outcomes.settle_with_landing(
            ticket.operation(),
            outcome.borrow().clone().unwrap(),
            *landing.borrow(),
        );
        assert_eq!(ticket.settlement(true), Settlement::Landed { revision: 1 });
        assert_eq!(
            driver.read_model().accepted.unwrap().document.name,
            "Renamed"
        );
    }

    #[test]
    fn an_edit_lands_at_the_revision_it_produced_and_saves_into_memory() {
        let driver = NativeEditDriver::new();
        driver.open_fixture("ticket-test", "Ticket test");
        let ticket = EditTicket::begin(
            &driver,
            "layout-inspector",
            Some("layout".into()),
            rename_resolver("Renamed"),
        );
        assert_eq!(ticket.settlement(true), Settlement::Landed { revision: 1 });
        assert!(!ticket.is_pending());
        assert_eq!(
            driver
                .saved_document("ticket-test")
                .expect("saved in memory")
                .name,
            "Renamed"
        );
    }

    #[test]
    fn a_held_core_reply_keeps_the_ticket_pending_until_released() {
        let driver = NativeEditDriver::new();
        driver.open_fixture("ticket-test", "Ticket test");
        driver.hold_next_core();
        let ticket = EditTicket::begin(
            &driver,
            "layout-inspector",
            Some("layout".into()),
            rename_resolver("Renamed"),
        );
        assert_eq!(ticket.settlement(true), Settlement::Pending);
        assert!(ticket.is_pending());
        driver.release_core();
        assert_eq!(ticket.settlement(true), Settlement::Landed { revision: 1 });
    }

    #[test]
    fn a_held_save_keeps_the_ticket_pending_until_released() {
        let driver = NativeEditDriver::new();
        driver.open_fixture("ticket-test", "Ticket test");
        driver.hold_next_save();
        let ticket = EditTicket::begin(
            &driver,
            "layout-inspector",
            Some("layout".into()),
            rename_resolver("Renamed"),
        );
        assert_eq!(ticket.settlement(true), Settlement::Pending);
        driver.release_save();
        assert_eq!(ticket.settlement(true), Settlement::Landed { revision: 1 });
    }

    #[test]
    fn an_unchanged_resolution_lands_at_the_current_revision_without_a_core_request() {
        let driver = NativeEditDriver::new();
        driver.open_fixture("ticket-test", "Ticket test");
        let before = driver.read_model().accepted.clone().unwrap();
        let ticket = EditTicket::begin(
            &driver,
            "layout-inspector",
            Some("layout".into()),
            EditResolver::new("layout-inspector", |_accepted| Resolution::Unchanged),
        );
        assert_eq!(
            ticket.settlement(true),
            Settlement::Landed {
                revision: before.document.revision
            }
        );
        let after = driver.read_model().accepted.clone().unwrap();
        assert_eq!(after.document.revision, before.document.revision);
        assert_eq!(after.token, before.token);
    }

    #[test]
    fn a_retired_target_fails_with_the_resolvers_reason() {
        let driver = NativeEditDriver::new();
        driver.open_fixture("ticket-test", "Ticket test");
        let ticket = EditTicket::begin(
            &driver,
            "layout-inspector",
            Some("layout".into()),
            EditResolver::new("layout-inspector", |_accepted| {
                Resolution::Retire("the selected part was deleted".into())
            }),
        );
        assert_eq!(
            ticket.settlement(true),
            Settlement::Failed {
                message: "The layout change was refused: the selected part was deleted".into()
            }
        );
    }

    #[test]
    fn a_departed_owner_retires_whatever_the_outcome() {
        let driver = NativeEditDriver::new();
        driver.open_fixture("ticket-test", "Ticket test");
        driver.hold_next_core();
        let ticket = EditTicket::begin(
            &driver,
            "layout-inspector",
            Some("layout".into()),
            rename_resolver("Renamed"),
        );
        assert_eq!(ticket.settlement(false), Settlement::Retired);
        driver.release_core();
        assert_eq!(
            ticket.settlement(false),
            Settlement::Retired,
            "a settled outcome is still retired for a departed owner"
        );
    }

    #[test]
    fn a_session_reopen_retires_the_queued_ticket() {
        let driver = NativeEditDriver::new();
        driver.open_fixture("ticket-test", "Ticket test");
        driver.hold_next_core();
        let first = EditTicket::begin(
            &driver,
            "layout-inspector",
            Some("layout".into()),
            rename_resolver("Renamed"),
        );
        driver.submit(Event::Open {
            operation_id: driver.allocate_operation(),
            document: fixture_document("ticket-test-2", "Second"),
        });
        let second = EditTicket::begin(
            &driver,
            "layout-inspector",
            Some("layout".into()),
            rename_resolver("Renamed again"),
        );
        driver.release_core();
        assert_eq!(first.settlement(true), Settlement::Landed { revision: 1 });
        assert_eq!(
            second.settlement(true),
            Settlement::Retired,
            "the reopen moved the session on before the queued edit ran"
        );
    }

    #[test]
    fn a_save_failure_fails_the_ticket_with_the_recovery_wording() {
        let driver = NativeEditDriver::new();
        driver.open_fixture("ticket-test", "Ticket test");
        driver.fail_next_save("quota exceeded");
        let ticket = EditTicket::begin(
            &driver,
            "layout-inspector",
            Some("layout".into()),
            rename_resolver("Renamed"),
        );
        assert_eq!(
            ticket.settlement(true),
            Settlement::Failed {
                message: "The layout completed, but the accepted document did not save: quota exceeded. Retry after recovery.".into()
            }
        );
    }

    #[test]
    fn a_refusal_during_recovery_fails_with_the_recovery_reason() {
        let driver = NativeEditDriver::new();
        driver.open_fixture("ticket-test", "Ticket test");
        driver.fail_next_save("quota exceeded");
        let failed = EditTicket::begin(
            &driver,
            "layout-inspector",
            Some("layout".into()),
            rename_resolver("Renamed"),
        );
        assert!(matches!(failed.settlement(true), Settlement::Failed { .. }));

        let refused = EditTicket::begin(
            &driver,
            "layout-inspector",
            Some("layout".into()),
            rename_resolver("Renamed again"),
        );
        assert_eq!(
            refused.settlement(true),
            Settlement::Failed {
                message: "The layout change was refused while saving is in recovery: save recovery is required".into()
            }
        );
    }

    #[test]
    fn an_executor_failure_fails_the_ticket() {
        let driver = NativeEditDriver::new();
        driver.open_fixture("ticket-test", "Ticket test");
        driver.fail_next_core("engine died");
        let ticket = EditTicket::begin(
            &driver,
            "layout-inspector",
            Some("layout".into()),
            rename_resolver("Renamed"),
        );
        assert_eq!(
            ticket.settlement(true),
            Settlement::Failed {
                message: "The layout change could not be applied: engine died".into()
            }
        );
    }

    #[test]
    fn every_outcome_maps_to_one_settlement() {
        let landing = Some(Landing {
            revision: 4,
            token: boardstudio_application::SnapshotToken(9),
        });
        let table: Vec<(TerminalOutcome, Option<Landing>, Settlement)> = vec![
            (
                TerminalOutcome::Completed,
                landing,
                Settlement::Landed { revision: 4 },
            ),
            (
                TerminalOutcome::Rejected(DOCUMENT_SESSION_CHANGED.into()),
                None,
                Settlement::Retired,
            ),
            (
                TerminalOutcome::Rejected("stale".into()),
                None,
                Settlement::Failed {
                    message: "The edit change was refused: stale".into(),
                },
            ),
            (
                TerminalOutcome::BlockedByRecovery("recovery".into()),
                None,
                Settlement::Failed {
                    message: "The edit change was refused while saving is in recovery: recovery"
                        .into(),
                },
            ),
            (
                TerminalOutcome::PersistenceFailed("disk".into()),
                None,
                Settlement::Failed {
                    message: "The edit completed, but the accepted document did not save: disk. Retry after recovery.".into(),
                },
            ),
            (
                TerminalOutcome::ExecutorFailed("engine".into()),
                None,
                Settlement::Failed {
                    message: "The edit change could not be applied: engine".into(),
                },
            ),
            (TerminalOutcome::Superseded, None, Settlement::Retired),
            (TerminalOutcome::Cancelled, None, Settlement::Retired),
            (TerminalOutcome::Closed, None, Settlement::Retired),
            (
                TerminalOutcome::Completed,
                None,
                Settlement::Failed {
                    message: "The edit change could not be confirmed where it landed. The accepted value was restored.".into(),
                },
            ),
        ];
        for (outcome, landing, expected) in table {
            assert_eq!(settlement_of(outcome, landing, None), expected);
        }
    }
}
