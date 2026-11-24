//! Applies one typed mechanical edit against the accepted document.
//!
//! Defaults, validation, linked-feature propagation and closure-clearance
//! projection live here, behind the mechanical module's seam. The Case adapter
//! submits a typed [`MechanicalEdit`] and the operation reports only the durable
//! ids whose values or memberships actually changed.
//!
//! One accepted edit owns its whole result: the effective configuration change,
//! absent closure-mount initialization and the generated closure-clearance
//! projection. Every mutation is staged on a clone, so a failure (including a
//! missing required mounting-hole template) leaves the accepted document untouched,
//! and the complete result is one Undo step. A change equal to the accepted value
//! that needs no initialization is a no-op that reports nothing.
use crate::model::*;

use super::change::{MechanicalChange, MechanicalDimension, MechanicalEdit, MechanicalTarget};
use super::inputs::effective_inputs;

mod changes;
mod closures;
mod feature_lifecycle;
mod identity;
mod ownership;
mod paired;
mod processes;
mod validation;

use changes::apply_change;
pub use changes::default_configuration;
pub use closures::resolve_target;
use closures::{
    apply_settings, closure_list_absent, initialize_target_closures, owner_id, paired_index,
    target_board_id,
};
use feature_lifecycle::{
    apply_feature_lifecycle, find_feature_value, instance_configuration_mut, pair_at,
    pair_owner_ids,
};
pub use feature_lifecycle::{install_feature, remove_feature};
pub use identity::{IdentityFloor, MechanicalIdentityLedger};
use identity::{
    identity_index_for, identity_reservation_of, merge_identity_floor, reserve_identity,
    reserve_opening_identity, reserve_vertex_identity, reserved_identity_floor,
};
pub use ownership::reconcile_physical_topology;
use ownership::{
    disable, enable, initialize_closures, link_mirror_pair, pair_index, persist, persist_edit,
    remove_physical_instance, unlink_mirror_pair,
};
use processes::{
    closure_hardware_mut, critical_fit_mut, hardware_mut, initial_switch_family,
    is_process_target, material_options, normalize_processes, process_default_thickness,
    process_mut, profile_family, resize_closure_insert, stabilizer_target_exists,
};
use validation::{
    ProfileTargetKind, mount_collection_mut, set_dimension, valid_profile_polygons,
    validate_dimension, validate_gasket_support_anchors, validate_mounts, validate_openings,
    validate_profile_openings, validate_profile_target,
};

const GENERATED_PART_PREFIX: &str = "case-closure/";
const GENERATED_DEFINITION_PREFIX: &str = "assembly-closure/definition/";
const AUTO_CLOSURE_PREFIX: &str = "auto-closure/";

/// Applies one mechanical edit, returning the changed durable ids.
///
/// The configuration owner (the canonical board id or the physical instance id)
/// is always the first reported id; profile, hardware, critical-fit, opening,
/// mount and process ids follow when the ported code changes them, then the
/// generated clearance part ids, definition ids and membership owners the
/// projection actually changed.
pub fn apply(
    doc: &mut ProjectDoc,
    board_contours: &[BoardContours],
    edit: &MechanicalEdit,
) -> Result<Vec<String>, String> {
    apply_with_ledger(
        doc,
        board_contours,
        edit,
        &MechanicalIdentityLedger::default(),
    )
}

/// Applies one mechanical edit with the owning Core session's identity high-water
/// marks. The public [`apply`] path supplies an empty ledger for standalone callers.
pub fn apply_with_ledger(
    doc: &mut ProjectDoc,
    board_contours: &[BoardContours],
    edit: &MechanicalEdit,
    identities: &MechanicalIdentityLedger,
) -> Result<Vec<String>, String> {
    // Stage the whole edit on a clone so any later failure leaves the accepted
    // document - configuration, initialization and projection - untouched.
    let mut next = doc.clone();
    let changed = apply_staged(&mut next, board_contours, edit, identities)?;
    *doc = next;
    Ok(changed)
}

/// The disposition of a submitted mechanical edit against the accepted document.
///
/// The evaluator answers the settlement question the adapter asks before it
/// commits: does this edit change durable state, does it equal the accepted value,
/// or has its target gone so the queued intent must retire?
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MechanicalEditDisposition {
    /// The edit changes durable state.
    Changed,
    /// The edit equals the accepted value and needs no initialization.
    Unchanged,
    /// The target has vanished or is ineligible, so the queued edit retires.
    Retire,
}

/// Evaluates one typed mechanical edit against the accepted document.
///
/// This is the read-only half of the operation: it shares the private apply and
/// evaluation implementation with [`apply`], so a resolver no-op is detected by
/// the same rules the commit uses, and it never exposes a prepared replacement
/// document. A missing or ineligible target is [`MechanicalEditDisposition::Retire`]
/// (the queued intent is dropped silently); structural problems - malformed values
/// or a missing or invalid mounting-hole template - are `Err` with no mutation.
/// `board_contours` is the accepted board outline projection: absent closure
/// mounts are proposed from it (never from a fresh recompute of the proposed
/// document), and contour-dependent projection and fit findings resolve against it.
pub fn evaluate_edit(
    document: &ProjectDoc,
    board_contours: &[BoardContours],
    edit: &MechanicalEdit,
) -> Result<MechanicalEditDisposition, String> {
    // A target that no longer exists retires: the queued intent was authored
    // against an entity that has since been removed, so it must be dropped rather
    // than redirected at whatever now occupies its place.
    if !target_exists(document, &edit.target) {
        return Ok(MechanicalEditDisposition::Retire);
    }
    // A physical half that exists but has no effective configuration is ineligible:
    // a pending geometry or settings edit retires instead of erroring, while Enable
    // stays possible and Disable remains available for stored or pair-owned settings.
    if !matches!(
        edit.change,
        MechanicalChange::Enable | MechanicalChange::Disable
    ) && matches!(edit.target, MechanicalTarget::Instance { .. })
        && effective_inputs(document, board_contours, &edit.target).is_err()
    {
        return Ok(MechanicalEditDisposition::Retire);
    }
    // An edit that addresses a durable entity which is gone retires too: a queued
    // intent must never be redirected at whatever later addition reused its place.
    if addressed_entity_is_gone(document, board_contours, edit) {
        return Ok(MechanicalEditDisposition::Retire);
    }
    let mut next = document.clone();
    match apply_staged(
        &mut next,
        board_contours,
        edit,
        &MechanicalIdentityLedger::default(),
    ) {
        Ok(changed) if changed.is_empty() => Ok(MechanicalEditDisposition::Unchanged),
        Ok(_) => Ok(MechanicalEditDisposition::Changed),
        Err(message) => Err(message),
    }
}

/// Whether the change addresses an opening/vertex identity that no longer exists.
///
/// Only durable identities are considered; a change that names none, or whose target
/// cannot be resolved at all, is left to the operation to reject.
fn addressed_entity_is_gone(
    document: &ProjectDoc,
    board_contours: &[BoardContours],
    edit: &MechanicalEdit,
) -> bool {
    let Ok(inputs) = effective_inputs(document, board_contours, &edit.target) else {
        return false;
    };
    let configuration = inputs.configuration;
    let opening = |id: &str| {
        configuration
            .openings
            .as_ref()
            .is_some_and(|openings| openings.iter().any(|entry| entry.id == id))
    };
    let vertex = |opening_id: &str, vertex_id: &str| {
        configuration
            .openings
            .as_ref()
            .and_then(|openings| openings.iter().find(|entry| entry.id == opening_id))
            .is_some_and(|entry| entry.vertices.iter().any(|v| v.id == vertex_id))
    };
    match &edit.change {
        MechanicalChange::RemoveOpening { opening_id }
        | MechanicalChange::AddOpeningVertex { opening_id } => !opening(opening_id),
        MechanicalChange::RemoveOpeningVertex {
            opening_id,
            vertex_id,
        } => !opening(opening_id) || !vertex(opening_id, vertex_id),
        MechanicalChange::SetOpeningDimension {
            opening_id,
            vertex_id,
            ..
        } => {
            !opening(opening_id)
                || vertex_id
                    .as_deref()
                    .is_some_and(|vertex_id| !vertex(opening_id, vertex_id))
        }
        _ => false,
    }
}

/// Whether the edit target still exists in the accepted document.
fn target_exists(document: &ProjectDoc, target: &MechanicalTarget) -> bool {
    match target {
        MechanicalTarget::Board { board_id } => {
            document.boards.iter().any(|board| board.id == *board_id)
        }
        MechanicalTarget::Instance { instance_id } => {
            document.hardware.as_ref().is_some_and(|hardware| {
                hardware
                    .instances
                    .iter()
                    .any(|instance| instance.id == *instance_id)
            })
        }
    }
}

fn apply_staged(
    doc: &mut ProjectDoc,
    board_contours: &[BoardContours],
    edit: &MechanicalEdit,
    identities: &MechanicalIdentityLedger,
) -> Result<Vec<String>, String> {
    let board_id = target_board_id(doc, &edit.target)?;
    match &edit.change {
        MechanicalChange::Enable => enable(doc, board_contours, &edit.target, &board_id, edit),
        MechanicalChange::Disable => disable(doc, &edit.target, &board_id, edit),
        MechanicalChange::InitializeClosures => {
            initialize_closures(doc, board_contours, &edit.target, &board_id, edit)
        }
        MechanicalChange::LinkMirrorPair { .. } => link_mirror_pair(doc, &board_id, edit),
        MechanicalChange::UnlinkMirrorPair { .. } => unlink_mirror_pair(doc, &board_id, edit),
        MechanicalChange::RemovePhysicalInstance => {
            remove_physical_instance(doc, board_contours, edit)
        }
        MechanicalChange::MakeFeatureLocal { .. }
        | MechanicalChange::OmitFeatureMember { .. }
        | MechanicalChange::RejoinFeatureMember { .. } => apply_feature_lifecycle(doc, edit),
        change => apply_settings(doc, board_contours, edit, change, &board_id, identities),
    }
}

/// Whether a mechanical change can create, move or remove generated clearance
/// geometry.
///
/// Projection rebuilds the generated closure-hole parts, definitions and
/// memberships. Whenever it is required the caller must supply the normalized
/// mounting-hole template, because the projected holes and their definitions are
/// derived from it. Enable/Disable and closure initialization change the closure
/// mount set itself; openings, mounts and mount collections, closure mounts, the
/// clearance and the mount dimensions that decide a generated hole diameter,
/// gasket support placement/reset and profile changes can all move the holes the
/// projection derives. Pure settings edits (battery, closure hardware, critical
/// fits, hardware text/quantity, process overrides, method/style switches, mount
/// type) leave every generated hole untouched.
///
/// The pair lifecycle belongs on the projection side too: linking, unlinking and
/// the per-feature localize/omit/rejoin actions move mounts, closure mounts,
/// openings and gasket supports into and out of each half's ordinary
/// configuration, so the generated holes derived from those features can appear,
/// disappear or move. Projection therefore always runs for them and the caller
/// must supply the mounting-hole template.
pub fn requires_projection(change: &MechanicalChange) -> bool {
    match change {
        MechanicalChange::Enable
        | MechanicalChange::Disable
        | MechanicalChange::InitializeClosures => true,
        MechanicalChange::AddOpening
        | MechanicalChange::RemoveOpening { .. }
        | MechanicalChange::AddOpeningVertex { .. }
        | MechanicalChange::RemoveOpeningVertex { .. }
        | MechanicalChange::SetOpeningDimension { .. }
        | MechanicalChange::AddMount { .. }
        | MechanicalChange::AddSuggestedClosureMounts(_)
        | MechanicalChange::SetMountCollection { .. }
        | MechanicalChange::AdoptClosurePositions(_)
        | MechanicalChange::SetMountDimension { .. }
        | MechanicalChange::SetMountPosition { .. }
        | MechanicalChange::SetMountKind { .. }
        | MechanicalChange::RemoveMount { .. }
        | MechanicalChange::SetGasketSupportPlacement { .. }
        | MechanicalChange::ResetGasketPlacement
        | MechanicalChange::AssignSwitchProfile { .. }
        | MechanicalChange::AssignStabilizerProfile { .. }
        | MechanicalChange::AssignImportedGeometryProfile { .. }
        | MechanicalChange::UpdateProfile(_)
        | MechanicalChange::RemoveProfile { .. }
        | MechanicalChange::SetSwitchFamily { .. } => true,
        // Linking, unlinking, member removal and the feature lifecycle rewrite each
        // half's ordinary mounts/openings/supports, so generated closure clearance
        // can change shape even though no settings field was edited.
        MechanicalChange::LinkMirrorPair { .. }
        | MechanicalChange::UnlinkMirrorPair { .. }
        | MechanicalChange::RemovePhysicalInstance
        | MechanicalChange::MakeFeatureLocal { .. }
        | MechanicalChange::OmitFeatureMember { .. }
        | MechanicalChange::RejoinFeatureMember { .. } => true,
        MechanicalChange::SetDimension { field, .. } => {
            // Only the clearance changes a generated closure-hole diameter.
            matches!(field, MechanicalDimension::Clearance)
        }
        _ => false,
    }
}

/// Whether a mechanical change can alter the inputs the outline composer reads.
///
/// Pure settings edits leave every outline input untouched, so the committed
/// outline cache is reused. Changes that can create, move or remove generated
/// parts/definitions or closure geometry - enable/disable, closure
/// initialization, openings, mounts and mount collections, closure clearance and
/// the dimensions that change generated holes, gasket support placement/reset and
/// profiles - feed the outline composer and must recompute. This is exactly the
/// projection classification: those are the changes that can alter the generated
/// clearances the composer reads.
///
/// The pair lifecycle shares that classification: it changes the ordinary
/// mounts/openings/supports the generated clearance geometry is derived from, so
/// the composer's generated parts/definitions can change and the outline cache
/// must recompute.
pub fn affects_outline(change: &MechanicalChange) -> bool {
    requires_projection(change)
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod identity_merge_tests;
