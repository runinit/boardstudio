//! Composition of inverse translation and the existing forward kernels (ticket 115).
//!
//! This module owns disposable drafts. It does not publish a revision or history entry; the edit
//! lifecycle publishes drafts returned by `prepare_transaction_with_ledger` and `prepare_open`.

use super::{
    HostedTransition, IndependentPartTransition, PartProjection, Projection,
    is_omitted_matrix_link, omitted_mirror_matrix, reconcile, unique_board_owner,
};
use crate::constraints::FramedConstraint;
use crate::mirrored_half::inverse::{self, Canonical, PreparedConstraintEdit};
use crate::model::{
    BoardContours, CompanionPlacementTarget, EditOperation, Finding, HalfExceptionState, Matrix,
    MirroredCompanionMode, MirroredCompanionTweak, MirroredExceptionState, MirroredItemKind, Pose2,
    Position, ProjectDoc, Scope, Severity,
};
use std::collections::{BTreeMap, BTreeSet};

#[cfg(test)]
#[path = "prepare/tests.rs"]
mod tests;

/// Builds the source-authoritative document draft used by Open without publishing it.
/// Open has its own route: it does not canonicalize an edit or use transaction preparation,
/// whose ReplaceDocument behavior intentionally preserves replacement geometry.
pub(crate) fn prepare_open(
    accepted: &ProjectDoc,
    _board_contours: &[BoardContours],
) -> Result<(ProjectDoc, Vec<Finding>), String> {
    let mut draft = accepted.clone();
    if let Some(keymap) = &draft.keymap {
        crate::keymap::validate(keymap)?;
    }
    crate::script::apply_scripts(&mut draft)?;
    crate::matrix::layout::resolve(&mut draft)?;

    // Repair only independently linked Parts that can be derived from an existing source before
    // strict Framed endpoint binding checks whether the visible mirror ID is present.
    let pair_views = native_pair_views(&draft);
    for pair in &pair_views {
        seed_missing_independent_parts(&mut draft, &pair.halves, pair.mapping)?;
        super::validate_local_circuit_targets(&draft, &pair.halves)?;
    }
    // Seeding accepts only the narrowly recoverable independent-Part absence. All other
    // malformed, ambiguous, Local/Omitted or unsupported missing records remain strict refusals.
    crate::mechanical::halves::validate_document(&draft)?;

    crate::constraints::resolve(&mut draft)?;
    crate::matrix::layout::sync_components(&mut draft)?;
    crate::constraints::resolve(&mut draft)?;
    crate::matrix::layout::sync_components(&mut draft)?;
    crate::generators::document_error(&draft)?;

    restore_local_matrix_companion_parts(accepted, &mut draft, &[])?;
    for pair in pair_views.iter().cloned() {
        let matrices: Vec<_> = pair
            .halves
            .correspondences
            .iter()
            .filter(|association| association.kind == MirroredItemKind::Matrix)
            .map(|association| (association.source_id.clone(), association.mirror_id.clone()))
            .collect();
        if matrices.is_empty() {
            let projection = super::projection_for_matrixless_pair(&pair.halves, pair.mapping);
            reconcile(&mut draft, &pair.pair_id, &projection)?;
        }
        for (source_matrix_id, mirror_matrix_id) in matrices {
            let projection = projection_for(
                &draft,
                &draft,
                None,
                &pair.halves,
                pair.mapping,
                &source_matrix_id,
                &mirror_matrix_id,
                false,
            )?;
            reconcile(&mut draft, &pair.pair_id, &projection)?;
        }
    }
    crate::outline_controls::detach_removed(&accepted.parts, &mut draft);
    crate::prune_removed_components(&accepted.parts, &mut draft);
    crate::matrix::layout::validate(&draft)?;
    crate::mechanical::halves::validate_document(&draft)?;

    let mut findings = Vec::new();
    for pair in pair_views {
        let targets = changed_linked_mirror_targets(accepted, &draft, &pair.pair_id, &pair.halves)?;
        if !targets.is_empty() {
            findings.push(Finding {
                id: format!("mirrored-half:open-repaired:{}", pair.pair_id),
                severity: Severity::Warning,
                scope: Scope::Layout,
                message:
                    "Linked mirror data was repaired from its source when this document opened."
                        .into(),
                target_ids: targets.into_iter().collect(),
            });
        }
    }
    Ok((draft, findings))
}

#[derive(Clone)]
struct NativePairView {
    pair_id: String,
    mapping: Pose2,
    halves: crate::model::MirroredHalves,
}

fn native_pair_views(document: &ProjectDoc) -> Vec<NativePairView> {
    document
        .hardware
        .as_ref()
        .into_iter()
        .flat_map(|hardware| &hardware.pairs)
        .filter_map(|pair| {
            let halves = pair.halves.as_ref()?;
            (halves.source_board_id != halves.mirror_board_id).then(|| NativePairView {
                pair_id: pair.id.clone(),
                mapping: pair.mapping,
                halves: halves.clone(),
            })
        })
        .collect()
}

fn changed_linked_mirror_targets(
    before_document: &ProjectDoc,
    after_document: &ProjectDoc,
    pair_id: &str,
    halves: &crate::model::MirroredHalves,
) -> Result<BTreeSet<String>, String> {
    let mut changed = BTreeSet::new();
    let before_board_keycaps = before_document
        .keycaps
        .as_ref()
        .and_then(|keycaps| keycaps.boards.get(&halves.mirror_board_id));
    let after_board_keycaps = after_document
        .keycaps
        .as_ref()
        .and_then(|keycaps| keycaps.boards.get(&halves.mirror_board_id));
    if serialize_matching_items(before_board_keycaps.into_iter())?
        != serialize_matching_items(after_board_keycaps.into_iter())?
    {
        changed.insert(halves.mirror_board_id.clone());
    }
    for association in &halves.correspondences {
        if halves.overlay_of(association).is_some_and(|exception| {
            matches!(
                exception.state,
                MirroredExceptionState::Local | MirroredExceptionState::Omitted
            )
        }) {
            continue;
        }
        let before_snapshot = mirror_item_snapshot(
            before_document,
            pair_id,
            association.kind,
            &association.mirror_id,
        )?;
        let after_snapshot = mirror_item_snapshot(
            after_document,
            pair_id,
            association.kind,
            &association.mirror_id,
        )?;
        let before_keycap =
            keycap_setting_snapshot(before_document, association.kind, &association.mirror_id)?;
        let after_keycap =
            keycap_setting_snapshot(after_document, association.kind, &association.mirror_id)?;
        if before_snapshot != after_snapshot || before_keycap != after_keycap {
            changed.insert(association.mirror_id.clone());
        }
    }
    Ok(changed)
}

fn keycap_setting_snapshot(
    document: &ProjectDoc,
    kind: MirroredItemKind,
    id: &str,
) -> Result<Option<Vec<u8>>, String> {
    let Some(keycaps) = document.keycaps.as_ref() else {
        return Ok(None);
    };
    match kind {
        MirroredItemKind::Part => serialize_matching_items(keycaps.keys.get(id).into_iter()),
        MirroredItemKind::Matrix => serialize_matching_items(keycaps.matrices.get(id).into_iter()),
        _ => Ok(None),
    }
}

fn serialize_matching_items<'a, T: serde::Serialize + 'a>(
    items: impl Iterator<Item = &'a T>,
) -> Result<Option<Vec<u8>>, String> {
    let items: Vec<_> = items.collect();
    if items.is_empty() {
        return Ok(None);
    }
    serde_json::to_vec(&items)
        .map(Some)
        .map_err(|error| error.to_string())
}

fn mirror_item_snapshot(
    document: &ProjectDoc,
    pair_id: &str,
    kind: MirroredItemKind,
    id: &str,
) -> Result<Option<Vec<u8>>, String> {
    match kind {
        MirroredItemKind::Part => {
            serialize_matching_items(document.parts.iter().filter(|item| item.id == id))
        }
        MirroredItemKind::Matrix => {
            serialize_matching_items(document.matrices.iter().filter(|item| item.id == id))
        }
        MirroredItemKind::Layout => {
            serialize_matching_items(document.layouts.iter().filter(|item| item.id == id))
        }
        MirroredItemKind::Outline => {
            serialize_matching_items(document.outline.iter().filter(|item| item.id() == id))
        }
        MirroredItemKind::Net => {
            serialize_matching_items(document.nets.iter().filter(|item| item.id == id))
        }
        MirroredItemKind::Assembly => {
            serialize_matching_items(document.assemblies.iter().filter(|item| item.id == id))
        }
        MirroredItemKind::Module => {
            serialize_matching_items(document.modules.iter().filter(|item| item.id == id))
        }
        MirroredItemKind::ElectricalBoard => serialize_matching_items(
            document
                .hardware
                .as_ref()
                .into_iter()
                .flat_map(|hardware| &hardware.boards)
                .filter(|item| item.board_id == id),
        ),
        MirroredItemKind::OutlineVersion => serialize_matching_items(
            document
                .board_outlines
                .iter()
                .flat_map(|outline| &outline.versions)
                .filter(|version| version.id == id),
        ),
        MirroredItemKind::MechanicalFeature => serialize_matching_items(
            document
                .hardware
                .as_ref()
                .into_iter()
                .flat_map(|hardware| &hardware.pairs)
                .filter(|pair| pair.id == pair_id)
                .flat_map(|pair| &pair.features)
                .filter(|feature| feature.id == id),
        ),
        MirroredItemKind::EmbeddedCircuit => serialize_matching_items(
            document
                .embedded_circuits
                .iter()
                .filter(|item| item.id == id),
        ),
    }
}

/// Recreate only a missing, independently corresponded Part. All source geometry and payload
/// still come from the shared derivation kernel; the durable correspondence supplies its target
/// ID, while the creation rules deterministically select reference and definition identities.
fn seed_missing_independent_parts(
    document: &mut ProjectDoc,
    halves: &crate::model::MirroredHalves,
    mapping: Pose2,
) -> Result<(), String> {
    let source_board_id = &halves.source_board_id;
    let mirror_board_id = &halves.mirror_board_id;
    let source_matrix_members: BTreeSet<_> = document
        .matrices
        .iter()
        .filter(|matrix| matrix.board_id.as_deref() == Some(source_board_id.as_str()))
        .flat_map(|matrix| matrix.part_ids.iter().cloned())
        .collect();
    let circuit_members: BTreeSet<_> = document
        .embedded_circuits
        .iter()
        .filter(|circuit| circuit.host_board_id == *source_board_id)
        .flat_map(|circuit| circuit.part_ids.iter().cloned())
        .collect();
    let mut seen_sources = BTreeSet::new();
    let mut seen_targets = BTreeSet::new();
    for association in halves
        .correspondences
        .iter()
        .filter(|association| association.kind == MirroredItemKind::Part)
    {
        if !seen_sources.insert(association.source_id.as_str()) {
            return Err(format!(
                "The Open repair repeats source part {:?}",
                association.source_id
            ));
        }
        if !seen_targets.insert(association.mirror_id.as_str()) {
            return Err(format!(
                "The Open repair repeats mirror part {:?}",
                association.mirror_id
            ));
        }
    }

    let mut planned_references = BTreeSet::new();
    for association in halves
        .correspondences
        .iter()
        .filter(|association| association.kind == MirroredItemKind::Part)
    {
        if halves.overlay_of(association).is_some()
            || source_matrix_members.contains(&association.source_id)
            || circuit_members.contains(&association.source_id)
        {
            continue;
        }
        let target_parts: Vec<_> = document
            .parts
            .iter()
            .filter(|part| part.id == association.mirror_id)
            .collect();
        if target_parts.len() > 1 {
            return Err(format!(
                "The Open repair mirror part {:?} is ambiguous",
                association.mirror_id
            ));
        }
        if !target_parts.is_empty() {
            continue;
        }
        let mut sources = document
            .parts
            .iter()
            .filter(|part| part.id == association.source_id);
        let source = sources.next().ok_or_else(|| {
            format!(
                "The Open repair source part {:?} is unavailable",
                association.source_id
            )
        })?;
        if sources.next().is_some() {
            return Err(format!(
                "The Open repair source part {:?} is ambiguous",
                association.source_id
            ));
        }
        if unique_board_owner(document, &association.source_id, "Open repair")? != source_board_id {
            return Err(format!(
                "The Open repair source part {:?} has stale board ownership",
                association.source_id
            ));
        }
        if id_exists_outside_parts(document, &association.mirror_id) {
            return Err(format!(
                "The Open repair target identity {:?} is already in use",
                association.mirror_id
            ));
        }
        let reference =
            super::super::mirror_reference(source_board_id, mirror_board_id, &source.reference);
        if !planned_references.insert(reference.clone())
            || document
                .parts
                .iter()
                .any(|part| part.reference == reference)
        {
            return Err(format!(
                "The Open repair mirror reference {reference:?} is already in use"
            ));
        }
        let target_board = unique_board(document, mirror_board_id)?;
        let registered_boards: Vec<_> = document
            .boards
            .iter()
            .flat_map(|board| {
                board
                    .part_ids
                    .iter()
                    .filter(move |id| *id == &association.mirror_id)
                    .map(move |_| board.id.as_str())
            })
            .collect();
        let already_registered = match registered_boards.as_slice() {
            [] => false,
            [registered_board] if *registered_board == target_board => true,
            _ => {
                return Err(format!(
                    "The missing mirror part {:?} has conflicting or repeated board registration",
                    association.mirror_id
                ));
            }
        };
        let definition = source.definition_id.clone();
        let target = crate::mirrored_half::derive::part(
            source,
            &association.mirror_id,
            &definition,
            &reference,
            mapping,
        );
        document.parts.push(target);
        if !already_registered {
            document
                .boards
                .iter_mut()
                .find(|board| board.id == target_board)
                .expect("unique target board was just checked")
                .part_ids
                .push(association.mirror_id.clone());
        }
    }
    Ok(())
}

fn unique_board(document: &ProjectDoc, id: &str) -> Result<String, String> {
    let mut boards = document.boards.iter().filter(|board| board.id == id);
    let board = boards
        .next()
        .ok_or_else(|| format!("The Open repair board {id:?} is unavailable"))?;
    if boards.next().is_some() {
        return Err(format!("The Open repair board {id:?} is ambiguous"));
    }
    Ok(board.id.clone())
}

fn id_exists_outside_parts(document: &ProjectDoc, id: &str) -> bool {
    document.boards.iter().any(|item| item.id == id)
        || document.modules.iter().any(|item| item.id == id)
        || document.matrices.iter().any(|item| item.id == id)
        || document.layouts.iter().any(|item| item.id == id)
        || document.outline.iter().any(|item| item.id() == id)
        || document.nets.iter().any(|item| item.id == id)
        || document
            .board_outlines
            .iter()
            .flat_map(|outline| &outline.versions)
            .any(|item| item.id == id)
}

fn set_companion_placement(
    document: &mut ProjectDoc,
    pair_id: &str,
    targets: &[CompanionPlacementTarget],
    mode: MirroredCompanionMode,
    tweak: Option<MirroredCompanionTweak>,
) -> Result<Vec<String>, String> {
    let pair = document
        .hardware
        .as_mut()
        .into_iter()
        .flat_map(|hardware| &mut hardware.pairs)
        .find(|pair| pair.id == pair_id);
    let Some(pair) = pair else {
        return Ok(Vec::new());
    };
    let Some(halves) = pair.halves.as_mut() else {
        return Ok(Vec::new());
    };
    let has_pair = targets
        .iter()
        .any(|target| matches!(target, CompanionPlacementTarget::Pair));
    if has_pair && tweak.is_some() {
        return Err("A pair default cannot carry a companion tweak".into());
    }
    if let Some(tweak) = tweak {
        crate::mechanical::halves::validate_companion(mode, Some(tweak))?;
    }
    let mut eligible = BTreeSet::new();
    for matrix_link in halves
        .correspondences
        .iter()
        .filter(|link| link.kind == MirroredItemKind::Matrix)
    {
        let Some(matrix) = document
            .matrices
            .iter()
            .find(|matrix| matrix.id == matrix_link.source_id)
        else {
            continue;
        };
        for cell in matrix
            .cells
            .iter()
            .filter(|cell| cell.enabled && !cell.deleted)
        {
            let key_id = crate::matrix::member_id(&matrix.id, cell.row, cell.column);
            for assembly in &cell.assemblies {
                if crate::mirrored_half::SYMMETRIC_DEFINITIONS
                    .contains(&assembly.definition_id.as_str())
                {
                    continue;
                }
                let companion_id = crate::matrix::companion_id(&key_id, &assembly.id);
                eligible.insert(companion_id);
            }
        }
    }
    let mut selected = BTreeSet::new();
    for target in targets {
        match target {
            CompanionPlacementTarget::Pair => {}
            CompanionPlacementTarget::Item {
                item_kind,
                mirror_id,
            } => {
                if let Some(link) = halves.correspondences.iter().find(|link| {
                    link.kind == *item_kind
                        && (link.mirror_id == *mirror_id || link.source_id == *mirror_id)
                }) && link.kind == MirroredItemKind::Part
                    && eligible.contains(&link.source_id)
                {
                    selected.insert(link.mirror_id.clone());
                }
            }
            CompanionPlacementTarget::Row { matrix_id, .. }
            | CompanionPlacementTarget::Column { matrix_id, .. }
            | CompanionPlacementTarget::Matrix { matrix_id } => {
                let Some(matrix_link) = halves.correspondences.iter().find(|link| {
                    link.kind == MirroredItemKind::Matrix
                        && (link.source_id == *matrix_id || link.mirror_id == *matrix_id)
                }) else {
                    continue;
                };
                let Some(matrix) = document
                    .matrices
                    .iter()
                    .find(|matrix| matrix.id == matrix_link.source_id)
                else {
                    continue;
                };
                let coordinates: BTreeSet<_> = matrix
                    .cells
                    .iter()
                    .filter(|cell| {
                        cell.enabled
                            && !cell.deleted
                            && match target {
                                CompanionPlacementTarget::Row { row, .. } => cell.row == *row,
                                CompanionPlacementTarget::Column { column, .. } => {
                                    cell.column == *column
                                }
                                CompanionPlacementTarget::Matrix { .. } => true,
                                _ => false,
                            }
                    })
                    .map(|cell| (cell.row, cell.column))
                    .collect();
                selected.extend(halves.correspondences.iter().filter_map(|link| {
                    let belongs = coordinates.iter().any(|(row, column)| {
                        let key = crate::matrix::member_id(&matrix.id, *row, *column);
                        link.kind == MirroredItemKind::Part
                            && eligible.contains(&link.source_id)
                            && link.source_id.starts_with(&format!("{key}/"))
                    });
                    belongs.then(|| link.mirror_id.clone())
                }));
            }
        }
    }
    let mut changed = Vec::new();
    if has_pair && halves.companion_mode != mode {
        if mode != MirroredCompanionMode::LiteralMirrorWithTweak {
            for link in &mut halves.correspondences {
                if link.companion_mode.is_none() && link.companion_tweak.is_some() {
                    link.companion_mode = Some(MirroredCompanionMode::LiteralMirrorWithTweak);
                }
            }
            for exception in &mut halves.exceptions {
                if exception.companion_mode.is_none() && exception.companion_tweak.is_some() {
                    exception.companion_mode = Some(MirroredCompanionMode::LiteralMirrorWithTweak);
                }
            }
        }
        halves.companion_mode = mode;
        changed.push(pair_id.to_owned());
    }
    for mirror_id in selected {
        let Some(link) = halves
            .correspondences
            .iter_mut()
            .find(|link| link.kind == MirroredItemKind::Part && link.mirror_id == mirror_id)
        else {
            continue;
        };
        if link.companion_mode != Some(mode) || link.companion_tweak != tweak {
            link.companion_mode = Some(mode);
            link.companion_tweak = tweak;
            changed.push(mirror_id);
        }
    }
    Ok(changed)
}

/// Builds a complete document draft for one edit without changing the accepted document.
#[cfg(test)]
pub(crate) fn prepare_transaction(
    accepted: &ProjectDoc,
    board_contours: &[BoardContours],
    operation: &EditOperation,
) -> Result<(ProjectDoc, Vec<String>), String> {
    prepare_transaction_with_ledger(
        accepted,
        board_contours,
        operation,
        &crate::mechanical::edit::MechanicalIdentityLedger::default(),
    )
}

/// Builds a complete document draft while consulting the owning session's identity
/// reservations, so a later addition never reuses an identity hidden by Undo or a
/// disabled configuration.
pub(crate) fn prepare_transaction_with_ledger(
    accepted: &ProjectDoc,
    board_contours: &[BoardContours],
    operation: &EditOperation,
    identities: &crate::mechanical::edit::MechanicalIdentityLedger,
) -> Result<(ProjectDoc, Vec<String>), String> {
    crate::mechanical::halves::validate_document(accepted)?;
    let mut canonical =
        inverse::canonicalize(accepted, operation).map_err(|refusal| refusal.to_string())?;
    if let EditOperation::ReplaceDocument { document } = &mut canonical.operation {
        super::lifecycle::retire_replacement_records(accepted, document)?;
    }
    let mut draft = accepted.clone();
    let mut changed =
        apply_canonical(&mut draft, accepted, board_contours, &canonical, identities)?;
    let lifecycle_before_resolution = matches!(
        canonical.operation,
        EditOperation::SetMechanical {
            configuration: None,
            target: Some(crate::mechanical::MechanicalTarget::Instance { .. }),
        }
    );

    if let Some(keymap) = &draft.keymap {
        crate::keymap::validate(keymap)?;
    }
    crate::electrical::preserve_handoff(accepted, &mut draft);
    crate::matrix::layout::validate(&draft)?;

    if lifecycle_before_resolution {
        changed.extend(prepare_lifecycle_pairs(accepted, &mut draft, &canonical)?);
    }

    let positions = operation_positions(&canonical.operation);
    let mut framed = initial_framed_constraints(&canonical);
    crate::matrix::layout::resolve(&mut draft)?;
    changed.extend(crate::constraints::resolve_prepared(
        &mut draft, positions, &framed,
    )?);
    changed.extend(crate::matrix::layout::sync_components(&mut draft)?);

    if !lifecycle_before_resolution {
        changed.extend(prepare_lifecycle_pairs(accepted, &mut draft, &canonical)?);
    }

    for addition in &canonical.matrix_additions {
        let mirrored =
            crate::mirrored_half::matrix_addition::mirror_added_matrix(&draft, addition)?;
        draft = mirrored.draft;
        changed.extend(mirrored.changed_ids);
    }

    framed = bind_deferred_constraints(&draft, &canonical, &framed)?;
    persist_retuned_constraints(accepted, &mut draft, &canonical, &framed)?;
    crate::matrix::layout::resolve(&mut draft)?;
    changed.extend(crate::constraints::resolve_prepared(
        &mut draft, positions, &framed,
    )?);
    changed.extend(crate::matrix::layout::sync_components(&mut draft)?);

    let local_parts_baseline = match &canonical.operation {
        EditOperation::ReplaceDocument { document } => document.as_ref(),
        _ => accepted,
    };
    changed.extend(restore_local_matrix_companion_parts(
        local_parts_baseline,
        &mut draft,
        operation_positions(&canonical.operation),
    )?);
    changed.extend(reconcile_pairs(accepted, &mut draft, &canonical)?);
    changed.extend(add_target_layout_memberships(
        &mut draft,
        &canonical.additions,
    )?);
    changed.extend(record_independent_additions(
        &mut draft,
        &canonical.additions,
    )?);
    crate::outline_controls::detach_removed(&accepted.parts, &mut draft);
    changed.extend(crate::prune_removed_components(&accepted.parts, &mut draft));

    resolve_mirror_replacement_electrical(accepted, &mut draft, &canonical.operation)?;

    validate_prepared_draft(accepted, &draft, &canonical.operation)?;

    let (mut actual_ids, _) = super::changed_document_fields(accepted, &draft)?;
    for id in changed {
        if entity_changed(accepted, &draft, &id)? {
            actual_ids.push(id);
        }
    }
    actual_ids.sort();
    actual_ids.dedup();
    Ok((draft, actual_ids))
}

/// Plans replacement pin assignments after reconciliation has projected linked pins and locks.
/// Placement-only Parts keep the existing 151 transaction behavior and do not require a board
/// controller merely because the resolver can report unrelated board-level findings.
fn resolve_mirror_replacement_electrical(
    accepted: &ProjectDoc,
    draft: &mut ProjectDoc,
    operation: &EditOperation,
) -> Result<(), String> {
    let (mirror_board_id, added_part_id) = match operation {
        EditOperation::AddMirrorOnlyPart { board_id, part, .. } => {
            (board_id.as_str(), Some(part.id.as_str()))
        }
        EditOperation::SetHalfException {
            pair_id,
            kind: MirroredItemKind::Part,
            state: HalfExceptionState::Linked,
            ..
        } => {
            let pairs: Vec<_> = accepted
                .hardware
                .as_ref()
                .into_iter()
                .flat_map(|hardware| &hardware.pairs)
                .filter(|pair| pair.id == *pair_id)
                .collect();
            let [pair] = pairs.as_slice() else {
                return Err("The exact lifecycle pair is unavailable or ambiguous".into());
            };
            let halves = pair
                .halves
                .as_ref()
                .ok_or_else(|| "The exact lifecycle pair has no native halves".to_string())?;
            (halves.mirror_board_id.as_str(), None)
        }
        _ => return Ok(()),
    };
    let rejoin_displaced_assignment = if added_part_id.is_none() {
        let assignments_before = electrical_assignments(accepted, mirror_board_id)?;
        let assignments_after = electrical_assignments(draft, mirror_board_id)?;
        match (assignments_before, assignments_after) {
            (Some(before), Some(after)) => before
                .iter()
                .any(|(function, pin)| after.get(function) != Some(pin)),
            _ => false,
        }
    } else {
        false
    };
    let mode = draft
        .hardware
        .as_ref()
        .into_iter()
        .flat_map(|hardware| &hardware.boards)
        .find(|board| board.board_id == mirror_board_id)
        .map_or(crate::electrical::ElectricalMode::default(), |board| {
            board.mode
        });
    let plan = crate::electrical::resolve(crate::electrical::ElectricalPlanRequest {
        document: draft.clone(),
        instance_id: None,
        mode,
        locks: Default::default(),
        controller_profile: None,
        board_id: Some(mirror_board_id.to_owned()),
        controller_part_id: None,
    });
    let placement_requires_plan =
        added_part_id.is_some_and(|part_id| plan_requires_electrical_for_part(&plan, part_id));
    if !placement_requires_plan && !rejoin_displaced_assignment {
        return Ok(());
    }
    if let Some(error) = plan
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.severity == "error")
    {
        return Err(error.message.clone());
    }
    crate::electrical::materialize_reviewed(draft, &plan, true)
}

fn electrical_assignments(
    document: &ProjectDoc,
    board_id: &str,
) -> Result<Option<BTreeMap<String, String>>, String> {
    let matches: Vec<_> = document
        .hardware
        .as_ref()
        .into_iter()
        .flat_map(|hardware| &hardware.boards)
        .filter(|board| board.board_id == board_id)
        .collect();
    match matches.as_slice() {
        [] => Ok(None),
        [board] => Ok(Some(board.assignments.clone())),
        _ => Err(format!("Mirror electrical board {board_id:?} is ambiguous")),
    }
}

fn plan_requires_electrical_for_part(
    plan: &crate::electrical::ElectricalPlan,
    part_id: &str,
) -> bool {
    plan.peripherals
        .iter()
        .any(|peripheral| peripheral.part_id == part_id)
        || plan
            .assignments
            .iter()
            .any(|assignment| assignment.key_id == part_id)
        || plan
            .nets
            .iter()
            .any(|net| net.pins.iter().any(|pin| pin.part_id == part_id))
        || plan
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.key_id.as_deref() == Some(part_id))
}

fn add_target_layout_memberships(
    document: &mut ProjectDoc,
    additions: &[inverse::PartAddition],
) -> Result<Vec<String>, String> {
    let mut changed = Vec::new();
    for addition in additions {
        let Some(layout_id) = &addition.mirror_layout_id else {
            continue;
        };
        let mirror_board_id = super::super::board_for_part(document, &addition.mirror_id)
            .ok_or_else(|| format!("Mirror Part {:?} has no board owner", addition.mirror_id))?
            .id
            .clone();
        let mut layouts = document
            .layouts
            .iter_mut()
            .filter(|layout| layout.id == *layout_id);
        let layout = layouts
            .next()
            .ok_or_else(|| format!("Target layout {layout_id:?} is unavailable"))?;
        if layouts.next().is_some() || layout.board_id != mirror_board_id {
            return Err(format!(
                "Target layout {layout_id:?} has ambiguous or inconsistent ownership"
            ));
        }
        if layout.part_ids.contains(&addition.mirror_id) {
            return Err(format!(
                "Target layout {layout_id:?} already contains Part {:?}",
                addition.mirror_id
            ));
        }
        layout.part_ids.push(addition.mirror_id.clone());
        changed.push(layout_id.clone());
    }
    Ok(changed)
}

fn validate_prepared_draft(
    accepted: &ProjectDoc,
    draft: &ProjectDoc,
    operation: &EditOperation,
) -> Result<(), String> {
    crate::matrix::layout::validate(draft)?;
    crate::mechanical::halves::validate_document(draft)?;
    if !matches!(operation, EditOperation::ReplaceDocument { .. }) {
        crate::generators::introduced_error(
            &crate::generators::parameter_problems(accepted),
            draft,
        )?;
    }
    validate_constraint_postcondition(draft)?;
    Ok(())
}

/// Stores only an actual MoveParts retune. Merely compiling an untouched legacy record for the
/// private solver must not upgrade its wire shape or create a history change.
fn persist_retuned_constraints(
    accepted: &ProjectDoc,
    draft: &mut ProjectDoc,
    canonical: &Canonical,
    framed: &[FramedConstraint],
) -> Result<(), String> {
    if !matches!(canonical.operation, EditOperation::MoveParts { .. }) {
        return Ok(());
    }
    let retuned_ids: BTreeSet<_> = canonical
        .prepared_constraints
        .iter()
        .filter_map(|edit| match edit {
            PreparedConstraintEdit::Upsert(record) => Some(record_id(record).to_owned()),
            PreparedConstraintEdit::DeferredTargetRetune(intent) => {
                Some(record_id(&intent.record).to_owned())
            }
            PreparedConstraintEdit::Remove { .. } => None,
        })
        .collect();
    for record in framed {
        let id = record_id(record);
        if !retuned_ids.contains(id) {
            continue;
        }
        let Some(stored) = accepted.constraints.iter().find(|item| item.id() == id) else {
            continue;
        };
        let original = crate::constraints::compile_legacy(accepted, stored)?;
        if framed_records_equivalent(&original, record) {
            continue;
        }
        let mut matches = draft.constraints.iter_mut().filter(|item| item.id() == id);
        let Some(slot) = matches.next() else {
            return Err(format!(
                "Prepared constraint {id:?} is unavailable for persistence"
            ));
        };
        if matches.next().is_some() {
            return Err(format!(
                "Prepared constraint {id:?} is ambiguous for persistence"
            ));
        }
        *slot = constraint_from_framed(record);
    }
    Ok(())
}

fn constraint_from_framed(record: &FramedConstraint) -> crate::model::Constraint {
    let FramedConstraint::Framed {
        id,
        source,
        target,
        relation,
    } = record;
    crate::model::Constraint::Framed {
        id: id.clone(),
        source: source.clone(),
        target: target.clone(),
        relation: relation.clone(),
    }
}

fn framed_records_equivalent(left: &FramedConstraint, right: &FramedConstraint) -> bool {
    left == right
}

fn validate_constraint_postcondition(draft: &ProjectDoc) -> Result<(), String> {
    if draft.constraints.is_empty() {
        return Ok(());
    }
    const POSE_TOLERANCE: f64 = 1e-9;
    let mut checked = draft.clone();
    crate::constraints::resolve_prepared(&mut checked, &[], &[])?;
    for original in &draft.parts {
        let mut matches = checked.parts.iter().filter(|part| part.id == original.id);
        let Some(solved) = matches.next() else {
            return Err(format!(
                "Prepared constraint postcondition lost part {:?}",
                original.id
            ));
        };
        if matches.next().is_some() {
            return Err(format!(
                "Prepared constraint postcondition found ambiguous part {:?}",
                original.id
            ));
        }
        if (solved.pose.at.x - original.pose.at.x).abs() >= POSE_TOLERANCE
            || (solved.pose.at.y - original.pose.at.y).abs() >= POSE_TOLERANCE
            || (solved.pose.rotation - original.pose.rotation).abs() >= POSE_TOLERANCE
        {
            let constraint_id = crate::constraints::driving_constraint(draft, &original.id)
                .ok()
                .flatten()
                .map(|constraint| format!(" for constraint {:?}", constraint.id()))
                .unwrap_or_default();
            return Err(format!(
                "Prepared constraint postcondition would move part {:?}{constraint_id}",
                original.id,
            ));
        }
    }
    Ok(())
}

fn prepare_lifecycle_pairs(
    accepted: &ProjectDoc,
    document: &mut ProjectDoc,
    canonical: &Canonical,
) -> Result<Vec<String>, String> {
    if matches!(
        canonical.operation,
        EditOperation::AddPart { .. }
            | EditOperation::AddMirrorOnlyPart { .. }
            | EditOperation::SetCompanionPlacement { .. }
            | EditOperation::ApplyMechanicalEdit { .. }
            | EditOperation::SetMechanical {
                configuration: Some(_),
                target: Some(crate::mechanical::MechanicalTarget::Instance { .. }),
                ..
            }
            | EditOperation::CreateMirroredPair { .. }
            | EditOperation::CreateMirroredHalf { .. }
            | EditOperation::ReplaceDocument { .. }
    ) {
        return Ok(Vec::new());
    }
    if let EditOperation::SetHalfException { pair_id, .. }
    | EditOperation::UnlinkHalves { pair_id } = &canonical.operation
    {
        let matches: Vec<_> = accepted
            .hardware
            .as_ref()
            .into_iter()
            .flat_map(|hardware| &hardware.pairs)
            .filter(|pair| pair.id == *pair_id && pair.halves.is_some())
            .collect();
        if matches.len() != 1 {
            return Err("The exact lifecycle pair is unavailable or ambiguous".into());
        }
    }
    let pair_removed_by_operation = matches!(
        &canonical.operation,
        EditOperation::SetMechanical {
            configuration: None,
            target: Some(crate::mechanical::MechanicalTarget::Instance { .. }),
        }
    );
    let pair_ids: Vec<_> = accepted
        .hardware
        .as_ref()
        .into_iter()
        .flat_map(|hardware| &hardware.pairs)
        .filter(|pair| pair.halves.is_some())
        .map(|pair| pair.id.clone())
        .collect();
    let mut changed = Vec::new();
    for pair_id in pair_ids {
        if !pair_removed_by_operation
            && !document
                .hardware
                .as_ref()
                .is_some_and(|hardware| hardware.pairs.iter().any(|pair| pair.id == pair_id))
        {
            continue;
        }
        if let Some(prepared) = super::lifecycle::prepare_edit_transition(
            accepted,
            document,
            &pair_id,
            &canonical.operation,
        )? {
            changed.extend(prepared.changed_document_ids);
            *document = prepared.draft;
        }
    }
    Ok(changed)
}

fn apply_canonical(
    draft: &mut ProjectDoc,
    accepted: &ProjectDoc,
    board_contours: &[BoardContours],
    canonical: &Canonical,
    identities: &crate::mechanical::edit::MechanicalIdentityLedger,
) -> Result<Vec<String>, String> {
    if let EditOperation::AddMirrorOnlyPart { board_id, part, .. } = &canonical.operation {
        if id_exists_outside_parts(accepted, &part.id) {
            return Err(format!(
                "Mirror-only placement Part ID {:?} conflicts with another document identity",
                part.id
            ));
        }
        let pairs = accepted
            .hardware
            .iter()
            .flat_map(|hardware| &hardware.pairs)
            .filter(|pair| {
                pair.halves.as_ref().is_some_and(|halves| {
                    halves.mirror_board_id == *board_id && halves.source_board_id != *board_id
                })
            })
            .collect::<Vec<_>>();
        if pairs.len() != 1 {
            return Err(format!(
                "Mirror-only placement requires one exact native mirror board; {board_id:?} matched {} pairs",
                pairs.len()
            ));
        }
    }
    if matches!(
        canonical.operation,
        EditOperation::SetHalfException { .. } | EditOperation::UnlinkHalves { .. }
    ) {
        return Ok(Vec::new());
    }
    if let EditOperation::SetCompanionPlacement {
        pair_id,
        targets,
        mode,
        tweak,
    } = &canonical.operation
    {
        return set_companion_placement(draft, pair_id, targets, *mode, *tweak);
    }
    if let EditOperation::ApplyMechanicalEdit { edit } = &canonical.operation
        && let crate::mechanical::MechanicalChange::UnlinkMirrorPair { instance_id } = &edit.change
    {
        let pair_id = pair_for_instance(accepted, instance_id)?;
        return crate::mechanical::halves::unlink_pair(draft, &pair_id);
    }
    if let EditOperation::SetMechanical {
        configuration,
        target: Some(target @ crate::mechanical::MechanicalTarget::Instance { .. }),
    } = &canonical.operation
    {
        return crate::mechanical::edit::apply_instance_configuration(
            draft,
            board_contours,
            target,
            configuration.as_deref().cloned(),
        );
    }
    let mut changed =
        crate::apply_with_ledger(draft, board_contours, &canonical.operation, identities)?;
    // Ticket 29 B4: removing a halves-pair source over an Omitted mirror
    // orphans the mirror layout shell — its matrix is absent by design and
    // its source is gone, so layout validation could never resolve it. Drop
    // shells left naming only such mirrors here; the kernel retires the pair
    // records. Anything else is untouched.
    if let EditOperation::RemoveMatrix { id } = &canonical.operation {
        let mut orphaned_mirrors = BTreeSet::new();
        for pair in accepted
            .hardware
            .iter()
            .flat_map(|hardware| &hardware.pairs)
        {
            let Some(halves) = pair.halves.as_ref() else {
                continue;
            };
            for record in &halves.correspondences {
                if record.kind == MirroredItemKind::Matrix && record.source_id == *id {
                    let omitted = halves.exceptions.iter().any(|exception| {
                        exception.kind == MirroredItemKind::Matrix
                            && exception.mirror_id == record.mirror_id
                            && exception.state == crate::model::MirroredExceptionState::Omitted
                    });
                    if omitted {
                        orphaned_mirrors.insert(record.mirror_id.clone());
                    }
                }
            }
        }
        if !orphaned_mirrors.is_empty() {
            for layout in &mut draft.layouts {
                layout
                    .matrix_ids
                    .retain(|matrix_id| !orphaned_mirrors.contains(matrix_id));
            }
            let before_layouts = draft.layouts.len();
            draft.layouts.retain(|layout| !layout.matrix_ids.is_empty());
            if draft.layouts.len() != before_layouts {
                changed.push(id.clone());
            }
        }
    }
    for addition in &canonical.hosted.circuits {
        for (member_id, pose) in &addition.member_poses {
            let member = draft
                .parts
                .iter_mut()
                .find(|part| part.id == *member_id)
                .ok_or_else(|| {
                    format!("The embedded circuit member {member_id:?} is unavailable")
                })?;
            member.pose = *pose;
        }
    }
    Ok(changed)
}

fn pair_for_instance(document: &ProjectDoc, instance_id: &str) -> Result<String, String> {
    let mut pairs = document
        .hardware
        .as_ref()
        .ok_or("Physical instances are unavailable")?
        .pairs
        .iter()
        .filter(|pair| pair.members.iter().any(|member| member == instance_id));
    let pair = pairs.next().ok_or_else(|| {
        format!("Physical instance {instance_id:?} is not part of a mechanical pair.")
    })?;
    if pairs.next().is_some() {
        return Err(format!(
            "Physical instance {instance_id:?} belongs to more than one mechanical pair."
        ));
    }
    Ok(pair.id.clone())
}

fn operation_positions(operation: &EditOperation) -> &[Position] {
    match operation {
        EditOperation::MoveParts { positions } => positions,
        _ => &[],
    }
}

fn initial_framed_constraints(canonical: &Canonical) -> Vec<FramedConstraint> {
    canonical
        .prepared_constraints
        .iter()
        .filter_map(|edit| match edit {
            PreparedConstraintEdit::Upsert(record) => Some(record.clone()),
            PreparedConstraintEdit::DeferredTargetRetune(intent) => Some(intent.record.clone()),
            PreparedConstraintEdit::Remove { .. } => None,
        })
        .collect()
}

fn bind_deferred_constraints(
    document: &ProjectDoc,
    canonical: &Canonical,
    already_framed: &[FramedConstraint],
) -> Result<Vec<FramedConstraint>, String> {
    let mut framed: Vec<_> = already_framed
        .iter()
        .filter(|record| {
            !canonical.prepared_constraints.iter().any(|edit| {
                matches!(edit, PreparedConstraintEdit::DeferredTargetRetune(intent) if record_id(&intent.record) == record_id(record))
            })
        })
        .cloned()
        .collect();
    for edit in &canonical.prepared_constraints {
        let PreparedConstraintEdit::DeferredTargetRetune(intent) = edit else {
            continue;
        };
        let source_id = authoritative_id(&intent.record, true);
        let target_id = authoritative_id(&intent.record, false);
        let source = part_pose(document, source_id)?;
        let target = part_pose(document, target_id)?;
        let owners = [
            intent
                .source_owner
                .as_deref()
                .map(|id| part_pose(document, id))
                .transpose()?,
            intent
                .target_owner
                .as_deref()
                .map(|id| part_pose(document, id))
                .transpose()?,
        ];
        match crate::constraints::prepare_target_retune(
            document,
            &intent.record,
            source,
            target,
            &intent.requested,
            owners,
        )? {
            crate::constraints::PreparedTargetRetune::Retuned(record) => framed.push(record),
            crate::constraints::PreparedTargetRetune::Deferred(_) => {
                return Err("A prepared constraint still lacks its resolved owner pose".into());
            }
        }
    }
    Ok(framed)
}

fn record_id(record: &FramedConstraint) -> &str {
    match record {
        FramedConstraint::Framed { id, .. } => id,
    }
}

fn authoritative_id(record: &FramedConstraint, source: bool) -> &str {
    let FramedConstraint::Framed {
        source: source_endpoint,
        target: target_endpoint,
        ..
    } = record;
    match if source {
        source_endpoint
    } else {
        target_endpoint
    } {
        crate::constraints::ConstraintEndpoint::Part { part_id } => part_id,
        crate::constraints::ConstraintEndpoint::LinkedMirror { source_part_id, .. } => {
            source_part_id
        }
    }
}

fn part_pose(document: &ProjectDoc, id: &str) -> Result<Pose2, String> {
    let mut parts = document.parts.iter().filter(|part| part.id == id);
    let part = parts
        .next()
        .ok_or_else(|| format!("Prepared constraint part {id:?} is unavailable"))?;
    if parts.next().is_some() {
        return Err(format!("Prepared constraint part {id:?} is ambiguous"));
    }
    Ok(part.pose)
}

/// Restore Local companion source data after generic matrix sync rematerializes it.
/// Reconciliation consumes this pose and rebases the target cell assembly in the same pass.
fn restore_local_matrix_companion_parts(
    accepted: &ProjectDoc,
    document: &mut ProjectDoc,
    positions: &[Position],
) -> Result<Vec<String>, String> {
    let mut changed = BTreeSet::new();
    let Some(hardware) = &accepted.hardware else {
        return Ok(Vec::new());
    };
    for pair in &hardware.pairs {
        let Some(halves) = &pair.halves else {
            continue;
        };
        for link in halves.correspondences.iter().filter(|item| {
            item.kind == MirroredItemKind::Matrix && halves.overlay_of(item).is_none()
        }) {
            let source = matrix_by_id(accepted, &link.source_id)?;
            let Some(mirror) = accepted
                .matrices
                .iter()
                .find(|item| item.id == link.mirror_id)
            else {
                continue;
            };
            for cell in source
                .cells
                .iter()
                .filter(|cell| cell.enabled && !cell.deleted)
            {
                let source_key = crate::matrix::member_id(&source.id, cell.row, cell.column);
                let mirror_key = crate::matrix::member_id(&mirror.id, cell.row, cell.column);
                if halves.correspondences.iter().any(|association| {
                    association.kind == MirroredItemKind::Part
                        && association.source_id == source_key
                        && association.mirror_id == mirror_key
                        && halves.overlay_of(association).is_some()
                }) {
                    continue;
                }
                for assembly in &cell.assemblies {
                    let source_id = crate::matrix::companion_id(&source_key, &assembly.id);
                    let mirror_id = crate::matrix::companion_id(&mirror_key, &assembly.id);
                    let Some(association) = halves.correspondences.iter().find(|association| {
                        association.kind == MirroredItemKind::Part
                            && association.source_id == source_id
                            && association.mirror_id == mirror_id
                    }) else {
                        continue;
                    };
                    if !halves
                        .overlay_of(association)
                        .is_some_and(|exception| exception.state == MirroredExceptionState::Local)
                    {
                        continue;
                    }
                    let Some(saved) = accepted.parts.iter().find(|part| part.id == mirror_id)
                    else {
                        continue;
                    };
                    let Some(current) = document.parts.iter_mut().find(|part| part.id == mirror_id)
                    else {
                        continue;
                    };
                    let mut desired = saved.clone();
                    if let Some(position) =
                        positions.iter().find(|position| position.id == mirror_id)
                    {
                        desired.pose.at = position.at;
                    }
                    if *current != desired {
                        *current = desired;
                        changed.insert(mirror_id);
                    }
                }
            }
        }
    }
    Ok(changed.into_iter().collect())
}

fn reconcile_pairs(
    accepted: &ProjectDoc,
    document: &mut ProjectDoc,
    canonical: &Canonical,
) -> Result<Vec<String>, String> {
    let pair_views = native_pair_views(document);
    let mut changed = Vec::new();
    for pair in pair_views {
        super::validate_local_circuit_targets(document, &pair.halves)?;
        let matrices: Vec<_> = pair
            .halves
            .correspondences
            .iter()
            .filter(|association| association.kind == MirroredItemKind::Matrix)
            .map(|association| (association.source_id.clone(), association.mirror_id.clone()))
            .collect();
        if matrices.is_empty() {
            if matches!(
                canonical.operation,
                EditOperation::AddPart { .. } | EditOperation::AddMirrorOnlyPart { .. }
            ) {
                let mut projection =
                    super::projection_for_matrixless_pair(&pair.halves, pair.mapping);
                projection.independent_part_transitions = transitions_for(
                    accepted,
                    document,
                    canonical,
                    &pair.halves,
                    &pair.halves.source_board_id,
                    &pair.halves.mirror_board_id,
                    false,
                )?
                .0;
                changed.extend(reconcile(document, &pair.pair_id, &projection)?);
            }
            continue;
        }
        for (index, (source_matrix_id, mirror_matrix_id)) in matrices.iter().enumerate() {
            if super::matrix_membership_changed(accepted, document, source_matrix_id) {
                // Lifecycle preparation has already projected this matrix's changed membership.
                // The fixed-membership reconciler owns only matrices whose membership stayed put.
                continue;
            }
            let projection = projection_for(
                accepted,
                document,
                Some(canonical),
                &pair.halves,
                pair.mapping,
                source_matrix_id,
                mirror_matrix_id,
                index == 0,
            )?;
            changed.extend(reconcile(document, &pair.pair_id, &projection)?);
        }
    }
    Ok(changed)
}

/// The forward projector materializes an independent Add using the selected identities. Commit
/// those identities to the native pair record only after projection succeeds, so the prepared
/// draft is linked for subsequent edits while refusals still leave the accepted document intact.
fn record_independent_additions(
    document: &mut ProjectDoc,
    additions: &[inverse::PartAddition],
) -> Result<Vec<String>, String> {
    use crate::model::{MirroredCorrespondence, MirroredItemKind};

    let mut changed = Vec::new();
    for addition in additions {
        let source_board = super::super::board_for_part(document, &addition.source_id)
            .ok_or_else(|| {
                format!(
                    "Independent source Part {:?} has no board owner",
                    addition.source_id
                )
            })?
            .id
            .clone();
        let mirror_board = super::super::board_for_part(document, &addition.mirror_id)
            .ok_or_else(|| {
                format!(
                    "Independent mirror Part {:?} has no board owner",
                    addition.mirror_id
                )
            })?
            .id
            .clone();
        let hardware = document
            .hardware
            .as_mut()
            .ok_or("Mirrored hardware is unavailable for independent Add")?;
        let mut matching_pairs = hardware.pairs.iter_mut().filter(|pair| {
            pair.halves.as_ref().is_some_and(|halves| {
                halves.source_board_id == source_board && halves.mirror_board_id == mirror_board
            })
        });
        let pair = matching_pairs.next().ok_or_else(|| {
            format!(
                "No directed mirrored pair owns independent Add {:?} → {:?}",
                addition.source_id, addition.mirror_id
            )
        })?;
        if matching_pairs.next().is_some() {
            return Err(format!(
                "Independent Add {:?} → {:?} has ambiguous pair ownership",
                addition.source_id, addition.mirror_id
            ));
        }
        let halves = pair
            .halves
            .as_mut()
            .ok_or("The independent Add pair lost its halves record")?;
        if halves.correspondences.iter().any(|record| {
            record.kind == MirroredItemKind::Part
                && (record.source_id == addition.source_id
                    || record.mirror_id == addition.mirror_id)
        }) {
            return Err(format!(
                "Independent Add {:?} → {:?} conflicts with existing pair provenance",
                addition.source_id, addition.mirror_id
            ));
        }
        halves.correspondences.push(MirroredCorrespondence {
            kind: MirroredItemKind::Part,
            source_id: addition.source_id.clone(),
            mirror_id: addition.mirror_id.clone(),
            companion_mode: None,
            companion_tweak: None,
        });
        changed.push(pair.id.clone());
    }
    Ok(changed)
}

#[allow(clippy::too_many_arguments)]
fn projection_for(
    accepted: &ProjectDoc,
    document: &ProjectDoc,
    canonical: Option<&Canonical>,
    halves: &crate::model::MirroredHalves,
    mapping: Pose2,
    source_matrix_id: &str,
    mirror_matrix_id: &str,
    include_transitions: bool,
) -> Result<Projection, String> {
    let source_matrix = matrix_by_id(document, source_matrix_id)?;
    let omitted_matrix = is_omitted_matrix_link(halves, source_matrix_id, mirror_matrix_id);
    let absent_omitted_matrix;
    let mirror_matrix = match matrix_by_id(document, mirror_matrix_id) {
        Ok(matrix) => matrix,
        Err(_) if omitted_matrix => {
            absent_omitted_matrix = omitted_mirror_matrix(
                source_matrix,
                mirror_matrix_id,
                &halves.mirror_board_id,
                halves,
            );
            &absent_omitted_matrix
        }
        Err(error) => return Err(error),
    };
    let source_board_id = source_matrix
        .board_id
        .clone()
        .ok_or("The source matrix has no board owner")?;
    let mirror_board_id = mirror_matrix
        .board_id
        .clone()
        .ok_or("The mirror matrix has no board owner")?;
    let mut parts = Vec::new();
    let mut locally_protected_mirror_parts = BTreeSet::new();
    // Members introduced under a detached matrix have no mirror target yet;
    // an explicit Rejoin adopts them. Any other missing correspondence stays
    // a refusal.
    let matrix_detached = halves
        .correspondences
        .iter()
        .find(|record| {
            record.kind == MirroredItemKind::Matrix
                && record.source_id == source_matrix_id
                && record.mirror_id == mirror_matrix_id
        })
        .is_some_and(|record| {
            halves.overlay_of(record).is_some_and(|exception| {
                matches!(
                    exception.state,
                    crate::model::MirroredExceptionState::Local
                        | crate::model::MirroredExceptionState::Omitted
                )
            })
        });
    for cell in source_matrix
        .cells
        .iter()
        .filter(|cell| cell.enabled && !cell.deleted)
    {
        let source_id = format!("matrix/{source_matrix_id}/r{}c{}", cell.row, cell.column);
        let association = match part_association(halves, &source_id) {
            Ok(association) => association,
            Err(_) if matrix_detached => continue,
            Err(error) => return Err(error),
        };
        if association.mirror_id
            != format!("matrix/{mirror_matrix_id}/r{}c{}", cell.row, cell.column)
        {
            return Err(format!(
                "Matrix part association for {source_id:?} has stale identity"
            ));
        }
        if halves
            .overlay_of(association)
            .is_some_and(|exception| exception.state == MirroredExceptionState::Local)
        {
            locally_protected_mirror_parts.insert(association.mirror_id.clone());
        } else {
            parts.push(PartProjection {
                source_id,
                mirror_id: association.mirror_id.clone(),
            });
        }
    }
    let mut final_owners = BTreeMap::new();
    for (matrix, owner) in [
        (source_matrix, source_board_id.clone()),
        (mirror_matrix, mirror_board_id.clone()),
    ] {
        for id in &matrix.part_ids {
            final_owners.insert(id.clone(), owner.clone());
        }
    }
    let (independent_part_transitions, hosted_transitions, hosted_removal_baselines) =
        match canonical {
            Some(canonical) => transitions_for(
                accepted,
                document,
                canonical,
                halves,
                &source_board_id,
                &mirror_board_id,
                include_transitions,
            )?,
            None if !include_transitions => (Vec::new(), Vec::new(), BTreeMap::new()),
            None => return Err("Edit projection requires canonical intent".into()),
        };
    let applied_independent_additions = if include_transitions {
        BTreeSet::new()
    } else {
        independent_part_transitions
            .iter()
            .filter_map(|transition| match transition {
                IndependentPartTransition::Add {
                    source_id,
                    mirror_id,
                    mirror_definition_id,
                    mirror_reference,
                } => Some((
                    source_id.clone(),
                    mirror_id.clone(),
                    mirror_definition_id.clone(),
                    mirror_reference.clone(),
                )),
                IndependentPartTransition::Remove { .. } => None,
            })
            .collect()
    };
    Ok(Projection {
        matrix: Some(super::MatrixLink {
            source_matrix_id: source_matrix_id.to_owned(),
            mirror_matrix_id: mirror_matrix_id.to_owned(),
        }),
        preserve_matrix_geometry: false,
        source_board_id,
        mirror_board_id,
        mapping,
        parts,
        independent_part_transitions,
        hosted_transitions,
        applied_independent_additions: std::cell::RefCell::new(applied_independent_additions),
        applied_hosted_operations: std::cell::RefCell::new(BTreeSet::new()),
        applied_hosted_circuit_additions: std::cell::RefCell::new(BTreeMap::new()),
        hosted_removal_baselines: std::cell::RefCell::new(hosted_removal_baselines),
        hosted_omission_baselines: std::cell::RefCell::new(BTreeMap::new()),
        outline_removal_baselines: std::cell::RefCell::new(BTreeMap::new()),
        locally_protected_mirror_parts,
        final_owners,
    })
}

fn matrix_by_id<'a>(document: &'a ProjectDoc, id: &str) -> Result<&'a Matrix, String> {
    let mut matrices = document.matrices.iter().filter(|matrix| matrix.id == id);
    let matrix = matrices
        .next()
        .ok_or_else(|| format!("Mirrored matrix {id:?} is unavailable"))?;
    if matrices.next().is_some() {
        return Err(format!("Mirrored matrix {id:?} is ambiguous"));
    }
    Ok(matrix)
}

fn part_association<'a>(
    halves: &'a crate::model::MirroredHalves,
    source_id: &str,
) -> Result<&'a crate::model::MirroredCorrespondence, String> {
    let mut associations = halves.correspondences.iter().filter(|association| {
        association.kind == MirroredItemKind::Part && association.source_id == source_id
    });
    let association = associations
        .next()
        .ok_or_else(|| format!("Matrix part {source_id:?} has no correspondence"))?;
    if associations.next().is_some() {
        return Err(format!(
            "Matrix part {source_id:?} has ambiguous correspondence"
        ));
    }
    Ok(association)
}

type RemovalBaselines = BTreeMap<(String, String, String), ProjectDoc>;

fn transitions_for(
    accepted: &ProjectDoc,
    document: &ProjectDoc,
    canonical: &Canonical,
    halves: &crate::model::MirroredHalves,
    source_board_id: &str,
    mirror_board_id: &str,
    include_removals_and_hosted: bool,
) -> Result<
    (
        Vec<IndependentPartTransition>,
        Vec<HostedTransition>,
        RemovalBaselines,
    ),
    String,
> {
    let mut parts = Vec::new();
    for addition in &canonical.additions {
        if super::super::board_for_part(document, &addition.source_id)
            .is_some_and(|board| board.id == source_board_id)
        {
            parts.push(IndependentPartTransition::Add {
                source_id: addition.source_id.clone(),
                mirror_id: addition.mirror_id.clone(),
                mirror_definition_id: addition.mirror_definition_id.clone(),
                mirror_reference: addition.mirror_reference.clone(),
            });
        }
    }
    if include_removals_and_hosted && let EditOperation::RemoveParts { ids } = &canonical.operation
    {
        for source_id in ids {
            if let Some(association) = halves.correspondences.iter().find(|association| {
                association.kind == MirroredItemKind::Part && association.source_id == *source_id
            }) {
                parts.push(IndependentPartTransition::Remove {
                    source_id: source_id.clone(),
                    mirror_id: association.mirror_id.clone(),
                });
            }
        }
    }

    let mut hosted = Vec::new();
    let mut removal_baselines = BTreeMap::new();
    if !include_removals_and_hosted {
        return Ok((parts, hosted, removal_baselines));
    }

    for addition in &canonical.hosted.modules {
        let owner = document
            .modules
            .iter()
            .find(|module| module.id == addition.source_id)
            .map(|module| module.host_board_id.as_str());
        if owner == Some(source_board_id) {
            hosted.push(HostedTransition::AddModule {
                source_id: addition.source_id.clone(),
                mirror_id: addition.mirror_id.clone(),
            });
        }
    }
    for addition in &canonical.hosted.circuits {
        let owner = document
            .embedded_circuits
            .iter()
            .find(|circuit| circuit.id == addition.source_id)
            .map(|circuit| circuit.host_board_id.as_str());
        if owner == Some(source_board_id) {
            hosted.push(HostedTransition::AddCircuit {
                source_id: addition.source_id.clone(),
                mirror_id: addition.mirror_id.clone(),
                member_ids: addition.member_ids.clone(),
                member_references: addition.member_references.clone(),
                definition_ids: addition.definition_ids.clone(),
                net_ids: addition.net_ids.clone(),
                member_poses: addition.member_poses.clone(),
            });
        }
    }
    match &canonical.operation {
        EditOperation::RemoveMountedModule { id } => {
            if let Some(association) = hosted_association(halves, MirroredItemKind::Module, id)
                && module_owner(accepted, id) == Some(source_board_id)
            {
                hosted.push(HostedTransition::RemoveModule {
                    source_id: id.clone(),
                    mirror_id: association.mirror_id.clone(),
                });
                removal_baselines.insert(
                    (
                        "remove-module".into(),
                        id.clone(),
                        association.mirror_id.clone(),
                    ),
                    accepted.clone(),
                );
            }
        }
        EditOperation::RemoveEmbeddedCircuit { id } => {
            if let Some(association) =
                hosted_association(halves, MirroredItemKind::EmbeddedCircuit, id)
                && circuit_owner(accepted, id) == Some(source_board_id)
            {
                hosted.push(HostedTransition::RemoveCircuit {
                    source_id: id.clone(),
                    mirror_id: association.mirror_id.clone(),
                });
                removal_baselines.insert(
                    (
                        "remove-circuit".into(),
                        id.clone(),
                        association.mirror_id.clone(),
                    ),
                    accepted.clone(),
                );
            }
        }
        _ => {}
    }
    let _ = mirror_board_id;
    Ok((parts, hosted, removal_baselines))
}

fn hosted_association<'a>(
    halves: &'a crate::model::MirroredHalves,
    kind: MirroredItemKind,
    source_id: &str,
) -> Option<&'a crate::model::MirroredCorrespondence> {
    halves
        .correspondences
        .iter()
        .find(|association| association.kind == kind && association.source_id == source_id)
}

fn module_owner<'a>(document: &'a ProjectDoc, id: &str) -> Option<&'a str> {
    document
        .modules
        .iter()
        .find(|module| module.id == id)
        .map(|module| module.host_board_id.as_str())
}

fn circuit_owner<'a>(document: &'a ProjectDoc, id: &str) -> Option<&'a str> {
    document
        .embedded_circuits
        .iter()
        .find(|circuit| circuit.id == id)
        .map(|circuit| circuit.host_board_id.as_str())
}

fn entity_changed(before: &ProjectDoc, after: &ProjectDoc, id: &str) -> Result<bool, String> {
    fn entities(
        document: &ProjectDoc,
        id: &str,
    ) -> Result<BTreeMap<&'static str, serde_json::Value>, String> {
        let value = serde_json::to_value(document).map_err(|error| error.to_string())?;
        let root = value
            .as_object()
            .ok_or_else(|| "The prepared document did not serialize as an object".to_owned())?;
        // A physical instance may share its board's id. Keep each domain owner in
        // the comparison so an unchanged Board cannot hide changed instance data.
        let mut matches = BTreeMap::new();
        for key in [
            "parts",
            "matrices",
            "boards",
            "definitions",
            "nets",
            "constraints",
            "outline",
            "layouts",
            "modules",
            "embeddedCircuits",
            "caseBodies",
        ] {
            if let Some(item) = root
                .get(key)
                .and_then(serde_json::Value::as_array)
                .and_then(|items| {
                    items
                        .iter()
                        .find(|item| item.get("id").and_then(serde_json::Value::as_str) == Some(id))
                })
            {
                matches.insert(key, item.clone());
            }
        }
        if let Some(hardware) = root.get("hardware") {
            if let Some(pairs) = hardware.get("pairs").and_then(serde_json::Value::as_array) {
                let owned: Vec<_> = pairs
                    .iter()
                    .filter(|pair| {
                        pair.get("members")
                            .and_then(serde_json::Value::as_array)
                            .is_some_and(|members| {
                                members.iter().any(|member| member.as_str() == Some(id))
                            })
                    })
                    .cloned()
                    .collect();
                if !owned.is_empty() {
                    // Shared construction and linked features change a member's
                    // effective configuration without changing its local slot.
                    matches.insert("hardware.memberPairs", serde_json::Value::Array(owned));
                }
            }
            for (key, owner) in [
                ("pairs", "hardware.pairs"),
                ("instances", "hardware.instances"),
            ] {
                if let Some(item) = hardware
                    .get(key)
                    .and_then(serde_json::Value::as_array)
                    .and_then(|items| {
                        items.iter().find(|item| {
                            item.get("id").and_then(serde_json::Value::as_str) == Some(id)
                        })
                    })
                {
                    matches.insert(owner, item.clone());
                }
            }
            if let Some(item) = hardware
                .get("boards")
                .and_then(serde_json::Value::as_array)
                .and_then(|items| {
                    items.iter().find(|item| {
                        item.get("boardId").and_then(serde_json::Value::as_str) == Some(id)
                    })
                })
            {
                matches.insert("hardware.boards", item.clone());
            }
        }
        Ok(matches)
    }
    Ok(entities(before, id)? != entities(after, id)?)
}
