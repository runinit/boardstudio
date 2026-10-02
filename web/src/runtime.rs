//! Browser composition runs identified effects; the headless session remains authoritative.
use crate::archive_export::{ArchiveExportOptions, ArchiveWorkFuture, archive_filename};
use boardstudio_application::{
    AcceptedSnapshot, Completion, Effect, Event, JobId, Lifecycle, OperationId, ReadModel,
    SaveResult, Scope, Session, SnapshotToken, TerminalOutcome,
};
use boardstudio_core::{
    electrical::{ElectricalMode, ElectricalPlan, ElectricalPlanRequest},
    model::{
        ArchiveEntry, ArchiveReply, ArchiveRequest, ArtifactReply, Board, CoreReply, CoreRequest,
        ErgogenJobResult, FinishExportRequest, HardwareTopology, Material, MechanicalAssembly,
        MechanicalConfiguration, Operation, OutlineFeature, OutlineSettings, ProjectDoc,
    },
};
use boardstudio_web::host::{BrowserStore, CoreWorker};
use boardstudio_web::{
    cad_jobs::{
        CadJobError, CadOperation, CadRequest, CadResult, CadSnapshotIdentity, captured_case_scene,
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
use web_sys::{Blob, BlobPropertyBag, HtmlAnchorElement, SvgElement, Url};

type Notifier = Rc<dyn Fn()>;
type Frame = (i32, Closure<dyn FnMut(f64)>);
struct Artifact {
    bytes: Vec<u8>,
    filename: String,
    media_type: Option<String>,
    scope: Scope,
    token: SnapshotToken,
}

#[derive(Clone, Copy)]
struct FirmwarePlanTarget<'a> {
    board_id: &'a str,
    instance_id: Option<&'a str>,
    label: &'a str,
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
    firmware_exports: RefCell<BTreeSet<OperationId>>,
    export_workers: RefCell<BTreeMap<OperationId, Rc<CadWorker>>>,
    native_case_preview: RefCell<crate::case_preview::NativePreviewState>,
    case_model_delivery: crate::presentation::model_delivery::ModelDeliveryAdapter,
    native_model_delivery: RefCell<NativeModelDeliveryState>,
    native_model_jobs: RefCell<BTreeSet<String>>,
    preview_generator: RefCell<Option<Rc<crate::preview_generator::PreviewGeneratorClient>>>,
    archive_export_options: ArchiveExportOptions,
    #[cfg(test)]
    definition_name_test_state: RefCell<Option<(AcceptedSnapshot, Option<Scope>)>>,
    #[cfg(test)]
    definition_name_test_events: RefCell<Vec<Event>>,
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
            firmware_exports: RefCell::new(BTreeSet::new()),
            export_workers: RefCell::new(BTreeMap::new()),
            native_case_preview: RefCell::new(Default::default()),
            case_model_delivery: Default::default(),
            native_model_delivery: RefCell::new(Default::default()),
            native_model_jobs: RefCell::new(BTreeSet::new()),
            preview_generator: RefCell::new(None),
            archive_export_options: ArchiveExportOptions::default(),
            #[cfg(test)]
            definition_name_test_state: RefCell::new(None),
            #[cfg(test)]
            definition_name_test_events: RefCell::new(Vec::new()),
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
        #[cfg(test)]
        if let Some((_, scope)) = self.definition_name_test_state.borrow().as_ref() {
            return scope.clone();
        }
        self.session.borrow().scope()
    }
    pub(crate) fn electrical_preview_executor_epoch(&self) -> u64 {
        self.session.borrow().core_executor_epoch().0
    }
    pub fn model(&self) -> ReadModel {
        #[cfg(test)]
        if let Some((snapshot, _)) = self.definition_name_test_state.borrow().as_ref() {
            return ReadModel {
                accepted: Some(snapshot.clone()),
                ..ReadModel::default()
            };
        }
        self.session.borrow().read_model().clone()
    }
    pub fn status(&self) -> String {
        self.status.borrow().clone()
    }
    pub(crate) fn embed_used_models(&self) -> bool {
        self.archive_export_options.embed_used_models()
    }
    pub(crate) fn set_embed_used_models(&self, value: bool) {
        if self.archive_export_options.set_embed_used_models(value) {
            self.changed();
        }
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
    fn invalidate_stale_native_case_preview(&self) {
        let stale = self
            .native_case_preview
            .borrow()
            .is_stale(|owner| self.preview_owner_is_current(owner));
        if stale {
            self.cancel_native_case_preview();
        }
    }

    pub(crate) fn cancel_native_case_preview(&self) {
        self.native_case_preview.borrow_mut().cancel();
        self.cancel_native_model_jobs();
    }

    fn cancel_native_model_jobs(&self) {
        let jobs = std::mem::take(&mut *self.native_model_jobs.borrow_mut());
        if let Some((_, worker)) = self.cad_worker.borrow().as_ref() {
            for job in jobs {
                let _ = worker.cancel(&job);
            }
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
        #[cfg(test)]
        if self.definition_name_test_state.borrow().is_some() {
            self.definition_name_test_events.borrow_mut().push(event);
            return;
        }
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
        self.invalidate_stale_native_case_preview();
        if self.scope() != previous_scope {
            self.cad_scene.borrow_mut().take();
            if let Some((_, worker)) = self.cad_worker.borrow_mut().take() {
                worker.close();
            }
        }
        self.changed();
        self.drive(effects);
    }

    #[cfg(test)]
    pub(crate) fn set_definition_name_test_state(
        &self,
        snapshot: AcceptedSnapshot,
        scope: Option<Scope>,
    ) {
        *self.definition_name_test_state.borrow_mut() = Some((snapshot, scope));
    }

    #[cfg(test)]
    pub(crate) fn take_definition_name_test_event(&self) -> Option<Event> {
        let mut events = self.definition_name_test_events.borrow_mut();
        if events.is_empty() {
            None
        } else {
            Some(events.remove(0))
        }
    }
    fn complete(self: &Rc<Self>, event: Completion) -> Vec<Effect> {
        let effects = self.session.borrow_mut().complete(event);
        self.invalidate_stale_native_case_preview();
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
                self.firmware_exports.borrow_mut().remove(&operation_id);
                self.archive_export_options.settle(operation_id);
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
            } => {
                let is_step_export = self.step_exports.borrow().contains(&operation_id);
                let is_firmware_export = self.firmware_exports.borrow_mut().remove(&operation_id);
                let work: Result<ArchiveWorkFuture<'_>, String> =
                    self.archive_export_options.dispatch(
                        operation_id,
                        is_step_export,
                        is_firmware_export,
                        || -> ArchiveWorkFuture<'_> {
                            Box::pin(self.step_bytes(operation_id, &snapshot, &scope))
                        },
                        |embed_used_models| -> ArchiveWorkFuture<'_> {
                            Box::pin(self.pack_archive(
                                operation_id,
                                &snapshot,
                                &scope,
                                embed_used_models,
                            ))
                        },
                        || -> ArchiveWorkFuture<'_> {
                            Box::pin(self.firmware_bytes(operation_id, &snapshot, &scope))
                        },
                    );
                let result = match work {
                    Ok(future) => future.await,
                    Err(reason) => Err(reason),
                };
                match result {
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
                        let (filename, media_type) = if is_firmware_export {
                            (
                                format!("{}-zmk.zip", snapshot.document.name),
                                Some("application/zip".to_owned()),
                            )
                        } else if is_step_export {
                            ("keyboard.step".to_owned(), None)
                        } else {
                            (archive_filename(&snapshot.document.name), None)
                        };
                        self.artifacts.borrow_mut().insert(
                            artifact_id.clone(),
                            Artifact {
                                bytes,
                                filename,
                                media_type,
                                scope: scope.clone(),
                                token: snapshot.token,
                            },
                        );
                        self.complete(Completion::ExportFinished {
                            operation_id,
                            token: snapshot.token,
                            scope: scope.clone(),
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
                }
            }
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
                    && let Err(error) = deliver(
                        &artifact.bytes,
                        &artifact.filename,
                        artifact.media_type.as_deref(),
                    )
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
                self.firmware_exports.borrow_mut().remove(&operation_id);
                self.archive_export_options.cancel(operation_id);
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

    pub(crate) fn native_case_preview(
        &self,
    ) -> Option<Rc<crate::case_preview::NativePreviewSnapshot>> {
        let current_scope = self.scope()?;
        let accepted = self.model().accepted?;
        self.native_case_preview
            .borrow()
            .published
            .as_ref()
            .filter(|preview| {
                preview.lease.matches(&preview.owner)
                    && preview.owner.scope == current_scope
                    && preview.owner.snapshot_token == accepted.token
                    && preview.owner.accepted_revision == accepted.document.revision
            })
            .cloned()
    }

    pub(crate) fn native_case_preview_pending(&self) -> bool {
        self.native_case_preview
            .borrow()
            .pending
            .as_ref()
            .is_some_and(|(owner, lease)| {
                lease.matches(owner) && self.preview_owner_is_current(owner)
            })
    }

    pub(crate) fn native_case_preview_error(&self) -> Option<String> {
        self.native_case_preview
            .borrow()
            .error
            .as_ref()
            .filter(|(owner, _)| self.preview_owner_is_current(owner))
            .map(|(_, error)| error.clone())
    }

    pub(crate) fn native_case_preview_key(&self) -> Option<(Scope, SnapshotToken, u64)> {
        let scope = self.scope()?;
        let accepted = self.model().accepted?;
        (scope.session_epoch == accepted.session_epoch
            && scope.document_id == accepted.document.id
            && accepted.scene.revision == accepted.document.revision)
            .then_some((scope, accepted.token, accepted.document.revision))
    }

    pub(crate) fn native_model_delivery(
        &self,
        preview: &crate::case_preview::NativePreviewSnapshot,
    ) -> Option<crate::presentation::model_delivery::ModelDeliveryRows> {
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
    pub(crate) async fn deliver_native_case_models(
        self: &Rc<Self>,
        preview: Rc<crate::case_preview::NativePreviewSnapshot>,
    ) -> Result<(), String> {
        if !self.native_preview_snapshot_is_current(&preview) {
            return Ok(());
        }
        let owner = crate::presentation::model_delivery::ModelOwnerIdentity::new(
            preview.owner.scope.clone(),
            preview.owner.snapshot_token,
            preview.owner.viewer_instance,
            preview.owner.projection_generation,
            &preview.lease,
        );
        let batch = crate::presentation::model_delivery::ModelBatchIdentity::new(
            owner.clone(),
            preview.owner.accepted_revision,
            preview.owner.batch_generation,
        );
        let native_paths =
            crate::presentation::model_delivery::native_model_path_assets(&preview.path_assets);
        let unique_model_paths = preview
            .preview
            .models
            .iter()
            .map(|model| model.path.clone())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();
        let Some(ergogen_ids_by_path) = resolve_native_model_paths(
            &self.native_model_delivery,
            &preview.owner,
            unique_model_paths,
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
        let selections = crate::presentation::model_delivery::resolve_preview_assets(
            &preview.preview.models,
            None,
            &native_paths,
            &preview.accepted_document,
            |path| ergogen_ids_by_path.get(path).cloned().flatten(),
            |asset_id| {
                crate::bundled_models::bundled_model(asset_id).map(|model| {
                    crate::presentation::model_delivery::ResolvedModelAsset {
                        id: model.id.to_owned(),
                        sha256: model.sha256.to_owned(),
                        filename: model.filename.to_owned(),
                        source: crate::presentation::model_delivery::ModelAssetSource::Packaged {
                            url_path: model.url_path.to_owned(),
                        },
                    }
                })
            },
        )
        .into_iter()
        .collect::<BTreeMap<_, _>>();
        let source_is_current = {
            let weak = Rc::downgrade(self);
            let preview = preview.clone();
            Rc::new(move || {
                weak.upgrade()
                    .is_some_and(|runtime| runtime.native_preview_snapshot_is_current(&preview))
            }) as Rc<dyn Fn() -> bool>
        };
        let ports = self.native_model_delivery_ports(&preview, source_is_current.clone());
        let results = self
            .case_model_delivery
            .deliver_models(
                preview.preview.revision,
                &preview.preview.models,
                &selections,
                &ports,
                &batch,
                source_is_current,
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

    fn native_preview_snapshot_is_current(
        &self,
        expected: &crate::case_preview::NativePreviewSnapshot,
    ) -> bool {
        self.native_case_preview().is_some_and(|current| {
            current.owner == expected.owner && Rc::ptr_eq(&current.lease, &expected.lease)
        })
    }

    async fn read_case_step_model(
        &self,
        bytes: Vec<u8>,
        scope: Scope,
        token: SnapshotToken,
        revision: u64,
        is_current: Rc<dyn Fn() -> bool>,
    ) -> Result<crate::presentation::model_delivery::MeshArrays, String> {
        if !is_current() || self.scope().as_ref() != Some(&scope) {
            return Err("Case STEP model request became stale before worker setup".into());
        }
        let worker = crate::case_model_lifecycle::case_model_worker(
            &self.cad_worker,
            &scope,
            |worker| !worker.is_closed(),
            || {
                CadWorker::new(&resource_url("assets/cad-worker/entry.js")?)
                    .map(Rc::new)
                    .map_err(|error| error.to_string())
            },
        )?;
        worker.ready().await.map_err(|error| error.to_string())?;
        if !is_current() || self.scope().as_ref() != Some(&scope) {
            return Err("Case STEP model request became stale before dispatch".into());
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
        let request = CadRequest {
            request_id: format!("case-model-step-{revision}-{operation}"),
            job_id: format!("case-model-step-{revision}-{operation}"),
            identity: identity.clone(),
            operation: CadOperation::ReadStep,
            prepared: None,
            input_bytes: bytes,
        };
        self.native_model_jobs
            .borrow_mut()
            .insert(request.job_id.clone());
        let reply = worker.request(request.clone()).await;
        self.native_model_jobs.borrow_mut().remove(&request.job_id);
        let reply = reply.map_err(|error| error.to_string())?;
        if !is_current() || self.scope().as_ref() != Some(&scope) {
            return Err("Case STEP model request became stale after parsing".into());
        }
        let result =
            validate_reply(&request, reply, &identity).map_err(|error| format!("{error:?}"))?;
        let mesh = result
            .mesh
            .ok_or_else(|| "STEP model reader returned no mesh".to_owned())?;
        Ok(crate::presentation::model_delivery::MeshArrays {
            positions: mesh.positions,
            normals: mesh.normals,
            colors: None,
        })
    }

    fn native_model_delivery_ports(
        self: &Rc<Self>,
        preview: &crate::case_preview::NativePreviewSnapshot,
        is_current: Rc<dyn Fn() -> bool>,
    ) -> crate::presentation::model_delivery::ModelDeliveryPorts {
        use crate::presentation::model_delivery::{
            MeshArrays, ModelAssetSource, ModelDeliveryPorts, ModelFuture, ResolvedModelAsset,
            VerifiedModelBytes,
        };
        let scope = preview.owner.scope.clone();
        let token = preview.owner.snapshot_token;
        let revision = preview.owner.accepted_revision;
        let viewer_instance = preview.owner.viewer_instance;
        let projection_generation = preview.owner.projection_generation;
        let lease = preview.lease.clone();
        let weak = Rc::downgrade(self);
        let scope_for_load = scope.clone();
        let lease_for_load = lease.clone();
        let current_for_load = is_current.clone();
        let load_verified_bytes = Rc::new(
            move |asset: ResolvedModelAsset| -> ModelFuture<Option<VerifiedModelBytes>> {
                let weak = weak.clone();
                let scope = scope_for_load.clone();
                let lease = lease_for_load.clone();
                let is_current = current_for_load.clone();
                Box::pin(async move {
                    let sha256 = asset.sha256.clone();
                    let runtime = weak
                        .upgrade()
                        .ok_or_else(|| "Case runtime was closed".to_owned())?;
                    if !is_current() || !lease.is_active() {
                        return Err("Case model asset request became stale before loading".into());
                    }
                    let cached = runtime.assets.borrow().get(&sha256).cloned();
                    let (bytes, cache_packaged) = match (asset.source.clone(), cached) {
                        (ModelAssetSource::Document, Some(bytes)) => (Some(bytes.to_vec()), false),
                        (ModelAssetSource::Document, None) => (
                            runtime
                                .store
                                .load_asset(sha256.clone())
                                .await
                                .map_err(|error| error.to_string())?
                                .map(|bytes| bytes.to_vec()),
                            false,
                        ),
                        (ModelAssetSource::Packaged { .. }, Some(bytes)) => {
                            (Some(bytes.to_vec()), false)
                        }
                        (ModelAssetSource::Packaged { url_path }, None) => {
                            let descriptor_matches = crate::bundled_models::bundled_model(
                                &asset.id,
                            )
                            .is_some_and(|model| {
                                model.url_path == url_path && model.sha256 == asset.sha256
                            });
                            if !descriptor_matches {
                                return Err(
                                    "Packaged model descriptor no longer matches the catalogue"
                                        .into(),
                                );
                            }
                            (
                                Some(crate::bundled_models::bundled_model_bytes(&asset.id).await?),
                                true,
                            )
                        }
                    };
                    if !is_current()
                        || !lease.is_active()
                        || runtime.scope().as_ref() != Some(&scope)
                    {
                        return Err("Case model asset request became stale after loading".into());
                    }
                    let Some(bytes) = bytes else {
                        return Ok(None);
                    };
                    let verified = VerifiedModelBytes::verify(bytes.clone(), &sha256)?;
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
                  owner: crate::presentation::model_delivery::ModelOwnerIdentity|
                  -> ModelFuture<MeshArrays> {
                let weak = weak.clone();
                let scope = scope.clone();
                let lease = lease.clone();
                let is_current = is_current.clone();
                Box::pin(async move {
                    if !owner.is_current_owner(
                        &scope,
                        token,
                        viewer_instance,
                        projection_generation,
                        &lease,
                    ) || !is_current()
                    {
                        return Err("Case STEP model request became stale before reading".into());
                    }
                    let runtime = weak
                        .upgrade()
                        .ok_or_else(|| "Case runtime was closed".to_owned())?;
                    runtime
                        .read_case_step_model(
                            bytes.bytes().to_vec(),
                            scope,
                            token,
                            revision,
                            is_current,
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

    pub(crate) async fn prepare_native_case_preview(
        self: &Rc<Self>,
        expected_scope: Scope,
        expected_token: SnapshotToken,
        expected_revision: u64,
    ) -> Result<(), String> {
        let accepted = self
            .model()
            .accepted
            .ok_or_else(|| "No accepted project is available for Case preview".to_owned())?;
        if accepted.token != expected_token
            || accepted.document.revision != expected_revision
            || self.scope().as_ref() != Some(&expected_scope)
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
            .borrow()
            .generation
            .checked_add(1)
            .ok_or_else(|| "Case preview generation identity exhausted".to_owned())?;
        self.native_case_preview.borrow_mut().generation = generation;
        let operation = self.operation().0;
        if operation == 0 || operation > 9_007_199_254_740_991 {
            return Err("Case preview request identity is outside the safe integer range".into());
        }
        let request_token = format!(
            "case-preview-{}-{}-{}",
            expected_token.0, expected_revision, operation
        );
        let core = self.core.borrow().clone();
        let core_executor_epoch = self.session.borrow().core_executor_epoch().0;
        let core_worker_identity = Rc::as_ptr(&core) as usize;
        let capture = match crate::case_preview::capture_native_preview(
            &accepted,
            &expected_scope,
            generation,
            generation,
            core_executor_epoch,
            core_worker_identity,
            request_token,
        ) {
            Ok(capture) => capture,
            Err(error) => {
                return Err(error);
            }
        };
        self.set_native_preview_pending(capture.owner.clone(), capture.lease.clone());
        let result = self
            .run_native_case_preview(&accepted, capture.clone(), operation)
            .await;
        match result {
            Ok(preview) if self.preview_owner_is_current(&preview.owner) => {
                self.native_case_preview.borrow_mut().publish(preview)?;
                self.changed();
                Ok(())
            }
            Ok(preview) => {
                preview.lease.invalidate();
                Err("The Case preview result became stale before publication".into())
            }
            Err(error) => {
                let owner_is_current = self.preview_owner_is_current(&capture.owner);
                capture.lease.invalidate();
                if owner_is_current {
                    if let Some((_, lease)) = self.native_case_preview.borrow_mut().pending.take() {
                        lease.invalidate();
                    }
                    self.native_case_preview.borrow_mut().error =
                        Some((capture.owner.clone(), error.clone()));
                    self.report(format!("Case board preview failed: {error}"));
                }
                Err(error)
            }
        }
    }

    fn set_native_preview_pending(
        &self,
        owner: crate::case_preview::CasePreviewOwnerIdentity,
        lease: Rc<crate::case_preview::CasePreviewOwnerLease>,
    ) {
        self.cancel_native_model_jobs();
        self.native_case_preview.borrow_mut().begin(owner, lease);
        self.changed();
    }

    fn preview_owner_is_current(
        &self,
        owner: &crate::case_preview::CasePreviewOwnerIdentity,
    ) -> bool {
        let model = self.model();
        let core = self.core.borrow().clone();
        self.scope().as_ref() == Some(&owner.scope)
            && self
                .native_case_preview
                .borrow()
                .published
                .as_ref()
                .is_none_or(|preview| preview.lease.matches(owner))
            && model.accepted.as_ref().is_some_and(|accepted| {
                accepted.token == owner.snapshot_token
                    && accepted.document.revision == owner.accepted_revision
                    && accepted.document.id == owner.scope.document_id
                    && accepted.session_epoch == owner.scope.session_epoch
                    && accepted.scene.revision == owner.accepted_revision
                    && std::sync::Arc::as_ptr(&accepted.scene) as usize
                        == owner.accepted_scene_identity
            })
            && self.native_case_preview.borrow().generation == owner.projection_generation
            && crate::case_preview::same_core_executor(
                owner.core_executor_epoch,
                self.session.borrow().core_executor_epoch().0,
                owner.core_worker_identity,
                Rc::as_ptr(&core) as usize,
            )
    }

    fn ensure_preview_owner_current(
        &self,
        accepted: &AcceptedSnapshot,
        owner: &crate::case_preview::CasePreviewOwnerIdentity,
        core: &Rc<CoreWorker>,
        core_epoch: u64,
    ) -> Result<(), String> {
        if !self.preview_owner_is_current(owner) {
            return Err("The accepted Case preview source changed during preparation".into());
        }
        if !self.preview_owner_lease_is_current(owner) {
            return Err("The Case preview owner lease was cancelled during preparation".into());
        }
        if accepted.token != owner.snapshot_token
            || accepted.document.revision != owner.accepted_revision
            || self.session.borrow().core_executor_epoch().0 != core_epoch
            || !Rc::ptr_eq(core, &self.core.borrow().clone())
        {
            return Err("The Core worker or accepted Case preview source changed".into());
        }
        Ok(())
    }

    fn preview_owner_lease_is_current(
        &self,
        owner: &crate::case_preview::CasePreviewOwnerIdentity,
    ) -> bool {
        self.native_case_preview.borrow().owns(owner)
    }

    async fn run_native_case_preview(
        &self,
        accepted: &AcceptedSnapshot,
        capture: crate::case_preview::NativePreviewCapture,
        operation: u64,
    ) -> Result<crate::case_preview::NativePreviewSnapshot, String> {
        let core = self.core.borrow().clone();
        let core_epoch = self.session.borrow().core_executor_epoch().0;
        self.ensure_preview_owner_current(accepted, &capture.owner, &core, core_epoch)?;
        let epoch = core_epoch.to_string();
        let prepare_id = format!("case-preview-{operation}-prepare");
        let prepare = crate::case_preview::prepare_artifact(prepare_id.clone(), &capture);
        let reply = core
            .artifact(&prepare_id, &epoch, &prepare)
            .await
            .map_err(|error| format!("Core preview preparation failed: {error}"))?;
        self.ensure_preview_owner_current(accepted, &capture.owner, &core, core_epoch)?;
        let plan = match reply {
            ArtifactReply::PreparePreview { id, result } if id == prepare_id => *result,
            ArtifactReply::Error { id, error } if id == prepare_id => {
                return Err(format!("Core rejected Case preview preparation: {error:?}"));
            }
            ArtifactReply::PreparePreview { .. } | ArtifactReply::Error { .. } => {
                return Err("Core returned a preview plan for another request".into());
            }
            _ => return Err("Core returned an unexpected preview preparation reply".into()),
        };
        if plan.snapshot_token != capture.owner.request_token
            || plan.revision != capture.owner.accepted_revision
            || plan.target
                != (boardstudio_core::model::ExportTarget::Board {
                    board_id: capture.owner.scope.board_id.clone(),
                })
        {
            return Err(
                "Core preview plan identity does not match the captured Case source".into(),
            );
        }

        let worker_request_id = operation;
        let worker_generation = capture.owner.projection_generation;
        if worker_generation == 0 || worker_generation > 9_007_199_254_740_991 {
            return Err("Case preview worker generation is outside the safe integer range".into());
        }
        let worker_request = serde_json::json!({
            "kind": "generate-preview-jobs",
            "worker_generation": worker_generation,
            "request_id": worker_request_id,
            "owner": {
                "scope": {
                    "sessionEpoch": capture.owner.scope.session_epoch.0,
                    "documentId": capture.owner.scope.document_id,
                    "boardId": capture.owner.scope.board_id,
                    "instanceId": capture.owner.scope.instance_id,
                },
                "token": capture.owner.request_token,
                "viewer_instance": worker_generation,
                "projection_generation": capture.owner.projection_generation,
            },
            "batch": {
                "accepted_revision": capture.owner.accepted_revision,
                "batch_generation": capture.owner.batch_generation,
            },
            "plan_key": {
                "snapshot_token": plan.snapshot_token,
                "revision": plan.revision,
                "job_ids": plan.jobs.iter().map(|job| job.job_id.clone()).collect::<Vec<_>>(),
            },
            "jobs": plan.jobs,
            "reserved_nets": plan.reserved_nets,
            "next_net_index": plan.next_net_index,
            "paths": plan.model_paths.iter().collect::<Vec<_>>(),
        });
        self.ensure_preview_owner_current(accepted, &capture.owner, &core, core_epoch)?;
        let worker = if let Some(worker) = self.preview_generator.borrow().as_ref() {
            worker.clone()
        } else {
            let url = resource_url("assets/preview-generator/worker.mjs")?;
            let worker = Rc::new(crate::preview_generator::PreviewGeneratorClient::new(&url)?);
            *self.preview_generator.borrow_mut() = Some(worker.clone());
            worker
        };
        let worker_reply = worker.generate(worker_request_id, &worker_request).await?;
        self.ensure_preview_owner_current(accepted, &capture.owner, &core, core_epoch)?;
        validate_preview_worker_envelope(
            &worker_reply,
            &worker_request,
            worker_generation,
            worker_request_id,
        )?;
        let results = serde_json::from_value::<Vec<ErgogenJobResult>>(
            worker_reply
                .get("results")
                .cloned()
                .ok_or_else(|| "Preview worker returned no conversion results".to_owned())?,
        )
        .map_err(|error| format!("Preview worker returned malformed results: {error}"))?;
        let finish_id = format!("case-preview-{operation}-finish");
        let finish = boardstudio_core::model::ArtifactRequest::FinishPreview {
            id: finish_id.clone(),
            request: FinishExportRequest { plan, results },
        };
        let reply = core
            .artifact(&finish_id, &epoch, &finish)
            .await
            .map_err(|error| format!("Core preview finish failed: {error}"))?;
        self.ensure_preview_owner_current(accepted, &capture.owner, &core, core_epoch)?;
        let preview = match reply {
            ArtifactReply::PreviewBoard { id, result } if id == finish_id => result,
            ArtifactReply::Error { id, error } if id == finish_id => {
                return Err(format!(
                    "Core rejected the completed Case preview: {error:?}"
                ));
            }
            ArtifactReply::PreviewBoard { .. } | ArtifactReply::Error { .. } => {
                return Err("Core returned a completed preview for another request".into());
            }
            _ => return Err("Core returned an unexpected completed-preview reply".into()),
        };
        crate::case_preview::accept_native_preview(capture, preview)
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
    pub(crate) fn export_firmware(self: &Rc<Self>) {
        let Some(scope) = self.scope() else {
            self.report("Select a board before exporting ZMK source.");
            return;
        };
        if self.model().accepted.is_none() {
            self.report("Firmware export requires a ready accepted snapshot.");
            return;
        }
        let operation_id = self.operation();
        self.firmware_exports.borrow_mut().insert(operation_id);
        self.submit(Event::StartExport {
            operation_id,
            scope,
        });
    }
    pub(crate) fn export_project_copy(self: &Rc<Self>) {
        if self.model().accepted.is_none() {
            return;
        }
        let Some(scope) = self.scope() else {
            return;
        };
        let operation_id = self.operation();
        self.archive_export_options.begin_archive(operation_id);
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

    async fn firmware_bytes(
        &self,
        operation_id: OperationId,
        snapshot: &AcceptedSnapshot,
        scope: &Scope,
    ) -> Result<Vec<u8>, String> {
        let document = snapshot.document.as_ref();
        self.ensure_firmware_export_current(operation_id, snapshot, scope, None, None)?;
        if !document
            .boards
            .iter()
            .any(|board| board.id == scope.board_id)
        {
            return Err("Select a board before export".into());
        }
        let hardware = document.hardware.as_ref();
        let instances = hardware
            .map(|hardware| hardware.instances.as_slice())
            .unwrap_or(&[]);
        let central = instances.iter().find(|instance| instance.role == "central");
        let peripheral = instances
            .iter()
            .find(|instance| instance.role == "peripheral");
        if hardware.is_some_and(|hardware| hardware.topology == HardwareTopology::Split)
            && (instances.len() != 2
                || central.is_none_or(|instance| instance.half != "left")
                || peripheral.is_none_or(|instance| instance.half != "right"))
        {
            return Err(
                "Split firmware requires a left central and a right peripheral assembly".into(),
            );
        }
        let primary_board_id = central.map_or(scope.board_id.as_str(), |instance| {
            instance.board_id.as_str()
        });
        let primary_instance_id = central.map_or(scope.instance_id.as_deref(), |instance| {
            Some(instance.id.as_str())
        });
        let core = self.core.borrow().clone();
        let executor_epoch = self.session.borrow().core_executor_epoch();
        self.ensure_firmware_export_current(
            operation_id,
            snapshot,
            scope,
            Some(&core),
            Some(executor_epoch),
        )?;
        let primary = self
            .resolve_firmware_plan(
                operation_id,
                snapshot,
                scope,
                &core,
                executor_epoch,
                FirmwarePlanTarget {
                    board_id: primary_board_id,
                    instance_id: primary_instance_id,
                    label: "central",
                },
            )
            .await?;
        let secondary = if let Some(instance) = peripheral {
            Some(
                self.resolve_firmware_plan(
                    operation_id,
                    snapshot,
                    scope,
                    &core,
                    executor_epoch,
                    FirmwarePlanTarget {
                        board_id: &instance.board_id,
                        instance_id: Some(&instance.id),
                        label: "peripheral",
                    },
                )
                .await?,
            )
        } else {
            None
        };
        self.ensure_firmware_export_current(
            operation_id,
            snapshot,
            scope,
            Some(&core),
            Some(executor_epoch),
        )?;
        let (request, _) = crate::firmware_request_adapter::firmware_request(
            document,
            &primary,
            secondary.as_ref(),
        )?;
        let generate_id = format!("firmware-generate-{}", operation_id.0);
        let request = CoreRequest::GenerateFirmware {
            id: generate_id.clone(),
            request,
        };
        let generated = core
            .request(&generate_id, &executor_epoch.0.to_string(), &request)
            .await;
        self.ensure_firmware_export_current(
            operation_id,
            snapshot,
            scope,
            Some(&core),
            Some(executor_epoch),
        )?;
        let generated =
            generated.map_err(|error| format!("Firmware generation failed: {error}"))?;
        let package = match generated {
            CoreReply::FirmwareGenerated { id, package } if id == generate_id => package,
            CoreReply::Error { id, message, .. } if id == generate_id => return Err(message),
            CoreReply::FirmwareGenerated { .. } | CoreReply::Error { .. } => {
                return Err("Core returned a firmware reply for another request.".into());
            }
            _ => return Err("Core returned an unexpected firmware-generation reply.".into()),
        };
        let mut files = package.files;
        let mut plan_value = serde_json::Map::new();
        plan_value.insert(
            "central".into(),
            serde_json::to_value(&primary).map_err(|error| error.to_string())?,
        );
        if let Some(peripheral) = &secondary {
            plan_value.insert(
                "peripheral".into(),
                serde_json::to_value(peripheral).map_err(|error| error.to_string())?,
            );
        }
        files.insert(
            "electrical-plan.json".into(),
            serde_json::to_string_pretty(&serde_json::Value::Object(plan_value))
                .map_err(|error| format!("Could not serialize electrical plan: {error}"))?,
        );
        self.ensure_firmware_export_current(
            operation_id,
            snapshot,
            scope,
            Some(&core),
            Some(executor_epoch),
        )?;
        let entries = files
            .keys()
            .enumerate()
            .map(|(index, path)| {
                Ok(ArchiveEntry {
                    path: path.clone(),
                    buffer_index: u32::try_from(index)
                        .map_err(|_| "Too many firmware package files".to_owned())?,
                })
            })
            .collect::<Result<Vec<_>, String>>()?;
        let metadata = serde_json::to_string(&ArchiveRequest::PackFiles { entries })
            .map_err(|error| format!("Could not prepare ZIP package request: {error}"))?;
        let buffers = files
            .values()
            .map(|contents| Uint8Array::from(contents.as_bytes()))
            .collect();
        let pack_id = format!("firmware-pack-{}", operation_id.0);
        let packed = core
            .archive(&pack_id, &executor_epoch.0.to_string(), &metadata, buffers)
            .await;
        self.ensure_firmware_export_current(
            operation_id,
            snapshot,
            scope,
            Some(&core),
            Some(executor_epoch),
        )?;
        let packed = packed.map_err(|error| format!("Firmware packaging failed: {error}"))?;
        match serde_json::from_str::<ArchiveReply>(&packed.metadata)
            .map_err(|error| format!("Could not read firmware package result: {error}"))?
        {
            ArchiveReply::Packed => packed
                .buffers
                .first()
                .map(Uint8Array::to_vec)
                .ok_or_else(|| "Firmware package returned no ZIP bytes.".into()),
            ArchiveReply::Error { message } => Err(format!("Firmware packaging failed: {message}")),
            ArchiveReply::Unpacked { .. } => {
                Err("Core returned an unexpected unpacked archive.".into())
            }
        }
    }

    async fn resolve_firmware_plan(
        &self,
        operation_id: OperationId,
        snapshot: &AcceptedSnapshot,
        scope: &Scope,
        core: &Rc<CoreWorker>,
        executor_epoch: boardstudio_application::ExecutorEpoch,
        target: FirmwarePlanTarget<'_>,
    ) -> Result<ElectricalPlan, String> {
        self.ensure_firmware_export_current(
            operation_id,
            snapshot,
            scope,
            Some(core),
            Some(executor_epoch),
        )?;
        let configuration = snapshot.document.hardware.as_ref().and_then(|hardware| {
            hardware
                .boards
                .iter()
                .find(|configuration| configuration.board_id == target.board_id)
        });
        let controller_part_id = snapshot
            .document
            .hardware
            .as_ref()
            .and_then(|hardware| {
                hardware
                    .instances
                    .iter()
                    .find(|instance| Some(instance.id.as_str()) == target.instance_id)
            })
            .and_then(|instance| instance.controller_part_id.clone())
            .or_else(|| {
                configuration.and_then(|configuration| configuration.controller_part_id.clone())
            });
        let request_id = format!("firmware-electrical-{}-{}", operation_id.0, target.label);
        let request = CoreRequest::ResolveElectrical {
            id: request_id.clone(),
            request: ElectricalPlanRequest {
                document: (*snapshot.document).clone(),
                instance_id: target.instance_id.map(str::to_owned),
                mode: configuration
                    .map_or(ElectricalMode::Matrix, |configuration| configuration.mode),
                locks: configuration.map_or_else(Default::default, |configuration| {
                    configuration.locks.clone()
                }),
                controller_profile: None,
                board_id: Some(target.board_id.to_owned()),
                controller_part_id,
            },
        };
        let reply = core
            .request(&request_id, &executor_epoch.0.to_string(), &request)
            .await;
        self.ensure_firmware_export_current(
            operation_id,
            snapshot,
            scope,
            Some(core),
            Some(executor_epoch),
        )?;
        let reply = reply.map_err(|error| format!("Wiring resolution failed: {error}"))?;
        let plan = match reply {
            CoreReply::ElectricalResolved { id, plan } if id == request_id => plan,
            CoreReply::Error { id, message, .. } if id == request_id => return Err(message),
            CoreReply::ElectricalResolved { .. } | CoreReply::Error { .. } => {
                return Err("Core returned a wiring reply for another firmware plan.".into());
            }
            _ => {
                return Err("Core returned an unexpected firmware wiring-resolution reply.".into());
            }
        };
        if plan.revision != snapshot.document.revision
            || plan.board_id.as_deref() != Some(target.board_id)
            || plan.instance_id.as_deref() != target.instance_id
        {
            return Err(
                "Core resolved firmware wiring for another board, instance, or revision.".into(),
            );
        }
        Ok(plan)
    }

    fn ensure_firmware_export_current(
        &self,
        operation_id: OperationId,
        snapshot: &AcceptedSnapshot,
        scope: &Scope,
        core: Option<&Rc<CoreWorker>>,
        executor_epoch: Option<boardstudio_application::ExecutorEpoch>,
    ) -> Result<(), String> {
        if !self.export_current(operation_id, snapshot.token, scope) {
            return Err("Firmware export was cancelled or superseded.".into());
        }
        let model = self.model();
        if model.accepted.as_ref().is_none_or(|current| {
            current.session_epoch != snapshot.session_epoch
                || current.document.id != snapshot.document.id
                || current.document.revision != snapshot.document.revision
                || current.scene.revision != snapshot.scene.revision
                || current.scene.revision != current.document.revision
        }) {
            return Err("The accepted project or scene changed during firmware export.".into());
        }
        if let (Some(expected_core), Some(expected_epoch)) = (core, executor_epoch)
            && (self.session.borrow().core_executor_epoch() != expected_epoch
                || !Rc::ptr_eq(expected_core, &self.core.borrow().clone()))
        {
            return Err("The Core worker changed during firmware export.".into());
        }
        Ok(())
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

#[derive(Default)]
struct NativeModelDeliveryState {
    pending: Option<crate::case_preview::CasePreviewOwnerIdentity>,
    failed: Option<crate::case_preview::CasePreviewOwnerIdentity>,
    published: Option<(
        crate::case_preview::CasePreviewOwnerIdentity,
        crate::presentation::model_delivery::ModelDeliveryRows,
    )>,
}

/// Admission and mapping completion for the version-triggered Case delivery callback.
async fn resolve_native_model_paths<
    F: std::future::Future<Output = Result<Vec<Option<String>>, String>>,
>(
    state: &RefCell<NativeModelDeliveryState>,
    owner: &crate::case_preview::CasePreviewOwnerIdentity,
    paths: Vec<String>,
    is_current: impl Fn() -> bool,
    changed: impl Fn(),
    resolve: impl FnOnce(Vec<String>) -> F,
) -> Result<Option<BTreeMap<String, Option<String>>>, String> {
    if !is_current() {
        return Ok(None);
    }
    {
        let mut state = state.borrow_mut();
        if state
            .published
            .as_ref()
            .is_some_and(|(published, _)| published == owner)
            || state.pending.as_ref() == Some(owner)
            || state.failed.as_ref() == Some(owner)
        {
            return Ok(None);
        }
        state.published = None;
        state.failed = None;
        state.pending = Some(owner.clone());
    }
    changed();
    let result = resolve(paths.clone()).await.and_then(|ids| {
        if ids.len() == paths.len() {
            Ok(ids)
        } else {
            Err("Ergogen returned an incomplete model path mapping".into())
        }
    });
    // Mapping imports may outlive the preview lease or a replacement request.
    // Stale completion must neither retire the replacement nor escape to global status.
    if !is_current() || state.borrow().pending.as_ref() != Some(owner) {
        let mut state = state.borrow_mut();
        if state.pending.as_ref() == Some(owner) {
            state.pending = None;
        }
        return Ok(None);
    }
    match result {
        Ok(ids) => Ok(Some(paths.into_iter().zip(ids).collect())),
        Err(error) => {
            {
                let mut state = state.borrow_mut();
                state.pending = None;
                // Runtime::report also notifies the version subscriber. Retain this exact
                // failure before notifying so it cannot automatically admit itself again.
                state.failed = Some(owner.clone());
            }
            changed();
            Err(error)
        }
    }
}

#[cfg(all(test, target_arch = "wasm32"))]
#[path = "native_model_mapping_tests.rs"]
mod native_model_mapping_tests;

fn validate_preview_worker_envelope(
    reply: &serde_json::Value,
    request: &serde_json::Value,
    worker_generation: u64,
    request_id: u64,
) -> Result<(), String> {
    if reply.get("kind").and_then(serde_json::Value::as_str) == Some("preview-generator-error") {
        let message = reply
            .get("message")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("preview conversion failed");
        return Err(format!("Ergogen preview conversion failed: {message}"));
    }
    if reply.get("kind").and_then(serde_json::Value::as_str) != Some("generated-preview-jobs")
        || reply.get("worker_generation") != Some(&serde_json::json!(worker_generation))
        || reply.get("request_id") != Some(&serde_json::json!(request_id))
    {
        return Err("Preview worker returned an unexpected request identity".into());
    }
    for field in ["owner", "batch", "plan_key"] {
        if reply.get(field) != request.get(field) {
            return Err(format!(
                "Preview worker changed its captured {field} identity"
            ));
        }
    }
    Ok(())
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
fn deliver(bytes: &[u8], filename: &str, media_type: Option<&str>) -> Result<(), String> {
    let parts = Array::new();
    parts.push(&Uint8Array::from(bytes));
    let blob = if let Some(media_type) = media_type {
        let options = BlobPropertyBag::new();
        options.set_type(media_type);
        Blob::new_with_u8_array_sequence_and_options(&parts, &options)
            .map_err(|e| format!("{e:?}"))?
    } else {
        Blob::new_with_u8_array_sequence(&parts).map_err(|e| format!("{e:?}"))?
    };
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
