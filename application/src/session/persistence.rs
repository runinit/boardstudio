use super::*;

pub(super) struct PendingSave {
    pub(super) save_attempt_id: SaveAttemptId,
    revision: u64,
    operation: Option<OperationId>,
}

pub(super) struct PendingOpen {
    operation_id: OperationId,
    pub(super) save_attempt_id: SaveAttemptId,
    document: Arc<ProjectDoc>,
    scene: Arc<SceneDelta>,
    core: boardstudio_core::CoreEngine,
}

impl Session {
    pub(super) fn close(&mut self, operation_id: OperationId, effects: &mut Vec<Effect>) {
        self.invalidate_gesture(effects);
        self.cancel_job(effects);
        self.retain_export(None, effects);
        if let Some(pending) = self.pending_open.take() {
            self.settle(pending.operation_id, TerminalOutcome::Cancelled, effects);
        }
        if matches!(self.model.durability, Durability::Failed { .. }) {
            self.reject(operation_id, "save the project before closing", effects);
            return;
        }
        if self.pending_save.is_some() {
            if let Some(prior) = self.close_operation.replace(operation_id) {
                self.settle(prior, TerminalOutcome::Superseded, effects);
            }
            self.model.lifecycle = Lifecycle::Closing;
        } else {
            self.finish_close(operation_id, effects);
        }
    }
    pub(super) fn finish_close(&mut self, operation_id: OperationId, effects: &mut Vec<Effect>) {
        self.model.lifecycle = Lifecycle::Closed;
        self.model.display_preview = None;
        self.display_preview_transaction_id = None;
        self.active_job = None;
        self.settle(operation_id, TerminalOutcome::Completed, effects);
    }    pub(super) fn open(
        &mut self,
        operation_id: OperationId,
        document: ProjectDoc,
        effects: &mut Vec<Effect>,
    ) {
        if self.pending_save.is_some() {
            self.reject(
                operation_id,
                "save the current project before opening another",
                effects,
            );
            return;
        }
        if let Err(reason) = (self.validate_open)(&document) {
            self.settle(operation_id, TerminalOutcome::Rejected(reason), effects);
            return;
        }
        let mut next_core = boardstudio_core::CoreEngine::new();
        let reply = next_core.handle(CoreRequest::Open {
            id: wire_id(operation_id),
            document,
        });
        match reply {
            CoreReply::Scene {
                scene, document, ..
            } if scene.revision == document.revision => {
                if let Some(prior) = self.pending_open.take() {
                    self.settle(prior.operation_id, TerminalOutcome::Superseded, effects);
                }
                let document = Arc::new(*document);
                let save_attempt_id = SaveAttemptId(self.next_save);
                self.next_save += 1;
                self.pending_open = Some(PendingOpen {
                    operation_id,
                    save_attempt_id,
                    document: document.clone(),
                    scene: Arc::new(scene),
                    core: next_core,
                });
                effects.push(Effect::Persist {
                    save_attempt_id,
                    document,
                });
            }
            CoreReply::Error { message, .. } => {
                self.model.last_error = Some(message.clone());
                self.settle(operation_id, TerminalOutcome::Rejected(message), effects);
            }
            CoreReply::Scene { .. } => {
                let failure = "core scene and document revisions differ";
                self.model.last_error = Some(failure.into());
                self.settle(
                    operation_id,
                    TerminalOutcome::ExecutorFailed(failure.into()),
                    effects,
                );
            }
            _ => {
                let failure = "core returned an unexpected reply variant";
                self.model.last_error = Some(failure.into());
                self.settle(
                    operation_id,
                    TerminalOutcome::ExecutorFailed(failure.into()),
                    effects,
                );
            }
        }
    }

    pub(super) fn start_save(&mut self, operation: Option<OperationId>, effects: &mut Vec<Effect>) {
        if self.pending_save.is_some() {
            return;
        }
        let Some(accepted) = self.model.accepted.as_ref() else {
            return;
        };
        let document = accepted.document.clone();
        let save_attempt_id = SaveAttemptId(self.next_save);
        self.next_save += 1;
        self.pending_save = Some(PendingSave {
            save_attempt_id,
            revision: document.revision,
            operation,
        });
        self.model.durability = Durability::Saving {
            revision: document.revision,
        };
        effects.push(Effect::Persist {
            save_attempt_id,
            document,
        });
    }

    pub(super) fn persist_completed(
        &mut self,
        save_attempt_id: SaveAttemptId,
        result: SaveResult,
        effects: &mut Vec<Effect>,
    ) {
        if let Some(pending) = self
            .pending_open
            .take_if(|pending| pending.save_attempt_id == save_attempt_id)
        {
            match result {
                SaveResult::Committed => {
                    self.invalidate_gesture(effects);
                    self.cancel_job(effects);
                    self.retain_export(None, effects);
                    self.core = pending.core;
                    if let Some(operation) =
                        self.pending_save.take().and_then(|save| save.operation)
                    {
                        self.settle(operation, TerminalOutcome::Superseded, effects);
                    }
                    let revision = pending.document.revision;
                    let epoch = SessionEpoch(self.epoch().0 + 1);
                    let token = SnapshotToken(self.next_token);
                    self.next_token += 1;
                    self.model.accepted = Some(AcceptedSnapshot {
                        token,
                        session_epoch: epoch,
                        document: pending.document,
                        scene: pending.scene,
                    });
                    self.model.display_preview = None;
                    self.display_preview_transaction_id = None;
                    self.model.selected_part_ids.clear();
                    self.model.active_instance_id = None;
                    self.model.last_error = None;
                    self.reconcile_selection_and_board();
                    self.model.durability = Durability::Saved { revision };
                    self.model.lifecycle = Lifecycle::Ready;
                    self.settle(pending.operation_id, TerminalOutcome::Completed, effects);
                }
                SaveResult::Aborted(reason) => {
                    self.model.last_error = Some(reason.clone());
                    self.settle(
                        pending.operation_id,
                        TerminalOutcome::PersistenceFailed(reason),
                        effects,
                    );
                }
            }
            return;
        }
        let Some(pending) = self
            .pending_save
            .take_if(|pending| pending.save_attempt_id == save_attempt_id)
        else {
            return;
        };
        let revision = self
            .model
            .accepted
            .as_ref()
            .expect("saving an open project")
            .document
            .revision;
        let outcome = match result {
            SaveResult::Committed if pending.revision != revision => {
                self.start_save(pending.operation, effects);
                return;
            }
            SaveResult::Committed => {
                self.model.durability = Durability::Saved { revision };
                TerminalOutcome::Completed
            }
            SaveResult::Aborted(reason) => {
                self.model.durability = Durability::Failed {
                    revision,
                    reason: reason.clone(),
                };
                TerminalOutcome::PersistenceFailed(reason)
            }
        };
        let failed = matches!(outcome, TerminalOutcome::PersistenceFailed(_));
        if let Some(operation) = pending.operation {
            self.settle(operation, outcome, effects);
        }
        if failed && let Some(operation) = self.close_operation.take() {
            self.model.lifecycle = Lifecycle::Ready;
            self.reject(operation, "save failed before close", effects);
        }
    }

    pub(super) fn retry_save(&mut self, operation_id: OperationId, effects: &mut Vec<Effect>) {
        if matches!(self.model.durability, Durability::Failed { .. }) {
            self.start_save(Some(operation_id), effects);
        } else {
            self.reject(operation_id, "there is no failed save to retry", effects);
        }
    }

}
