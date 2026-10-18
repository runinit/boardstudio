//! The only JavaScript evidence authority is a module-issued opaque owner.
use super::{collision, js_error, set};
use boardstudio_cad::qualification::{
    self as q, AnalysisMesh, PreparedAnalysisMesh, PreparedAnalysisView,
};
use js_sys::{Array, ArrayBuffer, Float64Array, Object, Reflect, Uint32Array};
use serde::{Deserialize, Serialize};
use wasm_bindgen::{prelude::*, JsCast};

const MAX_REVISION: u64 = 9_007_199_254_740_991;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ImportOptions {
    preview_repair_budget_mm: Option<f64>,
    target_deflection_mm: Option<f64>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", deny_unknown_fields)]
enum Source {
    #[serde(rename = "vendor-step", rename_all = "camelCase")]
    Vendor {
        source_sha256: String,
        import_options: ImportOptions,
    },
    #[serde(rename = "generated-body", rename_all = "camelCase")]
    Generated {
        body_id: String,
        revision: u64,
        geometry_identity: String,
    },
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct AnalysisRef {
    worker_generation: String,
    mesh_identity: String,
    source: Source,
    derivation_identity: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Snapshot {
    token: String,
    session_epoch: String,
    document_id: String,
    board_id: String,
    instance_id: Option<String>,
    revision: u64,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Context {
    snapshot: Snapshot,
    worker_generation: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Operand {
    model_id: String,
    source_sha256: Option<String>,
    generated_geometry_identity: Option<String>,
    import_options: collision::CadStepImportOptions,
    row_transform_f64: [f64; 16],
    analysis_ref: AnalysisRef,
    analysis_mesh: AnalysisMesh,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Metadata {
    left: Operand,
    right: Operand,
    clearance_threshold_mm: f64,
    requested_accuracy_mm: Option<f64>,
    qualification_limits: q::Limits,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Binding<'a> {
    snapshot_revision: u64,
    snapshot: &'a Snapshot,
    worker_generation: &'a str,
    left_model_id: &'a str,
    right_model_id: &'a str,
    left_mesh_identity: &'a str,
    right_mesh_identity: &'a str,
    left_analysis: &'a AnalysisRef,
    right_analysis: &'a AnalysisRef,
    left_mesh: &'a AnalysisMesh,
    right_mesh: &'a AnalysisMesh,
    left_row_transform_f64: [f64; 16],
    right_row_transform_f64: [f64; 16],
    clearance_threshold_mm: f64,
    requested_accuracy_mm: Option<f64>,
}

// Separate from Source: the provider never authors the worker's generated digest.
enum Origin {
    Generated { body_id: String, revision: u64 },
    Vendor { sha: String, options: ImportOptions },
}
#[wasm_bindgen]
pub struct AnalysisHandle {
    owner: PreparedAnalysisMesh,
    origin: Origin,
}
#[wasm_bindgen]
impl AnalysisHandle {
    #[wasm_bindgen(js_name = retainedStorageBytes)]
    pub fn retained_storage_bytes(&self) -> JsValue {
        let extra = match &self.origin {
            Origin::Generated { body_id, .. } => Some(body_id.capacity()),
            Origin::Vendor { sha, .. } => Some(sha.capacity()),
        };
        self.owner
            .retained_storage_bytes()
            .and_then(|n| n.checked_add(std::mem::size_of::<Self>()))
            .and_then(|n| n.checked_add(extra?))
            .and_then(|n| u64::try_from(n).ok())
            .filter(|n| *n <= MAX_REVISION)
            .map(|n| JsValue::from_f64(n as f64))
            .unwrap_or(JsValue::NULL)
    }
}
pub(super) fn generated_handle(
    owner: PreparedAnalysisMesh,
    body_id: String,
    revision: u64,
) -> AnalysisHandle {
    AnalysisHandle {
        owner,
        origin: Origin::Generated { body_id, revision },
    }
}
pub(super) fn vendor_handle(owner: PreparedAnalysisMesh, sha: String) -> AnalysisHandle {
    let settings = &owner.analysis_mesh().settings;
    let options = ImportOptions {
        preview_repair_budget_mm: settings.preview_repair_budget_mm,
        target_deflection_mm: settings.target_deflection_mm,
    };
    AnalysisHandle {
        owner,
        origin: Origin::Vendor { sha, options },
    }
}
pub(super) fn add_reports(result: &Object, owner: &PreparedAnalysisMesh) -> Result<(), JsValue> {
    set(result, "analysisMesh", &to_js(owner.analysis_mesh())?)?;
    set(
        result,
        "sourceQualification",
        &to_js(owner.source_qualification())?,
    )
}
fn to_js(value: &impl Serialize) -> Result<JsValue, JsValue> {
    value
        .serialize(&serde_wasm_bindgen::Serializer::json_compatible())
        .map_err(js_error)
}
fn get(value: &JsValue, key: &str) -> Result<JsValue, JsValue> {
    Reflect::get(value, &key.into())
}
fn keys(value: &JsValue, required: &[&str], optional: &[&str]) -> Result<(), JsValue> {
    let object = value
        .dyn_ref::<Object>()
        .filter(|_| value.is_object() && !value.is_null() && !Array::is_array(value))
        .ok_or_else(|| js_error("qualification expects an object"))?;
    let actual = Object::keys(object)
        .iter()
        .map(|x| x.as_string().unwrap_or_default())
        .collect::<Vec<_>>();
    if required.iter().any(|k| !actual.iter().any(|a| a == k))
        || actual
            .iter()
            .any(|k| !required.contains(&k.as_str()) && !optional.contains(&k.as_str()))
    {
        return Err(js_error("qualification object has missing or extra keys"));
    }
    Ok(())
}
fn descriptor_keys(value: &JsValue) -> Result<(), JsValue> {
    keys(
        value,
        &[
            "version",
            "meshIdentity",
            "sourceProof",
            "settings",
            "sourceLimits",
        ],
        &[],
    )?;
    keys(
        &get(value, "settings")?,
        &[
            "targetDeflectionMm",
            "selectedDeflectionMm",
            "angularRad",
            "previewRepairBudgetMm",
        ],
        &[],
    )?;
    let limits = get(value, "sourceLimits")?;
    match get(&limits, "profile")?.as_string().as_deref() {
        Some("planar-final-v2") => keys(
            &limits,
            &[
                "profile",
                "maxSourceBytes",
                "maxSourceRecords",
                "maxDecimalCharacters",
                "minimumDecimalExponent",
                "maximumDecimalExponent",
                "minimumExponent",
                "maximumExponent",
                "maxFinalTriangles",
                "maxDirectBoundMm",
                "cornerSnapDistanceSquaredMm2",
            ],
            &[],
        ),
        Some("analytic-generated-v1") => keys(
            &limits,
            &[
                "profile",
                "maxFinalTriangles",
                "minimumNonzeroMagnitude",
                "maximumMagnitude",
            ],
            &[],
        ),
        _ => Err(js_error("unsupported source profile")),
    }
}
fn operand_keys(value: &JsValue) -> Result<(), JsValue> {
    let vendor = Reflect::has(value, &"sourceSha256".into())?;
    let identity = if vendor {
        "sourceSha256"
    } else {
        "generatedGeometryIdentity"
    };
    keys(
        value,
        &[
            "modelId",
            identity,
            "importOptions",
            "rowTransformF64",
            "analysisRef",
            "analysisMesh",
        ],
        &[],
    )?;
    keys(
        &get(value, "importOptions")?,
        &[],
        &["targetDeflectionMm", "previewRepairBudgetMm"],
    )?;
    descriptor_keys(&get(value, "analysisMesh")?)?;
    let reference = get(value, "analysisRef")?;
    keys(
        &reference,
        &[
            "workerGeneration",
            "meshIdentity",
            "source",
            "derivationIdentity",
        ],
        &[],
    )?;
    let source = get(&reference, "source")?;
    match get(&source, "kind")?.as_string().as_deref() {
        Some("vendor-step") => {
            keys(&source, &["kind", "sourceSha256", "importOptions"], &[])?;
            keys(
                &get(&source, "importOptions")?,
                &["targetDeflectionMm", "previewRepairBudgetMm"],
                &[],
            )
        }
        Some("generated-body") => keys(
            &source,
            &["kind", "bodyId", "revision", "geometryIdentity"],
            &[],
        ),
        _ => Err(js_error("unsupported analysis source")),
    }
}
fn is_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
}
fn validate_context(context: &Context) -> Result<(), String> {
    fn decimal(value: &str) -> bool {
        value.parse::<u64>().is_ok_and(|x| x.to_string() == value)
    }
    let snapshot = &context.snapshot;
    if context.worker_generation.is_empty()
        || snapshot.document_id.is_empty()
        || snapshot.board_id.is_empty()
        || snapshot.instance_id.as_ref().is_some_and(String::is_empty)
        || snapshot.revision > MAX_REVISION
        || !decimal(&snapshot.token)
        || !decimal(&snapshot.session_epoch)
    {
        return Err("invalid qualification context".into());
    }
    Ok(())
}
impl Operand {
    fn raw(&self) -> collision::CadCollisionOperand {
        collision::CadCollisionOperand {
            model_id: self.model_id.clone(),
            source_sha256: self.source_sha256.clone(),
            generated_geometry_identity: self.generated_geometry_identity.clone(),
            import_options: self.import_options,
            row_transform_f64: self.row_transform_f64,
        }
    }
    fn authenticate(&self, handle: &AnalysisHandle, context: &Context) -> Result<(), String> {
        let reference = &self.analysis_ref;
        if self.analysis_mesh != *handle.owner.analysis_mesh()
            || reference.mesh_identity != self.analysis_mesh.mesh_identity
            || reference.worker_generation != context.worker_generation
            || reference.derivation_identity.is_empty()
        {
            return Err("analysis descriptor/reference/handle mismatch".into());
        }
        // Match Runtime's existing derived_step_cache_key suffix exactly; the
        // worker authenticates the source bytes before issuing this reference.
        let vendor_derivation = |sha: &str, options: &ImportOptions| {
            let mut key = format!("vendor-step-analysis-v2/{sha}");
            if let Some(budget) = options.preview_repair_budget_mm {
                key.push_str(&format!(":repair={:016x}", budget.to_bits()));
            }
            if let Some(target) = options.target_deflection_mm {
                key.push_str(&format!(":deflection={:016x}", target.to_bits()));
            }
            key
        };
        let generated_derivation = |identity: &str| {
            let requested = handle
                .owner
                .analysis_mesh()
                .settings
                .target_deflection_mm
                .map(|x| format!("{:016x}", x.to_bits()))
                .unwrap_or_else(|| "default".into());
            format!("{identity}/deflection={requested}/analytic-generated-v1")
        };
        match (&handle.origin, &reference.source) {
            (
                Origin::Vendor { sha, options },
                Source::Vendor {
                    source_sha256,
                    import_options,
                },
            ) if sha == source_sha256
                && self.source_sha256.as_ref() == Some(sha)
                && self.generated_geometry_identity.is_none()
                && options == import_options
                && options.target_deflection_mm == self.import_options.target_deflection_mm
                && options.preview_repair_budget_mm
                    == self.import_options.preview_repair_budget_mm
                && reference.derivation_identity == vendor_derivation(sha, options) =>
            {
                Ok(())
            }
            (
                Origin::Generated { body_id, revision },
                Source::Generated {
                    body_id: expected_id,
                    revision: expected_revision,
                    geometry_identity,
                },
            ) if body_id == expected_id
                && revision == expected_revision
                && *revision == context.snapshot.revision
                && self.generated_geometry_identity.as_ref() == Some(geometry_identity)
                && self.source_sha256.is_none()
                && geometry_identity
                    .strip_prefix("m1/generated-analysis-v2/")
                    .is_some_and(is_sha256)
                && reference.derivation_identity == generated_derivation(geometry_identity) =>
            {
                Ok(())
            }
            _ => Err("analysis source/handle mismatch".into()),
        }
    }
}
fn validate_buffers(
    positions: &Float64Array,
    tags: &Uint32Array,
    owner: &PreparedAnalysisMesh,
) -> Result<Vec<u32>, JsValue> {
    if positions.length() == 0
        || tags.length() == 0
        || !positions.buffer().is_instance_of::<ArrayBuffer>()
        || !tags.buffer().is_instance_of::<ArrayBuffer>()
    {
        return Err(js_error("detached, shared or empty qualification buffers"));
    }
    let positions = positions.to_vec();
    let tags = tags.to_vec();
    let matches = |expected_positions: &[f64], expected_tags: &[usize]| {
        positions.len() == expected_positions.len()
            && tags.len() == expected_tags.len()
            && positions
                .iter()
                .zip(expected_positions)
                .all(|(a, b)| a.to_bits() == b.to_bits())
            && tags
                .iter()
                .zip(expected_tags)
                .all(|(a, b)| usize::try_from(*a).ok() == Some(*b))
    };
    let valid = match owner.export_view() {
        PreparedAnalysisView::Generated(v) => matches(
            &v.emission.positions_mm_f64,
            &v.emission.triangle_face_occurrence_indices,
        ),
        PreparedAnalysisView::Vendor(v) => matches(
            v.positions_mm_f64.as_flattened(),
            v.triangle_face_occurrence_indices,
        ),
    };
    if !valid {
        return Err(js_error(
            "final arrays differ from retained analysis handle",
        ));
    }
    Ok(tags)
}

#[wasm_bindgen]
pub fn query_qualified_collision_mesh_pair(
    buffers: JsValue,
    metadata: JsValue,
    context: JsValue,
    left_handle: &AnalysisHandle,
    right_handle: &AnalysisHandle,
) -> Result<JsValue, JsValue> {
    keys(
        &buffers,
        &["leftPositions", "leftTags", "rightPositions", "rightTags"],
        &[],
    )?;
    keys(
        &metadata,
        &[
            "left",
            "right",
            "clearanceThresholdMm",
            "requestedAccuracyMm",
            "qualificationLimits",
        ],
        &[],
    )?;
    operand_keys(&get(&metadata, "left")?)?;
    operand_keys(&get(&metadata, "right")?)?;
    keys(
        &get(&metadata, "qualificationLimits")?,
        &[
            "maxTriangles",
            "maxNodePairs",
            "maxExactPairs",
            "maxContactBoxes",
            "minimumExponent",
            "maximumExponent",
            "requestedNumericalWidthMm",
        ],
        &[],
    )?;
    keys(&context, &["snapshot", "workerGeneration"], &[])?;
    keys(
        &get(&context, "snapshot")?,
        &[
            "token",
            "sessionEpoch",
            "documentId",
            "boardId",
            "instanceId",
            "revision",
        ],
        &[],
    )?;
    let metadata: Metadata = serde_wasm_bindgen::from_value(metadata).map_err(js_error)?;
    let context: Context = serde_wasm_bindgen::from_value(context).map_err(js_error)?;
    validate_context(&context).map_err(js_error)?;
    metadata.qualification_limits.validate().map_err(js_error)?;
    let raw = collision::CadCollisionQuery {
        left: metadata.left.raw(),
        right: metadata.right.raw(),
        clearance_threshold_mm: metadata.clearance_threshold_mm,
        requested_accuracy_mm: metadata.requested_accuracy_mm,
    };
    // The raw query owner keeps its existing validation and observation semantics.
    collision::validate_qualified_request(&raw).map_err(js_error)?;
    metadata
        .left
        .authenticate(left_handle, &context)
        .map_err(js_error)?;
    metadata
        .right
        .authenticate(right_handle, &context)
        .map_err(js_error)?;
    let left_positions = get(&buffers, "leftPositions")?
        .dyn_into::<Float64Array>()
        .map_err(|_| js_error("leftPositions must be Float64Array"))?;
    let left_tags = get(&buffers, "leftTags")?
        .dyn_into::<Uint32Array>()
        .map_err(|_| js_error("leftTags must be Uint32Array"))?;
    let right_positions = get(&buffers, "rightPositions")?
        .dyn_into::<Float64Array>()
        .map_err(|_| js_error("rightPositions must be Float64Array"))?;
    let right_tags = get(&buffers, "rightTags")?
        .dyn_into::<Uint32Array>()
        .map_err(|_| js_error("rightTags must be Uint32Array"))?;
    collision::checked_pair_buffer_bytes(
        left_positions.length() as usize,
        left_tags.length() as usize,
        right_positions.length() as usize,
        right_tags.length() as usize,
    )
    .map_err(js_error)?;
    let left_tags = validate_buffers(&left_positions, &left_tags, &left_handle.owner)?;
    let right_tags = validate_buffers(&right_positions, &right_tags, &right_handle.owner)?;
    let evidence = q::verify_analysis_pair(
        &left_handle.owner,
        &right_handle.owner,
        [
            metadata.left.row_transform_f64,
            metadata.right.row_transform_f64,
        ],
        metadata.qualification_limits,
    );
    let [left_world, right_world] = evidence.world_positions();
    let observation =
        collision::observe_qualified_world(left_world, &left_tags, right_world, &right_tags)
            .map_err(js_error)?;
    let binding = Binding {
        snapshot_revision: context.snapshot.revision,
        snapshot: &context.snapshot,
        worker_generation: &context.worker_generation,
        left_model_id: &metadata.left.model_id,
        right_model_id: &metadata.right.model_id,
        left_mesh_identity: &metadata.left.analysis_mesh.mesh_identity,
        right_mesh_identity: &metadata.right.analysis_mesh.mesh_identity,
        left_analysis: &metadata.left.analysis_ref,
        right_analysis: &metadata.right.analysis_ref,
        left_mesh: &metadata.left.analysis_mesh,
        right_mesh: &metadata.right.analysis_mesh,
        left_row_transform_f64: metadata.left.row_transform_f64,
        right_row_transform_f64: metadata.right.row_transform_f64,
        clearance_threshold_mm: metadata.clearance_threshold_mm,
        requested_accuracy_mm: metadata.requested_accuracy_mm,
    };
    let qualifier = to_js(&evidence)?
        .dyn_into::<Object>()
        .map_err(|_| js_error("query report is not object"))?;
    set(&qualifier, "version", &1.into())?;
    set(&qualifier, "algorithm", &"aabb-interval-exact-v1".into())?;
    set(&qualifier, "binding", &to_js(&binding)?)?;
    let result = Object::new();
    set(&result, "observation", &to_js(&observation)?)?;
    set(&result, "queryQualification", &qualifier)?;
    Ok(result.into())
}
