use super::*;

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
    #[serde(
        rename = "rowShift",
        default,
        skip_serializing_if = "crate::model::components::is_zero_vec2"
    )]
    pub row_shift: Vec2,
    #[serde(
        rename = "keyShift",
        default,
        skip_serializing_if = "crate::model::components::is_zero_vec2"
    )]
    pub key_shift: Vec2,
    #[serde(
        rename = "rowBend",
        default,
        skip_serializing_if = "crate::model::components::is_zero_f64"
    )]
    pub row_bend: f64,
    #[serde(
        rename = "keyBend",
        default,
        skip_serializing_if = "crate::model::components::is_zero_f64"
    )]
    pub key_bend: f64,
    /// Accumulated carried translation in unreflected matrix axes: each shift
    /// transported through the running carried heading, not a raw sum.
    #[serde(
        rename = "cumulativeShift",
        default,
        skip_serializing_if = "crate::model::components::is_zero_vec2"
    )]
    pub cumulative_shift: Vec2,
    /// Accumulated carried row/key bend in degrees; excludes Splay, Column
    /// rotation, Key rotation and matrix/anchor rotation.
    #[serde(
        rename = "cumulativeBend",
        default,
        skip_serializing_if = "crate::model::components::is_zero_f64"
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
#[cfg(feature = "wire-json")]
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
#[cfg(feature = "wire-json")]
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

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "wire-json", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "wire-json", serde(rename_all = "camelCase"))]
pub struct FootprintCompileJob {
    pub id: String,
    pub definition: PartDefinition,
    pub side: Side,
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "wire-json", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "wire-json", serde(rename_all = "kebab-case"))]
pub enum ArtifactErrorCode {
    ParseError,
    Validation,
    Unsupported,
    StaleResult,
    MismatchedResults,
    NotFound,
    Internal,
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "wire-json", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "wire-json", serde(rename_all = "camelCase"))]
pub struct ArtifactError {
    pub code: ArtifactErrorCode,
    pub message: String,
    pub diagnostics: Vec<ArtifactDiagnostic>,
}

impl ArtifactError {
    pub fn new(code: ArtifactErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            diagnostics: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "wire-json", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "wire-json", serde(rename_all = "camelCase"))]
pub struct FootprintPatch {
    #[cfg_attr(
        feature = "wire-json",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub footprint_name: Option<String>,
    #[cfg_attr(feature = "wire-json", serde(skip_serializing_if = "Option::is_none"))]
    pub reference: Option<String>,
    #[cfg_attr(feature = "wire-json", serde(skip_serializing_if = "Option::is_none"))]
    pub value: Option<String>,
    #[cfg_attr(feature = "wire-json", serde(skip_serializing_if = "Option::is_none"))]
    pub placement: Option<Pose2>,
    pub side: Side,
    pub pad_nets: BTreeMap<String, (u32, String)>,
    pub uuid_scope: String,
    #[cfg_attr(feature = "wire-json", serde(default))]
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

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "wire-json", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "wire-json", serde(rename_all = "camelCase"))]
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

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "wire-json", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "wire-json", serde(rename_all = "camelCase"))]
pub struct RenderedJob {
    pub snapshot_token: String,
    pub revision: u64,
    pub job_id: String,
    pub source: String,
    pub nets: Vec<ReservedNet>,
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "wire-json", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "wire-json", serde(rename_all = "camelCase"))]
pub struct FinishExportRequest {
    pub plan: ExportPlan,
    pub results: Vec<RenderedJob>,
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "wire-json", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "wire-json", serde(rename_all = "camelCase"))]
pub struct ArtifactFile {
    pub filename: String,
    pub content: String,
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "wire-json", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "wire-json", serde(rename_all = "camelCase"))]
pub struct ExportArtifact {
    pub snapshot_token: String,
    pub revision: u64,
    pub files: Vec<ArtifactFile>,
    #[cfg_attr(
        feature = "wire-json",
        serde(default, skip_serializing_if = "Vec::is_empty")
    )]
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

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "wire-json", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "wire-json", serde(tag = "kind", rename_all = "kebab-case"))]
#[cfg_attr(feature = "wire-json", serde(rename_all_fields = "camelCase"))]
pub enum ArtifactRequest {
    ReadBoardSource {
        id: String,
        source: String,
        placement: Pose2,
        flipped: bool,
    },
    ImportModuleBoard {
        id: String,
        #[cfg_attr(feature = "wire-json", serde(rename = "definitionId"))]
        definition_id: String,
        name: String,
        source: String,
        provenance: HardwareSource,
        family: String,
        variant: String,
        #[cfg_attr(
            feature = "wire-json",
            serde(default, skip_serializing_if = "Option::is_none")
        )]
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

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "wire-json", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "wire-json", serde(tag = "kind", rename_all = "kebab-case"))]
#[cfg_attr(feature = "wire-json", serde(rename_all_fields = "camelCase"))]
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

#[derive(Debug, PartialEq, Serialize, Deserialize)]
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

// Field-wise clone identical to the former derive, kept out of line for the
// same reason as ProjectDoc: configurations are cloned in every core crate.
impl Clone for MechanicalConfiguration {
    #[inline(never)]
    fn clone(&self) -> Self {
        Self {
            internal_gasket: self.internal_gasket.clone(),
            gasket_layout: self.gasket_layout.clone(),
            hardware: self.hardware.clone(),
            critical_fits: self.critical_fits.clone(),
            bottom_style: self.bottom_style.clone(),
            middle_frame: self.middle_frame,
            gasket_travel: self.gasket_travel,
            openings: self.openings.clone(),
            opening_allowance: self.opening_allowance,
            stabilizers: self.stabilizers.clone(),
            part_processes: self.part_processes.clone(),
            gasket: self.gasket.clone(),
            closure_mounts: self.closure_mounts.clone(),
            board_id: self.board_id.clone(),
            next_opening_identity: self.next_opening_identity,
            next_critical_fit_identity: self.next_critical_fit_identity,
            next_hardware_identity: self.next_hardware_identity,
            next_mount_identity: self.next_mount_identity,
            integrated_plate_frame: self.integrated_plate_frame,
            battery: self.battery.clone(),
            mounts: self.mounts.clone(),
            method: self.method.clone(),
            mount: self.mount.clone(),
            plate_thickness: self.plate_thickness,
            plate_foam_thickness: self.plate_foam_thickness,
            pcb_thickness: self.pcb_thickness,
            bottom_foam_thickness: self.bottom_foam_thickness,
            battery_height: self.battery_height,
            bottom_thickness: self.bottom_thickness,
            plate_to_pcb: self.plate_to_pcb,
            wall_thickness: self.wall_thickness,
            clearance: self.clearance,
            profiles: self.profiles.clone(),
        }
    }
}

/// Serde helper: omit an unset identity reservation so existing documents keep their shape.
pub(crate) fn is_zero(value: &u64) -> bool {
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

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "wire-json", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "wire-json", serde(tag = "kind", rename_all = "kebab-case"))]
pub enum KeycapBoardChange {
    Color { value: String },
    LegendColor { value: String },
    Clearance { value: f64 },
}
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "wire-json", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "wire-json", serde(tag = "kind", rename_all = "kebab-case"))]
pub enum KeycapMatrixChange {
    Profile { value: Option<KeycapProfile> },
    Mount { value: Option<KeycapMount> },
    FirstRow { value: u8 },
    WallThickness { value: f64 },
}
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "wire-json", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "wire-json", serde(tag = "kind", rename_all = "kebab-case"))]
pub enum KeycapKeyChange {
    Profile { value: Option<KeycapProfile> },
    Mount { value: Option<KeycapMount> },
    Legend { value: Option<String> },
    Color { value: Option<String> },
    Row { value: Option<u8> },
    Units { value: Option<Vec2> },
}
