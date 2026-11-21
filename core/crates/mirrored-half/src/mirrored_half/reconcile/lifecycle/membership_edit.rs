//! Lifecycle contexts, matrix pair association and matrix membership edits.

use super::super::{MembershipContext, TransitionState};
use super::effective;
use crate::model::{MirroredExceptionState, MirroredItemKind};

#[doc(hidden)]
pub fn lifecycle_context(
    document: &crate::model::ProjectDoc,
    pair_id: &str,
    matrix_pair: Option<super::super::MatrixPairContext>,
) -> Result<MembershipContext, String> {
    use super::super::{FinalOwner, TransitionAssociation};
    use crate::model::{MirroredExceptionState, MirroredItemKind};
    use std::collections::BTreeMap;

    let hardware = document
        .hardware
        .as_ref()
        .ok_or("Lifecycle hardware is unavailable")?;
    let mut pairs = hardware.pairs.iter().filter(|pair| pair.id == pair_id);
    let pair = pairs
        .next()
        .ok_or("The exact lifecycle pair is unavailable")?;
    if pairs.next().is_some() {
        return Err("The exact lifecycle pair is ambiguous".into());
    }
    let halves = pair
        .halves
        .as_ref()
        .ok_or("The exact lifecycle pair has no halves record")?;

    fn parent_ids(
        halves: &crate::model::MirroredHalves,
        record: &crate::model::MirroredCorrespondence,
    ) -> (Option<String>, Option<String>) {
        if record.kind == MirroredItemKind::Part {
            for matrix in halves
                .correspondences
                .iter()
                .filter(|item| item.kind == MirroredItemKind::Matrix)
            {
                let Some(suffix) = record
                    .source_id
                    .strip_prefix(&format!("matrix/{}/", matrix.source_id))
                else {
                    continue;
                };
                let Some(mirror_suffix) = record
                    .mirror_id
                    .strip_prefix(&format!("matrix/{}/", matrix.mirror_id))
                else {
                    return (None, None);
                };
                if let Some((slot, _)) = suffix.split_once('/') {
                    let Some((mirror_slot, _)) = mirror_suffix.split_once('/') else {
                        return (None, None);
                    };
                    if slot == mirror_slot {
                        return (
                            Some(format!("matrix/{}/{}", matrix.source_id, slot)),
                            Some(format!("matrix/{}/{}", matrix.mirror_id, mirror_slot)),
                        );
                    }
                    return (None, None);
                }
                // A top-level key with an aligned slot inherits its matrix
                // pair's state (spec §5): the matrix correspondence is its
                // parent, so whole-matrix Local/Omitted transitions reach
                // every member without per-key exception records.
                if suffix == mirror_suffix {
                    return (
                        Some(matrix.source_id.clone()),
                        Some(matrix.mirror_id.clone()),
                    );
                }
                return (None, None);
            }
        }
        if matches!(record.kind, MirroredItemKind::Part | MirroredItemKind::Net) {
            for circuit in halves
                .correspondences
                .iter()
                .filter(|item| item.kind == MirroredItemKind::EmbeddedCircuit)
            {
                if record
                    .source_id
                    .strip_prefix(&format!("embedded/{}/", circuit.source_id))
                    .is_some()
                    && record
                        .mirror_id
                        .strip_prefix(&format!("embedded/{}/", circuit.mirror_id))
                        .is_some()
                {
                    return (
                        Some(circuit.source_id.clone()),
                        Some(circuit.mirror_id.clone()),
                    );
                }
            }
        }
        (None, None)
    }

    let associations = halves
        .correspondences
        .iter()
        .map(|record| {
            let exception = halves.exceptions.iter().find(|exception| {
                exception.kind == record.kind && exception.mirror_id == record.mirror_id
            });
            let state = match exception.map(|exception| exception.state) {
                Some(MirroredExceptionState::Local) => TransitionState::Local,
                Some(MirroredExceptionState::Omitted) => TransitionState::Omitted,
                None => TransitionState::Linked,
            };
            let (parent_source_id, parent_mirror_id) = parent_ids(halves, record);
            TransitionAssociation {
                kind: record.kind,
                source_id: record.source_id.clone(),
                mirror_id: record.mirror_id.clone(),
                parent_source_id,
                parent_mirror_id,
                state,
                explicit_state: exception.is_some(),
                raw_mode_override: record.companion_mode,
                raw_tweak: record.companion_tweak,
                effective_mode: record.companion_mode.unwrap_or(halves.companion_mode),
                effective_tweak: record.companion_tweak,
            }
        })
        .collect();
    let mut context = MembershipContext {
        pair_id: pair_id.to_owned(),
        source_board_id: halves.source_board_id.clone(),
        mirror_board_id: halves.mirror_board_id.clone(),
        matrix_pair,
        mapping: pair.mapping,
        pair_right: halves.clone(),
        associations,
        owners: BTreeMap::new(),
        pending_rejoins: BTreeMap::new(),
        pending_pair_transitions: BTreeMap::new(),
        pending_pair_baseline: None,
    };
    for index in 0..context.associations.len() {
        let state = effective(&context, &context.associations[index], false, 0)?;
        context.associations[index].state = state;
    }
    for association in &context.associations {
        if association.kind != MirroredItemKind::Part {
            continue;
        }
        let source_owner = super::super::super::board_for_part(document, &association.source_id)
            .map(|board| board.id.clone())
            .ok_or_else(|| {
                format!(
                    "Lifecycle source Part {:?} has no board owner",
                    association.source_id
                )
            })?;
        let state = effective(&context, association, false, 0)?;
        context.owners.insert(
            association.source_id.clone(),
            FinalOwner::Board(source_owner),
        );
        let target_owner = if state == TransitionState::Omitted {
            FinalOwner::Absent
        } else {
            FinalOwner::Board(
                super::super::super::board_for_part(document, &association.mirror_id)
                    .map(|board| board.id.clone())
                    .ok_or_else(|| {
                        format!(
                            "Lifecycle mirror Part {:?} has no board owner",
                            association.mirror_id
                        )
                    })?,
            )
        };
        context
            .owners
            .insert(association.mirror_id.clone(), target_owner);
    }
    // The membership kernel validates ownership for every materialized Part in the document,
    // including independent Parts and associations filtered out of a selected-matrix view.
    // Bind those historical owners too; association-specific absent mirror IDs above keep their
    // stronger `Absent` value.
    for part in &document.parts {
        if context.owners.contains_key(&part.id) {
            continue;
        }
        let board_id = super::super::super::board_for_part(document, &part.id)
            .map(|board| board.id.clone())
            .ok_or_else(|| format!("Lifecycle Part {:?} has no board owner", part.id))?;
        context
            .owners
            .insert(part.id.clone(), FinalOwner::Board(board_id));
    }
    Ok(context)
}

#[doc(hidden)]
pub fn matrix_pair_for_association(
    document: &crate::model::ProjectDoc,
    halves: &crate::model::MirroredHalves,
    kind: crate::model::MirroredItemKind,
    source_id: &str,
    mirror_id: &str,
) -> Result<Option<super::super::MatrixPairContext>, String> {
    if kind == MirroredItemKind::Matrix {
        return Ok(Some(super::super::MatrixPairContext {
            source_id: source_id.to_owned(),
            mirror_id: mirror_id.to_owned(),
        }));
    }
    if kind != crate::model::MirroredItemKind::Part {
        return Ok(None);
    }

    let matrix_pairs: Vec<_> = halves
        .correspondences
        .iter()
        .filter(|record| record.kind == MirroredItemKind::Matrix)
        .filter_map(|record| {
            let (row, column, suffix) =
                super::super::matrix_identity_slot(source_id, &record.source_id)?;
            let mirror_member = if suffix.is_empty() {
                format!("matrix/{}/r{row}c{column}", record.mirror_id)
            } else {
                format!("matrix/{}/r{row}c{column}/{suffix}", record.mirror_id)
            };
            (mirror_member == mirror_id).then(|| super::super::MatrixPairContext {
                source_id: record.source_id.clone(),
                mirror_id: record.mirror_id.clone(),
            })
        })
        .collect();
    let source_matrix_memberships: Vec<_> = document
        .matrices
        .iter()
        .filter(|matrix| matrix.part_ids.iter().any(|id| id == source_id))
        .collect();
    let mirror_matrix_memberships: Vec<_> = document
        .matrices
        .iter()
        .filter(|matrix| matrix.part_ids.iter().any(|id| id == mirror_id))
        .collect();
    if matrix_pairs.is_empty() {
        if source_matrix_memberships.is_empty() && mirror_matrix_memberships.is_empty() {
            return Ok(None);
        }
        return Err("A matrix-owned Part lacks its exact durable Matrix correspondence".into());
    }
    let [matrix_pair] = matrix_pairs.as_slice() else {
        return Err("A matrix-owned Part has an ambiguous durable Matrix owner".into());
    };
    let source_board_id = &halves.source_board_id;
    let mirror_board_id = &halves.mirror_board_id;
    let mut source_matrices = document
        .matrices
        .iter()
        .filter(|matrix| matrix.id == matrix_pair.source_id);
    let source_matrix = source_matrices
        .next()
        .ok_or("The exact source Matrix owner is unavailable")?;
    if source_matrices.next().is_some()
        || source_matrix.board_id.as_deref() != Some(source_board_id.as_str())
        || !source_matrix.part_ids.contains(&source_id.to_owned())
        || source_matrix_memberships.len() != 1
        || source_matrix_memberships[0].id != source_matrix.id
    {
        return Err("The source Part does not have exact Matrix membership".into());
    }
    let mut mirror_matrices = document
        .matrices
        .iter()
        .filter(|matrix| matrix.id == matrix_pair.mirror_id);
    let mirror_matrix = mirror_matrices.next();
    if mirror_matrices.next().is_some() {
        return Err("The exact mirror Matrix owner is ambiguous".into());
    }
    let omitted_owner = halves.exceptions.iter().any(|exception| {
        exception.kind == MirroredItemKind::Matrix
            && exception.mirror_id == matrix_pair.mirror_id
            && exception.state == MirroredExceptionState::Omitted
            && halves.overlay_source(exception) == Some(matrix_pair.source_id.as_str())
    });
    let omitted_part = halves.exceptions.iter().any(|exception| {
        exception.kind == MirroredItemKind::Part
            && exception.mirror_id == mirror_id
            && exception.state == MirroredExceptionState::Omitted
            && halves.overlay_source(exception) == Some(source_id)
    });
    if let Some(mirror_matrix) = mirror_matrix {
        if mirror_matrix.board_id.as_deref() != Some(mirror_board_id.as_str())
            || mirror_matrix_memberships
                .iter()
                .any(|matrix| matrix.id != mirror_matrix.id)
            || (!mirror_matrix.part_ids.iter().any(|id| id == mirror_id)
                && !omitted_owner
                && !omitted_part)
        {
            return Err("The mirror Part lacks actual membership in its exact Matrix".into());
        }
    } else if !omitted_owner {
        return Err("The exact mirror Matrix owner is unavailable".into());
    }
    Ok(Some(matrix_pair.clone()))
}

pub(super) fn prepare_matrix_membership_edit(
    valid_before: &crate::model::ProjectDoc,
    source_operation: &crate::model::ProjectDoc,
    operation: &crate::model::EditOperation,
    selected: &MembershipContext,
    dispatch: crate::mirrored_half::EditDispatch,
) -> Result<super::super::MembershipTransitionOutput, String> {
    use super::super::{
        BaselineKind, FinalOwner, MembershipTransitionInput, PlannedAddition, TransitionState,
        prepare_membership_transition,
    };
    use crate::model::{EditOperation as Op, MirroredItemKind};
    use std::collections::{BTreeMap, BTreeSet};

    let matrix_pair = super::super::require_matrix_pair(selected)?;

    fn selected_matrix_part(
        association: &crate::model::MirroredCorrespondence,
        source_matrix_id: &str,
        mirror_matrix_id: &str,
    ) -> bool {
        association.kind == MirroredItemKind::Part
            && association
                .source_id
                .starts_with(&format!("matrix/{source_matrix_id}/"))
            && association
                .mirror_id
                .starts_with(&format!("matrix/{mirror_matrix_id}/"))
    }

    fn selected_matrix_association(
        association: &crate::model::MirroredCorrespondence,
        source_matrix_id: &str,
        mirror_matrix_id: &str,
    ) -> bool {
        match association.kind {
            MirroredItemKind::Matrix => {
                association.source_id == source_matrix_id
                    && association.mirror_id == mirror_matrix_id
            }
            MirroredItemKind::Part => {
                selected_matrix_part(association, source_matrix_id, mirror_matrix_id)
            }
            _ => true,
        }
    }

    fn selected_matrix_exception(
        exception: &crate::model::MirroredException,
        mirror_matrix_id: &str,
    ) -> bool {
        match exception.kind {
            MirroredItemKind::Matrix => exception.mirror_id == mirror_matrix_id,
            MirroredItemKind::Part => exception
                .mirror_id
                .starts_with(&format!("matrix/{mirror_matrix_id}/")),
            _ => true,
        }
    }

    fn filter_other_matrix_parts(
        document: &mut crate::model::ProjectDoc,
        context: &MembershipContext,
    ) -> Result<(), String> {
        let hardware = document
            .hardware
            .as_mut()
            .ok_or("Transition hardware is unavailable")?;
        let mut pairs = hardware
            .pairs
            .iter_mut()
            .filter(|pair| pair.id == context.pair_id);
        let pair = pairs
            .next()
            .ok_or("The exact transition pair is unavailable")?;
        if pairs.next().is_some() {
            return Err("The exact transition pair is ambiguous".into());
        }
        let halves = pair
            .halves
            .as_mut()
            .ok_or("The exact transition pair has no halves record")?;
        let matrix_pair = super::super::require_matrix_pair(context)?;
        halves.correspondences.retain(|association| {
            selected_matrix_association(
                association,
                matrix_pair.source_id.as_str(),
                matrix_pair.mirror_id.as_str(),
            )
        });
        halves.exceptions.retain(|exception| {
            selected_matrix_exception(exception, matrix_pair.mirror_id.as_str())
        });
        Ok(())
    }

    let mut filtered_before = valid_before.clone();
    let mut filtered_post_source = source_operation.clone();
    filter_other_matrix_parts(&mut filtered_before, selected)?;
    filter_other_matrix_parts(&mut filtered_post_source, selected)?;

    // Matrix removal uses the membership kernel, which only projects Part rows. Retire the
    // exact linked Layout rows whose source endpoint this operation removed before projecting
    // that selected matrix slice; their mirror Layout remains in the draft as ordinary geometry.
    let retired_layouts: Vec<_> = selected
        .associations
        .iter()
        .filter(|association| {
            association.kind == MirroredItemKind::Layout
                && !source_operation
                    .layouts
                    .iter()
                    .any(|layout| layout.id == association.source_id)
        })
        .cloned()
        .collect();
    fn remove_retired_layout_rows(
        document: &mut crate::model::ProjectDoc,
        pair_id: &str,
        retired: &[super::super::TransitionAssociation],
    ) -> Result<(), String> {
        if retired.is_empty() {
            return Ok(());
        }
        let pair = document
            .hardware
            .as_mut()
            .ok_or("Transition hardware is unavailable")?
            .pairs
            .iter_mut()
            .find(|pair| pair.id == pair_id)
            .ok_or("The exact transition pair is unavailable")?;
        let halves = pair
            .halves
            .as_mut()
            .ok_or("The exact transition pair has no halves record")?;
        halves.correspondences.retain(|record| {
            !retired.iter().any(|association| {
                record.kind == association.kind
                    && record.source_id == association.source_id
                    && record.mirror_id == association.mirror_id
            })
        });
        halves.exceptions.retain(|exception| {
            !retired.iter().any(|association| {
                exception.kind == association.kind && exception.mirror_id == association.mirror_id
            })
        });
        Ok(())
    }
    remove_retired_layout_rows(&mut filtered_before, &selected.pair_id, &retired_layouts)?;
    remove_retired_layout_rows(
        &mut filtered_post_source,
        &selected.pair_id,
        &retired_layouts,
    )?;
    let filtered_context = lifecycle_context(
        &filtered_before,
        &selected.pair_id,
        selected.matrix_pair.clone(),
    )?;
    let before = &filtered_before;
    let post_source = &filtered_post_source;
    let context = &filtered_context;

    // A detached matrix-level mirror owns its membership (spec §5): source
    // growth stays source-only, so no mirror additions are planned for it. A
    // Local mirror keeps its members; an Omitted mirror stays absent (its
    // record is gone by design, so its member view is empty).
    let matrix_detached = context.associations.iter().any(|association| {
        association.kind == MirroredItemKind::Matrix
            && association.source_id == matrix_pair.source_id.as_str()
            && association.mirror_id == matrix_pair.mirror_id.as_str()
            && matches!(
                association.state,
                TransitionState::Local | TransitionState::Omitted
            )
            && association.explicit_state
    });
    let source_before = before
        .matrices
        .iter()
        .find(|matrix| matrix.id == matrix_pair.source_id.as_str())
        .ok_or("The source transition Matrix is unavailable")?;
    let mirror_before = before
        .matrices
        .iter()
        .find(|matrix| matrix.id == matrix_pair.mirror_id.as_str());
    let mirror_before = match mirror_before {
        Some(matrix) => Some(matrix),
        None if matrix_detached => None,
        None => return Err("The mirror transition Matrix is unavailable".into()),
    };
    let source_after_ids: BTreeSet<_> = post_source
        .matrices
        .iter()
        .find(|matrix| matrix.id == matrix_pair.source_id.as_str())
        .map(|matrix| matrix.part_ids.iter().cloned().collect())
        .unwrap_or_default();

    fn mapped_source_id(
        id: &str,
        matrix_pair: &super::super::MatrixPairContext,
        operation: &crate::model::EditOperation,
        post_source_ids: &BTreeSet<String>,
    ) -> Option<String> {
        let (mut row, mut column, child) =
            super::super::matrix_identity_slot(id, matrix_pair.source_id.as_str())?;
        match operation {
            Op::RemoveMatrix { id } if id == matrix_pair.source_id.as_str() => return None,
            Op::RemoveMatrixRow { row: removed, .. } => {
                if row == *removed {
                    return None;
                }
                if row > *removed {
                    row -= 1;
                }
            }
            Op::RemoveMatrixColumn {
                column: removed, ..
            } => {
                if column == *removed {
                    return None;
                }
                if column > *removed {
                    column -= 1;
                }
            }
            _ => {}
        }
        let suffix = if child.is_empty() {
            String::new()
        } else {
            format!("/{child}")
        };
        let mapped = format!(
            "matrix/{}/r{row}c{column}{suffix}",
            matrix_pair.source_id.as_str()
        );
        post_source_ids.contains(&mapped).then_some(mapped)
    }

    let source_identity_map: BTreeMap<_, _> = source_before
        .part_ids
        .iter()
        .map(|id| {
            (
                id.clone(),
                mapped_source_id(id, matrix_pair, operation, &source_after_ids),
            )
        })
        .collect();
    let association_by_mirror: BTreeMap<_, _> = context
        .associations
        .iter()
        .filter(|association| association.kind == MirroredItemKind::Part)
        .map(|association| (association.mirror_id.as_str(), association))
        .collect();
    let mut mirror_identity_map = BTreeMap::new();
    // An Omitted-absent mirror has no record, so its member view is empty.
    let mirror_before_part_ids: &[String] = mirror_before
        .map(|matrix| matrix.part_ids.as_slice())
        .unwrap_or(&[]);
    for old_mirror_id in mirror_before_part_ids {
        let association = association_by_mirror
            .get(old_mirror_id.as_str())
            .ok_or_else(|| {
                format!("Mirror member {old_mirror_id:?} has no lifecycle association")
            })?;
        let mapped_source = source_identity_map
            .get(&association.source_id)
            .ok_or_else(|| {
                format!(
                    "Source member {:?} has no identity mapping",
                    association.source_id
                )
            })?;
        let mapped_mirror = if association.state == TransitionState::Omitted {
            None
        } else if let Some(mapped_source) = mapped_source {
            let suffix = mapped_source
                .strip_prefix(&format!("matrix/{}/", matrix_pair.source_id.as_str()))
                .ok_or("The mapped source member is outside its matrix")?;
            Some(format!(
                "matrix/{}/{suffix}",
                matrix_pair.mirror_id.as_str()
            ))
        } else if association.state == TransitionState::Local {
            Some(old_mirror_id.clone())
        } else {
            None
        };
        mirror_identity_map.insert(old_mirror_id.clone(), mapped_mirror);
    }

    let mapped_sources: BTreeSet<_> = source_identity_map.values().flatten().cloned().collect();
    let planned_source_ids: BTreeSet<_> = if matrix_detached {
        BTreeSet::new()
    } else {
        source_after_ids
            .difference(&mapped_sources)
            .cloned()
            .collect()
    };
    let mut used_references: BTreeSet<_> = before
        .parts
        .iter()
        .chain(post_source.parts.iter())
        .map(|part| part.reference.clone())
        .collect();
    let mut planned_additions = Vec::new();
    for source_id in planned_source_ids {
        let (row, column, child) =
            super::super::matrix_identity_slot(&source_id, matrix_pair.source_id.as_str())
                .ok_or_else(|| format!("Planned source member {source_id:?} is not canonical"))?;
        let suffix = if child.is_empty() {
            String::new()
        } else {
            format!("/{child}")
        };
        let mirror_id = format!(
            "matrix/{}/r{row}c{column}{suffix}",
            matrix_pair.mirror_id.as_str()
        );
        let mut source_parts = post_source.parts.iter().filter(|part| part.id == source_id);
        let source_part = source_parts
            .next()
            .ok_or_else(|| format!("Planned source Part {source_id:?} is unavailable"))?;
        if source_parts.next().is_some() {
            return Err(format!("Planned source Part {source_id:?} is ambiguous"));
        }
        let target_reference = crate::mirrored_half::free_reference(
            &crate::mirrored_half::mirror_reference(
                &context.source_board_id,
                &context.mirror_board_id,
                &source_part.reference,
            ),
            &mut used_references,
        );
        let (parent_source_id, parent_mirror_id) = if child.is_empty() {
            (None, None)
        } else {
            (
                Some(format!(
                    "matrix/{}/r{row}c{column}",
                    matrix_pair.source_id.as_str()
                )),
                Some(format!(
                    "matrix/{}/r{row}c{column}",
                    matrix_pair.mirror_id.as_str()
                )),
            )
        };
        planned_additions.push(PlannedAddition {
            source_id,
            mirror_id,
            source_reference: source_part.reference.clone(),
            mirror_reference: target_reference,
            parent_source_id,
            parent_mirror_id,
        });
    }

    let mut explicit_removal_closure = BTreeSet::new();
    explicit_removal_closure.extend(
        source_identity_map
            .iter()
            .filter_map(|(id, mapped)| mapped.is_none().then_some(id.clone())),
    );
    explicit_removal_closure.extend(
        mirror_identity_map
            .iter()
            .filter_map(|(id, mapped)| mapped.is_none().then_some(id.clone())),
    );
    // A matrix-level Omitted mirror never materializes members, so its mirror
    // map carries no entries for them. Retiring a source-removed Omitted child
    // still needs its canonical mirror slot as closure evidence.
    if matrix_detached {
        for association in &context.associations {
            if association.kind == MirroredItemKind::Part
                && association.state == TransitionState::Omitted
                && source_identity_map.get(&association.source_id) == Some(&None)
            {
                let suffix = association
                    .source_id
                    .strip_prefix(&format!("matrix/{}/", matrix_pair.source_id.as_str()));
                if let Some(suffix) = suffix {
                    explicit_removal_closure.insert(format!(
                        "matrix/{}/{suffix}",
                        matrix_pair.mirror_id.as_str()
                    ));
                }
            }
        }
    }

    let retained_mirror_ids: BTreeSet<_> = mirror_identity_map
        .values()
        .flatten()
        .cloned()
        .chain(
            planned_additions
                .iter()
                .map(|addition| addition.mirror_id.clone()),
        )
        .collect();
    let removed_source_ids: BTreeSet<_> = source_identity_map
        .iter()
        .filter_map(|(id, mapped)| mapped.is_none().then_some(id.clone()))
        .collect();
    let removed_mirror_ids: BTreeSet<_> = mirror_identity_map
        .iter()
        .filter_map(|(id, mapped)| mapped.is_none().then_some(id.clone()))
        .collect();
    let mut required_owners: BTreeSet<_> = before
        .parts
        .iter()
        .chain(post_source.parts.iter())
        .map(|part| part.id.clone())
        .collect();
    required_owners.extend(context.owners.keys().cloned());
    for addition in &planned_additions {
        required_owners.insert(addition.source_id.clone());
        required_owners.insert(addition.mirror_id.clone());
    }
    let mut final_owners = BTreeMap::new();
    for id in required_owners {
        let reused_target = retained_mirror_ids.contains(&id);
        let reused_source = source_identity_map
            .values()
            .flatten()
            .any(|mapped| mapped == &id)
            || planned_additions
                .iter()
                .any(|addition| addition.source_id == id);
        let owner = if retained_mirror_ids.contains(&id) {
            FinalOwner::Board(context.mirror_board_id.clone())
        } else if (removed_source_ids.contains(&id) && !reused_source)
            || (removed_mirror_ids.contains(&id) && !reused_target)
        {
            FinalOwner::Absent
        } else if let Some(board) = post_source
            .boards
            .iter()
            .find(|board| board.part_ids.contains(&id))
        {
            FinalOwner::Board(board.id.clone())
        } else {
            FinalOwner::Absent
        };
        final_owners.insert(id, owner);
    }

    let mut output = prepare_membership_transition(MembershipTransitionInput {
        dispatch,
        valid_before: before,
        post_source,
        context,
        baseline_kind: BaselineKind::MaterializedCurrentSchema,
        source_identity_map: &source_identity_map,
        mirror_identity_map: &mirror_identity_map,
        planned_additions: &planned_additions,
        final_owners: &final_owners,
        explicit_removal_closure: &explicit_removal_closure,
    })?;

    let original_halves = source_operation
        .hardware
        .as_ref()
        .and_then(|hardware| {
            hardware
                .pairs
                .iter()
                .find(|pair| pair.id == selected.pair_id)
        })
        .and_then(|pair| pair.halves.as_ref())
        .ok_or("The original transition pair is unavailable")?;
    let output_halves = output
        .draft
        .hardware
        .as_mut()
        .and_then(|hardware| {
            hardware
                .pairs
                .iter_mut()
                .find(|pair| pair.id == selected.pair_id)
        })
        .and_then(|pair| pair.halves.as_mut())
        .ok_or("The prepared transition pair is unavailable")?;
    for association in original_halves
        .correspondences
        .iter()
        .filter(|association| {
            !selected_matrix_association(
                association,
                matrix_pair.source_id.as_str(),
                matrix_pair.mirror_id.as_str(),
            ) && !retired_layouts.iter().any(|retired| {
                association.kind == retired.kind
                    && association.source_id == retired.source_id
                    && association.mirror_id == retired.mirror_id
            })
        })
    {
        if !output_halves.correspondences.iter().any(|current| {
            current.kind == association.kind
                && current.source_id == association.source_id
                && current.mirror_id == association.mirror_id
        }) {
            output_halves.correspondences.push(association.clone());
        }
    }
    for exception in original_halves.exceptions.iter().filter(|exception| {
        !selected_matrix_exception(exception, matrix_pair.mirror_id.as_str())
            && !retired_layouts.iter().any(|retired| {
                exception.kind == retired.kind && exception.mirror_id == retired.mirror_id
            })
    }) {
        if !output_halves.exceptions.iter().any(|current| {
            current.kind == exception.kind && current.mirror_id == exception.mirror_id
        }) {
            output_halves.exceptions.push(exception.clone());
        }
    }
    Ok(output)
}

/// Classifies only source associations the lifecycle can retire and performs their existing
/// typed owner-closure checks. Projection callers may consume this plan without copying geometry.
pub(super) fn classify_source_retirements(
    before: &crate::model::ProjectDoc,
    post_source: &crate::model::ProjectDoc,
    context: &MembershipContext,
    include_unselected_matrices: bool,
) -> Result<Vec<super::super::TransitionAssociation>, String> {
    use crate::model::MirroredItemKind;

    let mut retired = Vec::new();
    for owner in context.associations.iter().filter(|association| {
        association.kind == MirroredItemKind::Matrix
            && (context.matrix_pair.as_ref().is_some_and(|matrix_pair| {
                association.source_id == matrix_pair.source_id
                    && association.mirror_id == matrix_pair.mirror_id
            }) || (include_unselected_matrices && context.matrix_pair.is_none()))
    }) {
        if post_source
            .matrices
            .iter()
            .any(|matrix| matrix.id == owner.source_id)
        {
            continue;
        }
        if context
            .pending_pair_transitions
            .contains_key(&owner.source_id)
        {
            return Err(
                "A selected lifecycle Matrix source was removed before its transition".into(),
            );
        }
        if before
            .matrices
            .iter()
            .filter(|matrix| matrix.id == owner.source_id)
            .flat_map(|matrix| matrix.part_ids.iter())
            .any(|id| {
                post_source.parts.iter().any(|part| part.id == *id)
                    || post_source
                        .boards
                        .iter()
                        .any(|board| board.part_ids.contains(id))
            })
        {
            return Err("Source Matrix removal has an incomplete ownership closure".into());
        }
        retired.push(owner.clone());
    }
    for association in context
        .associations
        .iter()
        .filter(|association| association.kind == MirroredItemKind::Part)
    {
        if post_source
            .parts
            .iter()
            .any(|part| part.id == association.source_id)
        {
            continue;
        }
        if context
            .pending_pair_transitions
            .contains_key(&association.source_id)
        {
            return Err("A selected lifecycle source was removed before its transition".into());
        }
        if post_source.boards.iter().any(|board| {
            board
                .part_ids
                .iter()
                .any(|part_id| part_id == &association.source_id)
        }) || post_source.matrices.iter().any(|matrix| {
            matrix
                .part_ids
                .iter()
                .any(|part_id| part_id == &association.source_id)
        }) || post_source.embedded_circuits.iter().any(|circuit| {
            circuit
                .part_ids
                .iter()
                .any(|part_id| part_id == &association.source_id)
        }) {
            return Err(format!(
                "Source Part removal for {:?} has an incomplete ownership closure",
                association.source_id
            ));
        }
        retired.push(association.clone());
    }
    for association in context.associations.iter().filter(|association| {
        matches!(
            association.kind,
            MirroredItemKind::EmbeddedCircuit | MirroredItemKind::Net | MirroredItemKind::Layout
        )
    }) {
        let present = match association.kind {
            MirroredItemKind::EmbeddedCircuit => post_source
                .embedded_circuits
                .iter()
                .any(|circuit| circuit.id == association.source_id),
            MirroredItemKind::Net => post_source
                .nets
                .iter()
                .any(|net| net.id == association.source_id),
            MirroredItemKind::Layout => post_source
                .layouts
                .iter()
                .any(|layout| layout.id == association.source_id),
            _ => unreachable!(),
        };
        if present {
            continue;
        }
        if context
            .pending_pair_transitions
            .contains_key(&association.source_id)
        {
            return Err("A selected lifecycle source was removed before its transition".into());
        }
        if association.kind == MirroredItemKind::EmbeddedCircuit
            && before
                .embedded_circuits
                .iter()
                .filter(|circuit| circuit.id == association.source_id)
                .flat_map(|circuit| circuit.part_ids.iter())
                .any(|id| {
                    post_source.parts.iter().any(|part| part.id == *id)
                        || post_source
                            .boards
                            .iter()
                            .any(|board| board.part_ids.contains(id))
                })
        {
            return Err("Source circuit removal has an incomplete ownership closure".into());
        }
        if association.kind == MirroredItemKind::Net
            && post_source
                .boards
                .iter()
                .any(|board| board.net_ids.contains(&association.source_id))
        {
            return Err("A removed source net retains a board registration".into());
        }
        retired.push(association.clone());
    }
    Ok(retired)
}
