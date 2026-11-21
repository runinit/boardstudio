//! Validation helpers and output emission for the membership-transition kernel.

use super::{
    FinalOwner, MembershipContext, MembershipContextDelta, MembershipTransitionOutput,
    TransitionState, ValidatedTransitionContext, changed_document_fields,
    exact_omitted_matrix_companions, lifecycle, matrix_identity_slot, require_matrix_pair,
};
use crate::model::{Matrix, MirroredExceptionState, MirroredItemKind, Pose2, ProjectDoc, Vec2};
use std::collections::{BTreeMap, BTreeSet};

pub(super) fn pair_right<'a>(
    document: &'a ProjectDoc,
    pair_id: &str,
) -> Result<(&'a crate::model::MirroredHalves, Pose2), String> {
    let hardware = document
        .hardware
        .as_ref()
        .ok_or_else(|| "The transition hardware configuration is unavailable".to_owned())?;
    let mut pairs = hardware.pairs.iter().filter(|pair| pair.id == pair_id);
    let pair = pairs
        .next()
        .ok_or_else(|| format!("The transition pair {pair_id:?} is unavailable"))?;
    if pairs.next().is_some() {
        return Err(format!("The transition pair {pair_id:?} is ambiguous"));
    }
    let halves = pair
        .halves
        .as_ref()
        .ok_or_else(|| format!("The transition pair {pair_id:?} has no halves record"))?;
    Ok((halves, pair.mapping))
}

pub(super) fn transition_matrix_by_id<'a>(
    document: &'a ProjectDoc,
    id: &str,
) -> Result<&'a Matrix, String> {
    let mut matrices = document.matrices.iter().filter(|matrix| matrix.id == id);
    let matrix = matrices
        .next()
        .ok_or_else(|| format!("The transition matrix {id:?} is unavailable"))?;
    if matrices.next().is_some() {
        return Err(format!("The transition matrix {id:?} is ambiguous"));
    }
    Ok(matrix)
}

pub(super) fn registration_count(document: &ProjectDoc, id: &str) -> usize {
    document
        .boards
        .iter()
        .map(|board| {
            board
                .part_ids
                .iter()
                .filter(|part_id| part_id.as_str() == id)
                .count()
        })
        .sum()
}

pub(super) fn board_owner(document: &ProjectDoc, id: &str) -> Result<FinalOwner, String> {
    if registration_count(document, id) != 1 {
        return Err(format!(
            "The transition part {id:?} must have exactly one actual board registration"
        ));
    }
    let board = document
        .boards
        .iter()
        .find(|board| board.part_ids.iter().any(|part_id| part_id == id))
        .ok_or_else(|| format!("The transition part {id:?} has no board owner"))?;
    Ok(FinalOwner::Board(board.id.clone()))
}

pub(super) fn validate_owner_realization(
    document: &ProjectDoc,
    owners: &BTreeMap<String, FinalOwner>,
) -> Result<(), String> {
    for (id, owner) in owners {
        match owner {
            FinalOwner::Board(_) => {
                if document.parts.iter().filter(|part| part.id == *id).count() != 1
                    || board_owner(document, id)? != *owner
                {
                    return Err(format!(
                        "Board-owned identity {id:?} does not have exactly one matching Part registration"
                    ));
                }
            }
            FinalOwner::Absent => {
                if document.parts.iter().any(|part| part.id == *id)
                    || document
                        .boards
                        .iter()
                        .any(|board| board.part_ids.iter().any(|part_id| part_id == id))
                {
                    return Err(format!(
                        "Absent identity {id:?} still has a Part or board registration"
                    ));
                }
            }
        }
    }
    Ok(())
}

pub(super) fn validate_context_owners(
    document: &ProjectDoc,
    context: &MembershipContext,
) -> Result<(), String> {
    let mut part_ids = BTreeSet::new();
    for part in &document.parts {
        if !part_ids.insert(part.id.as_str()) {
            return Err(format!(
                "The historical Part identity {:?} is duplicated",
                part.id
            ));
        }
        let actual = board_owner(document, &part.id)?;
        if context.owners.get(&part.id) != Some(&actual)
            && !(actual == FinalOwner::Board(context.mirror_board_id.clone())
                && context.owners.get(&part.id) == Some(&FinalOwner::Absent)
                && lifecycle::pending_visible_omission(context, &part.id))
        {
            return Err(format!(
                "The historical owner context for {:?} does not match its registered board",
                part.id
            ));
        }
    }
    for (id, owner) in &context.owners {
        match owner {
            FinalOwner::Board(_) => {
                let pending_omitted_rejoin = context.associations.iter().any(|association| {
                    association.kind == MirroredItemKind::Part
                        && association.mirror_id == *id
                        && association.state == TransitionState::Linked
                        && context.pending_rejoins.get(&association.source_id)
                            == Some(&TransitionState::Omitted)
                });
                if pending_omitted_rejoin
                    && document.parts.iter().all(|part| part.id != *id)
                    && registration_count(document, id) == 0
                {
                    continue;
                }
                if board_owner(document, id)? != *owner
                    || document.parts.iter().filter(|part| part.id == *id).count() != 1
                {
                    return Err(format!(
                        "The historical owner context for {id:?} is not realized exactly once"
                    ));
                }
            }
            FinalOwner::Absent => {
                if (document.parts.iter().any(|part| part.id == *id)
                    || document
                        .boards
                        .iter()
                        .any(|board| board.part_ids.iter().any(|part_id| part_id == id)))
                    && !lifecycle::pending_visible_omission(context, id)
                {
                    return Err(format!(
                        "The historically absent identity {id:?} still has a Part registration"
                    ));
                }
            }
        }
    }
    Ok(())
}

pub(super) fn validate_constraints(document: &ProjectDoc) -> Result<(), String> {
    for constraint in &document.constraints {
        for endpoint in [constraint.source(), constraint.target()] {
            if document
                .parts
                .iter()
                .filter(|part| part.id == endpoint)
                .count()
                != 1
            {
                return Err(format!(
                    "Constraint {:?} must reference exactly one actual Part at {:?}",
                    constraint.id(),
                    endpoint
                ));
            }
        }
        if !constraint.is_finite() {
            return Err(format!(
                "Constraint {:?} contains non-finite geometry",
                constraint.id()
            ));
        }
    }
    Ok(())
}

pub(super) fn validate_transition_part_geometry(
    document: &ProjectDoc,
    members: &BTreeSet<String>,
    context: &MembershipContext,
) -> Result<(), String> {
    for id in members {
        let mut parts = document.parts.iter().filter(|part| part.id == *id);
        let Some(part) = parts.next() else {
            let pending_omitted_rejoin = context.associations.iter().any(|association| {
                association.kind == MirroredItemKind::Part
                    && association.mirror_id == *id
                    && association.state == TransitionState::Linked
                    && context.pending_rejoins.get(&association.source_id)
                        == Some(&TransitionState::Omitted)
                    && context.owners.get(id)
                        == Some(&FinalOwner::Board(context.mirror_board_id.clone()))
                    && context.pair_right.correspondences.iter().any(|record| {
                        record.kind == MirroredItemKind::Part
                            && record.source_id == association.source_id
                            && record.mirror_id == association.mirror_id
                            && record.companion_mode == association.raw_mode_override
                            && record.companion_tweak == association.raw_tweak
                    })
            });
            if pending_omitted_rejoin {
                continue;
            }
            let mut omitted = context.associations.iter().filter(|association| {
                association.kind == MirroredItemKind::Part
                    && association.mirror_id == *id
                    && association.state == TransitionState::Omitted
                    && context.owners.get(id) == Some(&FinalOwner::Absent)
            });
            let Some(association) = omitted.next() else {
                return Err(format!(
                    "Transition matrix member {:?} has no actual Part",
                    id
                ));
            };
            if omitted.next().is_some() {
                return Err(format!(
                    "Transition matrix member {:?} has ambiguous Omitted ownership",
                    id
                ));
            }
            let explicit_owner = if association.explicit_state {
                Some(association)
            } else {
                association
                    .parent_source_id
                    .as_deref()
                    .and_then(|parent_source| {
                        let parent_mirror = association.parent_mirror_id.as_deref()?;
                        context.associations.iter().find(|parent| {
                            parent.kind == MirroredItemKind::Part
                                && parent.source_id == parent_source
                                && parent.mirror_id == parent_mirror
                                && parent.state == TransitionState::Omitted
                                && parent.explicit_state
                        })
                    })
            };
            let Some(owner) = explicit_owner else {
                return Err(format!(
                    "Absent transition matrix member {:?} lacks an explicit Omitted owner",
                    id
                ));
            };
            let owner_is_durable = context.pair_right.exceptions.iter().any(|exception| {
                exception.kind == MirroredItemKind::Part
                    && context.pair_right.overlay_source(exception)
                        == Some(owner.source_id.as_str())
                    && exception.mirror_id == owner.mirror_id
                    && exception.state == crate::model::MirroredExceptionState::Omitted
            });
            let child_correspondence_is_retained = association.source_id == owner.source_id
                || context.pair_right.correspondences.iter().any(|record| {
                    record.kind == MirroredItemKind::Part
                        && record.source_id == association.source_id
                        && record.mirror_id == association.mirror_id
                        && record.companion_mode == association.raw_mode_override
                        && record.companion_tweak == association.raw_tweak
                });
            if !owner_is_durable || !child_correspondence_is_retained {
                return Err(format!(
                    "Absent transition matrix member {:?} lacks exact durable Omitted-owner provenance",
                    id
                ));
            }
            continue;
        };
        if parts.next().is_some()
            || !part.pose.at.x.is_finite()
            || !part.pose.at.y.is_finite()
            || !part.pose.rotation.is_finite()
        {
            return Err(format!(
                "Transition matrix member {:?} has ambiguous or non-finite geometry",
                id
            ));
        }
    }
    Ok(())
}

pub(super) fn validate_net_pad_ids(document: &ProjectDoc) -> Result<(), String> {
    for net in &document.nets {
        for pin in &net.pins {
            let mut parts = document.parts.iter().filter(|part| part.id == pin.part_id);
            let part = parts.next().ok_or_else(|| {
                format!("Net {:?} references missing Part {:?}", net.id, pin.part_id)
            })?;
            if parts.next().is_some() {
                return Err(format!(
                    "Net {:?} references ambiguous Part {:?}",
                    net.id, pin.part_id
                ));
            }
            let mut definitions = document
                .definitions
                .iter()
                .filter(|definition| definition.id == part.definition_id);
            let definition = definitions.next().ok_or_else(|| {
                format!(
                    "Net {:?} references Part {:?} with no definition",
                    net.id, part.id
                )
            })?;
            if definitions.next().is_some()
                || !definition.pads.iter().any(|pad| pad.id == pin.pad_id)
            {
                return Err(format!(
                    "Net {:?} pin {:?} does not reference an actual Pad.id",
                    net.id, pin.pad_id
                ));
            }
        }
    }
    Ok(())
}

pub(super) fn validate_no_native_legacy_overlap(
    document: &ProjectDoc,
    context: &MembershipContext,
) -> Result<(), String> {
    let matrix_pair = require_matrix_pair(context)?;

    for linked_layout in document
        .layouts
        .iter()
        .filter(|layout| layout.mirror_link.is_some())
    {
        let link = linked_layout.mirror_link.as_ref().unwrap();
        let source_layout = document
            .layouts
            .iter()
            .find(|layout| layout.id == link.source_id)
            .ok_or_else(|| "A legacy mirrored layout has no source layout".to_owned())?;
        if linked_layout
            .matrix_ids
            .iter()
            .chain(&source_layout.matrix_ids)
            .any(|id| id == &matrix_pair.source_id || id == &matrix_pair.mirror_id)
        {
            return Err("A matrix cannot use native and legacy mirror links together".into());
        }
    }
    Ok(())
}

pub(super) fn validate_matrix_association(
    context: &MembershipContext,
    expected_present: bool,
) -> Result<(), String> {
    let matrix_pair = require_matrix_pair(context)?;

    let mut associations = context.associations.iter().filter(|association| {
        association.kind == MirroredItemKind::Matrix
            && association.source_id == matrix_pair.source_id
            && association.mirror_id == matrix_pair.mirror_id
    });
    let association = associations.next();
    if associations.next().is_some() {
        return Err("The selected transition matrix association is ambiguous".into());
    }
    let mut correspondences = context.pair_right.correspondences.iter().filter(|record| {
        record.kind == MirroredItemKind::Matrix
            && record.source_id == matrix_pair.source_id
            && record.mirror_id == matrix_pair.mirror_id
    });
    let correspondence = correspondences.next();
    if correspondences.next().is_some() {
        return Err("The selected transition matrix association is ambiguous".into());
    }
    if !expected_present {
        if association.is_none()
            && correspondence.is_none()
            && !context
                .pair_right
                .exceptions
                .iter()
                .any(|e| e.kind == MirroredItemKind::Matrix)
        {
            return Ok(());
        }
        let correspondence = correspondence
            .ok_or_else(|| "The persisted pair matrix correspondence is unavailable".to_owned())?;
        if association.is_some()
            || correspondence.source_id != matrix_pair.source_id.as_str()
            || correspondence.mirror_id != matrix_pair.mirror_id.as_str()
        {
            return Err("The retired matrix context disagrees with persisted provenance".into());
        }
        return Ok(());
    }
    let association =
        association.ok_or_else(|| "The transition matrix association is unavailable".to_owned())?;
    let correspondence =
        correspondence.ok_or_else(|| "The pair matrix correspondence is unavailable".to_owned())?;
    // A matrix-level Local or Omitted exception is an explicit overlay on
    // the retained correspondence (spec §5): the mirror owns its
    // descriptor while the pair record stays intact. Local keeps its
    // geometry; Omitted stays live-yet-absent.
    let linked = association.state == TransitionState::Linked && !association.explicit_state;
    let matrix_exception = matches!(
        association.state,
        TransitionState::Local | TransitionState::Omitted
    ) && association.explicit_state;
    if association.source_id != matrix_pair.source_id.as_str()
        || association.mirror_id != matrix_pair.mirror_id.as_str()
        || !(linked || matrix_exception)
        || association.parent_source_id.is_some()
        || association.parent_mirror_id.is_some()
        || correspondence.source_id != association.source_id
        || correspondence.mirror_id != association.mirror_id
        || correspondence.companion_mode != association.raw_mode_override
        || correspondence.companion_tweak != association.raw_tweak
    {
        return Err(
            "The transition matrix association disagrees with its actual pair correspondence"
                .into(),
        );
    }
    Ok(())
}

pub(super) fn source_members(
    document: &ProjectDoc,
    matrix_id: &str,
    context: &MembershipContext,
    accept_previous_omitted: bool,
) -> Result<BTreeSet<String>, String> {
    let matrix = transition_matrix_by_id(document, matrix_id)?;
    crate::matrix::valid_matrix(matrix, document)?;
    let mut members = BTreeSet::new();
    let mut companion_members = BTreeSet::new();
    for cell in &matrix.cells {
        if cell.deleted || !cell.enabled {
            continue;
        }
        let key_id = format!("matrix/{}/r{}c{}", matrix.id, cell.row, cell.column);
        members.insert(key_id.clone());
        for assembly in &cell.assemblies {
            let companion_id = crate::matrix::companion_id(&key_id, &assembly.id);
            members.insert(companion_id.clone());
            companion_members.insert(companion_id);
        }
    }
    let has_omitted_matrix_members = context.pair_right.exceptions.iter().any(|exception| {
        exception.kind == MirroredItemKind::Part
            && exception.state == MirroredExceptionState::Omitted
            && companion_members.contains(&exception.mirror_id)
            && context.associations.iter().any(|association| {
                association.kind == MirroredItemKind::Part
                    && association.mirror_id == exception.mirror_id
            })
    });
    if has_omitted_matrix_members
        && context
            .matrix_pair
            .as_ref()
            .is_some_and(|pair| pair.mirror_id == matrix_id)
    {
        let matrix_pair = require_matrix_pair(context)?;
        let source = transition_matrix_by_id(document, matrix_pair.source_id.as_str())?;
        for omitted in exact_omitted_matrix_companions(source, matrix, &context.pair_right)? {
            let associations: Vec<_> = context
                .associations
                .iter()
                .filter(|association| {
                    association.kind == MirroredItemKind::Part
                        && association.source_id == omitted.source_id
                        && association.mirror_id == omitted.mirror_id
                })
                .collect();
            let [association] = associations.as_slice() else {
                continue;
            };
            let omitted_state = if accept_previous_omitted {
                context
                    .pending_pair_transitions
                    .get(&association.source_id)
                    .or_else(|| context.pending_rejoins.get(&association.source_id))
                    .copied()
                    .unwrap_or({
                        if association.explicit_state {
                            association.state
                        } else {
                            TransitionState::Linked
                        }
                    })
            } else {
                association.state
            };
            if omitted_state == TransitionState::Omitted
                && (association.explicit_state
                    || context
                        .pending_pair_transitions
                        .contains_key(&association.source_id)
                    || context.pending_rejoins.contains_key(&association.source_id))
                && association.parent_source_id.as_deref() == Some(omitted.source_key_id.as_str())
                && association.parent_mirror_id.as_deref() == Some(omitted.mirror_key_id.as_str())
            {
                members.remove(&omitted.mirror_id);
            }
        }
    }
    let actual: BTreeSet<_> = matrix.part_ids.iter().cloned().collect();
    if members != actual || actual.len() != matrix.part_ids.len() {
        return Err("The source matrix membership is not canonical".into());
    }
    Ok(members)
}

pub(super) fn finite_tweak(tweak: Option<crate::model::MirroredCompanionTweak>) -> bool {
    tweak.is_none_or(|value| {
        value.x.is_finite() && value.y.is_finite() && value.rotation.is_finite()
    })
}

pub(super) fn validate_association_settings(context: &MembershipContext) -> Result<(), String> {
    for association in &context.associations {
        let effective_mode = association
            .raw_mode_override
            .unwrap_or(context.pair_right.companion_mode);
        let effective_tweak = association.raw_tweak;
        if association.source_id.is_empty()
            || association.mirror_id.is_empty()
            || !finite_tweak(association.raw_tweak)
            || !finite_tweak(association.effective_tweak)
            || association.effective_mode != effective_mode
            || association.effective_tweak != effective_tweak
            || (association.effective_mode
                != crate::model::MirroredCompanionMode::LiteralMirrorWithTweak
                && association.effective_tweak.is_some())
        {
            return Err("The transition association context has invalid settings".into());
        }
    }
    Ok(())
}

pub(super) fn cell_offset_delta(
    matrix: &Matrix,
    row: u32,
    column: u32,
    world: Vec2,
    doc: &ProjectDoc,
) -> Result<Vec2, String> {
    let evaluated = crate::matrix::evaluated(matrix, doc)?;
    let cell = evaluated
        .cells
        .iter()
        .find(|c| c.row == row as usize && c.column == column as usize)
        .ok_or("Local key frame is missing")?;
    Ok(crate::matrix::evaluation::inverse_offset(
        &cell.offset_frame,
        world,
    ))
}

pub(super) fn emit_validated_transition(
    validated: &ValidatedTransitionContext<'_>,
    draft: ProjectDoc,
    next_context: MembershipContext,
    context_delta: MembershipContextDelta,
    expected_mirror_members: &BTreeSet<String>,
) -> Result<MembershipTransitionOutput, String> {
    let input = validated.input;
    let matrix_pair = require_matrix_pair(input.context)?;
    validate_matrix_association(input.context, !validated.already_removed)?;
    // A retained Local mirror keeps its record across its source removal,
    // so only the complete-removal arm is skipped below; the gate still
    // validates the retired pair side as absent.
    let matrix_retained_local = input.context.associations.iter().any(|association| {
        association.kind == MirroredItemKind::Matrix
            && association.source_id == matrix_pair.source_id.as_str()
            && association.mirror_id == matrix_pair.mirror_id.as_str()
            && association.state == TransitionState::Local
            && association.explicit_state
    });
    validate_matrix_association(&next_context, !validated.removes_paired_matrices)?;
    validate_association_settings(input.context)?;
    validate_association_settings(&next_context)?;
    if validated.removes_paired_matrices && !matrix_retained_local {
        if draft.matrices.iter().any(|matrix| {
            matrix.id == matrix_pair.source_id.as_str()
                || matrix.id == matrix_pair.mirror_id.as_str()
        }) || !expected_mirror_members.is_empty()
        {
            return Err("The emitted whole-matrix removal is incomplete".into());
        }
    } else {
        // A retained Local mirror has no source left to check; its mirror
        // must match the validated plan below.
        if !validated.removes_paired_matrices {
            let source = transition_matrix_by_id(&draft, matrix_pair.source_id.as_str())?;
            crate::matrix::valid_matrix(source, &draft)?;
            if source_members(&draft, matrix_pair.source_id.as_str(), &next_context, false)?
                != validated.post_source_members
            {
                return Err(
                    "The emitted source matrix disagrees with its validated operation map".into(),
                );
            }
        }
        // A matrix-level Omitted mirror is record-absent by design: an
        // absent record must pair with an empty member plan.
        let mirror_absent = next_context.associations.iter().any(|association| {
            association.kind == MirroredItemKind::Matrix
                && association.source_id == matrix_pair.source_id.as_str()
                && association.mirror_id == matrix_pair.mirror_id.as_str()
                && association.state == TransitionState::Omitted
                && association.explicit_state
        }) && draft
            .matrices
            .iter()
            .all(|matrix| matrix.id != matrix_pair.mirror_id.as_str());
        if mirror_absent {
            if !expected_mirror_members.is_empty() {
                return Err(
                    "The emitted mirror membership disagrees with its validated plan".into(),
                );
            }
        } else {
            let mirror = transition_matrix_by_id(&draft, matrix_pair.mirror_id.as_str())?;
            crate::matrix::valid_matrix(mirror, &draft)?;
            let actual_mirror_members: BTreeSet<_> = mirror.part_ids.iter().cloned().collect();
            if actual_mirror_members.len() != mirror.part_ids.len()
                || actual_mirror_members != *expected_mirror_members
            {
                return Err(
                    "The emitted mirror membership disagrees with its validated plan".into(),
                );
            }
        }
    }
    if next_context.owners != *input.final_owners || context_delta.owners != *input.final_owners {
        return Err(
            "The emitted context and owner result disagree with the validated final owners".into(),
        );
    }
    validate_context_owners(&draft, &next_context)?;
    validate_owner_realization(&draft, input.final_owners)?;
    let association_keys: BTreeSet<_> = next_context
        .associations
        .iter()
        .map(|association| format!("{:?}\0{}", association.kind, association.source_id))
        .collect();
    let association_mirrors: BTreeSet<_> = next_context
        .associations
        .iter()
        .map(|association| format!("{:?}\0{}", association.kind, association.mirror_id))
        .collect();
    if association_keys.len() != next_context.associations.len()
        || association_mirrors.len() != next_context.associations.len()
    {
        return Err("The emitted transition associations contain aliased identities".into());
    }
    for association in next_context
        .associations
        .iter()
        .filter(|association| association.kind == MirroredItemKind::Part)
    {
        if association.state == TransitionState::Linked {
            let Some((row, column, child)) =
                matrix_identity_slot(&association.source_id, matrix_pair.source_id.as_str())
            else {
                return Err(format!(
                    "The emitted linked source identity {:?} is not canonical",
                    association.source_id
                ));
            };
            let suffix = if child.is_empty() {
                String::new()
            } else {
                format!("/{child}")
            };
            let expected_mirror = format!(
                "matrix/{}/r{}c{}{}",
                matrix_pair.mirror_id.as_str(),
                row,
                column,
                suffix
            );
            if !validated
                .post_source_members
                .contains(&association.source_id)
                || association.mirror_id != expected_mirror
                || !expected_mirror_members.contains(&association.mirror_id)
            {
                return Err(format!(
                    "The emitted linked association {:?} disagrees with actual matrix membership",
                    association.source_id
                ));
            }
        } else if association.state == TransitionState::Omitted
            && expected_mirror_members.contains(&association.mirror_id)
        {
            return Err(format!(
                "The emitted Omitted identity {:?} remains in mirror membership",
                association.mirror_id
            ));
        }
    }
    for association in next_context
        .associations
        .iter()
        .filter(|association| association.kind == MirroredItemKind::Part)
    {
        let (row, column, child) =
            matrix_identity_slot(&association.source_id, matrix_pair.source_id.as_str())
                .ok_or_else(|| {
                    format!(
                        "The emitted Part association {:?} has no canonical source slot",
                        association.source_id
                    )
                })?;
        let suffix = if child.is_empty() {
            String::new()
        } else {
            format!("/{child}")
        };
        let expected_mirror = format!(
            "matrix/{}/r{}c{}{}",
            matrix_pair.mirror_id.as_str(),
            row,
            column,
            suffix
        );
        let expected_parents = if child.is_empty() {
            (None, None)
        } else {
            (
                Some(format!(
                    "matrix/{}/r{}c{}",
                    matrix_pair.source_id.as_str(),
                    row,
                    column
                )),
                Some(format!(
                    "matrix/{}/r{}c{}",
                    matrix_pair.mirror_id.as_str(),
                    row,
                    column
                )),
            )
        };
        // A top-level key owned by its matrix pair names that pair (spec
        // §5) instead of standing parentless.
        let expected_matrix_parents = (
            Some(matrix_pair.source_id.clone()),
            Some(matrix_pair.mirror_id.clone()),
        );
        if ((
            association.parent_source_id.clone(),
            association.parent_mirror_id.clone(),
        ) != expected_parents
            && !(child.is_empty()
                && (
                    association.parent_source_id.clone(),
                    association.parent_mirror_id.clone(),
                ) == expected_matrix_parents))
            || (child.is_empty() && association.mirror_id != expected_mirror)
        {
            return Err(format!(
                "The emitted Part association {:?} has inconsistent parent or mirror identity",
                association.source_id
            ));
        }
        if let (Some(parent_source), Some(parent_mirror)) = expected_parents
            && !next_context.associations.iter().any(|parent| {
                parent.kind == MirroredItemKind::Part
                    && parent.source_id == parent_source
                    && parent.mirror_id == parent_mirror
            })
        {
            return Err(format!(
                "The emitted Part association {:?} lost its actual parent closure",
                association.source_id
            ));
        }
        match association.state {
            TransitionState::Linked => {
                if association.mirror_id != expected_mirror
                    || !validated
                        .post_source_members
                        .contains(&association.source_id)
                    || !expected_mirror_members.contains(&association.mirror_id)
                {
                    return Err(format!(
                        "The emitted linked association {:?} disagrees with actual matrix membership",
                        association.source_id
                    ));
                }
            }
            TransitionState::Local => {
                if draft
                    .parts
                    .iter()
                    .filter(|part| part.id == association.mirror_id)
                    .count()
                    != 1
                    || board_owner(&draft, &association.mirror_id)?
                        != FinalOwner::Board(input.context.mirror_board_id.clone())
                {
                    return Err(format!(
                        "The emitted Local identity {:?} lacks its unique mirror-board Part realization",
                        association.mirror_id
                    ));
                }
                if validated
                    .post_source_members
                    .contains(&association.source_id)
                    && expected_mirror_members.contains(&association.mirror_id)
                    && association.mirror_id != expected_mirror
                {
                    return Err(format!(
                        "The emitted Local association {:?} disagrees with its canonical mirror identity",
                        association.source_id
                    ));
                }
            }
            TransitionState::Omitted => {
                if expected_mirror_members.contains(&association.mirror_id)
                    || draft
                        .parts
                        .iter()
                        .any(|part| part.id == association.mirror_id)
                    || registration_count(&draft, &association.mirror_id) != 0
                {
                    return Err(format!(
                        "The emitted Omitted identity {:?} remains materialized",
                        association.mirror_id
                    ));
                }
            }
        }
    }
    let (changed_document_ids, changed_field_paths) =
        changed_document_fields(input.post_source, &draft)?;
    Ok(MembershipTransitionOutput {
        draft,
        changed_document_ids,
        changed_field_paths,
        context: next_context,
        context_delta,
        final_owners: input.final_owners.clone(),
        notices: Vec::new(),
    })
}
