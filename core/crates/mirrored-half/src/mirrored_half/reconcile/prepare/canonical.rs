//! Canonical edit application and lifecycle-pair preparation.

use super::projection::{matrix_by_id, project_matrixless_placement};
use super::*;

fn version_deletion(operation: &EditOperation) -> Option<&crate::outline_edit::OutlineEdit> {
    match operation {
        EditOperation::ApplyOutlineEdit { edit }
            if edit.change == crate::outline_edit::OutlineChange::Delete =>
        {
            Some(edit)
        }
        _ => None,
    }
}

pub(super) fn prepare_lifecycle_pairs(
    accepted: &ProjectDoc,
    document: &mut ProjectDoc,
    canonical: &Canonical,
    dispatch: crate::mirrored_half::EditDispatch,
    directed: bool,
) -> Result<Vec<String>, String> {
    if matches!(
        canonical.operation,
        EditOperation::AddPart { .. }
            | EditOperation::AddMirrorOnlyPart { .. }
            | EditOperation::SetCompanionPlacement { .. }
            | EditOperation::ApplyMechanicalEdit { .. }
            | EditOperation::CreateMirroredPair { .. }
            | EditOperation::CreateMirroredHalf { .. }
            | EditOperation::ReplaceDocument { .. }
    ) {
        return Ok(Vec::new());
    }
    if let EditOperation::SetHalfException { pair_id, .. }
    | EditOperation::UnlinkHalves { pair_id } = &canonical.operation
    {
        let matches: Vec<_> = accepted
            .hardware
            .as_ref()
            .into_iter()
            .flat_map(|hardware| &hardware.pairs)
            .filter(|pair| pair.id == *pair_id && pair.halves.is_some())
            .collect();
        if matches.len() != 1 {
            return Err("The exact lifecycle pair is unavailable or ambiguous".into());
        }
    }
    let pairs = accepted
        .hardware
        .as_ref()
        .into_iter()
        .flat_map(|hardware| &hardware.pairs)
        .filter(|pair| pair.halves.is_some());
    let mut changed = Vec::new();
    for pair in pairs {
        let Some(current) = document
            .hardware
            .as_ref()
            .and_then(|h| h.pairs.iter().find(|row| row.id == pair.id))
        else {
            continue;
        };
        let retired_version =
            version_deletion(&canonical.operation).is_some() && current.halves != pair.halves;
        // Even a pair the edit did not touch may need its linked drift repaired.
        if directed && project_matrixless_placement(accepted, document, pair, &canonical.operation)?
        {
            continue;
        }
        if let Some(prepared) = super::super::lifecycle::prepare_edit_transition(
            // Version deletion has already retired its pair row. The resulting pair is
            // the baseline for ordinary geometry projection on this edit.
            if retired_version { document } else { accepted },
            document,
            &pair.id,
            &canonical.operation,
            dispatch,
        )? {
            changed.extend(prepared.changed_document_ids);
            *document = prepared.draft;
        }
    }
    Ok(changed)
}

pub(super) fn apply_canonical(
    draft: &mut ProjectDoc,
    accepted: &ProjectDoc,
    board_contours: &[BoardContours],
    canonical: &Canonical,
    identities: &crate::mechanical::edit::MechanicalIdentityLedger,
    dispatch: crate::mirrored_half::EditDispatch,
) -> Result<Vec<String>, String> {
    if let EditOperation::AddMirrorOnlyPart { board_id, part, .. } = &canonical.operation {
        if id_exists_outside_parts(accepted, &part.id) {
            return Err(format!(
                "Mirror-only placement Part ID {:?} conflicts with another document identity",
                part.id
            ));
        }
        let pairs = accepted
            .hardware
            .iter()
            .flat_map(|hardware| &hardware.pairs)
            .filter(|pair| {
                pair.halves.as_ref().is_some_and(|halves| {
                    halves.mirror_board_id == *board_id && halves.source_board_id != *board_id
                })
            })
            .collect::<Vec<_>>();
        if pairs.len() != 1 {
            return Err(format!(
                "Mirror-only placement requires one exact native mirror board; {board_id:?} matched {} pairs",
                pairs.len()
            ));
        }
    }
    if matches!(
        canonical.operation,
        EditOperation::SetHalfException { .. } | EditOperation::UnlinkHalves { .. }
    ) {
        return Ok(Vec::new());
    }
    // The confirmed native detach runs entirely in the lifecycle arm below;
    // anything else (same-board links, unpaired matrices) keeps the shared
    // apply path so its behavior is unchanged.
    if let EditOperation::DetachMatrixAnchorConfirmed { matrix_id, .. } = &canonical.operation {
        use crate::model::MirroredItemKind;
        let paired = draft
            .hardware
            .as_ref()
            .into_iter()
            .flat_map(|hardware| &hardware.pairs)
            .filter_map(|pair| pair.halves.as_ref())
            .flat_map(|halves| &halves.correspondences)
            .any(|record| {
                record.kind == MirroredItemKind::Matrix && record.source_id == *matrix_id
            });
        if paired {
            return Ok(Vec::new());
        }
    }
    if let EditOperation::SetCompanionPlacement {
        pair_id,
        targets,
        mode,
        tweak,
    } = &canonical.operation
    {
        return set_companion_placement(draft, pair_id, targets, *mode, *tweak);
    }
    if let EditOperation::ApplyMechanicalEdit { edit } = &canonical.operation
        && let crate::mechanical::MechanicalChange::UnlinkMirrorPair { instance_id } = &edit.change
    {
        let pair_id = pair_for_instance(accepted, instance_id)?;
        return crate::mechanical::halves::unlink_pair(draft, &pair_id);
    }
    let mut changed =
        (dispatch.apply_with_ledger)(draft, board_contours, &canonical.operation, identities)?;
    if let Some(edit) = version_deletion(&canonical.operation)
        && let crate::outline_edit::OutlineEditTarget::Version {
            board_id,
            version_id,
        } = &edit.target
    {
        changed.extend(super::super::outlines::retire_version(
            draft, edit, board_id, version_id,
        )?);
    }
    // Ticket 29 B4: removing a halves-pair source over an Omitted mirror
    // orphans the mirror layout shell — its matrix is absent by design and
    // its source is gone, so layout validation could never resolve it. Drop
    // shells left naming only such mirrors here; the kernel retires the pair
    // records. Anything else is untouched.
    if let EditOperation::RemoveMatrix { id } = &canonical.operation {
        let mut orphaned_mirrors = BTreeSet::new();
        for pair in accepted
            .hardware
            .iter()
            .flat_map(|hardware| &hardware.pairs)
        {
            let Some(halves) = pair.halves.as_ref() else {
                continue;
            };
            for record in &halves.correspondences {
                if record.kind == MirroredItemKind::Matrix && record.source_id == *id {
                    let omitted = halves.exceptions.iter().any(|exception| {
                        exception.kind == MirroredItemKind::Matrix
                            && exception.mirror_id == record.mirror_id
                            && exception.state == crate::model::MirroredExceptionState::Omitted
                    });
                    if omitted {
                        orphaned_mirrors.insert(record.mirror_id.clone());
                    }
                }
            }
        }
        if !orphaned_mirrors.is_empty() {
            for layout in &mut draft.layouts {
                layout
                    .matrix_ids
                    .retain(|matrix_id| !orphaned_mirrors.contains(matrix_id));
            }
            let before_layouts = draft.layouts.len();
            draft.layouts.retain(|layout| !layout.matrix_ids.is_empty());
            if draft.layouts.len() != before_layouts {
                changed.push(id.clone());
            }
        }
    }
    for addition in &canonical.hosted.circuits {
        for (member_id, pose) in &addition.member_poses {
            let member = draft
                .parts
                .iter_mut()
                .find(|part| part.id == *member_id)
                .ok_or_else(|| {
                    format!("The embedded circuit member {member_id:?} is unavailable")
                })?;
            member.pose = *pose;
        }
    }
    Ok(changed)
}

pub(super) fn pair_for_instance(
    document: &ProjectDoc,
    instance_id: &str,
) -> Result<String, String> {
    let mut pairs = document
        .hardware
        .as_ref()
        .ok_or("Physical instances are unavailable")?
        .pairs
        .iter()
        .filter(|pair| pair.members.iter().any(|member| member == instance_id));
    let pair = pairs.next().ok_or_else(|| {
        format!("Physical instance {instance_id:?} is not part of a mechanical pair.")
    })?;
    if pairs.next().is_some() {
        return Err(format!(
            "Physical instance {instance_id:?} belongs to more than one mechanical pair."
        ));
    }
    Ok(pair.id.clone())
}

pub(super) fn operation_positions(operation: &EditOperation) -> &[Position] {
    match operation {
        EditOperation::MoveParts { positions } => positions,
        _ => &[],
    }
}

pub(super) fn initial_framed_constraints(canonical: &Canonical) -> Vec<FramedConstraint> {
    canonical
        .prepared_constraints
        .iter()
        .filter_map(|edit| match edit {
            PreparedConstraintEdit::Upsert(record) => Some(record.clone()),
            PreparedConstraintEdit::DeferredTargetRetune(intent) => Some(intent.record.clone()),
            PreparedConstraintEdit::Remove { .. } => None,
        })
        .collect()
}

pub(super) fn bind_deferred_constraints(
    document: &ProjectDoc,
    canonical: &Canonical,
    already_framed: &[FramedConstraint],
) -> Result<Vec<FramedConstraint>, String> {
    let mut framed: Vec<_> = already_framed
        .iter()
        .filter(|record| {
            !canonical.prepared_constraints.iter().any(|edit| {
                matches!(edit, PreparedConstraintEdit::DeferredTargetRetune(intent) if record_id(&intent.record) == record_id(record))
            })
        })
        .cloned()
        .collect();
    for edit in &canonical.prepared_constraints {
        let PreparedConstraintEdit::DeferredTargetRetune(intent) = edit else {
            continue;
        };
        let source_id = authoritative_id(&intent.record, true);
        let target_id = authoritative_id(&intent.record, false);
        let source = part_pose(document, source_id)?;
        let target = part_pose(document, target_id)?;
        let owners = [
            intent
                .source_owner
                .as_deref()
                .map(|id| part_pose(document, id))
                .transpose()?,
            intent
                .target_owner
                .as_deref()
                .map(|id| part_pose(document, id))
                .transpose()?,
        ];
        match crate::constraints::prepare_target_retune(
            document,
            &intent.record,
            source,
            target,
            &intent.requested,
            owners,
        )? {
            crate::constraints::PreparedTargetRetune::Retuned(record) => framed.push(record),
            crate::constraints::PreparedTargetRetune::Deferred(_) => {
                return Err("A prepared constraint still lacks its resolved owner pose".into());
            }
        }
    }
    Ok(framed)
}

pub(super) fn record_id(record: &FramedConstraint) -> &str {
    match record {
        FramedConstraint::Framed { id, .. } => id,
    }
}

pub(super) fn authoritative_id(record: &FramedConstraint, source: bool) -> &str {
    let FramedConstraint::Framed {
        source: source_endpoint,
        target: target_endpoint,
        ..
    } = record;
    match if source {
        source_endpoint
    } else {
        target_endpoint
    } {
        crate::constraints::ConstraintEndpoint::Part { part_id } => part_id,
        crate::constraints::ConstraintEndpoint::LinkedMirror { source_part_id, .. } => {
            source_part_id
        }
    }
}

pub(super) fn part_pose(document: &ProjectDoc, id: &str) -> Result<Pose2, String> {
    let mut parts = document.parts.iter().filter(|part| part.id == id);
    let part = parts
        .next()
        .ok_or_else(|| format!("Prepared constraint part {id:?} is unavailable"))?;
    if parts.next().is_some() {
        return Err(format!("Prepared constraint part {id:?} is ambiguous"));
    }
    Ok(part.pose)
}

/// Restore Local companion source data after generic matrix sync rematerializes it.
/// Reconciliation consumes this pose and rebases the target cell assembly in the same pass.
pub(super) fn restore_local_matrix_companion_parts(
    accepted: &ProjectDoc,
    document: &mut ProjectDoc,
    operation: Option<&EditOperation>,
) -> Result<Vec<String>, String> {
    let positions = operation.map(operation_positions).unwrap_or_default();
    let mut changed = BTreeSet::new();
    let Some(hardware) = &accepted.hardware else {
        return Ok(Vec::new());
    };
    for pair in &hardware.pairs {
        let Some(halves) = &pair.halves else {
            continue;
        };
        for link in halves.correspondences.iter().filter(|item| {
            item.kind == MirroredItemKind::Matrix && halves.overlay_of(item).is_none()
        }) {
            let source = matrix_by_id(accepted, &link.source_id)?;
            let Some(mirror) = accepted
                .matrices
                .iter()
                .find(|item| item.id == link.mirror_id)
            else {
                continue;
            };
            let removed_axis = operation.and_then(|op| {
                super::super::transition::RemovedAxis::for_operation(op, &source.id)
            });
            for cell in source
                .cells
                .iter()
                .filter(|cell| cell.enabled && !cell.deleted)
            {
                // Compaction has already carried Local payloads to their new slots.
                // The old ID can now belong to a different, linked companion.
                if removed_axis.is_some_and(|axis| {
                    axis.remap(cell.row, cell.column) != Some((cell.row, cell.column))
                }) {
                    continue;
                }
                let source_key = crate::matrix::member_id(&source.id, cell.row, cell.column);
                let mirror_key = crate::matrix::member_id(&mirror.id, cell.row, cell.column);
                if halves.correspondences.iter().any(|association| {
                    association.kind == MirroredItemKind::Part
                        && association.source_id == source_key
                        && association.mirror_id == mirror_key
                        && halves.overlay_of(association).is_some()
                }) {
                    continue;
                }
                for assembly in &cell.assemblies {
                    let source_id = crate::matrix::companion_id(&source_key, &assembly.id);
                    let mirror_id = crate::matrix::companion_id(&mirror_key, &assembly.id);
                    let Some(association) = halves.correspondences.iter().find(|association| {
                        association.kind == MirroredItemKind::Part
                            && association.source_id == source_id
                            && association.mirror_id == mirror_id
                    }) else {
                        continue;
                    };
                    if !halves
                        .overlay_of(association)
                        .is_some_and(|exception| exception.state == MirroredExceptionState::Local)
                    {
                        continue;
                    }
                    let Some(saved) = accepted.parts.iter().find(|part| part.id == mirror_id)
                    else {
                        continue;
                    };
                    let Some(current) = document.parts.iter_mut().find(|part| part.id == mirror_id)
                    else {
                        continue;
                    };
                    let mut desired = saved.clone();
                    if let Some(position) =
                        positions.iter().find(|position| position.id == mirror_id)
                    {
                        desired.pose.at = position.at;
                    }
                    if *current != desired {
                        *current = desired;
                        changed.insert(mirror_id);
                    }
                }
            }
        }
    }
    Ok(changed.into_iter().collect())
}
