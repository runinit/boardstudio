//! Dedicated host for the separately built Monstertruck CAD WASM package.
use crate::cad_jobs::{
    CadBodyMesh, CadCollisionMeshPair, CadCollisionObservation, CadCollisionQuery, CadMesh,
    CadOperation, CadReply, CadReplyOutcome, CadRequest, CadResult, CadSnapshotIdentity,
    CadStepAccuracy, CadStepFaceOccurrence, CadStepImportOptions, CadStepLineOccurrence,
    CadStepPrecision, CadStepSourceFace, CadVendorStep, MAX_COLLISION_QUERY_BYTES,
    validate_collision_mesh_pair, validate_collision_query,
};
use crate::cad_qualification::{
    self as qualification, AnalysisMesh, AnalysisRef, Source, SourceQualification,
};
use boardstudio_core::hash::sha256_hex;
use boardstudio_core::model::KeycapSpec;
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
const KEYCAP_STEP_UNSUPPORTED: &str =
    "Keycap STEP export is unsupported; build_keycaps returns preview meshes only";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CadWorkerError(pub String);
impl fmt::Display for CadWorkerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for CadWorkerError {}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct WireReply {
    request_id: String,
    job_id: String,
    identity: CadSnapshotIdentity,
    operation: CadOperation,
    outcome: CadReplyOutcome,
    #[serde(default)]
    error: Option<String>,
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct WireRequest {
    request_id: String,
    job_id: String,
    identity: CadSnapshotIdentity,
    operation: CadOperation,
    prepared: Option<boardstudio_core::model::PreparedCaseAssemblyIR>,
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
    #[serde(skip)]
    collision_meshes: Option<CadCollisionMeshPair>,
    #[serde(skip)]
    input_bytes: Vec<u8>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RequestEnvelope {
    request_id: String,
    job_id: String,
    identity: CadSnapshotIdentity,
    operation: CadOperation,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct KeycapsInput {
    revision: u64,
    specs: Vec<KeycapSpec>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct KeycapsCadRequest {
    request_id: String,
    job_id: String,
    identity: CadSnapshotIdentity,
    operation: CadOperation,
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

    pub async fn request(&self, request: CadRequest) -> Result<CadReply, CadWorkerError> {
        validate_request(&request)?;
        self.ready().await?;
        let frame =
            serde_json::to_string(&request).map_err(|error| CadWorkerError(error.to_string()))?;
        let reply = self
            .request_frame_with_payloads(
                &request.request_id,
                &frame,
                &request.input_bytes,
                request.collision_meshes.as_ref(),
            )
            .await;
        if reply.is_err() {
            let _ = self.cancel(&request.job_id);
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
            request_id: request_id.clone(),
            job_id: job_id.clone(),
            identity: identity.clone(),
            operation,
            keycaps: KeycapsInput { revision, specs },
        })
        .map_err(|error| CadWorkerError(error.to_string()))?;
        let reply = self.request_frame(&request_id, &frame, &[]).await?;
        if reply.request_id != request_id
            || reply.job_id != job_id
            || reply.identity != identity
            || reply.operation != operation
        {
            return Err(CadWorkerError(format!(
                "CAD {kind} result belongs to another keycap request"
            )));
        }
        match reply.outcome {
            CadReplyOutcome::Cancelled => {
                return Err(CadWorkerError(format!("Keycap CAD {kind} was cancelled")));
            }
            CadReplyOutcome::Failed => {
                return Err(CadWorkerError(
                    reply
                        .error
                        .unwrap_or_else(|| format!("Keycap CAD {kind} failed")),
                ));
            }
            CadReplyOutcome::Completed => {}
        }
        let result = reply
            .result
            .ok_or_else(|| CadWorkerError("CAD worker returned no keycap meshes".into()))?;
        let valid_mesh = |body: &CadBodyMesh| {
            !body.id.is_empty()
                && !body.name.is_empty()
                && !body.positions.is_empty()
                && body
                    .positions
                    .len()
                    .is_multiple_of(if printable { 9 } else { 3 })
                && body.positions.len() == body.normals.len()
                && body
                    .positions
                    .iter()
                    .chain(&body.normals)
                    .all(|value| value.is_finite())
        };
        if result.revision != identity.revision
            || (printable && !result.step.is_empty())
            || result.bodies.len() != expected.len()
            || result
                .bodies
                .iter()
                .zip(expected)
                .any(|(body, expected_id)| body.id != expected_id || !valid_mesh(body))
        {
            return Err(CadWorkerError(format!(
                "Keycap CAD {kind} meshes are missing, malformed, or stale"
            )));
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

    async fn request_frame(
        &self,
        request_id: &str,
        frame: &str,
        input_bytes: &[u8],
    ) -> Result<CadReply, CadWorkerError> {
        self.request_frame_with_payloads(request_id, frame, input_bytes, None)
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
            let left_pos = Float64Array::from(meshes.left_positions_mm_f64.as_slice());
            let left_tags =
                Uint32Array::from(meshes.left_triangle_face_occurrence_indices.as_slice());
            let right_pos = Float64Array::from(meshes.right_positions_mm_f64.as_slice());
            let right_tags =
                Uint32Array::from(meshes.right_triangle_face_occurrence_indices.as_slice());
            for (field, buffer) in [
                ("leftPositions", left_pos.buffer()),
                ("leftTags", left_tags.buffer()),
                ("rightPositions", right_pos.buffer()),
                ("rightTags", right_tags.buffer()),
            ] {
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

/// Browser-only test seam for exercising the public worker request path with a mock Worker.
#[cfg(feature = "test-harness")]
#[wasm_bindgen]
pub async fn test_cad_worker_request(url: String) -> Result<u32, JsValue> {
    let worker = CadWorker::new(&url).map_err(|error| JsValue::from_str(&error.to_string()))?;
    let reply = worker
        .request(CadRequest {
            request_id: "decoder-regression-request".into(),
            job_id: "decoder-regression-job".into(),
            identity: CadSnapshotIdentity {
                token: 1,
                session_epoch: 1,
                document_id: "decoder-regression-document".into(),
                board_id: "decoder-regression-board".into(),
                instance_id: None,
                revision: 1,
            },
            operation: CadOperation::ReadStep,
            prepared: None,
            source_sha256: Some(
                "4bf5122f344554c53bde2ebb8cd2b7e3d1600ad631c385a5d7cce23c7785459a".into(),
            ),
            step_import_options: None,
            collision_query: None,
            generated_analysis_request: None,
            collision_meshes: None,
            input_bytes: vec![1],
        })
        .await
        .map_err(|error| JsValue::from_str(&error.to_string()))?;
    let result = reply
        .result
        .ok_or_else(|| JsValue::from_str("CAD test worker returned no result"))?;
    Ok(result.bodies.len() as u32)
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
                .remove(&wire.request_id)
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
                    request_id: wire.request_id,
                    job_id: wire.job_id,
                    identity: wire.identity,
                    operation: wire.operation,
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

fn decode_result(value: JsValue) -> Result<CadResult, String> {
    if Reflect::has(&value, &"queryQualification".into()).map_err(js_message)? {
        exact_fields(&value, &["revision", "collision", "queryQualification"])?;
    }
    let revision = Reflect::get(&value, &"revision".into())
        .map_err(js_message)?
        .as_f64()
        .ok_or_else(|| "CAD result omitted revision".to_string())?;
    if !revision.is_finite()
        || revision < 0.0
        || revision.fract() != 0.0
        || revision > crate::cad_jobs::MAX_CAD_REVISION as f64
    {
        return Err("CAD result revision is not an exact u53".into());
    }
    let revision = revision as u64;
    let step = read_u8_buffer(Reflect::get(&value, &"step".into()).map_err(js_message)?)?;
    let mesh_value = Reflect::get(&value, &"mesh".into()).map_err(js_message)?;
    let mesh = if mesh_value.is_null() || mesh_value.is_undefined() {
        None
    } else {
        Some(decode_mesh(mesh_value)?)
    };
    let bodies_value = Reflect::get(&value, &"bodies".into()).map_err(js_message)?;
    let body_values = if bodies_value.is_undefined() {
        Array::new()
    } else if Array::is_array(&bodies_value) {
        Array::from(&bodies_value)
    } else {
        return Err("CAD result bodies must be an array when present".into());
    };
    let mut bodies = Vec::with_capacity(body_values.length() as usize);
    for body in body_values.iter() {
        bodies.push(CadBodyMesh {
            id: string_field(&body, "id")?,
            name: string_field(&body, "name")?,
            positions: read_f32_buffer(
                Reflect::get(&body, &"positions".into()).map_err(js_message)?,
            )?,
            normals: read_f32_buffer(Reflect::get(&body, &"normals".into()).map_err(js_message)?)?,
        });
    }
    let bounds_value = Reflect::get(&value, &"bounds".into()).map_err(js_message)?;
    let bounds = if bounds_value.is_null() || bounds_value.is_undefined() {
        None
    } else {
        Some(serde_wasm_bindgen::from_value(bounds_value).map_err(|error| error.to_string())?)
    };
    let vendor_step_value = Reflect::get(&value, &"vendorStep".into()).map_err(js_message)?;
    let vendor_step = if vendor_step_value.is_null() || vendor_step_value.is_undefined() {
        None
    } else {
        Some(decode_vendor_step(vendor_step_value)?)
    };
    let collision_value = Reflect::get(&value, &"collision".into()).map_err(js_message)?;
    let collision = if collision_value.is_null() || collision_value.is_undefined() {
        None
    } else {
        Some(
            serde_wasm_bindgen::from_value::<CadCollisionObservation>(collision_value)
                .map_err(|error| error.to_string())?,
        )
    };
    Ok(CadResult {
        query_qualification: optional_strict_field(&value, "queryQualification")?,
        revision,
        step,
        mesh,
        bodies,
        bounds,
        vendor_step,
        collision,
        generated_analysis: {
            let analysis = Reflect::get(&value, &"generatedAnalysis".into()).map_err(js_message)?;
            if analysis.is_null() || analysis.is_undefined() {
                None
            } else {
                Some(decode_generated_analysis(analysis)?)
            }
        },
    })
}

fn decode_generated_analysis(
    value: JsValue,
) -> Result<crate::cad_jobs::CadGeneratedAnalysis, String> {
    validate_generated_payload_fields(&value, true)?;
    let positions_value =
        Reflect::get(&value, &"surfacePositionsMmF64".into()).map_err(js_message)?;
    let tags_value =
        Reflect::get(&value, &"triangleFaceOccurrenceIndices".into()).map_err(js_message)?;
    if !positions_value.is_instance_of::<Float64Array>()
        || !tags_value.is_instance_of::<Uint32Array>()
    {
        return Err(
            "generated analysis requires Float64Array positions and Uint32Array tags".into(),
        );
    }
    let positions = positions_value.unchecked_into::<Float64Array>();
    let tags = tags_value.unchecked_into::<Uint32Array>();
    validate_analysis_buffers(positions.as_ref(), tags.as_ref())?;
    if (positions.byte_length() as usize).saturating_add(tags.byte_length() as usize)
        > MAX_COLLISION_QUERY_BYTES
    {
        return Err("generated analysis exceeds the 64 MiB mesh limit".into());
    }
    // Deserialize metadata separately so large typed arrays retain their transfer path.
    let metadata = Object::new();
    for field in [
        "bodyId",
        "revision",
        "geometryIdentity",
        "sourceFaces",
        "faceOccurrences",
        "analysisErrorBoundMm",
        "meshClosedOriented",
        "deflectionMm",
        "analysisMesh",
        "sourceQualification",
        "analysisRef",
    ] {
        if Reflect::get(&value, &field.into())
            .map_err(js_message)?
            .is_undefined()
        {
            continue;
        }
        Reflect::set(
            &metadata,
            &field.into(),
            &Reflect::get(&value, &field.into()).map_err(js_message)?,
        )
        .map_err(js_message)?;
    }
    Reflect::set(&metadata, &"surfacePositionsMmF64".into(), &Array::new()).map_err(js_message)?;
    Reflect::set(
        &metadata,
        &"triangleFaceOccurrenceIndices".into(),
        &Array::new(),
    )
    .map_err(js_message)?;
    let mut analysis: crate::cad_jobs::CadGeneratedAnalysis =
        serde_wasm_bindgen::from_value(metadata.into()).map_err(|error| error.to_string())?;
    analysis.analysis_mesh = optional_strict_field(&value, "analysisMesh")?;
    analysis.source_qualification = optional_strict_field(&value, "sourceQualification")?;
    analysis.analysis_ref = optional_strict_field(&value, "analysisRef")?;
    analysis.surface_positions_mm_f64 = positions.to_vec();
    analysis.triangle_face_occurrence_indices = tags.to_vec();
    analysis.source_faces =
        strict_metadata(Reflect::get(&value, &"sourceFaces".into()).map_err(js_message)?)?;
    analysis.face_occurrences =
        strict_metadata(Reflect::get(&value, &"faceOccurrences".into()).map_err(js_message)?)?;
    Ok(analysis)
}

fn validate_generated_payload_fields(value: &JsValue, has_identity: bool) -> Result<(), String> {
    let mut fields = vec![
        "bodyId",
        "revision",
        "surfacePositionsMmF64",
        "triangleFaceOccurrenceIndices",
        "sourceFaces",
        "faceOccurrences",
        "analysisErrorBoundMm",
        "meshClosedOriented",
        "deflectionMm",
    ];
    if has_identity {
        fields.push("geometryIdentity");
    }
    if Reflect::has(value, &"analysisMesh".into()).unwrap_or(false) {
        fields.extend(["analysisMesh", "sourceQualification"]);
        fields.push(if has_identity {
            "analysisRef"
        } else {
            "analysisHandle"
        });
    }
    let object = value
        .dyn_ref::<Object>()
        .ok_or("generated analysis must be an object")?;
    let keys = Object::keys(object);
    if keys.length() as usize != fields.len()
        || fields
            .iter()
            .any(|field| !Reflect::has(value, &(*field).into()).unwrap_or(false))
        || keys.iter().any(|key| {
            key.as_string()
                .is_none_or(|key| !fields.contains(&key.as_str()))
        })
    {
        return Err("generated analysis has missing or unexpected fields".into());
    }
    Ok(())
}

fn decode_vendor_step(value: JsValue) -> Result<CadVendorStep, String> {
    let v2 = Reflect::has(&value, &"analysisMesh".into()).map_err(js_message)?;
    if v2 {
        exact_fields(
            &value,
            &[
                "sourceSha256",
                "importOptions",
                "sourceFaces",
                "faceOccurrences",
                "precision",
                "accuracy",
                "surfacePositionsMmF64",
                "triangleFaceOccurrenceIndices",
                "lineSegmentsMmF64",
                "lineRenderPositions",
                "lineOccurrences",
                "segmentLineOccurrenceIndices",
                "analysisMesh",
                "sourceQualification",
                "analysisRef",
            ],
        )?;
        validate_analysis_buffers(
            &Reflect::get(&value, &"surfacePositionsMmF64".into()).map_err(js_message)?,
            &Reflect::get(&value, &"triangleFaceOccurrenceIndices".into()).map_err(js_message)?,
        )?;
        strict_metadata::<Vec<CadStepSourceFace>>(
            Reflect::get(&value, &"sourceFaces".into()).map_err(js_message)?,
        )?;
        strict_metadata::<Vec<CadStepFaceOccurrence>>(
            Reflect::get(&value, &"faceOccurrences".into()).map_err(js_message)?,
        )?;
        strict_metadata::<Vec<CadStepPrecision>>(
            Reflect::get(&value, &"precision".into()).map_err(js_message)?,
        )?;
        strict_step_accuracy(Reflect::get(&value, &"accuracy".into()).map_err(js_message)?)?;
    }
    let source_sha256 = string_field(&value, "sourceSha256")?;
    let options_value = Reflect::get(&value, &"importOptions".into()).map_err(js_message)?;
    if v2 && !options_value.is_null() && !options_value.is_undefined() {
        allowed_fields(
            &options_value,
            &["previewRepairBudgetMm", "targetDeflectionMm"],
        )?;
    }
    let import_options = if options_value.is_null() || options_value.is_undefined() {
        CadStepImportOptions::default()
    } else {
        serde_wasm_bindgen::from_value::<CadStepImportOptions>(options_value)
            .map_err(|error| error.to_string())?
    };
    let surface_positions_mm_f64 = read_f64_buffer(
        Reflect::get(&value, &"surfacePositionsMmF64".into()).map_err(js_message)?,
    )?;
    let source_faces: Vec<CadStepSourceFace> = serde_wasm_bindgen::from_value(
        Reflect::get(&value, &"sourceFaces".into()).map_err(js_message)?,
    )
    .map_err(|error| error.to_string())?;
    let face_occurrences: Vec<CadStepFaceOccurrence> = serde_wasm_bindgen::from_value(
        Reflect::get(&value, &"faceOccurrences".into()).map_err(js_message)?,
    )
    .map_err(|error| error.to_string())?;
    let triangle_face_occurrence_indices = read_u32_buffer(
        Reflect::get(&value, &"triangleFaceOccurrenceIndices".into()).map_err(js_message)?,
    )?;
    let line_segments_mm_f64 =
        read_f64_buffer(Reflect::get(&value, &"lineSegmentsMmF64".into()).map_err(js_message)?)?;
    let line_render_positions =
        read_f32_buffer(Reflect::get(&value, &"lineRenderPositions".into()).map_err(js_message)?)?;
    let line_occurrences_value =
        Reflect::get(&value, &"lineOccurrences".into()).map_err(js_message)?;
    if !Array::is_array(&line_occurrences_value) {
        return Err("STEP vendorStep.lineOccurrences must be an array".into());
    }
    let line_occurrences_array = Array::from(&line_occurrences_value);
    let mut line_occurrences = Vec::with_capacity(line_occurrences_array.length() as usize);
    for item in line_occurrences_array.iter() {
        if v2 {
            exact_fields(
                &item,
                &[
                    "lineOccurrenceIndex",
                    "trimmedCurveId",
                    "sourceLineId",
                    "curveSetId",
                    "representationId",
                    "sourceUnitToMm",
                    "placementF64",
                    "declaredUncertaintyMm",
                ],
            )?;
        }
        let placement_vec =
            read_f64_buffer(Reflect::get(&item, &"placementF64".into()).map_err(js_message)?)?;
        let placement_f64: [f64; 16] = placement_vec
            .try_into()
            .map_err(|_| "STEP lineOccurrence placementF64 must have 16 values".to_string())?;
        let line_occurrence_index = Reflect::get(&item, &"lineOccurrenceIndex".into())
            .map_err(js_message)?
            .as_f64()
            .ok_or_else(|| "STEP lineOccurrence omitted lineOccurrenceIndex".to_string())?;
        if !line_occurrence_index.is_finite()
            || line_occurrence_index < 0.0
            || line_occurrence_index > u32::MAX as f64
            || line_occurrence_index.fract() != 0.0
        {
            return Err("STEP lineOccurrence lineOccurrenceIndex is invalid".into());
        }
        let source_unit_to_mm = Reflect::get(&item, &"sourceUnitToMm".into())
            .map_err(js_message)?
            .as_f64()
            .ok_or_else(|| "STEP lineOccurrence omitted sourceUnitToMm".to_string())?;
        let declared_uncertainty_value =
            Reflect::get(&item, &"declaredUncertaintyMm".into()).map_err(js_message)?;
        let declared_uncertainty_mm =
            if declared_uncertainty_value.is_null() || declared_uncertainty_value.is_undefined() {
                None
            } else {
                Some(declared_uncertainty_value.as_f64().ok_or_else(|| {
                    "STEP lineOccurrence declaredUncertaintyMm must be a number or null".to_string()
                })?)
            };
        line_occurrences.push(CadStepLineOccurrence {
            line_occurrence_index: line_occurrence_index as u32,
            trimmed_curve_id: string_field(&item, "trimmedCurveId")?,
            source_line_id: string_field(&item, "sourceLineId")?,
            curve_set_id: string_field(&item, "curveSetId")?,
            representation_id: string_field(&item, "representationId")?,
            source_unit_to_mm,
            placement_f64,
            declared_uncertainty_mm,
        });
    }
    let segment_line_occurrence_indices = read_u32_buffer(
        Reflect::get(&value, &"segmentLineOccurrenceIndices".into()).map_err(js_message)?,
    )?;
    let precision: Vec<CadStepPrecision> = serde_wasm_bindgen::from_value(
        Reflect::get(&value, &"precision".into()).map_err(js_message)?,
    )
    .map_err(|error| error.to_string())?;
    let accuracy: CadStepAccuracy = serde_wasm_bindgen::from_value(
        Reflect::get(&value, &"accuracy".into()).map_err(js_message)?,
    )
    .map_err(|error| error.to_string())?;
    Ok(CadVendorStep {
        analysis_ref: optional_strict_field(&value, "analysisRef")?,
        analysis_mesh: optional_strict_field(&value, "analysisMesh")?,
        source_qualification: optional_strict_field(&value, "sourceQualification")?,
        source_sha256,
        import_options,
        surface_positions_mm_f64,
        source_faces,
        face_occurrences,
        triangle_face_occurrence_indices,
        line_segments_mm_f64,
        line_render_positions,
        line_occurrences,
        segment_line_occurrence_indices,
        precision,
        accuracy,
    })
}

fn strict_step_accuracy(value: JsValue) -> Result<CadStepAccuracy, String> {
    // This pre-v2 field is omitted by the producer when no repair was requested.
    // Normalize only that established omission; v2 nullable fields stay required.
    let object = value
        .dyn_ref::<Object>()
        .ok_or("accuracy object required")?;
    let normalized = Object::assign(&Object::new(), object);
    let key = JsValue::from_str("selectedRepairBudgetMm");
    if !Object::has_own(&normalized, &key) {
        Reflect::set(&normalized, &key, &JsValue::NULL).map_err(js_message)?;
    }
    strict_metadata(normalized.into())
}

fn strict_metadata<T: serde::de::DeserializeOwned + Serialize>(
    value: JsValue,
) -> Result<T, String> {
    let decoded: T = serde_wasm_bindgen::from_value(value.clone()).map_err(|e| e.to_string())?;
    let expected =
        js_sys::JSON::parse(&serde_json::to_string(&decoded).map_err(|e| e.to_string())?)
            .map_err(js_message)?;
    fn keys(actual: &JsValue, expected: &JsValue) -> Result<(), String> {
        if Array::is_array(expected) {
            if !Array::is_array(actual) {
                return Err("metadata array required".into());
            }
            let a = Array::from(actual);
            let e = Array::from(expected);
            if a.length() != e.length() {
                return Err("metadata array length mismatch".into());
            }
            for i in 0..a.length() {
                keys(&a.get(i), &e.get(i))?;
            }
        } else if expected.is_object() && !expected.is_null() {
            let a = actual
                .dyn_ref::<Object>()
                .ok_or("metadata object required")?;
            let e = expected.unchecked_ref::<Object>();
            let names = Object::keys(e);
            if Object::keys(a).length() != names.length() {
                return Err("missing or unexpected metadata fields".into());
            }
            for name in names.iter() {
                if !Object::has_own(a, &name) {
                    return Err("missing own metadata field".into());
                }
                keys(
                    &Reflect::get(actual, &name).map_err(js_message)?,
                    &Reflect::get(expected, &name).map_err(js_message)?,
                )?;
            }
        } else if expected.is_null() && !actual.is_null() {
            return Err("explicit nullable metadata required".into());
        }
        Ok(())
    }
    keys(&value, &expected)?;
    Ok(decoded)
}

fn exact_fields(value: &JsValue, fields: &[&str]) -> Result<(), String> {
    let object = value
        .dyn_ref::<Object>()
        .ok_or("metadata object required")?;
    if Object::keys(object).length() as usize != fields.len()
        || fields
            .iter()
            .any(|field| !Object::has_own(object, &(*field).into()))
    {
        return Err("missing or unexpected metadata fields".into());
    }
    Ok(())
}

fn allowed_fields(value: &JsValue, fields: &[&str]) -> Result<(), String> {
    let object = value
        .dyn_ref::<Object>()
        .ok_or("metadata object required")?;
    if Object::keys(object).iter().any(|key| {
        !key.as_string()
            .is_some_and(|key| fields.contains(&key.as_str()))
    }) {
        return Err("unexpected metadata field".into());
    }
    Ok(())
}

fn raw_query_metadata(query: &CadCollisionQuery) -> Result<JsValue, String> {
    // The retained raw export has strict v1 keys. Selectors stay in the worker;
    // dropping them here cannot confer qualification on this fallback.
    let mut value = serde_json::to_value(query).map_err(|e| e.to_string())?;
    let object = value.as_object_mut().ok_or("raw query metadata object")?;
    object.remove("qualificationLimits");
    for side in ["left", "right"] {
        let operand = object
            .get_mut(side)
            .and_then(serde_json::Value::as_object_mut)
            .ok_or("raw query operand object")?;
        operand.remove("analysisRef");
        operand.remove("analysisMesh");
    }
    js_sys::JSON::parse(&value.to_string()).map_err(js_message)
}

fn optional_strict_field<T: serde::de::DeserializeOwned + Serialize>(
    value: &JsValue,
    field: &str,
) -> Result<Option<T>, String> {
    let value = Reflect::get(value, &field.into()).map_err(js_message)?;
    if value.is_undefined() {
        Ok(None)
    } else {
        strict_metadata(value).map(Some)
    }
}

fn decode_mesh(value: JsValue) -> Result<CadMesh, String> {
    Ok(CadMesh {
        positions: read_f32_buffer(Reflect::get(&value, &"positions".into()).map_err(js_message)?)?,
        normals: read_f32_buffer(Reflect::get(&value, &"normals".into()).map_err(js_message)?)?,
    })
}

fn string_field(value: &JsValue, name: &str) -> Result<String, String> {
    Reflect::get(value, &name.into())
        .map_err(js_message)?
        .as_string()
        .ok_or_else(|| format!("CAD result omitted {name}"))
}

fn read_f32_buffer(value: JsValue) -> Result<Vec<f32>, String> {
    if value.is_instance_of::<Float32Array>() {
        Ok(value.unchecked_into::<Float32Array>().to_vec())
    } else {
        serde_wasm_bindgen::from_value(value).map_err(|error| error.to_string())
    }
}

fn read_f64_buffer(value: JsValue) -> Result<Vec<f64>, String> {
    if value.is_instance_of::<Float64Array>() {
        Ok(value.unchecked_into::<Float64Array>().to_vec())
    } else {
        serde_wasm_bindgen::from_value(value).map_err(|error| error.to_string())
    }
}

fn validate_analysis_buffers(positions: &JsValue, tags: &JsValue) -> Result<(), String> {
    let positions = positions
        .dyn_ref::<Float64Array>()
        .ok_or("analysis positions require Float64Array")?;
    let tags = tags
        .dyn_ref::<Uint32Array>()
        .ok_or("analysis tags require Uint32Array")?;
    if !positions.buffer().is_instance_of::<js_sys::ArrayBuffer>()
        || !tags.buffer().is_instance_of::<js_sys::ArrayBuffer>()
        || positions.length() == 0
        || positions.length() % 9 != 0
        || tags.length() != positions.length() / 9
        || (positions.byte_length() as usize).saturating_add(tags.byte_length() as usize)
            > MAX_COLLISION_QUERY_BYTES
    {
        return Err("detached/shared/malformed or oversized analysis buffers".into());
    }
    Ok(())
}

fn read_u32_buffer(value: JsValue) -> Result<Vec<u32>, String> {
    if value.is_instance_of::<Uint32Array>() {
        Ok(value.unchecked_into::<Uint32Array>().to_vec())
    } else {
        serde_wasm_bindgen::from_value(value).map_err(|error| error.to_string())
    }
}

fn read_u8_buffer(value: JsValue) -> Result<Vec<u8>, String> {
    if value.is_null() || value.is_undefined() {
        return Ok(Vec::new());
    }
    if value.is_instance_of::<Uint8Array>() {
        Ok(value.unchecked_into::<Uint8Array>().to_vec())
    } else {
        serde_wasm_bindgen::from_value(value).map_err(|error| error.to_string())
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

#[cfg(all(test, target_arch = "wasm32"))]
mod keycaps_worker_tests {
    use super::*;
    use boardstudio_core::model::{KeycapMount, KeycapProfile, Pose2, Side, Vec2};
    use wasm_bindgen_test::wasm_bindgen_test;
    use web_sys::{Blob, BlobPropertyBag, Url};

    wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

    struct WorkerSource(String);

    impl WorkerSource {
        fn new(source: &str) -> Self {
            let parts = Array::of1(&JsValue::from_str(source));
            let options = BlobPropertyBag::new();
            options.set_type("text/javascript");
            let blob = Blob::new_with_str_sequence_and_options(&parts, &options).unwrap();
            Self(Url::create_object_url_with_blob(&blob).unwrap())
        }
    }

    impl Drop for WorkerSource {
        fn drop(&mut self) {
            let _ = Url::revoke_object_url(&self.0);
        }
    }

    fn spec() -> KeycapSpec {
        KeycapSpec {
            id: "key-1".into(),
            reference: "A1".into(),
            profile: KeycapProfile::Cherry,
            mount: KeycapMount::Mx,
            row: 1,
            size: Vec2 { x: 18.0, y: 18.0 },
            top_size: Vec2 { x: 12.0, y: 12.0 },
            height: 8.0,
            tilt: 0.0,
            dish_depth: 0.0,
            spherical: false,
            wall_thickness: 1.2,
            pose: Pose2 {
                at: Vec2 { x: 0.0, y: 0.0 },
                rotation: 0.0,
            },
            side: Side::Front,
            z: 8.4,
            travel: 4.0,
            legend: String::new(),
            color: "#b0b5bd".into(),
            legend_color: "#202124".into(),
        }
    }

    #[wasm_bindgen_test]
    async fn keycaps_print_uses_a_distinct_operation_and_one_fused_body() {
        let source = WorkerSource::new(
            r#"
          self.postMessage({kind:'ready'});
          self.onmessage = event => {
            const input = JSON.parse(event.data.frame);
            const correct = input.operation === 'print-keycaps';
            const replyOperation = input.requestId === 'print-stale' ? 'preview' : input.operation;
            const frame = JSON.stringify({requestId:input.requestId,jobId:input.jobId,
              identity:input.identity,operation:replyOperation,outcome:correct?'completed':'failed',
              error:correct?null:'Print must not use preview identity'});
            const result = {revision:input.identity.revision,step:new Uint8Array(),mesh:null,
              bodies:[{id:'keycap:key-1',name:'A1',positions:new Float32Array([0,0,0,1,0,0,0,1,0]),
                normals:new Float32Array([0,0,1,0,0,1,0,0,1])}],bounds:null};
            if(input.requestId === 'print-truncated') {
              result.bodies[0].positions = new Float32Array([0,0,0]);
              result.bodies[0].normals = new Float32Array([0,0,1]);
            }
            self.postMessage({kind:'reply',frame,result});
          };
        "#,
        );
        let worker = CadWorker::new(&source.0).unwrap();
        let identity = CadSnapshotIdentity {
            token: 1,
            session_epoch: 1,
            document_id: "print".into(),
            board_id: "board".into(),
            instance_id: None,
            revision: 1,
        };
        let mut cap = spec();
        cap.legend = "O".into();
        let result = worker
            .request_keycaps_print(
                "print-1".into(),
                "print-job".into(),
                identity.clone(),
                vec![cap.clone()],
            )
            .await
            .expect("print operation must not share preview identity or separate legend IDs");
        assert_eq!(result.bodies.len(), 1);
        for request in ["print-stale", "print-truncated"] {
            assert!(
                worker
                    .request_keycaps_print(
                        request.into(),
                        request.into(),
                        identity.clone(),
                        vec![cap.clone()]
                    )
                    .await
                    .is_err(),
                "stale or incomplete print results must refuse"
            );
        }
        worker.close();
    }

    #[wasm_bindgen_test]
    async fn keycaps_worker_failure_late_reply_stale_revision_and_retry_are_correlated() {
        let source = WorkerSource::new(
            r#"
            self.postMessage({ kind: "ready" });
            self.onmessage = (event) => {
              const input = JSON.parse(event.data.frame);
              const respond = (outcome, error, bodyId, revision = input.identity.revision) => {
                const frame = JSON.stringify({
                  requestId: input.requestId,
                  jobId: input.jobId,
                  identity: input.identity,
                  operation: input.operation,
                  outcome,
                  error,
                });
                const result = {
                  revision,
                  step: new Uint8Array([73, 83, 79, 45, 49, 48, 51, 48, 51, 45, 50, 49, 59]),
                  mesh: null,
                  bodies: input.operation === "preview"
                    ? [{ id: bodyId, name: "test cap", positions: new Float32Array([0, 0, 0]), normals: new Float32Array([0, 0, 1]) }]
                    : [],
                  bounds: null,
                };
                self.postMessage({ kind: "reply", frame, result });
              };
              if (input.requestId === "keycaps-preview-old") {
                respond("failed", "injected preview provider failure", "keycap:key-1");
                setTimeout(() => respond("completed", null, "keycap:late-old"), 25);
              } else if (input.requestId === "keycaps-preview-current") {
                setTimeout(() => respond("completed", null, "keycap:key-1"), 60);
              } else if (input.requestId === "keycaps-preview-stale") {
                respond("completed", null, "", input.identity.revision + 1);
              }
            };
            "#,
        );
        let worker = CadWorker::new(&source.0).unwrap();
        worker.ready().await.unwrap();

        let identity = CadSnapshotIdentity {
            token: 7,
            session_epoch: 2,
            document_id: "keycaps-worker-test".into(),
            board_id: "left".into(),
            instance_id: None,
            revision: 11,
        };
        let keycap = spec();
        let failed_preview = worker
            .request_keycaps_preview(
                "keycaps-preview-old".into(),
                "preview-old-job".into(),
                identity.clone(),
                vec![keycap.clone()],
            )
            .await
            .unwrap_err();
        assert_eq!(failed_preview.0, "injected preview provider failure");

        let current_preview = worker
            .request_keycaps_preview(
                "keycaps-preview-current".into(),
                "preview-current-job".into(),
                identity.clone(),
                vec![keycap.clone()],
            )
            .await
            .expect("the current preview must settle after the delayed old reply");
        assert_eq!(current_preview.revision, identity.revision);
        assert_eq!(current_preview.bodies[0].id, "keycap:key-1");

        let stale_preview = worker
            .request_keycaps_preview(
                "keycaps-preview-stale".into(),
                "preview-stale-job".into(),
                identity.clone(),
                vec![keycap],
            )
            .await
            .unwrap_err();
        assert_eq!(
            stale_preview.0,
            "Keycap CAD preview meshes are missing, malformed, or stale"
        );
        assert_eq!(
            keycap_operation_error(&CadOperation::ExportStep, true),
            Some(KEYCAP_STEP_UNSUPPORTED)
        );
        assert_eq!(
            keycap_operation_error(&CadOperation::ExportStep, false),
            None
        );
        worker.close();
    }

    #[wasm_bindgen_test]
    async fn read_step_and_collision_query_transfer_typed_arrays_and_validate_replies() {
        use crate::cad_jobs::{
            CadCollisionMeshPair, CadCollisionObservation, CadCollisionOperand, CadCollisionQuery,
            CadStepImportOptions, validate_reply,
        };

        let source = WorkerSource::new(
            r#"
            self.postMessage({ kind: "ready" });
            self.onmessage = (event) => {
              const input = JSON.parse(event.data.frame);
              if (input.operation === "read-step") {
                const frame = JSON.stringify({
                  requestId: input.requestId,
                  jobId: input.jobId,
                  identity: input.identity,
                  operation: input.operation,
                  outcome: "completed",
                  error: null,
                });
                const result = {
                  revision: input.identity.revision,
                  step: new Uint8Array(),
                  mesh: {
                    positions: new Float32Array([0, 0, 0, 1, 0, 0, 0, 1, 0]),
                    normals: new Float32Array([0, 0, 1, 0, 0, 1, 0, 0, 1]),
                  },
                  bodies: [],
                  bounds: { min: [0, 0, 0], max: [2, 2, 0] },
                  vendorStep: {
                    sourceSha256: input.sourceSha256,
                    importOptions: input.stepImportOptions ?? { previewRepairBudgetMm: null },
                    surfacePositionsMmF64: new Float64Array([0, 0, 0, 1, 0, 0, 0, 1, 0]),
                    sourceFaces: [{
                      solidId: "10",
                      shellIndex: 0,
                      faceId: "20",
                      supportId: "30",
                      supportKind: "PLANE",
                    }],
                    faceOccurrences: [{
                      sourceFaceIndex: 0,
                      solidOccurrenceIndex: 0,
                      representationId: "40",
                      sourceUncertaintyMm: 0.01,
                      repairDisplacementMm: null,
                    }],
                    triangleFaceOccurrenceIndices: new Uint32Array([0]),
                    lineSegmentsMmF64: new Float64Array([0, 0, 0, 2, 2, 0]),
                    lineRenderPositions: new Float32Array([0, 0, 0, 2, 2, 0]),
                    lineOccurrences: [{
                      lineOccurrenceIndex: 0,
                      trimmedCurveId: "414",
                      sourceLineId: "413",
                      curveSetId: "420",
                      representationId: "430",
                      sourceUnitToMm: 1.0,
                      placementF64: new Float64Array([1,0,0,0, 0,1,0,0, 0,0,1,0, 0,0,0,1]),
                      declaredUncertaintyMm: null,
                    }],
                    segmentLineOccurrenceIndices: new Uint32Array([0]),
                    precision: [],
                    accuracy: {
                      previewDeflectionMm: 0.1,
                      previewAngularRad: 0.5,
                      analysisErrorBoundMm: null,
                      boundsFromSourceGeometry: true,
                      meshClosedOriented: true,
                      previewFloorMm: 0.02,
                      previewDeclaredMultiplier: 2.0,
                      relaxedBeyondDeclaredPrecision: false,
                      sampledSupportChecks: 0,
                      maxSampledSupportResidualMm: null,
                      fallbackMeshedFaces: 0,
                    },
                  },
                };
                self.postMessage({ kind: "reply", frame, result });
              } else if (input.operation === "collision-query") {
                const leftTags = new Uint32Array(event.data.leftTags);
                const rightTags = new Uint32Array(event.data.rightTags);
                const frame = JSON.stringify({
                  requestId: input.requestId,
                  jobId: input.jobId,
                  identity: input.identity,
                  operation: input.operation,
                  outcome: "completed",
                  error: null,
                });
                const result = {
                  revision: input.identity.revision,
                  step: new Uint8Array(),
                  mesh: null,
                  bodies: [],
                  bounds: null,
                  collision: {
                    kind: "surface-separated",
                    surfaceDistanceMm: 1.25,
                    closestPoints: {
                      triangles: {
                        left: { triangleIndex: 0, faceOccurrenceIndex: leftTags[0] },
                        right: { triangleIndex: 0, faceOccurrenceIndex: rightTags[0] },
                      },
                      point1Mm: [1.0, 0.0, 0.0],
                      point2Mm: [2.25, 0.0, 0.0],
                    },
                  },
                };
                const q = input.collisionQuery;
                const unknown = () => ({state:'unknown',reasons:['evidence-unavailable']});
                result.queryQualification = {version:1,algorithm:'aabb-interval-exact-v1',limits:q.qualificationLimits,
                  binding:{snapshotRevision:input.identity.revision,snapshot:{...input.identity,token:String(input.identity.token),sessionEpoch:String(input.identity.sessionEpoch)},
                    workerGeneration:q.left.analysisRef.workerGeneration,leftModelId:q.left.modelId,rightModelId:q.right.modelId,
                    leftMeshIdentity:q.left.analysisRef.meshIdentity,rightMeshIdentity:q.right.analysisRef.meshIdentity,
                    leftAnalysis:q.left.analysisRef,rightAnalysis:q.right.analysisRef,leftMesh:q.left.analysisMesh,rightMesh:q.right.analysisMesh,
                    leftRowTransformF64:q.left.rowTransformF64,rightRowTransformF64:q.right.rowTransformF64,
                    clearanceThresholdMm:q.clearanceThresholdMm,requestedAccuracyMm:q.requestedAccuracyMm},
                  meshDistance:unknown(),sourceBounds:[unknown(),unknown()],contactRegion:unknown(),containment:unknown()};
                for (const key of ['step','mesh','bodies','bounds']) delete result[key];
                self.postMessage({ kind: "reply", frame, result });
              }
            };
            "#,
        );
        let worker = CadWorker::new(&source.0).unwrap();
        let identity = CadSnapshotIdentity {
            token: 1,
            session_epoch: 1,
            document_id: "step-collision".into(),
            board_id: "board".into(),
            instance_id: None,
            revision: 3,
        };
        let sha_a = "4bf5122f344554c53bde2ebb8cd2b7e3d1600ad631c385a5d7cce23c7785459a";
        let step_req = CadRequest {
            request_id: "step-1".into(),
            job_id: "step-job-1".into(),
            identity: identity.clone(),
            operation: CadOperation::ReadStep,
            prepared: None,
            source_sha256: Some(sha_a.into()),
            step_import_options: Some(CadStepImportOptions::default()),
            collision_query: None,
            generated_analysis_request: None,
            collision_meshes: None,
            input_bytes: vec![1],
        };
        let step_reply = worker.request(step_req.clone()).await.unwrap();
        let validated_step = validate_reply(&step_req, step_reply, &identity).unwrap();
        let vendor_step = validated_step.vendor_step.unwrap();
        assert_eq!(vendor_step.source_sha256, sha_a);
        assert_eq!(vendor_step.line_occurrences.len(), 1);
        assert_eq!(
            vendor_step.line_render_positions,
            [0.0, 0.0, 0.0, 2.0, 2.0, 0.0]
        );

        let id_mat = [
            1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
        ];
        let fixture: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../cad/wasm/tests/fixtures/generated-analysis-contract.json"
        ))
        .unwrap();
        let descriptor: qualification::AnalysisMesh = serde_json::from_value(
            fixture["qualificationV2"]["activation"]["descriptors"]["vendor"].clone(),
        )
        .unwrap();
        let mut reference: qualification::AnalysisRef = serde_json::from_value(
            fixture["qualificationV2"]["activation"]["workerReferences"]["vendor"].clone(),
        )
        .unwrap();
        reference.source = qualification::Source::VendorStep {
            source_sha256: sha_a.into(),
            import_options: CadStepImportOptions::default().into(),
        };
        reference.derivation_identity =
            qualification::derivation_identity(&reference.source, &descriptor);
        let collision_req = CadRequest {
            request_id: "col-1".into(),
            job_id: "col-job-1".into(),
            identity: identity.clone(),
            operation: CadOperation::CollisionQuery,
            prepared: None,
            source_sha256: None,
            step_import_options: None,
            collision_query: Some(CadCollisionQuery {
                left: CadCollisionOperand {
                    analysis_ref: Some(reference.clone()),
                    analysis_mesh: Some(descriptor.clone()),
                    model_id: "m1".into(),
                    source_sha256: Some(sha_a.into()),
                    generated_geometry_identity: None,
                    import_options: CadStepImportOptions::default(),
                    row_transform_f64: id_mat,
                },
                right: CadCollisionOperand {
                    analysis_ref: Some(reference.clone()),
                    analysis_mesh: Some(descriptor.clone()),
                    model_id: "m2".into(),
                    source_sha256: Some(sha_a.into()),
                    generated_geometry_identity: None,
                    import_options: CadStepImportOptions::default(),
                    row_transform_f64: id_mat,
                },
                clearance_threshold_mm: 0.5,
                requested_accuracy_mm: None,
                qualification_limits: Default::default(),
            }),
            generated_analysis_request: None,
            collision_meshes: Some(CadCollisionMeshPair {
                left_positions_mm_f64: vec![0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0],
                left_triangle_face_occurrence_indices: vec![7],
                right_positions_mm_f64: vec![2.25, 0.0, 0.0, 3.25, 0.0, 0.0, 2.25, 1.0, 0.0],
                right_triangle_face_occurrence_indices: vec![11],
            }),
            input_bytes: vec![],
        };
        let col_reply = worker.request(collision_req.clone()).await.unwrap();
        let validated_col = validate_reply(&collision_req, col_reply, &identity).unwrap();
        match validated_col.collision.unwrap() {
            CadCollisionObservation::SurfaceSeparated {
                surface_distance_mm,
                closest_points,
            } => {
                assert_eq!(surface_distance_mm, 1.25);
                assert_eq!(closest_points.triangles.left.face_occurrence_index, 7);
                assert_eq!(closest_points.triangles.right.face_occurrence_index, 11);
            }
            other => panic!("unexpected observation {other:?}"),
        }
        worker.close();
    }
}

fn js_error(error: JsValue) -> CadWorkerError {
    CadWorkerError(
        error
            .as_string()
            .unwrap_or_else(|| "JavaScript CAD worker failure".into()),
    )
}

#[wasm_bindgen]
pub fn start_cad_worker(cad_module_url: String) -> Result<(), JsValue> {
    let scope: DedicatedWorkerGlobalScope = js_sys::global().dyn_into()?;
    let target = scope.clone();
    spawn_local(async move {
        let module = match initialize_cad_module(&cad_module_url).await {
            Ok(module) => Rc::new(module),
            Err(error) => {
                let _ = target.post_message(&worker_message("init-failed", &error, None));
                return;
            }
        };
        let queue = Rc::new(RefCell::new(WorkerQueue::default()));
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
        let _ = target.post_message(&worker_message("ready", "", None));
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

struct WorkerQueue {
    pending: VecDeque<(WireRequest, Vec<u8>)>,
    cancelled: BTreeSet<String>,
    active: bool,
    active_job: Option<String>,
    active_request: Option<String>,
    generation: String,
    evidence: BTreeMap<String, Rc<OwnedAnalysis>>,
    evidence_recency: VecDeque<String>,
    retained_bytes: usize,
    latest_snapshot: Option<CadSnapshotIdentity>,
}

impl Default for WorkerQueue {
    fn default() -> Self {
        let generation = Function::new_no_args("return crypto.randomUUID()")
            .call0(&JsValue::NULL)
            .expect("worker UUID source")
            .as_string()
            .expect("worker UUID");
        Self {
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

struct OwnedAnalysis {
    handle: ProviderHandle,
    reference: AnalysisRef,
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
    fn cancel_job(&mut self, job: &str) {
        if self.active_job.as_deref() == Some(job)
            || self
                .pending
                .iter()
                .any(|(request, _)| request.job_id == job)
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
    fn remove_evidence(&mut self, identity: &str) {
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

    fn lookup(
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

fn worker_metadata_storage(
    reference: &AnalysisRef,
    descriptor: &AnalysisMesh,
    report: &SourceQualification,
    request: &WireRequest,
) -> Option<usize> {
    let mut strings = vec![
        &reference.worker_generation,
        &reference.mesh_identity,
        &reference.derivation_identity,
        &descriptor.version,
        &descriptor.mesh_identity,
        &descriptor.source_proof,
        &request.identity.document_id,
        &request.identity.board_id,
        &request.request_id,
        &request.job_id,
    ];
    if let Some(instance) = &request.identity.instance_id {
        strings.push(instance);
    }
    match &reference.source {
        Source::VendorStep { source_sha256, .. } => strings.push(source_sha256),
        Source::GeneratedBody {
            body_id,
            geometry_identity,
            ..
        } => strings.extend([body_id, geometry_identity]),
    }
    let reason_storage = match report {
        SourceQualification::Unknown { reasons } => {
            strings.extend(reasons);
            reasons
                .capacity()
                .checked_mul(std::mem::size_of::<String>())?
        }
        SourceQualification::Bounded {
            class,
            mesh_identity,
            accounting,
            ..
        } => {
            strings.extend([class, mesh_identity, accounting]);
            0
        }
    };
    // Include Rc bookkeeping and a conservative per-entry container allowance;
    // the provider accounts its own entire proof/catalogue allocation separately.
    let inline = std::mem::size_of::<OwnedAnalysis>() + 2 * std::mem::size_of::<usize>() + 1024;
    let bytes = strings
        .into_iter()
        .try_fold(inline.checked_add(reason_storage)?, |n, s| {
            n.checked_add(s.capacity())
        })?;
    bytes.checked_add(reference.mesh_identity.len().checked_mul(2)?)
}

fn stage_analysis(
    queue: &Rc<RefCell<WorkerQueue>>,
    module: &JsValue,
    request: &WireRequest,
    payload: &JsValue,
    source: Source,
) -> Result<Rc<OwnedAnalysis>, String> {
    let raw_handle = Reflect::get(payload, &"analysisHandle".into()).map_err(js_message)?;
    let class = Reflect::get(module, &"AnalysisHandle".into()).map_err(js_message)?;
    let is_owned = Function::new_with_args(
        "handle,owner",
        "return typeof owner === 'function' && handle instanceof owner",
    )
    .call2(&JsValue::NULL, &raw_handle, &class)
    .map_err(js_message)?
    .as_bool()
        == Some(true);
    if !is_owned {
        return Err("provider must return its opaque AnalysisHandle".into());
    }
    let handle = ProviderHandle(raw_handle);
    Reflect::delete_property(payload.unchecked_ref::<Object>(), &"analysisHandle".into())
        .map_err(js_message)?;
    let descriptor: AnalysisMesh =
        strict_metadata(Reflect::get(payload, &"analysisMesh".into()).map_err(js_message)?)?;
    let report: SourceQualification =
        strict_metadata(Reflect::get(payload, &"sourceQualification".into()).map_err(js_message)?)?;
    let reference = AnalysisRef {
        worker_generation: queue.borrow().generation.clone(),
        mesh_identity: descriptor.mesh_identity.clone(),
        derivation_identity: qualification::derivation_identity(&source, &descriptor),
        source,
    };
    let bound_value = match reference.source {
        Source::GeneratedBody { .. } => {
            Reflect::get(payload, &"analysisErrorBoundMm".into()).map_err(js_message)?
        }
        Source::VendorStep { .. } => Reflect::get(
            &Reflect::get(payload, &"accuracy".into()).map_err(js_message)?,
            &"analysisErrorBoundMm".into(),
        )
        .map_err(js_message)?,
    };
    let bound = if bound_value.is_null() {
        None
    } else {
        Some(bound_value.as_f64().ok_or("invalid source bound")?)
    };
    qualification::validate_analysis(&reference, &descriptor, &report, bound)?;
    let method = Reflect::get(&handle.0, &"retainedStorageBytes".into())
        .map_err(js_message)?
        .dyn_into::<Function>()
        .map_err(js_message)?;
    let cost = method.call0(&handle.0).map_err(js_message)?;
    let positions_value =
        Reflect::get(payload, &"surfacePositionsMmF64".into()).map_err(js_message)?;
    let tags_value =
        Reflect::get(payload, &"triangleFaceOccurrenceIndices".into()).map_err(js_message)?;
    validate_analysis_buffers(&positions_value, &tags_value)?;
    let positions = positions_value.unchecked_into::<Float64Array>().to_vec();
    let tags = tags_value.unchecked_into::<Uint32Array>().to_vec();
    let retained_bytes = if cost.is_null() {
        None
    } else {
        let number = cost.as_f64().ok_or("invalid retained storage cost")?;
        if !number.is_finite()
            || number < 0.0
            || number.fract() != 0.0
            || number > crate::cad_jobs::MAX_CAD_REVISION as f64
        {
            return Err("invalid retained storage cost".into());
        }
        (number as usize)
            .checked_add(positions.capacity().saturating_mul(8))
            .and_then(|n| n.checked_add(tags.capacity().saturating_mul(4)))
            .and_then(|n| {
                n.checked_add(worker_metadata_storage(
                    &reference,
                    &descriptor,
                    &report,
                    request,
                )?)
            })
    };
    Reflect::set(
        payload,
        &"analysisRef".into(),
        &js_sys::JSON::parse(&serde_json::to_string(&reference).map_err(|e| e.to_string())?)
            .map_err(js_message)?,
    )
    .map_err(js_message)?;
    Ok(Rc::new(OwnedAnalysis {
        handle,
        reference,
        descriptor,
        source: report,
        scope: request.identity.clone(),
        task: request.request_id.clone(),
        job: request.job_id.clone(),
        retained_bytes,
        positions,
        tags,
    }))
}

fn receive_worker_message(
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
    let mut request = match serde_json::from_str::<WireRequest>(&frame) {
        Ok(request) => request,
        Err(error) => {
            let envelope: RequestEnvelope =
                serde_json::from_str(&frame).map_err(|e| JsValue::from_str(&e.to_string()))?;
            return post_reply(
                scope,
                WireReply {
                    request_id: envelope.request_id,
                    job_id: envelope.job_id,
                    identity: envelope.identity,
                    operation: envelope.operation,
                    outcome: CadReplyOutcome::Failed,
                    error: Some(error.to_string()),
                }
                .into(),
                None,
                Array::new(),
            );
        }
    };
    if let Some(message) = keycap_operation_error(&request.operation, request.keycaps.is_some()) {
        return post_reply(
            scope,
            WireReply {
                request_id: request.request_id,
                job_id: request.job_id,
                identity: request.identity,
                operation: request.operation,
                outcome: CadReplyOutcome::Failed,
                error: Some(message.into()),
            }
            .into(),
            None,
            Array::new(),
        );
    }
    if let Some(keycaps) = request.keycaps.as_ref()
        && (request.prepared.is_some()
            || keycaps.revision != request.identity.revision
            || keycaps.specs.is_empty()
            || keycaps.specs.len() > MAX_KEYCAPS_PER_REQUEST)
    {
        return Err(JsValue::from_str(
            "Keycap CAD input does not match its captured revision",
        ));
    }
    if request.identity.revision > crate::cad_jobs::MAX_CAD_REVISION {
        return post_reply(
            scope,
            WireReply {
                request_id: request.request_id,
                job_id: request.job_id,
                identity: request.identity,
                operation: request.operation,
                outcome: CadReplyOutcome::Failed,
                error: Some("CAD revision is outside the exact JavaScript integer range".into()),
            }
            .into(),
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
    if request.operation == CadOperation::CollisionQuery {
        match decode_query_buffers(&data) {
            Ok(meshes) => request.collision_meshes = Some(meshes),
            Err(error) => {
                return post_reply(
                    scope,
                    WireReply {
                        request_id: request.request_id,
                        job_id: request.job_id,
                        identity: request.identity,
                        operation: request.operation,
                        outcome: CadReplyOutcome::Failed,
                        error: Some(error),
                    }
                    .into(),
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
            .any(|(item, _)| item.request_id == request.request_id)
            || queue.active_request.as_ref() == Some(&request.request_id)
        {
            return Err(JsValue::from_str("duplicate in-flight CAD request id"));
        }
        if queue
            .latest_snapshot
            .as_ref()
            .is_some_and(|old| !same_scope(old, &request.identity))
        {
            queue.evidence.clear();
            queue.evidence_recency.clear();
            queue.retained_bytes = 0;
        }
        queue.latest_snapshot = Some(request.identity.clone());
        queue.pending.push_back((request, input_bytes));
    }
    pump_worker(queue.clone(), scope.clone(), module.clone());
    Ok(())
}

fn keycap_operation_error(operation: &CadOperation, has_keycaps: bool) -> Option<&'static str> {
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
                queue.active_job = Some(request.0.job_id.clone());
                queue.active_request = Some(request.0.request_id.clone());
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
        let completed_job_id = request.job_id.clone();
        let completed_request_id = request.request_id.clone();
        let reply = run_request(&queue, &module, request, bytes).await;
        match reply {
            Ok((wire, result)) => {
                let _ = post_owned_result(&queue, &scope, wire, result);
            }
            Err((wire, reason, cancelled)) => {
                queue
                    .borrow_mut()
                    .remove_task_evidence(&completed_request_id);
                let metadata = WireReply {
                    request_id: wire.request_id,
                    job_id: wire.job_id,
                    identity: wire.identity,
                    operation: wire.operation,
                    outcome: if cancelled {
                        CadReplyOutcome::Cancelled
                    } else {
                        CadReplyOutcome::Failed
                    },
                    error: Some(reason),
                };
                let _ = post_reply(&scope, ReplyWire::from(metadata), None, Array::new());
            }
        }
        next_queue.borrow_mut().active = false;
        next_queue.borrow_mut().active_job = None;
        next_queue.borrow_mut().active_request = None;
        next_queue.borrow_mut().cancelled.remove(&completed_job_id);
        pump_worker(next_queue, next_scope, next_module);
    });
}

#[allow(clippy::result_large_err)]
async fn run_request(
    queue: &Rc<RefCell<WorkerQueue>>,
    module: &JsValue,
    request: WireRequest,
    bytes: Vec<u8>,
) -> Result<(WireReply, JsValue), (WireRequest, String, bool)> {
    let canceled = || queue.borrow().cancelled.contains(&request.job_id);
    if canceled() {
        return Err((request, "CAD job cancelled".into(), true));
    }
    let invoke = |name: &str, args: &[JsValue]| -> Result<JsValue, String> {
        let function = Reflect::get(module, &JsValue::from_str(name))
            .map_err(js_message)?
            .dyn_into::<Function>()
            .map_err(js_message)?;
        let args = Array::from_iter(args.iter());
        function.apply(&JsValue::NULL, &args).map_err(js_message)
    };
    let prepared_value = || -> Result<JsValue, String> {
        let prepared = request
            .prepared
            .as_ref()
            .ok_or_else(|| "CAD request omitted prepared case".to_owned())?;
        let text = serde_json::to_string(prepared).map_err(|error| error.to_string())?;
        js_sys::JSON::parse(&text).map_err(js_message)
    };
    let mut staged = None;
    let mut pinned = Vec::new();
    let result = match request.operation {
        CadOperation::AnalyzeGeneratedBody => {
            let metadata = request.generated_analysis_request.as_ref().ok_or_else(|| {
                (
                    request.clone(),
                    "generated analysis omitted metadata".into(),
                    false,
                )
            })?;
            let body = crate::cad_jobs::validate_generated_analysis_request(
                request.prepared.as_ref(),
                request.identity.revision,
                metadata,
            )
            .map_err(|error| (request.clone(), error, false))?;
            let body_value = js_sys::JSON::parse(
                &serde_json::to_string(body)
                    .map_err(|error| (request.clone(), error.to_string(), false))?,
            )
            .map_err(|error| (request.clone(), js_message(error), false))?;
            let options = Object::new();
            if let Some(target) = metadata.target_deflection_mm {
                Reflect::set(&options, &"targetDeflectionMm".into(), &target.into())
                    .map_err(|error| (request.clone(), js_message(error), false))?;
            }
            let analysis = invoke("analyze_generated_body", &[body_value, options.into()])
                .map_err(|error| (request.clone(), error, false))?;
            staged = Some(
                stage_analysis(
                    queue,
                    module,
                    &request,
                    &analysis,
                    Source::GeneratedBody {
                        body_id: metadata.body_id.clone(),
                        revision: request.identity.revision,
                        geometry_identity: metadata.geometry_identity.clone(),
                    },
                )
                .map_err(|error| (request.clone(), error, false))?,
            );
            if string_field(&analysis, "bodyId").ok().as_deref() != Some(metadata.body_id.as_str())
                || Reflect::get(&analysis, &"revision".into())
                    .ok()
                    .and_then(|v| v.as_f64())
                    != Some(request.identity.revision as f64)
            {
                return Err((
                    request.clone(),
                    "generated provider returned another body or revision".into(),
                    false,
                ));
            }
            Reflect::set(
                &analysis,
                &"geometryIdentity".into(),
                &metadata.geometry_identity.clone().into(),
            )
            .map_err(|error| (request.clone(), js_message(error), false))?;
            let decoded = decode_generated_analysis(analysis.clone())
                .map_err(|error| (request.clone(), error, false))?;
            crate::cad_jobs::validate_generated_analysis(
                metadata,
                request.identity.revision,
                &decoded,
            )
            .map_err(|error| (request.clone(), error, false))?;
            let wrapper = Object::new();
            Reflect::set(
                &wrapper,
                &"revision".into(),
                &(request.identity.revision as f64).into(),
            )
            .map_err(|error| (request.clone(), js_message(error), false))?;
            Reflect::set(&wrapper, &"generatedAnalysis".into(), &analysis)
                .map_err(|error| (request.clone(), js_message(error), false))?;
            wrapper.into()
        }
        CadOperation::Preview | CadOperation::PrintKeycaps => {
            if let Some(keycaps) = request.keycaps.as_ref() {
                // Match React's buildKeycaps adapter: yield between groups so the
                // dedicated worker can receive cancellation before starting more CAD.
                let bodies = Array::new();
                let printable = request.operation == CadOperation::PrintKeycaps;
                let batch_size = if printable {
                    1
                } else {
                    KEYCAPS_PREVIEW_BATCH_SIZE
                };
                for specs in keycaps.specs.chunks(batch_size) {
                    if canceled() {
                        return Err((request, "CAD job cancelled".into(), true));
                    }
                    let input = serde_json::to_string(&serde_json::json!({
                        "revision": keycaps.revision,
                        "specs": specs,
                        "printable": printable,
                    }))
                    .map_err(|error| (request.clone(), error.to_string(), false))?;
                    let input = js_sys::JSON::parse(&input)
                        .map_err(|error| (request.clone(), js_message(error), false))?;
                    let batch = invoke("build_keycaps", &[input])
                        .map_err(|error| (request.clone(), error, false))?;
                    let revision = Reflect::get(&batch, &"revision".into())
                        .map_err(|error| (request.clone(), js_message(error), false))?
                        .as_f64();
                    if revision != Some(request.identity.revision as f64) {
                        return Err((
                            request,
                            "Keycap preview provider returned another revision".into(),
                            false,
                        ));
                    }
                    let batch_bodies = Reflect::get(&batch, &"bodies".into())
                        .map_err(|error| (request.clone(), js_message(error), false))?;
                    for body in Array::from(&batch_bodies).iter() {
                        bodies.push(&body);
                    }
                    yield_to_worker().await;
                }
                if canceled() {
                    return Err((request, "CAD job cancelled".into(), true));
                }
                let result = Object::new();
                Reflect::set(
                    &result,
                    &"revision".into(),
                    &JsValue::from_f64(request.identity.revision as f64),
                )
                .map_err(|error| (request.clone(), js_message(error), false))?;
                Reflect::set(&result, &"bodies".into(), &bodies)
                    .map_err(|error| (request.clone(), js_message(error), false))?;
                result.into()
            } else {
                let prepared = request.prepared.as_ref().ok_or_else(|| {
                    (
                        request.clone(),
                        "CAD request omitted prepared case".to_owned(),
                        false,
                    )
                })?;
                let bodies = Array::new();
                for body in &prepared.bodies {
                    if canceled() {
                        return Err((request, "CAD job cancelled".into(), true));
                    }
                    let body_json = serde_json::to_string(body)
                        .map_err(|error| (request.clone(), error.to_string(), false))?;
                    let body_value = js_sys::JSON::parse(&body_json)
                        .map_err(|error| (request.clone(), js_message(error), false))?;
                    let mut body_geometry = serde_json::to_value(&body.body)
                        .map_err(|error| (request.clone(), error.to_string(), false))?;
                    if let Some(fields) = body_geometry.as_object_mut() {
                        fields.remove("id");
                        fields.remove("name");
                    }
                    let cache_input = serde_json::to_vec(&serde_json::json!({
                        "body": body_geometry,
                        "regions": &body.regions,
                    }))
                    .map_err(|error| (request.clone(), error.to_string(), false))?;
                    let digest = sha256_hex(&cache_input);
                    let key = format!(
                        "m1/{}/{}/{}/{}/{digest}",
                        request.identity.session_epoch,
                        request.identity.document_id,
                        request.identity.board_id,
                        request.identity.instance_id.as_deref().unwrap_or("board")
                    );
                    let progress = Closure::<dyn FnMut(JsValue, JsValue)>::new(|_, _| {});
                    let output = invoke(
                        "preview_body",
                        &[
                            body_value,
                            JsValue::from_str(&key),
                            progress.as_ref().clone(),
                        ],
                    )
                    .map_err(|error| (request.clone(), error, false))?;
                    let object = Object::new();
                    Reflect::set(&object, &"id".into(), &JsValue::from_str(&body.body.id))
                        .map_err(|error| (request.clone(), js_message(error), false))?;
                    Reflect::set(&object, &"name".into(), &JsValue::from_str(&body.body.name))
                        .map_err(|error| (request.clone(), js_message(error), false))?;
                    for field in ["positions", "normals"] {
                        let value = Reflect::get(&output, &field.into())
                            .map_err(|error| (request.clone(), js_message(error), false))?;
                        Reflect::set(&object, &field.into(), &value)
                            .map_err(|error| (request.clone(), js_message(error), false))?;
                    }
                    bodies.push(&object);
                    yield_to_worker().await;
                }
                let result = Object::new();
                Reflect::set(
                    &result,
                    &"revision".into(),
                    &JsValue::from_f64(prepared.revision as f64),
                )
                .map_err(|error| (request.clone(), js_message(error), false))?;
                Reflect::set(&result, &"bodies".into(), &bodies)
                    .map_err(|error| (request.clone(), js_message(error), false))?;
                result.into()
            }
        }
        CadOperation::Exact => invoke(
            "build_assembly",
            &[prepared_value().map_err(|error| (request.clone(), error, false))?],
        )
        .map_err(|error| (request.clone(), error, false))?,
        CadOperation::ExportStep => {
            let prepared = prepared_value().map_err(|error| (request.clone(), error, false))?;
            Reflect::set(&prepared, &"stepOnly".into(), &JsValue::TRUE)
                .map_err(|error| (request.clone(), js_message(error), false))?;
            let bodies = Array::from(
                &Reflect::get(&prepared, &"bodies".into())
                    .map_err(|error| (request.clone(), js_message(error), false))?,
            );
            // This worker owns a fresh module/cache. Empty keys intentionally
            // miss preview meshes while the provider rebuilds exact solids
            // from the same prepared regions without allocating triangulations.
            let keys = Array::new_with_length(bodies.length());
            for index in 0..bodies.length() {
                keys.set(index, JsValue::from_str("m1-independent-step-cache-miss"));
            }
            invoke("export_cached_assembly", &[prepared, keys.into()])
                .map_err(|error| (request.clone(), error, false))?
        }
        CadOperation::ReadStep => {
            if bytes.is_empty() || bytes.len() > MAX_STEP_BYTES {
                return Err((
                    request,
                    "STEP import failed: invalid file size".into(),
                    false,
                ));
            }
            let expected = request.source_sha256.as_deref().unwrap_or_default();
            if !crate::cad_jobs::is_sha256_hex(expected) || sha256_hex(&bytes) != expected {
                return Err((
                    request,
                    "STEP source failed SHA-256 verification".into(),
                    false,
                ));
            }
            let step_options = request
                .step_import_options
                .unwrap_or_default()
                .validate()
                .map_err(|error| (request.clone(), error, false))?;
            let options = Object::new();
            Reflect::set(
                &options,
                &"sourceSha256".into(),
                &JsValue::from_str(expected),
            )
            .map_err(|error| (request.clone(), js_message(error), false))?;
            if let Some(budget) = step_options.preview_repair_budget_mm {
                Reflect::set(
                    &options,
                    &"previewRepairBudgetMm".into(),
                    &JsValue::from_f64(budget),
                )
                .map_err(|error| (request.clone(), js_message(error), false))?;
            }
            if let Some(target) = step_options.target_deflection_mm {
                Reflect::set(&options, &"targetDeflectionMm".into(), &target.into())
                    .map_err(|error| (request.clone(), js_message(error), false))?;
            }
            let result = invoke(
                "read_step_model",
                &[Uint8Array::from(bytes.as_slice()).into(), options.into()],
            )
            .map_err(|error| (request.clone(), error, false))?;
            let vendor = Reflect::get(&result, &"vendorStep".into())
                .map_err(|e| (request.clone(), js_message(e), false))?;
            staged = Some(
                stage_analysis(
                    queue,
                    module,
                    &request,
                    &vendor,
                    Source::VendorStep {
                        source_sha256: expected.into(),
                        import_options: step_options.into(),
                    },
                )
                .map_err(|error| (request.clone(), error, false))?,
            );
            exact_fields(&result, &["mesh", "bounds", "vendorStep"])
                .map_err(|error| (request.clone(), error, false))?;
            let decoded =
                decode_vendor_step(vendor).map_err(|error| (request.clone(), error, false))?;
            let mesh = decode_mesh(
                Reflect::get(&result, &"mesh".into())
                    .map_err(|e| (request.clone(), js_message(e), false))?,
            )
            .map_err(|error| (request.clone(), error, false))?;
            let bounds = strict_metadata(
                Reflect::get(&result, &"bounds".into())
                    .map_err(|e| (request.clone(), js_message(e), false))?,
            )
            .map_err(|error| (request.clone(), error, false))?;
            crate::cad_jobs::validate_vendor_step(expected, step_options, &mesh, &bounds, &decoded)
                .map_err(|error| (request.clone(), error, false))?;
            result
        }
        CadOperation::CollisionQuery => {
            let query = request.collision_query.as_ref().ok_or_else(|| {
                (
                    request.clone(),
                    "collision query omitted metadata".into(),
                    false,
                )
            })?;
            validate_collision_query(query).map_err(|error| (request.clone(), error, false))?;
            let meshes = request.collision_meshes.as_ref().ok_or_else(|| {
                (
                    request.clone(),
                    "collision query omitted mesh buffers".into(),
                    false,
                )
            })?;
            validate_collision_mesh_pair(meshes)
                .map_err(|error| (request.clone(), error, false))?;
            let query_json = serde_json::to_string(query)
                .map_err(|error| (request.clone(), error.to_string(), false))?;
            let query_value = js_sys::JSON::parse(&query_json)
                .map_err(|error| (request.clone(), js_message(error), false))?;
            let left = queue
                .borrow_mut()
                .lookup(
                    &query.left,
                    &request.identity,
                    &meshes.left_positions_mm_f64,
                    &meshes.left_triangle_face_occurrence_indices,
                )
                .map_err(|error| (request.clone(), error, false))?;
            let right = queue
                .borrow_mut()
                .lookup(
                    &query.right,
                    &request.identity,
                    &meshes.right_positions_mm_f64,
                    &meshes.right_triangle_face_occurrence_indices,
                )
                .map_err(|error| (request.clone(), error, false))?;
            let generation = queue.borrow().generation.clone();
            let expected = qualification::expected_binding(query, &request.identity, &generation)
                .map_err(|error| (request.clone(), error, false))?;
            let result = if let (Some(left), Some(right)) = (left, right) {
                let buffers = Object::new();
                for (key, value) in [
                    (
                        "leftPositions",
                        Float64Array::from(meshes.left_positions_mm_f64.as_slice()).into(),
                    ),
                    (
                        "leftTags",
                        Uint32Array::from(meshes.left_triangle_face_occurrence_indices.as_slice())
                            .into(),
                    ),
                    (
                        "rightPositions",
                        Float64Array::from(meshes.right_positions_mm_f64.as_slice()).into(),
                    ),
                    (
                        "rightTags",
                        Uint32Array::from(meshes.right_triangle_face_occurrence_indices.as_slice())
                            .into(),
                    ),
                ] {
                    Reflect::set(&buffers, &key.into(), &value)
                        .map_err(|e| (request.clone(), js_message(e), false))?;
                }
                let context = js_sys::JSON::parse(&serde_json::json!({"snapshot": qualification::Snapshot::from(&request.identity), "workerGeneration": generation}).to_string())
                    .map_err(|e| (request.clone(), js_message(e), false))?;
                let result = invoke(
                    "query_qualified_collision_mesh_pair",
                    &[
                        buffers.into(),
                        query_value,
                        context,
                        left.handle.0.clone(),
                        right.handle.0.clone(),
                    ],
                )
                .map_err(|error| (request.clone(), error, false))?;
                let report: qualification::QueryQualification = strict_metadata(
                    Reflect::get(&result, &"queryQualification".into())
                        .map_err(|e| (request.clone(), js_message(e), false))?,
                )
                .map_err(|error| (request.clone(), error, false))?;
                qualification::validate_query_qualification(
                    report,
                    &expected,
                    &query.qualification_limits,
                    meshes,
                    Some([&left.source, &right.source]),
                )
                .map_err(|error| (request.clone(), error, false))?;
                pinned.extend([left, right]);
                result
            } else {
                let raw = invoke(
                    "query_collision_mesh_pair",
                    &[
                        Float64Array::from(meshes.left_positions_mm_f64.as_slice()).into(),
                        Uint32Array::from(meshes.left_triangle_face_occurrence_indices.as_slice())
                            .into(),
                        Float64Array::from(meshes.right_positions_mm_f64.as_slice()).into(),
                        Uint32Array::from(meshes.right_triangle_face_occurrence_indices.as_slice())
                            .into(),
                        raw_query_metadata(query)
                            .map_err(|error| (request.clone(), error, false))?,
                    ],
                )
                .map_err(|error| (request.clone(), error, false))?;
                let result = Object::new();
                Reflect::set(&result, &"observation".into(), &raw)
                    .map_err(|e| (request.clone(), js_message(e), false))?;
                let unknown =
                    qualification::unavailable(expected, query.qualification_limits.clone());
                let report = js_sys::JSON::parse(
                    &serde_json::to_string(&unknown)
                        .map_err(|e| (request.clone(), e.to_string(), false))?,
                )
                .map_err(|e| (request.clone(), js_message(e), false))?;
                Reflect::set(&result, &"queryQualification".into(), &report)
                    .map_err(|e| (request.clone(), js_message(e), false))?;
                result.into()
            };
            let object = result.dyn_ref::<Object>().ok_or_else(|| {
                (
                    request.clone(),
                    "qualified result must be an object".into(),
                    false,
                )
            })?;
            if Object::keys(object).length() != 2
                || !Object::has_own(object, &"observation".into())
                || !Object::has_own(object, &"queryQualification".into())
            {
                return Err((
                    request.clone(),
                    "qualified provider result has unexpected keys".into(),
                    false,
                ));
            }
            result
        }
    };
    // Let queued cancel messages run after synchronous CAD calls before deciding
    // whether their result may cross the worker boundary.
    yield_to_worker().await;
    if canceled() {
        return Err((
            request,
            "CAD job cancelled after the synchronous kernel call".into(),
            true,
        ));
    }
    if queue
        .borrow()
        .latest_snapshot
        .as_ref()
        .is_some_and(|latest| latest != &request.identity)
    {
        return Err((
            request,
            "CAD result became stale after kernel yield".into(),
            false,
        ));
    }
    if let Some(entry) = staged {
        queue
            .borrow_mut()
            .retain_analysis(entry)
            .map_err(|e| (request.clone(), e, false))?;
    }
    drop(pinned);
    let result = match request.operation {
        CadOperation::ReadStep => {
            let wrapper = Object::new();
            Reflect::set(
                &wrapper,
                &"revision".into(),
                &JsValue::from_f64(request.identity.revision as f64),
            )
            .map_err(|error| (request.clone(), js_message(error), false))?;
            for field in ["mesh", "bounds", "vendorStep"] {
                let value = Reflect::get(&result, &field.into())
                    .map_err(|error| (request.clone(), js_message(error), false))?;
                Reflect::set(&wrapper, &field.into(), &value)
                    .map_err(|error| (request.clone(), js_message(error), false))?;
            }
            wrapper.into()
        }
        CadOperation::CollisionQuery => {
            let wrapper = Object::new();
            Reflect::set(
                &wrapper,
                &"revision".into(),
                &JsValue::from_f64(request.identity.revision as f64),
            )
            .map_err(|error| (request.clone(), js_message(error), false))?;
            for (target, source) in [
                ("collision", "observation"),
                ("queryQualification", "queryQualification"),
            ] {
                let value = Reflect::get(&result, &source.into())
                    .map_err(|e| (request.clone(), js_message(e), false))?;
                Reflect::set(&wrapper, &target.into(), &value)
                    .map_err(|e| (request.clone(), js_message(e), false))?;
            }
            wrapper.into()
        }
        _ => result,
    };
    let wire = WireReply {
        request_id: request.request_id.clone(),
        job_id: request.job_id.clone(),
        identity: request.identity.clone(),
        operation: request.operation,
        outcome: CadReplyOutcome::Completed,
        error: None,
    };
    Ok((wire, result))
}

async fn yield_to_worker() {
    if let Some(promise) =
        Function::new_no_args("return new Promise(resolve => setTimeout(resolve, 0))")
            .call0(&JsValue::NULL)
            .ok()
            .and_then(|value| value.dyn_into::<Promise>().ok())
    {
        let _ = JsFuture::from(promise).await;
    }
}

fn post_result(
    scope: &DedicatedWorkerGlobalScope,
    wire: WireReply,
    result: JsValue,
) -> Result<(), JsValue> {
    let transfers = result_transfers(&wire.operation, &result)?;
    let metadata =
        serde_json::to_string(&wire).map_err(|error| JsValue::from_str(&error.to_string()))?;
    post_reply(scope, ReplyWire::from(metadata), Some(result), transfers)
}

fn post_owned_result(
    queue: &Rc<RefCell<WorkerQueue>>,
    scope: &DedicatedWorkerGlobalScope,
    wire: WireReply,
    result: JsValue,
) -> Result<(), JsValue> {
    if let Err(error) = post_result(scope, wire.clone(), result) {
        queue.borrow_mut().remove_task_evidence(&wire.request_id);
        return post_reply(
            scope,
            WireReply {
                outcome: CadReplyOutcome::Failed,
                error: Some(js_message(error)),
                ..wire
            }
            .into(),
            None,
            Array::new(),
        );
    }
    Ok(())
}

// Internal adapter keeps the worker's error and success paths on the same frame format.
struct ReplyWire(String);
impl From<String> for ReplyWire {
    fn from(value: String) -> Self {
        Self(value)
    }
}
impl From<WireReply> for ReplyWire {
    fn from(value: WireReply) -> Self {
        Self(serde_json::to_string(&value).unwrap_or_else(|_| "{}".into()))
    }
}
fn post_reply(
    scope: &DedicatedWorkerGlobalScope,
    wire: ReplyWire,
    result: Option<JsValue>,
    transfers: Array,
) -> Result<(), JsValue> {
    let message = Object::new();
    Reflect::set(&message, &"kind".into(), &"reply".into())?;
    Reflect::set(&message, &"frame".into(), &JsValue::from_str(&wire.0))?;
    if let Some(result) = result {
        Reflect::set(&message, &"result".into(), &result)?;
    }
    if transfers.length() == 0 {
        scope.post_message(&message)
    } else {
        scope.post_message_with_transfer(&message, &transfers)
    }
}

fn result_transfers(operation: &CadOperation, result: &JsValue) -> Result<Array, JsValue> {
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

fn worker_message(kind: &str, message: &str, _unused: Option<JsValue>) -> JsValue {
    let value = Object::new();
    let _ = Reflect::set(&value, &"kind".into(), &JsValue::from_str(kind));
    let _ = Reflect::set(&value, &"frame".into(), &JsValue::from_str(message));
    value.into()
}
fn js_message(value: JsValue) -> String {
    value
        .as_string()
        .unwrap_or_else(|| "JavaScript CAD provider failed".into())
}

#[cfg(test)]
mod generated_analysis_tests {
    use super::*;
    use wasm_bindgen_test::wasm_bindgen_test;

    #[wasm_bindgen_test]
    fn generated_analysis_shared_contract_decodes_and_rejects_mutated_identity_provenance_and_buffers()
     {
        use crate::cad_jobs::*;
        let fixture: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../cad/wasm/tests/fixtures/generated-analysis-contract.json"
        ))
        .unwrap();
        let body: boardstudio_core::model::PreparedCaseIR =
            serde_json::from_value(fixture["preparedBody"].clone()).unwrap();
        let geometry_identity = generated_geometry_identity(&body).unwrap();
        assert!(
            geometry_identity.starts_with(
                fixture["qualificationV2"]["generatedIdentityPrefix"]
                    .as_str()
                    .unwrap()
            )
        );
        let metadata = CadGeneratedAnalysisRequest {
            body_id: body.body.id.clone(),
            geometry_identity: geometry_identity.clone(),
            target_deflection_mm: None,
        };
        let identity = CadSnapshotIdentity {
            token: 1,
            session_epoch: 2,
            document_id: "fixture".into(),
            board_id: body.body.board_id.clone(),
            instance_id: None,
            revision: body.revision,
        };
        let request = CadRequest {
            request_id: "fixture-analysis".into(),
            job_id: "fixture-analysis".into(),
            identity: identity.clone(),
            operation: CadOperation::AnalyzeGeneratedBody,
            prepared: Some(boardstudio_core::model::PreparedCaseAssemblyIR {
                revision: body.revision,
                bodies: vec![body],
            }),
            generated_analysis_request: Some(metadata),
            source_sha256: None,
            step_import_options: None,
            collision_query: None,
            collision_meshes: None,
            input_bytes: vec![],
        };
        let decode_with_target = |payload: &serde_json::Value,
                                  hash: &str,
                                  typed: bool,
                                  target: Option<f64>|
         -> Result<CadResult, String> {
            let mut request = request.clone();
            request
                .generated_analysis_request
                .as_mut()
                .unwrap()
                .target_deflection_mm = target;
            let direct = js_sys::JSON::parse(&payload.to_string()).map_err(js_message)?;
            validate_generated_payload_fields(&direct, false)?;
            if typed {
                Reflect::set(
                    &direct,
                    &"surfacePositionsMmF64".into(),
                    &Float64Array::new(
                        &Reflect::get(&direct, &"surfacePositionsMmF64".into())
                            .map_err(js_message)?,
                    ),
                )
                .map_err(js_message)?;
                Reflect::set(
                    &direct,
                    &"triangleFaceOccurrenceIndices".into(),
                    &Uint32Array::new(
                        &Reflect::get(&direct, &"triangleFaceOccurrenceIndices".into())
                            .map_err(js_message)?,
                    ),
                )
                .map_err(js_message)?;
            }
            Reflect::set(&direct, &"geometryIdentity".into(), &hash.into()).map_err(js_message)?;
            let wrapper = Object::new();
            Reflect::set(
                &wrapper,
                &"revision".into(),
                &(identity.revision as f64).into(),
            )
            .map_err(js_message)?;
            Reflect::set(&wrapper, &"generatedAnalysis".into(), &direct).map_err(js_message)?;
            let result = decode_result(wrapper.into())?;
            validate_reply(
                &request,
                CadReply {
                    request_id: request.request_id.clone(),
                    job_id: request.job_id.clone(),
                    identity: identity.clone(),
                    operation: request.operation,
                    outcome: CadReplyOutcome::Completed,
                    result: Some(result),
                    error: None,
                },
                &identity,
            )
            .map_err(|error| format!("{error:?}"))
        };
        let decode = |payload: &serde_json::Value, hash: &str, typed: bool| {
            decode_with_target(payload, hash, typed, None)
        };
        let payload = &fixture["directPayload"];
        let mut accepted_bad_settings = Vec::new();
        for (target, returned) in [
            (None, 1.0),
            (Some(1.0), 0.5),
            (Some(0.01), 0.02),
            (Some(0.01), 0.005),
        ] {
            let mut malformed = payload.clone();
            malformed["deflectionMm"] = returned.into();
            if decode_with_target(&malformed, &geometry_identity, true, target).is_ok() {
                accepted_bad_settings.push((target, returned));
            }
        }
        assert!(
            accepted_bad_settings.is_empty(),
            "accepted wrong selected deflections: {accepted_bad_settings:?}"
        );
        for (target, selected) in [(None, 0.1), (Some(1.0), 0.1), (Some(0.01), 0.01)] {
            let mut valid = payload.clone();
            valid["deflectionMm"] = selected.into();
            let accepted = decode_with_target(&valid, &geometry_identity, true, target).unwrap();
            assert_eq!(
                accepted.generated_analysis.unwrap().analysis_error_bound_mm,
                None
            );
        }
        let result = decode(payload, &geometry_identity, true).unwrap();
        assert!(result.vendor_step.is_none() && result.mesh.is_none() && result.bodies.is_empty());
        let analysis = result.generated_analysis.unwrap();
        assert_eq!(
            analysis.analysis_error_bound_mm, None,
            "deflection and closed topology are not certificates"
        );
        assert_eq!(
            analysis.deflection_mm,
            payload["deflectionMm"].as_f64().unwrap()
        );
        for field in [
            "bodyId",
            "revision",
            "triangleFaceOccurrenceIndices",
            "faceOccurrences",
            "surfacePositionsMmF64",
        ] {
            let mut bad = payload.clone();
            match field {
                "bodyId" => bad[field] = "another-body".into(),
                "revision" => bad[field] = (identity.revision + 1).into(),
                "triangleFaceOccurrenceIndices" => bad[field][0] = u32::MAX.into(),
                "faceOccurrences" => bad[field][0]["sourceFaceIndex"] = u32::MAX.into(),
                _ => {
                    bad[field].as_array_mut().unwrap().pop();
                }
            }
            assert!(
                decode(&bad, &geometry_identity, true).is_err(),
                "reject mutated {field}"
            );
        }
        assert!(decode(payload, "another-geometry", true).is_err());
        assert!(
            decode(payload, &geometry_identity, false).is_err(),
            "ordinary JSON buffers cannot cross the worker boundary"
        );
        let mut bad = payload.clone();
        bad["sourceSha256"] = "a".repeat(64).into();
        assert!(
            decode(&bad, &geometry_identity, true).is_err(),
            "generated output is separate from STEP identity"
        );
    }

    #[wasm_bindgen_test]
    fn generated_analysis_transfers_the_original_typed_buffers() {
        let positions = Float64Array::from(&[0.0, 0.0, 0.0][..]);
        let tags = Uint32Array::from(&[0][..]);
        let analysis = Object::new();
        Reflect::set(&analysis, &"surfacePositionsMmF64".into(), &positions).unwrap();
        Reflect::set(&analysis, &"triangleFaceOccurrenceIndices".into(), &tags).unwrap();
        let result = Object::new();
        Reflect::set(&result, &"generatedAnalysis".into(), &analysis).unwrap();
        let transfers = result_transfers(&CadOperation::AnalyzeGeneratedBody, &result).unwrap();
        assert_eq!(transfers.length(), 2);
        assert!(
            Object::is(&transfers.get(0), &positions.buffer()),
            "transfer the actual result buffer"
        );
        assert!(
            Object::is(&transfers.get(1), &tags.buffer()),
            "transfer the actual provenance buffer"
        );
    }

    #[wasm_bindgen_test]
    async fn generated_analysis_worker_selects_one_body_checks_identity_and_recovers_after_failure()
    {
        let body: boardstudio_core::model::PreparedCaseIR = serde_json::from_value(serde_json::json!({
            "revision":7,"body":{"id":"plate","name":"Plate","boardId":"board","kind":"plate","thickness":2,"clearance":0},"regions":[]
        })).unwrap();
        let metadata = crate::cad_jobs::CadGeneratedAnalysisRequest {
            body_id: "plate".into(),
            geometry_identity: crate::cad_jobs::generated_geometry_identity(&body).unwrap(),
            target_deflection_mm: Some(0.01),
        };
        let mut request = WireRequest {
            request_id: "analysis-1".into(),
            job_id: "analysis-1".into(),
            identity: CadSnapshotIdentity {
                token: 3,
                session_epoch: 1,
                document_id: "doc".into(),
                board_id: "board".into(),
                instance_id: None,
                revision: 7,
            },
            operation: CadOperation::AnalyzeGeneratedBody,
            prepared: Some(boardstudio_core::model::PreparedCaseAssemblyIR {
                revision: 7,
                bodies: vec![body],
            }),
            generated_analysis_request: Some(metadata.clone()),
            source_sha256: None,
            step_import_options: None,
            collision_query: None,
            collision_meshes: None,
            keycaps: None,
            input_bytes: vec![],
        };
        let queue = Rc::new(RefCell::new(WorkerQueue::default()));
        let module = super::qualification_tests::controlled_provider(true);
        let (_, result) = run_request(&queue, &module, request.clone(), vec![])
            .await
            .unwrap_or_else(|(_, error, _)| panic!("{error}"));
        let decoded = decode_result(result).unwrap();
        let analysis = decoded.generated_analysis.unwrap();
        assert_eq!(analysis.geometry_identity, metadata.geometry_identity);
        assert_eq!(analysis.deflection_mm, 0.01);
        assert_eq!(analysis.analysis_error_bound_mm, None);
        assert_eq!(analysis.source_faces[0].face_index, 0);
        request
            .generated_analysis_request
            .as_mut()
            .unwrap()
            .geometry_identity
            .push('x');
        let failed = run_request(&queue, &module, request.clone(), vec![]).await;
        assert!(matches!(failed, Err((_, reason, false)) if reason.contains("identity")));
        request.generated_analysis_request = Some(metadata);
        queue.borrow_mut().cancelled.insert(request.job_id.clone());
        assert!(matches!(
            run_request(&queue, &module, request.clone(), vec![]).await,
            Err((_, _, true))
        ));
        queue.borrow_mut().cancelled.clear();
        assert!(
            run_request(&queue, &module, request, vec![]).await.is_ok(),
            "failed/cancelled work must not poison the worker"
        );
    }

    #[wasm_bindgen_test]
    async fn raw_worker_preview_without_prepared_or_keycap_input_fails_cleanly_and_worker_recovers()
    {
        let queue = Rc::new(RefCell::new(WorkerQueue::default()));
        let module = Rc::new(super::qualification_tests::controlled_provider(true));
        let replies: Rc<RefCell<Vec<WireReply>>> = Rc::new(RefCell::new(Vec::new()));
        let captured = replies.clone();
        let post_message = Closure::<dyn FnMut(JsValue, JsValue)>::new(
            move |message: JsValue, _transfers: JsValue| {
                if Reflect::get(&message, &"kind".into())
                    .ok()
                    .and_then(|v| v.as_string())
                    .as_deref()
                    == Some("reply")
                    && let Some(frame) = Reflect::get(&message, &"frame".into())
                        .ok()
                        .and_then(|v| v.as_string())
                {
                    captured
                        .borrow_mut()
                        .push(serde_json::from_str::<WireReply>(&frame).unwrap());
                }
            },
        );
        let target = Object::new();
        Reflect::set(&target, &"postMessage".into(), post_message.as_ref()).unwrap();
        let scope: DedicatedWorkerGlobalScope = target.unchecked_into();

        let identity = CadSnapshotIdentity {
            token: 19,
            session_epoch: 4,
            document_id: "raw-preview-doc".into(),
            board_id: "board-a".into(),
            instance_id: Some("inst-1".into()),
            revision: 7,
        };
        let missing_frame = serde_json::json!({
            "requestId": "raw-preview-missing",
            "jobId": "job-missing",
            "identity": identity,
            "operation": "preview",
            "prepared": null
        })
        .to_string();
        receive_worker_message(
            &queue,
            &scope,
            &module,
            worker_message("request", &missing_frame, None),
        )
        .unwrap();
        yield_to_worker().await;
        yield_to_worker().await;

        assert!(
            !queue.borrow().active,
            "worker queue must settle after failure"
        );
        assert!(queue.borrow().pending.is_empty());
        assert_eq!(queue.borrow().active_request, None);
        assert_eq!(queue.borrow().active_job, None);
        assert_eq!(replies.borrow().len(), 1);
        {
            let failed = &replies.borrow()[0];
            assert_eq!(failed.request_id, "raw-preview-missing");
            assert_eq!(failed.job_id, "job-missing");
            assert_eq!(failed.identity, identity);
            assert_eq!(failed.operation, CadOperation::Preview);
            assert_eq!(failed.outcome, CadReplyOutcome::Failed);
            assert!(
                failed
                    .error
                    .as_deref()
                    .is_some_and(|msg| msg.contains("prepared")),
                "expected descriptive missing-prepared error, got {:?}",
                failed.error
            );
        }

        let body: boardstudio_core::model::PreparedCaseIR = serde_json::from_value(
            serde_json::json!({
                "revision": 7,
                "body": {"id": "plate", "name": "Plate", "boardId": "board-a", "kind": "plate", "thickness": 2, "clearance": 0},
                "regions": []
            }),
        )
        .unwrap();
        let case_frame = serde_json::json!({
            "requestId": "raw-preview-case",
            "jobId": "job-case",
            "identity": identity,
            "operation": "preview",
            "prepared": { "revision": 7, "bodies": [body] }
        })
        .to_string();
        receive_worker_message(
            &queue,
            &scope,
            &module,
            worker_message("request", &case_frame, None),
        )
        .unwrap();
        for _ in 0..4 {
            yield_to_worker().await;
        }
        assert!(!queue.borrow().active);
        assert_eq!(replies.borrow().len(), 2);
        assert_eq!(replies.borrow()[1].request_id, "raw-preview-case");
        assert_eq!(replies.borrow()[1].outcome, CadReplyOutcome::Completed);

        let keycap = boardstudio_core::model::KeycapSpec {
            id: "k1".into(),
            reference: "A1".into(),
            profile: boardstudio_core::model::KeycapProfile::Cherry,
            mount: boardstudio_core::model::KeycapMount::Mx,
            row: 1,
            size: boardstudio_core::model::Vec2 { x: 18.0, y: 18.0 },
            top_size: boardstudio_core::model::Vec2 { x: 12.0, y: 12.0 },
            height: 8.0,
            tilt: 0.0,
            dish_depth: 0.0,
            spherical: false,
            wall_thickness: 1.2,
            pose: boardstudio_core::model::Pose2 {
                at: boardstudio_core::model::Vec2 { x: 0.0, y: 0.0 },
                rotation: 0.0,
            },
            side: boardstudio_core::model::Side::Front,
            z: 8.4,
            travel: 4.0,
            legend: String::new(),
            color: "#b0b5bd".into(),
            legend_color: "#202124".into(),
        };
        let keycap_frame = serde_json::json!({
            "requestId": "raw-preview-keycaps",
            "jobId": "job-keycaps",
            "identity": identity,
            "operation": "preview",
            "keycaps": { "revision": 7, "specs": [keycap] }
        })
        .to_string();
        receive_worker_message(
            &queue,
            &scope,
            &module,
            worker_message("request", &keycap_frame, None),
        )
        .unwrap();
        for _ in 0..4 {
            yield_to_worker().await;
        }
        assert!(!queue.borrow().active);
        assert_eq!(replies.borrow().len(), 3);
        assert_eq!(replies.borrow()[2].request_id, "raw-preview-keycaps");
        assert_eq!(replies.borrow()[2].outcome, CadReplyOutcome::Completed);
    }
}

#[cfg(test)]
#[path = "cad_worker_qualification_tests.rs"]
mod qualification_tests;
