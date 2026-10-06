//! Private, opt-in observation of Session terminal effects for page drafts.
use boardstudio_application::{OperationId, TerminalOutcome};
use std::{
    cell::RefCell,
    collections::BTreeMap,
    rc::{Rc, Weak},
};

pub type OutcomeSlot = Rc<RefCell<Option<TerminalOutcome>>>;

#[derive(Default)]
pub struct OperationOutcomes {
    waiting: RefCell<BTreeMap<OperationId, Weak<RefCell<Option<TerminalOutcome>>>>>,
}

impl OperationOutcomes {
    /// Observe a fresh operation before submitting it. Dropping the slot ends
    /// observation without cancelling the authoritative Session operation.
    pub fn observe(&self, operation: OperationId) -> OutcomeSlot {
        let mut waiting = self.waiting.borrow_mut();
        waiting.retain(|_, slot| slot.strong_count() != 0);
        if let Some(slot) = waiting.get(&operation).and_then(Weak::upgrade) {
            return slot;
        }
        let slot = Rc::new(RefCell::new(None));
        waiting.insert(operation, Rc::downgrade(&slot));
        slot
    }

    pub fn settle(&self, operation: OperationId, outcome: TerminalOutcome) -> bool {
        let slot = self
            .waiting
            .borrow_mut()
            .remove(&operation)
            .and_then(|slot| slot.upgrade());
        if let Some(slot) = slot {
            *slot.borrow_mut() = Some(outcome);
            true
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use boardstudio_application::{Effect, Event, SelectionMode, Session};

    #[test]
    fn identical_session_rejections_settle_their_own_operations() {
        let outcomes = OperationOutcomes::default();
        let mut session = Session::new();
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
}
