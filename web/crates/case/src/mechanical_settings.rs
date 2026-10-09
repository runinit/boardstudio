//! Private read-only presentation and field intents for the Case mechanical stack.
//! Root owns accepted-state admission and commits every intent through Runtime.
pub use crate::mechanical_feedback::{
    MechanicalSettingsFeedback, MechanicalSettingsFeedbackState, MechanicalSettingsIdentity,
};
use crate::mechanical_pair::{MechanicalPairFeatureRow, MechanicalPairLinkOption, feature_label};
use crate::mechanical_settings_controller::use_bound_mechanical_field;
pub use boardstudio_core::mechanical::change::{
    MechanicalChange as MechanicalSettingsPatch, MechanicalCriticalFitTextField,
    MechanicalDimension, MechanicalHardwareTextField, MechanicalMountCollection,
};
use boardstudio_core::model::{
    CaseOpening, GasketPlacement, HardwareTransport, InsertInstallation, InternalClosureHardware,
    MechanicalBattery, MechanicalBottomStyle, MechanicalBuiltinProfile, MechanicalCriticalFit,
    MechanicalExtraction, MechanicalFeatureState, MechanicalGasketAnchor, MechanicalGeometry,
    MechanicalHardwareSpecification, MechanicalMount, MechanicalOpening, MechanicalPartProcess,
    MechanicalPartProfile, MechanicalPurpose, MechanicalPurposeMapping, MechanicalStabilizerKind,
    MechanicalStabilizerOverride, MechanicalSwitchFamily, Mount, MountKind, PlateMethod, Pose2,
    ScrewDrive, ScrewHeadProfile, ScrewLengthDatum, Severity, Vec2,
};
use dioxus::prelude::*;
use dioxus_web::WebEventExt;
use std::rc::Rc;
use std::{future::Future, pin::Pin};
use wasm_bindgen::JsCast;
use web_sys::HtmlInputElement;

/// Narrow accepted values used by this control group; it is not an editable
/// configuration copy and deliberately excludes every field this ticket leaves alone.
#[derive(Clone, Debug, PartialEq)]
pub struct MechanicalSettingsValues {
    pub board_id: String,
    pub transport: HardwareTransport,
    pub battery: Option<MechanicalBattery>,
    pub suspension_mounts: Vec<Mount>,
    pub closure_mounts: Option<Vec<Mount>>,
    pub method: PlateMethod,
    pub mount: MechanicalMount,
    pub bottom_style: MechanicalBottomStyle,
    pub middle_frame: bool,
    pub integrated_plate_frame: bool,
    pub plate_thickness: f64,
    pub plate_foam_thickness: f64,
    pub pcb_thickness: f64,
    pub bottom_foam_thickness: f64,
    pub bottom_thickness: f64,
    pub wall_thickness: f64,
    pub clearance: f64,
    pub opening_allowance: f64,
    pub openings: Vec<MechanicalOpening>,
    pub internal_gasket: bool,
    pub closure_hardware: Option<InternalClosureHardware>,
    pub hardware: Vec<MechanicalHardwareSpecification>,
    pub critical_fits: Vec<MechanicalCriticalFit>,
    pub plate_to_pcb: f64,
    pub battery_height: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MechanicalProfileChoice {
    pub definition_id: String,
    pub name: String,
    pub source: Option<String>,
    pub family: Option<MechanicalSwitchFamily>,
    pub plate_to_pcb: Option<f64>,
    pub supported_thickness: Option<Vec2>,
    /// True only for a placed switch definition selected by the accepted parent projection.
    pub switch_family_selectable: bool,
    pub profile: MechanicalPartProfile,
    pub kicad_source: Option<String>,
}

/// The first argument is the assigned PartDefinition ID; the mount resolves its accepted KiCad source.
pub type ExtractionFuture = Pin<Box<dyn Future<Output = Result<MechanicalExtraction, String>>>>;

#[derive(Clone)]
pub struct MechanicalProfileExtractionPort(
    pub Rc<dyn Fn(String, Vec<MechanicalPurposeMapping>) -> ExtractionFuture>,
);

impl PartialEq for MechanicalProfileExtractionPort {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct MechanicalProfileTargetChoice {
    pub definition_id: String,
    pub name: String,
    pub switch_profile: bool,
    pub imported_geometry: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MechanicalProcessTarget {
    pub part_id: String,
    pub name: String,
    pub default_thickness: f64,
    pub process: Option<MechanicalPartProcess>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MechanicalStabilizerFit {
    pub part_id: String,
    pub reference: String,
    pub key_units: f64,
    pub default: MechanicalStabilizerOverride,
    pub current: MechanicalStabilizerOverride,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MechanicalLayerRow {
    pub id: String,
    pub label: String,
    pub z: f64,
    pub thickness: f64,
    /// Present only when the displayed completed Case assembly contains a body with this ID.
    pub resolved_body_thickness: Option<f64>,
    /// The row is retained from the last completed same-owner assembly while
    /// the accepted physical inputs have changed.
    pub is_previous: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MechanicalGasketSupportRow {
    pub id: String,
    pub region_id: String,
    pub outline_key: String,
    pub anchor: f64,
    pub pair_id: Option<String>,
    pub length: f64,
    pub width: f64,
    pub unlinked: bool,
    pub fit_error: Option<String>,
    pub is_previous: bool,
}

impl MechanicalGasketSupportRow {
    pub fn saved_anchor(&self) -> MechanicalGasketAnchor {
        MechanicalGasketAnchor {
            id: self.id.clone(),
            region_id: self.region_id.clone(),
            outline_key: self.outline_key.clone(),
            anchor: self.anchor,
            length: Some(self.length),
            width: Some(self.width),
            placement: Some(GasketPlacement::User),
            unlinked: self.unlinked,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct MechanicalFindingRow {
    pub id: String,
    pub severity: Severity,
    pub message: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MechanicalBoardMismatch {
    pub board_id: String,
    pub board_name: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MechanicalFitPart {
    pub id: String,
    pub name: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MechanicalHardwareMount {
    pub part_id: String,
    pub part_name: String,
    pub feature_id: String,
    pub kind: MountKind,
    pub x: f64,
    pub y: f64,
}

#[derive(Clone, Debug, PartialEq)]
enum MechanicalTextIntent {
    ClosureThread,
    ClosureScrewLengths,
    CriticalFit {
        fit_id: String,
        field: MechanicalCriticalFitTextField,
    },
    Hardware {
        hardware_id: String,
        field: MechanicalHardwareTextField,
    },
}

impl MechanicalTextIntent {
    fn patch(&self, value: String) -> Result<MechanicalSettingsPatch, String> {
        Ok(match self {
            Self::ClosureThread => MechanicalSettingsPatch::SetClosureThread(value),
            Self::ClosureScrewLengths => {
                let mut lengths = Vec::new();
                for item in value.split(',') {
                    let length = item
                        .trim()
                        .parse::<f64>()
                        .map_err(|_| "Enter positive lengths separated by commas.".to_owned())?;
                    if !length.is_finite() || length <= 0.0 {
                        return Err("Enter positive lengths separated by commas.".to_owned());
                    }
                    if !lengths.contains(&length) {
                        lengths.push(length);
                    }
                }
                if lengths.is_empty() {
                    return Err("Enter positive lengths separated by commas.".to_owned());
                }
                MechanicalSettingsPatch::SetClosureScrewLengths(lengths)
            }
            Self::CriticalFit { fit_id, field } => MechanicalSettingsPatch::SetCriticalFitText {
                fit_id: fit_id.clone(),
                field: *field,
                value,
            },
            Self::Hardware { hardware_id, field } => MechanicalSettingsPatch::SetHardwareText {
                hardware_id: hardware_id.clone(),
                field: *field,
                value,
            },
        })
    }
}

fn is_dimension_field(field_id: &str) -> bool {
    if field_id.starts_with("opening:") {
        return true;
    }
    if field_id.starts_with("mount:")
        && [
            "mount-position-x",
            "mount-position-y",
            "mount-hole-diameter",
            "mount-boss-diameter",
            "mount-boss-height",
        ]
        .iter()
        .any(|field| field_id.ends_with(field))
    {
        return true;
    }
    if field_id.starts_with("closure-")
        || field_id.starts_with("critical-fit:")
        || field_id.starts_with("hardware:")
    {
        return true;
    }
    [
        MechanicalDimension::PlateThickness,
        MechanicalDimension::PlateFoamThickness,
        MechanicalDimension::PcbThickness,
        MechanicalDimension::BottomFoamThickness,
        MechanicalDimension::BottomThickness,
        MechanicalDimension::WallThickness,
        MechanicalDimension::Clearance,
        MechanicalDimension::OpeningAllowance,
        MechanicalDimension::OpeningBottomZ,
        MechanicalDimension::OpeningHeight,
        MechanicalDimension::OpeningPointX,
        MechanicalDimension::OpeningPointY,
        MechanicalDimension::BatteryWidth,
        MechanicalDimension::BatteryDepth,
        MechanicalDimension::BatteryHeight,
        MechanicalDimension::BatteryCableWidth,
        MechanicalDimension::BatteryPositionX,
        MechanicalDimension::BatteryPositionY,
        MechanicalDimension::BatteryCableExitX,
        MechanicalDimension::BatteryCableExitY,
    ]
    .iter()
    .any(|field| field.field_id() == field_id)
}

pub trait MechanicalDimensionExt {
    fn field_id(self) -> &'static str;
    fn rule(self) -> NumberRule;
    fn step(self) -> &'static str;
}

impl MechanicalDimensionExt for MechanicalDimension {
    fn field_id(self) -> &'static str {
        match self {
            Self::PlateThickness => "plate-thickness",
            Self::PlateFoamThickness => "plate-foam-thickness",
            Self::PcbThickness => "pcb-thickness",
            Self::BottomFoamThickness => "bottom-foam-thickness",
            Self::BottomThickness => "bottom-thickness",
            Self::WallThickness => "wall-thickness",
            Self::Clearance => "clearance",
            Self::GasketSupportLength => "gasket-support-length",
            Self::GasketSupportWidth => "gasket-support-width",
            Self::OpeningAllowance => "opening-allowance",
            Self::OpeningBottomZ => "opening-bottom-z",
            Self::OpeningHeight => "opening-height",
            Self::OpeningPointX => "opening-point-x",
            Self::OpeningPointY => "opening-point-y",
            Self::BatteryWidth => "battery-width",
            Self::BatteryDepth => "battery-depth",
            Self::BatteryHeight => "battery-height",
            Self::BatteryCableWidth => "battery-cable-width",
            Self::BatteryPositionX => "battery-position-x",
            Self::BatteryPositionY => "battery-position-y",
            Self::BatteryCableExitX => "battery-cable-exit-x",
            Self::BatteryCableExitY => "battery-cable-exit-y",
            Self::MountPositionX => "mount-position-x",
            Self::MountPositionY => "mount-position-y",
            Self::MountHoleDiameter => "mount-hole-diameter",
            Self::MountBossDiameter => "mount-boss-diameter",
            Self::MountBossHeight => "mount-boss-height",
            Self::ClosureThreadDiameter => "closure-thread-diameter",
            Self::ClosurePitch => "closure-pitch",
            Self::ClosureHeadDiameter => "closure-head-diameter",
            Self::ClosureHeadHeight => "closure-head-height",
            Self::ClosureHoleDiameter => "closure-hole-diameter",
            Self::ClosureInsertDiameter => "closure-insert-diameter",
            Self::ClosureInsertLength => "closure-insert-length",
            Self::ClosureSeatDiameter => "closure-seat-diameter",
            Self::ClosureSeatDepth => "closure-seat-depth",
            Self::ClosureEngagement => "closure-engagement",
            Self::ClosureThreadStart => "closure-thread-start",
            Self::ClosureTipAllowance => "closure-tip-allowance",
            Self::ClosureBottomingClearance => "closure-bottoming-clearance",
            Self::ClosureRoof => "closure-roof",
            Self::ClosureSurround => "closure-surround",
            Self::ClosureSeatLeadDepth => "closure-seat-lead-depth",
            Self::ClosureSeatLeadDiameter => "closure-seat-lead-diameter",
            Self::ClosureBearingThickness => "closure-bearing-thickness",
            Self::CriticalFitFromX => "critical-fit-from-x",
            Self::CriticalFitFromY => "critical-fit-from-y",
            Self::CriticalFitToX => "critical-fit-to-x",
            Self::CriticalFitToY => "critical-fit-to-y",
            Self::HardwareLength => "hardware-length",
            Self::HardwareQuantity => "hardware-quantity",
        }
    }

    fn rule(self) -> NumberRule {
        match self {
            Self::OpeningAllowance => NumberRule::Bounded(-1, 1),
            Self::OpeningHeight
            | Self::BatteryWidth
            | Self::BatteryDepth
            | Self::BatteryHeight
            | Self::BatteryCableWidth
            | Self::MountHoleDiameter
            | Self::MountBossDiameter
            | Self::MountBossHeight => NumberRule::AtLeastTenth,
            Self::BatteryPositionX
            | Self::OpeningBottomZ
            | Self::OpeningPointX
            | Self::OpeningPointY
            | Self::BatteryPositionY
            | Self::BatteryCableExitX
            | Self::BatteryCableExitY
            | Self::MountPositionX
            | Self::MountPositionY
            | Self::CriticalFitFromX
            | Self::CriticalFitFromY
            | Self::CriticalFitToX
            | Self::CriticalFitToY => NumberRule::Coordinate,
            Self::HardwareLength => NumberRule::AtLeastTenth,
            Self::HardwareQuantity => NumberRule::AtLeastOne,
            Self::GasketSupportLength => NumberRule::AtLeastFive,
            Self::GasketSupportWidth => NumberRule::AtLeastHalf,
            _ => NumberRule::Nonnegative,
        }
    }

    fn step(self) -> &'static str {
        match self {
            Self::GasketSupportLength => "5",
            Self::GasketSupportWidth => "0.5",
            Self::HardwareQuantity => "1",
            _ => "0.1",
        }
    }
}

pub trait MechanicalSettingsPatchExt {
    fn field_id(&self) -> String;
}

impl MechanicalSettingsPatchExt for MechanicalSettingsPatch {
    fn field_id(&self) -> String {
        match self {
            Self::Enable => "configure".to_owned(),
            Self::InitializeClosures => "initialize-closures".to_owned(),
            Self::Disable => "disable".to_owned(),
            Self::SetBatteryEnabled(_) => "battery-enabled".to_owned(),
            Self::SetClosureInsertPreset(id) => format!("closure-insert:{id}"),
            Self::SetClosureDrive(_) => "closure-drive".to_owned(),
            Self::SetClosureInstallation(_) => "closure-installation".to_owned(),
            Self::SetClosureFixedLength(_) => "closure-fixed-length".to_owned(),
            Self::SetClosureThread(_) => "closure-thread".to_owned(),
            Self::SetClosureScrewLengths(_) => "closure-screw-lengths".to_owned(),
            Self::SetClosureHeadProfile(_) => "closure-head-profile".to_owned(),
            Self::SetClosureLengthDatum(_) => "closure-length-datum".to_owned(),
            Self::AddCriticalFit { .. } => "critical-fit:add".to_owned(),
            Self::RemoveCriticalFit { fit_id } => format!("critical-fit:{fit_id}:remove"),
            Self::SetCriticalFitText { fit_id, field, .. } => format!(
                "critical-fit:{fit_id}:{}",
                match field {
                    MechanicalCriticalFitTextField::Label => "label",
                    MechanicalCriticalFitTextField::Tolerance => "tolerance",
                }
            ),
            Self::SetCriticalFitPart { fit_id, .. } => format!("critical-fit:{fit_id}:part"),
            Self::SetCriticalFitPoint { fit_id, field, .. } => {
                format!("critical-fit:{fit_id}:{}", field.field_id())
            }
            Self::AddHardware { .. } => "hardware:add".to_owned(),
            Self::RemoveHardware { hardware_id } => format!("hardware:{hardware_id}:remove"),
            Self::SetHardwareMount { hardware_id, .. } => {
                format!("hardware:{hardware_id}:mount")
            }
            Self::SetHardwareText {
                hardware_id, field, ..
            } => format!(
                "hardware:{hardware_id}:{}",
                match field {
                    MechanicalHardwareTextField::Designation => "designation",
                    MechanicalHardwareTextField::Thread => "thread",
                    MechanicalHardwareTextField::Tolerance => "tolerance",
                    MechanicalHardwareTextField::Notes => "notes",
                }
            ),
            Self::SetHardwareDimension {
                hardware_id, field, ..
            } => format!("hardware:{hardware_id}:{}", field.field_id()),
            Self::AddOpening
            | Self::RemoveOpening { .. }
            | Self::AddOpeningVertex { .. }
            | Self::RemoveOpeningVertex { .. } => "case-openings".to_owned(),
            Self::SetOpeningDimension {
                opening_id,
                vertex_id,
                field,
                ..
            } => format!(
                "opening:{opening_id}:{}:{}",
                vertex_id.as_deref().map_or_else(
                    || "volume".to_owned(),
                    |vertex_id| format!("point:{vertex_id}")
                ),
                field.field_id(),
            ),
            Self::SetMethod(_) => "method".to_owned(),
            Self::SetMount(_) => "mount".to_owned(),
            Self::AddSuggestedClosureMounts(_) => "closure-mounts".to_owned(),
            Self::AddMount { collection } | Self::SetMountCollection { collection, .. } => {
                match collection {
                    MechanicalMountCollection::Suspension => "suspension-mounts".to_owned(),
                    MechanicalMountCollection::Closure => "closure-mounts".to_owned(),
                }
            }
            Self::SetMountDimension {
                collection,
                mount_id,
                field,
                ..
            } => format!(
                "mount:{}:{mount_id}:{}",
                mount_collection_id(*collection),
                field.field_id(),
            ),
            Self::SetMountPosition {
                collection,
                mount_id,
                ..
            } => format!(
                "mount:{}:{mount_id}:position",
                mount_collection_id(*collection),
            ),
            Self::SetMountKind {
                collection,
                mount_id,
                ..
            } => format!("mount:{}:{mount_id}:kind", mount_collection_id(*collection),),
            Self::RemoveMount {
                collection,
                mount_id,
            } => format!(
                "mount:{}:{mount_id}:remove",
                mount_collection_id(*collection),
            ),
            Self::AdoptClosurePositions(_) => "adopt-closure-positions".to_owned(),
            Self::SetBottomStyle(_) => "bottom-style".to_owned(),
            Self::SetMiddleFrame(_) => "middle-frame".to_owned(),
            Self::SetIntegratedPlateFrame(_) => "integrated-plate-frame".to_owned(),
            Self::SetDimension { field, .. } => field.field_id().to_owned(),
            Self::SetGasketSupportDimension {
                support_id, field, ..
            } => format!(
                "gasket-support:{support_id}:{}",
                match *field {
                    MechanicalDimension::GasketSupportLength => "length",
                    MechanicalDimension::GasketSupportWidth => "width",
                    _ => "dimension",
                }
            ),
            Self::SetGasketSupportUnlinked { support_id, .. } => {
                format!("gasket-support:{support_id}:link")
            }
            Self::SetGasketSupportPlacement { support_id, .. } => {
                format!("gasket-support:{support_id}:placement")
            }
            Self::ResetGasketPlacement => "reset-gasket-placement".to_owned(),
            Self::AssignSwitchProfile { definition_id, .. } => {
                format!("assign-switch-profile:{definition_id}")
            }
            Self::AssignStabilizerProfile { definition_id, .. } => {
                format!("assign-stabilizer-profile:{definition_id}")
            }
            Self::AssignImportedGeometryProfile { definition_id } => {
                format!("assign-imported-profile:{definition_id}")
            }
            Self::UpdateProfile(profile) => format!("profile:{}:update", profile.definition_id),
            Self::RemoveProfile { definition_id } => format!("profile:{definition_id}:remove"),
            Self::SetSwitchFamily { definition_id, .. } => {
                format!("switch-family:{definition_id}")
            }
            Self::AddProcessOverride { part_id } => format!("process:{part_id}:add"),
            Self::RemoveProcessOverride { part_id } => format!("process:{part_id}:remove"),
            Self::SetProcessMethod { part_id, .. } => format!("process:{part_id}:method"),
            Self::SetProcessMaterial { part_id, .. } => format!("process:{part_id}:material"),
            Self::SetProcessThickness { part_id, .. } => format!("process:{part_id}:thickness"),
            Self::SetStabilizerKind(stabilizer) => {
                format!("stabilizer:{}", stabilizer.part_id)
            }
            // The pair lifecycle is applied through the Core mechanical
            // operation; these field ids are the adapter's stable keys for it.
            Self::LinkMirrorPair { pair_id, .. } => format!("mechanical-pair:{pair_id}:link"),
            Self::UnlinkMirrorPair { instance_id } => {
                format!("mechanical-pair:{instance_id}:unlink")
            }
            Self::MakeFeatureLocal {
                feature_id,
                instance_id,
            } => format!("mechanical-feature:{instance_id}:{feature_id}:make-local"),
            Self::OmitFeatureMember {
                feature_id,
                instance_id,
            } => format!("mechanical-feature:{instance_id}:{feature_id}:omit"),
            Self::RejoinFeatureMember {
                feature_id,
                instance_id,
                ..
            } => format!("mechanical-feature:{instance_id}:{feature_id}:rejoin"),
            // The removal names its member through the edit target, so this key is the
            // panel's stable identity for the selected half's removal action.
            Self::RemovePhysicalInstance => "remove-physical-instance".to_owned(),
            // The conversion names exactly the members it converts, so its key follows
            // them and a per-half action cannot share another half's pending gate.
            Self::MaterializeLegacyConstruction { instance_ids } => {
                format!("materialize-legacy-construction:{}", instance_ids.join(","))
            }
        }
    }
}

fn mount_collection_id(collection: MechanicalMountCollection) -> &'static str {
    match collection {
        MechanicalMountCollection::Suspension => "suspension",
        MechanicalMountCollection::Closure => "closure",
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct MechanicalSettingsRequest {
    pub identity: MechanicalSettingsIdentity,
    pub request_id: u64,
    pub field_id: String,
    pub patch: MechanicalSettingsPatch,
    /// The fingerprint of the accepted inputs the reviewed result was produced from,
    /// for actions whose submission previews its result. The controller compares it
    /// against the accepted document when Session executes the submission, so a
    /// relevant change between the displayed review and its execution retires the
    /// action and refreshes the review. `None` for actions without a reviewed result.
    pub reviewed_fingerprint: Option<String>,
}

#[derive(Props, Clone, PartialEq)]
pub struct MechanicalSettingsProps {
    pub identity: MechanicalSettingsIdentity,
    /// Editor-lifetime sequence shared with the page owner, so an unmount/remount cannot reuse
    /// request IDs while retaining the same editor identity.
    pub request_sequence: Signal<u64>,
    pub values: Option<MechanicalSettingsValues>,
    pub profiles: Rc<[MechanicalProfileChoice]>,
    #[props(default)]
    pub extract_profile: Option<MechanicalProfileExtractionPort>,
    pub profile_targets: Rc<[MechanicalProfileTargetChoice]>,
    pub process_targets: Rc<[MechanicalProcessTarget]>,
    pub stabilizer_fits: Rc<[MechanicalStabilizerFit]>,
    pub layers: Rc<[MechanicalLayerRow]>,
    pub gasket_supports: Rc<[MechanicalGasketSupportRow]>,
    pub fit_parts: Rc<[MechanicalFitPart]>,
    pub fit_parts_resolved: bool,
    pub hardware_mounts: Rc<[MechanicalHardwareMount]>,
    pub suggested_mounts: Rc<[Mount]>,
    pub findings: Rc<[MechanicalFindingRow]>,
    /// The selected half's pair features, projected from the accepted pair store.
    /// Absent when the half belongs to no pair.
    #[props(default)]
    pub pair_features: Option<Rc<[MechanicalPairFeatureRow]>>,
    /// The candidate links an unpaired half can establish. Absent when there is
    /// nothing to link with.
    #[props(default)]
    pub pair_link_options: Option<Rc<[MechanicalPairLinkOption]>>,
    /// The other half when the selected half belongs to a mechanical pair.
    #[props(default)]
    pub pair_counterpart: Option<String>,
    /// Whether the selected half still uses the legacy shared-construction overlay, so
    /// its construction can be converted to its own local values.
    #[props(default)]
    pub construction_is_legacy: bool,
    /// The mount's read-only Core projection, so the review shows the complete result
    /// for the choices the user has actually made.
    #[props(default)]
    pub pair_projection_port: Option<crate::mechanical_pair::MechanicalPairProjectionPort>,
    pub selected_layer: String,
    pub mismatch: Option<MechanicalBoardMismatch>,
    pub editable: bool,
    pub disabled_reason: Option<String>,
    /// Bounded per-request feedback keeps a rejected raced submit from replacing the
    /// currently admitted operation's Pending/Landed response.
    pub feedback: Rc<[MechanicalSettingsFeedback]>,
    pub summary_feedback: Option<MechanicalSettingsFeedback>,
    pub on_request: EventHandler<MechanicalSettingsRequest>,
    pub on_select_layer: EventHandler<String>,
    pub on_show_finding: EventHandler<String>,
}

#[derive(Clone, Copy)]
struct MechanicalActionsPending(Memo<Vec<String>>);

fn mechanical_action_pending(field_id: &str) -> bool {
    try_consume_context::<MechanicalActionsPending>().is_some_and(|pending| {
        pending
            .0
            .read()
            .iter()
            .any(|id| id == field_id || (field_id.ends_with(':') && id.starts_with(field_id)))
    })
}

#[component]
pub fn MechanicalSettings(props: MechanicalSettingsProps) -> Element {
    let pending_feedback = props.feedback.clone();
    let action_pending = use_memo(use_reactive((&pending_feedback,), |(feedback,)| {
        feedback
            .iter()
            .filter(|entry| entry.state == MechanicalSettingsFeedbackState::Pending)
            .map(|entry| entry.field_id.clone())
            .collect()
    }));
    use_context_provider(|| MechanicalActionsPending(action_pending));

    let request_sequence = props.request_sequence;
    let identity = &props.identity;
    let configuration_matches = props
        .values
        .as_ref()
        .is_some_and(|values| values.board_id == identity.active_board_id);
    let owner_key = format!(
        "{}:{}:{}:{}:{}:{}:{:?}",
        identity.editor_instance_id,
        identity.scope_generation,
        identity.presentation_generation,
        identity.scope.session_epoch.0,
        identity.scope.document_id,
        identity.scope.board_id,
        identity.scope.instance_id,
    );
    let current_feedback = props.summary_feedback.as_ref().filter(|feedback| {
        feedback.identity.editor_instance_id == identity.editor_instance_id
            && feedback.identity.scope_generation == identity.scope_generation
            && feedback.identity.presentation_generation == identity.presentation_generation
            && feedback.identity.scope == identity.scope
            && feedback.identity.active_board_id == identity.active_board_id
            && feedback.identity.configuration_board_id == identity.configuration_board_id
    });

    if let Some(values) = props.values.as_ref().filter(|values| {
        values.board_id == identity.active_board_id
            && values.board_id == identity.configuration_board_id
    }) && let Some(layer) = props
        .layers
        .iter()
        .find(|layer| layer.id == props.selected_layer)
        && contextual_layer_fields(&layer.id, values).is_some()
    {
        return rsx! {
            ContextualLayerInspector {
                identity: props.identity.clone(),
                request_sequence,
                values: values.clone(),
                layer: layer.clone(),
                editable: props.editable,
                disabled_reason: props.disabled_reason.clone(),
                feedback: props.feedback.clone(),
                findings: props.findings.clone(),
                on_request: props.on_request,
                on_select_layer: props.on_select_layer,
                on_show_finding: props.on_show_finding,
                owner_key: owner_key.clone(),
            }
        };
    }

    if props.values.as_ref().is_some_and(|values| {
        values.board_id == identity.active_board_id
            && values.board_id == identity.configuration_board_id
    }) && let Some(support) = props.gasket_supports.iter().find(|support| {
        props.selected_layer == format!("gasket:{}:lower", support.id)
            || props.selected_layer == format!("gasket:{}:upper", support.id)
    }) {
        let anchors = props
            .gasket_supports
            .iter()
            .filter(|candidate| {
                candidate.id == support.id
                    || (!support.unlinked
                        && support.pair_id.as_deref() == Some(candidate.id.as_str())
                        && !candidate.unlinked)
            })
            .map(MechanicalGasketSupportRow::saved_anchor)
            .collect::<Vec<_>>();
        let index = props
            .gasket_supports
            .iter()
            .position(|candidate| candidate.id == support.id)
            .unwrap_or_default()
            + 1;
        return rsx! {
            section { class: "m1-mechanical-settings", aria_label: "Mechanical stack settings",
                button {
                    r#type: "button",
                    class: "m1-mechanical-context-return",
                    onclick: move |_| props.on_select_layer.call(String::new()),
                    "Assembly settings"
                }
                h2 { "Gasket {index}" }
                if support.is_previous {
                    p { role: "status", class: "m1-mechanical-previous-result",
                        "Showing previous generated geometry. Gasket measurements cannot be edited until the current assembly resolves."
                    }
                } else {
                    p { class: "m1-mechanical-help",
                        if support.unlinked { "This gasket is unlinked from its pair." } else { "This support is linked to its mirrored pair. Matching upper and lower pads resize together." }
                    }
                    if let Some(error) = support.fit_error.as_deref() {
                        p { role: "alert", "{error} Preview and export stay blocked until it fits." }
                    } else {
                        p { class: "m1-mechanical-help", "Fits at this position." }
                    }
                    for gasket_owner in [format!("{owner_key}:gasket:{}", support.id)] {
                    fieldset { key: "{gasket_owner}", class: "m1-mechanical-group", disabled: !props.editable,
                        DimensionField {
                            identity: props.identity.clone(),
                            request_sequence,
                            on_request: props.on_request,
                            feedback: props.feedback.clone(),
                            field: MechanicalDimension::GasketSupportLength,
                            label: "Cut length",
                            value: support.length,
                            editable: props.editable,
                            support_target: Some(GasketSupportDimensionTarget {
                                support_id: support.id.clone(), anchors: anchors.clone(),
                            }),
                        }
                        DimensionField {
                            identity: props.identity.clone(),
                            request_sequence,
                            on_request: props.on_request,
                            feedback: props.feedback.clone(),
                            field: MechanicalDimension::GasketSupportWidth,
                            label: "Pad width",
                            value: support.width,
                            editable: props.editable,
                            support_target: Some(GasketSupportDimensionTarget {
                                support_id: support.id.clone(), anchors: anchors.clone(),
                            }),
                        }
                        if !props.editable {
                            if let Some(reason) = props.disabled_reason.as_deref() {
                                p { role: "status", "{reason}" }
                            }
                        }
                    }
                    }
                    p { class: "m1-mechanical-help", "Prefer 10 mm cuts; use 5 mm increments for a tighter fit. Foam thickness and compression are shared by the floating stack." }
                    button {
                        r#type: "button",
                        class: "m1-mechanical-context-settings",
                        onclick: move |_| props.on_select_layer.call("gaskets".to_owned()),
                        "All gasket settings"
                    }
                }
                if !support.is_previous {
                    {gasket_fit_issues(props.findings.clone(), props.on_show_finding)}
                }
            }
        };
    }

    rsx! {
        section { class: "m1-mechanical-settings", aria_label: "Mechanical stack settings",
            h2 { "Case construction" }
            if let Some(features) = props.pair_features.as_ref().filter(|features| !features.is_empty()) {
                MechanicalPairControls {
                    identity: props.identity.clone(),
                    request_sequence,
                    features: features.clone(),
                    editable: props.editable,
                    feedback: props.feedback.clone(),
                    on_request: props.on_request,
                }
            } else if let Some(options) = props.pair_link_options.as_ref().filter(|options| !options.is_empty()) {
                MechanicalPairLinkControls {
                    identity: props.identity.clone(),
                    request_sequence,
                    options: options.clone(),
                    editable: props.editable,
                    projection_port: props.pair_projection_port.clone(),
                    on_request: props.on_request,
                }
            }
            if props.layers.iter().any(|layer| layer.is_previous) {
                p { role: "status", class: "m1-mechanical-previous-result",
                    "Showing generated layers from a previous accepted revision. Settings below reflect the current accepted configuration."
                }
            }
            if let Some(mismatch) = props.mismatch.as_ref() {
                p { role: "status", class: "m1-mechanical-mismatch",
                    "Mechanical settings are unavailable while {mismatch.board_name} is configured. Use the authored Case panel to return to that board."
                }
            }
            if let Some(values) = props.values.as_ref().filter(|_| configuration_matches) {
                if let Some(feedback) = current_feedback {
                    if feedback.state == MechanicalSettingsFeedbackState::Failed && !is_dimension_field(&feedback.field_id)
                        && let Some(message) = feedback.message.as_deref()
                    {
                        p { role: "alert", "{message}" }
                        if feedback.field_id == "initialize-closures" {
                            button {
                                r#type: "button",
                                disabled: mechanical_action_pending("initialize-closures") || !props.editable,
                                onclick: {
                                    let identity = props.identity.clone();
                                    let mut sequence = props.request_sequence;
                                    let on_request = props.on_request;
                                    move |_| send_request(&mut sequence, &identity, on_request, MechanicalSettingsPatch::InitializeClosures)
                                },
                                "Retry mounting locations"
                            }
                        }
                    }
                }
                if !props.editable {
                    if let Some(reason) = props.disabled_reason.as_deref() {
                        p { role: "status", "{reason}" }
                    }
                }
                p { class: "m1-mechanical-help", "Authored Case bodies remain saved while the generated stack is active. Disable the stack to return to authored bodies." }
                ConstructionControls {
                    identity: props.identity.clone(),
                    values: values.clone(),
                    editable: props.editable,
                    request_sequence,
                    on_request: props.on_request,
                }
                if props.construction_is_legacy {
                    MechanicalLegacyConstructionControl {
                        identity: props.identity.clone(),
                        request_sequence,
                        editable: props.editable,
                        on_request: props.on_request,
                    }
                }
                ProfileGuidance {
                    identity: props.identity.clone(),
                    owner_key: owner_key.clone(),
                    feedback: props.feedback.clone(),
                    profiles: props.profiles.clone(),
                    extract_profile: props.extract_profile.clone(),
                    targets: props.profile_targets.clone(),
                    plate_thickness: values.plate_thickness,
                    plate_to_pcb: values.plate_to_pcb,
                    editable: props.editable,
                    request_sequence,
                    on_request: props.on_request,
                }
                ManufacturingControls {
                    identity: props.identity.clone(),
                    targets: props.process_targets.clone(),
                    stabilizers: props.stabilizer_fits.clone(),
                    editable: props.editable,
                    feedback: props.feedback.clone(),
                    request_sequence,
                    on_request: props.on_request,
                }
                DimensionControls {
                    identity: props.identity.clone(),
                    values: values.clone(),
                    editable: props.editable,
                    disabled_reason: props.disabled_reason.clone(),
                    feedback: props.feedback.clone(),
                    request_sequence,
                    on_request: props.on_request,
                    owner_key: owner_key.clone(),
                }
                ResolvedStack {
                    layers: props.layers.clone(),
                    selected_layer: props.selected_layer.clone(),
                    on_select_layer: props.on_select_layer,
                }
                Diagnostics {
                    findings: props.findings.clone(),
                    on_show_finding: props.on_show_finding,
                }
                BatteryControls {
                    identity: props.identity.clone(),
                    request_sequence,
                    values: values.clone(),
                    editable: props.editable,
                    feedback: props.feedback.clone(),
                    on_request: props.on_request,
                    owner_key: owner_key.clone(),
                }
                for opening_owner in [format!("{owner_key}:case-openings")] {
                    OpeningControls {
                        key: "{opening_owner}",
                        identity: props.identity.clone(),
                        request_sequence,
                        values: values.clone(),
                        editable: props.editable,
                        feedback: props.feedback.clone(),
                        on_request: props.on_request,
                        owner_key: owner_key.clone(),
                    }
                }
                // A keyed dynamic owner boundary retires child field drafts on scope changes.
                for mounting_owner in [format!("{owner_key}:mounting")] {
                    MountingControls {
                        key: "{mounting_owner}",
                    identity: props.identity.clone(),
                    request_sequence,
                    values: values.clone(),
                    suggested_mounts: props.suggested_mounts.clone(),
                    editable: props.editable,
                    feedback: props.feedback.clone(),
                    on_request: props.on_request,
                    }
                }
                if values.mount == MechanicalMount::Gasket && values.internal_gasket {
                    section { class: "m1-mechanical-option-group", aria_label: "Gasket supports",
                        h3 { "Gasket supports" }
                        button {
                            r#type: "button",
                            class: "m1-mechanical-quiet",
                            disabled: mechanical_action_pending("reset-gasket-placement") || !props.editable,
                            onclick: {
                                let identity = props.identity.clone();
                                let mut sequence = request_sequence;
                                let on_request = props.on_request;
                                move |_| send_request(&mut sequence, &identity, on_request, MechanicalSettingsPatch::ResetGasketPlacement)
                            },
                            "Reset gasket placement"
                        }
                        p { class: "m1-mechanical-help",
                            "Reset releases manually positioned supports. Closure positions stay fixed."
                        }
                    }
                }
                if values.internal_gasket {
                    GasketClosureControls {
                        key: "{owner_key}:gasket-closure-hardware",
                        identity: props.identity.clone(),
                        request_sequence,
                        hardware: values.closure_hardware.clone(),
                        editable: props.editable,
                        feedback: props.feedback.clone(),
                        on_request: props.on_request,
                        owner_key: owner_key.clone(),
                    }
                }
                for fit_owner in [format!("{owner_key}:critical-fits")] {
                    CriticalFitControls {
                        key: "{fit_owner}",
                        identity: props.identity.clone(),
                        request_sequence,
                        fits: values.critical_fits.clone(),
                        hardware: values.hardware.clone(),
                        hardware_mounts: props.hardware_mounts.clone(),
                        fit_parts: props.fit_parts.clone(),
                        fit_parts_resolved: props.fit_parts_resolved,
                        values: values.clone(),
                        editable: props.editable,
                        feedback: props.feedback.clone(),
                        on_request: props.on_request,
                        owner_key: owner_key.clone(),
                    }
                }
                button {
                    r#type: "button",
                    class: "m1-mechanical-disable",
                    "aria-label": if props.pair_counterpart.is_some() { "Unlink and disable this half" } else { "Disable mechanical stack" },
                    disabled: !props.editable,
                    onclick: {
                        let identity = props.identity.clone();
                        let mut sequence = request_sequence;
                        let on_request = props.on_request;
                        move |_| send_request(&mut sequence, &identity, on_request, MechanicalSettingsPatch::Disable)
                    },
                    if props.pair_counterpart.is_some() { "Unlink and disable this half" } else { "Disable mechanical stack" }
                }
                if let Some(counterpart) = props.pair_counterpart.clone() {
                    p { class: "m1-mechanical-help",
                        "This half is linked with {counterpart}. Disabling removes that link and disables only this half's mechanical stack: the other half keeps its geometry and settings, and neither PCB is removed. One Undo restores the link and this stack."
                    }
                    MechanicalPairMemberRemoval {
                        identity: props.identity.clone(),
                        request_sequence,
                        counterpart,
                        editable: props.editable,
                        on_request: props.on_request,
                    }
                }
            } else if props.values.is_some() {
                p { role: "status", "The configured stack belongs to another board. Switch to that board to edit it." }
            } else {
                if let Some(feedback) = current_feedback {
                    if feedback.state == MechanicalSettingsFeedbackState::Pending {
                        p { role: "status", "Saving mechanical settings…" }
                    } else if feedback.state == MechanicalSettingsFeedbackState::Failed {
                        if let Some(message) = feedback.message.as_deref() { p { role: "alert", "{message}" } }
                    }
                }
                p { class: "m1-mechanical-help", "Configure a mechanical stack from the current board and represented switch fit profiles." }
                if !props.editable {
                    if let Some(reason) = props.disabled_reason.as_deref() { p { role: "status", "{reason}" } }
                }
                button {
                    r#type: "button",
                    class: "m1-mechanical-configure",
                    disabled: !props.editable,
                    onclick: {
                        let identity = props.identity.clone();
                        let mut sequence = request_sequence;
                        let on_request = props.on_request;
                        move |_| send_request(&mut sequence, &identity, on_request, MechanicalSettingsPatch::Enable)
                    },
                    "Configure mechanical stack"
                }
            }
        }
    }
}

#[derive(Props, Clone, PartialEq)]
struct MechanicalLegacyConstructionControlProps {
    identity: MechanicalSettingsIdentity,
    request_sequence: Signal<u64>,
    editable: bool,
    on_request: EventHandler<MechanicalSettingsRequest>,
}

/// Converting one legacy half's construction to its own local values.
///
/// The confirmation states the whole change before anything is submitted: this half's
/// current effective construction (including the overlaid hardware, critical fits and
/// profiles) becomes its own local values, its ownership becomes local, the other legacy
/// halves keep the baseline they had, and one Undo restores the ownership and the overlay.
/// Exactly one instance is named, and Core validates it.
#[component]
fn MechanicalLegacyConstructionControl(props: MechanicalLegacyConstructionControlProps) -> Element {
    let confirming = use_signal(|| false);
    let instance_id = props.identity.scope.instance_id.clone().unwrap_or_default();
    let patch = MechanicalSettingsPatch::MaterializeLegacyConstruction {
        instance_ids: vec![instance_id.clone()],
    };
    let pending = mechanical_action_pending(&patch.field_id()) || !props.editable;
    let mut confirming_state = confirming;
    rsx! {
        fieldset { class: "m1-mechanical-legacy-construction", disabled: !props.editable,
            legend { "Shared construction" }
            if confirming() {
                p { class: "m1-mechanical-help",
                    "Makes {instance_id}'s construction its own: the values it shows today (including the overlaid hardware, critical fits and profiles) become this half's local values and its construction becomes local. Other legacy halves keep their previous baseline. One Undo restores the ownership and the overlay."
                }
                button {
                    r#type: "button",
                    class: "m1-mechanical-quiet",
                    "aria-label": "Confirm making construction local for {instance_id}",
                    disabled: pending,
                    onclick: {
                        let identity = props.identity.clone();
                        let mut sequence = props.request_sequence;
                        let on_request = props.on_request;
                        let patch = patch.clone();
                        move |_| {
                            confirming_state.set(false);
                            send_request(&mut sequence, &identity, on_request, patch.clone());
                        }
                    },
                    "Make construction local"
                }
                button {
                    r#type: "button",
                    class: "m1-mechanical-quiet",
                    "aria-label": "Keep shared construction for {instance_id}",
                    onclick: move |_| confirming_state.set(false),
                    "Keep it shared"
                }
            } else {
                p { class: "m1-mechanical-help",
                    "This half still uses the shared construction baseline."
                }
                button {
                    r#type: "button",
                    class: "m1-mechanical-quiet",
                    "aria-label": "Make construction local for {instance_id}",
                    disabled: pending,
                    onclick: move |_| confirming_state.set(true),
                    "Make construction local…"
                }
            }
        }
    }
}

#[derive(Props, Clone, PartialEq)]
struct MechanicalPairMemberRemovalProps {
    identity: MechanicalSettingsIdentity,
    request_sequence: Signal<u64>,
    counterpart: String,
    editable: bool,
    on_request: EventHandler<MechanicalSettingsRequest>,
}

/// Removing one half of a mechanical pair, behind an explicit confirmation.
///
/// The typed removal is submitted only after the confirmation states the whole result:
/// this assembly leaves the project, the link is cleared, the other half keeps the
/// geometry and settings it shows today as its own local assembly, and the boards, PCBs
/// and parts stay. Core owns the materialization; the panel never writes geometry.
#[component]
fn MechanicalPairMemberRemoval(props: MechanicalPairMemberRemovalProps) -> Element {
    let confirming = use_signal(|| false);
    let instance_id = props.identity.scope.instance_id.clone().unwrap_or_default();
    let pending = mechanical_action_pending("remove-physical-instance") || !props.editable;
    let counterpart = props.counterpart.clone();
    let mut confirming_state = confirming;
    rsx! {
        section { class: "m1-mechanical-remove-member", aria_label: "Remove this physical assembly",
            if confirming() {
                p { class: "m1-mechanical-help",
                    "Removes {instance_id} from the project and clears the link. {counterpart} keeps the geometry and settings it shows today as its own local assembly. Your boards, PCBs and parts stay. One Undo restores the assembly and the link."
                }
                button {
                    r#type: "button",
                    class: "m1-mechanical-quiet",
                    "aria-label": "Confirm removing assembly {instance_id}",
                    disabled: pending,
                    onclick: {
                        let identity = props.identity.clone();
                        let mut sequence = props.request_sequence;
                        let on_request = props.on_request;
                        move |_| {
                            confirming_state.set(false);
                            send_request(
                                &mut sequence,
                                &identity,
                                on_request,
                                MechanicalSettingsPatch::RemovePhysicalInstance,
                            );
                        }
                    },
                    "Remove assembly"
                }
                button {
                    r#type: "button",
                    class: "m1-mechanical-quiet",
                    "aria-label": "Keep assembly {instance_id}",
                    onclick: move |_| confirming_state.set(false),
                    "Keep it"
                }
            } else {
                p { class: "m1-mechanical-help",
                    "Removing this assembly clears the link and removes only it; the other half stays as its own local assembly."
                }
                button {
                    r#type: "button",
                    class: "m1-mechanical-quiet",
                    "aria-label": "Remove assembly {instance_id}",
                    disabled: pending,
                    onclick: move |_| confirming_state.set(true),
                    "Remove this assembly…"
                }
            }
        }
    }
}

#[derive(Props, Clone, PartialEq)]
struct MechanicalPairLinkControlsProps {
    identity: MechanicalSettingsIdentity,
    request_sequence: Signal<u64>,
    options: Rc<[MechanicalPairLinkOption]>,
    editable: bool,
    /// The mount's read-only Core projection of the reviewed choices.
    projection_port: Option<crate::mechanical_pair::MechanicalPairProjectionPort>,
    on_request: EventHandler<MechanicalSettingsRequest>,
}

/// The reviewed link controls for a half that belongs to no pair yet.
///
/// The review shows the map the link will use, the construction source and the
/// durable-id feature matches the link would consolidate, so nothing is submitted
/// before the user has seen the complete proposed result. A translated or rotated
/// board frame has no derived map, so its map is entered explicitly.
#[component]
fn MechanicalPairLinkControls(props: MechanicalPairLinkControlsProps) -> Element {
    let selected = use_signal(|| 0_usize);
    let construction_from_other = use_signal(|| false);
    // Per-feature source choices, keyed by the reviewed half's durable feature id so a
    // re-render cannot hand a choice to a different feature.
    let sources = use_signal(std::collections::BTreeMap::<String, String>::new);
    let mapping_x = use_signal(String::new);
    let mapping_y = use_signal(String::new);
    let mapping_rotation = use_signal(String::new);
    let options = props.options.clone();
    let index = selected().min(options.len().saturating_sub(1));
    let option = options[index].clone();
    let number = |text: String| {
        text.trim()
            .parse::<f64>()
            .ok()
            .filter(|value| value.is_finite())
    };
    let explicit_mapping = number(mapping_x())
        .zip(number(mapping_y()))
        .map(|(x, y)| Pose2 {
            at: Vec2 { x, y },
            rotation: number(mapping_rotation()).unwrap_or(0.0),
        });
    let mapping = option.derived_mapping.or(explicit_mapping);
    let mapping_note = match (option.derived_mapping, mapping) {
        (Some(derived), _) => format!(
            "Derived from the halves' declared transforms: offset ({:?}, {:?}) · rotation {:?}°",
            derived.at.x, derived.at.y, derived.rotation
        ),
        (None, Some(reviewed)) => format!(
            "Reviewed map: offset ({:?}, {:?}) · rotation {:?}°",
            reviewed.at.x, reviewed.at.y, reviewed.rotation
        ),
        (None, None) => {
            "A translated or rotated board frame needs its map entered before linking.".to_owned()
        }
    };
    let construction_from_other_value = construction_from_other();
    let construction_source = if construction_from_other_value {
        option.other_instance_id.clone()
    } else {
        option.instance_id.clone()
    };

    // The complete proposed result is projected for the choices actually reviewed, from
    // Core through the mount's port. The cache key is the accepted snapshot plus every
    // reviewed choice, so a change reprojects and an unrelated render does not.
    let sources_value = sources();
    let projection_key = format!(
        "{}|{index}|{}|{}|{}|{construction_from_other_value}|{}",
        props.identity.snapshot_token.0,
        mapping_x(),
        mapping_y(),
        mapping_rotation(),
        sources_value
            .iter()
            .map(|(feature, source)| format!("{feature}={source}"))
            .collect::<Vec<_>>()
            .join(","),
    );
    let projection_cache = use_hook(|| {
        Rc::new(std::cell::RefCell::new(
            None::<(
                String,
                Option<Result<crate::mechanical_pair::MechanicalLinkPreview, String>>,
            )>,
        ))
    });
    let projection: Option<Result<crate::mechanical_pair::MechanicalLinkPreview, String>> = {
        let cached = projection_cache.borrow();
        match cached.as_ref() {
            Some((key, value)) if *key == projection_key => value.clone(),
            _ => {
                drop(cached);
                let value = props.projection_port.as_ref().map(|port| {
                    match crate::mechanical_pair::reviewed_link_change(
                        &option,
                        mapping,
                        construction_from_other_value,
                        &sources_value,
                    ) {
                        Some(change) => port.0(&change),
                        None => Err(
                            "A translated or rotated board frame needs its map entered before the result is projected."
                                .to_owned(),
                        ),
                    }
                });
                *projection_cache.borrow_mut() = Some((projection_key, value.clone()));
                value
            }
        }
    };
    // The offer's own projection is the fallback for callers without a port.
    let (preview, preview_note) = match projection {
        Some(Ok(preview)) => (Some(preview), None),
        Some(Err(reason)) => (None, Some(reason)),
        None => (option.preview.clone(), option.preview_note.clone()),
    };
    let blocked = mechanical_action_pending(&format!("mechanical-pair:{}:link", option.pair_id))
        || !props.editable
        || mapping.is_none()
        || preview.is_none()
        || option.reviewed_fingerprint.is_none();
    rsx! {
        section { class: "m1-mechanical-pair m1-mechanical-pair-link", aria_label: "Link a mechanical mirror pair",
            h3 { "Mirror another half" }
            p { class: "m1-mechanical-help",
                "Linking keeps shared mounts, closure mounts, openings and gasket supports reflected and linked. Features without a match stay local to their half."
            }
            label { class: "m1-mechanical-field",
                span { "Link with" }
                select {
                    "aria-label": "Link with",
                    disabled: !props.editable,
                    value: "{index}",
                    onchange: {
                        let mut selected = selected;
                        move |event: FormEvent| {
                            if let Ok(value) = event.value().parse::<usize>() {
                                selected.set(value);
                            }
                        }
                    },
                    for (candidate, item) in options.iter().enumerate() {
                        option { value: "{candidate}", "{item.other_name}" }
                    }
                }
            }
            label { class: "m1-mechanical-field",
                span { "Construction from" }
                select {
                    "aria-label": "Pair construction source",
                    disabled: !props.editable,
                    value: if construction_from_other_value { "other" } else { "this" },
                    onchange: {
                        let mut construction_from_other = construction_from_other;
                        move |event: FormEvent| {
                            construction_from_other.set(event.value() == "other");
                        }
                    },
                    option { value: "this", "{option.instance_id}" }
                    option { value: "other", "{option.other_instance_id}" }
                }
            }
            if option.derived_mapping.is_none() {
                div { class: "m1-mechanical-pair-mapping",
                    label { class: "m1-mechanical-field",
                        span { "Map offset X" }
                        input {
                            r#type: "number", "aria-label": "Pair mapping X", value: "{mapping_x}",
                            oninput: { let mut mapping_x = mapping_x; move |event: FormEvent| mapping_x.set(event.value()) }
                        }
                    }
                    label { class: "m1-mechanical-field",
                        span { "Map offset Y" }
                        input {
                            r#type: "number", "aria-label": "Pair mapping Y", value: "{mapping_y}",
                            oninput: { let mut mapping_y = mapping_y; move |event: FormEvent| mapping_y.set(event.value()) }
                        }
                    }
                    label { class: "m1-mechanical-field",
                        span { "Map rotation" }
                        input {
                            r#type: "number", "aria-label": "Pair mapping rotation", value: "{mapping_rotation}",
                            oninput: { let mut mapping_rotation = mapping_rotation; move |event: FormEvent| mapping_rotation.set(event.value()) }
                        }
                    }
                }
            }
            p { class: "m1-mechanical-help", "{mapping_note}" }
            if option.matched.is_empty() {
                p { class: "m1-mechanical-help", "Matched features: {option.matched_features}" }
            } else {
                p { class: "m1-mechanical-help",
                    "Matched features stay linked. Features without a match stay local to their half."
                }
            }
            for matched in option.matched.iter() {
                {
                    let matched = matched.clone();
                    let key = matched.first_feature_id.clone();
                    let chosen = sources()
                        .get(&key)
                        .cloned()
                        .unwrap_or_else(|| matched.source_instance_id.clone());
                    let first_label = matched
                        .first_geometry
                        .clone()
                        .unwrap_or_else(|| "no geometry".to_owned());
                    let second_label = matched
                        .second_geometry
                        .clone()
                        .unwrap_or_else(|| "no geometry".to_owned());
                    let row_id = format!("pair-source-{key}");
                    rsx! {
                        fieldset { key: "{row_id}", class: "m1-mechanical-pair-source",
                            "aria-label": "Source for {key}",
                            legend { "{feature_label(matched.kind)} {key} · {matched.second_feature_id}" }
                            label { class: "m1-mechanical-field",
                                span { "Shared geometry from" }
                                select {
                                    "aria-label": "Source for {key}",
                                    disabled: !props.editable,
                                    value: "{chosen}",
                                    onchange: {
                                        let mut sources = sources;
                                        let key = key.clone();
                                        move |event: FormEvent| {
                                            sources.write().insert(key.clone(), event.value());
                                        }
                                    },
                                    option { value: "{matched.source_instance_id}",
                                        "{matched.source_instance_id} · {first_label}"
                                    }
                                    option { value: "{option.other_instance_id}",
                                        "{option.other_instance_id} · {second_label}"
                                    }
                                }
                            }
                        }
                    }
                }
            }
            // The complete proposed result on both halves, read from Core for exactly
            // the choices reviewed above.
            if let Some(preview) = preview.clone() {
                div { class: "m1-mechanical-pair-preview", aria_label: "Proposed result on both halves",
                    for member in preview.members.iter() {
                        div { class: "m1-mechanical-pair-preview-member", "data-instance": "{member.instance_id}",
                            h4 { "{member.instance_id}" }
                            if member.features.is_empty() {
                                p { class: "m1-mechanical-help", "No mechanical features after linking." }
                            } else {
                                ul {
                                    for feature in member.features.iter() {
                                        li { "{feature}" }
                                    }
                                }
                            }
                            for finding in member.findings.iter() {
                                p { role: "alert", class: "m1-mechanical-pair-preview-finding", "{finding}" }
                            }
                        }
                    }
                }
            } else if let Some(note) = preview_note.clone() {
                p { class: "m1-mechanical-help",
                    "The complete proposed result is unavailable until you confirm: {note}"
                }
            }
            button {
                r#type: "button",
                class: "m1-mechanical-quiet",
                "aria-label": "Link {option.instance_id} with {option.other_instance_id}",
                disabled: blocked,
                onclick: {
                    let mut sequence = props.request_sequence;
                    let identity = props.identity.clone();
                    let callback = props.on_request;
                    let option = option.clone();
                    let reviewed = option.reviewed_fingerprint.clone();
                    move |_| {
                        // Exactly the choices the review projected and showed.
                        let Some(change) = crate::mechanical_pair::reviewed_link_change(
                            &option,
                            mapping,
                            construction_from_other_value,
                            &sources.peek(),
                        ) else {
                            return;
                        };
                        send_request_with_review(
                            &mut sequence,
                            &identity,
                            callback,
                            change,
                            reviewed.clone(),
                        );
                    }
                },
                "Link halves"
            }
        }
    }
}

#[derive(Props, Clone, PartialEq)]
struct MechanicalPairControlsProps {
    identity: MechanicalSettingsIdentity,
    request_sequence: Signal<u64>,
    features: Rc<[MechanicalPairFeatureRow]>,
    editable: bool,
    feedback: Rc<[MechanicalSettingsFeedback]>,
    on_request: EventHandler<MechanicalSettingsRequest>,
}

/// Per-feature controls for the selected half's mechanical mirror pair.
///
/// The state of every row comes from the accepted pair store, so there is no second
/// relationship store to drift. A linked feature can be made local or removed on this
/// half; a local or omitted feature can be rejoined from the other half, with the
/// geometry the rejoin takes shown before the commit.
#[component]
fn MechanicalPairControls(props: MechanicalPairControlsProps) -> Element {
    let features = props.features.clone();
    // Per-feature rejoin source choices, keyed by this half's durable feature id.
    let rejoin_sources = use_signal(std::collections::BTreeMap::<String, String>::new);
    rsx! {
        section { class: "m1-mechanical-pair", aria_label: "Mirrored pair features",
            h3 { "Mirrored pair" }
            p { class: "m1-mechanical-help",
                "Shared mechanical features stay linked to the other half. Make one local, remove it on this half, or rejoin it from the other half."
            }
            for row in features.iter() {
                {
                    let row = row.clone();
                    let key = format!("{}:{}", row.instance_id, row.feature_id);
                    let linked = row.state == MechanicalFeatureState::Linked;
                    let local_patch = MechanicalSettingsPatch::MakeFeatureLocal {
                        feature_id: row.feature_id.clone(),
                        instance_id: row.instance_id.clone(),
                    };
                    let omit_patch = MechanicalSettingsPatch::OmitFeatureMember {
                        feature_id: row.feature_id.clone(),
                        instance_id: row.instance_id.clone(),
                    };
                    // A local half may deliberately rejoin from its own current geometry,
                    // keeping this half's placement and re-deriving the other half; an
                    // omitted half has no geometry of its own to take.
                    let offers_own_source = row.state == MechanicalFeatureState::Local
                        && row.own_geometry.is_some();
                    let chosen_source = rejoin_sources
                        .read()
                        .get(&row.feature_id)
                        .cloned()
                        .unwrap_or_else(|| row.counterpart_instance_id.clone());
                    let source_is_own = chosen_source == row.instance_id;
                    let source_geometry = if source_is_own {
                        row.own_geometry.clone()
                    } else {
                        row.preview.clone()
                    };
                    let rejoin_patch = MechanicalSettingsPatch::RejoinFeatureMember {
                        feature_id: row.feature_id.clone(),
                        instance_id: row.instance_id.clone(),
                        source_instance_id: chosen_source.clone(),
                    };
                    let row_fields = [
                        local_patch.field_id(),
                        omit_patch.field_id(),
                        rejoin_patch.field_id(),
                    ];
                    let failure = props
                        .feedback
                        .iter()
                        .find(|entry| {
                            entry.state == MechanicalSettingsFeedbackState::Failed
                                && row_fields.contains(&entry.field_id)
                        })
                        .and_then(|entry| entry.message.clone());
                    let blocked = |patch: &MechanicalSettingsPatch| {
                        mechanical_action_pending(&patch.field_id()) || !props.editable
                    };
                    let local_blocked = blocked(&local_patch);
                    let omit_blocked = blocked(&omit_patch);
                    let rejoin_blocked = blocked(&rejoin_patch)
                        || source_geometry.is_none()
                        || row.rejoin_fingerprint.is_none();
                    let source_label = if source_is_own {
                        "this half"
                    } else {
                        "the other half"
                    };
                    let rejoin_label = if source_is_own {
                        "Rejoin from this half"
                    } else {
                        "Rejoin from the other half"
                    };
                    let state = match row.state {
                        MechanicalFeatureState::Linked => "linked to the other half",
                        MechanicalFeatureState::Local => "local to this half",
                        MechanicalFeatureState::Omitted => "removed on this half",
                    };
                    rsx! {
                        fieldset { key: "{key}", class: "m1-mechanical-pair-row", disabled: !props.editable,
                            "aria-label": "Mirrored {feature_label(row.kind).to_lowercase()}",
                            legend { "{feature_label(row.kind)} · {state}" }
                            if let Some(message) = failure {
                                p { role: "alert", "{message}" }
                            }
                            if linked {
                                button {
                                    r#type: "button",
                                    class: "m1-mechanical-quiet",
                                    "aria-label": "Make local {row.feature_id}",
                                    disabled: local_blocked,
                                    onclick: {
                                        let mut sequence = props.request_sequence;
                                        let identity = props.identity.clone();
                                        let callback = props.on_request;
                                        let patch = local_patch.clone();
                                        move |_| send_request(&mut sequence, &identity, callback, patch.clone())
                                    },
                                    "Make local"
                                }
                                button {
                                    r#type: "button",
                                    class: "m1-mechanical-quiet",
                                    "aria-label": "Remove on this half {row.feature_id}",
                                    disabled: omit_blocked,
                                    onclick: {
                                        let mut sequence = props.request_sequence;
                                        let identity = props.identity.clone();
                                        let callback = props.on_request;
                                        let patch = omit_patch.clone();
                                        move |_| send_request(&mut sequence, &identity, callback, patch.clone())
                                    },
                                    "Remove on this half"
                                }
                            } else {
                                if offers_own_source {
                                    label { class: "m1-mechanical-field",
                                        span { "Rejoin from" }
                                        select {
                                            "aria-label": "Rejoin source for {row.feature_id}",
                                            disabled: !props.editable,
                                            value: "{chosen_source}",
                                            onchange: {
                                                let mut rejoin_sources = rejoin_sources;
                                                let feature_id = row.feature_id.clone();
                                                move |event: FormEvent| {
                                                    rejoin_sources
                                                        .write()
                                                        .insert(feature_id.clone(), event.value());
                                                }
                                            },
                                            option { value: "{row.instance_id}",
                                                "this half · {row.own_geometry.clone().unwrap_or_default()}"
                                            }
                                            option { value: "{row.counterpart_instance_id}",
                                                "the other half · {row.preview.clone().unwrap_or_else(|| \"no geometry\".to_owned())}"
                                            }
                                        }
                                    }
                                }
                                if let Some(geometry) = source_geometry.clone() {
                                    p { class: "m1-mechanical-help",
                                        "Rejoin takes {geometry} from {chosen_source}."
                                    }
                                } else {
                                    p { class: "m1-mechanical-help",
                                        "The reviewed source half has no geometry for this feature, so it cannot be rejoined yet."
                                    }
                                }
                                button {
                                    r#type: "button",
                                    class: "m1-mechanical-quiet",
                                    "aria-label": "Rejoin {row.feature_id} from {source_label}",
                                    disabled: rejoin_blocked,
                                    onclick: {
                                        let mut sequence = props.request_sequence;
                                        let identity = props.identity.clone();
                                        let callback = props.on_request;
                                        let patch = rejoin_patch.clone();
                                        // The reviewed fingerprint the row was produced with
                                        // travels with the submission, so a relevant change
                                        // between this review and its execution retires it.
                                        let reviewed = row.rejoin_fingerprint.clone();
                                        move |_| {
                                            send_request_with_review(
                                                &mut sequence,
                                                &identity,
                                                callback,
                                                patch.clone(),
                                                reviewed.clone(),
                                            )
                                        }
                                    },
                                    "{rejoin_label}"
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[derive(Props, Clone, PartialEq)]
struct GasketClosureControlsProps {
    identity: MechanicalSettingsIdentity,
    request_sequence: Signal<u64>,
    hardware: Option<InternalClosureHardware>,
    editable: bool,
    feedback: Rc<[MechanicalSettingsFeedback]>,
    on_request: EventHandler<MechanicalSettingsRequest>,
    owner_key: String,
}

#[component]
fn GasketClosureControls(props: GasketClosureControlsProps) -> Element {
    let Some(hardware) = props.hardware.as_ref() else {
        return rsx! {};
    };
    let selected_preset = [
        ("m2-3", 2.0, 0.4, 3.0, 3.2),
        ("m2-4", 2.0, 0.4, 4.0, 3.2),
        ("m2.5-3", 2.5, 0.45, 3.0, 3.5),
        ("m2.5-4", 2.5, 0.45, 4.0, 3.5),
        ("m2.5-5", 2.5, 0.45, 5.0, 3.5),
        ("m3-3", 3.0, 0.5, 3.0, 4.2),
    ]
    .into_iter()
    .find(|(_, diameter, _, length, insert_diameter)| {
        hardware.thread_diameter == *diameter
            && hardware.insert_length == *length
            && hardware.insert_diameter == *insert_diameter
    })
    .map(|(id, ..)| id);
    let selected_length = hardware
        .fixed_length
        .map_or_else(|| "auto".to_owned(), |length| length.to_string());
    let request_sequence = props.request_sequence;
    let identity = props.identity.clone();
    let on_request = props.on_request;
    rsx! {
        details { class: "m1-mechanical-group", open: true, aria_label: "Closure hardware",
            summary { "Closure hardware · {hardware.thread}" }
            p { class: "m1-mechanical-help", "Custom screw and insert dimensions. Review these against your hardware; the starting M2 dimensions are not a supplier preset." }
            label { class: "m1-mechanical-field",
                span { "Insert size" }
                select {
                    aria_label: "Insert size",
                    disabled: !props.editable,
                    value: selected_preset.unwrap_or("custom"),
                    onchange: {
                        let mut sequence = request_sequence;
                        let identity = identity.clone();
                        move |event: FormEvent| {
                            let id = event.value();
                            if id != "custom" {
                                send_request(&mut sequence, &identity, on_request, MechanicalSettingsPatch::SetClosureInsertPreset(id));
                            }
                        }
                    },
                    option { value: "custom", "Custom dimensions" }
                    option { value: "m2-3", "M2 × L3 × OD3.2" }
                    option { value: "m2-4", "M2 × L4 × OD3.2" }
                    option { value: "m2.5-3", "M2.5 × L3 × OD3.5" }
                    option { value: "m2.5-4", "M2.5 × L4 × OD3.5" }
                    option { value: "m2.5-5", "M2.5 × L5 × OD3.5" }
                    option { value: "m3-3", "M3 × L3 × OD4.2" }
                }
            }
            label { class: "m1-mechanical-field",
                span { "Screw drive" }
                select {
                    disabled: !props.editable,
                    value: if hardware.drive == ScrewDrive::Hex { "hex" } else { "torx" },
                    onchange: {
                        let mut sequence = request_sequence;
                        let identity = identity.clone();
                        move |event: FormEvent| {
                            let drive = if event.value() == "torx" { ScrewDrive::Torx } else { ScrewDrive::Hex };
                            send_request(&mut sequence, &identity, on_request, MechanicalSettingsPatch::SetClosureDrive(drive));
                        }
                    },
                    option { value: "hex", "Hex socket" }
                    option { value: "torx", "Torx" }
                }
            }
            label { class: "m1-mechanical-field",
                span { "Insert installation" }
                select {
                    disabled: !props.editable,
                    value: if hardware.installation == InsertInstallation::HeatSet { "heat-set" } else { "tapped" },
                    onchange: {
                        let mut sequence = request_sequence;
                        let identity = identity.clone();
                        move |event: FormEvent| {
                            let installation = if event.value() == "tapped" { InsertInstallation::Tapped } else { InsertInstallation::HeatSet };
                            send_request(&mut sequence, &identity, on_request, MechanicalSettingsPatch::SetClosureInstallation(installation));
                        }
                    },
                    option { value: "heat-set", "Heat-set · printed top" }
                    option { value: "tapped", "Tapped · machined top" }
                }
            }
            label { class: "m1-mechanical-field",
                span { "Screw length" }
                select {
                    aria_label: "Screw length",
                    disabled: !props.editable,
                    value: selected_length,
                    onchange: {
                        let mut sequence = request_sequence;
                        let identity = identity.clone();
                        let lengths = hardware.screw_lengths.clone();
                        move |event: FormEvent| {
                            let fixed = event.value().parse::<f64>().ok();
                            if fixed.is_none() || fixed.is_some_and(|length| lengths.contains(&length)) {
                                send_request(&mut sequence, &identity, on_request, MechanicalSettingsPatch::SetClosureFixedLength(fixed));
                            }
                        }
                    },
                    option { value: "auto", "Automatic from available lengths" }
                    for length in hardware.screw_lengths.iter() {
                        option { value: "{length}", "{length} mm · fixed" }
                    }
                }
            }
            TextDraftField {
                key: "{props.owner_key}:closure-screw-lengths",
                identity: props.identity.clone(),
                request_sequence,
                on_request,
                feedback: props.feedback.clone(),
                label: "Available screw lengths (mm)",
                value: hardware.screw_lengths.iter().map(ToString::to_string).collect::<Vec<_>>().join(", "),
                intent: MechanicalTextIntent::ClosureScrewLengths,
                editable: props.editable,
            }
            details { class: "m1-mechanical-group", aria_label: "Advanced closure dimensions",
                summary { "Advanced closure dimensions · Custom · mm" }
                TextDraftField {
                    key: "{props.owner_key}:closure-thread",
                    identity: props.identity.clone(),
                    request_sequence,
                    on_request,
                    feedback: props.feedback.clone(),
                    label: "Thread designation",
                    value: hardware.thread.clone(),
                    intent: MechanicalTextIntent::ClosureThread,
                    editable: props.editable,
                }
                label { class: "m1-mechanical-field",
                    span { "Screw head profile" }
                    select {
                        disabled: !props.editable,
                        value: if hardware.head_profile == ScrewHeadProfile::Countersunk { "countersunk" } else { "flat" },
                        onchange: {
                            let mut sequence = request_sequence;
                            let identity = identity.clone();
                            move |event: FormEvent| {
                                let profile = if event.value() == "countersunk" { ScrewHeadProfile::Countersunk } else { ScrewHeadProfile::Flat };
                                send_request(&mut sequence, &identity, on_request, MechanicalSettingsPatch::SetClosureHeadProfile(profile));
                            }
                        },
                        option { value: "flat", "Flat bearing surface" }
                        option { value: "countersunk", "Countersunk" }
                    }
                }
                label { class: "m1-mechanical-field",
                    span { "Screw length measured from" }
                    select {
                        disabled: !props.editable,
                        value: match hardware.length_datum {
                            ScrewLengthDatum::UnderHead => "under-head",
                            ScrewLengthDatum::Overall => "overall",
                            ScrewLengthDatum::Unresolved => "unresolved",
                        },
                        onchange: {
                            let mut sequence = request_sequence;
                            let identity = identity.clone();
                            move |event: FormEvent| {
                                let datum = match event.value().as_str() {
                                    "overall" => ScrewLengthDatum::Overall,
                                    "unresolved" => ScrewLengthDatum::Unresolved,
                                    _ => ScrewLengthDatum::UnderHead,
                                };
                                send_request(&mut sequence, &identity, on_request, MechanicalSettingsPatch::SetClosureLengthDatum(datum));
                            }
                        },
                        option { value: "under-head", "Under the head" }
                        option { value: "overall", "Top of head · overall" }
                        option { value: "unresolved", "Unknown" }
                    }
                }
                for (field, label, value) in closure_dimension_fields(hardware) {
                    DimensionField {
                        key: "{props.owner_key}:{field.field_id()}",
                        identity: props.identity.clone(),
                        request_sequence,
                        on_request,
                        feedback: props.feedback.clone(),
                        field,
                        label,
                        value,
                        editable: props.editable,
                    }
                }
                p { class: "m1-mechanical-help", "Thread changes do not resize the insert. Countersunk lengths include the head. Incompatible dimensions appear in Mechanical diagnostics." }
            }
        }
    }
}

fn closure_dimension_fields(
    hardware: &InternalClosureHardware,
) -> [(MechanicalDimension, &'static str, f64); 18] {
    [
        (
            MechanicalDimension::ClosureThreadDiameter,
            "Thread diameter",
            hardware.thread_diameter,
        ),
        (
            MechanicalDimension::ClosurePitch,
            "Thread pitch",
            hardware.pitch,
        ),
        (
            MechanicalDimension::ClosureHeadDiameter,
            "Head diameter",
            hardware.head_diameter,
        ),
        (
            MechanicalDimension::ClosureHeadHeight,
            "Head height",
            hardware.head_height,
        ),
        (
            MechanicalDimension::ClosureHoleDiameter,
            "Screw clearance hole",
            hardware.hole_diameter,
        ),
        (
            MechanicalDimension::ClosureInsertDiameter,
            "Insert outside diameter",
            hardware.insert_diameter,
        ),
        (
            MechanicalDimension::ClosureInsertLength,
            "Insert length",
            hardware.insert_length,
        ),
        (
            MechanicalDimension::ClosureSeatDiameter,
            "Insert seat diameter",
            hardware.seat_diameter,
        ),
        (
            MechanicalDimension::ClosureSeatDepth,
            "Insert seat depth",
            hardware.seat_depth,
        ),
        (
            MechanicalDimension::ClosureEngagement,
            "Thread engagement",
            hardware.engagement,
        ),
        (
            MechanicalDimension::ClosureThreadStart,
            "Thread start offset",
            hardware.thread_start,
        ),
        (
            MechanicalDimension::ClosureTipAllowance,
            "Screw tip allowance",
            hardware.tip_allowance,
        ),
        (
            MechanicalDimension::ClosureBottomingClearance,
            "Bottoming clearance",
            hardware.bottoming_clearance,
        ),
        (
            MechanicalDimension::ClosureRoof,
            "Roof above insert",
            hardware.roof,
        ),
        (
            MechanicalDimension::ClosureSurround,
            "Material around insert",
            hardware.surround,
        ),
        (
            MechanicalDimension::ClosureSeatLeadDepth,
            "Seat lead-in depth",
            hardware.seat_lead_depth,
        ),
        (
            MechanicalDimension::ClosureSeatLeadDiameter,
            "Seat lead-in diameter",
            hardware.seat_lead_diameter,
        ),
        (
            MechanicalDimension::ClosureBearingThickness,
            "Material above screw head",
            hardware.bearing_thickness,
        ),
    ]
}

#[derive(Props, Clone, PartialEq)]
struct CriticalFitControlsProps {
    identity: MechanicalSettingsIdentity,
    request_sequence: Signal<u64>,
    hardware: Vec<MechanicalHardwareSpecification>,
    hardware_mounts: Rc<[MechanicalHardwareMount]>,
    fits: Vec<MechanicalCriticalFit>,
    fit_parts: Rc<[MechanicalFitPart]>,
    fit_parts_resolved: bool,
    values: MechanicalSettingsValues,
    editable: bool,
    feedback: Rc<[MechanicalSettingsFeedback]>,
    on_request: EventHandler<MechanicalSettingsRequest>,
    owner_key: String,
}

#[component]
fn CriticalFitControls(props: CriticalFitControlsProps) -> Element {
    let parts = critical_fit_parts(&props.values, &props.fit_parts, props.fit_parts_resolved);
    let mount_choices = hardware_mount_choices(&props.values, &props.hardware_mounts);
    let hardware = &props.hardware;
    let fits = &props.fits;
    let fit_rows = fits
        .iter()
        .cloned()
        .map(|fit| {
            let length = ((fit.to.x - fit.from.x).powi(2) + (fit.to.y - fit.from.y).powi(2)).sqrt();
            (fit, length)
        })
        .collect::<Vec<_>>();
    let request_sequence = props.request_sequence;
    let identity = props.identity.clone();
    let on_request = props.on_request;
    rsx! {
        details { class: "m1-mechanical-group", open: !hardware.is_empty() || !fits.is_empty(), aria_label: "Hardware and critical fits",
            summary { "Hardware & critical fits · {hardware.len()} hardware · {fits.len()} fits" }
            div { class: "m1-mechanical-spec-group",
                header {
                    strong { "Hardware" }
                    button {
                        r#type: "button",
                        class: "m1-mechanical-quiet",
                        disabled: mechanical_action_pending("hardware:add") || !props.editable || mount_choices.is_empty(),
                        onclick: {
                            let mut sequence = request_sequence;
                            let identity = identity.clone();
                            let on_request = on_request;
                            let choice = mount_choices.first().cloned();
                            move |_| if let Some(choice) = choice.as_ref() {
                                send_request(&mut sequence, &identity, on_request, MechanicalSettingsPatch::AddHardware {
                                    part_id: choice.part_id.clone(), feature_id: choice.feature_id.clone(),
                                });
                            }
                        },
                        "Add hardware"
                    }
                }
                if mount_choices.is_empty() {
                    p { class: "m1-mechanical-help", "Add a suspension or closure mount to link a fastener specification to its generated feature." }
                }
                for item in hardware.iter() {
                    article { class: "m1-mechanical-spec-card", key: "{props.owner_key}:hardware:{item.id}",
                        header {
                            strong { if item.designation.is_empty() { "Hardware item" } else { "{item.designation}" } }
                            button {
                                r#type: "button",
                                class: "m1-mechanical-quiet",
                                disabled: mechanical_action_pending(&format!("hardware:{}:remove", item.id)) || !props.editable,
                                onclick: {
                                    let mut sequence = request_sequence;
                                    let identity = identity.clone();
                                    let hardware_id = item.id.clone();
                                    move |_| send_request(&mut sequence, &identity, on_request, MechanicalSettingsPatch::RemoveHardware { hardware_id: hardware_id.clone() })
                                },
                                "Remove"
                            }
                        }
                        label { class: "m1-mechanical-field",
                            span { "Mount link" }
                            select {
                                aria_label: "Mount link for hardware {item.id}",
                                disabled: !props.editable,
                                value: mount_choices.iter().position(|choice| choice.part_id == item.part_id && choice.feature_id == item.feature_id).map(|index| index.to_string()).unwrap_or_default(),
                                onchange: {
                                    let mut sequence = request_sequence;
                                    let identity = identity.clone();
                                    let hardware_id = item.id.clone();
                                    let mount_choices = mount_choices.clone();
                                    move |event: FormEvent| if let Ok(index) = event.value().parse::<usize>()
                                        && let Some(choice) = mount_choices.get(index) {
                                            send_request(&mut sequence, &identity, on_request, MechanicalSettingsPatch::SetHardwareMount {
                                                hardware_id: hardware_id.clone(),
                                                part_id: choice.part_id.clone(),
                                                feature_id: choice.feature_id.clone(),
                                            });
                                        }
                                },
                                option { value: "", "Choose generated mount…" }
                                for (index, choice) in mount_choices.iter().enumerate() {
                                    option { key: "{choice.part_id}:{choice.feature_id}", value: "{index}", "{choice.part_name} · {mount_kind_label(&choice.kind)} · {choice.x:.1}, {choice.y:.1} mm" }
                                }
                            }
                        }
                        TextDraftField {
                            key: "{props.owner_key}:hardware:{item.id}:designation",
                            identity: props.identity.clone(), request_sequence, on_request, feedback: props.feedback.clone(),
                            label: "Designation", value: item.designation.clone(),
                            intent: MechanicalTextIntent::Hardware { hardware_id: item.id.clone(), field: MechanicalHardwareTextField::Designation },
                            editable: props.editable,
                        }
                        TextDraftField {
                            key: "{props.owner_key}:hardware:{item.id}:thread",
                            identity: props.identity.clone(), request_sequence, on_request, feedback: props.feedback.clone(),
                            label: "Thread", value: item.thread.clone(),
                            intent: MechanicalTextIntent::Hardware { hardware_id: item.id.clone(), field: MechanicalHardwareTextField::Thread },
                            editable: props.editable,
                        }
                        div { class: "m1-mechanical-fit-points",
                            DimensionField {
                                key: "{props.owner_key}:hardware:{item.id}:length",
                                identity: props.identity.clone(), request_sequence, on_request, feedback: props.feedback.clone(),
                                field: MechanicalDimension::HardwareLength, label: "Length", value: item.length,
                                editable: props.editable, hardware_target: Some(HardwareDimensionTarget { hardware_id: item.id.clone() }),
                            }
                            DimensionField {
                                key: "{props.owner_key}:hardware:{item.id}:quantity",
                                identity: props.identity.clone(), request_sequence, on_request, feedback: props.feedback.clone(),
                                field: MechanicalDimension::HardwareQuantity, label: "Quantity", value: f64::from(item.quantity),
                                editable: props.editable, hardware_target: Some(HardwareDimensionTarget { hardware_id: item.id.clone() }),
                            }
                        }
                        TextDraftField {
                            key: "{props.owner_key}:hardware:{item.id}:tolerance",
                            identity: props.identity.clone(), request_sequence, on_request, feedback: props.feedback.clone(),
                            label: "Tolerance", value: item.tolerance.clone().unwrap_or_default(),
                            intent: MechanicalTextIntent::Hardware { hardware_id: item.id.clone(), field: MechanicalHardwareTextField::Tolerance },
                            editable: props.editable,
                        }
                        TextDraftField {
                            key: "{props.owner_key}:hardware:{item.id}:notes",
                            identity: props.identity.clone(), request_sequence, on_request, feedback: props.feedback.clone(),
                            label: "Notes", value: item.notes.clone().unwrap_or_default(),
                            intent: MechanicalTextIntent::Hardware { hardware_id: item.id.clone(), field: MechanicalHardwareTextField::Notes },
                            editable: props.editable, multiline: true,
                        }
                    }
                }
            }
            div { class: "m1-mechanical-spec-group",
                header {
                    strong { "Critical fits" }
                    button {
                        r#type: "button",
                        class: "m1-mechanical-quiet",
                        disabled: mechanical_action_pending("critical-fit:add") || !props.editable,
                        onclick: {
                            let mut sequence = request_sequence;
                            let identity = identity.clone();
                            let part_id = parts.first().map_or_else(|| "plate".to_owned(), |part| part.id.clone());
                            move |_| send_request(&mut sequence, &identity, on_request, MechanicalSettingsPatch::AddCriticalFit { part_id: part_id.clone() })
                        },
                        "Add fit dimension"
                    }
                }
                p { class: "m1-mechanical-help", "Endpoints are document XY coordinates. Length is calculated from the saved endpoints." }
                for (fit, measured_length) in fit_rows.iter() {
                    article { class: "m1-mechanical-spec-card", key: "{props.owner_key}:fit:{fit.id}",
                        header {
                            strong { if fit.label.is_empty() { "Fit dimension" } else { "{fit.label}" } }
                            button {
                                r#type: "button",
                                class: "m1-mechanical-quiet",
                                disabled: mechanical_action_pending(&format!("critical-fit:{}:remove", fit.id)) || !props.editable,
                                onclick: {
                                    let mut sequence = request_sequence;
                                    let identity = identity.clone();
                                    let fit_id = fit.id.clone();
                                    move |_| send_request(&mut sequence, &identity, on_request, MechanicalSettingsPatch::RemoveCriticalFit { fit_id: fit_id.clone() })
                                },
                                "Remove"
                            }
                        }
                        label { class: "m1-mechanical-field",
                            span { "Generated part" }
                            select {
                                aria_label: "Part for critical fit {fit.id}",
                                disabled: !props.editable,
                                value: "{fit.part_id}",
                                onchange: {
                                    let mut sequence = request_sequence;
                                    let identity = identity.clone();
                                    let fit_id = fit.id.clone();
                                    move |event: FormEvent| send_request(
                                        &mut sequence,
                                        &identity,
                                        on_request,
                                        MechanicalSettingsPatch::SetCriticalFitPart { fit_id: fit_id.clone(), part_id: event.value() },
                                    )
                                },
                                for part in parts.iter() {
                                    option { value: "{part.id}", "{part.name}" }
                                }
                            }
                        }
                        TextDraftField {
                            key: "{props.owner_key}:fit:{fit.id}:label",
                            identity: props.identity.clone(),
                            request_sequence,
                            on_request,
                            feedback: props.feedback.clone(),
                            label: "Dimension name",
                            value: fit.label.clone(),
                            intent: MechanicalTextIntent::CriticalFit { fit_id: fit.id.clone(), field: MechanicalCriticalFitTextField::Label },
                            editable: props.editable,
                        }
                        TextDraftField {
                            key: "{props.owner_key}:fit:{fit.id}:tolerance",
                            identity: props.identity.clone(),
                            request_sequence,
                            on_request,
                            feedback: props.feedback.clone(),
                            label: "Tolerance",
                            value: fit.tolerance.clone(),
                            intent: MechanicalTextIntent::CriticalFit { fit_id: fit.id.clone(), field: MechanicalCriticalFitTextField::Tolerance },
                            editable: props.editable,
                        }
                        div { class: "m1-mechanical-fit-points",
                            DimensionField {
                                key: "{props.owner_key}:fit:{fit.id}:from-x",
                                identity: props.identity.clone(),
                                request_sequence,
                                on_request,
                                feedback: props.feedback.clone(),
                                field: MechanicalDimension::CriticalFitFromX,
                                label: "From X",
                                value: fit.from.x,
                                editable: props.editable,
                                accessible_label: Some(format!("Critical fit {} from X", fit.id)),
                                critical_fit_target: Some(CriticalFitDimensionTarget { fit_id: fit.id.clone() }),
                            }
                            DimensionField {
                                key: "{props.owner_key}:fit:{fit.id}:from-y",
                                identity: props.identity.clone(),
                                request_sequence,
                                on_request,
                                feedback: props.feedback.clone(),
                                field: MechanicalDimension::CriticalFitFromY,
                                label: "From Y",
                                value: fit.from.y,
                                editable: props.editable,
                                accessible_label: Some(format!("Critical fit {} from Y", fit.id)),
                                critical_fit_target: Some(CriticalFitDimensionTarget { fit_id: fit.id.clone() }),
                            }
                            DimensionField {
                                key: "{props.owner_key}:fit:{fit.id}:to-x",
                                identity: props.identity.clone(),
                                request_sequence,
                                on_request,
                                feedback: props.feedback.clone(),
                                field: MechanicalDimension::CriticalFitToX,
                                label: "To X",
                                value: fit.to.x,
                                editable: props.editable,
                                accessible_label: Some(format!("Critical fit {} to X", fit.id)),
                                critical_fit_target: Some(CriticalFitDimensionTarget { fit_id: fit.id.clone() }),
                            }
                            DimensionField {
                                key: "{props.owner_key}:fit:{fit.id}:to-y",
                                identity: props.identity.clone(),
                                request_sequence,
                                on_request,
                                feedback: props.feedback.clone(),
                                field: MechanicalDimension::CriticalFitToY,
                                label: "To Y",
                                value: fit.to.y,
                                editable: props.editable,
                                accessible_label: Some(format!("Critical fit {} to Y", fit.id)),
                                critical_fit_target: Some(CriticalFitDimensionTarget { fit_id: fit.id.clone() }),
                            }
                        }
                        output { class: "m1-mechanical-fit-result",
                            "Measured {measured_length:.2} mm · {fit.tolerance}"
                        }
                    }
                }
            }
        }
    }
}

fn critical_fit_parts(
    values: &MechanicalSettingsValues,
    resolved: &[MechanicalFitPart],
    is_resolved: bool,
) -> Vec<MechanicalFitPart> {
    if is_resolved {
        return resolved.to_vec();
    }
    let ids = [
        Some(("plate", "Plate")),
        (values.plate_foam_thickness > 0.0).then_some(("plate-foam", "Plate foam")),
        (values.bottom_foam_thickness > 0.0).then_some(("bottom-foam", "Bottom foam")),
        Some(("bottom", "Bottom")),
        (values.bottom_style == MechanicalBottomStyle::Sheet && values.middle_frame)
            .then_some(("middle-frame", "Middle frame")),
    ];
    ids.into_iter()
        .flatten()
        .map(|(id, name)| MechanicalFitPart {
            id: id.to_owned(),
            name: name.to_owned(),
        })
        .collect()
}

fn hardware_mount_choices(
    values: &MechanicalSettingsValues,
    resolved: &[MechanicalHardwareMount],
) -> Vec<MechanicalHardwareMount> {
    if !resolved.is_empty() {
        return resolved.to_vec();
    }
    let mut choices = Vec::new();
    if values.mount != MechanicalMount::Gasket {
        let part_id = if values.mount == MechanicalMount::Rigid {
            "plate"
        } else {
            "bottom"
        };
        let part_name = if part_id == "plate" {
            "Plate"
        } else {
            "Bottom"
        };
        choices.extend(
            values
                .suspension_mounts
                .iter()
                .map(|mount| MechanicalHardwareMount {
                    part_id: part_id.to_owned(),
                    part_name: part_name.to_owned(),
                    feature_id: mount.id.clone(),
                    kind: mount.kind.clone(),
                    x: mount.at.x,
                    y: mount.at.y,
                }),
        );
    }
    choices.extend(
        values
            .closure_mounts
            .as_deref()
            .unwrap_or_default()
            .iter()
            .map(|mount| MechanicalHardwareMount {
                part_id: "bottom".to_owned(),
                part_name: "Bottom".to_owned(),
                feature_id: mount.id.clone(),
                kind: mount.kind.clone(),
                x: mount.at.x,
                y: mount.at.y,
            }),
    );
    choices
}

fn mount_kind_label(kind: &MountKind) -> &'static str {
    match kind {
        MountKind::Hole => "hole",
        MountKind::Boss => "boss",
    }
}

#[derive(Props, Clone, PartialEq)]
struct MountingControlsProps {
    identity: MechanicalSettingsIdentity,
    request_sequence: Signal<u64>,
    values: MechanicalSettingsValues,
    suggested_mounts: Rc<[Mount]>,
    editable: bool,
    feedback: Rc<[MechanicalSettingsFeedback]>,
    on_request: EventHandler<MechanicalSettingsRequest>,
}

#[component]
fn MountingControls(props: MountingControlsProps) -> Element {
    let values = &props.values;
    let suspension_mounts = &values.suspension_mounts;
    let closure_mounts = values.closure_mounts.as_deref().unwrap_or_default();
    let reserved_mount_ids = suspension_mounts
        .iter()
        .chain(closure_mounts.iter())
        .map(|mount| mount.id.clone())
        .collect::<Vec<_>>();
    let suggestions = &props.suggested_mounts;
    rsx! {
        section { class: "m1-mechanical-option-group", aria_label: "Mounting and hardware",
            h3 { "Mounting & hardware" }
            if values.mount != MechanicalMount::Gasket {
                MountCollectionControls {
                    identity: props.identity.clone(),
                    request_sequence: props.request_sequence,
                    on_request: props.on_request,
                    editable: props.editable,
                    feedback: props.feedback.clone(),
                    collection: MechanicalMountCollection::Suspension,
                    label: "Suspension mounts",
                    mounts: suspension_mounts.clone(),
                    reserved_mount_ids: reserved_mount_ids.clone(),
                    allow_add: true,
                }
            }
            MountCollectionControls {
                identity: props.identity.clone(),
                request_sequence: props.request_sequence,
                on_request: props.on_request,
                editable: props.editable,
                feedback: props.feedback.clone(),
                collection: MechanicalMountCollection::Closure,
                label: "Closure screws",
                mounts: closure_mounts.to_vec(),
                reserved_mount_ids: reserved_mount_ids.clone(),
                allow_add: !values.internal_gasket,
            }
            details { class: "m1-mechanical-group", aria_label: "Suggested mount locations",
                summary { "Suggested mount locations · {suggestions.len()} candidates" }
                if suggestions.is_empty() {
                    p { class: "m1-mechanical-help", "Resolve the current geometry to see clearance-tested mounting locations." }
                } else if values.internal_gasket {
                    p { class: "m1-mechanical-help", "Candidates clear the current openings and battery envelope. Adopting them is explicit; later edits keep the chosen coordinates fixed." }
                    button {
                        r#type: "button",
                        class: "m1-mechanical-quiet",
                        disabled: mechanical_action_pending("adopt-closure-positions") || !props.editable,
                        onclick: {
                            let mut sequence = props.request_sequence;
                            let identity = props.identity.clone();
                            let callback = props.on_request;
                            let mounts = suggestions.to_vec();
                            move |_| send_request(&mut sequence, &identity, callback, MechanicalSettingsPatch::AdoptClosurePositions(mounts.clone()))
                        },
                        "Adopt closure positions"
                    }
                } else {
                    p { class: "m1-mechanical-help", "Clearance-tested positions on this board. Adopt them as suspension mounts or closure screws." }
                    div { class: "m1-mechanical-mount-candidates",
                        for (index, mount) in suggestions.iter().enumerate() {
                            span { key: "{mount.id}", "{index + 1}: X {mount.at.x:.1}, Y {mount.at.y:.1} mm" }
                        }
                    }
                    button {
                        r#type: "button",
                        class: "m1-mechanical-quiet",
                        disabled: mechanical_action_pending("suspension-mounts") || !props.editable,
                        onclick: {
                            let mut sequence = props.request_sequence;
                            let identity = props.identity.clone();
                            let callback = props.on_request;
                            let mounts = suggestions.to_vec();
                            move |_| send_request(&mut sequence, &identity, callback, MechanicalSettingsPatch::SetMountCollection { collection: MechanicalMountCollection::Suspension, mounts: mounts.clone() })
                        },
                        "Adopt suggested mounts"
                    }
                    button {
                        r#type: "button",
                        class: "m1-mechanical-quiet",
                        disabled: mechanical_action_pending("closure-mounts") || !props.editable,
                        onclick: {
                            let mut sequence = props.request_sequence;
                            let identity = props.identity.clone();
                            let callback = props.on_request;
                            let suggestions = suggestions.to_vec();
                            move |_| send_request(&mut sequence, &identity, callback, MechanicalSettingsPatch::AddSuggestedClosureMounts(suggestions.clone()))
                        },
                        "Add suggested closure screws"
                    }
                }
            }
        }
    }
}

#[derive(Props, Clone, PartialEq)]
struct MountCollectionControlsProps {
    identity: MechanicalSettingsIdentity,
    request_sequence: Signal<u64>,
    on_request: EventHandler<MechanicalSettingsRequest>,
    editable: bool,
    feedback: Rc<[MechanicalSettingsFeedback]>,
    collection: MechanicalMountCollection,
    label: &'static str,
    mounts: Vec<Mount>,
    reserved_mount_ids: Vec<String>,
    allow_add: bool,
}

#[component]
fn MountCollectionControls(props: MountCollectionControlsProps) -> Element {
    let mounts = props.mounts.clone();
    rsx! {
        details { class: "m1-mechanical-group", aria_label: "{props.label}",
            summary { "{props.label} · {mounts.len()}" }
            for (index, mount) in mounts.iter().enumerate() {
                MountRow {
                    key: "{mount.id}",
                    identity: props.identity.clone(),
                    request_sequence: props.request_sequence,
                    on_request: props.on_request,
                    feedback: props.feedback.clone(),
                    editable: props.editable,
                    collection: props.collection,
                    index: index + 1,
                    mount: mount.clone(),
                }
            }
            if mounts.is_empty() { p { class: "m1-mechanical-help", "No {props.label.to_lowercase()} configured." } }
            if props.allow_add {
                button {
                    r#type: "button",
                    class: "m1-mechanical-quiet",
                    disabled: mechanical_action_pending(&format!("{}-mounts", mount_collection_id(props.collection))) || !props.editable,
                    onclick: {
                        let mut sequence = props.request_sequence;
                        let identity = props.identity.clone();
                        let callback = props.on_request;
                        let collection = props.collection;
                        move |_| send_request(&mut sequence, &identity, callback, MechanicalSettingsPatch::AddMount { collection })
                    },
                    "Add {props.label.trim_end_matches('s').to_lowercase()}"
                }
            }
        }
    }
}

#[derive(Props, Clone, PartialEq)]
struct MountRowProps {
    identity: MechanicalSettingsIdentity,
    request_sequence: Signal<u64>,
    on_request: EventHandler<MechanicalSettingsRequest>,
    feedback: Rc<[MechanicalSettingsFeedback]>,
    editable: bool,
    collection: MechanicalMountCollection,
    index: usize,
    mount: Mount,
}

#[component]
fn MountRow(props: MountRowProps) -> Element {
    let mount = &props.mount;
    let label = if mount.kind == MountKind::Boss {
        "Boss"
    } else {
        "Hole"
    };
    let field_prefix = format!("{:?}:{}", props.collection, mount.id);
    let make_target = |id: &str| MountDimensionTarget {
        collection: props.collection,
        mount_id: id.to_owned(),
    };
    rsx! {
        fieldset { class: "m1-mechanical-mount-row", disabled: !props.editable,
            legend { "{label} {props.index}" }
            label { class: "m1-mechanical-field",
                span { "Mount type" }
                select {
                    value: if mount.kind == MountKind::Boss { "boss" } else { "hole" },
                    onchange: {
                        let mut sequence = props.request_sequence;
                        let identity = props.identity.clone();
                        let callback = props.on_request;
                        let collection = props.collection;
                        let mount_id = mount.id.clone();
                        move |event: FormEvent| if let Some(kind) = parse_mount_kind(&event.value()) {
                            send_request(&mut sequence, &identity, callback, MechanicalSettingsPatch::SetMountKind { collection, mount_id: mount_id.clone(), kind });
                        }
                    },
                    option { value: "hole", "Hole" }
                    option { value: "boss", "Boss" }
                }
            }
            DimensionField { key: "{field_prefix}:x",
                identity: props.identity.clone(), request_sequence: props.request_sequence,
                on_request: props.on_request, feedback: props.feedback.clone(), field: MechanicalDimension::MountPositionX,
                label: "Position X", value: mount.at.x, editable: props.editable, mount_target: Some(make_target(&mount.id)),
            }
            DimensionField { key: "{field_prefix}:y",
                identity: props.identity.clone(), request_sequence: props.request_sequence,
                on_request: props.on_request, feedback: props.feedback.clone(), field: MechanicalDimension::MountPositionY,
                label: "Position Y", value: mount.at.y, editable: props.editable, mount_target: Some(make_target(&mount.id)),
            }
            DimensionField { key: "{field_prefix}:hole",
                identity: props.identity.clone(), request_sequence: props.request_sequence,
                on_request: props.on_request, feedback: props.feedback.clone(), field: MechanicalDimension::MountHoleDiameter,
                label: "Hole diameter", value: mount.hole_diameter, editable: props.editable, mount_target: Some(make_target(&mount.id)),
            }
            DimensionField { key: "{field_prefix}:boss",
                identity: props.identity.clone(), request_sequence: props.request_sequence,
                on_request: props.on_request, feedback: props.feedback.clone(), field: MechanicalDimension::MountBossDiameter,
                label: "Boss diameter", value: mount.boss_diameter.unwrap_or(5.0), editable: props.editable, mount_target: Some(make_target(&mount.id)),
            }
            if mount.kind == MountKind::Boss {
                DimensionField { key: "{field_prefix}:height",
                    identity: props.identity.clone(), request_sequence: props.request_sequence,
                    on_request: props.on_request, feedback: props.feedback.clone(), field: MechanicalDimension::MountBossHeight,
                    label: "Boss height", value: mount.height.unwrap_or(5.0), editable: props.editable, mount_target: Some(make_target(&mount.id)),
                }
            }
            button {
                r#type: "button",
                class: "m1-mechanical-quiet",
                disabled: mechanical_action_pending(&format!("mount:{}:{}:remove", mount_collection_id(props.collection), mount.id)) || !props.editable,
                onclick: {
                    let mut sequence = props.request_sequence;
                    let identity = props.identity.clone();
                    let callback = props.on_request;
                    let collection = props.collection;
                    let mount_id = mount.id.clone();
                    move |_| send_request(&mut sequence, &identity, callback, MechanicalSettingsPatch::RemoveMount { collection, mount_id: mount_id.clone() })
                },
                "Remove"
            }
        }
    }
}

#[derive(Props, Clone, PartialEq)]
struct BatteryControlsProps {
    identity: MechanicalSettingsIdentity,
    request_sequence: Signal<u64>,
    values: MechanicalSettingsValues,
    editable: bool,
    feedback: Rc<[MechanicalSettingsFeedback]>,
    on_request: EventHandler<MechanicalSettingsRequest>,
    owner_key: String,
}

#[component]
fn BatteryControls(props: BatteryControlsProps) -> Element {
    let battery = props.values.battery.as_ref();
    let request_sequence = props.request_sequence;
    rsx! {
        section { class: "m1-mechanical-option-group", aria_label: "Openings and battery",
            h3 { "Openings & battery" }
            section { class: "m1-mechanical-group", aria_label: "Battery",
                h4 { "Battery" }
                if props.values.transport == HardwareTransport::Wireless {
                    p { class: "m1-mechanical-help", "Included for wireless. Set your cell dimensions; the battery shares the space below the PCB with bottom foam." }
                } else {
                    label { class: "m1-mechanical-check m1-mechanical-battery-toggle",
                        input {
                            r#type: "checkbox",
                            aria_label: "Include a battery envelope",
                            checked: battery.is_some(),
                            disabled: !props.editable,
                            onchange: {
                                let identity = props.identity.clone();
                                let on_request = props.on_request;
                                let mut sequence = request_sequence;
                                move |event: FormEvent| send_request(
                                    &mut sequence,
                                    &identity,
                                    on_request,
                                    MechanicalSettingsPatch::SetBatteryEnabled(event.checked()),
                                )
                            }
                        }
                        span { "Include a battery envelope" }
                    }
                }
                if let Some(battery) = battery {
                    fieldset { class: "m1-mechanical-group m1-mechanical-battery-fields", disabled: !props.editable,
                        legend { "Battery dimensions and cable · mm" }
                        DimensionField {
                            key: "{props.owner_key}:battery-width",
                            identity: props.identity.clone(),
                            request_sequence,
                            on_request: props.on_request,
                            feedback: props.feedback.clone(),
                            field: MechanicalDimension::BatteryWidth,
                            label: "Width",
                            value: battery.size.x,
                            editable: props.editable,
                        }
                        DimensionField {
                            key: "{props.owner_key}:battery-depth",
                            identity: props.identity.clone(),
                            request_sequence,
                            on_request: props.on_request,
                            feedback: props.feedback.clone(),
                            field: MechanicalDimension::BatteryDepth,
                            label: "Depth",
                            value: battery.size.y,
                            editable: props.editable,
                        }
                        DimensionField {
                            key: "{props.owner_key}:battery-height",
                            identity: props.identity.clone(),
                            request_sequence,
                            on_request: props.on_request,
                            feedback: props.feedback.clone(),
                            field: MechanicalDimension::BatteryHeight,
                            label: "Height",
                            value: battery.size.z,
                            editable: props.editable,
                        }
                        DimensionField {
                            key: "{props.owner_key}:battery-cable-width",
                            identity: props.identity.clone(),
                            request_sequence,
                            on_request: props.on_request,
                            feedback: props.feedback.clone(),
                            field: MechanicalDimension::BatteryCableWidth,
                            label: "Cable width",
                            value: battery.cable_width.unwrap_or(2.0),
                            editable: props.editable,
                        }
                        DimensionField {
                            key: "{props.owner_key}:battery-position-x",
                            identity: props.identity.clone(),
                            request_sequence,
                            on_request: props.on_request,
                            feedback: props.feedback.clone(),
                            field: MechanicalDimension::BatteryPositionX,
                            label: "Position X",
                            value: battery.at.x,
                            editable: props.editable,
                        }
                        DimensionField {
                            key: "{props.owner_key}:battery-position-y",
                            identity: props.identity.clone(),
                            request_sequence,
                            on_request: props.on_request,
                            feedback: props.feedback.clone(),
                            field: MechanicalDimension::BatteryPositionY,
                            label: "Position Y",
                            value: battery.at.y,
                            editable: props.editable,
                        }
                        DimensionField {
                            key: "{props.owner_key}:battery-cable-exit-x",
                            identity: props.identity.clone(),
                            request_sequence,
                            on_request: props.on_request,
                            feedback: props.feedback.clone(),
                            field: MechanicalDimension::BatteryCableExitX,
                            label: "Cable exit X",
                            value: battery.cable_exit.x,
                            editable: props.editable,
                        }
                        DimensionField {
                            key: "{props.owner_key}:battery-cable-exit-y",
                            identity: props.identity.clone(),
                            request_sequence,
                            on_request: props.on_request,
                            feedback: props.feedback.clone(),
                            field: MechanicalDimension::BatteryCableExitY,
                            label: "Cable exit Y",
                            value: battery.cable_exit.y,
                            editable: props.editable,
                        }
                    }
                }
            }
        }
    }
}

#[derive(Props, Clone, PartialEq)]
struct OpeningControlsProps {
    identity: MechanicalSettingsIdentity,
    request_sequence: Signal<u64>,
    values: MechanicalSettingsValues,
    editable: bool,
    feedback: Rc<[MechanicalSettingsFeedback]>,
    on_request: EventHandler<MechanicalSettingsRequest>,
    owner_key: String,
}

#[component]
fn OpeningControls(props: OpeningControlsProps) -> Element {
    let openings = &props.values.openings;
    let identity = props.identity.clone();
    let request_sequence = props.request_sequence;
    let on_request = props.on_request;
    let owner_key = props.owner_key.clone();
    rsx! {
        details { class: "m1-mechanical-group", aria_label: "Case openings",
            summary { "Case openings · {openings.len()}" }
            div { class: "m1-mechanical-openings",
                div { class: "m1-mechanical-opening-title",
                    strong { "Document-coordinate access openings" }
                    button {
                        r#type: "button",
                        class: "m1-mechanical-quiet",
                        disabled: mechanical_action_pending("case-openings") || !props.editable,
                        onclick: {
                            let mut sequence = request_sequence;
                            let identity = identity.clone();
                            move |_| {
                                send_request(&mut sequence, &identity, on_request, MechanicalSettingsPatch::AddOpening);
                            }
                        },
                        "Add volume"
                    }
                }
                if openings.is_empty() {
                    p { class: "m1-mechanical-help", "No volumes configured." }
                }
                for (opening_index, opening) in openings.iter().enumerate() {
                    {
                        let mut sequence = request_sequence;
                        let identity = identity.clone();
                        let remove_identity = identity.clone();
                        let owner = owner_key.clone();
                        let opening_id = opening.id.clone();
                        rsx! {
                            fieldset { class: "m1-mechanical-opening", key: "{owner}:opening:{opening_id}", disabled: !props.editable,
                                div { class: "m1-mechanical-opening-title",
                                    strong { "Volume {opening_index + 1}" }
                                    button {
                                        r#type: "button",
                                        class: "m1-mechanical-quiet",
                                        disabled: mechanical_action_pending("case-openings") || !props.editable,
                                        onclick: {
                                            let opening_id = opening.id.clone();
                                            move |_| send_request(&mut sequence, &remove_identity, on_request, MechanicalSettingsPatch::RemoveOpening { opening_id: opening_id.clone() })
                                        },
                                        "Remove"
                                    }
                                }
                                div { class: "m1-mechanical-opening-fields",
                                    DimensionField {
                                        key: "{owner}:opening:{opening_id}:bottom-z",
                                        identity: identity.clone(), request_sequence, on_request, feedback: props.feedback.clone(),
                                        field: MechanicalDimension::OpeningBottomZ, label: "Bottom Z", value: opening.z, editable: props.editable,
                                        accessible_label: Some(format!("Document-coordinate access opening {} bottom Z", opening_index + 1)),
                                        opening_target: Some(OpeningDimensionTarget { opening_id: opening_id.clone(), vertex_id: None }),
                                    }
                                    DimensionField {
                                        key: "{owner}:opening:{opening_id}:height",
                                        identity: identity.clone(), request_sequence, on_request, feedback: props.feedback.clone(),
                                        field: MechanicalDimension::OpeningHeight, label: "Height", value: opening.height, editable: props.editable,
                                        accessible_label: Some(format!("Document-coordinate access opening {} height", opening_index + 1)),
                                        opening_target: Some(OpeningDimensionTarget { opening_id: opening_id.clone(), vertex_id: None }),
                                    }
                                }
                                h4 { "XY footprint" }
                                for (point_index, vertex) in opening.vertices.iter().enumerate() {
                                    div { class: "m1-mechanical-opening-vertex",
                                        span { "Vertex {point_index + 1}" }
                                        DimensionField {
                                            key: "{owner}:opening:{opening_id}:point:{point_index}:x",
                                            identity: identity.clone(), request_sequence, on_request, feedback: props.feedback.clone(),
                                            field: MechanicalDimension::OpeningPointX, label: "X", value: vertex.point.x, editable: props.editable,
                                            accessible_label: Some(format!("Document-coordinate access opening {} vertex {} X", opening_index + 1, point_index + 1)),
                                            opening_target: Some(OpeningDimensionTarget { opening_id: opening_id.clone(), vertex_id: Some(vertex.id.clone()) }),
                                        }
                                        DimensionField {
                                            key: "{owner}:opening:{opening_id}:point:{point_index}:y",
                                            identity: identity.clone(), request_sequence, on_request, feedback: props.feedback.clone(),
                                            field: MechanicalDimension::OpeningPointY, label: "Y", value: vertex.point.y, editable: props.editable,
                                            accessible_label: Some(format!("Document-coordinate access opening {} vertex {} Y", opening_index + 1, point_index + 1)),
                                            opening_target: Some(OpeningDimensionTarget { opening_id: opening_id.clone(), vertex_id: Some(vertex.id.clone()) }),
                                        }
                                        button {
                                            r#type: "button",
                                            class: "m1-mechanical-quiet",
                                            aria_label: "Remove access opening vertex {point_index + 1}",
                                            disabled: mechanical_action_pending("case-openings") || !props.editable || opening.vertices.len() <= 3,
                                            onclick: {
                                                let mut sequence = request_sequence;
                                                let identity = identity.clone();
                                                let opening_id = opening_id.clone();
                                                let vertex_id = vertex.id.clone();
                                                move |_| send_request(&mut sequence, &identity, on_request, MechanicalSettingsPatch::RemoveOpeningVertex { opening_id: opening_id.clone(), vertex_id: vertex_id.clone() })
                                            },
                                            "×"
                                        }
                                    }
                                }
                                button {
                                    r#type: "button",
                                    class: "m1-mechanical-quiet",
                                    disabled: mechanical_action_pending("case-openings") || !props.editable,
                                    onclick: {
                                        let mut sequence = request_sequence;
                                        let identity = identity.clone();
                                        let opening_id = opening_id.clone();
                                        move |_| send_request(&mut sequence, &identity, on_request, MechanicalSettingsPatch::AddOpeningVertex { opening_id: opening_id.clone() })
                                    },
                                    "Add vertex"
                                }
                                if opening.vertices.len() < 3 {
                                    p { role: "alert", "An opening footprint needs at least three vertices." }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

fn contextual_layer_fields(
    id: &str,
    values: &MechanicalSettingsValues,
) -> Option<Vec<(MechanicalDimension, &'static str, f64)>> {
    Some(match id {
        "plate" => vec![(
            MechanicalDimension::PlateThickness,
            "Plate thickness",
            values.plate_thickness,
        )],
        "pcb" => vec![(
            MechanicalDimension::PcbThickness,
            "PCB thickness",
            values.pcb_thickness,
        )],
        "plate-foam" => vec![(
            MechanicalDimension::PlateFoamThickness,
            "Plate foam",
            values.plate_foam_thickness,
        )],
        "bottom-foam" => vec![(
            MechanicalDimension::BottomFoamThickness,
            "Bottom foam",
            values.bottom_foam_thickness,
        )],
        "bottom" => vec![
            (
                MechanicalDimension::BottomThickness,
                "Bottom thickness",
                values.bottom_thickness,
            ),
            (
                MechanicalDimension::WallThickness,
                "Wall thickness",
                values.wall_thickness,
            ),
            (
                MechanicalDimension::Clearance,
                "Clearance",
                values.clearance,
            ),
        ],
        "retainer" => vec![
            (
                MechanicalDimension::WallThickness,
                "Wall thickness",
                values.wall_thickness,
            ),
            (
                MechanicalDimension::Clearance,
                "Clearance",
                values.clearance,
            ),
        ],
        _ => return None,
    })
}

#[derive(Props, Clone, PartialEq)]
struct ContextualLayerInspectorProps {
    identity: MechanicalSettingsIdentity,
    request_sequence: Signal<u64>,
    values: MechanicalSettingsValues,
    layer: MechanicalLayerRow,
    editable: bool,
    disabled_reason: Option<String>,
    feedback: Rc<[MechanicalSettingsFeedback]>,
    findings: Rc<[MechanicalFindingRow]>,
    on_request: EventHandler<MechanicalSettingsRequest>,
    on_select_layer: EventHandler<String>,
    on_show_finding: EventHandler<String>,
    owner_key: String,
}

#[component]
fn ContextualLayerInspector(props: ContextualLayerInspectorProps) -> Element {
    let title = match props.layer.id.as_str() {
        "bottom" => "Bottom case",
        "retainer" => "Top case",
        _ => props.layer.label.as_str(),
    };
    let fields = contextual_layer_fields(&props.layer.id, &props.values).unwrap_or_default();
    let errors: Rc<[MechanicalFindingRow]> = Rc::from(
        props
            .findings
            .iter()
            .filter(|finding| finding.severity == Severity::Error)
            .cloned()
            .collect::<Vec<_>>(),
    );
    rsx! {
        section { class: "m1-mechanical-settings", aria_label: "Mechanical stack settings",
            button {
                r#type: "button",
                class: "m1-mechanical-context-return",
                onclick: move |_| props.on_select_layer.call(String::new()),
                "Assembly settings"
            }
            h2 { "{title}" }
            if props.layer.is_previous {
                p { role: "status", class: "m1-mechanical-previous-result",
                    "Showing previous generated geometry. Measurements describe the prior accepted revision; settings below reflect the current configuration."
                }
            }
            fieldset { class: "m1-mechanical-group", disabled: !props.editable,
                for (field, label, value) in fields {
                    DimensionField {
                        key: "{props.owner_key}:context:{props.layer.id}:{field.field_id()}",
                        identity: props.identity.clone(),
                        request_sequence: props.request_sequence,
                        on_request: props.on_request,
                        feedback: props.feedback.clone(),
                        field,
                        label,
                        value,
                        editable: props.editable,
                    }
                }
                if let Some(reason) = props.disabled_reason.as_deref() {
                    p { role: "status", "{reason}" }
                }
            }
            if let Some(thickness) = props.layer.resolved_body_thickness {
                p { class: "m1-mechanical-help", "Resolved thickness {thickness:.2} mm." }
            }
            if props.values.internal_gasket && matches!(props.layer.id.as_str(), "retainer" | "bottom") {
                button {
                    r#type: "button",
                    class: "m1-mechanical-context-hardware",
                    onclick: move |_| props.on_select_layer.call(String::new()),
                    "Edit shared closure hardware"
                }
            }
            if !errors.is_empty() {
                section { class: "m1-mechanical-group", aria_label: "Fit issues",
                    h3 { "Fit issues" }
                    ul {
                        for finding in errors.iter() {
                            { let id = finding.id.clone();
                            let message = finding.message.clone();
                            let on_show_finding = props.on_show_finding;
                            rsx! { li { key: "{id}",
                                span { "{message}" }
                                button {
                                    r#type: "button",
                                    onclick: move |_| on_show_finding.call(id.clone()),
                                    "Show"
                                }
                            } }
                            }
                        }
                    }
                }
            }
        }
    }
}

fn gasket_fit_issues(
    findings: Rc<[MechanicalFindingRow]>,
    on_show_finding: EventHandler<String>,
) -> Element {
    let errors: Vec<_> = findings
        .iter()
        .filter(|finding| finding.severity == Severity::Error)
        .cloned()
        .collect();
    if errors.is_empty() {
        return rsx! {};
    }
    rsx! {
        section { class: "m1-mechanical-group", aria_label: "Fit issues",
            h3 { "Fit issues" }
            ul {
                for finding in errors {
                    { let id = finding.id.clone();
                      let message = finding.message.clone();
                      rsx! { li { key: "{id}",
                          span { "{message}" }
                          button {
                              r#type: "button",
                              onclick: move |_| on_show_finding.call(id.clone()),
                              "Show"
                          }
                      } }
                    }
                }
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum NumberRule {
    Nonnegative,
    Bounded(i8, i8),
    AtLeastTenth,
    AtLeastOne,
    AtLeastFive,
    AtLeastHalf,
    Coordinate,
}

impl NumberRule {
    fn minimum(self) -> &'static str {
        match self {
            Self::Nonnegative => "0",
            Self::Bounded(-1, 1) => "-1",
            Self::Bounded(_, _) => "0",
            Self::AtLeastTenth => "0.1",
            Self::AtLeastOne => "1",
            Self::AtLeastFive => "5",
            Self::AtLeastHalf => "0.5",
            Self::Coordinate => "-1000000",
        }
    }

    fn error(self) -> &'static str {
        match self {
            Self::Nonnegative => "Enter a finite value of zero or greater.",
            Self::AtLeastTenth => "Enter a finite value of 0.1 mm or greater.",
            Self::AtLeastOne => "Enter a finite whole number of at least 1.",
            Self::AtLeastFive => "Enter a cut length of at least 5 mm.",
            Self::AtLeastHalf => "Enter a pad width of at least 0.5 mm.",
            Self::Coordinate => "Enter a finite value of −1,000,000 mm or greater.",
            Self::Bounded(min, max) => {
                if min == -1 && max == 1 {
                    "Enter a finite value from -1 to 1 mm."
                } else {
                    "Enter a finite value within the supported range."
                }
            }
        }
    }

    fn valid(self, value: f64) -> bool {
        value.is_finite()
            && match self {
                Self::Nonnegative => value >= 0.0,
                Self::AtLeastTenth => value >= 0.1,
                Self::AtLeastOne => value >= 1.0 && value.fract() == 0.0,
                Self::AtLeastFive => value >= 5.0,
                Self::AtLeastHalf => value >= 0.5,
                Self::Coordinate => value >= -1_000_000.0,
                Self::Bounded(min, max) => value >= f64::from(min) && value <= f64::from(max),
            }
    }
}

#[derive(Props, Clone, PartialEq)]
struct ConstructionControlsProps {
    identity: MechanicalSettingsIdentity,
    values: MechanicalSettingsValues,
    editable: bool,
    request_sequence: Signal<u64>,
    on_request: EventHandler<MechanicalSettingsRequest>,
}

#[derive(Props, Clone, PartialEq)]
struct ManufacturingControlsProps {
    identity: MechanicalSettingsIdentity,
    targets: Rc<[MechanicalProcessTarget]>,
    stabilizers: Rc<[MechanicalStabilizerFit]>,
    editable: bool,
    feedback: Rc<[MechanicalSettingsFeedback]>,
    request_sequence: Signal<u64>,
    on_request: EventHandler<MechanicalSettingsRequest>,
}

#[component]
fn ManufacturingControls(props: ManufacturingControlsProps) -> Element {
    let version = "2026-09-24";
    rsx! {
        section { class: "m1-mechanical-group", aria_label: "Manufacturing overrides",
            h3 { "Manufacturing overrides" }
            section { class: "m1-mechanical-profile", aria_label: "Per-part process overrides",
                h4 { "Per-part process overrides {props.targets.iter().filter(|target| target.process.is_some()).count()}" }
                for target in props.targets.iter() {
                    div { class: "m1-mechanical-profile", key: "{target.part_id}",
                        strong { "{target.name}" }
                        if let Some(process) = target.process.as_ref() {
                            button {
                                r#type: "button",
                                class: "m1-mechanical-context-settings",
                                disabled: mechanical_action_pending(&format!("process:{}:remove", target.part_id)) || !props.editable,
                                onclick: {
                                    let part_id = target.part_id.clone();
                                    let identity = props.identity.clone();
                                    let mut sequence = props.request_sequence;
                                    let on_request = props.on_request;
                                    move |_| send_request(&mut sequence, &identity, on_request, MechanicalSettingsPatch::RemoveProcessOverride { part_id: part_id.clone() })
                                },
                                "Remove"
                            }
                            div { class: "m1-mechanical-field",
                                label {
                                    span { "Method" }
                                    if target.part_id.ends_with("foam") {
                                        output { "Cut sheet" }
                                    } else {
                                        select {
                                            aria_label: "Method",
                                            disabled: !props.editable,
                                            value: "{method_id(&process.method)}",
                                            onchange: {
                                                let part_id = target.part_id.clone();
                                                let identity = props.identity.clone();
                                                let mut sequence = props.request_sequence;
                                                let on_request = props.on_request;
                                                move |event: FormEvent| if let Some(method) = parse_method(&event.value()) {
                                                    send_request(&mut sequence, &identity, on_request, MechanicalSettingsPatch::SetProcessMethod { part_id: part_id.clone(), method });
                                                }
                                            },
                                            option { value: "pcb-fr4", "PCB FR-4" }
                                            option { value: "printed", "3D printed" }
                                            option { value: "cnc", "CNC machined" }
                                            option { value: "cut-sheet", "Cut sheet" }
                                        }
                                    }
                                }
                            }
                            div { class: "m1-mechanical-field",
                                label {
                                    span { "Material" }
                                    select {
                                        aria_label: "Material",
                                        disabled: !props.editable,
                                        value: "{process.material}",
                                        onchange: {
                                            let part_id = target.part_id.clone();
                                            let identity = props.identity.clone();
                                            let mut sequence = props.request_sequence;
                                            let on_request = props.on_request;
                                            move |event: FormEvent| send_request(&mut sequence, &identity, on_request, MechanicalSettingsPatch::SetProcessMaterial { part_id: part_id.clone(), material: event.value() })
                                        },
                                        for material in process_material_options(&target.part_id, &process.method) {
                                            option { value: material, "{material}" }
                                        }
                                    }
                                }
                            }
                            DimensionField {
                                identity: props.identity.clone(),
                                request_sequence: props.request_sequence,
                                on_request: props.on_request,
                                feedback: props.feedback.clone(),
                                field: MechanicalDimension::PlateThickness,
                                label: "Finished thickness",
                                value: process_thickness(process, target.default_thickness),
                                editable: props.editable,
                                process_target: Some(target.part_id.clone()),
                            }
                            p { class: "m1-mechanical-help", "Constraint set " code { "{process_constraint_version(process)}" }
                                if !process.constraints_version.is_empty() && process.constraints_version != version {
                                    small { role: "alert", "Unsupported constraint set. Diagnostics will block export." }
                                }
                            }
                        } else {
                            button {
                                r#type: "button",
                                class: "m1-mechanical-context-settings",
                                disabled: mechanical_action_pending(&format!("process:{}:add", target.part_id)) || !props.editable,
                                onclick: {
                                    let part_id = target.part_id.clone();
                                    let identity = props.identity.clone();
                                    let mut sequence = props.request_sequence;
                                    let on_request = props.on_request;
                                    move |_| send_request(&mut sequence, &identity, on_request, MechanicalSettingsPatch::AddProcessOverride { part_id: part_id.clone() })
                                },
                                "Add process override"
                            }
                        }
                    }
                }
                p { class: "m1-mechanical-help", "Supported constraint set: {version}." }
            }
            section { class: "m1-mechanical-profile", aria_label: "Stabilizer fit",
                h4 { "Stabilizer fit {props.stabilizers.len()} wide keys" }
                for stabilizer in props.stabilizers.iter() {
                    div { class: "m1-mechanical-profile", key: "{stabilizer.part_id}",
                        strong { "{stabilizer.reference} · {stabilizer.key_units}u" }
                        label { class: "m1-mechanical-field",
                            span { "Stabilizer" }
                            select {
                                aria_label: "Stabilizer for {stabilizer.reference}",
                                disabled: !props.editable,
                                value: "{stabilizer_kind_id(&stabilizer.current.kind)}",
                                onchange: {
                                    let mut current = stabilizer.current.clone();
                                    let identity = props.identity.clone();
                                    let mut sequence = props.request_sequence;
                                    let on_request = props.on_request;
                                    move |event: FormEvent| if let Some(kind) = parse_stabilizer_kind(&event.value()) {
                                        current.kind = kind;
                                        send_request(&mut sequence, &identity, on_request, MechanicalSettingsPatch::SetStabilizerKind(current.clone()));
                                    }
                                },
                                option { value: "pcb-mount", "Cherry PCB mount · {stabilizer.default.units}u" }
                                option { value: "none", "None" }
                                if stabilizer.current.profile.is_some() {
                                    option { value: "plate-mount", "Custom plate mount" }
                                }
                            }
                        }
                        p { class: "m1-mechanical-help", "Size and orientation follow the key layout." }
                    }
                }
                if props.stabilizers.is_empty() {
                    p { class: "m1-mechanical-help", "No keys in this layout need stabilizers." }
                }
            }
        }
    }
}

fn process_material_options(part_id: &str, method: &PlateMethod) -> &'static [&'static str] {
    if part_id.ends_with("foam") {
        &["EVA"]
    } else {
        match method {
            PlateMethod::Printed => &["PLA", "ABS"],
            PlateMethod::Cnc => &["Aluminium"],
            PlateMethod::CutSheet => &["Acrylic"],
            PlateMethod::PcbFr4 => &["FR-4"],
        }
    }
}

fn process_thickness(process: &MechanicalPartProcess, default_thickness: f64) -> f64 {
    if process.thickness >= 0.0 {
        process.thickness
    } else {
        default_thickness
    }
}

fn process_constraint_version(process: &MechanicalPartProcess) -> &str {
    if process.constraints_version.is_empty() {
        "2026-09-24"
    } else {
        &process.constraints_version
    }
}

fn stabilizer_kind_id(kind: &MechanicalStabilizerKind) -> &'static str {
    match kind {
        MechanicalStabilizerKind::PcbMount => "pcb-mount",
        MechanicalStabilizerKind::None => "none",
        MechanicalStabilizerKind::PlateMount => "plate-mount",
    }
}

fn parse_stabilizer_kind(value: &str) -> Option<MechanicalStabilizerKind> {
    Some(match value {
        "pcb-mount" => MechanicalStabilizerKind::PcbMount,
        "none" => MechanicalStabilizerKind::None,
        "plate-mount" => MechanicalStabilizerKind::PlateMount,
        _ => return None,
    })
}

#[component]
fn ConstructionControls(props: ConstructionControlsProps) -> Element {
    let values = &props.values;
    rsx! {
        fieldset { class: "m1-mechanical-group", disabled: !props.editable,
            legend { "Construction" }
            label { class: "m1-mechanical-field",
                span { "Plate method" }
                select {
                    value: "{method_id(&values.method)}",
                    onchange: {
                        let identity = props.identity.clone();
                        let mut sequence = props.request_sequence;
                        let on_request = props.on_request;
                        move |event: FormEvent| if let Some(method) = parse_method(&event.value()) {
                            send_request(&mut sequence, &identity, on_request, MechanicalSettingsPatch::SetMethod(method));
                        }
                    },
                    option { value: "pcb-fr4", "PCB FR-4" }
                    option { value: "printed", "3D printed" }
                    option { value: "cnc", "CNC machined" }
                    option { value: "cut-sheet", "Cut sheet" }
                }
            }
            label { class: "m1-mechanical-field",
                span { "Mount style" }
                select {
                    value: "{mount_id(&values.mount)}",
                    onchange: {
                        let identity = props.identity.clone();
                        let mut sequence = props.request_sequence;
                        let on_request = props.on_request;
                        move |event: FormEvent| if let Some(mount) = parse_mount(&event.value()) {
                            send_request(&mut sequence, &identity, on_request, MechanicalSettingsPatch::SetMount(mount));
                        }
                    },
                    option { value: "tray", "Tray" }
                    option { value: "rigid", "Rigid mount" }
                    option { value: "gasket", "Gasket mount" }
                }
            }
            label { class: "m1-mechanical-field",
                span { "Bottom construction" }
                select {
                    disabled: values.mount == MechanicalMount::Gasket,
                    value: "{bottom_style_id(&values.bottom_style)}",
                    onchange: {
                        let identity = props.identity.clone();
                        let mut sequence = props.request_sequence;
                        let on_request = props.on_request;
                        move |event: FormEvent| if let Some(style) = parse_bottom_style(&event.value()) {
                            send_request(&mut sequence, &identity, on_request, MechanicalSettingsPatch::SetBottomStyle(style));
                        }
                    },
                    option { value: "shell", "Tray shell" }
                    option { value: "sheet", "Flat sheet" }
                }
            }
            if values.bottom_style == MechanicalBottomStyle::Sheet {
                label { class: "m1-mechanical-check",
                    input {
                        r#type: "checkbox",
                        checked: values.middle_frame,
                        onchange: {
                            let identity = props.identity.clone();
                            let mut sequence = props.request_sequence;
                            let on_request = props.on_request;
                            move |event: FormEvent| send_request(&mut sequence, &identity, on_request, MechanicalSettingsPatch::SetMiddleFrame(event.checked()))
                        },
                    }
                    span { "Add middle frame" }
                }
            }
            label { class: "m1-mechanical-check",
                input {
                    r#type: "checkbox",
                    disabled: values.mount == MechanicalMount::Gasket,
                    checked: values.integrated_plate_frame,
                    onchange: {
                        let identity = props.identity.clone();
                        let mut sequence = props.request_sequence;
                        let on_request = props.on_request;
                        move |event: FormEvent| send_request(&mut sequence, &identity, on_request, MechanicalSettingsPatch::SetIntegratedPlateFrame(event.checked()))
                    },
                }
                span { "Integrate plate frame into case" }
            }
        }
    }
}

#[derive(Props, Clone, PartialEq)]
struct ProfileGuidanceProps {
    identity: MechanicalSettingsIdentity,
    #[props(default)]
    owner_key: String,
    #[props(default)]
    feedback: Rc<[MechanicalSettingsFeedback]>,
    profiles: Rc<[MechanicalProfileChoice]>,
    #[props(default)]
    extract_profile: Option<MechanicalProfileExtractionPort>,
    targets: Rc<[MechanicalProfileTargetChoice]>,
    plate_thickness: f64,
    plate_to_pcb: f64,
    editable: bool,
    request_sequence: Signal<u64>,
    on_request: EventHandler<MechanicalSettingsRequest>,
}

#[component]
fn ProfileGuidance(props: ProfileGuidanceProps) -> Element {
    let mut selected_family = use_signal(String::new);
    let mut selected_stabilizer = use_signal(String::new);
    rsx! {
        section { class: "m1-mechanical-group", aria_label: "Switch fit profiles",
            h3 { "Switch fit profiles" }
            if props.profiles.is_empty() {
                p { class: "m1-mechanical-help", "No explicit switch fit profile is assigned. The supported plate thickness range is unavailable until a fit profile is assigned in Parts." }
                p { "Plate underside to PCB top: {props.plate_to_pcb:.2} mm · current configured gap." }
            }
            for profile in props.profiles.iter() {
                div { class: "m1-mechanical-profile", key: "{props.owner_key}:{profile.definition_id}",
                    strong { "{profile.name}" }
                    if let Some(source) = profile.source.as_deref() { p { "Profile source: {source}." } }
                    if profile.switch_family_selectable {
                        label { class: "m1-mechanical-field",
                            span { "Switch fit family" }
                            select {
                                disabled: !props.editable,
                                value: profile.family.map(family_id).unwrap_or(""),
                                onchange: {
                                    let definition_id = profile.definition_id.clone();
                                    let identity = props.identity.clone();
                                    let mut sequence = props.request_sequence;
                                    let on_request = props.on_request;
                                    move |event: FormEvent| {
                                        if let Some(family) = parse_family(&event.value()) {
                                            send_request(&mut sequence, &identity, on_request, MechanicalSettingsPatch::SetSwitchFamily { definition_id: definition_id.clone(), family });
                                        }
                                    }
                                },
                                option { value: "", "Choose switch family…" }
                                option { value: "mx", "MX" }
                                option { value: "choc-v1", "Choc v1" }
                                option { value: "choc-v2", "Choc v2" }
                            }
                        }
                    } else if let Some(family) = profile.family {
                        p { "Switch fit: {family_label(family)}" }
                    }
                    if let Some(gap) = profile.plate_to_pcb {
                        p { "Plate underside to PCB top: {gap:.2} mm · derived from switch fit." }
                    }
                    if let Some(range) = profile.supported_thickness {
                        p { "Supported plate thickness: {range.x:.2}–{range.y:.2} mm" }
                    } else {
                        p { role: "status", "Supplier review needed for the supported thickness range." }
                    }
                    button {
                        r#type: "button",
                        aria_label: "Remove profile {profile.name}",
                        class: "m1-mechanical-quiet",
                        disabled: mechanical_action_pending(&format!("profile:{}:remove", profile.definition_id)) || !props.editable,
                        onclick: {
                            let identity = props.identity.clone();
                            let definition_id = profile.definition_id.clone();
                            let mut sequence = props.request_sequence;
                            let on_request = props.on_request;
                            move |_| send_request(&mut sequence, &identity, on_request, MechanicalSettingsPatch::RemoveProfile { definition_id: definition_id.clone() })
                        },
                        "Remove profile"
                    }
                    ProfileGeometryEditor {
                        identity: props.identity.clone(),
                        feedback: props.feedback.clone(),
                        profile: profile.profile.clone(),
                        editable: props.editable,
                        request_sequence: props.request_sequence,
                        on_request: props.on_request,
                        extract_profile: if profile.kicad_source.is_some() { props.extract_profile.clone() } else { None },
                    }
                }
            }
            if props.profiles.iter().any(|profile| profile.family.is_none()) {
                p { role: "status", "Choose a supported switch family to resolve the plate gap." }
            }
            if props.targets.iter().any(|target| target.switch_profile) {
                div { class: "m1-mechanical-profile-assignment",
                    label { class: "m1-mechanical-field",
                        span { "Library fit profile" }
                        select {
                            aria_label: "Library fit profile",
                            disabled: !props.editable,
                            value: selected_family(),
                            onchange: move |event: FormEvent| selected_family.set(event.value()),
                            option { value: "", "Choose a switch family…" }
                            option { value: "mx", "MX switch" }
                            option { value: "choc-v1", "Choc v1 switch" }
                            option { value: "choc-v2", "Choc v2 switch" }
                        }
                    }
                    label { class: "m1-mechanical-field",
                        span { "Assign library fit profile to" }
                        select {
                            aria_label: "Assign library fit profile to",
                            disabled: mechanical_action_pending("assign-switch-profile:") || !props.editable || selected_family().is_empty(),
                            value: "",
                            onchange: {
                                let identity = props.identity.clone();
                                let mut sequence = props.request_sequence;
                                let on_request = props.on_request;
                                let selected_family = selected_family;
                                move |event: FormEvent| {
                                    let definition_id = event.value();
                                    if !definition_id.is_empty()
                                        && let Some(family) = parse_family(&selected_family())
                                    {
                                        send_request(
                                            &mut sequence,
                                            &identity,
                                            on_request,
                                            MechanicalSettingsPatch::AssignSwitchProfile {
                                                definition_id,
                                                family,
                                            },
                                        );
                                    }
                                }
                            },
                            option { value: "", "Choose a placed switch type…" }
                            for target in props.targets.iter().filter(|target| target.switch_profile) {
                                option { value: "{target.definition_id}", "{target.name}" }
                            }
                        }
                    }
                }
                div { class: "m1-mechanical-profile-assignment",
                    label { class: "m1-mechanical-field",
                        span { "Library stabilizer profile" }
                        select {
                            aria_label: "Library stabilizer profile",
                            disabled: !props.editable,
                            value: selected_stabilizer(),
                            onchange: move |event: FormEvent| selected_stabilizer.set(event.value()),
                            option { value: "", "Choose a stabilizer size…" }
                            option { value: "mx-stab2u", "MX stabilizer · 2u" }
                            option { value: "mx-stab625u", "MX stabilizer · 6.25u" }
                        }
                    }
                    label { class: "m1-mechanical-field",
                        span { "Assign library stabilizer profile to" }
                        select {
                            aria_label: "Assign library stabilizer profile to",
                            disabled: mechanical_action_pending("assign-stabilizer-profile:") || !props.editable || selected_stabilizer().is_empty(),
                            value: "",
                            onchange: {
                                let identity = props.identity.clone();
                                let mut sequence = props.request_sequence;
                                let on_request = props.on_request;
                                let selected_stabilizer = selected_stabilizer;
                                move |event: FormEvent| {
                                    let source = match selected_stabilizer().as_str() {
                                        "mx-stab2u" => Some(MechanicalBuiltinProfile::MxStab2u),
                                        "mx-stab625u" => Some(MechanicalBuiltinProfile::MxStab625u),
                                        _ => None,
                                    };
                                    if !event.value().is_empty()
                                        && let Some(source) = source
                                    {
                                        send_request(
                                            &mut sequence,
                                            &identity,
                                            on_request,
                                            MechanicalSettingsPatch::AssignStabilizerProfile {
                                                definition_id: event.value(),
                                                source,
                                            },
                                        );
                                    }
                                }
                            },
                            option { value: "", "Choose a placed switch type…" }
                            for target in props.targets.iter() {
                                option { value: "{target.definition_id}", "{target.name}" }
                            }
                        }
                    }
                }
            }
            if props.targets.iter().any(|target| target.imported_geometry) {
                label { class: "m1-mechanical-field",
                    span { "Custom KiCad geometry" }
                    select {
                        aria_label: "Assign custom geometry profile to",
                        disabled: mechanical_action_pending("assign-imported-profile:") || !props.editable,
                        value: "",
                        onchange: {
                            let identity = props.identity.clone();
                            let mut sequence = props.request_sequence;
                            let on_request = props.on_request;
                            move |event: FormEvent| {
                                if !event.value().is_empty() {
                                    send_request(
                                        &mut sequence,
                                        &identity,
                                        on_request,
                                        MechanicalSettingsPatch::AssignImportedGeometryProfile {
                                            definition_id: event.value(),
                                        },
                                    );
                                }
                            }
                        },
                        option { value: "", "Choose imported part type…" }
                        for target in props.targets.iter().filter(|target| target.imported_geometry) {
                            option { value: "{target.definition_id}", "{target.name}" }
                        }
                    }
                }
            }
            p { "Current plate thickness: {props.plate_thickness:.2} mm." }
        }
    }
}

#[derive(Props, Clone, PartialEq)]
struct ProfileGeometryEditorProps {
    identity: MechanicalSettingsIdentity,
    feedback: Rc<[MechanicalSettingsFeedback]>,
    profile: MechanicalPartProfile,
    editable: bool,
    request_sequence: Signal<u64>,
    on_request: EventHandler<MechanicalSettingsRequest>,
    extract_profile: Option<MechanicalProfileExtractionPort>,
}

#[component]
fn ProfileGeometryEditor(props: ProfileGeometryEditorProps) -> Element {
    let geometry_state = use_signal(|| None::<MechanicalGeometry>);
    let selected = use_signal(|| Vec::<String>::new());
    let purposes = use_signal(|| Vec::<MechanicalPurposeMapping>::new());
    let busy = use_signal(|| false);
    let error = use_signal(String::new);
    let extraction_port = props.extract_profile.clone();
    rsx! {
        section { class: "m1-mechanical-profile-editor", aria_label: "Profile geometry",
            if let Some(port) = extraction_port.clone() {
                section { class: "m1-mechanical-geometry-picker", aria_label: "Custom KiCad geometry",
                    div { class: "m1-mechanical-geometry-head",
                        strong { "Custom geometry layers" }
                        button {
                            r#type: "button",
                            aria_label: "Read KiCad layers",
                            disabled: !props.editable || busy(),
                            onclick: {
                                let definition_id = props.profile.definition_id.clone();
                                let port = port.clone();
                                let mut geometry_state = geometry_state;
                                let mut selected = selected;
                                let mut purposes = purposes;
                                let profile_source_geometry = props.profile.source_geometry.clone();
                                let mut busy = busy;
                                let mut error = error;
                                move |_| {
                                    busy.set(true);
                                    error.set(String::new());
                                    let definition_id = definition_id.clone();
                                    let port = port.clone();
                                    let existing_source_geometry = profile_source_geometry.clone();
                                    dioxus::prelude::spawn(async move {
                                        match (port.0)(definition_id, Vec::new()).await {
                                            Ok(result) => {
                                                selected.set(existing_source_geometry.as_ref().map_or_else(Vec::new, |source| source.source_ids.clone()));
                                                purposes.set(existing_source_geometry.as_ref().map_or_else(|| result.source_geometry.mappings.clone(), |source| source.mappings.clone()));
                                                geometry_state.set(Some(result.geometry));
                                            }
                                            Err(message) => error.set(message),
                                        }
                                        busy.set(false);
                                    });
                                }
                            },
                            if geometry_state().is_some() { "Reload layers" } else { "Read KiCad layers" }
                        }
                    }
                    if let Some(loaded_geometry) = geometry_state() {
                        p { class: "m1-mechanical-help", "Select exact primitives for plate cutouts, PCB holes, clearances or drawing guides, then apply the selected geometry." }
                        for primitive in loaded_geometry.primitives.iter() {
                            div { class: "m1-mechanical-geometry-row", key: "{primitive.id}",
                                label {
                                    input {
                                        r#type: "checkbox",
                                        checked: selected().contains(&primitive.id),
                                        aria_label: "Select geometry {primitive.id}",
                                        disabled: !props.editable || busy(),
                                        onchange: {
                                            let id = primitive.id.clone();
                                            let mut selected = selected;
                                            move |event: FormEvent| {
                                                let mut ids = selected().clone();
                                                if event.checked() {
                                                    if !ids.contains(&id) { ids.push(id.clone()); }
                                                } else { ids.retain(|entry| entry != &id); }
                                                selected.set(ids);
                                            }
                                        }
                                    }
                                    span { strong { "{primitive.layer.as_deref().unwrap_or(\"geometry\")}" } small { "{primitive.kind:?} · {primitive.source_group_id}" } }
                                }
                                label {
                                    span { "Purpose" }
                                    select {
                                        aria_label: "Purpose for {primitive.layer.as_deref().unwrap_or(\"geometry\")}",
                                        disabled: !props.editable || busy(),
                                        value: purposes().iter().find(|mapping| mapping.source_id.as_deref() == Some(primitive.id.as_str())).map_or("plate-cutout", |mapping| purpose_id(mapping.purpose)),
                                        onchange: {
                                            let id = primitive.id.clone();
                                            let mut purposes = purposes;
                                            move |event: FormEvent| {
                                                let Some(purpose) = parse_purpose(&event.value()) else { return; };
                                                let mut entries = purposes().clone();
                                                entries.retain(|mapping| mapping.source_id.as_deref() != Some(id.as_str()));
                                                entries.push(MechanicalPurposeMapping { source_id: Some(id.clone()), kind: None, layer: None, purpose });
                                                purposes.set(entries);
                                            }
                                        },
                                        option { value: "plate-cutout", "Plate cutout" }
                                        option { value: "electrical-pcb-mounting-hole", "PCB mounting hole" }
                                        option { value: "clearance-envelope", "Clearance envelope" }
                                        option { value: "drawing-guide", "Drawing guide" }
                                    }
                                }
                            }
                        }
                        button {
                            r#type: "button",
                            aria_label: "Apply selected geometry",
                            disabled: mechanical_action_pending(&format!("profile:{}:update", props.profile.definition_id)) || !props.editable || busy() || selected().is_empty(),
                            onclick: {
                                let definition_id = props.profile.definition_id.clone();
                                let port = port.clone();
                                let geometry_state = geometry_state;
                                let purposes = purposes;
                                let selected = selected;
                                let profile = props.profile.clone();
                                let identity = props.identity.clone();
                                let sequence = props.request_sequence;
                                let on_request = props.on_request;
                                let mut busy = busy;
                                let mut error = error;
                                move |_| {
                                    let ids = selected().clone();
                                    let mappings = geometry_state().as_ref().into_iter().flat_map(|geometry| geometry.primitives.iter()).filter(|primitive| ids.contains(&primitive.id)).map(|primitive| {
                                        let purpose = purposes().iter().find(|mapping| mapping.source_id.as_deref() == Some(primitive.id.as_str())).map_or(MechanicalPurpose::PlateCutout, |mapping| mapping.purpose);
                                        MechanicalPurposeMapping { source_id: Some(primitive.id.clone()), kind: None, layer: None, purpose }
                                    }).collect::<Vec<_>>();
                                    busy.set(true);
                                    error.set(String::new());
                                    let definition_id = definition_id.clone();
                                    let port = port.clone();
                                    let mut sequence = sequence;
                                    let profile = profile.clone();
                                    let identity = identity.clone();
                                    dioxus::prelude::spawn(async move {
                                        match (port.0)(definition_id, mappings).await {
                                            Ok(result) => {
                                                let mut next = profile;
                                                next.cutouts = result.plate_cutouts;
                                                next.clearances = Some(result.clearance_envelopes);
                                                next.pcb_holes = Some(result.pcb_holes);
                                                next.source_geometry = Some(result.source_geometry);
                                                send_request(&mut sequence, &identity, on_request, MechanicalSettingsPatch::UpdateProfile(next));
                                            }
                                            Err(message) => error.set(message),
                                        }
                                        busy.set(false);
                                    });
                                }
                            },
                            "Apply selected geometry"
                        }
                    }
                    if !error().is_empty() { p { role: "alert", "{error()}" } }
                }
            }
            ProfilePolygonListEditor {
                title: "Plate cutout geometry", polygons: props.profile.cutouts.clone(), editable: props.editable,
                on_change: {
                    let profile = props.profile.clone(); let identity = props.identity.clone();
                    let mut sequence = props.request_sequence; let on_request = props.on_request;
                    move |cutouts| { let mut next = profile.clone(); next.cutouts = cutouts;
                        send_request(&mut sequence, &identity, on_request, MechanicalSettingsPatch::UpdateProfile(next)); }
                },
            }
            ProfilePolygonListEditor {
                title: "Component clearance zones", polygons: props.profile.clearances.clone().unwrap_or_default(), editable: props.editable,
                on_change: {
                    let profile = props.profile.clone(); let identity = props.identity.clone();
                    let mut sequence = props.request_sequence; let on_request = props.on_request;
                    move |clearances| { let mut next = profile.clone(); next.clearances = Some(clearances);
                        send_request(&mut sequence, &identity, on_request, MechanicalSettingsPatch::UpdateProfile(next)); }
                },
            }
            ProfileOpeningListEditor {
                identity: props.identity.clone(), profile: props.profile.clone(),
                request_sequence: props.request_sequence, on_request: props.on_request,
                feedback: props.feedback.clone(), clearance_volume: false,
                title: "Access openings", openings: props.profile.openings.clone().unwrap_or_default(), editable: props.editable,
                on_change: {
                    let profile = props.profile.clone(); let identity = props.identity.clone();
                    let mut sequence = props.request_sequence; let on_request = props.on_request;
                    move |openings| { let mut next = profile.clone(); next.openings = Some(openings);
                        send_request(&mut sequence, &identity, on_request, MechanicalSettingsPatch::UpdateProfile(next)); }
                },
            }
            ProfileOpeningListEditor {
                identity: props.identity.clone(), profile: props.profile.clone(),
                request_sequence: props.request_sequence, on_request: props.on_request,
                feedback: props.feedback.clone(), clearance_volume: true,
                title: "Clearance volumes", openings: props.profile.clearance_volumes.clone().unwrap_or_default(), editable: props.editable,
                on_change: {
                    let profile = props.profile.clone(); let identity = props.identity.clone();
                    let mut sequence = props.request_sequence; let on_request = props.on_request;
                    move |clearance_volumes| { let mut next = profile.clone(); next.clearance_volumes = Some(clearance_volumes);
                        send_request(&mut sequence, &identity, on_request, MechanicalSettingsPatch::UpdateProfile(next)); }
                },
            }
        }
    }
}

#[derive(Props, Clone, PartialEq)]
struct ProfilePolygonListEditorProps {
    title: &'static str,
    polygons: Vec<Vec<Vec2>>,
    editable: bool,
    on_change: EventHandler<Vec<Vec<Vec2>>>,
}

#[component]
fn ProfilePolygonListEditor(props: ProfilePolygonListEditorProps) -> Element {
    rsx! {
        section { class: "m1-mechanical-profile-points", aria_label: "{props.title}",
            header {
                strong { "{props.title}" }
                button {
                    r#type: "button", class: "m1-mechanical-quiet",
                    aria_label: "Add polygon to {props.title}", disabled: !props.editable,
                    onclick: {
                        let mut polygons = props.polygons.clone();
                        let on_change = props.on_change;
                        move |_| {
                            polygons.push(vec![
                                Vec2 { x: -2.5, y: -2.5 }, Vec2 { x: 2.5, y: -2.5 },
                                Vec2 { x: 2.5, y: 2.5 }, Vec2 { x: -2.5, y: 2.5 },
                            ]);
                            on_change.call(polygons.clone());
                        }
                    },
                    "Add polygon"
                }
            }
            if props.polygons.is_empty() { p { class: "m1-mechanical-help", "No contours assigned." } }
            for (polygon_index, polygon) in props.polygons.iter().enumerate() {
                fieldset { class: "m1-mechanical-profile-polygon", key: "{props.title}:{polygon_index}", disabled: !props.editable,
                    div { class: "m1-mechanical-opening-title",
                        strong { "Contour {polygon_index + 1}" }
                        button {
                            r#type: "button", class: "m1-mechanical-quiet",
                            aria_label: "Remove contour {polygon_index + 1} from {props.title}", disabled: !props.editable,
                            onclick: {
                                let mut polygons = props.polygons.clone();
                                let on_change = props.on_change;
                                move |_| { polygons.remove(polygon_index); on_change.call(polygons.clone()); }
                            },
                            "Remove"
                        }
                    }
                    for (point_index, point) in polygon.iter().enumerate() {
                        div { class: "m1-mechanical-profile-point", key: "{polygon_index}:{point_index}",
                            span { "V{point_index + 1}" }
                            input {
                                r#type: "number", step: "0.1", value: "{point.x}", disabled: !props.editable,
                                aria_label: "{props.title} contour {polygon_index + 1} vertex {point_index + 1} X",
                                oninput: {
                                    let mut polygons = props.polygons.clone(); let on_change = props.on_change;
                                    move |event: FormEvent| if let Ok(value) = event.value().parse::<f64>() {
                                        polygons[polygon_index][point_index].x = value; on_change.call(polygons.clone());
                                    }
                                }
                            }
                            input {
                                r#type: "number", step: "0.1", value: "{point.y}", disabled: !props.editable,
                                aria_label: "{props.title} contour {polygon_index + 1} vertex {point_index + 1} Y",
                                oninput: {
                                    let mut polygons = props.polygons.clone(); let on_change = props.on_change;
                                    move |event: FormEvent| if let Ok(value) = event.value().parse::<f64>() {
                                        polygons[polygon_index][point_index].y = value; on_change.call(polygons.clone());
                                    }
                                }
                            }
                            button {
                                r#type: "button", class: "m1-mechanical-quiet",
                                aria_label: "Remove {props.title} vertex {point_index + 1}",
                                disabled: !props.editable || polygon.len() <= 3,
                                onclick: {
                                    let mut polygons = props.polygons.clone(); let on_change = props.on_change;
                                    move |_| { polygons[polygon_index].remove(point_index); on_change.call(polygons.clone()); }
                                },
                                "×"
                            }
                        }
                    }
                    button {
                        r#type: "button", class: "m1-mechanical-quiet", disabled: !props.editable,
                        aria_label: "Add vertex to {props.title} contour {polygon_index + 1}",
                        onclick: {
                            let mut polygons = props.polygons.clone(); let on_change = props.on_change;
                            move |_| { polygons[polygon_index].push(Vec2 { x: 0.0, y: 0.0 }); on_change.call(polygons.clone()); }
                        },
                        "Add vertex"
                    }
                }
            }
        }
    }
}

#[derive(Props, Clone, PartialEq)]
struct ProfileOpeningListEditorProps {
    identity: MechanicalSettingsIdentity,
    profile: MechanicalPartProfile,
    request_sequence: Signal<u64>,
    on_request: EventHandler<MechanicalSettingsRequest>,
    feedback: Rc<[MechanicalSettingsFeedback]>,
    clearance_volume: bool,
    title: &'static str,
    openings: Vec<CaseOpening>,
    editable: bool,
    on_change: EventHandler<Vec<CaseOpening>>,
}

#[component]
fn ProfileOpeningListEditor(props: ProfileOpeningListEditorProps) -> Element {
    rsx! {
        section { class: "m1-mechanical-profile-points", aria_label: "{props.title}",
            header {
                strong { "{props.title}" }
                button {
                    r#type: "button", class: "m1-mechanical-quiet",
                    aria_label: "Add volume to {props.title}", disabled: !props.editable,
                    onclick: {
                        let mut openings = props.openings.clone(); let on_change = props.on_change;
                        move |_| {
                            openings.push(CaseOpening { points: vec![
                                Vec2 { x: -2.5, y: -2.5 }, Vec2 { x: 2.5, y: -2.5 },
                                Vec2 { x: 2.5, y: 2.5 }, Vec2 { x: -2.5, y: 2.5 },
                            ], z: 0.0, height: 10.0 });
                            on_change.call(openings.clone());
                        }
                    },
                    "Add volume"
                }
            }
            if props.openings.is_empty() { p { class: "m1-mechanical-help", "No volumes configured." } }
            for (opening_index, opening) in props.openings.iter().enumerate() {
                fieldset { class: "m1-mechanical-opening", key: "{props.title}:{opening_index}", disabled: !props.editable,
                    div { class: "m1-mechanical-opening-title",
                        strong { "Volume {opening_index + 1}" }
                        button {
                            r#type: "button", class: "m1-mechanical-quiet",
                            aria_label: "Remove {props.title} volume {opening_index + 1}", disabled: !props.editable,
                            onclick: {
                                let mut openings = props.openings.clone(); let on_change = props.on_change;
                                move |_| { openings.remove(opening_index); on_change.call(openings.clone()); }
                            },
                            "Remove"
                        }
                    }
                    DimensionField {
                        identity: props.identity.clone(), request_sequence: props.request_sequence,
                        on_request: props.on_request, feedback: props.feedback.clone(),
                        field: MechanicalDimension::OpeningBottomZ, label: "Bottom Z", value: opening.z,
                        editable: props.editable,
                        accessible_label: format!("{} volume {} Bottom Z", props.title, opening_index + 1),
                        profile_volume_target: ProfileVolumeDimensionTarget {
                            profile: props.profile.clone(), opening_index,
                            clearance_volume: props.clearance_volume,
                        },
                    }
                    DimensionField {
                        identity: props.identity.clone(), request_sequence: props.request_sequence,
                        on_request: props.on_request, feedback: props.feedback.clone(),
                        field: MechanicalDimension::OpeningHeight, label: "Height", value: opening.height,
                        editable: props.editable,
                        accessible_label: format!("{} volume {} Height", props.title, opening_index + 1),
                        profile_volume_target: ProfileVolumeDimensionTarget {
                            profile: props.profile.clone(), opening_index,
                            clearance_volume: props.clearance_volume,
                        },
                    }
                    h4 { "XY footprint" }
                    for (point_index, point) in opening.points.iter().enumerate() {
                        div { class: "m1-mechanical-opening-vertex", key: "{opening_index}:{point_index}",
                            span { "V{point_index + 1}" }
                            input {
                                r#type: "number", step: "0.1", value: "{point.x}", disabled: !props.editable,
                                aria_label: "{props.title} volume {opening_index + 1} vertex {point_index + 1} X",
                                oninput: {
                                    let mut openings = props.openings.clone(); let on_change = props.on_change;
                                    move |event: FormEvent| if let Ok(value) = event.value().parse::<f64>() {
                                        openings[opening_index].points[point_index].x = value; on_change.call(openings.clone());
                                    }
                                }
                            }
                            input {
                                r#type: "number", step: "0.1", value: "{point.y}", disabled: !props.editable,
                                aria_label: "{props.title} volume {opening_index + 1} vertex {point_index + 1} Y",
                                oninput: {
                                    let mut openings = props.openings.clone(); let on_change = props.on_change;
                                    move |event: FormEvent| if let Ok(value) = event.value().parse::<f64>() {
                                        openings[opening_index].points[point_index].y = value; on_change.call(openings.clone());
                                    }
                                }
                            }
                            button {
                                r#type: "button", class: "m1-mechanical-quiet",
                                aria_label: "Remove {props.title} vertex {point_index + 1}",
                                disabled: !props.editable || opening.points.len() <= 3,
                                onclick: {
                                    let mut openings = props.openings.clone(); let on_change = props.on_change;
                                    move |_| { openings[opening_index].points.remove(point_index); on_change.call(openings.clone()); }
                                },
                                "×"
                            }
                        }
                    }
                    button {
                        r#type: "button", class: "m1-mechanical-quiet", disabled: !props.editable,
                        aria_label: "Add vertex to {props.title} volume {opening_index + 1}",
                        onclick: {
                            let mut openings = props.openings.clone(); let on_change = props.on_change;
                            move |_| { openings[opening_index].points.push(Vec2 { x: 0.0, y: 0.0 }); on_change.call(openings.clone()); }
                        },
                        "Add vertex"
                    }
                }
            }
        }
    }
}

fn parse_purpose(value: &str) -> Option<MechanicalPurpose> {
    match value {
        "plate-cutout" => Some(MechanicalPurpose::PlateCutout),
        "electrical-pcb-mounting-hole" => Some(MechanicalPurpose::ElectricalPcbMountingHole),
        "clearance-envelope" => Some(MechanicalPurpose::ClearanceEnvelope),
        "drawing-guide" => Some(MechanicalPurpose::DrawingGuide),
        _ => None,
    }
}

fn purpose_id(purpose: MechanicalPurpose) -> &'static str {
    match purpose {
        MechanicalPurpose::PlateCutout => "plate-cutout",
        MechanicalPurpose::ElectricalPcbMountingHole => "electrical-pcb-mounting-hole",
        MechanicalPurpose::ClearanceEnvelope => "clearance-envelope",
        MechanicalPurpose::DrawingGuide => "drawing-guide",
    }
}

#[derive(Props, Clone, PartialEq)]
struct DimensionControlsProps {
    identity: MechanicalSettingsIdentity,
    values: MechanicalSettingsValues,
    editable: bool,
    disabled_reason: Option<String>,
    feedback: Rc<[MechanicalSettingsFeedback]>,
    request_sequence: Signal<u64>,
    on_request: EventHandler<MechanicalSettingsRequest>,
    owner_key: String,
}

#[component]
fn DimensionControls(props: DimensionControlsProps) -> Element {
    let fields = [
        (
            MechanicalDimension::PlateThickness,
            "Plate thickness",
            props.values.plate_thickness,
        ),
        (
            MechanicalDimension::PlateFoamThickness,
            "Plate foam",
            props.values.plate_foam_thickness,
        ),
        (
            MechanicalDimension::PcbThickness,
            "PCB thickness",
            props.values.pcb_thickness,
        ),
        (
            MechanicalDimension::BottomFoamThickness,
            "Bottom foam",
            props.values.bottom_foam_thickness,
        ),
        (
            MechanicalDimension::BottomThickness,
            "Bottom thickness",
            props.values.bottom_thickness,
        ),
        (
            MechanicalDimension::WallThickness,
            "Wall thickness",
            props.values.wall_thickness,
        ),
        (
            MechanicalDimension::Clearance,
            "Clearance",
            props.values.clearance,
        ),
        (
            MechanicalDimension::OpeningAllowance,
            "Radial opening allowance",
            props.values.opening_allowance,
        ),
    ];
    let request_sequence = props.request_sequence;
    rsx! {
        fieldset { class: "m1-mechanical-group", disabled: !props.editable,
            legend { "Dimensions and clearances · mm" }
            for (field, label, value) in fields {
                DimensionField {
                    key: "{props.owner_key}:{field.field_id()}",
                    identity: props.identity.clone(),
                    request_sequence,
                    on_request: props.on_request,
                    feedback: props.feedback.clone(),
                    field,
                    label,
                    value,
                    editable: props.editable,
                }
            }
            if let Some(reason) = props.disabled_reason.as_deref() {
                p { role: "status", "{reason}" }
            }
        }
    }
}

#[derive(Props, Clone, PartialEq)]
struct DimensionFieldProps {
    identity: MechanicalSettingsIdentity,
    request_sequence: Signal<u64>,
    on_request: EventHandler<MechanicalSettingsRequest>,
    feedback: Rc<[MechanicalSettingsFeedback]>,
    field: MechanicalDimension,
    label: &'static str,
    value: f64,
    editable: bool,
    #[props(default)]
    support_target: Option<GasketSupportDimensionTarget>,
    #[props(default)]
    mount_target: Option<MountDimensionTarget>,
    #[props(default)]
    opening_target: Option<OpeningDimensionTarget>,
    #[props(default)]
    profile_volume_target: Option<ProfileVolumeDimensionTarget>,
    #[props(default)]
    accessible_label: Option<String>,
    #[props(default)]
    critical_fit_target: Option<CriticalFitDimensionTarget>,
    #[props(default)]
    hardware_target: Option<HardwareDimensionTarget>,
    #[props(default)]
    process_target: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
struct GasketSupportDimensionTarget {
    support_id: String,
    anchors: Vec<MechanicalGasketAnchor>,
}

#[derive(Clone, Debug, PartialEq)]
struct MountDimensionTarget {
    collection: MechanicalMountCollection,
    mount_id: String,
}

#[derive(Clone, Debug, PartialEq)]
struct OpeningDimensionTarget {
    opening_id: String,
    vertex_id: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
struct ProfileVolumeDimensionTarget {
    profile: MechanicalPartProfile,
    opening_index: usize,
    clearance_volume: bool,
}

#[derive(Clone, Debug, PartialEq)]
struct CriticalFitDimensionTarget {
    fit_id: String,
}

#[derive(Clone, Debug, PartialEq)]
struct HardwareDimensionTarget {
    hardware_id: String,
}

// Profile-volume controls emit the same UpdateProfile domain request, but each
// mounted dimension owns a distinct bounded helper observation and accepted text.
fn dimension_field_key(props: &DimensionFieldProps) -> String {
    let field = props.field.field_id();
    if let Some(target) = &props.profile_volume_target {
        format!(
            "field:profile-volume:{}:{}:{}:{field}",
            target.profile.definition_id, target.clearance_volume, target.opening_index
        )
    } else if let Some(target) = &props.support_target {
        format!("field:gasket-support:{}:{field}", target.support_id)
    } else if let Some(target) = &props.mount_target {
        format!(
            "field:mount:{}:{}:{field}",
            mount_collection_id(target.collection),
            target.mount_id
        )
    } else if let Some(target) = &props.opening_target {
        format!(
            "field:opening:{}:{:?}:{field}",
            target.opening_id, target.vertex_id
        )
    } else if let Some(target) = &props.critical_fit_target {
        format!("field:critical-fit:{}:{field}", target.fit_id)
    } else if let Some(target) = &props.hardware_target {
        format!("field:hardware:{}:{field}", target.hardware_id)
    } else if let Some(part_id) = &props.process_target {
        format!("field:process:{part_id}:{field}")
    } else {
        format!("field:dimension:{field}")
    }
}

#[component]
fn DimensionField(props: DimensionFieldProps) -> Element {
    let mut draft = use_signal(|| props.value.to_string());
    let mut dirty = use_signal(|| false);
    let mut error = use_signal(|| None::<String>);
    let mut failure = use_signal(|| None::<String>);
    let field_key = dimension_field_key(&props);
    let accepted = props.value.to_string();
    let (controller, pending) =
        use_bound_mechanical_field(field_key.clone(), accepted.clone(), draft, failure);
    use_effect(use_reactive(
        (&accepted, &pending),
        move |(accepted, pending)| {
            if pending {
                return;
            }
            if !*dirty.peek() && *draft.peek() != accepted {
                draft.set(accepted);
                error.set(None);
            }
        },
    ));

    let commit: Rc<dyn Fn()> = Rc::new({
        let identity = props.identity.clone();
        let field = props.field;
        let support_target = props.support_target.clone();
        let mount_target = props.mount_target.clone();
        let opening_target = props.opening_target.clone();
        let profile_volume_target = props.profile_volume_target.clone();
        let critical_fit_target = props.critical_fit_target.clone();
        let hardware_target = props.hardware_target.clone();
        let process_target = props.process_target.clone();
        let sequence = props.request_sequence;
        let on_request = props.on_request;
        move || {
            let mut sequence = sequence;
            let mut error = error;
            let mut dirty = dirty;
            let text = draft();
            let process_rule = process_target.as_deref().map(|target| {
                if target.ends_with("foam") {
                    NumberRule::Nonnegative
                } else {
                    NumberRule::AtLeastTenth
                }
            });
            let rule = process_rule.unwrap_or_else(|| field.rule());
            if !dirty() {
                return;
            }
            let Ok(value) = text.trim().parse::<f64>() else {
                error.set(Some(rule.error().to_owned()));
                return;
            };
            if !rule.valid(value) {
                error.set(Some(rule.error().to_owned()));
                return;
            }
            error.set(None);
            let patch = if let Some(target) = support_target.clone() {
                MechanicalSettingsPatch::SetGasketSupportDimension {
                    support_id: target.support_id,
                    anchors: target.anchors,
                    field,
                    value,
                }
            } else if let Some(target) = mount_target.clone() {
                MechanicalSettingsPatch::SetMountDimension {
                    collection: target.collection,
                    mount_id: target.mount_id,
                    field,
                    value,
                }
            } else if let Some(mut target) = profile_volume_target.clone() {
                let openings = if target.clearance_volume {
                    target.profile.clearance_volumes.as_mut()
                } else {
                    target.profile.openings.as_mut()
                };
                let Some(opening) =
                    openings.and_then(|openings| openings.get_mut(target.opening_index))
                else {
                    error.set(Some(
                        "This profile volume is no longer available.".to_owned(),
                    ));
                    return;
                };
                match field {
                    MechanicalDimension::OpeningBottomZ => opening.z = value,
                    MechanicalDimension::OpeningHeight => opening.height = value,
                    _ => return,
                }
                MechanicalSettingsPatch::UpdateProfile(target.profile)
            } else if let Some(target) = opening_target.clone() {
                MechanicalSettingsPatch::SetOpeningDimension {
                    opening_id: target.opening_id,
                    vertex_id: target.vertex_id,
                    field,
                    value,
                }
            } else if let Some(target) = critical_fit_target.clone() {
                MechanicalSettingsPatch::SetCriticalFitPoint {
                    fit_id: target.fit_id,
                    field,
                    value,
                }
            } else if let Some(target) = hardware_target.clone() {
                MechanicalSettingsPatch::SetHardwareDimension {
                    hardware_id: target.hardware_id,
                    field,
                    value,
                }
            } else if let Some(part_id) = process_target.clone() {
                MechanicalSettingsPatch::SetProcessThickness {
                    part_id,
                    thickness: value,
                }
            } else {
                MechanicalSettingsPatch::SetDimension { field, value }
            };
            let Some(request_id) = sequence().checked_add(1) else {
                error.set(Some(
                    "Mechanical request identity is exhausted; reopen the Case inspector."
                        .to_owned(),
                ));
                return;
            };
            sequence.set(request_id);
            let request = MechanicalSettingsRequest {
                identity: identity.clone(),
                request_id,
                field_id: patch.field_id(),
                patch,
                reviewed_fingerprint: None,
            };
            dirty.set(false);
            if let Some(controller) = controller.as_ref() {
                controller.request_field(
                    field_key.clone(),
                    draft,
                    failure,
                    text,
                    on_request,
                    request,
                );
            } else {
                on_request.call(request);
            }
        }
    });
    let error_text = error().or_else(|| failure());
    let status_text = pending.then_some("Saving…");
    rsx! {
        label { class: "m1-mechanical-dimension",
            span { "{props.label}" }
            span { class: "m1-mechanical-number",
                input {
                    r#type: "number",
                    step: props.field.step(),
                    min: props.process_target.as_deref().map(|target| {
                        if target.ends_with("foam") { "0" } else { "0.1" }
                    }).unwrap_or_else(|| props.field.rule().minimum()),
                    max: if props.field == MechanicalDimension::OpeningAllowance { "1" },
                    aria_label: props.accessible_label.as_deref().unwrap_or(props.label),
                    value: "{draft}",
                    disabled: !props.editable,
                    "aria-invalid": error_text.is_some(),
                    oninput: move |event: FormEvent| {
                        draft.set(event.value());
                        dirty.set(true);
                        error.set(None);
                        failure.set(None);
                    },
                    onblur: {
                        let commit = commit.clone();
                        move |_| commit()
                    },
                    onkeydown: {
                        let accepted = props.value;
                        move |event: KeyboardEvent| {
                            let key = event.data().key().to_string();
                            if key == "Enter" {
                                event.prevent_default();
                                if let Some(input) = event
                                    .data()
                                    .try_as_web_event()
                                    .and_then(|event| event.target())
                                    .and_then(|target| target.dyn_into::<HtmlInputElement>().ok())
                                {
                                    let _ = input.blur();
                                }
                            } else if key == "Escape" {
                                event.prevent_default();
                                draft.set(accepted.to_string());
                                dirty.set(false);
                                error.set(None);
                                failure.set(None);
                            }
                        }
                    }
                }
                if props.field == MechanicalDimension::HardwareQuantity {
                    small { "pcs" }
                } else {
                    small { "mm" }
                }
            }
            if let Some(error) = error_text.as_deref() { small { role: "alert", "{error}" } }
            if let Some(status) = status_text { small { role: "status", "{status}" } }
        }
    }
}

#[derive(Props, Clone, PartialEq)]
struct TextDraftFieldProps {
    identity: MechanicalSettingsIdentity,
    request_sequence: Signal<u64>,
    on_request: EventHandler<MechanicalSettingsRequest>,
    feedback: Rc<[MechanicalSettingsFeedback]>,
    label: &'static str,
    value: String,
    intent: MechanicalTextIntent,
    editable: bool,
    #[props(default)]
    multiline: bool,
}

#[component]
fn TextDraftField(props: TextDraftFieldProps) -> Element {
    let mut draft = use_signal(|| props.value.clone());
    let mut dirty = use_signal(|| false);
    let mut error = use_signal(|| None::<String>);
    let mut failure = use_signal(|| None::<String>);
    let field_key = format!("field:text:{:?}", props.intent);
    let accepted = props.value.clone();
    let (controller, pending) =
        use_bound_mechanical_field(field_key.clone(), accepted.clone(), draft, failure);
    use_effect(use_reactive(
        (&accepted, &pending),
        move |(accepted, pending)| {
            if pending {
                return;
            }
            if !*dirty.peek() && *draft.peek() != accepted {
                draft.set(accepted);
                error.set(None);
            }
        },
    ));

    let commit: Rc<dyn Fn()> = Rc::new({
        let identity = props.identity.clone();
        let intent = props.intent.clone();
        let sequence = props.request_sequence;
        let on_request = props.on_request;
        move || {
            let mut sequence = sequence;
            let mut error = error;
            let mut dirty = dirty;
            let draft = draft;
            let value = draft();
            if !dirty() {
                return;
            }
            let patch = match intent.patch(value.clone()) {
                Ok(patch) => patch,
                Err(message) => {
                    error.set(Some(message));
                    return;
                }
            };
            let Some(request_id) = sequence().checked_add(1) else {
                error.set(Some(
                    "Mechanical request identity is exhausted; reopen the Case inspector."
                        .to_owned(),
                ));
                return;
            };
            sequence.set(request_id);
            let request = MechanicalSettingsRequest {
                identity: identity.clone(),
                request_id,
                field_id: patch.field_id(),
                patch,
                reviewed_fingerprint: None,
            };
            dirty.set(false);
            error.set(None);
            if let Some(controller) = controller.as_ref() {
                controller.request_field(
                    field_key.clone(),
                    draft,
                    failure,
                    value,
                    on_request,
                    request,
                );
            } else {
                on_request.call(request);
            }
        }
    });
    let error_text = error().or_else(|| failure());
    let status_text = pending.then_some("Saving…");
    rsx! {
        label { class: "m1-mechanical-field",
            span { "{props.label}" }
            if props.multiline {
                textarea {
                    class: "m1-mechanical-text",
                    value: "{draft}",
                    disabled: !props.editable,
                    "aria-invalid": error_text.is_some(),
                    rows: 2,
                    oninput: move |event: FormEvent| {
                        draft.set(event.value());
                        dirty.set(true);
                        error.set(None);
                        failure.set(None);
                    },
                    onblur: {
                        let commit = commit.clone();
                        move |_| commit()
                    },
                    onkeydown: {
                        let accepted = props.value.clone();
                        move |event: KeyboardEvent| {
                            let key = event.data().key().to_string();
                            if key == "Escape" {
                                event.prevent_default();
                                draft.set(accepted.clone());
                                dirty.set(false);
                                error.set(None);
                                failure.set(None);
                            }
                        }
                    }
                }
            } else {
                input {
                    r#type: "text",
                    class: "m1-mechanical-text",
                    value: "{draft}",
                    disabled: !props.editable,
                    "aria-invalid": error_text.is_some(),
                    oninput: move |event: FormEvent| {
                        draft.set(event.value());
                        dirty.set(true);
                        error.set(None);
                        failure.set(None);
                    },
                    onblur: {
                        let commit = commit.clone();
                        move |_| commit()
                    },
                    onkeydown: {
                        let accepted = props.value.clone();
                        move |event: KeyboardEvent| {
                            let key = event.data().key().to_string();
                            if key == "Enter" {
                                event.prevent_default();
                                if let Some(input) = event
                                    .data()
                                    .try_as_web_event()
                                    .and_then(|event| event.target())
                                    .and_then(|target| target.dyn_into::<HtmlInputElement>().ok())
                                {
                                    let _ = input.blur();
                                }
                            } else if key == "Escape" {
                                event.prevent_default();
                                draft.set(accepted.clone());
                                dirty.set(false);
                                error.set(None);
                                failure.set(None);
                            }
                        }
                    }
                }
            }
            if let Some(error) = error_text.as_deref() { small { role: "alert", "{error}" } }
            if let Some(status) = status_text { small { role: "status", "{status}" } }
        }
    }
}

#[derive(Props, Clone, PartialEq)]
struct ResolvedStackProps {
    layers: Rc<[MechanicalLayerRow]>,
    selected_layer: String,
    on_select_layer: EventHandler<String>,
}

#[component]
fn ResolvedStack(props: ResolvedStackProps) -> Element {
    let is_previous = props.layers.iter().any(|layer| layer.is_previous);
    rsx! {
        section { class: "m1-mechanical-group", aria_label: "Resolved mechanical stack",
            h3 { if is_previous { "Previous resolved stack · {props.layers.len()} layers" } else { "Resolved stack · {props.layers.len()} layers" } }
            if props.layers.is_empty() {
                p { role: "status", "The stack appears after the current revision resolves." }
            } else {
                div { class: "m1-mechanical-stack",
                    for row in props.layers.iter() {
                        { let id = row.id.clone();
                        let label = row.label.clone();
                        let selected = props.selected_layer == id;
                        let callback = props.on_select_layer;
                        rsx! { button {
                            key: "{id}",
                            r#type: "button",
                            class: if selected { "m1-mechanical-layer is-selected" } else { "m1-mechanical-layer" },
                            aria_pressed: selected,
                            onclick: move |_| callback.call(if selected { String::new() } else { id.clone() }),
                            strong { "{label}" }
                            span { "{row.thickness:.2} mm" }
                            small { "Z {row.z:.2}" }
                            if row.is_previous { small { "Previous revision" } }
                        } }
                        }
                    }
                }
            }
        }
    }
}

#[derive(Props, Clone, PartialEq)]
struct DiagnosticsProps {
    findings: Rc<[MechanicalFindingRow]>,
    on_show_finding: EventHandler<String>,
}

#[component]
fn Diagnostics(props: DiagnosticsProps) -> Element {
    rsx! {
        section { class: "m1-mechanical-group", aria_label: "Mechanical diagnostics",
            h3 { "Mechanical diagnostics · {props.findings.len()}" }
            if props.findings.is_empty() {
                p { "No current mechanical findings." }
            }
            ul {
                for row in props.findings.iter() {
                    { let id = row.id.clone();
                    let severity = severity_label(&row.severity);
                    let callback = props.on_show_finding;
                    rsx! { li { key: "{id}",
                        span { "{severity}: {row.message}" }
                        button {
                            r#type: "button",
                            onclick: move |_| callback.call(id.clone()),
                            "Show"
                        }
                    } }
                    }
                }
            }
        }
    }
}

fn send_request(
    sequence: &mut Signal<u64>,
    identity: &MechanicalSettingsIdentity,
    on_request: EventHandler<MechanicalSettingsRequest>,
    patch: MechanicalSettingsPatch,
) {
    let Some(request_id) = sequence().checked_add(1) else {
        return;
    };
    sequence.set(request_id);
    send_request_with_review(sequence, identity, on_request, patch, None)
}

/// Send one request, optionally binding it to the reviewed result the user saw.
fn send_request_with_review(
    sequence: &mut Signal<u64>,
    identity: &MechanicalSettingsIdentity,
    on_request: EventHandler<MechanicalSettingsRequest>,
    patch: MechanicalSettingsPatch,
    reviewed_fingerprint: Option<String>,
) {
    let Some(request_id) = sequence().checked_add(1) else {
        return;
    };
    sequence.set(request_id);
    on_request.call(MechanicalSettingsRequest {
        identity: identity.clone(),
        request_id,
        field_id: patch.field_id(),
        patch,
        reviewed_fingerprint,
    });
}

fn method_id(value: &PlateMethod) -> &'static str {
    match value {
        PlateMethod::PcbFr4 => "pcb-fr4",
        PlateMethod::Printed => "printed",
        PlateMethod::Cnc => "cnc",
        PlateMethod::CutSheet => "cut-sheet",
    }
}

fn parse_method(value: &str) -> Option<PlateMethod> {
    Some(match value {
        "pcb-fr4" => PlateMethod::PcbFr4,
        "printed" => PlateMethod::Printed,
        "cnc" => PlateMethod::Cnc,
        "cut-sheet" => PlateMethod::CutSheet,
        _ => return None,
    })
}

fn mount_id(value: &MechanicalMount) -> &'static str {
    match value {
        MechanicalMount::Tray => "tray",
        MechanicalMount::Rigid => "rigid",
        MechanicalMount::Gasket => "gasket",
    }
}

fn parse_mount(value: &str) -> Option<MechanicalMount> {
    Some(match value {
        "tray" => MechanicalMount::Tray,
        "rigid" => MechanicalMount::Rigid,
        "gasket" => MechanicalMount::Gasket,
        _ => return None,
    })
}

fn parse_mount_kind(value: &str) -> Option<MountKind> {
    Some(match value {
        "hole" => MountKind::Hole,
        "boss" => MountKind::Boss,
        _ => return None,
    })
}

fn bottom_style_id(value: &MechanicalBottomStyle) -> &'static str {
    match value {
        MechanicalBottomStyle::Shell => "shell",
        MechanicalBottomStyle::Sheet => "sheet",
    }
}

fn parse_bottom_style(value: &str) -> Option<MechanicalBottomStyle> {
    Some(match value {
        "shell" => MechanicalBottomStyle::Shell,
        "sheet" => MechanicalBottomStyle::Sheet,
        _ => return None,
    })
}

fn family_id(value: MechanicalSwitchFamily) -> &'static str {
    match value {
        MechanicalSwitchFamily::Mx => "mx",
        MechanicalSwitchFamily::ChocV1 => "choc-v1",
        MechanicalSwitchFamily::ChocV2 => "choc-v2",
    }
}

fn family_label(value: MechanicalSwitchFamily) -> &'static str {
    match value {
        MechanicalSwitchFamily::Mx => "MX",
        MechanicalSwitchFamily::ChocV1 => "Choc v1",
        MechanicalSwitchFamily::ChocV2 => "Choc v2",
    }
}

fn parse_family(value: &str) -> Option<MechanicalSwitchFamily> {
    Some(match value {
        "mx" => MechanicalSwitchFamily::Mx,
        "choc-v1" => MechanicalSwitchFamily::ChocV1,
        "choc-v2" => MechanicalSwitchFamily::ChocV2,
        _ => return None,
    })
}

fn severity_label(value: &Severity) -> &'static str {
    match value {
        Severity::Error => "Error",
        Severity::Warning => "Warning",
        Severity::Info => "Info",
    }
}

#[cfg(test)]
mod contextual_layer_tests {
    use super::*;
    use boardstudio_application::{Scope, SessionEpoch, SnapshotToken};
    use boardstudio_core::model::{MechanicalFeatureKind, Vec3};
    use wasm_bindgen::JsCast;
    use wasm_bindgen_test::wasm_bindgen_test;

    wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

    fn test_identity_with_instance(instance_id: &str) -> MechanicalSettingsIdentity {
        let mut identity = test_identity();
        identity.scope.instance_id = Some(instance_id.to_owned());
        identity
    }

    fn test_identity() -> MechanicalSettingsIdentity {
        MechanicalSettingsIdentity {
            editor_instance_id: 1,
            scope_generation: 1,
            presentation_generation: 1,
            scope: Scope {
                session_epoch: SessionEpoch(1),
                document_id: "document".into(),
                board_id: "board".into(),
                instance_id: None,
            },
            snapshot_token: SnapshotToken(1),
            revision: 1,
            active_board_id: "board".into(),
            configuration_board_id: "board".into(),
        }
    }

    fn test_values(
        transport: HardwareTransport,
        battery: Option<MechanicalBattery>,
    ) -> MechanicalSettingsValues {
        MechanicalSettingsValues {
            board_id: "board".into(),
            transport,
            battery,
            suspension_mounts: vec![],
            closure_mounts: Some(vec![]),
            method: PlateMethod::Printed,
            mount: MechanicalMount::Rigid,
            bottom_style: MechanicalBottomStyle::Shell,
            middle_frame: false,
            integrated_plate_frame: false,
            plate_thickness: 1.5,
            plate_foam_thickness: 0.0,
            pcb_thickness: 1.6,
            bottom_foam_thickness: 0.0,
            bottom_thickness: 2.0,
            wall_thickness: 2.0,
            clearance: 0.2,
            opening_allowance: 0.0,
            openings: vec![],
            internal_gasket: true,
            closure_hardware: None,
            hardware: vec![],
            critical_fits: vec![],
            plate_to_pcb: 3.5,
            battery_height: 0.0,
        }
    }

    fn test_process(
        part_id: &str,
        method: PlateMethod,
        material: &str,
        thickness: f64,
    ) -> MechanicalProcessTarget {
        MechanicalProcessTarget {
            part_id: part_id.into(),
            name: part_id.replace('-', " "),
            default_thickness: thickness,
            process: Some(MechanicalPartProcess {
                part_id: part_id.into(),
                method,
                material: material.into(),
                thickness,
                constraints_version: "2026-09-24".into(),
            }),
        }
    }

    fn test_page() -> Element {
        test_page_with_previous(false, false)
    }

    fn previous_test_page() -> Element {
        test_page_with_previous(true, false)
    }

    fn test_page_with_previous(is_previous: bool, pending: bool) -> Element {
        let mut selected_layer = use_signal(|| "plate".to_owned());
        let mut shown_finding = use_signal(String::new);
        let request_sequence = use_signal(|| 0_u64);
        let identity = test_identity();
        let values = test_values(HardwareTransport::Wired, None);
        let mut process_targets = vec![
            test_process("plate", PlateMethod::Printed, "PLA", 1.5),
            test_process("plate-foam", PlateMethod::CutSheet, "EVA", 0.0),
            test_process("bottom-foam", PlateMethod::CutSheet, "EVA", 0.5),
            test_process("bottom", PlateMethod::Printed, "PLA", 2.0),
        ];
        process_targets.push(MechanicalProcessTarget {
            part_id: "part-1".into(),
            name: "U1 · Fixture part".into(),
            default_thickness: 1.5,
            process: None,
        });
        let layers = Rc::from([
            MechanicalLayerRow {
                id: "plate".into(),
                label: "Plate".into(),
                z: 0.0,
                thickness: 1.5,
                resolved_body_thickness: Some(1.5),
                is_previous,
            },
            MechanicalLayerRow {
                id: "pcb".into(),
                label: "PCB".into(),
                z: 0.0,
                thickness: 1.6,
                resolved_body_thickness: None,
                is_previous: false,
            },
            MechanicalLayerRow {
                id: "plate-foam".into(),
                label: "Plate foam".into(),
                z: 0.0,
                thickness: 0.5,
                resolved_body_thickness: None,
                is_previous: false,
            },
            MechanicalLayerRow {
                id: "bottom-foam".into(),
                label: "Bottom foam".into(),
                z: 0.0,
                thickness: 0.5,
                resolved_body_thickness: None,
                is_previous: false,
            },
            MechanicalLayerRow {
                id: "bottom".into(),
                label: "Bottom".into(),
                z: 0.0,
                thickness: 3.0,
                resolved_body_thickness: Some(3.0),
                is_previous: false,
            },
            MechanicalLayerRow {
                id: "retainer".into(),
                label: "Top case".into(),
                z: 0.0,
                thickness: 0.0,
                resolved_body_thickness: None,
                is_previous: false,
            },
        ]);
        let findings = Rc::from([
            MechanicalFindingRow {
                id: "current-error".into(),
                severity: Severity::Error,
                message: "Current clearance error".into(),
            },
            MechanicalFindingRow {
                id: "current-warning".into(),
                severity: Severity::Warning,
                message: "Current warning".into(),
            },
        ]);
        rsx! {
            MechanicalSettings {
                identity,
                request_sequence,
                values: Some(values),
                profiles: Rc::from([]),
                profile_targets: Rc::from([]),
                process_targets: Rc::from(process_targets),
                stabilizer_fits: Rc::from([]),
                layers,
                gasket_supports: Rc::from([]),
                fit_parts: Rc::from([]),
                fit_parts_resolved: false,
                hardware_mounts: Rc::from([]),
                suggested_mounts: Rc::from([]),
                findings,
                selected_layer: selected_layer(),
                mismatch: None,
                editable: true,
                disabled_reason: None,
                feedback: if pending { Rc::from([MechanicalSettingsFeedback {
                    identity: test_identity(), request_id: 1, field_id: "process:plate:remove".into(),
                    state: MechanicalSettingsFeedbackState::Pending, message: None,
                }]) } else { Rc::from([]) },
                summary_feedback: None,
                on_request: move |_| {},
                on_select_layer: move |id| selected_layer.set(id),
                on_show_finding: move |id| shown_finding.set(id),
            }
            div { id: "case-contextual-selected-finding", "{shown_finding}" }
        }
    }

    fn pending_action_test_page() -> Element {
        test_page_with_previous(false, true)
    }

    #[wasm_bindgen_test]
    async fn pending_action_disables_only_its_control() {
        mount_battery_test_page("case-pending-action-root", pending_action_test_page);
        rendered().await;
        element("#case-pending-action-root .m1-mechanical-context-return").click();
        rendered().await;
        let root = element("#case-pending-action-root");
        let buttons = root.query_selector_all("button").unwrap();
        let mut removed = 0;
        for index in 0..buttons.length() {
            let button = buttons
                .item(index)
                .unwrap()
                .dyn_into::<web_sys::HtmlElement>()
                .unwrap();
            match button.text_content().unwrap_or_default().trim() {
                "Remove" => {
                    assert_eq!(
                        button.has_attribute("disabled"),
                        removed == 0,
                        "only the pending plate removal is disabled"
                    );
                    removed += 1;
                }
                "Add process override" => assert!(
                    !button.has_attribute("disabled"),
                    "another action stays available"
                ),
                _ => {}
            }
        }
        assert_eq!(removed, 4);
        let fields = root.query_selector_all("input[type=number]").unwrap();
        assert!(fields.length() > 0);
        for index in 0..fields.length() {
            assert!(
                !fields
                    .item(index)
                    .unwrap()
                    .dyn_into::<web_sys::HtmlInputElement>()
                    .unwrap()
                    .disabled()
            );
        }
        root.remove();
    }

    fn empty_profile_guidance_test_page() -> Element {
        let request_sequence = use_signal(|| 0_u64);
        rsx! {
            ProfileGuidance {
                identity: test_identity(),
                profiles: Rc::from([]),
                targets: Rc::from([]),
                plate_thickness: 2.0,
                plate_to_pcb: 3.0,
                editable: true,
                request_sequence,
                on_request: move |_| {},
            }
        }
    }

    fn assigned_profile_guidance_test_page() -> Element {
        let request_sequence = use_signal(|| 0_u64);
        rsx! {
            ProfileGuidance {
                identity: test_identity(),
                profiles: Rc::from([MechanicalProfileChoice {
                    definition_id: "assigned-custom".into(),
                    name: "Custom tactile switch".into(),
                    source: Some("KiCad Custom tactile switch".into()),
                    family: None,
                    plate_to_pcb: Some(3.5),
                    supported_thickness: None,
                    switch_family_selectable: false,
                    profile: custom_profile_choice().profile,
                    kicad_source: Some("(footprint custom)".into()),
                }]),
                targets: Rc::from([]),
                plate_thickness: 1.6,
                plate_to_pcb: 3.5,
                editable: true,
                request_sequence,
                on_request: move |_| {},
            }
        }
    }

    #[wasm_bindgen_test]
    async fn assigned_profile_exposes_case_local_remove_control() {
        let document = web_sys::window().unwrap().document().unwrap();
        let root = document.create_element("div").unwrap();
        root.set_id("case-profile-remove-red-root");
        document.body().unwrap().append_child(&root).unwrap();
        dioxus_web::launch::launch_virtual_dom(
            VirtualDom::new(assigned_profile_guidance_test_page),
            dioxus_web::Config::new().rootnode(root.clone().into()),
        );
        rendered().await;
        let buttons = document
            .query_selector_all("#case-profile-remove-red-root button")
            .unwrap();
        let has_remove = (0..buttons.length()).any(|index| {
            buttons
                .item(index)
                .and_then(|button| button.text_content())
                .is_some_and(|text| text.trim() == "Remove profile")
        });
        assert!(
            has_remove,
            "assigned Case profile must expose its local Remove profile action"
        );
        root.remove();
    }

    const CUSTOM_PROFILE_FOOTPRINT: &str = r#"(footprint "Custom tactile switch"
        (fp_rect (start -9.5 9.5) (end 9.5 -9.5)
            (stroke (width 0.1) (type solid)) (fill none) (layer "Dwgs.User")))"#;

    fn custom_profile_choice() -> MechanicalProfileChoice {
        MechanicalProfileChoice {
            definition_id: "custom-part".into(),
            name: "Custom tactile switch".into(),
            source: Some("KiCad Custom tactile switch".into()),
            family: None,
            plate_to_pcb: Some(3.5),
            supported_thickness: None,
            switch_family_selectable: false,
            profile: MechanicalPartProfile {
                source_geometry: None,
                pcb_holes: None,
                clearance_volumes: None,
                openings: None,
                clearances: None,
                supported_thickness: None,
                switch_family: None,
                definition_id: "custom-part".into(),
                source: "KiCad Custom tactile switch".into(),
                cutouts: vec![],
                plate_to_pcb: 3.5,
            },
            kicad_source: Some(CUSTOM_PROFILE_FOOTPRINT.into()),
        }
    }

    fn custom_profile_test_page() -> Element {
        let request_sequence = use_signal(|| 0_u64);
        let mut request_count = use_signal(|| 0_u64);
        let mut last_request = use_signal(String::new);
        let mut profile = use_signal(custom_profile_choice);
        let mut saved_profile = use_signal(|| {
            let mut choice = custom_profile_choice();
            choice.definition_id = "saved-part".into();
            choice.name = "Saved profile without extraction state".into();
            choice.source = Some("Authored profile".into());
            choice.kicad_source = None;
            choice.profile.definition_id = "saved-part".into();
            choice.profile.source = "Authored profile".into();
            choice.profile.cutouts = vec![vec![
                Vec2 { x: -9.5, y: 9.5 },
                Vec2 { x: 9.5, y: 9.5 },
                Vec2 { x: 9.5, y: -9.5 },
                Vec2 { x: -9.5, y: -9.5 },
            ]];
            choice
        });
        let mut extracted_definition_ids = use_signal(Vec::<String>::new);
        let mut owner_key = use_signal(|| "test-owner-a".to_owned());
        let extraction =
            MechanicalProfileExtractionPort(Rc::new(move |definition_id, mappings| {
                let mut extracted_definition_ids = extracted_definition_ids;
                extracted_definition_ids.with_mut(|ids| ids.push(definition_id.clone()));
                Box::pin(async move {
                    let request = boardstudio_core::model::ArtifactRequest::ExtractMechanical {
                        id: "profile-extraction-test".to_owned(),
                        source: CUSTOM_PROFILE_FOOTPRINT.to_owned(),
                        mappings,
                        max_deviation_mm: 0.005,
                    };
                    let reply = boardstudio_core::artifact_request(
                        &serde_json::to_string(&request).unwrap(),
                    );
                    match serde_json::from_str::<boardstudio_core::model::ArtifactReply>(&reply)
                        .unwrap()
                    {
                        boardstudio_core::model::ArtifactReply::ExtractMechanical {
                            result,
                            ..
                        } => Ok(result),
                        boardstudio_core::model::ArtifactReply::Error { error, .. } => {
                            Err(error.message)
                        }
                        reply => panic!("unexpected mechanical extraction reply: {reply:?}"),
                    }
                })
            }));
        let extracted_ids_label = extracted_definition_ids().join(",");
        let owner_key_value = owner_key();
        rsx! {
            div { id: "case-custom-profile-test-root",
                ProfileGuidance {
                    identity: test_identity(),
                    owner_key: owner_key_value,
                    profiles: Rc::from([profile(), saved_profile()]),
                    targets: Rc::from([]),
                    plate_thickness: 1.6,
                    plate_to_pcb: 3.5,
                    editable: true,
                    request_sequence,
                    extract_profile: Some(extraction),
                    on_request: move |request: MechanicalSettingsRequest| {
                        request_count += 1;
                        if let MechanicalSettingsPatch::UpdateProfile(updated) = request.patch.clone() {
                            if updated.definition_id == "custom-part" {
                                let mut choice = profile();
                                choice.profile = updated;
                                profile.set(choice);
                            } else {
                                let mut choice = saved_profile();
                                choice.profile = updated;
                                saved_profile.set(choice);
                            }
                        }
                        last_request.set(format!(
                            "{:?}|reviewed={:?}",
                            request.patch, request.reviewed_fingerprint
                        ));
                    },
                }
                p { id: "case-custom-profile-last-request", "{last_request}" }
                p { id: "case-custom-profile-request-count", "{request_count}" }
                p { id: "case-custom-profile-extracted-definition-ids", "{extracted_ids_label}" }
                button { id: "case-profile-switch-owner", onclick: move |_| owner_key.set("test-owner-b".to_owned()), "Switch Case owner" }
            }
        }
    }

    #[wasm_bindgen_test]
    async fn custom_profile_geometry_controls_read_apply_edit_and_remove_assigned_profile() {
        let document = web_sys::window().unwrap().document().unwrap();
        let root = document.create_element("div").unwrap();
        root.set_id("case-custom-profile-test-host");
        document.body().unwrap().append_child(&root).unwrap();
        dioxus_web::launch::launch_virtual_dom(
            VirtualDom::new(custom_profile_test_page),
            dioxus_web::Config::new().rootnode(root.clone().into()),
        );
        rendered().await;
        let element = |selector: &str| {
            root.query_selector(selector)
                .unwrap()
                .unwrap()
                .dyn_into::<web_sys::HtmlElement>()
                .unwrap()
        };
        let saved_contour: web_sys::HtmlInputElement =
            element("input[aria-label='Plate cutout geometry contour 1 vertex 1 X']")
                .dyn_into()
                .unwrap();
        saved_contour.set_value("-9.4");
        let input_event = web_sys::EventInit::new();
        input_event.set_bubbles(true);
        saved_contour
            .dispatch_event(
                &web_sys::Event::new_with_event_init_dict("input", &input_event).unwrap(),
            )
            .unwrap();
        rendered().await;
        assert!(
            element("#case-custom-profile-last-request")
                .text_content()
                .unwrap()
                .contains("-9.4")
        );
        assert!(
            element("button[aria-label='Remove profile Custom tactile switch']").is_connected()
        );
        element("button[aria-label='Read KiCad layers']").click();
        rendered().await;
        assert_eq!(
            element("#case-custom-profile-extracted-definition-ids")
                .text_content()
                .unwrap(),
            "custom-part"
        );
        assert!(element("select[aria-label='Purpose for Dwgs.User']").is_connected());
        let checkbox: web_sys::HtmlInputElement =
            element("input[type='checkbox']").dyn_into().unwrap();
        checkbox.click();
        rendered().await;
        let request_count = || {
            element("#case-custom-profile-request-count")
                .text_content()
                .unwrap()
                .parse::<u64>()
                .unwrap()
        };
        let before_apply = request_count();
        element("button[aria-label='Apply selected geometry']").click();
        rendered().await;
        let extraction_error = root
            .query_selector(".m1-mechanical-geometry-picker [role='alert']")
            .unwrap()
            .and_then(|alert| alert.text_content());
        assert!(
            extraction_error.is_none(),
            "real Core rejected the UI mapping: {extraction_error:?}"
        );
        assert_eq!(
            request_count(),
            before_apply + 1,
            "Apply must submit one newly extracted profile"
        );
        assert_eq!(
            element("#case-custom-profile-extracted-definition-ids")
                .text_content()
                .unwrap(),
            "custom-part,custom-part"
        );
        assert!(
            element("#case-custom-profile-last-request")
                .text_content()
                .unwrap()
                .contains("UpdateProfile")
        );
        assert!(
            element("#case-custom-profile-last-request")
                .text_content()
                .unwrap()
                .contains("-9.5")
        );
        let cutout_x: web_sys::HtmlInputElement =
            element("input[aria-label='Plate cutout geometry contour 1 vertex 1 X']")
                .dyn_into()
                .unwrap();
        assert_eq!(
            cutout_x.value(),
            "-9.5",
            "Apply must replace the accepted contour with the real extraction"
        );
        cutout_x.set_value("-9.4");
        cutout_x
            .dispatch_event(
                &web_sys::Event::new_with_event_init_dict("input", &input_event).unwrap(),
            )
            .unwrap();
        rendered().await;
        assert!(
            element("#case-custom-profile-last-request")
                .text_content()
                .unwrap()
                .contains("-9.4")
        );
        element("button[aria-label='Remove profile Custom tactile switch']").click();
        rendered().await;
        assert!(
            element("#case-custom-profile-last-request")
                .text_content()
                .unwrap()
                .contains("RemoveProfile")
        );
        element("button#case-profile-switch-owner").click();
        rendered().await;
        assert!(
            root.query_selector("select[aria-label='Purpose for Dwgs.User']")
                .unwrap()
                .is_none()
        );
        root.remove();
    }

    #[wasm_bindgen_test]
    async fn profile_volume_height_escape_discards_and_enter_blur_commits_once() {
        let document = web_sys::window().unwrap().document().unwrap();
        let root = document.create_element("div").unwrap();
        root.set_id("case-profile-volume-draft-test-host");
        document.body().unwrap().append_child(&root).unwrap();
        dioxus_web::launch::launch_virtual_dom(
            VirtualDom::new(custom_profile_test_page),
            dioxus_web::Config::new().rootnode(root.clone().into()),
        );
        rendered().await;
        let query = |selector: &str| root.query_selector(selector).unwrap().unwrap();
        query("button[aria-label='Add volume to Clearance volumes']")
            .dyn_into::<web_sys::HtmlElement>()
            .unwrap()
            .click();
        rendered().await;
        let input = query("input[aria-label='Clearance volumes volume 1 Height']")
            .dyn_into::<web_sys::HtmlInputElement>()
            .unwrap();
        let dispatch_input = |value: &str| {
            input.set_value(value);
            let init = web_sys::EventInit::new();
            init.set_bubbles(true);
            input
                .dispatch_event(&web_sys::Event::new_with_event_init_dict("input", &init).unwrap())
                .unwrap();
        };
        let dispatch_key = |key: &str| {
            let init = web_sys::KeyboardEventInit::new();
            init.set_key(key);
            init.set_bubbles(true);
            input
                .dispatch_event(
                    &web_sys::KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &init)
                        .unwrap(),
                )
                .unwrap();
        };
        let dispatch_change = || {
            let init = web_sys::EventInit::new();
            init.set_bubbles(true);
            input
                .dispatch_event(&web_sys::Event::new_with_event_init_dict("change", &init).unwrap())
                .unwrap();
        };
        let count = || {
            query("#case-custom-profile-request-count")
                .text_content()
                .unwrap()
                .parse::<u64>()
                .unwrap()
        };

        // Establish the accepted 6 mm height in the retained reference journey.
        input.focus().unwrap();
        dispatch_input("6");
        input.blur().unwrap();
        dispatch_change();
        rendered().await;
        assert_eq!(input.value(), "6");
        let before = count();
        assert_eq!(before, 2, "one Add volume and one accepted height edit");

        input.focus().unwrap();
        dispatch_input("7.2");
        dispatch_key("Escape");
        input.blur().unwrap();
        dispatch_change();
        rendered().await;
        assert_eq!(
            count(),
            before,
            "Escape then blur must not submit the abandoned profile height"
        );
        assert_eq!(
            input.value(),
            "6",
            "Escape must restore the accepted height"
        );

        input.focus().unwrap();
        dispatch_input("6.5");
        dispatch_key("Enter");
        input.blur().unwrap();
        dispatch_change();
        rendered().await;
        assert_eq!(
            count(),
            before + 1,
            "Enter followed by blur/change must submit exactly once"
        );
        assert_eq!(input.value(), "6.5");

        input.focus().unwrap();
        dispatch_input("0.05");
        input.blur().unwrap();
        dispatch_change();
        rendered().await;
        assert_eq!(
            count(),
            before + 1,
            "invalid height below 0.1 mm must not submit"
        );
        input.focus().unwrap();
        dispatch_key("Escape");
        rendered().await;
        assert_eq!(
            input.value(),
            "6.5",
            "invalid input must leave the accepted height unchanged"
        );
        root.remove();
    }

    #[wasm_bindgen_test]
    async fn assigned_profile_adds_component_clearance_polygon_without_extraction() {
        let document = web_sys::window().unwrap().document().unwrap();
        let root = document.create_element("div").unwrap();
        root.set_id("case-profile-clearance-test-host");
        document.body().unwrap().append_child(&root).unwrap();
        dioxus_web::launch::launch_virtual_dom(
            VirtualDom::new(custom_profile_test_page),
            dioxus_web::Config::new().rootnode(root.clone().into()),
        );
        rendered().await;
        let element = |selector: &str| {
            root.query_selector(selector)
                .unwrap()
                .unwrap()
                .dyn_into::<web_sys::HtmlElement>()
                .unwrap()
        };

        element("button[aria-label='Add polygon to Component clearance zones']").click();
        rendered().await;

        let request = element("#case-custom-profile-last-request")
            .text_content()
            .unwrap();
        assert!(request.contains("UpdateProfile"));
        assert!(request.contains("clearances: Some"));
        assert!(request.contains("x: -2.5"));
        element("button[aria-label='Add volume to Access openings']").click();
        rendered().await;
        let request = element("#case-custom-profile-last-request")
            .text_content()
            .unwrap();
        assert!(request.contains("openings: Some"));
        assert!(request.contains("height: 10.0"));
        element("button[aria-label='Add volume to Clearance volumes']").click();
        rendered().await;
        let request = element("#case-custom-profile-last-request")
            .text_content()
            .unwrap();
        assert!(request.contains("clearance_volumes: Some"));
        assert!(request.contains("height: 10.0"));
        assert_eq!(
            element("#case-custom-profile-extracted-definition-ids")
                .text_content()
                .unwrap(),
            ""
        );
        root.remove();
    }

    fn text_commit_test_page() -> Element {
        let request_sequence = use_signal(|| 0_u64);
        let mut count = use_signal(|| 0_u32);
        let mut accepted = use_signal(|| "M2".to_owned());
        rsx! {
            TextDraftField {
                identity: test_identity(), request_sequence, feedback: Rc::from([]),
                label: "Thread", value: accepted(), intent: MechanicalTextIntent::ClosureThread,
                editable: true, on_request: move |_| count += 1,
            }
            output { "{count}" }
            button { onclick: move |_| accepted.set("M4".into()), "Land previous edit" }
        }
    }

    #[wasm_bindgen_test]
    async fn text_escape_discards_and_new_drafts_queue_before_acknowledgment() {
        mount_battery_test_page("case-text-commit-root", text_commit_test_page);
        rendered().await;
        let root = element("#case-text-commit-root");
        let input = root
            .query_selector("input")
            .unwrap()
            .unwrap()
            .dyn_into::<web_sys::HtmlInputElement>()
            .unwrap();
        let input_value = |value: &str| {
            input.focus().unwrap();
            input.set_value(value);
            let event = web_sys::EventInit::new();
            event.set_bubbles(true);
            input
                .dispatch_event(&web_sys::Event::new_with_event_init_dict("input", &event).unwrap())
                .unwrap();
        };
        input_value("M3");
        let escape = web_sys::KeyboardEventInit::new();
        escape.set_key("Escape");
        escape.set_bubbles(true);
        input
            .dispatch_event(
                &web_sys::KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &escape)
                    .unwrap(),
            )
            .unwrap();
        input.blur().unwrap();
        rendered().await;
        assert_eq!(
            root.query_selector("output")
                .unwrap()
                .unwrap()
                .text_content()
                .unwrap(),
            "0",
            "Escape must not submit on blur"
        );
        for value in ["M3", "M4"] {
            input_value(value);
            input.blur().unwrap();
            rendered().await;
        }
        assert_eq!(
            root.query_selector("output")
                .unwrap()
                .unwrap()
                .text_content()
                .unwrap(),
            "2"
        );
        assert_eq!(input.value(), "M4");
        input_value("M5");
        root.query_selector("button")
            .unwrap()
            .unwrap()
            .dyn_into::<web_sys::HtmlElement>()
            .unwrap()
            .click();
        rendered().await;
        assert_eq!(
            input.value(),
            "M5",
            "landing an earlier edit must preserve active typing"
        );
        root.remove();
    }

    #[component]
    fn ProcessThicknessTestPage() -> Element {
        let request_sequence = use_signal(|| 0_u64);
        let mut request_count = use_signal(|| 0_u32);
        rsx! {
            div { id: "case-process-thickness-test-root",
                MechanicalSettings {
                    identity: test_identity(),
                    request_sequence,
                    values: Some(test_values(HardwareTransport::Wired, None)),
                    profiles: Rc::from([]),
                    profile_targets: Rc::from([]),
                    process_targets: Rc::from([test_process("plate", PlateMethod::Printed, "PLA", 1.5)]),
                    stabilizer_fits: Rc::from([]),
                    layers: Rc::from([]),
                    gasket_supports: Rc::from([]),
                    fit_parts: Rc::from([]),
                    fit_parts_resolved: false,
                    hardware_mounts: Rc::from([]),
                    suggested_mounts: Rc::from([]),
                    findings: Rc::from([]),
                    selected_layer: String::new(),
                    mismatch: None,
                    editable: true,
                    disabled_reason: None,
                    feedback: Rc::from([]),
                    summary_feedback: None,
                    on_request: move |_| request_count += 1,
                    on_select_layer: move |_| {},
                    on_show_finding: move |_| {},
                }
            }
            div { id: "case-process-thickness-request-count", "{request_count()}" }
        }
    }

    #[component]
    fn BatteryTestPage() -> Element {
        let wired_request_sequence = use_signal(|| 0_u64);
        let wireless_request_sequence = use_signal(|| 0_u64);
        let mut last_request = use_signal(String::new);
        rsx! {
            div { id: "case-battery-wired-test-root",
                MechanicalSettings {
                    identity: test_identity(),
                    request_sequence: wired_request_sequence,
                    values: Some(test_values(HardwareTransport::Wired, None)),
                    profiles: Rc::from([]),
                    profile_targets: Rc::from([]),
                    process_targets: Rc::from([]),
                    stabilizer_fits: Rc::from([]),
                    layers: Rc::from([]),
                    gasket_supports: Rc::from([]),
                    fit_parts: Rc::from([]),
                    fit_parts_resolved: false,
                    hardware_mounts: Rc::from([]),
                    suggested_mounts: Rc::from([]),
                    findings: Rc::from([]),
                    selected_layer: String::new(),
                    mismatch: None,
                    editable: true,
                    disabled_reason: None,
                    feedback: Rc::from([]),
                    summary_feedback: None,
                    on_request: move |request: MechanicalSettingsRequest| {
                        last_request.set(format!("{:?}", request.patch))
                    },
                    on_select_layer: move |_| {},
                    on_show_finding: move |_| {},
                }
            }
            div { id: "case-battery-wireless-test-root",
                MechanicalSettings {
                    identity: test_identity(),
                    request_sequence: wireless_request_sequence,
                    values: Some(test_values(
                        HardwareTransport::Wireless,
                        Some(MechanicalBattery {
                            cable_width: None,
                            size: Vec3 { x: 30.0, y: 20.0, z: 6.0 },
                            at: Vec2 { x: 12.0, y: -4.0 },
                            cable_exit: Vec2 { x: 29.0, y: -4.0 },
                        }),
                    )),
                    profiles: Rc::from([]),
                    profile_targets: Rc::from([]),
                    process_targets: Rc::from([]),
                    stabilizer_fits: Rc::from([]),
                    layers: Rc::from([]),
                    gasket_supports: Rc::from([]),
                    fit_parts: Rc::from([]),
                    fit_parts_resolved: false,
                    hardware_mounts: Rc::from([]),
                    suggested_mounts: Rc::from([]),
                    findings: Rc::from([]),
                    selected_layer: String::new(),
                    mismatch: None,
                    editable: true,
                    disabled_reason: None,
                    feedback: Rc::from([]),
                    summary_feedback: None,
                    on_request: move |request: MechanicalSettingsRequest| {
                        last_request.set(format!("{:?}", request.patch))
                    },
                    on_select_layer: move |_| {},
                    on_show_finding: move |_| {},
                }
            }
            div { id: "case-battery-last-request", "{last_request}" }
        }
    }

    fn element(selector: &str) -> web_sys::HtmlElement {
        web_sys::window()
            .unwrap()
            .document()
            .unwrap()
            .query_selector(selector)
            .unwrap_or_else(|error| panic!("invalid selector {selector}: {error:?}"))
            .unwrap_or_else(|| panic!("no element matches {selector}"))
            .dyn_into()
            .unwrap_or_else(|_| panic!("element matching {selector} is not an HTMLElement"))
    }

    async fn rendered() {
        gloo_timers::future::TimeoutFuture::new(60).await;
    }

    fn mount_battery_test_page(root_id: &'static str, page: fn() -> Element) {
        let document = web_sys::window().unwrap().document().unwrap();
        let root = document.create_element("div").unwrap();
        root.set_id(root_id);
        document.body().unwrap().append_child(&root).unwrap();
        dioxus_web::launch::launch_virtual_dom(
            VirtualDom::new(page),
            dioxus_web::Config::new().rootnode(root.into()),
        );
    }

    #[wasm_bindgen_test]
    async fn selected_plate_uses_contextual_inspector_and_returns_to_assembly_settings() {
        let document = web_sys::window().unwrap().document().unwrap();
        let stylesheet = document.create_element("style").unwrap();
        stylesheet.set_text_content(Some(include_str!("../../../assets/m1.css")));
        document.head().unwrap().append_child(&stylesheet).unwrap();
        let root = document.create_element("div").unwrap();
        root.set_id("case-contextual-mechanical-settings-test-root");
        document.body().unwrap().append_child(&root).unwrap();
        dioxus_web::launch::launch_virtual_dom(
            VirtualDom::new(test_page),
            dioxus_web::Config::new().rootnode(root.into()),
        );
        rendered().await;

        let panel =
            element("#case-contextual-mechanical-settings-test-root .m1-mechanical-settings");
        let text = panel.text_content().unwrap_or_default();
        assert!(text.contains("Plate"));
        assert!(text.contains("Assembly settings"));
        assert!(text.contains("Plate thickness"));
        assert!(text.contains("Resolved thickness 1.50 mm."));
        assert!(text.contains("Current clearance error"));
        assert!(!text.contains("Current warning"));
        assert!(!text.contains("Case construction"));
        assert!(!text.contains("Bottom thickness"));
        element("#case-contextual-mechanical-settings-test-root .m1-mechanical-group[aria-label='Fit issues'] button").click();
        rendered().await;
        assert_eq!(
            element("#case-contextual-selected-finding")
                .text_content()
                .as_deref(),
            Some("current-error")
        );

        element("#case-contextual-mechanical-settings-test-root .m1-mechanical-context-return")
            .click();
        rendered().await;
        let assembly_panel =
            element("#case-contextual-mechanical-settings-test-root .m1-mechanical-settings");
        assert!(
            assembly_panel
                .query_selector("section[aria-label='Manufacturing overrides']")
                .unwrap()
                .is_some(),
            "the assembly settings should expose per-part manufacturing controls"
        );
        let manufacturing_text = assembly_panel.text_content().unwrap_or_default();
        assert!(manufacturing_text.contains("Per-part process overrides 4"));
        assert!(manufacturing_text.contains("Stabilizer fit 0 wide keys"));
        assert_eq!(
            assembly_panel
                .query_selector_all("select[aria-label='Material']")
                .unwrap()
                .length(),
            4,
            "each accepted standard process override should expose its material"
        );
        assert_eq!(
            assembly_panel
                .text_content()
                .unwrap_or_default()
                .matches("Add process override")
                .count(),
            1
        );
        let text =
            element("#case-contextual-mechanical-settings-test-root .m1-mechanical-settings")
                .text_content()
                .unwrap_or_default();
        assert!(text.contains("Case construction"));

        element("#case-contextual-mechanical-settings-test-root .m1-mechanical-stack button:nth-child(2)").click();
        rendered().await;
        let text =
            element("#case-contextual-mechanical-settings-test-root .m1-mechanical-settings")
                .text_content()
                .unwrap_or_default();
        assert!(text.contains("PCB thickness"));
        assert!(!text.contains("Plate thickness"));
        assert!(!text.contains("Resolved thickness"));
        element("#case-contextual-mechanical-settings-test-root .m1-mechanical-context-return")
            .click();
        rendered().await;

        element("#case-contextual-mechanical-settings-test-root .m1-mechanical-stack button:nth-child(3)").click();
        rendered().await;
        let text =
            element("#case-contextual-mechanical-settings-test-root .m1-mechanical-settings")
                .text_content()
                .unwrap_or_default();
        assert!(text.contains("Plate foam"));
        assert!(!text.contains("PCB thickness"));
        element("#case-contextual-mechanical-settings-test-root .m1-mechanical-context-return")
            .click();
        rendered().await;

        element("#case-contextual-mechanical-settings-test-root .m1-mechanical-stack button:nth-child(4)").click();
        rendered().await;
        let text =
            element("#case-contextual-mechanical-settings-test-root .m1-mechanical-settings")
                .text_content()
                .unwrap_or_default();
        assert!(text.contains("Bottom foam"));
        assert!(!text.contains("PCB thickness"));
        element("#case-contextual-mechanical-settings-test-root .m1-mechanical-context-return")
            .click();
        rendered().await;

        element("#case-contextual-mechanical-settings-test-root .m1-mechanical-stack button:nth-child(5)").click();
        rendered().await;
        let text =
            element("#case-contextual-mechanical-settings-test-root .m1-mechanical-settings")
                .text_content()
                .unwrap_or_default();
        assert!(text.contains("Bottom case"));
        assert!(text.contains("Bottom thickness"));
        assert!(text.contains("Wall thickness"));
        assert!(text.contains("Clearance"));
        assert!(text.contains("Resolved thickness 3.00 mm."));
        assert!(text.contains("Edit shared closure hardware"));
        assert!(!text.contains("Plate thickness"));
        assert!(!text.contains("Current warning"));

        element("#case-contextual-mechanical-settings-test-root .m1-mechanical-context-hardware")
            .click();
        rendered().await;
        let text =
            element("#case-contextual-mechanical-settings-test-root .m1-mechanical-settings")
                .text_content()
                .unwrap_or_default();
        assert!(text.contains("Case construction"));

        element("#case-contextual-mechanical-settings-test-root .m1-mechanical-stack button:nth-child(6)").click();
        rendered().await;
        let text =
            element("#case-contextual-mechanical-settings-test-root .m1-mechanical-settings")
                .text_content()
                .unwrap_or_default();
        assert!(text.contains("Top case"));
        assert!(text.contains("Wall thickness"));
        assert!(text.contains("Clearance"));
        assert!(!text.contains("Resolved thickness"));
        assert!(!text.contains("Plate thickness"));
    }

    #[wasm_bindgen_test]
    async fn unassigned_profile_guidance_shows_saved_plate_to_pcb_gap() {
        mount_battery_test_page(
            "case-empty-profile-guidance-test-root",
            empty_profile_guidance_test_page,
        );
        rendered().await;
        let text = element("#case-empty-profile-guidance-test-root")
            .text_content()
            .unwrap_or_default();
        assert!(
            text.contains("Plate underside to PCB top: 3.00 mm"),
            "show the accepted plate gap even when no explicit profile row is assigned"
        );
    }

    #[wasm_bindgen_test]
    async fn process_thickness_draft_escape_enter_and_blur_follow_numeric_field_semantics() {
        mount_battery_test_page(
            "case-process-thickness-test-root-host",
            ProcessThicknessTestPage,
        );
        rendered().await;
        let input =
            element("#case-process-thickness-test-root input[aria-label='Finished thickness']")
                .dyn_into::<web_sys::HtmlInputElement>()
                .unwrap();
        let dispatch_input = |input: &web_sys::HtmlInputElement, value: &str| {
            input.set_value(value);
            let init = web_sys::EventInit::new();
            init.set_bubbles(true);
            input
                .dispatch_event(&web_sys::Event::new_with_event_init_dict("input", &init).unwrap())
                .unwrap();
        };
        let dispatch_key = |input: &web_sys::HtmlInputElement, key: &str| {
            let init = web_sys::KeyboardEventInit::new();
            init.set_key(key);
            init.set_bubbles(true);
            input
                .dispatch_event(
                    &web_sys::KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &init)
                        .unwrap(),
                )
                .unwrap();
        };

        input.focus().unwrap();
        dispatch_input(&input, "2.5");
        dispatch_key(&input, "Escape");
        input.blur();
        let change = web_sys::EventInit::new();
        change.set_bubbles(true);
        input
            .dispatch_event(&web_sys::Event::new_with_event_init_dict("change", &change).unwrap())
            .unwrap();
        rendered().await;
        assert_eq!(input.value(), "1.5");
        assert_eq!(
            element("#case-process-thickness-request-count")
                .text_content()
                .as_deref(),
            Some("0"),
            "Escape then blur should restore the accepted value without submitting"
        );

        input.focus().unwrap();
        dispatch_input(&input, "2.0");
        dispatch_key(&input, "Enter");
        input.blur();
        let change = web_sys::EventInit::new();
        change.set_bubbles(true);
        input
            .dispatch_event(&web_sys::Event::new_with_event_init_dict("change", &change).unwrap())
            .unwrap();
        rendered().await;
        assert_eq!(
            element("#case-process-thickness-request-count")
                .text_content()
                .as_deref(),
            Some("1"),
            "Enter followed by blur should submit the thickness once"
        );

        input.focus().unwrap();
        dispatch_input(&input, "0.05");
        input.blur();
        rendered().await;
        assert_eq!(
            element("#case-process-thickness-request-count")
                .text_content()
                .as_deref(),
            Some("1"),
            "thickness below 0.1 mm should not submit"
        );
    }

    #[wasm_bindgen_test]
    async fn previous_generated_layer_context_stays_visible_and_is_labeled() {
        mount_battery_test_page("case-previous-settings-test-root", previous_test_page);
        rendered().await;
        let panel = element("#case-previous-settings-test-root .m1-mechanical-settings");
        let text = panel.text_content().unwrap_or_default();
        assert!(text.contains("previous generated geometry"));
        assert!(text.contains("Resolved thickness 1.50 mm."));
        assert!(text.contains("Plate thickness"));

        element("#case-previous-settings-test-root .m1-mechanical-context-return").click();
        rendered().await;
        let text = element("#case-previous-settings-test-root .m1-mechanical-settings")
            .text_content()
            .unwrap_or_default();
        assert!(text.contains("Previous resolved stack"));
        assert!(text.contains("Settings below reflect the current accepted configuration."));
        assert!(text.contains("Previous revision"));
    }

    #[wasm_bindgen_test]
    async fn battery_controls_match_wired_and_wireless_modes_and_dispatch_toggle() {
        mount_battery_test_page("case-battery-settings-test-root", BatteryTestPage);
        rendered().await;

        let panel = element("#case-battery-wired-test-root .m1-mechanical-settings");
        assert!(
            panel
                .text_content()
                .unwrap_or_default()
                .contains("Include a battery envelope"),
            "wired mechanical settings should expose the optional battery envelope toggle"
        );
        let toggle =
            element("#case-battery-wired-test-root input[aria-label='Include a battery envelope']")
                .dyn_into::<web_sys::HtmlInputElement>()
                .unwrap();
        assert!(!toggle.checked());
        assert!(!panel.text_content().unwrap_or_default().contains("Width"));

        toggle.set_checked(true);
        let change = web_sys::EventInit::new();
        change.set_bubbles(true);
        toggle
            .dispatch_event(&web_sys::Event::new_with_event_init_dict("change", &change).unwrap())
            .unwrap();
        rendered().await;
        assert_eq!(
            element("#case-battery-last-request")
                .text_content()
                .as_deref(),
            Some("SetBatteryEnabled(true)")
        );

        let panel = element("#case-battery-wireless-test-root .m1-mechanical-settings");
        let text = panel.text_content().unwrap_or_default();
        assert!(text.contains("Included for wireless."));
        assert!(!text.contains("Include a battery envelope"));
        for label in [
            "Width",
            "Depth",
            "Height",
            "Cable width",
            "Position X",
            "Position Y",
            "Cable exit X",
            "Cable exit Y",
        ] {
            assert!(
                text.contains(label),
                "wireless battery field {label} should be visible"
            );
        }

        let document = web_sys::window().unwrap().document().unwrap();
        let inputs = document
            .query_selector_all(
                "#case-battery-wireless-test-root .m1-mechanical-battery-fields input[type='number']",
            )
            .unwrap();
        assert_eq!(inputs.length(), 8);
        let expected_minimums = [
            "0.1", "0.1", "0.1", "0.1", "-1000000", "-1000000", "-1000000", "-1000000",
        ];
        for (index, minimum) in expected_minimums.into_iter().enumerate() {
            let input = inputs
                .item(index as u32)
                .unwrap()
                .dyn_into::<web_sys::HtmlInputElement>()
                .unwrap();
            assert_eq!(input.step(), "0.1");
            assert_eq!(input.min(), minimum);
        }
        let cable_width = inputs
            .item(3)
            .unwrap()
            .dyn_into::<web_sys::HtmlInputElement>()
            .unwrap();
        assert_eq!(cable_width.value().parse::<f64>().unwrap(), 2.0);
    }

    #[component]
    fn PairFeatureTestPage() -> Element {
        let request_sequence = use_signal(|| 0_u64);
        let mut last_request = use_signal(String::new);
        let rows: Rc<[MechanicalPairFeatureRow]> = Rc::from([
            MechanicalPairFeatureRow {
                feature_id: "mount-1-left".into(),
                kind: MechanicalFeatureKind::Mount,
                state: MechanicalFeatureState::Linked,
                instance_id: "left".into(),
                counterpart_instance_id: "right".into(),
                counterpart_feature_id: "mount-1-right".into(),
                preview: Some("Mount at (2.0, 3.0)".into()),
                own_geometry: Some("Mount at (2.0, 3.0)".into()),
                rejoin_fingerprint: Some("reviewed-mount".into()),
            },
            MechanicalPairFeatureRow {
                feature_id: "opening-1-left".into(),
                kind: MechanicalFeatureKind::Opening,
                state: MechanicalFeatureState::Local,
                instance_id: "left".into(),
                counterpart_instance_id: "right".into(),
                counterpart_feature_id: "opening-1-right".into(),
                preview: Some("Opening with 3 vertices".into()),
                own_geometry: Some("Opening with 4 vertices".into()),
                rejoin_fingerprint: Some("reviewed-opening".into()),
            },
            MechanicalPairFeatureRow {
                feature_id: "support-1-left".into(),
                kind: MechanicalFeatureKind::GasketSupport,
                state: MechanicalFeatureState::Omitted,
                instance_id: "left".into(),
                counterpart_instance_id: "right".into(),
                counterpart_feature_id: "support-1-right".into(),
                preview: None,
                own_geometry: None,
                rejoin_fingerprint: Some("reviewed-support".into()),
            },
        ]);
        rsx! {
            div { id: "case-pair-feature-test-root",
                MechanicalSettings {
                    identity: test_identity(),
                    request_sequence,
                    values: Some(test_values(HardwareTransport::Wired, None)),
                    profiles: Rc::from([]),
                    profile_targets: Rc::from([]),
                    process_targets: Rc::from([]),
                    stabilizer_fits: Rc::from([]),
                    layers: Rc::from([]),
                    gasket_supports: Rc::from([]),
                    fit_parts: Rc::from([]),
                    fit_parts_resolved: false,
                    hardware_mounts: Rc::from([]),
                    suggested_mounts: Rc::from([]),
                    findings: Rc::from([]),
                    pair_features: Some(rows),
                    selected_layer: String::new(),
                    mismatch: None,
                    editable: true,
                    disabled_reason: None,
                    feedback: Rc::from([]),
                    summary_feedback: None,
                    on_request: move |request: MechanicalSettingsRequest| {
                        last_request.set(format!(
                            "{:?}|reviewed={:?}",
                            request.patch, request.reviewed_fingerprint
                        ));
                    },
                    on_select_layer: move |_| {},
                    on_show_finding: move |_| {},
                }
                p { id: "case-pair-feature-last-request", "{last_request}" }
            }
        }
    }

    #[wasm_bindgen_test]
    async fn pair_feature_controls_are_reachable_and_submit_typed_pair_changes() {
        let document = web_sys::window().unwrap().document().unwrap();
        let root = document.create_element("div").unwrap();
        root.set_id("case-pair-feature-test-host");
        document.body().unwrap().append_child(&root).unwrap();
        dioxus_web::launch::launch_virtual_dom(
            VirtualDom::new(PairFeatureTestPage),
            dioxus_web::Config::new().rootnode(root.clone().into()),
        );
        rendered().await;

        // The panel renders the accepted pair state for every covered kind.
        assert_eq!(
            element(".m1-mechanical-pair h3").text_content().unwrap(),
            "Mirrored pair"
        );
        assert_eq!(
            element(".m1-mechanical-pair-row:nth-of-type(1) legend")
                .text_content()
                .unwrap(),
            "Mount · linked to the other half"
        );

        // Linked: make local, or remove on this half.
        element("button[aria-label='Make local mount-1-left']").click();
        rendered().await;
        let local = element("#case-pair-feature-last-request")
            .text_content()
            .unwrap();
        assert!(
            local.contains("MakeFeatureLocal") && local.contains("mount-1-left"),
            "the linked row submits an explicit make-local, got {local}"
        );
        element("button[aria-label='Remove on this half mount-1-left']").click();
        rendered().await;
        let omit = element("#case-pair-feature-last-request")
            .text_content()
            .unwrap();
        assert!(
            omit.contains("OmitFeatureMember") && omit.contains("mount-1-left"),
            "the linked row can remove the feature on this half, got {omit}"
        );

        // Local: the preview is shown before the commit, and rejoin names its source.
        assert!(
            element(".m1-mechanical-pair-row:nth-of-type(2) .m1-mechanical-help")
                .text_content()
                .unwrap()
                .contains("Opening with 3 vertices"),
            "a rejoin previews the geometry it will take"
        );
        element("button[aria-label='Rejoin opening-1-left from the other half']").click();
        rendered().await;
        let rejoin = element("#case-pair-feature-last-request")
            .text_content()
            .unwrap();
        assert!(
            rejoin.contains("RejoinFeatureMember")
                && rejoin.contains("opening-1-left")
                && rejoin.contains("\"right\""),
            "rejoin submits the reviewed source half, got {rejoin}"
        );
        assert!(
            rejoin.contains("reviewed=Some(\"reviewed-opening\")"),
            "the rejoin submission carries the fingerprint of the row the user reviewed, got {rejoin}"
        );

        // A local half can deliberately take its OWN geometry as the shared value.
        let rejoin_source = element("select[aria-label='Rejoin source for opening-1-left']");
        assert!(
            element(".m1-mechanical-pair-row:nth-of-type(2)")
                .text_content()
                .unwrap()
                .contains("Opening with 4 vertices"),
            "the review shows this half's own geometry as a source choice"
        );
        js_sys::Reflect::set(&rejoin_source, &"value".into(), &"left".into()).unwrap();
        let source_change = web_sys::EventInit::new();
        source_change.set_bubbles(true);
        rejoin_source
            .dispatch_event(
                &web_sys::Event::new_with_event_init_dict("change", &source_change).unwrap(),
            )
            .unwrap();
        rendered().await;
        assert!(
            element(".m1-mechanical-pair-row:nth-of-type(2)")
                .text_content()
                .unwrap()
                .contains("Rejoin takes Opening with 4 vertices from left"),
            "the preview follows the chosen source, got {}",
            element(".m1-mechanical-pair-row:nth-of-type(2)")
                .text_content()
                .unwrap()
        );
        element("button[aria-label='Rejoin opening-1-left from this half']").click();
        rendered().await;
        let own_source = element("#case-pair-feature-last-request")
            .text_content()
            .unwrap();
        assert!(
            own_source.contains("RejoinFeatureMember")
                && own_source.contains("source_instance_id: \"left\""),
            "the deliberate own-source rejoin is submitted as reviewed, got {own_source}"
        );

        // Omitted without counterpart geometry: reachable but not committable.
        let last_reviewed = element("#case-pair-feature-last-request")
            .text_content()
            .unwrap();
        let document = web_sys::window().unwrap().document().unwrap();
        assert!(
            document
                .query_selector(
                    "button[aria-label='Rejoin support-1-left from the other half']:disabled"
                )
                .unwrap()
                .is_some(),
            "an omitted feature whose other half has no geometry cannot be rejoined"
        );
        assert!(
            element(".m1-mechanical-pair-row:nth-of-type(3) .m1-mechanical-help")
                .text_content()
                .unwrap()
                .contains("no geometry"),
        );
        let after = element("#case-pair-feature-last-request")
            .text_content()
            .unwrap();
        assert_eq!(
            after, last_reviewed,
            "the disabled rejoin submits nothing when clicked or left alone"
        );
        let _ = MechanicalFeatureKind::GasketSupport;
        root.remove();
    }

    #[component]
    fn PairedDisableTestPage() -> Element {
        let request_sequence = use_signal(|| 0_u64);
        let mut last_request = use_signal(String::new);
        let mut requests = use_signal(|| 0_u32);
        rsx! {
            div { id: "case-paired-disable-test-root",
                MechanicalSettings {
                    identity: test_identity(),
                    request_sequence,
                    values: Some(test_values(HardwareTransport::Wired, None)),
                    profiles: Rc::from([]),
                    profile_targets: Rc::from([]),
                    process_targets: Rc::from([]),
                    stabilizer_fits: Rc::from([]),
                    layers: Rc::from([]),
                    gasket_supports: Rc::from([]),
                    fit_parts: Rc::from([]),
                    fit_parts_resolved: false,
                    hardware_mounts: Rc::from([]),
                    suggested_mounts: Rc::from([]),
                    findings: Rc::from([]),
                    pair_counterpart: Some("right".into()),
                    selected_layer: String::new(),
                    mismatch: None,
                    editable: true,
                    disabled_reason: None,
                    feedback: Rc::from([]),
                    summary_feedback: None,
                    on_request: move |request: MechanicalSettingsRequest| {
                        requests += 1;
                        last_request.set(format!("{:?}", request.patch));
                    },
                    on_select_layer: move |_| {},
                    on_show_finding: move |_| {},
                }
                p { id: "case-paired-disable-last-request", "{last_request}" }
                p { id: "case-paired-disable-count", "{requests}" }
            }
        }
    }

    #[component]
    fn PairedRemovalTestPage() -> Element {
        let request_sequence = use_signal(|| 0_u64);
        let mut last_request = use_signal(String::new);
        let mut requests = use_signal(|| 0_u32);
        rsx! {
            div { id: "case-paired-removal-test-root",
                MechanicalSettings {
                    identity: test_identity_with_instance("left"),
                    request_sequence,
                    values: Some(test_values(HardwareTransport::Wired, None)),
                    profiles: Rc::from([]),
                    profile_targets: Rc::from([]),
                    process_targets: Rc::from([]),
                    stabilizer_fits: Rc::from([]),
                    layers: Rc::from([]),
                    gasket_supports: Rc::from([]),
                    fit_parts: Rc::from([]),
                    fit_parts_resolved: false,
                    hardware_mounts: Rc::from([]),
                    suggested_mounts: Rc::from([]),
                    findings: Rc::from([]),
                    pair_counterpart: Some("right".into()),
                    selected_layer: String::new(),
                    mismatch: None,
                    editable: true,
                    disabled_reason: None,
                    feedback: Rc::from([]),
                    summary_feedback: None,
                    on_request: move |request: MechanicalSettingsRequest| {
                        requests += 1;
                        last_request.set(format!("{:?}", request.patch));
                    },
                    on_select_layer: move |_| {},
                    on_show_finding: move |_| {},
                }
                p { id: "case-paired-removal-last-request", "{last_request}" }
                p { id: "case-paired-removal-count", "{requests}" }
            }
        }
    }

    #[component]
    fn LegacyConstructionTestPage() -> Element {
        let request_sequence = use_signal(|| 0_u64);
        let mut last_request = use_signal(String::new);
        let mut requests = use_signal(|| 0_u32);
        rsx! {
            div { id: "case-legacy-construction-test-root",
                MechanicalSettings {
                    identity: test_identity_with_instance("left"),
                    request_sequence,
                    values: Some(test_values(HardwareTransport::Wired, None)),
                    profiles: Rc::from([]),
                    profile_targets: Rc::from([]),
                    process_targets: Rc::from([]),
                    stabilizer_fits: Rc::from([]),
                    layers: Rc::from([]),
                    gasket_supports: Rc::from([]),
                    fit_parts: Rc::from([]),
                    fit_parts_resolved: false,
                    hardware_mounts: Rc::from([]),
                    suggested_mounts: Rc::from([]),
                    findings: Rc::from([]),
                    construction_is_legacy: true,
                    selected_layer: String::new(),
                    mismatch: None,
                    editable: true,
                    disabled_reason: None,
                    feedback: Rc::from([]),
                    summary_feedback: None,
                    on_request: move |request: MechanicalSettingsRequest| {
                        requests += 1;
                        last_request.set(format!("{:?}", request.patch));
                    },
                    on_select_layer: move |_| {},
                    on_show_finding: move |_| {},
                }
                p { id: "case-legacy-construction-last-request", "{last_request}" }
                p { id: "case-legacy-construction-count", "{requests}" }
            }
        }
    }

    #[wasm_bindgen_test]
    async fn a_legacy_half_converts_its_construction_only_after_confirming_the_change() {
        let document = web_sys::window().unwrap().document().unwrap();
        let root = document.create_element("div").unwrap();
        root.set_id("case-legacy-construction-test-host");
        document.body().unwrap().append_child(&root).unwrap();
        dioxus_web::launch::launch_virtual_dom(
            VirtualDom::new(LegacyConstructionTestPage),
            dioxus_web::Config::new().rootnode(root.clone().into()),
        );
        rendered().await;

        // Reachable only for a legacy half, and it submits nothing on its own.
        let make_local = element("button[aria-label='Make construction local for left']");
        assert!(
            element("#case-legacy-construction-last-request")
                .text_content()
                .unwrap()
                .is_empty(),
            "showing the conversion control submits nothing"
        );

        // The confirmation states the whole change before anything is submitted.
        make_local.click();
        rendered().await;
        let confirmation = element(".m1-mechanical-legacy-construction")
            .text_content()
            .unwrap();
        assert!(
            confirmation.contains("left")
                && confirmation.contains("this half's local values")
                && confirmation.contains("Other legacy halves keep their previous baseline")
                && confirmation.contains("One Undo"),
            "the confirmation states what converting construction does, got {confirmation}"
        );
        assert!(
            element("#case-legacy-construction-last-request")
                .text_content()
                .unwrap()
                .is_empty(),
            "asking for confirmation submits nothing"
        );

        // Cancelling keeps the shared construction.
        element("button[aria-label='Keep shared construction for left']").click();
        rendered().await;
        assert!(
            element("#case-legacy-construction-last-request")
                .text_content()
                .unwrap()
                .is_empty(),
            "cancelling submits nothing"
        );

        // Confirming submits exactly one typed conversion naming one instance.
        element("button[aria-label='Make construction local for left']").click();
        rendered().await;
        element("button[aria-label='Confirm making construction local for left']").click();
        rendered().await;
        let submitted = element("#case-legacy-construction-last-request")
            .text_content()
            .unwrap();
        assert!(
            submitted.contains("MaterializeLegacyConstruction")
                && submitted.contains("instance_ids: [\"left\"]"),
            "one typed conversion names exactly this half, got {submitted}"
        );
        assert_eq!(
            element("#case-legacy-construction-count")
                .text_content()
                .unwrap(),
            "1",
            "exactly one conversion edit is submitted"
        );
        root.remove();
    }

    #[wasm_bindgen_test]
    async fn a_paired_half_removes_only_after_confirming_what_happens() {
        let document = web_sys::window().unwrap().document().unwrap();
        let root = document.create_element("div").unwrap();
        root.set_id("case-paired-removal-test-host");
        document.body().unwrap().append_child(&root).unwrap();
        dioxus_web::launch::launch_virtual_dom(
            VirtualDom::new(PairedRemovalTestPage),
            dioxus_web::Config::new().rootnode(root.clone().into()),
        );
        rendered().await;

        // The control exists for a linked half and nothing is submitted on its own.
        let remove = element("button[aria-label='Remove assembly left']");
        assert!(
            element("#case-paired-removal-last-request")
                .text_content()
                .unwrap()
                .is_empty(),
            "showing the removal control submits nothing"
        );

        // The first click asks for explicit confirmation of the whole change.
        remove.click();
        rendered().await;
        let confirmation = element(".m1-mechanical-remove-member")
            .text_content()
            .unwrap();
        assert!(
            confirmation.contains("right")
                && confirmation.contains("keeps the geometry and settings")
                && confirmation.contains("boards, PCBs and parts stay")
                && confirmation.contains("One Undo"),
            "the confirmation says what removing this half does, got {confirmation}"
        );
        assert!(
            element("#case-paired-removal-last-request")
                .text_content()
                .unwrap()
                .is_empty(),
            "asking for confirmation submits nothing"
        );

        // Cancelling keeps everything.
        element("button[aria-label='Keep assembly left']").click();
        rendered().await;
        assert!(
            element("#case-paired-removal-last-request")
                .text_content()
                .unwrap()
                .is_empty(),
            "cancelling submits nothing"
        );

        // Confirming submits exactly one typed physical-instance removal.
        element("button[aria-label='Remove assembly left']").click();
        rendered().await;
        element("button[aria-label='Confirm removing assembly left']").click();
        rendered().await;
        let submitted = element("#case-paired-removal-last-request")
            .text_content()
            .unwrap();
        assert!(
            submitted.contains("RemovePhysicalInstance"),
            "the confirmation submits the typed removal, got {submitted}"
        );
        assert_eq!(
            element("#case-paired-removal-count")
                .text_content()
                .unwrap(),
            "1",
            "one typed removal is submitted"
        );
        root.remove();
    }

    #[wasm_bindgen_test]
    async fn a_paired_half_disables_with_an_explicit_unlink_explanation_and_one_edit() {
        let document = web_sys::window().unwrap().document().unwrap();
        let root = document.create_element("div").unwrap();
        root.set_id("case-paired-disable-test-host");
        document.body().unwrap().append_child(&root).unwrap();
        dioxus_web::launch::launch_virtual_dom(
            VirtualDom::new(PairedDisableTestPage),
            dioxus_web::Config::new().rootnode(root.clone().into()),
        );
        rendered().await;

        let explanation = element(".m1-mechanical-disable ~ .m1-mechanical-help")
            .text_content()
            .unwrap();
        assert!(
            explanation.contains("right")
                && explanation.contains("keeps its geometry and settings")
                && explanation.contains("neither PCB is removed")
                && explanation.contains("One Undo"),
            "the panel explains what disabling a linked half does, got {explanation}"
        );
        element("button[aria-label='Unlink and disable this half']").click();
        rendered().await;
        let submitted = element("#case-paired-disable-last-request")
            .text_content()
            .unwrap();
        assert!(
            submitted.contains("Disable") && !submitted.contains("Unlink"),
            "one typed Disable carries the whole decision, got {submitted}"
        );
        assert_eq!(
            element("#case-paired-disable-count")
                .text_content()
                .unwrap(),
            "1",
            "exactly one edit is submitted, with no separate unlink item"
        );
        root.remove();
    }

    fn link_options() -> Rc<[MechanicalPairLinkOption]> {
        Rc::from([
            MechanicalPairLinkOption {
                pair_id: "pair/left".into(),
                instance_id: "left".into(),
                other_instance_id: "right".into(),
                other_name: "Right half".into(),
                derived_mapping: Some(Pose2 {
                    at: Vec2::default(),
                    rotation: 0.0,
                }),
                construction_source: "left".into(),
                correspondences: vec![
                    boardstudio_core::mechanical::change::MechanicalFeatureCorrespondence {
                        kind: MechanicalFeatureKind::Mount,
                        first_feature_id: "mount-1".into(),
                        second_feature_id: "mount-1".into(),
                        source_instance_id: None,
                    },
                ],
                matched_features: "Mount mount-1".into(),
                matched: vec![crate::mechanical_pair::MechanicalPairMatchedFeature {
                    kind: MechanicalFeatureKind::Mount,
                    first_feature_id: "mount-1-left".into(),
                    second_feature_id: "mount-1-right".into(),
                    first_geometry: Some("Mount at (2.0, 3.0)".into()),
                    second_geometry: Some("Mount at (-2.0, 3.0)".into()),
                    source_instance_id: "left".into(),
                }],
                reviewed_fingerprint: Some("reviewed-link".into()),
                preview: Some(crate::mechanical_pair::MechanicalLinkPreview {
                    members: vec![
                        crate::mechanical_pair::MechanicalLinkMemberPreview {
                            instance_id: "left".into(),
                            features: vec!["Mount at (2.0, 3.0)".into()],
                            findings: vec![],
                        },
                        crate::mechanical_pair::MechanicalLinkMemberPreview {
                            instance_id: "right".into(),
                            features: vec!["Mount at (-2.0, 3.0)".into()],
                            findings: vec![],
                        },
                    ],
                }),
                preview_note: None,
            },
            MechanicalPairLinkOption {
                pair_id: "pair/left".into(),
                instance_id: "left".into(),
                other_instance_id: "lower".into(),
                other_name: "Lower half".into(),
                derived_mapping: None,
                construction_source: "left".into(),
                correspondences: vec![
                    boardstudio_core::mechanical::change::MechanicalFeatureCorrespondence {
                        kind: MechanicalFeatureKind::Mount,
                        first_feature_id: "mount-1-left".into(),
                        second_feature_id: "mount-1-right".into(),
                        source_instance_id: Some("left".into()),
                    },
                ],
                matched_features: "Mount mount-1-left · mount-1-right".into(),
                matched: vec![crate::mechanical_pair::MechanicalPairMatchedFeature {
                    kind: MechanicalFeatureKind::Mount,
                    first_feature_id: "mount-1-left".into(),
                    second_feature_id: "mount-1-right".into(),
                    first_geometry: Some("Mount at (2.0, 3.0)".into()),
                    second_geometry: Some("Mount at (-2.0, 3.0)".into()),
                    source_instance_id: "left".into(),
                }],
                reviewed_fingerprint: Some("reviewed-link-explicit".into()),
                preview: None,
                preview_note: Some("the reviewed configuration needs a catalogue template".into()),
            },
        ])
    }

    /// A stand-in for the mount's projection port: it reports the reviewed map it was
    /// given, which is what the review displays and commits.
    fn echo_projection_port() -> crate::mechanical_pair::MechanicalPairProjectionPort {
        crate::mechanical_pair::MechanicalPairProjectionPort(Rc::new(|change| {
            let boardstudio_core::mechanical::change::MechanicalChange::LinkMirrorPair {
                mapping,
                ..
            } = change
            else {
                return Err("not a reviewed link".into());
            };
            let text = format!(
                "offset ({:?}, {:?}) rotation {:?}",
                mapping.at.x, mapping.at.y, mapping.rotation
            );
            Ok(crate::mechanical_pair::MechanicalLinkPreview {
                members: vec![
                    crate::mechanical_pair::MechanicalLinkMemberPreview {
                        instance_id: "left".into(),
                        features: vec![text.clone()],
                        findings: vec![],
                    },
                    crate::mechanical_pair::MechanicalLinkMemberPreview {
                        instance_id: "right".into(),
                        features: vec![text],
                        findings: vec![],
                    },
                ],
            })
        }))
    }

    #[component]
    fn PairLinkTestPage() -> Element {
        let request_sequence = use_signal(|| 0_u64);
        let mut last_request = use_signal(String::new);
        rsx! {
            div { id: "case-pair-link-test-root",
                MechanicalSettings {
                    identity: test_identity(),
                    request_sequence,
                    values: Some(test_values(HardwareTransport::Wired, None)),
                    profiles: Rc::from([]),
                    profile_targets: Rc::from([]),
                    process_targets: Rc::from([]),
                    stabilizer_fits: Rc::from([]),
                    layers: Rc::from([]),
                    gasket_supports: Rc::from([]),
                    fit_parts: Rc::from([]),
                    fit_parts_resolved: false,
                    hardware_mounts: Rc::from([]),
                    suggested_mounts: Rc::from([]),
                    findings: Rc::from([]),
                    pair_link_options: Some(link_options()),
                    pair_projection_port: Some(echo_projection_port()),
                    selected_layer: String::new(),
                    mismatch: None,
                    editable: true,
                    disabled_reason: None,
                    feedback: Rc::from([]),
                    summary_feedback: None,
                    on_request: move |request: MechanicalSettingsRequest| {
                        last_request.set(format!(
                            "{:?}|reviewed={:?}",
                            request.patch, request.reviewed_fingerprint
                        ));
                    },
                    on_select_layer: move |_| {},
                    on_show_finding: move |_| {},
                }
                p { id: "case-pair-link-last-request", "{last_request}" }
            }
        }
    }

    /// Link candidates with no offer-time projection, so only the port can supply one.
    fn port_review_options() -> Rc<[MechanicalPairLinkOption]> {
        Rc::from([
            MechanicalPairLinkOption {
                pair_id: "pair/left".into(),
                instance_id: "left".into(),
                other_instance_id: "right".into(),
                other_name: "Right half".into(),
                derived_mapping: Some(Pose2 {
                    at: Vec2::default(),
                    rotation: 0.0,
                }),
                construction_source: "left".into(),
                correspondences: vec![
                    boardstudio_core::mechanical::change::MechanicalFeatureCorrespondence {
                        kind: MechanicalFeatureKind::Mount,
                        first_feature_id: "mount-1-left".into(),
                        second_feature_id: "mount-1-right".into(),
                        source_instance_id: Some("left".into()),
                    },
                ],
                matched_features: "Mount mount-1-left · mount-1-right".into(),
                matched: vec![crate::mechanical_pair::MechanicalPairMatchedFeature {
                    kind: MechanicalFeatureKind::Mount,
                    first_feature_id: "mount-1-left".into(),
                    second_feature_id: "mount-1-right".into(),
                    first_geometry: Some("Mount at (2.0, 3.0)".into()),
                    second_geometry: Some("Mount at (-2.0, 3.0)".into()),
                    source_instance_id: "left".into(),
                }],
                reviewed_fingerprint: Some("reviewed-port".into()),
                preview: None,
                preview_note: Some("no offer-time projection".into()),
            },
            MechanicalPairLinkOption {
                pair_id: "pair/left".into(),
                instance_id: "left".into(),
                other_instance_id: "lower".into(),
                other_name: "Lower half".into(),
                derived_mapping: None,
                construction_source: "left".into(),
                correspondences: vec![
                    boardstudio_core::mechanical::change::MechanicalFeatureCorrespondence {
                        kind: MechanicalFeatureKind::Mount,
                        first_feature_id: "mount-1-left".into(),
                        second_feature_id: "mount-1-right".into(),
                        source_instance_id: Some("left".into()),
                    },
                ],
                matched_features: "Mount mount-1-left · mount-1-right".into(),
                matched: vec![crate::mechanical_pair::MechanicalPairMatchedFeature {
                    kind: MechanicalFeatureKind::Mount,
                    first_feature_id: "mount-1-left".into(),
                    second_feature_id: "mount-1-right".into(),
                    first_geometry: Some("Mount at (2.0, 3.0)".into()),
                    second_geometry: Some("Mount at (-2.0, 3.0)".into()),
                    source_instance_id: "left".into(),
                }],
                reviewed_fingerprint: Some("reviewed-port-explicit".into()),
                preview: None,
                preview_note: Some("no offer-time projection".into()),
            },
        ])
    }

    #[component]
    fn LinkProjectionPortTestPage() -> Element {
        let request_sequence = use_signal(|| 0_u64);
        let mut last_request = use_signal(String::new);
        let port = crate::mechanical_pair::MechanicalPairProjectionPort(Rc::new({
            move |change: &boardstudio_core::mechanical::change::MechanicalChange| {
                let boardstudio_core::mechanical::change::MechanicalChange::LinkMirrorPair {
                    members,
                    mapping,
                    correspondences,
                    construction_source,
                    ..
                } = change
                else {
                    return Err("not a reviewed link".into());
                };
                let sources = correspondences
                    .iter()
                    .map(|correspondence| {
                        correspondence
                            .source_instance_id
                            .clone()
                            .unwrap_or_else(|| "legacy".into())
                    })
                    .collect::<Vec<_>>()
                    .join(",");
                let _ = members;
                let text = format!(
                    "offset ({:?}, {:?}) rotation {:?} sources [{sources}] construction from {construction_source}",
                    mapping.at.x, mapping.at.y, mapping.rotation
                );
                Ok(crate::mechanical_pair::MechanicalLinkPreview {
                    members: vec![
                        crate::mechanical_pair::MechanicalLinkMemberPreview {
                            instance_id: "left".into(),
                            features: vec![text.clone()],
                            findings: vec![],
                        },
                        crate::mechanical_pair::MechanicalLinkMemberPreview {
                            instance_id: "right".into(),
                            features: vec![text],
                            findings: vec![],
                        },
                    ],
                })
            }
        }));
        rsx! {
            div { id: "case-link-projection-port-test-root",
                MechanicalSettings {
                    identity: test_identity_with_instance("left"),
                    request_sequence,
                    values: Some(test_values(HardwareTransport::Wired, None)),
                    profiles: Rc::from([]),
                    profile_targets: Rc::from([]),
                    process_targets: Rc::from([]),
                    stabilizer_fits: Rc::from([]),
                    layers: Rc::from([]),
                    gasket_supports: Rc::from([]),
                    fit_parts: Rc::from([]),
                    fit_parts_resolved: false,
                    hardware_mounts: Rc::from([]),
                    suggested_mounts: Rc::from([]),
                    findings: Rc::from([]),
                    pair_link_options: Some(port_review_options()),
                    pair_projection_port: Some(port),
                    selected_layer: String::new(),
                    mismatch: None,
                    editable: true,
                    disabled_reason: None,
                    feedback: Rc::from([]),
                    summary_feedback: None,
                    on_request: move |request: MechanicalSettingsRequest| {
                        last_request.set(format!("{:?}", request.patch));
                    },
                    on_select_layer: move |_| {},
                    on_show_finding: move |_| {},
                }
                p { id: "case-link-projection-last-request", "{last_request}" }
            }
        }
    }

    #[wasm_bindgen_test]
    async fn the_review_reprojects_for_every_reviewed_choice_before_confirming() {
        let document = web_sys::window().unwrap().document().unwrap();
        let root = document.create_element("div").unwrap();
        root.set_id("case-link-projection-port-test-host");
        document.body().unwrap().append_child(&root).unwrap();
        dioxus_web::launch::launch_virtual_dom(
            VirtualDom::new(LinkProjectionPortTestPage),
            dioxus_web::Config::new().rootnode(root.clone().into()),
        );
        rendered().await;

        // The displayed result comes from Core through the port, not from the offer.
        let review = element(".m1-mechanical-pair-preview")
            .text_content()
            .unwrap();
        assert!(
            review.contains("offset (0.0, 0.0) rotation 0.0 sources [left]"),
            "the review shows the port's complete result for the derived map, got {review}"
        );

        // A translated frame needs its map; until then there is no result and no commit.
        let select = element("select[aria-label='Link with']");
        js_sys::Reflect::set(&select, &"value".into(), &"1".into()).unwrap();
        let change = web_sys::EventInit::new();
        change.set_bubbles(true);
        select
            .dispatch_event(&web_sys::Event::new_with_event_init_dict("change", &change).unwrap())
            .unwrap();
        rendered().await;
        assert!(
            element(".m1-mechanical-pair-link")
                .text_content()
                .unwrap()
                .contains("complete proposed result is unavailable"),
            "an unmapped translated frame shows no result"
        );
        assert!(
            web_sys::window()
                .unwrap()
                .document()
                .unwrap()
                .query_selector("button[aria-label='Link left with lower']:disabled")
                .unwrap()
                .is_some(),
            "the review cannot be confirmed without a projected result"
        );

        // Entering the map reprojects and enables the commit.
        let input = |selector: &str, value: &str| {
            let input: web_sys::HtmlInputElement = element(selector).dyn_into().unwrap();
            input.set_value(value);
            let event = web_sys::EventInit::new();
            event.set_bubbles(true);
            input
                .dispatch_event(&web_sys::Event::new_with_event_init_dict("input", &event).unwrap())
                .unwrap();
        };
        input("input[aria-label='Pair mapping X']", "30");
        input("input[aria-label='Pair mapping Y']", "-5");
        input("input[aria-label='Pair mapping rotation']", "180");
        rendered().await;
        let review = element(".m1-mechanical-pair-preview")
            .text_content()
            .unwrap();
        assert!(
            review.contains("offset (30.0, -5.0) rotation 180.0"),
            "the entered map reprojects the result, got {review}"
        );
        assert!(
            web_sys::window()
                .unwrap()
                .document()
                .unwrap()
                .query_selector("button[aria-label='Link left with lower']:disabled")
                .unwrap()
                .is_none(),
            "a projected result makes the review committable"
        );

        // Choosing the other half for a feature reprojects the result.
        let source_row = element("select[aria-label='Source for mount-1-left']");
        // The other half of this candidate is "lower", so choosing it is the per-feature
        // source change the review must reproject for.
        js_sys::Reflect::set(&source_row, &"value".into(), &"lower".into()).unwrap();
        let source_change = web_sys::EventInit::new();
        source_change.set_bubbles(true);
        source_row
            .dispatch_event(
                &web_sys::Event::new_with_event_init_dict("change", &source_change).unwrap(),
            )
            .unwrap();
        rendered().await;
        assert!(
            element(".m1-mechanical-pair-preview")
                .text_content()
                .unwrap()
                .contains("sources [lower]"),
            "a per-feature source change reprojects the displayed result, got {}",
            element(".m1-mechanical-pair-preview")
                .text_content()
                .unwrap()
        );

        // Confirming submits exactly the choices the review showed.
        element("button[aria-label='Link left with lower']").click();
        rendered().await;
        let submitted = element("#case-link-projection-last-request")
            .text_content()
            .unwrap();
        assert!(
            submitted.contains("LinkMirrorPair")
                && submitted.contains("x: 30.0")
                && submitted.contains("rotation: 180.0")
                && submitted.contains("source_instance_id: Some(\"lower\")"),
            "the submission carries the reviewed choices, got {submitted}"
        );
        root.remove();
    }

    #[wasm_bindgen_test]
    async fn link_controls_review_a_candidate_and_submit_the_reviewed_link() {
        let document = web_sys::window().unwrap().document().unwrap();
        let root = document.create_element("div").unwrap();
        root.set_id("case-pair-link-test-host");
        document.body().unwrap().append_child(&root).unwrap();
        dioxus_web::launch::launch_virtual_dom(
            VirtualDom::new(PairLinkTestPage),
            dioxus_web::Config::new().rootnode(root.clone().into()),
        );
        rendered().await;

        // The review shows the map the link will use and the durable matches before
        // anything is submitted.
        assert_eq!(
            element(".m1-mechanical-pair-link h3")
                .text_content()
                .unwrap(),
            "Mirror another half"
        );
        let review = element(".m1-mechanical-pair-link").text_content().unwrap();
        assert!(
            review.contains("Derived from the halves' declared transforms"),
            "the review explains the map, got {review}"
        );
        assert!(
            review.contains("Mount mount-1-left · mount-1-right"),
            "the review lists the durable-id matches, got {review}"
        );
        assert!(
            element("#case-pair-link-last-request")
                .text_content()
                .unwrap()
                .is_empty(),
            "the review submits nothing by itself"
        );

        // Both halves of the complete proposed result are rendered before the commit,
        // read from Core rather than computed in the panel.
        let both_halves = element(".m1-mechanical-pair-preview")
            .text_content()
            .unwrap();
        assert!(
            both_halves.contains("left")
                && both_halves.contains("right")
                && both_halves.contains("offset (0.0, 0.0) rotation 0.0"),
            "the review shows both halves of the proposed result, got {both_halves}"
        );

        // Each match chooses its own source half, independently of the construction
        // source and of the other matches.
        let source_row = element("select[aria-label='Source for mount-1-left']");
        assert!(
            source_row.text_content().is_some(),
            "a matched feature offers its own source choice"
        );
        assert!(
            element(".m1-mechanical-pair-source")
                .text_content()
                .unwrap()
                .contains("Mount at (-2.0, 3.0)"),
            "the other half's geometry is shown for the choice"
        );
        js_sys::Reflect::set(&source_row, &"value".into(), &"right".into()).unwrap();
        let source_change = web_sys::EventInit::new();
        source_change.set_bubbles(true);
        source_row
            .dispatch_event(
                &web_sys::Event::new_with_event_init_dict("change", &source_change).unwrap(),
            )
            .unwrap();
        rendered().await;

        element("button[aria-label='Link left with right']").click();
        rendered().await;
        let linked = element("#case-pair-link-last-request")
            .text_content()
            .unwrap();
        assert!(
            linked.contains("LinkMirrorPair")
                && linked.contains("pair/left")
                && linked.contains("mount-1"),
            "the confirmed link submits the reviewed pair, got {linked}"
        );
        assert!(
            linked.contains("rotation: 0.0"),
            "the submitted link carries the reviewed map, got {linked}"
        );
        assert!(
            linked.contains("reviewed=Some(\"reviewed-link\")"),
            "the link submission carries the fingerprint of the candidate the user reviewed, got {linked}"
        );
        assert!(
            linked.contains("source_instance_id: Some(\"right\")"),
            "the link submission carries the source the user chose for that feature, got {linked}"
        );
        assert!(
            linked.contains("source_instance_id: Some(\"right\")")
                && linked.matches("source_instance_id: Some(").count() == 1,
            "each match carries its own source choice, got {linked}"
        );

        // A translated frame has no derived map: the review asks for one explicitly.
        let select = element("select[aria-label='Link with']");
        js_sys::Reflect::set(&select, &"value".into(), &"1".into()).unwrap();
        let change = web_sys::EventInit::new();
        change.set_bubbles(true);
        select
            .dispatch_event(&web_sys::Event::new_with_event_init_dict("change", &change).unwrap())
            .unwrap();
        rendered().await;
        assert!(
            element("input[aria-label='Pair mapping X']").is_connected(),
            "a translated frame gets explicit mapping inputs"
        );
        assert!(
            element(".m1-mechanical-pair-link")
                .text_content()
                .unwrap()
                .contains("complete proposed result is unavailable"),
            "a candidate without a projection says so instead of showing a partial result"
        );
        assert!(
            web_sys::window()
                .unwrap()
                .document()
                .unwrap()
                .query_selector("button[aria-label='Link left with lower']:disabled")
                .unwrap()
                .is_some(),
            "the explicit map is required before the link can be committed"
        );

        let mapping_input = |selector: &str, value: &str| {
            let input: web_sys::HtmlInputElement = element(selector).dyn_into().unwrap();
            input.set_value(value);
            let event = web_sys::EventInit::new();
            event.set_bubbles(true);
            input
                .dispatch_event(&web_sys::Event::new_with_event_init_dict("input", &event).unwrap())
                .unwrap();
        };
        mapping_input("input[aria-label='Pair mapping X']", "30");
        mapping_input("input[aria-label='Pair mapping Y']", "-5");
        mapping_input("input[aria-label='Pair mapping rotation']", "180");
        rendered().await;
        element("button[aria-label='Link left with lower']").click();
        rendered().await;
        let explicit = element("#case-pair-link-last-request")
            .text_content()
            .unwrap();
        assert!(
            explicit.contains("LinkMirrorPair")
                && explicit.contains("lower")
                && explicit.contains("x: 30.0")
                && explicit.contains("y: -5.0")
                && explicit.contains("rotation: 180.0"),
            "the explicit reviewed map is submitted for a translated frame, got {explicit}"
        );
        assert!(
            explicit.contains("reviewed=Some(\"reviewed-link-explicit\")"),
            "the explicit-map candidate's own review fingerprint travels with it, got {explicit}"
        );
        root.remove();
    }
}

#[cfg(test)]
mod mounted_settlement_tests {
    use super::*;
    use crate::mechanical_settings_controller::{
        MechanicalSettingsController, MechanicalSettingsCurrent, MechanicalSettingsPorts,
    };
    use crate::runtime::{Runtime, project_name_test_support as support};
    use boardstudio_application::Event;
    use boardstudio_core::model::{
        GasketConstructionVersion, InternalGasketConfiguration, PartDefinition, ProjectDoc,
    };
    use futures_channel::oneshot;
    use std::{cell::RefCell, collections::VecDeque};
    use wasm_bindgen_test::wasm_bindgen_test;

    #[derive(Clone, Default)]
    struct TestCatalogue(Rc<RefCell<VecDeque<CatalogueReply>>>);

    type CatalogueReply = oneshot::Receiver<Result<(), String>>;

    /// The normalized mounting-hole definition Core validates on every supplied
    /// template: the `ceoloide/mounting_hole_npth` source with a drilled pad.
    fn template() -> Rc<PartDefinition> {
        Rc::new(
            serde_json::from_value(serde_json::json!({
            "id": "generator:ceoloide/mounting_hole_npth", "name": "mounting hole npth",
            "kind": "custom",
            "pads": [{
                "id": "pad-0", "number": "", "at": { "x": 0, "y": 0 },
                "size": { "x": 2.2, "y": 2.2 }, "shape": "circle", "drill": 2.2,
                "plated": false, "rotation": 0
            }],
            "courtyard": [
                { "x": -1.1, "y": -1.1 }, { "x": 1.1, "y": -1.1 },
                { "x": 1.1, "y": 1.1 }, { "x": -1.1, "y": 1.1 }
            ],
            "envelopeSource": { "courtyard": "generated" },
            "envelopeNotice": "No closed courtyard is available; the outline uses physical graphics and pad extents.",
            "generator": { "source": "ceoloide/mounting_hole_npth", "version": "bundled-1", "parameters": {} }
        }))
            .unwrap(),
        )
    }

    fn profile() -> MechanicalPartProfile {
        MechanicalPartProfile {
            definition_id: "part-profile".into(),
            source: "Authored".into(),
            source_geometry: None,
            pcb_holes: None,
            clearance_volumes: None,
            openings: Some(vec![CaseOpening {
                points: vec![
                    Vec2 { x: 0.0, y: 0.0 },
                    Vec2 { x: 5.0, y: 0.0 },
                    Vec2 { x: 5.0, y: 5.0 },
                    Vec2 { x: 0.0, y: 5.0 },
                ],
                z: 0.0,
                height: 10.0,
            }]),
            clearances: None,
            supported_thickness: None,
            switch_family: None,
            cutouts: vec![],
            plate_to_pcb: 3.5,
        }
    }

    fn host() -> Element {
        let runtime = use_context::<Rc<Runtime>>();
        let version = use_signal(|| 0_u64);
        use_hook({
            let runtime = runtime.clone();
            move || {
                runtime.subscribe(Rc::new(move || {
                    let mut version = version;
                    version += 1;
                }))
            }
        });
        let _ = version();
        let feedback = use_signal(Vec::<MechanicalSettingsFeedback>::new);
        let sequence = use_signal(|| 0_u64);
        let mut show_dimension = use_signal(|| true);
        let mut show_case = use_signal(|| true);
        let mut presentation_generation = use_signal(|| 1_u64);
        let catalogue = use_context::<TestCatalogue>();
        let current = {
            let runtime = runtime.clone();
            Rc::new(move || {
                let model = runtime.model();
                let accepted = model.accepted?;
                Some(MechanicalSettingsCurrent {
                    identity: MechanicalSettingsIdentity {
                        editor_instance_id: 1,
                        scope_generation: 1,
                        presentation_generation: presentation_generation(),
                        scope: runtime.scope()?,
                        snapshot_token: accepted.token,
                        revision: accepted.document.revision,
                        active_board_id: "board".into(),
                        configuration_board_id: "board".into(),
                    },
                    configuration: accepted.document.mechanical.clone().map(Rc::new),
                    accepted,
                    editable: show_case(),
                    lifecycle: model.lifecycle,
                    durability: model.durability,
                })
            })
        };
        let controller = use_hook({
            let current = current.clone();
            let runtime = runtime.clone();
            move || {
                MechanicalSettingsController::new(MechanicalSettingsPorts {
                    current,
                    load_mounting_hole: Rc::new(move || {
                        let reply = catalogue.0.borrow_mut().pop_front();
                        Box::pin(async move {
                            if let Some(reply) = reply {
                                reply.await.unwrap()?;
                            }
                            Ok(template())
                        })
                    }),
                    edit_port: Rc::new(runtime),
                    publish: Rc::new(move |entry| {
                        let mut feedback = feedback;
                        feedback.write().push(entry);
                    }),
                })
            }
        });
        use_context_provider(|| controller.clone());
        use_effect(use_reactive(
            (&version(), &presentation_generation(), &show_case()),
            {
                let controller = controller.clone();
                move |_| controller.settle()
            },
        ));
        let current = current().unwrap();
        let configuration = current.configuration.unwrap();
        let dimension_controller = controller.clone();
        let text_controller = controller.clone();
        let profile_controller = controller.clone();
        let profile = configuration.profiles[0].clone();
        rsx! {
            button { class: "test-toggle-dimension", onclick: move |_| show_dimension.toggle(), "Toggle dimension" }
            button { class: "test-toggle-case", onclick: move |_| {
                presentation_generation += 1;
                show_case.toggle();
            }, "Toggle Case" }
            if show_case() {
            if show_dimension() {
            div { class: "test-mechanical-dimension",
                DimensionField {
                    identity: current.identity.clone(), request_sequence: sequence,
                    feedback: Rc::from(feedback().as_slice()),
                    field: MechanicalDimension::WallThickness, label: "Wall thickness",
                    value: configuration.wall_thickness, editable: true,
                    on_request: move |request| { dimension_controller.submit(request); },
                }
            }
            }
            div { class: "test-mechanical-text",
                TextDraftField {
                    identity: current.identity.clone(), request_sequence: sequence,
                    feedback: Rc::from(feedback().as_slice()), label: "Thread",
                    value: configuration.internal_gasket.as_ref().unwrap().hardware.thread.clone(),
                    intent: MechanicalTextIntent::ClosureThread, editable: true,
                    on_request: move |request| { text_controller.submit(request); },
                }
            }
            div { class: "test-mechanical-profile",
                ProfileOpeningListEditor {
                    identity: current.identity, request_sequence: sequence,
                    feedback: Rc::from(feedback().as_slice()),
                    profile: profile.clone(),
                    clearance_volume: false, title: "Access openings",
                    openings: profile.openings.clone().unwrap(), editable: true,
                    on_request: move |request| { profile_controller.submit(request); },
                    on_change: move |_| {},
                }
            }
            }
        }
    }

    async fn mounted_fields(project_id: &str) -> (Rc<Runtime>, web_sys::Element) {
        mounted_fields_with_catalogue(project_id, TestCatalogue::default()).await
    }

    async fn mounted_fields_with_catalogue(
        project_id: &str,
        catalogue: TestCatalogue,
    ) -> (Rc<Runtime>, web_sys::Element) {
        let runtime = support::new_runtime();
        let mut document = ProjectDoc::empty(project_id, "Mechanical field settlement");
        document.boards.push(
            serde_json::from_value(serde_json::json!({
                "id": "board", "name": "Board", "outlineIds": [], "partIds": [],
                "netIds": [], "thickness": 1.6, "traces": [], "vias": []
            }))
            .unwrap(),
        );
        let hardware = InternalClosureHardware {
            id: "custom-m2".into(),
            thread: "M2".into(),
            screw_lengths: vec![8.0, 10.0, 12.0],
            thread_diameter: 2.0,
            pitch: 0.4,
            drive: ScrewDrive::Hex,
            installation: InsertInstallation::HeatSet,
            length_datum: ScrewLengthDatum::UnderHead,
            head_profile: ScrewHeadProfile::Flat,
            fixed_length: None,
            head_diameter: 4.0,
            head_height: 1.0,
            hole_diameter: 2.2,
            insert_diameter: 3.2,
            insert_length: 3.0,
            seat_diameter: 2.8,
            seat_depth: 3.5,
            engagement: 2.5,
            thread_start: 0.2,
            tip_allowance: 0.1,
            bottoming_clearance: 0.5,
            roof: 1.5,
            surround: 1.5,
            seat_lead_depth: 0.25,
            seat_lead_diameter: 3.0,
            bearing_thickness: 1.5,
        };
        let gasket = InternalGasketConfiguration {
            version: GasketConstructionVersion::InternalV1,
            minimum_wall: 2.0,
            support_clearance: None,
            tolerance: 0.05,
            support_count: 4,
            auto_count: Some(true),
            hardware,
        };
        document.mechanical = Some(
            serde_json::from_value(serde_json::json!({
                "boardId": "board", "method": "printed", "mount": "rigid",
                "plateThickness": 1.5, "plateFoamThickness": 0.5, "pcbThickness": 1.6,
                "bottomFoamThickness": 0.4, "batteryHeight": 4.0, "bottomThickness": 3.0,
                "plateToPcb": 3.5, "wallThickness": 2.0, "clearance": 0.3, "profiles": [profile()],
                "closureMounts": [], "internalGasket": gasket,
            }))
            .unwrap(),
        );
        support::open_document(&runtime, document).await;
        let document = web_sys::window().unwrap().document().unwrap();
        let root = document.create_element("div").unwrap();
        document.body().unwrap().append_child(&root).unwrap();
        let dom = VirtualDom::new(host);
        dom.provide_root_context(runtime.clone());
        dom.provide_root_context(catalogue);
        dioxus_web::launch::launch_virtual_dom(
            dom,
            dioxus_web::Config::new().rootnode(root.clone().into()),
        );
        rendered().await;
        (runtime, root)
    }

    async fn rendered() {
        gloo_timers::future::TimeoutFuture::new(40).await;
    }

    async fn settle_fields(runtime: &Rc<Runtime>) {
        for _ in 0..15 {
            support::run_pending(runtime).await;
            rendered().await;
        }
    }

    fn input(root: &web_sys::Element, field: &str) -> HtmlInputElement {
        root.query_selector(&format!(".test-mechanical-{field} input"))
            .unwrap()
            .unwrap()
            .dyn_into()
            .unwrap()
    }

    fn type_value(input: &HtmlInputElement, value: &str) {
        input.focus().unwrap();
        input.set_value(value);
        let event = web_sys::EventInit::new();
        event.set_bubbles(true);
        input
            .dispatch_event(&web_sys::Event::new_with_event_init_dict("input", &event).unwrap())
            .unwrap();
    }

    fn inline_failure(root: &web_sys::Element, field: &str) -> Option<String> {
        root.query_selector(&format!(".test-mechanical-{field} [role='alert']"))
            .unwrap()
            .and_then(|alert| alert.text_content())
    }

    async fn dispose(runtime: &Rc<Runtime>, root: web_sys::Element, project_id: &str) {
        runtime
            .store
            .delete_project(project_id.into())
            .await
            .unwrap();
        runtime.unsubscribe();
        root.remove();
    }

    #[wasm_bindgen_test]
    async fn unchanged_failed_mechanical_fields_restore_accepted_text_and_report_inline() {
        for (field, submitted, accepted) in [("dimension", "3", "2"), ("text", "M3", "M2")] {
            let project_id = format!("mechanical-unchanged-{field}");
            let (runtime, root) = mounted_fields(&project_id).await;
            support::fail_next_core_reply(&runtime, "mechanical executor refused the edit");
            let input = input(&root, field);
            type_value(&input, submitted);
            input.blur().unwrap();
            rendered().await;
            settle_fields(&runtime).await;
            assert_eq!(
                input.value(),
                accepted,
                "an untouched submitted draft restores"
            );
            assert!(
                inline_failure(&root, field)
                    .is_some_and(|message| message.contains("mechanical executor refused"))
            );
            dispose(&runtime, root, &project_id).await;
        }
    }

    #[wasm_bindgen_test]
    async fn older_mechanical_failures_report_beside_newer_drafts_and_original_accepted_text() {
        for (field, submitted, newer) in [
            ("dimension", "3", "4"),
            ("dimension", "3", "2"),
            ("text", "M3", "M4"),
            ("text", "M3", "M2"),
        ] {
            let project_id = format!("mechanical-newer-{field}-{newer}");
            let (runtime, root) = mounted_fields(&project_id).await;
            support::fail_next_core_reply(&runtime, "older mechanical failure");
            let input = input(&root, field);
            type_value(&input, submitted);
            input.blur().unwrap();
            rendered().await;
            type_value(&input, newer);
            settle_fields(&runtime).await;
            assert_eq!(input.value(), newer, "the newer draft stays visible");
            assert!(
                inline_failure(&root, field)
                    .is_some_and(|message| message.contains("older mechanical failure")),
                "the older failure still reports beside the newer draft, including the original accepted value"
            );
            dispose(&runtime, root, &project_id).await;
        }
    }

    #[wasm_bindgen_test]
    async fn a_newer_original_mechanical_draft_survives_an_older_held_landing() {
        for (field, submitted, original) in [("dimension", "3", "2"), ("text", "M3", "M2")] {
            let project_id = format!("mechanical-original-{field}");
            let (runtime, root) = mounted_fields(&project_id).await;
            let (entered, release) = support::gate_next_core_reply(&runtime);
            let input = input(&root, field);
            type_value(&input, submitted);
            input.blur().unwrap();
            rendered().await;
            support::drive_pending(&runtime);
            entered.await.unwrap();
            type_value(&input, original);
            release.send(()).unwrap();
            settle_fields(&runtime).await;
            assert_eq!(
                input.value(),
                original,
                "returning to the original accepted text remains a newer draft"
            );
            assert!(inline_failure(&root, field).is_none());
            dispose(&runtime, root, &project_id).await;
        }
    }

    #[wasm_bindgen_test]
    async fn mounted_mechanical_latest_commit_keeps_both_edits_and_undo_steps() {
        let project_id = "mechanical-mounted-latest";
        let (runtime, root) = mounted_fields(project_id).await;
        let (entered, release) = support::gate_next_core_reply(&runtime);
        let input = input(&root, "dimension");
        type_value(&input, "3");
        input.blur().unwrap();
        rendered().await;
        support::drive_pending(&runtime);
        entered.await.unwrap();
        type_value(&input, "4");
        input.blur().unwrap();
        rendered().await;
        support::drive_pending(&runtime);
        release.send(()).unwrap();
        settle_fields(&runtime).await;
        assert_eq!(
            runtime
                .model()
                .accepted
                .unwrap()
                .document
                .mechanical
                .as_ref()
                .unwrap()
                .wall_thickness,
            4.0
        );
        assert_eq!(input.value(), "4");
        for (accepted, expected_text) in [(3.0, "3"), (2.0, "2")] {
            runtime.submit(Event::Undo {
                operation_id: runtime.operation(),
            });
            settle_fields(&runtime).await;
            assert_eq!(
                runtime
                    .model()
                    .accepted
                    .unwrap()
                    .document
                    .mechanical
                    .as_ref()
                    .unwrap()
                    .wall_thickness,
                accepted
            );
            assert_eq!(input.value(), expected_text);
        }
        dispose(&runtime, root, project_id).await;
    }
    #[wasm_bindgen_test]
    async fn catalogue_failure_restores_only_unchanged_drafts_and_reports_beside_newer_text() {
        for (field, submitted, newer, expected) in [
            ("dimension", "3", None, "2"),
            ("dimension", "3", Some("4"), "4"),
            ("dimension", "3", Some("2"), "2"),
            ("text", "M3", None, "M2"),
            ("text", "M3", Some("M4"), "M4"),
            ("text", "M3", Some("M2"), "M2"),
        ] {
            let (release, reply) = oneshot::channel();
            let catalogue = TestCatalogue::default();
            catalogue.0.borrow_mut().push_back(reply);
            let project_id = format!("mechanical-catalogue-{field}-{expected}");
            let (runtime, root) = mounted_fields_with_catalogue(&project_id, catalogue).await;
            let input = input(&root, field);
            type_value(&input, submitted);
            input.blur().unwrap();
            rendered().await;
            if let Some(newer) = newer {
                type_value(&input, newer);
            }
            release.send(Err("catalogue unavailable".into())).unwrap();
            settle_fields(&runtime).await;
            assert_eq!(input.value(), expected);
            assert!(
                inline_failure(&root, field)
                    .is_some_and(|message| message.contains("catalogue unavailable"))
            );
            assert_eq!(
                runtime
                    .model()
                    .accepted
                    .unwrap()
                    .document
                    .mechanical
                    .as_ref()
                    .unwrap()
                    .wall_thickness,
                2.0
            );
            dispose(&runtime, root, &project_id).await;
        }
    }

    #[wasm_bindgen_test]
    async fn preparation_from_an_unmounted_field_cannot_bind_a_replacement_field() {
        let (release, reply) = oneshot::channel();
        let catalogue = TestCatalogue::default();
        catalogue.0.borrow_mut().push_back(reply);
        let project_id = "mechanical-preparation-remount";
        let (runtime, root) = mounted_fields_with_catalogue(project_id, catalogue).await;
        support::fail_next_core_reply(&runtime, "old preparation failed");
        let old_input = input(&root, "dimension");
        type_value(&old_input, "3");
        old_input.blur().unwrap();
        rendered().await;
        let toggle = root
            .query_selector(".test-toggle-dimension")
            .unwrap()
            .unwrap()
            .dyn_into::<web_sys::HtmlElement>()
            .unwrap();
        toggle.click();
        rendered().await;
        assert!(
            root.query_selector(".test-mechanical-dimension")
                .unwrap()
                .is_none()
        );
        toggle.click();
        rendered().await;
        let replacement = input(&root, "dimension");
        type_value(&replacement, "3");
        release.send(Ok(())).unwrap();
        settle_fields(&runtime).await;
        assert_eq!(
            replacement.value(),
            "3",
            "the old submission must not restore the replacement's identical draft"
        );
        assert!(
            inline_failure(&root, "dimension").is_none(),
            "the old outcome must not reach the replacement binding"
        );
        assert_eq!(
            runtime
                .model()
                .accepted
                .unwrap()
                .document
                .mechanical
                .as_ref()
                .unwrap()
                .wall_thickness,
            2.0
        );
        dispose(&runtime, root, project_id).await;
    }

    fn profile_input(root: &web_sys::Element, dimension: &str) -> HtmlInputElement {
        root.query_selector(&format!(
            ".test-mechanical-profile input[aria-label='Access openings volume 1 {dimension}']"
        ))
        .unwrap()
        .unwrap()
        .dyn_into()
        .unwrap()
    }

    fn failure_at(input: &HtmlInputElement) -> Option<String> {
        input
            .closest("label")
            .unwrap()
            .unwrap()
            .query_selector("[role='alert']")
            .unwrap()
            .and_then(|alert| alert.text_content())
    }

    #[wasm_bindgen_test]
    async fn profile_volume_siblings_bind_distinct_fields_for_the_same_domain_request() {
        let project_id = "mechanical-profile-siblings";
        let (runtime, root) = mounted_fields(project_id).await;
        support::fail_next_core_reply(&runtime, "profile volume failed");
        let bottom = profile_input(&root, "Bottom Z");
        let height = profile_input(&root, "Height");
        type_value(&bottom, "2");
        bottom.blur().unwrap();
        rendered().await;
        type_value(&height, "15");
        settle_fields(&runtime).await;
        assert_eq!(bottom.value(), "0");
        assert!(
            failure_at(&bottom).is_some_and(|message| message.contains("profile volume failed"))
        );
        assert_eq!(
            height.value(),
            "15",
            "an untouched sibling's newer draft stays visible"
        );
        assert!(
            failure_at(&height).is_none(),
            "failure belongs to the submitted profile dimension"
        );
        dispose(&runtime, root, project_id).await;
    }

    #[wasm_bindgen_test]
    async fn catalogue_latest_preparation_keeps_identical_newer_text_while_pending() {
        let (release_a, reply_a) = oneshot::channel();
        let (release_b, reply_b) = oneshot::channel();
        let catalogue = TestCatalogue::default();
        catalogue.0.borrow_mut().extend([reply_a, reply_b]);
        let project_id = "mechanical-catalogue-latest-identical";
        let (runtime, root) = mounted_fields_with_catalogue(project_id, catalogue).await;
        let input = input(&root, "dimension");
        for _ in 0..2 {
            type_value(&input, "3");
            input.blur().unwrap();
            rendered().await;
        }
        release_a
            .send(Err("older catalogue failure".into()))
            .unwrap();
        settle_fields(&runtime).await;
        assert_eq!(
            input.value(),
            "3",
            "the newer identical submitted text remains pending"
        );
        assert!(
            inline_failure(&root, "dimension").is_none(),
            "older preparation must not report through the latest field"
        );
        release_b
            .send(Err("newer catalogue failure".into()))
            .unwrap();
        settle_fields(&runtime).await;
        assert_eq!(input.value(), "2");
        assert!(
            inline_failure(&root, "dimension")
                .is_some_and(|message| message.contains("newer catalogue failure"))
        );
        dispose(&runtime, root, project_id).await;
    }

    #[wasm_bindgen_test]
    async fn catalogue_latest_preparation_error_survives_older_failure_arriving_last() {
        let (release_a, reply_a) = oneshot::channel();
        let (release_b, reply_b) = oneshot::channel();
        let catalogue = TestCatalogue::default();
        catalogue.0.borrow_mut().extend([reply_a, reply_b]);
        let project_id = "mechanical-catalogue-latest-error";
        let (runtime, root) = mounted_fields_with_catalogue(project_id, catalogue).await;
        let input = input(&root, "dimension");
        for _ in 0..2 {
            type_value(&input, "3");
            input.blur().unwrap();
            rendered().await;
        }
        release_b
            .send(Err("newer catalogue failure".into()))
            .unwrap();
        settle_fields(&runtime).await;
        assert_eq!(input.value(), "2");
        assert!(
            inline_failure(&root, "dimension")
                .is_some_and(|message| message.contains("newer catalogue failure"))
        );
        release_a
            .send(Err("older catalogue failure".into()))
            .unwrap();
        settle_fields(&runtime).await;
        assert_eq!(input.value(), "2");
        assert!(
            inline_failure(&root, "dimension")
                .is_some_and(|message| message.contains("newer catalogue failure")),
            "the bounded latest admission still owns feedback after its preparation failed"
        );
        dispose(&runtime, root, project_id).await;
    }

    #[wasm_bindgen_test]
    async fn presentation_departure_retires_field_observation_without_canceling_session_edit() {
        let project_id = "mechanical-presentation-return";
        let (runtime, root) = mounted_fields(project_id).await;
        let (entered, release) = support::gate_next_core_reply(&runtime);
        let first_input = input(&root, "dimension");
        type_value(&first_input, "3");
        first_input.blur().unwrap();
        rendered().await;
        support::drive_pending(&runtime);
        entered.await.unwrap();
        let toggle = root
            .query_selector(".test-toggle-case")
            .unwrap()
            .unwrap()
            .dyn_into::<web_sys::HtmlElement>()
            .unwrap();
        toggle.click();
        rendered().await;
        assert!(
            root.query_selector(".test-mechanical-dimension")
                .unwrap()
                .is_none()
        );
        toggle.click();
        rendered().await;
        let returned = input(&root, "dimension");
        assert_eq!(returned.value(), "2");
        assert!(
            root.query_selector(".test-mechanical-dimension [role='status']")
                .unwrap()
                .is_none(),
            "returning to Case must not revive the old presentation's pending field observation"
        );
        release.send(()).unwrap();
        settle_fields(&runtime).await;
        assert_eq!(
            runtime
                .model()
                .accepted
                .unwrap()
                .document
                .mechanical
                .as_ref()
                .unwrap()
                .wall_thickness,
            3.0,
            "the already submitted Session edit still runs"
        );
        assert_eq!(returned.value(), "3");
        assert!(inline_failure(&root, "dimension").is_none());
        dispose(&runtime, root, project_id).await;
    }
    #[wasm_bindgen_test]
    async fn newer_catalogue_failure_and_draft_survive_an_older_actual_ticket_landing() {
        for newer in ["5", "2"] {
            let (ready, reply_a) = oneshot::channel();
            let (release_b, reply_b) = oneshot::channel();
            ready.send(Ok(())).unwrap();
            let catalogue = TestCatalogue::default();
            catalogue.0.borrow_mut().extend([reply_a, reply_b]);
            let project_id = format!("mechanical-mixed-phase-{newer}");
            let (runtime, root) = mounted_fields_with_catalogue(&project_id, catalogue).await;
            let (entered, release_a) = support::gate_next_core_reply(&runtime);
            let input = input(&root, "dimension");
            type_value(&input, "3");
            input.blur().unwrap();
            rendered().await;
            support::drive_pending(&runtime);
            entered.await.unwrap();
            type_value(&input, "4");
            input.blur().unwrap();
            rendered().await;
            type_value(&input, newer);
            release_b
                .send(Err("newer preparation failed".into()))
                .unwrap();
            settle_fields(&runtime).await;
            assert_eq!(input.value(), newer);
            assert!(
                inline_failure(&root, "dimension")
                    .is_some_and(|message| message.contains("newer preparation failed"))
            );
            assert!(
                root.query_selector(".test-mechanical-dimension [role='status']")
                    .unwrap()
                    .is_none(),
                "only the latest admitted request drives field pending"
            );
            release_a.send(()).unwrap();
            settle_fields(&runtime).await;
            assert_eq!(
                runtime
                    .model()
                    .accepted
                    .unwrap()
                    .document
                    .mechanical
                    .as_ref()
                    .unwrap()
                    .wall_thickness,
                3.0
            );
            assert_eq!(
                input.value(),
                newer,
                "the newer draft survives, including the original accepted value"
            );
            assert!(
                inline_failure(&root, "dimension")
                    .is_some_and(|message| message.contains("newer preparation failed")),
                "an older actual ticket cannot clear the newer preparation failure"
            );
            dispose(&runtime, root, &project_id).await;
        }
    }
}
