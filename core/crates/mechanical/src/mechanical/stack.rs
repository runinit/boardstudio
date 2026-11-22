//! Layer profile resolution and the resolved layer stack.

use super::{
    assigned_family, builtin_profile, effective_family, inferred_switch_family,
    is_legacy_mx_stabilizer, is_switch_position, normalized_processes, profile_source_for_family,
    subtract_foam_exclusions, switch_mounting_datum,
};
use crate::model::*;

pub(super) struct ResolutionInputs {
    pub(super) config: MechanicalConfiguration,
    pub(super) incompatible_switches: Vec<String>,
}

/// Resolve configuration and switch fit before any geometry is emitted.
/// Keeping this stage independent makes profile authority and fit diagnostics
/// testable without coupling them to case construction.
pub(super) fn resolve_profiles(
    document: &ProjectDoc,
    config: &MechanicalConfiguration,
) -> Option<ResolutionInputs> {
    let board = document
        .boards
        .iter()
        .find(|board| board.id == config.board_id)?;
    let switch_parts = document
        .parts
        .iter()
        .filter_map(|part| {
            if !board.part_ids.contains(&part.id) {
                return None;
            }
            let definition = document
                .definitions
                .iter()
                .find(|definition| definition.id == part.definition_id)?;
            is_switch_position(part, definition).then_some((part, definition))
        })
        .collect::<Vec<_>>();
    let mut effective_config = config.clone();
    for definition in &document.definitions {
        if let Some(profile) = &definition.mechanical_profile {
            effective_config
                .profiles
                .retain(|entry| entry.definition_id != definition.id);
            let mut inherited = profile.clone();
            inherited.definition_id = definition.id.clone();
            effective_config.profiles.push(inherited);
        }
    }
    effective_config.pcb_thickness = board.thickness;
    if effective_config.battery.is_none() {
        effective_config.battery_height = 0.0;
    }
    effective_config.part_processes = Some(normalized_processes(&effective_config));
    for (part, definition) in &switch_parts {
        let Some(family) = effective_family(&effective_config, part, definition) else {
            continue;
        };
        if !effective_config
            .profiles
            .iter()
            .any(|profile| profile.definition_id == part.definition_id)
            && let Ok(mut profile) = builtin_profile(
                part.definition_id.clone(),
                profile_source_for_family(family),
                switch_mounting_datum(family) - effective_config.plate_thickness,
            )
        {
            profile.switch_family = Some(family);
            effective_config.profiles.push(profile);
        }
        if let Some(profile) = effective_config
            .profiles
            .iter_mut()
            .find(|profile| profile.definition_id == part.definition_id)
        {
            profile.switch_family.get_or_insert(family);
        }
    }
    for profile in &mut effective_config.profiles {
        if profile.switch_family.is_none() && is_legacy_mx_stabilizer(profile) {
            profile.switch_family = Some(MechanicalSwitchFamily::Mx);
        }
        // Family compatibility also applies to separate stabilizer definitions.
        if let Some(family) = profile.switch_family {
            profile.plate_to_pcb = switch_mounting_datum(family) - effective_config.plate_thickness;
        }
    }
    let resolved_family = switch_parts
        .iter()
        .find_map(|(part, definition)| effective_family(&effective_config, part, definition));
    let mut incompatible_switches = resolved_family
        .map(|family| {
            let datum = switch_mounting_datum(family);
            switch_parts
                .iter()
                .filter_map(|(part, definition)| {
                    let other = effective_family(&effective_config, part, definition)?;
                    ((switch_mounting_datum(other) - datum).abs() > 0.001)
                        .then_some(part.id.clone())
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    for (part, definition) in &switch_parts {
        let assigned = assigned_family(&effective_config, part);
        let inferred = inferred_switch_family(part, definition);
        if assigned.is_some() && inferred.is_some() && assigned != inferred {
            incompatible_switches.push(part.id.clone());
        }
    }
    incompatible_switches.sort();
    incompatible_switches.dedup();
    if let Some(family) = resolved_family {
        effective_config.plate_to_pcb =
            switch_mounting_datum(family) - effective_config.plate_thickness;
    }
    if effective_config.mount == MechanicalMount::Gasket && effective_config.gasket_travel.is_none()
    {
        effective_config.gasket_travel = Some(0.3);
    }
    Some(ResolutionInputs {
        config: effective_config,
        incompatible_switches,
    })
}

pub(super) fn build_stack(
    config: &MechanicalConfiguration,
    pcb_bottom: f64,
    bottom_foam_z: f64,
    battery_z: f64,
    battery_height: f64,
    bottom_z: f64,
) -> Vec<MechanicalStackLayer> {
    [
        ("plate", config.plate_to_pcb, config.plate_thickness),
        ("plate-foam", 0.0, config.plate_foam_thickness),
        ("pcb", pcb_bottom, config.pcb_thickness),
        ("bottom-foam", bottom_foam_z, config.bottom_foam_thickness),
        ("battery", battery_z, battery_height),
        ("bottom", bottom_z, config.bottom_thickness),
    ]
    .into_iter()
    .filter(|(_, _, thickness)| *thickness > 0.0)
    .map(|(id, z, thickness)| MechanicalStackLayer {
        id: id.into(),
        z,
        thickness,
    })
    .collect()
}

pub(super) fn frame_dimensions_are_valid(config: &MechanicalConfiguration) -> bool {
    config.wall_thickness.is_finite()
        && config.wall_thickness > 0.0
        && config.clearance.is_finite()
        && config.clearance >= 0.0
}

pub(super) fn dimensions_are_valid(config: &MechanicalConfiguration) -> bool {
    let dimensions = [
        config.plate_thickness,
        config.pcb_thickness,
        config.bottom_thickness,
        config.plate_to_pcb,
    ];
    let clearances = [
        config.plate_foam_thickness,
        config.bottom_foam_thickness,
        config.battery_height,
    ];
    frame_dimensions_are_valid(config)
        && dimensions
            .iter()
            .all(|value| value.is_finite() && *value > 0.0)
        && clearances
            .iter()
            .all(|value| value.is_finite() && *value >= 0.0)
}

pub(super) fn resolve_foam_contours(
    base: &[Contour],
    clearances: &[Contour],
) -> Result<Vec<Contour>, String> {
    if clearances.is_empty() {
        return Ok(base.to_vec());
    }
    subtract_foam_exclusions(base, clearances)
}
