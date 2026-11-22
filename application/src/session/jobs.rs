use super::*;

impl Session {
    pub(super) fn complete_export(
        &mut self,
        operation_id: OperationId,
        result: Result<(SnapshotToken, Scope), String>,
        effects: &mut Vec<Effect>,
    ) {
        let Some(index) = self.exports.iter().position(|(op, _, _)| *op == operation_id) else {
            return;
        };
        let outcome = match result {
            Ok((token, scope)) if self.export_is_current(operation_id, token, &scope) => {
                TerminalOutcome::Completed
            }
            Ok(_) => TerminalOutcome::Cancelled,
            Err(reason) => TerminalOutcome::ExecutorFailed(reason),
        };
        self.exports.swap_remove(index);
        self.settle(operation_id, outcome, effects);
    }

    pub(super) fn retain_export(&mut self, retained: Option<OperationId>, effects: &mut Vec<Effect>) {
        for export in std::mem::take(&mut self.exports) {
            if Some(export.0) == retained {
                self.exports.push(export);
            } else {
                effects.push(Effect::CancelExport {
                    operation_id: export.0,
                });
                self.settle(export.0, TerminalOutcome::Cancelled, effects);
            }
        }
    }
    pub(super) fn cancel_job(&mut self, effects: &mut Vec<Effect>) {
        if let Some((job_id, operation_id, _)) = self.active_job.take() {
            effects.push(Effect::CancelJob { job_id });
            self.model.generation = GenerationStatus::Cancelled { job_id };
            self.settle(operation_id, TerminalOutcome::Cancelled, effects);
        }
    }
    pub(super) fn start_generation_now(
        &mut self,
        operation_id: OperationId,
        scope: Scope,
        effects: &mut Vec<Effect>,
    ) {
        let Some(snapshot) = self.model.accepted.clone().filter(|snapshot| {
            self.model.lifecycle == Lifecycle::Ready
                && self.model.durability
                    == Durability::Saved {
                        revision: snapshot.document.revision,
                    }
        }) else {
            self.reject(operation_id, "generation requires an accepted saved snapshot", effects);
            return;
        };
        if !self.scope_is_current(&scope) {
            self.reject(operation_id, "generation scope is stale", effects);
            return;
        }
        self.cancel_job(effects);
        let job_id = JobId(self.next_job);
        self.next_job += 1;
        self.active_job = Some((job_id, operation_id, scope.clone()));
        self.model.generation = GenerationStatus::Preparing { job_id };
        effects.push(Effect::RunGeneration {
            job_id,
            scope,
            snapshot,
        });
    }
    pub(super) fn start_export_now(
        &mut self,
        operation_id: OperationId,
        scope: Scope,
        effects: &mut Vec<Effect>,
    ) {
        let Some(snapshot) = self
            .model
            .accepted
            .clone()
            .filter(|_| self.model.lifecycle == Lifecycle::Ready)
        else {
            self.reject(operation_id, "export requires a ready accepted snapshot", effects);
            return;
        };
        if snapshot.scene.revision != snapshot.document.revision || !self.scope_is_current(&scope) {
            self.reject(
                operation_id,
                "export snapshot or scope is not current",
                effects,
            );
            return;
        }
        self.exports
            .push((operation_id, scope.clone(), snapshot.token));
        effects.push(Effect::RunExport {
            operation_id,
            scope,
            snapshot,
        });
    }
    pub(super) fn finish_generation(
        &mut self,
        job_id: JobId,
        scope: &Scope,
        status: GenerationStatus,
        outcome: TerminalOutcome,
        effects: &mut Vec<Effect>,
    ) {
        if let Some((active, operation, captured)) = self.active_job.as_ref()
            && *active == job_id
            && captured == scope
            && self.scope_is_current(scope)
        {
            let operation = *operation;
            self.active_job = None;
            if let TerminalOutcome::ExecutorFailed(reason) | TerminalOutcome::Rejected(reason) =
                &outcome
            {
                self.model.last_error = Some(reason.clone());
            }
            self.model.generation = status;
            self.settle(operation, outcome, effects);
        }
    }

}
