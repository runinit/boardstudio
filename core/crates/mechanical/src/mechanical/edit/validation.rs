//! Profile, mount, opening and dimension validation.

use super::super::change::{MechanicalDimension, MechanicalMountCollection};
use super::closure_hardware_mut;
use crate::model::*;

#[derive(Clone, Copy)]
pub(super) enum ProfileTargetKind {
    Switch,
    Stabilizer,
    ImportedGeometry,
}

pub(super) fn validate_profile_target(
    document: &ProjectDoc,
    board_id: &str,
    configuration: &MechanicalConfiguration,
    definition_id: &str,
    target_kind: ProfileTargetKind,
) -> Result<(), String> {
    if configuration.board_id != board_id {
        return Err("The mechanical configuration belongs to another board.".into());
    }
    if configuration
        .profiles
        .iter()
        .any(|profile| profile.definition_id == definition_id)
    {
        return Err("A mechanical profile is already assigned to this part type.".into());
    }
    let board = document
        .boards
        .iter()
        .find(|board| board.id == board_id)
        .ok_or_else(|| "The selected board is no longer available.".to_owned())?;
    let part = document
        .parts
        .iter()
        .find(|part| board.part_ids.contains(&part.id) && part.definition_id == definition_id);
    let definition = document
        .definitions
        .iter()
        .find(|definition| definition.id == definition_id);
    let eligible = match (part, definition) {
        (Some(_), Some(definition)) if matches!(target_kind, ProfileTargetKind::Switch) => {
            definition.kind == PartKind::Switch
                || definition
                    .generator
                    .as_ref()
                    .is_some_and(|generator| generator.source.ends_with("/switch_choc_v1_v2"))
        }
        (Some(_), Some(_)) if matches!(target_kind, ProfileTargetKind::Stabilizer) => true,
        (Some(_), Some(definition)) => definition.kicad_source.is_some(),
        _ => false,
    };
    if !eligible {
        return Err("The selected part type is not available for this profile assignment.".into());
    }
    Ok(())
}

pub(super) fn mount_collection_mut(
    configuration: &mut MechanicalConfiguration,
    collection: MechanicalMountCollection,
) -> &mut Vec<Mount> {
    match collection {
        MechanicalMountCollection::Suspension => &mut configuration.mounts,
        MechanicalMountCollection::Closure => {
            configuration.closure_mounts.get_or_insert_with(Vec::new)
        }
    }
}

pub(super) fn validate_gasket_support_anchors(
    anchors: &[MechanicalGasketAnchor],
) -> Result<(), String> {
    let mut ids = std::collections::HashSet::new();
    if anchors.iter().any(|anchor| {
        anchor.id.is_empty()
            || !ids.insert(&anchor.id)
            || !anchor.anchor.is_finite()
            || !(0.0..=1.0).contains(&anchor.anchor)
    }) {
        return Err("The gasket support placement is invalid.".into());
    }
    Ok(())
}

pub(super) fn validate_mounts(mounts: &[Mount]) -> Result<(), String> {
    let mut ids = std::collections::HashSet::new();
    for mount in mounts {
        if mount.id.is_empty() || !ids.insert(mount.id.as_str()) {
            return Err("Mount IDs must be non-empty and unique.".into());
        }
        if !mount.at.x.is_finite() || !mount.at.y.is_finite() {
            return Err("Mount positions must be finite values.".into());
        }
        validate_mount_positive(mount.hole_diameter, "Hole diameter")?;
        validate_mount_positive(mount.boss_diameter.unwrap_or(5.0), "Boss diameter")?;
        if mount.kind == MountKind::Boss {
            validate_mount_positive(mount.height.unwrap_or(5.0), "Boss height")?;
        }
    }
    Ok(())
}

pub(super) fn valid_profile_polygons(polygons: &[Vec<Vec2>]) -> bool {
    polygons.iter().all(|polygon| {
        polygon.len() >= 3
            && polygon
                .iter()
                .all(|point| point.x.is_finite() && point.y.is_finite())
    })
}

pub(super) fn validate_profile_openings(
    openings: &[CaseOpening],
    label: &str,
) -> Result<(), String> {
    for (index, opening) in openings.iter().enumerate() {
        validate_opening_geometry(&opening.points, opening.z, opening.height, label, index)?;
    }
    Ok(())
}

pub(super) fn validate_opening_geometry(
    points: &[Vec2],
    z: f64,
    height: f64,
    label: &str,
    index: usize,
) -> Result<(), String> {
    if points.len() < 3
        || points.iter().any(|point| {
            !point.x.is_finite()
                || !point.y.is_finite()
                || point.x < -boardstudio_contracts::MAX_COORDINATE_MM
                || point.y < -boardstudio_contracts::MAX_COORDINATE_MM
        })
    {
        return Err(format!(
            "{label} {} needs at least three vertices with finite coordinates no smaller than −1,000,000 mm.",
            index + 1
        ));
    }
    if !z.is_finite() || z < -boardstudio_contracts::MAX_COORDINATE_MM {
        return Err(format!(
            "{label} {} bottom Z must be a finite coordinate no smaller than −1,000,000 mm.",
            index + 1
        ));
    }
    if !height.is_finite() || height < 0.1 {
        return Err(format!(
            "{label} {} height must be at least 0.1 mm.",
            index + 1
        ));
    }
    Ok(())
}

pub(super) fn validate_openings(openings: &[MechanicalOpening]) -> Result<(), String> {
    for (index, opening) in openings.iter().enumerate() {
        validate_opening_geometry(
            &opening.points(),
            opening.z,
            opening.height,
            "Access opening",
            index,
        )?;
    }
    Ok(())
}

pub(super) fn validate_mount_positive(value: f64, label: &str) -> Result<(), String> {
    if value.is_finite() && value > 0.0 {
        Ok(())
    } else {
        Err(format!("{label} must be a finite value greater than zero."))
    }
}

pub(super) fn set_dimension(
    configuration: &mut MechanicalConfiguration,
    field: MechanicalDimension,
    value: f64,
) -> Result<(), String> {
    match field {
        MechanicalDimension::PlateThickness => configuration.plate_thickness = value,
        MechanicalDimension::PlateFoamThickness => configuration.plate_foam_thickness = value,
        MechanicalDimension::PcbThickness => configuration.pcb_thickness = value,
        MechanicalDimension::BottomFoamThickness => configuration.bottom_foam_thickness = value,
        MechanicalDimension::BottomThickness => configuration.bottom_thickness = value,
        MechanicalDimension::WallThickness => configuration.wall_thickness = value,
        MechanicalDimension::Clearance => configuration.clearance = value,
        MechanicalDimension::OpeningAllowance => configuration.opening_allowance = Some(value),
        MechanicalDimension::MountPositionX
        | MechanicalDimension::MountPositionY
        | MechanicalDimension::MountHoleDiameter
        | MechanicalDimension::MountBossDiameter
        | MechanicalDimension::MountBossHeight => {
            return Err("Mount dimensions require a current mount selection.".into());
        }
        MechanicalDimension::GasketSupportLength | MechanicalDimension::GasketSupportWidth => {
            return Err("Gasket support dimensions require a current support selection.".into());
        }
        MechanicalDimension::OpeningBottomZ
        | MechanicalDimension::OpeningHeight
        | MechanicalDimension::OpeningPointX
        | MechanicalDimension::OpeningPointY => {
            return Err("Opening dimensions require a current access opening selection.".into());
        }
        MechanicalDimension::ClosureThreadDiameter
        | MechanicalDimension::ClosurePitch
        | MechanicalDimension::ClosureHeadDiameter
        | MechanicalDimension::ClosureHeadHeight
        | MechanicalDimension::ClosureHoleDiameter
        | MechanicalDimension::ClosureInsertDiameter
        | MechanicalDimension::ClosureInsertLength
        | MechanicalDimension::ClosureSeatDiameter
        | MechanicalDimension::ClosureSeatDepth
        | MechanicalDimension::ClosureEngagement
        | MechanicalDimension::ClosureThreadStart
        | MechanicalDimension::ClosureTipAllowance
        | MechanicalDimension::ClosureBottomingClearance
        | MechanicalDimension::ClosureRoof
        | MechanicalDimension::ClosureSurround
        | MechanicalDimension::ClosureSeatLeadDepth
        | MechanicalDimension::ClosureSeatLeadDiameter
        | MechanicalDimension::ClosureBearingThickness => {
            let hardware = closure_hardware_mut(configuration)?;
            match field {
                MechanicalDimension::ClosureThreadDiameter => hardware.thread_diameter = value,
                MechanicalDimension::ClosurePitch => hardware.pitch = value,
                MechanicalDimension::ClosureHeadDiameter => hardware.head_diameter = value,
                MechanicalDimension::ClosureHeadHeight => hardware.head_height = value,
                MechanicalDimension::ClosureHoleDiameter => hardware.hole_diameter = value,
                MechanicalDimension::ClosureInsertDiameter => hardware.insert_diameter = value,
                MechanicalDimension::ClosureInsertLength => hardware.insert_length = value,
                MechanicalDimension::ClosureSeatDiameter => hardware.seat_diameter = value,
                MechanicalDimension::ClosureSeatDepth => hardware.seat_depth = value,
                MechanicalDimension::ClosureEngagement => hardware.engagement = value,
                MechanicalDimension::ClosureThreadStart => hardware.thread_start = value,
                MechanicalDimension::ClosureTipAllowance => hardware.tip_allowance = value,
                MechanicalDimension::ClosureBottomingClearance => {
                    hardware.bottoming_clearance = value
                }
                MechanicalDimension::ClosureRoof => hardware.roof = value,
                MechanicalDimension::ClosureSurround => hardware.surround = value,
                MechanicalDimension::ClosureSeatLeadDepth => hardware.seat_lead_depth = value,
                MechanicalDimension::ClosureSeatLeadDiameter => hardware.seat_lead_diameter = value,
                MechanicalDimension::ClosureBearingThickness => hardware.bearing_thickness = value,
                _ => unreachable!("closure dimension variant checked above"),
            }
        }
        MechanicalDimension::CriticalFitFromX
        | MechanicalDimension::CriticalFitFromY
        | MechanicalDimension::CriticalFitToX
        | MechanicalDimension::CriticalFitToY => {
            return Err("Critical-fit endpoint dimensions require a current fit selection.".into());
        }
        MechanicalDimension::HardwareLength | MechanicalDimension::HardwareQuantity => {
            return Err("Hardware dimensions require a current hardware selection.".into());
        }
        MechanicalDimension::BatteryWidth
        | MechanicalDimension::BatteryDepth
        | MechanicalDimension::BatteryHeight
        | MechanicalDimension::BatteryCableWidth
        | MechanicalDimension::BatteryPositionX
        | MechanicalDimension::BatteryPositionY
        | MechanicalDimension::BatteryCableExitX
        | MechanicalDimension::BatteryCableExitY => {
            let battery = configuration.battery.as_mut().ok_or_else(|| {
                "The battery envelope was removed before this field update.".to_owned()
            })?;
            match field {
                MechanicalDimension::BatteryWidth => battery.size.x = value,
                MechanicalDimension::BatteryDepth => battery.size.y = value,
                MechanicalDimension::BatteryHeight => battery.size.z = value,
                MechanicalDimension::BatteryCableWidth => battery.cable_width = Some(value),
                MechanicalDimension::BatteryPositionX => battery.at.x = value,
                MechanicalDimension::BatteryPositionY => battery.at.y = value,
                MechanicalDimension::BatteryCableExitX => battery.cable_exit.x = value,
                MechanicalDimension::BatteryCableExitY => battery.cable_exit.y = value,
                _ => unreachable!("battery dimension variant checked above"),
            }
        }
    }
    Ok(())
}

pub(super) fn validate_dimension(field: MechanicalDimension, value: f64) -> Result<(), String> {
    let valid = value.is_finite()
        && match field {
            MechanicalDimension::OpeningAllowance => (-1.0..=1.0).contains(&value),
            MechanicalDimension::BatteryWidth
            | MechanicalDimension::BatteryDepth
            | MechanicalDimension::BatteryHeight
            | MechanicalDimension::BatteryCableWidth
            | MechanicalDimension::HardwareLength => value >= 0.1,
            MechanicalDimension::HardwareQuantity => {
                value >= 1.0 && value.fract() == 0.0 && value <= f64::from(u32::MAX)
            }
            MechanicalDimension::OpeningHeight => value >= 0.1,
            MechanicalDimension::GasketSupportLength => value >= 5.0,
            MechanicalDimension::GasketSupportWidth => value >= 0.5,
            MechanicalDimension::OpeningBottomZ
            | MechanicalDimension::OpeningPointX
            | MechanicalDimension::OpeningPointY
            | MechanicalDimension::BatteryPositionX
            | MechanicalDimension::BatteryPositionY
            | MechanicalDimension::BatteryCableExitX
            | MechanicalDimension::BatteryCableExitY
            | MechanicalDimension::MountPositionX
            | MechanicalDimension::MountPositionY
            | MechanicalDimension::CriticalFitFromX
            | MechanicalDimension::CriticalFitFromY
            | MechanicalDimension::CriticalFitToX
            | MechanicalDimension::CriticalFitToY => {
                value >= -boardstudio_contracts::MAX_COORDINATE_MM
            }
            _ => value >= 0.0,
        };
    if valid {
        Ok(())
    } else if field == MechanicalDimension::GasketSupportLength {
        Err("Cut length must be at least 5 mm.".into())
    } else if field == MechanicalDimension::GasketSupportWidth {
        Err("Pad width must be at least 0.5 mm.".into())
    } else if field == MechanicalDimension::OpeningAllowance {
        Err("Opening allowance must be between −1 and 1 mm.".into())
    } else if matches!(
        field,
        MechanicalDimension::BatteryWidth
            | MechanicalDimension::BatteryDepth
            | MechanicalDimension::BatteryHeight
            | MechanicalDimension::BatteryCableWidth
    ) {
        Err("Battery dimensions and cable width must be at least 0.1 mm.".into())
    } else if field == MechanicalDimension::HardwareLength {
        Err("Hardware length must be at least 0.1 mm.".into())
    } else if field == MechanicalDimension::HardwareQuantity {
        Err("Hardware quantity must be a positive whole number.".into())
    } else if field == MechanicalDimension::OpeningHeight {
        Err("Access opening height must be at least 0.1 mm.".into())
    } else if matches!(
        field,
        MechanicalDimension::BatteryPositionX
            | MechanicalDimension::BatteryPositionY
            | MechanicalDimension::BatteryCableExitX
            | MechanicalDimension::BatteryCableExitY
    ) {
        Err("Battery positions and cable exits must be at least −1,000,000 mm.".into())
    } else if matches!(
        field,
        MechanicalDimension::CriticalFitFromX
            | MechanicalDimension::CriticalFitFromY
            | MechanicalDimension::CriticalFitToX
            | MechanicalDimension::CriticalFitToY
    ) {
        Err("Critical-fit coordinates must be at least −1,000,000 mm.".into())
    } else {
        Err("Mechanical dimensions must be finite and nonnegative.".into())
    }
}
