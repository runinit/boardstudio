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
