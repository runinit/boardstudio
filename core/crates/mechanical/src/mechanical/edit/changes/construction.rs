//! Case-body construction, battery, process and profile-engagement changes.
use super::*;

/// Applies one change in this domain; anything else is a routing error.
pub(super) fn apply(
    configuration: &mut MechanicalConfiguration,
    change: &MechanicalChange,
    document: &ProjectDoc,
    board_id: &str,
    changed: &mut Vec<String>,
) -> Result<(), String> {
    match change {
        MechanicalChange::SetBatteryEnabled(enabled) => {
            if document
                .hardware
                .as_ref()
                .is_some_and(|hardware| hardware.transport == HardwareTransport::Wireless)
            {
                return Err(
                    "Wireless battery envelopes are managed by the wireless configuration.".into(),
                );
            }
            if *enabled {
                if configuration.battery.is_some() {
                    return Err("A battery envelope is already configured.".into());
                }
                configuration.battery = Some(MechanicalBattery {
                    cable_width: Some(2.0),
                    size: Vec3 {
                        x: 30.0,
                        y: 20.0,
                        z: 6.0,
                    },
                    at: Vec2 { x: 0.0, y: 0.0 },
                    cable_exit: Vec2 { x: 0.0, y: 0.0 },
                });
            } else {
                if configuration.battery.is_none() {
                    return Err("There is no battery envelope to remove.".into());
                }
                configuration.battery = None;
            }
        }
        MechanicalChange::SetMethod(method) => configuration.method = method.clone(),
        MechanicalChange::SetBottomStyle(style) => {
            configuration.bottom_style = Some(style.clone());
        }
        MechanicalChange::SetMiddleFrame(enabled) => {
            configuration.middle_frame = Some(*enabled);
        }
        MechanicalChange::SetIntegratedPlateFrame(enabled) => {
            configuration.integrated_plate_frame = *enabled;
        }
        MechanicalChange::SetDimension { field, value } => {
            if matches!(
                *field,
                MechanicalDimension::GasketSupportLength | MechanicalDimension::GasketSupportWidth
            ) {
                return Err(
                    "Gasket support dimensions require a current support selection.".into(),
                );
            }
            if matches!(
                *field,
                MechanicalDimension::OpeningBottomZ
                    | MechanicalDimension::OpeningHeight
                    | MechanicalDimension::OpeningPointX
                    | MechanicalDimension::OpeningPointY
            ) {
                return Err(
                    "Opening dimensions require a current access opening selection.".into(),
                );
            }
            validate_dimension(*field, *value)?;
            set_dimension(configuration, *field, *value)?;
        }
        MechanicalChange::AddProcessOverride { part_id } => {
            if !is_process_target(document, part_id) {
                return Err("The selected Case part is no longer available.".into());
            }
            if configuration
                .part_processes
                .iter()
                .flatten()
                .any(|process| process.part_id == *part_id)
            {
                return Err("A process override already exists for this part.".into());
            }
            let thickness = process_default_thickness(configuration, part_id);
            let method = if part_id.ends_with("foam") {
                PlateMethod::CutSheet
            } else {
                configuration.method.clone()
            };
            configuration
                .part_processes
                .get_or_insert_with(Vec::new)
                .push(crate::mechanical::default_layer_process(
                    part_id, method, thickness,
                ));
            changed.push(part_id.clone());
        }
        MechanicalChange::RemoveProcessOverride { part_id } => {
            let processes = configuration.part_processes.as_mut().ok_or_else(|| {
                "The selected process override is no longer available.".to_owned()
            })?;
            let previous_len = processes.len();
            processes.retain(|process| process.part_id != *part_id);
            if processes.len() == previous_len {
                return Err("The selected process override is no longer available.".into());
            }
            changed.push(part_id.clone());
        }
        MechanicalChange::SetProcessMethod { part_id, method } => {
            if !is_process_target(document, part_id) || part_id.ends_with("foam") {
                return Err("The selected process method is not available.".into());
            }
            if part_id == "plate" {
                configuration.method = method.clone();
            } else {
                let process = process_mut(configuration, document, part_id)?;
                let previous_method = process.method.clone();
                process.method = method.clone();
                if previous_method != *method
                    && !material_options(part_id, method).contains(&process.material.as_str())
                {
                    process.material = crate::mechanical::default_material(part_id, method).into();
                }
            }
            changed.push(part_id.clone());
        }
        MechanicalChange::SetProcessMaterial { part_id, material } => {
            let process = process_mut(configuration, document, part_id)?;
            if !material_options(part_id, &process.method).contains(&material.as_str()) {
                return Err("The selected material is not available for this process.".into());
            }
            process.material = material.clone();
            changed.push(part_id.clone());
        }
        MechanicalChange::SetProcessThickness { part_id, thickness } => {
            let min = if part_id.ends_with("foam") { 0.0 } else { 0.1 };
            if !thickness.is_finite() || *thickness < min {
                return Err(if min == 0.0 {
                    "Foam thickness must be finite and nonnegative.".into()
                } else {
                    "Finished thickness must be at least 0.1 mm.".into()
                });
            }
            if !is_process_target(document, part_id) {
                return Err("The selected Case part is no longer available.".into());
            }
            match part_id.as_str() {
                "plate" => configuration.plate_thickness = *thickness,
                "plate-foam" => configuration.plate_foam_thickness = *thickness,
                "bottom-foam" => configuration.bottom_foam_thickness = *thickness,
                "bottom" => configuration.bottom_thickness = *thickness,
                _ => process_mut(configuration, document, part_id)?.thickness = *thickness,
            }
            changed.push(part_id.clone());
        }
        MechanicalChange::SetStabilizerKind(stabilizer) => {
            if !stabilizer_target_exists(document, board_id, &stabilizer.part_id) {
                return Err("The selected wide key no longer needs a stabilizer.".into());
            }
            let values = configuration.stabilizers.get_or_insert_with(Vec::new);
            upsert_item(values, stabilizer.clone(), |entry| &entry.part_id);
            changed.push(stabilizer.part_id.clone());
        }
        _ => return Err("A change was routed to the wrong edit domain.".into()),
    };
    Ok(())
}
