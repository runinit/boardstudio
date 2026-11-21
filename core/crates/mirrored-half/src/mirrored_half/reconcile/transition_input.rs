//! Validation of one membership-transition input against the accepted and staged documents.

use super::{
    FinalOwner, MembershipTransitionInput, TransitionState, ValidatedTransitionContext,
    board_owner, matrix_identity_slot, pair_right, require_matrix_pair, source_members,
    transition_matrix_by_id, validate_association_settings, validate_constraints,
    validate_context_owners, validate_distinct_associations, validate_matrix_association,
    validate_net_pad_ids, validate_no_native_legacy_overlap, validate_transition_part_geometry,
};
use crate::model::{MirroredItemKind, ProjectDoc};
use std::collections::{BTreeMap, BTreeSet};

pub(super) fn validate_transition_input<'a>(
    input: MembershipTransitionInput<'a>,
) -> Result<ValidatedTransitionContext<'a>, String> {
    let matrix_pair = require_matrix_pair(input.context)?;

    let context = input.context;
    validate_matrix_association(context, true)?;
    validate_association_settings(context)?;
    let (before_pair, before_mapping) = pair_right(input.valid_before, &context.pair_id)?;
    let (post_pair, post_mapping) = pair_right(input.post_source, &context.pair_id)?;
    if before_pair != post_pair
        || before_mapping != context.mapping
        || post_mapping != context.mapping
    {
        return Err("The transition pair record changed outside its private context".into());
    }
    validate_context_owners(input.valid_before, context)?;
    let snapshots = [input.valid_before, input.post_source];
    let snapshots = &snapshots[..];
    for document in snapshots {
        validate_net_pad_ids(document)?;
        validate_constraints(document)?;
        validate_no_native_legacy_overlap(document, context)?;
    }
    let before_halves = before_pair;
    if before_halves.source_board_id != context.source_board_id
        || before_halves.mirror_board_id != context.mirror_board_id
    {
        return Err("The transition context does not match its pair boards".into());
    }
    crate::mechanical::halves::validate_document(input.valid_before)?;

    let members = |document: &ProjectDoc, id: &str, board: &str, absent_allowed| {
        if absent_allowed && document.matrices.iter().all(|matrix| matrix.id != id) {
            return Ok(BTreeSet::new());
        }
        if transition_matrix_by_id(document, id)?.board_id.as_deref() != Some(board) {
            return Err("The transition matrices do not belong to the pair boards".into());
        }
        let members = source_members(document, id, context, true)?;
        validate_transition_part_geometry(document, &members, context)?;
        Ok::<_, String>(members)
    };
    let source = |document, absent_allowed| {
        members(
            document,
            &matrix_pair.source_id,
            &context.source_board_id,
            absent_allowed,
        )
    };
    let mirror = |document| {
        members(
            document,
            &matrix_pair.mirror_id,
            &context.mirror_board_id,
            context.matrix_state() == TransitionState::Omitted,
        )
    };
    let before_source_members = source(input.valid_before, false)?;
    let before_mirror_members = mirror(input.valid_before)?;
    mirror(input.post_source)?;
    let post_source_members = source(input.post_source, true)?;
    let removes_paired_matrices = input
        .post_source
        .matrices
        .iter()
        .all(|matrix| matrix.id != matrix_pair.source_id);
    if removes_paired_matrices {
        let source_members_retired = before_source_members.iter().all(|id| {
            input.source_identity_map.get(id) == Some(&None)
                && input.explicit_removal_closure.contains(id)
                && input.final_owners.get(id) == Some(&FinalOwner::Absent)
        });
        let mirror_members_resolved =
            before_mirror_members
                .iter()
                .all(|id| match input.mirror_identity_map.get(id) {
                    Some(None) => {
                        input.explicit_removal_closure.contains(id)
                            && input.final_owners.get(id) == Some(&FinalOwner::Absent)
                    }
                    Some(Some(target_id)) => context.associations.iter().any(|association| {
                        association.kind == MirroredItemKind::Part
                            && association.state == TransitionState::Local
                            && association.mirror_id == *id
                            && association.mirror_id == *target_id
                            && input.source_identity_map.get(&association.source_id) == Some(&None)
                            && input.final_owners.get(target_id)
                                == Some(&FinalOwner::Board(context.mirror_board_id.clone()))
                    }),
                    None => false,
                });
        if input.source_identity_map.values().any(Option::is_some)
            || !source_members_retired
            || !mirror_members_resolved
        {
            return Err(
                "Whole-matrix removal lacks complete source, mirror, and owner evidence".into(),
            );
        }
    }
    for association in context
        .associations
        .iter()
        .filter(|association| association.kind == MirroredItemKind::Part)
    {
        for (document, id) in snapshots.iter().flat_map(|&document| {
            [
                (document, association.source_id.as_str()),
                (document, association.mirror_id.as_str()),
            ]
        }) {
            let mut parts = document.parts.iter().filter(|part| part.id == id);
            if let Some(part) = parts.next()
                && (parts.next().is_some()
                    || !part.pose.at.x.is_finite()
                    || !part.pose.at.y.is_finite()
                    || !part.pose.rotation.is_finite())
            {
                return Err(format!(
                    "The associated Part {:?} has ambiguous or non-finite geometry",
                    id
                ));
            }
        }
    }
    let mapped_source_ids =
        validate_identity_map("source", input.source_identity_map, &before_source_members)?;
    let mapped_mirror_ids =
        validate_identity_map("mirror", input.mirror_identity_map, &before_mirror_members)?;
    let part_associations: Vec<_> = context
        .associations
        .iter()
        .filter(|association| association.kind == MirroredItemKind::Part)
        .collect();
    let associated_sources: BTreeSet<_> = part_associations
        .iter()
        .map(|association| association.source_id.clone())
        .collect();
    for association in &part_associations {
        if !context.owners.contains_key(&association.source_id)
            || !context.owners.contains_key(&association.mirror_id)
        {
            return Err(format!(
                "The historical owner context does not bind both sides of Part association {:?}",
                association.source_id
            ));
        }
    }
    if associated_sources.len() != part_associations.len() {
        return Err("The historical Part associations omit canonical source membership".into());
    }
    // Members added while their matrix link was detached (Local) or absent (Omitted) never
    // projected, so they hold no correspondence until an explicit Rejoin adopts them. They
    // may trim away without historical associations; anything else still refuses below.
    let matrix_unprojected = context.pair_right.exceptions.iter().any(|exception| {
        exception.kind == MirroredItemKind::Matrix
            && exception.mirror_id == matrix_pair.mirror_id
            && matches!(
                exception.state,
                crate::model::MirroredExceptionState::Local
                    | crate::model::MirroredExceptionState::Omitted
            )
    });
    if !before_source_members.is_subset(&associated_sources) && !matrix_unprojected {
        return Err("The historical Part associations omit canonical source membership".into());
    }
    for association in &part_associations {
        validate_slot_association(matrix_pair, association)?;
        if !before_source_members.contains(&association.source_id)
            && association.state != TransitionState::Local
        {
            return Err(format!(
                "Only a retained Local association may remain outside current source membership: {:?}",
                association.source_id
            ));
        }
        if association.state == TransitionState::Linked
            && !before_mirror_members.contains(&association.mirror_id)
        {
            return Err(format!(
                "The linked mirror identity {:?} is not an actual matrix member",
                association.mirror_id
            ));
        }
        let actual_mirror_parts = input
            .valid_before
            .parts
            .iter()
            .filter(|part| part.id == association.mirror_id)
            .count();
        if association.state == TransitionState::Local && actual_mirror_parts != 1 {
            return Err(format!(
                "The retained Local mirror identity {:?} has no unique actual Part",
                association.mirror_id
            ));
        }
        if association.state == TransitionState::Omitted && actual_mirror_parts > 1 {
            return Err(format!(
                "The materialized Omitted mirror identity {:?} is ambiguous",
                association.mirror_id
            ));
        }
    }
    let association_by_source: BTreeMap<_, _> = context
        .associations
        .iter()
        .filter(|association| association.kind == MirroredItemKind::Part)
        .map(|association| (association.source_id.as_str(), association))
        .collect();
    for (old_source, new_source) in input.source_identity_map {
        let Some(association) = association_by_source.get(old_source.as_str()) else {
            // Correspondence-less members of an unprojected matrix link carry no
            // association-dependent evidence; the injectivity check above already guards
            // their identity mapping.
            if matrix_unprojected {
                continue;
            }
            return Err(format!(
                "The source identity {old_source:?} has no private association"
            ));
        };
        if new_source.is_some() {
            validate_linked_placement(input.valid_before, context, association)?;
        }
        let expected_mirror = new_source.as_ref().and_then(|id| {
            id.strip_prefix(&format!("matrix/{}/", matrix_pair.source_id.as_str()))
                .map(|suffix| format!("matrix/{}/{suffix}", matrix_pair.mirror_id.as_str()))
        });
        let mapped_mirror = input.mirror_identity_map.get(&association.mirror_id);
        match association.state {
            TransitionState::Linked if mapped_mirror != Some(&expected_mirror) => {
                return Err(format!(
                    "The linked mirror identity disagrees with source identity {old_source:?}"
                ));
            }
            TransitionState::Local
                if new_source.is_some()
                    && before_mirror_members.contains(&association.mirror_id)
                    && mapped_mirror.is_none_or(Option::is_none) =>
            {
                return Err(format!(
                    "The retained Local identity {:?} has no mirror mapping while its source remains",
                    association.mirror_id
                ));
            }
            TransitionState::Omitted if mapped_mirror.is_some_and(Option::is_some) => {
                return Err(format!(
                    "The omitted mirror identity {:?} remains materialized",
                    association.mirror_id
                ));
            }
            TransitionState::Local
                if new_source.is_none() && mapped_mirror.is_some_and(Option::is_some) =>
            {
                let target = mapped_mirror.and_then(Option::as_ref).unwrap();
                if input.final_owners.get(target)
                    != Some(&FinalOwner::Board(context.mirror_board_id.clone()))
                {
                    return Err(format!(
                        "The retained Local identity {target:?} has no final mirror-board owner"
                    ));
                }
            }
            _ => {}
        }
    }
    let planned_source_ids: BTreeSet<_> = input
        .planned_additions
        .iter()
        .map(|addition| addition.source_id.clone())
        .collect();
    if planned_source_ids.len() != input.planned_additions.len() {
        return Err("The source operation map does not describe the post-operation matrix".into());
    }
    let mapped_or_planned: BTreeSet<_> = mapped_source_ids
        .union(&planned_source_ids)
        .cloned()
        .collect();
    if mapped_or_planned != post_source_members {
        // A detached mirror takes no projected members (spec §5): the only
        // permitted gap is genuine source-side growth under it.
        let uncovered: Vec<_> = post_source_members.difference(&mapped_or_planned).collect();
        let matrix_detached = context.matrix_state() != TransitionState::Linked;
        let source_growth = matrix_detached
            && !uncovered.is_empty()
            && uncovered.iter().all(|id| {
                matrix_identity_slot(id, matrix_pair.source_id.as_str()).is_some()
                    && !before_source_members.contains(*id)
            });
        if !source_growth {
            return Err(
                "The source operation map does not describe the post-operation matrix".into(),
            );
        }
    }

    validate_distinct_associations(context)?;
    for association in &context.associations {
        match (
            association.parent_source_id.as_deref(),
            association.parent_mirror_id.as_deref(),
        ) {
            (None, None) => {
                if association.state != TransitionState::Linked && !association.explicit_state {
                    return Err("A Local or Omitted owner lacks explicit state provenance".into());
                }
            }
            (Some(source), Some(mirror)) => {
                let parent = context
                    .associations
                    .iter()
                    .filter(|parent| {
                        parent.source_id == source
                            && parent.mirror_id == mirror
                            && (parent.kind == association.kind
                                || parent.kind == MirroredItemKind::Matrix)
                    })
                    .max_by_key(|parent| parent.kind == MirroredItemKind::Matrix)
                    .ok_or("The transition association has an unavailable parent")?;
                let prefix = if parent.kind == MirroredItemKind::Matrix {
                    "matrix/"
                } else {
                    ""
                };
                let source_suffix = association
                    .source_id
                    .strip_prefix(&format!("{prefix}{source}/"));
                if source_suffix.is_none()
                    || source_suffix
                        != association
                            .mirror_id
                            .strip_prefix(&format!("{prefix}{mirror}/"))
                {
                    return Err("The transition association has an invalid parent source".into());
                }
                if !association.explicit_state && association.state != parent.state {
                    return Err(
                        "An inherited child state disagrees with its parent association".into(),
                    );
                }
            }
            _ => return Err("The transition association has an incomplete parent closure".into()),
        }
        if association.state == TransitionState::Linked && association.explicit_state {
            return Err("The linked transition association has invalid state provenance".into());
        }
    }
    for old_id in input.explicit_removal_closure {
        let retained_omitted_identity = context.associations.iter().any(|association| {
            association.mirror_id == *old_id && association.state == TransitionState::Omitted
        });
        if !before_source_members.contains(old_id)
            && !before_mirror_members.contains(old_id)
            && !retained_omitted_identity
        {
            return Err(format!(
                "The removal closure contains unproven identity {old_id:?}"
            ));
        }
    }

    let mut required_owners: BTreeSet<_> = snapshots
        .iter()
        .flat_map(|snapshot| snapshot.parts.iter().map(|part| part.id.clone()))
        .collect();
    required_owners.extend(
        input
            .planned_additions
            .iter()
            .flat_map(|addition| [addition.source_id.clone(), addition.mirror_id.clone()]),
    );
    required_owners.extend(context.owners.keys().cloned());
    required_owners.extend(mapped_mirror_ids.iter().cloned());
    if input.final_owners.keys().cloned().collect::<BTreeSet<_>>() != required_owners {
        return Err("The final owner map is not total for the transition identity universe".into());
    }
    let mut transition_targets = mapped_source_ids.clone();
    for id in &mapped_source_ids {
        if let Some((row, column, child)) = matrix_identity_slot(id, matrix_pair.source_id.as_str())
        {
            let suffix = if child.is_empty() {
                String::new()
            } else {
                format!("/{child}")
            };
            transition_targets.insert(format!(
                "matrix/{}/r{row}c{column}{suffix}",
                matrix_pair.mirror_id.as_str()
            ));
        }
    }
    transition_targets.extend(
        input
            .planned_additions
            .iter()
            .flat_map(|addition| [addition.source_id.clone(), addition.mirror_id.clone()]),
    );
    for (id, owner) in input.final_owners {
        match owner {
            FinalOwner::Board(board_id) => {
                if !input
                    .post_source
                    .boards
                    .iter()
                    .any(|board| board.id == *board_id)
                {
                    return Err(format!("The final owner board {board_id:?} is unavailable"));
                }
                let expected_matrix_board =
                    if matrix_identity_slot(id, matrix_pair.source_id.as_str()).is_some() {
                        Some(context.source_board_id.as_str())
                    } else if matrix_identity_slot(id, matrix_pair.mirror_id.as_str()).is_some() {
                        Some(context.mirror_board_id.as_str())
                    } else {
                        None
                    };
                if expected_matrix_board.is_some_and(|expected| expected != board_id)
                    || (expected_matrix_board.is_none()
                        && !transition_targets.contains(id)
                        && board_owner(input.post_source, id)? != *owner)
                {
                    return Err(format!(
                        "The final owner for identity {id:?} contradicts its operation-bound board"
                    ));
                }
            }
            FinalOwner::Absent => {
                let remains_after_source =
                    input.post_source.parts.iter().any(|part| part.id == *id)
                        || input
                            .post_source
                            .boards
                            .iter()
                            .any(|board| board.part_ids.contains(id));
                if remains_after_source && !input.explicit_removal_closure.contains(id) {
                    return Err(format!(
                        "Absent identity {id:?} lacks explicit removal evidence"
                    ));
                }
            }
        }
    }

    let mut planned_source_ids = BTreeSet::new();
    let mut planned_mirror_ids = BTreeSet::new();
    let mut planned_references = BTreeSet::new();
    for addition in input.planned_additions {
        let source_parts: Vec<_> = input
            .post_source
            .parts
            .iter()
            .filter(|part| part.id == addition.source_id)
            .collect();
        let Some(source_part) = source_parts.first() else {
            return Err(format!(
                "The planned source Part {:?} is unavailable",
                addition.source_id
            ));
        };
        let source_slot = matrix_identity_slot(&addition.source_id, matrix_pair.source_id.as_str())
            .ok_or_else(|| {
                format!(
                    "The planned source identity {:?} is not canonical",
                    addition.source_id
                )
            })?;
        let mirror_slot = matrix_identity_slot(&addition.mirror_id, matrix_pair.mirror_id.as_str())
            .ok_or_else(|| {
                format!(
                    "The planned mirror identity {:?} is not canonical",
                    addition.mirror_id
                )
            })?;
        let expected_mirror = addition
            .source_id
            .strip_prefix(&format!("matrix/{}/", matrix_pair.source_id.as_str()))
            .map(|suffix| format!("matrix/{}/{suffix}", matrix_pair.mirror_id.as_str()));
        let expected_parents = if source_slot.2.is_empty() {
            (None, None)
        } else {
            let source_key = format!(
                "matrix/{}/r{}c{}",
                matrix_pair.source_id.as_str(),
                source_slot.0,
                source_slot.1
            );
            let mirror_key = format!(
                "matrix/{}/r{}c{}",
                matrix_pair.mirror_id.as_str(),
                mirror_slot.0,
                mirror_slot.1
            );
            (Some(source_key), Some(mirror_key))
        };
        if addition.source_id.is_empty()
            || addition.mirror_id.is_empty()
            || addition.source_reference.is_empty()
            || addition.mirror_reference.is_empty()
            || source_parts.len() != 1
            || source_part.reference != addition.source_reference
            || source_slot.0 != mirror_slot.0
            || source_slot.1 != mirror_slot.1
            || source_slot.2 != mirror_slot.2
            || expected_mirror.as_deref() != Some(addition.mirror_id.as_str())
            || (
                addition.parent_source_id.clone(),
                addition.parent_mirror_id.clone(),
            ) != expected_parents
            || !planned_source_ids.insert(addition.source_id.clone())
            || !planned_mirror_ids.insert(addition.mirror_id.clone())
            || !planned_references.insert(addition.source_reference.clone())
            || !planned_references.insert(addition.mirror_reference.clone())
            || input.valid_before.parts.iter().any(|part| {
                part.id == addition.source_id
                    || part.id == addition.mirror_id
                    || part.reference == addition.source_reference
                    || part.reference == addition.mirror_reference
            })
            || input.post_source.parts.iter().any(|part| {
                part.id == addition.mirror_id
                    || (part.id != addition.source_id
                        && part.reference == addition.source_reference)
                    || part.reference == addition.mirror_reference
            })
            || (!source_slot.2.is_empty()
                && (!input
                    .post_source
                    .parts
                    .iter()
                    .any(|part| part.id == expected_parents.0.as_deref().unwrap_or_default())
                    || !(input.planned_additions.iter().any(|parent| {
                        parent.source_id == expected_parents.0.as_deref().unwrap_or_default()
                            && parent.mirror_id == expected_parents.1.as_deref().unwrap_or_default()
                    }) || input
                        .valid_before
                        .parts
                        .iter()
                        .any(|part| part.id == expected_parents.1.as_deref().unwrap_or_default()))))
        {
            return Err("A planned transition addition collides or is incomplete".into());
        }
    }
    Ok(ValidatedTransitionContext {
        before_source_members,
        before_mirror_members,
        input,
        removes_paired_matrices,
        post_source_members,
        mapped_mirror_ids,
    })
}

fn validate_identity_map(
    side: &str,
    mapping: &BTreeMap<String, Option<String>>,
    members: &BTreeSet<String>,
) -> Result<BTreeSet<String>, String> {
    if let Some(unknown) = mapping.keys().find(|id| !members.contains(*id)) {
        return Err(format!(
            "The {side} identity map has unknown old identity {unknown:?}"
        ));
    }
    if mapping.len() != members.len() {
        return Err(format!(
            "The {side} identity map does not cover every old matrix member"
        ));
    }
    let targets: BTreeSet<_> = mapping.values().flatten().cloned().collect();
    if targets.len() != mapping.values().flatten().count() {
        return Err(format!("The {side} identity map is not injective"));
    }
    Ok(targets)
}

pub(super) fn validate_linked_placement(
    document: &crate::model::ProjectDoc,
    context: &super::MembershipContext,
    association: &super::TransitionAssociation,
) -> Result<(), String> {
    let pending_rejoin = context.pending_rejoins.contains_key(&association.source_id);
    if association.state == TransitionState::Linked
        && (crate::matrix::layout::companion_mode(document, &association.source_id)
            != crate::model::MatrixCompanionMode::Follows
            || (!pending_rejoin
                && crate::matrix::layout::companion_mode(document, &association.mirror_id)
                    != crate::model::MatrixCompanionMode::Follows))
    {
        return Err(
            "Linked matrix placements cannot retain authored or constrained geometry".into(),
        );
    }
    Ok(())
}

pub(super) fn validate_slot_association(
    matrix_pair: &super::MatrixPairContext,
    association: &super::TransitionAssociation,
) -> Result<(), String> {
    let (row, column, child) =
        matrix_identity_slot(&association.source_id, matrix_pair.source_id.as_str()).ok_or_else(
            || {
                format!(
                    "The associated source identity {:?} is not canonical",
                    association.source_id
                )
            },
        )?;
    let source_key = crate::matrix::member_id(&matrix_pair.source_id, row, column);
    let mirror_key = crate::matrix::member_id(&matrix_pair.mirror_id, row, column);
    let parents = (
        association.parent_source_id.as_deref(),
        association.parent_mirror_id.as_deref(),
    );
    let valid = if child.is_empty() {
        association.mirror_id == mirror_key
            && (parents == (None, None)
                || parents
                    == (
                        Some(matrix_pair.source_id.as_str()),
                        Some(matrix_pair.mirror_id.as_str()),
                    ))
    } else {
        association.mirror_id == crate::matrix::companion_id(&mirror_key, child)
            && parents == (Some(source_key.as_str()), Some(mirror_key.as_str()))
    };
    if !valid {
        return Err(format!(
            "The Part association {:?} disagrees with canonical membership or parent identity",
            association.source_id
        ));
    }
    Ok(())
}
