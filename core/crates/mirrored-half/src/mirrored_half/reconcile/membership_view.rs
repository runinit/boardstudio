//! Fixed matrix membership views: canonical slots, implicit cells and omitted companions.

use super::{Projection, RetainedCell, unique_board_owner};
use crate::model::{Matrix, MirroredExceptionState, MirroredItemKind, Part, ProjectDoc, Vec2};
use std::collections::{BTreeMap, BTreeSet};

#[doc(hidden)]
pub struct FixedMembershipView {
    #[doc(hidden)]
    pub normalized_source_matrix: Matrix,
    #[doc(hidden)]
    pub incoming_matrix: Matrix,
    pub(super) retained_cells: Vec<RetainedCell>,
    pub(super) retained_companion_parts: Vec<Part>,
    pub(super) omitted_parts: BTreeSet<String>,
    pub(super) omitted_matrix_companions: BTreeSet<String>,
}

pub(super) struct OmittedMatrixCompanion {
    pub(super) source_id: String,
    pub(super) mirror_id: String,
    pub(super) source_key_id: String,
    pub(super) mirror_key_id: String,
}

pub(super) fn exact_omitted_matrix_companions(
    source: &Matrix,
    mirror: &Matrix,
    halves: &crate::model::MirroredHalves,
) -> Result<Vec<OmittedMatrixCompanion>, String> {
    if !halves.exceptions.iter().any(|exception| {
        exception.kind == MirroredItemKind::Part
            && exception.state == MirroredExceptionState::Omitted
    }) {
        return Ok(Vec::new());
    }
    let mut omitted = Vec::new();
    for source_cell in source
        .cells
        .iter()
        .filter(|cell| cell.enabled && !cell.deleted)
    {
        let source_key_id =
            crate::matrix::member_id(&source.id, source_cell.row, source_cell.column);
        let mirror_key_id =
            crate::matrix::member_id(&mirror.id, source_cell.row, source_cell.column);
        for assembly in &source_cell.assemblies {
            let source_id = crate::matrix::companion_id(&source_key_id, &assembly.id);
            let mirror_id = crate::matrix::companion_id(&mirror_key_id, &assembly.id);
            let mut associations = halves.correspondences.iter().filter(|association| {
                association.kind == MirroredItemKind::Part
                    && (association.source_id == source_id || association.mirror_id == mirror_id)
            });
            let Some(association) = associations.next() else {
                continue;
            };
            if associations.next().is_some()
                || association.source_id != source_id
                || association.mirror_id != mirror_id
            {
                continue;
            }
            let mut overlays = halves.exceptions.iter().filter(|exception| {
                exception.kind == MirroredItemKind::Part && exception.mirror_id == mirror_id
            });
            let Some(overlay) = overlays.next() else {
                continue;
            };
            if overlays.next().is_some()
                || overlay.state != MirroredExceptionState::Omitted
                || halves.overlay_source(overlay) != Some(source_id.as_str())
            {
                continue;
            }
            let mirror_cells: Vec<_> = mirror
                .cells
                .iter()
                .filter(|cell| {
                    cell.enabled
                        && !cell.deleted
                        && (cell.row, cell.column) == (source_cell.row, source_cell.column)
                })
                .collect();
            let [mirror_cell] = mirror_cells.as_slice() else {
                return Err("An omitted matrix companion has no unique active mirror slot".into());
            };
            if !mirror_cell
                .assemblies
                .iter()
                .any(|candidate| candidate.id == assembly.id)
            {
                return Err("An omitted matrix companion has no mirror assembly slot".into());
            }
            omitted.push(OmittedMatrixCompanion {
                source_id: source_id.clone(),
                mirror_id,
                source_key_id: source_key_id.clone(),
                mirror_key_id: mirror_key_id.clone(),
            });
        }
    }
    Ok(omitted)
}

pub(super) fn remove_omitted_matrix_parts(
    document: &mut ProjectDoc,
    matrix_id: &str,
    omitted: &BTreeSet<String>,
) {
    if omitted.is_empty() {
        return;
    }
    crate::matrix::deletion::remove_parts_by_id(document, omitted);
    if let Some(matrix) = document
        .matrices
        .iter_mut()
        .find(|matrix| matrix.id == matrix_id)
    {
        matrix.part_ids.retain(|id| !omitted.contains(id));
    }
}

#[doc(hidden)]
pub fn build_fixed_membership_view(
    document: &ProjectDoc,
    halves: &crate::model::MirroredHalves,
    source: &Matrix,
    mirror: &Matrix,
    projected: &Matrix,
    projection: &Projection,
) -> Result<FixedMembershipView, String> {
    fixed_membership_view(
        document,
        halves,
        source,
        mirror,
        projected,
        projection.mapping,
        Some(projection),
    )
}

pub(super) fn fixed_membership_view(
    document: &ProjectDoc,
    halves: &crate::model::MirroredHalves,
    source: &Matrix,
    mirror: &Matrix,
    projected: &Matrix,
    mapping: crate::model::Pose2,
    explicit: Option<&Projection>,
) -> Result<FixedMembershipView, String> {
    // Missing coordinates use the same empty active-cell defaults as matrix materialization.
    // Explicit lifecycle tombstones remain inactive.
    let explicit_source_cells = &source.cells;
    let source = fill_implicit_active_cells(source)?;
    let mirror = fill_implicit_active_cells(mirror)?;
    let mut projected = fill_implicit_active_cells(projected)?;
    let inactive = inactive_slots(&source);
    // Explicitly Omitted mirror slots stay suppressed while their source slot
    // stays live (spec §5): only unexplained asymmetry refuses here. The
    // strict per-slot provenance check below still applies.
    let overlaid_omitted_slots: BTreeSet<Slot> = mirror
        .cells
        .iter()
        .filter(|cell| cell.deleted && !cell.enabled)
        .filter(|cell| {
            let mirror_id = format!("matrix/{}/r{}c{}", mirror.id, cell.row, cell.column);
            let source_id = format!("matrix/{}/r{}c{}", source.id, cell.row, cell.column);
            halves.exceptions.iter().any(|exception| {
                exception.kind == MirroredItemKind::Part
                    && exception.mirror_id == mirror_id
                    && exception.state == crate::model::MirroredExceptionState::Omitted
                    && halves.overlay_source(exception) == Some(source_id.as_str())
            })
        })
        .map(|cell| (cell.row, cell.column))
        .collect();
    let mirror_inactive: BTreeSet<Slot> = inactive_slots(&mirror)
        .difference(&overlaid_omitted_slots)
        .copied()
        .collect();
    if inactive != mirror_inactive || inactive != inactive_slots(&projected) {
        return Err("The source and mirror matrices have inconsistent inactive slots".into());
    }
    let mut tombstoned_member_ids = BTreeSet::new();
    for (matrix, slots) in [(&source, &inactive), (&mirror, &inactive)] {
        for (row, column) in slots {
            tombstoned_member_ids.insert(format!("matrix/{}/r{}c{}", matrix.id, row, column));
        }
    }
    let is_tombstoned_member = |member_id: &str| {
        tombstoned_member_ids
            .iter()
            .any(|id| member_id == id || member_id.starts_with(&format!("{id}/")))
    };
    if source
        .part_ids
        .iter()
        .chain(&mirror.part_ids)
        .any(|id| is_tombstoned_member(id))
        || document
            .parts
            .iter()
            .any(|part| is_tombstoned_member(&part.id))
        || document
            .boards
            .iter()
            .any(|board| board.part_ids.iter().any(|id| is_tombstoned_member(id)))
        || explicit.is_some_and(|projection| {
            projection.final_owners.keys().any(|id| is_tombstoned_member(id))
        })
        // Omitted slots keep their correspondence and overlay by design
        // (live-yet-absent at key level): only other records on tombstoned
        // slots refuse here.
        || halves.correspondences.iter().any(|association| {
            association.kind == MirroredItemKind::Part
                && (is_tombstoned_member(&association.source_id)
                    || is_tombstoned_member(&association.mirror_id))
                && !halves.overlay_of(association).is_some_and(|exception| {
                    exception.state == crate::model::MirroredExceptionState::Omitted
                })
        })
        || halves.exceptions.iter().any(|exception| {
            exception.kind == MirroredItemKind::Part
                && is_tombstoned_member(&exception.mirror_id)
                && exception.state != crate::model::MirroredExceptionState::Omitted
        })
    {
        return Err("A tombstoned matrix slot still has materialized or associated members".into());
    }
    // Projection order is derived data; keep it canonical so filling a sparse source does not
    // cause a one-time cell-vector reorder on the next reconciliation.
    projected.cells.sort_by_key(|cell| (cell.row, cell.column));
    // Explicitly Omitted mirror slots have no pose to reconcile: they leave
    // the active-slot partition to the strict per-slot provenance check
    // below, which still refuses anything it cannot prove. Both projections
    // tombstone them so member accounting and the write-back keep them
    // suppressed.
    let omitted_slot_ids: BTreeSet<Slot> = source
        .cells
        .iter()
        .filter(|cell| cell.enabled && !cell.deleted)
        .filter_map(|cell| {
            let source_id = format!("matrix/{}/r{}c{}", source.id, cell.row, cell.column);
            let mirror_id = format!("matrix/{}/r{}c{}", mirror.id, cell.row, cell.column);
            halves
                .exceptions
                .iter()
                .any(|exception| {
                    exception.kind == MirroredItemKind::Part
                        && exception.mirror_id == mirror_id
                        && exception.state == crate::model::MirroredExceptionState::Omitted
                        && halves.overlay_source(exception) == Some(source_id.as_str())
                })
                .then_some((cell.row, cell.column))
        })
        .collect();
    tombstone_omitted_cells(&mut projected, &omitted_slot_ids);

    if source.id == mirror.id
        || source.rows != mirror.rows
        || source.columns != mirror.columns
        || source.rows != projected.rows
        || source.columns != projected.columns
    {
        return Err("The source and mirror matrices do not have matching fixed dimensions".into());
    }
    let source_slots = canonical_slots(&source)?;
    let mirror_slots = canonical_slots(&mirror)?;
    let projected_slots = canonical_slots(&projected)?;
    let live_source_slots: BTreeSet<Slot> = source_slots
        .keys()
        .copied()
        .collect::<BTreeSet<_>>()
        .difference(&omitted_slot_ids)
        .copied()
        .collect();
    // The staged mirror matrix may already have dropped Omitted slots (native lifecycle
    // partitions them away) or still claim them while only the parts stay absent (same-board
    // durable absence): both shapes reconcile through the omitted branch below, which derives
    // absent mirror ids by slot either way.
    let all_source_slots: BTreeSet<Slot> = source_slots.keys().copied().collect();
    let mirror_slot_set: BTreeSet<Slot> = mirror_slots.keys().copied().collect();
    if projected_slots.keys().copied().collect::<BTreeSet<_>>() != live_source_slots
        || (mirror_slot_set != live_source_slots && mirror_slot_set != all_source_slots)
    {
        return Err("The source and mirror matrices do not have matching active slots".into());
    }
    let omitted_mirror_companions: BTreeSet<_> =
        exact_omitted_matrix_companions(&source, &mirror, halves)?
            .into_iter()
            .map(|companion| companion.mirror_id)
            .collect();
    let source_members = canonical_members(
        &source,
        &source_slots,
        &BTreeSet::new(),
        source.part_ids.iter(),
    )?;
    let mirror_members = canonical_members(
        &mirror,
        &mirror_slots,
        &omitted_mirror_companions,
        mirror.part_ids.iter(),
    )?;
    // Ignore suppressed companions when checking the projection's ownership. Keep the IDs on
    // the returned matrix until staging applies the omission.
    canonical_members(
        &projected,
        &projected_slots,
        &omitted_mirror_companions,
        projected
            .part_ids
            .iter()
            .filter(|id| !omitted_mirror_companions.contains(*id)),
    )?;

    let (explicit_links, mut retained_slots) = if let Some(projection) = explicit {
        let (linked, retained) = validate_explicit_membership(
            projection,
            &source_slots,
            &mirror_slots,
            &omitted_slot_ids,
            &live_source_slots,
            &source_members,
            &mirror_members,
        )?;
        (Some(linked), retained)
    } else {
        // Prepared projection has always required explicit source cells for live slots.
        // Filling sparse cells here must not create new bilateral membership.
        if explicit_source_cells.len() != source.cells.len()
            && live_source_slots.iter().any(|&(row, column)| {
                !explicit_source_cells
                    .iter()
                    .any(|cell| (cell.row, cell.column) == (row, column))
            })
        {
            return Err(
                "Linked and retained matrix memberships must form a disjoint complete partition"
                    .into(),
            );
        }
        (None, BTreeMap::new())
    };
    let owner_for = |id: &str| match explicit {
        Some(projection) => projection.final_owners.get(id).map(String::as_str),
        None if mirror_members.contains(id) => mirror.board_id.as_deref(),
        None if source_members.contains(id) => source.board_id.as_deref(),
        None => None,
    };
    let mut retained_companion_slots = BTreeSet::<(Slot, String)>::new();
    let mut retained_companion_parts = Vec::new();
    let mut omitted_cells = Vec::new();
    let mut omitted_parts = BTreeSet::new();
    for (slot, source_id) in &source_slots {
        // Omitted slots leave the active mirror partition: their mirror id
        // is the canonical slot identity the provenance check below proves.
        let derived_mirror_id;
        let mirror_id: &String = match mirror_slots.get(slot) {
            Some(id) => id,
            None if omitted_slot_ids.contains(slot) => {
                derived_mirror_id = format!("matrix/{}/r{}c{}", mirror.id, slot.0, slot.1);
                &derived_mirror_id
            }
            None => {
                return Err(
                    "The source and mirror matrices do not have matching active slots".into(),
                );
            }
        };
        let source_owner = unique_board_owner(document, source_id, "mirrored")?;
        let omitted = halves
            .exceptions
            .iter()
            .filter(|exception| {
                exception.kind == MirroredItemKind::Part
                    && halves.overlay_source(exception) == Some(source_id.as_str())
                    && exception.mirror_id == *mirror_id
                    && exception.state == crate::model::MirroredExceptionState::Omitted
            })
            .count();
        if omitted == 1 {
            if source_owner != halves.source_board_id
                || owner_for(source_id) != Some(source_owner)
                || document
                    .parts
                    .iter()
                    .filter(|part| part.id == *source_id)
                    .count()
                    != 1
                || halves
                    .correspondences
                    .iter()
                    .filter(|record| {
                        record.kind == MirroredItemKind::Part
                            && (record.source_id == *source_id || record.mirror_id == *mirror_id)
                    })
                    .count()
                    != 1
            {
                return Err("An Omitted matrix owner lacks exact source provenance".into());
            }
            let cell = mirror
                .cells
                .iter()
                .find(|cell| (cell.row, cell.column) == *slot)
                .ok_or_else(|| "The Omitted matrix cell is unavailable".to_owned())?
                .clone();
            // Companions follow the intact source cell's assemblies: the staged mirror cell may
            // already be tombstoned (assemblies cleared) while the companion correspondences
            // still need their absent-by-rule identities.
            let source_cell = source
                .cells
                .iter()
                .find(|cell| (cell.row, cell.column) == *slot)
                .ok_or_else(|| "The Omitted matrix source cell is unavailable".to_owned())?;
            let member_ids: Vec<_> = std::iter::once(mirror_id.clone())
                .chain(
                    source_cell
                        .assemblies
                        .iter()
                        .map(|assembly| crate::matrix::companion_id(mirror_id, &assembly.id)),
                )
                .collect();
            for id in &member_ids {
                // Absent members may carry no owner input at all; the materialization check
                // below proves they stay unregistered. A present owner input must still name
                // the mirror board — absence never blesses a contradictory owner. Present
                // members (explicit Local children) must stay mirror-owned.
                let materialized = document.parts.iter().any(|part| part.id == *id)
                    || document
                        .boards
                        .iter()
                        .any(|board| board.part_ids.contains(id));
                let explicit_local_child = id != mirror_id
                    && halves.exceptions.iter().any(|exception| {
                        exception.kind == MirroredItemKind::Part
                            && exception.mirror_id == *id
                            && exception.state == crate::model::MirroredExceptionState::Local
                    });
                let owner = owner_for(id);
                let owner_ok = owner == Some(halves.mirror_board_id.as_str())
                    || (!materialized && owner.is_none())
                    // The canonical pass has no matrix-owner entry for a detached Local
                    // child. Its live board registration is checked directly below.
                    || (explicit.is_none() && explicit_local_child && owner.is_none()
                        && document.parts.iter().filter(|part| part.id == *id).count() == 1
                        && !document.matrices.iter().any(|matrix| matrix.part_ids.contains(id)));
                if !owner_ok {
                    return Err(
                        "An Omitted matrix member has an incomplete or stale owner input".into(),
                    );
                }
                if explicit_local_child {
                    if unique_board_owner(document, id, "mirrored")? != halves.mirror_board_id {
                        return Err(
                            "A Local child of an Omitted owner has an invalid board owner".into(),
                        );
                    }
                } else {
                    if document.parts.iter().any(|part| part.id == *id)
                        || document
                            .boards
                            .iter()
                            .any(|board| board.part_ids.contains(id))
                    {
                        return Err(
                            "An Omitted matrix owner still has materialized inherited geometry"
                                .into(),
                        );
                    }
                    omitted_parts.insert(id.clone());
                }
            }
            omitted_cells.push(RetainedCell { member_ids, cell });
            continue;
        }
        let overlay = halves.exceptions.iter().find(|exception| {
            exception.kind == MirroredItemKind::Part && exception.mirror_id == *mirror_id
        });
        let linked = match &explicit_links {
            Some(slots) => slots.contains(slot),
            None => {
                let local = overlay
                    .is_some_and(|exception| exception.state == MirroredExceptionState::Local);
                if local {
                    retained_slots.insert(*slot, mirror_id.clone());
                }
                !local
            }
        };
        let mirror_owner = unique_board_owner(document, mirror_id, "mirrored")?;
        if source_owner != halves.source_board_id
            || mirror_owner != halves.mirror_board_id
            || owner_for(source_id) != Some(source_owner)
            || owner_for(mirror_id) != Some(mirror_owner)
        {
            return Err("A matrix part has an incomplete or stale owner input".into());
        }
        for id in [source_id, mirror_id] {
            let mut matches = document.parts.iter().filter(|part| part.id == *id);
            let part = matches
                .next()
                .ok_or_else(|| format!("The mirrored part {id:?} is unavailable"))?;
            if matches.next().is_some() {
                return Err(format!("The mirrored part {id:?} is ambiguous"));
            }
            if !point_is_finite(part.pose.at) || !part.pose.rotation.is_finite() {
                return Err(format!("The mirrored part {id:?} has non-finite geometry"));
            }
            if linked
                && crate::matrix::layout::companion_mode(document, id)
                    != crate::model::MatrixCompanionMode::Follows
            {
                return Err(
                    "Linked matrix keys cannot have authored or constrained placement".into(),
                );
            }
        }
        let mut associations = halves.correspondences.iter().filter(|association| {
            association.kind == MirroredItemKind::Part
                && (association.source_id == *source_id
                    || association.mirror_id == *mirror_id
                    || association.source_id == *mirror_id
                    || association.mirror_id == *source_id)
        });
        let association = associations
            .next()
            .ok_or_else(|| "A canonical matrix part association is missing".to_owned())?;
        if associations.next().is_some()
            || association.source_id != *source_id
            || association.mirror_id != *mirror_id
        {
            return Err("A canonical matrix part association is aliased or inconsistent".into());
        }
        // The correspondence is the identity in every state. A state overlay on a slot that is
        // linked would make the slot Local or Omitted, so it must not be projected over.
        if linked && overlay.is_some() {
            return Err("A linked matrix slot has a state overlay".into());
        }
        if linked == retained_slots.get(slot).is_some_and(|id| id == mirror_id) {
            return Err("A canonical matrix slot must be linked or retained exactly once".into());
        }

        let source_cell = source
            .cells
            .iter()
            .find(|cell| (cell.row, cell.column) == *slot)
            .ok_or_else(|| "A source matrix cell is unavailable".to_owned())?;
        let mirror_cell = mirror
            .cells
            .iter()
            .find(|cell| (cell.row, cell.column) == *slot)
            .ok_or_else(|| "A mirror matrix cell is unavailable".to_owned())?;
        let projected_cell = projected
            .cells
            .iter_mut()
            .find(|cell| (cell.row, cell.column) == *slot)
            .ok_or_else(|| "A projected matrix cell is unavailable".to_owned())?;
        if source_cell.assemblies_local == Some(true) || mirror_cell.assemblies_local == Some(true)
        {
            // Local assembly lists have no cross-half identity. The key stays linked, while
            // each half's materialized companions remain under that cell's own ownership.
            continue;
        }
        let source_assemblies: std::collections::BTreeMap<_, _> = source_cell
            .assemblies
            .iter()
            .map(|assembly| (assembly.id.as_str(), assembly))
            .collect();
        let mirror_assemblies: std::collections::BTreeMap<_, _> = mirror_cell
            .assemblies
            .iter()
            .map(|assembly| (assembly.id.as_str(), assembly))
            .collect();
        let projected_assemblies: std::collections::BTreeMap<_, _> = projected_cell
            .assemblies
            .iter()
            .map(|assembly| (assembly.id.as_str(), assembly))
            .collect();
        if source_assemblies.len() != source_cell.assemblies.len()
            || mirror_assemblies.len() != mirror_cell.assemblies.len()
            || projected_assemblies.len() != projected_cell.assemblies.len()
            || source_assemblies.keys().ne(mirror_assemblies.keys())
            || source_assemblies.keys().ne(projected_assemblies.keys())
        {
            return Err(
                "The linked matrix assemblies do not have matching fixed membership".into(),
            );
        }
        let source_key = document
            .parts
            .iter()
            .find(|part| part.id == *source_id)
            .ok_or_else(|| format!("The mirrored part {source_id:?} is unavailable"))?;
        for assembly_id in source_assemblies.keys() {
            let source_assembly = source_assemblies[assembly_id];
            let mirror_assembly = mirror_assemblies[assembly_id];
            let source_companion_id = crate::matrix::companion_id(source_id, assembly_id);
            let mirror_companion_id = crate::matrix::companion_id(mirror_id, assembly_id);
            if omitted_mirror_companions.contains(&mirror_companion_id) {
                let source_companion = document
                    .parts
                    .iter()
                    .find(|part| part.id == source_companion_id)
                    .ok_or_else(|| {
                        format!("The mirrored part {source_companion_id:?} is unavailable")
                    })?;
                if !point_is_finite(source_companion.pose.at)
                    || !source_companion.pose.rotation.is_finite()
                    || unique_board_owner(document, &source_companion_id, "mirrored")?
                        != halves.source_board_id
                    || owner_for(&source_companion_id) != Some(halves.source_board_id.as_str())
                    || document
                        .parts
                        .iter()
                        .any(|part| part.id == mirror_companion_id)
                    || document
                        .boards
                        .iter()
                        .any(|board| board.part_ids.iter().any(|id| id == &mirror_companion_id))
                    || owner_for(&mirror_companion_id).is_some()
                {
                    return Err(
                        "An omitted matrix companion remains materialized or has stale ownership"
                            .into(),
                    );
                }
                omitted_parts.insert(mirror_companion_id);
                continue;
            }
            let mut matches = document
                .parts
                .iter()
                .filter(|part| part.id == source_companion_id);
            let source_companion = matches.next().ok_or_else(|| {
                format!("The mirrored part {source_companion_id:?} is unavailable")
            })?;
            if matches.next().is_some() {
                return Err(format!(
                    "The mirrored part {source_companion_id:?} is ambiguous"
                ));
            }
            let mut matches = document
                .parts
                .iter()
                .filter(|part| part.id == mirror_companion_id);
            let mirror_companion = matches.next().ok_or_else(|| {
                format!("The mirrored part {mirror_companion_id:?} is unavailable")
            })?;
            if matches.next().is_some() {
                return Err(format!(
                    "The mirrored part {mirror_companion_id:?} is ambiguous"
                ));
            }
            let source_owner = unique_board_owner(document, &source_companion_id, "mirrored")?;
            let mirror_owner = unique_board_owner(document, &mirror_companion_id, "mirrored")?;
            if source_owner != halves.source_board_id
                || mirror_owner != halves.mirror_board_id
                || owner_for(&source_companion_id) != Some(source_owner)
                || owner_for(&mirror_companion_id) != Some(mirror_owner)
            {
                return Err("A matrix companion has an incomplete or stale owner input".into());
            }
            for part in [source_companion, mirror_companion] {
                if !point_is_finite(part.pose.at) || !part.pose.rotation.is_finite() {
                    return Err(format!(
                        "The mirrored part {:?} has non-finite geometry",
                        part.id
                    ));
                }
            }
            let mut companion_associations = halves.correspondences.iter().filter(|association| {
                association.kind == MirroredItemKind::Part
                    && (association.source_id == source_companion_id
                        || association.mirror_id == mirror_companion_id
                        || association.source_id == mirror_companion_id
                        || association.mirror_id == source_companion_id)
            });
            let (companion_mode, companion_tweak, retained_exception_state) =
                if let Some(association) = companion_associations.next() {
                    if companion_associations.next().is_some()
                        || association.source_id != source_companion_id
                        || association.mirror_id != mirror_companion_id
                    {
                        return Err(
                            "A canonical matrix companion association is aliased or inconsistent"
                                .into(),
                        );
                    }
                    let local_exception = halves
                        .overlay_of(association)
                        .filter(|exception| exception.state == MirroredExceptionState::Local)
                        .map(|exception| exception.state);
                    (
                        association.companion_mode.unwrap_or(halves.companion_mode),
                        association.companion_tweak,
                        local_exception,
                    )
                } else {
                    let mut matching_exceptions = halves
                        .exceptions
                        .iter()
                        .filter(|exception| exception.mirror_id == mirror_companion_id);
                    let exception = matching_exceptions.next().ok_or_else(|| {
                        "A canonical matrix companion association is missing".to_owned()
                    })?;
                    if matching_exceptions.next().is_some()
                        || exception.kind != MirroredItemKind::Part
                        || !matches!(
                            exception.state,
                            MirroredExceptionState::Local | MirroredExceptionState::Omitted
                        )
                    {
                        return Err(
                            "A canonical matrix companion exception is aliased or inconsistent"
                                .into(),
                        );
                    }
                    (
                        exception.companion_mode.unwrap_or(halves.companion_mode),
                        exception.companion_tweak,
                        Some(exception.state),
                    )
                };
            crate::mechanical::halves::validate_companion(companion_mode, companion_tweak)?;
            if retained_exception_state.is_none()
                && (source_assembly.definition_id != mirror_assembly.definition_id
                    || source_assembly.side != mirror_assembly.side)
            {
                return Err("The linked matrix assemblies have inconsistent definitions".into());
            }
            if linked
                && retained_exception_state.is_none()
                && (crate::matrix::layout::companion_mode(document, &source_companion_id)
                    != crate::model::MatrixCompanionMode::Follows
                    || crate::matrix::layout::companion_mode(document, &mirror_companion_id)
                        != crate::model::MatrixCompanionMode::Follows)
            {
                return Err(
                    "Linked matrix companions cannot have authored or constrained placement".into(),
                );
            }
            if !point_is_finite(source_assembly.offset)
                || !point_is_finite(mirror_assembly.offset)
                || source_assembly
                    .rotation
                    .is_some_and(|rotation| !rotation.is_finite())
                || mirror_assembly
                    .rotation
                    .is_some_and(|rotation| !rotation.is_finite())
            {
                return Err(
                    "The linked matrix assembly placement contains non-finite values".into(),
                );
            }
            if retained_exception_state.is_some() {
                // The exact matrix slot authenticates Local geometry; the materialized mirror
                // Part supplies its world pose while the projected matrix stores the rebased offset.
                retained_companion_slots.insert((*slot, (*assembly_id).to_owned()));
                retained_companion_parts.push(mirror_companion.clone());
                continue;
            }
            if !linked {
                continue;
            }
            let projected_assembly = projected_cell
                .assemblies
                .iter_mut()
                .find(|assembly| assembly.id == *assembly_id)
                .ok_or_else(|| "A projected matrix assembly is unavailable".to_owned())?;
            let derived = super::super::derive::companion_assembly(
                super::super::derive::CompanionAssemblyInput {
                    offset: projected_assembly.offset,
                    rotation: projected_assembly.rotation,
                    key: Some(source_key),
                    companion: Some(source_companion),
                    mapping,
                    mode: companion_mode,
                    tweak: companion_tweak,
                    stabiliser: super::super::SYMMETRIC_DEFINITIONS
                        .contains(&source_assembly.definition_id.as_str()),
                    fallback: false,
                },
            )
            .map_err(str::to_owned)?;
            if !point_is_finite(derived.0)
                || derived.1.is_some_and(|rotation| !rotation.is_finite())
            {
                return Err("The projected matrix assembly contains non-finite values".into());
            }
            projected_assembly.offset = derived.0;
            projected_assembly.rotation = derived.1;
        }
    }

    for (row, column) in source_slots.keys() {
        let source_cell = source
            .cells
            .iter()
            .find(|cell| (cell.row, cell.column) == (*row, *column))
            .ok_or_else(|| "A source matrix cell is unavailable".to_owned())?;
        let mirror_cell = mirror
            .cells
            .iter()
            .find(|cell| (cell.row, cell.column) == (*row, *column))
            .ok_or_else(|| "A mirror matrix cell is unavailable".to_owned())?;
        let projected_cell = projected
            .cells
            .iter_mut()
            .find(|cell| (cell.row, cell.column) == (*row, *column))
            .ok_or_else(|| "A projected matrix cell is unavailable".to_owned())?;
        if mirror_cell.assemblies_local != Some(true) {
            projected_cell.definition_id = source_cell.definition_id.clone();
            projected_cell.variant = source_cell.variant.clone();
            if source_cell.assemblies_local != Some(true) {
                // Reflection normalizes false to None; retain the authored distinction.
                projected_cell.assemblies_local = source_cell.assemblies_local;
            }
        }
    }
    for matrix in [&source, &mirror, &projected] {
        crate::matrix::valid_projection_matrix(matrix)?;
    }
    let evaluated = crate::matrix::evaluated(&projected, document)?;
    let projected_key_poses: BTreeMap<_, _> = evaluated
        .cells
        .iter()
        .map(|cell| ((cell.row as u32, cell.column as u32), cell.pose))
        .collect();
    for ((row, column), assembly_id) in retained_companion_slots {
        let key_pose = projected_key_poses
            .get(&(row, column))
            .ok_or_else(|| "The projected matrix cell pose is unavailable".to_owned())?;
        let mirror_key_id = format!("matrix/{}/r{}c{}", mirror.id, row, column);
        let mirror_companion_id = crate::matrix::companion_id(&mirror_key_id, &assembly_id);
        let companion = document
            .parts
            .iter()
            .find(|part| part.id == mirror_companion_id)
            .ok_or_else(|| format!("The mirrored part {mirror_companion_id:?} is unavailable"))?;
        let assembly = projected
            .cells
            .iter_mut()
            .find(|cell| (cell.row, cell.column) == (row, column))
            .and_then(|cell| {
                cell.assemblies
                    .iter_mut()
                    .find(|assembly| assembly.id == assembly_id)
            })
            .ok_or_else(|| "A projected matrix assembly is unavailable".to_owned())?;
        retain_companion_pose(assembly, *key_pose, companion)?;
    }
    for cell in &projected.cells {
        let key_pose = projected_key_poses
            .get(&(cell.row, cell.column))
            .ok_or_else(|| "The projected matrix cell pose is unavailable".to_owned())?;
        for assembly in &cell.assemblies {
            let (sin, cos) = key_pose.rotation.to_radians().sin_cos();
            let companion_at = Vec2 {
                x: key_pose.at.x + assembly.offset.x * cos - assembly.offset.y * sin,
                y: key_pose.at.y + assembly.offset.x * sin + assembly.offset.y * cos,
            };
            let companion_rotation = key_pose.rotation + assembly.rotation.unwrap_or(0.0);
            if !point_is_finite(companion_at) || !companion_rotation.is_finite() {
                return Err("The projected matrix companion has non-finite geometry".into());
            }
        }
    }

    let mut retained_cells = Vec::with_capacity(retained_slots.len());
    for ((row, column), id) in retained_slots {
        let retained = document
            .parts
            .iter()
            .find(|part| part.id == id)
            .ok_or_else(|| format!("The mirrored part {id:?} is unavailable"))?;
        let geometry = evaluated
            .cells
            .iter()
            .find(|cell| cell.row == row as usize && cell.column == column as usize)
            .ok_or("Retained key frame is missing")?;
        let delta = crate::matrix::evaluation::inverse_offset(
            &geometry.offset_frame,
            Vec2 {
                x: retained.pose.at.x - geometry.pose.at.x,
                y: retained.pose.at.y - geometry.pose.at.y,
            },
        );
        let old_cell = projected
            .cells
            .iter()
            .find(|cell| cell.row == row && cell.column == column)
            .ok_or("Retained key is missing")?;
        let old = old_cell.offset.unwrap_or_default();
        let cell_offset = Vec2 {
            x: old.x + delta.x,
            y: old.y + delta.y,
        };
        let sign = if matches!(
            projected.mirror,
            Some(crate::model::Mirror::X | crate::model::Mirror::Y)
        ) {
            -1.0
        } else {
            1.0
        };
        let cell_rotation = old_cell.rotation.unwrap_or_default()
            + sign * (retained.pose.rotation - geometry.pose.rotation);
        if !point_is_finite(cell_offset) || !cell_rotation.is_finite() {
            return Err("The retained cell inverse produced non-finite geometry".into());
        }
        let mut cell = projected
            .cells
            .iter()
            .find(|cell| cell.row == row && cell.column == column)
            .ok_or_else(|| "The retained matrix cell is unavailable".to_owned())?
            .clone();
        cell.offset = Some(cell_offset);
        cell.rotation = Some(cell_rotation);
        let source_cell = source
            .cells
            .iter()
            .find(|source_cell| (source_cell.row, source_cell.column) == (row, column))
            .ok_or_else(|| "The source matrix cell is unavailable".to_owned())?;
        let mirror_cell = mirror
            .cells
            .iter()
            .find(|mirror_cell| (mirror_cell.row, mirror_cell.column) == (row, column))
            .ok_or_else(|| "The mirror matrix cell is unavailable".to_owned())?;
        if source_cell.assemblies_local == Some(true) || mirror_cell.assemblies_local == Some(true)
        {
            cell.assemblies = mirror_cell.assemblies.clone();
        } else {
            for assembly in &mut cell.assemblies {
                let source_assembly = source_cell
                    .assemblies
                    .iter()
                    .find(|source_assembly| source_assembly.id == assembly.id)
                    .ok_or_else(|| "The retained source assembly is unavailable".to_owned())?;
                *assembly = mirror_cell
                    .assemblies
                    .iter()
                    .find(|mirror_assembly| mirror_assembly.id == source_assembly.id)
                    .ok_or_else(|| "The retained mirror assembly is unavailable".to_owned())?
                    .clone();
            }
        }
        let mut member_ids = vec![id.clone()];
        member_ids.extend(
            cell.assemblies
                .iter()
                .map(|assembly| crate::matrix::companion_id(&id, &assembly.id)),
        );
        retained_cells.push(RetainedCell { member_ids, cell });
    }
    retained_cells.extend(omitted_cells);
    retained_cells.sort_by_key(|retained| (retained.cell.row, retained.cell.column));

    let mut incoming_matrix = projected;
    for retained in &retained_cells {
        let cell = incoming_matrix
            .cells
            .iter_mut()
            .find(|cell| cell.row == retained.cell.row && cell.column == retained.cell.column)
            .ok_or_else(|| "The retained matrix cell is unavailable".to_owned())?;
        cell.enabled = false;
        incoming_matrix
            .part_ids
            .retain(|id| !retained.member_ids.contains(id));
    }
    Ok(FixedMembershipView {
        normalized_source_matrix: source,
        incoming_matrix,
        retained_cells,
        retained_companion_parts,
        omitted_parts,
        omitted_matrix_companions: omitted_mirror_companions,
    })
}

fn validate_explicit_membership(
    projection: &Projection,
    source_slots: &SlotIds,
    mirror_slots: &SlotIds,
    omitted_slot_ids: &BTreeSet<Slot>,
    live_source_slots: &BTreeSet<Slot>,
    source_members: &BTreeSet<String>,
    mirror_members: &BTreeSet<String>,
) -> Result<(BTreeSet<Slot>, SlotIds), String> {
    let mut linked_slots = BTreeSet::new();
    // Omitted slots leave pose reconciliation to the provenance check below.
    for part in &projection.parts {
        let source_slot = source_slots
            .iter()
            .find_map(|(slot, id)| (id == &part.source_id).then_some(*slot));
        if source_slot.is_some_and(|slot| omitted_slot_ids.contains(&slot)) {
            continue;
        }
        let mirror_slot = mirror_slots
            .iter()
            .find_map(|(slot, id)| (id == &part.mirror_id).then_some(*slot));
        let Some(slot) = source_slot.filter(|slot| Some(*slot) == mirror_slot) else {
            return Err("The linked matrix associations are missing or inconsistent".into());
        };
        // Canonical member IDs identify one slot on each matrix.
        if !linked_slots.insert(slot) {
            return Err("The linked matrix associations contain aliases".into());
        }
    }

    let mut retained_slots = SlotIds::new();
    for id in &projection.locally_protected_mirror_parts {
        let Some((slot, _)) = mirror_slots.iter().find(|(_, member)| *member == id) else {
            return Err(format!(
                "Locally protected mirror part {id:?} is not a matrix member"
            ));
        };
        retained_slots.insert(*slot, id.clone());
    }
    let partition_slots: std::collections::BTreeSet<_> = linked_slots
        .iter()
        .chain(retained_slots.keys())
        .copied()
        .collect();
    if linked_slots
        .iter()
        .any(|slot| retained_slots.contains_key(slot))
        || partition_slots != *live_source_slots
    {
        return Err(
            "Linked and retained matrix memberships must form a disjoint complete partition".into(),
        );
    }

    if !projection
        .final_owners
        .keys()
        .eq(source_members.union(mirror_members))
    {
        return Err("The final owner input is incomplete for matrix membership".into());
    }
    Ok((linked_slots, retained_slots))
}

pub(super) type Slot = (u32, u32);

pub(super) type SlotIds = std::collections::BTreeMap<Slot, String>;

pub(super) fn fill_implicit_active_cells(matrix: &Matrix) -> Result<Matrix, String> {
    if matrix.rows == 0 || matrix.columns == 0 {
        return Err("The reconciliation matrix has empty dimensions".into());
    }
    let count = matrix
        .rows
        .checked_mul(matrix.columns)
        .ok_or_else(|| "The reconciliation matrix dimensions overflow".to_owned())?;
    let mut coordinates = BTreeSet::new();
    for cell in &matrix.cells {
        if cell.row >= matrix.rows
            || cell.column >= matrix.columns
            || cell.enabled == cell.deleted
            || (cell.deleted && !cell.assemblies.is_empty())
        {
            return Err("The reconciliation matrix does not have fixed active membership".into());
        }
        if !coordinates.insert((cell.row, cell.column)) {
            return Err("The reconciliation matrix contains aliased cells".into());
        }
    }
    let mut normalized = matrix.clone();
    for row in 0..matrix.rows {
        for column in 0..matrix.columns {
            if coordinates.insert((row, column)) {
                normalized.cells.push(crate::model::MatrixCell {
                    shift: None,
                    bend: None,
                    deleted: false,
                    row,
                    column,
                    enabled: true,
                    definition_id: None,
                    variant: None,
                    offset: None,
                    rotation: None,
                    assemblies: vec![],
                    assemblies_local: None,
                });
            }
        }
    }
    if coordinates.len() != count as usize {
        return Err("The reconciliation matrix has incomplete active membership".into());
    }
    Ok(normalized)
}

pub(super) fn inactive_slots(matrix: &Matrix) -> BTreeSet<Slot> {
    matrix
        .cells
        .iter()
        .filter(|cell| cell.deleted && !cell.enabled)
        .map(|cell| (cell.row, cell.column))
        .collect()
}

pub(super) fn tombstone_omitted_cells(
    matrix: &mut crate::model::Matrix,
    omitted: &BTreeSet<(u32, u32)>,
) {
    let matrix_id = matrix.id.clone();
    let mut dropped = BTreeSet::new();
    for cell in matrix.cells.iter_mut() {
        if omitted.contains(&(cell.row, cell.column)) {
            let key_id = crate::matrix::member_id(&matrix_id, cell.row, cell.column);
            dropped.insert(key_id.clone());
            for assembly in &cell.assemblies {
                dropped.insert(crate::matrix::companion_id(&key_id, &assembly.id));
            }
            cell.deleted = true;
            cell.enabled = false;
            cell.assemblies.clear();
            cell.assemblies_local = None;
        }
    }
    matrix.part_ids.retain(|id| !dropped.contains(id));
}

pub(super) fn point_is_finite(point: Vec2) -> bool {
    point.x.is_finite() && point.y.is_finite()
}

pub(super) fn canonical_slots(matrix: &Matrix) -> Result<SlotIds, String> {
    if matrix.rows == 0 || matrix.columns == 0 {
        return Err("The reconciliation matrix has empty dimensions".into());
    }
    let count = matrix
        .rows
        .checked_mul(matrix.columns)
        .ok_or_else(|| "The reconciliation matrix dimensions overflow".to_owned())?;
    if matrix.cells.len() != count as usize {
        return Err("The reconciliation matrix does not have fixed active membership".into());
    }
    let mut coordinates = BTreeSet::new();
    let mut slots = SlotIds::new();
    for cell in &matrix.cells {
        if cell.row >= matrix.rows
            || cell.column >= matrix.columns
            || cell.enabled == cell.deleted
            || (cell.deleted && !cell.assemblies.is_empty())
        {
            return Err("The reconciliation matrix does not have fixed active membership".into());
        }
        if !coordinates.insert((cell.row, cell.column)) {
            return Err("The reconciliation matrix contains aliased cells".into());
        }
        if cell.deleted {
            continue;
        }
        let id = format!("matrix/{}/r{}c{}", matrix.id, cell.row, cell.column);
        slots.insert((cell.row, cell.column), id);
    }
    if coordinates.len() != count as usize
        || slots.len() + inactive_slots(matrix).len() != count as usize
    {
        return Err("The reconciliation matrix has incomplete active membership".into());
    }
    Ok(slots)
}

fn canonical_members<'a>(
    matrix: &Matrix,
    slots: &SlotIds,
    omitted_companions: &BTreeSet<String>,
    mut actual_ids: impl Iterator<Item = &'a String>,
) -> Result<std::collections::BTreeSet<String>, String> {
    let mut members: std::collections::BTreeSet<_> = slots.values().cloned().collect();
    for cell in &matrix.cells {
        if cell.deleted {
            continue;
        }
        let key_id = slots
            .get(&(cell.row, cell.column))
            .ok_or_else(|| "The reconciliation matrix has an unknown active slot".to_owned())?;
        let mut assembly_ids = std::collections::BTreeSet::new();
        for assembly in &cell.assemblies {
            if assembly.id.is_empty()
                || !assembly_ids.insert(assembly.id.as_str())
                || !assembly.offset.x.is_finite()
                || !assembly.offset.y.is_finite()
                || assembly
                    .rotation
                    .is_some_and(|rotation| !rotation.is_finite())
            {
                return Err("The reconciliation matrix has invalid assembly membership".into());
            }
            let id = crate::matrix::companion_id(key_id, &assembly.id);
            if !omitted_companions.contains(&id) && !members.insert(id) {
                return Err("The reconciliation matrix has aliased assembly membership".into());
            }
        }
    }
    let mut actual = BTreeSet::new();
    if !actual_ids.all(|id| actual.insert(id)) || !actual.into_iter().eq(&members) {
        return Err("The reconciliation matrix membership is not canonical".into());
    }
    Ok(members)
}

pub(super) fn retain_companion_pose(
    assembly: &mut crate::model::MatrixAssembly,
    key_pose: crate::model::Pose2,
    companion: &Part,
) -> Result<(), String> {
    let (sin, cos) = (-key_pose.rotation.to_radians()).sin_cos();
    let dx = companion.pose.at.x - key_pose.at.x;
    let dy = companion.pose.at.y - key_pose.at.y;
    let local_offset = Vec2 {
        x: dx * cos - dy * sin,
        y: dx * sin + dy * cos,
    };
    let local_rotation = companion.pose.rotation - key_pose.rotation;
    if !point_is_finite(local_offset) || !local_rotation.is_finite() {
        return Err("The retained Local matrix companion has non-finite geometry".into());
    }
    assembly.offset = local_offset;
    assembly.rotation = Some(local_rotation);
    Ok(())
}
