use super::*;

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

/// Maximum compressed project archive accepted by Core and browser file admission.
pub const MAX_PROJECT_ARCHIVE_BYTES: usize = 128 * 1024 * 1024;

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
pub(crate) fn is_zero_f64(value: &f64) -> bool {
    *value == 0.0
}
pub(crate) fn is_zero_vec2(value: &Vec2) -> bool {
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
