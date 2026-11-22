//! PCB handoff, physical STEP assembly and mechanical package exports.
use super::*;

impl Runtime {
    pub(super) async fn pcb_handoff_bytes(
        self: &Rc<Self>,
        operation_id: OperationId,
        initial_snapshot: &AcceptedSnapshot,
        scope: &Scope,
        draft: bool,
    ) -> Result<(Vec<u8>, SnapshotToken), String> {
        let mut capture = self
            .export_leases
            .borrow()
            .get(&operation_id)
            .cloned()
            .ok_or_else(|| "KiCad export owner was cancelled or superseded.".to_owned())?;
        if !matches!(capture.kind(), ExportKind::PcbHandoff { draft: captured_draft } if *captured_draft == draft)
            || capture.scope() != scope
            || capture.token() != initial_snapshot.token
            || capture.snapshot().document.revision != initial_snapshot.document.revision
        {
            return Err("KiCad export no longer matches its captured board.".into());
        }
        self.require_pcb_handoff_current(operation_id, &capture)?;
        let mut snapshot = initial_snapshot.clone();
        let mut plan =
            self.resolve_pcb_handoff_plan(operation_id, &snapshot, scope, None, &capture)?;
        let errors = plan
            .diagnostics
            .iter()
            .filter(|finding| finding.severity == "error")
            .map(|finding| finding.message.clone())
            .collect::<Vec<_>>();
        if !draft && !errors.is_empty() {
            return Err(errors.join("\n"));
        }
        if !pcb_wiring_is_applied(&snapshot.document, &plan) {
            capture = self.commit_pcb_handoff(
                operation_id,
                &capture,
                boardstudio_application::ExportCommitRequest::ApplyElectrical {
                    plan: plan.clone(),
                    draft,
                },
            )?;
            snapshot = self
                .model()
                .accepted
                .ok_or_else(|| "Accepted board snapshot disappeared after wiring.".to_owned())?;
            plan = self.resolve_pcb_handoff_plan(operation_id, &snapshot, scope, None, &capture)?;
            let errors = plan
                .diagnostics
                .iter()
                .filter(|finding| finding.severity == "error")
                .map(|finding| finding.message.clone())
                .collect::<Vec<_>>();
            if !draft && !errors.is_empty() {
                return Err(errors.join("\n"));
            }
        }
        self.require_pcb_handoff_current(operation_id, &capture)?;
        let mut populations = Vec::new();
        for instance in snapshot
            .document
            .hardware
            .as_ref()
            .into_iter()
            .flat_map(|hardware| hardware.instances.iter())
            .filter(|instance| instance.board_id == scope.board_id)
        {
            let population = self.resolve_pcb_handoff_plan(
                operation_id,
                &snapshot,
                scope,
                Some(&instance.id),
                &capture,
            )?;
            if !draft
                && population
                    .diagnostics
                    .iter()
                    .any(|finding| finding.severity == "error")
            {
                return Err(format!(
                    "Resolve wiring findings for {} before export",
                    instance.name
                ));
            }
            populations.push((instance.name.clone(), population));
        }
        self.require_pcb_handoff_current(operation_id, &capture)?;
        let archive = crate::pcb_handoff::build_handoff(
            crate::pcb_handoff::HandoffSource {
                operation_id,
                snapshot: &snapshot,
                scope,
                electrical_plan: plan.clone(),
                populations,
                draft,
            },
            crate::pcb_handoff::HandoffPorts { store: &self.store },
            || self.require_pcb_handoff_current(operation_id, &capture),
        )
        .await?;
        capture = self.commit_pcb_handoff(
            operation_id,
            &capture,
            boardstudio_application::ExportCommitRequest::ProtectElectricalHandoff { plan },
        )?;
        self.require_pcb_handoff_current(operation_id, &capture)?;
        Ok((archive, capture.token()))
    }

    /// Share complete physical PCB preparation between export and collision.
    pub(super) async fn prepare_physical_step_assembly(
        &self,
        request_prefix: &str,
        snapshot: &AcceptedSnapshot,
        scope: &Scope,
        is_current: Rc<dyn Fn() -> bool>,
    ) -> Result<PhysicalStepAssembly, String> {
        let ensure_current = || {
            if is_current() {
                Ok(())
            } else {
                Err("Physical PCB preparation was cancelled or became stale.".to_owned())
            }
        };
        ensure_current()?;
        let mut prepared = prepare_captured_case(
            (self.core_services.open_scratch)(),
            request_prefix,
            snapshot,
            scope,
            true,
        )
        .map_err(|error| format!("Mechanical assembly resolution failed: {error:?}"))?;
        ensure_current()?;
        let pcb_source = captured_pcb_source(snapshot, scope)
            .map_err(|error| format!("Could not capture imported PCB source: {error:?}"))?;
        let mut pcb_source_thickness = None;
        if let Some(source) = &pcb_source {
            let source_is_current = is_current.clone();
            let bytes = self
                .load_preview_document_asset(&source.asset, source_is_current)
                .await?;
            ensure_current()?;
            let request_id = format!("{request_prefix}-pcb-source");
            let request = ArtifactRequest::ReadBoardSource {
                id: request_id.clone(),
                source: String::from_utf8(bytes)
                    .map_err(|error| format!("PCB source is not valid UTF-8: {error}"))?,
                placement: source.placement,
                flipped: source.flipped,
            };
            let reply = artifact_reply(&request)?;
            let board = match reply {
                ArtifactReply::ReadBoardSource { id, result } if id == request_id => result,
                _ => return Err("Core returned PCB source for another request.".into()),
            };
            let indices = prepared
                .prepared
                .bodies
                .iter()
                .enumerate()
                .filter(|(_, body)| body.body.id == "pcb-reference")
                .map(|(index, _)| index)
                .collect::<Vec<_>>();
            if indices.len() != 1 {
                return Err("Prepared PCB reference is missing or ambiguous.".into());
            }
            let mut body = prepared.prepared.bodies[indices[0]].body.clone();
            pcb_source_thickness = board.thickness;
            if board
                .thickness
                .is_some_and(|thickness| thickness != body.thickness)
            {
                return Err(
                    "Imported PCB thickness does not match the mechanical PCB thickness.".into(),
                );
            }
            if board
                .pcb_source
                .as_ref()
                .is_none_or(|profile| profile.contours.is_empty())
                || board.contours.is_empty()
            {
                return Err("Core returned incomplete PCB source geometry.".into());
            }
            if body
                .features
                .as_ref()
                .is_some_and(|features| !features.is_empty())
                || body
                    .openings
                    .as_ref()
                    .is_some_and(|openings| !openings.is_empty())
                || body
                    .mounts
                    .as_ref()
                    .is_some_and(|mounts| !mounts.is_empty())
                || body.gasket.is_some()
                || body.pcb_source.is_some()
            {
                return Err(
                    "Imported source cannot replace a modified nominal PCB reference.".into(),
                );
            }
            body.z = Some(body.z.unwrap_or(0.0) + source.reference.elevation);
            body.pcb_source = board.pcb_source;
            body.mounts = (!board.round_drills.is_empty()).then_some(board.round_drills);
            let mut contours = board.contours;
            contours.extend(
                board
                    .holes
                    .into_iter()
                    .map(|points| boardstudio_model::model::Contour { points, hole: true }),
            );
            let source_ir = CaseIR {
                revision: prepared.identity.revision,
                body,
                contours,
            };
            let request_id = format!("{request_prefix}-pcb-prepare");
            let request = CoreRequest::PrepareCase {
                id: request_id.clone(),
                ir: CaseAssemblyIR {
                    revision: source_ir.revision,
                    bodies: vec![source_ir.clone()],
                },
            };
            let reply = (self.core_services.open_scratch)()(request);
            let mut ir = match reply {
                CoreReply::CasePrepared { id, ir } if id == request_id => ir,
                CoreReply::Error { id, message, .. } if id == request_id => return Err(message),
                _ => return Err("Core returned an unexpected source PCB preparation.".into()),
            };
            if ir.revision != prepared.identity.revision
                || ir.bodies.len() != 1
                || ir.bodies[0].revision != ir.revision
                || ir.bodies[0].body != source_ir.body
            {
                return Err("Core returned an incomplete or stale source PCB preparation.".into());
            }
            let mut pcb = ir.bodies.pop().expect("one source PCB body checked");
            // Compatibility polygons already contain these voids; exact CAD uses body mounts.
            for region in &mut pcb.regions {
                region.mounts.clear();
            }
            prepared.prepared.bodies[indices[0]] = pcb;
            if let Some(assembly) = &mut prepared.mechanical_assembly {
                assembly.pcb_reference = Some(source_ir);
                for layer in &mut assembly.stack {
                    if layer.id == "pcb" {
                        layer.z += source.reference.elevation;
                    }
                }
            }
        }
        Ok(PhysicalStepAssembly {
            prepared,
            pcb_source,
            pcb_source_thickness,
        })
    }

    pub(super) async fn mechanical_package_bytes(
        self: &Rc<Self>,
        operation_id: OperationId,
        initial_snapshot: &AcceptedSnapshot,
        scope: &Scope,
    ) -> Result<Vec<u8>, String> {
        let capture = self
            .export_leases
            .borrow()
            .get(&operation_id)
            .cloned()
            .ok_or_else(|| "Mechanical export owner was cancelled or superseded.".to_owned())?;
        if capture.scope() != scope
            || capture.token() != initial_snapshot.token
            || capture.snapshot().document.revision != initial_snapshot.document.revision
            || capture.snapshot().session_epoch != initial_snapshot.session_epoch
            || capture.snapshot().document.id != initial_snapshot.document.id
        {
            return Err("Mechanical export no longer matches its captured project.".into());
        }
        let runtime = self.clone();
        let ensure_current = || -> Result<(), String> {
            if runtime.export_lease_is_current(operation_id, &capture) {
                Ok(())
            } else {
                Err("Mechanical export was cancelled, superseded, or its source changed.".into())
            }
        };
        ensure_current()?;
        let document = captured_case_document(initial_snapshot, scope)
            .map_err(|error| format!("Could not capture mechanical document: {error:?}"))?;
        let scene = captured_case_scene(initial_snapshot, scope)
            .map_err(|error| format!("Could not capture mechanical scene: {error:?}"))?;
        let configuration = document
            .mechanical
            .as_ref()
            .filter(|configuration| configuration.board_id == scope.board_id)
            .ok_or_else(|| {
                "Enable a mechanical assembly for the selected board before export.".to_owned()
            })?;
        let outline_ready = initial_snapshot
            .scene
            .board_readiness
            .iter()
            .find(|item| item.board_id == scope.board_id)
            .map_or(
                initial_snapshot.document.boards.len() <= 1
                    && initial_snapshot.scene.readiness.outline,
                |item| item.outline,
            );
        if !outline_ready {
            return Err(
                "Resolve active outline findings before exporting plate or case artifacts.".into(),
            );
        }
        let contours = scene
            .board_contours
            .iter()
            .find(|item| item.board_id == scope.board_id)
            .map(|item| item.contours.clone())
            .ok_or_else(|| "The selected board has no resolved outline contours.".to_owned())?;
        let source_is_current = {
            let runtime = self.clone();
            let capture = capture.clone();
            Rc::new(move || runtime.export_lease_is_current(operation_id, &capture))
        };
        let PhysicalStepAssembly {
            prepared,
            pcb_source,
            pcb_source_thickness,
        } = self
            .prepare_physical_step_assembly(
                &format!("mechanical-export-{}", operation_id.0),
                initial_snapshot,
                scope,
                source_is_current,
            )
            .await?;
        ensure_current()?;
        let assembly = prepared
            .mechanical_assembly
            .as_ref()
            .ok_or_else(|| "Core did not resolve a generated mechanical assembly.".to_owned())?;
        if assembly.revision != capture.snapshot().document.revision
            || assembly.case.revision != capture.snapshot().document.revision
            || assembly.generation_blocked
        {
            return Err("Mechanical assembly is blocked or belongs to another revision.".into());
        }
        let errors = assembly
            .diagnostics
            .iter()
            .filter(|finding| finding.severity == boardstudio_model::model::Severity::Error)
            .map(|finding| finding.message.clone())
            .collect::<Vec<_>>();
        if !errors.is_empty() {
            return Err(errors.join("\n"));
        }

        let mut files: Vec<(String, Vec<u8>)> = Vec::new();
        let mut paths = BTreeSet::new();
        let plate_method = configuration
            .part_processes
            .as_ref()
            .and_then(|processes| processes.iter().find(|process| process.part_id == "plate"))
            .map(|process| &process.method)
            .unwrap_or(&configuration.method);
        if matches!(plate_method, boardstudio_model::model::PlateMethod::PcbFr4) {
            let request_id = format!("mechanical-plate-{}", operation_id.0);
            let request = ArtifactRequest::ExportMechanicalPlate {
                id: request_id.clone(),
                document: document.clone(),
                contours: contours.clone(),
            };
            let reply = artifact_reply(&request)?;
            let result = match reply {
                ArtifactReply::ExportMechanicalPlate { id, result } if id == request_id => result,
                _ => return Err("Core returned an unexpected mechanical plate artifact.".into()),
            };
            if result.revision != capture.snapshot().document.revision {
                return Err("Core returned a mechanical plate for another revision.".into());
            }
            for file in result.files {
                push_mechanical_file(
                    &mut files,
                    &mut paths,
                    format!("plate-kicad/{}", file.filename),
                    file.content.into_bytes(),
                )?;
            }
        }

        let worker_url = resource_url("assets/cad-worker/entry.js")?;
        let worker = Rc::new(CadWorker::new(&worker_url).map_err(|error| error.to_string())?);
        self.export_workers
            .borrow_mut()
            .insert(operation_id, worker.clone());
        let cad_output = async {
            worker.ready().await.map_err(|error| error.to_string())?;
            ensure_current()?;
            let assembled = request_exact_cad(
                &worker,
                operation_id,
                "assembly",
                &prepared.identity,
                prepared.prepared.clone(),
            )
            .await?;
            ensure_current()?;
            if assembled.step.is_empty() {
                return Err("CAD returned no assembled STEP data.".into());
            }
            push_mechanical_file(
                &mut files,
                &mut paths,
                "assembly.step".into(),
                assembled.step,
            )?;

            let board = document
                .boards
                .iter()
                .find(|board| board.id == scope.board_id)
                .ok_or_else(|| "The selected board is no longer available.".to_owned())?;
            for (index, source) in assembly.case.bodies.iter().enumerate() {
                ensure_current()?;
                let prepared_body = prepared
                    .prepared
                    .bodies
                    .iter()
                    .find(|body| body.body.id == source.body.id)
                    .cloned()
                    .ok_or_else(|| format!("Prepared CAD body '{}' is missing.", source.body.id))?;
                let part_name = format!(
                    "{}-{}",
                    index + 1,
                    mechanical_filename_component(&source.body.name)
                );
                let result = request_exact_cad(
                    &worker,
                    operation_id,
                    &format!("part-{index}"),
                    &prepared.identity,
                    boardstudio_model::model::PreparedCaseAssemblyIR {
                        revision: prepared.identity.revision,
                        bodies: vec![prepared_body],
                    },
                )
                .await?;
                ensure_current()?;
                let mesh = result
                    .mesh
                    .as_ref()
                    .ok_or_else(|| format!("CAD returned no mesh for {part_name}."))?;
                push_mechanical_file(
                    &mut files,
                    &mut paths,
                    format!("parts/{part_name}.step"),
                    result.step,
                )?;
                push_mechanical_file(
                    &mut files,
                    &mut paths,
                    format!("parts/{part_name}.stl"),
                    mechanical_stl(mesh)?,
                )?;

                let mut outline_contours = source.contours.clone();
                for mount in source.body.mounts.iter().flatten() {
                    outline_contours.push(boardstudio_model::model::Contour {
                        hole: true,
                        points: (0..96)
                            .map(|point| {
                                let angle = point as f64 * std::f64::consts::TAU / 96.0;
                                boardstudio_model::model::Vec2 {
                                    x: mount.at.x + mount.hole_diameter / 2.0 * angle.cos(),
                                    y: mount.at.y + mount.hole_diameter / 2.0 * angle.sin(),
                                }
                            })
                            .collect(),
                    });
                }
                let mut export_board = board.clone();
                export_board.id = source.body.id.clone();
                export_board.name = source.body.name.clone();
                export_board.outline_ids.clear();
                export_board.part_ids.clear();
                export_board.net_ids.clear();
                export_board.traces.clear();
                export_board.vias.clear();
                for (extension, format) in [
                    ("dxf", boardstudio_model::model::OutlineExportFormat::Dxf),
                    ("svg", boardstudio_model::model::OutlineExportFormat::Svg),
                ] {
                    let filename = format!("{part_name}.{extension}");
                    let request_id =
                        format!("mechanical-outline-{}-{index}-{extension}", operation_id.0);
                    let request = ArtifactRequest::ExportOutline {
                        id: request_id.clone(),
                        request: boardstudio_model::model::OutlineExportRequest {
                            filename: filename.clone(),
                            board: export_board.clone(),
                            contours: outline_contours.clone(),
                            format,
                        },
                    };
                    let reply = artifact_reply(&request)?;
                    let file = match reply {
                        ArtifactReply::ExportOutline { id, result } if id == request_id => result,
                        _ => return Err("Core returned an unexpected mechanical outline.".into()),
                    };
                    if file.filename != filename {
                        return Err("Core returned a mechanical outline under another name.".into());
                    }
                    push_mechanical_file(
                        &mut files,
                        &mut paths,
                        format!("outlines/{}", file.filename),
                        file.content.into_bytes(),
                    )?;
                }
            }
            Ok::<(), String>(())
        }
        .await;
        worker.close();
        self.export_workers.borrow_mut().remove(&operation_id);
        cad_output?;
        ensure_current()?;

        push_mechanical_file(
            &mut files,
            &mut paths,
            "FABRICATION.md".into(),
            mechanical_fabrication_notes(&document, assembly).into_bytes(),
        )?;
        push_mechanical_file(
            &mut files,
            &mut paths,
            "critical-fit.svg".into(),
            critical_fit_drawing(assembly, configuration)?.into_bytes(),
        )?;
        let assembly_json = serde_json::json!({
            "revision": capture.snapshot().document.revision,
            "pcbReference": if pcb_source.is_some() {
                "Source-backed unpopulated PCB; component solids are not included"
            } else { "Nominal unpopulated PCB only; component solids are not included" },
            "pcbSource": pcb_source.as_ref().map(|source| serde_json::json!({
                "referenceId": source.reference.id,
                "assetId": source.asset.id,
                "sha256": source.asset.sha256,
                "declaredThicknessMm": pcb_source_thickness,
            })),
            "stack": assembly.stack,
            "diagnostics": assembly.diagnostics,
        });
        push_mechanical_file(
            &mut files,
            &mut paths,
            "assembly.json".into(),
            serde_json::to_vec_pretty(&assembly_json)
                .map_err(|error| format!("Could not serialize mechanical assembly: {error}"))?,
        )?;
        ensure_current()?;
        crate::portable_archive::pack_files(files)
    }

    pub(super) fn resolve_pcb_handoff_plan(
        &self,
        operation_id: OperationId,
        snapshot: &AcceptedSnapshot,
        scope: &Scope,
        instance_id: Option<&str>,
        capture: &ExportLease,
    ) -> Result<ElectricalPlan, String> {
        if !self.export_lease_is_current(operation_id, capture) {
            return Err("Accepted board changed before wiring resolution.".into());
        }
        let request_id = format!(
            "pcb-handoff-{}-wiring-{}",
            operation_id.0,
            instance_id.unwrap_or("board")
        );
        let plan = self.resolve_export_wiring(snapshot, &scope.board_id, instance_id, request_id);
        self.require_pcb_handoff_current(operation_id, capture)?;
        plan
    }

    fn commit_pcb_handoff(
        self: &Rc<Self>,
        export_operation_id: OperationId,
        capture: &ExportLease,
        commit: boardstudio_application::ExportCommitRequest,
    ) -> Result<ExportLease, String> {
        if !self.export_lease_is_current(export_operation_id, capture) {
            return Err("KiCad export no longer owns the accepted wiring source.".into());
        }
        let operation_id = self.operation();
        let outcome = self.submit_event(
            Event::ExportCommit {
                operation_id,
                export_operation_id,
                token: capture.token(),
                scope: capture.scope().clone(),
                commit,
            },
            Some(operation_id),
        );
        if outcome != Some(TerminalOutcome::Completed) {
            return Err(format!(
                "Could not accept export wiring change: {outcome:?}"
            ));
        }
        let accepted = self
            .model()
            .accepted
            .ok_or("No accepted wiring snapshot.")?;
        let mut updated = capture.clone();
        updated.advance(accepted)?;
        if !self.session.borrow().export_is_current(
            export_operation_id,
            updated.token(),
            updated.scope(),
        ) {
            return Err("Export wiring changed the captured project or board.".into());
        }
        self.export_leases
            .borrow_mut()
            .insert(export_operation_id, updated.clone());
        Ok(updated)
    }

    pub(super) fn require_pcb_handoff_current(
        &self,
        operation_id: OperationId,
        capture: &ExportLease,
    ) -> Result<(), String> {
        if self.export_lease_is_current(operation_id, capture) {
            Ok(())
        } else {
            Err("KiCad export was cancelled, superseded, or its accepted source changed.".into())
        }
    }

    pub(super) async fn footprint_export_bytes(
        self: &Rc<Self>,
        operation_id: OperationId,
        snapshot: &AcceptedSnapshot,
        scope: &Scope,
    ) -> Result<Vec<u8>, String> {
        let capture = self
            .export_leases
            .borrow()
            .get(&operation_id)
            .cloned()
            .ok_or_else(|| "Footprint export owner was cancelled or superseded.".to_owned())?;
        if capture.scope() != scope
            || capture.token() != snapshot.token
            || capture.snapshot().document.revision != snapshot.document.revision
            || capture.snapshot().session_epoch != snapshot.session_epoch
            || capture.snapshot().document.id != snapshot.document.id
        {
            return Err("Footprint export no longer matches its captured project.".into());
        }
        let runtime = self.clone();
        let ensure_current = || {
            if runtime.export_lease_is_current(operation_id, &capture) {
                Ok(())
            } else {
                Err("Footprint export was cancelled, superseded, or its source changed.".into())
            }
        };
        crate::export_footprints::build_zip(
            crate::export_footprints::ExportSource {
                operation_id,
                snapshot,
                store: &self.store,
            },
            ensure_current,
        )
        .await
    }
}

async fn request_exact_cad(
    worker: &CadWorker,
    operation_id: OperationId,
    name: &str,
    identity: &CadSnapshotIdentity,
    prepared: boardstudio_model::model::PreparedCaseAssemblyIR,
) -> Result<CadResult, String> {
    let request_id = format!("mechanical-exact-{}-{name}", operation_id.0);
    let request = CadRequest {
        request_id,
        job_id: format!("mechanical-job-{}-{name}", operation_id.0),
        identity: identity.clone(),
        operation: CadOperation::Exact,
        prepared: Some(prepared),
        source_sha256: None,
        step_import_options: None,
        collision_query: None,
        generated_analysis_request: None,
        collision_meshes: None,
        input_bytes: Vec::new(),
    };
    let reply = worker
        .request(request.clone())
        .await
        .map_err(|error| format!("CAD mechanical geometry failed: {error}"))?;
    validate_reply(&request, reply, identity)
        .map_err(|error| format!("CAD mechanical geometry failed: {error:?}"))
}

pub(super) fn authored_case_filename(document: &ProjectDoc, scope: &Scope) -> String {
    let suffix = if document.boards.len() == 1 {
        String::new()
    } else {
        document
            .boards
            .iter()
            .find(|board| board.id == scope.board_id)
            .map(|board| format!("-{}", authored_case_filename_component(&board.name)))
            .unwrap_or_default()
    };
    format!("{}{suffix}-case.step", document.name)
}

fn authored_case_filename_component(value: &str) -> String {
    let mut result = String::with_capacity(value.len());
    let mut in_replacement = false;
    for character in value.chars() {
        if character.is_ascii_alphanumeric() || matches!(character, '_' | '.' | '-') {
            result.push(character);
            in_replacement = false;
        } else if !in_replacement {
            result.push('_');
            in_replacement = true;
        }
    }
    result
}
