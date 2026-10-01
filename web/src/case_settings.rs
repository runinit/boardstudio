//! Application presets ported from app/src/mechanicalPresets.ts.
use boardstudio_core::model::{MechanicalConfiguration, Part, PartDefinition, ProjectDoc};

/// Existing configuration keeps all extension settings; a new board uses reference defaults.
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
    let families: Vec<_> = document
        .parts
        .iter()
        .filter(|p| board.part_ids.contains(&p.id))
        .filter_map(|part| {
            let definition = document
                .definitions
                .iter()
                .find(|d| d.id == part.definition_id)?;
            let family = switch_family(definition, part);
            if definition.kind == boardstudio_core::model::PartKind::Switch
                || family.is_some()
                || definition
                    .generator
                    .as_ref()
                    .is_some_and(|g| g.source.ends_with("/switch_choc_v1_v2"))
            {
                Some(family)
            } else {
                None
            }
        })
        .collect();
    let family = families
        .first()
        .copied()
        .flatten()
        .filter(|family| families.iter().all(|candidate| *candidate == Some(*family)))
        .unwrap_or("mx");
    let plate: f64 = if family == "choc-v1" { 1.3 } else { 1.5 };
    let gap: f64 = if family == "choc-v1" { 3.5 } else { 5.0 } - plate;
    let foam = ((gap - 0.2).min(3.0) * 10.0 + 0.00001_f64).floor().max(0.0) / 10.0;
    let pcb = if board.thickness.is_finite() && board.thickness > 0.0 {
        board.thickness
    } else {
        1.6
    };
    let processes: Vec<_> = [("plate",plate),("plate-foam",foam),("bottom-foam",2.0),("bottom",3.0)].into_iter().map(|(id,thickness)| {
        let foam = id.ends_with("foam");
        serde_json::json!({"partId":id,"method":if foam {"cut-sheet"} else {"printed"},"material":if foam {"EVA"} else {"PLA"},"thickness":thickness,"constraintsVersion":"2026-09-24"})
    }).collect();
    serde_json::from_value(serde_json::json!({
        "boardId":board_id,"method":"printed","mount":"tray","integratedPlateFrame":false,
        "bottomStyle":"shell","middleFrame":false,"plateThickness":plate,"plateFoamThickness":foam,
        "pcbThickness":pcb,"bottomFoamThickness":2.0,"batteryHeight":0.0,"bottomThickness":3.0,
        "plateToPcb":gap,"wallThickness":2.0,"clearance":0.3,"mounts":[],"partProcesses":processes,"profiles":[]
    })).map_err(|error| error.to_string())
}

fn switch_family(definition: &PartDefinition, part: &Part) -> Option<&'static str> {
    let generator = definition.generator.as_ref()?;
    let source = generator.source.to_lowercase();
    if source == "ceoloide/switch_mx" {
        return Some("mx");
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
        (true, false) => Some("choc-v1"),
        (false, true) => Some("choc-v2"),
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
