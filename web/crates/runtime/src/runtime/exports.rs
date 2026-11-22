//! Export entry points, export leases and artifact generation.
use super::*;

impl Runtime {
    /// An existing resolved stack with suggested mounts must finish its saved initialization
    /// before a dependent generation or STEP capture. Archive saving remains independent.
    pub(super) fn mechanical_mount_initialization_pending(&self) -> bool {
        let model = self.model();
        let Some(accepted) = model.accepted.as_ref() else {
            return false;
        };
        let Some(scope) = self.scope() else {
            return false;
        };
        let scene = self.cad_scene();
        if !scene.as_ref().is_some_and(|scene| {
            scene.scope == scope
                && scene.token == accepted.token
                && scene.exact
                && scene
                    .mechanical
                    .as_ref()
                    .is_some_and(|assembly| !assembly.suggested_mounts.is_empty())
        }) {
            return false;
        }
        boardstudio_web_host::cad_jobs::captured_case_document(accepted, &scope).is_ok_and(
            |document| {
                document.mechanical.as_ref().is_some_and(|configuration| {
                    configuration.board_id == scope.board_id
                        && configuration.closure_mounts.is_none()
                        && configuration.mount != boardstudio_model::model::MechanicalMount::Gasket
                })
            },
        )
    }

    pub fn export_step(self: &Rc<Self>) {
        let Some(scope) = self.scope() else {
            self.apply_report(RuntimeReport::alert(
                "Select a board before exporting authored case geometry.",
            ));
            return;
        };
        let model = self.model();
        let Some(snapshot) = model.accepted.as_ref() else {
            self.apply_report(RuntimeReport::alert(
                "Authored Case STEP requires a ready accepted snapshot.",
            ));
            return;
        };
        if model.active_board_id != scope.board_id
            || snapshot.scene.revision != snapshot.document.revision
            || !snapshot
                .document
                .boards
                .iter()
                .any(|board| board.id == scope.board_id)
            || !snapshot
                .document
                .case_bodies
                .iter()
                .any(|body| body.board_id == scope.board_id)
            || !boardstudio_model::case_readiness::authored_case_geometry_ready(
                &snapshot.document,
                &snapshot.scene,
                &scope.board_id,
            )
        {
            self.apply_report(RuntimeReport::alert(
                "Resolve authored case findings for the selected board before export.",
            ));
            return;
        }
        self.clear_alert();
        let operation_id = self.operation();
        self.begin_export_lease(operation_id, ExportKind::Step, snapshot, &scope);
        self.submit(Event::StartExport {
            operation_id,
            scope,
        });
    }

    pub fn export_keycaps_stl(self: &Rc<Self>) {
        let Some(scope) = self.scope() else {
            self.apply_report(RuntimeReport::alert(
                "Select a board before exporting keycaps.",
            ));
            return;
        };
        let Some(snapshot) = self.model().accepted else {
            return;
        };
        if let Err(error) = self.ensure_keycaps_source_current(&snapshot, &scope) {
            self.apply_report(RuntimeReport::alert(error));
            return;
        }
        self.clear_alert();
        let operation_id = self.operation();
        self.begin_export_lease(operation_id, ExportKind::KeycapsStl, &snapshot, &scope);
        self.submit(Event::StartExport {
            operation_id,
            scope,
        });
    }

    pub fn export_mechanical(self: &Rc<Self>) {
        if self.mechanical_mount_initialization_pending() {
            self.apply_report(RuntimeReport::alert(
                "Finish preparing mounting locations before exporting the mechanical assembly.",
            ));
            return;
        }
        let Some(scope) = self.scope() else {
            self.apply_report(RuntimeReport::alert(
                "Select a physical board before exporting the mechanical assembly.",
            ));
            return;
        };
        let model = self.model();
        let Some(snapshot) = model.accepted else {
            self.apply_report(RuntimeReport::alert(
                "Mechanical export requires a ready accepted snapshot.",
            ));
            return;
        };
        if model.active_board_id != scope.board_id
            || snapshot.scene.revision != snapshot.document.revision
        {
            self.apply_report(RuntimeReport::alert(
                "The selected board is still resolving; export the current revision again.",
            ));
            return;
        }
        let document = match captured_case_document(&snapshot, &scope) {
            Ok(document) => document,
            Err(error) => {
                self.apply_report(RuntimeReport::alert(format!(
                    "Could not capture the selected mechanical configuration: {error:?}"
                )));
                return;
            }
        };
        let Some(configuration) = document.mechanical.as_ref() else {
            self.apply_report(RuntimeReport::alert(
                "Enable a mechanical assembly for the selected board before export.",
            ));
            return;
        };
        if configuration.board_id != scope.board_id {
            self.apply_report(RuntimeReport::alert(
                "The selected mechanical configuration belongs to another board.",
            ));
            return;
        }
        let outline_ready = snapshot
            .scene
            .board_readiness
            .iter()
            .find(|item| item.board_id == scope.board_id)
            .map_or(
                snapshot.document.boards.len() <= 1 && snapshot.scene.readiness.outline,
                |item| item.outline,
            );
        if !outline_ready {
            self.apply_report(RuntimeReport::alert(
                "Resolve active outline findings before exporting plate or case artifacts.",
            ));
            return;
        }
        let exact_generation = matches!(
            model.generation,
            boardstudio_application::GenerationStatus::Ready { exact: true, .. }
        );
        let exact_scene_ready = self.cad_scene().is_some_and(|cad| {
            cad.scope == scope
                && cad.token == snapshot.token
                && cad.exact
                && cad.mechanical.as_ref().is_some_and(|assembly| {
                    assembly.revision == snapshot.document.revision
                        && !assembly.generation_blocked
                        && !assembly.diagnostics.iter().any(|finding| {
                            finding.severity == boardstudio_model::model::Severity::Error
                        })
                })
        });
        if !exact_generation || !exact_scene_ready {
            self.apply_report(RuntimeReport::alert(
                "Update the current mechanical preview before export.",
            ));
            return;
        }
        let instance_suffix = match scope.instance_id.as_deref() {
            Some(instance_id) => {
                let Some(instance) = snapshot.document.hardware.as_ref().and_then(|hardware| {
                    hardware
                        .instances
                        .iter()
                        .find(|item| item.id == instance_id && item.board_id == scope.board_id)
                }) else {
                    self.apply_report(RuntimeReport::alert(
                        "The selected mechanical instance is no longer in this project.",
                    ));
                    return;
                };
                format!("-{}", mechanical_filename_component(&instance.name))
            }
            None => String::new(),
        };
        self.clear_alert();
        let operation_id = self.operation();
        self.begin_export_lease(
            operation_id,
            ExportKind::Mechanical {
                filename: format!(
                    "{}{}-mechanical.zip",
                    snapshot.document.name, instance_suffix
                ),
            },
            &snapshot,
            &scope,
        );
        self.submit(Event::StartExport {
            operation_id,
            scope,
        });
    }

    pub fn export_firmware(self: &Rc<Self>) {
        let Some(scope) = self.scope() else {
            self.report("Select a board before exporting ZMK source.");
            return;
        };
        let model = self.model();
        let Some(snapshot) = model.accepted else {
            self.report("Firmware export requires a ready accepted snapshot.");
            return;
        };
        self.clear_alert();
        let operation_id = self.operation();
        self.latest_firmware_export.set(Some(operation_id));
        self.begin_export_lease(operation_id, ExportKind::Firmware, &snapshot, &scope);
        self.submit(Event::StartExport {
            operation_id,
            scope,
        });
    }

    pub fn export_footprints(self: &Rc<Self>) {
        let Some(scope) = self.scope() else {
            self.apply_report(RuntimeReport::alert(
                "Open a project before exporting its footprint library.",
            ));
            return;
        };
        let model = self.model();
        let Some(snapshot) = model.accepted else {
            self.apply_report(RuntimeReport::alert(
                "Footprint export requires a ready accepted snapshot.",
            ));
            return;
        };
        if snapshot.document.definitions.is_empty() {
            self.apply_report(RuntimeReport::alert(
                "Add a component definition before exporting footprints.",
            ));
            return;
        }
        self.clear_alert();
        let operation_id = self.operation();
        self.begin_export_lease(operation_id, ExportKind::Footprints, &snapshot, &scope);
        self.submit(Event::StartExport {
            operation_id,
            scope,
        });
    }

    pub fn export_kicad_board(self: &Rc<Self>, draft: bool) {
        let Some(scope) = self.scope() else {
            self.apply_report(RuntimeReport::alert("Select a board before KiCad export."));
            return;
        };
        let model = self.model();
        let Some(snapshot) = model.accepted else {
            self.apply_report(RuntimeReport::alert(
                "KiCad export requires a ready accepted snapshot.",
            ));
            return;
        };
        let board_exists = snapshot
            .document
            .boards
            .iter()
            .any(|board| board.id == scope.board_id);
        let pcb_ready = snapshot
            .scene
            .board_readiness
            .iter()
            .find(|readiness| readiness.board_id == scope.board_id)
            .map_or(snapshot.scene.readiness.pcb, |readiness| readiness.pcb);
        if model.active_board_id != scope.board_id || !board_exists || !pcb_ready {
            self.apply_report(RuntimeReport::alert(
                "Resolve PCB findings before exporting this board.",
            ));
            return;
        }
        self.clear_alert();
        let operation_id = self.operation();
        self.begin_export_lease(
            operation_id,
            ExportKind::PcbHandoff { draft },
            &snapshot,
            &scope,
        );
        self.submit(Event::StartExport {
            operation_id,
            scope,
        });
    }

    pub fn export_project_copy(self: &Rc<Self>) {
        let Some(snapshot) = self.model().accepted else {
            return;
        };
        let Some(scope) = self.scope() else {
            return;
        };
        let operation_id = self.operation();
        self.begin_export_lease(
            operation_id,
            ExportKind::ProjectCopy {
                embed_used_models: self.embed_used_models(),
            },
            &snapshot,
            &scope,
        );
        self.submit(Event::StartExport {
            operation_id,
            scope,
        });
    }

    pub fn export_board_outline(
        self: &Rc<Self>,
        format: boardstudio_model::model::OutlineExportFormat,
    ) {
        let Some(scope) = self.scope() else {
            self.apply_report(RuntimeReport::alert(
                "Select a board before outline export.",
            ));
            return;
        };
        let Some(snapshot) = self.model().accepted else {
            self.apply_report(RuntimeReport::alert(
                "Outline export requires a ready accepted snapshot.",
            ));
            return;
        };
        let Some(_board) = snapshot
            .document
            .boards
            .iter()
            .find(|board| board.id == scope.board_id)
            .cloned()
        else {
            self.apply_report(RuntimeReport::alert(
                "Select a resolved board before outline export.",
            ));
            return;
        };
        let Some(_contours) = snapshot
            .scene
            .board_contours
            .iter()
            .find(|entry| entry.board_id == scope.board_id)
            .map(|entry| entry.contours.clone())
        else {
            self.apply_report(RuntimeReport::alert(
                "The selected board has no resolved outline.",
            ));
            return;
        };
        let operation_id = self.operation();
        self.begin_export_lease(
            operation_id,
            ExportKind::BoardOutline {
                dxf: matches!(format, boardstudio_model::model::OutlineExportFormat::Dxf),
            },
            &snapshot,
            &scope,
        );
        self.submit(Event::StartExport {
            operation_id,
            scope,
        });
    }

    pub(super) fn board_outline_bytes(
        &self,
        operation_id: OperationId,
        snapshot: &AcceptedSnapshot,
        scope: &Scope,
        dxf: bool,
    ) -> Result<(Vec<u8>, SnapshotToken), String> {
        let lease = self
            .exports
            .borrow()
            .get(&operation_id)
            .cloned()
            .ok_or_else(|| "Outline export owner was cancelled or superseded.".to_owned())?;
        if !matches!(&lease.kind, ExportKind::BoardOutline { dxf: captured } if *captured == dxf)
            || &lease.scope != scope
            || lease.token != snapshot.token
        {
            return Err("Outline export no longer matches its captured board.".into());
        }
        let board = snapshot
            .document
            .boards
            .iter()
            .find(|board| board.id == scope.board_id)
            .cloned()
            .ok_or_else(|| "Select a resolved board before outline export.".to_owned())?;
        let contours = snapshot
            .scene
            .board_contours
            .iter()
            .find(|entry| entry.board_id == scope.board_id)
            .map(|entry| entry.contours.clone())
            .ok_or_else(|| "The selected board has no resolved outline.".to_owned())?;
        let format = if dxf {
            boardstudio_model::model::OutlineExportFormat::Dxf
        } else {
            boardstudio_model::model::OutlineExportFormat::Svg
        };
        let format_name = if dxf { "dxf" } else { "svg" };
        let request_id = format!("export-outline-{}", operation_id.0);
        let request = ArtifactRequest::ExportOutline {
            id: request_id.clone(),
            request: boardstudio_model::model::OutlineExportRequest {
                filename: format!("{}.{}", snapshot.document.name, format_name),
                board,
                contours,
                format,
            },
        };
        if !self.export_lease_is_current(operation_id, &lease) {
            return Err("Outline export was cancelled, superseded, or its source changed.".into());
        }
        let reply = artifact_reply(&request)?;
        let file = match reply {
            ArtifactReply::ExportOutline { id, result } if id == request_id => result,
            ArtifactReply::Error { id, error } if id == request_id => {
                return Err(format!("Outline export failed: {}", error.message));
            }
            _ => return Err("Core returned an outline for another export request.".into()),
        };
        Ok((file.content.into_bytes(), snapshot.token))
    }

    pub(super) fn export_lease_is_current(
        &self,
        operation_id: OperationId,
        lease: &ExportState,
    ) -> bool {
        self.export_current(operation_id, lease.token, &lease.scope)
    }

    pub(super) fn export_current(
        &self,
        operation_id: OperationId,
        token: SnapshotToken,
        scope: &Scope,
    ) -> bool {
        self.session
            .borrow()
            .export_is_current(operation_id, token, scope)
            && self.export_delivery_is_current(operation_id, token, scope)
    }

    pub(super) fn begin_export_lease(
        &self,
        operation_id: OperationId,
        kind: ExportKind,
        snapshot: &AcceptedSnapshot,
        scope: &Scope,
    ) {
        self.exports.borrow_mut().insert(
            operation_id,
            ExportState {
                kind,
                token: snapshot.token,
                scope: scope.clone(),
            },
        );
    }

    pub(super) fn export_source_is_current(&self, state: &ExportState) -> bool {
        self.snapshot_current(state.token, &state.scope)
    }

    pub(super) fn export_delivery_is_current(
        &self,
        operation_id: OperationId,
        token: SnapshotToken,
        scope: &Scope,
    ) -> bool {
        self.exports
            .borrow()
            .get(&operation_id)
            .is_some_and(|state| {
                state.token == token
                    && &state.scope == scope
                    && self.export_source_is_current(state)
            })
    }

    pub(super) fn snapshot_current(&self, token: SnapshotToken, scope: &Scope) -> bool {
        self.scope().as_ref() == Some(scope)
            && self
                .model()
                .accepted
                .as_ref()
                .is_some_and(|s| s.token == token)
    }
    pub(super) fn deliver_export(
        &self,
        bytes: &[u8],
        filename: &str,
        media_type: Option<&str>,
    ) -> Result<(), String> {
        #[cfg(any(test, feature = "test-support"))]
        if let Some(adapters) = self.in_process_adapters.borrow().as_ref() {
            adapters.deliver(bytes, filename, media_type);
            return Ok(());
        }
        deliver(bytes, filename, media_type)
    }
    pub(super) async fn generate(
        self: &Rc<Self>,
        job_id: JobId,
        scope: Scope,
        snapshot: AcceptedSnapshot,
    ) -> Vec<Effect> {
        let cancelled = Rc::new(Cell::new(false));
        self.cad_jobs.borrow_mut().insert(job_id, cancelled.clone());
        let result = self
            .generate_current(job_id, &scope, &snapshot, &cancelled)
            .await;
        self.cad_jobs.borrow_mut().remove(&job_id);
        let current_job = matches!(self.model().generation,
            boardstudio_application::GenerationStatus::Preparing { job_id: active }
            | boardstudio_application::GenerationStatus::Running { job_id: active } if active == job_id);
        if result.is_err()
            && current_job
            && let Some((_, worker)) = self.cad_worker.borrow_mut().take()
        {
            worker.close();
        }
        match result {
            Ok(()) => self.complete(Completion::GenerationFinished {
                job_id,
                scope,
                exact: true,
            }),
            Err(CadJobError::Blocked(reason)) => self.complete(Completion::GenerationBlocked {
                job_id,
                scope,
                reason,
            }),
            Err(CadJobError::Failed(reason) | CadJobError::Stale(reason)) => {
                self.complete(Completion::GenerationFailed {
                    job_id,
                    scope,
                    reason,
                })
            }
            Err(CadJobError::Cancelled) => vec![],
        }
    }
    pub(super) async fn generate_current(
        &self,
        job_id: JobId,
        scope: &Scope,
        snapshot: &AcceptedSnapshot,
        cancelled: &Cell<bool>,
    ) -> Result<(), CadJobError> {
        let guard = || {
            let current_job = matches!(self.model().generation,
                boardstudio_application::GenerationStatus::Preparing { job_id: active }
                | boardstudio_application::GenerationStatus::Running { job_id: active } if active == job_id);
            if cancelled.get() || !current_job || !self.snapshot_current(snapshot.token, scope) {
                Err(CadJobError::Cancelled)
            } else {
                Ok(())
            }
        };
        guard()?;
        let prepared = prepare_captured_case(
            (self.core_services.open_scratch)(),
            &format!("case-{}", job_id.0),
            snapshot,
            scope,
            false,
        )?;
        guard()?;
        let previous = self.cad_worker.borrow_mut().take();
        let worker = match previous {
            Some((worker_scope, worker)) if worker_scope == *scope && !worker.is_closed() => worker,
            Some((_, worker)) => {
                worker.close();
                Rc::new(
                    CadWorker::new(
                        &resource_url("assets/cad-worker/entry.js").map_err(CadJobError::Failed)?,
                    )
                    .map_err(|e| CadJobError::Failed(e.to_string()))?,
                )
            }
            None => Rc::new(
                CadWorker::new(
                    &resource_url("assets/cad-worker/entry.js").map_err(CadJobError::Failed)?,
                )
                .map_err(|e| CadJobError::Failed(e.to_string()))?,
            ),
        };
        *self.cad_worker.borrow_mut() = Some((scope.clone(), worker.clone()));
        worker
            .ready()
            .await
            .map_err(|e| CadJobError::Failed(e.to_string()))?;
        guard()?;
        let contours = captured_case_scene(snapshot, scope)?
            .board_contours
            .into_iter()
            .find(|board| board.board_id == scope.board_id)
            .ok_or_else(|| CadJobError::Blocked("Case contours unavailable".into()))?
            .contours;
        for operation in [CadOperation::Preview, CadOperation::Exact] {
            let request = CadRequest {
                request_id: format!("case-{}-{operation:?}", job_id.0),
                job_id: format!("case-{}", job_id.0),
                identity: prepared.identity.clone(),
                operation,
                prepared: Some(prepared.prepared.clone()),
                source_sha256: None,
                step_import_options: None,
                collision_query: None,
                generated_analysis_request: None,
                collision_meshes: None,
                input_bytes: vec![],
            };
            let reply = worker
                .request(request.clone())
                .await
                .map_err(|e| CadJobError::Failed(e.to_string()))?;
            guard()?;
            let mut result = validate_reply(&request, reply, &prepared.identity)?;
            // Renderer retention needs meshes only; manufacturing exports have their own worker.
            result.step = Vec::new();
            *self.cad_scene.borrow_mut() = Some(Rc::new(CadScene {
                scope: scope.clone(),
                token: snapshot.token,
                snapshot: snapshot.clone(),
                result,
                prepared: prepared.prepared.clone(),
                physical_fingerprint: crate::case_generation_lifecycle::physical_case_fingerprint(
                    snapshot, scope,
                ),
                mechanical: prepared.mechanical_assembly.clone(),
                exact: operation == CadOperation::Exact,
                contours: contours.clone(),
            }));
            self.changed();
        }
        Ok(())
    }
    pub(super) fn require_export_current(
        &self,
        operation_id: OperationId,
        snapshot: &AcceptedSnapshot,
        scope: &Scope,
    ) -> Result<(), String> {
        if self.export_current(operation_id, snapshot.token, scope) {
            Ok(())
        } else {
            Err("Export lease is no longer current.".into())
        }
    }
}
