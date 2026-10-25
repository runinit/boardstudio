//! Private lifecycle operations for durable mirrored-half exceptions and Unlink.

#[cfg(test)]
#[path = "lifecycle/tests.rs"]
mod tests;

use super::{MembershipContext, TransitionState};
use crate::constraints::{ConstraintEndpoint, FramedConstraint};
use crate::model::{
    AggregateAnchor, AnchorPart, Constraint, HalfExceptionState, MirroredExceptionState,
    MirroredItemKind,
};
use std::collections::BTreeSet;

#[derive(Clone, Debug, PartialEq, Eq)]
enum ConstraintLifecycleEffect {
    MakeLocal {
        pair_id: String,
        source_part_id: String,
        mirror_part_id: String,
    },
    RemoveVisibleParts {
        part_ids: BTreeSet<String>,
    },
    Rejoin {
        pair_id: String,
        source_part_id: String,
        mirror_part_id: String,
    },
    Unlink {
        pair_id: String,
    },
}

#[derive(Clone, Debug, Default, PartialEq)]
struct ConstraintLifecycleDelta {
    upserts: Vec<FramedConstraint>,
    removals: Vec<String>,
}

/// Prepares endpoint edits after the owning lifecycle kernel has validated the exact transition
/// and removal closure. Records are compiled against the pre-transition document; removed IDs
/// are visible Part IDs from that same validated closure, never canonical aliases.
fn prepare_constraint_lifecycle_delta(
    records: &[FramedConstraint],
    effects: &[ConstraintLifecycleEffect],
) -> Result<ConstraintLifecycleDelta, String> {
    let mut seen_ids = BTreeSet::new();
    let mut upserts = std::collections::BTreeMap::new();
    let mut removals = BTreeSet::new();

    for record in records {
        let FramedConstraint::Framed {
            id,
            source: original_source,
            target: original_target,
            relation,
        } = record;
        if !seen_ids.insert(id.as_str()) {
            return Err(format!(
                "Constraint {id} appears more than once in lifecycle preparation"
            ));
        }

        let removed = effects.iter().any(|effect| {
            let ConstraintLifecycleEffect::RemoveVisibleParts { part_ids } = effect else {
                return false;
            };
            part_ids.contains(visible_part_id(original_source))
                || part_ids.contains(visible_part_id(original_target))
        });
        if removed {
            removals.insert(id.clone());
            continue;
        }

        let mut source = original_source.clone();
        let mut target = original_target.clone();
        let mut retire_on_rejoin = false;
        for effect in effects {
            match effect {
                ConstraintLifecycleEffect::MakeLocal {
                    pair_id,
                    source_part_id,
                    mirror_part_id,
                } => {
                    localize_endpoint(&mut source, pair_id, source_part_id, mirror_part_id);
                    localize_endpoint(&mut target, pair_id, source_part_id, mirror_part_id);
                }
                ConstraintLifecycleEffect::RemoveVisibleParts { .. } => {}
                ConstraintLifecycleEffect::Rejoin {
                    pair_id,
                    source_part_id,
                    mirror_part_id,
                } => {
                    if matches!(&target, ConstraintEndpoint::Part { part_id } if part_id == mirror_part_id)
                    {
                        retire_on_rejoin = true;
                        break;
                    }
                    if matches!(&source, ConstraintEndpoint::Part { part_id } if part_id == mirror_part_id)
                    {
                        source = linked_endpoint(pair_id, source_part_id, mirror_part_id);
                    }
                }
                ConstraintLifecycleEffect::Unlink { pair_id } => {
                    unlink_endpoint(&mut source, pair_id);
                    unlink_endpoint(&mut target, pair_id);
                }
            }
        }
        if retire_on_rejoin {
            removals.insert(id.clone());
        } else if source != *original_source || target != *original_target {
            upserts.insert(
                id.clone(),
                FramedConstraint::Framed {
                    id: id.clone(),
                    source,
                    target,
                    relation: relation.clone(),
                },
            );
        }
    }

    Ok(ConstraintLifecycleDelta {
        upserts: upserts.into_values().collect(),
        removals: removals.into_iter().collect(),
    })
}

fn visible_part_id(endpoint: &ConstraintEndpoint) -> &str {
    match endpoint {
        ConstraintEndpoint::Part { part_id } => part_id,
        ConstraintEndpoint::LinkedMirror { mirror_part_id, .. } => mirror_part_id,
    }
}

fn localize_endpoint(
    endpoint: &mut ConstraintEndpoint,
    pair_id: &str,
    source_part_id: &str,
    mirror_part_id: &str,
) {
    if matches!(endpoint, ConstraintEndpoint::LinkedMirror {
        pair_id: actual_pair_id,
        source_part_id: actual_source_part_id,
        mirror_part_id: actual_mirror_part_id,
    } if actual_pair_id == pair_id
        && actual_source_part_id == source_part_id
        && actual_mirror_part_id == mirror_part_id)
    {
        *endpoint = ConstraintEndpoint::Part {
            part_id: mirror_part_id.to_owned(),
        };
    }
}

fn linked_endpoint(
    pair_id: &str,
    source_part_id: &str,
    mirror_part_id: &str,
) -> ConstraintEndpoint {
    ConstraintEndpoint::LinkedMirror {
        pair_id: pair_id.to_owned(),
        source_part_id: source_part_id.to_owned(),
        mirror_part_id: mirror_part_id.to_owned(),
    }
}

fn unlink_endpoint(endpoint: &mut ConstraintEndpoint, pair_id: &str) {
    if let ConstraintEndpoint::LinkedMirror {
        pair_id: actual_pair_id,
        mirror_part_id,
        ..
    } = endpoint
        && actual_pair_id == pair_id
    {
        *endpoint = ConstraintEndpoint::Part {
            part_id: mirror_part_id.clone(),
        };
    }
}

/// The state the pair stores for an association. Its correspondence is its identity in every
/// state; a Local or Omitted overlay on that correspondence is the only thing that differs.
fn durable_state(
    context: &MembershipContext,
    association: &super::TransitionAssociation,
) -> Result<(TransitionState, bool), String> {
    let correspondences: Vec<_> = context
        .pair_right
        .correspondences
        .iter()
        .filter(|record| {
            record.kind == association.kind
                && (record.source_id == association.source_id
                    || record.mirror_id == association.mirror_id)
        })
        .collect();
    let [record] = correspondences.as_slice() else {
        return Err("A lifecycle association lacks exact exclusive durable provenance".into());
    };
    if record.source_id != association.source_id || record.mirror_id != association.mirror_id {
        return Err("A lifecycle association lacks exact exclusive durable provenance".into());
    }
    let overlays: Vec<_> = context
        .pair_right
        .exceptions
        .iter()
        .filter(|exception| {
            exception.kind == association.kind && exception.mirror_id == association.mirror_id
        })
        .collect();
    match overlays.as_slice() {
        [] => Ok((TransitionState::Linked, false)),
        [overlay] => Ok((
            match overlay.state {
                MirroredExceptionState::Local => TransitionState::Local,
                MirroredExceptionState::Omitted => TransitionState::Omitted,
            },
            true,
        )),
        _ => Err("A lifecycle association lacks exact exclusive durable provenance".into()),
    }
}

fn parent<'a>(
    context: &'a MembershipContext,
    association: &super::TransitionAssociation,
) -> Result<Option<&'a super::TransitionAssociation>, String> {
    match (&association.parent_source_id, &association.parent_mirror_id) {
        (None, None) => Ok(None),
        (Some(source), Some(mirror)) => {
            let mut parents = context
                .associations
                .iter()
                .filter(|a| a.source_id == *source && a.mirror_id == *mirror);
            let result = parents
                .next()
                .ok_or("Lifecycle ownership parent is unavailable")?;
            if parents.next().is_some() {
                return Err("Lifecycle ownership parent is ambiguous".into());
            }
            Ok(Some(result))
        }
        _ => Err("Lifecycle ownership has an incomplete parent edge".into()),
    }
}

fn effective(
    context: &MembershipContext,
    association: &super::TransitionAssociation,
    pending: bool,
    depth: usize,
) -> Result<TransitionState, String> {
    if depth > context.associations.len() {
        return Err("Lifecycle ownership contains a cycle".into());
    }
    let (durable, explicit) = durable_state(context, association)?;
    if pending && let Some(prior) = context.pending_pair_transitions.get(&association.source_id) {
        if *prior != durable {
            return Err("A pending lifecycle state disagrees with its exact baseline".into());
        }
        if association.explicit_state {
            if association.state == TransitionState::Linked {
                return Err("A pending explicit exception cannot be Linked".into());
            }
            return Ok(association.state);
        }
        return match parent(context, association)? {
            Some(owner) => effective(context, owner, pending, depth + 1),
            None => Ok(TransitionState::Linked),
        };
    }
    if explicit {
        return Ok(durable);
    }
    match parent(context, association)? {
        Some(owner) => effective(context, owner, pending, depth + 1),
        None => Ok(TransitionState::Linked),
    }
}

fn matrix_cell_child_has_exact_membership(
    document: &crate::model::ProjectDoc,
    context: &MembershipContext,
    owner: &super::TransitionAssociation,
    association: &super::TransitionAssociation,
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

fn validate_lifecycle_context(
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
                super::FinalOwner::Absent
            } else {
                super::FinalOwner::Board(context.mirror_board_id.clone())
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
fn project_lifecycle_matrix(
    draft: &mut crate::model::ProjectDoc,
    context: &MembershipContext,
    additions: &[super::TransitionAssociation],
) -> Result<(), String> {
    use super::{FinalOwner, MembershipTransitionInput};
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
    let output = super::prepare_membership_transition_kernel(MembershipTransitionInput {
        valid_before: &frame,
        post_source: &frame,
        context: &matrix_context,
        baseline_kind: super::BaselineKind::ChainedPrivateDraft,
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

/// Copper is matched to a pad by position; both come from authored geometry, so a hair of
/// tolerance is enough and anything coarser would grab a neighbouring pad's trace.
const COPPER_TOLERANCE: f64 = 1e-6;

fn near(a: crate::model::Vec2, b: crate::model::Vec2) -> bool {
    (a.x - b.x).abs() <= COPPER_TOLERANCE && (a.y - b.y).abs() <= COPPER_TOLERANCE
}

/// The world position of every pad of a part, from the part's pose and its definition.
fn pad_positions(
    document: &crate::model::ProjectDoc,
    part_id: &str,
) -> Vec<crate::model::OmittedPad> {
    let Some(part) = document.parts.iter().find(|part| part.id == part_id) else {
        return Vec::new();
    };
    let Some(definition) = document
        .definitions
        .iter()
        .find(|definition| definition.id == part.definition_id)
    else {
        return Vec::new();
    };
    definition
        .pads
        .iter()
        .map(|pad| crate::model::OmittedPad {
            part_id: part.id.clone(),
            pad_id: pad.id.clone(),
            at: crate::outline_controls::world(pad.at, part),
        })
        .collect()
}

/// The explicitly selected item an inherited item takes its state from.
fn explicit_root<'a>(
    context: &'a MembershipContext,
    association: &'a super::TransitionAssociation,
) -> &'a super::TransitionAssociation {
    let mut current = association;
    for _ in 0..context.associations.len() {
        if current.explicit_state {
            break;
        }
        match parent(context, current) {
            Ok(Some(owner)) => current = owner,
            _ => break,
        }
    }
    current
}

/// Removes the mirror-board copper that belonged only to the items this transition omits and
/// returns it by the explicitly omitted item that owns it: copper on a net the omission leaves
/// private, and any copper that ends on an omitted pad. The source half is never touched.
fn capture_omitted_copper(
    draft: &mut crate::model::ProjectDoc,
    post_source: &crate::model::ProjectDoc,
    context: &MembershipContext,
) -> Result<std::collections::BTreeMap<String, crate::model::OmittedCopper>, String> {
    use std::collections::BTreeMap;
    let mut newly = Vec::new();
    for association in &context.associations {
        if association.state == TransitionState::Omitted
            && effective(context, association, false, 0)? != TransitionState::Omitted
        {
            newly.push(association);
        }
    }
    if newly.is_empty() {
        return Ok(BTreeMap::new());
    }
    let mut part_owner: BTreeMap<String, String> = BTreeMap::new();
    let mut net_owner: BTreeMap<String, String> = BTreeMap::new();
    for association in &newly {
        let root = explicit_root(context, association).mirror_id.clone();
        match association.kind {
            MirroredItemKind::Part => {
                part_owner.insert(association.mirror_id.clone(), root);
            }
            MirroredItemKind::Net => {
                net_owner.insert(association.mirror_id.clone(), root);
            }
            _ => {}
        }
    }
    let omitted_parts: BTreeSet<String> = part_owner.keys().cloned().collect();

    let mut saved: BTreeMap<String, crate::model::OmittedCopper> = BTreeMap::new();
    let mut pad_points = Vec::new();
    for (part_id, owner) in &part_owner {
        for pad in pad_positions(draft, part_id) {
            pad_points.push((pad.at, owner.clone()));
            saved
                .entry(owner.clone())
                .or_insert_with(|| crate::model::OmittedCopper {
                    traces: Vec::new(),
                    vias: Vec::new(),
                    pads: Vec::new(),
                })
                .pads
                .push(pad);
        }
    }

    // A net is private to the omission when every non-controller pin of its source net is on an
    // omitted part. It goes with them, unless something the mirror authored still uses it.
    let source_controller = post_source
        .hardware
        .as_ref()
        .and_then(|hardware| {
            hardware
                .boards
                .iter()
                .find(|board| board.board_id == context.source_board_id)
        })
        .and_then(|board| board.controller_part_id.as_deref());
    let mirror_controller = source_controller
        .and_then(|id| {
            context
                .pair_right
                .correspondences
                .iter()
                .find(|record| record.kind == MirroredItemKind::Part && record.source_id == id)
        })
        .map(|record| record.mirror_id.clone());
    let local_mirror_controller = mirror_controller.as_ref().is_some_and(|mirror_id| {
        context.associations.iter().any(|association| {
            association.kind == MirroredItemKind::Part
                && association.mirror_id == *mirror_id
                && association.state == TransitionState::Local
        })
    });
    let mut rule_nets = BTreeSet::new();
    for record in context
        .pair_right
        .correspondences
        .iter()
        .filter(|record| record.kind == MirroredItemKind::Net)
    {
        if net_owner.contains_key(&record.mirror_id)
            || record.mirror_id.starts_with("embedded/")
            || context.pair_right.overlay_of(record).is_some()
        {
            continue;
        }
        let Some(mirror_net) = draft.nets.iter().find(|net| net.id == record.mirror_id) else {
            continue;
        };
        if !crate::mechanical::halves::private_net_is_absent_with(
            post_source,
            draft,
            &context.pair_right,
            &record.source_id,
            &omitted_parts,
        ) {
            continue;
        }
        let Some(source_net) = post_source
            .nets
            .iter()
            .find(|net| net.id == record.source_id)
        else {
            continue;
        };
        let owner = source_net
            .pins
            .iter()
            .filter_map(|pin| {
                context
                    .pair_right
                    .correspondences
                    .iter()
                    .find(|c| c.kind == MirroredItemKind::Part && c.source_id == pin.part_id)
            })
            .filter_map(|c| part_owner.get(&c.mirror_id))
            .min()
            .cloned();
        let Some(owner) = owner else {
            continue;
        };
        let still_used = mirror_net.pins.iter().any(|pin| {
            !omitted_parts.contains(&pin.part_id)
                && (Some(&pin.part_id) != mirror_controller.as_ref() || local_mirror_controller)
                && draft.parts.iter().any(|part| part.id == pin.part_id)
        }) || draft.embedded_circuits.iter().any(|circuit| {
            circuit.ports.values().any(|net| *net == record.mirror_id)
                || circuit.net_ids.contains(&record.mirror_id)
        });
        if still_used {
            continue;
        }
        net_owner.insert(record.mirror_id.clone(), owner);
        rule_nets.insert(record.mirror_id.clone());
    }

    let board = draft
        .boards
        .iter_mut()
        .find(|board| board.id == context.mirror_board_id)
        .ok_or("The Omit mirror board is unavailable")?;
    let owner_at = |at: crate::model::Vec2| {
        pad_points
            .iter()
            .find(|(pad, _)| near(*pad, at))
            .map(|(_, owner)| owner.clone())
    };
    let mut kept_traces = Vec::new();
    for trace in std::mem::take(&mut board.traces) {
        let owner = trace
            .net_id
            .as_ref()
            .and_then(|net| net_owner.get(net).cloned())
            .or_else(|| owner_at(trace.start))
            .or_else(|| owner_at(trace.end));
        match owner {
            Some(owner) => saved
                .entry(owner)
                .or_insert_with(|| crate::model::OmittedCopper {
                    traces: Vec::new(),
                    vias: Vec::new(),
                    pads: Vec::new(),
                })
                .traces
                .push(trace),
            None => kept_traces.push(trace),
        }
    }
    board.traces = kept_traces;
    let mut kept_vias = Vec::new();
    for via in std::mem::take(&mut board.vias) {
        let owner = via
            .net_id
            .as_ref()
            .and_then(|net| net_owner.get(net).cloned())
            .or_else(|| owner_at(via.at));
        match owner {
            Some(owner) => saved
                .entry(owner)
                .or_insert_with(|| crate::model::OmittedCopper {
                    traces: Vec::new(),
                    vias: Vec::new(),
                    pads: Vec::new(),
                })
                .vias
                .push(via),
            None => kept_vias.push(via),
        }
    }
    board.vias = kept_vias;
    board.net_ids.retain(|id| !rule_nets.contains(id));
    draft.nets.retain(|net| !rule_nets.contains(&net.id));
    // Pads without copper say nothing worth restoring.
    saved.retain(|_, copper| !copper.traces.is_empty() || !copper.vias.is_empty());
    Ok(saved)
}

/// A Rejoin brings back the nets the omission had left private, and the rejoined parts' pins on
/// the nets that stayed, from the current source.
fn restore_rejoined_nets(
    draft: &mut crate::model::ProjectDoc,
    post_source: &crate::model::ProjectDoc,
    context: &MembershipContext,
) -> Result<(), String> {
    let rejoined: BTreeSet<String> = context
        .associations
        .iter()
        .filter(|a| {
            a.kind == MirroredItemKind::Part
                && a.state == TransitionState::Linked
                && context.pending_rejoins.get(&a.source_id) == Some(&TransitionState::Omitted)
        })
        .map(|a| a.mirror_id.clone())
        .collect();
    if rejoined.is_empty() {
        return Ok(());
    }
    let part_map: std::collections::BTreeMap<_, _> = context
        .pair_right
        .correspondences
        .iter()
        .filter(|record| record.kind == MirroredItemKind::Part)
        .map(|record| (record.source_id.clone(), record.mirror_id.clone()))
        .collect();
    for record in context
        .pair_right
        .correspondences
        .iter()
        .filter(|record| record.kind == MirroredItemKind::Net)
    {
        if record.mirror_id.starts_with("embedded/")
            || context.pair_right.overlay_of(record).is_some()
        {
            continue;
        }
        let Some(source_net) = post_source
            .nets
            .iter()
            .find(|net| net.id == record.source_id)
        else {
            continue;
        };
        let mapped: Vec<crate::model::Pin> = source_net
            .pins
            .iter()
            .filter_map(|pin| {
                let mirror = part_map.get(&pin.part_id)?;
                draft
                    .parts
                    .iter()
                    .any(|part| part.id == *mirror)
                    .then(|| crate::model::Pin {
                        part_id: mirror.clone(),
                        pad_id: pin.pad_id.clone(),
                    })
            })
            .collect();
        if !mapped.iter().any(|pin| rejoined.contains(&pin.part_id)) {
            continue;
        }
        if let Some(net) = draft.nets.iter_mut().find(|net| net.id == record.mirror_id) {
            for pin in mapped
                .into_iter()
                .filter(|pin| rejoined.contains(&pin.part_id))
            {
                if !net.pins.contains(&pin) {
                    net.pins.push(pin);
                }
            }
        } else {
            let board = draft
                .boards
                .iter_mut()
                .find(|board| board.id == context.mirror_board_id)
                .ok_or("The Rejoin mirror board is unavailable")?;
            if board.net_ids.contains(&record.mirror_id) {
                return Err("An absent Rejoin net retains a board registration".into());
            }
            board.net_ids.push(record.mirror_id.clone());
            draft.nets.push(crate::model::Net {
                id: record.mirror_id.clone(),
                name: super::super::mirror_net_name(
                    &context.source_board_id,
                    &context.mirror_board_id,
                    &source_net.name,
                ),
                pins: mapped,
            });
        }
    }
    Ok(())
}

/// Puts back the copper an Omit saved, but only if every saved pad stands exactly where it did.
/// Otherwise nothing is restored and a notice says so; either way the saved copper is gone from
/// the pair, because Rejoin clears the overlay that held it.
fn restore_saved_copper(
    output: &mut super::MembershipTransitionOutput,
    context: &MembershipContext,
) -> Result<(), String> {
    for overlay in &context.pair_right.exceptions {
        let Some(copper) = &overlay.removed_copper else {
            continue;
        };
        let Some(record) = context.pair_right.overlaid_correspondence(overlay) else {
            continue;
        };
        let Some(association) = context.associations.iter().find(|a| {
            a.kind == record.kind
                && a.source_id == record.source_id
                && a.mirror_id == record.mirror_id
        }) else {
            continue;
        };
        if association.state != TransitionState::Linked
            || !context.pending_rejoins.contains_key(&association.source_id)
        {
            continue;
        }
        let unmoved = copper.pads.iter().all(|pad| {
            pad_positions(&output.draft, &pad.part_id)
                .iter()
                .any(|now| now.pad_id == pad.pad_id && near(now.at, pad.at))
        });
        if !unmoved {
            output.notices.push(format!(
                "Copper for {:?} was not restored: its part moved; re-route",
                overlay.mirror_id
            ));
            continue;
        }
        let board = output
            .draft
            .boards
            .iter_mut()
            .find(|board| board.id == context.mirror_board_id)
            .ok_or("The Rejoin mirror board is unavailable")?;
        if copper
            .traces
            .iter()
            .any(|trace| board.traces.iter().any(|live| live.id == trace.id))
            || copper
                .vias
                .iter()
                .any(|via| board.vias.iter().any(|live| live.id == via.id))
        {
            return Err(format!(
                "Saved copper for {:?} collides with live copper",
                overlay.mirror_id
            ));
        }
        board.traces.extend(copper.traces.iter().cloned());
        board.vias.extend(copper.vias.iter().cloned());
    }
    Ok(())
}

fn copy_part_payload(target: &mut crate::model::Part, source: &crate::model::Part) {
    target.side = source.side.clone();
    target.locked = source.locked;
    target.keycap = source.keycap;
    target.outline = source.outline.clone();
    target.properties = source.properties.clone();
    target.generator_parameters = source.generator_parameters.clone();
}

/// Coordinate durable exceptions with the existing forward projector. Membership-changing
/// matrix edits continue through `prepare_membership_transition`; this seam handles fixed
/// membership and selective independent/hosted exceptions.
/// Ticket 29: a Rejoin adopts the source anchor onto the mirror half. The
/// projection copies the anchor verbatim, so remap every target through the
/// pair's matrix correspondences and refuse — naming the missing key —
/// instead of materializing a cross-half anchor or silently retargeting.
fn adopt_rejoined_anchor(
    projected: &mut crate::model::Matrix,
    draft: &crate::model::ProjectDoc,
    context: &MembershipContext,
    matrix_pair: &super::MatrixPairContext,
) -> Result<(), String> {
    let Some(anchor) = projected.anchor.clone() else {
        return Ok(());
    };
    let counterpart = |matrix_id: &str| {
        context
            .pair_right
            .correspondences
            .iter()
            .find(|record| record.kind == MirroredItemKind::Matrix && record.source_id == matrix_id)
            .map(|record| record.mirror_id.clone())
    };
    // A counterpart slot is missing when its matrix record is gone, the
    // slot is out of range, or the slot is explicitly removed.
    let slot_missing = |mirror_id: &str, row: u32, column: u32| -> bool {
        let Some(record) = draft.matrices.iter().find(|matrix| matrix.id == mirror_id) else {
            return true;
        };
        if row >= record.rows || column >= record.columns {
            return true;
        }
        record
            .cells
            .iter()
            .any(|cell| cell.row == row && cell.column == column && (cell.deleted || !cell.enabled))
    };
    let mut next = anchor.clone();
    match &anchor.aggregate {
        Some(aggregate) => {
            let mut parts = Vec::with_capacity(aggregate.parts.len());
            for part in &aggregate.parts {
                let Some(mirror_id) = counterpart(&part.matrix_id) else {
                    return Err(format!(
                        "Cannot Rejoin {}: the adopted source anchor targets {:?}, which has no mirrored counterpart in this pair",
                        matrix_pair.mirror_id, part.matrix_id
                    ));
                };
                if slot_missing(&mirror_id, part.row, part.column) {
                    return Err(format!(
                        "Cannot Rejoin {}: the adopted source anchor targets matrix/{}/r{}c{}, whose mirror counterpart matrix/{}/r{}c{} is missing",
                        matrix_pair.mirror_id,
                        part.matrix_id,
                        part.row,
                        part.column,
                        mirror_id,
                        part.row,
                        part.column
                    ));
                }
                parts.push(AnchorPart {
                    matrix_id: mirror_id,
                    row: part.row,
                    column: part.column,
                });
            }
            next.aggregate = Some(AggregateAnchor {
                method: aggregate.method,
                parts,
            });
        }
        None => {
            let Some(mirror_id) = counterpart(&anchor.matrix_id) else {
                return Err(format!(
                    "Cannot Rejoin {}: the adopted source anchor targets {:?}, which has no mirrored counterpart in this pair",
                    matrix_pair.mirror_id, anchor.matrix_id
                ));
            };
            if slot_missing(&mirror_id, anchor.row, anchor.column) {
                return Err(format!(
                    "Cannot Rejoin {}: the adopted source anchor targets matrix/{}/r{}c{}, whose mirror counterpart matrix/{}/r{}c{} is missing",
                    matrix_pair.mirror_id,
                    anchor.matrix_id,
                    anchor.row,
                    anchor.column,
                    mirror_id,
                    anchor.row,
                    anchor.column
                ));
            }
            next.matrix_id = mirror_id;
        }
    }
    projected.anchor = Some(next);
    Ok(())
}

/// Ticket 29: the native shared-detach proposal. After reconciliation, a
/// plain detach that moved its linked native partner refuses with the
/// ticket-23 confirmation shape, so the UI can offer the confirmed variant
/// verbatim. A partner that owns its pose (Local/Omitted) never moves
/// through the projection and is skipped; anything else keeps its poses.
pub(super) fn native_detach_proposal(
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

pub(super) fn prepare_edit_transition(
    before: &crate::model::ProjectDoc,
    post_source: &crate::model::ProjectDoc,
    pair_id: &str,
    operation: &crate::model::EditOperation,
) -> Result<Option<super::PreparedLifecycleDraft>, String> {
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
        let output = prepare_lifecycle_transition(before, post_source, &pending)?;
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
            Some(super::MatrixPairContext {
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
        let output = prepare_lifecycle_transition(before, post_source, &pending)?;
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

    if let EditOperation::SetMechanical {
        configuration: None,
        target: Some(crate::mechanical::MechanicalTarget::Instance { instance_id }),
    } = operation
    {
        let hardware = before
            .hardware
            .as_ref()
            .ok_or("Physical instances are unavailable")?;
        let mut pairs = hardware.pairs.iter().filter(|pair| pair.id == pair_id);
        let Some(pair) = pairs.next() else {
            return Ok(None);
        };
        if pairs.next().is_some() {
            return Err("The exact lifecycle pair is ambiguous".into());
        }
        if !pair.members.iter().any(|member| member == instance_id) || pair.halves.is_none() {
            return Ok(None);
        }

        let records = lifecycle_constraint_records(before)?;
        let delta = prepare_constraint_lifecycle_delta(
            &records,
            &[ConstraintLifecycleEffect::Unlink {
                pair_id: pair.id.clone(),
            }],
        )?;
        let mut draft = post_source.clone();
        apply_constraint_lifecycle_delta(&mut draft, delta);
        return Ok(Some(super::PreparedLifecycleDraft {
            draft,
            changed_document_ids: Vec::new(),
        }));
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
        return Ok(Some(super::PreparedLifecycleDraft {
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
        .filter(|(source, _)| super::matrix_membership_changed(before, post_source, source))
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
                Some(super::MatrixPairContext {
                    source_id: source_matrix_id.clone(),
                    mirror_id: mirror_matrix_id.clone(),
                }),
            )?;
            let output = prepare_matrix_membership_edit(before, &draft, operation, &context)?;
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
        return Ok(Some(super::PreparedLifecycleDraft {
            draft,
            changed_document_ids,
        }));
    }
    let matrix_pair = matrix_pairs
        .into_iter()
        .next()
        .map(|(source_id, mirror_id)| super::MatrixPairContext {
            source_id,
            mirror_id,
        });
    let context = lifecycle_context(before, pair_id, matrix_pair)?;
    let output = prepare_lifecycle_transition(before, post_source, &context)?;
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
pub(super) fn retire_replacement_records(
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

fn removed_part_ids(
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

fn lifecycle_constraint_records(
    document: &crate::model::ProjectDoc,
) -> Result<Vec<FramedConstraint>, String> {
    document
        .constraints
        .iter()
        .map(|constraint| crate::constraints::compile_legacy(document, constraint))
        .collect()
}

fn apply_constraint_lifecycle_delta(
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

fn prepare_lifecycle_output(
    before: &crate::model::ProjectDoc,
    mut output: super::MembershipTransitionOutput,
    effects: &[ConstraintLifecycleEffect],
) -> Result<super::PreparedLifecycleDraft, String> {
    let records = lifecycle_constraint_records(before)?;
    let delta = prepare_constraint_lifecycle_delta(&records, effects)?;
    apply_constraint_lifecycle_delta(&mut output.draft, delta);
    Ok(super::PreparedLifecycleDraft {
        draft: output.draft,
        changed_document_ids: output.changed_document_ids,
    })
}

fn lifecycle_context(
    document: &crate::model::ProjectDoc,
    pair_id: &str,
    matrix_pair: Option<super::MatrixPairContext>,
) -> Result<MembershipContext, String> {
    use super::{FinalOwner, TransitionAssociation};
    use crate::model::{MirroredExceptionState, MirroredItemKind};
    use std::collections::BTreeMap;

    let hardware = document
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

    fn parent_ids(
        halves: &crate::model::MirroredHalves,
        record: &crate::model::MirroredCorrespondence,
    ) -> (Option<String>, Option<String>) {
        if record.kind == MirroredItemKind::Part {
            for matrix in halves
                .correspondences
                .iter()
                .filter(|item| item.kind == MirroredItemKind::Matrix)
            {
                let Some(suffix) = record
                    .source_id
                    .strip_prefix(&format!("matrix/{}/", matrix.source_id))
                else {
                    continue;
                };
                let Some(mirror_suffix) = record
                    .mirror_id
                    .strip_prefix(&format!("matrix/{}/", matrix.mirror_id))
                else {
                    return (None, None);
                };
                if let Some((slot, _)) = suffix.split_once('/') {
                    let Some((mirror_slot, _)) = mirror_suffix.split_once('/') else {
                        return (None, None);
                    };
                    if slot == mirror_slot {
                        return (
                            Some(format!("matrix/{}/{}", matrix.source_id, slot)),
                            Some(format!("matrix/{}/{}", matrix.mirror_id, mirror_slot)),
                        );
                    }
                    return (None, None);
                }
                // A top-level key with an aligned slot inherits its matrix
                // pair's state (spec §5): the matrix correspondence is its
                // parent, so whole-matrix Local/Omitted transitions reach
                // every member without per-key exception records.
                if suffix == mirror_suffix {
                    return (
                        Some(matrix.source_id.clone()),
                        Some(matrix.mirror_id.clone()),
                    );
                }
                return (None, None);
            }
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
                    return (
                        Some(circuit.source_id.clone()),
                        Some(circuit.mirror_id.clone()),
                    );
                }
            }
        }
        (None, None)
    }

    let associations = halves
        .correspondences
        .iter()
        .map(|record| {
            let exception = halves.exceptions.iter().find(|exception| {
                exception.kind == record.kind && exception.mirror_id == record.mirror_id
            });
            let state = match exception.map(|exception| exception.state) {
                Some(MirroredExceptionState::Local) => TransitionState::Local,
                Some(MirroredExceptionState::Omitted) => TransitionState::Omitted,
                None => TransitionState::Linked,
            };
            let (parent_source_id, parent_mirror_id) = parent_ids(halves, record);
            TransitionAssociation {
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
            }
        })
        .collect();
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
        pending_pair_baseline: None,
    };
    for index in 0..context.associations.len() {
        let state = effective(&context, &context.associations[index], false, 0)?;
        context.associations[index].state = state;
    }
    for association in &context.associations {
        if association.kind != MirroredItemKind::Part {
            continue;
        }
        let source_owner = super::super::board_for_part(document, &association.source_id)
            .map(|board| board.id.clone())
            .ok_or_else(|| {
                format!(
                    "Lifecycle source Part {:?} has no board owner",
                    association.source_id
                )
            })?;
        let state = effective(&context, association, false, 0)?;
        context.owners.insert(
            association.source_id.clone(),
            FinalOwner::Board(source_owner),
        );
        let target_owner = if state == TransitionState::Omitted {
            FinalOwner::Absent
        } else {
            FinalOwner::Board(
                super::super::board_for_part(document, &association.mirror_id)
                    .map(|board| board.id.clone())
                    .ok_or_else(|| {
                        format!(
                            "Lifecycle mirror Part {:?} has no board owner",
                            association.mirror_id
                        )
                    })?,
            )
        };
        context
            .owners
            .insert(association.mirror_id.clone(), target_owner);
    }
    // The membership kernel validates ownership for every materialized Part in the document,
    // including independent Parts and associations filtered out of a selected-matrix view.
    // Bind those historical owners too; association-specific absent mirror IDs above keep their
    // stronger `Absent` value.
    for part in &document.parts {
        if context.owners.contains_key(&part.id) {
            continue;
        }
        let board_id = super::super::board_for_part(document, &part.id)
            .map(|board| board.id.clone())
            .ok_or_else(|| format!("Lifecycle Part {:?} has no board owner", part.id))?;
        context
            .owners
            .insert(part.id.clone(), FinalOwner::Board(board_id));
    }
    Ok(context)
}

fn matrix_pair_for_association(
    document: &crate::model::ProjectDoc,
    halves: &crate::model::MirroredHalves,
    kind: crate::model::MirroredItemKind,
    source_id: &str,
    mirror_id: &str,
) -> Result<Option<super::MatrixPairContext>, String> {
    if kind == MirroredItemKind::Matrix {
        return Ok(Some(super::MatrixPairContext {
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
        .filter(|record| record.kind == MirroredItemKind::Matrix)
        .filter_map(|record| {
            let (row, column, suffix) = super::matrix_identity_slot(source_id, &record.source_id)?;
            let mirror_member = if suffix.is_empty() {
                format!("matrix/{}/r{row}c{column}", record.mirror_id)
            } else {
                format!("matrix/{}/r{row}c{column}/{suffix}", record.mirror_id)
            };
            (mirror_member == mirror_id).then(|| super::MatrixPairContext {
                source_id: record.source_id.clone(),
                mirror_id: record.mirror_id.clone(),
            })
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

// Ticket 150's intent-to-draft route for exception edits. Only lifecycle tests call it.
#[cfg(test)]
fn prepare_exception_intent(
    before: &crate::model::ProjectDoc,
    post_source: &crate::model::ProjectDoc,
    pair_id: &str,
    kind: crate::model::MirroredItemKind,
    source_id: &str,
    mirror_id: &str,
    state: TransitionState,
) -> Result<Vec<super::MembershipTransitionOutput>, String> {
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
    let matrix_pair = matrix_pair_for_association(before, halves, kind, source_id, mirror_id)?;
    let context = lifecycle_context(before, pair_id, matrix_pair)?;
    let pending = transition_exception_context(&context, kind, source_id, mirror_id, state)?;
    prepare_lifecycle_transition(before, post_source, &pending).map(|output| vec![output])
}

fn prepare_matrix_membership_edit(
    valid_before: &crate::model::ProjectDoc,
    source_operation: &crate::model::ProjectDoc,
    operation: &crate::model::EditOperation,
    selected: &MembershipContext,
) -> Result<super::MembershipTransitionOutput, String> {
    use super::{
        BaselineKind, FinalOwner, MembershipTransitionInput, PlannedAddition, TransitionState,
        prepare_membership_transition,
    };
    use crate::model::{EditOperation as Op, MirroredItemKind};
    use std::collections::{BTreeMap, BTreeSet};

    let matrix_pair = super::require_matrix_pair(selected)?;

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

    fn filter_other_matrix_parts(
        document: &mut crate::model::ProjectDoc,
        context: &MembershipContext,
    ) -> Result<(), String> {
        let hardware = document
            .hardware
            .as_mut()
            .ok_or("Transition hardware is unavailable")?;
        let mut pairs = hardware
            .pairs
            .iter_mut()
            .filter(|pair| pair.id == context.pair_id);
        let pair = pairs
            .next()
            .ok_or("The exact transition pair is unavailable")?;
        if pairs.next().is_some() {
            return Err("The exact transition pair is ambiguous".into());
        }
        let halves = pair
            .halves
            .as_mut()
            .ok_or("The exact transition pair has no halves record")?;
        let matrix_pair = super::require_matrix_pair(context)?;
        halves.correspondences.retain(|association| {
            selected_matrix_association(
                association,
                matrix_pair.source_id.as_str(),
                matrix_pair.mirror_id.as_str(),
            )
        });
        halves.exceptions.retain(|exception| {
            selected_matrix_exception(exception, matrix_pair.mirror_id.as_str())
        });
        Ok(())
    }

    let mut filtered_before = valid_before.clone();
    let mut filtered_post_source = source_operation.clone();
    filter_other_matrix_parts(&mut filtered_before, selected)?;
    filter_other_matrix_parts(&mut filtered_post_source, selected)?;

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
        .cloned()
        .collect();
    fn remove_retired_layout_rows(
        document: &mut crate::model::ProjectDoc,
        pair_id: &str,
        retired: &[super::TransitionAssociation],
    ) -> Result<(), String> {
        if retired.is_empty() {
            return Ok(());
        }
        let pair = document
            .hardware
            .as_mut()
            .ok_or("Transition hardware is unavailable")?
            .pairs
            .iter_mut()
            .find(|pair| pair.id == pair_id)
            .ok_or("The exact transition pair is unavailable")?;
        let halves = pair
            .halves
            .as_mut()
            .ok_or("The exact transition pair has no halves record")?;
        halves.correspondences.retain(|record| {
            !retired.iter().any(|association| {
                record.kind == association.kind
                    && record.source_id == association.source_id
                    && record.mirror_id == association.mirror_id
            })
        });
        halves.exceptions.retain(|exception| {
            !retired.iter().any(|association| {
                exception.kind == association.kind && exception.mirror_id == association.mirror_id
            })
        });
        Ok(())
    }
    remove_retired_layout_rows(&mut filtered_before, &selected.pair_id, &retired_layouts)?;
    remove_retired_layout_rows(
        &mut filtered_post_source,
        &selected.pair_id,
        &retired_layouts,
    )?;
    let filtered_context = lifecycle_context(
        &filtered_before,
        &selected.pair_id,
        selected.matrix_pair.clone(),
    )?;
    let before = &filtered_before;
    let post_source = &filtered_post_source;
    let context = &filtered_context;

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
        matrix_pair: &super::MatrixPairContext,
        operation: &crate::model::EditOperation,
        post_source_ids: &BTreeSet<String>,
    ) -> Option<String> {
        let (mut row, mut column, child) =
            super::matrix_identity_slot(id, matrix_pair.source_id.as_str())?;
        match operation {
            Op::RemoveMatrix { id } if id == matrix_pair.source_id.as_str() => return None,
            Op::RemoveMatrixRow { row: removed, .. } => {
                if row == *removed {
                    return None;
                }
                if row > *removed {
                    row -= 1;
                }
            }
            Op::RemoveMatrixColumn {
                column: removed, ..
            } => {
                if column == *removed {
                    return None;
                }
                if column > *removed {
                    column -= 1;
                }
            }
            _ => {}
        }
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

    let source_identity_map: BTreeMap<_, _> = source_before
        .part_ids
        .iter()
        .map(|id| {
            (
                id.clone(),
                mapped_source_id(id, matrix_pair, operation, &source_after_ids),
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
            super::matrix_identity_slot(&source_id, matrix_pair.source_id.as_str())
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
            .filter_map(|(id, mapped)| mapped.is_none().then_some(id.clone())),
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
    let removed_source_ids: BTreeSet<_> = source_identity_map
        .iter()
        .filter_map(|(id, mapped)| mapped.is_none().then_some(id.clone()))
        .collect();
    let removed_mirror_ids: BTreeSet<_> = mirror_identity_map
        .iter()
        .filter_map(|(id, mapped)| mapped.is_none().then_some(id.clone()))
        .collect();
    let mut required_owners: BTreeSet<_> = before
        .parts
        .iter()
        .chain(post_source.parts.iter())
        .map(|part| part.id.clone())
        .collect();
    required_owners.extend(context.owners.keys().cloned());
    for addition in &planned_additions {
        required_owners.insert(addition.source_id.clone());
        required_owners.insert(addition.mirror_id.clone());
    }
    let mut final_owners = BTreeMap::new();
    for id in required_owners {
        let reused_target = retained_mirror_ids.contains(&id);
        let reused_source = source_identity_map
            .values()
            .flatten()
            .any(|mapped| mapped == &id)
            || planned_additions
                .iter()
                .any(|addition| addition.source_id == id);
        let owner = if retained_mirror_ids.contains(&id) {
            FinalOwner::Board(context.mirror_board_id.clone())
        } else if (removed_source_ids.contains(&id) && !reused_source)
            || (removed_mirror_ids.contains(&id) && !reused_target)
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

    let mut output = prepare_membership_transition(MembershipTransitionInput {
        valid_before: before,
        post_source,
        context,
        baseline_kind: BaselineKind::MaterializedCurrentSchema,
        source_identity_map: &source_identity_map,
        mirror_identity_map: &mirror_identity_map,
        planned_additions: &planned_additions,
        final_owners: &final_owners,
        explicit_removal_closure: &explicit_removal_closure,
    })?;

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
fn classify_source_retirements(
    before: &crate::model::ProjectDoc,
    post_source: &crate::model::ProjectDoc,
    context: &MembershipContext,
    include_unselected_matrices: bool,
) -> Result<Vec<super::TransitionAssociation>, String> {
    use crate::model::MirroredItemKind;

    let mut retired = Vec::new();
    for owner in context.associations.iter().filter(|association| {
        association.kind == MirroredItemKind::Matrix
            && (context.matrix_pair.as_ref().is_some_and(|matrix_pair| {
                association.source_id == matrix_pair.source_id
                    && association.mirror_id == matrix_pair.mirror_id
            }) || (include_unselected_matrices && context.matrix_pair.is_none()))
    }) {
        if post_source
            .matrices
            .iter()
            .any(|matrix| matrix.id == owner.source_id)
        {
            continue;
        }
        if context
            .pending_pair_transitions
            .contains_key(&owner.source_id)
        {
            return Err(
                "A selected lifecycle Matrix source was removed before its transition".into(),
            );
        }
        if before
            .matrices
            .iter()
            .filter(|matrix| matrix.id == owner.source_id)
            .flat_map(|matrix| matrix.part_ids.iter())
            .any(|id| {
                post_source.parts.iter().any(|part| part.id == *id)
                    || post_source
                        .boards
                        .iter()
                        .any(|board| board.part_ids.contains(id))
            })
        {
            return Err("Source Matrix removal has an incomplete ownership closure".into());
        }
        retired.push(owner.clone());
    }
    for association in context
        .associations
        .iter()
        .filter(|association| association.kind == MirroredItemKind::Part)
    {
        if post_source
            .parts
            .iter()
            .any(|part| part.id == association.source_id)
        {
            continue;
        }
        if context
            .pending_pair_transitions
            .contains_key(&association.source_id)
        {
            return Err("A selected lifecycle source was removed before its transition".into());
        }
        if post_source.boards.iter().any(|board| {
            board
                .part_ids
                .iter()
                .any(|part_id| part_id == &association.source_id)
        }) || post_source.matrices.iter().any(|matrix| {
            matrix
                .part_ids
                .iter()
                .any(|part_id| part_id == &association.source_id)
        }) || post_source.embedded_circuits.iter().any(|circuit| {
            circuit
                .part_ids
                .iter()
                .any(|part_id| part_id == &association.source_id)
        }) {
            return Err(format!(
                "Source Part removal for {:?} has an incomplete ownership closure",
                association.source_id
            ));
        }
        retired.push(association.clone());
    }
    for association in context.associations.iter().filter(|association| {
        matches!(
            association.kind,
            MirroredItemKind::EmbeddedCircuit | MirroredItemKind::Net | MirroredItemKind::Layout
        )
    }) {
        let present = match association.kind {
            MirroredItemKind::EmbeddedCircuit => post_source
                .embedded_circuits
                .iter()
                .any(|circuit| circuit.id == association.source_id),
            MirroredItemKind::Net => post_source
                .nets
                .iter()
                .any(|net| net.id == association.source_id),
            MirroredItemKind::Layout => post_source
                .layouts
                .iter()
                .any(|layout| layout.id == association.source_id),
            _ => unreachable!(),
        };
        if present {
            continue;
        }
        if context
            .pending_pair_transitions
            .contains_key(&association.source_id)
        {
            return Err("A selected lifecycle source was removed before its transition".into());
        }
        if association.kind == MirroredItemKind::EmbeddedCircuit
            && before
                .embedded_circuits
                .iter()
                .filter(|circuit| circuit.id == association.source_id)
                .flat_map(|circuit| circuit.part_ids.iter())
                .any(|id| {
                    post_source.parts.iter().any(|part| part.id == *id)
                        || post_source
                            .boards
                            .iter()
                            .any(|board| board.part_ids.contains(id))
                })
        {
            return Err("Source circuit removal has an incomplete ownership closure".into());
        }
        if association.kind == MirroredItemKind::Net
            && post_source
                .boards
                .iter()
                .any(|board| board.net_ids.contains(&association.source_id))
        {
            return Err("A removed source net retains a board registration".into());
        }
        retired.push(association.clone());
    }
    Ok(retired)
}

fn prepare_lifecycle_transition(
    before: &crate::model::ProjectDoc,
    post_source: &crate::model::ProjectDoc,
    context: &MembershipContext,
) -> Result<super::MembershipTransitionOutput, String> {
    use super::{
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
            crate::apply(
                &mut draft,
                &[],
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
        let mut projected = super::project_matrix(&source, &seed, &record, context.mapping)?;
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
            let (row, column, suffix) = super::matrix_identity_slot(id, &matrix_pair.source_id)
                .ok_or("A new owner Rejoin member is not canonical")?;
            let mirror_id = if suffix.is_empty() {
                format!("matrix/{}/r{row}c{column}", matrix_pair.mirror_id)
            } else {
                format!("matrix/{}/r{row}c{column}/{suffix}", matrix_pair.mirror_id)
            };
            if draft.parts.iter().filter(|p| p.id == mirror_id).count() != 1 {
                return Err("A new owner Rejoin member lacks its exact materialized target".into());
            }
            additions.push(super::TransitionAssociation {
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
                        reference: super::super::free_reference(
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
            let projected = super::project_part(
                source,
                &seed,
                &record,
                context.mapping,
                super::super::derive::is_controller(
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
    project_lifecycle_matrix(&mut draft, context, &additions)?;
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
    apply_membership_context_delta(
        MembershipTransitionInput {
            valid_before: before,
            post_source,
            context,
            baseline_kind: super::BaselineKind::MaterializedCurrentSchema,
            source_identity_map: &empty_map,
            mirror_identity_map: &empty_map,
            planned_additions: &[],
            final_owners: &context.owners,
            explicit_removal_closure: &empty_set,
        },
        &mut output,
    )?;

    let projection = if context.matrix_pair.is_none() {
        super::projection_for_matrixless_pair(&output.context.pair_right, context.mapping)
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
        super::Projection {
            matrix: Some(super::MatrixLink {
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
                .map(|a| super::PartProjection {
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
    super::reconcile(&mut output.draft, &context.pair_id, &projection)?;
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
    (output.changed_document_ids, output.changed_field_paths) =
        super::changed_document_fields(post_source, &output.draft)?;
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
pub(super) fn pending_visible_omission(context: &MembershipContext, mirror_id: &str) -> bool {
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
fn transition_exception_context(
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
                    super::FinalOwner::Board(next.mirror_board_id.clone())
                }
                TransitionState::Omitted => super::FinalOwner::Absent,
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

pub(super) fn apply_membership_context_delta(
    input: super::MembershipTransitionInput<'_>,
    output: &mut super::MembershipTransitionOutput,
) -> Result<(), String> {
    use crate::model::{MirroredCorrespondence, MirroredExceptionState, MirroredItemKind};

    if output.context_delta.added.is_empty()
        && output.context_delta.retired.is_empty()
        && output.context_delta.relocated.is_empty()
        && input.context.pending_rejoins.is_empty()
        && input.context.pending_pair_transitions.is_empty()
    {
        return Ok(());
    }

    let mut candidate = output.draft.clone();
    let hardware = candidate
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
                        association: &super::TransitionAssociation|
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
                          old: &super::TransitionAssociation,
                          new: &super::TransitionAssociation|
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
        if added.kind != MirroredItemKind::Part || added.state != super::TransitionState::Linked {
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
        let mut targets = candidate
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
    crate::mechanical::halves::validate_document(&candidate)
        .map_err(|error| format!("The lifecycle transaction candidate is invalid: {error}"))?;
    let (changed_document_ids, changed_field_paths) =
        super::changed_document_fields(input.post_source, &candidate)?;
    output.draft = candidate;
    output.context.pair_right = next_halves;
    output.changed_document_ids = changed_document_ids;
    output.changed_field_paths = changed_field_paths;
    output.context.pending_rejoins.clear();
    output.context.pending_pair_transitions.clear();
    output.context.pending_pair_baseline = None;
    Ok(())
}
