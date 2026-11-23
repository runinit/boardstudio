//! Pure Case mechanical rules shared by the wasm inspector and native tests: the range each
//! dimension accepts and the stable field id a patch is reported under.
use boardstudio_core_mechanical::mechanical::change::{
    MechanicalChange as MechanicalSettingsPatch, MechanicalDimension,
};
use boardstudio_model::model::{InternalClosureHardware, MAX_COORDINATE_MM};

/// `(min, max, step)` the inspector offers for one dimension. Core still validates the commit.
pub fn dimension_range(field: MechanicalDimension) -> (f64, Option<f64>, f64) {
    use MechanicalDimension::*;
    match field {
        OpeningAllowance => (-1.0, Some(1.0), 0.1),
        OpeningHeight | BatteryWidth | BatteryDepth | BatteryHeight | BatteryCableWidth
        | MountHoleDiameter | MountBossDiameter | MountBossHeight | HardwareLength => {
            (0.1, None, 0.1)
        }
        HardwareQuantity => (1.0, None, 1.0),
        GasketSupportLength => (5.0, None, 5.0),
        GasketSupportWidth => (0.5, None, 0.5),
        OpeningBottomZ | OpeningPointX | OpeningPointY | BatteryPositionX | BatteryPositionY
        | BatteryCableExitX | BatteryCableExitY | MountPositionX | MountPositionY
        | CriticalFitFromX | CriticalFitFromY | CriticalFitToX | CriticalFitToY => {
            (-MAX_COORDINATE_MM, Some(MAX_COORDINATE_MM), 0.1)
        }
        _ => (0.0, None, 0.1),
    }
}

/// `(id, label, thread diameter, pitch, insert length, insert outside diameter)`.
pub const CLOSURE_INSERT_PRESETS: [(&str, &str, f64, f64, f64, f64); 6] = [
    ("m2-3", "M2 × L3 × OD3.2", 2.0, 0.4, 3.0, 3.2),
    ("m2-4", "M2 × L4 × OD3.2", 2.0, 0.4, 4.0, 3.2),
    ("m2.5-3", "M2.5 × L3 × OD3.5", 2.5, 0.45, 3.0, 3.5),
    ("m2.5-4", "M2.5 × L4 × OD3.5", 2.5, 0.45, 4.0, 3.5),
    ("m2.5-5", "M2.5 × L5 × OD3.5", 2.5, 0.45, 5.0, 3.5),
    ("m3-3", "M3 × L3 × OD4.2", 3.0, 0.5, 3.0, 4.2),
];

/// The preset whose thread, insert length and insert diameter the hardware matches.
pub fn closure_preset_id(hardware: &InternalClosureHardware) -> Option<&'static str> {
    CLOSURE_INSERT_PRESETS
        .iter()
        .find(|(_, _, diameter, _, length, insert)| {
            hardware.thread_diameter == *diameter
                && hardware.insert_length == *length
                && hardware.insert_diameter == *insert
        })
        .map(|(id, ..)| *id)
}

/// Positive, de-duplicated lengths from comma-separated text.
pub fn parse_screw_lengths(text: &str) -> Result<Vec<f64>, String> {
    let mut lengths = Vec::new();
    for item in text.split(',') {
        match item.trim().parse::<f64>() {
            Ok(length) if length.is_finite() && length > 0.0 => {
                if !lengths.contains(&length) {
                    lengths.push(length);
                }
            }
            _ => return Err("Enter positive lengths separated by commas.".into()),
        }
    }
    Ok(lengths)
}

/// The closure dimensions as `(field, label, value)` rows.
pub fn closure_dimension_fields(
    h: &InternalClosureHardware,
) -> [(MechanicalDimension, &'static str, f64); 18] {
    use MechanicalDimension as D;
    [
        (
            D::ClosureThreadDiameter,
            "Thread diameter",
            h.thread_diameter,
        ),
        (D::ClosurePitch, "Thread pitch", h.pitch),
        (D::ClosureHeadDiameter, "Head diameter", h.head_diameter),
        (D::ClosureHeadHeight, "Head height", h.head_height),
        (
            D::ClosureHoleDiameter,
            "Screw clearance hole",
            h.hole_diameter,
        ),
        (
            D::ClosureInsertDiameter,
            "Insert outside diameter",
            h.insert_diameter,
        ),
        (D::ClosureInsertLength, "Insert length", h.insert_length),
        (
            D::ClosureSeatDiameter,
            "Insert seat diameter",
            h.seat_diameter,
        ),
        (D::ClosureSeatDepth, "Insert seat depth", h.seat_depth),
        (D::ClosureEngagement, "Thread engagement", h.engagement),
        (D::ClosureThreadStart, "Thread start offset", h.thread_start),
        (
            D::ClosureTipAllowance,
            "Screw tip allowance",
            h.tip_allowance,
        ),
        (
            D::ClosureBottomingClearance,
            "Bottoming clearance",
            h.bottoming_clearance,
        ),
        (D::ClosureRoof, "Roof above insert", h.roof),
        (D::ClosureSurround, "Material around insert", h.surround),
        (
            D::ClosureSeatLeadDepth,
            "Seat lead-in depth",
            h.seat_lead_depth,
        ),
        (
            D::ClosureSeatLeadDiameter,
            "Seat lead-in diameter",
            h.seat_lead_diameter,
        ),
        (
            D::ClosureBearingThickness,
            "Material above screw head",
            h.bearing_thickness,
        ),
    ]
}

pub trait MechanicalSettingsPatchExt {
    /// Groups feedback for the same kind of edit; it is the patch's variant name.
    fn field_id(&self) -> String;
}

impl MechanicalSettingsPatchExt for MechanicalSettingsPatch {
    fn field_id(&self) -> String {
        let debug = format!("{self:?}");
        debug
            .split(['(', ' ', '{'])
            .next()
            .unwrap_or_default()
            .to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ranges_and_field_ids_follow_the_patch() {
        assert_eq!(
            dimension_range(MechanicalDimension::OpeningAllowance).1,
            Some(1.0)
        );
        assert_eq!(
            dimension_range(MechanicalDimension::HardwareQuantity).0,
            1.0
        );
        assert_eq!(
            dimension_range(MechanicalDimension::WallThickness),
            (0.0, None, 0.1)
        );
        let patch = MechanicalSettingsPatch::SetDimension {
            field: MechanicalDimension::Clearance,
            value: 0.2,
        };
        assert_eq!(patch.field_id(), "SetDimension");
        assert_eq!(MechanicalSettingsPatch::Enable.field_id(), "Enable");
    }

    #[test]
    fn screw_lengths_parse_positive_unique_values_only() {
        assert_eq!(parse_screw_lengths("4, 6,4"), Ok(vec![4.0, 6.0]));
        assert!(parse_screw_lengths("4, x").is_err());
        assert!(parse_screw_lengths("0").is_err());
        assert!(parse_screw_lengths("").is_err());
        assert_eq!(CLOSURE_INSERT_PRESETS.len(), 6);
    }
}
