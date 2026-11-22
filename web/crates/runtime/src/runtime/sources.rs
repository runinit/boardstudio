//! Source resolution for mechanical settings, electrical previews and keycap previews.
use super::*;

impl Runtime {
    /// Resolve mechanical settings against the exact accepted source and the proposed canonical
    /// document. The proposal carrier exists only for the existing effective-case projection;
    /// it is never installed in Session or any accepted/display authority.
    pub fn resolve_mechanical_settings(
        &self,
        accepted: AcceptedSnapshot,
        scope: Scope,
        proposed: ProjectDoc,
    ) -> Result<(MechanicalAssembly, MechanicalConfiguration), String> {
        self.ensure_keycaps_source_current(&accepted, &scope)?;
        if proposed.id != accepted.document.id
            || proposed.revision != accepted.document.revision
            || proposed.physical_instance_id != accepted.document.physical_instance_id
            || !mechanical_document_targets_scope(&proposed, &scope)
        {
            return Err(
                "The proposed mechanical settings do not match the accepted board scope.".into(),
            );
        }

        // This temporary carrier lets the established projection apply physical-instance
        // defaults (including wireless battery defaults) without granting it Session authority.
        let proposal_projection_input = AcceptedSnapshot {
            token: accepted.token,
            session_epoch: accepted.session_epoch,
            document: std::sync::Arc::new(proposed),
            scene: accepted.scene.clone(),
        };
        let effective_document = boardstudio_web_host::cad_jobs::captured_case_document(
            &proposal_projection_input,
            &scope,
        )
        .map_err(|error| format!("Could not project proposed mechanical settings: {error:?}"))?;
        drop(proposal_projection_input);
        let configuration = effective_document
            .mechanical
            .clone()
            .filter(|configuration| configuration.board_id == scope.board_id)
            .ok_or_else(|| {
                "The selected board has no effective mechanical configuration to resolve."
                    .to_owned()
            })?;
        if effective_document.physical_instance_id != scope.instance_id {
            return Err(
                "The effective mechanical document resolved another physical instance.".into(),
            );
        }

        // Physical contours come from the real accepted scene. Its projection performs the
        // required X reflection and winding reversal for flipped instances; never reflect twice.
        let contours = boardstudio_web_host::cad_jobs::captured_case_scene(&accepted, &scope)
            .map_err(|error| format!("Could not project accepted case contours: {error:?}"))?
            .board_contours
            .into_iter()
            .find(|board| board.board_id == scope.board_id)
            .map_or_else(Vec::new, |board| board.contours);

        let request_id = format!("mechanical-settings-{}", self.operation().0);
        let request = CoreRequest::ResolveMechanical {
            id: request_id.clone(),
            document: effective_document,
            contours,
        };
        let reply = (self.core_services.open_scratch)()(request);
        let assembly = match reply {
            CoreReply::MechanicalResolved { id, assembly } if id == request_id => assembly,
            CoreReply::Error { id, message, .. } if id == request_id => return Err(message),
            CoreReply::MechanicalResolved { .. } | CoreReply::Error { .. } => {
                return Err("Core returned a mechanical reply for another request.".into());
            }
            _ => return Err("Core returned an unexpected mechanical-resolution reply.".into()),
        };
        if assembly.revision != accepted.document.revision
            || assembly.case.revision != accepted.document.revision
        {
            return Err("Core resolved mechanical settings for another document revision.".into());
        }

        // Findings and empty closure results are valid resolver output. CAD readiness is an
        // independent export/generation gate and does not belong in this read-only resolver.
        Ok((assembly, configuration))
    }

    /// Wiring is board-scoped even when a physical instance is selected.
    pub fn resolve_electrical_preview(
        &self,
        accepted: AcceptedSnapshot,
        scope: Scope,
    ) -> Result<ElectricalPlan, String> {
        let board_scope = self.scope().map(|mut scope| {
            scope.instance_id = None;
            scope
        });
        if board_scope.as_ref() != Some(&scope)
            || self
                .model()
                .accepted
                .is_none_or(|current| current.token != accepted.token)
            || accepted.scene.revision != accepted.document.revision
        {
            return Err("The accepted PCB wiring source changed.".into());
        }
        let id = format!("pcb-electrical-{}", self.operation().0);
        let request = electrical_preview_request(&id, &accepted.document, &scope.board_id);
        match (self.core_services.open_scratch)()(request) {
            CoreReply::ElectricalResolved { plan, .. } => Ok(plan),
            CoreReply::Error { message, .. } => Err(message),
            _ => Err("Unexpected wiring resolution reply.".into()),
        }
    }

    /// Resolve keycap fit against one accepted canonical board and the matching prepared Case
    /// preview, if the current physical-instance preview belongs to this exact snapshot.
    pub fn resolve_keycaps_preview(
        &self,
        scope: Scope,
        token: SnapshotToken,
        revision: u64,
    ) -> Result<boardstudio_model::model::KeycapResolution, String> {
        self.resolve_keycaps_preview_source(scope, token, revision, false)
    }

    /// Case consumes the selected physical projection; Layout retains the canonical document.
    pub fn resolve_case_keycaps_preview(
        &self,
        scope: Scope,
        token: SnapshotToken,
        revision: u64,
    ) -> Result<boardstudio_model::model::KeycapResolution, String> {
        self.resolve_keycaps_preview_source(scope, token, revision, true)
    }

    fn resolve_keycaps_preview_source(
        &self,
        scope: Scope,
        token: SnapshotToken,
        revision: u64,
        physical_case: bool,
    ) -> Result<boardstudio_model::model::KeycapResolution, String> {
        let accepted = self
            .model()
            .accepted
            .ok_or_else(|| "The accepted Keycaps source is no longer open.".to_owned())?;
        if accepted.token != token || accepted.document.revision != revision {
            return Err("The accepted Keycaps source changed before resolution started.".into());
        }
        self.ensure_keycaps_source_current(&accepted, &scope)?;
        let case_preview = self.cad_scene().filter(|scene| {
            scene.exact
                && scene.scope == scope
                && scene.token == accepted.token
                && scene.snapshot.document.revision == accepted.document.revision
                && scene.prepared.revision == accepted.document.revision
        });
        let cases = case_preview.as_ref().map(|scene| scene.prepared.clone());
        let document = if physical_case {
            captured_case_document(&accepted, &scope)
                .map_err(|error| format!("Could not capture physical Case keycaps: {error:?}"))?
        } else {
            (*accepted.document).clone()
        };
        let request_id = format!("keycaps-fit-{}", self.operation().0);
        let request = CoreRequest::ResolveKeycaps {
            id: request_id.clone(),
            document,
            board_id: scope.board_id.clone(),
            cases,
        };
        let reply = (self.core_services.open_scratch)()(request);
        let result = match reply {
            CoreReply::KeycapsResolved { id, result } if id == request_id => result,
            CoreReply::Error { id, message, .. } if id == request_id => return Err(message),
            CoreReply::KeycapsResolved { .. } | CoreReply::Error { .. } => {
                return Err("Core returned a keycap fit reply for another request.".into());
            }
            _ => return Err("Core returned an unexpected keycap fit reply.".into()),
        };
        if result.revision != accepted.document.revision {
            return Err("Core resolved keycap fit for another document revision.".into());
        }
        Ok(result)
    }

    pub(super) fn ensure_keycaps_source_current(
        &self,
        accepted: &AcceptedSnapshot,
        scope: &Scope,
    ) -> Result<(), String> {
        if self.snapshot_current(accepted.token, scope)
            && accepted.scene.revision == accepted.document.revision
        {
            Ok(())
        } else {
            Err("The accepted preview source changed.".into())
        }
    }

    /// Build the mesh preview from the already accepted Core keycap resolution. This worker is
    /// independent from Case generation and STEP export so superseding a keycap view cannot
    /// cancel or publish through either owner.
    pub async fn request_keycaps_cad_preview(
        self: &Rc<Self>,
        input: KeycapsPreviewInput,
    ) -> Result<KeycapsCadPreview, String> {
        let generation = self
            .keycaps_preview_generation
            .get()
            .checked_add(1)
            .ok_or_else(|| "Keycap CAD preview identity exhausted".to_owned())?;
        self.keycaps_preview_generation.set(generation);
        if let Some((_, previous)) = self.keycaps_preview_worker.borrow_mut().take() {
            previous.close();
        }

        let accepted = self
            .model()
            .accepted
            .ok_or_else(|| "The accepted Keycaps source is no longer open.".to_owned())?;
        self.ensure_keycaps_source_current(&accepted, &input.scope)?;
        if accepted.token != input.token || accepted.document.revision != input.revision {
            return Err("The accepted Keycaps source changed before CAD preview started.".into());
        }
        let identity = CadSnapshotIdentity {
            token: accepted.token.0,
            session_epoch: accepted.session_epoch.0,
            document_id: accepted.document.id.clone(),
            board_id: input.scope.board_id.clone(),
            instance_id: input.scope.instance_id.clone(),
            revision: accepted.document.revision,
        };
        let current = || {
            self.ensure_keycaps_source_current(&accepted, &input.scope)?;
            if self.keycaps_preview_generation.get() != generation {
                return Err("Keycap CAD preview was cancelled or superseded.".into());
            }
            Ok::<(), String>(())
        };
        let preview = |bodies| KeycapsCadPreview {
            generation,
            scope: input.scope.clone(),
            token: input.token,
            revision: input.revision,
            specs: input.specs.clone(),
            bodies,
        };
        #[cfg(any(test, feature = "test-support"))]
        let executor = self.keycaps_preview_test_executor.borrow().clone();
        #[cfg(any(test, feature = "test-support"))]
        if let Some(executor) = executor {
            let bodies = executor(input.clone()).await?;
            current()?;
            return Ok(preview(bodies));
        }
        let worker = Rc::new(
            CadWorker::new(&resource_url("assets/cad-worker/entry.js")?)
                .map_err(|error| error.to_string())?,
        );
        *self.keycaps_preview_worker.borrow_mut() = Some((generation, worker.clone()));
        let result = async {
            current()?;
            worker.ready().await.map_err(|error| error.to_string())?;
            current()?;
            let request_id = format!("keycaps-preview-{generation}");
            let job_id = format!("keycaps-preview-job-{generation}");
            let result = worker
                .request_keycaps_preview(request_id, job_id, identity, input.specs.clone())
                .await
                .map_err(|error| error.to_string())?;
            current()?;
            Ok(preview(result.bodies))
        }
        .await;
        if self
            .keycaps_preview_worker
            .borrow()
            .as_ref()
            .is_some_and(|(active, owner)| *active == generation && Rc::ptr_eq(owner, &worker))
        {
            self.keycaps_preview_worker.borrow_mut().take();
        }
        worker.close();
        result
    }

    /// Retire the active preview worker on source change or viewer unmount.
    pub fn cancel_keycaps_cad_preview(&self) {
        self.keycaps_preview_generation
            .set(self.keycaps_preview_generation.get().saturating_add(1));
        if let Some((_, worker)) = self.keycaps_preview_worker.borrow_mut().take() {
            worker.close();
        }
    }
}

fn mechanical_document_targets_scope(document: &ProjectDoc, scope: &Scope) -> bool {
    if !document
        .boards
        .iter()
        .any(|board| board.id == scope.board_id)
    {
        return false;
    }
    match scope.instance_id.as_deref() {
        None => true,
        Some(instance_id) => document.hardware.as_ref().is_some_and(|hardware| {
            hardware
                .instances
                .iter()
                .filter(|instance| {
                    instance.id == instance_id && instance.board_id == scope.board_id
                })
                .count()
                == 1
        }),
    }
}
