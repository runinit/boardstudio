pub use crate::keymap::{
    EncoderBinding, EncoderDirection, KeyBinding, KeymapChange, KeymapConfiguration, KeymapLayer,
    KeymapMacro, MacroChange, MacroStep,
};
pub use boardstudio_contracts::{
    KeycapBoardSettings, KeycapConfiguration, KeycapKeySettings, KeycapMatrixSettings, KeycapMount,
    KeycapProfile, KeycapSpec, Pose2, Side, Vec2,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct ArchiveEntry {
    pub path: String,
    pub buffer_index: u32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct ArchiveAssetBuffer {
    pub sha256: String,
    pub buffer_index: u32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum ArchiveRequest {
    PackProject {
        #[serde(rename = "projectJson")]
        project_json: String,
        #[serde(
            rename = "archiveJson",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        #[cfg_attr(feature = "export-types", ts(optional))]
        archive_json: Option<String>,
        assets: Vec<ArchiveEntry>,
    },
    UnpackProject,
    PackFiles {
        entries: Vec<ArchiveEntry>,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum ArchiveReply {
    Packed,
    Unpacked {
        #[serde(rename = "projectJson")]
        project_json: String,
        assets: Vec<ArchiveAssetBuffer>,
    },
    Error {
        message: String,
    },
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
#[cfg_attr(feature = "export-types", ts(optional_fields))]
pub struct Vec3 {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
#[cfg_attr(feature = "export-types", ts(optional_fields))]
pub struct Material {
    pub id: String,
    pub name: String,
    pub thickness: f64,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
#[cfg_attr(feature = "export-types", ts(optional_fields))]
pub struct Pad {
    pub id: String,
    pub number: String,
    pub at: Vec2,
    pub size: Vec2,
    pub shape: PadShape,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub drill: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plated: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub side: Option<Side>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rotation: Option<f64>,
    #[serde(rename = "netId", skip_serializing_if = "Option::is_none")]
    pub net_id: Option<String>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
#[serde(rename_all = "lowercase")]
pub enum PadShape {
    Circle,
    Oval,
    Rect,
    Roundrect,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
#[cfg_attr(feature = "export-types", ts(optional_fields))]
pub struct PartModel {
    #[serde(rename = "assetId")]
    pub asset_id: String,
    pub offset: Vec3,
    pub rotation: Vec3,
    pub scale: Vec3,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct KicadSource {
    #[cfg_attr(feature = "export-types", ts(type = "1"))]
    pub format_version: u8,
    pub source: String,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
#[cfg_attr(feature = "export-types", ts(optional_fields))]
#[serde(deny_unknown_fields)]
pub struct PartDefinition {
    pub id: String,
    pub name: String,
    pub kind: PartKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub keycap: Option<Vec2>,
    #[serde(
        rename = "envelopeSource",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub envelope_source: Option<EnvelopeSource>,
    #[serde(
        rename = "kicadSource",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub kicad_source: Option<KicadSource>,
    #[cfg_attr(
        feature = "export-types",
        ts(as = "Option<BTreeMap<String, Vec<String>>>", optional)
    )]
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub terminals: BTreeMap<String, Vec<String>>,
    #[serde(
        rename = "matrixTerminals",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub matrix_terminals: Option<MatrixTerminals>,
    #[serde(
        rename = "envelopeNotice",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub envelope_notice: Option<String>,
    pub courtyard: Vec<Vec2>,
    pub pads: Vec<Pad>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub models: Option<Vec<PartModel>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub generator: Option<PartGenerator>,
    #[serde(
        rename = "mechanicalProfile",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub mechanical_profile: Option<MechanicalPartProfile>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", default)]
pub struct EnvelopeSource {
    #[cfg_attr(feature = "export-types", ts(optional = nullable))]
    pub courtyard: Option<EnvelopeOrigin>,
    #[cfg_attr(feature = "export-types", ts(optional = nullable))]
    pub keycap: Option<EnvelopeOrigin>,
}
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
#[serde(rename_all = "lowercase")]
pub enum EnvelopeOrigin {
    Generated,
    Authored,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
#[cfg_attr(feature = "export-types", ts(optional_fields))]
pub struct MatrixTerminals {
    pub row: String,
    pub column: String,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
#[cfg_attr(feature = "export-types", ts(optional_fields))]
pub struct PartGenerator {
    pub source: String,
    pub version: String,
    pub parameters: BTreeMap<String, serde_json::Value>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
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
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
#[cfg_attr(feature = "export-types", ts(optional_fields))]
pub struct Part {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub keycap: Option<Vec2>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub outline: Option<PartOutline>,
    pub id: String,
    #[serde(rename = "definitionId")]
    pub definition_id: String,
    pub reference: String,
    pub pose: Pose2,
    pub side: Side,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub locked: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub properties: Option<BTreeMap<String, serde_json::Value>>,
    #[serde(
        rename = "generatorParameters",
        skip_serializing_if = "Option::is_none"
    )]
    pub generator_parameters: Option<BTreeMap<String, serde_json::Value>>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
#[cfg_attr(feature = "export-types", ts(optional_fields))]
pub struct Pin {
    #[serde(rename = "partId")]
    pub part_id: String,
    #[serde(rename = "padId")]
    pub pad_id: String,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
#[cfg_attr(feature = "export-types", ts(optional_fields))]
pub struct Net {
    pub id: String,
    pub name: String,
    pub pins: Vec<Pin>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
#[cfg_attr(feature = "export-types", ts(optional_fields))]
#[serde(deny_unknown_fields)]
pub struct Matrix {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    pub rows: u32,
    pub columns: u32,
    pub pitch: Vec2,
    pub origin: Vec2,
    #[serde(rename = "definitionId")]
    pub definition_id: String,
    #[serde(rename = "partIds")]
    pub part_ids: Vec<String>,
    #[serde(rename = "boardId", skip_serializing_if = "Option::is_none")]
    pub board_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mirror: Option<Mirror>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rotation: Option<f64>,
    #[serde(rename = "edgeGap", skip_serializing_if = "Option::is_none")]
    pub edge_gap: Option<Vec2>,
    #[serde(rename = "diodeDirection", skip_serializing_if = "Option::is_none")]
    pub diode_direction: Option<DiodeDirection>,
    #[cfg_attr(feature = "export-types", ts(as = "Option<Vec<Vec2>>", optional))]
    #[serde(rename = "rowOffsets", default, skip_serializing_if = "Vec::is_empty")]
    pub row_offsets: Vec<Vec2>,
    #[cfg_attr(feature = "export-types", ts(as = "Option<Vec<Vec2>>", optional))]
    #[serde(
        rename = "columnOffsets",
        default,
        skip_serializing_if = "Vec::is_empty"
    )]
    pub column_offsets: Vec<Vec2>,
    #[cfg_attr(feature = "export-types", ts(as = "Option<Vec<f64>>", optional))]
    #[serde(
        rename = "columnStaggers",
        default,
        skip_serializing_if = "Vec::is_empty"
    )]
    pub column_staggers: Vec<f64>,
    #[cfg_attr(feature = "export-types", ts(as = "Option<Vec<f64>>", optional))]
    #[serde(
        rename = "columnSplays",
        default,
        skip_serializing_if = "Vec::is_empty"
    )]
    pub column_splays: Vec<f64>,
    #[cfg_attr(
        feature = "export-types",
        ts(as = "Option<Vec<Option<Vec2>>>", optional)
    )]
    #[serde(
        rename = "columnOrigins",
        default,
        skip_serializing_if = "Vec::is_empty"
    )]
    pub column_origins: Vec<Option<Vec2>>,
    #[cfg_attr(feature = "export-types", ts(as = "Option<Vec<MatrixCell>>", optional))]
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub cells: Vec<MatrixCell>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
#[cfg_attr(feature = "export-types", ts(optional_fields))]
#[serde(deny_unknown_fields)]
pub struct MatrixCell {
    pub row: u32,
    pub column: u32,
    pub enabled: bool,
    #[serde(rename = "definitionId", skip_serializing_if = "Option::is_none")]
    pub definition_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub variant: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub offset: Option<Vec2>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rotation: Option<f64>,
    #[cfg_attr(
        feature = "export-types",
        ts(as = "Option<Vec<MatrixAssembly>>", optional)
    )]
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub assemblies: Vec<MatrixAssembly>,
    #[serde(
        rename = "assembliesLocal",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub assemblies_local: Option<bool>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
#[cfg_attr(feature = "export-types", ts(optional_fields))]
pub struct MatrixAssembly {
    pub id: String,
    #[serde(rename = "definitionId")]
    pub definition_id: String,
    pub offset: Vec2,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rotation: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub side: Option<Side>,
}
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
#[serde(rename_all = "lowercase")]
pub enum Mirror {
    None,
    X,
    Y,
}
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
#[serde(rename_all = "lowercase")]
pub enum DiodeDirection {
    Row2col,
    Col2row,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", default)]
pub struct PartOutline {
    #[cfg_attr(feature = "export-types", ts(as = "Option<bool>", optional))]
    pub excluded: bool,
    #[cfg_attr(feature = "export-types", ts(as = "Option<f64>", optional))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub margin: Option<f64>,
    #[cfg_attr(feature = "export-types", ts(as = "Option<bool>", optional))]
    #[serde(skip_serializing_if = "is_false")]
    pub allow_body_overhang: bool,
}
fn is_false(value: &bool) -> bool {
    !value
}
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, Default)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
#[serde(rename_all = "lowercase")]
pub enum CornerStyle {
    #[default]
    Sharp,
    Fillet,
    Chamfer,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", default)]
pub struct OutlineSettings {
    pub corners: CornerStyle,
    pub size: f64,
    pub bridge_width: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "export-types", ts(optional))]
    pub repair: Option<OutlineRepairSettings>,
}
impl Default for OutlineSettings {
    fn default() -> Self {
        Self {
            corners: CornerStyle::Sharp,
            size: 2.0,
            bridge_width: 10.0,
            repair: None,
        }
    }
}
/// Generated cleanup is enabled for legacy documents as well as new outlines.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", default)]
pub struct OutlineRepairSettings {
    pub enabled: bool,
    pub maximum_gap_span: f64,
    pub minimum_connection_width: f64,
    pub edge_clearance: f64,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(
        feature = "export-types",
        ts(as = "Option<Vec<ProtectedOutlineGap>>", optional)
    )]
    pub keep_gaps: Vec<ProtectedOutlineGap>,
}
impl Default for OutlineRepairSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            maximum_gap_span: 20.0,
            minimum_connection_width: 2.0,
            edge_clearance: 0.0,
            keep_gaps: vec![],
        }
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
pub struct ProtectedOutlineGap {
    pub id: String,
    pub points: Vec<OutlineControlPoint>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
#[cfg_attr(feature = "export-types", ts(optional_fields))]
#[serde(rename_all = "camelCase")]
pub struct OutlineControlPoint {
    pub at: Vec2,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub part_id: Option<String>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
pub struct OutlineConnection {
    pub id: String,
    pub width: f64,
    pub points: Vec<OutlineControlPoint>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum OutlineFeature {
    Polygon {
        #[serde(
            default,
            rename = "anchorPartId",
            skip_serializing_if = "Option::is_none"
        )]
        #[cfg_attr(feature = "export-types", ts(optional))]
        anchor_part_id: Option<String>,
        id: String,
        points: Vec<Vec2>,
        operation: Operation,
    },
    Rect {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "export-types", ts(optional))]
        rotation: Option<f64>,
        #[serde(
            default,
            rename = "anchorPartId",
            skip_serializing_if = "Option::is_none"
        )]
        #[cfg_attr(feature = "export-types", ts(optional))]
        anchor_part_id: Option<String>,
        id: String,
        center: Vec2,
        size: Vec2,
        radius: f64,
        operation: Operation,
    },
    PartEnvelope {
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        #[cfg_attr(
            feature = "export-types",
            ts(as = "Option<Vec<OutlineConnection>>", optional)
        )]
        connections: Vec<OutlineConnection>,
        #[cfg_attr(feature = "export-types", ts(as = "Option<OutlineSettings>", optional))]
        #[serde(default)]
        settings: OutlineSettings,
        id: String,
        #[serde(rename = "partIds")]
        part_ids: Vec<String>,
        margin: f64,
        operation: Operation,
    },
}
impl OutlineFeature {
    pub fn id(&self) -> &str {
        match self {
            Self::Polygon { id, .. } | Self::Rect { id, .. } | Self::PartEnvelope { id, .. } => id,
        }
    }
    pub fn operation(&self) -> Operation {
        match self {
            Self::Polygon { operation, .. }
            | Self::Rect { operation, .. }
            | Self::PartEnvelope { operation, .. } => *operation,
        }
    }
}
/// One geometry owner per physical board. None selects the permanent Generated version.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct BoardOutline {
    pub board_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "export-types", ts(optional))]
    pub active_version_id: Option<String>,
    pub versions: Vec<OutlineVersion>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "export-types", ts(optional))]
    pub generated_last_valid: Option<OutlineSnapshot>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct OutlineProvenance {
    pub revision: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "export-types", ts(optional))]
    pub version_id: Option<String>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
pub struct OutlineVersion {
    pub id: String,
    pub name: String,
    pub source: OutlineProvenance,
    pub geometry: OutlineSnapshot,
}
/// Fixed world geometry, before corner finishing; source IDs are provenance only.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct OutlineSnapshot {
    pub features: Vec<OutlineFeature>,
    pub settings: OutlineSettings,
    pub expected_regions: u32,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(
        feature = "export-types",
        ts(as = "Option<Vec<OutlineBridge>>", optional)
    )]
    pub bridges: Vec<OutlineBridge>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(
        feature = "export-types",
        ts(as = "Option<Vec<ProtectedOutlineGap>>", optional)
    )]
    pub protected_gaps: Vec<ProtectedOutlineGap>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct OutlineBridge {
    pub id: String,
    pub width: f64,
    pub points: Vec<Vec2>,
    pub part_ids: Vec<String>,
    pub matrix_ids: Vec<String>,
    pub authored: bool,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct OutlineGap {
    pub id: String,
    pub feature_id: String,
    pub span: f64,
    pub points: Vec<OutlineControlPoint>,
    pub protected: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "export-types", ts(as = "Option<Vec<String>>", optional))]
    pub protected_ids: Vec<String>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct BoardOutlineScene {
    pub board_id: String,
    pub source_contours: Vec<Contour>,
    pub bridges: Vec<OutlineBridge>,
    pub gaps: Vec<OutlineGap>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct FindingMarker {
    pub finding_id: String,
    pub board_id: String,
    pub contours: Vec<Contour>,
}
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
#[serde(rename_all = "lowercase")]
pub enum Operation {
    Add,
    Subtract,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
#[cfg_attr(feature = "export-types", ts(optional_fields))]
pub struct Board {
    pub id: String,
    pub name: String,
    #[serde(rename = "outlineIds")]
    pub outline_ids: Vec<String>,
    #[serde(rename = "partIds")]
    pub part_ids: Vec<String>,
    #[serde(rename = "netIds")]
    pub net_ids: Vec<String>,
    pub thickness: f64,
    #[cfg_attr(
        feature = "export-types",
        ts(as = "Option<Vec<CopperTrace>>", optional)
    )]
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub traces: Vec<CopperTrace>,
    #[cfg_attr(feature = "export-types", ts(as = "Option<Vec<CopperVia>>", optional))]
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub vias: Vec<CopperVia>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
#[cfg_attr(feature = "export-types", ts(optional_fields))]
pub struct CopperTrace {
    pub id: String,
    pub start: Vec2,
    pub end: Vec2,
    pub width: f64,
    pub layer: Side,
    #[serde(rename = "netId", skip_serializing_if = "Option::is_none")]
    pub net_id: Option<String>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
#[cfg_attr(feature = "export-types", ts(optional_fields))]
pub struct CopperVia {
    pub id: String,
    pub at: Vec2,
    pub size: f64,
    pub drill: f64,
    #[serde(rename = "netId", skip_serializing_if = "Option::is_none")]
    pub net_id: Option<String>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
#[cfg_attr(feature = "export-types", ts(optional_fields))]
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gasket: Option<Gasket>,
}
/// Exact additions and seats, evaluated after shell cavities and before access openings.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
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
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
#[cfg_attr(feature = "export-types", ts(optional_fields))]
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
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
#[serde(rename_all = "lowercase")]
pub enum MountKind {
    Hole,
    Boss,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
#[cfg_attr(feature = "export-types", ts(optional_fields))]
pub struct Gasket {
    pub inset: f64,
    pub width: f64,
    pub depth: f64,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
#[serde(rename_all = "lowercase")]
pub enum CaseKind {
    Plate,
    Tray,
    Lid,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
#[cfg_attr(feature = "export-types", ts(optional_fields))]
pub struct Asset {
    pub id: String,
    pub name: String,
    #[serde(rename = "mediaType")]
    pub media_type: String,
    pub sha256: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub license: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
#[cfg_attr(feature = "export-types", ts(optional_fields))]
pub struct Script {
    pub id: String,
    pub name: String,
    pub source: String,
    pub enabled: bool,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum Constraint {
    Offset {
        id: String,
        #[serde(rename = "sourcePartId")]
        source_part_id: String,
        #[serde(rename = "targetPartId")]
        target_part_id: String,
        offset: Vec2,
        rotation: f64,
    },
    Mirror {
        id: String,
        #[serde(rename = "sourcePartId")]
        source_part_id: String,
        #[serde(rename = "targetPartId")]
        target_part_id: String,
        axis: MirrorAxis,
        coordinate: f64,
    },
}
impl Constraint {
    pub fn id(&self) -> &str {
        match self {
            Self::Offset { id, .. } | Self::Mirror { id, .. } => id,
        }
    }
    pub fn source(&self) -> &str {
        match self {
            Self::Offset { source_part_id, .. } | Self::Mirror { source_part_id, .. } => {
                source_part_id
            }
        }
    }
    pub fn target(&self) -> &str {
        match self {
            Self::Offset { target_part_id, .. } | Self::Mirror { target_part_id, .. } => {
                target_part_id
            }
        }
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
#[serde(rename_all = "lowercase")]
pub enum MirrorAxis {
    Vertical,
    Horizontal,
}
/// A named key layout and its independent, board-owned components.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
#[cfg_attr(feature = "export-types", ts(optional_fields))]
#[serde(rename_all = "camelCase")]
pub struct Layout {
    pub id: String,
    pub name: String,
    pub board_id: String,
    pub matrix_id: String,
    pub part_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mirror_link: Option<LayoutMirrorLink>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct LayoutMirrorLink {
    pub source_id: String,
    pub axis_x: f64,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
#[cfg_attr(feature = "export-types", ts(optional_fields))]
pub struct ProjectDoc {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub keymap: Option<KeymapConfiguration>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub keycaps: Option<KeycapConfiguration>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hardware: Option<HardwareConfiguration>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mechanical: Option<MechanicalConfiguration>,
    #[serde(
        rename = "boardReferences",
        default,
        skip_serializing_if = "Vec::is_empty"
    )]
    #[cfg_attr(
        feature = "export-types",
        ts(as = "Option<Vec<BoardReference>>", optional)
    )]
    pub board_references: Vec<BoardReference>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(
        feature = "export-types",
        ts(as = "Option<Vec<AssemblyDefinition>>", optional)
    )]
    pub assemblies: Vec<AssemblyDefinition>,
    #[cfg_attr(feature = "export-types", ts(type = "\"boardstudio/v2\""))]
    pub format: String,
    pub id: String,
    pub name: String,
    pub revision: u64,
    pub parameters: BTreeMap<String, serde_json::Value>,
    pub definitions: Vec<PartDefinition>,
    pub parts: Vec<Part>,
    pub matrices: Vec<Matrix>,
    #[cfg_attr(feature = "export-types", ts(as = "Option<Vec<Layout>>", optional))]
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub layouts: Vec<Layout>,
    pub nets: Vec<Net>,
    pub outline: Vec<OutlineFeature>,
    #[serde(
        rename = "boardOutlines",
        default,
        skip_serializing_if = "Vec::is_empty"
    )]
    #[cfg_attr(
        feature = "export-types",
        ts(as = "Option<Vec<BoardOutline>>", optional)
    )]
    pub board_outlines: Vec<BoardOutline>,
    pub boards: Vec<Board>,
    #[serde(rename = "caseBodies")]
    pub case_bodies: Vec<CaseBody>,
    pub materials: Vec<Material>,
    pub assets: Vec<Asset>,
    pub scripts: Vec<Script>,
    #[serde(default)]
    pub constraints: Vec<Constraint>,
}
impl ProjectDoc {
    pub fn empty(id: &str, name: &str) -> Self {
        Self {
            keymap: None,
            keycaps: None,
            hardware: None,
            mechanical: None,
            board_references: vec![],
            assemblies: vec![],
            format: "boardstudio/v2".into(),
            id: id.into(),
            name: name.into(),
            revision: 0,
            parameters: BTreeMap::new(),
            definitions: vec![],
            parts: vec![],
            matrices: vec![],
            layouts: vec![],
            nets: vec![],
            outline: vec![],
            board_outlines: vec![],
            boards: vec![],
            case_bodies: vec![],
            materials: vec![],
            assets: vec![],
            scripts: vec![],
            constraints: vec![],
        }
    }
}

/// Project-level physical/electrical topology for automatic wiring.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", default)]
pub struct HardwareConfiguration {
    pub topology: HardwareTopology,
    pub transport: HardwareTransport,
    pub instances: Vec<PhysicalBoardInstance>,
    pub boards: Vec<ElectricalBoardConfiguration>,
    pub shared_construction: Option<MechanicalConfiguration>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
#[serde(rename_all = "kebab-case")]
pub enum HardwareTopology {
    #[default]
    Unibody,
    Split,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
#[serde(rename_all = "kebab-case")]
pub enum HardwareTransport {
    #[default]
    None,
    Wireless,
    Wired,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct PhysicalBoardInstance {
    pub id: String,
    pub name: String,
    pub board_id: String,
    pub half: String,
    pub role: String,
    pub flipped: bool,
    pub controller_part_id: Option<String>,
    pub mechanical: Option<MechanicalConfiguration>,
    pub construction_linked: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", default)]
pub struct ElectricalBoardConfiguration {
    pub board_id: String,
    pub controller_part_id: Option<String>,
    pub mode: crate::electrical::ElectricalMode,
    pub locks: BTreeMap<String, String>,
    pub assignments: BTreeMap<String, String>,
    pub key_bindings: BTreeMap<String, String>,
    pub jumper_states: BTreeMap<String, crate::electrical_profiles::JumperState>,
    pub protected_handoff: Option<ElectricalHandoffBaseline>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct ElectricalHandoffBaseline {
    pub fingerprint: String,
    pub revision: u64,
    pub assignments: BTreeMap<String, String>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
#[serde(rename_all = "kebab-case")]
pub enum MatrixSplayAffect {
    Column,
    Following,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum MatrixSplayChange {
    Origin {
        world: Option<Vec2>,
    },
    Angle {
        angle: f64,
        affect: MatrixSplayAffect,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum EditOperation {
    EditKeymap {
        change: KeymapChange,
    },
    SetKeyBinding {
        #[serde(rename = "boardId")]
        board_id: String,
        #[serde(rename = "keyId")]
        key_id: String,
        binding: String,
    },
    SetKeycapBoard {
        #[serde(rename = "boardId")]
        board_id: String,
        change: KeycapBoardChange,
    },
    SetMatrixKeycaps {
        #[serde(rename = "matrixId")]
        matrix_id: String,
        change: KeycapMatrixChange,
    },
    SetKeycapKey {
        #[serde(rename = "keyId")]
        key_id: String,
        change: KeycapKeyChange,
    },
    SetMechanical {
        configuration: Option<MechanicalConfiguration>,
    },
    MoveParts {
        positions: Vec<Position>,
    },
    SetOutline {
        feature: OutlineFeature,
    },
    CopyOutline {
        #[serde(rename = "boardId")]
        board_id: String,
        #[serde(rename = "versionId")]
        version_id: String,
        name: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "export-types", ts(optional))]
        edit: Option<OutlineContourEdit>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "export-types", ts(optional))]
        feature: Option<OutlineFeature>,
    },
    SelectOutline {
        #[serde(rename = "boardId")]
        board_id: String,
        #[serde(rename = "versionId", default)]
        version_id: Option<String>,
    },
    RenameOutline {
        #[serde(rename = "boardId")]
        board_id: String,
        #[serde(rename = "versionId")]
        version_id: String,
        name: String,
    },
    RemoveOutline {
        #[serde(rename = "boardId")]
        board_id: String,
        #[serde(rename = "versionId")]
        version_id: String,
    },
    AddPart {
        part: Part,
        #[cfg_attr(feature = "export-types", ts(optional))]
        #[serde(rename = "boardId", skip_serializing_if = "Option::is_none")]
        board_id: Option<String>,
    },
    RemoveMatrix {
        id: String,
    },
    RemoveParts {
        ids: Vec<String>,
    },
    SetNet {
        net: Net,
    },
    SetCase {
        body: CaseBody,
    },
    SetMatrix {
        matrix: Matrix,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        definitions: Option<Vec<PartDefinition>>,
    },
    SetMatrixSplay {
        #[serde(rename = "matrixId")]
        matrix_id: String,
        column: u32,
        change: MatrixSplayChange,
    },
    SetConstraint {
        constraint: Constraint,
    },
    CreateMirroredPair {
        left: Layout,
        right: Layout,
        matrix: Matrix,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        definitions: Option<Vec<PartDefinition>>,
    },
    SetLayout {
        layout: Layout,
    },
    RemoveConstraint {
        id: String,
    },
    ReplaceDocument {
        document: ProjectDoc,
    },
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
pub struct OutlineContourEdit {
    pub contour: u32,
    pub points: Vec<Vec2>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
#[cfg_attr(feature = "export-types", ts(optional_fields))]
pub struct Position {
    pub id: String,
    pub at: Vec2,
}
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
#[serde(rename_all = "lowercase")]
pub enum EditPhase {
    Preview,
    Commit,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
#[cfg_attr(feature = "export-types", ts(optional_fields))]
pub struct EditCommand {
    #[serde(rename = "baseRevision")]
    pub base_revision: u64,
    #[serde(rename = "transactionId")]
    pub transaction_id: String,
    pub phase: EditPhase,
    #[serde(rename = "targetIds")]
    pub target_ids: Vec<String>,
    pub operation: EditOperation,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
#[cfg_attr(feature = "export-types", ts(optional_fields))]
pub struct Contour {
    pub points: Vec<Vec2>,
    pub hole: bool,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
pub struct CaseIR {
    pub revision: u64,
    pub body: CaseBody,
    pub contours: Vec<Contour>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
pub struct CaseAssemblyIR {
    pub revision: u64,
    pub bodies: Vec<CaseIR>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
pub struct PreparedCaseIR {
    pub revision: u64,
    pub body: CaseBody,
    pub regions: Vec<PreparedCaseRegion>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
pub struct PreparedCaseRegion {
    pub outer: Vec<Vec2>,
    pub holes: Vec<Vec<Vec2>>,
    pub cavities: Vec<Vec<Vec2>>,
    pub gaskets: Vec<PreparedGasketRegion>,
    pub mounts: Vec<Mount>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
pub struct PreparedGasketRegion {
    pub outer: Vec<Vec2>,
    pub holes: Vec<Vec<Vec2>>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
pub struct PreparedCaseAssemblyIR {
    pub revision: u64,
    pub bodies: Vec<PreparedCaseIR>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Error,
    Warning,
    Info,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
#[serde(rename_all = "lowercase")]
pub enum Scope {
    Layout,
    Outline,
    Pcb,
    Case,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
#[cfg_attr(feature = "export-types", ts(optional_fields))]
pub struct Finding {
    pub id: String,
    pub severity: Severity,
    pub scope: Scope,
    pub message: String,
    #[serde(rename = "targetIds")]
    pub target_ids: Vec<String>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
#[cfg_attr(feature = "export-types", ts(optional_fields))]
pub struct Readiness {
    pub layout: bool,
    pub outline: bool,
    pub pcb: bool,
    #[serde(rename = "case")]
    pub case_ready: bool,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
#[cfg_attr(feature = "export-types", ts(optional_fields))]
pub struct Transform {
    pub id: String,
    pub pose: Pose2,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
#[cfg_attr(feature = "export-types", ts(optional_fields))]
pub struct BoardContours {
    #[serde(rename = "boardId")]
    pub board_id: String,
    pub contours: Vec<Contour>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
#[cfg_attr(feature = "export-types", ts(optional_fields))]
pub struct BoardReadiness {
    #[serde(rename = "boardId")]
    pub board_id: String,
    pub outline: bool,
    pub pcb: bool,
    #[serde(rename = "case")]
    pub case_ready: bool,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
#[cfg_attr(feature = "export-types", ts(optional_fields))]
pub struct SceneDelta {
    pub revision: u64,
    #[serde(rename = "transactionId")]
    pub transaction_id: String,
    #[serde(rename = "changedIds")]
    pub changed_ids: Vec<String>,
    pub transforms: Vec<Transform>,
    #[serde(rename = "matrixScenes")]
    pub matrix_scenes: Vec<MatrixScene>,
    pub contours: Vec<Contour>,
    #[serde(rename = "boardContours")]
    pub board_contours: Vec<BoardContours>,
    #[serde(rename = "boardReadiness")]
    pub board_readiness: Vec<BoardReadiness>,
    #[serde(
        rename = "boardOutlineScenes",
        default,
        skip_serializing_if = "Vec::is_empty"
    )]
    #[cfg_attr(
        feature = "export-types",
        ts(as = "Option<Vec<BoardOutlineScene>>", optional)
    )]
    pub board_outline_scenes: Vec<BoardOutlineScene>,
    #[serde(
        rename = "findingMarkers",
        default,
        skip_serializing_if = "Vec::is_empty"
    )]
    #[cfg_attr(
        feature = "export-types",
        ts(as = "Option<Vec<FindingMarker>>", optional)
    )]
    pub finding_markers: Vec<FindingMarker>,
    pub findings: Vec<Finding>,
    pub readiness: Readiness,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
pub struct MatrixScene {
    #[serde(rename = "matrixId")]
    pub matrix_id: String,
    pub cells: Vec<MatrixSceneCell>,
    pub columns: Vec<MatrixColumnBasis>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
pub struct MatrixSceneCell {
    pub row: u32,
    pub column: u32,
    pub enabled: bool,
    #[serde(rename = "memberId", skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "export-types", ts(optional))]
    pub member_id: Option<String>,
    pub pose: Pose2,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
pub struct MatrixColumnBasis {
    pub column: u32,
    #[serde(rename = "splayOrigin")]
    pub splay_origin: Vec2,
    #[serde(rename = "splayAngle")]
    pub splay_angle: f64,
    #[serde(rename = "customOrigin")]
    pub custom_origin: bool,
    #[serde(rename = "axisX")]
    pub axis_x: Vec2,
    #[serde(rename = "axisY")]
    pub axis_y: Vec2,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum CoreRequest {
    #[serde(rename = "resolve-keycaps")]
    ResolveKeycaps {
        id: String,
        document: ProjectDoc,
        #[serde(rename = "boardId")]
        board_id: String,
        #[serde(default)]
        cases: Option<PreparedCaseAssemblyIR>,
    },
    #[serde(rename = "generate-firmware")]
    GenerateFirmware {
        id: String,
        request: crate::firmware::FirmwareRequest,
    },
    #[serde(rename = "resolve-electrical")]
    ResolveElectrical {
        id: String,
        request: crate::electrical::ElectricalPlanRequest,
    },
    #[serde(rename = "apply-electrical")]
    ApplyElectrical {
        id: String,
        #[serde(rename = "baseRevision")]
        base_revision: u64,
        plan: crate::electrical::ElectricalPlan,
        #[serde(default)]
        draft: bool,
    },
    #[serde(rename = "review-electrical-remap")]
    ReviewElectricalRemap {
        id: String,
        #[serde(rename = "baseRevision")]
        base_revision: u64,
        #[serde(rename = "boardId")]
        board_id: String,
        #[serde(rename = "expectedFingerprint")]
        expected_fingerprint: String,
    },
    #[serde(rename = "protect-electrical-handoff")]
    ProtectElectricalHandoff {
        id: String,
        #[serde(rename = "baseRevision")]
        base_revision: u64,
        #[serde(rename = "boardId")]
        board_id: String,
        plan: crate::electrical::ElectricalPlan,
    },
    #[serde(rename = "mechanical-profile")]
    MechanicalProfile {
        id: String,
        #[serde(rename = "definitionId")]
        definition_id: String,
        source: MechanicalBuiltinProfile,
        #[serde(rename = "plateToPcb")]
        plate_to_pcb: f64,
    },
    #[serde(rename = "resolve-mechanical")]
    ResolveMechanical {
        id: String,
        document: ProjectDoc,
        contours: Vec<Contour>,
    },
    Open {
        id: String,
        document: ProjectDoc,
    },
    Edit {
        id: String,
        command: EditCommand,
    },
    Undo {
        id: String,
    },
    Redo {
        id: String,
    },
    Snapshot {
        id: String,
    },
    #[serde(rename = "project-matrices")]
    ProjectMatrices {
        id: String,
        #[serde(rename = "baseRevision")]
        base_revision: u64,
        matrices: Vec<Matrix>,
    },
    #[serde(rename = "prepare-case")]
    PrepareCase {
        id: String,
        ir: CaseAssemblyIR,
    },
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum CoreReply {
    #[serde(rename = "keycaps-resolved")]
    KeycapsResolved {
        id: String,
        result: KeycapResolution,
    },
    #[serde(rename = "firmware-generated")]
    FirmwareGenerated {
        id: String,
        package: crate::firmware::FirmwarePackage,
    },
    #[serde(rename = "electrical-resolved")]
    ElectricalResolved {
        id: String,
        plan: crate::electrical::ElectricalPlan,
    },
    ElectricalApplied {
        id: String,
        revision: u64,
        document: ProjectDoc,
    },
    ElectricalHandoffProtected {
        id: String,
        revision: u64,
        document: ProjectDoc,
    },
    #[serde(rename = "mechanical-profile")]
    MechanicalProfile {
        id: String,
        profile: MechanicalPartProfile,
    },
    #[serde(rename = "mechanical-resolved")]
    MechanicalResolved {
        id: String,
        assembly: MechanicalAssembly,
    },
    Preview {
        id: String,
        scene: SceneDelta,
    },
    Scene {
        id: String,
        scene: SceneDelta,
        document: ProjectDoc,
    },
    #[serde(rename = "matrix-projections")]
    MatrixProjections {
        id: String,
        revision: u64,
        #[serde(rename = "matrixScenes")]
        matrix_scenes: Vec<MatrixScene>,
    },
    Error {
        id: String,
        message: String,
        revision: u64,
    },
    #[serde(rename = "case-prepared")]
    CasePrepared {
        id: String,
        ir: PreparedCaseAssemblyIR,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
#[serde(rename_all = "lowercase")]
pub enum OutlineExportFormat {
    Svg,
    Dxf,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct LocalTrace {
    pub id: String,
    pub start: Vec2,
    pub end: Vec2,
    pub width: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pad_id: Option<String>,
    pub layer: Side,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct LocalVia {
    pub id: String,
    pub at: Vec2,
    pub size: f64,
    pub drill: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pad_id: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct FootprintGeometry {
    pub side: Side,
    pub courtyard: Vec<Vec2>,
    pub pads: Vec<Pad>,
    pub traces: Vec<LocalTrace>,
    pub vias: Vec<LocalVia>,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
#[serde(rename_all = "kebab-case")]
pub enum ArtifactDiagnosticKind {
    Approximation,
    UnavailableModel,
    ExportUnsupported,
    ParseError,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct ArtifactDiagnostic {
    pub kind: ArtifactDiagnosticKind,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_start: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_end: Option<u64>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct CompiledFootprint {
    pub definition: PartDefinition,
    pub geometry: FootprintGeometry,
    pub diagnostics: Vec<ArtifactDiagnostic>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preview_svg: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct FootprintCompileJob {
    pub id: String,
    pub definition: PartDefinition,
    pub side: Side,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
#[serde(rename_all = "kebab-case")]
pub enum ArtifactErrorCode {
    ParseError,
    Validation,
    Unsupported,
    StaleResult,
    MismatchedResults,
    NotFound,
    Internal,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct ArtifactError {
    pub code: ArtifactErrorCode,
    pub message: String,
    pub diagnostics: Vec<ArtifactDiagnostic>,
}

impl ArtifactError {
    pub(crate) fn new(code: ArtifactErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            diagnostics: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct FootprintPatch {
    #[cfg_attr(feature = "export-types", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub footprint_name: Option<String>,
    #[cfg_attr(feature = "export-types", ts(optional))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reference: Option<String>,
    #[cfg_attr(feature = "export-types", ts(optional))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    #[cfg_attr(feature = "export-types", ts(optional))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub placement: Option<Pose2>,
    pub side: Side,
    pub pad_nets: BTreeMap<String, (u32, String)>,
    pub uuid_scope: String,
    #[cfg_attr(feature = "export-types", ts(as = "Option<Vec<String>>", optional))]
    #[serde(default)]
    pub model_forms: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct PrepareExportRequest {
    pub snapshot_token: String,
    pub expected_revision: u64,
    pub document: ProjectDoc,
    pub target: ExportTarget,
    pub contours: Vec<Contour>,
    pub model_paths: BTreeMap<String, String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
#[serde(tag = "kind", rename_all = "kebab-case")]
#[serde(rename_all_fields = "camelCase")]
pub enum ExportTarget {
    Board { board_id: String },
    StandaloneFootprints { definition_ids: Vec<String> },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct ErgogenJob {
    pub job_id: String,
    pub definition: PartDefinition,
    pub part: Part,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct ReservedNet {
    pub name: String,
    pub index: u32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct ExportPlan {
    pub snapshot_token: String,
    pub fingerprint: String,
    pub revision: u64,
    pub target: ExportTarget,
    pub jobs: Vec<ErgogenJob>,
    pub reserved_nets: Vec<ReservedNet>,
    pub next_net_index: u32,
    pub contours: Vec<Contour>,
    pub captured_document: ProjectDoc,
    pub model_paths: BTreeMap<String, String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct ErgogenJobResult {
    pub snapshot_token: String,
    pub revision: u64,
    pub job_id: String,
    pub source: String,
    pub nets: Vec<ReservedNet>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct FinishExportRequest {
    pub plan: ExportPlan,
    pub results: Vec<ErgogenJobResult>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct ArtifactFile {
    pub filename: String,
    pub content: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct ExportArtifact {
    pub snapshot_token: String,
    pub revision: u64,
    pub files: Vec<ArtifactFile>,
    #[cfg_attr(feature = "export-types", ts(as = "Option<Vec<String>>", optional))]
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub skipped_utilities: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct OutlineExportRequest {
    pub filename: String,
    pub board: Board,
    pub contours: Vec<Contour>,
    pub format: OutlineExportFormat,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
#[serde(tag = "kind", rename_all = "kebab-case")]
#[serde(rename_all_fields = "camelCase")]
pub enum ArtifactRequest {
    ExportMechanicalPlate {
        id: String,
        document: ProjectDoc,
        contours: Vec<Contour>,
    },
    ExtractMechanical {
        id: String,
        source: String,
        mappings: Vec<MechanicalPurposeMapping>,
        max_deviation_mm: f64,
    },
    PreviewBoard {
        id: String,
        source: String,
        revision: u64,
    },
    CompileFootprints {
        id: String,
        jobs: Vec<FootprintCompileJob>,
    },
    ImportFootprint {
        id: String,
        definition_id: String,
        source: String,
    },
    PrepareExport {
        id: String,
        request: PrepareExportRequest,
    },
    FinishExport {
        id: String,
        request: FinishExportRequest,
    },
    ExportOutline {
        id: String,
        request: OutlineExportRequest,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
#[serde(tag = "kind", rename_all = "kebab-case")]
#[serde(rename_all_fields = "camelCase")]
pub enum ArtifactReply {
    ExportMechanicalPlate {
        id: String,
        result: ExportArtifact,
    },
    ExtractMechanical {
        id: String,
        result: MechanicalExtraction,
    },
    PreviewBoard {
        id: String,
        result: PcbPreview,
    },
    CompileFootprints {
        id: String,
        result: Vec<CompiledFootprint>,
    },
    ImportFootprint {
        id: String,
        result: CompiledFootprint,
    },
    PrepareExport {
        id: String,
        result: ExportPlan,
    },
    FinishExport {
        id: String,
        result: ExportArtifact,
    },
    ExportOutline {
        id: String,
        result: ArtifactFile,
    },
    Error {
        id: String,
        error: ArtifactError,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
#[serde(rename_all = "kebab-case")]
pub enum MechanicalPurpose {
    ElectricalPcbMountingHole,
    PlateCutout,
    ClearanceEnvelope,
    DrawingGuide,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
#[serde(rename_all = "kebab-case")]
pub enum MechanicalGeometryKind {
    Line,
    Arc,
    Circle,
    Rectangle,
    Polygon,
    Drill,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "export-types", ts(optional_fields))]
pub struct MechanicalPurposeMapping {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<MechanicalGeometryKind>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub layer: Option<String>,
    pub purpose: MechanicalPurpose,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
#[serde(rename_all = "kebab-case")]
pub enum MechanicalDrillShape {
    Circle,
    Oval,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
#[serde(tag = "kind", rename_all = "kebab-case")]
#[serde(rename_all_fields = "camelCase")]
#[cfg_attr(feature = "export-types", ts(optional_fields))]
pub enum MechanicalShape {
    Line {
        start: Vec2,
        end: Vec2,
        width: Option<f64>,
    },
    Arc {
        start: Vec2,
        mid: Option<Vec2>,
        end: Vec2,
        angle_degrees: Option<f64>,
        width: Option<f64>,
    },
    Circle {
        center: Vec2,
        end: Vec2,
        width: Option<f64>,
    },
    Rectangle {
        start: Vec2,
        end: Vec2,
        width: Option<f64>,
    },
    Polygon {
        points: Vec<Vec2>,
        width: Option<f64>,
    },
    Drill {
        at: Vec2,
        size: Vec2,
        offset: Vec2,
        rotation_degrees: f64,
        shape: MechanicalDrillShape,
        plated: Option<bool>,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "export-types", ts(optional_fields))]
pub struct MechanicalPrimitive {
    pub id: String,
    pub source_group_id: String,
    pub kind: MechanicalGeometryKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub layer: Option<String>,
    #[serde(default)]
    pub layers: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub purpose: Option<MechanicalPurpose>,
    pub geometry: MechanicalShape,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "export-types", ts(optional_fields))]
pub struct MechanicalGeometry {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_name: Option<String>,
    pub primitives: Vec<MechanicalPrimitive>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct MechanicalExtraction {
    pub geometry: MechanicalGeometry,
    pub plate_cutouts: Vec<Vec<Vec2>>,
    pub clearance_envelopes: Vec<Vec<Vec2>>,
    pub pcb_holes: Vec<MechanicalPcbHole>,
    pub source_geometry: MechanicalProfileSource,
}

/// Render-only projection of a KiCad board. All coordinates are millimetres, Y up.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct PcbPreview {
    pub revision: u64,
    pub thickness: f64,
    pub contours: Vec<Contour>,
    pub surfaces: Vec<PcbSurface>,
    pub holes: Vec<Vec<Vec2>>,
    pub models: Vec<PcbModel>,
    pub diagnostics: Vec<String>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct PcbSurface {
    pub layer: String,
    pub points: Vec<Vec2>,
    pub width: f64,
    pub filled: bool,
    pub text: String,
    pub rotation: f64,
    pub text_size: f64,
}
/// Model transforms retain KiCad's clockwise ZYX convention, not Pose2's convention.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct PcbModel {
    pub id: String,
    pub reference: String,
    pub path: String,
    pub pose: Pose2,
    pub side: Side,
    pub offset: Vec3,
    pub rotation: Vec3,
    pub scale: Vec3,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct BoardReference {
    pub id: String,
    pub board_id: String,
    pub asset_id: String,
    pub enabled: bool,
    pub pose: Pose2,
    pub elevation: f64,
    pub model_assets: BTreeMap<String, String>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct AssemblyDefinition {
    pub id: String,
    pub name: String,
    pub members: Vec<AssemblyMember>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct AssemblyMember {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "export-types", ts(optional))]
    pub parameters: Option<BTreeMap<String, serde_json::Value>>,
    #[serde(rename = "modelMode", default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "export-types", ts(optional))]
    pub model_mode: Option<AssemblyModelMode>,
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "export-types", ts(optional))]
    pub definition_id: Option<String>,
    pub pose: Pose2,
    pub side: Side,
    pub models: Vec<PartModel>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
pub struct CaseBodyMesh {
    pub id: String,
    pub name: String,
    #[cfg_attr(feature = "export-types", ts(type = "Float32Array"))]
    pub positions: Vec<f32>,
    #[cfg_attr(feature = "export-types", ts(type = "Float32Array"))]
    pub normals: Vec<f32>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
#[serde(rename_all = "lowercase")]
pub enum AssemblyModelMode {
    Defaults,
    Custom,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
#[serde(rename_all = "kebab-case")]
#[derive(Default)]
pub enum PlateMethod {
    PcbFr4,
    #[default]
    Printed,
    Cnc,
    CutSheet,
}

fn unset_mechanical_dimension() -> f64 {
    -1.0
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
#[serde(rename_all = "kebab-case")]
pub enum MechanicalMount {
    Tray,
    Rigid,
    Gasket,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "export-types", ts(optional_fields))]
pub struct MechanicalPartProfile {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_geometry: Option<MechanicalProfileSource>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pcb_holes: Option<Vec<MechanicalPcbHole>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub clearance_volumes: Option<Vec<CaseOpening>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub openings: Option<Vec<CaseOpening>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub clearances: Option<Vec<Vec<Vec2>>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub supported_thickness: Option<Vec2>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub switch_family: Option<MechanicalSwitchFamily>,
    pub definition_id: String,
    pub source: String,
    pub cutouts: Vec<Vec<Vec2>>,
    pub plate_to_pcb: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct MechanicalGasketLayout {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "export-types", ts(optional))]
    pub auto_size: Option<bool>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "export-types", ts(optional))]
    pub adhesive_thickness: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "export-types", ts(optional))]
    pub minimum_foam_thickness: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "export-types", ts(optional))]
    pub preset_id: Option<GasketFoamPreset>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "export-types", ts(optional))]
    pub material: Option<String>,
    pub length: f64,
    pub width: f64,
    pub thickness: f64,
    pub compression: f64,
    #[serde(default)]
    pub supports: Vec<MechanicalGasketAnchor>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct MechanicalGasketAnchor {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "export-types", ts(optional))]
    pub length: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "export-types", ts(optional))]
    pub width: Option<f64>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "export-types", ts(optional))]
    pub placement: Option<GasketPlacement>,
    pub id: String,
    pub region_id: String,
    pub outline_key: String,
    pub anchor: f64,
    #[serde(default)]
    pub unlinked: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct MechanicalGasketSupport {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "export-types", ts(optional))]
    pub fit_error: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "export-types", ts(optional))]
    pub placement: Option<GasketPlacement>,
    pub id: String,
    pub region_id: String,
    pub outline_key: String,
    pub anchor: f64,
    pub at: Vec2,
    pub tangent: Vec2,
    pub normal: Vec2,
    pub length: f64,
    pub width: f64,
    pub z: f64,
    pub thickness: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pair_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mirror_axis: Option<f64>,
    pub unlinked: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct MechanicalGasketTrack {
    pub region_id: String,
    pub start: Vec2,
    pub end: Vec2,
    pub start_anchor: f64,
    pub end_anchor: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "export-types", ts(optional_fields))]
pub struct MechanicalConfiguration {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub internal_gasket: Option<InternalGasketConfiguration>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gasket_layout: Option<MechanicalGasketLayout>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hardware: Option<Vec<MechanicalHardwareSpecification>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub critical_fits: Option<Vec<MechanicalCriticalFit>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bottom_style: Option<MechanicalBottomStyle>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub middle_frame: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gasket_travel: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "export-types", ts(optional))]
    pub openings: Option<Vec<CaseOpening>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub opening_allowance: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stabilizers: Option<Vec<MechanicalStabilizerOverride>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub part_processes: Option<Vec<MechanicalPartProcess>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gasket: Option<Gasket>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub closure_mounts: Option<Vec<Mount>>,
    pub board_id: String,
    #[serde(default)]
    pub integrated_plate_frame: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub battery: Option<MechanicalBattery>,
    #[serde(default)]
    pub mounts: Vec<Mount>,
    pub method: PlateMethod,
    pub mount: MechanicalMount,
    pub plate_thickness: f64,
    pub plate_foam_thickness: f64,
    pub pcb_thickness: f64,
    pub bottom_foam_thickness: f64,
    pub battery_height: f64,
    pub bottom_thickness: f64,
    pub plate_to_pcb: f64,
    pub wall_thickness: f64,
    pub clearance: f64,
    pub profiles: Vec<MechanicalPartProfile>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct MechanicalStackLayer {
    pub id: String,
    pub z: f64,
    pub thickness: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct MechanicalAssembly {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(
        feature = "export-types",
        ts(as = "Option<Vec<MechanicalMaterialSpecification>>", optional)
    )]
    pub generated_materials: Vec<MechanicalMaterialSpecification>,
    #[serde(default)]
    pub gasket_supports: Vec<MechanicalGasketSupport>,
    #[serde(default)]
    pub gasket_tracks: Vec<MechanicalGasketTrack>,
    #[serde(default)]
    pub generated_hardware: Vec<MechanicalHardwareSpecification>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "export-types", ts(optional))]
    pub pcb_reference: Option<CaseIR>,
    pub suggested_mounts: Vec<Mount>,
    pub nominal_plate_contours: Vec<Contour>,
    pub revision: u64,
    pub plate_contours: Vec<Contour>,
    pub case: CaseAssemblyIR,
    pub stack: Vec<MechanicalStackLayer>,
    pub diagnostics: Vec<Finding>,
    #[serde(default)]
    pub generation_blocked: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct MechanicalBattery {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "export-types", ts(optional))]
    pub cable_width: Option<f64>,
    pub size: Vec3,
    pub at: Vec2,
    pub cable_exit: Vec2,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
#[serde(rename_all = "kebab-case")]
pub enum MechanicalBuiltinProfile {
    MxSwitch,
    ChocV1Switch,
    ChocV2Switch,
    MxStab2u,
    MxStab625u,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
#[serde(rename_all = "kebab-case")]
pub enum MechanicalSwitchFamily {
    Mx,
    ChocV1,
    ChocV2,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct MechanicalPartProcess {
    pub part_id: String,
    #[serde(default)]
    #[cfg_attr(feature = "export-types", ts(as = "Option<PlateMethod>", optional))]
    pub method: PlateMethod,
    #[serde(default)]
    #[cfg_attr(feature = "export-types", ts(as = "Option<String>", optional))]
    pub material: String,
    #[serde(default = "unset_mechanical_dimension")]
    #[cfg_attr(feature = "export-types", ts(as = "Option<f64>", optional))]
    pub thickness: f64,
    #[serde(default)]
    #[cfg_attr(feature = "export-types", ts(as = "Option<String>", optional))]
    pub constraints_version: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
#[serde(rename_all = "kebab-case")]
pub enum MechanicalStabilizerKind {
    None,
    PcbMount,
    PlateMount,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
#[cfg_attr(feature = "export-types", ts(optional_fields))]
#[serde(rename_all = "camelCase")]
pub struct MechanicalStabilizerOverride {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub profile: Option<MechanicalPartProfile>,
    pub part_id: String,
    pub kind: MechanicalStabilizerKind,
    pub units: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rotation: Option<f64>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
pub struct CaseOpening {
    pub points: Vec<Vec2>,
    pub z: f64,
    pub height: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct MechanicalProfileSource {
    pub text: String,
    pub sha256: String,
    pub mappings: Vec<MechanicalPurposeMapping>,
    pub source_ids: Vec<String>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct MechanicalPcbHole {
    pub source_id: String,
    pub at: Vec2,
    pub diameter: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
#[serde(rename_all = "lowercase")]
pub enum MechanicalBottomStyle {
    Shell,
    Sheet,
}

/// Manufacturing callout metadata; thread geometry is not modeled.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "export-types", ts(optional_fields))]
pub struct MechanicalHardwareSpecification {
    pub id: String,
    pub part_id: String,
    pub feature_id: String,
    pub designation: String,
    pub thread: String,
    pub length: f64,
    pub quantity: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tolerance: Option<String>,
}

/// A dimension between two document-space XY points, in millimetres.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct MechanicalCriticalFit {
    pub id: String,
    pub part_id: String,
    pub label: String,
    pub from: Vec2,
    pub to: Vec2,
    pub tolerance: String,
}

/// Opt-in construction; absence retains the legacy gasket generator.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
#[serde(rename_all = "kebab-case")]
pub enum GasketConstructionVersion {
    InternalV1,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct InternalGasketConfiguration {
    pub version: GasketConstructionVersion,
    #[serde(default = "default_internal_minimum_wall")]
    pub minimum_wall: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "export-types", ts(optional))]
    pub support_clearance: Option<f64>,
    pub tolerance: f64,
    pub support_count: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "export-types", ts(optional))]
    pub auto_count: Option<bool>,
    pub hardware: InternalClosureHardware,
}

/// Complete custom geometry. Catalog provenance and process approval are separate from geometry.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct InternalClosureHardware {
    pub drive: ScrewDrive,
    pub installation: InsertInstallation,
    pub thread_diameter: f64,
    pub pitch: f64,
    pub insert_length: f64,
    pub thread_start: f64,
    pub tip_allowance: f64,
    pub seat_lead_depth: f64,
    pub seat_lead_diameter: f64,
    pub bearing_thickness: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "export-types", ts(optional))]
    pub fixed_length: Option<f64>,
    pub length_datum: ScrewLengthDatum,
    pub head_profile: ScrewHeadProfile,
    pub id: String,
    pub thread: String,
    pub screw_lengths: Vec<f64>,
    pub head_diameter: f64,
    pub head_height: f64,
    pub hole_diameter: f64,
    pub insert_diameter: f64,
    pub seat_diameter: f64,
    pub seat_depth: f64,
    pub engagement: f64,
    pub bottoming_clearance: f64,
    pub roof: f64,
    pub surround: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
#[serde(rename_all = "kebab-case")]
pub enum ScrewLengthDatum {
    UnderHead,
    Overall,
    Unresolved,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
#[serde(rename_all = "kebab-case")]
pub enum ScrewHeadProfile {
    Flat,
    Countersunk,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
#[serde(rename_all = "kebab-case")]
pub enum GasketPlacement {
    Generated,
    User,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
#[serde(rename_all = "kebab-case")]
pub enum ScrewDrive {
    Hex,
    Torx,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
#[serde(rename_all = "kebab-case")]
pub enum InsertInstallation {
    HeatSet,
    Tapped,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
pub enum GasketFoamPreset {
    A2,
    A3,
    A4,
    B2,
    B3,
    B4,
    E2,
    E3,
    E4,
    F2,
    F3,
    F4,
    F5,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "export-types", ts(optional_fields))]
pub struct MechanicalMaterialSpecification {
    pub adhesive_thickness: f64,
    pub id: String,
    pub feature_id: String,
    pub quantity: u32,
    pub size: Vec3,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preset_id: Option<GasketFoamPreset>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub material: Option<String>,
    pub notes: String,
}

fn default_internal_minimum_wall() -> f64 {
    2.0
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct KeycapResolution {
    pub revision: u64,
    pub specs: Vec<KeycapSpec>,
    pub findings: Vec<Finding>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum KeycapBoardChange {
    Color { value: String },
    LegendColor { value: String },
    Clearance { value: f64 },
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum KeycapMatrixChange {
    Profile { value: Option<KeycapProfile> },
    Mount { value: Option<KeycapMount> },
    FirstRow { value: u8 },
    WallThickness { value: f64 },
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum KeycapKeyChange {
    Profile { value: Option<KeycapProfile> },
    Mount { value: Option<KeycapMount> },
    Legend { value: Option<String> },
    Color { value: Option<String> },
    Row { value: Option<u8> },
    Units { value: Option<Vec2> },
}
