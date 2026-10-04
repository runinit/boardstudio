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

/// Remembers selected generated keys across accepted cell-disable transitions.
/// The owner supplied by the mounted Layout is its full ScopedTreeContext, so
/// another project/session/board/cell cannot inherit the remembered IDs.
pub(crate) struct SelectionRetention<K> {
    last: Option<(K, Vec<String>)>,
    suspended: Option<(K, Vec<String>)>,
}

impl<K> Default for SelectionRetention<K> {
    fn default() -> Self {
        Self {
            last: None,
            suspended: None,
        }
    }
}

impl<K: Clone + PartialEq> SelectionRetention<K> {
    pub(crate) fn reconcile(
        &mut self,
        owner: Option<&K>,
        selected_ids: &[String],
        eligible_ids: &[String],
        _live_ids: &[String],
    ) -> Vec<String> {
        let eligible = |id: &str| eligible_ids.iter().any(|candidate| candidate == id);
        if owner.is_none_or(|owner| {
            self.last
                .as_ref()
                .is_some_and(|(last_owner, _)| last_owner != owner)
                || self
                    .suspended
                    .as_ref()
                    .is_some_and(|(held_owner, _)| held_owner != owner)
        }) {
            self.last = None;
            self.suspended = None;
        }

        let reconciled = selected_ids
            .iter()
            .filter(|id| eligible(id))
            .cloned()
            .collect::<Vec<_>>();

        let Some(owner) = owner else {
            return reconciled;
        };
        if !reconciled.is_empty() {
            self.last = Some((owner.clone(), reconciled.clone()));
            self.suspended = None;
            return reconciled;
        }

        if selected_ids.is_empty()
            && self.suspended.is_none()
            && let Some((last_owner, last_ids)) = &self.last
            && last_owner == owner
        {
            if last_ids.iter().any(|id| !eligible(id)) {
                self.suspended = Some((owner.clone(), last_ids.clone()));
            } else {
                // An explicit deselection while the target is still eligible must not
                // become a future undo-driven reselection.
                self.last = None;
            }
        }

        if let Some((held_owner, held_ids)) = &self.suspended
            && held_owner == owner
        {
            let restored = held_ids
                .iter()
                .filter(|id| eligible(id))
                .cloned()
                .collect::<Vec<_>>();
            if !restored.is_empty() {
                self.last = Some((owner.clone(), restored.clone()));
                self.suspended = None;
                return restored;
            }
        }
        reconciled
    }
}

#[cfg(test)]
mod selection_retention_tests {
    use super::*;
    use boardstudio_application::{
        Completion, Event, OperationId, SaveResult, Scope, SelectionMode, Session,
    };
    use boardstudio_core::{CoreEngine, model::*};
    use std::collections::BTreeMap;

    fn advance(
        session: &mut Session,
        core: &mut CoreEngine,
        initial: Vec<boardstudio_application::Effect>,
    ) {
        let mut pending = initial;
        while let Some(effect) = pending.pop() {
            match effect {
                boardstudio_application::Effect::Core {
                    request_id,
                    executor_epoch,
                    request,
                    ..
                } => {
                    let reply = core.handle(*request);
                    pending.extend(session.complete(Completion::Core {
                        request_id,
                        executor_epoch,
                        reply: Box::new(reply),
                    }));
                }
                boardstudio_application::Effect::Persist {
                    save_attempt_id, ..
                } => {
                    pending.extend(session.complete(Completion::Persist {
                        save_attempt_id,
                        result: SaveResult::Committed,
                    }));
                }
                _ => {}
            }
        }
    }

    fn live_ids(model: &boardstudio_application::ReadModel, key_id: &str) -> Vec<String> {
        let Some(snapshot) = &model.accepted else {
            return vec![];
        };
        let Some(board) = snapshot
            .document
            .boards
            .iter()
            .find(|board| board.id == model.active_board_id)
        else {
            return vec![];
        };
        board
            .part_ids
            .iter()
            .filter(|id| {
                id.as_str() == key_id && snapshot.document.parts.iter().any(|part| &part.id == *id)
            })
            .cloned()
            .collect()
    }

    fn eligible_ids(
        model: &boardstudio_application::ReadModel,
        matrix_id: &str,
        key_id: &str,
    ) -> Vec<String> {
        let Some(snapshot) = &model.accepted else {
            return vec![];
        };
        let live = live_ids(model, key_id);
        snapshot
            .scene
            .matrix_scenes
            .iter()
            .find(|scene| scene.matrix_id == matrix_id)
            .into_iter()
            .flat_map(|scene| &scene.cells)
            .filter(|cell| cell.enabled && cell.member_id.as_deref() == Some(key_id))
            .filter_map(|cell| cell.member_id.clone())
            .filter(|id| live.contains(id))
            .collect()
    }

    fn matrix_fixture() -> ProjectDoc {
        let mut document = ProjectDoc::empty("selection", "Selection");
        document.boards.push(Board {
            id: "main".into(),
            name: "Main".into(),
            outline_ids: vec![],
            part_ids: vec![],
            net_ids: vec![],
            thickness: 1.6,
            traces: vec![],
            vias: vec![],
        });
        document.definitions.push(PartDefinition {
            hardware_profile: None,
            input_profile: None,
            id: "switch".into(),
            name: "Switch".into(),
            kind: PartKind::Switch,
            keycap: None,
            envelope_source: None,
            kicad_source: None,
            terminals: BTreeMap::new(),
            matrix_terminals: None,
            envelope_notice: None,
            courtyard: vec![],
            pads: vec![],
            models: None,
            generator: None,
            mechanical_profile: None,
        });
        document
    }

    #[test]
    fn core_disable_then_undo_restores_scoped_key_selection() {
        let mut session = Session::new();
        let mut core = CoreEngine::new();
        let effects = session.submit(Event::Open {
            operation_id: OperationId(1),
            document: matrix_fixture(),
        });
        advance(&mut session, &mut core, effects);
        let matrix = Matrix {
            id: "matrix-main".into(),
            name: None,
            rows: 1,
            columns: 1,
            pitch: Vec2 { x: 19.05, y: 19.05 },
            origin: Vec2 { x: 0.0, y: 0.0 },
            definition_id: "switch".into(),
            part_ids: vec![],
            board_id: Some("main".into()),
            mirror: None,
            rotation: None,
            edge_gap: None,
            diode_direction: None,
            row_offsets: vec![],
            column_offsets: vec![],
            column_staggers: vec![],
            column_splays: vec![],
            column_origins: vec![],
            cells: vec![],
        };
        let effects = session.submit(Event::Edit {
            operation_id: OperationId(2),
            command: EditCommand {
                base_revision: 0,
                transaction_id: "create-matrix".into(),
                phase: EditPhase::Commit,
                target_ids: vec![matrix.id.clone()],
                operation: EditOperation::SetMatrix {
                    matrix,
                    definitions: None,
                },
            },
        });
        advance(&mut session, &mut core, effects);
        let key_id = "matrix/matrix-main/r0c0";
        let accepted = session.read_model().accepted.as_ref().unwrap();
        let scope = Scope {
            session_epoch: accepted.session_epoch,
            document_id: accepted.document.id.clone(),
            board_id: "main".into(),
            instance_id: None,
        };
        session.submit(Event::SelectMatrixCell {
            operation_id: OperationId(3),
            scope: scope.clone(),
            matrix_id: "matrix-main".into(),
            target_part_id: key_id.into(),
            part_ids: vec![key_id.into()],
            mode: SelectionMode::Replace,
        });
        let owner = (scope, "key:matrix-main:r0c0".to_owned());
        let mut retention = SelectionRetention::default();
        let before = session.read_model();
        let selected = before.selected_part_ids.clone();
        assert_eq!(selected, vec![key_id]);
        assert_eq!(eligible_ids(&before, "matrix-main", key_id), selected);
        assert_eq!(
            retention.reconcile(
                Some(&owner),
                &selected,
                &eligible_ids(&before, "matrix-main", key_id),
                &live_ids(&before, key_id)
            ),
            selected
        );

        let mut disabled = before.accepted.as_ref().unwrap().document.matrices[0].clone();
        disabled.cells.push(MatrixCell {
            row: 0,
            column: 0,
            enabled: false,
            definition_id: None,
            variant: None,
            offset: None,
            rotation: None,
            assemblies: vec![],
            assemblies_local: None,
        });
        let revision = before.accepted.as_ref().unwrap().document.revision;
        let effects = session.submit(Event::Edit {
            operation_id: OperationId(4),
            command: EditCommand {
                base_revision: revision,
                transaction_id: "disable-key".into(),
                phase: EditPhase::Commit,
                target_ids: vec!["matrix-main".into()],
                operation: EditOperation::SetMatrix {
                    matrix: disabled,
                    definitions: None,
                },
            },
        });
        advance(&mut session, &mut core, effects);
        let after_disable = session.read_model();
        assert!(
            live_ids(&after_disable, key_id).is_empty(),
            "Core removes the disabled generated part from the board"
        );
        assert!(eligible_ids(&after_disable, "matrix-main", key_id).is_empty());
        assert!(
            after_disable.selected_part_ids.is_empty(),
            "Session prunes the selected part before presentation reconciliation"
        );
        assert!(
            retention
                .reconcile(
                    Some(&owner),
                    &after_disable.selected_part_ids,
                    &eligible_ids(&after_disable, "matrix-main", key_id),
                    &live_ids(&after_disable, key_id)
                )
                .is_empty()
        );

        let effects = session.submit(Event::Undo {
            operation_id: OperationId(5),
        });
        advance(&mut session, &mut core, effects);
        let after_undo = session.read_model();
        assert_eq!(live_ids(&after_undo, key_id), selected);
        assert_eq!(eligible_ids(&after_undo, "matrix-main", key_id), selected);
        let restored = retention.reconcile(
            Some(&owner),
            &after_undo.selected_part_ids,
            &eligible_ids(&after_undo, "matrix-main", key_id),
            &live_ids(&after_undo, key_id),
        );
        assert_eq!(restored, selected);
    }

    #[test]
    fn deleting_a_disabled_key_discards_its_suspended_selection() {
        let mut retention = SelectionRetention::default();
        let owner = "matrix/r0c0";
        let selected = vec!["matrix/r0c0".to_owned()];
        assert_eq!(
            retention.reconcile(Some(&owner), &selected, &selected, &selected),
            selected
        );
        assert!(retention.reconcile(None, &[], &[], &[]).is_empty());
        assert!(
            retention
                .reconcile(Some(&owner), &[], &selected, &selected)
                .is_empty()
        );
    }

    #[test]
    fn explicit_clear_and_reopened_scope_do_not_restore_old_key_ids() {
        let mut retention = SelectionRetention::default();
        let original_scope = Scope {
            session_epoch: boardstudio_application::SessionEpoch(1),
            document_id: "same-project".into(),
            board_id: "main".into(),
            instance_id: None,
        };
        let owner = (original_scope.clone(), "key:matrix:r0c0".to_owned());
        let selected = vec!["matrix/matrix/r0c0".to_owned()];
        assert_eq!(
            retention.reconcile(Some(&owner), &selected, &selected, &selected),
            selected
        );

        // An explicit deselection while the Key is still eligible clears its remembered state.
        assert!(
            retention
                .reconcile(Some(&owner), &[], &selected, &selected)
                .is_empty()
        );
        assert!(retention.reconcile(Some(&owner), &[], &[], &[]).is_empty());

        // Recreate a suspended selection, then prove session and cell identity changes clear it.
        for changed_owner in [
            (
                Scope {
                    session_epoch: boardstudio_application::SessionEpoch(2),
                    ..original_scope.clone()
                },
                "key:matrix:r0c0".to_owned(),
            ),
            (original_scope.clone(), "key:matrix:r0c1".to_owned()),
        ] {
            assert_eq!(
                retention.reconcile(Some(&owner), &selected, &selected, &selected),
                selected
            );
            assert!(retention.reconcile(Some(&owner), &[], &[], &[]).is_empty());
            assert!(
                retention
                    .reconcile(Some(&changed_owner), &[], &[], &[])
                    .is_empty()
            );
            assert!(
                retention
                    .reconcile(Some(&changed_owner), &[], &selected, &selected)
                    .is_empty()
            );
        }
    }
}
