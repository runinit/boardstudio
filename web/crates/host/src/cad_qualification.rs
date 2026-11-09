//! Typed consumer side of generated/ANALYSIS_CONTRACT.md qualificationV2.
//! Reports are accepted only at the worker reply boundary, never in query input.
use crate::cad_jobs::{
    CadCollisionMeshPair, CadCollisionOperand, CadCollisionQuery, CadSnapshotIdentity,
    CadStepImportOptions, MAX_CAD_REVISION, is_sha256_hex, validate_affine_row_transform,
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
        Self {
            preview_repair_budget_mm: value.preview_repair_budget_mm,
            target_deflection_mm: value.target_deflection_mm,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", deny_unknown_fields)]
pub enum Source {
    #[serde(rename = "vendor-step", rename_all = "camelCase")]
    VendorStep {
        source_sha256: String,
        import_options: ImportOptions,
    },
    #[serde(rename = "generated-body", rename_all = "camelCase")]
    GeneratedBody {
        body_id: String,
        revision: u64,
        geometry_identity: String,
    },
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
        max_source_bytes: u32,
        max_source_records: u32,
        max_decimal_characters: u32,
        minimum_decimal_exponent: i32,
        maximum_decimal_exponent: i32,
        minimum_exponent: i32,
        maximum_exponent: i32,
        max_final_triangles: u32,
        max_direct_bound_mm: f64,
        corner_snap_distance_squared_mm2: f64,
    },
    #[serde(rename = "analytic-generated-v1", rename_all = "camelCase")]
    Generated {
        max_final_triangles: u32,
        minimum_nonzero_magnitude: f64,
        maximum_magnitude: f64,
    },
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
    Bounded {
        class: String,
        version: u32,
        direct_bound_mm: f64,
        source_precision_mm: f64,
        total_bound_mm: f64,
        mesh_identity: String,
        accounting: String,
    },
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
        Self {
            token: value.token.to_string(),
            session_epoch: value.session_epoch.to_string(),
            document_id: value.document_id.clone(),
            board_id: value.board_id.clone(),
            instance_id: value.instance_id.clone(),
            revision: value.revision,
        }
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
        Self {
            max_triangles: 4096,
            max_node_pairs: 100_000,
            max_exact_pairs: 2048,
            max_contact_boxes: 8,
            minimum_exponent: -128,
            maximum_exponent: 128,
            requested_numerical_width_mm: 1e-10,
        }
    }
}

impl Limits {
    pub fn validate(&self) -> Result<(), String> {
        if self.max_triangles == 0
            || self.max_triangles > 4096
            || self.max_node_pairs == 0
            || self.max_node_pairs > 100_000
            || self.max_exact_pairs == 0
            || self.max_exact_pairs > 2048
            || self.max_contact_boxes == 0
            || self.max_contact_boxes > 8
            || self.minimum_exponent < -128
            || self.maximum_exponent > 128
            || self.minimum_exponent > self.maximum_exponent
            || !positive(self.requested_numerical_width_mm)
            || self.requested_numerical_width_mm > 1e-10
        {
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
    Bounded {
        lower_mm: f64,
        upper_mm: f64,
        witness_triangles: [u32; 2],
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "state", deny_unknown_fields)]
pub enum WorldSourceBound {
    #[serde(rename = "unknown")]
    Unknown { reasons: Vec<String> },
    #[serde(rename = "bounded", rename_all = "camelCase")]
    Bounded {
        mesh_identity: String,
        method: String,
        norm_upper: f64,
        direct_bound_mm: f64,
        source_precision_mm: f64,
        transform_rounding_mm: f64,
        total_bound_mm: f64,
    },
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
    Boxes {
        source_inflation_mm: [f64; 2],
        pairs: Vec<ContactBoxPair>,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(tag = "state")]
pub enum Containment {
    #[serde(rename = "unknown")]
    Unknown { reasons: Vec<String> },
    #[serde(rename = "ruledOut")]
    RuledOut {
        #[serde(flatten)]
        evidence: ContainmentExclusion,
    },
    #[serde(rename = "containedLeft", rename_all = "camelCase")]
    ContainedLeft { correspondence_version: String },
    #[serde(rename = "containedRight", rename_all = "camelCase")]
    ContainedRight { correspondence_version: String },
}

impl<'de> Deserialize<'de> for Containment {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        use serde::de::Error;
        let mut value = serde_json::Value::deserialize(deserializer)?;
        let object = value
            .as_object_mut()
            .ok_or_else(|| D::Error::custom("containment object required"))?;
        let state = object
            .remove("state")
            .ok_or_else(|| D::Error::custom("containment state required"))?;
        match state.as_str() {
            Some("ruledOut") => serde_json::from_value(value)
                .map(|evidence| Self::RuledOut { evidence })
                .map_err(D::Error::custom),
            Some("unknown") if object.len() == 1 && object.contains_key("reasons") => {
                serde_json::from_value(object["reasons"].clone())
                    .map(|reasons| Self::Unknown { reasons })
                    .map_err(D::Error::custom)
            }
            Some(direction @ ("containedLeft" | "containedRight"))
                if object.len() == 1 && object.contains_key("correspondenceVersion") =>
            {
                let correspondence_version =
                    serde_json::from_value(object["correspondenceVersion"].clone())
                        .map_err(D::Error::custom)?;
                Ok(if direction == "containedLeft" {
                    Self::ContainedLeft {
                        correspondence_version,
                    }
                } else {
                    Self::ContainedRight {
                        correspondence_version,
                    }
                })
            }
            _ => Err(D::Error::custom("invalid containment state or fields")),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "method", deny_unknown_fields)]
pub enum ContainmentExclusion {
    #[serde(rename = "slab", rename_all = "camelCase")]
    Slab {
        axis: u32,
        left_upper_mm: f64,
        right_lower_mm: f64,
    },
    #[serde(rename = "parity", rename_all = "camelCase")]
    Parity {
        correspondence_version: String,
        component_count: [u32; 2],
    },
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
    pub fn report(&self) -> &QueryQualification {
        &self.0
    }
}

fn positive(value: f64) -> bool {
    value.is_finite() && value > 0.0
}
fn nonnegative(value: f64) -> bool {
    value.is_finite() && value >= 0.0
}
fn reasons_valid(reasons: &[String]) -> bool {
    !reasons.is_empty() && reasons.iter().all(|r| !r.trim().is_empty())
}
fn decimal(value: &str) -> bool {
    value.parse::<u64>().is_ok_and(|n| n.to_string() == value)
}

/// Error-free addition determines whether the rounded endpoint needs one ULP.
pub fn add_up(a: f64, b: f64) -> f64 {
    let sum = a + b;
    let virtual_b = sum - a;
    let error = (a - (sum - virtual_b)) + (b - virtual_b);
    if error > 0.0 { sum.next_up() } else { sum }
}
pub fn add_down(a: f64, b: f64) -> f64 {
    -add_up(-a, -b)
}
fn mul_up(a: f64, b: f64) -> f64 {
    let product = a * b;
    if !product.is_finite() || a == 0.0 || b == 0.0 {
        return product;
    }
    if product < f64::MIN_POSITIVE {
        // A subnormal residual can underflow even in FMA. Compare integer
        // significands instead; their exact product occupies at most 106 bits.
        fn parts(value: f64) -> (u128, i32) {
            let bits = value.to_bits();
            let exponent = ((bits >> 52) & 2047) as i32;
            let fraction = (bits & ((1_u64 << 52) - 1)) as u128;
            if exponent == 0 {
                (fraction, -1074)
            } else {
                (fraction | (1_u128 << 52), exponent - 1075)
            }
        }
        let (am, ae) = parts(a);
        let (bm, be) = parts(b);
        let (pm, pe) = parts(product);
        let exact = am * bm;
        let exact_bits = 128 - exact.leading_zeros() as i32;
        let product_bits = 128 - pm.leading_zeros() as i32;
        let exact_high = exact_bits + ae + be;
        let product_high = product_bits + pe;
        let rounded_down = pm == 0
            || exact_high > product_high
            || (exact_high == product_high
                && if ae + be < pe {
                    exact > (pm << (pe - ae - be))
                } else {
                    (exact << (ae + be - pe)) > pm
                });
        return if rounded_down {
            product.next_up()
        } else {
            product
        };
    }
    if a.mul_add(b, -product) > 0.0 {
        product.next_up()
    } else {
        product
    }
}

// A short error-free expansion compares the exact sum, avoiding a second
// rounding tax on a producer that rounded its complete contribution sum once.
fn encloses_sum(bound: f64, terms: &[f64]) -> bool {
    let mut expansion = vec![bound];
    for term in terms {
        let mut q = -*term;
        let mut next = Vec::with_capacity(expansion.len() + 1);
        for component in expansion {
            let sum = q + component;
            if !sum.is_finite() {
                return false;
            }
            let virtual_component = sum - q;
            let error = (q - (sum - virtual_component)) + (component - virtual_component);
            if error != 0.0 {
                next.push(error);
            }
            q = sum;
        }
        if q != 0.0 || next.is_empty() {
            next.push(q);
        }
        expansion = next;
    }
    expansion.last().is_some_and(|value| *value >= 0.0)
}

fn encloses_scaled_plus_rounding(bound: f64, local: f64, norm: f64, rounding: f64) -> bool {
    let product = local * norm;
    if product < f64::MIN_POSITIVE && local > 0.0 && norm > 0.0 {
        return encloses_sum(bound, &[mul_up(local, norm), rounding]);
    }
    product.is_finite() && encloses_sum(bound, &[product, local.mul_add(norm, -product), rounding])
}

pub fn pair_error_budget(lower: f64, upper: f64, left: f64, right: f64) -> f64 {
    add_up(add_up(mul_up(add_up(upper, -lower), 0.5), left), right)
}

pub fn derivation_identity(source: &Source, mesh: &AnalysisMesh) -> String {
    let selected = |value: Option<f64>| value.map(|v| format!("{:016x}", v.to_bits()));
    match source {
        Source::GeneratedBody {
            geometry_identity, ..
        } => format!(
            "{geometry_identity}/deflection={}/analytic-generated-v1",
            selected(mesh.settings.target_deflection_mm).unwrap_or_else(|| "default".into())
        ),
        Source::VendorStep {
            source_sha256,
            import_options,
        } => {
            let mut key = source_sha256.clone();
            if let Some(bits) = selected(import_options.preview_repair_budget_mm) {
                key.push_str(&format!(":repair={bits}"));
            }
            if let Some(bits) = selected(import_options.target_deflection_mm) {
                key.push_str(&format!(":deflection={bits}"));
            }
            format!("vendor-step-analysis-v2/{key}")
        }
    }
}

pub fn validate_analysis(
    reference: &AnalysisRef,
    mesh: &AnalysisMesh,
    source: &SourceQualification,
    legacy_bound: Option<f64>,
) -> Result<(), String> {
    validate_descriptor(reference, mesh)?;
    if let (
        SourceQualification::Bounded {
            direct_bound_mm, ..
        },
        SourceLimits::Planar {
            max_direct_bound_mm,
            ..
        },
    ) = (source, &mesh.source_limits)
        && direct_bound_mm > max_direct_bound_mm
    {
        return Err("bounded source exceeds its direct admission cap".into());
    }
    match source {
        SourceQualification::Unknown { reasons }
            if reasons_valid(reasons) && legacy_bound.is_none() =>
        {
            Ok(())
        }
        SourceQualification::Bounded {
            class,
            version,
            direct_bound_mm,
            source_precision_mm,
            total_bound_mm,
            mesh_identity,
            accounting,
        } if mesh_identity == &mesh.mesh_identity
            && accounting == "direct-includes-conversion-repair-trims-evaluation"
            && [*direct_bound_mm, *source_precision_mm, *total_bound_mm]
                .into_iter()
                .all(nonnegative)
            && *total_bound_mm >= add_up(*direct_bound_mm, *source_precision_mm)
            && legacy_bound.map(f64::to_bits) == Some(total_bound_mm.to_bits())
            && ((class == "analytic-generated"
                && *version == 1
                && mesh.source_proof == "analytic-generated-v1")
                || (class == "complete-native-line-plane"
                    && *version == 2
                    && mesh.source_proof == "planar-final-v2")) =>
        {
            Ok(())
        }
        _ => Err("invalid source qualification, accounting or legacy bound".into()),
    }
}

pub fn validate_source_triangle_cap(
    mesh: &AnalysisMesh,
    source: &SourceQualification,
    count: usize,
) -> Result<(), String> {
    let cap = match mesh.source_limits {
        SourceLimits::Planar {
            max_final_triangles,
            ..
        }
        | SourceLimits::Generated {
            max_final_triangles,
            ..
        } => max_final_triangles,
    };
    if matches!(source, SourceQualification::Bounded { .. }) && count > cap as usize {
        return Err("bounded source exceeds its final triangle cap".into());
    }
    Ok(())
}

pub fn validate_descriptor(reference: &AnalysisRef, mesh: &AnalysisMesh) -> Result<(), String> {
    if reference.worker_generation.trim().is_empty()
        || mesh.version != "final-f64-v1"
        || !mesh.mesh_identity.starts_with("final-f64-v1/")
        || mesh.mesh_identity.len() <= 13
        || reference.mesh_identity != mesh.mesh_identity
        || reference.derivation_identity != derivation_identity(&reference.source, mesh)
        || !positive(mesh.settings.selected_deflection_mm)
        || !positive(mesh.settings.angular_rad)
        || mesh
            .settings
            .target_deflection_mm
            .is_some_and(|v| !positive(v))
        || mesh
            .settings
            .preview_repair_budget_mm
            .is_some_and(|v| !positive(v))
    {
        return Err("invalid analysis identity or settings".into());
    }
    match (&reference.source, &mesh.source_limits) {
        (
            Source::GeneratedBody {
                body_id,
                revision,
                geometry_identity,
            },
            SourceLimits::Generated {
                max_final_triangles,
                minimum_nonzero_magnitude,
                maximum_magnitude,
            },
        ) if !body_id.trim().is_empty()
            && *revision <= MAX_CAD_REVISION
            && geometry_identity
                .strip_prefix("m1/generated-analysis-v2/")
                .is_some_and(is_sha256_hex)
            && mesh.source_proof == "analytic-generated-v1"
            && *max_final_triangles > 0
            && *max_final_triangles <= 10_000
            && positive(*minimum_nonzero_magnitude)
            && *minimum_nonzero_magnitude >= 1e-100
            && positive(*maximum_magnitude)
            && *maximum_magnitude <= 1e12
            && minimum_nonzero_magnitude <= maximum_magnitude
            && mesh.settings.preview_repair_budget_mm.is_none()
            && mesh.settings.selected_deflection_mm.to_bits()
                == mesh
                    .settings
                    .target_deflection_mm
                    .unwrap_or(0.1)
                    .min(0.1)
                    .to_bits() =>
        {
            Ok(())
        }
        (
            Source::VendorStep {
                source_sha256,
                import_options,
            },
            SourceLimits::Planar {
                max_source_bytes,
                max_source_records,
                max_decimal_characters,
                minimum_decimal_exponent,
                maximum_decimal_exponent,
                minimum_exponent,
                maximum_exponent,
                max_final_triangles,
                max_direct_bound_mm,
                corner_snap_distance_squared_mm2,
            },
        ) if is_sha256_hex(source_sha256)
            && mesh.source_proof == "planar-final-v2"
            && *max_source_bytes > 0
            && *max_source_bytes <= 2 * 1024 * 1024
            && *max_source_records > 0
            && *max_source_records <= 10_000
            && *max_decimal_characters > 0
            && *max_decimal_characters <= 80
            && *minimum_decimal_exponent >= -64
            && *maximum_decimal_exponent <= 64
            && minimum_decimal_exponent <= maximum_decimal_exponent
            && *minimum_exponent >= -128
            && *maximum_exponent <= 128
            && minimum_exponent <= maximum_exponent
            && *max_final_triangles > 0
            && *max_final_triangles <= 4096
            && nonnegative(*max_direct_bound_mm)
            && *max_direct_bound_mm <= 1e-8
            && nonnegative(*corner_snap_distance_squared_mm2)
            && *corner_snap_distance_squared_mm2 <= 1e-16
            && import_options.target_deflection_mm.map(f64::to_bits)
                == mesh.settings.target_deflection_mm.map(f64::to_bits)
            && import_options.preview_repair_budget_mm.map(f64::to_bits)
                == mesh.settings.preview_repair_budget_mm.map(f64::to_bits) =>
        {
            Ok(())
        }
        _ => Err("source kind, proof, profile or caps do not match".into()),
    }
}

pub fn validate_operand(operand: &CadCollisionOperand) -> Result<(), String> {
    let reference = operand.analysis_ref.as_ref().ok_or("missing analysisRef")?;
    let mesh = operand
        .analysis_mesh
        .as_ref()
        .ok_or("missing analysisMesh")?;
    validate_descriptor(reference, mesh)?;
    let matches = match &reference.source {
        Source::VendorStep {
            source_sha256,
            import_options,
        } => {
            operand.source_sha256.as_ref() == Some(source_sha256)
                && operand.generated_geometry_identity.is_none()
                && *import_options == operand.import_options.into()
        }
        Source::GeneratedBody {
            geometry_identity, ..
        } => {
            operand.generated_geometry_identity.as_ref() == Some(geometry_identity)
                && operand.source_sha256.is_none()
                && operand.import_options == CadStepImportOptions::default()
        }
    };
    if !matches {
        return Err("operand source does not match its analysis reference".into());
    }
    Ok(())
}

pub fn expected_binding(
    query: &CadCollisionQuery,
    snapshot: &CadSnapshotIdentity,
    generation: &str,
) -> Result<Binding, String> {
    let left = query
        .left
        .analysis_ref
        .as_ref()
        .ok_or("missing left analysisRef")?;
    let right = query
        .right
        .analysis_ref
        .as_ref()
        .ok_or("missing right analysisRef")?;
    Ok(Binding {
        snapshot_revision: snapshot.revision,
        snapshot: snapshot.into(),
        worker_generation: generation.into(),
        left_model_id: query.left.model_id.clone(),
        right_model_id: query.right.model_id.clone(),
        left_mesh_identity: left.mesh_identity.clone(),
        right_mesh_identity: right.mesh_identity.clone(),
        left_analysis: left.clone(),
        right_analysis: right.clone(),
        left_mesh: query
            .left
            .analysis_mesh
            .clone()
            .ok_or("missing left analysisMesh")?,
        right_mesh: query
            .right
            .analysis_mesh
            .clone()
            .ok_or("missing right analysisMesh")?,
        left_row_transform_f64: query.left.row_transform_f64,
        right_row_transform_f64: query.right.row_transform_f64,
        clearance_threshold_mm: query.clearance_threshold_mm,
        requested_accuracy_mm: query.requested_accuracy_mm,
    })
}

pub fn unavailable(binding: Binding, limits: Limits) -> QueryQualification {
    let reasons = || vec!["evidence-unavailable".to_owned()];
    QueryQualification {
        version: 1,
        algorithm: "aabb-interval-exact-v1".into(),
        binding,
        limits,
        mesh_distance: MeshDistance::Unknown { reasons: reasons() },
        source_bounds: [
            WorldSourceBound::Unknown { reasons: reasons() },
            WorldSourceBound::Unknown { reasons: reasons() },
        ],
        contact_region: ContactRegion::Unknown { reasons: reasons() },
        containment: Containment::Unknown { reasons: reasons() },
    }
}

pub fn validate_query_qualification(
    report: QueryQualification,
    expected: &Binding,
    limits: &Limits,
    meshes: &CadCollisionMeshPair,
    sources: Option<[&SourceQualification; 2]>,
) -> Result<ValidatedQueryQualification, String> {
    crate::cad_jobs::validate_collision_mesh_pair(meshes)?;
    // JSON equality distinguishes signed zero, which PartialEq on floats cannot.
    if serde_json::to_vec(&report.binding).ok() != serde_json::to_vec(expected).ok()
        || report.version != 1
        || report.algorithm != "aabb-interval-exact-v1"
        || &report.limits != limits
    {
        return Err("query qualification binding or effective limits mismatch".into());
    }
    limits.validate()?;
    let binding = &report.binding;
    if binding.snapshot_revision != binding.snapshot.revision
        || binding.snapshot.revision > MAX_CAD_REVISION
        || !decimal(&binding.snapshot.token)
        || !decimal(&binding.snapshot.session_epoch)
        || binding.snapshot.document_id.is_empty()
        || binding.snapshot.board_id.is_empty()
        || binding
            .snapshot
            .instance_id
            .as_ref()
            .is_some_and(String::is_empty)
        || binding.worker_generation.is_empty()
        || binding.left_model_id.is_empty()
        || binding.right_model_id.is_empty()
        || !nonnegative(binding.clearance_threshold_mm)
        || binding.requested_accuracy_mm.is_some_and(|v| !positive(v))
    {
        return Err("invalid complete query snapshot or request".into());
    }
    for (reference, mesh, identity, row) in [
        (
            &binding.left_analysis,
            &binding.left_mesh,
            &binding.left_mesh_identity,
            &binding.left_row_transform_f64,
        ),
        (
            &binding.right_analysis,
            &binding.right_mesh,
            &binding.right_mesh_identity,
            &binding.right_row_transform_f64,
        ),
    ] {
        validate_descriptor(reference, mesh)?;
        validate_affine_row_transform(row).map_err(|error| error.to_string())?;
        if identity != &mesh.mesh_identity {
            return Err("query mesh identity mismatch".into());
        }
        if let Source::GeneratedBody { revision, .. } = reference.source
            && revision != binding.snapshot.revision
        {
            return Err("stale generated source revision".into());
        }
    }
    let all_unavailable = matches!(&report.mesh_distance, MeshDistance::Unknown { reasons } if reasons == &["evidence-unavailable"])
        && report.source_bounds.iter().all(|s| matches!(s, WorldSourceBound::Unknown { reasons } if reasons == &["evidence-unavailable"]))
        && matches!(&report.contact_region, ContactRegion::Unknown { reasons } if reasons == &["evidence-unavailable"])
        && matches!(&report.containment, Containment::Unknown { reasons } if reasons == &["evidence-unavailable"]);
    if all_unavailable {
        return Ok(ValidatedQueryQualification(report));
    }
    if [&binding.left_analysis, &binding.right_analysis]
        .iter()
        .any(|r| r.worker_generation != binding.worker_generation)
    {
        return Err("qualification uses another worker generation".into());
    }
    let preflight_refusal = [
        (
            &meshes.left_positions_mm_f64,
            &binding.left_row_transform_f64,
        ),
        (
            &meshes.right_positions_mm_f64,
            &binding.right_row_transform_f64,
        ),
    ]
    .into_iter()
    .any(|(positions, row)| {
        positions.len() / 9 > limits.max_triangles as usize
            || positions.as_chunks::<3>().0.iter().any(|p| {
                (0..3).any(|axis| {
                    // Match the provider's actual f64 row operation order for its domain
                    // check; this does not create a source or transform-rounding proof.
                    let value = row[axis] * p[0]
                        + row[axis + 4] * p[1]
                        + row[axis + 8] * p[2]
                        + row[axis + 12];
                    !value.is_finite()
                        || (value != 0.0
                            && !(2_f64.powi(limits.minimum_exponent)
                                ..=2_f64.powi(limits.maximum_exponent))
                                .contains(&value.abs()))
                })
            })
    });
    if preflight_refusal
        && !(matches!(report.mesh_distance, MeshDistance::Unknown { .. })
            && report
                .source_bounds
                .iter()
                .all(|s| matches!(s, WorldSourceBound::Unknown { .. }))
            && matches!(report.contact_region, ContactRegion::Unknown { .. })
            && matches!(report.containment, Containment::Unknown { .. }))
    {
        return Err(
            "preflight cap refusal must leave every qualification component unknown".into(),
        );
    }
    match &report.mesh_distance {
        MeshDistance::Unknown { reasons } if reasons_valid(reasons) => {}
        MeshDistance::Bounded {
            lower_mm,
            upper_mm,
            witness_triangles,
        } if nonnegative(*lower_mm)
            && nonnegative(*upper_mm)
            && lower_mm <= upper_mm
            && add_up(*upper_mm, -*lower_mm) <= limits.requested_numerical_width_mm
            && (witness_triangles[0] as usize)
                < meshes.left_triangle_face_occurrence_indices.len()
            && (witness_triangles[1] as usize)
                < meshes.right_triangle_face_occurrence_indices.len()
            && meshes.left_triangle_face_occurrence_indices.len()
                <= limits.max_triangles as usize
            && meshes.right_triangle_face_occurrence_indices.len()
                <= limits.max_triangles as usize =>
        {
            let world_box = |positions: &[f64], triangle: u32, row: &[f64; 16]| {
                let mut bounds = [[f64::INFINITY; 3], [f64::NEG_INFINITY; 3]];
                for p in positions[triangle as usize * 9..triangle as usize * 9 + 9]
                    .as_chunks::<3>()
                    .0
                {
                    for axis in 0..3 {
                        let world = row[axis] * p[0]
                            + row[axis + 4] * p[1]
                            + row[axis + 8] * p[2]
                            + row[axis + 12];
                        bounds[0][axis] = bounds[0][axis].min(world);
                        bounds[1][axis] = bounds[1][axis].max(world);
                    }
                }
                bounds
            };
            let left = world_box(
                &meshes.left_positions_mm_f64,
                witness_triangles[0],
                &binding.left_row_transform_f64,
            );
            let right = world_box(
                &meshes.right_positions_mm_f64,
                witness_triangles[1],
                &binding.right_row_transform_f64,
            );
            if (0..3).any(|axis| {
                add_down(left[0][axis], -right[1][axis])
                    .max(add_down(right[0][axis], -left[1][axis]))
                    > *upper_mm
            }) {
                return Err("named witness triangles cannot support the query upper bound".into());
            }
        }
        _ => return Err("invalid mesh distance interval, seed or triangle cap".into()),
    }
    for (index, world) in report.source_bounds.iter().enumerate() {
        let mesh = [&binding.left_mesh, &binding.right_mesh][index];
        match world {
            WorldSourceBound::Unknown { reasons } if reasons_valid(reasons) => {}
            WorldSourceBound::Bounded {
                mesh_identity,
                method,
                norm_upper,
                direct_bound_mm,
                source_precision_mm,
                transform_rounding_mm,
                total_bound_mm,
            } => {
                if mesh_identity != &mesh.mesh_identity
                    || !positive(*norm_upper)
                    || ![
                        *direct_bound_mm,
                        *source_precision_mm,
                        *transform_rounding_mm,
                        *total_bound_mm,
                    ]
                    .into_iter()
                    .all(nonnegative)
                {
                    return Err("invalid world source contribution".into());
                }
                let row = [
                    &binding.left_row_transform_f64,
                    &binding.right_row_transform_f64,
                ][index];
                if [0, 1, 2, 4, 5, 6, 8, 9, 10]
                    .into_iter()
                    .any(|i| row[i].abs() > *norm_upper)
                {
                    return Err("world norm does not enclose the affine matrix entries".into());
                }
                let complete = match method.as_str() {
                    "scaled-local-plus-row-rounding" => encloses_sum(
                        *total_bound_mm,
                        &[
                            *direct_bound_mm,
                            *source_precision_mm,
                            *transform_rounding_mm,
                        ],
                    ),
                    "direct-world" => {
                        encloses_sum(*total_bound_mm, &[*direct_bound_mm, *source_precision_mm])
                    }
                    _ => return Err("unknown world accounting method".into()),
                };
                if !complete {
                    return Err("world bound omits contributions".into());
                }
                if let Some(sources) = sources {
                    let SourceQualification::Bounded {
                        direct_bound_mm: direct,
                        source_precision_mm: precision,
                        total_bound_mm: total,
                        ..
                    } = sources[index]
                    else {
                        return Err("unknown asset cannot acquire world source proof".into());
                    };
                    if *source_precision_mm < mul_up(*precision, *norm_upper)
                        || (method == "scaled-local-plus-row-rounding"
                            && (*direct_bound_mm < mul_up(*direct, *norm_upper)
                                || !encloses_scaled_plus_rounding(
                                    *total_bound_mm,
                                    *total,
                                    *norm_upper,
                                    *transform_rounding_mm,
                                )))
                    {
                        return Err("world source bound loses asset contributions".into());
                    }
                }
            }
            _ => return Err("invalid unknown world source reasons".into()),
        }
    }
    match &report.contact_region {
        ContactRegion::Unknown { reasons } if reasons_valid(reasons) => {}
        ContactRegion::Boxes {
            source_inflation_mm,
            pairs,
        } => {
            if pairs.is_empty() || pairs.len() > limits.max_contact_boxes as usize {
                return Err("empty or over-cap contact region".into());
            }
            for (index, bound) in report.source_bounds.iter().enumerate() {
                if !matches!(bound, WorldSourceBound::Bounded { total_bound_mm, .. } if total_bound_mm.to_bits() == source_inflation_mm[index].to_bits())
                {
                    return Err("contact inflation does not equal world source total".into());
                }
            }
            if pairs.iter().any(|p| {
                [&p.left_mm, &p.right_mm].into_iter().any(|b| {
                    (0..3)
                        .any(|i| !b[0][i].is_finite() || !b[1][i].is_finite() || b[0][i] > b[1][i])
                })
            }) {
                return Err("invalid contact boxes".into());
            }
        }
        _ => return Err("invalid contact reasons".into()),
    }
    match &report.containment {
        Containment::Unknown { reasons } if reasons_valid(reasons) => {}
        Containment::RuledOut {
            evidence:
                ContainmentExclusion::Slab {
                    axis,
                    left_upper_mm,
                    right_lower_mm,
                },
        } if *axis <= 2
            && left_upper_mm.is_finite()
            && right_lower_mm.is_finite()
            && left_upper_mm < right_lower_mm
            && report
                .source_bounds
                .iter()
                .all(|s| matches!(s, WorldSourceBound::Bounded { .. })) => {}
        // No provider currently implements the global correspondence needed here.
        _ => return Err("invalid or unavailable global containment evidence".into()),
    }
    Ok(ValidatedQueryQualification(report))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> serde_json::Value {
        serde_json::from_str(include_str!(
            "../../../../cad/wasm/tests/fixtures/generated-analysis-contract.json"
        ))
        .unwrap()
    }

    fn controlled_arrays(
        fixture: &serde_json::Value,
        report: &QueryQualification,
    ) -> CadCollisionMeshPair {
        // Canonical generated arrays and small feasible consumer seed controls.
        // Vendor controls are not the vendor producer's geometry or a proof.
        let plane: Vec<f64> =
            serde_json::from_value(fixture["directPayload"]["surfacePositionsMmF64"].clone())
                .unwrap();
        let plane_tags: Vec<u32> = serde_json::from_value(
            fixture["directPayload"]["triangleFaceOccurrenceIndices"].clone(),
        )
        .unwrap();
        let vendor = if matches!(
            report.binding.right_analysis.source,
            Source::GeneratedBody { .. }
        ) {
            vec![0., 0., 1.35, 1., 0., 1.35, 0., 1., 1.35]
        } else {
            vec![-0.625, -1., 0., -0.625, 1., 0., 0.625, -1., 0.]
        };
        let left_generated = matches!(
            report.binding.left_analysis.source,
            Source::GeneratedBody { .. }
        );
        let right_generated = matches!(
            report.binding.right_analysis.source,
            Source::GeneratedBody { .. }
        );
        CadCollisionMeshPair {
            left_positions_mm_f64: if left_generated {
                plane.clone()
            } else {
                vendor.clone()
            },
            left_triangle_face_occurrence_indices: if left_generated {
                plane_tags.clone()
            } else {
                vec![0]
            },
            right_positions_mm_f64: if right_generated { plane } else { vendor },
            right_triangle_face_occurrence_indices: if right_generated {
                plane_tags
            } else {
                vec![0]
            },
        }
    }

    #[test]
    fn shared_v2_reports_and_mutations_validate_at_the_consumer_boundary() {
        let fixture = fixture();
        let activation = &fixture["qualificationV2"]["activation"];
        // These are shape/consumer reports, never constructors for trusted handles.
        for value in [
            &activation["generatedPlaneQuery"],
            &activation["mixedVendorGeneratedQuery"],
            &fixture["qualificationV2"]["queryExamples"],
        ] {
            let report: QueryQualification = serde_json::from_value(value.clone()).unwrap();
            let arrays = controlled_arrays(&fixture, &report);
            validate_query_qualification(
                report.clone(),
                &report.binding,
                &report.limits,
                &arrays,
                None,
            )
            .unwrap();
        }
        let original: QueryQualification =
            serde_json::from_value(fixture["qualificationV2"]["queryExamples"].clone()).unwrap();
        let arrays = controlled_arrays(&fixture, &original);
        for mutation in activation["invalidMutations"].as_array().unwrap() {
            let mut bad = fixture["qualificationV2"]["queryExamples"].clone();
            let path = mutation["path"].as_str().unwrap();
            *bad.pointer_mut(path).unwrap() = mutation["value"].clone();
            let accepted = serde_json::from_value::<QueryQualification>(bad)
                .ok()
                .is_some_and(|report| {
                    validate_query_qualification(
                        report,
                        &original.binding,
                        &original.limits,
                        &arrays,
                        None,
                    )
                    .is_ok()
                });
            assert!(!accepted, "accepted {}", mutation["reason"]);
        }
        for name in ["plane", "vendor", "cylinderDefault", "cylinderRefined"] {
            let reference: AnalysisRef =
                serde_json::from_value(activation["workerReferences"][name].clone()).unwrap();
            let mesh: AnalysisMesh =
                serde_json::from_value(activation["descriptors"][name].clone()).unwrap();
            let source: SourceQualification =
                serde_json::from_value(activation["sourceReports"][name].clone()).unwrap();
            let bound = match source {
                SourceQualification::Bounded { total_bound_mm, .. } => Some(total_bound_mm),
                _ => None,
            };
            validate_analysis(&reference, &mesh, &source, bound).unwrap();
        }
    }

    #[test]
    fn snapshot_decimals_nullable_keys_unknown_fields_and_cap_reductions_are_strict() {
        let fixture = fixture();
        let activation = &fixture["qualificationV2"]["activation"];
        for key in activation["rejectedQueryKeys"].as_array().unwrap() {
            let mut query = activation["generatedQueryMetadata"].clone();
            query[key.as_str().unwrap()] = serde_json::Value::Null;
            assert!(serde_json::from_value::<CadCollisionQuery>(query).is_err());
        }
        let mut report = fixture["qualificationV2"]["activation"]["generatedPlaneQuery"].clone();
        report["binding"]["snapshot"]["token"] = u64::MAX.to_string().into();
        report["binding"]["snapshot"]["sessionEpoch"] = (u64::MAX - 1).to_string().into();
        let typed: QueryQualification = serde_json::from_value(report.clone()).unwrap();
        assert_eq!(typed.binding.snapshot.token, u64::MAX.to_string());
        report["binding"]["snapshot"]
            .as_object_mut()
            .unwrap()
            .remove("instanceId");
        assert!(serde_json::from_value::<QueryQualification>(report).is_err());
        let mut bad = fixture["qualificationV2"]["activation"]["generatedPlaneQuery"].clone();
        bad["sourceBounds"][0]["callerCertificate"] = true.into();
        assert!(serde_json::from_value::<QueryQualification>(bad).is_err());
        let mut limits = Limits {
            max_exact_pairs: 1,
            ..Limits::default()
        };
        limits.validate().unwrap();
        limits.max_exact_pairs = 2049;
        assert!(limits.validate().is_err());
        let mut limits = Limits {
            minimum_exponent: 0,
            maximum_exponent: 0,
            ..Limits::default()
        };
        limits.validate().unwrap();
        limits.requested_numerical_width_mm = 1e-10_f64.next_up();
        assert!(limits.validate().is_err());
        assert_eq!(add_up(0.0, 0.0), 0.0);
        assert!(add_up(0.01, 1e-10) >= 0.010000000100000002);
        assert_eq!(add_down(1.0, -0.0), 1.0);
        assert!(!encloses_sum(0.3, &[0.1, 0.2]));
        assert!(encloses_sum(0.3_f64.next_up(), &[0.1, 0.2]));
        // Exact-Fraction oracle: the full three-term sum rounds to this value;
        // rounding the first addition separately would incorrectly add one ULP.
        assert!(encloses_sum(0.010000000100000002, &[1e-10, 0.01, 1e-20]));
        assert!(!encloses_sum(
            0.010000000100000002_f64.next_down(),
            &[1e-10, 0.01, 1e-20]
        ));
        assert_eq!(mul_up(f64::from_bits(1), 0.5), f64::from_bits(1));
        assert_eq!(mul_up(f64::from_bits(2), 0.5), f64::from_bits(1));
        assert_eq!(mul_up(f64::from_bits(3), 0.5), f64::from_bits(2));
    }

    #[test]
    fn canonical_plate_seed_indices_must_resolve_the_actual_arrays() {
        let fixture = fixture();
        let report: QueryQualification = serde_json::from_value(
            fixture["qualificationV2"]["activation"]["generatedPlaneQuery"].clone(),
        )
        .unwrap();
        let arrays = controlled_arrays(&fixture, &report);
        let mut wrong = report.clone();
        let MeshDistance::Bounded {
            witness_triangles, ..
        } = &mut wrong.mesh_distance
        else {
            panic!("bounded shared plate report required");
        };
        assert_eq!(*witness_triangles, [2, 0]);
        *witness_triangles = [0, 0];
        assert!(
            validate_query_qualification(wrong, &report.binding, &report.limits, &arrays, None)
                .is_err()
        );
    }

    #[test]
    fn preflight_refusal_cannot_retain_source_or_slab_qualification() {
        let data = fixture();
        let mut report: QueryQualification = serde_json::from_value(
            data["qualificationV2"]["activation"]["generatedPlaneQuery"].clone(),
        )
        .unwrap();
        let positions: Vec<f64> =
            serde_json::from_value(data["directPayload"]["surfacePositionsMmF64"].clone()).unwrap();
        let tags: Vec<u32> =
            serde_json::from_value(data["directPayload"]["triangleFaceOccurrenceIndices"].clone())
                .unwrap();
        let meshes = CadCollisionMeshPair {
            left_positions_mm_f64: positions.clone(),
            right_positions_mm_f64: positions,
            left_triangle_face_occurrence_indices: tags.clone(),
            right_triangle_face_occurrence_indices: tags,
        };
        for exponent in [false, true] {
            if exponent {
                report.limits = Limits {
                    maximum_exponent: 0,
                    ..Limits::default()
                };
            } else {
                report.limits.max_triangles = 1;
            }
            report.mesh_distance = MeshDistance::Unknown {
                reasons: vec!["preflight resource cap".into()],
            };
            assert!(
                validate_query_qualification(
                    report.clone(),
                    &report.binding,
                    &report.limits,
                    &meshes,
                    None
                )
                .is_err(),
                "source/slab evidence cannot survive preflight refusal"
            );
            let mut unknown = unavailable(report.binding.clone(), report.limits.clone());
            let reasons = || vec!["preflight resource cap".into()];
            unknown.mesh_distance = MeshDistance::Unknown { reasons: reasons() };
            unknown.source_bounds =
                std::array::from_fn(|_| WorldSourceBound::Unknown { reasons: reasons() });
            unknown.contact_region = ContactRegion::Unknown { reasons: reasons() };
            unknown.containment = Containment::Unknown { reasons: reasons() };
            validate_query_qualification(unknown, &report.binding, &report.limits, &meshes, None)
                .unwrap();
        }
    }
}
