//! Wire types shared by the document engine and exact CAD implementation.
//! No geometry, worker, or platform dependencies belong in this crate.
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
#[cfg_attr(feature = "export-types", ts(optional_fields))]
pub struct Vec2 {
    pub x: f64,
    pub y: f64,
}
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
#[cfg_attr(feature = "export-types", ts(optional_fields))]
pub struct Pose2 {
    pub at: Vec2,
    pub rotation: f64,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
#[serde(rename_all = "lowercase")]
pub enum Side {
    Front,
    Back,
}
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
#[serde(rename_all = "kebab-case")]
pub enum KeycapProfile {
    Cherry,
    Oem,
    Dcs,
    Dsa,
    Sa,
    HiPro,
    G20,
    Choc,
}
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
#[serde(rename_all = "kebab-case")]
pub enum KeycapMount {
    Mx,
    ChocV1,
    ChocV2,
    Alps,
}
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", default)]
pub struct KeycapConfiguration {
    pub boards: BTreeMap<String, KeycapBoardSettings>,
    pub matrices: BTreeMap<String, KeycapMatrixSettings>,
    pub keys: BTreeMap<String, KeycapKeySettings>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", default)]
pub struct KeycapBoardSettings {
    pub color: String,
    pub legend_color: String,
    pub clearance: f64,
}
impl Default for KeycapBoardSettings {
    fn default() -> Self {
        Self {
            color: "#e8e4dc".into(),
            legend_color: "#202630".into(),
            clearance: 0.5,
        }
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", default)]
pub struct KeycapMatrixSettings {
    pub profile: Option<KeycapProfile>,
    pub mount: Option<KeycapMount>,
    pub first_row: u8,
    pub wall_thickness: f64,
}
impl Default for KeycapMatrixSettings {
    fn default() -> Self {
        Self {
            profile: None,
            mount: None,
            first_row: 1,
            wall_thickness: 1.2,
        }
    }
}
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", default)]
pub struct KeycapKeySettings {
    pub mount: Option<KeycapMount>,
    pub legend: Option<String>,
    pub color: Option<String>,
    pub row: Option<u8>,
    pub units: Option<Vec2>,
    pub profile: Option<KeycapProfile>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct KeycapSpec {
    pub id: String,
    pub reference: String,
    pub profile: KeycapProfile,
    pub mount: KeycapMount,
    pub row: u8,
    pub size: Vec2,
    pub top_size: Vec2,
    pub height: f64,
    pub tilt: f64,
    pub dish_depth: f64,
    pub spherical: bool,
    pub wall_thickness: f64,
    pub pose: Pose2,
    pub side: Side,
    pub z: f64,
    pub travel: f64,
    pub legend: String,
    pub color: String,
    pub legend_color: String,
}
