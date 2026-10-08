//! Defaults for supported mechanical presets.
use boardstudio_core::model::{
    Board, MechanicalConfiguration, MechanicalSwitchFamily, Part, PartDefinition, PlateMethod,
    ProjectDoc,
};

/// Existing configuration keeps all extension settings; a new board uses reference defaults.
///
/// The numeric defaults (plate thickness, mounting datum, plate-foam thickness and
/// the standard layer processes/materials) come from Core's public mechanical
/// helpers, so this adapter only assembles a fresh configuration. Core's canonical
/// full builder, `mechanical::edit::default_configuration`, is private; exposing it
/// would let this function disappear entirely.
pub fn initial_settings(
    document: &ProjectDoc,
    board_id: &str,
) -> Result<MechanicalConfiguration, String> {
    if let Some(config) = document
        .mechanical
        .as_ref()
        .filter(|c| c.board_id == board_id)
    {
        return Ok(config.clone());
    }
    let board = document
        .boards
        .iter()
        .find(|b| b.id == board_id)
        .ok_or("Board is unavailable")?;
    let family = inferred_family(document, board);
    let plate_thickness = boardstudio_core::mechanical::default_plate_thickness(family);
    let plate_to_pcb = boardstudio_core::mechanical::mounting_datum(family) - plate_thickness;
    let plate_foam_thickness =
        boardstudio_core::mechanical::default_plate_foam_thickness(plate_to_pcb);
    let pcb_thickness = if board.thickness.is_finite() && board.thickness > 0.0 {
        board.thickness
    } else {
        1.6
    };
    let bottom_foam_thickness = 2.0;
    let bottom_thickness = 3.0;
    let processes: Vec<_> = [
        ("plate", plate_thickness),
        ("plate-foam", plate_foam_thickness),
        ("bottom-foam", bottom_foam_thickness),
        ("bottom", bottom_thickness),
    ]
    .into_iter()
    .map(|(id, thickness)| {
        boardstudio_core::mechanical::default_layer_process(id, PlateMethod::Printed, thickness)
    })
    .collect();
    serde_json::from_value(serde_json::json!({
        "boardId":board_id,"method":"printed","mount":"tray","integratedPlateFrame":false,
        "bottomStyle":"shell","middleFrame":false,"plateThickness":plate_thickness,"plateFoamThickness":plate_foam_thickness,
        "pcbThickness":pcb_thickness,"bottomFoamThickness":bottom_foam_thickness,"batteryHeight":0.0,"bottomThickness":bottom_thickness,
        "plateToPcb":plate_to_pcb,"wallThickness":2.0,"clearance":0.3,"mounts":[],"partProcesses":processes,"profiles":[]
    }))
    .map_err(|error| error.to_string())
}

/// The board's shared switch family, or MX when the placed switches disagree.
fn inferred_family(document: &ProjectDoc, board: &Board) -> MechanicalSwitchFamily {
    let families: Vec<_> = document
        .parts
        .iter()
        .filter(|part| board.part_ids.contains(&part.id))
        .filter_map(|part| {
            let definition = document
                .definitions
                .iter()
                .find(|definition| definition.id == part.definition_id)?;
            is_switch_position(definition, part).then_some(switch_family(definition, part))
        })
        .collect();
    families
        .first()
        .copied()
        .flatten()
        .filter(|family| families.iter().all(|candidate| *candidate == Some(*family)))
        .unwrap_or(MechanicalSwitchFamily::Mx)
}

fn is_switch_position(definition: &PartDefinition, part: &Part) -> bool {
    matches!(definition.kind, boardstudio_core::model::PartKind::Switch)
        || switch_family(definition, part).is_some()
        || definition
            .generator
            .as_ref()
            .is_some_and(|generator| generator.source.ends_with("/switch_choc_v1_v2"))
}

fn switch_family(definition: &PartDefinition, part: &Part) -> Option<MechanicalSwitchFamily> {
    let generator = definition.generator.as_ref()?;
    let source = generator.source.to_lowercase();
    if source == "ceoloide/switch_mx" {
        return Some(MechanicalSwitchFamily::Mx);
    }
    if !source.ends_with("/switch_choc_v1_v2") {
        return None;
    }
    let enabled = |name: &str| {
        let value = part
            .generator_parameters
            .as_ref()
            .and_then(|p| p.get(name))
            .or_else(|| generator.parameters.get(name));
        value
            .and_then(|v| {
                v.as_bool()
                    .or_else(|| v.get("value").and_then(serde_json::Value::as_bool))
            })
            .unwrap_or(true)
    };
    match (enabled("choc_v1_support"), enabled("choc_v2_support")) {
        (true, false) => Some(MechanicalSwitchFamily::ChocV1),
        (false, true) => Some(MechanicalSwitchFamily::ChocV2),
        _ => None,
    }
}

/// Keep the standard process overrides in sync, matching MechanicalAssemblyPanel.update.
pub fn update_bottom_thickness(
    config: &mut MechanicalConfiguration,
    thickness: f64,
) -> Result<(), String> {
    if !thickness.is_finite() || thickness <= 0.0 {
        return Err("Bottom thickness must be positive.".into());
    }
    config.bottom_thickness = thickness;
    if let Some(processes) = &mut config.part_processes {
        for process in processes.iter_mut().filter(|p| p.part_id == "bottom") {
            process.thickness = thickness;
        }
    }
    Ok(())
}
