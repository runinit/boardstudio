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

/// The submit/observe port a ticket drives. Its read-only scope source retains Runtime
/// access for the observation lifetime. Scope contains session epoch, document, board and
/// instance identity; revisions and tokens do not affect ticket liveness. A mismatch
/// retires observation permanently without cancelling the Session operation.
pub trait EditTicketPort {
    fn allocate_operation(&self) -> OperationId;
    /// Return a reader that remains valid after `begin` returns.
    fn scope_source(&self) -> std::rc::Rc<dyn Fn() -> Option<boardstudio_application::Scope>>;
    fn observe(&self, operation: OperationId) -> (OutcomeSlot, LandingSlot);
    fn submit(&self, event: Event);
}

pub struct EditTicket {
    operation: OperationId,
    outcome: OutcomeSlot,
    landing: LandingSlot,
    feature: Option<String>,
    captured_scope: Option<boardstudio_application::Scope>,
    current_scope: std::rc::Rc<dyn Fn() -> Option<boardstudio_application::Scope>>,
    retired: std::rc::Rc<std::cell::Cell<bool>>,
}

impl Clone for EditTicket {
    fn clone(&self) -> Self {
        // The observation slots are shared: a clone observes the same settlement.
        Self {
            operation: self.operation,
            outcome: self.outcome.clone(),
            landing: self.landing.clone(),
            feature: self.feature.clone(),
            captured_scope: self.captured_scope.clone(),
            current_scope: self.current_scope.clone(),
            retired: self.retired.clone(),
        }
    }
}

impl EditTicket {
    /// Begin a pending edit for `resolver`. Scope is captured before submission and the
    /// outcome and landing are observed before the event is submitted, so a synchronous
    /// settlement cannot be missed. Clones share the latched retirement state. `feature`
    /// is the noun used in failure wording (for example `"layout"`).
    pub fn begin(
        port: &dyn EditTicketPort,
        label: &str,
        feature: Option<String>,
        resolver: EditResolver,
    ) -> Self {
        let current_scope = port.scope_source();
        let captured_scope = current_scope();
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
            captured_scope,
            current_scope,
            retired: std::rc::Rc::new(std::cell::Cell::new(false)),
        }
    }

    /// The operation the edit runs under.
    pub fn operation(&self) -> OperationId {
        self.operation
    }

    /// Whether the edit has not settled yet. One-shot actions disable their control while
    /// this answers true.
    pub fn is_pending(&self) -> bool {
        self.is_live() && self.outcome.borrow().is_none()
    }

    /// Read the settlement. `owner_is_live` answers only whether the panel's selection or
    /// mounted target still owns this observation; Scope lineage is checked by the ticket.
    /// A departed owner permanently retires the ticket whatever the outcome.
    pub fn settlement(&self, owner_is_live: bool) -> Settlement {
        if !owner_is_live {
            self.retired.set(true);
            return Settlement::Retired;
        }
        if !self.is_live() {
            return Settlement::Retired;
        }
        let Some(outcome) = self.outcome.borrow().clone() else {
            return Settlement::Pending;
        };
        let landing = *self.landing.borrow();
        settlement_of(outcome, landing, self.feature.as_deref())
    }

    fn is_live(&self) -> bool {
        if self.retired.get() {
            return false;
        }
        if (self.current_scope)() != self.captured_scope {
            self.retired.set(true);
            return false;
        }
        true
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
    #[cfg(any(target_arch = "wasm32", test, feature = "test-support"))]
    use crate::runtime::Runtime;

    #[cfg(any(target_arch = "wasm32", test, feature = "test-support"))]
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

        fn scope_source(&self) -> std::rc::Rc<dyn Fn() -> Option<boardstudio_application::Scope>> {
            let runtime = self.clone();
            std::rc::Rc::new(move || Runtime::scope(&runtime))
        }

        fn submit(&self, event: Event) {
            Runtime::submit(self, event)
        }
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;
    use boardstudio_application::{EditResolver, Event, Landing, Resolution, SnapshotToken};
    use boardstudio_core::model::{
        Board, EditOperation, EditPhase, Part, PartDefinition, PartKind, Pose2, ProjectDoc, Side,
        Vec2,
    };
    use std::rc::Rc;

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

    fn runtime_with_project(id: &str, name: &str) -> Rc<crate::runtime::Runtime> {
        let runtime = crate::runtime::Runtime::new();
        runtime.submit(Event::Open {
            operation_id: runtime.operation(),
            document: fixture_document(id, name),
        });
        runtime
    }

    fn runtime_with_two_boards() -> Rc<crate::runtime::Runtime> {
        let mut document = fixture_document("two-board-project", "Two board project");
        document.boards = ["board-1", "board-2"]
            .into_iter()
            .map(|id| Board {
                id: id.into(),
                name: id.into(),
                outline_ids: vec![],
                part_ids: vec![],
                net_ids: vec![],
                thickness: 1.6,
                traces: vec![],
                vias: vec![],
            })
            .collect();
        let runtime = crate::runtime::Runtime::new();
        runtime.submit(Event::Open {
            operation_id: runtime.operation(),
            document,
        });
        runtime
    }

    fn rename_resolver(name: &'static str) -> EditResolver {
        EditResolver::new(
            "layout-inspector",
            move |accepted: &boardstudio_application::AcceptedSnapshot| {
                let mut document = (*accepted.document).clone();
                document.name = name.into();
                Resolution::submit(
                    vec![document.id.clone()],
                    EditOperation::ReplaceDocument {
                        document: Box::new(document),
                    },
                )
            },
        )
    }

    #[test]
    fn native_runtime_settles_a_ticket_through_the_real_session_and_core() {
        let runtime = runtime_with_project("ticket-test", "Ticket test");
        let ticket = EditTicket::begin(
            &runtime,
            "native-owner",
            Some("field".into()),
            rename_resolver("Renamed"),
        );
        assert_eq!(ticket.settlement(true), Settlement::Landed { revision: 1 });
        assert_eq!(runtime.model().accepted.unwrap().document.name, "Renamed");
        assert_eq!(
            runtime.saved_document("ticket-test").unwrap().name,
            "Renamed"
        );
    }

    #[test]
    fn held_core_and_save_keep_a_ticket_pending_until_released() {
        let runtime = runtime_with_project("ticket-test", "Ticket test");
        runtime.hold_next_core();
        let core_ticket = EditTicket::begin(
            &runtime,
            "layout-inspector",
            Some("layout".into()),
            rename_resolver("Core held"),
        );
        assert!(runtime.core_entered());
        assert_eq!(core_ticket.settlement(true), Settlement::Pending);
        runtime.release_core();
        assert_eq!(
            core_ticket.settlement(true),
            Settlement::Landed { revision: 1 }
        );

        runtime.hold_next_save();
        let save_ticket = EditTicket::begin(
            &runtime,
            "layout-inspector",
            Some("layout".into()),
            rename_resolver("Save held"),
        );
        assert!(runtime.save_entered());
        assert_eq!(save_ticket.settlement(true), Settlement::Pending);
        runtime.release_save();
        assert_eq!(
            save_ticket.settlement(true),
            Settlement::Landed { revision: 2 }
        );
    }

    #[test]
    fn core_and_save_failures_settle_the_ticket_without_a_resolver() {
        let runtime = runtime_with_project("ticket-test", "Ticket test");
        runtime.fail_next_core("engine died");
        let core_failure = EditTicket::begin(
            &runtime,
            "layout-inspector",
            Some("layout".into()),
            rename_resolver("Core failure"),
        );
        assert_eq!(
            core_failure.settlement(true),
            Settlement::Failed {
                message: "The layout change could not be applied: engine died".into()
            }
        );

        let runtime = runtime_with_project("ticket-save-test", "Ticket save test");
        runtime.fail_next_save("quota exceeded");
        let save_failure = EditTicket::begin(
            &runtime,
            "layout-inspector",
            Some("layout".into()),
            rename_resolver("Save failure"),
        );
        assert_eq!(
            save_failure.settlement(true),
            Settlement::Failed {
                message: "The layout completed, but the accepted document did not save: quota exceeded. Retry after recovery.".into()
            }
        );
    }

    #[test]
    fn unchanged_and_retired_resolutions_use_session_settlement() {
        let runtime = runtime_with_project("ticket-test", "Ticket test");
        let before = runtime.model().accepted.unwrap();
        let unchanged = EditTicket::begin(
            &runtime,
            "layout-inspector",
            Some("layout".into()),
            EditResolver::new("layout-inspector", |_accepted| Resolution::Unchanged),
        );
        assert_eq!(
            unchanged.settlement(true),
            Settlement::Landed {
                revision: before.document.revision
            }
        );
        let retired = EditTicket::begin(
            &runtime,
            "layout-inspector",
            Some("layout".into()),
            EditResolver::new("layout-inspector", |_accepted| {
                Resolution::Retire("the selected part was deleted".into())
            }),
        );
        assert_eq!(
            retired.settlement(true),
            Settlement::Failed {
                message: "The layout change was refused: the selected part was deleted".into()
            }
        );
    }

    #[test]
    fn changing_the_session_retires_its_held_ticket() {
        let runtime = runtime_with_project("ticket-test", "Ticket test");
        runtime.hold_next_core();
        let ticket = EditTicket::begin(
            &runtime,
            "layout-inspector",
            Some("layout".into()),
            rename_resolver("Old project edit"),
        );
        runtime.submit(Event::Open {
            operation_id: runtime.operation(),
            document: fixture_document("ticket-test-2", "Second project"),
        });
        let queued = EditTicket::begin(
            &runtime,
            "layout-inspector",
            Some("layout".into()),
            rename_resolver("Queued edit"),
        );
        runtime.release_core();
        assert_eq!(ticket.settlement(true), Settlement::Retired);
        assert_eq!(queued.settlement(true), Settlement::Retired);
    }

    #[test]
    fn a_scope_departure_retires_every_ticket_clone_permanently() {
        let runtime = runtime_with_project("ticket-test", "Ticket test");
        runtime.hold_next_core();
        let ticket = EditTicket::begin(
            &runtime,
            "layout-inspector",
            Some("layout".into()),
            rename_resolver("Old project edit"),
        );
        let clone = ticket.clone();
        assert_eq!(ticket.settlement(true), Settlement::Pending);

        runtime.submit(Event::Open {
            operation_id: runtime.operation(),
            document: fixture_document("ticket-test-2", "Second project"),
        });
        assert_eq!(ticket.settlement(true), Settlement::Pending);
        runtime.release_core();
        assert_eq!(ticket.settlement(true), Settlement::Retired);
        assert_eq!(clone.settlement(true), Settlement::Retired);

        runtime.submit(Event::Open {
            operation_id: runtime.operation(),
            document: fixture_document("ticket-test", "Ticket test again"),
        });
        assert_eq!(ticket.settlement(true), Settlement::Retired);
        assert_eq!(clone.settlement(true), Settlement::Retired);
    }

    #[test]
    fn returning_to_the_captured_board_scope_does_not_revive_observation() {
        let runtime = runtime_with_two_boards();
        let captured_scope = runtime.scope().unwrap();
        runtime.hold_next_core();
        let ticket = EditTicket::begin(
            &runtime,
            "layout-inspector",
            Some("layout".into()),
            rename_resolver("Edit while navigating"),
        );
        let unobserved_clone = ticket.clone();

        runtime.submit(Event::Navigate {
            operation_id: runtime.operation(),
            board_id: "board-2".into(),
            instance_id: None,
        });
        assert_eq!(ticket.settlement(true), Settlement::Retired);

        runtime.submit(Event::Navigate {
            operation_id: runtime.operation(),
            board_id: "board-1".into(),
            instance_id: None,
        });
        assert_eq!(runtime.scope(), Some(captured_scope));
        assert_eq!(ticket.settlement(true), Settlement::Retired);
        assert_eq!(unobserved_clone.settlement(true), Settlement::Retired);
        runtime.release_core();
    }

    #[test]
    fn open_waits_for_a_held_save_then_retires_the_landed_ticket() {
        let runtime = runtime_with_project("ticket-test", "Ticket test");
        runtime.hold_next_save();
        let ticket = EditTicket::begin(
            &runtime,
            "layout-inspector",
            Some("layout".into()),
            rename_resolver("Saved before open"),
        );
        assert_eq!(ticket.settlement(true), Settlement::Pending);
        assert!(runtime.save_entered());

        runtime.submit(Event::Open {
            operation_id: runtime.operation(),
            document: fixture_document("ticket-test-2", "Second project"),
        });
        assert_eq!(ticket.settlement(true), Settlement::Pending);
        runtime.release_save();
        assert_eq!(ticket.settlement(true), Settlement::Retired);
    }

    #[test]
    fn same_scope_revision_changes_and_owner_departure_have_separate_lifetimes() {
        let runtime = runtime_with_project("ticket-test", "Ticket test");
        let ticket = EditTicket::begin(
            &runtime,
            "layout-inspector",
            Some("layout".into()),
            rename_resolver("First revision"),
        );
        assert_eq!(ticket.settlement(true), Settlement::Landed { revision: 1 });
        let newer = EditTicket::begin(
            &runtime,
            "layout-inspector",
            Some("layout".into()),
            rename_resolver("A later revision"),
        );
        assert_eq!(newer.settlement(true), Settlement::Landed { revision: 2 });
        assert_eq!(ticket.settlement(true), Settlement::Landed { revision: 1 });
        assert_eq!(ticket.settlement(false), Settlement::Retired);
        assert_eq!(ticket.settlement(true), Settlement::Retired);
    }

    #[test]
    fn every_outcome_maps_to_one_settlement() {
        let landing = Some(Landing {
            revision: 4,
            token: SnapshotToken(9),
        });
        let table: Vec<(TerminalOutcome, Option<Landing>, Settlement)> = vec![
            (TerminalOutcome::Completed, landing, Settlement::Landed { revision: 4 }),
            (TerminalOutcome::Rejected(DOCUMENT_SESSION_CHANGED.into()), None, Settlement::Retired),
            (TerminalOutcome::Rejected("stale".into()), None, Settlement::Failed { message: "The edit change was refused: stale".into() }),
            (TerminalOutcome::BlockedByRecovery("recovery".into()), None, Settlement::Failed { message: "The edit change was refused while saving is in recovery: recovery".into() }),
            (TerminalOutcome::PersistenceFailed("disk".into()), None, Settlement::Failed { message: "The edit completed, but the accepted document did not save: disk. Retry after recovery.".into() }),
            (TerminalOutcome::ExecutorFailed("engine".into()), None, Settlement::Failed { message: "The edit change could not be applied: engine".into() }),
            (TerminalOutcome::Superseded, None, Settlement::Retired),
            (TerminalOutcome::Cancelled, None, Settlement::Retired),
            (TerminalOutcome::Closed, None, Settlement::Retired),
            (TerminalOutcome::Completed, None, Settlement::Failed { message: "The edit change could not be confirmed where it landed. The accepted value was restored.".into() }),
        ];
        for (outcome, landing, expected) in table {
            assert_eq!(settlement_of(outcome, landing, None), expected);
        }
    }
}
