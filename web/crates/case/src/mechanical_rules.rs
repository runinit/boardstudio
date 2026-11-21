//! Pure Case mechanical rules shared by the wasm presentation and native tests.
//!
//! This module owns the dimension validation rules, stable field/request IDs and
//! pure projection helpers so fast native `#[test]` unit tests guard the data
//! mapping while the wasm browser suite keeps only interaction coverage.
use boardstudio_core_mechanical::mechanical::change::{
    MechanicalChange as MechanicalSettingsPatch, MechanicalCriticalFitTextField,
    MechanicalDimension, MechanicalHardwareTextField, MechanicalMountCollection,
};
use boardstudio_model::model::{
    InternalClosureHardware, MAX_COORDINATE_MM, MechanicalBottomStyle, MechanicalMount,
    MechanicalSwitchFamily, PlateMethod, Severity,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NumberRule {
    Nonnegative,
    Bounded(i8, i8),
    AtLeastTenth,
    AtLeastOne,
    AtLeastFive,
    AtLeastHalf,
    Coordinate,
}

impl NumberRule {
    pub fn minimum(self) -> &'static str {
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

    pub fn error(self) -> &'static str {
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

    pub fn valid(self, value: f64) -> bool {
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

pub fn is_dimension_field(field_id: &str) -> bool {
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

pub fn mount_collection_id(collection: MechanicalMountCollection) -> &'static str {
    match collection {
        MechanicalMountCollection::Suspension => "suspension",
        MechanicalMountCollection::Closure => "closure",
    }
}

pub fn method_id(value: &PlateMethod) -> &'static str {
    match value {
        PlateMethod::PcbFr4 => "pcb-fr4",
        PlateMethod::Printed => "printed",
        PlateMethod::Cnc => "cnc",
        PlateMethod::CutSheet => "cut-sheet",
    }
}

pub fn parse_method(value: &str) -> Option<PlateMethod> {
    match value {
        "pcb-fr4" => Some(PlateMethod::PcbFr4),
        "printed" => Some(PlateMethod::Printed),
        "cnc" => Some(PlateMethod::Cnc),
        "cut-sheet" => Some(PlateMethod::CutSheet),
        _ => None,
    }
}

pub fn mount_id(value: &MechanicalMount) -> &'static str {
    match value {
        MechanicalMount::Tray => "tray",
        MechanicalMount::Rigid => "rigid",
        MechanicalMount::Gasket => "gasket",
    }
}

pub fn parse_mount(value: &str) -> Option<MechanicalMount> {
    match value {
        "tray" => Some(MechanicalMount::Tray),
        "rigid" => Some(MechanicalMount::Rigid),
        "gasket" => Some(MechanicalMount::Gasket),
        _ => None,
    }
}

pub fn bottom_style_id(value: &MechanicalBottomStyle) -> &'static str {
    match value {
        MechanicalBottomStyle::Shell => "shell",
        MechanicalBottomStyle::Sheet => "sheet",
    }
}

pub fn parse_bottom_style(value: &str) -> Option<MechanicalBottomStyle> {
    match value {
        "shell" => Some(MechanicalBottomStyle::Shell),
        "sheet" => Some(MechanicalBottomStyle::Sheet),
        _ => None,
    }
}

pub fn family_id(value: MechanicalSwitchFamily) -> &'static str {
    match value {
        MechanicalSwitchFamily::Mx => "mx",
        MechanicalSwitchFamily::ChocV1 => "choc-v1",
        MechanicalSwitchFamily::ChocV2 => "choc-v2",
    }
}

pub fn family_label(value: MechanicalSwitchFamily) -> &'static str {
    match value {
        MechanicalSwitchFamily::Mx => "MX",
        MechanicalSwitchFamily::ChocV1 => "Choc v1",
        MechanicalSwitchFamily::ChocV2 => "Choc v2",
    }
}

pub fn parse_family(value: &str) -> Option<MechanicalSwitchFamily> {
    match value {
        "mx" => Some(MechanicalSwitchFamily::Mx),
        "choc-v1" => Some(MechanicalSwitchFamily::ChocV1),
        "choc-v2" => Some(MechanicalSwitchFamily::ChocV2),
        _ => None,
    }
}

pub fn severity_label(value: &Severity) -> &'static str {
    match value {
        Severity::Error => "Error",
        Severity::Warning => "Warning",
        Severity::Info => "Info",
    }
}

pub fn closure_dimension_fields(
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

pub fn patch_field_id(patch: &MechanicalSettingsPatch) -> String {
    match patch {
        MechanicalSettingsPatch::Enable => "configure".to_owned(),
        MechanicalSettingsPatch::Disable => "disable".to_owned(),
        MechanicalSettingsPatch::SetBatteryEnabled(_) => "battery-enabled".to_owned(),
        MechanicalSettingsPatch::SetMethod(_) => "method".to_owned(),
        MechanicalSettingsPatch::SetMount(_) => "mount".to_owned(),
        MechanicalSettingsPatch::SetBottomStyle(_) => "bottom-style".to_owned(),
        MechanicalSettingsPatch::SetMiddleFrame(_) => "middle-frame".to_owned(),
        MechanicalSettingsPatch::SetIntegratedPlateFrame(_) => "integrated-plate-frame".to_owned(),
        MechanicalSettingsPatch::SetDimension { field, .. } => field.field_id().to_owned(),
        MechanicalSettingsPatch::SetClosureThread(_) => "closure-thread".to_owned(),
        MechanicalSettingsPatch::SetClosureScrewLengths(_) => "closure-screw-lengths".to_owned(),
        MechanicalSettingsPatch::SetClosureDrive(_) => "closure-drive".to_owned(),
        MechanicalSettingsPatch::SetClosureInstallation(_) => "closure-installation".to_owned(),
        MechanicalSettingsPatch::SetClosureHeadProfile(_) => "closure-head-profile".to_owned(),
        MechanicalSettingsPatch::SetClosureLengthDatum(_) => "closure-length-datum".to_owned(),
        MechanicalSettingsPatch::AddCriticalFit { .. } => "critical-fit:add".to_owned(),
        MechanicalSettingsPatch::AddHardware { .. } => "hardware:add".to_owned(),
        MechanicalSettingsPatch::AddOpening => "case-openings".to_owned(),
        MechanicalSettingsPatch::SetCriticalFitText { fit_id, field, .. } => format!(
            "critical-fit:{fit_id}:{}",
            match field {
                MechanicalCriticalFitTextField::Label => "label",
                MechanicalCriticalFitTextField::Tolerance => "tolerance",
            }
        ),
        MechanicalSettingsPatch::SetHardwareText {
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
        _ => "other".to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn number_rules_cover_bounds_and_messages() {
        let cases = [
            (NumberRule::Nonnegative, 0.0, true, "0", "zero or greater"),
            (NumberRule::Nonnegative, -0.1, false, "0", "zero or greater"),
            (
                NumberRule::AtLeastTenth,
                0.1,
                true,
                "0.1",
                "0.1 mm or greater",
            ),
            (
                NumberRule::AtLeastTenth,
                0.09,
                false,
                "0.1",
                "0.1 mm or greater",
            ),
            (NumberRule::AtLeastOne, 1.0, true, "1", "at least 1"),
            (NumberRule::AtLeastOne, 1.5, false, "1", "at least 1"),
            (NumberRule::AtLeastFive, 5.0, true, "5", "at least 5 mm"),
            (NumberRule::AtLeastFive, 4.9, false, "5", "at least 5 mm"),
            (NumberRule::AtLeastHalf, 0.5, true, "0.5", "0.5 mm"),
            (NumberRule::AtLeastHalf, 0.49, false, "0.5", "0.5 mm"),
            (
                NumberRule::Coordinate,
                -1_000_000.0,
                true,
                "-1000000",
                "1,000,000",
            ),
            (
                NumberRule::Coordinate,
                -1_000_000.1,
                false,
                "-1000000",
                "1,000,000",
            ),
            (NumberRule::Bounded(-1, 1), 1.0, true, "-1", "-1 to 1"),
            (NumberRule::Bounded(-1, 1), 1.1, false, "-1", "-1 to 1"),
        ];
        for (rule, value, valid, min, message) in cases {
            assert_eq!(rule.valid(value), valid, "{rule:?} at {value}");
            assert_eq!(rule.minimum(), min);
            assert!(rule.error().contains(message), "{rule:?} message");
            assert!(!rule.valid(f64::NAN), "{rule:?} rejects NaN");
            assert!(!rule.valid(f64::INFINITY), "{rule:?} rejects infinite");
        }
    }

    #[test]
    fn dimension_field_ids_rules_and_steps_are_stable() {
        let table = [
            (
                MechanicalDimension::PlateThickness,
                "plate-thickness",
                NumberRule::Nonnegative,
                "0.1",
            ),
            (
                MechanicalDimension::OpeningAllowance,
                "opening-allowance",
                NumberRule::Bounded(-1, 1),
                "0.1",
            ),
            (
                MechanicalDimension::BatteryWidth,
                "battery-width",
                NumberRule::AtLeastTenth,
                "0.1",
            ),
            (
                MechanicalDimension::BatteryPositionX,
                "battery-position-x",
                NumberRule::Coordinate,
                "0.1",
            ),
            (
                MechanicalDimension::GasketSupportLength,
                "gasket-support-length",
                NumberRule::AtLeastFive,
                "5",
            ),
            (
                MechanicalDimension::GasketSupportWidth,
                "gasket-support-width",
                NumberRule::AtLeastHalf,
                "0.5",
            ),
            (
                MechanicalDimension::HardwareQuantity,
                "hardware-quantity",
                NumberRule::AtLeastOne,
                "1",
            ),
            (
                MechanicalDimension::HardwareLength,
                "hardware-length",
                NumberRule::AtLeastTenth,
                "0.1",
            ),
        ];
        for (field, id, rule, step) in table {
            assert_eq!(field.field_id(), id);
            assert_eq!(field.rule(), rule);
            assert_eq!(field.step(), step);
        }
    }

    #[test]
    fn dimension_field_routing_covers_families() {
        assert!(is_dimension_field("plate-thickness"));
        assert!(is_dimension_field("opening:open-1:volume:opening-bottom-z"));
        assert!(is_dimension_field("mount:suspension:m1:mount-position-x"));
        assert!(!is_dimension_field("mount:suspension:m1:unknown"));
        assert!(is_dimension_field("closure-thread-diameter"));
        assert!(is_dimension_field("critical-fit:f1:critical-fit-from-x"));
        assert!(is_dimension_field("hardware:h1:hardware-length"));
        assert!(is_dimension_field("opening-allowance"));
        assert!(!is_dimension_field("configure"));
        assert!(!is_dimension_field("battery-enabled"));
    }

    #[test]
    fn mount_collections_and_choice_ids_round_trip() {
        assert_eq!(
            mount_collection_id(MechanicalMountCollection::Suspension),
            "suspension"
        );
        assert_eq!(
            mount_collection_id(MechanicalMountCollection::Closure),
            "closure"
        );
        for (id, method) in [
            ("pcb-fr4", PlateMethod::PcbFr4),
            ("printed", PlateMethod::Printed),
            ("cnc", PlateMethod::Cnc),
            ("cut-sheet", PlateMethod::CutSheet),
        ] {
            assert_eq!(method_id(&method), id);
            assert_eq!(parse_method(id), Some(method));
        }
        assert_eq!(parse_method("bogus"), None);
        for (id, mount) in [
            ("tray", MechanicalMount::Tray),
            ("rigid", MechanicalMount::Rigid),
            ("gasket", MechanicalMount::Gasket),
        ] {
            assert_eq!(mount_id(&mount), id);
            assert_eq!(parse_mount(id), Some(mount));
        }
        assert_eq!(parse_mount("bogus"), None);
        assert_eq!(bottom_style_id(&MechanicalBottomStyle::Shell), "shell");
        assert_eq!(
            parse_bottom_style("sheet"),
            Some(MechanicalBottomStyle::Sheet)
        );
        assert_eq!(parse_bottom_style("bogus"), None);
        for (id, label, family) in [
            ("mx", "MX", MechanicalSwitchFamily::Mx),
            ("choc-v1", "Choc v1", MechanicalSwitchFamily::ChocV1),
            ("choc-v2", "Choc v2", MechanicalSwitchFamily::ChocV2),
        ] {
            assert_eq!(family_id(family), id);
            assert_eq!(family_label(family), label);
            assert_eq!(parse_family(id), Some(family));
        }
        assert_eq!(parse_family("bogus"), None);
        assert_eq!(severity_label(&Severity::Error), "Error");
        assert_eq!(severity_label(&Severity::Warning), "Warning");
        assert_eq!(severity_label(&Severity::Info), "Info");
    }

    #[test]
    fn closure_dimensions_project_all_eighteen_fields() {
        let hardware = InternalClosureHardware {
            drive: boardstudio_model::model::ScrewDrive::Torx,
            installation: boardstudio_model::model::InsertInstallation::HeatSet,
            thread_diameter: 3.0,
            pitch: 0.5,
            insert_length: 5.0,
            thread_start: 1.0,
            tip_allowance: 0.5,
            seat_lead_depth: 0.8,
            seat_lead_diameter: 4.1,
            bearing_thickness: 1.2,
            fixed_length: None,
            length_datum: boardstudio_model::model::ScrewLengthDatum::UnderHead,
            head_profile: boardstudio_model::model::ScrewHeadProfile::Flat,
            id: "closure".into(),
            thread: "M3".into(),
            screw_lengths: vec![6.0],
            head_diameter: 5.0,
            head_height: 2.0,
            hole_diameter: 3.2,
            insert_diameter: 4.0,
            seat_diameter: 4.2,
            seat_depth: 3.0,
            engagement: 4.0,
            bottoming_clearance: 0.3,
            roof: 1.0,
            surround: 2.0,
        };
        let fields = closure_dimension_fields(&hardware);
        assert_eq!(fields.len(), 18);
        let ids: Vec<_> = fields
            .iter()
            .map(|(field, _, _)| field.field_id())
            .collect();
        assert!(ids.contains(&"closure-thread-diameter"));
        assert!(ids.contains(&"closure-bearing-thickness"));
        assert_eq!(fields[0].2, 3.0);
    }

    #[test]
    fn patch_field_ids_cover_typed_intents() {
        let table: Vec<(MechanicalSettingsPatch, &str)> = vec![
            (MechanicalSettingsPatch::Enable, "configure"),
            (MechanicalSettingsPatch::Disable, "disable"),
            (
                MechanicalSettingsPatch::SetBatteryEnabled(true),
                "battery-enabled",
            ),
            (
                MechanicalSettingsPatch::SetMethod(PlateMethod::Printed),
                "method",
            ),
            (
                MechanicalSettingsPatch::SetDimension {
                    field: MechanicalDimension::WallThickness,
                    value: 2.0,
                },
                "wall-thickness",
            ),
            (
                MechanicalSettingsPatch::SetClosureThread("M3".into()),
                "closure-thread",
            ),
            (
                MechanicalSettingsPatch::AddCriticalFit {
                    part_id: "p".into(),
                },
                "critical-fit:add",
            ),
        ];
        for (patch, id) in table {
            assert_eq!(patch_field_id(&patch), id);
        }
    }
}
