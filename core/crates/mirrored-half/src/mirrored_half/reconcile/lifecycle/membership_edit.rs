//! Lifecycle contexts, matrix pair association and matrix membership edits.

use super::super::{MembershipContext, TransitionState};
use super::effective;
use crate::model::{MirroredExceptionState, MirroredItemKind};

pub(super) fn lifecycle_context(
    document: &crate::model::ProjectDoc,
    pair_id: &str,
    matrix_pair: Option<super::super::MatrixPairContext>,
) -> Result<MembershipContext, String> {
    use super::super::{FinalOwner, TransitionAssociation};
    use crate::model::{MirroredExceptionState, MirroredItemKind};
    use std::collections::BTreeMap;

    let pair = super::super::selected_pair(document, pair_id, "lifecycle")?;
    let halves = pair
        .halves
        .as_ref()
        .ok_or("The exact lifecycle pair has no halves record")?;

    fn parent_ids(
        document: &crate::model::ProjectDoc,
        halves: &crate::model::MirroredHalves,
        record: &crate::model::MirroredCorrespondence,
    ) -> Result<(Option<String>, Option<String>), String> {
        if let Some(owners) = super::super::matrix_owners(document, halves, record) {
            let parent = owners.key.unwrap_or(owners.matrix);
            return Ok((
                Some(parent.source_id.clone()),
                Some(parent.mirror_id.clone()),
            ));
        }
        // An unowned pair of canonical slot IDs can be a stale durable association.
        // Keep the same admission check for all records, including document replacement.
        if record.kind == MirroredItemKind::Part
            && halves
                .correspondences
                .iter()
                .any(|matrix| claims_matrix_slot(matrix, &record.source_id, &record.mirror_id))
        {
            return Err("A matrix-owned Part lacks its exact source slot or assembly".into());
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
                    return Ok((
                        Some(circuit.source_id.clone()),
                        Some(circuit.mirror_id.clone()),
                    ));
                }
            }
        }
        Ok((None, None))
    }

    let associations = halves
        .correspondences
        .iter()
        .map(|record| {
            let exception = halves.overlay_of(record);
            let state = match exception.map(|exception| exception.state) {
                Some(MirroredExceptionState::Local) => TransitionState::Local,
                Some(MirroredExceptionState::Omitted) => TransitionState::Omitted,
                None => TransitionState::Linked,
            };
            let (parent_source_id, parent_mirror_id) = parent_ids(document, halves, record)?;
            Ok(TransitionAssociation {
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
            })
        })
        .collect::<Result<_, String>>()?;
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
    };
    for index in 0..context.associations.len() {
        let state = effective(&context, &context.associations[index], false, 0)?;
        context.associations[index].state = state;
    }
    // Bind all historical board registrations. Sparse Omitted links supply the
    // only identities without an actual Part registration.
    for part in &document.parts {
        let board = super::super::super::board_for_part(document, &part.id)
            .ok_or_else(|| format!("Lifecycle Part {:?} has no board owner", part.id))?;
        context
            .owners
            .insert(part.id.clone(), FinalOwner::Board(board.id.clone()));
    }
    for association in context
        .associations
        .iter()
        .filter(|a| a.kind == MirroredItemKind::Part)
    {
        for id in [&association.source_id, &association.mirror_id] {
            if id == &association.mirror_id && association.state == TransitionState::Omitted {
                context.owners.insert(id.clone(), FinalOwner::Absent);
            } else if !context.owners.contains_key(id) {
                return Err(format!("Lifecycle Part {id:?} has no board owner"));
            }
        }
    }
    Ok(context)
}

// A matching pair of generated IDs claims a slot at admission. Only actual membership
// establishes an owner for inheritance and inverse editing.
fn claims_matrix_slot(
    matrix: &crate::model::MirroredCorrespondence,
    source_id: &str,
    mirror_id: &str,
) -> bool {
    if matrix.kind != MirroredItemKind::Matrix {
        return false;
    }
    let Some((row, column, assembly)) =
        super::super::matrix_identity_slot(source_id, &matrix.source_id)
    else {
        return false;
    };
    let key = crate::matrix::member_id(&matrix.mirror_id, row, column);
    if assembly.is_empty() {
        mirror_id == key
    } else {
        mirror_id == crate::matrix::companion_id(&key, assembly)
    }
}

pub(super) fn matrix_pair_for_association(
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
        .filter(|matrix| claims_matrix_slot(matrix, source_id, mirror_id))
        .map(|matrix| super::super::MatrixPairContext {
            source_id: matrix.source_id.clone(),
            mirror_id: matrix.mirror_id.clone(),
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
        FinalOwner, MembershipTransitionInput, PlannedAddition, TransitionState,
        changes::changed_document_ids,
        transition::{RemovedAxis, prepare_membership_projection},
    };
    use crate::model::{MirroredItemKind, ProjectDoc};
    use std::borrow::Cow;
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

    fn selected_matrix_document<'a>(
        document: &'a ProjectDoc,
        context: &MembershipContext,
        retired_layouts: &[&super::super::TransitionAssociation],
    ) -> Result<Cow<'a, ProjectDoc>, String> {
        let hardware = document
            .hardware
            .as_ref()
            .ok_or("Transition hardware is unavailable")?;
        let mut pairs = hardware
            .pairs
            .iter()
            .enumerate()
            .filter(|(_, pair)| pair.id == context.pair_id);
        let (index, pair) = pairs
            .next()
            .ok_or("The exact transition pair is unavailable")?;
        if pairs.next().is_some() {
            return Err("The exact transition pair is ambiguous".into());
        }
        let halves = pair
            .halves
            .as_ref()
            .ok_or("The exact transition pair has no halves record")?;
        let matrix_pair = super::super::require_matrix_pair(context)?;
        let retain_association = |association: &crate::model::MirroredCorrespondence| {
            selected_matrix_association(association, &matrix_pair.source_id, &matrix_pair.mirror_id)
                && (association.kind != MirroredItemKind::Layout
                    || !retired_layouts.iter().any(|retired| {
                        association.source_id == retired.source_id
                            && association.mirror_id == retired.mirror_id
                    }))
        };
        let retain_exception = |exception: &crate::model::MirroredException| {
            selected_matrix_exception(exception, &matrix_pair.mirror_id)
                && (exception.kind != MirroredItemKind::Layout
                    || !retired_layouts
                        .iter()
                        .any(|retired| exception.mirror_id == retired.mirror_id))
        };
        // Membership validation needs the narrowed pair rows. Borrow the original document
        // when those rows already describe exactly the selected matrix.
        if halves.correspondences.iter().all(retain_association)
            && halves.exceptions.iter().all(retain_exception)
        {
            return Ok(Cow::Borrowed(document));
        }
        let mut filtered = document.clone();
        let halves = filtered.hardware.as_mut().unwrap().pairs[index]
            .halves
            .as_mut()
            .unwrap();
        halves.correspondences.retain(retain_association);
        halves.exceptions.retain(retain_exception);
        Ok(Cow::Owned(filtered))
    }

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
        .collect();
    let filtered_before = selected_matrix_document(valid_before, selected, &retired_layouts)?;
    let filtered_post_source =
        selected_matrix_document(source_operation, selected, &retired_layouts)?;
    let filtered_context = match &filtered_before {
        Cow::Borrowed(_) => Cow::Borrowed(selected),
        Cow::Owned(document) => Cow::Owned(lifecycle_context(
            document,
            &selected.pair_id,
            selected.matrix_pair.clone(),
        )?),
    };
    let before = filtered_before.as_ref();
    let post_source = filtered_post_source.as_ref();
    let context = filtered_context.as_ref();

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
        axis: Option<RemovedAxis>,
        post_source_ids: &BTreeSet<String>,
    ) -> Option<String> {
        let (row, column, child) =
            super::super::matrix_identity_slot(id, matrix_pair.source_id.as_str())?;
        let (row, column) = match axis {
            Some(axis) => axis.remap(row, column)?,
            None => (row, column),
        };
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

    let axis = RemovedAxis::for_operation(operation, &matrix_pair.source_id);
    let source_identity_map: BTreeMap<_, _> = source_before
        .part_ids
        .iter()
        .map(|id| {
            (
                id.clone(),
                mapped_source_id(id, matrix_pair, axis, &source_after_ids),
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
            .filter_map(|(id, mapped)| (mapped.as_ref() != Some(id)).then_some(id.clone())),
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
    let mut required_owners: BTreeSet<_> = before
        .parts
        .iter()
        .chain(post_source.parts.iter())
        .map(|part| part.id.clone())
        .collect();
    required_owners.extend(context.owners.keys().cloned());
    required_owners.extend(retained_mirror_ids.iter().cloned());
    for addition in &planned_additions {
        required_owners.insert(addition.source_id.clone());
        required_owners.insert(addition.mirror_id.clone());
    }
    let mut final_owners = BTreeMap::new();
    for id in required_owners {
        let owner = if retained_mirror_ids.contains(&id) {
            FinalOwner::Board(context.mirror_board_id.clone())
        } else if mirror_identity_map.contains_key(&id)
            || (source_identity_map.contains_key(&id) && !source_after_ids.contains(&id))
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

    let mut output = prepare_membership_projection(
        MembershipTransitionInput {
            dispatch,
            valid_before: before,
            post_source,
            context,
            source_identity_map: &source_identity_map,
            mirror_identity_map: &mirror_identity_map,
            planned_additions: &planned_additions,
            final_owners: &final_owners,
            explicit_removal_closure: &explicit_removal_closure,
        },
        axis,
    )?;
    // Report against the selected matrix projection before restoring the other associations.
    output.changed_document_ids = changed_document_ids(post_source, &output.draft, &[])?;

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

    let board_part_remains = |id: &str| {
        post_source.parts.iter().any(|part| part.id == id)
            || post_source
                .boards
                .iter()
                .any(|board| board.part_ids.iter().any(|part| part == id))
    };
    let is_matrix = |association: &&super::super::TransitionAssociation| {
        association.kind == MirroredItemKind::Matrix
            && context
                .matrix_pair
                .as_ref()
                .map_or(include_unselected_matrices, |pair| {
                    association.source_id == pair.source_id
                        && association.mirror_id == pair.mirror_id
                })
    };
    let mut retired = Vec::new();
    let associations = &context.associations;
    let rows = associations
        .iter()
        .filter(is_matrix)
        .chain(
            associations
                .iter()
                .filter(|a| a.kind == MirroredItemKind::Part),
        )
        .chain(associations.iter().filter(|a| {
            matches!(
                a.kind,
                MirroredItemKind::EmbeddedCircuit
                    | MirroredItemKind::Net
                    | MirroredItemKind::Layout
            )
        }));
    for association in rows {
        let id = &association.source_id;
        let present = match association.kind {
            MirroredItemKind::Matrix => post_source.matrices.iter().any(|row| row.id == *id),
            MirroredItemKind::Part => post_source.parts.iter().any(|row| row.id == *id),
            MirroredItemKind::EmbeddedCircuit => post_source
                .embedded_circuits
                .iter()
                .any(|row| row.id == *id),
            MirroredItemKind::Net => post_source.nets.iter().any(|row| row.id == *id),
            MirroredItemKind::Layout => post_source.layouts.iter().any(|row| row.id == *id),
            _ => unreachable!(),
        };
        if present {
            continue;
        }
        if context.pending_pair_transitions.contains_key(id) {
            return Err("A selected lifecycle source was removed before its transition".into());
        }
        let stale = match association.kind {
            MirroredItemKind::Matrix => before
                .matrices
                .iter()
                .filter(|row| row.id == *id)
                .flat_map(|row| &row.part_ids)
                .any(|member| board_part_remains(member)),
            MirroredItemKind::EmbeddedCircuit => before
                .embedded_circuits
                .iter()
                .filter(|row| row.id == *id)
                .flat_map(|row| &row.part_ids)
                .any(|member| board_part_remains(member)),
            MirroredItemKind::Part => {
                board_part_remains(id)
                    || post_source
                        .matrices
                        .iter()
                        .any(|row| row.part_ids.contains(id))
                    || post_source
                        .embedded_circuits
                        .iter()
                        .any(|row| row.part_ids.contains(id))
            }
            MirroredItemKind::Net => post_source
                .boards
                .iter()
                .any(|board| board.net_ids.contains(id)),
            _ => false,
        };
        if stale {
            return Err(format!(
                "Source {:?} removal for {id:?} has an incomplete ownership closure",
                association.kind
            ));
        }
        retired.push(association.clone());
    }
    Ok(retired)
}
