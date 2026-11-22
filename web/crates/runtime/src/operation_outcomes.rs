//! Private, opt-in observation of Session terminal effects for page drafts.
#[cfg(any(target_arch = "wasm32", test, feature = "test-support"))]
use boardstudio_application::{DOCUMENT_SESSION_CHANGED, Landing};
use boardstudio_application::{OperationId, TerminalOutcome};
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

/// The accepted result of a synchronous edit, valid for the view and scope that submitted it.
#[derive(Clone)]
pub struct EditOutcome {
    operation: OperationId,
    scope: Weak<()>,
    result: Settlement,
    retired: Rc<Cell<bool>>,
}

impl EditOutcome {
    pub fn operation(&self) -> OperationId {
        self.operation
    }

    pub fn is_pending(&self) -> bool {
        false
    }

    pub fn settlement(&self, owner_is_live: bool) -> Settlement {
        if !owner_is_live {
            self.retired.set(true);
        }
        if self.retired.get() || self.scope.strong_count() == 0 {
            Settlement::Retired
        } else {
            self.result.clone()
        }
    }
}

#[derive(Default)]
pub struct OperationOutcomes {
    waiting: RefCell<BTreeMap<OperationId, Weak<RefCell<Option<TerminalOutcome>>>>>,
    scope: RefCell<Rc<()>>,
}

impl OperationOutcomes {
    pub(crate) fn retire_edit_scope(&self) {
        *self.scope.borrow_mut() = Rc::new(());
    }

    pub fn observe(&self, operation: OperationId) -> OutcomeSlot {
        let mut waiting = self.waiting.borrow_mut();
        waiting.retain(|_, slot| slot.strong_count() != 0);
        let entry = waiting.entry(operation).or_default();
        entry.upgrade().unwrap_or_else(|| {
            let slot = Rc::new(RefCell::new(None));
            *entry = Rc::downgrade(&slot);
            slot
        })
    }

    pub fn settle(&self, operation: OperationId, outcome: TerminalOutcome) -> bool {
        let Some(slot) = self
            .waiting
            .borrow_mut()
            .remove(&operation)
            .and_then(|slot| slot.upgrade())
        else {
            return false;
        };
        *slot.borrow_mut() = Some(outcome);
        true
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
        use boardstudio_application::Event;
        let operation = self.operation();
        let scope = Rc::downgrade(&self.operation_outcomes.scope.borrow());
        let result = self.submit_observed(
            Event::ResolveEdit {
                operation_id: operation,
                label: label.to_owned(),
                resolver,
            },
            Some(operation),
        );
        let (outcome, landing) = result.unwrap_or_else(|| {
            (
                TerminalOutcome::ExecutorFailed("The edit did not return a result.".into()),
                None,
            )
        });
        EditOutcome {
            operation,
            scope,
            result: settlement_of(&outcome, landing, feature.as_deref()),
            retired: Rc::new(Cell::new(false)),
        }
    }
}

#[cfg(any(target_arch = "wasm32", test, feature = "test-support"))]
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
    fn synchronous_edit_result_retires_with_its_view_or_scope() {
        let outcomes = OperationOutcomes::default();
        let edit = EditOutcome {
            operation: OperationId(1),
            scope: Rc::downgrade(&outcomes.scope.borrow()),
            result: settlement_of(
                &TerminalOutcome::Completed,
                Some(Landing {
                    revision: 3,
                    token: SnapshotToken(7),
                }),
                None,
            ),
            retired: Rc::new(Cell::new(false)),
        };
        assert_eq!(edit.settlement(true), Settlement::Landed { revision: 3 });
        outcomes.retire_edit_scope();
        assert_eq!(edit.settlement(true), Settlement::Retired);
    }
}
