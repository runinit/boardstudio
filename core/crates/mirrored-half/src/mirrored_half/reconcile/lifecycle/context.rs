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
        if matches!(
            association.kind,
            MirroredItemKind::Matrix | MirroredItemKind::Part
        ) {
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
