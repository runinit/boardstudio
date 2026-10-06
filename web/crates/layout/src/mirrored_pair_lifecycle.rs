//! Native-testable state transitions shared by the mounted mirrored-pair owner.
use boardstudio_application::{Durability, SnapshotToken};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PairFormStage {
    Setup,
    Preparing,
    Placement,
    Pending,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct PairFormState<T> {
    pub(crate) stage: PairFormStage,
    pub(crate) values: T,
}

impl<T> PairFormState<T> {
    pub(crate) fn new(values: T) -> Self {
        Self {
            stage: PairFormStage::Setup,
            values,
        }
    }

    pub(crate) fn setup_is_visible(&self) -> bool {
        self.stage != PairFormStage::Placement
    }

    pub(crate) fn setup_is_editable(&self) -> bool {
        self.stage == PairFormStage::Setup
    }
}

pub(crate) fn pair_cancel_is_allowed(active_owner: bool, save_pending: bool) -> bool {
    active_owner && !save_pending
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PairResultGuard {
    pub(crate) transaction_id: String,
    pub(crate) base_token: SnapshotToken,
    pub(crate) base_revision: u64,
}

pub(crate) fn accepted_saved_result_is_current(
    pending: &PairResultGuard,
    accepted_transaction_id: &str,
    accepted_token: SnapshotToken,
    accepted_revision: u64,
    ready: bool,
    durability: &Durability,
) -> bool {
    pending.transaction_id == accepted_transaction_id
        && accepted_token != pending.base_token
        && accepted_revision > pending.base_revision
        && ready
        && *durability
            == (Durability::Saved {
                revision: accepted_revision,
            })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escape_can_cancel_active_placement_but_not_a_pending_save() {
        let mut state = PairFormState::new("submitted draft");
        state.stage = PairFormStage::Placement;
        assert!(pair_cancel_is_allowed(true, false));
        assert!(!pair_cancel_is_allowed(true, true));
        assert!(!pair_cancel_is_allowed(false, false));
        assert_eq!(state.stage, PairFormStage::Placement);
    }

    #[test]
    fn setup_is_not_visible_as_an_editable_form_while_placement_owns_the_canvas() {
        let mut state = PairFormState::new("retained draft");
        assert!(state.setup_is_visible());
        assert!(state.setup_is_editable());
        state.stage = PairFormStage::Placement;
        assert!(!state.setup_is_visible());
        assert!(!state.setup_is_editable());
    }

    #[test]
    fn pair_settlement_requires_the_exact_accepted_operation_transaction() {
        let pending = PairResultGuard {
            transaction_id: "mirrored-pair-7-2-41".into(),
            base_token: SnapshotToken(4),
            base_revision: 11,
        };
        let saved = Durability::Saved { revision: 12 };
        assert!(accepted_saved_result_is_current(
            &pending,
            "mirrored-pair-7-2-41",
            SnapshotToken(5),
            12,
            true,
            &saved,
        ));
        assert!(!accepted_saved_result_is_current(
            &pending,
            "later-unrelated-edit",
            SnapshotToken(6),
            13,
            true,
            &Durability::Saved { revision: 13 },
        ));
        assert!(!accepted_saved_result_is_current(
            &pending,
            "mirrored-pair-7-2-41",
            SnapshotToken(4),
            12,
            true,
            &Durability::Saved { revision: 12 },
        ));
        assert!(!accepted_saved_result_is_current(
            &pending,
            "mirrored-pair-7-2-41",
            SnapshotToken(5),
            11,
            true,
            &Durability::Saved { revision: 11 },
        ));
        assert!(!accepted_saved_result_is_current(
            &pending,
            "mirrored-pair-7-2-41",
            SnapshotToken(5),
            12,
            false,
            &Durability::Saved { revision: 12 },
        ));
        assert!(!accepted_saved_result_is_current(
            &pending,
            "mirrored-pair-7-2-41",
            SnapshotToken(5),
            12,
            true,
            &Durability::Saving { revision: 12 },
        ));
    }
}
