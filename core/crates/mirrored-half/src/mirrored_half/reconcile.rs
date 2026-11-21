//! Forward reconciliation building blocks for already-corresponded mirrored values.
//!
//! Where things live (search the names; line numbers drift):
//! - This file: projection types (`Projection`, `IndependentPartTransition`, `HostedTransition`),
//!   membership-transition types (`MembershipContext`, `MembershipTransitionInput`, ...) and
//!   shared pair/matrix helpers.
//! - `membership_view`: `FixedMembershipView`, `build_fixed_membership_view`.
//! - `document`: the `reconcile` body, in order: baseline validators and `stage_matrix`;
//!   pair/halves lookup; independent part Add/Remove; `hosted::HostedTargets` and circuit pairs;
//!   hosted `AddCircuit` (`hosted::stage_hosted_circuit_additions`); board/layout
//!   correspondences; module targets; final validation, `outlines::reconcile_draft`.
//! - `hosted`: hosted circuit staging and hosted removal/omission baseline validators.
//! - `matrix_stage`: `MatrixStage`, `stage_matrix`, `project_matrix`, `project_part`.
//! - `transition`: `prepare_membership_transition_kernel`, `prepare_membership_transition`;
//!   its validators live in `transition_checks` and `transition_input`.
//! - Child modules: `lifecycle` (reconcile/lifecycle.rs), `outlines` (reconcile/outlines.rs),
//!   `prepare`, `settings`.
//! - `mod tests` in reconcile/tests.rs; outline tests are included from
//!   reconcile/outlines/tests.rs.

use crate::model::{
    Matrix, MirroredCorrespondence, MirroredExceptionState, MirroredHalves, MirroredItemKind,
    Pose2, ProjectDoc,
};
use std::collections::{BTreeMap, BTreeSet};

#[doc(hidden)]
pub mod lifecycle;
#[doc(hidden)]
pub mod prepare;
pub use prepare::{prepare_open, prepare_transaction_with_ledger};
mod outlines;
#[doc(hidden)]
pub mod settings;

mod changes;
mod document;
pub use changes::changed_document_fields;
mod hosted;
mod matrix_membership;
mod matrix_stage;
mod membership_view;
mod transition;
mod transition_checks;
mod transition_input;

pub use document::reconcile;
pub use hosted::{HostedCircuitAddProvenance, stage_hosted_correspondence};
use hosted::{
    stage_hosted_circuit_additions, validate_hosted_omission_baselines,
    validate_hosted_removal_baselines,
};
use matrix_stage::{MatrixStage, RetainedCell, stage_matrix};
pub use matrix_stage::{project_matrix, project_part};
pub use membership_view::{FixedMembershipView, build_fixed_membership_view};
use membership_view::{exact_omitted_matrix_companions, remove_omitted_matrix_parts};
pub use transition::{prepare_membership_transition, prepare_membership_transition_kernel};
use transition_checks::{
    board_owner, cell_offset_delta, emit_validated_transition, pair_right, source_members,
    transition_matrix_by_id, validate_association_settings, validate_constraints,
    validate_context_owners, validate_matrix_association, validate_net_pad_ids,
    validate_no_native_legacy_overlap, validate_transition_output,
    validate_transition_part_geometry,
};
use transition_input::validate_transition_input;

#[derive(Clone)]
#[doc(hidden)]
pub struct PartProjection {
    #[doc(hidden)]
    pub source_id: String,
    #[doc(hidden)]
    pub mirror_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[doc(hidden)]
pub enum IndependentPartTransition {
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
#[doc(hidden)]
pub enum HostedTransition {
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
#[doc(hidden)]
pub struct Projection {
    #[doc(hidden)]
    pub matrix: Option<MatrixLink>,
    /// Preserve geometry already prepared by the native membership lifecycle.
    #[doc(hidden)]
    pub preserve_matrix_geometry: bool,
    #[doc(hidden)]
    pub source_board_id: String,
    #[doc(hidden)]
    pub mirror_board_id: String,
    #[doc(hidden)]
    pub mapping: Pose2,
    #[doc(hidden)]
    pub parts: Vec<PartProjection>,
    #[doc(hidden)]
    pub independent_part_transitions: Vec<IndependentPartTransition>,
    #[doc(hidden)]
    pub hosted_transitions: Vec<HostedTransition>,
    #[doc(hidden)]
    pub applied_independent_additions:
        std::cell::RefCell<BTreeSet<(String, String, String, String)>>,
    // Ticket 107 records hosted Add provenance privately during reconciliation.
    #[doc(hidden)]
    pub applied_hosted_operations: std::cell::RefCell<BTreeSet<(String, String, String)>>,
    // Ticket 107 binds circuit Add replay to the exact selected maps and first output.
    #[doc(hidden)]
    pub applied_hosted_circuit_additions:
        std::cell::RefCell<BTreeMap<(String, String, String), HostedCircuitAddProvenance>>,
    // Ticket 107 keeps source-operation baselines private to the reconciliation kernel.
    #[doc(hidden)]
    pub hosted_removal_baselines:
        std::cell::RefCell<BTreeMap<(String, String, String), ProjectDoc>>,
    // Ticket 107 retains validated owner context for physically absent Omitted children.
    #[doc(hidden)]
    pub hosted_omission_baselines:
        std::cell::RefCell<BTreeMap<(String, String, String), ProjectDoc>>,
    // Ticket 108 authenticates outline removals against their valid pre-source pair.
    #[doc(hidden)]
    pub outline_removal_baselines:
        std::cell::RefCell<BTreeMap<(String, String, String), ProjectDoc>>,
    #[doc(hidden)]
    pub locally_protected_mirror_parts: std::collections::BTreeSet<String>,
    #[doc(hidden)]
    pub final_owners: std::collections::BTreeMap<String, String>,
}

#[derive(Clone)]
#[doc(hidden)]
pub struct MatrixLink {
    #[doc(hidden)]
    pub source_matrix_id: String,
    #[doc(hidden)]
    pub mirror_matrix_id: String,
}

// A structural command owns its axis even when every cell on that axis was
// already deleted. Other edits can also change membership as a side effect.
fn prepares_matrix_membership(
    operation: &crate::model::EditOperation,
    before: &ProjectDoc,
    after: &ProjectDoc,
    matrix_id: &str,
) -> bool {
    transition::RemovedAxis::for_operation(operation, matrix_id).is_some()
        || matrix_membership_changed(before, after, matrix_id)
}

#[doc(hidden)]
pub fn matrix_membership_changed(before: &ProjectDoc, after: &ProjectDoc, matrix_id: &str) -> bool {
    fn members<'a>(document: &'a ProjectDoc, matrix_id: &str) -> Option<&'a [String]> {
        document
            .matrices
            .iter()
            .find(|matrix| matrix.id == matrix_id)
            .map(|matrix| matrix.part_ids.as_slice())
    }
    // Setters usually keep canonical member order. Only compare unordered membership when
    // that order differs, retaining the existing duplicate and absent-matrix semantics.
    match (members(before, matrix_id), members(after, matrix_id)) {
        (Some(before), Some(after)) if before == after => false,
        (Some(before), Some(after)) if before.len() != after.len() => true,
        (Some(before), Some(after)) => {
            let mut before: Vec<_> = before.iter().collect();
            let mut after: Vec<_> = after.iter().collect();
            before.sort_unstable();
            after.sort_unstable();
            before != after
        }
        (None, None) => false,
        (None, Some(_)) | (Some(_), None) => true,
    }
}

fn projection_for_pair(halves: &crate::model::MirroredHalves, mapping: Pose2) -> Projection {
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

fn selected_pair<'a>(
    document: &'a ProjectDoc,
    id: &str,
    context: &str,
) -> Result<&'a crate::model::MechanicalMirrorPair, String> {
    let hardware = document
        .hardware
        .as_ref()
        .ok_or_else(|| format!("The {context} hardware is unavailable"))?;
    let mut matches = hardware.pairs.iter().filter(|pair| pair.id == id);
    let pair = matches
        .next()
        .ok_or_else(|| format!("The {context} pair {id:?} is unavailable"))?;
    if matches.next().is_some() {
        return Err(format!("The {context} pair {id:?} is ambiguous"));
    }
    Ok(pair)
}

fn same_pair_bytes(
    before: &ProjectDoc,
    after: &ProjectDoc,
    id: &str,
    context: &str,
) -> Result<bool, String> {
    let bytes = |document: &ProjectDoc| {
        serde_json::to_vec(selected_pair(document, id, context)?).map_err(|error| error.to_string())
    };
    Ok(bytes(before)? == bytes(after)?)
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

fn circuit_links(halves: &MirroredHalves) -> impl Iterator<Item = &MirroredCorrespondence> {
    halves
        .correspondences
        .iter()
        .filter(|link| link.kind == MirroredItemKind::EmbeddedCircuit)
}

fn effective_circuit_child_state<'a>(
    document: &ProjectDoc,
    halves: &MirroredHalves,
    correspondence: &MirroredCorrespondence,
    mut circuit_links: impl Iterator<Item = &'a MirroredCorrespondence>,
) -> Option<MirroredExceptionState> {
    if halves.exceptions.is_empty() {
        return None;
    }
    if let Some(exception) = halves.overlay_of(correspondence) {
        return Some(exception.state);
    }
    let child_id = correspondence.mirror_id.as_str();
    let owner = circuit_links.find(|owner| {
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[doc(hidden)]
pub enum TransitionState {
    Linked,
    Local,
    Omitted,
}

#[derive(Clone, Debug, PartialEq)]
#[doc(hidden)]
pub struct TransitionAssociation {
    #[doc(hidden)]
    pub kind: MirroredItemKind,
    #[doc(hidden)]
    pub source_id: String,
    #[doc(hidden)]
    pub mirror_id: String,
    #[doc(hidden)]
    pub parent_source_id: Option<String>,
    #[doc(hidden)]
    pub parent_mirror_id: Option<String>,
    #[doc(hidden)]
    pub state: TransitionState,
    #[doc(hidden)]
    pub explicit_state: bool,
    #[doc(hidden)]
    pub raw_mode_override: Option<crate::model::MirroredCompanionMode>,
    #[doc(hidden)]
    pub raw_tweak: Option<crate::model::MirroredCompanionTweak>,
    #[doc(hidden)]
    pub effective_mode: crate::model::MirroredCompanionMode,
    #[doc(hidden)]
    pub effective_tweak: Option<crate::model::MirroredCompanionTweak>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[doc(hidden)]
pub enum FinalOwner {
    Board(String),
    Absent,
}

#[derive(Clone, Debug, PartialEq)]
#[doc(hidden)]
pub struct MatrixPairContext {
    #[doc(hidden)]
    pub source_id: String,
    #[doc(hidden)]
    pub mirror_id: String,
}

#[derive(Clone, Debug, PartialEq)]
#[doc(hidden)]
pub struct MembershipContext {
    #[doc(hidden)]
    pub pair_id: String,
    #[doc(hidden)]
    pub source_board_id: String,
    #[doc(hidden)]
    pub mirror_board_id: String,
    #[doc(hidden)]
    pub matrix_pair: Option<MatrixPairContext>,
    #[doc(hidden)]
    pub mapping: Pose2,
    #[doc(hidden)]
    pub pair_right: crate::model::MirroredHalves,
    #[doc(hidden)]
    pub associations: Vec<TransitionAssociation>,
    #[doc(hidden)]
    pub owners: std::collections::BTreeMap<String, FinalOwner>,
    #[doc(hidden)]
    pub pending_rejoins: BTreeMap<String, TransitionState>,
    #[doc(hidden)]
    pub pending_pair_transitions: BTreeMap<String, TransitionState>,
    #[doc(hidden)]
    pub pending_pair_baseline: Option<crate::model::MirroredHalves>,
}

fn require_matrix_pair(context: &MembershipContext) -> Result<&MatrixPairContext, String> {
    context
        .matrix_pair
        .as_ref()
        .ok_or_else(|| "Matrix membership work requires an exact Matrix correspondence".to_owned())
}

#[derive(Clone, Debug, PartialEq)]
#[doc(hidden)]
pub struct PlannedAddition {
    #[doc(hidden)]
    pub source_id: String,
    #[doc(hidden)]
    pub mirror_id: String,
    #[doc(hidden)]
    pub source_reference: String,
    #[doc(hidden)]
    pub mirror_reference: String,
    #[doc(hidden)]
    pub parent_source_id: Option<String>,
    #[doc(hidden)]
    pub parent_mirror_id: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[doc(hidden)]
pub enum BaselineKind {
    MaterializedCurrentSchema,
    ChainedPrivateDraft,
}

#[derive(Clone, Copy)]
#[doc(hidden)]
pub struct MembershipTransitionInput<'a> {
    #[doc(hidden)]
    pub dispatch: super::EditDispatch,
    #[doc(hidden)]
    pub valid_before: &'a ProjectDoc,
    #[doc(hidden)]
    pub post_source: &'a ProjectDoc,
    #[doc(hidden)]
    pub context: &'a MembershipContext,
    #[doc(hidden)]
    pub baseline_kind: BaselineKind,
    #[doc(hidden)]
    pub source_identity_map: &'a std::collections::BTreeMap<String, Option<String>>,
    #[doc(hidden)]
    pub mirror_identity_map: &'a std::collections::BTreeMap<String, Option<String>>,
    #[doc(hidden)]
    pub planned_additions: &'a [PlannedAddition],
    #[doc(hidden)]
    pub final_owners: &'a std::collections::BTreeMap<String, FinalOwner>,
    #[doc(hidden)]
    pub explicit_removal_closure: &'a std::collections::BTreeSet<String>,
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
#[doc(hidden)]
pub struct MembershipContextDelta {
    #[doc(hidden)]
    pub added: Vec<TransitionAssociation>,
    #[doc(hidden)]
    pub retired: Vec<TransitionAssociation>,
    #[doc(hidden)]
    pub relocated: Vec<(String, String)>,
    #[doc(hidden)]
    pub owners: std::collections::BTreeMap<String, FinalOwner>,
    /// Mirror copper saved with an Omit, by the omitted item's mirror id.
    removed_copper: std::collections::BTreeMap<String, crate::model::OmittedCopper>,
}

#[derive(Clone, Debug, PartialEq)]
#[doc(hidden)]
pub struct MembershipTransitionOutput {
    #[doc(hidden)]
    pub draft: ProjectDoc,
    #[doc(hidden)]
    pub changed_document_ids: Vec<String>,
    #[doc(hidden)]
    pub changed_field_paths: Vec<String>,
    #[doc(hidden)]
    pub context: MembershipContext,
    #[doc(hidden)]
    pub context_delta: MembershipContextDelta,
    #[doc(hidden)]
    pub final_owners: std::collections::BTreeMap<String, FinalOwner>,
    /// What a transition could not do for the user, such as saved copper it could not restore.
    #[doc(hidden)]
    pub notices: Vec<String>,
}

#[doc(hidden)]
pub fn matrix_identity_slot<'a>(id: &'a str, matrix_id: &str) -> Option<(u32, u32, &'a str)> {
    let prefix = format!("matrix/{matrix_id}/r");
    let suffix = id.strip_prefix(&prefix)?;
    let (slot, companion) = suffix
        .split_once('/')
        .map_or((suffix, ""), |(slot, child)| (slot, child));
    let (row, column) = slot.split_once('c')?;
    Some((row.parse().ok()?, column.parse().ok()?, companion))
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
