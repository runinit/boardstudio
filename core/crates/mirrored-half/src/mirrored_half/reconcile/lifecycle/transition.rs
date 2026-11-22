//! Lifecycle transition preparation, exception contexts and membership deltas.

use super::super::{MembershipContext, TransitionState};
use super::{
    capture_omitted_copper, classify_source_retirements, copy_part_payload, durable_state,
    effective, restore_rejoined_nets, restore_saved_copper, validate_lifecycle_context,
};
use crate::model::MirroredExceptionState;
use std::collections::BTreeSet;

pub(super) fn prepare_lifecycle_projection(
    before: &crate::model::ProjectDoc,
    post_source: &crate::model::ProjectDoc,
    context: &MembershipContext,
    dispatch: crate::mirrored_half::EditDispatch,
) -> Result<super::super::MembershipTransitionOutput, String> {
    use super::super::{FinalOwner, MembershipContextDelta, MembershipTransitionOutput};
    use crate::model::MirroredItemKind;
    use std::collections::BTreeMap;

    let baseline = super::super::selected_pair(before, &context.pair_id, "lifecycle")?;
    if baseline.halves.as_ref() != Some(&context.pair_right) || baseline.mapping != context.mapping
    {
        return Err("The lifecycle pair baseline is stale or ambiguous".into());
    }
    if baseline != super::super::selected_pair(post_source, &context.pair_id, "lifecycle")? {
        return Err("The source operation changed the selected lifecycle pair".into());
    }
    validate_lifecycle_context(before, context)?;

    let mut draft = post_source.clone();
    // Omit takes the mirror copper that belonged only to the omitted items before any part goes.
    let removed_copper = capture_omitted_copper(&mut draft, post_source, context)?;
    let matrix_members: BTreeSet<_> = before
        .matrices
        .iter()
        .filter(|matrix| {
            context
                .matrix_pair
                .as_ref()
                .is_none_or(|pair| matrix.id == pair.source_id || matrix.id == pair.mirror_id)
        })
        .flat_map(|matrix| matrix.part_ids.iter().cloned())
        .collect();
    let mut removed = BTreeSet::new();
    let mut retired = classify_source_retirements(before, post_source, context, false)?;
    for module in context.associations.iter().filter(|row| {
        row.kind == MirroredItemKind::Module
            && !post_source
                .modules
                .iter()
                .any(|module| module.id == row.source_id)
    }) {
        if module.state == TransitionState::Linked {
            crate::modules::remove(&mut draft, &module.mirror_id)?;
        }
        retired.push(module.clone());
    }

    for owner in context.associations.iter().filter(|a| {
        a.kind == MirroredItemKind::EmbeddedCircuit
            && a.state == TransitionState::Linked
            && context.pending_rejoins.contains_key(&a.source_id)
    }) {
        if !draft
            .embedded_circuits
            .iter()
            .any(|c| c.id == owner.mirror_id)
        {
            let source = post_source
                .embedded_circuits
                .iter()
                .find(|c| c.id == owner.source_id)
                .ok_or("The exact Rejoin source circuit is unavailable")?;
            // Empty ownership shells are filled by the existing hosted projector below.
            draft.embedded_circuits.push(crate::model::EmbeddedCircuit {
                id: owner.mirror_id.clone(),
                definition_id: source.definition_id.clone(),
                host_board_id: context.mirror_board_id.clone(),
                part_ids: Vec::new(),
                net_ids: Vec::new(),
                ports: BTreeMap::new(),
            });
        }
    }
    for association in context.associations.iter().filter(|a| {
        a.kind == MirroredItemKind::Net && context.pending_rejoins.contains_key(&a.source_id)
    }) {
        if !draft.nets.iter().any(|net| net.id == association.mirror_id) {
            if !post_source
                .nets
                .iter()
                .any(|net| net.id == association.source_id)
            {
                return Err("The exact Rejoin source net is unavailable".into());
            }
            draft.nets.push(crate::model::Net {
                id: association.mirror_id.clone(),
                name: String::new(),
                pins: Vec::new(),
            });
            let board = draft
                .boards
                .iter_mut()
                .find(|board| board.id == context.mirror_board_id)
                .ok_or("The exact Rejoin mirror board is unavailable")?;
            if board.net_ids.contains(&association.mirror_id) {
                return Err("An absent Rejoin net retains a board registration".into());
            }
            board.net_ids.push(association.mirror_id.clone());
        }
    }
    let matrix_owner = context.matrix_pair.as_ref().and_then(|matrix_pair| {
        context.associations.iter().find(|a| {
            a.kind == MirroredItemKind::Matrix
                && a.source_id == matrix_pair.source_id
                && a.mirror_id == matrix_pair.mirror_id
        })
    });
    let source_matrix_removed = context.matrix_pair.as_ref().is_some_and(|matrix_pair| {
        !post_source
            .matrices
            .iter()
            .any(|matrix| matrix.id == matrix_pair.source_id)
    });
    if let (Some(_owner), Some(matrix_pair)) = (
        matrix_owner.filter(|owner| source_matrix_removed && owner.state != TransitionState::Local),
        context.matrix_pair.as_ref(),
    ) && let Some(matrix) = draft
        .matrices
        .iter_mut()
        .find(|m| m.id == matrix_pair.mirror_id)
    {
        let retained: BTreeSet<_> = context
            .associations
            .iter()
            .filter(|a| a.kind == MirroredItemKind::Part && a.state != TransitionState::Omitted)
            .map(|a| a.mirror_id.as_str())
            .collect();
        matrix.part_ids.retain(|id| !retained.contains(id.as_str()));
        (dispatch.apply_unscripted)(
            &mut draft,
            &crate::model::EditOperation::RemoveMatrix {
                id: matrix_pair.mirror_id.clone(),
            },
        )?;
    }
    if let Some(matrix_pair) = context.matrix_pair.as_ref()
        && matrix_owner.is_some_and(|owner| owner.state == TransitionState::Linked)
        && !source_matrix_removed
    {
        super::super::validate_no_native_legacy_overlap(post_source, context)?;
        super::super::validate_net_pad_ids(post_source)?;
        super::super::validate_constraints(post_source)?;
        super::super::validate_association_settings(context)?;

        for association in context
            .associations
            .iter()
            .filter(|a| a.kind == MirroredItemKind::Part && matrix_members.contains(&a.source_id))
        {
            super::super::transition_input::validate_slot_association(matrix_pair, association)?;
            super::super::transition_input::validate_linked_placement(
                post_source,
                context,
                association,
            )?;
        }
    }
    let additions = if let Some(owner) = matrix_owner
        && context
            .pending_pair_transitions
            .keys()
            .any(|source| source == &owner.source_id || matrix_members.contains(source))
    {
        // A sparse exception authorizes a change to these slots only after the
        // real source operation has left their established memberships intact.
        if [&owner.source_id, &owner.mirror_id]
            .into_iter()
            .any(|id| super::super::matrix_membership_changed(before, post_source, id))
        {
            return Err(
                "The pending exception's Matrix membership changed outside its selection".into(),
            );
        }
        if post_source != before {
            validate_lifecycle_context(post_source, context)?;
        }
        if owner.state == TransitionState::Linked {
            super::super::source_members(post_source, &owner.source_id, context, false)?;
            if before
                .matrices
                .iter()
                .any(|matrix| matrix.id == owner.mirror_id)
            {
                super::super::source_members(before, &owner.mirror_id, context, true)?;
            }
        }
        super::super::matrix_membership::materialize_selection(&mut draft, before, context, owner)?
    } else {
        Vec::new()
    };
    for association in context
        .associations
        .iter()
        .filter(|a| a.kind == MirroredItemKind::Part)
    {
        let mut sources = post_source
            .parts
            .iter()
            .filter(|part| part.id == association.source_id);
        let Some(source) = sources.next() else {
            if association.state != TransitionState::Local {
                removed.insert(association.mirror_id.clone());
            }
            continue;
        };
        if association.state == TransitionState::Omitted {
            // Slot materialization cannot remove a previously detached companion.
            removed.insert(association.mirror_id.clone());
            continue;
        }
        let matrix_owned = matrix_members.contains(&association.source_id)
            || matrix_members.contains(&association.mirror_id);
        if (context.matrix_pair.is_none()
            || matrix_owner.is_some_and(|owner| owner.state == TransitionState::Linked))
            && matrix_owned
        {
            continue;
        }
        if context.pending_rejoins.contains_key(&association.source_id) {
            if sources.next().is_some() {
                return Err("The exact Rejoin source Part is ambiguous".into());
            }
            let project = |definition: &str, reference: &str| {
                crate::mirrored_half::derive::part(
                    source,
                    &association.mirror_id,
                    definition,
                    reference,
                    context.mapping,
                    crate::mirrored_half::derive::is_controller(
                        &post_source.definitions,
                        &source.definition_id,
                    ),
                )
            };
            if let Some(target) = draft
                .parts
                .iter_mut()
                .find(|part| part.id == association.mirror_id)
            {
                *target = project(&target.definition_id, &target.reference);
            } else {
                if context.pending_rejoins.get(&association.source_id)
                    != Some(&TransitionState::Omitted)
                {
                    return Err("The Local Rejoin target Part is unavailable".into());
                }
                let mut references = draft
                    .parts
                    .iter()
                    .map(|part| part.reference.clone())
                    .collect();
                let reference = super::super::super::free_reference(
                    &format!("{}-{}", context.mirror_board_id, source.reference),
                    &mut references,
                );
                draft.parts.push(project(&source.definition_id, &reference));
                let board = draft
                    .boards
                    .iter_mut()
                    .find(|board| board.id == context.mirror_board_id)
                    .ok_or("The exact Rejoin mirror board is unavailable")?;
                if board.part_ids.contains(&association.mirror_id) {
                    return Err("The absent Rejoin Part retains a board registration".into());
                }
                board.part_ids.push(association.mirror_id.clone());
            }
        }
    }
    // Removing the members first would bypass the circuit deletion kernel's lock check.
    for circuit in retired.iter().filter(|row| {
        row.kind == MirroredItemKind::EmbeddedCircuit && row.state == TransitionState::Linked
    }) {
        if let Some(target) = draft
            .embedded_circuits
            .iter()
            .find(|item| item.id == circuit.mirror_id)
            && let Some(locked) = draft.parts.iter().find(|part| {
                removed.contains(&part.id)
                    && target.part_ids.contains(&part.id)
                    && part.locked == Some(true)
            })
        {
            return Err(format!(
                "Cannot remove circuit {:?} with locked member {:?}",
                target.id, locked.id
            ));
        }
    }
    crate::matrix::deletion::remove_parts_by_id(&mut draft, &removed);
    for owner in context.associations.iter().filter(|a| {
        a.kind == MirroredItemKind::EmbeddedCircuit
            && (a.state == TransitionState::Omitted
                || (a.state != TransitionState::Local
                    && !post_source
                        .embedded_circuits
                        .iter()
                        .any(|c| c.id == a.source_id)))
    }) {
        if let Some(circuit) = draft
            .embedded_circuits
            .iter_mut()
            .find(|c| c.id == owner.mirror_id)
        {
            let retained: BTreeSet<_> = context
                .associations
                .iter()
                .filter(|a| {
                    a.parent_source_id.as_deref() == Some(owner.source_id.as_str())
                        && a.parent_mirror_id.as_deref() == Some(owner.mirror_id.as_str())
                        && a.state != TransitionState::Omitted
                })
                .map(|a| a.mirror_id.as_str())
                .collect();
            circuit
                .part_ids
                .retain(|id| !retained.contains(id.as_str()));
            circuit.net_ids.retain(|id| !retained.contains(id.as_str()));
            crate::modules::remove_circuit(&mut draft, &owner.mirror_id)?;
        }
        if post_source
            .embedded_circuits
            .iter()
            .any(|c| c.id == owner.source_id)
            && context.associations.iter().any(|a| {
                a.kind == MirroredItemKind::Net
                    && a.state == TransitionState::Omitted
                    && a.parent_source_id.as_deref() == Some(owner.source_id.as_str())
                    && draft.nets.iter().any(|net| net.id == a.mirror_id)
            })
        {
            return Err("Omitted circuit ownership leaves an owned net in use; preserve that net explicitly first".into());
        }
    }
    restore_rejoined_nets(&mut draft, post_source, context)?;
    for added in &additions {
        let source = post_source
            .parts
            .iter()
            .find(|p| p.id == added.source_id)
            .ok_or("A new Rejoin source Part is unavailable")?;
        let target = draft
            .parts
            .iter_mut()
            .find(|p| p.id == added.mirror_id)
            .ok_or("A new Rejoin target Part is unavailable")?;
        copy_part_payload(target, source);
    }
    let mut output = MembershipTransitionOutput {
        draft,
        context: context.clone(),
        context_delta: MembershipContextDelta::default(),
        changed_document_ids: Vec::new(),
    };
    output.context_delta.removed_copper = removed_copper;
    output
        .context
        .associations
        .extend(additions.iter().cloned());
    output.context_delta.added = additions;
    for association in &retired {
        output.context.associations.retain(|a| {
            !(a.kind == association.kind
                && a.source_id == association.source_id
                && a.mirror_id == association.mirror_id)
        });
        output
            .context
            .owners
            .insert(association.source_id.clone(), FinalOwner::Absent);
        if association.state != TransitionState::Local {
            output
                .context
                .owners
                .insert(association.mirror_id.clone(), FinalOwner::Absent);
        }
    }
    output.context_delta.retired = retired;
    let mut output = apply_membership_context_delta(before, post_source, context, output)?;

    // The selection stages the affected slots. Canonical reconciliation uses the
    // durable matrix link to project the rest of the pair.
    let mut projection =
        super::super::projection_for_pair(&output.context.pair_right, context.mapping);
    if let Some(pair) = &context.matrix_pair {
        // Reconciliation authenticates the caller's board IDs against the stored pair.
        projection
            .source_board_id
            .clone_from(&context.source_board_id);
        projection
            .mirror_board_id
            .clone_from(&context.mirror_board_id);
        projection.matrix = Some(super::super::MatrixLink {
            source_matrix_id: pair.source_id.clone(),
            mirror_matrix_id: pair.mirror_id.clone(),
        });
    }
    // Inherited state is effective projection policy, not an additional stored exception: the
    // engine sees a transient overlay for each inherited child, and the stored halves return below.
    let engine_halves = output
        .draft
        .hardware
        .as_mut()
        .unwrap()
        .pairs
        .iter_mut()
        .find(|p| p.id == context.pair_id)
        .unwrap()
        .halves
        .as_mut()
        .unwrap();
    for a in output
        .context
        .associations
        .iter()
        .filter(|a| a.state != TransitionState::Linked && !a.explicit_state)
    {
        if engine_halves
            .exceptions
            .iter()
            .any(|e| e.kind == a.kind && e.mirror_id == a.mirror_id)
        {
            continue;
        }
        engine_halves
            .exceptions
            .push(crate::model::MirroredException {
                kind: a.kind,
                mirror_id: a.mirror_id.clone(),
                companion_mode: None,
                companion_tweak: None,
                state: if a.state == TransitionState::Omitted {
                    MirroredExceptionState::Omitted
                } else {
                    MirroredExceptionState::Local
                },
                removed_copper: None,
            });
    }
    super::super::document::reconcile_canonical(&mut output.draft, &context.pair_id, &projection)?;
    output
        .draft
        .hardware
        .as_mut()
        .unwrap()
        .pairs
        .iter_mut()
        .find(|p| p.id == context.pair_id)
        .unwrap()
        .halves = Some(output.context.pair_right.clone());
    restore_saved_copper(&mut output, context)?;
    crate::mechanical::halves::validate_document(&output.draft)?;
    for part in &output.draft.parts {
        let mut boards = output
            .draft
            .boards
            .iter()
            .filter(|board| board.part_ids.contains(&part.id));
        let board = boards.next().ok_or("A lifecycle Part has no board owner")?;
        if boards.next().is_some() {
            return Err("A lifecycle Part has ambiguous board ownership".into());
        }
        output
            .context
            .owners
            .insert(part.id.clone(), FinalOwner::Board(board.id.clone()));
    }
    if let Some(pair) = &context.matrix_pair
        && matrix_owner.is_some_and(|owner| owner.state == TransitionState::Linked)
        && !source_matrix_removed
    {
        let sources =
            super::super::source_members(&output.draft, &pair.source_id, &output.context, false)?;
        let mirrors =
            super::super::source_members(&output.draft, &pair.mirror_id, &output.context, false)?;
        let slots = output.context.associations.iter().filter(|a| {
            a.kind == MirroredItemKind::Part
                && (sources.contains(&a.source_id) || mirrors.contains(&a.mirror_id))
        });
        super::super::transition_checks::validate_emitted_slots(
            &output.draft,
            &output.context,
            &sources,
            &mirrors,
            slots,
        )?;
    }
    Ok(output)
}

/// Applies one already-validated lifecycle state to a source/mirror association.
pub(super) fn transition_exception_context(
    context: &MembershipContext,
    kind: crate::model::MirroredItemKind,
    source_id: &str,
    mirror_id: &str,
    state: TransitionState,
) -> Result<MembershipContext, String> {
    let mut next = context.clone();
    let mut selected = next
        .associations
        .iter()
        .enumerate()
        .filter(|(_, association)| {
            association.kind == kind
                && association.source_id == source_id
                && association.mirror_id == mirror_id
        });
    let (selected_index, selected_association) = selected
        .next()
        .map(|(index, association)| (index, association.clone()))
        .ok_or_else(|| "The lifecycle transition has no exact private association".to_owned())?;
    if selected.next().is_some() {
        return Err("The lifecycle transition association is ambiguous".into());
    }

    let exact_correspondences: Vec<_> = next
        .pair_right
        .correspondences
        .iter()
        .filter(|record| {
            record.kind == kind
                && (record.source_id == source_id
                    || record.mirror_id == mirror_id
                    || record.source_id == mirror_id
                    || record.mirror_id == source_id)
        })
        .collect();
    let (durable_state, _) = durable_state(&next, &selected_association)?;
    // Also exclude reversed endpoints on the selected correspondence.
    if exact_correspondences.len() != 1 {
        return Err("The lifecycle transition disagrees with exact pair provenance".into());
    }
    if state != durable_state {
        next.pending_pair_transitions
            .insert(source_id.to_owned(), durable_state);
    }
    next.associations[selected_index].explicit_state = state != TransitionState::Linked;
    next.associations[selected_index].state = state;
    let inherited_state = effective(&next, &next.associations[selected_index], true, 0)?;

    let mut affected = BTreeSet::from([selected_index]);
    loop {
        let mut added = false;
        for (index, association) in next.associations.iter().enumerate() {
            if affected.contains(&index) || association.explicit_state {
                continue;
            }
            let parent_selected =
                next.associations
                    .iter()
                    .enumerate()
                    .any(|(parent_index, parent)| {
                        affected.contains(&parent_index)
                            && association.parent_source_id.as_deref()
                                == Some(parent.source_id.as_str())
                            && association.parent_mirror_id.as_deref()
                                == Some(parent.mirror_id.as_str())
                    });
            if parent_selected {
                affected.insert(index);
                added = true;
            }
        }
        if !added {
            break;
        }
    }

    for index in affected {
        let association = &mut next.associations[index];
        association.state = inherited_state;
        association.explicit_state = index == selected_index && state != TransitionState::Linked;
        if association.kind == crate::model::MirroredItemKind::Part {
            let owner = match inherited_state {
                TransitionState::Linked | TransitionState::Local => {
                    super::super::FinalOwner::Board(next.mirror_board_id.clone())
                }
                TransitionState::Omitted => super::super::FinalOwner::Absent,
            };
            next.owners.insert(association.mirror_id.clone(), owner);
        }
    }
    next.pending_rejoins.clear();
    for association in &next.associations {
        let old = effective(&next, association, false, 0)?;
        if association.state == TransitionState::Linked && old != TransitionState::Linked {
            next.pending_rejoins
                .insert(association.source_id.clone(), old);
        }
    }

    Ok(next)
}

pub(in crate::mirrored_half::reconcile) fn apply_membership_context_delta(
    before: &crate::model::ProjectDoc,
    post_source: &crate::model::ProjectDoc,
    context: &MembershipContext,
    mut output: super::super::MembershipTransitionOutput,
) -> Result<super::super::MembershipTransitionOutput, String> {
    use crate::model::{MirroredCorrespondence, MirroredExceptionState, MirroredItemKind};

    if output.context_delta.added.is_empty()
        && output.context_delta.retired.is_empty()
        && output.context_delta.relocated.is_empty()
        && context.pending_rejoins.is_empty()
        && context.pending_pair_transitions.is_empty()
    {
        return Ok(output);
    }

    let hardware = output
        .draft
        .hardware
        .as_mut()
        .ok_or_else(|| "The lifecycle candidate has no hardware configuration".to_owned())?;
    let mut pairs = hardware
        .pairs
        .iter_mut()
        .filter(|pair| pair.id == context.pair_id);
    let pair = pairs
        .next()
        .ok_or_else(|| "The lifecycle candidate has no selected pair".to_owned())?;
    if pairs.next().is_some() {
        return Err("The lifecycle candidate pair is ambiguous".into());
    }
    let baseline_pair =
        super::super::selected_pair(before, &context.pair_id, "lifecycle baseline")?;
    if pair != baseline_pair {
        return Err("The reconciliation kernel changed persisted pair bytes".into());
    }
    let halves = pair
        .halves
        .as_mut()
        .ok_or_else(|| "The lifecycle candidate pair has no halves record".to_owned())?;

    // Read the relocation from each original row so a compacted slot cannot consume
    // another survivor's identity. Unselected rows retain their order.
    let mut changes = std::collections::BTreeMap::new();
    for old in &output.context_delta.retired {
        if changes
            .insert(
                (
                    old.kind as u8,
                    old.source_id.as_str(),
                    old.mirror_id.as_str(),
                ),
                None,
            )
            .is_some()
        {
            return Err(format!("Ambiguous durable association {:?}", old.source_id));
        }
    }
    for (old_id, new_id) in &output.context_delta.relocated {
        let old = context
            .associations
            .iter()
            .find(|a| a.kind == MirroredItemKind::Part && a.source_id == *old_id)
            .ok_or_else(|| format!("Missing relocated source {old_id:?}"))?;
        let new = output
            .context
            .associations
            .iter()
            .find(|a| a.kind == MirroredItemKind::Part && a.source_id == *new_id)
            .ok_or_else(|| format!("Missing relocation target {new_id:?}"))?;
        if changes
            .insert(
                (
                    old.kind as u8,
                    old.source_id.as_str(),
                    old.mirror_id.as_str(),
                ),
                Some(new),
            )
            .is_some()
        {
            return Err(format!("Ambiguous durable association {:?}", old.source_id));
        }
    }
    let record_from = |a: &super::super::TransitionAssociation| MirroredCorrespondence {
        kind: a.kind,
        source_id: a.source_id.clone(),
        mirror_id: a.mirror_id.clone(),
        companion_mode: a.raw_mode_override,
        companion_tweak: a.raw_tweak,
    };
    let mut overlays = std::collections::BTreeMap::new();
    let mut applied = BTreeSet::new();
    let mut next_halves = halves.clone();
    next_halves.correspondences.clear();
    for row in &halves.correspondences {
        let key = (
            row.kind as u8,
            row.source_id.as_str(),
            row.mirror_id.as_str(),
        );
        match changes.get(&key).copied() {
            Some(new) => {
                if !applied.insert(key) {
                    return Err(format!("Ambiguous durable association {:?}", row.source_id));
                }
                overlays.insert((row.kind as u8, row.mirror_id.as_str()), new);
                next_halves.correspondences.extend(new.map(record_from));
            }
            None => next_halves.correspondences.push(row.clone()),
        }
    }
    if let Some((_, source, _)) = changes.keys().find(|key| !applied.contains(*key)) {
        return Err(format!("Missing durable association {source:?}"));
    }
    next_halves.exceptions.retain_mut(|overlay| {
        match overlays.get(&(overlay.kind as u8, overlay.mirror_id.as_str())) {
            Some(Some(new)) => overlay.mirror_id.clone_from(&new.mirror_id),
            Some(None) => return false,
            None => {}
        }
        true
    });
    for added in &output.context_delta.added {
        if added.kind != MirroredItemKind::Part
            || added.state != super::super::TransitionState::Linked
        {
            return Err(
                "The transition planned an unsupported durable association addition".into(),
            );
        }
        if next_halves.correspondences.iter().any(|record| {
            record.kind == added.kind
                && (record.source_id == added.source_id || record.mirror_id == added.mirror_id)
        }) || next_halves
            .exceptions
            .iter()
            .any(|record| record.kind == added.kind && record.mirror_id == added.mirror_id)
        {
            return Err(format!(
                "Added Part association {:?} collides with durable pair ownership",
                added.source_id
            ));
        }
        next_halves.correspondences.push(record_from(added));
    }

    // Apply exact association changes only after the projection kernel succeeds. `next_halves`
    // starts as the authentic durable pair, so unrelated rows and their order are preserved.
    // Identity never moves: the correspondence stays in every state and only its overlay changes.
    for (source_id, prior_state) in &context.pending_pair_transitions {
        let mut associations = output
            .context
            .associations
            .iter()
            .filter(|association| association.source_id == *source_id);
        let association = associations.next().ok_or_else(|| {
            format!("The pending pair transition {source_id:?} has no association")
        })?;
        if associations.next().is_some() {
            return Err(format!(
                "The pending pair transition {source_id:?} is ambiguous"
            ));
        }
        let correspondence_matches = next_halves
            .correspondences
            .iter()
            .filter(|record| {
                record.kind == association.kind
                    && (record.source_id == *source_id || record.mirror_id == association.mirror_id)
            })
            .collect::<Vec<_>>();
        let exact = correspondence_matches.len() == 1
            && correspondence_matches[0].source_id == association.source_id
            && correspondence_matches[0].mirror_id == association.mirror_id
            && correspondence_matches[0].companion_mode == association.raw_mode_override
            && correspondence_matches[0].companion_tweak == association.raw_tweak;
        if !exact {
            return Err(format!(
                "The pending transition {source_id:?} disagrees with its original correspondence"
            ));
        }
        let overlay_indexes: Vec<_> = next_halves
            .exceptions
            .iter()
            .enumerate()
            .filter(|(_, record)| {
                record.kind == association.kind && record.mirror_id == association.mirror_id
            })
            .map(|(index, _)| index)
            .collect();
        match (prior_state, overlay_indexes.as_slice()) {
            (TransitionState::Linked, []) => {}
            (TransitionState::Local | TransitionState::Omitted, [index]) => {
                let old = &next_halves.exceptions[*index];
                let expected_state = match prior_state {
                    TransitionState::Local => MirroredExceptionState::Local,
                    TransitionState::Omitted => MirroredExceptionState::Omitted,
                    TransitionState::Linked => unreachable!(),
                };
                if old.state != expected_state
                    || old.companion_mode.is_some()
                    || old.companion_tweak.is_some()
                {
                    return Err(format!(
                        "The pending transition {source_id:?} disagrees with its original exception"
                    ));
                }
                next_halves.exceptions.remove(*index);
            }
            _ => {
                return Err(format!(
                    "The pending transition {source_id:?} lacks exclusive durable provenance"
                ));
            }
        }
        if association.explicit_state && association.state != TransitionState::Linked {
            next_halves
                .exceptions
                .push(crate::model::MirroredException {
                    kind: association.kind,
                    mirror_id: association.mirror_id.clone(),
                    state: match association.state {
                        TransitionState::Local => MirroredExceptionState::Local,
                        TransitionState::Omitted => MirroredExceptionState::Omitted,
                        TransitionState::Linked => unreachable!(),
                    },
                    companion_mode: None,
                    companion_tweak: None,
                    removed_copper: output
                        .context_delta
                        .removed_copper
                        .get(&association.mirror_id)
                        .cloned(),
                });
        }
    }

    // Inherited child Rejoins keep their existing durable correspondence, but their geometry
    // and payload still derive from the current source within this same candidate.
    for source_id in context.pending_rejoins.keys().filter(|source_id| {
        output
            .context
            .associations
            .iter()
            .any(|a| a.kind == MirroredItemKind::Part && a.source_id == **source_id)
    }) {
        let mut associations = output.context.associations.iter().filter(|association| {
            association.kind == MirroredItemKind::Part
                && association.source_id == *source_id
                && association.state == TransitionState::Linked
        });
        let association = associations.next().ok_or_else(|| {
            format!("The pending Rejoin {source_id:?} has no linked Part association")
        })?;
        if associations.next().is_some() {
            return Err(format!("The pending Rejoin {source_id:?} is ambiguous"));
        }
        let mut sources = post_source
            .parts
            .iter()
            .filter(|part| part.id == association.source_id);
        let source = sources.next().ok_or_else(|| {
            format!(
                "The Rejoined source Part {:?} is unavailable",
                association.source_id
            )
        })?;
        if sources.next().is_some() {
            return Err(format!(
                "The Rejoined source Part {:?} is ambiguous",
                association.source_id
            ));
        }
        let mut targets = output
            .draft
            .parts
            .iter_mut()
            .filter(|part| part.id == association.mirror_id);
        let target = targets.next().ok_or_else(|| {
            format!(
                "The Rejoined target Part {:?} is unavailable",
                association.mirror_id
            )
        })?;
        if targets.next().is_some() {
            return Err(format!(
                "The Rejoined target Part {:?} is ambiguous",
                association.mirror_id
            ));
        }
        copy_part_payload(target, source);
    }

    *halves = next_halves.clone();
    crate::mechanical::halves::validate_document(&output.draft)
        .map_err(|error| format!("The lifecycle transaction candidate is invalid: {error}"))?;
    output.context.pair_right = next_halves;
    output.context.pending_rejoins.clear();
    output.context.pending_pair_transitions.clear();
    Ok(output)
}
