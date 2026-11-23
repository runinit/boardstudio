//! Layout viewer source preparation and model delivery.
use super::*;

impl Runtime {
    pub fn layout_preview(&self) -> Option<Rc<crate::layout_viewer_source::LayoutPreviewSnapshot>> {
        self.layout_preview
            .published()
            .filter(|preview| self.layout_source_owner_is_current(&preview.owner))
    }

    pub fn layout_preview_pending(&self) -> bool {
        self.layout_preview.pending().is_some_and(|lease| {
            lease.is_active() && self.layout_source_owner_is_current(lease.identity())
        })
    }

    pub fn layout_preview_error(&self) -> Option<String> {
        self.layout_preview
            .current_error(|owner| self.snapshot_current(owner.snapshot_token, &owner.scope))
    }

    pub fn layout_source_generation(&self) -> Option<u64> {
        self.layout_preview
            .with_owner(|owner| owner.source_generation)
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
        if self.layout_preview.retire_generation(source_generation) {
            self.layout_model_rows.borrow_mut().take();
            self.changed();
        }
    }

    pub fn reconcile_layout_source_request(&self, expected: Option<(&Scope, SnapshotToken)>) {
        let stale_generation = self
            .layout_preview
            .with_owner(|owner| {
                (!expected.is_some_and(|(scope, token)| {
                    owner.scope == *scope && owner.snapshot_token == token
                }))
                .then_some(owner.source_generation)
            })
            .flatten();
        if let Some(generation) = stale_generation {
            self.retire_layout_source(generation);
        }
    }

    pub(super) fn layout_source_owner_is_current(
        &self,
        owner: &crate::layout_viewer_source::LayoutSourceIdentity,
    ) -> bool {
        self.snapshot_current(owner.snapshot_token, &owner.scope) && self.layout_preview.owns(owner)
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
        let batch = self.preview_model_batch(
            &preview.owner.scope,
            preview.owner.snapshot_token,
            preview.document.revision,
            preview.lease.clone(),
            source_is_current.clone(),
        );
        let modules = CoreEngine::resolve_modules(
            &preview.document,
            &preview.owner.scope.board_id,
            Some(preview.preview.thickness),
        )
        .map_err(|message| format!("Mounted-module model resolution failed: {message}"))?;
        if modules.revision != preview.document.revision {
            return Err("Core returned mounted-module models for another revision".into());
        }
        let module_placements = modules.model_placements;
        let bundled_ids_by_path = bundled_preview_paths(&preview.preview.models).await?;
        if !source_is_current() {
            return Ok(());
        }
        let mut rows = batch
            .deliver_board_models(
                &preview.preview.models,
                preview.board_reference.as_ref(),
                &preview.path_assets,
                &preview.document,
                &bundled_ids_by_path,
            )
            .await;
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
            let module_results = batch
                .adapter
                .deliver_module_placements(
                    &module_placements,
                    &selections,
                    &batch.ports,
                    &batch.owner,
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
            .accepted_snapshot()
            .ok_or_else(|| "No accepted project is available for Layout preview".to_owned())?;
        if !self.snapshot_current(expected_token, &expected_scope)
            || accepted.document.revision != expected_revision
        {
            return Err("The accepted Layout preview source changed before preparation".into());
        }
        if self.layout_preview().is_some() || self.layout_preview_pending() {
            return Ok(());
        }

        self.layout_preview.retire();
        let source_generation = self
            .layout_preview
            .next_generation("Layout source generation exhausted")?;
        let operation = self.operation().0;
        if operation == 0 || operation > MAX_SAFE_INTEGER {
            let error = "Layout preview operation identity is outside the safe integer range";
            self.layout_preview.fail_before_begin(
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
                self.layout_preview.fail_before_begin(
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
        self.layout_preview.begin(capture.lease.clone());
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
                self.layout_preview.publish(
                    preview,
                    source_generation,
                    "Layout source owner changed before preview publication",
                )?;
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
                    self.layout_preview.fail(&capture.lease, error.clone());
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
        let bytes = self
            .load_document_asset(&asset.sha256)
            .await?
            .ok_or_else(|| format!("Imported board asset {} is not stored", asset.id))?;
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
