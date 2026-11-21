//! Native detach proposals, edit transitions and replacement-record retirement.

use super::super::TransitionState;
use super::{
    ConstraintLifecycleDelta, ConstraintLifecycleEffect, classify_source_retirements,
    lifecycle_context, matrix_pair_for_association, prepare_constraint_lifecycle_delta,
    prepare_lifecycle_transition, prepare_matrix_membership_edit, transition_exception_context,
    validate_lifecycle_context,
};
use crate::constraints::FramedConstraint;
use crate::model::{Constraint, HalfExceptionState};
use std::collections::BTreeSet;

/// Ticket 29: the native shared-detach proposal. After reconciliation, a
/// plain detach that moved its linked native partner refuses with the
/// ticket-23 confirmation shape, so the UI can offer the confirmed variant
/// verbatim. A partner that owns its pose (Local/Omitted) never moves
/// through the projection and is skipped; anything else keeps its poses.
pub(in crate::mirrored_half::reconcile) fn native_detach_proposal(
    accepted: &crate::model::ProjectDoc,
    draft: &crate::model::ProjectDoc,
    matrix_id: &str,
) -> Option<String> {
    use crate::model::MirroredItemKind;
    let halves_list = accepted
        .hardware
        .as_ref()
        .into_iter()
        .flat_map(|hardware| &hardware.pairs)
        .filter_map(|pair| pair.halves.as_ref());
    for halves in halves_list {
        let Some(partner) = halves
            .correspondences
            .iter()
            .find(|record| record.kind == MirroredItemKind::Matrix && record.source_id == matrix_id)
            .map(|record| record.mirror_id.clone())
        else {
            continue;
        };
        if halves.exceptions.iter().any(|exception| {
            exception.kind == MirroredItemKind::Matrix && exception.mirror_id == partner
        }) {
            continue;
        }
        let pose = |document: &crate::model::ProjectDoc, id: &str| {
            document
                .parts
                .iter()
                .find(|part| part.id == id)
                .map(|part| part.pose)
        };
        let prefix = format!("matrix/{partner}/");
        let moved = halves
            .correspondences
            .iter()
            .filter(|record| {
                record.kind == MirroredItemKind::Part && record.mirror_id.starts_with(&prefix)
            })
            .any(|record| {
                match (
                    pose(accepted, &record.mirror_id),
                    pose(draft, &record.mirror_id),
                ) {
                    (Some(before), Some(after)) => before != after,
                    _ => false,
                }
            });
        if moved {
            return Some(format!(
                "Detaching {matrix_id} would move its linked partner {partner}; \
                 make {partner} Local to keep its pose and confirm the detach"
            ));
        }
    }
    None
}

pub(in crate::mirrored_half::reconcile) fn prepare_edit_transition(
    before: &crate::model::ProjectDoc,
    post_source: &crate::model::ProjectDoc,
    pair_id: &str,
    operation: &crate::model::EditOperation,
    dispatch: crate::mirrored_half::EditDispatch,
) -> Result<Option<super::super::PreparedLifecycleDraft>, String> {
    use crate::model::EditOperation;

    if let EditOperation::SetHalfException {
        pair_id: requested_pair_id,
        kind,
        mirror_id,
        state,
    } = operation
    {
        if requested_pair_id != pair_id {
            return Ok(None);
        }
        let hardware = before
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
        let mut selected = halves
            .correspondences
            .iter()
            .filter(|association| association.kind == *kind && association.mirror_id == *mirror_id);
        let association = selected
            .next()
            .ok_or("The exact lifecycle association is unavailable")?;
        if selected.next().is_some() {
            return Err("The exact lifecycle association is ambiguous".into());
        }
        let matrix_pair =
            matrix_pair_for_association(before, halves, *kind, &association.source_id, mirror_id)?;
        let context = lifecycle_context(before, pair_id, matrix_pair)?;
        let prior_state = context
            .associations
            .iter()
            .find(|item| {
                item.kind == *kind
                    && item.source_id == association.source_id
                    && item.mirror_id == *mirror_id
            })
            .ok_or("The exact lifecycle association is unavailable")?
            .state;
        let requested_state = match state {
            HalfExceptionState::Local => TransitionState::Local,
            HalfExceptionState::Omitted => TransitionState::Omitted,
            HalfExceptionState::Linked => TransitionState::Linked,
        };
        let pending = transition_exception_context(
            &context,
            *kind,
            &association.source_id,
            mirror_id,
            requested_state,
        )?;
        let output = prepare_lifecycle_transition(before, post_source, &pending, dispatch)?;
        let mut effects = Vec::new();
        if prior_state != requested_state {
            match requested_state {
                TransitionState::Local => effects.push(ConstraintLifecycleEffect::MakeLocal {
                    pair_id: pair_id.to_owned(),
                    source_part_id: association.source_id.clone(),
                    mirror_part_id: mirror_id.clone(),
                }),
                TransitionState::Omitted => {
                    effects.push(ConstraintLifecycleEffect::RemoveVisibleParts {
                        part_ids: removed_part_ids(before, &output.draft),
                    })
                }
                TransitionState::Linked => effects.push(ConstraintLifecycleEffect::Rejoin {
                    pair_id: pair_id.to_owned(),
                    source_part_id: association.source_id.clone(),
                    mirror_part_id: mirror_id.clone(),
                }),
            }
        }
        return prepare_lifecycle_output(before, output, &effects).map(Some);
    }

    // Ticket 29: the native shared detach. Detaching an anchored matrix
    // whose linked native partner follows the anchor would move the
    // partner, so the confirmed intent clears both anchors and records the
    // named Local exception in this one transition. Only the pair linking
    // the two matrices handles it; stale confirmations refuse unchanged.
    if let EditOperation::DetachMatrixAnchorConfirmed {
        matrix_id,
        local_matrix_id,
    } = operation
    {
        use crate::model::MirroredItemKind;
        let halves = before
            .hardware
            .as_ref()
            .into_iter()
            .flat_map(|hardware| &hardware.pairs)
            .filter(|pair| pair.id == pair_id)
            .filter_map(|pair| pair.halves.as_ref())
            .next();
        let Some(halves) = halves else {
            return Ok(None);
        };
        // The detaching matrix must be a linked source of this pair;
        // otherwise another pair (or the same-board path) owns the intent.
        if !halves
            .correspondences
            .iter()
            .any(|record| record.kind == MirroredItemKind::Matrix && record.source_id == *matrix_id)
        {
            return Ok(None);
        }
        if !halves.correspondences.iter().any(|record| {
            record.kind == MirroredItemKind::Matrix
                && record.source_id == *matrix_id
                && record.mirror_id == *local_matrix_id
        }) {
            return Err(format!(
                "Confirmed detach is stale: {local_matrix_id} is not the linked partner of {matrix_id}"
            ));
        }
        if halves.exceptions.iter().any(|exception| {
            exception.kind == MirroredItemKind::Matrix && exception.mirror_id == *local_matrix_id
        }) {
            return Err(format!(
                "Confirmed detach is stale: {local_matrix_id} is already excepted"
            ));
        }
        let context = lifecycle_context(
            before,
            pair_id,
            Some(super::super::MatrixPairContext {
                source_id: matrix_id.clone(),
                mirror_id: local_matrix_id.clone(),
            }),
        )?;
        let pending = transition_exception_context(
            &context,
            MirroredItemKind::Matrix,
            matrix_id,
            local_matrix_id,
            TransitionState::Local,
        )?;
        let output = prepare_lifecycle_transition(before, post_source, &pending, dispatch)?;
        let mut prepared = prepare_lifecycle_output(
            before,
            output,
            &[ConstraintLifecycleEffect::MakeLocal {
                pair_id: pair_id.to_owned(),
                source_part_id: matrix_id.clone(),
                mirror_part_id: local_matrix_id.clone(),
            }],
        )?;
        // The Local transition keeps the mirror's inherited anchor record;
        // freezing its world frame first detaches the mirror exactly like
        // the source below, so both poses survive in the one entry.
        let frozen = prepared
            .draft
            .matrices
            .iter()
            .find(|matrix| matrix.id == *local_matrix_id)
            .cloned()
            .ok_or_else(|| format!("The confirmed detach mirror {local_matrix_id} is missing"))?;
        if frozen.anchor.is_some() {
            let frame = crate::matrix::evaluation::evaluate(&frozen, &prepared.draft)
                .map_err(|message| message.to_string())?
                .frame;
            let mut next = frozen.clone();
            next.anchor = None;
            next.origin = frame.origin;
            next.rotation = if frame.rotation == 0.0 {
                None
            } else {
                Some(frame.rotation)
            };
            crate::matrix::set_matrix(&mut prepared.draft, &next)?;
            for part_id in &next.part_ids {
                if !prepared.changed_document_ids.contains(part_id) {
                    prepared.changed_document_ids.push(part_id.clone());
                }
            }
        }
        // Clearing the source anchor keeps its world frame.
        let mut detached = crate::matrix::anchors::detach(&mut prepared.draft, matrix_id)?;
        prepared.changed_document_ids.append(&mut detached);
        prepared.changed_document_ids.push(local_matrix_id.clone());
        return Ok(Some(prepared));
    }

    if let EditOperation::UnlinkHalves {
        pair_id: requested_pair_id,
    } = operation
    {
        if requested_pair_id != pair_id {
            return Ok(None);
        }
        let records = lifecycle_constraint_records(before)?;
        let delta = prepare_constraint_lifecycle_delta(
            &records,
            &[ConstraintLifecycleEffect::Unlink {
                pair_id: pair_id.to_owned(),
            }],
        )?;
        let mut draft = post_source.clone();
        let changed_document_ids = crate::mechanical::halves::unlink_pair(&mut draft, pair_id)?;
        apply_constraint_lifecycle_delta(&mut draft, delta);
        return Ok(Some(super::super::PreparedLifecycleDraft {
            draft,
            changed_document_ids,
        }));
    }

    let hardware = before
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
    let Some(halves) = &pair.halves else {
        return Ok(None);
    };
    let matrix_pairs: Vec<_> = halves
        .correspondences
        .iter()
        .filter(|record| record.kind == crate::model::MirroredItemKind::Matrix)
        .map(|record| (record.source_id.clone(), record.mirror_id.clone()))
        .collect();
    let changed_matrices: Vec<_> = matrix_pairs
        .iter()
        .filter(|(source, _)| super::super::matrix_membership_changed(before, post_source, source))
        .cloned()
        .collect();
    if !changed_matrices.is_empty() {
        // A single RemoveSelectedMatrixCells request can change membership in several matrix
        // pairs. Prepare each against the accepted baseline while carrying the prior private
        // draft forward, so each transition preserves the already-composed mirror membership.
        let mut draft = post_source.clone();
        let mut outputs = Vec::with_capacity(changed_matrices.len());
        for (source_matrix_id, mirror_matrix_id) in changed_matrices {
            let context = lifecycle_context(
                before,
                pair_id,
                Some(super::super::MatrixPairContext {
                    source_id: source_matrix_id.clone(),
                    mirror_id: mirror_matrix_id.clone(),
                }),
            )?;
            let output =
                prepare_matrix_membership_edit(before, &draft, operation, &context, dispatch)?;
            draft = output.draft.clone();
            outputs.push(output);
        }
        let mut draft = post_source.clone();
        let mut changed_document_ids = Vec::new();
        for output in outputs {
            let removed = removed_part_ids(before, &output.draft);
            let prepared = prepare_lifecycle_output(
                before,
                output,
                &[ConstraintLifecycleEffect::RemoveVisibleParts { part_ids: removed }],
            )?;
            changed_document_ids.extend(prepared.changed_document_ids);
            draft = prepared.draft;
        }
        return Ok(Some(super::super::PreparedLifecycleDraft {
            draft,
            changed_document_ids,
        }));
    }
    let matrix_pair = matrix_pairs
        .into_iter()
        .next()
        .map(|(source_id, mirror_id)| super::super::MatrixPairContext {
            source_id,
            mirror_id,
        });
    let context = lifecycle_context(before, pair_id, matrix_pair)?;
    let output =
        super::transition::prepare_lifecycle_projection(before, post_source, &context, dispatch)?;
    // Caller-owned draft: no per-pair full report. The outer transaction guarantees rollback
    // and owns the single final scoped report; the final diff already covers these IDs.
    let removed = removed_part_ids(before, &output.draft);
    prepare_lifecycle_output(
        before,
        output,
        &[ConstraintLifecycleEffect::RemoveVisibleParts { part_ids: removed }],
    )
    .map(Some)
}

/// Retires only exact, carried lifecycle rows whose typed source object was removed by a
/// replacement. The replacement payload remains the sole owner of geometry and target-local data.
#[doc(hidden)]
pub fn retire_replacement_records(
    accepted: &crate::model::ProjectDoc,
    replacement: &mut crate::model::ProjectDoc,
) -> Result<(), String> {
    use crate::model::{MirroredCorrespondence, MirroredException};

    #[derive(Clone)]
    struct Retirement {
        pair_id: String,
        row: MirroredCorrespondence,
        exception: Option<MirroredException>,
    }

    crate::mechanical::halves::validate_document(accepted)?;
    let accepted_hardware = accepted
        .hardware
        .as_ref()
        .ok_or("Lifecycle hardware is unavailable")?;
    let mut retirements = Vec::new();
    for accepted_pair in &accepted_hardware.pairs {
        let Some(accepted_halves) = accepted_pair.halves.as_ref() else {
            continue;
        };
        let Some(replacement_pair) = replacement.hardware.as_ref().and_then(|hardware| {
            hardware
                .pairs
                .iter()
                .find(|pair| pair.id == accepted_pair.id)
        }) else {
            continue;
        };
        let Some(replacement_halves) = replacement_pair.halves.as_ref() else {
            continue;
        };
        if replacement_pair.members != accepted_pair.members
            || replacement_pair.mapping != accepted_pair.mapping
            || replacement_halves.source_board_id != accepted_halves.source_board_id
            || replacement_halves.mirror_board_id != accepted_halves.mirror_board_id
        {
            continue;
        }
        let context = lifecycle_context(accepted, &accepted_pair.id, None)?;
        validate_lifecycle_context(accepted, &context)?;
        let typed_retirements = classify_source_retirements(accepted, replacement, &context, true)?;
        for retired in typed_retirements {
            let Some(row) = accepted_halves.correspondences.iter().find(|row| {
                row.kind == retired.kind
                    && row.source_id == retired.source_id
                    && row.mirror_id == retired.mirror_id
            }) else {
                continue;
            };
            let carried_rows: Vec<_> = replacement_halves
                .correspondences
                .iter()
                .filter(|candidate| {
                    candidate.kind == row.kind
                        && candidate.source_id == row.source_id
                        && candidate.mirror_id == row.mirror_id
                })
                .collect();
            match carried_rows.as_slice() {
                [] => continue,
                [carried] if *carried == row => {}
                [_] => continue,
                _ => return Err("ReplaceDocument duplicates a mirrored retirement row".into()),
            }
            let accepted_exception = accepted_halves.exceptions.iter().find(|exception| {
                exception.kind == row.kind && exception.mirror_id == row.mirror_id
            });
            let replacement_exceptions: Vec<_> = replacement_halves
                .exceptions
                .iter()
                .filter(|exception| {
                    exception.kind == row.kind && exception.mirror_id == row.mirror_id
                })
                .collect();
            let exception = match replacement_exceptions.as_slice() {
                [] => None,
                [candidate] if Some(*candidate) == accepted_exception => Some((*candidate).clone()),
                [_] => continue,
                _ => {
                    return Err("ReplaceDocument duplicates a mirrored retirement exception".into());
                }
            };
            retirements.push(Retirement {
                pair_id: accepted_pair.id.clone(),
                row: row.clone(),
                exception,
            });
        }
    }

    for retirement in retirements {
        let pair = replacement
            .hardware
            .as_mut()
            .and_then(|hardware| {
                hardware
                    .pairs
                    .iter_mut()
                    .find(|pair| pair.id == retirement.pair_id)
            })
            .ok_or("A planned lifecycle retirement pair disappeared")?;
        let halves = pair
            .halves
            .as_mut()
            .ok_or("A planned lifecycle retirement pair lost its halves metadata")?;
        halves.correspondences.retain(|row| {
            !(row.kind == retirement.row.kind
                && row.source_id == retirement.row.source_id
                && row.mirror_id == retirement.row.mirror_id
                && row == &retirement.row)
        });
        if let Some(exception) = retirement.exception {
            halves
                .exceptions
                .retain(|candidate| candidate != &exception);
        }
    }
    crate::mechanical::halves::validate_document(replacement)?;
    Ok(())
}

pub(super) fn removed_part_ids(
    before: &crate::model::ProjectDoc,
    after: &crate::model::ProjectDoc,
) -> BTreeSet<String> {
    let retained: BTreeSet<_> = after.parts.iter().map(|part| part.id.as_str()).collect();
    before
        .parts
        .iter()
        .filter(|part| !retained.contains(part.id.as_str()))
        .map(|part| part.id.clone())
        .collect()
}

pub(super) fn lifecycle_constraint_records(
    document: &crate::model::ProjectDoc,
) -> Result<Vec<FramedConstraint>, String> {
    document
        .constraints
        .iter()
        .map(|constraint| crate::constraints::compile_legacy(document, constraint))
        .collect()
}

pub(super) fn apply_constraint_lifecycle_delta(
    document: &mut crate::model::ProjectDoc,
    delta: ConstraintLifecycleDelta,
) {
    document
        .constraints
        .retain(|constraint| !delta.removals.iter().any(|id| constraint.id() == id));
    for record in delta.upserts {
        let FramedConstraint::Framed {
            id,
            source,
            target,
            relation,
        } = record;
        let constraint = Constraint::Framed {
            id,
            source,
            target,
            relation,
        };
        if let Some(existing) = document
            .constraints
            .iter_mut()
            .find(|existing| existing.id() == constraint.id())
        {
            *existing = constraint;
        } else {
            document.constraints.push(constraint);
        }
    }
}

pub(super) fn prepare_lifecycle_output(
    before: &crate::model::ProjectDoc,
    mut output: super::super::MembershipTransitionOutput,
    effects: &[ConstraintLifecycleEffect],
) -> Result<super::super::PreparedLifecycleDraft, String> {
    let records = lifecycle_constraint_records(before)?;
    let delta = prepare_constraint_lifecycle_delta(&records, effects)?;
    apply_constraint_lifecycle_delta(&mut output.draft, delta);
    Ok(super::super::PreparedLifecycleDraft {
        draft: output.draft,
        changed_document_ids: output.changed_document_ids,
    })
}
