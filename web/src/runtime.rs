//! Browser composition runs identified effects; the headless session remains authoritative.
use boardstudio_application::{
    AcceptedSnapshot, Completion, Effect, Event, JobId, Lifecycle, OperationId, ReadModel,
    SaveResult, Scope, Session, SnapshotToken, TerminalOutcome,
};
use boardstudio_core::model::{ArchiveReply, ProjectDoc};
use boardstudio_web::host::{BrowserStore, CoreWorker};
use boardstudio_web::{
    cad_jobs::{
        CadJobError, CadOperation, CadRequest, CadResult, prepare_captured_case,
        prepare_captured_step_assembly, validate_reply,
    },
    cad_worker::CadWorker,
};

pub struct CadScene {
    pub scope: Scope,
    pub token: SnapshotToken,
    pub snapshot: AcceptedSnapshot,
    pub result: CadResult,
    pub mechanical: Option<boardstudio_core::model::MechanicalAssembly>,
    pub exact: bool,
}
use js_sys::{Array, Uint8Array};
use std::{
    cell::{Cell, RefCell},
    collections::{BTreeMap, BTreeSet, VecDeque},
    rc::Rc,
};
use wasm_bindgen::{JsCast, closure::Closure};
use wasm_bindgen_futures::{JsFuture, spawn_local};
use web_sys::{Blob, HtmlAnchorElement, SvgElement, Url};

type Notifier = Rc<dyn Fn()>;
type Frame = (i32, Closure<dyn FnMut(f64)>);
struct Artifact {
    bytes: Vec<u8>,
    filename: String,
    scope: Scope,
    token: SnapshotToken,
}

impl PartialEq for CadScene {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

pub struct Runtime {
    session: RefCell<Session>,
    core: RefCell<Rc<CoreWorker>>,
    pub store: BrowserStore,
    next_operation: Cell<u64>,
    assets: RefCell<BTreeMap<String, Vec<u8>>>,
    artifacts: RefCell<BTreeMap<String, Artifact>>,
    cancelled_exports: RefCell<BTreeSet<OperationId>>,
    frames: RefCell<BTreeMap<u64, Frame>>,
    surface: RefCell<Option<SvgElement>>,
    notify: RefCell<Option<Notifier>>,
    status: RefCell<String>,
    open_sequence: Cell<u64>,
    cad_scene: RefCell<Option<Rc<CadScene>>>,
    cad_worker: RefCell<Option<(Scope, Rc<CadWorker>)>>,
    cad_jobs: RefCell<BTreeMap<JobId, Rc<Cell<bool>>>>,
    step_exports: RefCell<BTreeSet<OperationId>>,
    export_workers: RefCell<BTreeMap<OperationId, Rc<CadWorker>>>,
}
impl Runtime {
    pub fn new() -> Result<Rc<Self>, String> {
        let prefix = deployment_prefix()?;
        let runtime = Rc::new(Self {
            session: RefCell::new(Session::new()),
            core: RefCell::new(Rc::new(
                CoreWorker::new(&resource_url("assets/core-worker/entry.js")?)
                    .map_err(|e| e.to_string())?,
            )),
            store: BrowserStore::scoped(if prefix == "/" { "root" } else { "boardstudio" })
                .map_err(|e| e.to_string())?,
            next_operation: Cell::new(1),
            assets: RefCell::new(BTreeMap::new()),
            artifacts: RefCell::new(BTreeMap::new()),
            cancelled_exports: RefCell::new(BTreeSet::new()),
            frames: RefCell::new(BTreeMap::new()),
            surface: RefCell::new(None),
            notify: RefCell::new(None),
            status: RefCell::new("Open a saved keyboard or an editable demo copy.".into()),
            open_sequence: Cell::new(0),
            cad_scene: RefCell::new(None),
            cad_worker: RefCell::new(None),
            cad_jobs: RefCell::new(BTreeMap::new()),
            step_exports: RefCell::new(BTreeSet::new()),
            export_workers: RefCell::new(BTreeMap::new()),
        });
        let weak = Rc::downgrade(&runtime);
        spawn_local(async move {
            if let Err(error) = boardstudio_web::host::register_offline(prefix).await
                && let Some(runtime) = weak.upgrade()
            {
                runtime.report(format!("Offline setup failed; this keyboard still needs an online connection: {error:?}"));
            }
        });
        Ok(runtime)
    }
    pub fn operation(&self) -> OperationId {
        let id = self.next_operation.get();
        self.next_operation
            .set(id.checked_add(1).expect("operation identity exhausted"));
        OperationId(id)
    }
    pub fn scope(&self) -> Option<boardstudio_application::Scope> {
        self.session.borrow().scope()
    }
    pub fn model(&self) -> ReadModel {
        self.session.borrow().read_model().clone()
    }
    pub fn status(&self) -> String {
        self.status.borrow().clone()
    }
    pub fn subscribe(&self, notify: Notifier) {
        *self.notify.borrow_mut() = Some(notify);
    }
    pub fn unsubscribe(&self) {
        self.notify.borrow_mut().take();
    }
    pub fn surface(&self, surface: SvgElement) {
        *self.surface.borrow_mut() = Some(surface);
    }
    pub fn report(&self, status: impl Into<String>) {
        *self.status.borrow_mut() = status.into();
        self.changed();
    }
    fn changed(&self) {
        if let Some(notify) = self.notify.borrow().as_ref().cloned() {
            notify();
        }
    }
    pub fn submit(self: &Rc<Self>, event: Event) {
        let previous_scope = self.scope();
        let effects = self.session.borrow_mut().submit(event);
        if self.scope() != previous_scope {
            self.cad_scene.borrow_mut().take();
            if let Some((_, worker)) = self.cad_worker.borrow_mut().take() {
                worker.close();
            }
        }
        self.changed();
        self.drive(effects);
    }
    fn complete(self: &Rc<Self>, event: Completion) -> Vec<Effect> {
        let effects = self.session.borrow_mut().complete(event);
        self.changed();
        effects
    }
    fn drive(self: &Rc<Self>, effects: Vec<Effect>) {
        let this = self.clone();
        spawn_local(async move {
            let mut effects = VecDeque::from(effects);
            while let Some(effect) = effects.pop_front() {
                effects.extend(this.run(effect).await);
            }
            if this.model().lifecycle == Lifecycle::Closed {
                this.cancel_frames();
                if let Some((_, worker)) = this.cad_worker.borrow_mut().take() {
                    worker.close();
                }
                for (_, worker) in std::mem::take(&mut *this.export_workers.borrow_mut()) {
                    worker.close();
                }
                this.core.borrow().close();
            }
        });
    }
    async fn run(self: &Rc<Self>, effect: Effect) -> Vec<Effect> {
        match effect {
            Effect::Core {
                request_id,
                executor_epoch,
                request,
                ..
            } => {
                let core = self.core.borrow().clone();
                match core
                    .request(
                        &format!("m1-{}", request_id.0),
                        &executor_epoch.0.to_string(),
                        &request,
                    )
                    .await
                {
                    Ok(reply) => self.complete(Completion::Core {
                        request_id,
                        executor_epoch,
                        reply: Box::new(reply),
                    }),
                    Err(error) => self.complete(Completion::CoreFailed {
                        request_id,
                        executor_epoch,
                        reason: error.to_string(),
                    }),
                }
            }
            Effect::RestartCoreExecutor { executor_epoch } => {
                self.core.borrow().close();
                match resource_url("assets/core-worker/entry.js")
                    .and_then(|url| CoreWorker::new(&url).map_err(|e| e.to_string()))
                {
                    Ok(core) => {
                        let core = Rc::new(core);
                        *self.core.borrow_mut() = core.clone();
                        match core.ready().await {
                            Ok(()) => {
                                self.complete(Completion::ExecutorRestarted { executor_epoch })
                            }
                            Err(error) => {
                                self.report(format!("Executor restart failed: {error}"));
                                vec![]
                            }
                        }
                    }
                    Err(error) => {
                        self.report(error);
                        vec![]
                    }
                }
            }
            Effect::Persist {
                document,
                save_attempt_id,
                ..
            } => {
                let assets = self
                    .assets
                    .borrow()
                    .iter()
                    .filter(|(hash, _)| document.assets.iter().any(|a| &a.sha256 == *hash))
                    .map(|(hash, bytes)| (hash.clone(), bytes.clone()))
                    .collect();
                let result = match self.store.save_document(&document, &assets).await {
                    Ok(()) => {
                        for asset in &document.assets {
                            self.assets.borrow_mut().remove(&asset.sha256);
                        }
                        SaveResult::Committed
                    }
                    Err(error) => SaveResult::Aborted(error.to_string()),
                };
                self.complete(Completion::Persist {
                    save_attempt_id,
                    result,
                })
            }
            Effect::Settled {
                operation_id,
                outcome,
            } => {
                self.step_exports.borrow_mut().remove(&operation_id);
                match outcome {
                    TerminalOutcome::Completed => self.report("Saved locally."),
                    TerminalOutcome::Rejected(reason)
                    | TerminalOutcome::PersistenceFailed(reason)
                    | TerminalOutcome::BlockedByRecovery(reason)
                    | TerminalOutcome::ExecutorFailed(reason) => self.report(reason),
                    TerminalOutcome::Cancelled => self.report("Cancelled."),
                    TerminalOutcome::Closed => self.report("Editor closed."),
                    TerminalOutcome::Superseded => {}
                }
                vec![]
            }
            Effect::CapturePointer { pointer_id } => {
                if let Some(svg) = self.surface.borrow().as_ref()
                    && let Ok(pointer) = i32::try_from(pointer_id)
                {
                    let result = svg.set_pointer_capture(pointer);
                    if let Err(error) = result {
                        self.report(format!("Pointer capture failed: {error:?}"));
                    }
                }
                vec![]
            }
            Effect::ReleasePointer { pointer_id } => {
                if let Some(svg) = self.surface.borrow().as_ref()
                    && let Ok(pointer) = i32::try_from(pointer_id)
                    && svg.has_pointer_capture(pointer)
                {
                    let _ = svg.release_pointer_capture(pointer);
                }
                vec![]
            }
            Effect::RequestFrame { generation } => {
                if !self.frames.borrow().contains_key(&generation) {
                    let weak = Rc::downgrade(self);
                    let callback = Closure::<dyn FnMut(f64)>::new(move |_| {
                        if let Some(this) = weak.upgrade() {
                            this.submit(Event::GestureFrame { generation });
                            // Drop the executing callback after the browser returns from it.
                            spawn_local(async move {
                                this.frames.borrow_mut().remove(&generation);
                            });
                        }
                    });
                    match web_sys::window()
                        .ok_or_else(|| "window unavailable".to_owned())
                        .and_then(|window| {
                            window
                                .request_animation_frame(callback.as_ref().unchecked_ref())
                                .map_err(|e| format!("{e:?}"))
                        }) {
                        Ok(handle) => {
                            self.frames
                                .borrow_mut()
                                .insert(generation, (handle, callback));
                        }
                        Err(error) => self.report(error),
                    }
                }
                vec![]
            }
            Effect::CancelFrame { generation } => {
                if let Some((handle, _)) = self.frames.borrow_mut().remove(&generation)
                    && let Some(window) = web_sys::window()
                {
                    let _ = window.cancel_animation_frame(handle);
                }
                vec![]
            }
            Effect::RunExport {
                operation_id,
                scope,
                snapshot,
            } => match if self.step_exports.borrow().contains(&operation_id) {
                self.step_bytes(operation_id, &snapshot, &scope).await
            } else {
                self.pack_archive(&snapshot.document).await
            } {
                Ok(bytes) => {
                    let current = self.session.borrow().scope() == Some(scope.clone())
                        && self
                            .model()
                            .accepted
                            .as_ref()
                            .is_some_and(|s| s.token == snapshot.token);
                    let cancelled = self.cancelled_exports.borrow_mut().remove(&operation_id);
                    if !current || cancelled {
                        return self.complete(Completion::ExportFailed {
                            operation_id,
                            reason: "Export scope changed before delivery.".into(),
                        });
                    }
                    let artifact_id = format!("archive-{}", operation_id.0);
                    self.artifacts.borrow_mut().insert(
                        artifact_id.clone(),
                        Artifact {
                            bytes,
                            filename: if self.step_exports.borrow().contains(&operation_id) {
                                "keyboard.step"
                            } else {
                                "keyboard.boardstudio"
                            }
                            .into(),
                            scope: scope.clone(),
                            token: snapshot.token,
                        },
                    );
                    self.complete(Completion::ExportFinished {
                        operation_id,
                        token: snapshot.token,
                        scope,
                        artifact_id,
                    })
                }
                Err(reason) => {
                    self.cancelled_exports.borrow_mut().remove(&operation_id);
                    self.complete(Completion::ExportFailed {
                        operation_id,
                        reason,
                    })
                }
            },
            Effect::DeliverExport {
                artifact_id, token, ..
            } => {
                let artifact = self.artifacts.borrow_mut().remove(&artifact_id);
                if let Some(artifact) = artifact
                    && token == artifact.token
                    && self.scope() == Some(artifact.scope)
                    && self
                        .model()
                        .accepted
                        .as_ref()
                        .is_some_and(|s| s.token == token)
                    && let Err(error) = deliver(&artifact.bytes, &artifact.filename)
                {
                    self.report(error);
                }
                vec![]
            }
            Effect::RunGeneration {
                job_id,
                scope,
                snapshot,
                ..
            } => self.generate(job_id, scope, snapshot).await,
            Effect::CancelJob { job_id } => {
                if let Some(cancelled) = self.cad_jobs.borrow_mut().remove(&job_id) {
                    cancelled.set(true);
                }
                if let Some((_, worker)) = self.cad_worker.borrow_mut().take() {
                    worker.cancel(&format!("case-{}", job_id.0));
                    worker.close();
                }
                vec![]
            }
            Effect::CancelExport { operation_id } => {
                self.cancelled_exports.borrow_mut().insert(operation_id);
                if let Some(worker) = self.export_workers.borrow_mut().remove(&operation_id) {
                    worker.close();
                }
                self.artifacts
                    .borrow_mut()
                    .remove(&format!("archive-{}", operation_id.0));
                vec![]
            }
        }
    }
    pub fn cad_scene(&self) -> Option<Rc<CadScene>> {
        self.cad_scene
            .borrow()
            .as_ref()
            .filter(|scene| self.scope() == Some(scene.scope.clone()))
            .cloned()
    }
    pub fn export_step(self: &Rc<Self>) {
        let Some(scope) = self.scope() else {
            return;
        };
        let operation_id = self.operation();
        self.step_exports.borrow_mut().insert(operation_id);
        self.submit(Event::StartExport {
            operation_id,
            scope,
        });
    }
    fn snapshot_current(&self, token: SnapshotToken, scope: &Scope) -> bool {
        self.scope().as_ref() == Some(scope)
            && self
                .model()
                .accepted
                .as_ref()
                .is_some_and(|s| s.token == token)
    }
    async fn generate(
        self: &Rc<Self>,
        job_id: JobId,
        scope: Scope,
        snapshot: AcceptedSnapshot,
    ) -> Vec<Effect> {
        let cancelled = Rc::new(Cell::new(false));
        self.cad_jobs.borrow_mut().insert(job_id, cancelled.clone());
        let result = self
            .generate_current(job_id, &scope, &snapshot, &cancelled)
            .await;
        self.cad_jobs.borrow_mut().remove(&job_id);
        match result {
            Ok(()) => self.complete(Completion::GenerationFinished {
                job_id,
                scope,
                exact: true,
            }),
            Err(CadJobError::Blocked(reason)) => self.complete(Completion::GenerationBlocked {
                job_id,
                scope,
                reason,
            }),
            Err(CadJobError::Failed(reason) | CadJobError::Stale(reason)) => {
                self.complete(Completion::GenerationFailed {
                    job_id,
                    scope,
                    reason,
                })
            }
            Err(CadJobError::Cancelled) => vec![],
        }
    }
    async fn generate_current(
        &self,
        job_id: JobId,
        scope: &Scope,
        snapshot: &AcceptedSnapshot,
        cancelled: &Cell<bool>,
    ) -> Result<(), CadJobError> {
        let guard = || {
            if cancelled.get() || !self.snapshot_current(snapshot.token, scope) {
                Err(CadJobError::Cancelled)
            } else {
                Ok(())
            }
        };
        guard()?;
        let core = self.core.borrow().clone();
        let epoch = self.session.borrow().core_executor_epoch().0.to_string();
        let prepared = prepare_captured_case(
            &core,
            &epoch,
            &format!("case-{}", job_id.0),
            snapshot,
            scope,
        )
        .await?;
        guard()?;
        let previous = self.cad_worker.borrow_mut().take();
        let worker = match previous {
            Some((worker_scope, worker)) if worker_scope == *scope => worker,
            Some((_, worker)) => {
                worker.close();
                Rc::new(
                    CadWorker::new(
                        &resource_url("assets/cad-worker/entry.js").map_err(CadJobError::Failed)?,
                    )
                    .map_err(|e| CadJobError::Failed(e.to_string()))?,
                )
            }
            None => Rc::new(
                CadWorker::new(
                    &resource_url("assets/cad-worker/entry.js").map_err(CadJobError::Failed)?,
                )
                .map_err(|e| CadJobError::Failed(e.to_string()))?,
            ),
        };
        *self.cad_worker.borrow_mut() = Some((scope.clone(), worker.clone()));
        worker
            .ready()
            .await
            .map_err(|e| CadJobError::Failed(e.to_string()))?;
        guard()?;
        for operation in [CadOperation::Preview, CadOperation::Exact] {
            let request = CadRequest {
                request_id: format!("case-{}-{operation:?}", job_id.0),
                job_id: format!("case-{}", job_id.0),
                identity: prepared.identity.clone(),
                operation,
                prepared: Some(prepared.prepared.clone()),
                input_bytes: vec![],
            };
            let reply = worker
                .request(request.clone())
                .await
                .map_err(|e| CadJobError::Failed(e.to_string()))?;
            guard()?;
            let result = validate_reply(&request, reply, &prepared.identity)?;
            *self.cad_scene.borrow_mut() = Some(Rc::new(CadScene {
                scope: scope.clone(),
                token: snapshot.token,
                snapshot: snapshot.clone(),
                result,
                mechanical: prepared.mechanical_assembly.clone(),
                exact: operation == CadOperation::Exact,
            }));
            self.changed();
        }
        Ok(())
    }
    async fn step_bytes(
        &self,
        operation_id: OperationId,
        snapshot: &AcceptedSnapshot,
        scope: &Scope,
    ) -> Result<Vec<u8>, String> {
        let guard = || {
            if self.cancelled_exports.borrow().contains(&operation_id)
                || !self.snapshot_current(snapshot.token, scope)
            {
                Err("STEP export was cancelled or superseded.".to_owned())
            } else {
                Ok(())
            }
        };
        guard()?;
        let core = self.core.borrow().clone();
        let epoch = self.session.borrow().core_executor_epoch().0.to_string();
        let prepared = prepare_captured_step_assembly(
            &core,
            &epoch,
            &format!("step-{}", operation_id.0),
            snapshot,
            scope,
        )
        .await
        .map_err(|e| format!("{e:?}"))?;
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
                request_id: format!("step-{}", operation_id.0),
                job_id: format!("step-{}", operation_id.0),
                identity: prepared.identity.clone(),
                operation: CadOperation::ExportStep,
                prepared: Some(prepared.prepared),
                input_bytes: vec![],
            };
            let reply = worker
                .request(request.clone())
                .await
                .map_err(|e| e.to_string())?;
            guard()?;
            validate_reply(&request, reply, &prepared.identity)
                .map(|result| result.step)
                .map_err(|e| format!("{e:?}"))
        }
        .await;
        worker.close();
        self.export_workers.borrow_mut().remove(&operation_id);
        result
    }
    fn cancel_frames(&self) {
        if let Some(window) = web_sys::window() {
            for (_, (handle, _)) in std::mem::take(&mut *self.frames.borrow_mut()) {
                let _ = window.cancel_animation_frame(handle);
            }
        }
    }
    pub fn open_fixture(self: &Rc<Self>, name: &str) {
        let name = name.to_owned();
        let Ok(sequence) = self.begin_open() else {
            self.report("Open identity exhausted.");
            return;
        };
        let this = self.clone();
        spawn_local(async move {
            match fetch_bytes(&format!("assets/fixtures/{name}.boardstudio")).await {
                Ok(bytes) => {
                    if let Err(error) = this.import_archive_at(bytes, sequence).await
                        && this.open_sequence.get() == sequence
                    {
                        this.report(error);
                    }
                }
                Err(error) if this.open_sequence.get() == sequence => this.report(error),
                Err(_) => {}
            }
        });
    }
    fn begin_open(&self) -> Result<u64, String> {
        let sequence = self
            .open_sequence
            .get()
            .checked_add(1)
            .ok_or("open identity exhausted")?;
        self.open_sequence.set(sequence);
        Ok(sequence)
    }
    pub fn import_file(self: &Rc<Self>, file: web_sys::File) {
        let Ok(sequence) = self.begin_open() else {
            self.report("Open identity exhausted.");
            return;
        };
        let this = self.clone();
        spawn_local(async move {
            let result = match JsFuture::from(file.array_buffer()).await {
                Ok(buffer) => {
                    this.import_archive_at(Uint8Array::new(&buffer).to_vec(), sequence)
                        .await
                }
                Err(error) => Err(format!("Import read failed: {error:?}")),
            };
            if let Err(error) = result
                && this.open_sequence.get() == sequence
            {
                this.report(error);
            }
        });
    }
    async fn import_archive_at(
        self: &Rc<Self>,
        bytes: Vec<u8>,
        sequence: u64,
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
            .map_err(|e| e.to_string())?;
        let reply: ArchiveReply =
            serde_json::from_str(&result.metadata).map_err(|e| e.to_string())?;
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
            let bytes = result
                .buffers
                .get(asset.buffer_index as usize)
                .ok_or("Archive omitted an asset buffer")?;
            copied.insert(asset.sha256, bytes.to_vec());
        }
        if self.open_sequence.get() != sequence {
            return Err("Open superseded by another import.".into());
        }
        self.assets.borrow_mut().extend(copied);
        self.submit(Event::Open {
            operation_id: operation,
            document,
        });
        Ok(())
    }
    pub fn open_saved(self: &Rc<Self>, id: String) {
        let Ok(sequence) = self.begin_open() else {
            self.report("Open identity exhausted.");
            return;
        };
        let this = self.clone();
        spawn_local(async move {
            match this.store.load_document(id).await {
                Ok(Some(document)) if this.open_sequence.get() == sequence => {
                    this.submit(Event::Open {
                        operation_id: this.operation(),
                        document,
                    })
                }
                Ok(Some(_)) => {}
                Ok(None) if this.open_sequence.get() == sequence => {
                    this.report("Saved keyboard is unavailable.")
                }
                Err(error) if this.open_sequence.get() == sequence => {
                    this.report(error.to_string())
                }
                Ok(None) | Err(_) => {}
            }
        });
    }
    pub fn recover_saved(self: &Rc<Self>) {
        let Some(accepted) = self.model().accepted else {
            self.report("No durable document exists to recover; reopen an explicit saved copy.");
            return;
        };
        let this = self.clone();
        spawn_local(async move {
            let core = this.core.borrow().clone();
            if let Err(error) = core.ready().await {
                this.report(format!("Recovery executor unavailable: {error}"));
                return;
            }
            match this.store.load_document(accepted.document.id.clone()).await {
                Ok(Some(document))
                    if this
                        .model()
                        .accepted
                        .as_ref()
                        .is_some_and(|s| s.token == accepted.token)
                        && this.model().lifecycle == Lifecycle::RecoveryRequired =>
                {
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
    async fn pack_archive(&self, document: &ProjectDoc) -> Result<Vec<u8>, String> {
        let mut buffers = vec![];
        let mut entries = vec![];
        for asset in &document.assets {
            let bytes = self
                .store
                .load_asset(asset.sha256.clone())
                .await
                .map_err(|e| e.to_string())?
                .ok_or_else(|| format!("Missing asset: {}", asset.name))?;
            entries.push(serde_json::json!({"path": format!("assets/{}", asset.sha256), "bufferIndex": buffers.len()}));
            buffers.push(bytes);
        }
        let metadata = serde_json::json!({"kind":"pack-project", "projectJson":serde_json::to_string(document).map_err(|e| e.to_string())?, "assets":entries}).to_string();
        let operation = self.operation();
        let core = self.core.borrow().clone();
        let result = core
            .archive(&format!("pack-{}", operation.0), "1", &metadata, buffers)
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
impl Drop for Runtime {
    fn drop(&mut self) {
        self.cancel_frames();
        if let Some((_, worker)) = self.cad_worker.borrow_mut().take() {
            worker.close();
        }
        for (_, worker) in std::mem::take(&mut *self.export_workers.borrow_mut()) {
            worker.close();
        }
        self.core.borrow().close();
    }
}

pub fn deployment_prefix() -> Result<&'static str, String> {
    let path = web_sys::window()
        .ok_or("window unavailable")?
        .location()
        .pathname()
        .map_err(|e| format!("{e:?}"))?;
    Ok(
        if path == "/boardstudio" || path.starts_with("/boardstudio/") {
            "/boardstudio/"
        } else {
            "/"
        },
    )
}
pub fn resource_url(path: &str) -> Result<String, String> {
    let origin = web_sys::window()
        .ok_or("window unavailable")?
        .location()
        .origin()
        .map_err(|e| format!("{e:?}"))?;
    Ok(format!("{origin}{}{path}", deployment_prefix()?))
}
async fn fetch_bytes(path: &str) -> Result<Vec<u8>, String> {
    let window = web_sys::window().ok_or("window unavailable")?;
    let response = JsFuture::from(window.fetch_with_str(&resource_url(path)?))
        .await
        .map_err(|e| format!("{e:?}"))?
        .dyn_into::<web_sys::Response>()
        .map_err(|e| format!("{e:?}"))?;
    if !response.ok() {
        return Err(format!(
            "Required asset unavailable: {path} ({})",
            response.status()
        ));
    }
    let buffer = JsFuture::from(response.array_buffer().map_err(|e| format!("{e:?}"))?)
        .await
        .map_err(|e| format!("{e:?}"))?;
    Ok(Uint8Array::new(&buffer).to_vec())
}
fn deliver(bytes: &[u8], filename: &str) -> Result<(), String> {
    let parts = Array::new();
    parts.push(&Uint8Array::from(bytes));
    let blob = Blob::new_with_u8_array_sequence(&parts).map_err(|e| format!("{e:?}"))?;
    let url = Url::create_object_url_with_blob(&blob).map_err(|e| format!("{e:?}"))?;
    let result = (|| {
        let document = web_sys::window()
            .and_then(|w| w.document())
            .ok_or("document unavailable")?;
        let anchor = document
            .create_element("a")
            .map_err(|e| format!("{e:?}"))?
            .dyn_into::<HtmlAnchorElement>()
            .map_err(|e| format!("{e:?}"))?;
        anchor.set_href(&url);
        anchor.set_download(filename);
        let body = document.body().ok_or("document body unavailable")?;
        body.append_child(&anchor).map_err(|e| format!("{e:?}"))?;
        anchor.click();
        anchor.remove();
        Ok(())
    })();
    if result.is_ok() {
        spawn_local(async move {
            gloo_timers::future::TimeoutFuture::new(1_000).await;
            let _ = Url::revoke_object_url(&url);
        });
    } else {
        let _ = Url::revoke_object_url(&url);
    }
    result
}
