//! Forward reconciliation building blocks for already-corresponded mirrored values.
//!
//! Where things live (search the names; line numbers drift):
//! - Types: `Projection`, `IndependentPartTransition`, `HostedTransition`, `MatrixStage`.
//! - Fixed-membership view: `FixedMembershipView`, `build_fixed_membership_view`.
//! - `reconcile` body, in order: baseline validators and `stage_matrix`; pair/halves lookup;
//!   independent part Add/Remove; `overlay_state` and circuit pairs; hosted `AddCircuit`;
//!   board/layout correspondences; module targets; final validation, `outlines::reconcile`.
//! - Membership-transition kernel: `prepare_membership_transition_kernel`, `prepare_membership_transition`.
//! - Child modules: `lifecycle` (lifecycle.rs), `outlines` (reconcile/outlines.rs).
//! - `mod tests` at the bottom; outline tests in reconcile/outlines/tests.rs.

use crate::model::{
    Matrix, MirroredCorrespondence, MirroredExceptionState, MirroredItemKind, Part, Pose2,
    ProjectDoc, Vec2,
};
use std::collections::{BTreeMap, BTreeSet};

#[path = "lifecycle.rs"]
mod lifecycle;
#[allow(dead_code)] // Ticket 111 activates this prepared transaction in production.
mod prepare;
#[allow(unused_imports)] // The crate-only seam is intentionally dormant until ticket 111.
pub(crate) use prepare::{prepare_open, prepare_transaction};
mod outlines;
mod settings;
// Ticket 147: keycap settings and plain nets follow their source after creation.
#[cfg(test)]
mod settings_tests;
// Ticket 145: a source edit of a matrix the forward addition mirrored reaches the mirror.
#[cfg(test)]
#[path = "matrix_addition/reconcile_tests.rs"]
mod matrix_addition_reconcile;

#[derive(Clone)]
struct PartProjection {
    source_id: String,
    mirror_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[allow(dead_code)] // Ticket 106 prepares this private transition slice before activation.
enum IndependentPartTransition {
    Add {
        source_id: String,
        mirror_id: String,
        mirror_definition_id: String,
        mirror_reference: String,
    },
    Remove {
        source_id: String,
        mirror_id: String,
    },
}

#[derive(Clone, Debug, PartialEq)]
#[allow(dead_code)] // Ticket 107 prepares hosted item operations before activation in 111.
enum HostedTransition {
    AddModule {
        source_id: String,
        mirror_id: String,
    },
    RemoveModule {
        source_id: String,
        mirror_id: String,
    },
    AddCircuit {
        source_id: String,
        mirror_id: String,
        member_ids: BTreeMap<String, String>,
        member_references: BTreeMap<String, String>,
        definition_ids: BTreeMap<String, String>,
        net_ids: BTreeMap<String, String>,
        member_poses: BTreeMap<String, Pose2>,
    },
    RemoveCircuit {
        source_id: String,
        mirror_id: String,
    },
}

/// The narrow result consumed by the transaction coordinator after lifecycle validation.
pub(super) struct PreparedLifecycleDraft {
    pub(super) draft: ProjectDoc,
    pub(super) changed_document_ids: Vec<String>,
}

#[derive(Clone)]
struct Projection {
    matrix: Option<MatrixLink>,
    /// Preserve geometry already prepared by the native membership lifecycle.
    preserve_matrix_geometry: bool,
    source_board_id: String,
    mirror_board_id: String,
    mapping: Pose2,
    parts: Vec<PartProjection>,
    independent_part_transitions: Vec<IndependentPartTransition>,
    hosted_transitions: Vec<HostedTransition>,
    applied_independent_additions: std::cell::RefCell<BTreeSet<(String, String, String, String)>>,
    #[allow(dead_code)]
    // Ticket 107 records hosted Add provenance privately during reconciliation.
    applied_hosted_operations: std::cell::RefCell<BTreeSet<(String, String, String)>>,
    #[allow(dead_code)]
    // Ticket 107 binds circuit Add replay to the exact selected maps and first output.
    applied_hosted_circuit_additions:
        std::cell::RefCell<BTreeMap<(String, String, String), HostedCircuitAddProvenance>>,
    #[allow(dead_code)]
    // Ticket 107 keeps source-operation baselines private until lifecycle activation.
    hosted_removal_baselines: std::cell::RefCell<BTreeMap<(String, String, String), ProjectDoc>>,
    #[allow(dead_code)]
    // Ticket 107 retains validated owner context for physically absent Omitted children.
    hosted_omission_baselines: std::cell::RefCell<BTreeMap<(String, String, String), ProjectDoc>>,
    #[allow(dead_code)]
    // Ticket 108 authenticates outline removals against their valid pre-source pair.
    outline_removal_baselines: std::cell::RefCell<BTreeMap<(String, String, String), ProjectDoc>>,
    locally_protected_mirror_parts: std::collections::BTreeSet<String>,
    final_owners: std::collections::BTreeMap<String, String>,
}

#[derive(Clone)]
struct MatrixLink {
    source_matrix_id: String,
    mirror_matrix_id: String,
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

fn is_omitted_matrix_link(
    halves: &crate::model::MirroredHalves,
    source_matrix_id: &str,
    mirror_matrix_id: &str,
) -> bool {
    halves.exceptions.iter().any(|exception| {
        exception.kind == MirroredItemKind::Matrix
            && exception.mirror_id == mirror_matrix_id
            && exception.state == MirroredExceptionState::Omitted
            && halves.overlay_source(exception) == Some(source_matrix_id)
    })
}

fn unique_board_owner<'a>(
    document: &'a ProjectDoc,
    part_id: &str,
    context: &str,
) -> Result<&'a str, String> {
    let mut owners = document
        .boards
        .iter()
        .filter(|board| board.part_ids.iter().any(|id| id == part_id));
    let owner = owners
        .next()
        .ok_or_else(|| format!("The {context} part {part_id:?} has no board owner"))?;
    if owners.next().is_some() {
        return Err(format!(
            "The {context} part {part_id:?} has ambiguous board ownership"
        ));
    }
    Ok(&owner.id)
}

fn omitted_mirror_matrix(
    source: &Matrix,
    mirror_id: &str,
    mirror_board_id: &str,
    halves: &crate::model::MirroredHalves,
) -> Matrix {
    let source_members: BTreeSet<_> = source.part_ids.iter().map(String::as_str).collect();
    let mut mirror = source.clone();
    mirror.id = mirror_id.to_owned();
    mirror.board_id = Some(mirror_board_id.to_owned());
    mirror.part_ids = halves
        .correspondences
        .iter()
        .filter(|correspondence| {
            correspondence.kind == MirroredItemKind::Part
                && source_members.contains(correspondence.source_id.as_str())
        })
        .map(|correspondence| correspondence.mirror_id.clone())
        .collect();
    mirror
}

/// Keeps every Matrix-owned Part out of the independent-Part path while matrices reconcile one at a time.
fn matrix_members_for_pair(
    document: &ProjectDoc,
    halves: &crate::model::MirroredHalves,
) -> (BTreeSet<String>, BTreeSet<String>) {
    let mut source_members = BTreeSet::new();
    let mut mirror_members = BTreeSet::new();
    for matrix in &document.matrices {
        if matrix.board_id.as_deref() == Some(halves.source_board_id.as_str()) {
            source_members.extend(matrix.part_ids.iter().cloned());
        }
        if matrix.board_id.as_deref() == Some(halves.mirror_board_id.as_str()) {
            mirror_members.extend(matrix.part_ids.iter().cloned());
        }
    }
    for correspondence in halves
        .correspondences
        .iter()
        .filter(|item| item.kind == MirroredItemKind::Matrix)
    {
        let omitted =
            is_omitted_matrix_link(halves, &correspondence.source_id, &correspondence.mirror_id);
        if !omitted
            || document
                .matrices
                .iter()
                .any(|matrix| matrix.id == correspondence.mirror_id)
        {
            continue;
        }
        if let Some(source) = document
            .matrices
            .iter()
            .find(|matrix| matrix.id == correspondence.source_id)
        {
            mirror_members.extend(
                omitted_mirror_matrix(
                    source,
                    &correspondence.mirror_id,
                    &halves.mirror_board_id,
                    halves,
                )
                .part_ids,
            );
        }
    }
    (source_members, mirror_members)
}

#[derive(Clone, Debug, PartialEq)]
struct HostedCircuitAddProvenance {
    member_ids: BTreeMap<String, String>,
    member_references: BTreeMap<String, String>,
    definition_ids: BTreeMap<String, String>,
    net_ids: BTreeMap<String, String>,
    member_poses: BTreeMap<String, Pose2>,
    first_output: ProjectDoc,
}

#[derive(Default)]
struct HostedOmissionClosure {
    parts: BTreeSet<String>,
    nets: BTreeSet<String>,
}

struct RetainedCell {
    member_ids: Vec<String>,
    cell: crate::model::MatrixCell,
}

struct MatrixStage {
    draft: ProjectDoc,
    source_members: BTreeSet<String>,
    mirror_members: BTreeSet<String>,
    omitted_parts: BTreeSet<String>,
    allow_absent: bool,
}

struct FixedMembershipView {
    normalized_source_matrix: Matrix,
    incoming_matrix: Matrix,
    retained_cells: Vec<RetainedCell>,
    unassociated_companion_parts: Vec<Part>,
    omitted_parts: BTreeSet<String>,
}

fn build_fixed_membership_view(
    document: &ProjectDoc,
    halves: &crate::model::MirroredHalves,
    source: &Matrix,
    mirror: &Matrix,
    projected: &Matrix,
    projection: &Projection,
) -> Result<FixedMembershipView, String> {
    type Slot = (u32, u32);
    type SlotIds = std::collections::BTreeMap<Slot, String>;

    fn fill_implicit_active_cells(matrix: &Matrix) -> Result<Matrix, String> {
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
                return Err(
                    "The reconciliation matrix does not have fixed active membership".into(),
                );
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

    fn inactive_slots(matrix: &Matrix) -> BTreeSet<Slot> {
        matrix
            .cells
            .iter()
            .filter(|cell| cell.deleted && !cell.enabled)
            .map(|cell| (cell.row, cell.column))
            .collect()
    }

    // Missing coordinates use the same empty active-cell defaults as matrix materialization.
    // Explicit lifecycle tombstones remain inactive; contradictory states are refused above.
    let source = fill_implicit_active_cells(source)?;
    let mirror = fill_implicit_active_cells(mirror)?;
    let mut projected = fill_implicit_active_cells(projected)?;
    let inactive = inactive_slots(&source);
    if inactive != inactive_slots(&mirror) || inactive != inactive_slots(&projected) {
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
        || projection
            .final_owners
            .keys()
            .any(|id| is_tombstoned_member(id))
        || halves.correspondences.iter().any(|association| {
            association.kind == MirroredItemKind::Part
                && (is_tombstoned_member(&association.source_id)
                    || is_tombstoned_member(&association.mirror_id))
        })
        || halves.exceptions.iter().any(|exception| {
            exception.kind == MirroredItemKind::Part && is_tombstoned_member(&exception.mirror_id)
        })
    {
        return Err("A tombstoned matrix slot still has materialized or associated members".into());
    }
    // Projection order is derived data; keep it canonical so filling a sparse source does not
    // cause a one-time cell-vector reorder on the next reconciliation.
    projected.cells.sort_by_key(|cell| (cell.row, cell.column));
    let mut adjusted_projected = projected.clone();

    fn point_is_finite(point: Vec2) -> bool {
        point.x.is_finite() && point.y.is_finite()
    }

    fn canonical_slots(matrix: &Matrix) -> Result<SlotIds, String> {
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
                return Err(
                    "The reconciliation matrix does not have fixed active membership".into(),
                );
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

    fn canonical_members(
        matrix: &Matrix,
        slots: &SlotIds,
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
                if !members.insert(id) {
                    return Err("The reconciliation matrix has aliased assembly membership".into());
                }
            }
        }
        let actual: std::collections::BTreeSet<_> = matrix.part_ids.iter().cloned().collect();
        if actual.len() != matrix.part_ids.len() || actual != members {
            return Err("The reconciliation matrix membership is not canonical".into());
        }
        Ok(members)
    }

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
    if source_slots.keys().ne(mirror_slots.keys()) || source_slots.keys().ne(projected_slots.keys())
    {
        return Err("The source and mirror matrices do not have matching active slots".into());
    }
    let source_members = canonical_members(&source, &source_slots)?;
    let mirror_members = canonical_members(&mirror, &mirror_slots)?;
    canonical_members(&projected, &projected_slots)?;

    let mut linked_slots = std::collections::BTreeMap::<Slot, String>::new();
    let mut linked_source_ids = std::collections::BTreeSet::new();
    let mut linked_mirror_ids = std::collections::BTreeSet::new();
    let mut unassociated_companion_slots = BTreeSet::<(Slot, String)>::new();
    let mut unassociated_companion_parts = Vec::new();
    for part in &projection.parts {
        let source_slot = source_slots
            .iter()
            .find_map(|(slot, id)| (id == &part.source_id).then_some(*slot));
        let mirror_slot = mirror_slots
            .iter()
            .find_map(|(slot, id)| (id == &part.mirror_id).then_some(*slot));
        let Some(slot) = source_slot.filter(|slot| Some(*slot) == mirror_slot) else {
            return Err("The linked matrix associations are missing or inconsistent".into());
        };
        if !linked_source_ids.insert(part.source_id.clone())
            || !linked_mirror_ids.insert(part.mirror_id.clone())
            || linked_slots.insert(slot, part.mirror_id.clone()).is_some()
        {
            return Err("The linked matrix associations contain aliases".into());
        }
    }

    let mut retained_slots = std::collections::BTreeMap::<Slot, String>::new();
    for id in &projection.locally_protected_mirror_parts {
        let Some((slot, _)) = mirror_slots.iter().find(|(_, member)| *member == id) else {
            return Err(format!(
                "Locally protected mirror part {id:?} is not a matrix member"
            ));
        };
        if retained_slots.insert(*slot, id.clone()).is_some() {
            return Err("The retained matrix associations contain aliases".into());
        }
    }
    let partition_slots: std::collections::BTreeSet<_> = linked_slots
        .keys()
        .chain(retained_slots.keys())
        .copied()
        .collect();
    if linked_slots
        .keys()
        .any(|slot| retained_slots.contains_key(slot))
        || partition_slots != source_slots.keys().copied().collect()
        || projection.parts.len() != linked_slots.len()
        || projection.locally_protected_mirror_parts.len() != retained_slots.len()
        || linked_mirror_ids
            .iter()
            .any(|id| projection.locally_protected_mirror_parts.contains(id))
    {
        return Err(
            "Linked and retained matrix memberships must form a disjoint complete partition".into(),
        );
    }

    let affected_ids: std::collections::BTreeSet<_> =
        source_members.union(&mirror_members).cloned().collect();
    let owner_ids: std::collections::BTreeSet<_> =
        projection.final_owners.keys().cloned().collect();
    if owner_ids != affected_ids || projection.final_owners.len() != affected_ids.len() {
        return Err("The final owner input is incomplete for matrix membership".into());
    }
    let mut omitted_cells = Vec::new();
    let mut omitted_parts = BTreeSet::new();
    for (slot, source_id) in &source_slots {
        let mirror_id = &mirror_slots[slot];
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
                || projection.final_owners.get(source_id).map(String::as_str) != Some(source_owner)
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
            let member_ids: Vec<_> = std::iter::once(mirror_id.clone())
                .chain(
                    cell.assemblies
                        .iter()
                        .map(|assembly| crate::matrix::companion_id(mirror_id, &assembly.id)),
                )
                .collect();
            for id in &member_ids {
                if projection.final_owners.get(id).map(String::as_str)
                    != Some(halves.mirror_board_id.as_str())
                {
                    return Err(
                        "An Omitted matrix member has an incomplete or stale owner input".into(),
                    );
                }
                let explicit_local_child = id != mirror_id
                    && halves.exceptions.iter().any(|exception| {
                        exception.kind == MirroredItemKind::Part
                            && exception.mirror_id == *id
                            && exception.state == crate::model::MirroredExceptionState::Local
                    });
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
        let mirror_owner = unique_board_owner(document, mirror_id, "mirrored")?;
        if source_owner != halves.source_board_id
            || mirror_owner != halves.mirror_board_id
            || projection.final_owners.get(source_id).map(String::as_str) != Some(source_owner)
            || projection.final_owners.get(mirror_id).map(String::as_str) != Some(mirror_owner)
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
            if linked_slots.contains_key(slot)
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
        if linked_slots.get(slot).is_some_and(|id| id == mirror_id)
            && halves.exceptions.iter().any(|exception| {
                exception.kind == MirroredItemKind::Part && exception.mirror_id == *mirror_id
            })
        {
            return Err("A linked matrix slot has a state overlay".into());
        }
        if linked_slots.get(slot).is_some_and(|id| id == mirror_id)
            == retained_slots.get(slot).is_some_and(|id| id == mirror_id)
        {
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
            .iter()
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
                || projection
                    .final_owners
                    .get(&source_companion_id)
                    .map(String::as_str)
                    != Some(source_owner)
                || projection
                    .final_owners
                    .get(&mirror_companion_id)
                    .map(String::as_str)
                    != Some(mirror_owner)
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
            let (companion_mode, companion_tweak, unassociated_exception_state) =
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
                    (
                        association.companion_mode.unwrap_or(halves.companion_mode),
                        association.companion_tweak,
                        None,
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
            if unassociated_exception_state.is_none()
                && (source_assembly.definition_id != mirror_assembly.definition_id
                    || source_assembly.side != mirror_assembly.side)
            {
                return Err("The linked matrix assemblies have inconsistent definitions".into());
            }
            if linked_slots.contains_key(slot)
                && unassociated_exception_state.is_none()
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
            if unassociated_exception_state.is_some() {
                // A legacy exception has no source identity to project or omit. Its exact
                // matrix slot authenticates membership, while the materialized mirror Part
                // supplies the geometry that must survive both Local and Omitted records.
                unassociated_companion_slots.insert((*slot, (*assembly_id).to_owned()));
                unassociated_companion_parts.push(mirror_companion.clone());
                continue;
            }
            if !linked_slots.contains_key(slot) {
                continue;
            }
            let projected_assembly = projected_cell
                .assemblies
                .iter()
                .find(|assembly| assembly.id == *assembly_id)
                .ok_or_else(|| "A projected matrix assembly is unavailable".to_owned())?;
            let derived =
                super::derive::companion_assembly(super::derive::CompanionAssemblyInput {
                    offset: projected_assembly.offset,
                    rotation: projected_assembly.rotation,
                    key: Some(source_key),
                    companion: Some(source_companion),
                    mapping: projection.mapping,
                    mode: companion_mode,
                    tweak: companion_tweak,
                    stabiliser: super::SYMMETRIC_DEFINITIONS
                        .contains(&source_assembly.definition_id.as_str()),
                    fallback: false,
                })
                .map_err(str::to_owned)?;
            if !point_is_finite(derived.0)
                || derived.1.is_some_and(|rotation| !rotation.is_finite())
            {
                return Err("The projected matrix assembly contains non-finite values".into());
            }
            let projected_assembly = adjusted_projected
                .cells
                .iter_mut()
                .find(|cell| (cell.row, cell.column) == *slot)
                .and_then(|cell| {
                    cell.assemblies
                        .iter_mut()
                        .find(|assembly| assembly.id == *assembly_id)
                })
                .ok_or_else(|| "A projected matrix assembly is unavailable".to_owned())?;
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
        let projected_cell = adjusted_projected
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
    let finite_matrix = |matrix: &Matrix| {
        point_is_finite(matrix.origin)
            && point_is_finite(matrix.pitch)
            && matrix.rotation.is_none_or(f64::is_finite)
            && matrix
                .row_offsets
                .iter()
                .chain(&matrix.column_offsets)
                .chain(matrix.column_origins.iter().flatten())
                .all(|point| point_is_finite(*point))
            && matrix
                .column_staggers
                .iter()
                .chain(&matrix.column_splays)
                .all(|angle| angle.is_finite())
            && matrix.cells.iter().all(|cell| {
                cell.offset.is_none_or(point_is_finite) && cell.rotation.is_none_or(f64::is_finite)
            })
    };
    if !finite_matrix(&source) || !finite_matrix(&mirror) || !finite_matrix(&adjusted_projected) {
        return Err("The reconciliation matrix geometry contains non-finite values".into());
    }

    let mirror_axis = adjusted_projected
        .mirror
        .unwrap_or(crate::model::Mirror::None);
    let matrix_rotation = adjusted_projected.rotation.unwrap_or(0.0);
    let mut projected_key_poses = std::collections::BTreeMap::new();
    for cell in &adjusted_projected.cells {
        let row = cell.row as usize;
        let column = cell.column as usize;
        let row_offset = adjusted_projected
            .row_offsets
            .get(row)
            .copied()
            .unwrap_or_default();
        let column_offset = adjusted_projected
            .column_offsets
            .get(column)
            .copied()
            .unwrap_or_default();
        let cell_offset = cell.offset.unwrap_or_default();
        let stagger = adjusted_projected
            .column_staggers
            .iter()
            .take(column + 1)
            .sum::<f64>();
        let mut local = Vec2 {
            x: column as f64 * adjusted_projected.pitch.x
                + row_offset.x
                + column_offset.x
                + cell_offset.x,
            y: row as f64 * adjusted_projected.pitch.y
                + row_offset.y
                + column_offset.y
                + cell_offset.y
                + stagger,
        };
        if !point_is_finite(local) {
            return Err("The projected matrix produced non-finite cell geometry".into());
        }
        let mut splay_sum = 0.0;
        for index in (0..=column).rev() {
            let angle = adjusted_projected
                .column_splays
                .get(index)
                .copied()
                .unwrap_or(0.0);
            splay_sum += angle;
            let pivot = adjusted_projected
                .column_origins
                .get(index)
                .copied()
                .flatten()
                .unwrap_or(Vec2 {
                    x: index as f64 * adjusted_projected.pitch.x,
                    y: adjusted_projected
                        .column_staggers
                        .iter()
                        .take(index + 1)
                        .sum(),
                });
            if !point_is_finite(pivot) || !splay_sum.is_finite() {
                return Err("The projected matrix produced non-finite splay geometry".into());
            }
            let (sin, cos) = angle.to_radians().sin_cos();
            let dx = local.x - pivot.x;
            let dy = local.y - pivot.y;
            local = Vec2 {
                x: pivot.x + dx * cos - dy * sin,
                y: pivot.y + dx * sin + dy * cos,
            };
            if !point_is_finite(local) {
                return Err("The projected matrix produced non-finite splay geometry".into());
            }
        }
        match mirror_axis {
            crate::model::Mirror::X => local.x = -local.x,
            crate::model::Mirror::Y => local.y = -local.y,
            crate::model::Mirror::None => {}
        }
        let (sin, cos) = matrix_rotation.to_radians().sin_cos();
        let world = Vec2 {
            x: projected.origin.x + local.x * cos - local.y * sin,
            y: projected.origin.y + local.x * sin + local.y * cos,
        };
        let signed_splay = if matches!(
            mirror_axis,
            crate::model::Mirror::X | crate::model::Mirror::Y
        ) {
            -splay_sum
        } else {
            splay_sum
        };
        let rotation = matrix_rotation + signed_splay + cell.rotation.unwrap_or(0.0);
        if !point_is_finite(world) || !rotation.is_finite() {
            return Err("The projected matrix produced non-finite part geometry".into());
        }
        projected_key_poses.insert(
            (cell.row, cell.column),
            Pose2 {
                at: world,
                rotation,
            },
        );
    }
    for ((row, column), assembly_id) in unassociated_companion_slots {
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
        let assembly = adjusted_projected
            .cells
            .iter_mut()
            .find(|cell| (cell.row, cell.column) == (row, column))
            .and_then(|cell| {
                cell.assemblies
                    .iter_mut()
                    .find(|assembly| assembly.id == assembly_id)
            })
            .ok_or_else(|| "A projected matrix assembly is unavailable".to_owned())?;
        assembly.offset = local_offset;
        assembly.rotation = Some(local_rotation);
    }
    for cell in &adjusted_projected.cells {
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

    let (global_sin, global_cos) = (-matrix_rotation.to_radians()).sin_cos();
    let mut retained_cells = Vec::with_capacity(retained_slots.len());
    for ((row, column), id) in retained_slots {
        let retained = document
            .parts
            .iter()
            .find(|part| part.id == id)
            .ok_or_else(|| format!("The mirrored part {id:?} is unavailable"))?;
        let mut local = Vec2 {
            x: global_cos * (retained.pose.at.x - projected.origin.x)
                - global_sin * (retained.pose.at.y - projected.origin.y),
            y: global_sin * (retained.pose.at.x - projected.origin.x)
                + global_cos * (retained.pose.at.y - projected.origin.y),
        };
        match mirror_axis {
            crate::model::Mirror::X => local.x = -local.x,
            crate::model::Mirror::Y => local.y = -local.y,
            crate::model::Mirror::None => {}
        }
        for index in 0..=column as usize {
            let angle = -projected
                .column_splays
                .get(index)
                .copied()
                .unwrap_or(0.0)
                .to_radians();
            let pivot = projected
                .column_origins
                .get(index)
                .copied()
                .flatten()
                .unwrap_or(Vec2 {
                    x: index as f64 * projected.pitch.x,
                    y: projected.column_staggers.iter().take(index + 1).sum(),
                });
            let (sin, cos) = angle.sin_cos();
            let dx = local.x - pivot.x;
            let dy = local.y - pivot.y;
            local = Vec2 {
                x: pivot.x + dx * cos - dy * sin,
                y: pivot.y + dx * sin + dy * cos,
            };
            if !point_is_finite(local) {
                return Err("The retained cell inverse produced non-finite geometry".into());
            }
        }
        let row_offset = adjusted_projected
            .row_offsets
            .get(row as usize)
            .copied()
            .unwrap_or_default();
        let column_offset = adjusted_projected
            .column_offsets
            .get(column as usize)
            .copied()
            .unwrap_or_default();
        let stagger = adjusted_projected
            .column_staggers
            .iter()
            .take(column as usize + 1)
            .sum::<f64>();
        let cell_offset = Vec2 {
            x: local.x - column as f64 * projected.pitch.x - row_offset.x - column_offset.x,
            y: local.y - row as f64 * projected.pitch.y - row_offset.y - column_offset.y - stagger,
        };
        let splay_sum = adjusted_projected
            .column_splays
            .iter()
            .take(column as usize + 1)
            .sum::<f64>();
        let signed_splay = if matches!(
            mirror_axis,
            crate::model::Mirror::X | crate::model::Mirror::Y
        ) {
            -splay_sum
        } else {
            splay_sum
        };
        let cell_rotation = retained.pose.rotation - matrix_rotation - signed_splay;
        if !point_is_finite(cell_offset) || !cell_rotation.is_finite() {
            return Err("The retained cell inverse produced non-finite geometry".into());
        }
        let mut cell = adjusted_projected
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

    let mut incoming_matrix = adjusted_projected;
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
        unassociated_companion_parts,
        omitted_parts,
    })
}

fn stage_hosted_correspondence(
    document: &mut ProjectDoc,
    pair_id: &str,
    allow_existing: bool,
    correspondence: MirroredCorrespondence,
) -> Result<(), String> {
    let halves = document
        .hardware
        .as_mut()
        .and_then(|hardware| hardware.pairs.iter_mut().find(|pair| pair.id == pair_id))
        .and_then(|pair| pair.halves.as_mut())
        .ok_or_else(|| format!("The staged pair {pair_id:?} has no halves context"))?;
    let existing = halves.correspondences.iter().find(|existing| {
        existing.kind == correspondence.kind
            && (existing.source_id == correspondence.source_id
                || existing.mirror_id == correspondence.mirror_id)
    });
    match existing {
        Some(existing) if existing == &correspondence && allow_existing => Ok(()),
        Some(existing) if existing == &correspondence => Err(format!(
            "The selected hosted {:?} correspondence already exists without private replay provenance",
            correspondence.kind
        )),
        Some(_) => Err(format!(
            "The selected hosted {:?} correspondence conflicts with an existing identity",
            correspondence.kind
        )),
        None => {
            halves.correspondences.push(correspondence);
            Ok(())
        }
    }
}

#[allow(dead_code)] // Ticket 111 activates this private seam after the reconciliation slices are verified.
fn reconcile(
    document: &mut ProjectDoc,
    pair_id: &str,
    projection: &Projection,
) -> Result<Vec<String>, String> {
    fn matrix_by_id<'a>(document: &'a ProjectDoc, id: &str) -> Result<&'a Matrix, String> {
        let mut matches = document.matrices.iter().filter(|matrix| matrix.id == id);
        let matrix = matches
            .next()
            .ok_or_else(|| format!("The mirrored matrix {id:?} is unavailable"))?;
        if matches.next().is_some() {
            return Err(format!("The mirrored matrix {id:?} is ambiguous"));
        }
        Ok(matrix)
    }

    fn snapshot<T: serde::Serialize>(value: &T) -> Result<serde_json::Value, String> {
        serde_json::to_value(value).map_err(|error| error.to_string())
    }

    fn validate_hosted_removal_baselines(
        document: &ProjectDoc,
        pair_id: &str,
        projection: &Projection,
    ) -> Result<(), String> {
        fn pair_bytes(document: &ProjectDoc, pair_id: &str) -> Result<Vec<u8>, String> {
            let hardware = document.hardware.as_ref().ok_or_else(|| {
                "The hosted removal hardware configuration is unavailable".to_owned()
            })?;
            let mut pairs = hardware.pairs.iter().filter(|pair| pair.id == pair_id);
            let pair = pairs
                .next()
                .ok_or_else(|| format!("The hosted removal pair {pair_id:?} is unavailable"))?;
            if pairs.next().is_some() {
                return Err(format!("The hosted removal pair {pair_id:?} is ambiguous"));
            }
            serde_json::to_vec(pair).map_err(|error| error.to_string())
        }

        for transition in &projection.hosted_transitions {
            let (operation, source_id, mirror_id) = match transition {
                HostedTransition::RemoveModule {
                    source_id,
                    mirror_id,
                } => ("remove-module", source_id, mirror_id),
                HostedTransition::RemoveCircuit {
                    source_id,
                    mirror_id,
                } => ("remove-circuit", source_id, mirror_id),
                _ => continue,
            };
            let key = (operation.to_owned(), source_id.clone(), mirror_id.clone());
            let baseline = projection
                .hosted_removal_baselines
                .borrow()
                .get(&key)
                .cloned()
                .ok_or_else(|| {
                    format!("Hosted removal {source_id:?} has no private pre-source baseline")
                })?;
            crate::mechanical::halves::validate_document(&baseline).map_err(|error| {
                format!("Hosted removal {source_id:?} has an invalid pre-source baseline: {error}")
            })?;
            if pair_bytes(&baseline, pair_id)? != pair_bytes(document, pair_id)? {
                return Err(format!(
                    "Hosted removal {source_id:?} changed its durable pair record before reconciliation"
                ));
            }
            let source_exists = match operation {
                "remove-module" => baseline
                    .modules
                    .iter()
                    .any(|module| module.id == *source_id),
                _ => baseline
                    .embedded_circuits
                    .iter()
                    .any(|circuit| circuit.id == *source_id),
            };
            let target_exists = match operation {
                "remove-module" => baseline
                    .modules
                    .iter()
                    .any(|module| module.id == *mirror_id),
                _ => baseline
                    .embedded_circuits
                    .iter()
                    .any(|circuit| circuit.id == *mirror_id),
            };
            if !source_exists || !target_exists {
                return Err(format!(
                    "Hosted removal {source_id:?} is not owned by its complete pre-source baseline"
                ));
            }
            let baseline_pair = baseline
                .hardware
                .as_ref()
                .and_then(|hardware| hardware.pairs.iter().find(|pair| pair.id == pair_id))
                .ok_or_else(|| format!("Hosted removal {source_id:?} has no baseline pair"))?;
            let baseline_halves = baseline_pair.halves.as_ref().ok_or_else(|| {
                format!("Hosted removal {source_id:?} has no baseline halves context")
            })?;
            match operation {
                "remove-module" => {
                    let source = baseline
                        .modules
                        .iter()
                        .find(|module| module.id == *source_id)
                        .unwrap();
                    let target = baseline
                        .modules
                        .iter()
                        .find(|module| module.id == *mirror_id)
                        .unwrap();
                    let pair_associations: Vec<_> = baseline_halves
                        .correspondences
                        .iter()
                        .filter(|association| {
                            association.kind == MirroredItemKind::Module
                                && association.source_id == *source_id
                                && association.mirror_id == *mirror_id
                        })
                        .collect();
                    if pair_associations.len() != 1
                        || source.host_board_id != baseline_halves.source_board_id
                        || target.host_board_id != baseline_halves.mirror_board_id
                    {
                        return Err(format!(
                            "Hosted module removal {source_id:?} does not match one exact pair correspondence and its board owners"
                        ));
                    }
                }
                "remove-circuit" => {
                    let source = baseline
                        .embedded_circuits
                        .iter()
                        .find(|circuit| circuit.id == *source_id)
                        .unwrap();
                    let target = baseline
                        .embedded_circuits
                        .iter()
                        .find(|circuit| circuit.id == *mirror_id)
                        .unwrap();
                    let pair_associations: Vec<_> = baseline_halves
                        .correspondences
                        .iter()
                        .filter(|association| {
                            association.kind == MirroredItemKind::EmbeddedCircuit
                                && association.source_id == *source_id
                                && association.mirror_id == *mirror_id
                        })
                        .collect();
                    if pair_associations.len() != 1
                        || source.host_board_id != baseline_halves.source_board_id
                        || target.host_board_id != baseline_halves.mirror_board_id
                    {
                        return Err(format!(
                            "Hosted circuit removal {source_id:?} does not match one exact pair correspondence and its board owners"
                        ));
                    }
                    let source_parts: BTreeSet<_> = source.part_ids.iter().cloned().collect();
                    let target_parts: BTreeSet<_> = target.part_ids.iter().cloned().collect();
                    let source_nets: BTreeSet<_> = source.net_ids.iter().cloned().collect();
                    let target_nets: BTreeSet<_> = target.net_ids.iter().cloned().collect();
                    let source_prefix = format!("embedded/{source_id}/");
                    let mirror_prefix = format!("embedded/{mirror_id}/");
                    for association in &baseline_halves.correspondences {
                        let (source_closure, target_closure) = match association.kind {
                            MirroredItemKind::Part => (&source_parts, &target_parts),
                            MirroredItemKind::Net => (&source_nets, &target_nets),
                            _ => continue,
                        };
                        let source_owned = association.source_id.starts_with(&source_prefix);
                        let target_owned = association.mirror_id.starts_with(&mirror_prefix);
                        if source_owned != target_owned
                            || (source_owned
                                && (!source_closure.contains(&association.source_id)
                                    || !target_closure.contains(&association.mirror_id)))
                        {
                            return Err(format!(
                                "Hosted circuit removal {source_id:?} has a child correspondence outside its exact owned closure"
                            ));
                        }
                    }
                }
                _ => unreachable!(),
            }
            let source_still_exists = match operation {
                "remove-module" => document
                    .modules
                    .iter()
                    .any(|module| module.id == *source_id),
                _ => document
                    .embedded_circuits
                    .iter()
                    .any(|circuit| circuit.id == *source_id),
            };
            if source_still_exists {
                return Err(format!(
                    "Hosted removal {source_id:?} has not completed its source operation"
                ));
            }
        }
        Ok(())
    }

    fn validate_hosted_omission_baselines(
        document: &ProjectDoc,
        pair_id: &str,
        projection: &Projection,
    ) -> Result<HostedOmissionClosure, String> {
        fn pair_bytes(document: &ProjectDoc, pair_id: &str) -> Result<Vec<u8>, String> {
            let hardware = document.hardware.as_ref().ok_or_else(|| {
                "The hosted omission hardware configuration is unavailable".to_owned()
            })?;
            let mut pairs = hardware.pairs.iter().filter(|pair| pair.id == pair_id);
            let pair = pairs
                .next()
                .ok_or_else(|| format!("The hosted omission pair {pair_id:?} is unavailable"))?;
            if pairs.next().is_some() {
                return Err(format!("The hosted omission pair {pair_id:?} is ambiguous"));
            }
            serde_json::to_vec(pair).map_err(|error| error.to_string())
        }

        let mut closure = HostedOmissionClosure::default();
        for ((context_pair_id, source_id, mirror_id), baseline) in
            projection.hosted_omission_baselines.borrow().iter()
        {
            if context_pair_id != pair_id {
                continue;
            }
            crate::mechanical::halves::validate_document(baseline).map_err(|error| {
                format!(
                    "Hosted omission context for {source_id:?} has an invalid historical owner baseline: {error}"
                )
            })?;
            if pair_bytes(baseline, pair_id)? != pair_bytes(document, pair_id)? {
                return Err(format!(
                    "Hosted omission context for {source_id:?} changed its durable pair record"
                ));
            }
            let pair = baseline
                .hardware
                .as_ref()
                .and_then(|hardware| hardware.pairs.iter().find(|pair| pair.id == pair_id))
                .ok_or_else(|| format!("Hosted omission context for {source_id:?} has no pair"))?;
            let halves = pair.halves.as_ref().ok_or_else(|| {
                format!("Hosted omission context for {source_id:?} has no halves record")
            })?;
            if halves.source_board_id != projection.source_board_id
                || halves.mirror_board_id != projection.mirror_board_id
                || pair.mapping != projection.mapping
            {
                return Err(format!(
                    "Hosted omission context for {source_id:?} belongs to different boards or mapping"
                ));
            }
            let circuit_pairs: Vec<_> = halves
                .correspondences
                .iter()
                .filter(|association| {
                    association.kind == MirroredItemKind::EmbeddedCircuit
                        && association.source_id == *source_id
                        && association.mirror_id == *mirror_id
                })
                .collect();
            if circuit_pairs.len() != 1 {
                return Err(format!(
                    "Hosted omission context {source_id:?} → {mirror_id:?} lacks one exact circuit correspondence"
                ));
            }
            let source = baseline
                .embedded_circuits
                .iter()
                .find(|circuit| circuit.id == *source_id)
                .ok_or_else(|| format!("Hosted omission source circuit {source_id:?} is absent"))?;
            let mirror = baseline
                .embedded_circuits
                .iter()
                .find(|circuit| circuit.id == *mirror_id)
                .ok_or_else(|| format!("Hosted omission target circuit {mirror_id:?} is absent"))?;
            if source.host_board_id != halves.source_board_id
                || mirror.host_board_id != halves.mirror_board_id
            {
                return Err(format!(
                    "Hosted omission context {source_id:?} → {mirror_id:?} has stale circuit board ownership"
                ));
            }
            let current_source = document
                .embedded_circuits
                .iter()
                .find(|circuit| circuit.id == *source_id)
                .ok_or_else(|| format!("Hosted omission source circuit {source_id:?} is absent"))?;
            let current_mirror = document
                .embedded_circuits
                .iter()
                .find(|circuit| circuit.id == *mirror_id)
                .ok_or_else(|| format!("Hosted omission target circuit {mirror_id:?} is absent"))?;
            if current_source.host_board_id != halves.source_board_id
                || current_mirror.host_board_id != halves.mirror_board_id
            {
                return Err(format!(
                    "Hosted omission context {source_id:?} → {mirror_id:?} has stale current circuit ownership"
                ));
            }
            let source_prefix = format!("embedded/{source_id}/");
            let mirror_prefix = format!("embedded/{mirror_id}/");
            let mirror_board = document
                .boards
                .iter()
                .find(|board| board.id == halves.mirror_board_id)
                .ok_or_else(|| "The hosted omission target board is unavailable".to_owned())?;
            let baseline_mirror_board = baseline
                .boards
                .iter()
                .find(|board| board.id == halves.mirror_board_id)
                .ok_or_else(|| "The historical omission target board is unavailable".to_owned())?;
            let omissions: Vec<_> = halves
                .exceptions
                .iter()
                .filter(|exception| {
                    matches!(
                        exception.kind,
                        MirroredItemKind::Part | MirroredItemKind::Net
                    ) && exception.state == crate::model::MirroredExceptionState::Omitted
                        && exception.mirror_id.starts_with(&mirror_prefix)
                })
                .collect();
            if omissions.is_empty() {
                return Err(format!(
                    "Hosted omission context {source_id:?} → {mirror_id:?} has no explicit Omitted child owner facts"
                ));
            }
            for exception in omissions {
                let suffix = exception
                    .mirror_id
                    .strip_prefix(&mirror_prefix)
                    .ok_or_else(|| {
                        "The omitted identity is outside its circuit namespace".to_owned()
                    })?;
                let source_child_id = format!("{source_prefix}{suffix}");
                if exception.kind == MirroredItemKind::Part {
                    let baseline_part = baseline
                        .parts
                        .iter()
                        .find(|part| part.id == exception.mirror_id)
                        .ok_or_else(|| {
                            format!(
                                "The Omitted owner baseline lacks target part {:?}",
                                exception.mirror_id
                            )
                        })?;
                    if !source.part_ids.contains(&source_child_id)
                        || !mirror.part_ids.contains(&exception.mirror_id)
                        || !baseline_mirror_board
                            .part_ids
                            .contains(&exception.mirror_id)
                        || !baseline.parts.iter().any(|part| part.id == source_child_id)
                        || baseline_part.id != exception.mirror_id
                        || !current_source.part_ids.contains(&source_child_id)
                        || current_mirror.part_ids.contains(&exception.mirror_id)
                        || mirror_board.part_ids.contains(&exception.mirror_id)
                        || document
                            .parts
                            .iter()
                            .any(|part| part.id == exception.mirror_id)
                        || document.nets.iter().any(|net| {
                            net.pins
                                .iter()
                                .any(|pin| pin.part_id == exception.mirror_id)
                        })
                    {
                        return Err(format!(
                            "The Omitted target part {:?} does not match its exact absent child closure",
                            exception.mirror_id
                        ));
                    }
                    if halves.correspondences.iter().any(|association| {
                        association.kind == MirroredItemKind::Part
                            && (association.source_id == source_child_id
                                || association.mirror_id == exception.mirror_id)
                    }) {
                        return Err(format!(
                            "The Omitted target part {:?} still has a durable correspondence",
                            exception.mirror_id
                        ));
                    }
                    closure.parts.insert(exception.mirror_id.clone());
                } else {
                    let baseline_net = baseline
                        .nets
                        .iter()
                        .find(|net| net.id == exception.mirror_id)
                        .ok_or_else(|| {
                            format!(
                                "The Omitted owner baseline lacks target net {:?}",
                                exception.mirror_id
                            )
                        })?;
                    if !source.net_ids.contains(&source_child_id)
                        || !mirror.net_ids.contains(&exception.mirror_id)
                        || !baseline_mirror_board.net_ids.contains(&exception.mirror_id)
                        || !baseline.nets.iter().any(|net| net.id == source_child_id)
                        || baseline_net.id != exception.mirror_id
                        || !current_source.net_ids.contains(&source_child_id)
                        || current_mirror.net_ids.contains(&exception.mirror_id)
                        || mirror_board.net_ids.contains(&exception.mirror_id)
                        || document
                            .nets
                            .iter()
                            .any(|net| net.id == exception.mirror_id)
                        || document.embedded_circuits.iter().any(|circuit| {
                            circuit
                                .ports
                                .values()
                                .any(|net| net == &exception.mirror_id)
                        })
                    {
                        return Err(format!(
                            "The Omitted target net {:?} does not match its exact absent child closure",
                            exception.mirror_id
                        ));
                    }
                    if halves.correspondences.iter().any(|association| {
                        association.kind == MirroredItemKind::Net
                            && (association.source_id == source_child_id
                                || association.mirror_id == exception.mirror_id)
                    }) {
                        return Err(format!(
                            "The Omitted target net {:?} still has a durable correspondence",
                            exception.mirror_id
                        ));
                    }
                    closure.nets.insert(exception.mirror_id.clone());
                }
            }
        }
        Ok(closure)
    }

    fn stage_matrix(
        document: &ProjectDoc,
        halves: &crate::model::MirroredHalves,
        projection: &Projection,
        mapping: Pose2,
    ) -> Result<MatrixStage, String> {
        let Some(matrix_link) = projection.matrix.as_ref() else {
            if !projection.parts.is_empty() || !projection.locally_protected_mirror_parts.is_empty()
            {
                return Err("The matrixless projection retains Matrix ownership".into());
            }
            let (source_members, mirror_members) = matrix_members_for_pair(document, halves);
            return Ok(MatrixStage {
                draft: document.clone(),
                source_members,
                mirror_members,
                omitted_parts: BTreeSet::new(),
                allow_absent: true,
            });
        };
        let source_matrix_id = matrix_link.source_matrix_id.as_str();
        let mirror_matrix_id = matrix_link.mirror_matrix_id.as_str();
        let retired = projection.preserve_matrix_geometry
            && !document
                .matrices
                .iter()
                .any(|matrix| matrix.id == source_matrix_id)
            && !halves.correspondences.iter().any(|correspondence| {
                correspondence.kind == MirroredItemKind::Matrix
                    && (correspondence.source_id == source_matrix_id
                        || correspondence.mirror_id == mirror_matrix_id)
            })
            && !halves.exceptions.iter().any(|exception| {
                exception.kind == MirroredItemKind::Matrix
                    && exception.mirror_id == mirror_matrix_id
            });
        if retired {
            if !projection.parts.is_empty() || !projection.locally_protected_mirror_parts.is_empty()
            {
                return Err("The retired Matrix projection retains active ownership".into());
            }
            let (source_members, mirror_members) = matrix_members_for_pair(document, halves);
            return Ok(MatrixStage {
                draft: document.clone(),
                source_members,
                mirror_members,
                omitted_parts: BTreeSet::new(),
                allow_absent: true,
            });
        }
        let mut matrix_correspondences = halves.correspondences.iter().filter(|correspondence| {
            correspondence.kind == MirroredItemKind::Matrix
                && (correspondence.source_id == source_matrix_id
                    || correspondence.mirror_id == mirror_matrix_id
                    || correspondence.source_id == mirror_matrix_id
                    || correspondence.mirror_id == source_matrix_id)
        });
        // A Local or Omitted matrix keeps its correspondence; the overlay is only its state.
        let exact_matrix_exception = halves.exceptions.iter().find(|exception| {
            exception.kind == MirroredItemKind::Matrix
                && exception.mirror_id == mirror_matrix_id
                && halves.overlay_source(exception) == Some(source_matrix_id)
        });
        let matrix_correspondence = matrix_correspondences
            .next()
            .ok_or_else(|| "The matrix correspondence is unavailable".to_owned())?;
        if matrix_correspondences.next().is_some()
            || matrix_correspondence.source_id != source_matrix_id
            || matrix_correspondence.mirror_id != mirror_matrix_id
        {
            return Err(
                "The matrix correspondence is ambiguous or does not match the input".into(),
            );
        }

        let source_matrix = matrix_by_id(document, source_matrix_id)?;
        let absent_matrix;
        let mirror_matrix = match matrix_by_id(document, mirror_matrix_id) {
            Ok(matrix) => matrix,
            Err(_)
                if exact_matrix_exception.is_some_and(|exception| {
                    exception.state == crate::model::MirroredExceptionState::Omitted
                }) =>
            {
                absent_matrix = omitted_mirror_matrix(
                    source_matrix,
                    mirror_matrix_id,
                    &projection.mirror_board_id,
                    halves,
                );
                &absent_matrix
            }
            Err(error) => return Err(error),
        };
        if source_matrix.board_id.as_deref() != Some(halves.source_board_id.as_str())
            || mirror_matrix.board_id.as_deref() != Some(halves.mirror_board_id.as_str())
        {
            return Err(
                "The matrix correspondence does not belong to its source and mirror boards".into(),
            );
        }

        crate::matrix::valid_matrix(source_matrix, document)?;
        crate::matrix::valid_matrix(mirror_matrix, document)?;
        if !projection.mapping.at.x.is_finite()
            || !projection.mapping.at.y.is_finite()
            || !projection.mapping.rotation.is_finite()
        {
            return Err("The reconciliation mapping contains non-finite geometry".into());
        }

        let view = if exact_matrix_exception.is_some() || projection.preserve_matrix_geometry {
            FixedMembershipView {
                normalized_source_matrix: source_matrix.clone(),
                incoming_matrix: mirror_matrix.clone(),
                retained_cells: Vec::new(),
                unassociated_companion_parts: Vec::new(),
                omitted_parts: BTreeSet::new(),
            }
        } else {
            let projected_matrix =
                project_matrix(source_matrix, mirror_matrix, matrix_correspondence, mapping)?;
            crate::matrix::valid_matrix(&projected_matrix, document)?;
            build_fixed_membership_view(
                document,
                halves,
                source_matrix,
                mirror_matrix,
                &projected_matrix,
                projection,
            )?
        };

        // Keep both retained parts out of the incoming pass and the previous ownership
        // view, so the destructive setter cannot remove their parts or references.
        let normalized_source_matrix = view.normalized_source_matrix;
        let incoming_matrix = view.incoming_matrix;
        let mut staged = document.clone();
        if exact_matrix_exception.is_none() && !projection.preserve_matrix_geometry {
            let staged_old_matrix = staged
                .matrices
                .iter_mut()
                .find(|matrix| matrix.id == mirror_matrix.id)
                .ok_or_else(|| {
                    "The mirrored matrix disappeared from the staged document".to_owned()
                })?;
            let retained_ids: std::collections::BTreeSet<_> = view
                .retained_cells
                .iter()
                .flat_map(|retained| retained.member_ids.iter().cloned())
                .collect();
            staged_old_matrix
                .part_ids
                .retain(|id| !retained_ids.contains(id));
            if normalized_source_matrix.cells.len() != source_matrix.cells.len() {
                crate::matrix::set_matrix(&mut staged, &normalized_source_matrix)?;
            }
            crate::matrix::set_matrix(&mut staged, &incoming_matrix)?;
            for retained in &view.unassociated_companion_parts {
                let target = staged
                    .parts
                    .iter_mut()
                    .find(|part| part.id == retained.id)
                    .ok_or_else(|| {
                        format!(
                            "The unassociated matrix companion {:?} disappeared",
                            retained.id
                        )
                    })?;
                *target = retained.clone();
            }

            {
                let normalized_mirror_matrix = staged
                    .matrices
                    .iter_mut()
                    .find(|matrix| matrix.id == mirror_matrix.id)
                    .ok_or_else(|| "The normalized mirrored matrix is unavailable".to_owned())?;
                for retained in &view.retained_cells {
                    let normalized_cell = normalized_mirror_matrix
                        .cells
                        .iter_mut()
                        .find(|cell| {
                            cell.row == retained.cell.row && cell.column == retained.cell.column
                        })
                        .ok_or_else(|| {
                            "The normalized retained matrix cell is unavailable".to_owned()
                        })?;
                    *normalized_cell = retained.cell.clone();
                }
                let mut ordered_cells: Vec<_> = normalized_mirror_matrix.cells.iter().collect();
                ordered_cells.sort_by_key(|cell| (cell.row, cell.column));
                normalized_mirror_matrix.part_ids = ordered_cells
                    .into_iter()
                    .filter(|cell| cell.enabled && !cell.deleted)
                    .flat_map(|cell| {
                        let key_id =
                            format!("matrix/{}/r{}c{}", mirror_matrix.id, cell.row, cell.column);
                        std::iter::once(key_id.clone()).chain(cell.assemblies.iter().map(
                            move |assembly| crate::matrix::companion_id(&key_id, &assembly.id),
                        ))
                    })
                    .collect();
            }
        }

        let (source_matrix_members, mirror_matrix_members) =
            matrix_members_for_pair(document, halves);
        Ok(MatrixStage {
            draft: staged,
            source_members: source_matrix_members,
            mirror_members: mirror_matrix_members,
            omitted_parts: view.omitted_parts,
            allow_absent: exact_matrix_exception
                .is_some_and(|e| e.state == crate::model::MirroredExceptionState::Omitted),
        })
    }

    let hardware = document
        .hardware
        .as_ref()
        .ok_or_else(|| "The mirrored hardware configuration is unavailable".to_owned())?;
    let mut pairs = hardware.pairs.iter().filter(|pair| pair.id == pair_id);
    let pair = pairs
        .next()
        .ok_or_else(|| format!("The mirrored pair {pair_id:?} is unavailable"))?;
    if pairs.next().is_some() {
        return Err(format!("The mirrored pair {pair_id:?} is ambiguous"));
    }
    let halves = pair
        .halves
        .as_ref()
        .ok_or_else(|| format!("The mirrored pair {pair_id:?} has no halves record"))?;
    if halves.source_board_id != projection.source_board_id
        || halves.mirror_board_id != projection.mirror_board_id
        || pair.mapping != projection.mapping
    {
        return Err("The reconciliation input does not match its canonical pair".into());
    }
    validate_hosted_removal_baselines(document, pair_id, projection)?;
    let hosted_omission_closure =
        validate_hosted_omission_baselines(document, pair_id, projection)?;

    let MatrixStage {
        draft: mut staged,
        source_members: source_matrix_members,
        mirror_members: mirror_matrix_members,
        omitted_parts,
        allow_absent: allow_absent_matrix,
    } = stage_matrix(document, halves, projection, pair.mapping)?;
    let planned_independent_additions: BTreeMap<_, _> = projection
        .independent_part_transitions
        .iter()
        .filter_map(|transition| match transition {
            IndependentPartTransition::Add {
                source_id,
                mirror_id,
                mirror_definition_id,
                mirror_reference,
            } => Some((
                source_id.as_str(),
                (
                    mirror_id.as_str(),
                    mirror_definition_id.as_str(),
                    mirror_reference.as_str(),
                ),
            )),
            IndependentPartTransition::Remove { .. } => None,
        })
        .collect();
    let mut planned_hosted_operations = BTreeSet::new();
    let planned_independent_removals: BTreeMap<_, _> = projection
        .independent_part_transitions
        .iter()
        .filter_map(|transition| match transition {
            IndependentPartTransition::Remove {
                source_id,
                mirror_id,
            } => Some((source_id.as_str(), mirror_id.as_str())),
            IndependentPartTransition::Add { .. } => None,
        })
        .collect();
    if planned_independent_additions.len()
        != projection
            .independent_part_transitions
            .iter()
            .filter(|transition| matches!(transition, IndependentPartTransition::Add { .. }))
            .count()
    {
        return Err(
            "Independent Part addition evidence contains duplicate source identities".into(),
        );
    }
    if planned_independent_removals.len()
        != projection
            .independent_part_transitions
            .iter()
            .filter(|transition| matches!(transition, IndependentPartTransition::Remove { .. }))
            .count()
    {
        return Err(
            "Independent Part removal evidence contains duplicate source identities".into(),
        );
    }
    let mut independent_part_targets = BTreeMap::new();
    let hosted_circuit_members: BTreeSet<String> = halves
        .correspondences
        .iter()
        .filter(|association| association.kind == MirroredItemKind::EmbeddedCircuit)
        .flat_map(|association| {
            document
                .embedded_circuits
                .iter()
                .filter(move |circuit| {
                    circuit.id == association.source_id || circuit.id == association.mirror_id
                })
                .flat_map(|circuit| circuit.part_ids.iter().cloned())
        })
        .collect();
    for correspondence in halves
        .correspondences
        .iter()
        .filter(|correspondence| correspondence.kind == MirroredItemKind::Part)
    {
        if hosted_circuit_members.contains(&correspondence.source_id)
            || hosted_circuit_members.contains(&correspondence.mirror_id)
        {
            continue;
        }
        // A Local or Omitted item keeps its correspondence but is never projected over. A Local
        // independent target is still the item a layout names; an Omitted one is absent.
        if let Some(overlay) = halves.overlay_of(correspondence) {
            if overlay.state == crate::model::MirroredExceptionState::Local
                && !source_matrix_members.contains(&correspondence.source_id)
            {
                if unique_board_owner(document, &correspondence.source_id, "mirrored")?
                    != halves.source_board_id
                    || unique_board_owner(document, &correspondence.mirror_id, "mirrored")?
                        != halves.mirror_board_id
                {
                    return Err("An independent Local Part has stale ownership".into());
                }
                independent_part_targets.insert(
                    correspondence.source_id.clone(),
                    correspondence.mirror_id.clone(),
                );
            }
            continue;
        }
        let source_is_matrix_member = source_matrix_members.contains(&correspondence.source_id);
        let mirror_is_matrix_member = mirror_matrix_members.contains(&correspondence.mirror_id);
        if source_is_matrix_member || mirror_is_matrix_member {
            if source_is_matrix_member != mirror_is_matrix_member {
                return Err("A part correspondence crosses the matrix membership boundary".into());
            }
            continue;
        }

        if planned_independent_additions.contains_key(correspondence.source_id.as_str()) {
            return Err(format!(
                "The independent Part addition {:?} already has pair correspondence",
                correspondence.source_id
            ));
        }
        let pending_circuit_removal = projection.hosted_transitions.iter().any(|transition| {
            let HostedTransition::RemoveCircuit {
                source_id,
                mirror_id,
            } = transition
            else {
                return false;
            };
            correspondence
                .source_id
                .strip_prefix(&format!("embedded/{source_id}/"))
                .is_some()
                && correspondence
                    .mirror_id
                    .strip_prefix(&format!("embedded/{mirror_id}/"))
                    .is_some()
        });
        if pending_circuit_removal {
            continue;
        }
        if let Some(mirror_id) = planned_independent_removals.get(correspondence.source_id.as_str())
        {
            if *mirror_id != correspondence.mirror_id {
                return Err(format!(
                    "The independent Part removal for {:?} names a conflicting mirror identity",
                    correspondence.source_id
                ));
            }
            continue;
        }

        if unique_board_owner(document, &correspondence.source_id, "mirrored")?
            != halves.source_board_id
            || unique_board_owner(document, &correspondence.mirror_id, "mirrored")?
                != halves.mirror_board_id
        {
            return Err("An independent part correspondence has stale board ownership".into());
        }
        let mut sources = document
            .parts
            .iter()
            .filter(|part| part.id == correspondence.source_id);
        let source = sources.next().ok_or_else(|| {
            format!(
                "The source part {:?} is unavailable",
                correspondence.source_id
            )
        })?;
        if sources.next().is_some() {
            return Err(format!(
                "The source part {:?} is ambiguous",
                correspondence.source_id
            ));
        }
        let mut mirrors = document
            .parts
            .iter()
            .filter(|part| part.id == correspondence.mirror_id);
        let mirror = mirrors.next().ok_or_else(|| {
            format!(
                "The mirror part {:?} is unavailable",
                correspondence.mirror_id
            )
        })?;
        if mirrors.next().is_some() {
            return Err(format!(
                "The mirror part {:?} is ambiguous",
                correspondence.mirror_id
            ));
        }
        let projected = project_part(source, mirror, correspondence, pair.mapping)?;
        let mut staged_mirrors = staged
            .parts
            .iter_mut()
            .filter(|part| part.id == correspondence.mirror_id);
        let staged_mirror = staged_mirrors.next().ok_or_else(|| {
            format!(
                "The staged mirror part {:?} is unavailable",
                correspondence.mirror_id
            )
        })?;
        if staged_mirrors.next().is_some() {
            return Err(format!(
                "The staged mirror part {:?} is ambiguous",
                correspondence.mirror_id
            ));
        }
        *staged_mirror = projected;
        independent_part_targets.insert(
            correspondence.source_id.clone(),
            correspondence.mirror_id.clone(),
        );
    }

    let mut planned_target_ids = BTreeSet::new();
    let mut planned_target_references = BTreeSet::new();
    for transition in &projection.independent_part_transitions {
        let IndependentPartTransition::Add {
            mirror_id,
            mirror_reference,
            ..
        } = transition
        else {
            continue;
        };
        if !planned_target_ids.insert(mirror_id) {
            return Err(format!(
                "Independent Part additions select the same target identity {mirror_id:?}"
            ));
        }
        if !planned_target_references.insert(mirror_reference) {
            return Err(format!(
                "Independent Part additions select the same target reference {mirror_reference:?}"
            ));
        }
    }

    for transition in &projection.independent_part_transitions {
        let IndependentPartTransition::Add {
            source_id,
            mirror_id,
            mirror_definition_id,
            mirror_reference,
        } = transition
        else {
            continue;
        };
        if source_matrix_members.contains(source_id) || mirror_matrix_members.contains(mirror_id) {
            return Err("An independent Part addition aliases matrix membership".into());
        }
        if halves.correspondences.iter().any(|correspondence| {
            correspondence.kind == MirroredItemKind::Part
                && (correspondence.source_id == *source_id
                    || correspondence.mirror_id == *mirror_id)
        }) {
            return Err(format!(
                "The independent Part addition {source_id:?} has conflicting pair provenance"
            ));
        }
        if unique_board_owner(document, source_id, "mirrored")? != halves.source_board_id {
            return Err(format!(
                "The independent Part addition {source_id:?} has stale source ownership"
            ));
        }
        let mut sources = document.parts.iter().filter(|part| part.id == *source_id);
        let source = sources
            .next()
            .ok_or_else(|| format!("The independent source Part {source_id:?} is unavailable"))?;
        if sources.next().is_some() {
            return Err(format!(
                "The independent source Part {source_id:?} is ambiguous"
            ));
        }
        let mut definitions = document
            .definitions
            .iter()
            .filter(|definition| definition.id == *mirror_definition_id);
        if definitions.next().is_none() || definitions.next().is_some() {
            return Err(format!(
                "The independent target definition {mirror_definition_id:?} is unavailable or ambiguous"
            ));
        }
        if mirror_id.is_empty() || mirror_reference.is_empty() {
            return Err(format!(
                "The independent target identity {mirror_id:?} and reference must be nonempty"
            ));
        }
        let mut existing_targets = document.parts.iter().filter(|part| part.id == *mirror_id);
        let existing_target = existing_targets.next();
        if existing_targets.next().is_some()
            || document
                .parts
                .iter()
                .any(|part| part.reference == *mirror_reference && part.id != *mirror_id)
        {
            return Err(format!(
                "The independent target identity {mirror_id:?} or reference {mirror_reference:?} is ambiguous or already in use"
            ));
        }
        if existing_target.is_some_and(|part| {
            part.definition_id != *mirror_definition_id || part.reference != *mirror_reference
        }) {
            return Err(format!(
                "The existing independent target {mirror_id:?} does not match its selected definition and reference"
            ));
        }
        let has_prior_add_provenance =
            projection
                .applied_independent_additions
                .borrow()
                .contains(&(
                    source_id.clone(),
                    mirror_id.clone(),
                    mirror_definition_id.clone(),
                    mirror_reference.clone(),
                ));
        if existing_target.is_some() && !has_prior_add_provenance {
            return Err(format!(
                "The independent target {mirror_id:?} has no prior private Add provenance"
            ));
        }
        if existing_target.is_some_and(|_| {
            unique_board_owner(document, mirror_id, "mirrored").ok()
                != Some(halves.mirror_board_id.as_str())
        }) || (existing_target.is_none()
            && document
                .boards
                .iter()
                .any(|board| board.part_ids.contains(mirror_id)))
        {
            return Err(format!(
                "The independent target {mirror_id:?} has conflicting mirror-board registration"
            ));
        }
        let correspondence = MirroredCorrespondence {
            companion_mode: None,
            companion_tweak: None,
            kind: MirroredItemKind::Part,
            source_id: source_id.clone(),
            mirror_id: mirror_id.clone(),
        };
        let target_seed = existing_target.cloned().unwrap_or_else(|| Part {
            id: mirror_id.clone(),
            definition_id: mirror_definition_id.clone(),
            reference: mirror_reference.clone(),
            pose: Pose2 {
                at: Vec2::default(),
                rotation: 0.0,
            },
            side: source.side.clone(),
            keycap: None,
            outline: None,
            locked: None,
            properties: None,
            generator_parameters: None,
        });
        let projected = project_part(source, &target_seed, &correspondence, pair.mapping)?;
        if let Some(staged_target) = staged.parts.iter_mut().find(|part| part.id == *mirror_id) {
            *staged_target = projected;
        } else {
            staged.parts.push(projected);
            staged
                .boards
                .iter_mut()
                .find(|board| board.id == halves.mirror_board_id)
                .ok_or_else(|| "The independent target board is unavailable".to_owned())?
                .part_ids
                .push(mirror_id.clone());
        }
        independent_part_targets.insert(source_id.clone(), mirror_id.clone());
    }

    let mut removed_independent_targets = BTreeSet::new();
    for transition in &projection.independent_part_transitions {
        let IndependentPartTransition::Remove {
            source_id,
            mirror_id,
        } = transition
        else {
            continue;
        };
        if source_matrix_members.contains(source_id) || mirror_matrix_members.contains(mirror_id) {
            return Err("An independent Part removal aliases matrix membership".into());
        }
        if document.parts.iter().any(|part| part.id == *source_id)
            || document
                .boards
                .iter()
                .any(|board| board.part_ids.contains(source_id))
        {
            return Err(format!(
                "The removed independent source Part {source_id:?} remains materialized"
            ));
        }
        if halves.correspondences.iter().any(|correspondence| {
            correspondence.kind == MirroredItemKind::Part
                && (correspondence.source_id == *source_id
                    || correspondence.mirror_id == *mirror_id)
                && (correspondence.source_id != *source_id
                    || correspondence.mirror_id != *mirror_id)
        }) {
            return Err(format!(
                "The independent Part removal {source_id:?} has conflicting pair provenance"
            ));
        }
        let target_count = document
            .parts
            .iter()
            .filter(|part| part.id == *mirror_id)
            .count();
        if !removed_independent_targets.insert(mirror_id.clone()) {
            return Err(format!(
                "The independent target Part {mirror_id:?} is unavailable, aliased, or not mirror-owned"
            ));
        }
        match target_count {
            0 => {
                if document
                    .boards
                    .iter()
                    .any(|board| board.part_ids.contains(mirror_id))
                    || document
                        .layouts
                        .iter()
                        .any(|layout| layout.part_ids.contains(mirror_id))
                {
                    return Err(format!(
                        "The removed independent target Part {mirror_id:?} retains an owner or layout membership"
                    ));
                }
                continue;
            }
            1 if unique_board_owner(document, mirror_id, "mirrored")? == halves.mirror_board_id => {
            }
            _ => {
                return Err(format!(
                    "The independent target Part {mirror_id:?} is unavailable, aliased, or not mirror-owned"
                ));
            }
        }
        crate::matrix::deletion::remove_parts_by_id(
            &mut staged,
            &BTreeSet::from([mirror_id.clone()]),
        );
    }

    // Three views of the correspondences. `part_targets` and `net_targets` are the linked items,
    // the only ones a projection may write over. `reference_*` also name Local items, so a
    // reference to a Local item (a circuit member, a module connector, a lock) still resolves.
    // `omitted_source_*` name the items a reference must drop.
    let overlay_state = |kind: MirroredItemKind, correspondence: &MirroredCorrespondence| {
        debug_assert_eq!(correspondence.kind, kind);
        halves
            .overlay_of(correspondence)
            .map(|exception| exception.state)
    };
    let part_targets: BTreeMap<_, _> = halves
        .correspondences
        .iter()
        .filter(|association| {
            association.kind == MirroredItemKind::Part
                && overlay_state(MirroredItemKind::Part, association).is_none()
        })
        .map(|association| (association.source_id.clone(), association.mirror_id.clone()))
        .collect();
    let mut reference_part_targets: BTreeMap<_, _> = halves
        .correspondences
        .iter()
        .filter(|association| {
            association.kind == MirroredItemKind::Part
                && overlay_state(MirroredItemKind::Part, association)
                    != Some(crate::model::MirroredExceptionState::Omitted)
        })
        .map(|association| (association.source_id.clone(), association.mirror_id.clone()))
        .collect();
    let omitted_part_sources: BTreeSet<String> = halves
        .correspondences
        .iter()
        .filter(|association| {
            association.kind == MirroredItemKind::Part
                && overlay_state(MirroredItemKind::Part, association)
                    == Some(crate::model::MirroredExceptionState::Omitted)
        })
        .map(|association| association.source_id.clone())
        .collect();
    let net_targets: BTreeMap<_, _> = halves
        .correspondences
        .iter()
        .filter(|association| {
            association.kind == MirroredItemKind::Net
                && overlay_state(MirroredItemKind::Net, association).is_none()
        })
        .map(|association| (association.source_id.clone(), association.mirror_id.clone()))
        .collect();
    let mut reference_net_targets: BTreeMap<_, _> = halves
        .correspondences
        .iter()
        .filter(|association| {
            association.kind == MirroredItemKind::Net
                && overlay_state(MirroredItemKind::Net, association)
                    != Some(crate::model::MirroredExceptionState::Omitted)
        })
        .map(|association| (association.source_id.clone(), association.mirror_id.clone()))
        .collect();
    let compose_reference_target = |targets: &mut BTreeMap<String, String>,
                                    source_id: &str,
                                    mirror_id: &str,
                                    kind: MirroredItemKind|
     -> Result<(), String> {
        if targets.iter().any(|(existing_source, existing_mirror)| {
            existing_source != source_id && existing_mirror == mirror_id
        }) {
            return Err(format!(
                "The selected hosted {kind:?} reference target {mirror_id:?} is already in use"
            ));
        }
        match targets.get(source_id) {
            Some(existing) if existing == mirror_id => Ok(()),
            Some(_) => Err(format!(
                "The selected hosted {kind:?} reference conflicts with an existing identity"
            )),
            None => {
                targets.insert(source_id.to_owned(), mirror_id.to_owned());
                Ok(())
            }
        }
    };
    for transition in &projection.hosted_transitions {
        if let HostedTransition::AddCircuit {
            member_ids,
            net_ids,
            ..
        } = transition
        {
            for (source_id, mirror_id) in member_ids {
                compose_reference_target(
                    &mut reference_part_targets,
                    source_id,
                    mirror_id,
                    MirroredItemKind::Part,
                )?;
            }
            for (source_id, mirror_id) in net_ids {
                compose_reference_target(
                    &mut reference_net_targets,
                    source_id,
                    mirror_id,
                    MirroredItemKind::Net,
                )?;
            }
        }
    }
    let replayed_circuit_additions: BTreeSet<_> = projection
        .hosted_transitions
        .iter()
        .filter_map(|transition| match transition {
            HostedTransition::AddCircuit {
                source_id,
                mirror_id,
                ..
            } if projection
                .applied_hosted_circuit_additions
                .borrow()
                .contains_key(&("circuit".to_owned(), source_id.clone(), mirror_id.clone())) =>
            {
                Some((source_id.clone(), mirror_id.clone()))
            }
            _ => None,
        })
        .collect();
    let circuit_pairs: Vec<_> = halves
        .correspondences
        .iter()
        .filter(|association| {
            association.kind == MirroredItemKind::EmbeddedCircuit
                && !replayed_circuit_additions
                    .contains(&(association.source_id.clone(), association.mirror_id.clone()))
        })
        .map(|association| (association.source_id.clone(), association.mirror_id.clone()))
        .collect();
    for (source_id, mirror_id) in circuit_pairs {
        let omitted = halves.exceptions.iter().any(|exception| {
            exception.kind == MirroredItemKind::EmbeddedCircuit
                && exception.mirror_id == mirror_id
                && exception.state == crate::model::MirroredExceptionState::Omitted
        });
        let source = document
            .embedded_circuits
            .iter()
            .find(|circuit| circuit.id == source_id);
        if omitted || source.is_none() {
            let removal_requested = !omitted && projection.hosted_transitions.iter().any(|transition| {
                matches!(transition, HostedTransition::RemoveCircuit { source_id: s, mirror_id: m }
                    if s == &source_id && m == &mirror_id)
            });
            if removal_requested {
                let mut removed_parts: BTreeSet<_> = halves
                    .exceptions
                    .iter()
                    .filter(|exception| {
                        exception.kind == MirroredItemKind::Part
                            && exception.state == crate::model::MirroredExceptionState::Omitted
                            && exception
                                .mirror_id
                                .starts_with(&format!("embedded/{mirror_id}/"))
                    })
                    .map(|exception| exception.mirror_id.clone())
                    .collect();
                let mut removed_nets: BTreeSet<_> = halves
                    .exceptions
                    .iter()
                    .filter(|exception| {
                        exception.kind == MirroredItemKind::Net
                            && exception.state == crate::model::MirroredExceptionState::Omitted
                            && exception
                                .mirror_id
                                .starts_with(&format!("embedded/{mirror_id}/"))
                    })
                    .map(|exception| exception.mirror_id.clone())
                    .collect();
                if let Some(target_circuit) = staged
                    .embedded_circuits
                    .iter()
                    .find(|circuit| circuit.id == mirror_id)
                {
                    removed_parts.extend(target_circuit.part_ids.iter().cloned());
                    removed_nets.extend(target_circuit.net_ids.iter().cloned());
                    crate::modules::remove_circuit(&mut staged, &mirror_id)?;
                }
            }
            continue;
        }
        let source = source.unwrap();
        let target = staged
            .embedded_circuits
            .iter_mut()
            .find(|circuit| circuit.id == mirror_id)
            .ok_or_else(|| format!("The mirror embedded circuit {mirror_id:?} is unavailable"))?;
        let mut projected = source.clone();
        projected.id = mirror_id.clone();
        projected.host_board_id = halves.mirror_board_id.clone();
        projected.definition_id = target.definition_id.clone();
        // Only legacy, unassociated exceptions reach this: an item the mirror authored with no
        // correspondence is matched to its source member by namespace suffix. An associated Local
        // or Omitted item resolves through `reference_*_targets` above.
        let exception_target = |kind, member_id: &str| {
            let suffix = member_id.strip_prefix(&format!("embedded/{source_id}/"));
            halves.exceptions.iter().find_map(|exception| {
                if exception.kind != kind
                    || !matches!(
                        exception.state,
                        crate::model::MirroredExceptionState::Local
                            | crate::model::MirroredExceptionState::Omitted
                    )
                {
                    return None;
                }
                let target_suffix = exception
                    .mirror_id
                    .strip_prefix(&format!("embedded/{mirror_id}/"));
                let privately_absent = exception.state
                    == crate::model::MirroredExceptionState::Omitted
                    && match kind {
                        MirroredItemKind::Part => {
                            hosted_omission_closure.parts.contains(&exception.mirror_id)
                        }
                        MirroredItemKind::Net => {
                            hosted_omission_closure.nets.contains(&exception.mirror_id)
                        }
                        _ => false,
                    };
                if privately_absent {
                    return None;
                }
                (suffix.is_some() && suffix == target_suffix).then(|| exception.mirror_id.clone())
            })
        };
        projected.part_ids = source
            .part_ids
            .iter()
            .filter_map(|id| {
                reference_part_targets
                    .get(id)
                    .cloned()
                    .or_else(|| exception_target(MirroredItemKind::Part, id))
            })
            .collect();
        projected.net_ids = source
            .net_ids
            .iter()
            .filter_map(|id| {
                reference_net_targets
                    .get(id)
                    .cloned()
                    .or_else(|| exception_target(MirroredItemKind::Net, id))
            })
            .collect();
        projected.ports = source
            .ports
            .iter()
            .filter_map(|(port, net)| {
                reference_net_targets
                    .get(net)
                    .cloned()
                    .or_else(|| exception_target(MirroredItemKind::Net, net))
                    .map(|target_net| (port.clone(), target_net))
            })
            .collect();
        *target = projected;

        for member_id in &source.part_ids {
            let Some(target_id) = part_targets.get(member_id) else {
                continue;
            };
            let source_part = document
                .parts
                .iter()
                .find(|part| part.id == *member_id)
                .ok_or_else(|| format!("The circuit member {member_id:?} is unavailable"))?;
            let target_part = staged
                .parts
                .iter_mut()
                .find(|part| part.id == *target_id)
                .ok_or_else(|| {
                    format!("The mirrored circuit member {target_id:?} is unavailable")
                })?;
            let correspondence = MirroredCorrespondence {
                companion_mode: None,
                companion_tweak: None,
                kind: MirroredItemKind::Part,
                source_id: member_id.clone(),
                mirror_id: target_id.clone(),
            };
            *target_part = project_part(source_part, target_part, &correspondence, pair.mapping)?;
        }
        for source_member_id in &source.part_ids {
            let Some(target_id) = part_targets.get(source_member_id) else {
                continue;
            };
            let source_part = document
                .parts
                .iter()
                .find(|part| part.id == *source_member_id)
                .ok_or_else(|| format!("The circuit member {source_member_id:?} is unavailable"))?;
            let target_part = staged
                .parts
                .iter()
                .find(|part| part.id == *target_id)
                .ok_or_else(|| {
                    format!("The mirrored circuit member {target_id:?} is unavailable")
                })?;
            let source_namespace = format!("embedded/{source_id}/");
            let Some(suffix) = source_part.definition_id.strip_prefix(&source_namespace) else {
                continue;
            };
            let target_definition_id = format!("embedded/{mirror_id}/{suffix}");
            if let Some(definition) = document
                .definitions
                .iter()
                .find(|definition| definition.id == source_part.definition_id)
                && let Some(target_definition) = staged
                    .definitions
                    .iter_mut()
                    .find(|definition| definition.id == target_definition_id)
                && target_part.definition_id == target_definition_id
            {
                let mut projected_definition = definition.clone();
                projected_definition.id = target_definition_id;
                *target_definition = projected_definition;
            }
        }
        for net_id in &source.net_ids {
            let Some(target_id) = net_targets.get(net_id) else {
                continue;
            };
            let Some(source_net) = document.nets.iter().find(|net| net.id == *net_id) else {
                continue;
            };
            let Some(target_net) = staged.nets.iter_mut().find(|net| net.id == *target_id) else {
                continue;
            };
            let mut projected_net = source_net.clone();
            projected_net.id = target_id.clone();
            let source_owned_pins: Vec<_> = source_net
                .pins
                .iter()
                .filter_map(|pin| {
                    part_targets
                        .get(&pin.part_id)
                        .map(|part_id| crate::model::Pin {
                            part_id: part_id.clone(),
                            pad_id: pin.pad_id.clone(),
                        })
                })
                .collect();
            let linked_target_parts: BTreeSet<_> = part_targets.values().cloned().collect();
            let host_only_pins: Vec<_> = target_net
                .pins
                .iter()
                .filter(|pin| !linked_target_parts.contains(&pin.part_id))
                .cloned()
                .collect();
            projected_net.pins = source_owned_pins;
            projected_net.pins.extend(host_only_pins);
            *target_net = projected_net;
        }
    }

    for transition in &projection.hosted_transitions {
        let HostedTransition::AddCircuit {
            source_id,
            mirror_id,
            member_ids,
            member_references,
            definition_ids,
            net_ids,
            member_poses,
        } = transition
        else {
            continue;
        };
        let source = document
            .embedded_circuits
            .iter()
            .find(|circuit| circuit.id == *source_id)
            .ok_or_else(|| format!("The source embedded circuit {source_id:?} is unavailable"))?;
        let provenance_key = ("circuit".to_owned(), source_id.clone(), mirror_id.clone());
        let prior_provenance = projection
            .applied_hosted_circuit_additions
            .borrow()
            .get(&provenance_key)
            .cloned();
        let has_provenance = if let Some(prior) = &prior_provenance {
            if prior.member_ids != *member_ids
                || prior.member_references != *member_references
                || prior.definition_ids != *definition_ids
                || prior.net_ids != *net_ids
                || prior.member_poses != *member_poses
            {
                return Err(format!(
                    "The circuit addition {source_id:?} changed its selected maps after first output"
                ));
            }
            if prior.first_output != *document {
                return Err(format!(
                    "The circuit addition {source_id:?} no longer matches its complete first output"
                ));
            }
            true
        } else {
            if projection
                .applied_hosted_operations
                .borrow()
                .contains(&provenance_key)
            {
                return Err(format!(
                    "The circuit addition {source_id:?} has incomplete private replay provenance"
                ));
            }
            false
        };
        let member_targets: BTreeSet<_> = member_ids.values().cloned().collect();
        let definition_targets: BTreeSet<_> = definition_ids.values().cloned().collect();
        let net_targets_selected: BTreeSet<_> = net_ids.values().cloned().collect();
        let member_references_selected: BTreeSet<_> = member_references.values().collect();
        if member_targets.len() != member_ids.len()
            || definition_targets.len() != definition_ids.len()
            || net_targets_selected.len() != net_ids.len()
            || member_references_selected.len() != member_references.len()
            || member_ids.len() != source.part_ids.len()
            || member_ids.keys().any(|id| !source.part_ids.contains(id))
            || member_references
                .keys()
                .any(|id| !member_ids.contains_key(id))
            || source.net_ids.iter().any(|id| !net_ids.contains_key(id))
        {
            return Err("The circuit addition identity maps are incomplete or aliased".into());
        }
        let mut circuit = source.clone();
        circuit.id = mirror_id.clone();
        circuit.host_board_id = halves.mirror_board_id.clone();
        circuit.part_ids.clear();
        circuit.net_ids.clear();
        circuit.ports = source
            .ports
            .iter()
            .filter_map(|(port, id)| net_ids.get(id).map(|target| (port.clone(), target.clone())))
            .collect();
        for (source_member_id, target_member_id) in member_ids {
            let source_part = document
                .parts
                .iter()
                .find(|part| part.id == *source_member_id)
                .ok_or_else(|| format!("The circuit member {source_member_id:?} is unavailable"))?;
            let target_definition =
                definition_ids
                    .get(&source_part.definition_id)
                    .ok_or_else(|| {
                        format!(
                            "The circuit member definition {:?} has no selected target",
                            source_part.definition_id
                        )
                    })?;
            let target_reference = member_references.get(source_member_id).ok_or_else(|| {
                format!("The circuit member {source_member_id:?} has no selected reference")
            })?;
            let existing = staged
                .parts
                .iter()
                .position(|part| part.id == *target_member_id);
            if !has_provenance && existing.is_some() {
                return Err(format!(
                    "The selected target circuit member {target_member_id:?} already exists without private Add provenance"
                ));
            }
            if staged
                .parts
                .iter()
                .any(|part| part.reference == **target_reference && part.id != *target_member_id)
            {
                return Err(format!(
                    "The selected circuit member reference {target_reference:?} is already in use"
                ));
            }
            let correspondence = MirroredCorrespondence {
                companion_mode: None,
                companion_tweak: None,
                kind: MirroredItemKind::Part,
                source_id: source_member_id.clone(),
                mirror_id: target_member_id.clone(),
            };
            let target_part = if let Some(index) = existing {
                project_part(
                    source_part,
                    &staged.parts[index],
                    &correspondence,
                    pair.mapping,
                )?
            } else {
                let seed = Part {
                    id: target_member_id.clone(),
                    definition_id: target_definition.clone(),
                    reference: (*target_reference).clone(),
                    pose: Pose2 {
                        at: Vec2::default(),
                        rotation: 0.0,
                    },
                    side: source_part.side.clone(),
                    keycap: None,
                    outline: None,
                    locked: None,
                    properties: None,
                    generator_parameters: None,
                };
                project_part(source_part, &seed, &correspondence, pair.mapping)?
            };
            if let Some(index) = existing {
                staged.parts[index] = target_part;
            } else {
                staged.parts.push(target_part);
                let board = staged
                    .boards
                    .iter_mut()
                    .find(|board| board.id == halves.mirror_board_id)
                    .ok_or_else(|| "The target circuit board is unavailable".to_owned())?;
                if !board.part_ids.contains(target_member_id) {
                    board.part_ids.push(target_member_id.clone());
                }
            }
            circuit.part_ids.push(target_member_id.clone());
        }
        for (source_definition_id, target_definition_id) in definition_ids {
            let source_definition = document
                .definitions
                .iter()
                .find(|definition| definition.id == *source_definition_id)
                .ok_or_else(|| {
                    format!("The circuit definition {source_definition_id:?} is unavailable")
                })?;
            let mut definition = source_definition.clone();
            definition.id = target_definition_id.clone();
            if let Some(existing) = staged
                .definitions
                .iter_mut()
                .find(|existing| existing.id == *target_definition_id)
            {
                if !has_provenance {
                    return Err(format!(
                        "The selected target circuit definition {target_definition_id:?} already exists without private Add provenance"
                    ));
                }
                *existing = definition;
            } else {
                staged.definitions.push(definition);
            }
        }
        for source_net_id in &source.net_ids {
            let target_net_id = net_ids.get(source_net_id).ok_or_else(|| {
                format!("The circuit net {source_net_id:?} has no selected target")
            })?;
            let source_net = document
                .nets
                .iter()
                .find(|net| net.id == *source_net_id)
                .ok_or_else(|| format!("The circuit net {source_net_id:?} is unavailable"))?;
            let existing = staged.nets.iter().position(|net| net.id == *target_net_id);
            if let Some(index) = existing {
                let is_joined_host_net = halves.correspondences.iter().any(|association| {
                    association.kind == MirroredItemKind::Net
                        && association.source_id == *source_net_id
                        && association.mirror_id == *target_net_id
                });
                if !is_joined_host_net && !has_provenance {
                    return Err(format!(
                        "The selected target circuit net {target_net_id:?} already exists without private Add provenance"
                    ));
                }
                if is_joined_host_net {
                    let target_net = &mut staged.nets[index];
                    for (source_member_id, target_member_id) in member_ids {
                        for pin in source_net
                            .pins
                            .iter()
                            .filter(|pin| pin.part_id == *source_member_id)
                        {
                            let mapped = crate::model::Pin {
                                part_id: target_member_id.clone(),
                                pad_id: pin.pad_id.clone(),
                            };
                            if !target_net.pins.contains(&mapped) {
                                target_net.pins.push(mapped);
                            }
                        }
                    }
                } else {
                    let mut projected = source_net.clone();
                    projected.id = target_net_id.clone();
                    projected.pins = source_net
                        .pins
                        .iter()
                        .filter_map(|pin| {
                            member_ids.get(&pin.part_id).map(|id| crate::model::Pin {
                                part_id: id.clone(),
                                pad_id: pin.pad_id.clone(),
                            })
                        })
                        .collect();
                    staged.nets[index] = projected;
                }
            } else {
                let mut net = source_net.clone();
                net.id = target_net_id.clone();
                net.pins = source_net
                    .pins
                    .iter()
                    .filter_map(|pin| {
                        member_ids.get(&pin.part_id).map(|id| crate::model::Pin {
                            part_id: id.clone(),
                            pad_id: pin.pad_id.clone(),
                        })
                    })
                    .collect();
                staged.nets.push(net);
                let board = staged
                    .boards
                    .iter_mut()
                    .find(|board| board.id == halves.mirror_board_id)
                    .ok_or_else(|| "The target circuit board is unavailable".to_owned())?;
                if !board.net_ids.contains(target_net_id) {
                    board.net_ids.push(target_net_id.clone());
                }
            }
            circuit.net_ids.push(target_net_id.clone());
        }
        if let Some(existing) = staged
            .embedded_circuits
            .iter_mut()
            .find(|existing| existing.id == *mirror_id)
        {
            if !has_provenance {
                return Err(format!(
                    "The selected target circuit {mirror_id:?} already exists without private Add provenance"
                ));
            }
            *existing = circuit;
        } else {
            staged.embedded_circuits.push(circuit);
        }
        planned_hosted_operations.insert(provenance_key);
    }

    for correspondence in halves
        .correspondences
        .iter()
        .filter(|association| association.kind == MirroredItemKind::ElectricalBoard)
    {
        if correspondence.source_id != halves.source_board_id
            || correspondence.mirror_id != halves.mirror_board_id
            || halves.overlay_of(correspondence).is_some()
        {
            continue;
        }
        let source = document.hardware.as_ref().and_then(|hardware| {
            hardware
                .boards
                .iter()
                .find(|board| board.board_id == halves.source_board_id)
        });
        let target = staged.hardware.as_mut().and_then(|hardware| {
            hardware
                .boards
                .iter_mut()
                .find(|board| board.board_id == halves.mirror_board_id)
        });
        if let (Some(source), Some(target)) = (source, target) {
            let local_bindings = target.key_bindings.clone();
            let mirror_assignments = target.assignments.clone();
            let mirror_locks = target.locks.clone();
            // An item's function keys mention its part id as a whole path segment.
            let mentions = |key: &str, id: &str| {
                key == id
                    || key.starts_with(&format!("{id}/"))
                    || key.ends_with(&format!("/{id}"))
                    || key.contains(&format!("/{id}/"))
            };
            let mentions_omitted =
                |key: &str| omitted_part_sources.iter().any(|id| mentions(key, id));
            let map_key = |key: &str, ids: &BTreeMap<String, String>| {
                ids.iter()
                    .fold(key.to_owned(), |mapped, (source_id, target_id)| {
                        mapped.replace(source_id, target_id)
                    })
            };
            let map_value = |value: &String| {
                reference_net_targets
                    .get(value)
                    .or_else(|| reference_part_targets.get(value))
                    .cloned()
                    .unwrap_or_else(|| value.clone())
            };
            // Every corresponded part, Omitted ones included, so a stale mirror entry that names
            // an Omitted item is still recognized as derived from the source.
            let all_part_targets: BTreeMap<String, String> = halves
                .correspondences
                .iter()
                .filter(|association| association.kind == MirroredItemKind::Part)
                .map(|association| (association.source_id.clone(), association.mirror_id.clone()))
                .collect();
            let derived_keys: BTreeSet<String> = source
                .assignments
                .keys()
                .chain(source.locks.keys())
                .map(|key| map_key(key, &all_part_targets))
                .collect();
            let mut projected = source.clone();
            projected.board_id = halves.mirror_board_id.clone();
            // A Local controller is still the mirror's controller; an Omitted one has none.
            projected.controller_part_id = source
                .controller_part_id
                .as_ref()
                .and_then(|id| reference_part_targets.get(id).cloned());
            let mut assignments = BTreeMap::new();
            let mut locks = BTreeMap::new();
            // Mirrored items stay on the source's pins, so each mapped assignment is also a
            // lock. An Omitted item's entries are dropped, which frees its pins.
            for (key, value) in &source.assignments {
                if mentions_omitted(key) {
                    continue;
                }
                let (mapped_key, mapped_value) =
                    (map_key(key, &reference_part_targets), map_value(value));
                locks.insert(mapped_key.clone(), mapped_value.clone());
                assignments.insert(mapped_key, mapped_value);
            }
            for (key, value) in &source.locks {
                if mentions_omitted(key) {
                    continue;
                }
                locks.insert(map_key(key, &reference_part_targets), map_value(value));
            }
            // Entries the mirror authored itself survive unless they take a pin the source holds
            // for an item that is mirrored; that entry is released so Rejoin can reclaim the pin.
            let held: BTreeSet<String> = assignments
                .values()
                .chain(locks.values())
                .cloned()
                .collect();
            for (key, value) in mirror_assignments {
                if !derived_keys.contains(&key) && !held.contains(&value) {
                    assignments.entry(key).or_insert(value);
                }
            }
            for (key, value) in mirror_locks {
                if !derived_keys.contains(&key) && !held.contains(&value) {
                    locks.entry(key).or_insert(value);
                }
            }
            projected.assignments = assignments;
            projected.locks = locks;
            projected.key_bindings = local_bindings;
            *target = projected;
        }
    }

    for correspondence in halves
        .correspondences
        .iter()
        .filter(|correspondence| correspondence.kind == MirroredItemKind::Layout)
    {
        if halves.overlay_of(correspondence).is_some_and(|overlay| {
            matches!(
                overlay.state,
                crate::model::MirroredExceptionState::Local
                    | crate::model::MirroredExceptionState::Omitted
            )
        }) {
            continue;
        }
        let mut sources = document
            .layouts
            .iter()
            .filter(|layout| layout.id == correspondence.source_id);
        let source = sources.next().ok_or_else(|| {
            format!(
                "The source layout {:?} is unavailable",
                correspondence.source_id
            )
        })?;
        if sources.next().is_some() {
            return Err(format!(
                "The source layout {:?} is ambiguous",
                correspondence.source_id
            ));
        }
        let mut mirrors = document
            .layouts
            .iter()
            .filter(|layout| layout.id == correspondence.mirror_id);
        let mirror = mirrors.next().ok_or_else(|| {
            format!(
                "The mirror layout {:?} is unavailable",
                correspondence.mirror_id
            )
        })?;
        if mirrors.next().is_some() {
            return Err(format!(
                "The mirror layout {:?} is ambiguous",
                correspondence.mirror_id
            ));
        }
        let (source_is_current, mirror_is_current) = projection
            .matrix
            .as_ref()
            .map(|matrix| {
                (
                    source.matrix_id == matrix.source_matrix_id,
                    mirror.matrix_id == matrix.mirror_matrix_id,
                )
            })
            .unwrap_or((false, false));
        if source_is_current != mirror_is_current {
            return Err(
                "The paired layouts do not follow the reconciled matrix correspondence".into(),
            );
        }
        if !source_is_current {
            continue;
        }
        if source.board_id != halves.source_board_id || mirror.board_id != halves.mirror_board_id {
            return Err("A paired layout has stale board or matrix ownership".into());
        }

        let source_members: BTreeSet<_> = source.part_ids.iter().cloned().collect();
        let mirror_members: BTreeSet<_> = mirror.part_ids.iter().cloned().collect();
        if source_members.len() != source.part_ids.len()
            || mirror_members.len() != mirror.part_ids.len()
        {
            return Err("A paired layout contains duplicate part membership".into());
        }
        let mut projected_members =
            Vec::with_capacity(source.part_ids.len() + mirror.part_ids.len());
        let mut projected_member_ids = BTreeSet::new();
        for source_id in &source.part_ids {
            if halves
                .overlay_for_source(MirroredItemKind::Part, source_id)
                .is_some_and(|e| e.state == crate::model::MirroredExceptionState::Omitted)
            {
                continue;
            }
            let mirror_id = independent_part_targets.get(source_id).ok_or_else(|| {
                format!(
                    "The source layout part {source_id:?} has no independent part correspondence"
                )
            })?;
            if !projected_member_ids.insert(mirror_id.clone()) {
                return Err("A paired layout maps multiple source parts to one mirror part".into());
            }
            projected_members.push(mirror_id.clone());
        }
        projected_members.extend(
            mirror
                .part_ids
                .iter()
                .filter(|id| {
                    !projected_member_ids.contains(*id)
                        && !removed_independent_targets.contains(*id)
                        && halves
                            .correspondences
                            .iter()
                            .find(|item| {
                                item.kind == MirroredItemKind::Part && item.mirror_id == ***id
                            })
                            .is_none_or(|item| {
                                halves.overlay_of(item).is_some_and(|exception| {
                                    exception.state == crate::model::MirroredExceptionState::Local
                                })
                            })
                })
                .cloned(),
        );
        let staged_mirror = staged
            .layouts
            .iter_mut()
            .find(|layout| layout.id == correspondence.mirror_id)
            .ok_or_else(|| {
                format!(
                    "The staged mirror layout {:?} is unavailable",
                    correspondence.mirror_id
                )
            })?;
        staged_mirror.part_ids = projected_members;
    }

    let mut module_targets: BTreeMap<_, _> = halves
        .correspondences
        .iter()
        .filter(|association| association.kind == MirroredItemKind::Module)
        .map(|association| (association.source_id.clone(), association.mirror_id.clone()))
        .collect();
    for transition in &projection.hosted_transitions {
        if let HostedTransition::AddModule {
            source_id,
            mirror_id,
        } = transition
        {
            match module_targets.get(source_id) {
                Some(existing)
                    if existing == mirror_id
                        && projection.applied_hosted_operations.borrow().contains(&(
                            "module".to_owned(),
                            source_id.clone(),
                            mirror_id.clone(),
                        )) => {}
                Some(_) => {
                    return Err(format!(
                        "The module addition {source_id:?} already has a correspondence"
                    ));
                }
                None => {
                    module_targets.insert(source_id.clone(), mirror_id.clone());
                }
            }
        }
    }
    let unique_module_targets: BTreeSet<_> = module_targets.values().collect();
    if unique_module_targets.len() != module_targets.len() {
        return Err("The module correspondences contain aliased mirror identities".into());
    }
    let instances = document
        .hardware
        .as_ref()
        .ok_or_else(|| "Physical instances are unavailable".to_owned())?;
    let unique_instance_for_board = |board_id: &str| -> Result<String, String> {
        let mut matches = instances
            .instances
            .iter()
            .filter(|instance| instance.board_id == board_id);
        let instance = matches
            .next()
            .ok_or_else(|| format!("Board {board_id:?} has no physical instance"))?;
        if matches.next().is_some() {
            return Err(format!(
                "Board {board_id:?} has ambiguous physical instances"
            ));
        }
        Ok(instance.id.clone())
    };
    let source_instance = unique_instance_for_board(&halves.source_board_id)?;
    let mirror_instance = unique_instance_for_board(&halves.mirror_board_id)?;
    let module_context = super::ModuleMirror {
        from: &halves.source_board_id,
        to: &halves.mirror_board_id,
        source_instance: &source_instance,
        mirror_instance: &mirror_instance,
        mapping: pair.mapping,
        parts: &reference_part_targets,
        omitted_parts: &omitted_part_sources,
        modules: &module_targets,
    };
    for (source_id, mirror_id) in module_targets.iter().filter(|(source_id, _)| {
        !projection.hosted_transitions.iter().any(|transition| {
            matches!(transition,
            HostedTransition::AddModule { source_id: added, .. } if added == *source_id)
        })
    }) {
        let exception = halves.exceptions.iter().find(|exception| {
            exception.kind == MirroredItemKind::Module && exception.mirror_id == *mirror_id
        });
        if exception.is_some() {
            continue;
        }
        let source = document
            .modules
            .iter()
            .find(|module| module.id == *source_id);
        let target_exists = staged.modules.iter().any(|module| module.id == *mirror_id);
        match source {
            Some(source) if target_exists => {
                let projected = super::mirror_module(source, &module_context)?;
                let target = staged
                    .modules
                    .iter_mut()
                    .find(|module| module.id == *mirror_id)
                    .unwrap();
                *target = projected;
            }
            None if target_exists
                && projection.hosted_transitions.iter().any(|transition| {
                    matches!(transition,
                HostedTransition::RemoveModule { source_id: removed, mirror_id: removed_mirror }
                    if removed == source_id && removed_mirror == mirror_id)
                }) =>
            {
                staged.modules.retain(|module| module.id != *mirror_id);
            }
            _ => {}
        }
    }

    for transition in &projection.hosted_transitions {
        if let HostedTransition::AddModule {
            source_id,
            mirror_id,
        } = transition
        {
            let source = document
                .modules
                .iter()
                .find(|module| module.id == *source_id)
                .ok_or_else(|| format!("The source module {source_id:?} is unavailable"))?;
            let has_provenance = projection.applied_hosted_operations.borrow().contains(&(
                "module".to_owned(),
                source_id.clone(),
                mirror_id.clone(),
            ));
            let target = staged
                .modules
                .iter_mut()
                .find(|module| module.id == *mirror_id);
            match (target, has_provenance) {
                (None, false) => staged
                    .modules
                    .push(super::mirror_module(source, &module_context)?),
                (Some(target), true) => *target = super::mirror_module(source, &module_context)?,
                (Some(_), false) => {
                    return Err(format!(
                        "The selected target module {mirror_id:?} already exists without private Add provenance"
                    ));
                }
                (None, true) => {
                    return Err(format!(
                        "The previously added target module {mirror_id:?} is unavailable"
                    ));
                }
            }
            planned_hosted_operations.insert((
                "module".to_owned(),
                source_id.clone(),
                mirror_id.clone(),
            ));
        } else if let HostedTransition::RemoveModule {
            source_id,
            mirror_id,
        } = transition
        {
            let operation = (
                "remove-module".to_owned(),
                source_id.clone(),
                mirror_id.clone(),
            );
            if document
                .modules
                .iter()
                .any(|module| module.id == *mirror_id)
            {
                staged.modules.retain(|module| module.id != *mirror_id);
                planned_hosted_operations.insert(operation);
            } else if projection
                .applied_hosted_operations
                .borrow()
                .contains(&operation)
            {
                planned_hosted_operations.insert(operation);
            } else {
                return Err(format!(
                    "The target module {mirror_id:?} is already absent without private removal provenance"
                ));
            }
        } else if let HostedTransition::RemoveCircuit {
            source_id,
            mirror_id,
        } = transition
        {
            let operation = (
                "remove-circuit".to_owned(),
                source_id.clone(),
                mirror_id.clone(),
            );
            if document
                .embedded_circuits
                .iter()
                .any(|circuit| circuit.id == *mirror_id)
            {
                if staged
                    .embedded_circuits
                    .iter()
                    .any(|circuit| circuit.id == *mirror_id)
                {
                    crate::modules::remove_circuit(&mut staged, mirror_id)?;
                }
                planned_hosted_operations.insert(operation);
            } else if projection
                .applied_hosted_operations
                .borrow()
                .contains(&operation)
            {
                planned_hosted_operations.insert(operation);
            } else {
                return Err(format!(
                    "The target circuit {mirror_id:?} is already absent without private removal provenance"
                ));
            }
        }
    }

    if let Some(matrix_link) = projection.matrix.as_ref() {
        if let Some(final_matrix) = staged
            .matrices
            .iter()
            .find(|matrix| matrix.id == matrix_link.mirror_matrix_id)
            .cloned()
        {
            crate::matrix::valid_matrix(&final_matrix, &staged)?;
            for id in &final_matrix.part_ids {
                if omitted_parts.contains(id) {
                    continue;
                }
                if unique_board_owner(&staged, id, "mirrored")? != halves.mirror_board_id {
                    return Err(format!(
                        "Projected mirror matrix part {id:?} has an unexpected owner"
                    ));
                }
            }
        } else if !allow_absent_matrix {
            return Err("The final mirror matrix is unavailable".into());
        }
    } else if !allow_absent_matrix {
        return Err("The final mirror matrix is unavailable".into());
    }
    settings::project(document, &mut staged, halves)?;
    outlines::reconcile(&mut staged, pair_id, projection)?;

    for transition in &projection.hosted_transitions {
        match transition {
            HostedTransition::AddModule {
                source_id,
                mirror_id,
            } => stage_hosted_correspondence(
                &mut staged,
                pair_id,
                projection.applied_hosted_operations.borrow().contains(&(
                    "module".to_owned(),
                    source_id.clone(),
                    mirror_id.clone(),
                )),
                MirroredCorrespondence {
                    companion_mode: None,
                    companion_tweak: None,
                    kind: MirroredItemKind::Module,
                    source_id: source_id.clone(),
                    mirror_id: mirror_id.clone(),
                },
            )?,
            HostedTransition::AddCircuit {
                source_id,
                mirror_id,
                member_ids,
                net_ids,
                ..
            } => {
                let replay_key = ("circuit".to_owned(), source_id.clone(), mirror_id.clone());
                let has_replay_provenance = projection
                    .applied_hosted_circuit_additions
                    .borrow()
                    .contains_key(&replay_key);
                stage_hosted_correspondence(
                    &mut staged,
                    pair_id,
                    has_replay_provenance,
                    MirroredCorrespondence {
                        companion_mode: None,
                        companion_tweak: None,
                        kind: MirroredItemKind::EmbeddedCircuit,
                        source_id: source_id.clone(),
                        mirror_id: mirror_id.clone(),
                    },
                )?;
                for (source_member_id, mirror_member_id) in member_ids {
                    stage_hosted_correspondence(
                        &mut staged,
                        pair_id,
                        has_replay_provenance,
                        MirroredCorrespondence {
                            companion_mode: None,
                            companion_tweak: None,
                            kind: MirroredItemKind::Part,
                            source_id: source_member_id.clone(),
                            mirror_id: mirror_member_id.clone(),
                        },
                    )?;
                }
                for (source_net_id, mirror_net_id) in net_ids {
                    let is_joined_host_net = halves.correspondences.iter().any(|association| {
                        association.kind == MirroredItemKind::Net
                            && association.source_id == *source_net_id
                            && association.mirror_id == *mirror_net_id
                    });
                    if !is_joined_host_net {
                        stage_hosted_correspondence(
                            &mut staged,
                            pair_id,
                            has_replay_provenance,
                            MirroredCorrespondence {
                                companion_mode: None,
                                companion_tweak: None,
                                kind: MirroredItemKind::Net,
                                source_id: source_net_id.clone(),
                                mirror_id: mirror_net_id.clone(),
                            },
                        )?;
                    }
                }
            }
            _ => {}
        }
    }

    let mut validated_draft = staged.clone();
    let pair = validated_draft
        .hardware
        .as_mut()
        .and_then(|hardware| hardware.pairs.iter_mut().find(|pair| pair.id == pair_id))
        .ok_or_else(|| format!("The validated pair {pair_id:?} is unavailable"))?;
    let validated_halves = pair
        .halves
        .as_mut()
        .ok_or_else(|| format!("The validated pair {pair_id:?} has no halves context"))?;
    for (operation, source_id, mirror_id) in &planned_hosted_operations {
        match operation.as_str() {
            "remove-module" => {
                validated_halves.correspondences.retain(|association| {
                    !(association.kind == MirroredItemKind::Module
                        && association.source_id == *source_id
                        && association.mirror_id == *mirror_id)
                });
                validated_halves.exceptions.retain(|exception| {
                    !(exception.kind == MirroredItemKind::Module
                        && exception.mirror_id == *mirror_id
                        && !staged
                            .modules
                            .iter()
                            .any(|module| module.id == exception.mirror_id))
                });
            }
            "remove-circuit" => {
                let source_prefix = format!("embedded/{source_id}/");
                let mirror_prefix = format!("embedded/{mirror_id}/");
                validated_halves.correspondences.retain(|association| {
                    let circuit = association.kind == MirroredItemKind::EmbeddedCircuit
                        && association.source_id == *source_id
                        && association.mirror_id == *mirror_id;
                    let owned_part = association.kind == MirroredItemKind::Part
                        && association.source_id.starts_with(&source_prefix)
                        && association.mirror_id.starts_with(&mirror_prefix);
                    let owned_net = association.kind == MirroredItemKind::Net
                        && association.source_id.starts_with(&source_prefix)
                        && association.mirror_id.starts_with(&mirror_prefix);
                    !(circuit || owned_part || owned_net)
                });
                validated_halves.exceptions.retain(|exception| {
                    if !exception.mirror_id.starts_with(&mirror_prefix) {
                        return true;
                    }
                    match exception.kind {
                        MirroredItemKind::EmbeddedCircuit => staged
                            .embedded_circuits
                            .iter()
                            .any(|item| item.id == exception.mirror_id),
                        MirroredItemKind::Part => staged
                            .parts
                            .iter()
                            .any(|item| item.id == exception.mirror_id),
                        MirroredItemKind::Net => staged
                            .nets
                            .iter()
                            .any(|item| item.id == exception.mirror_id),
                        _ => true,
                    }
                });
            }
            _ => {}
        }
    }
    validated_halves.exceptions.retain(|exception| {
        !((exception.kind == MirroredItemKind::Part
            && hosted_omission_closure.parts.contains(&exception.mirror_id))
            || (exception.kind == MirroredItemKind::Net
                && hosted_omission_closure.nets.contains(&exception.mirror_id)))
    });
    outlines::prepare_validation(&mut validated_draft, pair_id, projection)?;
    crate::mechanical::features::validate(&validated_draft)?;
    crate::mechanical::halves::validate_document(&validated_draft)
        .map_err(|error| format!("The validated hosted draft is inconsistent: {error}"))?;

    let (changed_ids, _) = changed_document_fields(document, &staged)?;
    let first_output = staged.clone();

    *document = staged;
    {
        let mut applied_additions = projection.applied_independent_additions.borrow_mut();
        applied_additions.extend(planned_independent_additions.iter().map(
            |(source_id, (mirror_id, definition_id, reference))| {
                (
                    (*source_id).to_owned(),
                    (*mirror_id).to_owned(),
                    (*definition_id).to_owned(),
                    (*reference).to_owned(),
                )
            },
        ));
    }
    projection
        .applied_hosted_operations
        .borrow_mut()
        .extend(planned_hosted_operations);
    let mut circuit_additions = projection.applied_hosted_circuit_additions.borrow_mut();
    for transition in &projection.hosted_transitions {
        let HostedTransition::AddCircuit {
            source_id,
            mirror_id,
            member_ids,
            member_references,
            definition_ids,
            net_ids,
            member_poses,
        } = transition
        else {
            continue;
        };
        let key = ("circuit".to_owned(), source_id.clone(), mirror_id.clone());
        circuit_additions
            .entry(key)
            .or_insert_with(|| HostedCircuitAddProvenance {
                member_ids: member_ids.clone(),
                member_references: member_references.clone(),
                definition_ids: definition_ids.clone(),
                net_ids: net_ids.clone(),
                member_poses: member_poses.clone(),
                first_output: first_output.clone(),
            });
    }
    Ok(changed_ids)
}

/// Project one source matrix into its already assigned mirror identity.
///
/// Production dispatch stays dormant until the verified slices are activated together.
#[allow(dead_code)] // Ticket 111 owns activation after the reconciliation slices are verified.
fn project_matrix(
    source: &Matrix,
    mirror: &Matrix,
    correspondence: &MirroredCorrespondence,
    mapping: Pose2,
) -> Result<Matrix, String> {
    if correspondence.kind != crate::model::MirroredItemKind::Matrix
        || correspondence.source_id != source.id
        || correspondence.mirror_id != mirror.id
    {
        return Err("The matrix correspondence does not match its source and mirror".into());
    }

    crate::matrix::layout::reflected(source, mirror, mapping, false)
}

/// Project one source part through its established correspondence, retaining target identifiers.
#[allow(dead_code)] // Ticket 111 owns production activation after the reconciliation slices are verified.
fn project_part(
    source: &Part,
    mirror: &Part,
    correspondence: &MirroredCorrespondence,
    mapping: Pose2,
) -> Result<Part, String> {
    if correspondence.kind != crate::model::MirroredItemKind::Part
        || correspondence.source_id != source.id
        || correspondence.mirror_id != mirror.id
    {
        return Err("The part correspondence does not match its source and mirror".into());
    }

    Ok(super::derive::part(
        source,
        &mirror.id,
        &mirror.definition_id,
        &mirror.reference,
        mapping,
    ))
}

#[allow(dead_code)] // Ticket 106 prepares a private transition slice before lifecycle activation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TransitionState {
    Linked,
    Local,
    Omitted,
}

#[allow(dead_code)] // Ticket 106 prepares a private transition slice before lifecycle activation.
#[derive(Clone, Debug, PartialEq)]
struct TransitionAssociation {
    kind: MirroredItemKind,
    source_id: String,
    mirror_id: String,
    parent_source_id: Option<String>,
    parent_mirror_id: Option<String>,
    state: TransitionState,
    explicit_state: bool,
    raw_mode_override: Option<crate::model::MirroredCompanionMode>,
    raw_tweak: Option<crate::model::MirroredCompanionTweak>,
    effective_mode: crate::model::MirroredCompanionMode,
    effective_tweak: Option<crate::model::MirroredCompanionTweak>,
}

#[allow(dead_code)] // Ticket 106 prepares a private transition slice before lifecycle activation.
#[derive(Clone, Debug, PartialEq, Eq)]
enum FinalOwner {
    Board(String),
    Absent,
}

#[allow(dead_code)] // Ticket 106 prepares a private transition slice before lifecycle activation.
#[derive(Clone, Debug, PartialEq)]
struct MatrixPairContext {
    source_id: String,
    mirror_id: String,
}

#[allow(dead_code)] // Ticket 106 prepares a private transition slice before lifecycle activation.
#[derive(Clone, Debug, PartialEq)]
struct MembershipContext {
    pair_id: String,
    source_board_id: String,
    mirror_board_id: String,
    matrix_pair: Option<MatrixPairContext>,
    mapping: Pose2,
    pair_right: crate::model::MirroredHalves,
    associations: Vec<TransitionAssociation>,
    owners: std::collections::BTreeMap<String, FinalOwner>,
    pending_rejoins: BTreeMap<String, TransitionState>,
    pending_pair_transitions: BTreeMap<String, TransitionState>,
    pending_pair_baseline: Option<crate::model::MirroredHalves>,
}

fn require_matrix_pair(context: &MembershipContext) -> Result<&MatrixPairContext, String> {
    context
        .matrix_pair
        .as_ref()
        .ok_or_else(|| "Matrix membership work requires an exact Matrix correspondence".to_owned())
}

#[allow(dead_code)] // Ticket 106 prepares a private transition slice before lifecycle activation.
#[derive(Clone, Debug, PartialEq)]
struct PlannedAddition {
    source_id: String,
    mirror_id: String,
    source_reference: String,
    mirror_reference: String,
    parent_source_id: Option<String>,
    parent_mirror_id: Option<String>,
}

#[allow(dead_code)] // Ticket 106 prepares a private transition slice before lifecycle activation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum BaselineKind {
    MaterializedCurrentSchema,
    ChainedPrivateDraft,
}

#[derive(Clone, Copy)]
#[allow(dead_code)] // Ticket 106 prepares a private transition slice before lifecycle activation.
struct MembershipTransitionInput<'a> {
    valid_before: &'a ProjectDoc,
    post_source: &'a ProjectDoc,
    context: &'a MembershipContext,
    baseline_kind: BaselineKind,
    source_identity_map: &'a std::collections::BTreeMap<String, Option<String>>,
    mirror_identity_map: &'a std::collections::BTreeMap<String, Option<String>>,
    planned_additions: &'a [PlannedAddition],
    final_owners: &'a std::collections::BTreeMap<String, FinalOwner>,
    explicit_removal_closure: &'a std::collections::BTreeSet<String>,
}

#[allow(dead_code)] // Ticket 106 keeps canonical, validated input facts together before planning.
struct ValidatedTransitionContext<'a> {
    input: MembershipTransitionInput<'a>,
    removes_paired_matrices: bool,
    already_removed: bool,
    before_source_members: BTreeSet<String>,
    before_mirror_members: BTreeSet<String>,
    post_source_members: BTreeSet<String>,
    mapped_source_ids: BTreeSet<String>,
    mapped_mirror_ids: BTreeSet<String>,
}

#[allow(dead_code)] // Ticket 106 prepares a private transition slice before lifecycle activation.
#[derive(Clone, Debug, Default, PartialEq)]
struct MembershipContextDelta {
    added: Vec<TransitionAssociation>,
    retired: Vec<TransitionAssociation>,
    relocated: Vec<(String, String)>,
    owners: std::collections::BTreeMap<String, FinalOwner>,
    /// Mirror copper saved with an Omit, by the omitted item's mirror id.
    removed_copper: std::collections::BTreeMap<String, crate::model::OmittedCopper>,
}

#[allow(dead_code)] // Ticket 106 prepares a private transition slice before lifecycle activation.
#[derive(Clone, Debug, PartialEq)]
struct MembershipTransitionOutput {
    draft: ProjectDoc,
    changed_document_ids: Vec<String>,
    changed_field_paths: Vec<String>,
    context: MembershipContext,
    context_delta: MembershipContextDelta,
    final_owners: std::collections::BTreeMap<String, FinalOwner>,
    /// What a transition could not do for the user, such as saved copper it could not restore.
    /// Production surfacing belongs to ticket 111.
    notices: Vec<String>,
}

#[allow(dead_code)] // Ticket 106 prepares a private transition slice before lifecycle activation.
fn prepare_membership_transition_kernel(
    input: MembershipTransitionInput<'_>,
) -> Result<MembershipTransitionOutput, String> {
    let matrix_pair = require_matrix_pair(input.context)?;

    fn pair_right<'a>(
        document: &'a ProjectDoc,
        pair_id: &str,
    ) -> Result<(&'a crate::model::MirroredHalves, Pose2), String> {
        let hardware = document
            .hardware
            .as_ref()
            .ok_or_else(|| "The transition hardware configuration is unavailable".to_owned())?;
        let mut pairs = hardware.pairs.iter().filter(|pair| pair.id == pair_id);
        let pair = pairs
            .next()
            .ok_or_else(|| format!("The transition pair {pair_id:?} is unavailable"))?;
        if pairs.next().is_some() {
            return Err(format!("The transition pair {pair_id:?} is ambiguous"));
        }
        let halves = pair
            .halves
            .as_ref()
            .ok_or_else(|| format!("The transition pair {pair_id:?} has no halves record"))?;
        Ok((halves, pair.mapping))
    }

    fn matrix_by_id<'a>(document: &'a ProjectDoc, id: &str) -> Result<&'a Matrix, String> {
        let mut matrices = document.matrices.iter().filter(|matrix| matrix.id == id);
        let matrix = matrices
            .next()
            .ok_or_else(|| format!("The transition matrix {id:?} is unavailable"))?;
        if matrices.next().is_some() {
            return Err(format!("The transition matrix {id:?} is ambiguous"));
        }
        Ok(matrix)
    }

    fn registration_count(document: &ProjectDoc, id: &str) -> usize {
        document
            .boards
            .iter()
            .map(|board| {
                board
                    .part_ids
                    .iter()
                    .filter(|part_id| part_id.as_str() == id)
                    .count()
            })
            .sum()
    }

    fn board_owner(document: &ProjectDoc, id: &str) -> Result<FinalOwner, String> {
        if registration_count(document, id) != 1 {
            return Err(format!(
                "The transition part {id:?} must have exactly one actual board registration"
            ));
        }
        let board = document
            .boards
            .iter()
            .find(|board| board.part_ids.iter().any(|part_id| part_id == id))
            .ok_or_else(|| format!("The transition part {id:?} has no board owner"))?;
        Ok(FinalOwner::Board(board.id.clone()))
    }

    fn validate_owner_realization(
        document: &ProjectDoc,
        owners: &BTreeMap<String, FinalOwner>,
    ) -> Result<(), String> {
        for (id, owner) in owners {
            match owner {
                FinalOwner::Board(_) => {
                    if document.parts.iter().filter(|part| part.id == *id).count() != 1
                        || board_owner(document, id)? != *owner
                    {
                        return Err(format!(
                            "Board-owned identity {id:?} does not have exactly one matching Part registration"
                        ));
                    }
                }
                FinalOwner::Absent => {
                    if document.parts.iter().any(|part| part.id == *id)
                        || document
                            .boards
                            .iter()
                            .any(|board| board.part_ids.iter().any(|part_id| part_id == id))
                    {
                        return Err(format!(
                            "Absent identity {id:?} still has a Part or board registration"
                        ));
                    }
                }
            }
        }
        Ok(())
    }

    fn validate_context_owners(
        document: &ProjectDoc,
        context: &MembershipContext,
    ) -> Result<(), String> {
        let mut part_ids = BTreeSet::new();
        for part in &document.parts {
            if !part_ids.insert(part.id.as_str()) {
                return Err(format!(
                    "The historical Part identity {:?} is duplicated",
                    part.id
                ));
            }
            let actual = board_owner(document, &part.id)?;
            if context.owners.get(&part.id) != Some(&actual)
                && !(actual == FinalOwner::Board(context.mirror_board_id.clone())
                    && context.owners.get(&part.id) == Some(&FinalOwner::Absent)
                    && lifecycle::pending_visible_omission(context, &part.id))
            {
                return Err(format!(
                    "The historical owner context for {:?} does not match its registered board",
                    part.id
                ));
            }
        }
        for (id, owner) in &context.owners {
            match owner {
                FinalOwner::Board(_) => {
                    let pending_omitted_rejoin = context.associations.iter().any(|association| {
                        association.kind == MirroredItemKind::Part
                            && association.mirror_id == *id
                            && association.state == TransitionState::Linked
                            && context.pending_rejoins.get(&association.source_id)
                                == Some(&TransitionState::Omitted)
                    });
                    if pending_omitted_rejoin
                        && document.parts.iter().all(|part| part.id != *id)
                        && registration_count(document, id) == 0
                    {
                        continue;
                    }
                    if board_owner(document, id)? != *owner
                        || document.parts.iter().filter(|part| part.id == *id).count() != 1
                    {
                        return Err(format!(
                            "The historical owner context for {id:?} is not realized exactly once"
                        ));
                    }
                }
                FinalOwner::Absent => {
                    if (document.parts.iter().any(|part| part.id == *id)
                        || document
                            .boards
                            .iter()
                            .any(|board| board.part_ids.iter().any(|part_id| part_id == id)))
                        && !lifecycle::pending_visible_omission(context, id)
                    {
                        return Err(format!(
                            "The historically absent identity {id:?} still has a Part registration"
                        ));
                    }
                }
            }
        }
        Ok(())
    }

    fn validate_constraints(document: &ProjectDoc) -> Result<(), String> {
        for constraint in &document.constraints {
            for endpoint in [constraint.source(), constraint.target()] {
                if document
                    .parts
                    .iter()
                    .filter(|part| part.id == endpoint)
                    .count()
                    != 1
                {
                    return Err(format!(
                        "Constraint {:?} must reference exactly one actual Part at {:?}",
                        constraint.id(),
                        endpoint
                    ));
                }
            }
            if !constraint.is_finite() {
                return Err(format!(
                    "Constraint {:?} contains non-finite geometry",
                    constraint.id()
                ));
            }
        }
        Ok(())
    }

    fn validate_transition_part_geometry(
        document: &ProjectDoc,
        members: &BTreeSet<String>,
        context: &MembershipContext,
    ) -> Result<(), String> {
        for id in members {
            let mut parts = document.parts.iter().filter(|part| part.id == *id);
            let Some(part) = parts.next() else {
                let pending_omitted_rejoin = context.associations.iter().any(|association| {
                    association.kind == MirroredItemKind::Part
                        && association.mirror_id == *id
                        && association.state == TransitionState::Linked
                        && context.pending_rejoins.get(&association.source_id)
                            == Some(&TransitionState::Omitted)
                        && context.owners.get(id)
                            == Some(&FinalOwner::Board(context.mirror_board_id.clone()))
                        && context.pair_right.correspondences.iter().any(|record| {
                            record.kind == MirroredItemKind::Part
                                && record.source_id == association.source_id
                                && record.mirror_id == association.mirror_id
                                && record.companion_mode == association.raw_mode_override
                                && record.companion_tweak == association.raw_tweak
                        })
                });
                if pending_omitted_rejoin {
                    continue;
                }
                let mut omitted = context.associations.iter().filter(|association| {
                    association.kind == MirroredItemKind::Part
                        && association.mirror_id == *id
                        && association.state == TransitionState::Omitted
                        && context.owners.get(id) == Some(&FinalOwner::Absent)
                });
                let Some(association) = omitted.next() else {
                    return Err(format!(
                        "Transition matrix member {:?} has no actual Part",
                        id
                    ));
                };
                if omitted.next().is_some() {
                    return Err(format!(
                        "Transition matrix member {:?} has ambiguous Omitted ownership",
                        id
                    ));
                }
                let explicit_owner = if association.explicit_state {
                    Some(association)
                } else {
                    association
                        .parent_source_id
                        .as_deref()
                        .and_then(|parent_source| {
                            let parent_mirror = association.parent_mirror_id.as_deref()?;
                            context.associations.iter().find(|parent| {
                                parent.kind == MirroredItemKind::Part
                                    && parent.source_id == parent_source
                                    && parent.mirror_id == parent_mirror
                                    && parent.state == TransitionState::Omitted
                                    && parent.explicit_state
                            })
                        })
                };
                let Some(owner) = explicit_owner else {
                    return Err(format!(
                        "Absent transition matrix member {:?} lacks an explicit Omitted owner",
                        id
                    ));
                };
                let owner_is_durable = context.pair_right.exceptions.iter().any(|exception| {
                    exception.kind == MirroredItemKind::Part
                        && context.pair_right.overlay_source(exception)
                            == Some(owner.source_id.as_str())
                        && exception.mirror_id == owner.mirror_id
                        && exception.state == crate::model::MirroredExceptionState::Omitted
                });
                let child_correspondence_is_retained = association.source_id == owner.source_id
                    || context.pair_right.correspondences.iter().any(|record| {
                        record.kind == MirroredItemKind::Part
                            && record.source_id == association.source_id
                            && record.mirror_id == association.mirror_id
                            && record.companion_mode == association.raw_mode_override
                            && record.companion_tweak == association.raw_tweak
                    });
                if !owner_is_durable || !child_correspondence_is_retained {
                    return Err(format!(
                        "Absent transition matrix member {:?} lacks exact durable Omitted-owner provenance",
                        id
                    ));
                }
                continue;
            };
            if parts.next().is_some()
                || !part.pose.at.x.is_finite()
                || !part.pose.at.y.is_finite()
                || !part.pose.rotation.is_finite()
            {
                return Err(format!(
                    "Transition matrix member {:?} has ambiguous or non-finite geometry",
                    id
                ));
            }
        }
        Ok(())
    }

    fn validate_net_pad_ids(document: &ProjectDoc) -> Result<(), String> {
        for net in &document.nets {
            for pin in &net.pins {
                let mut parts = document.parts.iter().filter(|part| part.id == pin.part_id);
                let part = parts.next().ok_or_else(|| {
                    format!("Net {:?} references missing Part {:?}", net.id, pin.part_id)
                })?;
                if parts.next().is_some() {
                    return Err(format!(
                        "Net {:?} references ambiguous Part {:?}",
                        net.id, pin.part_id
                    ));
                }
                let mut definitions = document
                    .definitions
                    .iter()
                    .filter(|definition| definition.id == part.definition_id);
                let definition = definitions.next().ok_or_else(|| {
                    format!(
                        "Net {:?} references Part {:?} with no definition",
                        net.id, part.id
                    )
                })?;
                if definitions.next().is_some()
                    || !definition.pads.iter().any(|pad| pad.id == pin.pad_id)
                {
                    return Err(format!(
                        "Net {:?} pin {:?} does not reference an actual Pad.id",
                        net.id, pin.pad_id
                    ));
                }
            }
        }
        Ok(())
    }

    fn validate_no_native_legacy_overlap(
        document: &ProjectDoc,
        context: &MembershipContext,
    ) -> Result<(), String> {
        let matrix_pair = require_matrix_pair(context)?;

        for linked_layout in document
            .layouts
            .iter()
            .filter(|layout| layout.mirror_link.is_some())
        {
            let link = linked_layout.mirror_link.as_ref().unwrap();
            let source_layout = document
                .layouts
                .iter()
                .find(|layout| layout.id == link.source_id)
                .ok_or_else(|| "A legacy mirrored layout has no source layout".to_owned())?;
            if [
                linked_layout.matrix_id.as_str(),
                source_layout.matrix_id.as_str(),
            ]
            .into_iter()
            .any(|id| id == matrix_pair.source_id.as_str() || id == matrix_pair.mirror_id.as_str())
            {
                return Err("A matrix cannot use native and legacy mirror links together".into());
            }
        }
        Ok(())
    }

    fn validate_matrix_association(
        context: &MembershipContext,
        expected_present: bool,
    ) -> Result<(), String> {
        let matrix_pair = require_matrix_pair(context)?;

        let mut associations = context.associations.iter().filter(|association| {
            association.kind == MirroredItemKind::Matrix
                && association.source_id == matrix_pair.source_id
                && association.mirror_id == matrix_pair.mirror_id
        });
        let association = associations.next();
        if associations.next().is_some() {
            return Err("The selected transition matrix association is ambiguous".into());
        }
        let mut correspondences = context.pair_right.correspondences.iter().filter(|record| {
            record.kind == MirroredItemKind::Matrix
                && record.source_id == matrix_pair.source_id
                && record.mirror_id == matrix_pair.mirror_id
        });
        let correspondence = correspondences.next();
        if correspondences.next().is_some() {
            return Err("The selected transition matrix association is ambiguous".into());
        }
        if !expected_present {
            if association.is_none()
                && correspondence.is_none()
                && !context
                    .pair_right
                    .exceptions
                    .iter()
                    .any(|e| e.kind == MirroredItemKind::Matrix)
            {
                return Ok(());
            }
            let correspondence = correspondence.ok_or_else(|| {
                "The persisted pair matrix correspondence is unavailable".to_owned()
            })?;
            if association.is_some()
                || correspondence.source_id != matrix_pair.source_id.as_str()
                || correspondence.mirror_id != matrix_pair.mirror_id.as_str()
            {
                return Err(
                    "The retired matrix context disagrees with persisted provenance".into(),
                );
            }
            return Ok(());
        }
        let association = association
            .ok_or_else(|| "The transition matrix association is unavailable".to_owned())?;
        let correspondence = correspondence
            .ok_or_else(|| "The pair matrix correspondence is unavailable".to_owned())?;
        if association.source_id != matrix_pair.source_id.as_str()
            || association.mirror_id != matrix_pair.mirror_id.as_str()
            || association.state != TransitionState::Linked
            || association.explicit_state
            || association.parent_source_id.is_some()
            || association.parent_mirror_id.is_some()
            || correspondence.source_id != association.source_id
            || correspondence.mirror_id != association.mirror_id
            || correspondence.companion_mode != association.raw_mode_override
            || correspondence.companion_tweak != association.raw_tweak
        {
            return Err(
                "The transition matrix association disagrees with its actual pair correspondence"
                    .into(),
            );
        }
        Ok(())
    }

    fn source_members(document: &ProjectDoc, matrix_id: &str) -> Result<BTreeSet<String>, String> {
        let matrix = matrix_by_id(document, matrix_id)?;
        crate::matrix::valid_matrix(matrix, document)?;
        let mut members = BTreeSet::new();
        for cell in &matrix.cells {
            if cell.deleted || !cell.enabled {
                continue;
            }
            let key_id = format!("matrix/{}/r{}c{}", matrix.id, cell.row, cell.column);
            members.insert(key_id.clone());
            for assembly in &cell.assemblies {
                members.insert(crate::matrix::companion_id(&key_id, &assembly.id));
            }
        }
        if members != matrix.part_ids.iter().cloned().collect() {
            return Err("The source matrix membership is not canonical".into());
        }
        Ok(members)
    }

    fn finite_tweak(tweak: Option<crate::model::MirroredCompanionTweak>) -> bool {
        tweak.is_none_or(|value| {
            value.x.is_finite() && value.y.is_finite() && value.rotation.is_finite()
        })
    }

    fn validate_association_settings(context: &MembershipContext) -> Result<(), String> {
        for association in &context.associations {
            let effective_mode = association
                .raw_mode_override
                .unwrap_or(context.pair_right.companion_mode);
            let effective_tweak = association.raw_tweak;
            if association.source_id.is_empty()
                || association.mirror_id.is_empty()
                || !finite_tweak(association.raw_tweak)
                || !finite_tweak(association.effective_tweak)
                || association.effective_mode != effective_mode
                || association.effective_tweak != effective_tweak
                || (association.effective_mode
                    != crate::model::MirroredCompanionMode::LiteralMirrorWithTweak
                    && association.effective_tweak.is_some())
            {
                return Err("The transition association context has invalid settings".into());
            }
        }
        Ok(())
    }

    fn cell_offset_delta(matrix: &Matrix, column: u32, world: Vec2) -> Result<Vec2, String> {
        let mut local_x = Vec2 { x: 1.0, y: 0.0 };
        let mut local_y = Vec2 { x: 0.0, y: 1.0 };
        let splay = matrix
            .column_splays
            .iter()
            .take(column as usize + 1)
            .sum::<f64>()
            .to_radians();
        let rotate = |point: Vec2, angle: f64| {
            let (sin, cos) = angle.sin_cos();
            Vec2 {
                x: point.x * cos - point.y * sin,
                y: point.x * sin + point.y * cos,
            }
        };
        local_x = rotate(local_x, splay);
        local_y = rotate(local_y, splay);
        let reflect = |point: Vec2| match matrix.mirror.unwrap_or(crate::model::Mirror::None) {
            crate::model::Mirror::X => Vec2 {
                x: -point.x,
                y: point.y,
            },
            crate::model::Mirror::Y => Vec2 {
                x: point.x,
                y: -point.y,
            },
            crate::model::Mirror::None => point,
        };
        local_x = rotate(
            reflect(local_x),
            matrix.rotation.unwrap_or(0.0).to_radians(),
        );
        local_y = rotate(
            reflect(local_y),
            matrix.rotation.unwrap_or(0.0).to_radians(),
        );
        let determinant = local_x.x * local_y.y - local_y.x * local_x.y;
        if !determinant.is_finite() || determinant.abs() < f64::EPSILON {
            return Err("A Local matrix cell has an unsupported offset basis".into());
        }
        let delta = Vec2 {
            x: (world.x * local_y.y - local_y.x * world.y) / determinant,
            y: (local_x.x * world.y - world.x * local_x.y) / determinant,
        };
        if !delta.x.is_finite() || !delta.y.is_finite() {
            return Err("A Local matrix cell offset would be non-finite".into());
        }
        Ok(delta)
    }

    fn emit_validated_transition(
        validated: &ValidatedTransitionContext<'_>,
        draft: ProjectDoc,
        next_context: MembershipContext,
        context_delta: MembershipContextDelta,
        expected_mirror_members: &BTreeSet<String>,
    ) -> Result<MembershipTransitionOutput, String> {
        let input = validated.input;
        let matrix_pair = require_matrix_pair(input.context)?;
        validate_matrix_association(input.context, !validated.already_removed)?;
        validate_matrix_association(&next_context, !validated.removes_paired_matrices)?;
        validate_association_settings(input.context)?;
        validate_association_settings(&next_context)?;
        if validated.removes_paired_matrices {
            if draft.matrices.iter().any(|matrix| {
                matrix.id == matrix_pair.source_id.as_str()
                    || matrix.id == matrix_pair.mirror_id.as_str()
            }) || !expected_mirror_members.is_empty()
            {
                return Err("The emitted whole-matrix removal is incomplete".into());
            }
        } else {
            let source = matrix_by_id(&draft, matrix_pair.source_id.as_str())?;
            crate::matrix::valid_matrix(source, &draft)?;
            if source_members(&draft, matrix_pair.source_id.as_str())?
                != validated.post_source_members
            {
                return Err(
                    "The emitted source matrix disagrees with its validated operation map".into(),
                );
            }
            let mirror = matrix_by_id(&draft, matrix_pair.mirror_id.as_str())?;
            crate::matrix::valid_matrix(mirror, &draft)?;
            let actual_mirror_members: BTreeSet<_> = mirror.part_ids.iter().cloned().collect();
            if actual_mirror_members.len() != mirror.part_ids.len()
                || actual_mirror_members != *expected_mirror_members
            {
                return Err(
                    "The emitted mirror membership disagrees with its validated plan".into(),
                );
            }
        }
        if next_context.owners != *input.final_owners || context_delta.owners != *input.final_owners
        {
            return Err(
                "The emitted context and owner result disagree with the validated final owners"
                    .into(),
            );
        }
        validate_context_owners(&draft, &next_context)?;
        validate_owner_realization(&draft, input.final_owners)?;
        let association_keys: BTreeSet<_> = next_context
            .associations
            .iter()
            .map(|association| format!("{:?}\0{}", association.kind, association.source_id))
            .collect();
        let association_mirrors: BTreeSet<_> = next_context
            .associations
            .iter()
            .map(|association| format!("{:?}\0{}", association.kind, association.mirror_id))
            .collect();
        if association_keys.len() != next_context.associations.len()
            || association_mirrors.len() != next_context.associations.len()
        {
            return Err("The emitted transition associations contain aliased identities".into());
        }
        for association in next_context
            .associations
            .iter()
            .filter(|association| association.kind == MirroredItemKind::Part)
        {
            if association.state == TransitionState::Linked {
                let Some((row, column, child)) =
                    matrix_identity_slot(&association.source_id, matrix_pair.source_id.as_str())
                else {
                    return Err(format!(
                        "The emitted linked source identity {:?} is not canonical",
                        association.source_id
                    ));
                };
                let suffix = if child.is_empty() {
                    String::new()
                } else {
                    format!("/{child}")
                };
                let expected_mirror = format!(
                    "matrix/{}/r{}c{}{}",
                    matrix_pair.mirror_id.as_str(),
                    row,
                    column,
                    suffix
                );
                if !validated
                    .post_source_members
                    .contains(&association.source_id)
                    || association.mirror_id != expected_mirror
                    || !expected_mirror_members.contains(&association.mirror_id)
                {
                    return Err(format!(
                        "The emitted linked association {:?} disagrees with actual matrix membership",
                        association.source_id
                    ));
                }
            } else if association.state == TransitionState::Omitted
                && expected_mirror_members.contains(&association.mirror_id)
            {
                return Err(format!(
                    "The emitted Omitted identity {:?} remains in mirror membership",
                    association.mirror_id
                ));
            }
        }
        for association in next_context
            .associations
            .iter()
            .filter(|association| association.kind == MirroredItemKind::Part)
        {
            let (row, column, child) =
                matrix_identity_slot(&association.source_id, matrix_pair.source_id.as_str())
                    .ok_or_else(|| {
                        format!(
                            "The emitted Part association {:?} has no canonical source slot",
                            association.source_id
                        )
                    })?;
            let suffix = if child.is_empty() {
                String::new()
            } else {
                format!("/{child}")
            };
            let expected_mirror = format!(
                "matrix/{}/r{}c{}{}",
                matrix_pair.mirror_id.as_str(),
                row,
                column,
                suffix
            );
            let expected_parents = if child.is_empty() {
                (None, None)
            } else {
                (
                    Some(format!(
                        "matrix/{}/r{}c{}",
                        matrix_pair.source_id.as_str(),
                        row,
                        column
                    )),
                    Some(format!(
                        "matrix/{}/r{}c{}",
                        matrix_pair.mirror_id.as_str(),
                        row,
                        column
                    )),
                )
            };
            if (
                association.parent_source_id.clone(),
                association.parent_mirror_id.clone(),
            ) != expected_parents
                || (child.is_empty() && association.mirror_id != expected_mirror)
            {
                return Err(format!(
                    "The emitted Part association {:?} has inconsistent parent or mirror identity",
                    association.source_id
                ));
            }
            if let (Some(parent_source), Some(parent_mirror)) = expected_parents
                && !next_context.associations.iter().any(|parent| {
                    parent.kind == MirroredItemKind::Part
                        && parent.source_id == parent_source
                        && parent.mirror_id == parent_mirror
                })
            {
                return Err(format!(
                    "The emitted Part association {:?} lost its actual parent closure",
                    association.source_id
                ));
            }
            match association.state {
                TransitionState::Linked => {
                    if association.mirror_id != expected_mirror
                        || !validated
                            .post_source_members
                            .contains(&association.source_id)
                        || !expected_mirror_members.contains(&association.mirror_id)
                    {
                        return Err(format!(
                            "The emitted linked association {:?} disagrees with actual matrix membership",
                            association.source_id
                        ));
                    }
                }
                TransitionState::Local => {
                    if draft
                        .parts
                        .iter()
                        .filter(|part| part.id == association.mirror_id)
                        .count()
                        != 1
                        || board_owner(&draft, &association.mirror_id)?
                            != FinalOwner::Board(input.context.mirror_board_id.clone())
                    {
                        return Err(format!(
                            "The emitted Local identity {:?} lacks its unique mirror-board Part realization",
                            association.mirror_id
                        ));
                    }
                    if validated
                        .post_source_members
                        .contains(&association.source_id)
                        && expected_mirror_members.contains(&association.mirror_id)
                        && association.mirror_id != expected_mirror
                    {
                        return Err(format!(
                            "The emitted Local association {:?} disagrees with its canonical mirror identity",
                            association.source_id
                        ));
                    }
                }
                TransitionState::Omitted => {
                    if expected_mirror_members.contains(&association.mirror_id)
                        || draft
                            .parts
                            .iter()
                            .any(|part| part.id == association.mirror_id)
                        || registration_count(&draft, &association.mirror_id) != 0
                    {
                        return Err(format!(
                            "The emitted Omitted identity {:?} remains materialized",
                            association.mirror_id
                        ));
                    }
                }
            }
        }
        let (changed_document_ids, changed_field_paths) =
            changed_document_fields(input.post_source, &draft)?;
        Ok(MembershipTransitionOutput {
            draft,
            changed_document_ids,
            changed_field_paths,
            context: next_context,
            context_delta,
            final_owners: input.final_owners.clone(),
            notices: Vec::new(),
        })
    }

    fn validate_transition_input<'a>(
        input: MembershipTransitionInput<'a>,
    ) -> Result<ValidatedTransitionContext<'a>, String> {
        let matrix_pair = require_matrix_pair(input.context)?;

        fn durable_pair_before_pending_transitions(
            context: &MembershipContext,
            persisted: &crate::model::MirroredHalves,
        ) -> Result<crate::model::MirroredHalves, String> {
            match (
                &context.pending_pair_baseline,
                context.pending_pair_transitions.is_empty(),
            ) {
                (None, true) if context.pair_right == *persisted => Ok(persisted.clone()),
                (Some(baseline), false)
                    if baseline == persisted && context.pair_right == *persisted =>
                {
                    Ok(baseline.clone())
                }
                _ => Err("The private pending pair baseline is stale or incomplete".into()),
            }
        }

        let context = input.context;
        let matrices_already_removed = input.valid_before.matrices.iter().all(|matrix| {
            matrix.id != matrix_pair.source_id.as_str()
                && matrix.id != matrix_pair.mirror_id.as_str()
        });
        if matrices_already_removed {
            validate_matrix_association(context, false)?;
            validate_association_settings(context)?;
            if input.valid_before != input.post_source
                || !input.source_identity_map.is_empty()
                || !input.mirror_identity_map.is_empty()
                || !input.planned_additions.is_empty()
                || !input.explicit_removal_closure.is_empty()
                || input.final_owners != &context.owners
            {
                return Err(
                    "A retired matrix transition must be an unchanged, empty operation".into(),
                );
            }
            let (before_pair, before_mapping) = pair_right(input.valid_before, &context.pair_id)?;
            let (post_pair, post_mapping) = pair_right(input.post_source, &context.pair_id)?;
            if before_pair != &context.pair_right
                || post_pair != &context.pair_right
                || before_mapping != context.mapping
                || post_mapping != context.mapping
            {
                return Err(
                    "The retired matrix pair provenance changed outside its private context".into(),
                );
            }
            validate_context_owners(input.valid_before, context)?;
            validate_net_pad_ids(input.valid_before)?;
            validate_constraints(input.valid_before)?;
            validate_no_native_legacy_overlap(input.valid_before, context)?;
            return Ok(ValidatedTransitionContext {
                input,
                removes_paired_matrices: true,
                already_removed: true,
                before_source_members: BTreeSet::new(),
                before_mirror_members: BTreeSet::new(),
                post_source_members: BTreeSet::new(),
                mapped_source_ids: BTreeSet::new(),
                mapped_mirror_ids: BTreeSet::new(),
            });
        }
        validate_matrix_association(context, true)?;
        validate_association_settings(context)?;
        let (before_pair, before_mapping) = pair_right(input.valid_before, &context.pair_id)?;
        let (post_pair, post_mapping) = pair_right(input.post_source, &context.pair_id)?;
        let expected_durable_pair = durable_pair_before_pending_transitions(context, before_pair)?;
        if before_pair != post_pair
            || before_pair != &expected_durable_pair
            || post_pair != &expected_durable_pair
            || before_mapping != context.mapping
            || post_mapping != context.mapping
        {
            return Err("The transition pair record changed outside its private context".into());
        }
        validate_context_owners(input.valid_before, context)?;
        validate_net_pad_ids(input.valid_before)?;
        validate_net_pad_ids(input.post_source)?;
        validate_constraints(input.valid_before)?;
        validate_constraints(input.post_source)?;
        validate_no_native_legacy_overlap(input.valid_before, context)?;
        validate_no_native_legacy_overlap(input.post_source, context)?;
        let before_halves = before_pair;
        if before_halves.source_board_id != context.source_board_id
            || before_halves.mirror_board_id != context.mirror_board_id
        {
            return Err("The transition context does not match its pair boards".into());
        }
        if input.baseline_kind == BaselineKind::MaterializedCurrentSchema {
            crate::mechanical::halves::validate_document(input.valid_before)?;
        }

        let before_source = matrix_by_id(input.valid_before, matrix_pair.source_id.as_str())?;
        let before_mirror = matrix_by_id(input.valid_before, matrix_pair.mirror_id.as_str())?;
        let post_source = input
            .post_source
            .matrices
            .iter()
            .find(|matrix| matrix.id == matrix_pair.source_id.as_str());
        let post_mirror = matrix_by_id(input.post_source, matrix_pair.mirror_id.as_str())?;
        if before_source.board_id.as_deref() != Some(context.source_board_id.as_str())
            || post_source.is_some_and(|matrix| {
                matrix.board_id.as_deref() != Some(context.source_board_id.as_str())
            })
            || before_mirror.board_id.as_deref() != Some(context.mirror_board_id.as_str())
            || post_mirror.board_id.as_deref() != Some(context.mirror_board_id.as_str())
        {
            return Err("The transition matrices do not belong to the pair boards".into());
        }
        if input.baseline_kind == BaselineKind::MaterializedCurrentSchema {
            crate::matrix::valid_matrix(before_source, input.valid_before)?;
            crate::matrix::valid_matrix(before_mirror, input.valid_before)?;
        } else {
            source_members(input.valid_before, matrix_pair.source_id.as_str())?;
            source_members(input.valid_before, matrix_pair.mirror_id.as_str())?;
        }
        let before_source_members =
            source_members(input.valid_before, matrix_pair.source_id.as_str())?;
        let before_mirror_members =
            source_members(input.valid_before, matrix_pair.mirror_id.as_str())?;
        let post_source_members = if let Some(post_source) = post_source {
            crate::matrix::valid_matrix(post_source, input.post_source)?;
            source_members(input.post_source, matrix_pair.source_id.as_str())?
        } else {
            BTreeSet::new()
        };
        crate::matrix::valid_matrix(post_mirror, input.post_source)?;
        let before_transition_members: BTreeSet<_> = before_source_members
            .union(&before_mirror_members)
            .cloned()
            .collect();
        let post_mirror_members =
            source_members(input.post_source, matrix_pair.mirror_id.as_str())?;
        let post_transition_members: BTreeSet<_> = post_source_members
            .union(&post_mirror_members)
            .cloned()
            .collect();
        validate_transition_part_geometry(input.valid_before, &before_transition_members, context)?;
        validate_transition_part_geometry(input.post_source, &post_transition_members, context)?;
        if post_source.is_none() {
            let source_members_retired = before_source_members.iter().all(|id| {
                input.source_identity_map.get(id) == Some(&None)
                    && input.explicit_removal_closure.contains(id)
                    && input.final_owners.get(id) == Some(&FinalOwner::Absent)
            });
            let mirror_members_resolved =
                before_mirror_members
                    .iter()
                    .all(|id| match input.mirror_identity_map.get(id) {
                        Some(None) => {
                            input.explicit_removal_closure.contains(id)
                                && input.final_owners.get(id) == Some(&FinalOwner::Absent)
                        }
                        Some(Some(target_id)) => context.associations.iter().any(|association| {
                            association.kind == MirroredItemKind::Part
                                && association.state == TransitionState::Local
                                && association.mirror_id == *id
                                && association.mirror_id == *target_id
                                && input.source_identity_map.get(&association.source_id)
                                    == Some(&None)
                                && input.final_owners.get(target_id)
                                    == Some(&FinalOwner::Board(context.mirror_board_id.clone()))
                        }),
                        None => false,
                    });
            if input.source_identity_map.values().any(Option::is_some)
                || !source_members_retired
                || !mirror_members_resolved
            {
                return Err(
                    "Whole-matrix removal lacks complete source, mirror, and owner evidence".into(),
                );
            }
        }
        for association in context
            .associations
            .iter()
            .filter(|association| association.kind == MirroredItemKind::Part)
        {
            for (document, id) in [
                (input.valid_before, association.source_id.as_str()),
                (input.valid_before, association.mirror_id.as_str()),
                (input.post_source, association.source_id.as_str()),
                (input.post_source, association.mirror_id.as_str()),
            ] {
                let mut parts = document.parts.iter().filter(|part| part.id == id);
                if let Some(part) = parts.next()
                    && (parts.next().is_some()
                        || !part.pose.at.x.is_finite()
                        || !part.pose.at.y.is_finite()
                        || !part.pose.rotation.is_finite())
                {
                    return Err(format!(
                        "The associated Part {:?} has ambiguous or non-finite geometry",
                        id
                    ));
                }
            }
        }
        let mut expected_old_ids = before_source_members.clone();
        let mut mapped_source_ids = BTreeSet::new();
        for (old_id, new_id) in input.source_identity_map {
            if !expected_old_ids.remove(old_id) {
                return Err(format!(
                    "The source identity map has unknown old identity {old_id:?}"
                ));
            }
            if let Some(new_id) = new_id
                && !mapped_source_ids.insert(new_id.clone())
            {
                return Err("The source identity map is not injective".into());
            }
        }
        if !expected_old_ids.is_empty() {
            return Err("The source identity map does not cover every old matrix member".into());
        }
        let mut expected_old_mirror_ids = before_mirror_members.clone();
        let mut mapped_mirror_ids = BTreeSet::new();
        for (old_id, new_id) in input.mirror_identity_map {
            if !expected_old_mirror_ids.remove(old_id) {
                return Err(format!(
                    "The mirror identity map has unknown old identity {old_id:?}"
                ));
            }
            if let Some(new_id) = new_id
                && !mapped_mirror_ids.insert(new_id.clone())
            {
                return Err("The mirror identity map is not injective".into());
            }
        }
        if !expected_old_mirror_ids.is_empty() {
            return Err("The mirror identity map does not cover every old matrix member".into());
        }
        let part_associations: Vec<_> = context
            .associations
            .iter()
            .filter(|association| association.kind == MirroredItemKind::Part)
            .collect();
        let associated_sources: BTreeSet<_> = part_associations
            .iter()
            .map(|association| association.source_id.clone())
            .collect();
        for association in &part_associations {
            if !context.owners.contains_key(&association.source_id)
                || !context.owners.contains_key(&association.mirror_id)
            {
                return Err(format!(
                    "The historical owner context does not bind both sides of Part association {:?}",
                    association.source_id
                ));
            }
        }
        if associated_sources.len() != part_associations.len()
            || !before_source_members.is_subset(&associated_sources)
        {
            return Err("The historical Part associations omit canonical source membership".into());
        }
        for association in &part_associations {
            let (row, column, child) =
                matrix_identity_slot(&association.source_id, matrix_pair.source_id.as_str())
                    .ok_or_else(|| {
                        format!(
                            "The associated source identity {:?} is not canonical",
                            association.source_id
                        )
                    })?;
            let suffix = if child.is_empty() {
                String::new()
            } else {
                format!("/{child}")
            };
            let expected_mirror = format!(
                "matrix/{}/r{}c{}{}",
                matrix_pair.mirror_id.as_str(),
                row,
                column,
                suffix
            );
            let expected_parents = if child.is_empty() {
                (None, None)
            } else {
                (
                    Some(format!(
                        "matrix/{}/r{}c{}",
                        matrix_pair.source_id.as_str(),
                        row,
                        column
                    )),
                    Some(format!(
                        "matrix/{}/r{}c{}",
                        matrix_pair.mirror_id.as_str(),
                        row,
                        column
                    )),
                )
            };
            if association.mirror_id != expected_mirror
                || (
                    association.parent_source_id.clone(),
                    association.parent_mirror_id.clone(),
                ) != expected_parents
            {
                return Err(format!(
                    "The Part association {:?} disagrees with canonical membership or parent identity",
                    association.source_id
                ));
            }
            if input.baseline_kind == BaselineKind::MaterializedCurrentSchema {
                // The correspondence is the identity in every state; Local and Omitted are an
                // overlay on it. The persisted overlay must be the state the pair held before any
                // pending selection, so a pending Rejoin still sees the overlay it will clear.
                let pair_records: Vec<_> = context
                    .pair_right
                    .correspondences
                    .iter()
                    .filter(|record| {
                        record.kind == MirroredItemKind::Part
                            && (record.source_id == association.source_id
                                || record.mirror_id == association.mirror_id)
                    })
                    .collect();
                let overlays: Vec<_> = context
                    .pair_right
                    .exceptions
                    .iter()
                    .filter(|exception| {
                        exception.kind == MirroredItemKind::Part
                            && exception.mirror_id == association.mirror_id
                    })
                    .collect();
                let overlay_state = |state: TransitionState| match state {
                    TransitionState::Local => Some(crate::model::MirroredExceptionState::Local),
                    TransitionState::Omitted => Some(crate::model::MirroredExceptionState::Omitted),
                    TransitionState::Linked => None,
                };
                let pending_prior = context.pending_pair_transitions.get(&association.source_id);
                let expected_overlay = match pending_prior {
                    Some(prior) => overlay_state(*prior),
                    None if association.explicit_state => overlay_state(association.state),
                    None => None,
                };
                let exact_correspondence = pair_records.len() == 1
                    && pair_records[0].source_id == association.source_id
                    && pair_records[0].mirror_id == association.mirror_id
                    && pair_records[0].companion_mode == association.raw_mode_override
                    && pair_records[0].companion_tweak == association.raw_tweak;
                let overlay_matches = match overlays.as_slice() {
                    [] => {
                        expected_overlay.is_none()
                            // A kernel-only input may protect a part Local with no stored overlay.
                            || (pending_prior.is_none() && association.state != TransitionState::Linked)
                    }
                    [overlay] => {
                        Some(overlay.state) == expected_overlay
                            && overlay.companion_mode.is_none()
                            && overlay.companion_tweak.is_none()
                    }
                    _ => false,
                };
                if !exact_correspondence || !overlay_matches {
                    return Err(format!(
                        "The Part association {:?} disagrees with pair provenance",
                        association.source_id
                    ));
                }
            }
            if !before_source_members.contains(&association.source_id)
                && association.state != TransitionState::Local
            {
                return Err(format!(
                    "Only a retained Local association may remain outside current source membership: {:?}",
                    association.source_id
                ));
            }
            if association.state == TransitionState::Linked
                && !before_mirror_members.contains(&association.mirror_id)
                && context.pending_rejoins.get(&association.source_id)
                    != Some(&TransitionState::Omitted)
            {
                return Err(format!(
                    "The linked mirror identity {:?} is not an actual matrix member",
                    association.mirror_id
                ));
            }
            let actual_mirror_parts = input
                .valid_before
                .parts
                .iter()
                .filter(|part| part.id == association.mirror_id)
                .count();
            if association.state == TransitionState::Local && actual_mirror_parts != 1 {
                return Err(format!(
                    "The retained Local mirror identity {:?} has no unique actual Part",
                    association.mirror_id
                ));
            }
            if association.state == TransitionState::Omitted && actual_mirror_parts > 1 {
                return Err(format!(
                    "The materialized Omitted mirror identity {:?} is ambiguous",
                    association.mirror_id
                ));
            }
        }
        let pair_part_sources: BTreeSet<_> = context
            .pair_right
            .correspondences
            .iter()
            .filter(|record| record.kind == MirroredItemKind::Part)
            .map(|record| record.source_id.clone())
            .collect();
        let pair_part_record_count = context
            .pair_right
            .correspondences
            .iter()
            .filter(|record| record.kind == MirroredItemKind::Part)
            .count();
        if input.baseline_kind == BaselineKind::MaterializedCurrentSchema
            && (pair_part_sources != associated_sources
                || pair_part_sources.len() != pair_part_record_count)
        {
            return Err(
                "The pair and transition Part associations do not have bidirectional closure"
                    .into(),
            );
        }
        let association_by_source: BTreeMap<_, _> = context
            .associations
            .iter()
            .filter(|association| association.kind == MirroredItemKind::Part)
            .map(|association| (association.source_id.as_str(), association))
            .collect();
        for (old_source, new_source) in input.source_identity_map {
            let association = association_by_source
                .get(old_source.as_str())
                .ok_or_else(|| {
                    format!("The source identity {old_source:?} has no private association")
                })?;
            let pending_rejoin = context.pending_rejoins.contains_key(old_source);
            if association.state == TransitionState::Linked
                && new_source.is_some()
                && (crate::matrix::layout::companion_mode(input.valid_before, old_source)
                    != crate::model::MatrixCompanionMode::Follows
                    || (!pending_rejoin
                        && crate::matrix::layout::companion_mode(
                            input.valid_before,
                            &association.mirror_id,
                        ) != crate::model::MatrixCompanionMode::Follows))
            {
                return Err(
                    "Linked matrix placements cannot retain authored or constrained geometry"
                        .into(),
                );
            }
            let expected_mirror = new_source.as_ref().and_then(|id| {
                id.strip_prefix(&format!("matrix/{}/", matrix_pair.source_id.as_str()))
                    .map(|suffix| format!("matrix/{}/{suffix}", matrix_pair.mirror_id.as_str()))
            });
            let mapped_mirror = input.mirror_identity_map.get(&association.mirror_id);
            match association.state {
                TransitionState::Linked
                    if mapped_mirror != Some(&expected_mirror)
                        && !(context.pending_rejoins.get(old_source)
                            == Some(&TransitionState::Omitted)
                            && mapped_mirror.is_none()
                            && expected_mirror.as_deref()
                                == Some(association.mirror_id.as_str())) =>
                {
                    return Err(format!(
                        "The linked mirror identity disagrees with source identity {old_source:?}"
                    ));
                }
                TransitionState::Local
                    if new_source.is_some()
                        && before_mirror_members.contains(&association.mirror_id)
                        && mapped_mirror.is_none_or(Option::is_none) =>
                {
                    return Err(format!(
                        "The retained Local identity {:?} has no mirror mapping while its source remains",
                        association.mirror_id
                    ));
                }
                TransitionState::Omitted if mapped_mirror.is_some_and(Option::is_some) => {
                    return Err(format!(
                        "The omitted mirror identity {:?} remains materialized",
                        association.mirror_id
                    ));
                }
                TransitionState::Local
                    if new_source.is_none() && mapped_mirror.is_some_and(Option::is_some) =>
                {
                    let target = mapped_mirror.and_then(Option::as_ref).unwrap();
                    if input.final_owners.get(target)
                        != Some(&FinalOwner::Board(context.mirror_board_id.clone()))
                    {
                        return Err(format!(
                            "The retained Local identity {target:?} has no final mirror-board owner"
                        ));
                    }
                }
                _ => {}
            }
        }
        let planned_source_ids: BTreeSet<_> = input
            .planned_additions
            .iter()
            .map(|addition| addition.source_id.clone())
            .collect();
        if planned_source_ids.len() != input.planned_additions.len()
            || mapped_source_ids
                .union(&planned_source_ids)
                .cloned()
                .collect::<BTreeSet<_>>()
                != post_source_members
        {
            return Err(
                "The source operation map does not describe the post-operation matrix".into(),
            );
        }

        let association_keys: BTreeSet<_> = context
            .associations
            .iter()
            .map(|association| format!("{:?}\0{}", association.kind, association.source_id))
            .collect();
        let association_mirrors: BTreeSet<_> = context
            .associations
            .iter()
            .map(|association| format!("{:?}\0{}", association.kind, association.mirror_id))
            .collect();
        if association_keys.len() != context.associations.len()
            || association_mirrors.len() != context.associations.len()
        {
            return Err("The transition association context contains aliased identities".into());
        }
        for association in &context.associations {
            if association.state == TransitionState::Linked
                && (!association_keys.contains(&format!(
                    "{:?}\0{}",
                    association.kind, association.source_id
                )) || association.explicit_state)
            {
                return Err(
                    "The linked transition association has invalid state provenance".into(),
                );
            }
            match (
                association.parent_source_id.as_deref(),
                association.parent_mirror_id.as_deref(),
            ) {
                (None, None) => {
                    if association.state != TransitionState::Linked && !association.explicit_state {
                        return Err(
                            "A Local or Omitted owner lacks explicit state provenance".into()
                        );
                    }
                }
                (Some(parent_source), Some(parent_mirror)) => {
                    let source_child = association
                        .source_id
                        .strip_prefix(&format!("{parent_source}/"));
                    let mirror_child = association
                        .mirror_id
                        .strip_prefix(&format!("{parent_mirror}/"));
                    if source_child.is_none() || source_child != mirror_child {
                        return Err(
                            "The transition association has an invalid parent source".into()
                        );
                    }
                    let parent = context.associations.iter().find(|candidate| {
                        candidate.kind == association.kind
                            && candidate.source_id == parent_source
                            && candidate.mirror_id == parent_mirror
                    });
                    let Some(parent) = parent else {
                        return Err("The transition association has an unavailable parent".into());
                    };
                    if !association.explicit_state && association.state != parent.state {
                        return Err(
                            "An inherited child state disagrees with its parent association".into(),
                        );
                    }
                    if parent.state == TransitionState::Local
                        && association.state == TransitionState::Omitted
                        && !association.explicit_state
                    {
                        return Err(
                            "An Omitted child of a Local owner must be an explicit child override"
                                .into(),
                        );
                    }
                }
                _ => {
                    return Err(
                        "The transition association has an incomplete parent closure".into(),
                    );
                }
            }
        }
        for old_id in input.explicit_removal_closure {
            let retained_omitted_identity = context.associations.iter().any(|association| {
                association.mirror_id == *old_id && association.state == TransitionState::Omitted
            });
            if !before_source_members.contains(old_id)
                && !before_mirror.part_ids.contains(old_id)
                && !retained_omitted_identity
            {
                return Err(format!(
                    "The removal closure contains unproven identity {old_id:?}"
                ));
            }
        }

        let mut required_owners = BTreeSet::new();
        required_owners.extend(input.valid_before.parts.iter().map(|part| part.id.clone()));
        required_owners.extend(input.post_source.parts.iter().map(|part| part.id.clone()));
        required_owners.extend(
            input
                .planned_additions
                .iter()
                .flat_map(|addition| [addition.source_id.clone(), addition.mirror_id.clone()]),
        );
        required_owners.extend(context.owners.keys().cloned());
        if input.final_owners.keys().cloned().collect::<BTreeSet<_>>() != required_owners {
            return Err(
                "The final owner map is not total for the transition identity universe".into(),
            );
        }
        let mut transition_targets = mapped_source_ids.clone();
        for id in &mapped_source_ids {
            if let Some((row, column, child)) =
                matrix_identity_slot(id, matrix_pair.source_id.as_str())
            {
                let suffix = if child.is_empty() {
                    String::new()
                } else {
                    format!("/{child}")
                };
                transition_targets.insert(format!(
                    "matrix/{}/r{row}c{column}{suffix}",
                    matrix_pair.mirror_id.as_str()
                ));
            }
        }
        transition_targets.extend(
            input
                .planned_additions
                .iter()
                .flat_map(|addition| [addition.source_id.clone(), addition.mirror_id.clone()]),
        );
        for (id, owner) in input.final_owners {
            match owner {
                FinalOwner::Board(board_id) => {
                    if !input
                        .post_source
                        .boards
                        .iter()
                        .any(|board| board.id == *board_id)
                    {
                        return Err(format!("The final owner board {board_id:?} is unavailable"));
                    }
                    let expected_matrix_board =
                        if matrix_identity_slot(id, matrix_pair.source_id.as_str()).is_some() {
                            Some(context.source_board_id.as_str())
                        } else if matrix_identity_slot(id, matrix_pair.mirror_id.as_str()).is_some()
                        {
                            Some(context.mirror_board_id.as_str())
                        } else {
                            None
                        };
                    if expected_matrix_board.is_some_and(|expected| expected != board_id)
                        || (expected_matrix_board.is_none()
                            && !transition_targets.contains(id)
                            && board_owner(input.post_source, id)? != *owner)
                    {
                        return Err(format!(
                            "The final owner for identity {id:?} contradicts its operation-bound board"
                        ));
                    }
                }
                FinalOwner::Absent => {
                    let remains_after_source =
                        input.post_source.parts.iter().any(|part| part.id == *id)
                            || input
                                .post_source
                                .boards
                                .iter()
                                .any(|board| board.part_ids.contains(id));
                    if remains_after_source && !input.explicit_removal_closure.contains(id) {
                        return Err(format!(
                            "Absent identity {id:?} lacks explicit removal evidence"
                        ));
                    }
                }
            }
        }

        let mut planned_source_ids = BTreeSet::new();
        let mut planned_mirror_ids = BTreeSet::new();
        let mut planned_references = BTreeSet::new();
        for addition in input.planned_additions {
            let source_parts: Vec<_> = input
                .post_source
                .parts
                .iter()
                .filter(|part| part.id == addition.source_id)
                .collect();
            let Some(source_part) = source_parts.first() else {
                return Err(format!(
                    "The planned source Part {:?} is unavailable",
                    addition.source_id
                ));
            };
            let source_slot =
                matrix_identity_slot(&addition.source_id, matrix_pair.source_id.as_str())
                    .ok_or_else(|| {
                        format!(
                            "The planned source identity {:?} is not canonical",
                            addition.source_id
                        )
                    })?;
            let mirror_slot =
                matrix_identity_slot(&addition.mirror_id, matrix_pair.mirror_id.as_str())
                    .ok_or_else(|| {
                        format!(
                            "The planned mirror identity {:?} is not canonical",
                            addition.mirror_id
                        )
                    })?;
            let expected_mirror = addition
                .source_id
                .strip_prefix(&format!("matrix/{}/", matrix_pair.source_id.as_str()))
                .map(|suffix| format!("matrix/{}/{suffix}", matrix_pair.mirror_id.as_str()));
            let expected_parents = if source_slot.2.is_empty() {
                (None, None)
            } else {
                let source_key = format!(
                    "matrix/{}/r{}c{}",
                    matrix_pair.source_id.as_str(),
                    source_slot.0,
                    source_slot.1
                );
                let mirror_key = format!(
                    "matrix/{}/r{}c{}",
                    matrix_pair.mirror_id.as_str(),
                    mirror_slot.0,
                    mirror_slot.1
                );
                (Some(source_key), Some(mirror_key))
            };
            if addition.source_id.is_empty()
                || addition.mirror_id.is_empty()
                || addition.source_reference.is_empty()
                || addition.mirror_reference.is_empty()
                || source_parts.len() != 1
                || source_part.reference != addition.source_reference
                || source_slot.0 != mirror_slot.0
                || source_slot.1 != mirror_slot.1
                || source_slot.2 != mirror_slot.2
                || expected_mirror.as_deref() != Some(addition.mirror_id.as_str())
                || (
                    addition.parent_source_id.clone(),
                    addition.parent_mirror_id.clone(),
                ) != expected_parents
                || !planned_source_ids.insert(addition.source_id.clone())
                || !planned_mirror_ids.insert(addition.mirror_id.clone())
                || !planned_references.insert(addition.source_reference.clone())
                || !planned_references.insert(addition.mirror_reference.clone())
                || input.valid_before.parts.iter().any(|part| {
                    part.id == addition.source_id
                        || part.id == addition.mirror_id
                        || part.reference == addition.source_reference
                        || part.reference == addition.mirror_reference
                })
                || input.valid_before.parts.iter().any(|part| {
                    part.id == addition.mirror_id || part.reference == addition.mirror_reference
                })
                || input.post_source.parts.iter().any(|part| {
                    part.id == addition.mirror_id
                        || (part.id != addition.source_id
                            && part.reference == addition.source_reference)
                        || part.reference == addition.mirror_reference
                })
                || (source_slot.2.is_empty() && expected_parents != (None, None))
                || (!source_slot.2.is_empty()
                    && (!input
                        .post_source
                        .parts
                        .iter()
                        .any(|part| part.id == expected_parents.0.as_deref().unwrap_or_default())
                        || !(input.planned_additions.iter().any(|parent| {
                            parent.source_id == expected_parents.0.as_deref().unwrap_or_default()
                                && parent.mirror_id
                                    == expected_parents.1.as_deref().unwrap_or_default()
                        }) || input.valid_before.parts.iter().any(|part| {
                            part.id == expected_parents.1.as_deref().unwrap_or_default()
                        }))))
            {
                return Err("A planned transition addition collides or is incomplete".into());
            }
        }
        Ok(ValidatedTransitionContext {
            input,
            removes_paired_matrices: post_source.is_none(),
            already_removed: false,
            before_source_members,
            before_mirror_members,
            post_source_members,
            mapped_source_ids,
            mapped_mirror_ids,
        })
    }

    let validated = validate_transition_input(input)?;
    let input = validated.input;
    let context = input.context;
    let mapped_mirror_ids = &validated.mapped_mirror_ids;

    if validated.removes_paired_matrices {
        let mut draft = input.post_source.clone();
        let mirror_removal = if validated.already_removed {
            Vec::new()
        } else {
            let retained_local_parts: BTreeSet<_> = input
                .mirror_identity_map
                .iter()
                .filter_map(|(old, new)| new.as_ref().map(|_| old.clone()))
                .collect();
            if !retained_local_parts.is_empty() {
                let mirror = draft
                    .matrices
                    .iter_mut()
                    .find(|matrix| matrix.id == matrix_pair.mirror_id.as_str())
                    .ok_or_else(|| {
                        "The target matrix disappeared before Local detachment".to_owned()
                    })?;
                mirror
                    .part_ids
                    .retain(|id| !retained_local_parts.contains(id));
            }
            crate::apply(
                &mut draft,
                &[],
                &crate::model::EditOperation::RemoveMatrix {
                    id: matrix_pair.mirror_id.clone(),
                },
            )?
        };
        let removed_sources: BTreeSet<_> = context
            .associations
            .iter()
            .filter(|association| {
                let retained_local = association.state == TransitionState::Local
                    && input
                        .mirror_identity_map
                        .get(&association.mirror_id)
                        .is_some_and(Option::is_some);
                !retained_local
                    && (association.kind == MirroredItemKind::Matrix
                        || validated
                            .before_source_members
                            .contains(&association.source_id)
                        || validated
                            .before_mirror_members
                            .contains(&association.mirror_id))
            })
            .map(|association| association.source_id.clone())
            .collect();
        let removed_mirrors: BTreeSet<_> = context
            .associations
            .iter()
            .filter(|association| {
                let retained_local = association.state == TransitionState::Local
                    && input
                        .mirror_identity_map
                        .get(&association.mirror_id)
                        .is_some_and(Option::is_some);
                !retained_local
                    && (association.kind == MirroredItemKind::Matrix
                        || validated
                            .before_source_members
                            .contains(&association.source_id)
                        || validated
                            .before_mirror_members
                            .contains(&association.mirror_id))
            })
            .map(|association| association.mirror_id.clone())
            .collect();
        let mut next_context = context.clone();
        let retired: Vec<_> = next_context
            .associations
            .iter()
            .filter(|association| {
                removed_sources.contains(&association.source_id)
                    && removed_mirrors.contains(&association.mirror_id)
            })
            .cloned()
            .collect();
        next_context.associations.retain(|association| {
            !removed_sources.contains(&association.source_id)
                || !removed_mirrors.contains(&association.mirror_id)
        });
        next_context.owners = input.final_owners.clone();
        let context_delta = MembershipContextDelta {
            retired,
            owners: input.final_owners.clone(),
            ..MembershipContextDelta::default()
        };
        let expected_mirror_members = BTreeSet::new();
        let output = emit_validated_transition(
            &validated,
            draft,
            next_context,
            context_delta,
            &expected_mirror_members,
        )?;
        if !validated.already_removed && mirror_removal.is_empty() {
            return Err("The paired mirror matrix removal produced no document changes".into());
        }
        return Ok(output);
    }

    let is_identity = input.valid_before == input.post_source
        && input
            .source_identity_map
            .iter()
            .all(|(old, new)| new.as_deref() == Some(old))
        && input
            .mirror_identity_map
            .iter()
            .all(|(old, new)| new.as_deref() == Some(old))
        && validated.before_source_members == validated.mapped_source_ids
        && validated.before_mirror_members == validated.mapped_mirror_ids
        && input.planned_additions.is_empty()
        && input.explicit_removal_closure.is_empty()
        && context.pending_rejoins.is_empty()
        && context.pending_pair_transitions.is_empty();
    if is_identity {
        if context.owners != *input.final_owners {
            return Err("An identity transition changes its bound owner context".into());
        }
        let next_context = context.clone();
        let context_delta = MembershipContextDelta {
            owners: input.final_owners.clone(),
            ..MembershipContextDelta::default()
        };
        let expected_mirror_members = validated.mapped_mirror_ids.clone();
        return emit_validated_transition(
            &validated,
            input.post_source.clone(),
            next_context,
            context_delta,
            &expected_mirror_members,
        );
    }

    let mut removed_slots = BTreeSet::new();
    let mut shifted_slots = BTreeSet::new();
    for (old_id, new_id) in input.source_identity_map {
        let Some((old_row, old_column, old_suffix)) =
            matrix_identity_slot(old_id, matrix_pair.source_id.as_str())
        else {
            return Err(format!("The source identity {old_id:?} is not canonical"));
        };
        match new_id {
            None => {
                removed_slots.insert((old_row, old_column, old_suffix.to_owned()));
                if !input.explicit_removal_closure.contains(old_id) {
                    return Err(format!(
                        "Removed source identity {old_id:?} lacks explicit operation evidence"
                    ));
                }
            }
            Some(new_id) => {
                let Some((new_row, new_column, new_suffix)) =
                    matrix_identity_slot(new_id, matrix_pair.source_id.as_str())
                else {
                    return Err(format!(
                        "The mapped source identity {new_id:?} is not canonical"
                    ));
                };
                if old_suffix != new_suffix {
                    return Err("The source identity map changes a companion identity".into());
                }
                if (old_row, old_column) != (new_row, new_column) {
                    shifted_slots.insert(((old_row, old_column), (new_row, new_column)));
                }
            }
        }
    }
    let is_compaction = !removed_slots.is_empty()
        && !shifted_slots.is_empty()
        && input.planned_additions.is_empty();
    let is_trim =
        !removed_slots.is_empty() && shifted_slots.is_empty() && input.planned_additions.is_empty();
    let is_expansion =
        removed_slots.is_empty() && shifted_slots.is_empty() && !input.planned_additions.is_empty();
    let is_rejoin = removed_slots.is_empty()
        && shifted_slots.is_empty()
        && input.planned_additions.is_empty()
        && (!context.pending_rejoins.is_empty() || !context.pending_pair_transitions.is_empty());
    if !is_compaction && !is_trim && !is_expansion && !is_rejoin {
        return Err("The source operation is not a supported identity, trim, expansion, or single-axis compaction".into());
    }
    let compaction_axis = if is_compaction {
        Some({
            let &(removed_row, removed_column, _) = removed_slots
                .iter()
                .next()
                .ok_or_else(|| "The source operation has no removed slot".to_owned())?;
            let row_axis =
                shifted_slots
                    .iter()
                    .all(|((old_row, old_column), (new_row, new_column))| {
                        old_column == new_column && old_row != new_row
                    });
            let column_axis =
                shifted_slots
                    .iter()
                    .all(|((old_row, old_column), (new_row, new_column))| {
                        old_row == new_row && old_column != new_column
                    });
            if row_axis == column_axis {
                return Err(
                    "The source identity map does not describe one unambiguous axis compaction"
                        .into(),
                );
            }
            let index = if row_axis {
                removed_row
            } else {
                removed_column
            };
            if removed_slots.iter().any(|(row, column, _)| {
                if row_axis {
                    *row != index
                } else {
                    *column != index
                }
            }) {
                return Err("The source operation removes more than one matrix axis".into());
            }
            for (old, new) in &shifted_slots {
                let (old_axis, new_axis, other_axis_equal) = if row_axis {
                    (old.0, new.0, old.1 == new.1)
                } else {
                    (old.1, new.1, old.0 == new.0)
                };
                if !other_axis_equal
                    || old_axis == index
                    || new_axis != old_axis - u32::from(old_axis > index)
                {
                    return Err("The source identity map is not a canonical axis compaction".into());
                }
            }
            (row_axis, index)
        })
    } else {
        None
    };

    let mut staged = input.post_source.clone();
    for association in context.associations.iter().filter(|association| {
        association.kind == MirroredItemKind::Part
            && context.pending_rejoins.contains_key(&association.source_id)
            && association.state == TransitionState::Linked
    }) {
        let mut parts = staged
            .parts
            .iter_mut()
            .filter(|part| part.id == association.mirror_id);
        let Some(target) = parts.next() else {
            if context.pending_rejoins.get(&association.source_id)
                == Some(&TransitionState::Omitted)
            {
                // The matrix setter below materializes this authenticated absent target.
                continue;
            }
            return Err(format!(
                "The Rejoined mirror Part {:?} is unavailable",
                association.mirror_id
            ));
        };
        if parts.next().is_some() {
            return Err(format!(
                "The Rejoined mirror Part {:?} is ambiguous",
                association.mirror_id
            ));
        }
        if let Some(properties) = &mut target.properties {
            properties.remove("layoutOverride");
        }
    }
    if let Some((axis_row, removed_index)) = compaction_axis {
        crate::matrix::deletion::remove_axis(
            &mut staged,
            matrix_pair.mirror_id.as_str(),
            removed_index,
            axis_row,
        )?;
    } else if is_trim {
        let old_mirror = matrix_by_id(&staged, matrix_pair.mirror_id.as_str())?;
        let mut removed_cells: BTreeSet<_> = removed_slots
            .iter()
            .filter_map(|(row, column, suffix)| suffix.is_empty().then_some((*row, *column)))
            .collect();
        if removed_cells.is_empty() {
            return Err("The trimmed membership removes no complete matrix key".into());
        }
        if removed_cells
            .iter()
            .any(|(row, column)| *row >= old_mirror.rows || *column >= old_mirror.columns)
        {
            return Err(
                "The trim operation removes an identity outside the old target matrix".into(),
            );
        }
        let local_removed_cells: BTreeSet<_> = context
            .associations
            .iter()
            .filter(|association| {
                association.kind == MirroredItemKind::Part
                    && association.state == TransitionState::Local
                    && input.source_identity_map.get(&association.source_id) == Some(&None)
                    && input
                        .mirror_identity_map
                        .get(&association.mirror_id)
                        .is_some_and(Option::is_some)
            })
            .filter_map(|association| {
                matrix_identity_slot(&association.mirror_id, matrix_pair.mirror_id.as_str())
                    .map(|(row, column, _)| (row, column))
            })
            .collect();
        if !local_removed_cells.is_empty() {
            let target = staged
                .matrices
                .iter_mut()
                .find(|matrix| matrix.id == matrix_pair.mirror_id.as_str())
                .ok_or_else(|| {
                    "The target matrix disappeared before Local detachment".to_owned()
                })?;
            let protected_ids: BTreeSet<_> = target
                .part_ids
                .iter()
                .filter(|id| {
                    matrix_identity_slot(id, matrix_pair.mirror_id.as_str()).is_some_and(
                        |(row, column, _)| local_removed_cells.contains(&(row, column)),
                    )
                })
                .cloned()
                .collect();
            target.part_ids.retain(|id| !protected_ids.contains(id));
            for cell in &mut target.cells {
                if local_removed_cells.contains(&(cell.row, cell.column)) {
                    cell.deleted = true;
                    cell.enabled = false;
                    cell.assemblies.clear();
                    cell.assemblies_local = None;
                }
            }
            removed_cells.retain(|cell| !local_removed_cells.contains(cell));
        }
        if !removed_cells.is_empty() {
            crate::matrix::deletion::remove_cells(
                &mut staged,
                matrix_pair.mirror_id.as_str(),
                &removed_cells.into_iter().collect::<Vec<_>>(),
            )?;
        }
    }
    let source = matrix_by_id(&staged, matrix_pair.source_id.as_str())?.clone();
    let mirror = matrix_by_id(&staged, matrix_pair.mirror_id.as_str())?.clone();
    let matrix_correspondence = context
        .pair_right
        .correspondences
        .iter()
        .find(|association| {
            association.kind == MirroredItemKind::Matrix
                && association.source_id == matrix_pair.source_id.as_str()
                && association.mirror_id == matrix_pair.mirror_id.as_str()
        })
        .ok_or_else(|| "The transition matrix association is unavailable".to_owned())?;
    let mut projected = project_matrix(&source, &mirror, matrix_correspondence, context.mapping)?;
    let mut local_key_preservations = Vec::new();
    for cell in &mut projected.cells {
        if !cell.enabled || cell.deleted {
            continue;
        }
        let source_key_id = format!(
            "matrix/{}/r{}c{}",
            matrix_pair.source_id.as_str(),
            cell.row,
            cell.column
        );
        let Some(source_key) = staged.parts.iter().find(|part| part.id == source_key_id) else {
            return Err(format!(
                "The projected source key {source_key_id:?} is unavailable"
            ));
        };
        let original_key = context.associations.iter().find(|association| {
            association.kind == MirroredItemKind::Part
                && input.source_identity_map.get(&association.source_id)
                    == Some(&Some(source_key_id.clone()))
        });
        if original_key.is_some_and(|association| association.state == TransitionState::Omitted) {
            cell.deleted = true;
            cell.enabled = false;
            cell.assemblies.clear();
            cell.assemblies_local = None;
            continue;
        }
        if original_key.is_some_and(|association| association.state == TransitionState::Local) {
            let local_cell = mirror
                .cells
                .iter()
                .find(|candidate| candidate.row == cell.row && candidate.column == cell.column)
                .ok_or_else(|| {
                    format!(
                        "The retained Local matrix cell r{}c{} is unavailable",
                        cell.row, cell.column
                    )
                })?;
            let association = original_key.expect("the Local key association was checked");
            let mapped_source = input
                .source_identity_map
                .get(&association.source_id)
                .and_then(Clone::clone)
                .ok_or_else(|| {
                    "The retained Local key has no source identity mapping".to_owned()
                })?;
            let target_id = input
                .mirror_identity_map
                .get(&association.mirror_id)
                .and_then(Clone::clone)
                .or_else(|| {
                    mapped_source
                        .strip_prefix(&format!("matrix/{}/", matrix_pair.source_id.as_str()))
                        .map(|suffix| format!("matrix/{}/{suffix}", matrix_pair.mirror_id.as_str()))
                })
                .ok_or_else(|| {
                    "The retained Local key has no target identity mapping".to_owned()
                })?;
            local_key_preservations.push((
                cell.row,
                cell.column,
                association.mirror_id.clone(),
                target_id,
            ));
            *cell = local_cell.clone();
            cell.assemblies.retain(|assembly| {
                let source_child_id = crate::matrix::companion_id(&source_key_id, &assembly.id);
                context.associations.iter().all(|child| {
                    child.kind != MirroredItemKind::Part
                        || input.source_identity_map.get(&child.source_id)
                            != Some(&Some(source_child_id.clone()))
                        || child.state != TransitionState::Omitted
                        || !child.explicit_state
                })
            });
            continue;
        }
        let mut next_assemblies = Vec::with_capacity(cell.assemblies.len());
        for mut assembly in std::mem::take(&mut cell.assemblies) {
            let source_child_id = crate::matrix::companion_id(&source_key_id, &assembly.id);
            let mirror_child_id = crate::matrix::companion_id(
                &format!(
                    "matrix/{}/r{}c{}",
                    matrix_pair.mirror_id.as_str(),
                    cell.row,
                    cell.column
                ),
                &assembly.id,
            );
            let original = context.associations.iter().find(|association| {
                association.kind == MirroredItemKind::Part
                    && input.source_identity_map.get(&association.source_id)
                        == Some(&Some(source_child_id.clone()))
            });
            if original.is_none()
                && !input
                    .planned_additions
                    .iter()
                    .any(|addition| addition.source_id == source_child_id)
            {
                return Err(format!(
                    "The companion settings for {source_child_id:?} are unavailable"
                ));
            }
            if original.is_some_and(|association| association.state == TransitionState::Omitted) {
                continue;
            }
            if original.is_some_and(|association| association.state == TransitionState::Local) {
                let local_assembly = mirror
                    .cells
                    .iter()
                    .find(|candidate| candidate.row == cell.row && candidate.column == cell.column)
                    .and_then(|candidate| {
                        candidate
                            .assemblies
                            .iter()
                            .find(|item| item.id == assembly.id)
                    })
                    .ok_or_else(|| {
                        format!("The retained Local companion {mirror_child_id:?} is unavailable")
                    })?;
                assembly = local_assembly.clone();
                next_assemblies.push(assembly);
                continue;
            }
            let mode = original
                .map(|association| association.effective_mode)
                .unwrap_or(context.pair_right.companion_mode);
            let tweak = original.and_then(|association| association.effective_tweak);
            let source_companion = staged
                .parts
                .iter()
                .find(|part| part.id == source_child_id)
                .ok_or_else(|| {
                    format!("The projected source companion {source_child_id:?} is unavailable")
                })?;
            let (offset, rotation) =
                super::derive::companion_assembly(super::derive::CompanionAssemblyInput {
                    offset: assembly.offset,
                    rotation: assembly.rotation,
                    key: Some(source_key),
                    companion: Some(source_companion),
                    mapping: context.mapping,
                    mode,
                    tweak,
                    stabiliser: super::SYMMETRIC_DEFINITIONS
                        .contains(&source_companion.definition_id.as_str()),
                    fallback: false,
                })
                .map_err(str::to_owned)?;
            if !offset.x.is_finite()
                || !offset.y.is_finite()
                || rotation.is_some_and(|value| !value.is_finite())
            {
                return Err("The projected matrix companion contains non-finite geometry".into());
            }
            assembly.offset = offset;
            assembly.rotation = rotation;
            next_assemblies.push(assembly);
        }
        cell.assemblies = next_assemblies;
    }
    if !local_key_preservations.is_empty() {
        let mut candidate = staged.clone();
        crate::matrix::set_matrix(&mut candidate, &projected)?;
        for (row, column, old_id, target_id) in &local_key_preservations {
            let before_part = input
                .valid_before
                .parts
                .iter()
                .find(|part| part.id == *old_id)
                .ok_or_else(|| format!("The Local key snapshot {old_id:?} is unavailable"))?;
            let candidate_part = candidate
                .parts
                .iter()
                .find(|part| part.id == *target_id)
                .ok_or_else(|| format!("The candidate Local key {target_id:?} is unavailable"))?;
            let world_delta = Vec2 {
                x: before_part.pose.at.x - candidate_part.pose.at.x,
                y: before_part.pose.at.y - candidate_part.pose.at.y,
            };
            let offset_delta = cell_offset_delta(&projected, *column, world_delta)?;
            let cell = projected
                .cells
                .iter_mut()
                .find(|cell| cell.row == *row && cell.column == *column)
                .ok_or_else(|| format!("The Local matrix cell r{row}c{column} is unavailable"))?;
            let offset = cell.offset.unwrap_or_default();
            cell.offset = Some(Vec2 {
                x: offset.x + offset_delta.x,
                y: offset.y + offset_delta.y,
            });
        }
    }
    let mut expected_final_mirror_members = mapped_mirror_ids.clone();
    expected_final_mirror_members.extend(context.pending_rejoins.iter().filter_map(
        |(source_id, previous_state)| {
            if *previous_state == TransitionState::Omitted {
                context
                    .associations
                    .iter()
                    .find(|association| {
                        association.kind == MirroredItemKind::Part
                            && association.source_id == *source_id
                            && association.state == TransitionState::Linked
                    })
                    .map(|association| association.mirror_id.clone())
            } else {
                None
            }
        },
    ));
    expected_final_mirror_members.extend(
        input
            .planned_additions
            .iter()
            .map(|addition| addition.mirror_id.clone()),
    );
    let locally_detached_ids: BTreeSet<_> = context
        .associations
        .iter()
        .filter(|association| {
            association.kind == MirroredItemKind::Part
                && association.state == TransitionState::Local
                && input.source_identity_map.get(&association.source_id) == Some(&None)
        })
        .filter_map(|association| {
            input
                .mirror_identity_map
                .get(&association.mirror_id)
                .and_then(Clone::clone)
        })
        .collect();
    for id in &locally_detached_ids {
        expected_final_mirror_members.remove(id);
    }
    let removed_target_ids: BTreeSet<_> = mirror
        .part_ids
        .iter()
        .filter(|id| {
            !expected_final_mirror_members.contains(*id) && !locally_detached_ids.contains(*id)
        })
        .cloned()
        .collect();
    let _pruned_ids = crate::matrix::deletion::remove_parts_by_id(&mut staged, &removed_target_ids);
    crate::matrix::set_matrix(&mut staged, &projected)?;
    deduplicate_outline_members(&mut staged);
    for addition in input.planned_additions {
        let source_part = input
            .post_source
            .parts
            .iter()
            .find(|part| part.id == addition.source_id)
            .ok_or_else(|| {
                format!(
                    "The planned source part {:?} disappeared before materialization",
                    addition.source_id
                )
            })?;
        let mirror_part = staged
            .parts
            .iter_mut()
            .find(|part| part.id == addition.mirror_id)
            .ok_or_else(|| {
                format!(
                    "The planned mirror part {:?} was not generated",
                    addition.mirror_id
                )
            })?;
        mirror_part.reference = addition.mirror_reference.clone();
        mirror_part.locked = source_part.locked;
        mirror_part.keycap = source_part.keycap;
        mirror_part.outline = source_part.outline.clone();
        mirror_part.properties = source_part.properties.clone();
        mirror_part.generator_parameters = source_part.generator_parameters.clone();
    }

    let mut next_context = context.clone();
    let mut next_associations = Vec::with_capacity(context.associations.len());
    let mut delta = MembershipContextDelta {
        owners: input.final_owners.clone(),
        ..MembershipContextDelta::default()
    };
    for association in &context.associations {
        if association.kind != MirroredItemKind::Part {
            next_associations.push(association.clone());
            continue;
        }
        let mapped_source = input
            .source_identity_map
            .get(&association.source_id)
            .ok_or_else(|| {
                format!(
                    "The transition map omits associated source identity {:?}",
                    association.source_id
                )
            })?;
        let Some(mapped_source) = mapped_source else {
            if !input
                .explicit_removal_closure
                .contains(&association.source_id)
            {
                return Err(
                    "A removed source association lacks explicit operation evidence".into(),
                );
            }
            let mapped_mirror = input.mirror_identity_map.get(&association.mirror_id);
            if association.state == TransitionState::Omitted && mapped_mirror.is_none() {
                if !input
                    .explicit_removal_closure
                    .contains(&association.mirror_id)
                {
                    return Err(
                        "A removed Omitted association lacks its private closure evidence".into(),
                    );
                }
                delta.retired.push(association.clone());
                continue;
            }
            let mapped_mirror = mapped_mirror.ok_or_else(|| {
                format!("The mirror identity map omits {:?}", association.mirror_id)
            })?;
            if association.state == TransitionState::Local
                && let Some(mapped_mirror) = mapped_mirror
            {
                if input.final_owners.get(mapped_mirror)
                    != Some(&FinalOwner::Board(context.mirror_board_id.clone()))
                {
                    return Err(format!(
                        "Retained Local identity {mapped_mirror:?} has no final owner"
                    ));
                }
                // The visible Local part survives independently; its deleted source no longer
                // owns a durable association or private transition record.
                delta.retired.push(association.clone());
                continue;
            }
            if !input
                .explicit_removal_closure
                .contains(&association.mirror_id)
            {
                return Err("A removed association lacks explicit target closure evidence".into());
            }
            delta.retired.push(association.clone());
            continue;
        };
        let mapped_mirror_default = mapped_source
            .strip_prefix(&format!("matrix/{}/", matrix_pair.source_id.as_str()))
            .map(|suffix| format!("matrix/{}/{suffix}", matrix_pair.mirror_id.as_str()))
            .ok_or_else(|| "A mapped source identity is outside its matrix".to_owned())?;
        if association.state == TransitionState::Omitted {
            if input
                .mirror_identity_map
                .get(&association.mirror_id)
                .is_some_and(Option::is_some)
            {
                return Err(format!(
                    "The omitted mirror identity {:?} remains materialized",
                    association.mirror_id
                ));
            }
            let mut omitted = association.clone();
            omitted.source_id = mapped_source.clone();
            omitted.mirror_id = mapped_mirror_default;
            omitted.parent_source_id = association
                .parent_source_id
                .as_ref()
                .and_then(|id| input.source_identity_map.get(id))
                .and_then(Clone::clone);
            omitted.parent_mirror_id = omitted.parent_source_id.as_ref().map(|id| {
                id.strip_prefix(&format!("matrix/{}/", matrix_pair.source_id.as_str()))
                    .map(|suffix| format!("matrix/{}/{suffix}", matrix_pair.mirror_id.as_str()))
                    .unwrap_or_else(|| id.clone())
            });
            next_associations.push(omitted);
            continue;
        }
        let mapped_mirror = input
            .mirror_identity_map
            .get(&association.mirror_id)
            .and_then(Clone::clone)
            .unwrap_or(mapped_mirror_default);
        let mut relocated = association.clone();
        relocated.source_id = mapped_source.clone();
        relocated.mirror_id = mapped_mirror.clone();
        relocated.parent_source_id = association
            .parent_source_id
            .as_ref()
            .and_then(|id| input.source_identity_map.get(id))
            .and_then(Clone::clone);
        relocated.parent_mirror_id = relocated.parent_source_id.as_ref().map(|id| {
            id.strip_prefix(&format!("matrix/{}/", matrix_pair.source_id.as_str()))
                .map(|suffix| format!("matrix/{}/{suffix}", matrix_pair.mirror_id.as_str()))
                .unwrap_or_else(|| id.clone())
        });
        if relocated != *association {
            delta
                .relocated
                .push((association.source_id.clone(), mapped_source.clone()));
        }
        next_associations.push(relocated);
    }
    for addition in input.planned_additions {
        let association = TransitionAssociation {
            kind: MirroredItemKind::Part,
            source_id: addition.source_id.clone(),
            mirror_id: addition.mirror_id.clone(),
            parent_source_id: addition.parent_source_id.clone(),
            parent_mirror_id: addition.parent_mirror_id.clone(),
            state: TransitionState::Linked,
            explicit_state: false,
            raw_mode_override: None,
            raw_tweak: None,
            effective_mode: context.pair_right.companion_mode,
            effective_tweak: None,
        };
        delta.added.push(association.clone());
        next_associations.push(association);
    }
    next_context.associations = next_associations;
    next_context.owners = input.final_owners.clone();
    deduplicate_outline_members(&mut staged);
    emit_validated_transition(
        &validated,
        staged,
        next_context,
        delta,
        &expected_final_mirror_members,
    )
}

#[allow(dead_code)] // Ticket 109 atomically publishes the kernel result with durable association updates.
fn prepare_membership_transition(
    input: MembershipTransitionInput<'_>,
) -> Result<MembershipTransitionOutput, String> {
    if input.context.matrix_pair.is_none() {
        return Err("Matrix membership transition requires an exact Matrix correspondence".into());
    }
    let mut output = prepare_membership_transition_kernel(input)?;
    // The membership kernel retains detached Local geometry. Once its source is explicitly
    // deleted, the durable lifecycle also retires that geometry's former association.
    let detached: Vec<_> = output
        .context
        .associations
        .iter()
        .filter(|association| {
            association.kind == MirroredItemKind::Part
                && association.state == TransitionState::Local
                && input.source_identity_map.get(&association.source_id) == Some(&None)
                && input
                    .explicit_removal_closure
                    .contains(&association.source_id)
                && !output
                    .draft
                    .parts
                    .iter()
                    .any(|part| part.id == association.source_id)
        })
        .cloned()
        .collect();
    for association in detached {
        output.context.associations.retain(|a| {
            !(a.kind == association.kind
                && a.source_id == association.source_id
                && a.mirror_id == association.mirror_id)
        });
        output.context_delta.retired.push(association);
    }
    lifecycle::apply_membership_context_delta(input, &mut output)?;
    Ok(output)
}

#[allow(dead_code)] // Ticket 106 prepares a private transition slice before lifecycle activation.
fn matrix_identity_slot<'a>(id: &'a str, matrix_id: &str) -> Option<(u32, u32, &'a str)> {
    let prefix = format!("matrix/{matrix_id}/r");
    let suffix = id.strip_prefix(&prefix)?;
    let (slot, companion) = suffix
        .split_once('/')
        .map_or((suffix, ""), |(slot, child)| (slot, child));
    let (row, column) = slot.split_once('c')?;
    Some((row.parse().ok()?, column.parse().ok()?, companion))
}

#[allow(dead_code)] // Ticket 106 prepares a private transition slice before lifecycle activation.
fn changed_document_fields(
    before: &ProjectDoc,
    after: &ProjectDoc,
) -> Result<(Vec<String>, Vec<String>), String> {
    fn walk(
        before: &serde_json::Value,
        after: &serde_json::Value,
        path: &str,
        owner_id: Option<&str>,
        entity_array: bool,
        ids: &mut BTreeSet<String>,
        fields: &mut BTreeSet<String>,
    ) {
        if before == after {
            return;
        }
        match (before, after) {
            (serde_json::Value::Object(left), serde_json::Value::Object(right)) => {
                let keys: BTreeSet<_> = left.keys().chain(right.keys()).collect();
                for key in keys {
                    let child_path = format!("{path}.{key}");
                    let child_entity_array = (path == "$"
                        && matches!(
                            key.as_str(),
                            "parts"
                                | "matrices"
                                | "boards"
                                | "definitions"
                                | "nets"
                                | "constraints"
                                | "outline"
                                | "layouts"
                                | "modules"
                                | "embeddedCircuits"
                        ))
                        || child_path == "$.hardware.pairs";
                    match (left.get(key), right.get(key)) {
                        (Some(left), Some(right)) => walk(
                            left,
                            right,
                            &child_path,
                            owner_id,
                            child_entity_array,
                            ids,
                            fields,
                        ),
                        _ if owner_id.is_some() => {
                            ids.insert(owner_id.unwrap().to_owned());
                        }
                        _ => {
                            fields.insert(child_path);
                        }
                    }
                }
            }
            (serde_json::Value::Array(left), serde_json::Value::Array(right)) => {
                let left_by_id: Option<BTreeMap<_, _>> = left
                    .iter()
                    .map(|value| {
                        value
                            .get("id")
                            .and_then(serde_json::Value::as_str)
                            .map(|id| (id, value))
                    })
                    .collect();
                let right_by_id: Option<BTreeMap<_, _>> = right
                    .iter()
                    .map(|value| {
                        value
                            .get("id")
                            .and_then(serde_json::Value::as_str)
                            .map(|id| (id, value))
                    })
                    .collect();
                if let (Some(left_by_id), Some(right_by_id)) = (left_by_id, right_by_id) {
                    let keys: BTreeSet<_> = left_by_id
                        .keys()
                        .chain(right_by_id.keys())
                        .copied()
                        .collect();
                    for id in keys {
                        match (left_by_id.get(id), right_by_id.get(id)) {
                            (Some(left), Some(right)) => {
                                let item_owner = if entity_array { Some(id) } else { owner_id };
                                walk(
                                    left,
                                    right,
                                    &format!("{path}[id={id:?}]"),
                                    item_owner,
                                    false,
                                    ids,
                                    fields,
                                )
                            }
                            _ => {
                                if entity_array {
                                    ids.insert(id.to_owned());
                                } else if let Some(owner_id) = owner_id {
                                    ids.insert(owner_id.to_owned());
                                } else {
                                    fields.insert(format!("{path}[id={id:?}]"));
                                }
                            }
                        }
                    }
                    return;
                }
                let count = left.len().max(right.len());
                for index in 0..count {
                    let child_path = format!("{path}[{index}]");
                    match (left.get(index), right.get(index)) {
                        (Some(left), Some(right)) => {
                            walk(left, right, &child_path, owner_id, false, ids, fields)
                        }
                        _ if owner_id.is_some() => {
                            ids.insert(owner_id.unwrap().to_owned());
                        }
                        _ => {
                            fields.insert(child_path);
                        }
                    }
                }
            }
            _ if owner_id.is_some() => {
                ids.insert(owner_id.unwrap().to_owned());
            }
            _ => {
                fields.insert(path.to_owned());
            }
        }
    }

    let before = serde_json::to_value(before).map_err(|error| error.to_string())?;
    let after = serde_json::to_value(after).map_err(|error| error.to_string())?;
    let mut ids = BTreeSet::new();
    let mut fields = BTreeSet::new();
    walk(&before, &after, "$", None, false, &mut ids, &mut fields);
    Ok((ids.into_iter().collect(), fields.into_iter().collect()))
}

#[allow(dead_code)] // Ticket 106 prepares a private transition slice before lifecycle activation.
fn deduplicate_outline_members(document: &mut ProjectDoc) {
    fn deduplicate(values: &mut Vec<String>) {
        let mut seen = BTreeSet::new();
        values.retain(|value| seen.insert(value.clone()));
    }

    fn feature(feature: &mut crate::model::OutlineFeature) {
        if let crate::model::OutlineFeature::PartEnvelope { part_ids, .. } = feature {
            deduplicate(part_ids);
        }
    }

    fn snapshot(snapshot: &mut crate::model::OutlineSnapshot) {
        for feature_value in &mut snapshot.features {
            feature(feature_value);
        }
        for bridge in &mut snapshot.bridges {
            deduplicate(&mut bridge.part_ids);
        }
    }

    for feature_value in &mut document.outline {
        feature(feature_value);
    }
    for board_outline in &mut document.board_outlines {
        for version in &mut board_outline.versions {
            snapshot(&mut version.geometry);
        }
        if let Some(last_valid) = &mut board_outline.generated_last_valid {
            snapshot(last_valid);
        }
    }
}

#[cfg(test)]
mod tests {
    mod outlines_tests {
        include!("reconcile/outlines/tests.rs");
    }

    use std::collections::{BTreeMap, BTreeSet};

    use super::{
        BaselineKind, FinalOwner, HostedTransition, IndependentPartTransition, MatrixLink,
        MembershipContext, MembershipTransitionInput, PartProjection, PlannedAddition, Projection,
        TransitionAssociation, TransitionState, changed_document_fields,
        prepare_membership_transition, prepare_membership_transition_kernel, reconcile,
    };
    use crate::mirrored_half::{
        MechanicalChange, MechanicalEdit, MechanicalFeatureCorrespondence, MechanicalTarget,
    };
    use crate::model::*;

    fn fixture_mapping() -> Pose2 {
        Pose2 {
            at: Vec2 { x: 80.0, y: -25.0 },
            rotation: 31.0,
        }
    }

    fn pair_state_bytes(document: &ProjectDoc) -> Vec<u8> {
        serde_json::to_vec(&document.hardware.as_ref().unwrap().pairs[0]).unwrap()
    }

    fn fixture_board(id: &str) -> Board {
        Board {
            id: id.into(),
            name: id.into(),
            outline_ids: vec![],
            part_ids: vec![],
            net_ids: vec![],
            thickness: 1.6,
            traces: vec![],
            vias: vec![],
        }
    }

    fn fixture_mechanical_configuration(board_id: &str) -> MechanicalConfiguration {
        let mut configuration: MechanicalConfiguration =
            serde_json::from_value(serde_json::json!({
                "boardId": board_id,
                "method": "printed",
                "mount": "rigid",
                "plateThickness": 1.5,
                "plateFoamThickness": 0.5,
                "pcbThickness": 1.6,
                "bottomFoamThickness": 0.4,
                "batteryHeight": 4.0,
                "bottomThickness": 3.0,
                "plateToPcb": 3.5,
                "wallThickness": 2.0,
                "clearance": 0.3,
                "bottomStyle": "shell",
                "middleFrame": false,
                "openingAllowance": 0.25,
                "mounts": [],
                "closureMounts": [],
                "profiles": []
            }))
            .unwrap();
        configuration.mounts.push(Mount {
            id: "mount-1".into(),
            at: Vec2 { x: 2.0, y: 3.0 },
            kind: MountKind::Hole,
            hole_diameter: 2.2,
            boss_diameter: None,
            height: None,
        });
        configuration
    }

    fn fixture_instance(id: &str, board_id: &str) -> PhysicalBoardInstance {
        PhysicalBoardInstance {
            id: id.into(),
            name: id.into(),
            board_id: board_id.into(),
            half: id.into(),
            role: "controller".into(),
            flipped: false,
            controller_part_id: None,
            mechanical: Some(fixture_mechanical_configuration(board_id)),
            construction_ownership: MechanicalConstructionOwnership::Local,
            mechanical_identity_reservation: None,
        }
    }

    fn fixture_matrix(id: &str, board_id: &str) -> Matrix {
        Matrix {
            id: id.into(),
            name: None,
            rows: 1,
            columns: 2,
            pitch: Vec2 { x: 19.0, y: 19.0 },
            origin: Vec2::default(),
            definition_id: "switch".into(),
            part_ids: vec![],
            board_id: Some(board_id.into()),
            mirror: None,
            rotation: None,
            edge_gap: None,
            diode_direction: None,
            row_offsets: vec![],
            column_offsets: vec![],
            column_staggers: vec![],
            column_splays: vec![],
            column_origins: vec![],
            cells: (0..2)
                .map(|column| MatrixCell {
                    deleted: false,
                    row: 0,
                    column,
                    enabled: true,
                    definition_id: None,
                    variant: None,
                    offset: None,
                    rotation: None,
                    assemblies: vec![],
                    assemblies_local: None,
                })
                .collect(),
        }
    }

    fn fixture_mounting_hole() -> PartDefinition {
        serde_json::from_value(serde_json::json!({
            "id":"generator:ceoloide/mounting_hole_npth",
            "name":"mounting hole npth",
            "kind":"custom",
            "pads":[{"id":"pad-0","number":"","at":{"x":0,"y":0},
                "size":{"x":2.2,"y":2.2},"shape":"circle","drill":2.2,"plated":false,"rotation":0}],
            "courtyard":[{"x":-1.1,"y":-1.1},{"x":1.1,"y":-1.1},{"x":1.1,"y":1.1},{"x":-1.1,"y":1.1}],
            "envelopeSource":{"courtyard":"generated"},
            "envelopeNotice":"No closed courtyard is available; the outline uses physical graphics and pad extents.",
            "generator":{"source":"ceoloide/mounting_hole_npth","version":"bundled-1","parameters":{}}
        }))
        .unwrap()
    }

    fn fixture_document() -> Result<ProjectDoc, String> {
        let source_board = "source-board";
        let mirror_board = "mirror-board";
        let mut document = ProjectDoc::empty("reconcile-fixture", "Reconcile fixture");
        document.boards = vec![fixture_board(source_board), fixture_board(mirror_board)];
        document.definitions.push(
            serde_json::from_value(serde_json::json!({
                "id":"switch","name":"Switch","kind":"switch","courtyard":[],"pads":[]
            }))
            .unwrap(),
        );
        document.mechanical = Some(fixture_mechanical_configuration(source_board));
        document.hardware = Some(HardwareConfiguration {
            topology: HardwareTopology::Unibody,
            transport: HardwareTransport::None,
            instances: vec![
                fixture_instance("left", source_board),
                fixture_instance("right", mirror_board),
            ],
            boards: vec![],
            shared_construction: None,
            pairs: vec![],
        });

        crate::matrix::set_matrix(
            &mut document,
            &fixture_matrix("source-matrix", source_board),
        )?;
        crate::matrix::set_matrix(
            &mut document,
            &fixture_matrix("mirror-matrix", mirror_board),
        )?;

        crate::mechanical::edit::apply(
            &mut document,
            &[],
            &MechanicalEdit {
                target: MechanicalTarget::Board {
                    board_id: source_board.into(),
                },
                change: MechanicalChange::LinkMirrorPair {
                    pair_id: "pair-source-mirror".into(),
                    members: ["left".into(), "right".into()],
                    mapping: fixture_mapping(),
                    construction_source: "left".into(),
                    correspondences: vec![MechanicalFeatureCorrespondence {
                        kind: MechanicalFeatureKind::Mount,
                        first_feature_id: "mount-1".into(),
                        second_feature_id: "mount-1".into(),
                        source_instance_id: None,
                    }],
                },
                mounting_hole: Some(Box::new(fixture_mounting_hole())),
            },
        )?;

        let correspondences = vec![
            MirroredCorrespondence {
                companion_mode: None,
                companion_tweak: None,
                kind: MirroredItemKind::Matrix,
                source_id: "source-matrix".into(),
                mirror_id: "mirror-matrix".into(),
            },
            MirroredCorrespondence {
                companion_mode: None,
                companion_tweak: None,
                kind: MirroredItemKind::Part,
                source_id: "matrix/source-matrix/r0c0".into(),
                mirror_id: "matrix/mirror-matrix/r0c0".into(),
            },
            MirroredCorrespondence {
                companion_mode: None,
                companion_tweak: None,
                kind: MirroredItemKind::Part,
                source_id: "matrix/source-matrix/r0c1".into(),
                mirror_id: "matrix/mirror-matrix/r0c1".into(),
            },
        ];
        let halves = serde_json::from_value(serde_json::json!({
            "sourceBoardId": source_board,
            "mirrorBoardId": mirror_board,
            "correspondences": correspondences,
            "exceptions": []
        }))
        .map_err(|error| error.to_string())?;
        document.hardware.as_mut().unwrap().pairs[0].halves = Some(halves);
        Ok(document)
    }

    fn hosted_electrical_fixture() -> Result<(ProjectDoc, Projection), String> {
        let (mut document, projection) = fixed_geometry_fixture()?;
        let source_connector = "matrix/source-matrix/r0c0";
        let mirror_connector = "matrix/mirror-matrix/r0c0";
        let source_member = "embedded/source-circuit/U1";
        let mirror_member = "embedded/mirror-circuit/U1";
        let source_definition = "embedded/source-circuit/definition/u";
        let mirror_definition = "embedded/mirror-circuit/definition/u";
        let source_private_net = "embedded/source-circuit/N$1";
        let mirror_private_net = "embedded/mirror-circuit/N$1";
        let source_joined_net = "host-source-net";
        let mirror_joined_net = "host-mirror-net";

        for (id, reference, board_id) in [
            (source_member, "U1", "source-board"),
            (mirror_member, "U1", "mirror-board"),
        ] {
            let mut part: Part = serde_json::from_value(serde_json::json!({
                "id": id,
                "definitionId": if id == source_member { source_definition } else { mirror_definition },
                "reference": reference,
                "pose": { "at": { "x": 1.0, "y": 2.0 }, "rotation": 0.0 },
                "side": "front",
                "properties": { "fixturePayload": { "preserve": true } },
                "generatorParameters": { "fixtureMode": "literal" }
            }))
            .map_err(|error| error.to_string())?;
            part.pose.at = if id == source_member {
                Vec2 { x: 7.0, y: 9.0 }
            } else {
                Vec2 { x: -7.0, y: 9.0 }
            };
            document.parts.push(part);
            document
                .boards
                .iter_mut()
                .find(|board| board.id == board_id)
                .unwrap()
                .part_ids
                .push(id.into());
        }
        document.definitions.extend([
            serde_json::from_value(serde_json::json!({
                "id": source_definition, "name": "Source U", "kind": "custom",
                "courtyard": [],
                "pads": [{"id":"pad-1","number":"1","at":{"x":0.0,"y":0.0},
                    "size":{"x":1.0,"y":1.0},"shape":"circle"}]
            }))
            .map_err(|error| error.to_string())?,
            serde_json::from_value(serde_json::json!({
                "id": mirror_definition, "name": "Mirror U", "kind": "custom",
                "courtyard": [],
                "pads": [{"id":"pad-1","number":"1","at":{"x":0.0,"y":0.0},
                    "size":{"x":1.0,"y":1.0},"shape":"circle"}]
            }))
            .map_err(|error| error.to_string())?,
        ]);
        document.module_definitions.push(
            serde_json::from_value(serde_json::json!({
                "id": "module-definition",
                "name": "Fixture module",
                "family": "fixture",
                "variant": "source",
                "source": {
                    "repository": "https://example.invalid/fixture",
                    "revision": "fixture-revision",
                    "path": "module.kicad_pcb",
                    "license": "MIT"
                },
                "board": { "contours": [] },
                "mounts": [], "volumes": [], "openings": [], "models": [], "gates": [],
                "interfaces": [],
                "electrical": { "protocol": "pass-through", "requiredSignals": [], "logicVoltage": 3.3 },
                "constituents": []
            }))
            .map_err(|error| error.to_string())?,
        );
        for (id, definition_id, board_id, connector_id, at_x) in [
            (
                "source-module",
                "module-definition",
                "source-board",
                source_connector,
                4.0,
            ),
            (
                "mirror-module",
                "module-definition",
                "mirror-board",
                mirror_connector,
                -4.0,
            ),
        ] {
            document.modules.push(
                serde_json::from_value(serde_json::json!({
                    "id": id,
                    "definitionId": definition_id,
                    "hostBoardId": board_id,
                    "hostFace": "front",
                    "facingFace": "back",
                    "at": { "x": at_x, "y": 3.0 },
                    "rotation": 12.0,
                    "gap": 0.4,
                    "attachment": "board",
                    "connection": {
                        "hostConnectorPartId": connector_id,
                        "modulePortId": "host",
                        "busId": if id == "source-module" { "source-bus" } else { "mirror-bus" },
                        "assignments": {},
                        "cableType": "fixture-cable",
                        "railVoltages": {}
                    }
                }))
                .map_err(|error| error.to_string())?,
            );
        }
        document.embedded_circuits = vec![
            EmbeddedCircuit {
                id: "source-circuit".into(),
                definition_id: "module-definition".into(),
                host_board_id: "source-board".into(),
                part_ids: vec![source_member.into()],
                net_ids: vec![source_private_net.into(), source_joined_net.into()],
                ports: BTreeMap::from([("signal".into(), source_joined_net.into())]),
            },
            EmbeddedCircuit {
                id: "mirror-circuit".into(),
                definition_id: "module-definition".into(),
                host_board_id: "mirror-board".into(),
                part_ids: vec![mirror_member.into()],
                net_ids: vec![mirror_private_net.into(), mirror_joined_net.into()],
                ports: BTreeMap::from([("signal".into(), mirror_joined_net.into())]),
            },
        ];
        for (id, name, pin_id, host_id, board_id) in [
            (
                source_private_net,
                "PRIVATE",
                source_member,
                None,
                "source-board",
            ),
            (
                mirror_private_net,
                "PRIVATE MIRROR",
                mirror_member,
                None,
                "mirror-board",
            ),
            (
                source_joined_net,
                "HOST",
                source_member,
                Some(source_connector),
                "source-board",
            ),
            (
                mirror_joined_net,
                "HOST MIRROR",
                mirror_member,
                Some(mirror_connector),
                "mirror-board",
            ),
        ] {
            let mut pins = vec![Pin {
                part_id: pin_id.into(),
                pad_id: "pad-1".into(),
            }];
            if let Some(host_id) = host_id {
                pins.push(Pin {
                    part_id: host_id.into(),
                    pad_id: "pad-two".into(),
                });
            }
            document.nets.push(Net {
                id: id.into(),
                name: name.into(),
                pins,
            });
            document
                .boards
                .iter_mut()
                .find(|board| board.id == board_id)
                .unwrap()
                .net_ids
                .push(id.into());
        }
        for (board_id, controller_part_id, key_binding) in [
            ("source-board", source_member, "source-only-binding"),
            ("mirror-board", mirror_member, "mirror-local-binding"),
        ] {
            let mut electrical = ElectricalBoardConfiguration {
                board_id: board_id.into(),
                controller_part_id: Some(controller_part_id.into()),
                mode: crate::electrical::ElectricalMode::Direct,
                ..ElectricalBoardConfiguration::default()
            };
            electrical.locks.insert(
                format!("direct/{controller_part_id}"),
                if board_id == "source-board" {
                    "embedded/source-circuit/N$1".into()
                } else {
                    "mirror-local-lock".into()
                },
            );
            electrical.assignments.insert(
                format!("peripheral/{controller_part_id}/signal"),
                if board_id == "source-board" {
                    "host-source-net".into()
                } else {
                    "mirror-local-assignment".into()
                },
            );
            electrical
                .key_bindings
                .insert("local-function".into(), key_binding.into());
            document.hardware.as_mut().unwrap().boards.push(electrical);
        }
        document.board_references.push(BoardReference {
            id: "source-board-reference".into(),
            board_id: "source-board".into(),
            asset_id: "source-only-model".into(),
            enabled: true,
            pose: Pose2 {
                at: Vec2 { x: 3.0, y: 4.0 },
                rotation: 20.0,
            },
            elevation: 1.0,
            model_assets: BTreeMap::new(),
        });
        document
            .boards
            .iter_mut()
            .find(|board| board.id == "source-board")
            .unwrap()
            .traces
            .push(CopperTrace {
                id: "source-only-copper".into(),
                start: Vec2 { x: 1.0, y: 0.0 },
                end: Vec2 { x: 2.0, y: 0.0 },
                width: 0.25,
                layer: Side::Front,
                net_id: Some("host-source-net".into()),
            });

        let halves = document.hardware.as_mut().unwrap().pairs[0]
            .halves
            .as_mut()
            .unwrap();
        halves.correspondences.extend([
            MirroredCorrespondence {
                companion_mode: None,
                companion_tweak: None,
                kind: MirroredItemKind::Part,
                source_id: source_member.into(),
                mirror_id: mirror_member.into(),
            },
            MirroredCorrespondence {
                companion_mode: None,
                companion_tweak: None,
                kind: MirroredItemKind::Module,
                source_id: "source-module".into(),
                mirror_id: "mirror-module".into(),
            },
            MirroredCorrespondence {
                companion_mode: None,
                companion_tweak: None,
                kind: MirroredItemKind::EmbeddedCircuit,
                source_id: "source-circuit".into(),
                mirror_id: "mirror-circuit".into(),
            },
            MirroredCorrespondence {
                companion_mode: None,
                companion_tweak: None,
                kind: MirroredItemKind::ElectricalBoard,
                source_id: "source-board".into(),
                mirror_id: "mirror-board".into(),
            },
            MirroredCorrespondence {
                companion_mode: None,
                companion_tweak: None,
                kind: MirroredItemKind::Net,
                source_id: source_private_net.into(),
                mirror_id: mirror_private_net.into(),
            },
            MirroredCorrespondence {
                companion_mode: None,
                companion_tweak: None,
                kind: MirroredItemKind::Net,
                source_id: source_joined_net.into(),
                mirror_id: mirror_joined_net.into(),
            },
        ]);
        Ok((document, projection))
    }

    #[test]
    fn stage_hosted_correspondence_refuses_conflicts_and_requires_replay_provenance() {
        let (mut document, _) = hosted_electrical_fixture().unwrap();
        let existing = MirroredCorrespondence {
            companion_mode: None,
            companion_tweak: None,
            kind: MirroredItemKind::Module,
            source_id: "source-module".into(),
            mirror_id: "mirror-module".into(),
        };
        let before = document.clone();

        let same_source_conflict = MirroredCorrespondence {
            mirror_id: "mirror-module-other".into(),
            ..existing.clone()
        };
        assert!(
            super::stage_hosted_correspondence(
                &mut document,
                "pair-source-mirror",
                false,
                same_source_conflict,
            )
            .is_err()
        );
        assert_eq!(document, before);

        let same_mirror_conflict = MirroredCorrespondence {
            source_id: "source-module-other".into(),
            ..existing.clone()
        };
        assert!(
            super::stage_hosted_correspondence(
                &mut document,
                "pair-source-mirror",
                false,
                same_mirror_conflict,
            )
            .is_err()
        );
        assert_eq!(document, before);

        assert!(
            super::stage_hosted_correspondence(
                &mut document,
                "pair-source-mirror",
                false,
                existing.clone(),
            )
            .is_err()
        );
        assert_eq!(document, before);
        super::stage_hosted_correspondence(
            &mut document,
            "pair-source-mirror",
            true,
            existing.clone(),
        )
        .expect("private replay provenance admits the exact existing identity");
        assert_eq!(document, before);

        let fresh = MirroredCorrespondence {
            companion_mode: None,
            companion_tweak: None,
            kind: MirroredItemKind::Module,
            source_id: "source-module-added".into(),
            mirror_id: "mirror-module-added".into(),
        };
        super::stage_hosted_correspondence(
            &mut document,
            "pair-source-mirror",
            false,
            fresh.clone(),
        )
        .expect("a fresh selected identity is staged without replay provenance");
        let halves = document.hardware.as_ref().unwrap().pairs[0]
            .halves
            .as_ref()
            .unwrap();
        assert_eq!(
            halves
                .correspondences
                .iter()
                .filter(|association| **association == fresh)
                .count(),
            1
        );
        assert_eq!(
            halves.correspondences.len(),
            before.hardware.as_ref().unwrap().pairs[0]
                .halves
                .as_ref()
                .unwrap()
                .correspondences
                .len()
                + 1
        );
    }

    #[test]
    fn hosted_add_module_admits_its_pair_record_and_replays_idempotently() {
        let (mut document, mut projection) = hosted_electrical_fixture().unwrap();
        document
            .modules
            .retain(|module| module.id != "mirror-module");
        document.hardware.as_mut().unwrap().pairs[0]
            .halves
            .as_mut()
            .unwrap()
            .correspondences
            .retain(|association| {
                !(association.kind == MirroredItemKind::Module
                    && association.source_id == "source-module")
            });
        projection.hosted_transitions = vec![HostedTransition::AddModule {
            source_id: "source-module".into(),
            mirror_id: "mirror-module-added".into(),
        }];
        let pair_before = pair_state_bytes(&document);

        reconcile(&mut document, "pair-source-mirror", &projection).unwrap();
        let halves = document.hardware.as_ref().unwrap().pairs[0]
            .halves
            .as_ref()
            .unwrap();
        assert_eq!(
            halves
                .correspondences
                .iter()
                .filter(|association| {
                    association.kind == MirroredItemKind::Module
                        && association.source_id == "source-module"
                        && association.mirror_id == "mirror-module-added"
                })
                .count(),
            1,
            "the selected hosted module identity becomes a durable pair record"
        );
        let first_document = document.clone();
        let first_context = projection.applied_hosted_operations.borrow().clone();
        let first_circuit_context = projection.applied_hosted_circuit_additions.borrow().clone();
        let repeated = reconcile(&mut document, "pair-source-mirror", &projection);
        assert!(
            repeated.is_ok(),
            "the same hosted addition must accept its own full output"
        );
        assert!(repeated.unwrap().is_empty());
        assert_eq!(document, first_document);
        assert_eq!(
            *projection.applied_hosted_operations.borrow(),
            first_context
        );
        assert_eq!(
            *projection.applied_hosted_circuit_additions.borrow(),
            first_circuit_context
        );
        assert_eq!(
            first_context,
            BTreeSet::from([(
                "module".into(),
                "source-module".into(),
                "mirror-module-added".into()
            )])
        );
        assert_ne!(pair_state_bytes(&document), pair_before);

        let (mut stale, mut stale_projection) = hosted_electrical_fixture().unwrap();
        stale_projection.hosted_transitions = vec![HostedTransition::AddModule {
            source_id: "source-module".into(),
            mirror_id: "mirror-module".into(),
        }];
        let stale_before = stale.clone();
        assert!(reconcile(&mut stale, "pair-source-mirror", &stale_projection).is_err());
        assert_eq!(stale, stale_before);
    }

    #[test]
    fn hosted_add_circuit_admits_selected_pair_records_and_replays_idempotently() {
        let (mut document, mut projection) = hosted_electrical_fixture().unwrap();
        document
            .embedded_circuits
            .retain(|circuit| circuit.id != "mirror-circuit");
        document
            .parts
            .retain(|part| part.id != "embedded/mirror-circuit/U1");
        document
            .nets
            .iter_mut()
            .find(|net| net.id == "host-mirror-net")
            .unwrap()
            .pins
            .retain(|pin| pin.part_id != "embedded/mirror-circuit/U1");
        document
            .definitions
            .retain(|definition| definition.id != "embedded/mirror-circuit/definition/u");
        document
            .nets
            .retain(|net| net.id != "embedded/mirror-circuit/N$1");
        let mirror_board = document
            .boards
            .iter_mut()
            .find(|board| board.id == "mirror-board")
            .unwrap();
        mirror_board
            .part_ids
            .retain(|id| id != "embedded/mirror-circuit/U1");
        mirror_board
            .net_ids
            .retain(|id| id != "embedded/mirror-circuit/N$1");
        document.hardware.as_mut().unwrap().pairs[0]
            .halves
            .as_mut()
            .unwrap()
            .correspondences
            .retain(|association| !match association.kind {
                MirroredItemKind::Part => association.source_id == "embedded/source-circuit/U1",
                MirroredItemKind::EmbeddedCircuit => association.source_id == "source-circuit",
                MirroredItemKind::Net => association.source_id == "embedded/source-circuit/N$1",
                _ => false,
            });
        projection.hosted_transitions = vec![HostedTransition::AddCircuit {
            source_id: "source-circuit".into(),
            mirror_id: "mirror-circuit-added".into(),
            member_ids: BTreeMap::from([(
                "embedded/source-circuit/U1".into(),
                "embedded/mirror-circuit-added/U1".into(),
            )]),
            member_references: BTreeMap::from([("embedded/source-circuit/U1".into(), "U9".into())]),
            definition_ids: BTreeMap::from([(
                "embedded/source-circuit/definition/u".into(),
                "embedded/mirror-circuit-added/definition/u".into(),
            )]),
            net_ids: BTreeMap::from([
                (
                    "embedded/source-circuit/N$1".into(),
                    "embedded/mirror-circuit-added/N$1".into(),
                ),
                ("host-source-net".into(), "host-mirror-net".into()),
            ]),
            member_poses: BTreeMap::new(),
        }];
        let pair_before = pair_state_bytes(&document);

        reconcile(&mut document, "pair-source-mirror", &projection).unwrap();
        let halves = document.hardware.as_ref().unwrap().pairs[0]
            .halves
            .as_ref()
            .unwrap();
        for (kind, source_id, mirror_id) in [
            (
                MirroredItemKind::EmbeddedCircuit,
                "source-circuit",
                "mirror-circuit-added",
            ),
            (
                MirroredItemKind::Part,
                "embedded/source-circuit/U1",
                "embedded/mirror-circuit-added/U1",
            ),
            (
                MirroredItemKind::Net,
                "embedded/source-circuit/N$1",
                "embedded/mirror-circuit-added/N$1",
            ),
            (MirroredItemKind::Net, "host-source-net", "host-mirror-net"),
        ] {
            assert_eq!(
                halves
                    .correspondences
                    .iter()
                    .filter(|association| {
                        association.kind == kind
                            && association.source_id == source_id
                            && association.mirror_id == mirror_id
                    })
                    .count(),
                1,
                "selected hosted circuit identities become durable exactly once: {kind:?} {source_id:?} -> {mirror_id:?}"
            );
        }
        assert_eq!(
            document
                .parts
                .iter()
                .find(|part| part.id == "embedded/mirror-circuit-added/U1")
                .unwrap()
                .definition_id,
            "embedded/mirror-circuit-added/definition/u",
            "member definition identity stays in the existing namespace map"
        );
        let first_document = document.clone();
        let first_joined_net = document
            .nets
            .iter()
            .find(|net| net.id == "host-mirror-net")
            .unwrap()
            .clone();
        let first_context = projection.applied_hosted_operations.borrow().clone();
        let first_circuit_context = projection.applied_hosted_circuit_additions.borrow().clone();
        let repeated = reconcile(&mut document, "pair-source-mirror", &projection);
        assert!(
            repeated.is_ok(),
            "the same hosted circuit addition must accept its own full output"
        );
        let repeated_changes = repeated.unwrap();
        let repeated_joined_net = document
            .nets
            .iter()
            .find(|net| net.id == "host-mirror-net")
            .unwrap();
        assert!(
            repeated_changes.is_empty(),
            "authenticated AddCircuit replay must be idempotent: {repeated_changes:?}; before={first_joined_net:?}, after={repeated_joined_net:?}"
        );
        assert_eq!(document, first_document);
        assert_eq!(
            *projection.applied_hosted_operations.borrow(),
            first_context
        );
        assert_eq!(
            *projection.applied_hosted_circuit_additions.borrow(),
            first_circuit_context
        );
        assert_eq!(
            first_context,
            BTreeSet::from([(
                "circuit".into(),
                "source-circuit".into(),
                "mirror-circuit-added".into()
            )])
        );
        assert_ne!(pair_state_bytes(&document), pair_before);

        // The first AddCircuit path appends the member pin to the existing joined host net.
        // Once the addition is admitted, ordinary circuit projection owns subsequent updates and
        // applies its established source-net metadata normalization.
        let first_added_joined_name = first_joined_net.name.clone();
        projection.hosted_transitions.clear();
        projection.applied_hosted_operations.borrow_mut().clear();
        projection
            .applied_hosted_circuit_additions
            .borrow_mut()
            .clear();
        let source_member = document
            .parts
            .iter_mut()
            .find(|part| part.id == "embedded/source-circuit/U1")
            .unwrap();
        source_member.pose.at.x += 2.0;
        document
            .parts
            .iter_mut()
            .find(|part| part.id == "embedded/mirror-circuit-added/U1")
            .unwrap()
            .reference = "MIRROR-LOCAL9".into();
        let after_addition_edit = reconcile(&mut document, "pair-source-mirror", &projection)
            .expect("the admitted member maps participate in ordinary projection");
        assert!(after_addition_edit.contains(&"embedded/mirror-circuit-added/U1".to_owned()));
        let projected_member = document
            .parts
            .iter()
            .find(|part| part.id == "embedded/mirror-circuit-added/U1")
            .unwrap();
        assert_ne!(
            projected_member.pose,
            first_document
                .parts
                .iter()
                .find(|part| part.id == "embedded/mirror-circuit-added/U1")
                .unwrap()
                .pose
        );
        assert_eq!(projected_member.reference, "MIRROR-LOCAL9");
        let ordinary_joined_name = document
            .nets
            .iter()
            .find(|net| net.id == "host-mirror-net")
            .unwrap()
            .name
            .clone();
        assert_eq!(ordinary_joined_name, "HOST");
        assert_ne!(ordinary_joined_name, first_added_joined_name);

        let normalized_document = document.clone();
        let normalized_repeat = reconcile(&mut document, "pair-source-mirror", &projection)
            .expect("the ordinary circuit projection remains idempotent after normalization");
        assert!(normalized_repeat.is_empty());
        assert_eq!(document, normalized_document);

        let serialized = serde_json::to_vec(&document).unwrap();
        let mut reopened: ProjectDoc = serde_json::from_slice(&serialized).unwrap();
        let reopened_repeat = reconcile(&mut reopened, "pair-source-mirror", &projection)
            .expect("durable hosted correspondences survive save/open");
        assert!(reopened_repeat.is_empty());
        assert_eq!(reopened, normalized_document);

        let mut removal_input = reopened.clone();
        removal_input
            .constraints
            .retain(|constraint| constraint.id() != "retained-second-key-offset");
        let (removed, _) = super::prepare::prepare_transaction(
            &removal_input,
            &[],
            &crate::model::EditOperation::RemoveEmbeddedCircuit {
                id: "source-circuit".into(),
            },
        )
        .expect("the supported source transaction removes an admitted circuit");
        let removed_halves = removed.hardware.as_ref().unwrap().pairs[0]
            .halves
            .as_ref()
            .unwrap();
        assert!(!removed_halves.correspondences.iter().any(|association| {
            matches!(
                association.kind,
                MirroredItemKind::EmbeddedCircuit | MirroredItemKind::Part | MirroredItemKind::Net
            ) && (association.source_id == "source-circuit"
                || association
                    .source_id
                    .starts_with("embedded/source-circuit/"))
                && (association.mirror_id == "mirror-circuit-added"
                    || association
                        .mirror_id
                        .starts_with("embedded/mirror-circuit-added/"))
        }));
        assert!(removed_halves.correspondences.iter().any(|association| {
            association.kind == MirroredItemKind::Net
                && association.source_id == "host-source-net"
                && association.mirror_id == "host-mirror-net"
        }));
        assert!(
            !removed
                .embedded_circuits
                .iter()
                .any(|circuit| circuit.id == "mirror-circuit-added")
        );
        assert!(
            !removed
                .parts
                .iter()
                .any(|part| part.id == "embedded/mirror-circuit-added/U1")
        );
        if let Some(retained_private_net) = removed
            .nets
            .iter()
            .find(|net| net.id == "embedded/mirror-circuit-added/N$1")
        {
            assert!(retained_private_net.pins.is_empty());
        }

        let (mut stale, mut stale_projection) = hosted_electrical_fixture().unwrap();
        stale_projection.hosted_transitions = vec![HostedTransition::AddCircuit {
            source_id: "source-circuit".into(),
            mirror_id: "mirror-circuit".into(),
            member_ids: BTreeMap::from([(
                "embedded/source-circuit/U1".into(),
                "embedded/mirror-circuit/U1".into(),
            )]),
            member_references: BTreeMap::from([("embedded/source-circuit/U1".into(), "U1".into())]),
            definition_ids: BTreeMap::from([(
                "embedded/source-circuit/definition/u".into(),
                "embedded/mirror-circuit/definition/u".into(),
            )]),
            net_ids: BTreeMap::from([
                (
                    "embedded/source-circuit/N$1".into(),
                    "embedded/mirror-circuit/N$1".into(),
                ),
                ("host-source-net".into(), "host-mirror-net".into()),
            ]),
            member_poses: BTreeMap::new(),
        }];
        let stale_before = stale.clone();
        assert!(reconcile(&mut stale, "pair-source-mirror", &stale_projection).is_err());
        assert_eq!(stale, stale_before);
    }

    #[test]
    fn hosted_add_circuit_replay_binds_all_selected_identity_maps() {
        fn prepared_add() -> (ProjectDoc, Projection) {
            let (mut document, mut projection) = hosted_electrical_fixture().unwrap();
            let mut unrelated_definition = document
                .definitions
                .iter()
                .find(|definition| definition.id == "embedded/mirror-circuit/definition/u")
                .unwrap()
                .clone();
            unrelated_definition.id = "unrelated-definition".into();
            unrelated_definition.name = "Preserve this unrelated definition".into();
            let mut unrelated_member = document
                .parts
                .iter()
                .find(|part| part.id == "embedded/mirror-circuit/U1")
                .unwrap()
                .clone();
            unrelated_member.id = "unrelated/member".into();
            unrelated_member.reference = "U42".into();
            unrelated_member.definition_id = unrelated_definition.id.clone();
            document.definitions.push(unrelated_definition);
            document.parts.push(unrelated_member.clone());
            document.nets.push(Net {
                id: "unrelated-net".into(),
                name: "Preserve this unrelated net".into(),
                pins: vec![Pin {
                    part_id: unrelated_member.id.clone(),
                    pad_id: "pad-1".into(),
                }],
            });
            let mirror_board = document
                .boards
                .iter_mut()
                .find(|board| board.id == "mirror-board")
                .unwrap();
            mirror_board.part_ids.push(unrelated_member.id.clone());
            mirror_board.net_ids.push("unrelated-net".into());

            document
                .embedded_circuits
                .retain(|circuit| circuit.id != "mirror-circuit");
            document
                .parts
                .retain(|part| part.id != "embedded/mirror-circuit/U1");
            document
                .definitions
                .retain(|definition| definition.id != "embedded/mirror-circuit/definition/u");
            document
                .nets
                .retain(|net| net.id != "embedded/mirror-circuit/N$1");
            let mirror_board = document
                .boards
                .iter_mut()
                .find(|board| board.id == "mirror-board")
                .unwrap();
            mirror_board
                .part_ids
                .retain(|id| id != "embedded/mirror-circuit/U1");
            mirror_board
                .net_ids
                .retain(|id| id != "embedded/mirror-circuit/N$1");
            document.hardware.as_mut().unwrap().pairs[0]
                .halves
                .as_mut()
                .unwrap()
                .correspondences
                .retain(|association| !match association.kind {
                    MirroredItemKind::Part => association.source_id == "embedded/source-circuit/U1",
                    MirroredItemKind::EmbeddedCircuit => association.source_id == "source-circuit",
                    MirroredItemKind::Net => association.source_id == "embedded/source-circuit/N$1",
                    _ => false,
                });
            projection.hosted_transitions = vec![HostedTransition::AddCircuit {
                source_id: "source-circuit".into(),
                mirror_id: "mirror-circuit-added".into(),
                member_ids: BTreeMap::from([(
                    "embedded/source-circuit/U1".into(),
                    "embedded/mirror-circuit-added/U1".into(),
                )]),
                member_references: BTreeMap::from([(
                    "embedded/source-circuit/U1".into(),
                    "U9".into(),
                )]),
                definition_ids: BTreeMap::from([(
                    "embedded/source-circuit/definition/u".into(),
                    "embedded/mirror-circuit-added/definition/u".into(),
                )]),
                net_ids: BTreeMap::from([
                    (
                        "embedded/source-circuit/N$1".into(),
                        "embedded/mirror-circuit-added/N$1".into(),
                    ),
                    ("host-source-net".into(), "host-mirror-net".into()),
                ]),
                member_poses: BTreeMap::new(),
            }];
            (document, projection)
        }

        fn replay_map_is_bound(mutate: fn(&mut HostedTransition), label: &str) -> bool {
            let (mut document, mut projection) = prepared_add();
            crate::mechanical::halves::validate_document(&document)
                .expect("the unpaired preservation controls leave a valid pair baseline");
            reconcile(&mut document, "pair-source-mirror", &projection)
                .expect("the selected AddCircuit operation creates its first output");
            let first_document = document.clone();
            let first_context = projection.applied_hosted_operations.borrow().clone();
            let first_circuit_context =
                projection.applied_hosted_circuit_additions.borrow().clone();
            let pair_before = pair_state_bytes(&document);
            mutate(&mut projection.hosted_transitions[0]);

            let result = reconcile(&mut document, "pair-source-mirror", &projection);
            if result.is_ok() {
                return false;
            }
            assert_eq!(
                document, first_document,
                "changed {label} must refuse atomically"
            );
            assert_eq!(
                *projection.applied_hosted_operations.borrow(),
                first_context,
                "a refused replay cannot alter its full private context"
            );
            assert_eq!(
                *projection.applied_hosted_circuit_additions.borrow(),
                first_circuit_context,
                "a refused replay cannot alter bound AddCircuit provenance"
            );
            assert_eq!(pair_state_bytes(&document), pair_before);
            assert!(document.definitions.iter().any(|definition| {
                definition.id == "unrelated-definition"
                    && definition.name == "Preserve this unrelated definition"
            }));
            assert!(
                document
                    .parts
                    .iter()
                    .any(|part| { part.id == "unrelated/member" && part.reference == "U42" })
            );
            assert!(document.nets.iter().any(|net| {
                net.id == "unrelated-net"
                    && net.name == "Preserve this unrelated net"
                    && net.pins.iter().any(|pin| pin.part_id == "unrelated/member")
            }));
            true
        }

        let member_target = replay_map_is_bound(
            |transition| {
                let HostedTransition::AddCircuit { member_ids, .. } = transition else {
                    unreachable!()
                };
                member_ids.insert(
                    "embedded/source-circuit/U1".into(),
                    "unrelated/member".into(),
                );
            },
            "member target",
        );
        let member_reference = replay_map_is_bound(
            |transition| {
                let HostedTransition::AddCircuit {
                    member_references, ..
                } = transition
                else {
                    unreachable!()
                };
                member_references.insert("embedded/source-circuit/U1".into(), "U10".into());
            },
            "member reference",
        );
        let definition_target = replay_map_is_bound(
            |transition| {
                let HostedTransition::AddCircuit { definition_ids, .. } = transition else {
                    unreachable!()
                };
                definition_ids.insert(
                    "embedded/source-circuit/definition/u".into(),
                    "unrelated-definition".into(),
                );
            },
            "definition target",
        );
        let net_target = replay_map_is_bound(
            |transition| {
                let HostedTransition::AddCircuit { net_ids, .. } = transition else {
                    unreachable!()
                };
                net_ids.insert("embedded/source-circuit/N$1".into(), "unrelated-net".into());
            },
            "net target",
        );
        assert!(
            member_target && member_reference && definition_target && net_target,
            "changed AddCircuit maps must not authenticate replay; accepted: {:?}",
            [
                (!member_target).then_some("member target"),
                (!member_reference).then_some("member reference"),
                (!definition_target).then_some("definition target"),
                (!net_target).then_some("net target"),
            ]
            .into_iter()
            .flatten()
            .collect::<Vec<_>>()
        );
    }

    #[test]
    fn hosted_module_maps_instance_connector_upstream_and_bus_references() {
        let (mut document, projection) = hosted_electrical_fixture().unwrap();
        let source_module = document
            .modules
            .iter_mut()
            .find(|module| module.id == "source-module")
            .unwrap();
        source_module.host_instance_id = Some("left".into());
        source_module
            .connection
            .as_mut()
            .unwrap()
            .upstream_module_id = Some("source-upstream".into());
        let mut source_upstream = source_module.clone();
        source_upstream.id = "source-upstream".into();
        source_upstream.host_instance_id = Some("left".into());
        source_upstream
            .connection
            .as_mut()
            .unwrap()
            .upstream_module_id = None;
        let mut mirror_upstream = source_upstream.clone();
        mirror_upstream.id = "mirror-upstream".into();
        mirror_upstream.host_board_id = "mirror-board".into();
        mirror_upstream.host_instance_id = Some("right".into());
        mirror_upstream
            .connection
            .as_mut()
            .unwrap()
            .host_connector_part_id = "matrix/mirror-matrix/r0c0".into();
        mirror_upstream.connection.as_mut().unwrap().bus_id = "mirror-upstream-bus".into();
        mirror_upstream.at = Vec2 { x: 0.0, y: 0.0 };
        document.modules.push(source_upstream);
        document.modules.push(mirror_upstream);
        document.hardware.as_mut().unwrap().pairs[0]
            .halves
            .as_mut()
            .unwrap()
            .correspondences
            .push(MirroredCorrespondence {
                companion_mode: None,
                companion_tweak: None,
                kind: MirroredItemKind::Module,
                source_id: "source-upstream".into(),
                mirror_id: "mirror-upstream".into(),
            });

        reconcile(&mut document, "pair-source-mirror", &projection).unwrap();
        let mirror = document
            .modules
            .iter()
            .find(|module| module.id == "mirror-module")
            .unwrap();
        assert_eq!(mirror.host_instance_id.as_deref(), Some("right"));
        let connection = mirror.connection.as_ref().unwrap();
        assert_eq!(
            connection.host_connector_part_id,
            "matrix/mirror-matrix/r0c0"
        );
        assert_eq!(
            connection.upstream_module_id.as_deref(),
            Some("mirror-upstream")
        );
        assert_eq!(connection.bus_id, "mirror-board/source-bus");
    }

    #[test]
    fn hosted_module_refuses_unmapped_nonempty_references_atomically() {
        let (mut document, projection) = hosted_electrical_fixture().unwrap();
        let mut source_connector = document
            .parts
            .iter()
            .find(|part| part.id == "matrix/source-matrix/r0c0")
            .unwrap()
            .clone();
        source_connector.id = "source-unpaired-connector".into();
        document.parts.push(source_connector);
        document
            .boards
            .iter_mut()
            .find(|board| board.id == "source-board")
            .unwrap()
            .part_ids
            .push("source-unpaired-connector".into());
        document
            .modules
            .iter_mut()
            .find(|module| module.id == "source-module")
            .unwrap()
            .connection
            .as_mut()
            .unwrap()
            .host_connector_part_id = "source-unpaired-connector".into();
        let before = document.clone();
        assert!(reconcile(&mut document, "pair-source-mirror", &projection).is_err());
        assert_eq!(document, before);
    }

    #[test]
    fn hosted_circuit_definition_payload_changes_project_to_the_existing_definition() {
        let (mut document, projection) = hosted_electrical_fixture().unwrap();
        let source_definition = document
            .definitions
            .iter_mut()
            .find(|definition| definition.id == "embedded/source-circuit/definition/u")
            .unwrap();
        source_definition.name = "Updated Source U".into();
        source_definition.pads[0].size = Vec2 { x: 2.5, y: 3.5 };
        source_definition.pads[0].shape = PadShape::Roundrect;
        let changed = reconcile(&mut document, "pair-source-mirror", &projection).unwrap();
        let mirror = document
            .definitions
            .iter()
            .find(|definition| definition.id == "embedded/mirror-circuit/definition/u")
            .unwrap();
        assert_eq!(mirror.name, "Updated Source U");
        assert_eq!(mirror.pads[0].size, Vec2 { x: 2.5, y: 3.5 });
        assert!(changed.contains(&"embedded/mirror-circuit/definition/u".to_owned()));
    }

    #[test]
    fn hosted_joined_net_update_preserves_target_only_host_pins() {
        let (mut document, projection) = hosted_electrical_fixture().unwrap();
        let mut host_part = document
            .parts
            .iter()
            .find(|part| part.id == "matrix/mirror-matrix/r0c1")
            .unwrap()
            .clone();
        host_part.id = "mirror-host-only".into();
        document.parts.push(host_part);
        document
            .boards
            .iter_mut()
            .find(|board| board.id == "mirror-board")
            .unwrap()
            .part_ids
            .push("mirror-host-only".into());
        let host_net = document
            .nets
            .iter_mut()
            .find(|net| net.id == "host-mirror-net")
            .unwrap();
        host_net.pins.push(Pin {
            part_id: "mirror-host-only".into(),
            pad_id: "authored-host-pad".into(),
        });
        let source_net = document
            .nets
            .iter_mut()
            .find(|net| net.id == "host-source-net")
            .unwrap();
        source_net.name = "Edited host net".into();
        source_net.pins.push(Pin {
            part_id: "embedded/source-circuit/U1".into(),
            pad_id: "pad-two".into(),
        });

        reconcile(&mut document, "pair-source-mirror", &projection).unwrap();
        let target = document
            .nets
            .iter()
            .find(|net| net.id == "host-mirror-net")
            .unwrap();
        assert!(target.pins.contains(&Pin {
            part_id: "mirror-host-only".into(),
            pad_id: "authored-host-pad".into()
        }));
        assert!(target.pins.contains(&Pin {
            part_id: "embedded/mirror-circuit/U1".into(),
            pad_id: "pad-two".into()
        }));
        assert_eq!(target.name, "Edited host net");
    }

    #[test]
    fn hosted_circuit_local_member_and_net_exceptions_preserve_authored_payload() {
        let (mut document, projection) = hosted_electrical_fixture().unwrap();
        let halves = document.hardware.as_mut().unwrap().pairs[0]
            .halves
            .as_mut()
            .unwrap();
        halves.correspondences.retain(|association| {
            !((association.kind == MirroredItemKind::Part
                && association.mirror_id == "embedded/mirror-circuit/U1")
                || (association.kind == MirroredItemKind::Net
                    && association.mirror_id == "embedded/mirror-circuit/N$1"))
        });
        halves.exceptions.extend([
            MirroredException {
                companion_mode: None,
                companion_tweak: None,
                kind: MirroredItemKind::Part,
                removed_copper: None,
                mirror_id: "embedded/mirror-circuit/U1".into(),
                state: MirroredExceptionState::Local,
            },
            MirroredException {
                companion_mode: None,
                companion_tweak: None,
                kind: MirroredItemKind::Net,
                removed_copper: None,
                mirror_id: "embedded/mirror-circuit/N$1".into(),
                state: MirroredExceptionState::Local,
            },
        ]);
        let local_part = document
            .parts
            .iter_mut()
            .find(|part| part.id == "embedded/mirror-circuit/U1")
            .unwrap();
        local_part.reference = "LOCAL9".into();
        local_part
            .properties
            .as_mut()
            .unwrap()
            .insert("authoredLocal".into(), serde_json::json!({"v": 7}));
        document
            .nets
            .iter_mut()
            .find(|net| net.id == "embedded/mirror-circuit/N$1")
            .unwrap()
            .name = "Authored local net".into();
        document
            .parts
            .iter_mut()
            .find(|part| part.id == "embedded/source-circuit/U1")
            .unwrap()
            .reference = "U2".into();
        document
            .modules
            .iter_mut()
            .find(|module| module.id == "source-module")
            .unwrap()
            .at = Vec2 { x: 17.0, y: -8.0 };

        reconcile(&mut document, "pair-source-mirror", &projection).unwrap();
        let member = document
            .parts
            .iter()
            .find(|part| part.id == "embedded/mirror-circuit/U1")
            .unwrap();
        assert_eq!(member.reference, "LOCAL9");
        assert_eq!(
            member.properties.as_ref().unwrap()["authoredLocal"],
            serde_json::json!({"v": 7})
        );
        assert_eq!(
            document
                .nets
                .iter()
                .find(|net| net.id == "embedded/mirror-circuit/N$1")
                .unwrap()
                .name,
            "Authored local net"
        );
        let mirror_circuit = document
            .embedded_circuits
            .iter()
            .find(|circuit| circuit.id == "mirror-circuit")
            .unwrap();
        assert!(
            mirror_circuit
                .part_ids
                .contains(&"embedded/mirror-circuit/U1".to_owned())
        );
        assert!(
            mirror_circuit
                .net_ids
                .contains(&"embedded/mirror-circuit/N$1".to_owned())
        );
        assert!(
            (document
                .modules
                .iter()
                .find(|module| module.id == "mirror-module")
                .unwrap()
                .at
                .x
                - 108.0)
                .abs()
                < 0.02
        );
    }

    #[test]
    fn hosted_local_circuit_exception_preserves_authored_circuit_while_linked_neighbors_update() {
        let (mut document, projection) = hosted_electrical_fixture().unwrap();
        let halves = document.hardware.as_mut().unwrap().pairs[0]
            .halves
            .as_mut()
            .unwrap();
        halves.correspondences.retain(|association| {
            !(association.kind == MirroredItemKind::EmbeddedCircuit
                && association.mirror_id == "mirror-circuit")
        });
        halves.exceptions.push(MirroredException {
            companion_mode: None,
            companion_tweak: None,
            kind: MirroredItemKind::EmbeddedCircuit,
            removed_copper: None,
            mirror_id: "mirror-circuit".into(),
            state: MirroredExceptionState::Local,
        });
        document
            .embedded_circuits
            .iter_mut()
            .find(|circuit| circuit.id == "mirror-circuit")
            .unwrap()
            .ports
            .insert("local-authored".into(), "host-mirror-net".into());
        document
            .modules
            .iter_mut()
            .find(|module| module.id == "source-module")
            .unwrap()
            .at = Vec2 { x: 17.0, y: -8.0 };
        let pair_before = pair_state_bytes(&document);

        reconcile(&mut document, "pair-source-mirror", &projection).unwrap();
        let circuit = document
            .embedded_circuits
            .iter()
            .find(|circuit| circuit.id == "mirror-circuit")
            .unwrap();
        assert_eq!(
            circuit.ports.get("local-authored").map(String::as_str),
            Some("host-mirror-net")
        );
        assert!(
            (document
                .modules
                .iter()
                .find(|module| module.id == "mirror-module")
                .unwrap()
                .at
                .x
                - 108.0)
                .abs()
                < 0.02
        );
        assert_eq!(pair_state_bytes(&document), pair_before);
    }

    #[test]
    fn hosted_omitted_member_and_net_keep_their_unlinked_payload_under_linked_circuit() {
        let (mut document, projection) = hosted_electrical_fixture().unwrap();
        let halves = document.hardware.as_mut().unwrap().pairs[0]
            .halves
            .as_mut()
            .unwrap();
        halves.correspondences.retain(|association| {
            !((association.kind == MirroredItemKind::Part
                && association.mirror_id == "embedded/mirror-circuit/U1")
                || (association.kind == MirroredItemKind::Net
                    && association.mirror_id == "embedded/mirror-circuit/N$1"))
        });
        halves.exceptions.extend([
            MirroredException {
                companion_mode: None,
                companion_tweak: None,
                kind: MirroredItemKind::Part,
                removed_copper: None,
                mirror_id: "embedded/mirror-circuit/U1".into(),
                state: MirroredExceptionState::Omitted,
            },
            MirroredException {
                companion_mode: None,
                companion_tweak: None,
                kind: MirroredItemKind::Net,
                removed_copper: None,
                mirror_id: "embedded/mirror-circuit/N$1".into(),
                state: MirroredExceptionState::Omitted,
            },
        ]);
        document
            .parts
            .iter_mut()
            .find(|part| part.id == "embedded/mirror-circuit/U1")
            .unwrap()
            .reference = "OMITTED-U".into();
        document
            .nets
            .iter_mut()
            .find(|net| net.id == "embedded/mirror-circuit/N$1")
            .unwrap()
            .name = "Omitted private net".into();
        document
            .parts
            .iter_mut()
            .find(|part| part.id == "embedded/source-circuit/U1")
            .unwrap()
            .reference = "U2".into();
        document
            .modules
            .iter_mut()
            .find(|module| module.id == "source-module")
            .unwrap()
            .at = Vec2 { x: 17.0, y: -8.0 };
        let pair_before = pair_state_bytes(&document);

        reconcile(&mut document, "pair-source-mirror", &projection).unwrap();
        assert_eq!(
            document
                .parts
                .iter()
                .find(|part| part.id == "embedded/mirror-circuit/U1")
                .unwrap()
                .reference,
            "OMITTED-U"
        );
        assert_eq!(
            document
                .nets
                .iter()
                .find(|net| net.id == "embedded/mirror-circuit/N$1")
                .unwrap()
                .name,
            "Omitted private net"
        );
        assert!(
            document
                .embedded_circuits
                .iter()
                .any(|circuit| circuit.id == "mirror-circuit")
        );
        let mirror_circuit = document
            .embedded_circuits
            .iter()
            .find(|circuit| circuit.id == "mirror-circuit")
            .unwrap();
        assert!(
            mirror_circuit
                .part_ids
                .contains(&"embedded/mirror-circuit/U1".to_owned())
        );
        assert!(
            mirror_circuit
                .net_ids
                .contains(&"embedded/mirror-circuit/N$1".to_owned())
        );
        assert!(
            (document
                .modules
                .iter()
                .find(|module| module.id == "mirror-module")
                .unwrap()
                .at
                .x
                - 108.0)
                .abs()
                < 0.02
        );
        assert_eq!(pair_state_bytes(&document), pair_before);
    }

    #[test]
    fn hosted_omitted_member_and_net_can_be_absent_under_linked_circuit() {
        let (mut document, projection) = hosted_electrical_fixture().unwrap();
        reconcile(&mut document, "pair-source-mirror", &projection)
            .expect("the historical linked owner baseline is fully materialized first");
        let halves = document.hardware.as_mut().unwrap().pairs[0]
            .halves
            .as_mut()
            .unwrap();
        halves.correspondences.retain(|association| {
            !((association.kind == MirroredItemKind::Part
                && association.mirror_id == "embedded/mirror-circuit/U1")
                || (association.kind == MirroredItemKind::Net
                    && association.mirror_id == "embedded/mirror-circuit/N$1"))
        });
        halves.exceptions.extend([
            MirroredException {
                companion_mode: None,
                companion_tweak: None,
                kind: MirroredItemKind::Part,
                removed_copper: None,
                mirror_id: "embedded/mirror-circuit/U1".into(),
                state: MirroredExceptionState::Omitted,
            },
            MirroredException {
                companion_mode: None,
                companion_tweak: None,
                kind: MirroredItemKind::Net,
                removed_copper: None,
                mirror_id: "embedded/mirror-circuit/N$1".into(),
                state: MirroredExceptionState::Omitted,
            },
        ]);

        // The durable Omitted ownership facts are supplied before this target-side
        // materialization snapshot; the source members and their linked circuit stay
        // present, and the kernel must leave the pair record byte-identical.
        let pre_materialization = document.clone();
        crate::mechanical::halves::validate_document(&pre_materialization)
            .expect("the durable Omitted owner baseline has real target records");
        document
            .parts
            .retain(|part| part.id != "embedded/mirror-circuit/U1");
        document
            .nets
            .retain(|net| net.id != "embedded/mirror-circuit/N$1");
        let mirror_circuit = document
            .embedded_circuits
            .iter_mut()
            .find(|circuit| circuit.id == "mirror-circuit")
            .unwrap();
        mirror_circuit
            .part_ids
            .retain(|id| id != "embedded/mirror-circuit/U1");
        mirror_circuit
            .net_ids
            .retain(|id| id != "embedded/mirror-circuit/N$1");
        let mirror_board = document
            .boards
            .iter_mut()
            .find(|board| board.id == "mirror-board")
            .unwrap();
        mirror_board
            .part_ids
            .retain(|id| id != "embedded/mirror-circuit/U1");
        mirror_board
            .net_ids
            .retain(|id| id != "embedded/mirror-circuit/N$1");
        document
            .nets
            .iter_mut()
            .find(|net| net.id == "host-mirror-net")
            .unwrap()
            .pins
            .retain(|pin| pin.part_id != "embedded/mirror-circuit/U1");
        let omission_key = (
            "pair-source-mirror".into(),
            "source-circuit".into(),
            "mirror-circuit".into(),
        );
        let historical_pair_bytes = pair_state_bytes(&pre_materialization);
        projection
            .hosted_omission_baselines
            .borrow_mut()
            .insert(omission_key.clone(), pre_materialization.clone());
        let pair_before = pair_state_bytes(&document);
        assert_eq!(pair_before, historical_pair_bytes);
        let applied_before = projection.applied_hosted_operations.borrow().clone();
        let removal_context_before = projection.hosted_removal_baselines.borrow().clone();
        let omission_context_before = projection.hosted_omission_baselines.borrow().clone();
        document
            .modules
            .iter_mut()
            .find(|module| module.id == "source-module")
            .unwrap()
            .at = Vec2 { x: 17.0, y: -8.0 };

        let changed = reconcile(&mut document, "pair-source-mirror", &projection).expect(
            "explicit Omitted ownership permits absent target children under a linked circuit",
        );
        assert_eq!(changed, vec!["mirror-module"]);
        assert!(
            document
                .parts
                .iter()
                .all(|part| part.id != "embedded/mirror-circuit/U1")
        );
        assert!(
            document
                .nets
                .iter()
                .all(|net| net.id != "embedded/mirror-circuit/N$1")
        );
        let mirror_circuit = document
            .embedded_circuits
            .iter()
            .find(|circuit| circuit.id == "mirror-circuit")
            .expect("the linked target circuit remains independently owned");
        assert!(
            !mirror_circuit
                .part_ids
                .contains(&"embedded/mirror-circuit/U1".to_owned())
        );
        assert!(
            !mirror_circuit
                .net_ids
                .contains(&"embedded/mirror-circuit/N$1".to_owned())
        );
        assert!(document.embedded_circuits.iter().any(|circuit| {
            circuit.id == "source-circuit"
                && circuit
                    .part_ids
                    .contains(&"embedded/source-circuit/U1".to_owned())
                && circuit
                    .net_ids
                    .contains(&"embedded/source-circuit/N$1".to_owned())
        }));
        let joined_mirror_net = document
            .nets
            .iter()
            .find(|net| net.id == "host-mirror-net")
            .expect("the independently owned joined host net remains present");
        assert!(
            joined_mirror_net.pins.iter().any(|pin| {
                pin.part_id == "matrix/mirror-matrix/r0c0" && pin.pad_id == "pad-two"
            })
        );
        assert!(
            joined_mirror_net
                .pins
                .iter()
                .all(|pin| pin.part_id != "embedded/mirror-circuit/U1")
        );
        assert!(
            (document
                .modules
                .iter()
                .find(|module| module.id == "mirror-module")
                .unwrap()
                .at
                .x
                - 108.0)
                .abs()
                < 0.02
        );
        assert_eq!(pair_state_bytes(&document), pair_before);
        assert_eq!(
            *projection.applied_hosted_operations.borrow(),
            applied_before,
            "Omitted owner input does not create Add provenance"
        );
        assert_eq!(
            *projection.hosted_removal_baselines.borrow(),
            removal_context_before,
            "Omitted owner input does not create lifecycle removal evidence"
        );
        assert_eq!(
            *projection.hosted_omission_baselines.borrow(),
            omission_context_before,
            "the exact validated Omitted owner context remains unchanged"
        );

        let first_output = document.clone();
        let repeated = reconcile(&mut document, "pair-source-mirror", &projection)
            .expect("the updated omitted-child context is replayable");
        assert!(repeated.is_empty());
        assert_eq!(
            document, first_output,
            "a repeat changes no document identity"
        );
        assert_eq!(pair_state_bytes(&document), pair_before);
        assert_eq!(
            *projection.applied_hosted_operations.borrow(),
            applied_before
        );
        assert_eq!(
            *projection.hosted_removal_baselines.borrow(),
            removal_context_before
        );
        assert_eq!(
            *projection.hosted_omission_baselines.borrow(),
            omission_context_before
        );
        assert_ne!(pre_materialization.parts.len(), document.parts.len());
    }

    #[test]
    fn hosted_module_and_embedded_circuit_edits_follow_established_correspondences() {
        let (mut document, projection) = hosted_electrical_fixture().unwrap();
        reconcile(&mut document, "pair-source-mirror", &projection)
            .expect("the literal source and target records establish one valid baseline");
        let source_reference_before = document.board_references[0].clone();
        let source_copper_before = document
            .boards
            .iter()
            .find(|board| board.id == "source-board")
            .unwrap()
            .traces
            .clone();
        document
            .modules
            .iter_mut()
            .find(|module| module.id == "source-module")
            .unwrap()
            .at = Vec2 { x: 17.0, y: -8.0 };
        document
            .modules
            .iter_mut()
            .find(|module| module.id == "source-module")
            .unwrap()
            .rotation = 37.0;
        let source_member = document
            .parts
            .iter_mut()
            .find(|part| part.id == "embedded/source-circuit/U1")
            .unwrap();
        source_member.pose = Pose2 {
            at: Vec2 { x: 11.0, y: 13.0 },
            rotation: 23.0,
        };
        source_member
            .properties
            .as_mut()
            .unwrap()
            .insert("edited".into(), serde_json::json!({"revision": 2}));
        document
            .nets
            .iter_mut()
            .find(|net| net.id == "embedded/source-circuit/N$1")
            .unwrap()
            .name = "PRIVATE EDITED".into();
        document
            .nets
            .iter_mut()
            .find(|net| net.id == "embedded/source-circuit/N$1")
            .unwrap()
            .pins
            .push(Pin {
                part_id: "matrix/source-matrix/r0c0".into(),
                pad_id: "pad-two".into(),
            });
        document
            .embedded_circuits
            .iter_mut()
            .find(|circuit| circuit.id == "source-circuit")
            .unwrap()
            .ports
            .insert("signal".into(), "embedded/source-circuit/N$1".into());
        let pair_before = pair_state_bytes(&document);
        let changed = reconcile(&mut document, "pair-source-mirror", &projection).unwrap();
        assert_eq!(pair_state_bytes(&document), pair_before);

        let mirror_module = document
            .modules
            .iter()
            .find(|module| module.id == "mirror-module")
            .unwrap();
        assert!((mirror_module.at.x - 108.0).abs() < 0.02);
        assert!((mirror_module.at.y + 47.0).abs() < 0.02);
        assert_eq!(mirror_module.rotation, 233.0);
        assert_eq!(
            mirror_module
                .connection
                .as_ref()
                .unwrap()
                .host_connector_part_id,
            "matrix/mirror-matrix/r0c0"
        );
        let mirror_member = document
            .parts
            .iter()
            .find(|part| part.id == "embedded/mirror-circuit/U1")
            .unwrap();
        assert!((mirror_member.pose.at.x - 87.0).abs() < 0.02);
        assert!((mirror_member.pose.at.y + 41.0).abs() < 0.02);
        assert_eq!(mirror_member.pose.rotation, 247.0);
        assert_eq!(
            mirror_member.properties.as_ref().unwrap().get("edited"),
            Some(&serde_json::json!({"revision": 2}))
        );
        let mirror_private_net = document
            .nets
            .iter()
            .find(|net| net.id == "embedded/mirror-circuit/N$1")
            .unwrap();
        assert_eq!(mirror_private_net.name, "PRIVATE EDITED");
        assert_eq!(
            mirror_private_net.pins[0].part_id,
            "embedded/mirror-circuit/U1"
        );
        assert_eq!(
            mirror_private_net.pins[1].part_id,
            "matrix/mirror-matrix/r0c0"
        );
        assert_eq!(mirror_private_net.pins[1].pad_id, "pad-two");
        assert_eq!(
            document
                .embedded_circuits
                .iter()
                .find(|circuit| circuit.id == "mirror-circuit")
                .unwrap()
                .ports["signal"],
            "embedded/mirror-circuit/N$1"
        );
        assert_eq!(
            changed,
            vec![
                "embedded/mirror-circuit/N$1",
                "embedded/mirror-circuit/U1",
                "mirror-circuit",
                "mirror-module",
            ]
        );
        let mirror_electrical = document
            .hardware
            .as_ref()
            .unwrap()
            .boards
            .iter()
            .find(|entry| entry.board_id == "mirror-board")
            .unwrap();
        assert_eq!(
            mirror_electrical.controller_part_id.as_deref(),
            Some("embedded/mirror-circuit/U1")
        );
        assert_eq!(
            mirror_electrical
                .locks
                .get("direct/embedded/mirror-circuit/U1")
                .map(String::as_str),
            Some("embedded/mirror-circuit/N$1")
        );
        assert_eq!(
            mirror_electrical
                .assignments
                .get("peripheral/embedded/mirror-circuit/U1/signal")
                .map(String::as_str),
            Some("host-mirror-net")
        );
        assert_eq!(
            mirror_electrical
                .key_bindings
                .get("local-function")
                .map(String::as_str),
            Some("mirror-local-binding")
        );
        assert_eq!(document.board_references, vec![source_reference_before]);
        assert_eq!(
            document
                .boards
                .iter()
                .find(|board| board.id == "source-board")
                .unwrap()
                .traces,
            source_copper_before
        );
        assert!(
            !document
                .boards
                .iter()
                .find(|board| board.id == "mirror-board")
                .unwrap()
                .traces
                .iter()
                .any(|trace| trace.id == "source-only-copper")
        );
        let first_output = document.clone();
        let repeated_changed = reconcile(&mut document, "pair-source-mirror", &projection).unwrap();
        assert!(
            repeated_changed.is_empty(),
            "the same hosted projection converges"
        );
        assert_eq!(
            document, first_output,
            "whole document and private pair context repeat exactly"
        );
    }

    #[test]
    fn removing_a_hosted_module_keeps_its_independently_owned_connector() {
        let (mut document, mut projection) = hosted_electrical_fixture().unwrap();
        crate::mechanical::halves::validate_document(&document)
            .expect("the canonical pre-source module baseline validates");
        projection.hosted_removal_baselines.borrow_mut().insert(
            (
                "remove-module".into(),
                "source-module".into(),
                "mirror-module".into(),
            ),
            document.clone(),
        );
        crate::modules::remove(&mut document, "source-module")
            .expect("the ordinary source operation removes only the module record");
        projection.hosted_transitions = vec![HostedTransition::RemoveModule {
            source_id: "source-module".into(),
            mirror_id: "mirror-module".into(),
        }];
        let pair_before = pair_state_bytes(&document);

        reconcile(&mut document, "pair-source-mirror", &projection)
            .expect("the source module removal has a complete target identity");
        assert_eq!(pair_state_bytes(&document), pair_before);
        assert!(projection.applied_hosted_operations.borrow().contains(&(
            "remove-module".into(),
            "source-module".into(),
            "mirror-module".into()
        )));

        assert!(
            !document
                .modules
                .iter()
                .any(|module| module.id == "mirror-module")
        );
        assert!(
            document
                .parts
                .iter()
                .any(|part| part.id == "matrix/mirror-matrix/r0c0")
        );
        assert!(
            document
                .boards
                .iter()
                .find(|board| board.id == "mirror-board")
                .unwrap()
                .part_ids
                .contains(&"matrix/mirror-matrix/r0c0".into())
        );
        let first_output = document.clone();
        let first_context = projection.applied_hosted_operations.borrow().clone();
        let repeated = reconcile(&mut document, "pair-source-mirror", &projection);
        assert!(
            repeated.is_ok(),
            "the same hosted removal must accept its full output: {:?}",
            repeated.as_ref().err()
        );
        assert!(repeated.unwrap().is_empty());
        assert_eq!(document, first_output);
        assert_eq!(
            *projection.applied_hosted_operations.borrow(),
            first_context
        );
    }

    #[test]
    fn adding_a_hosted_module_uses_the_selected_module_and_connector_ids() {
        let (mut document, mut projection) = hosted_electrical_fixture().unwrap();
        document
            .modules
            .retain(|module| module.id != "mirror-module");
        document.hardware.as_mut().unwrap().pairs[0]
            .halves
            .as_mut()
            .unwrap()
            .correspondences
            .retain(|association| {
                !(association.kind == MirroredItemKind::Module
                    && association.source_id == "source-module")
            });
        projection.hosted_transitions = vec![HostedTransition::AddModule {
            source_id: "source-module".into(),
            mirror_id: "mirror-module-added".into(),
        }];
        let pair_before = pair_state_bytes(&document);

        reconcile(&mut document, "pair-source-mirror", &projection)
            .expect("the caller selects the new module identity explicitly");
        assert_ne!(pair_state_bytes(&document), pair_before);
        assert!(
            document.hardware.as_ref().unwrap().pairs[0]
                .halves
                .as_ref()
                .unwrap()
                .correspondences
                .iter()
                .any(|association| {
                    association.kind == MirroredItemKind::Module
                        && association.source_id == "source-module"
                        && association.mirror_id == "mirror-module-added"
                })
        );
        assert!(projection.applied_hosted_operations.borrow().contains(&(
            "module".into(),
            "source-module".into(),
            "mirror-module-added".into()
        )));

        let mirror = document
            .modules
            .iter()
            .find(|module| module.id == "mirror-module-added")
            .unwrap();
        assert_eq!(mirror.host_board_id, "mirror-board");
        assert_eq!(
            mirror.connection.as_ref().unwrap().host_connector_part_id,
            "matrix/mirror-matrix/r0c0"
        );

        let original_mirror_pose = mirror.at;
        projection.hosted_transitions.clear();
        projection.applied_hosted_operations.borrow_mut().clear();
        projection
            .applied_hosted_circuit_additions
            .borrow_mut()
            .clear();
        let source_module = document
            .modules
            .iter_mut()
            .find(|module| module.id == "source-module")
            .unwrap();
        source_module.at.x += 5.0;
        source_module.gap = 0.75;
        source_module.connection.as_mut().unwrap().bus_id = "updated-bus".into();
        document
            .modules
            .iter_mut()
            .find(|module| module.id == "mirror-module-added")
            .unwrap()
            .connection
            .as_mut()
            .unwrap()
            .bus_id = "mirror-local-edit".into();

        let changes = reconcile(&mut document, "pair-source-mirror", &projection)
            .expect("the durable module correspondence supports later source and mirror edits");
        assert!(changes.contains(&"mirror-module-added".to_owned()));
        let projected = document
            .modules
            .iter()
            .find(|module| module.id == "mirror-module-added")
            .unwrap();
        assert_ne!(projected.at, original_mirror_pose);
        assert_eq!(projected.gap, 0.75);
        assert_eq!(
            projected.connection.as_ref().unwrap().bus_id,
            "mirror-board/updated-bus"
        );
        assert_eq!(
            projected
                .connection
                .as_ref()
                .unwrap()
                .host_connector_part_id,
            "matrix/mirror-matrix/r0c0"
        );
        let normalized = document.clone();
        assert!(
            reconcile(&mut document, "pair-source-mirror", &projection)
                .unwrap()
                .is_empty()
        );
        assert_eq!(document, normalized);
        let serialized = serde_json::to_vec(&document).unwrap();
        let mut reopened: ProjectDoc = serde_json::from_slice(&serialized).unwrap();
        assert!(
            reconcile(&mut reopened, "pair-source-mirror", &projection)
                .unwrap()
                .is_empty()
        );
        assert_eq!(reopened, normalized);
    }

    #[test]
    fn adding_an_embedded_circuit_uses_the_selected_namespaced_id_maps() {
        let (mut document, mut projection) = hosted_electrical_fixture().unwrap();
        document
            .embedded_circuits
            .retain(|circuit| circuit.id != "mirror-circuit");
        document
            .parts
            .retain(|part| part.id != "embedded/mirror-circuit/U1");
        document
            .definitions
            .retain(|definition| definition.id != "embedded/mirror-circuit/definition/u");
        document
            .nets
            .retain(|net| net.id != "embedded/mirror-circuit/N$1");
        let mirror_board = document
            .boards
            .iter_mut()
            .find(|board| board.id == "mirror-board")
            .unwrap();
        mirror_board
            .part_ids
            .retain(|id| id != "embedded/mirror-circuit/U1");
        mirror_board
            .net_ids
            .retain(|id| id != "embedded/mirror-circuit/N$1");
        document.hardware.as_mut().unwrap().pairs[0]
            .halves
            .as_mut()
            .unwrap()
            .correspondences
            .retain(|association| !match association.kind {
                MirroredItemKind::Part => association.source_id == "embedded/source-circuit/U1",
                MirroredItemKind::EmbeddedCircuit => association.source_id == "source-circuit",
                MirroredItemKind::Net => association.source_id == "embedded/source-circuit/N$1",
                _ => false,
            });
        projection.hosted_transitions = vec![HostedTransition::AddCircuit {
            source_id: "source-circuit".into(),
            mirror_id: "mirror-circuit".into(),
            member_ids: BTreeMap::from([(
                "embedded/source-circuit/U1".into(),
                "embedded/mirror-circuit/U1".into(),
            )]),
            member_references: BTreeMap::from([(
                "embedded/source-circuit/U1".into(),
                "mirror-U1".into(),
            )]),
            definition_ids: BTreeMap::from([(
                "embedded/source-circuit/definition/u".into(),
                "embedded/mirror-circuit/definition/u".into(),
            )]),
            net_ids: BTreeMap::from([
                (
                    "embedded/source-circuit/N$1".into(),
                    "embedded/mirror-circuit/N$1".into(),
                ),
                ("host-source-net".into(), "host-mirror-net".into()),
            ]),
            member_poses: BTreeMap::new(),
        }];
        let pair_before_add = pair_state_bytes(&document);

        let before_collision = document.clone();
        projection.hosted_transitions = vec![HostedTransition::AddCircuit {
            source_id: "source-circuit".into(),
            mirror_id: "mirror-circuit".into(),
            member_ids: BTreeMap::from([(
                "embedded/source-circuit/U1".into(),
                "geometry-probe".into(),
            )]),
            member_references: BTreeMap::from([(
                "embedded/source-circuit/U1".into(),
                "PROBE".into(),
            )]),
            definition_ids: BTreeMap::from([(
                "embedded/source-circuit/definition/u".into(),
                "embedded/mirror-circuit/definition/u".into(),
            )]),
            net_ids: BTreeMap::from([
                (
                    "embedded/source-circuit/N$1".into(),
                    "embedded/mirror-circuit/N$1".into(),
                ),
                ("host-source-net".into(), "host-mirror-net".into()),
            ]),
            member_poses: BTreeMap::new(),
        }];
        assert!(
            reconcile(&mut document, "pair-source-mirror", &projection).is_err(),
            "an existing target Part cannot be claimed by a circuit addition"
        );
        assert_eq!(
            document, before_collision,
            "target collision refusal is atomic"
        );
        projection.hosted_transitions = vec![HostedTransition::AddCircuit {
            source_id: "source-circuit".into(),
            mirror_id: "mirror-circuit".into(),
            member_ids: BTreeMap::from([(
                "embedded/source-circuit/U1".into(),
                "embedded/mirror-circuit/U1".into(),
            )]),
            member_references: BTreeMap::from([(
                "embedded/source-circuit/U1".into(),
                "mirror-U1".into(),
            )]),
            definition_ids: BTreeMap::from([(
                "embedded/source-circuit/definition/u".into(),
                "embedded/mirror-circuit/definition/u".into(),
            )]),
            net_ids: BTreeMap::from([
                (
                    "embedded/source-circuit/N$1".into(),
                    "embedded/mirror-circuit/N$1".into(),
                ),
                ("host-source-net".into(), "host-mirror-net".into()),
            ]),
            member_poses: BTreeMap::new(),
        }];

        reconcile(&mut document, "pair-source-mirror", &projection)
            .expect("the caller supplies a complete one-to-one circuit identity map");
        assert_ne!(pair_state_bytes(&document), pair_before_add);
        let admitted = document.hardware.as_ref().unwrap().pairs[0]
            .halves
            .as_ref()
            .unwrap();
        for (kind, source_id, mirror_id) in [
            (
                MirroredItemKind::EmbeddedCircuit,
                "source-circuit",
                "mirror-circuit",
            ),
            (
                MirroredItemKind::Part,
                "embedded/source-circuit/U1",
                "embedded/mirror-circuit/U1",
            ),
            (
                MirroredItemKind::Net,
                "embedded/source-circuit/N$1",
                "embedded/mirror-circuit/N$1",
            ),
        ] {
            assert!(admitted.correspondences.iter().any(|association| {
                association.kind == kind
                    && association.source_id == source_id
                    && association.mirror_id == mirror_id
            }));
        }
        assert!(projection.applied_hosted_operations.borrow().contains(&(
            "circuit".into(),
            "source-circuit".into(),
            "mirror-circuit".into()
        )));

        let mirror_member = document
            .parts
            .iter()
            .find(|part| part.id == "embedded/mirror-circuit/U1")
            .unwrap();
        assert_eq!(mirror_member.reference, "mirror-U1");
        assert_eq!(
            mirror_member.definition_id,
            "embedded/mirror-circuit/definition/u"
        );
        assert!(
            document
                .definitions
                .iter()
                .any(|definition| definition.id == "embedded/mirror-circuit/definition/u")
        );
        assert!(
            document
                .nets
                .iter()
                .any(|net| net.id == "embedded/mirror-circuit/N$1")
        );
        let mirror_private = document
            .nets
            .iter()
            .find(|net| net.id == "embedded/mirror-circuit/N$1")
            .unwrap();
        assert_eq!(
            mirror_private.pins,
            vec![Pin {
                part_id: "embedded/mirror-circuit/U1".into(),
                pad_id: "pad-1".into()
            }]
        );
        assert!(document.nets.iter().any(|net| {
            net.id == "host-mirror-net"
                && net
                    .pins
                    .iter()
                    .any(|pin| pin.part_id == "embedded/mirror-circuit/U1")
        }));
        assert!(
            document
                .embedded_circuits
                .iter()
                .any(|circuit| circuit.id == "mirror-circuit"
                    && circuit.part_ids == vec!["embedded/mirror-circuit/U1"])
        );
    }

    #[test]
    fn circuit_removal_preserves_joined_host_nets_copper_and_unrelated_parts() {
        let (mut document, mut projection) = hosted_electrical_fixture().unwrap();
        crate::mechanical::halves::validate_document(&document)
            .expect("the canonical pre-source circuit baseline validates");
        projection.hosted_removal_baselines.borrow_mut().insert(
            (
                "remove-circuit".into(),
                "source-circuit".into(),
                "mirror-circuit".into(),
            ),
            document.clone(),
        );
        crate::modules::remove_circuit(&mut document, "source-circuit")
            .expect("the ordinary source operation removes the source circuit");
        document
            .boards
            .iter_mut()
            .find(|board| board.id == "mirror-board")
            .unwrap()
            .traces
            .push(CopperTrace {
                id: "joined-net-route".into(),
                start: Vec2 { x: 1.0, y: 2.0 },
                end: Vec2 { x: 3.0, y: 4.0 },
                width: 0.25,
                layer: Side::Front,
                net_id: Some("host-mirror-net".into()),
            });
        projection.hosted_transitions = vec![HostedTransition::RemoveCircuit {
            source_id: "source-circuit".into(),
            mirror_id: "mirror-circuit".into(),
        }];
        let pair_before = pair_state_bytes(&document);

        reconcile(&mut document, "pair-source-mirror", &projection)
            .expect("the source circuit removal reconciles through the existing domain closure");
        assert_eq!(pair_state_bytes(&document), pair_before);
        assert!(projection.applied_hosted_operations.borrow().contains(&(
            "remove-circuit".into(),
            "source-circuit".into(),
            "mirror-circuit".into()
        )));

        let mirror_board = document
            .boards
            .iter()
            .find(|board| board.id == "mirror-board")
            .unwrap();
        assert!(
            !document
                .embedded_circuits
                .iter()
                .any(|circuit| circuit.id == "mirror-circuit")
        );
        assert!(
            !document
                .parts
                .iter()
                .any(|part| part.id == "embedded/mirror-circuit/U1")
        );
        assert!(
            document
                .parts
                .iter()
                .any(|part| part.id == "geometry-probe")
        );
        assert!(mirror_board.part_ids.contains(&"geometry-probe".into()));
        assert!(document.nets.iter().any(|net| net.id == "host-mirror-net"));
        assert!(document.nets.iter().any(|net| {
            net.id == "host-mirror-net"
                && net
                    .pins
                    .iter()
                    .any(|pin| pin.part_id == "matrix/mirror-matrix/r0c0")
        }));
        assert!(
            mirror_board
                .traces
                .iter()
                .any(|trace| trace.id == "joined-net-route"
                    && trace.net_id.as_deref() == Some("host-mirror-net"))
        );
        assert!(
            !document
                .definitions
                .iter()
                .any(|definition| definition.id == "embedded/mirror-circuit/definition/u")
        );
        let first_output = document.clone();
        let first_context = projection.applied_hosted_operations.borrow().clone();
        let repeated = reconcile(&mut document, "pair-source-mirror", &projection);
        assert!(
            repeated.is_ok(),
            "the same hosted removal must accept its full output: {:?}",
            repeated.as_ref().err()
        );
        assert!(repeated.unwrap().is_empty());
        assert_eq!(document, first_output);
        assert_eq!(
            *projection.applied_hosted_operations.borrow(),
            first_context
        );
    }

    #[test]
    fn locked_circuit_removal_refuses_atomically() {
        let (mut document, mut projection) = hosted_electrical_fixture().unwrap();
        projection.hosted_removal_baselines.borrow_mut().insert(
            (
                "remove-circuit".into(),
                "source-circuit".into(),
                "mirror-circuit".into(),
            ),
            document.clone(),
        );
        crate::modules::remove_circuit(&mut document, "source-circuit")
            .expect("the ordinary source operation removes the source circuit");
        document
            .parts
            .iter_mut()
            .find(|part| part.id == "embedded/mirror-circuit/U1")
            .unwrap()
            .locked = Some(true);
        projection.hosted_transitions = vec![HostedTransition::RemoveCircuit {
            source_id: "source-circuit".into(),
            mirror_id: "mirror-circuit".into(),
        }];
        let before = document.clone();

        assert!(reconcile(&mut document, "pair-source-mirror", &projection).is_err());
        assert_eq!(
            document, before,
            "a late domain refusal cannot publish a partial circuit cleanup"
        );
    }

    #[test]
    fn hosted_module_removal_refuses_an_unpaired_foreign_target_atomically() {
        let (mut document, mut projection) = hosted_electrical_fixture().unwrap();
        let mut unrelated_source = document
            .modules
            .iter()
            .find(|module| module.id == "source-module")
            .unwrap()
            .clone();
        unrelated_source.id = "unpaired-source-module".into();
        let mut unrelated_target = document
            .modules
            .iter()
            .find(|module| module.id == "mirror-module")
            .unwrap()
            .clone();
        unrelated_target.id = "unpaired-target-module".into();
        document
            .modules
            .extend([unrelated_source, unrelated_target]);
        crate::mechanical::halves::validate_document(&document)
            .expect("the unpaired modules are outside valid pair ownership");
        projection.hosted_removal_baselines.borrow_mut().insert(
            (
                "remove-module".into(),
                "unpaired-source-module".into(),
                "unpaired-target-module".into(),
            ),
            document.clone(),
        );
        crate::modules::remove(&mut document, "unpaired-source-module")
            .expect("the ordinary source operation removes the selected source module");
        projection.hosted_transitions = vec![HostedTransition::RemoveModule {
            source_id: "unpaired-source-module".into(),
            mirror_id: "unpaired-target-module".into(),
        }];
        let before = document.clone();
        let pair_before = pair_state_bytes(&document);
        let applied_before = projection.applied_hosted_operations.borrow().clone();
        let baselines_before = projection.hosted_removal_baselines.borrow().clone();

        assert!(
            reconcile(&mut document, "pair-source-mirror", &projection).is_err(),
            "a valid source removal cannot authorize deleting a target outside its pair"
        );
        assert_eq!(document, before);
        assert_eq!(pair_state_bytes(&document), pair_before);
        assert_eq!(
            *projection.applied_hosted_operations.borrow(),
            applied_before
        );
        assert_eq!(
            *projection.hosted_removal_baselines.borrow(),
            baselines_before
        );
        assert!(
            document
                .modules
                .iter()
                .any(|module| module.id == "unpaired-target-module")
        );
    }

    #[test]
    fn hosted_circuit_removal_refuses_an_unpaired_foreign_target_atomically() {
        let (mut document, mut projection) = hosted_electrical_fixture().unwrap();
        document.embedded_circuits.extend([
            EmbeddedCircuit {
                id: "unpaired-source-circuit".into(),
                definition_id: "module-definition".into(),
                host_board_id: "source-board".into(),
                part_ids: vec![],
                net_ids: vec![],
                ports: BTreeMap::new(),
            },
            EmbeddedCircuit {
                id: "unpaired-target-circuit".into(),
                definition_id: "module-definition".into(),
                host_board_id: "mirror-board".into(),
                part_ids: vec![],
                net_ids: vec![],
                ports: BTreeMap::new(),
            },
        ]);
        crate::mechanical::halves::validate_document(&document)
            .expect("the unpaired circuits are outside valid pair ownership");
        projection.hosted_removal_baselines.borrow_mut().insert(
            (
                "remove-circuit".into(),
                "unpaired-source-circuit".into(),
                "unpaired-target-circuit".into(),
            ),
            document.clone(),
        );
        crate::modules::remove_circuit(&mut document, "unpaired-source-circuit")
            .expect("the ordinary source operation removes the selected source circuit");
        projection.hosted_transitions = vec![HostedTransition::RemoveCircuit {
            source_id: "unpaired-source-circuit".into(),
            mirror_id: "unpaired-target-circuit".into(),
        }];
        let before = document.clone();
        let pair_before = pair_state_bytes(&document);
        let applied_before = projection.applied_hosted_operations.borrow().clone();
        let baselines_before = projection.hosted_removal_baselines.borrow().clone();

        assert!(
            reconcile(&mut document, "pair-source-mirror", &projection).is_err(),
            "a valid source removal cannot authorize deleting a circuit outside its pair"
        );
        assert_eq!(document, before);
        assert_eq!(pair_state_bytes(&document), pair_before);
        assert_eq!(
            *projection.applied_hosted_operations.borrow(),
            applied_before
        );
        assert_eq!(
            *projection.hosted_removal_baselines.borrow(),
            baselines_before
        );
        assert!(
            document
                .embedded_circuits
                .iter()
                .any(|circuit| circuit.id == "unpaired-target-circuit")
        );
    }

    #[test]
    fn local_module_exception_keeps_other_hosted_correspondences_linked() {
        let (mut document, projection) = hosted_electrical_fixture().unwrap();
        reconcile(&mut document, "pair-source-mirror", &projection).unwrap();
        let local_module_pose = Pose2 {
            at: Vec2 {
                x: 999.0,
                y: -888.0,
            },
            rotation: 61.0,
        };
        let halves = document.hardware.as_mut().unwrap().pairs[0]
            .halves
            .as_mut()
            .unwrap();
        halves.correspondences.retain(|association| {
            !(association.kind == MirroredItemKind::Module
                && association.mirror_id == "mirror-module")
        });
        halves.exceptions.push(MirroredException {
            companion_mode: None,
            companion_tweak: None,
            kind: MirroredItemKind::Module,
            removed_copper: None,
            mirror_id: "mirror-module".into(),
            state: MirroredExceptionState::Local,
        });
        let mirror_module = document
            .modules
            .iter_mut()
            .find(|module| module.id == "mirror-module")
            .unwrap();
        mirror_module.at = local_module_pose.at;
        mirror_module.rotation = local_module_pose.rotation;
        document
            .modules
            .iter_mut()
            .find(|module| module.id == "source-module")
            .unwrap()
            .at = Vec2 { x: 21.0, y: 15.0 };
        document
            .parts
            .iter_mut()
            .find(|part| part.id == "embedded/source-circuit/U1")
            .unwrap()
            .pose
            .at = Vec2 { x: 14.0, y: 12.0 };
        let pair_before = pair_state_bytes(&document);

        reconcile(&mut document, "pair-source-mirror", &projection)
            .expect("a Local module is retained while other items reconcile");
        assert_eq!(pair_state_bytes(&document), pair_before);

        let mirror_module = document
            .modules
            .iter()
            .find(|module| module.id == "mirror-module")
            .unwrap();
        assert_eq!(mirror_module.at, local_module_pose.at);
        assert_eq!(mirror_module.rotation, local_module_pose.rotation);
        assert_eq!(
            mirror_module
                .connection
                .as_ref()
                .unwrap()
                .host_connector_part_id,
            "matrix/mirror-matrix/r0c0"
        );
        let mirror_member = document
            .parts
            .iter()
            .find(|part| part.id == "embedded/mirror-circuit/U1")
            .unwrap();
        assert!((mirror_member.pose.at.x - 88.0).abs() < 0.02);
        assert!((mirror_member.pose.at.y + 44.0).abs() < 0.02);
    }

    #[test]
    fn omitted_embedded_circuit_exception_stays_durable_and_unprojected() {
        let (mut document, projection) = hosted_electrical_fixture().unwrap();
        let halves = document.hardware.as_mut().unwrap().pairs[0]
            .halves
            .as_mut()
            .unwrap();
        halves.correspondences.retain(|association| {
            !((association.kind == MirroredItemKind::EmbeddedCircuit
                && association.mirror_id == "mirror-circuit")
                || (association.kind == MirroredItemKind::Part
                    && association.mirror_id == "embedded/mirror-circuit/U1")
                || (association.kind == MirroredItemKind::Net
                    && association.mirror_id == "embedded/mirror-circuit/N$1"))
        });
        halves.exceptions.extend([
            MirroredException {
                companion_mode: None,
                companion_tweak: None,
                kind: MirroredItemKind::EmbeddedCircuit,
                removed_copper: None,
                mirror_id: "mirror-circuit".into(),
                state: MirroredExceptionState::Omitted,
            },
            MirroredException {
                companion_mode: None,
                companion_tweak: None,
                kind: MirroredItemKind::Part,
                removed_copper: None,
                mirror_id: "embedded/mirror-circuit/U1".into(),
                state: MirroredExceptionState::Omitted,
            },
            MirroredException {
                companion_mode: None,
                companion_tweak: None,
                kind: MirroredItemKind::Net,
                removed_copper: None,
                mirror_id: "embedded/mirror-circuit/N$1".into(),
                state: MirroredExceptionState::Omitted,
            },
        ]);
        document
            .parts
            .iter_mut()
            .find(|part| part.id == "embedded/source-circuit/U1")
            .unwrap()
            .pose
            .at = Vec2 { x: 14.0, y: 12.0 };
        let mirror_circuit_before = document
            .embedded_circuits
            .iter()
            .find(|circuit| circuit.id == "mirror-circuit")
            .unwrap()
            .clone();
        let mirror_member_before = document
            .parts
            .iter()
            .find(|part| part.id == "embedded/mirror-circuit/U1")
            .unwrap()
            .clone();
        let mirror_net_before = document
            .nets
            .iter()
            .find(|net| net.id == "embedded/mirror-circuit/N$1")
            .unwrap()
            .clone();
        let pair_before = pair_state_bytes(&document);

        reconcile(&mut document, "pair-source-mirror", &projection)
            .expect("an unlinked Omitted circuit remains under exception ownership");
        assert_eq!(pair_state_bytes(&document), pair_before);

        assert_eq!(
            document
                .embedded_circuits
                .iter()
                .find(|circuit| circuit.id == "mirror-circuit")
                .unwrap(),
            &mirror_circuit_before
        );
        assert_eq!(
            document
                .parts
                .iter()
                .find(|part| part.id == "embedded/mirror-circuit/U1")
                .unwrap(),
            &mirror_member_before
        );
        assert_eq!(
            document
                .nets
                .iter()
                .find(|net| net.id == "embedded/mirror-circuit/N$1")
                .unwrap(),
            &mirror_net_before
        );
    }

    fn fixed_geometry_matrix(id: &str, board_id: &str, source_geometry: bool) -> Matrix {
        let mut matrix = Matrix {
            id: id.into(),
            name: None,
            rows: 2,
            columns: 3,
            pitch: Vec2 { x: 10.0, y: 20.0 },
            origin: if source_geometry {
                Vec2 { x: 3.0, y: -2.0 }
            } else {
                Vec2::default()
            },
            definition_id: "switch".into(),
            part_ids: vec![],
            board_id: Some(board_id.into()),
            mirror: None,
            rotation: None,
            edge_gap: None,
            diode_direction: None,
            row_offsets: vec![],
            column_offsets: vec![],
            column_staggers: vec![],
            column_splays: vec![],
            column_origins: vec![],
            cells: (0..2)
                .flat_map(|row| {
                    (0..3).map(move |column| MatrixCell {
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
                    })
                })
                .collect(),
        };
        if source_geometry {
            matrix.row_offsets = vec![Vec2::default(), Vec2 { x: 1.0, y: 2.0 }];
            matrix.column_offsets = vec![
                Vec2::default(),
                Vec2 { x: 3.0, y: 4.0 },
                Vec2 { x: -2.0, y: 1.0 },
            ];
            matrix.column_staggers = vec![1.0, 2.0, -1.0];
            matrix.column_splays = vec![90.0, -90.0, 90.0];
            matrix.column_origins = vec![
                Some(Vec2 { x: 2.0, y: 3.0 }),
                None,
                Some(Vec2 { x: 17.0, y: -4.0 }),
            ];
            let final_cell = matrix
                .cells
                .iter_mut()
                .find(|cell| cell.row == 0 && cell.column == 2)
                .unwrap();
            final_cell.offset = Some(Vec2 { x: 2.0, y: -3.0 });
            final_cell.rotation = Some(90.0);
        }
        matrix
    }

    fn fixed_geometry_fixture() -> Result<(ProjectDoc, Projection), String> {
        let mut document = fixture_document()?;
        let mapping = Pose2 {
            at: Vec2 { x: 100.0, y: -30.0 },
            rotation: 90.0,
        };
        document.hardware.as_mut().unwrap().pairs[0].mapping = mapping;
        crate::matrix::set_matrix(
            &mut document,
            &fixed_geometry_matrix("source-matrix", "source-board", true),
        )?;
        crate::matrix::set_matrix(
            &mut document,
            &fixed_geometry_matrix("mirror-matrix", "mirror-board", false),
        )?;

        let halves = document.hardware.as_mut().unwrap().pairs[0]
            .halves
            .as_mut()
            .unwrap();
        halves.correspondences = vec![MirroredCorrespondence {
            companion_mode: None,
            companion_tweak: None,
            kind: MirroredItemKind::Matrix,
            source_id: "source-matrix".into(),
            mirror_id: "mirror-matrix".into(),
        }];
        for row in 0..2 {
            for column in 0..3 {
                halves.correspondences.push(MirroredCorrespondence {
                    companion_mode: None,
                    companion_tweak: None,
                    kind: MirroredItemKind::Part,
                    source_id: format!("matrix/source-matrix/r{row}c{column}"),
                    mirror_id: format!("matrix/mirror-matrix/r{row}c{column}"),
                });
            }
        }

        let definition = document
            .definitions
            .iter_mut()
            .find(|definition| definition.id == "switch")
            .unwrap();
        definition.pads = vec![
            crate::model::Pad {
                id: "pad-one".into(),
                number: "1".into(),
                at: Vec2::default(),
                size: Vec2 { x: 1.0, y: 1.0 },
                shape: crate::model::PadShape::Circle,
                drill: None,
                plated: None,
                side: None,
                rotation: None,
                net_id: None,
            },
            crate::model::Pad {
                id: "pad-two".into(),
                number: "2".into(),
                at: Vec2 { x: 1.0, y: 0.0 },
                size: Vec2 { x: 1.0, y: 1.0 },
                shape: crate::model::PadShape::Circle,
                drill: None,
                plated: None,
                side: None,
                rotation: None,
                net_id: None,
            },
        ];
        let mut probe = document
            .parts
            .iter()
            .find(|part| part.id == "matrix/mirror-matrix/r0c2")
            .unwrap()
            .clone();
        probe.id = "geometry-probe".into();
        probe.reference = "PROBE".into();
        probe.pose = Pose2 {
            at: Vec2 { x: 4.0, y: 2.0 },
            rotation: 0.0,
        };
        document.parts.push(probe);
        document
            .boards
            .iter_mut()
            .find(|board| board.id == "mirror-board")
            .unwrap()
            .part_ids
            .push("geometry-probe".into());
        document.nets.push(crate::model::Net {
            id: "retained-second-key-net".into(),
            name: "RETAINED SECOND KEY".into(),
            pins: vec![
                crate::model::Pin {
                    part_id: "matrix/mirror-matrix/r1c1".into(),
                    pad_id: "pad-one".into(),
                },
                crate::model::Pin {
                    part_id: "geometry-probe".into(),
                    pad_id: "pad-two".into(),
                },
            ],
        });
        document
            .boards
            .iter_mut()
            .find(|board| board.id == "mirror-board")
            .unwrap()
            .net_ids
            .push("retained-second-key-net".into());
        document.constraints.push(crate::model::Constraint::Offset {
            id: "retained-second-key-offset".into(),
            source_part_id: "geometry-probe".into(),
            target_part_id: "matrix/mirror-matrix/r1c1".into(),
            offset: Vec2 { x: 6.0, y: 18.0 },
            rotation: 0.0,
        });
        document
            .outline
            .push(crate::model::OutlineFeature::PartEnvelope {
                connections: vec![],
                settings: Default::default(),
                id: "retained-keys-envelope".into(),
                part_ids: vec![
                    "matrix/mirror-matrix/r0c0".into(),
                    "matrix/mirror-matrix/r0c1".into(),
                    "matrix/mirror-matrix/r0c2".into(),
                    "matrix/mirror-matrix/r1c0".into(),
                    "matrix/mirror-matrix/r1c1".into(),
                    "matrix/mirror-matrix/r1c2".into(),
                    "geometry-probe".into(),
                ],
                margin: 2.0,
                operation: crate::model::Operation::Add,
            });
        document
            .boards
            .iter_mut()
            .find(|board| board.id == "mirror-board")
            .unwrap()
            .outline_ids
            .push("retained-keys-envelope".into());

        let linked_slots = [(0, 1), (0, 2), (1, 0), (1, 2)];
        let parts = linked_slots
            .into_iter()
            .map(|(row, column)| PartProjection {
                source_id: format!("matrix/source-matrix/r{row}c{column}"),
                mirror_id: format!("matrix/mirror-matrix/r{row}c{column}"),
            })
            .collect();
        let protected = BTreeSet::from([
            "matrix/mirror-matrix/r0c0".to_owned(),
            "matrix/mirror-matrix/r1c1".to_owned(),
        ]);
        let mut final_owners = BTreeMap::new();
        for id in document
            .matrices
            .iter()
            .filter(|matrix| matrix.id == "source-matrix" || matrix.id == "mirror-matrix")
            .flat_map(|matrix| matrix.part_ids.iter())
        {
            let owner = document
                .boards
                .iter()
                .find(|board| board.part_ids.contains(id))
                .unwrap();
            final_owners.insert(id.clone(), owner.id.clone());
        }
        let projection = Projection {
            matrix: Some(MatrixLink {
                source_matrix_id: "source-matrix".into(),
                mirror_matrix_id: "mirror-matrix".into(),
            }),
            preserve_matrix_geometry: false,
            source_board_id: "source-board".into(),
            mirror_board_id: "mirror-board".into(),
            mapping,
            parts,
            independent_part_transitions: vec![],
            hosted_transitions: vec![],
            applied_independent_additions: std::cell::RefCell::new(BTreeSet::new()),
            applied_hosted_operations: std::cell::RefCell::new(BTreeSet::new()),
            applied_hosted_circuit_additions: std::cell::RefCell::new(BTreeMap::new()),
            hosted_removal_baselines: std::cell::RefCell::new(BTreeMap::new()),
            hosted_omission_baselines: std::cell::RefCell::new(BTreeMap::new()),
            outline_removal_baselines: std::cell::RefCell::new(BTreeMap::new()),
            locally_protected_mirror_parts: protected,
            final_owners,
        };
        Ok((document, projection))
    }

    fn fixed_membership_assembly_fixture(
        mapping_rotation: f64,
    ) -> Result<(ProjectDoc, Projection), String> {
        let (mut document, mut projection) = fixed_geometry_fixture()?;
        let mut diode = document.definitions[0].clone();
        diode.id = "diode".into();
        diode.name = "Diode".into();
        diode.pads = vec![
            crate::model::Pad {
                id: "pad-one".into(),
                number: "1".into(),
                at: Vec2::default(),
                size: Vec2 { x: 1.0, y: 1.0 },
                shape: crate::model::PadShape::Circle,
                drill: None,
                plated: None,
                side: None,
                rotation: None,
                net_id: None,
            },
            crate::model::Pad {
                id: "pad-two".into(),
                number: "2".into(),
                at: Vec2 { x: 1.0, y: 0.0 },
                size: Vec2 { x: 1.0, y: 1.0 },
                shape: crate::model::PadShape::Circle,
                drill: None,
                plated: None,
                side: None,
                rotation: None,
                net_id: None,
            },
        ];
        let mut stabiliser = diode.clone();
        stabiliser.id = "kicad:marbastlib/STAB_MX_2u".into();
        stabiliser.name = "Stabiliser".into();
        document.definitions.extend([diode, stabiliser]);

        let make_assemblies = |matrix: &mut Matrix| {
            for cell in &mut matrix.cells {
                cell.assemblies = vec![MatrixAssembly {
                    id: "diode".into(),
                    definition_id: "diode".into(),
                    offset: Vec2 { x: 2.0, y: 3.0 },
                    rotation: Some(30.0),
                    side: Some(Side::Back),
                }];
                if (cell.row, cell.column) == (0, 1) {
                    cell.assemblies.push(MatrixAssembly {
                        id: "fallback".into(),
                        definition_id: "diode".into(),
                        offset: Vec2 { x: 5.0, y: -2.0 },
                        rotation: Some(45.0),
                        side: Some(Side::Back),
                    });
                }
                if (cell.row, cell.column) == (1, 2) {
                    cell.assemblies.push(MatrixAssembly {
                        id: "stab".into(),
                        definition_id: "kicad:marbastlib/STAB_MX_2u".into(),
                        offset: Vec2 { x: 4.0, y: 0.0 },
                        rotation: Some(0.0),
                        side: Some(Side::Front),
                    });
                }
            }
        };
        for (id, board_id) in [
            ("source-matrix", "source-board"),
            ("mirror-matrix", "mirror-board"),
        ] {
            let mut matrix = document
                .matrices
                .iter()
                .find(|matrix| matrix.id == id)
                .unwrap()
                .clone();
            make_assemblies(&mut matrix);
            if id == "source-matrix" {
                matrix.origin = Vec2 { x: 3.0, y: -2.0 };
                matrix.row_offsets = vec![Vec2::default(), Vec2 { x: 1.0, y: 2.0 }];
                matrix.column_offsets = vec![
                    Vec2::default(),
                    Vec2 { x: 3.0, y: 4.0 },
                    Vec2 { x: -2.0, y: 1.0 },
                ];
                matrix.column_staggers = vec![1.0, 2.0, -1.0];
                matrix.column_splays = vec![90.0, -90.0, 90.0];
                matrix.column_origins = vec![
                    Some(Vec2 { x: 2.0, y: 3.0 }),
                    None,
                    Some(Vec2 { x: 17.0, y: -4.0 }),
                ];
                let final_cell = matrix
                    .cells
                    .iter_mut()
                    .find(|cell| (cell.row, cell.column) == (0, 2))
                    .unwrap();
                final_cell.offset = Some(Vec2 { x: 2.0, y: -3.0 });
                final_cell.rotation = Some(90.0);
            }
            crate::matrix::set_matrix(&mut document, &matrix)?;
            debug_assert_eq!(matrix.board_id.as_deref(), Some(board_id));
        }

        let pair = &mut document.hardware.as_mut().unwrap().pairs[0];
        pair.mapping = Pose2 {
            at: Vec2 { x: 100.0, y: -30.0 },
            rotation: mapping_rotation,
        };
        let halves = pair.halves.as_mut().unwrap();
        halves.companion_mode = MirroredCompanionMode::DuplicatePosition;
        halves.correspondences = vec![MirroredCorrespondence {
            companion_mode: None,
            companion_tweak: None,
            kind: MirroredItemKind::Matrix,
            source_id: "source-matrix".into(),
            mirror_id: "mirror-matrix".into(),
        }];
        for row in 0..2 {
            for column in 0..3 {
                let key_source = format!("matrix/source-matrix/r{row}c{column}");
                let key_mirror = format!("matrix/mirror-matrix/r{row}c{column}");
                halves.correspondences.push(MirroredCorrespondence {
                    companion_mode: None,
                    companion_tweak: None,
                    kind: MirroredItemKind::Part,
                    source_id: key_source.clone(),
                    mirror_id: key_mirror.clone(),
                });
                let mut companion_ids = vec!["diode"];
                if (row, column) == (0, 1) {
                    companion_ids.push("fallback");
                }
                if (row, column) == (1, 2) {
                    companion_ids.push("stab");
                }
                for assembly_id in companion_ids {
                    let mode = match (row, column, assembly_id) {
                        (0, 1, "diode") | (1, 2, "diode") => {
                            Some(MirroredCompanionMode::LiteralMirror)
                        }
                        (1, 0, "diode") => Some(MirroredCompanionMode::LiteralMirrorWithTweak),
                        (0, 1, "fallback") => Some(MirroredCompanionMode::LiteralMirror),
                        _ => None,
                    };
                    let tweak = ((row, column, assembly_id) == (1, 0, "diode")).then_some(
                        MirroredCompanionTweak {
                            x: 1.0,
                            y: -2.0,
                            rotation: 15.0,
                        },
                    );
                    halves.correspondences.push(MirroredCorrespondence {
                        companion_mode: mode,
                        companion_tweak: tweak,
                        kind: MirroredItemKind::Part,
                        source_id: format!("{key_source}/{assembly_id}"),
                        mirror_id: format!("{key_mirror}/{assembly_id}"),
                    });
                }
            }
        }
        if mapping_rotation == 0.0 {
            projection.mapping.rotation = 0.0;
        } else {
            projection.mapping.rotation = mapping_rotation;
        }
        projection.parts = [(0, 1), (0, 2), (1, 0), (1, 2)]
            .into_iter()
            .map(|(row, column)| PartProjection {
                source_id: format!("matrix/source-matrix/r{row}c{column}"),
                mirror_id: format!("matrix/mirror-matrix/r{row}c{column}"),
            })
            .collect();
        projection.final_owners.clear();
        for id in document
            .matrices
            .iter()
            .filter(|matrix| matrix.id == "source-matrix" || matrix.id == "mirror-matrix")
            .flat_map(|matrix| matrix.part_ids.iter())
        {
            let owner = document
                .boards
                .iter()
                .find(|board| board.part_ids.contains(id))
                .unwrap();
            projection.final_owners.insert(id.clone(), owner.id.clone());
        }

        let mut probe = document
            .parts
            .iter()
            .find(|part| part.id == "matrix/mirror-matrix/r0c2")
            .unwrap()
            .clone();
        probe.id = "assembly-probe".into();
        probe.reference = "ASSEMBLY-PROBE".into();
        document.parts.push(probe);
        document
            .boards
            .iter_mut()
            .find(|board| board.id == "mirror-board")
            .unwrap()
            .part_ids
            .push("assembly-probe".into());
        for (index, retained_id) in [
            "matrix/mirror-matrix/r0c0/diode",
            "matrix/mirror-matrix/r1c1/diode",
        ]
        .into_iter()
        .enumerate()
        {
            let net_id = format!("retained-diode-net-{index}");
            document.nets.push(Net {
                id: net_id.clone(),
                name: net_id.clone(),
                pins: vec![
                    Pin {
                        part_id: retained_id.into(),
                        pad_id: "pad-one".into(),
                    },
                    Pin {
                        part_id: "assembly-probe".into(),
                        pad_id: "pad-two".into(),
                    },
                ],
            });
            document
                .boards
                .iter_mut()
                .find(|board| board.id == "mirror-board")
                .unwrap()
                .net_ids
                .push(net_id);
            document.constraints.push(Constraint::Offset {
                id: format!("retained-diode-constraint-{index}"),
                source_part_id: "assembly-probe".into(),
                target_part_id: retained_id.into(),
                offset: Vec2 { x: 2.0, y: -1.0 },
                rotation: 0.0,
            });
        }
        document.outline.push(OutlineFeature::PartEnvelope {
            connections: vec![],
            settings: Default::default(),
            id: "retained-diodes-envelope".into(),
            part_ids: vec![
                "matrix/mirror-matrix/r0c0/diode".into(),
                "matrix/mirror-matrix/r0c0".into(),
                "matrix/mirror-matrix/r0c1".into(),
                "matrix/mirror-matrix/r0c1/diode".into(),
                "matrix/mirror-matrix/r0c1/fallback".into(),
                "matrix/mirror-matrix/r0c2".into(),
                "matrix/mirror-matrix/r0c2/diode".into(),
                "matrix/mirror-matrix/r1c0".into(),
                "matrix/mirror-matrix/r1c0/diode".into(),
                "matrix/mirror-matrix/r1c1".into(),
                "matrix/mirror-matrix/r1c1/diode".into(),
                "matrix/mirror-matrix/r1c2".into(),
                "matrix/mirror-matrix/r1c2/diode".into(),
                "matrix/mirror-matrix/r1c2/stab".into(),
                "assembly-probe".into(),
            ],
            margin: 2.0,
            operation: Operation::Add,
        });
        document
            .boards
            .iter_mut()
            .find(|board| board.id == "mirror-board")
            .unwrap()
            .outline_ids
            .push("retained-diodes-envelope".into());
        for part in document
            .parts
            .iter_mut()
            .filter(|part| projection.final_owners.contains_key(&part.id))
        {
            part.locked = Some(true);
            part.properties = Some(BTreeMap::from([(
                "custom".into(),
                serde_json::json!({"part": part.id}),
            )]));
            part.generator_parameters = Some(BTreeMap::from([(
                "custom".into(),
                serde_json::json!({"part": part.id}),
            )]));
        }
        Ok((document, projection))
    }

    #[test]
    fn fixed_membership_assembly_fixture_reconciles_existing_companions() {
        let expected_changed = vec![
            "matrix/mirror-matrix/r0c1",
            "matrix/mirror-matrix/r0c1/diode",
            "matrix/mirror-matrix/r0c1/fallback",
            "matrix/mirror-matrix/r0c2",
            "matrix/mirror-matrix/r0c2/diode",
            "matrix/mirror-matrix/r1c0",
            "matrix/mirror-matrix/r1c0/diode",
            "matrix/mirror-matrix/r1c2",
            "matrix/mirror-matrix/r1c2/diode",
            "matrix/mirror-matrix/r1c2/stab",
            "mirror-matrix",
        ];
        let source_keys = [
            ("matrix/source-matrix/r0c0", (7.0, -1.0, 90.0)),
            ("matrix/source-matrix/r0c1", (8.0, 13.0, 0.0)),
            ("matrix/source-matrix/r0c2", (8.0, 5.0, 180.0)),
            ("matrix/source-matrix/r1c0", (-15.0, 0.0, 90.0)),
            ("matrix/source-matrix/r1c1", (9.0, 35.0, 0.0)),
            ("matrix/source-matrix/r1c2", (-17.0, 4.0, 90.0)),
        ];
        let source_companions = [
            ("matrix/source-matrix/r0c0/diode", (4.0, 1.0, 120.0)),
            ("matrix/source-matrix/r0c1/diode", (10.0, 16.0, 30.0)),
            ("matrix/source-matrix/r0c2/diode", (6.0, 2.0, 210.0)),
            ("matrix/source-matrix/r1c0/diode", (-18.0, 2.0, 120.0)),
            ("matrix/source-matrix/r1c1/diode", (11.0, 38.0, 30.0)),
            ("matrix/source-matrix/r1c2/diode", (-20.0, 6.0, 120.0)),
            ("matrix/source-matrix/r1c2/stab", (-17.0, 8.0, 90.0)),
            ("matrix/source-matrix/r0c1/fallback", (13.0, 11.0, 45.0)),
        ];
        let expected_rotated = [
            ("matrix/mirror-matrix/r0c0", (0.0, 0.0, 0.0)),
            ("matrix/mirror-matrix/r0c1", (87.0, -38.0, 90.0)),
            ("matrix/mirror-matrix/r0c2", (95.0, -38.0, -90.0)),
            ("matrix/mirror-matrix/r1c0", (100.0, -15.0, 0.0)),
            ("matrix/mirror-matrix/r1c1", (10.0, 20.0, 0.0)),
            ("matrix/mirror-matrix/r1c2", (96.0, -13.0, 0.0)),
            ("matrix/mirror-matrix/r0c0/diode", (2.0, 3.0, 30.0)),
            ("matrix/mirror-matrix/r0c1/diode", (84.0, -40.0, 60.0)),
            ("matrix/mirror-matrix/r0c2/diode", (98.0, -40.0, -60.0)),
            ("matrix/mirror-matrix/r1c0/diode", (99.0, -14.0, -15.0)),
            ("matrix/mirror-matrix/r1c1/diode", (12.0, 23.0, 30.0)),
            ("matrix/mirror-matrix/r1c2/diode", (94.0, -10.0, -30.0)),
            ("matrix/mirror-matrix/r1c2/stab", (92.0, -13.0, 90.0)),
            ("matrix/mirror-matrix/r0c1/fallback", (89.0, -43.0, 45.0)),
        ];
        let expected_axis = [
            ("matrix/mirror-matrix/r0c0", (0.0, 0.0, 0.0)),
            ("matrix/mirror-matrix/r0c1", (92.0, -17.0, 0.0)),
            ("matrix/mirror-matrix/r0c2", (92.0, -25.0, -180.0)),
            ("matrix/mirror-matrix/r1c0", (115.0, -30.0, -90.0)),
            ("matrix/mirror-matrix/r1c1", (10.0, 20.0, 0.0)),
            ("matrix/mirror-matrix/r1c2", (117.0, -26.0, -90.0)),
            ("matrix/mirror-matrix/r0c0/diode", (2.0, 3.0, 30.0)),
            ("matrix/mirror-matrix/r0c1/diode", (90.0, -14.0, -30.0)),
            ("matrix/mirror-matrix/r0c2/diode", (90.0, -28.0, -150.0)),
            ("matrix/mirror-matrix/r1c0/diode", (116.0, -29.0, -105.0)),
            ("matrix/mirror-matrix/r1c1/diode", (12.0, 23.0, 30.0)),
            ("matrix/mirror-matrix/r1c2/diode", (120.0, -24.0, -120.0)),
            ("matrix/mirror-matrix/r1c2/stab", (117.0, -22.0, 90.0)),
            ("matrix/mirror-matrix/r0c1/fallback", (87.0, -19.0, -45.0)),
        ];
        let assert_pose = |document: &ProjectDoc, id: &str, expected: (f64, f64, f64)| {
            let part = document.parts.iter().find(|part| part.id == id).unwrap();
            assert!((part.pose.at.x - expected.0).abs() < 1e-9, "{id} x");
            assert!((part.pose.at.y - expected.1).abs() < 1e-9, "{id} y");
            let heading_delta = (part.pose.rotation - expected.2 + 180.0).rem_euclid(360.0) - 180.0;
            assert!(
                heading_delta.abs() < 1e-9,
                "{id} rotation: {}",
                part.pose.rotation
            );
        };
        let non_pose = |part: &Part| {
            let mut value = serde_json::to_value(part).unwrap();
            value.as_object_mut().unwrap().remove("pose");
            value
        };

        for (mapping_rotation, expected_members, frame, local_cells) in [
            (
                90.0,
                &expected_rotated[..],
                (102.0, -33.0, 90.0),
                [((0, 0), (101.0, 37.0, 0.0)), ((1, 1), (-59.0, 55.0, -90.0))],
            ),
            (
                0.0,
                &expected_axis[..],
                (97.0, -32.0, 0.0),
                [((0, 0), (31.0, -93.0, 90.0)), ((1, 1), (81.0, 15.0, 0.0))],
            ),
        ] {
            let (mut document, projection) =
                fixed_membership_assembly_fixture(mapping_rotation).unwrap();
            for id in ["source-matrix", "mirror-matrix"] {
                let matrix = document
                    .matrices
                    .iter()
                    .find(|matrix| matrix.id == id)
                    .unwrap();
                crate::matrix::valid_matrix(matrix, &document).unwrap();
            }
            crate::mechanical::features::validate(&document).unwrap();
            crate::mechanical::halves::validate_document(&document).unwrap();
            assert_eq!(projection.parts.len(), 4);
            assert_eq!(projection.locally_protected_mirror_parts.len(), 2);
            assert_eq!(projection.final_owners.len(), 28);
            let member_parts: Vec<_> = document
                .parts
                .iter()
                .filter(|part| projection.final_owners.contains_key(&part.id))
                .collect();
            assert_eq!(member_parts.len(), 28);
            assert_eq!(
                member_parts
                    .iter()
                    .map(|part| part.reference.as_str())
                    .collect::<BTreeSet<_>>()
                    .len(),
                28
            );
            assert!(member_parts.iter().all(|part| {
                part.locked == Some(true)
                    && part.properties.as_ref().unwrap()["custom"]
                        == serde_json::json!({"part": part.id})
                    && part.generator_parameters.as_ref().unwrap()["custom"]
                        == serde_json::json!({"part": part.id})
            }));
            for matrix_id in ["source-matrix", "mirror-matrix"] {
                let matrix = document
                    .matrices
                    .iter()
                    .find(|matrix| matrix.id == matrix_id)
                    .unwrap();
                assert_eq!(matrix.part_ids.len(), 14);
            }
            assert_eq!(
                document.hardware.as_ref().unwrap().pairs[0]
                    .halves
                    .as_ref()
                    .unwrap()
                    .correspondences
                    .iter()
                    .filter(|association| association.kind == MirroredItemKind::Part)
                    .count(),
                14
            );
            for (id, expected) in source_keys.iter().chain(source_companions.iter()) {
                assert_pose(&document, id, *expected);
            }
            for (retained_id, expected_net, expected_constraint) in [
                (
                    "matrix/mirror-matrix/r0c0/diode",
                    "retained-diode-net-0",
                    "retained-diode-constraint-0",
                ),
                (
                    "matrix/mirror-matrix/r1c1/diode",
                    "retained-diode-net-1",
                    "retained-diode-constraint-1",
                ),
            ] {
                assert!(document.nets.iter().any(|net| {
                    net.id == expected_net
                        && net
                            .pins
                            .iter()
                            .any(|pin| pin.part_id == retained_id && pin.pad_id == "pad-one")
                }));
                assert!(
                    document
                        .constraints
                        .iter()
                        .any(|constraint| constraint.id() == expected_constraint)
                );
                assert!(document.outline.iter().any(|feature| matches!(feature, OutlineFeature::PartEnvelope { part_ids, .. } if part_ids.iter().any(|id| id == retained_id))));
                assert!(document.boards.iter().any(|board| {
                    board.id == "mirror-board"
                        && board.part_ids.iter().any(|id| id == retained_id)
                        && board.net_ids.contains(&expected_net.into())
                        && board
                            .outline_ids
                            .contains(&"retained-diodes-envelope".into())
                }));
            }

            let entry = document.clone();
            let source_ids = entry
                .matrices
                .iter()
                .find(|matrix| matrix.id == "source-matrix")
                .unwrap()
                .part_ids
                .clone();
            let retained_ids = [
                "matrix/mirror-matrix/r0c0",
                "matrix/mirror-matrix/r0c0/diode",
                "matrix/mirror-matrix/r1c1",
                "matrix/mirror-matrix/r1c1/diode",
            ];
            let source_parts: Vec<_> = source_ids
                .iter()
                .map(|id| {
                    entry
                        .parts
                        .iter()
                        .find(|part| &part.id == id)
                        .unwrap()
                        .clone()
                })
                .collect();
            let retained_parts: Vec<_> = retained_ids
                .iter()
                .map(|id| {
                    entry
                        .parts
                        .iter()
                        .find(|part| part.id == *id)
                        .unwrap()
                        .clone()
                })
                .collect();
            let affected_ids: Vec<_> = entry
                .matrices
                .iter()
                .filter(|matrix| matrix.id == "source-matrix" || matrix.id == "mirror-matrix")
                .flat_map(|matrix| matrix.part_ids.iter())
                .cloned()
                .collect();
            let non_pose_before: Vec<_> = affected_ids
                .iter()
                .map(|id| non_pose(entry.parts.iter().find(|part| &part.id == id).unwrap()))
                .collect();
            let references_before = serde_json::json!({
                "boards": entry.boards,
                "nets": entry.nets,
                "constraints": entry.constraints,
                "boardOutlines": entry.board_outlines,
                "outline": entry.outline,
            });
            let correspondences_before = entry.hardware.as_ref().unwrap().pairs[0]
                .halves
                .as_ref()
                .unwrap()
                .correspondences
                .clone();

            let changed = reconcile(&mut document, "pair-source-mirror", &projection).unwrap();
            assert_eq!(changed, expected_changed);
            assert_eq!(
                document.hardware.as_ref().unwrap().pairs[0]
                    .halves
                    .as_ref()
                    .unwrap()
                    .correspondences,
                correspondences_before
            );
            for (id, expected) in expected_members {
                assert_pose(&document, id, *expected);
            }
            let final_matrix = document
                .matrices
                .iter()
                .find(|matrix| matrix.id == "mirror-matrix")
                .unwrap();
            assert_eq!(
                final_matrix.origin,
                Vec2 {
                    x: frame.0,
                    y: frame.1
                }
            );
            assert!((final_matrix.rotation.unwrap_or(0.0) - frame.2).abs() < 1e-9);
            assert_eq!(final_matrix.mirror, Some(Mirror::X));
            let expected_part_ids = [
                "matrix/mirror-matrix/r0c0",
                "matrix/mirror-matrix/r0c0/diode",
                "matrix/mirror-matrix/r0c1",
                "matrix/mirror-matrix/r0c1/diode",
                "matrix/mirror-matrix/r0c1/fallback",
                "matrix/mirror-matrix/r0c2",
                "matrix/mirror-matrix/r0c2/diode",
                "matrix/mirror-matrix/r1c0",
                "matrix/mirror-matrix/r1c0/diode",
                "matrix/mirror-matrix/r1c1",
                "matrix/mirror-matrix/r1c1/diode",
                "matrix/mirror-matrix/r1c2",
                "matrix/mirror-matrix/r1c2/diode",
                "matrix/mirror-matrix/r1c2/stab",
            ];
            assert_eq!(final_matrix.part_ids, expected_part_ids);
            for ((row, column), (x, y, rotation)) in local_cells {
                let cell = final_matrix
                    .cells
                    .iter()
                    .find(|cell| (cell.row, cell.column) == (row, column))
                    .unwrap();
                assert!((cell.offset.unwrap().x - x).abs() < 1e-9);
                assert!((cell.offset.unwrap().y - y).abs() < 1e-9);
                assert!((cell.rotation.unwrap() - rotation).abs() < 1e-9);
            }
            for (id, expected) in source_ids.iter().zip(&source_parts) {
                assert_eq!(
                    document.parts.iter().find(|part| &part.id == id).unwrap(),
                    expected
                );
            }
            for (id, expected) in retained_ids.iter().zip(&retained_parts) {
                assert_eq!(
                    document.parts.iter().find(|part| part.id == *id).unwrap(),
                    expected
                );
            }
            let non_pose_after: Vec<_> = affected_ids
                .iter()
                .map(|id| non_pose(document.parts.iter().find(|part| &part.id == id).unwrap()))
                .collect();
            assert_eq!(non_pose_after, non_pose_before);
            assert_eq!(
                serde_json::json!({
                    "boards": document.boards,
                    "nets": document.nets,
                    "constraints": document.constraints,
                    "boardOutlines": document.board_outlines,
                    "outline": document.outline,
                }),
                references_before
            );
            for (id, owner) in &projection.final_owners {
                assert_eq!(
                    document
                        .boards
                        .iter()
                        .filter(|board| board.part_ids.contains(id))
                        .map(|board| board.id.as_str())
                        .collect::<Vec<_>>(),
                    vec![owner.as_str()],
                    "owner for {id}"
                );
            }

            let mut disposable = document.clone();
            crate::matrix::set_matrix(&mut disposable, final_matrix).unwrap();
            let rematerialized = disposable
                .matrices
                .iter()
                .find(|matrix| matrix.id == "mirror-matrix")
                .unwrap();
            assert_eq!(rematerialized, final_matrix);
            for (id, expected) in expected_members {
                assert_pose(&disposable, id, *expected);
            }
            let before_repeat = document.clone();
            assert!(
                reconcile(&mut document, "pair-source-mirror", &projection)
                    .unwrap()
                    .is_empty()
            );
            assert_eq!(document, before_repeat);
        }
    }

    #[test]
    fn fixed_membership_view_preserves_consistent_selected_cell_tombstones() {
        let (mut document, mut projection) = fixed_geometry_fixture().unwrap();
        let source_key = "matrix/source-matrix/r0c1";
        let mirror_key = "matrix/mirror-matrix/r0c1";
        for (matrix_id, key_id) in [("source-matrix", source_key), ("mirror-matrix", mirror_key)] {
            let matrix = document
                .matrices
                .iter_mut()
                .find(|matrix| matrix.id == matrix_id)
                .unwrap();
            let cell = matrix
                .cells
                .iter_mut()
                .find(|cell| (cell.row, cell.column) == (0, 1))
                .unwrap();
            cell.enabled = false;
            cell.deleted = true;
            cell.assemblies.clear();
            matrix.part_ids.retain(|id| id != key_id);
            document.parts.retain(|part| part.id != key_id);
            for board in &mut document.boards {
                board.part_ids.retain(|id| id != key_id);
            }
        }
        let halves = document.hardware.as_mut().unwrap().pairs[0]
            .halves
            .as_mut()
            .unwrap();
        halves.correspondences.retain(|association| {
            !(association.kind == MirroredItemKind::Part
                && association.source_id == source_key
                && association.mirror_id == mirror_key)
        });
        projection.parts.retain(|part| part.source_id != source_key);
        projection.final_owners.remove(source_key);
        projection.final_owners.remove(mirror_key);

        let halves = document.hardware.as_ref().unwrap().pairs[0]
            .halves
            .as_ref()
            .unwrap();
        let source = document
            .matrices
            .iter()
            .find(|matrix| matrix.id == "source-matrix")
            .unwrap();
        let mirror = document
            .matrices
            .iter()
            .find(|matrix| matrix.id == "mirror-matrix")
            .unwrap();
        let matrix_association = halves
            .correspondences
            .iter()
            .find(|association| association.kind == MirroredItemKind::Matrix)
            .unwrap();
        let projected =
            super::project_matrix(source, mirror, matrix_association, projection.mapping).unwrap();
        let view = super::build_fixed_membership_view(
            &document,
            halves,
            source,
            mirror,
            &projected,
            &projection,
        )
        .expect("matching explicit selected-cell tombstones are valid inactive coordinates");

        for matrix in [&view.normalized_source_matrix, &view.incoming_matrix] {
            let tombstone = matrix
                .cells
                .iter()
                .find(|cell| (cell.row, cell.column) == (0, 1))
                .unwrap();
            assert!(tombstone.deleted);
            assert!(!tombstone.enabled);
        }

        let source = document
            .matrices
            .iter()
            .find(|matrix| matrix.id == "source-matrix")
            .unwrap()
            .clone();
        let mut edited_source = source;
        edited_source
            .cells
            .iter_mut()
            .find(|cell| (cell.row, cell.column) == (1, 0))
            .unwrap()
            .offset = Some(Vec2 { x: 4.0, y: -1.0 });
        crate::matrix::set_matrix(&mut document, &edited_source)
            .expect("the later source edit preserves its existing active membership");
        let expected_source = document
            .matrices
            .iter()
            .find(|matrix| matrix.id == "source-matrix")
            .unwrap()
            .clone();
        super::reconcile(&mut document, "pair-source-mirror", &projection)
            .expect("ordinary reconciliation keeps the selected slot tombstoned");
        let mirror = document
            .matrices
            .iter()
            .find(|matrix| matrix.id == "mirror-matrix")
            .unwrap();
        let tombstone = mirror
            .cells
            .iter()
            .find(|cell| (cell.row, cell.column) == (0, 1))
            .unwrap();
        assert!(tombstone.deleted);
        assert!(!tombstone.enabled);
        assert!(!mirror.part_ids.contains(&mirror_key.into()));
        assert!(!document.parts.iter().any(|part| part.id == mirror_key));
        assert_eq!(
            document
                .matrices
                .iter()
                .find(|matrix| matrix.id == "source-matrix")
                .unwrap(),
            &expected_source
        );
        let after_first = document.clone();
        assert!(
            super::reconcile(&mut document, "pair-source-mirror", &projection)
                .unwrap()
                .is_empty()
        );
        assert_eq!(document, after_first);
    }

    #[test]
    fn selected_tombstone_survives_prepared_source_and_mirror_edits() {
        fn tombstone(document: &ProjectDoc, matrix_id: &str) -> crate::model::MatrixCell {
            document
                .matrices
                .iter()
                .find(|matrix| matrix.id == matrix_id)
                .unwrap()
                .cells
                .iter()
                .find(|cell| (cell.row, cell.column) == (0, 0))
                .unwrap()
                .clone()
        }

        fn assert_tombstone(document: &ProjectDoc, matrix_id: &str) {
            let matrix = document
                .matrices
                .iter()
                .find(|matrix| matrix.id == matrix_id)
                .unwrap();
            let cell = tombstone(document, matrix_id);
            assert!(!cell.enabled && cell.deleted);
            let key_id = format!("matrix/{matrix_id}/r0c0");
            assert!(!matrix.part_ids.iter().any(|id| id == &key_id));
            assert!(
                !document.parts.iter().any(|part| {
                    part.id == key_id || part.id.starts_with(&format!("{key_id}/"))
                })
            );
            assert!(!document.boards.iter().any(|board| {
                board
                    .part_ids
                    .iter()
                    .any(|id| id == &key_id || id.starts_with(&format!("{key_id}/")))
            }));
            let halves = document.hardware.as_ref().unwrap().pairs[0]
                .halves
                .as_ref()
                .unwrap();
            assert!(!halves.correspondences.iter().any(|association| {
                association.source_id == key_id
                    || association.mirror_id == key_id
                    || association.source_id.starts_with(&format!("{key_id}/"))
                    || association.mirror_id.starts_with(&format!("{key_id}/"))
            }));
        }

        let accepted = fixture_document().unwrap();
        let (removed, _) = super::prepare_transaction(
            &accepted,
            &[],
            &crate::model::EditOperation::RemoveSelectedMatrixCells {
                cells: BTreeMap::from([("source-matrix".into(), vec![(0, 0)])]),
            },
        )
        .expect("selected-cell removal stores a valid matrix tombstone");
        let removed_tombstones = [
            tombstone(&removed, "source-matrix"),
            tombstone(&removed, "mirror-matrix"),
        ];
        assert_tombstone(&removed, "source-matrix");
        assert_tombstone(&removed, "mirror-matrix");

        let source_move = crate::model::EditOperation::MoveParts {
            positions: vec![crate::model::Position {
                id: "matrix/source-matrix/r0c1".into(),
                at: Vec2 { x: 43.0, y: 17.0 },
            }],
        };
        let (source_edited, _) = super::prepare_transaction(&removed, &[], &source_move)
            .expect("later source edits reconcile around inactive membership");
        assert_tombstone(&source_edited, "source-matrix");
        assert_tombstone(&source_edited, "mirror-matrix");
        assert_eq!(
            tombstone(&source_edited, "source-matrix"),
            removed_tombstones[0]
        );
        assert_eq!(
            tombstone(&source_edited, "mirror-matrix"),
            removed_tombstones[1]
        );

        let mirror_move = crate::model::EditOperation::MoveParts {
            positions: vec![crate::model::Position {
                id: "matrix/mirror-matrix/r0c1".into(),
                at: Vec2 { x: 71.0, y: 83.0 },
            }],
        };
        let (mirror_edited, _) = super::prepare_transaction(&source_edited, &[], &mirror_move)
            .expect("later mirror edits preserve inactive membership");
        assert_tombstone(&mirror_edited, "source-matrix");
        assert_tombstone(&mirror_edited, "mirror-matrix");
        assert_eq!(
            tombstone(&mirror_edited, "source-matrix"),
            removed_tombstones[0]
        );
        assert_eq!(
            tombstone(&mirror_edited, "mirror-matrix"),
            removed_tombstones[1]
        );
        let (repeated, repeated_changes) =
            super::prepare_transaction(&mirror_edited, &[], &mirror_move)
                .expect("repeated mirror edit remains prepared");
        assert!(repeated_changes.is_empty());
        assert_eq!(repeated, mirror_edited);

        let (all_inactive, _) = super::prepare_transaction(
            &removed,
            &[],
            &crate::model::EditOperation::RemoveSelectedMatrixCells {
                cells: BTreeMap::from([("source-matrix".into(), vec![(0, 1)])]),
            },
        )
        .expect("all-slots-inactive paired matrices remain valid");
        for matrix_id in ["source-matrix", "mirror-matrix"] {
            let matrix = all_inactive
                .matrices
                .iter()
                .find(|matrix| matrix.id == matrix_id)
                .unwrap();
            assert_eq!(matrix.part_ids, Vec::<String>::new());
            assert!(
                matrix
                    .cells
                    .iter()
                    .all(|cell| !cell.enabled && cell.deleted)
            );
        }
    }

    #[test]
    fn fixed_membership_view_rejects_tombstone_mismatches_and_stale_members() {
        fn tombstone_slot(document: &mut ProjectDoc, matrix_id: &str) {
            let key_id = format!("matrix/{matrix_id}/r0c1");
            let matrix = document
                .matrices
                .iter_mut()
                .find(|matrix| matrix.id == matrix_id)
                .unwrap();
            let cell = matrix
                .cells
                .iter_mut()
                .find(|cell| (cell.row, cell.column) == (0, 1))
                .unwrap();
            cell.enabled = false;
            cell.deleted = true;
            cell.assemblies.clear();
            matrix.part_ids.retain(|id| id != &key_id);
        }

        fn fixed_view_error(document: &ProjectDoc, projection: &Projection) -> String {
            let halves = document.hardware.as_ref().unwrap().pairs[0]
                .halves
                .as_ref()
                .unwrap();
            let source = document
                .matrices
                .iter()
                .find(|matrix| matrix.id == "source-matrix")
                .unwrap();
            let mirror = document
                .matrices
                .iter()
                .find(|matrix| matrix.id == "mirror-matrix")
                .unwrap();
            let association = halves
                .correspondences
                .iter()
                .find(|association| association.kind == MirroredItemKind::Matrix)
                .unwrap();
            let projected =
                super::project_matrix(source, mirror, association, projection.mapping).unwrap();
            match super::build_fixed_membership_view(
                document, halves, source, mirror, &projected, projection,
            ) {
                Err(error) => error,
                Ok(_) => panic!("expected malformed fixed membership to be refused"),
            }
        }

        let (mut document, projection) = fixed_geometry_fixture().unwrap();
        tombstone_slot(&mut document, "source-matrix");
        assert!(fixed_view_error(&document, &projection).contains("inconsistent inactive slots"));

        let (mut document, projection) = fixed_geometry_fixture().unwrap();
        tombstone_slot(&mut document, "mirror-matrix");
        assert!(fixed_view_error(&document, &projection).contains("inconsistent inactive slots"));

        fn clean_tombstone_fixture() -> (ProjectDoc, Projection) {
            let (mut document, mut projection) = fixed_geometry_fixture().unwrap();
            tombstone_slot(&mut document, "source-matrix");
            tombstone_slot(&mut document, "mirror-matrix");
            let source_id = "matrix/source-matrix/r0c1";
            let mirror_id = "matrix/mirror-matrix/r0c1";
            document
                .parts
                .retain(|part| part.id != source_id && part.id != mirror_id);
            for board in &mut document.boards {
                board
                    .part_ids
                    .retain(|id| id != source_id && id != mirror_id);
            }
            document.hardware.as_mut().unwrap().pairs[0]
                .halves
                .as_mut()
                .unwrap()
                .correspondences
                .retain(|association| {
                    !(association.kind == MirroredItemKind::Part
                        && association.source_id == source_id
                        && association.mirror_id == mirror_id)
                });
            projection.parts.retain(|part| part.source_id != source_id);
            projection.final_owners.remove(source_id);
            projection.final_owners.remove(mirror_id);
            (document, projection)
        }

        for stale_kind in ["part", "board owner", "owner input", "correspondence"] {
            let (mut document, mut projection) = clean_tombstone_fixture();
            let source_id = "matrix/source-matrix/r0c1";
            let mirror_id = "matrix/mirror-matrix/r0c1";
            match stale_kind {
                "part" => {
                    let mut part = document
                        .parts
                        .iter()
                        .find(|part| part.id == "matrix/mirror-matrix/r0c2")
                        .unwrap()
                        .clone();
                    part.id = mirror_id.into();
                    document.parts.push(part);
                }
                "board owner" => document
                    .boards
                    .iter_mut()
                    .find(|board| board.id == "mirror-board")
                    .unwrap()
                    .part_ids
                    .push(mirror_id.into()),
                "owner input" => {
                    projection
                        .final_owners
                        .insert(mirror_id.into(), "mirror-board".into());
                }
                "correspondence" => document.hardware.as_mut().unwrap().pairs[0]
                    .halves
                    .as_mut()
                    .unwrap()
                    .correspondences
                    .push(MirroredCorrespondence {
                        companion_mode: None,
                        companion_tweak: None,
                        kind: MirroredItemKind::Part,
                        source_id: source_id.into(),
                        mirror_id: mirror_id.into(),
                    }),
                _ => unreachable!(),
            }
            assert!(
                fixed_view_error(&document, &projection)
                    .contains("materialized or associated members"),
                "stale {stale_kind} was not refused"
            );
        }

        let (mut document, projection) = fixed_geometry_fixture().unwrap();
        let source_cell = document
            .matrices
            .iter_mut()
            .find(|matrix| matrix.id == "source-matrix")
            .unwrap()
            .cells
            .iter_mut()
            .find(|cell| (cell.row, cell.column) == (0, 1))
            .unwrap();
        source_cell.enabled = false;
        source_cell.deleted = false;
        assert!(fixed_view_error(&document, &projection).contains("fixed active membership"));
    }

    #[test]
    fn fixed_membership_view_accepts_unassociated_local_or_omitted_matrix_children() {
        for state in [
            MirroredExceptionState::Local,
            MirroredExceptionState::Omitted,
        ] {
            let (document, projection) = fixed_membership_assembly_fixture(90.0).unwrap();
            let source_id = "matrix/source-matrix/r0c1/diode";
            let mirror_id = "matrix/mirror-matrix/r0c1/diode";
            let mut document = document;
            let halves = document.hardware.as_mut().unwrap().pairs[0]
                .halves
                .as_mut()
                .unwrap();
            halves.correspondences.retain(|association| {
                !(association.kind == MirroredItemKind::Part
                    && association.source_id == source_id
                    && association.mirror_id == mirror_id)
            });
            halves.exceptions.push(MirroredException {
                companion_mode: Some(MirroredCompanionMode::LiteralMirrorWithTweak),
                companion_tweak: Some(MirroredCompanionTweak {
                    x: 1.0,
                    y: -2.0,
                    rotation: 15.0,
                }),
                kind: MirroredItemKind::Part,
                mirror_id: mirror_id.into(),
                state,
                removed_copper: None,
            });
            crate::mechanical::halves::validate_document(&document)
                .expect("the unassociated exception names an existing matrix child");

            let halves = document.hardware.as_ref().unwrap().pairs[0]
                .halves
                .as_ref()
                .unwrap();
            let source = document
                .matrices
                .iter()
                .find(|matrix| matrix.id == "source-matrix")
                .unwrap();
            let mirror = document
                .matrices
                .iter()
                .find(|matrix| matrix.id == "mirror-matrix")
                .unwrap();
            let matrix_association = halves
                .correspondences
                .iter()
                .find(|association| association.kind == MirroredItemKind::Matrix)
                .unwrap();
            let projected =
                super::project_matrix(source, mirror, matrix_association, projection.mapping)
                    .unwrap();
            let view = super::build_fixed_membership_view(
                &document,
                halves,
                source,
                mirror,
                &projected,
                &projection,
            )
            .expect("the exact matrix slot authenticates its unassociated child exception");
            let incoming_cell = view
                .incoming_matrix
                .cells
                .iter()
                .find(|cell| (cell.row, cell.column) == (0, 1))
                .unwrap();
            assert!(
                incoming_cell
                    .assemblies
                    .iter()
                    .any(|assembly| assembly.id == "diode")
            );
            let legacy_part = document
                .parts
                .iter()
                .find(|part| part.id == mirror_id)
                .unwrap()
                .clone();
            let pair_before =
                serde_json::to_vec(&document.hardware.as_ref().unwrap().pairs[0]).unwrap();
            super::reconcile(&mut document, "pair-source-mirror", &projection)
                .unwrap_or_else(|error| panic!("{state:?} legacy exception: {error}"));
            assert_eq!(
                document.parts.iter().find(|part| part.id == mirror_id),
                Some(&legacy_part)
            );
            let exception = document.hardware.as_ref().unwrap().pairs[0]
                .halves
                .as_ref()
                .unwrap()
                .exceptions
                .iter()
                .find(|exception| exception.mirror_id == mirror_id)
                .unwrap();
            assert_eq!(exception.state, state);
            assert_eq!(
                exception.companion_mode,
                Some(MirroredCompanionMode::LiteralMirrorWithTweak)
            );
            assert_eq!(
                exception.companion_tweak,
                Some(MirroredCompanionTweak {
                    x: 1.0,
                    y: -2.0,
                    rotation: 15.0,
                })
            );
            let pair_after =
                serde_json::to_vec(&document.hardware.as_ref().unwrap().pairs[0]).unwrap();
            assert_eq!(pair_after, pair_before);
            let first = document.clone();
            assert!(
                super::reconcile(&mut document, "pair-source-mirror", &projection)
                    .unwrap()
                    .is_empty()
            );
            assert_eq!(document, first);
        }
    }

    #[test]
    fn fixed_membership_view_preserves_authored_cell_defaults_and_variants() {
        let (mut document, projection) = fixed_membership_assembly_fixture(90.0).unwrap();
        let matrix_default = document
            .matrices
            .iter()
            .find(|matrix| matrix.id == "source-matrix")
            .unwrap()
            .definition_id
            .clone();
        let mut alternate_definition = document.definitions[0].clone();
        alternate_definition.id = "alternate-switch".into();
        document.definitions.push(alternate_definition);
        {
            let source_matrix = document
                .matrices
                .iter_mut()
                .find(|matrix| matrix.id == "source-matrix")
                .unwrap();
            let cell = source_matrix
                .cells
                .iter_mut()
                .find(|cell| (cell.row, cell.column) == (0, 1))
                .unwrap();
            // The definition override is redundant today but remains an authored per-cell
            // choice if the matrix default changes later.
            cell.definition_id = Some(matrix_default.clone());
            cell.variant = Some("preset/authored-red".into());
            cell.assemblies_local = Some(false);
            source_matrix
                .cells
                .iter_mut()
                .find(|cell| (cell.row, cell.column) == (0, 2))
                .unwrap()
                .definition_id = Some("alternate-switch".into());
        }
        let source_matrix = document
            .matrices
            .iter()
            .find(|matrix| matrix.id == "source-matrix")
            .unwrap()
            .clone();
        crate::matrix::set_matrix(&mut document, &source_matrix)
            .expect("the source materializes its explicit and alternate definitions");

        let halves = document.hardware.as_ref().unwrap().pairs[0]
            .halves
            .as_ref()
            .unwrap();
        let source = document
            .matrices
            .iter()
            .find(|matrix| matrix.id == "source-matrix")
            .unwrap();
        let mirror = document
            .matrices
            .iter()
            .find(|matrix| matrix.id == "mirror-matrix")
            .unwrap();
        let correspondence = halves
            .correspondences
            .iter()
            .find(|association| association.kind == MirroredItemKind::Matrix)
            .unwrap();
        let projected =
            super::project_matrix(source, mirror, correspondence, projection.mapping).unwrap();
        let view = super::build_fixed_membership_view(
            &document,
            halves,
            source,
            mirror,
            &projected,
            &projection,
        )
        .expect("authored cell fields are valid reconciliation input");

        let cell = view
            .incoming_matrix
            .cells
            .iter()
            .find(|cell| (cell.row, cell.column) == (0, 1))
            .unwrap();
        assert_eq!(cell.definition_id.as_deref(), Some(matrix_default.as_str()));
        assert_eq!(cell.variant.as_deref(), Some("preset/authored-red"));
        assert_eq!(cell.assemblies_local, Some(false));
        let alternate = view
            .incoming_matrix
            .cells
            .iter()
            .find(|cell| (cell.row, cell.column) == (0, 2))
            .unwrap();
        assert_eq!(alternate.definition_id.as_deref(), Some("alternate-switch"));

        let source_before_reconcile = document
            .matrices
            .iter()
            .find(|matrix| matrix.id == "source-matrix")
            .unwrap()
            .clone();
        super::reconcile(&mut document, "pair-source-mirror", &projection)
            .expect("authored cell definitions materialize through reconciliation");
        assert_eq!(
            document
                .matrices
                .iter()
                .find(|matrix| matrix.id == "source-matrix")
                .unwrap(),
            &source_before_reconcile,
            "reconciliation preserves the complete authored source matrix"
        );
        for (matrix_id, coordinate, expected) in [
            ("source-matrix", (0, 1), matrix_default.as_str()),
            ("mirror-matrix", (0, 1), matrix_default.as_str()),
            ("source-matrix", (0, 2), "alternate-switch"),
            ("mirror-matrix", (0, 2), "alternate-switch"),
        ] {
            let key_id = crate::matrix::member_id(matrix_id, coordinate.0, coordinate.1);
            let key = document
                .parts
                .iter()
                .find(|part| part.id == key_id)
                .unwrap_or_else(|| panic!("generated key {key_id} exists"));
            assert_eq!(key.definition_id, expected, "{key_id}");
        }
    }

    #[test]
    fn fixed_membership_view_fills_only_missing_active_cells_with_defaults() {
        let (mut document, projection) = fixed_geometry_fixture().unwrap();
        for matrix_id in ["source-matrix", "mirror-matrix"] {
            document
                .matrices
                .iter_mut()
                .find(|matrix| matrix.id == matrix_id)
                .unwrap()
                .cells
                .retain(|cell| (cell.row, cell.column) != (0, 1));
        }

        let halves = document.hardware.as_ref().unwrap().pairs[0]
            .halves
            .as_ref()
            .unwrap();
        let source = document
            .matrices
            .iter()
            .find(|matrix| matrix.id == "source-matrix")
            .unwrap();
        let mirror = document
            .matrices
            .iter()
            .find(|matrix| matrix.id == "mirror-matrix")
            .unwrap();
        let correspondence = halves
            .correspondences
            .iter()
            .find(|association| association.kind == MirroredItemKind::Matrix)
            .unwrap();
        let projected =
            super::project_matrix(source, mirror, correspondence, projection.mapping).unwrap();
        let view = super::build_fixed_membership_view(
            &document,
            halves,
            source,
            mirror,
            &projected,
            &projection,
        )
        .expect("an absent active cell uses the matrix materializer defaults");

        let normalized_source_cell = view
            .normalized_source_matrix
            .cells
            .iter()
            .find(|cell| (cell.row, cell.column) == (0, 1))
            .unwrap();
        assert!(normalized_source_cell.enabled);
        assert!(!normalized_source_cell.deleted);
        assert_eq!(normalized_source_cell.definition_id, None);
        assert_eq!(view.normalized_source_matrix.cells.len(), 6);

        let cell = view
            .incoming_matrix
            .cells
            .iter()
            .find(|cell| (cell.row, cell.column) == (0, 1))
            .unwrap();
        assert!(cell.enabled);
        assert!(!cell.deleted);
        assert_eq!(cell.definition_id, None);
        assert_eq!(cell.variant, None);
        assert_eq!(cell.offset, None);
        assert_eq!(cell.rotation, None);
        assert!(cell.assemblies.is_empty());
        assert_eq!(cell.assemblies_local, None);
        assert_eq!(view.incoming_matrix.cells.len(), 6);

        super::reconcile(&mut document, "pair-source-mirror", &projection)
            .expect("both halves materialize the missing active cell");
        let first = document.clone();
        for matrix_id in ["source-matrix", "mirror-matrix"] {
            let matrix = document
                .matrices
                .iter()
                .find(|matrix| matrix.id == matrix_id)
                .unwrap();
            assert_eq!(matrix.cells.len(), 6);
            let filled = matrix
                .cells
                .iter()
                .find(|cell| (cell.row, cell.column) == (0, 1))
                .unwrap();
            assert!(filled.enabled);
            assert!(!filled.deleted);
            assert_eq!(filled.definition_id, None);
            assert_eq!(filled.variant, None);
            assert_eq!(filled.assemblies_local, None);
        }
        let repeated = super::reconcile(&mut document, "pair-source-mirror", &projection)
            .expect("repeated sparse reconciliation is stable");
        assert_eq!(document, first);
        assert!(
            repeated.is_empty(),
            "repeated reconciliation reported {repeated:?}"
        );
    }

    #[test]
    fn source_and_mirror_local_assemblies_have_no_correspondence_and_survive_reconcile() {
        let (mut document, projection) = fixed_membership_assembly_fixture(90.0).unwrap();
        let mut target_definition = document.definitions[0].clone();
        target_definition.id = "target-local-switch".into();
        document.definitions.push(target_definition);
        for matrix_id in ["source-matrix", "mirror-matrix"] {
            let cell = document
                .matrices
                .iter_mut()
                .find(|matrix| matrix.id == matrix_id)
                .unwrap()
                .cells
                .iter_mut()
                .find(|cell| (cell.row, cell.column) == (0, 0))
                .unwrap();
            cell.assemblies_local = Some(true);
            cell.variant = Some(if matrix_id == "source-matrix" {
                "source-local-variant".into()
            } else {
                "target-local-variant".into()
            });
            if matrix_id == "mirror-matrix" {
                cell.definition_id = Some("target-local-switch".into());
            }
        }
        let mut source_matrix = document
            .matrices
            .iter()
            .find(|matrix| matrix.id == "source-matrix")
            .unwrap()
            .clone();
        source_matrix.cells[0].assemblies.push(MatrixAssembly {
            id: "source-only".into(),
            definition_id: "diode".into(),
            offset: Vec2 { x: 9.0, y: 1.0 },
            rotation: Some(0.0),
            side: Some(Side::Back),
        });
        crate::matrix::set_matrix(&mut document, &source_matrix).unwrap();
        let target_matrix = document
            .matrices
            .iter()
            .find(|matrix| matrix.id == "mirror-matrix")
            .unwrap()
            .clone();
        crate::matrix::set_matrix(&mut document, &target_matrix).unwrap();
        let mut projection = projection;
        projection.final_owners.insert(
            "matrix/source-matrix/r0c0/source-only".into(),
            "source-board".into(),
        );
        document.hardware.as_mut().unwrap().pairs[0]
            .halves
            .as_mut()
            .unwrap()
            .correspondences
            .retain(|association| {
                association.source_id != "matrix/source-matrix/r0c0/diode"
                    && association.mirror_id != "matrix/mirror-matrix/r0c0/diode"
            });
        let mirror_assembly = document
            .matrices
            .iter()
            .find(|matrix| matrix.id == "mirror-matrix")
            .unwrap()
            .cells[0]
            .assemblies[0]
            .clone();
        let target_cell = document
            .matrices
            .iter()
            .find(|matrix| matrix.id == "mirror-matrix")
            .unwrap()
            .cells
            .iter()
            .find(|cell| (cell.row, cell.column) == (0, 0))
            .unwrap()
            .clone();
        let source_assemblies = document
            .matrices
            .iter()
            .find(|matrix| matrix.id == "source-matrix")
            .unwrap()
            .cells[0]
            .assemblies
            .clone();
        let mirror_part = document
            .parts
            .iter()
            .find(|part| part.id == "matrix/mirror-matrix/r0c0/diode")
            .unwrap()
            .clone();

        reconcile(&mut document, "pair-source-mirror", &projection)
            .expect("independent local companion lists do not need pair associations");

        assert_eq!(
            document
                .matrices
                .iter()
                .find(|matrix| matrix.id == "source-matrix")
                .unwrap()
                .cells[0]
                .assemblies,
            source_assemblies
        );
        let source_cell = document
            .matrices
            .iter()
            .find(|matrix| matrix.id == "source-matrix")
            .unwrap()
            .cells
            .iter()
            .find(|cell| (cell.row, cell.column) == (0, 0))
            .unwrap();
        assert_eq!(source_cell.definition_id, None);
        assert_eq!(source_cell.variant.as_deref(), Some("source-local-variant"));
        let source_key = document
            .parts
            .iter()
            .find(|part| part.id == "matrix/source-matrix/r0c0")
            .unwrap();
        assert_eq!(source_key.definition_id, "switch");
        assert_eq!(
            source_key
                .properties
                .as_ref()
                .and_then(|properties| properties.get("variant")),
            Some(&serde_json::json!("source-local-variant"))
        );
        assert_eq!(
            document
                .matrices
                .iter()
                .find(|matrix| matrix.id == "mirror-matrix")
                .unwrap()
                .cells[0]
                .assemblies[0],
            mirror_assembly
        );
        let mirrored_cell = document
            .matrices
            .iter()
            .find(|matrix| matrix.id == "mirror-matrix")
            .unwrap()
            .cells
            .iter()
            .find(|cell| (cell.row, cell.column) == (0, 0))
            .unwrap();
        assert_eq!(mirrored_cell.definition_id, target_cell.definition_id);
        assert_eq!(mirrored_cell.variant, target_cell.variant);
        assert_eq!(mirrored_cell.assemblies_local, target_cell.assemblies_local);
        assert_eq!(mirrored_cell.assemblies, target_cell.assemblies);
        assert_eq!(
            mirrored_cell.definition_id.as_deref(),
            Some("target-local-switch")
        );
        assert_eq!(
            mirrored_cell.variant.as_deref(),
            Some("target-local-variant")
        );
        let target_key = document
            .parts
            .iter()
            .find(|part| part.id == "matrix/mirror-matrix/r0c0")
            .unwrap();
        assert_eq!(target_key.definition_id, "target-local-switch");
        assert_eq!(
            target_key
                .properties
                .as_ref()
                .and_then(|properties| properties.get("variant")),
            Some(&serde_json::json!("target-local-variant"))
        );
        assert_eq!(
            document
                .parts
                .iter()
                .find(|part| part.id == mirror_part.id)
                .unwrap(),
            &mirror_part
        );
        let after_first = document.clone();
        assert!(
            reconcile(&mut document, "pair-source-mirror", &projection)
                .unwrap()
                .is_empty()
        );
        assert_eq!(document, after_first);
    }

    #[test]
    fn fixed_membership_assembly_rejections_are_atomic() {
        fn assert_rejected(mut document: ProjectDoc, projection: &Projection, reason: &str) {
            let before = serde_json::to_value(&document).unwrap();
            let Err(error) = reconcile(&mut document, "pair-source-mirror", projection) else {
                panic!("expected atomic rejection containing {reason:?}");
            };
            assert!(error.contains(reason), "expected {reason:?}, got {error:?}");
            assert_eq!(serde_json::to_value(&document).unwrap(), before);
        }

        for (part_id, authored) in [
            ("matrix/mirror-matrix/r0c1", true),
            ("matrix/source-matrix/r0c1", true),
            ("matrix/mirror-matrix/r0c1", false),
            ("matrix/source-matrix/r0c1", false),
        ] {
            let (mut document, projection) = fixed_membership_assembly_fixture(90.0).unwrap();
            if authored {
                document
                    .parts
                    .iter_mut()
                    .find(|part| part.id == part_id)
                    .unwrap()
                    .properties
                    .as_mut()
                    .unwrap()
                    .insert("layoutOverride".into(), serde_json::json!(1));
            } else {
                document.constraints.push(Constraint::Offset {
                    id: "linked-key-constraint".into(),
                    source_part_id: "assembly-probe".into(),
                    target_part_id: part_id.into(),
                    offset: Vec2 { x: 1.0, y: 2.0 },
                    rotation: 0.0,
                });
            }
            assert_rejected(
                document,
                &projection,
                "Linked matrix keys cannot have authored or constrained placement",
            );
        }

        let (mut document, projection) = fixed_membership_assembly_fixture(90.0).unwrap();
        document
            .parts
            .retain(|part| part.id != "matrix/source-matrix/r0c0/diode");
        assert_rejected(document, &projection, "unavailable");

        let (mut document, projection) = fixed_membership_assembly_fixture(90.0).unwrap();
        document.hardware.as_mut().unwrap().pairs[0]
            .halves
            .as_mut()
            .unwrap()
            .correspondences
            .retain(|association| association.source_id != "matrix/source-matrix/r0c0/diode");
        assert_rejected(document, &projection, "companion association is missing");

        let (mut document, projection) = fixed_membership_assembly_fixture(90.0).unwrap();
        let association = document.hardware.as_mut().unwrap().pairs[0]
            .halves
            .as_mut()
            .unwrap()
            .correspondences
            .iter_mut()
            .find(|association| association.source_id == "matrix/source-matrix/r0c0/diode")
            .unwrap();
        association.source_id = "matrix/mirror-matrix/r0c0/diode".into();
        association.mirror_id = "matrix/source-matrix/r0c0/diode".into();
        assert_rejected(
            document,
            &projection,
            "companion association is aliased or inconsistent",
        );

        let (mut document, projection) = fixed_membership_assembly_fixture(90.0).unwrap();
        let association = document.hardware.as_mut().unwrap().pairs[0]
            .halves
            .as_mut()
            .unwrap()
            .correspondences
            .iter()
            .find(|association| association.source_id == "matrix/source-matrix/r0c0/diode")
            .unwrap()
            .clone();
        document.hardware.as_mut().unwrap().pairs[0]
            .halves
            .as_mut()
            .unwrap()
            .correspondences
            .push(association);
        assert_rejected(
            document,
            &projection,
            "companion association is aliased or inconsistent",
        );

        let (mut document, projection) = fixed_membership_assembly_fixture(90.0).unwrap();
        let association = document.hardware.as_mut().unwrap().pairs[0]
            .halves
            .as_mut()
            .unwrap()
            .correspondences
            .iter_mut()
            .find(|association| association.source_id == "matrix/source-matrix/r0c0/diode")
            .unwrap();
        association.mirror_id = "matrix/mirror-matrix/r0c1/diode".into();
        assert_rejected(
            document,
            &projection,
            "companion association is aliased or inconsistent",
        );

        let (document, mut projection) = fixed_membership_assembly_fixture(90.0).unwrap();
        projection
            .final_owners
            .remove("matrix/mirror-matrix/r1c1/diode");
        assert_rejected(document, &projection, "final owner input is incomplete");

        let (mut document, projection) = fixed_membership_assembly_fixture(90.0).unwrap();
        document
            .boards
            .iter_mut()
            .find(|board| board.id == "mirror-board")
            .unwrap()
            .part_ids
            .retain(|id| id != "matrix/mirror-matrix/r1c1/diode");
        assert_rejected(document, &projection, "no board owner");

        let (mut document, projection) = fixed_membership_assembly_fixture(90.0).unwrap();
        document
            .boards
            .iter_mut()
            .find(|board| board.id == "source-board")
            .unwrap()
            .part_ids
            .push("matrix/mirror-matrix/r1c1/diode".into());
        assert_rejected(document, &projection, "ambiguous board ownership");

        let (mut document, projection) = fixed_membership_assembly_fixture(90.0).unwrap();
        document
            .boards
            .iter_mut()
            .find(|board| board.id == "mirror-board")
            .unwrap()
            .part_ids
            .retain(|id| id != "matrix/mirror-matrix/r0c1/diode");
        document
            .boards
            .iter_mut()
            .find(|board| board.id == "source-board")
            .unwrap()
            .part_ids
            .push("matrix/mirror-matrix/r0c1/diode".into());
        assert_rejected(
            document,
            &projection,
            "companion has an incomplete or stale owner input",
        );

        let (mut document, mut projection) = fixed_membership_assembly_fixture(90.0).unwrap();
        let matrix = document
            .matrices
            .iter_mut()
            .find(|matrix| matrix.id == "source-matrix")
            .unwrap();
        matrix
            .cells
            .iter_mut()
            .find(|cell| (cell.row, cell.column) == (0, 0))
            .unwrap()
            .assemblies
            .clear();
        matrix
            .part_ids
            .retain(|id| id != "matrix/source-matrix/r0c0/diode");
        document
            .parts
            .retain(|part| part.id != "matrix/source-matrix/r0c0/diode");
        document
            .boards
            .iter_mut()
            .find(|board| board.id == "source-board")
            .unwrap()
            .part_ids
            .retain(|id| id != "matrix/source-matrix/r0c0/diode");
        document.hardware.as_mut().unwrap().pairs[0]
            .halves
            .as_mut()
            .unwrap()
            .correspondences
            .retain(|association| association.source_id != "matrix/source-matrix/r0c0/diode");
        projection
            .final_owners
            .remove("matrix/source-matrix/r0c0/diode");
        assert_rejected(document, &projection, "membership is not canonical");

        let (mut document, projection) = fixed_membership_assembly_fixture(90.0).unwrap();
        document
            .matrices
            .iter_mut()
            .find(|matrix| matrix.id == "source-matrix")
            .unwrap()
            .cells[0]
            .assemblies[0]
            .offset
            .x = f64::NAN;
        assert_rejected(document, &projection, "Matrix assembly is invalid");

        let (mut document, projection) = fixed_membership_assembly_fixture(90.0).unwrap();
        let association = document.hardware.as_mut().unwrap().pairs[0]
            .halves
            .as_mut()
            .unwrap()
            .correspondences
            .iter_mut()
            .find(|association| association.source_id == "matrix/source-matrix/r1c0/diode")
            .unwrap();
        association.companion_tweak.as_mut().unwrap().rotation = f64::NAN;
        assert_rejected(document, &projection, "finite");

        let (mut document, projection) = fixed_membership_assembly_fixture(90.0).unwrap();
        let mut source = document
            .matrices
            .iter()
            .find(|matrix| matrix.id == "source-matrix")
            .unwrap()
            .clone();
        source
            .cells
            .iter_mut()
            .find(|cell| (cell.row, cell.column) == (1, 0))
            .unwrap()
            .assemblies[0]
            .offset
            .x = f64::MAX * 0.9;
        crate::matrix::set_matrix(&mut document, &source).unwrap();
        let association = document.hardware.as_mut().unwrap().pairs[0]
            .halves
            .as_mut()
            .unwrap()
            .correspondences
            .iter_mut()
            .find(|association| association.source_id == "matrix/source-matrix/r1c0/diode")
            .unwrap();
        association.companion_tweak.as_mut().unwrap().x = -f64::MAX * 0.9;
        assert_rejected(document, &projection, "non-finite values");

        let (mut document, projection) = fixed_membership_assembly_fixture(90.0).unwrap();
        document.constraints.push(Constraint::Offset {
            id: "linked-companion-constraint".into(),
            source_part_id: "assembly-probe".into(),
            target_part_id: "matrix/mirror-matrix/r0c1/diode".into(),
            offset: Vec2 { x: 1.0, y: 2.0 },
            rotation: 0.0,
        });
        assert_rejected(document, &projection, "authored or constrained placement");

        let (mut document, projection) = fixed_membership_assembly_fixture(90.0).unwrap();
        let companion = document
            .parts
            .iter_mut()
            .find(|part| part.id == "matrix/mirror-matrix/r0c1/diode")
            .unwrap();
        companion
            .properties
            .get_or_insert_with(BTreeMap::new)
            .insert("layoutOverride".into(), serde_json::json!(1));
        assert_rejected(document, &projection, "authored or constrained placement");
    }

    #[test]
    fn fixed_membership_view_rejects_legacy_exception_aliases_and_foreign_owners() {
        fn legacy_child_fixture() -> (ProjectDoc, Projection, String) {
            let (mut document, projection) = fixed_membership_assembly_fixture(90.0).unwrap();
            let source_id = "matrix/source-matrix/r0c1/diode";
            let mirror_id = "matrix/mirror-matrix/r0c1/diode".to_owned();
            let halves = document.hardware.as_mut().unwrap().pairs[0]
                .halves
                .as_mut()
                .unwrap();
            halves.correspondences.retain(|association| {
                !(association.kind == MirroredItemKind::Part
                    && association.source_id == source_id
                    && association.mirror_id == mirror_id)
            });
            halves.exceptions.push(MirroredException {
                companion_mode: Some(MirroredCompanionMode::LiteralMirror),
                companion_tweak: None,
                kind: MirroredItemKind::Part,
                mirror_id: mirror_id.clone(),
                state: MirroredExceptionState::Local,
                removed_copper: None,
            });
            (document, projection, mirror_id)
        }

        fn assert_rejected(mut document: ProjectDoc, projection: &Projection, reason: &str) {
            let before = serde_json::to_value(&document).unwrap();
            let Err(error) = reconcile(&mut document, "pair-source-mirror", projection) else {
                panic!("expected atomic rejection containing {reason:?}");
            };
            assert!(error.contains(reason), "expected {reason:?}, got {error:?}");
            assert_eq!(serde_json::to_value(&document).unwrap(), before);
        }

        let (mut document, projection, _) = legacy_child_fixture();
        document.hardware.as_mut().unwrap().pairs[0]
            .halves
            .as_mut()
            .unwrap()
            .exceptions[0]
            .mirror_id = "matrix/mirror-matrix/r0c2/diode".into();
        assert_rejected(document, &projection, "companion association is missing");

        let (mut document, projection, _) = legacy_child_fixture();
        let halves = document.hardware.as_mut().unwrap().pairs[0]
            .halves
            .as_mut()
            .unwrap();
        halves.exceptions[0].kind = MirroredItemKind::Net;
        assert_rejected(
            document,
            &projection,
            "companion exception is aliased or inconsistent",
        );

        let (mut document, projection, _) = legacy_child_fixture();
        let source_id = "matrix/source-matrix/r0c1/diode";
        let mirror_id = "matrix/mirror-matrix/r0c1/diode";
        let halves = document.hardware.as_mut().unwrap().pairs[0]
            .halves
            .as_mut()
            .unwrap();
        halves.correspondences.push(MirroredCorrespondence {
            companion_mode: None,
            companion_tweak: None,
            kind: MirroredItemKind::Part,
            source_id: mirror_id.into(),
            mirror_id: source_id.into(),
        });
        assert_rejected(
            document,
            &projection,
            "companion association is aliased or inconsistent",
        );

        let (mut document, projection, _mirror_id) = legacy_child_fixture();
        let exception = document.hardware.as_mut().unwrap().pairs[0]
            .halves
            .as_mut()
            .unwrap()
            .exceptions[0]
            .clone();
        document.hardware.as_mut().unwrap().pairs[0]
            .halves
            .as_mut()
            .unwrap()
            .exceptions
            .push(exception);
        assert_rejected(
            document,
            &projection,
            "companion exception is aliased or inconsistent",
        );
        let _ = mirror_id;

        let (mut document, projection, mirror_id) = legacy_child_fixture();
        document
            .boards
            .iter_mut()
            .find(|board| board.id == "mirror-board")
            .unwrap()
            .part_ids
            .retain(|id| id != &mirror_id);
        document
            .boards
            .iter_mut()
            .find(|board| board.id == "source-board")
            .unwrap()
            .part_ids
            .push(mirror_id);
        assert_rejected(document, &projection, "incomplete or stale owner input");
    }

    #[test]
    fn fixed_membership_geometry_reconciles_multiple_retained_keys() {
        let (mut document, projection) = fixed_geometry_fixture().unwrap();
        let source = document
            .matrices
            .iter()
            .find(|matrix| matrix.id == "source-matrix")
            .unwrap();
        let mirror = document
            .matrices
            .iter()
            .find(|matrix| matrix.id == "mirror-matrix")
            .unwrap();
        crate::matrix::valid_matrix(source, &document).unwrap();
        crate::matrix::valid_matrix(mirror, &document).unwrap();
        crate::mechanical::features::validate(&document).unwrap();
        crate::mechanical::halves::validate_document(&document).unwrap();
        assert_eq!(source.part_ids.len(), 6);
        assert_eq!(mirror.part_ids.len(), 6);
        assert_eq!(projection.parts.len(), 4);
        assert_eq!(projection.locally_protected_mirror_parts.len(), 2);
        assert_eq!(projection.final_owners.len(), 12);
        assert_eq!(
            document.hardware.as_ref().unwrap().pairs[0]
                .halves
                .as_ref()
                .unwrap()
                .correspondences
                .iter()
                .filter(|association| association.kind == MirroredItemKind::Part)
                .count(),
            6
        );
        for (id, expected) in [
            ("matrix/source-matrix/r0c0", (7.0, -1.0, 90.0)),
            ("matrix/source-matrix/r0c1", (8.0, 13.0, 0.0)),
            ("matrix/source-matrix/r0c2", (8.0, 5.0, 180.0)),
            ("matrix/source-matrix/r1c0", (-15.0, 0.0, 90.0)),
            ("matrix/source-matrix/r1c1", (9.0, 35.0, 0.0)),
            ("matrix/source-matrix/r1c2", (-17.0, 4.0, 90.0)),
        ] {
            let part = document.parts.iter().find(|part| part.id == id).unwrap();
            assert!((part.pose.at.x - expected.0).abs() < 1e-9, "{id} x");
            assert!((part.pose.at.y - expected.1).abs() < 1e-9, "{id} y");
            assert!(
                (part.pose.rotation - expected.2).abs() < 1e-9,
                "{id} rotation"
            );
        }

        let entry = document.clone();
        let entry_source_matrix = source.clone();
        let entry_source_parts: Vec<_> = entry_source_matrix
            .part_ids
            .iter()
            .map(|id| {
                serde_json::to_value(entry.parts.iter().find(|part| part.id == *id).unwrap())
                    .unwrap()
            })
            .collect();
        let entry_mirror_matrix = entry
            .matrices
            .iter()
            .find(|matrix| matrix.id == "mirror-matrix")
            .unwrap();
        let affected_part_ids: Vec<_> = entry_source_matrix
            .part_ids
            .iter()
            .chain(&entry_mirror_matrix.part_ids)
            .cloned()
            .collect();
        let non_pose = |part: &Part| {
            let mut value = serde_json::to_value(part).unwrap();
            value.as_object_mut().unwrap().remove("pose");
            value
        };
        let entry_non_pose: Vec<_> = affected_part_ids
            .iter()
            .map(|id| non_pose(entry.parts.iter().find(|part| part.id == *id).unwrap()))
            .collect();
        let retained_ids = ["matrix/mirror-matrix/r0c0", "matrix/mirror-matrix/r1c1"];
        let retained_parts: Vec<_> = retained_ids
            .iter()
            .map(|id| {
                serde_json::to_value(entry.parts.iter().find(|part| part.id == *id).unwrap())
                    .unwrap()
            })
            .collect();
        let unchanged_references = serde_json::json!({
            "boards": entry.boards,
            "nets": entry.nets,
            "constraints": entry.constraints,
            "boardOutlines": entry.board_outlines,
            "outline": entry.outline,
        });

        let changed_ids = reconcile(&mut document, "pair-source-mirror", &projection).unwrap();
        assert_eq!(
            changed_ids,
            vec![
                "matrix/mirror-matrix/r0c1",
                "matrix/mirror-matrix/r0c2",
                "matrix/mirror-matrix/r1c0",
                "matrix/mirror-matrix/r1c2",
                "mirror-matrix",
            ]
        );
        let source_after = document
            .matrices
            .iter()
            .find(|matrix| matrix.id == "source-matrix")
            .unwrap();
        assert_eq!(source_after, &entry_source_matrix);
        let source_parts_after: Vec<_> = entry_source_matrix
            .part_ids
            .iter()
            .map(|id| {
                serde_json::to_value(document.parts.iter().find(|part| part.id == *id).unwrap())
                    .unwrap()
            })
            .collect();
        assert_eq!(source_parts_after, entry_source_parts);
        let non_pose_after: Vec<_> = affected_part_ids
            .iter()
            .map(|id| non_pose(document.parts.iter().find(|part| part.id == *id).unwrap()))
            .collect();
        assert_eq!(non_pose_after, entry_non_pose);

        let expected_final = [
            ("matrix/mirror-matrix/r0c0", (0.0, 0.0, 0.0)),
            ("matrix/mirror-matrix/r0c1", (87.0, -38.0, 90.0)),
            ("matrix/mirror-matrix/r0c2", (95.0, -38.0, -90.0)),
            ("matrix/mirror-matrix/r1c0", (100.0, -15.0, 0.0)),
            ("matrix/mirror-matrix/r1c1", (10.0, 20.0, 0.0)),
            ("matrix/mirror-matrix/r1c2", (96.0, -13.0, 0.0)),
        ];
        for (id, expected) in expected_final {
            let part = document.parts.iter().find(|part| part.id == id).unwrap();
            assert!((part.pose.at.x - expected.0).abs() < 1e-9, "{id} x");
            assert!((part.pose.at.y - expected.1).abs() < 1e-9, "{id} y");
            assert!(
                (part.pose.rotation - expected.2).abs() < 1e-9,
                "{id} rotation"
            );
        }
        for (id, expected) in retained_ids.iter().zip(&retained_parts) {
            let retained = document.parts.iter().find(|part| part.id == *id).unwrap();
            assert_eq!(serde_json::to_value(retained).unwrap(), *expected);
        }

        let final_matrix = document
            .matrices
            .iter()
            .find(|matrix| matrix.id == "mirror-matrix")
            .unwrap();
        assert_eq!(final_matrix.origin, Vec2 { x: 102.0, y: -33.0 });
        assert_eq!(final_matrix.rotation, Some(90.0));
        assert_eq!(final_matrix.mirror, Some(Mirror::X));
        assert_eq!(
            final_matrix.part_ids,
            vec![
                "matrix/mirror-matrix/r0c0",
                "matrix/mirror-matrix/r0c1",
                "matrix/mirror-matrix/r0c2",
                "matrix/mirror-matrix/r1c0",
                "matrix/mirror-matrix/r1c1",
                "matrix/mirror-matrix/r1c2",
            ]
        );
        for (row, column, expected_offset, expected_rotation) in [
            (0, 0, Vec2 { x: 101.0, y: 37.0 }, 0.0),
            (1, 1, Vec2 { x: -59.0, y: 55.0 }, -90.0),
        ] {
            let cell = final_matrix
                .cells
                .iter()
                .find(|cell| cell.row == row && cell.column == column)
                .unwrap();
            let offset = cell.offset.unwrap();
            assert!((offset.x - expected_offset.x).abs() < 1e-9);
            assert!((offset.y - expected_offset.y).abs() < 1e-9);
            assert!((cell.rotation.unwrap() - expected_rotation).abs() < 1e-9);
        }
        assert!(
            final_matrix
                .cells
                .iter()
                .all(|cell| cell.enabled && !cell.deleted)
        );

        let mut disposable = document.clone();
        crate::matrix::set_matrix(&mut disposable, final_matrix).unwrap();
        let rematerialized = disposable
            .matrices
            .iter()
            .find(|matrix| matrix.id == "mirror-matrix")
            .unwrap();
        assert_eq!(rematerialized, final_matrix);
        for (id, expected) in expected_final {
            let part = disposable.parts.iter().find(|part| part.id == id).unwrap();
            assert!(
                (part.pose.at.x - expected.0).abs() < 1e-9,
                "{id} rematerialized x"
            );
            assert!(
                (part.pose.at.y - expected.1).abs() < 1e-9,
                "{id} rematerialized y"
            );
            assert!(
                (part.pose.rotation - expected.2).abs() < 1e-9,
                "{id} rematerialized rotation"
            );
        }
        assert_eq!(
            serde_json::json!({
                "boards": document.boards,
                "nets": document.nets,
                "constraints": document.constraints,
                "boardOutlines": document.board_outlines,
                "outline": document.outline,
            }),
            unchanged_references
        );
        for (id, expected_owner) in &projection.final_owners {
            let owner = document
                .boards
                .iter()
                .find(|board| board.part_ids.iter().any(|part_id| part_id == id))
                .unwrap();
            assert_eq!(&owner.id, expected_owner, "owner for {id}");
        }
        assert!(
            document
                .boards
                .iter()
                .find(|board| board.id == "source-board")
                .unwrap()
                .net_ids
                .is_empty()
        );
        assert!(
            document
                .boards
                .iter()
                .find(|board| board.id == "source-board")
                .unwrap()
                .outline_ids
                .is_empty()
        );
        let mirror_board = document
            .boards
            .iter()
            .find(|board| board.id == "mirror-board")
            .unwrap();
        assert_eq!(mirror_board.net_ids, vec!["retained-second-key-net"]);
        assert_eq!(mirror_board.outline_ids, vec!["retained-keys-envelope"]);
        assert!(mirror_board.part_ids.contains(&"geometry-probe".to_owned()));

        let before_repeat = document.clone();
        assert!(
            reconcile(&mut document, "pair-source-mirror", &projection)
                .unwrap()
                .is_empty()
        );
        assert_eq!(document, before_repeat);
    }

    #[test]
    fn fixed_membership_rejections_are_atomic_and_reach_the_intended_guard() {
        fn assert_rejected(
            mut document: ProjectDoc,
            projection: &Projection,
            expected_reason: &str,
        ) {
            let before = serde_json::to_value(&document).unwrap();
            let error = reconcile(&mut document, "pair-source-mirror", projection).unwrap_err();
            assert!(
                error.contains(expected_reason),
                "expected {expected_reason:?}, got {error:?}"
            );
            assert_eq!(serde_json::to_value(&document).unwrap(), before);
        }

        let (mut document, projection) = fixed_geometry_fixture().unwrap();
        document.hardware.as_mut().unwrap().pairs[0]
            .halves
            .as_mut()
            .unwrap()
            .correspondences
            .push(MirroredCorrespondence {
                companion_mode: None,
                companion_tweak: None,
                kind: MirroredItemKind::Matrix,
                source_id: "mirror-matrix".into(),
                mirror_id: "source-matrix".into(),
            });
        assert_rejected(document, &projection, "matrix correspondence is ambiguous");

        let (document, mut projection) = fixed_geometry_fixture().unwrap();
        projection.parts.push(PartProjection {
            source_id: "matrix/source-matrix/r0c1".into(),
            mirror_id: "matrix/mirror-matrix/r0c1".into(),
        });
        assert_rejected(
            document,
            &projection,
            "linked matrix associations contain aliases",
        );

        let (mut document, projection) = fixed_geometry_fixture().unwrap();
        document
            .parts
            .retain(|part| part.id != "matrix/source-matrix/r0c0");
        assert_rejected(document, &projection, "unavailable");

        let (document, mut projection) = fixed_geometry_fixture().unwrap();
        projection.final_owners.remove("matrix/mirror-matrix/r1c1");
        assert_rejected(document, &projection, "final owner input is incomplete");

        let (mut document, projection) = fixed_geometry_fixture().unwrap();
        let part_id = "matrix/source-matrix/r0c1";
        document
            .boards
            .iter_mut()
            .find(|board| board.id == "source-board")
            .unwrap()
            .part_ids
            .retain(|id| id != part_id);
        document
            .boards
            .iter_mut()
            .find(|board| board.id == "mirror-board")
            .unwrap()
            .part_ids
            .push(part_id.into());
        assert_rejected(document, &projection, "owner input");

        let (mut document, projection) = fixed_geometry_fixture().unwrap();
        document
            .matrices
            .iter_mut()
            .find(|matrix| matrix.id == "source-matrix")
            .unwrap()
            .cells[1]
            .enabled = false;
        assert_rejected(document, &projection, "fixed active membership");

        let (mut document, projection) = fixed_geometry_fixture().unwrap();
        document
            .matrices
            .iter_mut()
            .find(|matrix| matrix.id == "source-matrix")
            .unwrap()
            .part_ids
            .push("matrix/source-matrix/r0c0".into());
        assert_rejected(document, &projection, "membership is not canonical");

        let (mut document, projection) = fixed_geometry_fixture().unwrap();
        document
            .matrices
            .iter_mut()
            .find(|matrix| matrix.id == "source-matrix")
            .unwrap()
            .column_splays[1] = f64::NAN;
        assert_rejected(document, &projection, "finite");

        let (mut document, projection) = fixed_geometry_fixture().unwrap();
        document
            .matrices
            .iter_mut()
            .find(|matrix| matrix.id == "source-matrix")
            .unwrap()
            .column_splays = vec![f64::MAX, f64::MAX, 0.0];
        assert_rejected(document, &projection, "non-finite splay geometry");

        let (mut document, projection) = fixed_geometry_fixture().unwrap();
        document.hardware.as_mut().unwrap().pairs[0]
            .halves
            .as_mut()
            .unwrap()
            .correspondences
            .retain(|association| association.source_id != "matrix/source-matrix/r1c1");
        assert_rejected(
            document,
            &projection,
            "canonical matrix part association is missing",
        );
    }

    #[test]
    fn valid_pair_fixture_materializes_both_matrices_and_passes_validators() {
        let document = fixture_document().unwrap();

        assert_eq!(document.matrices.len(), 2);
        for matrix_id in ["source-matrix", "mirror-matrix"] {
            let matrix = document
                .matrices
                .iter()
                .find(|matrix| matrix.id == matrix_id)
                .unwrap();
            assert_eq!(matrix.part_ids.len(), 2);
            assert!(
                matrix
                    .part_ids
                    .iter()
                    .all(|id| document.parts.iter().any(|part| &part.id == id))
            );
        }
        assert!(
            document
                .parts
                .iter()
                .any(|part| part.id == "matrix/source-matrix/r0c0")
        );
        assert!(
            document
                .parts
                .iter()
                .any(|part| part.id == "matrix/source-matrix/r0c1")
        );
        assert!(
            document
                .parts
                .iter()
                .any(|part| part.id == "matrix/mirror-matrix/r0c0")
        );
        assert!(
            document
                .parts
                .iter()
                .any(|part| part.id == "matrix/mirror-matrix/r0c1")
        );
        crate::mechanical::features::validate(&document).unwrap();
        crate::mechanical::halves::validate_document(&document).unwrap();
    }

    #[test]
    fn changed_source_matrix_projects_linked_neighbor_after_entry_snapshot() {
        let mut document = fixture_document().unwrap();
        let mut moved_source = document
            .matrices
            .iter()
            .find(|matrix| matrix.id == "source-matrix")
            .unwrap()
            .clone();
        moved_source.origin = Vec2 { x: 27.0, y: -14.0 };
        moved_source.rotation = Some(43.0);
        crate::matrix::set_matrix(&mut document, &moved_source).unwrap();

        let definition = document
            .definitions
            .iter_mut()
            .find(|definition| definition.id == "switch")
            .unwrap();
        definition.pads = vec![
            crate::model::Pad {
                id: "pad-one".into(),
                number: "1".into(),
                at: Vec2 { x: 0.0, y: 0.0 },
                size: Vec2 { x: 1.0, y: 1.0 },
                shape: crate::model::PadShape::Circle,
                drill: None,
                plated: None,
                side: None,
                rotation: None,
                net_id: None,
            },
            crate::model::Pad {
                id: "pad-two".into(),
                number: "2".into(),
                at: Vec2 { x: 1.0, y: 0.0 },
                size: Vec2 { x: 1.0, y: 1.0 },
                shape: crate::model::PadShape::Circle,
                drill: None,
                plated: None,
                side: None,
                rotation: None,
                net_id: None,
            },
        ];

        let probe_id = "projection-probe";
        let mut probe = document
            .parts
            .iter()
            .find(|part| part.id == "matrix/mirror-matrix/r0c1")
            .unwrap()
            .clone();
        probe.id = probe_id.into();
        probe.reference = "PROBE".into();
        probe.pose.at = Vec2 { x: 4.0, y: 2.0 };
        probe.pose.rotation = 0.0;
        document.parts.push(probe);
        document
            .boards
            .iter_mut()
            .find(|board| board.id == "mirror-board")
            .unwrap()
            .part_ids
            .push(probe_id.into());

        let retained_net_id = "retained-two-pin-net";
        document.nets.push(crate::model::Net {
            id: retained_net_id.into(),
            name: "RETAINED".into(),
            pins: vec![
                crate::model::Pin {
                    part_id: "matrix/mirror-matrix/r0c0".into(),
                    pad_id: "pad-one".into(),
                },
                crate::model::Pin {
                    part_id: probe_id.into(),
                    pad_id: "pad-two".into(),
                },
            ],
        });
        document
            .boards
            .iter_mut()
            .find(|board| board.id == "mirror-board")
            .unwrap()
            .net_ids
            .push(retained_net_id.into());

        let retained_offset = crate::model::Constraint::Offset {
            id: "retained-stationary-offset".into(),
            source_part_id: probe_id.into(),
            target_part_id: "matrix/mirror-matrix/r0c0".into(),
            offset: Vec2 { x: -4.0, y: -2.0 },
            rotation: 0.0,
        };
        document.constraints.push(retained_offset.clone());

        let retained_envelope_id = "retained-global-envelope";
        document
            .outline
            .push(crate::model::OutlineFeature::PartEnvelope {
                connections: vec![],
                settings: Default::default(),
                id: retained_envelope_id.into(),
                part_ids: vec![
                    "matrix/mirror-matrix/r0c0".into(),
                    "matrix/mirror-matrix/r0c1".into(),
                    probe_id.into(),
                ],
                margin: 2.0,
                operation: crate::model::Operation::Add,
            });
        document
            .boards
            .iter_mut()
            .find(|board| board.id == "mirror-board")
            .unwrap()
            .outline_ids
            .push(retained_envelope_id.into());

        let assert_retained_net_is_valid = |document: &ProjectDoc| {
            let net = document
                .nets
                .iter()
                .find(|net| net.id == retained_net_id)
                .unwrap();
            assert_eq!(
                net.pins,
                vec![
                    crate::model::Pin {
                        part_id: "matrix/mirror-matrix/r0c0".into(),
                        pad_id: "pad-one".into(),
                    },
                    crate::model::Pin {
                        part_id: probe_id.into(),
                        pad_id: "pad-two".into(),
                    },
                ]
            );
            let owner = document
                .boards
                .iter()
                .find(|board| board.net_ids.contains(&net.id))
                .unwrap();
            assert_eq!(owner.id, "mirror-board");
            assert_eq!(
                document
                    .boards
                    .iter()
                    .filter(|board| board.net_ids.contains(&net.id))
                    .count(),
                1
            );
            for pin in &net.pins {
                assert!(owner.part_ids.contains(&pin.part_id));
                let part = document
                    .parts
                    .iter()
                    .find(|part| part.id == pin.part_id)
                    .unwrap();
                let definition = document
                    .definitions
                    .iter()
                    .find(|definition| definition.id == part.definition_id)
                    .unwrap();
                assert!(
                    definition
                        .pads
                        .iter()
                        .any(|pad| pad.id == pin.pad_id && !pad.id.is_empty())
                );
            }

            let retained_mirror = document
                .parts
                .iter()
                .find(|part| part.id == "matrix/mirror-matrix/r0c0")
                .unwrap();
            assert_eq!(
                retained_mirror.pose,
                crate::model::Pose2 {
                    at: Vec2 { x: 0.0, y: 0.0 },
                    rotation: 0.0,
                }
            );
            let probe = document
                .parts
                .iter()
                .find(|part| part.id == probe_id)
                .unwrap();
            assert_eq!(
                probe.pose,
                crate::model::Pose2 {
                    at: Vec2 { x: 4.0, y: 2.0 },
                    rotation: 0.0,
                }
            );

            let envelope = document
                .outline
                .iter()
                .find(|feature| {
                    matches!(feature, crate::model::OutlineFeature::PartEnvelope { id, .. } if id == retained_envelope_id)
                })
                .unwrap();
            let crate::model::OutlineFeature::PartEnvelope { part_ids, .. } = envelope else {
                unreachable!("the retained feature was found as a part envelope");
            };
            assert_eq!(
                part_ids,
                &vec![
                    "matrix/mirror-matrix/r0c0".to_owned(),
                    "matrix/mirror-matrix/r0c1".to_owned(),
                    probe_id.to_owned(),
                ]
            );
            assert_eq!(
                document
                    .boards
                    .iter()
                    .filter(|board| board.outline_ids.contains(&retained_envelope_id.to_owned()))
                    .map(|board| board.id.as_str())
                    .collect::<Vec<_>>(),
                vec!["mirror-board"]
            );
            assert!(
                document
                    .boards
                    .iter()
                    .find(|board| board.id == "source-board")
                    .unwrap()
                    .net_ids
                    .iter()
                    .all(|net_id| net_id != retained_net_id)
            );
        };
        assert_retained_net_is_valid(&document);
        crate::mechanical::halves::validate_document(&document).unwrap();

        // The entry snapshot is deliberately taken after the source's ordinary edit/materialization.
        let entry = document.clone();
        let entry_source_matrix = entry
            .matrices
            .iter()
            .find(|matrix| matrix.id == "source-matrix")
            .unwrap()
            .clone();
        let entry_mirror_matrix = entry
            .matrices
            .iter()
            .find(|matrix| matrix.id == "mirror-matrix")
            .unwrap()
            .clone();
        let matrix_correspondence = MirroredCorrespondence {
            companion_mode: None,
            companion_tweak: None,
            kind: MirroredItemKind::Matrix,
            source_id: entry_source_matrix.id.clone(),
            mirror_id: entry_mirror_matrix.id.clone(),
        };
        let expected_mirror_matrix = project_matrix(
            &entry_source_matrix,
            &entry_mirror_matrix,
            &matrix_correspondence,
            fixture_mapping(),
        )
        .unwrap();
        let mut expected_document = entry.clone();
        crate::matrix::set_matrix(&mut expected_document, &expected_mirror_matrix).unwrap();

        let protected_local_id = "matrix/mirror-matrix/r0c0";
        let linked_source_id = "matrix/source-matrix/r0c1";
        let linked_mirror_id = "matrix/mirror-matrix/r0c1";
        let entry_probe = entry
            .parts
            .iter()
            .find(|part| part.id == probe_id)
            .unwrap()
            .clone();
        let entry_probe_owner = entry
            .boards
            .iter()
            .find(|board| board.part_ids.contains(&probe_id.to_owned()))
            .unwrap()
            .id
            .clone();
        let entry_local = entry
            .parts
            .iter()
            .find(|part| part.id == protected_local_id)
            .unwrap();

        let mut final_owners = BTreeMap::new();
        for id in [
            "matrix/source-matrix/r0c0",
            linked_source_id,
            protected_local_id,
            linked_mirror_id,
        ] {
            let owner = entry
                .boards
                .iter()
                .find(|board| board.part_ids.iter().any(|part_id| part_id == id))
                .map(|board| board.id.clone())
                .unwrap();
            final_owners.insert(id.to_owned(), owner);
        }
        let projection = Projection {
            matrix: Some(MatrixLink {
                source_matrix_id: entry_source_matrix.id.clone(),
                mirror_matrix_id: entry_mirror_matrix.id.clone(),
            }),
            preserve_matrix_geometry: false,
            source_board_id: "source-board".into(),
            mirror_board_id: "mirror-board".into(),
            mapping: fixture_mapping(),
            parts: vec![PartProjection {
                source_id: linked_source_id.into(),
                mirror_id: linked_mirror_id.into(),
            }],
            independent_part_transitions: vec![],
            hosted_transitions: vec![],
            applied_independent_additions: std::cell::RefCell::new(BTreeSet::new()),
            applied_hosted_operations: std::cell::RefCell::new(BTreeSet::new()),
            applied_hosted_circuit_additions: std::cell::RefCell::new(BTreeMap::new()),
            hosted_removal_baselines: std::cell::RefCell::new(BTreeMap::new()),
            hosted_omission_baselines: std::cell::RefCell::new(BTreeMap::new()),
            outline_removal_baselines: std::cell::RefCell::new(BTreeMap::new()),
            locally_protected_mirror_parts: BTreeSet::from([protected_local_id.into()]),
            final_owners: final_owners.clone(),
        };

        let entry_source_matrix_json = serde_json::to_value(&entry_source_matrix).unwrap();
        let entry_source_parts_json: Vec<_> = ["matrix/source-matrix/r0c0", linked_source_id]
            .into_iter()
            .map(|id| {
                serde_json::to_value(entry.parts.iter().find(|part| part.id == id).unwrap())
                    .unwrap()
            })
            .collect();
        let entry_mirror_ids = entry_mirror_matrix.part_ids.clone();
        let entry_local_json = serde_json::to_value(entry_local).unwrap();
        let protected_references = serde_json::json!({
            "boards": &entry.boards,
            "nets": &entry.nets,
            "constraints": &entry.constraints,
            "boardOutlines": &entry.board_outlines,
            "definitions": &entry.definitions,
            "outline": &entry.outline,
        });
        let changed_ids = reconcile(&mut document, "pair-source-mirror", &projection).unwrap();

        assert_retained_net_is_valid(&document);
        crate::mechanical::halves::validate_document(&document).unwrap();
        assert_eq!(
            document.constraints.iter().find(|constraint| {
                matches!(constraint, crate::model::Constraint::Offset { id, .. } if id == "retained-stationary-offset")
            }),
            Some(&retained_offset)
        );
        let actual_probe = document
            .parts
            .iter()
            .find(|part| part.id == probe_id)
            .unwrap();
        assert_eq!(actual_probe, &entry_probe);
        let actual_probe_owner = document
            .boards
            .iter()
            .find(|board| board.part_ids.contains(&probe_id.to_owned()))
            .unwrap()
            .id
            .clone();
        assert_eq!(actual_probe_owner, entry_probe_owner);
        assert!(
            document
                .boards
                .iter()
                .find(|board| board.id == entry_probe_owner)
                .unwrap()
                .part_ids
                .contains(&probe_id.to_owned())
        );

        let actual_neighbor = document
            .parts
            .iter()
            .find(|part| part.id == linked_mirror_id)
            .unwrap();
        assert!(
            (actual_neighbor.pose.at.x - 45.48221151584141).abs() < 1e-9,
            "the linked neighbor must receive the independently calculated reflected x position"
        );
        assert!(
            (actual_neighbor.pose.at.y - -46.956048106863605).abs() < 1e-9,
            "the linked neighbor must receive the independently calculated reflected y position"
        );
        assert!(
            (actual_neighbor.pose.rotation - -12.0).abs() < 1e-9,
            "the linked neighbor must receive the independently calculated reflected heading"
        );

        assert_eq!(
            serde_json::to_value(
                document
                    .matrices
                    .iter()
                    .find(|matrix| matrix.id == "source-matrix")
                    .unwrap()
            )
            .unwrap(),
            entry_source_matrix_json
        );
        let actual_source_parts_json: Vec<_> = ["matrix/source-matrix/r0c0", linked_source_id]
            .into_iter()
            .map(|id| {
                serde_json::to_value(document.parts.iter().find(|part| part.id == id).unwrap())
                    .unwrap()
            })
            .collect();
        assert_eq!(actual_source_parts_json, entry_source_parts_json);
        let actual_mirror_matrix = document
            .matrices
            .iter()
            .find(|matrix| matrix.id == "mirror-matrix")
            .unwrap();
        assert_eq!(actual_mirror_matrix.id, entry_mirror_matrix.id);
        assert_eq!(actual_mirror_matrix.part_ids, entry_mirror_ids);
        let mut rematerialized = entry.clone();
        crate::matrix::set_matrix(&mut rematerialized, actual_mirror_matrix).unwrap();
        let rematerialized_matrix = rematerialized
            .matrices
            .iter()
            .find(|matrix| matrix.id == "mirror-matrix")
            .unwrap();
        assert_eq!(
            serde_json::to_value(actual_mirror_matrix).unwrap(),
            serde_json::to_value(rematerialized_matrix).unwrap(),
            "the final active matrix must already be a complete normalized materializer input"
        );
        let actual_retained = document
            .parts
            .iter()
            .find(|part| part.id == protected_local_id)
            .unwrap();
        let rematerialized_retained = rematerialized
            .parts
            .iter()
            .find(|part| part.id == protected_local_id)
            .unwrap();
        for pose in [&actual_retained.pose, &rematerialized_retained.pose] {
            assert!((pose.at.x - 0.0).abs() < 1e-9);
            assert!((pose.at.y - 0.0).abs() < 1e-9);
            assert!((pose.rotation - 0.0).abs() < 1e-9);
        }
        let mut actual_non_pose = serde_json::to_value(actual_retained).unwrap();
        let mut rematerialized_non_pose = serde_json::to_value(rematerialized_retained).unwrap();
        actual_non_pose.as_object_mut().unwrap().remove("pose");
        rematerialized_non_pose
            .as_object_mut()
            .unwrap()
            .remove("pose");
        assert_eq!(
            actual_non_pose, rematerialized_non_pose,
            "the retained member's identity and non-pose data must agree with rematerialization"
        );
        assert_eq!(
            serde_json::to_value(
                document
                    .parts
                    .iter()
                    .find(|part| part.id == protected_local_id)
                    .unwrap()
            )
            .unwrap(),
            entry_local_json
        );
        assert_eq!(
            serde_json::json!({
                "boards": &document.boards,
                "nets": &document.nets,
                "constraints": &document.constraints,
                "boardOutlines": &document.board_outlines,
                "definitions": &document.definitions,
                "outline": &document.outline,
            }),
            protected_references
        );

        let actual_final_owners: BTreeMap<_, _> = final_owners
            .keys()
            .map(|id| {
                let owner = document
                    .boards
                    .iter()
                    .find(|board| board.part_ids.iter().any(|part_id| part_id == id))
                    .map(|board| board.id.clone())
                    .unwrap();
                (id.clone(), owner)
            })
            .collect();
        assert_eq!(actual_final_owners, final_owners);

        let mut expected_changed_ids = BTreeSet::new();
        if serde_json::to_value(&expected_mirror_matrix).unwrap()
            != serde_json::to_value(&entry_mirror_matrix).unwrap()
        {
            expected_changed_ids.insert(entry_mirror_matrix.id.clone());
        }
        let before = entry
            .parts
            .iter()
            .find(|part| part.id == linked_mirror_id)
            .unwrap();
        let after = expected_document
            .parts
            .iter()
            .find(|part| part.id == linked_mirror_id)
            .unwrap();
        if serde_json::to_value(before).unwrap() != serde_json::to_value(after).unwrap() {
            expected_changed_ids.insert(linked_mirror_id.into());
        }
        assert_eq!(
            changed_ids,
            expected_changed_ids.into_iter().collect::<Vec<_>>()
        );
        let before_second_reconcile = document.clone();
        assert!(
            reconcile(&mut document, "pair-source-mirror", &projection)
                .unwrap()
                .is_empty()
        );
        assert_eq!(document, before_second_reconcile);

        let before_rejected = serde_json::to_value(&document).unwrap();
        let unsupported = Projection {
            matrix: projection.matrix.clone(),
            preserve_matrix_geometry: projection.preserve_matrix_geometry,
            source_board_id: projection.source_board_id.clone(),
            mirror_board_id: projection.mirror_board_id.clone(),
            mapping: projection.mapping,
            parts: vec![],
            independent_part_transitions: vec![],
            hosted_transitions: vec![],
            applied_independent_additions: std::cell::RefCell::new(BTreeSet::new()),
            applied_hosted_operations: std::cell::RefCell::new(BTreeSet::new()),
            applied_hosted_circuit_additions: std::cell::RefCell::new(BTreeMap::new()),
            hosted_removal_baselines: std::cell::RefCell::new(BTreeMap::new()),
            hosted_omission_baselines: std::cell::RefCell::new(BTreeMap::new()),
            outline_removal_baselines: std::cell::RefCell::new(BTreeMap::new()),
            locally_protected_mirror_parts: projection.locally_protected_mirror_parts.clone(),
            final_owners: projection.final_owners.clone(),
        };
        assert!(reconcile(&mut document, "pair-source-mirror", &unsupported).is_err());
        assert_eq!(serde_json::to_value(&document).unwrap(), before_rejected);

        let mismatched_pair = Projection {
            matrix: projection.matrix.clone(),
            preserve_matrix_geometry: projection.preserve_matrix_geometry,
            mapping: Pose2 {
                at: projection.mapping.at,
                rotation: projection.mapping.rotation + 1.0,
            },
            source_board_id: projection.source_board_id.clone(),
            mirror_board_id: projection.mirror_board_id.clone(),
            parts: projection
                .parts
                .iter()
                .map(|part| PartProjection {
                    source_id: part.source_id.clone(),
                    mirror_id: part.mirror_id.clone(),
                })
                .collect(),
            independent_part_transitions: vec![],
            hosted_transitions: vec![],
            applied_independent_additions: std::cell::RefCell::new(BTreeSet::new()),
            applied_hosted_operations: std::cell::RefCell::new(BTreeSet::new()),
            applied_hosted_circuit_additions: std::cell::RefCell::new(BTreeMap::new()),
            hosted_removal_baselines: std::cell::RefCell::new(BTreeMap::new()),
            hosted_omission_baselines: std::cell::RefCell::new(BTreeMap::new()),
            outline_removal_baselines: std::cell::RefCell::new(BTreeMap::new()),
            locally_protected_mirror_parts: projection.locally_protected_mirror_parts.clone(),
            final_owners: projection.final_owners.clone(),
        };
        assert!(reconcile(&mut document, "pair-source-mirror", &mismatched_pair).is_err());
        assert_eq!(serde_json::to_value(&document).unwrap(), before_rejected);
    }

    #[test]
    fn reconcile_projects_associated_independent_parts_and_preserves_fixed_layouts() {
        let (mut document, mut projection) = fixed_geometry_fixture().unwrap();
        let mut source = document
            .parts
            .iter()
            .find(|part| part.id == "matrix/source-matrix/r0c1")
            .unwrap()
            .clone();
        source.id = "independent-source".into();
        source.reference = "J1".into();
        source.pose = Pose2 {
            at: Vec2 { x: 10.0, y: 15.0 },
            rotation: 25.0,
        };
        source.keycap = Some(Vec2 { x: 22.0, y: 18.0 });
        source.outline = Some(PartOutline {
            excluded: true,
            margin: Some(3.5),
            allow_body_overhang: true,
        });
        source.side = Side::Back;
        source.locked = Some(false);
        source.properties = Some(BTreeMap::from([
            (
                "custom".into(),
                serde_json::json!({"board":"source","revision":7}),
            ),
            ("userFlag".into(), serde_json::json!(true)),
        ]));
        source.generator_parameters = Some(BTreeMap::from([
            ("variant".into(), serde_json::json!("non-default")),
            ("width".into(), serde_json::json!(2.25)),
        ]));
        let mut target = source.clone();
        target.id = "independent-mirror".into();
        target.definition_id = "mirror-switch".into();
        target.reference = "J9".into();
        target.pose = Pose2 {
            at: Vec2::default(),
            rotation: 0.0,
        };
        target.keycap = None;
        target.outline = None;
        target.side = Side::Front;
        target.locked = Some(true);
        target.properties = Some(BTreeMap::from([(
            "local".into(),
            serde_json::json!("target"),
        )]));
        target.generator_parameters =
            Some(BTreeMap::from([("local".into(), serde_json::json!(9))]));
        let mut target_definition = document.definitions[0].clone();
        target_definition.id = "mirror-switch".into();
        target_definition.name = "Mirror switch".into();
        document.definitions.push(target_definition);
        document.parts.extend([source.clone(), target]);
        let mut unrelated_part = document
            .parts
            .iter()
            .find(|part| part.id == "geometry-probe")
            .unwrap()
            .clone();
        unrelated_part.id = "unrelated-part".into();
        unrelated_part.reference = "LOCAL1".into();
        document.parts.push(unrelated_part);
        let mut added_source = source.clone();
        added_source.id = "independent-source-added".into();
        added_source.reference = "J2".into();
        added_source.pose.at = Vec2 { x: -8.0, y: 6.0 };
        document.parts.push(added_source.clone());
        document
            .boards
            .iter_mut()
            .find(|board| board.id == "source-board")
            .unwrap()
            .part_ids
            .push(source.id.clone());
        document
            .boards
            .iter_mut()
            .find(|board| board.id == "source-board")
            .unwrap()
            .part_ids
            .push(added_source.id.clone());
        document
            .boards
            .iter_mut()
            .find(|board| board.id == "mirror-board")
            .unwrap()
            .part_ids
            .push("independent-mirror".into());
        document
            .boards
            .iter_mut()
            .find(|board| board.id == "mirror-board")
            .unwrap()
            .part_ids
            .push("unrelated-part".into());

        let halves = document.hardware.as_mut().unwrap().pairs[0]
            .halves
            .as_mut()
            .unwrap();
        halves.correspondences.extend([
            MirroredCorrespondence {
                companion_mode: None,
                companion_tweak: None,
                kind: MirroredItemKind::Part,
                source_id: "independent-source".into(),
                mirror_id: "independent-mirror".into(),
            },
            MirroredCorrespondence {
                companion_mode: None,
                companion_tweak: None,
                kind: MirroredItemKind::Layout,
                source_id: "source-layout".into(),
                mirror_id: "mirror-layout".into(),
            },
        ]);
        let source_layout = Layout {
            id: "source-layout".into(),
            name: "Source layout".into(),
            board_id: "source-board".into(),
            matrix_id: "source-matrix".into(),
            part_ids: vec!["independent-source".into()],
            mirror_link: None,
        };
        let mirror_layout = Layout {
            id: "mirror-layout".into(),
            name: "My target layout".into(),
            board_id: "mirror-board".into(),
            matrix_id: "mirror-matrix".into(),
            part_ids: vec!["geometry-probe".into(), "independent-mirror".into()],
            mirror_link: None,
        };
        projection.independent_part_transitions = vec![IndependentPartTransition::Add {
            source_id: "independent-source-added".into(),
            mirror_id: "independent-mirror-added".into(),
            mirror_definition_id: "mirror-switch".into(),
            mirror_reference: "J99".into(),
        }];
        crate::matrix::set_matrix(
            &mut document,
            &fixture_matrix("unrelated-matrix", "mirror-board"),
        )
        .unwrap();
        let unrelated_layout = Layout {
            id: "unrelated-layout".into(),
            name: "Unrelated local layout".into(),
            board_id: "mirror-board".into(),
            matrix_id: "unrelated-matrix".into(),
            part_ids: vec!["unrelated-part".into()],
            mirror_link: None,
        };
        document.layouts = vec![
            source_layout.clone(),
            mirror_layout.clone(),
            unrelated_layout.clone(),
        ];

        assert!(
            document
                .parts
                .iter()
                .all(|part| part.id != "independent-mirror-added")
        );
        assert!(document.parts.iter().all(|part| part.reference != "J99"));

        crate::apply(
            &mut document,
            &[],
            &crate::model::EditOperation::SetLayout {
                layout: Layout {
                    part_ids: vec![
                        "independent-source-added".into(),
                        "independent-source".into(),
                    ],
                    ..source_layout
                },
            },
        )
        .expect("the ordinary source SetLayout operation records its membership change");

        crate::mechanical::halves::validate_document(&document).unwrap();
        let source_before = document
            .parts
            .iter()
            .find(|part| part.id == "independent-source")
            .unwrap()
            .clone();
        let unrelated_part_before = document
            .parts
            .iter()
            .find(|part| part.id == "unrelated-part")
            .unwrap()
            .clone();
        let unrelated_owner_ids_before: Vec<_> = document
            .boards
            .iter()
            .filter(|board| board.part_ids.contains(&"unrelated-part".to_owned()))
            .map(|board| board.id.clone())
            .collect();
        let original = document.clone();

        let valid_addition_evidence = projection.independent_part_transitions.clone();
        let mut invalid_addition_outcomes = Vec::new();

        let mut unrelated_projection = projection.clone();
        unrelated_projection.independent_part_transitions[0] = IndependentPartTransition::Add {
            source_id: "independent-source-added".into(),
            mirror_id: "unrelated-part".into(),
            mirror_definition_id: unrelated_part_before.definition_id.clone(),
            mirror_reference: unrelated_part_before.reference.clone(),
        };
        let mut unrelated_target = original.clone();
        let unrelated_target_before = unrelated_target.clone();
        let unrelated_result = reconcile(
            &mut unrelated_target,
            "pair-source-mirror",
            &unrelated_projection,
        );
        invalid_addition_outcomes.push((
            "first add cannot adopt an unrelated mirror-owned target",
            unrelated_result.is_err(),
            unrelated_target == unrelated_target_before,
        ));

        let mut two_sources = original.clone();
        let mut second_source = added_source.clone();
        second_source.id = "independent-source-outside-layout".into();
        second_source.reference = "Q77".into();
        second_source.pose = Pose2 {
            at: Vec2 { x: 31.0, y: -17.0 },
            rotation: 71.0,
        };
        second_source.properties = Some(BTreeMap::from([(
            "sibling".into(),
            serde_json::json!("distinct payload"),
        )]));
        assert_ne!(second_source.pose, added_source.pose);
        assert_ne!(second_source.properties, added_source.properties);
        two_sources.parts.push(second_source.clone());
        two_sources
            .boards
            .iter_mut()
            .find(|board| board.id == "source-board")
            .unwrap()
            .part_ids
            .push(second_source.id.clone());
        crate::mechanical::halves::validate_document(&two_sources)
            .expect("the sibling source outside the paired Layout is valid");
        assert!(
            two_sources.layouts[0]
                .part_ids
                .iter()
                .all(|id| id != &second_source.id)
        );
        assert!(two_sources.parts.iter().all(|part| {
            part.id == "independent-source-outside-layout" || part.reference != "Q91"
        }));
        assert!(two_sources.parts.iter().all(|part| {
            part.id == "independent-source-outside-layout" || part.reference != "Q92"
        }));
        assert!(two_sources.parts.iter().all(|part| {
            !matches!(
                part.id.as_str(),
                "planned-independent-target"
                    | "planned-independent-target-a"
                    | "planned-independent-target-b"
            )
        }));

        let mut aliased_id_projection = projection.clone();
        aliased_id_projection.independent_part_transitions = vec![
            IndependentPartTransition::Add {
                source_id: "independent-source-added".into(),
                mirror_id: "planned-independent-target".into(),
                mirror_definition_id: "mirror-switch".into(),
                mirror_reference: "Q91".into(),
            },
            IndependentPartTransition::Add {
                source_id: second_source.id.clone(),
                mirror_id: "planned-independent-target".into(),
                mirror_definition_id: "mirror-switch".into(),
                mirror_reference: "Q91".into(),
            },
        ];
        let mut aliased_target_id = two_sources.clone();
        let aliased_target_id_before = aliased_target_id.clone();
        let aliased_id_result = reconcile(
            &mut aliased_target_id,
            "pair-source-mirror",
            &aliased_id_projection,
        );
        invalid_addition_outcomes.push((
            "sibling additions cannot select the same fresh target ID and reference",
            aliased_id_result.is_err(),
            aliased_target_id == aliased_target_id_before,
        ));

        let mut aliased_reference_projection = projection.clone();
        aliased_reference_projection.independent_part_transitions = vec![
            IndependentPartTransition::Add {
                source_id: "independent-source-added".into(),
                mirror_id: "planned-independent-target-a".into(),
                mirror_definition_id: "mirror-switch".into(),
                mirror_reference: "Q92".into(),
            },
            IndependentPartTransition::Add {
                source_id: second_source.id,
                mirror_id: "planned-independent-target-b".into(),
                mirror_definition_id: "mirror-switch".into(),
                mirror_reference: "Q92".into(),
            },
        ];
        let mut aliased_target_reference = two_sources.clone();
        let aliased_target_reference_before = aliased_target_reference.clone();
        let aliased_reference_result = reconcile(
            &mut aliased_target_reference,
            "pair-source-mirror",
            &aliased_reference_projection,
        );
        invalid_addition_outcomes.push((
            "sibling additions cannot select distinct targets with one fresh reference",
            aliased_reference_result.is_err(),
            aliased_target_reference == aliased_target_reference_before,
        ));
        assert!(
            invalid_addition_outcomes
                .iter()
                .all(|(_, refused, unchanged)| *refused && *unchanged),
            "invalid independent Add evidence must refuse atomically: {invalid_addition_outcomes:?}"
        );

        projection.independent_part_transitions = valid_addition_evidence.clone();
        projection.independent_part_transitions[0] = IndependentPartTransition::Add {
            source_id: "independent-source-added".into(),
            mirror_id: "independent-mirror-added".into(),
            mirror_definition_id: "missing-mirror-definition".into(),
            mirror_reference: "J99".into(),
        };
        let mut unavailable_definition = original.clone();
        let refused = reconcile(
            &mut unavailable_definition,
            "pair-source-mirror",
            &projection,
        )
        .unwrap_err();
        assert!(refused.contains("target definition"));
        assert_eq!(unavailable_definition, original);

        projection.independent_part_transitions[0] = IndependentPartTransition::Add {
            source_id: "independent-source-added".into(),
            mirror_id: "independent-mirror-added".into(),
            mirror_definition_id: "mirror-switch".into(),
            mirror_reference: "LOCAL1".into(),
        };
        let mut conflicting_reference = original.clone();
        let refused = reconcile(
            &mut conflicting_reference,
            "pair-source-mirror",
            &projection,
        )
        .unwrap_err();
        assert!(refused.contains("reference") || refused.contains("already in use"));
        assert_eq!(conflicting_reference, original);

        projection.independent_part_transitions = valid_addition_evidence.clone();
        let mut stale_source_owner = original.clone();
        stale_source_owner
            .boards
            .iter_mut()
            .find(|board| board.id == "source-board")
            .unwrap()
            .part_ids
            .retain(|id| id != "independent-source-added");
        stale_source_owner
            .boards
            .iter_mut()
            .find(|board| board.id == "mirror-board")
            .unwrap()
            .part_ids
            .push("independent-source-added".into());
        let stale_source_owner_before = stale_source_owner.clone();
        let refused =
            reconcile(&mut stale_source_owner, "pair-source-mirror", &projection).unwrap_err();
        assert!(refused.contains("stale source ownership"));
        assert_eq!(stale_source_owner, stale_source_owner_before);

        let changed = reconcile(&mut document, "pair-source-mirror", &projection).unwrap();
        assert_eq!(
            changed,
            vec![
                "independent-mirror",
                "independent-mirror-added",
                "matrix/mirror-matrix/r0c1",
                "matrix/mirror-matrix/r0c2",
                "matrix/mirror-matrix/r1c0",
                "matrix/mirror-matrix/r1c2",
                "mirror-board",
                "mirror-layout",
                "mirror-matrix",
            ]
        );
        let mirrored = document
            .parts
            .iter()
            .find(|part| part.id == "independent-mirror")
            .unwrap();
        assert_eq!(mirrored.id, "independent-mirror");
        assert_eq!(mirrored.definition_id, "mirror-switch");
        assert_eq!(mirrored.reference, "J9");
        assert_eq!(mirrored.keycap, source.keycap);
        assert_eq!(mirrored.outline, source.outline);
        assert_eq!(mirrored.side, source.side);
        assert_eq!(mirrored.locked, source.locked);
        assert_eq!(mirrored.properties, source.properties);
        assert_eq!(mirrored.generator_parameters, source.generator_parameters);
        assert_eq!(
            mirrored.pose,
            Pose2 {
                at: Vec2 { x: 85.0, y: -40.0 },
                rotation: 245.0,
            }
        );
        assert!(changed.contains(&"independent-mirror".to_owned()));
        assert!(changed.contains(&"independent-mirror-added".to_owned()));
        let added_mirror = document
            .parts
            .iter()
            .find(|part| part.id == "independent-mirror-added")
            .expect("the explicit independent addition creates its selected target ID");
        assert_eq!(added_mirror.definition_id, "mirror-switch");
        assert_eq!(added_mirror.reference, "J99");
        assert_eq!(added_mirror.keycap, added_source.keycap);
        assert_eq!(added_mirror.outline, added_source.outline);
        assert_eq!(added_mirror.side, added_source.side);
        assert_eq!(added_mirror.locked, added_source.locked);
        assert_eq!(added_mirror.properties, added_source.properties);
        assert_eq!(
            added_mirror.generator_parameters,
            added_source.generator_parameters
        );
        assert_eq!(
            added_mirror.pose,
            Pose2 {
                at: Vec2 { x: 94.0, y: -22.0 },
                rotation: 245.0,
            }
        );
        assert!(
            document
                .boards
                .iter()
                .any(|board| board.id == "mirror-board"
                    && board.part_ids.contains(&"independent-mirror-added".into()))
        );
        assert_eq!(
            document
                .parts
                .iter()
                .find(|part| part.id == "independent-source")
                .unwrap(),
            &source_before
        );
        assert_eq!(document.layouts[1].id, mirror_layout.id);
        assert_eq!(document.layouts[1].name, mirror_layout.name);
        assert_eq!(document.layouts[1].board_id, mirror_layout.board_id);
        assert_eq!(document.layouts[1].matrix_id, mirror_layout.matrix_id);
        assert_eq!(document.layouts[1].mirror_link, mirror_layout.mirror_link);
        assert_eq!(
            document.layouts[1].part_ids,
            vec![
                "independent-mirror-added",
                "independent-mirror",
                "geometry-probe"
            ]
        );
        assert!(changed.contains(&"mirror-layout".to_owned()));
        assert_eq!(document.layouts[2], unrelated_layout);
        assert_eq!(
            document
                .parts
                .iter()
                .find(|part| part.id == "unrelated-part")
                .unwrap(),
            &unrelated_part_before
        );
        let unrelated_owner_ids_after: Vec<_> = document
            .boards
            .iter()
            .filter(|board| board.part_ids.contains(&"unrelated-part".to_owned()))
            .map(|board| board.id.clone())
            .collect();
        assert_eq!(unrelated_owner_ids_after, unrelated_owner_ids_before);

        let mut repeated_from_same_input = original.clone();
        reconcile(
            &mut repeated_from_same_input,
            "pair-source-mirror",
            &projection,
        )
        .unwrap();
        assert_eq!(repeated_from_same_input, document);
        let first_output = document.clone();
        let applied_additions_before_repeat =
            projection.applied_independent_additions.borrow().clone();
        let second_changed = reconcile(&mut document, "pair-source-mirror", &projection).unwrap();
        assert!(second_changed.is_empty());
        assert_eq!(document, first_output);
        assert_eq!(
            *projection.applied_independent_additions.borrow(),
            applied_additions_before_repeat,
            "updated private Add provenance converges with the repeated output"
        );

        let mut removal_source = document.clone();
        removal_source.nets.push(crate::model::Net {
            id: "independent-removal-net".into(),
            name: "Independent removal control".into(),
            pins: vec![
                crate::model::Pin {
                    part_id: "independent-mirror-added".into(),
                    pad_id: "pad-two".into(),
                },
                crate::model::Pin {
                    part_id: "geometry-probe".into(),
                    pad_id: "pad-two".into(),
                },
            ],
        });
        removal_source
            .boards
            .iter_mut()
            .find(|board| board.id == "mirror-board")
            .unwrap()
            .net_ids
            .push("independent-removal-net".into());
        removal_source
            .constraints
            .push(crate::model::Constraint::Offset {
                id: "independent-removal-offset".into(),
                source_part_id: "independent-mirror-added".into(),
                target_part_id: "geometry-probe".into(),
                offset: Vec2 { x: 4.0, y: -2.0 },
                rotation: 5.0,
            });
        let mut expected_retained_keys = removal_source
            .outline
            .iter()
            .find_map(|feature| match feature {
                OutlineFeature::PartEnvelope { id, part_ids, .. }
                    if id == "retained-keys-envelope" =>
                {
                    Some(part_ids.clone())
                }
                _ => None,
            })
            .expect("the baseline carries a registered retained-key envelope");
        expected_retained_keys.push("independent-mirror-added".into());
        if let Some(OutlineFeature::PartEnvelope { part_ids, .. }) =
            removal_source.outline.iter_mut().find(|feature| {
                matches!(feature, OutlineFeature::PartEnvelope { id, .. }
                if id == "retained-keys-envelope")
            })
        {
            part_ids.push("independent-mirror-added".into());
        }
        crate::apply(
            &mut removal_source,
            &[],
            &crate::model::EditOperation::RemoveParts {
                ids: vec!["independent-source-added".into()],
            },
        )
        .unwrap();
        assert!(
            removal_source
                .parts
                .iter()
                .all(|part| part.id != "independent-source-added")
        );
        assert!(removal_source.outline.iter().any(|feature| {
            matches!(feature, OutlineFeature::PartEnvelope { id, part_ids, .. }
                if id == "retained-keys-envelope"
                    && part_ids.contains(&"independent-mirror-added".into()))
        }));
        projection.independent_part_transitions = vec![IndependentPartTransition::Remove {
            source_id: "independent-source-added".into(),
            mirror_id: "independent-mirror-added".into(),
        }];
        let persisted_pair_before = removal_source.hardware.as_ref().unwrap().pairs[0]
            .halves
            .as_ref()
            .unwrap()
            .correspondences
            .clone();
        let mut refused_removal = removal_source.clone();
        refused_removal.layouts[1]
            .part_ids
            .push("independent-mirror".into());
        let refusal_before = refused_removal.clone();
        let refusal =
            reconcile(&mut refused_removal, "pair-source-mirror", &projection).unwrap_err();
        assert!(refusal.contains("paired layout contains duplicate part membership"));
        assert_eq!(refused_removal, refusal_before);

        projection.independent_part_transitions = vec![IndependentPartTransition::Remove {
            source_id: "independent-source-added".into(),
            mirror_id: "independent-mirror".into(),
        }];
        let mut conflicting_removal_identity = removal_source.clone();
        let conflicting_removal_before = conflicting_removal_identity.clone();
        let refusal = reconcile(
            &mut conflicting_removal_identity,
            "pair-source-mirror",
            &projection,
        )
        .unwrap_err();
        assert!(refusal.contains("conflicting pair provenance"));
        assert_eq!(conflicting_removal_identity, conflicting_removal_before);
        projection.independent_part_transitions = vec![IndependentPartTransition::Remove {
            source_id: "independent-source-added".into(),
            mirror_id: "independent-mirror-added".into(),
        }];

        let removed = reconcile(&mut removal_source, "pair-source-mirror", &projection).unwrap();
        assert!(
            removal_source
                .parts
                .iter()
                .all(|part| part.id != "independent-mirror-added")
        );
        assert!(
            removal_source
                .boards
                .iter()
                .all(|board| !board.part_ids.contains(&"independent-mirror-added".into()))
        );
        assert!(
            removal_source.layouts[1]
                .part_ids
                .iter()
                .all(|id| id != "independent-mirror-added")
        );
        assert_eq!(
            removal_source
                .nets
                .iter()
                .find(|net| net.id == "independent-removal-net")
                .unwrap()
                .pins,
            vec![crate::model::Pin {
                part_id: "geometry-probe".into(),
                pad_id: "pad-two".into(),
            }]
        );
        assert!(
            removal_source
                .constraints
                .iter()
                .all(|constraint| constraint.id() != "independent-removal-offset")
        );
        expected_retained_keys.retain(|id| id != "independent-mirror-added");
        assert!(removal_source.outline.iter().any(|feature| {
            matches!(feature, OutlineFeature::PartEnvelope { id, part_ids, .. }
                if id == "retained-keys-envelope" && part_ids == &expected_retained_keys)
        }));
        assert_eq!(
            removed,
            vec![
                "independent-mirror-added",
                "independent-removal-net",
                "independent-removal-offset",
                "mirror-board",
                "mirror-layout",
                "retained-keys-envelope",
            ]
        );
        assert_eq!(
            removal_source.hardware.as_ref().unwrap().pairs[0]
                .halves
                .as_ref()
                .unwrap()
                .correspondences,
            persisted_pair_before
        );
        let first_removal_output = removal_source.clone();
        let repeated_removal =
            reconcile(&mut removal_source, "pair-source-mirror", &projection).unwrap();
        assert!(repeated_removal.is_empty());
        assert_eq!(removal_source, first_removal_output);
    }

    #[test]
    fn source_layout_removal_prunes_linked_projection_and_keeps_local_and_unpaired_members() {
        let (mut document, projection) = fixed_geometry_fixture().unwrap();
        let source_linked = "layout-linked-source";
        let mirror_linked = "layout-linked-mirror";
        let source_local = "layout-local-source";
        let mirror_local = "layout-local-mirror";
        let mut linked_source_part = document
            .parts
            .iter()
            .find(|part| part.id == "matrix/source-matrix/r0c1")
            .unwrap()
            .clone();
        linked_source_part.id = source_linked.into();
        linked_source_part.reference = "LINKED-SOURCE".into();
        let mut linked_mirror_part = document
            .parts
            .iter()
            .find(|part| part.id == "matrix/mirror-matrix/r0c1")
            .unwrap()
            .clone();
        linked_mirror_part.id = mirror_linked.into();
        linked_mirror_part.reference = "LINKED-MIRROR".into();
        let mut local_source_part = linked_source_part.clone();
        local_source_part.id = source_local.into();
        local_source_part.reference = "LOCAL-SOURCE".into();
        let mut local_mirror_part = local_source_part.clone();
        local_mirror_part.id = mirror_local.into();
        local_mirror_part.reference = "LOCAL-MIRROR".into();
        document.parts.extend([
            linked_source_part,
            linked_mirror_part,
            local_source_part,
            local_mirror_part,
        ]);
        document
            .boards
            .iter_mut()
            .find(|board| board.id == "source-board")
            .unwrap()
            .part_ids
            .extend([source_linked.into(), source_local.into()]);
        document
            .boards
            .iter_mut()
            .find(|board| board.id == "mirror-board")
            .unwrap()
            .part_ids
            .extend([mirror_linked.into(), mirror_local.into()]);

        let halves = document.hardware.as_mut().unwrap().pairs[0]
            .halves
            .as_mut()
            .unwrap();
        halves.correspondences.extend([
            MirroredCorrespondence {
                companion_mode: None,
                companion_tweak: None,
                kind: MirroredItemKind::Part,
                source_id: source_linked.into(),
                mirror_id: mirror_linked.into(),
            },
            MirroredCorrespondence {
                companion_mode: None,
                companion_tweak: None,
                kind: MirroredItemKind::Part,
                source_id: source_local.into(),
                mirror_id: mirror_local.into(),
            },
            MirroredCorrespondence {
                companion_mode: None,
                companion_tweak: None,
                kind: MirroredItemKind::Layout,
                source_id: "source-layout-removal".into(),
                mirror_id: "mirror-layout-removal".into(),
            },
        ]);
        halves.exceptions.push(
            serde_json::from_value(serde_json::json!({
                "kind": "part", "mirrorId": mirror_local, "state": "local"
            }))
            .unwrap(),
        );

        let source_layout = Layout {
            id: "source-layout-removal".into(),
            name: "Source layout".into(),
            board_id: "source-board".into(),
            matrix_id: "source-matrix".into(),
            part_ids: vec![source_linked.into(), source_local.into()],
            mirror_link: None,
        };
        let mirror_layout = Layout {
            id: "mirror-layout-removal".into(),
            name: "Mirror layout".into(),
            board_id: "mirror-board".into(),
            matrix_id: "mirror-matrix".into(),
            part_ids: vec![
                mirror_linked.into(),
                mirror_local.into(),
                "geometry-probe".into(),
            ],
            mirror_link: None,
        };
        document.layouts = vec![source_layout.clone(), mirror_layout];
        crate::mechanical::halves::validate_document(&document).unwrap();

        let request = crate::model::EditOperation::SetLayout {
            layout: Layout {
                part_ids: vec![mirror_local.into(), "geometry-probe".into()],
                ..document.layouts[1].clone()
            },
        };
        let canonical = crate::mirrored_half::inverse::canonicalize(&document, &request)
            .expect("the mirror request maps into source membership");
        let expected_source = crate::model::EditOperation::SetLayout {
            layout: Layout {
                part_ids: vec![source_local.into()],
                ..source_layout
            },
        };
        assert_eq!(canonical.operation, expected_source);
        crate::apply(&mut document, &[], &canonical.operation)
            .expect("the translated source layout removes its linked member");
        reconcile(&mut document, "pair-source-mirror", &projection)
            .expect("the removal and retained target members reconcile");

        assert_eq!(
            document.layouts[1].part_ids,
            vec![mirror_local.to_owned(), "geometry-probe".to_owned()]
        );
    }

    #[test]
    fn mirror_layout_reorder_survives_source_apply_and_reconciliation() {
        let (mut document, projection) = fixed_geometry_fixture().unwrap();
        let source_header = "layout-header-source";
        let source_u1 = "layout-u1-source";
        let mirror_header = "layout-header-mirror";
        let mirror_u1 = "layout-u1-mirror";
        let mut source_header_part = document
            .parts
            .iter()
            .find(|part| part.id == "matrix/source-matrix/r0c0")
            .unwrap()
            .clone();
        source_header_part.id = source_header.into();
        source_header_part.reference = "LAYOUT-HEADER-SOURCE".into();
        let mut source_u1_part = document
            .parts
            .iter()
            .find(|part| part.id == "matrix/source-matrix/r0c1")
            .unwrap()
            .clone();
        source_u1_part.id = source_u1.into();
        source_u1_part.reference = "LAYOUT-U1-SOURCE".into();
        let mut mirror_header_part = document
            .parts
            .iter()
            .find(|part| part.id == "matrix/mirror-matrix/r0c0")
            .unwrap()
            .clone();
        mirror_header_part.id = mirror_header.into();
        mirror_header_part.reference = "LAYOUT-HEADER-MIRROR".into();
        let mut mirror_u1_part = document
            .parts
            .iter()
            .find(|part| part.id == "matrix/mirror-matrix/r0c1")
            .unwrap()
            .clone();
        mirror_u1_part.id = mirror_u1.into();
        mirror_u1_part.reference = "LAYOUT-U1-MIRROR".into();
        document.parts.extend([
            source_header_part,
            source_u1_part,
            mirror_header_part,
            mirror_u1_part,
        ]);
        document
            .boards
            .iter_mut()
            .find(|board| board.id == "source-board")
            .unwrap()
            .part_ids
            .extend([source_header.into(), source_u1.into()]);
        document
            .boards
            .iter_mut()
            .find(|board| board.id == "mirror-board")
            .unwrap()
            .part_ids
            .extend([mirror_header.into(), mirror_u1.into()]);
        document.hardware.as_mut().unwrap().pairs[0]
            .halves
            .as_mut()
            .unwrap()
            .correspondences
            .extend([
                MirroredCorrespondence {
                    companion_mode: None,
                    companion_tweak: None,
                    kind: MirroredItemKind::Part,
                    source_id: source_header.into(),
                    mirror_id: mirror_header.into(),
                },
                MirroredCorrespondence {
                    companion_mode: None,
                    companion_tweak: None,
                    kind: MirroredItemKind::Part,
                    source_id: source_u1.into(),
                    mirror_id: mirror_u1.into(),
                },
            ]);
        document.hardware.as_mut().unwrap().pairs[0]
            .halves
            .as_mut()
            .unwrap()
            .exceptions
            .push(
                serde_json::from_value(serde_json::json!({
                    "kind": "part", "mirrorId": mirror_u1, "state": "local"
                }))
                .unwrap(),
            );
        let source_layout = Layout {
            id: "source-layout-reorder".into(),
            name: "Source layout".into(),
            board_id: "source-board".into(),
            matrix_id: "source-matrix".into(),
            part_ids: vec![source_header.into(), source_u1.into()],
            mirror_link: None,
        };
        let mirror_layout = Layout {
            id: "mirror-layout-reorder".into(),
            name: "Mirror layout".into(),
            board_id: "mirror-board".into(),
            matrix_id: "mirror-matrix".into(),
            part_ids: vec![
                mirror_header.into(),
                mirror_u1.into(),
                "geometry-probe".into(),
            ],
            mirror_link: None,
        };
        document.layouts = vec![source_layout.clone(), mirror_layout.clone()];
        document.hardware.as_mut().unwrap().pairs[0]
            .halves
            .as_mut()
            .unwrap()
            .correspondences
            .push(MirroredCorrespondence {
                companion_mode: None,
                companion_tweak: None,
                kind: MirroredItemKind::Layout,
                source_id: source_layout.id.clone(),
                mirror_id: mirror_layout.id.clone(),
            });

        let requested = Layout {
            part_ids: vec![
                mirror_u1.into(),
                mirror_header.into(),
                "geometry-probe".into(),
            ],
            ..mirror_layout
        };
        let canonical = crate::mirrored_half::inverse::canonicalize(
            &document,
            &EditOperation::SetLayout {
                layout: requested.clone(),
            },
        )
        .expect("the reordered mirror request maps to source member order");
        assert_eq!(
            canonical.operation,
            EditOperation::SetLayout {
                layout: Layout {
                    part_ids: vec![source_u1.into(), source_header.into()],
                    ..source_layout
                },
            }
        );
        crate::apply(&mut document, &[], &canonical.operation)
            .expect("the canonical source layout applies");
        reconcile(&mut document, "pair-source-mirror", &projection)
            .expect("the reordered source layout reconciles");

        assert_eq!(document.layouts[1].part_ids, requested.part_ids);
    }

    #[test]
    fn local_mirror_layout_edit_survives_source_reconciliation() {
        let (mut document, projection) = fixed_geometry_fixture().unwrap();
        let source_part_id = "local-layout-source-part";
        let mirror_part_id = "local-layout-mirror-part";
        let mut source_part = document
            .parts
            .iter()
            .find(|part| part.id == "matrix/source-matrix/r0c0")
            .unwrap()
            .clone();
        source_part.id = source_part_id.into();
        source_part.reference = "LOCAL-SOURCE".into();
        let mut mirror_part = document
            .parts
            .iter()
            .find(|part| part.id == "matrix/mirror-matrix/r0c0")
            .unwrap()
            .clone();
        mirror_part.id = mirror_part_id.into();
        mirror_part.reference = "LOCAL-MIRROR".into();
        document.parts.extend([source_part, mirror_part]);
        document
            .boards
            .iter_mut()
            .find(|board| board.id == "source-board")
            .unwrap()
            .part_ids
            .push(source_part_id.into());
        document
            .boards
            .iter_mut()
            .find(|board| board.id == "mirror-board")
            .unwrap()
            .part_ids
            .push(mirror_part_id.into());
        let halves = document.hardware.as_mut().unwrap().pairs[0]
            .halves
            .as_mut()
            .unwrap();
        halves.correspondences.extend([
            MirroredCorrespondence {
                companion_mode: None,
                companion_tweak: None,
                kind: MirroredItemKind::Part,
                source_id: source_part_id.into(),
                mirror_id: mirror_part_id.into(),
            },
            MirroredCorrespondence {
                companion_mode: None,
                companion_tweak: None,
                kind: MirroredItemKind::Layout,
                source_id: "source-local-layout".into(),
                mirror_id: "mirror-local-layout".into(),
            },
        ]);
        halves.exceptions.extend([
            serde_json::from_value(serde_json::json!({
                "kind": "part", "mirrorId": mirror_part_id, "state": "local"
            }))
            .unwrap(),
            serde_json::from_value(serde_json::json!({
                "kind": "layout", "mirrorId": "mirror-local-layout", "state": "local"
            }))
            .unwrap(),
        ]);
        document.layouts = vec![
            Layout {
                id: "source-local-layout".into(),
                name: "Source layout".into(),
                board_id: "source-board".into(),
                matrix_id: "source-matrix".into(),
                part_ids: vec![source_part_id.into()],
                mirror_link: None,
            },
            Layout {
                id: "mirror-local-layout".into(),
                name: "Mirror layout".into(),
                board_id: "mirror-board".into(),
                matrix_id: "mirror-matrix".into(),
                part_ids: vec![mirror_part_id.into(), "geometry-probe".into()],
                mirror_link: None,
            },
        ];

        let requested = Layout {
            name: "Authored local name".into(),
            part_ids: vec!["geometry-probe".into()],
            ..document.layouts[1].clone()
        };
        crate::apply(
            &mut document,
            &[],
            &EditOperation::SetLayout {
                layout: requested.clone(),
            },
        )
        .expect("the local mirror layout edit applies");
        reconcile(&mut document, "pair-source-mirror", &projection)
            .expect("the local layout does not need source projection");

        assert_eq!(document.layouts[1].name, requested.name);
        assert_eq!(document.layouts[1].part_ids, requested.part_ids);
    }

    #[test]
    fn omitted_mirror_layout_stays_absent_after_source_edit_and_reconciliation() {
        let (mut document, projection) = fixed_geometry_fixture().unwrap();
        let mut source_part = document
            .parts
            .iter()
            .find(|part| part.id == "matrix/source-matrix/r0c0")
            .unwrap()
            .clone();
        source_part.id = "omitted-layout-source-part".into();
        source_part.reference = "OMITTED-SOURCE".into();
        document.parts.push(source_part);
        document
            .boards
            .iter_mut()
            .find(|board| board.id == "source-board")
            .unwrap()
            .part_ids
            .push("omitted-layout-source-part".into());
        let halves = document.hardware.as_mut().unwrap().pairs[0]
            .halves
            .as_mut()
            .unwrap();
        halves.correspondences.push(MirroredCorrespondence {
            companion_mode: None,
            companion_tweak: None,
            kind: MirroredItemKind::Layout,
            source_id: "source-omitted-layout".into(),
            mirror_id: "mirror-omitted-layout".into(),
        });
        halves.exceptions.push(
            serde_json::from_value(serde_json::json!({
                "kind": "layout", "mirrorId": "mirror-omitted-layout", "state": "omitted"
            }))
            .unwrap(),
        );
        document.layouts.push(Layout {
            id: "source-omitted-layout".into(),
            name: "Source layout".into(),
            board_id: "source-board".into(),
            matrix_id: "source-matrix".into(),
            part_ids: vec![],
            mirror_link: None,
        });

        let requested = Layout {
            name: "Edited source layout".into(),
            part_ids: vec!["omitted-layout-source-part".into()],
            ..document.layouts[0].clone()
        };
        crate::apply(
            &mut document,
            &[],
            &EditOperation::SetLayout {
                layout: requested.clone(),
            },
        )
        .expect("the source layout edit applies");
        reconcile(&mut document, "pair-source-mirror", &projection)
            .expect("an omitted mirror layout stays absent");

        assert_eq!(document.layouts[0], requested);
        assert!(
            document
                .layouts
                .iter()
                .all(|layout| layout.id != "mirror-omitted-layout")
        );
    }

    #[test]
    fn legacy_layout_link_syncs_components_without_changing_an_unrelated_layout() {
        let mut document = fixture_document().unwrap();
        for id in [
            "legacy-source-matrix",
            "legacy-target-matrix",
            "unrelated-matrix",
        ] {
            crate::matrix::set_matrix(&mut document, &fixture_matrix(id, "source-board")).unwrap();
        }
        let mut source = document
            .parts
            .iter()
            .find(|part| part.id == "matrix/source-matrix/r0c0")
            .unwrap()
            .clone();
        source.id = "legacy-source-part".into();
        source.reference = "J3".into();
        source.pose = Pose2 {
            at: Vec2 { x: 12.0, y: -4.0 },
            rotation: 30.0,
        };
        let mut unrelated = source.clone();
        unrelated.id = "legacy-unrelated-part".into();
        unrelated.reference = "R7".into();
        unrelated.pose = Pose2 {
            at: Vec2 { x: -9.0, y: 21.0 },
            rotation: 72.0,
        };
        document.parts.extend([source.clone(), unrelated.clone()]);
        let source_board = document
            .boards
            .iter_mut()
            .find(|board| board.id == "source-board")
            .unwrap();
        source_board
            .part_ids
            .extend([source.id.clone(), unrelated.id.clone()]);
        document.layouts = vec![
            Layout {
                id: "legacy-source-layout".into(),
                name: "Legacy source".into(),
                board_id: "source-board".into(),
                matrix_id: "legacy-source-matrix".into(),
                part_ids: vec![source.id.clone()],
                mirror_link: None,
            },
            Layout {
                id: "legacy-target-layout".into(),
                name: "Legacy mirror".into(),
                board_id: "source-board".into(),
                matrix_id: "legacy-target-matrix".into(),
                part_ids: vec![],
                mirror_link: Some(LayoutMirrorLink {
                    source_id: "legacy-source-layout".into(),
                    axis_x: 50.0,
                }),
            },
            Layout {
                id: "unrelated-layout".into(),
                name: "Unrelated local".into(),
                board_id: "source-board".into(),
                matrix_id: "unrelated-matrix".into(),
                part_ids: vec![unrelated.id.clone()],
                mirror_link: None,
            },
        ];

        crate::matrix::layout::validate(&document).unwrap();
        let unrelated_layout_before = document.layouts[2].clone();
        let unrelated_part_before = unrelated.clone();
        let unrelated_owner_ids_before: Vec<_> = document
            .boards
            .iter()
            .filter(|board| board.part_ids.contains(&unrelated.id))
            .map(|board| board.id.clone())
            .collect();
        crate::matrix::layout::sync_components(&mut document).unwrap();
        crate::matrix::layout::validate(&document).unwrap();

        let target_id = "legacy-target-layout::mirror::legacy-source-part";
        let mirrored = document
            .parts
            .iter()
            .find(|part| part.id == target_id)
            .expect("the linked independent component is materialized");
        assert_eq!(mirrored.reference, "J3_M");
        assert_eq!(
            mirrored.pose,
            Pose2 {
                at: Vec2 { x: 88.0, y: -4.0 },
                rotation: 150.0,
            }
        );
        assert_eq!(
            mirrored.properties.as_ref().unwrap()["boardstudio.mirroredComponentSource"],
            serde_json::json!("legacy-source-part")
        );
        assert_eq!(document.layouts[1].part_ids, vec![target_id.to_owned()]);
        assert_eq!(document.layouts[2], unrelated_layout_before);
        assert_eq!(
            document
                .parts
                .iter()
                .find(|part| part.id == "legacy-unrelated-part")
                .unwrap(),
            &unrelated_part_before
        );
        let unrelated_owner_ids_after: Vec<_> = document
            .boards
            .iter()
            .filter(|board| board.part_ids.contains(&unrelated.id))
            .map(|board| board.id.clone())
            .collect();
        assert_eq!(unrelated_owner_ids_after, unrelated_owner_ids_before);
    }

    use super::{project_matrix, project_part};
    use crate::model::Part;
    use crate::model::{Matrix, MirroredCorrespondence, MirroredItemKind, Pose2, Vec2};

    fn matrix(id: &str, board_id: &str, rows: u32, columns: u32, origin: Vec2) -> Matrix {
        Matrix {
            id: id.into(),
            name: None,
            rows,
            columns,
            pitch: Vec2 { x: 19.0, y: 19.0 },
            origin,
            definition_id: "switch".into(),
            part_ids: vec![],
            board_id: Some(board_id.into()),
            mirror: None,
            rotation: None,
            edge_gap: None,
            diode_direction: None,
            row_offsets: vec![],
            column_offsets: vec![],
            column_staggers: vec![],
            column_splays: vec![],
            column_origins: vec![],
            cells: vec![],
        }
    }

    #[test]
    fn changed_source_matrix_reprojects_existing_mirror_geometry_and_keeps_its_identity() {
        let source = matrix("left-matrix", "left-board", 2, 3, Vec2 { x: 20.0, y: 7.0 });
        let mirror = matrix("right-matrix", "right-board", 1, 1, Vec2 { x: 5.0, y: 1.0 });
        let correspondence = MirroredCorrespondence {
            companion_mode: None,
            companion_tweak: None,
            kind: MirroredItemKind::Matrix,
            source_id: source.id.clone(),
            mirror_id: mirror.id.clone(),
        };

        let projected = project_matrix(
            &source,
            &mirror,
            &correspondence,
            Pose2 {
                at: Vec2 { x: 200.0, y: 0.0 },
                rotation: 0.0,
            },
        )
        .expect("the existing matrix correspondence can be projected");

        assert_eq!(projected.id, "right-matrix");
        assert_eq!(projected.board_id.as_deref(), Some("right-board"));
        assert_eq!(projected.rows, 2);
        assert_eq!(projected.columns, 3);
        assert_eq!(projected.origin, Vec2 { x: 180.0, y: 7.0 });
    }

    #[test]
    fn a_rotated_pair_map_reprojects_matrix_origin_and_heading_once() {
        let mut source = matrix("left-matrix", "left-board", 2, 3, Vec2 { x: 5.0, y: 2.0 });
        source.rotation = Some(15.0);
        let mirror = matrix("right-matrix", "right-board", 1, 1, Vec2 { x: 5.0, y: 1.0 });
        let correspondence = MirroredCorrespondence {
            companion_mode: None,
            companion_tweak: None,
            kind: MirroredItemKind::Matrix,
            source_id: source.id.clone(),
            mirror_id: mirror.id.clone(),
        };

        let projected = project_matrix(
            &source,
            &mirror,
            &correspondence,
            Pose2 {
                at: Vec2 { x: 20.0, y: -3.0 },
                rotation: 90.0,
            },
        )
        .expect("the existing matrix correspondence can be projected");

        assert_eq!(projected.id, "right-matrix");
        assert_eq!(projected.rows, 2);
        assert_eq!(projected.columns, 3);
        assert_eq!(projected.origin, Vec2 { x: 18.0, y: -8.0 });
        assert_eq!(projected.rotation, Some(75.0));
    }

    #[test]
    fn changed_source_part_pose_keeps_existing_mirror_identity_and_source_parameters() {
        let source = part(
            "left/part",
            "source-definition",
            "U1",
            Pose2 {
                at: Vec2 { x: 20.0, y: 7.0 },
                rotation: 35.0,
            },
            "back",
            9,
        );
        let mirror = part(
            "right/part",
            "mirror-definition",
            "U9",
            Pose2 {
                at: Vec2 { x: 5.0, y: 1.0 },
                rotation: 0.0,
            },
            "front",
            2,
        );
        let correspondence = MirroredCorrespondence {
            companion_mode: None,
            companion_tweak: None,
            kind: MirroredItemKind::Part,
            source_id: source.id.clone(),
            mirror_id: mirror.id.clone(),
        };

        let projected = project_part(
            &source,
            &mirror,
            &correspondence,
            Pose2 {
                at: Vec2 { x: 200.0, y: 0.0 },
                rotation: 0.0,
            },
        )
        .expect("the existing part correspondence can be projected");

        assert_eq!(projected.id, "right/part");
        assert_eq!(projected.definition_id, "mirror-definition");
        assert_eq!(projected.reference, "U9");
        assert_eq!(
            projected.pose,
            Pose2 {
                at: Vec2 { x: 180.0, y: 7.0 },
                rotation: 145.0,
            }
        );
        assert_eq!(projected.side, source.side);
        assert_eq!(projected.generator_parameters, source.generator_parameters);
        assert_eq!(projected.properties, source.properties);
    }

    fn part(
        id: &str,
        definition_id: &str,
        reference: &str,
        pose: Pose2,
        side: &str,
        custom: u64,
    ) -> Part {
        serde_json::from_value(serde_json::json!({
            "id": id,
            "definitionId": definition_id,
            "reference": reference,
            "pose": pose,
            "side": side,
            "properties": {"custom": custom},
            "generatorParameters": {"side": side, "custom": custom}
        }))
        .expect("the compact part fixture is valid")
    }

    #[test]
    fn membership_transition_accepts_a_valid_identity_operation_as_a_no_op() {
        let (document, _) = fixed_geometry_fixture().expect("the fixture is constructed");
        assert!(crate::mechanical::halves::validate_document(&document).is_ok());
        for matrix_id in ["source-matrix", "mirror-matrix"] {
            let matrix = document
                .matrices
                .iter()
                .find(|matrix| matrix.id == matrix_id)
                .expect("the fixture contains both matrices");
            assert!(crate::matrix::valid_matrix(matrix, &document).is_ok());
        }

        let pair = &document.hardware.as_ref().unwrap().pairs[0];
        let halves = pair.halves.as_ref().unwrap();
        let mut context = MembershipContext {
            pair_id: pair.id.clone(),
            source_board_id: halves.source_board_id.clone(),
            mirror_board_id: halves.mirror_board_id.clone(),
            matrix_pair: Some(super::MatrixPairContext {
                source_id: "source-matrix".into(),
                mirror_id: "mirror-matrix".into(),
            }),
            mapping: pair.mapping,
            pair_right: halves.clone(),
            associations: halves
                .correspondences
                .iter()
                .map(|association| TransitionAssociation {
                    kind: association.kind,
                    source_id: association.source_id.clone(),
                    mirror_id: association.mirror_id.clone(),
                    parent_source_id: None,
                    parent_mirror_id: None,
                    state: TransitionState::Linked,
                    explicit_state: false,
                    raw_mode_override: association.companion_mode,
                    raw_tweak: association.companion_tweak,
                    effective_mode: association.companion_mode.unwrap_or(halves.companion_mode),
                    effective_tweak: association.companion_tweak,
                })
                .collect(),
            owners: BTreeMap::new(),
            pending_rejoins: BTreeMap::new(),
            pending_pair_transitions: BTreeMap::new(),
            pending_pair_baseline: None,
        };
        context.associations[1].state = TransitionState::Local;
        context.associations[1].explicit_state = true;
        let constrained_key = context
            .associations
            .iter_mut()
            .find(|association| association.mirror_id == "matrix/mirror-matrix/r1c1")
            .expect("the constrained mirror key has a private association");
        constrained_key.state = TransitionState::Local;
        constrained_key.explicit_state = true;
        let mut source_identity_map = BTreeMap::new();
        for id in &document.matrices[0].part_ids {
            source_identity_map.insert(id.clone(), Some(id.clone()));
        }
        let mirror_identity_map: BTreeMap<_, _> = document
            .matrices
            .iter()
            .find(|matrix| matrix.id == "mirror-matrix")
            .unwrap()
            .part_ids
            .iter()
            .map(|id| (id.clone(), Some(id.clone())))
            .collect();
        let mut final_owners = BTreeMap::new();
        for part in &document.parts {
            let owner = document
                .boards
                .iter()
                .find(|board| board.part_ids.contains(&part.id))
                .expect("every fixture part has one owner");
            final_owners.insert(part.id.clone(), FinalOwner::Board(owner.id.clone()));
        }
        context.owners = final_owners.clone();
        let explicit_removal_closure = BTreeSet::new();
        let planned_additions = [];

        let result = prepare_membership_transition(MembershipTransitionInput {
            valid_before: &document,
            post_source: &document,
            context: &context,
            baseline_kind: BaselineKind::MaterializedCurrentSchema,
            source_identity_map: &source_identity_map,
            mirror_identity_map: &mirror_identity_map,
            planned_additions: &planned_additions,
            final_owners: &final_owners,
            explicit_removal_closure: &explicit_removal_closure,
        });

        assert!(
            result.is_ok(),
            "valid no-op transition was rejected: {result:?}"
        );
        let output = result.unwrap();
        assert_eq!(output.draft, document);
        assert!(output.changed_document_ids.is_empty());
        assert!(output.changed_field_paths.is_empty());
        assert_eq!(output.context, context);
        assert_eq!(output.final_owners, final_owners);
    }

    #[test]
    fn membership_transition_removes_a_whole_paired_matrix_from_a_source_operation() {
        let (mut before, _, mut context, _, _, _) =
            membership_compaction_fixture().expect("the paired matrix fixture is valid");
        crate::matrix::set_matrix(
            &mut before,
            &fixture_matrix("unrelated-matrix", "mirror-board"),
        )
        .expect("the unrelated matrix is materialized through Core");
        let unrelated_matrix = before
            .matrices
            .iter()
            .find(|matrix| matrix.id == "unrelated-matrix")
            .unwrap()
            .clone();
        let unrelated_member = unrelated_matrix.part_ids[0].clone();
        for id in &unrelated_matrix.part_ids {
            context
                .owners
                .insert(id.clone(), FinalOwner::Board("mirror-board".into()));
        }
        let source_member = before
            .matrices
            .iter()
            .find(|matrix| matrix.id == context.matrix_pair.as_ref().unwrap().source_id.as_str())
            .unwrap()
            .part_ids[0]
            .clone();
        let mirror_member = before
            .matrices
            .iter()
            .find(|matrix| matrix.id == context.matrix_pair.as_ref().unwrap().mirror_id.as_str())
            .unwrap()
            .part_ids[0]
            .clone();
        let source_layout = Layout {
            id: "source-layout".into(),
            name: "Source matrix layout".into(),
            board_id: context.source_board_id.clone(),
            matrix_id: context
                .matrix_pair
                .as_ref()
                .unwrap()
                .source_id
                .as_str()
                .to_owned(),
            part_ids: vec![source_member],
            mirror_link: None,
        };
        let mirror_layout = Layout {
            id: "mirror-layout".into(),
            name: "Target matrix layout".into(),
            board_id: context.mirror_board_id.clone(),
            matrix_id: context
                .matrix_pair
                .as_ref()
                .unwrap()
                .mirror_id
                .as_str()
                .to_owned(),
            part_ids: vec![mirror_member],
            mirror_link: None,
        };
        let unrelated_layout = Layout {
            id: "unrelated-layout".into(),
            name: "Unrelated nonempty layout".into(),
            board_id: context.mirror_board_id.clone(),
            matrix_id: unrelated_matrix.id.clone(),
            part_ids: vec![unrelated_member.clone()],
            mirror_link: None,
        };
        before.layouts = vec![source_layout, mirror_layout, unrelated_layout.clone()];
        let unrelated_envelope = OutlineFeature::PartEnvelope {
            connections: vec![],
            settings: Default::default(),
            id: "unrelated-layout-envelope".into(),
            part_ids: vec![unrelated_member.clone()],
            margin: 1.5,
            operation: crate::model::Operation::Add,
        };
        before.outline.push(unrelated_envelope.clone());
        before
            .boards
            .iter_mut()
            .find(|board| board.id == context.mirror_board_id)
            .unwrap()
            .outline_ids
            .push("unrelated-layout-envelope".into());
        let source_net_id = format!(
            "matrix/{}/net/empty",
            context.matrix_pair.as_ref().unwrap().source_id.as_str()
        );
        let mirror_net_id = format!(
            "matrix/{}/net/empty",
            context.matrix_pair.as_ref().unwrap().mirror_id.as_str()
        );
        for (board_id, net_id) in [
            (context.source_board_id.as_str(), source_net_id.as_str()),
            (context.mirror_board_id.as_str(), mirror_net_id.as_str()),
        ] {
            before
                .boards
                .iter_mut()
                .find(|board| board.id == board_id)
                .unwrap()
                .net_ids
                .push(net_id.into());
            before.nets.push(Net {
                id: net_id.into(),
                name: "unused matrix net".into(),
                pins: vec![],
            });
        }
        crate::mechanical::halves::validate_document(&before)
            .expect("the pre-edit pair is a valid materialized project");

        let mut post_source = before.clone();
        let source_changed = crate::apply(
            &mut post_source,
            &[],
            &EditOperation::RemoveMatrix {
                id: context
                    .matrix_pair
                    .as_ref()
                    .unwrap()
                    .source_id
                    .as_str()
                    .to_owned(),
            },
        )
        .expect("the ordinary source RemoveMatrix operation succeeds");
        assert!(
            source_changed
                .iter()
                .any(|id| id == context.matrix_pair.as_ref().unwrap().source_id.as_str())
        );
        assert!(
            !post_source
                .matrices
                .iter()
                .any(|matrix| matrix.id == context.matrix_pair.as_ref().unwrap().source_id.as_str())
        );

        let source_members: BTreeSet<_> = before
            .matrices
            .iter()
            .find(|matrix| matrix.id == context.matrix_pair.as_ref().unwrap().source_id.as_str())
            .expect("the source matrix exists in the pre-edit baseline")
            .part_ids
            .iter()
            .cloned()
            .collect();
        let mirror_members: BTreeSet<_> = before
            .matrices
            .iter()
            .find(|matrix| matrix.id == context.matrix_pair.as_ref().unwrap().mirror_id.as_str())
            .expect("the mirror matrix exists in the pre-edit baseline")
            .part_ids
            .iter()
            .cloned()
            .collect();
        let source_identity_map: BTreeMap<_, _> = source_members
            .iter()
            .cloned()
            .map(|id| (id, None))
            .collect();
        let mirror_identity_map: BTreeMap<_, _> = mirror_members
            .iter()
            .cloned()
            .map(|id| (id, None))
            .collect();
        let mut final_owners = context.owners.clone();
        for id in source_members.iter().chain(&mirror_members) {
            final_owners.insert(id.clone(), FinalOwner::Absent);
        }
        let explicit_removal_closure: BTreeSet<_> = source_members
            .iter()
            .chain(&mirror_members)
            .cloned()
            .collect();

        let incomplete_closure: BTreeSet<_> =
            explicit_removal_closure.iter().skip(1).cloned().collect();
        assert!(
            prepare_membership_transition_kernel(MembershipTransitionInput {
                valid_before: &before,
                post_source: &post_source,
                context: &context,
                baseline_kind: BaselineKind::MaterializedCurrentSchema,
                source_identity_map: &source_identity_map,
                mirror_identity_map: &mirror_identity_map,
                planned_additions: &[],
                final_owners: &final_owners,
                explicit_removal_closure: &incomplete_closure,
            })
            .is_err()
        );
        assert!(
            !post_source
                .matrices
                .iter()
                .any(|matrix| matrix.id == context.matrix_pair.as_ref().unwrap().source_id.as_str())
        );

        let output = prepare_membership_transition_kernel(MembershipTransitionInput {
            valid_before: &before,
            post_source: &post_source,
            context: &context,
            baseline_kind: BaselineKind::MaterializedCurrentSchema,
            source_identity_map: &source_identity_map,
            mirror_identity_map: &mirror_identity_map,
            planned_additions: &[],
            final_owners: &final_owners,
            explicit_removal_closure: &explicit_removal_closure,
        })
        .expect("removing the source matrix projects its paired mirror removal");

        assert!(
            !output
                .draft
                .matrices
                .iter()
                .any(|matrix| matrix.id == context.matrix_pair.as_ref().unwrap().source_id.as_str())
        );
        assert!(
            !output
                .draft
                .matrices
                .iter()
                .any(|matrix| matrix.id == context.matrix_pair.as_ref().unwrap().mirror_id.as_str())
        );
        assert!(
            output
                .draft
                .parts
                .iter()
                .all(|part| !mirror_members.contains(&part.id))
        );
        assert_eq!(
            output
                .draft
                .matrices
                .iter()
                .find(|matrix| matrix.id == unrelated_matrix.id),
            Some(&unrelated_matrix)
        );
        assert_eq!(
            output
                .draft
                .layouts
                .iter()
                .find(|layout| layout.id == unrelated_layout.id),
            Some(&unrelated_layout)
        );
        assert!(
            !output
                .draft
                .layouts
                .iter()
                .any(|layout| layout.id == "mirror-layout")
        );
        assert!(!output.draft.nets.iter().any(|net| net.id == mirror_net_id));
        let retained_net = output
            .draft
            .nets
            .iter()
            .find(|net| net.id == "retained-second-key-net")
            .expect("the nonempty unrelated net survives target cleanup");
        assert_eq!(
            retained_net.pins,
            vec![Pin {
                part_id: "geometry-probe".into(),
                pad_id: "pad-two".into(),
            }]
        );
        assert!(
            !output
                .draft
                .constraints
                .iter()
                .any(|constraint| { constraint.id() == "retained-second-key-offset" })
        );
        let mut expected_retained_envelope = post_source
            .outline
            .iter()
            .find(|feature| {
                matches!(feature, OutlineFeature::PartEnvelope { id, .. }
                    if id == "retained-keys-envelope")
            })
            .expect("the source operation leaves the mirror envelope in its preimage")
            .clone();
        if let OutlineFeature::PartEnvelope { part_ids, .. } = &mut expected_retained_envelope {
            part_ids.retain(|id| !mirror_members.contains(id));
        }
        assert!(output.draft.outline.contains(&expected_retained_envelope));
        assert!(output.draft.outline.contains(&unrelated_envelope));
        assert!(
            output
                .draft
                .boards
                .iter()
                .any(|board| board.part_ids.contains(&unrelated_member))
        );
        assert!(
            output
                .changed_document_ids
                .iter()
                .any(|id| id == context.matrix_pair.as_ref().unwrap().mirror_id.as_str())
        );
        assert!(
            output
                .changed_document_ids
                .contains(&"mirror-layout".to_owned())
        );
        assert!(output.changed_document_ids.contains(&mirror_net_id));
        for id in &mirror_members {
            assert!(output.changed_document_ids.contains(id));
        }
        let expected_target_delta: BTreeSet<_> = mirror_members
            .iter()
            .cloned()
            .chain([
                context
                    .matrix_pair
                    .as_ref()
                    .unwrap()
                    .mirror_id
                    .as_str()
                    .to_owned(),
                "mirror-layout".into(),
                mirror_net_id.clone(),
                context.mirror_board_id.clone(),
                "retained-keys-envelope".into(),
                "retained-second-key-net".into(),
                "retained-second-key-offset".into(),
            ])
            .collect();
        assert_eq!(
            output.changed_document_ids,
            expected_target_delta.into_iter().collect::<Vec<_>>()
        );
        assert!(
            source_members
                .iter()
                .all(|id| !output.changed_document_ids.contains(id))
        );
        assert!(output.context.associations.iter().all(|association| {
            association.source_id != context.matrix_pair.as_ref().unwrap().source_id.as_str()
                && association.mirror_id != context.matrix_pair.as_ref().unwrap().mirror_id.as_str()
                && !source_members.contains(&association.source_id)
                && !mirror_members.contains(&association.mirror_id)
        }));
        assert_eq!(output.final_owners, final_owners);
        assert_eq!(
            output.context.pair_right.correspondences, context.pair_right.correspondences,
            "the private retirement leaves persisted pair provenance byte-for-byte intact"
        );
        let repeated = prepare_membership_transition_kernel(MembershipTransitionInput {
            valid_before: &output.draft,
            post_source: &output.draft,
            context: &output.context,
            baseline_kind: BaselineKind::MaterializedCurrentSchema,
            source_identity_map: &BTreeMap::new(),
            mirror_identity_map: &BTreeMap::new(),
            planned_additions: &[],
            final_owners: &output.final_owners,
            explicit_removal_closure: &BTreeSet::new(),
        })
        .expect("the retired matrix transition converges in its updated context");
        assert_eq!(repeated.draft, output.draft);
        assert!(repeated.changed_document_ids.is_empty());
        assert_eq!(repeated.context, output.context);
    }

    #[test]
    fn membership_transition_retires_a_whole_pair_but_preserves_local_mirror_parts() {
        let (before, _, mut context, _, _, _) =
            membership_compaction_fixture().expect("the paired matrix fixture is valid");
        let source_members: BTreeSet<_> = before
            .matrices
            .iter()
            .find(|matrix| matrix.id == context.matrix_pair.as_ref().unwrap().source_id.as_str())
            .unwrap()
            .part_ids
            .iter()
            .cloned()
            .collect();
        let mirror_members: BTreeSet<_> = before
            .matrices
            .iter()
            .find(|matrix| matrix.id == context.matrix_pair.as_ref().unwrap().mirror_id.as_str())
            .unwrap()
            .part_ids
            .iter()
            .cloned()
            .collect();
        let local_association = context
            .associations
            .iter_mut()
            .find(|association| {
                association.kind == MirroredItemKind::Part
                    && association.source_id == "matrix/source-matrix/r0c3"
            })
            .expect("the selected key has a paired Part association");
        local_association.state = TransitionState::Local;
        local_association.explicit_state = true;
        let local_source_id = local_association.source_id.clone();
        let local_mirror_id = local_association.mirror_id.clone();
        let local_child_source_id = format!("{local_source_id}/diode");
        let local_child_mirror_id = format!("{local_mirror_id}/diode");
        let local_child = context
            .associations
            .iter_mut()
            .find(|association| association.source_id == local_child_source_id)
            .expect("the Local key has a materialized companion association");
        local_child.state = TransitionState::Local;
        local_child.explicit_state = false;
        let retained_parts =
            BTreeSet::from([local_mirror_id.clone(), local_child_mirror_id.clone()]);

        let mut before = before;
        before.nets.push(Net {
            id: "retained-local-net".into(),
            name: "retained Local PadID net".into(),
            pins: vec![Pin {
                part_id: local_child_mirror_id.clone(),
                pad_id: "pad-two".into(),
            }],
        });
        before
            .boards
            .iter_mut()
            .find(|board| board.id == context.mirror_board_id)
            .unwrap()
            .net_ids
            .push("retained-local-net".into());
        before.constraints.push(Constraint::Offset {
            id: "retained-local-constraint".into(),
            source_part_id: local_mirror_id.clone(),
            target_part_id: local_child_mirror_id.clone(),
            offset: Vec2 { x: 1.25, y: -2.5 },
            rotation: 7.0,
        });
        before.outline.push(OutlineFeature::PartEnvelope {
            connections: vec![],
            settings: crate::model::OutlineSettings::default(),
            id: "retained-local-outline".into(),
            part_ids: vec![local_mirror_id.clone(), local_child_mirror_id.clone()],
            margin: 1.75,
            operation: crate::model::Operation::Add,
        });
        crate::mechanical::halves::validate_document(&before)
            .expect("the retained Local dependency controls are valid before removal");

        let mut post_source = before.clone();
        crate::apply(
            &mut post_source,
            &[],
            &EditOperation::RemoveMatrix {
                id: context
                    .matrix_pair
                    .as_ref()
                    .unwrap()
                    .source_id
                    .as_str()
                    .to_owned(),
            },
        )
        .expect("the source operation removes its matrix through Core");
        let source_identity_map: BTreeMap<_, _> = source_members
            .iter()
            .cloned()
            .map(|id| (id, None))
            .collect();
        let mirror_identity_map: BTreeMap<_, _> = mirror_members
            .iter()
            .cloned()
            .map(|id| {
                let mapped = retained_parts.contains(&id).then(|| id.clone());
                (id, mapped)
            })
            .collect();
        let mut final_owners = context.owners.clone();
        for id in &source_members {
            final_owners.insert(id.clone(), FinalOwner::Absent);
        }
        for id in &mirror_members {
            final_owners.insert(
                id.clone(),
                if retained_parts.contains(id) {
                    FinalOwner::Board(context.mirror_board_id.clone())
                } else {
                    FinalOwner::Absent
                },
            );
        }
        let explicit_removal_closure: BTreeSet<_> = source_members
            .iter()
            .chain(
                mirror_members
                    .iter()
                    .filter(|id| !retained_parts.contains(*id)),
            )
            .cloned()
            .collect();
        let output = prepare_membership_transition_kernel(MembershipTransitionInput {
            valid_before: &before,
            post_source: &post_source,
            context: &context,
            baseline_kind: BaselineKind::MaterializedCurrentSchema,
            source_identity_map: &source_identity_map,
            mirror_identity_map: &mirror_identity_map,
            planned_additions: &[],
            final_owners: &final_owners,
            explicit_removal_closure: &explicit_removal_closure,
        })
        .expect("a Local mirror identity survives paired matrix retirement");

        assert!(!output.draft.matrices.iter().any(|matrix| matrix.id
            == context.matrix_pair.as_ref().unwrap().source_id.as_str()
            || matrix.id == context.matrix_pair.as_ref().unwrap().mirror_id.as_str()));
        assert!(
            !output
                .draft
                .parts
                .iter()
                .any(|part| part.id == local_source_id)
        );
        assert_eq!(
            output
                .draft
                .parts
                .iter()
                .find(|part| part.id == local_mirror_id),
            before.parts.iter().find(|part| part.id == local_mirror_id)
        );
        assert_eq!(
            output
                .draft
                .parts
                .iter()
                .find(|part| part.id == local_child_mirror_id),
            before
                .parts
                .iter()
                .find(|part| part.id == local_child_mirror_id)
        );
        assert!(
            output
                .draft
                .boards
                .iter()
                .find(|board| board.id == context.mirror_board_id)
                .unwrap()
                .part_ids
                .contains(&local_mirror_id)
        );
        assert!(
            output
                .draft
                .boards
                .iter()
                .find(|board| board.id == context.mirror_board_id)
                .unwrap()
                .part_ids
                .contains(&local_child_mirror_id)
        );
        assert!(output.draft.nets.iter().any(|net| {
            net.id == "retained-local-net"
                && net
                    .pins
                    .iter()
                    .any(|pin| pin.part_id == local_child_mirror_id && pin.pad_id == "pad-two")
        }));
        assert!(output.draft.constraints.iter().any(|constraint| {
            constraint.id() == "retained-local-constraint"
                && constraint.source() == local_mirror_id
                && constraint.target() == local_child_mirror_id
        }));
        assert!(output.draft.outline.iter().any(|feature| {
            feature.id() == "retained-local-outline"
                && matches!(feature, OutlineFeature::PartEnvelope { part_ids, .. } if part_ids.contains(&local_mirror_id) && part_ids.contains(&local_child_mirror_id))
        }));
        assert!(output.context.associations.iter().any(|association| {
            association.source_id == local_source_id
                && association.mirror_id == local_mirror_id
                && association.state == TransitionState::Local
        }));
        assert_eq!(output.final_owners, final_owners);
        assert!(
            output
                .changed_document_ids
                .iter()
                .any(|id| id == context.matrix_pair.as_ref().unwrap().mirror_id.as_str())
        );
        assert!(!output.changed_document_ids.contains(&local_mirror_id));
        assert!(!output.changed_document_ids.contains(&local_child_mirror_id));

        let repeated = prepare_membership_transition_kernel(MembershipTransitionInput {
            valid_before: &output.draft,
            post_source: &output.draft,
            context: &output.context,
            baseline_kind: BaselineKind::MaterializedCurrentSchema,
            source_identity_map: &BTreeMap::new(),
            mirror_identity_map: &BTreeMap::new(),
            planned_additions: &[],
            final_owners: &output.final_owners,
            explicit_removal_closure: &BTreeSet::new(),
        })
        .expect("retained Local ownership converges after matrix retirement");
        assert_eq!(repeated.draft, output.draft);
        assert!(repeated.changed_document_ids.is_empty());
        assert_eq!(repeated.context, output.context);
    }

    #[test]
    fn membership_transition_rejects_incomplete_and_aliased_maps_before_staging() {
        let (before, post_source, context, source_map, final_owners, closure) =
            membership_compaction_fixture().expect("the valid transition fixture is constructed");
        let mirror_map: BTreeMap<_, _> = source_map
            .iter()
            .map(|(old, new)| {
                let old_mirror = old.replace("matrix/source-matrix/", "matrix/mirror-matrix/");
                let new_mirror = new
                    .as_ref()
                    .map(|id| id.replace("matrix/source-matrix/", "matrix/mirror-matrix/"));
                (old_mirror, new_mirror)
            })
            .collect();
        let original_before = before.clone();
        let original_post = post_source.clone();
        let original_context = context.clone();

        let mut incomplete = source_map.clone();
        incomplete.pop_first();
        let error = prepare_membership_transition(MembershipTransitionInput {
            valid_before: &before,
            post_source: &post_source,
            context: &context,
            baseline_kind: BaselineKind::MaterializedCurrentSchema,
            source_identity_map: &incomplete,
            mirror_identity_map: &mirror_map,
            planned_additions: &[],
            final_owners: &final_owners,
            explicit_removal_closure: &closure,
        })
        .expect_err("an incomplete historical source map is refused");
        assert!(
            error.contains("does not cover every old matrix member"),
            "{error}"
        );

        let mut aliased = source_map.clone();
        let mut survivors = aliased
            .iter()
            .filter_map(|(old, new)| new.as_ref().map(|_| old.clone()));
        let first = survivors.next().unwrap();
        let second = survivors.next().unwrap();
        let target = aliased[&first].clone();
        aliased.insert(second, target);
        let error = prepare_membership_transition(MembershipTransitionInput {
            valid_before: &before,
            post_source: &post_source,
            context: &context,
            baseline_kind: BaselineKind::MaterializedCurrentSchema,
            source_identity_map: &aliased,
            mirror_identity_map: &mirror_map,
            planned_additions: &[],
            final_owners: &final_owners,
            explicit_removal_closure: &closure,
        })
        .expect_err("an ambiguous survivor map is refused");
        assert!(error.contains("not injective"), "{error}");

        let mut non_finite_context = context.clone();
        let association = non_finite_context
            .associations
            .iter_mut()
            .find(|association| association.kind == MirroredItemKind::Part)
            .unwrap();
        association.raw_tweak = Some(MirroredCompanionTweak {
            x: f64::NAN,
            y: 0.0,
            rotation: 0.0,
        });
        association.effective_mode = MirroredCompanionMode::LiteralMirrorWithTweak;
        association.effective_tweak = association.raw_tweak;
        let error = prepare_membership_transition(MembershipTransitionInput {
            valid_before: &before,
            post_source: &post_source,
            context: &non_finite_context,
            baseline_kind: BaselineKind::MaterializedCurrentSchema,
            source_identity_map: &source_map,
            mirror_identity_map: &mirror_map,
            planned_additions: &[],
            final_owners: &final_owners,
            explicit_removal_closure: &closure,
        })
        .expect_err("non-finite captured settings are refused");
        assert!(error.contains("invalid settings"), "{error}");

        let mut stale_before = before.clone();
        stale_before.hardware.as_mut().unwrap().pairs[0]
            .mapping
            .at
            .x += 1.0;
        let error = prepare_membership_transition(MembershipTransitionInput {
            valid_before: &stale_before,
            post_source: &post_source,
            context: &context,
            baseline_kind: BaselineKind::MaterializedCurrentSchema,
            source_identity_map: &source_map,
            mirror_identity_map: &mirror_map,
            planned_additions: &[],
            final_owners: &final_owners,
            explicit_removal_closure: &closure,
        })
        .expect_err("a stale pre-edit pair binding is refused");
        assert!(
            error.contains("changed outside its private context"),
            "{error}"
        );

        let mut constrained_before = before.clone();
        let constraint = Constraint::Offset {
            id: "surviving-linked-placement".into(),
            source_part_id: "geometry-probe".into(),
            target_part_id: "matrix/mirror-matrix/r0c0/diode".into(),
            offset: Vec2 { x: 99.0, y: -35.0 },
            rotation: 90.0,
        };
        constrained_before.constraints.push(constraint.clone());
        let mut constrained_post_source = post_source.clone();
        constrained_post_source.constraints.push(constraint);
        let error = prepare_membership_transition(MembershipTransitionInput {
            valid_before: &constrained_before,
            post_source: &constrained_post_source,
            context: &context,
            baseline_kind: BaselineKind::MaterializedCurrentSchema,
            source_identity_map: &source_map,
            mirror_identity_map: &mirror_map,
            planned_additions: &[],
            final_owners: &final_owners,
            explicit_removal_closure: &closure,
        })
        .expect_err("a surviving linked constraint is refused atomically");
        assert!(error.contains("constrained geometry"), "{error}");
        assert!(
            constrained_before
                .constraints
                .iter()
                .any(|constraint| constraint.id() == "surviving-linked-placement")
        );
        assert_eq!(before, original_before);
        assert_eq!(post_source, original_post);
        assert_eq!(context, original_context);
    }

    #[test]
    fn document_diff_reports_idless_nested_binding_paths() {
        let (mut before, _) = fixed_geometry_fixture().expect("the fixture is valid");
        let mut keymap = crate::keymap::KeymapConfiguration::default();
        keymap.layers[0].bindings.insert(
            "matrix/mirror-matrix/r0c0".into(),
            crate::keymap::KeyBinding::KeyPress {
                keycode: "A".into(),
            },
        );
        before.keymap = Some(keymap.clone());
        let mut after = before.clone();
        keymap.layers[0].bindings.insert(
            "matrix/mirror-matrix/r0c0".into(),
            crate::keymap::KeyBinding::KeyPress {
                keycode: "B".into(),
            },
        );
        after.keymap = Some(keymap);
        let (ids, paths) = changed_document_fields(&before, &after).unwrap();
        assert!(ids.is_empty());
        assert_eq!(paths.len(), 1);
        assert!(paths[0].contains("keymap"));
        assert!(paths[0].contains("matrix/mirror-matrix/r0c0"));
    }

    type CompactionFixture = (
        ProjectDoc,
        ProjectDoc,
        MembershipContext,
        BTreeMap<String, Option<String>>,
        BTreeMap<String, FinalOwner>,
        BTreeSet<String>,
    );

    fn membership_compaction_fixture() -> Result<CompactionFixture, String> {
        let (mut before, _) = fixed_geometry_fixture()?;
        let mut diode = before.definitions[0].clone();
        diode.id = "diode".into();
        diode.name = "Diode".into();
        before.definitions.push(diode);
        let matrix_association = before.hardware.as_ref().unwrap().pairs[0]
            .halves
            .as_ref()
            .unwrap()
            .correspondences
            .iter()
            .find(|association| association.kind == MirroredItemKind::Matrix)
            .unwrap()
            .clone();
        let matrix_cells = || {
            (0..2)
                .flat_map(|row| {
                    (0..4).map(move |column| MatrixCell {
                        deleted: false,
                        row,
                        column,
                        enabled: true,
                        definition_id: None,
                        variant: None,
                        offset: None,
                        rotation: None,
                        assemblies: vec![MatrixAssembly {
                            id: "diode".into(),
                            definition_id: "diode".into(),
                            offset: Vec2 { x: 2.0, y: 3.0 },
                            rotation: Some(30.0),
                            side: Some(Side::Back),
                        }],
                        assemblies_local: None,
                    })
                })
                .collect::<Vec<_>>()
        };
        let mut source_matrix = before
            .matrices
            .iter()
            .find(|matrix| matrix.id == "source-matrix")
            .unwrap()
            .clone();
        source_matrix.rows = 2;
        source_matrix.columns = 4;
        source_matrix.pitch = Vec2 { x: 10.0, y: 20.0 };
        source_matrix.origin = Vec2 { x: 5.0, y: 1.0 };
        source_matrix.row_offsets.clear();
        source_matrix.column_offsets.clear();
        source_matrix.column_staggers.clear();
        source_matrix.column_splays.clear();
        source_matrix.column_origins.clear();
        source_matrix.cells = matrix_cells();
        crate::matrix::set_matrix(&mut before, &source_matrix)?;
        let mut mirror_matrix = before
            .matrices
            .iter()
            .find(|matrix| matrix.id == "mirror-matrix")
            .unwrap()
            .clone();
        mirror_matrix.rows = 2;
        mirror_matrix.columns = 4;
        mirror_matrix.cells = matrix_cells();
        let mapping = before.hardware.as_ref().unwrap().pairs[0].mapping;
        let mut mirror_matrix =
            project_matrix(&source_matrix, &mirror_matrix, &matrix_association, mapping)?;
        // Keep the materialized baseline consistent with the per-association mode data below:
        // B duplicates the source placement; D mirrors it and then applies its tweak.
        for cell in &mut mirror_matrix.cells {
            let Some(diode) = cell
                .assemblies
                .iter_mut()
                .find(|assembly| assembly.id == "diode")
            else {
                continue;
            };
            match cell.column {
                1 => {
                    diode.offset = Vec2 { x: 2.0, y: 3.0 };
                    diode.rotation = Some(30.0);
                }
                3 => {
                    diode.offset = Vec2 { x: -1.0, y: 1.0 };
                    diode.rotation = Some(-15.0);
                }
                _ => {}
            }
        }
        crate::matrix::set_matrix(&mut before, &mirror_matrix)?;
        let halves = before.hardware.as_mut().unwrap().pairs[0]
            .halves
            .as_mut()
            .unwrap();
        halves.companion_mode = MirroredCompanionMode::LiteralMirror;
        halves.correspondences = vec![MirroredCorrespondence {
            companion_mode: None,
            companion_tweak: None,
            kind: MirroredItemKind::Matrix,
            source_id: "source-matrix".into(),
            mirror_id: "mirror-matrix".into(),
        }];
        for row in 0..2 {
            for column in 0..4 {
                let source_key = format!("matrix/source-matrix/r{row}c{column}");
                let mirror_key = format!("matrix/mirror-matrix/r{row}c{column}");
                halves.correspondences.push(MirroredCorrespondence {
                    companion_mode: None,
                    companion_tweak: None,
                    kind: MirroredItemKind::Part,
                    source_id: source_key.clone(),
                    mirror_id: mirror_key.clone(),
                });
                let (mode, tweak) = if column == 1 {
                    (Some(MirroredCompanionMode::DuplicatePosition), None)
                } else if column == 2 {
                    (Some(MirroredCompanionMode::LiteralMirror), None)
                } else if column == 3 {
                    (
                        Some(MirroredCompanionMode::LiteralMirrorWithTweak),
                        Some(MirroredCompanionTweak {
                            x: 1.0,
                            y: -2.0,
                            rotation: 15.0,
                        }),
                    )
                } else {
                    (None, None)
                };
                halves.correspondences.push(MirroredCorrespondence {
                    companion_mode: mode,
                    companion_tweak: tweak,
                    kind: MirroredItemKind::Part,
                    source_id: format!("{source_key}/diode"),
                    mirror_id: format!("{mirror_key}/diode"),
                });
            }
        }
        for part in &mut before.parts {
            part.properties = Some(BTreeMap::from([(
                "logical-payload".into(),
                serde_json::json!(part.id.clone()),
            )]));
            part.generator_parameters = Some(BTreeMap::from([(
                "logical-payload".into(),
                serde_json::json!(part.id.clone()),
            )]));
            part.locked = Some(true);
            part.keycap = Some(Vec2 { x: 18.0, y: 17.0 });
        }
        crate::mechanical::halves::validate_document(&before)?;

        let pair = &before.hardware.as_ref().unwrap().pairs[0];
        let halves = pair.halves.as_ref().unwrap();
        let mut associations = Vec::new();
        for association in &halves.correspondences {
            let is_child = association.source_id.ends_with("/diode");
            associations.push(TransitionAssociation {
                kind: association.kind,
                source_id: association.source_id.clone(),
                mirror_id: association.mirror_id.clone(),
                parent_source_id: is_child
                    .then(|| association.source_id.trim_end_matches("/diode").to_owned()),
                parent_mirror_id: is_child
                    .then(|| association.mirror_id.trim_end_matches("/diode").to_owned()),
                state: TransitionState::Linked,
                explicit_state: false,
                raw_mode_override: association.companion_mode,
                raw_tweak: association.companion_tweak,
                effective_mode: association.companion_mode.unwrap_or(halves.companion_mode),
                effective_tweak: association.companion_tweak,
            });
        }
        let mut context = MembershipContext {
            pair_id: pair.id.clone(),
            source_board_id: halves.source_board_id.clone(),
            mirror_board_id: halves.mirror_board_id.clone(),
            matrix_pair: Some(super::MatrixPairContext {
                source_id: "source-matrix".into(),
                mirror_id: "mirror-matrix".into(),
            }),
            mapping: pair.mapping,
            pair_right: halves.clone(),
            associations,
            owners: BTreeMap::new(),
            pending_rejoins: BTreeMap::new(),
            pending_pair_transitions: BTreeMap::new(),
            pending_pair_baseline: None,
        };
        let mut context_owners = BTreeMap::new();
        for part in &before.parts {
            let owner = before
                .boards
                .iter()
                .find(|board| board.part_ids.contains(&part.id))
                .unwrap();
            context_owners.insert(part.id.clone(), FinalOwner::Board(owner.id.clone()));
        }
        context.owners = context_owners;

        let source = before
            .matrices
            .iter()
            .find(|matrix| matrix.id == "source-matrix")
            .unwrap();
        let mut identity_map = BTreeMap::new();
        for id in &source.part_ids {
            let (row, column, companion) =
                super::matrix_identity_slot(id, "source-matrix").unwrap();
            let next = if column == 1 {
                None
            } else {
                let next_column = if column > 1 { column - 1 } else { column };
                let suffix = if companion.is_empty() {
                    String::new()
                } else {
                    format!("/{companion}")
                };
                Some(format!("matrix/source-matrix/r{row}c{next_column}{suffix}"))
            };
            identity_map.insert(id.clone(), next);
        }
        let mut post_source = before.clone();
        crate::matrix::deletion::remove_axis(&mut post_source, "source-matrix", 1, false)?;
        let mut final_owners = BTreeMap::new();
        let mut identity_universe: BTreeSet<_> =
            before.parts.iter().map(|part| part.id.clone()).collect();
        identity_universe.extend(post_source.parts.iter().map(|part| part.id.clone()));
        let mut final_matrix_ids = BTreeSet::new();
        for id in identity_map.values().flatten() {
            final_matrix_ids.insert(id.clone());
            let (row, column, child) = super::matrix_identity_slot(id, "source-matrix").unwrap();
            let suffix = if child.is_empty() {
                String::new()
            } else {
                format!("/{child}")
            };
            final_matrix_ids.insert(format!("matrix/mirror-matrix/r{row}c{column}{suffix}"));
        }
        let prior_matrix_ids: BTreeSet<_> = before
            .matrices
            .iter()
            .filter(|matrix| matrix.id == "source-matrix" || matrix.id == "mirror-matrix")
            .flat_map(|matrix| matrix.part_ids.iter().cloned())
            .collect();
        for id in identity_universe {
            let owner = if final_matrix_ids.contains(&id) {
                FinalOwner::Board(if id.starts_with("matrix/source-matrix/") {
                    "source-board".into()
                } else {
                    "mirror-board".into()
                })
            } else if prior_matrix_ids.contains(&id) {
                FinalOwner::Absent
            } else {
                post_source
                    .boards
                    .iter()
                    .find(|board| board.part_ids.contains(&id))
                    .map(|board| FinalOwner::Board(board.id.clone()))
                    .unwrap_or(FinalOwner::Absent)
            };
            final_owners.insert(id, owner);
        }
        let mut explicit_closure = BTreeSet::new();
        for association in &context.associations {
            if identity_map.get(&association.source_id) == Some(&None) {
                explicit_closure.insert(association.source_id.clone());
                explicit_closure.insert(association.mirror_id.clone());
            } else if let Some(mapped_source) = identity_map
                .get(&association.source_id)
                .and_then(Option::as_ref)
            {
                let mapped_mirror = mapped_source
                    .strip_prefix("matrix/source-matrix/")
                    .map(|suffix| format!("matrix/mirror-matrix/{suffix}"))
                    .unwrap();
                if mapped_mirror != association.mirror_id {
                    explicit_closure.insert(association.mirror_id.clone());
                }
            }
        }
        Ok((
            before,
            post_source,
            context,
            identity_map,
            final_owners,
            explicit_closure,
        ))
    }

    fn add_membership_cleanup_sentinels(document: &mut ProjectDoc) {
        let removed = "matrix/mirror-matrix/r0c1/diode";
        let shifted = "matrix/mirror-matrix/r0c2/diode";
        let mut keymap = KeymapConfiguration::default();
        keymap.layers[0].bindings.insert(
            removed.into(),
            KeyBinding::KeyPress {
                keycode: "A".into(),
            },
        );
        keymap.layers[0].bindings.insert(
            shifted.into(),
            KeyBinding::KeyPress {
                keycode: "B".into(),
            },
        );
        keymap.layers[0].sensors.insert(
            removed.into(),
            EncoderBinding {
                clockwise: KeyBinding::KeyPress {
                    keycode: "C".into(),
                },
                counterclockwise: KeyBinding::KeyPress {
                    keycode: "D".into(),
                },
            },
        );
        keymap.layers[0].sensors.insert(
            shifted.into(),
            EncoderBinding {
                clockwise: KeyBinding::KeyPress {
                    keycode: "E".into(),
                },
                counterclockwise: KeyBinding::KeyPress {
                    keycode: "F".into(),
                },
            },
        );
        document.keymap = Some(keymap);
        let mut keycaps = KeycapConfiguration::default();
        keycaps.keys.insert(
            removed.into(),
            KeycapKeySettings {
                legend: Some("removed".into()),
                ..KeycapKeySettings::default()
            },
        );
        keycaps.keys.insert(
            shifted.into(),
            KeycapKeySettings {
                legend: Some("shifted sentinel".into()),
                ..KeycapKeySettings::default()
            },
        );
        document.keycaps = Some(keycaps);
        let hardware = document.hardware.as_mut().unwrap();
        hardware.instances[1].controller_part_id = Some(removed.into());
        let mut electrical = ElectricalBoardConfiguration {
            board_id: "mirror-board".into(),
            ..ElectricalBoardConfiguration::default()
        };
        electrical
            .locks
            .insert(format!("direct/{removed}"), "lock sentinel".into());
        electrical
            .assignments
            .insert(format!("link/{shifted}"), "assignment sentinel".into());
        electrical
            .key_bindings
            .insert(shifted.into(), "binding sentinel".into());
        hardware.boards.push(electrical);
        document.embedded_circuits.push(EmbeddedCircuit {
            id: "closure-circuit".into(),
            definition_id: "closure-definition".into(),
            host_board_id: "mirror-board".into(),
            part_ids: vec![removed.into(), shifted.into()],
            net_ids: vec![],
            ports: BTreeMap::new(),
        });
        let settings = OutlineSettings {
            size: 4.25,
            bridge_width: 12.5,
            repair: Some(OutlineRepairSettings {
                enabled: true,
                maximum_gap_span: 23.0,
                minimum_connection_width: 2.75,
                edge_clearance: 0.4,
                keep_gaps: vec![ProtectedOutlineGap {
                    id: "closure-settings-gap".into(),
                    points: vec![OutlineControlPoint {
                        at: Vec2 { x: 1.0, y: 2.0 },
                        part_id: Some(removed.into()),
                    }],
                }],
            }),
            ..OutlineSettings::default()
        };
        let feature = OutlineFeature::PartEnvelope {
            id: "closure-envelope".into(),
            part_ids: vec![removed.into(), shifted.into()],
            margin: 2.25,
            operation: Operation::Add,
            settings: settings.clone(),
            connections: vec![OutlineConnection {
                id: "closure-connection".into(),
                width: 3.5,
                points: vec![
                    OutlineControlPoint {
                        at: Vec2 { x: 3.0, y: 4.0 },
                        part_id: Some(removed.into()),
                    },
                    OutlineControlPoint {
                        at: Vec2 { x: 5.0, y: 6.0 },
                        part_id: Some(shifted.into()),
                    },
                ],
            }],
        };
        document.outline.push(feature.clone());
        document
            .boards
            .iter_mut()
            .find(|board| board.id == "mirror-board")
            .unwrap()
            .outline_ids
            .push("closure-envelope".into());
        let snapshot = OutlineSnapshot {
            features: vec![feature],
            settings,
            expected_regions: 1,
            bridges: vec![OutlineBridge {
                id: "closure-bridge".into(),
                width: 2.75,
                points: vec![Vec2 { x: 7.0, y: 8.0 }],
                part_ids: vec![removed.into(), shifted.into()],
                matrix_ids: vec!["mirror-matrix".into()],
                authored: true,
            }],
            protected_gaps: vec![ProtectedOutlineGap {
                id: "closure-snapshot-gap".into(),
                points: vec![OutlineControlPoint {
                    at: Vec2 { x: 9.0, y: 10.0 },
                    part_id: Some(shifted.into()),
                }],
            }],
        };
        document.board_outlines.push(BoardOutline {
            board_id: "mirror-board".into(),
            active_version_id: Some("closure-version".into()),
            versions: vec![OutlineVersion {
                id: "closure-version".into(),
                name: "Closure sentinel".into(),
                source: OutlineProvenance {
                    revision: 12,
                    version_id: None,
                },
                geometry: snapshot.clone(),
                ownership: OutlineOwnership::Fixed,
            }],
            generated_last_valid: Some(snapshot),
            retired_version_ids: vec![],
        });
        for (id, connector) in [("closure-module", removed), ("shifted-module", shifted)] {
            document.modules.push(serde_json::from_value(serde_json::json!({
                "id":id, "definitionId":"closure-definition", "hostBoardId":"mirror-board",
                "hostFace":"front", "facingFace":"back", "at":{"x":1.0,"y":2.0}, "rotation":17.0,
                "gap":0.3, "attachment":"board", "connection":{
                    "hostConnectorPartId":connector, "modulePortId":"port", "busId":"bus",
                    "assignments":{}, "cableType":"A", "railVoltages":{}
                }
            })).unwrap());
        }
    }

    #[test]
    fn membership_transition_expands_with_planned_pair_identities_and_references() {
        let (before, _, mut context, _, _, _) =
            membership_compaction_fixture().expect("the linked matrix fixture is valid");
        for (source_id, explicit_state) in [
            ("matrix/source-matrix/r1c1", true),
            ("matrix/source-matrix/r1c1/diode", false),
        ] {
            let association = context
                .associations
                .iter_mut()
                .find(|association| association.source_id == source_id)
                .unwrap();
            association.state = TransitionState::Local;
            association.explicit_state = explicit_state;
        }
        let mut post_source = before.clone();
        let source_matrix = before
            .matrices
            .iter()
            .find(|matrix| matrix.id == "source-matrix")
            .unwrap();
        let mut expanded = source_matrix.clone();
        expanded.columns = 5;
        for row in 0..2 {
            expanded.cells.push(MatrixCell {
                deleted: false,
                row,
                column: 4,
                enabled: true,
                definition_id: None,
                variant: None,
                offset: None,
                rotation: None,
                assemblies: vec![MatrixAssembly {
                    id: "diode".into(),
                    definition_id: "diode".into(),
                    offset: Vec2 { x: 2.0, y: 3.0 },
                    rotation: Some(30.0),
                    side: Some(Side::Back),
                }],
                assemblies_local: None,
            });
        }
        crate::matrix::set_matrix(&mut post_source, &expanded)
            .expect("the planned source additions materialize");

        let source_identity_map: BTreeMap<_, _> = source_matrix
            .part_ids
            .iter()
            .map(|id| (id.clone(), Some(id.clone())))
            .collect();
        let mirror_identity_map: BTreeMap<_, _> = before
            .matrices
            .iter()
            .find(|matrix| matrix.id == "mirror-matrix")
            .unwrap()
            .part_ids
            .iter()
            .map(|id| (id.clone(), Some(id.clone())))
            .collect();
        let mut additions = Vec::new();
        for row in 0..2 {
            let source_key = format!("matrix/source-matrix/r{row}c4");
            let mirror_key = format!("matrix/mirror-matrix/r{row}c4");
            for (source_id, mirror_id, parent_source_id, parent_mirror_id, mirror_reference) in [
                (
                    source_key.clone(),
                    mirror_key.clone(),
                    None,
                    None,
                    format!("EXP_M_R{row}C4"),
                ),
                (
                    format!("{source_key}/diode"),
                    format!("{mirror_key}/diode"),
                    Some(source_key.clone()),
                    Some(mirror_key.clone()),
                    format!("EXP_M_D_R{row}C4"),
                ),
            ] {
                let source_part = post_source
                    .parts
                    .iter()
                    .find(|part| part.id == source_id)
                    .unwrap();
                additions.push(PlannedAddition {
                    source_id,
                    mirror_id,
                    source_reference: source_part.reference.clone(),
                    mirror_reference,
                    parent_source_id,
                    parent_mirror_id,
                });
            }
        }
        let mut final_owners: BTreeMap<_, _> = post_source
            .parts
            .iter()
            .map(|part| {
                let board = post_source
                    .boards
                    .iter()
                    .find(|board| board.part_ids.contains(&part.id))
                    .unwrap();
                (part.id.clone(), FinalOwner::Board(board.id.clone()))
            })
            .collect();
        for addition in &additions {
            final_owners.insert(
                addition.mirror_id.clone(),
                FinalOwner::Board("mirror-board".into()),
            );
        }

        let output = prepare_membership_transition_kernel(MembershipTransitionInput {
            valid_before: &before,
            post_source: &post_source,
            context: &context,
            baseline_kind: BaselineKind::MaterializedCurrentSchema,
            source_identity_map: &source_identity_map,
            mirror_identity_map: &mirror_identity_map,
            planned_additions: &additions,
            final_owners: &final_owners,
            explicit_removal_closure: &BTreeSet::new(),
        })
        .expect("a complete planned expansion is projected atomically");
        let mirror = output
            .draft
            .matrices
            .iter()
            .find(|matrix| matrix.id == "mirror-matrix")
            .unwrap();
        assert_eq!((mirror.rows, mirror.columns), (2, 5));
        assert_eq!(mirror.part_ids.len(), 20);
        for addition in &additions {
            assert_eq!(
                output
                    .draft
                    .parts
                    .iter()
                    .find(|part| part.id == addition.mirror_id)
                    .unwrap()
                    .reference,
                addition.mirror_reference
            );
        }
        assert_eq!(output.context_delta.added.len(), 4);
        assert_eq!(output.context.associations.len(), 21);
        assert_eq!(output.context.pair_right, context.pair_right);
    }

    #[test]
    fn membership_transition_chains_two_by_three_expansion_and_two_by_two_trim() {
        let assert_pose = |document: &ProjectDoc, id: &str, expected: Pose2| {
            let actual = document
                .parts
                .iter()
                .find(|part| part.id == id)
                .unwrap()
                .pose;
            assert!(
                (actual.at.x - expected.at.x).abs() < 1e-9,
                "{id}: {actual:?}"
            );
            assert!(
                (actual.at.y - expected.at.y).abs() < 1e-9,
                "{id}: {actual:?}"
            );
            assert!(
                (actual.rotation - expected.rotation).abs() < 1e-9,
                "{id}: {actual:?}"
            );
        };
        let (mut before, _, mut context, _, _, _) =
            membership_compaction_fixture().expect("the linked matrix fixture is valid");
        crate::matrix::deletion::remove_axis(&mut before, "source-matrix", 3, false)
            .expect("the source fixture narrows to three columns");
        crate::matrix::deletion::remove_axis(&mut before, "mirror-matrix", 3, false)
            .expect("the mirror fixture narrows to three columns");
        let mut initial_source = before
            .matrices
            .iter()
            .find(|matrix| matrix.id == "source-matrix")
            .unwrap()
            .clone();
        initial_source.origin = Vec2 { x: 3.0, y: -2.0 };
        initial_source.edge_gap = Some(Vec2 { x: 4.25, y: 6.5 });
        crate::matrix::set_matrix(&mut before, &initial_source)
            .expect("the initial source frame materializes at the approved 2x3 origin");
        let matrix_association = context
            .pair_right
            .correspondences
            .iter()
            .find(|association| association.kind == MirroredItemKind::Matrix)
            .unwrap();
        let projected_initial = project_matrix(
            before
                .matrices
                .iter()
                .find(|matrix| matrix.id == "source-matrix")
                .unwrap(),
            before
                .matrices
                .iter()
                .find(|matrix| matrix.id == "mirror-matrix")
                .unwrap(),
            matrix_association,
            context.mapping,
        )
        .expect("the initial mirror frame is derived from the source frame");
        let mut initial_mirror = projected_initial;
        assert_eq!(
            initial_mirror.edge_gap,
            Some(Vec2 { x: 4.25, y: 6.5 }),
            "matrix projection preserves a nondefault edge gap"
        );
        for (row, column, x, y) in [(0, 1, -93.0, 62.0), (1, 1, -113.0, 42.0)] {
            let cell = initial_mirror
                .cells
                .iter_mut()
                .find(|cell| cell.row == row && cell.column == column)
                .unwrap();
            cell.offset = Some(Vec2 { x, y });
            cell.rotation = Some(-90.0);
            let diode = cell
                .assemblies
                .iter_mut()
                .find(|assembly| assembly.id == "diode")
                .unwrap();
            diode.offset = Vec2 { x: 2.0, y: 3.0 };
            diode.rotation = Some(30.0);
            diode.side = Some(Side::Back);
        }
        crate::matrix::set_matrix(&mut before, &initial_mirror)
            .expect("both Local keys and diode children are materialized independently");
        let mirror_members_now = before
            .matrices
            .iter()
            .find(|matrix| matrix.id == "mirror-matrix")
            .unwrap()
            .part_ids
            .clone();
        before.nets.push(crate::model::Net {
            id: "net-clean".into(),
            name: "PROBE OMITTED LINKED".into(),
            pins: vec![
                crate::model::Pin {
                    part_id: "geometry-probe".into(),
                    pad_id: "pad-two".into(),
                },
                crate::model::Pin {
                    part_id: "matrix/mirror-matrix/r0c2/diode".into(),
                    pad_id: "pad-one".into(),
                },
                crate::model::Pin {
                    part_id: "matrix/mirror-matrix/r1c2/diode".into(),
                    pad_id: "pad-two".into(),
                },
            ],
        });
        for (id, part_id, pad_id) in [
            ("net-local-r0", "matrix/mirror-matrix/r0c1/diode", "pad-one"),
            ("net-local-r1", "matrix/mirror-matrix/r1c1/diode", "pad-two"),
        ] {
            before.nets.push(crate::model::Net {
                id: id.into(),
                name: format!("LOCAL DEPENDENCY {id}"),
                pins: vec![crate::model::Pin {
                    part_id: part_id.into(),
                    pad_id: pad_id.into(),
                }],
            });
        }
        before
            .boards
            .iter_mut()
            .find(|board| board.id == "mirror-board")
            .unwrap()
            .net_ids
            .extend([
                "net-clean".into(),
                "net-local-r0".into(),
                "net-local-r1".into(),
            ]);
        before.constraints.push(crate::model::Constraint::Offset {
            id: "constraint-omit".into(),
            source_part_id: "geometry-probe".into(),
            target_part_id: "matrix/mirror-matrix/r0c2/diode".into(),
            offset: Vec2 { x: 95.0, y: -57.0 },
            rotation: 60.0,
        });
        for (id, target_part_id, offset) in [
            (
                "constraint-local-r0",
                "matrix/mirror-matrix/r0c1/diode",
                Vec2 { x: 38.0, y: 51.0 },
            ),
            (
                "constraint-local-r1",
                "matrix/mirror-matrix/r1c1/diode",
                Vec2 { x: 38.0, y: 71.0 },
            ),
        ] {
            before.constraints.push(crate::model::Constraint::Offset {
                id: id.into(),
                source_part_id: "geometry-probe".into(),
                target_part_id: target_part_id.into(),
                offset,
                rotation: 30.0,
            });
        }
        let mut outline_members: Vec<_> = mirror_members_now.into_iter().collect();
        outline_members.push("geometry-probe".into());
        before
            .outline
            .push(crate::model::OutlineFeature::PartEnvelope {
                id: "outline-mirror".into(),
                connections: vec![
                    OutlineConnection {
                        id: "local-r0-connection".into(),
                        width: 1.25,
                        points: vec![OutlineControlPoint {
                            at: Vec2 { x: 2.0, y: 3.0 },
                            part_id: Some("matrix/mirror-matrix/r0c1/diode".into()),
                        }],
                    },
                    OutlineConnection {
                        id: "local-r1-connection".into(),
                        width: 1.5,
                        points: vec![OutlineControlPoint {
                            at: Vec2 { x: 2.0, y: 3.0 },
                            part_id: Some("matrix/mirror-matrix/r1c1/diode".into()),
                        }],
                    },
                ],
                settings: crate::model::OutlineSettings {
                    corners: crate::model::CornerStyle::Fillet,
                    size: 3.5,
                    bridge_width: 7.25,
                    repair: Some(crate::model::OutlineRepairSettings {
                        enabled: true,
                        maximum_gap_span: 12.0,
                        minimum_connection_width: 1.25,
                        edge_clearance: 0.75,
                        keep_gaps: vec![crate::model::ProtectedOutlineGap {
                            id: "local-r0-keep-gap".into(),
                            points: vec![OutlineControlPoint {
                                at: Vec2 { x: 1.5, y: 2.5 },
                                part_id: Some("matrix/mirror-matrix/r0c1/diode".into()),
                            }],
                        }],
                    }),
                },
                part_ids: outline_members,
                margin: 1.75,
                operation: crate::model::Operation::Add,
            });
        before
            .boards
            .iter_mut()
            .find(|board| board.id == "mirror-board")
            .unwrap()
            .outline_ids
            .push("outline-mirror".into());
        before.outline.retain(|feature| {
            !matches!(feature, crate::model::OutlineFeature::PartEnvelope { id, .. } if id == "retained-keys-envelope")
        });
        before
            .boards
            .iter_mut()
            .find(|board| board.id == "mirror-board")
            .unwrap()
            .outline_ids
            .retain(|id| id != "retained-keys-envelope");
        if let Some(crate::model::Constraint::Offset { offset, .. }) = before.constraints.iter_mut()
            .find(|constraint| matches!(constraint, crate::model::Constraint::Offset { id, .. } if id == "retained-second-key-offset"))
        {
            *offset = Vec2 { x: 36.0, y: 68.0 };
        }
        let source_members = before
            .matrices
            .iter()
            .find(|matrix| matrix.id == "source-matrix")
            .unwrap()
            .part_ids
            .clone();
        let mirror_members = before
            .matrices
            .iter()
            .find(|matrix| matrix.id == "mirror-matrix")
            .unwrap()
            .part_ids
            .clone();
        let halves = before.hardware.as_mut().unwrap().pairs[0]
            .halves
            .as_mut()
            .unwrap();
        halves.correspondences.retain(|association| {
            association.kind == MirroredItemKind::Matrix
                || source_members.contains(&association.source_id)
        });
        context.associations.retain(|association| {
            association.kind == MirroredItemKind::Matrix
                || source_members.contains(&association.source_id)
                    && mirror_members.contains(&association.mirror_id)
        });
        context
            .owners
            .retain(|id, _| before.parts.iter().any(|part| part.id == *id));
        context.pair_right = halves.clone();
        for (source_id, state) in [
            ("matrix/source-matrix/r0c1", TransitionState::Local),
            ("matrix/source-matrix/r0c1/diode", TransitionState::Local),
            ("matrix/source-matrix/r1c1", TransitionState::Local),
            ("matrix/source-matrix/r1c1/diode", TransitionState::Local),
            ("matrix/source-matrix/r0c2", TransitionState::Omitted),
            ("matrix/source-matrix/r0c2/diode", TransitionState::Omitted),
        ] {
            let association = context
                .associations
                .iter_mut()
                .find(|association| association.source_id == source_id)
                .expect("the three-column context contains each state");
            association.state = state;
            association.explicit_state = !source_id.ends_with("/diode");
        }
        assert!(crate::mechanical::halves::validate_document(&before).is_ok());

        let local_ids = BTreeSet::from([
            "matrix/mirror-matrix/r0c1/diode".to_owned(),
            "matrix/mirror-matrix/r1c1/diode".to_owned(),
        ]);
        let mut local_keymap = KeymapConfiguration::default();
        local_keymap.layers[0].bindings.insert(
            "matrix/mirror-matrix/r0c1/diode".into(),
            KeyBinding::KeyPress {
                keycode: "LOCAL-R0".into(),
            },
        );
        local_keymap.layers[0].sensors.insert(
            "matrix/mirror-matrix/r1c1/diode".into(),
            EncoderBinding {
                clockwise: KeyBinding::KeyPress {
                    keycode: "LOCAL-R1-CW".into(),
                },
                counterclockwise: KeyBinding::KeyPress {
                    keycode: "LOCAL-R1-CCW".into(),
                },
            },
        );
        before.keymap = Some(local_keymap);
        let mut local_keycaps = KeycapConfiguration::default();
        local_keycaps.keys.insert(
            "matrix/mirror-matrix/r0c1/diode".into(),
            KeycapKeySettings {
                legend: Some("LOCAL R0".into()),
                ..KeycapKeySettings::default()
            },
        );
        local_keycaps.keys.insert(
            "matrix/mirror-matrix/r1c1/diode".into(),
            KeycapKeySettings {
                legend: Some("LOCAL R1".into()),
                ..KeycapKeySettings::default()
            },
        );
        before.keycaps = Some(local_keycaps);
        let local_part_payloads: BTreeMap<_, _> = before
            .parts
            .iter()
            .filter(|part| local_ids.contains(&part.id))
            .map(|part| (part.id.clone(), part.clone()))
            .collect();
        let local_net_pins: BTreeMap<_, _> = before
            .nets
            .iter()
            .filter(|net| net.id == "net-local-r0" || net.id == "net-local-r1")
            .map(|net| (net.id.clone(), net.pins.clone()))
            .collect();
        let local_constraints: Vec<_> = before
            .constraints
            .iter()
            .filter(|constraint| {
                local_ids.contains(constraint.source()) || local_ids.contains(constraint.target())
            })
            .cloned()
            .collect();
        let (local_connections, local_outline_settings) = match before
            .outline
            .iter()
            .find(|feature| matches!(feature, OutlineFeature::PartEnvelope { id, .. } if id == "outline-mirror"))
            .unwrap()
        {
            OutlineFeature::PartEnvelope { connections, settings, .. } => {
                (connections.clone(), settings.clone())
            }
            _ => unreachable!(),
        };
        let local_bindings: BTreeMap<_, _> = before.keymap.as_ref().unwrap().layers[0]
            .bindings
            .iter()
            .filter(|(id, _)| local_ids.contains(*id))
            .map(|(id, value)| (id.clone(), value.clone()))
            .collect();
        let local_sensors: BTreeMap<_, _> = before.keymap.as_ref().unwrap().layers[0]
            .sensors
            .iter()
            .filter(|(id, _)| local_ids.contains(*id))
            .map(|(id, value)| (id.clone(), value.clone()))
            .collect();
        let local_keycap_settings: BTreeMap<_, _> = before
            .keycaps
            .as_ref()
            .unwrap()
            .keys
            .iter()
            .filter(|(id, _)| local_ids.contains(*id))
            .map(|(id, value)| (id.clone(), value.clone()))
            .collect();
        let assert_local_retention = |document: &ProjectDoc, phase: &str| {
            for (id, expected) in &local_part_payloads {
                let actual: Vec<_> = document
                    .parts
                    .iter()
                    .filter(|part| part.id == *id)
                    .collect();
                assert_eq!(
                    actual,
                    vec![expected],
                    "{phase}: full Local Part payload {id}"
                );
                assert_eq!(
                    document
                        .boards
                        .iter()
                        .filter(|board| board.part_ids.contains(id))
                        .count(),
                    1,
                    "{phase}: Local board registration {id}"
                );
                assert_eq!(
                    document
                        .matrices
                        .iter()
                        .find(|matrix| matrix.id == "mirror-matrix")
                        .unwrap()
                        .part_ids
                        .iter()
                        .filter(|member| *member == id)
                        .count(),
                    1,
                    "{phase}: Local matrix registration {id}"
                );
            }
            let actual_net_pins: BTreeMap<_, _> = document
                .nets
                .iter()
                .filter(|net| net.id == "net-local-r0" || net.id == "net-local-r1")
                .map(|net| (net.id.clone(), net.pins.clone()))
                .collect();
            assert_eq!(
                actual_net_pins, local_net_pins,
                "{phase}: Local dependency nets"
            );
            let actual_constraints: Vec<_> = document
                .constraints
                .iter()
                .filter(|constraint| {
                    local_ids.contains(constraint.source())
                        || local_ids.contains(constraint.target())
                })
                .cloned()
                .collect();
            assert_eq!(
                actual_constraints, local_constraints,
                "{phase}: Local constraints"
            );
            let (actual_connections, actual_settings) = match document
                .outline
                .iter()
                .find(|feature| matches!(feature, OutlineFeature::PartEnvelope { id, .. } if id == "outline-mirror"))
                .unwrap()
            {
                OutlineFeature::PartEnvelope { connections, settings, .. } => (connections, settings),
                _ => unreachable!(),
            };
            assert_eq!(
                actual_connections, &local_connections,
                "{phase}: Local outline controls"
            );
            assert_eq!(
                actual_settings, &local_outline_settings,
                "{phase}: nondefault outline settings and Local keep-gap reference"
            );
            let actual_bindings: BTreeMap<_, _> = document.keymap.as_ref().unwrap().layers[0]
                .bindings
                .iter()
                .filter(|(id, _)| local_ids.contains(*id))
                .map(|(id, value)| (id.clone(), value.clone()))
                .collect();
            assert_eq!(
                actual_bindings, local_bindings,
                "{phase}: Local key bindings"
            );
            let actual_sensors: BTreeMap<_, _> = document.keymap.as_ref().unwrap().layers[0]
                .sensors
                .iter()
                .filter(|(id, _)| local_ids.contains(*id))
                .map(|(id, value)| (id.clone(), value.clone()))
                .collect();
            assert_eq!(
                actual_sensors, local_sensors,
                "{phase}: Local sensor bindings"
            );
            let actual_keycaps: BTreeMap<_, _> = document
                .keycaps
                .as_ref()
                .unwrap()
                .keys
                .iter()
                .filter(|(id, _)| local_ids.contains(*id))
                .map(|(id, value)| (id.clone(), value.clone()))
                .collect();
            assert_eq!(
                actual_keycaps, local_keycap_settings,
                "{phase}: Local keycap settings"
            );
        };
        assert_local_retention(&before, "before");

        let source_before = before
            .matrices
            .iter()
            .find(|matrix| matrix.id == "source-matrix")
            .unwrap();
        let mirror_before = before
            .matrices
            .iter()
            .find(|matrix| matrix.id == "mirror-matrix")
            .unwrap();
        let mut expanded_source = source_before.clone();
        expanded_source.columns = 4;
        expanded_source.origin = Vec2 { x: 5.0, y: 1.0 };
        for row in 0..2 {
            expanded_source.cells.push(MatrixCell {
                deleted: false,
                row,
                column: 3,
                enabled: true,
                definition_id: None,
                variant: None,
                offset: None,
                rotation: None,
                assemblies: vec![MatrixAssembly {
                    id: "diode".into(),
                    definition_id: "diode".into(),
                    offset: Vec2 { x: 2.0, y: 3.0 },
                    rotation: Some(30.0),
                    side: Some(Side::Back),
                }],
                assemblies_local: None,
            });
        }
        let mut expanded_source_document = before.clone();
        crate::matrix::set_matrix(&mut expanded_source_document, &expanded_source)
            .expect("the post-source expansion is materialized");
        let source_identity_map: BTreeMap<_, _> = source_before
            .part_ids
            .iter()
            .map(|id| (id.clone(), Some(id.clone())))
            .collect();
        let mirror_identity_map: BTreeMap<_, _> = mirror_before
            .part_ids
            .iter()
            .map(|id| {
                let omitted =
                    id == "matrix/mirror-matrix/r0c2" || id == "matrix/mirror-matrix/r0c2/diode";
                (id.clone(), (!omitted).then(|| id.clone()))
            })
            .collect();
        let mut additions = Vec::new();
        for row in 0..2 {
            let source_key = format!("matrix/source-matrix/r{row}c3");
            let mirror_key = format!("matrix/mirror-matrix/r{row}c3");
            for (source_id, mirror_id, parent_source_id, parent_mirror_id, mirror_reference) in [
                (
                    source_key.clone(),
                    mirror_key.clone(),
                    None,
                    None,
                    format!("CHAIN_M_R{row}C3"),
                ),
                (
                    format!("{source_key}/diode"),
                    format!("{mirror_key}/diode"),
                    Some(source_key.clone()),
                    Some(mirror_key.clone()),
                    format!("CHAIN_M_D_R{row}C3"),
                ),
            ] {
                let source_part = expanded_source_document
                    .parts
                    .iter()
                    .find(|part| part.id == source_id)
                    .unwrap();
                additions.push(PlannedAddition {
                    source_id,
                    mirror_id,
                    source_reference: source_part.reference.clone(),
                    mirror_reference,
                    parent_source_id,
                    parent_mirror_id,
                });
            }
        }
        let omitted_key_id = "matrix/mirror-matrix/r0c2";
        let omitted_mirror_id = "matrix/mirror-matrix/r0c2/diode";
        let mut expanded_owners: BTreeMap<_, _> = expanded_source_document
            .parts
            .iter()
            .map(|part| {
                let owner = expanded_source_document
                    .boards
                    .iter()
                    .find(|board| board.part_ids.contains(&part.id))
                    .unwrap();
                (part.id.clone(), FinalOwner::Board(owner.id.clone()))
            })
            .collect();
        for addition in &additions {
            expanded_owners.insert(
                addition.mirror_id.clone(),
                FinalOwner::Board("mirror-board".into()),
            );
        }
        expanded_owners.insert(omitted_key_id.into(), FinalOwner::Absent);
        expanded_owners.insert(omitted_mirror_id.into(), FinalOwner::Absent);
        let expanded = prepare_membership_transition_kernel(MembershipTransitionInput {
            valid_before: &before,
            post_source: &expanded_source_document,
            context: &context,
            baseline_kind: BaselineKind::MaterializedCurrentSchema,
            source_identity_map: &source_identity_map,
            mirror_identity_map: &mirror_identity_map,
            planned_additions: &additions,
            final_owners: &expanded_owners,
            explicit_removal_closure: &BTreeSet::from([omitted_key_id.into(), omitted_mirror_id.into()]),
        })
        .expect("the approved expansion removes the initial Omitted companion and adds four planned identities");
        assert_local_retention(&expanded.draft, "expanded");
        let expanded_mirror = expanded
            .draft
            .matrices
            .iter()
            .find(|matrix| matrix.id == "mirror-matrix")
            .unwrap();
        assert_eq!((expanded_mirror.rows, expanded_mirror.columns), (2, 4));
        assert_eq!(
            expanded
                .draft
                .matrices
                .iter()
                .find(|matrix| matrix.id == "source-matrix")
                .unwrap()
                .part_ids
                .len(),
            16
        );
        assert_eq!(expanded_mirror.part_ids.len(), 14);
        for id in [omitted_key_id, omitted_mirror_id] {
            assert!(!expanded.draft.parts.iter().any(|part| part.id == id));
        }
        assert_eq!(expanded.context_delta.added.len(), 4);
        let expected_expansion_ids: BTreeSet<_> = [
            "constraint-omit",
            "matrix/mirror-matrix/r0c0",
            "matrix/mirror-matrix/r0c0/diode",
            "matrix/mirror-matrix/r0c2",
            "matrix/mirror-matrix/r0c2/diode",
            "matrix/mirror-matrix/r0c3",
            "matrix/mirror-matrix/r0c3/diode",
            "matrix/mirror-matrix/r1c0",
            "matrix/mirror-matrix/r1c0/diode",
            "matrix/mirror-matrix/r1c2",
            "matrix/mirror-matrix/r1c2/diode",
            "matrix/mirror-matrix/r1c3",
            "matrix/mirror-matrix/r1c3/diode",
            "mirror-board",
            "mirror-matrix",
            "net-clean",
            "outline-mirror",
        ]
        .into_iter()
        .map(str::to_owned)
        .collect();
        assert_eq!(
            expanded
                .changed_document_ids
                .iter()
                .cloned()
                .collect::<BTreeSet<_>>(),
            expected_expansion_ids
        );
        assert_eq!(expanded.changed_document_ids.len(), 17);
        assert_eq!(expanded.context.associations.len(), 17);
        for id in [
            "matrix/source-matrix/r0c1/diode",
            "matrix/source-matrix/r1c1/diode",
            "matrix/source-matrix/r0c2/diode",
        ] {
            let association = expanded
                .context
                .associations
                .iter()
                .find(|association| association.source_id == id)
                .unwrap();
            assert!(!association.explicit_state, "{id} inherits from its key");
            assert_eq!(
                association.parent_source_id.as_deref(),
                Some(id.trim_end_matches("/diode"))
            );
        }
        assert_eq!(
            expanded
                .context
                .associations
                .iter()
                .filter(|association| association.state == TransitionState::Linked)
                .count(),
            11
        );
        assert_eq!(
            expanded
                .context
                .associations
                .iter()
                .filter(|association| association.state == TransitionState::Local)
                .count(),
            4
        );
        assert_eq!(
            expanded
                .context
                .associations
                .iter()
                .filter(|association| association.state == TransitionState::Omitted)
                .count(),
            2
        );
        assert_eq!(
            expanded
                .final_owners
                .keys()
                .filter(|id| id.starts_with("matrix/source-matrix/")
                    || id.starts_with("matrix/mirror-matrix/"))
                .count(),
            32
        );
        let cleaned_net = expanded
            .draft
            .nets
            .iter()
            .find(|net| net.id == "net-clean")
            .unwrap();
        assert_eq!(
            cleaned_net.pins,
            vec![
                crate::model::Pin {
                    part_id: "geometry-probe".into(),
                    pad_id: "pad-two".into()
                },
                crate::model::Pin {
                    part_id: "matrix/mirror-matrix/r1c2/diode".into(),
                    pad_id: "pad-two".into()
                },
            ]
        );
        assert!(expanded.context.associations.iter().any(|association| {
            association.state == TransitionState::Omitted
                && association.mirror_id == omitted_mirror_id
        }));
        assert_pose(
            &expanded.draft,
            "matrix/mirror-matrix/r0c0",
            Pose2 {
                at: Vec2 { x: 99.0, y: -35.0 },
                rotation: 90.0,
            },
        );
        assert_pose(
            &expanded.draft,
            "matrix/mirror-matrix/r0c1",
            Pose2 {
                at: Vec2 { x: 40.0, y: 50.0 },
                rotation: 0.0,
            },
        );
        assert_pose(
            &expanded.draft,
            "matrix/mirror-matrix/r1c1/diode",
            Pose2 {
                at: Vec2 { x: 42.0, y: 73.0 },
                rotation: 30.0,
            },
        );
        assert_pose(
            &expanded.draft,
            "matrix/mirror-matrix/r1c2/diode",
            Pose2 {
                at: Vec2 { x: 76.0, y: -57.0 },
                rotation: 60.0,
            },
        );
        assert!(!expanded.draft.constraints.iter().any(|constraint| {
            matches!(constraint, crate::model::Constraint::Offset { id, .. } if id == "constraint-omit")
        }));
        let repeat_source_map: BTreeMap<_, _> = expanded
            .draft
            .matrices
            .iter()
            .find(|matrix| matrix.id == "source-matrix")
            .unwrap()
            .part_ids
            .iter()
            .map(|id| (id.clone(), Some(id.clone())))
            .collect();
        let repeat_mirror_map: BTreeMap<_, _> = expanded
            .draft
            .matrices
            .iter()
            .find(|matrix| matrix.id == "mirror-matrix")
            .unwrap()
            .part_ids
            .iter()
            .map(|id| (id.clone(), Some(id.clone())))
            .collect();
        let repeated = prepare_membership_transition_kernel(MembershipTransitionInput {
            valid_before: &expanded.draft,
            post_source: &expanded.draft,
            context: &expanded.context,
            baseline_kind: BaselineKind::ChainedPrivateDraft,
            source_identity_map: &repeat_source_map,
            mirror_identity_map: &repeat_mirror_map,
            planned_additions: &[],
            final_owners: &expanded.final_owners,
            explicit_removal_closure: &BTreeSet::new(),
        })
        .expect("the exact private expansion state repeats without changes");
        assert_local_retention(&repeated.draft, "repeated expansion");
        assert_eq!(repeated.draft, expanded.draft);
        assert_eq!(repeated.context, expanded.context);
        assert!(repeated.changed_document_ids.is_empty());
        assert!(repeated.changed_field_paths.is_empty());

        let mut shrink_before = expanded.draft.clone();
        shrink_before
            .constraints
            .push(crate::model::Constraint::Offset {
                id: "constraint-linked".into(),
                source_part_id: "geometry-probe".into(),
                target_part_id: "matrix/mirror-matrix/r1c2/diode".into(),
                offset: Vec2 { x: 72.0, y: -59.0 },
                rotation: 60.0,
            });
        let probe = shrink_before
            .parts
            .iter()
            .find(|part| part.id == "geometry-probe")
            .unwrap();
        let linked_endpoint = shrink_before
            .parts
            .iter()
            .find(|part| part.id == "matrix/mirror-matrix/r1c2/diode")
            .unwrap();
        assert_eq!(
            probe.pose,
            Pose2 {
                at: Vec2 { x: 4.0, y: 2.0 },
                rotation: 0.0
            }
        );
        assert_pose(
            &shrink_before,
            "matrix/mirror-matrix/r1c2/diode",
            Pose2 {
                at: Vec2 { x: 76.0, y: -57.0 },
                rotation: 60.0,
            },
        );
        assert!((linked_endpoint.pose.at.x - probe.pose.at.x - 72.0).abs() < 1e-9);
        assert!((linked_endpoint.pose.at.y - probe.pose.at.y + 59.0).abs() < 1e-9);

        let mut trimmed_source = shrink_before.clone();
        let source_matrix = shrink_before
            .matrices
            .iter()
            .find(|matrix| matrix.id == "source-matrix")
            .unwrap();
        let mut trimmed = source_matrix.clone();
        trimmed.columns = 2;
        trimmed.cells.retain(|cell| cell.column < 2);
        crate::matrix::set_matrix(&mut trimmed_source, &trimmed)
            .expect("the post-source trim is materialized");
        let source_identity_map: BTreeMap<_, _> = source_matrix
            .part_ids
            .iter()
            .map(|id| {
                let (_, column, _) = super::matrix_identity_slot(id, "source-matrix").unwrap();
                (id.clone(), (column < 2).then(|| id.clone()))
            })
            .collect();
        let mirror_matrix = shrink_before
            .matrices
            .iter()
            .find(|matrix| matrix.id == "mirror-matrix")
            .unwrap();
        let mirror_identity_map: BTreeMap<_, _> = mirror_matrix
            .part_ids
            .iter()
            .map(|id| {
                let (_, column, _) = super::matrix_identity_slot(id, "mirror-matrix").unwrap();
                (id.clone(), (column < 2).then(|| id.clone()))
            })
            .collect();
        let mut removal_closure = BTreeSet::new();
        for (id, mapped) in &source_identity_map {
            if mapped.is_none() {
                removal_closure.insert(id.clone());
                removal_closure
                    .insert(id.replace("matrix/source-matrix/", "matrix/mirror-matrix/"));
            }
        }
        removal_closure.insert(omitted_mirror_id.into());
        let mut final_owners = BTreeMap::new();
        for part in &shrink_before.parts {
            let removed = source_identity_map.get(&part.id) == Some(&None)
                || mirror_identity_map.get(&part.id) == Some(&None)
                || part.id == omitted_mirror_id;
            let owner = if removed {
                FinalOwner::Absent
            } else {
                let board = trimmed_source
                    .boards
                    .iter()
                    .find(|board| board.part_ids.contains(&part.id));
                board
                    .map(|board| FinalOwner::Board(board.id.clone()))
                    .unwrap_or(FinalOwner::Absent)
            };
            final_owners.insert(part.id.clone(), owner);
        }
        for id in expanded.context.owners.keys() {
            final_owners.entry(id.clone()).or_insert(FinalOwner::Absent);
        }
        let trimmed = prepare_membership_transition_kernel(MembershipTransitionInput {
            valid_before: &shrink_before,
            post_source: &trimmed_source,
            context: &expanded.context,
            baseline_kind: BaselineKind::ChainedPrivateDraft,
            source_identity_map: &source_identity_map,
            mirror_identity_map: &mirror_identity_map,
            planned_additions: &[],
            final_owners: &final_owners,
            explicit_removal_closure: &removal_closure,
        })
        .expect("the private expansion draft and full context chain into the two-column trim");
        assert_local_retention(&trimmed.draft, "trimmed");
        let final_mirror = trimmed
            .draft
            .matrices
            .iter()
            .find(|matrix| matrix.id == "mirror-matrix")
            .unwrap();
        assert_eq!((final_mirror.rows, final_mirror.columns), (2, 2));
        assert_eq!(final_mirror.part_ids.len(), 8);
        assert_eq!(
            trimmed
                .draft
                .parts
                .iter()
                .filter(|part| part.id.starts_with("matrix/source-matrix/"))
                .count(),
            8
        );
        assert_eq!(
            trimmed
                .draft
                .parts
                .iter()
                .filter(|part| part.id.starts_with("matrix/mirror-matrix/"))
                .count(),
            8
        );
        assert_eq!(trimmed.context_delta.retired.len(), 8);
        let expected_shrink_ids: BTreeSet<_> = [
            "constraint-linked",
            "matrix/mirror-matrix/r0c3",
            "matrix/mirror-matrix/r0c3/diode",
            "matrix/mirror-matrix/r1c2",
            "matrix/mirror-matrix/r1c2/diode",
            "matrix/mirror-matrix/r1c3",
            "matrix/mirror-matrix/r1c3/diode",
            "mirror-board",
            "mirror-matrix",
            "net-clean",
            "outline-mirror",
        ]
        .into_iter()
        .map(str::to_owned)
        .collect();
        assert_eq!(
            trimmed
                .changed_document_ids
                .iter()
                .cloned()
                .collect::<BTreeSet<_>>(),
            expected_shrink_ids
        );
        assert_eq!(trimmed.changed_document_ids.len(), 11);
        assert_eq!(
            trimmed
                .final_owners
                .keys()
                .filter(|id| id.starts_with("matrix/source-matrix/")
                    || id.starts_with("matrix/mirror-matrix/"))
                .count(),
            32
        );
        assert_eq!(trimmed.context.associations.len(), 9);
        assert_eq!(
            trimmed
                .context
                .associations
                .iter()
                .filter(|association| association.state == TransitionState::Linked)
                .count(),
            5
        );
        assert_eq!(
            trimmed
                .context
                .associations
                .iter()
                .filter(|association| association.state == TransitionState::Local)
                .count(),
            4
        );
        assert_eq!(
            trimmed
                .draft
                .nets
                .iter()
                .find(|net| net.id == "net-clean")
                .unwrap()
                .pins,
            vec![crate::model::Pin {
                part_id: "geometry-probe".into(),
                pad_id: "pad-two".into()
            },]
        );
        for id in [
            "matrix/mirror-matrix/r0c1/diode",
            "matrix/mirror-matrix/r1c1/diode",
        ] {
            assert!(trimmed.draft.parts.iter().any(|part| part.id == id));
            assert!(trimmed.context.associations.iter().any(|association| {
                association.mirror_id == id && association.state == TransitionState::Local
            }));
        }
        for id in [
            "matrix/source-matrix/r0c1/diode",
            "matrix/source-matrix/r1c1/diode",
        ] {
            let association = trimmed
                .context
                .associations
                .iter()
                .find(|association| association.source_id == id)
                .unwrap();
            assert_eq!(association.state, TransitionState::Local);
            assert!(!association.explicit_state, "{id} remains inherited");
        }
        let retired_omitted_child = trimmed
            .context_delta
            .retired
            .iter()
            .find(|association| association.source_id == "matrix/source-matrix/r0c2/diode")
            .unwrap();
        assert_eq!(retired_omitted_child.state, TransitionState::Omitted);
        assert!(!retired_omitted_child.explicit_state);
        assert_eq!(trimmed.context.pair_right, context.pair_right);
        let final_source = trimmed
            .draft
            .matrices
            .iter()
            .find(|matrix| matrix.id == "source-matrix")
            .unwrap();
        let repeat_source_map: BTreeMap<_, _> = final_source
            .part_ids
            .iter()
            .map(|id| (id.clone(), Some(id.clone())))
            .collect();
        let repeat_mirror_map: BTreeMap<_, _> = final_mirror
            .part_ids
            .iter()
            .map(|id| (id.clone(), Some(id.clone())))
            .collect();
        let repeated = prepare_membership_transition_kernel(MembershipTransitionInput {
            valid_before: &trimmed.draft,
            post_source: &trimmed.draft,
            context: &trimmed.context,
            baseline_kind: BaselineKind::ChainedPrivateDraft,
            source_identity_map: &repeat_source_map,
            mirror_identity_map: &repeat_mirror_map,
            planned_additions: &[],
            final_owners: &trimmed.final_owners,
            explicit_removal_closure: &BTreeSet::new(),
        })
        .expect("the exact post-shrink draft and context repeat without changes");
        assert_local_retention(&repeated.draft, "repeated trim");
        assert_eq!(repeated.draft, trimmed.draft);
        assert_eq!(repeated.context, trimmed.context);
        assert!(repeated.changed_document_ids.is_empty());
        assert!(repeated.changed_field_paths.is_empty());
    }

    #[test]
    fn membership_transition_trims_only_explicitly_removed_linked_closures() {
        let (mut before, _, mut context, _, _, _) =
            membership_compaction_fixture().expect("the linked matrix fixture is valid");
        for (source_id, state) in [
            ("matrix/source-matrix/r0c0/diode", TransitionState::Local),
            ("matrix/source-matrix/r1c1", TransitionState::Local),
            ("matrix/source-matrix/r0c3", TransitionState::Local),
            ("matrix/source-matrix/r0c2", TransitionState::Omitted),
        ] {
            let mirror_id = source_id.replace("source-matrix", "mirror-matrix");
            for association in &mut context.associations {
                if (association.source_id == source_id && association.mirror_id == mirror_id)
                    || (association.parent_source_id.as_deref() == Some(source_id)
                        && association.parent_mirror_id.as_deref() == Some(mirror_id.as_str()))
                {
                    if association.explicit_state && association.source_id != source_id {
                        continue;
                    }
                    association.state = state;
                    association.explicit_state = association.source_id == source_id;
                    context.owners.insert(
                        association.mirror_id.clone(),
                        match state {
                            TransitionState::Local => {
                                FinalOwner::Board(context.mirror_board_id.clone())
                            }
                            TransitionState::Omitted => FinalOwner::Absent,
                            TransitionState::Linked => {
                                FinalOwner::Board(context.mirror_board_id.clone())
                            }
                        },
                    );
                }
            }
            context.pair_right.exceptions.push(MirroredException {
                kind: MirroredItemKind::Part,
                mirror_id,
                state: match state {
                    TransitionState::Local => MirroredExceptionState::Local,
                    TransitionState::Omitted => MirroredExceptionState::Omitted,
                    TransitionState::Linked => unreachable!(),
                },
                companion_mode: None,
                companion_tweak: None,
                removed_copper: None,
            });
        }
        before.hardware.as_mut().unwrap().pairs[0]
            .halves
            .as_mut()
            .unwrap()
            .clone_from(&context.pair_right);
        let mut sentinel_matrix = before
            .matrices
            .iter()
            .find(|matrix| matrix.id == "mirror-matrix")
            .unwrap()
            .clone();
        for (row, column, x, y, rotation) in [(0, 0, 7.25, -9.5, 121.0), (1, 1, -3.5, 11.25, -37.0)]
        {
            let cell = sentinel_matrix
                .cells
                .iter_mut()
                .find(|cell| cell.row == row && cell.column == column)
                .unwrap();
            let assembly = cell
                .assemblies
                .iter_mut()
                .find(|item| item.id == "diode")
                .unwrap();
            assembly.offset = Vec2 { x, y };
            assembly.rotation = Some(rotation);
            assembly.side = Some(Side::Front);
        }
        crate::matrix::set_matrix(&mut before, &sentinel_matrix)
            .expect("Local child sentinels remain a valid matrix");
        let omitted_mirror_ids = [
            "matrix/mirror-matrix/r0c2",
            "matrix/mirror-matrix/r0c2/diode",
        ];
        before
            .parts
            .retain(|part| !omitted_mirror_ids.contains(&part.id.as_str()));
        before
            .boards
            .iter_mut()
            .find(|board| board.id == "mirror-board")
            .unwrap()
            .part_ids
            .retain(|part_id| !omitted_mirror_ids.contains(&part_id.as_str()));
        for omitted_mirror_id in omitted_mirror_ids {
            assert!(before.parts.iter().all(|part| part.id != omitted_mirror_id));
            assert!(before.boards.iter().all(|board| {
                !board
                    .part_ids
                    .iter()
                    .any(|part_id| part_id == omitted_mirror_id)
            }));
        }
        assert_eq!(
            context
                .pair_right
                .exceptions
                .iter()
                .filter(|exception| {
                    context.pair_right.overlay_source(exception)
                        == Some("matrix/source-matrix/r0c2")
                        && exception.mirror_id == "matrix/mirror-matrix/r0c2"
                        && exception.state == MirroredExceptionState::Omitted
                })
                .count(),
            1,
            "the owner record alone expresses the inherited Omitted closure"
        );
        assert!(context.pair_right.exceptions.iter().all(|exception| {
            context.pair_right.overlay_source(exception) != Some("matrix/source-matrix/r0c2/diode")
        }));
        assert!(
            context.pair_right.correspondences.iter().any(|record| {
                record.kind == MirroredItemKind::Part
                    && record.source_id == "matrix/source-matrix/r0c2/diode"
                    && record.mirror_id == "matrix/mirror-matrix/r0c2/diode"
                    && record.companion_mode == Some(MirroredCompanionMode::LiteralMirror)
            }),
            "the inherited child keeps its correspondence and per-item projection settings"
        );
        before.nets.push(Net {
            id: "retained-local-net".into(),
            name: "RETAINED LOCAL NET".into(),
            pins: vec![Pin {
                part_id: "matrix/mirror-matrix/r0c3/diode".into(),
                pad_id: "pad-two".into(),
            }],
        });
        before
            .boards
            .iter_mut()
            .find(|board| board.id == "mirror-board")
            .unwrap()
            .net_ids
            .push("retained-local-net".into());
        crate::mechanical::halves::validate_document(&before)
            .expect("the exact parent Omitted association owns its absent child closure");
        let source_before = before
            .matrices
            .iter()
            .find(|matrix| matrix.id == "source-matrix")
            .unwrap();
        let mirror_before = before
            .matrices
            .iter()
            .find(|matrix| matrix.id == "mirror-matrix")
            .unwrap();
        let mut post_source = before.clone();
        let mut trimmed = source_before.clone();
        trimmed.columns = 2;
        trimmed.cells.retain(|cell| cell.column < 2);
        crate::matrix::set_matrix(&mut post_source, &trimmed)
            .expect("the source dimension trim is materialized");

        let mut source_identity_map = BTreeMap::new();
        for id in &source_before.part_ids {
            let (_, column, _) = super::matrix_identity_slot(id, "source-matrix").unwrap();
            source_identity_map.insert(id.clone(), (column < 2).then(|| id.clone()));
        }
        let mirror_identity_map: BTreeMap<_, _> = mirror_before
            .part_ids
            .iter()
            .map(|id| {
                let (_, column, _) = super::matrix_identity_slot(id, "mirror-matrix").unwrap();
                let locally_retained = id.starts_with("matrix/mirror-matrix/r0c3");
                let omitted = context.associations.iter().any(|association| {
                    association.mirror_id == *id && association.state == TransitionState::Omitted
                });
                (
                    id.clone(),
                    ((column < 2 || locally_retained) && !omitted).then(|| id.clone()),
                )
            })
            .collect();
        let mut explicit_removal_closure = BTreeSet::new();
        for (source_id, target_id) in &source_identity_map {
            if target_id.is_none() {
                explicit_removal_closure.insert(source_id.clone());
                explicit_removal_closure
                    .insert(source_id.replace("matrix/source-matrix/", "matrix/mirror-matrix/"));
            }
        }
        let mut final_owners = context.owners.clone();
        for part in &before.parts {
            let locally_retained = context.associations.iter().any(|association| {
                association.state == TransitionState::Local
                    && association.mirror_id == part.id
                    && mirror_identity_map
                        .get(&part.id)
                        .is_some_and(Option::is_some)
            });
            let is_removed_matrix_part = (source_identity_map.get(&part.id) == Some(&None)
                || mirror_identity_map.get(&part.id) == Some(&None))
                && !locally_retained;
            let owner = if is_removed_matrix_part {
                FinalOwner::Absent
            } else {
                let board = post_source
                    .boards
                    .iter()
                    .find(|board| board.part_ids.contains(&part.id));
                board
                    .map(|board| FinalOwner::Board(board.id.clone()))
                    .unwrap_or(FinalOwner::Absent)
            };
            final_owners.insert(part.id.clone(), owner);
        }

        let output = prepare_membership_transition(MembershipTransitionInput {
            valid_before: &before,
            post_source: &post_source,
            context: &context,
            baseline_kind: BaselineKind::MaterializedCurrentSchema,
            source_identity_map: &source_identity_map,
            mirror_identity_map: &mirror_identity_map,
            planned_additions: &[],
            final_owners: &final_owners,
            explicit_removal_closure: &explicit_removal_closure,
        })
        .expect("the explicit trim removes the corresponding target closures");
        let mirror = output
            .draft
            .matrices
            .iter()
            .find(|matrix| matrix.id == "mirror-matrix")
            .unwrap();
        assert_eq!((mirror.rows, mirror.columns), (2, 2));
        assert_eq!(mirror.part_ids.len(), 8);
        assert_eq!(output.context_delta.retired.len(), 8);
        assert_eq!(output.context.associations.len(), 9);
        for removed_source_id in [
            "matrix/source-matrix/r0c2",
            "matrix/source-matrix/r0c2/diode",
            "matrix/source-matrix/r0c3",
            "matrix/source-matrix/r0c3/diode",
        ] {
            assert!(
                output
                    .context
                    .pair_right
                    .exceptions
                    .iter()
                    .all(|exception| {
                        exception.mirror_id
                            != removed_source_id.replace("source-matrix", "mirror-matrix")
                    }),
                "source removal cleans only the exception for removed item {removed_source_id}"
            );
            assert!(
                output
                    .context
                    .pair_right
                    .correspondences
                    .iter()
                    .all(|record| { record.source_id != removed_source_id })
            );
            assert!(
                output
                    .context
                    .associations
                    .iter()
                    .all(|association| { association.source_id != removed_source_id })
            );
        }
        for id in [
            "matrix/mirror-matrix/r0c3",
            "matrix/mirror-matrix/r0c3/diode",
        ] {
            assert!(
                output.draft.parts.iter().any(|part| part.id == id),
                "Local geometry {id} is retained"
            );
            assert!(
                output
                    .draft
                    .boards
                    .iter()
                    .find(|board| board.id == "mirror-board")
                    .unwrap()
                    .part_ids
                    .contains(&id.to_owned())
            );
            assert!(
                !mirror.part_ids.contains(&id.to_owned()),
                "out-of-bounds Local geometry is detached from matrix membership"
            );
            assert_eq!(
                output.final_owners.get(id),
                Some(&FinalOwner::Board("mirror-board".into()))
            );
        }
        assert_eq!(
            output
                .draft
                .nets
                .iter()
                .find(|net| net.id == "retained-local-net")
                .unwrap()
                .pins,
            vec![Pin {
                part_id: "matrix/mirror-matrix/r0c3/diode".into(),
                pad_id: "pad-two".into(),
            }]
        );
        for row in 0..2 {
            for column in 0..2 {
                let id = format!("matrix/mirror-matrix/r{row}c{column}");
                let before_part = before.parts.iter().find(|part| part.id == id).unwrap();
                let after_part = output
                    .draft
                    .parts
                    .iter()
                    .find(|part| part.id == id)
                    .unwrap();
                assert_eq!(after_part.properties, before_part.properties);
                assert_eq!(
                    after_part.generator_parameters,
                    before_part.generator_parameters
                );
            }
            for column in 2..4 {
                if row == 0 && column == 3 {
                    continue;
                }
                for id in [
                    format!("matrix/mirror-matrix/r{row}c{column}"),
                    format!("matrix/mirror-matrix/r{row}c{column}/diode"),
                ] {
                    assert!(!output.draft.parts.iter().any(|part| part.id == id));
                    assert_eq!(output.final_owners.get(&id), Some(&FinalOwner::Absent));
                }
            }
        }
        for (row, column, x, y, rotation) in [(0, 0, 7.25, -9.5, 121.0), (1, 1, -3.5, 11.25, -37.0)]
        {
            let cell = mirror
                .cells
                .iter()
                .find(|cell| cell.row == row && cell.column == column)
                .unwrap();
            let assembly = cell
                .assemblies
                .iter()
                .find(|item| item.id == "diode")
                .unwrap();
            assert_eq!(assembly.offset, Vec2 { x, y });
            assert_eq!(assembly.rotation, Some(rotation));
            assert_eq!(assembly.side, Some(Side::Front));
            let id = format!("matrix/mirror-matrix/r{row}c{column}/diode");
            let before_part = before.parts.iter().find(|part| part.id == id).unwrap();
            let after_part = output
                .draft
                .parts
                .iter()
                .find(|part| part.id == id)
                .unwrap();
            assert_eq!(
                after_part, before_part,
                "Local child payload {id} is preserved"
            );
        }
        let round_tripped: ProjectDoc =
            serde_json::from_slice(&serde_json::to_vec(&output.draft).unwrap()).unwrap();
        assert_eq!(round_tripped, output.draft);
        assert_eq!(
            round_tripped.hardware.as_ref().unwrap().pairs[0]
                .halves
                .as_ref()
                .unwrap(),
            &output.context.pair_right
        );
        let repeated_source_map: BTreeMap<_, _> = round_tripped
            .matrices
            .iter()
            .find(|matrix| matrix.id == "source-matrix")
            .unwrap()
            .part_ids
            .iter()
            .map(|id| (id.clone(), Some(id.clone())))
            .collect();
        let repeated_mirror_map: BTreeMap<_, _> = round_tripped
            .matrices
            .iter()
            .find(|matrix| matrix.id == "mirror-matrix")
            .unwrap()
            .part_ids
            .iter()
            .map(|id| (id.clone(), Some(id.clone())))
            .collect();
        let repeated = prepare_membership_transition(MembershipTransitionInput {
            valid_before: &round_tripped,
            post_source: &round_tripped,
            context: &output.context,
            baseline_kind: BaselineKind::MaterializedCurrentSchema,
            source_identity_map: &repeated_source_map,
            mirror_identity_map: &repeated_mirror_map,
            planned_additions: &[],
            final_owners: &output.final_owners,
            explicit_removal_closure: &BTreeSet::new(),
        })
        .expect("source retirement converges after a document round trip");
        assert_eq!(repeated.draft, round_tripped);
        assert_eq!(repeated.context, output.context);
        assert!(repeated.changed_document_ids.is_empty());
        assert!(repeated.changed_field_paths.is_empty());
    }

    #[test]
    fn membership_transition_compacts_ids_and_carries_logical_payload_and_settings() {
        let (mut before, _, context, identity_map, final_owners, explicit_closure) =
            membership_compaction_fixture().expect("the materialized compaction fixture is valid");
        add_membership_cleanup_sentinels(&mut before);
        let mut post_source = before.clone();
        crate::matrix::deletion::remove_axis(&mut post_source, "source-matrix", 1, false)
            .expect("the source operation remaps populated dependency sentinels");
        crate::mechanical::halves::validate_document(&before)
            .expect("populated before evidence remains canonical");
        let mirror_identity_map: BTreeMap<_, _> = identity_map
            .iter()
            .map(|(old, new)| {
                let mirror_old = old.replace("matrix/source-matrix/", "matrix/mirror-matrix/");
                let mirror_new = new
                    .as_ref()
                    .map(|id| id.replace("matrix/source-matrix/", "matrix/mirror-matrix/"));
                (mirror_old, mirror_new)
            })
            .collect();
        let additions = [];
        let output = prepare_membership_transition_kernel(MembershipTransitionInput {
            valid_before: &before,
            post_source: &post_source,
            context: &context,
            baseline_kind: BaselineKind::MaterializedCurrentSchema,
            source_identity_map: &identity_map,
            mirror_identity_map: &mirror_identity_map,
            planned_additions: &additions,
            final_owners: &final_owners,
            explicit_removal_closure: &explicit_closure,
        })
        .expect("the explicit axis-removal operation compacts the mirror");

        let removed = "matrix/mirror-matrix/r0c1/diode";
        let shifted = "matrix/mirror-matrix/r0c2/diode";
        assert_eq!(
            output.draft.keymap.as_ref().unwrap().layers[0]
                .bindings
                .get(removed),
            Some(&KeyBinding::KeyPress {
                keycode: "B".into()
            }),
            "the shifted identity carries its payload across the removed key"
        );
        assert_eq!(
            output.draft.keymap.as_ref().unwrap().layers[0]
                .sensors
                .get(removed)
                .unwrap()
                .clockwise,
            KeyBinding::KeyPress {
                keycode: "E".into()
            }
        );
        assert!(
            !output.draft.keymap.as_ref().unwrap().layers[0]
                .sensors
                .contains_key(shifted)
        );
        assert!(
            !output
                .draft
                .keycaps
                .as_ref()
                .unwrap()
                .keys
                .contains_key(shifted)
        );
        assert_eq!(
            output
                .draft
                .keycaps
                .as_ref()
                .unwrap()
                .keys
                .get(removed)
                .unwrap()
                .legend
                .as_deref(),
            Some("shifted sentinel")
        );
        assert_eq!(
            output.draft.hardware.as_ref().unwrap().instances[1].controller_part_id,
            None
        );
        assert_eq!(
            output.draft.hardware.as_ref().unwrap().boards[0]
                .locks
                .get(&format!("direct/{removed}"))
                .map(String::as_str),
            None
        );
        assert_eq!(
            output.draft.hardware.as_ref().unwrap().boards[0]
                .assignments
                .get(&format!("link/{removed}"))
                .map(String::as_str),
            Some("assignment sentinel")
        );
        assert!(
            !output.draft.hardware.as_ref().unwrap().boards[0]
                .key_bindings
                .contains_key(shifted)
        );
        assert_eq!(
            output.draft.hardware.as_ref().unwrap().boards[0]
                .key_bindings
                .get(removed)
                .map(String::as_str),
            Some("binding sentinel")
        );
        assert_eq!(output.draft.embedded_circuits[0].part_ids, vec![removed]);
        assert!(
            output
                .draft
                .modules
                .iter()
                .find(|module| module.id == "closure-module")
                .unwrap()
                .connection
                .is_none()
        );
        assert_eq!(
            output
                .draft
                .modules
                .iter()
                .find(|module| module.id == "shifted-module")
                .unwrap()
                .connection
                .as_ref()
                .unwrap()
                .host_connector_part_id,
            removed
        );
        let OutlineFeature::PartEnvelope {
            part_ids,
            connections,
            settings,
            ..
        } = output
            .draft
            .outline
            .iter()
            .find(|feature| feature.id() == "closure-envelope")
            .unwrap()
        else {
            panic!("the closure envelope remains a part envelope");
        };
        let current_members: BTreeSet<_> = output
            .draft
            .matrices
            .iter()
            .find(|matrix| matrix.id == "mirror-matrix")
            .unwrap()
            .part_ids
            .iter()
            .cloned()
            .collect();
        assert_eq!(
            part_ids.iter().cloned().collect::<BTreeSet<_>>(),
            current_members
        );
        assert_eq!(connections[0].points[0].part_id, None);
        assert_eq!(connections[0].points[1].part_id.as_deref(), Some(removed));
        assert_eq!(
            settings.repair.as_ref().unwrap().keep_gaps[0].points[0].part_id,
            None
        );
        let outline = output
            .draft
            .board_outlines
            .iter()
            .find(|outline| outline.board_id == "mirror-board")
            .unwrap();
        let version = outline
            .versions
            .iter()
            .find(|version| version.id == "closure-version")
            .unwrap();
        assert!(
            matches!(&version.geometry.features[0], OutlineFeature::PartEnvelope { part_ids, connections, .. }
            if part_ids == &vec![removed.to_owned()]
                && connections[0].points[0].part_id.is_none()
                && connections[0].points[1].part_id.as_deref() == Some(removed))
        );
        assert_eq!(version.geometry.bridges[0].part_ids, vec![removed]);
        assert_eq!(
            version.geometry.protected_gaps[0].points[0]
                .part_id
                .as_deref(),
            Some(removed)
        );
        assert_eq!(
            outline.generated_last_valid.as_ref().unwrap().bridges[0].part_ids,
            vec![removed]
        );

        let mirror = output
            .draft
            .matrices
            .iter()
            .find(|matrix| matrix.id == "mirror-matrix")
            .unwrap();
        assert_eq!((mirror.rows, mirror.columns), (2, 3));
        assert_eq!(mirror.part_ids.len(), 12);
        let d_diode = output
            .draft
            .parts
            .iter()
            .find(|part| part.id == "matrix/mirror-matrix/r0c2/diode")
            .unwrap();
        assert_eq!(
            d_diode.pose,
            Pose2 {
                at: Vec2 { x: 98.0, y: -56.0 },
                rotation: 75.0
            }
        );
        assert_eq!(
            d_diode.properties.as_ref().unwrap()["logical-payload"],
            serde_json::json!("matrix/mirror-matrix/r0c3/diode")
        );
        let moved_diode = output
            .context
            .associations
            .iter()
            .find(|association| association.source_id == "matrix/source-matrix/r0c2/diode")
            .unwrap();
        assert_eq!(moved_diode.mirror_id, "matrix/mirror-matrix/r0c2/diode");
        assert_eq!(
            moved_diode.raw_mode_override,
            Some(MirroredCompanionMode::LiteralMirrorWithTweak)
        );
        assert_eq!(
            moved_diode.raw_tweak,
            Some(MirroredCompanionTweak {
                x: 1.0,
                y: -2.0,
                rotation: 15.0
            })
        );
        let preserved_a = output
            .context
            .associations
            .iter()
            .find(|association| association.source_id == "matrix/source-matrix/r0c0/diode")
            .unwrap();
        assert_eq!(preserved_a.raw_mode_override, None);
        assert_eq!(
            preserved_a.effective_mode,
            MirroredCompanionMode::LiteralMirror
        );
        let preserved_c = output
            .context
            .associations
            .iter()
            .find(|association| association.source_id == "matrix/source-matrix/r0c1/diode")
            .unwrap();
        assert_eq!(
            preserved_c.raw_mode_override,
            Some(MirroredCompanionMode::LiteralMirror)
        );
        assert_eq!(preserved_c.raw_tweak, None);
        let retired_b = output
            .context_delta
            .retired
            .iter()
            .find(|association| association.source_id == "matrix/source-matrix/r0c1/diode")
            .unwrap();
        assert_eq!(
            retired_b.raw_mode_override,
            Some(MirroredCompanionMode::DuplicatePosition)
        );
        assert!(
            output
                .changed_document_ids
                .contains(&"mirror-board".to_owned())
        );
        let expected_changed_ids: BTreeSet<_> = std::iter::once("mirror-board".to_owned())
            .chain(std::iter::once("mirror-matrix".to_owned()))
            .chain([
                "closure-circuit".to_owned(),
                "closure-envelope".to_owned(),
                "closure-module".to_owned(),
                "retained-second-key-net".to_owned(),
                "retained-second-key-offset".to_owned(),
                "retained-keys-envelope".to_owned(),
                "shifted-module".to_owned(),
            ])
            .chain((0..2).flat_map(|row| {
                (1..4).flat_map(move |column| {
                    let key = format!("matrix/mirror-matrix/r{row}c{column}");
                    [key.clone(), format!("{key}/diode")]
                })
            }))
            .collect();
        assert_eq!(
            output
                .changed_document_ids
                .iter()
                .cloned()
                .collect::<BTreeSet<_>>(),
            expected_changed_ids
        );
        assert_eq!(output.context_delta.retired.len(), 4);
        assert_eq!(output.context_delta.relocated.len(), 8);
        assert_eq!(output.context.associations.len(), 13);
        assert_eq!(output.context_delta.owners, output.final_owners);
        assert_eq!(output.draft.definitions, post_source.definitions);
        assert_eq!(
            output
                .draft
                .parts
                .iter()
                .find(|part| part.id == "geometry-probe"),
            post_source
                .parts
                .iter()
                .find(|part| part.id == "geometry-probe")
        );
        let retained_net = output
            .draft
            .nets
            .iter()
            .find(|net| net.id == "retained-second-key-net")
            .unwrap();
        assert_eq!(retained_net.name, "RETAINED SECOND KEY");
        assert_eq!(
            retained_net.pins,
            vec![crate::model::Pin {
                part_id: "geometry-probe".into(),
                pad_id: "pad-two".into(),
            }]
        );
        assert!(!output.draft.constraints.iter().any(|constraint| {
            matches!(constraint, crate::model::Constraint::Offset { id, .. } if id == "retained-second-key-offset")
        }));
        for id in [
            "matrix/source-matrix/r0c3",
            "matrix/source-matrix/r0c3/diode",
            "matrix/source-matrix/r1c3",
            "matrix/source-matrix/r1c3/diode",
            "matrix/mirror-matrix/r0c3",
            "matrix/mirror-matrix/r0c3/diode",
            "matrix/mirror-matrix/r1c3",
            "matrix/mirror-matrix/r1c3/diode",
        ] {
            assert_eq!(
                output.final_owners.get(id),
                Some(&FinalOwner::Absent),
                "{id}"
            );
        }

        let remapped = output.context.clone();
        let next_map: BTreeMap<_, _> = output
            .draft
            .matrices
            .iter()
            .find(|matrix| matrix.id == "source-matrix")
            .unwrap()
            .part_ids
            .iter()
            .map(|id| (id.clone(), Some(id.clone())))
            .collect();
        let next_mirror_map: BTreeMap<_, _> = output
            .draft
            .matrices
            .iter()
            .find(|matrix| matrix.id == "mirror-matrix")
            .unwrap()
            .part_ids
            .iter()
            .map(|id| (id.clone(), Some(id.clone())))
            .collect();
        let repeated = prepare_membership_transition_kernel(MembershipTransitionInput {
            valid_before: &output.draft,
            post_source: &output.draft,
            context: &remapped,
            baseline_kind: BaselineKind::ChainedPrivateDraft,
            source_identity_map: &next_map,
            mirror_identity_map: &next_mirror_map,
            planned_additions: &additions,
            final_owners: &output.final_owners,
            explicit_removal_closure: &BTreeSet::new(),
        })
        .expect("the chained private state is valid for an identity repeat");
        assert_eq!(repeated.draft, output.draft);
        assert_eq!(repeated.context, output.context);
        assert!(repeated.changed_document_ids.is_empty());
        assert!(repeated.changed_field_paths.is_empty());
    }

    #[test]
    fn independent_transition_accepts_inherited_local_child_provenance() {
        let (before, _, mut context, _, _, _) = membership_compaction_fixture().unwrap();
        assert!(crate::mechanical::halves::validate_document(&before).is_ok());
        let parent = context
            .associations
            .iter_mut()
            .find(|a| a.source_id == "matrix/source-matrix/r1c0")
            .unwrap();
        parent.state = TransitionState::Local;
        parent.explicit_state = true;
        {
            let child = context
                .associations
                .iter_mut()
                .find(|a| a.source_id == "matrix/source-matrix/r1c0/diode")
                .unwrap();
            child.state = TransitionState::Local;
            child.explicit_state = false;
        }
        let constrained_parent = context
            .associations
            .iter_mut()
            .find(|association| association.source_id == "matrix/source-matrix/r1c1")
            .unwrap();
        constrained_parent.state = TransitionState::Local;
        constrained_parent.explicit_state = true;
        let constrained_child = context
            .associations
            .iter_mut()
            .find(|association| association.source_id == "matrix/source-matrix/r1c1/diode")
            .unwrap();
        constrained_child.state = TransitionState::Local;
        constrained_child.explicit_state = false;
        let child = context
            .associations
            .iter()
            .find(|a| a.source_id == "matrix/source-matrix/r1c0/diode")
            .unwrap();
        assert_eq!(
            child.parent_source_id.as_deref(),
            Some("matrix/source-matrix/r1c0")
        );
        assert_eq!(
            child.parent_mirror_id.as_deref(),
            Some("matrix/mirror-matrix/r1c0")
        );
        let source_map = before
            .matrices
            .iter()
            .find(|m| m.id == "source-matrix")
            .unwrap()
            .part_ids
            .iter()
            .map(|id| (id.clone(), Some(id.clone())))
            .collect();
        let mirror_map = before
            .matrices
            .iter()
            .find(|m| m.id == "mirror-matrix")
            .unwrap()
            .part_ids
            .iter()
            .map(|id| (id.clone(), Some(id.clone())))
            .collect();
        let result = prepare_membership_transition(MembershipTransitionInput {
            valid_before: &before,
            post_source: &before,
            context: &context,
            baseline_kind: BaselineKind::MaterializedCurrentSchema,
            source_identity_map: &source_map,
            mirror_identity_map: &mirror_map,
            planned_additions: &[],
            final_owners: &context.owners,
            explicit_removal_closure: &BTreeSet::new(),
        });
        assert!(
            result.is_ok(),
            "a child inheriting its explicit Local parent must be valid: {:?}",
            result.err()
        );
    }

    #[test]
    fn independent_transition_rejects_wrong_board_final_owner() {
        let (before, post_source, context, source_map, mut final_owners, closure) =
            membership_compaction_fixture().unwrap();
        assert!(crate::mechanical::halves::validate_document(&before).is_ok());
        let mirror_map = source_map
            .iter()
            .map(|(old, new)| {
                (
                    old.replace("matrix/source-matrix/", "matrix/mirror-matrix/"),
                    new.as_ref()
                        .map(|id| id.replace("matrix/source-matrix/", "matrix/mirror-matrix/")),
                )
            })
            .collect();
        let id = "matrix/mirror-matrix/r1c1/diode";
        final_owners.insert(id.into(), FinalOwner::Board("source-board".into()));
        let result = prepare_membership_transition(MembershipTransitionInput {
            valid_before: &before,
            post_source: &post_source,
            context: &context,
            baseline_kind: BaselineKind::MaterializedCurrentSchema,
            source_identity_map: &source_map,
            mirror_identity_map: &mirror_map,
            planned_additions: &[],
            final_owners: &final_owners,
            explicit_removal_closure: &closure,
        });
        if let Ok(output) = &result {
            let actual = output
                .draft
                .boards
                .iter()
                .find(|b| b.part_ids.contains(&id.to_owned()))
                .unwrap();
            assert_eq!(
                actual.id, "mirror-board",
                "the producer still registers the Part on its mirror board"
            );
            assert_eq!(
                output.final_owners[id],
                FinalOwner::Board("source-board".into()),
                "the returned owner declaration contradicts the actual draft"
            );
        }
        assert!(
            result.is_err(),
            "a contradictory final owner for the second-row companion must be rejected"
        );
    }

    #[test]
    fn independent_transition_rejects_inconsistent_raw_effective_settings() {
        let (before, post_source, mut context, source_map, final_owners, closure) =
            membership_compaction_fixture().unwrap();
        assert!(crate::mechanical::halves::validate_document(&before).is_ok());
        let mirror_map = source_map
            .iter()
            .map(|(old, new)| {
                (
                    old.replace("matrix/source-matrix/", "matrix/mirror-matrix/"),
                    new.as_ref()
                        .map(|id| id.replace("matrix/source-matrix/", "matrix/mirror-matrix/")),
                )
            })
            .collect();
        let child = context
            .associations
            .iter_mut()
            .find(|a| a.source_id == "matrix/source-matrix/r0c3/diode")
            .unwrap();
        assert_eq!(
            child.raw_mode_override,
            Some(MirroredCompanionMode::LiteralMirrorWithTweak)
        );
        assert_eq!(
            child.raw_tweak,
            Some(MirroredCompanionTweak {
                x: 1.0,
                y: -2.0,
                rotation: 15.0
            })
        );
        child.effective_mode = MirroredCompanionMode::DuplicatePosition;
        child.effective_tweak = None;
        let result = prepare_membership_transition(MembershipTransitionInput {
            valid_before: &before,
            post_source: &post_source,
            context: &context,
            baseline_kind: BaselineKind::MaterializedCurrentSchema,
            source_identity_map: &source_map,
            mirror_identity_map: &mirror_map,
            planned_additions: &[],
            final_owners: &final_owners,
            explicit_removal_closure: &closure,
        });
        assert!(
            result.is_err(),
            "effective settings must agree with the preserved raw override and tweak"
        );
    }

    #[test]
    fn membership_transition_rejects_missing_second_retained_child_owner_atomically() {
        let (before, post_source, context, source_map, mut final_owners, closure) =
            membership_compaction_fixture().expect("the materialized compaction fixture is valid");
        let missing = "matrix/mirror-matrix/r1c1/diode";
        assert!(final_owners.remove(missing).is_some());
        let before_snapshot = before.clone();
        let post_source_snapshot = post_source.clone();
        let context_snapshot = context.clone();
        let result = prepare_membership_transition(MembershipTransitionInput {
            valid_before: &before,
            post_source: &post_source,
            context: &context,
            baseline_kind: BaselineKind::MaterializedCurrentSchema,
            source_identity_map: &source_map,
            mirror_identity_map: &source_map
                .iter()
                .map(|(old, new)| {
                    (
                        old.replace("matrix/source-matrix/", "matrix/mirror-matrix/"),
                        new.as_ref()
                            .map(|id| id.replace("matrix/source-matrix/", "matrix/mirror-matrix/")),
                    )
                })
                .collect(),
            planned_additions: &[],
            final_owners: &final_owners,
            explicit_removal_closure: &closure,
        });
        assert!(result.is_err());
        assert_eq!(before, before_snapshot);
        assert_eq!(post_source, post_source_snapshot);
        assert_eq!(context, context_snapshot);
    }

    // Independent literal correction of the writer's compaction before fixture.
    // No projector computes these mode/tweak expectations. B uses (2,3)@30;
    // D reflects then applies (1,-2,+15), yielding (-1,1)@-15.
    fn repair_fresh_coherent_compaction_fixture() -> Result<CompactionFixture, String> {
        let (mut before, _, context, source_map, owners, closure) =
            membership_compaction_fixture()?;
        let mut mirror = before
            .matrices
            .iter()
            .find(|m| m.id == "mirror-matrix")
            .unwrap()
            .clone();
        for cell in &mut mirror.cells {
            let assembly = cell
                .assemblies
                .iter_mut()
                .find(|a| a.id == "diode")
                .unwrap();
            match cell.column {
                1 => {
                    assembly.offset = Vec2 { x: 2.0, y: 3.0 };
                    assembly.rotation = Some(30.0);
                }
                3 => {
                    assembly.offset = Vec2 { x: -1.0, y: 1.0 };
                    assembly.rotation = Some(-15.0);
                }
                _ => {}
            }
        }
        crate::matrix::set_matrix(&mut before, &mirror)?;
        let Constraint::Offset {
            offset, rotation, ..
        } = before
            .constraints
            .iter_mut()
            .find(|c| c.id() == "retained-second-key-offset")
            .unwrap()
        else {
            unreachable!()
        };
        *offset = Vec2 { x: 75.0, y: -47.0 };
        *rotation = 90.0;
        crate::mechanical::halves::validate_document(&before)?;
        for matrix in &before.matrices {
            crate::matrix::valid_matrix(matrix, &before)?;
        }
        for (id, x, y, rotation) in [
            ("matrix/mirror-matrix/r0c0/diode", 96.0, -37.0, 60.0),
            ("matrix/mirror-matrix/r1c0/diode", 76.0, -37.0, 60.0),
            ("matrix/mirror-matrix/r0c1/diode", 96.0, -43.0, 120.0),
            ("matrix/mirror-matrix/r1c1/diode", 76.0, -43.0, 120.0),
            ("matrix/mirror-matrix/r0c2/diode", 96.0, -57.0, 60.0),
            ("matrix/mirror-matrix/r1c2/diode", 76.0, -57.0, 60.0),
            ("matrix/mirror-matrix/r0c3/diode", 98.0, -66.0, 75.0),
            ("matrix/mirror-matrix/r1c3/diode", 78.0, -66.0, 75.0),
        ] {
            let actual = before.parts.iter().find(|p| p.id == id).unwrap().pose;
            assert!(
                (actual.at.x - x).abs() < 1e-9
                    && (actual.at.y - y).abs() < 1e-9
                    && (actual.rotation - rotation).abs() < 1e-9,
                "independent valid-before literal {id}: {actual:?}"
            );
        }
        for part in &before.parts {
            assert_eq!(before.parts.iter().filter(|p| p.id == part.id).count(), 1);
            assert_eq!(
                before
                    .boards
                    .iter()
                    .filter(|b| b.part_ids.contains(&part.id))
                    .count(),
                1
            );
            assert!(
                part.pose.at.x.is_finite()
                    && part.pose.at.y.is_finite()
                    && part.pose.rotation.is_finite()
            );
        }
        for net in &before.nets {
            for pin in &net.pins {
                let part = before.parts.iter().find(|p| p.id == pin.part_id).unwrap();
                let definition = before
                    .definitions
                    .iter()
                    .find(|d| d.id == part.definition_id)
                    .unwrap();
                assert!(definition.pads.iter().any(|p| p.id == pin.pad_id));
            }
        }
        for constraint in &before.constraints {
            let source = before
                .parts
                .iter()
                .find(|p| p.id == constraint.source())
                .unwrap();
            let target = before
                .parts
                .iter()
                .find(|p| p.id == constraint.target())
                .unwrap();
            if let Constraint::Offset {
                offset, rotation, ..
            } = constraint
            {
                assert!((target.pose.at.x - source.pose.at.x - offset.x).abs() < 1e-9);
                assert!((target.pose.at.y - source.pose.at.y - offset.y).abs() < 1e-9);
                assert!((target.pose.rotation - source.pose.rotation - rotation).abs() < 1e-9);
            }
        }
        let mut post = before.clone();
        crate::matrix::deletion::remove_axis(&mut post, "source-matrix", 1, false)?;
        Ok((before, post, context, source_map, owners, closure))
    }

    #[test]
    fn repair_fresh_corrected_literal_control_compacts_and_repeats() {
        let (before, post, context, source_map, owners, closure) =
            repair_fresh_coherent_compaction_fixture().unwrap();
        let mirror_map = source_map
            .iter()
            .map(|(old, new)| {
                (
                    old.replace("matrix/source-matrix/", "matrix/mirror-matrix/"),
                    new.as_ref()
                        .map(|id| id.replace("matrix/source-matrix/", "matrix/mirror-matrix/")),
                )
            })
            .collect();
        let output = prepare_membership_transition(MembershipTransitionInput {
            valid_before: &before,
            post_source: &post,
            context: &context,
            baseline_kind: BaselineKind::MaterializedCurrentSchema,
            source_identity_map: &source_map,
            mirror_identity_map: &mirror_map,
            planned_additions: &[],
            final_owners: &owners,
            explicit_removal_closure: &closure,
        })
        .unwrap();
        for (id, x, y) in [
            ("matrix/mirror-matrix/r0c2/diode", 98.0, -56.0),
            ("matrix/mirror-matrix/r1c2/diode", 78.0, -56.0),
        ] {
            assert_eq!(
                output.draft.parts.iter().find(|p| p.id == id).unwrap().pose,
                Pose2 {
                    at: Vec2 { x, y },
                    rotation: 75.0
                }
            );
        }
        let source_map = output
            .draft
            .matrices
            .iter()
            .find(|m| m.id == "source-matrix")
            .unwrap()
            .part_ids
            .iter()
            .map(|id| (id.clone(), Some(id.clone())))
            .collect();
        let mirror_map = output
            .draft
            .matrices
            .iter()
            .find(|m| m.id == "mirror-matrix")
            .unwrap()
            .part_ids
            .iter()
            .map(|id| (id.clone(), Some(id.clone())))
            .collect();
        let repeated = prepare_membership_transition(MembershipTransitionInput {
            valid_before: &output.draft,
            post_source: &output.draft,
            context: &output.context,
            baseline_kind: BaselineKind::ChainedPrivateDraft,
            source_identity_map: &source_map,
            mirror_identity_map: &mirror_map,
            planned_additions: &[],
            final_owners: &output.final_owners,
            explicit_removal_closure: &BTreeSet::new(),
        })
        .unwrap();
        assert_eq!(repeated.draft, output.draft);
        assert_eq!(repeated.context, output.context);
        assert!(repeated.changed_document_ids.is_empty());
        assert!(repeated.changed_field_paths.is_empty());
    }

    #[test]
    fn repair_fresh_refuses_invalid_before_pad_identity() {
        let (mut before, _, context, _, _, _) = repair_fresh_coherent_compaction_fixture().unwrap();
        let net = before
            .nets
            .iter_mut()
            .find(|n| n.id == "retained-second-key-net")
            .unwrap();
        assert_eq!(net.pins[0].pad_id, "pad-one");
        net.pins[0].pad_id = "1".into();
        let part = before
            .parts
            .iter()
            .find(|p| p.id == net.pins[0].part_id)
            .unwrap();
        let definition = before
            .definitions
            .iter()
            .find(|d| d.id == part.definition_id)
            .unwrap();
        assert!(definition.pads.iter().any(|p| p.number == "1"));
        assert!(!definition.pads.iter().any(|p| p.id == "1"));
        assert!(crate::mechanical::halves::validate_document(&before).is_ok());
        let source_map = before
            .matrices
            .iter()
            .find(|m| m.id == "source-matrix")
            .unwrap()
            .part_ids
            .iter()
            .map(|id| (id.clone(), Some(id.clone())))
            .collect();
        let mirror_map = before
            .matrices
            .iter()
            .find(|m| m.id == "mirror-matrix")
            .unwrap()
            .part_ids
            .iter()
            .map(|id| (id.clone(), Some(id.clone())))
            .collect();
        let snapshot = before.clone();
        let context_snapshot = context.clone();
        let result = prepare_membership_transition(MembershipTransitionInput {
            valid_before: &before,
            post_source: &before,
            context: &context,
            baseline_kind: BaselineKind::MaterializedCurrentSchema,
            source_identity_map: &source_map,
            mirror_identity_map: &mirror_map,
            planned_additions: &[],
            final_owners: &context.owners,
            explicit_removal_closure: &BTreeSet::new(),
        });
        assert_eq!(before, snapshot);
        assert_eq!(context, context_snapshot);
        assert!(
            result.is_err(),
            "a touched historical net referencing Pad.number instead of actual Pad.id must refuse before staging"
        );
    }

    #[test]
    fn repair_fresh_refuses_chained_identity_ambiguous_actual_board_owner() {
        let (before, post, context, source_map, owners, closure) =
            repair_fresh_coherent_compaction_fixture().unwrap();
        let mirror_map = source_map
            .iter()
            .map(|(old, new)| {
                (
                    old.replace("matrix/source-matrix/", "matrix/mirror-matrix/"),
                    new.as_ref()
                        .map(|id| id.replace("matrix/source-matrix/", "matrix/mirror-matrix/")),
                )
            })
            .collect();
        let output = prepare_membership_transition(MembershipTransitionInput {
            valid_before: &before,
            post_source: &post,
            context: &context,
            baseline_kind: BaselineKind::MaterializedCurrentSchema,
            source_identity_map: &source_map,
            mirror_identity_map: &mirror_map,
            planned_additions: &[],
            final_owners: &owners,
            explicit_removal_closure: &closure,
        })
        .expect("establish an actually returned private draft and full context first");
        let mut invalid = output.draft.clone();
        let second_child = "matrix/mirror-matrix/r1c1/diode";
        invalid
            .boards
            .iter_mut()
            .find(|b| b.id == "source-board")
            .unwrap()
            .part_ids
            .push(second_child.into());
        assert_eq!(
            invalid
                .boards
                .iter()
                .filter(|b| b.part_ids.iter().any(|id| id == second_child))
                .count(),
            2
        );
        let source_map = invalid
            .matrices
            .iter()
            .find(|m| m.id == "source-matrix")
            .unwrap()
            .part_ids
            .iter()
            .map(|id| (id.clone(), Some(id.clone())))
            .collect();
        let mirror_map = invalid
            .matrices
            .iter()
            .find(|m| m.id == "mirror-matrix")
            .unwrap()
            .part_ids
            .iter()
            .map(|id| (id.clone(), Some(id.clone())))
            .collect();
        let snapshot = invalid.clone();
        let context_snapshot = output.context.clone();
        let result = prepare_membership_transition(MembershipTransitionInput {
            valid_before: &invalid,
            post_source: &invalid,
            context: &output.context,
            baseline_kind: BaselineKind::ChainedPrivateDraft,
            source_identity_map: &source_map,
            mirror_identity_map: &mirror_map,
            planned_additions: &[],
            final_owners: &output.final_owners,
            explicit_removal_closure: &BTreeSet::new(),
        });
        assert_eq!(invalid, snapshot);
        assert_eq!(output.context, context_snapshot);
        assert!(
            result.is_err(),
            "a chained no-op must validate the second child has exactly one actual board owner"
        );
    }

    #[test]
    fn repair_fresh_refuses_wrong_historical_owner_context() {
        let (before, post, mut context, source_map, owners, closure) =
            repair_fresh_coherent_compaction_fixture().unwrap();
        let second_child = "matrix/mirror-matrix/r1c1/diode";
        assert_eq!(
            context.owners[second_child],
            FinalOwner::Board("mirror-board".into())
        );
        context.owners.insert(
            second_child.into(),
            FinalOwner::Board("source-board".into()),
        );
        assert!(
            before
                .boards
                .iter()
                .find(|b| b.id == "mirror-board")
                .unwrap()
                .part_ids
                .iter()
                .any(|id| id == second_child)
        );
        let mirror_map = source_map
            .iter()
            .map(|(old, new)| {
                (
                    old.replace("matrix/source-matrix/", "matrix/mirror-matrix/"),
                    new.as_ref()
                        .map(|id| id.replace("matrix/source-matrix/", "matrix/mirror-matrix/")),
                )
            })
            .collect();
        let before_snapshot = before.clone();
        let post_snapshot = post.clone();
        let context_snapshot = context.clone();
        let result = prepare_membership_transition(MembershipTransitionInput {
            valid_before: &before,
            post_source: &post,
            context: &context,
            baseline_kind: BaselineKind::MaterializedCurrentSchema,
            source_identity_map: &source_map,
            mirror_identity_map: &mirror_map,
            planned_additions: &[],
            final_owners: &owners,
            explicit_removal_closure: &closure,
        });
        assert_eq!(before, before_snapshot);
        assert_eq!(post, post_snapshot);
        assert_eq!(context, context_snapshot);
        assert!(
            result.is_err(),
            "the valid-before owner context must match the historical board registration, even when final owners are correct"
        );
    }

    #[test]
    fn repair_fresh_refuses_surviving_linked_key_constraint() {
        let (mut before, _, context, source_map, owners, closure) =
            repair_fresh_coherent_compaction_fixture().unwrap();
        let constraint = Constraint::Offset {
            id: "fresh-linked-C-key".into(),
            source_part_id: "geometry-probe".into(),
            target_part_id: "matrix/mirror-matrix/r0c2".into(),
            offset: Vec2 { x: 95.0, y: -57.0 },
            rotation: 90.0,
        };
        let key = before
            .parts
            .iter()
            .find(|p| p.id == constraint.target())
            .unwrap();
        assert_eq!(
            key.pose,
            Pose2 {
                at: Vec2 { x: 99.0, y: -55.0 },
                rotation: 90.0
            }
        );
        before.constraints.push(constraint);
        let mut post = before.clone();
        crate::matrix::deletion::remove_axis(&mut post, "source-matrix", 1, false).unwrap();
        let mirror_map = source_map
            .iter()
            .map(|(old, new)| {
                (
                    old.replace("matrix/source-matrix/", "matrix/mirror-matrix/"),
                    new.as_ref()
                        .map(|id| id.replace("matrix/source-matrix/", "matrix/mirror-matrix/")),
                )
            })
            .collect();
        let before_snapshot = before.clone();
        let post_snapshot = post.clone();
        let context_snapshot = context.clone();
        let result = prepare_membership_transition(MembershipTransitionInput {
            valid_before: &before,
            post_source: &post,
            context: &context,
            baseline_kind: BaselineKind::MaterializedCurrentSchema,
            source_identity_map: &source_map,
            mirror_identity_map: &mirror_map,
            planned_additions: &[],
            final_owners: &owners,
            explicit_removal_closure: &closure,
        });
        assert_eq!(before, before_snapshot);
        assert_eq!(post, post_snapshot);
        assert_eq!(context, context_snapshot);
        if let Ok(output) = &result {
            let key = output
                .draft
                .parts
                .iter()
                .find(|p| p.id == "matrix/mirror-matrix/r0c1")
                .unwrap();
            let constraint = output
                .draft
                .constraints
                .iter()
                .find(|c| c.id() == "fresh-linked-C-key")
                .unwrap();
            assert_eq!(key.pose.at, Vec2 { x: 99.0, y: -45.0 });
            assert_eq!(constraint.target(), key.id);
            assert!(
                matches!(constraint, Constraint::Offset { offset, .. } if *offset == Vec2 { x: 95.0, y: -57.0 })
            );
        }
        assert!(
            result.is_err(),
            "surviving constrained linked keys must refuse; otherwise C moves while its retained constraint offset is stale"
        );
    }

    #[test]
    fn repair_fresh_omission_cleans_idless_dependencies() {
        let (mut before, _, mut context, source_map, mut owners, mut closure) =
            repair_fresh_coherent_compaction_fixture().unwrap();
        let parent = "matrix/source-matrix/r0c0";
        let target_parent = "matrix/mirror-matrix/r0c0";
        let target_child = "matrix/mirror-matrix/r0c0/diode";
        for association in context
            .associations
            .iter_mut()
            .filter(|a| a.source_id == parent || a.parent_source_id.as_deref() == Some(parent))
        {
            association.state = TransitionState::Omitted;
            association.explicit_state = association.parent_source_id.is_none();
        }
        let mut keymap = KeymapConfiguration::default();
        keymap.layers[0].bindings.insert(
            target_child.into(),
            KeyBinding::KeyPress {
                keycode: "OMITTED_SENTINEL".into(),
            },
        );
        before.keymap = Some(keymap);
        let mut post = before.clone();
        crate::matrix::deletion::remove_axis(&mut post, "source-matrix", 1, false).unwrap();
        let mut mirror_map: BTreeMap<_, _> = source_map
            .iter()
            .map(|(old, new)| {
                (
                    old.replace("matrix/source-matrix/", "matrix/mirror-matrix/"),
                    new.as_ref()
                        .map(|id| id.replace("matrix/source-matrix/", "matrix/mirror-matrix/")),
                )
            })
            .collect();
        for id in [target_parent, target_child] {
            mirror_map.insert(id.into(), None);
            owners.insert(id.into(), FinalOwner::Absent);
            closure.insert(id.into());
        }
        let output = prepare_membership_transition_kernel(MembershipTransitionInput {
            valid_before: &before,
            post_source: &post,
            context: &context,
            baseline_kind: BaselineKind::MaterializedCurrentSchema,
            source_identity_map: &source_map,
            mirror_identity_map: &mirror_map,
            planned_additions: &[],
            final_owners: &owners,
            explicit_removal_closure: &closure,
        })
        .expect(
            "a complete explicit Omitted closure under the accepted source operation is supported",
        );
        assert!(!output.draft.parts.iter().any(|p| p.id == target_child));
        assert_eq!(output.final_owners[target_child], FinalOwner::Absent);
        assert!(
            !output.draft.keymap.as_ref().unwrap().layers[0]
                .bindings
                .contains_key(target_child),
            "omitted geometry removal must clean populated keymap dependencies too"
        );
    }

    #[test]
    fn repair_fresh_accepts_explicit_omitted_child_of_local_owner() {
        let (before, post, mut context, source_map, mut owners, mut closure) =
            repair_fresh_coherent_compaction_fixture().unwrap();
        let parent = "matrix/source-matrix/r1c0";
        let target_child = "matrix/mirror-matrix/r1c0/diode";
        let parent_record = context
            .associations
            .iter_mut()
            .find(|a| a.source_id == parent)
            .unwrap();
        parent_record.state = TransitionState::Local;
        parent_record.explicit_state = true;
        let child = context
            .associations
            .iter_mut()
            .find(|a| a.parent_source_id.as_deref() == Some(parent))
            .unwrap();
        child.state = TransitionState::Omitted;
        child.explicit_state = true;
        assert_eq!(
            child.parent_mirror_id.as_deref(),
            Some("matrix/mirror-matrix/r1c0")
        );
        let mut mirror_map: BTreeMap<_, _> = source_map
            .iter()
            .map(|(old, new)| {
                (
                    old.replace("matrix/source-matrix/", "matrix/mirror-matrix/"),
                    new.as_ref()
                        .map(|id| id.replace("matrix/source-matrix/", "matrix/mirror-matrix/")),
                )
            })
            .collect();
        mirror_map.insert(target_child.into(), None);
        owners.insert(target_child.into(), FinalOwner::Absent);
        closure.insert(target_child.into());
        let result = prepare_membership_transition_kernel(MembershipTransitionInput {
            valid_before: &before,
            post_source: &post,
            context: &context,
            baseline_kind: BaselineKind::MaterializedCurrentSchema,
            source_identity_map: &source_map,
            mirror_identity_map: &mirror_map,
            planned_additions: &[],
            final_owners: &owners,
            explicit_removal_closure: &closure,
        });
        assert!(
            result.is_ok(),
            "a valid explicit Omitted child override must survive its Local parent policy: {:?}",
            result.err()
        );
    }

    #[test]
    fn repair_fresh_accepts_shifted_local_parent_with_explicit_omitted_child() {
        let (before, post, mut context, source_map, mut owners, mut closure) =
            repair_fresh_coherent_compaction_fixture().unwrap();
        let parent = "matrix/source-matrix/r1c2";
        let target_child = "matrix/mirror-matrix/r1c2/diode";
        let parent_record = context
            .associations
            .iter_mut()
            .find(|a| a.source_id == parent)
            .unwrap();
        parent_record.state = TransitionState::Local;
        parent_record.explicit_state = true;
        let child = context
            .associations
            .iter_mut()
            .find(|a| a.parent_source_id.as_deref() == Some(parent))
            .unwrap();
        child.state = TransitionState::Omitted;
        child.explicit_state = true;
        assert_eq!(
            child.parent_mirror_id.as_deref(),
            Some("matrix/mirror-matrix/r1c2")
        );
        let mut mirror_map: BTreeMap<_, _> = source_map
            .iter()
            .map(|(old, new)| {
                (
                    old.replace("matrix/source-matrix/", "matrix/mirror-matrix/"),
                    new.as_ref()
                        .map(|id| id.replace("matrix/source-matrix/", "matrix/mirror-matrix/")),
                )
            })
            .collect();
        mirror_map.insert(target_child.into(), None);
        owners.insert("matrix/mirror-matrix/r1c1/diode".into(), FinalOwner::Absent);
        closure.insert(target_child.into());
        let result = prepare_membership_transition_kernel(MembershipTransitionInput {
            valid_before: &before,
            post_source: &post,
            context: &context,
            baseline_kind: BaselineKind::MaterializedCurrentSchema,
            source_identity_map: &source_map,
            mirror_identity_map: &mirror_map,
            planned_additions: &[],
            final_owners: &owners,
            explicit_removal_closure: &closure,
        });
        assert!(
            result.is_ok(),
            "a valid explicit Omitted child override must survive its shifted Local parent compaction policy: {:?}",
            result.err()
        );
    }

    #[test]
    fn repair_fresh_refuses_native_and_legacy_matrix_overlap() {
        let (mut before, _, mut context, _, _, _) =
            repair_fresh_coherent_compaction_fixture().unwrap();
        let mut third = before
            .matrices
            .iter()
            .find(|m| m.id == "source-matrix")
            .unwrap()
            .clone();
        third.id = "legacy-third-matrix".into();
        third.part_ids.clear();
        crate::matrix::set_matrix(&mut before, &third).unwrap();
        before.layouts = vec![
            Layout {
                id: "legacy-source-layout".into(),
                name: "Legacy source".into(),
                board_id: "source-board".into(),
                matrix_id: "source-matrix".into(),
                part_ids: vec![],
                mirror_link: None,
            },
            Layout {
                id: "legacy-third-layout".into(),
                name: "Legacy third".into(),
                board_id: "source-board".into(),
                matrix_id: "legacy-third-matrix".into(),
                part_ids: vec![],
                mirror_link: Some(LayoutMirrorLink {
                    source_id: "legacy-source-layout".into(),
                    axis_x: 10.0,
                }),
            },
        ];
        before
            .constraints
            .retain(|constraint| constraint.id() != "retained-second-key-offset");
        crate::matrix::layout::validate(&before)
            .expect("the legacy pair alone is valid on one board");
        crate::mechanical::halves::validate_document(&before)
            .expect("the native record alone is valid");
        for part in &before.parts {
            let board = before
                .boards
                .iter()
                .find(|b| b.part_ids.contains(&part.id))
                .unwrap();
            context
                .owners
                .insert(part.id.clone(), FinalOwner::Board(board.id.clone()));
        }
        let source_map = before
            .matrices
            .iter()
            .find(|m| m.id == "source-matrix")
            .unwrap()
            .part_ids
            .iter()
            .map(|id| (id.clone(), Some(id.clone())))
            .collect();
        let mirror_map = before
            .matrices
            .iter()
            .find(|m| m.id == "mirror-matrix")
            .unwrap()
            .part_ids
            .iter()
            .map(|id| (id.clone(), Some(id.clone())))
            .collect();
        let result = prepare_membership_transition(MembershipTransitionInput {
            valid_before: &before,
            post_source: &before,
            context: &context,
            baseline_kind: BaselineKind::MaterializedCurrentSchema,
            source_identity_map: &source_map,
            mirror_identity_map: &mirror_map,
            planned_additions: &[],
            final_owners: &context.owners,
            explicit_removal_closure: &BTreeSet::new(),
        });
        assert!(
            result.is_err(),
            "a matrix with both valid native and same-board legacy links must refuse this transition"
        );
    }

    #[test]
    fn repair_fresh_refuses_unbound_planned_source_reference() {
        let (mut before, _, context, _, _, _) =
            repair_fresh_coherent_compaction_fixture().expect("the linked matrix fixture is valid");
        before
            .constraints
            .retain(|c| c.id() != "retained-second-key-offset");
        let mut post_source = before.clone();
        let source_matrix = before
            .matrices
            .iter()
            .find(|matrix| matrix.id == "source-matrix")
            .unwrap();
        let mut expanded = source_matrix.clone();
        expanded.columns = 5;
        for row in 0..2 {
            expanded.cells.push(MatrixCell {
                deleted: false,
                row,
                column: 4,
                enabled: true,
                definition_id: None,
                variant: None,
                offset: None,
                rotation: None,
                assemblies: vec![MatrixAssembly {
                    id: "diode".into(),
                    definition_id: "diode".into(),
                    offset: Vec2 { x: 2.0, y: 3.0 },
                    rotation: Some(30.0),
                    side: Some(Side::Back),
                }],
                assemblies_local: None,
            });
        }
        crate::matrix::set_matrix(&mut post_source, &expanded)
            .expect("the planned source additions materialize");

        let source_identity_map: BTreeMap<_, _> = source_matrix
            .part_ids
            .iter()
            .map(|id| (id.clone(), Some(id.clone())))
            .collect();
        let mirror_identity_map: BTreeMap<_, _> = before
            .matrices
            .iter()
            .find(|matrix| matrix.id == "mirror-matrix")
            .unwrap()
            .part_ids
            .iter()
            .map(|id| (id.clone(), Some(id.clone())))
            .collect();
        let mut additions = Vec::new();
        for row in 0..2 {
            let source_key = format!("matrix/source-matrix/r{row}c4");
            let mirror_key = format!("matrix/mirror-matrix/r{row}c4");
            for (source_id, mirror_id, parent_source_id, parent_mirror_id, mirror_reference) in [
                (
                    source_key.clone(),
                    mirror_key.clone(),
                    None,
                    None,
                    format!("EXP_M_R{row}C4"),
                ),
                (
                    format!("{source_key}/diode"),
                    format!("{mirror_key}/diode"),
                    Some(source_key.clone()),
                    Some(mirror_key.clone()),
                    format!("EXP_M_D_R{row}C4"),
                ),
            ] {
                let source_part = post_source
                    .parts
                    .iter()
                    .find(|part| part.id == source_id)
                    .unwrap();
                additions.push(PlannedAddition {
                    source_id,
                    mirror_id,
                    source_reference: source_part.reference.clone(),
                    mirror_reference,
                    parent_source_id,
                    parent_mirror_id,
                });
            }
        }
        let mut final_owners: BTreeMap<_, _> = post_source
            .parts
            .iter()
            .map(|part| {
                let board = post_source
                    .boards
                    .iter()
                    .find(|board| board.part_ids.contains(&part.id))
                    .unwrap();
                (part.id.clone(), FinalOwner::Board(board.id.clone()))
            })
            .collect();
        for addition in &additions {
            final_owners.insert(
                addition.mirror_id.clone(),
                FinalOwner::Board("mirror-board".into()),
            );
        }

        additions[0].source_reference = "SUPPLIED_BUT_NOT_ACTUAL_SOURCE_REFERENCE".into();

        let result = prepare_membership_transition(MembershipTransitionInput {
            valid_before: &before,
            post_source: &post_source,
            context: &context,
            baseline_kind: BaselineKind::MaterializedCurrentSchema,
            source_identity_map: &source_identity_map,
            mirror_identity_map: &mirror_identity_map,
            planned_additions: &additions,
            final_owners: &final_owners,
            explicit_removal_closure: &BTreeSet::new(),
        });
        assert!(
            result.is_err(),
            "planned source reference must bind the actual post-source Part"
        );
    }

    #[test]
    fn repair_fresh_refuses_missing_planned_child_parent() {
        let (mut before, _, context, _, _, _) =
            repair_fresh_coherent_compaction_fixture().expect("the linked matrix fixture is valid");
        before
            .constraints
            .retain(|c| c.id() != "retained-second-key-offset");
        let mut post_source = before.clone();
        let source_matrix = before
            .matrices
            .iter()
            .find(|matrix| matrix.id == "source-matrix")
            .unwrap();
        let mut expanded = source_matrix.clone();
        expanded.columns = 5;
        for row in 0..2 {
            expanded.cells.push(MatrixCell {
                deleted: false,
                row,
                column: 4,
                enabled: true,
                definition_id: None,
                variant: None,
                offset: None,
                rotation: None,
                assemblies: vec![MatrixAssembly {
                    id: "diode".into(),
                    definition_id: "diode".into(),
                    offset: Vec2 { x: 2.0, y: 3.0 },
                    rotation: Some(30.0),
                    side: Some(Side::Back),
                }],
                assemblies_local: None,
            });
        }
        crate::matrix::set_matrix(&mut post_source, &expanded)
            .expect("the planned source additions materialize");

        let source_identity_map: BTreeMap<_, _> = source_matrix
            .part_ids
            .iter()
            .map(|id| (id.clone(), Some(id.clone())))
            .collect();
        let mirror_identity_map: BTreeMap<_, _> = before
            .matrices
            .iter()
            .find(|matrix| matrix.id == "mirror-matrix")
            .unwrap()
            .part_ids
            .iter()
            .map(|id| (id.clone(), Some(id.clone())))
            .collect();
        let mut additions = Vec::new();
        for row in 0..2 {
            let source_key = format!("matrix/source-matrix/r{row}c4");
            let mirror_key = format!("matrix/mirror-matrix/r{row}c4");
            for (source_id, mirror_id, parent_source_id, parent_mirror_id, mirror_reference) in [
                (
                    source_key.clone(),
                    mirror_key.clone(),
                    None,
                    None,
                    format!("EXP_M_R{row}C4"),
                ),
                (
                    format!("{source_key}/diode"),
                    format!("{mirror_key}/diode"),
                    Some(source_key.clone()),
                    Some(mirror_key.clone()),
                    format!("EXP_M_D_R{row}C4"),
                ),
            ] {
                let source_part = post_source
                    .parts
                    .iter()
                    .find(|part| part.id == source_id)
                    .unwrap();
                additions.push(PlannedAddition {
                    source_id,
                    mirror_id,
                    source_reference: source_part.reference.clone(),
                    mirror_reference,
                    parent_source_id,
                    parent_mirror_id,
                });
            }
        }
        let mut final_owners: BTreeMap<_, _> = post_source
            .parts
            .iter()
            .map(|part| {
                let board = post_source
                    .boards
                    .iter()
                    .find(|board| board.part_ids.contains(&part.id))
                    .unwrap();
                (part.id.clone(), FinalOwner::Board(board.id.clone()))
            })
            .collect();
        for addition in &additions {
            final_owners.insert(
                addition.mirror_id.clone(),
                FinalOwner::Board("mirror-board".into()),
            );
        }

        additions[1].parent_source_id = Some("matrix/source-matrix/r0c99".into());
        additions[1].parent_mirror_id = Some("matrix/mirror-matrix/r0c99".into());

        let result = prepare_membership_transition(MembershipTransitionInput {
            valid_before: &before,
            post_source: &post_source,
            context: &context,
            baseline_kind: BaselineKind::MaterializedCurrentSchema,
            source_identity_map: &source_identity_map,
            mirror_identity_map: &mirror_identity_map,
            planned_additions: &additions,
            final_owners: &final_owners,
            explicit_removal_closure: &BTreeSet::new(),
        });
        assert!(
            result.is_err(),
            "planned child parent closure must exist and match before staging"
        );
    }

    #[test]
    fn repair_fresh_refuses_duplicate_planned_mirror_references() {
        let (mut before, _, context, _, _, _) =
            repair_fresh_coherent_compaction_fixture().expect("the linked matrix fixture is valid");
        before
            .constraints
            .retain(|c| c.id() != "retained-second-key-offset");
        let mut post_source = before.clone();
        let source_matrix = before
            .matrices
            .iter()
            .find(|matrix| matrix.id == "source-matrix")
            .unwrap();
        let mut expanded = source_matrix.clone();
        expanded.columns = 5;
        for row in 0..2 {
            expanded.cells.push(MatrixCell {
                deleted: false,
                row,
                column: 4,
                enabled: true,
                definition_id: None,
                variant: None,
                offset: None,
                rotation: None,
                assemblies: vec![MatrixAssembly {
                    id: "diode".into(),
                    definition_id: "diode".into(),
                    offset: Vec2 { x: 2.0, y: 3.0 },
                    rotation: Some(30.0),
                    side: Some(Side::Back),
                }],
                assemblies_local: None,
            });
        }
        crate::matrix::set_matrix(&mut post_source, &expanded)
            .expect("the planned source additions materialize");

        let source_identity_map: BTreeMap<_, _> = source_matrix
            .part_ids
            .iter()
            .map(|id| (id.clone(), Some(id.clone())))
            .collect();
        let mirror_identity_map: BTreeMap<_, _> = before
            .matrices
            .iter()
            .find(|matrix| matrix.id == "mirror-matrix")
            .unwrap()
            .part_ids
            .iter()
            .map(|id| (id.clone(), Some(id.clone())))
            .collect();
        let mut additions = Vec::new();
        for row in 0..2 {
            let source_key = format!("matrix/source-matrix/r{row}c4");
            let mirror_key = format!("matrix/mirror-matrix/r{row}c4");
            for (source_id, mirror_id, parent_source_id, parent_mirror_id, mirror_reference) in [
                (
                    source_key.clone(),
                    mirror_key.clone(),
                    None,
                    None,
                    format!("EXP_M_R{row}C4"),
                ),
                (
                    format!("{source_key}/diode"),
                    format!("{mirror_key}/diode"),
                    Some(source_key.clone()),
                    Some(mirror_key.clone()),
                    format!("EXP_M_D_R{row}C4"),
                ),
            ] {
                let source_part = post_source
                    .parts
                    .iter()
                    .find(|part| part.id == source_id)
                    .unwrap();
                additions.push(PlannedAddition {
                    source_id,
                    mirror_id,
                    source_reference: source_part.reference.clone(),
                    mirror_reference,
                    parent_source_id,
                    parent_mirror_id,
                });
            }
        }
        let mut final_owners: BTreeMap<_, _> = post_source
            .parts
            .iter()
            .map(|part| {
                let board = post_source
                    .boards
                    .iter()
                    .find(|board| board.part_ids.contains(&part.id))
                    .unwrap();
                (part.id.clone(), FinalOwner::Board(board.id.clone()))
            })
            .collect();
        for addition in &additions {
            final_owners.insert(
                addition.mirror_id.clone(),
                FinalOwner::Board("mirror-board".into()),
            );
        }

        additions[2].mirror_reference = additions[0].mirror_reference.clone();

        let result = prepare_membership_transition(MembershipTransitionInput {
            valid_before: &before,
            post_source: &post_source,
            context: &context,
            baseline_kind: BaselineKind::MaterializedCurrentSchema,
            source_identity_map: &source_identity_map,
            mirror_identity_map: &mirror_identity_map,
            planned_additions: &additions,
            final_owners: &final_owners,
            explicit_removal_closure: &BTreeSet::new(),
        });
        assert!(
            result.is_err(),
            "distinct planned additions cannot publish duplicate references"
        );
    }

    #[test]
    fn repair_fresh_compaction_before_matches_approved_literals() {
        let (before, _, _, _, _, _) = membership_compaction_fixture().unwrap();
        let mut mismatches = Vec::new();
        for (id, x, y, rotation) in [
            ("matrix/mirror-matrix/r0c1/diode", 96.0, -43.0, 120.0),
            ("matrix/mirror-matrix/r1c1/diode", 76.0, -43.0, 120.0),
            ("matrix/mirror-matrix/r0c2/diode", 96.0, -57.0, 60.0),
            ("matrix/mirror-matrix/r1c2/diode", 76.0, -57.0, 60.0),
            ("matrix/mirror-matrix/r0c3/diode", 98.0, -66.0, 75.0),
            ("matrix/mirror-matrix/r1c3/diode", 78.0, -66.0, 75.0),
        ] {
            let actual = before.parts.iter().find(|p| p.id == id).unwrap().pose;
            if (actual.at.x - x).abs() > 1e-9
                || (actual.at.y - y).abs() > 1e-9
                || (actual.rotation - rotation).abs() > 1e-9
            {
                mismatches.push((
                    id,
                    actual,
                    Pose2 {
                        at: Vec2 { x, y },
                        rotation,
                    },
                ));
            }
        }
        assert!(
            mismatches.is_empty(),
            "the accepted compaction before fixture must materialize B/C/D according to their existing raw settings: {mismatches:?}"
        );
    }

    #[test]
    fn repair_fresh_chain_has_nonempty_both_local_child_dependencies() {
        let _assert_pose = |document: &ProjectDoc, id: &str, expected: Pose2| {
            let actual = document
                .parts
                .iter()
                .find(|part| part.id == id)
                .unwrap()
                .pose;
            assert!(
                (actual.at.x - expected.at.x).abs() < 1e-9,
                "{id}: {actual:?}"
            );
            assert!(
                (actual.at.y - expected.at.y).abs() < 1e-9,
                "{id}: {actual:?}"
            );
            assert!(
                (actual.rotation - expected.rotation).abs() < 1e-9,
                "{id}: {actual:?}"
            );
        };
        let (mut before, _, mut context, _, _, _) =
            membership_compaction_fixture().expect("the linked matrix fixture is valid");
        crate::matrix::deletion::remove_axis(&mut before, "source-matrix", 3, false)
            .expect("the source fixture narrows to three columns");
        crate::matrix::deletion::remove_axis(&mut before, "mirror-matrix", 3, false)
            .expect("the mirror fixture narrows to three columns");
        let mut initial_source = before
            .matrices
            .iter()
            .find(|matrix| matrix.id == "source-matrix")
            .unwrap()
            .clone();
        initial_source.origin = Vec2 { x: 3.0, y: -2.0 };
        crate::matrix::set_matrix(&mut before, &initial_source)
            .expect("the initial source frame materializes at the approved 2x3 origin");
        let matrix_association = context
            .pair_right
            .correspondences
            .iter()
            .find(|association| association.kind == MirroredItemKind::Matrix)
            .unwrap();
        let projected_initial = project_matrix(
            before
                .matrices
                .iter()
                .find(|matrix| matrix.id == "source-matrix")
                .unwrap(),
            before
                .matrices
                .iter()
                .find(|matrix| matrix.id == "mirror-matrix")
                .unwrap(),
            matrix_association,
            context.mapping,
        )
        .expect("the initial mirror frame is derived from the source frame");
        let mut initial_mirror = projected_initial;
        for (row, column, x, y) in [(0, 1, -93.0, 62.0), (1, 1, -113.0, 42.0)] {
            let cell = initial_mirror
                .cells
                .iter_mut()
                .find(|cell| cell.row == row && cell.column == column)
                .unwrap();
            cell.offset = Some(Vec2 { x, y });
            cell.rotation = Some(-90.0);
            let diode = cell
                .assemblies
                .iter_mut()
                .find(|assembly| assembly.id == "diode")
                .unwrap();
            diode.offset = Vec2 { x: 2.0, y: 3.0 };
            diode.rotation = Some(30.0);
            diode.side = Some(Side::Back);
        }
        crate::matrix::set_matrix(&mut before, &initial_mirror)
            .expect("both Local keys and diode children are materialized independently");
        let mirror_members_now = before
            .matrices
            .iter()
            .find(|matrix| matrix.id == "mirror-matrix")
            .unwrap()
            .part_ids
            .clone();
        before.nets.push(crate::model::Net {
            id: "net-clean".into(),
            name: "PROBE OMITTED LINKED".into(),
            pins: vec![
                crate::model::Pin {
                    part_id: "geometry-probe".into(),
                    pad_id: "pad-two".into(),
                },
                crate::model::Pin {
                    part_id: "matrix/mirror-matrix/r0c2/diode".into(),
                    pad_id: "pad-one".into(),
                },
                crate::model::Pin {
                    part_id: "matrix/mirror-matrix/r1c2/diode".into(),
                    pad_id: "pad-two".into(),
                },
            ],
        });
        before
            .boards
            .iter_mut()
            .find(|board| board.id == "mirror-board")
            .unwrap()
            .net_ids
            .push("net-clean".into());
        before.constraints.push(crate::model::Constraint::Offset {
            id: "constraint-omit".into(),
            source_part_id: "geometry-probe".into(),
            target_part_id: "matrix/mirror-matrix/r0c2/diode".into(),
            offset: Vec2 { x: 95.0, y: -57.0 },
            rotation: 60.0,
        });
        let mut outline_members: Vec<_> = mirror_members_now.into_iter().collect();
        outline_members.push("geometry-probe".into());
        before
            .outline
            .push(crate::model::OutlineFeature::PartEnvelope {
                id: "outline-mirror".into(),
                connections: vec![
                    OutlineConnection {
                        id: "local-r0-connection".into(),
                        width: 1.25,
                        points: vec![OutlineControlPoint {
                            at: Vec2 { x: 2.0, y: 3.0 },
                            part_id: Some("matrix/mirror-matrix/r0c1/diode".into()),
                        }],
                    },
                    OutlineConnection {
                        id: "local-r1-connection".into(),
                        width: 1.5,
                        points: vec![OutlineControlPoint {
                            at: Vec2 { x: 2.0, y: 3.0 },
                            part_id: Some("matrix/mirror-matrix/r1c1/diode".into()),
                        }],
                    },
                ],
                settings: Default::default(),
                part_ids: outline_members,
                margin: 1.75,
                operation: crate::model::Operation::Add,
            });
        before
            .boards
            .iter_mut()
            .find(|board| board.id == "mirror-board")
            .unwrap()
            .outline_ids
            .push("outline-mirror".into());
        before.outline.retain(|feature| {
            !matches!(feature, crate::model::OutlineFeature::PartEnvelope { id, .. } if id == "retained-keys-envelope")
        });
        before
            .boards
            .iter_mut()
            .find(|board| board.id == "mirror-board")
            .unwrap()
            .outline_ids
            .retain(|id| id != "retained-keys-envelope");
        if let Some(crate::model::Constraint::Offset { offset, .. }) = before.constraints.iter_mut()
            .find(|constraint| matches!(constraint, crate::model::Constraint::Offset { id, .. } if id == "retained-second-key-offset"))
        {
            *offset = Vec2 { x: 36.0, y: 68.0 };
        }
        let source_members = before
            .matrices
            .iter()
            .find(|matrix| matrix.id == "source-matrix")
            .unwrap()
            .part_ids
            .clone();
        let mirror_members = before
            .matrices
            .iter()
            .find(|matrix| matrix.id == "mirror-matrix")
            .unwrap()
            .part_ids
            .clone();
        let halves = before.hardware.as_mut().unwrap().pairs[0]
            .halves
            .as_mut()
            .unwrap();
        halves.correspondences.retain(|association| {
            association.kind == MirroredItemKind::Matrix
                || source_members.contains(&association.source_id)
        });
        context.associations.retain(|association| {
            association.kind == MirroredItemKind::Matrix
                || source_members.contains(&association.source_id)
                    && mirror_members.contains(&association.mirror_id)
        });
        context
            .owners
            .retain(|id, _| before.parts.iter().any(|part| part.id == *id));
        context.pair_right = halves.clone();
        for (source_id, state) in [
            ("matrix/source-matrix/r0c1", TransitionState::Local),
            ("matrix/source-matrix/r0c1/diode", TransitionState::Local),
            ("matrix/source-matrix/r1c1", TransitionState::Local),
            ("matrix/source-matrix/r1c1/diode", TransitionState::Local),
            ("matrix/source-matrix/r0c2", TransitionState::Omitted),
            ("matrix/source-matrix/r0c2/diode", TransitionState::Omitted),
        ] {
            let association = context
                .associations
                .iter_mut()
                .find(|association| association.source_id == source_id)
                .expect("the three-column context contains each state");
            association.state = state;
            association.explicit_state = !source_id.ends_with("/diode");
        }
        assert!(crate::mechanical::halves::validate_document(&before).is_ok());

        for (id, part_id, pad_id) in [
            ("net-local-r0", "matrix/mirror-matrix/r0c1/diode", "pad-one"),
            ("net-local-r1", "matrix/mirror-matrix/r1c1/diode", "pad-two"),
        ] {
            before.nets.push(crate::model::Net {
                id: id.into(),
                name: format!("LOCAL DEPENDENCY {id}"),
                pins: vec![Pin {
                    part_id: part_id.into(),
                    pad_id: pad_id.into(),
                }],
            });
        }
        before
            .boards
            .iter_mut()
            .find(|board| board.id == "mirror-board")
            .unwrap()
            .net_ids
            .extend(["net-local-r0".into(), "net-local-r1".into()]);
        for (id, target_part_id, offset) in [
            (
                "constraint-local-r0",
                "matrix/mirror-matrix/r0c1/diode",
                Vec2 { x: 38.0, y: 51.0 },
            ),
            (
                "constraint-local-r1",
                "matrix/mirror-matrix/r1c1/diode",
                Vec2 { x: 38.0, y: 71.0 },
            ),
        ] {
            before.constraints.push(Constraint::Offset {
                id: id.into(),
                source_part_id: "geometry-probe".into(),
                target_part_id: target_part_id.into(),
                offset,
                rotation: 30.0,
            });
        }
        assert!(crate::mechanical::halves::validate_document(&before).is_ok());

        let mut missing = Vec::new();
        for id in [
            "matrix/mirror-matrix/r0c1/diode",
            "matrix/mirror-matrix/r1c1/diode",
        ] {
            if !before
                .nets
                .iter()
                .any(|n| n.pins.iter().any(|p| p.part_id == id))
            {
                missing.push(format!("{id}: net pin"));
            }
            if !before
                .constraints
                .iter()
                .any(|c| c.source() == id || c.target() == id)
            {
                missing.push(format!("{id}: constraint"));
            }
        }
        assert!(
            missing.is_empty(),
            "both Local diodes require independent nonempty dependency preservation proof: {missing:?}"
        );
        for (part_id, net_id, pad_id) in [
            ("matrix/mirror-matrix/r0c1/diode", "net-local-r0", "pad-one"),
            ("matrix/mirror-matrix/r1c1/diode", "net-local-r1", "pad-two"),
        ] {
            let net = before.nets.iter().find(|net| net.id == net_id).unwrap();
            assert_eq!(
                net.pins,
                vec![Pin {
                    part_id: part_id.into(),
                    pad_id: pad_id.into(),
                }],
                "each Local diode keeps its own nonempty actual-Pad dependency net"
            );
            assert!(
                before
                    .boards
                    .iter()
                    .find(|board| board.id == "mirror-board")
                    .unwrap()
                    .net_ids
                    .contains(&net_id.to_owned())
            );
        }
    }

    #[test]
    fn repair_fresh_final_diff_reports_real_module_and_circuit_ids() {
        let (mut before, _, _, _, _, _) = repair_fresh_coherent_compaction_fixture().unwrap();
        add_membership_cleanup_sentinels(&mut before);
        let mut after = before.clone();
        after
            .modules
            .iter_mut()
            .find(|m| m.id == "shifted-module")
            .unwrap()
            .connection
            .as_mut()
            .unwrap()
            .host_connector_part_id = "matrix/mirror-matrix/r0c1/diode".into();
        after
            .embedded_circuits
            .iter_mut()
            .find(|c| c.id == "closure-circuit")
            .unwrap()
            .part_ids = vec!["matrix/mirror-matrix/r0c1/diode".into()];
        let (ids, paths) = changed_document_fields(&before, &after).unwrap();
        assert!(
            ids.contains(&"shifted-module".into()) && ids.contains(&"closure-circuit".into()),
            "changed dependency objects with actual IDs need real IDs, while idless maps use paths: IDs={ids:?}, paths={paths:?}"
        );
    }

    #[test]
    fn repair_fresh_final_addition_copies_nondefault_source_parameters() {
        let (mut before, _, context, _, _, _) =
            repair_fresh_coherent_compaction_fixture().expect("the linked matrix fixture is valid");
        before
            .constraints
            .retain(|c| c.id() != "retained-second-key-offset");
        let mut post_source = before.clone();
        let source_matrix = before
            .matrices
            .iter()
            .find(|matrix| matrix.id == "source-matrix")
            .unwrap();
        let mut expanded = source_matrix.clone();
        expanded.columns = 5;
        for row in 0..2 {
            expanded.cells.push(MatrixCell {
                deleted: false,
                row,
                column: 4,
                enabled: true,
                definition_id: None,
                variant: None,
                offset: None,
                rotation: None,
                assemblies: vec![MatrixAssembly {
                    id: "diode".into(),
                    definition_id: "diode".into(),
                    offset: Vec2 { x: 2.0, y: 3.0 },
                    rotation: Some(30.0),
                    side: Some(Side::Back),
                }],
                assemblies_local: None,
            });
        }
        crate::matrix::set_matrix(&mut post_source, &expanded)
            .expect("the planned source additions materialize");

        let source_identity_map: BTreeMap<_, _> = source_matrix
            .part_ids
            .iter()
            .map(|id| (id.clone(), Some(id.clone())))
            .collect();
        let mirror_identity_map: BTreeMap<_, _> = before
            .matrices
            .iter()
            .find(|matrix| matrix.id == "mirror-matrix")
            .unwrap()
            .part_ids
            .iter()
            .map(|id| (id.clone(), Some(id.clone())))
            .collect();
        let mut additions = Vec::new();
        for row in 0..2 {
            let source_key = format!("matrix/source-matrix/r{row}c4");
            let mirror_key = format!("matrix/mirror-matrix/r{row}c4");
            for (source_id, mirror_id, parent_source_id, parent_mirror_id, mirror_reference) in [
                (
                    source_key.clone(),
                    mirror_key.clone(),
                    None,
                    None,
                    format!("EXP_M_R{row}C4"),
                ),
                (
                    format!("{source_key}/diode"),
                    format!("{mirror_key}/diode"),
                    Some(source_key.clone()),
                    Some(mirror_key.clone()),
                    format!("EXP_M_D_R{row}C4"),
                ),
            ] {
                let source_part = post_source
                    .parts
                    .iter()
                    .find(|part| part.id == source_id)
                    .unwrap();
                additions.push(PlannedAddition {
                    source_id,
                    mirror_id,
                    source_reference: source_part.reference.clone(),
                    mirror_reference,
                    parent_source_id,
                    parent_mirror_id,
                });
            }
        }
        let mut final_owners: BTreeMap<_, _> = post_source
            .parts
            .iter()
            .map(|part| {
                let board = post_source
                    .boards
                    .iter()
                    .find(|board| board.part_ids.contains(&part.id))
                    .unwrap();
                (part.id.clone(), FinalOwner::Board(board.id.clone()))
            })
            .collect();
        for addition in &additions {
            final_owners.insert(
                addition.mirror_id.clone(),
                FinalOwner::Board("mirror-board".into()),
            );
        }

        let source_id = additions[0].source_id.clone();
        let mirror_id = additions[0].mirror_id.clone();
        let source = post_source
            .parts
            .iter_mut()
            .find(|p| p.id == source_id)
            .unwrap();
        source.properties = Some(BTreeMap::from([(
            "new-source-label".into(),
            serde_json::json!("ADDED SOURCE LABEL"),
        )]));
        source.generator_parameters = Some(BTreeMap::from([(
            "new-source-parameter".into(),
            serde_json::json!(17),
        )]));
        source.outline = Some(crate::model::PartOutline {
            excluded: true,
            margin: Some(2.75),
            allow_body_overhang: true,
        });
        let source_snapshot = source.clone();
        assert!(!source_snapshot.properties.as_ref().unwrap().is_empty());
        assert!(
            !source_snapshot
                .generator_parameters
                .as_ref()
                .unwrap()
                .is_empty()
        );
        let output = prepare_membership_transition(MembershipTransitionInput {
            valid_before: &before,
            post_source: &post_source,
            context: &context,
            baseline_kind: BaselineKind::MaterializedCurrentSchema,
            source_identity_map: &source_identity_map,
            mirror_identity_map: &mirror_identity_map,
            planned_additions: &additions,
            final_owners: &final_owners,
            explicit_removal_closure: &BTreeSet::new(),
        })
        .unwrap();
        let mirror = output
            .draft
            .parts
            .iter()
            .find(|p| p.id == mirror_id)
            .unwrap();
        assert_eq!(
            (
                &mirror.properties,
                &mirror.generator_parameters,
                &mirror.outline,
            ),
            (
                &source_snapshot.properties,
                &source_snapshot.generator_parameters,
                &source_snapshot.outline,
            ),
            "new linked materialized Part must copy nondefault labels, parameters, and PartOutline"
        );
    }

    fn consolidated_fresh_identity_fixture() -> Result<CompactionFixture, String> {
        let (before, _, mut context, _, _, _) = repair_fresh_coherent_compaction_fixture()?;
        for association in &mut context.associations {
            if association.source_id == "matrix/source-matrix/r1c1" {
                association.state = TransitionState::Local;
                association.explicit_state = true;
            } else if association.source_id == "matrix/source-matrix/r1c1/diode" {
                association.state = TransitionState::Local;
                association.explicit_state = false;
            }
        }
        let source_map = before
            .matrices
            .iter()
            .find(|m| m.id == "source-matrix")
            .unwrap()
            .part_ids
            .iter()
            .map(|id| (id.clone(), Some(id.clone())))
            .collect();
        let owners = context.owners.clone();
        Ok((
            before.clone(),
            before,
            context,
            source_map,
            owners,
            BTreeSet::new(),
        ))
    }

    #[test]
    fn consolidated_fresh_valid_identity_control() {
        let (before, _, context, source_map, owners, _) =
            consolidated_fresh_identity_fixture().unwrap();

        let mirror_map: BTreeMap<_, _> = before
            .matrices
            .iter()
            .find(|m| m.id == "mirror-matrix")
            .unwrap()
            .part_ids
            .iter()
            .map(|id| (id.clone(), Some(id.clone())))
            .collect();

        let snapshot = before.clone();
        let context_snapshot = context.clone();
        let result = prepare_membership_transition(MembershipTransitionInput {
            valid_before: &before,
            post_source: &before,
            context: &context,
            baseline_kind: BaselineKind::MaterializedCurrentSchema,
            source_identity_map: &source_map,
            mirror_identity_map: &mirror_map,
            planned_additions: &[],
            final_owners: &owners,
            explicit_removal_closure: &BTreeSet::new(),
        });
        assert_eq!(before, snapshot, "caller document remains unchanged");
        assert_eq!(
            context, context_snapshot,
            "caller context remains unchanged"
        );

        let output = result.expect("coherent accepted identity fixture");
        assert_eq!(output.draft, before);
        assert_eq!(output.context, context);
        assert!(output.changed_document_ids.is_empty());
        assert!(output.changed_field_paths.is_empty());
    }

    #[test]
    fn consolidated_fresh_refuses_missing_historical_constraint_endpoint() {
        let (mut before, _, context, source_map, owners, _) =
            consolidated_fresh_identity_fixture().unwrap();
        let Constraint::Offset { target_part_id, .. } = before
            .constraints
            .iter_mut()
            .find(|c| c.id() == "retained-second-key-offset")
            .unwrap()
        else {
            unreachable!()
        };
        *target_part_id = "missing-historical-endpoint".into();
        assert!(
            !before
                .parts
                .iter()
                .any(|p| p.id == "missing-historical-endpoint")
        );
        assert!(crate::mechanical::halves::validate_document(&before).is_ok());
        for matrix in &before.matrices {
            crate::matrix::valid_matrix(matrix, &before).unwrap();
        }

        let mirror_map: BTreeMap<_, _> = before
            .matrices
            .iter()
            .find(|m| m.id == "mirror-matrix")
            .unwrap()
            .part_ids
            .iter()
            .map(|id| (id.clone(), Some(id.clone())))
            .collect();

        let snapshot = before.clone();
        let context_snapshot = context.clone();
        let result = prepare_membership_transition(MembershipTransitionInput {
            valid_before: &before,
            post_source: &before,
            context: &context,
            baseline_kind: BaselineKind::MaterializedCurrentSchema,
            source_identity_map: &source_map,
            mirror_identity_map: &mirror_map,
            planned_additions: &[],
            final_owners: &owners,
            explicit_removal_closure: &BTreeSet::new(),
        });
        assert_eq!(before, snapshot, "caller document remains unchanged");
        assert_eq!(
            context, context_snapshot,
            "caller context remains unchanged"
        );

        assert!(
            result.is_err(),
            "valid-before touched constraint endpoints must exist before identity success"
        );
    }

    #[test]
    fn consolidated_fresh_refuses_missing_actual_child_parent_context() {
        let (before, _, mut context, source_map, owners, _) =
            consolidated_fresh_identity_fixture().unwrap();
        let child = context
            .associations
            .iter_mut()
            .find(|a| a.source_id == "matrix/source-matrix/r1c3/diode")
            .unwrap();
        assert_eq!(
            child.parent_source_id.as_deref(),
            Some("matrix/source-matrix/r1c3")
        );
        assert_eq!(
            child.parent_mirror_id.as_deref(),
            Some("matrix/mirror-matrix/r1c3")
        );
        child.parent_source_id = None;
        child.parent_mirror_id = None;

        let mirror_map: BTreeMap<_, _> = before
            .matrices
            .iter()
            .find(|m| m.id == "mirror-matrix")
            .unwrap()
            .part_ids
            .iter()
            .map(|id| (id.clone(), Some(id.clone())))
            .collect();

        let snapshot = before.clone();
        let context_snapshot = context.clone();
        let result = prepare_membership_transition(MembershipTransitionInput {
            valid_before: &before,
            post_source: &before,
            context: &context,
            baseline_kind: BaselineKind::MaterializedCurrentSchema,
            source_identity_map: &source_map,
            mirror_identity_map: &mirror_map,
            planned_additions: &[],
            final_owners: &owners,
            explicit_removal_closure: &BTreeSet::new(),
        });
        assert_eq!(before, snapshot, "caller document remains unchanged");
        assert_eq!(
            context, context_snapshot,
            "caller context remains unchanged"
        );

        assert!(
            result.is_err(),
            "a canonical historical child cannot lose its actual parent closure"
        );
    }

    #[test]
    fn consolidated_fresh_refuses_phantom_historical_association() {
        let (before, _, mut context, source_map, owners, _) =
            consolidated_fresh_identity_fixture().unwrap();
        let mut phantom = context
            .associations
            .iter()
            .find(|a| a.source_id == "matrix/source-matrix/r0c0")
            .unwrap()
            .clone();
        phantom.source_id = "matrix/source-matrix/r99c99".into();
        phantom.mirror_id = "matrix/mirror-matrix/r99c99".into();
        assert!(
            !before
                .parts
                .iter()
                .any(|p| p.id == phantom.source_id || p.id == phantom.mirror_id)
        );
        context.associations.push(phantom);

        let mirror_map: BTreeMap<_, _> = before
            .matrices
            .iter()
            .find(|m| m.id == "mirror-matrix")
            .unwrap()
            .part_ids
            .iter()
            .map(|id| (id.clone(), Some(id.clone())))
            .collect();

        let snapshot = before.clone();
        let context_snapshot = context.clone();
        let result = prepare_membership_transition(MembershipTransitionInput {
            valid_before: &before,
            post_source: &before,
            context: &context,
            baseline_kind: BaselineKind::MaterializedCurrentSchema,
            source_identity_map: &source_map,
            mirror_identity_map: &mirror_map,
            planned_additions: &[],
            final_owners: &owners,
            explicit_removal_closure: &BTreeSet::new(),
        });
        assert_eq!(before, snapshot, "caller document remains unchanged");
        assert_eq!(
            context, context_snapshot,
            "caller context remains unchanged"
        );

        assert!(
            result.is_err(),
            "historical association identities must bind actual members/owners, including extra records"
        );
    }

    #[test]
    fn consolidated_fresh_refuses_duplicate_same_board_registration() {
        let (mut before, _, context, source_map, owners, _) =
            consolidated_fresh_identity_fixture().unwrap();
        let id = "matrix/mirror-matrix/r1c1/diode";
        assert_eq!(
            before
                .boards
                .iter()
                .flat_map(|b| &b.part_ids)
                .filter(|p| p.as_str() == id)
                .count(),
            1
        );
        before
            .boards
            .iter_mut()
            .find(|b| b.id == "mirror-board")
            .unwrap()
            .part_ids
            .push(id.into());
        assert_eq!(
            before
                .boards
                .iter()
                .flat_map(|b| &b.part_ids)
                .filter(|p| p.as_str() == id)
                .count(),
            2
        );

        let mirror_map: BTreeMap<_, _> = before
            .matrices
            .iter()
            .find(|m| m.id == "mirror-matrix")
            .unwrap()
            .part_ids
            .iter()
            .map(|id| (id.clone(), Some(id.clone())))
            .collect();

        let snapshot = before.clone();
        let context_snapshot = context.clone();
        let result = prepare_membership_transition(MembershipTransitionInput {
            valid_before: &before,
            post_source: &before,
            context: &context,
            baseline_kind: BaselineKind::MaterializedCurrentSchema,
            source_identity_map: &source_map,
            mirror_identity_map: &mirror_map,
            planned_additions: &[],
            final_owners: &owners,
            explicit_removal_closure: &BTreeSet::new(),
        });
        assert_eq!(before, snapshot, "caller document remains unchanged");
        assert_eq!(
            context, context_snapshot,
            "caller context remains unchanged"
        );

        assert!(
            result.is_err(),
            "Board final owner must have exactly one actual registration, even within one board"
        );
    }

    #[test]
    fn consolidated_fresh_refuses_nonfinite_historical_local_pose() {
        let (mut before, _, context, source_map, owners, _) =
            consolidated_fresh_identity_fixture().unwrap();
        let part = before
            .parts
            .iter_mut()
            .find(|p| p.id == "matrix/mirror-matrix/r1c1/diode")
            .unwrap();
        part.pose.at.x = f64::INFINITY;
        assert!(!part.pose.at.x.is_finite());

        let mirror_map: BTreeMap<_, _> = before
            .matrices
            .iter()
            .find(|m| m.id == "mirror-matrix")
            .unwrap()
            .part_ids
            .iter()
            .map(|id| (id.clone(), Some(id.clone())))
            .collect();

        let snapshot = before.clone();
        let context_snapshot = context.clone();
        let result = prepare_membership_transition(MembershipTransitionInput {
            valid_before: &before,
            post_source: &before,
            context: &context,
            baseline_kind: BaselineKind::MaterializedCurrentSchema,
            source_identity_map: &source_map,
            mirror_identity_map: &mirror_map,
            planned_additions: &[],
            final_owners: &owners,
            explicit_removal_closure: &BTreeSet::new(),
        });
        assert_eq!(before, snapshot, "caller document remains unchanged");
        assert_eq!(
            context, context_snapshot,
            "caller context remains unchanged"
        );

        assert!(
            result.is_err(),
            "historical retained Local geometry must be finite before every success"
        );
    }

    #[test]
    fn consolidated_fresh_refuses_nonfinite_historical_constraint() {
        let (mut before, _, context, source_map, owners, _) =
            consolidated_fresh_identity_fixture().unwrap();
        let Constraint::Offset { offset, .. } = before
            .constraints
            .iter_mut()
            .find(|c| c.id() == "retained-second-key-offset")
            .unwrap()
        else {
            unreachable!()
        };
        offset.x = f64::INFINITY;

        let mirror_map: BTreeMap<_, _> = before
            .matrices
            .iter()
            .find(|m| m.id == "mirror-matrix")
            .unwrap()
            .part_ids
            .iter()
            .map(|id| (id.clone(), Some(id.clone())))
            .collect();

        let snapshot = before.clone();
        let context_snapshot = context.clone();
        let result = prepare_membership_transition(MembershipTransitionInput {
            valid_before: &before,
            post_source: &before,
            context: &context,
            baseline_kind: BaselineKind::MaterializedCurrentSchema,
            source_identity_map: &source_map,
            mirror_identity_map: &mirror_map,
            planned_additions: &[],
            final_owners: &owners,
            explicit_removal_closure: &BTreeSet::new(),
        });
        assert_eq!(before, snapshot, "caller document remains unchanged");
        assert_eq!(
            context, context_snapshot,
            "caller context remains unchanged"
        );

        assert!(
            result.is_err(),
            "historical touched constraint geometry must be finite before every success"
        );
    }

    /// The finite check is one `Constraint::is_finite`, so every kind a transition can meet
    /// must be refused by it with the message that names the constraint.
    #[test]
    fn consolidated_fresh_names_a_nonfinite_constraint_of_every_kind() {
        for kind in ["offset", "part-offset", "mirror"] {
            for nonfinite in [f64::INFINITY, f64::NAN] {
                let (mut before, _, context, source_map, owners, _) =
                    consolidated_fresh_identity_fixture().unwrap();
                let existing = before
                    .constraints
                    .iter_mut()
                    .find(|c| c.id() == "retained-second-key-offset")
                    .unwrap();
                let (id, source_part_id, target_part_id) = (
                    existing.id().to_owned(),
                    existing.source().to_owned(),
                    existing.target().to_owned(),
                );
                *existing = match kind {
                    "offset" => Constraint::Offset {
                        id,
                        source_part_id,
                        target_part_id,
                        offset: Vec2 {
                            x: nonfinite,
                            y: 1.0,
                        },
                        rotation: 0.0,
                    },
                    "part-offset" => Constraint::PartOffset {
                        id,
                        source_part_id,
                        target_part_id,
                        offset: Vec2 { x: 1.0, y: 1.0 },
                        rotation: nonfinite,
                    },
                    _ => Constraint::Mirror {
                        id,
                        source_part_id,
                        target_part_id,
                        axis: MirrorAxis::Vertical,
                        coordinate: nonfinite,
                    },
                };
                let mirror_map: BTreeMap<_, _> = before
                    .matrices
                    .iter()
                    .find(|m| m.id == "mirror-matrix")
                    .unwrap()
                    .part_ids
                    .iter()
                    .map(|id| (id.clone(), Some(id.clone())))
                    .collect();
                let message = prepare_membership_transition(MembershipTransitionInput {
                    valid_before: &before,
                    post_source: &before,
                    context: &context,
                    baseline_kind: BaselineKind::MaterializedCurrentSchema,
                    source_identity_map: &source_map,
                    mirror_identity_map: &mirror_map,
                    planned_additions: &[],
                    final_owners: &owners,
                    explicit_removal_closure: &BTreeSet::new(),
                })
                .map(|_| ())
                .unwrap_err();
                assert!(
                    message.contains("retained-second-key-offset")
                        && message.contains("non-finite geometry"),
                    "{kind} {nonfinite}: {message}"
                );
            }
        }
    }

    #[test]
    fn consolidated_fresh_refuses_local_mirror_deletion_on_identity() {
        let (before, _, context, source_map, owners, _) =
            consolidated_fresh_identity_fixture().unwrap();

        let mut mirror_map: BTreeMap<_, _> = before
            .matrices
            .iter()
            .find(|m| m.id == "mirror-matrix")
            .unwrap()
            .part_ids
            .iter()
            .map(|id| (id.clone(), Some(id.clone())))
            .collect();

        mirror_map.insert("matrix/mirror-matrix/r1c1/diode".into(), None);

        let snapshot = before.clone();
        let context_snapshot = context.clone();
        let result = prepare_membership_transition(MembershipTransitionInput {
            valid_before: &before,
            post_source: &before,
            context: &context,
            baseline_kind: BaselineKind::MaterializedCurrentSchema,
            source_identity_map: &source_map,
            mirror_identity_map: &mirror_map,
            planned_additions: &[],
            final_owners: &owners,
            explicit_removal_closure: &BTreeSet::new(),
        });
        assert_eq!(before, snapshot, "caller document remains unchanged");
        assert_eq!(
            context, context_snapshot,
            "caller context remains unchanged"
        );

        assert!(
            result.is_err(),
            "identity success cannot silently ignore a Local child's None mirror mapping"
        );
    }

    #[test]
    fn consolidated_fresh_refuses_materialized_omitted_child_on_chained_identity() {
        let (before, post, context, source_map, owners, closure) =
            repair_fresh_coherent_compaction_fixture().unwrap();
        let mirror_map = source_map
            .iter()
            .map(|(old, new)| {
                (
                    old.replace("matrix/source-matrix/", "matrix/mirror-matrix/"),
                    new.as_ref()
                        .map(|id| id.replace("matrix/source-matrix/", "matrix/mirror-matrix/")),
                )
            })
            .collect();
        let output = prepare_membership_transition(MembershipTransitionInput {
            valid_before: &before,
            post_source: &post,
            context: &context,
            baseline_kind: BaselineKind::MaterializedCurrentSchema,
            source_identity_map: &source_map,
            mirror_identity_map: &mirror_map,
            planned_additions: &[],
            final_owners: &owners,
            explicit_removal_closure: &closure,
        })
        .expect("valid preceding compaction and private context control");
        let before = output.draft;
        let mut context = output.context;
        let owners = output.final_owners;
        let source_map = before
            .matrices
            .iter()
            .find(|m| m.id == "source-matrix")
            .unwrap()
            .part_ids
            .iter()
            .map(|id| (id.clone(), Some(id.clone())))
            .collect();

        let mut mirror_map: BTreeMap<_, _> = before
            .matrices
            .iter()
            .find(|m| m.id == "mirror-matrix")
            .unwrap()
            .part_ids
            .iter()
            .map(|id| (id.clone(), Some(id.clone())))
            .collect();

        let child = context
            .associations
            .iter_mut()
            .find(|a| a.source_id == "matrix/source-matrix/r1c2/diode")
            .unwrap();
        child.state = TransitionState::Omitted;
        child.explicit_state = true;
        let id = child.mirror_id.clone();
        assert!(before.parts.iter().any(|p| p.id == id));
        assert_eq!(
            owners.get(&id),
            Some(&FinalOwner::Board("mirror-board".into()))
        );
        mirror_map.insert(id.clone(), None);
        let snapshot = before.clone();
        let context_snapshot = context.clone();
        let result = prepare_membership_transition(MembershipTransitionInput {
            valid_before: &before,
            post_source: &before,
            context: &context,
            baseline_kind: BaselineKind::ChainedPrivateDraft,
            source_identity_map: &source_map,
            mirror_identity_map: &mirror_map,
            planned_additions: &[],
            final_owners: &owners,
            explicit_removal_closure: &BTreeSet::new(),
        });
        assert_eq!(before, snapshot);
        assert_eq!(context, context_snapshot);
        assert!(
            result.is_err(),
            "chained Omitted state must bind absent geometry/owner before no-op success"
        );
    }

    #[test]
    fn durable_local_exception_is_atomic_and_repeats_through_parent_kernel() {
        let (mut document, projection) = fixed_geometry_fixture().unwrap();
        let mirror_id = projection
            .locally_protected_mirror_parts
            .iter()
            .next()
            .unwrap()
            .clone();
        let selected = PartProjection {
            source_id: mirror_id.replace("mirror-matrix", "source-matrix"),
            mirror_id,
        };
        let other_retained = projection
            .locally_protected_mirror_parts
            .iter()
            .find(|id| **id != selected.mirror_id)
            .unwrap()
            .clone();
        let target = document
            .parts
            .iter_mut()
            .find(|part| part.id == selected.mirror_id)
            .unwrap();
        target.reference = "LOCAL-OWNER".into();
        target.pose = Pose2 {
            at: Vec2 { x: -23.5, y: 41.25 },
            rotation: 117.0,
        };
        target.properties.get_or_insert_with(BTreeMap::new).insert(
            "localOwnerPayload".into(),
            serde_json::json!({"revision": 9}),
        );
        let expected_local = target.clone();

        let halves = document.hardware.as_mut().unwrap().pairs[0]
            .halves
            .as_mut()
            .unwrap();
        for mirror_id in [&selected.mirror_id, &other_retained] {
            halves.exceptions.push(MirroredException {
                companion_mode: None,
                companion_tweak: None,
                kind: MirroredItemKind::Part,
                mirror_id: mirror_id.clone(),
                state: MirroredExceptionState::Local,
                removed_copper: None,
            });
        }

        // This is a positive parent-kernel control: the selected Local owner is already present
        // in the validated retained-member partition. The child transition regressions separately
        // prove construction of that partition from the durable source/mirror exception.
        crate::mechanical::halves::validate_document(&document)
            .expect("the durable Local exception and its matrix item are valid");
        let serialized = serde_json::to_vec(&document).unwrap();
        document = serde_json::from_slice(&serialized).unwrap();
        crate::mechanical::halves::validate_document(&document)
            .expect("the Local association and visible geometry survive document round-trip");
        let pair_before = pair_state_bytes(&document);
        let independent_context_before = projection.applied_independent_additions.borrow().clone();
        let hosted_context_before = projection.applied_hosted_operations.borrow().clone();
        let circuit_context_before = projection.applied_hosted_circuit_additions.borrow().clone();
        let hosted_removal_before = projection.hosted_removal_baselines.borrow().clone();
        let hosted_omission_before = projection.hosted_omission_baselines.borrow().clone();
        let outline_removal_before = projection.outline_removal_baselines.borrow().clone();

        reconcile(&mut document, "pair-source-mirror", &projection)
            .expect("the exact durable Local owner state reconciles successfully");
        let first_output = document.clone();
        assert_eq!(
            document
                .parts
                .iter()
                .find(|part| part.id == selected.mirror_id),
            Some(&expected_local),
            "Local owns the complete target payload while the linked matrix path runs"
        );
        assert_eq!(pair_state_bytes(&document), pair_before);
        let repeated = reconcile(&mut document, "pair-source-mirror", &projection)
            .expect("the same complete owner state is repeatable");
        assert!(
            repeated.is_empty(),
            "repeat reconciliation reports no changed IDs"
        );
        assert_eq!(document, first_output);
        assert_eq!(pair_state_bytes(&document), pair_before);
        assert_eq!(
            *projection.applied_independent_additions.borrow(),
            independent_context_before
        );
        assert_eq!(
            *projection.applied_hosted_operations.borrow(),
            hosted_context_before
        );
        assert_eq!(
            *projection.applied_hosted_circuit_additions.borrow(),
            circuit_context_before
        );
        assert_eq!(
            *projection.hosted_removal_baselines.borrow(),
            hosted_removal_before
        );
        assert_eq!(
            *projection.hosted_omission_baselines.borrow(),
            hosted_omission_before
        );
        assert_eq!(
            *projection.outline_removal_baselines.borrow(),
            outline_removal_before
        );

        let source_matrix_before = document
            .matrices
            .iter()
            .find(|matrix| matrix.id == "source-matrix")
            .unwrap()
            .clone();
        let linked_before: BTreeMap<_, _> = document
            .parts
            .iter()
            .filter(|part| {
                part.id.starts_with("matrix/mirror-matrix/")
                    && part.id != selected.mirror_id
                    && part.id != other_retained
            })
            .map(|part| (part.id.clone(), part.pose))
            .collect();
        let local_matrix_cell = selected
            .source_id
            .strip_prefix("matrix/source-matrix/")
            .unwrap();
        let (local_row, local_column) = local_matrix_cell.split_once('c').unwrap();
        let local_row = local_row.strip_prefix('r').unwrap().parse::<u32>().unwrap();
        let local_column = local_column.parse::<u32>().unwrap();
        let local_slot_before = source_matrix_before
            .cells
            .iter()
            .find(|cell| cell.row == local_row && cell.column == local_column)
            .unwrap();
        assert!(local_slot_before.enabled);

        let mut edited_source_matrix = source_matrix_before;
        edited_source_matrix.origin.x += 7.0;
        crate::matrix::set_matrix(&mut document, &edited_source_matrix)
            .expect("an ordinary Core matrix edit materializes before reconciliation");
        reconcile(&mut document, "pair-source-mirror", &projection)
            .expect("the Local exception remains valid during a linked source matrix edit");
        assert_eq!(
            document
                .parts
                .iter()
                .find(|part| part.id == selected.mirror_id),
            Some(&expected_local),
            "the Local target keeps its visible geometry after the ordinary source edit"
        );
        assert!(
            linked_before.iter().any(|(id, pose)| {
                document
                    .parts
                    .iter()
                    .find(|part| part.id == *id)
                    .is_some_and(|part| part.pose != *pose)
            }),
            "unselected linked matrix members still follow the ordinary source edit"
        );

        let mut invalid_projection = projection.clone();
        invalid_projection.final_owners.remove(&selected.mirror_id);
        let mut invalid_document = first_output.clone();
        let invalid_document_before = invalid_document.clone();
        let invalid_context_before = invalid_projection
            .applied_hosted_operations
            .borrow()
            .clone();
        assert!(
            reconcile(
                &mut invalid_document,
                "pair-source-mirror",
                &invalid_projection
            )
            .is_err(),
            "an incomplete owner partition is refused"
        );
        assert_eq!(invalid_document, invalid_document_before);
        assert_eq!(
            *invalid_projection.applied_hosted_operations.borrow(),
            invalid_context_before
        );
    }

    #[test]
    fn durable_omitted_independent_part_stays_absent_through_parent_kernel() {
        let (mut document, projection) = fixed_geometry_fixture().unwrap();
        let mirror_id = "matrix/mirror-matrix/r0c1";
        document
            .boards
            .iter_mut()
            .find(|board| board.id == "mirror-board")
            .unwrap()
            .part_ids
            .retain(|id| id != mirror_id);

        let halves = document.hardware.as_mut().unwrap().pairs[0]
            .halves
            .as_mut()
            .unwrap();
        document.parts.retain(|part| part.id != mirror_id);
        halves.exceptions.push(MirroredException {
            companion_mode: None,
            companion_tweak: None,
            kind: MirroredItemKind::Part,
            mirror_id: mirror_id.into(),
            state: MirroredExceptionState::Omitted,
            removed_copper: None,
        });
        crate::mechanical::halves::validate_document(&document)
            .expect("the source-bound Omitted target may be physically absent");
        let serialized = serde_json::to_vec(&document).unwrap();
        document = serde_json::from_slice(&serialized).unwrap();
        crate::mechanical::halves::validate_document(&document)
            .expect("the source-bound Omitted absence survives document round-trip");
        let pair_before = pair_state_bytes(&document);
        let linked_before: BTreeMap<_, _> = document
            .parts
            .iter()
            .filter(|part| part.id.starts_with("matrix/mirror-matrix/") && part.id != mirror_id)
            .map(|part| (part.id.clone(), part.pose))
            .collect();
        let mut source_matrix = document
            .matrices
            .iter()
            .find(|matrix| matrix.id == "source-matrix")
            .unwrap()
            .clone();
        source_matrix.origin.x += 5.0;
        crate::matrix::set_matrix(&mut document, &source_matrix)
            .expect("an ordinary Core matrix edit changes the omitted item's source");

        reconcile(&mut document, "pair-source-mirror", &projection)
            .expect("a repeated reconciliation leaves the selected Omitted item absent");
        assert!(document.parts.iter().all(|part| part.id != mirror_id));
        assert!(
            document
                .boards
                .iter()
                .all(|board| !board.part_ids.iter().any(|id| id == mirror_id))
        );
        assert!(
            linked_before.iter().any(|(id, pose)| {
                document
                    .parts
                    .iter()
                    .find(|part| part.id == *id)
                    .is_some_and(|part| part.pose != *pose)
            }),
            "linked matrix members continue to follow the ordinary source edit"
        );
        let first_output = document.clone();
        assert!(
            reconcile(&mut document, "pair-source-mirror", &projection)
                .unwrap()
                .is_empty()
        );
        assert_eq!(document, first_output);
        assert_eq!(pair_state_bytes(&document), pair_before);

        let mut invalid_projection = projection.clone();
        invalid_projection
            .final_owners
            .insert(mirror_id.into(), "source-board".into());
        let mut invalid_document = document.clone();
        assert!(
            reconcile(
                &mut invalid_document,
                "pair-source-mirror",
                &invalid_projection
            )
            .is_err(),
            "Omitted absence does not waive the exact final board owner"
        );
        assert_eq!(invalid_document, document);
    }
}
