//! Layout viewer source preparation and model delivery.
use super::*;

impl Runtime {
    pub fn layout_preview(&self) -> Option<Rc<crate::layout_viewer_source::LayoutPreviewSnapshot>> {
        self.layout_preview
            .borrow()
            .published
            .as_ref()
            .filter(|preview| self.layout_source_owner_is_current(&preview.owner))
            .cloned()
    }

    pub fn layout_preview_pending(&self) -> bool {
        self.layout_preview
            .borrow()
            .pending
            .as_ref()
            .is_some_and(|(owner, lease)| {
                lease.matches(owner) && self.layout_source_owner_is_current(owner)
            })
    }

    pub fn layout_preview_error(&self) -> Option<String> {
        self.layout_preview
            .borrow()
            .error
            .as_ref()
            .filter(|(owner, _)| self.snapshot_current(owner.snapshot_token, &owner.scope))
            .map(|(_, error)| error.clone())
    }

    pub fn layout_source_generation(&self) -> Option<u64> {
        let state = self.layout_preview.borrow();
        state
            .published
            .as_ref()
            .map(|preview| preview.owner.source_generation)
            .or_else(|| {
                state
                    .pending
                    .as_ref()
                    .map(|(owner, _)| owner.source_generation)
            })
            .or_else(|| {
                state
                    .error
                    .as_ref()
                    .map(|(owner, _)| owner.source_generation)
            })
    }

    pub fn layout_model_delivery(
        &self,
        preview: &crate::layout_viewer_source::LayoutPreviewSnapshot,
    ) -> Option<crate::model_delivery::ModelDeliveryRows> {
        self.layout_model_rows
            .borrow()
            .as_ref()
            .filter(|(owner, _)| {
                owner == &preview.owner
                    && preview.lease.matches(owner)
                    && self.layout_source_owner_is_current(owner)
            })
            .map(|(_, rows)| rows.clone())
    }

    pub fn retire_layout_source(&self, source_generation: u64) {
        let retired = self
            .layout_preview
            .borrow_mut()
            .retire_generation(source_generation);
        if retired {
            self.layout_model_rows.borrow_mut().take();
            self.changed();
        }
    }

    pub fn reconcile_layout_source_request(&self, expected: Option<(&Scope, SnapshotToken, u64)>) {
        let retired = self
            .layout_preview
            .borrow_mut()
            .retire_unless_request_matches(expected.map(|(scope, token, _)| (scope, token)));
        if retired {
            self.layout_model_rows.borrow_mut().take();
            self.changed();
        }
    }

    pub(super) fn layout_source_owner_is_current(
        &self,
        owner: &crate::layout_viewer_source::LayoutSourceIdentity,
    ) -> bool {
        self.snapshot_current(owner.snapshot_token, &owner.scope)
            && self.layout_preview.borrow().owns(owner)
    }

    pub(super) async fn deliver_layout_models(
        self: &Rc<Self>,
        preview: Rc<crate::layout_viewer_source::LayoutPreviewSnapshot>,
    ) -> Result<(), String> {
        if !self.layout_source_owner_is_current(&preview.owner) {
            return Ok(());
        }
        if self
            .layout_model_rows
            .borrow()
            .as_ref()
            .is_some_and(|(owner, _)| owner == &preview.owner)
        {
            return Ok(());
        }
        let owner = crate::model_delivery::ModelOwnerIdentity::capture(
            preview.owner.snapshot_token,
            preview.lease.clone(),
        );
        let source_is_current = {
            let weak = Rc::downgrade(self);
            let preview = preview.clone();
            Rc::new(move || {
                weak.upgrade().is_some_and(|runtime| {
                    runtime.layout_source_owner_is_current(&preview.owner)
                        && preview.lease.matches(&preview.owner)
                })
            }) as Rc<dyn Fn() -> bool>
        };
        let module_request_id = format!(
            "layout-module-models-{}-{}-{}",
            preview.owner.snapshot_token.0,
            preview.document.revision,
            preview.owner.source_generation
        );
        let module_request = CoreRequest::ResolveModules {
            id: module_request_id.clone(),
            document: preview.document.as_ref().clone(),
            board_id: preview.owner.scope.board_id.clone(),
            preview_top_z: Some(preview.preview.thickness),
        };
        let module_reply = (self.core_services.open_scratch)()(module_request);
        let module_placements = match module_reply {
            CoreReply::ModulesResolved { id, result }
                if id == module_request_id && result.revision == preview.document.revision =>
            {
                result.model_placements
            }
            CoreReply::ModulesResolved { .. } => {
                return Err(
                    "Core returned mounted-module models for another request or revision".into(),
                );
            }
            CoreReply::Error { id, message, .. } if id == module_request_id => {
                return Err(format!("Mounted-module model resolution failed: {message}"));
            }
            _ => return Err("Core returned an unexpected mounted-module model reply".into()),
        };
        let native_paths = crate::model_delivery::native_model_path_assets(&preview.path_assets);
        let bundled_ids_by_path = bundled_preview_paths(&preview.preview.models).await?;
        if !source_is_current() {
            return Ok(());
        }
        let selections = crate::model_delivery::resolve_preview_assets(
            &preview.preview.models,
            preview.board_reference.as_ref(),
            &native_paths,
            &preview.document,
            |path| bundled_ids_by_path.get(path).cloned().flatten(),
            packaged_model_asset,
        )
        .into_iter()
        .collect::<BTreeMap<_, _>>();
        let ports = self.model_delivery_ports(
            preview.owner.scope.clone(),
            preview.owner.snapshot_token,
            preview.document.revision,
            source_is_current.clone(),
        );
        let board_results = self
            .case_model_delivery
            .deliver_models(
                &preview.preview.models,
                &selections,
                &ports,
                &owner,
                source_is_current.clone(),
            )
            .await;
        let mut rows = board_results;
        if !module_placements.is_empty() {
            let selections = module_placements
                .iter()
                .map(|placement| {
                    let selection = crate::model_delivery::select_model_asset_id(
                        &placement.asset_id,
                        &preview.document.assets,
                        packaged_model_asset,
                    );
                    (placement.id.clone(), selection)
                })
                .collect::<BTreeMap<_, _>>();
            let module_results = self
                .case_model_delivery
                .deliver_module_placements(
                    &module_placements,
                    &selections,
                    &ports,
                    &owner,
                    source_is_current.clone(),
                )
                .await;
            match (&mut rows, module_results) {
                (Some(rows), Some(modules)) => {
                    rows.delivered.extend(modules.delivered);
                    rows.pending.extend(modules.pending);
                    rows.failures.extend(modules.failures);
                }
                (_, None) => return Ok(()),
                (None, Some(modules)) => rows = Some(modules),
            }
        }
        if let Some(rows) = rows
            && source_is_current()
            && self.layout_source_owner_is_current(&preview.owner)
        {
            *self.layout_model_rows.borrow_mut() = Some((preview.owner.clone(), rows));
            self.changed();
        }
        Ok(())
    }

    pub async fn prepare_layout_preview(
        self: &Rc<Self>,
        expected_scope: Scope,
        expected_token: SnapshotToken,
        expected_revision: u64,
    ) -> Result<(), String> {
        let accepted = self
            .with_model(|model| model.accepted.clone())
            .ok_or_else(|| "No accepted project is available for Layout preview".to_owned())?;
        if !self.snapshot_current(expected_token, &expected_scope)
            || accepted.document.revision != expected_revision
        {
            return Err("The accepted Layout preview source changed before preparation".into());
        }
        if self.layout_preview().is_some() || self.layout_preview_pending() {
            return Ok(());
        }

        let source_generation = self.layout_preview.borrow_mut().next_generation()?;
        let operation = self.operation().0;
        if operation == 0 || operation > 9_007_199_254_740_991 {
            let error = "Layout preview operation identity is outside the safe integer range";
            self.layout_preview.borrow_mut().fail_before_begin(
                crate::layout_viewer_source::LayoutSourceIdentity::from_accepted(
                    &accepted,
                    &expected_scope,
                    source_generation,
                ),
                error.into(),
            );
            self.changed();
            return Err(error.into());
        }
        let request_token = format!(
            "layout-preview-{}-{}-{}",
            expected_token.0, expected_revision, operation
        );
        let model_paths = crate::case_preview::preview_model_paths(&accepted.document);
        let capture = match crate::layout_viewer_source::LayoutSourceCapture::capture(
            &accepted,
            &expected_scope,
            source_generation,
            request_token,
            model_paths,
        ) {
            Ok(capture) => capture,
            Err(error) => {
                self.layout_preview.borrow_mut().fail_before_begin(
                    crate::layout_viewer_source::LayoutSourceIdentity::from_accepted(
                        &accepted,
                        &expected_scope,
                        source_generation,
                    ),
                    error.clone(),
                );
                self.changed();
                return Err(error);
            }
        };
        self.layout_preview.borrow_mut().begin(&capture);
        self.layout_model_rows.borrow_mut().take();
        self.changed();

        let owner = capture.owner.clone();
        let lease = capture.lease.clone();
        let weak = Rc::downgrade(self);
        let is_current: Rc<dyn Fn() -> bool> = Rc::new(move || {
            weak.upgrade().is_some_and(|runtime| {
                runtime.layout_source_owner_is_current(&owner) && lease.matches(&owner)
            })
        });
        let result = match &capture.request {
            crate::layout_viewer_source::LayoutPreviewRequest::Authored(request) => self
                .run_preview_pipeline(
                    request.as_ref().clone(),
                    operation,
                    capture.owner.scope.clone(),
                    capture.owner.source_generation,
                    is_current.clone(),
                ),
            crate::layout_viewer_source::LayoutPreviewRequest::Imported { asset, .. } => {
                self.run_imported_layout_preview(&capture, asset, operation, is_current.clone())
                    .await
            }
        };
        match result.and_then(|preview| capture.accept_preview(preview)) {
            Ok(preview) if is_current() => {
                self.layout_preview.borrow_mut().publish(preview)?;
                let published = self.layout_preview();
                self.changed();
                if let Some(published) = published {
                    let runtime = self.clone();
                    spawn_local(async move {
                        let current_runtime = Rc::downgrade(&runtime);
                        let reporter = runtime.clone();
                        crate::model_delivery::settle_layout_model_delivery(
                            published.clone(),
                            runtime.deliver_layout_models(published),
                            move || {
                                current_runtime
                                    .upgrade()
                                    .and_then(|runtime| runtime.layout_preview())
                            },
                            move |error| {
                                reporter.report(format!("Layout model preview failed: {error}"));
                            },
                        )
                        .await;
                    });
                }
                Ok(())
            }
            Ok(preview) => {
                preview.lease.invalidate();
                Err("Layout preview result became stale before publication".into())
            }
            Err(error) => {
                let still_current = is_current();
                if still_current {
                    self.layout_preview
                        .borrow_mut()
                        .fail(capture.owner.clone(), error.clone());
                    self.changed();
                } else {
                    capture.lease.invalidate();
                }
                Err(error)
            }
        }
    }

    pub(super) async fn run_imported_layout_preview(
        &self,
        capture: &crate::layout_viewer_source::LayoutSourceCapture,
        asset: &boardstudio_model::model::Asset,
        operation: u64,
        is_current: Rc<dyn Fn() -> bool>,
    ) -> Result<PcbPreview, String> {
        let request_id = format!("layout-preview-{operation}-imported");
        self.run_imported_board_preview(asset, request_id.clone(), is_current, |source| {
            capture.artifact_request(request_id, Some(source))
        })
        .await
    }

    /// Both preview owners use the same SHA-verified asset and Core artifact path.
    /// The caller retains its request construction and source-liveness checks.
    pub(super) async fn run_imported_board_preview(
        &self,
        asset: &boardstudio_model::model::Asset,
        request_id: String,
        is_current: Rc<dyn Fn() -> bool>,
        make_request: impl FnOnce(String) -> Result<ArtifactRequest, String>,
    ) -> Result<PcbPreview, String> {
        let bytes = self
            .load_preview_document_asset(asset, is_current.clone())
            .await?;
        let source = String::from_utf8(bytes)
            .map_err(|error| format!("Imported board asset is not valid UTF-8: {error}"))?;
        let request = make_request(source)?;
        if !is_current() {
            return Err("Imported board preview source changed.".into());
        }
        self.board_preview(&request, &request_id)
    }

    /// Parse a routed KiCad board for the accepted reference editor before its
    /// bytes and BoardReference are admitted through the normal edit path.
    pub async fn preview_routed_board_source(
        &self,
        request_id: String,
        source: String,
        revision: u64,
    ) -> Result<PcbPreview, String> {
        if request_id.is_empty() {
            return Err("Routed-board preview request identity must not be empty".into());
        }
        self.board_preview(
            &ArtifactRequest::PreviewBoard {
                id: request_id.clone(),
                source,
                revision,
            },
            &request_id,
        )
    }

    pub(super) fn board_preview(
        &self,
        request: &ArtifactRequest,
        request_id: &str,
    ) -> Result<PcbPreview, String> {
        match artifact_reply(request)? {
            ArtifactReply::PreviewBoard { id, result } if id == request_id => Ok(result),
            _ => Err("Core returned an unexpected board preview reply.".into()),
        }
    }

    pub(super) async fn load_preview_document_asset(
        &self,
        asset: &boardstudio_model::model::Asset,
        is_current: Rc<dyn Fn() -> bool>,
    ) -> Result<Vec<u8>, String> {
        if asset.sha256.is_empty() || !is_current() {
            return Err("Imported board asset identity is missing or stale".into());
        }
        let cached = self.assets.borrow().get(&asset.sha256).cloned();
        let bytes = if let Some(bytes) = cached {
            bytes
        } else {
            self.store
                .load_asset(asset.sha256.clone())
                .await
                .map_err(|error| error.to_string())?
                .ok_or_else(|| format!("Imported board asset {} is not stored", asset.id))?
                .to_vec()
        };
        if !is_current() {
            return Err("Imported board asset request became stale after loading".into());
        }
        verify_source_digest(asset, &bytes)?;
        self.assets
            .borrow_mut()
            .insert(asset.sha256.clone(), bytes.clone());
        Ok(bytes)
    }
}
