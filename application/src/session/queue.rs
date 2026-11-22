use super::*;

impl Session {
    pub(super) fn queue_open(
        &mut self,
        operation_id: OperationId,
        document: ProjectDoc,
        effects: &mut Vec<Effect>,
    ) {
        if matches!(
            self.model.durability,
            Durability::Saving { .. } | Durability::Failed { .. }
        ) {
            self.settle(
                operation_id,
                TerminalOutcome::Rejected("save the current project before opening another".into()),
                effects,
            );
            return;
        }
        if let Err(reason) = (self.validate_open)(&document) {
            self.settle(operation_id, TerminalOutcome::Rejected(reason), effects);
            return;
        }
        // Pointer-up already owns its commit, including one waiting for a preview reply.
        let committing = self
            .gesture
            .as_ref()
            .is_some_and(|gesture| gesture.commit_queued || gesture.final_positions.is_some());
        if !committing {
            self.invalidate_gesture(effects);
        }
        self.cancel_job(effects);
        self.cancel_exports(effects);
        self.queue.push_back(Intent {
            operation_id,
            submitted_epoch: self.epoch(),
            kind: IntentKind::Open(Box::new(document)),
            strict_revision: false,
        });
        self.model.lifecycle = Lifecycle::Opening;
    }
    pub(super) fn enqueue(
        &mut self,
        operation_id: OperationId,
        kind: IntentKind,
        strict_revision: bool,
        effects: &mut Vec<Effect>,
    ) {
        if self.model.lifecycle == Lifecycle::Closing {
            self.settle(
                operation_id,
                TerminalOutcome::BlockedByRecovery("session is closing".into()),
                effects,
            );
            return;
        }
        if self.model.lifecycle == Lifecycle::RecoveryRequired {
            self.settle(
                operation_id,
                TerminalOutcome::BlockedByRecovery("save recovery is required".into()),
                effects,
            );
            return;
        }
        if self.model.accepted.is_none() {
            self.settle(
                operation_id,
                TerminalOutcome::Rejected("no accepted document is open".into()),
                effects,
            );
            return;
        }
        let epoch = self.epoch();
        let captured_revision = match &kind {
            IntentKind::Edit(command) if strict_revision => Some(command.base_revision),
            IntentKind::ReviewElectricalRemap { base_revision, .. } if strict_revision => {
                Some(*base_revision)
            }
            IntentKind::ExportCommit { commit, .. } => Some(match commit {
                ExportCommitRequest::ApplyElectrical { plan, .. }
                | ExportCommitRequest::ProtectElectricalHandoff { plan } => plan.revision,
            }),
            _ => None,
        };
        if captured_revision.is_some_and(|revision| {
            revision != self.model.accepted.as_ref().unwrap().document.revision
        }) {
            self.settle(
                operation_id,
                TerminalOutcome::Rejected("captured edit revision is stale".into()),
                effects,
            );
            return;
        }
        self.queue.push_back(Intent {
            operation_id,
            submitted_epoch: epoch,
            kind,
            strict_revision,
        });
    }
    pub(super) fn enqueue_aux(
        &mut self,
        operation_id: OperationId,
        kind: IntentKind,
        effects: &mut Vec<Effect>,
    ) {
        if self.model.lifecycle == Lifecycle::RecoveryRequired
            || self.model.lifecycle == Lifecycle::Closing
        {
            self.settle(
                operation_id,
                TerminalOutcome::BlockedByRecovery(
                    "session cannot start jobs during recovery or close".into(),
                ),
                effects,
            );
            return;
        }
        if let Some(scope) = match &kind {
            IntentKind::Generate(scope) | IntentKind::Export(scope) => Some(scope),
            _ => None,
        } && !self.scope_is_current(scope)
        {
            self.settle(
                operation_id,
                TerminalOutcome::Rejected("job scope is stale".into()),
                effects,
            );
            return;
        }
        self.queue.push_back(Intent {
            operation_id,
            submitted_epoch: self.epoch(),
            kind,
            strict_revision: false,
        });
    }
    pub(super) fn pump(&mut self, effects: &mut Vec<Effect>) {
        if self.active_core.is_some()
            || self.model.lifecycle == Lifecycle::RecoveryRequired
            || self.model.lifecycle == Lifecycle::Closed
        {
            return;
        }
        while let Some(intent) = self.queue.pop_front() {
            if self.pending_save.is_some() && matches!(intent.kind, IntentKind::Open(_)) {
                self.queue.push_front(intent);
                return;
            }
            if intent.submitted_epoch != self.epoch() && !matches!(intent.kind, IntentKind::Open(_))
            {
                self.settle(
                    intent.operation_id,
                    TerminalOutcome::Rejected(DOCUMENT_SESSION_CHANGED.into()),
                    effects,
                );
                continue;
            }
            if let IntentKind::Edit(command) = &intent.kind
                && command.phase == EditPhase::Preview
                && self
                    .model
                    .accepted
                    .as_ref()
                    .is_none_or(|snapshot| snapshot.document.revision != command.base_revision)
            {
                self.settle(intent.operation_id, TerminalOutcome::Superseded, effects);
                continue;
            }
            if let IntentKind::Generate(scope) = &intent.kind {
                if self.model.lifecycle != Lifecycle::Ready {
                    self.queue.push_front(intent);
                    return;
                }
                self.start_generation_now(intent.operation_id, scope.clone(), effects);
                continue;
            }
            if let IntentKind::Export(scope) = &intent.kind {
                if self.model.lifecycle != Lifecycle::Ready {
                    self.queue.push_front(intent);
                    return;
                }
                self.start_export_now(intent.operation_id, scope.clone(), effects);
                continue;
            }
            if let IntentKind::ClearPreview {
                token,
                revision,
                board_id,
                transaction_id,
            } = &intent.kind
            {
                let current = self.model.accepted.as_ref().is_some_and(|snapshot| {
                    snapshot.token == *token
                        && snapshot.document.revision == *revision
                        && self.model.active_board_id == *board_id
                });
                if current {
                    if self.display_preview_transaction_id.as_deref()
                        == Some(transaction_id.as_str())
                    {
                        self.model.display_preview = None;
                        self.model.snap_guide = None;
                        self.display_preview_transaction_id = None;
                    }
                    self.settle(intent.operation_id, TerminalOutcome::Completed, effects);
                } else {
                    self.settle(intent.operation_id, TerminalOutcome::Superseded, effects);
                }
                continue;
            }
            if let IntentKind::ResolveEdit { label, resolver } = &intent.kind {
                // Enqueue refuses edits without an accepted document, and nothing clears
                // one afterwards, so the resolver always has the snapshot to read.
                let accepted_snapshot = self
                    .model
                    .accepted
                    .as_ref()
                    .expect("queued edits always have an accepted document")
                    .clone();
                let landing = Landing {
                    revision: accepted_snapshot.document.revision,
                    token: accepted_snapshot.token,
                };
                match resolver.resolve(&accepted_snapshot) {
                    Resolution::Submit(mut command) => {
                        if command.phase != EditPhase::Commit {
                            self.settle(
                                intent.operation_id,
                                TerminalOutcome::Rejected(
                                    "a resolved edit must be a commit; previews do not resolve"
                                        .into(),
                                ),
                                effects,
                            );
                        } else {
                            if command.transaction_id.is_empty() {
                                command.transaction_id =
                                    format!("m1-{label}-{}", intent.operation_id.0);
                            }
                            command.base_revision = accepted_snapshot.document.revision;
                            self.queue.push_front(Intent {
                                operation_id: intent.operation_id,
                                submitted_epoch: intent.submitted_epoch,
                                kind: IntentKind::Edit(command),
                                strict_revision: false,
                            });
                        }
                    }
                    Resolution::Unchanged => {
                        self.settle_landed(intent.operation_id, landing, effects);
                    }
                    Resolution::Retire(reason) => {
                        self.settle(
                            intent.operation_id,
                            TerminalOutcome::Rejected(reason),
                            effects,
                        );
                    }
                }
                continue;
            }
            let captured_revision = match &intent.kind {
                IntentKind::Edit(command)
                | IntentKind::GesturePreview(command)
                | IntentKind::GestureCommit(command)
                    if intent.strict_revision =>
                {
                    Some(command.base_revision)
                }
                IntentKind::ReviewElectricalRemap { base_revision, .. }
                    if intent.strict_revision =>
                {
                    Some(*base_revision)
                }
                IntentKind::ExportCommit { owner, commit } => {
                    if !self.export_is_current(owner.operation_id, owner.token, &owner.scope) {
                        self.settle(intent.operation_id, TerminalOutcome::Superseded, effects);
                        continue;
                    }
                    Some(match commit {
                        ExportCommitRequest::ApplyElectrical { plan, .. }
                        | ExportCommitRequest::ProtectElectricalHandoff { plan } => plan.revision,
                    })
                }
                _ => None,
            };
            if captured_revision.is_some_and(|revision| {
                self.model
                    .accepted
                    .as_ref()
                    .is_none_or(|s| s.document.revision != revision)
            }) {
                self.settle(
                    intent.operation_id,
                    TerminalOutcome::Rejected("captured edit revision is stale".into()),
                    effects,
                );
                self.finish_gesture_commit(None, false, effects);
                continue;
            }
            let request_id = RequestId(self.next_request);
            self.next_request += 1;
            let request_wire_id = format!("m1-{}", request_id.0);
            let preview_transaction_id = match &intent.kind {
                IntentKind::Edit(command) if command.phase == EditPhase::Preview => {
                    Some(command.transaction_id.clone())
                }
                IntentKind::GesturePreview(command) => Some(command.transaction_id.clone()),
                _ => None,
            };
            let export_owner = match &intent.kind {
                IntentKind::ExportCommit { owner, .. } => Some(owner.clone()),
                _ => None,
            };
            let (request, kind, gesture_generation) = match intent.kind {
                IntentKind::ResolveEdit { .. } => {
                    unreachable!("resolved intents are handled before the request is built")
                }
                IntentKind::Open(document) => (
                    CoreRequest::Open {
                        id: request_wire_id.clone(),
                        document: *document,
                    },
                    ActiveKind::Open,
                    None,
                ),
                IntentKind::Edit(mut command) => {
                    if !intent.strict_revision
                        && let Some(accepted) = &self.model.accepted
                    {
                        command.base_revision = accepted.document.revision;
                    }
                    let kind = if command.phase == EditPhase::Preview {
                        ActiveKind::Preview
                    } else {
                        ActiveKind::Commit
                    };
                    (
                        CoreRequest::Edit {
                            id: request_wire_id.clone(),
                            command,
                        },
                        kind,
                        None,
                    )
                }
                IntentKind::ReviewElectricalRemap {
                    base_revision,
                    board_id,
                    expected_fingerprint,
                } => (
                    CoreRequest::ReviewElectricalRemap {
                        id: request_wire_id.clone(),
                        base_revision,
                        board_id,
                        expected_fingerprint,
                    },
                    ActiveKind::ReviewElectricalRemap,
                    None,
                ),
                IntentKind::ExportCommit { owner, commit } => {
                    let request = match commit {
                        ExportCommitRequest::ApplyElectrical { plan, draft } => {
                            CoreRequest::ApplyElectrical {
                                id: request_wire_id.clone(),
                                base_revision: plan.revision,
                                plan,
                                draft,
                            }
                        }
                        ExportCommitRequest::ProtectElectricalHandoff { plan } => {
                            CoreRequest::ProtectElectricalHandoff {
                                id: request_wire_id.clone(),
                                base_revision: plan.revision,
                                board_id: owner.scope.board_id,
                                plan,
                            }
                        }
                    };
                    (request, ActiveKind::ExportCommit, None)
                }
                IntentKind::Undo => (
                    CoreRequest::Undo {
                        id: request_wire_id.clone(),
                    },
                    ActiveKind::Undo,
                    None,
                ),
                IntentKind::Redo => (
                    CoreRequest::Redo {
                        id: request_wire_id.clone(),
                    },
                    ActiveKind::Redo,
                    None,
                ),
                IntentKind::GesturePreview(command) => (
                    CoreRequest::Edit {
                        id: request_wire_id.clone(),
                        command,
                    },
                    ActiveKind::Preview,
                    self.gesture.as_ref().map(|g| g.generation),
                ),
                IntentKind::Generate(_) | IntentKind::Export(_) => return,
                IntentKind::ClearPreview { .. } => return,
                IntentKind::GestureCommit(mut command) => {
                    if let Some(g) = &self.gesture {
                        command.base_revision = g.base_revision;
                        command.transaction_id = g.transaction_id.clone();
                    }
                    (
                        CoreRequest::Edit {
                            id: request_wire_id.clone(),
                            command,
                        },
                        ActiveKind::GestureCommit,
                        self.gesture.as_ref().map(|g| g.generation),
                    )
                }
            };
            self.model.lifecycle = if kind == ActiveKind::Open {
                Lifecycle::Opening
            } else {
                Lifecycle::Applying
            };
            self.active_core = Some(ActiveCore {
                request_id,
                executor_epoch: self.core_epoch,
                operation_id: intent.operation_id,
                kind,
                gesture_generation,
                preview_transaction_id,
                preview_cancelled: false,
                export_owner,
            });
            effects.push(Effect::Core {
                operation_id: intent.operation_id,
                request_id,
                executor_epoch: self.core_epoch,
                request: Box::new(request),
            });
            return;
        }
        if self.model.lifecycle != Lifecycle::RecoveryRequired && self.close_operation.is_none() {
            self.model.lifecycle = if self.model.accepted.is_some() {
                Lifecycle::Ready
            } else {
                Lifecycle::Empty
            };
        }
    }
}
