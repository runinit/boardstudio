//! Captured-snapshot case preparation and CAD job identity guards.
#[cfg(feature = "page")]
use boardstudio_application::{AcceptedSnapshot, Scope};
#[cfg(all(test, feature = "page"))]
use boardstudio_core::CoreEngine;
use boardstudio_core::hash::sha256_hex;
use boardstudio_core::model::*;
pub use boardstudio_core::model::{
    CadStepImportOptions, is_sha256_hex, validate_affine_row_transform,
};
use serde::{Deserialize, Serialize};

pub const MAX_CAD_REVISION: u64 = 9_007_199_254_740_991;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CadSnapshotIdentity {
    pub token: u64,
    pub session_epoch: u64,
    pub document_id: String,
    pub board_id: String,
    pub instance_id: Option<String>,
    pub revision: u64,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum CadOperation {
    Preview,
    PrintKeycaps,
    Exact,
    ExportStep,
    ReadStep,
    AnalyzeGeneratedBody,
    CollisionQuery,
}

pub const MAX_COLLISION_QUERY_BYTES: usize = 64 * 1024 * 1024;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CadCollisionQuery {
    pub left: CadCollisionOperand,
    pub right: CadCollisionOperand,
    pub clearance_threshold_mm: f64,
    #[serde(deserialize_with = "required_accuracy")]
    pub requested_accuracy_mm: Option<f64>,
    pub qualification_limits: crate::cad_qualification::Limits,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CadCollisionOperand {
    pub model_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_sha256: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub generated_geometry_identity: Option<String>,
    #[serde(default)]
    pub import_options: CadStepImportOptions,
    pub row_transform_f64: [f64; 16],
    pub analysis_ref: Option<crate::cad_qualification::AnalysisRef>,
    pub analysis_mesh: Option<crate::cad_qualification::AnalysisMesh>,
}

fn required_accuracy<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<f64>, D::Error> {
    Option::deserialize(deserializer)
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct CadCollisionMeshPair {
    pub left_positions_mm_f64: Vec<f64>,
    pub left_triangle_face_occurrence_indices: Vec<u32>,
    pub right_positions_mm_f64: Vec<f64>,
    pub right_triangle_face_occurrence_indices: Vec<u32>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CadTriangleProvenance {
    pub triangle_index: u32,
    pub face_occurrence_index: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CadTrianglePairProvenance {
    pub left: CadTriangleProvenance,
    pub right: CadTriangleProvenance,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CadContactWitness {
    pub triangles: CadTrianglePairProvenance,
    pub point1_mm: [f64; 3],
    pub point2_mm: [f64; 3],
    pub normal1: [f64; 3],
    pub normal2: [f64; 3],
    pub triangle_pair_contact_dist_mm: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CadClosestPointsWitness {
    pub triangles: CadTrianglePairProvenance,
    pub point1_mm: [f64; 3],
    pub point2_mm: [f64; 3],
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum CadCollisionObservation {
    #[serde(rename_all = "camelCase")]
    SurfaceIntersecting {
        intersection_triangles: CadTrianglePairProvenance,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        contact_witness: Option<CadContactWitness>,
    },
    #[serde(rename_all = "camelCase")]
    SurfaceSeparated {
        surface_distance_mm: f64,
        closest_points: CadClosestPointsWitness,
    },
    #[serde(rename_all = "camelCase")]
    Uncertain {
        reason: String,
        #[serde(default)]
        partial_intersection: bool,
    },
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct CadVendorStep {
    pub analysis_ref: Option<crate::cad_qualification::AnalysisRef>,
    pub analysis_mesh: Option<crate::cad_qualification::AnalysisMesh>,
    pub source_qualification: Option<crate::cad_qualification::SourceQualification>,
    pub source_sha256: String,
    #[serde(default)]
    pub import_options: CadStepImportOptions,
    pub surface_positions_mm_f64: Vec<f64>,
    pub source_faces: Vec<CadStepSourceFace>,
    pub face_occurrences: Vec<CadStepFaceOccurrence>,
    pub triangle_face_occurrence_indices: Vec<u32>,
    pub line_segments_mm_f64: Vec<f64>,
    pub line_render_positions: Vec<f32>,
    pub line_occurrences: Vec<CadStepLineOccurrence>,
    pub segment_line_occurrence_indices: Vec<u32>,
    pub precision: Vec<CadStepPrecision>,
    pub accuracy: CadStepAccuracy,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", default)]
pub struct CadStepSourceFace {
    pub solid_id: String,
    pub shell_index: u32,
    pub face_id: String,
    pub support_id: String,
    pub support_kind: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct CadStepFaceOccurrence {
    pub source_face_index: u32,
    pub solid_occurrence_index: u32,
    pub representation_id: String,
    pub source_uncertainty_mm: Option<f64>,
    pub repair_displacement_mm: Option<f64>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct CadStepLineOccurrence {
    pub line_occurrence_index: u32,
    pub trimmed_curve_id: String,
    pub source_line_id: String,
    pub curve_set_id: String,
    pub representation_id: String,
    pub source_unit_to_mm: f64,
    pub placement_f64: [f64; 16],
    pub declared_uncertainty_mm: Option<f64>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct CadStepPrecision {
    pub representation_id: String,
    pub context_id: String,
    pub geometric_dimension: Option<u32>,
    pub declarations: Vec<CadStepUncertainty>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct CadStepUncertainty {
    pub entity_id: String,
    pub length_unit_id: String,
    pub millimetres: f64,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct CadStepAccuracy {
    pub preview_deflection_mm: f64,
    pub preview_angular_rad: f64,
    pub analysis_error_bound_mm: Option<f64>,
    pub selected_repair_budget_mm: Option<f64>,
    pub max_repair_displacement_mm: Option<f64>,
    pub bounds_from_source_geometry: bool,
    pub mesh_closed_oriented: bool,
    pub preview_floor_mm: f64,
    pub preview_declared_multiplier: f64,
    pub relaxed_beyond_declared_precision: bool,
    pub sampled_support_checks: u32,
    pub max_sampled_support_residual_mm: Option<f64>,
    pub fallback_meshed_faces: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CadRequest {
    pub request_id: String,
    pub job_id: String,
    pub identity: CadSnapshotIdentity,
    pub operation: CadOperation,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prepared: Option<PreparedCaseAssemblyIR>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_sha256: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub step_import_options: Option<CadStepImportOptions>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub collision_query: Option<CadCollisionQuery>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub generated_analysis_request: Option<CadGeneratedAnalysisRequest>,
    #[serde(skip)]
    pub collision_meshes: Option<CadCollisionMeshPair>,
    #[serde(skip)]
    pub input_bytes: Vec<u8>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CadMesh {
    pub positions: Vec<f32>,
    pub normals: Vec<f32>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CadBodyMesh {
    pub id: String,
    pub name: String,
    pub positions: Vec<f32>,
    pub normals: Vec<f32>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CadBounds {
    pub min: [f64; 3],
    pub max: [f64; 3],
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct CadResult {
    pub query_qualification: Option<crate::cad_qualification::QueryQualification>,
    pub revision: u64,
    pub step: Vec<u8>,
    pub mesh: Option<CadMesh>,
    pub bodies: Vec<CadBodyMesh>,
    pub bounds: Option<CadBounds>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vendor_step: Option<CadVendorStep>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub collision: Option<CadCollisionObservation>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub generated_analysis: Option<CadGeneratedAnalysis>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CadGeneratedAnalysisRequest {
    pub body_id: String,
    pub geometry_identity: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_deflection_mm: Option<f64>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CadGeneratedAnalysis {
    #[serde(default)]
    pub analysis_ref: Option<crate::cad_qualification::AnalysisRef>,
    #[serde(default)]
    pub analysis_mesh: Option<crate::cad_qualification::AnalysisMesh>,
    #[serde(default)]
    pub source_qualification: Option<crate::cad_qualification::SourceQualification>,
    pub body_id: String,
    pub revision: u64,
    pub geometry_identity: String,
    pub surface_positions_mm_f64: Vec<f64>,
    pub triangle_face_occurrence_indices: Vec<u32>,
    pub source_faces: Vec<CadGeneratedSourceFace>,
    pub face_occurrences: Vec<CadGeneratedFaceOccurrence>,
    pub analysis_error_bound_mm: Option<f64>,
    pub mesh_closed_oriented: bool,
    pub deflection_mm: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CadGeneratedSourceFace {
    pub solid_index: u32,
    pub shell_index: u32,
    pub face_index: u32,
    pub support_kind: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CadGeneratedFaceOccurrence {
    pub source_face_index: u32,
    pub solid_occurrence_index: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CadReply {
    pub request_id: String,
    pub job_id: String,
    pub identity: CadSnapshotIdentity,
    pub operation: CadOperation,
    pub outcome: CadReplyOutcome,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result: Option<CadResult>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum CadReplyOutcome {
    Completed,
    Cancelled,
    Failed,
}

pub fn validate_reply(
    request: &CadRequest,
    mut reply: CadReply,
    current: &CadSnapshotIdentity,
) -> Result<CadResult, CadJobError> {
    if reply.request_id != request.request_id
        || reply.job_id != request.job_id
        || reply.operation != request.operation
        || reply.identity != request.identity
        || reply.identity != *current
    {
        return Err(CadJobError::Stale(
            "CAD result belongs to another job or accepted snapshot".into(),
        ));
    }
    match reply.outcome {
        CadReplyOutcome::Cancelled => Err(CadJobError::Cancelled),
        CadReplyOutcome::Failed => Err(CadJobError::Failed(
            reply
                .error
                .clone()
                .unwrap_or_else(|| "CAD job failed".into()),
        )),
        CadReplyOutcome::Completed => {
            let result = reply
                .result
                .take()
                .ok_or_else(|| CadJobError::Failed("CAD worker returned no result".into()))?;
            if result.revision != request.identity.revision {
                return Err(CadJobError::Stale(
                    "CAD result has another document revision".into(),
                ));
            }
            validate_result_payload(request, &result)?;
            Ok(result)
        }
    }
}

fn validate_result_payload(request: &CadRequest, result: &CadResult) -> Result<(), CadJobError> {
    let bad = |reason: &str| CadJobError::Failed(format!("invalid CAD result: {reason}"));
    let valid_mesh_parts = |positions: &[f32], normals: &[f32]| {
        !positions.is_empty()
            && positions.len().is_multiple_of(3)
            && positions.len() == normals.len()
            && positions
                .iter()
                .chain(normals)
                .all(|value| value.is_finite())
    };
    let valid_mesh = |mesh: &CadMesh| valid_mesh_parts(&mesh.positions, &mesh.normals);
    let valid_bounds = |bounds: &CadBounds| {
        bounds
            .min
            .iter()
            .chain(&bounds.max)
            .all(|value| value.is_finite())
            && (0..3).all(|axis| bounds.min[axis] <= bounds.max[axis])
    };
    match request.operation {
        CadOperation::AnalyzeGeneratedBody => {
            let metadata = request
                .generated_analysis_request
                .as_ref()
                .ok_or_else(|| bad("generated analysis omitted request metadata"))?;
            validate_generated_analysis_request(
                request.prepared.as_ref(),
                request.identity.revision,
                metadata,
            )
            .map_err(|reason| bad(&reason))?;
            let analysis = result
                .generated_analysis
                .as_ref()
                .ok_or_else(|| bad("generated analysis omitted payload"))?;
            validate_generated_analysis(metadata, request.identity.revision, analysis)
                .map_err(|reason| bad(&reason))?;
        }
        CadOperation::PrintKeycaps => {
            return Err(bad("printable keycaps require the keycap request adapter"));
        }
        CadOperation::Preview => {
            let expected = request
                .prepared
                .as_ref()
                .map(|prepared| {
                    prepared
                        .bodies
                        .iter()
                        .map(|body| body.body.id.as_str())
                        .collect::<Vec<_>>()
                })
                .ok_or_else(|| bad("preview has no prepared bodies"))?;
            if result.bodies.len() != expected.len()
                || result.bodies.iter().zip(expected).any(|(body, id)| {
                    body.id != id || !valid_mesh_parts(&body.positions, &body.normals)
                })
            {
                return Err(bad(
                    "preview body meshes are missing, malformed, or out of order",
                ));
            }
        }
        CadOperation::Exact => {
            if !result.step.starts_with(b"ISO-10303-21;") || result.step.len() > 32 * 1024 * 1024 {
                return Err(bad("exact assembly omitted valid STEP data"));
            }
            if !result.mesh.as_ref().is_some_and(valid_mesh)
                || result.bodies.is_empty()
                || result.bodies.iter().any(|body| {
                    body.id.is_empty() || !valid_mesh_parts(&body.positions, &body.normals)
                })
                || result
                    .bounds
                    .as_ref()
                    .is_some_and(|bounds| !valid_bounds(bounds))
            {
                return Err(bad(
                    "exact assembly mesh, bodies, or supplied bounds are invalid",
                ));
            }
        }
        CadOperation::ExportStep => {
            if !result.step.starts_with(b"ISO-10303-21;") || result.step.len() > 32 * 1024 * 1024 {
                return Err(bad("STEP export omitted valid STEP data"));
            }
        }
        CadOperation::ReadStep => {
            let Some(expected_sha) = request
                .source_sha256
                .as_deref()
                .filter(|sha| is_sha256_hex(sha))
            else {
                return Err(bad("STEP import requires a canonical source SHA-256"));
            };
            let expected_options = request
                .step_import_options
                .unwrap_or_default()
                .validate()
                .map_err(|reason| bad(&reason))?;
            let (Some(mesh), Some(bounds)) = (
                result.mesh.as_ref().filter(|mesh| valid_mesh(mesh)),
                result.bounds.as_ref().filter(|bounds| valid_bounds(bounds)),
            ) else {
                return Err(bad("STEP import mesh or bounds are invalid"));
            };
            let Some(vendor_step) = result.vendor_step.as_ref() else {
                return Err(bad("STEP import omitted vendor STEP source metadata"));
            };
            validate_vendor_step(expected_sha, expected_options, mesh, bounds, vendor_step)
                .map_err(|reason| bad(&reason))?;
        }
        CadOperation::CollisionQuery => {
            let Some(query) = request.collision_query.as_ref() else {
                return Err(bad("collision query omitted metadata"));
            };
            validate_collision_query(query).map_err(|reason| bad(&reason))?;
            if let Some(meshes) = request.collision_meshes.as_ref() {
                validate_collision_mesh_pair(meshes).map_err(|reason| bad(&reason))?;
            }
            let Some(collision) = result.collision.as_ref() else {
                return Err(bad("collision query omitted observation"));
            };
            validate_collision_observation(request.collision_meshes.as_ref(), collision)
                .map_err(|reason| bad(&reason))?;
            let report = result
                .query_qualification
                .clone()
                .ok_or_else(|| bad("missing query qualification"))?;
            let expected = crate::cad_qualification::expected_binding(
                query,
                &request.identity,
                &report.binding.worker_generation,
            )
            .map_err(|reason| bad(&reason))?;
            crate::cad_qualification::validate_query_qualification(
                report,
                &expected,
                &query.qualification_limits,
                request
                    .collision_meshes
                    .as_ref()
                    .ok_or_else(|| bad("missing query buffers"))?,
                None,
            )
            .map_err(|reason| bad(&reason))?;
        }
    }
    Ok(())
}

/// Shares the production generated geometry cache's m1 revision. Include the
/// complete prepared body, rather than presentation meshes or a STEP source hash.
pub fn generated_geometry_identity(body: &PreparedCaseIR) -> Result<String, String> {
    let bytes = serde_json::to_vec(body).map_err(|error| error.to_string())?;
    let digest = sha256_hex(&bytes);
    Ok(format!("m1/generated-analysis-v2/{digest}"))
}

pub fn validate_generated_analysis_request<'a>(
    prepared: Option<&'a PreparedCaseAssemblyIR>,
    revision: u64,
    metadata: &CadGeneratedAnalysisRequest,
) -> Result<&'a PreparedCaseIR, String> {
    let assembly = prepared
        .filter(|assembly| assembly.revision == revision)
        .ok_or("generated analysis requires the captured prepared assembly")?;
    let bodies = assembly
        .bodies
        .iter()
        .filter(|body| body.body.id == metadata.body_id)
        .collect::<Vec<_>>();
    if bodies.len() != 1 || bodies[0].revision != revision {
        return Err("generated analysis requires exactly one body at the captured revision".into());
    }
    if metadata
        .target_deflection_mm
        .is_some_and(|value| !value.is_finite() || value <= 0.0)
    {
        return Err("generated targetDeflectionMm must be finite and positive".into());
    }
    if generated_geometry_identity(bodies[0])? != metadata.geometry_identity {
        return Err("generated geometry identity does not match prepared body".into());
    }
    Ok(bodies[0])
}

pub fn validate_generated_analysis(
    metadata: &CadGeneratedAnalysisRequest,
    revision: u64,
    analysis: &CadGeneratedAnalysis,
) -> Result<(), String> {
    match (
        &analysis.analysis_ref,
        &analysis.analysis_mesh,
        &analysis.source_qualification,
    ) {
        (Some(reference), Some(mesh), Some(source)) => {
            crate::cad_qualification::validate_analysis(
                reference,
                mesh,
                source,
                analysis.analysis_error_bound_mm,
            )?;
            crate::cad_qualification::validate_source_triangle_cap(
                mesh,
                source,
                analysis.triangle_face_occurrence_indices.len(),
            )?;
            if reference.source
                != (crate::cad_qualification::Source::GeneratedBody {
                    body_id: analysis.body_id.clone(),
                    revision,
                    geometry_identity: metadata.geometry_identity.clone(),
                })
                || mesh.settings.target_deflection_mm.map(f64::to_bits)
                    != metadata.target_deflection_mm.map(f64::to_bits)
                || mesh.settings.selected_deflection_mm.to_bits()
                    != analysis.deflection_mm.to_bits()
            {
                return Err("generated source descriptor does not match preparation".into());
            }
        }
        (None, None, None) if analysis.analysis_error_bound_mm.is_none() => {}
        _ => return Err("incomplete generated qualification metadata".into()),
    }
    if analysis.body_id != metadata.body_id
        || analysis.revision != revision
        || analysis.geometry_identity != metadata.geometry_identity
    {
        return Err("generated analysis belongs to another body, geometry or revision".into());
    }
    let selected_deflection_mm = metadata.target_deflection_mm.unwrap_or(0.1).min(0.1);
    if metadata
        .target_deflection_mm
        .is_some_and(|target| !target.is_finite() || target <= 0.0)
        || !analysis.deflection_mm.is_finite()
        || analysis.deflection_mm <= 0.0
        || analysis.deflection_mm != selected_deflection_mm
        || analysis
            .analysis_error_bound_mm
            .is_some_and(|bound| !bound.is_finite() || bound < 0.0)
    {
        return Err("generated analysis has invalid or insufficient accuracy settings".into());
    }
    let positions = &analysis.surface_positions_mm_f64;
    if positions.is_empty()
        || !positions.len().is_multiple_of(9)
        || positions.len().saturating_mul(8).saturating_add(
            analysis
                .triangle_face_occurrence_indices
                .len()
                .saturating_mul(4),
        ) > MAX_COLLISION_QUERY_BYTES
        || positions.iter().any(|value| !value.is_finite())
        || analysis.triangle_face_occurrence_indices.len() != positions.len() / 9
        || analysis
            .triangle_face_occurrence_indices
            .iter()
            .any(|&index| index as usize >= analysis.face_occurrences.len())
        || analysis
            .face_occurrences
            .iter()
            .any(|occurrence| occurrence.source_face_index as usize >= analysis.source_faces.len())
        || analysis
            .source_faces
            .iter()
            .any(|face| face.support_kind.trim().is_empty())
    {
        return Err("generated analysis mesh or retained face occurrences are invalid".into());
    }
    Ok(())
}

pub fn valid_step_entity_id(value: &str) -> bool {
    !value.is_empty() && (value == "0" || !value.starts_with('0')) && value.parse::<u64>().is_ok()
}

pub fn validate_vendor_step(
    expected_sha: &str,
    expected_options: CadStepImportOptions,
    mesh: &CadMesh,
    bounds: &CadBounds,
    vendor_step: &CadVendorStep,
) -> Result<(), String> {
    match (
        &vendor_step.analysis_ref,
        &vendor_step.analysis_mesh,
        &vendor_step.source_qualification,
    ) {
        (Some(reference), Some(descriptor), Some(source)) => {
            crate::cad_qualification::validate_analysis(
                reference,
                descriptor,
                source,
                vendor_step.accuracy.analysis_error_bound_mm,
            )?;
            crate::cad_qualification::validate_source_triangle_cap(
                descriptor,
                source,
                vendor_step.triangle_face_occurrence_indices.len(),
            )?;
            if let crate::cad_qualification::SourceQualification::Bounded {
                source_precision_mm,
                ..
            } = source
            {
                let declared = vendor_step
                    .precision
                    .iter()
                    .flat_map(|p| &p.declarations)
                    .map(|d| d.millimetres)
                    .chain(
                        vendor_step
                            .face_occurrences
                            .iter()
                            .filter_map(|f| f.source_uncertainty_mm),
                    )
                    .fold(0_f64, f64::max);
                if *source_precision_mm < declared {
                    return Err("source qualification omits declared STEP precision".into());
                }
            }
            if reference.source
                != (crate::cad_qualification::Source::VendorStep {
                    source_sha256: expected_sha.to_owned(),
                    import_options: expected_options.into(),
                })
                || descriptor.settings.selected_deflection_mm.to_bits()
                    != vendor_step.accuracy.preview_deflection_mm.to_bits()
                || descriptor.settings.angular_rad.to_bits()
                    != vendor_step.accuracy.preview_angular_rad.to_bits()
            {
                return Err("vendor source descriptor does not match authenticated import".into());
            }
        }
        (None, None, None) if vendor_step.accuracy.analysis_error_bound_mm.is_none() => {}
        _ => return Err("incomplete vendor qualification metadata".into()),
    }
    if !is_sha256_hex(expected_sha)
        || !is_sha256_hex(&vendor_step.source_sha256)
        || vendor_step.source_sha256 != expected_sha
    {
        return Err("STEP import returned a mismatched or non-canonical sourceSha256".into());
    }
    let expected_options = expected_options.validate()?;
    vendor_step.import_options.validate()?;
    if !vendor_step.import_options.same_identity(expected_options) {
        return Err("STEP import returned mismatched import options".into());
    }
    if mesh.positions.is_empty()
        || !mesh.positions.len().is_multiple_of(9)
        || mesh.normals.len() != mesh.positions.len()
        || mesh
            .positions
            .iter()
            .chain(&mesh.normals)
            .any(|value| !value.is_finite())
        || vendor_step.surface_positions_mm_f64.len() != mesh.positions.len()
        || vendor_step
            .surface_positions_mm_f64
            .iter()
            .any(|value| !value.is_finite())
        || vendor_step
            .surface_positions_mm_f64
            .iter()
            .zip(&mesh.positions)
            .any(|(source, render)| (*source as f32) != *render)
    {
        return Err("STEP import surface buffers are missing or inconsistent".into());
    }
    if vendor_step.source_faces.is_empty()
        || vendor_step.source_faces.iter().any(|face| {
            !valid_step_entity_id(&face.solid_id)
                || !valid_step_entity_id(&face.face_id)
                || !valid_step_entity_id(&face.support_id)
                || face.support_kind.trim().is_empty()
        })
    {
        return Err("STEP import source face provenance is missing or invalid".into());
    }
    if vendor_step.face_occurrences.is_empty()
        || vendor_step.face_occurrences.iter().any(|face| {
            (face.source_face_index as usize) >= vendor_step.source_faces.len()
                || !valid_step_entity_id(&face.representation_id)
                || face
                    .source_uncertainty_mm
                    .is_some_and(|value| !value.is_finite() || value <= 0.0)
                || face
                    .repair_displacement_mm
                    .is_some_and(|value| !value.is_finite() || value < 0.0)
        })
    {
        return Err("STEP import face occurrence provenance is missing or invalid".into());
    }
    let triangle_count = mesh.positions.len() / 9;
    if vendor_step.triangle_face_occurrence_indices.len() != triangle_count
        || vendor_step
            .triangle_face_occurrence_indices
            .iter()
            .any(|index| (*index as usize) >= vendor_step.face_occurrences.len())
    {
        return Err("STEP import triangle face occurrence indices are invalid".into());
    }
    if !vendor_step.line_segments_mm_f64.len().is_multiple_of(6)
        || vendor_step.line_render_positions.len() != vendor_step.line_segments_mm_f64.len()
        || vendor_step.segment_line_occurrence_indices.len()
            != vendor_step.line_segments_mm_f64.len() / 6
        || vendor_step
            .line_segments_mm_f64
            .iter()
            .any(|value| !value.is_finite())
        || vendor_step
            .line_render_positions
            .iter()
            .any(|value| !value.is_finite())
        || vendor_step
            .line_segments_mm_f64
            .iter()
            .zip(&vendor_step.line_render_positions)
            .any(|(source, render)| (*source as f32) != *render)
        || vendor_step
            .line_occurrences
            .iter()
            .enumerate()
            .any(|(index, line)| {
                line.line_occurrence_index as usize != index
                    || !valid_step_entity_id(&line.trimmed_curve_id)
                    || !valid_step_entity_id(&line.source_line_id)
                    || !valid_step_entity_id(&line.curve_set_id)
                    || !valid_step_entity_id(&line.representation_id)
                    || !line.source_unit_to_mm.is_finite()
                    || line.source_unit_to_mm <= 0.0
                    || line.placement_f64.iter().any(|value| !value.is_finite())
                    || line
                        .declared_uncertainty_mm
                        .is_some_and(|value| !value.is_finite() || value <= 0.0)
            })
        || vendor_step
            .segment_line_occurrence_indices
            .iter()
            .any(|index| (*index as usize) >= vendor_step.line_occurrences.len())
    {
        return Err("STEP import source line buffers or metadata are invalid".into());
    }
    if vendor_step.precision.iter().any(|precision| {
        !valid_step_entity_id(&precision.representation_id)
            || !valid_step_entity_id(&precision.context_id)
            || precision.declarations.iter().any(|declaration| {
                !valid_step_entity_id(&declaration.entity_id)
                    || !valid_step_entity_id(&declaration.length_unit_id)
                    || !declaration.millimetres.is_finite()
                    || declaration.millimetres <= 0.0
            })
    }) {
        return Err("STEP import precision declarations are invalid".into());
    }
    let accuracy = &vendor_step.accuracy;
    if !accuracy.preview_deflection_mm.is_finite()
        || accuracy.preview_deflection_mm <= 0.0
        || !accuracy.preview_angular_rad.is_finite()
        || accuracy.preview_angular_rad <= 0.0
        || accuracy
            .analysis_error_bound_mm
            .is_some_and(|value| !value.is_finite() || value < 0.0)
        || accuracy
            .selected_repair_budget_mm
            .is_some_and(|value| !value.is_finite() || value <= 0.0)
        || accuracy
            .max_repair_displacement_mm
            .is_some_and(|value| !value.is_finite() || value < 0.0)
        || expected_options
            .target_deflection_mm
            .is_some_and(|target| accuracy.preview_deflection_mm > target)
        || !accuracy.preview_floor_mm.is_finite()
        || accuracy.preview_floor_mm < 0.0
        || !accuracy.preview_declared_multiplier.is_finite()
        || accuracy.preview_declared_multiplier < 0.0
        || accuracy
            .max_sampled_support_residual_mm
            .is_some_and(|value| !value.is_finite() || value < 0.0)
    {
        return Err("STEP import accuracy metadata is invalid".into());
    }
    let bounds_tolerance = if accuracy.bounds_from_source_geometry {
        0.03
    } else {
        0.0
    };
    let contains = |positions: &[f64]| {
        positions.len().is_multiple_of(3)
            && positions.as_chunks::<3>().0.iter().all(|point| {
                (0..3).all(|axis| {
                    bounds.min[axis] - bounds_tolerance <= point[axis]
                        && point[axis] <= bounds.max[axis] + bounds_tolerance
                })
            })
    };
    if !contains(&vendor_step.surface_positions_mm_f64)
        || !contains(&vendor_step.line_segments_mm_f64)
    {
        return Err("STEP import bounds omit returned surface or line coordinates".into());
    }
    Ok(())
}

pub fn validate_collision_query(query: &CadCollisionQuery) -> Result<(), String> {
    query.qualification_limits.validate()?;
    if !query.clearance_threshold_mm.is_finite() || query.clearance_threshold_mm < 0.0 {
        return Err("collision clearanceThresholdMm must be finite and non-negative".into());
    }
    if let Some(accuracy) = query.requested_accuracy_mm
        && (!accuracy.is_finite() || accuracy <= 0.0)
    {
        return Err("collision requestedAccuracyMm must be finite and positive".into());
    }
    for operand in [&query.left, &query.right] {
        if operand.model_id.trim().is_empty() {
            return Err("collision operand modelId must not be empty".into());
        }
        match (&operand.source_sha256, &operand.generated_geometry_identity) {
            (Some(sha), None) if is_sha256_hex(sha) => {}
            (None, Some(identity))
                if !identity.trim().is_empty()
                    && operand.import_options == CadStepImportOptions::default() => {}
            _ => {
                return Err(
                    "collision operand requires exactly one valid STEP or generated identity"
                        .into(),
                );
            }
        }
        operand.import_options.validate()?;
        validate_affine_row_transform(&operand.row_transform_f64)
            .map_err(|error| error.to_string())?;
        crate::cad_qualification::validate_operand(operand)?;
    }
    Ok(())
}

pub fn validate_collision_mesh_pair(meshes: &CadCollisionMeshPair) -> Result<(), String> {
    let total_bytes = meshes
        .left_positions_mm_f64
        .len()
        .saturating_mul(8)
        .saturating_add(
            meshes
                .left_triangle_face_occurrence_indices
                .len()
                .saturating_mul(4),
        )
        .saturating_add(meshes.right_positions_mm_f64.len().saturating_mul(8))
        .saturating_add(
            meshes
                .right_triangle_face_occurrence_indices
                .len()
                .saturating_mul(4),
        );
    if total_bytes > MAX_COLLISION_QUERY_BYTES {
        return Err("collision mesh buffers exceed the 64 MiB limit".into());
    }
    for (positions, tags) in [
        (
            &meshes.left_positions_mm_f64,
            &meshes.left_triangle_face_occurrence_indices,
        ),
        (
            &meshes.right_positions_mm_f64,
            &meshes.right_triangle_face_occurrence_indices,
        ),
    ] {
        if positions.is_empty()
            || !positions.len().is_multiple_of(9)
            || tags.len() != positions.len() / 9
            || positions.len() / 3 > u32::MAX as usize
            || positions.iter().any(|value| !value.is_finite())
        {
            return Err("collision mesh buffers or triangle tags are invalid".into());
        }
    }
    Ok(())
}

pub fn validate_collision_observation(
    meshes: Option<&CadCollisionMeshPair>,
    observation: &CadCollisionObservation,
) -> Result<(), String> {
    let valid_pair = |pair: &CadTrianglePairProvenance| -> bool {
        let Some(meshes) = meshes else {
            return true;
        };
        let left_tris = meshes.left_triangle_face_occurrence_indices.len();
        let right_tris = meshes.right_triangle_face_occurrence_indices.len();
        (pair.left.triangle_index as usize) < left_tris
            && meshes.left_triangle_face_occurrence_indices[pair.left.triangle_index as usize]
                == pair.left.face_occurrence_index
            && (pair.right.triangle_index as usize) < right_tris
            && meshes.right_triangle_face_occurrence_indices[pair.right.triangle_index as usize]
                == pair.right.face_occurrence_index
    };
    match observation {
        CadCollisionObservation::SurfaceIntersecting {
            intersection_triangles,
            contact_witness,
        } => {
            if !valid_pair(intersection_triangles) {
                return Err("collision intersection triangle provenance is invalid".into());
            }
            if let Some(witness) = contact_witness
                && (!valid_pair(&witness.triangles)
                    || witness
                        .point1_mm
                        .iter()
                        .chain(&witness.point2_mm)
                        .chain(&witness.normal1)
                        .chain(&witness.normal2)
                        .any(|value| !value.is_finite())
                    || !witness.triangle_pair_contact_dist_mm.is_finite())
            {
                return Err("collision contact witness is invalid".into());
            }
        }
        CadCollisionObservation::SurfaceSeparated {
            surface_distance_mm,
            closest_points,
        } => {
            if !surface_distance_mm.is_finite()
                || *surface_distance_mm < 0.0
                || !valid_pair(&closest_points.triangles)
                || closest_points
                    .point1_mm
                    .iter()
                    .chain(&closest_points.point2_mm)
                    .any(|value| !value.is_finite())
            {
                return Err("collision separated surface observation is invalid".into());
            }
        }
        CadCollisionObservation::Uncertain { reason, .. } => {
            if reason.trim().is_empty() {
                return Err("collision uncertain observation omitted reason".into());
            }
        }
    }
    Ok(())
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CadJobError {
    Stale(String),
    Blocked(String),
    Failed(String),
    Cancelled,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PreparedCadSnapshot {
    pub identity: CadSnapshotIdentity,
    pub prepared: PreparedCaseAssemblyIR,
    pub mechanical_assembly: Option<MechanicalAssembly>,
}

pub fn revision_supported_by_cad_boundary(revision: u64) -> bool {
    revision <= MAX_CAD_REVISION
}

/// Return consumer-owned geometry for the selected physical board instance.
/// The accepted snapshot remains immutable; callers use this scene alongside
/// the corresponding `PreparedCadSnapshot` for preview rendering.
#[cfg(feature = "page")]
pub fn captured_case_scene(
    snapshot: &AcceptedSnapshot,
    scope: &Scope,
) -> Result<SceneDelta, CadJobError> {
    captured_case_inputs(snapshot, scope).map(|(_, scene)| scene)
}

/// Return the consumer-owned document projection for a selected physical
/// board instance, including effective shared mechanical defaults and flip
/// transforms. The accepted snapshot is never mutated.
#[cfg(feature = "page")]
pub fn captured_case_document(
    snapshot: &AcceptedSnapshot,
    scope: &Scope,
) -> Result<ProjectDoc, CadJobError> {
    captured_case_inputs(snapshot, scope).map(|(document, _)| document)
}

#[cfg(feature = "page")]
#[derive(Clone, Debug, PartialEq)]
pub struct CapturedPcbSource {
    pub reference: BoardReference,
    pub asset: Asset,
    pub placement: Pose2,
    pub flipped: bool,
}

/// Capture raw PCB ownership before the Case display projection disables mirrored references.
#[cfg(feature = "page")]
pub fn captured_pcb_source(
    snapshot: &AcceptedSnapshot,
    scope: &Scope,
) -> Result<Option<CapturedPcbSource>, CadJobError> {
    let document = snapshot.document.as_ref();
    if snapshot.session_epoch != scope.session_epoch
        || document.id != scope.document_id
        || snapshot.scene.revision != document.revision
    {
        return Err(CadJobError::Stale("PCB source scope changed".into()));
    }
    if document
        .boards
        .iter()
        .filter(|board| board.id == scope.board_id)
        .count()
        != 1
    {
        return Err(CadJobError::Blocked(
            "PCB source board is missing or ambiguous".into(),
        ));
    }
    let flipped = if let Some(id) = &scope.instance_id {
        let instances = document
            .hardware
            .as_ref()
            .into_iter()
            .flat_map(|hardware| &hardware.instances)
            .filter(|instance| instance.id == *id && instance.board_id == scope.board_id)
            .collect::<Vec<_>>();
        if instances.len() != 1 {
            return Err(CadJobError::Blocked(
                "PCB source physical instance is missing or ambiguous".into(),
            ));
        }
        instances[0].flipped
    } else {
        false
    };
    let references = document
        .board_references
        .iter()
        .filter(|reference| reference.board_id == scope.board_id && reference.enabled)
        .collect::<Vec<_>>();
    if references.is_empty() {
        return Ok(None);
    }
    if references.len() != 1 {
        return Err(CadJobError::Blocked(
            "PCB source reference is ambiguous".into(),
        ));
    }
    let reference = references[0];
    if !reference.elevation.is_finite()
        || !reference.pose.at.x.is_finite()
        || !reference.pose.at.y.is_finite()
        || !reference.pose.rotation.is_finite()
    {
        return Err(CadJobError::Blocked(
            "PCB source placement must be finite".into(),
        ));
    }
    let assets = document
        .assets
        .iter()
        .filter(|asset| asset.id == reference.asset_id)
        .collect::<Vec<_>>();
    if assets.len() != 1 || assets[0].sha256.is_empty() {
        return Err(CadJobError::Blocked(
            "PCB source asset is missing or ambiguous".into(),
        ));
    }
    let mut placement = reference.pose;
    if flipped {
        // World-X reflection after placement equals reflected local X at (-tx, ty, -angle).
        placement.at.x = -placement.at.x;
        placement.rotation = -placement.rotation;
    }
    Ok(Some(CapturedPcbSource {
        reference: reference.clone(),
        asset: assets[0].clone(),
        placement,
        flipped,
    }))
}

#[cfg(feature = "page")]
fn captured_case_inputs(
    snapshot: &AcceptedSnapshot,
    scope: &Scope,
) -> Result<(ProjectDoc, SceneDelta), CadJobError> {
    if scope.session_epoch != snapshot.session_epoch
        || scope.document_id != snapshot.document.id
        || snapshot.scene.revision != snapshot.document.revision
    {
        return Err(CadJobError::Stale(
            "case scene scope or accepted revision changed".into(),
        ));
    }
    effective_case_inputs(&snapshot.document, &snapshot.scene, scope)
}

/// Resolve mechanical policy and prepare case regions using the public core protocol.
/// The CAD package uses JavaScript numbers for revision fields, so unsupported
/// wide identities fail before crossing that boundary.
#[cfg(all(target_arch = "wasm32", feature = "page"))]
pub async fn prepare_captured_case(
    core: &dyn crate::host::CoreExecutor,
    executor_epoch: &str,
    request_prefix: &str,
    snapshot: &AcceptedSnapshot,
    scope: &Scope,
) -> Result<PreparedCadSnapshot, CadJobError> {
    prepare_captured_case_for_step(core, executor_epoch, request_prefix, snapshot, scope, false)
        .await
}

/// Prepare a mechanical export assembly with the nominal PCB reference body,
/// matching the existing mechanical assembly export contract.
#[cfg(all(target_arch = "wasm32", feature = "page"))]
pub async fn prepare_captured_step_assembly(
    core: &dyn crate::host::CoreExecutor,
    executor_epoch: &str,
    request_prefix: &str,
    snapshot: &AcceptedSnapshot,
    scope: &Scope,
) -> Result<PreparedCadSnapshot, CadJobError> {
    prepare_captured_case_for_step(core, executor_epoch, request_prefix, snapshot, scope, true)
        .await
}

#[cfg(all(target_arch = "wasm32", feature = "page"))]
async fn prepare_captured_case_for_step(
    core: &dyn crate::host::CoreExecutor,
    executor_epoch: &str,
    request_prefix: &str,
    snapshot: &AcceptedSnapshot,
    scope: &Scope,
    include_pcb_reference: bool,
) -> Result<PreparedCadSnapshot, CadJobError> {
    let (identity, initial) = preparation_request(snapshot, scope, request_prefix)?;
    let reply = core
        .request(
            &format!("{request_prefix}-resolve"),
            executor_epoch,
            &initial,
        )
        .await
        .map_err(|error| CadJobError::Failed(error.to_string()))?;
    let mut pcb_reference = None;
    let mechanical_assembly;
    let prepare = match reply {
        CoreReply::CasePrepared { id: _, ir } => {
            return validate_prepared(identity, ir, None);
        }
        CoreReply::MechanicalResolved { assembly, .. } => {
            if assembly.revision != identity.revision || assembly.case.revision != identity.revision
            {
                return Err(CadJobError::Stale(
                    "core resolved another case revision".into(),
                ));
            }
            if assembly.generation_blocked {
                return Err(CadJobError::Blocked(
                    "mechanical findings block case generation".into(),
                ));
            }
            let errors = assembly
                .diagnostics
                .iter()
                .filter(|finding| finding.severity == Severity::Error)
                .map(|finding| finding.message.as_str())
                .collect::<Vec<_>>();
            if !errors.is_empty() {
                return Err(CadJobError::Blocked(errors.join("\n")));
            }
            if include_pcb_reference {
                let (document, contours) = match &initial {
                    CoreRequest::ResolveMechanical {
                        document, contours, ..
                    } => (document, contours),
                    _ => unreachable!("mechanical reply requires a mechanical request"),
                };
                let configuration = document.mechanical.as_ref().ok_or_else(|| {
                    CadJobError::Blocked("mechanical export has no selected configuration".into())
                })?;
                pcb_reference = Some(assembly.pcb_reference.clone().unwrap_or_else(|| CaseIR {
                    revision: identity.revision,
                    contours: contours.clone(),
                    body: CaseBody {
                        id: "pcb-reference".into(),
                        name: "PCB reference — unpopulated".into(),
                        board_id: configuration.board_id.clone(),
                        kind: CaseKind::Plate,
                        thickness: configuration.pcb_thickness,
                        clearance: 0.0,
                        material_id: None,
                        z: Some(-configuration.pcb_thickness),
                        wall_height: None,
                        wall_thickness: None,
                        mounts: None,
                        pcb_source: None,
                        gasket: None,
                        openings: None,
                        features: None,
                    },
                }));
            }
            mechanical_assembly = Some(assembly.clone());
            CoreRequest::PrepareCase {
                id: format!("{request_prefix}-prepare"),
                ir: assembly.case,
            }
        }
        CoreReply::Error { message, .. } => return Err(CadJobError::Blocked(message)),
        _ => {
            return Err(CadJobError::Failed(
                "unexpected core case preparation reply".into(),
            ));
        }
    };
    let reply = core
        .request(
            &format!("{request_prefix}-prepare"),
            executor_epoch,
            &prepare,
        )
        .await
        .map_err(|error| CadJobError::Failed(error.to_string()))?;
    let mut prepared = match reply {
        CoreReply::CasePrepared { ir, .. } => Ok(ir),
        CoreReply::Error { message, .. } => Err(CadJobError::Blocked(message)),
        _ => Err(CadJobError::Failed(
            "unexpected core case preparation reply".into(),
        )),
    }?;
    if let Some(reference) = pcb_reference {
        let request_id = format!("{request_prefix}-pcb-reference");
        let prepare_reference = CoreRequest::PrepareCase {
            id: request_id.clone(),
            ir: CaseAssemblyIR {
                revision: identity.revision,
                bodies: vec![reference],
            },
        };
        let reply = core
            .request(&request_id, executor_epoch, &prepare_reference)
            .await
            .map_err(|error| CadJobError::Failed(error.to_string()))?;
        match reply {
            CoreReply::CasePrepared { ir, .. } if ir.revision == identity.revision => {
                prepared.bodies.extend(ir.bodies);
            }
            CoreReply::CasePrepared { .. } => {
                return Err(CadJobError::Stale(
                    "PCB reference has another revision".into(),
                ));
            }
            CoreReply::Error { message, .. } => return Err(CadJobError::Blocked(message)),
            _ => {
                return Err(CadJobError::Failed(
                    "unexpected PCB reference preparation reply".into(),
                ));
            }
        }
    }
    validate_prepared(identity, prepared, mechanical_assembly)
}

#[cfg(all(feature = "page", any(target_arch = "wasm32", test)))]
fn validate_prepared(
    identity: CadSnapshotIdentity,
    prepared: PreparedCaseAssemblyIR,
    mechanical_assembly: Option<MechanicalAssembly>,
) -> Result<PreparedCadSnapshot, CadJobError> {
    if prepared.revision != identity.revision
        || prepared
            .bodies
            .iter()
            .any(|body| body.revision != identity.revision)
    {
        return Err(CadJobError::Stale(
            "prepared case does not match its captured revision".into(),
        ));
    }
    if prepared.bodies.is_empty() {
        return Err(CadJobError::Blocked(
            "the selected board has no case bodies".into(),
        ));
    }
    Ok(PreparedCadSnapshot {
        identity,
        prepared,
        mechanical_assembly,
    })
}

#[cfg(feature = "page")]
fn effective_case_inputs(
    document: &ProjectDoc,
    scene: &SceneDelta,
    scope: &Scope,
) -> Result<(ProjectDoc, SceneDelta), CadJobError> {
    let board = document
        .boards
        .iter()
        .find(|board| board.id == scope.board_id)
        .ok_or_else(|| {
            CadJobError::Blocked("selected board is not in the captured document".into())
        })?;
    let selected_instance = match &scope.instance_id {
        None => None,
        Some(instance_id) => {
            let instance = document
                .hardware
                .as_ref()
                .and_then(|hardware| {
                    hardware
                        .instances
                        .iter()
                        .find(|instance| instance.id == *instance_id)
                })
                .ok_or_else(|| {
                    CadJobError::Blocked("selected physical board instance is unavailable".into())
                })?;
            if instance.board_id != scope.board_id {
                return Err(CadJobError::Stale(
                    "selected instance belongs to another board".into(),
                ));
            }
            Some(instance)
        }
    };

    let flipped = selected_instance.is_some_and(|instance| instance.flipped);

    // Core owns the effective-document projection: the fully defaulted configuration,
    // the physical-instance identity, flipped-instance part/reference mirroring and the
    // target board's mirrored contours. The host keeps only the scene transform and
    // target-contour mirroring that are not part of the document projection.
    let target = match &scope.instance_id {
        Some(instance_id) => boardstudio_core::mechanical::MechanicalTarget::Instance {
            instance_id: instance_id.clone(),
        },
        None => boardstudio_core::mechanical::MechanicalTarget::Board {
            board_id: scope.board_id.clone(),
        },
    };
    let (effective, _target_contours) =
        boardstudio_core::mechanical::inputs::effective_document(document, &target)
            .map_err(CadJobError::Blocked)?;

    let mut effective_scene = scene.clone();
    if flipped {
        // The scene transform and contour mirroring are not part of the document
        // projection, so the host keeps them and mirrors the accepted scene's own
        // target-board contours in place. Core's `effective_document` returns the
        // same contours for its own resolver and clearance projection.
        let selected_parts = board
            .part_ids
            .iter()
            .collect::<std::collections::HashSet<_>>();
        for transform in &mut effective_scene.transforms {
            if selected_parts.contains(&transform.id) {
                transform.pose.at.x = -transform.pose.at.x;
                transform.pose.rotation = -transform.pose.rotation;
            }
        }
        for board_contours in &mut effective_scene.board_contours {
            if board_contours.board_id == scope.board_id {
                for contour in &mut board_contours.contours {
                    for point in &mut contour.points {
                        point.x = -point.x;
                    }
                    contour.points.reverse();
                }
            }
        }
    }
    Ok((effective, effective_scene))
}

/// Whether the selected target's own accepted findings leave its Case output clear.
///
/// Uses the approved target-scoped effective read, so a finding naming a sibling half of
/// the same board never blocks this target, and a finding naming this target does.
#[cfg(all(feature = "page", any(target_arch = "wasm32", test)))]
fn target_case_findings_are_clear(
    document: &ProjectDoc,
    scene: &SceneDelta,
    scope: &Scope,
) -> bool {
    let target = match &scope.instance_id {
        Some(instance_id) => boardstudio_core::mechanical::MechanicalTarget::Instance {
            instance_id: instance_id.clone(),
        },
        None => boardstudio_core::mechanical::MechanicalTarget::Board {
            board_id: scope.board_id.clone(),
        },
    };
    match boardstudio_core::mechanical::effective_inputs(document, &scene.board_contours, &target) {
        // Core owns the rule; the adapter only selects the target.
        Ok(inputs) => inputs.case_findings_are_clear(),
        // An unresolvable target is decided by the shared board-level admission.
        Err(_) => true,
    }
}

#[cfg(all(feature = "page", any(target_arch = "wasm32", test)))]
fn preparation_request(
    snapshot: &AcceptedSnapshot,
    scope: &Scope,
    request_prefix: &str,
) -> Result<(CadSnapshotIdentity, CoreRequest), CadJobError> {
    let document = &snapshot.document;
    let scene = &snapshot.scene;
    if scope.session_epoch != snapshot.session_epoch
        || scope.document_id != document.id
        || scene.revision != document.revision
    {
        return Err(CadJobError::Stale(
            "case preparation snapshot or scope changed".into(),
        ));
    }
    if !revision_supported_by_cad_boundary(document.revision) {
        return Err(CadJobError::Blocked(
            "case revision exceeds the exact integer range of the existing CAD JavaScript boundary"
                .into(),
        ));
    }
    if !document
        .boards
        .iter()
        .any(|board| board.id == scope.board_id)
    {
        return Err(CadJobError::Blocked(
            "selected board is not in the captured document".into(),
        ));
    }
    let (effective_document, effective_scene) = effective_case_inputs(document, scene, scope)?;
    let configured = effective_document
        .mechanical
        .as_ref()
        .is_some_and(|case| case.board_id == scope.board_id);
    // Admission is decided per selected target: the target-scoped effective read is the
    // authority for this half's Case problems. A conflict that names a sibling half of
    // the same PCB is not this target's problem, while this target's own conflict blocks
    // its case generation even though the board-level admission cannot see the half.
    if !boardstudio_core::case_preparation_ready(
        &effective_document,
        &effective_scene,
        &scope.board_id,
        configured,
    ) || !target_case_findings_are_clear(document, scene, scope)
    {
        return Err(CadJobError::Blocked(
            "selected board is not ready for case generation".into(),
        ));
    }
    let contours = effective_scene
        .board_contours
        .iter()
        .find(|entry| entry.board_id == scope.board_id)
        .map(|entry| entry.contours.clone())
        .unwrap_or_default();
    let request = if configured {
        CoreRequest::ResolveMechanical {
            id: format!("{request_prefix}-resolve"),
            document: effective_document.clone(),
            contours,
        }
    } else {
        let bodies = effective_document
            .case_bodies
            .iter()
            .filter(|body| body.board_id == scope.board_id)
            .cloned()
            .map(|body| CaseIR {
                revision: effective_document.revision,
                body,
                contours: contours.clone(),
            })
            .collect();
        CoreRequest::PrepareCase {
            id: format!("{request_prefix}-resolve"),
            ir: CaseAssemblyIR {
                revision: effective_document.revision,
                bodies,
            },
        }
    };
    Ok((
        CadSnapshotIdentity {
            token: snapshot.token.0,
            session_epoch: snapshot.session_epoch.0,
            document_id: effective_document.id.clone(),
            board_id: scope.board_id.clone(),
            instance_id: scope.instance_id.clone(),
            revision: document.revision,
        },
        request,
    ))
}

#[cfg(all(test, feature = "page"))]
mod tests {
    use super::*;
    use boardstudio_application::{SessionEpoch, SnapshotToken};
    use boardstudio_core::model::{
        Board, BoardContours, BoardReadiness, CaseBody, CaseKind, SceneDelta,
    };

    #[test]
    fn pcb_source_capture_keeps_original_reference_and_composes_physical_reflection_once() {
        let mut document = ProjectDoc::empty("source-doc", "PCB source");
        document.boards.push(
            serde_json::from_value(serde_json::json!({
                "id":"board", "name":"Board", "outlineIds":[],"partIds":[],"netIds":[],
                "thickness":1.6,"traces":[],"vias":[]
            }))
            .unwrap(),
        );
        document.assets.push(
            serde_json::from_value(serde_json::json!({
                "id":"raw", "name":"routed.kicad_pcb", "mediaType":"application/vnd.kicad.pcb",
                "sha256":"0".repeat(64)
            }))
            .unwrap(),
        );
        document.board_references.push(BoardReference {
            id: "reference".into(),
            board_id: "board".into(),
            asset_id: "raw".into(),
            enabled: true,
            pose: Pose2 {
                at: Vec2 { x: 100., y: 200. },
                rotation: 90.,
            },
            elevation: 3.,
            model_assets: Default::default(),
        });
        document.hardware = Some(
            serde_json::from_value(serde_json::json!({
                "topology":"unibody","transport":"none","boards":[],"instances":[{
                    "id":"right","name":"Right","boardId":"board","half":"right","role":"central",
                    "flipped":true,"controllerPartId":null,"mechanical":null
                }]
            }))
            .unwrap(),
        );
        let CoreReply::Scene {
            document, scene, ..
        } = CoreEngine::new().handle(CoreRequest::Open {
            id: "open".into(),
            document,
        })
        else {
            panic!("source fixture must open");
        };
        let snapshot = AcceptedSnapshot {
            token: SnapshotToken(1),
            session_epoch: SessionEpoch(1),
            document: std::sync::Arc::new(*document),
            scene: std::sync::Arc::new(scene),
        };
        let scope = Scope {
            session_epoch: SessionEpoch(1),
            document_id: "source-doc".into(),
            board_id: "board".into(),
            instance_id: Some("right".into()),
        };
        let before = snapshot.document.clone();
        let captured = captured_pcb_source(&snapshot, &scope).unwrap().unwrap();
        assert!(captured.flipped);
        assert_eq!(
            captured.placement,
            Pose2 {
                at: Vec2 { x: -100., y: 200. },
                rotation: -90.
            }
        );
        assert_eq!(captured.reference.pose.at.x, 100.);
        assert_eq!(captured.reference.elevation, 3.);
        assert_eq!(captured.asset.id, "raw");
        assert_eq!(
            snapshot.document, before,
            "capture must not disable or mutate the source"
        );

        let mut invalid = snapshot.clone();
        std::sync::Arc::make_mut(&mut invalid.document)
            .board_references
            .push(BoardReference {
                id: "second-reference".into(),
                ..captured.reference.clone()
            });
        assert!(matches!(
            captured_pcb_source(&invalid, &scope),
            Err(CadJobError::Blocked(_))
        ));
        let mut invalid = snapshot.clone();
        std::sync::Arc::make_mut(&mut invalid.document)
            .assets
            .clear();
        assert!(matches!(
            captured_pcb_source(&invalid, &scope),
            Err(CadJobError::Blocked(_))
        ));
        let mut invalid = snapshot.clone();
        std::sync::Arc::make_mut(&mut invalid.scene).revision += 1;
        assert!(matches!(
            captured_pcb_source(&invalid, &scope),
            Err(CadJobError::Stale(_))
        ));
        let mut authored = snapshot;
        std::sync::Arc::make_mut(&mut authored.document)
            .board_references
            .clear();
        assert!(captured_pcb_source(&authored, &scope).unwrap().is_none());
    }

    #[test]
    fn cad_revision_boundary_rejects_values_that_would_lose_integer_identity() {
        assert!(revision_supported_by_cad_boundary(MAX_CAD_REVISION));
        assert!(!revision_supported_by_cad_boundary(MAX_CAD_REVISION + 1));
        assert!(!revision_supported_by_cad_boundary(u64::MAX));
    }

    #[test]
    fn completed_cad_reply_requires_current_operation_and_valid_payload() {
        let identity = CadSnapshotIdentity {
            token: 3,
            session_epoch: 2,
            document_id: "doc".into(),
            board_id: "board".into(),
            instance_id: None,
            revision: 7,
        };
        let request = CadRequest {
            request_id: "req".into(),
            job_id: "job".into(),
            identity: identity.clone(),
            operation: CadOperation::ExportStep,
            prepared: None,
            source_sha256: None,
            step_import_options: None,
            collision_query: None,
            generated_analysis_request: None,
            collision_meshes: None,
            input_bytes: vec![],
        };
        let mut reply = CadReply {
            request_id: "req".into(),
            job_id: "job".into(),
            identity: identity.clone(),
            operation: CadOperation::ExportStep,
            outcome: CadReplyOutcome::Completed,
            result: Some(CadResult {
                revision: 7,
                step: b"ISO-10303-21;\nEND-ISO-10303-21;".to_vec(),
                ..CadResult::default()
            }),
            error: None,
        };
        assert!(validate_reply(&request, reply.clone(), &identity).is_ok());

        reply.operation = CadOperation::Exact;
        assert!(matches!(
            validate_reply(&request, reply.clone(), &identity),
            Err(CadJobError::Stale(_))
        ));
        reply.operation = CadOperation::ExportStep;
        reply.result.as_mut().unwrap().step = b"not STEP".to_vec();
        assert!(matches!(
            validate_reply(&request, reply, &identity),
            Err(CadJobError::Failed(_))
        ));
    }

    fn sample_vendor_step(sha256: &str) -> CadVendorStep {
        CadVendorStep {
            analysis_ref: None,
            analysis_mesh: None,
            source_qualification: None,
            source_sha256: sha256.into(),
            import_options: CadStepImportOptions::default(),
            surface_positions_mm_f64: vec![0., 0., 0., 1., 0., 0., 0., 1., 0.],
            source_faces: vec![CadStepSourceFace {
                solid_id: "10".into(),
                shell_index: 0,
                face_id: "20".into(),
                support_id: "30".into(),
                support_kind: "PLANE".into(),
            }],
            face_occurrences: vec![CadStepFaceOccurrence {
                source_face_index: 0,
                solid_occurrence_index: 0,
                representation_id: "40".into(),
                source_uncertainty_mm: Some(0.01),
                repair_displacement_mm: None,
            }],
            triangle_face_occurrence_indices: vec![0],
            line_segments_mm_f64: vec![],
            line_render_positions: vec![],
            line_occurrences: vec![],
            segment_line_occurrence_indices: vec![],
            precision: vec![CadStepPrecision {
                representation_id: "40".into(),
                context_id: "50".into(),
                geometric_dimension: Some(3),
                declarations: vec![CadStepUncertainty {
                    entity_id: "60".into(),
                    length_unit_id: "70".into(),
                    millimetres: 0.01,
                }],
            }],
            accuracy: CadStepAccuracy {
                preview_deflection_mm: 0.1,
                preview_angular_rad: 0.5,
                analysis_error_bound_mm: None,
                bounds_from_source_geometry: true,
                mesh_closed_oriented: false,
                preview_floor_mm: 0.02,
                preview_declared_multiplier: 2.0,
                relaxed_beyond_declared_precision: false,
                sampled_support_checks: 0,
                max_sampled_support_residual_mm: None,
                fallback_meshed_faces: 0,
                selected_repair_budget_mm: None,
                max_repair_displacement_mm: None,
            },
        }
    }

    #[test]
    fn read_step_reply_requires_canonical_source_identity() {
        let sha = "4bf5122f344554c53bde2ebb8cd2b7e3d1600ad631c385a5d7cce23c7785459a";
        let mut request_json = serde_json::json!({
            "requestId": "step-source", "jobId": "step-source",
            "identity": {"token": 1, "sessionEpoch": 1, "documentId": "doc",
                "boardId": "board", "instanceId": null, "revision": 7},
            "operation": "read-step", "prepared": null,
            "sourceSha256": sha
        });
        let request: CadRequest = serde_json::from_value(request_json.clone()).unwrap();
        let reply = CadReply {
            request_id: request.request_id.clone(),
            job_id: request.job_id.clone(),
            identity: request.identity.clone(),
            operation: CadOperation::ReadStep,
            outcome: CadReplyOutcome::Completed,
            result: Some(CadResult {
                revision: 7,
                mesh: Some(CadMesh {
                    positions: vec![0., 0., 0., 1., 0., 0., 0., 1., 0.],
                    normals: vec![0., 0., 1., 0., 0., 1., 0., 0., 1.],
                }),
                bounds: Some(CadBounds {
                    min: [0., 0., 0.],
                    max: [1., 1., 0.],
                }),
                vendor_step: Some(sample_vendor_step(sha)),
                ..CadResult::default()
            }),
            error: None,
        };
        assert!(validate_reply(&request, reply.clone(), &request.identity).is_ok());
        let mut missing_vendor_step = reply.clone();
        missing_vendor_step.result.as_mut().unwrap().vendor_step = None;
        assert!(matches!(
            validate_reply(&request, missing_vendor_step, &request.identity),
            Err(CadJobError::Failed(_))
        ));
        let mut mismatched_sha = reply.clone();
        mismatched_sha
            .result
            .as_mut()
            .unwrap()
            .vendor_step
            .as_mut()
            .unwrap()
            .source_sha256 = "0".repeat(64);
        assert!(matches!(
            validate_reply(&request, mismatched_sha, &request.identity),
            Err(CadJobError::Failed(_))
        ));
        for invalid in [
            serde_json::Value::Null,
            serde_json::json!(""),
            serde_json::json!("00"),
            serde_json::json!("4BF5122F344554C53BDE2EBB8CD2B7E3D1600AD631C385A5D7CCE23C7785459A"),
            serde_json::json!("gbf5122f344554c53bde2ebb8cd2b7e3d1600ad631c385a5d7cce23c7785459a"),
        ] {
            request_json["sourceSha256"] = invalid;
            let request: CadRequest = serde_json::from_value(request_json.clone()).unwrap();
            assert!(
                matches!(
                    validate_reply(&request, reply.clone(), &request.identity),
                    Err(CadJobError::Failed(_))
                ),
                "source identity must be canonical: {}",
                request_json["sourceSha256"]
            );
        }
    }

    #[test]
    fn read_step_reply_requires_the_captured_result_revision() {
        let sha = "4bf5122f344554c53bde2ebb8cd2b7e3d1600ad631c385a5d7cce23c7785459a";
        let identity = CadSnapshotIdentity {
            token: 3,
            session_epoch: 2,
            document_id: "doc".into(),
            board_id: "board".into(),
            instance_id: None,
            revision: 7,
        };
        let request = CadRequest {
            request_id: "read-step-request".into(),
            job_id: "read-step-job".into(),
            identity: identity.clone(),
            operation: CadOperation::ReadStep,
            prepared: None,
            source_sha256: Some(sha.into()),
            step_import_options: None,
            collision_query: None,
            generated_analysis_request: None,
            collision_meshes: None,
            input_bytes: vec![],
        };
        let mut reply = CadReply {
            request_id: request.request_id.clone(),
            job_id: request.job_id.clone(),
            identity: identity.clone(),
            operation: CadOperation::ReadStep,
            outcome: CadReplyOutcome::Completed,
            result: Some(CadResult {
                revision: 7,
                mesh: Some(CadMesh {
                    positions: vec![0., 0., 0., 1., 0., 0., 0., 1., 0.],
                    normals: vec![0., 0., 1., 0., 0., 1., 0., 0., 1.],
                }),
                bounds: Some(CadBounds {
                    min: [0., 0., 0.],
                    max: [1., 1., 0.],
                }),
                vendor_step: Some(sample_vendor_step(sha)),
                ..CadResult::default()
            }),
            error: None,
        };
        assert!(validate_reply(&request, reply.clone(), &identity).is_ok());
        for other_revision in [0, 6, 8] {
            reply.result.as_mut().unwrap().revision = other_revision;
            assert!(matches!(
                validate_reply(&request, reply.clone(), &identity),
                Err(CadJobError::Stale(_))
            ));
        }
    }

    #[test]
    fn exact_accepts_public_case_payload_without_optional_bounds() {
        let identity = CadSnapshotIdentity {
            token: 9,
            session_epoch: 4,
            document_id: "doc".into(),
            board_id: "board".into(),
            instance_id: None,
            revision: 7,
        };
        let request = CadRequest {
            request_id: "exact-request".into(),
            job_id: "exact-job".into(),
            identity: identity.clone(),
            operation: CadOperation::Exact,
            prepared: None,
            source_sha256: None,
            step_import_options: None,
            collision_query: None,
            generated_analysis_request: None,
            collision_meshes: None,
            input_bytes: vec![],
        };
        // The public provider's case-result payload contains revision, STEP,
        // mesh, and bodies; unlike its STEP import payload, it omits bounds.
        let reply = CadReply {
            request_id: request.request_id.clone(),
            job_id: request.job_id.clone(),
            identity: identity.clone(),
            operation: CadOperation::Exact,
            outcome: CadReplyOutcome::Completed,
            result: Some(CadResult {
                revision: 7,
                step: b"ISO-10303-21;\nEND-ISO-10303-21;".to_vec(),
                mesh: Some(CadMesh {
                    positions: vec![0.0, 0.0, 0.0],
                    normals: vec![0.0, 0.0, 1.0],
                }),
                bodies: vec![CadBodyMesh {
                    id: "case-body".into(),
                    name: "Case body".into(),
                    positions: vec![0.0, 0.0, 0.0],
                    normals: vec![0.0, 0.0, 1.0],
                }],
                bounds: None,
                ..CadResult::default()
            }),
            error: None,
        };

        assert!(validate_reply(&request, reply.clone(), &identity).is_ok());

        let mut malformed_bounds = reply.clone();
        malformed_bounds.result.as_mut().unwrap().bounds = Some(CadBounds {
            min: [2.0, 0.0, 0.0],
            max: [1.0, 0.0, 0.0],
        });
        assert!(matches!(
            validate_reply(&request, malformed_bounds, &identity),
            Err(CadJobError::Failed(_))
        ));

        let mut read_request = request;
        read_request.operation = CadOperation::ReadStep;
        read_request.source_sha256 =
            Some("4bf5122f344554c53bde2ebb8cd2b7e3d1600ad631c385a5d7cce23c7785459a".into());
        let mut read_reply = reply;
        read_reply.operation = CadOperation::ReadStep;
        assert!(matches!(
            validate_reply(&read_request, read_reply, &identity),
            Err(CadJobError::Failed(_))
        ));
    }

    #[test]
    fn flipped_instance_reflects_only_its_board_parts_and_scene_transform() {
        let mut document = ProjectDoc::empty("doc", "Fixture");
        document.revision = 4;
        document.outline.push(OutlineFeature::Rect {
            anchor_part_id: None,
            id: "outline-a".into(),
            center: Vec2 { x: 10.0, y: 0.0 },
            size: Vec2 { x: 20.0, y: 10.0 },
            radius: 0.0,
            rotation: None,
            operation: Operation::Add,
        });
        document.boards.push(Board {
            id: "board-a".into(),
            name: "Board A".into(),
            outline_ids: vec!["outline-a".into()],
            part_ids: vec!["part-a".into()],
            net_ids: vec![],
            thickness: 1.6,
            traces: vec![],
            vias: vec![],
        });
        document.boards.push(Board {
            id: "board-b".into(),
            name: "Board B".into(),
            outline_ids: vec![],
            part_ids: vec!["part-b".into()],
            net_ids: vec![],
            thickness: 1.6,
            traces: vec![],
            vias: vec![],
        });
        let part = |id: &str, x: f64| Part {
            id: id.into(),
            definition_id: "switch".into(),
            reference: id.into(),
            pose: Pose2 {
                at: Vec2 { x, y: 2.0 },
                rotation: 27.0,
            },
            side: Side::Front,
            anchor: None,
            locked: None,
            keycap: None,
            outline: None,
            properties: None,
            generator_parameters: None,
        };
        document.parts = vec![part("part-a", 3.0), part("part-b", 8.0)];
        document.hardware = Some(HardwareConfiguration {
            instances: vec![PhysicalBoardInstance {
                id: "instance-a".into(),
                name: "A, flipped".into(),
                board_id: "board-a".into(),
                half: "left".into(),
                role: "controller".into(),
                flipped: true,
                controller_part_id: None,
                mechanical: None,
                construction_ownership:
                    boardstudio_core::model::MechanicalConstructionOwnership::Legacy,
                mechanical_identity_reservation: None,
            }],
            ..HardwareConfiguration::default()
        });
        let scene = SceneDelta {
            revision: 4,
            transaction_id: String::new(),
            changed_ids: vec![],
            transforms: vec![
                Transform {
                    id: "part-a".into(),
                    pose: Pose2 {
                        at: Vec2 { x: 3.0, y: 2.0 },
                        rotation: 27.0,
                    },
                },
                Transform {
                    id: "part-b".into(),
                    pose: Pose2 {
                        at: Vec2 { x: 8.0, y: 2.0 },
                        rotation: 27.0,
                    },
                },
            ],
            matrix_scenes: vec![],
            contours: vec![],
            board_contours: vec![BoardContours {
                board_id: "board-a".into(),
                contours: vec![Contour {
                    points: vec![
                        Vec2 { x: 0.0, y: 5.0 },
                        Vec2 { x: 0.0, y: -5.0 },
                        Vec2 { x: 20.0, y: -5.0 },
                        Vec2 { x: 20.0, y: 5.0 },
                    ],
                    hole: false,
                }],
            }],
            board_readiness: vec![],
            board_outline_scenes: vec![],
            module_scenes: vec![],
            finding_markers: vec![],
            findings: vec![],
            readiness: Readiness {
                layout: true,
                outline: true,
                pcb: true,
                case_ready: true,
            },
        };
        let scope = Scope {
            session_epoch: SessionEpoch(1),
            document_id: "doc".into(),
            board_id: "board-a".into(),
            instance_id: Some("instance-a".into()),
        };
        let snapshot = AcceptedSnapshot {
            token: SnapshotToken(9),
            session_epoch: scope.session_epoch,
            document: std::sync::Arc::new(document.clone()),
            scene: std::sync::Arc::new(scene.clone()),
        };
        let captured_document = captured_case_document(&snapshot, &scope).unwrap();
        let captured_scene = captured_case_scene(&snapshot, &scope).unwrap();
        let (effective, scene) = effective_case_inputs(&document, &scene, &scope).unwrap();
        assert_eq!(effective.parts[0].pose.at.x, -3.0);
        assert_eq!(effective.parts[0].pose.rotation, -27.0);
        assert_eq!(effective.parts[0].side, Side::Back);
        assert_eq!(effective.parts[1], document.parts[1]);
        assert_eq!(scene.transforms[0].pose.at.x, -3.0);
        assert_eq!(scene.transforms[1].pose.at.x, 8.0);
        // The target board's scene contours come from Core's mirrored projection.
        let board_contours = scene
            .board_contours
            .iter()
            .find(|entry| entry.board_id == "board-a")
            .unwrap();
        assert!(!board_contours.contours.is_empty());
        assert!(
            board_contours
                .contours
                .iter()
                .flat_map(|contour| &contour.points)
                .all(|point| point.x <= 0.0),
            "the mirrored contours negate X"
        );
        assert_eq!(
            captured_document.physical_instance_id.as_deref(),
            Some("instance-a")
        );
        assert_eq!(captured_document, effective);
        assert_eq!(captured_scene, scene);
    }

    #[test]
    fn captured_document_and_scene_prepare_through_the_public_core_provider() {
        let mut document = ProjectDoc::empty("doc", "Fixture");
        document.revision = 7;
        document.boards.push(Board {
            id: "board".into(),
            name: "Board".into(),
            outline_ids: vec![],
            part_ids: vec![],
            net_ids: vec![],
            thickness: 1.6,
            traces: vec![],
            vias: vec![],
        });
        document.case_bodies.push(CaseBody {
            features: None,
            openings: None,
            id: "plate".into(),
            name: "Plate".into(),
            board_id: "board".into(),
            kind: CaseKind::Plate,
            thickness: 2.0,
            clearance: 0.0,
            material_id: Some("pla".into()),
            z: None,
            wall_height: None,
            wall_thickness: None,
            mounts: None,
            pcb_source: None,
            gasket: None,
        });
        let contour = Contour {
            points: vec![
                Vec2 { x: 0.0, y: 0.0 },
                Vec2 { x: 30.0, y: 0.0 },
                Vec2 { x: 30.0, y: 20.0 },
                Vec2 { x: 0.0, y: 20.0 },
            ],
            hole: false,
        };
        let scene = SceneDelta {
            revision: 7,
            transaction_id: String::new(),
            changed_ids: vec![],
            transforms: vec![],
            matrix_scenes: vec![],
            contours: vec![],
            board_contours: vec![BoardContours {
                board_id: "board".into(),
                contours: vec![contour],
            }],
            board_readiness: vec![BoardReadiness {
                board_id: "board".into(),
                outline: true,
                pcb: false,
                case_ready: false,
            }],
            board_outline_scenes: vec![],
            module_scenes: vec![],
            finding_markers: vec![],
            findings: vec![Finding {
                id: "case:plate:material".into(),
                severity: Severity::Error,
                scope: boardstudio_core::model::Scope::Case,
                message: "Case material is missing".into(),
                target_ids: vec!["plate".into(), "pla".into()],
            }],
            readiness: Readiness {
                layout: true,
                outline: true,
                pcb: false,
                case_ready: false,
            },
        };
        let snapshot = AcceptedSnapshot {
            token: SnapshotToken(3),
            session_epoch: SessionEpoch(2),
            document: std::sync::Arc::new(document),
            scene: std::sync::Arc::new(scene),
        };
        let scope = Scope {
            session_epoch: SessionEpoch(2),
            document_id: "doc".into(),
            board_id: "board".into(),
            instance_id: None,
        };
        let (identity, request) =
            preparation_request(&snapshot, &scope, "fixture").expect("captured request");
        let mut engine = CoreEngine::new();
        let reply = engine.handle(request);
        let CoreReply::CasePrepared { ir, .. } = reply else {
            panic!("core preparation failed: {reply:?}")
        };
        let prepared =
            validate_prepared(identity, ir, None).expect("revision guarded prepared case");
        assert_eq!(prepared.prepared.bodies[0].body.id, "plate");
        assert_eq!(prepared.prepared.bodies[0].regions.len(), 1);
        assert_eq!(prepared.prepared.bodies[0].regions[0].outer.len(), 4);
    }

    #[test]
    fn scope_or_scene_revision_mismatch_is_rejected_before_core_work() {
        let mut document = ProjectDoc::empty("doc", "Fixture");
        document.boards.push(Board {
            id: "board".into(),
            name: "Board".into(),
            outline_ids: vec![],
            part_ids: vec![],
            net_ids: vec![],
            thickness: 1.6,
            traces: vec![],
            vias: vec![],
        });
        let snapshot = AcceptedSnapshot {
            token: SnapshotToken(1),
            session_epoch: SessionEpoch(2),
            document: std::sync::Arc::new(document),
            scene: std::sync::Arc::new(SceneDelta {
                revision: 1,
                transaction_id: String::new(),
                changed_ids: vec![],
                transforms: vec![],
                matrix_scenes: vec![],
                contours: vec![],
                board_contours: vec![],
                board_readiness: vec![],
                board_outline_scenes: vec![],
                module_scenes: vec![],
                finding_markers: vec![],
                findings: vec![],
                readiness: Readiness {
                    layout: false,
                    outline: false,
                    pcb: false,
                    case_ready: false,
                },
            }),
        };
        let scope = Scope {
            session_epoch: SessionEpoch(2),
            document_id: "other".into(),
            board_id: "board".into(),
            instance_id: None,
        };
        assert!(matches!(
            preparation_request(&snapshot, &scope, "fixture"),
            Err(CadJobError::Stale(_))
        ));
    }

    /// Two halves sharing one PCB where the reflected linked mount clashes with the
    /// second half's own hardware. Admission must be decided per selected target.
    #[cfg(feature = "page")]
    #[test]
    fn case_admission_is_scoped_to_the_selected_half_of_a_shared_pcb() {
        use boardstudio_core::model::{
            HardwareConfiguration, HardwareTopology, HardwareTransport, MechanicalConfiguration,
            MechanicalConstructionOwnership, MechanicalFeature, MechanicalFeatureMember,
            MechanicalFeatureState, MechanicalFeatureValue, MechanicalMirrorPair, Mount, MountKind,
            PhysicalBoardInstance, Pose2,
        };
        fn configuration() -> MechanicalConfiguration {
            serde_json::from_value(serde_json::json!({
                "boardId": "board",
                "method": "printed",
                "mount": "rigid",
                "plateThickness": 1.5,
                "plateFoamThickness": 0.5,
                "pcbThickness": 1.6,
                "bottomFoamThickness": 0.4,
                "batteryHeight": 4.0,
                "bottomThickness": 3.0,
                "plateToPcb": 3.5,
                "wallThickness": 2.0,
                "clearance": 0.3,
                "bottomStyle": "shell",
                "middleFrame": false,
                "openingAllowance": 0.25,
                "mounts": [],
                "closureMounts": [],
                "profiles": []
            }))
            .unwrap()
        }
        fn mount(id: &str, x: f64, y: f64) -> Mount {
            Mount {
                id: id.into(),
                at: Vec2 { x, y },
                kind: MountKind::Hole,
                hole_diameter: 2.2,
                boss_diameter: None,
                height: None,
            }
        }
        fn instance(id: &str, mounts: Vec<Mount>) -> PhysicalBoardInstance {
            let mut mechanical = configuration();
            mechanical.mounts = mounts;
            PhysicalBoardInstance {
                id: id.into(),
                name: id.into(),
                board_id: "board".into(),
                half: id.into(),
                role: "controller".into(),
                flipped: false,
                controller_part_id: None,
                mechanical: Some(mechanical),
                construction_ownership: MechanicalConstructionOwnership::Local,
                mechanical_identity_reservation: None,
            }
        }

        let mut document = ProjectDoc::empty("doc", "Fixture");
        document.revision = 7;
        document.boards.push(Board {
            id: "board".into(),
            name: "Board".into(),
            outline_ids: vec![],
            part_ids: vec![],
            net_ids: vec![],
            thickness: 1.6,
            traces: vec![],
            vias: vec![],
        });
        document.case_bodies.push(CaseBody {
            features: None,
            openings: None,
            id: "plate".into(),
            name: "Plate".into(),
            board_id: "board".into(),
            kind: CaseKind::Plate,
            thickness: 2.0,
            clearance: 0.0,
            material_id: Some("pla".into()),
            z: None,
            wall_height: None,
            wall_thickness: None,
            mounts: None,
            pcb_source: None,
            gasket: None,
        });
        let left = instance("left", vec![mount("mount-1", 2.0, 3.0)]);
        let right = instance(
            "right",
            vec![
                mount("mount-1", 7.0, 3.0),
                mount("mount-encoder", -2.0, 3.0),
            ],
        );
        let construction = boardstudio_core::mechanical::features::shared_construction(
            left.mechanical.as_ref().unwrap(),
        );
        document.hardware = Some(HardwareConfiguration {
            topology: HardwareTopology::Unibody,
            transport: HardwareTransport::None,
            instances: vec![left, right],
            boards: vec![],
            shared_construction: None,
            pairs: vec![MechanicalMirrorPair {
                id: "pair-left-right".into(),
                members: ["left".into(), "right".into()],
                mapping: Pose2 {
                    at: Vec2 { x: 0.0, y: 0.0 },
                    rotation: 0.0,
                },
                construction,
                construction_source: None,
                halves: None,
                features: vec![MechanicalFeature {
                    id: "mount-1".into(),
                    kind: MechanicalFeatureKind::Mount,
                    author: "left".into(),
                    value: MechanicalFeatureValue::Mount {
                        mount: mount("mount-1", 2.0, 3.0),
                    },
                    members: vec![
                        MechanicalFeatureMember {
                            instance_id: "left".into(),
                            feature_id: "mount-1".into(),
                            state: MechanicalFeatureState::Linked,
                        },
                        MechanicalFeatureMember {
                            instance_id: "right".into(),
                            feature_id: "mount-1".into(),
                            state: MechanicalFeatureState::Linked,
                        },
                    ],
                }],
            }],
        });
        let scene = SceneDelta {
            revision: 7,
            transaction_id: String::new(),
            changed_ids: vec![],
            transforms: vec![],
            matrix_scenes: vec![],
            contours: vec![],
            board_contours: vec![BoardContours {
                board_id: "board".into(),
                contours: vec![Contour {
                    points: vec![
                        Vec2 { x: -40.0, y: -30.0 },
                        Vec2 { x: 40.0, y: -30.0 },
                        Vec2 { x: 40.0, y: 30.0 },
                        Vec2 { x: -40.0, y: 30.0 },
                    ],
                    hole: false,
                }],
            }],
            board_readiness: vec![BoardReadiness {
                board_id: "board".into(),
                outline: true,
                pcb: true,
                case_ready: false,
            }],
            board_outline_scenes: vec![],
            module_scenes: vec![],
            finding_markers: vec![],
            findings: vec![],
            readiness: Readiness {
                layout: true,
                outline: true,
                pcb: true,
                case_ready: false,
            },
        };
        let snapshot = AcceptedSnapshot {
            token: SnapshotToken(9),
            session_epoch: SessionEpoch(1),
            document: std::sync::Arc::new(document),
            scene: std::sync::Arc::new(scene),
        };
        let scope = |instance_id: Option<&str>| Scope {
            session_epoch: SessionEpoch(1),
            document_id: "doc".into(),
            board_id: "board".into(),
            instance_id: instance_id.map(str::to_string),
        };

        // The conflict belongs to the right half, so the left half is admitted.
        assert!(
            preparation_request(&snapshot, &scope(Some("left")), "fixture").is_ok(),
            "a sibling half's conflict must not block this half"
        );
        // The half that owns the conflict is blocked.
        assert!(matches!(
            preparation_request(&snapshot, &scope(Some("right")), "fixture"),
            Err(CadJobError::Blocked(_))
        ));
    }
}

#[cfg(test)]
mod generated_collision_identity_tests {
    use super::*;

    #[test]
    fn generated_analysis_checks_body_geometry_revision_accuracy_and_retained_tags() {
        let body: PreparedCaseIR = serde_json::from_value(serde_json::json!({
            "revision": 7, "body": {"id":"plate","name":"Plate","boardId":"board","kind":"plate","thickness":2.0,"clearance":0.0},
            "regions": [{"outer":[{"x":0,"y":0},{"x":1,"y":0},{"x":0,"y":1}],"holes":[],"cavities":[],"gaskets":[],"mounts":[]}]
        })).unwrap();
        let prepared = PreparedCaseAssemblyIR {
            revision: 7,
            bodies: vec![body.clone()],
        };
        let metadata = CadGeneratedAnalysisRequest {
            body_id: "plate".into(),
            geometry_identity: generated_geometry_identity(&body).unwrap(),
            target_deflection_mm: Some(0.01),
        };
        assert!(validate_generated_analysis_request(Some(&prepared), 7, &metadata).is_ok());
        let mut changed = prepared.clone();
        changed.bodies[0].body.thickness += 0.1;
        assert!(validate_generated_analysis_request(Some(&changed), 7, &metadata).is_err());
        assert!(validate_generated_analysis_request(Some(&prepared), 8, &metadata).is_err());
        let mut duplicate = prepared.clone();
        duplicate.bodies.push(body);
        assert!(validate_generated_analysis_request(Some(&duplicate), 7, &metadata).is_err());
        let analysis = CadGeneratedAnalysis {
            analysis_ref: None,
            analysis_mesh: None,
            source_qualification: None,
            body_id: "plate".into(),
            revision: 7,
            geometry_identity: metadata.geometry_identity.clone(),
            surface_positions_mm_f64: vec![0., 0., 0., 1., 0., 0., 0., 1., 0.],
            triangle_face_occurrence_indices: vec![0],
            source_faces: vec![CadGeneratedSourceFace {
                solid_index: 0,
                shell_index: 0,
                face_index: 4,
                support_kind: "plane".into(),
            }],
            face_occurrences: vec![CadGeneratedFaceOccurrence {
                source_face_index: 0,
                solid_occurrence_index: 0,
            }],
            analysis_error_bound_mm: None,
            mesh_closed_oriented: true,
            deflection_mm: 0.01,
        };
        assert!(validate_generated_analysis(&metadata, 7, &analysis).is_ok());
        let mut bad = analysis.clone();
        bad.deflection_mm = 0.1;
        assert!(
            validate_generated_analysis(&metadata, 7, &bad).is_err(),
            "coarse mesh cannot satisfy a tighter request"
        );
        let mut bad = analysis.clone();
        bad.triangle_face_occurrence_indices[0] = 1;
        assert!(validate_generated_analysis(&metadata, 7, &bad).is_err());
        let mut bad = analysis.clone();
        bad.geometry_identity.push('x');
        assert!(validate_generated_analysis(&metadata, 7, &bad).is_err());
        let mut bad = analysis;
        bad.analysis_error_bound_mm = Some(f64::NAN);
        assert!(validate_generated_analysis(&metadata, 7, &bad).is_err());
    }

    #[test]
    fn collision_operand_accepts_generated_identity_without_a_step_hash_and_rejects_ambiguity() {
        let fixture: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../cad/wasm/tests/fixtures/generated-analysis-contract.json"
        ))
        .unwrap();
        let mut operand =
            fixture["qualificationV2"]["activation"]["generatedQueryMetadata"]["left"].clone();
        let query = |operand: serde_json::Value| -> Result<(), String> {
            let query: CadCollisionQuery = serde_json::from_value(serde_json::json!({
                "left": operand, "right": operand, "clearanceThresholdMm": 0.0,
                "requestedAccuracyMm": null, "qualificationLimits": crate::cad_qualification::Limits::default()
            }))
            .map_err(|error| error.to_string())?;
            validate_collision_query(&query)
        };
        assert!(
            query(operand.clone()).is_ok(),
            "generated identity must not require a fake STEP SHA"
        );
        operand["sourceSha256"] = "a".repeat(64).into();
        assert!(
            query(operand.clone()).is_err(),
            "two identities are ambiguous"
        );
        operand.as_object_mut().unwrap().remove("sourceSha256");
        operand["importOptions"] = serde_json::json!({"previewRepairBudgetMm": 0.1});
        assert!(
            query(operand).is_err(),
            "generated geometry cannot carry STEP repair settings"
        );
    }
}
