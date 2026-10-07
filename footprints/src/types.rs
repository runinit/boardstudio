//! Wire types for the values the provider reads and returns. They match the
//! serialized form of the document types in `boardstudio_core::model` so that
//! Core can convert (or, at cutover, re-export them) without a translation
//! layer. `core` depends on this crate, so the types live here.
use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

pub use boardstudio_contracts::{Pose2, Side, Vec2};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PadShape {
    Circle,
    Oval,
    Rect,
    Roundrect,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Pad {
    pub id: String,
    pub number: String,
    pub at: Vec2,
    pub size: Vec2,
    pub shape: PadShape,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub drill: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plated: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub side: Option<Side>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rotation: Option<f64>,
    #[serde(rename = "netId", default, skip_serializing_if = "Option::is_none")]
    pub net_id: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Vec3 {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ModelBinding {
    #[serde(rename = "assetId")]
    pub asset_id: String,
    pub offset: Vec3,
    pub rotation: Vec3,
    pub scale: Vec3,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PartKind {
    Switch,
    Controller,
    Connector,
    Encoder,
    Passive,
    Custom,
    Utility,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MatrixTerminals {
    pub row: String,
    pub column: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EnvelopeOrigin {
    Generated,
    Authored,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct EnvelopeSource {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub courtyard: Option<EnvelopeOrigin>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub keycap: Option<EnvelopeOrigin>,
}

/// The `generator` field of a definition.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GeneratorRef {
    pub source: String,
    pub version: String,
    #[serde(default)]
    pub parameters: BTreeMap<String, Value>,
}

/// The fields of a placed part that rendering reads.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PartRef {
    pub id: String,
    pub reference: String,
    pub pose: Pose2,
    pub side: Side,
    #[serde(
        rename = "generatorParameters",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub generator_parameters: Option<BTreeMap<String, Value>>,
}

/// A net name and the index KiCad will see.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ReservedNet {
    pub name: String,
    pub index: u32,
}
