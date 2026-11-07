//! Ticket 019: the pure refresh-safety decision for adopting a waiting build.
//!
//! [`update_safety`] answers whether reloading to adopt a waiting build risks
//! committed work. The browser Runtime wires its live owners into
//! [`UpdateSafetyInputs`]; this module owns only the decision, so its unit tests
//! run natively under `cargo test` while mounted tests prove the wiring.
//!
//! Owners read at activation: Session engine work ([`Session::has_pending_work`],
//! queued intents and executing Core requests), the save leg
//! ([`Durability`], the Runtime's started-or-retained save buffer and the save
//! conflict latch), and storage readiness (initialization and its error).
//!
//! Draft values live in component-local signals (inspector, layout,
//! key-selection, PCB, keycaps and mechanical panels plus the setup
//! project-name draft) with no global registry. Two signals still make them
//! visible to the gate. A valid typed value converts to detectable engine work
//! when focus moves: activating Update Now blurs the focused field, which runs
//! the panels' standard blur-commit. A value its panel refuses to commit never
//! reaches the engine, but the shared field controls mark it `aria-invalid`;
//! the Runtime reads that live marker into [`UpdateSafetyInputs::invalid_draft`]
//! and the gate defers until the user fixes or clears the field.
//!
//! Remaining recorded gap: a component that holds an uncommitted draft without
//! committing it on blur and without marking the field invalid is not visible.
//! The warning offers cancellation only: an acknowledgment is never permission
//! to discard work.
//!
//! [`Session::has_pending_work`]: boardstudio_application::Session::has_pending_work
use boardstudio_application::{Durability, Lifecycle};

/// One reason adopting the waiting build must defer. Variants name the owning
/// condition so the warning can tell the user what to resolve.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UpdateBlocker {
    /// Committed edits are still moving through the engine.
    PendingWork,
    /// A save started but is unanswered, or the accepted document runs ahead of
    /// its last save.
    SaveInFlight,
    /// A save failed, conflicted, or is retained after failure. Never safe.
    SaveFailed,
    /// Storage is still initializing or failed to initialize, so saves cannot
    /// be verified.
    StorageNotReady,
    /// The session needs recovery before anything else happens.
    RecoveryRequired,
    /// A visible field holds a value its panel refuses to commit (marked
    /// `aria-invalid`). It lives only in the page, so reloading would discard it.
    InvalidDraft,
}

/// The live refresh-safety owners, read at activation time.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UpdateSafetyInputs {
    pub lifecycle: Lifecycle,
    pub durability: Durability,
    pub accepted_revision: Option<u64>,
    pub session_pending_work: bool,
    /// The Runtime's save buffer: set while a save runs, retained after failure.
    pub save_started_or_retained: bool,
    pub save_conflict: bool,
    pub storage_initializing: bool,
    pub storage_failed: bool,
    /// A visible field is marked invalid: an uncommittable Draft value that no
    /// engine owner can see.
    pub invalid_draft: bool,
}

impl UpdateSafetyInputs {
    /// A settled library: nothing open, nothing moving, storage ready.
    pub fn settled_empty() -> Self {
        Self {
            lifecycle: Lifecycle::Empty,
            durability: Durability::NoDocument,
            accepted_revision: None,
            session_pending_work: false,
            save_started_or_retained: false,
            save_conflict: false,
            storage_initializing: false,
            storage_failed: false,
            invalid_draft: false,
        }
    }

    /// A settled open project: accepted revision `revision` is saved.
    pub fn settled_saved(revision: u64) -> Self {
        Self {
            lifecycle: Lifecycle::Ready,
            durability: Durability::Saved { revision },
            accepted_revision: Some(revision),
            ..Self::settled_empty()
        }
    }
}

/// Which production control started the adoption flow. Cancellation returns
/// keyboard focus to the opener, or to the surviving control when the opener
/// is gone.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UpdateOpener {
    Toast,
    Header,
}

/// The warning the bridge shows when adoption must defer. Every variant offers
/// cancellation only: a warning acknowledgment is never permission to discard
/// work, and the update waits until the user resolves the cause and chooses
/// Update Now again.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UpdateDialog {
    /// Committed work is at risk; adoption deferred until it settles.
    Deferred { blockers: Vec<UpdateBlocker> },
    /// Other open tabs keep the waiting build from adopting safely. They are
    /// never forced, closed or reloaded; the user closes them and retries.
    OtherTabs,
}

/// The adoption flow phase. Anything but [`UpdatePhase::Idle`] ignores further
/// activation, so repeated clicks and stale notifications cannot adopt twice.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum UpdatePhase {
    #[default]
    Idle,
    /// Safety passed; the adoption request is being issued.
    Checking,
    /// Safety passed; the waiting worker is taking over.
    Adopting,
    /// The worker took over but the page has not reloaded: work began after the
    /// safety check, so the reload waits until every owner settles again.
    Installed,
}

/// Decide whether adopting the waiting build risks committed work.
///
/// `Ok(())` means every owner is settled: no engine work, no save in flight
/// or failed, storage verified, and no recovery outstanding. Any `Err` lists
/// every independent reason adoption must defer; the caller warns and offers
/// cancellation only, never a discard-and-proceed path.
pub fn update_safety(inputs: &UpdateSafetyInputs) -> Result<(), Vec<UpdateBlocker>> {
    let mut blockers = Vec::new();
    if inputs.lifecycle == Lifecycle::RecoveryRequired {
        blockers.push(UpdateBlocker::RecoveryRequired);
    }
    if inputs.storage_initializing || inputs.storage_failed {
        blockers.push(UpdateBlocker::StorageNotReady);
    }
    if inputs.session_pending_work {
        blockers.push(UpdateBlocker::PendingWork);
    }
    if inputs.save_conflict || matches!(inputs.durability, Durability::Failed { .. }) {
        blockers.push(UpdateBlocker::SaveFailed);
    }
    let save_unresolved = match &inputs.durability {
        Durability::Saving { .. } => true,
        Durability::Failed { .. } => false,
        Durability::Saved { revision } => inputs.accepted_revision != Some(*revision),
        Durability::NoDocument => inputs.accepted_revision.is_some(),
    };
    if save_unresolved
        || (inputs.save_started_or_retained
            && !blockers.contains(&UpdateBlocker::SaveFailed)
            && !matches!(inputs.durability, Durability::Saving { .. }))
    {
        blockers.push(UpdateBlocker::SaveInFlight);
    }
    if inputs.invalid_draft {
        blockers.push(UpdateBlocker::InvalidDraft);
    }
    if blockers.is_empty() {
        Ok(())
    } else {
        Err(blockers)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_blocked(inputs: &UpdateSafetyInputs, expected: &[UpdateBlocker]) {
        assert_eq!(
            update_safety(inputs),
            Err(expected.to_vec()),
            "wrong safety decision for {inputs:?}"
        );
    }

    #[test]
    fn settled_states_are_safe() {
        assert_eq!(update_safety(&UpdateSafetyInputs::settled_empty()), Ok(()));
        assert_eq!(update_safety(&UpdateSafetyInputs::settled_saved(0)), Ok(()));
        assert_eq!(update_safety(&UpdateSafetyInputs::settled_saved(7)), Ok(()));
    }

    #[test]
    fn engine_work_blocks_even_with_a_saved_document() {
        let inputs = UpdateSafetyInputs {
            session_pending_work: true,
            ..UpdateSafetyInputs::settled_saved(3)
        };
        assert_blocked(&inputs, &[UpdateBlocker::PendingWork]);
    }

    #[test]
    fn saving_never_counts_as_safe() {
        for inputs in [
            UpdateSafetyInputs {
                lifecycle: Lifecycle::Saving,
                durability: Durability::Saving { revision: 4 },
                accepted_revision: Some(3),
                ..UpdateSafetyInputs::settled_empty()
            },
            // Accepted ahead of its last save: the save has not caught up.
            UpdateSafetyInputs {
                durability: Durability::Saved { revision: 3 },
                accepted_revision: Some(4),
                ..UpdateSafetyInputs::settled_saved(3)
            },
            // An accepted document with no durability record at all.
            UpdateSafetyInputs {
                durability: Durability::NoDocument,
                accepted_revision: Some(1),
                ..UpdateSafetyInputs::settled_empty()
            },
            // The Runtime save buffer disagrees with an otherwise clean record:
            // a save started that the Session has not answered yet.
            UpdateSafetyInputs {
                save_started_or_retained: true,
                ..UpdateSafetyInputs::settled_saved(2)
            },
        ] {
            assert_blocked(&inputs, &[UpdateBlocker::SaveInFlight]);
        }
    }

    #[test]
    fn failed_or_conflicted_saves_never_count_as_safe() {
        for inputs in [
            UpdateSafetyInputs {
                durability: Durability::Failed {
                    revision: 5,
                    reason: "quota exceeded".into(),
                },
                accepted_revision: Some(5),
                save_started_or_retained: true,
                ..UpdateSafetyInputs::settled_empty()
            },
            UpdateSafetyInputs {
                save_conflict: true,
                ..UpdateSafetyInputs::settled_saved(5)
            },
        ] {
            assert_blocked(&inputs, &[UpdateBlocker::SaveFailed]);
        }
    }

    #[test]
    fn storage_and_recovery_block_with_their_own_reasons() {
        assert_blocked(
            &UpdateSafetyInputs {
                storage_initializing: true,
                ..UpdateSafetyInputs::settled_saved(1)
            },
            &[UpdateBlocker::StorageNotReady],
        );
        assert_blocked(
            &UpdateSafetyInputs {
                storage_failed: true,
                ..UpdateSafetyInputs::settled_saved(1)
            },
            &[UpdateBlocker::StorageNotReady],
        );
        assert_blocked(
            &UpdateSafetyInputs {
                lifecycle: Lifecycle::RecoveryRequired,
                ..UpdateSafetyInputs::settled_saved(1)
            },
            &[UpdateBlocker::RecoveryRequired],
        );
    }

    #[test]
    fn independent_blockers_collect_together() {
        assert_blocked(
            &UpdateSafetyInputs {
                lifecycle: Lifecycle::RecoveryRequired,
                storage_initializing: true,
                session_pending_work: true,
                durability: Durability::Saving { revision: 2 },
                accepted_revision: Some(1),
                save_started_or_retained: true,
                save_conflict: true,
                storage_failed: false,
                invalid_draft: true,
            },
            &[
                UpdateBlocker::RecoveryRequired,
                UpdateBlocker::StorageNotReady,
                UpdateBlocker::PendingWork,
                UpdateBlocker::SaveFailed,
                UpdateBlocker::SaveInFlight,
                UpdateBlocker::InvalidDraft,
            ],
        );
    }

    #[test]
    fn an_invalid_draft_blocks_an_otherwise_settled_project() {
        assert_blocked(
            &UpdateSafetyInputs {
                invalid_draft: true,
                ..UpdateSafetyInputs::settled_saved(3)
            },
            &[UpdateBlocker::InvalidDraft],
        );
        assert_blocked(
            &UpdateSafetyInputs {
                invalid_draft: true,
                ..UpdateSafetyInputs::settled_empty()
            },
            &[UpdateBlocker::InvalidDraft],
        );
    }
}
