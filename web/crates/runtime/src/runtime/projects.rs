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
        self.submit(Event::Open {
            operation_id,
            document,
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
        let expected = self.persistence.borrow().observed_token(&project_id);
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
                Some(next) => self.remember_saved(self.load_saved(next.id).await?),
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
            self.wait_for_project_open(outcome, &replacement_id, sequence)
                .await?;
        }
        self.store
            .delete_project_expected(project_id.clone(), expected.as_ref())
            .await
            .map_err(|error| format!("Could not delete the saved keyboard: {error}"))?;
        Ok(())
    }

    pub(super) async fn wait_for_project_open(
        self: &Rc<Self>,
        outcome: crate::operation_outcomes::OutcomeSlot,
        expected_id: &str,
        expected_sequence: u64,
    ) -> Result<AcceptedSnapshot, String> {
        for _ in 0..1_200 {
            #[cfg(all(any(test, feature = "test-support"), target_arch = "wasm32"))]
            test_hooks::run_pending(self).await;
            if let Some(result) = outcome.borrow_mut().take() {
                if result != TerminalOutcome::Completed {
                    return Err(format!(
                        "Could not save the replacement keyboard: {result:?}"
                    ));
                }
                return self
                    .accepted_snapshot()
                    .filter(|accepted| {
                        self.open_sequence.get() == expected_sequence
                            && accepted.document.id == expected_id
                    })
                    .ok_or_else(|| "A newer project open superseded the replacement.".into());
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
        self.spawn_project_open(move |this| async move {
            let buffer = JsFuture::from(file.array_buffer()).await;
            if this.open_sequence.get() != sequence {
                return;
            }
            let result = buffer
                .map_err(|error| format!("Import read failed: {error:?}"))
                .and_then(|buffer| this.import_archive(Uint8Array::new(&buffer).to_vec()));
            if let Err(error) = result {
                this.apply_report(RuntimeReport::status(error).transient_error());
            }
        });
    }

    fn import_archive(self: &Rc<Self>, bytes: Vec<u8>) -> Result<(), String> {
        let (metadata, mut buffers) =
            boardstudio_core::archive::request("{\"kind\":\"unpack-project\"}", &[bytes]);
        let reply = serde_json::from_str(&metadata).map_err(|error| error.to_string())?;
        let ArchiveReply::Unpacked {
            project_json,
            assets,
        } = reply
        else {
            return Err(format!("Archive rejected: {reply:?}"));
        };
        let document = serde_json::from_str(&project_json).map_err(|error| error.to_string())?;
        let copied = assets
            .into_iter()
            .map(|asset| {
                buffers
                    .get_mut(asset.buffer_index as usize)
                    .map(|bytes| (asset.sha256, std::mem::take(bytes)))
                    .ok_or("Archive omitted an asset buffer")
            })
            .collect::<Result<BTreeMap<_, _>, _>>()?;
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
        self.spawn_project_open(move |this| async move {
            let loaded = this.load_saved(id).await;
            if this.open_sequence.get() != sequence {
                return;
            }
            let result = (|| {
                let saved = loaded?;
                this.confirm_project_replacement(&saved.document)?;
                this.open_document(this.remember_saved(saved), false);
                Ok::<(), String>(())
            })();
            if let Err(error) = result {
                this.report(format!("Could not open the saved keyboard: {error}"));
            }
        });
    }

    fn spawn_project_open<F: Future<Output = ()> + 'static>(
        self: &Rc<Self>,
        open: impl FnOnce(Rc<Self>) -> F,
    ) {
        #[cfg(any(test, feature = "test-support"))]
        let observer = self
            .test
            .in_process_adapters
            .borrow()
            .as_ref()
            .and_then(|adapters| adapters.take_open_observer());
        let work = open(self.clone());
        spawn_local(async move {
            work.await;
            #[cfg(any(test, feature = "test-support"))]
            if let Some(done) = observer {
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
            runtime.session.borrow().read_model().lifecycle == expected_lifecycle
                && runtime.snapshot_current(expected.token, expected_scope)
        };
        if !still_expected(self) {
            return Err("A newer project open superseded recovery of the original project.".into());
        }
        let sequence = self.begin_open()?;
        let document = self.load_saved(id.clone()).await?;
        if self.open_sequence.get() != sequence || !still_expected(self) {
            return Err("A newer project open superseded recovery of the original project.".into());
        }
        let document = self.remember_saved(document);
        let outcome = self.open_document(document, false);
        self.wait_for_project_open(outcome, &id, sequence).await
    }

    async fn load_saved(&self, id: String) -> Result<SavedProject, String> {
        self.store
            .load_document_with_token(id)
            .await
            .map_err(|error| error.to_string())?
            .ok_or_else(|| "The saved keyboard is unavailable.".into())
    }

    pub(super) fn remember_saved(&self, saved: SavedProject) -> ProjectDoc {
        self.persistence
            .borrow()
            .observe_token(saved.document.id.clone(), saved.token);
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
        CoreEngine::validate_open(document)?;
        let model = self.model();
        if matches!(model.durability, Durability::Saving { .. }) {
            return Err("Save the current keyboard before switching projects.".into());
        }
        let save_failed = matches!(model.durability, Durability::Failed { .. });
        if model.accepted.is_none() && !save_failed {
            return Ok(());
        }
        let message = if save_failed {
            format!(
                r#"Open "{}"? Your latest edits could not be saved and will be discarded."#,
                document.name
            )
        } else {
            format!(
                r#"Open "{}"? Uncommitted form drafts will be discarded."#,
                document.name
            )
        };
        let confirmed = web_sys::window()
            .and_then(|window| window.confirm_with_message(&message).ok())
            .unwrap_or(false);
        confirmed
            .then_some(())
            .ok_or_else(|| "Project switch canceled; your current work was kept.".into())
    }

    pub(super) async fn pack_archive(
        &self,
        export_id: OperationId,
        snapshot: &AcceptedSnapshot,
        scope: &Scope,
        embed_used_models: bool,
    ) -> Result<Vec<u8>, String> {
        let guard = || self.require_export_current(export_id, snapshot, scope);
        let document = snapshot.document.as_ref();
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
            let bytes = self
                .load_document_asset(&asset.sha256)
                .await?
                .ok_or_else(|| format!("Missing asset: {}", asset.name))?;
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
            boardstudio_core::archive::request(&prepared.metadata, &prepared.buffers);
        match serde_json::from_str::<ArchiveReply>(&metadata).map_err(|e| e.to_string())? {
            ArchiveReply::Packed => buffers
                .into_iter()
                .next()
                .ok_or_else(|| "Archive returned no bytes.".into()),
            reply => Err(format!("Archive export rejected: {reply:?}")),
        }
    }
}
