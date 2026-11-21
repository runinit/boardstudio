//! STEP, keycap STL and firmware artifact bytes.
use super::*;

impl Runtime {
    pub(super) async fn step_bytes(
        &self,
        operation_id: OperationId,
        snapshot: &AcceptedSnapshot,
        scope: &Scope,
    ) -> Result<Vec<u8>, String> {
        let core = self.core.borrow().clone();
        let executor_epoch = self.session.borrow().core_executor_epoch();
        let guard = || -> Result<(), String> {
            let current_core = self.core.borrow().clone();
            let current = self.model().accepted;
            if self.export_current(operation_id, snapshot.token, scope)
                && self.scope().as_ref() == Some(scope)
                && self.session.borrow().core_executor_epoch() == executor_epoch
                && Rc::ptr_eq(&core, &current_core)
                && current.as_ref().is_some_and(|accepted| {
                    accepted.token == snapshot.token
                        && accepted.session_epoch == snapshot.session_epoch
                        && accepted.document.id == snapshot.document.id
                        && accepted.document.revision == snapshot.document.revision
                        && accepted.scene.revision == snapshot.document.revision
                })
            {
                Ok(())
            } else {
                Err("Authored Case STEP was cancelled, superseded, or its source changed.".into())
            }
        };
        guard()?;
        let bodies = snapshot
            .document
            .case_bodies
            .iter()
            .filter(|body| body.board_id == scope.board_id)
            .cloned()
            .map(|body| CaseIR {
                revision: snapshot.document.revision,
                body,
                contours: snapshot
                    .scene
                    .board_contours
                    .iter()
                    .find(|entry| entry.board_id == scope.board_id)
                    .map_or_else(Vec::new, |entry| entry.contours.clone()),
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
        let prepared = core
            .request(&request_id, &executor_epoch.0.to_string(), &request)
            .await
            .map_err(|error| format!("Authored case preparation failed: {error}"))?;
        guard()?;
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
        let core = self.core.borrow().clone();
        let guard = || self.require_export_current(operation_id, snapshot, scope, Some(&core));
        guard()?;
        let resolution = self
            .resolve_case_keycaps_preview(scope.clone(), snapshot.token, snapshot.document.revision)
            .await?;
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
        let entries = files
            .iter()
            .enumerate()
            .map(|(index, (path, _))| {
                Ok(ArchiveEntry {
                    path: path.clone(),
                    buffer_index: u32::try_from(index)
                        .map_err(|_| "Too many keycap STL files.".to_owned())?,
                })
            })
            .collect::<Result<Vec<_>, String>>()?;
        let metadata = serde_json::to_string(&ArchiveRequest::PackFiles { entries })
            .map_err(|error| error.to_string())?;
        let buffers = files
            .iter()
            .map(|(_, bytes)| Uint8Array::from(bytes.as_slice()))
            .collect();
        guard()?;
        let epoch = self.session.borrow().core_executor_epoch().0.to_string();
        let packed = core
            .archive(
                &format!("keycaps-stl-archive-{}", operation_id.0),
                &epoch,
                &metadata,
                buffers,
            )
            .await
            .map_err(|error| format!("Keycap STL packaging failed: {error}"))?;
        guard()?;
        match serde_json::from_str::<ArchiveReply>(&packed.metadata)
            .map_err(|error| error.to_string())?
        {
            ArchiveReply::Packed if packed.buffers.len() == 1 => Ok(packed.buffers[0].to_vec()),
            _ => Err("Core returned an incomplete keycap STL archive.".into()),
        }
    }

    pub(super) async fn firmware_bytes(
        &self,
        operation_id: OperationId,
        snapshot: &AcceptedSnapshot,
        scope: &Scope,
    ) -> Result<Vec<u8>, String> {
        let document = snapshot.document.as_ref();
        self.require_export_current(operation_id, snapshot, scope, None)?;
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
        let (core, executor_epoch) = self.current_firmware_executor();
        self.require_export_current(operation_id, snapshot, scope, Some(&core))?;
        let primary = self
            .resolve_firmware_plan(
                operation_id,
                snapshot,
                scope,
                &core,
                executor_epoch,
                FirmwarePlanTarget {
                    board_id: primary_board_id,
                    instance_id: primary_instance_id,
                    label: "central",
                },
            )
            .await?;
        let secondary = if let Some(instance) = peripheral {
            Some(
                self.resolve_firmware_plan(
                    operation_id,
                    snapshot,
                    scope,
                    &core,
                    executor_epoch,
                    FirmwarePlanTarget {
                        board_id: &instance.board_id,
                        instance_id: Some(&instance.id),
                        label: "peripheral",
                    },
                )
                .await?,
            )
        } else {
            None
        };
        self.require_export_current(operation_id, snapshot, scope, Some(&core))?;
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
        let generated = core
            .request(&generate_id, &executor_epoch.0.to_string(), &request)
            .await;
        self.require_export_current(operation_id, snapshot, scope, Some(&core))?;
        let generated =
            generated.map_err(|error| format!("Firmware generation failed: {error}"))?;
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
        self.require_export_current(operation_id, snapshot, scope, Some(&core))?;
        let entries = files
            .keys()
            .enumerate()
            .map(|(index, path)| {
                Ok(ArchiveEntry {
                    path: path.clone(),
                    buffer_index: u32::try_from(index)
                        .map_err(|_| "Too many firmware package files".to_owned())?,
                })
            })
            .collect::<Result<Vec<_>, String>>()?;
        let metadata = serde_json::to_string(&ArchiveRequest::PackFiles { entries })
            .map_err(|error| format!("Could not prepare ZIP package request: {error}"))?;
        let buffers = files
            .values()
            .map(|contents| Uint8Array::from(contents.as_bytes()))
            .collect();
        let pack_id = format!("firmware-pack-{}", operation_id.0);
        let packed = core
            .archive(&pack_id, &executor_epoch.0.to_string(), &metadata, buffers)
            .await;
        self.require_export_current(operation_id, snapshot, scope, Some(&core))?;
        let packed = packed.map_err(|error| format!("Firmware packaging failed: {error}"))?;
        match serde_json::from_str::<ArchiveReply>(&packed.metadata)
            .map_err(|error| format!("Could not read firmware package result: {error}"))?
        {
            ArchiveReply::Packed => packed
                .buffers
                .first()
                .map(Uint8Array::to_vec)
                .ok_or_else(|| "Firmware package returned no ZIP bytes.".into()),
            ArchiveReply::Error { message } => Err(format!("Firmware packaging failed: {message}")),
            ArchiveReply::Unpacked { .. } => {
                Err("Core returned an unexpected unpacked archive.".into())
            }
        }
    }

    pub(super) async fn resolve_firmware_plan(
        &self,
        operation_id: OperationId,
        snapshot: &AcceptedSnapshot,
        scope: &Scope,
        core: &Rc<dyn CoreExecutor>,
        executor_epoch: boardstudio_application::ExecutorEpoch,
        target: FirmwarePlanTarget<'_>,
    ) -> Result<ElectricalPlan, String> {
        self.require_export_current(operation_id, snapshot, scope, Some(core))?;
        let configuration = snapshot.document.hardware.as_ref().and_then(|hardware| {
            hardware
                .boards
                .iter()
                .find(|configuration| configuration.board_id == target.board_id)
        });
        let controller_part_id = snapshot
            .document
            .hardware
            .as_ref()
            .and_then(|hardware| {
                hardware
                    .instances
                    .iter()
                    .find(|instance| Some(instance.id.as_str()) == target.instance_id)
            })
            .and_then(|instance| instance.controller_part_id.clone())
            .or_else(|| {
                configuration.and_then(|configuration| configuration.controller_part_id.clone())
            });
        let request_id = format!("firmware-electrical-{}-{}", operation_id.0, target.label);
        let request = CoreRequest::ResolveElectrical {
            id: request_id.clone(),
            request: ElectricalPlanRequest {
                document: (*snapshot.document).clone(),
                instance_id: target.instance_id.map(str::to_owned),
                mode: configuration
                    .map_or(ElectricalMode::Matrix, |configuration| configuration.mode),
                locks: configuration.map_or_else(Default::default, |configuration| {
                    configuration.locks.clone()
                }),
                controller_profile: None,
                board_id: Some(target.board_id.to_owned()),
                controller_part_id,
            },
        };
        let reply = core
            .request(&request_id, &executor_epoch.0.to_string(), &request)
            .await;
        self.require_export_current(operation_id, snapshot, scope, Some(core))?;
        let reply = reply.map_err(|error| format!("Wiring resolution failed: {error}"))?;
        let plan = match reply {
            CoreReply::ElectricalResolved { id, plan } if id == request_id => plan,
            CoreReply::Error { id, message, .. } if id == request_id => return Err(message),
            CoreReply::ElectricalResolved { .. } | CoreReply::Error { .. } => {
                return Err("Core returned a wiring reply for another firmware plan.".into());
            }
            _ => {
                return Err("Core returned an unexpected firmware wiring-resolution reply.".into());
            }
        };
        if plan.revision != snapshot.document.revision
            || plan.board_id.as_deref() != Some(target.board_id)
            || plan.instance_id.as_deref() != target.instance_id
        {
            return Err(
                "Core resolved firmware wiring for another board, instance, or revision.".into(),
            );
        }
        Ok(plan)
    }
}
