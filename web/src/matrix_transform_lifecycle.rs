//! Private transform admission and outcome-retention policy, shared by the mounted controller
//! and native regressions. Draft merging remains field-local in the inspector.
use boardstudio_application::{SnapshotToken, TerminalOutcome};

#[derive(Clone, Copy)]
pub(crate) struct AcceptedIdentity {
    pub(crate) token: SnapshotToken,
    pub(crate) revision: u64,
}

impl AcceptedIdentity {
    pub(crate) fn admits(self, captured: Self) -> bool {
        self.token == captured.token && self.revision == captured.revision
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum PendingSettlement {
    Wait,
    Suppress,
    Settle(TerminalOutcome),
}

pub(crate) fn pending_settlement(
    outcome: Option<TerminalOutcome>,
    target_is_current: bool,
) -> PendingSettlement {
    // Visibility controls feedback, never the lifetime of an already admitted request.
    let Some(outcome) = outcome else {
        return PendingSettlement::Wait;
    };
    if !target_is_current {
        return PendingSettlement::Suppress;
    }
    PendingSettlement::Settle(outcome)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::operation_outcomes::OperationOutcomes;
    use boardstudio_application::OperationId;

    #[test]
    fn another_field_revision_rejects_a_captured_transform_action() {
        let captured = AcceptedIdentity {
            token: SnapshotToken(11),
            revision: 7,
        };
        // The edited field's baseline can be unchanged after an unrelated field commits.
        // Its old event must still be rejected; a newly rendered action may preserve the draft.
        assert!(
            !AcceptedIdentity {
                token: SnapshotToken(12),
                revision: 8
            }
            .admits(captured)
        );
        assert!(
            AcceptedIdentity {
                token: SnapshotToken(12),
                revision: 8
            }
            .admits(AcceptedIdentity {
                token: SnapshotToken(12),
                revision: 8
            })
        );
    }

    #[test]
    fn replacement_token_and_revision_are_independently_checked() {
        let captured = AcceptedIdentity {
            token: SnapshotToken(11),
            revision: 7,
        };
        assert!(
            !AcceptedIdentity {
                token: SnapshotToken(12),
                revision: 7
            }
            .admits(captured)
        );
        assert!(
            !AcceptedIdentity {
                token: SnapshotToken(11),
                revision: 8
            }
            .admits(captured)
        );
    }

    #[test]
    fn hidden_pending_keeps_its_exact_observer_until_terminal_settlement() {
        let outcomes = OperationOutcomes::default();
        let operation = OperationId(31);
        let mut pending = Some(outcomes.observe(operation));
        let observed = pending.as_ref().unwrap().borrow().clone();
        if pending_settlement(observed, false) != PendingSettlement::Wait {
            pending = None;
        }
        assert!(
            pending.is_some(),
            "hiding the captured target must retain its pending observer"
        );
        assert!(outcomes.settle(operation, TerminalOutcome::Completed));
        assert_eq!(
            pending_settlement(pending.as_ref().unwrap().borrow().clone(), false),
            PendingSettlement::Suppress
        );
    }

    #[test]
    fn terminal_failure_after_scope_loss_retires_without_retargeting_feedback() {
        let outcomes = OperationOutcomes::default();
        let old_operation = OperationId(31);
        let new_operation = OperationId(32);
        let old = outcomes.observe(old_operation);
        let new = outcomes.observe(new_operation);
        assert!(outcomes.settle(
            old_operation,
            TerminalOutcome::PersistenceFailed("disk".into())
        ));
        assert_eq!(
            pending_settlement(old.borrow().clone(), false),
            PendingSettlement::Suppress
        );
        assert_eq!(
            pending_settlement(new.borrow().clone(), true),
            PendingSettlement::Wait
        );
        assert!(new.borrow().is_none());
    }

    #[test]
    fn current_target_settles_only_its_observed_outcome() {
        assert_eq!(pending_settlement(None, true), PendingSettlement::Wait);
        assert_eq!(
            pending_settlement(Some(TerminalOutcome::Completed), true),
            PendingSettlement::Settle(TerminalOutcome::Completed)
        );
        let failure = TerminalOutcome::Rejected("invalid transform".into());
        assert_eq!(
            pending_settlement(Some(failure.clone()), true),
            PendingSettlement::Settle(failure)
        );
    }
}
