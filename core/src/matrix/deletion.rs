//! Deletion retains disabled slots but removes tombstones from generated geometry.
use super::{layout, member_id, set_matrix};
use crate::model::*;
use std::collections::{BTreeMap, BTreeSet};

type PartChanges = BTreeMap<String, Option<String>>;

fn remapped(id: &str, changes: &PartChanges) -> Option<String> {
    changes.get(id).cloned().unwrap_or_else(|| Some(id.into()))
}
fn remap_constraint_endpoint(
    endpoint: &mut ConstraintEndpoint,
    changes: &PartChanges,
) -> Option<()> {
    match endpoint {
        ConstraintEndpoint::Part { part_id } => {
            *part_id = remapped(part_id, changes)?;
        }
        ConstraintEndpoint::LinkedMirror {
            source_part_id,
            mirror_part_id,
            ..
        } => {
            // Deletion follows the authored visible identity. A removed authority alone does
            // not discard a record whose visible endpoint still exists.
            *mirror_part_id = remapped(mirror_part_id, changes)?;
            if let Some(next_source) = remapped(source_part_id, changes) {
                *source_part_id = next_source;
            }
        }
    }
    Some(())
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
            // Matrix members and companions carry world poses through removal.
            point.at = crate::outline_controls::world(point.at, &part.pose, part.side.clone());
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
                    // Matrix members and companions carry world poses.
                    *center =
                        crate::outline_controls::world(*center, &part.pose, part.side.clone());
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
                    // Matrix members and companions carry world poses.
                    for point in points {
                        *point =
                            crate::outline_controls::world(*point, &part.pose, part.side.clone());
                    }
                }
                remap_option(anchor_part_id, changes);
            }
        }
    }
}

/// Rename retained coordinate-derived identities simultaneously, so a removed
/// slot's old ID can never donate its metadata to the next surviving key.
///
/// A linked outline version drops the authored choices that required a removed key, in
/// this same edit, instead of detaching them to world points; the ids it dropped return.
fn remap_parts(doc: &mut ProjectDoc, changes: &PartChanges) -> Vec<String> {
    let before = doc.parts.clone();
    doc.parts
        .retain(|part| changes.get(&part.id) != Some(&None));
    crate::outline_controls::detach_removed(&before, doc);
    let removed: Vec<String> = changes
        .iter()
        .filter(|(_, next)| next.is_none())
        .map(|(id, _)| id.clone())
        .collect();
    let pruned = crate::outline_versions::prune_removed_sources(doc, &removed);
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
    doc.constraints.retain_mut(|constraint| match constraint {
        Constraint::Offset {
            source_part_id,
            target_part_id,
            ..
        }
        | Constraint::PartOffset {
            source_part_id,
            target_part_id,
            ..
        }
        | Constraint::Mirror {
            source_part_id,
            target_part_id,
            ..
        } => {
            let (Some(next_source), Some(next_target)) = (
                remapped(source_part_id, changes),
                remapped(target_part_id, changes),
            ) else {
                return false;
            };
            *source_part_id = next_source;
            *target_part_id = next_target;
            true
        }
        Constraint::Framed { source, target, .. } => {
            remap_constraint_endpoint(source, changes).is_some()
                && remap_constraint_endpoint(target, changes).is_some()
        }
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
        if let Some(link) = &mut layout.mirror_link {
            link.correspondences.retain_mut(|record| {
                if record.kind != MirroredItemKind::Part {
                    return true;
                }
                let (Some(source), Some(mirror)) = (
                    remapped(&record.source_id, changes),
                    remapped(&record.mirror_id, changes),
                ) else {
                    return false;
                };
                record.source_id = source;
                record.mirror_id = mirror;
                true
            });
            link.exceptions.retain_mut(|exception| {
                if exception.kind != MirroredItemKind::Part {
                    return true;
                }
                let Some(id) = remapped(&exception.mirror_id, changes) else {
                    return false;
                };
                exception.mirror_id = id;
                true
            });
        }
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
    pruned
}

fn matrices(doc: &ProjectDoc, matrix_id: &str) -> Result<Vec<Matrix>, String> {
    let current = doc
        .matrices
        .iter()
        .find(|matrix| matrix.id == matrix_id)
        .ok_or("Matrix does not exist")?;
    let mut result = vec![current.clone()];
    // Ticket 21: an independent (Local) or absent (Omitted) matrix edits
    // alone; its partner follows through `sync` (live for Omitted, skipped
    // for Local) instead of joining the grouped storage edit.
    if layout::edits_alone(doc, matrix_id) {
        return Ok(result);
    }
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

/// Remove exactly the supplied part identities and remap their project dependencies.
pub(crate) fn remove_parts_by_id(
    doc: &mut ProjectDoc,
    removed_ids: &BTreeSet<String>,
) -> Vec<String> {
    let changes: PartChanges = removed_ids.iter().cloned().map(|id| (id, None)).collect();
    remap_parts(doc, &changes)
}
pub(crate) fn remove_cells(
    doc: &mut ProjectDoc,
    matrix_id: &str,
    cells: &[(u32, u32)],
) -> Result<Vec<String>, String> {
    layout::refuse_omitted_storage_edit(doc, matrix_id, cells)?;
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
                    shift: None,
                    bend: None,
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
    let pruned = remap_parts(doc, &changes);
    let mut changed = changed_ids(&changes);
    changed.extend(pruned);
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
    // Ticket 21: an absent matrix has no independent storage state; its
    // record follows the source. Omitted keys ride structural compaction.
    layout::refuse_omitted_storage_edit(doc, matrix_id, &[])?;
    if dimension == 1 {
        let mut changes = PartChanges::new();
        for matrix in &matrices {
            for row in 0..matrix.rows {
                for column in 0..matrix.columns {
                    cell_changes(matrix, row, column, None, &mut changes);
                }
            }
        }
        let pruned = remap_parts(doc, &changes);
        let mut changed = changed_ids(&changes);
        changed.extend(pruned);
        for matrix in matrices {
            changed.extend(crate::apply(
                doc,
                &[],
                &EditOperation::RemoveMatrix { id: matrix.id },
            )?);
        }
        return Ok(changed);
    }
    let mut changes = PartChanges::new();
    // Anchors at the removed slot detach with their frame while the anchor key
    // still resolves; later anchors shift after the axis removal is stored.
    let compacted: Vec<String> = matrices.iter().map(|matrix| matrix.id.clone()).collect();
    let mut changed = super::anchors::remap_compacted(doc, &compacted, index, row_axis)?;
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
            remove_index(&mut matrix.row_layout, index);
            if let Some(first) = matrix.row_layout.first_mut() {
                first.padding = None;
            }
        } else {
            matrix.columns -= 1;
            remove_index(&mut matrix.column_layout, index);
            if let Some(first) = matrix.column_layout.first_mut() {
                first.spread = None;
            }
        }
        remap_ids(&mut matrix.part_ids, &changes);
    }
    let pruned = remap_parts(doc, &changes);
    changed.extend(changed_ids(&changes));
    changed.extend(pruned);
    for matrix in matrices {
        changed.extend(set_matrix(doc, &matrix)?);
    }
    changed.extend(layout::sync(doc, matrix_id)?);
    changed.extend(super::anchors::shift_compacted(
        doc, &compacted, index, row_axis,
    )?);
    Ok(changed)
}

/// Resolve linked halves to one geometry source and union their coordinates, so
/// a grouped selection cannot execute the same mirrored action twice.
pub(crate) fn edit_selected_cells(
    doc: &mut ProjectDoc,
    selected: &BTreeMap<String, Vec<(u32, u32)>>,
    enabled: Option<bool>,
) -> Result<Vec<String>, String> {
    // Ticket 21: refuse absent targets before grouping merges halves.
    for (matrix_id, cells) in selected {
        layout::refuse_omitted_storage_edit(doc, matrix_id, cells)?;
    }
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
        // Ticket 21: an independent or absent matrix is its own group;
        // grouped selections only merge cleanly linked halves.
        let canonical = if layout::edits_alone(doc, matrix_id) {
            matrix_id.clone()
        } else {
            layout::partner(doc, matrix_id).map_or_else(
                || matrix_id.clone(),
                |(partner, _)| partner.min(matrix_id.clone()),
            )
        };
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
                        shift: None,
                        bend: None,
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

#[cfg(test)]
mod framed_constraint_deletion_tests {
    use super::*;

    fn part(id: &str) -> Part {
        Part {
            anchor: None,
            id: id.into(),
            definition_id: "key".into(),
            reference: id.into(),
            pose: Pose2 {
                at: Vec2 { x: 0.0, y: 0.0 },
                rotation: 0.0,
            },
            side: Side::Front,
            keycap: None,
            outline: None,
            locked: None,
            properties: None,
            generator_parameters: None,
        }
    }

    #[test]
    fn framed_constraint_survives_authority_removal_until_its_visible_endpoint_is_removed() {
        let mut doc = ProjectDoc::empty("doc", "Doc");
        doc.parts = vec![part("source"), part("mirror"), part("local")];
        doc.constraints.push(Constraint::Framed {
            id: "c".into(),
            source: ConstraintEndpoint::LinkedMirror {
                pair_id: "pair".into(),
                source_part_id: "source".into(),
                mirror_part_id: "mirror".into(),
            },
            target: ConstraintEndpoint::Part {
                part_id: "local".into(),
            },
            relation: ConstraintRelation::Offset {
                offset: Vec2 { x: 1.0, y: 2.0 },
                rotation: 0.0,
            },
        });

        remap_parts(&mut doc, &BTreeMap::from([("source".into(), None)]));
        assert_eq!(doc.constraints.len(), 1);
        assert_eq!(doc.constraints[0].source(), "mirror");

        remap_parts(&mut doc, &BTreeMap::from([("mirror".into(), None)]));
        assert!(doc.constraints.is_empty());
    }
}
