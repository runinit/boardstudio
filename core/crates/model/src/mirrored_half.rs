/// Imported MX stabilisers: ADR-0010 keeps their world heading in every companion mode. Their
/// position takes the X reflection, and they never raise an uncertain-heading finding.
pub const SYMMETRIC_DEFINITIONS: [&str; 2] = [
    "kicad:marbastlib/STAB_MX_2u",
    "kicad:marbastlib/STAB_MX_6.25u",
];

use crate::model::{Part, Pose2};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MirroredPair {
    pub id: String,
    pub source: MirrorDestination,
    pub destination: MirrorDestination,
    pub mapping: Pose2,
    pub overrides: Vec<PartOverride>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "id", rename_all = "camelCase")]
pub enum MirrorDestination {
    SecondBoard(String),
    SameBoardLayout(String),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PartOverride {
    pub source_id: String,
    pub state: MirrorOverride,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "payload", rename_all = "camelCase")]
pub enum MirrorOverride {
    Local(Box<Part>),
    Omitted,
    CompanionPose(Pose2),
}

pub fn mirrored_matrix_id_prefix(pair_id: &str) -> String {
    use std::fmt::Write;
    let mut prefix = String::with_capacity(8 + pair_id.len() * 2);
    prefix.push_str("mirror_");
    for byte in pair_id.bytes() {
        let _ = write!(prefix, "{byte:02x}");
    }
    prefix.push('_');
    prefix
}

pub fn mirrored_matrix_id(pair_id: &str, source_id: &str) -> String {
    let mut id = mirrored_matrix_id_prefix(pair_id);
    id.push_str(source_id);
    id
}
