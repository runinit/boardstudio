//! Deletion retains disabled slots but removes tombstones from generated geometry.
use super::{layout, member_id, set_matrix};
use crate::model::*;
use std::collections::{BTreeMap, BTreeSet};

type PartChanges = BTreeMap<String, Option<String>>;

fn remapped(id: &str, changes: &PartChanges) -> Option<String> {
    changes.get(id).cloned().unwrap_or_else(|| Some(id.into()))
}
fn remap_ids(ids: &mut Vec<String>, changes: &PartChanges) {
    *ids = ids.iter().filter_map(|id| remapped(id, changes)).collect();
}
fn remap_map<T>(values: &mut BTreeMap<String, T>, changes: &PartChanges) {
    *values = std::mem::take(values)
        .into_iter()
        .filter_map(|(id, value)| remapped(&id, changes).map(|id| (id, value)))
        .collect();
}
fn remap_option(id: &mut Option<String>, changes: &PartChanges) {
    *id = id.as_deref().and_then(|id| remapped(id, changes));
}
fn remap_controls(points: &mut [OutlineControlPoint], changes: &PartChanges, before: &[Part]) {
    for point in points {
        if let Some(id) = &point.part_id
            && changes.get(id) == Some(&None)
            && let Some(part) = before.iter().find(|part| &part.id == id)
        {
            point.at = crate::outline_controls::world(point.at, part);
        }
        remap_option(&mut point.part_id, changes);
    }
}
fn remap_settings(settings: &mut OutlineSettings, changes: &PartChanges, before: &[Part]) {
    for gap in settings
        .repair
        .iter_mut()
        .flat_map(|repair| &mut repair.keep_gaps)
    {
        remap_controls(&mut gap.points, changes, before);
    }
}
fn remap_outline(features: &mut [OutlineFeature], changes: &PartChanges, before: &[Part]) {
    for feature in features {
        match feature {
            OutlineFeature::PartEnvelope {
                part_ids,
                connections,
                settings,
                ..
            } => {
                remap_ids(part_ids, changes);
                for connection in connections {
                    remap_controls(&mut connection.points, changes, before);
                }
                remap_settings(settings, changes, before);
            }
            OutlineFeature::Rect {
                anchor_part_id,
                center,
                rotation,
                ..
            } => {
                if let Some(id) = anchor_part_id.as_ref()
                    && changes.get(id) == Some(&None)
                    && let Some(part) = before.iter().find(|part| &part.id == id)
                {
                    *center = crate::outline_controls::world(*center, part);
                    *rotation = Some(
                        part.pose.rotation
                            + rotation.unwrap_or(0.0)
                                * if part.side == Side::Back { -1.0 } else { 1.0 },
                    );
                }
                remap_option(anchor_part_id, changes);
            }
            OutlineFeature::Polygon {
                anchor_part_id,
                points,
                ..
            } => {
                if let Some(id) = anchor_part_id.as_ref()
                    && changes.get(id) == Some(&None)
                    && let Some(part) = before.iter().find(|part| &part.id == id)
                {
                    for point in points {
                        *point = crate::outline_controls::world(*point, part);
                    }
                }
                remap_option(anchor_part_id, changes);
            }
        }
    }
}

/// Rename retained coordinate-derived identities simultaneously, so a removed
/// slot's old ID can never donate its metadata to the next surviving key.
fn remap_parts(doc: &mut ProjectDoc, changes: &PartChanges) {
    let before = doc.parts.clone();
    doc.parts
        .retain(|part| changes.get(&part.id) != Some(&None));
    crate::outline_controls::detach_removed(&before, doc);
    for part in &mut doc.parts {
        if let Some(Some(id)) = changes.get(&part.id) {
            part.id = id.clone();
        }
        if let Some(properties) = &mut part.properties {
            // Mirror component links refer to generated companions across halves.
            if let Some(serde_json::Value::String(id)) =
                properties.get_mut("boardstudio.mirroredComponentSource")
            {
                if let Some(next) = remapped(id, changes) {
                    *id = next;
                } else {
                    properties.remove("boardstudio.mirroredComponentSource");
                }
            }
        }
    }
    doc.constraints.retain_mut(|constraint| {
        let (source, target) = match constraint {
            Constraint::Offset {
                source_part_id,
                target_part_id,
                ..
            }
            | Constraint::Mirror {
                source_part_id,
                target_part_id,
                ..
            } => (source_part_id, target_part_id),
        };
        let (Some(next_source), Some(next_target)) =
            (remapped(source, changes), remapped(target, changes))
        else {
            return false;
        };
        *source = next_source;
        *target = next_target;
        true
    });
    for net in &mut doc.nets {
        net.pins.retain_mut(|pin| {
            if let Some(id) = remapped(&pin.part_id, changes) {
                pin.part_id = id;
                true
            } else {
                false
            }
        });
    }
    for board in &mut doc.boards {
        remap_ids(&mut board.part_ids, changes);
    }
    for matrix in &mut doc.matrices {
        remap_ids(&mut matrix.part_ids, changes);
    }
    for layout in &mut doc.layouts {
        remap_ids(&mut layout.part_ids, changes);
    }
    remap_outline(&mut doc.outline, changes, &before);
    for state in &mut doc.board_outlines {
        for snapshot in state
            .versions
            .iter_mut()
            .map(|version| &mut version.geometry)
            .chain(state.generated_last_valid.iter_mut())
        {
            remap_outline(&mut snapshot.features, changes, &before);
            remap_settings(&mut snapshot.settings, changes, &before);
            for bridge in &mut snapshot.bridges {
                remap_ids(&mut bridge.part_ids, changes);
            }
            for gap in &mut snapshot.protected_gaps {
                remap_controls(&mut gap.points, changes, &before);
            }
        }
    }
    if let Some(keymap) = &mut doc.keymap {
        for layer in &mut keymap.layers {
            remap_map(&mut layer.bindings, changes);
            remap_map(&mut layer.sensors, changes);
        }
    }
    if let Some(keycaps) = &mut doc.keycaps {
        remap_map(&mut keycaps.keys, changes);
    }
    if let Some(hardware) = &mut doc.hardware {
        let mut functions = changes.clone();
        for (old, new) in changes {
            for prefix in ["direct/", "link/"] {
                functions.insert(
                    format!("{prefix}{old}"),
                    new.as_ref().map(|id| format!("{prefix}{id}")),
                );
            }
        }
        for instance in &mut hardware.instances {
            remap_option(&mut instance.controller_part_id, changes);
        }
        for board in &mut hardware.boards {
            remap_option(&mut board.controller_part_id, changes);
            remap_map(&mut board.locks, &functions);
            remap_map(&mut board.assignments, &functions);
            remap_map(&mut board.key_bindings, changes);
            // Fabricated handoff baselines stay immutable; Core reports changes against them.
        }
    }
    for module in &mut doc.modules {
        if let Some(connection) = &mut module.connection {
            if let Some(id) = remapped(&connection.host_connector_part_id, changes) {
                connection.host_connector_part_id = id;
            } else {
                module.connection = None;
            }
        }
    }
    for circuit in &mut doc.embedded_circuits {
        remap_ids(&mut circuit.part_ids, changes);
    }
}

fn matrices(doc: &ProjectDoc, matrix_id: &str) -> Result<Vec<Matrix>, String> {
    let current = doc
        .matrices
        .iter()
        .find(|matrix| matrix.id == matrix_id)
        .ok_or("Matrix does not exist")?;
    let mut result = vec![current.clone()];
    if let Some((partner, _)) = layout::partner(doc, matrix_id) {
        result.push(
            doc.matrices
                .iter()
                .find(|matrix| matrix.id == partner)
                .ok_or("Linked matrix is missing")?
                .clone(),
        );
    }
    Ok(result)
}
fn cell_changes(
    matrix: &Matrix,
    row: u32,
    column: u32,
    next: Option<(u32, u32)>,
    changes: &mut PartChanges,
) {
    let old = member_id(&matrix.id, row, column);
    let new = next.map(|(row, column)| member_id(&matrix.id, row, column));
    changes.insert(old.clone(), new.clone());
    let prefix = format!("{old}/");
    for id in &matrix.part_ids {
        if let Some(suffix) = id.strip_prefix(&prefix) {
            changes.insert(
                id.clone(),
                new.as_ref().map(|next| format!("{next}/{suffix}")),
            );
        }
    }
}
fn changed_ids(changes: &PartChanges) -> Vec<String> {
    changes
        .keys()
        .cloned()
        .chain(changes.values().flatten().cloned())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

pub(crate) fn remove_cells(
    doc: &mut ProjectDoc,
    matrix_id: &str,
    cells: &[(u32, u32)],
) -> Result<Vec<String>, String> {
    let mut matrices = matrices(doc, matrix_id)?;
    if cells
        .iter()
        .any(|(row, column)| *row >= matrices[0].rows || *column >= matrices[0].columns)
    {
        return Err("Matrix cell coordinate is invalid".into());
    }
    let cells: BTreeSet<_> = cells.iter().copied().collect();
    let mut changes = PartChanges::new();
    for matrix in &mut matrices {
        for &(row, column) in &cells {
            cell_changes(matrix, row, column, None, &mut changes);
            if let Some(cell) = matrix
                .cells
                .iter_mut()
                .find(|cell| cell.row == row && cell.column == column)
            {
                cell.deleted = true;
                cell.enabled = false;
                cell.assemblies.clear();
            } else {
                matrix.cells.push(MatrixCell {
                    deleted: true,
                    row,
                    column,
                    enabled: false,
                    definition_id: None,
                    variant: None,
                    offset: None,
                    rotation: None,
                    assemblies: vec![],
                    assemblies_local: None,
                });
            }
        }
    }
    remap_parts(doc, &changes);
    let mut changed = changed_ids(&changes);
    for matrix in matrices {
        changed.extend(set_matrix(doc, &matrix)?);
    }
    changed.extend(layout::sync(doc, matrix_id)?);
    Ok(changed)
}

fn remove_index<T>(values: &mut Vec<T>, index: u32) {
    if (index as usize) < values.len() {
        values.remove(index as usize);
    }
}

pub(crate) fn remove_axis(
    doc: &mut ProjectDoc,
    matrix_id: &str,
    index: u32,
    row_axis: bool,
) -> Result<Vec<String>, String> {
    let mut matrices = matrices(doc, matrix_id)?;
    let dimension = if row_axis {
        matrices[0].rows
    } else {
        matrices[0].columns
    };
    if index >= dimension {
        return Err("Matrix axis index is invalid".into());
    }
    if dimension == 1 {
        let mut changes = PartChanges::new();
        for matrix in &matrices {
            for row in 0..matrix.rows {
                for column in 0..matrix.columns {
                    cell_changes(matrix, row, column, None, &mut changes);
                }
            }
        }
        remap_parts(doc, &changes);
        let mut changed = changed_ids(&changes);
        for matrix in matrices {
            changed.extend(crate::apply(
                doc,
                &EditOperation::RemoveMatrix { id: matrix.id },
            )?);
        }
        return Ok(changed);
    }
    let mut changes = PartChanges::new();
    for matrix in &mut matrices {
        for row in 0..matrix.rows {
            for column in 0..matrix.columns {
                let axis = if row_axis { row } else { column };
                if axis < index {
                    continue;
                }
                let next = (axis != index).then(|| {
                    if row_axis {
                        (row - 1, column)
                    } else {
                        (row, column - 1)
                    }
                });
                cell_changes(matrix, row, column, next, &mut changes);
            }
        }
        matrix.cells.retain_mut(|cell| {
            let axis = if row_axis {
                &mut cell.row
            } else {
                &mut cell.column
            };
            if *axis == index {
                return false;
            }
            if *axis > index {
                *axis -= 1;
            }
            true
        });
        if row_axis {
            matrix.rows -= 1;
            remove_index(&mut matrix.row_offsets, index);
        } else {
            matrix.columns -= 1;
            remove_index(&mut matrix.column_offsets, index);
            remove_index(&mut matrix.column_staggers, index);
            remove_index(&mut matrix.column_splays, index);
            remove_index(&mut matrix.column_origins, index);
        }
        remap_ids(&mut matrix.part_ids, &changes);
    }
    remap_parts(doc, &changes);
    let mut changed = changed_ids(&changes);
    for matrix in matrices {
        changed.extend(set_matrix(doc, &matrix)?);
    }
    changed.extend(layout::sync(doc, matrix_id)?);
    Ok(changed)
}

/// Resolve linked halves to one geometry source and union their coordinates, so
/// a grouped selection cannot execute the same mirrored action twice.
pub(crate) fn edit_selected_cells(
    doc: &mut ProjectDoc,
    selected: &BTreeMap<String, Vec<(u32, u32)>>,
    enabled: Option<bool>,
) -> Result<Vec<String>, String> {
    let mut groups: BTreeMap<String, BTreeSet<(u32, u32)>> = BTreeMap::new();
    for (matrix_id, cells) in selected {
        let matrix = doc
            .matrices
            .iter()
            .find(|matrix| &matrix.id == matrix_id)
            .ok_or("Matrix does not exist")?;
        for &(row, column) in cells {
            if row >= matrix.rows || column >= matrix.columns {
                return Err("Matrix cell coordinate is invalid".into());
            }
            if matrix
                .cells
                .iter()
                .any(|cell| cell.row == row && cell.column == column && cell.deleted)
            {
                return Err("Matrix cell was deleted".into());
            }
        }
        let canonical = layout::partner(doc, matrix_id).map_or_else(
            || matrix_id.clone(),
            |(partner, _)| partner.min(matrix_id.clone()),
        );
        groups
            .entry(canonical)
            .or_default()
            .extend(cells.iter().copied());
    }
    let mut changed = vec![];
    for (matrix_id, cells) in groups {
        if let Some(enabled) = enabled {
            let mut matrix = doc
                .matrices
                .iter()
                .find(|matrix| matrix.id == matrix_id)
                .ok_or("Matrix does not exist")?
                .clone();
            for (row, column) in cells {
                if let Some(cell) = matrix
                    .cells
                    .iter_mut()
                    .find(|cell| cell.row == row && cell.column == column)
                {
                    cell.enabled = enabled;
                } else {
                    matrix.cells.push(MatrixCell {
                        deleted: false,
                        row,
                        column,
                        enabled,
                        definition_id: None,
                        variant: None,
                        offset: None,
                        rotation: None,
                        assemblies: vec![],
                        assemblies_local: None,
                    });
                }
            }
            changed.extend(set_matrix(doc, &matrix)?);
            changed.extend(layout::sync(doc, &matrix_id)?);
        } else {
            changed.extend(remove_cells(
                doc,
                &matrix_id,
                &cells.into_iter().collect::<Vec<_>>(),
            )?);
        }
    }
    Ok(changed)
}
