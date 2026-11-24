//! Key moves, assembly placements, exception transitions and linked matrix removal.

use super::{exception_state, omitted, partner, retune_driven_constraint, stored_part_pose, sync};
use crate::{matrix, model::*};
use std::collections::{BTreeMap, BTreeSet};

/// Whether `id` is a matrix key: the primary member of a cell, never a companion of one.
pub fn is_matrix_key(doc: &ProjectDoc, id: &str) -> bool {
    primary_member(doc, id).is_some()
}

pub(super) fn primary_member<'a>(doc: &'a ProjectDoc, id: &str) -> Option<(&'a Matrix, u32, u32)> {
    for matrix in &doc.matrices {
        if !matrix.part_ids.iter().any(|part| part == id) {
            continue;
        }
        if let Some(((row, column), _)) = matrix::cell_members(matrix)
            .into_iter()
            .find(|(_, member)| member == id)
        {
            return Some((matrix, row, column));
        }
    }
    None
}

/// Numeric moves and keyboard nudges must update the same parametric geometry
/// as pointer edits; absolute part overrides would break the next linked edit.
pub fn move_keys(
    doc: &mut ProjectDoc,
    positions: &[Position],
) -> Result<(Vec<String>, BTreeSet<String>), String> {
    let before: BTreeMap<_, _> = doc
        .parts
        .iter()
        .map(|part| (part.id.clone(), part.pose))
        .collect();
    let mut matrices = BTreeMap::new();
    let mut handled = BTreeSet::new();
    let mut unchanged = BTreeSet::new();
    let mut carried = Vec::new();
    let mut constrained = Vec::new();
    for position in positions {
        let Some((source, row, column)) = primary_member(doc, &position.id) else {
            continue;
        };
        let part = doc
            .parts
            .iter()
            .find(|part| part.id == position.id)
            .ok_or("Key is missing")?;
        if part.locked == Some(true) {
            return Err(format!("Part {} is locked", part.id));
        }
        if !position.at.x.is_finite() || !position.at.y.is_finite() {
            return Err("Position must be finite".into());
        }
        if position.at == part.pose.at {
            handled.insert(position.id.clone());
            unchanged.insert(position.id.clone());
            continue;
        }
        if let Some(constraint) = doc.constraint_driving_part(&position.id)? {
            if constraint.is_mirror() {
                return Err(crate::constraints::mirror_drag_refusal(&position.id));
            }
            let mut pose = part.pose;
            pose.at = position.at;
            constrained.push((position.id.clone(), pose));
        }
        let matrix = matrices
            .entry(source.id.clone())
            .or_insert_with(|| source.clone());
        let displacement = Vec2 {
            x: position.at.x - part.pose.at.x,
            y: position.at.y - part.pose.at.y,
        };
        let cell = source
            .cells
            .iter()
            .find(|cell| cell.row == row && cell.column == column);
        if let Some(cell) = cell {
            for assembly in &cell.assemblies {
                let id = matrix::companion_id(&position.id, &assembly.id);
                if positions.iter().any(|position| position.id == id)
                    || doc.constraint_driving_part(&id).ok().flatten().is_some()
                {
                    continue;
                }
                if let Some(companion) = doc.parts.iter().find(|part| part.id == id)
                    && companion.properties.as_ref().is_some_and(|properties| {
                        properties.get(matrix::OVERRIDE) == Some(&serde_json::json!(1))
                    })
                {
                    let mut pose = companion.pose;
                    pose.at.x += displacement.x;
                    pose.at.y += displacement.y;
                    carried.push((id, pose));
                }
            }
        }
        // Older absolute key moves are absorbed into the cell's parametric offset.
        let evaluated = matrix::evaluated(source, doc)?;
        let generated = matrix::evaluation::cell_pose(&evaluated, row, column)?.at;
        let delta = Vec2 {
            x: position.at.x - generated.x,
            y: position.at.y - generated.y,
        };
        let frame = &evaluated
            .cells
            .iter()
            .find(|c| c.row == row as usize && c.column == column as usize)
            .ok_or("Key frame is missing")?
            .offset_frame;
        let local = matrix::evaluation::inverse_offset(frame, delta);
        let index = matrix
            .cells
            .iter()
            .position(|cell| cell.row == row && cell.column == column)
            .unwrap_or_else(|| {
                matrix.cells.push(MatrixCell {
                    shift: None,
                    bend: None,
                    deleted: false,
                    row,
                    column,
                    enabled: true,

                    definition_id: None,
                    variant: None,
                    offset: None,
                    rotation: None,
                    assemblies: vec![],
                    assemblies_local: None,
                });
                matrix.cells.len() - 1
            });
        let cell = &mut matrix.cells[index];
        let offset = cell.offset.unwrap_or_default();
        cell.offset = Some(Vec2 {
            x: offset.x + local.x,
            y: offset.y + local.y,
        });
        handled.insert(position.id.clone());
    }
    // Editing both sides at once would give two competing values for one link.
    if matrices
        .keys()
        .any(|id| partner(doc, id).is_some_and(|(other, _)| matrices.contains_key(&other)))
    {
        return Err("Move one linked half at a time".into());
    }
    let mut changed = vec![];
    for part in &mut doc.parts {
        if handled.contains(&part.id)
            && !unchanged.contains(&part.id)
            && let Some(properties) = &mut part.properties
        {
            properties.remove(matrix::OVERRIDE);
        }
    }
    for (id, pose) in constrained.into_iter().chain(carried) {
        let turn = doc
            .parts
            .iter()
            .find(|part| part.id == id)
            .map(|part| pose.rotation - super::evaluated_part_pose(doc, part).rotation);
        retune_driven_constraint(doc, &id, pose, turn)?;
        let stored = stored_part_pose(doc, &id, pose);
        if let Some(part) = doc.parts.iter_mut().find(|part| part.id == id) {
            part.pose = stored;
        }
        changed.push(id);
    }
    for matrix in matrices.values() {
        changed.extend(matrix::set_matrix(doc, matrix)?);
        changed.extend(sync(doc, &matrix.id)?);
    }
    changed.retain(|id| {
        before.get(id).is_none_or(|pose| {
            doc.parts
                .iter()
                .find(|part| &part.id == id)
                .is_none_or(|part| part.pose != *pose)
        })
    });
    Ok((changed, handled))
}

pub struct AssemblyPlacement {
    pub parent_id: String,
    pub local_pose: Pose2,
    pub authored_pose: Option<Pose2>,
    pub mode: MatrixCompanionMode,
}

/// Classify a generated companion the same way the solver places it: an explicit
/// constraint wins, then an authored pose, otherwise the companion follows its
/// key. The placement solver and the scene projection both read this mode, so the
/// precedence rule has one source.
pub fn companion_mode(doc: &ProjectDoc, companion_id: &str) -> MatrixCompanionMode {
    if let Some(constraint) = doc.constraint_driving_part(companion_id).ok().flatten() {
        return MatrixCompanionMode::Constrained {
            constraint_id: constraint.id().to_string(),
            source_part_id: constraint.source().to_string(),
        };
    }
    let authored = doc
        .parts
        .iter()
        .find(|part| part.id == companion_id)
        .is_some_and(|part| {
            part.properties.as_ref().is_some_and(|properties| {
                properties.get(matrix::OVERRIDE) == Some(&serde_json::json!(1))
            })
        });
    if authored {
        MatrixCompanionMode::Authored
    } else {
        MatrixCompanionMode::Follows
    }
}

pub fn assembly_placements(
    doc: &ProjectDoc,
    explicit: &[Position],
) -> BTreeMap<String, AssemblyPlacement> {
    let parts: BTreeMap<_, _> = doc
        .parts
        .iter()
        .map(|part| (part.id.as_str(), part))
        .collect();
    let mut placements = BTreeMap::new();
    for matrix in &doc.matrices {
        for cell in &matrix.cells {
            if !cell.enabled || cell.deleted {
                continue;
            }
            let parent_id = matrix::member_id(&matrix.id, cell.row, cell.column);
            let Some(parent) = parts.get(parent_id.as_str()) else {
                continue;
            };
            for assembly in &cell.assemblies {
                let id = matrix::companion_id(&parent_id, &assembly.id);
                if explicit.iter().any(|position| position.id == id) {
                    continue;
                }
                let Some(companion) = parts.get(id.as_str()) else {
                    continue;
                };
                let mode = companion_mode(doc, &id);
                let authored = matches!(mode, MatrixCompanionMode::Authored);
                let local_pose = if authored {
                    let (sin, cos) = parent.pose.rotation.to_radians().sin_cos();
                    let dx = companion.pose.at.x - parent.pose.at.x;
                    let dy = companion.pose.at.y - parent.pose.at.y;
                    Pose2 {
                        at: Vec2 {
                            x: dx * cos + dy * sin,
                            y: -dx * sin + dy * cos,
                        },
                        rotation: companion.pose.rotation - parent.pose.rotation,
                    }
                } else {
                    Pose2 {
                        at: assembly.offset,
                        rotation: assembly.rotation.unwrap_or(0.0),
                    }
                };
                placements.insert(
                    id,
                    AssemblyPlacement {
                        parent_id: parent_id.clone(),
                        local_pose,
                        authored_pose: authored.then_some(companion.pose),
                        mode,
                    },
                );
            }
        }
    }
    placements
}

pub fn reconcile_constraint_keys(
    doc: &mut ProjectDoc,
    moves: &BTreeMap<String, (Pose2, Pose2)>,
    solved: &BTreeMap<String, Pose2>,
) -> Result<Vec<String>, String> {
    let mut matrices = BTreeMap::new();
    for (id, (_, next)) in moves {
        let Some((source, row, column)) = primary_member(doc, id) else {
            continue;
        };
        let evaluated = matrix::evaluated(source, doc)?;
        let geometry = evaluated
            .cells
            .iter()
            .find(|c| c.row == row as usize && c.column == column as usize)
            .ok_or("Key frame is missing")?;
        let local = matrix::evaluation::inverse_offset(
            &geometry.offset_frame,
            Vec2 {
                x: next.at.x - geometry.pose.at.x,
                y: next.at.y - geometry.pose.at.y,
            },
        );
        let matrix = matrices
            .entry(source.id.clone())
            .or_insert_with(|| source.clone());
        let index = matrix
            .cells
            .iter()
            .position(|cell| cell.row == row && cell.column == column)
            .unwrap_or_else(|| {
                matrix.cells.push(MatrixCell {
                    shift: None,
                    bend: None,
                    deleted: false,
                    row,
                    column,
                    enabled: true,
                    definition_id: None,
                    variant: None,
                    offset: None,
                    rotation: None,
                    assemblies: vec![],
                    assemblies_local: None,
                });
                matrix.cells.len() - 1
            });
        let cell = &mut matrix.cells[index];
        let offset = cell.offset.unwrap_or_default();
        cell.offset = Some(Vec2 {
            x: offset.x + local.x,
            y: offset.y + local.y,
        });
        let sign = if matches!(source.mirror, Some(Mirror::X | Mirror::Y)) {
            -1.0
        } else {
            1.0
        };
        cell.rotation = Some(
            cell.rotation.unwrap_or_default() + sign * (next.rotation - geometry.pose.rotation),
        );
    }
    let mut changed = Vec::new();
    for matrix in matrices.values() {
        changed.extend(matrix::set_matrix(doc, matrix)?);
        changed.extend(sync(doc, &matrix.id)?);
    }
    // The combined dependency solver owns final poses, including independently
    // constrained and explicitly positioned companions overwritten by generation.
    // Solved poses are board-space; storage converts back to the half frame.
    for index in 0..doc.parts.len() {
        let id = doc.parts[index].id.clone();
        if let Some(pose) = solved.get(&id) {
            doc.parts[index].pose = stored_part_pose(doc, &id, *pose);
        }
    }
    Ok(changed)
}

/// Follow-deleted mirror matrices and their member parts for one removal,
/// plus retired absent (Omitted) records that leave no parts behind.
#[derive(Default)]
pub struct LinkedRemovalFollow {
    pub matrices: Vec<String>,
    pub part_ids: Vec<String>,
    pub retired: Vec<String>,
}

/// Ticket 21: source-removal lifecycle for linked matrix pairs. A Local
/// partner survives complete with its dimensions, layout and parts; only
/// the defunct pair records retire. A linked partner follows the deletion;
/// an Omitted partner stays absent and its record retires. Dependants of
/// retired records detach keeping their frame (an asymmetric detach still
/// refuses naming its Local exception). Unrelated pairs, surviving
/// ownership and anchors are preserved; no replacement is joined.
pub fn plan_linked_matrix_removal(
    doc: &mut ProjectDoc,
    matrix_id: &str,
    detached: &mut Vec<String>,
) -> Result<LinkedRemovalFollow, String> {
    let pairs: Vec<(String, String)> = doc
        .layouts
        .iter()
        .filter_map(|layout| layout.mirror_link.as_ref())
        .flat_map(|link| link.correspondences.iter())
        .filter(|record| {
            record.kind == MirroredItemKind::Matrix
                && (record.source_id == matrix_id || record.mirror_id == matrix_id)
        })
        .map(|record| (record.source_id.clone(), record.mirror_id.clone()))
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect();
    let mut follow = LinkedRemovalFollow::default();
    let mut retired_members: Vec<String> = vec![matrix_id.to_string()];
    for (source_id, mirror_id) in &pairs {
        let removed_is_source = source_id == matrix_id;
        let partner_id = if removed_is_source {
            mirror_id.clone()
        } else {
            source_id.clone()
        };
        if removed_is_source {
            match exception_state(doc, MirroredItemKind::Matrix, &partner_id) {
                // The Local partner survives complete; only the defunct
                // pair records retire below.
                Some(MirroredExceptionState::Local) => {}
                // The absent partner stays absent: detach its dependants
                // keeping their frame, then retire its record.
                Some(MirroredExceptionState::Omitted) => {
                    detached.extend(matrix::anchors::detach_dependants(doc, &partner_id)?);
                    doc.matrices.retain(|matrix| matrix.id != partner_id);
                    follow.retired.push(partner_id.clone());
                    retired_members.push(partner_id.clone());
                }
                // A linked partner follows the deletion with its parts.
                None => {
                    detached.extend(matrix::anchors::detach_dependants(doc, &partner_id)?);
                    if let Some(record) = doc.matrices.iter().find(|matrix| matrix.id == partner_id)
                    {
                        follow.part_ids.extend(record.part_ids.iter().cloned());
                    }
                    doc.matrices.retain(|matrix| matrix.id != partner_id);
                    follow.matrices.push(partner_id.clone());
                    retired_members.push(partner_id.clone());
                }
            }
        }
        retire_pair_records(doc, source_id, mirror_id, &retired_members);
    }
    Ok(follow)
}

/// Retire the matrix/part correspondences and member exceptions for one
/// matrix pair, limited to members of removed records. Surviving members
/// keep their exceptions and ownership.
pub(super) fn retire_pair_records(
    doc: &mut ProjectDoc,
    source_id: &str,
    mirror_id: &str,
    retired_members: &[String],
) {
    let source_prefix = format!("matrix/{source_id}/");
    let mirror_prefix = format!("matrix/{mirror_id}/");
    for layout in &mut doc.layouts {
        let Some(link) = layout.mirror_link.as_mut() else {
            continue;
        };
        link.correspondences.retain(|record| {
            if record.kind == MirroredItemKind::Matrix
                && ((record.source_id == source_id && record.mirror_id == mirror_id)
                    || (record.source_id == mirror_id && record.mirror_id == source_id))
            {
                return false;
            }
            if record.kind == MirroredItemKind::Part
                && ((record.source_id.starts_with(&source_prefix)
                    && record.mirror_id.starts_with(&mirror_prefix))
                    || (record.source_id.starts_with(&mirror_prefix)
                        && record.mirror_id.starts_with(&source_prefix)))
            {
                return false;
            }
            true
        });
        link.exceptions.retain(|exception| {
            if exception.kind == MirroredItemKind::Matrix && exception.mirror_id == mirror_id {
                return !retired_members.iter().any(|removed| removed == mirror_id);
            }
            if exception.kind == MirroredItemKind::Part {
                for removed in retired_members {
                    if exception
                        .mirror_id
                        .starts_with(&format!("matrix/{removed}/"))
                    {
                        return false;
                    }
                }
            }
            true
        });
    }
}

/// Ticket 21: record a Local matrix exception for a confirmed linked
/// detach. Refuses when the matrix is not a linked mirror member or the
/// overlay already exists, so stale confirmations change nothing.
pub fn add_local_matrix_exception(doc: &mut ProjectDoc, mirror_id: &str) -> Result<(), String> {
    let layout = doc
        .layouts
        .iter_mut()
        .find(|layout| {
            layout.matrix_ids.iter().any(|id| id == mirror_id) && layout.mirror_link.is_some()
        })
        .ok_or_else(|| format!("Matrix {mirror_id} is not a linked mirror member"))?;
    let Some(link) = layout.mirror_link.as_mut() else {
        return Err(format!("Matrix {mirror_id} is not a linked mirror member"));
    };
    if link.exceptions.iter().any(|exception| {
        exception.kind == MirroredItemKind::Matrix && exception.mirror_id == mirror_id
    }) {
        return Err(format!(
            "Confirmed detach is stale: {mirror_id} is already excepted"
        ));
    }
    link.exceptions.push(MirroredException {
        kind: MirroredItemKind::Matrix,
        mirror_id: mirror_id.to_string(),
        state: MirroredExceptionState::Local,
    });
    Ok(())
}

/// Ticket 21: an Omitted matrix has no independent storage state — it
/// follows its source live and rejoins to adopt it. Refuse direct storage
/// edits naming it (or its absent members) instead of silently diverging
/// the live record or discarding the edit on the next sync.
pub fn refuse_omitted_storage_edit(
    doc: &ProjectDoc,
    matrix_id: &str,
    cells: &[(u32, u32)],
) -> Result<(), String> {
    if exception_state(doc, MirroredItemKind::Matrix, matrix_id)
        == Some(MirroredExceptionState::Omitted)
    {
        return Err(format!(
            "Matrix {matrix_id} is Omitted and follows its source; Rejoin it to edit"
        ));
    }
    for (row, column) in cells {
        let key_id = crate::matrix::member_id(matrix_id, *row, *column);
        if omitted(doc, matrix_id, &key_id) {
            return Err(format!(
                "Key {key_id} is Omitted and follows its source; Rejoin it to edit"
            ));
        }
    }
    Ok(())
}

/// Whether a matrix edits alone: an independent (Local) or absent (Omitted)
/// matrix neither joins a grouped storage edit nor is joined by one. Its
/// partner follows through `sync` (live for Omitted, skipped for Local).
pub fn edits_alone(doc: &ProjectDoc, matrix_id: &str) -> bool {
    if matches!(
        exception_state(doc, MirroredItemKind::Matrix, matrix_id),
        Some(MirroredExceptionState::Local | MirroredExceptionState::Omitted)
    ) {
        return true;
    }
    partner(doc, matrix_id).is_some_and(|(partner, _)| {
        matches!(
            exception_state(doc, MirroredItemKind::Matrix, &partner),
            Some(MirroredExceptionState::Local | MirroredExceptionState::Omitted)
        )
    })
}
