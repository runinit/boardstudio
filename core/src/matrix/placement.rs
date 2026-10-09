//! Named matrix placement parameters. `SetMatrixPlacement` routes here so Core
//! owns the index and value rules; a gesture and an Inspector field reach the same
//! matrix for a given value.
use super::{OVERRIDE, member_id, valid_projection_matrix};
use crate::model::{
    Matrix, MatrixPlacementChange, Part, ProjectDoc, Vec2, convert_offset_available,
};
use std::collections::BTreeMap;

fn grow<T: Clone + Default>(values: &mut Vec<T>, index: usize) {
    if values.len() <= index {
        values.resize(index + 1, T::default());
    }
}

fn finite(value: Vec2) -> bool {
    value.x.is_finite() && value.y.is_finite()
}

/// Apply one named placement change, changing only the entry it names. Existing
/// vectors are padded with zeros so an index beyond the stored length is valid.
pub(crate) fn update(matrix: &Matrix, change: &MatrixPlacementChange) -> Result<Matrix, String> {
    valid_projection_matrix(matrix)?;
    let mut after = matrix.clone();
    match change {
        MatrixPlacementChange::ColumnStagger { column, value } => {
            if *column >= matrix.columns {
                return Err("Matrix placement column is out of bounds".into());
            }
            if !value.is_finite() {
                return Err("Matrix placement value is not finite".into());
            }
            if after
                .column_staggers
                .get(*column as usize)
                .copied()
                .unwrap_or(0.0)
                == *value
            {
                return Ok(matrix.clone());
            }
            grow(&mut after.column_staggers, *column as usize);
            after.column_staggers[*column as usize] = *value;
        }
        MatrixPlacementChange::ColumnOffset { column, value } => {
            if *column >= matrix.columns {
                return Err("Matrix placement column is out of bounds".into());
            }
            if !finite(*value) {
                return Err("Matrix placement value is not finite".into());
            }
            if after
                .column_offsets
                .get(*column as usize)
                .copied()
                .unwrap_or_default()
                == *value
            {
                return Ok(matrix.clone());
            }
            grow(&mut after.column_offsets, *column as usize);
            after.column_offsets[*column as usize] = *value;
        }
        MatrixPlacementChange::RowOffset { row, value } => {
            if *row >= matrix.rows {
                return Err("Matrix placement row is out of bounds".into());
            }
            if !finite(*value) {
                return Err("Matrix placement value is not finite".into());
            }
            if after
                .row_offsets
                .get(*row as usize)
                .copied()
                .unwrap_or_default()
                == *value
            {
                return Ok(matrix.clone());
            }
            grow(&mut after.row_offsets, *row as usize);
            after.row_offsets[*row as usize] = *value;
        }
        MatrixPlacementChange::ColumnOffsetToStagger { column } => {
            if *column >= matrix.columns {
                return Err("Matrix placement column is out of bounds".into());
            }
            let index = *column as usize;
            let amount = after
                .column_offsets
                .get(index)
                .map_or(0.0, |offset| offset.y);
            // Nothing to convert is a valid no-op, whatever the splay.
            if amount == 0.0 {
                return Ok(matrix.clone());
            }
            convert_offset_available(matrix, *column)?;
            grow(&mut after.column_staggers, index);
            after.column_staggers[index] += amount;
            if *column + 1 < matrix.columns {
                grow(&mut after.column_staggers, index + 1);
                after.column_staggers[index + 1] -= amount;
            }
            after.column_offsets[index].y = 0.0;
        }
    }
    Ok(after)
}

/// Shift authored companions by the world delta of their key, so a placement
/// change keeps the relative pose the user authored. Companions with an explicit
/// constraint keep that constraint instead: the constraint solver settles them.
pub(crate) fn carry_authored_companions(
    doc: &mut ProjectDoc,
    before: &BTreeMap<String, Part>,
) -> Vec<String> {
    let mut shifts: Vec<(String, Vec2)> = Vec::new();
    for matrix in &doc.matrices {
        for cell in &matrix.cells {
            if !cell.enabled || cell.deleted {
                continue;
            }
            let key_id = member_id(&matrix.id, cell.row, cell.column);
            let Some(previous) = before.get(&key_id) else {
                continue;
            };
            let Some(key) = doc.parts.iter().find(|part| part.id == key_id) else {
                continue;
            };
            let delta = Vec2 {
                x: key.pose.at.x - previous.pose.at.x,
                y: key.pose.at.y - previous.pose.at.y,
            };
            if delta.x == 0.0 && delta.y == 0.0 {
                continue;
            }
            for assembly in &cell.assemblies {
                let id = format!("{key_id}/{}", assembly.id);
                if doc
                    .constraints
                    .iter()
                    .any(|constraint| constraint.target() == id)
                {
                    continue;
                }
                let authored = doc
                    .parts
                    .iter()
                    .find(|part| part.id == id)
                    .is_some_and(|part| {
                        part.properties.as_ref().is_some_and(|properties| {
                            properties.get(OVERRIDE) == Some(&serde_json::json!(1))
                        })
                    });
                if authored {
                    shifts.push((id, delta));
                }
            }
        }
    }
    let mut changed = Vec::new();
    for (id, delta) in shifts {
        if let Some(part) = doc.parts.iter_mut().find(|part| part.id == id) {
            part.pose.at.x += delta.x;
            part.pose.at.y += delta.y;
            changed.push(id);
        }
    }
    changed
}
