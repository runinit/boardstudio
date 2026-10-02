//! Browser composition runs identified effects; the headless session remains authoritative.
use boardstudio_application::{
    AcceptedSnapshot, Completion, Effect, Event, JobId, Lifecycle, OperationId, ReadModel,
    SaveResult, Scope, Session, SnapshotToken, TerminalOutcome,
};
use boardstudio_core::{
    electrical::{ElectricalMode, ElectricalPlan, ElectricalPlanRequest},
    model::{
        ArchiveReply, Board, CoreReply, CoreRequest, Material, MechanicalAssembly,
        MechanicalConfiguration, Operation, OutlineFeature, OutlineSettings, ProjectDoc,
    },
};
use boardstudio_web::host::{BrowserStore, CoreWorker};
use boardstudio_web::{
    cad_jobs::{
        CadJobError, CadOperation, CadRequest, CadResult, captured_case_scene,
        prepare_captured_case, prepare_captured_step_assembly, validate_reply,
    },
    cad_worker::CadWorker,
};

pub struct CadScene {
    pub scope: Scope,
    pub token: SnapshotToken,
    pub snapshot: AcceptedSnapshot,
    pub result: CadResult,
    pub(crate) prepared: boardstudio_core::model::PreparedCaseAssemblyIR,
    pub mechanical: Option<boardstudio_core::model::MechanicalAssembly>,
    pub exact: bool,
    pub contours: Vec<boardstudio_core::model::Contour>,
}
use js_sys::{Array, Function, Reflect, Uint8Array};
use std::{
    cell::{Cell, RefCell},
    collections::{BTreeMap, BTreeSet, VecDeque},
    rc::Rc,
};
use wasm_bindgen::{JsCast, JsValue, closure::Closure};
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

fn browser_uuid() -> Result<String, String> {
    let crypto = Reflect::get(&js_sys::global(), &JsValue::from_str("crypto"))
        .map_err(|error| format!("Could not access browser project identity service: {error:?}"))?;
    let random_uuid = Reflect::get(&crypto, &JsValue::from_str("randomUUID"))
        .map_err(|error| format!("Could not access browser project identity service: {error:?}"))?
        .dyn_into::<Function>()
        .map_err(|_| "The browser cannot generate a safe project identity.".to_string())?;
    random_uuid
        .call0(&crypto)
        .map_err(|error| format!("Could not generate a project identity: {error:?}"))?
        .as_string()
        .ok_or_else(|| "Could not generate a project identity.".to_string())
}

pub struct Runtime {
    session: RefCell<Session>,
    operation_outcomes: crate::operation_outcomes::OperationOutcomes,
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
    embed_used_models: Cell<bool>,
}
impl Runtime {
    pub fn new() -> Result<Rc<Self>, String> {
        let prefix = deployment_prefix()?;
        let runtime = Rc::new(Self {
            session: RefCell::new(Session::new()),
            operation_outcomes: crate::operation_outcomes::OperationOutcomes::default(),
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
            embed_used_models: Cell::new(true),
        });
        // Reserve the startup open identity synchronously, before any explicit
        // open action can supersede restoration of the last durable project.
        runtime.restore_active_project();
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
    fn restore_active_project(self: &Rc<Self>) {
        match self.store.active_project_id("") {
            Ok(project_id) if !project_id.is_empty() => self.open_saved(project_id),
            Ok(_) => {}
            Err(error) => self.report(format!(
                "Could not read the last active keyboard preference; choose a saved keyboard from the library. {error}"
            )),
        }
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
    pub(crate) fn electrical_preview_executor_epoch(&self) -> u64 {
        self.session.borrow().core_executor_epoch().0
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
    pub(crate) fn observe_operation(
        &self,
        operation: OperationId,
    ) -> crate::operation_outcomes::OutcomeSlot {
        self.operation_outcomes.observe(operation)
    }

    /// Open a fresh blank keyboard through the normal Session persistence path.
    pub(crate) fn create_new_keyboard(
        self: &Rc<Self>,
    ) -> Result<(String, crate::operation_outcomes::OutcomeSlot), String> {
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

        let operation_id = self.operation();
        let outcome = self.observe_operation(operation_id);
        self.submit(Event::Open {
            operation_id,
            document,
        });
        Ok((project_id, outcome))
    }

    /// Resolve mechanical settings against the exact accepted source and the proposed canonical
    /// document. The proposal carrier exists only for the existing effective-case projection;
    /// it is never installed in Session or any accepted/display authority.
    pub(crate) async fn resolve_mechanical_settings(
        &self,
        accepted: AcceptedSnapshot,
        scope: Scope,
        proposed: ProjectDoc,
    ) -> Result<(MechanicalAssembly, MechanicalConfiguration), String> {
        validate_mechanical_source(&accepted, &scope)?;
        if proposed.id != accepted.document.id
            || proposed.revision != accepted.document.revision
            || proposed.physical_instance_id != accepted.document.physical_instance_id
            || !mechanical_document_targets_scope(&proposed, &scope)
        {
            return Err(
                "The proposed mechanical settings do not match the accepted board scope.".into(),
            );
        }
        self.ensure_mechanical_source_current(&accepted, &scope)?;

        // This temporary carrier lets the established projection apply physical-instance
        // defaults (including wireless battery defaults) without granting it Session authority.
        let proposal_projection_input = AcceptedSnapshot {
            token: accepted.token,
            session_epoch: accepted.session_epoch,
            document: std::sync::Arc::new(proposed),
            scene: accepted.scene.clone(),
        };
        let effective_document =
            boardstudio_web::cad_jobs::captured_case_document(&proposal_projection_input, &scope)
                .map_err(|error| {
                format!("Could not project proposed mechanical settings: {error:?}")
            })?;
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
        let contours = boardstudio_web::cad_jobs::captured_case_scene(&accepted, &scope)
            .map_err(|error| format!("Could not project accepted case contours: {error:?}"))?
            .board_contours
            .into_iter()
            .find(|board| board.board_id == scope.board_id)
            .map_or_else(Vec::new, |board| board.contours);

        let core = self.core.borrow().clone();
        let executor_epoch = self.session.borrow().core_executor_epoch().0;
        self.ensure_mechanical_source_current(&accepted, &scope)?;
        let current_core = self.core.borrow().clone();
        if self.session.borrow().core_executor_epoch().0 != executor_epoch
            || !Rc::ptr_eq(&core, &current_core)
        {
            return Err("The Core worker changed before mechanical resolution started.".into());
        }

        let request_id = format!("mechanical-settings-{}", self.operation().0);
        let request = CoreRequest::ResolveMechanical {
            id: request_id.clone(),
            document: effective_document,
            contours,
        };
        let executor_epoch_string = executor_epoch.to_string();
        let reply = core
            .request(&request_id, &executor_epoch_string, &request)
            .await;

        self.ensure_mechanical_source_current(&accepted, &scope)?;
        let current_core = self.core.borrow().clone();
        if self.session.borrow().core_executor_epoch().0 != executor_epoch
            || !Rc::ptr_eq(&core, &current_core)
        {
            return Err("The Core worker changed during mechanical resolution.".into());
        }
        let reply = reply.map_err(|error| format!("Mechanical resolution failed: {error}"))?;

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

    /// Resolve the accepted PCB's read-only wiring plan through the current Core worker.
    /// The board scope deliberately has no physical instance; switch selection is not part of
    /// this plan's lifetime.
    pub(crate) async fn resolve_electrical_preview(
        &self,
        accepted: AcceptedSnapshot,
        scope: Scope,
    ) -> Result<ElectricalPlan, String> {
        validate_electrical_source(&accepted, &scope)?;
        self.ensure_electrical_source_current(&accepted, &scope)?;

        let configuration = accepted.document.hardware.as_ref().and_then(|hardware| {
            hardware
                .boards
                .iter()
                .find(|item| item.board_id == scope.board_id)
        });
        let request_id = format!("pcb-electrical-{}", self.operation().0);
        let request = CoreRequest::ResolveElectrical {
            id: request_id.clone(),
            request: ElectricalPlanRequest {
                document: (*accepted.document).clone(),
                instance_id: None,
                mode: configuration.map_or(ElectricalMode::Matrix, |item| item.mode),
                locks: configuration.map_or_else(Default::default, |item| item.locks.clone()),
                controller_profile: None,
                board_id: Some(scope.board_id.clone()),
                controller_part_id: configuration.and_then(|item| item.controller_part_id.clone()),
            },
        };
        let core = self.core.borrow().clone();
        let executor_epoch = self.session.borrow().core_executor_epoch();
        self.ensure_electrical_source_current(&accepted, &scope)?;
        if self.session.borrow().core_executor_epoch() != executor_epoch
            || !Rc::ptr_eq(&core, &self.core.borrow().clone())
        {
            return Err("The Core worker changed before wiring resolution started.".into());
        }

        let reply = core
            .request(&request_id, &executor_epoch.0.to_string(), &request)
            .await;

        // Recheck both accepted board identity and the exact worker owner before interpreting a
        // reply. An old worker's reply cannot become current after a restart/reopen ABA.
        self.ensure_electrical_source_current(&accepted, &scope)?;
        if self.session.borrow().core_executor_epoch() != executor_epoch
            || !Rc::ptr_eq(&core, &self.core.borrow().clone())
        {
            return Err("The Core worker changed during wiring resolution.".into());
        }
        let reply = reply.map_err(|error| format!("Wiring resolution failed: {error}"))?;
        let plan = match reply {
            CoreReply::ElectricalResolved { id, plan } if id == request_id => plan,
            CoreReply::Error { id, message, .. } if id == request_id => return Err(message),
            CoreReply::ElectricalResolved { .. } | CoreReply::Error { .. } => {
                return Err("Core returned a wiring reply for another request.".into());
            }
            _ => return Err("Core returned an unexpected wiring-resolution reply.".into()),
        };
        if plan.revision != accepted.document.revision
            || plan.board_id.as_deref() != Some(scope.board_id.as_str())
            || plan.instance_id.is_some()
        {
            return Err("Core resolved wiring for another board or revision.".into());
        }
        Ok(plan)
    }

    /// Resolve keycap fit against one accepted canonical board and the matching prepared Case
    /// preview, if the current physical-instance preview belongs to this exact snapshot.
    pub(crate) async fn resolve_keycaps_preview(
        &self,
        scope: Scope,
        token: SnapshotToken,
        revision: u64,
    ) -> Result<boardstudio_core::model::KeycapResolution, String> {
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
        let core = self.core.borrow().clone();
        let executor_epoch = self.session.borrow().core_executor_epoch();
        if self.session.borrow().core_executor_epoch() != executor_epoch
            || !Rc::ptr_eq(&core, &self.core.borrow().clone())
        {
            return Err("The Core worker changed before keycap fit resolution started.".into());
        }

        let request_id = format!("keycaps-fit-{}", self.operation().0);
        let request = CoreRequest::ResolveKeycaps {
            id: request_id.clone(),
            document: (*accepted.document).clone(),
            board_id: scope.board_id.clone(),
            cases,
        };
        let reply = core
            .request(&request_id, &executor_epoch.0.to_string(), &request)
            .await;

        self.ensure_keycaps_source_current(&accepted, &scope)?;
        if self.session.borrow().core_executor_epoch() != executor_epoch
            || !Rc::ptr_eq(&core, &self.core.borrow().clone())
        {
            return Err("The Core worker changed during keycap fit resolution.".into());
        }
        let current_case = self.cad_scene().filter(|scene| {
            scene.exact
                && scene.scope == scope
                && scene.token == accepted.token
                && scene.snapshot.document.revision == accepted.document.revision
                && scene.prepared.revision == accepted.document.revision
        });
        if current_case.as_ref().map(|scene| &scene.prepared)
            != case_preview.as_ref().map(|scene| &scene.prepared)
        {
            return Err("The Case preview changed during keycap fit resolution.".into());
        }

        let reply = reply.map_err(|error| format!("Keycap fit resolution failed: {error}"))?;
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

    fn ensure_keycaps_source_current(
        &self,
        accepted: &AcceptedSnapshot,
        scope: &Scope,
    ) -> Result<(), String> {
        let model = self.model();
        let Some(current) = model.accepted.as_ref() else {
            return Err("The accepted Keycaps source is no longer open.".into());
        };
        if self.scope().as_ref() != Some(scope)
            || model.active_board_id.as_str() != scope.board_id.as_str()
            || scope.session_epoch != accepted.session_epoch
            || scope.document_id != accepted.document.id
            || current.session_epoch != accepted.session_epoch
            || current.token != accepted.token
            || current.document.id != accepted.document.id
            || current.document.revision != accepted.document.revision
            || current.scene.revision != accepted.scene.revision
            || accepted.scene.revision != accepted.document.revision
        {
            return Err("The accepted Keycaps source changed during resolution.".into());
        }
        Ok(())
    }

    fn ensure_electrical_source_current(
        &self,
        accepted: &AcceptedSnapshot,
        scope: &Scope,
    ) -> Result<(), String> {
        let model = self.model();
        let Some(current) = model.accepted.as_ref() else {
            return Err("The accepted PCB wiring source is no longer open.".into());
        };
        let current_scope = self.scope();
        let same_board_scope = current_scope.as_ref().is_some_and(|current| {
            current.session_epoch == scope.session_epoch
                && current.document_id == scope.document_id
                && current.board_id == scope.board_id
        });
        if !same_board_scope
            || model.active_board_id != scope.board_id
            || scope.instance_id.is_some()
            || scope.session_epoch != accepted.session_epoch
            || scope.document_id != accepted.document.id
            || current.session_epoch != accepted.session_epoch
            || current.token != accepted.token
            || current.document.id != accepted.document.id
            || current.document.revision != accepted.document.revision
            || current.scene.revision != accepted.scene.revision
        {
            return Err("The accepted PCB wiring source changed during resolution.".into());
        }
        Ok(())
    }

    fn ensure_mechanical_source_current(
        &self,
        accepted: &AcceptedSnapshot,
        scope: &Scope,
    ) -> Result<(), String> {
        let model = self.model();
        let Some(current) = model.accepted.as_ref() else {
            return Err("The accepted mechanical source is no longer open.".into());
        };
        if self.scope().as_ref() != Some(scope)
            || scope.session_epoch != accepted.session_epoch
            || scope.document_id != accepted.document.id
            || current.session_epoch != accepted.session_epoch
            || current.token != accepted.token
            || current.document.id != accepted.document.id
            || current.document.revision != accepted.document.revision
            || current.scene.revision != accepted.scene.revision
            || accepted.scene.revision != accepted.document.revision
        {
            return Err("The accepted mechanical source changed during resolution.".into());
        }
        Ok(())
    }

    pub fn submit(self: &Rc<Self>, event: Event) {
        if matches!(&event, Event::StartGeneration { .. })
            && self.mechanical_mount_initialization_pending()
        {
            self.report(
                "Finish preparing mounting locations before generating the mechanical assembly.",
            );
            return;
        }
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
                let observed = self
                    .operation_outcomes
                    .settle(operation_id, outcome.clone());
                self.step_exports.borrow_mut().remove(&operation_id);
                match outcome {
                    TerminalOutcome::Completed => self.report("Saved locally."),
                    TerminalOutcome::Rejected(reason)
                    | TerminalOutcome::PersistenceFailed(reason)
                    | TerminalOutcome::BlockedByRecovery(reason)
                    | TerminalOutcome::ExecutorFailed(reason) => self.report(reason),
                    TerminalOutcome::Cancelled => self.report("Cancelled."),
                    TerminalOutcome::Closed => self.report("Editor closed."),
                    TerminalOutcome::Superseded => {
                        if observed {
                            self.changed();
                        }
                    }
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
                self.pack_archive(
                    operation_id,
                    &snapshot,
                    &scope,
                    self.embed_used_models.get(),
                )
                .await
            } {
                Ok(bytes) => {
                    let current = self.export_current(operation_id, snapshot.token, &scope);
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
                if let Some((_, worker)) = self.cad_worker.borrow().as_ref() {
                    // Retain valid same-scope cache updates; the worker yields between bodies.
                    // Scope replacement and close still terminate the departing worker.
                    let _ = worker.cancel(&format!("case-{}", job_id.0));
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
    /// An existing resolved stack with suggested mounts must finish its saved initialization
    /// before a dependent generation or STEP capture. Archive saving remains independent.
    fn mechanical_mount_initialization_pending(&self) -> bool {
        let model = self.model();
        let Some(accepted) = model.accepted.as_ref() else {
            return false;
        };
        let Some(scope) = self.scope() else {
            return false;
        };
        let scene = self.cad_scene();
        if !scene.as_ref().is_some_and(|scene| {
            scene.scope == scope
                && scene.token == accepted.token
                && scene.exact
                && scene
                    .mechanical
                    .as_ref()
                    .is_some_and(|assembly| !assembly.suggested_mounts.is_empty())
        }) {
            return false;
        }
        boardstudio_web::cad_jobs::captured_case_document(accepted, &scope).is_ok_and(|document| {
            document.mechanical.as_ref().is_some_and(|configuration| {
                configuration.board_id == scope.board_id
                    && configuration.closure_mounts.is_none()
                    && configuration.mount != boardstudio_core::model::MechanicalMount::Gasket
            })
        })
    }

    pub fn export_step(self: &Rc<Self>) {
        if self.mechanical_mount_initialization_pending() {
            self.report(
                "Finish preparing mounting locations before exporting the mechanical assembly.",
            );
            return;
        }
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
    fn export_current(
        &self,
        operation_id: OperationId,
        token: SnapshotToken,
        scope: &Scope,
    ) -> bool {
        self.session
            .borrow()
            .export_is_current(operation_id, token, scope)
            && !self.cancelled_exports.borrow().contains(&operation_id)
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
        let current_job = matches!(self.model().generation,
            boardstudio_application::GenerationStatus::Preparing { job_id: active }
            | boardstudio_application::GenerationStatus::Running { job_id: active } if active == job_id);
        if result.is_err()
            && current_job
            && let Some((_, worker)) = self.cad_worker.borrow_mut().take()
        {
            worker.close();
        }
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
            let current_job = matches!(self.model().generation,
                boardstudio_application::GenerationStatus::Preparing { job_id: active }
                | boardstudio_application::GenerationStatus::Running { job_id: active } if active == job_id);
            if cancelled.get() || !current_job || !self.snapshot_current(snapshot.token, scope) {
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
            Some((worker_scope, worker)) if worker_scope == *scope && !worker.is_closed() => worker,
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
        let contours = captured_case_scene(snapshot, scope)?
            .board_contours
            .into_iter()
            .find(|board| board.board_id == scope.board_id)
            .ok_or_else(|| CadJobError::Blocked("Case contours unavailable".into()))?
            .contours;
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
            let mut result = validate_reply(&request, reply, &prepared.identity)?;
            // Renderer retention needs meshes only; manufacturing exports have their own worker.
            result.step = Vec::new();
            *self.cad_scene.borrow_mut() = Some(Rc::new(CadScene {
                scope: scope.clone(),
                token: snapshot.token,
                snapshot: snapshot.clone(),
                result,
                prepared: prepared.prepared.clone(),
                mechanical: prepared.mechanical_assembly.clone(),
                exact: operation == CadOperation::Exact,
                contours: contours.clone(),
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
            if !self.export_current(operation_id, snapshot.token, scope) {
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
                Ok(Some(_)) => {}
                Ok(None) if this.open_sequence.get() == sequence => this.report(
                    "The saved keyboard is unavailable. Choose an available copy from the library.",
                ),
                Err(error) if this.open_sequence.get() == sequence => {
                    this.report(format!("Could not open the saved keyboard: {error}"))
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
    async fn pack_archive(
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
        guard()?;
        let document = snapshot.document.as_ref();
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
                .store
                .load_asset(asset.sha256.clone())
                .await
                .map_err(|e| e.to_string())?
                .ok_or_else(|| format!("Missing asset: {}", asset.name))?;
            guard()?;
            local_asset_bytes.insert(asset.sha256.clone(), bytes.to_vec());
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

fn validate_mechanical_source(accepted: &AcceptedSnapshot, scope: &Scope) -> Result<(), String> {
    if scope.session_epoch != accepted.session_epoch
        || scope.document_id != accepted.document.id
        || accepted.scene.revision != accepted.document.revision
    {
        return Err("The accepted mechanical source does not match its scope or revision.".into());
    }
    if !mechanical_document_targets_scope(&accepted.document, scope) {
        return Err(
            "The accepted mechanical source does not contain the selected board scope.".into(),
        );
    }
    Ok(())
}

fn validate_electrical_source(accepted: &AcceptedSnapshot, scope: &Scope) -> Result<(), String> {
    if scope.instance_id.is_some()
        || scope.session_epoch != accepted.session_epoch
        || scope.document_id != accepted.document.id
        || accepted.scene.revision != accepted.document.revision
    {
        return Err(
            "The accepted wiring source does not match its board scope or revision.".into(),
        );
    }
    if !accepted
        .document
        .boards
        .iter()
        .any(|board| board.id == scope.board_id)
    {
        return Err("The selected board is not present in the accepted wiring source.".into());
    }
    Ok(())
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
