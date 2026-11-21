//! Lifecycle context and matrix ownership validation.

use super::super::{MembershipContext, TransitionState};
use super::{effective, parent};
use crate::model::MirroredItemKind;

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

pub(super) fn validate_lifecycle_context(
    document: &crate::model::ProjectDoc,
    context: &MembershipContext,
) -> Result<(), String> {
    for association in &context.associations {
        let old = effective(context, association, false, 0)?;
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
        if association.kind == MirroredItemKind::Part {
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
    Ok(())
}
