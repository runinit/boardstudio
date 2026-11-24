//! Switch, stabilizer and imported-geometry profile assignments.
use super::*;

/// Applies one change in this domain, or `None` when another domain owns it.
pub(super) fn try_apply(
    configuration: &mut MechanicalConfiguration,
    change: &MechanicalChange,
    document: &ProjectDoc,
    board_id: &str,
    changed: &mut Vec<String>,
) -> Result<Option<()>, String> {
    match change {
        MechanicalChange::AssignSwitchProfile {
            definition_id,
            family,
        } => {
            validate_profile_target(
                document,
                board_id,
                configuration,
                definition_id,
                ProfileTargetKind::Switch,
            )?;
            let existing_family = profile_family(configuration)
                .or_else(|| initial_switch_family(document, &configuration.board_id));
            let family_changed = existing_family != Some(*family);
            let plate_thickness = if family_changed {
                crate::mechanical::default_plate_thickness(*family)
            } else {
                configuration.plate_thickness
            };
            let plate_to_pcb = crate::mechanical::mounting_datum(*family) - plate_thickness;
            let source = match family {
                MechanicalSwitchFamily::Mx => MechanicalBuiltinProfile::MxSwitch,
                MechanicalSwitchFamily::ChocV1 => MechanicalBuiltinProfile::ChocV1Switch,
                MechanicalSwitchFamily::ChocV2 => MechanicalBuiltinProfile::ChocV2Switch,
            };
            let mut profile =
                crate::mechanical::builtin_profile(definition_id.clone(), source, plate_to_pcb)?;
            if profile.definition_id != *definition_id {
                return Err("Core returned a switch profile for another part type.".into());
            }
            profile.switch_family = Some(*family);
            profile.plate_to_pcb = plate_to_pcb;
            if family_changed {
                configuration.plate_thickness = plate_thickness;
                configuration.plate_to_pcb = plate_to_pcb;
                configuration.plate_foam_thickness =
                    crate::mechanical::default_plate_foam_thickness(plate_to_pcb);
            }
            configuration.profiles.push(profile);
            changed.push(definition_id.clone());
        }
        MechanicalChange::AssignStabilizerProfile {
            definition_id,
            source,
        } => {
            if !matches!(
                source,
                MechanicalBuiltinProfile::MxStab2u | MechanicalBuiltinProfile::MxStab625u
            ) {
                return Err("The selected stabilizer profile is not supported.".into());
            }
            validate_profile_target(
                document,
                board_id,
                configuration,
                definition_id,
                ProfileTargetKind::Stabilizer,
            )?;
            let mut profile = crate::mechanical::builtin_profile(
                definition_id.clone(),
                source.clone(),
                configuration.plate_to_pcb,
            )?;
            if profile.definition_id != *definition_id || profile.switch_family.is_some() {
                return Err("Core returned a different stabilizer profile.".into());
            }
            profile.plate_to_pcb = configuration.plate_to_pcb;
            configuration.profiles.push(profile);
            changed.push(definition_id.clone());
        }
        MechanicalChange::AssignImportedGeometryProfile { definition_id } => {
            let (part, definition) = validate_profile_target(
                document,
                board_id,
                configuration,
                definition_id,
                ProfileTargetKind::ImportedGeometry,
            )?;
            let family = inferred_switch_family(part, definition);
            configuration.profiles.push(MechanicalPartProfile {
                source_geometry: None,
                pcb_holes: None,
                clearance_volumes: None,
                openings: None,
                clearances: None,
                supported_thickness: None,
                switch_family: family,
                definition_id: definition.id.clone(),
                source: format!("KiCad {}", definition.name),
                cutouts: Vec::new(),
                plate_to_pcb: family.map_or(configuration.plate_to_pcb, |family| {
                    crate::mechanical::mounting_datum(family) - configuration.plate_thickness
                }),
            });
            changed.push(definition_id.clone());
        }
        MechanicalChange::UpdateProfile(next) => {
            if configuration.board_id != board_id {
                return Err("The mechanical configuration belongs to another board.".into());
            }
            let Some(profile) = configuration
                .profiles
                .iter_mut()
                .find(|profile| profile.definition_id == next.definition_id)
            else {
                return Err("The selected mechanical profile is no longer assigned.".into());
            };
            let extraction_changed = profile.source_geometry != next.source_geometry;
            if extraction_changed {
                let definition = document
                    .definitions
                    .iter()
                    .find(|definition| definition.id == next.definition_id)
                    .ok_or_else(|| {
                        "The selected mechanical profile is no longer available.".to_owned()
                    })?;
                if definition.kicad_source.as_ref().is_none_or(|source| {
                    next.source_geometry
                        .as_ref()
                        .is_none_or(|geometry| geometry.text != source.source)
                }) {
                    return Err(
                        "Extracted geometry must come from the assigned KiCad source.".into(),
                    );
                }
            } else if profile.pcb_holes != next.pcb_holes {
                return Err("PCB mounting holes can only be changed by extracting the assigned KiCad geometry.".into());
            }
            if profile.switch_family != next.switch_family
                || profile.supported_thickness != next.supported_thickness
                || profile.plate_to_pcb != next.plate_to_pcb
                || profile.source != next.source
            {
                return Err(
                    "Only extracted geometry for the assigned KiCad profile can be updated here."
                        .into(),
                );
            }
            if !valid_profile_polygons(&next.cutouts)
                || next
                    .clearances
                    .as_deref()
                    .is_some_and(|polygons| !valid_profile_polygons(polygons))
                || next.pcb_holes.iter().flatten().any(|hole| {
                    !hole.at.x.is_finite()
                        || !hole.at.y.is_finite()
                        || !hole.diameter.is_finite()
                        || hole.diameter <= 0.0
                })
            {
                return Err(
                    "Profile polygons and mounting holes must contain valid finite coordinates."
                        .into(),
                );
            }
            validate_profile_openings(
                next.openings.as_deref().unwrap_or_default(),
                "Access opening",
            )?;
            validate_profile_openings(
                next.clearance_volumes.as_deref().unwrap_or_default(),
                "Clearance volume",
            )?;
            let definition_id = next.definition_id.clone();
            *profile = next.clone();
            changed.push(definition_id);
        }
        MechanicalChange::RemoveProfile { definition_id } => {
            if configuration.board_id != board_id {
                return Err("The mechanical configuration belongs to another board.".into());
            }
            let previous_len = configuration.profiles.len();
            configuration
                .profiles
                .retain(|profile| profile.definition_id != *definition_id);
            if configuration.profiles.len() == previous_len {
                return Err("The selected mechanical profile is no longer assigned.".into());
            }
            changed.push(definition_id.clone());
        }
        MechanicalChange::SetSwitchFamily {
            definition_id,
            family,
        } => {
            if !configuration
                .profiles
                .iter()
                .any(|profile| profile.definition_id == *definition_id)
            {
                return Err("The selected switch profile is no longer available.".into());
            }
            let plate_thickness = crate::mechanical::default_plate_thickness(*family);
            let plate_to_pcb = crate::mechanical::mounting_datum(*family) - plate_thickness;
            for profile in &mut configuration.profiles {
                if profile.definition_id == *definition_id {
                    profile.switch_family = Some(*family);
                    profile.plate_to_pcb = plate_to_pcb;
                } else if let Some(other_family) = profile.switch_family {
                    profile.plate_to_pcb =
                        crate::mechanical::mounting_datum(other_family) - plate_thickness;
                }
            }
            configuration.plate_thickness = plate_thickness;
            configuration.plate_to_pcb = plate_to_pcb;
            configuration.plate_foam_thickness =
                crate::mechanical::default_plate_foam_thickness(plate_to_pcb);
            changed.push(definition_id.clone());
        }
        _ => return Ok(None),
    };
    Ok(Some(()))
}
