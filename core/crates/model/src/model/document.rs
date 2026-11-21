use super::*;

#[derive(Debug, PartialEq, Serialize, Deserialize)]
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
    #[serde(default)]
    pub scripts: Vec<Script>,
    #[serde(default)]
    pub constraints: Vec<Constraint>,
}
fn legacy_format_version() -> u32 {
    crate::migrate::LEGACY_VERSION
}

// Field-wise clone identical to the former derive, kept out of line: every core
// crate clones whole documents for drafts, and an inlined derived clone was
// instantiated once per calling crate (and once per codegen unit) in release
// wasm. One shared body here is measurably smaller with the split graph.
impl Clone for ProjectDoc {
    #[inline(never)]
    fn clone(&self) -> Self {
        Self {
            physical_instance_id: self.physical_instance_id.clone(),
            module_definitions: self.module_definitions.clone(),
            modules: self.modules.clone(),
            embedded_circuits: self.embedded_circuits.clone(),
            keymap: self.keymap.clone(),
            keycaps: self.keycaps.clone(),
            hardware: self.hardware.clone(),
            mechanical: self.mechanical.clone(),
            board_references: self.board_references.clone(),
            assemblies: self.assemblies.clone(),
            format: self.format.clone(),
            format_version: self.format_version,
            id: self.id.clone(),
            name: self.name.clone(),
            revision: self.revision,
            parameters: self.parameters.clone(),
            definitions: self.definitions.clone(),
            parts: self.parts.clone(),
            matrices: self.matrices.clone(),
            layouts: self.layouts.clone(),
            nets: self.nets.clone(),
            outline: self.outline.clone(),
            board_outlines: self.board_outlines.clone(),
            boards: self.boards.clone(),
            case_bodies: self.case_bodies.clone(),
            materials: self.materials.clone(),
            assets: self.assets.clone(),
            scripts: self.scripts.clone(),
            constraints: self.constraints.clone(),
        }
    }
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
    #[serde(default, skip_serializing_if = "crate::model::mechanical::is_zero")]
    pub next_opening_identity: u64,
    #[serde(default, skip_serializing_if = "crate::model::mechanical::is_zero")]
    pub next_vertex_identity: u64,
    #[serde(default, skip_serializing_if = "crate::model::mechanical::is_zero")]
    pub next_critical_fit_identity: u64,
    #[serde(default, skip_serializing_if = "crate::model::mechanical::is_zero")]
    pub next_hardware_identity: u64,
    #[serde(default, skip_serializing_if = "crate::model::mechanical::is_zero")]
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
    /// Ticket 07: set a detached independent part's board rotation in one
    /// Undo entry. Anchored parts keep using `SetPartAnchorValues::Rotation`;
    /// members, companions, locked and constraint-driven parts refuse.
    SetPartRotation {
        #[serde(rename = "partId")]
        part_id: String,
        rotation: f64,
    },
    RemoveConstraint {
        id: String,
    },
    /// Changes only the project name. The name must contain a non-whitespace character.
    RenameProject {
        name: String,
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
