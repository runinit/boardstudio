//! Main-thread document session and host-owned effects.
//!
//! This crate deliberately contains no browser handles. A host runs each emitted effect and
//! returns its completion through [`Session::complete`].

mod gesture;
mod jobs;
mod persistence;
mod selection;
mod types;

use boardstudio_core::{CommittedScene, CoreEngine, EditOutcome};
use boardstudio_model::model::{
    EditCommand, EditOperation, EditPhase, Matrix, MatrixScene, Position, ProjectDoc, Vec2,
};
use gesture::Gesture;
use persistence::{PendingOpen, PendingSave};
use std::sync::Arc;
pub use types::*;

pub struct Session {
    model: ReadModel,
    core: CoreEngine,
    display_preview_transaction_id: Option<String>,
    pending_save: Option<PendingSave>,
    pending_open: Option<PendingOpen>,
    next_save: u64,
    next_token: u64,
    next_gesture: u64,
    next_job: u64,
    active_job: Option<(JobId, OperationId, Scope)>,
    exports: Vec<(OperationId, Scope, SnapshotToken)>,
    close_operation: Option<OperationId>,
    gesture: Option<Gesture>,
}

impl Default for Session {
    fn default() -> Self {
        Self::new()
    }
}

impl Session {
    pub fn new() -> Self {
        Self {
            model: ReadModel::default(),
            core: CoreEngine::new(),
            display_preview_transaction_id: None,
            pending_save: None,
            pending_open: None,
            next_save: 1,
            next_token: 1,
            next_gesture: 1,
            next_job: 1,
            active_job: None,
            exports: vec![],
            close_operation: None,
            gesture: None,
        }
    }
    pub fn read_model(&self) -> &ReadModel {
        &self.model
    }

    pub fn project_matrices(
        &self,
        base_revision: u64,
        matrices: Vec<Matrix>,
    ) -> Result<Vec<MatrixScene>, String> {
        self.core
            .project_matrices(base_revision, matrices)
            .map(|(_, scenes)| scenes)
    }

    pub fn scope(&self) -> Option<Scope> {
        let accepted = self.model.accepted.as_ref()?;
        Some(Scope {
            session_epoch: accepted.session_epoch,
            document_id: accepted.document.id.clone(),
            board_id: self.model.active_board_id.clone(),
            instance_id: self.model.active_instance_id.clone(),
        })
    }
    /// Whether an asynchronous export is still owned by this session and its
    /// captured document, board, instance, and snapshot remain current.
    /// Checking membership also closes the cancellation window before a host
    /// has processed the corresponding `CancelExport` effect.
    pub fn export_is_current(
        &self,
        operation_id: OperationId,
        token: SnapshotToken,
        scope: &Scope,
    ) -> bool {
        self.exports
            .iter()
            .any(|(operation, captured_scope, captured_token)| {
                *operation == operation_id && captured_scope == scope && *captured_token == token
            })
            && self.snapshot_is_current(token, scope)
    }

    fn scope_is_current(&self, scope: &Scope) -> bool {
        self.model.accepted.as_ref().is_some_and(|snapshot| {
            snapshot.session_epoch == scope.session_epoch
                && snapshot.document.id == scope.document_id
        }) && self.model.active_board_id == scope.board_id
            && self.model.active_instance_id == scope.instance_id
    }
    pub fn snapshot_is_current(&self, token: SnapshotToken, scope: &Scope) -> bool {
        self.model
            .accepted
            .as_ref()
            .is_some_and(|s| s.token == token)
            && self.scope_is_current(scope)
    }
}

pub fn event_operation(event: &Event) -> Option<OperationId> {
    match event {
        Event::Open { operation_id, .. }
        | Event::GestureBegin { operation_id, .. }
        | Event::PreviewEdit { operation_id, .. }
        | Event::ClearPreview { operation_id, .. }
        | Event::ReviewElectricalRemap { operation_id, .. }
        | Event::Undo { operation_id }
        | Event::Redo { operation_id }
        | Event::RetrySave { operation_id }
        | Event::ResolveEdit { operation_id, .. }
        | Event::SelectParts { operation_id, .. }
        | Event::SelectMatrixCell { operation_id, .. }
        | Event::Navigate { operation_id, .. }
        | Event::SetCamera { operation_id, .. }
        | Event::StartGeneration { operation_id, .. }
        | Event::CancelGeneration { operation_id }
        | Event::StartExport { operation_id, .. }
        | Event::ExportCommit { operation_id, .. }
        | Event::Close { operation_id } => Some(*operation_id),
        _ => None,
    }
}
impl Session {
    fn resolve_edit(
        &mut self,
        operation_id: OperationId,
        label: String,
        resolver: EditResolver,
        effects: &mut Vec<Effect>,
    ) {
        if self.model.lifecycle == Lifecycle::Closing {
            self.reject(operation_id, "no editable project is open", effects);
            return;
        }
        let Some(accepted) = self.model.accepted.as_ref() else {
            self.reject(operation_id, "open a project before editing", effects);
            return;
        };
        let command = match resolver.resolve(accepted) {
            Resolution::Submit(mut command) if command.phase == EditPhase::Commit => {
                command.base_revision = accepted.document.revision;
                if command.transaction_id.is_empty() {
                    command.transaction_id = format!("m1-{label}-{}", operation_id.0);
                }
                command
            }
            Resolution::Unchanged => {
                self.settle_landed(
                    operation_id,
                    Landing {
                        revision: accepted.document.revision,
                        token: accepted.token,
                    },
                    effects,
                );
                return;
            }
            resolution => {
                let reason = match resolution {
                    Resolution::Retire(reason) => reason,
                    _ => "a resolved edit must be a commit".into(),
                };
                self.settle(operation_id, TerminalOutcome::Rejected(reason), effects);
                return;
            }
        };
        self.apply_edit(operation_id, command, false, effects);
    }

    fn apply_edit(
        &mut self,
        operation_id: OperationId,
        command: EditCommand,
        gesture: bool,
        effects: &mut Vec<Effect>,
    ) {
        self.mutate(
            operation_id,
            Some(command.base_revision),
            gesture,
            None,
            |core| core.apply_edit(command),
            effects,
        );
    }

    fn mutate(
        &mut self,
        operation_id: OperationId,
        revision: Option<u64>,
        gesture: bool,
        export_owner: Option<OperationId>,
        apply: impl FnOnce(&mut CoreEngine) -> Result<EditOutcome, String>,
        effects: &mut Vec<Effect>,
    ) {
        if matches!(self.model.lifecycle, Lifecycle::Closing | Lifecycle::Closed)
            || self.model.accepted.is_none()
        {
            self.reject(operation_id, "no editable project is open", effects);
            return;
        }
        if revision.is_some_and(|revision| {
            self.model
                .accepted
                .as_ref()
                .is_none_or(|s| s.document.revision != revision)
        }) {
            if gesture {
                self.clear_gesture(effects);
            }
            self.reject(operation_id, "captured edit revision is stale", effects);
            return;
        }
        match apply(&mut self.core) {
            Ok(EditOutcome::Preview(scene)) => {
                self.display_preview_transaction_id = Some(scene.transaction_id.clone());
                self.model.display_preview = Some(Arc::new(scene));
                if !gesture {
                    self.settle(operation_id, TerminalOutcome::Completed, effects);
                }
            }
            Ok(EditOutcome::Committed(CommittedScene { scene, document })) => {
                let document = Arc::new(document);
                let epoch = self.epoch();
                self.cancel_job(effects);
                self.retain_export(export_owner, effects);
                let token = SnapshotToken(self.next_token);
                self.next_token += 1;
                let revision = document.revision;
                let landing = Landing { revision, token };
                self.model.accepted = Some(AcceptedSnapshot {
                    token,
                    session_epoch: epoch,
                    document,
                    scene: Arc::new(scene),
                });
                if export_owner.is_some()
                    && let Some(retained) = self.exports.first_mut()
                {
                    retained.2 = token;
                }
                self.model.display_preview = None;
                self.display_preview_transaction_id = None;
                self.model.durability = Durability::Saving { revision };
                self.model.last_error = None;
                self.reconcile_selection_and_board();
                self.model.lifecycle = Lifecycle::Ready;
                if gesture {
                    self.clear_gesture(effects);
                } else {
                    self.invalidate_gesture(effects);
                }
                self.settle_landed(operation_id, landing, effects);
                self.start_save(None, effects);
            }
            Err(message) => {
                self.model.last_error = Some(message.clone());
                if gesture {
                    self.clear_gesture(effects);
                }
                self.settle(operation_id, TerminalOutcome::Rejected(message), effects);
            }
        }
    }

    pub fn submit(&mut self, event: Event) -> Vec<Effect> {
        let mut effects = Vec::new();
        if self.model.lifecycle == Lifecycle::Closed {
            if let Some(operation_id) = event_operation(&event) {
                self.settle(operation_id, TerminalOutcome::Closed, &mut effects);
            }
            return effects;
        }
        match event {
            Event::Open {
                operation_id,
                document,
            } => {
                self.open(operation_id, document, &mut effects);
            }
            Event::PreviewEdit {
                operation_id,
                transaction_id,
                target_ids,
                operation,
            } => {
                let base_revision = self
                    .model
                    .accepted
                    .as_ref()
                    .map_or(0, |snapshot| snapshot.document.revision);
                self.apply_edit(
                    operation_id,
                    EditCommand {
                        base_revision,
                        transaction_id,
                        phase: EditPhase::Preview,
                        target_ids,
                        operation,
                    },
                    false,
                    &mut effects,
                );
            }
            Event::ResolveEdit {
                operation_id,
                label,
                resolver,
            } => self.resolve_edit(operation_id, label, resolver, &mut effects),
            Event::ClearPreview {
                operation_id,
                token,
                revision,
                board_id,
                transaction_id,
            } => {
                let current = self.model.accepted.as_ref().is_some_and(|accepted| {
                    accepted.token == token
                        && accepted.document.revision == revision
                        && self.model.active_board_id == board_id
                });
                if current
                    && self.display_preview_transaction_id.as_deref() == Some(&transaction_id)
                {
                    self.model.display_preview = None;
                    self.model.snap_guide = None;
                    self.display_preview_transaction_id = None;
                }
                self.settle(
                    operation_id,
                    if current {
                        TerminalOutcome::Completed
                    } else {
                        TerminalOutcome::Superseded
                    },
                    &mut effects,
                );
            }
            Event::ReviewElectricalRemap {
                operation_id,
                base_revision,
                board_id,
                expected_fingerprint,
            } => {
                self.mutate(
                    operation_id,
                    Some(base_revision),
                    false,
                    None,
                    |core| {
                        core.review_electrical_remap(base_revision, board_id, expected_fingerprint)
                            .map(EditOutcome::Committed)
                    },
                    &mut effects,
                );
            }
            Event::Undo { operation_id } | Event::Redo { operation_id } => {
                let apply = if matches!(event, Event::Undo { .. }) {
                    CoreEngine::undo
                } else {
                    CoreEngine::redo
                };
                self.mutate(
                    operation_id,
                    None,
                    false,
                    None,
                    |core| apply(core).map(EditOutcome::Committed),
                    &mut effects,
                );
            }
            Event::RetrySave { operation_id } => self.retry_save(operation_id, &mut effects),
            Event::SelectParts {
                operation_id,
                part_ids,
                range_part_ids,
                mode,
            } => self.select_parts(operation_id, part_ids, range_part_ids, mode, &mut effects),
            Event::SelectMatrixCell {
                operation_id,
                scope,
                matrix_id,
                target_part_id,
                part_ids,
                mode,
            } => self.select_matrix_cell(
                operation_id,
                selection::MatrixCellSelection {
                    scope,
                    matrix_id,
                    target_part_id,
                    part_ids,
                    mode,
                },
                &mut effects,
            ),
            Event::Navigate {
                operation_id,
                board_id,
                instance_id,
            } => {
                if let Some(snapshot) = &self.model.accepted {
                    let (board_ok, instance_ok) = {
                        let document = crate::resolved_document(&snapshot.document);
                        let board_ok = document.boards.iter().any(|board| board.id == board_id);
                        let instance_ok = instance_id.as_ref().is_none_or(|id| {
                            document.hardware.as_ref().is_some_and(|h| {
                                h.instances
                                    .iter()
                                    .any(|i| &i.id == id && i.board_id == board_id)
                            })
                        });
                        (board_ok, instance_ok)
                    };
                    if board_ok && instance_ok {
                        self.model.active_board_id = board_id;
                        self.model.active_instance_id = instance_id;
                        self.invalidate_gesture(&mut effects);
                        self.cancel_job(&mut effects);
                        self.retain_export(None, &mut effects);
                        self.settle(operation_id, TerminalOutcome::Completed, &mut effects);
                    } else {
                        self.reject(
                            operation_id,
                            "board or instance is not in the accepted document",
                            &mut effects,
                        );
                    }
                } else {
                    self.reject(operation_id, "no accepted document is open", &mut effects);
                }
            }
            Event::SetCamera {
                operation_id,
                center,
                zoom,
            } => {
                if center.x.is_finite() && center.y.is_finite() && zoom.is_finite() && zoom > 0.0 {
                    self.model.camera = CameraState { center, zoom };
                    self.settle(operation_id, TerminalOutcome::Completed, &mut effects);
                } else {
                    self.reject(
                        operation_id,
                        "camera values must be finite and zoom positive",
                        &mut effects,
                    );
                }
            }
            Event::GestureBegin {
                operation_id,
                pointer_id,
                target_ids,
                transaction_id,
                start,
                pitch,
                snap_fraction,
                geometry_snap,
                gap,
                pixels_per_world,
                alt,
            } => self.begin_gesture(
                operation_id,
                pointer_id,
                target_ids,
                transaction_id,
                start,
                pitch,
                snap_fraction,
                geometry_snap,
                gap,
                pixels_per_world,
                alt,
                &mut effects,
            ),
            Event::GestureSample {
                pointer_id,
                positions,
                pixels_per_world,
                alt,
            } => self.gesture_sample(
                pointer_id,
                positions,
                pixels_per_world,
                alt,
                false,
                &mut effects,
            ),
            Event::GestureFrame { generation } => self.gesture_frame(generation, &mut effects),
            Event::GestureEnd {
                pointer_id,
                final_positions,
                pixels_per_world,
                alt,
            } => self.gesture_sample(
                pointer_id,
                final_positions,
                pixels_per_world,
                alt,
                true,
                &mut effects,
            ),
            Event::GestureCancel { pointer_id } => {
                if self
                    .gesture
                    .as_ref()
                    .is_some_and(|g| g.pointer_id == pointer_id)
                {
                    self.invalidate_gesture(&mut effects);
                }
            }
            Event::StartGeneration {
                operation_id,
                scope,
            } => self.start_generation_now(operation_id, scope, &mut effects),
            Event::CancelGeneration { operation_id } => {
                self.cancel_job(&mut effects);
                self.settle(operation_id, TerminalOutcome::Completed, &mut effects);
            }
            Event::StartExport {
                operation_id,
                scope,
            } => self.start_export_now(operation_id, scope, &mut effects),
            Event::ExportCommit {
                operation_id,
                export_operation_id,
                token,
                scope,
                commit,
            } => {
                if !self.export_is_current(export_operation_id, token, &scope) {
                    self.settle(operation_id, TerminalOutcome::Superseded, &mut effects);
                } else {
                    let revision = match &commit {
                        ExportCommitRequest::ApplyElectrical { plan, .. }
                        | ExportCommitRequest::ProtectElectricalHandoff { plan } => plan.revision,
                    };
                    self.mutate(
                        operation_id,
                        Some(revision),
                        false,
                        Some(export_operation_id),
                        |core| {
                            match commit {
                                ExportCommitRequest::ApplyElectrical { plan, draft } => {
                                    core.apply_electrical(revision, plan, draft)
                                }
                                ExportCommitRequest::ProtectElectricalHandoff { plan } => {
                                    core.protect_electrical_handoff(revision, scope.board_id, plan)
                                }
                            }
                            .map(EditOutcome::Committed)
                        },
                        &mut effects,
                    );
                }
            }
            Event::Close { operation_id } => self.close(operation_id, &mut effects),
        }
        effects
    }

    pub fn complete(&mut self, completion: Completion) -> Vec<Effect> {
        let mut effects = Vec::new();
        match completion {
            Completion::Persist {
                save_attempt_id,
                result,
            } => self.persist_completed(save_attempt_id, result, &mut effects),
            Completion::GenerationFinished {
                job_id,
                scope,
                exact,
            } => {
                self.finish_generation(
                    job_id,
                    &scope,
                    GenerationStatus::Ready { job_id, exact },
                    TerminalOutcome::Completed,
                    &mut effects,
                );
            }
            Completion::GenerationFailed {
                job_id,
                scope,
                reason,
                blocked,
            } => {
                let (status, outcome) = if blocked {
                    (
                        GenerationStatus::Blocked {
                            job_id,
                            reason: reason.clone(),
                        },
                        TerminalOutcome::Rejected(reason),
                    )
                } else {
                    (
                        GenerationStatus::Failed {
                            job_id,
                            reason: reason.clone(),
                        },
                        TerminalOutcome::ExecutorFailed(reason),
                    )
                };
                self.finish_generation(job_id, &scope, status, outcome, &mut effects);
            }
            Completion::Export {
                operation_id,
                result,
            } => self.complete_export(operation_id, result, &mut effects),
        }
        if self.pending_save.is_none()
            && let Some(operation_id) = self.close_operation.take()
        {
            self.finish_close(operation_id, &mut effects);
        }
        effects
    }
    fn reject(&mut self, operation: OperationId, reason: &str, effects: &mut Vec<Effect>) {
        self.settle(operation, TerminalOutcome::Rejected(reason.into()), effects);
    }

    fn settle(
        &mut self,
        operation_id: OperationId,
        outcome: TerminalOutcome,
        effects: &mut Vec<Effect>,
    ) {
        effects.push(Effect::Settled {
            operation_id,
            outcome,
            landing: None,
        });
    }

    /// Settle a completed operation at the accepted snapshot it just installed, so callers
    /// read exactly where it landed.
    fn settle_landed(
        &mut self,
        operation_id: OperationId,
        landing: Landing,
        effects: &mut Vec<Effect>,
    ) {
        effects.push(Effect::Settled {
            operation_id,
            outcome: TerminalOutcome::Completed,
            landing: Some(landing),
        });
    }
    fn epoch(&self) -> SessionEpoch {
        self.model
            .accepted
            .as_ref()
            .map_or(SessionEpoch(0), |s| s.session_epoch)
    }
}
