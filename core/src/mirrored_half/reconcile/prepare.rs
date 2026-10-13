//! Inert composition of inverse translation and the existing forward kernels (ticket 115).
//!
//! This module owns a disposable draft. It does not publish a revision or history entry; ticket
//! 111 will connect the single preparation entry point to the normal edit lifecycle.

use super::{
    HostedTransition, IndependentPartTransition, PartProjection, Projection,
    is_omitted_matrix_link, omitted_mirror_matrix, reconcile, unique_board_owner,
};
use crate::constraints::FramedConstraint;
use crate::mirrored_half::inverse::{self, Canonical, PreparedConstraintEdit};
use crate::model::{
    BoardContours, EditOperation, Finding, Matrix, MirroredExceptionState, MirroredItemKind, Pose2,
    Position, ProjectDoc, Scope, Severity,
};
use std::collections::{BTreeMap, BTreeSet};

#[cfg(test)]
#[path = "prepare/tests.rs"]
mod tests;

/// Builds the source-authoritative document draft used by Open without publishing it.
/// Open has its own route: it does not canonicalize an edit or use transaction preparation,
/// whose ReplaceDocument behavior intentionally preserves replacement geometry.
fn prepare_open(
    accepted: &ProjectDoc,
    _board_contours: &[BoardContours],
) -> Result<(ProjectDoc, Vec<Finding>), String> {
    let mut draft = accepted.clone();
    if let Some(keymap) = &draft.keymap {
        crate::keymap::validate(keymap)?;
    }
    crate::script::apply_scripts(&mut draft)?;
    crate::matrix::layout::resolve(&mut draft)?;
    crate::constraints::resolve(&mut draft)?;
    crate::matrix::layout::sync_components(&mut draft)?;
    crate::constraints::resolve(&mut draft)?;
    crate::matrix::layout::sync_components(&mut draft)?;
    crate::generators::document_error(&draft)?;

    let pair_views = native_pair_views(&draft);
    for pair in &pair_views {
        seed_missing_independent_parts(&mut draft, &pair.halves, pair.mapping)?;
    }
    // Seeding accepts only the narrowly recoverable independent-Part absence. All other
    // malformed, ambiguous, Local/Omitted or unsupported missing records remain strict refusals.
    crate::mechanical::halves::validate_document(&draft)?;

    for pair in pair_views.iter().cloned() {
        let matrices: Vec<_> = pair
            .halves
            .correspondences
            .iter()
            .filter(|association| association.kind == MirroredItemKind::Matrix)
            .map(|association| (association.source_id.clone(), association.mirror_id.clone()))
            .collect();
        if matrices.is_empty() {
            let projection = projection_for_matrixless_pair(&pair.halves, pair.mapping);
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

fn projection_for_matrixless_pair(
    halves: &crate::model::MirroredHalves,
    mapping: Pose2,
) -> Projection {
    Projection {
        matrix: None,
        preserve_matrix_geometry: true,
        source_board_id: halves.source_board_id.clone(),
        mirror_board_id: halves.mirror_board_id.clone(),
        mapping,
        parts: Vec::new(),
        independent_part_transitions: Vec::new(),
        hosted_transitions: Vec::new(),
        applied_independent_additions: std::cell::RefCell::new(BTreeSet::new()),
        applied_hosted_operations: std::cell::RefCell::new(BTreeSet::new()),
        applied_hosted_circuit_additions: std::cell::RefCell::new(BTreeMap::new()),
        hosted_removal_baselines: std::cell::RefCell::new(BTreeMap::new()),
        hosted_omission_baselines: std::cell::RefCell::new(BTreeMap::new()),
        outline_removal_baselines: std::cell::RefCell::new(BTreeMap::new()),
        locally_protected_mirror_parts: BTreeSet::new(),
        final_owners: BTreeMap::new(),
    }
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

/// Builds a complete document draft for one edit without changing the accepted document.
pub(crate) fn prepare_transaction(
    accepted: &ProjectDoc,
    board_contours: &[BoardContours],
    operation: &EditOperation,
) -> Result<(ProjectDoc, Vec<String>), String> {
    crate::mechanical::halves::validate_document(accepted)?;
    let canonical =
        inverse::canonicalize(accepted, operation).map_err(|refusal| refusal.to_string())?;
    let mut draft = accepted.clone();
    let mut changed = apply_canonical(&mut draft, accepted, board_contours, &canonical)?;

    if let Some(keymap) = &draft.keymap {
        crate::keymap::validate(keymap)?;
    }
    crate::electrical::preserve_handoff(accepted, &mut draft);
    crate::matrix::layout::validate(&draft)?;

    let positions = operation_positions(&canonical.operation);
    let mut framed = initial_framed_constraints(&canonical);
    changed.extend(resolve_with_frames(&mut draft, positions, &framed)?);
    changed.extend(crate::matrix::layout::sync_components(&mut draft)?);

    changed.extend(prepare_lifecycle_pairs(accepted, &mut draft, &canonical)?);

    for addition in &canonical.matrix_additions {
        let mirrored =
            crate::mirrored_half::matrix_addition::mirror_added_matrix(&draft, addition)?;
        draft = mirrored.draft;
        changed.extend(mirrored.changed_ids);
    }

    framed = bind_deferred_constraints(&draft, &canonical, &framed)?;
    changed.extend(resolve_with_frames(&mut draft, positions, &framed)?);
    changed.extend(crate::matrix::layout::sync_components(&mut draft)?);

    changed.extend(reconcile_pairs(accepted, &mut draft, &canonical)?);
    changed.extend(record_independent_additions(
        &mut draft,
        &canonical.additions,
    )?);
    crate::outline_controls::detach_removed(&accepted.parts, &mut draft);
    changed.extend(crate::prune_removed_components(&accepted.parts, &mut draft));

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
    Ok(())
}

fn prepare_lifecycle_pairs(
    accepted: &ProjectDoc,
    document: &mut ProjectDoc,
    canonical: &Canonical,
) -> Result<Vec<String>, String> {
    if matches!(
        canonical.operation,
        EditOperation::ApplyMechanicalEdit { .. }
            | EditOperation::CreateMirroredPair { .. }
            | EditOperation::CreateMirroredHalf { .. }
            | EditOperation::ReplaceDocument { .. }
    ) {
        return Ok(Vec::new());
    }
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
        if !document
            .hardware
            .as_ref()
            .is_some_and(|hardware| hardware.pairs.iter().any(|pair| pair.id == pair_id))
        {
            continue;
        }
        for output in super::lifecycle::prepare_edit_transition(
            accepted,
            document,
            &pair_id,
            &canonical.operation,
        )? {
            changed.extend(output.changed_document_ids);
            *document = output.draft;
        }
    }
    Ok(changed)
}

fn apply_canonical(
    draft: &mut ProjectDoc,
    accepted: &ProjectDoc,
    board_contours: &[BoardContours],
    canonical: &Canonical,
) -> Result<Vec<String>, String> {
    if let EditOperation::ApplyMechanicalEdit { edit } = &canonical.operation
        && let crate::mechanical::MechanicalChange::UnlinkMirrorPair { instance_id } = &edit.change
    {
        let pair_id = pair_for_instance(accepted, instance_id)?;
        return crate::mechanical::halves::unlink_pair(draft, &pair_id);
    }
    crate::apply(draft, board_contours, &canonical.operation)
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

fn resolve_with_frames(
    document: &mut ProjectDoc,
    positions: &[Position],
    framed: &[FramedConstraint],
) -> Result<Vec<String>, String> {
    let replaced_ids: BTreeSet<_> = framed.iter().map(record_id).collect();
    if replaced_ids.is_empty() {
        return crate::constraints::resolve_prepared(document, positions, framed);
    }
    let stored = std::mem::take(&mut document.constraints);
    document.constraints = stored
        .iter()
        .filter(|constraint| !replaced_ids.contains(constraint.id()))
        .cloned()
        .collect();
    let resolved = crate::constraints::resolve_prepared(document, positions, framed);
    document.constraints = stored;
    resolved
}

fn reconcile_pairs(
    accepted: &ProjectDoc,
    document: &mut ProjectDoc,
    canonical: &Canonical,
) -> Result<Vec<String>, String> {
    let pair_views = native_pair_views(document);
    let mut changed = Vec::new();
    for pair in pair_views {
        let matrices: Vec<_> = pair
            .halves
            .correspondences
            .iter()
            .filter(|association| association.kind == MirroredItemKind::Matrix)
            .map(|association| (association.source_id.clone(), association.mirror_id.clone()))
            .collect();
        for (index, (source_matrix_id, mirror_matrix_id)) in matrices.iter().enumerate() {
            if matrix_membership_changed(accepted, document, source_matrix_id) {
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

fn matrix_membership_changed(before: &ProjectDoc, after: &ProjectDoc, id: &str) -> bool {
    let before = before
        .matrices
        .iter()
        .find(|matrix| matrix.id == id)
        .map(|matrix| &matrix.part_ids);
    let after = after
        .matrices
        .iter()
        .find(|matrix| matrix.id == id)
        .map(|matrix| &matrix.part_ids);
    before != after
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
    for cell in source_matrix
        .cells
        .iter()
        .filter(|cell| cell.enabled && !cell.deleted)
    {
        let source_id = format!("matrix/{source_matrix_id}/r{}c{}", cell.row, cell.column);
        let association = part_association(halves, &source_id)?;
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
        if include_transitions {
            let canonical = canonical.ok_or("Edit projection requires canonical intent")?;
            transitions_for(
                accepted,
                document,
                canonical,
                halves,
                &source_board_id,
                &mirror_board_id,
            )?
        } else {
            (Vec::new(), Vec::new(), BTreeMap::new())
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
        applied_independent_additions: std::cell::RefCell::new(BTreeSet::new()),
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
    if let EditOperation::RemoveParts { ids } = &canonical.operation {
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
            });
        }
    }
    let mut removal_baselines = BTreeMap::new();
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
    fn entity(document: &ProjectDoc, id: &str) -> Result<Option<serde_json::Value>, String> {
        let value = serde_json::to_value(document).map_err(|error| error.to_string())?;
        let root = value
            .as_object()
            .ok_or_else(|| "The prepared document did not serialize as an object".to_owned())?;
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
                return Ok(Some(item.clone()));
            }
        }
        if let Some(hardware) = root.get("hardware") {
            for key in ["pairs", "instances"] {
                if let Some(item) = hardware
                    .get(key)
                    .and_then(serde_json::Value::as_array)
                    .and_then(|items| {
                        items.iter().find(|item| {
                            item.get("id").and_then(serde_json::Value::as_str) == Some(id)
                        })
                    })
                {
                    return Ok(Some(item.clone()));
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
                return Ok(Some(item.clone()));
            }
        }
        Ok(None)
    }
    Ok(entity(before, id)? != entity(after, id)?)
}
