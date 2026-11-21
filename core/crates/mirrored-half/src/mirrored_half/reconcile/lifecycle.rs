//! Private lifecycle operations for durable mirrored-half exceptions and Unlink.

use super::{MembershipContext, TransitionState};
use crate::constraints::{ConstraintEndpoint, FramedConstraint};
use crate::model::MirroredExceptionState;
use std::collections::BTreeSet;

mod context;
mod copper;
mod edit_transition;
mod membership_edit;
mod transition;

pub use context::validate_lifecycle_context;
pub(super) use copper::adopt_rejoined_anchor;
use copper::{
    capture_omitted_copper, copy_part_payload, restore_rejoined_nets, restore_saved_copper,
};
pub use edit_transition::retire_replacement_records;
pub(super) use edit_transition::{native_detach_proposal, prepare_edit_transition};
use membership_edit::{classify_source_retirements, prepare_matrix_membership_edit};
pub use membership_edit::{lifecycle_context, matrix_pair_for_association};
pub(super) use transition::{apply_membership_context_delta, pending_visible_omission};
pub use transition::{prepare_lifecycle_transition, transition_exception_context};

#[derive(Clone, Debug, PartialEq, Eq)]
#[doc(hidden)]
pub enum ConstraintLifecycleEffect {
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
#[doc(hidden)]
pub struct ConstraintLifecycleDelta {
    #[doc(hidden)]
    pub upserts: Vec<FramedConstraint>,
    #[doc(hidden)]
    pub removals: Vec<String>,
}

/// Prepares endpoint edits after the owning lifecycle kernel has validated the exact transition
/// and removal closure. Records are compiled against the pre-transition document; removed IDs
/// are visible Part IDs from that same validated closure, never canonical aliases.
#[doc(hidden)]
pub fn prepare_constraint_lifecycle_delta(
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

// Ticket 150's intent-to-draft route for exception edits. Only lifecycle tests call it.
