//! Browser composition runs identified effects; the headless session remains authoritative.
use crate::archive_export::{ArchiveExportOptions, ArchiveWorkFuture, archive_filename};
pub use crate::core_services::CoreServices;
use crate::core_services::DocumentCore;
use crate::export_lease::{ExportKind, ExportLease, ExportLeases};
use crate::mechanical_package::{
    critical_fit_drawing, mechanical_fabrication_notes, mechanical_filename_component,
    mechanical_stl, push_mechanical_file,
};
use crate::native_model_mapping::{NativeModelDeliveryState, resolve_native_model_paths};
use crate::pcb_wiring_mode_operation::electrical_preview_request;
use boardstudio_application::{
    AcceptedSnapshot, Completion, Durability, Effect, Event, JobId, Lifecycle, OperationId,
    ReadModel, SaveResult, Scope, Session, SnapshotToken, TerminalOutcome,
};
use boardstudio_core_kernel::hash::sha256_hex;
use boardstudio_core_mechanical::electrical::{
    ElectricalMode, ElectricalPlan, ElectricalPlanRequest,
};
use boardstudio_model::model::{
    ArchiveEntry, ArchiveReply, ArchiveRequest, ArtifactReply, ArtifactRequest, Board,
    CaseAssemblyIR, CaseIR, CompiledFootprint, CoreReply, CoreRequest, HardwareTopology,
    KeycapSpec, Material, MechanicalAssembly, MechanicalBuiltinProfile, MechanicalConfiguration,
    MechanicalExtraction, MechanicalPartProfile, MechanicalPurposeMapping, MechanicalSwitchFamily,
    Operation, OutlineFeature, OutlineSettings, PcbPreview, PrepareExportRequest, ProjectDoc,
};
pub use boardstudio_web_host::host::CoreExecutor;

/// The stable identity of the current Core executor: captures compare it to detect that the
/// executor was replaced while their operation was in flight.
fn core_executor_identity(core: &Rc<dyn CoreExecutor>) -> usize {
    Rc::as_ptr(core) as *const () as usize
}
use boardstudio_web_host::host::{
    BrowserStore, InProcessCoreExecutor, SaveError, SavedProject, StorageSession, StorageToken,
};
use boardstudio_web_host::{
    cad_jobs::{
        CadJobError, CadOperation, CadRequest, CadResult, CadSnapshotIdentity,
        captured_case_document, captured_case_scene, captured_pcb_source, prepare_captured_case,
        prepare_captured_step_assembly, validate_reply,
    },
    cad_worker::CadWorker,
};
#[cfg(test)]
use sha2::{Digest, Sha256};
use std::sync::Arc;

type CoreExecutorFactory = dyn Fn() -> Result<Rc<dyn CoreExecutor>, String>;

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

struct PhysicalStepAssembly {
    prepared: boardstudio_web_host::cad_jobs::PreparedCadSnapshot,
    pcb_source: Option<boardstudio_web_host::cad_jobs::CapturedPcbSource>,
    pcb_source_thickness: Option<f64>,
}

pub struct CadScene {
    pub scope: Scope,
    pub token: SnapshotToken,
    pub snapshot: AcceptedSnapshot,
    pub result: CadResult,
    pub prepared: boardstudio_model::model::PreparedCaseAssemblyIR,
    pub physical_fingerprint: Option<[u8; 32]>,
    pub mechanical: Option<boardstudio_model::model::MechanicalAssembly>,
    pub exact: bool,
    pub contours: Vec<boardstudio_model::model::Contour>,
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
    board_outline: bool,
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
    notification_error: Option<bool>,
}

/// A one-shot result for the shell's single notification consumer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimeNotification {
    pub message: String,
    pub is_error: bool,
}

impl RuntimeReport {
    fn status(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            severity: RuntimeReportSeverity::Status,
            notification_error: None,
        }
    }

    fn alert(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            severity: RuntimeReportSeverity::Alert,
            notification_error: None,
        }
    }

    fn transient(mut self) -> Self {
        self.notification_error = Some(self.severity == RuntimeReportSeverity::Alert);
        self
    }

    fn transient_error(mut self) -> Self {
        self.notification_error = Some(true);
        self
    }

    fn clear_alert(&mut self) {
        if self.severity == RuntimeReportSeverity::Alert {
            self.message.clear();
            self.severity = RuntimeReportSeverity::Status;
        }
    }
}

fn export_terminal_report(
    capture_is_current: bool,
    outcome: TerminalOutcome,
    delivery_error: Option<String>,
) -> Option<RuntimeReport> {
    if !capture_is_current {
        return None;
    }
    match outcome {
        TerminalOutcome::Completed => Some(match delivery_error {
            Some(error) => RuntimeReport::alert(error).transient(),
            None => RuntimeReport::status("Saved locally.").transient(),
        }),
        TerminalOutcome::Rejected(reason) | TerminalOutcome::ExecutorFailed(reason) => {
            Some(RuntimeReport::alert(reason).transient())
        }
        TerminalOutcome::PersistenceFailed(reason) | TerminalOutcome::BlockedByRecovery(reason) => {
            Some(RuntimeReport::alert(reason))
        }
        TerminalOutcome::Cancelled => Some(RuntimeReport::status("Cancelled.").transient()),
        TerminalOutcome::Closed => Some(RuntimeReport::status("Editor closed.")),
        TerminalOutcome::Superseded => None,
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

/// The document persistence port: the save Runtime performs for Session `Persist` effects.
/// Loading, listing, deleting and the active-project preference stay on the browser store.
/// Production saves through IndexedDB; tests install the in-process memory adapter from
/// `in_process_support`.
pub trait DocumentPersistence {
    fn save_document<'a>(
        &'a self,
        document: &'a ProjectDoc,
        assets: &'a BTreeMap<String, Vec<u8>>,
        expected: Option<&'a StorageToken>,
    ) -> Pin<Box<dyn Future<Output = Result<StorageToken, SaveError>> + 'a>>;
}

impl DocumentPersistence for BrowserStore {
    fn save_document<'a>(
        &'a self,
        document: &'a ProjectDoc,
        assets: &'a BTreeMap<String, Vec<u8>>,
        expected: Option<&'a StorageToken>,
    ) -> Pin<Box<dyn Future<Output = Result<StorageToken, SaveError>> + 'a>> {
        Box::pin(async move {
            BrowserStore::save_document_expected(self, document, assets, expected).await
        })
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

type GeneratedAnalysisResult =
    Result<Rc<boardstudio_web_host::cad_jobs::CadGeneratedAnalysis>, String>;
struct PendingGeneratedAnalysis {
    task: String,
    batch: u64,
    waiters: Vec<futures_channel::oneshot::Sender<GeneratedAnalysisResult>>,
}

struct StepReadContext {
    scope: Scope,
    token: SnapshotToken,
    revision: u64,
    is_current: Rc<dyn Fn() -> bool>,
    collision: bool,
}

fn generated_analysis_cache_key(
    scene: &CadScene,
    geometry_identity: &str,
    target: Option<f64>,
) -> String {
    format!(
        "{:?}/{}/{geometry_identity}/{:?}",
        scene.scope,
        scene.token.0,
        target.map(f64::to_bits)
    )
}

pub struct Runtime {
    session: RefCell<Session>,
    core_services: CoreServices,
    document_core: Option<RefCell<DocumentCore>>,
    pub(crate) operation_outcomes: crate::operation_outcomes::OperationOutcomes,
    core: RefCell<Rc<dyn CoreExecutor>>,
    core_executor_factory: RefCell<Rc<CoreExecutorFactory>>,
    pub store: BrowserStore,
    persistence: RefCell<Rc<dyn DocumentPersistence>>,
    storage_session: RefCell<Option<StorageSession>>,
    storage_initializing: Cell<bool>,
    storage_error: RefCell<Option<String>>,
    saved_tokens: RefCell<BTreeMap<String, StorageToken>>,
    pending_document: RefCell<Option<Arc<ProjectDoc>>>,
    save_conflict: Cell<bool>,
    next_operation: Cell<u64>,
    assets: RefCell<BTreeMap<String, Vec<u8>>>,
    artifacts: RefCell<BTreeMap<String, Artifact>>,
    export_leases: RefCell<ExportLeases>,
    frames: RefCell<BTreeMap<u64, Frame>>,
    surface: RefCell<Option<SvgElement>>,
    notify: RefCell<Option<Notifier>>,
    status: RefCell<RuntimeReport>,
    notifications: RefCell<Vec<RuntimeNotification>>,
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
    latest_firmware_export: Cell<Option<OperationId>>,
    export_delivery_errors: RefCell<BTreeMap<OperationId, String>>,
    export_workers: RefCell<BTreeMap<OperationId, Rc<CadWorker>>>,
    native_case_preview: RefCell<crate::case_preview::NativePreviewState>,
    case_model_delivery: crate::model_delivery::ModelDeliveryAdapter,
    // Tight preparation must not consume the separate missing-evidence retry.
    collision_vendor_refinement: crate::model_delivery::ModelDeliveryAdapter,
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
    generated_analysis_cache:
        RefCell<BTreeMap<String, Rc<boardstudio_web_host::cad_jobs::CadGeneratedAnalysis>>>,
    generated_analysis_prepared:
        RefCell<BTreeMap<String, boardstudio_model::model::PreparedCaseAssemblyIR>>,
    generated_analysis_pending: RefCell<BTreeMap<String, PendingGeneratedAnalysis>>,
    collision_batch: Cell<u64>,
    collision_alive: RefCell<Rc<Cell<bool>>>,
    collision_generated_attempts: RefCell<BTreeSet<String>>,
    archive_export_options: ArchiveExportOptions,
    #[cfg(any(test, feature = "test-support"))]
    case_gesture_preview_test_executor: RefCell<Option<Rc<CaseGesturePreviewTestExecutor>>>,
    #[cfg(any(test, feature = "test-support"))]
    keycaps_preview_test_executor: RefCell<Option<Rc<KeycapsPreviewTestExecutor>>>,
    #[cfg(any(test, feature = "test-support"))]
    in_process_adapters: RefCell<Option<Rc<dyn test_hooks::InstalledAdapters>>>,
    #[cfg(any(test, feature = "test-support"))]
    held_effects: RefCell<Vec<Effect>>,
    #[cfg(any(test, feature = "test-support"))]
    in_process_drives: RefCell<Vec<futures_channel::oneshot::Receiver<()>>>,
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

mod collision;
mod effects;
mod export_bytes;
mod exports;
mod layout_preview;
mod mechanical_export;
mod projects;
mod scene_preview;
mod sources;

use mechanical_export::authored_case_filename;

impl Runtime {
    /// Keep one injected engine for the document; scratch probes create their own engines.
    /// Archive and artifact work remains on the auxiliary Core worker.
    pub fn new(core_services: CoreServices) -> Result<Rc<Self>, String> {
        let document_core = (core_services.open_scratch)();
        Self::new_with_restoration(true, core_services, Some(document_core))
    }

    fn new_with_restoration(
        restore_active_project: bool,
        core_services: CoreServices,
        document_core: Option<DocumentCore>,
    ) -> Result<Rc<Self>, String> {
        let prefix = deployment_prefix()?;
        let store = if restore_active_project {
            BrowserStore::scoped(if prefix == "/" { "root" } else { "boardstudio" })
        } else {
            BrowserStore::new(format!("boardstudio-test-{}", browser_uuid()?))
        }
        .map_err(|e| e.to_string())?;
        Self::new_with_store(restore_active_project, store, core_services, document_core)
    }

    fn new_with_store(
        restore_active_project: bool,
        store: BrowserStore,
        core_services: CoreServices,
        document_core: Option<DocumentCore>,
    ) -> Result<Rc<Self>, String> {
        #[cfg(not(any(test, feature = "test-support")))]
        let prefix = deployment_prefix()?;
        // Each executor (and each restart) owns a fresh scratch engine, as each worker did.
        let core_executor_factory: Rc<CoreExecutorFactory> = Rc::new(move || {
            Ok(Rc::new(InProcessCoreExecutor::new(
                (core_services.open_scratch)(),
                core_services.archive,
                core_services.artifact,
            )) as Rc<dyn CoreExecutor>)
        });
        let runtime = Rc::new(Self {
            session: RefCell::new(Session::new(core_services.validate_open)),
            core_services,
            document_core: document_core.map(RefCell::new),
            operation_outcomes: crate::operation_outcomes::OperationOutcomes::default(),
            core: RefCell::new(core_executor_factory.clone()()?),
            core_executor_factory: RefCell::new(core_executor_factory),
            persistence: RefCell::new(Rc::new(store.clone())),
            store,
            storage_session: RefCell::default(),
            storage_initializing: Cell::new(restore_active_project),
            storage_error: RefCell::new(None),
            saved_tokens: RefCell::default(),
            pending_document: RefCell::default(),
            save_conflict: Cell::new(false),
            next_operation: Cell::new(1),
            assets: RefCell::new(BTreeMap::new()),
            artifacts: RefCell::new(BTreeMap::new()),
            export_leases: RefCell::new(ExportLeases::default()),
            frames: RefCell::new(BTreeMap::new()),
            surface: RefCell::new(None),
            notify: RefCell::new(None),
            status: RefCell::new(RuntimeReport::status(
                "Open a saved keyboard or an editable demo copy.",
            )),
            notifications: RefCell::default(),
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
            latest_firmware_export: Cell::new(None),
            export_delivery_errors: RefCell::new(BTreeMap::new()),
            export_workers: RefCell::new(BTreeMap::new()),
            native_case_preview: RefCell::new(Default::default()),
            case_model_delivery: Default::default(),
            collision_vendor_refinement: Default::default(),
            native_model_delivery: RefCell::new(Default::default()),
            layout_preview: RefCell::new(Default::default()),
            layout_model_rows: RefCell::new(None),
            layout_model_batch_generation: Cell::new(0),
            native_model_jobs: RefCell::new(BTreeSet::new()),
            generated_analysis_cache: RefCell::new(BTreeMap::new()),
            generated_analysis_prepared: RefCell::new(BTreeMap::new()),
            generated_analysis_pending: RefCell::new(BTreeMap::new()),
            collision_batch: Cell::new(0),
            collision_alive: RefCell::new(Rc::new(Cell::new(true))),
            collision_generated_attempts: RefCell::new(BTreeSet::new()),
            archive_export_options: ArchiveExportOptions::default(),
            #[cfg(any(test, feature = "test-support"))]
            case_gesture_preview_test_executor: RefCell::new(None),
            #[cfg(any(test, feature = "test-support"))]
            keycaps_preview_test_executor: RefCell::new(None),
            #[cfg(any(test, feature = "test-support"))]
            in_process_adapters: RefCell::new(None),
            #[cfg(any(test, feature = "test-support"))]
            held_effects: RefCell::new(Vec::new()),
            #[cfg(any(test, feature = "test-support"))]
            in_process_drives: RefCell::new(Vec::new()),
        });
        // Storage cleanup finishes before any editor, import, or restored history can use assets.
        if restore_active_project {
            let weak = Rc::downgrade(&runtime);
            let store = runtime.store.clone();
            spawn_local(async move {
                let result = store.start_session().await;
                if let Some(runtime) = weak.upgrade() {
                    match result {
                        Ok((lease, cleanup_error)) => {
                            *runtime.storage_session.borrow_mut() = lease;
                            runtime.storage_initializing.set(false);
                            runtime.restore_active_project();
                            if let Some(error) = cleanup_error {
                                runtime.report(format!("Unused model cleanup was deferred; saved keyboards were kept: {error}"));
                            }
                            runtime.changed();
                        }
                        Err(error) => {
                            runtime.storage_initialization_failed(&error.to_string());
                        }
                    }
                }
            });
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
    pub fn operation(&self) -> OperationId {
        let id = self.next_operation.get();
        self.next_operation
            .set(id.checked_add(1).expect("operation identity exhausted"));
        OperationId(id)
    }
    pub fn scope(&self) -> Option<boardstudio_application::Scope> {
        self.session.borrow().scope()
    }
    pub fn electrical_preview_executor_epoch(&self) -> u64 {
        self.session.borrow().core_executor_epoch().0
    }
    /// The Core entry points the page injected (Open admission and scratch probes).
    pub fn core_services(&self) -> CoreServices {
        self.core_services
    }
    pub fn model(&self) -> ReadModel {
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

    /// Ask the document Core to project candidate matrices with its authoritative
    /// layout geometry. This is a private preview path; candidates are never installed in Session.
    pub async fn project_matrices(
        &self,
        base_revision: u64,
        matrices: Vec<boardstudio_model::model::Matrix>,
    ) -> Result<Vec<boardstudio_model::model::MatrixScene>, String> {
        let executor_epoch = self.session.borrow().core_executor_epoch();
        let core = self.core.borrow().clone();
        let request_id = format!("matrix-projection-{}", self.operation().0);
        let expected_ids: Vec<_> = matrices.iter().map(|matrix| matrix.id.clone()).collect();
        let request = CoreRequest::ProjectMatrices {
            id: request_id.clone(),
            base_revision,
            matrices,
        };
        let reply = if let Some(document_core) = &self.document_core {
            if self.session.borrow().read_model().lifecycle == Lifecycle::Closed {
                return Err("Matrix preview failed: editor is closed".into());
            }
            (document_core.borrow_mut())(request)
        } else {
            core.request(&request_id, &executor_epoch.0.to_string(), &request)
                .await
                .map_err(|error| format!("Matrix preview failed: {error}"))?
        };
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
    fn storage_initialization_failed(&self, error: &str) {
        let message =
            format!("Storage could not be initialized safely. Reload this page to retry. {error}");
        *self.storage_error.borrow_mut() = Some(message.clone());
        self.apply_report(RuntimeReport::alert(message));
    }

    /// Whether storage is ready for project imports, or why initialization failed.
    pub fn storage_readiness(&self) -> Result<bool, String> {
        match self.storage_error.borrow().as_ref() {
            Some(error) => Err(error.clone()),
            None => Ok(!self.storage_initializing.get()),
        }
    }
    pub fn status(&self) -> String {
        self.status.borrow().message.clone()
    }
    pub fn status_is_alert(&self) -> bool {
        self.status.borrow().severity == RuntimeReportSeverity::Alert
    }
    pub fn status_is_transient(&self) -> bool {
        self.status.borrow().notification_error.is_some()
    }
    /// Drain every pending result in order, including identical results between renders.
    /// Only the shell consumes this queue; reading it does not trigger another render.
    pub fn take_notifications(&self) -> Vec<RuntimeNotification> {
        std::mem::take(&mut *self.notifications.borrow_mut())
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
        if let Some(is_error) = report.notification_error {
            self.notifications.borrow_mut().push(RuntimeNotification {
                message: report.message.clone(),
                is_error,
            });
        }
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
}

#[cfg(all(test, target_arch = "wasm32"))]
mod case_preview_source_tests;

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

#[path = "collision_check.rs"]
mod collision_check;

pub use collision_check::{CollisionCheckInput, CollisionCheckReport};

#[cfg(all(test, target_arch = "wasm32"))]
#[path = "runtime_collision_browser_tests.rs"]
mod collision_browser_tests;

#[cfg(test)]
#[path = "source_pcb_export_tests.rs"]
mod source_pcb_export_tests;

#[cfg(all(test, target_arch = "wasm32"))]
#[path = "cad_runtime_borrow_tests.rs"]
mod cad_runtime_borrow_tests;

#[cfg(test)]
pub use crate::fixtures::firmware_export_test_support;

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
#[path = "test_hooks.rs"]
pub mod test_hooks;

#[cfg(test)]
pub use crate::fixtures::{in_process_support, project_name_test_support};

#[cfg(test)]
mod firmware_export_tests;

#[cfg(all(test, target_arch = "wasm32"))]
mod notification_tests;

#[cfg(all(test, target_arch = "wasm32"))]
mod cad_scene_rebind_tests;

#[cfg(all(test, target_arch = "wasm32"))]
mod project_open_supersession_tests;


#[cfg(all(test, target_arch = "wasm32"))]
mod offline_safety_tests;

/// Quota-recovery RED tests: with origin storage full every save aborts, so the
/// in-app delete path must not rewrite durable state before the single-key
/// delete, and create-new must discard the latched pending doc without
/// committing it. Saves are failed through the persistence port (the quota
/// simulation); single-key deletes go straight to the store and must succeed.
#[cfg(all(test, target_arch = "wasm32"))]
mod quota_recovery_deletion_tests;
