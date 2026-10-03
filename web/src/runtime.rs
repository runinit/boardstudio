//! Browser composition runs identified effects; the headless session remains authoritative.
use crate::archive_export::{ArchiveExportOptions, ArchiveWorkFuture, archive_filename};
use crate::pcb_wiring_mode_operation::electrical_preview_request;
#[cfg(test)]
use boardstudio_application::GenerationStatus;
use boardstudio_application::{
    AcceptedSnapshot, Completion, Durability, Effect, Event, JobId, Lifecycle, OperationId,
    ReadModel, SaveResult, Scope, Session, SnapshotToken, TerminalOutcome,
};
use boardstudio_core::{
    electrical::{ElectricalMode, ElectricalPlan, ElectricalPlanRequest},
    model::{
        ArchiveEntry, ArchiveReply, ArchiveRequest, ArtifactReply, ArtifactRequest, Board,
        CompiledFootprint, CoreReply, CoreRequest, ErgogenJobResult, FinishExportRequest,
        HardwareTopology, KeycapSpec, Material, MechanicalAssembly, MechanicalBuiltinProfile,
        MechanicalConfiguration, MechanicalPartProfile, MechanicalSwitchFamily, Operation,
        OutlineFeature, OutlineSettings, PcbPreview, PrepareExportRequest, ProjectDoc,
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
use sha2::{Digest, Sha256};

pub struct CadScene {
    pub scope: Scope,
    pub token: SnapshotToken,
    pub snapshot: AcceptedSnapshot,
    pub result: CadResult,
    pub(crate) prepared: boardstudio_core::model::PreparedCaseAssemblyIR,
    pub(crate) physical_fingerprint: Option<[u8; 32]>,
    pub mechanical: Option<boardstudio_core::model::MechanicalAssembly>,
    pub exact: bool,
    pub contours: Vec<boardstudio_core::model::Contour>,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct KeycapsPreviewInput {
    pub(crate) scope: Scope,
    pub(crate) token: SnapshotToken,
    pub(crate) revision: u64,
    pub(crate) specs: Vec<KeycapSpec>,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct KeycapsCadPreview {
    pub(crate) generation: u64,
    pub(crate) scope: Scope,
    pub(crate) token: SnapshotToken,
    pub(crate) revision: u64,
    pub(crate) specs: Vec<KeycapSpec>,
    pub(crate) bodies: Vec<boardstudio_web::cad_jobs::CadBodyMesh>,
}

#[cfg(test)]
struct ProjectNamePersistGate {
    entered: futures_channel::oneshot::Sender<()>,
    release: futures_channel::oneshot::Receiver<()>,
}

#[cfg(test)]
enum ProjectNamePersistTestBehavior {
    Fail(String),
    Gate(ProjectNamePersistGate),
}
use gloo_timers::future::TimeoutFuture;
use js_sys::{Array, Function, JsString, Object, Reflect, Uint8Array};
use std::{
    cell::{Cell, RefCell},
    collections::{BTreeMap, BTreeSet, VecDeque},
    future::Future,
    pin::Pin,
    rc::Rc,
};
use wasm_bindgen::{JsCast, JsValue, closure::Closure};
use wasm_bindgen_futures::{JsFuture, spawn_local};
use web_sys::{Blob, BlobPropertyBag, HtmlAnchorElement, SvgElement, Url};

type Notifier = Rc<dyn Fn()>;
type Frame = (i32, Closure<dyn FnMut(f64)>);
struct Artifact {
    operation_id: OperationId,
    bytes: Vec<u8>,
    filename: String,
    media_type: Option<String>,
    scope: Scope,
    token: SnapshotToken,
    firmware: bool,
    keycaps_step: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RuntimeReportSeverity {
    Status,
    Alert,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct RuntimeReport {
    message: String,
    severity: RuntimeReportSeverity,
}

impl RuntimeReport {
    fn status(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            severity: RuntimeReportSeverity::Status,
        }
    }

    fn alert(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            severity: RuntimeReportSeverity::Alert,
        }
    }

    fn clear_alert(&mut self) {
        if self.severity == RuntimeReportSeverity::Alert {
            self.message.clear();
            self.severity = RuntimeReportSeverity::Status;
        }
    }
}

fn firmware_export_terminal_report(
    capture_is_current: bool,
    outcome: TerminalOutcome,
    delivery_error: Option<String>,
) -> Option<RuntimeReport> {
    if !capture_is_current {
        return None;
    }
    match outcome {
        TerminalOutcome::Completed => Some(match delivery_error {
            Some(error) => RuntimeReport::alert(error),
            None => RuntimeReport::status("Saved locally."),
        }),
        TerminalOutcome::Rejected(reason)
        | TerminalOutcome::PersistenceFailed(reason)
        | TerminalOutcome::BlockedByRecovery(reason)
        | TerminalOutcome::ExecutorFailed(reason) => Some(RuntimeReport::alert(reason)),
        TerminalOutcome::Cancelled => Some(RuntimeReport::status("Cancelled.")),
        TerminalOutcome::Closed => Some(RuntimeReport::status("Editor closed.")),
        TerminalOutcome::Superseded => None,
    }
}

fn firmware_export_worker_is_current(
    expected_epoch: boardstudio_application::ExecutorEpoch,
    current_epoch: boardstudio_application::ExecutorEpoch,
    same_worker: bool,
) -> bool {
    expected_epoch == current_epoch && same_worker
}

fn firmware_export_bytes_for_delivery(
    result: Result<Vec<u8>, String>,
    owner_is_current: bool,
    cancelled: bool,
) -> Result<Vec<u8>, String> {
    match result {
        Ok(_) if !owner_is_current || cancelled => {
            Err("Export scope changed before delivery.".into())
        }
        other => other,
    }
}

fn pcb_wiring_is_applied(document: &ProjectDoc, plan: &ElectricalPlan) -> bool {
    let Some(board_id) = plan.board_id.as_deref() else {
        return false;
    };
    if plan.revision != document.revision {
        return false;
    }
    let Some(board) = document.boards.iter().find(|board| board.id == board_id) else {
        return false;
    };
    let configuration = document.hardware.as_ref().and_then(|hardware| {
        hardware
            .boards
            .iter()
            .find(|entry| entry.board_id == board_id)
    });
    let prefix = format!("generated/electrical/{board_id}/");
    let current = document
        .nets
        .iter()
        .filter(|net| net.id.starts_with(&prefix))
        .collect::<Vec<_>>();
    current.len() == plan.nets.len()
        && configuration.is_some_and(|configuration| {
            configuration.mode == plan.mode
                && configuration.controller_part_id == plan.controller_part_id
        })
        && plan
            .nets
            .iter()
            .all(|net| current.contains(&net) && board.net_ids.contains(&net.id))
}

fn firmware_export_capture_matches(
    capture: &FirmwareExportCapture,
    scope: Option<&Scope>,
    snapshot: Option<&FirmwareAcceptedIdentity>,
) -> bool {
    scope == Some(&capture.scope)
        && snapshot.is_some_and(|snapshot| {
            snapshot.session_epoch == capture.session_epoch
                && snapshot.document_id == capture.document_id
                && snapshot.token == capture.token
                && snapshot.revision == capture.revision
                && snapshot.scene_revision == capture.revision
        })
}

fn firmware_export_is_latest(
    operation_id: OperationId,
    latest_operation_id: Option<OperationId>,
) -> bool {
    latest_operation_id == Some(operation_id)
}

type FirmwareExecutorFuture<'a, T> = Pin<Box<dyn Future<Output = Result<T, String>> + 'a>>;

/// Narrow private adapter for the three awaited Core operations in a firmware export. Keeping
/// the await points behind this interface lets lifecycle tests replace the worker while a real
/// production export is suspended, without adding a second provider or changing Core's API.
pub(crate) trait FirmwareExportExecutor {
    fn request<'a>(
        &'a self,
        request_id: &'a str,
        executor_epoch: &'a str,
        request: &'a CoreRequest,
    ) -> FirmwareExecutorFuture<'a, CoreReply>;

    fn archive<'a>(
        &'a self,
        request_id: &'a str,
        executor_epoch: &'a str,
        metadata: &'a str,
        buffers: Vec<Uint8Array>,
    ) -> FirmwareExecutorFuture<'a, (String, Vec<Uint8Array>)>;
}

impl FirmwareExportExecutor for CoreWorker {
    fn request<'a>(
        &'a self,
        request_id: &'a str,
        executor_epoch: &'a str,
        request: &'a CoreRequest,
    ) -> FirmwareExecutorFuture<'a, CoreReply> {
        Box::pin(async move {
            CoreWorker::request(self, request_id, executor_epoch, request)
                .await
                .map_err(|error| error.to_string())
        })
    }

    fn archive<'a>(
        &'a self,
        request_id: &'a str,
        executor_epoch: &'a str,
        metadata: &'a str,
        buffers: Vec<Uint8Array>,
    ) -> FirmwareExecutorFuture<'a, (String, Vec<Uint8Array>)> {
        Box::pin(async move {
            CoreWorker::archive(self, request_id, executor_epoch, metadata, buffers)
                .await
                .map(|result| (result.metadata, result.buffers))
                .map_err(|error| error.to_string())
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct FirmwareExportCapture {
    scope: Scope,
    token: SnapshotToken,
    revision: u64,
    session_epoch: boardstudio_application::SessionEpoch,
    document_id: String,
    executor_epoch: boardstudio_application::ExecutorEpoch,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct FootprintExportCapture {
    scope: Scope,
    token: SnapshotToken,
    revision: u64,
    session_epoch: boardstudio_application::SessionEpoch,
    document_id: String,
    executor_epoch: boardstudio_application::ExecutorEpoch,
    core_worker_identity: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct PcbHandoffCapture {
    scope: Scope,
    token: SnapshotToken,
    revision: u64,
    session_epoch: boardstudio_application::SessionEpoch,
    document_id: String,
    executor_epoch: boardstudio_application::ExecutorEpoch,
    core_worker_identity: usize,
    draft: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct FirmwareAcceptedIdentity {
    session_epoch: boardstudio_application::SessionEpoch,
    document_id: String,
    token: SnapshotToken,
    revision: u64,
    scene_revision: u64,
}

#[cfg(test)]
struct FirmwareExportTestContext {
    accepted: AcceptedSnapshot,
    scope: Option<Scope>,
    current_executor: Rc<dyn FirmwareExportExecutor>,
    executor_epoch: boardstudio_application::ExecutorEpoch,
}

#[cfg(test)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct FirmwareTestDelivery {
    pub(crate) bytes: Vec<u8>,
    pub(crate) filename: String,
    pub(crate) media_type: Option<String>,
}

impl From<&AcceptedSnapshot> for FirmwareAcceptedIdentity {
    fn from(snapshot: &AcceptedSnapshot) -> Self {
        Self {
            session_epoch: snapshot.session_epoch,
            document_id: snapshot.document.id.clone(),
            token: snapshot.token,
            revision: snapshot.document.revision,
            scene_revision: snapshot.scene.revision,
        }
    }
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

/// Allocate the same browser UUID used by the Session-owned project lifecycle.
pub(crate) fn new_project_id() -> Result<String, String> {
    browser_uuid()
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
    status: RefCell<RuntimeReport>,
    open_sequence: Cell<u64>,
    project_deletion_pending: Cell<bool>,
    project_deletion_open: Cell<Option<OperationId>>,
    cad_scene: RefCell<Option<Rc<CadScene>>>,
    cad_worker: RefCell<Option<(Scope, Rc<CadWorker>)>>,
    keycaps_preview_generation: Cell<u64>,
    keycaps_preview_worker: RefCell<Option<(u64, Rc<CadWorker>)>>,
    cad_jobs: RefCell<BTreeMap<JobId, Rc<Cell<bool>>>>,
    step_exports: RefCell<BTreeSet<OperationId>>,
    keycaps_step_exports: RefCell<BTreeSet<OperationId>>,
    firmware_exports: RefCell<BTreeMap<OperationId, FirmwareExportCapture>>,
    footprint_exports: RefCell<BTreeMap<OperationId, FootprintExportCapture>>,
    pcb_handoff_exports: RefCell<BTreeMap<OperationId, PcbHandoffCapture>>,
    latest_firmware_export: Cell<Option<OperationId>>,
    firmware_export_delivery_errors: RefCell<BTreeMap<OperationId, String>>,
    export_workers: RefCell<BTreeMap<OperationId, Rc<CadWorker>>>,
    native_case_preview: RefCell<crate::case_preview::NativePreviewState>,
    case_model_delivery: crate::presentation::model_delivery::ModelDeliveryAdapter,
    native_model_delivery: RefCell<NativeModelDeliveryState>,
    layout_preview: RefCell<crate::presentation::layout_viewer_source::LayoutPreviewState>,
    layout_model_rows: RefCell<
        Option<(
            crate::presentation::layout_viewer_source::LayoutSourceIdentity,
            crate::presentation::model_delivery::ModelDeliveryRows,
        )>,
    >,
    layout_model_batch_generation: Cell<u64>,
    native_model_jobs: RefCell<BTreeSet<String>>,
    preview_generator: RefCell<Option<Rc<crate::preview_generator::PreviewGeneratorClient>>>,
    archive_export_options: ArchiveExportOptions,
    #[cfg(test)]
    definition_name_test_state: RefCell<Option<(AcceptedSnapshot, Option<Scope>)>>,
    #[cfg(test)]
    layout_component_inspector_test_state: RefCell<Option<(ReadModel, Option<Scope>)>>,
    #[cfg(test)]
    layout_component_inspector_test_events: RefCell<Vec<Event>>,
    #[cfg(test)]
    definition_name_test_events: RefCell<Vec<Event>>,
    #[cfg(test)]
    definition_name_test_generation: RefCell<Option<GenerationStatus>>,
    #[cfg(test)]
    firmware_export_test_context: RefCell<Option<FirmwareExportTestContext>>,
    #[cfg(test)]
    firmware_export_test_effects: RefCell<Vec<Effect>>,
    #[cfg(test)]
    firmware_export_test_events: RefCell<Vec<Event>>,
    #[cfg(test)]
    firmware_export_test_deliveries: RefCell<Vec<FirmwareTestDelivery>>,
    #[cfg(test)]
    project_name_test_core: RefCell<Option<boardstudio_core::CoreEngine>>,
    #[cfg(test)]
    project_name_persist_test_behavior: RefCell<Option<ProjectNamePersistTestBehavior>>,
    #[cfg(test)]
    project_name_test_effects: RefCell<Vec<Effect>>,
}

struct ProjectDeletionLease {
    runtime: Rc<Runtime>,
}

impl Drop for ProjectDeletionLease {
    fn drop(&mut self) {
        self.runtime.project_deletion_open.set(None);
        self.runtime.project_deletion_pending.set(false);
    }
}

impl Runtime {
    pub fn new() -> Result<Rc<Self>, String> {
        Self::new_with_restoration(true)
    }

    fn new_with_restoration(restore_active_project: bool) -> Result<Rc<Self>, String> {
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
            status: RefCell::new(RuntimeReport::status(
                "Open a saved keyboard or an editable demo copy.",
            )),
            open_sequence: Cell::new(0),
            project_deletion_pending: Cell::new(false),
            project_deletion_open: Cell::new(None),
            cad_scene: RefCell::new(None),
            cad_worker: RefCell::new(None),
            keycaps_preview_generation: Cell::new(0),
            keycaps_preview_worker: RefCell::new(None),
            cad_jobs: RefCell::new(BTreeMap::new()),
            step_exports: RefCell::new(BTreeSet::new()),
            keycaps_step_exports: RefCell::new(BTreeSet::new()),
            firmware_exports: RefCell::new(BTreeMap::new()),
            footprint_exports: RefCell::new(BTreeMap::new()),
            pcb_handoff_exports: RefCell::new(BTreeMap::new()),
            latest_firmware_export: Cell::new(None),
            firmware_export_delivery_errors: RefCell::new(BTreeMap::new()),
            export_workers: RefCell::new(BTreeMap::new()),
            native_case_preview: RefCell::new(Default::default()),
            case_model_delivery: Default::default(),
            native_model_delivery: RefCell::new(Default::default()),
            layout_preview: RefCell::new(Default::default()),
            layout_model_rows: RefCell::new(None),
            layout_model_batch_generation: Cell::new(0),
            native_model_jobs: RefCell::new(BTreeSet::new()),
            preview_generator: RefCell::new(None),
            archive_export_options: ArchiveExportOptions::default(),
            #[cfg(test)]
            definition_name_test_state: RefCell::new(None),
            #[cfg(test)]
            layout_component_inspector_test_state: RefCell::new(None),
            #[cfg(test)]
            layout_component_inspector_test_events: RefCell::new(Vec::new()),
            #[cfg(test)]
            definition_name_test_events: RefCell::new(Vec::new()),
            #[cfg(test)]
            definition_name_test_generation: RefCell::new(None),
            #[cfg(test)]
            firmware_export_test_context: RefCell::new(None),
            #[cfg(test)]
            firmware_export_test_effects: RefCell::new(Vec::new()),
            #[cfg(test)]
            firmware_export_test_events: RefCell::new(Vec::new()),
            #[cfg(test)]
            firmware_export_test_deliveries: RefCell::new(Vec::new()),
            #[cfg(test)]
            project_name_test_core: RefCell::new(None),
            #[cfg(test)]
            project_name_persist_test_behavior: RefCell::new(None),
            #[cfg(test)]
            project_name_test_effects: RefCell::new(Vec::new()),
        });
        // Reserve the startup open identity synchronously, before any explicit
        // open action can supersede restoration of the last durable project.
        if restore_active_project {
            runtime.restore_active_project();
        }
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
        if let Some(context) = self.firmware_export_test_context.borrow().as_ref() {
            return context.scope.clone();
        }
        #[cfg(test)]
        if let Some((_, scope)) = self.definition_name_test_state.borrow().as_ref() {
            return scope.clone();
        }
        #[cfg(test)]
        if let Some((_, scope)) = self.layout_component_inspector_test_state.borrow().as_ref() {
            return scope.clone();
        }
        self.session.borrow().scope()
    }
    pub(crate) fn electrical_preview_executor_epoch(&self) -> u64 {
        #[cfg(test)]
        if let Some(context) = self.firmware_export_test_context.borrow().as_ref() {
            return context.executor_epoch.0;
        }
        self.session.borrow().core_executor_epoch().0
    }
    pub fn model(&self) -> ReadModel {
        #[cfg(test)]
        if let Some(context) = self.firmware_export_test_context.borrow().as_ref() {
            return ReadModel {
                accepted: Some(context.accepted.clone()),
                active_board_id: context
                    .scope
                    .as_ref()
                    .map(|scope| scope.board_id.clone())
                    .unwrap_or_default(),
                active_instance_id: context
                    .scope
                    .as_ref()
                    .and_then(|scope| scope.instance_id.clone()),
                ..ReadModel::default()
            };
        }
        #[cfg(test)]
        if let Some((snapshot, scope)) = self.definition_name_test_state.borrow().as_ref() {
            return ReadModel {
                accepted: Some(snapshot.clone()),
                active_board_id: scope
                    .as_ref()
                    .map(|scope| scope.board_id.clone())
                    .unwrap_or_default(),
                active_instance_id: scope.as_ref().and_then(|scope| scope.instance_id.clone()),
                generation: self
                    .definition_name_test_generation
                    .borrow()
                    .clone()
                    .unwrap_or(GenerationStatus::Idle),
                ..ReadModel::default()
            };
        }
        #[cfg(test)]
        if let Some((model, _)) = self.layout_component_inspector_test_state.borrow().as_ref() {
            return model.clone();
        }
        self.session.borrow().read_model().clone()
    }

    /// Load a reviewed bundled switch fit into the Parts editor's local draft.
    /// This never submits an accepted edit; the existing Parts profile Save path
    /// remains the sole owner of document history.
    pub(crate) async fn standard_switch_profile(
        &self,
        operation_id: OperationId,
        definition_id: String,
        family: MechanicalSwitchFamily,
        plate_to_pcb: f64,
    ) -> Result<MechanicalPartProfile, String> {
        let executor_epoch = self.session.borrow().core_executor_epoch();
        let core = self.core.borrow().clone();
        let request_id = format!("parts-standard-profile-{}", operation_id.0);
        let source = match family {
            MechanicalSwitchFamily::Mx => MechanicalBuiltinProfile::MxSwitch,
            MechanicalSwitchFamily::ChocV1 => MechanicalBuiltinProfile::ChocV1Switch,
            MechanicalSwitchFamily::ChocV2 => MechanicalBuiltinProfile::ChocV2Switch,
        };
        let request = CoreRequest::MechanicalProfile {
            id: request_id.clone(),
            definition_id: definition_id.clone(),
            source,
            plate_to_pcb,
        };
        let reply = core
            .request(&request_id, &executor_epoch.0.to_string(), &request)
            .await
            .map_err(|error| format!("Standard switch fit failed: {error}"))?;
        if self.session.borrow().core_executor_epoch() != executor_epoch
            || !Rc::ptr_eq(&core, &self.core.borrow())
        {
            return Err("Core worker changed during standard switch fit lookup.".into());
        }
        crate::parts_mechanical_profile::standard_profile_reply_matches(
            reply,
            &request_id,
            &definition_id,
            family,
            plate_to_pcb,
        )
    }

    /// Import a source-owned KiCad footprint through the existing artifact worker.
    /// The Parts owner admits the returned definition into Session after rechecking scope.
    pub(crate) async fn import_footprint(
        &self,
        request_id: String,
        definition_id: String,
        source: String,
    ) -> Result<CompiledFootprint, String> {
        let executor_epoch = self.session.borrow().core_executor_epoch();
        let core = self.core.borrow().clone();
        let request = ArtifactRequest::ImportFootprint {
            id: request_id.clone(),
            definition_id,
            source,
        };
        let reply = core
            .artifact(&request_id, &executor_epoch.0.to_string(), &request)
            .await
            .map_err(|error| format!("Could not import KiCad footprint: {error}"))?;
        if self.session.borrow().core_executor_epoch() != executor_epoch
            || !Rc::ptr_eq(&core, &self.core.borrow())
        {
            return Err(
                "The KiCad footprint import became stale when the Core worker changed.".into(),
            );
        }
        match reply {
            ArtifactReply::ImportFootprint { id, result } if id == request_id => Ok(*result),
            ArtifactReply::Error { id, error } if id == request_id => {
                let diagnostics = error
                    .diagnostics
                    .iter()
                    .map(|diagnostic| diagnostic.message.as_str())
                    .collect::<Vec<_>>();
                let details = if diagnostics.is_empty() {
                    String::new()
                } else {
                    format!(" {}", diagnostics.join(" "))
                };
                Err(format!("{}{}", error.message, details))
            }
            ArtifactReply::ImportFootprint { .. } | ArtifactReply::Error { .. } => {
                Err("Core returned a KiCad footprint result for another request.".into())
            }
            _ => Err("Core returned an unexpected KiCad footprint import reply.".into()),
        }
    }

    /// Ask the existing Core worker to project candidate matrices with its authoritative
    /// layout geometry. This is a private preview path; candidates are never installed in Session.
    pub(crate) async fn project_matrices(
        &self,
        base_revision: u64,
        matrices: Vec<boardstudio_core::model::Matrix>,
    ) -> Result<Vec<boardstudio_core::model::MatrixScene>, String> {
        let executor_epoch = self.session.borrow().core_executor_epoch();
        let core = self.core.borrow().clone();
        let request_id = format!("matrix-projection-{}", self.operation().0);
        let expected_ids: Vec<_> = matrices.iter().map(|matrix| matrix.id.clone()).collect();
        let request = CoreRequest::ProjectMatrices {
            id: request_id.clone(),
            base_revision,
            matrices,
        };
        let reply = core
            .request(&request_id, &executor_epoch.0.to_string(), &request)
            .await
            .map_err(|error| format!("Matrix preview failed: {error}"))?;
        if self.session.borrow().core_executor_epoch() != executor_epoch
            || !Rc::ptr_eq(&core, &self.core.borrow())
        {
            return Err("The Core worker changed during matrix preview.".into());
        }
        match reply {
            CoreReply::MatrixProjections {
                id,
                revision,
                matrix_scenes,
            } if id == request_id && revision == base_revision => {
                let actual_ids: Vec<_> = matrix_scenes
                    .iter()
                    .map(|scene| scene.matrix_id.clone())
                    .collect();
                if actual_ids != expected_ids {
                    return Err("Core returned a different matrix preview set.".into());
                }
                Ok(matrix_scenes)
            }
            CoreReply::Error { id, message, .. } if id == request_id => Err(message),
            CoreReply::MatrixProjections { .. } | CoreReply::Error { .. } => {
                Err("Core returned a stale matrix preview reply.".into())
            }
            _ => Err("Core returned an unexpected matrix preview reply.".into()),
        }
    }
    pub fn status(&self) -> String {
        self.status.borrow().message.clone()
    }
    pub(crate) fn status_is_alert(&self) -> bool {
        self.status.borrow().severity == RuntimeReportSeverity::Alert
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
        self.apply_report(RuntimeReport::status(status));
    }
    fn apply_report(&self, report: RuntimeReport) {
        *self.status.borrow_mut() = report;
        self.changed();
    }
    fn clear_alert(&self) {
        self.status.borrow_mut().clear_alert();
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
        self.create_new_keyboard_inner(false)
    }

    fn create_new_keyboard_inner(
        self: &Rc<Self>,
        allow_project_deletion: bool,
    ) -> Result<(String, crate::operation_outcomes::OutcomeSlot), String> {
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

        let operation_id = self.operation();
        let outcome = self.observe_operation(operation_id);
        if allow_project_deletion {
            self.project_deletion_open.set(Some(operation_id));
        }
        self.submit(Event::Open {
            operation_id,
            document,
        });
        Ok((project_id, outcome))
    }

    pub(crate) async fn delete_saved_project(
        self: &Rc<Self>,
        project_id: String,
    ) -> Result<(), String> {
        let lease = self.begin_project_deletion()?;
        self.supersede_pending_opens_for_deletion()?;
        let initial = if self.model().lifecycle == Lifecycle::RecoveryRequired {
            self.recover_active_project_for_deletion().await?
        } else {
            self.wait_for_saved_active_document().await?
        };

        let deleting_active = initial.document.id == project_id;
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
                    .load_document(replacement_id.clone())
                    .await
                    .map_err(|error| format!("Could not open the next saved keyboard: {error}"))?
                    .ok_or_else(|| {
                        "The next saved keyboard is unavailable; the current keyboard was kept."
                            .to_owned()
                    })?;
                self.ensure_delete_owner(&initial)?;
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
            .delete_project(project_id)
            .await
            .map_err(|error| format!("This keyboard could not be deleted. Try again. {error}"))?;
        drop(lease);
        Ok(())
    }

    fn begin_project_deletion(self: &Rc<Self>) -> Result<ProjectDeletionLease, String> {
        if self.project_deletion_pending.replace(true) {
            return Err("Another saved keyboard deletion is already in progress.".into());
        }
        Ok(ProjectDeletionLease {
            runtime: self.clone(),
        })
    }

    fn supersede_pending_opens_for_deletion(&self) -> Result<u64, String> {
        let sequence = self
            .open_sequence
            .get()
            .checked_add(1)
            .ok_or_else(|| "Open identity exhausted.".to_owned())?;
        self.open_sequence.set(sequence);
        Ok(sequence)
    }

    fn ensure_delete_owner(&self, initial: &AcceptedSnapshot) -> Result<(), String> {
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

    async fn wait_for_saved_active_document(self: &Rc<Self>) -> Result<AcceptedSnapshot, String> {
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

    async fn open_project_for_deletion(
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

    async fn recover_active_project_for_deletion(
        self: &Rc<Self>,
    ) -> Result<AcceptedSnapshot, String> {
        let accepted = self.model().accepted.ok_or_else(|| {
            "Recover or open a saved keyboard before deleting the current keyboard.".to_owned()
        })?;
        let document = self
            .store
            .load_document(accepted.document.id.clone())
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

    async fn wait_for_project_open(
        self: &Rc<Self>,
        outcome: crate::operation_outcomes::OutcomeSlot,
        expected_id: &str,
        expected_sequence: Option<u64>,
    ) -> Result<AcceptedSnapshot, String> {
        for _ in 0..1_200 {
            #[cfg(all(test, target_arch = "wasm32"))]
            crate::runtime::project_name_test_support::run_pending(self).await;
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

        let request_id = format!("pcb-electrical-{}", self.operation().0);
        let request = electrical_preview_request(&request_id, &accepted.document, &scope.board_id);
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

    /// Build the mesh preview from the already accepted Core keycap resolution. This worker is
    /// independent from Case generation and STEP export so superseding a keycap view cannot
    /// cancel or publish through either owner.
    pub(crate) async fn request_keycaps_cad_preview(
        self: &Rc<Self>,
        input: KeycapsPreviewInput,
    ) -> Result<KeycapsCadPreview, String> {
        let generation = self
            .keycaps_preview_generation
            .get()
            .checked_add(1)
            .ok_or_else(|| "Keycap CAD preview identity exhausted".to_owned())?;
        self.keycaps_preview_generation.set(generation);
        if let Some((_, previous)) = self.keycaps_preview_worker.borrow_mut().take() {
            previous.close();
        }

        let accepted = self
            .model()
            .accepted
            .ok_or_else(|| "The accepted Keycaps source is no longer open.".to_owned())?;
        self.ensure_keycaps_source_current(&accepted, &input.scope)?;
        if accepted.token != input.token || accepted.document.revision != input.revision {
            return Err("The accepted Keycaps source changed before CAD preview started.".into());
        }
        let identity = CadSnapshotIdentity {
            token: accepted.token.0,
            session_epoch: accepted.session_epoch.0,
            document_id: accepted.document.id.clone(),
            board_id: input.scope.board_id.clone(),
            instance_id: input.scope.instance_id.clone(),
            revision: accepted.document.revision,
        };
        let worker = Rc::new(
            CadWorker::new(&resource_url("assets/cad-worker/entry.js")?)
                .map_err(|error| error.to_string())?,
        );
        *self.keycaps_preview_worker.borrow_mut() = Some((generation, worker.clone()));
        let current =
            || {
                if self.keycaps_preview_generation.get() != generation
                    || !self.keycaps_preview_worker.borrow().as_ref().is_some_and(
                        |(active, owner)| *active == generation && Rc::ptr_eq(owner, &worker),
                    )
                {
                    return Err("Keycap CAD preview was cancelled or superseded.".to_owned());
                }
                self.ensure_keycaps_source_current(&accepted, &input.scope)?;
                if self.model().accepted.as_ref().is_none_or(|snapshot| {
                    snapshot.token != input.token || snapshot.document.revision != input.revision
                }) {
                    return Err("Keycap CAD preview belongs to an older accepted revision.".into());
                }
                Ok(())
            };
        let result = async {
            current()?;
            worker.ready().await.map_err(|error| error.to_string())?;
            current()?;
            let request_id = format!("keycaps-preview-{generation}");
            let job_id = format!("keycaps-preview-job-{generation}");
            let result = worker
                .request_keycaps_preview(request_id, job_id, identity, input.specs.clone())
                .await
                .map_err(|error| error.to_string())?;
            current()?;
            Ok(KeycapsCadPreview {
                generation,
                scope: input.scope.clone(),
                token: input.token,
                revision: input.revision,
                specs: input.specs.clone(),
                bodies: result.bodies,
            })
        }
        .await;
        if self
            .keycaps_preview_worker
            .borrow()
            .as_ref()
            .is_some_and(|(active, owner)| *active == generation && Rc::ptr_eq(owner, &worker))
        {
            self.keycaps_preview_worker.borrow_mut().take();
        }
        worker.close();
        result
    }

    /// Retire the active preview worker on source change or viewer unmount.
    pub(crate) fn cancel_keycaps_cad_preview(&self) {
        self.keycaps_preview_generation
            .set(self.keycaps_preview_generation.get().saturating_add(1));
        if let Some((_, worker)) = self.keycaps_preview_worker.borrow_mut().take() {
            worker.close();
        }
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
        if self.project_deletion_pending.get() {
            let open_operation = match &event {
                Event::Open { operation_id, .. }
                | Event::RecoverWithDocument { operation_id, .. } => Some(*operation_id),
                _ => None,
            };
            if let Some(operation_id) = open_operation {
                if self.project_deletion_open.get() == Some(operation_id) {
                    self.project_deletion_open.set(None);
                } else {
                    self.report("Wait for the saved keyboard deletion to finish before opening another project.");
                    return;
                }
            }
        }
        #[cfg(test)]
        let test_event = event.clone();
        #[cfg(test)]
        if self
            .layout_component_inspector_test_state
            .borrow()
            .is_some()
        {
            self.layout_component_inspector_test_events
                .borrow_mut()
                .push(event);
            return;
        }
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
        let previous_snapshot = self
            .model()
            .accepted
            .as_ref()
            .map(|snapshot| (snapshot.token, snapshot.document.revision));
        let effects = self.session.borrow_mut().submit(event);
        self.invalidate_stale_native_case_preview();
        let accepted_snapshot = self
            .model()
            .accepted
            .as_ref()
            .map(|snapshot| (snapshot.token, snapshot.document.revision));
        if self.scope() != previous_scope || accepted_snapshot != previous_snapshot {
            self.cancel_keycaps_cad_preview();
        }
        if self.scope() != previous_scope {
            self.cad_scene.borrow_mut().take();
            if let Some((_, worker)) = self.cad_worker.borrow_mut().take() {
                worker.close();
            }
        }
        self.changed();
        #[cfg(test)]
        if self.firmware_export_test_context.borrow().is_some() {
            self.firmware_export_test_events
                .borrow_mut()
                .push(test_event);
            self.firmware_export_test_effects
                .borrow_mut()
                .extend(effects);
            return;
        }
        #[cfg(test)]
        if self.project_name_test_core.borrow().is_some() {
            self.project_name_test_effects.borrow_mut().extend(effects);
            return;
        }
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
    pub(crate) fn set_cad_scene_test(&self, scene: Option<Rc<CadScene>>) {
        *self.cad_scene.borrow_mut() = scene;
    }

    #[cfg(test)]
    pub(crate) fn set_definition_name_test_generation(&self, generation: GenerationStatus) {
        *self.definition_name_test_generation.borrow_mut() = Some(generation);
    }

    #[cfg(test)]
    pub(crate) fn set_layout_component_inspector_test_state(
        &self,
        model: ReadModel,
        scope: Option<Scope>,
    ) {
        *self.layout_component_inspector_test_state.borrow_mut() = Some((model, scope));
    }

    #[cfg(test)]
    pub(crate) fn take_layout_component_inspector_test_events(&self) -> Vec<Event> {
        std::mem::take(&mut *self.layout_component_inspector_test_events.borrow_mut())
    }

    #[cfg(test)]
    pub(crate) fn settle_layout_component_inspector_test_operation(
        &self,
        operation: OperationId,
        outcome: TerminalOutcome,
    ) -> bool {
        self.operation_outcomes.settle(operation, outcome)
    }

    #[cfg(test)]
    fn set_firmware_export_test_context(
        &self,
        accepted: AcceptedSnapshot,
        scope: Option<Scope>,
        current_executor: Rc<dyn FirmwareExportExecutor>,
        executor_epoch: boardstudio_application::ExecutorEpoch,
        next_operation: u64,
    ) {
        *self.firmware_export_test_context.borrow_mut() = Some(FirmwareExportTestContext {
            accepted,
            scope,
            current_executor,
            executor_epoch,
        });
        self.next_operation.set(next_operation);
    }

    #[cfg(test)]
    fn replace_firmware_export_test_executor(
        &self,
        executor: Rc<dyn FirmwareExportExecutor>,
        epoch: boardstudio_application::ExecutorEpoch,
    ) {
        let mut context = self.firmware_export_test_context.borrow_mut();
        let context = context
            .as_mut()
            .expect("firmware export test context is installed");
        context.current_executor = executor;
        context.executor_epoch = epoch;
    }

    #[cfg(test)]
    fn replace_firmware_export_test_owner(&self, accepted: AcceptedSnapshot, scope: Option<Scope>) {
        let mut context = self.firmware_export_test_context.borrow_mut();
        let context = context
            .as_mut()
            .expect("firmware export test context is installed");
        context.accepted = accepted;
        context.scope = scope;
    }

    #[cfg(test)]
    fn take_firmware_export_test_effects(&self) -> Vec<Effect> {
        std::mem::take(&mut *self.firmware_export_test_effects.borrow_mut())
    }

    #[cfg(test)]
    fn take_firmware_export_test_events(&self) -> Vec<Event> {
        std::mem::take(&mut *self.firmware_export_test_events.borrow_mut())
    }

    #[cfg(test)]
    fn take_firmware_export_test_deliveries(&self) -> Vec<FirmwareTestDelivery> {
        std::mem::take(&mut *self.firmware_export_test_deliveries.borrow_mut())
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
        let previous_scope = self.scope();
        let previous_snapshot = self
            .model()
            .accepted
            .as_ref()
            .map(|snapshot| (snapshot.token, snapshot.document.revision));
        let effects = self.session.borrow_mut().complete(event);
        self.invalidate_stale_native_case_preview();
        let accepted_snapshot = self
            .model()
            .accepted
            .as_ref()
            .map(|snapshot| (snapshot.token, snapshot.document.revision));
        if self.scope() != previous_scope || accepted_snapshot != previous_snapshot {
            self.cancel_keycaps_cad_preview();
        }
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
                this.cancel_keycaps_cad_preview();
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
                #[cfg(test)]
                {
                    let use_test_core = self.project_name_test_core.borrow().is_some();
                    if use_test_core {
                        let reply = self
                            .project_name_test_core
                            .borrow_mut()
                            .as_mut()
                            .expect("test CoreEngine was checked above")
                            .handle(*request);
                        return self.complete(Completion::Core {
                            request_id,
                            executor_epoch,
                            reply: Box::new(reply),
                        });
                    }
                }
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
                #[cfg(test)]
                let test_behavior = self.project_name_persist_test_behavior.borrow_mut().take();
                #[cfg(test)]
                let test_failure = match test_behavior {
                    Some(ProjectNamePersistTestBehavior::Fail(reason)) => Some(reason),
                    Some(ProjectNamePersistTestBehavior::Gate(gate)) => {
                        let _ = gate.entered.send(());
                        let _ = gate.release.await;
                        None
                    }
                    None => None,
                };
                #[cfg(test)]
                let result = if let Some(reason) = test_failure {
                    SaveResult::Aborted(reason)
                } else {
                    match self.store.save_document(&document, &assets).await {
                        Ok(()) => {
                            for asset in &document.assets {
                                self.assets.borrow_mut().remove(&asset.sha256);
                            }
                            SaveResult::Committed
                        }
                        Err(error) => SaveResult::Aborted(error.to_string()),
                    }
                };
                #[cfg(not(test))]
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
                let is_keycaps_step_export =
                    self.keycaps_step_exports.borrow_mut().remove(&operation_id);
                let firmware_capture = self.firmware_exports.borrow_mut().remove(&operation_id);
                self.footprint_exports.borrow_mut().remove(&operation_id);
                self.pcb_handoff_exports.borrow_mut().remove(&operation_id);
                let delivery_error = self
                    .firmware_export_delivery_errors
                    .borrow_mut()
                    .remove(&operation_id);
                self.archive_export_options.settle(operation_id);
                let firmware_owner_is_current = firmware_capture.as_ref().is_some_and(|capture| {
                    self.firmware_export_capture_is_current(operation_id, capture)
                });
                if firmware_capture.is_some() {
                    if let Some(report) = firmware_export_terminal_report(
                        firmware_owner_is_current,
                        outcome,
                        delivery_error,
                    ) {
                        self.apply_report(report);
                    }
                } else if is_keycaps_step_export {
                    match outcome {
                        TerminalOutcome::Completed => {}
                        TerminalOutcome::Rejected(reason)
                        | TerminalOutcome::PersistenceFailed(reason)
                        | TerminalOutcome::BlockedByRecovery(reason)
                        | TerminalOutcome::ExecutorFailed(reason) => {
                            self.apply_report(RuntimeReport::alert(reason));
                        }
                        TerminalOutcome::Cancelled => self.report("Cancelled."),
                        TerminalOutcome::Closed => self.report("Editor closed."),
                        TerminalOutcome::Superseded => {
                            if observed {
                                self.changed();
                            }
                        }
                    }
                } else {
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
                let is_keycaps_step_export =
                    self.keycaps_step_exports.borrow().contains(&operation_id);
                let is_step_export =
                    self.step_exports.borrow().contains(&operation_id) || is_keycaps_step_export;
                let is_firmware_export = self.firmware_exports.borrow().contains_key(&operation_id);
                let is_footprint_export =
                    self.footprint_exports.borrow().contains_key(&operation_id);
                let is_pcb_handoff_export = self
                    .pcb_handoff_exports
                    .borrow()
                    .contains_key(&operation_id);
                let result = if is_pcb_handoff_export {
                    let draft = self
                        .pcb_handoff_exports
                        .borrow()
                        .get(&operation_id)
                        .is_some_and(|capture| capture.draft);
                    self.pcb_handoff_bytes(operation_id, &snapshot, &scope, draft)
                        .await
                } else if is_footprint_export {
                    self.footprint_export_bytes(operation_id, &snapshot, &scope)
                        .await
                } else {
                    let work: Result<ArchiveWorkFuture<'_>, String> =
                        self.archive_export_options.dispatch(
                            operation_id,
                            is_step_export,
                            is_firmware_export,
                            || -> ArchiveWorkFuture<'_> {
                                if is_keycaps_step_export {
                                    Box::pin(self.keycaps_step_bytes(
                                        operation_id,
                                        &snapshot,
                                        &scope,
                                    ))
                                } else {
                                    Box::pin(self.step_bytes(operation_id, &snapshot, &scope))
                                }
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
                    match work {
                        Ok(future) => future.await,
                        Err(reason) => Err(reason),
                    }
                };
                let result = if is_firmware_export {
                    let owner_is_current =
                        self.export_current(operation_id, snapshot.token, &scope);
                    let cancelled = self.cancelled_exports.borrow_mut().remove(&operation_id);
                    firmware_export_bytes_for_delivery(result, owner_is_current, cancelled)
                } else {
                    result
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
                        let (filename, media_type) = if is_footprint_export {
                            (
                                format!("{}-footprints.zip", snapshot.document.name),
                                Some("application/zip".to_owned()),
                            )
                        } else if is_firmware_export {
                            (
                                format!("{}-zmk.zip", snapshot.document.name),
                                Some("application/zip".to_owned()),
                            )
                        } else if is_keycaps_step_export {
                            (
                                format!("{}-keycaps.step", snapshot.document.name),
                                Some("model/step".to_owned()),
                            )
                        } else if is_step_export {
                            ("keyboard.step".to_owned(), None)
                        } else if is_pcb_handoff_export {
                            let draft = self
                                .pcb_handoff_exports
                                .borrow()
                                .get(&operation_id)
                                .is_some_and(|capture| capture.draft);
                            (
                                format!(
                                    "{}-{}pcb-handoff.zip",
                                    snapshot.document.name,
                                    if draft { "draft-" } else { "" }
                                ),
                                Some("application/zip".to_owned()),
                            )
                        } else {
                            (archive_filename(&snapshot.document.name), None)
                        };
                        self.artifacts.borrow_mut().insert(
                            artifact_id.clone(),
                            Artifact {
                                operation_id,
                                bytes,
                                filename,
                                media_type,
                                scope: scope.clone(),
                                token: snapshot.token,
                                firmware: is_firmware_export,
                                keycaps_step: is_keycaps_step_export,
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
                    && self.scope().as_ref() == Some(&artifact.scope)
                    && self
                        .model()
                        .accepted
                        .as_ref()
                        .is_some_and(|s| s.token == token)
                    && let Err(error) = self.deliver_artifact(&artifact)
                {
                    if artifact.firmware {
                        self.firmware_export_delivery_errors
                            .borrow_mut()
                            .insert(artifact.operation_id, error);
                    } else if artifact.keycaps_step {
                        self.apply_report(RuntimeReport::alert(error));
                    } else {
                        self.report(error);
                    }
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
                self.archive_export_options.cancel(operation_id);
                self.footprint_exports.borrow_mut().remove(&operation_id);
                self.pcb_handoff_exports.borrow_mut().remove(&operation_id);
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
        let current_scope = self.scope()?;
        let accepted = self.model().accepted?;
        let mut cached = self.cad_scene.borrow_mut();
        let scene = cached.as_ref()?;
        if scene.scope != current_scope {
            return None;
        }
        if scene.token != accepted.token
            && crate::case_generation_lifecycle::may_rebind_completed_case_result(
                scene.exact,
                &scene.scope,
                scene.token,
                scene.physical_fingerprint,
                &current_scope,
                accepted.token,
                crate::case_generation_lifecycle::physical_case_fingerprint(
                    &accepted,
                    &current_scope,
                ),
            )
        {
            let mut rebound = CadScene {
                scope: scene.scope.clone(),
                token: accepted.token,
                snapshot: accepted.clone(),
                result: scene.result.clone(),
                prepared: scene.prepared.clone(),
                physical_fingerprint: scene.physical_fingerprint,
                mechanical: scene.mechanical.clone(),
                exact: scene.exact,
                contours: scene.contours.clone(),
            };
            let revision = accepted.document.revision;
            rebound.result.revision = revision;
            rebound.prepared.revision = revision;
            for body in &mut rebound.prepared.bodies {
                body.revision = revision;
            }
            if let Some(mechanical) = &mut rebound.mechanical {
                mechanical.revision = revision;
                mechanical.case.revision = revision;
                for body in &mut mechanical.case.bodies {
                    body.revision = revision;
                }
            }
            *cached = Some(Rc::new(rebound));
        }
        cached.as_ref().cloned()
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

    pub(crate) fn layout_preview(
        &self,
    ) -> Option<Rc<crate::presentation::layout_viewer_source::LayoutPreviewSnapshot>> {
        self.layout_preview
            .borrow()
            .published
            .as_ref()
            .filter(|preview| self.layout_source_owner_is_current(&preview.owner))
            .cloned()
    }

    pub(crate) fn layout_preview_pending(&self) -> bool {
        self.layout_preview
            .borrow()
            .pending
            .as_ref()
            .is_some_and(|(owner, lease)| {
                lease.matches(owner) && self.layout_source_owner_is_current(owner)
            })
    }

    pub(crate) fn layout_preview_error(&self) -> Option<String> {
        let scope = self.scope()?;
        let accepted = self.model().accepted?;
        self.layout_preview
            .borrow()
            .error
            .as_ref()
            .filter(|(owner, _)| owner.matches_current(&accepted, &scope, owner.source_generation))
            .map(|(_, error)| error.clone())
    }

    pub(crate) fn layout_source_generation(&self) -> Option<u64> {
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

    pub(crate) fn layout_model_delivery(
        &self,
        preview: &crate::presentation::layout_viewer_source::LayoutPreviewSnapshot,
    ) -> Option<crate::presentation::model_delivery::ModelDeliveryRows> {
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

    pub(crate) fn retire_layout_source(&self, source_generation: u64) {
        let retired = self
            .layout_preview
            .borrow_mut()
            .retire_generation(source_generation);
        if retired {
            self.layout_model_rows.borrow_mut().take();
            self.changed();
        }
    }

    pub(crate) fn reconcile_layout_source_request(
        &self,
        expected: Option<(&Scope, SnapshotToken, u64)>,
    ) {
        let retired = self
            .layout_preview
            .borrow_mut()
            .retire_unless_request_matches(expected);
        if retired {
            self.layout_model_rows.borrow_mut().take();
            self.changed();
        }
    }

    fn layout_source_owner_is_current(
        &self,
        owner: &crate::presentation::layout_viewer_source::LayoutSourceIdentity,
    ) -> bool {
        let Some(scope) = self.scope() else {
            return false;
        };
        let Some(accepted) = self.model().accepted else {
            return false;
        };
        owner.matches_current(&accepted, &scope, owner.source_generation)
            && self.layout_preview.borrow().owns(owner)
    }

    async fn deliver_layout_models(
        self: &Rc<Self>,
        preview: Rc<crate::presentation::layout_viewer_source::LayoutPreviewSnapshot>,
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
        let batch_generation = self
            .layout_model_batch_generation
            .get()
            .checked_add(1)
            .ok_or_else(|| "Layout model batch identity exhausted".to_owned())?;
        self.layout_model_batch_generation.set(batch_generation);
        let owner = crate::presentation::model_delivery::ModelOwnerIdentity::new_layout(
            preview.owner.scope.clone(),
            preview.owner.snapshot_token,
            preview.owner.source_generation,
            &preview.lease,
        );
        let batch = crate::presentation::model_delivery::ModelBatchIdentity::new(
            owner,
            preview.owner.accepted_revision,
            batch_generation,
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
        let mut ergogen_ids_by_path = BTreeMap::new();
        if !unique_model_paths.is_empty() {
            let ids =
                crate::bundled_models::generated_model_asset_ids_for_paths(&unique_model_paths)
                    .await?;
            if !source_is_current() {
                return Ok(());
            }
            if ids.len() != unique_model_paths.len() {
                return Err("Model path resolver returned an incomplete Layout mapping".into());
            }
            ergogen_ids_by_path.extend(unique_model_paths.iter().cloned().zip(ids));
        }
        let selections = crate::presentation::model_delivery::resolve_preview_assets(
            &preview.preview.models,
            preview.board_reference.as_ref(),
            &native_paths,
            &preview.document,
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
        let ports = self.layout_model_delivery_ports(&preview, source_is_current.clone());
        let results = self
            .case_model_delivery
            .deliver_models(
                preview.preview.revision,
                &preview.preview.models,
                &selections,
                &ports,
                &batch,
                source_is_current.clone(),
            )
            .await;
        if let Some(rows) = results
            && source_is_current()
            && self.layout_source_owner_is_current(&preview.owner)
        {
            *self.layout_model_rows.borrow_mut() = Some((preview.owner.clone(), rows));
            self.changed();
        }
        Ok(())
    }

    pub(crate) async fn prepare_layout_preview(
        self: &Rc<Self>,
        expected_scope: Scope,
        expected_token: SnapshotToken,
        expected_revision: u64,
    ) -> Result<(), String> {
        let accepted = self
            .model()
            .accepted
            .ok_or_else(|| "No accepted project is available for Layout preview".to_owned())?;
        if accepted.token != expected_token
            || accepted.document.revision != expected_revision
            || self.scope().as_ref() != Some(&expected_scope)
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
                crate::presentation::layout_viewer_source::LayoutSourceIdentity::from_accepted(
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
        let capture = match crate::presentation::layout_viewer_source::LayoutSourceCapture::capture(
            &accepted,
            &expected_scope,
            source_generation,
            request_token,
            model_paths,
        ) {
            Ok(capture) => capture,
            Err(error) => {
                self.layout_preview.borrow_mut().fail_before_begin(
                    crate::presentation::layout_viewer_source::LayoutSourceIdentity::from_accepted(
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
            crate::presentation::layout_viewer_source::LayoutPreviewRequest::Authored(request) => {
                self.run_preview_pipeline(
                    request.as_ref().clone(),
                    operation,
                    capture.owner.scope.clone(),
                    capture.owner.source_generation,
                    is_current.clone(),
                )
                .await
            }
            crate::presentation::layout_viewer_source::LayoutPreviewRequest::Imported {
                asset,
                ..
            } => {
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
                        crate::presentation::model_delivery::settle_layout_model_delivery(
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

    async fn run_imported_layout_preview(
        &self,
        capture: &crate::presentation::layout_viewer_source::LayoutSourceCapture,
        asset: &boardstudio_core::model::Asset,
        operation: u64,
        is_current: Rc<dyn Fn() -> bool>,
    ) -> Result<PcbPreview, String> {
        let bytes = self
            .load_layout_document_asset(asset, is_current.clone())
            .await?;
        let source = String::from_utf8(bytes)
            .map_err(|error| format!("Imported board asset is not valid UTF-8: {error}"))?;
        let request_id = format!("layout-preview-{operation}-imported");
        let request = capture.artifact_request(request_id.clone(), Some(source))?;
        let core = self.core.borrow().clone();
        let core_epoch = self.session.borrow().core_executor_epoch().0;
        if !is_current() {
            return Err("Imported Layout preview source became stale before Core dispatch".into());
        }
        let epoch = core_epoch.to_string();
        let reply = core
            .artifact(&request_id, &epoch, &request)
            .await
            .map_err(|error| format!("Imported board preview failed: {error}"))?;
        if !is_current()
            || self.session.borrow().core_executor_epoch().0 != core_epoch
            || !Rc::ptr_eq(&core, &self.core.borrow().clone())
        {
            return Err("Imported Layout preview source changed during Core preview".into());
        }
        match reply {
            ArtifactReply::PreviewBoard { id, result } if id == request_id => Ok(result),
            ArtifactReply::Error { id, error } if id == request_id => Err(format!(
                "Core rejected the imported board preview: {error:?}"
            )),
            ArtifactReply::PreviewBoard { .. } | ArtifactReply::Error { .. } => {
                Err("Core returned an imported board preview for another request".into())
            }
            _ => Err("Core returned an unexpected imported-board preview reply".into()),
        }
    }

    async fn load_layout_document_asset(
        &self,
        asset: &boardstudio_core::model::Asset,
        is_current: Rc<dyn Fn() -> bool>,
    ) -> Result<Vec<u8>, String> {
        if asset.sha256.is_empty() || !is_current() {
            return Err("Imported Layout asset identity is missing or stale".into());
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
            return Err("Imported Layout asset request became stale after loading".into());
        }
        let digest = Sha256::digest(&bytes);
        let digest = digest
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        if digest != asset.sha256 {
            return Err("Imported Layout asset bytes do not match the accepted SHA-256".into());
        }
        self.assets
            .borrow_mut()
            .insert(asset.sha256.clone(), bytes.clone());
        Ok(bytes)
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

    async fn read_step_model(
        &self,
        bytes: Vec<u8>,
        scope: Scope,
        token: SnapshotToken,
        revision: u64,
        is_current: Rc<dyn Fn() -> bool>,
    ) -> Result<crate::presentation::model_delivery::MeshArrays, String> {
        if !is_current() || self.scope().as_ref() != Some(&scope) {
            return Err("STEP model request became stale before worker setup".into());
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
            return Err("STEP model request became stale after parsing".into());
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
        let scope = preview.owner.scope.clone();
        let token = preview.owner.snapshot_token;
        let viewer_instance = preview.owner.viewer_instance;
        let projection_generation = preview.owner.projection_generation;
        let lease = preview.lease.clone();
        let owner_is_current = Rc::new(
            move |owner: &crate::presentation::model_delivery::ModelOwnerIdentity| {
                owner.is_current_owner(
                    &scope,
                    token,
                    viewer_instance,
                    projection_generation,
                    &lease,
                )
            },
        );
        self.model_delivery_ports(
            preview.owner.scope.clone(),
            token,
            preview.owner.accepted_revision,
            is_current,
            owner_is_current,
        )
    }

    fn layout_model_delivery_ports(
        self: &Rc<Self>,
        preview: &crate::presentation::layout_viewer_source::LayoutPreviewSnapshot,
        is_current: Rc<dyn Fn() -> bool>,
    ) -> crate::presentation::model_delivery::ModelDeliveryPorts {
        let scope = preview.owner.scope.clone();
        let token = preview.owner.snapshot_token;
        let source_generation = preview.owner.source_generation;
        let lease = preview.lease.clone();
        let owner_is_current = Rc::new(
            move |owner: &crate::presentation::model_delivery::ModelOwnerIdentity| {
                owner.is_current_layout_owner(&scope, token, source_generation, &lease)
            },
        );
        self.model_delivery_ports(
            preview.owner.scope.clone(),
            token,
            preview.owner.accepted_revision,
            is_current,
            owner_is_current,
        )
    }

    fn model_delivery_ports(
        self: &Rc<Self>,
        scope: Scope,
        token: SnapshotToken,
        revision: u64,
        is_current: Rc<dyn Fn() -> bool>,
        owner_is_current: Rc<
            dyn Fn(&crate::presentation::model_delivery::ModelOwnerIdentity) -> bool,
        >,
    ) -> crate::presentation::model_delivery::ModelDeliveryPorts {
        use crate::presentation::model_delivery::{
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
                    if !is_current() || runtime.scope().as_ref() != Some(&scope) {
                        return Err("Model asset request became stale after loading".into());
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
        let owner_is_current = owner_is_current.clone();
        let read_step = Rc::new(
            move |bytes: VerifiedModelBytes,
                  owner: crate::presentation::model_delivery::ModelOwnerIdentity|
                  -> ModelFuture<MeshArrays> {
                let weak = weak.clone();
                let scope = scope.clone();
                let is_current = is_current.clone();
                let owner_is_current = owner_is_current.clone();
                Box::pin(async move {
                    if !owner_is_current(&owner) || !is_current() {
                        return Err("STEP model request became stale before reading".into());
                    }
                    let runtime = weak
                        .upgrade()
                        .ok_or_else(|| "Model runtime was closed".to_owned())?;
                    runtime
                        .read_step_model(bytes.bytes().to_vec(), scope, token, revision, is_current)
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
            .run_native_case_preview(capture.clone(), operation)
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

    /// Prepare a Parts-only disposable sample through the existing Core,
    /// generator worker, and model-delivery ports. `capture` is owned by the
    /// mounted Parts preview and becomes stale as soon as that selection or
    /// accepted project changes.
    pub(crate) async fn prepare_parts_library_preview(
        self: &Rc<Self>,
        capture: crate::parts_preview::PartsPreviewCapture,
    ) -> Result<crate::parts_preview::PartsPreviewSnapshot, String> {
        if !self.parts_preview_capture_is_current(&capture) {
            capture.lease.invalidate();
            return Err("Parts preview source changed before preparation".into());
        }
        let operation = self.operation().0;
        if operation == 0 || operation > 9_007_199_254_740_991 {
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
        let preview = self
            .run_preview_pipeline(
                capture.request.clone(),
                operation,
                capture.sample_scope.clone(),
                capture.owner.source_generation,
                is_current.clone(),
            )
            .await?;
        if !is_current() {
            capture.lease.invalidate();
            return Err("Parts preview became stale after geometry preparation".into());
        }

        let native_paths =
            crate::presentation::model_delivery::native_model_path_assets(&capture.path_assets);
        let unique_model_paths = preview
            .models
            .iter()
            .map(|model| model.path.clone())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();
        let mut ergogen_ids_by_path = BTreeMap::new();
        if !unique_model_paths.is_empty() {
            let ids =
                crate::bundled_models::generated_model_asset_ids_for_paths(&unique_model_paths)
                    .await?;
            if !is_current() {
                capture.lease.invalidate();
                return Err("Parts model resolution became stale".into());
            }
            if ids.len() != unique_model_paths.len() {
                capture.lease.invalidate();
                return Err("Model path resolver returned an incomplete Parts mapping".into());
            }
            ergogen_ids_by_path.extend(unique_model_paths.iter().cloned().zip(ids));
        }
        let selections = crate::presentation::model_delivery::resolve_preview_assets(
            &preview.models,
            None,
            &native_paths,
            &capture.sample_document,
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
        let owner = crate::presentation::model_delivery::ModelOwnerIdentity::new_parts(
            capture.owner.scope.clone(),
            capture.owner.snapshot_token,
            capture.owner.source_generation,
            &capture.lease,
        );
        let batch = crate::presentation::model_delivery::ModelBatchIdentity::new(
            owner.clone(),
            capture.owner.accepted_revision,
            capture.owner.source_generation,
        );
        let owner_scope = capture.owner.scope.clone();
        let owner_token = capture.owner.snapshot_token;
        let source_generation = capture.owner.source_generation;
        let lease = capture.lease.clone();
        let owner_is_current = Rc::new(
            move |candidate: &crate::presentation::model_delivery::ModelOwnerIdentity| {
                candidate.is_current_parts_owner(
                    &owner_scope,
                    owner_token,
                    source_generation,
                    &lease,
                )
            },
        );
        let ports = self.model_delivery_ports(
            capture.owner.scope.clone(),
            capture.owner.snapshot_token,
            capture.owner.accepted_revision,
            is_current.clone(),
            owner_is_current,
        );
        let model_rows = self
            .case_model_delivery
            .deliver_models(
                preview.revision,
                &preview.models,
                &selections,
                &ports,
                &batch,
                is_current.clone(),
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

    fn parts_preview_capture_is_current(
        &self,
        capture: &crate::parts_preview::PartsPreviewCapture,
    ) -> bool {
        let Some(scope) = self.scope() else {
            return false;
        };
        let Some(accepted) = self.model().accepted else {
            return false;
        };
        capture.lease.matches(&capture.owner)
            && capture.owner.scope == scope
            && capture.owner.snapshot_token == accepted.token
            && capture.owner.accepted_revision == accepted.document.revision
            && capture.owner.scope.session_epoch == accepted.session_epoch
            && capture.owner.scope.document_id == accepted.document.id
            && capture.owner.accepted_document_identity
                == std::sync::Arc::as_ptr(&accepted.document) as usize
    }

    pub(crate) fn parts_preview_snapshot_is_current(
        &self,
        preview: &crate::parts_preview::PartsPreviewSnapshot,
    ) -> bool {
        let Some(scope) = self.scope() else {
            return false;
        };
        let Some(accepted) = self.model().accepted else {
            return false;
        };
        preview.lease.matches(&preview.owner)
            && preview.owner.scope == scope
            && preview.owner.snapshot_token == accepted.token
            && preview.owner.accepted_revision == accepted.document.revision
            && preview.owner.scope.session_epoch == accepted.session_epoch
            && preview.owner.scope.document_id == accepted.document.id
            && preview.owner.accepted_document_identity
                == std::sync::Arc::as_ptr(&accepted.document) as usize
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

    fn preview_owner_lease_is_current(
        &self,
        owner: &crate::case_preview::CasePreviewOwnerIdentity,
    ) -> bool {
        self.native_case_preview.borrow().owns(owner)
    }

    async fn run_native_case_preview(
        self: &Rc<Self>,
        capture: crate::case_preview::NativePreviewCapture,
        operation: u64,
    ) -> Result<crate::case_preview::NativePreviewSnapshot, String> {
        let owner = capture.owner.clone();
        let lease = capture.lease.clone();
        let weak = Rc::downgrade(self);
        let preview = self
            .run_preview_pipeline(
                capture.request.clone(),
                operation,
                owner.scope.clone(),
                owner.projection_generation,
                Rc::new(move || {
                    weak.upgrade().is_some_and(|runtime| {
                        runtime.preview_owner_is_current(&owner)
                            && runtime.preview_owner_lease_is_current(&owner)
                            && lease.matches(&owner)
                    })
                }),
            )
            .await?;
        crate::case_preview::accept_native_preview(capture, preview)
    }

    /// One existing Core→preview-generator→Core path shared by canonical Layout
    /// and the physical Case producer. Source construction and liveness stay with
    /// each workflow; this method owns only the established preview pipeline.
    async fn run_preview_pipeline(
        &self,
        request: PrepareExportRequest,
        operation: u64,
        scope: Scope,
        source_generation: u64,
        is_current: Rc<dyn Fn() -> bool>,
    ) -> Result<PcbPreview, String> {
        if source_generation == 0 || source_generation > 9_007_199_254_740_991 {
            return Err("Preview source generation is outside the safe integer range".into());
        }
        if request.expected_revision != request.document.revision
            || request.target
                != (boardstudio_core::model::ExportTarget::Board {
                    board_id: scope.board_id.clone(),
                })
        {
            return Err("Preview request does not match its captured board source".into());
        }
        let core = self.core.borrow().clone();
        let core_epoch = self.session.borrow().core_executor_epoch().0;
        let ensure_current = || {
            if !is_current() {
                return Err("Accepted board preview source became stale".to_owned());
            }
            if self.session.borrow().core_executor_epoch().0 != core_epoch
                || !Rc::ptr_eq(&core, &self.core.borrow().clone())
            {
                return Err("Core worker changed during board preview generation".to_owned());
            }
            Ok(())
        };
        ensure_current()?;

        let epoch = core_epoch.to_string();
        let prepare_id = format!("board-preview-{operation}-prepare");
        let prepare = boardstudio_core::model::ArtifactRequest::PreparePreview {
            id: prepare_id.clone(),
            request: request.clone(),
        };
        let reply = core
            .artifact(&prepare_id, &epoch, &prepare)
            .await
            .map_err(|error| format!("Core preview preparation failed: {error}"))?;
        ensure_current()?;
        let plan = match reply {
            ArtifactReply::PreparePreview { id, result } if id == prepare_id => *result,
            ArtifactReply::Error { id, error } if id == prepare_id => {
                return Err(format!(
                    "Core rejected board preview preparation: {error:?}"
                ));
            }
            ArtifactReply::PreparePreview { .. } | ArtifactReply::Error { .. } => {
                return Err("Core returned a preview plan for another request".into());
            }
            _ => return Err("Core returned an unexpected preview preparation reply".into()),
        };
        if plan.snapshot_token != request.snapshot_token
            || plan.revision != request.expected_revision
            || plan.target != request.target
        {
            return Err("Core preview plan does not match the captured board source".into());
        }

        let worker_request_id = operation;
        let worker_request = serde_json::json!({
            "kind": "generate-preview-jobs",
            "worker_generation": source_generation,
            "request_id": worker_request_id,
            "owner": {
                "scope": {
                    "sessionEpoch": scope.session_epoch.0,
                    "documentId": scope.document_id,
                    "boardId": scope.board_id,
                    "instanceId": scope.instance_id,
                },
                "token": request.snapshot_token,
                "viewer_instance": 0,
                "projection_generation": source_generation,
            },
            "batch": {
                "accepted_revision": request.expected_revision,
                "batch_generation": source_generation,
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
        ensure_current()?;
        let worker = if let Some(worker) = self.preview_generator.borrow().as_ref() {
            worker.clone()
        } else {
            let url = resource_url("assets/preview-generator/worker.mjs")?;
            let worker = Rc::new(crate::preview_generator::PreviewGeneratorClient::new(&url)?);
            *self.preview_generator.borrow_mut() = Some(worker.clone());
            worker
        };
        let worker_reply = worker.generate(worker_request_id, &worker_request).await?;
        ensure_current()?;
        validate_preview_worker_envelope(
            &worker_reply,
            &worker_request,
            source_generation,
            worker_request_id,
        )?;
        let results = serde_json::from_value::<Vec<ErgogenJobResult>>(
            worker_reply
                .get("results")
                .cloned()
                .ok_or_else(|| "Preview worker returned no conversion results".to_owned())?,
        )
        .map_err(|error| format!("Preview worker returned malformed results: {error}"))?;
        let finish_id = format!("board-preview-{operation}-finish");
        let finish = boardstudio_core::model::ArtifactRequest::FinishPreview {
            id: finish_id.clone(),
            request: FinishExportRequest { plan, results },
        };
        let reply = core
            .artifact(&finish_id, &epoch, &finish)
            .await
            .map_err(|error| format!("Core preview finish failed: {error}"))?;
        ensure_current()?;
        let preview = match reply {
            ArtifactReply::PreviewBoard { id, result } if id == finish_id => result,
            ArtifactReply::Error { id, error } if id == finish_id => {
                return Err(format!(
                    "Core rejected the completed board preview: {error:?}"
                ));
            }
            ArtifactReply::PreviewBoard { .. } | ArtifactReply::Error { .. } => {
                return Err("Core returned a completed preview for another request".into());
            }
            _ => return Err("Core returned an unexpected completed-preview reply".into()),
        };
        if preview.revision != request.expected_revision {
            return Err("Core returned a board preview for another revision".into());
        }
        Ok(preview)
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
    pub(crate) fn export_keycaps_step(self: &Rc<Self>) {
        let Some(scope) = self.scope() else {
            self.apply_report(RuntimeReport::alert("Select a board before export"));
            return;
        };
        let model = self.model();
        let Some(snapshot) = model.accepted else {
            self.apply_report(RuntimeReport::alert("The project is still opening"));
            return;
        };
        if model.active_board_id != scope.board_id
            || !snapshot
                .document
                .boards
                .iter()
                .any(|board| board.id == scope.board_id)
        {
            self.apply_report(RuntimeReport::alert("Select a board before export"));
            return;
        }
        self.clear_alert();
        let operation_id = self.operation();
        self.keycaps_step_exports.borrow_mut().insert(operation_id);
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
        let model = self.model();
        let Some(snapshot) = model.accepted else {
            self.report("Firmware export requires a ready accepted snapshot.");
            return;
        };
        self.clear_alert();
        let operation_id = self.operation();
        let (_, executor_epoch) = self.current_firmware_executor();
        self.latest_firmware_export.set(Some(operation_id));
        self.firmware_exports.borrow_mut().insert(
            operation_id,
            FirmwareExportCapture {
                scope: scope.clone(),
                token: snapshot.token,
                revision: snapshot.document.revision,
                session_epoch: snapshot.session_epoch,
                document_id: snapshot.document.id.clone(),
                executor_epoch,
            },
        );
        self.submit(Event::StartExport {
            operation_id,
            scope,
        });
    }

    pub(crate) fn export_footprints(self: &Rc<Self>) {
        let Some(scope) = self.scope() else {
            self.apply_report(RuntimeReport::alert(
                "Open a project before exporting its footprint library.",
            ));
            return;
        };
        let model = self.model();
        let Some(snapshot) = model.accepted else {
            self.apply_report(RuntimeReport::alert(
                "Footprint export requires a ready accepted snapshot.",
            ));
            return;
        };
        if snapshot.document.definitions.is_empty() {
            self.apply_report(RuntimeReport::alert(
                "Add a component definition before exporting footprints.",
            ));
            return;
        }
        self.clear_alert();
        let operation_id = self.operation();
        let core = self.core.borrow().clone();
        let executor_epoch = self.session.borrow().core_executor_epoch();
        self.footprint_exports.borrow_mut().insert(
            operation_id,
            FootprintExportCapture {
                scope: scope.clone(),
                token: snapshot.token,
                revision: snapshot.document.revision,
                session_epoch: snapshot.session_epoch,
                document_id: snapshot.document.id.clone(),
                executor_epoch,
                core_worker_identity: Rc::as_ptr(&core) as usize,
            },
        );
        self.submit(Event::StartExport {
            operation_id,
            scope,
        });
    }

    pub(crate) fn export_kicad_board(self: &Rc<Self>, draft: bool) {
        let Some(scope) = self.scope() else {
            self.apply_report(RuntimeReport::alert("Select a board before KiCad export."));
            return;
        };
        let model = self.model();
        let Some(snapshot) = model.accepted else {
            self.apply_report(RuntimeReport::alert(
                "KiCad export requires a ready accepted snapshot.",
            ));
            return;
        };
        let board_exists = snapshot
            .document
            .boards
            .iter()
            .any(|board| board.id == scope.board_id);
        let pcb_ready = snapshot
            .scene
            .board_readiness
            .iter()
            .find(|readiness| readiness.board_id == scope.board_id)
            .map_or(snapshot.scene.readiness.pcb, |readiness| readiness.pcb);
        if model.active_board_id != scope.board_id || !board_exists || !pcb_ready {
            self.apply_report(RuntimeReport::alert(
                "Resolve PCB findings before exporting this board.",
            ));
            return;
        }
        self.clear_alert();
        let operation_id = self.operation();
        let core = self.core.borrow().clone();
        let capture = PcbHandoffCapture {
            scope: scope.clone(),
            token: snapshot.token,
            revision: snapshot.document.revision,
            session_epoch: snapshot.session_epoch,
            document_id: snapshot.document.id.clone(),
            executor_epoch: self.session.borrow().core_executor_epoch(),
            core_worker_identity: Rc::as_ptr(&core) as usize,
            draft,
        };
        self.pcb_handoff_exports
            .borrow_mut()
            .insert(operation_id, capture);
        self.submit(Event::StartExport {
            operation_id,
            scope,
        });
    }

    async fn pcb_handoff_bytes(
        self: &Rc<Self>,
        operation_id: OperationId,
        initial_snapshot: &AcceptedSnapshot,
        scope: &Scope,
        draft: bool,
    ) -> Result<Vec<u8>, String> {
        let mut capture = self
            .pcb_handoff_exports
            .borrow()
            .get(&operation_id)
            .cloned()
            .ok_or_else(|| "KiCad export owner was cancelled or superseded.".to_owned())?;
        if capture.draft != draft
            || capture.scope != *scope
            || capture.token != initial_snapshot.token
            || capture.revision != initial_snapshot.document.revision
        {
            return Err("KiCad export no longer matches its captured board.".into());
        }
        let core = self.core.borrow().clone();
        self.require_pcb_handoff_current(operation_id, &capture, &core)?;
        let mut snapshot = initial_snapshot.clone();
        let mut plan = self
            .resolve_pcb_handoff_plan(operation_id, &snapshot, scope, None, &core, &capture)
            .await?;
        let errors = plan
            .diagnostics
            .iter()
            .filter(|finding| finding.severity == "error")
            .map(|finding| finding.message.clone())
            .collect::<Vec<_>>();
        if !draft && !errors.is_empty() {
            return Err(errors.join("\n"));
        }
        if !pcb_wiring_is_applied(&snapshot.document, &plan) {
            capture = self
                .commit_pcb_handoff(
                    operation_id,
                    &capture,
                    boardstudio_application::ExportCommitRequest::ApplyElectrical {
                        plan: plan.clone(),
                        draft,
                    },
                )
                .await?;
            snapshot = self
                .model()
                .accepted
                .ok_or_else(|| "Accepted board snapshot disappeared after wiring.".to_owned())?;
            plan = self
                .resolve_pcb_handoff_plan(operation_id, &snapshot, scope, None, &core, &capture)
                .await?;
            let errors = plan
                .diagnostics
                .iter()
                .filter(|finding| finding.severity == "error")
                .map(|finding| finding.message.clone())
                .collect::<Vec<_>>();
            if !draft && !errors.is_empty() {
                return Err(errors.join("\n"));
            }
        }
        self.require_pcb_handoff_current(operation_id, &capture, &core)?;
        let mut populations = Vec::new();
        for instance in snapshot
            .document
            .hardware
            .as_ref()
            .into_iter()
            .flat_map(|hardware| hardware.instances.iter())
            .filter(|instance| instance.board_id == scope.board_id)
        {
            let population = self
                .resolve_pcb_handoff_plan(
                    operation_id,
                    &snapshot,
                    scope,
                    Some(&instance.id),
                    &core,
                    &capture,
                )
                .await?;
            if !draft
                && population
                    .diagnostics
                    .iter()
                    .any(|finding| finding.severity == "error")
            {
                return Err(format!(
                    "Resolve wiring findings for {} before export",
                    instance.name
                ));
            }
            populations.push((instance.name.clone(), population));
        }
        self.require_pcb_handoff_current(operation_id, &capture, &core)?;
        let generator = || {
            if let Some(worker) = self.preview_generator.borrow().as_ref() {
                return Ok(worker.clone());
            }
            let url = resource_url("assets/preview-generator/worker.mjs")?;
            let worker = Rc::new(
                crate::preview_generator::PreviewGeneratorClient::new(&url)
                    .map_err(|error| error.to_string())?,
            );
            *self.preview_generator.borrow_mut() = Some(worker.clone());
            Ok(worker)
        };
        let archive = {
            let runtime = self.clone();
            let is_current = || {
                if runtime.pcb_handoff_capture_is_current(operation_id, &capture, &core) {
                    Ok(())
                } else {
                    Err(
                        "KiCad export was cancelled, superseded, or its accepted source changed."
                            .into(),
                    )
                }
            };
            crate::pcb_handoff::build_handoff(
                crate::pcb_handoff::HandoffSource {
                    operation_id,
                    snapshot: &snapshot,
                    scope,
                    electrical_plan: plan.clone(),
                    populations,
                    draft,
                },
                crate::pcb_handoff::HandoffPorts {
                    core: &core,
                    store: &self.store,
                    executor_epoch: capture.executor_epoch.0,
                },
                is_current,
                generator,
            )
            .await?
        };
        if !self.pcb_handoff_capture_is_current(operation_id, &capture, &core) {
            return Err("KiCad export was superseded before wiring protection.".into());
        }
        capture = self
            .commit_pcb_handoff(
                operation_id,
                &capture,
                boardstudio_application::ExportCommitRequest::ProtectElectricalHandoff { plan },
            )
            .await?;
        if !self.pcb_handoff_capture_is_current(operation_id, &capture, &core) {
            return Err("KiCad handoff was superseded before delivery.".into());
        }
        Ok(archive)
    }

    async fn resolve_pcb_handoff_plan(
        &self,
        operation_id: OperationId,
        snapshot: &AcceptedSnapshot,
        scope: &Scope,
        instance_id: Option<&str>,
        core: &Rc<CoreWorker>,
        capture: &PcbHandoffCapture,
    ) -> Result<ElectricalPlan, String> {
        if !self.pcb_handoff_capture_is_current(operation_id, capture, core) {
            return Err("Accepted board changed before wiring resolution.".into());
        }
        let configuration = snapshot.document.hardware.as_ref().and_then(|hardware| {
            hardware
                .boards
                .iter()
                .find(|configuration| configuration.board_id == scope.board_id)
        });
        let controller_part_id = snapshot
            .document
            .hardware
            .as_ref()
            .and_then(|hardware| {
                hardware
                    .instances
                    .iter()
                    .find(|instance| Some(instance.id.as_str()) == instance_id)
            })
            .and_then(|instance| instance.controller_part_id.clone())
            .or_else(|| {
                configuration.and_then(|configuration| configuration.controller_part_id.clone())
            });
        let request_id = format!(
            "pcb-handoff-{}-wiring-{}",
            operation_id.0,
            instance_id.unwrap_or("board")
        );
        let request = CoreRequest::ResolveElectrical {
            id: request_id.clone(),
            request: ElectricalPlanRequest {
                document: (*snapshot.document).clone(),
                instance_id: instance_id.map(str::to_owned),
                mode: configuration
                    .map_or(ElectricalMode::Matrix, |configuration| configuration.mode),
                locks: configuration.map_or_else(Default::default, |configuration| {
                    configuration.locks.clone()
                }),
                controller_profile: None,
                board_id: Some(scope.board_id.clone()),
                controller_part_id,
            },
        };
        let reply = core
            .request(&request_id, &capture.executor_epoch.0.to_string(), &request)
            .await
            .map_err(|error| format!("Wiring resolution failed: {error}"))?;
        if !self.pcb_handoff_capture_is_current(operation_id, capture, core) {
            return Err("Accepted board changed during wiring resolution.".into());
        }
        let plan = match reply {
            CoreReply::ElectricalResolved { id, plan } if id == request_id => plan,
            CoreReply::Error { id, message, .. } if id == request_id => return Err(message),
            CoreReply::ElectricalResolved { .. } | CoreReply::Error { .. } => {
                return Err("Core returned wiring for another KiCad export.".into());
            }
            _ => return Err("Core returned an unexpected wiring reply.".into()),
        };
        if plan.revision != snapshot.document.revision
            || plan.board_id.as_deref() != Some(scope.board_id.as_str())
            || plan.instance_id.as_deref() != instance_id
        {
            return Err("Core resolved wiring for another board or revision.".into());
        }
        Ok(plan)
    }

    async fn commit_pcb_handoff(
        self: &Rc<Self>,
        export_operation_id: OperationId,
        capture: &PcbHandoffCapture,
        commit: boardstudio_application::ExportCommitRequest,
    ) -> Result<PcbHandoffCapture, String> {
        if !self.pcb_handoff_capture_is_current(
            export_operation_id,
            capture,
            &self.core.borrow().clone(),
        ) {
            return Err("KiCad export no longer owns the accepted wiring source.".into());
        }
        let operation_id = self.operation();
        let outcome = self.observe_operation(operation_id);
        self.submit(Event::ExportCommit {
            operation_id,
            export_operation_id,
            token: capture.token,
            scope: capture.scope.clone(),
            commit,
        });
        for _ in 0..1_200 {
            if let Some(outcome) = outcome.borrow_mut().take() {
                if outcome != TerminalOutcome::Completed {
                    return Err(format!(
                        "Could not accept export wiring change: {outcome:?}"
                    ));
                }
                let accepted = self.model().accepted.ok_or_else(|| {
                    "Accepted board snapshot disappeared after export wiring.".to_owned()
                })?;
                let mut updated = capture.clone();
                updated.token = accepted.token;
                updated.revision = accepted.document.revision;
                if accepted.session_epoch != updated.session_epoch
                    || accepted.document.id != updated.document_id
                    || accepted.scene.revision != updated.revision
                    || !self.export_current(export_operation_id, updated.token, &updated.scope)
                {
                    return Err("Export wiring changed the captured project or board.".into());
                }
                self.pcb_handoff_exports
                    .borrow_mut()
                    .insert(export_operation_id, updated.clone());
                return Ok(updated);
            }
            TimeoutFuture::new(25).await;
        }
        Err("Export wiring save did not complete.".into())
    }

    fn pcb_handoff_capture_is_current(
        &self,
        operation_id: OperationId,
        capture: &PcbHandoffCapture,
        core: &Rc<CoreWorker>,
    ) -> bool {
        let current_core = self.core.borrow().clone();
        let accepted = self.model().accepted;
        self.pcb_handoff_exports.borrow().get(&operation_id) == Some(capture)
            && self.export_current(operation_id, capture.token, &capture.scope)
            && self.scope().as_ref() == Some(&capture.scope)
            && accepted.as_ref().is_some_and(|snapshot| {
                snapshot.token == capture.token
                    && snapshot.document.id == capture.document_id
                    && snapshot.document.revision == capture.revision
                    && snapshot.scene.revision == capture.revision
                    && snapshot.session_epoch == capture.session_epoch
            })
            && self.session.borrow().core_executor_epoch() == capture.executor_epoch
            && Rc::as_ptr(&current_core) as usize == capture.core_worker_identity
            && Rc::ptr_eq(core, &current_core)
    }

    fn require_pcb_handoff_current(
        &self,
        operation_id: OperationId,
        capture: &PcbHandoffCapture,
        core: &Rc<CoreWorker>,
    ) -> Result<(), String> {
        if self.pcb_handoff_capture_is_current(operation_id, capture, core) {
            Ok(())
        } else {
            Err("KiCad export was cancelled, superseded, or its accepted source changed.".into())
        }
    }

    async fn footprint_export_bytes(
        self: &Rc<Self>,
        operation_id: OperationId,
        snapshot: &AcceptedSnapshot,
        scope: &Scope,
    ) -> Result<Vec<u8>, String> {
        let capture = self
            .footprint_exports
            .borrow()
            .get(&operation_id)
            .cloned()
            .ok_or_else(|| "Footprint export owner was cancelled or superseded.".to_owned())?;
        if capture.scope != *scope
            || capture.token != snapshot.token
            || capture.revision != snapshot.document.revision
            || capture.session_epoch != snapshot.session_epoch
            || capture.document_id != snapshot.document.id
        {
            return Err("Footprint export no longer matches its captured project.".into());
        }
        let core = self.core.borrow().clone();
        let runtime = self.clone();
        let ensure_current = || {
            if runtime.footprint_export_capture_is_current(operation_id, &capture, &core) {
                Ok(())
            } else {
                Err("Footprint export was cancelled, superseded, or its source changed.".into())
            }
        };
        let preview_generator = || {
            if let Some(worker) = runtime.preview_generator.borrow().as_ref() {
                return Ok(worker.clone());
            }
            let url = resource_url("assets/preview-generator/worker.mjs")?;
            let worker = Rc::new(
                crate::preview_generator::PreviewGeneratorClient::new(&url)
                    .map_err(|error| error.to_string())?,
            );
            *runtime.preview_generator.borrow_mut() = Some(worker.clone());
            Ok(worker)
        };
        crate::export_footprints::build_zip(
            crate::export_footprints::ExportSource {
                operation_id,
                snapshot,
                scope,
                core: &core,
                store: &self.store,
                executor_epoch: capture.executor_epoch.0,
            },
            ensure_current,
            preview_generator,
        )
        .await
    }

    fn footprint_export_capture_is_current(
        &self,
        operation_id: OperationId,
        capture: &FootprintExportCapture,
        core: &Rc<CoreWorker>,
    ) -> bool {
        let current_core = self.core.borrow().clone();
        let accepted = self.model().accepted;
        self.footprint_exports.borrow().get(&operation_id) == Some(capture)
            && self.export_current(operation_id, capture.token, &capture.scope)
            && self.scope().as_ref() == Some(&capture.scope)
            && accepted.as_ref().is_some_and(|snapshot| {
                snapshot.token == capture.token
                    && snapshot.document.id == capture.document_id
                    && snapshot.document.revision == capture.revision
                    && snapshot.scene.revision == capture.revision
                    && snapshot.session_epoch == capture.session_epoch
            })
            && self.session.borrow().core_executor_epoch() == capture.executor_epoch
            && Rc::as_ptr(&current_core) as usize == capture.core_worker_identity
            && Rc::ptr_eq(core, &current_core)
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

    pub(crate) fn export_board_outline(
        self: &Rc<Self>,
        format: boardstudio_core::model::OutlineExportFormat,
    ) {
        let Some(scope) = self.scope() else {
            self.apply_report(RuntimeReport::alert(
                "Select a board before outline export.",
            ));
            return;
        };
        let Some(snapshot) = self.model().accepted else {
            self.apply_report(RuntimeReport::alert(
                "Outline export requires a ready accepted snapshot.",
            ));
            return;
        };
        let Some(board) = snapshot
            .document
            .boards
            .iter()
            .find(|board| board.id == scope.board_id)
            .cloned()
        else {
            self.apply_report(RuntimeReport::alert(
                "Select a resolved board before outline export.",
            ));
            return;
        };
        let Some(contours) = snapshot
            .scene
            .board_contours
            .iter()
            .find(|entry| entry.board_id == scope.board_id)
            .map(|entry| entry.contours.clone())
        else {
            self.apply_report(RuntimeReport::alert(
                "The selected board has no resolved outline.",
            ));
            return;
        };
        let runtime = self.clone();
        let operation_id = self.operation();
        spawn_local(async move {
            let core = runtime.core.borrow().clone();
            let executor_epoch = runtime.session.borrow().core_executor_epoch();
            let request_id = format!("export-outline-{}", operation_id.0);
            let format_name = match format {
                boardstudio_core::model::OutlineExportFormat::Svg => "svg",
                boardstudio_core::model::OutlineExportFormat::Dxf => "dxf",
            };
            let request = ArtifactRequest::ExportOutline {
                id: request_id.clone(),
                request: boardstudio_core::model::OutlineExportRequest {
                    filename: format!("{}.{}", snapshot.document.name, format_name),
                    board,
                    contours,
                    format,
                },
            };
            let result = core
                .artifact(&request_id, &executor_epoch.0.to_string(), &request)
                .await
                .map_err(|error| format!("Outline export failed: {error}"))
                .and_then(|reply| match reply {
                    ArtifactReply::ExportOutline { id, result } if id == request_id => Ok(result),
                    ArtifactReply::Error { id, error } if id == request_id => {
                        Err(format!("Outline export failed: {}", error.message))
                    }
                    _ => Err("Core returned an outline for another export request.".into()),
                });
            let current = runtime.scope().as_ref() == Some(&scope)
                && runtime.model().accepted.as_ref().is_some_and(|current| {
                    current.token == snapshot.token
                        && current.session_epoch == snapshot.session_epoch
                        && current.document.id == snapshot.document.id
                        && current.document.revision == snapshot.document.revision
                })
                && runtime.session.borrow().core_executor_epoch() == executor_epoch
                && Rc::ptr_eq(&core, &*runtime.core.borrow());
            if !current {
                return;
            }
            match result {
                Ok(file) => {
                    let media_type = match format {
                        boardstudio_core::model::OutlineExportFormat::Svg => "image/svg+xml",
                        boardstudio_core::model::OutlineExportFormat::Dxf => "application/dxf",
                    };
                    let delivery =
                        deliver(file.content.as_bytes(), &file.filename, Some(media_type));
                    match delivery {
                        Ok(()) => runtime.report(format!("Saved {}.", file.filename)),
                        Err(error) => runtime.apply_report(RuntimeReport::alert(error)),
                    }
                }
                Err(error) => runtime.apply_report(RuntimeReport::alert(error)),
            }
        });
    }
    fn export_current(
        &self,
        operation_id: OperationId,
        token: SnapshotToken,
        scope: &Scope,
    ) -> bool {
        #[cfg(test)]
        if let Some(context) = self.firmware_export_test_context.borrow().as_ref() {
            return context.scope.as_ref() == Some(scope)
                && context.accepted.token == token
                && self
                    .session
                    .borrow()
                    .export_is_current(operation_id, token, scope)
                && !self.cancelled_exports.borrow().contains(&operation_id);
        }
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
    fn firmware_export_capture_is_current(
        &self,
        operation_id: OperationId,
        capture: &FirmwareExportCapture,
    ) -> bool {
        let model = self.model();
        let accepted = model.accepted.as_ref().map(FirmwareAcceptedIdentity::from);
        firmware_export_is_latest(operation_id, self.latest_firmware_export.get())
            && self.current_firmware_executor().1 == capture.executor_epoch
            && firmware_export_capture_matches(capture, self.scope().as_ref(), accepted.as_ref())
    }
    fn current_firmware_executor(
        &self,
    ) -> (
        Rc<dyn FirmwareExportExecutor>,
        boardstudio_application::ExecutorEpoch,
    ) {
        #[cfg(test)]
        if let Some(context) = self.firmware_export_test_context.borrow().as_ref() {
            return (context.current_executor.clone(), context.executor_epoch);
        }
        let core = self.core.borrow().clone();
        let executor: Rc<dyn FirmwareExportExecutor> = core;
        (executor, self.session.borrow().core_executor_epoch())
    }
    fn deliver_artifact(&self, artifact: &Artifact) -> Result<(), String> {
        #[cfg(test)]
        if self.firmware_export_test_context.borrow().is_some() && artifact.firmware {
            self.firmware_export_test_deliveries
                .borrow_mut()
                .push(FirmwareTestDelivery {
                    bytes: artifact.bytes.clone(),
                    filename: artifact.filename.clone(),
                    media_type: artifact.media_type.clone(),
                });
            return Ok(());
        }
        deliver(
            &artifact.bytes,
            &artifact.filename,
            artifact.media_type.as_deref(),
        )
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
                physical_fingerprint: crate::case_generation_lifecycle::physical_case_fingerprint(
                    snapshot, scope,
                ),
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

    async fn keycaps_step_bytes(
        &self,
        operation_id: OperationId,
        snapshot: &AcceptedSnapshot,
        scope: &Scope,
    ) -> Result<Vec<u8>, String> {
        let guard = || {
            if !self.export_current(operation_id, snapshot.token, scope) {
                return Err("Keycap STEP export was cancelled or superseded.".to_owned());
            }
            self.ensure_keycaps_source_current(snapshot, scope)
        };
        guard()?;
        let core = self.core.borrow().clone();
        let executor_epoch = self.session.borrow().core_executor_epoch();
        let request_id = format!("keycaps-step-resolve-{}", operation_id.0);
        let request = CoreRequest::ResolveKeycaps {
            id: request_id.clone(),
            document: (*snapshot.document).clone(),
            board_id: scope.board_id.clone(),
            cases: None,
        };
        let reply = core
            .request(&request_id, &executor_epoch.0.to_string(), &request)
            .await
            .map_err(|error| format!("Keycap STEP resolution failed: {error}"))?;
        guard()?;
        if self.session.borrow().core_executor_epoch() != executor_epoch
            || !Rc::ptr_eq(&core, &self.core.borrow())
        {
            return Err("The Core worker changed during keycap STEP resolution.".into());
        }
        let resolution = match reply {
            CoreReply::KeycapsResolved { id, result } if id == request_id => result,
            CoreReply::Error { id, message, .. } if id == request_id => return Err(message),
            CoreReply::KeycapsResolved { .. } | CoreReply::Error { .. } => {
                return Err("Core returned a stale keycap STEP resolution reply.".into());
            }
            _ => return Err("Core returned an unexpected keycap STEP resolution reply.".into()),
        };
        if resolution.revision != snapshot.document.revision {
            return Err("Core resolved keycaps for another document revision.".into());
        }
        let errors = resolution
            .findings
            .iter()
            .filter(|finding| finding.severity == boardstudio_core::model::Severity::Error)
            .map(|finding| finding.message.as_str())
            .collect::<Vec<_>>();
        if !errors.is_empty() {
            return Err(errors.join("\n"));
        }
        if resolution.specs.is_empty() {
            return Err("Choose a keycap profile in Keymap before export".into());
        }
        guard()?;
        let identity = CadSnapshotIdentity {
            token: snapshot.token.0,
            session_epoch: snapshot.session_epoch.0,
            document_id: snapshot.document.id.clone(),
            board_id: scope.board_id.clone(),
            instance_id: scope.instance_id.clone(),
            revision: snapshot.document.revision,
        };
        let worker = Rc::new(
            CadWorker::new(&resource_url("assets/cad-worker/entry.js")?)
                .map_err(|error| error.to_string())?,
        );
        self.export_workers
            .borrow_mut()
            .insert(operation_id, worker.clone());
        let request_id = format!("keycaps-step-cad-{}", operation_id.0);
        let result = async {
            worker.ready().await.map_err(|error| error.to_string())?;
            guard()?;
            let reply = worker
                .request_keycaps_step(
                    request_id.clone(),
                    format!("keycaps-step-{}", operation_id.0),
                    identity.clone(),
                    resolution.specs,
                )
                .await
                .map_err(|error| error.to_string())?;
            guard()?;
            let request = CadRequest {
                request_id,
                job_id: format!("keycaps-step-{}", operation_id.0),
                identity: identity.clone(),
                operation: CadOperation::ExportStep,
                prepared: None,
                input_bytes: vec![],
            };
            validate_reply(&request, reply, &identity)
                .map(|result| result.step)
                .map_err(|error| format!("{error:?}"))
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
        let (core, executor_epoch) = self.current_firmware_executor();
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
        let (packed_metadata, packed_buffers) =
            packed.map_err(|error| format!("Firmware packaging failed: {error}"))?;
        match serde_json::from_str::<ArchiveReply>(&packed_metadata)
            .map_err(|error| format!("Could not read firmware package result: {error}"))?
        {
            ArchiveReply::Packed => packed_buffers
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
        core: &Rc<dyn FirmwareExportExecutor>,
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
        core: Option<&Rc<dyn FirmwareExportExecutor>>,
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
        if let (Some(expected_core), Some(expected_epoch)) = (core, executor_epoch) {
            let (current_core, current_epoch) = self.current_firmware_executor();
            if !firmware_export_worker_is_current(
                expected_epoch,
                current_epoch,
                Rc::ptr_eq(expected_core, &current_core),
            ) {
                return Err("The Core worker changed during firmware export.".into());
            }
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
    pub(crate) async fn fixture_document(&self, name: &'static str) -> Result<ProjectDoc, String> {
        let bytes = fetch_bytes(&format!("assets/fixtures/{name}.json")).await?;
        serde_json::from_slice(&bytes)
            .map_err(|error| format!("Could not read the {name} demo preview: {error}"))
    }
    pub fn open_fixture(self: &Rc<Self>, name: &str) {
        let name = name.to_owned();
        let sequence = match self.begin_open() {
            Ok(sequence) => sequence,
            Err(error) => {
                self.report(error);
                return;
            }
        };
        let copy_id = match browser_uuid() {
            Ok(copy_id) => copy_id,
            Err(error) => {
                if self.open_sequence.get() == sequence {
                    self.report(error);
                }
                return;
            }
        };
        let this = self.clone();
        spawn_local(async move {
            match fetch_bytes(&format!("assets/fixtures/{name}.boardstudio")).await {
                Ok(bytes) => {
                    if let Err(error) = this.import_archive_at(bytes, sequence, Some(copy_id)).await
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
        let sequence = match self.begin_open() {
            Ok(sequence) => sequence,
            Err(error) => {
                self.report(error);
                return;
            }
        };
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
                this.report(error);
            }
        });
    }
    async fn import_archive_at(
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
        self.assets.borrow_mut().extend(copied);
        self.submit(Event::Open {
            operation_id: operation,
            document,
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

    /// Reopen a durable project after an owned variant operation failed, but only while the
    /// session/document that requested recovery is still active. The saved copy is confirmed
    /// through the same Session open/recovery outcome used by the project library.
    pub(crate) async fn reopen_saved_if_current(
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
            .load_document(id.clone())
            .await
            .map_err(|error| format!("Could not load the saved original project: {error}"))?
            .ok_or_else(|| "The saved original project is no longer available.".to_owned())?;
        if self.open_sequence.get() != sequence || !still_expected(self) {
            return Err("A newer project open superseded recovery of the original project.".into());
        }
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

pub(crate) fn validate_preview_worker_envelope(
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
        if let Some((_, worker)) = self.keycaps_preview_worker.get_mut().take() {
            worker.close();
        }
        if let Some((_, worker)) = self.cad_worker.borrow_mut().take() {
            worker.close();
        }
        for (_, worker) in std::mem::take(&mut *self.export_workers.borrow_mut()) {
            worker.close();
        }
        self.core.borrow().close();
    }
}

#[cfg(test)]
pub(crate) mod firmware_export_test_support {
    use super::*;
    use boardstudio_application::{Completion, Effect, Event, SaveResult, Session};
    use boardstudio_core::{CoreEngine, firmware::FirmwarePackage, model::Board};
    use futures_channel::oneshot;

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub(crate) enum Stage {
        Resolution,
        Generation,
        Packaging,
    }

    impl Stage {
        fn label(self) -> &'static str {
            match self {
                Self::Resolution => "resolution",
                Self::Generation => "generation",
                Self::Packaging => "packaging",
            }
        }
    }

    struct Gate {
        stage: Stage,
        entered: oneshot::Sender<&'static str>,
        release: oneshot::Receiver<()>,
    }

    pub(crate) struct ControlledExecutor {
        fail_at: Option<Stage>,
        gate: RefCell<Option<Gate>>,
    }

    impl ControlledExecutor {
        pub(crate) fn succeeding() -> Rc<Self> {
            Rc::new(Self {
                fail_at: None,
                gate: RefCell::new(None),
            })
        }

        pub(crate) fn failing(stage: Stage) -> Rc<Self> {
            Rc::new(Self {
                fail_at: Some(stage),
                gate: RefCell::new(None),
            })
        }

        pub(crate) fn gated(
            stage: Stage,
        ) -> (
            Rc<Self>,
            oneshot::Receiver<&'static str>,
            oneshot::Sender<()>,
        ) {
            let (entered_tx, entered_rx) = oneshot::channel();
            let (release_tx, release_rx) = oneshot::channel();
            let executor = Rc::new(Self {
                fail_at: None,
                gate: RefCell::new(Some(Gate {
                    stage,
                    entered: entered_tx,
                    release: release_rx,
                })),
            });
            (executor, entered_rx, release_tx)
        }

        fn gate_for(&self, stage: Stage) -> Option<Gate> {
            let mut gate = self.gate.borrow_mut();
            (gate.as_ref().is_some_and(|gate| gate.stage == stage))
                .then(|| gate.take())
                .flatten()
        }

        async fn wait_at(&self, stage: Stage) {
            if let Some(gate) = self.gate_for(stage) {
                let _ = gate.entered.send(stage.label());
                let _ = gate.release.await;
            }
        }
    }

    impl FirmwareExportExecutor for ControlledExecutor {
        fn request<'a>(
            &'a self,
            request_id: &'a str,
            _executor_epoch: &'a str,
            request: &'a CoreRequest,
        ) -> FirmwareExecutorFuture<'a, CoreReply> {
            let request_id = request_id.to_owned();
            let response = match request {
                CoreRequest::ResolveElectrical { request, .. } => {
                    if self.fail_at == Some(Stage::Resolution) {
                        Err("injected electrical-resolution failure".to_owned())
                    } else {
                        Ok(CoreReply::ElectricalResolved {
                            id: request_id.clone(),
                            plan: resolved_plan(request),
                        })
                    }
                }
                CoreRequest::GenerateFirmware { .. } => {
                    if self.fail_at == Some(Stage::Generation) {
                        Err("injected firmware-generation failure".to_owned())
                    } else {
                        Ok(CoreReply::FirmwareGenerated {
                            id: request_id.clone(),
                            package: FirmwarePackage {
                                files: BTreeMap::from([(
                                    "config/boards/test.keymap".to_owned(),
                                    "// generated test keymap".to_owned(),
                                )]),
                                warnings: Vec::new(),
                            },
                        })
                    }
                }
                _ => Err("unexpected Core request in firmware test".to_owned()),
            };
            let stage = match request {
                CoreRequest::ResolveElectrical { .. } => Stage::Resolution,
                CoreRequest::GenerateFirmware { .. } => Stage::Generation,
                _ => unreachable!("unexpected requests returned an error above"),
            };
            Box::pin(async move {
                self.wait_at(stage).await;
                response
            })
        }

        fn archive<'a>(
            &'a self,
            _request_id: &'a str,
            _executor_epoch: &'a str,
            _metadata: &'a str,
            _buffers: Vec<Uint8Array>,
        ) -> FirmwareExecutorFuture<'a, (String, Vec<Uint8Array>)> {
            Box::pin(async move {
                self.wait_at(Stage::Packaging).await;
                if self.fail_at == Some(Stage::Packaging) {
                    return Err("injected ZIP-packaging failure".into());
                }
                Ok((
                    serde_json::to_string(&ArchiveReply::Packed)
                        .map_err(|error| error.to_string())?,
                    vec![Uint8Array::from(&[0x50, 0x4b, 0x03, 0x04][..])],
                ))
            })
        }
    }

    fn resolved_plan(request: &ElectricalPlanRequest) -> ElectricalPlan {
        let board_id = request.board_id.clone().unwrap_or_default();
        ElectricalPlan {
            instance_id: request.instance_id.clone(),
            jumpers: Vec::new(),
            module_aliases: Default::default(),
            mode: request.mode,
            assignments: vec![boardstudio_core::electrical::ElectricalAssignment {
                key_id: "key-1".into(),
                matrix_id: "matrix-1".into(),
                row: 0,
                column: 0,
                row_pin: "R0".into(),
                column_pin: "C0".into(),
                locked: false,
                row_firmware_gpio: None,
                column_firmware_gpio: None,
                direct_gpio: None,
            }],
            row_pins: vec!["R0".into()],
            column_pins: vec!["C0".into()],
            diagnostics: Vec::new(),
            fingerprint: "controlled-firmware-plan".into(),
            board_id: Some(board_id),
            controller_part_id: Some("controller-1".into()),
            revision: request.document.revision,
            controller_profile: Some("test-controller".into()),
            free_pins: Vec::new(),
            nets: Vec::new(),
            diode_direction: "column2row".into(),
            peripherals: Vec::new(),
            peripheral_pins: Default::default(),
            peripheral_terminals: Default::default(),
        }
    }

    pub(crate) fn opened_session() -> (Session, AcceptedSnapshot, Scope) {
        let mut session = Session::new();
        let mut core = CoreEngine::new();
        let mut document = ProjectDoc::empty("zmk-export-test", "ZMK export test");
        document.boards.push(Board {
            id: "main-board".into(),
            name: "Main board".into(),
            outline_ids: Vec::new(),
            part_ids: Vec::new(),
            net_ids: Vec::new(),
            thickness: 1.6,
            traces: Vec::new(),
            vias: Vec::new(),
        });
        let mut effects = session.submit(Event::Open {
            operation_id: OperationId(1),
            document,
        });
        while let Some(effect) = effects.pop() {
            match effect {
                Effect::Core {
                    request_id,
                    executor_epoch,
                    request,
                    ..
                } => effects.extend(session.complete(Completion::Core {
                    request_id,
                    executor_epoch,
                    reply: Box::new(core.handle(*request)),
                })),
                Effect::Persist {
                    save_attempt_id, ..
                } => {
                    effects.extend(session.complete(Completion::Persist {
                        save_attempt_id,
                        result: SaveResult::Committed,
                    }));
                }
                _ => {}
            }
        }
        let accepted = session.read_model().accepted.clone().unwrap();
        let scope = session.scope().unwrap();
        (session, accepted, scope)
    }

    pub(crate) fn configure_runtime(
        runtime: &Runtime,
        session: Session,
        accepted: AcceptedSnapshot,
        scope: Scope,
        executor: Rc<dyn FirmwareExportExecutor>,
        epoch: boardstudio_application::ExecutorEpoch,
        next_operation: u64,
    ) {
        *runtime.session.borrow_mut() = session;
        runtime.set_firmware_export_test_context(
            accepted,
            Some(scope),
            executor,
            epoch,
            next_operation,
        );
    }

    pub(crate) fn new_runtime() -> Rc<Runtime> {
        Runtime::new().expect("browser runtime fixture initializes")
    }

    pub(crate) async fn run_effects(runtime: &Rc<Runtime>, initial: Vec<Effect>) {
        let mut effects = VecDeque::from(initial);
        while let Some(effect) = effects.pop_front() {
            effects.extend(runtime.run(effect).await);
        }
    }

    pub(crate) fn start_export(runtime: &Rc<Runtime>) -> Vec<Effect> {
        runtime.export_firmware();
        runtime.take_firmware_export_test_effects()
    }

    pub(crate) fn replace_executor(
        runtime: &Runtime,
        executor: Rc<ControlledExecutor>,
        epoch: boardstudio_application::ExecutorEpoch,
    ) {
        runtime.replace_firmware_export_test_executor(executor, epoch);
    }

    pub(crate) fn replace_owner(
        runtime: &Runtime,
        accepted: AcceptedSnapshot,
        scope: Option<Scope>,
    ) {
        runtime.replace_firmware_export_test_owner(accepted, scope);
    }

    pub(crate) fn take_deliveries(runtime: &Runtime) -> Vec<FirmwareTestDelivery> {
        runtime.take_firmware_export_test_deliveries()
    }

    pub(crate) fn take_events(runtime: &Runtime) -> Vec<Event> {
        runtime.take_firmware_export_test_events()
    }

    pub(crate) fn take_effects(runtime: &Runtime) -> Vec<Effect> {
        runtime.take_firmware_export_test_effects()
    }

    pub(crate) fn session_model(runtime: &Runtime) -> ReadModel {
        runtime.session.borrow().read_model().clone()
    }

    pub(crate) fn has_artifacts(runtime: &Runtime) -> bool {
        !runtime.artifacts.borrow().is_empty()
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

#[cfg(test)]
pub(crate) mod project_name_test_support {
    use super::*;

    pub(crate) fn new_runtime() -> Rc<Runtime> {
        Runtime::new_with_restoration(false).expect("browser runtime fixture initializes")
    }

    pub(crate) fn install(runtime: &Runtime, session: Session, core: boardstudio_core::CoreEngine) {
        *runtime.session.borrow_mut() = session;
        *runtime.project_name_test_core.borrow_mut() = Some(core);
        runtime.changed();
    }

    pub(crate) fn replace_session(runtime: &Runtime, session: Session) {
        *runtime.session.borrow_mut() = session;
        runtime.changed();
    }

    pub(crate) fn fail_next_persist(runtime: &Runtime, reason: impl Into<String>) {
        *runtime.project_name_persist_test_behavior.borrow_mut() =
            Some(ProjectNamePersistTestBehavior::Fail(reason.into()));
    }

    pub(crate) fn gate_next_persist(
        runtime: &Runtime,
    ) -> (
        futures_channel::oneshot::Receiver<()>,
        futures_channel::oneshot::Sender<()>,
    ) {
        let (entered_tx, entered_rx) = futures_channel::oneshot::channel();
        let (release_tx, release_rx) = futures_channel::oneshot::channel();
        *runtime.project_name_persist_test_behavior.borrow_mut() = Some(
            ProjectNamePersistTestBehavior::Gate(ProjectNamePersistGate {
                entered: entered_tx,
                release: release_rx,
            }),
        );
        (entered_rx, release_tx)
    }

    pub(crate) fn observe(
        runtime: &Runtime,
        operation: OperationId,
    ) -> crate::operation_outcomes::OutcomeSlot {
        runtime.operation_outcomes.observe(operation)
    }

    pub(crate) fn observe_next(runtime: &Runtime) -> crate::operation_outcomes::OutcomeSlot {
        runtime
            .operation_outcomes
            .observe(OperationId(runtime.next_operation.get()))
    }

    pub(crate) async fn run_pending(runtime: &Rc<Runtime>) {
        let mut pending = VecDeque::from(std::mem::take(
            &mut *runtime.project_name_test_effects.borrow_mut(),
        ));
        while let Some(effect) = pending.pop_front() {
            pending.extend(runtime.run(effect).await);
        }
    }

    pub(crate) fn drive_pending(runtime: &Rc<Runtime>) {
        let effects = std::mem::take(&mut *runtime.project_name_test_effects.borrow_mut());
        runtime.drive(effects);
    }
}

#[cfg(test)]
mod firmware_export_tests {
    use super::*;
    use crate::runtime::firmware_export_test_support as test_support;
    use boardstudio_application::{ExecutorEpoch, SessionEpoch};
    use futures_channel::oneshot;
    use std::task::Poll;
    use wasm_bindgen_test::wasm_bindgen_test;

    fn capture() -> FirmwareExportCapture {
        FirmwareExportCapture {
            scope: Scope {
                session_epoch: SessionEpoch(5),
                document_id: "project-a".into(),
                board_id: "board-a".into(),
                instance_id: None,
            },
            token: SnapshotToken(17),
            revision: 6,
            session_epoch: SessionEpoch(5),
            document_id: "project-a".into(),
            executor_epoch: ExecutorEpoch(11),
        }
    }

    fn accepted(capture: &FirmwareExportCapture) -> FirmwareAcceptedIdentity {
        FirmwareAcceptedIdentity {
            session_epoch: capture.session_epoch,
            document_id: capture.document_id.clone(),
            token: capture.token,
            revision: capture.revision,
            scene_revision: capture.revision,
        }
    }

    fn run_export_effect(effects: Vec<Effect>) -> (OperationId, Effect) {
        let exports = effects
            .into_iter()
            .filter(|effect| matches!(effect, Effect::RunExport { .. }))
            .collect::<Vec<_>>();
        assert_eq!(exports.len(), 1, "one click starts one export operation");
        let effect = exports.into_iter().next().unwrap();
        let Effect::RunExport { operation_id, .. } = &effect else {
            unreachable!()
        };
        (*operation_id, effect)
    }

    fn runtime_fixture(
        executor: Rc<test_support::ControlledExecutor>,
    ) -> (Rc<Runtime>, AcceptedSnapshot, Scope) {
        let runtime = test_support::new_runtime();
        let (session, accepted, scope) = test_support::opened_session();
        let executor: Rc<dyn FirmwareExportExecutor> = executor;
        test_support::configure_runtime(
            &runtime,
            session,
            accepted.clone(),
            scope.clone(),
            executor,
            ExecutorEpoch(11),
            100,
        );
        (runtime, accepted, scope)
    }

    async fn wait_for_actual_stage(
        run: &mut Pin<Box<dyn Future<Output = Vec<Effect>> + '_>>,
        entered: oneshot::Receiver<&'static str>,
    ) -> &'static str {
        let mut entered = Box::pin(entered);
        std::future::poll_fn(|context| {
            if let Poll::Ready(effects) = run.as_mut().poll(context) {
                panic!("production RunExport completed before the gated stage: {effects:?}");
            }
            match entered.as_mut().poll(context) {
                Poll::Ready(Ok(stage)) => Poll::Ready(stage),
                Poll::Ready(Err(_)) => panic!("controlled stage notification was dropped"),
                Poll::Pending => Poll::Pending,
            }
        })
        .await
    }

    #[wasm_bindgen_test]
    async fn production_run_export_injects_both_provider_failures_and_retries_without_editing_session()
     {
        for (stage, expected) in [
            (
                test_support::Stage::Generation,
                "Firmware generation failed: injected firmware-generation failure",
            ),
            (
                test_support::Stage::Packaging,
                "Firmware packaging failed: injected ZIP-packaging failure",
            ),
        ] {
            let (runtime, accepted, scope) =
                runtime_fixture(test_support::ControlledExecutor::failing(stage));
            let before = runtime.session.borrow().read_model().clone();
            let (operation_id, run_effect) =
                run_export_effect(test_support::start_export(&runtime));
            assert!(matches!(
                test_support::take_events(&runtime).as_slice(),
                [Event::StartExport { operation_id: emitted, scope: emitted_scope }]
                    if *emitted == operation_id && emitted_scope == &scope
            ));

            test_support::run_effects(&runtime, vec![run_effect]).await;

            assert_eq!(runtime.session.borrow().read_model(), &before);
            assert_eq!(runtime.model().accepted.as_ref(), Some(&accepted));
            assert_eq!(runtime.scope().as_ref(), Some(&scope));
            assert_eq!(runtime.status(), expected);
            assert!(runtime.status_is_alert());
            assert!(runtime.artifacts.borrow().is_empty());
            assert!(test_support::take_deliveries(&runtime).is_empty());

            let succeeding = test_support::ControlledExecutor::succeeding();
            let succeeding_executor: Rc<dyn FirmwareExportExecutor> = succeeding;
            runtime.replace_firmware_export_test_executor(succeeding_executor, ExecutorEpoch(11));
            let retry_effects = test_support::start_export(&runtime);
            assert!(
                !runtime.status_is_alert(),
                "retry clears the prior firmware alert"
            );
            let (retry_operation, retry_effect) = run_export_effect(retry_effects);
            assert_ne!(retry_operation, operation_id);
            test_support::run_effects(&runtime, vec![retry_effect]).await;

            let deliveries = test_support::take_deliveries(&runtime);
            assert_eq!(
                deliveries.len(),
                1,
                "retry through the same Runtime action delivers once"
            );
            assert_eq!(deliveries[0].filename, "ZMK export test-zmk.zip");
            assert_eq!(deliveries[0].media_type.as_deref(), Some("application/zip"));
            assert_eq!(runtime.session.borrow().read_model(), &before);
            assert!(!runtime.status_is_alert());
        }
    }

    #[wasm_bindgen_test]
    async fn replacing_worker_during_each_actual_await_rejects_old_output_and_suppresses_its_report()
     {
        for (stage, expected_label) in [
            (test_support::Stage::Resolution, "resolution"),
            (test_support::Stage::Generation, "generation"),
            (test_support::Stage::Packaging, "packaging"),
        ] {
            let (gated_executor, entered, release) = test_support::ControlledExecutor::gated(stage);
            let (runtime, _accepted, _scope) = runtime_fixture(gated_executor);
            let before = runtime.session.borrow().read_model().clone();
            let (_operation, effect) = run_export_effect(test_support::start_export(&runtime));
            let mut run: Pin<Box<dyn Future<Output = Vec<Effect>> + '_>> =
                Box::pin(runtime.run(effect));
            assert_eq!(
                wait_for_actual_stage(&mut run, entered).await,
                expected_label
            );

            test_support::replace_executor(
                &runtime,
                test_support::ControlledExecutor::succeeding(),
                ExecutorEpoch(12),
            );
            release
                .send(())
                .expect("release suspended worker operation");
            let effects = run.await;
            test_support::run_effects(&runtime, effects).await;

            assert_eq!(runtime.session.borrow().read_model(), &before);
            assert!(runtime.artifacts.borrow().is_empty());
            assert!(test_support::take_deliveries(&runtime).is_empty());
            assert!(
                !runtime.status_is_alert(),
                "obsolete worker failures stay silent"
            );
        }
    }

    #[wasm_bindgen_test]
    async fn production_completion_after_owner_switch_does_not_deliver_or_report() {
        let (executor, entered, release) =
            test_support::ControlledExecutor::gated(test_support::Stage::Generation);
        let (runtime, accepted, scope) = runtime_fixture(executor);
        let before = runtime.session.borrow().read_model().clone();
        let (_, effect) = run_export_effect(test_support::start_export(&runtime));
        let mut run: Pin<Box<dyn Future<Output = Vec<Effect>> + '_>> =
            Box::pin(runtime.run(effect));
        assert_eq!(wait_for_actual_stage(&mut run, entered).await, "generation");

        let mut next_accepted = accepted;
        next_accepted.token = SnapshotToken(next_accepted.token.0 + 1);
        let next_scope = Scope {
            board_id: "next-board".into(),
            ..scope
        };
        test_support::replace_owner(&runtime, next_accepted.clone(), Some(next_scope.clone()));
        release.send(()).expect("release old project generation");
        test_support::run_effects(&runtime, run.await).await;

        assert_eq!(runtime.model().accepted.as_ref(), Some(&next_accepted));
        assert_eq!(runtime.scope().as_ref(), Some(&next_scope));
        assert_eq!(runtime.session.borrow().read_model(), &before);
        assert!(runtime.artifacts.borrow().is_empty());
        assert!(test_support::take_deliveries(&runtime).is_empty());
        assert!(
            !runtime.status_is_alert(),
            "an old project failure must not be announced in the new project"
        );
    }

    #[wasm_bindgen_test]
    async fn production_older_export_can_deliver_without_replacing_newer_report() {
        let (executor, entered, release) =
            test_support::ControlledExecutor::gated(test_support::Stage::Generation);
        let (runtime, _accepted, _scope) = runtime_fixture(executor);
        let (_, old_effect) = run_export_effect(test_support::start_export(&runtime));
        let mut old_run: Pin<Box<dyn Future<Output = Vec<Effect>> + '_>> =
            Box::pin(runtime.run(old_effect));
        assert_eq!(
            wait_for_actual_stage(&mut old_run, entered).await,
            "generation"
        );

        let (new_operation, new_effect) = run_export_effect(test_support::start_export(&runtime));
        assert_ne!(new_operation, OperationId(100));
        test_support::run_effects(&runtime, vec![new_effect]).await;
        let new_deliveries = test_support::take_deliveries(&runtime);
        assert_eq!(new_deliveries.len(), 1, "newest attempt delivers once");
        assert_eq!(runtime.status(), "Saved locally.");

        release.send(()).expect("release older generation");
        test_support::run_effects(&runtime, old_run.await).await;
        let older_deliveries = test_support::take_deliveries(&runtime);
        assert_eq!(
            older_deliveries.len(),
            1,
            "Session keeps a still-current concurrent export independently deliverable"
        );
        assert_eq!(older_deliveries[0].filename, "ZMK export test-zmk.zip");
        assert_eq!(runtime.status(), "Saved locally.");
        assert!(!runtime.status_is_alert());
    }

    #[wasm_bindgen_test]
    fn firmware_failure_announcement_has_explicit_alert_severity_and_stale_owner_is_silent() {
        for reason in [
            "Firmware generation failed: worker rejected the plan.",
            "Firmware packaging failed: ZIP worker returned no bytes.",
        ] {
            let report = firmware_export_terminal_report(
                true,
                TerminalOutcome::ExecutorFailed(reason.into()),
                None,
            )
            .expect("current firmware failure is announced");
            assert_eq!(report.message, reason);
            assert_eq!(report.severity, RuntimeReportSeverity::Alert);
        }

        assert!(
            firmware_export_terminal_report(
                false,
                TerminalOutcome::ExecutorFailed("late failure from old project".into()),
                None,
            )
            .is_none()
        );

        let mut report = RuntimeReport::status(
            "Firmware generation failed: same text is routine status without typed severity.",
        );
        assert_eq!(report.severity, RuntimeReportSeverity::Status);
        report = firmware_export_terminal_report(
            true,
            TerminalOutcome::Completed,
            Some("Browser refused the firmware download.".into()),
        )
        .expect("delivery failure is announced");
        assert_eq!(report.severity, RuntimeReportSeverity::Alert);
        report.clear_alert();
        assert_eq!(report.severity, RuntimeReportSeverity::Status);
        assert!(report.message.is_empty());
    }

    #[wasm_bindgen_test]
    fn firmware_completion_is_suppressed_after_project_board_or_snapshot_changes() {
        let capture = capture();
        let accepted = accepted(&capture);
        assert!(firmware_export_capture_matches(
            &capture,
            Some(&capture.scope),
            Some(&accepted),
        ));

        let other_board = Scope {
            board_id: "board-b".into(),
            ..capture.scope.clone()
        };
        assert!(!firmware_export_capture_matches(
            &capture,
            Some(&other_board),
            Some(&accepted),
        ));

        let mut newer_snapshot = accepted.clone();
        newer_snapshot.token = SnapshotToken(18);
        assert!(!firmware_export_capture_matches(
            &capture,
            Some(&capture.scope),
            Some(&newer_snapshot),
        ));

        let mut other_project = accepted;
        other_project.document_id = "project-b".into();
        assert!(!firmware_export_capture_matches(
            &capture,
            Some(&capture.scope),
            Some(&other_project),
        ));
    }

    #[wasm_bindgen_test]
    fn older_export_cannot_replace_a_newer_attempts_report() {
        let first = OperationId(41);
        let second = OperationId(42);
        assert!(firmware_export_is_latest(second, Some(second)));
        assert!(!firmware_export_is_latest(first, Some(second)));
        assert!(!firmware_export_is_latest(first, None));
    }

    #[wasm_bindgen_test]
    fn worker_replacement_at_resolution_generation_or_packaging_boundary_invalidates_attempt() {
        let expected = ExecutorEpoch(11);
        let stages = ["resolution", "generation", "packaging"];
        for stage in stages {
            assert!(firmware_export_worker_is_current(expected, expected, true));
            assert!(
                !firmware_export_worker_is_current(expected, ExecutorEpoch(12), false),
                "worker replacement during {stage} must invalidate the export"
            );
        }
    }

    #[wasm_bindgen_test]
    fn generation_and_zip_failures_cannot_produce_deliverable_firmware_bytes() {
        for error in [
            "Firmware generation failed: test worker rejection.",
            "Firmware packaging failed: test ZIP rejection.",
        ] {
            let result = firmware_export_bytes_for_delivery(Err(error.into()), true, false);
            assert_eq!(result, Err(error.into()));
        }
        assert_eq!(
            firmware_export_bytes_for_delivery(Ok(vec![1, 2, 3]), false, false),
            Err("Export scope changed before delivery.".into())
        );
        assert_eq!(
            firmware_export_bytes_for_delivery(Ok(vec![1, 2, 3]), true, true),
            Err("Export scope changed before delivery.".into())
        );
    }
}

#[cfg(all(test, target_arch = "wasm32"))]
mod cad_scene_rebind_tests {
    use super::*;
    use crate::case_generation_lifecycle::physical_case_fingerprint;
    use std::sync::Arc;
    use wasm_bindgen_test::wasm_bindgen_test;

    fn accepted_after_edit(
        original: &AcceptedSnapshot,
        token: u64,
        revision: u64,
        board_thickness: Option<f64>,
    ) -> AcceptedSnapshot {
        let mut document = (*original.document).clone();
        document.revision = revision;
        document.name.push_str(" updated");
        if let Some(thickness) = board_thickness {
            document.boards[0].thickness = thickness;
        }
        let mut scene = (*original.scene).clone();
        scene.revision = revision;
        AcceptedSnapshot {
            token: SnapshotToken(token),
            session_epoch: original.session_epoch,
            document: Arc::new(document),
            scene: Arc::new(scene),
        }
    }

    fn runtime_with_scene(exact: bool) -> (Rc<Runtime>, AcceptedSnapshot, Scope) {
        let runtime = Runtime::new().expect("browser runtime fixture initializes");
        let (_, accepted, scope) = firmware_export_test_support::opened_session();
        runtime.set_definition_name_test_state(accepted.clone(), Some(scope.clone()));
        let fingerprint = physical_case_fingerprint(&accepted, &scope)
            .expect("the accepted board fixture has physical inputs");
        *runtime.cad_scene.borrow_mut() = Some(Rc::new(CadScene {
            scope: scope.clone(),
            token: accepted.token,
            snapshot: accepted.clone(),
            result: CadResult {
                revision: accepted.document.revision,
                ..CadResult::default()
            },
            prepared: boardstudio_core::model::PreparedCaseAssemblyIR {
                revision: accepted.document.revision,
                bodies: Vec::new(),
            },
            physical_fingerprint: Some(fingerprint),
            mechanical: None,
            exact,
            contours: Vec::new(),
        }));
        (runtime, accepted, scope)
    }

    #[wasm_bindgen_test]
    fn cad_scene_rebinds_only_exact_output_with_matching_physical_inputs() {
        let (exact_runtime, original, scope) = runtime_with_scene(true);
        let current = accepted_after_edit(
            &original,
            original.token.0 + 1,
            original.document.revision + 1,
            None,
        );
        exact_runtime.set_definition_name_test_state(current.clone(), Some(scope.clone()));
        let rebound = exact_runtime
            .cad_scene()
            .expect("same-scope completed output remains available");
        assert_eq!(rebound.token, current.token);
        assert_eq!(
            rebound.snapshot.document.revision,
            current.document.revision
        );
        assert_eq!(rebound.result.revision, current.document.revision);
        assert_eq!(rebound.prepared.revision, current.document.revision);

        let (preview_runtime, original, scope) = runtime_with_scene(false);
        let current = accepted_after_edit(
            &original,
            original.token.0 + 1,
            original.document.revision + 1,
            None,
        );
        preview_runtime.set_definition_name_test_state(current, Some(scope));
        let preview = preview_runtime
            .cad_scene()
            .expect("captured in-flight output remains inspectable");
        assert!(!preview.exact);
        assert_eq!(
            preview.token, original.token,
            "in-flight output keeps its captured token"
        );
        assert_eq!(
            preview.snapshot.document.revision,
            original.document.revision
        );

        let (changed_runtime, original, scope) = runtime_with_scene(true);
        let current = accepted_after_edit(
            &original,
            original.token.0 + 1,
            original.document.revision + 1,
            Some(2.0),
        );
        changed_runtime.set_definition_name_test_state(current, Some(scope));
        let previous = changed_runtime
            .cad_scene()
            .expect("same-scope old output remains available for stale display");
        assert_eq!(previous.token, original.token);
        assert_eq!(
            previous.snapshot.document.revision,
            original.document.revision
        );
    }

    #[wasm_bindgen_test]
    fn cad_scene_retires_old_scope_instead_of_rebinding_across_physical_owner() {
        let (runtime, original, scope) = runtime_with_scene(true);
        let mut alternate_scope = scope.clone();
        alternate_scope.board_id = "other-board".into();
        let current = accepted_after_edit(
            &original,
            original.token.0 + 1,
            original.document.revision + 1,
            None,
        );
        runtime.set_definition_name_test_state(current, Some(alternate_scope));
        assert!(runtime.cad_scene().is_none());
    }
}
