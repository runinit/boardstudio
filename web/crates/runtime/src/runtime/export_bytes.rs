//! STEP, keycap STL and firmware artifact bytes.
use super::*;

impl Runtime {
    pub(super) async fn step_bytes(
        &self,
        operation_id: OperationId,
        snapshot: &AcceptedSnapshot,
        scope: &Scope,
    ) -> Result<Vec<u8>, String> {
        let guard = || self.require_export_current(operation_id, snapshot, scope);
        guard()?;
        let contours = snapshot
            .scene
            .board_contours
            .iter()
            .find(|entry| entry.board_id == scope.board_id)
            .map(|entry| entry.contours.as_slice())
            .unwrap_or_default();
        let bodies = snapshot
            .document
            .case_bodies
            .iter()
            .filter(|body| body.board_id == scope.board_id)
            .cloned()
            .map(|body| CaseIR {
                revision: snapshot.document.revision,
                body,
                contours: contours.to_vec(),
            })
            .collect::<Vec<_>>();
        if bodies.is_empty() {
            return Err("The selected board has no saved authored case bodies.".into());
        }
        let request_id = format!("authored-case-step-{}", operation_id.0);
        let request = CoreRequest::PrepareCase {
            id: request_id.clone(),
            ir: CaseAssemblyIR {
                revision: snapshot.document.revision,
                bodies,
            },
        };
        let prepared = (self.core_services.open_scratch)()(request);
        let prepared = match prepared {
            CoreReply::CasePrepared { id, ir } if id == request_id => ir,
            CoreReply::Error { id, message, .. } if id == request_id => return Err(message),
            _ => return Err("Core returned an unexpected authored case preparation.".into()),
        };
        if prepared.revision != snapshot.document.revision
            || prepared
                .bodies
                .iter()
                .any(|body| body.revision != snapshot.document.revision)
        {
            return Err("Core prepared authored case bodies for another revision.".into());
        }
        if prepared.bodies.is_empty() {
            return Err("Core prepared no authored case bodies.".into());
        }
        let identity = CadSnapshotIdentity {
            token: snapshot.token.0,
            session_epoch: snapshot.session_epoch.0,
            document_id: snapshot.document.id.clone(),
            board_id: scope.board_id.clone(),
            instance_id: scope.instance_id.clone(),
            revision: snapshot.document.revision,
        };
        guard()?;
        let worker = Rc::new(
            CadWorker::new(&resource_url("assets/cad-worker/entry.js")?)
                .map_err(|e| e.to_string())?,
        );
        self.export_workers
            .borrow_mut()
            .insert(operation_id, worker.clone());
        let result = async {
            worker.ready().await.map_err(|e| e.to_string())?;
            guard()?;
            let request = CadRequest {
                request_id: format!("authored-case-step-{}", operation_id.0),
                job_id: format!("authored-case-step-{}", operation_id.0),
                identity: identity.clone(),
                operation: CadOperation::ExportStep,
                prepared: Some(prepared),
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
                .map_err(|e| e.to_string())?;
            guard()?;
            validate_reply(&request, reply, &identity)
                .map(|result| result.step)
                .map_err(|e| format!("{e:?}"))
        }
        .await;
        worker.close();
        self.export_workers.borrow_mut().remove(&operation_id);
        result
    }

    pub(super) async fn keycaps_stl_bytes(
        &self,
        operation_id: OperationId,
        snapshot: &AcceptedSnapshot,
        scope: &Scope,
    ) -> Result<Vec<u8>, String> {
        let guard = || self.require_export_current(operation_id, snapshot, scope);
        guard()?;
        let resolution = self.resolve_case_keycaps_preview(
            scope.clone(),
            snapshot.token,
            snapshot.document.revision,
        )?;
        guard()?;
        let errors = resolution
            .findings
            .iter()
            .filter(|finding| finding.severity == boardstudio_model::model::Severity::Error)
            .map(|finding| finding.message.as_str())
            .collect::<Vec<_>>();
        if !errors.is_empty() {
            return Err(errors.join("\n"));
        }
        if resolution.specs.is_empty() {
            return Err("Choose a keycap profile before exporting keycap STLs.".into());
        }
        let identity = CadSnapshotIdentity {
            token: snapshot.token.0,
            session_epoch: snapshot.session_epoch.0,
            document_id: snapshot.document.id.clone(),
            board_id: scope.board_id.clone(),
            instance_id: scope.instance_id.clone(),
            revision: snapshot.document.revision,
        };
        let worker = Rc::new(
            CadWorker::new(&resource_url("assets/cad-worker/entry.js")?)
                .map_err(|error| error.to_string())?,
        );
        self.export_workers
            .borrow_mut()
            .insert(operation_id, worker.clone());
        let result = async {
            worker.ready().await.map_err(|error| error.to_string())?;
            guard()?;
            let result = worker
                .request_keycaps_print(
                    format!("keycaps-stl-{}", operation_id.0),
                    format!("keycaps-print-{}", operation_id.0),
                    identity,
                    resolution.specs,
                )
                .await
                .map_err(|error| error.to_string())?;
            guard()?;
            crate::keycap_package::files(result)
        }
        .await;
        worker.close();
        self.export_workers.borrow_mut().remove(&operation_id);
        let files = result?;
        guard()?;
        crate::portable_archive::pack_files(files)
    }

    pub(super) fn firmware_bytes(
        &self,
        operation_id: OperationId,
        snapshot: &AcceptedSnapshot,
        scope: &Scope,
    ) -> Result<Vec<u8>, String> {
        self.require_export_current(operation_id, snapshot, scope)?;
        let document = snapshot.document.as_ref();
        if !document
            .boards
            .iter()
            .any(|board| board.id == scope.board_id)
        {
            return Err("Select a board before export".into());
        }
        let hardware = document.hardware.as_ref();
        let instances = hardware
            .map(|hardware| hardware.instances.as_slice())
            .unwrap_or(&[]);
        let central = instances.iter().find(|instance| instance.role == "central");
        let peripheral = instances
            .iter()
            .find(|instance| instance.role == "peripheral");
        if hardware.is_some_and(|hardware| hardware.topology == HardwareTopology::Split)
            && (instances.len() != 2
                || central.is_none_or(|instance| instance.half != "left")
                || peripheral.is_none_or(|instance| instance.half != "right"))
        {
            return Err(
                "Split firmware requires a left central and a right peripheral assembly".into(),
            );
        }
        let primary_board_id = central.map_or(scope.board_id.as_str(), |instance| {
            instance.board_id.as_str()
        });
        let primary_instance_id = central.map_or(scope.instance_id.as_deref(), |instance| {
            Some(instance.id.as_str())
        });
        let primary = self.resolve_export_wiring(
            snapshot,
            primary_board_id,
            primary_instance_id,
            format!("firmware-electrical-{}-central", operation_id.0),
        )?;
        let secondary = peripheral
            .map(|instance| {
                self.resolve_export_wiring(
                    snapshot,
                    &instance.board_id,
                    Some(&instance.id),
                    format!("firmware-electrical-{}-peripheral", operation_id.0),
                )
            })
            .transpose()?;
        let (request, _) = crate::firmware_request_adapter::firmware_request(
            document,
            &primary,
            secondary.as_ref(),
        )?;
        let generate_id = format!("firmware-generate-{}", operation_id.0);
        let request = CoreRequest::GenerateFirmware {
            id: generate_id.clone(),
            request,
        };
        let generated = boardstudio_core::CoreEngine::new().handle(request);
        let package = match generated {
            CoreReply::FirmwareGenerated { id, package } if id == generate_id => package,
            CoreReply::Error { id, message, .. } if id == generate_id => return Err(message),
            CoreReply::FirmwareGenerated { .. } | CoreReply::Error { .. } => {
                return Err("Core returned a firmware reply for another request.".into());
            }
            _ => return Err("Core returned an unexpected firmware-generation reply.".into()),
        };
        let mut files = package.files;
        let mut plan_value = serde_json::Map::new();
        plan_value.insert(
            "central".into(),
            serde_json::to_value(&primary).map_err(|error| error.to_string())?,
        );
        if let Some(peripheral) = &secondary {
            plan_value.insert(
                "peripheral".into(),
                serde_json::to_value(peripheral).map_err(|error| error.to_string())?,
            );
        }
        files.insert(
            "electrical-plan.json".into(),
            serde_json::to_string_pretty(&serde_json::Value::Object(plan_value))
                .map_err(|error| format!("Could not serialize electrical plan: {error}"))?,
        );
        crate::portable_archive::pack_files(
            files
                .into_iter()
                .map(|(path, contents)| (path, contents.into_bytes())),
        )
    }

    pub(super) fn resolve_export_wiring(
        &self,
        snapshot: &AcceptedSnapshot,
        board_id: &str,
        instance_id: Option<&str>,
        request_id: String,
    ) -> Result<ElectricalPlan, String> {
        let configuration = snapshot.document.hardware.as_ref().and_then(|hardware| {
            hardware
                .boards
                .iter()
                .find(|configuration| configuration.board_id == board_id)
        });
        let controller_part_id = snapshot
            .document
            .hardware
            .as_ref()
            .and_then(|hardware| {
                hardware
                    .instances
                    .iter()
                    .find(|instance| Some(instance.id.as_str()) == instance_id)
            })
            .and_then(|instance| instance.controller_part_id.clone())
            .or_else(|| {
                configuration.and_then(|configuration| configuration.controller_part_id.clone())
            });
        let request = CoreRequest::ResolveElectrical {
            id: request_id.clone(),
            request: ElectricalPlanRequest {
                document: (*snapshot.document).clone(),
                instance_id: instance_id.map(str::to_owned),
                mode: configuration
                    .map_or(ElectricalMode::Matrix, |configuration| configuration.mode),
                locks: configuration.map_or_else(Default::default, |configuration| {
                    configuration.locks.clone()
                }),
                controller_profile: None,
                board_id: Some(board_id.to_owned()),
                controller_part_id,
            },
        };
        let reply = boardstudio_core::CoreEngine::new().handle(request);
        let plan = match reply {
            CoreReply::ElectricalResolved { id, plan } if id == request_id => plan,
            CoreReply::Error { id, message, .. } if id == request_id => return Err(message),
            CoreReply::ElectricalResolved { .. } | CoreReply::Error { .. } => {
                return Err("Core returned wiring for another request.".into());
            }
            _ => {
                return Err("Core returned an unexpected wiring reply.".into());
            }
        };
        if plan.revision != snapshot.document.revision
            || plan.board_id.as_deref() != Some(board_id)
            || plan.instance_id.as_deref() != instance_id
        {
            return Err("Core resolved wiring for another board, instance, or revision.".into());
        }
        Ok(plan)
    }
}
