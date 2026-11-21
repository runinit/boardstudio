use super::*;

impl Session {
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
                if self.model.lifecycle == Lifecycle::RecoveryRequired {
                    self.settle(
                        operation_id,
                        TerminalOutcome::BlockedByRecovery(
                            "explicit recovery choice required".into(),
                        ),
                        &mut effects,
                    );
                } else {
                    self.queue_open(operation_id, document, &mut effects);
                }
            }
            Event::RecoverWithDocument {
                operation_id,
                document,
            } => {
                if self.model.lifecycle != Lifecycle::RecoveryRequired {
                    self.settle(
                        operation_id,
                        TerminalOutcome::Rejected("session is not awaiting recovery".into()),
                        &mut effects,
                    );
                } else if let Err(reason) = (self.validate_open)(&document) {
                    self.settle(
                        operation_id,
                        TerminalOutcome::Rejected(reason),
                        &mut effects,
                    );
                } else {
                    // The user chose the last known persisted document. Discard the
                    // failed retained candidate so it cannot block the explicit reopen.
                    self.pending_save = None;
                    self.model.durability = self
                        .model
                        .accepted
                        .as_ref()
                        .map(|snapshot| Durability::Saved {
                            revision: snapshot.document.revision,
                        })
                        .unwrap_or(Durability::NoDocument);
                    self.model.last_error = None;
                    self.queue_open(operation_id, document, &mut effects);
                }
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
                self.enqueue(
                    operation_id,
                    IntentKind::Edit(EditCommand {
                        base_revision,
                        transaction_id,
                        phase: EditPhase::Preview,
                        target_ids,
                        operation,
                    }),
                    false,
                    &mut effects,
                )
            }
            Event::ResolveEdit {
                operation_id,
                label,
                resolver,
            } => self.enqueue(
                operation_id,
                IntentKind::ResolveEdit { label, resolver },
                false,
                &mut effects,
            ),
            Event::ClearPreview {
                operation_id,
                token,
                revision,
                board_id,
                transaction_id,
            } => {
                if let Some(active) = self.active_core.as_mut().filter(|active| {
                    active.preview_transaction_id.as_deref() == Some(transaction_id.as_str())
                }) {
                    active.preview_cancelled = true;
                }
                self.enqueue(
                    operation_id,
                    IntentKind::ClearPreview {
                        token,
                        revision,
                        board_id,
                        transaction_id,
                    },
                    false,
                    &mut effects,
                )
            }
            Event::ReviewElectricalRemap {
                operation_id,
                base_revision,
                board_id,
                expected_fingerprint,
            } => self.enqueue(
                operation_id,
                IntentKind::ReviewElectricalRemap {
                    base_revision,
                    board_id,
                    expected_fingerprint,
                },
                true,
                &mut effects,
            ),
            Event::Undo { operation_id } => {
                self.enqueue(operation_id, IntentKind::Undo, false, &mut effects)
            }
            Event::Redo { operation_id } => {
                self.enqueue(operation_id, IntentKind::Redo, false, &mut effects)
            }
            Event::RetrySave { operation_id } => self.retry_save(operation_id, &mut effects),
            Event::SelectParts {
                operation_id,
                part_ids,
                range_part_ids,
                mode,
            } => {
                if let Some(snapshot) = &self.model.accepted {
                    let valid = snapshot
                        .document
                        .parts
                        .iter()
                        .map(|part| part.id.as_str())
                        .collect::<HashSet<_>>();
                    let incoming = part_ids
                        .into_iter()
                        .filter(|id| valid.contains(id.as_str()))
                        .collect::<Vec<_>>();
                    match mode {
                        SelectionMode::Replace => self.model.selected_part_ids = incoming.clone(),
                        SelectionMode::Add => {
                            for id in &incoming {
                                if !self.model.selected_part_ids.contains(id) {
                                    self.model.selected_part_ids.push(id.clone());
                                }
                            }
                        }
                        SelectionMode::Toggle => {
                            for id in &incoming {
                                if let Some(index) = self
                                    .model
                                    .selected_part_ids
                                    .iter()
                                    .position(|current| current == id)
                                {
                                    self.model.selected_part_ids.remove(index);
                                } else {
                                    self.model.selected_part_ids.push(id.clone());
                                }
                            }
                        }
                        SelectionMode::Range => {
                            let valid_order = range_part_ids
                                .iter()
                                .filter(|id| valid.contains(id.as_str()))
                                .cloned()
                                .collect::<Vec<_>>();
                            let anchor =
                                self.model.selection_anchor_id.as_ref().or(incoming.first());
                            if let (Some(anchor), Some(last)) = (anchor, incoming.last())
                                && let (Some(a), Some(b)) = (
                                    valid_order.iter().position(|id| id == anchor),
                                    valid_order.iter().position(|id| id == last),
                                )
                            {
                                self.model.selected_part_ids =
                                    valid_order[a.min(b)..=a.max(b)].to_vec();
                            }
                        }
                    }
                    self.model.selection_mode = mode;
                    if let Some(anchor) = incoming.first() {
                        self.model.selection_anchor_id = Some(anchor.clone());
                    }
                    self.settle(operation_id, TerminalOutcome::Completed, &mut effects);
                } else {
                    self.settle(
                        operation_id,
                        TerminalOutcome::Rejected("no accepted document is open".into()),
                        &mut effects,
                    );
                }
            }
            Event::SelectMatrixCell {
                operation_id,
                scope,
                matrix_id,
                target_part_id,
                part_ids,
                mode,
            } => {
                let Some(snapshot) = self.model.accepted.as_ref() else {
                    self.settle(
                        operation_id,
                        TerminalOutcome::Rejected("no accepted document is open".into()),
                        &mut effects,
                    );
                    return effects;
                };
                let scope_is_current = snapshot.session_epoch == scope.session_epoch
                    && snapshot.document.id == scope.document_id
                    && self.model.active_board_id == scope.board_id
                    && self.model.active_instance_id == scope.instance_id;
                let Some(board) = snapshot
                    .document
                    .boards
                    .iter()
                    .find(|board| board.id == scope.board_id)
                else {
                    self.settle(operation_id, TerminalOutcome::Superseded, &mut effects);
                    return effects;
                };
                let Some(matrix) = snapshot
                    .document
                    .matrices
                    .iter()
                    .find(|matrix| matrix.id == matrix_id)
                else {
                    self.settle(operation_id, TerminalOutcome::Superseded, &mut effects);
                    return effects;
                };
                let scope_is_current = scope_is_current
                    && matrix
                        .board_id
                        .as_ref()
                        .is_none_or(|board_id| board_id == &scope.board_id);
                let board_parts = board
                    .part_ids
                    .iter()
                    .map(String::as_str)
                    .collect::<HashSet<_>>();
                let document_parts = snapshot
                    .document
                    .parts
                    .iter()
                    .map(|part| part.id.as_str())
                    .collect::<HashSet<_>>();
                let Some(scene) = snapshot
                    .scene
                    .matrix_scenes
                    .iter()
                    .find(|scene| scene.matrix_id == matrix_id)
                else {
                    self.settle(operation_id, TerminalOutcome::Superseded, &mut effects);
                    return effects;
                };
                let mut primary_cells = scene
                    .cells
                    .iter()
                    .filter_map(|cell| {
                        let id = cell.member_id.as_deref()?;
                        (cell.enabled
                            && matrix.part_ids.iter().any(|member| member == id)
                            && board_parts.contains(id)
                            && document_parts.contains(id))
                        .then(|| (cell.row, cell.column, id.to_owned()))
                    })
                    .collect::<Vec<_>>();
                primary_cells.sort_by_key(|(row, column, _)| (*row, *column));
                let target_is_live = primary_cells.iter().any(|(_, _, id)| id == &target_part_id);
                if !scope_is_current || !target_is_live {
                    self.settle(operation_id, TerminalOutcome::Superseded, &mut effects);
                    return effects;
                }
                let incoming = part_ids
                    .into_iter()
                    .filter(|id| {
                        board_parts.contains(id.as_str()) && document_parts.contains(id.as_str())
                    })
                    .collect::<Vec<_>>();
                let mut retained_anchor = self.model.selection_anchor_id.clone();
                let mut effective_mode = mode;
                let selected = if mode == SelectionMode::Range {
                    let anchor = retained_anchor.as_deref().and_then(|anchor| {
                        primary_cells
                            .iter()
                            .find(|(_, _, id)| id == anchor)
                            .map(|(row, column, _)| (*row, *column, anchor.to_owned()))
                    });
                    let target = primary_cells
                        .iter()
                        .find(|(_, _, id)| id == &target_part_id)
                        .map(|(row, column, _)| (*row, *column));
                    if let (
                        Some((anchor_row, anchor_column, _)),
                        Some((target_row, target_column)),
                    ) = (anchor, target)
                    {
                        primary_cells
                            .iter()
                            .filter(|(row, column, _)| {
                                *row >= anchor_row.min(target_row)
                                    && *row <= anchor_row.max(target_row)
                                    && *column >= anchor_column.min(target_column)
                                    && *column <= anchor_column.max(target_column)
                            })
                            .map(|(_, _, id)| id.clone())
                            .collect::<Vec<_>>()
                    } else {
                        effective_mode = SelectionMode::Replace;
                        retained_anchor = Some(target_part_id.clone());
                        incoming
                    }
                } else {
                    retained_anchor = Some(target_part_id.clone());
                    incoming
                };
                match effective_mode {
                    SelectionMode::Replace | SelectionMode::Range => {
                        self.model.selected_part_ids = selected;
                    }
                    SelectionMode::Add => {
                        for id in selected {
                            if !self.model.selected_part_ids.contains(&id) {
                                self.model.selected_part_ids.push(id);
                            }
                        }
                    }
                    SelectionMode::Toggle => {
                        for id in selected {
                            if let Some(index) = self
                                .model
                                .selected_part_ids
                                .iter()
                                .position(|current| current == &id)
                            {
                                self.model.selected_part_ids.remove(index);
                            } else {
                                self.model.selected_part_ids.push(id);
                            }
                        }
                    }
                }
                self.model.selection_mode = effective_mode;
                self.model.selection_anchor_id = retained_anchor;
                self.settle(operation_id, TerminalOutcome::Completed, &mut effects);
            }
            Event::Navigate {
                operation_id,
                board_id,
                instance_id,
            } => {
                if let Some(snapshot) = &self.model.accepted {
                    let board_ok = snapshot
                        .document
                        .boards
                        .iter()
                        .any(|board| board.id == board_id);
                    let instance_ok = instance_id.as_ref().is_none_or(|id| {
                        snapshot.document.hardware.as_ref().is_some_and(|h| {
                            h.instances
                                .iter()
                                .any(|i| &i.id == id && i.board_id == board_id)
                        })
                    });
                    if board_ok && instance_ok {
                        self.model.active_board_id = board_id;
                        self.model.active_instance_id = instance_id;
                        self.invalidate_gesture(&mut effects);
                        self.cancel_job(&mut effects);
                        self.cancel_exports(&mut effects);
                        self.settle(operation_id, TerminalOutcome::Completed, &mut effects);
                    } else {
                        self.settle(
                            operation_id,
                            TerminalOutcome::Rejected(
                                "board or instance is not in the accepted document".into(),
                            ),
                            &mut effects,
                        );
                    }
                } else {
                    self.settle(
                        operation_id,
                        TerminalOutcome::Rejected("no accepted document is open".into()),
                        &mut effects,
                    );
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
                    self.settle(
                        operation_id,
                        TerminalOutcome::Rejected(
                            "camera values must be finite and zoom positive".into(),
                        ),
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
            } => {
                if self.gesture.is_none() && self.model.lifecycle == Lifecycle::Ready {
                    let generation = self.next_gesture;
                    self.next_gesture += 1;
                    let base_revision = self
                        .model
                        .accepted
                        .as_ref()
                        .map_or(0, |s| s.document.revision);
                    self.gesture = Some(Gesture {
                        operation_id,
                        pointer_id,
                        generation,
                        transaction_id,
                        base_revision,
                        targets: target_ids,
                        start: start.clone(),
                        latest: start,
                        sent_positions: None,
                        commit_queued: false,
                        pitch,
                        snap_fraction,
                        geometry_snap,
                        gap,
                        pixels_per_world,
                        snap_state: MagneticSnapState::default(),
                        alt,
                        frame_pending: false,
                        changed: false,
                        preview_in_flight: false,
                        final_positions: None,
                    });
                    self.model.gesture = self.gesture.as_ref().map(gesture_view);
                    effects.push(Effect::CapturePointer { pointer_id });
                }
            }
            Event::GestureSample {
                pointer_id,
                positions,
                pixels_per_world,
                alt,
            } => {
                if let Some(pixels_per_world) = pixels_per_world
                    && let Some(gesture) = self
                        .gesture
                        .as_mut()
                        .filter(|gesture| gesture.pointer_id == pointer_id)
                {
                    gesture.pixels_per_world = Some(pixels_per_world);
                }
                self.gesture_sample(pointer_id, positions, alt, false, &mut effects)
            }
            Event::GestureFrame { generation } => self.gesture_frame(generation, &mut effects),
            Event::GestureEnd {
                pointer_id,
                final_positions,
                pixels_per_world,
                alt,
            } => {
                if let Some(pixels_per_world) = pixels_per_world
                    && let Some(gesture) = self
                        .gesture
                        .as_mut()
                        .filter(|gesture| gesture.pointer_id == pointer_id)
                {
                    gesture.pixels_per_world = Some(pixels_per_world);
                }
                self.gesture_sample(pointer_id, final_positions, alt, true, &mut effects)
            }
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
            } => self.enqueue_aux(operation_id, IntentKind::Generate(scope), &mut effects),
            Event::CancelGeneration { operation_id } => {
                self.cancel_job(&mut effects);
                self.settle(operation_id, TerminalOutcome::Completed, &mut effects);
            }
            Event::StartExport {
                operation_id,
                scope,
            } => self.enqueue_aux(operation_id, IntentKind::Export(scope), &mut effects),
            Event::ExportCommit {
                operation_id,
                export_operation_id,
                token,
                scope,
                commit,
            } => {
                let owner = ExportCommitOwner {
                    operation_id: export_operation_id,
                    token,
                    scope,
                };
                if !self.export_is_current(owner.operation_id, owner.token, &owner.scope) {
                    self.settle(operation_id, TerminalOutcome::Superseded, &mut effects);
                } else {
                    self.enqueue(
                        operation_id,
                        IntentKind::ExportCommit { owner, commit },
                        true,
                        &mut effects,
                    );
                }
            }
            Event::Close { operation_id } => self.close(operation_id, &mut effects),
        }
        if self.active_core.is_none()
            && self.pending_save.is_none()
            && self.model.lifecycle != Lifecycle::RecoveryRequired
        {
            self.pump(&mut effects);
        }
        effects
    }

    pub fn complete(&mut self, completion: Completion) -> Vec<Effect> {
        let mut effects = Vec::new();
        match completion {
            Completion::Core {
                request_id,
                executor_epoch,
                reply,
            } => self.core_completed(request_id, executor_epoch, *reply, &mut effects),
            Completion::Persist {
                save_attempt_id,
                result,
            } => self.persist_completed(save_attempt_id, result, &mut effects),
            Completion::CoreFailed {
                request_id,
                executor_epoch,
                reason,
            } => self.core_failed(request_id, executor_epoch, reason, &mut effects),
            Completion::ExecutorRestarted { executor_epoch } => {
                if executor_epoch == self.core_epoch {
                    self.core_epoch = executor_epoch;
                }
            }
            Completion::GenerationProgress {
                job_id,
                scope,
                message: _,
            } => {
                if self.job_is_current(job_id, &scope) {
                    self.model.generation = GenerationStatus::Running { job_id };
                }
            }
            Completion::GenerationFinished {
                job_id,
                scope,
                exact,
            } => {
                if self.job_is_current(job_id, &scope) {
                    self.model.generation = GenerationStatus::Ready { job_id, exact };
                    if let Some((_, operation, _)) = self.active_job.take() {
                        self.settle(operation, TerminalOutcome::Completed, &mut effects);
                    }
                }
            }
            Completion::GenerationFailed {
                job_id,
                scope,
                reason,
            } => {
                if self.job_is_current(job_id, &scope) {
                    self.model.generation = GenerationStatus::Failed {
                        job_id,
                        reason: reason.clone(),
                    };
                    self.model.last_error = Some(reason.clone());
                    if let Some((_, operation, _)) = self.active_job.take() {
                        self.settle(
                            operation,
                            TerminalOutcome::ExecutorFailed(reason),
                            &mut effects,
                        );
                    }
                }
            }
            Completion::GenerationBlocked {
                job_id,
                scope,
                reason,
            } => {
                if self.job_is_current(job_id, &scope) {
                    self.model.generation = GenerationStatus::Blocked {
                        job_id,
                        reason: reason.clone(),
                    };
                    self.model.last_error = Some(reason.clone());
                    if let Some((_, operation, _)) = self.active_job.take() {
                        self.settle(operation, TerminalOutcome::Rejected(reason), &mut effects);
                    }
                }
            }
            Completion::ExportFinished {
                operation_id,
                token,
                scope,
                artifact_id,
            } => {
                let valid = self
                    .exports
                    .iter()
                    .any(|(op, expected_scope, expected_token)| {
                        *op == operation_id && *expected_scope == scope && *expected_token == token
                    })
                    && self.snapshot_matches(token, &scope);
                self.exports.retain(|(op, _, _)| *op != operation_id);
                if valid {
                    effects.push(Effect::DeliverExport {
                        operation_id,
                        artifact_id,
                        token,
                    });
                    self.settle(operation_id, TerminalOutcome::Completed, &mut effects);
                } else {
                    self.settle(operation_id, TerminalOutcome::Cancelled, &mut effects);
                }
            }
            Completion::ExportFailed {
                operation_id,
                reason,
            } => {
                self.exports
                    .retain(|(registered_operation, _, _)| *registered_operation != operation_id);
                self.settle(
                    operation_id,
                    TerminalOutcome::ExecutorFailed(reason),
                    &mut effects,
                );
            }
        }
        if self.active_core.is_none()
            && self.pending_save.is_none()
            && self.model.lifecycle != Lifecycle::RecoveryRequired
        {
            if let Some(operation_id) = self.close_operation.take() {
                self.finish_close(operation_id, &mut effects);
            } else {
                self.pump(&mut effects);
            }
        }
        effects
    }
}
