//! Session event submission and effect execution.
use super::*;
use boardstudio_application::{ExecutorEpoch, RequestId};
use boardstudio_model::model::EditPhase;

impl Runtime {
    pub fn submit(self: &Rc<Self>, event: Event) {
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
                    return;
                }
            }
        }
        if matches!(&event, Event::StartGeneration { .. })
            && self.mechanical_mount_initialization_pending()
        {
            self.report(
                "Finish preparing mounting locations before generating the mechanical assembly.",
            );
            return;
        }
        let previous_scope = self.scope();
        let previous_snapshot = self
            .model()
            .accepted
            .as_ref()
            .map(|snapshot| (snapshot.token, snapshot.document.revision));
        let effects = self.session.borrow_mut().submit(event);
        self.invalidate_stale_native_case_preview();
        let accepted_snapshot = self
            .model()
            .accepted
            .as_ref()
            .map(|snapshot| (snapshot.token, snapshot.document.revision));
        if self.scope() != previous_scope || accepted_snapshot != previous_snapshot {
            self.cancel_active_case_gesture_preview();
            self.cancel_keycaps_cad_preview();
        }
        if self.scope() != previous_scope {
            self.cad_scene.borrow_mut().take();
            if let Some((_, worker)) = self.cad_worker.borrow_mut().take() {
                worker.close();
            }
        }
        self.changed();
        #[cfg(any(test, feature = "test-support"))]
        if self.in_process_adapters.borrow().is_some() {
            self.held_effects.borrow_mut().extend(effects);
            return;
        }
        self.drive(effects);
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
        let core = self.core.borrow().clone();
        let capture = crate::case_preview::capture_native_preview(
            &accepted,
            &scope,
            generation,
            generation,
            self.session.borrow().core_executor_epoch().0,
            core_executor_identity(&core),
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
        #[cfg(test)]
        let profile = match &event {
            Completion::Core { .. } => super::document_core_tests::profile::Timer::start(
                super::document_core_tests::profile::Phase::PrepareSave,
            ),
            Completion::Persist { .. } => super::document_core_tests::profile::Timer::start(
                super::document_core_tests::profile::Phase::PublishSnapshot,
            ),
            _ => None,
        };
        let previous_scope = self.scope();
        let previous_snapshot = self
            .model()
            .accepted
            .as_ref()
            .map(|snapshot| (snapshot.token, snapshot.document.revision));
        let effects = self.session.borrow_mut().complete(event);
        self.invalidate_stale_native_case_preview();
        let accepted_snapshot = self
            .model()
            .accepted
            .as_ref()
            .map(|snapshot| (snapshot.token, snapshot.document.revision));
        if self.scope() != previous_scope || accepted_snapshot != previous_snapshot {
            self.cancel_active_case_gesture_preview();
            self.cancel_keycaps_cad_preview();
        }
        self.changed();
        #[cfg(test)]
        drop(profile);
        effects
    }
    pub(super) fn drive(self: &Rc<Self>, effects: Vec<Effect>) {
        let this = self.clone();
        let mut effects = VecDeque::from(effects);
        // An unblocked commit can finish its Core request on the submit stack.
        // Honor preceding effects, including pointer release and job cancellation,
        // and leave persistence and terminal delivery with the effect driver.
        if let Some(document_core) = &self.document_core
            && matches!(
                effects.front(),
                Some(Effect::Core { request, .. })
                    if matches!(request.as_ref(), CoreRequest::Edit { command, .. }
                        if command.phase == EditPhase::Commit)
            )
            && let Some(Effect::Core {
                request_id,
                executor_epoch,
                request,
                ..
            }) = effects.pop_front()
        {
            effects.extend(self.run_document_core(
                document_core,
                request_id,
                executor_epoch,
                *request,
            ));
        }
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
                this.core.borrow().close();
            }
            #[cfg(any(test, feature = "test-support"))]
            if let Some(finished) = finished {
                let _ = finished.send(());
            }
        });
    }

    fn run_document_core(
        self: &Rc<Self>,
        document_core: &RefCell<DocumentCore>,
        request_id: RequestId,
        executor_epoch: ExecutorEpoch,
        request: CoreRequest,
    ) -> Vec<Effect> {
        if !self
            .session
            .borrow()
            .core_request_is_current(request_id, executor_epoch)
        {
            return vec![];
        }
        let reply = (document_core.borrow_mut())(request);
        self.complete(Completion::Core {
            request_id,
            executor_epoch,
            reply: Box::new(reply),
        })
    }

    pub(super) async fn run(self: &Rc<Self>, effect: Effect) -> Vec<Effect> {
        match effect {
            Effect::Core {
                request_id,
                executor_epoch,
                request,
                ..
            } => {
                if let Some(document_core) = &self.document_core {
                    return self.run_document_core(
                        document_core,
                        request_id,
                        executor_epoch,
                        *request,
                    );
                }
                let core = self.core.borrow().clone();
                match core
                    .request(
                        &format!("m1-{}", request_id.0),
                        &executor_epoch.0.to_string(),
                        &request,
                    )
                    .await
                {
                    Ok(reply) => self.complete(Completion::Core {
                        request_id,
                        executor_epoch,
                        reply: Box::new(reply),
                    }),
                    Err(error) => self.complete(Completion::CoreFailed {
                        request_id,
                        executor_epoch,
                        reason: error.to_string(),
                    }),
                }
            }
            Effect::RestartCoreExecutor { executor_epoch } => {
                self.core.borrow().close();
                let create_executor = self.core_executor_factory.borrow().clone();
                match create_executor() {
                    Ok(core) => {
                        *self.core.borrow_mut() = core.clone();
                        match core.ready().await {
                            Ok(()) => {
                                self.complete(Completion::ExecutorRestarted { executor_epoch })
                            }
                            Err(error) => {
                                self.report(format!("Executor restart failed: {error}"));
                                vec![]
                            }
                        }
                    }
                    Err(error) => {
                        self.report(error);
                        vec![]
                    }
                }
            }
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
                *self.pending_document.borrow_mut() = Some(document.clone());
                let expected = self.saved_tokens.borrow().get(&document.id).cloned();
                let result = match persistence
                    .save_document(&document, &assets, expected.as_ref())
                    .await
                {
                    Ok(token) => {
                        self.saved_tokens
                            .borrow_mut()
                            .insert(document.id.clone(), token);
                        self.pending_document.borrow_mut().take();
                        self.save_conflict.set(false);
                        for asset in &document.assets {
                            self.assets.borrow_mut().remove(&asset.sha256);
                        }
                        SaveResult::Committed
                    }
                    Err(error) => {
                        self.save_conflict.set(error == SaveError::Conflict);
                        SaveResult::Aborted(error.to_string())
                    }
                };
                self.complete(Completion::Persist {
                    save_attempt_id,
                    result,
                })
            }
            Effect::Settled {
                operation_id,
                outcome,
                landing,
            } => {
                #[cfg(test)]
                let _profile = super::document_core_tests::profile::Timer::start(
                    super::document_core_tests::profile::Phase::DeliverSettlement,
                );
                let observed = self.operation_outcomes.settle_with_landing(
                    operation_id,
                    outcome.clone(),
                    landing,
                );
                let lease = self.export_leases.borrow_mut().finish(operation_id);
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
                self.archive_export_options.settle(operation_id);
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
                            report.message = if kind == Some(ExportKind::ProjectCopy) {
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
                        TerminalOutcome::Completed => {
                            self.apply_report(RuntimeReport::status("Saved locally.").transient());
                        }
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
                    .get(operation_id)
                    .map(|lease| lease.kind().clone());
                let is_step_export = matches!(kind.as_ref(), Some(ExportKind::Step));
                let is_keycaps_export = kind == Some(ExportKind::KeycapsStl);
                let is_firmware_export = kind == Some(ExportKind::Firmware);
                let is_footprint_export = kind == Some(ExportKind::Footprints);
                let is_pcb_handoff_export =
                    matches!(kind.as_ref(), Some(ExportKind::PcbHandoff { .. }));
                let is_mechanical_export =
                    matches!(kind.as_ref(), Some(ExportKind::Mechanical { .. }));
                let is_board_outline_export =
                    matches!(kind.as_ref(), Some(ExportKind::BoardOutline { .. }));
                let result = if let Some(ExportKind::BoardOutline { dxf }) = kind.as_ref() {
                    self.board_outline_bytes(operation_id, &snapshot, &scope, *dxf)
                        .await
                } else if is_keycaps_export {
                    self.keycaps_stl_bytes(operation_id, &snapshot, &scope)
                        .await
                        .map(|bytes| (bytes, snapshot.token))
                } else if is_pcb_handoff_export {
                    let draft =
                        matches!(kind.as_ref(), Some(ExportKind::PcbHandoff { draft: true }));
                    self.pcb_handoff_bytes(operation_id, &snapshot, &scope, draft)
                        .await
                } else if is_mechanical_export {
                    self.mechanical_package_bytes(operation_id, &snapshot, &scope)
                        .await
                        .map(|bytes| (bytes, snapshot.token))
                } else if is_footprint_export {
                    self.footprint_export_bytes(operation_id, &snapshot, &scope)
                        .await
                        .map(|bytes| (bytes, snapshot.token))
                } else {
                    let work: Result<ArchiveWorkFuture<'_>, String> =
                        self.archive_export_options.dispatch(
                            operation_id,
                            is_step_export,
                            is_firmware_export,
                            || -> ArchiveWorkFuture<'_> {
                                Box::pin(self.step_bytes(operation_id, &snapshot, &scope))
                            },
                            |embed_used_models| -> ArchiveWorkFuture<'_> {
                                Box::pin(self.pack_archive(
                                    operation_id,
                                    &snapshot,
                                    &scope,
                                    embed_used_models,
                                ))
                            },
                            || -> ArchiveWorkFuture<'_> {
                                Box::pin(self.firmware_bytes(operation_id, &snapshot, &scope))
                            },
                        );
                    match work {
                        Ok(future) => future.await.map(|bytes| (bytes, snapshot.token)),
                        Err(reason) => Err(reason),
                    }
                };
                let result = result.and_then(|(bytes, token)| {
                    self.export_current(operation_id, token, &scope)
                        .then_some((bytes, token))
                        .ok_or_else(|| "Export scope changed before delivery.".to_owned())
                });
                match result {
                    Ok((bytes, delivery_token)) => {
                        let current = self.export_current(operation_id, delivery_token, &scope);
                        if !current {
                            return self.complete(Completion::ExportFailed {
                                operation_id,
                                reason: "Export scope changed before delivery.".into(),
                            });
                        }
                        let artifact_id = format!("archive-{}", operation_id.0);
                        let (filename, media_type) = if is_footprint_export {
                            (
                                format!("{}-footprints.zip", snapshot.document.name),
                                Some("application/zip".to_owned()),
                            )
                        } else if is_firmware_export {
                            (
                                format!("{}-zmk.zip", snapshot.document.name),
                                Some("application/zip".to_owned()),
                            )
                        } else if is_keycaps_export {
                            (
                                format!("{}-keycaps.zip", snapshot.document.name),
                                Some("application/zip".to_owned()),
                            )
                        } else if is_step_export {
                            (
                                authored_case_filename(&snapshot.document, &scope),
                                Some("model/step".to_owned()),
                            )
                        } else if is_pcb_handoff_export {
                            let draft = matches!(
                                kind.as_ref(),
                                Some(ExportKind::PcbHandoff { draft: true })
                            );
                            (
                                format!(
                                    "{}-{}pcb-handoff.zip",
                                    snapshot.document.name,
                                    if draft { "draft-" } else { "" }
                                ),
                                Some("application/zip".to_owned()),
                            )
                        } else if is_mechanical_export {
                            let filename = match kind.as_ref() {
                                Some(ExportKind::Mechanical { filename }) => filename.clone(),
                                _ => format!("{}-mechanical.zip", snapshot.document.name),
                            };
                            (filename, Some("application/zip".to_owned()))
                        } else if is_board_outline_export {
                            let dxf = matches!(
                                kind.as_ref(),
                                Some(ExportKind::BoardOutline { dxf: true })
                            );
                            (
                                format!(
                                    "{}.{}",
                                    snapshot.document.name,
                                    if dxf { "dxf" } else { "svg" }
                                ),
                                Some(
                                    if dxf {
                                        "application/dxf"
                                    } else {
                                        "image/svg+xml"
                                    }
                                    .to_owned(),
                                ),
                            )
                        } else {
                            (archive_filename(&snapshot.document.name), None)
                        };
                        self.artifacts.borrow_mut().insert(
                            artifact_id.clone(),
                            Artifact {
                                operation_id,
                                bytes,
                                filename,
                                media_type,
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
                self.export_leases.borrow_mut().cancel(operation_id);
                self.export_delivery_errors
                    .borrow_mut()
                    .remove(&operation_id);
                self.archive_export_options.cancel(operation_id);
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
