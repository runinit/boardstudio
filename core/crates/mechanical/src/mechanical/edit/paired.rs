//! Shared paired-feature edit and persistence helpers.
//!
//! Linked-feature propagation, pair-owned writeback and the generated-closure
//! projection tail are shared by the ownership, lifecycle, replacement and
//! settings paths, so every path routes each changed value to the ownership
//! that owns it through one implementation.

use super::super::features;
use super::closures::{dedup_changed, project, validate_mounting_hole};
use super::feature_lifecycle::{
    feature_ids, find_feature_value, pair_at, pair_mut_at, pair_owner_ids, remove_feature,
};
use crate::model::*;

/// Propagates one edited effective configuration into its pair's ownership.
///
/// A linked feature belongs to its pair, so a change made from either member
/// updates the pair's single authored value (stored in the editing member's
/// frame) and never a second writable copy on the member. An ordinary delete of
/// a linked feature is the linked deletion: it removes both counterparts and the
/// relationship itself, while an explicit omit keeps a one-half exception. A
/// `Pair` member's shared construction change is written to the pair. Every
/// other field stays in the member's local configuration, which the caller
/// persists. This is the pair-owned half of [`super::ownership::persist_edit`].
pub(super) fn persist_paired_feature_edit(
    doc: &mut ProjectDoc,
    instance_id: &str,
    pair_index: usize,
    before: &MechanicalConfiguration,
    after: &MechanicalConfiguration,
    local: &mut MechanicalConfiguration,
    changed: &mut Vec<String>,
) -> Result<(), String> {
    // A change to a linked feature or to the pair's shared construction
    // changes the derived geometry both halves show, so the report names the
    // pair and both member instances.
    let mut pair_touched = false;
    let pair = pair_at(doc, pair_index)?.clone();
    for feature in &pair.features {
        let Some(member) = feature
            .members
            .iter()
            .find(|member| member.instance_id == *instance_id)
        else {
            continue;
        };
        if member.state == MechanicalFeatureState::Linked {
            let after_value = find_feature_value(after, feature.kind, &member.feature_id);
            let before_value = find_feature_value(before, feature.kind, &member.feature_id);
            if after_value != before_value {
                pair_touched = true;
                let pair = pair_mut_at(doc, pair_index)?;
                match after_value {
                    Some(value) => {
                        // The editing member's frame authors the pair's
                        // single value; the other half derives from it.
                        let target = pair
                            .features
                            .iter_mut()
                            .find(|entry| entry.id == feature.id)
                            .ok_or("The mechanical feature is no longer in its pair.")?;
                        target.value = value;
                        target.author = instance_id.to_string();
                    }
                    None => {
                        // Ordinary delete of a linked feature removes both
                        // counterparts: the relationship itself goes. An
                        // explicit omit is what keeps a one-half exception.
                        features::remove_linked_feature(pair, &feature.id)?;
                    }
                }
            }
        }
        // A linked feature is never persisted on the member; a `Local`
        // exception stays in its ordinary configuration.
        if member.state != MechanicalFeatureState::Local {
            remove_feature(local, feature.kind, &member.feature_id);
        }
    }
    // Generic additions on a paired half join the pair by default, so both
    // halves show the new feature and neither keeps a private copy. Only
    // explicit exceptions (Make local) stay local.
    for kind in [
        MechanicalFeatureKind::Mount,
        MechanicalFeatureKind::ClosureMount,
        MechanicalFeatureKind::Opening,
        MechanicalFeatureKind::GasketSupport,
    ] {
        let existing = feature_ids(before, kind);
        let added: Vec<String> = feature_ids(after, kind)
            .into_iter()
            .filter(|id| !existing.contains(id))
            .collect();
        for id in added {
            let Some(value) = find_feature_value(after, kind, &id) else {
                continue;
            };
            let pair = pair_mut_at(doc, pair_index)?;
            if pair.features.iter().any(|feature| feature.id == id) {
                continue;
            }
            pair_touched = true;
            features::add_linked_feature(
                pair,
                MechanicalFeature {
                    id: id.clone(),
                    kind,
                    author: instance_id.to_string(),
                    value,
                    members: Vec::new(),
                },
            )?;
            remove_feature(local, kind, &id);
        }
    }
    let before_shared = features::shared_construction(before);
    let after_shared = features::shared_construction(after);
    if before_shared != after_shared {
        pair_touched = true;
        pair_mut_at(doc, pair_index)?.construction = after_shared;
    }
    if pair_touched {
        changed.extend(pair_owner_ids(doc, pair_index)?);
    }
    Ok(())
}

/// Rejects a malformed supplied mounting-hole template before anything mutates.
///
/// Settings-only edits validate the template even when the projection never
/// runs, so a malformed catalogue entry is always an error.
pub(super) fn validate_supplied_template(
    mounting_hole: Option<&PartDefinition>,
) -> Result<(), String> {
    if let Some(template) = mounting_hole {
        validate_mounting_hole(template)?;
    }
    Ok(())
}

/// Runs the owned projection and records only the generated ids it changed.
///
/// Generated closure holes and memberships refresh in the same atomic edit, and
/// the reported ids stay deduplicated against the ids the edit already named.
pub(super) fn project_changed(
    doc: &mut ProjectDoc,
    mounting_hole: Option<&PartDefinition>,
    changed: &mut Vec<String>,
) -> Result<(), String> {
    project(doc, mounting_hole, changed)?;
    dedup_changed(changed);
    Ok(())
}
