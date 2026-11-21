#![allow(unused_imports)]
//! Private read-only presentation and field intents for the Case mechanical stack.
//! Root owns accepted-state admission and commits every intent through Runtime.
pub use crate::mechanical_feedback::{
    MechanicalSettingsFeedback, MechanicalSettingsFeedbackState, MechanicalSettingsIdentity,
};
use crate::mechanical_pair::{MechanicalPairFeatureRow, MechanicalPairLinkOption, feature_label};
use crate::mechanical_settings_controller::use_bound_mechanical_field;
pub use boardstudio_core_mechanical::mechanical::change::{
    MechanicalChange as MechanicalSettingsPatch, MechanicalCriticalFitTextField,
    MechanicalDimension, MechanicalHardwareTextField, MechanicalMountCollection,
};
use boardstudio_model::model::{
    CaseOpening, GasketPlacement, HardwareTransport, InsertInstallation, InternalClosureHardware,
    MAX_COORDINATE_MM, MechanicalBattery, MechanicalBottomStyle, MechanicalBuiltinProfile,
    MechanicalCriticalFit, MechanicalExtraction, MechanicalFeatureState, MechanicalGasketAnchor,
    MechanicalGeometry, MechanicalHardwareSpecification, MechanicalMount, MechanicalOpening,
    MechanicalPartProcess, MechanicalPartProfile, MechanicalPurpose, MechanicalPurposeMapping,
    MechanicalStabilizerKind, MechanicalStabilizerOverride, MechanicalSwitchFamily, Mount,
    MountKind, PlateMethod, Pose2, ScrewDrive, ScrewHeadProfile, ScrewLengthDatum, Severity, Vec2,
};
use boardstudio_web_ui_shared::ui::{
    alert::{Alert, AlertVariant},
    badge::{Badge, BadgeVariant},
    button::{Button, ButtonSize, ButtonVariant},
    callout::{Callout, CalloutVariant},
    checkbox::{Checkbox, CheckboxState},
    collapsible::{Collapsible, CollapsibleContent, CollapsibleTrigger},
    combobox::{Combobox, ComboboxOption},
    field::{Field, FieldError, FieldLabel},
    input::InputType,
    input_group::{InputGroup, InputGroupInput, InputGroupText, InputGroupTextarea},
    item::{Item, ItemActions, ItemContent, ItemDescription, ItemGroup},
    select::{Select, SelectOption},
    toggle_group::{ToggleGroup, ToggleItem},
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

pub(crate) fn is_dimension_field(field_id: &str) -> bool {
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

pub(crate) trait MechanicalDimensionExt {
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

pub(crate) fn mount_collection_id(collection: MechanicalMountCollection) -> &'static str {
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
    #[props(default)]
    pub preview_values: Option<MechanicalSettingsValues>,
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
pub(crate) struct MechanicalActionsPending(pub(crate) Memo<Vec<String>>);

pub(crate) fn mechanical_action_pending(field_id: &str) -> bool {
    try_consume_context::<MechanicalActionsPending>().is_some_and(|pending| {
        pending
            .0
            .read()
            .iter()
            .any(|id| id == field_id || (field_id.ends_with(':') && id.starts_with(field_id)))
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum NumberRule {
    Nonnegative,
    Bounded(i8, i8),
    AtLeastTenth,
    AtLeastOne,
    AtLeastFive,
    AtLeastHalf,
    Coordinate,
}

impl NumberRule {
    pub(crate) fn minimum(self) -> &'static str {
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

    pub(crate) fn error(self) -> &'static str {
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

    pub(crate) fn valid(self, value: f64) -> bool {
        value.is_finite()
            && match self {
                Self::Nonnegative => value >= 0.0,
                Self::AtLeastTenth => value >= 0.1,
                Self::AtLeastOne => value >= 1.0 && value.fract() == 0.0,
                Self::AtLeastFive => value >= 5.0,
                Self::AtLeastHalf => value >= 0.5,
                Self::Coordinate => value >= -MAX_COORDINATE_MM,
                Self::Bounded(min, max) => value >= f64::from(min) && value <= f64::from(max),
            }
    }
}

pub(crate) fn send_request(
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
pub(crate) fn send_request_with_review(
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

pub(crate) fn method_id(value: &PlateMethod) -> &'static str {
    match value {
        PlateMethod::PcbFr4 => "pcb-fr4",
        PlateMethod::Printed => "printed",
        PlateMethod::Cnc => "cnc",
        PlateMethod::CutSheet => "cut-sheet",
    }
}

pub(crate) fn parse_method(value: &str) -> Option<PlateMethod> {
    Some(match value {
        "pcb-fr4" => PlateMethod::PcbFr4,
        "printed" => PlateMethod::Printed,
        "cnc" => PlateMethod::Cnc,
        "cut-sheet" => PlateMethod::CutSheet,
        _ => return None,
    })
}

pub(crate) fn mount_id(value: &MechanicalMount) -> &'static str {
    match value {
        MechanicalMount::Tray => "tray",
        MechanicalMount::Rigid => "rigid",
        MechanicalMount::Gasket => "gasket",
    }
}

pub(crate) fn parse_mount(value: &str) -> Option<MechanicalMount> {
    Some(match value {
        "tray" => MechanicalMount::Tray,
        "rigid" => MechanicalMount::Rigid,
        "gasket" => MechanicalMount::Gasket,
        _ => return None,
    })
}

pub(crate) fn parse_mount_kind(value: &str) -> Option<MountKind> {
    Some(match value {
        "hole" => MountKind::Hole,
        "boss" => MountKind::Boss,
        _ => return None,
    })
}

pub(crate) fn bottom_style_id(value: &MechanicalBottomStyle) -> &'static str {
    match value {
        MechanicalBottomStyle::Shell => "shell",
        MechanicalBottomStyle::Sheet => "sheet",
    }
}

pub(crate) fn parse_bottom_style(value: &str) -> Option<MechanicalBottomStyle> {
    Some(match value {
        "shell" => MechanicalBottomStyle::Shell,
        "sheet" => MechanicalBottomStyle::Sheet,
        _ => return None,
    })
}

pub(crate) fn family_id(value: MechanicalSwitchFamily) -> &'static str {
    match value {
        MechanicalSwitchFamily::Mx => "mx",
        MechanicalSwitchFamily::ChocV1 => "choc-v1",
        MechanicalSwitchFamily::ChocV2 => "choc-v2",
    }
}

pub(crate) fn family_label(value: MechanicalSwitchFamily) -> &'static str {
    match value {
        MechanicalSwitchFamily::Mx => "MX",
        MechanicalSwitchFamily::ChocV1 => "Choc v1",
        MechanicalSwitchFamily::ChocV2 => "Choc v2",
    }
}

pub(crate) fn parse_family(value: &str) -> Option<MechanicalSwitchFamily> {
    Some(match value {
        "mx" => MechanicalSwitchFamily::Mx,
        "choc-v1" => MechanicalSwitchFamily::ChocV1,
        "choc-v2" => MechanicalSwitchFamily::ChocV2,
        _ => return None,
    })
}

pub(crate) fn severity_label(value: &Severity) -> &'static str {
    match value {
        Severity::Error => "Error",
        Severity::Warning => "Warning",
        Severity::Info => "Info",
    }
}

pub(crate) fn severity_badge(value: &Severity) -> BadgeVariant {
    match value {
        Severity::Error => BadgeVariant::Destructive,
        Severity::Warning => BadgeVariant::Warning,
        Severity::Info => BadgeVariant::Info,
    }
}
