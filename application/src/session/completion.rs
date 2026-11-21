use super::*;

impl Session {
    pub(super) fn core_completed(
        &mut self,
        request_id: RequestId,
        executor_epoch: ExecutorEpoch,
        reply: CoreReply,
        effects: &mut Vec<Effect>,
    ) {
        let Some(active) = self.active_core.take() else {
            return;
        };
        if active.request_id != request_id
            || active.executor_epoch != executor_epoch
            || executor_epoch != self.core_epoch
        {
            self.active_core = Some(active);
            return;
        }
        if reply_id(&reply) != format!("m1-{}", request_id.0) {
            self.model.lifecycle = Lifecycle::RecoveryRequired;
            self.model.last_error = Some("core reply ID did not match request".into());
            self.settle(
                active.operation_id,
                TerminalOutcome::ExecutorFailed("core reply ID did not match request".into()),
                effects,
            );
            self.block_queue("core reply ID mismatch", effects);
            return;
        }
        match (active.kind, reply) {
            (ActiveKind::Preview, CoreReply::Preview { scene, .. }) => {
                if !active.preview_cancelled
                    && active.gesture_generation.is_none_or(|generation| {
                        self.gesture
                            .as_ref()
                            .is_some_and(|g| g.generation == generation)
                    })
                {
                    self.model.display_preview = Some(Arc::new(scene));
                    self.display_preview_transaction_id = active.preview_transaction_id.clone();
                }
                if active.gesture_generation.is_none() {
                    self.settle(active.operation_id, TerminalOutcome::Completed, effects);
                }
                if let Some(g) = &mut self.gesture
                    && active.gesture_generation == Some(g.generation)
                {
                    g.preview_in_flight = false;
                }
                self.dispatch_latest_gesture_preview(effects);
            }
            (
                ActiveKind::Open
                | ActiveKind::Commit
                | ActiveKind::ReviewElectricalRemap
                | ActiveKind::ExportCommit
                | ActiveKind::Undo
                | ActiveKind::Redo
                | ActiveKind::GestureCommit,
                CoreReply::Scene {
                    scene, document, ..
                },
            ) => {
                let document = Arc::new(*document);
                if scene.revision != document.revision {
                    self.model.lifecycle = Lifecycle::RecoveryRequired;
                    self.model.last_error = Some("core scene and document revisions differ".into());
                    self.settle(
                        active.operation_id,
                        TerminalOutcome::ExecutorFailed(
                            "core scene and document revisions differ".into(),
                        ),
                        effects,
                    );
                    return;
                }
                let save_attempt_id = SaveAttemptId(self.next_save);
                self.next_save += 1;
                let assets = document.assets.iter().map(AssetRef::from).collect();
                let scene = Arc::new(scene);
                self.pending_save = Some(PendingSave {
                    operation_id: active.operation_id,
                    document: document.clone(),
                    scene,
                    kind: active.kind,
                    save_attempt_id,
                    retry_operation_id: None,
                    export_owner: active.export_owner,
                });
                self.model.lifecycle = Lifecycle::Saving;
                self.model.durability = Durability::Saving {
                    revision: document.revision,
                };
                effects.push(Effect::Persist {
                    operation_id: active.operation_id,
                    save_attempt_id,
                    document,
                    assets,
                });
            }
            (_, CoreReply::Error { message, .. }) => {
                self.model.last_error = Some(message.clone());
                self.settle(
                    active.operation_id,
                    TerminalOutcome::Rejected(message),
                    effects,
                );
                self.model.lifecycle = if self.model.accepted.is_some() {
                    Lifecycle::Ready
                } else {
                    Lifecycle::Empty
                };
                if active.gesture_generation.is_some() {
                    self.finish_gesture_commit(active.gesture_generation, false, effects);
                }
            }
            _ => {
                let reason = "core returned an unexpected reply variant".to_string();
                self.model.last_error = Some(reason.clone());
                self.model.lifecycle = Lifecycle::RecoveryRequired;
                self.settle(
                    active.operation_id,
                    TerminalOutcome::ExecutorFailed(reason.clone()),
                    effects,
                );
                self.block_queue(&reason, effects);
            }
        }
    }
    pub(super) fn persist_completed(
        &mut self,
        save_attempt_id: SaveAttemptId,
        result: SaveResult,
        effects: &mut Vec<Effect>,
    ) {
        let Some(pending) = self.pending_save.take() else {
            return;
        };
        if pending.save_attempt_id != save_attempt_id {
            self.pending_save = Some(pending);
            return;
        }
        let attempt_operation = pending.retry_operation_id.unwrap_or(pending.operation_id);
        match result {
            SaveResult::Aborted(reason) => {
                self.model.lifecycle = Lifecycle::RecoveryRequired;
                self.model.durability = Durability::Failed {
                    revision: pending.document.revision,
                    reason: reason.clone(),
                };
                self.model.last_error = Some(reason.clone());
                self.pending_save = Some(pending);
                self.settle(
                    attempt_operation,
                    TerminalOutcome::PersistenceFailed(reason.clone()),
                    effects,
                );
                self.block_queue(&reason, effects);
                if let Some(close_operation) = self.close_operation.take() {
                    self.settle(
                        close_operation,
                        TerminalOutcome::BlockedByRecovery(
                            "pending save failed; resolve recovery before closing".into(),
                        ),
                        effects,
                    );
                }
            }
            SaveResult::Committed => {
                let is_open = pending.kind == ActiveKind::Open;
                let epoch = if is_open {
                    SessionEpoch(self.epoch().0 + 1)
                } else {
                    self.epoch()
                };
                self.cancel_job(effects);
                if let Some(owner) = &pending.export_owner {
                    self.cancel_exports_except(owner.operation_id, effects);
                } else {
                    self.cancel_exports(effects);
                }
                let token = SnapshotToken(self.next_token);
                self.next_token += 1;
                let revision = pending.document.revision;
                let landing = Landing { revision, token };
                self.model.accepted = Some(AcceptedSnapshot {
                    token,
                    session_epoch: epoch,
                    document: pending.document,
                    scene: pending.scene,
                });
                if let Some(owner) = &pending.export_owner
                    && let Some(export) =
                        self.exports
                            .iter_mut()
                            .find(|(operation, scope, old_token)| {
                                *operation == owner.operation_id
                                    && *scope == owner.scope
                                    && *old_token == owner.token
                            })
                {
                    export.2 = token;
                }
                self.model.display_preview = None;
                self.display_preview_transaction_id = None;
                self.model.durability = Durability::Saved { revision };
                self.model.last_error = None;
                if is_open {
                    self.model.selected_part_ids.clear();
                    self.model.active_instance_id = None;
                }
                self.reconcile_selection_and_board();
                self.model.lifecycle = Lifecycle::Ready;
                self.settle_landed(pending.operation_id, landing, effects);
                if let Some(retry_operation_id) = pending.retry_operation_id {
                    self.settle_landed(retry_operation_id, landing, effects);
                }
                self.finish_gesture_commit(None, true, effects);
                if let Some(operation) = self.close_operation.take() {
                    self.finish_close(operation, effects);
                }
            }
        }
    }
    pub(super) fn retry_save(&mut self, operation_id: OperationId, effects: &mut Vec<Effect>) {
        let Some(mut pending) = self.pending_save.take() else {
            self.settle(
                operation_id,
                TerminalOutcome::Rejected("there is no retained save to retry".into()),
                effects,
            );
            return;
        };
        if self.model.lifecycle != Lifecycle::RecoveryRequired {
            self.pending_save = Some(pending);
            self.settle(
                operation_id,
                TerminalOutcome::Rejected("session is not in save recovery".into()),
                effects,
            );
            return;
        }
        let save_attempt_id = SaveAttemptId(self.next_save);
        self.next_save += 1;
        pending.save_attempt_id = save_attempt_id;
        pending.retry_operation_id = Some(operation_id);
        self.model.lifecycle = Lifecycle::Saving;
        self.model.durability = Durability::Saving {
            revision: pending.document.revision,
        };
        let document = pending.document.clone();
        let assets = document.assets.iter().map(AssetRef::from).collect();
        self.pending_save = Some(pending);
        effects.push(Effect::Persist {
            operation_id,
            save_attempt_id,
            document,
            assets,
        });
    }
    pub(super) fn core_failed(
        &mut self,
        request_id: RequestId,
        executor_epoch: ExecutorEpoch,
        reason: String,
        effects: &mut Vec<Effect>,
    ) {
        let Some(active) = self.active_core.take() else {
            return;
        };
        if active.request_id != request_id || active.executor_epoch != executor_epoch {
            self.active_core = Some(active);
            return;
        }
        self.core_epoch = ExecutorEpoch(self.core_epoch.0 + 1);
        effects.push(Effect::RestartCoreExecutor {
            executor_epoch: self.core_epoch,
        });
        self.model.lifecycle = Lifecycle::RecoveryRequired;
        self.model.last_error = Some(reason.clone());
        self.settle(
            active.operation_id,
            TerminalOutcome::ExecutorFailed(reason.clone()),
            effects,
        );
        self.block_queue("core outcome is unknown; explicit reopen required", effects);
        if let Some(close_operation) = self.close_operation.take() {
            self.settle(
                close_operation,
                TerminalOutcome::BlockedByRecovery(
                    "core outcome is unknown; explicit reopen required".into(),
                ),
                effects,
            );
        }
        self.finish_gesture_commit(active.gesture_generation, false, effects);
    }
    pub(super) fn block_queue(&mut self, reason: &str, effects: &mut Vec<Effect>) {
        while let Some(intent) = self.queue.pop_front() {
            self.settle(
                intent.operation_id,
                TerminalOutcome::BlockedByRecovery(reason.into()),
                effects,
            );
        }
    }
    pub(super) fn settle(
        &mut self,
        operation_id: OperationId,
        outcome: TerminalOutcome,
        effects: &mut Vec<Effect>,
    ) {
        if self.settled.insert(operation_id) {
            self.push_settled(operation_id, outcome, None, effects);
        }
    }

    /// Settle a completed operation at the accepted snapshot it just installed, so callers
    /// read exactly where it landed. A retried save re-reports the original operation with
    /// its eventual landing after the earlier failure report, so the dedup set is updated
    /// without suppressing this report.
    pub(super) fn settle_landed(
        &mut self,
        operation_id: OperationId,
        landing: Landing,
        effects: &mut Vec<Effect>,
    ) {
        self.settled.insert(operation_id);
        self.push_settled(
            operation_id,
            TerminalOutcome::Completed,
            Some(landing),
            effects,
        );
    }

    fn push_settled(
        &mut self,
        operation_id: OperationId,
        outcome: TerminalOutcome,
        landing: Option<Landing>,
        effects: &mut Vec<Effect>,
    ) {
        effects.push(Effect::Settled {
            operation_id,
            outcome,
            landing,
        });
    }
    pub(super) fn epoch(&self) -> SessionEpoch {
        self.model
            .accepted
            .as_ref()
            .map_or(SessionEpoch(0), |s| s.session_epoch)
    }
    fn reconcile_selection_and_board(&mut self) {
        let Some(snapshot) = &self.model.accepted else {
            return;
        };
        let parts = snapshot
            .document
            .parts
            .iter()
            .map(|part| part.id.as_str())
            .collect::<HashSet<_>>();
        self.model
            .selected_part_ids
            .retain(|id| parts.contains(id.as_str()));
        if !snapshot
            .document
            .boards
            .iter()
            .any(|board| board.id == self.model.active_board_id)
        {
            self.model.active_board_id = snapshot
                .document
                .boards
                .first()
                .map_or_else(String::new, |board| board.id.clone());
        }
        if self.model.active_instance_id.as_ref().is_some_and(|id| {
            !snapshot.document.hardware.as_ref().is_some_and(|h| {
                h.instances
                    .iter()
                    .any(|i| &i.id == id && i.board_id == self.model.active_board_id)
            })
        }) {
            self.model.active_instance_id = None;
        }
    }
}
