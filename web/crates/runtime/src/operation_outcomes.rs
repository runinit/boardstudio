//! Private, opt-in observation of Session terminal effects for page drafts.
use boardstudio_application::{Landing, OperationId, TerminalOutcome};
use std::{
    cell::RefCell,
    collections::BTreeMap,
    rc::{Rc, Weak},
};

pub type OutcomeSlot = Rc<RefCell<Option<TerminalOutcome>>>;
pub type LandingSlot = Rc<RefCell<Option<Landing>>>;

#[derive(Default)]
struct ObservedSlots {
    outcome: Weak<RefCell<Option<TerminalOutcome>>>,
    landing: Weak<RefCell<Option<Landing>>>,
}

impl ObservedSlots {
    fn outcome_slot(&mut self) -> OutcomeSlot {
        self.outcome.upgrade().unwrap_or_else(|| {
            let slot = Rc::new(RefCell::new(None));
            self.outcome = Rc::downgrade(&slot);
            slot
        })
    }

    fn landing_slot(&mut self) -> LandingSlot {
        self.landing.upgrade().unwrap_or_else(|| {
            let slot = Rc::new(RefCell::new(None));
            self.landing = Rc::downgrade(&slot);
            slot
        })
    }
}

#[derive(Default)]
pub struct OperationOutcomes {
    waiting: RefCell<BTreeMap<OperationId, ObservedSlots>>,
}

impl OperationOutcomes {
    /// Observe a fresh operation before submitting it. Dropping the slot ends
    /// observation without cancelling the authoritative Session operation.
    pub fn observe(&self, operation: OperationId) -> OutcomeSlot {
        self.observe_with_landing(operation).0
    }

    /// Observe a fresh operation and receive a second slot carrying where the operation
    /// landed. The outcome slot behaves exactly as `observe`'s, whichever way round the
    /// two observation calls happen.
    pub fn observe_with_landing(&self, operation: OperationId) -> (OutcomeSlot, LandingSlot) {
        let mut waiting = self.waiting.borrow_mut();
        waiting.retain(|_, slots| {
            slots.outcome.strong_count() != 0 || slots.landing.strong_count() != 0
        });
        let slots = waiting.entry(operation).or_default();
        (slots.outcome_slot(), slots.landing_slot())
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
        let Some(slot) = slots.outcome.upgrade() else {
            return false;
        };
        *slot.borrow_mut() = Some(outcome);
        if let Some(landing_slot) = slots.landing.upgrade() {
            *landing_slot.borrow_mut() = landing;
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use boardstudio_application::{Effect, Event, Landing, SelectionMode, Session, SnapshotToken};

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
    fn a_landing_observer_reads_the_outcome_and_where_the_operation_landed() {
        let outcomes = OperationOutcomes::default();
        let (slot, landing) = outcomes.observe_with_landing(OperationId(1));
        let plain = outcomes.observe(OperationId(1));
        assert!(
            Rc::ptr_eq(&slot, &plain),
            "the plain observer keeps its exact behaviour"
        );

        assert!(outcomes.settle_with_landing(
            OperationId(1),
            TerminalOutcome::Completed,
            Some(Landing {
                revision: 3,
                token: SnapshotToken(7),
            }),
        ));
        assert_eq!(*slot.borrow(), Some(TerminalOutcome::Completed));
        assert_eq!(
            *landing.borrow(),
            Some(Landing {
                revision: 3,
                token: SnapshotToken(7),
            })
        );

        let (_outcome_slot, landing) = outcomes.observe_with_landing(OperationId(2));
        assert!(outcomes.settle(
            OperationId(2),
            TerminalOutcome::Rejected("no landing for this one".into())
        ));
        assert_eq!(*landing.borrow(), None);
    }
}
