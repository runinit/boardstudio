//! Typed consumer side of generated/ANALYSIS_CONTRACT.md qualificationV2.
//! Reports are accepted only at the worker reply boundary, never in query input.
use crate::cad_jobs::{
    CadCollisionMeshPair, CadCollisionOperand, CadCollisionQuery, CadSnapshotIdentity,
    CadStepImportOptions, MAX_CAD_REVISION, valid_source_sha256, validate_affine_row_transform,
};
use serde::{Deserialize, Deserializer, Serialize};

fn nullable<'de, D: Deserializer<'de>, T: Deserialize<'de>>(d: D) -> Result<Option<T>, D::Error> {
    Option::deserialize(d)
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ImportOptions {
    #[serde(deserialize_with = "nullable")]
    pub preview_repair_budget_mm: Option<f64>,
    #[serde(deserialize_with = "nullable")]
    pub target_deflection_mm: Option<f64>,
}

impl From<CadStepImportOptions> for ImportOptions {
    fn from(value: CadStepImportOptions) -> Self {
        Self { preview_repair_budget_mm: value.preview_repair_budget_mm, target_deflection_mm: value.target_deflection_mm }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", deny_unknown_fields)]
pub enum Source {
    #[serde(rename = "vendor-step", rename_all = "camelCase")]
    VendorStep { source_sha256: String, import_options: ImportOptions },
    #[serde(rename = "generated-body", rename_all = "camelCase")]
    GeneratedBody { body_id: String, revision: u64, geometry_identity: String },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AnalysisRef {
    pub worker_generation: String,
    pub mesh_identity: String,
    pub source: Source,
    pub derivation_identity: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MeshSettings {
    #[serde(deserialize_with = "nullable")]
    pub target_deflection_mm: Option<f64>,
    pub selected_deflection_mm: f64,
    pub angular_rad: f64,
    #[serde(deserialize_with = "nullable")]
    pub preview_repair_budget_mm: Option<f64>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "profile", deny_unknown_fields)]
pub enum SourceLimits {
    #[serde(rename = "planar-final-v2", rename_all = "camelCase")]
    Planar {
        max_source_bytes: u32, max_source_records: u32, max_decimal_characters: u32,
        minimum_decimal_exponent: i32, maximum_decimal_exponent: i32,
        minimum_exponent: i32, maximum_exponent: i32, max_final_triangles: u32,
        max_direct_bound_mm: f64, corner_snap_distance_squared_mm2: f64,
    },
    #[serde(rename = "analytic-generated-v1", rename_all = "camelCase")]
    Generated { max_final_triangles: u32, minimum_nonzero_magnitude: f64, maximum_magnitude: f64 },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AnalysisMesh {
    pub version: String,
    pub mesh_identity: String,
    pub source_proof: String,
    pub settings: MeshSettings,
    pub source_limits: SourceLimits,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "state", deny_unknown_fields)]
pub enum SourceQualification {
    #[serde(rename = "unknown")]
    Unknown { reasons: Vec<String> },
    #[serde(rename = "bounded", rename_all = "camelCase")]
    Bounded { class: String, version: u32, direct_bound_mm: f64, source_precision_mm: f64,
        total_bound_mm: f64, mesh_identity: String, accounting: String },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Snapshot {
    pub token: String,
    pub session_epoch: String,
    pub document_id: String,
    pub board_id: String,
    #[serde(deserialize_with = "nullable")]
    pub instance_id: Option<String>,
    pub revision: u64,
}

impl From<&CadSnapshotIdentity> for Snapshot {
    fn from(value: &CadSnapshotIdentity) -> Self {
        Self { token: value.token.to_string(), session_epoch: value.session_epoch.to_string(),
            document_id: value.document_id.clone(), board_id: value.board_id.clone(),
            instance_id: value.instance_id.clone(), revision: value.revision }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Binding {
    pub snapshot_revision: u64,
    pub snapshot: Snapshot,
    pub worker_generation: String,
    pub left_model_id: String,
    pub right_model_id: String,
    pub left_mesh_identity: String,
    pub right_mesh_identity: String,
    pub left_analysis: AnalysisRef,
    pub right_analysis: AnalysisRef,
    pub left_mesh: AnalysisMesh,
    pub right_mesh: AnalysisMesh,
    pub left_row_transform_f64: [f64; 16],
    pub right_row_transform_f64: [f64; 16],
    pub clearance_threshold_mm: f64,
    #[serde(deserialize_with = "nullable")]
    pub requested_accuracy_mm: Option<f64>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Limits {
    pub max_triangles: u32,
    pub max_node_pairs: u32,
    pub max_exact_pairs: u32,
    pub max_contact_boxes: u32,
    pub minimum_exponent: i32,
    pub maximum_exponent: i32,
    pub requested_numerical_width_mm: f64,
}

impl Default for Limits {
    fn default() -> Self {
        Self { max_triangles: 4096, max_node_pairs: 100_000, max_exact_pairs: 2048,
            max_contact_boxes: 8, minimum_exponent: -128, maximum_exponent: 128,
            requested_numerical_width_mm: 1e-10 }
    }
}

impl Limits {
    pub fn validate(&self) -> Result<(), String> {
        if self.max_triangles == 0 || self.max_triangles > 4096
            || self.max_node_pairs == 0 || self.max_node_pairs > 100_000
            || self.max_exact_pairs == 0 || self.max_exact_pairs > 2048
            || self.max_contact_boxes == 0 || self.max_contact_boxes > 8
            || self.minimum_exponent < -128 || self.maximum_exponent > 128
            || self.minimum_exponent >= self.maximum_exponent
            || !positive(self.requested_numerical_width_mm) {
            return Err("invalid qualification limits or unsupported cap increase".into());
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "state", deny_unknown_fields)]
pub enum MeshDistance {
    #[serde(rename = "unknown")]
    Unknown { reasons: Vec<String> },
    #[serde(rename = "bounded", rename_all = "camelCase")]
    Bounded { lower_mm: f64, upper_mm: f64, witness_triangles: [u32; 2] },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "state", deny_unknown_fields)]
pub enum WorldSourceBound {
    #[serde(rename = "unknown")]
    Unknown { reasons: Vec<String> },
    #[serde(rename = "bounded", rename_all = "camelCase")]
    Bounded { mesh_identity: String, method: String, norm_upper: f64, direct_bound_mm: f64,
        source_precision_mm: f64, transform_rounding_mm: f64, total_bound_mm: f64 },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ContactBoxPair {
    pub left_mm: [[f64; 3]; 2],
    pub right_mm: [[f64; 3]; 2],
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "state", deny_unknown_fields)]
pub enum ContactRegion {
    #[serde(rename = "unknown")]
    Unknown { reasons: Vec<String> },
    #[serde(rename = "boxes", rename_all = "camelCase")]
    Boxes { source_inflation_mm: [f64; 2], pairs: Vec<ContactBoxPair> },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "state", deny_unknown_fields)]
pub enum Containment {
    #[serde(rename = "unknown")]
    Unknown { reasons: Vec<String> },
    #[serde(rename = "ruledOut")]
    RuledOut { #[serde(flatten)] evidence: ContainmentExclusion },
    #[serde(rename = "containedLeft", rename_all = "camelCase")]
    ContainedLeft { correspondence_version: String },
    #[serde(rename = "containedRight", rename_all = "camelCase")]
    ContainedRight { correspondence_version: String },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "method", deny_unknown_fields)]
pub enum ContainmentExclusion {
    #[serde(rename = "slab", rename_all = "camelCase")]
    Slab { axis: u32, left_upper_mm: f64, right_lower_mm: f64 },
    #[serde(rename = "parity", rename_all = "camelCase")]
    Parity { correspondence_version: String, component_count: [u32; 2] },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct QueryQualification {
    pub version: u32,
    pub algorithm: String,
    pub binding: Binding,
    pub limits: Limits,
    pub mesh_distance: MeshDistance,
    pub source_bounds: [WorldSourceBound; 2],
    pub contact_region: ContactRegion,
    pub containment: Containment,
}

/// Only successful boundary validation creates the value assessment consumes.
#[derive(Clone, Debug, PartialEq)]
pub struct ValidatedQueryQualification(QueryQualification);

impl ValidatedQueryQualification {
    pub fn report(&self) -> &QueryQualification { &self.0 }
}

fn positive(value: f64) -> bool { value.is_finite() && value > 0.0 }
fn nonnegative(value: f64) -> bool { value.is_finite() && value >= 0.0 }
fn reasons_valid(reasons: &[String]) -> bool { !reasons.is_empty() && reasons.iter().all(|r| !r.trim().is_empty()) }
fn decimal(value: &str) -> bool { value.parse::<u64>().is_ok_and(|n| n.to_string() == value) }

/// Error-free addition determines whether the rounded endpoint needs one ULP.
pub fn add_up(a: f64, b: f64) -> f64 {
    let sum = a + b;
    let virtual_b = sum - a;
    let error = (a - (sum - virtual_b)) + (b - virtual_b);
    if error > 0.0 { sum.next_up() } else { sum }
}
pub fn add_down(a: f64, b: f64) -> f64 { -add_up(-a, -b) }
fn mul_up(a: f64, b: f64) -> f64 {
    let product = a * b;
    if a.mul_add(b, -product) > 0.0 { product.next_up() } else { product }
}

pub fn derivation_identity(source: &Source, mesh: &AnalysisMesh) -> String {
    let selected = |value: Option<f64>| value.map(|v| format!("{:016x}", v.to_bits()));
    match source {
        Source::GeneratedBody { geometry_identity, .. } => format!("{geometry_identity}/deflection={}/analytic-generated-v1", selected(mesh.settings.target_deflection_mm).unwrap_or_else(|| "default".into())),
        Source::VendorStep { source_sha256, import_options } => {
            let mut key = source_sha256.clone();
            if let Some(bits) = selected(import_options.preview_repair_budget_mm) { key.push_str(&format!("/repair-budget={bits}")); }
            if let Some(bits) = selected(import_options.target_deflection_mm) { key.push_str(&format!("/target-deflection={bits}")); }
            format!("vendor-step-analysis-v2/{key}")
        }
    }
}

pub fn validate_analysis(reference: &AnalysisRef, mesh: &AnalysisMesh, source: &SourceQualification, legacy_bound: Option<f64>) -> Result<(), String> {
    validate_descriptor(reference, mesh)?;
    match source {
        SourceQualification::Unknown { reasons } if reasons_valid(reasons) && legacy_bound.is_none() => Ok(()),
        SourceQualification::Bounded { class, version, direct_bound_mm, source_precision_mm, total_bound_mm, mesh_identity, accounting }
            if mesh_identity == &mesh.mesh_identity
                && accounting == "direct-includes-conversion-repair-trims-evaluation"
                && [*direct_bound_mm, *source_precision_mm, *total_bound_mm].into_iter().all(nonnegative)
                && *total_bound_mm >= add_up(*direct_bound_mm, *source_precision_mm)
                && legacy_bound.map(f64::to_bits) == Some(total_bound_mm.to_bits())
                && ((class == "analytic-generated" && *version == 1 && mesh.source_proof == "analytic-generated-v1")
                    || (class == "complete-native-line-plane" && *version == 2 && mesh.source_proof == "planar-final-v2")) => Ok(()),
        _ => Err("invalid source qualification, accounting or legacy bound".into()),
    }
}

pub fn validate_descriptor(reference: &AnalysisRef, mesh: &AnalysisMesh) -> Result<(), String> {
    if reference.worker_generation.trim().is_empty() || mesh.version != "final-f64-v1"
        || !mesh.mesh_identity.starts_with("final-f64-v1/") || mesh.mesh_identity.len() <= 13
        || reference.mesh_identity != mesh.mesh_identity
        || reference.derivation_identity != derivation_identity(&reference.source, mesh)
        || !positive(mesh.settings.selected_deflection_mm) || !positive(mesh.settings.angular_rad)
        || mesh.settings.target_deflection_mm.is_some_and(|v| !positive(v))
        || mesh.settings.preview_repair_budget_mm.is_some_and(|v| !positive(v)) {
        return Err("invalid analysis identity or settings".into());
    }
    match (&reference.source, &mesh.source_limits) {
        (Source::GeneratedBody { body_id, revision, geometry_identity }, SourceLimits::Generated { max_final_triangles, minimum_nonzero_magnitude, maximum_magnitude })
            if !body_id.trim().is_empty() && *revision <= MAX_CAD_REVISION
                && geometry_identity.strip_prefix("m1/generated-analysis-v2/").is_some_and(valid_source_sha256)
                && mesh.source_proof == "analytic-generated-v1" && *max_final_triangles > 0 && *max_final_triangles <= 10_000
                && positive(*minimum_nonzero_magnitude) && *minimum_nonzero_magnitude >= 1e-100
                && positive(*maximum_magnitude) && *maximum_magnitude <= 1e12 && minimum_nonzero_magnitude <= maximum_magnitude
                && mesh.settings.preview_repair_budget_mm.is_none()
                && mesh.settings.selected_deflection_mm.to_bits() == mesh.settings.target_deflection_mm.unwrap_or(0.1).min(0.1).to_bits() => Ok(()),
        (Source::VendorStep { source_sha256, import_options }, SourceLimits::Planar { max_source_bytes, max_source_records, max_decimal_characters, minimum_decimal_exponent, maximum_decimal_exponent, minimum_exponent, maximum_exponent, max_final_triangles, max_direct_bound_mm, corner_snap_distance_squared_mm2 })
            if valid_source_sha256(source_sha256) && mesh.source_proof == "planar-final-v2"
                && *max_source_bytes > 0 && *max_source_bytes <= 2*1024*1024 && *max_source_records > 0 && *max_source_records <= 10_000
                && *max_decimal_characters > 0 && *max_decimal_characters <= 80
                && *minimum_decimal_exponent >= -64 && *maximum_decimal_exponent <= 64 && minimum_decimal_exponent <= maximum_decimal_exponent
                && *minimum_exponent >= -128 && *maximum_exponent <= 128 && minimum_exponent <= maximum_exponent
                && *max_final_triangles > 0 && *max_final_triangles <= 4096
                && nonnegative(*max_direct_bound_mm) && *max_direct_bound_mm <= 1e-8
                && nonnegative(*corner_snap_distance_squared_mm2) && *corner_snap_distance_squared_mm2 <= 1e-16
                && import_options.target_deflection_mm.map(f64::to_bits) == mesh.settings.target_deflection_mm.map(f64::to_bits)
                && import_options.preview_repair_budget_mm.map(f64::to_bits) == mesh.settings.preview_repair_budget_mm.map(f64::to_bits) => Ok(()),
        _ => Err("source kind, proof, profile or caps do not match".into()),
    }
}

pub fn validate_operand(operand: &CadCollisionOperand) -> Result<(), String> {
    let reference = operand.analysis_ref.as_ref().ok_or("missing analysisRef")?;
    let mesh = operand.analysis_mesh.as_ref().ok_or("missing analysisMesh")?;
    validate_descriptor(reference, mesh)?;
    let matches = match &reference.source {
        Source::VendorStep { source_sha256, import_options } => operand.source_sha256.as_ref() == Some(source_sha256)
            && operand.generated_geometry_identity.is_none() && *import_options == operand.import_options.into(),
        Source::GeneratedBody { geometry_identity, .. } => operand.generated_geometry_identity.as_ref() == Some(geometry_identity)
            && operand.source_sha256.is_none() && operand.import_options == CadStepImportOptions::default(),
    };
    if !matches { return Err("operand source does not match its analysis reference".into()); }
    Ok(())
}

pub fn expected_binding(query: &CadCollisionQuery, snapshot: &CadSnapshotIdentity, generation: &str) -> Result<Binding, String> {
    let left = query.left.analysis_ref.as_ref().ok_or("missing left analysisRef")?;
    let right = query.right.analysis_ref.as_ref().ok_or("missing right analysisRef")?;
    Ok(Binding { snapshot_revision: snapshot.revision, snapshot: snapshot.into(), worker_generation: generation.into(),
        left_model_id: query.left.model_id.clone(), right_model_id: query.right.model_id.clone(),
        left_mesh_identity: left.mesh_identity.clone(), right_mesh_identity: right.mesh_identity.clone(),
        left_analysis: left.clone(), right_analysis: right.clone(),
        left_mesh: query.left.analysis_mesh.clone().ok_or("missing left analysisMesh")?,
        right_mesh: query.right.analysis_mesh.clone().ok_or("missing right analysisMesh")?,
        left_row_transform_f64: query.left.row_transform_f64, right_row_transform_f64: query.right.row_transform_f64,
        clearance_threshold_mm: query.clearance_threshold_mm, requested_accuracy_mm: query.requested_accuracy_mm })
}

pub fn unavailable(binding: Binding, limits: Limits) -> QueryQualification {
    let reasons = || vec!["evidence-unavailable".to_owned()];
    QueryQualification { version: 1, algorithm: "aabb-interval-exact-v1".into(), binding, limits,
        mesh_distance: MeshDistance::Unknown { reasons: reasons() },
        source_bounds: [WorldSourceBound::Unknown { reasons: reasons() }, WorldSourceBound::Unknown { reasons: reasons() }],
        contact_region: ContactRegion::Unknown { reasons: reasons() }, containment: Containment::Unknown { reasons: reasons() } }
}

pub fn validate_query_qualification(report: QueryQualification, expected: &Binding, limits: &Limits, meshes: &CadCollisionMeshPair, sources: Option<[&SourceQualification; 2]>) -> Result<ValidatedQueryQualification, String> {
    // JSON equality distinguishes signed zero, which PartialEq on floats cannot.
    if serde_json::to_vec(&report.binding).ok() != serde_json::to_vec(expected).ok()
        || report.version != 1 || report.algorithm != "aabb-interval-exact-v1" || &report.limits != limits {
        return Err("query qualification binding or effective limits mismatch".into());
    }
    limits.validate()?;
    let binding = &report.binding;
    if binding.snapshot_revision != binding.snapshot.revision || binding.snapshot.revision > MAX_CAD_REVISION
        || !decimal(&binding.snapshot.token) || !decimal(&binding.snapshot.session_epoch)
        || binding.snapshot.document_id.is_empty() || binding.snapshot.board_id.is_empty()
        || binding.snapshot.instance_id.as_ref().is_some_and(String::is_empty)
        || binding.worker_generation.is_empty() || binding.left_model_id.is_empty() || binding.right_model_id.is_empty()
        || !nonnegative(binding.clearance_threshold_mm) || binding.requested_accuracy_mm.is_some_and(|v| !positive(v)) {
        return Err("invalid complete query snapshot or request".into());
    }
    for (reference, mesh, identity, row) in [(&binding.left_analysis, &binding.left_mesh, &binding.left_mesh_identity, &binding.left_row_transform_f64), (&binding.right_analysis, &binding.right_mesh, &binding.right_mesh_identity, &binding.right_row_transform_f64)] {
        validate_descriptor(reference, mesh)?;
        validate_affine_row_transform(row)?;
        if identity != &mesh.mesh_identity { return Err("query mesh identity mismatch".into()); }
        if let Source::GeneratedBody { revision, .. } = reference.source
            && revision != binding.snapshot.revision { return Err("stale generated source revision".into()); }
    }
    let all_unavailable = matches!(&report.mesh_distance, MeshDistance::Unknown { reasons } if reasons == &["evidence-unavailable"])
        && report.source_bounds.iter().all(|s| matches!(s, WorldSourceBound::Unknown { reasons } if reasons == &["evidence-unavailable"]))
        && matches!(&report.contact_region, ContactRegion::Unknown { reasons } if reasons == &["evidence-unavailable"])
        && matches!(&report.containment, Containment::Unknown { reasons } if reasons == &["evidence-unavailable"]);
    if all_unavailable { return Ok(ValidatedQueryQualification(report)); }
    if [&binding.left_analysis, &binding.right_analysis].iter().any(|r| r.worker_generation != binding.worker_generation) {
        return Err("qualification uses another worker generation".into());
    }
    match &report.mesh_distance {
        MeshDistance::Unknown { reasons } if reasons_valid(reasons) => {},
        MeshDistance::Bounded { lower_mm, upper_mm, witness_triangles }
            if nonnegative(*lower_mm) && nonnegative(*upper_mm) && lower_mm <= upper_mm
                && (witness_triangles[0] as usize) < meshes.left_triangle_face_occurrence_indices.len()
                && (witness_triangles[1] as usize) < meshes.right_triangle_face_occurrence_indices.len()
                && meshes.left_triangle_face_occurrence_indices.len() <= limits.max_triangles as usize
                && meshes.right_triangle_face_occurrence_indices.len() <= limits.max_triangles as usize => {},
        _ => return Err("invalid mesh distance interval, seed or triangle cap".into()),
    }
    for (index, world) in report.source_bounds.iter().enumerate() {
        let mesh = [&binding.left_mesh, &binding.right_mesh][index];
        match world {
            WorldSourceBound::Unknown { reasons } if reasons_valid(reasons) => {},
            WorldSourceBound::Bounded { mesh_identity, method, norm_upper, direct_bound_mm, source_precision_mm, transform_rounding_mm, total_bound_mm } => {
                if mesh_identity != &mesh.mesh_identity || !positive(*norm_upper)
                    || ![*direct_bound_mm, *source_precision_mm, *transform_rounding_mm, *total_bound_mm].into_iter().all(nonnegative) {
                    return Err("invalid world source contribution".into());
                }
                let minimum = match method.as_str() {
                    "scaled-local-plus-row-rounding" => add_up(add_up(*direct_bound_mm, *source_precision_mm), *transform_rounding_mm),
                    "direct-world" => add_up(*direct_bound_mm, *source_precision_mm),
                    _ => return Err("unknown world accounting method".into()),
                };
                if *total_bound_mm < minimum { return Err("world bound omits contributions".into()); }
                if let Some(sources) = sources {
                    let SourceQualification::Bounded { direct_bound_mm: direct, source_precision_mm: precision, total_bound_mm: total, .. } = sources[index] else { return Err("unknown asset cannot acquire world source proof".into()); };
                    if *source_precision_mm < mul_up(*precision, *norm_upper)
                        || (method == "scaled-local-plus-row-rounding" && (*direct_bound_mm < mul_up(*direct, *norm_upper) || *total_bound_mm < add_up(mul_up(*total, *norm_upper), *transform_rounding_mm))) {
                        return Err("world source bound loses asset contributions".into());
                    }
                }
            },
            _ => return Err("invalid unknown world source reasons".into()),
        }
    }
    match &report.contact_region {
        ContactRegion::Unknown { reasons } if reasons_valid(reasons) => {},
        ContactRegion::Boxes { source_inflation_mm, pairs } => {
            if pairs.is_empty() || pairs.len() > limits.max_contact_boxes as usize { return Err("empty or over-cap contact region".into()); }
            for (index, bound) in report.source_bounds.iter().enumerate() {
                if !matches!(bound, WorldSourceBound::Bounded { total_bound_mm, .. } if total_bound_mm.to_bits() == source_inflation_mm[index].to_bits()) { return Err("contact inflation does not equal world source total".into()); }
            }
            if pairs.iter().any(|p| [&p.left_mm, &p.right_mm].into_iter().any(|b| (0..3).any(|i| !b[0][i].is_finite() || !b[1][i].is_finite() || b[0][i] > b[1][i]))) { return Err("invalid contact boxes".into()); }
        },
        _ => return Err("invalid contact reasons".into()),
    }
    match &report.containment {
        Containment::Unknown { reasons } if reasons_valid(reasons) => {},
        Containment::RuledOut { evidence: ContainmentExclusion::Slab { axis, left_upper_mm, right_lower_mm } }
            if *axis <= 2 && left_upper_mm.is_finite() && right_lower_mm.is_finite() && left_upper_mm < right_lower_mm
                && report.source_bounds.iter().all(|s| matches!(s, WorldSourceBound::Bounded { .. })) => {},
        // No provider currently implements the global correspondence needed here.
        _ => return Err("invalid or unavailable global containment evidence".into()),
    }
    Ok(ValidatedQueryQualification(report))
}
