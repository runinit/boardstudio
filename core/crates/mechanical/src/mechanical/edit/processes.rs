//! Layer process normalization, material options and configuration accessors.

use super::super::change::MechanicalChange;
use super::super::{inferred_switch_family, is_switch_position};
use super::validation::find_item_mut;
use crate::model::*;

pub(super) fn normalize_processes(
    configuration: &mut MechanicalConfiguration,
    change: &MechanicalChange,
) {
    let processes = configuration.part_processes.get_or_insert_with(Vec::new);
    let top_level_method_change = matches!(change, MechanicalChange::SetMethod(_))
        || matches!(
            change,
            MechanicalChange::SetProcessMethod { part_id, .. } if part_id == "plate"
        );
    for process in processes.iter_mut() {
        let is_standard = matches!(
            process.part_id.as_str(),
            "plate" | "plate-foam" | "bottom-foam" | "bottom"
        );
        let is_foam = process.part_id.ends_with("foam");
        let method = if is_foam {
            PlateMethod::CutSheet
        } else if process.part_id == "plate" || (top_level_method_change && is_standard) {
            configuration.method.clone()
        } else {
            process.method.clone()
        };
        let method_changed =
            method != process.method || (top_level_method_change && is_standard && !is_foam);
        let valid_material =
            material_options(&process.part_id, &method).contains(&process.material.as_str());
        if is_standard {
            if method_changed || !valid_material {
                process.material =
                    crate::mechanical::default_material(&process.part_id, &method).into();
            }
            process.method = method;
            process.thickness = match process.part_id.as_str() {
                "plate" => configuration.plate_thickness,
                "plate-foam" => configuration.plate_foam_thickness,
                "bottom-foam" => configuration.bottom_foam_thickness,
                "bottom" => configuration.bottom_thickness,
                _ => process.thickness,
            };
        }
    }
}

pub(super) fn is_process_target(document: &ProjectDoc, part_id: &str) -> bool {
    matches!(part_id, "plate" | "plate-foam" | "bottom-foam" | "bottom")
        || document.parts.iter().any(|part| part.id == part_id)
}

pub(super) fn process_default_thickness(
    configuration: &MechanicalConfiguration,
    part_id: &str,
) -> f64 {
    match part_id {
        "plate" => configuration.plate_thickness,
        "plate-foam" => configuration.plate_foam_thickness,
        "bottom-foam" => configuration.bottom_foam_thickness,
        "bottom" => configuration.bottom_thickness,
        _ => configuration.plate_thickness,
    }
}

pub(super) fn process_mut<'a>(
    configuration: &'a mut MechanicalConfiguration,
    document: &ProjectDoc,
    part_id: &str,
) -> Result<&'a mut MechanicalPartProcess, String> {
    if !is_process_target(document, part_id) {
        return Err("The selected Case part is no longer available.".into());
    }
    configuration
        .part_processes
        .as_mut()
        .and_then(|processes| {
            processes
                .iter_mut()
                .find(|process| process.part_id == part_id)
        })
        .ok_or_else(|| "The selected process override is no longer available.".to_owned())
}

pub(super) fn stabilizer_target_exists(
    document: &ProjectDoc,
    board_id: &str,
    part_id: &str,
) -> bool {
    let Some(board) = document.boards.iter().find(|board| board.id == board_id) else {
        return false;
    };
    let Some(part) = document
        .parts
        .iter()
        .find(|part| part.id == part_id && board.part_ids.contains(&part.id))
    else {
        return false;
    };
    let Some(definition) = document
        .definitions
        .iter()
        .find(|definition| definition.id == part.definition_id)
    else {
        return false;
    };
    let Some(size) = part.keycap.or(definition.keycap) else {
        return false;
    };
    if size.x.max(size.y) < 37.0
        || definition
            .generator
            .as_ref()
            .is_none_or(|generator| generator.source.to_lowercase() != "ceoloide/switch_mx")
    {
        return false;
    }
    !document.parts.iter().any(|stabilizer| {
        board.part_ids.contains(&stabilizer.id)
            && document
                .definitions
                .iter()
                .find(|candidate| candidate.id == stabilizer.definition_id)
                .and_then(|candidate| candidate.kicad_source.as_ref())
                .is_some_and(|source| source.source.contains("(footprint \"STAB_MX_"))
            && (stabilizer.pose.at.x - part.pose.at.x).hypot(stabilizer.pose.at.y - part.pose.at.y)
                < 0.01
    })
}

pub(super) fn material_options(part_id: &str, method: &PlateMethod) -> &'static [&'static str] {
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

pub(super) fn profile_family(
    configuration: &MechanicalConfiguration,
) -> Option<MechanicalSwitchFamily> {
    let mut families = configuration
        .profiles
        .iter()
        .filter_map(|profile| profile.switch_family);
    let first = families.next()?;
    families.all(|family| family == first).then_some(first)
}

pub(super) fn initial_switch_family(
    document: &ProjectDoc,
    board_id: &str,
) -> Option<MechanicalSwitchFamily> {
    let board = document.boards.iter().find(|board| board.id == board_id)?;
    let candidates = document
        .parts
        .iter()
        .filter(|part| board.part_ids.contains(&part.id))
        .filter_map(|part| {
            let definition = document
                .definitions
                .iter()
                .find(|definition| definition.id == part.definition_id)?;
            is_switch_position(part, definition).then(|| inferred_switch_family(part, definition))
        })
        .collect::<Vec<_>>();
    let [Some(first), rest @ ..] = candidates.as_slice() else {
        return None;
    };
    rest.iter()
        .all(|family| family.as_ref() == Some(first))
        .then_some(*first)
}

pub(super) fn family_assignment_changes(
    existing_family: Option<MechanicalSwitchFamily>,
    selected_family: MechanicalSwitchFamily,
) -> bool {
    existing_family != Some(selected_family)
}

pub(super) fn closure_hardware_mut(
    configuration: &mut MechanicalConfiguration,
) -> Result<&mut InternalClosureHardware, String> {
    configuration
        .internal_gasket
        .as_mut()
        .map(|settings| &mut settings.hardware)
        .ok_or_else(|| "Internal gasket closure hardware is no longer available.".into())
}

pub(super) fn critical_fit_mut<'a>(
    configuration: &'a mut MechanicalConfiguration,
    fit_id: &str,
) -> Result<&'a mut MechanicalCriticalFit, String> {
    find_item_mut(
        configuration
            .critical_fits
            .as_deref_mut()
            .unwrap_or_default(),
        |entry| entry.id == fit_id,
        "The selected critical fit is no longer available.",
    )
}

pub(super) fn hardware_mut<'a>(
    configuration: &'a mut MechanicalConfiguration,
    hardware_id: &str,
) -> Result<&'a mut MechanicalHardwareSpecification, String> {
    find_item_mut(
        configuration.hardware.as_deref_mut().unwrap_or_default(),
        |entry| entry.id == hardware_id,
        "The selected hardware entry is no longer available.",
    )
}

pub(super) fn resize_closure_insert(
    hardware: &mut InternalClosureHardware,
    id: &str,
) -> Result<(), String> {
    let (thread_diameter, pitch, insert_length, insert_diameter) = match id {
        "m2-3" => (2.0, 0.4, 3.0, 3.2),
        "m2-4" => (2.0, 0.4, 4.0, 3.2),
        "m2.5-3" => (2.5, 0.45, 3.0, 3.5),
        "m2.5-4" => (2.5, 0.45, 4.0, 3.5),
        "m2.5-5" => (2.5, 0.45, 5.0, 3.5),
        "m3-3" => (3.0, 0.5, 3.0, 4.2),
        _ => return Err("The selected insert preset is unavailable.".into()),
    };
    hardware.id = format!("custom-{id}");
    hardware.thread = format!("M{thread_diameter} × {pitch}");
    hardware.thread_diameter = thread_diameter;
    hardware.pitch = pitch;
    hardware.insert_length = insert_length;
    hardware.insert_diameter = insert_diameter;
    hardware.seat_diameter = insert_diameter - 0.4;
    hardware.seat_lead_diameter = insert_diameter - 0.2;
    hardware.seat_depth = insert_length + hardware.bottoming_clearance;
    hardware.engagement = (insert_length - 0.5).min(insert_length - hardware.thread_start);
    hardware.hole_diameter = thread_diameter + 0.2;
    hardware.head_diameter = thread_diameter * 2.0;
    hardware.head_height = thread_diameter / 2.0;
    Ok(())
}
