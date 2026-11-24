//! Typed mechanical change vocabulary and the edit that carries it.
//!
//! The payload names the specific user change; defaults, effective settings,
//! feature ownership and generated clearance parts stay behind the mechanical
//! module's seam. UI field IDs, labels and drafts live in the Case adapter.
use crate::model::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum MechanicalMountCollection {
    Suspension,
    Closure,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum MechanicalDimension {
    PlateThickness,
    PlateFoamThickness,
    PcbThickness,
    BottomFoamThickness,
    BottomThickness,
    WallThickness,
    Clearance,
    GasketSupportLength,
    GasketSupportWidth,
    OpeningAllowance,
    OpeningBottomZ,
    OpeningHeight,
    OpeningPointX,
    OpeningPointY,
    BatteryWidth,
    BatteryDepth,
    BatteryHeight,
    BatteryCableWidth,
    BatteryPositionX,
    BatteryPositionY,
    BatteryCableExitX,
    BatteryCableExitY,
    MountPositionX,
    MountPositionY,
    MountHoleDiameter,
    MountBossDiameter,
    MountBossHeight,
    ClosureThreadDiameter,
    ClosurePitch,
    ClosureHeadDiameter,
    ClosureHeadHeight,
    ClosureHoleDiameter,
    ClosureInsertDiameter,
    ClosureInsertLength,
    ClosureSeatDiameter,
    ClosureSeatDepth,
    ClosureEngagement,
    ClosureThreadStart,
    ClosureTipAllowance,
    ClosureBottomingClearance,
    ClosureRoof,
    ClosureSurround,
    ClosureSeatLeadDepth,
    ClosureSeatLeadDiameter,
    ClosureBearingThickness,
    CriticalFitFromX,
    CriticalFitFromY,
    CriticalFitToX,
    CriticalFitToY,
    HardwareLength,
    HardwareQuantity,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum MechanicalCriticalFitTextField {
    Label,
    Tolerance,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum MechanicalHardwareTextField {
    Designation,
    Thread,
    Tolerance,
    Notes,
}

/// One reviewed match between the two members of a mechanical mirror pair.
///
/// `first_feature_id` names the feature on `members[0]` and `second_feature_id`
/// the matching feature on `members[1]` that the link replaces; both must exist
/// with the given kind. Linking consolidates the two local features into one
/// shared value authored by the reviewed `source_instance_id` (or `members[0]`
/// when absent). The relationship keeps the authored value's id on both halves.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MechanicalFeatureCorrespondence {
    pub kind: MechanicalFeatureKind,
    pub first_feature_id: String,
    pub second_feature_id: String,
    /// The half whose accepted geometry becomes the shared authored value for this
    /// feature. `None` keeps the legacy behaviour of sourcing from `members[0]`.
    /// The choice is per feature and independent of the pair's construction source.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_instance_id: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum MechanicalChange {
    Enable,
    InitializeClosures,
    Disable,
    SetBatteryEnabled(bool),
    SetClosureInsertPreset(String),
    SetClosureDrive(ScrewDrive),
    SetClosureInstallation(InsertInstallation),
    SetClosureFixedLength(Option<f64>),
    SetClosureThread(String),
    SetClosureScrewLengths(Vec<f64>),
    SetClosureHeadProfile(ScrewHeadProfile),
    SetClosureLengthDatum(ScrewLengthDatum),
    AddCriticalFit {
        part_id: String,
    },
    RemoveCriticalFit {
        fit_id: String,
    },
    SetCriticalFitText {
        fit_id: String,
        field: MechanicalCriticalFitTextField,
        value: String,
    },
    SetCriticalFitPart {
        fit_id: String,
        part_id: String,
    },
    SetCriticalFitPoint {
        fit_id: String,
        field: MechanicalDimension,
        value: f64,
    },
    AddHardware {
        part_id: String,
        feature_id: String,
    },
    RemoveHardware {
        hardware_id: String,
    },
    SetHardwareMount {
        hardware_id: String,
        part_id: String,
        feature_id: String,
    },
    SetHardwareText {
        hardware_id: String,
        field: MechanicalHardwareTextField,
        value: String,
    },
    SetHardwareDimension {
        hardware_id: String,
        field: MechanicalDimension,
        value: f64,
    },
    AddOpening,
    RemoveOpening {
        opening_id: String,
    },
    AddOpeningVertex {
        opening_id: String,
    },
    RemoveOpeningVertex {
        opening_id: String,
        vertex_id: String,
    },
    SetOpeningDimension {
        opening_id: String,
        vertex_id: Option<String>,
        field: MechanicalDimension,
        value: f64,
    },
    SetMethod(PlateMethod),
    SetMount(MechanicalMount),
    AddMount {
        collection: MechanicalMountCollection,
    },
    AddSuggestedClosureMounts(Vec<Mount>),
    SetMountCollection {
        collection: MechanicalMountCollection,
        mounts: Vec<Mount>,
    },
    SetMountDimension {
        collection: MechanicalMountCollection,
        mount_id: String,
        field: MechanicalDimension,
        value: f64,
    },
    SetMountPosition {
        collection: MechanicalMountCollection,
        mount_id: String,
        at: Vec2,
    },
    SetMountKind {
        collection: MechanicalMountCollection,
        mount_id: String,
        kind: MountKind,
    },
    RemoveMount {
        collection: MechanicalMountCollection,
        mount_id: String,
    },
    AdoptClosurePositions(Vec<Mount>),
    SetBottomStyle(MechanicalBottomStyle),
    SetMiddleFrame(bool),
    SetIntegratedPlateFrame(bool),
    SetDimension {
        field: MechanicalDimension,
        value: f64,
    },
    SetGasketSupportDimension {
        support_id: String,
        anchors: Vec<MechanicalGasketAnchor>,
        field: MechanicalDimension,
        value: f64,
    },
    SetGasketSupportPlacement {
        support_id: String,
        anchors: Vec<MechanicalGasketAnchor>,
    },
    SetGasketSupportUnlinked {
        support_id: String,
        pair_id: Option<String>,
        anchors: Vec<MechanicalGasketAnchor>,
    },
    ResetGasketPlacement,
    AssignSwitchProfile {
        definition_id: String,
        family: MechanicalSwitchFamily,
    },
    AssignStabilizerProfile {
        definition_id: String,
        source: MechanicalBuiltinProfile,
    },
    AssignImportedGeometryProfile {
        definition_id: String,
    },
    UpdateProfile(MechanicalPartProfile),
    RemoveProfile {
        definition_id: String,
    },
    SetSwitchFamily {
        definition_id: String,
        family: MechanicalSwitchFamily,
    },
    AddProcessOverride {
        part_id: String,
    },
    RemoveProcessOverride {
        part_id: String,
    },
    SetProcessMethod {
        part_id: String,
        method: PlateMethod,
    },
    SetProcessMaterial {
        part_id: String,
        material: String,
    },
    SetProcessThickness {
        part_id: String,
        thickness: f64,
    },
    SetStabilizerKind(MechanicalStabilizerOverride),
    /// Establishes an explicit mechanical mirror pair between two physical
    /// halves: the shared construction comes from `construction_source`, and
    /// each correspondence becomes one linked feature authored by `members[0]`.
    LinkMirrorPair {
        pair_id: String,
        members: [String; 2],
        mapping: Pose2,
        construction_source: String,
        correspondences: Vec<MechanicalFeatureCorrespondence>,
    },
    /// Dissolves the pair that owns `instance_id`, materializing the pair's
    /// construction and linked geometry locally so nothing moves.
    UnlinkMirrorPair {
        instance_id: String,
    },
    /// Removes one physical half from its explicit mechanical pair. The surviving
    /// half keeps the geometry, construction and hardware it currently shows,
    /// materialized locally, and the pair is cleared. The target must be the
    /// removed half's instance; a board target is malformed.
    RemovePhysicalInstance,
    /// Makes one linked feature local on one half without moving its visible
    /// geometry; the other half and every other feature keep their link.
    MakeFeatureLocal {
        feature_id: String,
        instance_id: String,
    },
    /// Removes one linked feature from one half only (a local omission); the
    /// other half and every other feature keep their link.
    OmitFeatureMember {
        feature_id: String,
        instance_id: String,
    },
    /// Clears one half's local/omitted exception, taking `source_instance_id`'s
    /// geometry as the authored value and re-deriving the counterpart.
    RejoinFeatureMember {
        feature_id: String,
        instance_id: String,
        source_instance_id: String,
    },
}

/// The board or physical half a mechanical edit targets.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum MechanicalTarget {
    /// The canonical project-level mechanical configuration.
    Board {
        #[serde(rename = "boardId")]
        board_id: String,
    },
    /// One physical half's configuration.
    Instance {
        #[serde(rename = "instanceId")]
        instance_id: String,
    },
}

/// One typed mechanical edit: a typed target, a typed change and the optional
/// normalized mounting-hole catalogue definition projection needs.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MechanicalEdit {
    pub target: MechanicalTarget,
    pub change: MechanicalChange,
    /// Supplied whenever projection must create or update a generated
    /// closure-hole footprint. Validated against its declared source identity.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mounting_hole: Option<Box<PartDefinition>>,
}
