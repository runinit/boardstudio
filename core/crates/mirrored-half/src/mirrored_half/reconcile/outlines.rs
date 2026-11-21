//! Private outline inputs and projections for the forward kernel.

use super::Projection;
use crate::model::{
    MirroredExceptionState, MirroredItemKind, OutlineFeature, OutlineVersion, ProjectDoc,
};
use std::collections::{BTreeMap, BTreeSet};

/// Retire the pair identity after the canonical outline edit deleted its authored version.
pub(super) fn retire_version(
    draft: &mut ProjectDoc,
    edit: &crate::outline_edit::OutlineEdit,
    board_id: &str,
    version_id: &str,
) -> Result<Vec<String>, String> {
    let mut changed = Vec::new();
    let mut targets = Vec::new();
    for halves in draft
        .hardware
        .iter_mut()
        .flat_map(|h| &mut h.pairs)
        .filter_map(|p| p.halves.as_mut())
    {
        if halves.source_board_id == board_id {
            let Some(index) = halves.correspondences.iter().position(|row| {
                row.kind == MirroredItemKind::OutlineVersion && row.source_id == version_id
            }) else {
                continue;
            };
            let row = halves.correspondences.remove(index);
            if halves.overlay_of(&row).is_none() && halves.mirror_board_id != board_id {
                targets.push((halves.mirror_board_id.clone(), row.mirror_id.clone()));
            }
            halves
                .exceptions
                .retain(|e| !(e.kind == row.kind && e.mirror_id == row.mirror_id));
        } else if halves.mirror_board_id == board_id {
            // Inverse editing leaves a Local version on its own board. Deleting it
            // omits that target while the source and its identity remain live.
            if let Some(index) = halves.exceptions.iter().position(|e| {
                e.kind == MirroredItemKind::OutlineVersion && e.mirror_id == version_id
            }) {
                if halves
                    .overlaid_correspondence(&halves.exceptions[index])
                    .is_some()
                {
                    halves.exceptions[index].state = crate::model::MirroredExceptionState::Omitted;
                } else {
                    halves.exceptions.remove(index);
                }
            }
        }
    }
    for (board_id, version_id) in targets {
        if let Some(state) = draft
            .board_outlines
            .iter_mut()
            .find(|state| state.board_id == board_id)
        {
            state.generated_last_valid = None;
        }
        changed.extend(crate::outline_edit::apply(
            draft,
            &crate::outline_edit::OutlineEdit {
                target: crate::outline_edit::OutlineEditTarget::Version {
                    board_id,
                    version_id,
                },
                ..edit.clone()
            },
        )?);
    }
    Ok(changed)
}

fn mapped_identity(
    source_board: &str,
    mirror_board: &str,
    id: &str,
    associations: &BTreeMap<String, String>,
) -> String {
    associations
        .get(id)
        .cloned()
        .unwrap_or_else(|| crate::mirrored_half::mirror_id(source_board, mirror_board, id))
}

fn is_local(document: &ProjectDoc, mirror_id: &str, kind: MirroredItemKind) -> bool {
    document
        .hardware
        .as_ref()
        .into_iter()
        .flat_map(|hardware| &hardware.pairs)
        .filter_map(|pair| pair.halves.as_ref())
        .flat_map(|halves| &halves.exceptions)
        .any(|exception| {
            exception.kind == kind
                && exception.mirror_id == mirror_id
                && exception.state == MirroredExceptionState::Local
        })
}

fn target_part_is_independent(document: &ProjectDoc, mirror_board_id: &str, part_id: &str) -> bool {
    if part_id.starts_with("case-closure/") || part_id.starts_with("embedded/") {
        return false;
    }
    if !document.parts.iter().any(|part| part.id == part_id) {
        return false;
    }
    if document
        .parts
        .iter()
        .filter(|candidate| candidate.id == part_id)
        .count()
        != 1
        || super::unique_board_owner(document, part_id, "outline member").ok()
            != Some(mirror_board_id)
    {
        return false;
    }
    if document
        .matrices
        .iter()
        .any(|matrix| matrix.part_ids.iter().any(|id| id == part_id))
        || document
            .embedded_circuits
            .iter()
            .any(|circuit| circuit.part_ids.iter().any(|id| id == part_id))
        || document.modules.iter().any(|module| {
            module
                .connection
                .as_ref()
                .is_some_and(|connection| connection.host_connector_part_id == part_id)
        })
    {
        return false;
    }
    let associations: Vec<_> = document
        .hardware
        .as_ref()
        .into_iter()
        .flat_map(|hardware| &hardware.pairs)
        .filter_map(|pair| pair.halves.as_ref())
        .flat_map(|halves| &halves.correspondences)
        .filter(|association| {
            association.kind == MirroredItemKind::Part && association.mirror_id == part_id
        })
        .collect();
    associations.is_empty()
        || (associations.len() == 1 && is_local(document, part_id, MirroredItemKind::Part))
}

fn retain_target_envelope_members(
    document: &mut ProjectDoc,
    mirror_board_id: &str,
    prior_members: &BTreeMap<String, Vec<String>>,
    changed: &mut BTreeSet<String>,
) {
    let eligible: BTreeSet<_> = prior_members
        .values()
        .flatten()
        .filter(|part_id| target_part_is_independent(document, mirror_board_id, part_id))
        .cloned()
        .collect();
    for feature in &mut document.outline {
        let OutlineFeature::PartEnvelope { id, part_ids, .. } = feature else {
            continue;
        };
        let Some(prior) = prior_members.get(id) else {
            continue;
        };
        let mut seen: BTreeSet<_> = part_ids.iter().cloned().collect();
        let before = part_ids.len();
        for part_id in prior {
            if eligible.contains(part_id) && seen.insert(part_id.clone()) {
                part_ids.push(part_id.clone());
            }
        }
        if part_ids.len() != before {
            changed.insert(id.clone());
        }
    }
}

/// Project the source board's outlines onto the caller's private reconciliation draft.
pub(super) fn reconcile_draft(
    draft: &mut ProjectDoc,
    pair_id: &str,
    projection: &Projection,
) -> Result<(), String> {
    let pair = super::selected_pair(draft, pair_id, "outline reconciliation")?;
    let halves = pair.halves.as_ref().ok_or_else(|| {
        format!("The outline reconciliation pair {pair_id:?} has no halves context")
    })?;
    if halves.source_board_id != projection.source_board_id
        || halves.mirror_board_id != projection.mirror_board_id
        || pair.mapping != projection.mapping
    {
        return Err("The outline reconciliation context does not match its pair".into());
    }

    let targets = |kind| {
        halves
            .correspondences
            .iter()
            .filter(|association| association.kind == kind)
            .map(|association| (association.source_id.clone(), association.mirror_id.clone()))
            .collect::<BTreeMap<_, _>>()
    };
    let outline_map = targets(MirroredItemKind::Outline);
    let version_map = targets(MirroredItemKind::OutlineVersion);
    let part_map = targets(MirroredItemKind::Part);
    let matrix_map = targets(MirroredItemKind::Matrix);
    let target_outline_ids: BTreeSet<_> = draft
        .boards
        .iter()
        .find(|board| board.id == projection.mirror_board_id)
        .ok_or_else(|| "The selected target board is unavailable".to_owned())?
        .outline_ids
        .iter()
        .cloned()
        .collect();
    let prior_target_envelope_members: BTreeMap<_, _> = draft
        .outline
        .iter()
        .filter_map(|feature| match feature {
            OutlineFeature::PartEnvelope { id, part_ids, .. }
                if target_outline_ids.contains(id) =>
            {
                Some((id.clone(), part_ids.clone()))
            }
            _ => None,
        })
        .collect();
    let source_board = draft
        .boards
        .iter()
        .find(|board| board.id == projection.source_board_id)
        .ok_or_else(|| "The selected source board is unavailable".to_owned())?;
    let matrix_owned_parts = super::super::matrix_owned_parts(draft, &source_board.id);
    let controller_parts = super::super::controller_parts(draft, &source_board.id);
    let renames = super::super::Renames {
        identity: &|id| {
            mapped_identity(
                &projection.source_board_id,
                &projection.mirror_board_id,
                id,
                &outline_map,
            )
        },
        part: &|id| part_map.get(id).cloned().unwrap_or_else(|| id.to_owned()),
        full_rule_part: &|id| !matrix_owned_parts.contains(id) && !controller_parts.contains(id),
    };
    for outline_id in &source_board.outline_ids {
        if !outline_map.contains_key(outline_id) {
            return Err(format!(
                "Source outline {outline_id:?} has no selected mirror correspondence"
            ));
        }
    }
    if let Some(source_outline) = draft
        .board_outlines
        .iter()
        .find(|outline| outline.board_id == projection.source_board_id)
    {
        for version in &source_outline.versions {
            if !version_map.contains_key(&version.id) {
                return Err(format!(
                    "Source outline version {:?} has no selected mirror correspondence",
                    version.id
                ));
            }
        }
    }

    let mut changed = BTreeSet::new();
    let outline_correspondences: Vec<_> = halves
        .correspondences
        .iter()
        .filter(|association| association.kind == MirroredItemKind::Outline)
        .cloned()
        .collect();
    for association in outline_correspondences {
        if is_local(draft, &association.mirror_id, MirroredItemKind::Outline) {
            continue;
        }
        let source = draft
            .outline
            .iter()
            .find(|feature| feature.id() == association.source_id)
            .ok_or_else(|| {
                format!(
                    "The selected source outline {:?} is unavailable",
                    association.source_id
                )
            })?;
        let projected = super::super::mirror_feature(source, projection.mapping, &renames);
        let target = draft
            .outline
            .iter_mut()
            .find(|feature| feature.id() == association.mirror_id)
            .ok_or_else(|| {
                format!(
                    "The selected target outline {:?} is unavailable",
                    association.mirror_id
                )
            })?;
        if *target != projected {
            *target = projected;
            changed.insert(association.mirror_id);
        }
    }
    retain_target_envelope_members(
        draft,
        &projection.mirror_board_id,
        &prior_target_envelope_members,
        &mut changed,
    );

    if version_map.is_empty() {
        return Ok(());
    }

    let source_outlines: Vec<_> = draft
        .board_outlines
        .iter()
        .filter(|outline| outline.board_id == projection.source_board_id)
        .cloned()
        .collect();
    let target_index = draft
        .board_outlines
        .iter()
        .position(|outline| outline.board_id == projection.mirror_board_id)
        .ok_or_else(|| "The selected target board outline is unavailable".to_owned())?;
    if let Some(source_outline) = source_outlines.first() {
        let mut target_outline = draft.board_outlines[target_index].clone();
        for source_version in &source_outline.versions {
            let Some(target_id) = version_map.get(&source_version.id) else {
                continue;
            };
            if is_local(draft, target_id, MirroredItemKind::OutlineVersion) {
                continue;
            }
            let projected = OutlineVersion {
                id: target_id.clone(),
                name: source_version.name.clone(),
                source: crate::model::OutlineProvenance {
                    revision: source_version.source.revision,
                    version_id: source_version.source.version_id.as_ref().map(|id| {
                        mapped_identity(
                            &projection.source_board_id,
                            &projection.mirror_board_id,
                            id,
                            &version_map,
                        )
                    }),
                },
                geometry: super::super::mirror_snapshot(
                    &source_version.geometry,
                    projection.mapping,
                    &renames,
                    &matrix_map,
                ),
                ownership: source_version.ownership,
            };
            if let Some(existing) = target_outline
                .versions
                .iter_mut()
                .find(|version| version.id == *target_id)
                && *existing != projected
            {
                *existing = projected;
                changed.insert(target_id.clone());
            }
        }
        if let Some(source_active) = &source_outline.active_version_id {
            if let Some(target_active) = version_map.get(source_active)
                && target_outline
                    .versions
                    .iter()
                    .any(|v| v.id == *target_active)
                && target_outline.active_version_id.as_ref() != Some(target_active)
            {
                target_outline.active_version_id = Some(target_active.clone());
                changed.insert(target_active.clone());
            }
        } else if let Some(target_active) = target_outline.active_version_id.clone()
            && version_map
                .values()
                .any(|target_id| target_id == &target_active)
            && !is_local(draft, &target_active, MirroredItemKind::OutlineVersion)
        {
            target_outline.active_version_id = None;
            changed.insert(target_active);
        }
        if changed
            .iter()
            .any(|id| version_map.values().any(|target_id| target_id == id))
        {
            target_outline.generated_last_valid = None;
        }
        if draft.board_outlines[target_index] != target_outline {
            draft.board_outlines[target_index] = target_outline;
        }
    }

    Ok(())
}
