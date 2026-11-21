//! Composition of inverse translation and the existing forward kernels (ticket 115).
//!
//! This module owns disposable drafts. It does not publish a revision or history entry; the edit
//! lifecycle publishes drafts returned by `prepare_transaction_with_ledger` and `prepare_open`.

use super::changes::ChangeScope;
use super::document::reconcile_with_report;
use super::{
    HostedTransition, IndependentPartTransition, PartProjection, Projection,
    is_omitted_matrix_link, omitted_mirror_matrix, unique_board_owner,
};
use crate::constraints::FramedConstraint;
use crate::mirrored_half::inverse::{self, Canonical, PreparedConstraintEdit};
use crate::model::{
    BoardContours, CompanionPlacementTarget, EditOperation, Finding, HalfExceptionState, Matrix,
    MirroredCompanionMode, MirroredCompanionTweak, MirroredExceptionState, MirroredItemKind, Pose2,
    Position, ProjectDoc, Scope, Severity,
};
use std::collections::{BTreeMap, BTreeSet};

mod canonical;
mod projection;
mod settings_edit;
#[cfg(test)]
mod tests;
pub use canonical::apply_canonical;
use canonical::{
    bind_deferred_constraints, initial_framed_constraints, operation_positions,
    prepare_lifecycle_pairs, record_id, restore_local_matrix_companion_parts,
};
use projection::{PairProjection, projection_for, reconcile_pairs, record_independent_additions};

/// Builds the source-authoritative document draft used by Open without publishing it.
/// Open has its own route: it does not canonicalize an edit or use transaction preparation,
/// whose ReplaceDocument behavior intentionally preserves replacement geometry.
pub fn prepare_open(
    accepted: &ProjectDoc,
    _board_contours: &[BoardContours],
) -> Result<(ProjectDoc, Vec<Finding>), String> {
    let mut draft = accepted.clone();
    // Legacy scripts are accepted for compatibility but never executed or retained.
    draft.scripts.clear();
    if let Some(keymap) = &draft.keymap {
        crate::keymap::validate(keymap)?;
    }
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
    for pair in &pair_views {
        let pair_projection = PairProjection::new(&pair.halves, pair.mapping);
        let mut matrices = pair_projection.matrix_links().peekable();
        if matrices.peek().is_none() {
            let projection = super::projection_for_matrixless_pair(&pair.halves, pair.mapping);
            reconcile_with_report(&mut draft, &pair.pair_id, &projection, None)?;
        }
        for matrix_link in matrices {
            let projection =
                projection_for(&draft, &draft, None, &pair_projection, matrix_link, false)?;
            reconcile_with_report(&mut draft, &pair.pair_id, &projection, None)?;
        }
    }
    crate::outline_controls::detach_removed(&accepted.parts, &mut draft);
    crate::outline_versions::prune_removed_components(&accepted.parts, &mut draft);
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
            crate::mirrored_half::derive::is_controller(
                &document.definitions,
                &source.definition_id,
            ),
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

/// Builds a complete document draft while consulting the owning session's identity
/// reservations, so a later addition never reuses an identity hidden by Undo or a
/// disabled configuration.
pub fn prepare_transaction_with_ledger(
    accepted: &ProjectDoc,
    board_contours: &[BoardContours],
    operation: &EditOperation,
    identities: &crate::mechanical::edit::MechanicalIdentityLedger,
    dispatch: crate::mirrored_half::EditDispatch,
) -> Result<(ProjectDoc, Vec<String>), String> {
    prepare_transaction(
        accepted,
        board_contours,
        operation,
        identities,
        dispatch,
        true,
    )
}

fn prepare_transaction(
    accepted: &ProjectDoc,
    board_contours: &[BoardContours],
    operation: &EditOperation,
    identities: &crate::mechanical::edit::MechanicalIdentityLedger,
    dispatch: crate::mirrored_half::EditDispatch,
    directed: bool,
) -> Result<(ProjectDoc, Vec<String>), String> {
    crate::mechanical::halves::validate_document(accepted)?;
    let mut canonical = inverse::canonicalize_with_dispatch(accepted, operation, dispatch)
        .map_err(|refusal| refusal.to_string())?;
    if let EditOperation::ReplaceDocument { document } = &mut canonical.operation {
        super::lifecycle::retire_replacement_records(accepted, document)?;
    }
    let mut draft = accepted.clone();
    let mut changed = apply_canonical(
        &mut draft,
        accepted,
        board_contours,
        &canonical,
        identities,
        dispatch,
    )?;

    if let Some(keymap) = &draft.keymap {
        crate::keymap::validate(keymap)?;
    }
    crate::electrical::preserve_handoff(accepted, &mut draft);
    crate::matrix::layout::validate(&draft)?;

    let positions = operation_positions(&canonical.operation);
    let mut framed = initial_framed_constraints(&canonical);
    crate::matrix::layout::resolve(&mut draft)?;
    changed.extend(crate::constraints::resolve_prepared(
        &mut draft, positions, &framed,
    )?);
    changed.extend(crate::matrix::layout::sync_components(&mut draft)?);

    if directed
        && let Some(changed) = settings_edit::project(accepted, &mut draft, &canonical.operation)?
    {
        validate_prepared_draft(accepted, &draft, &canonical.operation)?;
        return Ok((draft, changed));
    }
    let report = if directed && stable_placement(accepted, &draft, &canonical.operation) {
        ChangeScope::Placement
    } else {
        ChangeScope::Full
    };
    // Lifecycle projects matrixless pairs and supplies the poses used to retune constraints.
    changed.extend(prepare_lifecycle_pairs(
        accepted, &mut draft, &canonical, dispatch,
    )?);

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
    // Caller-owned draft: reconcile each pair in place without per-pair full reports. The
    // outer transaction guarantees rollback and the single final scoped report below owns
    // the ordered changed IDs.
    reconcile_pairs(accepted, &mut draft, &canonical)?;
    // Ticket 29: a plain detach that moved its linked native partner
    // refuses with the ticket-23 confirmation proposal instead of silently
    // jumping geometry; the draft is discarded unchanged.
    if let EditOperation::DetachMatrixAnchor { matrix_id } = &canonical.operation
        && let Some(proposal) =
            super::lifecycle::native_detach_proposal(accepted, &draft, matrix_id)
    {
        return Err(proposal);
    }
    changed.extend(add_target_layout_memberships(
        &mut draft,
        &canonical.additions,
    )?);
    changed.extend(record_independent_additions(
        &mut draft,
        &canonical.additions,
    )?);
    crate::outline_controls::detach_removed(&accepted.parts, &mut draft);
    changed.extend(crate::outline_versions::prune_removed_components(
        &accepted.parts,
        &mut draft,
    ));

    resolve_mirror_replacement_electrical(accepted, &mut draft, &canonical.operation)?;

    validate_prepared_draft(accepted, &draft, &canonical.operation)?;

    let changed = report.ids(accepted, &draft, &changed)?;
    Ok((draft, changed))
}

fn stable_placement(before: &ProjectDoc, after: &ProjectDoc, operation: &EditOperation) -> bool {
    fn matrix_membership(matrix: &Matrix) -> (&str, Option<&str>, u32, u32, &[String]) {
        (
            &matrix.id,
            matrix.board_id.as_deref(),
            matrix.rows,
            matrix.columns,
            &matrix.part_ids,
        )
    }

    // A SetLayout/SetMatrix payload can carry structural intent even though the
    // same operation is used by ordinary renderer placement gestures.
    matches!(
        operation,
        EditOperation::MoveParts { .. }
            | EditOperation::SetPartRotation { .. }
            | EditOperation::SetPartAnchor { .. }
            | EditOperation::SetPartAnchorFollow { .. }
            | EditOperation::SetPartAnchorValues { .. }
            | EditOperation::DetachPartAnchor { .. }
            | EditOperation::SetMatrixPlacement { .. }
            | EditOperation::SetMatrixSplay { .. }
            | EditOperation::SetMatrix {
                definitions: None,
                ..
            }
            | EditOperation::SetLayout { .. }
            | EditOperation::SetCompanionPlacement { .. }
    ) && before
        .layouts
        .iter()
        .zip(&after.layouts)
        .all(|(left, right)| {
            left.mirror_link.is_none()
                && right.mirror_link.is_none()
                && left.id == right.id
                && left.board_id == right.board_id
                && left.part_ids == right.part_ids
                && left.matrix_ids == right.matrix_ids
        })
        && before.layouts.len() == after.layouts.len()
        && before
            .boards
            .iter()
            .map(|board| (&board.id, &board.part_ids))
            .eq(after
                .boards
                .iter()
                .map(|board| (&board.id, &board.part_ids)))
        && before
            .matrices
            .iter()
            .map(matrix_membership)
            .eq(after.matrices.iter().map(matrix_membership))
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

#[doc(hidden)]
pub fn validate_prepared_draft(
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
