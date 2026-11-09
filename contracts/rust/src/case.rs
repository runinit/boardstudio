use crate::{MechanicalShape, Vec2};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CaseBody {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub features: Option<Vec<CaseFeature>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub openings: Option<Vec<CaseOpening>>,
    pub id: String,
    pub name: String,
    #[serde(rename = "boardId")]
    pub board_id: String,
    pub kind: CaseKind,
    pub thickness: f64,
    pub clearance: f64,
    #[serde(rename = "materialId", skip_serializing_if = "Option::is_none")]
    pub material_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub z: Option<f64>,
    #[serde(rename = "wallHeight", skip_serializing_if = "Option::is_none")]
    pub wall_height: Option<f64>,
    #[serde(rename = "wallThickness", skip_serializing_if = "Option::is_none")]
    pub wall_thickness: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mounts: Option<Vec<Mount>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pcb_source: Option<CasePcbSource>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gasket: Option<Gasket>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CasePcbSource {
    pub contours: Vec<CasePcbSourceContour>,
    pub slots: Vec<CasePcbSlot>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CasePcbSourceContour {
    pub id: String,
    pub outer_id: Option<String>,
    pub primitives: Vec<CasePcbSourcePrimitive>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CasePcbSourcePrimitive {
    pub source_id: String,
    pub segment_index: u32,
    pub geometry: MechanicalShape,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub curve_support: Option<CasePcbCircularSupport>,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CasePcbCircularSupport {
    pub center: Vec2,
    pub radius_mm: f64,
    pub start_direction: Vec2,
    pub sweep_radians: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CasePcbSlot {
    pub id: String,
    pub geometry: MechanicalShape,
}

/// Exact additions and seats, evaluated after shell cavities and before access openings.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum CaseFeature {
    SupportPrism {
        id: String,
        points: Vec<Vec2>,
        z: f64,
        height: f64,
    },
    RoundSeat {
        id: String,
        at: Vec2,
        z: f64,
        height: f64,
        diameter: f64,
    },
    ConicalSeat {
        id: String,
        at: Vec2,
        z: f64,
        height: f64,
        diameter: f64,
        #[serde(rename = "endDiameter")]
        end_diameter: f64,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Mount {
    pub id: String,
    pub at: Vec2,
    pub kind: MountKind,
    #[serde(rename = "holeDiameter")]
    pub hole_diameter: f64,
    #[serde(rename = "bossDiameter", skip_serializing_if = "Option::is_none")]
    pub boss_diameter: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub height: Option<f64>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MountKind {
    Hole,
    Boss,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Gasket {
    pub inset: f64,
    pub width: f64,
    pub depth: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CaseKind {
    Plate,
    Tray,
    Lid,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Contour {
    pub points: Vec<Vec2>,
    pub hole: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CaseIR {
    pub revision: u64,
    pub body: CaseBody,
    pub contours: Vec<Contour>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CaseAssemblyIR {
    pub revision: u64,
    pub bodies: Vec<CaseIR>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PreparedCaseIR {
    pub revision: u64,
    pub body: CaseBody,
    pub regions: Vec<PreparedCaseRegion>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PreparedCaseRegion {
    pub outer: Vec<Vec2>,
    pub holes: Vec<Vec<Vec2>>,
    pub cavities: Vec<Vec<Vec2>>,
    pub gaskets: Vec<PreparedGasketRegion>,
    pub mounts: Vec<Mount>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PreparedGasketRegion {
    pub outer: Vec<Vec2>,
    pub holes: Vec<Vec<Vec2>>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PreparedCaseAssemblyIR {
    pub revision: u64,
    pub bodies: Vec<PreparedCaseIR>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CaseOpening {
    pub points: Vec<Vec2>,
    pub z: f64,
    pub height: f64,
}
