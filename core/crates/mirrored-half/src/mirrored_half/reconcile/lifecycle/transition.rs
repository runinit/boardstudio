//! Lifecycle transition preparation, exception contexts and membership deltas.

use super::super::{MembershipContext, TransitionState};
use super::{
    adopt_rejoined_anchor, capture_omitted_copper, classify_source_retirements, copy_part_payload,
    durable_state, effective, project_lifecycle_matrix, restore_rejoined_nets,
    restore_saved_copper, validate_lifecycle_context,
};
use crate::model::MirroredExceptionState;
use std::collections::BTreeSet;

#[doc(hidden)]
pub fn prepare_lifecycle_transition(
    before: &crate::model::ProjectDoc,
    post_source: &crate::model::ProjectDoc,
    context: &MembershipContext,
    dispatch: crate::mirrored_half::EditDispatch,
) -> Result<super::super::MembershipTransitionOutput, String> {
    let mut output = prepare_lifecycle_projection(before, post_source, context, dispatch)?;
    (output.changed_document_ids, output.changed_field_paths) =
        super::super::changed_document_fields(post_source, &output.draft)?;
    Ok(output)
}

pub(super) fn prepare_lifecycle_projection(
    before: &crate::model::ProjectDoc,
    post_source: &crate::model::ProjectDoc,
    context: &MembershipContext,
    dispatch: crate::mirrored_half::EditDispatch,
) -> Result<super::super::MembershipTransitionOutput, String> {
    use super::super::{
        FinalOwner, MembershipContextDelta, MembershipTransitionInput, MembershipTransitionOutput,
    };
    use crate::model::{MirroredItemKind, Part};
    use std::collections::BTreeMap;

    fn selected_pair<'a>(
        document: &'a crate::model::ProjectDoc,
        context: &MembershipContext,
    ) -> Result<&'a crate::model::MechanicalMirrorPair, String> {
        let mut pairs = document
            .hardware
            .as_ref()
            .ok_or("Lifecycle hardware is unavailable")?
            .pairs
            .iter()
            .filter(|pair| pair.id == context.pair_id);
        let pair = pairs
            .next()
            .ok_or("The exact lifecycle pair is unavailable")?;
        if pairs.next().is_some()
            || pair.halves.as_ref() != Some(&context.pair_right)
            || pair.mapping != context.mapping
        {
            return Err("The lifecycle pair baseline is stale or ambiguous".into());
        }
        Ok(pair)
    }

    crate::mechanical::halves::validate_document(before)?;
    if selected_pair(before, context)? != selected_pair(post_source, context)? {
        return Err("The source operation changed the selected lifecycle pair".into());
    }
    match (
        context.pending_pair_transitions.is_empty(),
        &context.pending_pair_baseline,
    ) {
        (true, None) => {}
        (false, Some(baseline)) if baseline == &context.pair_right => {}
        _ => return Err("The private pending pair baseline is stale or incomplete".into()),
    }
    validate_lifecycle_context(before, context)?;

    let mut draft = post_source.clone();
    // Omit takes the mirror copper that belonged only to the omitted items before any part goes.
    let removed_copper = capture_omitted_copper(&mut draft, post_source, context)?;
    let matrix_members: BTreeSet<_> = context
        .matrix_pair
        .as_ref()
        .map(|matrix_pair| {
            before
                .matrices
                .iter()
                .filter(|matrix| {
                    matrix.id == matrix_pair.source_id || matrix.id == matrix_pair.mirror_id
                })
                .flat_map(|matrix| matrix.part_ids.iter().cloned())
                .collect()
        })
        .or_else(|| {
            context.matrix_pair.is_none().then(|| {
                before
                    .matrices
                    .iter()
                    .flat_map(|matrix| matrix.part_ids.iter().cloned())
                    .collect()
            })
        })
        .unwrap_or_default();
    let mut removed = BTreeSet::new();
    let retired = classify_source_retirements(before, post_source, context, false)?;
    let mut additions = Vec::new();
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
    // A matrix-level Omitted mirror with a live source goes live-yet-absent
    // (spec §5): the mirror record is dropped while its correspondences stay
    // live. The layout shell keeps naming the absent matrix so the pair's
    // layout correspondence keeps following the reconciled matrix pair;
    // Rejoin recreates the matrix from source. A source removal with a
    // non-Local mirror still retires the whole pair side through the full
    // removal cascade below.
    if let (Some(_owner), Some(matrix_pair)) = (
        matrix_owner
            .filter(|owner| owner.state == TransitionState::Omitted && !source_matrix_removed),
        context.matrix_pair.as_ref(),
    ) {
        draft.matrices.retain(|m| m.id != matrix_pair.mirror_id);
    }
    if let (Some(_owner), Some(matrix_pair)) = (
        matrix_owner.filter(|owner| source_matrix_removed && owner.state != TransitionState::Local),
        context.matrix_pair.as_ref(),
    ) {
        if let Some(matrix) = draft
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
    } else if let (Some(matrix_owner), Some(matrix_pair)) = (
        matrix_owner.filter(|owner| context.pending_rejoins.contains_key(&owner.source_id)),
        context.matrix_pair.as_ref(),
    ) {
        let source = draft
            .matrices
            .iter()
            .find(|m| m.id == matrix_pair.source_id)
            .ok_or("The Rejoin source Matrix is unavailable")?
            .clone();
        let retained: Vec<_> = context
            .associations
            .iter()
            .filter(|a| a.kind == MirroredItemKind::Part && a.state == TransitionState::Local)
            .filter_map(|a| draft.parts.iter().find(|p| p.id == a.mirror_id).cloned())
            .collect();
        let mut seed = source.clone();
        seed.id = matrix_pair.mirror_id.clone();
        seed.board_id = Some(context.mirror_board_id.clone());
        seed.part_ids.clear();
        let record = crate::model::MirroredCorrespondence {
            kind: MirroredItemKind::Matrix,
            source_id: source.id.clone(),
            mirror_id: seed.id.clone(),
            companion_mode: matrix_owner.raw_mode_override,
            companion_tweak: matrix_owner.raw_tweak,
        };
        let mut projected = super::super::project_matrix(&source, &seed, &record, context.mapping)?;
        adopt_rejoined_anchor(&mut projected, &draft, context, matrix_pair)?;
        for cell in &mut projected.cells {
            let key = format!(
                "matrix/{}/r{}c{}",
                matrix_pair.source_id, cell.row, cell.column
            );
            if context.associations.iter().any(|a| {
                a.kind == MirroredItemKind::Part
                    && a.source_id == key
                    && a.state == TransitionState::Omitted
            }) {
                cell.enabled = false;
                cell.deleted = true;
                cell.assemblies.clear();
            } else {
                cell.assemblies.retain(|assembly| {
                    !context.associations.iter().any(|a| {
                        a.kind == MirroredItemKind::Part
                            && a.source_id == crate::matrix::companion_id(&key, &assembly.id)
                            && a.state == TransitionState::Omitted
                    })
                });
            }
        }
        seed.part_ids = retained.iter().map(|part| part.id.clone()).collect();
        if !draft.matrices.iter().any(|m| m.id == seed.id) {
            draft.matrices.push(seed);
        }
        crate::matrix::set_matrix(&mut draft, &projected)?;
        for local in retained {
            let target = draft
                .parts
                .iter_mut()
                .find(|p| p.id == local.id)
                .ok_or("An explicit Local child is outside the current Rejoin Matrix")?;
            *target = local;
        }
        // New cells introduced while an owner was Local acquire associations only on this
        // explicitly authorized owner Rejoin, after the normal setter assigns their targets.
        for id in &source.part_ids {
            if context
                .associations
                .iter()
                .any(|a| a.kind == MirroredItemKind::Part && a.source_id == *id)
            {
                continue;
            }
            let (row, column, suffix) =
                super::super::matrix_identity_slot(id, &matrix_pair.source_id)
                    .ok_or("A new owner Rejoin member is not canonical")?;
            let mirror_id = if suffix.is_empty() {
                format!("matrix/{}/r{row}c{column}", matrix_pair.mirror_id)
            } else {
                format!("matrix/{}/r{row}c{column}/{suffix}", matrix_pair.mirror_id)
            };
            if draft.parts.iter().filter(|p| p.id == mirror_id).count() != 1 {
                return Err("A new owner Rejoin member lacks its exact materialized target".into());
            }
            additions.push(super::super::TransitionAssociation {
                kind: MirroredItemKind::Part,
                source_id: id.clone(),
                mirror_id,
                parent_source_id: Some(if suffix.is_empty() {
                    matrix_pair.source_id.clone()
                } else {
                    format!("matrix/{}/r{row}c{column}", matrix_pair.source_id)
                }),
                parent_mirror_id: Some(if suffix.is_empty() {
                    matrix_pair.mirror_id.clone()
                } else {
                    format!("matrix/{}/r{row}c{column}", matrix_pair.mirror_id)
                }),
                state: TransitionState::Linked,
                explicit_state: false,
                raw_mode_override: None,
                raw_tweak: None,
                effective_mode: context.pair_right.companion_mode,
                effective_tweak: None,
            });
        }
    }
    for association in context
        .associations
        .iter()
        .filter(|a| a.kind == MirroredItemKind::Part)
    {
        if !post_source
            .parts
            .iter()
            .any(|part| part.id == association.source_id)
        {
            if association.state != TransitionState::Local {
                removed.insert(association.mirror_id.clone());
            }
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
        if association.state == TransitionState::Omitted {
            removed.insert(association.mirror_id.clone());
        } else if context.pending_rejoins.contains_key(&association.source_id) {
            let mut sources = post_source
                .parts
                .iter()
                .filter(|part| part.id == association.source_id);
            let source = sources
                .next()
                .ok_or("The exact Rejoin source Part is unavailable")?;
            if sources.next().is_some() {
                return Err("The exact Rejoin source Part is ambiguous".into());
            }
            let target = draft
                .parts
                .iter()
                .find(|part| part.id == association.mirror_id)
                .cloned();
            let seed = match target {
                Some(part) => part,
                None if context.pending_rejoins.get(&association.source_id)
                    == Some(&TransitionState::Omitted) =>
                {
                    let mut references = draft
                        .parts
                        .iter()
                        .map(|part| part.reference.clone())
                        .collect();
                    Part {
                        id: association.mirror_id.clone(),
                        definition_id: source.definition_id.clone(),
                        reference: super::super::super::free_reference(
                            &format!("{}-{}", context.mirror_board_id, source.reference),
                            &mut references,
                        ),
                        ..source.clone()
                    }
                }
                None => return Err("The Local Rejoin target Part is unavailable".into()),
            };
            let record = crate::model::MirroredCorrespondence {
                kind: association.kind,
                source_id: association.source_id.clone(),
                mirror_id: association.mirror_id.clone(),
                companion_mode: association.raw_mode_override,
                companion_tweak: association.raw_tweak,
            };
            let projected = super::super::project_part(
                source,
                &seed,
                &record,
                context.mapping,
                super::super::super::derive::is_controller(
                    &post_source.definitions,
                    &source.definition_id,
                ),
            )?;
            if let Some(target) = draft
                .parts
                .iter_mut()
                .find(|part| part.id == association.mirror_id)
            {
                *target = projected;
            } else {
                draft.parts.push(projected);
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
    draft = project_lifecycle_matrix(draft, context, &additions, dispatch)?;
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
        final_owners: context.owners.clone(),
        changed_document_ids: Vec::new(),
        changed_field_paths: Vec::new(),
        notices: Vec::new(),
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
    let empty_map = BTreeMap::new();
    let empty_set = BTreeSet::new();
    let mut output = apply_membership_context_delta(
        MembershipTransitionInput {
            dispatch,
            valid_before: before,
            post_source,
            context,
            baseline_kind: super::super::BaselineKind::MaterializedCurrentSchema,
            source_identity_map: &empty_map,
            mirror_identity_map: &empty_map,
            planned_additions: &[],
            final_owners: &context.owners,
            explicit_removal_closure: &empty_set,
        },
        output,
    )?;

    let projection = if context.matrix_pair.is_none() {
        super::super::projection_for_matrixless_pair(&output.context.pair_right, context.mapping)
    } else {
        let matrix_pair = context
            .matrix_pair
            .as_ref()
            .ok_or("The exact lifecycle Matrix correspondence is unavailable")?;
        let source = output
            .draft
            .matrices
            .iter()
            .find(|matrix| matrix.id == matrix_pair.source_id);
        let mirror = output
            .draft
            .matrices
            .iter()
            .find(|matrix| matrix.id == matrix_pair.mirror_id);
        let key_ids: BTreeSet<_> = source
            .iter()
            .flat_map(|m| {
                m.cells
                    .iter()
                    .map(|cell| format!("matrix/{}/r{}c{}", m.id, cell.row, cell.column))
            })
            .collect();
        super::super::Projection {
            matrix: Some(super::super::MatrixLink {
                source_matrix_id: matrix_pair.source_id.clone(),
                mirror_matrix_id: matrix_pair.mirror_id.clone(),
            }),
            preserve_matrix_geometry: true,
            source_board_id: context.source_board_id.clone(),
            mirror_board_id: context.mirror_board_id.clone(),
            mapping: context.mapping,
            parts: output
                .context
                .associations
                .iter()
                .filter(|a| {
                    a.kind == MirroredItemKind::Part
                        && key_ids.contains(&a.source_id)
                        && a.state == TransitionState::Linked
                })
                .map(|a| super::super::PartProjection {
                    source_id: a.source_id.clone(),
                    mirror_id: a.mirror_id.clone(),
                })
                .collect(),
            locally_protected_mirror_parts: output
                .context
                .associations
                .iter()
                .filter(|a| {
                    a.kind == MirroredItemKind::Part
                        && key_ids.contains(&a.source_id)
                        && a.state == TransitionState::Local
                })
                .map(|a| a.mirror_id.clone())
                .collect(),
            final_owners: source
                .iter()
                .flat_map(|m| m.part_ids.iter())
                .map(|id| (id.clone(), context.source_board_id.clone()))
                .chain(
                    mirror
                        .iter()
                        .flat_map(|matrix| matrix.part_ids.iter())
                        .map(|id| (id.clone(), context.mirror_board_id.clone())),
                )
                .collect(),
            independent_part_transitions: Vec::new(),
            hosted_transitions: Vec::new(),
            applied_independent_additions: Default::default(),
            applied_hosted_operations: Default::default(),
            applied_hosted_circuit_additions: Default::default(),
            hosted_removal_baselines: Default::default(),
            hosted_omission_baselines: Default::default(),
            outline_removal_baselines: Default::default(),
        }
    };
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
    super::super::document::reconcile_with_report(
        &mut output.draft,
        &context.pair_id,
        &projection,
        false,
    )?;
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
    output.final_owners = output.context.owners.clone();
    output.context_delta.owners = output.final_owners.clone();
    Ok(output)
}

/// A pending Omit may still have visible baseline geometry until the kernel removes it.
/// Follow exact private ownership edges to the selected durable record, never ID prefixes.
pub(in crate::mirrored_half::reconcile) fn pending_visible_omission(
    context: &MembershipContext,
    mirror_id: &str,
) -> bool {
    let mut matches = context.associations.iter().filter(|association| {
        association.kind == crate::model::MirroredItemKind::Part
            && association.mirror_id == mirror_id
            && association.state == TransitionState::Omitted
    });
    let Some(mut association) = matches.next() else {
        return false;
    };
    if matches.next().is_some() {
        return false;
    }
    for _ in 0..context.associations.len() {
        if let Some(prior) = context.pending_pair_transitions.get(&association.source_id) {
            if !association.explicit_state {
                return false;
            }
            return match prior {
                TransitionState::Linked => {
                    context
                        .pair_right
                        .correspondences
                        .iter()
                        .filter(|record| {
                            record.kind == association.kind
                                && record.source_id == association.source_id
                                && record.mirror_id == association.mirror_id
                        })
                        .count()
                        == 1
                        && !context.pair_right.exceptions.iter().any(|record| {
                            record.kind == association.kind
                                && record.mirror_id == association.mirror_id
                        })
                }
                TransitionState::Local => {
                    context
                        .pair_right
                        .exceptions
                        .iter()
                        .filter(|record| {
                            record.kind == association.kind
                                && record.mirror_id == association.mirror_id
                                && record.state == crate::model::MirroredExceptionState::Local
                        })
                        .count()
                        == 1
                        && context.pair_right.correspondences.iter().any(|record| {
                            record.kind == association.kind
                                && record.source_id == association.source_id
                                && record.mirror_id == association.mirror_id
                        })
                }
                TransitionState::Omitted => false,
            };
        }
        if association.explicit_state {
            return false;
        }
        let mut parents = context.associations.iter().filter(|parent| {
            association.parent_source_id.as_deref() == Some(parent.source_id.as_str())
                && association.parent_mirror_id.as_deref() == Some(parent.mirror_id.as_str())
                && parent.state == TransitionState::Omitted
        });
        let Some(parent) = parents.next() else {
            return false;
        };
        if parents.next().is_some() {
            return false;
        }
        association = parent;
    }
    false
}

/// Applies one already-validated lifecycle state to a source/mirror association.
#[doc(hidden)]
pub fn transition_exception_context(
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
    let exact_exceptions: Vec<_> = next
        .pair_right
        .exceptions
        .iter()
        .filter(|record| record.kind == kind && record.mirror_id == mirror_id)
        .collect();
    let (durable_state, _) = durable_state(&next, &selected_association)?;
    if next
        .pending_pair_transitions
        .get(source_id)
        .is_some_and(|prior| *prior != durable_state)
    {
        return Err("The lifecycle transition disagrees with its pending baseline".into());
    }
    // The correspondence is exact in every state; Local and Omitted add one overlay on it.
    let exact_correspondence = exact_correspondences.len() == 1
        && exact_correspondences[0].source_id == source_id
        && exact_correspondences[0].mirror_id == mirror_id;
    match durable_state {
        TransitionState::Linked if exact_correspondence && exact_exceptions.is_empty() => {}
        TransitionState::Local | TransitionState::Omitted
            if exact_correspondence
                && exact_exceptions.len() == 1
                && exact_exceptions[0].state
                    == match durable_state {
                        TransitionState::Local => crate::model::MirroredExceptionState::Local,
                        TransitionState::Omitted => crate::model::MirroredExceptionState::Omitted,
                        TransitionState::Linked => unreachable!(),
                    } => {}
        _ => return Err("The lifecycle transition disagrees with exact pair provenance".into()),
    }
    let original_state = durable_state;
    let returns_to_baseline = state == original_state;
    if returns_to_baseline {
        next.pending_pair_transitions.remove(source_id);
        if next.pending_pair_transitions.is_empty() {
            next.pending_pair_baseline = None;
        }
    } else {
        next.pending_pair_baseline
            .get_or_insert_with(|| context.pair_right.clone());
        next.pending_pair_transitions
            .insert(source_id.to_owned(), original_state);
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
    input: super::super::MembershipTransitionInput<'_>,
    mut output: super::super::MembershipTransitionOutput,
) -> Result<super::super::MembershipTransitionOutput, String> {
    use crate::model::{MirroredCorrespondence, MirroredExceptionState, MirroredItemKind};

    if output.context_delta.added.is_empty()
        && output.context_delta.retired.is_empty()
        && output.context_delta.relocated.is_empty()
        && input.context.pending_rejoins.is_empty()
        && input.context.pending_pair_transitions.is_empty()
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
        .filter(|pair| pair.id == input.context.pair_id);
    let pair = pairs
        .next()
        .ok_or_else(|| "The lifecycle candidate has no selected pair".to_owned())?;
    if pairs.next().is_some() {
        return Err("The lifecycle candidate pair is ambiguous".into());
    }
    let mut baseline_pairs = input
        .valid_before
        .hardware
        .as_ref()
        .ok_or_else(|| "The lifecycle baseline has no hardware configuration".to_owned())?
        .pairs
        .iter()
        .filter(|baseline| baseline.id == input.context.pair_id);
    let baseline_pair = baseline_pairs
        .next()
        .ok_or_else(|| "The lifecycle baseline has no selected pair".to_owned())?;
    if baseline_pairs.next().is_some() || pair != baseline_pair {
        return Err("The reconciliation kernel changed persisted pair bytes".into());
    }
    let halves = pair
        .halves
        .as_mut()
        .ok_or_else(|| "The lifecycle candidate pair has no halves record".to_owned())?;

    let mut next_halves = halves.clone();
    let remove_exact = |halves: &mut crate::model::MirroredHalves,
                        association: &super::super::TransitionAssociation|
     -> Result<(), String> {
        let matches: Vec<_> = halves
            .correspondences
            .iter()
            .enumerate()
            .filter(|(_, record)| {
                record.kind == association.kind
                    && record.source_id == association.source_id
                    && record.mirror_id == association.mirror_id
            })
            .map(|(index, _)| index)
            .collect();
        let [index] = matches.as_slice() else {
            return Err(format!(
                "Retired {:?} association {:?} does not have one exact durable pair record",
                association.kind, association.source_id
            ));
        };
        halves.correspondences.remove(*index);
        // A retired association also retires its state overlay; a Local target stays as a plain
        // mirror item with no source.
        halves.exceptions.retain(|record| {
            !(record.kind == association.kind && record.mirror_id == association.mirror_id)
        });
        Ok(())
    };

    let relocate_exact = |halves: &mut crate::model::MirroredHalves,
                          old: &super::super::TransitionAssociation,
                          new: &super::super::TransitionAssociation|
     -> Result<(), String> {
        let matches: Vec<_> = halves
            .correspondences
            .iter()
            .enumerate()
            .filter(|(_, record)| {
                record.kind == old.kind
                    && record.source_id == old.source_id
                    && record.mirror_id == old.mirror_id
            })
            .map(|(index, _)| index)
            .collect();
        let [index] = matches.as_slice() else {
            return Err(format!(
                "Relocated {:?} association {:?} has no exact durable pair record",
                old.kind, old.source_id
            ));
        };
        let record = &mut halves.correspondences[*index];
        record.source_id = new.source_id.clone();
        record.mirror_id = new.mirror_id.clone();
        record.companion_mode = new.raw_mode_override;
        record.companion_tweak = new.raw_tweak;
        for overlay in halves
            .exceptions
            .iter_mut()
            .filter(|record| record.kind == old.kind && record.mirror_id == old.mirror_id)
        {
            overlay.mirror_id = new.mirror_id.clone();
        }
        Ok(())
    };

    for retired in &output.context_delta.retired {
        remove_exact(&mut next_halves, retired)?;
    }
    for (old_source_id, new_source_id) in &output.context_delta.relocated {
        let old = input
            .context
            .associations
            .iter()
            .find(|association| association.source_id == *old_source_id)
            .ok_or_else(|| format!("The relocated source {old_source_id:?} is unavailable"))?;
        let new = output
            .context
            .associations
            .iter()
            .find(|association| association.source_id == *new_source_id)
            .ok_or_else(|| format!("The relocated source {new_source_id:?} is unavailable"))?;
        relocate_exact(&mut next_halves, old, new)?;
    }
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
        next_halves.correspondences.push(MirroredCorrespondence {
            kind: added.kind,
            source_id: added.source_id.clone(),
            mirror_id: added.mirror_id.clone(),
            companion_mode: added.raw_mode_override,
            companion_tweak: added.raw_tweak,
        });
    }

    if output.context.pair_right != input.context.pair_right
        || output.context.pending_pair_transitions != input.context.pending_pair_transitions
        || output.context.pending_pair_baseline != input.context.pending_pair_baseline
    {
        return Err("The reconciliation kernel changed pending private pair evidence".into());
    }
    match (
        input.context.pending_pair_transitions.is_empty(),
        input.context.pending_pair_baseline.as_ref(),
    ) {
        (true, None) => {}
        (false, Some(snapshot))
            if baseline_pair.halves.as_ref() == Some(snapshot)
                && snapshot == &input.context.pair_right => {}
        _ => return Err("The private pending pair baseline is stale or incomplete".into()),
    }

    // Apply exact association changes only after the projection kernel succeeds. `next_halves`
    // starts as the authentic durable pair, so unrelated rows and their order are preserved.
    // Identity never moves: the correspondence stays in every state and only its overlay changes.
    for (source_id, prior_state) in &input.context.pending_pair_transitions {
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
    for source_id in input.context.pending_rejoins.keys().filter(|source_id| {
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
        let mut sources = input
            .post_source
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
    output.context.pending_pair_baseline = None;
    Ok(output)
}
