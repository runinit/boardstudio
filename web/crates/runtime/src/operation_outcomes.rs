//! Private, opt-in observation of Session terminal effects for page drafts.
use boardstudio_application::{DOCUMENT_SESSION_CHANGED, Landing, OperationId, TerminalOutcome};
use std::{
    cell::{Cell, RefCell},
    collections::BTreeMap,
    rc::{Rc, Weak},
};

pub type OutcomeSlot = Rc<RefCell<Option<TerminalOutcome>>>;
/// Presentation of a resolved edit's Session outcome.
#[derive(Clone, Debug, PartialEq)]
pub enum Settlement {
    Pending,
    Landed { revision: u64 },
    Failed { message: String },
    Retired,
}

/// The result receiver for an observed edit. Runtime owns the scope lifetime and
/// Session owns the operation, including any save that outlives this receiver.
#[derive(Clone)]
pub struct EditOutcome {
    operation: OperationId,
    observation: Rc<EditObservation>,
}

struct EditObservation {
    scope: Weak<()>,
    feature: Option<String>,
    result: RefCell<Settlement>,
    retired: Cell<bool>,
}

impl EditOutcome {
    pub fn operation(&self) -> OperationId {
        self.operation
    }

    pub fn is_pending(&self) -> bool {
        self.observation.is_live() && *self.observation.result.borrow() == Settlement::Pending
    }

    pub fn settlement(&self, owner_is_live: bool) -> Settlement {
        let observation = &self.observation;
        if !owner_is_live {
            observation.retired.set(true);
        }
        if !observation.is_live() {
            return Settlement::Retired;
        }
        observation.result.borrow().clone()
    }
}

impl EditObservation {
    fn is_live(&self) -> bool {
        !self.retired.get() && self.scope.strong_count() != 0
    }
}

#[derive(Default)]
struct ObservedSlots {
    outcome: Weak<RefCell<Option<TerminalOutcome>>>,
    edit: Weak<EditObservation>,
}

#[derive(Default)]
pub struct OperationOutcomes {
    waiting: RefCell<BTreeMap<OperationId, ObservedSlots>>,
    scope: RefCell<Rc<()>>,
}

impl OperationOutcomes {
    pub(crate) fn retire_edit_scope(&self) {
        *self.scope.borrow_mut() = Rc::new(());
    }

    fn register<T>(
        &self,
        operation: OperationId,
        register: impl FnOnce(&mut ObservedSlots) -> T,
    ) -> T {
        let mut waiting = self.waiting.borrow_mut();
        waiting
            .retain(|_, slots| slots.outcome.strong_count() != 0 || slots.edit.strong_count() != 0);
        register(waiting.entry(operation).or_default())
    }

    /// Observe a fresh operation before submitting it. Dropping the slot ends
    /// observation without cancelling the authoritative Session operation.
    pub fn observe(&self, operation: OperationId) -> OutcomeSlot {
        self.register(operation, |slots| {
            slots.outcome.upgrade().unwrap_or_else(|| {
                let slot = Rc::new(RefCell::new(None));
                slots.outcome = Rc::downgrade(&slot);
                slot
            })
        })
    }

    /// Register the receiver before submitting, including for synchronous completion.
    pub fn observe_edit(&self, operation: OperationId, feature: Option<String>) -> EditOutcome {
        let observation = Rc::new(EditObservation {
            scope: Rc::downgrade(&self.scope.borrow()),
            feature,
            result: RefCell::new(Settlement::Pending),
            retired: Cell::new(false),
        });
        self.register(operation, |slots| slots.edit = Rc::downgrade(&observation));
        EditOutcome {
            operation,
            observation,
        }
    }

    pub fn settle(&self, operation: OperationId, outcome: TerminalOutcome) -> bool {
        self.settle_with_landing(operation, outcome, None)
    }

    pub fn settle_with_landing(
        &self,
        operation: OperationId,
        outcome: TerminalOutcome,
        landing: Option<Landing>,
    ) -> bool {
        let slots = self.waiting.borrow_mut().remove(&operation);
        let Some(slots) = slots else { return false };
        let plain = slots.outcome.upgrade();
        let edit = slots.edit.upgrade();
        if let Some(edit) = &edit {
            *edit.result.borrow_mut() = settlement_of(&outcome, landing, edit.feature.as_deref());
        }
        if let Some(slot) = &plain {
            *slot.borrow_mut() = Some(outcome);
        }
        plain.is_some() || edit.is_some()
    }
}

#[cfg(any(target_arch = "wasm32", test, feature = "test-support"))]
impl crate::runtime::Runtime {
    pub fn resolve_edit(
        self: &Rc<Self>,
        label: &str,
        feature: Option<String>,
        resolver: boardstudio_application::EditResolver,
    ) -> EditOutcome {
        let operation_id = self.operation();
        let outcome = self.operation_outcomes.observe_edit(operation_id, feature);
        self.submit(boardstudio_application::Event::ResolveEdit {
            operation_id,
            label: label.to_owned(),
            resolver,
        });
        outcome
    }
}

/// The one mapping from a terminal outcome and its landing to a settlement.
fn settlement_of(
    outcome: &TerminalOutcome,
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

#[cfg(test)]
mod tests {
    use super::*;
    use boardstudio_application::{Effect, Event, Landing, SelectionMode, Session, SnapshotToken};

    #[test]
    fn identical_session_rejections_settle_their_own_operations() {
        let outcomes = OperationOutcomes::default();
        let mut session = Session::new(boardstudio_core::CoreEngine::validate_open);
        let first = outcomes.observe(OperationId(1));
        let second = outcomes.observe(OperationId(2));
        for operation_id in [OperationId(1), OperationId(2)] {
            for effect in session.submit(Event::SelectParts {
                operation_id,
                mode: SelectionMode::Replace,
                part_ids: Vec::new(),
                range_part_ids: Vec::new(),
            }) {
                if let Effect::Settled {
                    operation_id,
                    outcome,
                    ..
                } = effect
                {
                    outcomes.settle(operation_id, outcome);
                }
            }
        }
        assert!(matches!(
            *first.borrow(),
            Some(TerminalOutcome::Rejected(_))
        ));
        assert_eq!(*first.borrow(), *second.borrow());
        assert!(outcomes.waiting.borrow().is_empty());
    }

    #[test]
    fn dropped_view_and_unrelated_completion_do_not_acknowledge_current_view() {
        let outcomes = OperationOutcomes::default();
        let abandoned = outcomes.observe(OperationId(1));
        drop(abandoned);
        let current = outcomes.observe(OperationId(2));
        assert!(!outcomes.settle(OperationId(1), TerminalOutcome::Completed));
        assert!(!outcomes.settle(
            OperationId(3),
            TerminalOutcome::Rejected("other job".into())
        ));
        assert!(current.borrow().is_none());
        assert!(outcomes.settle(
            OperationId(2),
            TerminalOutcome::PersistenceFailed("disk".into())
        ));
        assert_eq!(
            *current.borrow(),
            Some(TerminalOutcome::PersistenceFailed("disk".into()))
        );
    }

    #[test]
    fn typed_and_edit_observers_receive_the_same_completion() {
        let outcomes = OperationOutcomes::default();
        let operation = OperationId(1);
        let edit = outcomes.observe_edit(operation, None);
        let plain = outcomes.observe(operation);
        assert!(outcomes.settle_with_landing(
            operation,
            TerminalOutcome::Completed,
            Some(Landing {
                revision: 3,
                token: SnapshotToken(7),
            }),
        ));
        assert_eq!(*plain.borrow(), Some(TerminalOutcome::Completed));
        assert_eq!(edit.settlement(true), Settlement::Landed { revision: 3 });
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
#[path = "edit_outcome_tests.rs"]
mod edit_outcome_tests;
