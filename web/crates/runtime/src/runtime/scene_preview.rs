//! The CAD scene, Case gesture preview, native Case preview and parts library preview.
use super::*;

impl Runtime {
    pub fn cad_scene(&self) -> Option<Rc<CadScene>> {
        let current_scope = self.scope()?;
        let accepted = self.accepted_snapshot()?;
        crate::case_generation_lifecycle::current_cad_scene(
            &mut self.cad_scene.borrow_mut(),
            &current_scope,
            &accepted,
        )
    }

    pub fn case_gesture_preview_scene(&self, source: &CadScene) -> Option<Rc<CadScene>> {
        let owner = self.case_gesture_preview.borrow().active_owner()?;
        if !self.case_gesture_preview_is_current(&owner)
            || owner.scope != source.scope
            || owner.snapshot_token != source.token
            || owner.revision != source.snapshot.document.revision
        {
            return None;
        }
        self.case_gesture_preview
            .borrow()
            .scene(&owner.scope, owner.snapshot_token, owner.revision)
    }

    pub fn case_gesture_preview_message(&self) -> Option<String> {
        let owner = self.case_gesture_preview.borrow().active_owner()?;
        self.case_gesture_preview_is_current(&owner)
            .then(|| self.case_gesture_preview.borrow().message(&owner))
            .flatten()
    }

    /// Start a disposable preview of one Case gesture draft. It uses the
    /// accepted scene and snapshot identity, but only the Case CAD preview
    /// result is overlaid; it never replaces the accepted generation cache.
    pub fn update_case_gesture_preview(
        self: &Rc<Self>,
        scope: Scope,
        token: SnapshotToken,
        revision: u64,
        draft_document: ProjectDoc,
    ) -> Option<crate::case_gesture_preview::CaseGesturePreviewOwner> {
        let accepted = self.accepted_snapshot()?;
        if !self.snapshot_current(token, &scope)
            || accepted.document.revision != revision
            || draft_document.id != accepted.document.id
            || draft_document.revision != revision
            || accepted.scene.revision != revision
        {
            return None;
        }
        // A newer pointer sample replaces the pending owner while retaining
        // the last accepted provisional scene for display. Retiring only the
        // worker job here also makes its eventual reply stale without ending
        // the gesture overlay itself.
        if let Some(previous) = self.case_gesture_preview.borrow().active_owner() {
            self.cancel_case_gesture_preview_job(&previous);
        }
        let owner = self
            .case_gesture_preview
            .borrow_mut()
            .begin(scope.clone(), token, revision)
            .ok()?;
        let job_id = format!("case-gesture-{}-{}-{}", token.0, revision, owner.generation);
        *self.case_gesture_preview_job.borrow_mut() = Some((owner.clone(), job_id.clone()));
        let mut draft_snapshot = accepted;
        draft_snapshot.document = std::sync::Arc::new(draft_document);
        self.changed();
        let runtime = Rc::downgrade(self);
        let worker_job_id = job_id.clone();
        let task_owner = owner.clone();
        spawn_local(async move {
            let Some(runtime) = runtime.upgrade() else {
                return;
            };
            let result = runtime
                .prepare_case_gesture_preview(
                    task_owner.clone(),
                    draft_snapshot,
                    worker_job_id.clone(),
                )
                .await;
            if runtime
                .case_gesture_preview_job
                .borrow()
                .as_ref()
                .is_some_and(|(active, active_job)| {
                    active == &task_owner && active_job == &worker_job_id
                })
            {
                runtime.case_gesture_preview_job.borrow_mut().take();
            }
            match result {
                Ok(scene) if runtime.case_gesture_preview_is_current(&task_owner) => {
                    runtime
                        .case_gesture_preview
                        .borrow_mut()
                        .publish(&task_owner, Rc::new(scene));
                    runtime.changed();
                }
                Err(error) if runtime.case_gesture_preview_is_current(&task_owner) => {
                    runtime
                        .case_gesture_preview
                        .borrow_mut()
                        .fail(&task_owner, error);
                    runtime.changed();
                }
                _ => {}
            }
        });
        Some(owner)
    }

    pub(super) async fn prepare_case_gesture_preview(
        self: &Rc<Self>,
        owner: crate::case_gesture_preview::CaseGesturePreviewOwner,
        snapshot: AcceptedSnapshot,
        job_id: String,
    ) -> Result<CadScene, String> {
        let is_current = || self.case_gesture_preview_is_current(&owner);
        if !is_current() {
            return Err("Case gesture preview was superseded".into());
        }
        // Coalesce rapid pointer samples. A newer move advances the owner while
        // this delay is pending, so only the latest draft reaches Core/CAD.
        TimeoutFuture::new(45).await;
        if !is_current() {
            return Err("Case gesture preview was superseded".into());
        }
        let prepared = prepare_captured_case(&snapshot, &owner.scope, false)
            .map_err(|error| format!("Case preview preparation failed: {error:?}"))?;
        if !is_current() {
            return Err("Case gesture preview was superseded".into());
        }
        let worker = self.case_cad_worker(&owner.scope, true)?;
        worker.ready().await.map_err(|error| error.to_string())?;
        if !is_current() {
            let _ = worker.cancel(&job_id);
            return Err("Case gesture preview was superseded".into());
        }
        let request = prepared_cad_request(
            format!("{job_id}-preview"),
            job_id,
            &prepared.identity,
            CadOperation::Preview,
            prepared.prepared.clone(),
        );
        let reply = worker
            .request(request.clone())
            .await
            .map_err(|error| error.to_string())?;
        if !is_current() {
            return Err("Case gesture preview was superseded".into());
        }
        let mut result = validate_reply(&request, reply, &prepared.identity)
            .map_err(|error| format!("Case preview CAD failed: {error:?}"))?;
        result.step.clear();
        let contours = captured_board_contours(&snapshot, &owner.scope)
            .map_err(|error| format!("Case preview contours failed: {error:?}"))?
            .ok_or_else(|| "Case preview contours are unavailable".to_owned())?;
        Ok(CadScene {
            scope: owner.scope.clone(),
            token: owner.snapshot_token,
            snapshot,
            result,
            prepared: prepared.prepared,
            physical_fingerprint: None,
            mechanical: prepared.mechanical_assembly,
            exact: false,
            contours,
        })
    }

    pub fn native_case_preview(&self) -> Option<Rc<crate::case_preview::NativePreviewSnapshot>> {
        self.native_case_preview.published().filter(|preview| {
            preview.lease.matches(&preview.owner)
                && self.snapshot_current(preview.owner.snapshot_token, &preview.owner.scope)
        })
    }

    pub fn native_case_preview_pending(&self) -> bool {
        self.native_case_preview.pending().is_some_and(|lease| {
            lease.is_active() && self.preview_owner_is_current(lease.identity())
        })
    }

    pub fn native_case_preview_error(&self) -> Option<String> {
        self.native_case_preview
            .current_error(|owner| self.preview_owner_is_current(owner))
    }

    pub fn native_case_preview_key(&self) -> Option<(Scope, SnapshotToken, u64)> {
        let scope = self.scope()?;
        let accepted = self.accepted_snapshot()?;
        (scope.session_epoch == accepted.session_epoch
            && scope.document_id == accepted.document.id
            && accepted.scene.revision == accepted.document.revision)
            .then_some((scope, accepted.token, accepted.document.revision))
    }

    pub fn native_model_delivery(
        &self,
        preview: &crate::case_preview::NativePreviewSnapshot,
    ) -> Option<crate::model_delivery::ModelDeliveryRows> {
        self.native_model_delivery
            .borrow()
            .published
            .as_ref()
            .filter(|(owner, _)| owner == &preview.owner && preview.lease.matches(owner))
            .map(|(_, rows)| rows.clone())
    }

    /// Decode all models for this exact accepted native preview. The published
    /// rows replace the pending state only after the batch settles, preserving
    /// React's board-first / Promise.all success behavior.
    pub async fn deliver_native_case_models(
        self: &Rc<Self>,
        preview: Rc<crate::case_preview::NativePreviewSnapshot>,
    ) -> Result<(), String> {
        if !self.native_preview_snapshot_is_current(&preview) {
            return Ok(());
        }
        let source_is_current = {
            let weak = Rc::downgrade(self);
            let preview = preview.clone();
            Rc::new(move || {
                weak.upgrade()
                    .is_some_and(|runtime| runtime.native_preview_snapshot_is_current(&preview))
            }) as Rc<dyn Fn() -> bool>
        };
        let batch = self.preview_model_batch(
            &preview.owner.scope,
            preview.owner.snapshot_token,
            preview.accepted_document.revision,
            preview.lease.clone(),
            source_is_current,
        );
        let Some(bundled_ids_by_path) = resolve_native_model_paths(
            &self.native_model_delivery,
            &preview.owner,
            unique_model_paths(&preview.preview.models),
            || self.native_preview_snapshot_is_current(&preview),
            || self.changed(),
            |paths| async move {
                crate::bundled_models::generated_model_asset_ids_for_paths(&paths).await
            },
        )
        .await?
        else {
            return Ok(());
        };
        let results = batch
            .deliver_board_models(
                &preview.preview.models,
                preview.board_reference.as_ref(),
                &preview.path_assets,
                &preview.accepted_document,
                &bundled_ids_by_path,
            )
            .await;
        let mut state = self.native_model_delivery.borrow_mut();
        if state.pending.as_ref() == Some(&preview.owner) {
            state.pending = None;
        }
        let published = if let Some(rows) = results
            && self.native_preview_snapshot_is_current(&preview)
        {
            state.published = Some((preview.owner.clone(), rows));
            true
        } else {
            false
        };
        drop(state);
        if published {
            self.changed();
        }
        Ok(())
    }

    pub(super) fn native_preview_snapshot_is_current(
        &self,
        expected: &crate::case_preview::NativePreviewSnapshot,
    ) -> bool {
        self.native_case_preview().is_some_and(|current| {
            current.owner == expected.owner && Rc::ptr_eq(&current.lease, &expected.lease)
        })
    }

    pub(super) fn model_delivery_ports(
        self: &Rc<Self>,
        scope: Scope,
        token: SnapshotToken,
        revision: u64,
        is_current: Rc<dyn Fn() -> bool>,
    ) -> crate::model_delivery::ModelDeliveryPorts {
        use crate::model_delivery::{
            MeshArrays, ModelAssetSource, ModelDeliveryPorts, ModelFuture, ResolvedModelAsset,
            VerifiedModelBytes,
        };
        let weak = Rc::downgrade(self);
        let scope_for_load = scope.clone();
        let current_for_load = is_current.clone();
        let load_verified_bytes = Rc::new(
            move |asset: ResolvedModelAsset| -> ModelFuture<Option<VerifiedModelBytes>> {
                let weak = weak.clone();
                let scope = scope_for_load.clone();
                let is_current = current_for_load.clone();
                Box::pin(async move {
                    let sha256 = asset.sha256.clone();
                    let runtime = weak
                        .upgrade()
                        .ok_or_else(|| "Model runtime was closed".to_owned())?;
                    if !is_current() {
                        return Err("Model asset request became stale before loading".into());
                    }
                    let packaged = matches!(asset.source, ModelAssetSource::Packaged { .. });
                    let mut cache_packaged = false;
                    let bytes = if packaged {
                        let cached = runtime.assets.borrow().get(&sha256).cloned();
                        if cached.is_some() {
                            cached
                        } else {
                            if packaged_model_asset(&asset.id).as_ref() != Some(&asset) {
                                return Err(
                                    "Packaged model descriptor no longer matches the catalogue"
                                        .into(),
                                );
                            }
                            cache_packaged = true;
                            Some(crate::bundled_models::bundled_model_bytes(&asset.id).await?)
                        }
                    } else {
                        runtime.load_document_asset(&sha256).await?
                    };
                    if !is_current() || !runtime.snapshot_current(token, &scope) {
                        return Err("Model asset request became stale after loading".into());
                    }
                    let Some(bytes) = bytes else {
                        return Ok(None);
                    };
                    let verified = VerifiedModelBytes::verify(bytes.clone(), &sha256)?;
                    let verified = match asset.source {
                        ModelAssetSource::Packaged { .. } => verified.with_packaged_source(),
                        ModelAssetSource::Document => verified,
                    };
                    if cache_packaged {
                        runtime.assets.borrow_mut().insert(sha256, bytes);
                    }
                    Ok(Some(verified))
                })
            },
        );
        let decode_stl = Rc::new(|bytes: VerifiedModelBytes| -> ModelFuture<MeshArrays> {
            Box::pin(
                async move { crate::renderer_host_page::decode_stl(bytes.bytes().to_vec()).await },
            )
        });
        let decode_wrl = Rc::new(|bytes: VerifiedModelBytes| -> ModelFuture<MeshArrays> {
            Box::pin(
                async move { crate::renderer_host_page::decode_wrl(bytes.bytes().to_vec()).await },
            )
        });
        let weak = Rc::downgrade(self);
        let read_step = Rc::new(
            move |bytes: VerifiedModelBytes,
                  owner: crate::model_delivery::ModelOwnerIdentity|
                  -> ModelFuture<MeshArrays> {
                let weak = weak.clone();
                let scope = scope.clone();
                let is_current = is_current.clone();
                Box::pin(async move {
                    if !owner.is_current(token) || !is_current() {
                        return Err("STEP model request became stale before reading".into());
                    }
                    let runtime = weak
                        .upgrade()
                        .ok_or_else(|| "Model runtime was closed".to_owned())?;
                    runtime
                        .read_step_model(
                            bytes,
                            Default::default(),
                            StepReadContext {
                                scope,
                                token,
                                revision,
                                is_current,
                                collision: false,
                            },
                        )
                        .await
                })
            },
        );
        ModelDeliveryPorts {
            load_verified_bytes,
            decode_stl,
            decode_wrl,
            read_step,
        }
    }

    pub async fn prepare_native_case_preview(
        self: &Rc<Self>,
        expected_scope: Scope,
        expected_token: SnapshotToken,
        expected_revision: u64,
    ) -> Result<(), String> {
        let accepted = self
            .accepted_snapshot()
            .ok_or_else(|| "No accepted project is available for Case preview".to_owned())?;
        if !self.snapshot_current(expected_token, &expected_scope)
            || accepted.document.revision != expected_revision
        {
            return Err("The accepted Case preview source changed before preparation".into());
        }
        if self.native_case_preview().is_some() {
            return Ok(());
        }
        if self.native_case_preview_pending() {
            return Ok(());
        }

        let generation = self
            .native_case_preview
            .next_generation("Case preview generation identity exhausted")?;
        let operation = self.operation().0;
        if operation == 0 || operation > MAX_SAFE_INTEGER {
            return Err("Case preview request identity is outside the safe integer range".into());
        }
        let request_token = format!(
            "case-preview-{}-{}-{}",
            expected_token.0, expected_revision, operation
        );
        let capture = match crate::case_preview::capture_native_preview(
            &accepted,
            &expected_scope,
            generation,
            request_token.clone(),
        ) {
            Ok(capture) => capture,
            Err(error) => {
                let owner = crate::case_preview::CasePreviewOwnerIdentity::capture(
                    &accepted,
                    &expected_scope,
                    generation,
                    request_token,
                );
                self.native_case_preview
                    .fail_before_begin(owner, error.clone());
                self.changed();
                return Err(error);
            }
        };
        self.set_native_preview_pending(capture.lease.clone());
        let result = self
            .run_native_case_preview(capture.clone(), operation)
            .await;
        match result {
            Ok(preview) if self.preview_owner_is_current(&preview.owner) => {
                let generation = preview.owner.projection_generation;
                self.native_case_preview.publish(
                    preview,
                    generation,
                    "The Case preview owner was cancelled before publication",
                )?;
                self.changed();
                Ok(())
            }
            Ok(preview) => {
                preview.lease.invalidate();
                Err("The Case preview result became stale before publication".into())
            }
            Err(error) => {
                let is_current = self.preview_owner_is_current(&capture.owner);
                capture.lease.invalidate();
                if is_current {
                    self.native_case_preview.fail(&capture.lease, error.clone());
                    self.report(format!("Case board preview failed: {error}"));
                }
                Err(error)
            }
        }
    }

    /// Prepare a Parts-only disposable sample through the existing Core,
    /// generator worker, and model-delivery ports. `capture` is owned by the
    /// mounted Parts preview and becomes stale as soon as that selection or
    /// accepted project changes.
    pub async fn prepare_parts_library_preview(
        self: &Rc<Self>,
        capture: crate::parts_preview::PartsPreviewCapture,
    ) -> Result<crate::parts_preview::PartsPreviewSnapshot, String> {
        if !self.parts_preview_capture_is_current(&capture) {
            capture.lease.invalidate();
            return Err("Parts preview source changed before preparation".into());
        }
        let operation = self.operation().0;
        if operation == 0 || operation > MAX_SAFE_INTEGER {
            capture.lease.invalidate();
            return Err(
                "Parts preview operation identity is outside the safe integer range".into(),
            );
        }
        let current_capture = capture.clone();
        let weak = Rc::downgrade(self);
        let is_current: Rc<dyn Fn() -> bool> = Rc::new(move || {
            weak.upgrade()
                .is_some_and(|runtime| runtime.parts_preview_capture_is_current(&current_capture))
        });
        let preview = self.run_preview_pipeline(
            capture.request.clone(),
            operation,
            capture.sample_scope.clone(),
            capture.owner.source_generation,
            is_current.clone(),
        )?;
        if !is_current() {
            capture.lease.invalidate();
            return Err("Parts preview became stale after geometry preparation".into());
        }

        let bundled_ids_by_path = bundled_preview_paths(&preview.models).await?;
        if !is_current() {
            capture.lease.invalidate();
            return Err("Parts model resolution became stale".into());
        }
        let batch = self.preview_model_batch(
            &capture.owner.scope,
            capture.owner.snapshot_token,
            capture.sample_document.revision,
            capture.lease.clone(),
            is_current.clone(),
        );
        let model_rows = batch
            .deliver_board_models(
                &preview.models,
                None,
                &capture.path_assets,
                &capture.sample_document,
                &bundled_ids_by_path,
            )
            .await;
        if !is_current() {
            capture.lease.invalidate();
            return Err("Parts model delivery became stale before publication".into());
        }
        let model_rows =
            model_rows.ok_or_else(|| "Parts model delivery was superseded".to_owned())?;
        capture.accept_preview(preview, Some(model_rows))
    }

    pub(super) fn parts_preview_capture_is_current(
        &self,
        capture: &crate::parts_preview::PartsPreviewCapture,
    ) -> bool {
        capture.lease.matches(&capture.owner)
            && self.snapshot_current(capture.owner.snapshot_token, &capture.owner.scope)
    }

    pub fn parts_preview_snapshot_is_current(
        &self,
        preview: &crate::parts_preview::PartsPreviewSnapshot,
    ) -> bool {
        preview.lease.matches(&preview.owner)
            && self.snapshot_current(preview.owner.snapshot_token, &preview.owner.scope)
    }

    pub(super) fn set_native_preview_pending(
        &self,
        lease: Rc<crate::case_preview::CasePreviewOwnerLease>,
    ) {
        self.cancel_native_model_jobs();
        self.native_case_preview.begin(lease);
        self.changed();
    }

    pub(super) fn preview_owner_is_current(
        &self,
        owner: &crate::case_preview::CasePreviewOwnerIdentity,
    ) -> bool {
        self.snapshot_current(owner.snapshot_token, &owner.scope)
            && self.native_case_preview.generation() == owner.projection_generation
    }

    pub(super) fn preview_owner_lease_is_current(
        &self,
        owner: &crate::case_preview::CasePreviewOwnerIdentity,
    ) -> bool {
        self.native_case_preview.owns(owner)
    }

    pub(super) async fn run_native_case_preview(
        self: &Rc<Self>,
        capture: crate::case_preview::NativePreviewCapture,
        operation: u64,
    ) -> Result<crate::case_preview::NativePreviewSnapshot, String> {
        let owner = capture.owner.clone();
        let lease = capture.lease.clone();
        let weak = Rc::downgrade(self);
        let is_current: Rc<dyn Fn() -> bool> = Rc::new(move || {
            weak.upgrade().is_some_and(|runtime| {
                runtime.preview_owner_is_current(&owner)
                    && runtime.preview_owner_lease_is_current(&owner)
                    && lease.matches(&owner)
            })
        });
        let preview = match &capture.request {
            crate::case_preview::CasePreviewRequest::Authored(request) => self
                .run_preview_pipeline(
                    request.as_ref().clone(),
                    operation,
                    capture.owner.scope.clone(),
                    capture.owner.projection_generation,
                    is_current,
                )?,
            crate::case_preview::CasePreviewRequest::Imported { asset, .. } => {
                let request_id = format!("case-preview-{operation}-imported");
                self.run_imported_board_preview(asset, request_id.clone(), is_current, |source| {
                    capture.imported_artifact_request(request_id, source)
                })
                .await?
            }
        };
        crate::case_preview::accept_native_preview(capture, preview)
    }

    /// One Core request shared by canonical Layout
    /// and the physical Case producer. Source construction and liveness stay with
    /// each workflow; this method owns only the established preview pipeline.
    pub(super) fn run_preview_pipeline(
        &self,
        request: PrepareExportRequest,
        operation: u64,
        scope: Scope,
        source_generation: u64,
        is_current: Rc<dyn Fn() -> bool>,
    ) -> Result<PcbPreview, String> {
        if source_generation == 0 || source_generation > MAX_SAFE_INTEGER {
            return Err("Preview source generation is outside the safe integer range".into());
        }
        if request.expected_revision != request.document.revision
            || request.target
                != (boardstudio_model::model::ExportTarget::Board {
                    board_id: scope.board_id.clone(),
                })
        {
            return Err("Preview request does not match its captured board source".into());
        }
        if !is_current() {
            return Err("Accepted board preview source became stale".into());
        }
        let preview_id = format!("board-preview-{operation}");
        let preview = self.board_preview(
            &ArtifactRequest::PreviewPcb {
                id: preview_id.clone(),
                request: request.clone(),
            },
            &preview_id,
        )?;
        if preview.revision != request.expected_revision {
            return Err("Core returned a board preview for another revision".into());
        }
        Ok(preview)
    }
}
