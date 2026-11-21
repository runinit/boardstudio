//! Materialize matrix slots after structural or lifecycle input authentication.

use super::{
    MembershipContext, TransitionAssociation, TransitionState, cell_offset_delta,
    lifecycle::adopt_rejoined_anchor, project_matrix, remove_omitted_matrix_parts,
    require_matrix_pair, transition_matrix_by_id,
};
use crate::model::{Matrix, MirroredItemKind, ProjectDoc, Vec2};
use std::collections::BTreeSet;

pub(super) fn slot_projection<'a>(
    staged: &ProjectDoc,
    before: &ProjectDoc,
    context: &MembershipContext,
    mirror: &Matrix,
    association_for: impl Fn(&str) -> Option<&'a TransitionAssociation>,
    added: impl Fn(&str) -> bool,
) -> Result<(Matrix, BTreeSet<String>), String> {
    let matrix_pair = require_matrix_pair(context)?;
    let source = transition_matrix_by_id(staged, &matrix_pair.source_id)?;
    let record = context
        .pair_right
        .correspondences
        .iter()
        .find(|record| {
            record.kind == MirroredItemKind::Matrix
                && record.source_id == source.id
                && record.mirror_id == mirror.id
        })
        .ok_or("The transition matrix association is unavailable")?;
    let mut projected = project_matrix(source, mirror, record, context.mapping)?;
    if context.pending_rejoins.contains_key(&source.id) {
        projected.anchor = source.anchor.clone();
        adopt_rejoined_anchor(&mut projected, staged, context, matrix_pair)?;
    }
    let mut omitted_target_parts = BTreeSet::new();
    let mut local_key_preservations = Vec::new();
    // A Rejoined owner uses the source descriptor as its seed. Explicit Local
    // hardware still belongs to the previous target, including its assembly overrides.
    let previous = staged.matrices.iter().find(|m| m.id == mirror.id);
    let omitted_owner = context.associations.iter().any(|a| {
        a.kind == MirroredItemKind::Matrix
            && a.source_id == source.id
            && a.state == TransitionState::Omitted
    });
    for cell in &mut projected.cells {
        if !cell.enabled || cell.deleted {
            continue;
        }
        let source_key_id = crate::matrix::member_id(&matrix_pair.source_id, cell.row, cell.column);
        let mirror_key_id = crate::matrix::member_id(&matrix_pair.mirror_id, cell.row, cell.column);
        let Some(source_key) = staged.parts.iter().find(|part| part.id == source_key_id) else {
            return Err(format!(
                "The projected source key {source_key_id:?} is unavailable"
            ));
        };
        let original_key = association_for(&source_key_id);
        if original_key.is_some_and(|association| association.state == TransitionState::Omitted)
            || (original_key.is_none() && omitted_owner)
        {
            cell.deleted = true;
            cell.enabled = false;
            cell.assemblies.clear();
            cell.assemblies_local = None;
            continue;
        }
        if original_key.is_none() && !added(&source_key_id) {
            return Err(format!(
                "The source identity {source_key_id:?} has no private association"
            ));
        }
        if let Some(association) = original_key.filter(|a| a.state == TransitionState::Local) {
            if let Some(previous) = previous {
                *cell = previous
                    .cells
                    .iter()
                    .find(|candidate| candidate.row == cell.row && candidate.column == cell.column)
                    .ok_or_else(|| {
                        format!(
                            "The retained Local matrix cell r{}c{} is unavailable",
                            cell.row, cell.column
                        )
                    })?
                    .clone();
            }
            local_key_preservations.push((
                cell.row,
                cell.column,
                association.mirror_id.clone(),
                mirror_key_id.clone(),
            ));
            for assembly in &mut cell.assemblies {
                let source_child_id = crate::matrix::companion_id(&source_key_id, &assembly.id);
                let Some(child) = association_for(&source_child_id) else {
                    continue;
                };
                if child.state == TransitionState::Omitted && child.explicit_state {
                    omitted_target_parts
                        .insert(crate::matrix::companion_id(&mirror_key_id, &assembly.id));
                } else if previous.is_none() && child.state == TransitionState::Local {
                    // An Omitted matrix has no saved assembly descriptor. Both Local
                    // Parts survive on the board, including independently authored offsets.
                    let local_pose =
                        |id: &str| {
                            before.parts.iter().find(|p| p.id == id).ok_or_else(|| {
                                format!("The retained Local Part {id:?} is unavailable")
                            })
                        };
                    super::membership_view::retain_companion_pose(
                        assembly,
                        local_pose(&association.mirror_id)?.pose,
                        local_pose(&child.mirror_id)?,
                    )?;
                }
            }
            continue;
        }
        let mut next_assemblies = Vec::with_capacity(cell.assemblies.len());
        for mut assembly in std::mem::take(&mut cell.assemblies) {
            let source_child_id = crate::matrix::companion_id(&source_key_id, &assembly.id);
            let mirror_child_id = crate::matrix::companion_id(&mirror_key_id, &assembly.id);
            let original = association_for(&source_child_id);
            if original.is_none() && !added(&source_child_id) {
                return Err(format!(
                    "The companion settings for {source_child_id:?} are unavailable"
                ));
            }
            if original.is_some_and(|association| association.state == TransitionState::Omitted) {
                omitted_target_parts.insert(mirror_child_id);
                next_assemblies.push(assembly);
                continue;
            }
            if original.is_some_and(|association| association.state == TransitionState::Local) {
                let local_assembly = previous
                    .into_iter()
                    .flat_map(|matrix| &matrix.cells)
                    .find(|candidate| candidate.row == cell.row && candidate.column == cell.column)
                    .and_then(|candidate| {
                        candidate
                            .assemblies
                            .iter()
                            .find(|item| item.id == assembly.id)
                    });
                if let Some(local) = local_assembly {
                    assembly = local.clone();
                } else if previous.is_some()
                    && !context.pending_rejoins.contains_key(&source_key_id)
                {
                    return Err(format!(
                        "The retained Local companion {mirror_child_id:?} is unavailable"
                    ));
                }
                next_assemblies.push(assembly);
                continue;
            }
            let mode = original
                .map(|association| association.effective_mode)
                .unwrap_or(context.pair_right.companion_mode);
            let tweak = original.and_then(|association| association.effective_tweak);
            let source_companion = staged
                .parts
                .iter()
                .find(|part| part.id == source_child_id)
                .ok_or_else(|| {
                    format!("The projected source companion {source_child_id:?} is unavailable")
                })?;
            let (offset, rotation) = crate::mirrored_half::derive::companion_assembly(
                crate::mirrored_half::derive::CompanionAssemblyInput {
                    offset: assembly.offset,
                    rotation: assembly.rotation,
                    key: Some(source_key),
                    companion: Some(source_companion),
                    mapping: context.mapping,
                    mode,
                    tweak,
                    stabiliser: crate::mirrored_half::SYMMETRIC_DEFINITIONS
                        .contains(&source_companion.definition_id.as_str()),
                    fallback: false,
                },
            )
            .map_err(str::to_owned)?;
            if !offset.x.is_finite()
                || !offset.y.is_finite()
                || rotation.is_some_and(|value| !value.is_finite())
            {
                return Err("The projected matrix companion contains non-finite geometry".into());
            }
            assembly.offset = offset;
            assembly.rotation = rotation;
            next_assemblies.push(assembly);
        }
        cell.assemblies = next_assemblies;
    }
    if !local_key_preservations.is_empty() {
        let mut candidate = staged.clone();
        let local_ids = context
            .associations
            .iter()
            .filter(|a| {
                a.kind == MirroredItemKind::Part
                    && a.state == TransitionState::Local
                    && context.pending_rejoins.contains_key(&source.id)
            })
            .map(|a| a.mirror_id.as_str());
        set_slots(&mut candidate, &projected, local_ids)?;
        for (row, column, old_id, target_id) in &local_key_preservations {
            let before_part = before
                .parts
                .iter()
                .find(|part| part.id == *old_id)
                .ok_or_else(|| format!("The Local key snapshot {old_id:?} is unavailable"))?;
            let candidate_part = candidate
                .parts
                .iter()
                .find(|part| part.id == *target_id)
                .ok_or_else(|| format!("The candidate Local key {target_id:?} is unavailable"))?;
            let world_delta = Vec2 {
                x: before_part.pose.at.x - candidate_part.pose.at.x,
                y: before_part.pose.at.y - candidate_part.pose.at.y,
            };
            let offset_delta =
                cell_offset_delta(&projected, *row, *column, world_delta, &candidate)?;
            let cell = projected
                .cells
                .iter_mut()
                .find(|cell| cell.row == *row && cell.column == *column)
                .ok_or_else(|| format!("The Local matrix cell r{row}c{column} is unavailable"))?;
            let offset = cell.offset.unwrap_or_default();
            cell.offset = Some(Vec2 {
                x: offset.x + offset_delta.x,
                y: offset.y + offset_delta.y,
            });
        }
    }
    if !context.pending_rejoins.is_empty() {
        let evaluated = crate::matrix::evaluated(&projected, staged)?;
        for cell in &mut projected.cells {
            let key = crate::matrix::member_id(&source.id, cell.row, cell.column);
            if !context.pending_rejoins.contains_key(&key) {
                continue;
            }
            for assembly in &mut cell.assemblies {
                let id = crate::matrix::companion_id(&key, &assembly.id);
                if let Some(local) =
                    association_for(&id).filter(|a| a.state == TransitionState::Local)
                {
                    let part = before
                        .parts
                        .iter()
                        .find(|p| p.id == local.mirror_id)
                        .ok_or("The retained Local companion is unavailable")?;
                    let key_pose = evaluated
                        .cells
                        .iter()
                        .find(|item| {
                            item.row == cell.row as usize && item.column == cell.column as usize
                        })
                        .ok_or("The Rejoined key pose is unavailable")?
                        .pose;
                    super::membership_view::retain_companion_pose(assembly, key_pose, part)?;
                }
            }
        }
    }
    Ok((projected, omitted_target_parts))
}

pub(super) fn materialize_selection(
    draft: &mut ProjectDoc,
    before: &ProjectDoc,
    context: &MembershipContext,
    owner: &TransitionAssociation,
) -> Result<Vec<TransitionAssociation>, String> {
    let mut additions = Vec::new();
    let matrix_pair = require_matrix_pair(context)?;
    if owner.state == TransitionState::Omitted {
        draft.matrices.retain(|matrix| matrix.id != owner.mirror_id);
        return Ok(additions);
    }
    let retained: Vec<_> = context
        .associations
        .iter()
        .filter(|a| {
            a.kind == MirroredItemKind::Part
                && a.state == TransitionState::Local
                && super::matrix_identity_slot(&a.mirror_id, &matrix_pair.mirror_id).is_some()
        })
        .filter_map(|a| before.parts.iter().find(|p| p.id == a.mirror_id).cloned())
        .collect();
    for association in &context.associations {
        if association.kind == MirroredItemKind::Part
            && context.pending_rejoins.contains_key(&association.source_id)
            && let Some(part) = draft
                .parts
                .iter_mut()
                .find(|p| p.id == association.mirror_id)
            && let Some(properties) = &mut part.properties
        {
            properties.remove("layoutOverride");
        }
    }
    if owner.state == TransitionState::Local {
        let current = draft
            .matrices
            .iter()
            .find(|m| m.id == matrix_pair.mirror_id.as_str())
            .ok_or("The Local lifecycle Matrix is unavailable")?
            .clone();
        let mut next = current.clone();
        for cell in &mut next.cells {
            let key = crate::matrix::member_id(&matrix_pair.mirror_id, cell.row, cell.column);
            if context.associations.iter().any(|a| {
                a.kind == MirroredItemKind::Part
                    && a.mirror_id == key
                    && a.state == TransitionState::Omitted
            }) {
                cell.enabled = false;
                cell.deleted = true;
                cell.assemblies.clear();
            } else {
                cell.assemblies.retain(|assembly| {
                    !context.associations.iter().any(|a| {
                        a.kind == MirroredItemKind::Part
                            && a.state == TransitionState::Omitted
                            && a.mirror_id == crate::matrix::companion_id(&key, &assembly.id)
                    })
                });
            }
        }
        if next != current {
            materialize_slots(draft, &next, &BTreeSet::new(), retained)?;
        }
        return Ok(additions);
    }
    let source = transition_matrix_by_id(draft, &matrix_pair.source_id)?;
    let mut seed;
    let current = draft
        .matrices
        .iter()
        .find(|m| m.id == matrix_pair.mirror_id);
    let mirror = if context.pending_rejoins.contains_key(&owner.source_id) {
        // Rejoin adopts overlapping slots already owned by this matrix. The setter drops
        // target-only growth and materializes source-only growth.
        for id in &source.part_ids {
            if context
                .associations
                .iter()
                .any(|a| a.kind == MirroredItemKind::Part && a.source_id == *id)
            {
                continue;
            }
            let (row, column, suffix) = super::matrix_identity_slot(id, &matrix_pair.source_id)
                .ok_or("A new owner Rejoin member is not canonical")?;
            let mirror_key = crate::matrix::member_id(&matrix_pair.mirror_id, row, column);
            let mirror_id = if suffix.is_empty() {
                mirror_key.clone()
            } else {
                crate::matrix::companion_id(&mirror_key, suffix)
            };
            if draft.parts.iter().any(|p| p.id == mirror_id)
                && !current.is_some_and(|matrix| matrix.part_ids.contains(&mirror_id))
            {
                return Err(format!(
                    "A new owner Rejoin member collides with existing Part {mirror_id:?}"
                ));
            }
            additions.push(TransitionAssociation {
                kind: MirroredItemKind::Part,
                source_id: id.clone(),
                mirror_id,
                parent_source_id: Some(if suffix.is_empty() {
                    matrix_pair.source_id.clone()
                } else {
                    crate::matrix::member_id(&matrix_pair.source_id, row, column)
                }),
                parent_mirror_id: Some(if suffix.is_empty() {
                    matrix_pair.mirror_id.clone()
                } else {
                    mirror_key
                }),
                state: TransitionState::Linked,
                explicit_state: false,
                raw_mode_override: None,
                raw_tweak: None,
                effective_mode: context.pair_right.companion_mode,
                effective_tweak: None,
            });
        }
        seed = source.clone();
        seed.id.clone_from(&matrix_pair.mirror_id);
        seed.board_id = Some(context.mirror_board_id.clone());
        seed.part_ids.clear();
        &seed
    } else {
        current.ok_or("The lifecycle mirror Matrix is unavailable")?
    };
    let (projected, omitted) = slot_projection(
        draft,
        before,
        context,
        mirror,
        |id| {
            context
                .associations
                .iter()
                .find(|a| a.kind == MirroredItemKind::Part && a.source_id == id)
        },
        |id| additions.iter().any(|a| a.source_id == id),
    )?;
    materialize_slots(draft, &projected, &omitted, retained)?;
    Ok(additions)
}

pub(super) fn materialize_slots(
    draft: &mut ProjectDoc,
    projected: &Matrix,
    omitted: &BTreeSet<String>,
    retained: Vec<crate::model::Part>,
) -> Result<(), String> {
    set_slots(draft, projected, retained.iter().map(|p| p.id.as_str()))?;
    remove_omitted_matrix_parts(draft, &projected.id, omitted);
    for local in retained {
        let target = draft
            .parts
            .iter_mut()
            .find(|p| p.id == local.id)
            .ok_or("A retained Local Matrix child was removed")?;
        *target = local;
    }
    Ok(())
}

/// Keep visible Local children registered exactly when the setter will materialize
/// their slot. Children of a tombstone survive independently on the board.
fn set_slots<'a>(
    draft: &mut ProjectDoc,
    projected: &Matrix,
    local_ids: impl Iterator<Item = &'a str>,
) -> Result<(), String> {
    if !draft.matrices.iter().any(|m| m.id == projected.id) {
        let mut seed = projected.clone();
        seed.part_ids.clear();
        draft.matrices.push(seed);
    }
    let current = draft
        .matrices
        .iter_mut()
        .find(|m| m.id == projected.id)
        .ok_or("The target matrix is unavailable")?;
    for id in local_ids {
        let Some((row, column, suffix)) = super::matrix_identity_slot(id, &projected.id) else {
            continue;
        };
        let cell = projected
            .cells
            .iter()
            .find(|cell| cell.row == row && cell.column == column);
        let active = row < projected.rows
            && column < projected.columns
            && cell.is_none_or(|c| c.enabled && !c.deleted)
            && (suffix.is_empty()
                || cell.is_some_and(|c| c.assemblies.iter().any(|a| a.id == suffix)));
        if active {
            if !current.part_ids.iter().any(|member| member == id) {
                current.part_ids.push(id.to_owned());
            }
        } else {
            current.part_ids.retain(|member| member != id);
        }
    }
    crate::matrix::set_matrix(draft, projected)?;
    Ok(())
}
