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
//! - `mod tests` in reconcile/tests.rs; outline tests are included from
//!   reconcile/outlines/tests.rs.

use crate::model::{
    Matrix, MirroredCorrespondence, MirroredExceptionState, MirroredHalves, MirroredItemKind,
    OutlineFeature, Part, Pose2, ProjectDoc, Vec2,
};
use std::collections::{BTreeMap, BTreeSet};

#[path = "lifecycle.rs"]
mod lifecycle;
mod prepare;
#[cfg(test)]
pub(crate) use prepare::prepare_transaction;
pub(crate) use prepare::{prepare_open, prepare_transaction_with_ledger};
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
    // Ticket 107 records hosted Add provenance privately during reconciliation.
    applied_hosted_operations: std::cell::RefCell<BTreeSet<(String, String, String)>>,
    // Ticket 107 binds circuit Add replay to the exact selected maps and first output.
    applied_hosted_circuit_additions:
        std::cell::RefCell<BTreeMap<(String, String, String), HostedCircuitAddProvenance>>,
    // Ticket 107 keeps source-operation baselines private to the reconciliation kernel.
    hosted_removal_baselines: std::cell::RefCell<BTreeMap<(String, String, String), ProjectDoc>>,
    // Ticket 107 retains validated owner context for physically absent Omitted children.
    hosted_omission_baselines: std::cell::RefCell<BTreeMap<(String, String, String), ProjectDoc>>,
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

fn matrix_membership_changed(before: &ProjectDoc, after: &ProjectDoc, matrix_id: &str) -> bool {
    let before = before
        .matrices
        .iter()
        .find(|matrix| matrix.id == matrix_id)
        .map(|matrix| {
            matrix
                .part_ids
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>()
        });
    let after = after
        .matrices
        .iter()
        .find(|matrix| matrix.id == matrix_id)
        .map(|matrix| {
            matrix
                .part_ids
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>()
        });
    match (before, after) {
        (Some(mut before), Some(mut after)) => {
            before.sort_unstable();
            after.sort_unstable();
            before != after
        }
        (None, None) => false,
        (None, Some(_)) | (Some(_), None) => true,
    }
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

fn effective_circuit_child_state(
    document: &ProjectDoc,
    halves: &MirroredHalves,
    correspondence: &MirroredCorrespondence,
) -> Option<crate::model::MirroredExceptionState> {
    if let Some(exception) = halves.overlay_of(correspondence) {
        return Some(exception.state);
    }
    let child_id = correspondence.mirror_id.as_str();
    let owner = halves.correspondences.iter().find(|owner| {
        if owner.kind != MirroredItemKind::EmbeddedCircuit {
            return false;
        }
        let Some(circuit) = document
            .embedded_circuits
            .iter()
            .find(|circuit| circuit.id == owner.mirror_id)
        else {
            return false;
        };
        match correspondence.kind {
            MirroredItemKind::Part => circuit.part_ids.iter().any(|id| id == child_id),
            MirroredItemKind::Net => {
                child_id.starts_with(&format!("embedded/{}/", owner.mirror_id))
                    && circuit.net_ids.iter().any(|id| id == child_id)
            }
            _ => false,
        }
    })?;
    halves.overlay_of(owner).map(|exception| exception.state)
}

fn validate_local_circuit_targets(
    document: &ProjectDoc,
    halves: &MirroredHalves,
) -> Result<(), String> {
    for exception in halves.exceptions.iter().filter(|exception| {
        exception.kind == MirroredItemKind::EmbeddedCircuit
            && exception.state == crate::model::MirroredExceptionState::Local
    }) {
        let mut owners = halves.correspondences.iter().filter(|owner| {
            owner.kind == MirroredItemKind::EmbeddedCircuit
                && owner.mirror_id == exception.mirror_id
                && halves.overlay_of(owner).is_some_and(|overlay| {
                    overlay.state == crate::model::MirroredExceptionState::Local
                })
        });
        let Some(owner) = owners.next() else {
            // Historical unassociated Local circuits keep their existing unvalidated behavior.
            continue;
        };
        if owners.next().is_some() {
            return Err(format!(
                "Local embedded circuit {:?} has duplicate owner correspondences",
                exception.mirror_id
            ));
        }
        let source_matches = document
            .embedded_circuits
            .iter()
            .filter(|circuit| circuit.id == owner.source_id)
            .count();
        if source_matches != 1 {
            return Err(format!(
                "Local embedded circuit source {:?} is unavailable or ambiguous",
                owner.source_id
            ));
        }
        let mut targets = document
            .embedded_circuits
            .iter()
            .filter(|circuit| circuit.id == owner.mirror_id);
        let Some(target) = targets.next() else {
            return Err(format!(
                "Local embedded circuit target {:?} is unavailable",
                owner.mirror_id
            ));
        };
        if targets.next().is_some() || target.host_board_id != halves.mirror_board_id {
            return Err(format!(
                "Local embedded circuit target {:?} is ambiguous or belongs to the wrong board",
                owner.mirror_id
            ));
        }
        if document
            .module_definitions
            .iter()
            .filter(|definition| definition.id == target.definition_id)
            .count()
            != 1
        {
            return Err(format!(
                "Local embedded circuit {:?} has an unavailable module definition",
                owner.mirror_id
            ));
        }
        let member_prefix = format!("embedded/{}/", owner.mirror_id);
        let mut members = BTreeSet::new();
        for member_id in &target.part_ids {
            if !members.insert(member_id) || !member_id.starts_with(&member_prefix) {
                return Err(format!(
                    "Local embedded circuit {:?} has a duplicate or foreign member {member_id:?}",
                    owner.mirror_id
                ));
            }
            if document
                .parts
                .iter()
                .filter(|part| part.id == *member_id)
                .count()
                != 1
                || unique_board_owner(document, member_id, "Local circuit member")?
                    != halves.mirror_board_id
            {
                return Err(format!(
                    "Local embedded circuit member {member_id:?} is unavailable or has the wrong owner"
                ));
            }
            let Some(part) = document.parts.iter().find(|part| part.id == *member_id) else {
                return Err(format!(
                    "Local embedded circuit member {member_id:?} is unavailable"
                ));
            };
            let definition_is_foreign_embedded = part.definition_id.starts_with("embedded/")
                && !part
                    .definition_id
                    .starts_with(&format!("{member_prefix}definition/"));
            if definition_is_foreign_embedded
                || document
                    .definitions
                    .iter()
                    .filter(|definition| definition.id == part.definition_id)
                    .count()
                    != 1
            {
                return Err(format!(
                    "Local embedded circuit member {member_id:?} has an unavailable or foreign definition"
                ));
            }
        }
        let mut net_ids = BTreeSet::new();
        for net_id in &target.net_ids {
            if !net_ids.insert(net_id) {
                return Err(format!(
                    "Local embedded circuit {:?} has duplicate net reference {net_id:?}",
                    owner.mirror_id
                ));
            }
            if document.nets.iter().filter(|net| net.id == *net_id).count() != 1 {
                return Err(format!(
                    "Local embedded circuit net {net_id:?} is unavailable or ambiguous"
                ));
            }
            let mut net_owners = document
                .boards
                .iter()
                .filter(|board| board.net_ids.iter().any(|candidate| candidate == net_id));
            let Some(net_owner) = net_owners.next() else {
                return Err(format!(
                    "Local embedded circuit net {net_id:?} has no board owner"
                ));
            };
            if net_owners.next().is_some() || net_owner.id != halves.mirror_board_id {
                return Err(format!(
                    "Local embedded circuit net {net_id:?} is ambiguous or belongs to the wrong board"
                ));
            }
            let Some(net) = document.nets.iter().find(|net| net.id == *net_id) else {
                return Err(format!(
                    "Local embedded circuit net {net_id:?} is unavailable"
                ));
            };
            for pin in &net.pins {
                if document
                    .parts
                    .iter()
                    .filter(|part| part.id == pin.part_id)
                    .count()
                    != 1
                    || unique_board_owner(document, &pin.part_id, "Local circuit net pin")?
                        != halves.mirror_board_id
                {
                    return Err(format!(
                        "Local embedded circuit net {net_id:?} has a missing or foreign pin {:?}",
                        pin.part_id
                    ));
                }
            }
        }
        for (port, net_id) in &target.ports {
            if !net_ids.contains(net_id) {
                return Err(format!(
                    "Local embedded circuit port {port:?} references unregistered net {net_id:?}"
                ));
            }
        }
    }
    Ok(())
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
    retained_companion_parts: Vec<Part>,
    omitted_parts: BTreeSet<String>,
    omitted_matrix_companions: BTreeSet<String>,
}

struct OmittedMatrixCompanion {
    source_id: String,
    mirror_id: String,
    source_key_id: String,
    mirror_key_id: String,
}

fn exact_omitted_matrix_companions(
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

fn remove_omitted_matrix_parts(
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
        || projection
            .final_owners
            .keys()
            .any(|id| is_tombstoned_member(id))
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
    fn tombstone_omitted_cells(matrix: &mut crate::model::Matrix, omitted: &BTreeSet<(u32, u32)>) {
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
    let mut adjusted_projected = projected.clone();
    tombstone_omitted_cells(&mut adjusted_projected, &omitted_slot_ids);
    // Member accounting below reads the suppressed projection.
    let mut projected = adjusted_projected.clone();

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
        omitted_companions: &BTreeSet<String>,
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
    projected
        .part_ids
        .retain(|id| !omitted_mirror_companions.contains(id));
    let source_members = canonical_members(&source, &source_slots, &BTreeSet::new())?;
    let mirror_members = canonical_members(&mirror, &mirror_slots, &omitted_mirror_companions)?;
    canonical_members(&projected, &projected_slots, &omitted_mirror_companions)?;

    let mut linked_slots = std::collections::BTreeMap::<Slot, String>::new();
    let mut linked_source_ids = std::collections::BTreeSet::new();
    let mut linked_mirror_ids = std::collections::BTreeSet::new();
    let mut retained_companion_slots = BTreeSet::<(Slot, String)>::new();
    let mut retained_companion_parts = Vec::new();
    // Omitted slots leave pose reconciliation to the provenance check below.
    let mut omitted_part_projections = 0;
    for part in &projection.parts {
        let source_slot = source_slots
            .iter()
            .find_map(|(slot, id)| (id == &part.source_id).then_some(*slot));
        if source_slot.is_some_and(|slot| omitted_slot_ids.contains(&slot)) {
            omitted_part_projections += 1;
            continue;
        }
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
        || partition_slots != live_source_slots
        || projection.parts.len() != linked_slots.len() + omitted_part_projections
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
                let owner = projection.final_owners.get(id).map(String::as_str);
                let owner_ok = owner == Some(halves.mirror_board_id.as_str())
                    || (!materialized && owner.is_none());
                if !owner_ok {
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
                    || projection
                        .final_owners
                        .get(&source_companion_id)
                        .map(String::as_str)
                        != Some(halves.source_board_id.as_str())
                    || document
                        .parts
                        .iter()
                        .any(|part| part.id == mirror_companion_id)
                    || document
                        .boards
                        .iter()
                        .any(|board| board.part_ids.iter().any(|id| id == &mirror_companion_id))
                    || projection.final_owners.contains_key(&mirror_companion_id)
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
            if linked_slots.contains_key(slot)
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
    for matrix in [&source, &mirror, &adjusted_projected] {
        crate::matrix::valid_projection_matrix(matrix)?;
    }
    let evaluated = crate::matrix::evaluated(&adjusted_projected, document)?;
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
        let old_cell = adjusted_projected
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
            adjusted_projected.mirror,
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
        retained_companion_parts,
        omitted_parts,
        omitted_matrix_companions: omitted_mirror_companions,
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
                retained_companion_parts: Vec::new(),
                omitted_parts: BTreeSet::new(),
                omitted_matrix_companions: BTreeSet::new(),
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
            for retained in &view.retained_companion_parts {
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
            remove_omitted_matrix_parts(
                &mut staged,
                &mirror_matrix.id,
                &view.omitted_matrix_companions,
            );
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
    validate_local_circuit_targets(document, halves)?;
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
        // Members absent by rule (part-level Omitted keys and their companions) keep their
        // correspondence but leave the staged mirror membership: like overlaid items above,
        // they are never projected over, so they cannot cross the membership boundary.
        if omitted_parts.contains(&correspondence.mirror_id) {
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
        let projected = project_part(
            source,
            mirror,
            correspondence,
            pair.mapping,
            super::derive::is_controller(&document.definitions, &source.definition_id),
        )?;
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
        let projected = project_part(
            source,
            &target_seed,
            &correspondence,
            pair.mapping,
            super::derive::is_controller(&document.definitions, &source.definition_id),
        )?;
        if let Some(staged_target) = staged.parts.iter_mut().find(|part| part.id == *mirror_id) {
            *staged_target = projected;
        } else {
            staged.parts.push(projected);
            let target_board = staged
                .boards
                .iter_mut()
                .find(|board| board.id == halves.mirror_board_id)
                .ok_or_else(|| "The independent target board is unavailable".to_owned())?;
            target_board.part_ids.push(mirror_id.clone());
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
        effective_circuit_child_state(document, halves, correspondence)
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
        .map(|association| {
            (
                association.source_id.clone(),
                association.mirror_id.clone(),
                halves.overlay_of(association).is_some_and(|overlay| {
                    overlay.state == crate::model::MirroredExceptionState::Local
                }),
            )
        })
        .collect();
    for (source_id, mirror_id, local_owner) in circuit_pairs {
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
        // A Local owner keeps the full target record; independently corresponded joined nets
        // still flow through their own targets below.
        if !local_owner {
            *target = projected;
        }

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
            *target_part = project_part(
                source_part,
                target_part,
                &correspondence,
                pair.mapping,
                super::derive::is_controller(&document.definitions, &source_part.definition_id),
            )?;
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
                    super::derive::is_controller(&document.definitions, &source_part.definition_id),
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
                project_part(
                    source_part,
                    &seed,
                    &correspondence,
                    pair.mapping,
                    super::derive::is_controller(&document.definitions, &source_part.definition_id),
                )?
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
        // An item's function keys mention its part id as a whole path segment.
        let mentions = |key: &str, id: &str| {
            key == id
                || key.starts_with(&format!("{id}/"))
                || key.ends_with(&format!("/{id}"))
                || key.contains(&format!("/{id}/"))
        };
        // Global peripheral role keys are shared by every consumer of that role, unlike
        // `peripheral/<part-id>/...` keys. Only mirror consumers keep these mapped roles active
        // after their source consumer is Omitted.
        let mut source_global_roles = BTreeSet::new();
        for requirement in
            crate::electrical_peripherals::describe(document, &halves.source_board_id)
        {
            for (_, function) in requirement.gpio_terminals {
                if !mentions(&function, &requirement.part_id) {
                    source_global_roles.insert(function);
                }
            }
        }
        let mut active_mirror_global_roles = BTreeSet::new();
        for requirement in crate::electrical_peripherals::describe(&staged, &halves.mirror_board_id)
        {
            for (_, function) in requirement.gpio_terminals {
                if !mentions(&function, &requirement.part_id) {
                    active_mirror_global_roles.insert(function);
                }
            }
        }
        let target = staged.hardware.as_mut().and_then(|hardware| {
            hardware
                .boards
                .iter_mut()
                .find(|board| board.board_id == halves.mirror_board_id)
        });
        let mut controller_part_to_sync = None;
        if let (Some(source), Some(target)) = (source, target) {
            let local_bindings = target.key_bindings.clone();
            let local_protected_handoff = target.protected_handoff.clone();
            let mirror_assignments = target.assignments.clone();
            let mirror_locks = target.locks.clone();
            let mentions_omitted =
                |key: &str| omitted_part_sources.iter().any(|id| mentions(key, id));
            let inactive_global_role = |key: &str| {
                source_global_roles.contains(key) && !active_mirror_global_roles.contains(key)
            };
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
            projected.controller_part_id = source
                .controller_part_id
                .as_ref()
                .and_then(|id| reference_part_targets.get(id).cloned());
            let mut assignments = BTreeMap::new();
            let mut locks = BTreeMap::new();
            // Mirrored items stay on the source's pins, so each mapped assignment is also a
            // lock. An Omitted item's entries are dropped, which frees its pins. A global role
            // is projected only while some mirror peripheral still consumes it.
            for (key, value) in &source.assignments {
                if mentions_omitted(key) || inactive_global_role(key) {
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
                .chain(
                    locks
                        .iter()
                        .filter(|(key, _)| !inactive_global_role(key))
                        .map(|(_, value)| value),
                )
                .cloned()
                .collect();
            for (key, value) in &mirror_assignments {
                if !derived_keys.contains(key) && !held.contains(value) {
                    assignments.entry(key.clone()).or_insert(value.clone());
                }
            }
            for (key, value) in &mirror_locks {
                if !derived_keys.contains(key) && !held.contains(value) {
                    locks.entry(key.clone()).or_insert(value.clone());
                }
            }
            projected.assignments = assignments;
            projected.locks = locks;
            projected.key_bindings = local_bindings;
            projected.protected_handoff = local_protected_handoff;
            controller_part_to_sync = projected.controller_part_id.clone();
            *target = projected;
        }
        if let Some(controller_part_id) = controller_part_to_sync {
            let generated_net_prefix = format!("generated/electrical/{}/", halves.mirror_board_id);
            if staged
                .nets
                .iter()
                .any(|net| net.id.starts_with(&generated_net_prefix))
            {
                let controller_parts = BTreeSet::from([controller_part_id]);
                crate::electrical::sync_terminal_parameters_for_parts(
                    &mut staged,
                    &controller_parts,
                );
            }
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
                    source.matrix_ids.contains(&matrix.source_matrix_id),
                    mirror.matrix_ids.contains(&matrix.mirror_matrix_id),
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
    let target_outline_ids = staged
        .boards
        .iter()
        .find(|board| board.id == halves.mirror_board_id)
        .ok_or_else(|| "The selected mirror board is unavailable".to_owned())?
        .outline_ids
        .clone();
    for transition in &projection.independent_part_transitions {
        let IndependentPartTransition::Add {
            source_id,
            mirror_id,
            ..
        } = transition
        else {
            continue;
        };
        for feature in &mut staged.outline {
            if let OutlineFeature::PartEnvelope { id, part_ids, .. } = feature
                && target_outline_ids.contains(id)
            {
                for member in part_ids.iter_mut() {
                    if member == source_id {
                        *member = mirror_id.clone();
                    }
                }
                let mut seen = BTreeSet::new();
                part_ids.retain(|member| seen.insert(member.clone()));
            }
        }
    }

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
fn project_part(
    source: &Part,
    mirror: &Part,
    correspondence: &MirroredCorrespondence,
    mapping: Pose2,
    is_controller: bool,
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
        is_controller,
    ))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TransitionState {
    Linked,
    Local,
    Omitted,
}

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

#[derive(Clone, Debug, PartialEq, Eq)]
enum FinalOwner {
    Board(String),
    Absent,
}

#[derive(Clone, Debug, PartialEq)]
struct MatrixPairContext {
    source_id: String,
    mirror_id: String,
}

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

#[derive(Clone, Debug, PartialEq)]
struct PlannedAddition {
    source_id: String,
    mirror_id: String,
    source_reference: String,
    mirror_reference: String,
    parent_source_id: Option<String>,
    parent_mirror_id: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum BaselineKind {
    MaterializedCurrentSchema,
    ChainedPrivateDraft,
}

#[derive(Clone, Copy)]
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

#[derive(Clone, Debug, Default, PartialEq)]
struct MembershipContextDelta {
    added: Vec<TransitionAssociation>,
    retired: Vec<TransitionAssociation>,
    relocated: Vec<(String, String)>,
    owners: std::collections::BTreeMap<String, FinalOwner>,
    /// Mirror copper saved with an Omit, by the omitted item's mirror id.
    removed_copper: std::collections::BTreeMap<String, crate::model::OmittedCopper>,
}

#[derive(Clone, Debug, PartialEq)]
struct MembershipTransitionOutput {
    draft: ProjectDoc,
    changed_document_ids: Vec<String>,
    changed_field_paths: Vec<String>,
    context: MembershipContext,
    context_delta: MembershipContextDelta,
    final_owners: std::collections::BTreeMap<String, FinalOwner>,
    /// What a transition could not do for the user, such as saved copper it could not restore.
    notices: Vec<String>,
}

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
            if linked_layout
                .matrix_ids
                .iter()
                .chain(&source_layout.matrix_ids)
                .any(|id| id == &matrix_pair.source_id || id == &matrix_pair.mirror_id)
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
        // A matrix-level Local or Omitted exception is an explicit overlay on
        // the retained correspondence (spec §5): the mirror owns its
        // descriptor while the pair record stays intact. Local keeps its
        // geometry; Omitted stays live-yet-absent.
        let linked = association.state == TransitionState::Linked && !association.explicit_state;
        let matrix_exception = matches!(
            association.state,
            TransitionState::Local | TransitionState::Omitted
        ) && association.explicit_state;
        if association.source_id != matrix_pair.source_id.as_str()
            || association.mirror_id != matrix_pair.mirror_id.as_str()
            || !(linked || matrix_exception)
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

    fn source_members(
        document: &ProjectDoc,
        matrix_id: &str,
        context: &MembershipContext,
        accept_previous_omitted: bool,
    ) -> Result<BTreeSet<String>, String> {
        let matrix = matrix_by_id(document, matrix_id)?;
        crate::matrix::valid_matrix(matrix, document)?;
        let mut members = BTreeSet::new();
        let mut companion_members = BTreeSet::new();
        for cell in &matrix.cells {
            if cell.deleted || !cell.enabled {
                continue;
            }
            let key_id = format!("matrix/{}/r{}c{}", matrix.id, cell.row, cell.column);
            members.insert(key_id.clone());
            for assembly in &cell.assemblies {
                let companion_id = crate::matrix::companion_id(&key_id, &assembly.id);
                members.insert(companion_id.clone());
                companion_members.insert(companion_id);
            }
        }
        let has_omitted_matrix_members = context.pair_right.exceptions.iter().any(|exception| {
            exception.kind == MirroredItemKind::Part
                && exception.state == MirroredExceptionState::Omitted
                && companion_members.contains(&exception.mirror_id)
                && context.associations.iter().any(|association| {
                    association.kind == MirroredItemKind::Part
                        && association.mirror_id == exception.mirror_id
                })
        });
        if has_omitted_matrix_members
            && context
                .matrix_pair
                .as_ref()
                .is_some_and(|pair| pair.mirror_id == matrix_id)
        {
            let matrix_pair = require_matrix_pair(context)?;
            let source = matrix_by_id(document, matrix_pair.source_id.as_str())?;
            for omitted in exact_omitted_matrix_companions(source, matrix, &context.pair_right)? {
                let associations: Vec<_> = context
                    .associations
                    .iter()
                    .filter(|association| {
                        association.kind == MirroredItemKind::Part
                            && association.source_id == omitted.source_id
                            && association.mirror_id == omitted.mirror_id
                    })
                    .collect();
                let [association] = associations.as_slice() else {
                    continue;
                };
                let omitted_state = if accept_previous_omitted {
                    context
                        .pending_pair_transitions
                        .get(&association.source_id)
                        .or_else(|| context.pending_rejoins.get(&association.source_id))
                        .copied()
                        .unwrap_or({
                            if association.explicit_state {
                                association.state
                            } else {
                                TransitionState::Linked
                            }
                        })
                } else {
                    association.state
                };
                if omitted_state == TransitionState::Omitted
                    && (association.explicit_state
                        || context
                            .pending_pair_transitions
                            .contains_key(&association.source_id)
                        || context.pending_rejoins.contains_key(&association.source_id))
                    && association.parent_source_id.as_deref()
                        == Some(omitted.source_key_id.as_str())
                    && association.parent_mirror_id.as_deref()
                        == Some(omitted.mirror_key_id.as_str())
                {
                    members.remove(&omitted.mirror_id);
                }
            }
        }
        let actual: BTreeSet<_> = matrix.part_ids.iter().cloned().collect();
        if members != actual || actual.len() != matrix.part_ids.len() {
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

    fn cell_offset_delta(
        matrix: &Matrix,
        row: u32,
        column: u32,
        world: Vec2,
        doc: &ProjectDoc,
    ) -> Result<Vec2, String> {
        let evaluated = crate::matrix::evaluated(matrix, doc)?;
        let cell = evaluated
            .cells
            .iter()
            .find(|c| c.row == row as usize && c.column == column as usize)
            .ok_or("Local key frame is missing")?;
        Ok(crate::matrix::evaluation::inverse_offset(
            &cell.offset_frame,
            world,
        ))
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
        // A retained Local mirror keeps its record across its source removal,
        // so only the complete-removal arm is skipped below; the gate still
        // validates the retired pair side as absent.
        let matrix_retained_local = input.context.associations.iter().any(|association| {
            association.kind == MirroredItemKind::Matrix
                && association.source_id == matrix_pair.source_id.as_str()
                && association.mirror_id == matrix_pair.mirror_id.as_str()
                && association.state == TransitionState::Local
                && association.explicit_state
        });
        validate_matrix_association(&next_context, !validated.removes_paired_matrices)?;
        validate_association_settings(input.context)?;
        validate_association_settings(&next_context)?;
        if validated.removes_paired_matrices && !matrix_retained_local {
            if draft.matrices.iter().any(|matrix| {
                matrix.id == matrix_pair.source_id.as_str()
                    || matrix.id == matrix_pair.mirror_id.as_str()
            }) || !expected_mirror_members.is_empty()
            {
                return Err("The emitted whole-matrix removal is incomplete".into());
            }
        } else {
            // A retained Local mirror has no source left to check; its mirror
            // must match the validated plan below.
            if !validated.removes_paired_matrices {
                let source = matrix_by_id(&draft, matrix_pair.source_id.as_str())?;
                crate::matrix::valid_matrix(source, &draft)?;
                if source_members(&draft, matrix_pair.source_id.as_str(), &next_context, false)?
                    != validated.post_source_members
                {
                    return Err(
                        "The emitted source matrix disagrees with its validated operation map"
                            .into(),
                    );
                }
            }
            // A matrix-level Omitted mirror is record-absent by design: an
            // absent record must pair with an empty member plan.
            let mirror_absent = next_context.associations.iter().any(|association| {
                association.kind == MirroredItemKind::Matrix
                    && association.source_id == matrix_pair.source_id.as_str()
                    && association.mirror_id == matrix_pair.mirror_id.as_str()
                    && association.state == TransitionState::Omitted
                    && association.explicit_state
            }) && draft
                .matrices
                .iter()
                .all(|matrix| matrix.id != matrix_pair.mirror_id.as_str());
            if mirror_absent {
                if !expected_mirror_members.is_empty() {
                    return Err(
                        "The emitted mirror membership disagrees with its validated plan".into(),
                    );
                }
            } else {
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
            // A top-level key owned by its matrix pair names that pair (spec
            // §5) instead of standing parentless.
            let expected_matrix_parents = (
                Some(matrix_pair.source_id.clone()),
                Some(matrix_pair.mirror_id.clone()),
            );
            if ((
                association.parent_source_id.clone(),
                association.parent_mirror_id.clone(),
            ) != expected_parents
                && !(child.is_empty()
                    && (
                        association.parent_source_id.clone(),
                        association.parent_mirror_id.clone(),
                    ) == expected_matrix_parents))
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

        // A matrix-level Omitted mirror is record-absent by design
        // (live-yet-absent, spec §5): member accounting treats the missing
        // record as empty. Any other absent mirror stays a refusal.
        let matrix_omitted = context.associations.iter().any(|association| {
            association.kind == MirroredItemKind::Matrix
                && association.source_id == matrix_pair.source_id.as_str()
                && association.mirror_id == matrix_pair.mirror_id.as_str()
                && association.state == TransitionState::Omitted
                && association.explicit_state
        });
        let before_source = matrix_by_id(input.valid_before, matrix_pair.source_id.as_str())?;
        let before_mirror_absent = matrix_omitted
            && input
                .valid_before
                .matrices
                .iter()
                .all(|matrix| matrix.id != matrix_pair.mirror_id.as_str());
        let before_mirror: Option<&Matrix> = if before_mirror_absent {
            None
        } else {
            Some(matrix_by_id(
                input.valid_before,
                matrix_pair.mirror_id.as_str(),
            )?)
        };
        let post_source = input
            .post_source
            .matrices
            .iter()
            .find(|matrix| matrix.id == matrix_pair.source_id.as_str());
        let post_mirror_absent = matrix_omitted
            && input
                .post_source
                .matrices
                .iter()
                .all(|matrix| matrix.id != matrix_pair.mirror_id.as_str());
        let post_mirror: Option<&Matrix> = if post_mirror_absent {
            None
        } else {
            Some(matrix_by_id(
                input.post_source,
                matrix_pair.mirror_id.as_str(),
            )?)
        };
        if before_source.board_id.as_deref() != Some(context.source_board_id.as_str())
            || post_source.is_some_and(|matrix| {
                matrix.board_id.as_deref() != Some(context.source_board_id.as_str())
            })
            || before_mirror.is_some_and(|matrix| {
                matrix.board_id.as_deref() != Some(context.mirror_board_id.as_str())
            })
            || post_mirror.is_some_and(|matrix| {
                matrix.board_id.as_deref() != Some(context.mirror_board_id.as_str())
            })
        {
            return Err("The transition matrices do not belong to the pair boards".into());
        }
        if input.baseline_kind == BaselineKind::MaterializedCurrentSchema {
            crate::matrix::valid_matrix(before_source, input.valid_before)?;
            if let Some(before_mirror) = before_mirror {
                crate::matrix::valid_matrix(before_mirror, input.valid_before)?;
            }
        } else {
            source_members(
                input.valid_before,
                matrix_pair.source_id.as_str(),
                context,
                true,
            )?;
            if !before_mirror_absent {
                source_members(
                    input.valid_before,
                    matrix_pair.mirror_id.as_str(),
                    context,
                    true,
                )?;
            }
        }
        let before_source_members = source_members(
            input.valid_before,
            matrix_pair.source_id.as_str(),
            context,
            true,
        )?;
        let before_mirror_members = if before_mirror_absent {
            BTreeSet::new()
        } else {
            source_members(
                input.valid_before,
                matrix_pair.mirror_id.as_str(),
                context,
                true,
            )?
        };
        let post_source_members = if let Some(post_source) = post_source {
            crate::matrix::valid_matrix(post_source, input.post_source)?;
            source_members(
                input.post_source,
                matrix_pair.source_id.as_str(),
                context,
                true,
            )?
        } else {
            BTreeSet::new()
        };
        if let Some(post_mirror) = post_mirror {
            crate::matrix::valid_matrix(post_mirror, input.post_source)?;
        }
        let before_transition_members: BTreeSet<_> = before_source_members
            .union(&before_mirror_members)
            .cloned()
            .collect();
        let post_mirror_members = if post_mirror_absent {
            BTreeSet::new()
        } else {
            source_members(
                input.post_source,
                matrix_pair.mirror_id.as_str(),
                context,
                true,
            )?
        };
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
        if associated_sources.len() != part_associations.len() {
            return Err("The historical Part associations omit canonical source membership".into());
        }
        // Members added while their matrix link was detached (Local) or absent (Omitted) never
        // projected, so they hold no correspondence until an explicit Rejoin adopts them. They
        // may trim away without historical associations; anything else still refuses below.
        let matrix_unprojected = context.pair_right.exceptions.iter().any(|exception| {
            exception.kind == MirroredItemKind::Matrix
                && exception.mirror_id == matrix_pair.mirror_id
                && matches!(
                    exception.state,
                    crate::model::MirroredExceptionState::Local
                        | crate::model::MirroredExceptionState::Omitted
                )
        });
        if !before_source_members.is_subset(&associated_sources) && !matrix_unprojected {
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
            // A top-level key owned by its matrix pair names that pair (spec
            // §5) instead of standing parentless.
            let expected_matrix_parents = (
                Some(matrix_pair.source_id.clone()),
                Some(matrix_pair.mirror_id.clone()),
            );
            if association.mirror_id != expected_mirror
                || ((
                    association.parent_source_id.clone(),
                    association.parent_mirror_id.clone(),
                ) != expected_parents
                    && !(child.is_empty()
                        && (
                            association.parent_source_id.clone(),
                            association.parent_mirror_id.clone(),
                        ) == expected_matrix_parents))
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
            let Some(association) = association_by_source.get(old_source.as_str()) else {
                // Correspondence-less members of an unprojected matrix link carry no
                // association-dependent evidence; the injectivity check above already guards
                // their identity mapping.
                if matrix_unprojected {
                    continue;
                }
                return Err(format!(
                    "The source identity {old_source:?} has no private association"
                ));
            };
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
        if planned_source_ids.len() != input.planned_additions.len() {
            return Err(
                "The source operation map does not describe the post-operation matrix".into(),
            );
        }
        let mapped_or_planned: BTreeSet<_> = mapped_source_ids
            .union(&planned_source_ids)
            .cloned()
            .collect();
        if mapped_or_planned != post_source_members {
            // A detached mirror takes no projected members (spec §5): the only
            // permitted gap is genuine source-side growth under it.
            let uncovered: Vec<_> = post_source_members.difference(&mapped_or_planned).collect();
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
            let source_growth = matrix_detached
                && !uncovered.is_empty()
                && uncovered.iter().all(|id| {
                    matrix_identity_slot(id, matrix_pair.source_id.as_str()).is_some()
                        && !before_source_members.contains(*id)
                });
            if !source_growth {
                return Err(
                    "The source operation map does not describe the post-operation matrix".into(),
                );
            }
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
                    // A matrix-owned top-level key names its matrix pair (spec
                    // §5); a companion names its key. Either parent carries
                    // the inherited state.
                    let matrix_parent = context.associations.iter().find(|candidate| {
                        candidate.kind == MirroredItemKind::Matrix
                            && candidate.source_id == parent_source
                            && candidate.mirror_id == parent_mirror
                    });
                    if let Some(matrix_parent) = matrix_parent {
                        let source_suffix = association
                            .source_id
                            .strip_prefix(&format!("matrix/{parent_source}/"));
                        let mirror_suffix = association
                            .mirror_id
                            .strip_prefix(&format!("matrix/{parent_mirror}/"));
                        if source_suffix.is_none() || source_suffix != mirror_suffix {
                            return Err(
                                "The transition association has an invalid parent source".into()
                            );
                        }
                        if !association.explicit_state && association.state != matrix_parent.state {
                            return Err(
                                "An inherited child state disagrees with its parent association"
                                    .into(),
                            );
                        }
                        if matrix_parent.state == TransitionState::Local
                            && association.state == TransitionState::Omitted
                            && !association.explicit_state
                        {
                            return Err(
                                "An Omitted child of a Local owner must be an explicit child override"
                                    .into(),
                            );
                        }
                    } else {
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
                            return Err(
                                "The transition association has an unavailable parent".into()
                            );
                        };
                        if !association.explicit_state && association.state != parent.state {
                            return Err(
                                "An inherited child state disagrees with its parent association"
                                    .into(),
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
                && !before_mirror.is_some_and(|mirror| mirror.part_ids.contains(old_id))
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
        // A matrix-level Local mirror survives its source removal as a
        // complete standalone matrix (spec §5): the mirror record, its
        // members, and their associations are all retained. A matrix-level
        // Omitted mirror is already absent and stays that way.
        let matrix_retained_local = context.associations.iter().any(|association| {
            association.kind == MirroredItemKind::Matrix
                && association.source_id == matrix_pair.source_id.as_str()
                && association.mirror_id == matrix_pair.mirror_id.as_str()
                && association.state == TransitionState::Local
                && association.explicit_state
        });
        let matrix_omitted = context.associations.iter().any(|association| {
            association.kind == MirroredItemKind::Matrix
                && association.source_id == matrix_pair.source_id.as_str()
                && association.mirror_id == matrix_pair.mirror_id.as_str()
                && association.state == TransitionState::Omitted
                && association.explicit_state
        });
        let mirror_absent = draft
            .matrices
            .iter()
            .all(|matrix| matrix.id != matrix_pair.mirror_id.as_str());
        // The source removal itself is the change when the mirror is
        // retained or already absent; only an executed mirror removal must
        // report its own changes below.
        let skip_mirror_removal =
            validated.already_removed || matrix_retained_local || (matrix_omitted && mirror_absent);
        let mirror_removal = if skip_mirror_removal {
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
        // A source removal over a detached mirror retires every record of
        // the pair: with the source gone, the halves record cannot excuse
        // surviving correspondences, so all of them go. A retained Local
        // mirror keeps its record and members but loses its links, landing
        // as a plain standalone matrix; an Omitted mirror stays absent
        // without resurrection. Only defunct links retire — the defunct
        // source-layout link already retired before the kernel.
        let (retired, retained): (Vec<_>, Vec<_>) = if matrix_omitted || matrix_retained_local {
            (next_context.associations.clone(), Vec::new())
        } else {
            let retired: Vec<_> = next_context
                .associations
                .iter()
                .filter(|association| {
                    removed_sources.contains(&association.source_id)
                        && removed_mirrors.contains(&association.mirror_id)
                })
                .cloned()
                .collect();
            let retained: Vec<_> = next_context
                .associations
                .iter()
                .filter(|association| {
                    !removed_sources.contains(&association.source_id)
                        || !removed_mirrors.contains(&association.mirror_id)
                })
                .cloned()
                .collect();
            (retired, retained)
        };
        next_context.associations = retained;
        next_context.owners = input.final_owners.clone();
        let context_delta = MembershipContextDelta {
            retired,
            owners: input.final_owners.clone(),
            ..MembershipContextDelta::default()
        };
        let expected_mirror_members = if matrix_retained_local {
            validated.mapped_mirror_ids.clone()
        } else {
            BTreeSet::new()
        };
        let output = emit_validated_transition(
            &validated,
            draft,
            next_context,
            context_delta,
            &expected_mirror_members,
        )?;
        if !skip_mirror_removal && mirror_removal.is_empty() {
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
    // A detached mirror owns its membership (spec §5): source-side growth
    // projects nothing, so the map carries no additions for the kernel to
    // materialize. The map check above already proved the residual change is
    // genuine source growth; every other arm keeps its shape. Only a Local
    // mirror skips projection below; an Omitted mirror still follows the
    // source record while staying absent.
    let matrix_detached = context.pending_rejoins.is_empty()
        && context.pending_pair_transitions.is_empty()
        && context.associations.iter().any(|association| {
            association.kind == MirroredItemKind::Matrix
                && association.source_id == matrix_pair.source_id.as_str()
                && association.mirror_id == matrix_pair.mirror_id.as_str()
                && matches!(
                    association.state,
                    TransitionState::Local | TransitionState::Omitted
                )
                && association.explicit_state
        });
    let matrix_detached_local = context.pending_rejoins.is_empty()
        && context.pending_pair_transitions.is_empty()
        && context.associations.iter().any(|association| {
            association.kind == MirroredItemKind::Matrix
                && association.source_id == matrix_pair.source_id.as_str()
                && association.mirror_id == matrix_pair.mirror_id.as_str()
                && association.state == TransitionState::Local
                && association.explicit_state
        });
    let matrix_detached_omitted = context.pending_rejoins.is_empty()
        && context.pending_pair_transitions.is_empty()
        && context.associations.iter().any(|association| {
            association.kind == MirroredItemKind::Matrix
                && association.source_id == matrix_pair.source_id.as_str()
                && association.mirror_id == matrix_pair.mirror_id.as_str()
                && association.state == TransitionState::Omitted
                && association.explicit_state
        });
    if matrix_detached && !input.planned_additions.is_empty() {
        return Err("A detached mirror takes no projected additions".into());
    }
    let is_detached_expansion = removed_slots.is_empty()
        && shifted_slots.is_empty()
        && input.planned_additions.is_empty()
        && (matrix_detached_local || matrix_detached_omitted);
    if !is_compaction && !is_trim && !is_expansion && !is_rejoin && !is_detached_expansion {
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
    // An Omitted-absent mirror has no record to stage or project onto: the
    // per-cell pass and write-back below are skipped with it.
    let mirror_absent = matrix_detached_omitted
        && staged
            .matrices
            .iter()
            .all(|matrix| matrix.id != matrix_pair.mirror_id.as_str());
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
    // A detached Local mirror owns its descriptor, and an absent mirror has
    // no record at all: membership staging never rewrites either, whatever
    // the source operation removed or compacted.
    if let Some((axis_row, removed_index)) =
        compaction_axis.filter(|_| !matrix_detached_local && !mirror_absent)
    {
        crate::matrix::deletion::remove_axis(
            &mut staged,
            matrix_pair.mirror_id.as_str(),
            removed_index,
            axis_row,
        )?;
    } else if is_trim && !matrix_detached_local && !mirror_absent {
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
    let mirror: Option<Matrix> = staged
        .matrices
        .iter()
        .find(|matrix| matrix.id == matrix_pair.mirror_id.as_str())
        .cloned();
    if mirror.is_none() && !mirror_absent {
        return Err(format!(
            "The transition matrix {:?} is unavailable",
            matrix_pair.mirror_id
        ));
    }
    // Detached Local: the mirror owns its descriptor and geometry, so the
    // projection below is skipped and the owned descriptor is reused verbatim.
    // An absent mirror likewise skips everything: the placeholder below never
    // reaches the write-back. (The per-cell loop carries a no-op filter to
    // keep this diff flat; the preservation pass stays empty, so no
    // compensation dirties the mirror.)
    let mut projected = mirror.clone().unwrap_or_else(|| source.clone());
    let mut local_key_preservations = Vec::new();
    let mut omitted_target_parts = BTreeSet::new();
    if !matrix_detached_local && !mirror_absent {
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
        let present = mirror.clone().ok_or_else(|| {
            format!(
                "The transition matrix {:?} is unavailable",
                matrix_pair.mirror_id
            )
        })?;
        projected = project_matrix(&source, &present, matrix_correspondence, context.mapping)?;
    }
    // The loop only executes with a present mirror (see its filter); the owned
    // snapshot keeps the body total without re-proving presence per cell.
    let mirror_cells: Vec<_> = mirror
        .iter()
        .flat_map(|present| present.cells.iter().cloned())
        .collect();
    for cell in projected
        .cells
        .iter_mut()
        .filter(|_| !matrix_detached_local && !mirror_absent)
    {
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
        if original_key.is_none() && matrix_detached_omitted {
            // An Omitted mirror follows the source record without
            // materializing: novel slots stay disabled with no companions, so
            // later member accounting sees a canonical absent mirror.
            cell.deleted = true;
            cell.enabled = false;
            cell.assemblies.clear();
            cell.assemblies_local = None;
            continue;
        }
        if original_key.is_some_and(|association| association.state == TransitionState::Omitted) {
            cell.deleted = true;
            cell.enabled = false;
            cell.assemblies.clear();
            cell.assemblies_local = None;
            continue;
        }
        if original_key.is_some_and(|association| association.state == TransitionState::Local) {
            let local_cell = mirror_cells
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
            for assembly in &cell.assemblies {
                let source_child_id = crate::matrix::companion_id(&source_key_id, &assembly.id);
                if context.associations.iter().any(|child| {
                    child.kind == MirroredItemKind::Part
                        && input.source_identity_map.get(&child.source_id)
                            == Some(&Some(source_child_id.clone()))
                        && child.state == TransitionState::Omitted
                        && child.explicit_state
                }) {
                    omitted_target_parts.insert(crate::matrix::companion_id(
                        &format!(
                            "matrix/{}/r{}c{}",
                            matrix_pair.mirror_id.as_str(),
                            cell.row,
                            cell.column
                        ),
                        &assembly.id,
                    ));
                }
            }
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
                omitted_target_parts.insert(mirror_child_id);
                next_assemblies.push(assembly);
                continue;
            }
            if original.is_some_and(|association| association.state == TransitionState::Local) {
                let local_assembly = mirror_cells
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
            let offset_delta =
                cell_offset_delta(&projected, *row, *column, world_delta, &candidate)?;
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
    // Detached Local keeps the mapped (identity) members: rejoins cannot be
    // pending here by construction, and additions are refused above, so both
    // extensions below are naturally empty on the detached path.
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
        .iter()
        .flat_map(|mirror| mirror.part_ids.iter())
        .filter(|id| {
            !expected_final_mirror_members.contains(*id) && !locally_detached_ids.contains(*id)
        })
        .cloned()
        .collect();
    let _pruned_ids = crate::matrix::deletion::remove_parts_by_id(&mut staged, &removed_target_ids);
    // A detached Local mirror reuses its owned descriptor verbatim, and an
    // absent mirror has no record to write: in both cases the projection
    // write-back is skipped (the prune set above is empty here).
    if !matrix_detached_local && !mirror_absent {
        crate::matrix::set_matrix(&mut staged, &projected)?;
    }
    remove_omitted_matrix_parts(
        &mut staged,
        matrix_pair.mirror_id.as_str(),
        &omitted_target_parts,
    );
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

fn matrix_identity_slot<'a>(id: &'a str, matrix_id: &str) -> Option<(u32, u32, &'a str)> {
    let prefix = format!("matrix/{matrix_id}/r");
    let suffix = id.strip_prefix(&prefix)?;
    let (slot, companion) = suffix
        .split_once('/')
        .map_or((suffix, ""), |(slot, child)| (slot, child));
    let (row, column) = slot.split_once('c')?;
    Some((row.parse().ok()?, column.parse().ok()?, companion))
}

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
pub(in crate::mirrored_half) mod tests;
