//! Dedicated host for the separately built Monstertruck CAD WASM package.

mod analysis_storage;
mod dispatch;
mod protocol;

use analysis_storage::stage_analysis;
use dispatch::*;
use protocol::*;

use crate::cad_jobs::{
    CadBodyMesh, CadCollisionMeshPair, CadCollisionQuery, CadMesh, CadOperation, CadReply,
    CadReplyError, CadReplyOutcome, CadRequest, CadResult, CadSnapshotIdentity, CadStepAccuracy,
    CadStepImportOptions, CadStepLineOccurrence, CadVendorStep, MAX_COLLISION_QUERY_BYTES,
    valid_body_meshes, validate_collision_mesh_pair, validate_collision_query,
    validate_completed_reply, validate_triangle_tag_buffers,
};
use crate::cad_qualification::{
    self as qualification, AnalysisMesh, AnalysisRef, Source, SourceQualification,
};
use boardstudio_core_kernel::hash::sha256_hex;
use boardstudio_model::model::{KeycapSpec, PreparedCaseAssemblyIR};
use futures_channel::oneshot;
use js_sys::{
    Array, Float32Array, Float64Array, Function, Object, Promise, Reflect, Uint8Array, Uint32Array,
};
use serde::{Deserialize, Serialize};
use std::{
    cell::RefCell,
    collections::{BTreeMap, BTreeSet, VecDeque},
    fmt,
    rc::Rc,
};
use wasm_bindgen::{JsCast, JsValue, closure::Closure, prelude::wasm_bindgen};
use wasm_bindgen_futures::{JsFuture, spawn_local};
use web_sys::{
    DedicatedWorkerGlobalScope, ErrorEvent, Event, MessageEvent, Worker, WorkerOptions, WorkerType,
};

const MAX_STEP_BYTES: usize = 32 * 1024 * 1024;
const KEYCAPS_PREVIEW_BATCH_SIZE: usize = 8;
// Keep batching within cad/wasm's whole-request keycap limit.
const MAX_KEYCAPS_PER_REQUEST: usize = 4096;
pub const KEYCAP_STEP_UNSUPPORTED: &str =
    "Keycap STEP export is unsupported; build_keycaps returns preview meshes only";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CadWorkerError(pub String);
impl fmt::Display for CadWorkerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for CadWorkerError {}

// Wire envelope types stay `pub` so `boardstudio-web`'s page suite can drive them via
// `test_hooks`, while worker production code routes through private `dispatch`/`protocol` helpers.
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CadEnvelope {
    pub request_id: String,
    pub job_id: String,
    pub identity: CadSnapshotIdentity,
    pub operation: CadOperation,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WireReply {
    #[serde(flatten)]
    pub header: CadEnvelope,
    pub outcome: CadReplyOutcome,
    #[serde(default)]
    pub error: Option<String>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WireRequest<P = PreparedCaseAssemblyIR> {
    #[serde(flatten)]
    pub header: CadEnvelope,
    pub prepared: Option<P>,
    pub source_sha256: Option<String>,
    pub step_import_options: Option<CadStepImportOptions>,
    pub collision_query: Option<CadCollisionQuery>,
    pub generated_analysis_request: Option<crate::cad_jobs::CadGeneratedAnalysisRequest>,
    pub keycaps: Option<KeycapsInput>,
    #[serde(skip)]
    pub collision_meshes: Option<CadCollisionMeshPair>,
    #[serde(skip)]
    pub input_bytes: Vec<u8>,
    pub packaged_step: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct FlatWireRequest {
    request_id: String,
    job_id: String,
    identity: CadSnapshotIdentity,
    operation: CadOperation,
    prepared: Option<boardstudio_model::model::PreparedCaseAssemblyIR>,
    #[serde(default)]
    source_sha256: Option<String>,
    #[serde(default)]
    step_import_options: Option<CadStepImportOptions>,
    #[serde(default)]
    collision_query: Option<CadCollisionQuery>,
    #[serde(default)]
    generated_analysis_request: Option<crate::cad_jobs::CadGeneratedAnalysisRequest>,
    #[serde(default)]
    keycaps: Option<KeycapsInput>,
    #[serde(default)]
    packaged_step: bool,
}

impl From<FlatWireRequest> for WireRequest {
    fn from(raw: FlatWireRequest) -> Self {
        Self {
            header: CadEnvelope {
                request_id: raw.request_id,
                job_id: raw.job_id,
                identity: raw.identity,
                operation: raw.operation,
            },
            prepared: raw.prepared,
            source_sha256: raw.source_sha256,
            step_import_options: raw.step_import_options,
            collision_query: raw.collision_query,
            generated_analysis_request: raw.generated_analysis_request,
            keycaps: raw.keycaps,
            collision_meshes: None,
            input_bytes: Vec::new(),
            packaged_step: raw.packaged_step,
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KeycapsInput {
    revision: u64,
    specs: Vec<KeycapSpec>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct KeycapsCadRequest {
    #[serde(flatten)]
    header: CadEnvelope,
    keycaps: KeycapsInput,
}

struct Pending {
    sender: oneshot::Sender<Result<CadReply, CadWorkerError>>,
}
struct ClientState {
    ready: bool,
    closed: bool,
    ready_waiters: Vec<oneshot::Sender<Result<(), CadWorkerError>>>,
    pending: BTreeMap<String, Pending>,
}

/// Page-side client. Each request is correlated and every worker failure settles all callers.
pub struct CadWorker {
    worker: Worker,
    state: Rc<RefCell<ClientState>>,
    message: Closure<dyn FnMut(MessageEvent)>,
    error: Closure<dyn FnMut(Event)>,
    message_error: Closure<dyn FnMut(Event)>,
}

impl CadWorker {
    pub fn new(url: &str) -> Result<Self, CadWorkerError> {
        let options = WorkerOptions::new();
        options.set_type(WorkerType::Module);
        let worker = Worker::new_with_options(url, &options).map_err(js_error)?;
        let state = Rc::new(RefCell::new(ClientState {
            ready: false,
            closed: false,
            ready_waiters: vec![],
            pending: BTreeMap::new(),
        }));
        let receiving = state.clone();
        let message = Closure::<dyn FnMut(MessageEvent)>::new(move |event: MessageEvent| {
            receive(&receiving, event.data())
        });
        let failed = state.clone();
        let error = Closure::<dyn FnMut(Event)>::new(move |event: Event| {
            let reason = event
                .dyn_ref::<ErrorEvent>()
                .map(|error| error.message())
                .filter(|message| !message.is_empty())
                .unwrap_or_else(|| "CAD worker failed".into());
            fail_all(&failed, CadWorkerError(reason));
        });
        let failed = state.clone();
        let message_error = Closure::<dyn FnMut(Event)>::new(move |_: Event| {
            fail_all(
                &failed,
                CadWorkerError("CAD worker message could not be decoded".into()),
            );
        });
        worker.set_onmessage(Some(message.as_ref().unchecked_ref()));
        worker.set_onerror(Some(error.as_ref().unchecked_ref()));
        worker.set_onmessageerror(Some(message_error.as_ref().unchecked_ref()));
        Ok(Self {
            worker,
            state,
            message,
            error,
            message_error,
        })
    }

    /// Whether this worker has failed or been closed and cannot serve requests.
    pub fn is_closed(&self) -> bool {
        self.state.borrow().closed
    }

    pub async fn ready(&self) -> Result<(), CadWorkerError> {
        let (sender, receiver) = oneshot::channel();
        {
            let mut state = self.state.borrow_mut();
            if state.ready {
                return Ok(());
            }
            if state.closed {
                return Err(CadWorkerError("CAD worker is closed".into()));
            }
            state.ready_waiters.push(sender);
        }
        receiver
            .await
            .map_err(|_| CadWorkerError("CAD worker readiness was abandoned".into()))?
    }

    pub async fn request(
        &self,
        request: impl std::borrow::Borrow<CadRequest>,
    ) -> Result<CadReply, CadWorkerError> {
        self.request_with_step_origin(std::borrow::Borrow::borrow(&request), false)
            .await
    }

    pub async fn request_packaged_step(
        &self,
        request: CadRequest,
    ) -> Result<CadReply, CadWorkerError> {
        self.request_with_step_origin(&request, true).await
    }

    async fn request_with_step_origin(
        &self,
        request: &CadRequest,
        packaged_step: bool,
    ) -> Result<CadReply, CadWorkerError> {
        validate_request(request)?;
        self.ready().await?;
        let wire = WireRequest {
            header: CadEnvelope {
                request_id: request.request_id.clone(),
                job_id: request.job_id.clone(),
                identity: request.identity.clone(),
                operation: request.operation,
            },
            prepared: request.prepared.as_ref(),
            source_sha256: request.source_sha256.clone(),
            step_import_options: request.step_import_options,
            collision_query: request.collision_query.clone(),
            generated_analysis_request: request.generated_analysis_request.clone(),
            keycaps: None,
            collision_meshes: None,
            input_bytes: Vec::new(),
            packaged_step,
        };
        let frame =
            serde_json::to_string(&wire).map_err(|error| CadWorkerError(error.to_string()))?;
        let reply = self
            .request_frame_with_payloads(
                &wire.header.request_id,
                &frame,
                &request.input_bytes,
                request.collision_meshes.as_ref(),
            )
            .await;
        if reply.is_err() {
            let _ = self.cancel(&wire.header.job_id);
        }
        reply
    }

    /// Build preview meshes from Core-resolved KeycapSpecs in this worker.
    /// This remains a feature-specific wire adapter; public case CAD requests stay unchanged.
    pub async fn request_keycaps_preview(
        &self,
        request_id: String,
        job_id: String,
        identity: CadSnapshotIdentity,
        specs: Vec<KeycapSpec>,
    ) -> Result<CadResult, CadWorkerError> {
        self.request_keycaps(request_id, job_id, identity, specs, CadOperation::Preview)
            .await
    }

    async fn request_keycaps(
        &self,
        request_id: String,
        job_id: String,
        identity: CadSnapshotIdentity,
        specs: Vec<KeycapSpec>,
        operation: CadOperation,
    ) -> Result<CadResult, CadWorkerError> {
        let printable = operation == CadOperation::PrintKeycaps;
        let kind = if printable { "print" } else { "preview" };
        if request_id.is_empty()
            || job_id.is_empty()
            || identity.revision > crate::cad_jobs::MAX_CAD_REVISION
            || specs.is_empty()
            || specs.len() > MAX_KEYCAPS_PER_REQUEST
            || (printable
                && (specs.iter().any(|spec| spec.id.trim().is_empty())
                    || specs
                        .iter()
                        .map(|spec| &spec.id)
                        .collect::<BTreeSet<_>>()
                        .len()
                        != specs.len()))
        {
            return Err(CadWorkerError(format!(
                "Keycap {kind} input does not match its accepted snapshot"
            )));
        }
        self.ready().await?;
        let expected = specs
            .iter()
            .flat_map(|spec| {
                let mut ids = vec![format!("keycap:{}", spec.id)];
                if !printable && !spec.legend.trim().is_empty() {
                    ids.push(format!("keycap-legend:{}", spec.id));
                }
                ids
            })
            .collect::<Vec<_>>();
        let revision = identity.revision;
        let frame = serde_json::to_string(&KeycapsCadRequest {
            header: CadEnvelope {
                request_id: request_id.clone(),
                job_id: job_id.clone(),
                identity: identity.clone(),
                operation,
            },
            keycaps: KeycapsInput { revision, specs },
        })
        .map_err(|error| CadWorkerError(error.to_string()))?;
        let reply = self
            .request_frame_with_payloads(&request_id, &frame, &[], None)
            .await?;
        let malformed = || format!("Keycap CAD {kind} meshes are missing, malformed, or stale");
        let result = validate_completed_reply(reply, &request_id, &job_id, &identity, operation)
            .map_err(|error| {
                CadWorkerError(match error {
                    CadReplyError::WrongEnvelope => {
                        format!("CAD {kind} result belongs to another keycap request")
                    }
                    CadReplyError::WrongRevision => malformed(),
                    CadReplyError::Cancelled => format!("Keycap CAD {kind} was cancelled"),
                    CadReplyError::Failed(reason) => {
                        reason.unwrap_or_else(|| format!("Keycap CAD {kind} failed"))
                    }
                    CadReplyError::MissingResult => "CAD worker returned no keycap meshes".into(),
                })
            })?;
        if (printable && !result.step.is_empty())
            || !valid_body_meshes(
                &result.bodies,
                expected.iter().map(String::as_str),
                if printable { 9 } else { 3 },
            )
            || result.bodies.iter().any(|body| body.name.is_empty())
        {
            return Err(CadWorkerError(malformed()));
        }
        Ok(result)
    }

    pub async fn request_keycaps_print(
        &self,
        request_id: String,
        job_id: String,
        identity: CadSnapshotIdentity,
        specs: Vec<KeycapSpec>,
    ) -> Result<CadResult, CadWorkerError> {
        self.request_keycaps(
            request_id,
            job_id,
            identity,
            specs,
            CadOperation::PrintKeycaps,
        )
        .await
    }

    async fn request_frame_with_payloads(
        &self,
        request_id: &str,
        frame: &str,
        input_bytes: &[u8],
        collision_meshes: Option<&CadCollisionMeshPair>,
    ) -> Result<CadReply, CadWorkerError> {
        if self.state.borrow().closed {
            return Err(CadWorkerError("CAD worker is closed".into()));
        }
        let object = Object::new();
        Reflect::set(&object, &"kind".into(), &"request".into()).map_err(js_error)?;
        Reflect::set(&object, &"frame".into(), &JsValue::from_str(frame)).map_err(js_error)?;
        let transfers = Array::new();
        if !input_bytes.is_empty() {
            let bytes = Uint8Array::from(input_bytes);
            let buffer = bytes.buffer();
            Reflect::set(&object, &"inputBytes".into(), &buffer).map_err(js_error)?;
            transfers.push(&buffer);
        }
        if let Some(meshes) = collision_meshes {
            for (field, _, buffer) in collision_mesh_buffers(meshes) {
                Reflect::set(&object, &field.into(), &buffer).map_err(js_error)?;
                transfers.push(&buffer);
            }
        }
        let (sender, receiver) = oneshot::channel();
        {
            let mut state = self.state.borrow_mut();
            if state.closed {
                return Err(CadWorkerError("CAD worker is closed".into()));
            }
            if state.pending.contains_key(request_id) {
                return Err(CadWorkerError("duplicate in-flight CAD request id".into()));
            }
            state
                .pending
                .insert(request_id.to_owned(), Pending { sender });
        }
        let posted = if transfers.length() == 0 {
            self.worker.post_message(&object)
        } else {
            self.worker.post_message_with_transfer(&object, &transfers)
        };
        if let Err(error) = posted
            && let Some(pending) = self.state.borrow_mut().pending.remove(request_id)
        {
            let _ = pending.sender.send(Err(js_error(error)));
        }
        receiver
            .await
            .map_err(|_| CadWorkerError("CAD worker reply caller was abandoned".into()))?
    }

    pub fn cancel(&self, job_id: &str) -> Result<(), CadWorkerError> {
        if self.state.borrow().closed {
            return Ok(());
        }
        let object = Object::new();
        Reflect::set(&object, &"kind".into(), &"cancel".into()).map_err(js_error)?;
        Reflect::set(&object, &"jobId".into(), &JsValue::from_str(job_id)).map_err(js_error)?;
        self.worker.post_message(&object).map_err(js_error)
    }

    pub fn close(&self) {
        self.worker.terminate();
        fail_all(
            &self.state,
            CadWorkerError("CAD worker closed; pending jobs were cancelled".into()),
        );
    }
}

impl Drop for CadWorker {
    fn drop(&mut self) {
        self.worker.terminate();
        fail_all(
            &self.state,
            CadWorkerError("CAD worker client dropped".into()),
        );
        // Closures stay installed until Worker termination; fields are then dropped together.
        let _ = (&self.message, &self.error, &self.message_error);
    }
}

fn validate_request(request: &CadRequest) -> Result<(), CadWorkerError> {
    if request.operation == CadOperation::AnalyzeGeneratedBody {
        if !request.input_bytes.is_empty()
            || request.source_sha256.is_some()
            || request.step_import_options.is_some()
            || request.collision_query.is_some()
            || request.collision_meshes.is_some()
        {
            return Err(CadWorkerError(
                "generated analysis cannot include STEP or collision payloads".into(),
            ));
        }
        let metadata = request
            .generated_analysis_request
            .as_ref()
            .ok_or_else(|| CadWorkerError("generated analysis omitted request metadata".into()))?;
        crate::cad_jobs::validate_generated_analysis_request(
            request.prepared.as_ref(),
            request.identity.revision,
            metadata,
        )
        .map_err(CadWorkerError)?;
    } else if request.generated_analysis_request.is_some() {
        return Err(CadWorkerError(
            "unexpected generated analysis metadata".into(),
        ));
    }
    if request.request_id.is_empty() || request.job_id.is_empty() {
        return Err(CadWorkerError(
            "CAD request and job identities must not be empty".into(),
        ));
    }
    if request.identity.revision > crate::cad_jobs::MAX_CAD_REVISION {
        return Err(CadWorkerError(
            "CAD revision is outside the exact JavaScript integer range".into(),
        ));
    }
    match request.operation {
        CadOperation::PrintKeycaps => Err(CadWorkerError(
            "Printable keycaps require the keycap request adapter".into(),
        )),
        CadOperation::ReadStep
            if request.prepared.is_some()
                || request.collision_query.is_some()
                || request.collision_meshes.is_some()
                || request.input_bytes.is_empty()
                || request.input_bytes.len() > MAX_STEP_BYTES =>
        {
            Err(CadWorkerError(
                "STEP import requires a non-empty file no larger than 32 MiB".into(),
            ))
        }
        CadOperation::ReadStep
            if !request
                .source_sha256
                .as_deref()
                .is_some_and(crate::cad_jobs::is_sha256_hex) =>
        {
            Err(CadWorkerError(
                "STEP import requires a canonical source SHA-256".into(),
            ))
        }
        CadOperation::ReadStep => request
            .step_import_options
            .unwrap_or_default()
            .validate()
            .map(|_| ())
            .map_err(CadWorkerError),
        CadOperation::CollisionQuery => {
            if request.prepared.is_some()
                || !request.input_bytes.is_empty()
                || request.source_sha256.is_some()
                || request.step_import_options.is_some()
            {
                return Err(CadWorkerError(
                    "collision query cannot include case or STEP byte payloads".into(),
                ));
            }
            let query = request
                .collision_query
                .as_ref()
                .ok_or_else(|| CadWorkerError("collision query omitted metadata".into()))?;
            validate_collision_query(query).map_err(CadWorkerError)?;
            let meshes = request
                .collision_meshes
                .as_ref()
                .ok_or_else(|| CadWorkerError("collision query omitted mesh buffers".into()))?;
            validate_collision_mesh_pair(meshes).map_err(CadWorkerError)
        }
        _ if request.prepared.as_ref().is_none_or(|prepared| {
            prepared.revision != request.identity.revision
                || prepared
                    .bodies
                    .iter()
                    .any(|body| body.revision != request.identity.revision)
        }) =>
        {
            Err(CadWorkerError(
                "prepared CAD input does not match captured revision".into(),
            ))
        }
        _ if !request.input_bytes.is_empty() => Err(CadWorkerError(
            "unexpected byte payload for case CAD request".into(),
        )),
        _ => Ok(()),
    }
}

fn receive(state: &Rc<RefCell<ClientState>>, data: JsValue) {
    let kind = Reflect::get(&data, &"kind".into())
        .ok()
        .and_then(|value| value.as_string());
    match kind.as_deref() {
        Some("ready") => {
            let mut state = state.borrow_mut();
            if state.closed {
                return;
            }
            state.ready = true;
            for waiter in state.ready_waiters.drain(..) {
                let _ = waiter.send(Ok(()));
            }
        }
        Some("init-failed") => {
            let reason = Reflect::get(&data, &"frame".into())
                .ok()
                .and_then(|value| value.as_string())
                .unwrap_or_else(|| "CAD worker initialization failed".into());
            fail_all(state, CadWorkerError(reason));
        }
        Some("reply") => {
            let Some(encoded) = Reflect::get(&data, &"frame".into())
                .ok()
                .and_then(|value| value.as_string())
            else {
                fail_all(state, CadWorkerError("CAD reply omitted metadata".into()));
                return;
            };
            let wire = match serde_json::from_str::<WireReply>(&encoded) {
                Ok(wire) => wire,
                Err(error) => {
                    fail_all(
                        state,
                        CadWorkerError(format!("invalid CAD reply metadata: {error}")),
                    );
                    return;
                }
            };
            let sender = state
                .borrow_mut()
                .pending
                .remove(&wire.header.request_id)
                .map(|pending| pending.sender);
            if let Some(sender) = sender {
                let result = if wire.outcome == CadReplyOutcome::Completed {
                    decode_result(
                        Reflect::get(&data, &"result".into()).unwrap_or(JsValue::UNDEFINED),
                    )
                    .map(Some)
                    .map_err(|error| CadWorkerError(format!("invalid CAD result: {error}")))
                } else {
                    Ok(None)
                };
                let reply = result.map(|result| CadReply {
                    request_id: wire.header.request_id,
                    job_id: wire.header.job_id,
                    identity: wire.header.identity,
                    operation: wire.header.operation,
                    outcome: wire.outcome,
                    result,
                    error: wire.error,
                });
                let _ = sender.send(reply);
            }
        }
        _ => fail_all(
            state,
            CadWorkerError("CAD worker sent an unknown frame kind".into()),
        ),
    }
}

fn fail_all(state: &Rc<RefCell<ClientState>>, error: CadWorkerError) {
    let mut state = state.borrow_mut();
    if state.closed {
        return;
    }
    state.closed = true;
    for waiter in state.ready_waiters.drain(..) {
        let _ = waiter.send(Err(error.clone()));
    }
    for (_, pending) in std::mem::take(&mut state.pending) {
        let _ = pending.sender.send(Err(error.clone()));
    }
}

// Test seam for the page suite's CAD worker tests: re-exports the wire envelope,
// queue, and validation helpers the migrated browser tests drive.
// The re-exported items live in the private `dispatch`/`protocol` modules, so
// they stay crate-internal unless the page suite reaches them through here.
#[cfg(any(test, feature = "test-support"))]
pub mod test_hooks {
    pub use super::dispatch::{run_request, yield_to_worker};
    pub use super::protocol::{
        decode_result, strict_metadata, strict_step_accuracy, validate_analysis_buffers,
        validate_generated_payload_fields,
    };
    pub use super::{
        CadEnvelope, KEYCAP_STEP_UNSUPPORTED, OwnedAnalysis, WireReply, WireRequest, WorkerQueue,
        js_message, keycap_operation_error, post_owned_result, receive_worker_message,
        result_transfers, worker_message,
    };
}

fn js_error(error: JsValue) -> CadWorkerError {
    CadWorkerError(js_message_or(error, "JavaScript CAD worker failure"))
}

#[wasm_bindgen]
pub fn start_cad_worker(
    cad_module_url: String,
    load_baked_step: Option<Function>,
) -> Result<(), JsValue> {
    let scope: DedicatedWorkerGlobalScope = js_sys::global().dyn_into()?;
    let target = scope.clone();
    spawn_local(async move {
        let module = match initialize_cad_module(&cad_module_url).await {
            Ok(module) => Rc::new(module),
            Err(error) => {
                let _ = target.post_message(&worker_message("init-failed", &error));
                return;
            }
        };
        let queue = Rc::new(RefCell::new(WorkerQueue {
            load_baked_step,
            ..Default::default()
        }));
        let receiving = queue.clone();
        let worker_scope = target.clone();
        let worker_module = module.clone();
        let callback = Closure::<dyn FnMut(MessageEvent)>::new(move |event: MessageEvent| {
            if let Err(error) =
                receive_worker_message(&receiving, &worker_scope, &worker_module, event.data())
            {
                wasm_bindgen::throw_val(error);
            }
        });
        target.set_onmessage(Some(callback.as_ref().unchecked_ref()));
        callback.forget();
        let _ = target.post_message(&worker_message("ready", ""));
    });
    Ok(())
}

async fn initialize_cad_module(url: &str) -> Result<JsValue, String> {
    let importer = Function::new_with_args("url", "return import(url)");
    let imported = importer
        .call1(&JsValue::NULL, &JsValue::from_str(url))
        .map_err(js_message)?;
    let module = JsFuture::from(imported.dyn_into::<Promise>().map_err(js_message)?)
        .await
        .map_err(js_message)?;
    let init = Reflect::get(&module, &"default".into())
        .map_err(js_message)?
        .dyn_into::<Function>()
        .map_err(js_message)?;
    let initialized = init.call0(&JsValue::NULL).map_err(js_message)?;
    JsFuture::from(initialized.dyn_into::<Promise>().map_err(js_message)?)
        .await
        .map_err(js_message)?;
    Ok(module)
}

pub struct WorkerQueue {
    load_baked_step: Option<Function>,
    pub pending: VecDeque<(WireRequest, Vec<u8>)>,
    pub cancelled: BTreeSet<String>,
    pub active: bool,
    pub active_job: Option<String>,
    pub active_request: Option<String>,
    pub generation: String,
    pub evidence: BTreeMap<String, Rc<OwnedAnalysis>>,
    evidence_recency: VecDeque<String>,
    pub retained_bytes: usize,
    pub latest_snapshot: Option<CadSnapshotIdentity>,
}

impl Default for WorkerQueue {
    fn default() -> Self {
        let generation = Function::new_no_args("return crypto.randomUUID()")
            .call0(&JsValue::NULL)
            .expect("worker UUID source")
            .as_string()
            .expect("worker UUID");
        Self {
            load_baked_step: None,
            pending: VecDeque::new(),
            cancelled: BTreeSet::new(),
            active: false,
            active_job: None,
            active_request: None,
            generation,
            evidence: BTreeMap::new(),
            evidence_recency: VecDeque::new(),
            retained_bytes: 0,
            latest_snapshot: None,
        }
    }
}

struct ProviderHandle(JsValue);
impl Drop for ProviderHandle {
    fn drop(&mut self) {
        if let Ok(free) = Reflect::get(&self.0, &"free".into())
            && let Some(free) = free.dyn_ref::<Function>()
        {
            let _ = free.call0(&self.0);
        }
    }
}

pub struct OwnedAnalysis {
    handle: ProviderHandle,
    pub reference: AnalysisRef,
    descriptor: AnalysisMesh,
    source: SourceQualification,
    scope: CadSnapshotIdentity,
    task: String,
    job: String,
    retained_bytes: Option<usize>,
    // Transport comparison only. Proof remains exclusively in the opaque owner.
    positions: Vec<f64>,
    tags: Vec<u32>,
}

fn same_scope(a: &CadSnapshotIdentity, b: &CadSnapshotIdentity) -> bool {
    a.session_epoch == b.session_epoch
        && a.document_id == b.document_id
        && a.board_id == b.board_id
        && a.instance_id == b.instance_id
}

impl WorkerQueue {
    pub fn cancel_job(&mut self, job: &str) {
        if self.active_job.as_deref() == Some(job)
            || self
                .pending
                .iter()
                .any(|(request, _)| request.header.job_id == job)
        {
            self.cancelled.insert(job.into());
        }
        let keys = self
            .evidence
            .iter()
            .filter(|(_, entry)| entry.job == job)
            .map(|(key, _)| key.clone())
            .collect::<Vec<_>>();
        for key in keys {
            self.remove_evidence(&key);
        }
    }
    pub fn remove_evidence(&mut self, identity: &str) {
        if let Some(entry) = self.evidence.remove(identity) {
            self.retained_bytes -= entry.retained_bytes.unwrap_or(0);
        }
        self.evidence_recency.retain(|key| key != identity);
    }

    fn remove_task_evidence(&mut self, task: &str) {
        let keys = self
            .evidence
            .iter()
            .filter(|(_, e)| e.task == task)
            .map(|(key, _)| key.clone())
            .collect::<Vec<_>>();
        for key in keys {
            self.remove_evidence(&key);
        }
    }

    fn retain_analysis(&mut self, entry: Rc<OwnedAnalysis>) -> Result<(), String> {
        let Some(cost) = entry
            .retained_bytes
            .filter(|&n| n <= MAX_COLLISION_QUERY_BYTES)
        else {
            return Ok(());
        };
        let key = entry.reference.mesh_identity.clone();
        if self.evidence.contains_key(&key) {
            return Err("provider reused an owned mesh identity".into());
        }
        while self.evidence.len() >= 80
            || self.retained_bytes.saturating_add(cost) > MAX_COLLISION_QUERY_BYTES
        {
            let Some(victim) = self.evidence_recency.front().cloned() else {
                break;
            };
            self.remove_evidence(&victim);
        }
        self.retained_bytes += cost;
        self.evidence_recency.push_back(key.clone());
        self.evidence.insert(key, entry);
        Ok(())
    }

    pub fn lookup(
        &mut self,
        operand: &crate::cad_jobs::CadCollisionOperand,
        scope: &CadSnapshotIdentity,
        positions: &[f64],
        tags: &[u32],
    ) -> Result<Option<Rc<OwnedAnalysis>>, String> {
        qualification::validate_operand(operand)?;
        let reference = operand.analysis_ref.as_ref().ok_or("missing reference")?;
        if reference.worker_generation != self.generation {
            return Ok(None);
        }
        let Some(entry) = self.evidence.get(&reference.mesh_identity).cloned() else {
            return Ok(None);
        };
        if !same_scope(scope, &entry.scope)
            || &entry.reference != reference
            || serde_json::to_vec(&operand.analysis_mesh).ok()
                != serde_json::to_vec(&Some(&entry.descriptor)).ok()
            || entry.positions.len() != positions.len()
            || entry.tags != tags
            || entry
                .positions
                .iter()
                .zip(positions)
                .any(|(a, b)| a.to_bits() != b.to_bits())
        {
            return Err("known analysis reference has mismatched scope/source/descriptor".into());
        }
        self.evidence_recency
            .retain(|key| key != &reference.mesh_identity);
        self.evidence_recency
            .push_back(reference.mesh_identity.clone());
        Ok(Some(entry))
    }
}

pub fn receive_worker_message(
    queue: &Rc<RefCell<WorkerQueue>>,
    scope: &DedicatedWorkerGlobalScope,
    module: &Rc<JsValue>,
    data: JsValue,
) -> Result<(), JsValue> {
    let kind = Reflect::get(&data, &"kind".into())?
        .as_string()
        .unwrap_or_default();
    if kind == "cancel" {
        if let Some(job_id) = Reflect::get(&data, &"jobId".into())?.as_string() {
            queue.borrow_mut().cancel_job(&job_id);
        }
        return Ok(());
    }
    if kind != "request" {
        return Err(JsValue::from_str("unknown CAD worker message kind"));
    }
    let frame = Reflect::get(&data, &"frame".into())?
        .as_string()
        .ok_or_else(|| JsValue::from_str("CAD request omitted frame"))?;
    let mut request: WireRequest = match serde_json::from_str::<FlatWireRequest>(&frame) {
        Ok(request) => request.into(),
        Err(error) => {
            let header: CadEnvelope =
                serde_json::from_str(&frame).map_err(|e| JsValue::from_str(&e.to_string()))?;
            return post_reply(
                scope,
                WireReply {
                    header,
                    outcome: CadReplyOutcome::Failed,
                    error: Some(error.to_string()),
                },
                None,
                Array::new(),
            );
        }
    };
    if let Some(message) =
        keycap_operation_error(&request.header.operation, request.keycaps.is_some())
    {
        return post_reply(
            scope,
            WireReply {
                header: request.header,
                outcome: CadReplyOutcome::Failed,
                error: Some(message.into()),
            },
            None,
            Array::new(),
        );
    }
    if let Some(keycaps) = request.keycaps.as_ref()
        && (request.prepared.is_some()
            || keycaps.revision != request.header.identity.revision
            || keycaps.specs.is_empty()
            || keycaps.specs.len() > MAX_KEYCAPS_PER_REQUEST)
    {
        return Err(JsValue::from_str(
            "Keycap CAD input does not match its captured revision",
        ));
    }
    if request.header.identity.revision > crate::cad_jobs::MAX_CAD_REVISION {
        return post_reply(
            scope,
            WireReply {
                header: request.header,
                outcome: CadReplyOutcome::Failed,
                error: Some("CAD revision is outside the exact JavaScript integer range".into()),
            },
            None,
            Array::new(),
        );
    }
    if let Some(buffer) = Reflect::get(&data, &"inputBytes".into())
        .ok()
        .filter(|value| !value.is_undefined())
    {
        request.input_bytes = Uint8Array::new(&buffer).to_vec();
    }
    if request.header.operation == CadOperation::CollisionQuery {
        match decode_query_buffers(&data) {
            Ok(meshes) => request.collision_meshes = Some(meshes),
            Err(error) => {
                return post_reply(
                    scope,
                    WireReply {
                        header: request.header,
                        outcome: CadReplyOutcome::Failed,
                        error: Some(error),
                    },
                    None,
                    Array::new(),
                );
            }
        }
    }
    let input_bytes = std::mem::take(&mut request.input_bytes);
    {
        let mut queue = queue.borrow_mut();
        if queue
            .pending
            .iter()
            .any(|(item, _)| item.header.request_id == request.header.request_id)
            || queue.active_request.as_ref() == Some(&request.header.request_id)
        {
            return Err(JsValue::from_str("duplicate in-flight CAD request id"));
        }
        if queue
            .latest_snapshot
            .as_ref()
            .is_some_and(|old| !same_scope(old, &request.header.identity))
        {
            queue.evidence.clear();
            queue.evidence_recency.clear();
            queue.retained_bytes = 0;
        }
        queue.latest_snapshot = Some(request.header.identity.clone());
        queue.pending.push_back((request, input_bytes));
    }
    pump_worker(queue.clone(), scope.clone(), module.clone());
    Ok(())
}

pub fn keycap_operation_error(operation: &CadOperation, has_keycaps: bool) -> Option<&'static str> {
    if !has_keycaps {
        return (operation == &CadOperation::PrintKeycaps)
            .then_some("Printable keycaps require keycap input");
    }
    match operation {
        CadOperation::Preview | CadOperation::PrintKeycaps => None,
        CadOperation::ExportStep => Some(KEYCAP_STEP_UNSUPPORTED),
        CadOperation::Exact
        | CadOperation::ReadStep
        | CadOperation::CollisionQuery
        | CadOperation::AnalyzeGeneratedBody => Some("Keycap CAD input is only valid for preview"),
    }
}

fn decode_query_buffers(data: &JsValue) -> Result<CadCollisionMeshPair, String> {
    let mut buffers = Vec::new();
    let mut bytes = 0_usize;
    for (name, width) in [
        ("leftPositions", 8),
        ("leftTags", 4),
        ("rightPositions", 8),
        ("rightTags", 4),
    ] {
        let value = Reflect::get(data, &name.into()).map_err(js_message)?;
        let buffer = value
            .dyn_into::<js_sys::ArrayBuffer>()
            .map_err(|_| format!("{name} requires an owned ArrayBuffer"))?;
        let length = buffer.byte_length() as usize;
        if length == 0 || !length.is_multiple_of(width) {
            return Err(format!("{name} is detached, empty or misaligned"));
        }
        bytes = bytes.saturating_add(length);
        buffers.push(buffer);
    }
    if bytes > MAX_COLLISION_QUERY_BYTES {
        return Err("collision pair exceeds 64 MiB".into());
    }
    let meshes = CadCollisionMeshPair {
        left_positions_mm_f64: Float64Array::new(&buffers[0]).to_vec(),
        left_triangle_face_occurrence_indices: Uint32Array::new(&buffers[1]).to_vec(),
        right_positions_mm_f64: Float64Array::new(&buffers[2]).to_vec(),
        right_triangle_face_occurrence_indices: Uint32Array::new(&buffers[3]).to_vec(),
    };
    validate_collision_mesh_pair(&meshes)?;
    Ok(meshes)
}

fn pump_worker(
    queue: Rc<RefCell<WorkerQueue>>,
    scope: DedicatedWorkerGlobalScope,
    module: Rc<JsValue>,
) {
    let request = {
        let mut queue = queue.borrow_mut();
        if queue.active {
            return;
        }
        match queue.pending.pop_front() {
            Some(request) => {
                queue.active = true;
                queue.active_job = Some(request.0.header.job_id.clone());
                queue.active_request = Some(request.0.header.request_id.clone());
                request
            }
            None => return,
        }
    };
    let next_queue = queue.clone();
    let next_scope = scope.clone();
    let next_module = module.clone();
    spawn_local(async move {
        let (request, bytes) = request;
        let completed_job_id = request.header.job_id.clone();
        let completed_request_id = request.header.request_id.clone();
        let reply = run_request(&queue, &module, request, bytes).await;
        match reply {
            Ok((wire, result)) => {
                let _ = post_owned_result(&queue, &scope, wire, result);
            }
            Err((header, reason, cancelled)) => {
                queue
                    .borrow_mut()
                    .remove_task_evidence(&completed_request_id);
                let metadata = WireReply {
                    header,
                    outcome: if cancelled {
                        CadReplyOutcome::Cancelled
                    } else {
                        CadReplyOutcome::Failed
                    },
                    error: Some(reason),
                };
                let _ = post_reply(&scope, metadata, None, Array::new());
            }
        }
        next_queue.borrow_mut().active = false;
        next_queue.borrow_mut().active_job = None;
        next_queue.borrow_mut().active_request = None;
        next_queue.borrow_mut().cancelled.remove(&completed_job_id);
        pump_worker(next_queue, next_scope, next_module);
    });
}

fn post_result(
    scope: &DedicatedWorkerGlobalScope,
    wire: WireReply,
    result: JsValue,
) -> Result<(), JsValue> {
    let transfers = result_transfers(&wire.header.operation, &result)?;
    post_reply(scope, wire, Some(result), transfers)
}

pub fn post_owned_result(
    queue: &Rc<RefCell<WorkerQueue>>,
    scope: &DedicatedWorkerGlobalScope,
    wire: WireReply,
    result: JsValue,
) -> Result<(), JsValue> {
    if let Err(error) = post_result(scope, wire.clone(), result) {
        queue
            .borrow_mut()
            .remove_task_evidence(&wire.header.request_id);
        return post_reply(
            scope,
            WireReply {
                outcome: CadReplyOutcome::Failed,
                error: Some(js_message(error)),
                ..wire
            },
            None,
            Array::new(),
        );
    }
    Ok(())
}

fn post_reply(
    scope: &DedicatedWorkerGlobalScope,
    wire: WireReply,
    result: Option<JsValue>,
    transfers: Array,
) -> Result<(), JsValue> {
    let metadata =
        serde_json::to_string(&wire).map_err(|error| JsValue::from_str(&error.to_string()))?;
    let message = Object::new();
    Reflect::set(&message, &"kind".into(), &"reply".into())?;
    Reflect::set(&message, &"frame".into(), &JsValue::from_str(&metadata))?;
    if let Some(result) = result {
        Reflect::set(&message, &"result".into(), &result)?;
    }
    if transfers.length() == 0 {
        scope.post_message(&message)
    } else {
        scope.post_message_with_transfer(&message, &transfers)
    }
}

pub fn result_transfers(operation: &CadOperation, result: &JsValue) -> Result<Array, JsValue> {
    let transfers = Array::new();
    match operation {
        CadOperation::AnalyzeGeneratedBody => {
            let analysis = Reflect::get(result, &"generatedAnalysis".into())?;
            for field in ["surfacePositionsMmF64", "triangleFaceOccurrenceIndices"] {
                add_buffer_transfer(&transfers, &Reflect::get(&analysis, &field.into())?)?;
            }
        }
        CadOperation::Preview | CadOperation::PrintKeycaps => {
            let bodies = Array::from(&Reflect::get(result, &"bodies".into())?);
            for body in bodies.iter() {
                add_mesh_transfers(&transfers, &body)?;
            }
        }
        CadOperation::Exact => {
            add_buffer_transfer(&transfers, &Reflect::get(result, &"step".into())?)?;
            add_mesh_transfers(&transfers, &Reflect::get(result, &"mesh".into())?)?;
            let bodies = Array::from(&Reflect::get(result, &"bodies".into())?);
            for body in bodies.iter() {
                add_mesh_transfers(&transfers, &body)?;
            }
        }
        CadOperation::ExportStep => {
            add_buffer_transfer(&transfers, &Reflect::get(result, &"step".into())?)?
        }
        CadOperation::ReadStep => {
            add_mesh_transfers(&transfers, &Reflect::get(result, &"mesh".into())?)?;
            let vendor_step = Reflect::get(result, &"vendorStep".into())?;
            if !vendor_step.is_null() && !vendor_step.is_undefined() {
                for field in [
                    "surfacePositionsMmF64",
                    "triangleFaceOccurrenceIndices",
                    "lineSegmentsMmF64",
                    "lineRenderPositions",
                    "segmentLineOccurrenceIndices",
                ] {
                    let value = Reflect::get(&vendor_step, &field.into())?;
                    add_buffer_transfer(&transfers, &value)?;
                }
                let occurrences = Reflect::get(&vendor_step, &"lineOccurrences".into())?;
                if Array::is_array(&occurrences) {
                    for item in Array::from(&occurrences).iter() {
                        let placement = Reflect::get(&item, &"placementF64".into())?;
                        add_buffer_transfer(&transfers, &placement)?;
                    }
                }
            }
        }
        CadOperation::CollisionQuery => {}
    }
    Ok(transfers)
}

fn add_mesh_transfers(transfers: &Array, mesh: &JsValue) -> Result<(), JsValue> {
    for field in ["positions", "normals"] {
        let value = Reflect::get(mesh, &field.into())?;
        add_buffer_transfer(transfers, &value)?;
    }
    Ok(())
}
fn add_buffer_transfer(transfers: &Array, value: &JsValue) -> Result<(), JsValue> {
    let buffer = if value.is_instance_of::<Uint8Array>() {
        value.unchecked_ref::<Uint8Array>().buffer()
    } else if value.is_instance_of::<Float64Array>() {
        value.unchecked_ref::<Float64Array>().buffer()
    } else if value.is_instance_of::<Uint32Array>() {
        value.unchecked_ref::<Uint32Array>().buffer()
    } else if value.is_instance_of::<Float32Array>() {
        value.unchecked_ref::<Float32Array>().buffer()
    } else {
        return Err(JsValue::from_str(
            "CAD result transfer requires a typed array",
        ));
    };
    transfers.push(&buffer);
    Ok(())
}

pub fn worker_message(kind: &str, message: &str) -> JsValue {
    let value = Object::new();
    let _ = Reflect::set(&value, &"kind".into(), &JsValue::from_str(kind));
    let _ = Reflect::set(&value, &"frame".into(), &JsValue::from_str(message));
    value.into()
}
pub fn js_message(value: JsValue) -> String {
    js_message_or(value, "JavaScript CAD provider failed")
}

fn js_message_or(value: JsValue, fallback: &str) -> String {
    value.as_string().unwrap_or_else(|| fallback.into())
}
