//! Session event submission and effect execution.
use super::*;
use boardstudio_application::{EditResolver, SnapshotToken};

fn observed_outcome(effects: &[Effect], observe: Option<OperationId>) -> Option<TerminalOutcome> {
    effects.iter().find_map(|effect| match effect {
        Effect::Settled {
            operation_id,
            outcome,
            ..
        } if observe == Some(*operation_id) => Some(outcome.clone()),
        _ => None,
    })
}

impl Runtime {
    pub fn submit(self: &Rc<Self>, event: Event) {
        self.submit_event(event, None);
    }

    /// Resolve against the current accepted snapshot and return the in-memory result.
    pub fn commit(
        self: &Rc<Self>,
        label: &str,
        resolver: EditResolver,
    ) -> Result<AcceptedSnapshot, String> {
        let operation_id = self.operation();
        let result = self.submit_event(
            Event::ResolveEdit {
                operation_id,
                label: label.into(),
                resolver,
            },
            Some(operation_id),
        );
        match result {
            Some(TerminalOutcome::Completed) => self
                .model()
                .accepted
                .ok_or_else(|| "No accepted keyboard after the edit.".into()),
            Some(
                TerminalOutcome::Rejected(reason)
                | TerminalOutcome::ExecutorFailed(reason)
                | TerminalOutcome::BlockedByRecovery(reason),
            ) => Err(reason),
            Some(outcome) => Err(format!("The change could not be applied: {outcome:?}")),
            None => Err("The change did not complete synchronously.".into()),
        }
    }

    pub(super) fn submit_event(
        self: &Rc<Self>,
        event: Event,
        observe: Option<OperationId>,
    ) -> Option<TerminalOutcome> {
        if self.project_deletion_pending.get() {
            let open_operation = match &event {
                Event::Open { operation_id, .. }
                | Event::RecoverWithDocument { operation_id, .. } => Some(*operation_id),
                _ => None,
            };
            if let Some(operation_id) = open_operation {
                if self.project_deletion_open.get() == Some(operation_id) {
                    self.project_deletion_open.set(None);
                } else {
                    self.report("Wait for the saved keyboard deletion to finish before opening another project.");
                    return None;
                }
            }
        }
        if matches!(&event, Event::StartGeneration { .. })
            && self.mechanical_mount_initialization_pending()
        {
            self.report(
                "Finish preparing mounting locations before generating the mechanical assembly.",
            );
            return None;
        }
        let previous_scope = self.scope();
        let previous_snapshot = self.accepted_identity();
        let effects = self.session.borrow_mut().submit(event);
        let scope_changed = self.scope() != previous_scope;
        if scope_changed {
            self.operation_outcomes.retire_edit_scope();
        }
        self.invalidate_stale_native_case_preview();
        if scope_changed || self.accepted_identity() != previous_snapshot {
            self.cancel_collision_checks();
            self.cancel_active_case_gesture_preview();
            self.cancel_keycaps_cad_preview();
        }
        if scope_changed {
            self.cad_scene.borrow_mut().take();
            if let Some((_, worker)) = self.cad_worker.borrow_mut().take() {
                worker.close();
            }
        }
        self.changed();
        #[cfg(any(test, feature = "test-support"))]
        if self.in_process_adapters.borrow().is_some() {
            let result = observed_outcome(&effects, observe);
            self.held_effects.borrow_mut().extend(effects);
            return result;
        }
        self.drive_effects(effects, observe)
    }

    #[cfg(any(test, feature = "test-support"))]
    pub fn set_case_gesture_preview_executor_test(
        &self,
        executor: Rc<CaseGesturePreviewTestExecutor>,
    ) {
        *self.case_gesture_preview_test_executor.borrow_mut() = Some(executor);
    }

    #[cfg(any(test, feature = "test-support"))]
    pub fn case_gesture_preview_active_test(&self) -> bool {
        self.case_gesture_preview.borrow().active_owner().is_some()
    }

    #[cfg(any(test, feature = "test-support"))]
    pub fn set_cad_scene_test(&self, scene: Option<Rc<CadScene>>) {
        *self.cad_scene.borrow_mut() = scene;
    }

    #[cfg(any(test, feature = "test-support"))]
    pub fn set_keycaps_preview_executor_test(&self, executor: Rc<KeycapsPreviewTestExecutor>) {
        *self.keycaps_preview_test_executor.borrow_mut() = Some(executor);
    }

    #[cfg(any(test, feature = "test-support"))]
    pub fn install_case_pcb_preview_test(&self) {
        let accepted = self.model().accepted.expect("accepted preview fixture");
        let scope = self.scope().expect("selected physical fixture scope");
        let generation = self.native_case_preview.borrow().generation + 1;
        let capture = crate::case_preview::capture_native_preview(
            &accepted,
            &scope,
            generation,
            generation,
            format!("case-keycaps-test-{generation}"),
        )
        .expect("valid accepted Case preview input");
        self.native_case_preview.borrow_mut().generation = generation;
        self.set_native_preview_pending(capture.owner.clone(), capture.lease.clone());
        let preview = crate::case_preview::accept_native_preview(
            capture,
            PcbPreview {
                revision: accepted.document.revision,
                thickness: 1.6,
                contours: vec![],
                surfaces: vec![],
                holes: vec![],
                models: vec![],
                diagnostics: vec![],
            },
        )
        .expect("controlled PCB payload preserves the accepted preview owner");
        self.native_case_preview
            .borrow_mut()
            .publish(preview)
            .unwrap();
        self.changed();
    }

    pub(super) fn complete(self: &Rc<Self>, event: Completion) -> Vec<Effect> {
        let previous_scope = self.scope();
        let previous_snapshot = self.accepted_identity();
        let effects = self.session.borrow_mut().complete(event);
        let scope_changed = self.scope() != previous_scope;
        if scope_changed {
            self.operation_outcomes.retire_edit_scope();
        }
        self.invalidate_stale_native_case_preview();
        if scope_changed || self.accepted_identity() != previous_snapshot {
            self.cancel_collision_checks();
            self.cancel_active_case_gesture_preview();
            self.cancel_keycaps_cad_preview();
        }
        self.changed();
        effects
    }

    fn accepted_identity(&self) -> Option<(SnapshotToken, u64)> {
        self.session
            .borrow()
            .read_model()
            .accepted
            .as_ref()
            .map(|snapshot| (snapshot.token, snapshot.document.revision))
    }

    #[cfg(any(test, feature = "test-support"))]
    pub(super) fn drive(self: &Rc<Self>, effects: Vec<Effect>) {
        self.drive_effects(effects, None);
    }

    fn drive_effects(
        self: &Rc<Self>,
        effects: Vec<Effect>,
        observe: Option<OperationId>,
    ) -> Option<TerminalOutcome> {
        let this = self.clone();
        let result = observed_outcome(&effects, observe);
        let mut effects = VecDeque::from(effects);
        #[cfg(any(test, feature = "test-support"))]
        let finished = self.in_process_adapters.borrow().as_ref().map(|_| {
            let (finished, receive) = futures_channel::oneshot::channel();
            self.in_process_drives.borrow_mut().push(receive);
            finished
        });
        spawn_local(async move {
            while let Some(effect) = effects.pop_front() {
                effects.extend(this.run(effect).await);
            }
            if this.model().lifecycle == Lifecycle::Closed {
                this.cancel_frames();
                this.cancel_keycaps_cad_preview();
                if let Some((_, worker)) = this.cad_worker.borrow_mut().take() {
                    worker.close();
                }
                for (_, worker) in std::mem::take(&mut *this.export_workers.borrow_mut()) {
                    worker.close();
                }
            }
            #[cfg(any(test, feature = "test-support"))]
            if let Some(finished) = finished {
                let _ = finished.send(());
            }
        });
        result
    }

    pub(super) async fn run(self: &Rc<Self>, effect: Effect) -> Vec<Effect> {
        match effect {
            Effect::Persist {
                document,
                save_attempt_id,
                ..
            } => {
                let assets = self
                    .assets
                    .borrow()
                    .iter()
                    .filter(|(hash, _)| document.assets.iter().any(|a| &a.sha256 == *hash))
                    .map(|(hash, bytes)| (hash.clone(), bytes.clone()))
                    .collect();
                let persistence = self.persistence.borrow().clone();
                let expected = self.saved_tokens.borrow().get(&document.id).cloned();
                let result = match persistence
                    .save_document(&document, &assets, expected.as_ref())
                    .await
                {
                    Ok(token) => {
                        self.saved_tokens
                            .borrow_mut()
                            .insert(document.id.clone(), token);
                        for asset in &document.assets {
                            self.assets.borrow_mut().remove(&asset.sha256);
                        }
                        SaveResult::Committed
                    }
                    Err(error) => SaveResult::Aborted(error.to_string()),
                };
                let persisted = result == SaveResult::Committed;
                let effects = self.complete(Completion::Persist {
                    save_attempt_id,
                    result,
                });
                match self.model().durability {
                    Durability::Saved { .. } if persisted => {
                        self.apply_report(RuntimeReport::status("Saved locally.").transient());
                    }
                    Durability::Failed { reason, .. } => {
                        self.report(reason);
                    }
                    _ => {}
                }
                effects
            }
            Effect::Settled {
                operation_id,
                outcome,
                landing,
            } => {
                let observed = self.operation_outcomes.settle_with_landing(
                    operation_id,
                    outcome.clone(),
                    landing,
                );
                let lease = self.export_leases.borrow_mut().remove(&operation_id);
                let kind = lease.as_ref().map(|lease| lease.kind().clone());
                let is_board_outline_export =
                    matches!(kind.as_ref(), Some(ExportKind::BoardOutline { .. }));
                let firmware_capture = (kind == Some(ExportKind::Firmware))
                    .then_some(lease.as_ref())
                    .flatten();
                let is_firmware_export = firmware_capture.is_some();
                let delivery_error = self
                    .export_delivery_errors
                    .borrow_mut()
                    .remove(&operation_id);
                let firmware_owner_is_current = firmware_capture.is_some_and(|lease| {
                    self.export_source_is_current(lease)
                        && self.latest_firmware_export.get() == Some(operation_id)
                });
                if is_firmware_export {
                    if let Some(report) =
                        export_terminal_report(firmware_owner_is_current, outcome, delivery_error)
                    {
                        self.apply_report(report);
                    }
                } else if is_board_outline_export {
                    let lease_is_current = lease
                        .as_ref()
                        .is_some_and(|lease| self.export_source_is_current(lease));
                    if lease_is_current {
                        match outcome {
                            TerminalOutcome::Completed => {}
                            TerminalOutcome::Rejected(reason)
                            | TerminalOutcome::ExecutorFailed(reason) => {
                                self.apply_report(RuntimeReport::alert(reason).transient());
                            }
                            TerminalOutcome::PersistenceFailed(reason)
                            | TerminalOutcome::BlockedByRecovery(reason) => {
                                self.apply_report(RuntimeReport::alert(reason));
                            }
                            TerminalOutcome::Cancelled => self.report("Cancelled."),
                            TerminalOutcome::Closed => self.report("Editor closed."),
                            TerminalOutcome::Superseded => {
                                if observed {
                                    self.changed();
                                }
                            }
                        }
                    }
                } else if let Some(lease) = lease {
                    let downloaded =
                        outcome == TerminalOutcome::Completed && delivery_error.is_none();
                    if let Some(mut report) = export_terminal_report(
                        self.export_source_is_current(&lease),
                        outcome,
                        delivery_error,
                    ) {
                        if downloaded {
                            report.message =
                                if matches!(kind, Some(ExportKind::ProjectCopy { .. })) {
                                    "Project copy downloaded."
                                } else {
                                    "Export finished."
                                }
                                .into();
                        }
                        self.apply_report(report);
                    }
                } else {
                    match outcome {
                        TerminalOutcome::Completed => {}
                        TerminalOutcome::Rejected(reason)
                        | TerminalOutcome::PersistenceFailed(reason)
                        | TerminalOutcome::BlockedByRecovery(reason)
                        | TerminalOutcome::ExecutorFailed(reason) => self.report(reason),
                        TerminalOutcome::Cancelled => {
                            self.apply_report(RuntimeReport::status("Cancelled.").transient());
                        }
                        TerminalOutcome::Closed => self.report("Editor closed."),
                        TerminalOutcome::Superseded => {
                            if observed {
                                self.changed();
                            }
                        }
                    }
                }
                vec![]
            }
            Effect::CapturePointer { pointer_id } => {
                if let Some(svg) = self.surface.borrow().as_ref()
                    && let Ok(pointer) = i32::try_from(pointer_id)
                {
                    let result = svg.set_pointer_capture(pointer);
                    if let Err(error) = result {
                        self.report(format!("Pointer capture failed: {error:?}"));
                    }
                }
                vec![]
            }
            Effect::ReleasePointer { pointer_id } => {
                if let Some(svg) = self.surface.borrow().as_ref()
                    && let Ok(pointer) = i32::try_from(pointer_id)
                    && svg.has_pointer_capture(pointer)
                {
                    let _ = svg.release_pointer_capture(pointer);
                }
                vec![]
            }
            Effect::RequestFrame { generation } => {
                if !self.frames.borrow().contains_key(&generation) {
                    let weak = Rc::downgrade(self);
                    let callback = Closure::<dyn FnMut(f64)>::new(move |_| {
                        if let Some(this) = weak.upgrade() {
                            this.submit(Event::GestureFrame { generation });
                            // Drop the executing callback after the browser returns from it.
                            spawn_local(async move {
                                this.frames.borrow_mut().remove(&generation);
                            });
                        }
                    });
                    match web_sys::window()
                        .ok_or_else(|| "window unavailable".to_owned())
                        .and_then(|window| {
                            window
                                .request_animation_frame(callback.as_ref().unchecked_ref())
                                .map_err(|e| format!("{e:?}"))
                        }) {
                        Ok(handle) => {
                            self.frames
                                .borrow_mut()
                                .insert(generation, (handle, callback));
                        }
                        Err(error) => self.report(error),
                    }
                }
                vec![]
            }
            Effect::CancelFrame { generation } => {
                if let Some((handle, _)) = self.frames.borrow_mut().remove(&generation)
                    && let Some(window) = web_sys::window()
                {
                    let _ = window.cancel_animation_frame(handle);
                }
                vec![]
            }
            Effect::RunExport {
                operation_id,
                scope,
                snapshot,
            } => {
                if self.export_current(operation_id, snapshot.token, &scope) {
                    self.apply_report(RuntimeReport::status("Export started.").transient());
                }
                let kind = self
                    .export_leases
                    .borrow()
                    .get(&operation_id)
                    .map(|lease| lease.kind().clone());
                let is_board_outline_export = matches!(kind, Some(ExportKind::BoardOutline { .. }));
                let result = match kind.as_ref() {
                    Some(ExportKind::BoardOutline { dxf }) => {
                        self.board_outline_bytes(operation_id, &snapshot, &scope, *dxf)
                    }
                    Some(ExportKind::PcbHandoff { draft }) => {
                        self.pcb_handoff_bytes(operation_id, &snapshot, &scope, *draft)
                            .await
                    }
                    Some(kind) => {
                        let bytes = match kind {
                            ExportKind::Step => {
                                self.step_bytes(operation_id, &snapshot, &scope).await
                            }
                            ExportKind::KeycapsStl => {
                                self.keycaps_stl_bytes(operation_id, &snapshot, &scope)
                                    .await
                            }
                            ExportKind::Firmware => {
                                self.firmware_bytes(operation_id, &snapshot, &scope)
                            }
                            ExportKind::Footprints => {
                                self.footprint_export_bytes(operation_id, &snapshot, &scope)
                                    .await
                            }
                            ExportKind::Mechanical { .. } => {
                                self.mechanical_package_bytes(operation_id, &snapshot, &scope)
                                    .await
                            }
                            ExportKind::ProjectCopy { embed_used_models } => {
                                self.pack_archive(
                                    operation_id,
                                    &snapshot,
                                    &scope,
                                    *embed_used_models,
                                )
                                .await
                            }
                            _ => unreachable!(),
                        };
                        bytes.map(|bytes| (bytes, snapshot.token))
                    }
                    None => Err("Export is no longer active.".into()),
                };
                let result = result.and_then(|(bytes, token)| {
                    self.export_current(operation_id, token, &scope)
                        .then_some((bytes, token))
                        .ok_or_else(|| "Export scope changed before delivery.".to_owned())
                });
                match result {
                    Ok((bytes, delivery_token)) => {
                        let artifact_id = format!("archive-{}", operation_id.0);
                        let name = &snapshot.document.name;
                        let (filename, media_type) =
                            match kind.expect("successful export has a kind") {
                                ExportKind::Footprints => {
                                    (format!("{name}-footprints.zip"), Some("application/zip"))
                                }
                                ExportKind::Firmware => {
                                    (format!("{name}-zmk.zip"), Some("application/zip"))
                                }
                                ExportKind::KeycapsStl => {
                                    (format!("{name}-keycaps.zip"), Some("application/zip"))
                                }
                                ExportKind::Step => (
                                    authored_case_filename(&snapshot.document, &scope),
                                    Some("model/step"),
                                ),
                                ExportKind::PcbHandoff { draft } => {
                                    let suffix = if draft {
                                        "draft-pcb-handoff"
                                    } else {
                                        "pcb-handoff"
                                    };
                                    (format!("{name}-{suffix}.zip"), Some("application/zip"))
                                }
                                ExportKind::Mechanical { filename } => {
                                    (filename, Some("application/zip"))
                                }
                                ExportKind::BoardOutline { dxf: true } => {
                                    (format!("{name}.dxf"), Some("application/dxf"))
                                }
                                ExportKind::BoardOutline { dxf: false } => {
                                    (format!("{name}.svg"), Some("image/svg+xml"))
                                }
                                ExportKind::ProjectCopy { .. } => {
                                    (crate::archive_export::archive_filename(name), None)
                                }
                            };
                        self.artifacts.borrow_mut().insert(
                            artifact_id.clone(),
                            Artifact {
                                operation_id,
                                bytes,
                                filename,
                                media_type: media_type.map(str::to_owned),
                                scope: scope.clone(),
                                token: delivery_token,
                                board_outline: is_board_outline_export,
                            },
                        );
                        self.complete(Completion::ExportFinished {
                            operation_id,
                            token: delivery_token,
                            scope: scope.clone(),
                            artifact_id,
                        })
                    }
                    Err(reason) => self.complete(Completion::ExportFailed {
                        operation_id,
                        reason,
                    }),
                }
            }
            Effect::DeliverExport {
                artifact_id, token, ..
            } => {
                let artifact = self.artifacts.borrow_mut().remove(&artifact_id);
                if let Some(artifact) = artifact
                    && token == artifact.token
                    && self.export_delivery_is_current(
                        artifact.operation_id,
                        token,
                        &artifact.scope,
                    )
                {
                    match self.deliver_artifact(&artifact) {
                        Ok(()) if artifact.board_outline => {
                            self.apply_report(
                                RuntimeReport::status(format!("Saved {}.", artifact.filename))
                                    .transient(),
                            );
                        }
                        Ok(()) => {}
                        Err(error) if artifact.board_outline => {
                            self.apply_report(RuntimeReport::alert(error).transient());
                        }
                        Err(error) => {
                            self.export_delivery_errors
                                .borrow_mut()
                                .insert(artifact.operation_id, error);
                        }
                    }
                }
                vec![]
            }
            Effect::RunGeneration {
                job_id,
                scope,
                snapshot,
                ..
            } => self.generate(job_id, scope, snapshot).await,
            Effect::CancelJob { job_id } => {
                if let Some(cancelled) = self.cad_jobs.borrow_mut().remove(&job_id) {
                    cancelled.set(true);
                }
                if let Some((_, worker)) = self.cad_worker.borrow().as_ref() {
                    // Retain valid same-scope cache updates; the worker yields between bodies.
                    // Scope replacement and close still terminate the departing worker.
                    let _ = worker.cancel(&format!("case-{}", job_id.0));
                }
                vec![]
            }
            Effect::CancelExport { operation_id } => {
                self.export_leases.borrow_mut().remove(&operation_id);
                self.export_delivery_errors
                    .borrow_mut()
                    .remove(&operation_id);
                if let Some(worker) = self.export_workers.borrow_mut().remove(&operation_id) {
                    worker.close();
                }
                self.artifacts
                    .borrow_mut()
                    .remove(&format!("archive-{}", operation_id.0));
                vec![]
            }
        }
    }
}
