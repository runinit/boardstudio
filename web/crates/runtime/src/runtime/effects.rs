//! Session event submission and effect execution.
use super::*;
use crate::operation_outcomes::{commit_result, observed_outcome};
use boardstudio_application::{EditResolver, Landing, SnapshotToken};

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
        commit_result(result, || self.accepted_snapshot())
    }

    pub(super) fn submit_event(
        self: &Rc<Self>,
        event: Event,
        observe: Option<OperationId>,
    ) -> Option<TerminalOutcome> {
        self.submit_observed(event, observe)
            .map(|(outcome, _)| outcome)
    }

    pub(crate) fn submit_observed(
        self: &Rc<Self>,
        event: Event,
        observe: Option<OperationId>,
    ) -> Option<(TerminalOutcome, Option<Landing>)> {
        if self.project_deletion_pending.get() {
            let open_operation = match &event {
                Event::Open { operation_id, .. } => Some(*operation_id),
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
        self.refresh_after_session(previous_scope, previous_snapshot);
        let result = observed_outcome(&effects, observe);
        #[cfg(any(test, feature = "test-support"))]
        if self.in_process_adapters.borrow().is_some() {
            self.held_effects.borrow_mut().extend(effects);
            return result;
        }
        self.drive_effects(effects);
        result
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
        let accepted = self.accepted_snapshot().expect("accepted preview fixture");
        let scope = self.scope().expect("selected physical fixture scope");
        let generation = self.native_case_preview.borrow().generation + 1;
        let capture = crate::case_preview::capture_native_preview(
            &accepted,
            &scope,
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
        let persist = matches!(&event, Completion::Persist { .. });
        let prior = persist
            .then(|| self.with_model(|model| (model.lifecycle.clone(), model.durability.clone())));
        let effects = self.session.borrow_mut().complete(event);
        if let Some((lifecycle, durability)) = prior {
            let (same_lifecycle, next_durability) =
                self.with_model(|model| (model.lifecycle == lifecycle, model.durability.clone()));
            let snapshot_changed = self.accepted_identity() != previous_snapshot;
            let report = match &next_durability {
                Durability::Saved { .. } if snapshot_changed || next_durability != durability => {
                    Some(RuntimeReport::status("Saved locally.").transient())
                }
                Durability::Failed { reason, .. } if next_durability != durability => {
                    Some(RuntimeReport::status(reason))
                }
                _ => effects.iter().find_map(|effect| match effect {
                    Effect::Settled {
                        outcome: TerminalOutcome::PersistenceFailed(reason),
                        ..
                    } => Some(RuntimeReport::status(reason)),
                    _ => None,
                }),
            };
            let has_report = report.is_some();
            if let Some(report) = report {
                self.record_report(report);
            }
            if same_lifecycle && !snapshot_changed && self.scope() == previous_scope {
                if next_durability != durability {
                    let notify = self.notify_durability.borrow().clone();
                    if let Some(notify) = notify {
                        notify();
                    }
                    return effects;
                }
                if !has_report {
                    return effects;
                }
            }
        }
        self.refresh_after_session(previous_scope, previous_snapshot);
        effects
    }

    fn refresh_after_session(
        &self,
        previous_scope: Option<Scope>,
        previous_snapshot: Option<SnapshotToken>,
    ) {
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
    }

    fn accepted_identity(&self) -> Option<SnapshotToken> {
        self.with_model(|model| model.accepted.as_ref().map(|snapshot| snapshot.token))
    }

    #[cfg(any(test, feature = "test-support"))]
    pub(super) fn drive(self: &Rc<Self>, effects: Vec<Effect>) {
        self.drive_effects(effects);
    }

    fn drive_effects(self: &Rc<Self>, effects: Vec<Effect>) {
        let this = self.clone();
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
            if this.with_model(|model| model.lifecycle == Lifecycle::Closed) {
                this.close_browser_workers();
            }
            #[cfg(any(test, feature = "test-support"))]
            if let Some(finished) = finished {
                let _ = finished.send(());
            }
        });
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
                let expected = persistence.observed_token(&document.id);
                let result = match persistence
                    .save_document(&document, &assets, expected.as_ref())
                    .await
                {
                    Ok(_) => {
                        let mut staged = self.assets.borrow_mut();
                        for hash in assets.keys() {
                            staged.remove(hash);
                        }
                        SaveResult::Committed
                    }
                    Err(error) => SaveResult::Aborted(error.to_string()),
                };
                return self.complete(Completion::Persist {
                    save_attempt_id,
                    result,
                });
            }
            Effect::Settled {
                operation_id,
                outcome,
                ..
            } => {
                let observed = self
                    .operation_outcomes
                    .settle(operation_id, outcome.clone());
                let lease = self.exports.borrow_mut().remove(&operation_id);
                let kind = lease.as_ref().map(|state| &state.kind);
                let current = lease
                    .as_ref()
                    .is_none_or(|lease| self.export_source_is_current(lease))
                    && (!matches!(kind, Some(ExportKind::Firmware))
                        || self.latest_firmware_export.get() == Some(operation_id));
                if current {
                    if outcome == TerminalOutcome::Superseded && observed {
                        self.changed();
                    }
                    if let Some(report) = terminal_report(outcome, kind) {
                        self.apply_report(report);
                    }
                }
            }
            Effect::CapturePointer { pointer_id } | Effect::ReleasePointer { pointer_id } => {
                if let (Some(svg), Ok(pointer)) =
                    (self.surface.borrow().as_ref(), i32::try_from(pointer_id))
                {
                    if matches!(effect, Effect::CapturePointer { .. }) {
                        if let Err(error) = svg.set_pointer_capture(pointer) {
                            self.report(format!("Pointer capture failed: {error:?}"));
                        }
                    } else if svg.has_pointer_capture(pointer) {
                        let _ = svg.release_pointer_capture(pointer);
                    }
                }
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
            }
            Effect::CancelFrame { generation } => {
                if let Some((handle, _)) = self.frames.borrow_mut().remove(&generation)
                    && let Some(window) = web_sys::window()
                {
                    let _ = window.cancel_animation_frame(handle);
                }
            }
            Effect::RunExport {
                operation_id,
                scope,
                snapshot,
            } => {
                let kind = self
                    .exports
                    .borrow()
                    .get(&operation_id)
                    .map(|state| state.kind.clone());
                let result = async {
                    let kind = kind.ok_or("Export is no longer active.")?;
                    self.require_export_current(operation_id, &snapshot, &scope)?;
                    self.apply_report(RuntimeReport::status("Export started.").transient());
                    let name = &snapshot.document.name;
                    let mut token = snapshot.token;
                    let zip = |bytes, suffix: &str| {
                        (
                            bytes,
                            format!("{name}-{suffix}.zip"),
                            Some("application/zip"),
                        )
                    };
                    let (bytes, filename, media_type) = match &kind {
                        ExportKind::BoardOutline { dxf } => {
                            use boardstudio_model::model::{
                                OutlineExportFormat, OutlineExportRequest,
                            };
                            let (format, extension, media) = if *dxf {
                                (OutlineExportFormat::Dxf, "dxf", "application/dxf")
                            } else {
                                (OutlineExportFormat::Svg, "svg", "image/svg+xml")
                            };
                            let (board, contours) =
                                super::exports::outline_geometry(&snapshot, &scope)?;
                            let filename = format!("{name}.{extension}");
                            let file = boardstudio_core::artifact::outline::export_outline(
                                OutlineExportRequest {
                                    filename: filename.clone(),
                                    board: board.clone(),
                                    contours: contours.to_vec(),
                                    format,
                                },
                            )
                            .map_err(|error| error.message)?;
                            (file.content.into_bytes(), filename, Some(media))
                        }
                        ExportKind::ProjectCopy { embed_used_models } => (
                            self.pack_archive(operation_id, &snapshot, &scope, *embed_used_models)
                                .await?,
                            crate::archive_export::archive_filename(name),
                            None,
                        ),
                        ExportKind::Step => (
                            self.step_bytes(operation_id, &snapshot, &scope).await?,
                            authored_case_filename(&snapshot.document, &scope),
                            Some("model/step"),
                        ),
                        ExportKind::Mechanical { filename } => (
                            self.mechanical_package_bytes(operation_id, &snapshot, &scope)
                                .await?,
                            filename.clone(),
                            Some("application/zip"),
                        ),
                        ExportKind::PcbHandoff { draft } => {
                            let (bytes, accepted) = self
                                .pcb_handoff_bytes(operation_id, &snapshot, &scope, *draft)
                                .await?;
                            token = accepted;
                            zip(
                                bytes,
                                if *draft {
                                    "draft-pcb-handoff"
                                } else {
                                    "pcb-handoff"
                                },
                            )
                        }
                        ExportKind::KeycapsStl => zip(
                            self.keycaps_stl_bytes(operation_id, &snapshot, &scope)
                                .await?,
                            "keycaps",
                        ),
                        ExportKind::Footprints => zip(
                            self.footprint_export_bytes(operation_id, &snapshot, &scope)
                                .await?,
                            "footprints",
                        ),
                        ExportKind::Firmware => {
                            zip(self.firmware_bytes(operation_id, &snapshot, &scope)?, "zmk")
                        }
                    };
                    if !self.export_current(operation_id, token, &scope) {
                        return Err("Export scope changed before delivery.".into());
                    }
                    self.deliver_export(&bytes, &filename, media_type)?;
                    if matches!(kind, ExportKind::BoardOutline { .. }) {
                        self.apply_report(
                            RuntimeReport::status(format!("Saved {filename}.")).transient(),
                        );
                    }
                    Ok(token)
                }
                .await;
                return self.complete(Completion::Export {
                    operation_id,
                    result: result.map(|token| (token, scope)),
                });
            }
            Effect::RunGeneration {
                job_id,
                scope,
                snapshot,
                ..
            } => return self.generate(job_id, scope, snapshot).await,
            Effect::CancelJob { job_id } => {
                if let Some(cancelled) = self.cad_jobs.borrow_mut().remove(&job_id) {
                    cancelled.set(true);
                }
                if let Some((_, worker)) = self.cad_worker.borrow().as_ref() {
                    // Retain valid same-scope cache updates; the worker yields between bodies.
                    // Scope replacement and close still terminate the departing worker.
                    let _ = worker.cancel(&format!("case-{}", job_id.0));
                }
            }
            Effect::CancelExport { operation_id } => {
                self.exports.borrow_mut().remove(&operation_id);
                if let Some(worker) = self.export_workers.borrow_mut().remove(&operation_id) {
                    worker.close();
                }
            }
        }
        Vec::new()
    }
}

#[cfg(test)]
#[path = "persist_notification_tests.rs"]
mod persist_notification_tests;
