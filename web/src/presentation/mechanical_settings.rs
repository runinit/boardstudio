//! Private read-only presentation and field intents for the Case mechanical stack.
//! Root owns accepted-state admission and commits every intent through Runtime.
pub(crate) use crate::mechanical_feedback::{
    MechanicalSettingsFeedback, MechanicalSettingsFeedbackState, MechanicalSettingsIdentity,
};
use boardstudio_core::model::{
    CaseOpening, GasketPlacement, HardwareTransport, InsertInstallation, InternalClosureHardware,
    MechanicalBattery, MechanicalBottomStyle, MechanicalCriticalFit, MechanicalGasketAnchor,
    MechanicalMount, MechanicalSwitchFamily, Mount, MountKind, PlateMethod, ScrewDrive,
    ScrewHeadProfile, ScrewLengthDatum, Severity, Vec2,
};
use dioxus::prelude::*;
use dioxus_web::WebEventExt;
use std::rc::Rc;
use wasm_bindgen::JsCast;
use web_sys::HtmlInputElement;

/// Narrow accepted values used by this control group; it is not an editable
/// configuration copy and deliberately excludes every field this ticket leaves alone.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct MechanicalSettingsValues {
    pub(crate) board_id: String,
    pub(crate) transport: HardwareTransport,
    pub(crate) battery: Option<MechanicalBattery>,
    pub(crate) suspension_mounts: Vec<Mount>,
    pub(crate) closure_mounts: Option<Vec<Mount>>,
    pub(crate) method: PlateMethod,
    pub(crate) mount: MechanicalMount,
    pub(crate) bottom_style: MechanicalBottomStyle,
    pub(crate) middle_frame: bool,
    pub(crate) integrated_plate_frame: bool,
    pub(crate) plate_thickness: f64,
    pub(crate) plate_foam_thickness: f64,
    pub(crate) pcb_thickness: f64,
    pub(crate) bottom_foam_thickness: f64,
    pub(crate) bottom_thickness: f64,
    pub(crate) wall_thickness: f64,
    pub(crate) clearance: f64,
    pub(crate) opening_allowance: f64,
    pub(crate) openings: Vec<CaseOpening>,
    pub(crate) internal_gasket: bool,
    pub(crate) closure_hardware: Option<InternalClosureHardware>,
    pub(crate) critical_fits: Vec<MechanicalCriticalFit>,
    pub(crate) plate_to_pcb: f64,
    pub(crate) battery_height: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct MechanicalProfileChoice {
    pub(crate) definition_id: String,
    pub(crate) name: String,
    pub(crate) source: Option<String>,
    pub(crate) family: Option<MechanicalSwitchFamily>,
    pub(crate) plate_to_pcb: Option<f64>,
    pub(crate) supported_thickness: Option<Vec2>,
    /// True only for a placed switch definition selected by the accepted parent projection.
    pub(crate) switch_family_selectable: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct MechanicalLayerRow {
    pub(crate) id: String,
    pub(crate) label: String,
    pub(crate) z: f64,
    pub(crate) thickness: f64,
    /// Present only when the displayed completed Case assembly contains a body with this ID.
    pub(crate) resolved_body_thickness: Option<f64>,
    /// The row is retained from the last completed same-owner assembly while
    /// the accepted physical inputs have changed.
    pub(crate) is_previous: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct MechanicalGasketSupportRow {
    pub(crate) id: String,
    pub(crate) region_id: String,
    pub(crate) outline_key: String,
    pub(crate) anchor: f64,
    pub(crate) pair_id: Option<String>,
    pub(crate) length: f64,
    pub(crate) width: f64,
    pub(crate) unlinked: bool,
    pub(crate) fit_error: Option<String>,
    pub(crate) is_previous: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MechanicalMountCollection {
    Suspension,
    Closure,
}

impl MechanicalGasketSupportRow {
    pub(crate) fn saved_anchor(&self) -> MechanicalGasketAnchor {
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
pub(crate) struct MechanicalFindingRow {
    pub(crate) id: String,
    pub(crate) severity: Severity,
    pub(crate) message: String,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct MechanicalBoardMismatch {
    pub(crate) board_id: String,
    pub(crate) board_name: String,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct MechanicalFitPart {
    pub(crate) id: String,
    pub(crate) name: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MechanicalDimension {
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
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MechanicalCriticalFitTextField {
    Label,
    Tolerance,
}

#[derive(Clone, Debug, PartialEq)]
enum MechanicalTextIntent {
    ClosureThread,
    ClosureScrewLengths,
    CriticalFit {
        fit_id: String,
        field: MechanicalCriticalFitTextField,
    },
}

impl MechanicalTextIntent {
    fn field_id(&self) -> String {
        match self {
            Self::ClosureThread => "closure-thread".to_owned(),
            Self::ClosureScrewLengths => "closure-screw-lengths".to_owned(),
            Self::CriticalFit { fit_id, field } => format!(
                "critical-fit:{fit_id}:{}",
                match field {
                    MechanicalCriticalFitTextField::Label => "label",
                    MechanicalCriticalFitTextField::Tolerance => "tolerance",
                }
            ),
        }
    }

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
    if field_id.starts_with("closure-") || field_id.starts_with("critical-fit:") {
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

impl MechanicalDimension {
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
            Self::GasketSupportLength => NumberRule::AtLeastFive,
            Self::GasketSupportWidth => NumberRule::AtLeastHalf,
            _ => NumberRule::Nonnegative,
        }
    }

    fn step(self) -> &'static str {
        match self {
            Self::GasketSupportLength => "5",
            Self::GasketSupportWidth => "0.5",
            _ => "0.1",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum MechanicalSettingsPatch {
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
    SetOpenings(Vec<CaseOpening>),
    SetOpeningDimension {
        opening_index: usize,
        point_index: Option<usize>,
        field: MechanicalDimension,
        value: f64,
    },
    SetMethod(PlateMethod),
    SetMount(MechanicalMount),
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
    SetSwitchFamily {
        definition_id: String,
        family: MechanicalSwitchFamily,
    },
}

impl MechanicalSettingsPatch {
    pub(crate) fn field_id(&self) -> String {
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
            Self::SetOpenings(_) => "case-openings".to_owned(),
            Self::SetOpeningDimension {
                opening_index,
                point_index,
                field,
                ..
            } => format!(
                "opening:{opening_index}:{}:{}",
                point_index.map_or_else(|| "volume".to_owned(), |index| format!("point:{index}")),
                field.field_id(),
            ),
            Self::SetMethod(_) => "method".to_owned(),
            Self::SetMount(_) => "mount".to_owned(),
            Self::SetMountCollection { collection, .. } => match collection {
                MechanicalMountCollection::Suspension => "suspension-mounts".to_owned(),
                MechanicalMountCollection::Closure => "closure-mounts".to_owned(),
            },
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
            Self::SetSwitchFamily { definition_id, .. } => {
                format!("switch-family:{definition_id}")
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
pub(crate) struct MechanicalSettingsRequest {
    pub(crate) identity: MechanicalSettingsIdentity,
    pub(crate) request_id: u64,
    pub(crate) field_id: String,
    pub(crate) patch: MechanicalSettingsPatch,
}

#[derive(Props, Clone, PartialEq)]
pub(crate) struct MechanicalSettingsProps {
    pub(crate) identity: MechanicalSettingsIdentity,
    /// Editor-lifetime sequence shared with the page owner, so an unmount/remount cannot reuse
    /// request IDs while retaining the same editor identity.
    pub(crate) request_sequence: Signal<u64>,
    pub(crate) values: Option<MechanicalSettingsValues>,
    pub(crate) profiles: Rc<[MechanicalProfileChoice]>,
    pub(crate) layers: Rc<[MechanicalLayerRow]>,
    pub(crate) gasket_supports: Rc<[MechanicalGasketSupportRow]>,
    pub(crate) fit_parts: Rc<[MechanicalFitPart]>,
    pub(crate) fit_parts_resolved: bool,
    pub(crate) suggested_mounts: Rc<[Mount]>,
    pub(crate) findings: Rc<[MechanicalFindingRow]>,
    pub(crate) selected_layer: String,
    pub(crate) mismatch: Option<MechanicalBoardMismatch>,
    pub(crate) editable: bool,
    pub(crate) disabled_reason: Option<String>,
    /// Bounded per-request feedback keeps a rejected raced submit from replacing the
    /// currently admitted operation's Pending/Saved response.
    pub(crate) feedback: Rc<[MechanicalSettingsFeedback]>,
    pub(crate) summary_feedback: Option<MechanicalSettingsFeedback>,
    pub(crate) on_request: EventHandler<MechanicalSettingsRequest>,
    pub(crate) on_select_layer: EventHandler<String>,
    pub(crate) on_show_finding: EventHandler<String>,
}

#[component]
pub(crate) fn MechanicalSettings(props: MechanicalSettingsProps) -> Element {
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
                    if feedback.state == MechanicalSettingsFeedbackState::Pending {
                        p { role: "status", "Saving mechanical settings…" }
                    } else if feedback.state == MechanicalSettingsFeedbackState::Saved {
                        p { role: "status", "Mechanical settings saved." }
                    } else if !is_dimension_field(&feedback.field_id)
                        && let Some(message) = feedback.message.as_deref()
                    {
                        p { role: "alert", "{message}" }
                        if feedback.field_id == "initialize-closures" {
                            button {
                                r#type: "button",
                                disabled: !props.editable,
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
                ProfileGuidance {
                    identity: props.identity.clone(),
                    profiles: props.profiles.clone(),
                    plate_thickness: values.plate_thickness,
                    editable: props.editable,
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
                OpeningControls {
                    key: "{owner_key}:case-openings",
                    identity: props.identity.clone(),
                    request_sequence,
                    values: values.clone(),
                    editable: props.editable,
                    feedback: props.feedback.clone(),
                    on_request: props.on_request,
                    owner_key: owner_key.clone(),
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
                            disabled: !props.editable,
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
                CriticalFitControls {
                    key: "{owner_key}:critical-fits",
                    identity: props.identity.clone(),
                    request_sequence,
                    fits: values.critical_fits.clone(),
                    fit_parts: props.fit_parts.clone(),
                    fit_parts_resolved: props.fit_parts_resolved,
                    values: values.clone(),
                    editable: props.editable,
                    feedback: props.feedback.clone(),
                    on_request: props.on_request,
                    owner_key: owner_key.clone(),
                }
                button {
                    r#type: "button",
                    class: "m1-mechanical-disable",
                    disabled: !props.editable,
                    onclick: {
                        let identity = props.identity.clone();
                        let mut sequence = request_sequence;
                        let on_request = props.on_request;
                        move |_| send_request(&mut sequence, &identity, on_request, MechanicalSettingsPatch::Disable)
                    },
                    "Disable mechanical stack"
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
        details { class: "m1-mechanical-group", open: !fits.is_empty(), aria_label: "Hardware and critical fits",
            summary { "Hardware & critical fits · {fits.len()} fits" }
            div { class: "m1-mechanical-spec-group",
                header {
                    strong { "Critical fits" }
                    button {
                        r#type: "button",
                        class: "m1-mechanical-quiet",
                        disabled: !props.editable,
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
                                disabled: !props.editable,
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
                        disabled: !props.editable,
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
                        disabled: !props.editable,
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
                        disabled: !props.editable,
                        onclick: {
                            let mut sequence = props.request_sequence;
                            let identity = props.identity.clone();
                            let callback = props.on_request;
                            let mut mounts = closure_mounts.to_vec();
                            let height = values.plate_to_pcb + values.pcb_thickness + values.bottom_foam_thickness.max(values.battery_height);
                            let additions = suggestions.iter().cloned().map(|mut mount| {
                                mount.id = format!("auto-closure/{}", mount.id);
                                mount.kind = MountKind::Boss;
                                mount.hole_diameter = 2.2;
                                mount.height = Some(height);
                                mount
                            }).collect::<Vec<_>>();
                            mounts.extend(additions);
                            move |_| send_request(&mut sequence, &identity, callback, MechanicalSettingsPatch::SetMountCollection { collection: MechanicalMountCollection::Closure, mounts: mounts.clone() })
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
    let reserved_mount_ids = props.reserved_mount_ids.clone();
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
                    disabled: !props.editable,
                    onclick: {
                        let mut sequence = props.request_sequence;
                        let identity = props.identity.clone();
                        let callback = props.on_request;
                        let collection = props.collection;
                        let mut next_mounts = mounts.clone();
                        let all = reserved_mount_ids.iter().cloned().collect::<std::collections::HashSet<_>>();
                        let mut ordinal = 1usize;
                        let id = loop {
                            let candidate = format!("case-mechanical-mount-{ordinal}");
                            if !all.contains(&candidate) { break candidate; }
                            ordinal += 1;
                        };
                        next_mounts.push(Mount {
                            id,
                            at: Vec2 { x: 0.0, y: 0.0 },
                            kind: MountKind::Hole,
                            hole_diameter: 2.5,
                            boss_diameter: Some(5.0),
                            height: Some(5.0),
                        });
                        move |_| send_request(&mut sequence, &identity, callback, MechanicalSettingsPatch::SetMountCollection { collection, mounts: next_mounts.clone() })
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
                disabled: !props.editable,
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
    let add_openings = props.values.openings.clone();
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
                        disabled: !props.editable,
                        onclick: {
                            let mut sequence = request_sequence;
                            let identity = identity.clone();
                            move |_| {
                                let mut next = add_openings.clone();
                                next.push(CaseOpening {
                                    points: vec![
                                        Vec2 { x: -2.5, y: -2.5 },
                                        Vec2 { x: 2.5, y: -2.5 },
                                        Vec2 { x: 2.5, y: 2.5 },
                                        Vec2 { x: -2.5, y: 2.5 },
                                    ],
                                    z: 0.0,
                                    height: 10.0,
                                });
                                send_request(&mut sequence, &identity, on_request, MechanicalSettingsPatch::SetOpenings(next));
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
                        let mut next = props.values.openings.clone();
                        next.remove(opening_index);
                        let mut sequence = request_sequence;
                        let identity = identity.clone();
                        let remove_identity = identity.clone();
                        let owner = owner_key.clone();
                        rsx! {
                            fieldset { class: "m1-mechanical-opening", key: "{owner}:opening:{opening_index}", disabled: !props.editable,
                                div { class: "m1-mechanical-opening-title",
                                    strong { "Volume {opening_index + 1}" }
                                    button {
                                        r#type: "button",
                                        class: "m1-mechanical-quiet",
                                        disabled: !props.editable,
                                        onclick: move |_| send_request(&mut sequence, &remove_identity, on_request, MechanicalSettingsPatch::SetOpenings(next.clone())),
                                        "Remove"
                                    }
                                }
                                div { class: "m1-mechanical-opening-fields",
                                    DimensionField {
                                        key: "{owner}:opening:{opening_index}:bottom-z",
                                        identity: identity.clone(), request_sequence, on_request, feedback: props.feedback.clone(),
                                        field: MechanicalDimension::OpeningBottomZ, label: "Bottom Z", value: opening.z, editable: props.editable,
                                        accessible_label: Some(format!("Document-coordinate access opening {} bottom Z", opening_index + 1)),
                                        opening_target: Some(OpeningDimensionTarget { opening_index, point_index: None }),
                                    }
                                    DimensionField {
                                        key: "{owner}:opening:{opening_index}:height",
                                        identity: identity.clone(), request_sequence, on_request, feedback: props.feedback.clone(),
                                        field: MechanicalDimension::OpeningHeight, label: "Height", value: opening.height, editable: props.editable,
                                        accessible_label: Some(format!("Document-coordinate access opening {} height", opening_index + 1)),
                                        opening_target: Some(OpeningDimensionTarget { opening_index, point_index: None }),
                                    }
                                }
                                h4 { "XY footprint" }
                                for (point_index, point) in opening.points.iter().enumerate() {
                                    div { class: "m1-mechanical-opening-vertex",
                                        span { "Vertex {point_index + 1}" }
                                        DimensionField {
                                            key: "{owner}:opening:{opening_index}:point:{point_index}:x",
                                            identity: identity.clone(), request_sequence, on_request, feedback: props.feedback.clone(),
                                            field: MechanicalDimension::OpeningPointX, label: "X", value: point.x, editable: props.editable,
                                            accessible_label: Some(format!("Document-coordinate access opening {} vertex {} X", opening_index + 1, point_index + 1)),
                                            opening_target: Some(OpeningDimensionTarget { opening_index, point_index: Some(point_index) }),
                                        }
                                        DimensionField {
                                            key: "{owner}:opening:{opening_index}:point:{point_index}:y",
                                            identity: identity.clone(), request_sequence, on_request, feedback: props.feedback.clone(),
                                            field: MechanicalDimension::OpeningPointY, label: "Y", value: point.y, editable: props.editable,
                                            accessible_label: Some(format!("Document-coordinate access opening {} vertex {} Y", opening_index + 1, point_index + 1)),
                                            opening_target: Some(OpeningDimensionTarget { opening_index, point_index: Some(point_index) }),
                                        }
                                        button {
                                            r#type: "button",
                                            class: "m1-mechanical-quiet",
                                            aria_label: "Remove access opening vertex {point_index + 1}",
                                            disabled: !props.editable || opening.points.len() <= 3,
                                            onclick: {
                                                let mut sequence = request_sequence;
                                                let identity = identity.clone();
                                                let mut next = props.values.openings.clone();
                                                next[opening_index].points.remove(point_index);
                                                move |_| send_request(&mut sequence, &identity, on_request, MechanicalSettingsPatch::SetOpenings(next.clone()))
                                            },
                                            "×"
                                        }
                                    }
                                }
                                button {
                                    r#type: "button",
                                    class: "m1-mechanical-quiet",
                                    disabled: !props.editable,
                                    onclick: {
                                        let mut sequence = request_sequence;
                                        let identity = identity.clone();
                                        let mut next = props.values.openings.clone();
                                        next[opening_index].points.push(Vec2 { x: 0.0, y: 0.0 });
                                        move |_| send_request(&mut sequence, &identity, on_request, MechanicalSettingsPatch::SetOpenings(next.clone()))
                                    },
                                    "Add vertex"
                                }
                                if opening.points.len() < 3 {
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
            Self::AtLeastFive => "5",
            Self::AtLeastHalf => "0.5",
            Self::Coordinate => "-1000000",
        }
    }

    fn error(self) -> &'static str {
        match self {
            Self::Nonnegative => "Enter a finite value of zero or greater.",
            Self::AtLeastTenth => "Enter a finite value of 0.1 mm or greater.",
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
    profiles: Rc<[MechanicalProfileChoice]>,
    plate_thickness: f64,
    editable: bool,
    request_sequence: Signal<u64>,
    on_request: EventHandler<MechanicalSettingsRequest>,
}

#[component]
fn ProfileGuidance(props: ProfileGuidanceProps) -> Element {
    rsx! {
        section { class: "m1-mechanical-group", aria_label: "Switch fit profiles",
            h3 { "Switch fit profiles" }
            if props.profiles.is_empty() {
                p { class: "m1-mechanical-help", "No switch fit profile is available. Select or add a supported switch family in Parts to resolve the plate gap and supported thickness." }
            }
            for profile in props.profiles.iter() {
                div { class: "m1-mechanical-profile", key: "{profile.definition_id}",
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
                }
            }
            if props.profiles.iter().any(|profile| profile.family.is_none()) {
                p { role: "status", "Choose a supported switch family to resolve the plate gap." }
            }
            p { "Current plate thickness: {props.plate_thickness:.2} mm." }
        }
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
    accessible_label: Option<String>,
    #[props(default)]
    critical_fit_target: Option<CriticalFitDimensionTarget>,
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
    opening_index: usize,
    point_index: Option<usize>,
}

#[derive(Clone, Debug, PartialEq)]
struct CriticalFitDimensionTarget {
    fit_id: String,
}

#[component]
fn DimensionField(props: DimensionFieldProps) -> Element {
    let mut draft = use_signal(|| props.value.to_string());
    let mut dirty = use_signal(|| false);
    let mut error = use_signal(|| None::<String>);
    let mut submitted = use_signal(|| None::<MechanicalSettingsRequest>);
    let mut field_status = use_signal(|| None::<String>);
    let mut previous_accepted = use_signal(|| props.value);

    let saved_value = props.value;
    let feedback_entries = props.feedback.clone();
    let mut draft_for_ack = draft;
    let mut dirty_for_ack = dirty;
    let mut error_for_ack = error;
    let mut submitted_for_ack = submitted;
    let mut status_for_ack = field_status;
    let submitted_copy = submitted;
    use_effect(use_reactive(
        (&feedback_entries, &saved_value),
        move |(feedback_entries, accepted)| {
            let accepted_changed = *previous_accepted.read() != accepted;
            if accepted_changed {
                previous_accepted.set(accepted);
            }
            let Some(request) = submitted_copy.read().clone() else {
                if accepted_changed {
                    draft_for_ack.set(accepted.to_string());
                    dirty_for_ack.set(false);
                    error_for_ack.set(None);
                    status_for_ack.set(None);
                }
                return;
            };
            let feedback = feedback_entries.iter().find(|feedback| {
                feedback.identity == request.identity
                    && feedback.request_id == request.request_id
                    && feedback.field_id == request.field_id
            });
            if let Some(feedback) = feedback {
                match feedback.state {
                    MechanicalSettingsFeedbackState::Pending => {
                        // The accepted token can advance before persistence settles. Keep this
                        // field's submitted draft until its exact request receives Saved or Failed.
                        status_for_ack.set(Some("Saving…".to_owned()));
                        return;
                    }
                    MechanicalSettingsFeedbackState::Saved => {
                        draft_for_ack.set(accepted.to_string());
                        dirty_for_ack.set(false);
                        error_for_ack.set(None);
                        status_for_ack.set(Some("Saved".to_owned()));
                        submitted_for_ack.set(None);
                        return;
                    }
                    MechanicalSettingsFeedbackState::Failed => {
                        error_for_ack.set(Some(feedback.message.clone().unwrap_or_else(|| {
                            "This setting was not saved. Review the value and retry.".to_owned()
                        })));
                        status_for_ack.set(None);
                        submitted_for_ack.set(None);
                        return;
                    }
                }
            }
            if accepted_changed {
                draft_for_ack.set(accepted.to_string());
                dirty_for_ack.set(false);
                error_for_ack.set(None);
                status_for_ack.set(None);
                submitted_for_ack.set(None);
            }
        },
    ));

    let commit: Rc<dyn Fn()> = Rc::new({
        let identity = props.identity.clone();
        let field = props.field;
        let support_target = props.support_target.clone();
        let mount_target = props.mount_target.clone();
        let opening_target = props.opening_target.clone();
        let critical_fit_target = props.critical_fit_target.clone();
        let accepted = props.value;
        let sequence = props.request_sequence;
        let on_request = props.on_request;
        move || {
            let mut sequence = sequence;
            let mut error = error;
            let mut field_status = field_status;
            let mut submitted = submitted;
            let mut dirty = dirty;
            let mut draft = draft;
            let text = draft();
            if submitted().is_some() {
                return;
            }
            let Ok(value) = text.trim().parse::<f64>() else {
                error.set(Some(field.rule().error().to_owned()));
                return;
            };
            if !field.rule().valid(value) {
                error.set(Some(field.rule().error().to_owned()));
                return;
            }
            error.set(None);
            if value == accepted {
                draft.set(accepted.to_string());
                dirty.set(false);
                field_status.set(None);
                submitted.set(None);
                return;
            }
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
            } else if let Some(target) = opening_target.clone() {
                MechanicalSettingsPatch::SetOpeningDimension {
                    opening_index: target.opening_index,
                    point_index: target.point_index,
                    field,
                    value,
                }
            } else if let Some(target) = critical_fit_target.clone() {
                MechanicalSettingsPatch::SetCriticalFitPoint {
                    fit_id: target.fit_id,
                    field,
                    value,
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
            };
            submitted.set(Some(request.clone()));
            field_status.set(Some("Saving…".to_owned()));
            on_request.call(request);
        }
    });
    let error_text = error();
    let status_text = field_status();
    rsx! {
        label { class: "m1-mechanical-dimension",
            span { "{props.label}" }
            span { class: "m1-mechanical-number",
                input {
                    r#type: "number",
                    step: props.field.step(),
                    min: props.field.rule().minimum(),
                    max: if props.field == MechanicalDimension::OpeningAllowance { "1" },
                    aria_label: props.accessible_label.as_deref().unwrap_or(props.label),
                    value: "{draft}",
                    disabled: !props.editable,
                    "aria-invalid": error_text.is_some(),
                    oninput: move |event: FormEvent| {
                        draft.set(event.value());
                        dirty.set(true);
                        error.set(None);
                        field_status.set(None);
                        submitted.set(None);
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
                                submitted.set(None);
                                field_status.set(None);
                            }
                        }
                    }
                }
                small { "mm" }
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
}

#[component]
fn TextDraftField(props: TextDraftFieldProps) -> Element {
    let mut draft = use_signal(|| props.value.clone());
    let mut error = use_signal(|| None::<String>);
    let mut submitted = use_signal(|| None::<MechanicalSettingsRequest>);
    let mut status = use_signal(|| None::<String>);
    let mut previous_accepted = use_signal(|| props.value.clone());
    let accepted = props.value.clone();
    let feedback_entries = props.feedback.clone();
    let mut draft_for_ack = draft;
    let mut error_for_ack = error;
    let mut submitted_for_ack = submitted;
    let mut status_for_ack = status;
    let submitted_copy = submitted;
    use_effect(use_reactive(
        (&feedback_entries, &accepted),
        move |(feedback_entries, accepted)| {
            let accepted_changed = *previous_accepted.read() != accepted;
            if accepted_changed {
                previous_accepted.set(accepted.clone());
            }
            let Some(request) = submitted_copy.read().clone() else {
                if accepted_changed {
                    draft_for_ack.set(accepted);
                    error_for_ack.set(None);
                    status_for_ack.set(None);
                }
                return;
            };
            let feedback = feedback_entries.iter().find(|feedback| {
                feedback.identity == request.identity
                    && feedback.request_id == request.request_id
                    && feedback.field_id == request.field_id
            });
            match feedback.map(|feedback| (&feedback.state, &feedback.message)) {
                Some((MechanicalSettingsFeedbackState::Pending, _)) => {
                    status_for_ack.set(Some("Saving…".to_owned()));
                }
                Some((MechanicalSettingsFeedbackState::Saved, _)) => {
                    draft_for_ack.set(accepted);
                    error_for_ack.set(None);
                    status_for_ack.set(Some("Saved".to_owned()));
                    submitted_for_ack.set(None);
                }
                Some((MechanicalSettingsFeedbackState::Failed, message)) => {
                    error_for_ack.set(Some(message.clone().unwrap_or_else(|| {
                        "This setting was not saved. Review the value and retry.".to_owned()
                    })));
                    status_for_ack.set(None);
                    submitted_for_ack.set(None);
                }
                None if accepted_changed => {
                    draft_for_ack.set(accepted);
                    error_for_ack.set(None);
                    status_for_ack.set(None);
                    submitted_for_ack.set(None);
                }
                None => {}
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
            let mut status = status;
            let mut submitted = submitted;
            let mut draft = draft;
            let value = draft();
            if submitted().is_some() {
                return;
            }
            if value == accepted {
                error.set(None);
                status.set(None);
                return;
            }
            let patch = match intent.patch(value) {
                Ok(patch) => patch,
                Err(message) => {
                    error.set(Some(message));
                    status.set(None);
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
            };
            submitted.set(Some(request.clone()));
            status.set(Some("Saving…".to_owned()));
            error.set(None);
            on_request.call(request);
        }
    });
    let error_text = error();
    let status_text = status();
    rsx! {
        label { class: "m1-mechanical-field",
            span { "{props.label}" }
            input {
                r#type: "text",
                class: "m1-mechanical-text",
                value: "{draft}",
                disabled: !props.editable,
                "aria-invalid": error_text.is_some(),
                oninput: move |event: FormEvent| {
                    draft.set(event.value());
                    error.set(None);
                    status.set(None);
                    submitted.set(None);
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
                            error.set(None);
                            submitted.set(None);
                            status.set(None);
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
    on_request.call(MechanicalSettingsRequest {
        identity: identity.clone(),
        request_id,
        field_id: patch.field_id(),
        patch,
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
    use boardstudio_core::model::Vec3;
    use wasm_bindgen::JsCast;
    use wasm_bindgen_test::wasm_bindgen_test;

    wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

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
            critical_fits: vec![],
            plate_to_pcb: 3.5,
            battery_height: 0.0,
        }
    }

    fn test_page() -> Element {
        test_page_with_previous(false)
    }

    fn previous_test_page() -> Element {
        test_page_with_previous(true)
    }

    fn test_page_with_previous(is_previous: bool) -> Element {
        let mut selected_layer = use_signal(|| "plate".to_owned());
        let mut shown_finding = use_signal(String::new);
        let request_sequence = use_signal(|| 0_u64);
        let identity = test_identity();
        let values = test_values(HardwareTransport::Wired, None);
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
                layers,
                gasket_supports: Rc::from([]),
                fit_parts: Rc::from([]),
                fit_parts_resolved: false,
                suggested_mounts: Rc::from([]),
                findings,
                selected_layer: selected_layer(),
                mismatch: None,
                editable: true,
                disabled_reason: None,
                feedback: Rc::from([]),
                summary_feedback: None,
                on_request: move |_| {},
                on_select_layer: move |id| selected_layer.set(id),
                on_show_finding: move |id| shown_finding.set(id),
            }
            div { id: "case-contextual-selected-finding", "{shown_finding}" }
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
                    layers: Rc::from([]),
                    gasket_supports: Rc::from([]),
                    fit_parts: Rc::from([]),
                    fit_parts_resolved: false,
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
                    layers: Rc::from([]),
                    gasket_supports: Rc::from([]),
                    fit_parts: Rc::from([]),
                    fit_parts_resolved: false,
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
            .unwrap()
            .unwrap()
            .dyn_into()
            .unwrap()
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
        stylesheet.set_text_content(Some(include_str!("../../assets/m1.css")));
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
}
