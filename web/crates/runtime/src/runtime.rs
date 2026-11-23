//! Browser composition runs identified effects; the headless session remains authoritative.
pub use crate::core_services::CoreServices;
use crate::mechanical_package::{
    critical_fit_drawing, mechanical_fabrication_notes, mechanical_filename_component,
    mechanical_stl, push_mechanical_file,
};
use crate::model_delivery::{
    ModelDeliveryAdapter, ModelDeliveryPorts, ModelDeliveryRows, ModelOwnerIdentity,
    ModelSourceLease,
};
use crate::native_model_mapping::{NativeModelDeliveryState, resolve_native_model_paths};
use crate::physical_pcb::{artifact_reply, install_source_pcb, verify_source_digest};
pub use crate::runtime_report::RuntimeNotification;
use crate::runtime_report::{RuntimeReport, publish};
use boardstudio_application::{
    AcceptedSnapshot, Completion, Durability, Effect, Event, JobId, Lifecycle, OperationId,
    ReadModel, SaveResult, Scope, Session, SnapshotToken, TerminalOutcome,
};
use boardstudio_core_mechanical::electrical::{
    ElectricalMode, ElectricalPlan, ElectricalPlanRequest,
};
use boardstudio_model::model::{
    ArchiveReply, ArtifactReply, ArtifactRequest, Board, BoardReference, CaseAssemblyIR, CaseIR,
    CompiledFootprint, CoreReply, CoreRequest, HardwareTopology, KeycapSpec, Material,
    MechanicalAssembly, MechanicalBuiltinProfile, MechanicalConfiguration, MechanicalExtraction,
    MechanicalPartProfile, MechanicalPurposeMapping, MechanicalSwitchFamily, Operation,
    OutlineFeature, OutlineSettings, PcbModel, PcbPreview, PrepareExportRequest, ProjectDoc,
};

use boardstudio_web_host::browser::browser_uuid;
use boardstudio_web_host::host::{
    BrowserStore, SaveError, SavedProject, StorageSession, StorageToken,
};
use boardstudio_web_host::{
    cad_jobs::{
        CadJobError, CadOperation, CadRequest, CadResult, CadSnapshotIdentity, MAX_SAFE_INTEGER,
        captured_case_document, captured_case_scene, captured_pcb_source, prepare_captured_case,
        validate_reply,
    },
    cad_worker::CadWorker,
};

fn new_cad_worker() -> Result<Rc<CadWorker>, String> {
    let url = resource_url("assets/cad-worker/entry.js")?;
    CadWorker::new(&url)
        .map(Rc::new)
        .map_err(|error| error.to_string())
}

fn prepared_cad_request(
    request_id: String,
    job_id: String,
    identity: &CadSnapshotIdentity,
    operation: CadOperation,
    prepared: boardstudio_model::model::PreparedCaseAssemblyIR,
) -> CadRequest {
    CadRequest {
        request_id,
        job_id,
        identity: identity.clone(),
        operation,
        prepared: Some(prepared),
        source_sha256: None,
        step_import_options: None,
        collision_query: None,
        generated_analysis_request: None,
        collision_meshes: None,
        input_bytes: Vec::new(),
    }
}

struct PhysicalStepAssembly {
    prepared: boardstudio_web_host::cad_jobs::PreparedCadSnapshot,
    pcb_source: Option<boardstudio_web_host::cad_jobs::CapturedPcbSource>,
    pcb_source_thickness: Option<f64>,
}

pub use crate::case_generation_lifecycle::CadScene;

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
use js_sys::{Array, Reflect, Uint8Array};
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
fn terminal_report(outcome: TerminalOutcome, export: Option<&ExportKind>) -> Option<RuntimeReport> {
    let (message, alert, transient) = match outcome {
        TerminalOutcome::Completed => match export? {
            ExportKind::BoardOutline { .. } => return None,
            kind => {
                let status = match kind {
                    ExportKind::Firmware => "Saved locally.",
                    ExportKind::ProjectCopy { .. } => "Project copy downloaded.",
                    _ => "Export finished.",
                };
                (status.into(), false, true)
            }
        },
        TerminalOutcome::Rejected(reason) | TerminalOutcome::ExecutorFailed(reason) => {
            (reason, export.is_some(), export.is_some())
        }
        TerminalOutcome::PersistenceFailed(_) => return None,
        TerminalOutcome::Cancelled => (
            "Cancelled.".into(),
            false,
            !matches!(export, Some(ExportKind::BoardOutline { .. })),
        ),
        TerminalOutcome::Closed => ("Editor closed.".into(), false, false),
        TerminalOutcome::Superseded => return None,
    };
    Some(RuntimeReport {
        message,
        is_alert: alert,
        notification_error: transient.then_some(alert),
    })
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
    fn observed_token(&self, id: &str) -> Option<StorageToken>;
    fn observe_token(&self, id: String, token: StorageToken);

    fn save_document<'a>(
        &'a self,
        document: &'a ProjectDoc,
        assets: &'a BTreeMap<String, Vec<u8>>,
        expected: Option<&'a StorageToken>,
    ) -> Pin<Box<dyn Future<Output = Result<StorageToken, SaveError>> + 'a>>;
}

impl DocumentPersistence for BrowserStore {
    fn observed_token(&self, id: &str) -> Option<StorageToken> {
        BrowserStore::observed_token(self, id)
    }

    fn observe_token(&self, id: String, token: StorageToken) {
        BrowserStore::observe_token(self, id, token);
    }

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

impl PartialEq for CadScene {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
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

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum ExportKind {
    ProjectCopy { embed_used_models: bool },
    Step,
    KeycapsStl,
    Firmware,
    Footprints,
    PcbHandoff { draft: bool },
    Mechanical { filename: String },
    BoardOutline { dxf: bool },
}

#[derive(Clone)]
struct ExportState {
    kind: ExportKind,
    token: SnapshotToken,
    scope: Scope,
}

type BeforeUnloadClosure = Closure<dyn FnMut(web_sys::Event)>;

pub struct Runtime {
    session: RefCell<Session>,
    core_services: CoreServices,
    pub(crate) operation_outcomes: crate::operation_outcomes::OperationOutcomes,
    pub store: BrowserStore,
    persistence: RefCell<Rc<dyn DocumentPersistence>>,
    storage_session: RefCell<Option<StorageSession>>,
    storage_initializing: Cell<bool>,
    storage_error: RefCell<Option<String>>,
    before_unload: RefCell<Option<BeforeUnloadClosure>>,
    next_operation: Cell<u64>,
    assets: RefCell<BTreeMap<String, Vec<u8>>>,
    exports: RefCell<BTreeMap<OperationId, ExportState>>,
    frames: RefCell<BTreeMap<u64, Frame>>,
    surface: RefCell<Option<SvgElement>>,
    notify: RefCell<Option<Notifier>>,
    notify_durability: RefCell<Option<Notifier>>,
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
    export_workers: RefCell<BTreeMap<OperationId, Rc<CadWorker>>>,
    native_case_preview: crate::case_preview::NativePreviewState,
    case_model_delivery: ModelDeliveryAdapter,
    // Tight preparation must not consume the separate missing-evidence retry.
    collision_vendor_refinement: crate::model_delivery::ModelDeliveryAdapter,
    native_model_delivery: RefCell<NativeModelDeliveryState>,
    layout_preview: crate::layout_viewer_source::LayoutPreviewState,
    layout_model_rows: RefCell<
        Option<(
            crate::layout_viewer_source::LayoutSourceIdentity,
            crate::model_delivery::ModelDeliveryRows,
        )>,
    >,
    native_model_jobs: RefCell<BTreeSet<String>>,
    generated_analysis_cache:
        RefCell<BTreeMap<String, Rc<boardstudio_web_host::cad_jobs::CadGeneratedAnalysis>>>,
    generated_analysis_prepared:
        RefCell<BTreeMap<String, boardstudio_model::model::PreparedCaseAssemblyIR>>,
    generated_analysis_pending: RefCell<BTreeMap<String, PendingGeneratedAnalysis>>,
    collision_batch: Cell<u64>,
    collision_alive: RefCell<Rc<Cell<bool>>>,
    collision_generated_attempts: RefCell<BTreeSet<String>>,
    embed_used_models: Cell<bool>,
    #[cfg(any(test, feature = "test-support"))]
    test: TestRuntimeState,
}

#[cfg(any(test, feature = "test-support"))]
type CaseGesturePreviewTestExecutor = dyn Fn(AcceptedSnapshot, Scope) -> Result<CadScene, String>;

#[cfg(any(test, feature = "test-support"))]
type KeycapsPreviewTestExecutor = dyn Fn(
    KeycapsPreviewInput,
) -> crate::model_delivery::ModelFuture<
    Vec<boardstudio_web_host::cad_jobs::CadBodyMesh>,
>;

#[cfg(any(test, feature = "test-support"))]
#[derive(Default)]
struct TestRuntimeState {
    case_gesture_preview_test_executor: RefCell<Option<Rc<CaseGesturePreviewTestExecutor>>>,
    keycaps_preview_test_executor: RefCell<Option<Rc<KeycapsPreviewTestExecutor>>>,
    in_process_adapters: RefCell<Option<Rc<dyn test_hooks::InstalledAdapters>>>,
    held_effects: RefCell<Vec<Effect>>,
    in_process_drives: RefCell<Vec<futures_channel::oneshot::Receiver<()>>>,
}

#[cfg(any(test, feature = "test-support"))]
impl Runtime {
    pub fn set_case_gesture_preview_executor_test(
        &self,
        executor: Rc<CaseGesturePreviewTestExecutor>,
    ) {
        *self.test.case_gesture_preview_test_executor.borrow_mut() = Some(executor);
    }

    pub fn case_gesture_preview_active_test(&self) -> bool {
        self.case_gesture_preview.borrow().active_owner().is_some()
    }

    pub fn set_cad_scene_test(&self, scene: Option<Rc<CadScene>>) {
        *self.cad_scene.borrow_mut() = scene;
    }

    pub fn set_keycaps_preview_executor_test(&self, executor: Rc<KeycapsPreviewTestExecutor>) {
        *self.test.keycaps_preview_test_executor.borrow_mut() = Some(executor);
    }

    pub fn install_case_pcb_preview_test(&self) {
        let accepted = self.accepted_snapshot().expect("accepted preview fixture");
        let scope = self.scope().expect("selected physical fixture scope");
        let generation = self
            .native_case_preview
            .next_generation("Case preview generation identity exhausted")
            .unwrap();
        let capture = crate::case_preview::capture_native_preview(
            &accepted,
            &scope,
            generation,
            format!("case-keycaps-test-{generation}"),
        )
        .expect("valid accepted Case preview input");
        self.set_native_preview_pending(capture.lease.clone());
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
            .publish(
                preview,
                generation,
                "fixture owner changed before publication",
            )
            .unwrap();
        self.changed();
    }
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
fn packaged_model_asset(asset_id: &str) -> Option<crate::model_delivery::ResolvedModelAsset> {
    use crate::model_delivery::{ModelAssetSource, ResolvedModelAsset};
    let model = crate::bundled_models::bundled_model(asset_id)?;
    Some(ResolvedModelAsset {
        id: model.id.to_owned(),
        sha256: model.sha256.to_owned(),
        filename: model.filename.to_owned(),
        source: ModelAssetSource::Packaged {
            url_path: model.url_path.to_owned(),
        },
    })
}

fn unique_model_paths(models: &[PcbModel]) -> Vec<String> {
    models
        .iter()
        .map(|model| model.path.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

async fn bundled_preview_paths(
    models: &[PcbModel],
) -> Result<BTreeMap<String, Option<String>>, String> {
    let paths = unique_model_paths(models);
    if paths.is_empty() {
        return Ok(BTreeMap::new());
    }
    let ids = crate::bundled_models::generated_model_asset_ids_for_paths(&paths).await?;
    if ids.len() != paths.len() {
        return Err("Model path resolver returned an incomplete mapping".into());
    }
    Ok(paths.into_iter().zip(ids).collect())
}

struct PreviewModelBatch<'a> {
    adapter: &'a ModelDeliveryAdapter,
    owner: ModelOwnerIdentity,
    ports: ModelDeliveryPorts,
    is_current: Rc<dyn Fn() -> bool>,
}

impl PreviewModelBatch<'_> {
    async fn deliver_board_models(
        &self,
        models: &[PcbModel],
        reference: Option<&BoardReference>,
        path_assets: &BTreeMap<String, String>,
        document: &ProjectDoc,
        bundled_ids_by_path: &BTreeMap<String, Option<String>>,
    ) -> Option<ModelDeliveryRows> {
        let native_paths = crate::model_delivery::native_model_path_assets(path_assets);
        let selections = crate::model_delivery::resolve_preview_assets(
            models,
            reference,
            &native_paths,
            document,
            |path| bundled_ids_by_path.get(path).cloned().flatten(),
            packaged_model_asset,
        )
        .into_iter()
        .collect();
        self.adapter
            .deliver_models(
                models,
                &selections,
                &self.ports,
                &self.owner,
                self.is_current.clone(),
            )
            .await
    }
}

mod layout_preview;
mod mechanical_export;
mod projects;
mod scene_preview;
mod sources;

use mechanical_export::authored_case_filename;

impl Runtime {
    fn preview_model_batch(
        self: &Rc<Self>,
        scope: &Scope,
        token: SnapshotToken,
        revision: u64,
        lease: Rc<dyn ModelSourceLease>,
        is_current: Rc<dyn Fn() -> bool>,
    ) -> PreviewModelBatch<'_> {
        PreviewModelBatch {
            adapter: &self.case_model_delivery,
            owner: ModelOwnerIdentity::capture(token, lease),
            ports: self.model_delivery_ports(scope.clone(), token, revision, is_current.clone()),
            is_current,
        }
    }

    fn case_cad_worker(
        &self,
        scope: &Scope,
        close_replaced: bool,
    ) -> Result<Rc<CadWorker>, String> {
        crate::case_model_lifecycle::case_model_worker(
            &self.cad_worker,
            scope,
            |worker| !worker.is_closed(),
            || {
                // A replaced gesture worker must cancel its in-flight requests immediately.
                if close_replaced && let Some((_, previous)) = self.cad_worker.borrow_mut().take() {
                    previous.close();
                }
                new_cad_worker()
            },
        )
    }

    async fn load_document_asset(&self, sha256: &str) -> Result<Option<Vec<u8>>, String> {
        let cached = self.assets.borrow().get(sha256).cloned();
        if cached.is_some() {
            return Ok(cached);
        }
        self.store
            .load_asset(sha256.to_owned())
            .await
            .map(|bytes| bytes.map(|bytes| bytes.to_vec()))
            .map_err(|error| error.to_string())
    }

    /// Keep one injected engine for the document; scratch probes create their own engines.
    /// Archive and artifact operations also execute in process.
    pub fn new(core_services: CoreServices) -> Result<Rc<Self>, String> {
        Self::new_with_restoration(true, core_services)
    }

    fn new_with_restoration(
        restore_active_project: bool,
        core_services: CoreServices,
    ) -> Result<Rc<Self>, String> {
        let prefix = deployment_prefix()?;
        let store = if restore_active_project {
            BrowserStore::scoped(if prefix == "/" { "root" } else { "boardstudio" })
        } else {
            BrowserStore::new(format!("boardstudio-test-{}", browser_uuid()?))
        }
        .map_err(|e| e.to_string())?;
        Self::new_with_store(restore_active_project, store, core_services)
    }

    fn new_with_store(
        restore_active_project: bool,
        store: BrowserStore,
        core_services: CoreServices,
    ) -> Result<Rc<Self>, String> {
        #[cfg(not(any(test, feature = "test-support")))]
        let prefix = deployment_prefix()?;
        let runtime = Rc::new(Self {
            session: RefCell::new(Session::new(core_services.validate_open)),
            core_services,
            operation_outcomes: crate::operation_outcomes::OperationOutcomes::default(),
            persistence: RefCell::new(Rc::new(store.clone())),
            store,
            storage_session: RefCell::default(),
            storage_initializing: Cell::new(restore_active_project),
            storage_error: RefCell::new(None),
            before_unload: RefCell::default(),
            next_operation: Cell::new(1),
            assets: RefCell::new(BTreeMap::new()),
            exports: RefCell::default(),
            frames: RefCell::new(BTreeMap::new()),
            surface: RefCell::new(None),
            notify: RefCell::default(),
            notify_durability: RefCell::default(),
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
            export_workers: RefCell::new(BTreeMap::new()),
            native_case_preview: Default::default(),
            case_model_delivery: Default::default(),
            collision_vendor_refinement: Default::default(),
            native_model_delivery: RefCell::new(Default::default()),
            layout_preview: Default::default(),
            layout_model_rows: RefCell::new(None),
            native_model_jobs: RefCell::new(BTreeSet::new()),
            generated_analysis_cache: RefCell::new(BTreeMap::new()),
            generated_analysis_prepared: RefCell::new(BTreeMap::new()),
            generated_analysis_pending: RefCell::new(BTreeMap::new()),
            collision_batch: Cell::new(0),
            collision_alive: RefCell::new(Rc::new(Cell::new(true))),
            collision_generated_attempts: RefCell::new(BTreeSet::new()),
            embed_used_models: Cell::new(true),
            #[cfg(any(test, feature = "test-support"))]
            test: Default::default(),
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
        if restore_active_project && let Some(window) = web_sys::window() {
            let runtime_ref = Rc::downgrade(&runtime);
            let listener =
                Closure::<dyn FnMut(web_sys::Event)>::new(move |event: web_sys::Event| {
                    if runtime_ref.upgrade().is_some_and(|runtime| {
                        runtime.with_model(|model| {
                            matches!(
                                model.durability,
                                Durability::Saving { .. } | Durability::Failed { .. }
                            )
                        })
                    }) {
                        event.prevent_default();
                        let _ = Reflect::set(event.as_ref(), &"returnValue".into(), &JsValue::TRUE);
                    }
                });
            if window
                .add_event_listener_with_callback("beforeunload", listener.as_ref().unchecked_ref())
                .is_ok()
            {
                *runtime.before_unload.borrow_mut() = Some(listener);
            }
        }
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
    /// The Core entry points the page injected (Open admission and scratch probes).
    pub fn core_services(&self) -> CoreServices {
        self.core_services
    }
    pub fn model(&self) -> ReadModel {
        self.with_model(Clone::clone)
    }

    /// Read the current model during a synchronous borrow of the session.
    pub fn with_model<R>(&self, f: impl FnOnce(&ReadModel) -> R) -> R {
        f(self.session.borrow().read_model())
    }

    fn accepted_snapshot(&self) -> Option<AcceptedSnapshot> {
        self.with_model(|model| model.accepted.clone())
    }

    /// Load a reviewed bundled switch fit into the Parts editor's local draft.
    /// This never submits an accepted edit; the existing Parts profile Save path
    /// remains the sole owner of document history.
    pub fn standard_switch_profile(
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
        let request_id = format!("parts-standard-profile-{}", operation_id.0);
        let request = CoreRequest::MechanicalProfile {
            id: request_id,
            definition_id,
            source,
            plate_to_pcb,
        };
        match (self.core_services.open_scratch)()(request) {
            CoreReply::MechanicalProfile { profile, .. } => Ok(profile),
            CoreReply::Error { message, .. } => Err(message),
            _ => Err("Core returned an unexpected standard mechanical profile reply.".into()),
        }
    }

    /// Import a source-owned KiCad footprint through the existing artifact worker.
    /// The Parts owner admits the returned definition into Session after rechecking scope.
    pub fn import_footprint(
        &self,
        request_id: String,
        definition_id: String,
        source: String,
    ) -> Result<CompiledFootprint, String> {
        let request = ArtifactRequest::ImportFootprint {
            id: request_id,
            definition_id,
            source,
        };
        match artifact_reply(&request)? {
            ArtifactReply::ImportFootprint { result, .. } => Ok(*result),
            _ => Err("Unexpected KiCad artifact reply.".into()),
        }
    }

    /// Read KiCad mechanical geometry for a Parts editor draft. The caller owns the
    /// selected-definition and scope admission before applying any returned geometry.
    pub fn extract_mechanical_profile(
        &self,
        request_id: String,
        source: String,
        mappings: Vec<MechanicalPurposeMapping>,
        max_deviation_mm: f64,
    ) -> Result<MechanicalExtraction, String> {
        let request = ArtifactRequest::ExtractMechanical {
            id: request_id,
            source,
            mappings,
            max_deviation_mm,
        };
        match artifact_reply(&request)? {
            ArtifactReply::ExtractMechanical { result, .. } => Ok(result),
            _ => Err("Unexpected KiCad artifact reply.".into()),
        }
    }

    /// Ask the document Core to project candidate matrices with its authoritative
    /// layout geometry. This is a private preview path; candidates are never installed in Session.
    pub fn project_matrices(
        &self,
        base_revision: u64,
        matrices: Vec<boardstudio_model::model::Matrix>,
    ) -> Result<Vec<boardstudio_model::model::MatrixScene>, String> {
        let expected_ids: Vec<_> = matrices.iter().map(|matrix| matrix.id.clone()).collect();
        if self.session.borrow().read_model().lifecycle == Lifecycle::Closed {
            return Err("Matrix preview failed: editor is closed".into());
        }
        let reply =
            self.session
                .borrow_mut()
                .project_matrices(String::new(), base_revision, matrices);
        match reply {
            CoreReply::MatrixProjections {
                revision,
                matrix_scenes,
                ..
            } if revision == base_revision => {
                let actual_ids: Vec<_> = matrix_scenes
                    .iter()
                    .map(|scene| scene.matrix_id.clone())
                    .collect();
                if actual_ids != expected_ids {
                    return Err("Core returned a different matrix preview set.".into());
                }
                Ok(matrix_scenes)
            }
            CoreReply::Error { message, .. } => Err(message),
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
        self.status.borrow().is_alert
    }
    pub fn status_is_transient(&self) -> bool {
        self.status.borrow().notification_error.is_some()
    }
    /// Drain every pending result in order, including identical results between renders.
    /// Only the shell consumes this queue; reading it does not trigger another render.
    pub fn take_notifications(&self) -> Vec<RuntimeNotification> {
        crate::runtime_report::take_notifications(&self.notifications)
    }
    pub fn embed_used_models(&self) -> bool {
        self.embed_used_models.get()
    }
    pub fn set_embed_used_models(&self, value: bool) {
        if self.embed_used_models.replace(value) != value {
            self.changed();
        }
    }
    pub fn subscribe(&self, notify: Notifier) {
        *self.notify.borrow_mut() = Some(notify);
    }
    /// Observe save-only changes independently of workbench snapshots.
    pub fn subscribe_durability(&self, notify: Notifier) {
        *self.notify_durability.borrow_mut() = Some(notify);
    }

    pub fn unsubscribe_durability(&self) {
        self.notify_durability.borrow_mut().take();
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
        self.record_report(report);
        self.changed();
    }

    fn record_report(&self, report: RuntimeReport) {
        publish(&self.status, &self.notifications, report, || {});
    }
    fn changed(&self) {
        if let Some(notify) = self.notify.borrow().as_ref().cloned() {
            notify();
        }
    }
    fn invalidate_stale_native_case_preview(&self) {
        let stale = self
            .native_case_preview
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
            && self.snapshot_current(owner.snapshot_token, &owner.scope)
    }

    pub fn cancel_native_case_preview(&self) {
        self.native_case_preview
            .retire_generation(self.native_case_preview.generation());
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

impl Runtime {
    fn close_browser_workers(&self) {
        self.cancel_frames();
        if let Some((_, worker)) = self.keycaps_preview_worker.borrow_mut().take() {
            worker.close();
        }
        if let Some((_, worker)) = self.cad_worker.borrow_mut().take() {
            worker.close();
        }
        for (_, worker) in std::mem::take(&mut *self.export_workers.borrow_mut()) {
            worker.close();
        }
    }
}

impl Drop for Runtime {
    fn drop(&mut self) {
        if let Some(listener) = self.before_unload.get_mut().take()
            && let Some(window) = web_sys::window()
        {
            let _ = window.remove_event_listener_with_callback(
                "beforeunload",
                listener.as_ref().unchecked_ref(),
            );
        }
        self.close_browser_workers();
    }
}

#[path = "collision_check.rs"]
mod collision_check;

pub use collision_check::{CollisionCheckInput, CollisionCheckReport};

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
