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
        {
            return Err(
                "The proposed mechanical settings do not match the accepted board scope.".into(),
            );
        }

        if let Some(id) = &scope.instance_id
            && proposed
                .hardware
                .iter()
                .flat_map(|hardware| &hardware.instances)
                .filter(|instance| instance.id == *id)
                .count()
                > 1
        {
            return Err("The proposed mechanical instance is ambiguous.".into());
        }
        // This temporary carrier lets the established projection apply physical-instance
        // defaults (including wireless battery defaults) without granting it Session authority.
        let proposal_projection_input = AcceptedSnapshot {
            document: std::sync::Arc::new(proposed),
            ..accepted.clone()
        };
        let effective_document = boardstudio_web_host::cad_jobs::captured_case_document(
            &proposal_projection_input,
            &scope,
        )
        .map_err(|error| format!("Could not project proposed mechanical settings: {error:?}"))?;
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

        let assembly = boardstudio_core::mechanical::resolve(&effective_document, &contours);
        Ok((assembly, configuration))
    }

    /// Wiring is board-scoped even when a physical instance is selected.
    pub fn resolve_electrical_preview(
        &self,
        accepted: AcceptedSnapshot,
        scope: Scope,
    ) -> Result<ElectricalPlan, String> {
        let mut active_scope = scope.clone();
        active_scope.instance_id = self.with_model(|model| model.active_instance_id.clone());
        if scope.instance_id.is_some()
            || !self.snapshot_current(accepted.token, &active_scope)
            || accepted.scene.revision != accepted.document.revision
        {
            return Err("The accepted PCB wiring source changed.".into());
        }
        self.resolve_export_wiring(&accepted, &scope.board_id, None)
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
        let accepted = self.keycaps_snapshot(&scope, token, revision)?;
        let cases = self
            .cad_scene()
            .filter(|scene| {
                scene.exact
                    && scene.scope == scope
                    && scene.token == token
                    && scene.prepared.revision == revision
            })
            .map(|scene| scene.prepared.clone());
        let document = if physical_case {
            captured_case_document(&accepted, &scope)
                .map_err(|error| format!("Could not capture physical Case keycaps: {error:?}"))?
        } else {
            (*accepted.document).clone()
        };
        let request = CoreRequest::ResolveKeycaps {
            id: format!("keycaps-fit-{}", self.operation().0),
            document,
            board_id: scope.board_id.clone(),
            cases,
        };
        let reply = (self.core_services.open_scratch)()(request);
        let result = match reply {
            CoreReply::KeycapsResolved { result, .. } => result,
            CoreReply::Error { message, .. } => return Err(message),
            _ => return Err("Core returned an unexpected keycap fit reply.".into()),
        };
        if result.revision != accepted.document.revision {
            return Err("Core resolved keycap fit for another document revision.".into());
        }
        Ok(result)
    }

    fn keycaps_snapshot(
        &self,
        scope: &Scope,
        token: SnapshotToken,
        revision: u64,
    ) -> Result<AcceptedSnapshot, String> {
        self.accepted_snapshot()
            .filter(|accepted| {
                accepted.token == token
                    && accepted.document.revision == revision
                    && self.ensure_keycaps_source_current(accepted, scope).is_ok()
            })
            .ok_or_else(|| "The accepted Keycaps source changed.".into())
    }

    pub(super) fn ensure_keycaps_source_current(
        &self,
        accepted: &AcceptedSnapshot,
        scope: &Scope,
    ) -> Result<(), String> {
        (self.snapshot_current(accepted.token, scope)
            && accepted.scene.revision == accepted.document.revision)
            .then_some(())
            .ok_or_else(|| "The accepted preview source changed.".into())
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
        self.close_keycaps_preview_worker();

        let accepted = self.keycaps_snapshot(&input.scope, input.token, input.revision)?;
        let identity = super::export_bytes::snapshot_identity(&accepted, &input.scope);
        let current = || {
            if self.keycaps_preview_generation.get() != generation {
                return Err("Keycap CAD preview was cancelled or superseded.".into());
            }
            self.ensure_keycaps_source_current(&accepted, &input.scope)
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
        let executor = self.test.keycaps_preview_test_executor.borrow().clone();
        #[cfg(any(test, feature = "test-support"))]
        if let Some(executor) = executor {
            let bodies = executor(input.clone()).await?;
            current()?;
            return Ok(preview(bodies));
        }
        let worker = new_cad_worker()?;
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
        self.keycaps_preview_worker
            .borrow_mut()
            .take_if(|(active, _)| *active == generation);
        worker.close();
        result
    }

    /// Retire the active preview worker on source change or viewer unmount.
    pub fn cancel_keycaps_cad_preview(&self) {
        self.keycaps_preview_generation
            .set(self.keycaps_preview_generation.get().saturating_add(1));
        self.close_keycaps_preview_worker();
    }

    fn close_keycaps_preview_worker(&self) {
        if let Some((_, worker)) = self.keycaps_preview_worker.borrow_mut().take() {
            worker.close();
        }
    }
}
