//! STEP model reads, collision subject queries and generated-body analysis.
use super::*;

impl Runtime {
    async fn model_worker(&self, scope: &Scope) -> Result<Rc<CadWorker>, String> {
        let worker = crate::case_model_lifecycle::case_model_worker(
            &self.cad_worker,
            scope,
            |worker| !worker.is_closed(),
            || {
                CadWorker::new(&resource_url("assets/cad-worker/entry.js")?)
                    .map(Rc::new)
                    .map_err(|error| error.to_string())
            },
        )?;
        worker.ready().await.map_err(|error| error.to_string())?;
        Ok(worker)
    }

    pub(super) async fn read_step_model(
        &self,
        bytes: crate::model_delivery::VerifiedModelBytes,
        import_options: boardstudio_web_host::cad_jobs::CadStepImportOptions,
        context: StepReadContext,
    ) -> Result<crate::model_delivery::MeshArrays, String> {
        let StepReadContext {
            scope,
            token,
            revision,
            is_current,
            collision,
        } = context;
        if !is_current() || self.scope().as_ref() != Some(&scope) {
            return Err("STEP model request became stale before worker setup".into());
        }
        let worker = self.model_worker(&scope).await?;
        if !is_current() || self.scope().as_ref() != Some(&scope) {
            return Err("STEP model request became stale before dispatch".into());
        }
        let operation = self.operation().0;
        let identity = CadSnapshotIdentity {
            token: token.0,
            session_epoch: scope.session_epoch.0,
            document_id: scope.document_id.clone(),
            board_id: scope.board_id.clone(),
            instance_id: scope.instance_id.clone(),
            revision,
        };
        let prefix = if collision {
            "case-model-evidence-step"
        } else if import_options.target_deflection_mm.is_some() {
            "case-model-refine-step"
        } else {
            "case-model-step"
        };
        let request = CadRequest {
            request_id: format!("{prefix}-{revision}-{operation}"),
            job_id: format!("{prefix}-{revision}-{operation}"),
            identity: identity.clone(),
            operation: CadOperation::ReadStep,
            prepared: None,
            source_sha256: Some(bytes.sha256().to_owned()),
            step_import_options: Some(import_options),
            collision_query: None,
            generated_analysis_request: None,
            collision_meshes: None,
            input_bytes: bytes.bytes().to_vec(),
        };
        self.native_model_jobs
            .borrow_mut()
            .insert(request.job_id.clone());
        let reply = if bytes.is_packaged() {
            worker.request_packaged_step(request.clone()).await
        } else {
            worker.request(request.clone()).await
        };
        let active = self.native_model_jobs.borrow_mut().remove(&request.job_id);
        let reply = reply.map_err(|error| error.to_string())?;
        if !active || !is_current() || self.scope().as_ref() != Some(&scope) {
            let _ = worker.cancel(&request.job_id);
            return Err("STEP model request became stale after parsing".into());
        }
        let result = validate_reply(&request, reply, &identity).map_err(|error| {
            let _ = worker.cancel(&request.job_id);
            format!("{error:?}")
        })?;
        let mesh = result
            .mesh
            .ok_or_else(|| "STEP model reader returned no mesh".to_owned())?;
        Ok(crate::model_delivery::MeshArrays {
            positions: mesh.positions,
            normals: mesh.normals,
            colors: None,
            step_bounds: result.bounds,
            vendor_step: result.vendor_step.map(Rc::new),
        })
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn query_collision_subjects(
        &self,
        left: &crate::model_delivery::CadCollisionSubject,
        right: &crate::model_delivery::CadCollisionSubject,
        clearance_threshold_mm: f64,
        requested_accuracy_mm: Option<f64>,
        scope: Scope,
        token: SnapshotToken,
        revision: u64,
        is_current: Rc<dyn Fn() -> bool>,
    ) -> Result<crate::model_delivery::CadCollisionResult, String> {
        use crate::model_delivery::{CadCollisionAssessment, CadCollisionResult};
        use boardstudio_web_host::{cad_jobs::CadCollisionQuery, cad_qualification::MeshDistance};
        let batch = self.collision_batch.get();
        let identity = CadSnapshotIdentity {
            token: token.0,
            session_epoch: scope.session_epoch.0,
            document_id: scope.document_id.clone(),
            board_id: scope.board_id.clone(),
            instance_id: scope.instance_id.clone(),
            revision,
        };
        let mut subjects = [left.clone(), right.clone()];
        for attempt in 0..2 {
            if batch != self.collision_batch.get()
                || !is_current()
                || !self.collision_snapshot_is_current(&scope, token, revision)
            {
                return Err("collision request became stale before dispatch".into());
            }
            let operands = subjects[0]
                .operand()
                .and_then(|left| subjects[1].operand().map(|right| (left, right)));
            let (left, right) = match operands {
                Ok(operands) => operands,
                Err(reason) => {
                    return Ok(CadCollisionResult {
                        source_interval_mm: None,
                        query_qualification: None,
                        observation: None,
                        left_provenance: None,
                        right_provenance: None,
                        assessment: CadCollisionAssessment::Unknown {
                            reasons: vec![reason],
                            refinement_requested: requested_accuracy_mm.is_some(),
                        },
                    });
                }
            };
            let query = CadCollisionQuery {
                left,
                right,
                clearance_threshold_mm,
                requested_accuracy_mm,
                qualification_limits: Default::default(),
            };
            let mut result = self
                .dispatch_collision_subjects(
                    [&subjects[0], &subjects[1]],
                    query,
                    scope.clone(),
                    identity.clone(),
                    is_current.clone(),
                )
                .await?;
            let unavailable = result.query_qualification.as_ref().is_some_and(|r| matches!(&r.mesh_distance, MeshDistance::Unknown { reasons } if reasons == &["evidence-unavailable"]));
            if attempt == 1 || !unavailable {
                return Ok(result);
            }
            for subject in &mut subjects {
                match self
                    .refresh_collision_subject(subject, &scope, token, revision, is_current.clone())
                    .await
                {
                    Ok(refreshed) => *subject = refreshed,
                    Err(error) => {
                        if let CadCollisionAssessment::Unknown { reasons, .. } =
                            &mut result.assessment
                        {
                            reasons.push(error);
                        }
                        if batch != self.collision_batch.get() || !is_current() {
                            return Err("collision preparation became stale".into());
                        }
                        return Ok(result);
                    }
                }
            }
        }
        unreachable!("bounded collision evidence recovery")
    }

    pub(super) async fn dispatch_collision_subjects(
        &self,
        subjects: [&crate::model_delivery::CadCollisionSubject; 2],
        query: boardstudio_web_host::cad_jobs::CadCollisionQuery,
        scope: Scope,
        identity: CadSnapshotIdentity,
        is_current: Rc<dyn Fn() -> bool>,
    ) -> Result<crate::model_delivery::CadCollisionResult, String> {
        let [left, right] = subjects;
        let token = SnapshotToken(identity.token);
        let revision = identity.revision;
        let clearance_threshold_mm = query.clearance_threshold_mm;
        let requested_accuracy_mm = query.requested_accuracy_mm;
        let batch = self.collision_batch.get();
        use crate::model_delivery::assess_subject_collision_observation;
        use boardstudio_web_host::cad_jobs::CadCollisionMeshPair;
        if !is_current() || !self.collision_snapshot_is_current(&scope, token, revision) {
            return Err("collision request became stale before worker setup".into());
        }
        let (left_positions, left_tags) = left.mesh().ok_or("left analysis mesh is unavailable")?;
        let (right_positions, right_tags) =
            right.mesh().ok_or("right analysis mesh is unavailable")?;
        let pair_bytes = left_positions
            .len()
            .saturating_mul(8)
            .saturating_add(left_tags.len().saturating_mul(4))
            .saturating_add(right_positions.len().saturating_mul(8))
            .saturating_add(right_tags.len().saturating_mul(4));
        if pair_bytes > boardstudio_web_host::cad_jobs::MAX_COLLISION_QUERY_BYTES {
            return Err("collision pair exceeds the 64 MiB limit before copying".into());
        }
        let worker = self.model_worker(&scope).await?;
        if self.collision_batch.get() != batch
            || !is_current()
            || !self.collision_snapshot_is_current(&scope, token, revision)
        {
            return Err("collision request became stale before dispatch".into());
        }
        let operation = self.operation().0;
        let request = CadRequest {
            request_id: format!("case-model-collision-{revision}-{operation}"),
            job_id: format!("case-model-collision-{revision}-{operation}"),
            identity: identity.clone(),
            operation: CadOperation::CollisionQuery,
            prepared: None,
            source_sha256: None,
            step_import_options: None,
            generated_analysis_request: None,
            collision_query: Some(query),
            collision_meshes: Some(CadCollisionMeshPair {
                left_positions_mm_f64: left_positions.to_vec(),
                left_triangle_face_occurrence_indices: left_tags.to_vec(),
                right_positions_mm_f64: right_positions.to_vec(),
                right_triangle_face_occurrence_indices: right_tags.to_vec(),
            }),
            input_bytes: Vec::new(),
        };
        self.native_model_jobs
            .borrow_mut()
            .insert(request.job_id.clone());
        let reply = worker.request(request.clone()).await;
        let active = self.native_model_jobs.borrow_mut().remove(&request.job_id);
        let reply = reply.map_err(|error| error.to_string())?;
        if !active
            || self.collision_batch.get() != batch
            || !is_current()
            || !self.collision_snapshot_is_current(&scope, token, revision)
        {
            return Err("collision request became stale after parsing".into());
        }
        let result =
            validate_reply(&request, reply, &identity).map_err(|error| format!("{error:?}"))?;
        let query = request
            .collision_query
            .as_ref()
            .ok_or("missing collision request")?;
        let report = result
            .query_qualification
            .clone()
            .ok_or("missing query qualification")?;
        let expected = boardstudio_web_host::cad_qualification::expected_binding(
            query,
            &identity,
            &report.binding.worker_generation,
        )?;
        let validated = boardstudio_web_host::cad_qualification::validate_query_qualification(
            report,
            &expected,
            &query.qualification_limits,
            request
                .collision_meshes
                .as_ref()
                .ok_or("missing collision buffers")?,
            left.source_qualification()
                .zip(right.source_qualification())
                .map(|(left, right)| [left, right]),
        )?;
        Ok(assess_subject_collision_observation(
            left,
            right,
            clearance_threshold_mm,
            requested_accuracy_mm,
            Some((&validated, &expected)),
            result
                .collision
                .ok_or("collision query returned no observation")?,
        ))
    }

    pub fn cancel_collision_checks(&self) {
        self.collision_alive.borrow().set(false);
        *self.collision_alive.borrow_mut() = Rc::new(Cell::new(true));
        self.collision_batch.set(
            self.collision_batch
                .get()
                .checked_add(1)
                .expect("collision batch exhausted"),
        );
        self.collision_generated_attempts.borrow_mut().clear();
        let jobs = self
            .native_model_jobs
            .borrow()
            .iter()
            .filter(|job| {
                job.starts_with("case-model-collision-")
                    || job.starts_with("case-model-analysis-")
                    || job.starts_with("case-model-refine-step-")
                    || job.starts_with("case-model-evidence-step-")
            })
            .cloned()
            .collect::<Vec<_>>();
        for job in jobs {
            self.native_model_jobs.borrow_mut().remove(&job);
            if let Some((_, worker)) = self.cad_worker.borrow().as_ref() {
                let _ = worker.cancel(&job);
            }
        }
    }

    pub(super) fn collision_snapshot_is_current(
        &self,
        scope: &Scope,
        token: SnapshotToken,
        revision: u64,
    ) -> bool {
        self.scope().as_ref() == Some(scope)
            && self.model().accepted.is_some_and(|accepted| {
                accepted.token == token
                    && accepted.document.revision == revision
                    && accepted.session_epoch == scope.session_epoch
                    && accepted.document.id == scope.document_id
            })
    }

    pub(super) fn collision_current(
        &self,
        is_current: Rc<dyn Fn() -> bool>,
    ) -> Rc<dyn Fn() -> bool> {
        let alive = self.collision_alive.borrow().clone();
        Rc::new(move || alive.get() && is_current())
    }

    pub(super) async fn verified_collision_step_bytes(
        &self,
        sha: &str,
    ) -> Result<crate::model_delivery::VerifiedModelBytes, String> {
        let cached = self.assets.borrow().get(sha).cloned();
        let bytes = match cached {
            Some(bytes) => bytes,
            None => self
                .store
                .load_asset(sha.into())
                .await
                .map_err(|e| e.to_string())?
                .ok_or("original vendor STEP bytes are unavailable")?
                .to_vec(),
        };
        crate::model_delivery::VerifiedModelBytes::verify(bytes, sha)
    }

    async fn load_collision_evidence(
        &self,
        cache: &crate::model_delivery::ModelDeliveryAdapter,
        sha: &str,
        options: boardstudio_web_host::cad_jobs::CadStepImportOptions,
        context: StepReadContext,
    ) -> Result<Rc<boardstudio_web_host::cad_jobs::CadVendorStep>, String> {
        let batch = format!(
            "{:?}/{}/{}/{}",
            context.scope,
            context.token.0,
            context.revision,
            self.collision_batch.get()
        );
        let current = context.is_current.clone();
        let load = async {
            let bytes = self.verified_collision_step_bytes(sha).await?;
            self.read_step_model(bytes, options, context).await
        };
        cache
            .refresh_step_evidence(sha, options, &batch, current, load)
            .await?
            .vendor_step
            .clone()
            .ok_or_else(|| "STEP evidence omitted its source asset".into())
    }

    pub(super) async fn refresh_collision_subject(
        &self,
        subject: &crate::model_delivery::CadCollisionSubject,
        scope: &Scope,
        token: SnapshotToken,
        revision: u64,
        is_current: Rc<dyn Fn() -> bool>,
    ) -> Result<crate::model_delivery::CadCollisionSubject, String> {
        use crate::model_delivery::CadCollisionSubject;
        let current = self.collision_current(is_current);
        if !current() || !self.collision_snapshot_is_current(scope, token, revision) {
            return Err("evidence preparation became stale".into());
        }
        let mut refreshed = subject.clone();
        match &mut refreshed {
            CadCollisionSubject::VendorModel {
                vendor_step: Some(asset),
                ..
            } => {
                *asset = self
                    .load_collision_evidence(
                        &self.case_model_delivery,
                        &asset.source_sha256,
                        asset.import_options,
                        StepReadContext {
                            scope: scope.clone(),
                            token,
                            revision,
                            is_current: current.clone(),
                            collision: true,
                        },
                    )
                    .await?;
            }
            CadCollisionSubject::GeneratedCaseBody {
                analysis: Some(asset),
                ..
            } => {
                let scene = self
                    .cad_scene()
                    .filter(|s| {
                        s.scope == *scope
                            && s.token == token
                            && s.snapshot.document.revision == revision
                    })
                    .ok_or("generated evidence has no current exact scene")?;
                let target = asset
                    .analysis_mesh
                    .as_ref()
                    .ok_or("generated evidence omitted descriptor")?
                    .settings
                    .target_deflection_mm;
                let key = generated_analysis_cache_key(&scene, &asset.geometry_identity, target);
                let first = self
                    .collision_generated_attempts
                    .borrow_mut()
                    .insert(key.clone());
                if first {
                    self.generated_analysis_cache.borrow_mut().remove(&key);
                }
                if !first
                    && !self.generated_analysis_cache.borrow().contains_key(&key)
                    && !self.generated_analysis_pending.borrow().contains_key(&key)
                {
                    return Err(
                        "generated evidence preparation already failed in this batch".into(),
                    );
                }
                let prepared = self
                    .generated_analysis_prepared
                    .borrow()
                    .get(&asset.geometry_identity)
                    .cloned()
                    .ok_or("complete prepared source for generated evidence is unavailable")?;
                *asset = self
                    .analyze_prepared_generated_body(
                        &scene,
                        &prepared,
                        &asset.body_id,
                        target,
                        current.clone(),
                    )
                    .await?;
            }
            _ => return Err("source evidence cannot be prepared".into()),
        }
        if !current() || !self.collision_snapshot_is_current(scope, token, revision) {
            return Err("evidence preparation became stale after settlement".into());
        }
        Ok(refreshed)
    }

    pub async fn refine_collision_vendor(
        &self,
        scene: &CadScene,
        subject: &crate::model_delivery::CadCollisionSubject,
        target_deflection_mm: f64,
        is_current: Rc<dyn Fn() -> bool>,
    ) -> Result<crate::model_delivery::CadCollisionSubject, String> {
        use crate::model_delivery::CadCollisionSubject;
        let CadCollisionSubject::VendorModel {
            id,
            reference,
            row_transform_f64,
            vendor_step: Some(asset),
        } = subject
        else {
            return Err(format!(
                "{} has no retained vendor STEP source for refinement",
                subject.label()
            ));
        };
        if !is_current()
            || !self.collision_snapshot_is_current(
                &scene.scope,
                scene.token,
                scene.snapshot.document.revision,
            )
        {
            return Err("vendor refinement became stale".into());
        }
        let is_current = self.collision_current(is_current);
        let options = boardstudio_web_host::cad_jobs::CadStepImportOptions {
            target_deflection_mm: Some(target_deflection_mm),
            ..asset.import_options
        };
        options.validate()?;
        let evidence = self
            .load_collision_evidence(
                &self.collision_vendor_refinement,
                &asset.source_sha256,
                options,
                StepReadContext {
                    scope: scene.scope.clone(),
                    token: scene.token,
                    revision: scene.snapshot.document.revision,
                    is_current,
                    collision: true,
                },
            )
            .await?;
        Ok(CadCollisionSubject::VendorModel {
            id: id.clone(),
            reference: reference.clone(),
            row_transform_f64: *row_transform_f64,
            vendor_step: Some(evidence),
        })
    }

    pub async fn generated_collision_subjects(
        self: &Rc<Self>,
        scene: &CadScene,
        accuracy: Option<f64>,
        current: Rc<dyn Fn() -> bool>,
    ) -> (Vec<crate::model_delivery::CadCollisionSubject>, Vec<String>) {
        use crate::model_delivery::{CadCollisionSubject, IDENTITY_ROW_TRANSFORM_F64};
        let mut subjects = Vec::new();
        let mut errors = Vec::new();
        for body in scene
            .prepared
            .bodies
            .iter()
            .filter(|body| body.body.id != "pcb-reference")
        {
            let analysis = match self
                .analyze_generated_case_body(scene, &body.body.id, accuracy, current.clone())
                .await
            {
                Ok(analysis) => Some(analysis),
                Err(error) => {
                    errors.push(format!("{}: {error}", body.body.name));
                    None
                }
            };
            subjects.push(CadCollisionSubject::GeneratedCaseBody {
                id: body.body.id.clone(),
                name: body.body.name.clone(),
                row_transform_f64: IDENTITY_ROW_TRANSFORM_F64,
                analysis,
            });
            if !current() {
                return (Vec::new(), Vec::new());
            }
        }
        let pcb_current = {
            let runtime = self.clone();
            let current = current.clone();
            let scene_identity = scene as *const CadScene as usize;
            Rc::new(move || {
                current()
                    && runtime
                        .cad_scene()
                        .is_some_and(|scene| Rc::as_ptr(&scene) as usize == scene_identity)
            })
        };
        let pcb = async {
            let physical = self
                .prepare_physical_step_assembly(
                    &format!("collision-pcb-{}", self.operation().0),
                    &scene.snapshot,
                    &scene.scope,
                    pcb_current.clone(),
                )
                .await?;
            self.analyze_prepared_generated_body(
                scene,
                &physical.prepared.prepared,
                "pcb-reference",
                accuracy,
                pcb_current.clone(),
            )
            .await
        }
        .await;
        let analysis = match pcb {
            Ok(analysis) if pcb_current() => Some(analysis),
            Ok(_) => {
                errors.push("PCB unavailable: physical source preparation became stale".into());
                None
            }
            Err(error) => {
                errors.push(format!("PCB unavailable: {error}"));
                None
            }
        };
        if !current() {
            return (Vec::new(), Vec::new());
        }
        subjects.push(CadCollisionSubject::GeneratedCaseBody {
            id: "pcb-reference".into(),
            name: "PCB".into(),
            row_transform_f64: IDENTITY_ROW_TRANSFORM_F64,
            analysis,
        });
        (subjects, errors)
    }

    pub async fn analyze_generated_case_body(
        &self,
        scene: &CadScene,
        body_id: &str,
        target_deflection_mm: Option<f64>,
        is_current: Rc<dyn Fn() -> bool>,
    ) -> Result<Rc<boardstudio_web_host::cad_jobs::CadGeneratedAnalysis>, String> {
        self.analyze_prepared_generated_body(
            scene,
            &scene.prepared,
            body_id,
            target_deflection_mm,
            is_current,
        )
        .await
    }

    pub(super) async fn analyze_prepared_generated_body(
        &self,
        scene: &CadScene,
        prepared: &boardstudio_model::model::PreparedCaseAssemblyIR,
        body_id: &str,
        target_deflection_mm: Option<f64>,
        is_current: Rc<dyn Fn() -> bool>,
    ) -> Result<Rc<boardstudio_web_host::cad_jobs::CadGeneratedAnalysis>, String> {
        use boardstudio_web_host::cad_jobs::{
            CadGeneratedAnalysisRequest, generated_geometry_identity,
        };
        let is_current = self.collision_current(is_current);
        let revision = scene.snapshot.document.revision;
        if !is_current()
            || !scene.exact
            || !self.collision_snapshot_is_current(&scene.scope, scene.token, revision)
            || !self
                .cad_scene()
                .is_some_and(|current| std::ptr::eq(current.as_ref(), scene))
        {
            return Err("generated analysis requires the current exact scene".into());
        }
        let body = prepared
            .bodies
            .iter()
            .find(|body| body.body.id == body_id)
            .ok_or("generated analysis body is not in the prepared scene")?;
        let metadata = CadGeneratedAnalysisRequest {
            body_id: body_id.into(),
            geometry_identity: generated_geometry_identity(body)?,
            target_deflection_mm,
        };
        boardstudio_web_host::cad_jobs::validate_generated_analysis_request(
            Some(prepared),
            revision,
            &metadata,
        )?;
        let cache_key =
            generated_analysis_cache_key(scene, &metadata.geometry_identity, target_deflection_mm);
        if let Some(cached) = self.generated_analysis_cache.borrow().get(&cache_key) {
            boardstudio_web_host::cad_jobs::validate_generated_analysis(
                &metadata, revision, cached,
            )?;
            return Ok(cached.clone());
        }
        let worker = self.model_worker(&scene.scope).await?;
        if !is_current() || !self.collision_snapshot_is_current(&scene.scope, scene.token, revision)
        {
            return Err("generated analysis became stale before dispatch".into());
        }
        let receiver = {
            let mut pending = self.generated_analysis_pending.borrow_mut();
            if pending
                .get(&cache_key)
                .is_some_and(|entry| entry.batch != self.collision_batch.get())
                && let Some(old) = pending.remove(&cache_key)
            {
                for waiter in old.waiters {
                    let _ = waiter.send(Err("generated analysis batch was retired".into()));
                }
            }
            pending.get_mut(&cache_key).map(|pending| {
                let (sender, receiver) = futures_channel::oneshot::channel();
                pending.waiters.push(sender);
                receiver
            })
        };
        if let Some(receiver) = receiver {
            let result = receiver
                .await
                .map_err(|_| "generated analysis task was abandoned")?;
            if !is_current()
                || !self.collision_snapshot_is_current(&scene.scope, scene.token, revision)
            {
                return Err("joined generated analysis became stale".into());
            }
            return result;
        }
        let operation = self.operation().0;
        let identity = CadSnapshotIdentity {
            token: scene.token.0,
            session_epoch: scene.scope.session_epoch.0,
            document_id: scene.scope.document_id.clone(),
            board_id: scene.scope.board_id.clone(),
            instance_id: scene.scope.instance_id.clone(),
            revision,
        };
        let request = CadRequest {
            request_id: format!("case-model-analysis-{revision}-{operation}"),
            job_id: format!("case-model-analysis-{revision}-{operation}"),
            identity: identity.clone(),
            operation: CadOperation::AnalyzeGeneratedBody,
            prepared: Some(prepared.clone()),
            generated_analysis_request: Some(metadata),
            source_sha256: None,
            step_import_options: None,
            collision_query: None,
            collision_meshes: None,
            input_bytes: Vec::new(),
        };
        self.native_model_jobs
            .borrow_mut()
            .insert(request.job_id.clone());
        self.generated_analysis_pending.borrow_mut().insert(
            cache_key.clone(),
            PendingGeneratedAnalysis {
                task: request.job_id.clone(),
                batch: self.collision_batch.get(),
                waiters: Vec::new(),
            },
        );
        let result: GeneratedAnalysisResult = async {
            let reply = worker.request(request.clone()).await;
            let active = self.native_model_jobs.borrow_mut().remove(&request.job_id);
            let reply = reply.map_err(|error| error.to_string())?;
            if !active
                || !is_current()
                || !self.collision_snapshot_is_current(&scene.scope, scene.token, revision)
                || !self
                    .cad_scene()
                    .is_some_and(|current| std::ptr::eq(current.as_ref(), scene))
            {
                return Err("generated analysis became stale after dispatch".into());
            }
            let result =
                validate_reply(&request, reply, &identity).map_err(|error| format!("{error:?}"))?;
            let analysis = Rc::new(
                result
                    .generated_analysis
                    .ok_or("generated analysis returned no payload")?,
            );
            Ok(analysis)
        }
        .await;
        let pending = self
            .generated_analysis_pending
            .borrow_mut()
            .remove(&cache_key)
            .ok_or("generated task lost settlement authority")?;
        if result.is_err() {
            let _ = worker.cancel(&request.job_id);
        }
        if pending.task != request.job_id {
            self.generated_analysis_pending
                .borrow_mut()
                .insert(cache_key, pending);
            return Err("generated task was superseded before settlement".into());
        }
        for waiter in pending.waiters {
            let _ = waiter.send(result.clone());
        }
        let analysis = result?;
        let mut cache = self.generated_analysis_cache.borrow_mut();
        let bytes = |asset: &boardstudio_web_host::cad_jobs::CadGeneratedAnalysis| {
            asset.surface_positions_mm_f64.len() * 8
                + asset.triangle_face_occurrence_indices.len() * 4
        };
        if cache.len() >= 64
            || cache.values().map(|asset| bytes(asset)).sum::<usize>() + bytes(&analysis)
                > 64 * 1024 * 1024
        {
            cache.clear();
            self.generated_analysis_prepared.borrow_mut().clear();
        }
        self.generated_analysis_prepared.borrow_mut().insert(
            analysis.geometry_identity.clone(),
            boardstudio_model::model::PreparedCaseAssemblyIR {
                revision,
                bodies: vec![body.clone()],
            },
        );
        cache.insert(cache_key, analysis.clone());
        Ok(analysis)
    }
}
