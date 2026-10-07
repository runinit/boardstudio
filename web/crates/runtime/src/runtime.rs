//! Browser composition runs identified effects; the headless session remains authoritative.
use crate::archive_export::{ArchiveExportOptions, ArchiveWorkFuture, archive_filename};
use crate::pcb_wiring_mode_operation::electrical_preview_request;
#[cfg(any(test, feature = "test-support"))]
use boardstudio_application::GenerationStatus;
use boardstudio_application::{
    AcceptedSnapshot, Completion, Durability, Effect, Event, JobId, Lifecycle, OperationId,
    ReadModel, SaveResult, Scope, Session, SnapshotToken, TerminalOutcome,
};
use boardstudio_core::{
    electrical::{ElectricalMode, ElectricalPlan, ElectricalPlanRequest},
    model::{
        ArchiveEntry, ArchiveReply, ArchiveRequest, ArtifactReply, ArtifactRequest, Board,
        CaseAssemblyIR, CaseIR, CompiledFootprint, CoreReply, CoreRequest, HardwareTopology,
        KeycapSpec, Material, MechanicalAssembly, MechanicalBuiltinProfile,
        MechanicalConfiguration, MechanicalExtraction, MechanicalPartProfile,
        MechanicalPurposeMapping, MechanicalSwitchFamily, Operation, OutlineFeature,
        OutlineSettings, PcbPreview, PrepareExportRequest, ProjectDoc,
    },
};
pub use boardstudio_web_host::host::CoreExecutor;

/// The stable identity of the current Core executor: captures compare it to detect that the
/// executor was replaced while their operation was in flight.
fn core_executor_identity(core: &Rc<dyn CoreExecutor>) -> usize {
    Rc::as_ptr(core) as *const () as usize
}
use boardstudio_web_host::host::{BrowserStore, CoreWorker};
use boardstudio_web_host::{
    cad_jobs::{
        CadJobError, CadOperation, CadRequest, CadResult, CadSnapshotIdentity,
        captured_case_document, captured_case_scene, prepare_captured_case,
        prepare_captured_step_assembly, validate_reply,
    },
    cad_worker::CadWorker,
};
use sha2::{Digest, Sha256};

#[cfg(any(test, feature = "test-support"))]
type CaseGesturePreviewTestExecutor = dyn Fn(AcceptedSnapshot, Scope) -> Result<CadScene, String>;

#[cfg(any(test, feature = "test-support"))]
type KeycapsPreviewTestExecutor = dyn Fn(
    KeycapsPreviewInput,
) -> std::pin::Pin<
    Box<
        dyn std::future::Future<
                Output = Result<Vec<boardstudio_web_host::cad_jobs::CadBodyMesh>, String>,
            >,
    >,
>;

pub struct CadScene {
    pub scope: Scope,
    pub token: SnapshotToken,
    pub snapshot: AcceptedSnapshot,
    pub result: CadResult,
    pub prepared: boardstudio_core::model::PreparedCaseAssemblyIR,
    pub physical_fingerprint: Option<[u8; 32]>,
    pub mechanical: Option<boardstudio_core::model::MechanicalAssembly>,
    pub exact: bool,
    pub contours: Vec<boardstudio_core::model::Contour>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct KeycapsPreviewInput {
    pub scope: Scope,
    pub token: SnapshotToken,
    pub revision: u64,
    pub specs: Vec<KeycapSpec>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct KeycapsCadPreview {
    pub generation: u64,
    pub scope: Scope,
    pub token: SnapshotToken,
    pub revision: u64,
    pub specs: Vec<KeycapSpec>,
    pub bodies: Vec<boardstudio_web_host::cad_jobs::CadBodyMesh>,
}

#[cfg(any(test, feature = "test-support"))]
struct ImportArchiveTestGate {
    entered: futures_channel::oneshot::Sender<()>,
    release: futures_channel::oneshot::Receiver<()>,
    metadata: String,
    buffers: Vec<Vec<u8>>,
}

#[cfg(any(test, feature = "test-support"))]
struct OpenSavedLoadTestGate {
    entered: futures_channel::oneshot::Sender<()>,
    release: futures_channel::oneshot::Receiver<()>,
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

/// The document persistence port: the save Runtime performs for Session `Persist` effects.
/// Loading, listing, deleting and the active-project preference stay on the browser store.
/// Production saves through IndexedDB; tests install the in-process memory adapter from
/// `in_process_support`.
pub trait DocumentPersistence {
    fn save_document<'a>(
        &'a self,
        document: &'a ProjectDoc,
        assets: &'a BTreeMap<String, Vec<u8>>,
    ) -> Pin<Box<dyn Future<Output = Result<(), String>> + 'a>>;
}

impl DocumentPersistence for BrowserStore {
    fn save_document<'a>(
        &'a self,
        document: &'a ProjectDoc,
        assets: &'a BTreeMap<String, Vec<u8>>,
    ) -> Pin<Box<dyn Future<Output = Result<(), String>> + 'a>> {
        Box::pin(async move {
            BrowserStore::save_document(self, document, assets)
                .await
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
struct MechanicalExportCapture {
    scope: Scope,
    token: SnapshotToken,
    revision: u64,
    session_epoch: boardstudio_application::SessionEpoch,
    document_id: String,
    executor_epoch: boardstudio_application::ExecutorEpoch,
    core_worker_identity: usize,
    filename: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct FirmwareAcceptedIdentity {
    session_epoch: boardstudio_application::SessionEpoch,
    document_id: String,
    token: SnapshotToken,
    revision: u64,
    scene_revision: u64,
}

#[cfg(any(test, feature = "test-support"))]
struct FirmwareExportTestContext {
    accepted: AcceptedSnapshot,
    scope: Option<Scope>,
    current_executor: Rc<dyn CoreExecutor>,
    executor_epoch: boardstudio_application::ExecutorEpoch,
}

#[cfg(any(test, feature = "test-support"))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FirmwareTestDelivery {
    pub bytes: Vec<u8>,
    pub filename: String,
    pub media_type: Option<String>,
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
pub fn new_project_id() -> Result<String, String> {
    browser_uuid()
}

pub struct Runtime {
    session: RefCell<Session>,
    operation_outcomes: crate::operation_outcomes::OperationOutcomes,
    core: RefCell<Rc<dyn CoreExecutor>>,
    core_executor_factory: RefCell<Rc<dyn Fn() -> Result<Rc<dyn CoreExecutor>, String>>>,
    pub store: BrowserStore,
    persistence: RefCell<Rc<dyn DocumentPersistence>>,
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
    case_gesture_preview: RefCell<crate::case_gesture_preview::CaseGesturePreviewState>,
    case_gesture_preview_job:
        RefCell<Option<(crate::case_gesture_preview::CaseGesturePreviewOwner, String)>>,
    keycaps_preview_generation: Cell<u64>,
    keycaps_preview_worker: RefCell<Option<(u64, Rc<CadWorker>)>>,
    cad_jobs: RefCell<BTreeMap<JobId, Rc<Cell<bool>>>>,
    step_exports: RefCell<BTreeSet<OperationId>>,
    keycaps_step_exports: RefCell<BTreeSet<OperationId>>,
    firmware_exports: RefCell<BTreeMap<OperationId, FirmwareExportCapture>>,
    footprint_exports: RefCell<BTreeMap<OperationId, FootprintExportCapture>>,
    pcb_handoff_exports: RefCell<BTreeMap<OperationId, PcbHandoffCapture>>,
    mechanical_exports: RefCell<BTreeMap<OperationId, MechanicalExportCapture>>,
    latest_firmware_export: Cell<Option<OperationId>>,
    firmware_export_delivery_errors: RefCell<BTreeMap<OperationId, String>>,
    export_workers: RefCell<BTreeMap<OperationId, Rc<CadWorker>>>,
    native_case_preview: RefCell<crate::case_preview::NativePreviewState>,
    case_model_delivery: crate::model_delivery::ModelDeliveryAdapter,
    native_model_delivery: RefCell<NativeModelDeliveryState>,
    layout_preview: RefCell<crate::layout_viewer_source::LayoutPreviewState>,
    layout_model_rows: RefCell<
        Option<(
            crate::layout_viewer_source::LayoutSourceIdentity,
            crate::model_delivery::ModelDeliveryRows,
        )>,
    >,
    layout_model_batch_generation: Cell<u64>,
    native_model_jobs: RefCell<BTreeSet<String>>,
    archive_export_options: ArchiveExportOptions,
    #[cfg(any(test, feature = "test-support"))]
    definition_name_test_state: RefCell<Option<(AcceptedSnapshot, Option<Scope>)>>,
    // Preserve accepted scope/event fixtures while mounting transient ReadModel states.
    #[cfg(any(test, feature = "test-support"))]
    definition_name_test_model: RefCell<Option<ReadModel>>,
    #[cfg(any(test, feature = "test-support"))]
    definition_name_test_events: RefCell<Vec<Event>>,
    #[cfg(any(test, feature = "test-support"))]
    definition_name_test_generation: RefCell<Option<GenerationStatus>>,
    #[cfg(any(test, feature = "test-support"))]
    firmware_export_test_context: RefCell<Option<FirmwareExportTestContext>>,
    #[cfg(any(test, feature = "test-support"))]
    firmware_export_test_effects: RefCell<Vec<Effect>>,
    #[cfg(any(test, feature = "test-support"))]
    firmware_export_test_events: RefCell<Vec<Event>>,
    #[cfg(any(test, feature = "test-support"))]
    firmware_export_test_deliveries: RefCell<Vec<FirmwareTestDelivery>>,
    #[cfg(any(test, feature = "test-support"))]
    case_gesture_preview_test_executor: RefCell<Option<Rc<CaseGesturePreviewTestExecutor>>>,
    #[cfg(any(test, feature = "test-support"))]
    keycaps_preview_test_executor: RefCell<Option<Rc<KeycapsPreviewTestExecutor>>>,
    #[cfg(any(test, feature = "test-support"))]
    in_process_adapters: RefCell<Option<Rc<crate::runtime::in_process_support::InProcessAdapters>>>,
    #[cfg(any(test, feature = "test-support"))]
    layout_component_inspector_test_state: RefCell<Option<(ReadModel, Option<Scope>)>>,
    #[cfg(any(test, feature = "test-support"))]
    layout_component_inspector_test_events: RefCell<Vec<Event>>,
    #[cfg(any(test, feature = "test-support"))]
    held_effects: RefCell<Vec<Effect>>,
    #[cfg(any(test, feature = "test-support"))]
    import_archive_test_gate: RefCell<Option<ImportArchiveTestGate>>,
    #[cfg(any(test, feature = "test-support"))]
    open_saved_load_test_gate: RefCell<Option<OpenSavedLoadTestGate>>,
    #[cfg(any(test, feature = "test-support"))]
    import_file_test_done: RefCell<Option<futures_channel::oneshot::Sender<()>>>,
    #[cfg(any(test, feature = "test-support"))]
    open_saved_test_done: RefCell<Option<futures_channel::oneshot::Sender<()>>>,
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
        let core_executor_factory: Rc<dyn Fn() -> Result<Rc<dyn CoreExecutor>, String>> =
            Rc::new(|| {
                resource_url("assets/core-worker/entry.js")
                    .and_then(|url| CoreWorker::new(&url).map_err(|e| e.to_string()))
                    .map(|worker| Rc::new(worker) as Rc<dyn CoreExecutor>)
            });
        let store = BrowserStore::scoped(if prefix == "/" { "root" } else { "boardstudio" })
            .map_err(|e| e.to_string())?;
        let runtime = Rc::new(Self {
            session: RefCell::new(Session::new()),
            operation_outcomes: crate::operation_outcomes::OperationOutcomes::default(),
            core: RefCell::new(core_executor_factory.clone()()?),
            core_executor_factory: RefCell::new(core_executor_factory),
            persistence: RefCell::new(Rc::new(store.clone())),
            store,
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
            case_gesture_preview: RefCell::new(Default::default()),
            case_gesture_preview_job: RefCell::new(None),
            keycaps_preview_generation: Cell::new(0),
            keycaps_preview_worker: RefCell::new(None),
            cad_jobs: RefCell::new(BTreeMap::new()),
            step_exports: RefCell::new(BTreeSet::new()),
            keycaps_step_exports: RefCell::new(BTreeSet::new()),
            firmware_exports: RefCell::new(BTreeMap::new()),
            footprint_exports: RefCell::new(BTreeMap::new()),
            pcb_handoff_exports: RefCell::new(BTreeMap::new()),
            mechanical_exports: RefCell::new(BTreeMap::new()),
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
            archive_export_options: ArchiveExportOptions::default(),
            #[cfg(any(test, feature = "test-support"))]
            definition_name_test_state: RefCell::new(None),
            #[cfg(any(test, feature = "test-support"))]
            definition_name_test_model: RefCell::new(None),
            #[cfg(any(test, feature = "test-support"))]
            definition_name_test_events: RefCell::new(Vec::new()),
            #[cfg(any(test, feature = "test-support"))]
            definition_name_test_generation: RefCell::new(None),
            #[cfg(any(test, feature = "test-support"))]
            firmware_export_test_context: RefCell::new(None),
            #[cfg(any(test, feature = "test-support"))]
            firmware_export_test_effects: RefCell::new(Vec::new()),
            #[cfg(any(test, feature = "test-support"))]
            firmware_export_test_events: RefCell::new(Vec::new()),
            #[cfg(any(test, feature = "test-support"))]
            firmware_export_test_deliveries: RefCell::new(Vec::new()),
            #[cfg(any(test, feature = "test-support"))]
            case_gesture_preview_test_executor: RefCell::new(None),
            #[cfg(any(test, feature = "test-support"))]
            keycaps_preview_test_executor: RefCell::new(None),
            #[cfg(any(test, feature = "test-support"))]
            in_process_adapters: RefCell::new(None),
            #[cfg(any(test, feature = "test-support"))]
            layout_component_inspector_test_state: RefCell::new(None),
            #[cfg(any(test, feature = "test-support"))]
            layout_component_inspector_test_events: RefCell::new(Vec::new()),
            #[cfg(any(test, feature = "test-support"))]
            held_effects: RefCell::new(Vec::new()),
            #[cfg(any(test, feature = "test-support"))]
            import_archive_test_gate: RefCell::new(None),
            #[cfg(any(test, feature = "test-support"))]
            open_saved_load_test_gate: RefCell::new(None),
            #[cfg(any(test, feature = "test-support"))]
            import_file_test_done: RefCell::new(None),
            #[cfg(any(test, feature = "test-support"))]
            open_saved_test_done: RefCell::new(None),
        });
        // Reserve the startup open identity synchronously, before any explicit
        // open action can supersede restoration of the last durable project.
        if restore_active_project {
            runtime.restore_active_project();
        }
        // Test servers do not serve the release service worker, so registering it only races
        // a 404 report against whatever status the test is observing.
        #[cfg(not(any(test, feature = "test-support")))]
        let weak = Rc::downgrade(&runtime);
        #[cfg(not(any(test, feature = "test-support")))]
        spawn_local(async move {
            if let Err(error) = boardstudio_web_host::host::register_offline(prefix).await
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
    /// Observe an operation's terminal outcome and where it landed; edit tickets use this.
    pub(crate) fn observe_operation_with_landing(
        &self,
        operation: OperationId,
    ) -> (
        crate::operation_outcomes::OutcomeSlot,
        crate::operation_outcomes::LandingSlot,
    ) {
        self.operation_outcomes.observe_with_landing(operation)
    }
    pub fn operation(&self) -> OperationId {
        let id = self.next_operation.get();
        self.next_operation
            .set(id.checked_add(1).expect("operation identity exhausted"));
        OperationId(id)
    }
    pub fn scope(&self) -> Option<boardstudio_application::Scope> {
        #[cfg(any(test, feature = "test-support"))]
        if let Some(context) = self.firmware_export_test_context.borrow().as_ref() {
            return context.scope.clone();
        }
        #[cfg(any(test, feature = "test-support"))]
        if let Some((_, scope)) = self.definition_name_test_state.borrow().as_ref() {
            return scope.clone();
        }
        #[cfg(any(test, feature = "test-support"))]
        if let Some((_, scope)) = self
            .layout_component_inspector_test_state
            .borrow()
            .as_ref()
        {
            return scope.clone();
        }
        self.session.borrow().scope()
    }
    pub fn electrical_preview_executor_epoch(&self) -> u64 {
        #[cfg(any(test, feature = "test-support"))]
        if let Some(context) = self.firmware_export_test_context.borrow().as_ref() {
            return context.executor_epoch.0;
        }
        self.session.borrow().core_executor_epoch().0
    }
    pub fn model(&self) -> ReadModel {
        #[cfg(any(test, feature = "test-support"))]
        if let Some(model) = self.definition_name_test_model.borrow().as_ref() {
            return model.clone();
        }
        #[cfg(any(test, feature = "test-support"))]
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
        #[cfg(any(test, feature = "test-support"))]
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
        #[cfg(any(test, feature = "test-support"))]
        if let Some((model, _)) = self
            .layout_component_inspector_test_state
            .borrow()
            .as_ref()
        {
            return model.clone();
        }
        self.session.borrow().read_model().clone()
    }

    /// Load a reviewed bundled switch fit into the Parts editor's local draft.
    /// This never submits an accepted edit; the existing Parts profile Save path
    /// remains the sole owner of document history.
    pub async fn standard_switch_profile(
        &self,
        operation_id: OperationId,
        definition_id: String,
        family: MechanicalSwitchFamily,
        plate_to_pcb: f64,
    ) -> Result<MechanicalPartProfile, String> {
        let source = match family {
            MechanicalSwitchFamily::Mx => MechanicalBuiltinProfile::MxSwitch,
            MechanicalSwitchFamily::ChocV1 => MechanicalBuiltinProfile::ChocV1Switch,
            MechanicalSwitchFamily::ChocV2 => MechanicalBuiltinProfile::ChocV2Switch,
        };
        self.standard_builtin_profile(
            operation_id,
            definition_id,
            source,
            Some(family),
            plate_to_pcb,
        )
        .await
    }

    /// Load a reviewed Core builtin profile for the Case editor's private assignment path.
    pub async fn standard_builtin_profile(
        &self,
        operation_id: OperationId,
        definition_id: String,
        source: MechanicalBuiltinProfile,
        expected_family: Option<MechanicalSwitchFamily>,
        plate_to_pcb: f64,
    ) -> Result<MechanicalPartProfile, String> {
        let executor_epoch = self.session.borrow().core_executor_epoch();
        let core = self.core.borrow().clone();
        let request_id = format!("parts-standard-profile-{}", operation_id.0);
        let request = CoreRequest::MechanicalProfile {
            id: request_id.clone(),
            definition_id: definition_id.clone(),
            source: source.clone(),
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
        match reply {
            CoreReply::MechanicalProfile { id, profile } if id == request_id => {
                if profile.definition_id != definition_id
                    || profile.switch_family != expected_family
                    || (profile.plate_to_pcb - plate_to_pcb).abs() > f64::EPSILON
                    || (matches!(
                        source,
                        MechanicalBuiltinProfile::MxStab2u | MechanicalBuiltinProfile::MxStab625u
                    ) && profile.source_geometry.is_none())
                {
                    return Err("Core returned a different standard mechanical profile.".into());
                }
                Ok(profile)
            }
            CoreReply::Error { id, message, .. } if id == request_id => Err(message),
            CoreReply::MechanicalProfile { .. } | CoreReply::Error { .. } => {
                Err("Core returned a stale standard mechanical profile reply.".into())
            }
            _ => Err("Core returned an unexpected standard mechanical profile reply.".into()),
        }
    }

    /// Import a source-owned KiCad footprint through the existing artifact worker.
    /// The Parts owner admits the returned definition into Session after rechecking scope.
    pub async fn import_footprint(
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

    /// Read KiCad mechanical geometry for a Parts editor draft. The caller owns the
    /// selected-definition and scope admission before applying any returned geometry.
    pub async fn extract_mechanical_profile(
        &self,
        request_id: String,
        source: String,
        mappings: Vec<MechanicalPurposeMapping>,
        max_deviation_mm: f64,
    ) -> Result<MechanicalExtraction, String> {
        let executor_epoch = self.session.borrow().core_executor_epoch();
        let core = self.core.borrow().clone();
        let request = ArtifactRequest::ExtractMechanical {
            id: request_id.clone(),
            source,
            mappings,
            max_deviation_mm,
        };
        let reply = core
            .artifact(&request_id, &executor_epoch.0.to_string(), &request)
            .await
            .map_err(|error| format!("Could not read KiCad mechanical geometry: {error}"))?;
        if self.session.borrow().core_executor_epoch() != executor_epoch
            || !Rc::ptr_eq(&core, &self.core.borrow())
        {
            return Err(
                "The KiCad geometry result became stale when the Core worker changed.".into(),
            );
        }
        match reply {
            ArtifactReply::ExtractMechanical { id, result } if id == request_id => Ok(result),
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
            ArtifactReply::ExtractMechanical { .. } | ArtifactReply::Error { .. } => {
                Err("Core returned KiCad geometry for another request.".into())
            }
            _ => Err("Core returned an unexpected KiCad geometry reply.".into()),
        }
    }

    /// Ask the existing Core worker to project candidate matrices with its authoritative
    /// layout geometry. This is a private preview path; candidates are never installed in Session.
    pub async fn project_matrices(
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
    pub fn status_is_alert(&self) -> bool {
        self.status.borrow().severity == RuntimeReportSeverity::Alert
    }
    pub fn embed_used_models(&self) -> bool {
        self.archive_export_options.embed_used_models()
    }
    pub fn set_embed_used_models(&self, value: bool) {
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

    fn cancel_case_gesture_preview_job(
        &self,
        owner: &crate::case_gesture_preview::CaseGesturePreviewOwner,
    ) {
        let job = {
            let mut active = self.case_gesture_preview_job.borrow_mut();
            if active.as_ref().is_some_and(|(current, _)| current == owner) {
                active.take().map(|(_, job)| job)
            } else {
                None
            }
        };
        if let Some(job) = job
            && let Some((_, worker)) = self.cad_worker.borrow().as_ref()
        {
            let _ = worker.cancel(&job);
        }
    }

    fn cancel_active_case_gesture_preview(&self) {
        let owner = self.case_gesture_preview.borrow().active_owner();
        if let Some(owner) = owner {
            self.cancel_case_gesture_preview_job(&owner);
            self.case_gesture_preview.borrow_mut().cancel(&owner);
        }
    }

    pub fn cancel_case_gesture_preview(
        &self,
        owner: &crate::case_gesture_preview::CaseGesturePreviewOwner,
    ) {
        if self.case_gesture_preview.borrow_mut().cancel(owner) {
            self.cancel_case_gesture_preview_job(owner);
            self.changed();
        }
    }

    fn case_gesture_preview_is_current(
        &self,
        owner: &crate::case_gesture_preview::CaseGesturePreviewOwner,
    ) -> bool {
        self.case_gesture_preview.borrow().is_current(owner)
            && self.scope().as_ref() == Some(&owner.scope)
            && self.model().accepted.as_ref().is_some_and(|accepted| {
                accepted.token == owner.snapshot_token
                    && accepted.document.revision == owner.revision
                    && accepted.document.id == owner.scope.document_id
                    && accepted.session_epoch == owner.scope.session_epoch
            })
    }

    pub fn cancel_native_case_preview(&self) {
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
    pub fn observe_operation(
        &self,
        operation: OperationId,
    ) -> crate::operation_outcomes::OutcomeSlot {
        self.operation_outcomes.observe(operation)
    }

    /// Open a fresh blank keyboard through the normal Session persistence path.
    pub fn create_new_keyboard(
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

    pub async fn delete_saved_project(self: &Rc<Self>, project_id: String) -> Result<(), String> {
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
            #[cfg(all(any(test, feature = "test-support"), target_arch = "wasm32"))]
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
    pub async fn resolve_mechanical_settings(
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
        let effective_document = boardstudio_web_host::cad_jobs::captured_case_document(
            &proposal_projection_input,
            &scope,
        )
        .map_err(|error| format!("Could not project proposed mechanical settings: {error:?}"))?;
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
        let contours = boardstudio_web_host::cad_jobs::captured_case_scene(&accepted, &scope)
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
    pub async fn resolve_electrical_preview(
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
    pub async fn resolve_keycaps_preview(
        &self,
        scope: Scope,
        token: SnapshotToken,
        revision: u64,
    ) -> Result<boardstudio_core::model::KeycapResolution, String> {
        self.resolve_keycaps_preview_source(scope, token, revision, false)
            .await
    }

    /// Case consumes the selected physical projection; Layout retains the canonical document.
    pub async fn resolve_case_keycaps_preview(
        &self,
        scope: Scope,
        token: SnapshotToken,
        revision: u64,
    ) -> Result<boardstudio_core::model::KeycapResolution, String> {
        self.resolve_keycaps_preview_source(scope, token, revision, true)
            .await
    }

    async fn resolve_keycaps_preview_source(
        &self,
        scope: Scope,
        token: SnapshotToken,
        revision: u64,
        physical_case: bool,
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

        let document = if physical_case {
            captured_case_document(&accepted, &scope)
                .map_err(|error| format!("Could not capture physical Case keycaps: {error:?}"))?
        } else {
            (*accepted.document).clone()
        };
        let request_id = format!("keycaps-fit-{}", self.operation().0);
        let request = CoreRequest::ResolveKeycaps {
            id: request_id.clone(),
            document,
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
    pub async fn request_keycaps_cad_preview(
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
        #[cfg(any(test, feature = "test-support"))]
        if let Some(executor) = self.keycaps_preview_test_executor.borrow().clone() {
            let bodies = executor(input.clone()).await?;
            self.ensure_keycaps_source_current(&accepted, &input.scope)?;
            if self.keycaps_preview_generation.get() != generation {
                return Err("Keycap CAD preview was cancelled or superseded.".into());
            }
            return Ok(KeycapsCadPreview {
                generation,
                scope: input.scope,
                token: input.token,
                revision: input.revision,
                specs: input.specs,
                bodies,
            });
        }
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
    pub fn cancel_keycaps_cad_preview(&self) {
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
        #[cfg(any(test, feature = "test-support"))]
        let test_event = event.clone();
        #[cfg(any(test, feature = "test-support"))]
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
        #[cfg(any(test, feature = "test-support"))]
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
            self.cancel_active_case_gesture_preview();
            self.cancel_keycaps_cad_preview();
        }
        if self.scope() != previous_scope {
            self.cad_scene.borrow_mut().take();
            if let Some((_, worker)) = self.cad_worker.borrow_mut().take() {
                worker.close();
            }
        }
        self.changed();
        #[cfg(any(test, feature = "test-support"))]
        if self.firmware_export_test_context.borrow().is_some() {
            self.firmware_export_test_events
                .borrow_mut()
                .push(test_event);
            self.firmware_export_test_effects
                .borrow_mut()
                .extend(effects);
            return;
        }
        #[cfg(any(test, feature = "test-support"))]
        if self.in_process_adapters.borrow().is_some() {
            self.held_effects.borrow_mut().extend(effects);
            return;
        }
        self.drive(effects);
    }

    #[cfg(any(test, feature = "test-support"))]
    pub fn set_definition_name_test_state(&self, snapshot: AcceptedSnapshot, scope: Option<Scope>) {
        *self.definition_name_test_state.borrow_mut() = Some((snapshot, scope));
    }

    #[cfg(any(test, feature = "test-support"))]
    pub fn set_definition_name_test_model(&self, model: ReadModel) {
        *self.definition_name_test_model.borrow_mut() = Some(model);
    }

    #[cfg(any(test, feature = "test-support"))]
    pub fn set_case_gesture_preview_executor_test(
        &self,
        executor: Rc<CaseGesturePreviewTestExecutor>,
    ) {
        *self.case_gesture_preview_test_executor.borrow_mut() = Some(executor);
    }

    #[cfg(any(test, feature = "test-support"))]
    pub fn case_gesture_preview_active_test(&self) -> bool {
        self.case_gesture_preview.borrow().active_owner().is_some()
    }

    #[cfg(any(test, feature = "test-support"))]
    pub fn set_cad_scene_test(&self, scene: Option<Rc<CadScene>>) {
        *self.cad_scene.borrow_mut() = scene;
    }

    #[cfg(any(test, feature = "test-support"))]
    pub fn set_keycaps_preview_executor_test(&self, executor: Rc<KeycapsPreviewTestExecutor>) {
        *self.keycaps_preview_test_executor.borrow_mut() = Some(executor);
    }

    #[cfg(any(test, feature = "test-support"))]
    pub fn install_case_pcb_preview_test(&self) {
        let accepted = self.model().accepted.expect("accepted preview fixture");
        let scope = self.scope().expect("selected physical fixture scope");
        let generation = self.native_case_preview.borrow().generation + 1;
        let core = self.core.borrow().clone();
        let capture = crate::case_preview::capture_native_preview(
            &accepted,
            &scope,
            generation,
            generation,
            self.session.borrow().core_executor_epoch().0,
            core_executor_identity(&core),
            format!("case-keycaps-test-{generation}"),
        )
        .expect("valid accepted Case preview input");
        self.native_case_preview.borrow_mut().generation = generation;
        self.set_native_preview_pending(capture.owner.clone(), capture.lease.clone());
        let preview = crate::case_preview::accept_native_preview(
            capture,
            PcbPreview {
                revision: accepted.document.revision,
                thickness: 1.6,
                contours: vec![],
                surfaces: vec![],
                holes: vec![],
                models: vec![],
                diagnostics: vec![],
            },
        )
        .expect("controlled PCB payload preserves the accepted preview owner");
        self.native_case_preview
            .borrow_mut()
            .publish(preview)
            .unwrap();
        self.changed();
    }

    #[cfg(any(test, feature = "test-support"))]
    pub fn set_layout_component_inspector_test_state(
        &self,
        model: ReadModel,
        scope: Option<Scope>,
    ) {
        *self.layout_component_inspector_test_state.borrow_mut() = Some((model, scope));
    }

    #[cfg(any(test, feature = "test-support"))]
    pub fn take_layout_component_inspector_test_events(&self) -> Vec<Event> {
        std::mem::take(&mut *self.layout_component_inspector_test_events.borrow_mut())
    }

    #[cfg(any(test, feature = "test-support"))]
    pub fn settle_layout_component_inspector_test_operation(
        &self,
        operation: OperationId,
        outcome: TerminalOutcome,
    ) -> bool {
        self.operation_outcomes.settle(operation, outcome)
    }

    #[cfg(any(test, feature = "test-support"))]
    pub fn set_definition_name_test_generation(&self, generation: GenerationStatus) {
        *self.definition_name_test_generation.borrow_mut() = Some(generation);
    }

    #[cfg(any(test, feature = "test-support"))]
    fn set_firmware_export_test_context(
        &self,
        accepted: AcceptedSnapshot,
        scope: Option<Scope>,
        current_executor: Rc<dyn CoreExecutor>,
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

    #[cfg(any(test, feature = "test-support"))]
    fn replace_firmware_export_test_executor(
        &self,
        executor: Rc<dyn CoreExecutor>,
        epoch: boardstudio_application::ExecutorEpoch,
    ) {
        let mut context = self.firmware_export_test_context.borrow_mut();
        let context = context
            .as_mut()
            .expect("firmware export test context is installed");
        context.current_executor = executor;
        context.executor_epoch = epoch;
    }

    #[cfg(any(test, feature = "test-support"))]
    fn replace_firmware_export_test_owner(&self, accepted: AcceptedSnapshot, scope: Option<Scope>) {
        let mut context = self.firmware_export_test_context.borrow_mut();
        let context = context
            .as_mut()
            .expect("firmware export test context is installed");
        context.accepted = accepted;
        context.scope = scope;
    }

    #[cfg(any(test, feature = "test-support"))]
    fn take_firmware_export_test_effects(&self) -> Vec<Effect> {
        std::mem::take(&mut *self.firmware_export_test_effects.borrow_mut())
    }

    #[cfg(any(test, feature = "test-support"))]
    fn take_firmware_export_test_events(&self) -> Vec<Event> {
        std::mem::take(&mut *self.firmware_export_test_events.borrow_mut())
    }

    #[cfg(any(test, feature = "test-support"))]
    fn take_firmware_export_test_deliveries(&self) -> Vec<FirmwareTestDelivery> {
        std::mem::take(&mut *self.firmware_export_test_deliveries.borrow_mut())
    }

    #[cfg(any(test, feature = "test-support"))]
    pub fn take_definition_name_test_event(&self) -> Option<Event> {
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
            self.cancel_active_case_gesture_preview();
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
                let create_executor = self.core_executor_factory.borrow().clone();
                match create_executor() {
                    Ok(core) => {
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
                let persistence = self.persistence.borrow().clone();
                let result = match persistence.save_document(&document, &assets).await {
                    Ok(()) => {
                        for asset in &document.assets {
                            self.assets.borrow_mut().remove(&asset.sha256);
                        }
                        SaveResult::Committed
                    }
                    Err(error) => SaveResult::Aborted(error),
                };
                self.complete(Completion::Persist {
                    save_attempt_id,
                    result,
                })
            }
            Effect::Settled {
                operation_id,
                outcome,
                landing,
            } => {
                let observed = self.operation_outcomes.settle_with_landing(
                    operation_id,
                    outcome.clone(),
                    landing,
                );
                self.step_exports.borrow_mut().remove(&operation_id);
                let is_keycaps_step_export =
                    self.keycaps_step_exports.borrow_mut().remove(&operation_id);
                let firmware_capture = self.firmware_exports.borrow_mut().remove(&operation_id);
                self.footprint_exports.borrow_mut().remove(&operation_id);
                self.pcb_handoff_exports.borrow_mut().remove(&operation_id);
                self.mechanical_exports.borrow_mut().remove(&operation_id);
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
                let is_mechanical_export =
                    self.mechanical_exports.borrow().contains_key(&operation_id);
                let result = if is_pcb_handoff_export {
                    let draft = self
                        .pcb_handoff_exports
                        .borrow()
                        .get(&operation_id)
                        .is_some_and(|capture| capture.draft);
                    self.pcb_handoff_bytes(operation_id, &snapshot, &scope, draft)
                        .await
                } else if is_mechanical_export {
                    self.mechanical_package_bytes(operation_id, &snapshot, &scope)
                        .await
                        .map(|bytes| (bytes, snapshot.token))
                } else if is_footprint_export {
                    self.footprint_export_bytes(operation_id, &snapshot, &scope)
                        .await
                        .map(|bytes| (bytes, snapshot.token))
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
                        Ok(future) => future.await.map(|bytes| (bytes, snapshot.token)),
                        Err(reason) => Err(reason),
                    }
                };
                let result = if is_firmware_export {
                    let owner_is_current =
                        self.export_current(operation_id, snapshot.token, &scope);
                    let cancelled = self.cancelled_exports.borrow_mut().remove(&operation_id);
                    firmware_export_bytes_for_delivery(
                        result.map(|(bytes, _)| bytes),
                        owner_is_current,
                        cancelled,
                    )
                    .map(|bytes| (bytes, snapshot.token))
                } else {
                    result
                };
                match result {
                    Ok((bytes, delivery_token)) => {
                        let current = self.export_current(operation_id, delivery_token, &scope);
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
                            (
                                authored_case_filename(&snapshot.document, &scope),
                                Some("model/step".to_owned()),
                            )
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
                        } else if is_mechanical_export {
                            let filename = self
                                .mechanical_exports
                                .borrow()
                                .get(&operation_id)
                                .map(|capture| capture.filename.clone())
                                .unwrap_or_else(|| {
                                    format!("{}-mechanical.zip", snapshot.document.name)
                                });
                            (filename, Some("application/zip".to_owned()))
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
                                token: delivery_token,
                                firmware: is_firmware_export,
                                keycaps_step: is_keycaps_step_export,
                            },
                        );
                        self.complete(Completion::ExportFinished {
                            operation_id,
                            token: delivery_token,
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
                self.mechanical_exports.borrow_mut().remove(&operation_id);
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

    pub fn case_gesture_preview_scene(&self, source: &CadScene) -> Option<Rc<CadScene>> {
        let owner = self.case_gesture_preview.borrow().active_owner()?;
        if !self.case_gesture_preview_is_current(&owner)
            || owner.scope != source.scope
            || owner.snapshot_token != source.token
            || owner.revision != source.snapshot.document.revision
        {
            return None;
        }
        self.case_gesture_preview
            .borrow()
            .scene(&owner.scope, owner.snapshot_token, owner.revision)
    }

    pub fn case_gesture_preview_message(&self) -> Option<String> {
        let owner = self.case_gesture_preview.borrow().active_owner()?;
        self.case_gesture_preview_is_current(&owner)
            .then(|| self.case_gesture_preview.borrow().message(&owner))
            .flatten()
    }

    /// Start a disposable preview of one Case gesture draft. It uses the
    /// accepted scene and snapshot identity, but only the Case CAD preview
    /// result is overlaid; it never replaces the accepted generation cache.
    pub fn update_case_gesture_preview(
        self: &Rc<Self>,
        scope: Scope,
        token: SnapshotToken,
        revision: u64,
        draft_document: ProjectDoc,
    ) -> Option<crate::case_gesture_preview::CaseGesturePreviewOwner> {
        let accepted = self.model().accepted?;
        if self.scope().as_ref() != Some(&scope)
            || accepted.token != token
            || accepted.document.revision != revision
            || accepted.document.id != scope.document_id
            || accepted.session_epoch != scope.session_epoch
            || draft_document.id != accepted.document.id
            || draft_document.revision != revision
            || accepted.scene.revision != revision
        {
            return None;
        }
        // A newer pointer sample replaces the pending owner while retaining
        // the last accepted provisional scene for display. Retiring only the
        // worker job here also makes its eventual reply stale without ending
        // the gesture overlay itself.
        if let Some(previous) = self.case_gesture_preview.borrow().active_owner() {
            self.cancel_case_gesture_preview_job(&previous);
        }
        let owner = self
            .case_gesture_preview
            .borrow_mut()
            .begin(scope.clone(), token, revision)
            .ok()?;
        let job_id = format!("case-gesture-{}-{}-{}", token.0, revision, owner.generation);
        *self.case_gesture_preview_job.borrow_mut() = Some((owner.clone(), job_id.clone()));
        let mut draft_snapshot = accepted;
        draft_snapshot.document = std::sync::Arc::new(draft_document);
        self.changed();
        let runtime = Rc::downgrade(self);
        let worker_job_id = job_id.clone();
        let task_owner = owner.clone();
        spawn_local(async move {
            let Some(runtime) = runtime.upgrade() else {
                return;
            };
            let result = runtime
                .prepare_case_gesture_preview(
                    task_owner.clone(),
                    draft_snapshot,
                    worker_job_id.clone(),
                )
                .await;
            if runtime
                .case_gesture_preview_job
                .borrow()
                .as_ref()
                .is_some_and(|(active, active_job)| {
                    active == &task_owner && active_job == &worker_job_id
                })
            {
                runtime.case_gesture_preview_job.borrow_mut().take();
            }
            match result {
                Ok(scene) if runtime.case_gesture_preview_is_current(&task_owner) => {
                    runtime
                        .case_gesture_preview
                        .borrow_mut()
                        .publish(&task_owner, Rc::new(scene));
                    runtime.changed();
                }
                Err(error) if runtime.case_gesture_preview_is_current(&task_owner) => {
                    runtime
                        .case_gesture_preview
                        .borrow_mut()
                        .fail(&task_owner, error);
                    runtime.changed();
                }
                _ => {}
            }
        });
        Some(owner)
    }

    async fn prepare_case_gesture_preview(
        self: &Rc<Self>,
        owner: crate::case_gesture_preview::CaseGesturePreviewOwner,
        snapshot: AcceptedSnapshot,
        job_id: String,
    ) -> Result<CadScene, String> {
        let is_current = || self.case_gesture_preview_is_current(&owner);
        if !is_current() {
            return Err("Case gesture preview was superseded".into());
        }
        // Coalesce rapid pointer samples. A newer move advances the owner while
        // this delay is pending, so only the latest draft reaches Core/CAD.
        TimeoutFuture::new(45).await;
        if !is_current() {
            return Err("Case gesture preview was superseded".into());
        }
        #[cfg(any(test, feature = "test-support"))]
        {
            let executor = self.case_gesture_preview_test_executor.borrow().clone();
            if let Some(executor) = executor {
                return executor(snapshot, owner.scope.clone());
            }
        }
        let core = self.core.borrow().clone();
        let executor_epoch = self.session.borrow().core_executor_epoch().0.to_string();
        let prepared = prepare_captured_case(
            core.as_ref(),
            &executor_epoch,
            &job_id,
            &snapshot,
            &owner.scope,
        )
        .await
        .map_err(|error| format!("Case preview preparation failed: {error:?}"))?;
        if !is_current() {
            return Err("Case gesture preview was superseded".into());
        }
        let existing_worker = self
            .cad_worker
            .borrow()
            .as_ref()
            .filter(|(scope, worker)| scope == &owner.scope && !worker.is_closed())
            .map(|(_, worker)| worker.clone());
        let worker = if let Some(worker) = existing_worker {
            worker
        } else {
            if let Some((_, previous)) = self.cad_worker.borrow_mut().take() {
                previous.close();
            }
            let worker = Rc::new(
                CadWorker::new(
                    &resource_url("assets/cad-worker/entry.js")
                        .map_err(|error| error.to_string())?,
                )
                .map_err(|error| error.to_string())?,
            );
            *self.cad_worker.borrow_mut() = Some((owner.scope.clone(), worker.clone()));
            worker
        };
        worker.ready().await.map_err(|error| error.to_string())?;
        if !is_current() {
            let _ = worker.cancel(&job_id);
            return Err("Case gesture preview was superseded".into());
        }
        let request = CadRequest {
            request_id: format!("{job_id}-preview"),
            job_id: job_id.clone(),
            identity: prepared.identity.clone(),
            operation: CadOperation::Preview,
            prepared: Some(prepared.prepared.clone()),
            input_bytes: vec![],
        };
        let reply = worker
            .request(request.clone())
            .await
            .map_err(|error| error.to_string())?;
        if !is_current() {
            return Err("Case gesture preview was superseded".into());
        }
        let mut result = validate_reply(&request, reply, &prepared.identity)
            .map_err(|error| format!("Case preview CAD failed: {error:?}"))?;
        result.step.clear();
        let contours = captured_case_scene(&snapshot, &owner.scope)
            .map_err(|error| format!("Case preview contours failed: {error:?}"))?
            .board_contours
            .into_iter()
            .find(|board| board.board_id == owner.scope.board_id)
            .ok_or_else(|| "Case preview contours are unavailable".to_owned())?
            .contours;
        Ok(CadScene {
            scope: owner.scope.clone(),
            token: owner.snapshot_token,
            snapshot,
            result,
            prepared: prepared.prepared,
            physical_fingerprint: None,
            mechanical: prepared.mechanical_assembly,
            exact: false,
            contours,
        })
    }

    pub fn native_case_preview(&self) -> Option<Rc<crate::case_preview::NativePreviewSnapshot>> {
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
        let scope = self.scope()?;
        let accepted = self.model().accepted?;
        self.layout_preview
            .borrow()
            .error
            .as_ref()
            .filter(|(owner, _)| owner.matches_current(&accepted, &scope, owner.source_generation))
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
            .retire_unless_request_matches(expected);
        if retired {
            self.layout_model_rows.borrow_mut().take();
            self.changed();
        }
    }

    fn layout_source_owner_is_current(
        &self,
        owner: &crate::layout_viewer_source::LayoutSourceIdentity,
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
        let batch_generation = self
            .layout_model_batch_generation
            .get()
            .checked_add(1)
            .ok_or_else(|| "Layout model batch identity exhausted".to_owned())?;
        self.layout_model_batch_generation.set(batch_generation);
        let owner = crate::model_delivery::ModelOwnerIdentity::new_layout(
            preview.owner.scope.clone(),
            preview.owner.snapshot_token,
            preview.owner.source_generation,
            &preview.lease,
        );
        let batch = crate::model_delivery::ModelBatchIdentity::new(
            owner,
            preview.owner.accepted_revision,
            batch_generation,
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
            preview.owner.accepted_revision,
            preview.owner.source_generation
        );
        let core_executor_epoch = self.session.borrow().core_executor_epoch();
        let core_executor_epoch_text = core_executor_epoch.0.to_string();
        let core = self.core.borrow().clone();
        let module_request = CoreRequest::ResolveModules {
            id: module_request_id.clone(),
            document: preview.document.as_ref().clone(),
            board_id: preview.owner.scope.board_id.clone(),
            preview_top_z: Some(preview.preview.thickness),
        };
        let module_reply = core
            .request(
                &module_request_id,
                &core_executor_epoch_text,
                &module_request,
            )
            .await
            .map_err(|error| format!("Mounted-module model resolution failed: {error}"))?;
        if !source_is_current()
            || self.session.borrow().core_executor_epoch() != core_executor_epoch
            || !Rc::ptr_eq(&core, &self.core.borrow())
        {
            return Ok(());
        }
        let module_placements = match module_reply {
            CoreReply::ModulesResolved { id, result }
                if id == module_request_id
                    && result.revision == preview.owner.accepted_revision =>
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
        let unique_model_paths = preview
            .preview
            .models
            .iter()
            .map(|model| model.path.clone())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();
        let mut bundled_ids_by_path = BTreeMap::new();
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
            bundled_ids_by_path.extend(unique_model_paths.iter().cloned().zip(ids));
        }
        let selections = crate::model_delivery::resolve_preview_assets(
            &preview.preview.models,
            preview.board_reference.as_ref(),
            &native_paths,
            &preview.document,
            |path| bundled_ids_by_path.get(path).cloned().flatten(),
            |asset_id| {
                crate::bundled_models::bundled_model(asset_id).map(|model| {
                    crate::model_delivery::ResolvedModelAsset {
                        id: model.id.to_owned(),
                        sha256: model.sha256.to_owned(),
                        filename: model.filename.to_owned(),
                        source: crate::model_delivery::ModelAssetSource::Packaged {
                            url_path: model.url_path.to_owned(),
                        },
                    }
                })
            },
        )
        .into_iter()
        .collect::<BTreeMap<_, _>>();
        let ports = self.layout_model_delivery_ports(&preview, source_is_current.clone());
        let board_results = self
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
        let mut rows = board_results;
        if !module_placements.is_empty() {
            let selections = module_placements
                .iter()
                .map(|placement| {
                    let selection = crate::model_delivery::select_model_asset_id(
                        &placement.asset_id,
                        &preview.document.assets,
                        |asset_id| {
                            crate::bundled_models::bundled_model(asset_id).map(|model| {
                                crate::model_delivery::ResolvedModelAsset {
                                    id: model.id.to_owned(),
                                    sha256: model.sha256.to_owned(),
                                    filename: model.filename.to_owned(),
                                    source: crate::model_delivery::ModelAssetSource::Packaged {
                                        url_path: model.url_path.to_owned(),
                                    },
                                }
                            })
                        },
                    );
                    (placement.id.clone(), selection)
                })
                .collect::<BTreeMap<_, _>>();
            let module_results = self
                .case_model_delivery
                .deliver_module_placements(
                    preview.owner.accepted_revision,
                    &module_placements,
                    &selections,
                    &ports,
                    &batch,
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
            crate::layout_viewer_source::LayoutPreviewRequest::Authored(request) => {
                self.run_preview_pipeline(
                    request.as_ref().clone(),
                    operation,
                    capture.owner.scope.clone(),
                    capture.owner.source_generation,
                    is_current.clone(),
                )
                .await
            }
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

    async fn run_imported_layout_preview(
        &self,
        capture: &crate::layout_viewer_source::LayoutSourceCapture,
        asset: &boardstudio_core::model::Asset,
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
    async fn run_imported_board_preview(
        &self,
        asset: &boardstudio_core::model::Asset,
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
        let core = self.core.borrow().clone();
        let core_epoch = self.session.borrow().core_executor_epoch().0;
        if !is_current() {
            return Err("Imported board preview source became stale before Core dispatch".into());
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
            return Err("Imported board preview source changed during Core preview".into());
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
        let executor_epoch = self.session.borrow().core_executor_epoch();
        let core = self.core.borrow().clone();
        let request = ArtifactRequest::PreviewBoard {
            id: request_id.clone(),
            source,
            revision,
        };
        let reply = core
            .artifact(&request_id, &executor_epoch.0.to_string(), &request)
            .await
            .map_err(|error| format!("Could not preview routed KiCad board: {error}"))?;
        if self.session.borrow().core_executor_epoch() != executor_epoch
            || !Rc::ptr_eq(&core, &self.core.borrow())
        {
            return Err("Routed-board preview became stale when the Core worker changed".into());
        }
        match reply {
            ArtifactReply::PreviewBoard { id, result } if id == request_id => Ok(result),
            ArtifactReply::Error { id, error } if id == request_id => {
                let details = error
                    .diagnostics
                    .iter()
                    .map(|diagnostic| diagnostic.message.as_str())
                    .collect::<Vec<_>>();
                if details.is_empty() {
                    Err(error.message)
                } else {
                    Err(format!("{} {}", error.message, details.join(" ")))
                }
            }
            ArtifactReply::PreviewBoard { .. } | ArtifactReply::Error { .. } => {
                Err("Core returned a routed-board preview for another request".into())
            }
            _ => Err("Core returned an unexpected routed-board preview reply".into()),
        }
    }

    async fn load_preview_document_asset(
        &self,
        asset: &boardstudio_core::model::Asset,
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
        let digest = Sha256::digest(&bytes);
        let digest = digest
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        if digest != asset.sha256 {
            return Err("Imported board asset bytes do not match the accepted SHA-256".into());
        }
        self.assets
            .borrow_mut()
            .insert(asset.sha256.clone(), bytes.clone());
        Ok(bytes)
    }

    pub fn native_case_preview_pending(&self) -> bool {
        self.native_case_preview
            .borrow()
            .pending
            .as_ref()
            .is_some_and(|(owner, lease)| {
                lease.matches(owner) && self.preview_owner_is_current(owner)
            })
    }

    pub fn native_case_preview_error(&self) -> Option<String> {
        self.native_case_preview
            .borrow()
            .error
            .as_ref()
            .filter(|(owner, _)| self.preview_owner_is_current(owner))
            .map(|(_, error)| error.clone())
    }

    pub fn native_case_preview_key(&self) -> Option<(Scope, SnapshotToken, u64)> {
        let scope = self.scope()?;
        let accepted = self.model().accepted?;
        (scope.session_epoch == accepted.session_epoch
            && scope.document_id == accepted.document.id
            && accepted.scene.revision == accepted.document.revision)
            .then_some((scope, accepted.token, accepted.document.revision))
    }

    pub fn native_model_delivery(
        &self,
        preview: &crate::case_preview::NativePreviewSnapshot,
    ) -> Option<crate::model_delivery::ModelDeliveryRows> {
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
    pub async fn deliver_native_case_models(
        self: &Rc<Self>,
        preview: Rc<crate::case_preview::NativePreviewSnapshot>,
    ) -> Result<(), String> {
        if !self.native_preview_snapshot_is_current(&preview) {
            return Ok(());
        }
        let owner = crate::model_delivery::ModelOwnerIdentity::new(
            preview.owner.scope.clone(),
            preview.owner.snapshot_token,
            preview.owner.viewer_instance,
            preview.owner.projection_generation,
            &preview.lease,
        );
        let batch = crate::model_delivery::ModelBatchIdentity::new(
            owner.clone(),
            preview.owner.accepted_revision,
            preview.owner.batch_generation,
        );
        let native_paths = crate::model_delivery::native_model_path_assets(&preview.path_assets);
        let unique_model_paths = preview
            .preview
            .models
            .iter()
            .map(|model| model.path.clone())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();
        let Some(bundled_ids_by_path) = resolve_native_model_paths(
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
        let selections = crate::model_delivery::resolve_preview_assets(
            &preview.preview.models,
            preview.board_reference.as_ref(),
            &native_paths,
            &preview.accepted_document,
            |path| bundled_ids_by_path.get(path).cloned().flatten(),
            |asset_id| {
                crate::bundled_models::bundled_model(asset_id).map(|model| {
                    crate::model_delivery::ResolvedModelAsset {
                        id: model.id.to_owned(),
                        sha256: model.sha256.to_owned(),
                        filename: model.filename.to_owned(),
                        source: crate::model_delivery::ModelAssetSource::Packaged {
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
    ) -> Result<crate::model_delivery::MeshArrays, String> {
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
        Ok(crate::model_delivery::MeshArrays {
            positions: mesh.positions,
            normals: mesh.normals,
            colors: None,
        })
    }

    fn native_model_delivery_ports(
        self: &Rc<Self>,
        preview: &crate::case_preview::NativePreviewSnapshot,
        is_current: Rc<dyn Fn() -> bool>,
    ) -> crate::model_delivery::ModelDeliveryPorts {
        let scope = preview.owner.scope.clone();
        let token = preview.owner.snapshot_token;
        let viewer_instance = preview.owner.viewer_instance;
        let projection_generation = preview.owner.projection_generation;
        let lease = preview.lease.clone();
        let owner_is_current = Rc::new(move |owner: &crate::model_delivery::ModelOwnerIdentity| {
            owner.is_current_owner(
                &scope,
                token,
                viewer_instance,
                projection_generation,
                &lease,
            )
        });
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
        preview: &crate::layout_viewer_source::LayoutPreviewSnapshot,
        is_current: Rc<dyn Fn() -> bool>,
    ) -> crate::model_delivery::ModelDeliveryPorts {
        let scope = preview.owner.scope.clone();
        let token = preview.owner.snapshot_token;
        let source_generation = preview.owner.source_generation;
        let lease = preview.lease.clone();
        let owner_is_current = Rc::new(move |owner: &crate::model_delivery::ModelOwnerIdentity| {
            owner.is_current_layout_owner(&scope, token, source_generation, &lease)
        });
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
        owner_is_current: Rc<dyn Fn(&crate::model_delivery::ModelOwnerIdentity) -> bool>,
    ) -> crate::model_delivery::ModelDeliveryPorts {
        use crate::model_delivery::{
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
                  owner: crate::model_delivery::ModelOwnerIdentity|
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

    pub async fn prepare_native_case_preview(
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
        let core_worker_identity = core_executor_identity(&core);
        let capture = match crate::case_preview::capture_native_preview(
            &accepted,
            &expected_scope,
            generation,
            generation,
            core_executor_epoch,
            core_worker_identity,
            request_token.clone(),
        ) {
            Ok(capture) => capture,
            Err(error) => {
                let owner = crate::case_preview::CasePreviewOwnerIdentity::capture(
                    &accepted,
                    &expected_scope,
                    generation,
                    generation,
                    core_executor_epoch,
                    core_worker_identity,
                    request_token,
                );
                self.native_case_preview.borrow_mut().error = Some((owner, error.clone()));
                self.changed();
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
    pub async fn prepare_parts_library_preview(
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

        let native_paths = crate::model_delivery::native_model_path_assets(&capture.path_assets);
        let unique_model_paths = preview
            .models
            .iter()
            .map(|model| model.path.clone())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();
        let mut bundled_ids_by_path = BTreeMap::new();
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
            bundled_ids_by_path.extend(unique_model_paths.iter().cloned().zip(ids));
        }
        let selections = crate::model_delivery::resolve_preview_assets(
            &preview.models,
            None,
            &native_paths,
            &capture.sample_document,
            |path| bundled_ids_by_path.get(path).cloned().flatten(),
            |asset_id| {
                crate::bundled_models::bundled_model(asset_id).map(|model| {
                    crate::model_delivery::ResolvedModelAsset {
                        id: model.id.to_owned(),
                        sha256: model.sha256.to_owned(),
                        filename: model.filename.to_owned(),
                        source: crate::model_delivery::ModelAssetSource::Packaged {
                            url_path: model.url_path.to_owned(),
                        },
                    }
                })
            },
        )
        .into_iter()
        .collect::<BTreeMap<_, _>>();
        let owner = crate::model_delivery::ModelOwnerIdentity::new_parts(
            capture.owner.scope.clone(),
            capture.owner.snapshot_token,
            capture.owner.source_generation,
            &capture.lease,
        );
        let batch = crate::model_delivery::ModelBatchIdentity::new(
            owner.clone(),
            capture.owner.accepted_revision,
            capture.owner.source_generation,
        );
        let owner_scope = capture.owner.scope.clone();
        let owner_token = capture.owner.snapshot_token;
        let source_generation = capture.owner.source_generation;
        let lease = capture.lease.clone();
        let owner_is_current = Rc::new(
            move |candidate: &crate::model_delivery::ModelOwnerIdentity| {
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

    pub fn parts_preview_snapshot_is_current(
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
                core_executor_identity(&core),
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
        let is_current: Rc<dyn Fn() -> bool> = Rc::new(move || {
            weak.upgrade().is_some_and(|runtime| {
                runtime.preview_owner_is_current(&owner)
                    && runtime.preview_owner_lease_is_current(&owner)
                    && lease.matches(&owner)
            })
        });
        let preview = match &capture.request {
            crate::case_preview::CasePreviewRequest::Authored(request) => {
                self.run_preview_pipeline(
                    request.as_ref().clone(),
                    operation,
                    capture.owner.scope.clone(),
                    capture.owner.projection_generation,
                    is_current,
                )
                .await?
            }
            crate::case_preview::CasePreviewRequest::Imported { asset, .. } => {
                let request_id = format!("case-preview-{operation}-imported");
                self.run_imported_board_preview(asset, request_id.clone(), is_current, |source| {
                    capture.imported_artifact_request(request_id, source)
                })
                .await?
            }
        };
        crate::case_preview::accept_native_preview(capture, preview)
    }

    /// One Core request shared by canonical Layout
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
        let ensure_current = || -> Result<(), String> {
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
        let preview_id = format!("board-preview-{operation}");
        let preview_request = boardstudio_core::model::ArtifactRequest::PreviewPcb {
            id: preview_id.clone(),
            request: request.clone(),
        };
        let reply = core
            .artifact(&preview_id, &epoch, &preview_request)
            .await
            .map_err(|error| format!("Core board preview failed: {error}"))?;
        ensure_current()?;
        let preview = match reply {
            ArtifactReply::PreviewBoard { id, result } if id == preview_id => result,
            ArtifactReply::Error { id, error } if id == preview_id => {
                return Err(format!("Core rejected the board preview: {error:?}"));
            }
            ArtifactReply::PreviewBoard { .. } | ArtifactReply::Error { .. } => {
                return Err("Core returned a board preview for another request".into());
            }
            _ => return Err("Core returned an unexpected board preview reply".into()),
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
        boardstudio_web_host::cad_jobs::captured_case_document(accepted, &scope).is_ok_and(
            |document| {
                document.mechanical.as_ref().is_some_and(|configuration| {
                    configuration.board_id == scope.board_id
                        && configuration.closure_mounts.is_none()
                        && configuration.mount != boardstudio_core::model::MechanicalMount::Gasket
                })
            },
        )
    }

    pub fn export_step(self: &Rc<Self>) {
        let Some(scope) = self.scope() else {
            self.apply_report(RuntimeReport::alert(
                "Select a board before exporting authored case geometry.",
            ));
            return;
        };
        let model = self.model();
        let Some(snapshot) = model.accepted.as_ref() else {
            self.apply_report(RuntimeReport::alert(
                "Authored Case STEP requires a ready accepted snapshot.",
            ));
            return;
        };
        if model.active_board_id != scope.board_id
            || snapshot.scene.revision != snapshot.document.revision
            || !snapshot
                .document
                .boards
                .iter()
                .any(|board| board.id == scope.board_id)
            || !snapshot
                .document
                .case_bodies
                .iter()
                .any(|body| body.board_id == scope.board_id)
            || !boardstudio_core::authored_case_geometry_ready(
                &snapshot.document,
                &snapshot.scene,
                &scope.board_id,
            )
        {
            self.apply_report(RuntimeReport::alert(
                "Resolve authored case findings for the selected board before export.",
            ));
            return;
        }
        self.clear_alert();
        let operation_id = self.operation();
        self.step_exports.borrow_mut().insert(operation_id);
        self.submit(Event::StartExport {
            operation_id,
            scope,
        });
    }

    pub fn export_mechanical(self: &Rc<Self>) {
        if self.mechanical_mount_initialization_pending() {
            self.apply_report(RuntimeReport::alert(
                "Finish preparing mounting locations before exporting the mechanical assembly.",
            ));
            return;
        }
        let Some(scope) = self.scope() else {
            self.apply_report(RuntimeReport::alert(
                "Select a physical board before exporting the mechanical assembly.",
            ));
            return;
        };
        let model = self.model();
        let Some(snapshot) = model.accepted else {
            self.apply_report(RuntimeReport::alert(
                "Mechanical export requires a ready accepted snapshot.",
            ));
            return;
        };
        if model.active_board_id != scope.board_id
            || snapshot.scene.revision != snapshot.document.revision
        {
            self.apply_report(RuntimeReport::alert(
                "The selected board is still resolving; export the current revision again.",
            ));
            return;
        }
        let document = match captured_case_document(&snapshot, &scope) {
            Ok(document) => document,
            Err(error) => {
                self.apply_report(RuntimeReport::alert(format!(
                    "Could not capture the selected mechanical configuration: {error:?}"
                )));
                return;
            }
        };
        let Some(configuration) = document.mechanical.as_ref() else {
            self.apply_report(RuntimeReport::alert(
                "Enable a mechanical assembly for the selected board before export.",
            ));
            return;
        };
        if configuration.board_id != scope.board_id {
            self.apply_report(RuntimeReport::alert(
                "The selected mechanical configuration belongs to another board.",
            ));
            return;
        }
        let outline_ready = snapshot
            .scene
            .board_readiness
            .iter()
            .find(|item| item.board_id == scope.board_id)
            .map_or(
                snapshot.document.boards.len() <= 1 && snapshot.scene.readiness.outline,
                |item| item.outline,
            );
        if !outline_ready {
            self.apply_report(RuntimeReport::alert(
                "Resolve active outline findings before exporting plate or case artifacts.",
            ));
            return;
        }
        let exact_generation = matches!(
            model.generation,
            boardstudio_application::GenerationStatus::Ready { exact: true, .. }
        );
        let exact_scene_ready = self.cad_scene().is_some_and(|cad| {
            cad.scope == scope
                && cad.token == snapshot.token
                && cad.exact
                && cad.mechanical.as_ref().is_some_and(|assembly| {
                    assembly.revision == snapshot.document.revision
                        && !assembly.generation_blocked
                        && !assembly.diagnostics.iter().any(|finding| {
                            finding.severity == boardstudio_core::model::Severity::Error
                        })
                })
        });
        if !exact_generation || !exact_scene_ready {
            self.apply_report(RuntimeReport::alert(
                "Update the current mechanical preview before export.",
            ));
            return;
        }
        let instance_suffix = match scope.instance_id.as_deref() {
            Some(instance_id) => {
                let Some(instance) = snapshot.document.hardware.as_ref().and_then(|hardware| {
                    hardware
                        .instances
                        .iter()
                        .find(|item| item.id == instance_id && item.board_id == scope.board_id)
                }) else {
                    self.apply_report(RuntimeReport::alert(
                        "The selected mechanical instance is no longer in this project.",
                    ));
                    return;
                };
                format!("-{}", mechanical_filename_component(&instance.name))
            }
            None => String::new(),
        };
        self.clear_alert();
        let operation_id = self.operation();
        let core = self.core.borrow().clone();
        self.mechanical_exports.borrow_mut().insert(
            operation_id,
            MechanicalExportCapture {
                scope: scope.clone(),
                token: snapshot.token,
                revision: snapshot.document.revision,
                session_epoch: snapshot.session_epoch,
                document_id: snapshot.document.id.clone(),
                executor_epoch: self.session.borrow().core_executor_epoch(),
                core_worker_identity: core_executor_identity(&core),
                filename: format!(
                    "{}{}-mechanical.zip",
                    snapshot.document.name, instance_suffix
                ),
            },
        );
        self.submit(Event::StartExport {
            operation_id,
            scope,
        });
    }

    pub fn export_keycaps_step(self: &Rc<Self>) {
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
    pub fn export_firmware(self: &Rc<Self>) {
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

    pub fn export_footprints(self: &Rc<Self>) {
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
                core_worker_identity: core_executor_identity(&core),
            },
        );
        self.submit(Event::StartExport {
            operation_id,
            scope,
        });
    }

    pub fn export_kicad_board(self: &Rc<Self>, draft: bool) {
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
            core_worker_identity: core_executor_identity(&core),
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
    ) -> Result<(Vec<u8>, SnapshotToken), String> {
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
        let (archive, protected_capture) = crate::pcb_handoff::package_then_protect(
            {
                let runtime = self.clone();
                let package_capture = capture.clone();
                let package_core = core.clone();
                let package_plan = plan.clone();
                async move {
                    let is_current = || {
                        if runtime.pcb_handoff_capture_is_current(
                            operation_id,
                            &package_capture,
                            &package_core,
                        ) {
                            Ok(())
                        } else {
                            Err("KiCad export was cancelled, superseded, or its accepted source changed."
                                .into())
                        }
                    };
                    crate::pcb_handoff::build_handoff(
                        crate::pcb_handoff::HandoffSource {
                            operation_id,
                            snapshot: &snapshot,
                            scope,
                            electrical_plan: package_plan,
                            populations,
                            draft,
                        },
                        crate::pcb_handoff::HandoffPorts {
                            core: package_core.as_ref(),
                            store: &runtime.store,
                            executor_epoch: package_capture.executor_epoch.0,
                        },
                        is_current,
                    )
                    .await
                }
            },
            || {
                if self.pcb_handoff_capture_is_current(operation_id, &capture, &core) {
                    Ok(())
                } else {
                    Err("KiCad export was superseded before wiring protection.".into())
                }
            },
            || {
                self.commit_pcb_handoff(
                    operation_id,
                    &capture,
                    boardstudio_application::ExportCommitRequest::ProtectElectricalHandoff {
                        plan,
                    },
                )
            },
            |protected_capture| {
                if self.pcb_handoff_capture_is_current(operation_id, protected_capture, &core) {
                    Ok(())
                } else {
                    Err("KiCad handoff was superseded before delivery.".into())
                }
            },
        )
        .await?;
        capture = protected_capture;
        Ok((archive, capture.token))
    }

    async fn mechanical_package_bytes(
        self: &Rc<Self>,
        operation_id: OperationId,
        initial_snapshot: &AcceptedSnapshot,
        scope: &Scope,
    ) -> Result<Vec<u8>, String> {
        let capture = self
            .mechanical_exports
            .borrow()
            .get(&operation_id)
            .cloned()
            .ok_or_else(|| "Mechanical export owner was cancelled or superseded.".to_owned())?;
        if capture.scope != *scope
            || capture.token != initial_snapshot.token
            || capture.revision != initial_snapshot.document.revision
            || capture.session_epoch != initial_snapshot.session_epoch
            || capture.document_id != initial_snapshot.document.id
        {
            return Err("Mechanical export no longer matches its captured project.".into());
        }
        let core = self.core.borrow().clone();
        let runtime = self.clone();
        let ensure_current = || -> Result<(), String> {
            if runtime.mechanical_export_capture_is_current(operation_id, &capture, &core) {
                Ok(())
            } else {
                Err("Mechanical export was cancelled, superseded, or its source changed.".into())
            }
        };
        ensure_current()?;
        let document = captured_case_document(initial_snapshot, scope)
            .map_err(|error| format!("Could not capture mechanical document: {error:?}"))?;
        let scene = captured_case_scene(initial_snapshot, scope)
            .map_err(|error| format!("Could not capture mechanical scene: {error:?}"))?;
        let configuration = document
            .mechanical
            .as_ref()
            .filter(|configuration| configuration.board_id == scope.board_id)
            .ok_or_else(|| {
                "Enable a mechanical assembly for the selected board before export.".to_owned()
            })?;
        let outline_ready = initial_snapshot
            .scene
            .board_readiness
            .iter()
            .find(|item| item.board_id == scope.board_id)
            .map_or(
                initial_snapshot.document.boards.len() <= 1
                    && initial_snapshot.scene.readiness.outline,
                |item| item.outline,
            );
        if !outline_ready {
            return Err(
                "Resolve active outline findings before exporting plate or case artifacts.".into(),
            );
        }
        let contours = scene
            .board_contours
            .iter()
            .find(|item| item.board_id == scope.board_id)
            .map(|item| item.contours.clone())
            .ok_or_else(|| "The selected board has no resolved outline contours.".to_owned())?;
        let executor_epoch = capture.executor_epoch.0.to_string();
        let prepared = prepare_captured_step_assembly(
            core.as_ref(),
            &executor_epoch,
            &format!("mechanical-export-{}", operation_id.0),
            initial_snapshot,
            scope,
        )
        .await
        .map_err(|error| format!("Mechanical assembly resolution failed: {error:?}"))?;
        ensure_current()?;
        let assembly = prepared
            .mechanical_assembly
            .as_ref()
            .ok_or_else(|| "Core did not resolve a generated mechanical assembly.".to_owned())?;
        if assembly.revision != capture.revision
            || assembly.case.revision != capture.revision
            || assembly.generation_blocked
        {
            return Err("Mechanical assembly is blocked or belongs to another revision.".into());
        }
        let errors = assembly
            .diagnostics
            .iter()
            .filter(|finding| finding.severity == boardstudio_core::model::Severity::Error)
            .map(|finding| finding.message.clone())
            .collect::<Vec<_>>();
        if !errors.is_empty() {
            return Err(errors.join("\n"));
        }

        let mut files: Vec<(String, Vec<u8>)> = Vec::new();
        let mut paths = BTreeSet::new();
        let plate_method = configuration
            .part_processes
            .as_ref()
            .and_then(|processes| processes.iter().find(|process| process.part_id == "plate"))
            .map(|process| &process.method)
            .unwrap_or(&configuration.method);
        if matches!(plate_method, boardstudio_core::model::PlateMethod::PcbFr4) {
            let request_id = format!("mechanical-plate-{}", operation_id.0);
            let request = ArtifactRequest::ExportMechanicalPlate {
                id: request_id.clone(),
                document: document.clone(),
                contours: contours.clone(),
            };
            let reply = core
                .artifact(&request_id, &executor_epoch, &request)
                .await
                .map_err(|error| format!("Mechanical plate export failed: {error}"))?;
            ensure_current()?;
            let result = match reply {
                ArtifactReply::ExportMechanicalPlate { id, result } if id == request_id => result,
                ArtifactReply::Error { id, error } if id == request_id => {
                    return Err(format!("Mechanical plate export failed: {}", error.message));
                }
                _ => return Err("Core returned an unexpected mechanical plate artifact.".into()),
            };
            if result.revision != capture.revision {
                return Err("Core returned a mechanical plate for another revision.".into());
            }
            for file in result.files {
                push_mechanical_file(
                    &mut files,
                    &mut paths,
                    format!("plate-kicad/{}", file.filename),
                    file.content.into_bytes(),
                )?;
            }
        }

        let worker_url = resource_url("assets/cad-worker/entry.js")?;
        let worker = Rc::new(CadWorker::new(&worker_url).map_err(|error| error.to_string())?);
        self.export_workers
            .borrow_mut()
            .insert(operation_id, worker.clone());
        let cad_output = async {
            worker.ready().await.map_err(|error| error.to_string())?;
            ensure_current()?;
            let assembled = request_exact_cad(
                &worker,
                operation_id,
                "assembly",
                &prepared.identity,
                prepared.prepared.clone(),
            )
            .await?;
            ensure_current()?;
            if assembled.step.is_empty() {
                return Err("CAD returned no assembled STEP data.".into());
            }
            push_mechanical_file(
                &mut files,
                &mut paths,
                "assembly.step".into(),
                assembled.step,
            )?;

            let board = document
                .boards
                .iter()
                .find(|board| board.id == scope.board_id)
                .ok_or_else(|| "The selected board is no longer available.".to_owned())?;
            for (index, source) in assembly.case.bodies.iter().enumerate() {
                ensure_current()?;
                let prepared_body = prepared
                    .prepared
                    .bodies
                    .iter()
                    .find(|body| body.body.id == source.body.id)
                    .cloned()
                    .ok_or_else(|| format!("Prepared CAD body '{}' is missing.", source.body.id))?;
                let part_name = format!(
                    "{}-{}",
                    index + 1,
                    mechanical_filename_component(&source.body.name)
                );
                let result = request_exact_cad(
                    &worker,
                    operation_id,
                    &format!("part-{index}"),
                    &prepared.identity,
                    boardstudio_core::model::PreparedCaseAssemblyIR {
                        revision: prepared.identity.revision,
                        bodies: vec![prepared_body],
                    },
                )
                .await?;
                ensure_current()?;
                let mesh = result
                    .mesh
                    .as_ref()
                    .ok_or_else(|| format!("CAD returned no mesh for {part_name}."))?;
                push_mechanical_file(
                    &mut files,
                    &mut paths,
                    format!("parts/{part_name}.step"),
                    result.step,
                )?;
                push_mechanical_file(
                    &mut files,
                    &mut paths,
                    format!("parts/{part_name}.stl"),
                    mechanical_stl(mesh)?,
                )?;

                let mut outline_contours = source.contours.clone();
                for mount in source.body.mounts.iter().flatten() {
                    outline_contours.push(boardstudio_core::model::Contour {
                        hole: true,
                        points: (0..96)
                            .map(|point| {
                                let angle = point as f64 * std::f64::consts::TAU / 96.0;
                                boardstudio_core::model::Vec2 {
                                    x: mount.at.x + mount.hole_diameter / 2.0 * angle.cos(),
                                    y: mount.at.y + mount.hole_diameter / 2.0 * angle.sin(),
                                }
                            })
                            .collect(),
                    });
                }
                let mut export_board = board.clone();
                export_board.id = source.body.id.clone();
                export_board.name = source.body.name.clone();
                export_board.outline_ids.clear();
                export_board.part_ids.clear();
                export_board.net_ids.clear();
                export_board.traces.clear();
                export_board.vias.clear();
                for (extension, format) in [
                    ("dxf", boardstudio_core::model::OutlineExportFormat::Dxf),
                    ("svg", boardstudio_core::model::OutlineExportFormat::Svg),
                ] {
                    let filename = format!("{part_name}.{extension}");
                    let request_id =
                        format!("mechanical-outline-{}-{index}-{extension}", operation_id.0);
                    let request = ArtifactRequest::ExportOutline {
                        id: request_id.clone(),
                        request: boardstudio_core::model::OutlineExportRequest {
                            filename: filename.clone(),
                            board: export_board.clone(),
                            contours: outline_contours.clone(),
                            format,
                        },
                    };
                    let reply = core
                        .artifact(&request_id, &executor_epoch, &request)
                        .await
                        .map_err(|error| format!("Mechanical outline export failed: {error}"))?;
                    ensure_current()?;
                    let file = match reply {
                        ArtifactReply::ExportOutline { id, result } if id == request_id => result,
                        ArtifactReply::Error { id, error } if id == request_id => {
                            return Err(format!(
                                "Mechanical outline export failed: {}",
                                error.message
                            ));
                        }
                        _ => return Err("Core returned an unexpected mechanical outline.".into()),
                    };
                    if file.filename != filename {
                        return Err("Core returned a mechanical outline under another name.".into());
                    }
                    push_mechanical_file(
                        &mut files,
                        &mut paths,
                        format!("outlines/{}", file.filename),
                        file.content.into_bytes(),
                    )?;
                }
            }
            Ok::<(), String>(())
        }
        .await;
        worker.close();
        self.export_workers.borrow_mut().remove(&operation_id);
        cad_output?;
        ensure_current()?;

        push_mechanical_file(
            &mut files,
            &mut paths,
            "FABRICATION.md".into(),
            mechanical_fabrication_notes(&document, assembly).into_bytes(),
        )?;
        push_mechanical_file(
            &mut files,
            &mut paths,
            "critical-fit.svg".into(),
            critical_fit_drawing(assembly, configuration)?.into_bytes(),
        )?;
        let assembly_json = serde_json::json!({
            "revision": capture.revision,
            "pcbReference": "Nominal unpopulated PCB only; component solids are not included",
            "stack": assembly.stack,
            "diagnostics": assembly.diagnostics,
        });
        push_mechanical_file(
            &mut files,
            &mut paths,
            "assembly.json".into(),
            serde_json::to_vec_pretty(&assembly_json)
                .map_err(|error| format!("Could not serialize mechanical assembly: {error}"))?,
        )?;
        ensure_current()?;
        let entries = files
            .iter()
            .enumerate()
            .map(|(index, (path, _))| {
                Ok(ArchiveEntry {
                    path: path.clone(),
                    buffer_index: u32::try_from(index)
                        .map_err(|_| "Too many mechanical package files.".to_owned())?,
                })
            })
            .collect::<Result<Vec<_>, String>>()?;
        let metadata = serde_json::to_string(&ArchiveRequest::PackFiles { entries })
            .map_err(|error| format!("Could not prepare mechanical ZIP request: {error}"))?;
        let buffers = files
            .iter()
            .map(|(_, contents)| Uint8Array::from(contents.as_slice()))
            .collect();
        let archive_id = format!("mechanical-archive-{}", operation_id.0);
        let packed = core
            .archive(&archive_id, &executor_epoch, &metadata, buffers)
            .await
            .map_err(|error| format!("Mechanical packaging failed: {error}"))?;
        ensure_current()?;
        match serde_json::from_str::<ArchiveReply>(&packed.metadata)
            .map_err(|error| format!("Could not read mechanical ZIP result: {error}"))?
        {
            ArchiveReply::Packed => packed
                .buffers
                .first()
                .map(Uint8Array::to_vec)
                .filter(|bytes| !bytes.is_empty())
                .ok_or_else(|| "Mechanical ZIP provider returned no bytes.".into()),
            ArchiveReply::Error { message } => {
                Err(format!("Mechanical packaging failed: {message}"))
            }
            ArchiveReply::Unpacked { .. } => {
                Err("Core returned an unpacked project for mechanical export.".into())
            }
        }
    }

    fn mechanical_export_capture_is_current(
        &self,
        operation_id: OperationId,
        capture: &MechanicalExportCapture,
        core: &Rc<dyn CoreExecutor>,
    ) -> bool {
        let current_core = self.core.borrow().clone();
        self.mechanical_exports.borrow().get(&operation_id) == Some(capture)
            && self.export_current(operation_id, capture.token, &capture.scope)
            && self.scope().as_ref() == Some(&capture.scope)
            && self.model().accepted.as_ref().is_some_and(|snapshot| {
                snapshot.token == capture.token
                    && snapshot.document.id == capture.document_id
                    && snapshot.document.revision == capture.revision
                    && snapshot.scene.revision == capture.revision
                    && snapshot.session_epoch == capture.session_epoch
            })
            && self.session.borrow().core_executor_epoch() == capture.executor_epoch
            && core_executor_identity(&current_core) == capture.core_worker_identity
            && Rc::ptr_eq(core, &current_core)
    }

    async fn resolve_pcb_handoff_plan(
        &self,
        operation_id: OperationId,
        snapshot: &AcceptedSnapshot,
        scope: &Scope,
        instance_id: Option<&str>,
        core: &Rc<dyn CoreExecutor>,
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
        core: &Rc<dyn CoreExecutor>,
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
            && core_executor_identity(&current_core) == capture.core_worker_identity
            && Rc::ptr_eq(core, &current_core)
    }

    fn require_pcb_handoff_current(
        &self,
        operation_id: OperationId,
        capture: &PcbHandoffCapture,
        core: &Rc<dyn CoreExecutor>,
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
        crate::export_footprints::build_zip(
            crate::export_footprints::ExportSource {
                operation_id,
                snapshot,
                core: core.as_ref(),
                store: &self.store,
                executor_epoch: capture.executor_epoch.0,
            },
            ensure_current,
        )
        .await
    }

    fn footprint_export_capture_is_current(
        &self,
        operation_id: OperationId,
        capture: &FootprintExportCapture,
        core: &Rc<dyn CoreExecutor>,
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
            && core_executor_identity(&current_core) == capture.core_worker_identity
            && Rc::ptr_eq(core, &current_core)
    }

    pub fn export_project_copy(self: &Rc<Self>) {
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

    pub fn export_board_outline(
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
        #[cfg(any(test, feature = "test-support"))]
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
    ) -> (Rc<dyn CoreExecutor>, boardstudio_application::ExecutorEpoch) {
        #[cfg(any(test, feature = "test-support"))]
        if let Some(context) = self.firmware_export_test_context.borrow().as_ref() {
            return (context.current_executor.clone(), context.executor_epoch);
        }
        let core = self.core.borrow().clone();
        let executor: Rc<dyn CoreExecutor> = core;
        (executor, self.session.borrow().core_executor_epoch())
    }
    fn deliver_artifact(&self, artifact: &Artifact) -> Result<(), String> {
        #[cfg(any(test, feature = "test-support"))]
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
            core.as_ref(),
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
        let core = self.core.borrow().clone();
        let executor_epoch = self.session.borrow().core_executor_epoch();
        let guard = || -> Result<(), String> {
            let current_core = self.core.borrow().clone();
            let current = self.model().accepted;
            if self.export_current(operation_id, snapshot.token, scope)
                && self.scope().as_ref() == Some(scope)
                && self.session.borrow().core_executor_epoch() == executor_epoch
                && Rc::ptr_eq(&core, &current_core)
                && current.as_ref().is_some_and(|accepted| {
                    accepted.token == snapshot.token
                        && accepted.session_epoch == snapshot.session_epoch
                        && accepted.document.id == snapshot.document.id
                        && accepted.document.revision == snapshot.document.revision
                        && accepted.scene.revision == snapshot.document.revision
                })
            {
                Ok(())
            } else {
                Err("Authored Case STEP was cancelled, superseded, or its source changed.".into())
            }
        };
        guard()?;
        let bodies = snapshot
            .document
            .case_bodies
            .iter()
            .filter(|body| body.board_id == scope.board_id)
            .cloned()
            .map(|body| CaseIR {
                revision: snapshot.document.revision,
                body,
                contours: snapshot
                    .scene
                    .board_contours
                    .iter()
                    .find(|entry| entry.board_id == scope.board_id)
                    .map_or_else(Vec::new, |entry| entry.contours.clone()),
            })
            .collect::<Vec<_>>();
        if bodies.is_empty() {
            return Err("The selected board has no saved authored case bodies.".into());
        }
        let request_id = format!("authored-case-step-{}", operation_id.0);
        let request = CoreRequest::PrepareCase {
            id: request_id.clone(),
            ir: CaseAssemblyIR {
                revision: snapshot.document.revision,
                bodies,
            },
        };
        let prepared = core
            .request(&request_id, &executor_epoch.0.to_string(), &request)
            .await
            .map_err(|error| format!("Authored case preparation failed: {error}"))?;
        guard()?;
        let prepared = match prepared {
            CoreReply::CasePrepared { id, ir } if id == request_id => ir,
            CoreReply::Error { id, message, .. } if id == request_id => return Err(message),
            _ => return Err("Core returned an unexpected authored case preparation.".into()),
        };
        if prepared.revision != snapshot.document.revision
            || prepared
                .bodies
                .iter()
                .any(|body| body.revision != snapshot.document.revision)
        {
            return Err("Core prepared authored case bodies for another revision.".into());
        }
        if prepared.bodies.is_empty() {
            return Err("Core prepared no authored case bodies.".into());
        }
        let identity = CadSnapshotIdentity {
            token: snapshot.token.0,
            session_epoch: snapshot.session_epoch.0,
            document_id: snapshot.document.id.clone(),
            board_id: scope.board_id.clone(),
            instance_id: scope.instance_id.clone(),
            revision: snapshot.document.revision,
        };
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
                request_id: format!("authored-case-step-{}", operation_id.0),
                job_id: format!("authored-case-step-{}", operation_id.0),
                identity: identity.clone(),
                operation: CadOperation::ExportStep,
                prepared: Some(prepared),
                input_bytes: vec![],
            };
            let reply = worker
                .request(request.clone())
                .await
                .map_err(|e| e.to_string())?;
            guard()?;
            validate_reply(&request, reply, &identity)
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
        core: &Rc<dyn CoreExecutor>,
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
        core: Option<&Rc<dyn CoreExecutor>>,
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
    pub async fn fixture_document(&self, name: &'static str) -> Result<ProjectDoc, String> {
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
            #[cfg(any(test, feature = "test-support"))]
            if let Some(done) = this.import_file_test_done.borrow_mut().take() {
                let _ = done.send(());
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
        #[cfg(any(test, feature = "test-support"))]
        let test_gate = { self.import_archive_test_gate.borrow_mut().take() };
        #[cfg(any(test, feature = "test-support"))]
        let result = if let Some(gate) = test_gate {
            let _ = gate.entered.send(());
            let _ = gate.release.await;
            Ok(boardstudio_web_host::host::ArchiveResult {
                metadata: gate.metadata,
                buffers: gate
                    .buffers
                    .into_iter()
                    .map(|buffer| Uint8Array::from(buffer.as_slice()))
                    .collect(),
            })
        } else {
            core.archive(
                &format!("import-{}", operation.0),
                "1",
                "{\"kind\":\"unpack-project\"}",
                vec![Uint8Array::from(bytes.as_slice())],
            )
            .await
            .map_err(|e| e.to_string())
        };
        #[cfg(not(any(test, feature = "test-support")))]
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
            let loaded = this.store.load_document(id).await;
            #[cfg(any(test, feature = "test-support"))]
            let loaded = {
                let gate = { this.open_saved_load_test_gate.borrow_mut().take() };
                if let Some(gate) = gate {
                    let _ = gate.entered.send(());
                    let _ = gate.release.await;
                }
                loaded
            };
            match loaded {
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
            #[cfg(any(test, feature = "test-support"))]
            if let Some(done) = this.open_saved_test_done.borrow_mut().take() {
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
        crate::model_delivery::ModelDeliveryRows,
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
            Err("Core returned an incomplete model path mapping".into())
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
mod case_preview_source_tests {
    use super::*;
    use wasm_bindgen_test::wasm_bindgen_test;

    #[wasm_bindgen_test]
    async fn imported_capture_failure_is_visible_only_to_its_current_case_owner() {
        let runtime = project_name_test_support::new_runtime();
        let mut document = firmware_export_test_support::board_document();
        document.boards.push(Board {
            id: "another-board".into(),
            name: "Another board".into(),
            outline_ids: Vec::new(),
            part_ids: Vec::new(),
            net_ids: Vec::new(),
            thickness: 1.6,
            traces: Vec::new(),
            vias: Vec::new(),
        });
        document
            .board_references
            .push(boardstudio_core::model::BoardReference {
                id: "missing-routed-source".into(),
                board_id: document.boards[0].id.clone(),
                asset_id: "missing-asset".into(),
                enabled: true,
                pose: boardstudio_core::model::Pose2 {
                    at: boardstudio_core::model::Vec2 { x: 0.0, y: 0.0 },
                    rotation: 0.0,
                },
                elevation: 0.0,
                model_assets: Default::default(),
            });
        project_name_test_support::open_document(&runtime, document).await;
        let accepted = runtime.model().accepted.expect("the Case project is accepted");
        let scope = runtime.scope().expect("the accepted Case project has a scope");
        let error = runtime
            .prepare_native_case_preview(scope.clone(), accepted.token, accepted.document.revision)
            .await
            .expect_err("missing imported asset must fail before Core dispatch");
        assert!(error.contains("Imported board asset is unavailable"));
        assert_eq!(runtime.native_case_preview_error().as_ref(), Some(&error));
        assert!(!runtime.native_case_preview_pending());
        assert!(runtime.native_case_preview().is_none());
        project_name_test_support::navigate(&runtime, "another-board").await;
        assert!(
            runtime.native_case_preview_error().is_none(),
            "old capture failures cannot follow another owner"
        );
    }
}

#[cfg(all(test, target_arch = "wasm32"))]
#[path = "native_model_mapping_tests.rs"]
mod native_model_mapping_tests;

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

#[cfg(any(test, feature = "test-support"))]
#[path = "in_process_support.rs"]
pub mod in_process_support;

#[cfg(any(test, feature = "test-support"))]
pub mod firmware_export_test_support {
    use super::*;
    use boardstudio_application::{Completion, Effect, Event, SaveResult, Session};
    use boardstudio_core::{CoreEngine, firmware::FirmwarePackage, model::Board};
    use boardstudio_web_host::host::{ArchiveResult, CoreExecutorFuture, HostError};
    use futures_channel::oneshot;

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum Stage {
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

    pub struct ControlledExecutor {
        fail_at: Option<Stage>,
        gate: RefCell<Option<Gate>>,
    }

    impl ControlledExecutor {
        pub fn succeeding() -> Rc<Self> {
            Rc::new(Self {
                fail_at: None,
                gate: RefCell::new(None),
            })
        }

        pub fn failing(stage: Stage) -> Rc<Self> {
            Rc::new(Self {
                fail_at: Some(stage),
                gate: RefCell::new(None),
            })
        }

        pub fn gated(
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

    impl CoreExecutor for ControlledExecutor {
        fn request<'a>(
            &'a self,
            request_id: &'a str,
            _executor_epoch: &'a str,
            request: &'a CoreRequest,
        ) -> CoreExecutorFuture<'a, CoreReply> {
            let request_id = request_id.to_owned();
            let response = match request {
                CoreRequest::ResolveElectrical { request, .. } => {
                    if self.fail_at == Some(Stage::Resolution) {
                        Err(HostError("injected electrical-resolution failure".into()))
                    } else {
                        Ok(CoreReply::ElectricalResolved {
                            id: request_id.clone(),
                            plan: resolved_plan(request),
                        })
                    }
                }
                CoreRequest::GenerateFirmware { .. } => {
                    if self.fail_at == Some(Stage::Generation) {
                        Err(HostError("injected firmware-generation failure".into()))
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
                _ => Err(HostError("unexpected Core request in firmware test".into())),
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
        ) -> CoreExecutorFuture<'a, ArchiveResult> {
            Box::pin(async move {
                self.wait_at(Stage::Packaging).await;
                if self.fail_at == Some(Stage::Packaging) {
                    return Err(HostError("injected ZIP-packaging failure".into()));
                }
                Ok(ArchiveResult {
                    metadata: serde_json::to_string(&ArchiveReply::Packed)
                        .map_err(|error| HostError(error.to_string()))?,
                    buffers: vec![Uint8Array::from(&[0x50, 0x4b, 0x03, 0x04][..])],
                })
            })
        }

        fn artifact<'a>(
            &'a self,
            _request_id: &'a str,
            _executor_epoch: &'a str,
            _request: &'a ArtifactRequest,
        ) -> CoreExecutorFuture<'a, ArtifactReply> {
            Box::pin(async move {
                Err(HostError(
                    "firmware export tests do not request artifacts".into(),
                ))
            })
        }

        fn ready<'a>(&'a self) -> CoreExecutorFuture<'a, ()> {
            Box::pin(async move { Ok(()) })
        }

        fn close(&self) {}
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

    /// The one-board document `opened_session` opens. Browser tests that need a real accepted
    /// board project start from it, add what they exercise, and open it through the in-process
    /// adapter (`project_name_test_support::open_document`).
    pub fn board_document() -> ProjectDoc {
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
        document
    }

    pub fn opened_session() -> (Session, AcceptedSnapshot, Scope) {
        let mut session = Session::new();
        let mut core = CoreEngine::new();
        let mut effects = session.submit(Event::Open {
            operation_id: OperationId(1),
            document: board_document(),
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

    pub fn configure_runtime(
        runtime: &Runtime,
        session: Session,
        accepted: AcceptedSnapshot,
        scope: Scope,
        executor: Rc<dyn CoreExecutor>,
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

    pub fn new_runtime() -> Rc<Runtime> {
        Runtime::new().expect("browser runtime fixture initializes")
    }

    pub async fn run_effects(runtime: &Rc<Runtime>, initial: Vec<Effect>) {
        let mut effects = VecDeque::from(initial);
        while let Some(effect) = effects.pop_front() {
            effects.extend(runtime.run(effect).await);
        }
    }

    pub fn start_export(runtime: &Rc<Runtime>) -> Vec<Effect> {
        runtime.export_firmware();
        runtime.take_firmware_export_test_effects()
    }

    pub fn replace_executor(
        runtime: &Runtime,
        executor: Rc<ControlledExecutor>,
        epoch: boardstudio_application::ExecutorEpoch,
    ) {
        runtime.replace_firmware_export_test_executor(executor, epoch);
    }

    pub fn replace_owner(runtime: &Runtime, accepted: AcceptedSnapshot, scope: Option<Scope>) {
        runtime.replace_firmware_export_test_owner(accepted, scope);
    }

    pub fn take_deliveries(runtime: &Runtime) -> Vec<FirmwareTestDelivery> {
        runtime.take_firmware_export_test_deliveries()
    }

    pub fn take_events(runtime: &Runtime) -> Vec<Event> {
        runtime.take_firmware_export_test_events()
    }

    pub fn take_effects(runtime: &Runtime) -> Vec<Effect> {
        runtime.take_firmware_export_test_effects()
    }

    pub fn session_model(runtime: &Runtime) -> ReadModel {
        runtime.session.borrow().read_model().clone()
    }

    pub fn has_artifacts(runtime: &Runtime) -> bool {
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

async fn request_exact_cad(
    worker: &CadWorker,
    operation_id: OperationId,
    name: &str,
    identity: &CadSnapshotIdentity,
    prepared: boardstudio_core::model::PreparedCaseAssemblyIR,
) -> Result<CadResult, String> {
    let request_id = format!("mechanical-exact-{}-{name}", operation_id.0);
    let request = CadRequest {
        request_id,
        job_id: format!("mechanical-job-{}-{name}", operation_id.0),
        identity: identity.clone(),
        operation: CadOperation::Exact,
        prepared: Some(prepared),
        input_bytes: Vec::new(),
    };
    let reply = worker
        .request(request.clone())
        .await
        .map_err(|error| format!("CAD mechanical geometry failed: {error}"))?;
    validate_reply(&request, reply, identity)
        .map_err(|error| format!("CAD mechanical geometry failed: {error:?}"))
}

fn push_mechanical_file(
    files: &mut Vec<(String, Vec<u8>)>,
    paths: &mut BTreeSet<String>,
    path: String,
    bytes: Vec<u8>,
) -> Result<(), String> {
    if path.is_empty()
        || path.starts_with('/')
        || path.contains('\\')
        || path
            .split('/')
            .any(|part| part.is_empty() || part == "." || part == "..")
    {
        return Err(format!(
            "Mechanical exporter produced an unsafe path: {path}"
        ));
    }
    if !paths.insert(path.clone()) {
        return Err(format!(
            "Mechanical exporter produced a duplicate path: {path}"
        ));
    }
    files.push((path, bytes));
    Ok(())
}

fn mechanical_filename_component(value: &str) -> String {
    let mut result = String::with_capacity(value.len());
    let mut in_replacement = false;
    for character in value.chars() {
        if character.is_ascii_alphanumeric() || matches!(character, '_' | '-') {
            result.push(character);
            in_replacement = false;
        } else if !in_replacement {
            result.push('-');
            in_replacement = true;
        }
    }
    result
}

fn authored_case_filename(document: &ProjectDoc, scope: &Scope) -> String {
    let suffix = if document.boards.len() == 1 {
        String::new()
    } else {
        document
            .boards
            .iter()
            .find(|board| board.id == scope.board_id)
            .map(|board| format!("-{}", authored_case_filename_component(&board.name)))
            .unwrap_or_default()
    };
    format!("{}{suffix}-case.step", document.name)
}

fn authored_case_filename_component(value: &str) -> String {
    let mut result = String::with_capacity(value.len());
    let mut in_replacement = false;
    for character in value.chars() {
        if character.is_ascii_alphanumeric() || matches!(character, '_' | '.' | '-') {
            result.push(character);
            in_replacement = false;
        } else if !in_replacement {
            result.push('_');
            in_replacement = true;
        }
    }
    result
}

fn mechanical_stl(mesh: &boardstudio_web_host::cad_jobs::CadMesh) -> Result<Vec<u8>, String> {
    if mesh.positions.len() % 9 != 0 || mesh.positions.len() != mesh.normals.len() {
        return Err("CAD returned incomplete mechanical STL triangles.".into());
    }
    let triangles = mesh.positions.len() / 9;
    let triangle_count = u32::try_from(triangles)
        .map_err(|_| "Mechanical STL exceeds the supported triangle count.".to_owned())?;
    let byte_length = 84usize
        .checked_add(
            triangles
                .checked_mul(50)
                .ok_or_else(|| "Mechanical STL is too large.".to_owned())?,
        )
        .ok_or_else(|| "Mechanical STL is too large.".to_owned())?;
    let mut bytes = vec![0u8; byte_length];
    const HEADER: &[u8] = b"Board Studio mechanical part; coordinates in millimetres";
    bytes[..HEADER.len()].copy_from_slice(HEADER);
    bytes[80..84].copy_from_slice(&triangle_count.to_le_bytes());
    for triangle in 0..triangles {
        let offset = 84 + triangle * 50;
        for component in 0..3 {
            let value = mesh.normals[triangle * 9 + component];
            bytes[offset + component * 4..offset + component * 4 + 4]
                .copy_from_slice(&value.to_le_bytes());
        }
        for component in 0..9 {
            let value = mesh.positions[triangle * 9 + component];
            let index = offset + 12 + component * 4;
            bytes[index..index + 4].copy_from_slice(&value.to_le_bytes());
        }
    }
    Ok(bytes)
}

fn mechanical_xml_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

fn serialized_enum_label<T: serde::Serialize>(value: &T) -> String {
    serde_json::to_value(value)
        .ok()
        .and_then(|value| value.as_str().map(str::to_owned))
        .unwrap_or_default()
}

fn mechanical_fabrication_notes(document: &ProjectDoc, assembly: &MechanicalAssembly) -> String {
    let Some(configuration) = document.mechanical.as_ref() else {
        return String::new();
    };
    let mut hardware = configuration.hardware.clone().unwrap_or_default();
    hardware.extend(assembly.generated_hardware.iter().cloned());
    let mounts = assembly
        .case
        .bodies
        .iter()
        .find(|body| body.body.id == "plate")
        .and_then(|body| body.body.mounts.as_ref())
        .cloned()
        .unwrap_or_default();
    let method = serialized_enum_label(&configuration.method);
    let mount = serialized_enum_label(&configuration.mount);
    let plate_to_pcb = assembly
        .stack
        .iter()
        .find(|layer| layer.id == "plate")
        .map_or(configuration.plate_to_pcb, |layer| layer.z);
    let material_note = if method == "pcb-fr4" {
        "Plate substrate: FR4, no copper or plated holes. Confirm grade, finish and thickness tolerance with the fabricator."
    } else {
        "Material grade, finish and mechanical properties must be selected with the fabricator; no material grade is inferred from the process choice."
    };
    let mount_notes = if mounts.is_empty() {
        "none".to_owned()
    } else {
        mounts
            .iter()
            .map(|item| {
                format!(
                    "{}: diameter {} mm at ({}, {}) mm",
                    item.id, item.hole_diameter, item.at.x, item.at.y
                )
            })
            .collect::<Vec<_>>()
            .join("; ")
    };
    let hardware_notes = if hardware.is_empty() {
        "- No hardware specifications recorded.".to_owned()
    } else {
        hardware
            .iter()
            .map(|item| {
                format!(
                    "- {}: {} × {}; thread {}; length {} mm; part {}, mount {}{}{}",
                    item.id,
                    item.quantity,
                    item.designation,
                    item.thread,
                    item.length,
                    item.part_id,
                    item.feature_id,
                    item.tolerance
                        .as_ref()
                        .map(|value| format!("; tolerance {value}"))
                        .unwrap_or_default(),
                    item.notes
                        .as_ref()
                        .map(|value| format!("; {value}"))
                        .unwrap_or_default(),
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    };
    let processes = configuration
        .part_processes
        .as_ref()
        .filter(|items| !items.is_empty())
        .map(|items| {
            items
                .iter()
                .map(|part| {
                    format!(
                        "- {}: {}; material {}; finished thickness {} mm; constraint set {}",
                        part.part_id,
                        serialized_enum_label(&part.method),
                        part.material,
                        part.thickness,
                        part.constraints_version
                    )
                })
                .collect::<Vec<_>>()
                .join("\n")
        })
        .unwrap_or_else(|| "- No per-part overrides.".into());
    let gasket_note = if mount == "gasket" {
        let thickness = configuration
            .gasket_layout
            .as_ref()
            .map_or(2.0, |layout| layout.thickness);
        let compression = configuration
            .gasket_layout
            .as_ref()
            .map_or(0.15, |layout| layout.compression);
        format!(
            "\nGasket stock: EVA, {thickness} mm uncompressed; {}% nominal assembly compression. Exported assembly strips depict compressed thickness; cut the strip outlines from the specified uncompressed stock.\n",
            100.0 * compression
        )
    } else {
        String::new()
    };
    let critical_fits = configuration
        .critical_fits
        .as_ref()
        .filter(|items| !items.is_empty())
        .map(|items| {
            items
                .iter()
                .map(|fit| {
                    let length = (fit.to.x - fit.from.x).hypot(fit.to.y - fit.from.y);
                    format!(
                        "- {}: {} — {}: {:.3} mm, {}; from ({}, {}) to ({}, {}) mm.",
                        fit.id,
                        fit.part_id,
                        fit.label,
                        length,
                        fit.tolerance,
                        fit.from.x,
                        fit.from.y,
                        fit.to.x,
                        fit.to.y
                    )
                })
                .collect::<Vec<_>>()
                .join("\n")
        })
        .unwrap_or_else(|| "- No critical dimension annotations recorded.".into());
    let openings = assembly
        .nominal_plate_contours
        .iter()
        .filter(|contour| contour.hole)
        .enumerate()
        .map(|(index, contour)| {
            let min_x = contour.points.iter().map(|point| point.x).fold(f64::INFINITY, f64::min);
            let max_x = contour.points.iter().map(|point| point.x).fold(f64::NEG_INFINITY, f64::max);
            let min_y = contour.points.iter().map(|point| point.y).fold(f64::INFINITY, f64::min);
            let max_y = contour.points.iter().map(|point| point.y).fold(f64::NEG_INFINITY, f64::max);
            format!(
                "- Opening {}: {:.3} × {:.3} mm; review the full contour for corner radii and retention tabs.",
                index + 1,
                max_x - min_x,
                max_y - min_y
            )
        })
        .collect::<Vec<_>>();
    let openings = if openings.is_empty() {
        "- No profile openings.".to_owned()
    } else {
        openings.join("\n")
    };
    let profiles = if configuration.profiles.is_empty() {
        "- No mechanical profiles configured.".to_owned()
    } else {
        configuration
            .profiles
            .iter()
            .map(|profile| format!("- {}: {}", profile.definition_id, profile.source))
            .collect::<Vec<_>>()
            .join("\n")
    };
    let diagnostics = if assembly.diagnostics.is_empty() {
        "- No resolver diagnostics.".to_owned()
    } else {
        assembly
            .diagnostics
            .iter()
            .map(|finding| {
                format!(
                    "- {}: {}",
                    serialized_enum_label(&finding.severity),
                    finding.message
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    };
    format!(
        "# Mechanical fabrication specification\n\nRevision: {}\nProcess: {method}\nMount system: {mount}\nPlate finished thickness: {} mm\nPCB nominal thickness: {} mm\nPlate-to-PCB distance: {plate_to_pcb} mm\nPlate foam thickness: {} mm\nBottom foam thickness: {} mm\nWall thickness: {} mm\n\nThe assembled STEP includes a nominal, unpopulated PCB reference. Component solids are not included. This reference is excluded from manufacturing part exports.\n\n## Material and hardware\n\n{material_note}\nPlate mounting holes: {mount_notes}.\nHardware specifications (metadata only; threads are not modeled):\n{hardware_notes}\nFastener compatibility, washers, inserts, torque, gasket material and adhesive specifications are not inferred from hole diameter. Review recorded hardware against the assembled stack before ordering.\n\nPer-part process specifications:\n{processes}\n{gasket_note}\n## Critical fit and process allowances\n\nPlate STEP, STL, SVG, DXF and KiCad geometry uses the same resolved millimetre design. Explicit radial opening allowance: {} mm. Foam retains nominal exclusions. No automatic shrinkage, kerf or tool-radius compensation has been applied. Account for the recorded opening allowance before applying any additional reviewed CAM compensation; preserve the nominal source. CNC internal corners require a compatible tool radius or explicitly reviewed relief. Printed shrinkage and cut-sheet kerf require a measured process coupon. Critical interfaces: switch retention, stabilizer cutouts, plate-to-PCB distance, fastener fit and battery clearance. Confirm each before fabrication.\n\nRecorded critical dimensions:\n{critical_fits}\n\nNominal opening extents (bounding dimensions, not replacement profiles):\n{openings}\n\nProfile sources:\n{profiles}\n\nDiagnostics:\n{diagnostics}\n",
        assembly.revision,
        configuration.plate_thickness,
        configuration.pcb_thickness,
        configuration.plate_foam_thickness,
        configuration.bottom_foam_thickness,
        configuration.wall_thickness,
        configuration.opening_allowance.unwrap_or(0.0)
    )
}

fn critical_fit_drawing(
    assembly: &MechanicalAssembly,
    configuration: &boardstudio_core::model::MechanicalConfiguration,
) -> Result<String, String> {
    use boardstudio_core::model::Vec2;

    let contours = &assembly.nominal_plate_contours;
    let fits = configuration.critical_fits.as_deref().unwrap_or_default();
    let mut hardware = configuration.hardware.clone().unwrap_or_default();
    hardware.extend(assembly.generated_hardware.iter().cloned());
    let referenced_parts = fits
        .iter()
        .map(|fit| fit.part_id.as_str())
        .chain(hardware.iter().map(|item| item.part_id.as_str()))
        .collect::<BTreeSet<_>>();
    let reference_bodies = assembly
        .case
        .bodies
        .iter()
        .filter(|entry| {
            entry.body.id != "plate" && referenced_parts.contains(entry.body.id.as_str())
        })
        .collect::<Vec<_>>();
    let points = contours
        .iter()
        .flat_map(|contour| contour.points.iter())
        .chain(fits.iter().flat_map(|fit| [&fit.from, &fit.to]))
        .chain(reference_bodies.iter().flat_map(|entry| {
            entry
                .contours
                .iter()
                .flat_map(|contour| contour.points.iter())
        }))
        .collect::<Vec<_>>();
    if points.is_empty() {
        return Err("No plate geometry for critical-fit drawing.".into());
    }
    if points
        .iter()
        .any(|point| !point.x.is_finite() || !point.y.is_finite())
    {
        return Err("Invalid critical-fit coordinates.".into());
    }
    let min_x = points
        .iter()
        .map(|point| point.x)
        .fold(f64::INFINITY, f64::min);
    let max_x = points
        .iter()
        .map(|point| point.x)
        .fold(f64::NEG_INFINITY, f64::max);
    let min_y = points
        .iter()
        .map(|point| point.y)
        .fold(f64::INFINITY, f64::min);
    let max_y = points
        .iter()
        .map(|point| point.y)
        .fold(f64::NEG_INFINITY, f64::max);
    let width = max_x - min_x;
    let height = max_y - min_y;
    let path_data = |points: &[Vec2]| {
        points
            .iter()
            .enumerate()
            .map(|(index, point)| {
                format!(
                    "{} {} {}",
                    if index == 0 { "M" } else { "L" },
                    point.x,
                    -point.y
                )
            })
            .collect::<Vec<_>>()
            .join(" ")
    };
    let paths = contours
        .iter()
        .map(|contour| format!("<path d=\"{} Z\"/>", path_data(&contour.points)))
        .collect::<String>();
    let reference_paths = reference_bodies
        .iter()
        .map(|entry| {
            let paths = entry
                .contours
                .iter()
                .map(|contour| format!("<path d=\"{} Z\"/>", path_data(&contour.points)))
                .collect::<String>();
            format!(
                "<g data-part=\"{}\" fill=\"none\" stroke=\"#8c959b\" stroke-width=\"0.1\" stroke-dasharray=\"0.7 0.5\">{paths}</g>",
                mechanical_xml_escape(&entry.body.id)
            )
        })
        .collect::<String>();
    let mounts = assembly
        .case
        .bodies
        .iter()
        .find(|body| body.body.id == "plate")
        .and_then(|body| body.body.mounts.as_ref())
        .into_iter()
        .flatten()
        .map(|mount| {
            format!(
                "<circle cx=\"{}\" cy=\"{}\" r=\"{}\"/>",
                mount.at.x,
                -mount.at.y,
                mount.hole_diameter / 2.0
            )
        })
        .collect::<String>();
    let mut dimensions = String::new();
    for fit in fits {
        let dx = fit.to.x - fit.from.x;
        let dy = fit.to.y - fit.from.y;
        let length = dx.hypot(dy);
        if !length.is_finite() || length <= 0.0 {
            return Err("Critical-fit endpoints must be distinct.".into());
        }
        let offset = Vec2 {
            x: -dy / length * 5.0,
            y: dx / length * 5.0,
        };
        let a = Vec2 {
            x: fit.from.x + offset.x,
            y: fit.from.y + offset.y,
        };
        let b = Vec2 {
            x: fit.to.x + offset.x,
            y: fit.to.y + offset.y,
        };
        let label = format!(
            "{}: {} — {:.3} mm {}",
            fit.part_id, fit.label, length, fit.tolerance
        );
        dimensions.push_str(&format!(
            "<g data-fit=\"{}\"><path d=\"M {} {} L {} {} M {} {} L {} {}\" fill=\"none\" stroke=\"#42657a\" stroke-width=\"0.12\"/><path d=\"M {} {} L {} {}\" fill=\"none\" stroke=\"#42657a\" stroke-width=\"0.15\" marker-start=\"url(#dimension-arrow)\" marker-end=\"url(#dimension-arrow)\"/><text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-family=\"sans-serif\" font-size=\"2.2\" fill=\"#23495e\">{}</text></g>",
            mechanical_xml_escape(&fit.id),
            fit.from.x, -fit.from.y, a.x, -a.y,
            fit.to.x, -fit.to.y, b.x, -b.y,
            a.x, -a.y, b.x, -b.y,
            (a.x + b.x) / 2.0,
            -(a.y + b.y) / 2.0 - 1.0,
            mechanical_xml_escape(&label),
        ));
    }
    let mut callouts = String::new();
    for (index, item) in hardware.iter().enumerate() {
        let mount = assembly
            .case
            .bodies
            .iter()
            .find(|entry| entry.body.id == item.part_id)
            .and_then(|entry| entry.body.mounts.as_ref())
            .and_then(|mounts| mounts.iter().find(|mount| mount.id == item.feature_id))
            .ok_or_else(|| {
                format!(
                    "Hardware {} is not linked to a generated mounting feature.",
                    item.id
                )
            })?;
        let x = max_x + 12.0;
        let y = -max_y + index as f64 * 9.0;
        let label = format!(
            "{} × {}; {} × {} mm",
            item.quantity, item.designation, item.thread, item.length
        );
        let feature = format!(
            "{}/{}{}",
            item.part_id,
            item.feature_id,
            item.tolerance
                .as_ref()
                .map(|value| format!("; {value}"))
                .unwrap_or_default()
        );
        callouts.push_str(&format!(
            "<g data-hardware=\"{}\"><path d=\"M {} {} L {} {}\" fill=\"none\" stroke=\"#705b35\" stroke-width=\"0.12\"/><circle cx=\"{}\" cy=\"{}\" r=\"0.4\" fill=\"#705b35\"/><text x=\"{}\" y=\"{}\" font-family=\"sans-serif\" font-size=\"2.3\" fill=\"#493a21\">{}<tspan x=\"{}\" dy=\"3\">{}</tspan></text></g>",
            mechanical_xml_escape(&item.id),
            mount.at.x, -mount.at.y, x - 2.0, y,
            mount.at.x, -mount.at.y,
            x, y, mechanical_xml_escape(&label), x, mechanical_xml_escape(&feature),
        ));
    }
    let drawing_width = width + if hardware.is_empty() { 40.0 } else { 135.0 };
    let drawing_height = (height + 50.0).max(hardware.len() as f64 * 9.0 + 40.0);
    Ok(format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{drawing_width}mm\" height=\"{drawing_height}mm\" viewBox=\"{} {} {drawing_width} {drawing_height}\"><defs><marker id=\"dimension-arrow\" viewBox=\"0 0 10 10\" refX=\"5\" refY=\"5\" markerWidth=\"3\" markerHeight=\"3\" orient=\"auto-start-reverse\"><path d=\"M 0 0 L 10 5 L 0 10 Z\" fill=\"#42657a\"/></marker></defs>{reference_paths}<g fill=\"none\" stroke=\"#111\" stroke-width=\"0.15\">{paths}{mounts}<path d=\"M {min_x} {} v 4 M {max_x} {} v 4 M {min_x} {} H {max_x}\"/></g>{dimensions}{callouts}<g font-family=\"sans-serif\" font-size=\"2.5\" fill=\"#111\"><text x=\"{min_x}\" y=\"{}\">NOMINAL ASSEMBLY XY — CRITICAL FIT REVIEW</text><text x=\"{min_x}\" y=\"{}\">Extents: {:.3} × {:.3} mm</text><text x=\"{min_x}\" y=\"{}\">Nominal geometry; see specification for opening allowance.</text><text x=\"{min_x}\" y=\"{}\">Hardware callouts are specifications; threads are not modeled.</text><text x=\"{min_x}\" y=\"{}\">See FABRICATION.md for fit and hardware specifications.</text></g></svg>",
        min_x - 20.0,
        -max_y - 20.0,
        -min_y + 5.0,
        -min_y + 5.0,
        -min_y + 7.0,
        -max_y - 10.0,
        -min_y + 12.0,
        width,
        height,
        -min_y + 17.0,
        -min_y + 22.0,
        -min_y + 27.0,
    ))
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

#[cfg(any(test, feature = "test-support"))]
pub mod project_name_test_support {
    use super::*;
    use crate::runtime::in_process_support::InProcessAdapters;

    pub fn new_runtime() -> Rc<Runtime> {
        Runtime::new_with_restoration(false).expect("browser runtime fixture initializes")
    }

    /// Replace the runtime's session and route Core execution through an in-process
    /// `CoreEngine`. Saves keep running through the production browser-store port (gated by
    /// `gate_next_persist` / `fail_next_persist`); call `install_memory_persistence` to save
    /// into memory instead.
    pub fn install(runtime: &Runtime, session: Session, core: boardstudio_core::CoreEngine) {
        *runtime.session.borrow_mut() = session;
        let adapters = Rc::new(InProcessAdapters::new(core, runtime.store.clone()));
        adapters.attach(runtime);
        *runtime.in_process_adapters.borrow_mut() = Some(adapters);
        runtime.changed();
    }

    /// Route saves into the in-process memory store. Requires `install` to have run.
    pub fn install_memory_persistence(runtime: &Runtime) {
        installed(runtime).use_memory_saves();
    }

    /// Install the in-process adapters with memory saves and open `document` through the real
    /// Session and Core, leaving it accepted, saved and ready. Browser tests that need an
    /// opened project start here instead of giving Runtime a fabricated read model.
    pub async fn open_document(runtime: &Rc<Runtime>, document: ProjectDoc) {
        install(runtime, Session::new(), boardstudio_core::CoreEngine::new());
        install_memory_persistence(runtime);
        runtime.submit(Event::Open {
            operation_id: runtime.operation(),
            document,
        });
        run_pending(runtime).await;
    }

    /// Commit `document` as one `ReplaceDocument` edit through the real Session and Core,
    /// driving every pending effect, and return the snapshot it was accepted as.
    pub async fn replace_document(
        runtime: &Rc<Runtime>,
        transaction_id: &str,
        target_id: &str,
        document: ProjectDoc,
    ) -> AcceptedSnapshot {
        let base_revision = runtime
            .model()
            .accepted
            .expect("an accepted document is open")
            .document
            .revision;
        runtime.submit(Event::Edit {
            operation_id: runtime.operation(),
            command: boardstudio_core::model::EditCommand {
                base_revision,
                transaction_id: transaction_id.to_owned(),
                phase: boardstudio_core::model::EditPhase::Commit,
                target_ids: vec![target_id.to_owned()],
                operation: boardstudio_core::model::EditOperation::ReplaceDocument {
                    document: Box::new(document),
                },
            },
        });
        run_pending(runtime).await;
        runtime
            .model()
            .accepted
            .expect("the edited document stays accepted")
    }

    /// Make `board_id` the active board through the real Session.
    pub async fn navigate(runtime: &Rc<Runtime>, board_id: &str) {
        runtime.submit(Event::Navigate {
            operation_id: runtime.operation(),
            board_id: board_id.to_owned(),
            instance_id: None,
        });
        run_pending(runtime).await;
    }

    /// The effects Session asked Runtime to run that no test has driven yet. Generation and
    /// export jobs are outside the adapter, so tests read these to see what Session started.
    /// Taking them also keeps a later `run_pending` from starting the job.
    pub fn take_held_effects(runtime: &Runtime) -> Vec<Effect> {
        std::mem::take(&mut *runtime.held_effects.borrow_mut())
    }

    /// Deliver a completion to Session as the executor that ran the effect would have. Tests
    /// use it for the generation jobs the adapter does not run; the effects it produces are
    /// held like those of a submitted event.
    pub fn complete(runtime: &Rc<Runtime>, completion: Completion) {
        let effects = runtime.complete(completion);
        runtime.held_effects.borrow_mut().extend(effects);
    }

    fn installed(runtime: &Runtime) -> Rc<InProcessAdapters> {
        runtime
            .in_process_adapters
            .borrow()
            .clone()
            .expect("in-process adapters are installed")
    }

    pub fn gate_import_archive(
        runtime: &Runtime,
        document: &ProjectDoc,
    ) -> (
        futures_channel::oneshot::Receiver<()>,
        futures_channel::oneshot::Sender<()>,
    ) {
        let (entered, entered_rx) = futures_channel::oneshot::channel();
        let (release, release_rx) = futures_channel::oneshot::channel();
        let reply = ArchiveReply::Unpacked {
            project_json: serde_json::to_string(document).expect("fixture document serializes"),
            assets: Vec::new(),
        };
        *runtime.import_archive_test_gate.borrow_mut() = Some(ImportArchiveTestGate {
            entered,
            release: release_rx,
            metadata: serde_json::to_string(&reply).expect("archive reply serializes"),
            buffers: Vec::new(),
        });
        (entered_rx, release)
    }

    pub fn gate_open_saved_result(
        runtime: &Runtime,
    ) -> (
        futures_channel::oneshot::Receiver<()>,
        futures_channel::oneshot::Sender<()>,
    ) {
        let (entered, entered_rx) = futures_channel::oneshot::channel();
        let (release, release_rx) = futures_channel::oneshot::channel();
        *runtime.open_saved_load_test_gate.borrow_mut() = Some(OpenSavedLoadTestGate {
            entered,
            release: release_rx,
        });
        (entered_rx, release)
    }

    pub fn track_import_file(runtime: &Runtime) -> futures_channel::oneshot::Receiver<()> {
        let (done, done_rx) = futures_channel::oneshot::channel();
        *runtime.import_file_test_done.borrow_mut() = Some(done);
        done_rx
    }

    pub fn track_open_saved(runtime: &Runtime) -> futures_channel::oneshot::Receiver<()> {
        let (done, done_rx) = futures_channel::oneshot::channel();
        *runtime.open_saved_test_done.borrow_mut() = Some(done);
        done_rx
    }

    pub fn replace_session(runtime: &Runtime, session: Session) {
        *runtime.session.borrow_mut() = session;
        runtime.changed();
    }

    pub fn fail_next_persist(runtime: &Runtime, reason: impl Into<String>) {
        installed(runtime).persistence().fail_next_save(reason);
    }

    pub fn gate_next_persist(
        runtime: &Runtime,
    ) -> (
        futures_channel::oneshot::Receiver<()>,
        futures_channel::oneshot::Sender<()>,
    ) {
        installed(runtime).persistence().gate_next_save()
    }

    /// Fail the next Core reply with the supplied reason.
    pub fn fail_next_core_reply(runtime: &Runtime, reason: impl Into<String>) {
        installed(runtime).current_core().fail_next_reply(reason);
    }

    /// Hold the next Core reply until the returned release sender fires. The entered
    /// receiver observes the request reaching the executor.
    pub fn gate_next_core_reply(
        runtime: &Runtime,
    ) -> (
        futures_channel::oneshot::Receiver<()>,
        futures_channel::oneshot::Sender<()>,
    ) {
        installed(runtime).current_core().gate_next_reply()
    }

    /// The copy a completed save left in the in-process memory store.
    pub fn saved_document(runtime: &Runtime, project_id: &str) -> Option<ProjectDoc> {
        installed(runtime).persistence().saved_document(project_id)
    }

    /// The in-process Core executor currently installed on the runtime.
    pub fn in_process_core(
        runtime: &Runtime,
    ) -> Rc<crate::runtime::in_process_support::InProcessCore> {
        installed(runtime).current_core()
    }

    pub fn observe(
        runtime: &Runtime,
        operation: OperationId,
    ) -> crate::operation_outcomes::OutcomeSlot {
        runtime.operation_outcomes.observe(operation)
    }

    pub fn observe_next(runtime: &Runtime) -> crate::operation_outcomes::OutcomeSlot {
        runtime
            .operation_outcomes
            .observe(OperationId(runtime.next_operation.get()))
    }

    pub async fn run_pending(runtime: &Rc<Runtime>) {
        let mut pending = VecDeque::from(std::mem::take(&mut *runtime.held_effects.borrow_mut()));
        while let Some(effect) = pending.pop_front() {
            pending.extend(runtime.run(effect).await);
        }
    }

    pub fn drive_pending(runtime: &Rc<Runtime>) {
        let effects = std::mem::take(&mut *runtime.held_effects.borrow_mut());
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
        let executor: Rc<dyn CoreExecutor> = executor;
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
            let succeeding_executor: Rc<dyn CoreExecutor> = succeeding;
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
    use wasm_bindgen_test::wasm_bindgen_test;

    async fn runtime_with_scene(exact: bool) -> (Rc<Runtime>, AcceptedSnapshot, Scope) {
        let runtime = project_name_test_support::new_runtime();
        let mut document = firmware_export_test_support::board_document();
        document.boards.push(Board {
            id: "other-board".into(),
            name: "Other board".into(),
            outline_ids: Vec::new(),
            part_ids: Vec::new(),
            net_ids: Vec::new(),
            thickness: 1.6,
            traces: Vec::new(),
            vias: Vec::new(),
        });
        project_name_test_support::open_document(&runtime, document).await;
        let accepted = runtime.model().accepted.expect("the Case project is accepted");
        let scope = runtime.scope().expect("the accepted Case project has a scope");
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

    /// Land an edit through the real Session: the project name changes and, when given, so
    /// does the first board's thickness (a physical Case input).
    async fn edited(
        runtime: &Rc<Runtime>,
        scope: &Scope,
        board_thickness: Option<f64>,
    ) -> AcceptedSnapshot {
        let mut document = (*runtime
            .model()
            .accepted
            .expect("the Case project is accepted")
            .document)
            .clone();
        document.name.push_str(" updated");
        if let Some(thickness) = board_thickness {
            document.boards[0].thickness = thickness;
        }
        project_name_test_support::replace_document(
            runtime,
            "cad-scene-rebind-edit",
            &scope.board_id,
            document,
        )
        .await
    }

    #[wasm_bindgen_test]
    async fn cad_scene_rebinds_only_exact_output_with_matching_physical_inputs() {
        let (exact_runtime, _original, scope) = runtime_with_scene(true).await;
        let current = edited(&exact_runtime, &scope, None).await;
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

        let (preview_runtime, original, scope) = runtime_with_scene(false).await;
        edited(&preview_runtime, &scope, None).await;
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

        let (changed_runtime, original, scope) = runtime_with_scene(true).await;
        edited(&changed_runtime, &scope, Some(2.0)).await;
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
    async fn cad_scene_retires_old_scope_instead_of_rebinding_across_physical_owner() {
        let (runtime, _original, scope) = runtime_with_scene(true).await;
        edited(&runtime, &scope, None).await;
        let old_owner_scene = runtime.cad_scene.borrow().clone();
        // Navigating retires the cached scene on its own; put the old board's scene back so the
        // accessor itself has to refuse it for the new physical owner.
        project_name_test_support::navigate(&runtime, "other-board").await;
        *runtime.cad_scene.borrow_mut() = old_owner_scene;
        assert!(runtime.cad_scene().is_none());
    }
}

#[cfg(all(test, target_arch = "wasm32"))]
mod project_open_supersession_tests {
    use super::*;
    use crate::runtime::project_name_test_support as test_support;
    use boardstudio_core::CoreEngine;
    use wasm_bindgen_test::wasm_bindgen_test;

    async fn fixture() -> (Rc<Runtime>, ProjectDoc, ProjectDoc) {
        let runtime = test_support::new_runtime();
        let suffix = new_project_id().expect("browser fixture id is available");
        let (session, accepted, _) = firmware_export_test_support::opened_session();
        let current = (*accepted.document).clone();
        let mut stale_saved = current.clone();
        stale_saved.id = format!("stale-saved-{suffix}");
        stale_saved.name = "Stale saved project".into();
        let mut newer_saved = current.clone();
        newer_saved.id = format!("newer-saved-{suffix}");
        newer_saved.name = "Newer saved project".into();
        runtime
            .store
            .save_document(&stale_saved, &BTreeMap::new())
            .await
            .expect("stale saved fixture persists");
        runtime
            .store
            .save_document(&newer_saved, &BTreeMap::new())
            .await
            .expect("newer saved fixture persists");
        test_support::install(&runtime, session, CoreEngine::new());
        (runtime, stale_saved, newer_saved)
    }

    fn archive_file() -> web_sys::File {
        let bytes = Uint8Array::from(&b"controlled archive input"[..]);
        web_sys::File::new_with_u8_array_sequence(
            &Array::of1(&bytes.into()),
            "controlled-import.boardstudio",
        )
        .expect("archive File fixture constructs")
    }

    fn imported_document(base: &ProjectDoc) -> ProjectDoc {
        let mut document = base.clone();
        document.id = format!(
            "imported-{}",
            new_project_id().expect("browser fixture id is available")
        );
        document.name = "Imported newer project".into();
        document
    }

    async fn wait_until_current_is_durable(runtime: &Rc<Runtime>, expected: &ProjectDoc) {
        for _ in 0..100 {
            test_support::run_pending(runtime).await;
            if runtime
                .model()
                .accepted
                .as_ref()
                .is_some_and(|accepted| accepted.document.id == expected.id)
            {
                assert_eq!(
                    runtime.store.active_project_id("").unwrap(),
                    expected.id,
                    "the newer project is durably current"
                );
                assert_eq!(
                    runtime
                        .store
                        .load_document(expected.id.clone())
                        .await
                        .unwrap(),
                    Some(expected.clone()),
                    "the newer project remains persisted"
                );
                return;
            }
            TimeoutFuture::new(1).await;
        }
        panic!("newer project did not become accepted and durable");
    }

    #[wasm_bindgen_test]
    async fn delayed_import_cannot_replace_a_newer_saved_project_open() {
        let (runtime, _, newer) = fixture().await;
        let imported = imported_document(&newer);
        let (archive_entered, release_archive) =
            test_support::gate_import_archive(&runtime, &imported);
        let import_done = test_support::track_import_file(&runtime);
        runtime.import_file(archive_file());
        archive_entered
            .await
            .expect("import reached unpack reply boundary");

        let open_done = test_support::track_open_saved(&runtime);
        runtime.open_saved(newer.id.clone());
        wait_until_current_is_durable(&runtime, &newer).await;
        open_done.await.expect("saved open route completed");

        release_archive.send(()).expect("release delayed import");
        import_done.await.expect("stale import route completed");
        assert_eq!(
            runtime.model().accepted.as_ref().unwrap().document.id,
            newer.id,
            "late archive output cannot replace the newer accepted project"
        );
        assert_eq!(runtime.store.active_project_id("").unwrap(), newer.id);
        assert!(
            runtime
                .store
                .load_document(imported.id)
                .await
                .unwrap()
                .is_none()
        );
    }

    #[wasm_bindgen_test]
    async fn delayed_saved_open_cannot_replace_a_newer_import() {
        let (runtime, stale_saved, current) = fixture().await;
        let imported = imported_document(&current);
        let (open_entered, release_open) = test_support::gate_open_saved_result(&runtime);
        let open_done = test_support::track_open_saved(&runtime);
        runtime.open_saved(stale_saved.id.clone());
        open_entered
            .await
            .expect("saved document read reached pre-admission boundary");

        let (archive_entered, release_archive) =
            test_support::gate_import_archive(&runtime, &imported);
        let import_done = test_support::track_import_file(&runtime);
        runtime.import_file(archive_file());
        archive_entered
            .await
            .expect("import reached unpack reply boundary");
        release_archive.send(()).expect("release newer import");
        wait_until_current_is_durable(&runtime, &imported).await;
        import_done.await.expect("new import route completed");

        release_open.send(()).expect("release stale saved open");
        open_done.await.expect("stale saved open route completed");
        assert_eq!(
            runtime.model().accepted.as_ref().unwrap().document.id,
            imported.id,
            "late saved-open output cannot replace the newer accepted import"
        );
        assert_eq!(runtime.store.active_project_id("").unwrap(), imported.id);
        assert_eq!(
            runtime
                .store
                .load_document(stale_saved.id.clone())
                .await
                .unwrap(),
            Some(stale_saved),
            "the superseded saved project remains in the library"
        );
    }
}

#[cfg(all(test, target_arch = "wasm32"))]
mod in_process_adapter_tests {
    use super::*;
    use crate::runtime::project_name_test_support as test_support;
    use boardstudio_application::RequestId;
    use boardstudio_core::model::{EditCommand, EditOperation, EditPhase};
    use wasm_bindgen_test::wasm_bindgen_test;

    /// A runtime whose Core requests run through a fresh in-process engine and whose saves
    /// land in the adapter's memory store.
    fn installed_runtime() -> Rc<Runtime> {
        let runtime = test_support::new_runtime();
        test_support::install(
            &runtime,
            Session::new(),
            boardstudio_core::CoreEngine::new(),
        );
        test_support::install_memory_persistence(&runtime);
        runtime
    }

    /// Open the fixture document through the runtime itself, so every test exercises the
    /// installed adapter from open through save.
    async fn submit_open(runtime: &Rc<Runtime>, name: &str) -> AcceptedSnapshot {
        let slot = test_support::observe_next(runtime);
        runtime.submit(Event::Open {
            operation_id: runtime.operation(),
            document: ProjectDoc::empty("adapter-test", name),
        });
        test_support::run_pending(runtime).await;
        assert_eq!(
            wait_outcome(&slot).await,
            TerminalOutcome::Completed,
            "the fixture open is accepted"
        );
        runtime.model().accepted.unwrap()
    }

    fn rename_edit(accepted: &AcceptedSnapshot, name: &str) -> EditCommand {
        let mut document = (*accepted.document).clone();
        document.name = name.into();
        EditCommand {
            base_revision: accepted.document.revision,
            transaction_id: format!("m1-adapter-test-{}", name),
            phase: EditPhase::Commit,
            target_ids: vec![document.id.clone()],
            operation: EditOperation::ReplaceDocument {
                document: Box::new(document),
            },
        }
    }

    fn submit_edit(
        runtime: &Rc<Runtime>,
        accepted: &AcceptedSnapshot,
        name: &str,
    ) -> crate::operation_outcomes::OutcomeSlot {
        let slot = test_support::observe_next(runtime);
        runtime.submit(Event::Edit {
            operation_id: runtime.operation(),
            command: rename_edit(accepted, name),
        });
        slot
    }

    async fn wait_outcome(slot: &crate::operation_outcomes::OutcomeSlot) -> TerminalOutcome {
        for _ in 0..200 {
            if let Some(outcome) = slot.borrow().clone() {
                return outcome;
            }
            TimeoutFuture::new(1).await;
        }
        panic!("operation outcome never settled");
    }

    #[wasm_bindgen_test]
    async fn installed_adapter_accepts_an_edit_and_saves_a_memory_copy() {
        let runtime = installed_runtime();
        let accepted = submit_open(&runtime, "Adapter test").await;
        let slot = submit_edit(&runtime, &accepted, "Renamed in memory");
        test_support::run_pending(&runtime).await;
        assert_eq!(wait_outcome(&slot).await, TerminalOutcome::Completed);
        let current = runtime.model().accepted.unwrap();
        assert_eq!(current.document.name, "Renamed in memory");
        assert_eq!(current.document.revision, accepted.document.revision + 1);
        let saved = test_support::saved_document(&runtime, "adapter-test")
            .expect("the completed edit leaves a saved copy in memory");
        assert_eq!(saved.name, "Renamed in memory");
    }

    #[wasm_bindgen_test]
    async fn gated_save_holds_saving_until_released_then_reports_saved() {
        let runtime = installed_runtime();
        let accepted = submit_open(&runtime, "Adapter test").await;
        let (entered, release) = test_support::gate_next_persist(&runtime);
        let slot = submit_edit(&runtime, &accepted, "Gated save");
        test_support::drive_pending(&runtime);
        entered
            .await
            .expect("the gated save reaches the persistence port");
        assert!(
            matches!(runtime.model().durability, Durability::Saving { .. }),
            "the edit stays pending in Saving while the save is held"
        );
        release.send(()).expect("release the held save");
        assert_eq!(wait_outcome(&slot).await, TerminalOutcome::Completed);
        let current = runtime.model().accepted.unwrap();
        assert_eq!(current.document.name, "Gated save");
        assert_eq!(
            runtime.model().durability,
            Durability::Saved {
                revision: current.document.revision
            }
        );
        assert_eq!(
            test_support::saved_document(&runtime, "adapter-test")
                .expect("released save lands in memory")
                .name,
            "Gated save"
        );
    }

    #[wasm_bindgen_test]
    async fn failed_save_requires_recovery_and_keeps_the_accepted_document() {
        let runtime = installed_runtime();
        let accepted = submit_open(&runtime, "Adapter test").await;
        test_support::fail_next_persist(&runtime, "injected durable write failure");
        let slot = submit_edit(&runtime, &accepted, "Must not commit");
        test_support::run_pending(&runtime).await;
        assert_eq!(
            wait_outcome(&slot).await,
            TerminalOutcome::PersistenceFailed("injected durable write failure".into())
        );
        assert_eq!(runtime.model().lifecycle, Lifecycle::RecoveryRequired);
        assert_eq!(
            runtime.model().durability,
            Durability::Failed {
                revision: accepted.document.revision + 1,
                reason: "injected durable write failure".into(),
            }
        );
        assert_eq!(
            runtime.model().accepted.unwrap().document.name,
            "Adapter test",
            "the accepted document keeps the pre-edit value"
        );
        assert_eq!(
            test_support::saved_document(&runtime, "adapter-test")
                .expect("the open left a saved copy in memory")
                .name,
            "Adapter test",
            "the failed edit's save never replaced the accepted copy"
        );
    }

    #[wasm_bindgen_test]
    async fn gated_core_reply_keeps_the_edit_pending_until_released() {
        let runtime = installed_runtime();
        let accepted = submit_open(&runtime, "Adapter test").await;
        let (entered, release) = test_support::gate_next_core_reply(&runtime);
        let slot = submit_edit(&runtime, &accepted, "Parked rename");
        test_support::drive_pending(&runtime);
        entered
            .await
            .expect("the edit request reaches the in-process executor");
        assert_eq!(
            runtime.model().accepted.unwrap().document.revision,
            accepted.document.revision,
            "nothing is accepted while the reply is held"
        );
        release.send(()).expect("release the held reply");
        assert_eq!(wait_outcome(&slot).await, TerminalOutcome::Completed);
        assert_eq!(
            runtime.model().accepted.unwrap().document.name,
            "Parked rename"
        );
    }

    #[wasm_bindgen_test]
    async fn restart_builds_a_fresh_engine_and_stale_replies_are_ignored() {
        let runtime = installed_runtime();
        let accepted = submit_open(&runtime, "Adapter test").await;
        let old_core = test_support::in_process_core(&runtime);
        let old_epoch = runtime.session.borrow().core_executor_epoch();
        test_support::fail_next_core_reply(&runtime, "injected core failure");
        let slot = submit_edit(&runtime, &accepted, "Never applied");
        test_support::run_pending(&runtime).await;
        assert_eq!(
            wait_outcome(&slot).await,
            TerminalOutcome::ExecutorFailed("injected core failure".into())
        );
        assert_eq!(runtime.model().lifecycle, Lifecycle::RecoveryRequired);
        let new_core = test_support::in_process_core(&runtime);
        assert!(
            !Rc::ptr_eq(&old_core, &new_core),
            "the executor restart builds a fresh in-process engine"
        );
        assert!(old_core.is_closed());
        assert!(!new_core.is_closed());
        let current_epoch = runtime.session.borrow().core_executor_epoch();
        assert_ne!(current_epoch, old_epoch, "the executor epoch advanced");
        runtime.complete(Completion::Core {
            request_id: RequestId(2),
            executor_epoch: old_epoch,
            reply: Box::new(CoreReply::Error {
                id: "m1-2".into(),
                message: "late reply from the replaced executor".into(),
                revision: 0,
            }),
        });
        assert_eq!(
            runtime.model().accepted.unwrap().document.name,
            "Adapter test",
            "a late reply from the replaced executor changes nothing"
        );
        assert_eq!(runtime.model().lifecycle, Lifecycle::RecoveryRequired);
    }
}
