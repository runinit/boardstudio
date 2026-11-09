//! Private outline inputs and projections for the forward kernel.

use super::Projection;
use crate::model::{
    MirroredExceptionState, MirroredItemKind, OutlineFeature, OutlineVersion, ProjectDoc,
};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, PartialEq, Eq)]
struct RetiredOutlineItem {
    kind: MirroredItemKind,
    source_id: String,
    mirror_id: String,
}

fn pair_bytes(document: &ProjectDoc, pair_id: &str) -> Result<Vec<u8>, String> {
    let hardware = document
        .hardware
        .as_ref()
        .ok_or_else(|| "The outline reconciliation hardware is unavailable".to_owned())?;
    let mut pairs = hardware.pairs.iter().filter(|pair| pair.id == pair_id);
    let pair = pairs
        .next()
        .ok_or_else(|| format!("The outline reconciliation pair {pair_id:?} is unavailable"))?;
    if pairs.next().is_some() {
        return Err(format!(
            "The outline reconciliation pair {pair_id:?} is ambiguous"
        ));
    }
    serde_json::to_vec(pair).map_err(|error| error.to_string())
}

fn removal_associations(
    document: &ProjectDoc,
    pair_id: &str,
    projection: &Projection,
) -> Result<Vec<RetiredOutlineItem>, String> {
    let mut retired = Vec::new();
    for ((context_pair, source_id, mirror_id), baseline) in
        projection.outline_removal_baselines.borrow().iter()
    {
        if context_pair != pair_id {
            continue;
        }
        crate::mechanical::halves::validate_document(baseline).map_err(|error| {
            format!("Outline removal {source_id:?} has an invalid pre-source baseline: {error}")
        })?;
        if pair_bytes(baseline, pair_id)? != pair_bytes(document, pair_id)? {
            return Err(format!(
                "Outline removal {source_id:?} changed durable pair bytes before reconciliation"
            ));
        }
        let baseline_hardware = baseline
            .hardware
            .as_ref()
            .ok_or_else(|| "The outline removal baseline hardware is unavailable".to_owned())?;
        let baseline_pair = baseline_hardware
            .pairs
            .iter()
            .find(|pair| pair.id == pair_id)
            .ok_or_else(|| {
                format!("The outline removal baseline pair {pair_id:?} is unavailable")
            })?;
        let halves = baseline_pair.halves.as_ref().ok_or_else(|| {
            format!("The outline removal baseline pair {pair_id:?} has no halves context")
        })?;
        if halves.source_board_id != projection.source_board_id
            || halves.mirror_board_id != projection.mirror_board_id
            || baseline_pair.mapping != projection.mapping
        {
            return Err(format!(
                "Outline removal {source_id:?} belongs to a different source, target or mapping"
            ));
        }
        let associations: Vec<_> = halves
            .correspondences
            .iter()
            .filter(|association| {
                matches!(
                    association.kind,
                    MirroredItemKind::Outline | MirroredItemKind::OutlineVersion
                ) && association.source_id == *source_id
                    && association.mirror_id == *mirror_id
            })
            .collect();
        if associations.len() != 1 {
            return Err(format!(
                "Outline removal {source_id:?} → {mirror_id:?} lacks one exact baseline correspondence"
            ));
        }
        let kind = associations[0].kind;
        match kind {
            MirroredItemKind::OutlineVersion => {
                let source_outline = baseline
                    .board_outlines
                    .iter()
                    .find(|outline| outline.board_id == halves.source_board_id)
                    .ok_or_else(|| {
                        "The outline removal source board has no outline state".to_owned()
                    })?;
                let target_outline = baseline
                    .board_outlines
                    .iter()
                    .find(|outline| outline.board_id == halves.mirror_board_id)
                    .ok_or_else(|| {
                        "The outline removal target board has no outline state".to_owned()
                    })?;
                if !source_outline
                    .versions
                    .iter()
                    .any(|version| version.id == *source_id)
                    || (!target_outline
                        .versions
                        .iter()
                        .any(|version| version.id == *mirror_id)
                        && !target_outline.retired_version_ids.contains(mirror_id))
                {
                    return Err(format!(
                        "Outline removal {source_id:?} → {mirror_id:?} was not present on the selected source and target boards"
                    ));
                }
                let current_source_has_item = document
                    .board_outlines
                    .iter()
                    .find(|outline| outline.board_id == projection.source_board_id)
                    .is_some_and(|outline| {
                        outline
                            .versions
                            .iter()
                            .any(|version| version.id == *source_id)
                    });
                if current_source_has_item {
                    return Err(format!(
                        "Outline removal evidence for {source_id:?} does not match a removed source version"
                    ));
                }
            }
            MirroredItemKind::Outline => {
                let source_board = baseline
                    .boards
                    .iter()
                    .find(|board| board.id == halves.source_board_id)
                    .ok_or_else(|| "The outline removal source board is unavailable".to_owned())?;
                let target_board = baseline
                    .boards
                    .iter()
                    .find(|board| board.id == halves.mirror_board_id)
                    .ok_or_else(|| "The outline removal target board is unavailable".to_owned())?;
                if !source_board.outline_ids.contains(source_id)
                    || !target_board.outline_ids.contains(mirror_id)
                    || !baseline
                        .outline
                        .iter()
                        .any(|feature| feature.id() == *source_id)
                    || !baseline
                        .outline
                        .iter()
                        .any(|feature| feature.id() == *mirror_id)
                {
                    return Err(format!(
                        "Outline removal {source_id:?} → {mirror_id:?} was not present on the selected source and target boards"
                    ));
                }
                let current_source_has_item = document
                    .boards
                    .iter()
                    .find(|board| board.id == projection.source_board_id)
                    .is_some_and(|board| board.outline_ids.contains(source_id))
                    || document
                        .outline
                        .iter()
                        .any(|feature| feature.id() == *source_id);
                if current_source_has_item {
                    return Err(format!(
                        "Outline removal evidence for {source_id:?} does not match a removed source outline"
                    ));
                }
            }
            _ => unreachable!("the association filter only selects outline kinds"),
        }
        if retired.iter().any(|item: &RetiredOutlineItem| {
            item.kind == kind && item.source_id == *source_id && item.mirror_id == *mirror_id
        }) {
            return Err(format!(
                "Outline removal {source_id:?} is recorded more than once"
            ));
        }
        retired.push(RetiredOutlineItem {
            kind,
            source_id: source_id.clone(),
            mirror_id: mirror_id.clone(),
        });
    }
    Ok(retired)
}

/// Remove only authenticated retired-outline associations from a validation copy.
pub(super) fn prepare_validation(
    document: &mut ProjectDoc,
    pair_id: &str,
    projection: &Projection,
) -> Result<(), String> {
    let retired = removal_associations(document, pair_id, projection)?;
    if retired.is_empty() {
        return Ok(());
    }
    let pair = document
        .hardware
        .as_mut()
        .and_then(|hardware| hardware.pairs.iter_mut().find(|pair| pair.id == pair_id))
        .ok_or_else(|| format!("The outline validation pair {pair_id:?} is unavailable"))?;
    let halves = pair
        .halves
        .as_mut()
        .ok_or_else(|| format!("The outline validation pair {pair_id:?} has no halves context"))?;
    for item in retired {
        let before = halves.correspondences.len();
        halves.correspondences.retain(|association| {
            !(association.kind == item.kind
                && association.source_id == item.source_id
                && association.mirror_id == item.mirror_id)
        });
        if before - halves.correspondences.len() != 1 {
            return Err(format!(
                "Outline removal {:?} → {:?} is not the exact selected association",
                item.source_id, item.mirror_id
            ));
        }
    }
    Ok(())
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

fn map_snapshot(
    snapshot: &crate::model::OutlineSnapshot,
    projection: &Projection,
    associations: &BTreeMap<String, String>,
    part_associations: &BTreeMap<String, String>,
    matrix_associations: &BTreeMap<String, String>,
    matrix_owned_parts: &BTreeSet<String>,
    controller_parts: &BTreeSet<String>,
) -> crate::model::OutlineSnapshot {
    let identity = |id: &str| {
        mapped_identity(
            &projection.source_board_id,
            &projection.mirror_board_id,
            id,
            associations,
        )
    };
    let part = |id: &str| {
        part_associations
            .get(id)
            .cloned()
            .unwrap_or_else(|| id.to_owned())
    };
    let full_rule_part =
        |id: &str| !matrix_owned_parts.contains(id) && !controller_parts.contains(id);
    let renames = super::super::Renames {
        identity: &identity,
        part: &part,
        full_rule_part: &full_rule_part,
    };
    let matrix_map: BTreeMap<_, _> = matrix_associations
        .iter()
        .map(|(source, target)| (source.clone(), target.clone()))
        .collect();
    super::super::mirror_snapshot(snapshot, projection.mapping, &renames, &matrix_map)
}

#[allow(clippy::too_many_arguments)] // The two part-rule sets travel with the map they qualify.
fn map_feature(
    feature: &OutlineFeature,
    mapping: crate::model::Pose2,
    source_board: &str,
    mirror_board: &str,
    associations: &BTreeMap<String, String>,
    part_associations: &BTreeMap<String, String>,
    matrix_owned_parts: &BTreeSet<String>,
    controller_parts: &BTreeSet<String>,
) -> OutlineFeature {
    let identity = |id: &str| mapped_identity(source_board, mirror_board, id, associations);
    let part = |id: &str| {
        part_associations
            .get(id)
            .cloned()
            .unwrap_or_else(|| id.to_owned())
    };
    let full_rule_part =
        |id: &str| !matrix_owned_parts.contains(id) && !controller_parts.contains(id);
    let renames = super::super::Renames {
        identity: &identity,
        part: &part,
        full_rule_part: &full_rule_part,
    };
    super::super::mirror_feature(feature, mapping, &renames)
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

/// Project the source board's established outline correspondences into a candidate document.
pub(super) fn reconcile(
    document: &mut ProjectDoc,
    pair_id: &str,
    projection: &Projection,
) -> Result<Vec<String>, String> {
    let hardware = document
        .hardware
        .as_ref()
        .ok_or_else(|| "The outline reconciliation hardware is unavailable".to_owned())?;
    let mut pair_matches = hardware.pairs.iter().filter(|pair| pair.id == pair_id);
    let pair = pair_matches
        .next()
        .ok_or_else(|| format!("The outline reconciliation pair {pair_id:?} is unavailable"))?;
    if pair_matches.next().is_some() {
        return Err(format!(
            "The outline reconciliation pair {pair_id:?} is ambiguous"
        ));
    }
    let halves = pair.halves.as_ref().ok_or_else(|| {
        format!("The outline reconciliation pair {pair_id:?} has no halves context")
    })?;
    if halves.source_board_id != projection.source_board_id
        || halves.mirror_board_id != projection.mirror_board_id
        || pair.mapping != projection.mapping
    {
        return Err("The outline reconciliation context does not match its pair".into());
    }
    let pair_bytes_before = pair_bytes(document, pair_id)?;
    let retired = removal_associations(document, pair_id, projection)?;

    let outline_map: BTreeMap<_, _> = halves
        .correspondences
        .iter()
        .filter(|association| association.kind == MirroredItemKind::Outline)
        .map(|association| (association.source_id.clone(), association.mirror_id.clone()))
        .collect();
    let version_map: BTreeMap<_, _> = halves
        .correspondences
        .iter()
        .filter(|association| association.kind == MirroredItemKind::OutlineVersion)
        .map(|association| (association.source_id.clone(), association.mirror_id.clone()))
        .collect();
    let part_map: BTreeMap<_, _> = halves
        .correspondences
        .iter()
        .filter(|association| association.kind == MirroredItemKind::Part)
        .map(|association| (association.source_id.clone(), association.mirror_id.clone()))
        .collect();
    let matrix_map: BTreeMap<_, _> = halves
        .correspondences
        .iter()
        .filter(|association| association.kind == MirroredItemKind::Matrix)
        .map(|association| (association.source_id.clone(), association.mirror_id.clone()))
        .collect();
    let target_outline_ids: BTreeSet<_> = document
        .boards
        .iter()
        .find(|board| board.id == projection.mirror_board_id)
        .ok_or_else(|| "The selected target board is unavailable".to_owned())?
        .outline_ids
        .iter()
        .cloned()
        .collect();
    let prior_target_envelope_members: BTreeMap<_, _> = document
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
    let retired_outline_ids: BTreeSet<_> = retired
        .iter()
        .filter(|item| item.kind == MirroredItemKind::Outline)
        .map(|item| item.source_id.clone())
        .collect();
    let retired_version_ids: BTreeSet<_> = retired
        .iter()
        .filter(|item| item.kind == MirroredItemKind::OutlineVersion)
        .map(|item| item.source_id.clone())
        .collect();
    let source_board = document
        .boards
        .iter()
        .find(|board| board.id == projection.source_board_id)
        .ok_or_else(|| "The selected source board is unavailable".to_owned())?;
    let matrix_owned_parts = super::super::matrix_owned_parts(document, &source_board.id);
    let controller_parts = super::super::controller_parts(document, &source_board.id);
    for outline_id in &source_board.outline_ids {
        if !outline_map.contains_key(outline_id) && !retired_outline_ids.contains(outline_id) {
            return Err(format!(
                "Source outline {outline_id:?} has no selected mirror correspondence"
            ));
        }
    }
    if let Some(source_outline) = document
        .board_outlines
        .iter()
        .find(|outline| outline.board_id == projection.source_board_id)
    {
        for version in &source_outline.versions {
            if !version_map.contains_key(&version.id) && !retired_version_ids.contains(&version.id)
            {
                return Err(format!(
                    "Source outline version {:?} has no selected mirror correspondence",
                    version.id
                ));
            }
        }
    }

    let mut staged = document.clone();
    let mut changed = BTreeSet::new();
    let outline_correspondences: Vec<_> = halves
        .correspondences
        .iter()
        .filter(|association| association.kind == MirroredItemKind::Outline)
        .cloned()
        .collect();
    for association in outline_correspondences {
        if retired_outline_ids.contains(&association.source_id) {
            continue;
        }
        if is_local(&staged, &association.mirror_id, MirroredItemKind::Outline) {
            continue;
        }
        let source = staged
            .outline
            .iter()
            .find(|feature| feature.id() == association.source_id)
            .ok_or_else(|| {
                format!(
                    "The selected source outline {:?} is unavailable",
                    association.source_id
                )
            })?;
        let projected = map_feature(
            source,
            projection.mapping,
            &projection.source_board_id,
            &projection.mirror_board_id,
            &outline_map,
            &part_map,
            &matrix_owned_parts,
            &controller_parts,
        );
        let target = staged
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
        &mut staged,
        &projection.mirror_board_id,
        &prior_target_envelope_members,
        &mut changed,
    );

    if !retired_outline_ids.is_empty() {
        let target_board = staged
            .boards
            .iter_mut()
            .find(|board| board.id == projection.mirror_board_id)
            .ok_or_else(|| "The selected target board is unavailable".to_owned())?;
        for item in retired
            .iter()
            .filter(|item| item.kind == MirroredItemKind::Outline)
        {
            let had_feature = staged
                .outline
                .iter()
                .any(|feature| feature.id() == item.mirror_id);
            let had_reference = target_board
                .outline_ids
                .iter()
                .any(|id| id == &item.mirror_id);
            staged
                .outline
                .retain(|feature| feature.id() != item.mirror_id);
            target_board.outline_ids.retain(|id| id != &item.mirror_id);
            if had_feature || had_reference {
                changed.insert(item.mirror_id.clone());
            }
        }
    }

    let version_work = !version_map.is_empty()
        || retired
            .iter()
            .any(|item| item.kind == MirroredItemKind::OutlineVersion);
    if !version_work {
        if pair_bytes(&staged, pair_id)? != pair_bytes_before {
            return Err("Outline reconciliation changed durable pair bytes".into());
        }
        let changed = changed.into_iter().collect();
        *document = staged;
        return Ok(changed);
    }

    let source_outlines: Vec<_> = staged
        .board_outlines
        .iter()
        .filter(|outline| outline.board_id == projection.source_board_id)
        .cloned()
        .collect();
    let target_index = staged
        .board_outlines
        .iter()
        .position(|outline| outline.board_id == projection.mirror_board_id)
        .ok_or_else(|| "The selected target board outline is unavailable".to_owned())?;
    if let Some(source_outline) = source_outlines.first() {
        let mut target_outline = staged.board_outlines[target_index].clone();
        for source_version in &source_outline.versions {
            let Some(target_id) = version_map.get(&source_version.id) else {
                continue;
            };
            if is_local(&staged, target_id, MirroredItemKind::OutlineVersion) {
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
                geometry: map_snapshot(
                    &source_version.geometry,
                    projection,
                    &outline_map,
                    &part_map,
                    &matrix_map,
                    &matrix_owned_parts,
                    &controller_parts,
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
        for item in retired
            .iter()
            .filter(|item| item.kind == MirroredItemKind::OutlineVersion)
        {
            let target_id = &item.mirror_id;
            let mut did_change = false;
            if let Some(target) = target_outline
                .versions
                .iter()
                .position(|version| version.id == *target_id)
            {
                target_outline.versions.remove(target);
                did_change = true;
            } else if !target_outline.retired_version_ids.contains(target_id) {
                return Err(format!(
                    "The selected retired target version {target_id:?} is unavailable"
                ));
            }
            if !target_outline.retired_version_ids.contains(target_id) {
                target_outline.retired_version_ids.push(target_id.clone());
                did_change = true;
            }
            if target_outline.active_version_id.as_deref() == Some(target_id) {
                target_outline.active_version_id = source_outline
                    .active_version_id
                    .as_ref()
                    .and_then(|id| version_map.get(id).cloned());
                did_change = true;
            }
            if did_change {
                changed.insert(target_id.clone());
            }
        }
        if let Some(source_active) = &source_outline.active_version_id {
            if let Some(target_active) = version_map.get(source_active)
                && target_outline.active_version_id.as_ref() != Some(target_active)
            {
                target_outline.active_version_id = Some(target_active.clone());
                changed.insert(target_active.clone());
            }
        } else if let Some(target_active) = target_outline.active_version_id.clone()
            && version_map
                .values()
                .any(|target_id| target_id == &target_active)
            && !is_local(&staged, &target_active, MirroredItemKind::OutlineVersion)
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
        if staged.board_outlines[target_index] != target_outline {
            staged.board_outlines[target_index] = target_outline;
        }
    }

    if pair_bytes(&staged, pair_id)? != pair_bytes_before {
        return Err("Outline reconciliation changed durable pair bytes".into());
    }
    let changed = changed.into_iter().collect();
    *document = staged;
    Ok(changed)
}
