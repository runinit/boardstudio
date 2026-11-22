//! Export entry points, export leases and artifact generation.
use super::*;

impl Runtime {
    /// An existing resolved stack with suggested mounts must finish its saved initialization
    /// before a dependent generation or STEP capture. Archive saving remains independent.
    pub(super) fn mechanical_mount_initialization_pending(&self) -> bool {
        let Some((accepted, scope)) = self.model().accepted.zip(self.scope()) else {
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
        captured_case_document(&accepted, &scope).is_ok_and(|document| {
            document.mechanical.as_ref().is_some_and(|configuration| {
                configuration.board_id == scope.board_id
                    && configuration.closure_mounts.is_none()
                    && configuration.mount != boardstudio_model::model::MechanicalMount::Gasket
            })
        })
    }

    fn start_export(self: &Rc<Self>, kind: ExportKind, snapshot: &AcceptedSnapshot, scope: Scope) {
        if !matches!(
            kind,
            ExportKind::BoardOutline { .. } | ExportKind::ProjectCopy { .. }
        ) {
            self.clear_alert();
        }
        let operation_id = self.operation();
        if kind == ExportKind::Firmware {
            self.latest_firmware_export.set(Some(operation_id));
        }
        self.exports.borrow_mut().insert(
            operation_id,
            ExportState {
                kind,
                token: snapshot.token,
                scope: scope.clone(),
            },
        );
        self.submit(Event::StartExport {
            operation_id,
            scope,
        });
    }

    pub fn export_step(self: &Rc<Self>) {
        let Some((snapshot, scope)) = self.model().accepted.zip(self.scope()) else {
            self.apply_report(RuntimeReport::alert(
                "Select a board before exporting authored case geometry.",
            ));
            return;
        };
        if !boardstudio_model::case_readiness::authored_case_geometry_ready(
            &snapshot.document,
            &snapshot.scene,
            &scope.board_id,
        ) {
            self.apply_report(RuntimeReport::alert(
                "Resolve authored case findings for the selected board before export.",
            ));
            return;
        }
        self.start_export(ExportKind::Step, &snapshot, scope);
    }

    pub fn export_keycaps_stl(self: &Rc<Self>) {
        let Some((snapshot, scope)) = self.model().accepted.zip(self.scope()) else {
            self.apply_report(RuntimeReport::alert(
                "Select a board before exporting keycaps.",
            ));
            return;
        };
        if let Err(error) = self.ensure_keycaps_source_current(&snapshot, &scope) {
            self.apply_report(RuntimeReport::alert(error));
            return;
        }
        self.start_export(ExportKind::KeycapsStl, &snapshot, scope);
    }

    pub fn export_mechanical(self: &Rc<Self>) {
        if self.mechanical_mount_initialization_pending() {
            self.apply_report(RuntimeReport::alert(
                "Finish preparing mounting locations before exporting the mechanical assembly.",
            ));
            return;
        }
        let model = self.model();
        let Some((snapshot, scope)) = model.accepted.as_ref().zip(self.scope()) else {
            self.apply_report(RuntimeReport::alert(
                "Select a physical board before exporting the mechanical assembly.",
            ));
            return;
        };
        if snapshot.scene.revision != snapshot.document.revision {
            self.apply_report(RuntimeReport::alert(
                "The selected board is still resolving; export the current revision again.",
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
        self.start_export(
            ExportKind::Mechanical {
                filename: format!(
                    "{}{}-mechanical.zip",
                    snapshot.document.name, instance_suffix
                ),
            },
            snapshot,
            scope,
        );
    }

    pub fn export_firmware(self: &Rc<Self>) {
        let Some((snapshot, scope)) = self.model().accepted.zip(self.scope()) else {
            self.report("Select a board before exporting ZMK source.");
            return;
        };
        self.start_export(ExportKind::Firmware, &snapshot, scope);
    }

    pub fn export_footprints(self: &Rc<Self>) {
        let Some((snapshot, scope)) = self.model().accepted.zip(self.scope()) else {
            self.apply_report(RuntimeReport::alert(
                "Open a project before exporting its footprint library.",
            ));
            return;
        };
        if snapshot.document.definitions.is_empty() {
            self.apply_report(RuntimeReport::alert(
                "Add a component definition before exporting footprints.",
            ));
            return;
        }
        self.start_export(ExportKind::Footprints, &snapshot, scope);
    }

    pub fn export_kicad_board(self: &Rc<Self>, draft: bool) {
        let Some((snapshot, scope)) = self.model().accepted.zip(self.scope()) else {
            self.apply_report(RuntimeReport::alert("Select a board before KiCad export."));
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
        if !board_exists || !pcb_ready {
            self.apply_report(RuntimeReport::alert(
                "Resolve PCB findings before exporting this board.",
            ));
            return;
        }
        self.start_export(ExportKind::PcbHandoff { draft }, &snapshot, scope);
    }

    pub fn export_project_copy(self: &Rc<Self>) {
        let Some((snapshot, scope)) = self.model().accepted.zip(self.scope()) else {
            return;
        };
        self.start_export(
            ExportKind::ProjectCopy {
                embed_used_models: self.embed_used_models(),
            },
            &snapshot,
            scope,
        );
    }

    pub fn export_board_outline(
        self: &Rc<Self>,
        format: boardstudio_model::model::OutlineExportFormat,
    ) {
        let Some((snapshot, scope)) = self.model().accepted.zip(self.scope()) else {
            self.apply_report(RuntimeReport::alert(
                "Select a board before outline export.",
            ));
            return;
        };
        if let Err(error) = outline_geometry(&snapshot, &scope) {
            self.apply_report(RuntimeReport::alert(error));
            return;
        }
        self.start_export(
            ExportKind::BoardOutline {
                dxf: matches!(format, boardstudio_model::model::OutlineExportFormat::Dxf),
            },
            &snapshot,
            scope,
        );
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
            && self
                .exports
                .borrow()
                .get(&operation_id)
                .is_some_and(|state| state.token == token && &state.scope == scope)
    }

    pub(super) fn export_source_is_current(&self, state: &ExportState) -> bool {
        self.snapshot_current(state.token, &state.scope)
    }

    pub(super) fn snapshot_current(&self, token: SnapshotToken, scope: &Scope) -> bool {
        self.session.borrow().snapshot_is_current(token, scope)
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
            Err(CadJobError::Blocked(reason)) => self.complete(Completion::GenerationFailed {
                job_id,
                scope,
                reason,
                blocked: true,
            }),
            Err(CadJobError::Failed(reason) | CadJobError::Stale(reason)) => {
                self.complete(Completion::GenerationFailed {
                    job_id,
                    scope,
                    reason,
                    blocked: false,
                })
            }
            Err(CadJobError::Cancelled) => Vec::new(),
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
        let job = format!("case-{}", job_id.0);
        let prepared = prepare_captured_case(
            (self.core_services.open_scratch)(),
            &job,
            snapshot,
            scope,
            false,
        )?;
        guard()?;
        let worker = crate::case_model_lifecycle::case_model_worker(
            &self.cad_worker,
            scope,
            |worker| !worker.is_closed(),
            || {
                let url = resource_url("assets/cad-worker/entry.js")?;
                CadWorker::new(&url)
                    .map(Rc::new)
                    .map_err(|error| error.to_string())
            },
        )
        .map_err(CadJobError::Failed)?;
        worker
            .ready()
            .await
            .map_err(|error| CadJobError::Failed(error.to_string()))?;
        guard()?;
        let contours = captured_case_scene(snapshot, scope)?
            .board_contours
            .into_iter()
            .find(|board| board.board_id == scope.board_id)
            .ok_or_else(|| CadJobError::Blocked("Case contours unavailable".into()))?
            .contours;
        let physical_fingerprint =
            crate::case_generation_lifecycle::physical_case_fingerprint(snapshot, scope);
        for operation in [CadOperation::Preview, CadOperation::Exact] {
            let request = CadRequest {
                request_id: format!("{job}-{operation:?}"),
                job_id: job.clone(),
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
                physical_fingerprint,
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

pub(super) fn outline_geometry<'a>(
    snapshot: &'a AcceptedSnapshot,
    scope: &Scope,
) -> Result<(&'a Board, &'a [boardstudio_model::model::Contour]), String> {
    let board = snapshot
        .document
        .boards
        .iter()
        .find(|board| board.id == scope.board_id)
        .ok_or_else(|| "Select a resolved board before outline export.".to_owned())?;
    let contours = snapshot
        .scene
        .board_contours
        .iter()
        .find(|entry| entry.board_id == scope.board_id)
        .map(|entry| entry.contours.as_slice())
        .ok_or_else(|| "The selected board has no resolved outline.".to_owned())?;
    Ok((board, contours))
}
