use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MeshSettings {
    pub target_deflection_mm: Option<f64>,
    pub selected_deflection_mm: f64,
    pub angular_rad: f64,
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
impl SourceLimits {
    pub(super) fn planar() -> Self {
        Self::Planar {
            max_source_bytes: 2 * 1024 * 1024,
            max_source_records: 10_000,
            max_decimal_characters: 80,
            minimum_decimal_exponent: -64,
            maximum_decimal_exponent: 64,
            minimum_exponent: -128,
            maximum_exponent: 128,
            max_final_triangles: 4096,
            max_direct_bound_mm: 1e-8,
            corner_snap_distance_squared_mm2: 1e-16,
        }
    }
    pub(super) fn generated() -> Self {
        Self::Generated {
            max_final_triangles: 10_000,
            minimum_nonzero_magnitude: 1e-100,
            maximum_magnitude: 1e12,
        }
    }
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

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(tag = "state")]
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
impl SourceQualification {
    pub fn total_bound_mm(&self) -> Option<f64> {
        match self {
            Self::Bounded { total_bound_mm, .. } => Some(*total_bound_mm),
            Self::Unknown { .. } => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
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
        let supported = Self::default();
        if self.max_triangles == 0
            || self.max_triangles > supported.max_triangles
            || self.max_node_pairs == 0
            || self.max_node_pairs > supported.max_node_pairs
            || self.max_exact_pairs == 0
            || self.max_exact_pairs > supported.max_exact_pairs
            || self.max_contact_boxes == 0
            || self.max_contact_boxes > supported.max_contact_boxes
            || self.minimum_exponent < supported.minimum_exponent
            || self.maximum_exponent > supported.maximum_exponent
            || self.minimum_exponent > self.maximum_exponent
            || !self.requested_numerical_width_mm.is_finite()
            || self.requested_numerical_width_mm <= 0.
            || self.requested_numerical_width_mm > supported.requested_numerical_width_mm
        {
            return Err("invalid or unsupported qualification limits".into());
        }
        Ok(())
    }
}

#[derive(Debug, Serialize)]
#[serde(tag = "state")]
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
#[derive(Debug, Serialize)]
#[serde(tag = "state")]
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
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContactBoxPair {
    pub left_mm: [[f64; 3]; 2],
    pub right_mm: [[f64; 3]; 2],
}
#[derive(Debug, Serialize)]
#[serde(tag = "state")]
pub enum ContactRegion {
    #[serde(rename = "unknown")]
    Unknown { reasons: Vec<String> },
    #[serde(rename = "boxes", rename_all = "camelCase")]
    Boxes {
        source_inflation_mm: [f64; 2],
        pairs: Vec<ContactBoxPair>,
    },
}
#[derive(Debug, Serialize)]
#[serde(tag = "state")]
pub enum Containment {
    #[serde(rename = "unknown")]
    Unknown { reasons: Vec<String> },
    #[serde(rename = "ruledOut", rename_all = "camelCase")]
    Slab {
        method: String,
        axis: u32,
        left_upper_mm: f64,
        right_lower_mm: f64,
    },
}

/// Numeric evidence plus the single final world view consumed by the provider.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QueryEvidence {
    pub limits: Limits,
    pub mesh_distance: MeshDistance,
    pub source_bounds: [WorldSourceBound; 2],
    pub contact_region: ContactRegion,
    pub containment: Containment,
    #[serde(skip)]
    pub(crate) world_positions: [Vec<f64>; 2],
    #[serde(skip)]
    pub(crate) counters: [usize; 2],
}
impl QueryEvidence {
    /// Completed exact triangle-pair and visited node-pair counts.
    pub fn query_counters(&self) -> [usize; 2] {
        self.counters
    }
    pub fn world_positions(&self) -> [&[f64]; 2] {
        [&self.world_positions[0], &self.world_positions[1]]
    }
}
