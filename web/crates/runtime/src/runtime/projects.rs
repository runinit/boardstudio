//! Project lifecycle: creating, opening, importing, deleting and recovering saved projects.
use super::*;

fn new_keyboard() -> Result<ProjectDoc, String> {
    let project_id = browser_uuid()?;
    let board_id = browser_uuid()?;
    let outline_id = browser_uuid()?;
    let mut document = ProjectDoc::empty(&project_id, "Untitled keyboard");
    document.outline.push(OutlineFeature::PartEnvelope {
        connections: Vec::new(),
        settings: OutlineSettings::default(),
        id: outline_id.clone(),
        part_ids: Vec::new(),
        margin: 4.0,
        operation: Operation::Add,
    });
    document.boards.push(Board {
        id: board_id,
        name: "Main board".into(),
        outline_ids: vec![outline_id],
        part_ids: Vec::new(),
        net_ids: Vec::new(),
        thickness: 1.6,
        traces: Vec::new(),
        vias: Vec::new(),
    });
    document.materials.push(Material {
        id: "pla".into(),
        name: "PLA".into(),
        thickness: 3.0,
    });
    Ok(document)
}

impl Runtime {
    pub fn create_new_keyboard(
        self: &Rc<Self>,
    ) -> Result<(String, crate::operation_outcomes::OutcomeSlot), String> {
        self.begin_open()?;
        let document = new_keyboard()?;
        self.confirm_project_replacement(&document)?;
        let id = document.id.clone();
        let outcome = self.open_document(document, false);
        Ok((id, outcome))
    }

    pub fn open_demo_copy(self: &Rc<Self>, mut document: ProjectDoc) -> Result<(), String> {
        self.begin_open()?;
        document.id = browser_uuid()?;
        document.revision = 0;
        self.confirm_project_replacement(&document)?;
        self.open_document(document, false);
        Ok(())
    }

    fn open_document(
        self: &Rc<Self>,
        document: ProjectDoc,
        for_deletion: bool,
    ) -> crate::operation_outcomes::OutcomeSlot {
        let operation_id = self.operation();
        let outcome = self.observe_operation(operation_id);
        if for_deletion {
            self.project_deletion_open.set(Some(operation_id));
        }
        self.submit(if self.model().lifecycle == Lifecycle::RecoveryRequired {
            Event::RecoverWithDocument {
                operation_id,
                document,
            }
        } else {
            Event::Open {
                operation_id,
                document,
            }
        });
        outcome
    }

    pub async fn delete_saved_project(self: &Rc<Self>, project_id: String) -> Result<(), String> {
        let sequence = self.begin_open()?;
        self.project_deletion_pending.set(true);
        let _deletion = ProjectDeletionLease {
            runtime: self.clone(),
        };
        let model = self.model();
        let deleting_active = model
            .accepted
            .as_ref()
            .is_some_and(|accepted| accepted.document.id == project_id);
        let expected = if deleting_active {
            self.saved_tokens.borrow().get(&project_id).cloned()
        } else {
            self.store.observed_token(&project_id)
        };
        if deleting_active {
            if !matches!(model.durability, Durability::Saved { .. }) {
                return Err("Save the active keyboard before deleting it.".into());
            }
            let remaining = self
                .store
                .list_documents()
                .await
                .map_err(|error| error.to_string())?;
            let replacement = match remaining
                .into_iter()
                .filter(|document| document.id != project_id)
                .min_by(|left, right| left.name.cmp(&right.name))
            {
                Some(next) => self.remember_saved(
                    self.store
                        .load_document_with_token(next.id)
                        .await
                        .map_err(|error| error.to_string())?
                        .ok_or("The replacement keyboard is no longer available.")?,
                ),
                None => new_keyboard()?,
            };
            let current = self.model();
            if current.accepted.as_ref().map(|accepted| accepted.token)
                != model.accepted.as_ref().map(|accepted| accepted.token)
                || !matches!(current.durability, Durability::Saved { .. })
            {
                return Err("The active keyboard changed during deletion.".into());
            }
            let replacement_id = replacement.id.clone();
            let outcome = self.open_document(replacement, true);
            self.wait_for_project_open(outcome, &replacement_id, Some(sequence))
                .await?;
        }
        self.store
            .delete_project_expected(project_id.clone(), expected.as_ref())
            .await
            .map_err(|error| format!("Could not delete the saved keyboard: {error}"))?;
        self.saved_tokens.borrow_mut().remove(&project_id);
        Ok(())
    }

    pub(super) async fn wait_for_project_open(
        self: &Rc<Self>,
        outcome: crate::operation_outcomes::OutcomeSlot,
        expected_id: &str,
        expected_sequence: Option<u64>,
    ) -> Result<AcceptedSnapshot, String> {
        for _ in 0..1_200 {
            #[cfg(all(any(test, feature = "test-support"), target_arch = "wasm32"))]
            test_hooks::run_pending(self).await;
            if let Some(outcome) = outcome.borrow_mut().take() {
                if outcome != TerminalOutcome::Completed {
                    return Err(format!(
                        "The replacement keyboard could not be saved; the original keyboard was kept. {outcome:?}"
                    ));
                }
                if expected_sequence.is_some_and(|sequence| self.open_sequence.get() != sequence) {
                    return Err(
                        "A newer project open superseded the replacement; the saved keyboard was kept."
                            .into(),
                    );
                }
                let accepted = self
                    .model()
                    .accepted
                    .ok_or_else(|| "The replacement keyboard was not accepted.".to_owned())?;
                if accepted.document.id != expected_id {
                    return Err(
                        "The replacement keyboard did not become active; the saved keyboard was kept."
                            .into(),
                    );
                }
                return Ok(accepted);
            }
            TimeoutFuture::new(25).await;
        }
        Err(
            "The replacement keyboard did not finish saving; the original keyboard was kept."
                .into(),
        )
    }

    pub(super) fn cancel_frames(&self) {
        if let Some(window) = web_sys::window() {
            for (_, (handle, _)) in std::mem::take(&mut *self.frames.borrow_mut()) {
                let _ = window.cancel_animation_frame(handle);
            }
        }
    }
    pub(super) fn begin_open(&self) -> Result<u64, String> {
        if self.storage_initializing.get() {
            return Err("Storage is still initializing; try again shortly.".into());
        }
        if self.project_deletion_pending.get() {
            return Err("A saved keyboard deletion is in progress.".into());
        }
        let sequence = self
            .open_sequence
            .get()
            .checked_add(1)
            .ok_or("open identity exhausted")?;
        self.open_sequence.set(sequence);
        Ok(sequence)
    }
    pub fn import_file(self: &Rc<Self>, file: web_sys::File) {
        if file.size() > boardstudio_model::model::MAX_PROJECT_ARCHIVE_BYTES as f64 {
            self.apply_report(
                RuntimeReport::status("Project archive exceeds the 128 MiB size limit.")
                    .transient_error(),
            );
            return;
        }
        let sequence = match self.begin_open() {
            Ok(sequence) => sequence,
            Err(error) => {
                self.report(error);
                return;
            }
        };
        #[cfg(any(test, feature = "test-support"))]
        let open_observer = self
            .in_process_adapters
            .borrow()
            .as_ref()
            .and_then(|adapters| adapters.take_open_observer());
        let this = self.clone();
        spawn_local(async move {
            let result = match JsFuture::from(file.array_buffer()).await {
                Ok(buffer) => this.import_archive_at(Uint8Array::new(&buffer).to_vec(), sequence),
                Err(error) => Err(format!("Import read failed: {error:?}")),
            };
            if let Err(error) = result
                && this.open_sequence.get() == sequence
            {
                this.apply_report(RuntimeReport::status(error).transient_error());
            }
            #[cfg(any(test, feature = "test-support"))]
            if let Some(done) = open_observer {
                let _ = done.send(());
            }
        });
    }
    pub(super) fn import_archive_at(
        self: &Rc<Self>,
        bytes: Vec<u8>,
        sequence: u64,
    ) -> Result<(), String> {
        let (metadata, buffers) =
            (self.core_services.archive)("{\"kind\":\"unpack-project\"}", &[bytes]);
        let reply: ArchiveReply = serde_json::from_str(&metadata).map_err(|e| e.to_string())?;
        let ArchiveReply::Unpacked {
            project_json,
            assets,
        } = reply
        else {
            return Err(format!("Archive rejected: {reply:?}"));
        };
        let document: ProjectDoc =
            serde_json::from_str(&project_json).map_err(|e| e.to_string())?;
        let mut copied = BTreeMap::new();
        for asset in assets {
            let bytes = buffers
                .get(asset.buffer_index as usize)
                .ok_or("Archive omitted an asset buffer")?;
            copied.insert(asset.sha256, bytes.to_vec());
        }
        if self.open_sequence.get() != sequence {
            return Err("Open superseded by another import.".into());
        }
        self.confirm_project_replacement(&document)?;
        self.assets.borrow_mut().extend(copied);
        self.open_document(document, false);
        Ok(())
    }
    pub fn open_saved(self: &Rc<Self>, id: String) {
        let sequence = match self.begin_open() {
            Ok(sequence) => sequence,
            Err(error) => {
                self.report(error);
                return;
            }
        };
        #[cfg(any(test, feature = "test-support"))]
        let open_observer = self
            .in_process_adapters
            .borrow()
            .as_ref()
            .and_then(|adapters| adapters.take_open_observer());
        let this = self.clone();
        spawn_local(async move {
            let loaded = this.store.load_document_with_token(id).await;
            #[cfg(any(test, feature = "test-support"))]
            {
                let adapters = this.in_process_adapters.borrow().clone();
                if let Some(adapters) = adapters {
                    adapters.store_read_completed().await;
                }
            }
            match loaded {
                Ok(Some(saved)) if this.open_sequence.get() == sequence => {
                    match this.confirm_project_replacement(&saved.document) {
                        Err(error) => {
                            this.report(format!("Could not open the saved keyboard: {error}"))
                        }
                        Ok(()) => {
                            let document = this.remember_saved(saved);
                            this.open_document(document, false);
                        }
                    }
                }
                Ok(Some(_)) => {}
                Ok(None) if this.open_sequence.get() == sequence => this.report(
                    "The saved keyboard is unavailable. Choose an available copy from the library.",
                ),
                Err(error) if this.open_sequence.get() == sequence => {
                    this.report(format!("Could not open the saved keyboard: {error}"))
                }
                Ok(None) | Err(_) => {}
            }
            #[cfg(any(test, feature = "test-support"))]
            if let Some(done) = open_observer {
                let _ = done.send(());
            }
        });
    }

    /// Reopen a durable project after an owned variant operation failed, but only while the
    /// session/document that requested recovery is still active. The saved copy is confirmed
    /// through the same Session open/recovery outcome used by the project library.
    pub async fn reopen_saved_if_current(
        self: &Rc<Self>,
        id: String,
        expected: &AcceptedSnapshot,
        expected_lifecycle: Lifecycle,
        expected_scope: &Scope,
    ) -> Result<AcceptedSnapshot, String> {
        let still_expected = |runtime: &Runtime| {
            let model = runtime.model();
            model.lifecycle == expected_lifecycle
                && runtime.scope().as_ref() == Some(expected_scope)
                && model.accepted.is_some_and(|accepted| {
                    accepted.session_epoch == expected.session_epoch
                        && accepted.document.id == expected.document.id
                        && accepted.token == expected.token
                        && accepted.document.revision == expected.document.revision
                })
        };
        if !still_expected(self) {
            return Err("A newer project open superseded recovery of the original project.".into());
        }
        let sequence = self.begin_open()?;
        let document = self
            .store
            .load_document_with_token(id.clone())
            .await
            .map_err(|error| format!("Could not load the saved original project: {error}"))?
            .ok_or_else(|| "The saved original project is no longer available.".to_owned())?;
        if self.open_sequence.get() != sequence || !still_expected(self) {
            return Err("A newer project open superseded recovery of the original project.".into());
        }
        let document = self.remember_saved(document);
        let outcome = self.open_document(document, false);
        self.wait_for_project_open(outcome, &id, Some(sequence))
            .await
    }

    pub(super) fn remember_saved(&self, saved: SavedProject) -> ProjectDoc {
        self.saved_tokens
            .borrow_mut()
            .insert(saved.document.id.clone(), saved.token);
        saved.document
    }

    pub fn download_raw_project(self: &Rc<Self>, key: JsValue) {
        let this = self.clone();
        spawn_local(async move {
            let result = match this.store.raw_project(&key).await {
                Ok(Some((json, _))) => deliver(
                    json.as_bytes(),
                    "boardstudio-unreadable-project.json",
                    Some("application/json"),
                ),
                Ok(None) => Err("The saved record is no longer available.".into()),
                Err(error) => Err(error.to_string()),
            };
            if let Err(error) = result {
                this.apply_report(
                    RuntimeReport::status(format!(
                        "Raw saved data could not be downloaded: {error}"
                    ))
                    .transient_error(),
                );
            }
        });
    }

    pub(super) fn confirm_project_replacement(&self, document: &ProjectDoc) -> Result<(), String> {
        (self.core_services.validate_open)(document)?;
        let model = self.model();
        if matches!(
            model.durability,
            Durability::Saving { .. } | Durability::Failed { .. }
        ) {
            return Err("Save the current keyboard before switching projects.".into());
        }
        if model.accepted.is_none() {
            return Ok(());
        }
        let message = format!(
            r#"Open "{}"? Uncommitted form drafts will be discarded."#,
            document.name
        );
        let confirmed = web_sys::window()
            .and_then(|window| window.confirm_with_message(&message).ok())
            .unwrap_or(false);
        if confirmed {
            Ok(())
        } else {
            Err("Project switch canceled; your current work was kept.".into())
        }
    }

    pub(super) async fn pack_archive(
        &self,
        export_id: OperationId,
        snapshot: &AcceptedSnapshot,
        scope: &Scope,
        embed_used_models: bool,
    ) -> Result<Vec<u8>, String> {
        let guard = || {
            if self.export_current(export_id, snapshot.token, scope) {
                Ok(())
            } else {
                Err("Archive export was cancelled or superseded.".to_owned())
            }
        };
        self.pack_document(snapshot.document.as_ref(), embed_used_models, guard)
            .await
    }

    pub(super) async fn pack_document(
        &self,
        document: &ProjectDoc,
        embed_used_models: bool,
        guard: impl Fn() -> Result<(), String>,
    ) -> Result<Vec<u8>, String> {
        guard()?;
        let mut bundled_bytes = Vec::new();
        if embed_used_models {
            let generated_ids = crate::bundled_models::generated_model_ids(document).await?;
            guard()?;
            let existing_ids = document
                .assets
                .iter()
                .map(|asset| asset.id.as_str())
                .collect::<BTreeSet<_>>();
            for model in crate::portable_archive::referenced_models(document, generated_ids)? {
                if existing_ids.contains(model.id) {
                    continue;
                }
                let bytes = crate::bundled_models::bundled_model_bytes(model.id).await?;
                guard()?;
                bundled_bytes.push((model, bytes));
            }
        }
        let mut local_asset_bytes = BTreeMap::new();
        for asset in &document.assets {
            let pending_bytes = self.assets.borrow().get(&asset.sha256).cloned();
            let bytes = match pending_bytes {
                Some(bytes) => bytes,
                None => self
                    .store
                    .load_asset(asset.sha256.clone())
                    .await
                    .map_err(|error| error.to_string())?
                    .ok_or_else(|| format!("Missing asset: {}", asset.name))?
                    .to_vec(),
            };
            guard()?;
            local_asset_bytes.insert(asset.sha256.clone(), bytes);
        }
        let prepared = crate::portable_archive::prepare_archive(
            document,
            &local_asset_bytes,
            bundled_bytes,
            embed_used_models,
        )?;
        guard()?;
        let (metadata, buffers) =
            (self.core_services.archive)(&prepared.metadata, &prepared.buffers);
        match serde_json::from_str::<ArchiveReply>(&metadata).map_err(|e| e.to_string())? {
            ArchiveReply::Packed => buffers
                .into_iter()
                .next()
                .ok_or_else(|| "Archive returned no bytes.".into()),
            reply => Err(format!("Archive export rejected: {reply:?}")),
        }
    }
}
