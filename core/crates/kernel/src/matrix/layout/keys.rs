//! Key moves, assembly placements, exception transitions and linked matrix removal.

use super::{
    MIRRORED_COMPONENT_SOURCE, axis_mapping, exception_state, omitted, partner, reflected,
    remap_anchor_target, set_component_pose, stored_part_pose, sync,
};
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

pub fn has_linked_positions(doc: &ProjectDoc, positions: &[Position]) -> bool {
    let mut affected: BTreeSet<_> = positions
        .iter()
        .map(|position| position.id.as_str())
        .collect();
    loop {
        let previous = affected.len();
        for id in affected.clone() {
            if let Some(constraint) = doc.constraint_driving_part(id).ok().flatten() {
                affected.insert(constraint.target());
            }
        }
        if affected.len() == previous {
            break;
        }
    }
    if affected.iter().any(|id| primary_member(doc, id).is_some()) {
        return true;
    }
    positions.iter().any(|position| {
        primary_member(doc, &position.id).is_some()
            || doc.parts.iter().any(|part| {
                part.id == position.id
                    && part.properties.as_ref().is_some_and(|properties| {
                        properties.contains_key(MIRRORED_COMPONENT_SOURCE)
                    })
            })
            || doc.parts.iter().any(|target| {
                target
                    .properties
                    .as_ref()
                    .and_then(|properties| properties.get(MIRRORED_COMPONENT_SOURCE))
                    .and_then(serde_json::Value::as_str)
                    == Some(position.id.as_str())
            })
            || doc
                .constraint_driving_part(&position.id)
                .ok()
                .flatten()
                .is_some_and(|constraint| constraint.offset_axes().is_some())
    })
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
        set_component_pose(doc, &id, pose)?;
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

/// Ticket 21: same-board exception lifecycle for matrix/key overlays.
///
/// Runs on the accepted layout after it replaces the current one, before
/// `resolve()`. Added Omitted matrices shed their inherited parts here so
/// the scene delta names them; resolve keeps the live record in sync.
/// Making an anchored matrix Local detaches its own anchor keeping its
/// pose. Rejoin and Omitted-to-Local materialize parts from the live
/// record; resolve then adopts the current source layout and anchor. Local
/// keys returning to a shared state reset their independent offset and
/// rotation to the followed cell. Explicit child exceptions are otherwise
/// preserved untouched.
pub fn apply_exception_transitions(
    doc: &mut ProjectDoc,
    previous: &Layout,
    incoming: &Layout,
) -> Result<Vec<String>, String> {
    let Some(next_link) = incoming.mirror_link.as_ref() else {
        return Ok(vec![]);
    };
    let prev_link = previous.mirror_link.as_ref();
    if prev_link.is_some_and(|link| link.source_id != next_link.source_id) {
        return Ok(vec![]);
    }
    let prev_state = |kind: MirroredItemKind, mirror_id: &str| {
        prev_link
            .and_then(|link| {
                link.exceptions
                    .iter()
                    .find(|exception| exception.kind == kind && exception.mirror_id == mirror_id)
            })
            .map(|exception| exception.state)
    };
    let mut changed = Vec::new();
    // Matrix records and parts first, so key resets below see live cells.
    let matrix_ids: BTreeSet<&str> = incoming
        .matrix_ids
        .iter()
        .chain(previous.matrix_ids.iter())
        .map(String::as_str)
        .collect();
    for mirror_id in matrix_ids {
        let before = prev_state(MirroredItemKind::Matrix, mirror_id);
        let after = next_link
            .exceptions
            .iter()
            .find(|exception| {
                exception.kind == MirroredItemKind::Matrix && exception.mirror_id == mirror_id
            })
            .map(|exception| exception.state);
        if before == after {
            continue;
        }
        let record = doc
            .matrices
            .iter()
            .find(|matrix| matrix.id == mirror_id)
            .cloned();
        match after {
            // Making an anchored matrix Local detaches its own anchor and
            // keeps its pose; a Local record never authors its partner, so
            // the paired sync inside detach is a no-op for it.
            Some(MirroredExceptionState::Local)
                if record
                    .as_ref()
                    .is_some_and(|matrix| matrix.anchor.is_some()) =>
            {
                changed.extend(matrix::anchors::detach(doc, mirror_id)?);
            }
            // An absent matrix sheds (or keeps shed) its inherited parts
            // now so the scene delta names them; resolve keeps the live
            // record following the source.
            Some(MirroredExceptionState::Omitted) => {
                if let Some(record) = record {
                    changed.extend(matrix::set_matrix(doc, &record)?);
                }
            }
            // Omitted-to-Local and Rejoin materialize parts from the live
            // record now; resolve adopts the current source layout/anchor.
            // A record-absent Omitted member synthesizes from its source.
            Some(MirroredExceptionState::Local) | None if before.is_some() => {
                let record = match record {
                    Some(record) => record,
                    None if before == Some(MirroredExceptionState::Omitted) => {
                        synthesize_rejoin_record(doc, incoming, mirror_id)?
                    }
                    None => return Err(format!("Linked matrix {mirror_id} is missing")),
                };
                changed.extend(matrix::set_matrix(doc, &record)?);
            }
            _ => {}
        }
    }
    // Key overlays: an Omitted key needs its correspondence overlay, and a
    // key leaving Local returns to the followed cell instead of freezing
    // its former Local placement.
    let mut part_transitions: Vec<(String, Option<MirroredExceptionState>)> = next_link
        .exceptions
        .iter()
        .filter(|exception| exception.kind == MirroredItemKind::Part)
        .map(|exception| (exception.mirror_id.clone(), Some(exception.state)))
        .collect();
    if let Some(prev_link) = prev_link {
        for exception in &prev_link.exceptions {
            if exception.kind != MirroredItemKind::Part
                || next_link.exceptions.iter().any(|next| {
                    next.kind == MirroredItemKind::Part && next.mirror_id == exception.mirror_id
                })
            {
                continue;
            }
            // A removed overlay rejoins the shared layout.
            part_transitions.push((exception.mirror_id.clone(), None));
        }
    }
    for (mirror_id, after) in part_transitions {
        let before = prev_state(MirroredItemKind::Part, &mirror_id);
        if before == after {
            continue;
        }
        if after == Some(MirroredExceptionState::Omitted) {
            ensure_part_correspondence(doc, &incoming.id, &mirror_id)?;
        }
        if before == Some(MirroredExceptionState::Local) && after != before {
            reset_key_to_followed(doc, &incoming.id, &mirror_id)?;
        }
        // Shed the part when the key becomes absent; materialize it when
        // the key returns from an excepted state. `set_matrix` reports the
        // shed/created part, so the scene delta names it; resolve converges
        // idempotently afterward.
        let shed = after == Some(MirroredExceptionState::Omitted);
        let materialize = before.is_some() && !shed;
        if (shed || materialize)
            && let Some(owner) = owning_matrix(doc, &incoming.id, &mirror_id)
        {
            let record = doc
                .matrices
                .iter()
                .find(|matrix| matrix.id == owner)
                .cloned();
            if let Some(record) = record {
                changed.extend(matrix::set_matrix(doc, &record)?);
            }
        }
    }
    changed.sort();
    changed.dedup();
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
    let link = layout.mirror_link.as_mut().expect("linked mirror");
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
        companion_mode: None,
        companion_tweak: None,
        removed_copper: None,
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

/// The mirror matrix in `layout_id` owning `mirror_id`, by slot identity.
pub(super) fn owning_matrix(doc: &ProjectDoc, layout_id: &str, mirror_id: &str) -> Option<String> {
    let layout = doc.layouts.iter().find(|layout| layout.id == layout_id)?;
    layout
        .matrix_ids
        .iter()
        .find(|matrix_id| mirror_id.starts_with(&format!("matrix/{matrix_id}/r")))
        .cloned()
}

/// Parse `matrix/<matrix>/r<row>c<column>` keys; companions resolve to their key.
pub(super) fn split_key_id(id: &str) -> Option<(String, u32, u32)> {
    let rest = id.strip_prefix("matrix/")?;
    let (matrix_id, slot) = rest.split_once("/r")?;
    let slot = slot.split('/').next()?;
    let (row, column) = slot.split_once('c')?;
    Some((
        matrix_id.to_string(),
        row.parse().ok()?,
        column.parse().ok()?,
    ))
}

/// Ensure an Omitted key overlays a part correspondence, adding it by slot
/// when the source slot exists. Refuses naming the missing counterpart
/// instead of overlaying nothing.
pub(super) fn ensure_part_correspondence(
    doc: &mut ProjectDoc,
    layout_id: &str,
    mirror_id: &str,
) -> Result<(), String> {
    let layout = doc
        .layouts
        .iter()
        .find(|layout| layout.id == layout_id)
        .ok_or("Linked layout is missing")?;
    let link = layout
        .mirror_link
        .as_ref()
        .ok_or("Linked layout is missing")?;
    if link
        .correspondences
        .iter()
        .any(|record| record.kind == MirroredItemKind::Part && record.mirror_id == mirror_id)
    {
        return Ok(());
    }
    let (mirror_matrix, row, column) = split_key_id(mirror_id)
        .ok_or_else(|| format!("Layout exception {mirror_id:?} overlays no correspondence"))?;
    let source_matrix = link
        .correspondences
        .iter()
        .find(|record| record.kind == MirroredItemKind::Matrix && record.mirror_id == mirror_matrix)
        .map(|record| record.source_id.clone())
        .ok_or_else(|| format!("Layout exception {mirror_id:?} overlays no correspondence"))?;
    let source = matrix::evaluation::logical_matrix(&source_matrix, doc)
        .map_err(|_| format!("Layout exception {mirror_id:?} overlays no correspondence"))?;
    if row >= source.rows || column >= source.columns {
        return Err(format!(
            "Layout exception {mirror_id:?} names a missing counterpart in matrix {source_matrix:?}; resize or Rejoin its parent instead of choosing another key"
        ));
    }
    let source_id = matrix::member_id(&source_matrix, row, column);
    if link
        .correspondences
        .iter()
        .any(|record| record.kind == MirroredItemKind::Part && record.source_id == source_id)
    {
        return Err(format!(
            "Layout exception {mirror_id:?} names a missing counterpart in matrix {source_matrix:?}; resize or Rejoin its parent instead of choosing another key"
        ));
    }
    let layout = doc
        .layouts
        .iter_mut()
        .find(|layout| layout.id == layout_id)
        .ok_or("Linked layout is missing")?;
    if let Some(link) = layout.mirror_link.as_mut() {
        link.correspondences.push(MirroredCorrespondence {
            kind: MirroredItemKind::Part,
            source_id,
            mirror_id: mirror_id.to_string(),
            companion_mode: None,
            companion_tweak: None,
        });
    }
    Ok(())
}

/// Ticket 21: materialize a record-absent Omitted member on Rejoin (or
/// Omitted-to-Local): clone its source record through the pair map and
/// retarget its anchor by correspondence. A missing counterpart refuses
/// naming it instead of silently retargeting.
pub(super) fn synthesize_rejoin_record(
    doc: &ProjectDoc,
    layout: &Layout,
    mirror_id: &str,
) -> Result<Matrix, String> {
    let link = layout
        .mirror_link
        .as_ref()
        .ok_or("Linked layout is missing")?;
    let source_id = link
        .correspondences
        .iter()
        .find(|record| record.kind == MirroredItemKind::Matrix && record.mirror_id == mirror_id)
        .map(|record| record.source_id.clone())
        .ok_or_else(|| format!("Linked matrix {mirror_id} has no source correspondence"))?;
    let source = matrix::evaluation::logical_matrix(&source_id, doc)
        .map_err(|_| format!("Linked matrix {mirror_id} has no source correspondence"))?;
    let owned = source.into_owned();
    let placeholder = Matrix {
        id: mirror_id.to_string(),
        name: None,
        board_id: Some(layout.board_id.clone()),
        part_ids: Vec::new(),
        ..owned.clone()
    };
    let mut synthesized = reflected(
        &owned,
        &placeholder,
        axis_mapping(link.axis_x),
        false,
        link.split_gap.is_some(),
    )?;
    synthesized.board_id = Some(layout.board_id.clone());
    if let Some(anchor) = synthesized.anchor.clone() {
        let mapped =
            remap_anchor_target(&link.correspondences, &anchor.matrix_id, true, &layout.id)?;
        if let Some(result_anchor) = synthesized.anchor.as_mut() {
            result_anchor.matrix_id = mapped;
        }
    }
    Ok(synthesized)
}

/// Return a key leaving Local to the followed cell: copy the counterpart's
/// independent offset/rotation, or clear them when the key follows no
/// counterpart. The next resolve then materializes the shared pose.
pub(super) fn reset_key_to_followed(
    doc: &mut ProjectDoc,
    layout_id: &str,
    mirror_id: &str,
) -> Result<(), String> {
    let Some((mirror_matrix, row, column)) = split_key_id(mirror_id) else {
        return Ok(());
    };
    let source_cell = doc
        .layouts
        .iter()
        .find(|layout| layout.id == layout_id)
        .and_then(|layout| layout.mirror_link.as_ref())
        .and_then(|link| {
            link.correspondences.iter().find(|record| {
                record.kind == MirroredItemKind::Matrix && record.mirror_id == mirror_matrix
            })
        })
        .and_then(|record| {
            doc.matrices
                .iter()
                .find(|matrix| matrix.id == record.source_id)
        })
        .and_then(|source| {
            source
                .cells
                .iter()
                .find(|cell| cell.row == row && cell.column == column)
        })
        .map(|cell| (cell.offset, cell.rotation));
    let Some(mirror) = doc
        .matrices
        .iter_mut()
        .find(|matrix| matrix.id == mirror_matrix)
    else {
        return Ok(());
    };
    if let Some(cell) = mirror
        .cells
        .iter_mut()
        .find(|cell| cell.row == row && cell.column == column)
    {
        let (offset, rotation) = source_cell.unwrap_or((None, None));
        cell.offset = offset;
        cell.rotation = rotation;
    }
    Ok(())
}
