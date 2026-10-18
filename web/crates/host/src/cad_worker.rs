//! Dedicated host for the separately built Monstertruck CAD WASM package.
use crate::cad_jobs::{
    CadBodyMesh, CadCollisionMeshPair, CadCollisionObservation, CadCollisionQuery, CadMesh,
    CadOperation, CadReply, CadReplyOutcome, CadRequest, CadResult, CadSnapshotIdentity,
    CadStepAccuracy, CadStepFaceOccurrence, CadStepImportOptions, CadStepLineOccurrence,
    CadStepPrecision, CadStepSourceFace, CadVendorStep, MAX_COLLISION_QUERY_BYTES,
    validate_collision_mesh_pair, validate_collision_query,
};
use boardstudio_core::model::KeycapSpec;
use futures_channel::oneshot;
use js_sys::{
    Array, Float32Array, Float64Array, Function, Object, Promise, Reflect, Uint8Array, Uint32Array,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
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

#[derive(Serialize, Deserialize)]
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
#[serde(rename_all = "camelCase")]
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
    keycaps: Option<KeycapsInput>,
    #[serde(skip)]
    collision_meshes: Option<CadCollisionMeshPair>,
    #[serde(skip)]
    input_bytes: Vec<u8>,
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
        self.request_frame_with_payloads(
            &request.request_id,
            &frame,
            &request.input_bytes,
            request.collision_meshes.as_ref(),
        )
        .await
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
                .is_some_and(crate::cad_jobs::valid_source_sha256) =>
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
    let revision = Reflect::get(&value, &"revision".into())
        .map_err(js_message)?
        .as_f64()
        .ok_or_else(|| "CAD result omitted revision".to_string())? as u64;
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
        revision,
        step,
        mesh,
        bodies,
        bounds,
        vendor_step,
        collision,
    })
}

fn decode_vendor_step(value: JsValue) -> Result<CadVendorStep, String> {
    let source_sha256 = string_field(&value, "sourceSha256")?;
    let options_value = Reflect::get(&value, &"importOptions".into()).map_err(js_message)?;
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
                    model_id: "m1".into(),
                    source_sha256: sha_a.into(),
                    import_options: CadStepImportOptions::default(),
                    row_transform_f64: id_mat,
                },
                right: CadCollisionOperand {
                    model_id: "m2".into(),
                    source_sha256: sha_a.into(),
                    import_options: CadStepImportOptions::default(),
                    row_transform_f64: id_mat,
                },
                clearance_threshold_mm: 0.5,
                requested_accuracy_mm: None,
            }),
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

#[derive(Default)]
struct WorkerQueue {
    pending: VecDeque<(WireRequest, Vec<u8>)>,
    cancelled: BTreeSet<String>,
    active: bool,
    active_job: Option<String>,
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
            let mut queue = queue.borrow_mut();
            if queue.active_job.as_deref() == Some(job_id.as_str())
                || queue
                    .pending
                    .iter()
                    .any(|(request, _)| request.job_id == job_id)
            {
                queue.cancelled.insert(job_id);
            }
        }
        return Ok(());
    }
    if kind != "request" {
        return Err(JsValue::from_str("unknown CAD worker message kind"));
    }
    let frame = Reflect::get(&data, &"frame".into())?
        .as_string()
        .ok_or_else(|| JsValue::from_str("CAD request omitted frame"))?;
    let mut request = serde_json::from_str::<WireRequest>(&frame)
        .map_err(|error| JsValue::from_str(&error.to_string()))?;
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
        let get_buf = |name: &str| -> Result<JsValue, JsValue> {
            Reflect::get(&data, &name.into()).and_then(|v| {
                if v.is_undefined() || v.is_null() {
                    Err(JsValue::from_str(&format!(
                        "collision request omitted {name}"
                    )))
                } else {
                    Ok(v)
                }
            })
        };
        let left_pos = Float64Array::new(&get_buf("leftPositions")?);
        let left_tags = Uint32Array::new(&get_buf("leftTags")?);
        let right_pos = Float64Array::new(&get_buf("rightPositions")?);
        let right_tags = Uint32Array::new(&get_buf("rightTags")?);
        let total_bytes = (left_pos.byte_length() as usize)
            .saturating_add(left_tags.byte_length() as usize)
            .saturating_add(right_pos.byte_length() as usize)
            .saturating_add(right_tags.byte_length() as usize);
        if total_bytes <= MAX_COLLISION_QUERY_BYTES {
            request.collision_meshes = Some(CadCollisionMeshPair {
                left_positions_mm_f64: left_pos.to_vec(),
                left_triangle_face_occurrence_indices: left_tags.to_vec(),
                right_positions_mm_f64: right_pos.to_vec(),
                right_triangle_face_occurrence_indices: right_tags.to_vec(),
            });
        }
    }
    let input_bytes = std::mem::take(&mut request.input_bytes);
    {
        let mut queue = queue.borrow_mut();
        if queue
            .pending
            .iter()
            .any(|(item, _)| item.request_id == request.request_id)
        {
            return Err(JsValue::from_str("duplicate in-flight CAD request id"));
        }
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
        CadOperation::Exact | CadOperation::ReadStep | CadOperation::CollisionQuery => {
            Some("Keycap CAD input is only valid for preview")
        }
    }
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
        let reply = run_request(&queue, &module, request, bytes).await;
        match reply {
            Ok((wire, result)) => {
                let _ = post_result(&scope, wire, result);
            }
            Err((wire, reason, cancelled)) => {
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
    let result = match request.operation {
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
                let prepared = request.prepared.as_ref().expect("request validated");
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
            if !crate::cad_jobs::valid_source_sha256(expected) || sha256_hex(&bytes) != expected {
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
            invoke(
                "read_step_model",
                &[Uint8Array::from(bytes.as_slice()).into(), options.into()],
            )
            .map_err(|error| (request.clone(), error, false))?
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
            invoke(
                "query_collision_mesh_pair",
                &[
                    Float64Array::from(meshes.left_positions_mm_f64.as_slice()).into(),
                    Uint32Array::from(meshes.left_triangle_face_occurrence_indices.as_slice())
                        .into(),
                    Float64Array::from(meshes.right_positions_mm_f64.as_slice()).into(),
                    Uint32Array::from(meshes.right_triangle_face_occurrence_indices.as_slice())
                        .into(),
                    query_value,
                ],
            )
            .map_err(|error| (request.clone(), error, false))?
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
            Reflect::set(&wrapper, &"collision".into(), &result)
                .map_err(|error| (request.clone(), js_message(error), false))?;
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
        Uint8Array::new(value).buffer()
    } else if value.is_instance_of::<Float64Array>() {
        Float64Array::new(value).buffer()
    } else if value.is_instance_of::<Uint32Array>() {
        Uint32Array::new(value).buffer()
    } else {
        Float32Array::new(value).buffer()
    };
    transfers.push(&buffer);
    Ok(())
}

fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
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
