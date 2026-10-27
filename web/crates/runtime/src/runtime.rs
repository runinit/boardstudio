//! Browser composition runs identified effects; the headless session remains authoritative.
use crate::archive_export::{ArchiveExportOptions, ArchiveWorkFuture, archive_filename};
use crate::export_lease::{ExportKind, ExportLease, ExportLeases};
use crate::mechanical_package::{
    critical_fit_drawing, mechanical_fabrication_notes, mechanical_filename_component,
    mechanical_stl, push_mechanical_file,
};
use crate::pcb_wiring_mode_operation::electrical_preview_request;
use boardstudio_application::{
    AcceptedSnapshot, Completion, Durability, Effect, Event, JobId, Lifecycle, OperationId,
    ReadModel, SaveResult, Scope, Session, SnapshotToken, TerminalOutcome,
};
use boardstudio_core::{
    electrical::{ElectricalMode, ElectricalPlan, ElectricalPlanRequest},
    hash::sha256_hex,
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
use boardstudio_web_host::host::{
    BrowserStore, CoreWorker, SaveError, SavedProject, StorageSession, StorageToken,
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
    operation_outcomes: crate::operation_outcomes::OperationOutcomes,
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
    firmware_export_delivery_errors: RefCell<BTreeMap<OperationId, String>>,
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
        RefCell<BTreeMap<String, boardstudio_core::model::PreparedCaseAssemblyIR>>,
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
    in_process_adapters: RefCell<Option<Rc<crate::runtime::in_process_support::InProcessAdapters>>>,
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

impl Runtime {
    pub fn new() -> Result<Rc<Self>, String> {
        Self::new_with_restoration(true)
    }

    fn new_with_restoration(restore_active_project: bool) -> Result<Rc<Self>, String> {
        let prefix = deployment_prefix()?;
        let store = if restore_active_project {
            BrowserStore::scoped(if prefix == "/" { "root" } else { "boardstudio" })
        } else {
            BrowserStore::new(format!("boardstudio-test-{}", browser_uuid()?))
        }
        .map_err(|e| e.to_string())?;
        Self::new_with_store(restore_active_project, store)
    }

    fn new_with_store(
        restore_active_project: bool,
        store: BrowserStore,
    ) -> Result<Rc<Self>, String> {
        #[cfg(not(any(test, feature = "test-support")))]
        let prefix = deployment_prefix()?;
        let core_executor_factory: Rc<CoreExecutorFactory> = Rc::new(|| {
            resource_url("assets/core-worker/entry.js")
                .and_then(|url| CoreWorker::new(&url).map_err(|e| e.to_string()))
                .map(|worker| Rc::new(worker) as Rc<dyn CoreExecutor>)
        });
        let runtime = Rc::new(Self {
            session: RefCell::new(Session::new()),
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
            firmware_export_delivery_errors: RefCell::new(BTreeMap::new()),
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
        self.session.borrow().scope()
    }
    pub fn electrical_preview_executor_epoch(&self) -> u64 {
        self.session.borrow().core_executor_epoch().0
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

    fn create_new_keyboard_inner(
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
        let test_executor = self.keycaps_preview_test_executor.borrow().clone();
        #[cfg(any(test, feature = "test-support"))]
        if let Some(executor) = test_executor {
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
        if self.in_process_adapters.borrow().is_some() {
            self.held_effects.borrow_mut().extend(effects);
            return;
        }
        self.drive(effects);
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
        #[cfg(any(test, feature = "test-support"))]
        let finished = self.in_process_adapters.borrow().as_ref().map(|_| {
            let (finished, receive) = futures_channel::oneshot::channel();
            self.in_process_drives.borrow_mut().push(receive);
            finished
        });
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
            #[cfg(any(test, feature = "test-support"))]
            if let Some(finished) = finished {
                let _ = finished.send(());
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
                *self.pending_document.borrow_mut() = Some(document.clone());
                let expected = self.saved_tokens.borrow().get(&document.id).cloned();
                let result = match persistence
                    .save_document(&document, &assets, expected.as_ref())
                    .await
                {
                    Ok(token) => {
                        self.saved_tokens
                            .borrow_mut()
                            .insert(document.id.clone(), token);
                        self.pending_document.borrow_mut().take();
                        self.save_conflict.set(false);
                        for asset in &document.assets {
                            self.assets.borrow_mut().remove(&asset.sha256);
                        }
                        SaveResult::Committed
                    }
                    Err(error) => {
                        self.save_conflict.set(error == SaveError::Conflict);
                        SaveResult::Aborted(error.to_string())
                    }
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
                let lease = self.export_leases.borrow_mut().finish(operation_id);
                let kind = lease.as_ref().map(|lease| lease.kind().clone());
                let is_board_outline_export =
                    matches!(kind.as_ref(), Some(ExportKind::BoardOutline { .. }));
                let firmware_capture = (kind == Some(ExportKind::Firmware))
                    .then_some(lease.as_ref())
                    .flatten();
                let is_firmware_export = firmware_capture.is_some();
                let delivery_error = self
                    .firmware_export_delivery_errors
                    .borrow_mut()
                    .remove(&operation_id);
                self.archive_export_options.settle(operation_id);
                let firmware_owner_is_current = firmware_capture.is_some_and(|lease| {
                    self.export_source_is_current(lease)
                        && self.latest_firmware_export.get() == Some(operation_id)
                });
                if is_firmware_export {
                    if let Some(report) = firmware_export_terminal_report(
                        firmware_owner_is_current,
                        outcome,
                        delivery_error,
                    ) {
                        self.apply_report(report);
                    }
                } else if is_board_outline_export {
                    let lease_is_current = lease
                        .as_ref()
                        .is_some_and(|lease| self.export_source_is_current(lease));
                    if lease_is_current {
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
                let kind = self
                    .export_leases
                    .borrow()
                    .get(operation_id)
                    .map(|lease| lease.kind().clone());
                let is_step_export = matches!(kind.as_ref(), Some(ExportKind::Step));
                let is_keycaps_export = kind == Some(ExportKind::KeycapsStl);
                let is_firmware_export = kind == Some(ExportKind::Firmware);
                let is_footprint_export = kind == Some(ExportKind::Footprints);
                let is_pcb_handoff_export =
                    matches!(kind.as_ref(), Some(ExportKind::PcbHandoff { .. }));
                let is_mechanical_export =
                    matches!(kind.as_ref(), Some(ExportKind::Mechanical { .. }));
                let is_board_outline_export =
                    matches!(kind.as_ref(), Some(ExportKind::BoardOutline { .. }));
                let result = if let Some(ExportKind::BoardOutline { dxf }) = kind.as_ref() {
                    self.board_outline_bytes(operation_id, &snapshot, &scope, *dxf)
                        .await
                } else if is_keycaps_export {
                    self.keycaps_stl_bytes(operation_id, &snapshot, &scope)
                        .await
                        .map(|bytes| (bytes, snapshot.token))
                } else if is_pcb_handoff_export {
                    let draft =
                        matches!(kind.as_ref(), Some(ExportKind::PcbHandoff { draft: true }));
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
                    match work {
                        Ok(future) => future.await.map(|bytes| (bytes, snapshot.token)),
                        Err(reason) => Err(reason),
                    }
                };
                let result = result.and_then(|(bytes, token)| {
                    self.export_current(operation_id, token, &scope)
                        .then_some((bytes, token))
                        .ok_or_else(|| "Export scope changed before delivery.".to_owned())
                });
                match result {
                    Ok((bytes, delivery_token)) => {
                        let current = self.export_current(operation_id, delivery_token, &scope);
                        if !current {
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
                        } else if is_keycaps_export {
                            (
                                format!("{}-keycaps.zip", snapshot.document.name),
                                Some("application/zip".to_owned()),
                            )
                        } else if is_step_export {
                            (
                                authored_case_filename(&snapshot.document, &scope),
                                Some("model/step".to_owned()),
                            )
                        } else if is_pcb_handoff_export {
                            let draft = matches!(
                                kind.as_ref(),
                                Some(ExportKind::PcbHandoff { draft: true })
                            );
                            (
                                format!(
                                    "{}-{}pcb-handoff.zip",
                                    snapshot.document.name,
                                    if draft { "draft-" } else { "" }
                                ),
                                Some("application/zip".to_owned()),
                            )
                        } else if is_mechanical_export {
                            let filename = match kind.as_ref() {
                                Some(ExportKind::Mechanical { filename }) => filename.clone(),
                                _ => format!("{}-mechanical.zip", snapshot.document.name),
                            };
                            (filename, Some("application/zip".to_owned()))
                        } else if is_board_outline_export {
                            let dxf = matches!(
                                kind.as_ref(),
                                Some(ExportKind::BoardOutline { dxf: true })
                            );
                            (
                                format!(
                                    "{}.{}",
                                    snapshot.document.name,
                                    if dxf { "dxf" } else { "svg" }
                                ),
                                Some(
                                    if dxf {
                                        "application/dxf"
                                    } else {
                                        "image/svg+xml"
                                    }
                                    .to_owned(),
                                ),
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
                                token: delivery_token,
                                firmware: is_firmware_export,
                                board_outline: is_board_outline_export,
                            },
                        );
                        self.complete(Completion::ExportFinished {
                            operation_id,
                            token: delivery_token,
                            scope: scope.clone(),
                            artifact_id,
                        })
                    }
                    Err(reason) => self.complete(Completion::ExportFailed {
                        operation_id,
                        reason,
                    }),
                }
            }
            Effect::DeliverExport {
                artifact_id, token, ..
            } => {
                let artifact = self.artifacts.borrow_mut().remove(&artifact_id);
                if let Some(artifact) = artifact
                    && token == artifact.token
                    && self.export_delivery_is_current(
                        artifact.operation_id,
                        token,
                        &artifact.scope,
                    )
                {
                    match self.deliver_artifact(&artifact) {
                        Ok(()) if artifact.board_outline => {
                            self.report(format!("Saved {}.", artifact.filename));
                        }
                        Ok(()) => {}
                        Err(error) if artifact.board_outline => {
                            self.apply_report(RuntimeReport::alert(error));
                        }
                        Err(error) if artifact.firmware => {
                            self.firmware_export_delivery_errors
                                .borrow_mut()
                                .insert(artifact.operation_id, error);
                        }
                        Err(error) => self.report(error),
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
                self.export_leases.borrow_mut().cancel(operation_id);
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
            source_sha256: None,
            step_import_options: None,
            collision_query: None,
            generated_analysis_request: None,
            collision_meshes: None,
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
        let digest = sha256_hex(&bytes);
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
        bytes: crate::model_delivery::VerifiedModelBytes,
        import_options: boardstudio_web_host::cad_jobs::CadStepImportOptions,
        context: StepReadContext,
    ) -> Result<crate::model_delivery::MeshArrays, String> {
        let StepReadContext {
            scope,
            token,
            revision,
            is_current,
            collision,
        } = context;
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
        let prefix = if collision {
            "case-model-evidence-step"
        } else if import_options.target_deflection_mm.is_some() {
            "case-model-refine-step"
        } else {
            "case-model-step"
        };
        let request = CadRequest {
            request_id: format!("{prefix}-{revision}-{operation}"),
            job_id: format!("{prefix}-{revision}-{operation}"),
            identity: identity.clone(),
            operation: CadOperation::ReadStep,
            prepared: None,
            source_sha256: Some(bytes.sha256().to_owned()),
            step_import_options: Some(import_options),
            collision_query: None,
            generated_analysis_request: None,
            collision_meshes: None,
            input_bytes: bytes.bytes().to_vec(),
        };
        self.native_model_jobs
            .borrow_mut()
            .insert(request.job_id.clone());
        let reply = worker.request(request.clone()).await;
        let active = self.native_model_jobs.borrow_mut().remove(&request.job_id);
        let reply = reply.map_err(|error| error.to_string())?;
        if !active || !is_current() || self.scope().as_ref() != Some(&scope) {
            let _ = worker.cancel(&request.job_id);
            return Err("STEP model request became stale after parsing".into());
        }
        let result = validate_reply(&request, reply, &identity).map_err(|error| {
            let _ = worker.cancel(&request.job_id);
            format!("{error:?}")
        })?;
        let mesh = result
            .mesh
            .ok_or_else(|| "STEP model reader returned no mesh".to_owned())?;
        Ok(crate::model_delivery::MeshArrays {
            positions: mesh.positions,
            normals: mesh.normals,
            colors: None,
            step_bounds: result.bounds,
            vendor_step: result.vendor_step.map(Rc::new),
        })
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn query_vendor_step_collision(
        &self,
        left: &crate::model_delivery::DeliveredModel,
        left_explicit_pcb_transform: Option<[f64; 16]>,
        right: &crate::model_delivery::DeliveredModel,
        right_explicit_pcb_transform: Option<[f64; 16]>,
        clearance_threshold_mm: f64,
        requested_accuracy_mm: Option<f64>,
        scope: Scope,
        token: SnapshotToken,
        revision: u64,
        is_current: Rc<dyn Fn() -> bool>,
    ) -> Result<crate::model_delivery::CadCollisionResult, String> {
        use crate::model_delivery::{CadCollisionSubject, resolve_physical_model_row_transform};
        let left = CadCollisionSubject::VendorModel {
            id: left.id.clone(),
            reference: left.id.clone(),
            row_transform_f64: resolve_physical_model_row_transform(
                left,
                left_explicit_pcb_transform,
            )?,
            vendor_step: left.mesh.vendor_step.clone(),
        };
        let right = CadCollisionSubject::VendorModel {
            id: right.id.clone(),
            reference: right.id.clone(),
            row_transform_f64: resolve_physical_model_row_transform(
                right,
                right_explicit_pcb_transform,
            )?,
            vendor_step: right.mesh.vendor_step.clone(),
        };
        self.query_collision_subjects(
            &left,
            &right,
            clearance_threshold_mm,
            requested_accuracy_mm,
            scope,
            token,
            revision,
            is_current,
        )
        .await
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn query_collision_subjects(
        &self,
        left: &crate::model_delivery::CadCollisionSubject,
        right: &crate::model_delivery::CadCollisionSubject,
        clearance_threshold_mm: f64,
        requested_accuracy_mm: Option<f64>,
        scope: Scope,
        token: SnapshotToken,
        revision: u64,
        is_current: Rc<dyn Fn() -> bool>,
    ) -> Result<crate::model_delivery::CadCollisionResult, String> {
        use crate::model_delivery::{CadCollisionAssessment, CadCollisionResult};
        use boardstudio_web_host::{cad_jobs::CadCollisionQuery, cad_qualification::MeshDistance};
        let batch = self.collision_batch.get();
        let identity = CadSnapshotIdentity {
            token: token.0,
            session_epoch: scope.session_epoch.0,
            document_id: scope.document_id.clone(),
            board_id: scope.board_id.clone(),
            instance_id: scope.instance_id.clone(),
            revision,
        };
        let mut subjects = [left.clone(), right.clone()];
        for attempt in 0..2 {
            if batch != self.collision_batch.get()
                || !is_current()
                || !self.collision_snapshot_is_current(&scope, token, revision)
            {
                return Err("collision request became stale before dispatch".into());
            }
            let operands = subjects[0]
                .operand()
                .and_then(|left| subjects[1].operand().map(|right| (left, right)));
            let (left, right) = match operands {
                Ok(operands) => operands,
                Err(reason) => {
                    return Ok(CadCollisionResult {
                        source_interval_mm: None,
                        query_qualification: None,
                        observation: None,
                        left_provenance: None,
                        right_provenance: None,
                        assessment: CadCollisionAssessment::Unknown {
                            reasons: vec![reason],
                            refinement_requested: requested_accuracy_mm.is_some(),
                        },
                    });
                }
            };
            let query = CadCollisionQuery {
                left,
                right,
                clearance_threshold_mm,
                requested_accuracy_mm,
                qualification_limits: Default::default(),
            };
            let mut result = self
                .dispatch_collision_subjects(
                    [&subjects[0], &subjects[1]],
                    query,
                    scope.clone(),
                    identity.clone(),
                    is_current.clone(),
                )
                .await?;
            let unavailable = result.query_qualification.as_ref().is_some_and(|r| matches!(&r.mesh_distance, MeshDistance::Unknown { reasons } if reasons == &["evidence-unavailable"]));
            if attempt == 1 || !unavailable {
                return Ok(result);
            }
            for subject in &mut subjects {
                match self
                    .refresh_collision_subject(subject, &scope, token, revision, is_current.clone())
                    .await
                {
                    Ok(refreshed) => *subject = refreshed,
                    Err(error) => {
                        if let CadCollisionAssessment::Unknown { reasons, .. } =
                            &mut result.assessment
                        {
                            reasons.push(error);
                        }
                        if batch != self.collision_batch.get() || !is_current() {
                            return Err("collision preparation became stale".into());
                        }
                        return Ok(result);
                    }
                }
            }
        }
        unreachable!("bounded collision evidence recovery")
    }

    async fn dispatch_collision_subjects(
        &self,
        subjects: [&crate::model_delivery::CadCollisionSubject; 2],
        query: boardstudio_web_host::cad_jobs::CadCollisionQuery,
        scope: Scope,
        identity: CadSnapshotIdentity,
        is_current: Rc<dyn Fn() -> bool>,
    ) -> Result<crate::model_delivery::CadCollisionResult, String> {
        let [left, right] = subjects;
        let token = SnapshotToken(identity.token);
        let revision = identity.revision;
        let clearance_threshold_mm = query.clearance_threshold_mm;
        let requested_accuracy_mm = query.requested_accuracy_mm;
        let batch = self.collision_batch.get();
        use crate::model_delivery::assess_subject_collision_observation;
        use boardstudio_web_host::cad_jobs::CadCollisionMeshPair;
        if !is_current() || !self.collision_snapshot_is_current(&scope, token, revision) {
            return Err("collision request became stale before worker setup".into());
        }
        let (left_positions, left_tags) = left.mesh().ok_or("left analysis mesh is unavailable")?;
        let (right_positions, right_tags) =
            right.mesh().ok_or("right analysis mesh is unavailable")?;
        let pair_bytes = left_positions
            .len()
            .saturating_mul(8)
            .saturating_add(left_tags.len().saturating_mul(4))
            .saturating_add(right_positions.len().saturating_mul(8))
            .saturating_add(right_tags.len().saturating_mul(4));
        if pair_bytes > boardstudio_web_host::cad_jobs::MAX_COLLISION_QUERY_BYTES {
            return Err("collision pair exceeds the 64 MiB limit before copying".into());
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
        if self.collision_batch.get() != batch
            || !is_current()
            || !self.collision_snapshot_is_current(&scope, token, revision)
        {
            return Err("collision request became stale before dispatch".into());
        }
        let operation = self.operation().0;
        let request = CadRequest {
            request_id: format!("case-model-collision-{revision}-{operation}"),
            job_id: format!("case-model-collision-{revision}-{operation}"),
            identity: identity.clone(),
            operation: CadOperation::CollisionQuery,
            prepared: None,
            source_sha256: None,
            step_import_options: None,
            generated_analysis_request: None,
            collision_query: Some(query),
            collision_meshes: Some(CadCollisionMeshPair {
                left_positions_mm_f64: left_positions.to_vec(),
                left_triangle_face_occurrence_indices: left_tags.to_vec(),
                right_positions_mm_f64: right_positions.to_vec(),
                right_triangle_face_occurrence_indices: right_tags.to_vec(),
            }),
            input_bytes: Vec::new(),
        };
        self.native_model_jobs
            .borrow_mut()
            .insert(request.job_id.clone());
        let reply = worker.request(request.clone()).await;
        let active = self.native_model_jobs.borrow_mut().remove(&request.job_id);
        let reply = reply.map_err(|error| error.to_string())?;
        if !active
            || self.collision_batch.get() != batch
            || !is_current()
            || !self.collision_snapshot_is_current(&scope, token, revision)
        {
            return Err("collision request became stale after parsing".into());
        }
        let result =
            validate_reply(&request, reply, &identity).map_err(|error| format!("{error:?}"))?;
        let query = request
            .collision_query
            .as_ref()
            .ok_or("missing collision request")?;
        let report = result
            .query_qualification
            .clone()
            .ok_or("missing query qualification")?;
        let expected = boardstudio_web_host::cad_qualification::expected_binding(
            query,
            &identity,
            &report.binding.worker_generation,
        )?;
        let validated = boardstudio_web_host::cad_qualification::validate_query_qualification(
            report,
            &expected,
            &query.qualification_limits,
            request
                .collision_meshes
                .as_ref()
                .ok_or("missing collision buffers")?,
            left.source_qualification()
                .zip(right.source_qualification())
                .map(|(left, right)| [left, right]),
        )?;
        Ok(assess_subject_collision_observation(
            left,
            right,
            clearance_threshold_mm,
            requested_accuracy_mm,
            Some((&validated, &expected)),
            result
                .collision
                .ok_or("collision query returned no observation")?,
        ))
    }

    pub fn cancel_collision_checks(&self) {
        self.collision_alive.borrow().set(false);
        *self.collision_alive.borrow_mut() = Rc::new(Cell::new(true));
        self.collision_batch.set(
            self.collision_batch
                .get()
                .checked_add(1)
                .expect("collision batch exhausted"),
        );
        self.collision_generated_attempts.borrow_mut().clear();
        let jobs = self
            .native_model_jobs
            .borrow()
            .iter()
            .filter(|job| {
                job.starts_with("case-model-collision-")
                    || job.starts_with("case-model-analysis-")
                    || job.starts_with("case-model-refine-step-")
                    || job.starts_with("case-model-evidence-step-")
            })
            .cloned()
            .collect::<Vec<_>>();
        for job in jobs {
            self.native_model_jobs.borrow_mut().remove(&job);
            if let Some((_, worker)) = self.cad_worker.borrow().as_ref() {
                let _ = worker.cancel(&job);
            }
        }
    }

    fn collision_snapshot_is_current(
        &self,
        scope: &Scope,
        token: SnapshotToken,
        revision: u64,
    ) -> bool {
        self.scope().as_ref() == Some(scope)
            && self.model().accepted.is_some_and(|accepted| {
                accepted.token == token
                    && accepted.document.revision == revision
                    && accepted.session_epoch == scope.session_epoch
                    && accepted.document.id == scope.document_id
            })
    }

    fn collision_current(&self, is_current: Rc<dyn Fn() -> bool>) -> Rc<dyn Fn() -> bool> {
        let alive = self.collision_alive.borrow().clone();
        Rc::new(move || alive.get() && is_current())
    }

    async fn verified_collision_step_bytes(
        &self,
        sha: &str,
    ) -> Result<crate::model_delivery::VerifiedModelBytes, String> {
        let cached = self.assets.borrow().get(sha).cloned();
        let bytes = match cached {
            Some(bytes) => bytes,
            None => self
                .store
                .load_asset(sha.into())
                .await
                .map_err(|e| e.to_string())?
                .ok_or("original vendor STEP bytes are unavailable")?
                .to_vec(),
        };
        crate::model_delivery::VerifiedModelBytes::verify(bytes, sha)
    }

    async fn refresh_collision_subject(
        &self,
        subject: &crate::model_delivery::CadCollisionSubject,
        scope: &Scope,
        token: SnapshotToken,
        revision: u64,
        is_current: Rc<dyn Fn() -> bool>,
    ) -> Result<crate::model_delivery::CadCollisionSubject, String> {
        use crate::model_delivery::CadCollisionSubject;
        let current = self.collision_current(is_current);
        if !current() || !self.collision_snapshot_is_current(scope, token, revision) {
            return Err("evidence preparation became stale".into());
        }
        let mut refreshed = subject.clone();
        match &mut refreshed {
            CadCollisionSubject::VendorModel {
                vendor_step: Some(asset),
                ..
            } => {
                let batch = format!(
                    "{scope:?}/{}/{revision}/{}",
                    token.0,
                    self.collision_batch.get()
                );
                let options = asset.import_options;
                let sha = asset.source_sha256.clone();
                let load = async {
                    let bytes = self.verified_collision_step_bytes(&sha).await?;
                    self.read_step_model(
                        bytes,
                        options,
                        StepReadContext {
                            scope: scope.clone(),
                            token,
                            revision,
                            is_current: current.clone(),
                            collision: true,
                        },
                    )
                    .await
                };
                let mesh = self
                    .case_model_delivery
                    .refresh_step_evidence(&sha, options, &batch, current.clone(), load)
                    .await?;
                *asset = mesh
                    .vendor_step
                    .clone()
                    .ok_or("reprepared STEP omitted full source asset")?;
            }
            CadCollisionSubject::GeneratedCaseBody {
                analysis: Some(asset),
                ..
            } => {
                let scene = self
                    .cad_scene()
                    .filter(|s| {
                        s.scope == *scope
                            && s.token == token
                            && s.snapshot.document.revision == revision
                    })
                    .ok_or("generated evidence has no current exact scene")?;
                let target = asset
                    .analysis_mesh
                    .as_ref()
                    .ok_or("generated evidence omitted descriptor")?
                    .settings
                    .target_deflection_mm;
                let key = generated_analysis_cache_key(&scene, &asset.geometry_identity, target);
                let first = self
                    .collision_generated_attempts
                    .borrow_mut()
                    .insert(key.clone());
                if first {
                    self.generated_analysis_cache.borrow_mut().remove(&key);
                }
                if !first
                    && !self.generated_analysis_cache.borrow().contains_key(&key)
                    && !self.generated_analysis_pending.borrow().contains_key(&key)
                {
                    return Err(
                        "generated evidence preparation already failed in this batch".into(),
                    );
                }
                let prepared = self
                    .generated_analysis_prepared
                    .borrow()
                    .get(&asset.geometry_identity)
                    .cloned()
                    .ok_or("complete prepared source for generated evidence is unavailable")?;
                *asset = self
                    .analyze_prepared_generated_body(
                        &scene,
                        &prepared,
                        &asset.body_id,
                        target,
                        current.clone(),
                    )
                    .await?;
            }
            _ => return Err("source evidence cannot be prepared".into()),
        }
        if !current() || !self.collision_snapshot_is_current(scope, token, revision) {
            return Err("evidence preparation became stale after settlement".into());
        }
        Ok(refreshed)
    }

    pub async fn refine_collision_vendor(
        &self,
        scene: &CadScene,
        subject: &crate::model_delivery::CadCollisionSubject,
        target_deflection_mm: f64,
        is_current: Rc<dyn Fn() -> bool>,
    ) -> Result<crate::model_delivery::CadCollisionSubject, String> {
        use crate::model_delivery::CadCollisionSubject;
        let CadCollisionSubject::VendorModel {
            id,
            reference,
            row_transform_f64,
            vendor_step: Some(asset),
        } = subject
        else {
            return Err(format!(
                "{} has no retained vendor STEP source for refinement",
                subject.label()
            ));
        };
        if !is_current()
            || !self.collision_snapshot_is_current(
                &scene.scope,
                scene.token,
                scene.snapshot.document.revision,
            )
        {
            return Err("vendor refinement became stale".into());
        }
        let is_current = self.collision_current(is_current);
        let options = boardstudio_web_host::cad_jobs::CadStepImportOptions {
            target_deflection_mm: Some(target_deflection_mm),
            ..asset.import_options
        };
        options.validate()?;
        let batch = format!(
            "{:?}/{}/{}/{}",
            scene.scope,
            scene.token.0,
            scene.snapshot.document.revision,
            self.collision_batch.get()
        );
        let load = async {
            let bytes = self
                .verified_collision_step_bytes(&asset.source_sha256)
                .await?;
            self.read_step_model(
                bytes,
                options,
                StepReadContext {
                    scope: scene.scope.clone(),
                    token: scene.token,
                    revision: scene.snapshot.document.revision,
                    is_current: is_current.clone(),
                    collision: true,
                },
            )
            .await
        };
        let arrays = self
            .collision_vendor_refinement
            .refresh_step_evidence(
                &asset.source_sha256,
                options,
                &batch,
                is_current.clone(),
                load,
            )
            .await?;
        Ok(CadCollisionSubject::VendorModel {
            id: id.clone(),
            reference: reference.clone(),
            row_transform_f64: *row_transform_f64,
            vendor_step: arrays.vendor_step.clone(),
        })
    }

    pub async fn generated_collision_subjects(
        self: &Rc<Self>,
        scene: &CadScene,
        accuracy: Option<f64>,
        current: Rc<dyn Fn() -> bool>,
    ) -> (Vec<crate::model_delivery::CadCollisionSubject>, Vec<String>) {
        use crate::model_delivery::{CadCollisionSubject, IDENTITY_ROW_TRANSFORM_F64};
        let mut subjects = Vec::new();
        let mut errors = Vec::new();
        for body in scene
            .prepared
            .bodies
            .iter()
            .filter(|body| body.body.id != "pcb-reference")
        {
            let analysis = match self
                .analyze_generated_case_body(scene, &body.body.id, accuracy, current.clone())
                .await
            {
                Ok(analysis) => Some(analysis),
                Err(error) => {
                    errors.push(format!("{}: {error}", body.body.name));
                    None
                }
            };
            subjects.push(CadCollisionSubject::GeneratedCaseBody {
                id: body.body.id.clone(),
                name: body.body.name.clone(),
                row_transform_f64: IDENTITY_ROW_TRANSFORM_F64,
                analysis,
            });
            if !current() {
                return (Vec::new(), Vec::new());
            }
        }
        let core = self.core.borrow().clone();
        let epoch = self.session.borrow().core_executor_epoch();
        let pcb_current: Rc<dyn Fn() -> bool> = {
            let runtime = self.clone();
            let core = core.clone();
            let current = current.clone();
            let scope = scene.scope.clone();
            let token = scene.token;
            let revision = scene.snapshot.document.revision;
            let scene_identity = scene as *const CadScene as usize;
            Rc::new(move || {
                current()
                    && runtime.collision_snapshot_is_current(&scope, token, revision)
                    && runtime.session.borrow().core_executor_epoch() == epoch
                    && Rc::ptr_eq(&core, &runtime.core.borrow())
                    && runtime
                        .cad_scene()
                        .is_some_and(|scene| Rc::as_ptr(&scene) as usize == scene_identity)
            })
        };
        let pcb = async {
            let physical = self
                .prepare_physical_step_assembly(
                    core.as_ref(),
                    &epoch.0.to_string(),
                    &format!("collision-pcb-{}", self.operation().0),
                    &scene.snapshot,
                    &scene.scope,
                    pcb_current.clone(),
                )
                .await?;
            self.analyze_prepared_generated_body(
                scene,
                &physical.prepared.prepared,
                "pcb-reference",
                accuracy,
                pcb_current.clone(),
            )
            .await
        }
        .await;
        let analysis = match pcb {
            Ok(analysis) if pcb_current() => Some(analysis),
            Ok(_) => {
                errors.push("PCB unavailable: physical source preparation became stale".into());
                None
            }
            Err(error) => {
                errors.push(format!("PCB unavailable: {error}"));
                None
            }
        };
        if !current() {
            return (Vec::new(), Vec::new());
        }
        subjects.push(CadCollisionSubject::GeneratedCaseBody {
            id: "pcb-reference".into(),
            name: "PCB".into(),
            row_transform_f64: IDENTITY_ROW_TRANSFORM_F64,
            analysis,
        });
        (subjects, errors)
    }

    pub async fn analyze_generated_case_body(
        &self,
        scene: &CadScene,
        body_id: &str,
        target_deflection_mm: Option<f64>,
        is_current: Rc<dyn Fn() -> bool>,
    ) -> Result<Rc<boardstudio_web_host::cad_jobs::CadGeneratedAnalysis>, String> {
        self.analyze_prepared_generated_body(
            scene,
            &scene.prepared,
            body_id,
            target_deflection_mm,
            is_current,
        )
        .await
    }

    async fn analyze_prepared_generated_body(
        &self,
        scene: &CadScene,
        prepared: &boardstudio_core::model::PreparedCaseAssemblyIR,
        body_id: &str,
        target_deflection_mm: Option<f64>,
        is_current: Rc<dyn Fn() -> bool>,
    ) -> Result<Rc<boardstudio_web_host::cad_jobs::CadGeneratedAnalysis>, String> {
        use boardstudio_web_host::cad_jobs::{
            CadGeneratedAnalysisRequest, generated_geometry_identity,
        };
        let is_current = self.collision_current(is_current);
        let revision = scene.snapshot.document.revision;
        if !is_current()
            || !scene.exact
            || !self.collision_snapshot_is_current(&scene.scope, scene.token, revision)
            || !self
                .cad_scene()
                .is_some_and(|current| std::ptr::eq(current.as_ref(), scene))
        {
            return Err("generated analysis requires the current exact scene".into());
        }
        let body = prepared
            .bodies
            .iter()
            .find(|body| body.body.id == body_id)
            .ok_or("generated analysis body is not in the prepared scene")?;
        let metadata = CadGeneratedAnalysisRequest {
            body_id: body_id.into(),
            geometry_identity: generated_geometry_identity(body)?,
            target_deflection_mm,
        };
        boardstudio_web_host::cad_jobs::validate_generated_analysis_request(
            Some(prepared),
            revision,
            &metadata,
        )?;
        let cache_key =
            generated_analysis_cache_key(scene, &metadata.geometry_identity, target_deflection_mm);
        if let Some(cached) = self.generated_analysis_cache.borrow().get(&cache_key) {
            boardstudio_web_host::cad_jobs::validate_generated_analysis(
                &metadata, revision, cached,
            )?;
            return Ok(cached.clone());
        }
        let worker = crate::case_model_lifecycle::case_model_worker(
            &self.cad_worker,
            &scene.scope,
            |worker| !worker.is_closed(),
            || {
                CadWorker::new(&resource_url("assets/cad-worker/entry.js")?)
                    .map(Rc::new)
                    .map_err(|error| error.to_string())
            },
        )?;
        worker.ready().await.map_err(|error| error.to_string())?;
        if !is_current() || !self.collision_snapshot_is_current(&scene.scope, scene.token, revision)
        {
            return Err("generated analysis became stale before dispatch".into());
        }
        let receiver = {
            let mut pending = self.generated_analysis_pending.borrow_mut();
            if pending
                .get(&cache_key)
                .is_some_and(|entry| entry.batch != self.collision_batch.get())
                && let Some(old) = pending.remove(&cache_key)
            {
                for waiter in old.waiters {
                    let _ = waiter.send(Err("generated analysis batch was retired".into()));
                }
            }
            pending.get_mut(&cache_key).map(|pending| {
                let (sender, receiver) = futures_channel::oneshot::channel();
                pending.waiters.push(sender);
                receiver
            })
        };
        if let Some(receiver) = receiver {
            let result = receiver
                .await
                .map_err(|_| "generated analysis task was abandoned")?;
            if !is_current()
                || !self.collision_snapshot_is_current(&scene.scope, scene.token, revision)
            {
                return Err("joined generated analysis became stale".into());
            }
            return result;
        }
        let operation = self.operation().0;
        let identity = CadSnapshotIdentity {
            token: scene.token.0,
            session_epoch: scene.scope.session_epoch.0,
            document_id: scene.scope.document_id.clone(),
            board_id: scene.scope.board_id.clone(),
            instance_id: scene.scope.instance_id.clone(),
            revision,
        };
        let request = CadRequest {
            request_id: format!("case-model-analysis-{revision}-{operation}"),
            job_id: format!("case-model-analysis-{revision}-{operation}"),
            identity: identity.clone(),
            operation: CadOperation::AnalyzeGeneratedBody,
            prepared: Some(prepared.clone()),
            generated_analysis_request: Some(metadata),
            source_sha256: None,
            step_import_options: None,
            collision_query: None,
            collision_meshes: None,
            input_bytes: Vec::new(),
        };
        self.native_model_jobs
            .borrow_mut()
            .insert(request.job_id.clone());
        self.generated_analysis_pending.borrow_mut().insert(
            cache_key.clone(),
            PendingGeneratedAnalysis {
                task: request.job_id.clone(),
                batch: self.collision_batch.get(),
                waiters: Vec::new(),
            },
        );
        let result: GeneratedAnalysisResult = async {
            let reply = worker.request(request.clone()).await;
            let active = self.native_model_jobs.borrow_mut().remove(&request.job_id);
            let reply = reply.map_err(|error| error.to_string())?;
            if !active
                || !is_current()
                || !self.collision_snapshot_is_current(&scene.scope, scene.token, revision)
                || !self
                    .cad_scene()
                    .is_some_and(|current| std::ptr::eq(current.as_ref(), scene))
            {
                return Err("generated analysis became stale after dispatch".into());
            }
            let result =
                validate_reply(&request, reply, &identity).map_err(|error| format!("{error:?}"))?;
            let analysis = Rc::new(
                result
                    .generated_analysis
                    .ok_or("generated analysis returned no payload")?,
            );
            Ok(analysis)
        }
        .await;
        let pending = self
            .generated_analysis_pending
            .borrow_mut()
            .remove(&cache_key)
            .ok_or("generated task lost settlement authority")?;
        if result.is_err() {
            let _ = worker.cancel(&request.job_id);
        }
        if pending.task != request.job_id {
            self.generated_analysis_pending
                .borrow_mut()
                .insert(cache_key, pending);
            return Err("generated task was superseded before settlement".into());
        }
        for waiter in pending.waiters {
            let _ = waiter.send(result.clone());
        }
        let analysis = result?;
        let mut cache = self.generated_analysis_cache.borrow_mut();
        let bytes = |asset: &boardstudio_web_host::cad_jobs::CadGeneratedAnalysis| {
            asset.surface_positions_mm_f64.len() * 8
                + asset.triangle_face_occurrence_indices.len() * 4
        };
        if cache.len() >= 64
            || cache.values().map(|asset| bytes(asset)).sum::<usize>() + bytes(&analysis)
                > 64 * 1024 * 1024
        {
            cache.clear();
            self.generated_analysis_prepared.borrow_mut().clear();
        }
        self.generated_analysis_prepared.borrow_mut().insert(
            analysis.geometry_identity.clone(),
            boardstudio_core::model::PreparedCaseAssemblyIR {
                revision,
                bodies: vec![body.clone()],
            },
        );
        cache.insert(cache_key, analysis.clone());
        Ok(analysis)
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
                        .read_step_model(
                            bytes,
                            Default::default(),
                            StepReadContext {
                                scope,
                                token,
                                revision,
                                is_current,
                                collision: false,
                            },
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
        self.begin_export_lease(operation_id, ExportKind::Step, snapshot, &scope);
        self.submit(Event::StartExport {
            operation_id,
            scope,
        });
    }

    pub fn export_keycaps_stl(self: &Rc<Self>) {
        let Some(scope) = self.scope() else {
            self.apply_report(RuntimeReport::alert(
                "Select a board before exporting keycaps.",
            ));
            return;
        };
        let Some(snapshot) = self.model().accepted else {
            return;
        };
        if let Err(error) = self.ensure_keycaps_source_current(&snapshot, &scope) {
            self.apply_report(RuntimeReport::alert(error));
            return;
        }
        self.clear_alert();
        let operation_id = self.operation();
        self.begin_export_lease(operation_id, ExportKind::KeycapsStl, &snapshot, &scope);
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
        self.begin_export_lease(
            operation_id,
            ExportKind::Mechanical {
                filename: format!(
                    "{}{}-mechanical.zip",
                    snapshot.document.name, instance_suffix
                ),
            },
            &snapshot,
            &scope,
        );
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
        self.latest_firmware_export.set(Some(operation_id));
        self.begin_export_lease(operation_id, ExportKind::Firmware, &snapshot, &scope);
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
        self.begin_export_lease(operation_id, ExportKind::Footprints, &snapshot, &scope);
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
        self.begin_export_lease(
            operation_id,
            ExportKind::PcbHandoff { draft },
            &snapshot,
            &scope,
        );
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
            .export_leases
            .borrow()
            .get(operation_id)
            .cloned()
            .ok_or_else(|| "KiCad export owner was cancelled or superseded.".to_owned())?;
        if !matches!(capture.kind(), ExportKind::PcbHandoff { draft: captured_draft } if *captured_draft == draft)
            || capture.scope() != scope
            || capture.token() != initial_snapshot.token
            || capture.snapshot().document.revision != initial_snapshot.document.revision
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
                        if runtime.export_lease_is_current(
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
                            executor_epoch: package_capture.executor_epoch().0,
                        },
                        is_current,
                    )
                    .await
                }
            },
            || {
                if self.export_lease_is_current(operation_id, &capture, &core) {
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
                if self.export_lease_is_current(operation_id, protected_capture, &core) {
                    Ok(())
                } else {
                    Err("KiCad handoff was superseded before delivery.".into())
                }
            },
        )
        .await?;
        capture = protected_capture;
        Ok((archive, capture.token()))
    }

    /// Share complete physical PCB preparation between export and collision.
    async fn prepare_physical_step_assembly(
        &self,
        core: &dyn boardstudio_web_host::host::CoreExecutor,
        executor_epoch: &str,
        request_prefix: &str,
        snapshot: &AcceptedSnapshot,
        scope: &Scope,
        is_current: Rc<dyn Fn() -> bool>,
    ) -> Result<PhysicalStepAssembly, String> {
        let ensure_current = || {
            if is_current() {
                Ok(())
            } else {
                Err("Physical PCB preparation was cancelled or became stale.".to_owned())
            }
        };
        ensure_current()?;
        let mut prepared =
            prepare_captured_step_assembly(core, executor_epoch, request_prefix, snapshot, scope)
                .await
                .map_err(|error| format!("Mechanical assembly resolution failed: {error:?}"))?;
        ensure_current()?;
        let pcb_source = captured_pcb_source(snapshot, scope)
            .map_err(|error| format!("Could not capture imported PCB source: {error:?}"))?;
        let mut pcb_source_thickness = None;
        if let Some(source) = &pcb_source {
            let source_is_current = is_current.clone();
            let bytes = self
                .load_preview_document_asset(&source.asset, source_is_current)
                .await?;
            ensure_current()?;
            let request_id = format!("{request_prefix}-pcb-source");
            let request = ArtifactRequest::ReadBoardSource {
                id: request_id.clone(),
                source: String::from_utf8(bytes)
                    .map_err(|error| format!("PCB source is not valid UTF-8: {error}"))?,
                placement: source.placement,
                flipped: source.flipped,
            };
            let reply = core
                .artifact(&request_id, executor_epoch, &request)
                .await
                .map_err(|error| format!("Could not read complete PCB source: {error}"))?;
            ensure_current()?;
            let board = match reply {
                ArtifactReply::ReadBoardSource { id, result } if id == request_id => result,
                ArtifactReply::Error { id, error } if id == request_id => return Err(error.message),
                _ => return Err("Core returned PCB source for another request.".into()),
            };
            let indices = prepared
                .prepared
                .bodies
                .iter()
                .enumerate()
                .filter(|(_, body)| body.body.id == "pcb-reference")
                .map(|(index, _)| index)
                .collect::<Vec<_>>();
            if indices.len() != 1 {
                return Err("Prepared PCB reference is missing or ambiguous.".into());
            }
            let mut body = prepared.prepared.bodies[indices[0]].body.clone();
            pcb_source_thickness = board.thickness;
            if board
                .thickness
                .is_some_and(|thickness| thickness != body.thickness)
            {
                return Err(
                    "Imported PCB thickness does not match the mechanical PCB thickness.".into(),
                );
            }
            if board
                .pcb_source
                .as_ref()
                .is_none_or(|profile| profile.contours.is_empty())
                || board.contours.is_empty()
            {
                return Err("Core returned incomplete PCB source geometry.".into());
            }
            if body
                .features
                .as_ref()
                .is_some_and(|features| !features.is_empty())
                || body
                    .openings
                    .as_ref()
                    .is_some_and(|openings| !openings.is_empty())
                || body
                    .mounts
                    .as_ref()
                    .is_some_and(|mounts| !mounts.is_empty())
                || body.gasket.is_some()
                || body.pcb_source.is_some()
            {
                return Err(
                    "Imported source cannot replace a modified nominal PCB reference.".into(),
                );
            }
            body.z = Some(body.z.unwrap_or(0.0) + source.reference.elevation);
            body.pcb_source = board.pcb_source;
            body.mounts = (!board.round_drills.is_empty()).then_some(board.round_drills);
            let mut contours = board.contours;
            contours.extend(
                board
                    .holes
                    .into_iter()
                    .map(|points| boardstudio_core::model::Contour { points, hole: true }),
            );
            let source_ir = CaseIR {
                revision: prepared.identity.revision,
                body,
                contours,
            };
            let request_id = format!("{request_prefix}-pcb-prepare");
            let request = CoreRequest::PrepareCase {
                id: request_id.clone(),
                ir: CaseAssemblyIR {
                    revision: source_ir.revision,
                    bodies: vec![source_ir.clone()],
                },
            };
            let reply = core
                .request(&request_id, executor_epoch, &request)
                .await
                .map_err(|error| format!("PCB source preparation failed: {error}"))?;
            ensure_current()?;
            let mut ir = match reply {
                CoreReply::CasePrepared { id, ir } if id == request_id => ir,
                CoreReply::Error { id, message, .. } if id == request_id => return Err(message),
                _ => return Err("Core returned an unexpected source PCB preparation.".into()),
            };
            if ir.revision != prepared.identity.revision
                || ir.bodies.len() != 1
                || ir.bodies[0].revision != ir.revision
                || ir.bodies[0].body != source_ir.body
            {
                return Err("Core returned an incomplete or stale source PCB preparation.".into());
            }
            let mut pcb = ir.bodies.pop().expect("one source PCB body checked");
            // Compatibility polygons already contain these voids; exact CAD uses body mounts.
            for region in &mut pcb.regions {
                region.mounts.clear();
            }
            prepared.prepared.bodies[indices[0]] = pcb;
            if let Some(assembly) = &mut prepared.mechanical_assembly {
                assembly.pcb_reference = Some(source_ir);
                for layer in &mut assembly.stack {
                    if layer.id == "pcb" {
                        layer.z += source.reference.elevation;
                    }
                }
            }
        }
        Ok(PhysicalStepAssembly {
            prepared,
            pcb_source,
            pcb_source_thickness,
        })
    }

    async fn mechanical_package_bytes(
        self: &Rc<Self>,
        operation_id: OperationId,
        initial_snapshot: &AcceptedSnapshot,
        scope: &Scope,
    ) -> Result<Vec<u8>, String> {
        let capture = self
            .export_leases
            .borrow()
            .get(operation_id)
            .cloned()
            .ok_or_else(|| "Mechanical export owner was cancelled or superseded.".to_owned())?;
        if capture.scope() != scope
            || capture.token() != initial_snapshot.token
            || capture.snapshot().document.revision != initial_snapshot.document.revision
            || capture.snapshot().session_epoch != initial_snapshot.session_epoch
            || capture.snapshot().document.id != initial_snapshot.document.id
        {
            return Err("Mechanical export no longer matches its captured project.".into());
        }
        let core = self.core.borrow().clone();
        let runtime = self.clone();
        let ensure_current = || -> Result<(), String> {
            if runtime.export_lease_is_current(operation_id, &capture, &core) {
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
        let executor_epoch = capture.executor_epoch().0.to_string();
        let source_is_current = {
            let runtime = self.clone();
            let capture = capture.clone();
            let core = core.clone();
            Rc::new(move || runtime.export_lease_is_current(operation_id, &capture, &core))
        };
        let PhysicalStepAssembly {
            prepared,
            pcb_source,
            pcb_source_thickness,
        } = self
            .prepare_physical_step_assembly(
                core.as_ref(),
                &executor_epoch,
                &format!("mechanical-export-{}", operation_id.0),
                initial_snapshot,
                scope,
                source_is_current,
            )
            .await?;
        ensure_current()?;
        let assembly = prepared
            .mechanical_assembly
            .as_ref()
            .ok_or_else(|| "Core did not resolve a generated mechanical assembly.".to_owned())?;
        if assembly.revision != capture.snapshot().document.revision
            || assembly.case.revision != capture.snapshot().document.revision
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
            if result.revision != capture.snapshot().document.revision {
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
            "revision": capture.snapshot().document.revision,
            "pcbReference": if pcb_source.is_some() {
                "Source-backed unpopulated PCB; component solids are not included"
            } else { "Nominal unpopulated PCB only; component solids are not included" },
            "pcbSource": pcb_source.as_ref().map(|source| serde_json::json!({
                "referenceId": source.reference.id,
                "assetId": source.asset.id,
                "sha256": source.asset.sha256,
                "declaredThicknessMm": pcb_source_thickness,
            })),
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

    async fn resolve_pcb_handoff_plan(
        &self,
        operation_id: OperationId,
        snapshot: &AcceptedSnapshot,
        scope: &Scope,
        instance_id: Option<&str>,
        core: &Rc<dyn CoreExecutor>,
        capture: &ExportLease,
    ) -> Result<ElectricalPlan, String> {
        if !self.export_lease_is_current(operation_id, capture, core) {
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
            .request(
                &request_id,
                &capture.executor_epoch().0.to_string(),
                &request,
            )
            .await
            .map_err(|error| format!("Wiring resolution failed: {error}"))?;
        if !self.export_lease_is_current(operation_id, capture, core) {
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
        capture: &ExportLease,
        commit: boardstudio_application::ExportCommitRequest,
    ) -> Result<ExportLease, String> {
        if !self.export_lease_is_current(export_operation_id, capture, &self.core.borrow().clone())
        {
            return Err("KiCad export no longer owns the accepted wiring source.".into());
        }
        let operation_id = self.operation();
        let outcome = self.observe_operation(operation_id);
        self.submit(Event::ExportCommit {
            operation_id,
            export_operation_id,
            token: capture.token(),
            scope: capture.scope().clone(),
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
                updated.advance(accepted.clone())?;
                if accepted.session_epoch != updated.snapshot().session_epoch
                    || accepted.document.id != updated.snapshot().document.id
                    || accepted.scene.revision != updated.snapshot().document.revision
                    || !self.session.borrow().export_is_current(
                        export_operation_id,
                        updated.token(),
                        updated.scope(),
                    )
                {
                    return Err("Export wiring changed the captured project or board.".into());
                }
                self.export_leases
                    .borrow_mut()
                    .begin(export_operation_id, updated.clone());
                return Ok(updated);
            }
            TimeoutFuture::new(25).await;
        }
        Err("Export wiring save did not complete.".into())
    }

    fn require_pcb_handoff_current(
        &self,
        operation_id: OperationId,
        capture: &ExportLease,
        core: &Rc<dyn CoreExecutor>,
    ) -> Result<(), String> {
        if self.export_lease_is_current(operation_id, capture, core) {
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
            .export_leases
            .borrow()
            .get(operation_id)
            .cloned()
            .ok_or_else(|| "Footprint export owner was cancelled or superseded.".to_owned())?;
        if capture.scope() != scope
            || capture.token() != snapshot.token
            || capture.snapshot().document.revision != snapshot.document.revision
            || capture.snapshot().session_epoch != snapshot.session_epoch
            || capture.snapshot().document.id != snapshot.document.id
        {
            return Err("Footprint export no longer matches its captured project.".into());
        }
        let core = self.core.borrow().clone();
        let runtime = self.clone();
        let ensure_current = || {
            if runtime.export_lease_is_current(operation_id, &capture, &core) {
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
                executor_epoch: capture.executor_epoch().0,
            },
            ensure_current,
        )
        .await
    }

    pub fn export_project_copy(self: &Rc<Self>) {
        let Some(snapshot) = self.model().accepted else {
            return;
        };
        let Some(scope) = self.scope() else {
            return;
        };
        let operation_id = self.operation();
        self.archive_export_options.begin_archive(operation_id);
        self.begin_export_lease(operation_id, ExportKind::ProjectCopy, &snapshot, &scope);
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
        let Some(_board) = snapshot
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
        let Some(_contours) = snapshot
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
        let operation_id = self.operation();
        self.begin_export_lease(
            operation_id,
            ExportKind::BoardOutline {
                dxf: matches!(format, boardstudio_core::model::OutlineExportFormat::Dxf),
            },
            &snapshot,
            &scope,
        );
        self.submit(Event::StartExport {
            operation_id,
            scope,
        });
    }

    async fn board_outline_bytes(
        &self,
        operation_id: OperationId,
        snapshot: &AcceptedSnapshot,
        scope: &Scope,
        dxf: bool,
    ) -> Result<(Vec<u8>, SnapshotToken), String> {
        let lease = self
            .export_leases
            .borrow()
            .get(operation_id)
            .cloned()
            .ok_or_else(|| "Outline export owner was cancelled or superseded.".to_owned())?;
        if !matches!(lease.kind(), ExportKind::BoardOutline { dxf: captured } if *captured == dxf)
            || lease.scope() != scope
            || lease.token() != snapshot.token
        {
            return Err("Outline export no longer matches its captured board.".into());
        }
        let board = snapshot
            .document
            .boards
            .iter()
            .find(|board| board.id == scope.board_id)
            .cloned()
            .ok_or_else(|| "Select a resolved board before outline export.".to_owned())?;
        let contours = snapshot
            .scene
            .board_contours
            .iter()
            .find(|entry| entry.board_id == scope.board_id)
            .map(|entry| entry.contours.clone())
            .ok_or_else(|| "The selected board has no resolved outline.".to_owned())?;
        let format = if dxf {
            boardstudio_core::model::OutlineExportFormat::Dxf
        } else {
            boardstudio_core::model::OutlineExportFormat::Svg
        };
        let format_name = if dxf { "dxf" } else { "svg" };
        let request_id = format!("export-outline-{}", operation_id.0);
        let request = ArtifactRequest::ExportOutline {
            id: request_id.clone(),
            request: boardstudio_core::model::OutlineExportRequest {
                filename: format!("{}.{}", snapshot.document.name, format_name),
                board,
                contours,
                format,
            },
        };
        let core = self.core.borrow().clone();
        if !self.export_lease_is_current(operation_id, &lease, &core) {
            return Err("Outline export was cancelled, superseded, or its source changed.".into());
        }
        let reply = core
            .artifact(&request_id, &lease.executor_epoch().0.to_string(), &request)
            .await
            .map_err(|error| format!("Outline export failed: {error}"))?;
        if !self.export_lease_is_current(operation_id, &lease, &core) {
            return Err("Outline export was cancelled, superseded, or its source changed.".into());
        }
        let file = match reply {
            ArtifactReply::ExportOutline { id, result } if id == request_id => result,
            ArtifactReply::Error { id, error } if id == request_id => {
                return Err(format!("Outline export failed: {}", error.message));
            }
            _ => return Err("Core returned an outline for another export request.".into()),
        };
        Ok((file.content.into_bytes(), snapshot.token))
    }

    fn export_lease_is_current(
        &self,
        operation_id: OperationId,
        lease: &ExportLease,
        worker: &Rc<dyn CoreExecutor>,
    ) -> bool {
        let core = self.core.borrow().clone();
        self.export_leases.borrow().get(operation_id) == Some(lease)
            && self.export_current(operation_id, lease.token(), lease.scope())
            && Rc::ptr_eq(worker, &core)
    }

    fn export_current(
        &self,
        operation_id: OperationId,
        token: SnapshotToken,
        scope: &Scope,
    ) -> bool {
        let core = self.core.borrow().clone();
        let session = self.session.borrow();
        self.export_leases
            .borrow()
            .get(operation_id)
            .is_some_and(|lease| {
                let session_owns_export = session.export_is_current(operation_id, token, scope)
                    && session.scope().as_ref() == Some(scope);
                lease.token() == token
                    && lease.scope() == scope
                    && lease
                        .require_current(
                            session_owns_export,
                            session.read_model().accepted.as_ref(),
                            session.core_executor_epoch(),
                            core_executor_identity(&core),
                        )
                        .is_ok()
            })
    }

    fn begin_export_lease(
        &self,
        operation_id: OperationId,
        kind: ExportKind,
        snapshot: &AcceptedSnapshot,
        scope: &Scope,
    ) {
        let core = self.core.borrow().clone();
        let executor_epoch = self.session.borrow().core_executor_epoch();
        self.export_leases.borrow_mut().begin(
            operation_id,
            ExportLease::new(
                kind,
                snapshot.clone(),
                scope.clone(),
                executor_epoch,
                core_executor_identity(&core),
            ),
        );
    }

    fn export_source_is_current(&self, lease: &ExportLease) -> bool {
        let model = self.model();
        let core = self.core.borrow().clone();
        let session = self.session.borrow();
        lease.source_is_current(
            session.scope().as_ref(),
            model.accepted.as_ref(),
            session.core_executor_epoch(),
            core_executor_identity(&core),
        )
    }
    fn export_delivery_is_current(
        &self,
        operation_id: OperationId,
        token: SnapshotToken,
        scope: &Scope,
    ) -> bool {
        self.export_leases
            .borrow()
            .get(operation_id)
            .cloned()
            .is_some_and(|lease| {
                lease.token() == token
                    && lease.scope() == scope
                    && self.export_source_is_current(&lease)
            })
    }
    fn snapshot_current(&self, token: SnapshotToken, scope: &Scope) -> bool {
        self.scope().as_ref() == Some(scope)
            && self
                .model()
                .accepted
                .as_ref()
                .is_some_and(|s| s.token == token)
    }
    fn current_firmware_executor(
        &self,
    ) -> (Rc<dyn CoreExecutor>, boardstudio_application::ExecutorEpoch) {
        let core = self.core.borrow().clone();
        let executor: Rc<dyn CoreExecutor> = core;
        (executor, self.session.borrow().core_executor_epoch())
    }
    fn deliver_artifact(&self, artifact: &Artifact) -> Result<(), String> {
        #[cfg(any(test, feature = "test-support"))]
        if let Some(adapters) = self.in_process_adapters.borrow().as_ref() {
            adapters.deliver(artifact);
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
                source_sha256: None,
                step_import_options: None,
                collision_query: None,
                generated_analysis_request: None,
                collision_meshes: None,
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
                source_sha256: None,
                step_import_options: None,
                collision_query: None,
                generated_analysis_request: None,
                collision_meshes: None,
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

    async fn keycaps_stl_bytes(
        &self,
        operation_id: OperationId,
        snapshot: &AcceptedSnapshot,
        scope: &Scope,
    ) -> Result<Vec<u8>, String> {
        let core = self.core.borrow().clone();
        let guard = || self.require_export_current(operation_id, snapshot, scope, Some(&core));
        guard()?;
        let resolution = self
            .resolve_case_keycaps_preview(scope.clone(), snapshot.token, snapshot.document.revision)
            .await?;
        guard()?;
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
            return Err("Choose a keycap profile before exporting keycap STLs.".into());
        }
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
        let result = async {
            worker.ready().await.map_err(|error| error.to_string())?;
            guard()?;
            let result = worker
                .request_keycaps_print(
                    format!("keycaps-stl-{}", operation_id.0),
                    format!("keycaps-print-{}", operation_id.0),
                    identity,
                    resolution.specs,
                )
                .await
                .map_err(|error| error.to_string())?;
            guard()?;
            crate::keycap_package::files(result)
        }
        .await;
        worker.close();
        self.export_workers.borrow_mut().remove(&operation_id);
        let files = result?;
        let entries = files
            .iter()
            .enumerate()
            .map(|(index, (path, _))| {
                Ok(ArchiveEntry {
                    path: path.clone(),
                    buffer_index: u32::try_from(index)
                        .map_err(|_| "Too many keycap STL files.".to_owned())?,
                })
            })
            .collect::<Result<Vec<_>, String>>()?;
        let metadata = serde_json::to_string(&ArchiveRequest::PackFiles { entries })
            .map_err(|error| error.to_string())?;
        let buffers = files
            .iter()
            .map(|(_, bytes)| Uint8Array::from(bytes.as_slice()))
            .collect();
        guard()?;
        let epoch = self.session.borrow().core_executor_epoch().0.to_string();
        let packed = core
            .archive(
                &format!("keycaps-stl-archive-{}", operation_id.0),
                &epoch,
                &metadata,
                buffers,
            )
            .await
            .map_err(|error| format!("Keycap STL packaging failed: {error}"))?;
        guard()?;
        match serde_json::from_str::<ArchiveReply>(&packed.metadata)
            .map_err(|error| error.to_string())?
        {
            ArchiveReply::Packed if packed.buffers.len() == 1 => Ok(packed.buffers[0].to_vec()),
            _ => Err("Core returned an incomplete keycap STL archive.".into()),
        }
    }

    async fn firmware_bytes(
        &self,
        operation_id: OperationId,
        snapshot: &AcceptedSnapshot,
        scope: &Scope,
    ) -> Result<Vec<u8>, String> {
        let document = snapshot.document.as_ref();
        self.require_export_current(operation_id, snapshot, scope, None)?;
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
        self.require_export_current(operation_id, snapshot, scope, Some(&core))?;
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
        self.require_export_current(operation_id, snapshot, scope, Some(&core))?;
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
        self.require_export_current(operation_id, snapshot, scope, Some(&core))?;
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
        self.require_export_current(operation_id, snapshot, scope, Some(&core))?;
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
        self.require_export_current(operation_id, snapshot, scope, Some(&core))?;
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
        self.require_export_current(operation_id, snapshot, scope, Some(core))?;
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
        self.require_export_current(operation_id, snapshot, scope, Some(core))?;
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

    fn require_export_current(
        &self,
        operation_id: OperationId,
        snapshot: &AcceptedSnapshot,
        scope: &Scope,
        worker: Option<&Rc<dyn CoreExecutor>>,
    ) -> Result<(), String> {
        let lease = self
            .export_leases
            .borrow()
            .get(operation_id)
            .cloned()
            .ok_or_else(|| "Export lease is no longer current.".to_owned())?;
        if lease.token() != snapshot.token
            || lease.scope() != scope
            || lease.snapshot().document.revision != snapshot.document.revision
        {
            return Err("Export lease is no longer current.".into());
        }
        let core = self.core.borrow().clone();
        let session = self.session.borrow();
        let session_owns_export = session.export_is_current(operation_id, lease.token(), scope)
            && session.scope().as_ref() == Some(scope);
        lease.require_current(
            session_owns_export,
            session.read_model().accepted.as_ref(),
            session.core_executor_epoch(),
            core_executor_identity(&core),
        )?;
        if worker.is_some_and(|worker| !Rc::ptr_eq(worker, &core)) {
            return Err("Export lease is no longer current.".into());
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
    fn begin_open(&self) -> Result<u64, String> {
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
        if file.size() > boardstudio_core::archive::MAX_PROJECT_ARCHIVE_BYTES as f64 {
            self.report("Project archive exceeds the 128 MiB size limit.");
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
                this.report(error);
            }
            #[cfg(any(test, feature = "test-support"))]
            if let Some(done) = open_observer {
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

    fn remember_saved(&self, saved: SavedProject) -> ProjectDoc {
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
                this.report(format!("Raw saved data could not be downloaded: {error}"));
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
                this.report(format!("Pending edits could not be downloaded: {error}"));
            }
        });
    }

    async fn pending_archive(&self) -> Result<(Arc<ProjectDoc>, Vec<u8>), String> {
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

    fn confirm_discard_pending(&self) -> bool {
        self.pending_document.borrow().is_none() || web_sys::window().and_then(|window| window.confirm_with_message(
            "Discard your pending edits and reload the saved keyboard? Download your pending edits first if you want to keep them."
        ).ok()).unwrap_or(false)
    }

    fn confirm_project_replacement(&self, document: &ProjectDoc) -> Result<(), String> {
        boardstudio_core::CoreEngine::validate_open(document)?;
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
            let core = this.core.borrow().clone();
            if let Err(error) = core.ready().await {
                this.report(format!("Recovery executor unavailable: {error}"));
                return;
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
        self.pack_document(snapshot.document.as_ref(), embed_used_models, guard)
            .await
    }

    async fn pack_document(
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
        let mut document = in_process_support::board_document();
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
        let accepted = runtime
            .model()
            .accepted
            .expect("the Case project is accepted");
        let scope = runtime
            .scope()
            .expect("the accepted Case project has a scope");
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

#[path = "collision_check.rs"]
mod collision_check;

pub use collision_check::{CollisionCheckInput, CollisionCheckReport};

#[cfg(any(test, feature = "test-support"))]
#[path = "in_process_support.rs"]
pub mod in_process_support;

#[cfg(all(test, target_arch = "wasm32"))]
#[path = "runtime_collision_browser_tests.rs"]
mod collision_browser_tests;

#[cfg(test)]
#[path = "source_pcb_export_tests.rs"]
mod source_pcb_export_tests;

#[cfg(all(test, target_arch = "wasm32"))]
#[path = "cad_runtime_borrow_tests.rs"]
mod cad_runtime_borrow_tests;

#[cfg(any(test, feature = "test-support"))]
pub mod firmware_export_test_support {
    use super::*;
    use crate::runtime::in_process_support::{DeliveredArtifact, InProcessCore};
    use crate::runtime::project_name_test_support as support;
    use boardstudio_application::Scope;
    use boardstudio_core::{firmware::FirmwarePackage, model::*};
    use futures_channel::oneshot;

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum Stage {
        Resolution,
        Generation,
        Packaging,
    }

    fn resolution(request: &CoreRequest) -> bool {
        matches!(request, CoreRequest::ResolveElectrical { .. })
    }
    fn generation(request: &CoreRequest) -> bool {
        matches!(request, CoreRequest::GenerateFirmware { .. })
    }
    fn generated(request: &CoreRequest) -> Option<CoreReply> {
        let CoreRequest::GenerateFirmware { id, .. } = request else {
            return None;
        };
        Some(CoreReply::FirmwareGenerated {
            id: id.clone(),
            package: FirmwarePackage {
                files: BTreeMap::from([(
                    "config/boards/test.keymap".into(),
                    "// generated test keymap".into(),
                )]),
                warnings: Vec::new(),
            },
        })
    }

    pub async fn fixture() -> (Rc<Runtime>, AcceptedSnapshot, Scope) {
        let runtime = support::new_runtime();
        support::open_document(&runtime, wired_document()).await;
        support::in_process_core(&runtime).script_replies(generated);
        let accepted = runtime.model().accepted.unwrap();
        let scope = runtime.scope().unwrap();
        (runtime, accepted, scope)
    }

    pub fn fail(runtime: &Runtime, stage: Stage) {
        let core = support::in_process_core(runtime);
        match stage {
            Stage::Resolution => {
                core.fail_matching_reply(resolution, "injected electrical-resolution failure")
            }
            Stage::Generation => {
                core.fail_matching_reply(generation, "injected firmware-generation failure")
            }
            Stage::Packaging => core.fail_next_archive("injected ZIP-packaging failure"),
        }
    }

    pub fn gate(runtime: &Runtime, stage: Stage) -> (oneshot::Receiver<()>, oneshot::Sender<()>) {
        let core = support::in_process_core(runtime);
        match stage {
            Stage::Resolution => core.gate_matching_reply(resolution),
            Stage::Generation => core.gate_matching_reply(generation),
            Stage::Packaging => core.gate_next_archive(),
        }
    }

    pub async fn restart_executor(runtime: &Rc<Runtime>) {
        let old: Rc<InProcessCore> = support::in_process_core(runtime);
        old.fail_next_reply("controlled Session executor failure");
        let document = (*runtime.model().accepted.unwrap().document).clone();
        support::replace_document(runtime, "restart-fixture", &document.id.clone(), document).await;
        assert!(old.is_closed());
        assert!(!Rc::ptr_eq(&old, &support::in_process_core(runtime)));
        support::in_process_core(runtime).script_replies(generated);
    }

    pub async fn run_effects(runtime: &Rc<Runtime>, initial: Vec<Effect>) {
        let mut effects = VecDeque::from(initial);
        while let Some(effect) = effects.pop_front() {
            effects.extend(support::run_effect(runtime, effect).await);
        }
    }

    pub fn start_export(runtime: &Rc<Runtime>) -> Vec<Effect> {
        runtime.export_firmware();
        support::take_held_effects(runtime)
    }
    pub fn take_effects(runtime: &Runtime) -> Vec<Effect> {
        support::take_held_effects(runtime)
    }
    pub fn take_deliveries(runtime: &Runtime) -> Vec<DeliveredArtifact> {
        runtime
            .in_process_adapters
            .borrow()
            .as_ref()
            .unwrap()
            .take_deliveries()
    }
    pub fn session_model(runtime: &Runtime) -> ReadModel {
        runtime.model()
    }
    pub fn has_artifacts(runtime: &Runtime) -> bool {
        !runtime.artifacts.borrow().is_empty()
    }

    fn definition(
        id: &str,
        kind: PartKind,
        terminals: BTreeMap<String, Vec<String>>,
        source: Option<&str>,
    ) -> PartDefinition {
        let pads = terminals
            .values()
            .flatten()
            .map(|id| Pad {
                id: id.clone(),
                number: id.clone(),
                at: Vec2::default(),
                size: Vec2 { x: 1.0, y: 1.0 },
                shape: PadShape::Circle,
                drill: None,
                plated: Some(true),
                side: None,
                rotation: None,
                net_id: None,
            })
            .collect();
        PartDefinition {
            hardware_profile: None,
            input_profile: None,
            id: id.into(),
            name: id.into(),
            kind,
            keycap: None,
            envelope_source: None,
            kicad_source: None,
            terminals,
            matrix_terminals: Some(MatrixTerminals {
                row: "row".into(),
                column: "column".into(),
            }),
            envelope_notice: None,
            courtyard: vec![],
            pads,
            models: None,
            generator: source.map(|s| PartGenerator {
                source: s.into(),
                version: "test".into(),
                parameters: BTreeMap::new(),
            }),
            mechanical_profile: None,
        }
    }

    fn part(id: &str, definition_id: &str) -> Part {
        Part {
            id: id.into(),
            definition_id: definition_id.into(),
            reference: id.into(),
            pose: Pose2 {
                at: Vec2::default(),
                rotation: 0.0,
            },
            side: Side::Front,
            anchor: None,
            locked: None,
            keycap: None,
            outline: None,
            properties: None,
            generator_parameters: None,
        }
    }

    fn wired_document() -> ProjectDoc {
        let mut doc = ProjectDoc::empty("zmk-export-test", "ZMK export test");
        let mut sw_terms = BTreeMap::new();
        sw_terms.insert("row".into(), vec!["1".into()]);
        sw_terms.insert("column".into(), vec!["2".into()]);
        doc.definitions
            .push(definition("switch", PartKind::Switch, sw_terms, None));
        let mut diode_terms = BTreeMap::new();
        diode_terms.insert("anode".into(), vec!["A".into()]);
        diode_terms.insert("cathode".into(), vec!["K".into()]);
        doc.definitions
            .push(definition("diode", PartKind::Passive, diode_terms, None));
        let mut mcu_terms = BTreeMap::new();
        for p in [
            "GND", "P1", "P2", "P3", "P4", "P5", "P6", "P7", "P8", "P9", "P10", "P14", "P15",
            "P16", "P18", "P19", "P20", "P21",
        ] {
            mcu_terms.insert(p.into(), vec![p.into()]);
        }
        doc.definitions.push(definition(
            "mcu",
            PartKind::Controller,
            mcu_terms,
            Some("ceoloide/mcu_nice_nano"),
        ));
        let mut matrix_parts = vec![];
        for r in 0..2 {
            for c in 0..2 {
                let id = format!("matrix/m/r{r}c{c}");
                matrix_parts.push(id.clone());
                doc.parts.push(part(&id, "switch"));
                let did = format!("{id}/diode");
                matrix_parts.push(did.clone());
                doc.parts.push(part(&did, "diode"));
            }
        }
        doc.parts.push(part("mcu-left", "mcu"));
        doc.boards.push(Board {
            id: "board-a".into(),
            name: "A".into(),
            outline_ids: vec![],
            part_ids: doc.parts.iter().map(|p| p.id.clone()).collect(),
            net_ids: vec![],
            thickness: 1.6,
            traces: vec![],
            vias: vec![],
        });
        doc.matrices.push(Matrix {
            id: "m".into(),
            name: None,
            rows: 2,
            columns: 2,
            pitch: Vec2 { x: 19.0, y: 19.0 },
            origin: Vec2::default(),
            definition_id: "switch".into(),
            part_ids: matrix_parts,
            board_id: Some("board-a".into()),
            mirror: None,
            rotation: None,
            edge_gap: None,

            diode_direction: Some(DiodeDirection::Row2col),
            row_layout: vec![],
            column_layout: vec![],
            anchor: None,
            cells: (0..2)
                .flat_map(|r| {
                    (0..2).map(move |c| MatrixCell {
                        shift: None,
                        bend: None,
                        deleted: false,
                        row: r,
                        column: c,
                        enabled: true,

                        definition_id: Some("switch".into()),
                        variant: None,
                        offset: None,
                        rotation: None,
                        assemblies: vec![],
                        assemblies_local: None,
                    })
                })
                .collect(),
        });
        doc.hardware = Some(HardwareConfiguration {
            topology: HardwareTopology::Unibody,
            transport: HardwareTransport::None,
            instances: Vec::new(),
            shared_construction: None,
            pairs: Vec::new(),
            boards: vec![ElectricalBoardConfiguration {
                board_id: "board-a".into(),
                controller_part_id: Some("mcu-left".into()),
                ..Default::default()
            }],
        });
        doc
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
        source_sha256: None,
        step_import_options: None,
        collision_query: None,
        generated_analysis_request: None,
        collision_meshes: None,
        input_bytes: Vec::new(),
    };
    let reply = worker
        .request(request.clone())
        .await
        .map_err(|error| format!("CAD mechanical geometry failed: {error}"))?;
    validate_reply(&request, reply, identity)
        .map_err(|error| format!("CAD mechanical geometry failed: {error:?}"))
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
pub mod project_name_test_support {
    use super::*;
    use crate::runtime::in_process_support::InProcessAdapters;
    use boardstudio_application::{EditResolver, Resolution};

    pub fn new_runtime() -> Rc<Runtime> {
        Runtime::new_with_restoration(false).expect("browser runtime fixture initializes")
    }

    pub fn set_storage_initializing(runtime: &Runtime, initializing: bool) {
        runtime.storage_initializing.set(initializing);
        runtime.changed();
    }

    pub fn fail_storage_initialization(runtime: &Runtime, error: &str) {
        runtime.storage_initializing.set(true);
        runtime.storage_initialization_failed(error);
    }

    /// Replace the runtime's session and route Core execution through an in-process
    /// `CoreEngine`. Saves keep running through the production browser-store port (gated by
    /// `gate_next_persist` / `fail_next_persist`); call `install_memory_persistence` to save
    /// into memory instead.
    pub fn install(runtime: &Runtime, session: Session, core: boardstudio_core::CoreEngine) {
        runtime.saved_tokens.borrow_mut().clear();
        if let Some(accepted) = session.read_model().accepted.as_ref()
            && let Some(token) = runtime.store.observed_token(&accepted.document.id)
        {
            runtime
                .saved_tokens
                .borrow_mut()
                .insert(accepted.document.id.clone(), token);
        }
        *runtime.session.borrow_mut() = session;
        let adapters = Rc::new(InProcessAdapters::new(core, runtime.store.clone()));
        adapters.attach(runtime);
        *runtime.in_process_adapters.borrow_mut() = Some(adapters);
        runtime.changed();
    }

    /// Route saves into the in-process memory store. Requires `install` to have run.
    pub fn install_memory_persistence(runtime: &Runtime) {
        installed(runtime).use_memory_saves();
        runtime.saved_tokens.borrow_mut().clear();
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
        submit_fixed_command(
            runtime,
            boardstudio_core::model::EditCommand {
                base_revision,
                transaction_id: transaction_id.to_owned(),
                phase: boardstudio_core::model::EditPhase::Commit,
                target_ids: vec![target_id.to_owned()],
                operation: boardstudio_core::model::EditOperation::ReplaceDocument {
                    document: Box::new(document),
                },
            },
        );
        run_pending(runtime).await;
        runtime
            .model()
            .accepted
            .expect("the edited document stays accepted")
    }

    /// Submit a fixed command through the same resolver route as a production commit.
    pub fn submit_fixed_command(
        runtime: &Rc<Runtime>,
        command: boardstudio_core::model::EditCommand,
    ) {
        runtime.submit(fixed_command_event(runtime.operation(), command));
    }

    /// Build a resolver event for tests that need to submit through their own observer.
    pub fn fixed_command_event(
        operation_id: OperationId,
        command: boardstudio_core::model::EditCommand,
    ) -> Event {
        let boardstudio_core::model::EditCommand {
            transaction_id,
            target_ids,
            operation,
            ..
        } = command;
        Event::ResolveEdit {
            operation_id,
            label: "test fixed command".into(),
            resolver: EditResolver::new("test fixed command", move |_| {
                if transaction_id.is_empty() {
                    Resolution::submit(target_ids.clone(), operation.clone())
                } else {
                    Resolution::submit_with_transaction_id(
                        target_ids.clone(),
                        transaction_id.clone(),
                        operation.clone(),
                    )
                }
            }),
        }
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
    /// export jobs are outside the adapter, so tests read these to see what Session started
    /// (an empty list means nothing was submitted). Taking them also keeps a later
    /// `run_pending` from starting the job. Effects are only held once the adapter is
    /// installed, so a runtime without it is a test mistake, not an empty answer.
    pub fn take_held_effects(runtime: &Runtime) -> Vec<Effect> {
        installed(runtime);
        std::mem::take(&mut *runtime.held_effects.borrow_mut())
    }

    /// Deliver a completion to Session as the executor that ran the effect would have. Tests
    /// use it for the generation jobs the adapter does not run; the effects it produces are
    /// held like those of a submitted event.
    pub fn deliver_completion(runtime: &Rc<Runtime>, completion: Completion) {
        installed(runtime);
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
        let core = installed(runtime).current_core();
        let reply = ArchiveReply::Unpacked {
            project_json: serde_json::to_string(document).expect("fixture document serializes"),
            assets: Vec::new(),
        };
        core.reply_to_next_archive(boardstudio_web_host::host::ArchiveResult {
            metadata: serde_json::to_string(&reply).expect("archive reply serializes"),
            buffers: Vec::new(),
        });
        core.gate_next_archive()
    }

    pub fn gate_open_saved_result(
        runtime: &Runtime,
    ) -> (
        futures_channel::oneshot::Receiver<()>,
        futures_channel::oneshot::Sender<()>,
    ) {
        installed(runtime).gate_next_store_read()
    }

    pub fn track_import_file(runtime: &Runtime) -> futures_channel::oneshot::Receiver<()> {
        installed(runtime).observe_next_open()
    }

    pub fn track_open_saved(runtime: &Runtime) -> futures_channel::oneshot::Receiver<()> {
        installed(runtime).observe_next_open()
    }

    pub fn track_next_persist_target(runtime: &Runtime) -> futures_channel::oneshot::Receiver<()> {
        installed(runtime).persistence().observe_next_target()
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

    pub fn gate_next_core_reply_then_fail(
        runtime: &Runtime,
        _reason: impl Into<String>,
    ) -> (
        futures_channel::oneshot::Receiver<()>,
        futures_channel::oneshot::Sender<()>,
    ) {
        installed(runtime)
            .current_core()
            .gate_next_reply_then_fail(_reason)
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

    /// Drive one real effect, returning its continuations so a test can pause between them.
    pub async fn run_effect(runtime: &Rc<Runtime>, effect: Effect) -> Vec<Effect> {
        runtime.run(effect).await
    }

    pub async fn run_pending(runtime: &Rc<Runtime>) {
        loop {
            let pending = std::mem::take(&mut *runtime.held_effects.borrow_mut());
            if !pending.is_empty() {
                // Keep Runtime effects off the mounted test's WASM poll stack too.
                runtime.drive(pending);
            }
            let drives = std::mem::take(&mut *runtime.in_process_drives.borrow_mut());
            if drives.is_empty() {
                break;
            }
            for finished in drives {
                finished
                    .await
                    .expect("in-process effect driver was dropped");
            }
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
    use futures_channel::oneshot;
    use std::task::Poll;
    use wasm_bindgen_test::wasm_bindgen_test;

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

    async fn wait_for_actual_stage(
        run: &mut Pin<Box<dyn Future<Output = Vec<Effect>> + '_>>,
        entered: oneshot::Receiver<()>,
    ) {
        let mut entered = Box::pin(entered);
        std::future::poll_fn(|context| {
            if let Poll::Ready(effects) = run.as_mut().poll(context) {
                panic!("production RunExport completed before the gated stage: {effects:?}");
            }
            match entered.as_mut().poll(context) {
                Poll::Ready(Ok(())) => Poll::Ready(()),
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
            let (runtime, accepted, scope) = test_support::fixture().await;
            test_support::fail(&runtime, stage);
            let before = runtime.session.borrow().read_model().clone();
            let (operation_id, run_effect) =
                run_export_effect(test_support::start_export(&runtime));
            assert!(
                matches!(&run_effect, Effect::RunExport { scope: emitted_scope, .. } if emitted_scope == &scope)
            );

            test_support::run_effects(&runtime, vec![run_effect]).await;

            assert_eq!(runtime.session.borrow().read_model(), &before);
            assert_eq!(runtime.model().accepted.as_ref(), Some(&accepted));
            assert_eq!(runtime.scope().as_ref(), Some(&scope));
            assert_eq!(runtime.status(), expected);
            assert!(runtime.status_is_alert());
            assert!(runtime.artifacts.borrow().is_empty());
            assert!(test_support::take_deliveries(&runtime).is_empty());

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
        for stage in [
            test_support::Stage::Resolution,
            test_support::Stage::Generation,
            test_support::Stage::Packaging,
        ] {
            let (runtime, _accepted, _scope) = test_support::fixture().await;
            let (entered, release) = test_support::gate(&runtime, stage);
            let before = runtime.session.borrow().read_model().clone();
            let (_operation, effect) = run_export_effect(test_support::start_export(&runtime));
            let mut run: Pin<Box<dyn Future<Output = Vec<Effect>> + '_>> =
                Box::pin(runtime.run(effect));
            wait_for_actual_stage(&mut run, entered).await;
            let previous_epoch = runtime.electrical_preview_executor_epoch();
            test_support::restart_executor(&runtime).await;
            assert!(runtime.electrical_preview_executor_epoch() > previous_epoch);
            let restart_status = runtime.status();
            let after_restart = runtime.model();
            assert_eq!(after_restart.accepted, before.accepted);
            assert_eq!(after_restart.lifecycle, Lifecycle::RecoveryRequired);
            release
                .send(())
                .expect("release suspended worker operation");
            let effects = run.await;
            test_support::run_effects(&runtime, effects).await;

            assert_eq!(runtime.session.borrow().read_model(), &after_restart);
            assert!(runtime.artifacts.borrow().is_empty());
            assert!(test_support::take_deliveries(&runtime).is_empty());
            assert_eq!(
                runtime.status(),
                restart_status,
                "obsolete worker failure stays silent"
            );
        }
    }

    #[wasm_bindgen_test]
    async fn production_completion_after_owner_switch_does_not_deliver_or_report() {
        let (runtime, accepted, _scope) = test_support::fixture().await;
        let (entered, release) = test_support::gate(&runtime, test_support::Stage::Generation);
        let (_, effect) = run_export_effect(test_support::start_export(&runtime));
        let mut run: Pin<Box<dyn Future<Output = Vec<Effect>> + '_>> =
            Box::pin(runtime.run(effect));
        wait_for_actual_stage(&mut run, entered).await;

        let mut next_document = (*accepted.document).clone();
        next_document.id = "next-project".into();
        runtime.submit(Event::Open {
            operation_id: runtime.operation(),
            document: next_document,
        });
        project_name_test_support::run_pending(&runtime).await;
        let next_accepted = runtime.model().accepted.unwrap();
        let next_scope = runtime.scope().unwrap();
        release.send(()).expect("release old project generation");
        test_support::run_effects(&runtime, run.await).await;

        assert_eq!(runtime.model().accepted.as_ref(), Some(&next_accepted));
        assert_eq!(runtime.scope().as_ref(), Some(&next_scope));
        assert!(runtime.artifacts.borrow().is_empty());
        assert!(test_support::take_deliveries(&runtime).is_empty());
        assert!(
            !runtime.status_is_alert(),
            "an old project failure must not be announced in the new project"
        );
    }

    #[wasm_bindgen_test]
    async fn production_older_export_can_deliver_without_replacing_newer_report() {
        let (runtime, _accepted, _scope) = test_support::fixture().await;
        let (entered, release) = test_support::gate(&runtime, test_support::Stage::Generation);
        let (_, old_effect) = run_export_effect(test_support::start_export(&runtime));
        let mut old_run: Pin<Box<dyn Future<Output = Vec<Effect>> + '_>> =
            Box::pin(runtime.run(old_effect));
        wait_for_actual_stage(&mut old_run, entered).await;

        let (new_operation, new_effect) = run_export_effect(test_support::start_export(&runtime));
        assert!(new_operation.0 > 0);
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
}

#[cfg(all(test, target_arch = "wasm32"))]
mod cad_scene_rebind_tests {
    use super::*;
    use crate::case_generation_lifecycle::physical_case_fingerprint;
    use wasm_bindgen_test::wasm_bindgen_test;

    async fn runtime_with_scene(exact: bool) -> (Rc<Runtime>, AcceptedSnapshot, Scope) {
        let runtime = project_name_test_support::new_runtime();
        let mut document = in_process_support::board_document();
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
        let accepted = runtime
            .model()
            .accepted
            .expect("the Case project is accepted");
        let scope = runtime
            .scope()
            .expect("the accepted Case project has a scope");
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

    struct ConfirmChoice {
        original: JsValue,
        calls: Rc<Cell<u32>>,
        answer: Rc<Cell<bool>>,
        _callback: Closure<dyn FnMut(JsValue) -> bool>,
    }

    impl ConfirmChoice {
        fn new() -> Self {
            let window = web_sys::window().unwrap();
            let original = Reflect::get(&window, &"confirm".into()).unwrap();
            let calls = Rc::new(Cell::new(0));
            let answer = Rc::new(Cell::new(false));
            let count = calls.clone();
            let result = answer.clone();
            let callback = Closure::<dyn FnMut(JsValue) -> bool>::new(move |_| {
                count.set(count.get() + 1);
                result.get()
            });
            Reflect::set(&window, &"confirm".into(), callback.as_ref()).unwrap();
            Self {
                original,
                calls,
                answer,
                _callback: callback,
            }
        }
    }

    impl Drop for ConfirmChoice {
        fn drop(&mut self) {
            Reflect::set(
                &web_sys::window().unwrap(),
                &"confirm".into(),
                &self.original,
            )
            .unwrap();
        }
    }

    #[wasm_bindgen_test]
    async fn unknown_outcome_recovery_still_confirms_discard_before_new() {
        let runtime = test_support::new_runtime();
        test_support::install(&runtime, Session::new(), CoreEngine::new());
        let current = ProjectDoc::empty("unknown-current", "Current");
        runtime.submit(Event::Open {
            operation_id: runtime.operation(),
            document: current.clone(),
        });
        test_support::run_pending(&runtime).await;
        assert_eq!(*runtime.model().accepted.unwrap().document, current);
        test_support::fail_next_core_reply(&runtime, "unknown worker outcome");
        let mut edited = current.clone();
        edited.name = "Unknown edit".into();
        test_support::replace_document(&runtime, "unknown-before-new", &current.id, edited).await;
        assert_eq!(runtime.model().lifecycle, Lifecycle::RecoveryRequired);
        assert!(!runtime.has_pending_edits());
        let before = runtime.model();
        let confirm = ConfirmChoice::new();
        assert!(
            runtime.create_new_keyboard().is_err(),
            "an unknown outcome does not authorize discarding local drafts"
        );
        assert_eq!(confirm.calls.get(), 1);
        assert_eq!(runtime.model(), before);
        assert_eq!(
            runtime.store.list_projects().await.unwrap().documents,
            vec![current.clone()]
        );
        assert_eq!(
            runtime
                .store
                .load_document(current.id.clone())
                .await
                .unwrap(),
            Some(current)
        );
    }

    #[wasm_bindgen_test]
    async fn new_project_cancel_keeps_drafts_and_publication_waits_for_persistence() {
        let runtime = test_support::new_runtime();
        test_support::install(&runtime, Session::new(), CoreEngine::new());
        let current = ProjectDoc::empty("new-keeps-current", "Current");
        runtime.submit(Event::Open {
            operation_id: runtime.operation(),
            document: current.clone(),
        });
        test_support::run_pending(&runtime).await;
        assert_eq!(*runtime.model().accepted.unwrap().document, current);
        let mut edited = current.clone();
        edited.name = "Current with history".into();
        let current =
            (*test_support::replace_document(&runtime, "before-new", &current.id, edited)
                .await
                .document)
                .clone();
        assert_eq!(current.revision, 1);
        runtime.submit(Event::GestureBegin {
            operation_id: runtime.operation(),
            pointer_id: 19,
            target_ids: Vec::new(),
            transaction_id: "retained-draft".into(),
            start: Vec::new(),
            pitch: boardstudio_core::model::Vec2 { x: 19.05, y: 19.05 },
            snap_fraction: 0.0,
            geometry_snap: false,
            gap: None,
            alt: false,
        });
        test_support::run_pending(&runtime).await;
        let before = runtime.model();
        assert!(before.gesture.is_some());
        let confirm = ConfirmChoice::new();
        assert!(
            runtime.create_new_keyboard().is_err(),
            "cancel must keep the current draft"
        );
        assert_eq!(confirm.calls.get(), 1);
        assert_eq!(runtime.model(), before);
        assert_eq!(
            runtime.store.list_projects().await.unwrap().documents,
            vec![current.clone()]
        );
        confirm.answer.set(true);
        let (entered, release) = test_support::gate_next_persist(&runtime);
        let (id, outcome) = runtime.create_new_keyboard().unwrap();
        test_support::drive_pending(&runtime);
        entered.await.unwrap();
        assert_eq!(runtime.model().accepted, before.accepted);
        assert_ne!(id, current.id);
        assert!(
            runtime
                .store
                .load_document(id.clone())
                .await
                .unwrap()
                .is_none()
        );
        release.send(()).unwrap();
        test_support::run_pending(&runtime).await;
        assert_eq!(*outcome.borrow(), Some(TerminalOutcome::Completed));
        let accepted = runtime.model().accepted.unwrap();
        assert_eq!(accepted.document.id, id);
        assert_ne!(
            accepted.session_epoch,
            before.accepted.unwrap().session_epoch
        );
        assert!(runtime.model().gesture.is_none());
        assert_eq!(
            runtime
                .store
                .load_document(current.id.clone())
                .await
                .unwrap(),
            Some(current)
        );
        let undo = test_support::observe_next(&runtime);
        runtime.submit(Event::Undo {
            operation_id: runtime.operation(),
        });
        test_support::run_pending(&runtime).await;
        assert!(matches!(
            &*undo.borrow(),
            Some(TerminalOutcome::Rejected(_))
        ));
        assert_eq!(runtime.model().accepted.unwrap(), accepted);
    }

    #[wasm_bindgen_test]
    async fn unsupported_saved_target_is_validated_before_discard_confirmation() {
        let runtime = test_support::new_runtime();
        test_support::install(&runtime, Session::new(), CoreEngine::new());
        let current = ProjectDoc::empty("recover-pending", "Current");
        runtime.submit(Event::Open {
            operation_id: runtime.operation(),
            document: current.clone(),
        });
        test_support::run_pending(&runtime).await;
        assert_eq!(*runtime.model().accepted.unwrap().document, current);
        test_support::fail_next_persist(&runtime, "quota");
        let mut pending = current.clone();
        pending.name = "Keep my failed edit".into();
        test_support::replace_document(&runtime, "pending-recovery", &current.id, pending).await;
        assert_eq!(runtime.model().lifecycle, Lifecycle::RecoveryRequired);
        let before = runtime.model();
        let retained = runtime.pending_document.borrow().clone().unwrap();
        let mut old = ProjectDoc::empty("recover-unsupported", "Older");
        old.format_version = 6;
        runtime
            .store
            .save_document(&old, &BTreeMap::new())
            .await
            .unwrap();
        let confirm = ConfirmChoice::new();
        let done = test_support::track_open_saved(&runtime);
        runtime.open_saved(old.id);
        assert_eq!(
            confirm.calls.get(),
            0,
            "invalid targets must not ask to discard work"
        );
        done.await.unwrap();
        test_support::run_pending(&runtime).await;
        assert_eq!(confirm.calls.get(), 0);
        assert_eq!(runtime.model(), before);
        assert!(Arc::ptr_eq(
            runtime.pending_document.borrow().as_ref().unwrap(),
            &retained
        ));
        assert!(runtime.status().contains("No conversion"));
        runtime.submit(Event::RetrySave {
            operation_id: runtime.operation(),
        });
        test_support::run_pending(&runtime).await;
        assert_eq!(
            runtime.model().accepted.unwrap().document.name,
            "Keep my failed edit"
        );
    }

    #[wasm_bindgen_test]
    async fn unsupported_saved_open_preserves_current_work_storage_and_undo() {
        let runtime = test_support::new_runtime();
        test_support::install(&runtime, Session::new(), CoreEngine::new());
        let current = ProjectDoc::empty("recovery-current", "Before edit");
        runtime.submit(Event::Open {
            operation_id: runtime.operation(),
            document: current.clone(),
        });
        test_support::run_pending(&runtime).await;
        assert_eq!(*runtime.model().accepted.unwrap().document, current);
        let mut edited = current.clone();
        edited.name = "Accepted edit".into();
        test_support::replace_document(&runtime, "recovery-edit", &current.id, edited).await;
        let before = runtime.model().accepted.unwrap();
        let scope = runtime.scope();
        let mut older = ProjectDoc::empty("recovery-older", "Older saved keyboard");
        older.format_version = 6;
        let payload = b"preserved unsupported-project payload".to_vec();
        let hash = sha256_hex(&payload);
        older.assets.push(boardstudio_core::model::Asset {
            id: "older-asset".into(),
            name: "retained.bin".into(),
            media_type: "application/octet-stream".into(),
            sha256: hash.clone(),
            license: None,
            source: None,
        });
        runtime
            .store
            .save_document(&older, &BTreeMap::from([(hash.clone(), payload.clone())]))
            .await
            .unwrap();
        let raw_before = runtime
            .store
            .raw_project(&older.id.clone().into())
            .await
            .unwrap()
            .unwrap()
            .0;
        let listing = runtime.store.list_projects().await.unwrap();
        assert_eq!(listing.documents, vec![(*before.document).clone()]);
        assert_eq!(listing.unreadable.len(), 1);
        assert!(listing.unreadable[0].error.contains("format 6"));
        let done = test_support::track_open_saved(&runtime);
        runtime.open_saved(older.id.clone());
        done.await.unwrap();
        test_support::run_pending(&runtime).await;
        assert!(runtime.status().contains("No conversion"));
        assert_eq!(runtime.model().accepted.unwrap(), before);
        assert_eq!(runtime.scope(), scope);
        assert_eq!(
            runtime
                .store
                .load_asset(hash.clone())
                .await
                .unwrap()
                .map(|bytes| bytes.to_vec()),
            Some(payload.clone())
        );
        assert_eq!(
            runtime
                .store
                .raw_project(&older.id.clone().into())
                .await
                .unwrap()
                .unwrap()
                .0,
            raw_before
        );
        assert_eq!(
            runtime
                .store
                .load_document(current.id.clone())
                .await
                .unwrap(),
            Some((*before.document).clone())
        );
        let confirm = ConfirmChoice::new();
        let incoming_bytes = b"must not be partially imported".to_vec();
        let incoming_hash = sha256_hex(&incoming_bytes);
        for (version, revision, reason) in [
            (6, 0, "No conversion"),
            (7, u64::MAX, "revision is exhausted"),
        ] {
            let mut candidate = older.clone();
            candidate.id = format!("refused-import-{version}");
            candidate.format_version = version;
            candidate.revision = revision;
            candidate.assets[0].sha256 = incoming_hash.clone();
            let metadata = serde_json::to_string(&ArchiveRequest::PackFiles {
                entries: vec![
                    ArchiveEntry {
                        path: "project.json".into(),
                        buffer_index: 0,
                    },
                    ArchiveEntry {
                        path: format!("assets/{incoming_hash}"),
                        buffer_index: 1,
                    },
                    ArchiveEntry {
                        path: "archive.json".into(),
                        buffer_index: 2,
                    },
                ],
            })
            .unwrap();
            let (reply, packed) = boardstudio_core::archive::request(
                &metadata,
                &[
                    serde_json::to_vec(&candidate).unwrap(),
                    incoming_bytes.clone(),
                    b"{\"author\":\"retained metadata\"}".to_vec(),
                ],
            );
            assert!(matches!(
                serde_json::from_str::<ArchiveReply>(&reply).unwrap(),
                ArchiveReply::Packed
            ));
            let file = web_sys::File::new_with_u8_array_sequence(
                &Array::of1(&Uint8Array::from(packed[0].as_slice()).into()),
                "refused.boardstudio",
            )
            .unwrap();
            let pending_assets = runtime.assets.borrow().clone();
            let model = runtime.model();
            let done = test_support::track_import_file(&runtime);
            runtime.import_file(file.clone());
            done.await.unwrap();
            test_support::run_pending(&runtime).await;
            assert!(runtime.status().contains(reason), "{}", runtime.status());
            assert_eq!(runtime.model(), model);
            assert_eq!(*runtime.assets.borrow(), pending_assets);
            assert_eq!(confirm.calls.get(), 0);
            assert!(
                runtime
                    .store
                    .load_document(candidate.id)
                    .await
                    .unwrap()
                    .is_none()
            );
            assert!(
                runtime
                    .store
                    .load_asset(incoming_hash.clone())
                    .await
                    .unwrap()
                    .is_none()
            );
            assert_eq!(
                Uint8Array::new(&JsFuture::from(file.array_buffer()).await.unwrap()).to_vec(),
                packed[0]
            );
        }
        runtime.submit(Event::Undo {
            operation_id: runtime.operation(),
        });
        test_support::run_pending(&runtime).await;
        assert_eq!(
            runtime.model().accepted.unwrap().document.name,
            "Before edit"
        );
        assert_eq!(
            runtime
                .store
                .load_asset(hash.clone())
                .await
                .unwrap()
                .map(|bytes| bytes.to_vec()),
            Some(payload)
        );
        assert_eq!(
            runtime
                .store
                .raw_project(&older.id.into())
                .await
                .unwrap()
                .unwrap()
                .0,
            raw_before
        );
    }

    async fn fixture() -> (Rc<Runtime>, ProjectDoc, ProjectDoc) {
        let runtime = test_support::new_runtime();
        let suffix = new_project_id().expect("browser fixture id is available");
        let (session, accepted, _) = in_process_support::opened_session();
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
        let confirm = ConfirmChoice::new();
        confirm.answer.set(true);
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
        let confirm = ConfirmChoice::new();
        confirm.answer.set(true);
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
        test_support::submit_fixed_command(runtime, rename_edit(accepted, name));
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

#[cfg(all(test, target_arch = "wasm32"))]
mod offline_safety_tests {
    use super::*;
    use boardstudio_core::model::{EditCommand, EditOperation, EditPhase};
    use wasm_bindgen_test::wasm_bindgen_test;

    #[wasm_bindgen_test]
    async fn conflicting_runtime_preserves_pending_document_and_assets_for_download() {
        let first = project_name_test_support::new_runtime();
        project_name_test_support::install(
            &first,
            Session::new(),
            boardstudio_core::CoreEngine::new(),
        );
        let mut document = in_process_support::board_document();
        document.id = format!("runtime-conflict-{}", js_sys::Date::now());
        document.name = "Original".into();
        first.submit(Event::Open {
            operation_id: first.operation(),
            document: document.clone(),
        });
        project_name_test_support::run_pending(&first).await;
        let second = Runtime::new_with_store(false, first.store.clone()).unwrap();
        project_name_test_support::install(
            &second,
            Session::new(),
            boardstudio_core::CoreEngine::new(),
        );
        let done = project_name_test_support::track_open_saved(&second);
        second.open_saved(document.id.clone());
        done.await.unwrap();
        project_name_test_support::run_pending(&second).await;
        let stale = second.model().accepted.unwrap();
        let accepted = first.model().accepted.unwrap();
        let mut winner = (*accepted.document).clone();
        winner.name = "First tab saved".into();
        project_name_test_support::submit_fixed_command(
            &first,
            EditCommand {
                base_revision: accepted.document.revision,
                transaction_id: "first-tab".into(),
                phase: EditPhase::Commit,
                target_ids: Vec::new(),
                operation: EditOperation::ReplaceDocument {
                    document: Box::new(winner),
                },
            },
        );
        project_name_test_support::run_pending(&first).await;
        let mut pending = (*stale.document).clone();
        pending.name = "Second tab pending".into();
        let bytes = b"model retained after conflict".to_vec();
        let hash: String = Sha256::digest(&bytes)
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect();
        pending.assets.push(
            serde_json::from_value(serde_json::json!({
                "id":"pending-model", "name":"pending.step", "mediaType":"model/step", "sha256":hash
            }))
            .unwrap(),
        );
        second
            .assets
            .borrow_mut()
            .insert(hash.clone(), bytes.clone());
        project_name_test_support::submit_fixed_command(
            &second,
            EditCommand {
                base_revision: stale.document.revision,
                transaction_id: "second-tab".into(),
                phase: EditPhase::Commit,
                target_ids: Vec::new(),
                operation: EditOperation::ReplaceDocument {
                    document: Box::new(pending),
                },
            },
        );
        project_name_test_support::run_pending(&second).await;
        assert_eq!(second.model().lifecycle, Lifecycle::RecoveryRequired);
        assert!(second.has_save_conflict());
        assert_eq!(second.model().accepted.unwrap().document.name, "Original");
        assert_eq!(
            first
                .store
                .load_document(document.id.clone())
                .await
                .unwrap()
                .unwrap()
                .name,
            "First tab saved"
        );
        second.submit(Event::RetrySave {
            operation_id: second.operation(),
        });
        project_name_test_support::run_pending(&second).await;
        assert!(
            second.has_save_conflict(),
            "retry cannot adopt the other tab's token"
        );
        let window = web_sys::window().unwrap();
        let old_confirm = Reflect::get(&window, &"confirm".into()).unwrap();
        let cancel = Closure::<dyn FnMut(JsValue) -> bool>::new(|_| false);
        Reflect::set(&window, &"confirm".into(), cancel.as_ref()).unwrap();
        let done = project_name_test_support::track_open_saved(&second);
        second.open_saved(document.id.clone());
        done.await.unwrap();
        assert_eq!(
            second.delete_saved_project("another-keyboard".into()).await,
            Err("Pending edits were kept; no keyboard was deleted.".into())
        );
        Reflect::set(&window, &"confirm".into(), &old_confirm).unwrap();
        drop(cancel);
        assert_eq!(second.model().lifecycle, Lifecycle::RecoveryRequired);
        second.submit(Event::RetrySave {
            operation_id: second.operation(),
        });
        project_name_test_support::run_pending(&second).await;
        assert!(
            second.has_save_conflict(),
            "cancelled recovery must not adopt a new storage token"
        );
        let (captured, archive) = second.pending_archive().await.unwrap();
        assert_eq!(captured.name, "Second tab pending");
        let (reply, buffers) =
            boardstudio_core::archive::request("{\"kind\":\"unpack-project\"}", &[archive]);
        let ArchiveReply::Unpacked {
            project_json,
            assets,
        } = serde_json::from_str(&reply).unwrap()
        else {
            panic!("pending copy was not a valid archive: {reply}")
        };
        let unpacked: ProjectDoc = serde_json::from_str(&project_json).unwrap();
        assert_eq!(unpacked.name, "Second tab pending");
        assert_eq!(buffers[assets[0].buffer_index as usize], bytes);
        assert!(
            second.store.load_asset(hash).await.unwrap().is_none(),
            "the conflicting asset write was rolled back"
        );
    }

    #[wasm_bindgen_test]
    async fn archive_exactly_at_size_limit_reaches_normal_validation() {
        let window = web_sys::window().unwrap();
        let original = Reflect::get(&window, &"confirm".into()).unwrap();
        let confirm = Closure::<dyn FnMut(JsValue) -> bool>::new(|_| true);
        Reflect::set(&window, &"confirm".into(), confirm.as_ref()).unwrap();
        let runtime = project_name_test_support::new_runtime();
        project_name_test_support::open_document(&runtime, in_process_support::board_document())
            .await;
        let imported = ProjectDoc::empty("archive-at-limit", "At limit");
        let (mut entered, release) =
            project_name_test_support::gate_import_archive(&runtime, &imported);
        let done = project_name_test_support::track_import_file(&runtime);
        let file = web_sys::File::new_with_u8_array_sequence(&Array::new(), "at-limit.boardstudio")
            .unwrap();
        let descriptor = Object::new();
        Reflect::set(
            &descriptor,
            &"value".into(),
            &JsValue::from_f64(128.0 * 1024.0 * 1024.0),
        )
        .unwrap();
        Object::define_property(&Object::from(file.clone()), &"size".into(), &descriptor);
        runtime.import_file(file);
        let mut admitted = false;
        for _ in 0..200 {
            if matches!(entered.try_recv(), Ok(Some(()))) {
                admitted = true;
                break;
            }
            TimeoutFuture::new(1).await;
        }
        assert!(
            admitted,
            "exactly-at-limit files must reach Core: {}",
            runtime.status()
        );
        release.send(()).unwrap();
        done.await.unwrap();
        Reflect::set(&window, &"confirm".into(), &original).unwrap();
        project_name_test_support::run_pending(&runtime).await;
        assert_eq!(
            runtime.model().accepted.unwrap().document.id,
            "archive-at-limit"
        );
    }

    #[wasm_bindgen_test]
    async fn oversized_import_is_rejected_before_opening_or_reading() {
        let runtime = project_name_test_support::new_runtime();
        project_name_test_support::open_document(&runtime, in_process_support::board_document())
            .await;
        let before = runtime.model().accepted.unwrap();
        let file =
            web_sys::File::new_with_u8_array_sequence(&Array::new(), "oversized.boardstudio")
                .unwrap();
        let descriptor = Object::new();
        Reflect::set(
            &descriptor,
            &"value".into(),
            &JsValue::from_f64(128.0 * 1024.0 * 1024.0 + 1.0),
        )
        .unwrap();
        Object::define_property(&Object::from(file.clone()), &"size".into(), &descriptor);
        runtime.import_file(file);
        assert!(
            runtime.status().contains("128 MiB"),
            "oversized files must be rejected synchronously: {}",
            runtime.status()
        );
        assert_eq!(runtime.model().accepted.unwrap().token, before.token);
    }
}

/// Quota-recovery RED tests: with origin storage full every save aborts, so the
/// in-app delete path must not rewrite durable state before the single-key
/// delete, and create-new must discard the latched pending doc without
/// committing it. Saves are failed through the persistence port (the quota
/// simulation); single-key deletes go straight to the store and must succeed.
#[cfg(all(test, target_arch = "wasm32"))]
mod quota_recovery_deletion_tests {
    use super::*;
    use crate::runtime::project_name_test_support as test_support;
    use boardstudio_core::CoreEngine;
    use wasm_bindgen_test::wasm_bindgen_test;

    struct ConfirmChoice {
        original: JsValue,
        calls: Rc<Cell<u32>>,
        answer: Rc<Cell<bool>>,
        _callback: Closure<dyn FnMut(JsValue) -> bool>,
    }

    impl ConfirmChoice {
        fn new() -> Self {
            let window = web_sys::window().unwrap();
            let original = Reflect::get(&window, &"confirm".into()).unwrap();
            let calls = Rc::new(Cell::new(0));
            let answer = Rc::new(Cell::new(false));
            let count = calls.clone();
            let result = answer.clone();
            let callback = Closure::<dyn FnMut(JsValue) -> bool>::new(move |_| {
                count.set(count.get() + 1);
                result.get()
            });
            Reflect::set(&window, &"confirm".into(), callback.as_ref()).unwrap();
            Self {
                original,
                calls,
                answer,
                _callback: callback,
            }
        }
    }

    impl Drop for ConfirmChoice {
        fn drop(&mut self) {
            Reflect::set(
                &web_sys::window().unwrap(),
                &"confirm".into(),
                &self.original,
            )
            .unwrap();
        }
    }

    async fn quota_latched_runtime(active_name: &str) -> Rc<Runtime> {
        let runtime = test_support::new_runtime();
        test_support::install(&runtime, Session::new(), CoreEngine::new());
        let active = ProjectDoc::empty("quota-delete-active", active_name);
        runtime.submit(Event::Open {
            operation_id: runtime.operation(),
            document: active.clone(),
        });
        test_support::run_pending(&runtime).await;
        let other = ProjectDoc::empty("quota-delete-other", "Other board");
        runtime
            .store
            .save_document(&other, &BTreeMap::new())
            .await
            .unwrap();
        test_support::fail_next_persist(&runtime, "QuotaExceededError: origin storage is full");
        let mut edited = active.clone();
        edited.name = "Oversized failed edit".into();
        test_support::replace_document(&runtime, "quota-delete-edit", &active.id, edited).await;
        assert_eq!(runtime.model().lifecycle, Lifecycle::RecoveryRequired);
        assert!(
            runtime.has_pending_edits(),
            "the aborted save latches pending edits"
        );
        runtime
    }

    #[wasm_bindgen_test]
    async fn quota_delete_non_active_skips_recovery_writes_and_keeps_pending_for_explicit_retry() {
        let runtime = quota_latched_runtime("Active board").await;
        // Persistent quota: every save aborts, not just the latched one.
        test_support::fail_next_persist(&runtime, "QuotaExceededError: origin storage is full");
        let active_id = "quota-delete-active";
        let mut pending_target = test_support::track_next_persist_target(&runtime);
        let confirm = ConfirmChoice::new();
        confirm.answer.set(true);
        let result = runtime
            .delete_saved_project("quota-delete-other".into())
            .await;
        assert!(
            result.is_ok(),
            "quota delete must succeed where the save aborted: {result:?}"
        );
        assert_eq!(
            confirm.calls.get(),
            1,
            "one explicit discard-confirm gates the recovery delete"
        );
        test_support::run_pending(&runtime).await;
        assert!(
            pending_target.try_recv().unwrap().is_none(),
            "no recovery or replacement write may run before the single-key delete"
        );
        assert!(
            runtime
                .store
                .load_document("quota-delete-other".into())
                .await
                .unwrap()
                .is_none(),
            "the other keyboard is gone"
        );
        assert_eq!(
            runtime
                .store
                .load_document(active_id.into())
                .await
                .unwrap()
                .unwrap()
                .name,
            "Active board",
            "the durable active copy is untouched"
        );
        assert_eq!(runtime.model().accepted.unwrap().document.id, active_id);
        assert_eq!(runtime.model().lifecycle, Lifecycle::RecoveryRequired);
        assert!(
            runtime.has_pending_edits(),
            "the pending doc stays available for explicit retry or download"
        );
        let remaining = runtime.store.list_documents().await.unwrap();
        assert!(
            remaining
                .iter()
                .all(|document| document.id != "quota-delete-fresh"),
            "no replacement keyboard is created by the delete"
        );
    }

    #[wasm_bindgen_test]
    async fn quota_delete_active_removes_only_the_saved_copy() {
        let runtime = quota_latched_runtime("Active board").await;
        // Persistent quota: every save aborts, not just the latched one.
        test_support::fail_next_persist(&runtime, "QuotaExceededError: origin storage is full");
        let active_id = "quota-delete-active";
        let confirm = ConfirmChoice::new();
        confirm.answer.set(true);
        let result = runtime.delete_saved_project(active_id.into()).await;
        assert!(
            result.is_ok(),
            "quota delete of the active keyboard must succeed: {result:?}"
        );
        test_support::run_pending(&runtime).await;
        assert!(
            runtime
                .store
                .load_document(active_id.into())
                .await
                .unwrap()
                .is_none(),
            "the active saved copy is gone"
        );
        assert!(
            runtime
                .store
                .load_document("quota-delete-other".into())
                .await
                .unwrap()
                .is_some(),
            "the other keyboard is untouched"
        );
        assert_eq!(
            runtime.model().accepted.unwrap().document.id,
            active_id,
            "the accepted in-memory keyboard stays for the explicit next step"
        );
        assert_eq!(runtime.model().lifecycle, Lifecycle::RecoveryRequired);
        assert!(
            !runtime.saved_tokens.borrow().contains_key(active_id),
            "the saved token is dropped with the record"
        );
    }

    #[wasm_bindgen_test]
    async fn quota_create_new_discards_pending_without_committing_oversized_doc() {
        let runtime = quota_latched_runtime("Active board").await;
        test_support::fail_next_persist(&runtime, "QuotaExceededError: origin storage is full");
        let confirm = ConfirmChoice::new();
        confirm.answer.set(true);
        let (id, outcome) = runtime
            .create_new_keyboard()
            .expect("discard-confirmed create-new must replace");
        assert_eq!(confirm.calls.get(), 1);
        assert!(
            !runtime.has_pending_edits(),
            "the explicit discard clears the latched pending doc immediately"
        );
        test_support::run_pending(&runtime).await;
        assert!(
            matches!(
                &*outcome.borrow(),
                Some(TerminalOutcome::PersistenceFailed(_))
            ),
            "the fresh save fails explicitly under quota instead of retrying silently"
        );
        assert_eq!(
            runtime.model().accepted.unwrap().document.id,
            "quota-delete-active",
            "under quota the accepted keyboard stays until a save succeeds"
        );
        let durable = runtime
            .store
            .load_document("quota-delete-active".into())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            durable.name, "Active board",
            "the oversized doc is never committed"
        );
        assert_eq!(durable.revision, 0);
        // The escape hatch retains only the fresh small doc: an explicit retry
        // (quota simulation spent) commits and installs it, proving the
        // oversized candidate was discarded rather than retried.
        runtime.submit(Event::RetrySave {
            operation_id: runtime.operation(),
        });
        test_support::run_pending(&runtime).await;
        assert_eq!(
            runtime.model().accepted.unwrap().document.id,
            id,
            "explicit retry installs the fresh keyboard, not the oversized edit"
        );
        assert!(
            runtime
                .store
                .load_document(id.clone())
                .await
                .unwrap()
                .is_some(),
            "the fresh keyboard is durably saved"
        );
    }
}
