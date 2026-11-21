//! The membership-transition kernel and its public preparation entry point.

use super::{
    FinalOwner, MembershipContextDelta, MembershipTransitionInput, MembershipTransitionOutput,
    TransitionAssociation, TransitionState, cell_offset_delta, deduplicate_outline_members,
    emit_validated_transition, lifecycle, matrix_identity_slot, project_matrix,
    remove_omitted_matrix_parts, require_matrix_pair, transition_matrix_by_id,
    validate_transition_input,
};
use crate::model::{Matrix, MirroredItemKind, Vec2};
use std::collections::BTreeSet;

#[doc(hidden)]
pub fn prepare_membership_transition_kernel(
    input: MembershipTransitionInput<'_>,
) -> Result<MembershipTransitionOutput, String> {
    let mut output = project_membership(input)?;
    (output.changed_document_ids, output.changed_field_paths) =
        super::changed_document_fields(input.post_source, &output.draft)?;
    Ok(output)
}

pub(super) fn project_membership(
    input: MembershipTransitionInput<'_>,
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
        let matrix_retained_local = context.associations.iter().any(|association| {
            association.kind == MirroredItemKind::Matrix
                && association.source_id == matrix_pair.source_id.as_str()
                && association.mirror_id == matrix_pair.mirror_id.as_str()
                && association.state == TransitionState::Local
                && association.explicit_state
        });
        let matrix_omitted = context.associations.iter().any(|association| {
            association.kind == MirroredItemKind::Matrix
                && association.source_id == matrix_pair.source_id.as_str()
                && association.mirror_id == matrix_pair.mirror_id.as_str()
                && association.state == TransitionState::Omitted
                && association.explicit_state
        });
        let mirror_absent = draft
            .matrices
            .iter()
            .all(|matrix| matrix.id != matrix_pair.mirror_id.as_str());
        // The source removal itself is the change when the mirror is
        // retained or already absent; only an executed mirror removal must
        // report its own changes below.
        let skip_mirror_removal =
            validated.already_removed || matrix_retained_local || (matrix_omitted && mirror_absent);
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
        let removed_sources: BTreeSet<_> = context
            .associations
            .iter()
            .filter(|association| {
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
            })
            .map(|association| association.source_id.clone())
            .collect();
        let removed_mirrors: BTreeSet<_> = context
            .associations
            .iter()
            .filter(|association| {
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
            })
            .map(|association| association.mirror_id.clone())
            .collect();
        let mut next_context = context.clone();
        // A source removal over a detached mirror retires every record of
        // the pair: with the source gone, the halves record cannot excuse
        // surviving correspondences, so all of them go. A retained Local
        // mirror keeps its record and members but loses its links, landing
        // as a plain standalone matrix; an Omitted mirror stays absent
        // without resurrection. Only defunct links retire — the defunct
        // source-layout link already retired before the kernel.
        let (retired, retained): (Vec<_>, Vec<_>) = if matrix_omitted || matrix_retained_local {
            (next_context.associations.clone(), Vec::new())
        } else {
            let retired: Vec<_> = next_context
                .associations
                .iter()
                .filter(|association| {
                    removed_sources.contains(&association.source_id)
                        && removed_mirrors.contains(&association.mirror_id)
                })
                .cloned()
                .collect();
            let retained: Vec<_> = next_context
                .associations
                .iter()
                .filter(|association| {
                    !removed_sources.contains(&association.source_id)
                        || !removed_mirrors.contains(&association.mirror_id)
                })
                .cloned()
                .collect();
            (retired, retained)
        };
        next_context.associations = retained;
        next_context.owners = input.final_owners.clone();
        let context_delta = MembershipContextDelta {
            retired,
            owners: input.final_owners.clone(),
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

    let is_identity = input.valid_before == input.post_source
        && input
            .source_identity_map
            .iter()
            .all(|(old, new)| new.as_deref() == Some(old))
        && input
            .mirror_identity_map
            .iter()
            .all(|(old, new)| new.as_deref() == Some(old))
        && validated.before_source_members == validated.mapped_source_ids
        && validated.before_mirror_members == validated.mapped_mirror_ids
        && input.planned_additions.is_empty()
        && input.explicit_removal_closure.is_empty()
        && context.pending_rejoins.is_empty()
        && context.pending_pair_transitions.is_empty();
    if is_identity {
        if context.owners != *input.final_owners {
            return Err("An identity transition changes its bound owner context".into());
        }
        let next_context = context.clone();
        let context_delta = MembershipContextDelta {
            owners: input.final_owners.clone(),
            ..MembershipContextDelta::default()
        };
        let expected_mirror_members = validated.mapped_mirror_ids.clone();
        return emit_validated_transition(
            &validated,
            input.post_source.clone(),
            next_context,
            context_delta,
            &expected_mirror_members,
        );
    }

    let mut removed_slots = BTreeSet::new();
    let mut shifted_slots = BTreeSet::new();
    for (old_id, new_id) in input.source_identity_map {
        let Some((old_row, old_column, old_suffix)) =
            matrix_identity_slot(old_id, matrix_pair.source_id.as_str())
        else {
            return Err(format!("The source identity {old_id:?} is not canonical"));
        };
        match new_id {
            None => {
                removed_slots.insert((old_row, old_column, old_suffix.to_owned()));
                if !input.explicit_removal_closure.contains(old_id) {
                    return Err(format!(
                        "Removed source identity {old_id:?} lacks explicit operation evidence"
                    ));
                }
            }
            Some(new_id) => {
                let Some((new_row, new_column, new_suffix)) =
                    matrix_identity_slot(new_id, matrix_pair.source_id.as_str())
                else {
                    return Err(format!(
                        "The mapped source identity {new_id:?} is not canonical"
                    ));
                };
                if old_suffix != new_suffix {
                    return Err("The source identity map changes a companion identity".into());
                }
                if (old_row, old_column) != (new_row, new_column) {
                    shifted_slots.insert(((old_row, old_column), (new_row, new_column)));
                }
            }
        }
    }
    let is_compaction = !removed_slots.is_empty()
        && !shifted_slots.is_empty()
        && input.planned_additions.is_empty();
    let is_trim =
        !removed_slots.is_empty() && shifted_slots.is_empty() && input.planned_additions.is_empty();
    let is_expansion =
        removed_slots.is_empty() && shifted_slots.is_empty() && !input.planned_additions.is_empty();
    let is_rejoin = removed_slots.is_empty()
        && shifted_slots.is_empty()
        && input.planned_additions.is_empty()
        && (!context.pending_rejoins.is_empty() || !context.pending_pair_transitions.is_empty());
    // A detached mirror owns its membership (spec §5): source-side growth
    // projects nothing, so the map carries no additions for the kernel to
    // materialize. The map check above already proved the residual change is
    // genuine source growth; every other arm keeps its shape. Only a Local
    // mirror skips projection below; an Omitted mirror still follows the
    // source record while staying absent.
    let matrix_detached = context.pending_rejoins.is_empty()
        && context.pending_pair_transitions.is_empty()
        && context.associations.iter().any(|association| {
            association.kind == MirroredItemKind::Matrix
                && association.source_id == matrix_pair.source_id.as_str()
                && association.mirror_id == matrix_pair.mirror_id.as_str()
                && matches!(
                    association.state,
                    TransitionState::Local | TransitionState::Omitted
                )
                && association.explicit_state
        });
    let matrix_detached_local = context.pending_rejoins.is_empty()
        && context.pending_pair_transitions.is_empty()
        && context.associations.iter().any(|association| {
            association.kind == MirroredItemKind::Matrix
                && association.source_id == matrix_pair.source_id.as_str()
                && association.mirror_id == matrix_pair.mirror_id.as_str()
                && association.state == TransitionState::Local
                && association.explicit_state
        });
    let matrix_detached_omitted = context.pending_rejoins.is_empty()
        && context.pending_pair_transitions.is_empty()
        && context.associations.iter().any(|association| {
            association.kind == MirroredItemKind::Matrix
                && association.source_id == matrix_pair.source_id.as_str()
                && association.mirror_id == matrix_pair.mirror_id.as_str()
                && association.state == TransitionState::Omitted
                && association.explicit_state
        });
    if matrix_detached && !input.planned_additions.is_empty() {
        return Err("A detached mirror takes no projected additions".into());
    }
    let is_detached_expansion = removed_slots.is_empty()
        && shifted_slots.is_empty()
        && input.planned_additions.is_empty()
        && (matrix_detached_local || matrix_detached_omitted);
    if !is_compaction && !is_trim && !is_expansion && !is_rejoin && !is_detached_expansion {
        return Err("The source operation is not a supported identity, trim, expansion, or single-axis compaction".into());
    }
    let compaction_axis = if is_compaction {
        Some({
            let &(removed_row, removed_column, _) = removed_slots
                .iter()
                .next()
                .ok_or_else(|| "The source operation has no removed slot".to_owned())?;
            let row_axis =
                shifted_slots
                    .iter()
                    .all(|((old_row, old_column), (new_row, new_column))| {
                        old_column == new_column && old_row != new_row
                    });
            let column_axis =
                shifted_slots
                    .iter()
                    .all(|((old_row, old_column), (new_row, new_column))| {
                        old_row == new_row && old_column != new_column
                    });
            if row_axis == column_axis {
                return Err(
                    "The source identity map does not describe one unambiguous axis compaction"
                        .into(),
                );
            }
            let index = if row_axis {
                removed_row
            } else {
                removed_column
            };
            if removed_slots.iter().any(|(row, column, _)| {
                if row_axis {
                    *row != index
                } else {
                    *column != index
                }
            }) {
                return Err("The source operation removes more than one matrix axis".into());
            }
            for (old, new) in &shifted_slots {
                let (old_axis, new_axis, other_axis_equal) = if row_axis {
                    (old.0, new.0, old.1 == new.1)
                } else {
                    (old.1, new.1, old.0 == new.0)
                };
                if !other_axis_equal
                    || old_axis == index
                    || new_axis != old_axis - u32::from(old_axis > index)
                {
                    return Err("The source identity map is not a canonical axis compaction".into());
                }
            }
            (row_axis, index)
        })
    } else {
        None
    };

    let mut staged = input.post_source.clone();
    // An Omitted-absent mirror has no record to stage or project onto: the
    // per-cell pass and write-back below are skipped with it.
    let mirror_absent = matrix_detached_omitted
        && staged
            .matrices
            .iter()
            .all(|matrix| matrix.id != matrix_pair.mirror_id.as_str());
    for association in context.associations.iter().filter(|association| {
        association.kind == MirroredItemKind::Part
            && context.pending_rejoins.contains_key(&association.source_id)
            && association.state == TransitionState::Linked
    }) {
        let mut parts = staged
            .parts
            .iter_mut()
            .filter(|part| part.id == association.mirror_id);
        let Some(target) = parts.next() else {
            if context.pending_rejoins.get(&association.source_id)
                == Some(&TransitionState::Omitted)
            {
                // The matrix setter below materializes this authenticated absent target.
                continue;
            }
            return Err(format!(
                "The Rejoined mirror Part {:?} is unavailable",
                association.mirror_id
            ));
        };
        if parts.next().is_some() {
            return Err(format!(
                "The Rejoined mirror Part {:?} is ambiguous",
                association.mirror_id
            ));
        }
        if let Some(properties) = &mut target.properties {
            properties.remove("layoutOverride");
        }
    }
    // A detached Local mirror owns its descriptor, and an absent mirror has
    // no record at all: membership staging never rewrites either, whatever
    // the source operation removed or compacted.
    if let Some((axis_row, removed_index)) =
        compaction_axis.filter(|_| !matrix_detached_local && !mirror_absent)
    {
        crate::matrix::deletion::remove_axis(
            &mut staged,
            matrix_pair.mirror_id.as_str(),
            removed_index,
            axis_row,
            input.dispatch.apply_unscripted,
        )?;
    } else if is_trim && !matrix_detached_local && !mirror_absent {
        let old_mirror = transition_matrix_by_id(&staged, matrix_pair.mirror_id.as_str())?;
        let mut removed_cells: BTreeSet<_> = removed_slots
            .iter()
            .filter_map(|(row, column, suffix)| suffix.is_empty().then_some((*row, *column)))
            .collect();
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
    let source = transition_matrix_by_id(&staged, matrix_pair.source_id.as_str())?.clone();
    let mirror: Option<Matrix> = staged
        .matrices
        .iter()
        .find(|matrix| matrix.id == matrix_pair.mirror_id.as_str())
        .cloned();
    if mirror.is_none() && !mirror_absent {
        return Err(format!(
            "The transition matrix {:?} is unavailable",
            matrix_pair.mirror_id
        ));
    }
    // Detached Local: the mirror owns its descriptor and geometry, so the
    // projection below is skipped and the owned descriptor is reused verbatim.
    // An absent mirror likewise skips everything: the placeholder below never
    // reaches the write-back. (The per-cell loop carries a no-op filter to
    // keep this diff flat; the preservation pass stays empty, so no
    // compensation dirties the mirror.)
    let mut projected = mirror.clone().unwrap_or_else(|| source.clone());
    let mut local_key_preservations = Vec::new();
    let mut omitted_target_parts = BTreeSet::new();
    if !matrix_detached_local && !mirror_absent {
        let matrix_correspondence = context
            .pair_right
            .correspondences
            .iter()
            .find(|association| {
                association.kind == MirroredItemKind::Matrix
                    && association.source_id == matrix_pair.source_id.as_str()
                    && association.mirror_id == matrix_pair.mirror_id.as_str()
            })
            .ok_or_else(|| "The transition matrix association is unavailable".to_owned())?;
        let present = mirror.clone().ok_or_else(|| {
            format!(
                "The transition matrix {:?} is unavailable",
                matrix_pair.mirror_id
            )
        })?;
        projected = project_matrix(&source, &present, matrix_correspondence, context.mapping)?;
    }
    // The loop only executes with a present mirror (see its filter); the owned
    // snapshot keeps the body total without re-proving presence per cell.
    let mirror_cells: Vec<_> = mirror
        .iter()
        .flat_map(|present| present.cells.iter().cloned())
        .collect();
    for cell in projected
        .cells
        .iter_mut()
        .filter(|_| !matrix_detached_local && !mirror_absent)
    {
        if !cell.enabled || cell.deleted {
            continue;
        }
        let source_key_id = format!(
            "matrix/{}/r{}c{}",
            matrix_pair.source_id.as_str(),
            cell.row,
            cell.column
        );
        let Some(source_key) = staged.parts.iter().find(|part| part.id == source_key_id) else {
            return Err(format!(
                "The projected source key {source_key_id:?} is unavailable"
            ));
        };
        let original_key = context.associations.iter().find(|association| {
            association.kind == MirroredItemKind::Part
                && input.source_identity_map.get(&association.source_id)
                    == Some(&Some(source_key_id.clone()))
        });
        if original_key.is_none() && matrix_detached_omitted {
            // An Omitted mirror follows the source record without
            // materializing: novel slots stay disabled with no companions, so
            // later member accounting sees a canonical absent mirror.
            cell.deleted = true;
            cell.enabled = false;
            cell.assemblies.clear();
            cell.assemblies_local = None;
            continue;
        }
        if original_key.is_some_and(|association| association.state == TransitionState::Omitted) {
            cell.deleted = true;
            cell.enabled = false;
            cell.assemblies.clear();
            cell.assemblies_local = None;
            continue;
        }
        if original_key.is_some_and(|association| association.state == TransitionState::Local) {
            let local_cell = mirror_cells
                .iter()
                .find(|candidate| candidate.row == cell.row && candidate.column == cell.column)
                .ok_or_else(|| {
                    format!(
                        "The retained Local matrix cell r{}c{} is unavailable",
                        cell.row, cell.column
                    )
                })?;
            let association = original_key.expect("the Local key association was checked");
            let mapped_source = input
                .source_identity_map
                .get(&association.source_id)
                .and_then(Clone::clone)
                .ok_or_else(|| {
                    "The retained Local key has no source identity mapping".to_owned()
                })?;
            let target_id = input
                .mirror_identity_map
                .get(&association.mirror_id)
                .and_then(Clone::clone)
                .or_else(|| {
                    mapped_source
                        .strip_prefix(&format!("matrix/{}/", matrix_pair.source_id.as_str()))
                        .map(|suffix| format!("matrix/{}/{suffix}", matrix_pair.mirror_id.as_str()))
                })
                .ok_or_else(|| {
                    "The retained Local key has no target identity mapping".to_owned()
                })?;
            local_key_preservations.push((
                cell.row,
                cell.column,
                association.mirror_id.clone(),
                target_id,
            ));
            *cell = local_cell.clone();
            for assembly in &cell.assemblies {
                let source_child_id = crate::matrix::companion_id(&source_key_id, &assembly.id);
                if context.associations.iter().any(|child| {
                    child.kind == MirroredItemKind::Part
                        && input.source_identity_map.get(&child.source_id)
                            == Some(&Some(source_child_id.clone()))
                        && child.state == TransitionState::Omitted
                        && child.explicit_state
                }) {
                    omitted_target_parts.insert(crate::matrix::companion_id(
                        &format!(
                            "matrix/{}/r{}c{}",
                            matrix_pair.mirror_id.as_str(),
                            cell.row,
                            cell.column
                        ),
                        &assembly.id,
                    ));
                }
            }
            continue;
        }
        let mut next_assemblies = Vec::with_capacity(cell.assemblies.len());
        for mut assembly in std::mem::take(&mut cell.assemblies) {
            let source_child_id = crate::matrix::companion_id(&source_key_id, &assembly.id);
            let mirror_child_id = crate::matrix::companion_id(
                &format!(
                    "matrix/{}/r{}c{}",
                    matrix_pair.mirror_id.as_str(),
                    cell.row,
                    cell.column
                ),
                &assembly.id,
            );
            let original = context.associations.iter().find(|association| {
                association.kind == MirroredItemKind::Part
                    && input.source_identity_map.get(&association.source_id)
                        == Some(&Some(source_child_id.clone()))
            });
            if original.is_none()
                && !input
                    .planned_additions
                    .iter()
                    .any(|addition| addition.source_id == source_child_id)
            {
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
                let local_assembly = mirror_cells
                    .iter()
                    .find(|candidate| candidate.row == cell.row && candidate.column == cell.column)
                    .and_then(|candidate| {
                        candidate
                            .assemblies
                            .iter()
                            .find(|item| item.id == assembly.id)
                    })
                    .ok_or_else(|| {
                        format!("The retained Local companion {mirror_child_id:?} is unavailable")
                    })?;
                assembly = local_assembly.clone();
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
            let (offset, rotation) = super::super::derive::companion_assembly(
                super::super::derive::CompanionAssemblyInput {
                    offset: assembly.offset,
                    rotation: assembly.rotation,
                    key: Some(source_key),
                    companion: Some(source_companion),
                    mapping: context.mapping,
                    mode,
                    tweak,
                    stabiliser: super::super::SYMMETRIC_DEFINITIONS
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
        crate::matrix::set_matrix(&mut candidate, &projected)?;
        for (row, column, old_id, target_id) in &local_key_preservations {
            let before_part = input
                .valid_before
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
    // Detached Local keeps the mapped (identity) members: rejoins cannot be
    // pending here by construction, and additions are refused above, so both
    // extensions below are naturally empty on the detached path.
    let mut expected_final_mirror_members = mapped_mirror_ids.clone();
    expected_final_mirror_members.extend(context.pending_rejoins.iter().filter_map(
        |(source_id, previous_state)| {
            if *previous_state == TransitionState::Omitted {
                context
                    .associations
                    .iter()
                    .find(|association| {
                        association.kind == MirroredItemKind::Part
                            && association.source_id == *source_id
                            && association.state == TransitionState::Linked
                    })
                    .map(|association| association.mirror_id.clone())
            } else {
                None
            }
        },
    ));
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
    if !matrix_detached_local && !mirror_absent {
        crate::matrix::set_matrix(&mut staged, &projected)?;
    }
    remove_omitted_matrix_parts(
        &mut staged,
        matrix_pair.mirror_id.as_str(),
        &omitted_target_parts,
    );
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
    let mut delta = MembershipContextDelta {
        owners: input.final_owners.clone(),
        ..MembershipContextDelta::default()
    };
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

#[doc(hidden)]
pub fn prepare_membership_transition(
    input: MembershipTransitionInput<'_>,
) -> Result<MembershipTransitionOutput, String> {
    if input.context.matrix_pair.is_none() {
        return Err("Matrix membership transition requires an exact Matrix correspondence".into());
    }
    let mut output = prepare_membership_projection(input)?;
    (output.changed_document_ids, output.changed_field_paths) =
        super::changed_document_fields(input.post_source, &output.draft)?;
    Ok(output)
}

pub(super) fn prepare_membership_projection(
    input: MembershipTransitionInput<'_>,
) -> Result<MembershipTransitionOutput, String> {
    let mut output = project_membership(input)?;
    // The membership kernel retains detached Local geometry. Once its source is explicitly
    // deleted, the durable lifecycle also retires that geometry's former association.
    let detached: Vec<_> = output
        .context
        .associations
        .iter()
        .filter(|association| {
            association.kind == MirroredItemKind::Part
                && association.state == TransitionState::Local
                && input.source_identity_map.get(&association.source_id) == Some(&None)
                && input
                    .explicit_removal_closure
                    .contains(&association.source_id)
                && !output
                    .draft
                    .parts
                    .iter()
                    .any(|part| part.id == association.source_id)
        })
        .cloned()
        .collect();
    for association in detached {
        output.context.associations.retain(|a| {
            !(a.kind == association.kind
                && a.source_id == association.source_id
                && a.mirror_id == association.mirror_id)
        });
        output.context_delta.retired.push(association);
    }
    lifecycle::apply_membership_context_delta(input, &mut output)?;
    Ok(output)
}
