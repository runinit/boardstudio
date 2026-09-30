use super::color_valid;
use crate::model::*;

/// Apply field edits to the current document, preserving unrelated configuration.
pub(crate) fn apply_edit(doc: &mut ProjectDoc, op: &EditOperation) -> Result<Vec<String>, String> {
    match op {
        EditOperation::SetKeyBinding {
            board_id,
            key_id,
            binding,
        } => {
            let board = doc
                .boards
                .iter()
                .find(|board| board.id == *board_id)
                .ok_or_else(|| format!("Unknown board {board_id}"))?;
            let part_id = key_id.strip_suffix("/push").unwrap_or(key_id);
            if !board.part_ids.contains(&part_id.to_string())
                || !doc.parts.iter().any(|part| part.id == part_id)
            {
                return Err(format!("Key {key_id} does not belong to board {board_id}"));
            }
            let hardware = doc.hardware.get_or_insert_with(Default::default);
            let index = hardware
                .boards
                .iter()
                .position(|board| board.board_id == *board_id);
            let board = match index {
                Some(index) => &mut hardware.boards[index],
                None => {
                    hardware.boards.push(ElectricalBoardConfiguration {
                        board_id: board_id.clone(),
                        ..Default::default()
                    });
                    hardware.boards.last_mut().unwrap()
                }
            };
            board.key_bindings.insert(key_id.clone(), binding.clone());
            Ok(vec![board_id.clone(), key_id.clone()])
        }
        EditOperation::SetKeycapBoard { board_id, change } => {
            if !doc.boards.iter().any(|board| board.id == *board_id) {
                return Err(format!("Unknown board {board_id}"));
            }
            match change {
                KeycapBoardChange::Color { value } | KeycapBoardChange::LegendColor { value }
                    if !color_valid(value) =>
                {
                    return Err("Keycap colours require #RRGGBB".into());
                }
                KeycapBoardChange::Clearance { value }
                    if !value.is_finite() || !(0.0..=5.0).contains(value) =>
                {
                    return Err("Keycap clearance must be between 0 and 5 mm".into());
                }
                _ => {}
            }
            let settings = doc
                .keycaps
                .get_or_insert_with(Default::default)
                .boards
                .entry(board_id.clone())
                .or_default();
            match change {
                KeycapBoardChange::Color { value } => settings.color = value.clone(),
                KeycapBoardChange::LegendColor { value } => settings.legend_color = value.clone(),
                KeycapBoardChange::Clearance { value } => settings.clearance = *value,
            }
            Ok(vec![board_id.clone()])
        }
        EditOperation::SetMatrixKeycaps { matrix_id, change } => {
            let matrix = doc
                .matrices
                .iter()
                .find(|matrix| matrix.id == *matrix_id)
                .ok_or_else(|| format!("Unknown matrix {matrix_id}"))?;
            let mut ids = matrix.part_ids.clone();
            ids.push(matrix_id.clone());
            match change {
                KeycapMatrixChange::FirstRow { value } if !(1..=5).contains(value) => {
                    return Err("Keycap row must be between 1 and 5".into());
                }
                KeycapMatrixChange::WallThickness { value }
                    if !value.is_finite() || !(0.8..=2.0).contains(value) =>
                {
                    return Err("Keycap wall must be between 0.8 and 2 mm".into());
                }
                _ => {}
            }
            let settings = doc
                .keycaps
                .get_or_insert_with(Default::default)
                .matrices
                .entry(matrix_id.clone())
                .or_default();
            match change {
                KeycapMatrixChange::Profile { value } => settings.profile = *value,
                KeycapMatrixChange::Mount { value } => settings.mount = *value,
                KeycapMatrixChange::FirstRow { value } => settings.first_row = *value,
                KeycapMatrixChange::WallThickness { value } => settings.wall_thickness = *value,
            }
            Ok(ids)
        }
        EditOperation::SetKeycapKey { key_id, change } => {
            if !doc.parts.iter().any(|part| part.id == *key_id) {
                return Err(format!("Unknown key {key_id}"));
            }
            match change {
                KeycapKeyChange::Color { value: Some(value) } if !color_valid(value) => {
                    return Err("Keycap colours require #RRGGBB".into());
                }
                KeycapKeyChange::Legend { value: Some(value) }
                    if value.chars().count() > 12 || value.chars().any(char::is_control) =>
                {
                    return Err("Keycap legend allows at most 12 printable characters".into());
                }
                KeycapKeyChange::Row { value: Some(value) } if !(1..=5).contains(value) => {
                    return Err("Keycap row must be between 1 and 5".into());
                }
                KeycapKeyChange::Units { value: Some(value) }
                    if !value.x.is_finite()
                        || !value.y.is_finite()
                        || !(0.75..=7.0).contains(&value.x)
                        || !(0.75..=7.0).contains(&value.y) =>
                {
                    return Err("Keycap dimensions must be between 0.75 and 7 units".into());
                }
                _ => {}
            }
            let settings = doc
                .keycaps
                .get_or_insert_with(Default::default)
                .keys
                .entry(key_id.clone())
                .or_default();
            match change {
                KeycapKeyChange::Profile { value } => settings.profile = *value,
                KeycapKeyChange::Mount { value } => settings.mount = *value,
                KeycapKeyChange::Color { value } => settings.color = value.clone(),
                KeycapKeyChange::Legend { value } => settings.legend = value.clone(),
                KeycapKeyChange::Row { value } => settings.row = *value,
                KeycapKeyChange::Units { value } => settings.units = *value,
            }
            Ok(vec![key_id.clone()])
        }
        _ => unreachable!("only keymap edits enter this module"),
    }
}
