//! Lifecycle context validation and matrix projection for a membership transition.

use super::super::{MembershipContext, TransitionState};
use super::{durable_state, effective, matrix_pair_for_association, parent};
use crate::model::MirroredItemKind;
use std::collections::BTreeSet;

pub(super) fn matrix_cell_child_has_exact_membership(
    document: &crate::model::ProjectDoc,
    context: &MembershipContext,
    owner: &super::super::TransitionAssociation,
    association: &super::super::TransitionAssociation,
    require_target_membership: bool,
) -> bool {
    let matching_owners = context
        .pair_right
        .correspondences
        .iter()
        .filter(|record| record.kind == MirroredItemKind::Matrix)
        .filter(|record| {
            let mut source_matrices = document
                .matrices
                .iter()
                .filter(|matrix| matrix.id == record.source_id);
            let Some(source_matrix) = source_matrices.next() else {
                return false;
            };
            if source_matrices.next().is_some()
                || source_matrix.board_id.as_deref() != Some(context.source_board_id.as_str())
                || !source_matrix.part_ids.contains(&owner.source_id)
                || !source_matrix.part_ids.contains(&association.source_id)
            {
                return false;
            }

            let mut mirror_matrices = document
                .matrices
                .iter()
                .filter(|matrix| matrix.id == record.mirror_id);
            let mirror_matrix = mirror_matrices.next();
            if mirror_matrices.next().is_some() {
                return false;
            }
            if require_target_membership
                && !mirror_matrix.is_some_and(|matrix| {
                    matrix.board_id.as_deref() == Some(context.mirror_board_id.as_str())
                        && matrix.part_ids.contains(&owner.mirror_id)
                        && matrix.part_ids.contains(&association.mirror_id)
                })
            {
                return false;
            }

            source_matrix.cells.iter().any(|cell| {
                let source_key =
                    format!("matrix/{}/r{}c{}", source_matrix.id, cell.row, cell.column);
                let mirror_key =
                    format!("matrix/{}/r{}c{}", record.mirror_id, cell.row, cell.column);
                source_key == owner.source_id
                    && mirror_key == owner.mirror_id
                    && cell.assemblies.iter().any(|assembly| {
                        association.source_id
                            == crate::matrix::companion_id(&source_key, &assembly.id)
                            && association.mirror_id
                                == crate::matrix::companion_id(&mirror_key, &assembly.id)
                    })
            })
        })
        .count();
    matching_owners == 1
}

#[doc(hidden)]
pub fn validate_lifecycle_context(
    document: &crate::model::ProjectDoc,
    context: &MembershipContext,
) -> Result<(), String> {
    let mut sources = BTreeSet::new();
    let mut mirrors = BTreeSet::new();
    let mut expected_rejoins = std::collections::BTreeMap::new();
    for source_id in context.pending_pair_transitions.keys() {
        let mut selected = context
            .associations
            .iter()
            .filter(|association| association.source_id == *source_id);
        let association = selected
            .next()
            .ok_or("A selected lifecycle owner lacks its exact association")?;
        if selected.next().is_some() {
            return Err("A selected lifecycle owner is ambiguous".into());
        }
        if association.kind == MirroredItemKind::Part {
            let actual_owner = matrix_pair_for_association(
                document,
                &context.pair_right,
                association.kind,
                &association.source_id,
                &association.mirror_id,
            )?;
            if actual_owner
                .as_ref()
                .is_some_and(|owner| context.matrix_pair.as_ref() != Some(owner))
            {
                return Err(
                    "The selected lifecycle Matrix owner disagrees with actual ownership".into(),
                );
            }
        }
    }
    for association in &context.associations {
        if !sources.insert((
            format!("{:?}", association.kind),
            association.source_id.as_str(),
        )) || !mirrors.insert((
            format!("{:?}", association.kind),
            association.mirror_id.as_str(),
        )) {
            return Err("Lifecycle associations contain duplicate identities".into());
        }
        let old = effective(context, association, false, 0)?;
        let settings = context
            .pair_right
            .correspondences
            .iter()
            .find(|r| {
                r.kind == association.kind
                    && r.source_id == association.source_id
                    && r.mirror_id == association.mirror_id
            })
            .map(|r| (r.companion_mode, r.companion_tweak));
        if settings != Some((association.raw_mode_override, association.raw_tweak))
            || association.effective_mode
                != association
                    .raw_mode_override
                    .unwrap_or(context.pair_right.companion_mode)
            || association.effective_tweak != association.raw_tweak
        {
            return Err("Lifecycle companion settings disagree with their durable record".into());
        }
        if let Some(owner) = parent(context, association)? {
            let valid_edge = match (owner.kind, association.kind) {
                (MirroredItemKind::Matrix, MirroredItemKind::Part) => {
                    document.matrices.iter().any(|matrix| {
                        matrix.id == owner.source_id
                            && matrix.board_id.as_deref() == Some(context.source_board_id.as_str())
                            && matrix.part_ids.contains(&association.source_id)
                            && association
                                .source_id
                                .strip_prefix(&format!("matrix/{}/", owner.source_id))
                                .is_some_and(|suffix| {
                                    association.mirror_id
                                        == format!("matrix/{}/{suffix}", owner.mirror_id)
                                })
                    })
                }
                (MirroredItemKind::Part, MirroredItemKind::Part) => {
                    matrix_cell_child_has_exact_membership(
                        document,
                        context,
                        owner,
                        association,
                        false,
                    )
                }
                (
                    MirroredItemKind::EmbeddedCircuit,
                    MirroredItemKind::Part | MirroredItemKind::Net,
                ) => document.embedded_circuits.iter().any(|circuit| {
                    circuit.id == owner.source_id
                        && circuit.host_board_id == context.source_board_id
                        && match association.kind {
                            MirroredItemKind::Part => {
                                circuit.part_ids.contains(&association.source_id)
                            }
                            _ => circuit.net_ids.contains(&association.source_id),
                        }
                        && association
                            .source_id
                            .strip_prefix(&format!("embedded/{}/", owner.source_id))
                            .is_some_and(|suffix| {
                                association.mirror_id
                                    == format!("embedded/{}/{suffix}", owner.mirror_id)
                            })
                }),
                _ => false,
            };
            if !valid_edge {
                return Err(
                    "Lifecycle ownership does not match the domain's exact owned membership".into(),
                );
            }
            let target_owned = match owner.kind {
                MirroredItemKind::EmbeddedCircuit => document
                    .embedded_circuits
                    .iter()
                    .find(|c| c.id == owner.mirror_id)
                    .map(|c| match association.kind {
                        MirroredItemKind::Part => c.part_ids.contains(&association.mirror_id),
                        _ => c.net_ids.contains(&association.mirror_id),
                    })
                    .unwrap_or(false),
                MirroredItemKind::Matrix => document.matrices.iter().any(|matrix| {
                    matrix.id == owner.mirror_id
                        && matrix.board_id.as_deref() == Some(context.mirror_board_id.as_str())
                        && matrix.part_ids.contains(&association.mirror_id)
                }),
                MirroredItemKind::Part => matrix_cell_child_has_exact_membership(
                    document,
                    context,
                    owner,
                    association,
                    true,
                ),
                _ => false,
            };
            let mut omitted_owner = false;
            let mut ancestor = Some(owner);
            for _ in 0..context.associations.len() {
                let Some(current) = ancestor else {
                    break;
                };
                omitted_owner |= effective(context, current, false, 0)? == TransitionState::Omitted;
                ancestor = parent(context, current)?;
            }
            if !target_owned && old != TransitionState::Omitted && !omitted_owner {
                return Err("Lifecycle ownership does not match actual target membership".into());
            }
        }
        let desired = effective(context, association, true, 0)?;
        let (_, durable_explicit) = durable_state(context, association)?;
        let explicit = if context
            .pending_pair_transitions
            .contains_key(&association.source_id)
        {
            association.explicit_state
        } else {
            durable_explicit
        };
        if association.state != desired || association.explicit_state != explicit {
            return Err("Lifecycle ownership state disagrees with its selected owner or explicit child exception".into());
        }
        if desired == TransitionState::Linked && old != TransitionState::Linked {
            expected_rejoins.insert(association.source_id.clone(), old);
        }
        if association.kind == MirroredItemKind::Part {
            let expected = if desired == TransitionState::Omitted {
                super::super::FinalOwner::Absent
            } else {
                super::super::FinalOwner::Board(context.mirror_board_id.clone())
            };
            if context.owners.get(&association.mirror_id) != Some(&expected) {
                return Err("Lifecycle ownership has a stale final Part owner".into());
            }
            for (id, board_id, absent) in [
                (&association.source_id, &context.source_board_id, false),
                (
                    &association.mirror_id,
                    &context.mirror_board_id,
                    old == TransitionState::Omitted,
                ),
            ] {
                let count = document.parts.iter().filter(|part| part.id == *id).count();
                let owners: Vec<_> = document
                    .boards
                    .iter()
                    .filter(|board| board.part_ids.contains(id))
                    .collect();
                if (absent && (count != 0 || !owners.is_empty()))
                    || (!absent && (count != 1 || owners.len() != 1 || owners[0].id != *board_id))
                {
                    return Err(
                        "Lifecycle ownership disagrees with the actual baseline Part closure"
                            .into(),
                    );
                }
                if document
                    .parts
                    .iter()
                    .filter(|part| part.id == *id)
                    .any(|part| {
                        !part.pose.at.x.is_finite()
                            || !part.pose.at.y.is_finite()
                            || !part.pose.rotation.is_finite()
                    })
                {
                    return Err("Lifecycle Part geometry is not finite".into());
                }
            }
        }
    }
    for record in &context.pair_right.correspondences {
        if context
            .associations
            .iter()
            .filter(|a| {
                a.kind == record.kind
                    && a.source_id == record.source_id
                    && a.mirror_id == record.mirror_id
            })
            .count()
            != 1
        {
            return Err("The lifecycle context omits a durable correspondence".into());
        }
    }
    if context.pending_pair_transitions.keys().any(|source| {
        context
            .associations
            .iter()
            .filter(|a| a.source_id == *source)
            .count()
            != 1
    }) || context.pending_rejoins != expected_rejoins
    {
        return Err("The pending lifecycle selection is stale or incomplete".into());
    }
    Ok(())
}

/// Reuse the membership kernel for fixed matrix exceptions as well as membership edits.
/// The outer transaction authenticates the real baseline; this private view narrows ownership
/// to the matrix and temporarily normalizes an explicitly Rejoined owner for that kernel.
pub(super) fn project_lifecycle_matrix(
    draft: &mut crate::model::ProjectDoc,
    context: &MembershipContext,
    additions: &[super::super::TransitionAssociation],
    dispatch: crate::mirrored_half::EditDispatch,
) -> Result<(), String> {
    use super::super::{FinalOwner, MembershipTransitionInput};
    use crate::model::MirroredItemKind;
    use std::collections::BTreeMap;
    let Some(matrix_pair) = context.matrix_pair.as_ref() else {
        return Ok(());
    };
    if !draft
        .matrices
        .iter()
        .any(|m| m.id == matrix_pair.source_id.as_str())
    {
        return Ok(());
    }
    let owner = context
        .associations
        .iter()
        .find(|a| {
            a.kind == MirroredItemKind::Matrix
                && a.source_id == matrix_pair.source_id.as_str()
                && a.mirror_id == matrix_pair.mirror_id.as_str()
        })
        .ok_or("The exact lifecycle Matrix association is unavailable")?;
    if owner.state == TransitionState::Omitted {
        // A matrix-level Omitted mirror is record-absent by design (the
        // halves record requires it): nothing projects onto it. A stale
        // record would only survive outside this lifecycle; leave it for the
        // kernel's member accounting to refuse rather than rewriting it here.
        return Ok(());
    }
    if owner.state == TransitionState::Local {
        let current = draft
            .matrices
            .iter()
            .find(|m| m.id == matrix_pair.mirror_id.as_str())
            .ok_or("The Local lifecycle Matrix is unavailable")?
            .clone();
        let mut next = current.clone();
        for cell in &mut next.cells {
            let key = format!(
                "matrix/{}/r{}c{}",
                matrix_pair.mirror_id.as_str(),
                cell.row,
                cell.column
            );
            if context.associations.iter().any(|a| {
                a.kind == MirroredItemKind::Part
                    && a.mirror_id == key
                    && a.state == TransitionState::Omitted
            }) {
                cell.enabled = false;
                cell.deleted = true;
                cell.assemblies.clear();
            } else {
                cell.assemblies.retain(|assembly| {
                    !context.associations.iter().any(|a| {
                        a.kind == MirroredItemKind::Part
                            && a.state == TransitionState::Omitted
                            && a.mirror_id == crate::matrix::companion_id(&key, &assembly.id)
                    })
                });
            }
        }
        if next != current {
            // The local descriptor owns its geometry. Only remove selected slots; preserve
            // every retained Local Part byte-for-byte across the existing native setter.
            let retained: Vec<_> = context
                .associations
                .iter()
                .filter(|a| a.kind == MirroredItemKind::Part && a.state == TransitionState::Local)
                .filter_map(|a| draft.parts.iter().find(|p| p.id == a.mirror_id).cloned())
                .collect();
            let materialized: BTreeSet<_> = next
                .cells
                .iter()
                .filter(|cell| cell.enabled && !cell.deleted)
                .flat_map(|cell| {
                    let key = format!("matrix/{}/r{}c{}", next.id, cell.row, cell.column);
                    std::iter::once(key.clone()).chain(
                        cell.assemblies
                            .iter()
                            .map(move |a| crate::matrix::companion_id(&key, &a.id)),
                    )
                })
                .collect();
            let detached: Vec<_> = retained
                .iter()
                .filter(|p| current.part_ids.contains(&p.id) && !materialized.contains(&p.id))
                .cloned()
                .collect();
            draft
                .matrices
                .iter_mut()
                .find(|m| m.id == current.id)
                .unwrap()
                .part_ids
                .retain(|id| !detached.iter().any(|p| p.id == *id));
            crate::matrix::set_matrix(draft, &next)?;
            for local in retained {
                let target = draft
                    .parts
                    .iter_mut()
                    .find(|p| p.id == local.id)
                    .ok_or("A retained Local Matrix child was removed")?;
                *target = local;
            }
        }
        return Ok(());
    }
    let source = draft
        .matrices
        .iter()
        .find(|m| m.id == matrix_pair.source_id.as_str())
        .ok_or("The lifecycle source Matrix is unavailable")?;
    let mirror = draft
        .matrices
        .iter()
        .find(|m| m.id == matrix_pair.mirror_id.as_str())
        .ok_or("The lifecycle mirror Matrix is unavailable")?;
    let source_map: BTreeMap<_, _> = source
        .part_ids
        .iter()
        .map(|id| (id.clone(), Some(id.clone())))
        .collect();
    let mut matrix_context = context.clone();
    for added in additions {
        matrix_context.associations.push(added.clone());
        matrix_context
            .pair_right
            .correspondences
            .push(crate::model::MirroredCorrespondence {
                kind: added.kind,
                source_id: added.source_id.clone(),
                mirror_id: added.mirror_id.clone(),
                companion_mode: added.raw_mode_override,
                companion_tweak: added.raw_tweak,
            });
    }
    matrix_context.associations.retain(|a| {
        (a.kind == MirroredItemKind::Matrix && a.source_id == matrix_pair.source_id.as_str())
            || (a.kind == MirroredItemKind::Part
                && a.source_id
                    .starts_with(&format!("matrix/{}/", matrix_pair.source_id.as_str())))
    });
    for association in &mut matrix_context.associations {
        if association.parent_source_id.as_deref() == Some(matrix_pair.source_id.as_str()) {
            association.parent_source_id = None;
            association.parent_mirror_id = None;
        }
    }
    let matrix_sources: BTreeSet<_> = matrix_context
        .associations
        .iter()
        .filter(|a| a.kind == MirroredItemKind::Part)
        .map(|a| a.source_id.clone())
        .collect();
    matrix_context
        .pending_pair_transitions
        .retain(|source, _| matrix_sources.contains(source));
    matrix_context
        .pending_rejoins
        .retain(|source, _| matrix_sources.contains(source));
    if context
        .pending_pair_transitions
        .contains_key(&owner.source_id)
    {
        let matching: Vec<_> = matrix_context
            .pair_right
            .exceptions
            .iter()
            .enumerate()
            .filter(|(_, e)| e.kind == owner.kind && e.mirror_id == owner.mirror_id)
            .map(|(index, _)| index)
            .collect();
        let [index] = matching.as_slice() else {
            return Err("The Rejoined Matrix has no exact durable owner exception".into());
        };
        // The correspondence is retained in every state; Rejoin clears only the overlay.
        matrix_context.pair_right.exceptions.remove(*index);
        matrix_context.pending_rejoins.retain(|source, _| {
            let mut association = matrix_context
                .associations
                .iter()
                .find(|a| a.source_id == *source);
            for _ in 0..matrix_context.associations.len() {
                let Some(current) = association else {
                    return false;
                };
                if matrix_context
                    .pending_pair_transitions
                    .contains_key(&current.source_id)
                {
                    return true;
                }
                association = matrix_context.associations.iter().find(|parent| {
                    current.parent_source_id.as_deref() == Some(parent.source_id.as_str())
                        && current.parent_mirror_id.as_deref() == Some(parent.mirror_id.as_str())
                });
            }
            false
        });
    }
    matrix_context.pending_pair_baseline = (!matrix_context.pending_pair_transitions.is_empty())
        .then(|| matrix_context.pair_right.clone());
    for part in &draft.parts {
        let mut boards = draft
            .boards
            .iter()
            .filter(|b| b.part_ids.contains(&part.id));
        let board = boards
            .next()
            .ok_or("A lifecycle matrix draft Part has no owner")?;
        if boards.next().is_some() {
            return Err("A lifecycle matrix draft Part has ambiguous ownership".into());
        }
        matrix_context
            .owners
            .insert(part.id.clone(), FinalOwner::Board(board.id.clone()));
    }
    for (id, owner) in &mut matrix_context.owners {
        if !draft.parts.iter().any(|p| p.id == *id) {
            *owner = FinalOwner::Absent;
        }
    }
    for association in &matrix_context.associations {
        if association.kind == MirroredItemKind::Part {
            matrix_context.owners.insert(
                association.mirror_id.clone(),
                if association.state == TransitionState::Omitted {
                    FinalOwner::Absent
                } else {
                    FinalOwner::Board(context.mirror_board_id.clone())
                },
            );
        }
    }
    let mirror_map: BTreeMap<_, _> = mirror
        .part_ids
        .iter()
        .map(|id| {
            let keep = !matrix_context.associations.iter().any(|a| {
                a.kind == MirroredItemKind::Part
                    && a.mirror_id == *id
                    && a.state == TransitionState::Omitted
            });
            (id.clone(), keep.then(|| id.clone()))
        })
        .collect();
    let closure: BTreeSet<_> = mirror_map
        .iter()
        .filter(|(_, mapped)| mapped.is_none())
        .map(|(id, _)| id.clone())
        .collect();
    let mut frame = draft.clone();
    frame
        .hardware
        .as_mut()
        .unwrap()
        .pairs
        .iter_mut()
        .find(|p| p.id == context.pair_id)
        .unwrap()
        .halves = Some(matrix_context.pair_right.clone());
    let output = super::super::transition::project_membership(MembershipTransitionInput {
        dispatch,
        valid_before: &frame,
        post_source: &frame,
        context: &matrix_context,
        baseline_kind: super::super::BaselineKind::ChainedPrivateDraft,
        source_identity_map: &source_map,
        mirror_identity_map: &mirror_map,
        planned_additions: &[],
        final_owners: &matrix_context.owners,
        explicit_removal_closure: &closure,
    })?;
    let mut projected = output.draft;
    projected
        .hardware
        .as_mut()
        .unwrap()
        .pairs
        .iter_mut()
        .find(|p| p.id == context.pair_id)
        .unwrap()
        .halves = Some(context.pair_right.clone());
    *draft = projected;
    Ok(())
}
