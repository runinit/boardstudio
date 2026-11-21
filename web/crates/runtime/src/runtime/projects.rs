//! Project lifecycle: creating, opening, importing, deleting and recovering saved projects.
use super::*;

impl Runtime {
    /// Open a fresh blank keyboard through the normal Session persistence path.
    pub fn create_new_keyboard(
        self: &Rc<Self>,
    ) -> Result<(String, crate::operation_outcomes::OutcomeSlot), String> {
        self.create_new_keyboard_inner(false)
    }

    /// Open a fresh, locally persisted copy of a shipped demo project.
    pub fn open_demo_copy(self: &Rc<Self>, mut document: ProjectDoc) -> Result<(), String> {
        if self.storage_initializing.get() {
            return Err("Storage is still initializing; try again shortly.".into());
        }
        if self.project_deletion_pending.get() {
            return Err("A saved keyboard deletion is in progress.".into());
        }
        document.id = browser_uuid()?;
        document.revision = 0;
        self.confirm_project_replacement(&document)?;
        self.begin_open()?;
        let operation_id = self.operation();
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
        Ok(())
    }

    pub(super) fn create_new_keyboard_inner(
        self: &Rc<Self>,
        allow_project_deletion: bool,
    ) -> Result<(String, crate::operation_outcomes::OutcomeSlot), String> {
        if self.storage_initializing.get() {
            return Err("Storage is still initializing; try again shortly.".into());
        }
        if self.project_deletion_pending.get() && !allow_project_deletion {
            return Err("A saved keyboard deletion is in progress.".into());
        }
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

        if !allow_project_deletion {
            self.confirm_project_replacement(&document)?;
            // The confirm above explicitly discarded pending edits: drop the
            // retained download buffer now so a latched quota failure cannot
            // outlive recovery. Session drops its own retained save when the
            // RecoverWithDocument below is submitted; the Persist emitted
            // afterwards carries only the new document.
            self.pending_document.borrow_mut().take();
            self.begin_open()?;
        }

        let operation_id = self.operation();
        let outcome = self.observe_operation(operation_id);
        if allow_project_deletion {
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
        Ok((project_id, outcome))
    }

    pub async fn delete_saved_project(self: &Rc<Self>, project_id: String) -> Result<(), String> {
        let recovery = self.model().lifecycle == Lifecycle::RecoveryRequired;
        if recovery && !self.confirm_discard_pending() {
            return Err("Pending edits were kept; no keyboard was deleted.".into());
        }
        let lease = self.begin_project_deletion()?;
        self.supersede_pending_opens_for_deletion()?;
        if recovery {
            // Quota escape hatch: the confirm above explicitly discarded pending
            // work, so remove the single key without rewriting durable state
            // first. Recovery and replacement writes re-hit a full origin and
            // abort, which used to make in-app deletion impossible. Opening a
            // replacement stays a later, explicit user action; the retained
            // pending doc stays available for explicit retry or download.
            self.store
                .delete_project_by_key(project_id.clone())
                .await
                .map_err(|error| {
                    format!("This keyboard could not be deleted. Try again. {error}")
                })?;
            self.saved_tokens.borrow_mut().remove(&project_id);
            drop(lease);
            return Ok(());
        }
        let initial = self.wait_for_saved_active_document().await?;

        let deleting_active = initial.document.id == project_id;
        let expected_delete = if deleting_active {
            self.saved_tokens.borrow().get(&project_id).cloned()
        } else {
            self.store.observed_token(&project_id)
        };
        let expected_current = if deleting_active {
            let mut remaining = self
                .store
                .list_documents()
                .await
                .map_err(|error| format!("Could not list saved keyboards: {error}"))?;
            remaining.retain(|document| document.id != project_id);
            remaining.sort_by(|left, right| {
                JsString::from(left.name.clone())
                    .locale_compare(&right.name, &Array::new(), &Object::new())
                    .cmp(&0)
            });

            if let Some(first) = remaining.first() {
                let replacement_id = first.id.clone();
                let replacement = self
                    .store
                    .load_document_with_token(replacement_id.clone())
                    .await
                    .map_err(|error| format!("Could not open the next saved keyboard: {error}"))?
                    .ok_or_else(|| {
                        "The next saved keyboard is unavailable; the current keyboard was kept."
                            .to_owned()
                    })?;
                self.ensure_delete_owner(&initial)?;
                let replacement = self.remember_saved(replacement);
                match self
                    .open_project_for_deletion(replacement, replacement_id)
                    .await
                {
                    Ok(accepted) => Some(accepted),
                    Err(error) => {
                        if self.model().lifecycle == Lifecycle::RecoveryRequired
                            && self.ensure_delete_owner(&initial).is_ok()
                            && let Err(restore_error) =
                                self.recover_active_project_for_deletion().await
                        {
                            return Err(format!(
                                "{error} The current keyboard could not be restored: {restore_error}"
                            ));
                        }
                        return Err(error);
                    }
                }
            } else {
                self.ensure_delete_owner(&initial)?;
                let (replacement_id, outcome) = self.create_new_keyboard_inner(true)?;
                Some(
                    self.wait_for_project_open(
                        outcome,
                        &replacement_id,
                        Some(self.open_sequence.get()),
                    )
                    .await?,
                )
            }
        } else {
            None
        };

        let current = self
            .model()
            .accepted
            .ok_or_else(|| "The active keyboard changed before deletion completed.".to_owned())?;
        if let Some(expected) = expected_current {
            if current.document.id != expected.document.id
                || current.session_epoch != expected.session_epoch
                || current.token != expected.token
                || current.document.revision != expected.document.revision
            {
                return Err(
                    "The replacement keyboard changed before deletion completed; the saved keyboard was kept."
                        .into(),
                );
            }
        } else if current.document.id != initial.document.id
            || current.session_epoch != initial.session_epoch
        {
            return Err(
                "The active keyboard changed before deletion completed; the saved keyboard was kept."
                    .into(),
            );
        }

        self.store
            .delete_project_expected(project_id.clone(), expected_delete.as_ref())
            .await
            .map_err(|error| format!("This keyboard could not be deleted. Try again. {error}"))?;
        self.saved_tokens.borrow_mut().remove(&project_id);
        drop(lease);
        Ok(())
    }

    pub(super) fn begin_project_deletion(self: &Rc<Self>) -> Result<ProjectDeletionLease, String> {
        if self.project_deletion_pending.replace(true) {
            return Err("Another saved keyboard deletion is already in progress.".into());
        }
        Ok(ProjectDeletionLease {
            runtime: self.clone(),
        })
    }

    pub(super) fn supersede_pending_opens_for_deletion(&self) -> Result<u64, String> {
        let sequence = self
            .open_sequence
            .get()
            .checked_add(1)
            .ok_or_else(|| "Open identity exhausted.".to_owned())?;
        self.open_sequence.set(sequence);
        Ok(sequence)
    }

    pub(super) fn ensure_delete_owner(&self, initial: &AcceptedSnapshot) -> Result<(), String> {
        let current = self
            .model()
            .accepted
            .ok_or_else(|| "The active keyboard changed before deletion completed.".to_owned())?;
        if current.document.id != initial.document.id
            || current.session_epoch != initial.session_epoch
        {
            return Err(
                "The active keyboard changed before deletion completed; the saved keyboard was kept."
                    .into(),
            );
        }
        Ok(())
    }

    pub(super) async fn wait_for_saved_active_document(
        self: &Rc<Self>,
    ) -> Result<AcceptedSnapshot, String> {
        for _ in 0..1_200 {
            let model = self.model();
            if model.lifecycle == Lifecycle::Ready
                && let Some(accepted) = model.accepted
                && matches!(
                    model.durability,
                    Durability::Saved { revision } if revision == accepted.document.revision
                )
            {
                return Ok(accepted);
            }
            if model.lifecycle == Lifecycle::RecoveryRequired {
                return Err(
                    "Recover or open a saved keyboard before deleting the current keyboard.".into(),
                );
            }
            TimeoutFuture::new(25).await;
        }
        Err("The active keyboard did not finish saving; no saved keyboard was deleted.".into())
    }

    pub(super) async fn open_project_for_deletion(
        self: &Rc<Self>,
        document: ProjectDoc,
        expected_id: String,
    ) -> Result<AcceptedSnapshot, String> {
        let sequence = self.supersede_pending_opens_for_deletion()?;
        let operation_id = self.operation();
        let outcome = self.observe_operation(operation_id);
        self.project_deletion_open.set(Some(operation_id));
        if self.model().lifecycle == Lifecycle::RecoveryRequired {
            self.submit(Event::RecoverWithDocument {
                operation_id,
                document,
            });
        } else {
            self.submit(Event::Open {
                operation_id,
                document,
            });
        }
        self.wait_for_project_open(outcome, &expected_id, Some(sequence))
            .await
    }

    pub(super) async fn recover_active_project_for_deletion(
        self: &Rc<Self>,
    ) -> Result<AcceptedSnapshot, String> {
        let accepted = self.model().accepted.ok_or_else(|| {
            "Recover or open a saved keyboard before deleting the current keyboard.".to_owned()
        })?;
        let document = self
            .store
            .load_document_with_token(accepted.document.id.clone())
            .await
            .map_err(|error| format!("Could not restore the current saved keyboard: {error}"))?
            .ok_or_else(|| {
                "The current keyboard has no durable saved copy; deletion was cancelled.".to_owned()
            })?;
        let current = self
            .model()
            .accepted
            .ok_or_else(|| "The active keyboard changed during deletion recovery.".to_owned())?;
        if current.document.id != accepted.document.id || current.token != accepted.token {
            return Err("The active keyboard changed during deletion recovery.".into());
        }
        let document = self.remember_saved(document);
        let operation_id = self.operation();
        let outcome = self.observe_operation(operation_id);
        let sequence = self.open_sequence.get();
        self.project_deletion_open.set(Some(operation_id));
        self.submit(Event::RecoverWithDocument {
            operation_id,
            document,
        });
        self.wait_for_project_open(outcome, &accepted.document.id, Some(sequence))
            .await
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
                Ok(buffer) => {
                    this.import_archive_at(Uint8Array::new(&buffer).to_vec(), sequence, None)
                        .await
                }
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
    pub(super) async fn import_archive_at(
        self: &Rc<Self>,
        bytes: Vec<u8>,
        sequence: u64,
        fresh_copy_id: Option<String>,
    ) -> Result<(), String> {
        let operation = self.operation();
        let core = self.core.borrow().clone();
        let result = core
            .archive(
                &format!("import-{}", operation.0),
                "1",
                "{\"kind\":\"unpack-project\"}",
                vec![Uint8Array::from(bytes.as_slice())],
            )
            .await
            .map_err(|e| e.to_string());
        let result = result?;
        let reply: ArchiveReply =
            serde_json::from_str(&result.metadata).map_err(|e| e.to_string())?;
        let ArchiveReply::Unpacked {
            project_json,
            assets,
        } = reply
        else {
            return Err(format!("Archive rejected: {reply:?}"));
        };
        let mut document: ProjectDoc =
            serde_json::from_str(&project_json).map_err(|e| e.to_string())?;
        if let Some(copy_id) = fresh_copy_id {
            document.id = copy_id;
        }
        let mut copied = BTreeMap::new();
        for asset in assets {
            let bytes = result
                .buffers
                .get(asset.buffer_index as usize)
                .ok_or("Archive omitted an asset buffer")?;
            copied.insert(asset.sha256, bytes.to_vec());
        }
        if self.open_sequence.get() != sequence {
            return Err("Open superseded by another import.".into());
        }
        self.confirm_project_replacement(&document)?;
        self.assets.borrow_mut().extend(copied);
        self.submit(if self.model().lifecycle == Lifecycle::RecoveryRequired {
            Event::RecoverWithDocument {
                operation_id: operation,
                document,
            }
        } else {
            Event::Open {
                operation_id: operation,
                document,
            }
        });
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
                            let operation_id = this.operation();
                            if this.model().lifecycle == Lifecycle::RecoveryRequired {
                                this.submit(Event::RecoverWithDocument {
                                    operation_id,
                                    document,
                                });
                            } else {
                                this.submit(Event::Open {
                                    operation_id,
                                    document,
                                });
                            }
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
        let operation_id = self.operation();
        let outcome = self.observe_operation(operation_id);
        if self.model().lifecycle == Lifecycle::RecoveryRequired {
            self.submit(Event::RecoverWithDocument {
                operation_id,
                document,
            });
        } else {
            self.submit(Event::Open {
                operation_id,
                document,
            });
        }
        self.wait_for_project_open(outcome, &id, Some(sequence))
            .await
    }

    pub(super) fn remember_saved(&self, saved: SavedProject) -> ProjectDoc {
        self.saved_tokens
            .borrow_mut()
            .insert(saved.document.id.clone(), saved.token);
        saved.document
    }

    pub fn has_save_conflict(&self) -> bool {
        self.save_conflict.get()
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

    pub fn download_pending_edits(self: &Rc<Self>) {
        let this = self.clone();
        spawn_local(async move {
            let result = this.pending_archive().await.and_then(|(document, bytes)| {
                deliver(
                    &bytes,
                    &archive_filename(&format!("{}-pending", document.name)),
                    None,
                )
            });
            if let Err(error) = result {
                this.apply_report(
                    RuntimeReport::status(format!(
                        "Pending edits could not be downloaded: {error}"
                    ))
                    .transient_error(),
                );
            }
        });
    }

    pub(super) async fn pending_archive(&self) -> Result<(Arc<ProjectDoc>, Vec<u8>), String> {
        let document = self
            .pending_document
            .borrow()
            .clone()
            .ok_or("There are no pending edits to download.")?;
        let guard = || {
            if self.model().lifecycle == Lifecycle::RecoveryRequired
                && self
                    .pending_document
                    .borrow()
                    .as_ref()
                    .is_some_and(|pending| Arc::ptr_eq(pending, &document))
            {
                Ok(())
            } else {
                Err("Pending edits changed during export; download them again.".into())
            }
        };
        let bytes = self.pack_document(&document, true, guard).await?;
        guard()?;
        Ok((document, bytes))
    }

    pub(super) fn confirm_discard_pending(&self) -> bool {
        self.pending_document.borrow().is_none() || web_sys::window().and_then(|window| window.confirm_with_message(
            "Discard your pending edits and reload the saved keyboard? Download your pending edits first if you want to keep them."
        ).ok()).unwrap_or(false)
    }

    pub(super) fn confirm_project_replacement(&self, document: &ProjectDoc) -> Result<(), String> {
        (self.core_services.validate_open)(document)?;
        let model = self.model();
        let pending = self.has_pending_edits();
        if model.accepted.is_none() && !pending {
            return Ok(());
        }
        let message = if model.lifecycle == Lifecycle::RecoveryRequired && pending {
            format!(
                "Discard your pending edits and open \"{}\"? Download your pending edits first if you want to keep them.",
                document.name
            )
        } else {
            format!(
                "Open \"{}\"? Any uncommitted drafts will be discarded. Owned edits and saves will finish before switching.",
                document.name
            )
        };
        let confirmed = web_sys::window()
            .and_then(|window| window.confirm_with_message(&message).ok())
            .unwrap_or(false);
        if confirmed {
            Ok(())
        } else {
            Err("Project switch canceled; your current work was kept.".into())
        }
    }

    pub fn has_pending_edits(&self) -> bool {
        self.pending_document.borrow().is_some()
    }

    pub fn recover_saved(self: &Rc<Self>) {
        if !self.confirm_discard_pending() {
            return;
        }
        let accepted_token = self.model().accepted.map(|accepted| accepted.token);
        let pending = self.pending_document.borrow().clone();
        let project_id = pending
            .as_ref()
            .map(|document| document.id.clone())
            .or_else(|| {
                self.model()
                    .accepted
                    .map(|accepted| accepted.document.id.clone())
            });
        let Some(project_id) = project_id else {
            self.report("No durable document exists to recover; reopen an explicit saved copy.");
            return;
        };
        let this = self.clone();
        spawn_local(async move {
            if this.document_core.is_none() {
                let core = this.core.borrow().clone();
                if let Err(error) = core.ready().await {
                    this.report(format!("Recovery executor unavailable: {error}"));
                    return;
                }
            }
            match this.store.load_document_with_token(project_id).await {
                Ok(Some(saved))
                    if this.model().lifecycle == Lifecycle::RecoveryRequired
                        && this.model().accepted.map(|accepted| accepted.token)
                            == accepted_token
                        && match (this.pending_document.borrow().as_ref(), pending.as_ref()) {
                            (Some(current), Some(expected)) => Arc::ptr_eq(current, expected),
                            (None, None) => true,
                            _ => false,
                        } =>
                {
                    let document = this.remember_saved(saved);
                    this.submit(Event::RecoverWithDocument {
                        operation_id: this.operation(),
                        document,
                    });
                }
                Ok(_) => {
                    this.report("Durable recovery copy is unavailable or the session changed.")
                }
                Err(error) => this.report(error.to_string()),
            }
        });
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
        let operation = self.operation();
        let core = self.core.borrow().clone();
        let buffers = prepared
            .buffers
            .iter()
            .map(|bytes| Uint8Array::from(bytes.as_slice()))
            .collect();
        let result = core
            .archive(
                &format!("pack-{}", operation.0),
                "1",
                &prepared.metadata,
                buffers,
            )
            .await
            .map_err(|e| e.to_string())?;
        match serde_json::from_str::<ArchiveReply>(&result.metadata).map_err(|e| e.to_string())? {
            ArchiveReply::Packed => result
                .buffers
                .first()
                .map(Uint8Array::to_vec)
                .ok_or_else(|| "Archive returned no bytes.".into()),
            reply => Err(format!("Archive export rejected: {reply:?}")),
        }
    }
}
