pub use crate::hardware::{HardwareGate, HardwareOutput, HardwareProfile, HardwareSource, VikRole};
pub use crate::inputs::{EncoderDriver, InputProfile, PressContacts, PressScanMode, RotaryProfile};
pub use crate::keymap::{
    EncoderBinding, EncoderDirection, KeyBinding, KeymapChange, KeymapConfiguration, KeymapLayer,
    KeymapMacro, MacroChange, MacroStep,
};
pub use crate::modules::{
    EmbeddedCircuit, ModuleAttachment, ModuleBoard, ModuleCircuit, ModuleCircuitRepair,
    ModuleConnection, ModuleConstituent, ModuleDefinition, ModuleElectrical, ModuleInterface,
    ModuleModelCandidate, ModuleModelPlacement, ModuleProtocol, ModuleResolution, ModuleSupport,
    ModuleSupportGeometry, ModuleVolume, MountedModule, ResolvedModule, ResolvedModuleFootprint,
    ResolvedModulePad, VikSignal,
};
pub use boardstudio_contracts::{
    AffineTransformError, CadStepImportOptions, MAX_COORDINATE_MM, is_sha256_hex,
    validate_affine_row_transform,
};
pub use boardstudio_contracts::{
    CaseAssemblyIR, CaseBody, CaseFeature, CaseIR, CaseKind, CaseOpening, CasePcbCircularSupport,
    CasePcbSlot, CasePcbSource, CasePcbSourceContour, CasePcbSourcePrimitive, Contour, Gasket,
    Mount, MountKind, PreparedCaseAssemblyIR, PreparedCaseIR, PreparedCaseRegion,
    PreparedGasketRegion,
};
pub use boardstudio_contracts::{
    KeycapBoardSettings, KeycapConfiguration, KeycapKeySettings, KeycapMatrixSettings, KeycapMount,
    KeycapProfile, KeycapSpec, Pose2, Side, Vec2,
};
pub use boardstudio_contracts::{MechanicalDrillShape, MechanicalShape};
pub use boardstudio_contracts::{polygon_contains, polygon_signed_area};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArchiveEntry {
    pub path: String,
    pub buffer_index: u32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArchiveAssetBuffer {
    pub sha256: String,
    pub buffer_index: u32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
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
        archive_json: Option<String>,
        assets: Vec<ArchiveEntry>,
    },
    UnpackProject,
    PackFiles {
        entries: Vec<ArchiveEntry>,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
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
pub struct Vec3 {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Material {
    pub id: String,
    pub name: String,
    pub thickness: f64,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
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
#[serde(rename_all = "lowercase")]
pub enum PadShape {
    Circle,
    Oval,
    Rect,
    Roundrect,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PartModel {
    #[serde(rename = "assetId")]
    pub asset_id: String,
    pub offset: Vec3,
    pub rotation: Vec3,
    pub scale: Vec3,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KicadSource {
    pub format_version: u8,
    pub source: String,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PartDefinition {
    #[serde(
        rename = "hardwareProfile",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub hardware_profile: Option<HardwareProfile>,
    #[serde(
        rename = "inputProfile",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub input_profile: Option<InputProfile>,
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
#[serde(rename_all = "camelCase", default)]
pub struct EnvelopeSource {
    pub courtyard: Option<EnvelopeOrigin>,
    pub keycap: Option<EnvelopeOrigin>,
}
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EnvelopeOrigin {
    Generated,
    Authored,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MatrixTerminals {
    pub row: String,
    pub column: String,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PartGenerator {
    pub source: String,
    pub version: String,
    pub parameters: BTreeMap<String, serde_json::Value>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
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
    /// Ticket 13: column-vocabulary anchor for an independent part. `None` is
    /// a detached part placed from the half origin; `Some` derives the
    /// evaluated pose from the anchor key, so anchor edits move the part.
    /// Ignored while the part is a matrix member or generated companion.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub anchor: Option<PartAnchor>,
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

/// Ticket 13: an independent part's placement in the column vocabulary. The
/// part follows the anchor key's pose through the Follow subset; Spread,
/// Stagger and Splay carry in the anchor frame, then Offset and Rotation
/// apply for the part alone. `matrix_id` names the anchor key's matrix, which
/// must live in the part's own layout.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct PartAnchor {
    pub matrix_id: String,
    pub row: u32,
    pub column: u32,
    pub follow: Follow,
    pub spread: f64,
    pub stagger: f64,
    pub splay: f64,
    pub offset: Vec2,
    pub rotation: f64,
}

/// Ticket 13: one typed edit to an anchored part's carried or local values.
/// Spread/Stagger/Splay carry from the anchor; Offset/Rotation are the part's
/// own. Anchor retargets, Follow changes and Detach are separate operations
/// because they re-solve the values to keep the pose.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum PartAnchorValueChange {
    Spread(f64),
    Stagger(f64),
    Splay(f64),
    Offset(Vec2),
    Rotation(f64),
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Pin {
    #[serde(rename = "partId")]
    pub part_id: String,
    #[serde(rename = "padId")]
    pub pad_id: String,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Net {
    pub id: String,
    pub name: String,
    pub pins: Vec<Pin>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
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
    #[serde(
        rename = "columnLayout",
        default,
        skip_serializing_if = "Vec::is_empty"
    )]
    pub column_layout: Vec<ColumnLayout>,
    #[serde(rename = "rowLayout", default, skip_serializing_if = "Vec::is_empty")]
    pub row_layout: Vec<RowLayout>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub anchor: Option<MatrixAnchor>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub cells: Vec<MatrixCell>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MatrixCell {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shift: Option<Vec2>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bend: Option<f64>,
    /// Removed slots have neither generated members nor snapping geometry.
    #[serde(default, skip_serializing_if = "is_false")]
    pub deleted: bool,
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
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub assemblies: Vec<MatrixAssembly>,
    #[serde(
        rename = "assembliesLocal",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub assemblies_local: Option<bool>,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default, deny_unknown_fields)]
pub struct ColumnLayout {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spread: Option<f64>,
    #[serde(skip_serializing_if = "is_zero_f64")]
    pub stagger: f64,
    #[serde(skip_serializing_if = "is_zero_f64")]
    pub splay: f64,
    #[serde(skip_serializing_if = "is_zero_f64")]
    pub rotation: f64,
    #[serde(skip_serializing_if = "is_zero_vec2")]
    pub splay_origin: Vec2,
    #[serde(skip_serializing_if = "is_zero_vec2")]
    pub offset: Vec2,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default, deny_unknown_fields)]
pub struct RowLayout {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub padding: Option<f64>,
    #[serde(skip_serializing_if = "is_zero_vec2")]
    pub shift: Vec2,
    #[serde(skip_serializing_if = "is_zero_f64")]
    pub bend: f64,
    #[serde(skip_serializing_if = "is_zero_vec2")]
    pub offset: Vec2,
}
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "Vec<FollowComponent>", into = "Vec<FollowComponent>")]
pub struct Follow {
    pub x: bool,
    pub y: bool,
    pub rotation: bool,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
enum FollowComponent {
    X,
    Y,
    Rotation,
}
impl TryFrom<Vec<FollowComponent>> for Follow {
    type Error = &'static str;
    fn try_from(values: Vec<FollowComponent>) -> Result<Self, Self::Error> {
        for (index, value) in values.iter().enumerate() {
            if values[..index].contains(value) {
                return Err("Follow components must be unique");
            }
        }
        Ok(Self {
            x: values.contains(&FollowComponent::X),
            y: values.contains(&FollowComponent::Y),
            rotation: values.contains(&FollowComponent::Rotation),
        })
    }
}
impl From<Follow> for Vec<FollowComponent> {
    fn from(follow: Follow) -> Self {
        [
            (follow.x, FollowComponent::X),
            (follow.y, FollowComponent::Y),
            (follow.rotation, FollowComponent::Rotation),
        ]
        .into_iter()
        .filter_map(|(enabled, component)| enabled.then_some(component))
        .collect()
    }
}
impl Default for Follow {
    fn default() -> Self {
        Self {
            x: true,
            y: true,
            rotation: true,
        }
    }
}
/// Stored anchor target: the `single | aggregate` union from ticket 28
/// (`anchor.target: { single: {matrixId,row,column} } | { aggregate: {...} }`
/// with top-level `follow` unchanged). The untagged document grammar keeps
/// the flat shape so saved single-key anchors open byte-identical: a bare
/// `{matrixId,row,column,follow}` document is the single arm, a
/// `{follow,aggregate}` document is the aggregate arm, and a document naming
/// both arms is refused at admission, matching upstream's ref+aggregate
/// throw. The arms are matched explicitly (rather than with an untagged
/// enum) so each arm's inner refusal — e.g. the unknown aggregate method —
/// keeps its own error instead of collapsing into an untagged mismatch.
#[derive(Clone, Debug, PartialEq)]
pub struct MatrixAnchor {
    /// Single-key target. Inert while the aggregate arm is present: the
    /// admission grammar never populates both arms, so in-memory single
    /// fields on an aggregate anchor stay empty/zero and `targets()` never
    /// reads them. Inert fields are also omitted when an aggregate anchor
    /// serializes, so saved aggregate anchors reopen.
    pub matrix_id: String,
    pub row: u32,
    pub column: u32,
    pub follow: Follow,
    /// Aggregate arm (average / intersection over single-key parts). When
    /// present, evaluation resolves every part and combines the poses before
    /// Origin/Rotation and Follow apply; the single-key fields above are
    /// ignored. Parts are single-key refs only: nesting is unrepresentable.
    pub aggregate: Option<AggregateAnchor>,
}

impl serde::Serialize for MatrixAnchor {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;
        match &self.aggregate {
            None => {
                let mut anchor = serializer.serialize_struct("MatrixAnchor", 4)?;
                anchor.serialize_field("matrixId", &self.matrix_id)?;
                anchor.serialize_field("row", &self.row)?;
                anchor.serialize_field("column", &self.column)?;
                anchor.serialize_field("follow", &self.follow)?;
                anchor.end()
            }
            Some(aggregate) => {
                let mut anchor = serializer.serialize_struct("MatrixAnchor", 2)?;
                anchor.serialize_field("follow", &self.follow)?;
                anchor.serialize_field("aggregate", aggregate)?;
                anchor.end()
            }
        }
    }
}

impl<'de> serde::Deserialize<'de> for MatrixAnchor {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_map(AnchorUnionVisitor)
    }
}

struct AnchorUnionVisitor;

impl<'de> serde::de::Visitor<'de> for AnchorUnionVisitor {
    type Value = MatrixAnchor;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str(
            "a single-key anchor {matrixId, row, column, follow} or an aggregate anchor {follow, aggregate}",
        )
    }

    fn visit_map<A: serde::de::MapAccess<'de>>(self, mut map: A) -> Result<MatrixAnchor, A::Error> {
        use serde::de::Error;
        let mut matrix_id: Option<String> = None;
        let mut row: Option<u32> = None;
        let mut column: Option<u32> = None;
        let mut follow = Follow::default();
        let mut follow_seen = false;
        let mut aggregate: Option<AggregateAnchor> = None;
        while let Some(key) = map.next_key::<String>()? {
            match key.as_str() {
                "matrixId" => {
                    if matrix_id.is_some() {
                        return Err(A::Error::duplicate_field("matrixId"));
                    }
                    matrix_id = Some(map.next_value()?);
                }
                "row" => {
                    if row.is_some() {
                        return Err(A::Error::duplicate_field("row"));
                    }
                    row = Some(map.next_value()?);
                }
                "column" => {
                    if column.is_some() {
                        return Err(A::Error::duplicate_field("column"));
                    }
                    column = Some(map.next_value()?);
                }
                "follow" => {
                    if follow_seen {
                        return Err(A::Error::duplicate_field("follow"));
                    }
                    follow_seen = true;
                    follow = map.next_value()?;
                }
                "aggregate" => {
                    if aggregate.is_some() {
                        return Err(A::Error::duplicate_field("aggregate"));
                    }
                    aggregate = Some(map.next_value()?);
                }
                other => {
                    return Err(A::Error::unknown_field(
                        other,
                        &["matrixId", "row", "column", "follow", "aggregate"],
                    ));
                }
            }
        }
        if aggregate.is_some() && (matrix_id.is_some() || row.is_some() || column.is_some()) {
            return Err(A::Error::custom(
                "Matrix anchor cannot combine a single-key target (matrixId/row/column) with an aggregate arm; name one target",
            ));
        }
        Ok(MatrixAnchor {
            matrix_id: matrix_id.unwrap_or_default(),
            row: row.unwrap_or_default(),
            column: column.unwrap_or_default(),
            follow,
            aggregate,
        })
    }
}

impl MatrixAnchor {
    /// Every single-key target this anchor resolves: the single key, or each
    /// aggregate part in order. Evaluation order, cycle walks and part-aware
    /// guards consume this so aggregates never fall back to the inert
    /// single-key fields.
    pub fn targets(&self) -> Vec<AnchorPart> {
        match &self.aggregate {
            None => vec![AnchorPart {
                matrix_id: self.matrix_id.clone(),
                row: self.row,
                column: self.column,
            }],
            Some(aggregate) => aggregate.parts.clone(),
        }
    }

    pub fn is_aggregate(&self) -> bool {
        self.aggregate.is_some()
    }
}

/// Aggregate anchor target: one combined pose from N single-key parts.
/// Unknown methods are refused at admission; the default is upstream's
/// `average`. An empty part list parses but refuses at evaluation.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AggregateAnchor {
    #[serde(default)]
    pub method: AggregateMethod,
    #[serde(default)]
    pub parts: Vec<AnchorPart>,
}

/// Upstream Ergogen aggregators, nothing else. Deserializing any other
/// method name fails, which is the unknown-method refusal.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AggregateMethod {
    #[default]
    Average,
    Intersect,
}

/// One single-key part reference. There is no nested-anchor arm here, so
/// aggregates cannot nest by construction.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AnchorPart {
    pub matrix_id: String,
    pub row: u32,
    pub column: u32,
}
fn is_zero_f64(value: &f64) -> bool {
    *value == 0.0
}
fn is_zero_vec2(value: &Vec2) -> bool {
    *value == Vec2::default()
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
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
#[serde(rename_all = "lowercase")]
pub enum Mirror {
    None,
    X,
    Y,
}
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DiodeDirection {
    Row2col,
    Col2row,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct PartOutline {
    pub excluded: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub margin: Option<f64>,
    #[serde(skip_serializing_if = "is_false")]
    pub allow_body_overhang: bool,
}
fn is_false(value: &bool) -> bool {
    !value
}
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum CornerStyle {
    #[default]
    Sharp,
    Fillet,
    Chamfer,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct OutlineSettings {
    pub corners: CornerStyle,
    pub size: f64,
    pub bridge_width: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
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
#[serde(rename_all = "camelCase", default)]
pub struct OutlineRepairSettings {
    pub enabled: bool,
    pub maximum_gap_span: f64,
    pub minimum_connection_width: f64,
    pub edge_clearance: f64,
    #[serde(skip_serializing_if = "Vec::is_empty")]
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
pub struct ProtectedOutlineGap {
    pub id: String,
    pub points: Vec<OutlineControlPoint>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OutlineControlPoint {
    pub at: Vec2,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub part_id: Option<String>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct OutlineConnection {
    pub id: String,
    pub width: f64,
    pub points: Vec<OutlineControlPoint>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum OutlineFeature {
    Polygon {
        #[serde(
            default,
            rename = "anchorPartId",
            skip_serializing_if = "Option::is_none"
        )]
        anchor_part_id: Option<String>,
        id: String,
        points: Vec<Vec2>,
        operation: Operation,
    },
    Rect {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        rotation: Option<f64>,
        #[serde(
            default,
            rename = "anchorPartId",
            skip_serializing_if = "Option::is_none"
        )]
        anchor_part_id: Option<String>,
        id: String,
        center: Vec2,
        size: Vec2,
        radius: f64,
        operation: Operation,
    },
    PartEnvelope {
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        connections: Vec<OutlineConnection>,
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
#[serde(rename_all = "camelCase")]
pub struct BoardOutline {
    pub board_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub active_version_id: Option<String>,
    pub versions: Vec<OutlineVersion>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub generated_last_valid: Option<OutlineSnapshot>,
    /// Version identities this board used and removed, in removal order.
    ///
    /// Core never reissues one, so a pending edit that names a removed version can never
    /// land on the version created after it. Feature, connection and gap identities are
    /// namespaced by their version identity, so reserving that identity reserves the
    /// whole derived namespace. Omitted while empty, which keeps older documents' shape.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub retired_version_ids: Vec<String>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OutlineProvenance {
    pub revision: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version_id: Option<String>,
}
/// Who owns a version's boundary shape.
///
/// [`OutlineOwnership::Fixed`] is GLOSSARY.md's "edited outline": the geometry stays put
/// when components move. [`OutlineOwnership::Linked`] is an "outline refinement" that
/// follows the board's current included components. Absence in a persisted document
/// means fixed, so existing copies never gain a link and their serialized shape is
/// unchanged.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum OutlineOwnership {
    #[default]
    Fixed,
    Linked,
}
impl OutlineOwnership {
    /// Serde helper: fixed ownership is the default and is left out of the document.
    pub fn is_fixed(&self) -> bool {
        matches!(self, Self::Fixed)
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct OutlineVersion {
    pub id: String,
    pub name: String,
    pub source: OutlineProvenance,
    pub geometry: OutlineSnapshot,
    /// Explicit owner of this version's shape; provenance is origin information only.
    #[serde(default, skip_serializing_if = "OutlineOwnership::is_fixed")]
    pub ownership: OutlineOwnership,
}
/// Fixed world geometry, before corner finishing; source IDs are provenance only.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OutlineSnapshot {
    pub features: Vec<OutlineFeature>,
    pub settings: OutlineSettings,
    pub expected_regions: u32,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub bridges: Vec<OutlineBridge>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub protected_gaps: Vec<ProtectedOutlineGap>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
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
#[serde(rename_all = "camelCase")]
pub struct OutlineGap {
    pub id: String,
    pub feature_id: String,
    pub span: f64,
    pub points: Vec<OutlineControlPoint>,
    pub protected: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub protected_ids: Vec<String>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BoardOutlineScene {
    pub board_id: String,
    pub source_contours: Vec<Contour>,
    pub bridges: Vec<OutlineBridge>,
    pub gaps: Vec<OutlineGap>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FindingMarker {
    pub finding_id: String,
    pub board_id: String,
    pub contours: Vec<Contour>,
}
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Operation {
    Add,
    Subtract,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
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
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub traces: Vec<CopperTrace>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub vias: Vec<CopperVia>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
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
pub struct CopperVia {
    pub id: String,
    pub at: Vec2,
    pub size: f64,
    pub drill: f64,
    #[serde(rename = "netId", skip_serializing_if = "Option::is_none")]
    pub net_id: Option<String>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
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
pub struct Script {
    pub id: String,
    pub name: String,
    pub source: String,
    pub enabled: bool,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum ConstraintEndpoint {
    Part {
        #[serde(rename = "partId")]
        part_id: String,
    },
    LinkedMirror {
        #[serde(rename = "pairId")]
        pair_id: String,
        #[serde(rename = "sourcePartId")]
        source_part_id: String,
        #[serde(rename = "mirrorPartId")]
        mirror_part_id: String,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum ConstraintRelation {
    /// `offset` is a vector along the board's axes, added to the source position.
    Offset { offset: Vec2, rotation: f64 },
    /// `offset` runs along the source's own axes (ADR-0011).
    PartOffset { offset: Vec2, rotation: f64 },
    /// Reflection across a line with a point on it and its tangent direction in degrees.
    MirrorLine { at: Vec2, angle: f64 },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
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
    /// Like `Offset`, but `offset` runs along the source part's own axes, so a target
    /// held to a turning source keeps its pose in the source's frame. See ADR-0011.
    PartOffset {
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
    Framed {
        id: String,
        source: ConstraintEndpoint,
        target: ConstraintEndpoint,
        relation: ConstraintRelation,
    },
}
impl Constraint {
    /// An offset constraint whose numbers run along `axes`: `Board` builds `offset`, `Part`
    /// builds `part-offset`.
    pub fn offset(
        axes: OffsetAxes,
        id: String,
        source_part_id: String,
        target_part_id: String,
        offset: Vec2,
        rotation: f64,
    ) -> Self {
        match axes {
            OffsetAxes::Board => Self::Offset {
                id,
                source_part_id,
                target_part_id,
                offset,
                rotation,
            },
            OffsetAxes::Part => Self::PartOffset {
                id,
                source_part_id,
                target_part_id,
                offset,
                rotation,
            },
        }
    }
    pub fn id(&self) -> &str {
        match self {
            Self::Offset { id, .. }
            | Self::PartOffset { id, .. }
            | Self::Mirror { id, .. }
            | Self::Framed { id, .. } => id,
        }
    }
    pub fn source(&self) -> &str {
        match self {
            Self::Offset { source_part_id, .. }
            | Self::PartOffset { source_part_id, .. }
            | Self::Mirror { source_part_id, .. } => source_part_id,
            Self::Framed { source, .. } => match source {
                ConstraintEndpoint::Part { part_id } => part_id,
                ConstraintEndpoint::LinkedMirror { mirror_part_id, .. } => mirror_part_id,
            },
        }
    }
    pub fn target(&self) -> &str {
        match self {
            Self::Offset { target_part_id, .. }
            | Self::PartOffset { target_part_id, .. }
            | Self::Mirror { target_part_id, .. } => target_part_id,
            Self::Framed { target, .. } => match target {
                ConstraintEndpoint::Part { part_id } => part_id,
                ConstraintEndpoint::LinkedMirror { mirror_part_id, .. } => mirror_part_id,
            },
        }
    }
    /// Whether this reflects across a coordinate rather than offsetting.
    pub fn is_mirror(&self) -> bool {
        matches!(
            self,
            Self::Mirror { .. }
                | Self::Framed {
                    relation: ConstraintRelation::MirrorLine { .. },
                    ..
                }
        )
    }
    /// The axes an offset constraint runs along, or `None` for a mirror.
    pub fn offset_axes(&self) -> Option<OffsetAxes> {
        match self {
            Self::Offset { .. } => Some(OffsetAxes::Board),
            Self::PartOffset { .. } => Some(OffsetAxes::Part),
            Self::Framed {
                relation: ConstraintRelation::Offset { .. },
                ..
            } => Some(OffsetAxes::Board),
            Self::Framed {
                relation: ConstraintRelation::PartOffset { .. },
                ..
            } => Some(OffsetAxes::Part),
            Self::Mirror { .. }
            | Self::Framed {
                relation: ConstraintRelation::MirrorLine { .. },
                ..
            } => None,
        }
    }
    /// The offset and turn an offset constraint stores, whichever axes they run along, or
    /// `None` for a mirror.
    pub fn offset_numbers(&self) -> Option<(Vec2, f64)> {
        match self {
            Self::Offset {
                offset, rotation, ..
            }
            | Self::PartOffset {
                offset, rotation, ..
            }
            | Self::Framed {
                relation: ConstraintRelation::Offset { offset, rotation },
                ..
            }
            | Self::Framed {
                relation: ConstraintRelation::PartOffset { offset, rotation },
                ..
            } => Some((*offset, *rotation)),
            Self::Mirror { .. }
            | Self::Framed {
                relation: ConstraintRelation::MirrorLine { .. },
                ..
            } => None,
        }
    }
    /// Whether every stored number is finite.
    pub fn is_finite(&self) -> bool {
        match self {
            Self::Offset {
                offset, rotation, ..
            }
            | Self::PartOffset {
                offset, rotation, ..
            } => offset.x.is_finite() && offset.y.is_finite() && rotation.is_finite(),
            Self::Mirror { coordinate, .. } => coordinate.is_finite(),
            Self::Framed { relation, .. } => relation.is_finite(),
        }
    }
}

impl ConstraintRelation {
    fn is_finite(&self) -> bool {
        match self {
            Self::Offset { offset, rotation } | Self::PartOffset { offset, rotation } => {
                offset.x.is_finite() && offset.y.is_finite() && rotation.is_finite()
            }
            Self::MirrorLine { at, angle } => {
                at.x.is_finite() && at.y.is_finite() && angle.is_finite()
            }
        }
    }
}
/// The axes an offset constraint's offset runs along: `Board` is `offset`, `Part` is
/// `part-offset`. See ADR-0011.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum OffsetAxes {
    Board,
    Part,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MirrorAxis {
    Vertical,
    Horizontal,
}
/// A named key layout and its independent, board-owned components.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Layout {
    pub id: String,
    pub name: String,
    pub board_id: String,
    pub matrix_ids: Vec<String>,
    pub part_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mirror_link: Option<LayoutMirrorLink>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LayoutMirrorLink {
    pub source_id: String,
    pub axis_x: f64,
    /// Stored Split gap: the key-edge to key-edge distance between the halves,
    /// measured at the half-origin column. Absent on documents saved before the
    /// half-frame contract; those keep board-coordinate placement until their
    /// first committed geometry edit adopts the half frame.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub split_gap: Option<f64>,
    /// Current spacing: the evaluated key-edge distance, always
    /// `max(split_gap, minimum)` once a committed edit has run the adaptive
    /// pass. Absent until the first committed edit after the half frame is
    /// adopted; while absent, evaluation follows `split_gap` exactly so saved
    /// boards open with byte-identical geometry.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spacing: Option<f64>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub correspondences: Vec<MirroredCorrespondence>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub exceptions: Vec<MirroredException>,
}
impl LayoutMirrorLink {
    pub fn partner_matrix(&self, id: &str) -> Option<&str> {
        self.correspondences.iter().find_map(|record| {
            if record.kind != MirroredItemKind::Matrix {
                return None;
            }
            if record.source_id == id {
                Some(record.mirror_id.as_str())
            } else if record.mirror_id == id {
                Some(record.source_id.as_str())
            } else {
                None
            }
        })
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ProjectDoc {
    /// View-only physical context used by Rust assembly preparation; canonical edits omit it.
    #[serde(
        rename = "physicalInstanceId",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub physical_instance_id: Option<String>,
    #[serde(
        rename = "moduleDefinitions",
        default,
        skip_serializing_if = "Vec::is_empty"
    )]
    pub module_definitions: Vec<ModuleDefinition>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub modules: Vec<MountedModule>,
    #[serde(
        rename = "embeddedCircuits",
        default,
        skip_serializing_if = "Vec::is_empty"
    )]
    pub embedded_circuits: Vec<EmbeddedCircuit>,
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
    pub board_references: Vec<BoardReference>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub assemblies: Vec<AssemblyDefinition>,
    pub format: String,
    /// Schema version of the document; a document without one is version 1.
    #[serde(rename = "formatVersion", default = "legacy_format_version")]
    pub format_version: u32,
    pub id: String,
    pub name: String,
    pub revision: u64,
    pub parameters: BTreeMap<String, serde_json::Value>,
    pub definitions: Vec<PartDefinition>,
    pub parts: Vec<Part>,
    pub matrices: Vec<Matrix>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub layouts: Vec<Layout>,
    pub nets: Vec<Net>,
    pub outline: Vec<OutlineFeature>,
    #[serde(
        rename = "boardOutlines",
        default,
        skip_serializing_if = "Vec::is_empty"
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
fn legacy_format_version() -> u32 {
    crate::migrate::LEGACY_VERSION
}

impl ProjectDoc {
    /// Finds the constraint that drives this part through its effective mirror authority.
    ///
    /// A part and its linked mirror resolve to the same driving constraint. Multiple records
    /// on that authority return a located error instead of selecting one.
    pub fn constraint_driving_part(&self, part_id: &str) -> Result<Option<&Constraint>, String> {
        crate::constraints::driving_constraint(self, part_id)
    }

    pub fn empty(id: &str, name: &str) -> Self {
        Self {
            module_definitions: vec![],
            physical_instance_id: None,
            modules: vec![],
            embedded_circuits: vec![],
            keymap: None,
            keycaps: None,
            hardware: None,
            mechanical: None,
            board_references: vec![],
            assemblies: vec![],
            format: "boardstudio/v2".into(),
            format_version: crate::migrate::CURRENT_VERSION,
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
#[serde(rename_all = "camelCase", default)]
pub struct HardwareConfiguration {
    pub topology: HardwareTopology,
    pub transport: HardwareTransport,
    pub instances: Vec<PhysicalBoardInstance>,
    pub boards: Vec<ElectricalBoardConfiguration>,
    pub shared_construction: Option<MechanicalConfiguration>,
    /// Explicit mechanical mirror pairs between physical instances.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub pairs: Vec<MechanicalMirrorPair>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum HardwareTopology {
    #[default]
    Unibody,
    Split,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum HardwareTransport {
    #[default]
    None,
    Wireless,
    Wired,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
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
    /// Who owns this instance's construction values. Replaces the old
    /// `constructionLinked` flag, whose value never affected the effective overlay.
    #[serde(default)]
    pub construction_ownership: MechanicalConstructionOwnership,
    /// Monotonic identity reservations that outlive this half's mechanical
    /// configuration, so disabling a half cannot let a later addition reuse an opening
    /// or vertex identity that a queued intent may still name. Absent for legacy
    /// documents and for halves that never reserved one, which keeps their serialized
    /// shape unchanged, so no document-version migration is required.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mechanical_identity_reservation: Option<MechanicalIdentityReservation>,
}

/// Monotonic high-water marks for a physical half's authored opening and vertex
/// identities. See [`PhysicalBoardInstance::mechanical_identity_reservation`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MechanicalIdentityReservation {
    #[serde(default, skip_serializing_if = "is_zero")]
    pub next_opening_identity: u64,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub next_vertex_identity: u64,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub next_critical_fit_identity: u64,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub next_hardware_identity: u64,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub next_mount_identity: u64,
}

impl MechanicalIdentityReservation {
    /// The pointwise maximum of two reservations.
    pub fn merged(self, other: Self) -> Self {
        Self {
            next_opening_identity: self.next_opening_identity.max(other.next_opening_identity),
            next_vertex_identity: self.next_vertex_identity.max(other.next_vertex_identity),
            next_critical_fit_identity: self
                .next_critical_fit_identity
                .max(other.next_critical_fit_identity),
            next_hardware_identity: self
                .next_hardware_identity
                .max(other.next_hardware_identity),
            next_mount_identity: self.next_mount_identity.max(other.next_mount_identity),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
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
#[serde(rename_all = "camelCase")]
pub struct ElectricalHandoffBaseline {
    pub fingerprint: String,
    pub revision: u64,
    pub assignments: BTreeMap<String, String>,
}

/// Who owns an instance's construction values.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum MechanicalConstructionOwnership {
    /// Today's effective shared overlay with first-instance fallback, kept until
    /// the user explicitly converts the instance.
    #[default]
    Legacy,
    /// The instance's own mechanical configuration is authoritative.
    Local,
    /// A [`MechanicalMirrorPair`]'s construction is authoritative.
    Pair,
}

/// An explicit mechanical mirror pair between exactly two physical instances.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MechanicalMirrorPair {
    pub id: String,
    pub members: [String; 2],
    /// The map from the first member's Case frame to the second's: a [`Pose2`]
    /// composed with a fixed X reflection,
    /// `q = R(rotation) * (-p.x, p.y) + at`. `rotation` uses the project's degree
    /// convention, and the exact inverse maps the second member back to the first.
    pub mapping: Pose2,
    pub construction: MechanicalConstruction,
    /// The member whose geometry authors the pair's construction and closure proposals. Absent
    /// on pairs linked before format 5, where the first member is the source.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub construction_source: Option<String>,
    /// The mirrored-half record (ADR-0010). Absent means the pair links mechanical features only.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub halves: Option<MirroredHalves>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub features: Vec<MechanicalFeature>,
}

/// Placement of a switch's diode or LED on its mirrored half (ADR-0010).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum MirroredCompanionMode {
    /// Reflect the companion's switch-local position and heading.
    #[default]
    LiteralMirror,
    /// Keep the source's switch-local position and heading.
    DuplicatePosition,
    /// Reflect, then add the item's switch-local adjustment.
    LiteralMirrorWithTweak,
}

impl MirroredCompanionMode {
    fn is_literal_mirror(&self) -> bool {
        *self == Self::LiteralMirror
    }
}

/// An additive adjustment in the mirrored switch's local frame, in mm and degrees.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct MirroredCompanionTweak {
    pub x: f64,
    pub y: f64,
    pub rotation: f64,
}

/// A target for one companion placement edit.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum CompanionPlacementTarget {
    Pair,
    Item {
        #[serde(rename = "itemKind")]
        item_kind: MirroredItemKind,
        #[serde(rename = "mirrorId")]
        mirror_id: String,
    },
    Row {
        #[serde(rename = "matrixId")]
        matrix_id: String,
        row: u32,
    },
    Column {
        #[serde(rename = "matrixId")]
        matrix_id: String,
        column: u32,
    },
    Matrix {
        #[serde(rename = "matrixId")]
        matrix_id: String,
    },
}

/// Which boards a mirrored pair relates, and how their items correspond.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MirroredHalves {
    /// Default for companion correspondences without a per-item override. Absent in older v5 documents.
    #[serde(
        default,
        skip_serializing_if = "MirroredCompanionMode::is_literal_mirror"
    )]
    pub companion_mode: MirroredCompanionMode,
    /// The board whose items are authored.
    pub source_board_id: String,
    /// The board that reflects the source.
    pub mirror_board_id: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub correspondences: Vec<MirroredCorrespondence>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub exceptions: Vec<MirroredException>,
}

/// The mirror of one source item. Each side is unique per kind.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MirroredCorrespondence {
    /// Overrides the pair default for this companion only.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub companion_mode: Option<MirroredCompanionMode>,
    /// Retained per item; only valid with the effective `literal-mirror-with-tweak` mode.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub companion_tweak: Option<MirroredCompanionTweak>,
    pub kind: MirroredItemKind,
    pub source_id: String,
    pub mirror_id: String,
}

/// A state overlay on a mirror item: kept `local` or `omitted`. The item's identity stays in its
/// correspondence; an exception with no matching correspondence is a legacy, unassociated record
/// whose mirror item is present and has no inferred source. An exception never changes the source
/// or a neighbouring item.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MirroredException {
    /// Retained when a companion becomes local or omitted, for a later Rejoin.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub companion_mode: Option<MirroredCompanionMode>,
    /// Retained per item; only valid with the effective `literal-mirror-with-tweak` mode.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub companion_tweak: Option<MirroredCompanionTweak>,
    pub kind: MirroredItemKind,
    pub mirror_id: String,
    pub state: MirroredExceptionState,
    /// Mirror-board copper removed with an Omitted item, kept so a Rejoin can restore it while the
    /// source pads still line up. Only valid on an Omitted exception that overlays a correspondence.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub removed_copper: Option<OmittedCopper>,
}

/// Board copper that belonged only to an Omitted mirror item, and where its pads stood.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OmittedCopper {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub traces: Vec<CopperTrace>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub vias: Vec<CopperVia>,
    /// World position of each omitted pad when the copper was removed.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub pads: Vec<OmittedPad>,
}

/// A mirror pad's world position at the moment its copper was saved.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OmittedPad {
    pub part_id: String,
    pub pad_id: String,
    pub at: Vec2,
}

/// The kinds of item a mirrored half reflects.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum MirroredItemKind {
    Matrix,
    Layout,
    Part,
    Outline,
    Net,
    Assembly,
    /// A module mounted on the board (`modules`, by its host board).
    Module,
    /// A board's electrical configuration (`hardware.boards`, by board id).
    ElectricalBoard,
    /// A fixed outline version of a board (`boardOutlines`, by version id).
    OutlineVersion,
    MechanicalFeature,
    /// An embedded circuit hosted on the board (`embeddedCircuits`, by its host board).
    EmbeddedCircuit,
}

/// Where a [`EditOperation::CreateMirroredHalf`] puts the second half.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum MirroredHalfMode {
    /// A new board that receives a reflected copy of the source board.
    NewBoard {
        #[serde(rename = "boardId")]
        board_id: String,
        #[serde(rename = "instanceId")]
        instance_id: String,
    },
    /// A flipped instance of the source board. It copies nothing: the same board is used twice.
    ReuseBoard {
        #[serde(rename = "instanceId")]
        instance_id: String,
    },
}

/// Optional replacements for one physical instance's labels at mirror creation.
/// Omitted fields keep the source's stored labels or the mirror's default labels.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MirroredHalfLabel {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub half: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
}

/// Labels applied to the source and mirror in the same creation edit (ADR-0010).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MirroredHalfLabels {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<MirroredHalfLabel>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mirror: Option<MirroredHalfLabel>,
}

/// Physical setup values applied atomically with creation of its mirror pair.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MirroredHalfPhysicalSetup {
    pub topology: HardwareTopology,
    pub transport: HardwareTransport,
    pub shared_construction: MechanicalConfiguration,
    pub source_instance: PhysicalBoardInstance,
}

/// How copied item ids are named. Ids are `<board>/<local>` or `matrix/<board>-<cluster>/...`:
/// every whole `<source board>` token is replaced by the mirror board id, and an id that names
/// no such token is prefixed with `<mirror board>/`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum MirroredIdRule {
    BoardPrefix,
}

/// The states an exception can take. A linked item needs no exception.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum MirroredExceptionState {
    Local,
    Omitted,
}

/// Requested exception state for a half. `Linked` removes any stored exception overlay.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum HalfExceptionState {
    Local,
    Omitted,
    Linked,
}

/// The rigid map from the first member's Case frame to the second's.
///
/// The map is a `Pose2` composed with a fixed X reflection:
/// `q = R(rotation) * (-p.x, p.y) + at`. `rotation` uses the project's degree
/// convention, and the exact inverse maps the second member back to the first.
/// The shared construction subset of a mechanical pair.
///
/// Board identity/thickness, batteries, part profiles, stabilizer overrides and
/// hardware-specific fits/specifications stay local on each instance.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MechanicalConstruction {
    pub method: PlateMethod,
    pub mount: MechanicalMount,
    pub integrated_plate_frame: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bottom_style: Option<MechanicalBottomStyle>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub middle_frame: Option<bool>,
    pub plate_thickness: f64,
    pub plate_foam_thickness: f64,
    pub bottom_foam_thickness: f64,
    pub bottom_thickness: f64,
    pub plate_to_pcb: f64,
    pub wall_thickness: f64,
    pub clearance: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gasket: Option<Gasket>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gasket_travel: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub opening_allowance: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub part_processes: Option<Vec<MechanicalPartProcess>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub internal_gasket: Option<InternalGasketConfiguration>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gasket_layout: Option<MechanicalGasketLayout>,
}

/// One linked mechanical feature of a pair.
///
/// The shared value is stored once with its authoring member; counterpart
/// geometry is derived in Core rather than persisted as a second value.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MechanicalFeature {
    pub id: String,
    pub kind: MechanicalFeatureKind,
    /// The instance that authored the shared value.
    pub author: String,
    pub value: MechanicalFeatureValue,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub members: Vec<MechanicalFeatureMember>,
}

/// One member's identity and state for a linked feature.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MechanicalFeatureMember {
    pub instance_id: String,
    pub feature_id: String,
    pub state: MechanicalFeatureState,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum MechanicalFeatureKind {
    Mount,
    ClosureMount,
    Opening,
    GasketSupport,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum MechanicalFeatureState {
    Linked,
    Local,
    Omitted,
}

/// The single authored value of a linked feature.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum MechanicalFeatureValue {
    Mount { mount: Mount },
    Opening { opening: MechanicalOpening },
    GasketSupport { anchor: MechanicalGasketAnchor },
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum MatrixSplayAffect {
    Column,
    Following,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum MatrixSplayChange {
    Origin {
        world: Option<Vec2>,
    },
    /// Ctrl is captured by the caller at pointer-down. This intent never compensates steps.
    OriginSwing {
        world: Option<Vec2>,
    },
    /// Fields and nudges edit the stored relative origin and always hold keys still.
    OriginRelative {
        value: Vec2,
    },
    Angle {
        angle: f64,
        affect: MatrixSplayAffect,
    },
}

/// A named matrix parameter change, applied to one column or row. Values are
/// absolute and in matrix coordinates, matching the Inspector fields.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum MatrixPlacementChange {
    ColumnSpread {
        column: u32,
        value: Option<f64>,
    },
    RowPadding {
        row: u32,
        value: Option<f64>,
    },
    RowBend {
        row: u32,
        value: f64,
    },
    KeyShift {
        row: u32,
        column: u32,
        value: Vec2,
    },
    KeyBend {
        row: u32,
        column: u32,
        value: f64,
    },
    RowShift {
        row: u32,
        value: Vec2,
    },
    MatrixTranslation {
        delta: Vec2,
    },
    ColumnTranslation {
        column: u32,
        delta: Vec2,
    },
    ColumnStagger {
        column: u32,
        value: f64,
    },
    ColumnOffset {
        column: u32,
        value: Vec2,
    },
    RowOffset {
        row: u32,
        value: Vec2,
    },
    /// One-shot: move the column's offset Y into its stagger and compensate the next
    /// column, so every pose is unchanged. See [`convert_offset_available`].
    ColumnOffsetToStagger {
        column: u32,
    },
}

/// Why `ColumnOffsetToStagger` is refused for this column, or `Ok` when the
/// conversion keeps every pose. The stagger moves a default-origin splay pivot,
/// so a splayed column with a default origin would move. One source for Core and
/// the Inspector.
pub fn convert_offset_available(matrix: &Matrix, column: u32) -> Result<(), String> {
    if column >= matrix.columns {
        return Err("Matrix placement column is out of bounds".into());
    }
    let values = matrix
        .column_layout
        .get(column as usize)
        .copied()
        .unwrap_or_default();
    if values.splay != 0.0
        || values.rotation != 0.0
        || matrix.row_layout.iter().any(|r| r.bend != 0.0)
        || matrix
            .cells
            .iter()
            .any(|c| c.column == column && c.bend.unwrap_or_default() != 0.0)
    {
        return Err("Converting requires zero Splay, Column rotation and carried Bend".into());
    }
    Ok(())
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum EditOperation {
    SetModuleDefinition {
        definition: ModuleDefinition,
    },
    SetMountedModule {
        instance: Box<MountedModule>,
        #[serde(default)]
        definition: Option<Box<ModuleDefinition>>,
        /// Source-backed VIK host connector to place atomically with a new connection.
        #[serde(default, rename = "hostConnectorDefinition")]
        host_connector_definition: Option<Box<PartDefinition>>,
    },
    RemoveMountedModule {
        id: String,
    },
    EmbedModuleCircuit {
        id: String,
        definition: ModuleDefinition,
        #[serde(rename = "hostBoardId")]
        host_board_id: String,
        pose: Pose2,
        side: Side,
        #[serde(default)]
        joins: BTreeMap<String, String>,
    },
    RemoveEmbeddedCircuit {
        id: String,
    },
    SetInputScanMode {
        #[serde(rename = "partId")]
        part_id: String,
        mode: PressScanMode,
    },
    /// Set a board's electrical mode. This changes no component placement or
    /// outline input, so Core can retain its current resolved outline geometry.
    SetWiringMode {
        #[serde(rename = "boardId")]
        board_id: String,
        mode: crate::electrical::ElectricalMode,
    },
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
    /// One typed keycap-size resize. Core validates every entry before writing
    /// anything, so a keycap resize commits the typed sizes, the reflow poses
    /// and the cell offsets atomically with one Undo entry instead of a
    /// whole-document replacement.
    SetKeycapSize {
        /// part_id to its new physical keycap size in mm.
        sizes: BTreeMap<String, Vec2>,
        /// Absolute pose updates from the key-size reflow.
        moves: Vec<Position>,
        /// Cell offset updates from the key-size reflow.
        offsets: Vec<KeycapCellOffset>,
    },
    MoveParts {
        positions: Vec<Position>,
    },
    AddPart {
        part: Part,
        #[serde(rename = "boardId", skip_serializing_if = "Option::is_none")]
        board_id: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        definition: Option<PartDefinition>,
        #[serde(rename = "layoutId", default, skip_serializing_if = "Option::is_none")]
        layout_id: Option<String>,
    },
    AddMirrorOnlyPart {
        part: Part,
        #[serde(rename = "boardId")]
        board_id: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        definition: Option<PartDefinition>,
        #[serde(rename = "layoutId", default, skip_serializing_if = "Option::is_none")]
        layout_id: Option<String>,
    },
    RemoveMatrix {
        id: String,
    },
    SetMatrixCellsEnabled {
        cells: BTreeMap<String, Vec<(u32, u32)>>,
        enabled: bool,
    },
    RemoveSelectedMatrixCells {
        cells: BTreeMap<String, Vec<(u32, u32)>>,
    },
    RemoveMatrixCells {
        #[serde(rename = "matrixId")]
        matrix_id: String,
        cells: Vec<(u32, u32)>,
    },
    RemoveMatrixRow {
        #[serde(rename = "matrixId")]
        matrix_id: String,
        row: u32,
    },
    RemoveMatrixColumn {
        #[serde(rename = "matrixId")]
        matrix_id: String,
        column: u32,
    },
    RemoveParts {
        ids: Vec<String>,
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
    /// Edits one named matrix placement parameter. Core owns the parameter rule,
    /// so the Inspector value and an equivalent gesture submit identical intents.
    SetMatrixPlacement {
        #[serde(rename = "matrixId")]
        matrix_id: String,
        change: MatrixPlacementChange,
    },
    /// Creates a 1x3 matrix anchored to one key of another matrix. Core assigns
    /// the anchor target's board, joins its Layout, uses Origin (0, -pitch.y)
    /// with all Follow components and materializes members in one Undo entry.
    AddAnchoredMatrix {
        id: String,
        anchor: MatrixAnchor,
    },
    /// Re-targets one matrix's anchor and Follow subset in one Undo entry.
    SetMatrixAnchor {
        #[serde(rename = "matrixId")]
        matrix_id: String,
        anchor: MatrixAnchor,
    },
    /// Clears one matrix's anchor after writing its current effective world
    /// frame to Origin/Rotation, so every pose is unchanged. Linked groups
    /// detach together; a detach that would need a new Local exception is
    /// refused and names it instead of silently changing one side.
    DetachMatrixAnchor {
        #[serde(rename = "matrixId")]
        matrix_id: String,
    },
    /// Applies a confirmed linked detach that needs one additional Local
    /// matrix exception: clears both anchors, keeps both world poses, and
    /// records the named Local exception in the same Undo entry. The named
    /// matrix must be the linked partner needing the exception; anything
    /// else refuses as a stale confirmation and changes nothing.
    DetachMatrixAnchorConfirmed {
        #[serde(rename = "matrixId")]
        matrix_id: String,
        #[serde(rename = "localMatrixId")]
        local_matrix_id: String,
    },
    SetConstraint {
        constraint: Constraint,
    },
    /// Re-expresses one offset constraint on the other axes without moving any part. `to`
    /// names the kind it becomes. See ADR-0011.
    ConvertOffsetConstraint {
        #[serde(rename = "constraintId")]
        constraint_id: String,
        to: OffsetAxes,
    },
    /// Applies one typed mechanical settings/feature change. Core owns defaults,
    /// validation, linked-feature propagation and closure-clearance projection,
    /// so the settings Inspector and an equivalent adapter submit the same intent.
    ApplyMechanicalEdit {
        edit: Box<crate::mechanical::change::MechanicalEdit>,
    },
    /// Applies one typed outline change. Core owns target eligibility, validation,
    /// identity allocation from the captured creation seed and mutation, so the
    /// outline adapter submits named changes instead of a prepared replacement
    /// document.
    ApplyOutlineEdit {
        edit: Box<crate::outline_edit::OutlineEdit>,
    },
    CreateMirroredPair {
        left: Layout,
        right: Layout,
        matrix: Box<Matrix>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        definitions: Option<Vec<PartDefinition>>,
    },
    /// Creates the second half of a mechanical pair from its source instance (ADR-0010): a new
    /// board that receives a reflection of the source board through `mapping`, or a flipped
    /// instance of the same board. The pair, its construction source and its mirrored-half
    /// record are created in the same accepted edit. Every new id must be fresh.
    CreateMirroredHalf {
        #[serde(rename = "pairId")]
        pair_id: String,
        #[serde(rename = "sourceInstanceId")]
        source_instance_id: String,
        mode: MirroredHalfMode,
        /// Optional source and mirror labels, validated and applied atomically with creation.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        labels: Option<MirroredHalfLabels>,
        /// Optional physical setup committed in the same edit as pair creation.
        #[serde(
            default,
            rename = "physicalSetup",
            skip_serializing_if = "Option::is_none"
        )]
        physical_setup: Option<MirroredHalfPhysicalSetup>,
        /// Default switch-companion placement for the created pair.
        #[serde(
            default,
            rename = "companionMode",
            skip_serializing_if = "MirroredCompanionMode::is_literal_mirror"
        )]
        companion_mode: MirroredCompanionMode,
        /// Initial adjustment copied into every companion's correspondence (never a pair-wide tweak).
        #[serde(
            default,
            rename = "companionTweak",
            skip_serializing_if = "Option::is_none"
        )]
        companion_tweak: Option<MirroredCompanionTweak>,
        mapping: Pose2,
        #[serde(rename = "idRule")]
        id_rule: MirroredIdRule,
        /// The normalized mounting-hole definition, needed when the source has closure mounts:
        /// the second half's clearance holes are generated from them, as for `LinkMirrorPair`.
        #[serde(
            default,
            rename = "mountingHole",
            skip_serializing_if = "Option::is_none"
        )]
        mounting_hole: Option<Box<PartDefinition>>,
    },
    SetHalfException {
        #[serde(rename = "pairId")]
        pair_id: String,
        // EditOperation is internally tagged by `kind`, so the item category uses itemKind.
        #[serde(rename = "itemKind")]
        kind: MirroredItemKind,
        #[serde(rename = "mirrorId")]
        mirror_id: String,
        state: HalfExceptionState,
    },
    SetCompanionPlacement {
        #[serde(rename = "pairId")]
        pair_id: String,
        targets: Vec<CompanionPlacementTarget>,
        mode: MirroredCompanionMode,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        tweak: Option<MirroredCompanionTweak>,
    },
    UnlinkHalves {
        #[serde(rename = "pairId")]
        pair_id: String,
    },
    SetLayout {
        layout: Layout,
    },
    /// Sets the stored Split gap of a mirrored pair in one Undo entry. The
    /// halves move apart symmetrically about the split axis; matrix origins
    /// keep their half-frame values and the partner still reflects exactly.
    SetSplitGap {
        #[serde(rename = "layoutId")]
        layout_id: String,
        #[serde(rename = "splitGap")]
        split_gap: f64,
    },
    /// Mirrors an independent part on both halves of a split pair, or makes
    /// it local to its own half, in one Undo entry. Mirroring records a Part
    /// correspondence on the mirror link and materializes the partner at the
    /// reflected half-frame pose; making it local removes the partner. Either
    /// copy edits both while the link lasts.
    SetMirroredPart {
        #[serde(rename = "partId")]
        part_id: String,
        mirrored: bool,
    },
    /// Ticket 13, retarget rule 04: retarget an independent part's anchor
    /// key (Pick) to a new target and Follow subset. Core keeps the stored
    /// carried (Spread/Stagger/Splay) and local (Offset/Rotation) values, so
    /// the part moves to the fresh placement from the new anchor key, in one
    /// Undo entry.
    SetPartAnchor {
        #[serde(rename = "partId")]
        part_id: String,
        anchor: PartAnchor,
    },
    /// Ticket 13: change the Follow subset an anchored part follows (the
    /// anchor key stays). Core re-solves the carried and local values so the
    /// evaluated pose is unchanged, in one Undo entry.
    SetPartAnchorFollow {
        #[serde(rename = "partId")]
        part_id: String,
        follow: Follow,
    },
    /// Ticket 13: clear an independent part's anchor (Detach), keeping its
    /// evaluated pose from the half origin, in one Undo entry.
    DetachPartAnchor {
        #[serde(rename = "partId")]
        part_id: String,
    },
    /// Ticket 13: edit one carried (Spread/Stagger/Splay) or local
    /// (Offset/Rotation) value of an anchored part, in one Undo entry.
    SetPartAnchorValues {
        #[serde(rename = "partId")]
        part_id: String,
        change: PartAnchorValueChange,
    },
    RemoveConstraint {
        id: String,
    },
    ReplaceDocument {
        document: Box<ProjectDoc>,
    },
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct OutlineContourEdit {
    pub contour: u32,
    pub points: Vec<Vec2>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Position {
    pub id: String,
    pub at: Vec2,
}
/// One matrix cell offset update carried by a typed keycap-size resize.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KeycapCellOffset {
    pub matrix_id: String,
    pub row: u32,
    pub column: u32,
    pub offset: Option<Vec2>,
}
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EditPhase {
    Preview,
    Commit,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
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
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Error,
    Warning,
    Info,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Scope {
    Layout,
    Outline,
    Pcb,
    Case,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Finding {
    pub id: String,
    pub severity: Severity,
    pub scope: Scope,
    pub message: String,
    #[serde(rename = "targetIds")]
    pub target_ids: Vec<String>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Readiness {
    pub layout: bool,
    pub outline: bool,
    pub pcb: bool,
    #[serde(rename = "case")]
    pub case_ready: bool,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Transform {
    pub id: String,
    pub pose: Pose2,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BoardContours {
    #[serde(rename = "boardId")]
    pub board_id: String,
    pub contours: Vec<Contour>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BoardReadiness {
    #[serde(rename = "boardId")]
    pub board_id: String,
    pub outline: bool,
    pub pcb: bool,
    #[serde(rename = "case")]
    pub case_ready: bool,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SceneDelta {
    #[serde(
        rename = "moduleScenes",
        default,
        skip_serializing_if = "Vec::is_empty"
    )]
    pub module_scenes: Vec<ResolvedModule>,
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
    pub board_outline_scenes: Vec<BoardOutlineScene>,
    #[serde(
        rename = "findingMarkers",
        default,
        skip_serializing_if = "Vec::is_empty"
    )]
    pub finding_markers: Vec<FindingMarker>,
    pub findings: Vec<Finding>,
    pub readiness: Readiness,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MatrixScene {
    #[serde(rename = "matrixId")]
    pub matrix_id: String,
    pub cells: Vec<MatrixSceneCell>,
    pub columns: Vec<MatrixColumnBasis>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub companions: Vec<MatrixCompanionPlacement>,
}

/// How Core places a generated companion relative to its matrix key. The mode is
/// derived from the same rules the placement solver uses, so presentation never
/// re-derives precedence. `Constrained` names the explicit constraint that wins.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MatrixCompanionMode {
    Follows,
    Authored,
    Constrained {
        #[serde(rename = "constraintId")]
        constraint_id: String,
        #[serde(rename = "sourcePartId")]
        source_part_id: String,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MatrixCompanionPlacement {
    #[serde(rename = "partId")]
    pub part_id: String,
    #[serde(rename = "keyId")]
    pub key_id: String,
    pub mode: MatrixCompanionMode,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MatrixSceneCell {
    pub row: u32,
    pub column: u32,
    pub enabled: bool,
    #[serde(rename = "memberId", skip_serializing_if = "Option::is_none")]
    pub member_id: Option<String>,
    pub pose: Pose2,
    #[serde(rename = "shiftFrame")]
    pub shift_frame: crate::matrix::kernel::ComposedFrame,
    #[serde(rename = "offsetFrame")]
    pub offset_frame: crate::matrix::kernel::ComposedFrame,
    /// Carried row/key contributions from the layout kernel, in stored axes.
    /// Relations reads these instead of recomputing layout maths.
    #[serde(rename = "rowShift", default, skip_serializing_if = "is_zero_vec2")]
    pub row_shift: Vec2,
    #[serde(rename = "keyShift", default, skip_serializing_if = "is_zero_vec2")]
    pub key_shift: Vec2,
    #[serde(rename = "rowBend", default, skip_serializing_if = "is_zero_f64")]
    pub row_bend: f64,
    #[serde(rename = "keyBend", default, skip_serializing_if = "is_zero_f64")]
    pub key_bend: f64,
    /// Accumulated carried translation in unreflected matrix axes: each shift
    /// transported through the running carried heading, not a raw sum.
    #[serde(
        rename = "cumulativeShift",
        default,
        skip_serializing_if = "is_zero_vec2"
    )]
    pub cumulative_shift: Vec2,
    /// Accumulated carried row/key bend in degrees; excludes Splay, Column
    /// rotation, Key rotation and matrix/anchor rotation.
    #[serde(
        rename = "cumulativeBend",
        default,
        skip_serializing_if = "is_zero_f64"
    )]
    pub cumulative_bend: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MatrixColumnBasis {
    pub column: u32,
    pub base: Vec2,
    pub rotation: f64,
    #[serde(rename = "relativeOrigin")]
    pub relative_origin: Vec2,
    #[serde(rename = "spreadAxis", skip_serializing_if = "Option::is_none")]
    pub spread_axis: Option<Vec2>,
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
    /// World direction this column's keys move per +1 mm of its column stagger.
    #[serde(rename = "staggerAxis")]
    pub stagger_axis: Vec2,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum CoreRequest {
    #[serde(rename = "resolve-modules")]
    ResolveModules {
        id: String,
        document: ProjectDoc,
        #[serde(rename = "boardId")]
        board_id: String,
        #[serde(
            default,
            rename = "previewTopZ",
            skip_serializing_if = "Option::is_none"
        )]
        preview_top_z: Option<f64>,
    },
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
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum CoreReply {
    #[serde(rename = "modules-resolved")]
    ModulesResolved {
        id: String,
        result: ModuleResolution,
    },
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
        document: Box<ProjectDoc>,
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
#[serde(rename_all = "lowercase")]
pub enum OutlineExportFormat {
    Svg,
    Dxf,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
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
#[serde(rename_all = "camelCase")]
pub struct FootprintGeometry {
    pub side: Side,
    pub courtyard: Vec<Vec2>,
    pub pads: Vec<Pad>,
    pub traces: Vec<LocalTrace>,
    pub vias: Vec<LocalVia>,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ArtifactDiagnosticKind {
    Approximation,
    UnavailableModel,
    ExportUnsupported,
    ParseError,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
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
#[serde(rename_all = "camelCase")]
pub struct CompiledFootprint {
    pub definition: PartDefinition,
    pub geometry: FootprintGeometry,
    pub diagnostics: Vec<ArtifactDiagnostic>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preview_svg: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FootprintCompileJob {
    pub id: String,
    pub definition: PartDefinition,
    pub side: Side,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
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
#[serde(rename_all = "camelCase")]
pub struct FootprintPatch {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub footprint_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reference: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub placement: Option<Pose2>,
    pub side: Side,
    pub pad_nets: BTreeMap<String, (u32, String)>,
    pub uuid_scope: String,
    #[serde(default)]
    pub model_forms: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
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
#[serde(tag = "kind", rename_all = "kebab-case")]
#[serde(rename_all_fields = "camelCase")]
pub enum ExportTarget {
    Board { board_id: String },
    StandaloneFootprints { definition_ids: Vec<String> },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RenderJob {
    pub job_id: String,
    pub definition: PartDefinition,
    pub part: Part,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReservedNet {
    pub name: String,
    pub index: u32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportPlan {
    pub snapshot_token: String,
    pub fingerprint: String,
    pub revision: u64,
    pub target: ExportTarget,
    pub jobs: Vec<RenderJob>,
    pub reserved_nets: Vec<ReservedNet>,
    pub next_net_index: u32,
    pub contours: Vec<Contour>,
    pub captured_document: ProjectDoc,
    pub model_paths: BTreeMap<String, String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RenderedJob {
    pub snapshot_token: String,
    pub revision: u64,
    pub job_id: String,
    pub source: String,
    pub nets: Vec<ReservedNet>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FinishExportRequest {
    pub plan: ExportPlan,
    pub results: Vec<RenderedJob>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArtifactFile {
    pub filename: String,
    pub content: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportArtifact {
    pub snapshot_token: String,
    pub revision: u64,
    pub files: Vec<ArtifactFile>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub skipped_utilities: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OutlineExportRequest {
    pub filename: String,
    pub board: Board,
    pub contours: Vec<Contour>,
    pub format: OutlineExportFormat,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
#[serde(rename_all_fields = "camelCase")]
pub enum ArtifactRequest {
    ReadBoardSource {
        id: String,
        source: String,
        placement: Pose2,
        flipped: bool,
    },
    ImportModuleBoard {
        id: String,
        #[serde(rename = "definitionId")]
        definition_id: String,
        name: String,
        source: String,
        provenance: HardwareSource,
        family: String,
        variant: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        repair: Option<ModuleCircuitRepair>,
    },
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
    /// Plan, render and assemble the PCB preview of one board.
    PreviewPcb {
        id: String,
        request: PrepareExportRequest,
    },
    /// Plan, render and assemble KiCad files for a board or standalone footprints.
    ExportPcb {
        id: String,
        request: PrepareExportRequest,
    },
    ExportOutline {
        id: String,
        request: OutlineExportRequest,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
#[serde(rename_all_fields = "camelCase")]
pub enum ArtifactReply {
    ReadBoardSource {
        id: String,
        result: ModuleBoard,
    },
    ImportModuleBoard {
        id: String,
        result: Box<ModuleDefinition>,
    },
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
        result: Box<CompiledFootprint>,
    },
    ExportPcb {
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
#[serde(rename_all = "kebab-case")]
pub enum MechanicalPurpose {
    ElectricalPcbMountingHole,
    PlateCutout,
    ClearanceEnvelope,
    DrawingGuide,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
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
#[serde(rename_all = "camelCase")]
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
#[serde(rename_all = "camelCase")]
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
#[serde(rename_all = "camelCase")]
pub struct MechanicalGeometry {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_name: Option<String>,
    pub primitives: Vec<MechanicalPrimitive>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
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
#[serde(rename_all = "camelCase")]
pub struct AssemblyDefinition {
    pub id: String,
    pub name: String,
    pub members: Vec<AssemblyMember>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssemblyMember {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parameters: Option<BTreeMap<String, serde_json::Value>>,
    #[serde(rename = "modelMode", default, skip_serializing_if = "Option::is_none")]
    pub model_mode: Option<AssemblyModelMode>,
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub definition_id: Option<String>,
    pub pose: Pose2,
    pub side: Side,
    pub models: Vec<PartModel>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CaseBodyMesh {
    pub id: String,
    pub name: String,
    pub positions: Vec<f32>,
    pub normals: Vec<f32>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AssemblyModelMode {
    Defaults,
    Custom,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
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
#[serde(rename_all = "kebab-case")]
pub enum MechanicalMount {
    Tray,
    Rigid,
    Gasket,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
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
#[serde(rename_all = "camelCase")]
pub struct MechanicalGasketLayout {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auto_size: Option<bool>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub adhesive_thickness: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub minimum_foam_thickness: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preset_id: Option<GasketFoamPreset>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub material: Option<String>,
    pub length: f64,
    pub width: f64,
    pub thickness: f64,
    pub compression: f64,
    #[serde(default)]
    pub supports: Vec<MechanicalGasketAnchor>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MechanicalGasketAnchor {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub length: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub width: Option<f64>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub placement: Option<GasketPlacement>,
    pub id: String,
    pub region_id: String,
    pub outline_key: String,
    pub anchor: f64,
    #[serde(default)]
    pub unlinked: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MechanicalGasketSupport {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fit_error: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
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
#[serde(rename_all = "camelCase")]
pub struct MechanicalGasketTrack {
    pub region_id: String,
    pub start: Vec2,
    pub end: Vec2,
    pub start_anchor: f64,
    pub end_anchor: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
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
    pub openings: Option<Vec<MechanicalOpening>>,
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
    /// Monotonic reservation for authored opening identities.
    ///
    /// Zero means nothing has been reserved yet: the first allocation normalizes to
    /// the highest identity the configuration already carries, so a legacy document
    /// keeps its geometry. Once reserved, an identity is never handed out again, so a
    /// queued intent naming a removed opening can never reach a later addition.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub next_opening_identity: u64,
    /// Monotonic reservations for identities that have no per-entity counter.
    ///
    /// Zero means nothing has been reserved yet: the first allocation normalizes to the
    /// highest identity the configuration already carries. Once reserved, a removed
    /// identity is never handed out again within the document session. Absence needs no
    /// format migration: it is the same unset state as zero, and a successful Open starts
    /// a new session that seeds these counters from the live identities it receives.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub next_critical_fit_identity: u64,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub next_hardware_identity: u64,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub next_mount_identity: u64,
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

/// Serde helper: omit an unset identity reservation so existing documents keep their shape.
fn is_zero(value: &u64) -> bool {
    *value == 0
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MechanicalStackLayer {
    pub id: String,
    pub z: f64,
    pub thickness: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MechanicalAssembly {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub finding_markers: Vec<FindingMarker>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub modules: Vec<ResolvedModule>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub generated_materials: Vec<MechanicalMaterialSpecification>,
    #[serde(default)]
    pub gasket_supports: Vec<MechanicalGasketSupport>,
    #[serde(default)]
    pub gasket_tracks: Vec<MechanicalGasketTrack>,
    #[serde(default)]
    pub generated_hardware: Vec<MechanicalHardwareSpecification>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
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
#[serde(rename_all = "camelCase")]
pub struct MechanicalBattery {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cable_width: Option<f64>,
    pub size: Vec3,
    pub at: Vec2,
    pub cable_exit: Vec2,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum MechanicalBuiltinProfile {
    MxSwitch,
    ChocV1Switch,
    ChocV2Switch,
    MxStab2u,
    MxStab625u,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum MechanicalSwitchFamily {
    Mx,
    ChocV1,
    ChocV2,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MechanicalPartProcess {
    pub part_id: String,
    #[serde(default)]
    pub method: PlateMethod,
    #[serde(default)]
    pub material: String,
    #[serde(default = "unset_mechanical_dimension")]
    pub thickness: f64,
    #[serde(default)]
    pub constraints_version: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum MechanicalStabilizerKind {
    None,
    PcbMount,
    PlateMount,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
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

/// An authored mechanical opening with stable identity for edit targeting.
///
/// Only `MechanicalConfiguration.openings` uses this identified shape; profile and
/// derived geometry keeps plain [`CaseOpening`].
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MechanicalOpening {
    pub id: String,
    pub vertices: Vec<MechanicalOpeningVertex>,
    pub z: f64,
    pub height: f64,
    /// Monotonic reservation for this opening's authored vertex identities, with the
    /// same zero-means-unset rule as [`MechanicalConfiguration::next_opening_identity`].
    /// The address is the opening id together with the vertex id, so a removed vertex
    /// identity is never reused within its opening.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub next_vertex_identity: u64,
}

impl MechanicalOpening {
    /// The authored outline in authoring order.
    pub fn points(&self) -> Vec<Vec2> {
        self.vertices.iter().map(|vertex| vertex.point).collect()
    }

    /// The plain derived shape, for the existing cavity and clearance projection.
    pub fn as_case_opening(&self) -> CaseOpening {
        CaseOpening {
            points: self.points(),
            z: self.z,
            height: self.height,
        }
    }
}

impl From<&MechanicalOpening> for CaseOpening {
    fn from(opening: &MechanicalOpening) -> Self {
        opening.as_case_opening()
    }
}

/// One editable vertex of an authored mechanical opening, addressed by ID rather
/// than by its position in the outline.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MechanicalOpeningVertex {
    pub id: String,
    pub point: Vec2,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MechanicalProfileSource {
    pub text: String,
    pub sha256: String,
    pub mappings: Vec<MechanicalPurposeMapping>,
    pub source_ids: Vec<String>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MechanicalPcbHole {
    pub source_id: String,
    pub at: Vec2,
    pub diameter: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MechanicalBottomStyle {
    Shell,
    Sheet,
}

/// Manufacturing callout metadata; thread geometry is not modeled.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
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
#[serde(rename_all = "kebab-case")]
pub enum GasketConstructionVersion {
    InternalV1,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InternalGasketConfiguration {
    pub version: GasketConstructionVersion,
    #[serde(default = "default_internal_minimum_wall")]
    pub minimum_wall: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub support_clearance: Option<f64>,
    pub tolerance: f64,
    pub support_count: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auto_count: Option<bool>,
    pub hardware: InternalClosureHardware,
}

/// Complete custom geometry. Catalog provenance and process approval are separate from geometry.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
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
#[serde(rename_all = "kebab-case")]
pub enum ScrewLengthDatum {
    UnderHead,
    Overall,
    Unresolved,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ScrewHeadProfile {
    Flat,
    Countersunk,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum GasketPlacement {
    Generated,
    User,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ScrewDrive {
    Hex,
    Torx,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum InsertInstallation {
    HeatSet,
    Tapped,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
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
#[serde(rename_all = "camelCase")]
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
#[serde(rename_all = "camelCase")]
pub struct KeycapResolution {
    pub revision: u64,
    pub specs: Vec<KeycapSpec>,
    pub findings: Vec<Finding>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum KeycapBoardChange {
    Color { value: String },
    LegendColor { value: String },
    Clearance { value: f64 },
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum KeycapMatrixChange {
    Profile { value: Option<KeycapProfile> },
    Mount { value: Option<KeycapMount> },
    FirstRow { value: u8 },
    WallThickness { value: f64 },
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum KeycapKeyChange {
    Profile { value: Option<KeycapProfile> },
    Mount { value: Option<KeycapMount> },
    Legend { value: Option<String> },
    Color { value: Option<String> },
    Row { value: Option<u8> },
    Units { value: Option<Vec2> },
}

#[cfg(test)]
mod constraint_tests {
    use super::*;

    fn held(axes: Option<OffsetAxes>, x: f64, rotation: f64) -> Constraint {
        let (id, source_part_id, target_part_id) = ("c".to_owned(), "a".to_owned(), "b".to_owned());
        match axes {
            Some(axes) => Constraint::offset(
                axes,
                id,
                source_part_id,
                target_part_id,
                Vec2 { x, y: -4.0 },
                rotation,
            ),
            None => Constraint::Mirror {
                id,
                source_part_id,
                target_part_id,
                axis: MirrorAxis::Vertical,
                coordinate: x,
            },
        }
    }

    #[test]
    fn offset_builds_the_kind_its_axes_name_and_carries_every_field() {
        let offset = Vec2 { x: 3.0, y: -4.0 };
        let build =
            |axes| Constraint::offset(axes, "c".into(), "a".into(), "b".into(), offset, 5.0);
        assert_eq!(
            build(OffsetAxes::Board),
            Constraint::Offset {
                id: "c".into(),
                source_part_id: "a".into(),
                target_part_id: "b".into(),
                offset,
                rotation: 5.0,
            }
        );
        assert_eq!(
            build(OffsetAxes::Part),
            Constraint::PartOffset {
                id: "c".into(),
                source_part_id: "a".into(),
                target_part_id: "b".into(),
                offset,
                rotation: 5.0,
            }
        );
    }

    #[test]
    fn offset_axes_and_numbers_read_both_offset_kinds_and_nothing_for_a_mirror() {
        let numbers = Some((Vec2 { x: 1.0, y: -4.0 }, 5.0));
        let board = held(Some(OffsetAxes::Board), 1.0, 5.0);
        let part = held(Some(OffsetAxes::Part), 1.0, 5.0);
        let mirror = held(None, 1.0, 5.0);
        assert_eq!(board.offset_axes(), Some(OffsetAxes::Board));
        assert_eq!(part.offset_axes(), Some(OffsetAxes::Part));
        assert_eq!(mirror.offset_axes(), None);
        assert_eq!(board.offset_numbers(), numbers);
        assert_eq!(part.offset_numbers(), numbers);
        assert_eq!(mirror.offset_numbers(), None);
        assert!(!board.is_mirror() && !part.is_mirror() && mirror.is_mirror());
    }

    #[test]
    fn a_non_finite_number_in_any_kind_is_not_finite() {
        for axes in [Some(OffsetAxes::Board), Some(OffsetAxes::Part), None] {
            assert!(held(axes, 1.0, 5.0).is_finite(), "{axes:?} finite");
            for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
                assert!(!held(axes, bad, 5.0).is_finite(), "{axes:?} x {bad}");
            }
        }
        for axes in [OffsetAxes::Board, OffsetAxes::Part] {
            assert!(
                !held(Some(axes), 1.0, f64::NAN).is_finite(),
                "{axes:?} rotation"
            );
        }
    }
}
