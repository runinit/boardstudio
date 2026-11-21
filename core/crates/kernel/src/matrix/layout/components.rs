//! Linked component sync, moves and removal.

use super::{
    MIRRORED_COMPONENT_SOURCE, MIRRORED_COMPONENT_SUBSTITUTE, adopt_unrecorded_mirrors,
    ensure_part_mirror, evaluated_part_pose, is_member_or_companion, layout_half_offset,
    materialize_unrecorded_partners, retire_part_correspondences, stored_part_pose,
};
use crate::{matrix, model::*};
use boardstudio_model::mechanical::pair;
use std::collections::{BTreeMap, BTreeSet};

/// Shared orphan pass for both sync paths: a mirror copy whose source left
/// the layout keeps its pose but loses the link (a graceful unmirror when a
/// part moves layouts), while a copy whose source is gone is removed with its
/// record. Ticket 12: either outcome retires Part correspondences naming the
/// copy, so no dangling record survives.
pub(super) fn cleanup_orphaned_components(
    doc: &mut ProjectDoc,
    source_layout_id: &str,
    target_layout_id: &str,
) -> Result<Vec<String>, String> {
    let source_ids: BTreeSet<String> = doc
        .layouts
        .iter()
        .find(|layout| layout.id == source_layout_id)
        .map(|layout| layout.part_ids.iter().cloned().collect())
        .unwrap_or_default();
    let target_layout_parts: Vec<String> = doc
        .layouts
        .iter()
        .find(|layout| layout.id == target_layout_id)
        .map(|layout| layout.part_ids.clone())
        .unwrap_or_default();
    let orphaned: Vec<_> = target_layout_parts
        .iter()
        .filter_map(|target_id| {
            let target = doc.parts.iter().find(|part| &part.id == target_id)?;
            let source_id = target
                .properties
                .as_ref()?
                .get(MIRRORED_COMPONENT_SOURCE)?
                .as_str()?;
            (!source_ids.contains(source_id)).then(|| {
                (
                    target_id.clone(),
                    source_id.to_string(),
                    doc.parts.iter().any(|part| part.id == source_id),
                )
            })
        })
        .collect();
    let mut changed = Vec::new();
    for (target_id, source_id, source_exists) in orphaned {
        retire_part_correspondences(doc, std::slice::from_ref(&target_id));
        if source_exists {
            if let Some(part) = doc.parts.iter_mut().find(|part| part.id == target_id) {
                clear_component_link(part);
                changed.push(target_id);
            }
        } else {
            remove_component(doc, &target_id);
            changed.extend([target_id, source_id]);
        }
    }
    Ok(changed)
}

pub fn sync_components(doc: &mut ProjectDoc) -> Result<Vec<String>, String> {
    let pairs: Vec<_> = doc
        .layouts
        .iter()
        .filter_map(|target| {
            let link = target.mirror_link.as_ref()?;
            let source = doc
                .layouts
                .iter()
                .find(|layout| layout.id == link.source_id)?;
            Some((source.id.clone(), target.id.clone(), link.axis_x))
        })
        .collect();
    let mut changed = Vec::new();
    for (source_layout_id, target_layout_id, axis_x) in pairs {
        adopt_unassigned_components(doc, &source_layout_id, axis_x, &mut changed)?;
        let gapped = doc.layouts.iter().any(|layout| {
            layout.id == target_layout_id
                && layout
                    .mirror_link
                    .as_ref()
                    .is_some_and(|link| link.split_gap.or(link.spacing).is_some())
        });
        if gapped {
            // Ticket 12: stored-gap pairs mirror by Part correspondence in the
            // half frame; local parts (no record) never sync. Missing partners
            // materialize from the source copy, so undo and replacement
            // converge on the reflected pose. Legacy property-linked copies
            // join the correspondence here; entry points already store them
            // half-frame.
            changed.extend(cleanup_orphaned_components(
                doc,
                &source_layout_id,
                &target_layout_id,
            )?);
            changed.extend(adopt_unrecorded_mirrors(
                doc,
                &source_layout_id,
                &target_layout_id,
            )?);
            // Ticket 12: independent source parts mirror by default: record
            // the missing correspondences so the ensure pass below realizes
            // them. Local pins (substitute flag) and recorded pairs stay out.
            changed.extend(materialize_unrecorded_partners(
                doc,
                &source_layout_id,
                &target_layout_id,
            )?);
            // Ticket 12: matrix members and companions stay owned by the
            // matrix sync (slot reflection + refresh), never by the part
            // correspondence sync: their stored poses are board-space, not
            // half-frame.
            let records: Vec<MirroredCorrespondence> = doc
                .layouts
                .iter()
                .find(|layout| layout.id == target_layout_id)
                .and_then(|layout| layout.mirror_link.as_ref())
                .map(|link| {
                    link.correspondences
                        .iter()
                        .filter(|record| {
                            record.kind == MirroredItemKind::Part
                                && !is_member_or_companion(doc, &record.source_id)
                                && !is_member_or_companion(doc, &record.mirror_id)
                        })
                        .cloned()
                        .collect()
                })
                .unwrap_or_default();
            for record in records {
                // Stale records (authority removed by a cell edit the
                // membership sync has not retired yet) never fail the edit;
                // the membership sync owns member record lifecycle.
                if doc.parts.iter().any(|part| part.id == record.source_id) {
                    changed.extend(ensure_part_mirror(doc, &record.source_id)?);
                }
            }
            continue;
        }
        let source_ids = doc
            .layouts
            .iter()
            .find(|layout| layout.id == source_layout_id)
            .ok_or("Linked component source layout is missing")?
            .part_ids
            .clone();
        changed.extend(cleanup_orphaned_components(
            doc,
            &source_layout_id,
            &target_layout_id,
        )?);

        for source_id in source_ids {
            let Some(mut source) = doc.parts.iter().find(|part| part.id == source_id).cloned()
            else {
                continue;
            };
            let target_id = doc.parts.iter().find_map(|target| {
                (target
                    .properties
                    .as_ref()
                    .and_then(|properties| properties.get(MIRRORED_COMPONENT_SOURCE))
                    .and_then(serde_json::Value::as_str)
                    == Some(source_id.as_str()))
                .then(|| target.id.clone())
            });
            if source
                .properties
                .as_ref()
                .and_then(|properties| properties.get(MIRRORED_COMPONENT_SUBSTITUTE))
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(false)
            {
                changed.push(source_id.clone());
                changed.extend(target_id);
                continue;
            }
            if let Some(target_id) = target_id.as_ref().filter(|id| {
                doc.parts
                    .iter()
                    .find(|part| &part.id == *id)
                    .and_then(|part| part.properties.as_ref())
                    .and_then(|properties| properties.get(MIRRORED_COMPONENT_SUBSTITUTE))
                    .and_then(serde_json::Value::as_bool)
                    .unwrap_or(false)
            }) {
                changed.push(target_id.clone());
                continue;
            }

            let target_id = match target_id {
                Some(id) => id,
                None => {
                    let base = format!("{target_layout_id}::mirror::{source_id}");
                    if doc.parts.iter().any(|part| part.id == base) {
                        let mut suffix = 1;
                        while doc
                            .parts
                            .iter()
                            .any(|part| part.id == format!("{base}::{suffix}"))
                        {
                            suffix += 1;
                        }
                        format!("{base}::{suffix}")
                    } else {
                        base
                    }
                }
            };
            // One rule for every mirrored part heading: controllers keep the plain reflection
            // so both halves share the world rotation; the axis map is `forward_angle = -r`
            // with `mapping.rotation == 0`.
            let is_controller = doc.definitions.iter().any(|definition| {
                definition.id == source.definition_id && definition.kind == PartKind::Controller
            });
            let axis_mapping = crate::model::Pose2 {
                at: Vec2 {
                    x: 2.0 * axis_x,
                    y: 0.0,
                },
                rotation: 0.0,
            };
            // Ticket 13: mirror the fresh evaluated pose; the stored pose goes
            // stale while anchored. Unanchored parts evaluate to their stored
            // pose, which keeps this identical.
            let source_board = evaluated_part_pose(doc, &source);
            let mut target_pose = crate::model::Pose2 {
                at: Vec2 {
                    x: 2.0 * axis_x - source_board.at.x,
                    y: source_board.at.y,
                },
                rotation: pair::mirrored_part_heading(
                    is_controller,
                    axis_mapping,
                    source_board.rotation,
                ),
            };
            let target_exists = doc.parts.iter().any(|part| part.id == target_id);
            if let Some(target) = doc
                .parts
                .iter()
                .find(|part| part.id == target_id)
                .filter(|_| {
                    doc.constraint_driving_part(&target_id)
                        .ok()
                        .flatten()
                        .is_some()
                })
            {
                let source_pose = crate::model::Pose2 {
                    at: Vec2 {
                        x: 2.0 * axis_x - target.pose.at.x,
                        y: target.pose.at.y,
                    },
                    rotation: pair::mirrored_part_heading(
                        is_controller,
                        axis_mapping,
                        target.pose.rotation,
                    ),
                };
                set_component_pose(doc, &source_id, source_pose)?;
                source = doc
                    .parts
                    .iter()
                    .find(|part| part.id == source_id)
                    .cloned()
                    .ok_or("Mirrored component source is missing")?;
                changed.push(source_id.clone());
                target_pose = crate::model::Pose2 {
                    at: Vec2 {
                        x: 2.0 * axis_x - source.pose.at.x,
                        y: source.pose.at.y,
                    },
                    rotation: pair::mirrored_part_heading(
                        is_controller,
                        axis_mapping,
                        source.pose.rotation,
                    ),
                };
            }

            let mut mirrored = source.clone();
            mirrored.id = target_id.clone();
            mirrored.reference = format!("{}_M", source.reference);
            mirrored.pose = target_pose;
            mirrored
                .properties
                .get_or_insert_with(BTreeMap::new)
                .insert(
                    MIRRORED_COMPONENT_SOURCE.into(),
                    serde_json::json!(source_id),
                );
            mirrored
                .properties
                .as_mut()
                .unwrap()
                .remove(MIRRORED_COMPONENT_SUBSTITUTE);
            if let Some(target) = doc.parts.iter_mut().find(|part| part.id == target_id) {
                *target = mirrored;
            } else {
                doc.parts.push(mirrored);
                if let Some(layout) = doc
                    .layouts
                    .iter_mut()
                    .find(|layout| layout.id == target_layout_id)
                {
                    layout.part_ids.push(target_id.clone());
                }
                let board_id = doc
                    .layouts
                    .iter()
                    .find(|layout| layout.id == target_layout_id)
                    .map(|layout| layout.board_id.clone())
                    .ok_or("Linked component target layout is missing")?;
                for board in &mut doc.boards {
                    if board.id == board_id && !board.part_ids.contains(&target_id) {
                        board.part_ids.push(target_id.clone());
                    }
                }
            }
            sync_component_nets(doc, &source_id, &target_id);
            for feature in &mut doc.outline {
                if let crate::model::OutlineFeature::PartEnvelope { part_ids, .. } = feature
                    && part_ids.contains(&source_id)
                    && !part_ids.contains(&target_id)
                {
                    part_ids.push(target_id.clone());
                }
            }
            changed.extend([source_id, target_id]);
            if !target_exists {
                changed.push(target_layout_id.clone());
            }
        }
    }
    Ok(changed)
}

pub fn move_linked_component(
    doc: &mut ProjectDoc,
    id: &str,
    at: Vec2,
) -> Result<Option<Vec<String>>, String> {
    let Some(target) = doc.parts.iter().find(|part| part.id == id) else {
        return Ok(None);
    };
    let Some(properties) = target.properties.as_ref() else {
        return Ok(None);
    };
    if properties
        .get(MIRRORED_COMPONENT_SUBSTITUTE)
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false)
    {
        return Ok(None);
    }
    let Some(source_id) = properties
        .get(MIRRORED_COMPONENT_SOURCE)
        .and_then(serde_json::Value::as_str)
        .map(str::to_string)
    else {
        return Ok(None);
    };
    let target_id = target.id.clone();
    let Some(source_layout) = doc
        .layouts
        .iter()
        .find(|layout| layout.part_ids.contains(&source_id))
    else {
        return Ok(None);
    };
    let Some(target_layout) = doc.layouts.iter().find(|layout| {
        layout
            .mirror_link
            .as_ref()
            .is_some_and(|link| link.source_id == source_layout.id)
    }) else {
        return Ok(None);
    };
    let axis_x = target_layout
        .mirror_link
        .as_ref()
        .ok_or("Linked component target layout is missing")?
        .axis_x;
    let is_controller = doc.definitions.iter().any(|definition| {
        definition.id == target.definition_id && definition.kind == PartKind::Controller
    });
    let axis_mapping = crate::model::Pose2 {
        at: Vec2 {
            x: 2.0 * axis_x,
            y: 0.0,
        },
        rotation: 0.0,
    };
    let rotation = pair::mirrored_part_heading(is_controller, axis_mapping, target.pose.rotation);
    set_component_pose(
        doc,
        &source_id,
        crate::model::Pose2 {
            at: Vec2 {
                x: 2.0 * axis_x - at.x,
                y: at.y,
            },
            rotation,
        },
    )?;
    Ok(Some(vec![source_id, target_id]))
}

pub(super) fn adopt_unassigned_components(
    doc: &mut ProjectDoc,
    source_layout_id: &str,
    axis_x: f64,
    changed: &mut Vec<String>,
) -> Result<(), String> {
    let source_layout = doc
        .layouts
        .iter()
        .find(|layout| layout.id == source_layout_id)
        .ok_or("Linked component source layout is missing")?;
    let board_id = source_layout.board_id.clone();
    let key_positions: Vec<_> = doc
        .matrices
        .iter()
        .filter(|matrix| source_layout.matrix_ids.contains(&matrix.id))
        .flat_map(matrix::cell_members)
        .filter_map(|(_, id)| doc.parts.iter().find(|part| part.id == id))
        .map(|part| part.pose.at.x)
        .collect();
    if key_positions.is_empty() {
        return Ok(());
    }
    let source_center = key_positions.iter().sum::<f64>() / key_positions.len() as f64;
    if (source_center - axis_x).abs() <= f64::EPSILON {
        return Ok(());
    }
    let source_is_left = source_center < axis_x;

    let mut assigned: BTreeSet<_> = doc
        .layouts
        .iter()
        .flat_map(|layout| layout.part_ids.iter().cloned())
        .collect();
    for matrix in &doc.matrices {
        assigned.extend(matrix.part_ids.iter().cloned());
        assigned.extend(matrix::cell_members(matrix).into_values());
    }
    let board_parts = doc
        .boards
        .iter()
        .find(|board| board.id == board_id)
        .ok_or("Linked component board is missing")?
        .part_ids
        .clone();
    let unassigned: Vec<_> = board_parts
        .into_iter()
        // Case hardware is resolved in board coordinates, independently of layout mirrors.
        .filter(|id| !assigned.contains(id) && !id.starts_with("case-closure/"))
        .filter_map(|id| {
            let part = doc.parts.iter().find(|part| part.id == id)?;
            let on_source_side = if source_is_left {
                part.pose.at.x < axis_x
            } else {
                part.pose.at.x > axis_x
            };
            on_source_side.then_some(id)
        })
        .collect();
    if unassigned.is_empty() {
        return Ok(());
    }
    let mut assigned_now = Vec::new();
    if let Some(layout) = doc
        .layouts
        .iter_mut()
        .find(|layout| layout.id == source_layout_id)
    {
        for id in unassigned {
            if !layout.part_ids.contains(&id) {
                layout.part_ids.push(id.clone());
                changed.push(id.clone());
                assigned_now.push(id);
            }
        }
    }
    // Ticket 12: newly assigned parts join the half frame immediately on
    // stored-gap pairs, so the legacy board-space pose never leaks into the
    // mirrored sync. Positions above are still board-space at this point.
    if let Some(offset) = layout_half_offset(doc, source_layout_id) {
        let convert: Vec<String> = assigned_now
            .into_iter()
            .filter(|id| !is_member_or_companion(doc, id))
            .collect();
        for part in &mut doc.parts {
            if convert.contains(&part.id) {
                part.pose.at.x -= offset.x;
            }
        }
    }
    Ok(())
}

/// Ticket 12: retune the constraint driving `id` for a move to the
/// board-space `pose`, so a dragged mirror copy carries its source's held
/// offset with it. A mirror-constraint refusal stops the drag. `None` leaves
/// a stored rotation bit-for-bit, like a direct drag. Returns whether a
/// constraint was retuned.
pub fn retune_driven_constraint(
    doc: &mut ProjectDoc,
    id: &str,
    board: crate::model::Pose2,
    turn: Option<f64>,
) -> Result<bool, String> {
    // A fresh partner has no drivers yet; the record below creates it.
    let Some(current) = doc.parts.iter().find(|part| part.id == id) else {
        return Ok(false);
    };
    let current_board = evaluated_part_pose(doc, current);
    let Some(constraint_id) = doc
        .constraint_driving_part(id)?
        .map(crate::model::Constraint::id)
        .map(str::to_owned)
    else {
        return Ok(false);
    };
    let constraint = doc
        .constraints
        .iter_mut()
        .find(|constraint| constraint.id() == constraint_id)
        .ok_or_else(|| format!("Constraint {constraint_id} disappeared"))?;
    let delta = Vec2 {
        x: board.at.x - current_board.at.x,
        y: board.at.y - current_board.at.y,
    };
    if crate::constraints::retune_offset(&doc.parts, constraint, delta, turn)?
        == crate::constraints::Retune::Mirror
    {
        return Err(format!(
            "Cannot move mirrored component because source {id} is controlled by a mirror constraint"
        ));
    }
    Ok(true)
}

pub(super) fn set_component_pose(
    doc: &mut ProjectDoc,
    id: &str,
    pose: crate::model::Pose2,
) -> Result<(), String> {
    // Ticket 12: poses arrive board-space; storage converts. The retune delta
    // measures board-space against the evaluated pose.
    let current_rotation = doc
        .parts
        .iter()
        .find(|part| part.id == id)
        .map(|part| evaluated_part_pose(doc, part).rotation);
    let turn = current_rotation.map(|rotation| pose.rotation - rotation);
    retune_driven_constraint(doc, id, pose, turn)?;
    let stored = stored_part_pose(doc, id, pose);
    let part = doc
        .parts
        .iter_mut()
        .find(|part| part.id == id)
        .ok_or("Mirrored component source is missing")?;
    part.pose = stored;
    Ok(())
}

pub(super) fn sync_component_nets(doc: &mut ProjectDoc, source_id: &str, target_id: &str) {
    for net in &mut doc.nets {
        let source_pins: Vec<_> = net
            .pins
            .iter()
            .filter(|pin| pin.part_id == source_id)
            .cloned()
            .collect();
        net.pins.retain(|pin| pin.part_id != target_id);
        net.pins.extend(source_pins.into_iter().map(|mut pin| {
            pin.part_id = target_id.to_string();
            pin
        }));
    }
}

pub(super) fn clear_component_link(part: &mut Part) {
    if let Some(properties) = part.properties.as_mut() {
        properties.remove(MIRRORED_COMPONENT_SOURCE);
        properties.remove(MIRRORED_COMPONENT_SUBSTITUTE);
        if properties.is_empty() {
            part.properties = None;
        }
    }
}

pub fn unlink_components(doc: &mut ProjectDoc, layout_id: &str) -> Vec<String> {
    let Some((part_ids, matrix_ids)) = doc
        .layouts
        .iter()
        .find(|layout| layout.id == layout_id)
        .map(|layout| (layout.part_ids.clone(), layout.matrix_ids.clone()))
    else {
        return Vec::new();
    };
    let mut changed = Vec::new();
    for id in part_ids {
        if let Some(part) = doc.parts.iter_mut().find(|part| part.id == id) {
            clear_component_link(part);
            changed.push(id);
        }
    }
    for matrix in doc
        .matrices
        .iter_mut()
        .filter(|matrix| matrix_ids.contains(&matrix.id))
    {
        for cell in &mut matrix.cells {
            if cell.assemblies_local == Some(true) {
                cell.assemblies_local = None;
            }
        }
        changed.push(matrix.id.clone());
    }
    changed
}

pub(super) fn remove_component(doc: &mut ProjectDoc, id: &str) {
    doc.parts.retain(|part| part.id != id);
    doc.constraints
        .retain(|constraint| constraint.source() != id && constraint.target() != id);
    for board in &mut doc.boards {
        board.part_ids.retain(|part_id| part_id != id);
    }
    for layout in &mut doc.layouts {
        layout.part_ids.retain(|part_id| part_id != id);
    }
    for matrix in &mut doc.matrices {
        matrix.part_ids.retain(|part_id| part_id != id);
    }
    for net in &mut doc.nets {
        net.pins.retain(|pin| pin.part_id != id);
    }
    for feature in &mut doc.outline {
        if let crate::model::OutlineFeature::PartEnvelope { part_ids, .. } = feature {
            part_ids.retain(|part_id| part_id != id);
        }
    }
}

pub fn mirrored_component_ids(doc: &ProjectDoc, requested: &[String]) -> Vec<String> {
    let mut ids: BTreeSet<_> = requested.iter().cloned().collect();
    for target in &doc.parts {
        let Some(source_id) = target
            .properties
            .as_ref()
            .and_then(|properties| properties.get(MIRRORED_COMPONENT_SOURCE))
            .and_then(serde_json::Value::as_str)
        else {
            continue;
        };
        if ids.contains(&target.id) || ids.contains(source_id) {
            ids.insert(target.id.clone());
            ids.insert(source_id.to_string());
        }
    }
    ids.into_iter().collect()
}

pub fn remove_matrix(doc: &mut ProjectDoc, matrix_id: &str) {
    let removed: BTreeSet<_> = doc
        .layouts
        .iter()
        .filter(|layout| layout.matrix_ids == [matrix_id])
        .map(|layout| layout.id.clone())
        .collect();
    let detached_parts: Vec<_> = doc
        .layouts
        .iter()
        .filter(|layout| {
            removed.contains(&layout.id)
                || layout
                    .mirror_link
                    .as_ref()
                    .is_some_and(|link| removed.contains(&link.source_id))
        })
        .flat_map(|layout| layout.part_ids.iter().cloned())
        .collect();
    doc.layouts.retain(|layout| !removed.contains(&layout.id));
    for layout in &mut doc.layouts {
        layout.matrix_ids.retain(|id| id != matrix_id);
    }
    for layout in &mut doc.layouts {
        if layout
            .mirror_link
            .as_ref()
            .is_some_and(|link| removed.contains(&link.source_id))
        {
            layout.mirror_link = None;
        }
    }
    for id in detached_parts {
        if let Some(part) = doc.parts.iter_mut().find(|part| part.id == id) {
            clear_component_link(part);
        }
    }
}
