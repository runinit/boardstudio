//! The membership-transition kernel and its public preparation entry point.

use super::{
    FinalOwner, MembershipContextDelta, MembershipTransitionInput, MembershipTransitionOutput,
    TransitionAssociation, TransitionState, deduplicate_outline_members, emit_validated_transition,
    lifecycle, matrix_identity_slot, require_matrix_pair, transition_matrix_by_id,
    validate_transition_input,
};
use crate::model::{EditOperation, MirroredItemKind};
use std::collections::BTreeSet;

/// The structural command identifies the removed axis, including when it was empty.
#[derive(Clone, Copy)]
pub(super) enum RemovedAxis {
    Row(u32),
    Column(u32),
}

impl RemovedAxis {
    pub(super) fn for_operation(operation: &EditOperation, source: &str) -> Option<Self> {
        match operation {
            EditOperation::RemoveMatrixRow { matrix_id, row } if matrix_id == source => {
                Some(Self::Row(*row))
            }
            EditOperation::RemoveMatrixColumn { matrix_id, column } if matrix_id == source => {
                Some(Self::Column(*column))
            }
            _ => None,
        }
    }

    pub(super) fn remap(self, row: u32, column: u32) -> Option<(u32, u32)> {
        match self {
            Self::Row(index) => (row != index).then(|| (row - u32::from(row > index), column)),
            Self::Column(index) => {
                (column != index).then(|| (row, column - u32::from(column > index)))
            }
        }
    }
}

fn project_membership(
    input: MembershipTransitionInput<'_>,
    axis: Option<RemovedAxis>,
) -> Result<MembershipTransitionOutput, String> {
    let matrix_pair = require_matrix_pair(input.context)?;

    let validated = validate_transition_input(input)?;
    let input = validated.input;
    let context = input.context;
    let mapped_mirror_ids = &validated.mapped_mirror_ids;

    if validated.removes_paired_matrices {
        let mut draft = input.post_source.clone();
        // A matrix-level Local mirror survives its source removal as a
        // complete standalone matrix (spec §5): the mirror record, its
        // members, and their associations are all retained. A matrix-level
        // Omitted mirror is already absent and stays that way.
        let matrix_retained_local = context.matrix_state() == TransitionState::Local;
        let matrix_omitted = context.matrix_state() == TransitionState::Omitted;
        let mirror_absent = draft
            .matrices
            .iter()
            .all(|matrix| matrix.id != matrix_pair.mirror_id.as_str());
        // The source removal itself is the change when the mirror is
        // retained or already absent; only an executed mirror removal must
        // report its own changes below.
        let skip_mirror_removal = matrix_retained_local || (matrix_omitted && mirror_absent);
        let mirror_removal = if skip_mirror_removal {
            Vec::new()
        } else {
            let retained_local_parts: BTreeSet<_> = input
                .mirror_identity_map
                .iter()
                .filter_map(|(old, new)| new.as_ref().map(|_| old.clone()))
                .collect();
            if !retained_local_parts.is_empty() {
                let mirror = draft
                    .matrices
                    .iter_mut()
                    .find(|matrix| matrix.id == matrix_pair.mirror_id.as_str())
                    .ok_or_else(|| {
                        "The target matrix disappeared before Local detachment".to_owned()
                    })?;
                mirror
                    .part_ids
                    .retain(|id| !retained_local_parts.contains(id));
            }
            (input.dispatch.apply_unscripted)(
                &mut draft,
                &crate::model::EditOperation::RemoveMatrix {
                    id: matrix_pair.mirror_id.clone(),
                },
            )?
        };
        let retires_member = |association: &TransitionAssociation| {
            let retained_local = association.state == TransitionState::Local
                && input
                    .mirror_identity_map
                    .get(&association.mirror_id)
                    .is_some_and(Option::is_some);
            !retained_local
                && (association.kind == MirroredItemKind::Matrix
                    || validated
                        .before_source_members
                        .contains(&association.source_id)
                    || validated
                        .before_mirror_members
                        .contains(&association.mirror_id))
        };
        let mut next_context = context.clone();
        // A source removal over a detached mirror retires every record of
        // the pair: with the source gone, the halves record cannot excuse
        // surviving correspondences, so all of them go. A retained Local
        // mirror keeps its record and members but loses its links, landing
        // as a plain standalone matrix; an Omitted mirror stays absent
        // without resurrection. Only defunct links retire — the defunct
        // source-layout link already retired before the kernel.
        let (retired, retained) = context
            .associations
            .iter()
            .cloned()
            .partition(|association| {
                matrix_omitted || matrix_retained_local || retires_member(association)
            });
        next_context.associations = retained;
        next_context.owners = input.final_owners.clone();
        let context_delta = MembershipContextDelta {
            retired,

            ..MembershipContextDelta::default()
        };
        let expected_mirror_members = if matrix_retained_local {
            validated.mapped_mirror_ids.clone()
        } else {
            BTreeSet::new()
        };
        let output = emit_validated_transition(
            &validated,
            draft,
            next_context,
            context_delta,
            &expected_mirror_members,
        )?;
        if !skip_mirror_removal && mirror_removal.is_empty() {
            return Err("The paired mirror matrix removal produced no document changes".into());
        }
        return Ok(output);
    }

    let state = context.matrix_state();
    let matrix_detached_local = state == TransitionState::Local;
    let matrix_detached_omitted = state == TransitionState::Omitted;
    let detached = matrix_detached_local || matrix_detached_omitted;
    if detached && !input.planned_additions.is_empty() {
        return Err("A detached mirror takes no projected additions".into());
    }
    let removal = membership_removal(input, axis, detached)?;
    let compacts_axis = matches!(removal, MembershipRemoval::Axis(_));

    let mut staged = input.post_source.clone();
    // An Omitted-absent mirror has no record to stage or project onto: the
    // per-cell pass and write-back below are skipped with it.
    let mirror_absent = matrix_detached_omitted
        && staged
            .matrices
            .iter()
            .all(|matrix| matrix.id != matrix_pair.mirror_id.as_str());
    // A detached Local mirror owns its descriptor, and an absent mirror has
    // no record at all: membership staging never rewrites either, whatever
    // the source operation removed or compacted.
    if let MembershipRemoval::Axis(axis) = removal
        && !matrix_detached_local
        && !mirror_absent
    {
        let (axis_row, removed_index) = match axis {
            RemovedAxis::Row(row) => (true, row),
            RemovedAxis::Column(column) => (false, column),
        };
        crate::matrix::deletion::remove_axis(
            &mut staged,
            matrix_pair.mirror_id.as_str(),
            removed_index,
            axis_row,
            input.dispatch.apply_unscripted,
        )?;
    } else if let MembershipRemoval::Cells(mut removed_cells) = removal
        && !matrix_detached_local
        && !mirror_absent
    {
        let old_mirror = transition_matrix_by_id(&staged, matrix_pair.mirror_id.as_str())?;
        if removed_cells.is_empty() {
            return Err("The trimmed membership removes no complete matrix key".into());
        }
        if removed_cells
            .iter()
            .any(|(row, column)| *row >= old_mirror.rows || *column >= old_mirror.columns)
        {
            return Err(
                "The trim operation removes an identity outside the old target matrix".into(),
            );
        }
        let local_removed_cells: BTreeSet<_> = context
            .associations
            .iter()
            .filter(|association| {
                association.kind == MirroredItemKind::Part
                    && association.state == TransitionState::Local
                    && input.source_identity_map.get(&association.source_id) == Some(&None)
                    && input
                        .mirror_identity_map
                        .get(&association.mirror_id)
                        .is_some_and(Option::is_some)
            })
            .filter_map(|association| {
                matrix_identity_slot(&association.mirror_id, matrix_pair.mirror_id.as_str())
                    .map(|(row, column, _)| (row, column))
            })
            .collect();
        if !local_removed_cells.is_empty() {
            let target = staged
                .matrices
                .iter_mut()
                .find(|matrix| matrix.id == matrix_pair.mirror_id.as_str())
                .ok_or_else(|| {
                    "The target matrix disappeared before Local detachment".to_owned()
                })?;
            let protected_ids: BTreeSet<_> = target
                .part_ids
                .iter()
                .filter(|id| {
                    matrix_identity_slot(id, matrix_pair.mirror_id.as_str()).is_some_and(
                        |(row, column, _)| local_removed_cells.contains(&(row, column)),
                    )
                })
                .cloned()
                .collect();
            target.part_ids.retain(|id| !protected_ids.contains(id));
            for cell in &mut target.cells {
                if local_removed_cells.contains(&(cell.row, cell.column)) {
                    cell.deleted = true;
                    cell.enabled = false;
                    cell.assemblies.clear();
                    cell.assemblies_local = None;
                }
            }
            removed_cells.retain(|cell| !local_removed_cells.contains(cell));
        }
        if !removed_cells.is_empty() {
            crate::matrix::deletion::remove_cells(
                &mut staged,
                matrix_pair.mirror_id.as_str(),
                &removed_cells.into_iter().collect::<Vec<_>>(),
            )?;
        }
    }
    let mirror = staged
        .matrices
        .iter()
        .find(|m| m.id == matrix_pair.mirror_id)
        .cloned();
    let projection = if matrix_detached_local || mirror_absent {
        None
    } else {
        let mirror = mirror.as_ref().ok_or("The target matrix is unavailable")?;
        Some(super::matrix_membership::slot_projection(
            &staged,
            input.valid_before,
            context,
            mirror,
            |id| {
                context.associations.iter().find(|a| {
                    a.kind == MirroredItemKind::Part
                        && input
                            .source_identity_map
                            .get(&a.source_id)
                            .and_then(Option::as_deref)
                            == Some(id)
                })
            },
            |id| input.planned_additions.iter().any(|a| a.source_id == id),
        )?)
    };
    let mut expected_final_mirror_members = mapped_mirror_ids.clone();
    expected_final_mirror_members.extend(
        input
            .planned_additions
            .iter()
            .map(|addition| addition.mirror_id.clone()),
    );
    let locally_detached_ids: BTreeSet<_> = context
        .associations
        .iter()
        .filter(|association| {
            association.kind == MirroredItemKind::Part
                && association.state == TransitionState::Local
                && input.source_identity_map.get(&association.source_id) == Some(&None)
        })
        .filter_map(|association| {
            input
                .mirror_identity_map
                .get(&association.mirror_id)
                .and_then(Clone::clone)
        })
        .collect();
    for id in &locally_detached_ids {
        expected_final_mirror_members.remove(id);
    }
    let removed_target_ids: BTreeSet<_> = mirror
        .iter()
        .flat_map(|mirror| mirror.part_ids.iter())
        .filter(|id| {
            !expected_final_mirror_members.contains(*id) && !locally_detached_ids.contains(*id)
        })
        .cloned()
        .collect();
    let _pruned_ids = crate::matrix::deletion::remove_parts_by_id(&mut staged, &removed_target_ids);
    // A detached Local mirror reuses its owned descriptor verbatim, and an
    // absent mirror has no record to write: in both cases the projection
    // write-back is skipped (the prune set above is empty here).
    if let Some((projected, omitted)) = projection {
        // The axis setter has already moved staged Parts into their new slots.
        // Only the compensated descriptor can supply their final Local poses.
        let retained = if compacts_axis {
            Vec::new()
        } else {
            context
                .associations
                .iter()
                .filter(|a| a.kind == MirroredItemKind::Part && a.state == TransitionState::Local)
                .filter_map(|a| input.mirror_identity_map.get(&a.mirror_id)?.as_ref())
                .filter_map(|id| staged.parts.iter().find(|p| p.id == *id).cloned())
                .collect()
        };
        super::matrix_membership::materialize_slots(&mut staged, &projected, &omitted, retained)?;
    }
    deduplicate_outline_members(&mut staged);
    for addition in input.planned_additions {
        let source_part = input
            .post_source
            .parts
            .iter()
            .find(|part| part.id == addition.source_id)
            .ok_or_else(|| {
                format!(
                    "The planned source part {:?} disappeared before materialization",
                    addition.source_id
                )
            })?;
        let mirror_part = staged
            .parts
            .iter_mut()
            .find(|part| part.id == addition.mirror_id)
            .ok_or_else(|| {
                format!(
                    "The planned mirror part {:?} was not generated",
                    addition.mirror_id
                )
            })?;
        mirror_part.reference = addition.mirror_reference.clone();
        mirror_part.locked = source_part.locked;
        mirror_part.keycap = source_part.keycap;
        mirror_part.outline = source_part.outline.clone();
        mirror_part.properties = source_part.properties.clone();
        mirror_part.generator_parameters = source_part.generator_parameters.clone();
    }

    let mut next_context = context.clone();
    let mut next_associations = Vec::with_capacity(context.associations.len());
    let mut delta = MembershipContextDelta::default();
    for association in &context.associations {
        if association.kind != MirroredItemKind::Part {
            next_associations.push(association.clone());
            continue;
        }
        let mapped_source = input
            .source_identity_map
            .get(&association.source_id)
            .ok_or_else(|| {
                format!(
                    "The transition map omits associated source identity {:?}",
                    association.source_id
                )
            })?;
        let Some(mapped_source) = mapped_source else {
            if !input
                .explicit_removal_closure
                .contains(&association.source_id)
            {
                return Err(
                    "A removed source association lacks explicit operation evidence".into(),
                );
            }
            let mapped_mirror = input.mirror_identity_map.get(&association.mirror_id);
            if association.state == TransitionState::Omitted && mapped_mirror.is_none() {
                if !input
                    .explicit_removal_closure
                    .contains(&association.mirror_id)
                {
                    return Err(
                        "A removed Omitted association lacks its private closure evidence".into(),
                    );
                }
                delta.retired.push(association.clone());
                continue;
            }
            let mapped_mirror = mapped_mirror.ok_or_else(|| {
                format!("The mirror identity map omits {:?}", association.mirror_id)
            })?;
            if association.state == TransitionState::Local
                && let Some(mapped_mirror) = mapped_mirror
            {
                if input.final_owners.get(mapped_mirror)
                    != Some(&FinalOwner::Board(context.mirror_board_id.clone()))
                {
                    return Err(format!(
                        "Retained Local identity {mapped_mirror:?} has no final owner"
                    ));
                }
                // The visible Local part survives independently; its deleted source no longer
                // owns a durable association or private transition record.
                delta.retired.push(association.clone());
                continue;
            }
            if !input
                .explicit_removal_closure
                .contains(&association.mirror_id)
            {
                return Err("A removed association lacks explicit target closure evidence".into());
            }
            delta.retired.push(association.clone());
            continue;
        };
        let mapped_mirror_default = mapped_source
            .strip_prefix(&format!("matrix/{}/", matrix_pair.source_id.as_str()))
            .map(|suffix| format!("matrix/{}/{suffix}", matrix_pair.mirror_id.as_str()))
            .ok_or_else(|| "A mapped source identity is outside its matrix".to_owned())?;
        if association.state == TransitionState::Omitted {
            if input
                .mirror_identity_map
                .get(&association.mirror_id)
                .is_some_and(Option::is_some)
            {
                return Err(format!(
                    "The omitted mirror identity {:?} remains materialized",
                    association.mirror_id
                ));
            }
            let mut omitted = association.clone();
            omitted.source_id = mapped_source.clone();
            omitted.mirror_id = mapped_mirror_default;
            omitted.parent_source_id = association
                .parent_source_id
                .as_ref()
                .and_then(|id| input.source_identity_map.get(id))
                .and_then(Clone::clone);
            omitted.parent_mirror_id = omitted.parent_source_id.as_ref().map(|id| {
                id.strip_prefix(&format!("matrix/{}/", matrix_pair.source_id.as_str()))
                    .map(|suffix| format!("matrix/{}/{suffix}", matrix_pair.mirror_id.as_str()))
                    .unwrap_or_else(|| id.clone())
            });
            next_associations.push(omitted);
            continue;
        }
        let mapped_mirror = input
            .mirror_identity_map
            .get(&association.mirror_id)
            .and_then(Clone::clone)
            .unwrap_or(mapped_mirror_default);
        let mut relocated = association.clone();
        relocated.source_id = mapped_source.clone();
        relocated.mirror_id = mapped_mirror.clone();
        relocated.parent_source_id = association
            .parent_source_id
            .as_ref()
            .and_then(|id| input.source_identity_map.get(id))
            .and_then(Clone::clone);
        relocated.parent_mirror_id = relocated.parent_source_id.as_ref().map(|id| {
            id.strip_prefix(&format!("matrix/{}/", matrix_pair.source_id.as_str()))
                .map(|suffix| format!("matrix/{}/{suffix}", matrix_pair.mirror_id.as_str()))
                .unwrap_or_else(|| id.clone())
        });
        if relocated != *association {
            delta
                .relocated
                .push((association.source_id.clone(), mapped_source.clone()));
        }
        next_associations.push(relocated);
    }
    for addition in input.planned_additions {
        let association = TransitionAssociation {
            kind: MirroredItemKind::Part,
            source_id: addition.source_id.clone(),
            mirror_id: addition.mirror_id.clone(),
            parent_source_id: addition.parent_source_id.clone(),
            parent_mirror_id: addition.parent_mirror_id.clone(),
            state: TransitionState::Linked,
            explicit_state: false,
            raw_mode_override: None,
            raw_tweak: None,
            effective_mode: context.pair_right.companion_mode,
            effective_tweak: None,
        };
        delta.added.push(association.clone());
        next_associations.push(association);
    }
    next_context.associations = next_associations;
    next_context.owners = input.final_owners.clone();
    deduplicate_outline_members(&mut staged);
    emit_validated_transition(
        &validated,
        staged,
        next_context,
        delta,
        &expected_final_mirror_members,
    )
}

enum MembershipRemoval {
    Axis(RemovedAxis),
    Cells(BTreeSet<(u32, u32)>),
    None,
}

fn membership_removal(
    input: MembershipTransitionInput<'_>,
    axis: Option<RemovedAxis>,
    detached: bool,
) -> Result<MembershipRemoval, String> {
    let matrix = &require_matrix_pair(input.context)?.source_id;
    let slot = |id| {
        matrix_identity_slot(id, matrix)
            .ok_or_else(|| format!("The source identity {id:?} is not canonical"))
    };
    if let Some(axis) = axis {
        if !input.planned_additions.is_empty() {
            return Err("Axis removal cannot introduce members".into());
        }
        // A trailing removal may detach a Local member as an independent part.
        // The cell trim below protects it from the destructive matrix setter.
        let detaches_local = input.context.associations.iter().any(|a| {
            a.state == TransitionState::Local
                && input.source_identity_map.get(&a.source_id) == Some(&None)
        });
        if !detaches_local
            || input
                .source_identity_map
                .iter()
                .any(|(old, new)| new.as_ref().is_some_and(|new| new != old))
        {
            return Ok(MembershipRemoval::Axis(axis));
        }
    }
    let mut removed_cells = BTreeSet::new();
    let mut removes_members = false;
    for (old, _) in input
        .source_identity_map
        .iter()
        .filter(|(_, new)| new.is_none())
    {
        let (row, column, child) = slot(old)?;
        removes_members = true;
        if child.is_empty() {
            removed_cells.insert((row, column));
        }
    }
    if removes_members {
        if !input.planned_additions.is_empty() {
            return Err("A compaction or trim cannot introduce members".into());
        }
        Ok(MembershipRemoval::Cells(removed_cells))
    } else if detached || !input.planned_additions.is_empty() {
        Ok(MembershipRemoval::None)
    } else {
        Err("The source operation does not describe a membership transition".into())
    }
}

pub(super) fn prepare_membership_projection(
    input: MembershipTransitionInput<'_>,
    axis: Option<RemovedAxis>,
) -> Result<MembershipTransitionOutput, String> {
    let output = project_membership(input, axis)?;
    lifecycle::apply_membership_context_delta(
        input.valid_before,
        input.post_source,
        input.context,
        output,
    )
}
