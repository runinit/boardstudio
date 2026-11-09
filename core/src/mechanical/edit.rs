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

use super::change::{
    MechanicalChange, MechanicalCriticalFitTextField, MechanicalDimension, MechanicalEdit,
    MechanicalHardwareTextField, MechanicalMountCollection, MechanicalTarget,
};
use super::features;
use super::inputs::{effective_document, effective_inputs};
use super::{inferred_switch_family, is_switch_position};
use std::collections::BTreeMap;

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
pub(crate) fn apply_with_ledger(
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
        MechanicalChange::MaterializeLegacyConstruction { .. } => {
            materialize_legacy_construction(doc, &board_id, edit)
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
/// Legacy conversion is deliberately absent: it materializes each selected member's
/// current effective values without changing any closure mount, so the generated
/// clearance geometry it derives is unchanged and no mounting-hole template is needed.
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

fn apply_settings(
    doc: &mut ProjectDoc,
    board_contours: &[BoardContours],
    edit: &MechanicalEdit,
    change: &MechanicalChange,
    board_id: &str,
    identities: &MechanicalIdentityLedger,
) -> Result<Vec<String>, String> {
    // A supplied template is validated even when the edit is settings-only and the
    // projection never runs, so a malformed catalogue entry is always an error.
    if let Some(template) = edit.mounting_hole.as_deref() {
        validate_mounting_hole(template)?;
    }
    let inputs = effective_inputs(doc, &[], &edit.target)?;
    let mut configuration = inputs.configuration;
    let before = configuration.clone();
    let mut changed = Vec::new();
    let identity_floor =
        reserved_identity_floor(doc, &edit.target).merged(identities.floors(&edit.target));
    apply_change(
        &mut configuration,
        change,
        doc,
        board_id,
        identity_floor,
        &mut changed,
    )?;
    // Closure mounts are initialized whenever the resolved stack still has none
    // and the mount is not a gasket; explicit `Some(vec![])` is deliberate.
    let needs_initialization =
        configuration.closure_mounts.is_none() && configuration.mount != MechanicalMount::Gasket;
    if configuration == before && !needs_initialization {
        return Ok(Vec::new());
    }
    // Whether projection needs a template is decided by the actual generated holes,
    // not by the change kind: removing the final generated hole creates nothing.
    let projection_required = requires_projection(change) || needs_initialization;
    changed.insert(0, owner_id(&edit.target, board_id));
    if needs_initialization {
        persist(doc, &edit.target, configuration.clone())?;
        let suggested = initialize_target_closures(
            doc,
            board_contours,
            &edit.target,
            &configuration,
            &mut changed,
        )?;
        configuration.closure_mounts = Some(suggested);
    }
    // Route each changed value to the ownership that owns it: the pair's single
    // authored value for linked features, the pair or shared source for shared
    // construction, and the member's own configuration for everything local.
    persist_edit(doc, &edit.target, &before, configuration, &mut changed)?;
    if projection_required {
        project(doc, edit.mounting_hole.as_deref(), &mut changed)?;
    }
    dedup_changed(&mut changed);
    Ok(changed)
}

/// Runs the owned projection and records only the generated ids it changed.
///
/// The optional template is validated by [`validate_mounting_hole`] before use. No
/// template is required when the projected hole set is empty (for example when the
/// final generated hole is removed): the old generated parts, definitions and
/// memberships are still cleaned up.
fn project(
    doc: &mut ProjectDoc,
    mounting_hole: Option<&PartDefinition>,
    changed: &mut Vec<String>,
) -> Result<(), String> {
    if let Some(template) = mounting_hole {
        validate_mounting_hole(template)?;
    }
    let before = doc.clone();
    let projected =
        crate::mechanical::clearance::project_owned_closure_clearance_with_optional_template(
            before.clone(),
            mounting_hole,
        )?;
    changed.extend(generated_change_ids(&before, &projected));
    *doc = projected;
    Ok(())
}

/// Validates a supplied normalized mounting-hole definition.
///
/// The projection derives its generated footprints from the bundled
/// `ceoloide/mounting_hole_npth` entry, so a definition from another source or one
/// without a drilled pad is rejected before anything mutates.
fn validate_mounting_hole(template: &PartDefinition) -> Result<(), String> {
    let source = template
        .generator
        .as_ref()
        .map(|generator| generator.source.as_str());
    if source != Some("ceoloide/mounting_hole_npth") {
        return Err(
            "The mounting-hole definition must be the normalized ceoloide/mounting_hole_npth catalogue entry."
                .into(),
        );
    }
    if !template.pads.iter().any(|pad| pad.drill.is_some()) {
        return Err("The mounting-hole definition has no drilled pad.".into());
    }
    Ok(())
}

/// Resolves the assembly from the proposed document and materializes its
/// suggested closure mounts onto the proposed configuration.
///
/// The caller must already have persisted the proposed configuration into the
/// target slot so [`effective_document`] resolves the same stack the user is
/// configuring. The result is the resolved effective configuration, matching the
/// adapter's `prepare_document`: initialization adopts the effective value the
/// resolver returned.
fn initialize_closure_mounts(
    doc: &ProjectDoc,
    board_contours: &[BoardContours],
    target: &MechanicalTarget,
    configuration: &MechanicalConfiguration,
) -> Result<Vec<Mount>, String> {
    let (effective, recomputed) = effective_document(doc, target)?;
    // The accepted outline projection is the geometry the user saw, so absent
    // closure mounts are proposed from it rather than from a fresh recompute of the
    // proposed document. Falling back to the recomputed contours keeps callers that
    // have no accepted projection (or a board without one) working.
    let accepted = accepted_target_contours(doc, board_contours, target);
    let contours = accepted.unwrap_or(recomputed);
    let assembly = crate::mechanical::resolve(&effective, &contours);
    let height = configuration.plate_to_pcb
        + configuration.pcb_thickness
        + configuration
            .bottom_foam_thickness
            .max(configuration.battery_height);
    // Only the suggested mounts come from the resolved assembly: the caller's edited
    // configuration is what gets persisted, so initializing cannot drop the field edits
    // the same operation just made (a legacy half's shared field, for example).
    Ok(assembly
        .suggested_mounts
        .into_iter()
        .map(|mut mount| {
            mount.id = format!("{AUTO_CLOSURE_PREFIX}{}", mount.id);
            mount.kind = MountKind::Boss;
            mount.hole_diameter = 2.2;
            mount.height = Some(height);
            mount
        })
        .collect())
}

/// The closure list an initialization stores on the target's own configuration.
///
/// A half outside a pair proposes from its own outline, as before. A paired half never
/// proposes for itself: the pair's closure mounts are proposed once, from its construction
/// source, and linked to the pair (see [`link_pair_closure_proposals`]), so the half stores an
/// empty list of its own. `changed` gains the pair's ids when the pair gains closure mounts.
fn initialize_target_closures(
    doc: &mut ProjectDoc,
    board_contours: &[BoardContours],
    target: &MechanicalTarget,
    configuration: &MechanicalConfiguration,
    changed: &mut Vec<String>,
) -> Result<Vec<Mount>, String> {
    if let Some(pair_index) = paired_index(doc, target) {
        if link_pair_closure_proposals(doc, board_contours, pair_index)? {
            changed.extend(pair_owner_ids(doc, pair_index));
        }
        return Ok(Vec::new());
    }
    initialize_closure_mounts(doc, board_contours, target, configuration)
}

/// Proposes a pair's closure mounts once, from its construction source, and links them.
///
/// The construction source is the pair's recorded `constructionSource`. A pair linked before
/// format 5 records none, and its first member is the source. When the source already has a
/// closure list, that list becomes the pair's linked mounts; otherwise the source's own outline
/// proposes them. The source then stores an empty list of its own, so the same mounts are never
/// stored twice. The other half's stored closure list is left as it is: it is hardware that half
/// carries, and linking does not delete it. A pair that already has closure mounts proposes
/// nothing, and neither does a pair whose source is a gasket. Returns whether mounts were linked.
fn link_pair_closure_proposals(
    doc: &mut ProjectDoc,
    board_contours: &[BoardContours],
    pair_index: usize,
) -> Result<bool, String> {
    let pair = doc
        .hardware
        .as_ref()
        .and_then(|hardware| hardware.pairs.get(pair_index))
        .ok_or("The mechanical pair is unavailable.")?;
    if pair
        .features
        .iter()
        .any(|feature| feature.kind == MechanicalFeatureKind::ClosureMount)
    {
        return Ok(false);
    }
    let source = pair
        .construction_source
        .clone()
        .unwrap_or_else(|| pair.members[0].clone());
    let target = MechanicalTarget::Instance {
        instance_id: source.clone(),
    };
    let configuration = effective_inputs(doc, &[], &target)?.configuration;
    if configuration.mount == MechanicalMount::Gasket {
        return Ok(false);
    }
    let mounts = match configuration.closure_mounts.clone() {
        Some(mounts) => mounts,
        None => initialize_closure_mounts(doc, board_contours, &target, &configuration)?,
    };
    instance_configuration_mut(doc, &source)?.closure_mounts = Some(Vec::new());
    let pair = &mut doc
        .hardware
        .as_mut()
        .ok_or("Physical instances are unavailable")?
        .pairs[pair_index];
    for mount in mounts {
        let id = mount.id.clone();
        features::add_linked_feature(
            pair,
            MechanicalFeature {
                id,
                kind: MechanicalFeatureKind::ClosureMount,
                author: source.clone(),
                value: MechanicalFeatureValue::Mount { mount },
                members: Vec::new(),
            },
        )?;
    }
    Ok(true)
}

/// The pair an instance target belongs to, by its index in the document's pairs.
fn paired_index(doc: &ProjectDoc, target: &MechanicalTarget) -> Option<usize> {
    match target {
        MechanicalTarget::Instance { instance_id } => pair_index(doc, instance_id),
        MechanicalTarget::Board { .. } => None,
    }
}

/// Whether an instance's effective stack has no closure list and is not a gasket, so its
/// closure mounts still need initialization.
fn closure_list_absent(doc: &ProjectDoc, target: &MechanicalTarget) -> bool {
    effective_inputs(doc, &[], target).is_ok_and(|inputs| {
        inputs.configuration.closure_mounts.is_none()
            && inputs.configuration.mount != MechanicalMount::Gasket
    })
}

/// The accepted contours of the edit target's board, mirrored for a flipped half.
///
/// `None` when the accepted projection has no entry for that board, so initialization
/// falls back to the contours recomputed from the proposed document.
fn accepted_target_contours(
    document: &ProjectDoc,
    board_contours: &[BoardContours],
    target: &MechanicalTarget,
) -> Option<Vec<Contour>> {
    let (board_id, flipped) = match target {
        MechanicalTarget::Board { board_id } => (board_id.as_str(), false),
        MechanicalTarget::Instance { instance_id } => {
            let instance = document
                .hardware
                .as_ref()?
                .instances
                .iter()
                .find(|instance| instance.id == *instance_id)?;
            (instance.board_id.as_str(), instance.flipped)
        }
    };
    let declared = board_contours
        .iter()
        .find(|entry| entry.board_id == board_id)?
        .contours
        .clone();
    Some(if flipped {
        super::inputs::mirror_contours(declared)
    } else {
        declared
    })
}

/// The generated part ids, generated definition ids and membership owners that
/// differ between two documents.
fn generated_change_ids(before: &ProjectDoc, after: &ProjectDoc) -> Vec<String> {
    let mut ids = std::collections::BTreeSet::new();
    for part in &after.parts {
        if part.id.starts_with(GENERATED_PART_PREFIX)
            && before.parts.iter().find(|entry| entry.id == part.id) != Some(part)
        {
            ids.insert(part.id.clone());
        }
    }
    for part in &before.parts {
        if part.id.starts_with(GENERATED_PART_PREFIX)
            && !after.parts.iter().any(|entry| entry.id == part.id)
        {
            ids.insert(part.id.clone());
        }
    }
    for definition in &after.definitions {
        if definition.id.starts_with(GENERATED_DEFINITION_PREFIX)
            && before
                .definitions
                .iter()
                .find(|entry| entry.id == definition.id)
                != Some(definition)
        {
            ids.insert(definition.id.clone());
        }
    }
    for definition in &before.definitions {
        if definition.id.starts_with(GENERATED_DEFINITION_PREFIX)
            && !after
                .definitions
                .iter()
                .any(|entry| entry.id == definition.id)
        {
            ids.insert(definition.id.clone());
        }
    }
    let generated_set = |part_ids: Option<&Vec<String>>| {
        part_ids
            .into_iter()
            .flatten()
            .filter(|id| id.starts_with(GENERATED_PART_PREFIX))
            .cloned()
            .collect::<std::collections::BTreeSet<_>>()
    };
    for board in &after.boards {
        let before_ids = before
            .boards
            .iter()
            .find(|entry| entry.id == board.id)
            .map(|entry| &entry.part_ids);
        if generated_set(before_ids) != generated_set(Some(&board.part_ids)) {
            ids.insert(board.id.clone());
        }
    }
    for layout in &after.layouts {
        let before_ids = before
            .layouts
            .iter()
            .find(|entry| entry.id == layout.id)
            .map(|entry| &entry.part_ids);
        if generated_set(before_ids) != generated_set(Some(&layout.part_ids)) {
            ids.insert(layout.id.clone());
        }
    }
    ids.into_iter().collect()
}

fn dedup_changed(changed: &mut Vec<String>) {
    let mut seen = std::collections::BTreeSet::new();
    changed.retain(|id| seen.insert(id.clone()));
}

/// Resolves an edit target to its board id and optional physical instance.
pub(crate) fn resolve_target<'a>(
    doc: &'a ProjectDoc,
    target: &MechanicalTarget,
) -> Result<(String, Option<&'a PhysicalBoardInstance>), String> {
    match target {
        MechanicalTarget::Board { board_id } => Ok((board_id.clone(), None)),
        MechanicalTarget::Instance { instance_id } => {
            let hardware = doc
                .hardware
                .as_ref()
                .ok_or("Physical instances are unavailable")?;
            let instance = hardware
                .instances
                .iter()
                .find(|instance| instance.id == *instance_id)
                .ok_or("The selected physical instance is unavailable")?;
            Ok((instance.board_id.clone(), Some(instance)))
        }
    }
}

fn target_board_id(doc: &ProjectDoc, target: &MechanicalTarget) -> Result<String, String> {
    resolve_target(doc, target).map(|(board_id, _)| board_id)
}

fn owner_id(target: &MechanicalTarget, board_id: &str) -> String {
    match target {
        MechanicalTarget::Board { .. } => board_id.to_string(),
        MechanicalTarget::Instance { instance_id } => instance_id.clone(),
    }
}

/// Writes the effective configuration back to the target's stored slot.
///
/// Canonical edits write `doc.mechanical`; instance edits write that instance's
/// `mechanical`. The stored value is the effective configuration the patch
/// produced, matching the adapter's `prepare_document` + `persist_configuration`
/// sequence.
fn persist(
    doc: &mut ProjectDoc,
    target: &MechanicalTarget,
    configuration: MechanicalConfiguration,
) -> Result<(), String> {
    match target {
        MechanicalTarget::Board { board_id } => {
            if !doc.boards.iter().any(|board| board.id == *board_id) {
                return Err("Board is unavailable".into());
            }
            doc.mechanical = Some(configuration);
            Ok(())
        }
        MechanicalTarget::Instance { instance_id } => {
            let hardware = doc
                .hardware
                .as_mut()
                .ok_or("Physical instances are unavailable")?;
            let instance = hardware
                .instances
                .iter_mut()
                .find(|instance| instance.id == *instance_id)
                .ok_or("The selected physical instance is unavailable")?;
            instance.mechanical = Some(configuration);
            Ok(())
        }
    }
}

/// Persists an edited effective configuration to the ownership that owns each part.
///
/// A linked feature belongs to its pair, so a change made from either member updates
/// the pair's single authored value (stored in the editing member's frame) and never
/// a second writable copy on the member. An ordinary delete of a linked feature is the
/// linked deletion: it removes both counterparts and the relationship itself, while an
/// explicit omit stores a one-half exception. A `Pair` member's shared construction
/// change is written to the pair, a `Legacy` member's overlay change is written to the
/// shared source so the next effective read returns it, and every other field stays
/// local.
fn persist_edit(
    doc: &mut ProjectDoc,
    target: &MechanicalTarget,
    before: &MechanicalConfiguration,
    after: MechanicalConfiguration,
    changed: &mut Vec<String>,
) -> Result<(), String> {
    let mut local = after.clone();
    if let MechanicalTarget::Instance { instance_id } = target {
        let pair_index = doc.hardware.as_ref().and_then(|hardware| {
            hardware
                .pairs
                .iter()
                .position(|pair| pair.members.contains(instance_id))
        });
        match pair_index {
            Some(pair_index) => {
                // A change to a linked feature or to the pair's shared construction
                // changes the derived geometry both halves show, so the report names the
                // pair and both member instances.
                let mut pair_touched = false;
                let pair = doc.hardware.as_ref().unwrap().pairs[pair_index].clone();
                for feature in &pair.features {
                    let Some(member) = feature
                        .members
                        .iter()
                        .find(|member| member.instance_id == *instance_id)
                    else {
                        continue;
                    };
                    if member.state == MechanicalFeatureState::Linked {
                        let after_value =
                            find_feature_value(&after, feature.kind, &member.feature_id);
                        let before_value =
                            find_feature_value(before, feature.kind, &member.feature_id);
                        if after_value != before_value {
                            pair_touched = true;
                            let pair = &mut doc.hardware.as_mut().unwrap().pairs[pair_index];
                            match after_value {
                                Some(value) => {
                                    // The editing member's frame authors the pair's
                                    // single value; the other half derives from it.
                                    let target = pair
                                        .features
                                        .iter_mut()
                                        .find(|entry| entry.id == feature.id)
                                        .ok_or(
                                            "The mechanical feature is no longer in its pair.",
                                        )?;
                                    target.value = value;
                                    target.author = instance_id.clone();
                                }
                                None => {
                                    // Ordinary delete of a linked feature removes both
                                    // counterparts: the relationship itself goes. An
                                    // explicit omit is what keeps a one-half exception.
                                    features::remove_linked_feature(pair, &feature.id)?;
                                }
                            }
                        }
                    }
                    // A linked feature is never persisted on the member; a `Local`
                    // exception stays in its ordinary configuration.
                    if member.state != MechanicalFeatureState::Local {
                        remove_feature(&mut local, feature.kind, &member.feature_id);
                    }
                }
                // Generic additions on a paired half join the pair by default, so both
                // halves show the new feature and neither keeps a private copy. Only
                // explicit exceptions (Make local) stay local.
                for kind in [
                    MechanicalFeatureKind::Mount,
                    MechanicalFeatureKind::ClosureMount,
                    MechanicalFeatureKind::Opening,
                    MechanicalFeatureKind::GasketSupport,
                ] {
                    let existing = feature_ids(before, kind);
                    let added: Vec<String> = feature_ids(&after, kind)
                        .into_iter()
                        .filter(|id| !existing.contains(id))
                        .collect();
                    for id in added {
                        let Some(value) = find_feature_value(&after, kind, &id) else {
                            continue;
                        };
                        let pair = &mut doc.hardware.as_mut().unwrap().pairs[pair_index];
                        if pair.features.iter().any(|feature| feature.id == id) {
                            continue;
                        }
                        pair_touched = true;
                        features::add_linked_feature(
                            pair,
                            MechanicalFeature {
                                id: id.clone(),
                                kind,
                                author: instance_id.clone(),
                                value,
                                members: Vec::new(),
                            },
                        )?;
                        remove_feature(&mut local, kind, &id);
                    }
                }
                let before_shared = features::shared_construction(before);
                let after_shared = features::shared_construction(&after);
                if before_shared != after_shared {
                    pair_touched = true;
                    doc.hardware.as_mut().unwrap().pairs[pair_index].construction = after_shared;
                }
                if pair_touched {
                    changed.extend(pair_owner_ids(doc, pair_index));
                }
            }
            None => {
                let ownership = doc
                    .hardware
                    .as_ref()
                    .and_then(|hardware| {
                        hardware
                            .instances
                            .iter()
                            .find(|instance| instance.id == *instance_id)
                    })
                    .map(|instance| instance.construction_ownership);
                let before_shared = features::shared_construction(before);
                let after_shared = features::shared_construction(&after);
                if ownership == Some(MechanicalConstructionOwnership::Legacy)
                    && before_shared != after_shared
                    && let Some(hardware) = doc.hardware.as_mut()
                {
                    // Legacy reads use the explicit shared construction, or the
                    // first configured instance when it is absent. Write the same
                    // owner so the next effective read retains the edit.
                    if let Some(shared) = hardware.shared_construction.as_mut() {
                        features::apply_construction(shared, &after_shared);
                    } else if let Some((source_id, shared)) =
                        hardware.instances.iter_mut().find_map(|instance| {
                            instance
                                .mechanical
                                .as_mut()
                                .map(|configuration| (instance.id.clone(), configuration))
                        })
                    {
                        features::apply_construction(shared, &after_shared);
                        changed.push(source_id);
                    }
                }
            }
        }
    }
    persist(doc, target, local)
}

/// Enables a target that has no effective configuration yet.
///
/// The fresh configuration is closure-initialized and projected like any other
/// change, because its `closure_mounts` starts absent.
fn enable(
    doc: &mut ProjectDoc,
    board_contours: &[BoardContours],
    target: &MechanicalTarget,
    board_id: &str,
    edit: &MechanicalEdit,
) -> Result<Vec<String>, String> {
    // A target that already carries its own effective configuration is unchanged;
    // the edit reports nothing. A board with no stored configuration is still
    // eligible and gets the Core-owned defaults, so eligibility is not the same as
    // "has stored settings" for a canonical board.
    let already_effective = match target {
        MechanicalTarget::Board { board_id } => doc
            .mechanical
            .as_ref()
            .is_some_and(|configuration| configuration.board_id == *board_id),
        MechanicalTarget::Instance { .. } => effective_inputs(doc, &[], target).is_ok(),
    };
    if already_effective {
        // A legacy second half resolves from the shared source but has no closure list of its
        // own, so enabling it still initializes its closure mounts (generator-parameters
        // ticket 06, finding B). Any other effective half is unchanged.
        if matches!(target, MechanicalTarget::Instance { .. }) && closure_list_absent(doc, target) {
            return initialize_closures(doc, board_contours, target, board_id, edit);
        }
        return Ok(Vec::new());
    }
    if let Some(template) = edit.mounting_hole.as_deref() {
        validate_mounting_hole(template)?;
    }
    // The fresh configuration carries the Core defaults (battery, stabilizers and the
    // rest), exactly as the effective read would resolve it, so enabling and the
    // initialization that follows persist the same configuration the adapter shows.
    let mut configuration = super::inputs::apply_mechanical_defaults(
        doc,
        default_configuration(doc, board_id)?,
        board_id,
        false,
    );
    let mut changed = vec![owner_id(target, board_id)];
    persist(doc, target, configuration.clone())?;
    let suggested =
        initialize_target_closures(doc, board_contours, target, &configuration, &mut changed)?;
    configuration.closure_mounts = Some(suggested);
    persist(doc, target, configuration)?;
    project(doc, edit.mounting_hole.as_deref(), &mut changed)?;
    dedup_changed(&mut changed);
    Ok(changed)
}

/// Clears the target's stored configuration; absent configuration is a no-op.
///
/// Disabling can remove generated closure geometry, so it is projection-required
/// and must remove the stale generated clearance parts in the same edit.
fn disable(
    doc: &mut ProjectDoc,
    target: &MechanicalTarget,
    board_id: &str,
    edit: &MechanicalEdit,
) -> Result<Vec<String>, String> {
    let present = match target {
        MechanicalTarget::Board { .. } => doc
            .mechanical
            .as_ref()
            .is_some_and(|configuration| configuration.board_id == *board_id),
        MechanicalTarget::Instance { instance_id } => {
            doc.hardware
                .as_ref()
                .and_then(|hardware| {
                    hardware
                        .instances
                        .iter()
                        .find(|instance| instance.id == *instance_id)
                })
                .is_some_and(|instance| instance.mechanical.is_some())
                || pair_index(doc, instance_id).is_some()
        }
    };
    if !present {
        return Ok(Vec::new());
    }
    if let Some(template) = edit.mounting_hole.as_deref() {
        validate_mounting_hole(template)?;
    }
    let mut changed = Vec::new();
    // Disabling a paired half is the explicit "unlink and disable this half": both
    // halves keep the model they showed, the pair goes, and only the selected half
    // loses its mechanical stack. The user label and confirmation belong to the UI.
    let member_pair = match target {
        MechanicalTarget::Instance { instance_id } => pair_index(doc, instance_id),
        MechanicalTarget::Board { .. } => None,
    };
    if let Some(pair_index) = member_pair {
        let (pair_id, members) = dissolve_pair_ownership(doc, pair_index)?;
        changed.push(pair_id);
        changed.extend(members);
    }
    match target {
        MechanicalTarget::Board { .. } => doc.mechanical = None,
        MechanicalTarget::Instance { instance_id } => {
            let hardware = doc
                .hardware
                .as_mut()
                .ok_or("Physical instances are unavailable")?;
            let instance = hardware
                .instances
                .iter_mut()
                .find(|instance| instance.id == *instance_id)
                .ok_or("The selected physical instance is unavailable")?;
            // The reservations outlive the removed stack, so re-enabling this half can
            // never hand a removed opening or vertex identity to a later addition.
            if let Some(configuration) = instance.mechanical.as_ref() {
                let reservation = identity_reservation_of(configuration);
                instance.mechanical_identity_reservation =
                    Some(match instance.mechanical_identity_reservation {
                        Some(existing) => existing.merged(reservation),
                        None => reservation,
                    });
            }
            instance.mechanical = None;
        }
    }
    changed.push(owner_id(target, board_id));
    project(doc, edit.mounting_hole.as_deref(), &mut changed)?;
    dedup_changed(&mut changed);
    Ok(changed)
}

/// Initializes absent closure mounts from the resolved assembly and projects them.
///
/// An already-present collection or a gasket mount is not eligible and reports the
/// adapter's message; explicit `Some(vec![])` therefore stays deliberate.
fn initialize_closures(
    doc: &mut ProjectDoc,
    board_contours: &[BoardContours],
    target: &MechanicalTarget,
    board_id: &str,
    edit: &MechanicalEdit,
) -> Result<Vec<String>, String> {
    let inputs = effective_inputs(doc, &[], target)?;
    if inputs.configuration.closure_mounts.is_some()
        || inputs.configuration.mount == MechanicalMount::Gasket
    {
        return Err("The current stack no longer needs mounting-location initialization.".into());
    }
    if let Some(template) = edit.mounting_hole.as_deref() {
        validate_mounting_hole(template)?;
    }
    let mut changed = vec![owner_id(target, board_id)];
    // A paired half's effective stack carries the pair's linked features. Only its own stored
    // configuration is written back, so those features are not copied into it.
    let mut configuration = match target {
        MechanicalTarget::Instance { instance_id } if paired_index(doc, target).is_some() => {
            instance_configuration_mut(doc, instance_id)?.clone()
        }
        _ => inputs.configuration,
    };
    persist(doc, target, configuration.clone())?;
    let suggested =
        initialize_target_closures(doc, board_contours, target, &configuration, &mut changed)?;
    configuration.closure_mounts = Some(suggested);
    persist(doc, target, configuration)?;
    project(doc, edit.mounting_hole.as_deref(), &mut changed)?;
    dedup_changed(&mut changed);
    Ok(changed)
}

/// Establishes an explicit mechanical mirror pair between two physical halves.
///
/// Both members must exist and must not already belong to a pair, the mapping must
/// be finite, the construction source must be one of the members and the pair id
/// must be new. The pair takes the construction source's shared subset and one
/// linked feature per reviewed correspondence. Each correspondence chooses its own
/// source half (`source_instance_id`, defaulting to the first member), whose accepted
/// effective geometry becomes the single authored value; the other half derives the
/// exact inverse. Both members become `Pair`-owned; a missing stored configuration
/// is materialized first so pair ownership always has a valid local base for the
/// fields the pair does not own.
fn link_mirror_pair(
    doc: &mut ProjectDoc,
    _board_id: &str,
    edit: &MechanicalEdit,
) -> Result<Vec<String>, String> {
    let MechanicalChange::LinkMirrorPair {
        pair_id,
        members,
        mapping,
        construction_source,
        correspondences,
    } = &edit.change
    else {
        return Err("The link operation was dispatched with another change.".into());
    };
    let hardware = doc
        .hardware
        .as_ref()
        .ok_or("Physical instances are unavailable")?;
    if hardware.pairs.iter().any(|pair| pair.id == *pair_id) {
        return Err(format!(
            "A mechanical pair named {pair_id:?} already exists."
        ));
    }
    let [first, second] = members;
    if first == second {
        return Err("A mechanical pair requires two distinct physical instances.".into());
    }
    for member in members {
        // The members may belong to different boards: one reversible PCB used twice
        // and distinct left/right PCB designs are both supported explicit pairs. The
        // map is supplied, never inferred from names, coordinates or flip flags.
        if !hardware.instances.iter().any(|entry| entry.id == *member) {
            return Err(format!("Physical instance {member:?} is unavailable."));
        }
        if hardware
            .pairs
            .iter()
            .any(|pair| pair.members.iter().any(|candidate| candidate == member))
        {
            return Err(format!(
                "Physical instance {member:?} already belongs to a mechanical pair."
            ));
        }
    }
    if !mapping.at.x.is_finite() || !mapping.at.y.is_finite() || !mapping.rotation.is_finite() {
        return Err("The mechanical pair mapping must be finite.".into());
    }
    if !members.contains(construction_source) {
        return Err(format!(
            "The construction source {construction_source:?} is not a pair member."
        ));
    }

    // Resolve both halves before anything mutates: their effective
    // configurations give the source's shared construction and each half's
    // local features.
    let configurations = [
        effective_inputs(
            doc,
            &[],
            &MechanicalTarget::Instance {
                instance_id: members[0].clone(),
            },
        )?
        .configuration,
        effective_inputs(
            doc,
            &[],
            &MechanicalTarget::Instance {
                instance_id: members[1].clone(),
            },
        )?
        .configuration,
    ];
    let source_index = usize::from(construction_source != first);
    let construction = features::shared_construction(&configurations[source_index]);

    let mut pair = MechanicalMirrorPair {
        id: pair_id.clone(),
        members: members.clone(),
        mapping: *mapping,
        construction,
        construction_source: Some(construction_source.clone()),
        halves: None,
        features: Vec::new(),
    };
    for correspondence in correspondences {
        // Each reviewed match chooses its own source half; absent choice keeps the
        // legacy first-member source. Its accepted effective geometry is read before
        // anything mutates and becomes the pair's single authored value.
        let source_id = correspondence
            .source_instance_id
            .as_deref()
            .unwrap_or(first.as_str());
        let source_index = members
            .iter()
            .position(|member| member == source_id)
            .ok_or_else(|| {
                format!(
                    "The reviewed source {source_id:?} is not a source member of the mechanical pair."
                )
            })?;
        let (source_feature_id, other_feature_id) = if source_index == 0 {
            (
                &correspondence.first_feature_id,
                &correspondence.second_feature_id,
            )
        } else {
            (
                &correspondence.second_feature_id,
                &correspondence.first_feature_id,
            )
        };
        let value = find_feature_value(
            &configurations[source_index],
            correspondence.kind,
            source_feature_id,
        )
        .ok_or_else(|| {
            format!(
                "Physical instance {source_id:?} has no {:?} feature {source_feature_id:?}.",
                correspondence.kind
            )
        })?;
        if find_feature_value(
            &configurations[1 - source_index],
            correspondence.kind,
            other_feature_id,
        )
        .is_none()
        {
            return Err(format!(
                "Physical instance {:?} has no {:?} feature {other_feature_id:?}.",
                members[1 - source_index],
                correspondence.kind
            ));
        }
        features::add_linked_feature(
            &mut pair,
            MechanicalFeature {
                id: correspondence.first_feature_id.clone(),
                kind: correspondence.kind,
                author: source_id.to_string(),
                value,
                members: Vec::new(),
            },
        )?;
    }

    // The second half shows the reflected counterpart of the reviewed source
    // geometry; the first half keeps its authored value in place.
    let mut second_installs = Vec::new();
    for feature in &pair.features {
        if let Some(derived) = features::counterpart(&pair, feature, first)? {
            second_installs.push((feature.kind, derived.feature_id, derived.value));
        }
    }

    let hardware = doc
        .hardware
        .as_mut()
        .ok_or("Physical instances are unavailable")?;
    for (index, member) in members.iter().enumerate() {
        let instance = hardware
            .instances
            .iter_mut()
            .find(|entry| entry.id == *member)
            .ok_or("Physical instance is unavailable")?;
        if instance.mechanical.is_none() {
            instance.mechanical = Some(configurations[index].clone());
        }
        instance.construction_ownership = MechanicalConstructionOwnership::Pair;
    }
    {
        for correspondence in correspondences {
            for (index, member) in members.iter().enumerate() {
                let feature_id = if index == 0 {
                    correspondence.first_feature_id.as_str()
                } else {
                    correspondence.second_feature_id.as_str()
                };
                let configuration = hardware
                    .instances
                    .iter_mut()
                    .find(|entry| &entry.id == member)
                    .and_then(|instance| instance.mechanical.as_mut())
                    .ok_or("Physical instance is unavailable")?;
                remove_feature(configuration, correspondence.kind, feature_id);
            }
        }
    }
    hardware.pairs.push(pair);
    // The document's own rules decide whether the reviewed link is consistent.
    features::validate(doc)?;

    let mut changed = vec![pair_id.clone(), members[0].clone(), members[1].clone()];
    project(doc, edit.mounting_hole.as_deref(), &mut changed)?;
    dedup_changed(&mut changed);
    Ok(changed)
}

/// Removes one physical half from its pair, preserving the survivor's visible model.
///
/// The target must name the removed half. The survivor's accepted effective model -
/// its pair-derived geometry, shared construction and hardware-specific values - is
/// materialized into its own configuration first, so nothing it showed is lost, and
/// then the pair is cleared and the member removed with its references. Generated
/// closure holes and memberships refresh in the same atomic edit.
fn remove_physical_instance(
    doc: &mut ProjectDoc,
    board_contours: &[BoardContours],
    edit: &MechanicalEdit,
) -> Result<Vec<String>, String> {
    let MechanicalTarget::Instance { instance_id } = &edit.target else {
        return Err(
            "Removing a physical instance needs a physical-instance target; a board target is malformed."
                .into(),
        );
    };
    if let Some(template) = edit.mounting_hole.as_deref() {
        validate_mounting_hole(template)?;
    }
    let (pair_index, survivor_id) = pair_membership(doc, instance_id)?;
    // The survivor's visible model is read from the accepted state before the pair is
    // cleared, so materializing it cannot lose pair-derived geometry or construction.
    let survivor = effective_inputs(
        doc,
        board_contours,
        &MechanicalTarget::Instance {
            instance_id: survivor_id.clone(),
        },
    )?;
    let pair_id = doc.hardware.as_ref().unwrap().pairs[pair_index].id.clone();
    {
        let hardware = doc
            .hardware
            .as_mut()
            .ok_or("Physical instances are unavailable")?;
        hardware.pairs.remove(pair_index);
        let entry = hardware
            .instances
            .iter_mut()
            .find(|instance| instance.id == survivor_id)
            .ok_or("The surviving physical instance is unavailable")?;
        entry.mechanical = Some(survivor.configuration);
        entry.construction_ownership = MechanicalConstructionOwnership::Local;
        hardware
            .instances
            .retain(|instance| instance.id != *instance_id);
    }
    if doc.physical_instance_id.as_deref() == Some(instance_id.as_str()) {
        doc.physical_instance_id = None;
    }
    let mut changed = vec![pair_id, survivor_id, instance_id.clone()];
    project(doc, edit.mounting_hole.as_deref(), &mut changed)?;
    dedup_changed(&mut changed);
    Ok(changed)
}

/// Converts selected legacy-ownership members to local ownership without changing
/// what they show.
///
/// Every selected member must exist on the target board, be selected once and still use
/// the legacy shared construction; anything else is an error with no mutation. Each
/// member's current effective values - including the hardware, critical fits and
/// profiles the legacy overlay supplied - are materialized into its own configuration
/// before its ownership becomes `Local`. When the remaining legacy members resolve their
/// shared subset through a converted member (the first-instance fallback), the prior
/// common construction is captured as the legacy shared source first, so their baseline
/// is preserved and later edits to the converted member cannot feed back into it.
fn materialize_legacy_construction(
    doc: &mut ProjectDoc,
    board_id: &str,
    edit: &MechanicalEdit,
) -> Result<Vec<String>, String> {
    let MechanicalChange::MaterializeLegacyConstruction { instance_ids } = &edit.change else {
        return Err("The conversion operation was dispatched with another change.".into());
    };
    if instance_ids.is_empty() {
        return Err("Select at least one physical instance to convert.".into());
    }
    let mut seen = std::collections::BTreeSet::new();
    for instance_id in instance_ids {
        if !seen.insert(instance_id) {
            return Err(format!(
                "Physical instance {instance_id:?} is selected more than once."
            ));
        }
    }
    {
        let hardware = doc
            .hardware
            .as_ref()
            .ok_or("Physical instances are unavailable")?;
        for instance_id in instance_ids {
            let instance = hardware
                .instances
                .iter()
                .find(|instance| instance.id == *instance_id)
                .ok_or_else(|| format!("Physical instance {instance_id:?} is unavailable."))?;
            if instance.board_id != board_id {
                return Err(format!(
                    "Physical instance {instance_id:?} belongs to another board."
                ));
            }
            if instance.construction_ownership != MechanicalConstructionOwnership::Legacy {
                return Err(format!(
                    "Physical instance {instance_id:?} does not use the legacy shared construction."
                ));
            }
        }
    }
    // The prior common construction is captured before conversion when the remaining
    // legacy members would otherwise fall back to a converted member's configuration.
    let captured_source = {
        let hardware = doc.hardware.as_ref().unwrap();
        if hardware.shared_construction.is_some() {
            None
        } else {
            hardware
                .instances
                .iter()
                .find(|instance| instance.mechanical.is_some())
                .filter(|instance| instance_ids.contains(&instance.id))
                .map(|instance| instance.mechanical.clone().unwrap())
        }
    };
    // Resolve every selected member's effective values before anything mutates.
    let materialized = instance_ids
        .iter()
        .map(|instance_id| {
            effective_inputs(
                doc,
                &[],
                &MechanicalTarget::Instance {
                    instance_id: instance_id.clone(),
                },
            )
            .map(|inputs| (instance_id.clone(), inputs.configuration))
        })
        .collect::<Result<Vec<_>, String>>()?;
    let hardware = doc
        .hardware
        .as_mut()
        .ok_or("Physical instances are unavailable")?;
    if let Some(common) = &captured_source {
        hardware.shared_construction = Some(common.clone());
    }
    for (instance_id, configuration) in materialized {
        let instance = hardware
            .instances
            .iter_mut()
            .find(|instance| instance.id == instance_id)
            .ok_or("The selected physical instance is unavailable.")?;
        instance.mechanical = Some(configuration);
        instance.construction_ownership = MechanicalConstructionOwnership::Local;
    }
    let mut changed = vec![board_id.to_string()];
    changed.extend(instance_ids.iter().cloned());
    dedup_changed(&mut changed);
    Ok(changed)
}

/// The pair that owns `instance_id`, as its index and the other member's id.
fn pair_membership(doc: &ProjectDoc, instance_id: &str) -> Result<(usize, String), String> {
    let hardware = doc
        .hardware
        .as_ref()
        .ok_or("Physical instances are unavailable")?;
    let Some(index) = hardware
        .pairs
        .iter()
        .position(|pair| pair.members.iter().any(|member| member == instance_id))
    else {
        return Err(format!(
            "Physical instance {instance_id:?} is not part of a mechanical pair."
        ));
    };
    let survivor = hardware.pairs[index]
        .members
        .iter()
        .find(|member| *member != instance_id)
        .cloned()
        .ok_or("The mechanical pair has no other member.")?;
    Ok((index, survivor))
}

/// Repairs mechanical pair ownership after a wholesale topology replacement.
///
/// A replacement document can add or remove physical instances without going through
/// the typed pair operations. Any pair whose member is gone is dropped, and its
/// survivor keeps the model it showed before the replacement by materializing that
/// accepted effective configuration locally. An instance still marked pair-owned
/// without a pair becomes local, so a topology change can never leave dangling
/// ownership or lose visible geometry. The remaining pairs are validated.
pub(crate) fn reconcile_physical_topology(
    doc: &mut ProjectDoc,
    before: &ProjectDoc,
) -> Result<Vec<String>, String> {
    let Some(hardware) = doc.hardware.as_ref() else {
        return Ok(Vec::new());
    };
    let live: std::collections::BTreeSet<String> = hardware
        .instances
        .iter()
        .map(|instance| instance.id.clone())
        .collect();
    let dangling: Vec<usize> = hardware
        .pairs
        .iter()
        .enumerate()
        .filter(|(_, pair)| {
            pair.members
                .iter()
                .any(|member| !live.contains(member.as_str()))
        })
        .map(|(index, _)| index)
        .collect();
    let mut changed = Vec::new();
    for index in dangling.into_iter().rev() {
        let pair = doc.hardware.as_ref().unwrap().pairs[index].clone();
        doc.hardware.as_mut().unwrap().pairs.remove(index);
        changed.push(pair.id.clone());
        let survivor = pair
            .members
            .iter()
            .find(|member| live.contains(member.as_str()))
            .cloned();
        if let Some(survivor_id) = survivor {
            materialize_survivor(doc, before, &survivor_id)?;
            changed.push(survivor_id);
        }
    }
    let hardware = doc
        .hardware
        .as_mut()
        .ok_or("Physical instances are unavailable")?;
    let paired: std::collections::BTreeSet<String> = hardware
        .pairs
        .iter()
        .flat_map(|pair| pair.members.iter().cloned())
        .collect();
    let orphans: Vec<String> = hardware
        .instances
        .iter()
        .filter(|instance| {
            instance.construction_ownership == MechanicalConstructionOwnership::Pair
                && !paired.contains(&instance.id)
        })
        .map(|instance| instance.id.clone())
        .collect();
    for instance_id in orphans {
        materialize_survivor(doc, before, &instance_id)?;
        changed.push(instance_id);
    }
    // A replacement must not introduce an inconsistent pair either.
    features::validate(doc)?;
    Ok(changed)
}

/// Writes an instance's accepted effective model into its own configuration.
///
/// Used when pair ownership ends without the typed lifecycle, so the half keeps the
/// geometry and construction it currently shows and becomes locally owned.
fn materialize_survivor(
    doc: &mut ProjectDoc,
    before: &ProjectDoc,
    instance_id: &str,
) -> Result<(), String> {
    let materialized = effective_inputs(
        before,
        &[],
        &MechanicalTarget::Instance {
            instance_id: instance_id.to_string(),
        },
    )
    .ok()
    .map(|inputs| inputs.configuration);
    let hardware = doc
        .hardware
        .as_mut()
        .ok_or("Physical instances are unavailable")?;
    let Some(instance) = hardware
        .instances
        .iter_mut()
        .find(|instance| instance.id == instance_id)
    else {
        return Ok(());
    };
    if let Some(mut configuration) = materialized {
        // Keep the reservations high enough for every identity the pair made visible.
        let floor = reserved_identity_floor(
            before,
            &MechanicalTarget::Instance {
                instance_id: instance_id.to_string(),
            },
        );
        merge_identity_floor(&mut configuration, floor)?;
        instance.mechanical = Some(configuration);
    }
    instance.construction_ownership = MechanicalConstructionOwnership::Local;
    Ok(())
}

/// Dissolves the mechanical pair that owns `instance_id`.
///
/// Before the pair is removed each member's effective configuration (including
/// the pair's shared construction) is written to its own slot and every linked
/// feature's visible geometry is materialized, so the halves keep exactly the
/// geometry they showed while paired. Both members become `Local`-owned, which
/// is what keeps their shared subset authoritative after the pair is gone.
fn unlink_mirror_pair(
    doc: &mut ProjectDoc,
    _board_id: &str,
    edit: &MechanicalEdit,
) -> Result<Vec<String>, String> {
    let MechanicalChange::UnlinkMirrorPair { instance_id } = &edit.change else {
        return Err("The unlink operation was dispatched with another change.".into());
    };
    let pair_index = pair_index(doc, instance_id).ok_or_else(|| {
        format!("Physical instance {instance_id:?} is not part of a mechanical pair.")
    })?;
    let (pair_id, members) = dissolve_pair_ownership(doc, pair_index)?;
    let mut changed = vec![pair_id];
    changed.extend(members);
    project(doc, edit.mounting_hole.as_deref(), &mut changed)?;
    dedup_changed(&mut changed);
    Ok(changed)
}

/// The index of the mechanical pair that owns `instance_id`, if any.
fn pair_index(doc: &ProjectDoc, instance_id: &str) -> Option<usize> {
    doc.hardware
        .as_ref()?
        .pairs
        .iter()
        .position(|pair| pair.members.iter().any(|member| member == instance_id))
}

/// Materializes both halves' visible models locally and clears their pair.
///
/// Shared by explicit Unlink and by disabling a paired half, so both preserve the
/// geometry, shared construction and hardware-specific settings each half showed while
/// the pair is gone. Returns the pair id and its members.
fn dissolve_pair_ownership(
    doc: &mut ProjectDoc,
    pair_index: usize,
) -> Result<(String, Vec<String>), String> {
    let pair = doc
        .hardware
        .as_ref()
        .ok_or("Physical instances are unavailable")?
        .pairs
        .get(pair_index)
        .cloned()
        .ok_or("The mechanical pair is unavailable.")?;
    for member in &pair.members {
        let target = MechanicalTarget::Instance {
            instance_id: member.clone(),
        };
        // The accepted effective configuration already carries this half's visible
        // linked geometry, shared construction and local hardware, so writing it
        // locally makes that model authoritative without moving anything.
        let mut configuration = effective_inputs(doc, &[], &target)?.configuration;
        merge_identity_floor(&mut configuration, reserved_identity_floor(doc, &target))?;
        persist(doc, &target, configuration)?;
        let hardware = doc
            .hardware
            .as_mut()
            .ok_or("Physical instances are unavailable")?;
        let instance = hardware
            .instances
            .iter_mut()
            .find(|entry| entry.id == *member)
            .ok_or("Physical instance is unavailable")?;
        instance.construction_ownership = MechanicalConstructionOwnership::Local;
    }
    doc.hardware
        .as_mut()
        .ok_or("Physical instances are unavailable")?
        .pairs
        .remove(pair_index);
    Ok((pair.id.clone(), pair.members.to_vec()))
}

/// Applies one per-feature lifecycle action and projects the result.
fn apply_feature_lifecycle(
    doc: &mut ProjectDoc,
    edit: &MechanicalEdit,
) -> Result<Vec<String>, String> {
    let changed = match &edit.change {
        MechanicalChange::MakeFeatureLocal {
            feature_id,
            instance_id,
        } => make_feature_local(doc, feature_id, instance_id)?,
        MechanicalChange::OmitFeatureMember {
            feature_id,
            instance_id,
        } => omit_feature_member(doc, feature_id, instance_id)?,
        MechanicalChange::RejoinFeatureMember {
            feature_id,
            instance_id,
            source_instance_id,
        } => rejoin_feature_member(doc, feature_id, instance_id, source_instance_id)?,
        _ => return Err("The feature lifecycle was dispatched with another change.".into()),
    };
    if changed.is_empty() {
        return Ok(changed);
    }
    let mut changed = changed;
    project(doc, edit.mounting_hole.as_deref(), &mut changed)?;
    dedup_changed(&mut changed);
    Ok(changed)
}

/// Where a linked feature lives and the state of one half.
struct PairFeatureLocation {
    pair_index: usize,
    feature_index: usize,
    state: MechanicalFeatureState,
}

fn locate_pair_feature(
    doc: &ProjectDoc,
    instance_id: &str,
    feature_id: &str,
) -> Result<PairFeatureLocation, String> {
    let hardware = doc
        .hardware
        .as_ref()
        .ok_or("Physical instances are unavailable")?;
    let pair_index = hardware
        .pairs
        .iter()
        .position(|pair| features::member_state(pair, instance_id, feature_id).is_some())
        .ok_or_else(|| {
            format!("Physical instance {instance_id:?} has no linked feature {feature_id:?}.")
        })?;
    let pair = &hardware.pairs[pair_index];
    let feature_index = pair
        .features
        .iter()
        .position(|feature| feature_member_id(feature, instance_id) == Some(feature_id))
        .ok_or_else(|| {
            format!(
                "Mechanical pair {:?} has no feature {feature_id:?}.",
                pair.id
            )
        })?;
    let state = features::member_state(pair, instance_id, feature_id)
        .ok_or("Mechanical feature state is unavailable")?;
    Ok(PairFeatureLocation {
        pair_index,
        feature_index,
        state,
    })
}

/// Materializes one half's visible geometry and makes it local.
///
/// A half that is already local is unchanged. The other half and every other
/// feature keep their state; the copied geometry replaces the same member
/// feature id in the half's ordinary configuration, so nothing moves. When the
/// selected half is the author, authorship moves to the other linked half with
/// the stored value re-expressed in that half's frame, so the pair keeps exactly
/// one linked author and the other half still derives the same geometry.
fn make_feature_local(
    doc: &mut ProjectDoc,
    feature_id: &str,
    instance_id: &str,
) -> Result<Vec<String>, String> {
    let location = locate_pair_feature(doc, instance_id, feature_id)?;
    match location.state {
        MechanicalFeatureState::Local => return Ok(Vec::new()),
        MechanicalFeatureState::Omitted => {
            return Err(format!(
                "Feature {feature_id:?} is omitted on {instance_id:?}; rejoin it before making it local."
            ));
        }
        MechanicalFeatureState::Linked => {}
    }
    let (kind, visible) = {
        let pair = doc.hardware.as_ref().unwrap().pairs[location.pair_index].clone();
        let feature = &pair.features[location.feature_index];
        (
            feature.kind,
            feature_visible_value(&pair, feature, instance_id)?,
        )
    };
    if let Some(value) = visible {
        let configuration = instance_configuration_mut(doc, instance_id)?;
        install_feature(configuration, kind, feature_id, &value)?;
    }
    transfer_authorship(doc, &location, instance_id)?;
    features::make_local(
        &mut doc.hardware.as_mut().unwrap().pairs[location.pair_index],
        instance_id,
        feature_id,
    )?;
    let mut changed = vec![feature_id.to_string(), instance_id.to_string()];
    changed.extend(pair_owner_ids(doc, location.pair_index));
    Ok(changed)
}

/// Removes one half's geometry and records a local omission.
///
/// The other half and every other feature keep their state. An already omitted
/// half is unchanged. When the selected half is the author, authorship moves to
/// the other linked half first, so the surviving half still derives the same
/// geometry with no second authored value.
fn omit_feature_member(
    doc: &mut ProjectDoc,
    feature_id: &str,
    instance_id: &str,
) -> Result<Vec<String>, String> {
    let location = locate_pair_feature(doc, instance_id, feature_id)?;
    if location.state == MechanicalFeatureState::Omitted {
        return Ok(Vec::new());
    }
    let kind = doc.hardware.as_ref().unwrap().pairs[location.pair_index].features
        [location.feature_index]
        .kind;
    remove_feature(
        instance_configuration_mut(doc, instance_id)?,
        kind,
        feature_id,
    );
    transfer_authorship(doc, &location, instance_id)?;
    features::omit_member(
        &mut doc.hardware.as_mut().unwrap().pairs[location.pair_index],
        instance_id,
        feature_id,
    )?;
    let mut changed = vec![feature_id.to_string(), instance_id.to_string()];
    changed.extend(pair_owner_ids(doc, location.pair_index));
    Ok(changed)
}

/// Moves a feature's authorship off `instance_id` when that half was the author.
///
/// The stored value is rewritten in the other linked half's frame, so that half
/// derives exactly the geometry it showed before and the pair keeps its single
/// authored value. A no-op when `instance_id` is not the author.
fn transfer_authorship(
    doc: &mut ProjectDoc,
    location: &PairFeatureLocation,
    instance_id: &str,
) -> Result<(), String> {
    let pair = doc.hardware.as_ref().unwrap().pairs[location.pair_index].clone();
    let feature = &pair.features[location.feature_index];
    if feature.author != instance_id {
        return Ok(());
    }
    let other = pair
        .members
        .iter()
        .find(|member| member.as_str() != instance_id)
        .cloned()
        .ok_or("The mechanical pair has no other member.")?;
    let other_visible = feature_visible_value(&pair, feature, &other)?.ok_or_else(|| {
        format!("Physical instance {other:?} has no linked geometry to author the feature.")
    })?;
    let pair = &mut doc.hardware.as_mut().unwrap().pairs[location.pair_index];
    let feature = &mut pair.features[location.feature_index];
    feature.value = other_visible;
    feature.author = other;
    Ok(())
}

/// Clears one half's exception, adopting the chosen source's geometry.
///
/// The chosen source's visible (or local) geometry becomes the shared authored
/// value, the counterpart is derived from it through the pair map, and that
/// reflected geometry is installed on the rejoined half so it shows the reviewed
/// result immediately. The other half and every other feature keep their state.
fn rejoin_feature_member(
    doc: &mut ProjectDoc,
    feature_id: &str,
    instance_id: &str,
    source_instance_id: &str,
) -> Result<Vec<String>, String> {
    let location = locate_pair_feature(doc, instance_id, feature_id)?;
    if location.state == MechanicalFeatureState::Linked {
        return Ok(Vec::new());
    }
    let pair = doc.hardware.as_ref().unwrap().pairs[location.pair_index].clone();
    if !pair
        .members
        .iter()
        .any(|member| member == source_instance_id)
    {
        return Err(format!(
            "Physical instance {source_instance_id:?} is not a member of mechanical pair {:?}.",
            pair.id
        ));
    }
    // The selected half may deliberately choose its own current geometry as the source,
    // so the reviewed result can keep that half's placement and re-derive the other.
    let kind = pair.features[location.feature_index].kind;
    let source_state =
        features::member_state(&pair, source_instance_id, feature_id).ok_or_else(|| {
            format!(
                "Physical instance {source_instance_id:?} has no linked feature {feature_id:?}."
            )
        })?;
    let source_value = match source_state {
        MechanicalFeatureState::Omitted => {
            return Err(format!(
                "Physical instance {source_instance_id:?} has no geometry for feature {feature_id:?}."
            ));
        }
        MechanicalFeatureState::Linked => {
            feature_visible_value(&pair, &pair.features[location.feature_index], source_instance_id)?
                .ok_or_else(|| {
                    format!(
                        "Physical instance {source_instance_id:?} has no visible geometry for feature {feature_id:?}."
                    )
                })?
        }
        MechanicalFeatureState::Local => find_feature_value(
            &effective_inputs(
                doc,
                &[],
                &MechanicalTarget::Instance {
                    instance_id: source_instance_id.into(),
                },
            )?
            .configuration,
            kind,
            feature_id,
        )
        .ok_or_else(|| {
            format!(
                "Physical instance {source_instance_id:?} has no local geometry for feature {feature_id:?}."
            )
        })?,
    };
    {
        let pair = &mut doc.hardware.as_mut().unwrap().pairs[location.pair_index];
        let feature = &mut pair.features[location.feature_index];
        feature.value = source_value;
        feature.author = source_instance_id.to_string();
        features::rejoin_member(pair, instance_id, feature_id)?;
    }
    let derived = {
        let pair = &doc.hardware.as_ref().unwrap().pairs[location.pair_index];
        let feature = &pair.features[location.feature_index];
        if let Some(derived) = features::counterpart(pair, feature, source_instance_id)? {
            Some(derived)
        } else {
            // The chosen source may itself be a local exception. Its geometry is
            // now the shared authored value, so derive the counterpart from a
            // linked view without clearing the source's own exception.
            let mut probe = pair.clone();
            for member in &mut probe.features[location.feature_index].members {
                member.state = MechanicalFeatureState::Linked;
            }
            features::counterpart(
                &probe,
                &probe.features[location.feature_index],
                source_instance_id,
            )?
        }
    };
    // The pair is the single source for this feature again, so the half that is not the
    // source derives its reflected geometry on read; no copy is written back to a local
    // configuration. The derivation is still checked, so a source that cannot derive the
    // counterpart half is rejected. Choosing the selected half's own geometry makes the
    // other half the one that derives.
    let expected_counterpart = if source_instance_id == instance_id {
        pair.members
            .iter()
            .find(|member| *member != instance_id)
            .cloned()
    } else {
        Some(instance_id.to_string())
    };
    if let Some(derived) = derived
        && expected_counterpart.as_deref() != Some(derived.instance_id.as_str())
    {
        return Err("The rejoin source did not derive the counterpart half.".into());
    }
    // The pair owns the rejoined feature again, so the half's local copy must go:
    // a `Linked` member stores no geometry of its own.
    let member_feature_id = {
        let pair = &doc.hardware.as_ref().unwrap().pairs[location.pair_index];
        feature_member_id(&pair.features[location.feature_index], instance_id)
            .map(str::to_string)
            .ok_or_else(|| {
                format!(
                    "Physical instance {instance_id:?} has no record for feature {feature_id:?}."
                )
            })?
    };
    let configuration = instance_configuration_mut(doc, instance_id)?;
    remove_feature(configuration, kind, &member_feature_id);
    let mut changed = vec![
        feature_id.to_string(),
        instance_id.to_string(),
        source_instance_id.to_string(),
    ];
    changed.extend(pair_owner_ids(doc, location.pair_index));
    Ok(changed)
}

fn feature_member_id<'a>(feature: &'a MechanicalFeature, instance_id: &str) -> Option<&'a str> {
    feature
        .members
        .iter()
        .find(|member| member.instance_id == instance_id)
        .map(|member| member.feature_id.as_str())
}

/// One half's visible geometry for a linked feature, or `None` when that half
/// owns its geometry locally or is omitted.
fn feature_visible_value(
    pair: &MechanicalMirrorPair,
    feature: &MechanicalFeature,
    instance_id: &str,
) -> Result<Option<MechanicalFeatureValue>, String> {
    let member = feature
        .members
        .iter()
        .find(|member| member.instance_id == instance_id)
        .ok_or_else(|| {
            format!(
                "Mechanical feature {:?} has no record for {instance_id:?}.",
                feature.id
            )
        })?;
    match member.state {
        MechanicalFeatureState::Linked => {
            if feature.author == instance_id {
                Ok(Some(feature.value.clone()))
            } else {
                Ok(features::counterpart(pair, feature, &feature.author)?
                    .map(|derived| derived.value))
            }
        }
        MechanicalFeatureState::Local | MechanicalFeatureState::Omitted => Ok(None),
    }
}

/// The pair and both member instances a pair-record change touched.
fn pair_owner_ids(doc: &ProjectDoc, pair_index: usize) -> Vec<String> {
    let pair = &doc.hardware.as_ref().unwrap().pairs[pair_index];
    let mut ids = vec![pair.id.clone()];
    ids.extend(pair.members.iter().cloned());
    ids
}

/// The durable ids one configuration carries for a feature kind.
fn feature_ids(
    configuration: &MechanicalConfiguration,
    kind: MechanicalFeatureKind,
) -> std::collections::BTreeSet<String> {
    let ids = match kind {
        MechanicalFeatureKind::Mount => configuration
            .mounts
            .iter()
            .map(|mount| mount.id.clone())
            .collect::<Vec<_>>(),
        MechanicalFeatureKind::ClosureMount => configuration
            .closure_mounts
            .iter()
            .flatten()
            .map(|mount| mount.id.clone())
            .collect(),
        MechanicalFeatureKind::Opening => configuration
            .openings
            .iter()
            .flatten()
            .map(|opening| opening.id.clone())
            .collect(),
        MechanicalFeatureKind::GasketSupport => configuration
            .gasket_layout
            .iter()
            .flat_map(|layout| layout.supports.iter())
            .map(|anchor| anchor.id.clone())
            .collect(),
    };
    ids.into_iter().collect()
}

fn find_feature_value(
    configuration: &MechanicalConfiguration,
    kind: MechanicalFeatureKind,
    feature_id: &str,
) -> Option<MechanicalFeatureValue> {
    match kind {
        MechanicalFeatureKind::Mount => configuration
            .mounts
            .iter()
            .find(|mount| mount.id == feature_id)
            .cloned()
            .map(|mount| MechanicalFeatureValue::Mount { mount }),
        MechanicalFeatureKind::ClosureMount => configuration
            .closure_mounts
            .as_ref()?
            .iter()
            .find(|mount| mount.id == feature_id)
            .cloned()
            .map(|mount| MechanicalFeatureValue::Mount { mount }),
        MechanicalFeatureKind::Opening => configuration
            .openings
            .as_ref()?
            .iter()
            .find(|opening| opening.id == feature_id)
            .cloned()
            .map(|opening| MechanicalFeatureValue::Opening { opening }),
        MechanicalFeatureKind::GasketSupport => configuration
            .gasket_layout
            .as_ref()?
            .supports
            .iter()
            .find(|anchor| anchor.id == feature_id)
            .cloned()
            .map(|anchor| MechanicalFeatureValue::GasketSupport { anchor }),
    }
}

/// Installs visible geometry under the destination member's durable identity.
/// The pair's authored value can carry a different member's id; vertex and anchor
/// associations remain part of the value and are preserved.
pub(crate) fn install_feature(
    configuration: &mut MechanicalConfiguration,
    kind: MechanicalFeatureKind,
    feature_id: &str,
    value: &MechanicalFeatureValue,
) -> Result<(), String> {
    match (kind, value) {
        (MechanicalFeatureKind::Mount, MechanicalFeatureValue::Mount { mount }) => {
            replace_mount(&mut configuration.mounts, feature_id, mount.clone());
        }
        (MechanicalFeatureKind::ClosureMount, MechanicalFeatureValue::Mount { mount }) => {
            let mounts = configuration.closure_mounts.get_or_insert_with(Vec::new);
            replace_mount(mounts, feature_id, mount.clone());
        }
        (MechanicalFeatureKind::Opening, MechanicalFeatureValue::Opening { opening }) => {
            let mut opening = opening.clone();
            opening.id = feature_id.to_owned();
            let openings = configuration.openings.get_or_insert_with(Vec::new);
            if let Some(existing) = openings
                .iter_mut()
                .find(|existing| existing.id == feature_id)
            {
                *existing = opening;
            } else {
                openings.push(opening);
            }
        }
        (
            MechanicalFeatureKind::GasketSupport,
            MechanicalFeatureValue::GasketSupport { anchor },
        ) => {
            let mut anchor = anchor.clone();
            anchor.id = feature_id.to_owned();
            let layout = configuration
                .gasket_layout
                .get_or_insert_with(crate::mechanical::default_gasket_layout);
            if let Some(existing) = layout
                .supports
                .iter_mut()
                .find(|existing| existing.id == feature_id)
            {
                *existing = anchor;
            } else {
                layout.supports.push(anchor);
            }
        }
        _ => {
            return Err(format!(
                "Mechanical feature {feature_id:?} has a value that does not match its kind."
            ));
        }
    }
    Ok(())
}

fn replace_mount(mounts: &mut Vec<Mount>, id: &str, mut mount: Mount) {
    mount.id = id.to_owned();
    if let Some(existing) = mounts.iter_mut().find(|existing| existing.id == id) {
        *existing = mount;
    } else {
        mounts.push(mount);
    }
}

pub(crate) fn remove_feature(
    configuration: &mut MechanicalConfiguration,
    kind: MechanicalFeatureKind,
    feature_id: &str,
) {
    match kind {
        MechanicalFeatureKind::Mount => {
            configuration.mounts.retain(|mount| mount.id != feature_id);
        }
        MechanicalFeatureKind::ClosureMount => {
            if let Some(mounts) = configuration.closure_mounts.as_mut() {
                mounts.retain(|mount| mount.id != feature_id);
            }
        }
        MechanicalFeatureKind::Opening => {
            if let Some(openings) = configuration.openings.as_mut() {
                openings.retain(|opening| opening.id != feature_id);
            }
        }
        MechanicalFeatureKind::GasketSupport => {
            if let Some(layout) = configuration.gasket_layout.as_mut() {
                layout.supports.retain(|anchor| anchor.id != feature_id);
            }
        }
    }
}

/// The mutable stored configuration of one physical instance.
fn instance_configuration_mut<'a>(
    doc: &'a mut ProjectDoc,
    instance_id: &str,
) -> Result<&'a mut MechanicalConfiguration, String> {
    let hardware = doc
        .hardware
        .as_mut()
        .ok_or("Physical instances are unavailable")?;
    let instance = hardware
        .instances
        .iter_mut()
        .find(|instance| instance.id == instance_id)
        .ok_or_else(|| format!("Physical instance {instance_id:?} is unavailable."))?;
    instance.mechanical.as_mut().ok_or_else(|| {
        format!("Physical instance {instance_id:?} has no stored mechanical configuration.")
    })
}

/// Reference defaults for a board that has no stored configuration.
///
/// Ports the adapter's `case_settings::initial_settings` family selection, plate
/// and foam derivation, board thickness and standard process overrides onto the
/// Core-owned default helpers.
pub(crate) fn default_configuration(
    document: &ProjectDoc,
    board_id: &str,
) -> Result<MechanicalConfiguration, String> {
    let board = document
        .boards
        .iter()
        .find(|board| board.id == board_id)
        .ok_or("Board is unavailable")?;
    let family = initial_switch_family(document, board_id).unwrap_or(MechanicalSwitchFamily::Mx);
    let plate_thickness = crate::mechanical::default_plate_thickness(family);
    let plate_to_pcb = crate::mechanical::mounting_datum(family) - plate_thickness;
    let plate_foam_thickness = crate::mechanical::default_plate_foam_thickness(plate_to_pcb);
    let pcb_thickness = if board.thickness.is_finite() && board.thickness > 0.0 {
        board.thickness
    } else {
        1.6
    };
    let bottom_foam_thickness = 2.0;
    let bottom_thickness = 3.0;
    let method = PlateMethod::Printed;
    let part_processes = vec![
        crate::mechanical::default_layer_process("plate", method.clone(), plate_thickness),
        crate::mechanical::default_layer_process(
            "plate-foam",
            PlateMethod::CutSheet,
            plate_foam_thickness,
        ),
        crate::mechanical::default_layer_process(
            "bottom-foam",
            PlateMethod::CutSheet,
            bottom_foam_thickness,
        ),
        crate::mechanical::default_layer_process("bottom", method.clone(), bottom_thickness),
    ];
    Ok(MechanicalConfiguration {
        internal_gasket: None,
        gasket_layout: None,
        hardware: None,
        critical_fits: None,
        bottom_style: Some(MechanicalBottomStyle::Shell),
        middle_frame: Some(false),
        gasket_travel: None,
        openings: None,
        opening_allowance: None,
        stabilizers: None,
        part_processes: Some(part_processes),
        gasket: None,
        closure_mounts: None,
        board_id: board_id.to_string(),
        next_opening_identity: 0,
        next_critical_fit_identity: 0,
        next_hardware_identity: 0,
        next_mount_identity: 0,
        integrated_plate_frame: false,
        battery: None,
        mounts: Vec::new(),
        method,
        mount: MechanicalMount::Tray,
        plate_thickness,
        plate_foam_thickness,
        pcb_thickness,
        bottom_foam_thickness,
        battery_height: 0.0,
        bottom_thickness,
        plate_to_pcb,
        wall_thickness: 2.0,
        clearance: 0.3,
        profiles: Vec::new(),
    })
}

fn apply_change(
    configuration: &mut MechanicalConfiguration,
    change: &MechanicalChange,
    document: &ProjectDoc,
    board_id: &str,
    identity_floor: IdentityFloor,
    changed: &mut Vec<String>,
) -> Result<(), String> {
    let previous = configuration.clone();
    match change {
        MechanicalChange::Enable
        | MechanicalChange::Disable
        | MechanicalChange::InitializeClosures
        | MechanicalChange::LinkMirrorPair { .. }
        | MechanicalChange::UnlinkMirrorPair { .. }
        | MechanicalChange::RemovePhysicalInstance
        | MechanicalChange::MaterializeLegacyConstruction { .. }
        | MechanicalChange::MakeFeatureLocal { .. }
        | MechanicalChange::OmitFeatureMember { .. }
        | MechanicalChange::RejoinFeatureMember { .. } => {
            return Err(
                "Configure, Disable, closure initialization, the pair lifecycle and legacy conversion are handled by the operation."
                    .into(),
            );
        }
        MechanicalChange::AssignSwitchProfile {
            definition_id,
            family,
        } => {
            validate_profile_target(
                document,
                board_id,
                configuration,
                definition_id,
                ProfileTargetKind::Switch,
            )?;
            let existing_family = profile_family(configuration)
                .or_else(|| initial_switch_family(document, &configuration.board_id));
            let family_changed = family_assignment_changes(existing_family, *family);
            let plate_thickness = if family_changed {
                crate::mechanical::default_plate_thickness(*family)
            } else {
                configuration.plate_thickness
            };
            let plate_to_pcb = crate::mechanical::mounting_datum(*family) - plate_thickness;
            let source = match family {
                MechanicalSwitchFamily::Mx => MechanicalBuiltinProfile::MxSwitch,
                MechanicalSwitchFamily::ChocV1 => MechanicalBuiltinProfile::ChocV1Switch,
                MechanicalSwitchFamily::ChocV2 => MechanicalBuiltinProfile::ChocV2Switch,
            };
            let mut profile =
                crate::mechanical::builtin_profile(definition_id.clone(), source, plate_to_pcb)?;
            if profile.definition_id != *definition_id {
                return Err("Core returned a switch profile for another part type.".into());
            }
            profile.switch_family = Some(*family);
            profile.plate_to_pcb = plate_to_pcb;
            if family_changed {
                configuration.plate_thickness = plate_thickness;
                configuration.plate_to_pcb = plate_to_pcb;
                configuration.plate_foam_thickness =
                    crate::mechanical::default_plate_foam_thickness(plate_to_pcb);
            }
            configuration.profiles.push(profile);
            changed.push(definition_id.clone());
        }
        MechanicalChange::AssignStabilizerProfile {
            definition_id,
            source,
        } => {
            if !matches!(
                source,
                MechanicalBuiltinProfile::MxStab2u | MechanicalBuiltinProfile::MxStab625u
            ) {
                return Err("The selected stabilizer profile is not supported.".into());
            }
            validate_profile_target(
                document,
                board_id,
                configuration,
                definition_id,
                ProfileTargetKind::Stabilizer,
            )?;
            let mut profile = crate::mechanical::builtin_profile(
                definition_id.clone(),
                source.clone(),
                configuration.plate_to_pcb,
            )?;
            if profile.definition_id != *definition_id || profile.switch_family.is_some() {
                return Err("Core returned a different stabilizer profile.".into());
            }
            profile.plate_to_pcb = configuration.plate_to_pcb;
            configuration.profiles.push(profile);
            changed.push(definition_id.clone());
        }
        MechanicalChange::AssignImportedGeometryProfile { definition_id } => {
            validate_profile_target(
                document,
                board_id,
                configuration,
                definition_id,
                ProfileTargetKind::ImportedGeometry,
            )?;
            let definition = document
                .definitions
                .iter()
                .find(|definition| definition.id == *definition_id)
                .ok_or_else(|| "The imported part definition is no longer available.".to_owned())?;
            let family = document
                .parts
                .iter()
                .find(|part| {
                    part.definition_id == *definition_id
                        && document
                            .boards
                            .iter()
                            .any(|board| board.id == *board_id && board.part_ids.contains(&part.id))
                })
                .and_then(|part| inferred_switch_family(part, definition));
            configuration.profiles.push(MechanicalPartProfile {
                source_geometry: None,
                pcb_holes: None,
                clearance_volumes: None,
                openings: None,
                clearances: None,
                supported_thickness: None,
                switch_family: family,
                definition_id: definition.id.clone(),
                source: format!("KiCad {}", definition.name),
                cutouts: Vec::new(),
                plate_to_pcb: family.map_or(configuration.plate_to_pcb, |family| {
                    crate::mechanical::mounting_datum(family) - configuration.plate_thickness
                }),
            });
            changed.push(definition_id.clone());
        }
        MechanicalChange::SetBatteryEnabled(enabled) => {
            if document
                .hardware
                .as_ref()
                .is_some_and(|hardware| hardware.transport == HardwareTransport::Wireless)
            {
                return Err(
                    "Wireless battery envelopes are managed by the wireless configuration.".into(),
                );
            }
            if *enabled {
                if configuration.battery.is_some() {
                    return Err("A battery envelope is already configured.".into());
                }
                configuration.battery = Some(MechanicalBattery {
                    cable_width: Some(2.0),
                    size: Vec3 {
                        x: 30.0,
                        y: 20.0,
                        z: 6.0,
                    },
                    at: Vec2 { x: 0.0, y: 0.0 },
                    cable_exit: Vec2 { x: 0.0, y: 0.0 },
                });
            } else {
                if configuration.battery.is_none() {
                    return Err("There is no battery envelope to remove.".into());
                }
                configuration.battery = None;
            }
        }
        MechanicalChange::SetClosureInsertPreset(id) => {
            resize_closure_insert(closure_hardware_mut(configuration)?, id)?;
        }
        MechanicalChange::SetClosureDrive(drive) => {
            closure_hardware_mut(configuration)?.drive = drive.clone();
        }
        MechanicalChange::SetClosureInstallation(installation) => {
            closure_hardware_mut(configuration)?.installation = installation.clone();
        }
        MechanicalChange::SetClosureFixedLength(length) => {
            let hardware = closure_hardware_mut(configuration)?;
            if let Some(length) = length
                && (!length.is_finite()
                    || *length <= 0.0
                    || !hardware.screw_lengths.contains(length))
            {
                return Err(
                    "Choose an available positive screw length or automatic length.".into(),
                );
            }
            hardware.fixed_length = *length;
        }
        MechanicalChange::SetClosureThread(thread) => {
            closure_hardware_mut(configuration)?.thread = thread.clone();
        }
        MechanicalChange::SetClosureScrewLengths(lengths) => {
            if lengths.is_empty()
                || lengths
                    .iter()
                    .any(|length| !length.is_finite() || *length <= 0.0)
            {
                return Err("Enter positive lengths separated by commas.".into());
            }
            let hardware = closure_hardware_mut(configuration)?;
            hardware.screw_lengths = lengths.clone();
            if hardware
                .fixed_length
                .is_some_and(|fixed| !hardware.screw_lengths.contains(&fixed))
            {
                hardware.fixed_length = None;
            }
        }
        MechanicalChange::SetClosureHeadProfile(profile) => {
            closure_hardware_mut(configuration)?.head_profile = profile.clone();
        }
        MechanicalChange::SetClosureLengthDatum(datum) => {
            closure_hardware_mut(configuration)?.length_datum = datum.clone();
        }
        MechanicalChange::AddCriticalFit { part_id } => {
            if part_id.is_empty() {
                return Err("A generated part must be selected for a critical fit.".into());
            }
            let highest = configuration
                .critical_fits
                .as_ref()
                .into_iter()
                .flatten()
                .filter_map(|fit| identity_index_for(&fit.id, "case-critical-fit"))
                .max()
                .unwrap_or(0);
            let fit_id = reserve_identity(
                &mut configuration.next_critical_fit_identity,
                highest,
                identity_floor.critical_fit,
                "case-critical-fit",
                "Critical fit",
            )?;
            configuration
                .critical_fits
                .get_or_insert_with(Vec::new)
                .push(MechanicalCriticalFit {
                    id: fit_id.clone(),
                    part_id: part_id.clone(),
                    label: "Critical dimension".into(),
                    from: Vec2 { x: 0.0, y: 0.0 },
                    to: Vec2 { x: 10.0, y: 0.0 },
                    tolerance: "±0.2 mm".into(),
                });
            changed.push(fit_id);
        }
        MechanicalChange::RemoveCriticalFit { fit_id } => {
            let fits = configuration
                .critical_fits
                .as_mut()
                .ok_or_else(|| "The selected critical fit is no longer available.".to_owned())?;
            let index = fits
                .iter()
                .position(|fit| fit.id == *fit_id)
                .ok_or_else(|| "The selected critical fit is no longer available.".to_owned())?;
            fits.remove(index);
            changed.push(fit_id.clone());
        }
        MechanicalChange::SetCriticalFitText {
            fit_id,
            field,
            value,
        } => {
            let fit = critical_fit_mut(configuration, fit_id)?;
            match field {
                MechanicalCriticalFitTextField::Label => fit.label = value.clone(),
                MechanicalCriticalFitTextField::Tolerance => fit.tolerance = value.clone(),
            }
            changed.push(fit_id.clone());
        }
        MechanicalChange::SetCriticalFitPart { fit_id, part_id } => {
            if part_id.is_empty() {
                return Err("A generated part must be selected for a critical fit.".into());
            }
            critical_fit_mut(configuration, fit_id)?.part_id = part_id.clone();
            changed.push(fit_id.clone());
        }
        MechanicalChange::SetCriticalFitPoint {
            fit_id,
            field,
            value,
        } => {
            if !matches!(
                *field,
                MechanicalDimension::CriticalFitFromX
                    | MechanicalDimension::CriticalFitFromY
                    | MechanicalDimension::CriticalFitToX
                    | MechanicalDimension::CriticalFitToY
            ) {
                return Err("The selected critical-fit endpoint is unavailable.".into());
            }
            validate_dimension(*field, *value)?;
            let fit = critical_fit_mut(configuration, fit_id)?;
            match field {
                MechanicalDimension::CriticalFitFromX => fit.from.x = *value,
                MechanicalDimension::CriticalFitFromY => fit.from.y = *value,
                MechanicalDimension::CriticalFitToX => fit.to.x = *value,
                MechanicalDimension::CriticalFitToY => fit.to.y = *value,
                _ => return Err("The selected critical-fit endpoint is unavailable.".into()),
            }
            changed.push(fit_id.clone());
        }
        MechanicalChange::AddHardware {
            part_id,
            feature_id,
        } => {
            if part_id.is_empty() || feature_id.is_empty() {
                return Err("Link the hardware to a generated mount feature.".into());
            }
            let highest = configuration
                .hardware
                .as_ref()
                .into_iter()
                .flatten()
                .filter_map(|entry| identity_index_for(&entry.id, "case-mechanical-hardware"))
                .max()
                .unwrap_or(0);
            let hardware_id = reserve_identity(
                &mut configuration.next_hardware_identity,
                highest,
                identity_floor.hardware,
                "case-mechanical-hardware",
                "Hardware",
            )?;
            configuration.hardware.get_or_insert_with(Vec::new).push(
                MechanicalHardwareSpecification {
                    id: hardware_id.clone(),
                    part_id: part_id.clone(),
                    feature_id: feature_id.clone(),
                    designation: "Socket screw".into(),
                    thread: "M2 × 0.4".into(),
                    length: 6.0,
                    quantity: 4,
                    notes: Some(String::new()),
                    tolerance: None,
                },
            );
            changed.push(hardware_id);
        }
        MechanicalChange::RemoveHardware { hardware_id } => {
            let entries = configuration
                .hardware
                .as_mut()
                .ok_or_else(|| "The selected hardware entry is no longer available.".to_owned())?;
            let index = entries
                .iter()
                .position(|entry| entry.id == *hardware_id)
                .ok_or_else(|| "The selected hardware entry is no longer available.".to_owned())?;
            entries.remove(index);
            changed.push(hardware_id.clone());
        }
        MechanicalChange::SetHardwareMount {
            hardware_id,
            part_id,
            feature_id,
        } => {
            if part_id.is_empty() || feature_id.is_empty() {
                return Err("Link the hardware to a generated mount feature.".into());
            }
            let entry = hardware_mut(configuration, hardware_id)?;
            entry.part_id = part_id.clone();
            entry.feature_id = feature_id.clone();
            changed.push(hardware_id.clone());
        }
        MechanicalChange::SetHardwareText {
            hardware_id,
            field,
            value,
        } => {
            let entry = hardware_mut(configuration, hardware_id)?;
            match field {
                MechanicalHardwareTextField::Designation => entry.designation = value.clone(),
                MechanicalHardwareTextField::Thread => entry.thread = value.clone(),
                MechanicalHardwareTextField::Tolerance => entry.tolerance = Some(value.clone()),
                MechanicalHardwareTextField::Notes => entry.notes = Some(value.clone()),
            }
            changed.push(hardware_id.clone());
        }
        MechanicalChange::SetHardwareDimension {
            hardware_id,
            field,
            value,
        } => {
            let entry = hardware_mut(configuration, hardware_id)?;
            match field {
                MechanicalDimension::HardwareLength => {
                    if !value.is_finite() || *value < 0.1 {
                        return Err("Hardware length must be at least 0.1 mm.".into());
                    }
                    entry.length = *value;
                }
                MechanicalDimension::HardwareQuantity => {
                    if !value.is_finite()
                        || *value < 1.0
                        || value.fract() != 0.0
                        || *value > f64::from(u32::MAX)
                    {
                        return Err("Hardware quantity must be a positive whole number.".into());
                    }
                    entry.quantity = *value as u32;
                }
                _ => return Err("The selected hardware dimension is unavailable.".into()),
            }
            changed.push(hardware_id.clone());
        }
        MechanicalChange::AddOpening => {
            let id = reserve_opening_identity(configuration, identity_floor.opening)?;
            let mut opening = MechanicalOpening {
                id: id.clone(),
                vertices: Vec::new(),
                z: 0.0,
                height: 10.0,
                next_vertex_identity: 0,
            };
            for point in [
                Vec2 { x: -2.5, y: -2.5 },
                Vec2 { x: 2.5, y: -2.5 },
                Vec2 { x: 2.5, y: 2.5 },
                Vec2 { x: -2.5, y: 2.5 },
            ] {
                let vertex_id = reserve_vertex_identity(&mut opening, identity_floor.vertex)?;
                opening.vertices.push(MechanicalOpeningVertex {
                    id: vertex_id,
                    point,
                });
            }
            configuration
                .openings
                .get_or_insert_with(Vec::new)
                .push(opening);
            changed.push(id);
        }
        MechanicalChange::RemoveOpening { opening_id } => {
            let openings = configuration
                .openings
                .as_mut()
                .ok_or("The selected access opening is no longer available.")?;
            let position = openings
                .iter()
                .position(|opening| opening.id == *opening_id)
                .ok_or("The selected access opening is no longer available.")?;
            openings.remove(position);
            changed.push(opening_id.clone());
        }
        MechanicalChange::AddOpeningVertex { opening_id } => {
            let openings = configuration
                .openings
                .as_mut()
                .ok_or("The selected access opening is no longer available.")?;
            let opening = openings
                .iter_mut()
                .find(|opening| opening.id == *opening_id)
                .ok_or("The selected access opening is no longer available.")?;
            // Reserved from the opening's own monotonic counter, so a removed vertex
            // id can never be handed out again within that opening.
            let vertex_id = reserve_vertex_identity(opening, identity_floor.vertex)?;
            opening.vertices.push(MechanicalOpeningVertex {
                id: vertex_id.clone(),
                point: Vec2 { x: 0.0, y: 0.0 },
            });
            let opening_id = opening.id.clone();
            validate_openings(openings)?;
            changed.push(opening_id);
            changed.push(vertex_id);
        }
        MechanicalChange::RemoveOpeningVertex {
            opening_id,
            vertex_id,
        } => {
            let openings = configuration
                .openings
                .as_mut()
                .ok_or("The selected access opening is no longer available.")?;
            let opening = openings
                .iter_mut()
                .find(|opening| opening.id == *opening_id)
                .ok_or("The selected access opening is no longer available.")?;
            let position = opening
                .vertices
                .iter()
                .position(|vertex| vertex.id == *vertex_id)
                .ok_or("The selected opening vertex is no longer available.")?;
            if opening.vertices.len() <= 3 {
                return Err("An opening footprint needs at least three vertices.".into());
            }
            opening.vertices.remove(position);
            let opening_id = opening.id.clone();
            validate_openings(openings)?;
            changed.push(opening_id);
            changed.push(vertex_id.clone());
        }
        MechanicalChange::SetOpeningDimension {
            opening_id,
            vertex_id,
            field,
            value,
        } => {
            let is_volume_field = matches!(
                *field,
                MechanicalDimension::OpeningBottomZ | MechanicalDimension::OpeningHeight
            );
            let is_point_field = matches!(
                *field,
                MechanicalDimension::OpeningPointX | MechanicalDimension::OpeningPointY
            );
            if !(is_volume_field && vertex_id.is_none() || is_point_field && vertex_id.is_some()) {
                return Err("The selected opening field is unavailable.".into());
            }
            validate_dimension(*field, *value)?;
            let openings = configuration
                .openings
                .as_mut()
                .ok_or_else(|| "The selected access opening is no longer available.".to_owned())?;
            let opening = openings
                .iter_mut()
                .find(|opening| opening.id == *opening_id)
                .ok_or_else(|| "The selected access opening is no longer available.".to_owned())?;
            if let Some(vertex_id) = vertex_id {
                let vertex = opening
                    .vertices
                    .iter_mut()
                    .find(|vertex| vertex.id == *vertex_id)
                    .ok_or_else(|| {
                        "The selected opening vertex is no longer available.".to_owned()
                    })?;
                match field {
                    MechanicalDimension::OpeningPointX => vertex.point.x = *value,
                    MechanicalDimension::OpeningPointY => vertex.point.y = *value,
                    _ => return Err("The selected opening field is unavailable.".into()),
                }
                let opening_id = opening.id.clone();
                changed.push(opening_id);
                changed.push(vertex_id.clone());
            } else {
                match field {
                    MechanicalDimension::OpeningBottomZ => opening.z = *value,
                    MechanicalDimension::OpeningHeight => opening.height = *value,
                    _ => return Err("The selected opening field is unavailable.".into()),
                }
                changed.push(opening.id.clone());
            }
        }
        MechanicalChange::SetMethod(method) => configuration.method = method.clone(),
        MechanicalChange::SetMount(mount) => {
            configuration.mount = mount.clone();
            if *mount == MechanicalMount::Gasket {
                if let Some(closures) = &mut configuration.closure_mounts {
                    closures.retain(|entry| !entry.id.starts_with("auto-closure/"));
                }
                configuration
                    .gasket_layout
                    .get_or_insert_with(crate::mechanical::default_gasket_layout);
                if configuration.internal_gasket.is_none() {
                    configuration.gasket_travel = Some(0.1);
                }
                configuration
                    .internal_gasket
                    .get_or_insert_with(crate::mechanical::default_internal_gasket);
                configuration.integrated_plate_frame = false;
                configuration.bottom_style = Some(MechanicalBottomStyle::Shell);
                configuration.middle_frame = Some(false);
            }
        }
        MechanicalChange::AddMount { collection } => {
            if *collection == MechanicalMountCollection::Closure
                && configuration.internal_gasket.is_some()
            {
                return Err("Adding closure screws is unavailable with an internal gasket.".into());
            }
            // The old first-gap search skipped ids used by the whole document. Keeping
            // those collision exclusions as the high-water mark makes the next id
            // monotonic without ever colliding with a Part or Case-body mount.
            let highest = configuration
                .mounts
                .iter()
                .chain(configuration.closure_mounts.iter().flatten())
                .chain(
                    document
                        .case_bodies
                        .iter()
                        .flat_map(|body| body.mounts.iter().flatten()),
                )
                .map(|mount| &mount.id)
                .chain(document.parts.iter().map(|part| &part.id))
                .filter_map(|id| identity_index_for(id, "case-mechanical-mount"))
                .max()
                .unwrap_or(0);
            let id = reserve_identity(
                &mut configuration.next_mount_identity,
                highest,
                identity_floor.mount,
                "case-mechanical-mount",
                "Mount",
            )?;
            mount_collection_mut(configuration, *collection).push(Mount {
                id: id.clone(),
                at: Vec2 { x: 0.0, y: 0.0 },
                kind: MountKind::Hole,
                hole_diameter: 2.5,
                boss_diameter: Some(5.0),
                height: Some(5.0),
            });
            changed.push(id);
        }
        MechanicalChange::AddSuggestedClosureMounts(suggestions) => {
            if configuration.internal_gasket.is_some() {
                return Err("Adding closure screws is unavailable with an internal gasket.".into());
            }
            let height = configuration.plate_to_pcb
                + configuration.pcb_thickness
                + configuration
                    .bottom_foam_thickness
                    .max(configuration.battery_height);
            let mounts = configuration.closure_mounts.get_or_insert_with(Vec::new);
            for suggestion in suggestions {
                let id = format!("auto-closure/{}", suggestion.id);
                if mounts.iter().any(|mount| mount.id == id) {
                    continue;
                }
                let mut mount = suggestion.clone();
                mount.id = id.clone();
                mount.kind = MountKind::Boss;
                mount.hole_diameter = 2.2;
                mount.height = Some(height);
                mounts.push(mount);
                changed.push(id);
            }
            validate_mounts(mounts)?;
        }
        MechanicalChange::SetMountCollection { collection, mounts } => {
            validate_mounts(mounts)?;
            match collection {
                MechanicalMountCollection::Suspension => {
                    configuration.mounts = mounts.clone();
                }
                MechanicalMountCollection::Closure => {
                    if configuration.internal_gasket.is_some() {
                        let existing = configuration
                            .closure_mounts
                            .iter()
                            .flatten()
                            .map(|mount| mount.id.as_str())
                            .collect::<std::collections::HashSet<_>>();
                        if mounts
                            .iter()
                            .any(|mount| !existing.contains(mount.id.as_str()))
                        {
                            return Err(
                                "Adding closure screws is unavailable with an internal gasket."
                                    .into(),
                            );
                        }
                    }
                    configuration.closure_mounts = Some(mounts.clone());
                }
            }
        }
        MechanicalChange::AdoptClosurePositions(mounts) => {
            if configuration.internal_gasket.is_none() {
                return Err("Closure positions can only be adopted for an internal gasket.".into());
            }
            validate_mounts(mounts)?;
            configuration.closure_mounts = Some(mounts.clone());
        }
        MechanicalChange::SetMountDimension {
            collection,
            mount_id,
            field,
            value,
        } => {
            validate_dimension(*field, *value)?;
            let mounts = mount_collection_mut(configuration, *collection);
            let mount = mounts
                .iter_mut()
                .find(|mount| mount.id == *mount_id)
                .ok_or_else(|| "The selected mount is no longer available.".to_owned())?;
            match field {
                MechanicalDimension::MountPositionX => mount.at.x = *value,
                MechanicalDimension::MountPositionY => mount.at.y = *value,
                MechanicalDimension::MountHoleDiameter => mount.hole_diameter = *value,
                MechanicalDimension::MountBossDiameter => mount.boss_diameter = Some(*value),
                MechanicalDimension::MountBossHeight if mount.kind == MountKind::Boss => {
                    mount.height = Some(*value)
                }
                _ => return Err("The selected mount dimension is unavailable.".into()),
            }
            changed.push(mount_id.clone());
        }
        MechanicalChange::SetMountPosition {
            collection,
            mount_id,
            at,
        } => {
            if mount_id.is_empty() || !at.x.is_finite() || !at.y.is_finite() {
                return Err("Mount positions must be finite values.".into());
            }
            let mounts = mount_collection_mut(configuration, *collection);
            let mount = mounts
                .iter_mut()
                .find(|mount| mount.id == *mount_id)
                .ok_or_else(|| "The selected mount is no longer available.".to_owned())?;
            mount.at = *at;
            changed.push(mount_id.clone());
        }
        MechanicalChange::SetMountKind {
            collection,
            mount_id,
            kind,
        } => {
            let mounts = mount_collection_mut(configuration, *collection);
            let mount = mounts
                .iter_mut()
                .find(|mount| mount.id == *mount_id)
                .ok_or_else(|| "The selected mount is no longer available.".to_owned())?;
            mount.kind = kind.clone();
            changed.push(mount_id.clone());
        }
        MechanicalChange::RemoveMount {
            collection,
            mount_id,
        } => {
            if mount_id.is_empty() {
                return Err("The selected mount is no longer available.".into());
            }
            if *collection == MechanicalMountCollection::Suspension
                && configuration.mount == MechanicalMount::Gasket
            {
                return Err("Suspension mounts are unavailable for a gasket mount.".into());
            }
            let mounts = mount_collection_mut(configuration, *collection);
            let Some(index) = mounts.iter().position(|mount| mount.id == *mount_id) else {
                return Err("The selected mount is no longer available.".into());
            };
            mounts.remove(index);
            changed.push(mount_id.clone());
        }
        MechanicalChange::SetBottomStyle(style) => {
            configuration.bottom_style = Some(style.clone());
        }
        MechanicalChange::SetMiddleFrame(enabled) => {
            configuration.middle_frame = Some(*enabled);
        }
        MechanicalChange::SetIntegratedPlateFrame(enabled) => {
            configuration.integrated_plate_frame = *enabled;
        }
        MechanicalChange::SetDimension { field, value } => {
            if matches!(
                *field,
                MechanicalDimension::GasketSupportLength | MechanicalDimension::GasketSupportWidth
            ) {
                return Err(
                    "Gasket support dimensions require a current support selection.".into(),
                );
            }
            if matches!(
                *field,
                MechanicalDimension::OpeningBottomZ
                    | MechanicalDimension::OpeningHeight
                    | MechanicalDimension::OpeningPointX
                    | MechanicalDimension::OpeningPointY
            ) {
                return Err(
                    "Opening dimensions require a current access opening selection.".into(),
                );
            }
            validate_dimension(*field, *value)?;
            set_dimension(configuration, *field, *value)?;
        }
        MechanicalChange::SetGasketSupportDimension {
            support_id,
            anchors,
            field,
            value,
        } => {
            if !matches!(
                *field,
                MechanicalDimension::GasketSupportLength | MechanicalDimension::GasketSupportWidth
            ) {
                return Err("The selected gasket field is unavailable.".into());
            }
            validate_dimension(*field, *value)?;
            if support_id.is_empty()
                || anchors.is_empty()
                || anchors.len() > 2
                || !anchors.iter().any(|anchor| anchor.id == *support_id)
            {
                return Err("The selected gasket support is no longer available.".into());
            }
            let mut ids = std::collections::HashSet::new();
            if anchors
                .iter()
                .any(|anchor| anchor.id.is_empty() || !ids.insert(&anchor.id))
            {
                return Err("The linked gasket support selection is invalid.".into());
            }
            if configuration.mount != MechanicalMount::Gasket {
                return Err("The current configuration no longer contains gasket supports.".into());
            }
            let layout = configuration
                .gasket_layout
                .get_or_insert_with(crate::mechanical::default_gasket_layout);
            for source in anchors {
                let mut anchor = source.clone();
                if *field == MechanicalDimension::GasketSupportLength {
                    anchor.length = Some(*value);
                } else {
                    anchor.width = Some(*value);
                }
                anchor.placement = Some(GasketPlacement::User);
                if let Some(existing) = layout
                    .supports
                    .iter_mut()
                    .find(|entry| entry.id == anchor.id)
                {
                    *existing = anchor;
                } else {
                    layout.supports.push(anchor);
                }
            }
            changed.push(support_id.clone());
        }
        MechanicalChange::SetGasketSupportPlacement {
            support_id,
            anchors,
        } => {
            if support_id.is_empty()
                || anchors.is_empty()
                || anchors.len() > 2
                || !anchors.iter().any(|anchor| anchor.id == *support_id)
            {
                return Err("The selected gasket support is no longer available.".into());
            }
            validate_gasket_support_anchors(anchors)?;
            if configuration.mount != MechanicalMount::Gasket {
                return Err("The current configuration no longer contains gasket supports.".into());
            }
            let layout = configuration
                .gasket_layout
                .get_or_insert_with(crate::mechanical::default_gasket_layout);
            for source in anchors {
                let mut anchor = source.clone();
                anchor.placement = Some(GasketPlacement::User);
                if let Some(existing) = layout
                    .supports
                    .iter_mut()
                    .find(|entry| entry.id == anchor.id)
                {
                    *existing = anchor;
                } else {
                    layout.supports.push(anchor);
                }
            }
            changed.push(support_id.clone());
        }
        MechanicalChange::SetGasketSupportUnlinked {
            support_id,
            pair_id,
            anchors,
        } => {
            if support_id.is_empty()
                || anchors.is_empty()
                || anchors.len() > 2
                || !anchors.iter().any(|anchor| anchor.id == *support_id)
            {
                return Err("The selected gasket support is no longer available.".into());
            }
            let mut ids = std::collections::HashSet::new();
            if anchors
                .iter()
                .any(|anchor| anchor.id.is_empty() || !ids.insert(&anchor.id))
            {
                return Err("The linked gasket support selection is invalid.".into());
            }
            let pair_is_present = pair_id
                .as_ref()
                .is_some_and(|pair_id| ids.contains(pair_id));
            if pair_id.as_deref() == Some(support_id.as_str())
                || pair_id.is_some() != pair_is_present
                || anchors.len() != if pair_id.is_some() { 2 } else { 1 }
            {
                return Err("The selected gasket support pair is no longer available.".into());
            }
            if anchors
                .iter()
                .find(|anchor| anchor.id == *support_id)
                .is_some_and(|anchor| anchor.unlinked)
            {
                return Err("The selected gasket support is already unlinked.".into());
            }
            if configuration.mount != MechanicalMount::Gasket {
                return Err("The current configuration no longer contains gasket supports.".into());
            }
            let layout = configuration
                .gasket_layout
                .get_or_insert_with(crate::mechanical::default_gasket_layout);
            for source in anchors {
                let mut anchor = source.clone();
                anchor.unlinked = true;
                anchor.placement = Some(GasketPlacement::User);
                if let Some(existing) = layout
                    .supports
                    .iter_mut()
                    .find(|entry| entry.id == anchor.id)
                {
                    *existing = anchor;
                } else {
                    layout.supports.push(anchor);
                }
            }
            changed.push(support_id.clone());
        }
        MechanicalChange::ResetGasketPlacement => {
            if configuration.mount != MechanicalMount::Gasket
                || configuration.internal_gasket.is_none()
            {
                return Err("The current configuration no longer contains gasket supports.".into());
            }
            configuration
                .gasket_layout
                .get_or_insert_with(crate::mechanical::default_gasket_layout)
                .supports
                .clear();
        }
        MechanicalChange::UpdateProfile(next) => {
            if configuration.board_id != board_id {
                return Err("The mechanical configuration belongs to another board.".into());
            }
            let Some(profile) = configuration
                .profiles
                .iter_mut()
                .find(|profile| profile.definition_id == next.definition_id)
            else {
                return Err("The selected mechanical profile is no longer assigned.".into());
            };
            let extraction_changed = profile.source_geometry != next.source_geometry;
            if extraction_changed {
                let definition = document
                    .definitions
                    .iter()
                    .find(|definition| definition.id == next.definition_id)
                    .ok_or_else(|| {
                        "The selected mechanical profile is no longer available.".to_owned()
                    })?;
                if definition.kicad_source.as_ref().is_none_or(|source| {
                    next.source_geometry
                        .as_ref()
                        .is_none_or(|geometry| geometry.text != source.source)
                }) {
                    return Err(
                        "Extracted geometry must come from the assigned KiCad source.".into(),
                    );
                }
            } else if profile.pcb_holes != next.pcb_holes {
                return Err("PCB mounting holes can only be changed by extracting the assigned KiCad geometry.".into());
            }
            if profile.switch_family != next.switch_family
                || profile.supported_thickness != next.supported_thickness
                || profile.plate_to_pcb != next.plate_to_pcb
                || profile.source != next.source
            {
                return Err(
                    "Only extracted geometry for the assigned KiCad profile can be updated here."
                        .into(),
                );
            }
            if !valid_profile_polygons(&next.cutouts)
                || next
                    .clearances
                    .as_deref()
                    .is_some_and(|polygons| !valid_profile_polygons(polygons))
                || next.pcb_holes.iter().flatten().any(|hole| {
                    !hole.at.x.is_finite()
                        || !hole.at.y.is_finite()
                        || !hole.diameter.is_finite()
                        || hole.diameter <= 0.0
                })
            {
                return Err(
                    "Profile polygons and mounting holes must contain valid finite coordinates."
                        .into(),
                );
            }
            validate_profile_openings(
                next.openings.as_deref().unwrap_or_default(),
                "Access opening",
            )?;
            validate_profile_openings(
                next.clearance_volumes.as_deref().unwrap_or_default(),
                "Clearance volume",
            )?;
            let definition_id = next.definition_id.clone();
            *profile = next.clone();
            changed.push(definition_id);
        }
        MechanicalChange::RemoveProfile { definition_id } => {
            if configuration.board_id != board_id {
                return Err("The mechanical configuration belongs to another board.".into());
            }
            let previous_len = configuration.profiles.len();
            configuration
                .profiles
                .retain(|profile| profile.definition_id != *definition_id);
            if configuration.profiles.len() == previous_len {
                return Err("The selected mechanical profile is no longer assigned.".into());
            }
            changed.push(definition_id.clone());
        }
        MechanicalChange::SetSwitchFamily {
            definition_id,
            family,
        } => {
            if !configuration
                .profiles
                .iter()
                .any(|profile| profile.definition_id == *definition_id)
            {
                return Err("The selected switch profile is no longer available.".into());
            }
            let plate_thickness = crate::mechanical::default_plate_thickness(*family);
            let plate_to_pcb = crate::mechanical::mounting_datum(*family) - plate_thickness;
            for profile in &mut configuration.profiles {
                if profile.definition_id == *definition_id {
                    profile.switch_family = Some(*family);
                    profile.plate_to_pcb = plate_to_pcb;
                } else if let Some(other_family) = profile.switch_family {
                    profile.plate_to_pcb =
                        crate::mechanical::mounting_datum(other_family) - plate_thickness;
                }
            }
            configuration.plate_thickness = plate_thickness;
            configuration.plate_to_pcb = plate_to_pcb;
            configuration.plate_foam_thickness =
                crate::mechanical::default_plate_foam_thickness(plate_to_pcb);
            changed.push(definition_id.clone());
        }
        MechanicalChange::AddProcessOverride { part_id } => {
            if !is_process_target(document, part_id) {
                return Err("The selected Case part is no longer available.".into());
            }
            if configuration
                .part_processes
                .iter()
                .flatten()
                .any(|process| process.part_id == *part_id)
            {
                return Err("A process override already exists for this part.".into());
            }
            let thickness = process_default_thickness(configuration, part_id);
            let method = if part_id.ends_with("foam") {
                PlateMethod::CutSheet
            } else {
                configuration.method.clone()
            };
            configuration
                .part_processes
                .get_or_insert_with(Vec::new)
                .push(crate::mechanical::default_layer_process(
                    part_id, method, thickness,
                ));
            changed.push(part_id.clone());
        }
        MechanicalChange::RemoveProcessOverride { part_id } => {
            let processes = configuration.part_processes.as_mut().ok_or_else(|| {
                "The selected process override is no longer available.".to_owned()
            })?;
            let previous_len = processes.len();
            processes.retain(|process| process.part_id != *part_id);
            if processes.len() == previous_len {
                return Err("The selected process override is no longer available.".into());
            }
            changed.push(part_id.clone());
        }
        MechanicalChange::SetProcessMethod { part_id, method } => {
            if !is_process_target(document, part_id) || part_id.ends_with("foam") {
                return Err("The selected process method is not available.".into());
            }
            if part_id == "plate" {
                configuration.method = method.clone();
            } else {
                let process = process_mut(configuration, document, part_id)?;
                let previous_method = process.method.clone();
                process.method = method.clone();
                if previous_method != *method
                    && !material_options(part_id, method).contains(&process.material.as_str())
                {
                    process.material = crate::mechanical::default_material(part_id, method).into();
                }
            }
            changed.push(part_id.clone());
        }
        MechanicalChange::SetProcessMaterial { part_id, material } => {
            let process = process_mut(configuration, document, part_id)?;
            if !material_options(part_id, &process.method).contains(&material.as_str()) {
                return Err("The selected material is not available for this process.".into());
            }
            process.material = material.clone();
            changed.push(part_id.clone());
        }
        MechanicalChange::SetProcessThickness { part_id, thickness } => {
            let min = if part_id.ends_with("foam") { 0.0 } else { 0.1 };
            if !thickness.is_finite() || *thickness < min {
                return Err(if min == 0.0 {
                    "Foam thickness must be finite and nonnegative.".into()
                } else {
                    "Finished thickness must be at least 0.1 mm.".into()
                });
            }
            if !is_process_target(document, part_id) {
                return Err("The selected Case part is no longer available.".into());
            }
            match part_id.as_str() {
                "plate" => configuration.plate_thickness = *thickness,
                "plate-foam" => configuration.plate_foam_thickness = *thickness,
                "bottom-foam" => configuration.bottom_foam_thickness = *thickness,
                "bottom" => configuration.bottom_thickness = *thickness,
                _ => process_mut(configuration, document, part_id)?.thickness = *thickness,
            }
            changed.push(part_id.clone());
        }
        MechanicalChange::SetStabilizerKind(stabilizer) => {
            if !stabilizer_target_exists(document, board_id, &stabilizer.part_id) {
                return Err("The selected wide key no longer needs a stabilizer.".into());
            }
            let values = configuration.stabilizers.get_or_insert_with(Vec::new);
            if let Some(existing) = values
                .iter_mut()
                .find(|existing| existing.part_id == stabilizer.part_id)
            {
                *existing = stabilizer.clone();
            } else {
                values.push(stabilizer.clone());
            }
            changed.push(stabilizer.part_id.clone());
        }
    }

    let family =
        profile_family(configuration).or_else(|| initial_switch_family(document, board_id));
    let new_plate_thickness = match change {
        MechanicalChange::SetDimension {
            field: MechanicalDimension::PlateThickness,
            value,
        } => Some(*value),
        MechanicalChange::SetProcessThickness { part_id, thickness } if part_id == "plate" => {
            Some(*thickness)
        }
        MechanicalChange::SetSwitchFamily { .. } => Some(configuration.plate_thickness),
        _ => None,
    };
    if let Some(new_plate_thickness) = new_plate_thickness
        && let Some(family) = family
    {
        let previous_gap = crate::mechanical::mounting_datum(family) - previous.plate_thickness;
        configuration.plate_to_pcb =
            crate::mechanical::mounting_datum(family) - new_plate_thickness;
        for profile in &mut configuration.profiles {
            if profile.switch_family == Some(family) {
                profile.plate_to_pcb = configuration.plate_to_pcb;
            }
        }
        if (previous.plate_foam_thickness
            - crate::mechanical::default_plate_foam_thickness(previous_gap))
        .abs()
            < 0.001
        {
            configuration.plate_foam_thickness =
                crate::mechanical::default_plate_foam_thickness(configuration.plate_to_pcb);
        }
    }
    // A change that leaves the effective configuration untouched is a no-op, so the
    // process normalization side effect must not materialize an empty override list.
    if *configuration == previous {
        return Ok(());
    }
    if !skip_process_normalization(change) {
        normalize_processes(configuration, change);
    }
    Ok(())
}

fn skip_process_normalization(change: &MechanicalChange) -> bool {
    matches!(
        change,
        MechanicalChange::SetBatteryEnabled(_)
            | MechanicalChange::SetClosureInsertPreset(_)
            | MechanicalChange::SetClosureDrive(_)
            | MechanicalChange::SetClosureInstallation(_)
            | MechanicalChange::SetClosureFixedLength(_)
            | MechanicalChange::SetClosureThread(_)
            | MechanicalChange::SetClosureScrewLengths(_)
            | MechanicalChange::SetClosureHeadProfile(_)
            | MechanicalChange::SetClosureLengthDatum(_)
            | MechanicalChange::AddCriticalFit { .. }
            | MechanicalChange::RemoveCriticalFit { .. }
            | MechanicalChange::SetCriticalFitText { .. }
            | MechanicalChange::SetCriticalFitPart { .. }
            | MechanicalChange::SetCriticalFitPoint { .. }
            | MechanicalChange::AddHardware { .. }
            | MechanicalChange::RemoveHardware { .. }
            | MechanicalChange::SetHardwareMount { .. }
            | MechanicalChange::SetHardwareText { .. }
            | MechanicalChange::SetHardwareDimension { .. }
            | MechanicalChange::AddOpening
            | MechanicalChange::RemoveOpening { .. }
            | MechanicalChange::AddOpeningVertex { .. }
            | MechanicalChange::RemoveOpeningVertex { .. }
            | MechanicalChange::SetOpeningDimension { .. }
            | MechanicalChange::SetStabilizerKind(_)
    ) || matches!(
        change,
        MechanicalChange::SetMountPosition { .. }
            | MechanicalChange::SetGasketSupportPlacement { .. }
            | MechanicalChange::ResetGasketPlacement
    ) || matches!(
        change,
        MechanicalChange::SetDimension {
            field: MechanicalDimension::BatteryWidth
                | MechanicalDimension::OpeningBottomZ
                | MechanicalDimension::OpeningHeight
                | MechanicalDimension::OpeningPointX
                | MechanicalDimension::OpeningPointY
                | MechanicalDimension::BatteryDepth
                | MechanicalDimension::BatteryHeight
                | MechanicalDimension::BatteryCableWidth
                | MechanicalDimension::BatteryPositionX
                | MechanicalDimension::BatteryPositionY
                | MechanicalDimension::BatteryCableExitX
                | MechanicalDimension::BatteryCableExitY
                | MechanicalDimension::ClosureThreadDiameter
                | MechanicalDimension::ClosurePitch
                | MechanicalDimension::ClosureHeadDiameter
                | MechanicalDimension::ClosureHeadHeight
                | MechanicalDimension::ClosureHoleDiameter
                | MechanicalDimension::ClosureInsertDiameter
                | MechanicalDimension::ClosureInsertLength
                | MechanicalDimension::ClosureSeatDiameter
                | MechanicalDimension::ClosureSeatDepth
                | MechanicalDimension::ClosureEngagement
                | MechanicalDimension::ClosureThreadStart
                | MechanicalDimension::ClosureTipAllowance
                | MechanicalDimension::ClosureBottomingClearance
                | MechanicalDimension::ClosureRoof
                | MechanicalDimension::ClosureSurround
                | MechanicalDimension::ClosureSeatLeadDepth
                | MechanicalDimension::ClosureSeatLeadDiameter
                | MechanicalDimension::ClosureBearingThickness,
            ..
        }
    )
}

#[derive(Clone, Copy)]
enum ProfileTargetKind {
    Switch,
    Stabilizer,
    ImportedGeometry,
}

fn validate_profile_target(
    document: &ProjectDoc,
    board_id: &str,
    configuration: &MechanicalConfiguration,
    definition_id: &str,
    target_kind: ProfileTargetKind,
) -> Result<(), String> {
    if configuration.board_id != board_id {
        return Err("The mechanical configuration belongs to another board.".into());
    }
    if configuration
        .profiles
        .iter()
        .any(|profile| profile.definition_id == definition_id)
    {
        return Err("A mechanical profile is already assigned to this part type.".into());
    }
    let board = document
        .boards
        .iter()
        .find(|board| board.id == board_id)
        .ok_or_else(|| "The selected board is no longer available.".to_owned())?;
    let part = document
        .parts
        .iter()
        .find(|part| board.part_ids.contains(&part.id) && part.definition_id == definition_id);
    let definition = document
        .definitions
        .iter()
        .find(|definition| definition.id == definition_id);
    let eligible = match (part, definition) {
        (Some(_), Some(definition)) if matches!(target_kind, ProfileTargetKind::Switch) => {
            definition.kind == PartKind::Switch
                || definition
                    .generator
                    .as_ref()
                    .is_some_and(|generator| generator.source.ends_with("/switch_choc_v1_v2"))
        }
        (Some(_), Some(_)) if matches!(target_kind, ProfileTargetKind::Stabilizer) => true,
        (Some(_), Some(definition)) => definition.kicad_source.is_some(),
        _ => false,
    };
    if !eligible {
        return Err("The selected part type is not available for this profile assignment.".into());
    }
    Ok(())
}

fn mount_collection_mut(
    configuration: &mut MechanicalConfiguration,
    collection: MechanicalMountCollection,
) -> &mut Vec<Mount> {
    match collection {
        MechanicalMountCollection::Suspension => &mut configuration.mounts,
        MechanicalMountCollection::Closure => {
            configuration.closure_mounts.get_or_insert_with(Vec::new)
        }
    }
}

fn validate_gasket_support_anchors(anchors: &[MechanicalGasketAnchor]) -> Result<(), String> {
    let mut ids = std::collections::HashSet::new();
    if anchors.iter().any(|anchor| {
        anchor.id.is_empty()
            || !ids.insert(&anchor.id)
            || !anchor.anchor.is_finite()
            || !(0.0..=1.0).contains(&anchor.anchor)
    }) {
        return Err("The gasket support placement is invalid.".into());
    }
    Ok(())
}

fn validate_mounts(mounts: &[Mount]) -> Result<(), String> {
    let mut ids = std::collections::HashSet::new();
    for mount in mounts {
        if mount.id.is_empty() || !ids.insert(mount.id.as_str()) {
            return Err("Mount IDs must be non-empty and unique.".into());
        }
        if !mount.at.x.is_finite() || !mount.at.y.is_finite() {
            return Err("Mount positions must be finite values.".into());
        }
        validate_mount_positive(mount.hole_diameter, "Hole diameter")?;
        validate_mount_positive(mount.boss_diameter.unwrap_or(5.0), "Boss diameter")?;
        if mount.kind == MountKind::Boss {
            validate_mount_positive(mount.height.unwrap_or(5.0), "Boss height")?;
        }
    }
    Ok(())
}

fn valid_profile_polygons(polygons: &[Vec<Vec2>]) -> bool {
    polygons.iter().all(|polygon| {
        polygon.len() >= 3
            && polygon
                .iter()
                .all(|point| point.x.is_finite() && point.y.is_finite())
    })
}

fn validate_profile_openings(openings: &[CaseOpening], label: &str) -> Result<(), String> {
    for (index, opening) in openings.iter().enumerate() {
        validate_opening_geometry(&opening.points, opening.z, opening.height, label, index)?;
    }
    Ok(())
}

fn validate_opening_geometry(
    points: &[Vec2],
    z: f64,
    height: f64,
    label: &str,
    index: usize,
) -> Result<(), String> {
    if points.len() < 3
        || points.iter().any(|point| {
            !point.x.is_finite()
                || !point.y.is_finite()
                || point.x < -boardstudio_contracts::MAX_COORDINATE_MM
                || point.y < -boardstudio_contracts::MAX_COORDINATE_MM
        })
    {
        return Err(format!(
            "{label} {} needs at least three vertices with finite coordinates no smaller than −1,000,000 mm.",
            index + 1
        ));
    }
    if !z.is_finite() || z < -boardstudio_contracts::MAX_COORDINATE_MM {
        return Err(format!(
            "{label} {} bottom Z must be a finite coordinate no smaller than −1,000,000 mm.",
            index + 1
        ));
    }
    if !height.is_finite() || height < 0.1 {
        return Err(format!(
            "{label} {} height must be at least 0.1 mm.",
            index + 1
        ));
    }
    Ok(())
}

fn validate_openings(openings: &[MechanicalOpening]) -> Result<(), String> {
    for (index, opening) in openings.iter().enumerate() {
        validate_opening_geometry(
            &opening.points(),
            opening.z,
            opening.height,
            "Access opening",
            index,
        )?;
    }
    Ok(())
}

/// The reserved high-water marks an edit's allocation must not hand out again.
#[derive(Clone, Copy, Default)]
pub(crate) struct IdentityFloor {
    opening: u64,
    vertex: u64,
    critical_fit: u64,
    hardware: u64,
    mount: u64,
}

impl IdentityFloor {
    fn merged(self, other: Self) -> Self {
        Self {
            opening: self.opening.max(other.opening),
            vertex: self.vertex.max(other.vertex),
            critical_fit: self.critical_fit.max(other.critical_fit),
            hardware: self.hardware.max(other.hardware),
            mount: self.mount.max(other.mount),
        }
    }
}

fn exhausted_identity(kind: &str) -> String {
    format!("{kind} identity is exhausted.")
}

/// Allocates the next monotonic identity from `counter`.
///
/// `counter` is the next free number; `u64::MAX` means the namespace can never
/// advance again. A live or reserved `MAX` is refused rather than saturated into a
/// reusable identity.
fn reserve_identity(
    counter: &mut u64,
    highest: u64,
    floor: u64,
    prefix: &str,
    kind: &str,
) -> Result<String, String> {
    if *counter == u64::MAX || highest == u64::MAX || floor == u64::MAX {
        return Err(exhausted_identity(kind));
    }
    let next = (*counter).max(1).max(highest + 1).max(floor + 1);
    *counter = next.saturating_add(1);
    Ok(format!("{prefix}-{next}"))
}

/// Reserves the next authored opening or vertex identity for a configuration.
///
/// The stored reservation is monotonic, so a removed opening or vertex identity is
/// never handed out again and an intent queued against it can never reach a later
/// addition. The first reservation for a configuration that predates the field
/// normalizes to the highest identity it already carries, which preserves existing
/// geometry and ids.
fn reserve_opening_identity(
    configuration: &mut MechanicalConfiguration,
    floor: u64,
) -> Result<String, String> {
    let highest = configuration
        .openings
        .as_ref()
        .into_iter()
        .flatten()
        .map(|opening| identity_index(&opening.id))
        .max()
        .unwrap_or(0);
    reserve_identity(
        &mut configuration.next_opening_identity,
        highest,
        floor,
        "opening",
        "Opening",
    )
}

fn reserve_vertex_identity(opening: &mut MechanicalOpening, floor: u64) -> Result<String, String> {
    let highest = opening
        .vertices
        .iter()
        .map(|vertex| identity_index(&vertex.id))
        .max()
        .unwrap_or(0);
    reserve_identity(
        &mut opening.next_vertex_identity,
        highest,
        floor,
        "vertex",
        "Vertex",
    )
}

/// The identities a target's member reserves for its pair-owned features.
///
/// A linked feature is owned by the pair and is invisible to the member's raw
/// configuration, and stays reserved while the member omits it. Reserving from those
/// identities keeps a new local addition from taking one the pair owns for this member,
/// which would otherwise let a rejoin produce two openings under one identity or let a
/// queued intent reach the wrong opening.
fn reserved_identity_floor(document: &ProjectDoc, target: &MechanicalTarget) -> IdentityFloor {
    let MechanicalTarget::Instance { instance_id } = target else {
        return IdentityFloor::default();
    };
    let Some(hardware) = document.hardware.as_ref() else {
        return IdentityFloor::default();
    };
    let member_features = || {
        hardware
            .pairs
            .iter()
            .flat_map(|pair| pair.features.iter())
            .filter_map(move |feature| {
                feature
                    .members
                    .iter()
                    .find(|member| member.instance_id == *instance_id)
                    .map(|member| (feature, member))
            })
    };
    let floor = |prefix: &str| {
        member_features()
            .filter_map(|(_, member)| identity_index_for(&member.feature_id, prefix))
            .max()
            .unwrap_or(0)
    };
    let mount = member_features()
        .filter(|(feature, _)| {
            matches!(
                feature.kind,
                MechanicalFeatureKind::Mount | MechanicalFeatureKind::ClosureMount
            )
        })
        .filter_map(|(feature, member)| {
            let mount_id = match &feature.value {
                MechanicalFeatureValue::Mount { mount } => Some(mount.id.as_str()),
                _ => None,
            };
            identity_index_for(&member.feature_id, "case-mechanical-mount")
                .into_iter()
                .chain(
                    mount_id
                        .into_iter()
                        .filter_map(|id| identity_index_for(id, "case-mechanical-mount")),
                )
                .max()
        })
        .max()
        .unwrap_or(0);
    // A half that outlived its own configuration (a disabled half) keeps the highest
    // identity it ever reserved, so re-enabling cannot reuse a removed one.
    let stored = hardware
        .instances
        .iter()
        .find(|instance| instance.id == *instance_id)
        .and_then(|instance| instance.mechanical_identity_reservation)
        .unwrap_or_default();
    IdentityFloor {
        opening: floor("opening").max(reservation_floor(stored.next_opening_identity)),
        vertex: floor("vertex").max(reservation_floor(stored.next_vertex_identity)),
        critical_fit: reservation_floor(stored.next_critical_fit_identity),
        hardware: reservation_floor(stored.next_hardware_identity),
        mount: mount.max(reservation_floor(stored.next_mount_identity)),
    }
}

/// A stored `next == MAX` is the exhausted sentinel: no further identity is handed out.
fn reservation_floor(next: u64) -> u64 {
    if next == u64::MAX {
        u64::MAX
    } else {
        next.saturating_sub(1)
    }
}

/// Who owns a mechanical configuration's identity namespace.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum IdentityConfigOwner {
    Board(String),
    Instance(String),
}

/// The per-opening vertex namespace. Vertex identities only need to stay unique
/// within their owning opening, so two openings may both use `vertex-1`.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct IdentityVertexOwner {
    config: IdentityConfigOwner,
    opening_id: String,
}

#[derive(Clone, Copy, Debug, Default)]
struct IdentityCounter {
    next: u64,
    exhausted: bool,
}

impl IdentityCounter {
    fn merged(self, other: Self) -> Self {
        Self {
            next: self.next.max(other.next),
            exhausted: self.exhausted || other.exhausted,
        }
    }

    fn floor(self) -> u64 {
        if self.exhausted {
            u64::MAX
        } else {
            self.next.saturating_sub(1)
        }
    }

    /// A live `MAX` identity leaves no further number; a stored `MAX` counter
    /// without a live `MAX` still has `MAX` available.
    fn captured(stored: u64, highest: u64) -> Self {
        Self {
            next: stored.max(highest.saturating_add(1)),
            exhausted: stored == u64::MAX || highest == u64::MAX,
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
struct IdentityCounters {
    opening: IdentityCounter,
    vertex: IdentityCounter,
    critical_fit: IdentityCounter,
    hardware: IdentityCounter,
    mount: IdentityCounter,
}

impl IdentityCounters {
    fn merged(self, other: Self) -> Self {
        Self {
            opening: self.opening.merged(other.opening),
            vertex: self.vertex.merged(other.vertex),
            critical_fit: self.critical_fit.merged(other.critical_fit),
            hardware: self.hardware.merged(other.hardware),
            mount: self.mount.merged(other.mount),
        }
    }

    fn from_reservation(reservation: MechanicalIdentityReservation) -> Self {
        Self {
            opening: IdentityCounter {
                next: reservation.next_opening_identity,
                exhausted: reservation.next_opening_identity == u64::MAX,
            },
            vertex: IdentityCounter {
                next: reservation.next_vertex_identity,
                exhausted: reservation.next_vertex_identity == u64::MAX,
            },
            critical_fit: IdentityCounter {
                next: reservation.next_critical_fit_identity,
                exhausted: reservation.next_critical_fit_identity == u64::MAX,
            },
            hardware: IdentityCounter {
                next: reservation.next_hardware_identity,
                exhausted: reservation.next_hardware_identity == u64::MAX,
            },
            mount: IdentityCounter {
                next: reservation.next_mount_identity,
                exhausted: reservation.next_mount_identity == u64::MAX,
            },
        }
    }
}

/// Session high-water marks for mechanical identities that outlive Undo snapshots,
/// disabled configurations and temporarily absent owners.
///
/// Core owns allocation; this ledger only raises reservations and never mints an
/// identity. A successful explicit Open starts a new ledger (a new document session);
/// failed opens, previews, failed edits, undo/redo, panel remounts and disable/re-enable
/// keep it.
#[derive(Clone, Debug, Default)]
pub(crate) struct MechanicalIdentityLedger {
    configs: BTreeMap<IdentityConfigOwner, IdentityCounters>,
    vertices: BTreeMap<IdentityVertexOwner, IdentityCounter>,
}

impl MechanicalIdentityLedger {
    /// A ledger seeded from every live identity and stored counter in `document`.
    pub(crate) fn seeded(document: &ProjectDoc) -> Self {
        let mut ledger = Self::default();
        ledger.capture(document);
        ledger
    }

    /// Raises the ledger to cover every identity and counter currently in `document`.
    pub(crate) fn capture(&mut self, document: &ProjectDoc) {
        if let Some(configuration) = &document.mechanical {
            self.capture_config(
                IdentityConfigOwner::Board(configuration.board_id.clone()),
                configuration,
            );
        }
        let Some(hardware) = &document.hardware else {
            return;
        };
        for instance in &hardware.instances {
            let owner = IdentityConfigOwner::Instance(instance.id.clone());
            if let Some(configuration) = &instance.mechanical {
                self.capture_config(owner.clone(), configuration);
            }
            if let Some(reservation) = instance.mechanical_identity_reservation {
                let counters = IdentityCounters::from_reservation(reservation);
                self.configs
                    .entry(owner)
                    .and_modify(|entry| *entry = entry.merged(counters))
                    .or_insert(counters);
            }
        }
        // A linked opening is stored only in the pair's authored value; its vertex
        // counter still belongs to each member's vertex namespace.
        for pair in &hardware.pairs {
            for feature in &pair.features {
                let MechanicalFeatureValue::Opening { opening } = &feature.value else {
                    continue;
                };
                let counter = IdentityCounter::captured(
                    opening.next_vertex_identity,
                    opening
                        .vertices
                        .iter()
                        .map(|vertex| identity_index(&vertex.id))
                        .max()
                        .unwrap_or(0),
                );
                for member in &feature.members {
                    let key = IdentityVertexOwner {
                        config: IdentityConfigOwner::Instance(member.instance_id.clone()),
                        opening_id: member.feature_id.clone(),
                    };
                    self.vertices
                        .entry(key)
                        .and_modify(|entry| *entry = entry.merged(counter))
                        .or_insert(counter);
                }
            }
        }
    }

    fn capture_config(
        &mut self,
        owner: IdentityConfigOwner,
        configuration: &MechanicalConfiguration,
    ) {
        let highest_opening = configuration
            .openings
            .iter()
            .flatten()
            .map(|opening| identity_index(&opening.id))
            .max()
            .unwrap_or(0);
        let highest_vertex = configuration
            .openings
            .iter()
            .flatten()
            .flat_map(|opening| opening.vertices.iter())
            .map(|vertex| identity_index(&vertex.id))
            .max()
            .unwrap_or(0);
        let highest_critical_fit = configuration
            .critical_fits
            .iter()
            .flatten()
            .filter_map(|fit| identity_index_for(&fit.id, "case-critical-fit"))
            .max()
            .unwrap_or(0);
        let highest_hardware = configuration
            .hardware
            .iter()
            .flatten()
            .filter_map(|entry| identity_index_for(&entry.id, "case-mechanical-hardware"))
            .max()
            .unwrap_or(0);
        let highest_mount = configuration
            .mounts
            .iter()
            .chain(configuration.closure_mounts.iter().flatten())
            .filter_map(|mount| identity_index_for(&mount.id, "case-mechanical-mount"))
            .max()
            .unwrap_or(0);
        let counters = IdentityCounters {
            opening: IdentityCounter::captured(
                configuration.next_opening_identity,
                highest_opening,
            ),
            vertex: IdentityCounter::captured(
                configuration
                    .openings
                    .iter()
                    .flatten()
                    .map(|opening| opening.next_vertex_identity)
                    .max()
                    .unwrap_or(0),
                highest_vertex,
            ),
            critical_fit: IdentityCounter::captured(
                configuration.next_critical_fit_identity,
                highest_critical_fit,
            ),
            hardware: IdentityCounter::captured(
                configuration.next_hardware_identity,
                highest_hardware,
            ),
            mount: IdentityCounter::captured(configuration.next_mount_identity, highest_mount),
        };
        self.configs
            .entry(owner.clone())
            .and_modify(|entry| *entry = entry.merged(counters))
            .or_insert(counters);
        for opening in configuration.openings.iter().flatten() {
            let key = IdentityVertexOwner {
                config: owner.clone(),
                opening_id: opening.id.clone(),
            };
            let counter = IdentityCounter::captured(
                opening.next_vertex_identity,
                opening
                    .vertices
                    .iter()
                    .map(|vertex| identity_index(&vertex.id))
                    .max()
                    .unwrap_or(0),
            );
            self.vertices
                .entry(key)
                .and_modify(|entry| *entry = entry.merged(counter))
                .or_insert(counter);
        }
    }

    /// The session high-water floors for one target's allocation.
    ///
    /// Allocation merges these with the document's live identities and persisted
    /// counters, so Undo, a disabled configuration or a temporarily absent owner
    /// cannot make a removed identity available again.
    pub(crate) fn floors(&self, target: &MechanicalTarget) -> IdentityFloor {
        let owner = match target {
            MechanicalTarget::Board { board_id } => IdentityConfigOwner::Board(board_id.clone()),
            MechanicalTarget::Instance { instance_id } => {
                IdentityConfigOwner::Instance(instance_id.clone())
            }
        };
        let counters = self.configs.get(&owner).copied().unwrap_or_default();
        let vertex = self
            .vertices
            .iter()
            .filter(|(key, _)| key.config == owner)
            .map(|(_, counter)| counter.floor())
            .max()
            .unwrap_or(0)
            .max(counters.vertex.floor());
        IdentityFloor {
            opening: counters.opening.floor(),
            vertex,
            critical_fit: counters.critical_fit.floor(),
            hardware: counters.hardware.floor(),
            mount: counters.mount.floor(),
        }
    }
}

/// The highest opening and vertex identity a configuration can already hand out.
fn identity_reservation_of(
    configuration: &MechanicalConfiguration,
) -> MechanicalIdentityReservation {
    let openings = configuration.openings.as_ref();
    let highest_opening = openings
        .into_iter()
        .flatten()
        .map(|opening| identity_index(&opening.id))
        .max()
        .unwrap_or(0);
    let highest_vertex = openings
        .into_iter()
        .flatten()
        .flat_map(|opening| opening.vertices.iter())
        .map(|vertex| identity_index(&vertex.id))
        .max()
        .unwrap_or(0);
    MechanicalIdentityReservation {
        next_opening_identity: configuration
            .next_opening_identity
            .max(highest_opening.saturating_add(1)),
        next_vertex_identity: openings
            .into_iter()
            .flatten()
            .map(|opening| opening.next_vertex_identity)
            .max()
            .unwrap_or(0)
            .max(highest_vertex.saturating_add(1)),
        next_critical_fit_identity: configuration.next_critical_fit_identity.max(
            configuration
                .critical_fits
                .as_ref()
                .into_iter()
                .flatten()
                .map(|fit| identity_index_for(&fit.id, "case-critical-fit").unwrap_or(0))
                .max()
                .unwrap_or(0)
                .saturating_add(1),
        ),
        next_hardware_identity: configuration.next_hardware_identity.max(
            configuration
                .hardware
                .as_ref()
                .into_iter()
                .flatten()
                .map(|entry| identity_index_for(&entry.id, "case-mechanical-hardware").unwrap_or(0))
                .max()
                .unwrap_or(0)
                .saturating_add(1),
        ),
        next_mount_identity: configuration.next_mount_identity.max(
            configuration
                .mounts
                .iter()
                .chain(configuration.closure_mounts.iter().flatten())
                .filter_map(|mount| identity_index_for(&mount.id, "case-mechanical-mount"))
                .max()
                .unwrap_or(0)
                .saturating_add(1),
        ),
    }
}

/// Raises a materialized configuration's reservations to cover pair-owned identities.
///
/// Used when ownership ends and a half keeps the model it showed: the persisted counters
/// then cover every identity that was visible through the pair, so a later local addition
/// cannot take one of them.
fn merge_identity_floor(
    configuration: &mut MechanicalConfiguration,
    floor: IdentityFloor,
) -> Result<(), String> {
    let raised = |current: u64, floor: u64, kind: &str| -> Result<u64, String> {
        if floor == u64::MAX {
            if current == u64::MAX {
                return Ok(current);
            }
            return Err(exhausted_identity(kind));
        }
        Ok(current.max(floor + 1))
    };
    configuration.next_opening_identity = raised(
        configuration.next_opening_identity,
        floor.opening,
        "Opening",
    )?;
    if floor.critical_fit > 0 {
        configuration.next_critical_fit_identity = raised(
            configuration.next_critical_fit_identity,
            floor.critical_fit,
            "Critical fit",
        )?;
    }
    if floor.hardware > 0 {
        configuration.next_hardware_identity = raised(
            configuration.next_hardware_identity,
            floor.hardware,
            "Hardware",
        )?;
    }
    // Live mounts already seed allocation from their highest ordinal; only a
    // reserved mount that is not visible here needs a persisted counter.
    let highest_live_mount = configuration
        .mounts
        .iter()
        .chain(configuration.closure_mounts.iter().flatten())
        .filter_map(|mount| identity_index_for(&mount.id, "case-mechanical-mount"))
        .max()
        .unwrap_or(0);
    if floor.mount > 0 && floor.mount > highest_live_mount {
        configuration.next_mount_identity =
            raised(configuration.next_mount_identity, floor.mount, "Mount")?;
    }
    if let Some(openings) = configuration.openings.as_mut() {
        for opening in openings {
            opening.next_vertex_identity =
                raised(opening.next_vertex_identity, floor.vertex, "Vertex")?;
        }
    }
    Ok(())
}

/// The numeric suffix of a `prefix-N` identity, or `None` for another prefix.
fn identity_index_for(id: &str, prefix: &str) -> Option<u64> {
    id.strip_prefix(prefix)?.strip_prefix('-')?.parse().ok()
}

/// The numeric suffix of an `opening-N`/`vertex-N` identity, or zero.
fn identity_index(id: &str) -> u64 {
    id.rsplit('-')
        .next()
        .and_then(|index| index.parse().ok())
        .unwrap_or(0)
}

fn validate_mount_positive(value: f64, label: &str) -> Result<(), String> {
    if value.is_finite() && value > 0.0 {
        Ok(())
    } else {
        Err(format!("{label} must be a finite value greater than zero."))
    }
}

fn set_dimension(
    configuration: &mut MechanicalConfiguration,
    field: MechanicalDimension,
    value: f64,
) -> Result<(), String> {
    match field {
        MechanicalDimension::PlateThickness => configuration.plate_thickness = value,
        MechanicalDimension::PlateFoamThickness => configuration.plate_foam_thickness = value,
        MechanicalDimension::PcbThickness => configuration.pcb_thickness = value,
        MechanicalDimension::BottomFoamThickness => configuration.bottom_foam_thickness = value,
        MechanicalDimension::BottomThickness => configuration.bottom_thickness = value,
        MechanicalDimension::WallThickness => configuration.wall_thickness = value,
        MechanicalDimension::Clearance => configuration.clearance = value,
        MechanicalDimension::OpeningAllowance => configuration.opening_allowance = Some(value),
        MechanicalDimension::MountPositionX
        | MechanicalDimension::MountPositionY
        | MechanicalDimension::MountHoleDiameter
        | MechanicalDimension::MountBossDiameter
        | MechanicalDimension::MountBossHeight => {
            return Err("Mount dimensions require a current mount selection.".into());
        }
        MechanicalDimension::GasketSupportLength | MechanicalDimension::GasketSupportWidth => {
            return Err("Gasket support dimensions require a current support selection.".into());
        }
        MechanicalDimension::OpeningBottomZ
        | MechanicalDimension::OpeningHeight
        | MechanicalDimension::OpeningPointX
        | MechanicalDimension::OpeningPointY => {
            return Err("Opening dimensions require a current access opening selection.".into());
        }
        MechanicalDimension::ClosureThreadDiameter
        | MechanicalDimension::ClosurePitch
        | MechanicalDimension::ClosureHeadDiameter
        | MechanicalDimension::ClosureHeadHeight
        | MechanicalDimension::ClosureHoleDiameter
        | MechanicalDimension::ClosureInsertDiameter
        | MechanicalDimension::ClosureInsertLength
        | MechanicalDimension::ClosureSeatDiameter
        | MechanicalDimension::ClosureSeatDepth
        | MechanicalDimension::ClosureEngagement
        | MechanicalDimension::ClosureThreadStart
        | MechanicalDimension::ClosureTipAllowance
        | MechanicalDimension::ClosureBottomingClearance
        | MechanicalDimension::ClosureRoof
        | MechanicalDimension::ClosureSurround
        | MechanicalDimension::ClosureSeatLeadDepth
        | MechanicalDimension::ClosureSeatLeadDiameter
        | MechanicalDimension::ClosureBearingThickness => {
            let hardware = closure_hardware_mut(configuration)?;
            match field {
                MechanicalDimension::ClosureThreadDiameter => hardware.thread_diameter = value,
                MechanicalDimension::ClosurePitch => hardware.pitch = value,
                MechanicalDimension::ClosureHeadDiameter => hardware.head_diameter = value,
                MechanicalDimension::ClosureHeadHeight => hardware.head_height = value,
                MechanicalDimension::ClosureHoleDiameter => hardware.hole_diameter = value,
                MechanicalDimension::ClosureInsertDiameter => hardware.insert_diameter = value,
                MechanicalDimension::ClosureInsertLength => hardware.insert_length = value,
                MechanicalDimension::ClosureSeatDiameter => hardware.seat_diameter = value,
                MechanicalDimension::ClosureSeatDepth => hardware.seat_depth = value,
                MechanicalDimension::ClosureEngagement => hardware.engagement = value,
                MechanicalDimension::ClosureThreadStart => hardware.thread_start = value,
                MechanicalDimension::ClosureTipAllowance => hardware.tip_allowance = value,
                MechanicalDimension::ClosureBottomingClearance => {
                    hardware.bottoming_clearance = value
                }
                MechanicalDimension::ClosureRoof => hardware.roof = value,
                MechanicalDimension::ClosureSurround => hardware.surround = value,
                MechanicalDimension::ClosureSeatLeadDepth => hardware.seat_lead_depth = value,
                MechanicalDimension::ClosureSeatLeadDiameter => hardware.seat_lead_diameter = value,
                MechanicalDimension::ClosureBearingThickness => hardware.bearing_thickness = value,
                _ => unreachable!("closure dimension variant checked above"),
            }
        }
        MechanicalDimension::CriticalFitFromX
        | MechanicalDimension::CriticalFitFromY
        | MechanicalDimension::CriticalFitToX
        | MechanicalDimension::CriticalFitToY => {
            return Err("Critical-fit endpoint dimensions require a current fit selection.".into());
        }
        MechanicalDimension::HardwareLength | MechanicalDimension::HardwareQuantity => {
            return Err("Hardware dimensions require a current hardware selection.".into());
        }
        MechanicalDimension::BatteryWidth
        | MechanicalDimension::BatteryDepth
        | MechanicalDimension::BatteryHeight
        | MechanicalDimension::BatteryCableWidth
        | MechanicalDimension::BatteryPositionX
        | MechanicalDimension::BatteryPositionY
        | MechanicalDimension::BatteryCableExitX
        | MechanicalDimension::BatteryCableExitY => {
            let battery = configuration.battery.as_mut().ok_or_else(|| {
                "The battery envelope was removed before this field update.".to_owned()
            })?;
            match field {
                MechanicalDimension::BatteryWidth => battery.size.x = value,
                MechanicalDimension::BatteryDepth => battery.size.y = value,
                MechanicalDimension::BatteryHeight => battery.size.z = value,
                MechanicalDimension::BatteryCableWidth => battery.cable_width = Some(value),
                MechanicalDimension::BatteryPositionX => battery.at.x = value,
                MechanicalDimension::BatteryPositionY => battery.at.y = value,
                MechanicalDimension::BatteryCableExitX => battery.cable_exit.x = value,
                MechanicalDimension::BatteryCableExitY => battery.cable_exit.y = value,
                _ => unreachable!("battery dimension variant checked above"),
            }
        }
    }
    Ok(())
}

fn validate_dimension(field: MechanicalDimension, value: f64) -> Result<(), String> {
    let valid = value.is_finite()
        && match field {
            MechanicalDimension::OpeningAllowance => (-1.0..=1.0).contains(&value),
            MechanicalDimension::BatteryWidth
            | MechanicalDimension::BatteryDepth
            | MechanicalDimension::BatteryHeight
            | MechanicalDimension::BatteryCableWidth
            | MechanicalDimension::HardwareLength => value >= 0.1,
            MechanicalDimension::HardwareQuantity => {
                value >= 1.0 && value.fract() == 0.0 && value <= f64::from(u32::MAX)
            }
            MechanicalDimension::OpeningHeight => value >= 0.1,
            MechanicalDimension::GasketSupportLength => value >= 5.0,
            MechanicalDimension::GasketSupportWidth => value >= 0.5,
            MechanicalDimension::OpeningBottomZ
            | MechanicalDimension::OpeningPointX
            | MechanicalDimension::OpeningPointY
            | MechanicalDimension::BatteryPositionX
            | MechanicalDimension::BatteryPositionY
            | MechanicalDimension::BatteryCableExitX
            | MechanicalDimension::BatteryCableExitY
            | MechanicalDimension::MountPositionX
            | MechanicalDimension::MountPositionY
            | MechanicalDimension::CriticalFitFromX
            | MechanicalDimension::CriticalFitFromY
            | MechanicalDimension::CriticalFitToX
            | MechanicalDimension::CriticalFitToY => {
                value >= -boardstudio_contracts::MAX_COORDINATE_MM
            }
            _ => value >= 0.0,
        };
    if valid {
        Ok(())
    } else if field == MechanicalDimension::GasketSupportLength {
        Err("Cut length must be at least 5 mm.".into())
    } else if field == MechanicalDimension::GasketSupportWidth {
        Err("Pad width must be at least 0.5 mm.".into())
    } else if field == MechanicalDimension::OpeningAllowance {
        Err("Opening allowance must be between −1 and 1 mm.".into())
    } else if matches!(
        field,
        MechanicalDimension::BatteryWidth
            | MechanicalDimension::BatteryDepth
            | MechanicalDimension::BatteryHeight
            | MechanicalDimension::BatteryCableWidth
    ) {
        Err("Battery dimensions and cable width must be at least 0.1 mm.".into())
    } else if field == MechanicalDimension::HardwareLength {
        Err("Hardware length must be at least 0.1 mm.".into())
    } else if field == MechanicalDimension::HardwareQuantity {
        Err("Hardware quantity must be a positive whole number.".into())
    } else if field == MechanicalDimension::OpeningHeight {
        Err("Access opening height must be at least 0.1 mm.".into())
    } else if matches!(
        field,
        MechanicalDimension::BatteryPositionX
            | MechanicalDimension::BatteryPositionY
            | MechanicalDimension::BatteryCableExitX
            | MechanicalDimension::BatteryCableExitY
    ) {
        Err("Battery positions and cable exits must be at least −1,000,000 mm.".into())
    } else if matches!(
        field,
        MechanicalDimension::CriticalFitFromX
            | MechanicalDimension::CriticalFitFromY
            | MechanicalDimension::CriticalFitToX
            | MechanicalDimension::CriticalFitToY
    ) {
        Err("Critical-fit coordinates must be at least −1,000,000 mm.".into())
    } else {
        Err("Mechanical dimensions must be finite and nonnegative.".into())
    }
}

fn normalize_processes(configuration: &mut MechanicalConfiguration, change: &MechanicalChange) {
    let processes = configuration.part_processes.get_or_insert_with(Vec::new);
    let top_level_method_change = matches!(change, MechanicalChange::SetMethod(_))
        || matches!(
            change,
            MechanicalChange::SetProcessMethod { part_id, .. } if part_id == "plate"
        );
    for process in processes.iter_mut() {
        let is_standard = matches!(
            process.part_id.as_str(),
            "plate" | "plate-foam" | "bottom-foam" | "bottom"
        );
        let is_foam = process.part_id.ends_with("foam");
        let method = if is_foam {
            PlateMethod::CutSheet
        } else if process.part_id == "plate" || (top_level_method_change && is_standard) {
            configuration.method.clone()
        } else {
            process.method.clone()
        };
        let method_changed =
            method != process.method || (top_level_method_change && is_standard && !is_foam);
        let valid_material =
            material_options(&process.part_id, &method).contains(&process.material.as_str());
        if is_standard {
            if method_changed || !valid_material {
                process.material =
                    crate::mechanical::default_material(&process.part_id, &method).into();
            }
            process.method = method;
            process.thickness = match process.part_id.as_str() {
                "plate" => configuration.plate_thickness,
                "plate-foam" => configuration.plate_foam_thickness,
                "bottom-foam" => configuration.bottom_foam_thickness,
                "bottom" => configuration.bottom_thickness,
                _ => process.thickness,
            };
        }
    }
}

fn is_process_target(document: &ProjectDoc, part_id: &str) -> bool {
    matches!(part_id, "plate" | "plate-foam" | "bottom-foam" | "bottom")
        || document.parts.iter().any(|part| part.id == part_id)
}

fn process_default_thickness(configuration: &MechanicalConfiguration, part_id: &str) -> f64 {
    match part_id {
        "plate" => configuration.plate_thickness,
        "plate-foam" => configuration.plate_foam_thickness,
        "bottom-foam" => configuration.bottom_foam_thickness,
        "bottom" => configuration.bottom_thickness,
        _ => configuration.plate_thickness,
    }
}

fn process_mut<'a>(
    configuration: &'a mut MechanicalConfiguration,
    document: &ProjectDoc,
    part_id: &str,
) -> Result<&'a mut MechanicalPartProcess, String> {
    if !is_process_target(document, part_id) {
        return Err("The selected Case part is no longer available.".into());
    }
    configuration
        .part_processes
        .as_mut()
        .and_then(|processes| {
            processes
                .iter_mut()
                .find(|process| process.part_id == part_id)
        })
        .ok_or_else(|| "The selected process override is no longer available.".to_owned())
}

fn stabilizer_target_exists(document: &ProjectDoc, board_id: &str, part_id: &str) -> bool {
    let Some(board) = document.boards.iter().find(|board| board.id == board_id) else {
        return false;
    };
    let Some(part) = document
        .parts
        .iter()
        .find(|part| part.id == part_id && board.part_ids.contains(&part.id))
    else {
        return false;
    };
    let Some(definition) = document
        .definitions
        .iter()
        .find(|definition| definition.id == part.definition_id)
    else {
        return false;
    };
    let Some(size) = part.keycap.or(definition.keycap) else {
        return false;
    };
    if size.x.max(size.y) < 37.0
        || definition
            .generator
            .as_ref()
            .is_none_or(|generator| generator.source.to_lowercase() != "ceoloide/switch_mx")
    {
        return false;
    }
    !document.parts.iter().any(|stabilizer| {
        board.part_ids.contains(&stabilizer.id)
            && document
                .definitions
                .iter()
                .find(|candidate| candidate.id == stabilizer.definition_id)
                .and_then(|candidate| candidate.kicad_source.as_ref())
                .is_some_and(|source| source.source.contains("(footprint \"STAB_MX_"))
            && (stabilizer.pose.at.x - part.pose.at.x).hypot(stabilizer.pose.at.y - part.pose.at.y)
                < 0.01
    })
}

fn material_options(part_id: &str, method: &PlateMethod) -> &'static [&'static str] {
    if part_id.ends_with("foam") {
        &["EVA"]
    } else {
        match method {
            PlateMethod::Printed => &["PLA", "ABS"],
            PlateMethod::Cnc => &["Aluminium"],
            PlateMethod::CutSheet => &["Acrylic"],
            PlateMethod::PcbFr4 => &["FR-4"],
        }
    }
}

fn profile_family(configuration: &MechanicalConfiguration) -> Option<MechanicalSwitchFamily> {
    let mut families = configuration
        .profiles
        .iter()
        .filter_map(|profile| profile.switch_family);
    let first = families.next()?;
    families.all(|family| family == first).then_some(first)
}

fn initial_switch_family(document: &ProjectDoc, board_id: &str) -> Option<MechanicalSwitchFamily> {
    let board = document.boards.iter().find(|board| board.id == board_id)?;
    let candidates = document
        .parts
        .iter()
        .filter(|part| board.part_ids.contains(&part.id))
        .filter_map(|part| {
            let definition = document
                .definitions
                .iter()
                .find(|definition| definition.id == part.definition_id)?;
            is_switch_position(part, definition).then(|| inferred_switch_family(part, definition))
        })
        .collect::<Vec<_>>();
    let [Some(first), rest @ ..] = candidates.as_slice() else {
        return None;
    };
    rest.iter()
        .all(|family| family.as_ref() == Some(first))
        .then_some(*first)
}

fn family_assignment_changes(
    existing_family: Option<MechanicalSwitchFamily>,
    selected_family: MechanicalSwitchFamily,
) -> bool {
    existing_family != Some(selected_family)
}

fn closure_hardware_mut(
    configuration: &mut MechanicalConfiguration,
) -> Result<&mut InternalClosureHardware, String> {
    configuration
        .internal_gasket
        .as_mut()
        .map(|settings| &mut settings.hardware)
        .ok_or_else(|| "Internal gasket closure hardware is no longer available.".into())
}

fn critical_fit_mut<'a>(
    configuration: &'a mut MechanicalConfiguration,
    fit_id: &str,
) -> Result<&'a mut MechanicalCriticalFit, String> {
    configuration
        .critical_fits
        .as_mut()
        .and_then(|fits| fits.iter_mut().find(|fit| fit.id == fit_id))
        .ok_or_else(|| "The selected critical fit is no longer available.".into())
}

fn hardware_mut<'a>(
    configuration: &'a mut MechanicalConfiguration,
    hardware_id: &str,
) -> Result<&'a mut MechanicalHardwareSpecification, String> {
    configuration
        .hardware
        .as_mut()
        .and_then(|entries| entries.iter_mut().find(|entry| entry.id == hardware_id))
        .ok_or_else(|| "The selected hardware entry is no longer available.".into())
}

fn resize_closure_insert(hardware: &mut InternalClosureHardware, id: &str) -> Result<(), String> {
    let (thread_diameter, pitch, insert_length, insert_diameter) = match id {
        "m2-3" => (2.0, 0.4, 3.0, 3.2),
        "m2-4" => (2.0, 0.4, 4.0, 3.2),
        "m2.5-3" => (2.5, 0.45, 3.0, 3.5),
        "m2.5-4" => (2.5, 0.45, 4.0, 3.5),
        "m2.5-5" => (2.5, 0.45, 5.0, 3.5),
        "m3-3" => (3.0, 0.5, 3.0, 4.2),
        _ => return Err("The selected insert preset is unavailable.".into()),
    };
    hardware.id = format!("custom-{id}");
    hardware.thread = format!("M{thread_diameter} × {pitch}");
    hardware.thread_diameter = thread_diameter;
    hardware.pitch = pitch;
    hardware.insert_length = insert_length;
    hardware.insert_diameter = insert_diameter;
    hardware.seat_diameter = insert_diameter - 0.4;
    hardware.seat_lead_diameter = insert_diameter - 0.2;
    hardware.seat_depth = insert_length + hardware.bottoming_clearance;
    hardware.engagement = (insert_length - 0.5).min(insert_length - hardware.thread_start);
    hardware.hole_diameter = thread_diameter + 0.2;
    hardware.head_diameter = thread_diameter * 2.0;
    hardware.head_height = thread_diameter / 2.0;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mechanical::change::{
        MechanicalChange, MechanicalDimension, MechanicalFeatureCorrespondence,
    };
    use crate::model::{
        HardwareConfiguration, HardwareTopology, HardwareTransport, MechanicalConfiguration,
        MechanicalConstructionOwnership, MechanicalFeatureKind, MechanicalFeatureState,
        MechanicalFeatureValue, MechanicalGasketAnchor, MechanicalMirrorPair, Mount, MountKind,
        PartDefinition, PhysicalBoardInstance, Pose2, Vec2,
    };
    use serde_json::json;

    fn board(id: &str, part_ids: Vec<String>) -> Board {
        Board {
            id: id.into(),
            name: id.into(),
            outline_ids: vec![],
            part_ids,
            net_ids: vec![],
            thickness: 1.6,
            traces: vec![],
            vias: vec![],
        }
    }

    fn configuration() -> MechanicalConfiguration {
        serde_json::from_value(json!({
            "boardId": "board",
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
            // Deliberate empty closure collection: most settings tests must not
            // trigger closure initialization and its required template.
            "closureMounts": [],
            "profiles": []
        }))
        .unwrap()
    }

    /// The same configuration with absent closure mounts, for initialization tests.
    fn configuration_without_closures() -> MechanicalConfiguration {
        let mut configuration = configuration();
        configuration.closure_mounts = None;
        configuration
    }

    /// The normalized bundled mounting-hole catalogue entry projection expects.
    fn template() -> PartDefinition {
        serde_json::from_value(json!({
            "id":"generator:ceoloide/mounting_hole_npth", "name":"mounting hole npth",
            "kind":"custom", "pads":[{"id":"pad-0", "number":"", "at":{"x":0,"y":0},
                "size":{"x":2.2,"y":2.2}, "shape":"circle", "drill":2.2,"plated":false,"rotation":0}],
            "courtyard":[{"x":-1.1,"y":-1.1},{"x":1.1,"y":-1.1},{"x":1.1,"y":1.1},{"x":-1.1,"y":1.1}],
            "envelopeSource":{"courtyard":"generated"},
            "envelopeNotice":"No closed courtyard is available; the outline uses physical graphics and pad extents.",
            "generator":{"source":"ceoloide/mounting_hole_npth", "version":"bundled-1", "parameters":{}}
        }))
        .unwrap()
    }

    fn document() -> ProjectDoc {
        let mut document = ProjectDoc::empty("doc", "Fixture");
        document.boards.push(board("board", vec![]));
        document.mechanical = Some(configuration());
        document
    }

    /// A board whose outline yields at least one suggested closure mount.
    fn document_with_outline() -> ProjectDoc {
        let mut document = ProjectDoc::empty("doc", "Fixture");
        document.outline.push(OutlineFeature::Rect {
            anchor_part_id: None,
            id: "outline".into(),
            center: Vec2 { x: 0.0, y: 0.0 },
            size: Vec2 { x: 80.0, y: 60.0 },
            radius: 0.0,
            rotation: None,
            operation: Operation::Add,
        });
        document.boards.push(board("board", vec![]));
        document.boards[0].outline_ids = vec!["outline".into()];
        document.mechanical = Some(configuration_without_closures());
        document
    }

    fn board_target() -> MechanicalTarget {
        MechanicalTarget::Board {
            board_id: "board".into(),
        }
    }

    fn instance_target() -> MechanicalTarget {
        MechanicalTarget::Instance {
            instance_id: "left".into(),
        }
    }

    fn apply_change(doc: &mut ProjectDoc, change: MechanicalChange) -> Result<Vec<String>, String> {
        apply(
            doc,
            &[],
            &MechanicalEdit {
                target: board_target(),
                change,
                mounting_hole: Some(Box::new(template())),
            },
        )
    }

    fn stored(doc: &ProjectDoc) -> &MechanicalConfiguration {
        doc.mechanical.as_ref().expect("stored configuration")
    }

    fn anchor(id: &str, anchor: f64) -> MechanicalGasketAnchor {
        MechanicalGasketAnchor {
            length: Some(12.0),
            width: Some(3.0),
            placement: Some(GasketPlacement::User),
            id: id.into(),
            region_id: id.into(),
            outline_key: id.into(),
            anchor,
            unlinked: false,
        }
    }

    #[test]
    fn the_initial_switch_family_follows_generator_inference_and_part_overrides() {
        let choc = "ceoloide/switch_choc_v1_v2";
        type Generator<'a> = Option<(&'a str, serde_json::Value)>;
        let v1_params = json!({"choc_v1_support":true,"choc_v2_support":false});
        let family_document =
            |parts: &[(&str, &str, Option<serde_json::Value>)],
             definitions: &[(&str, &str, Generator)]| {
                let mut doc = ProjectDoc::empty("family", "family");
                doc.boards.push(board(
                    "board",
                    parts.iter().map(|(id, _, _)| id.to_string()).collect(),
                ));
                for (id, definition_id, parameters) in parts {
                    let mut part = json!({
                        "id":id, "definitionId":definition_id, "reference":id,
                        "pose":{"at":{"x":0,"y":0},"rotation":0}, "side":"front"
                    });
                    if let Some(parameters) = parameters {
                        part["generatorParameters"] = parameters.clone();
                    }
                    doc.parts.push(serde_json::from_value(part).unwrap());
                }
                for (id, kind, generator) in definitions {
                    let mut definition = json!({
                        "id":id, "name":id, "kind":kind, "courtyard":[], "pads":[]
                    });
                    if let Some((source, parameters)) = generator {
                        definition["generator"] =
                            json!({"source":source,"version":"1","parameters":parameters});
                    }
                    doc.definitions
                        .push(serde_json::from_value(definition).unwrap());
                }
                doc
            };
        use MechanicalSwitchFamily::{ChocV1, ChocV2, Mx};
        for (doc, expected) in [
            (
                family_document(
                    &[("a", "mx", None)],
                    &[("mx", "switch", Some(("ceoloide/switch_mx", json!({}))))],
                ),
                Some(Mx),
            ),
            // A family-bearing generator admits a part whose kind is not a switch.
            (
                family_document(
                    &[("a", "mx", None)],
                    &[("mx", "controller", Some(("Ceoloide/Switch_MX", json!({}))))],
                ),
                Some(Mx),
            ),
            (
                family_document(
                    &[(
                        "a",
                        "c",
                        Some(json!({"choc_v1_support":false,"choc_v2_support":true})),
                    )],
                    &[("c", "switch", Some((choc, v1_params.clone())))],
                ),
                Some(ChocV2),
            ),
            (
                family_document(
                    &[("a", "c", None)],
                    &[("c", "switch", Some((choc, json!({}))))],
                ),
                None,
            ),
            (
                family_document(
                    &[("a", "mx", None), ("b", "c", None)],
                    &[
                        ("mx", "switch", Some(("ceoloide/switch_mx", json!({})))),
                        ("c", "switch", Some((choc, v1_params.clone()))),
                    ],
                ),
                None,
            ),
            (
                family_document(
                    &[("a", "c", None), ("b", "c", None)],
                    &[("c", "switch", Some((choc, v1_params)))],
                ),
                Some(ChocV1),
            ),
            (
                family_document(&[("a", "s", None)], &[("s", "switch", None)]),
                None,
            ),
            (
                family_document(&[("a", "k", None)], &[("k", "controller", None)]),
                None,
            ),
        ] {
            assert_eq!(initial_switch_family(&doc, "board"), expected);
        }
    }

    #[test]
    fn reflected_feature_installation_preserves_each_members_identity() {
        let opening = MechanicalOpening {
            id: "left-feature".into(),
            vertices: vec![MechanicalOpeningVertex {
                id: "vertex-4".into(),
                point: Vec2 { x: 2.0, y: 3.0 },
            }],
            z: 1.0,
            height: 5.0,
            next_vertex_identity: 5,
        };
        let values = [
            (
                MechanicalFeatureKind::Mount,
                MechanicalFeatureValue::Mount {
                    mount: mount("left-feature", 8.0, 9.0),
                },
            ),
            (
                MechanicalFeatureKind::ClosureMount,
                MechanicalFeatureValue::Mount {
                    mount: mount("left-feature", 8.0, 9.0),
                },
            ),
            (
                MechanicalFeatureKind::Opening,
                MechanicalFeatureValue::Opening { opening },
            ),
            (
                MechanicalFeatureKind::GasketSupport,
                MechanicalFeatureValue::GasketSupport {
                    anchor: anchor("left-feature", 0.25),
                },
            ),
        ];
        for (kind, value) in values {
            let mut configuration = configuration();
            install_feature(&mut configuration, kind, "right-feature", &value).unwrap();
            let installed = find_feature_value(&configuration, kind, "right-feature")
                .unwrap_or_else(|| panic!("the {kind:?} counterpart must use its member identity"));
            match (&value, installed) {
                (
                    MechanicalFeatureValue::Mount { mount: authored },
                    MechanicalFeatureValue::Mount { mount: installed },
                ) => {
                    assert_eq!(installed.id, "right-feature");
                    assert_eq!(installed.at, authored.at);
                    assert_eq!(installed.hole_diameter, authored.hole_diameter);
                }
                (
                    MechanicalFeatureValue::Opening { opening: authored },
                    MechanicalFeatureValue::Opening { opening: installed },
                ) => {
                    assert_eq!(installed.id, "right-feature");
                    assert_eq!(installed.vertices, authored.vertices);
                    assert_eq!(installed.z, authored.z);
                    assert_eq!(installed.height, authored.height);
                    assert_eq!(
                        installed.next_vertex_identity,
                        authored.next_vertex_identity
                    );
                }
                (
                    MechanicalFeatureValue::GasketSupport { anchor: authored },
                    MechanicalFeatureValue::GasketSupport { anchor: installed },
                ) => {
                    assert_eq!(installed.id, "right-feature");
                    assert_eq!(installed.region_id, authored.region_id);
                    assert_eq!(installed.outline_key, authored.outline_key);
                    assert_eq!(installed.anchor, authored.anchor);
                }
                _ => panic!("installation must preserve the feature kind"),
            }
            assert!(find_feature_value(&configuration, kind, "left-feature").is_none());
            let once = configuration.clone();
            install_feature(&mut configuration, kind, "right-feature", &value).unwrap();
            assert_eq!(
                configuration, once,
                "reinstallation replaces the same member"
            );
        }
    }

    #[test]
    fn a_removed_opening_or_vertex_retires_a_later_queued_edit_without_redirecting_it() {
        // An earlier queued edit addressed to a surviving vertex must still hit
        // that vertex after a different opening was removed before it.
        let mut doc = document();
        apply_change(&mut doc, MechanicalChange::AddOpening).unwrap();
        apply_change(&mut doc, MechanicalChange::AddOpening).unwrap();
        assert_eq!(
            stored(&doc)
                .openings
                .as_ref()
                .unwrap()
                .iter()
                .map(|opening| opening.id.as_str())
                .collect::<Vec<_>>(),
            vec!["opening-1", "opening-2"]
        );
        apply_change(
            &mut doc,
            MechanicalChange::SetOpeningDimension {
                opening_id: "opening-2".into(),
                vertex_id: Some("vertex-1".into()),
                field: MechanicalDimension::OpeningPointX,
                value: 4.0,
            },
        )
        .unwrap();
        apply_change(
            &mut doc,
            MechanicalChange::RemoveOpening {
                opening_id: "opening-1".into(),
            },
        )
        .unwrap();
        // The surviving opening keeps its durable id even though it is now index 0.
        assert_eq!(stored(&doc).openings.as_ref().unwrap().len(), 1);
        apply_change(
            &mut doc,
            MechanicalChange::SetOpeningDimension {
                opening_id: "opening-2".into(),
                vertex_id: Some("vertex-1".into()),
                field: MechanicalDimension::OpeningPointX,
                value: 9.0,
            },
        )
        .unwrap();
        let opening = &stored(&doc).openings.as_ref().unwrap()[0];
        assert_eq!(opening.id, "opening-2");
        assert_eq!(opening.vertices[0].id, "vertex-1");
        assert_eq!(opening.vertices[0].point.x, 9.0);

        // An edit whose addressed vertex was removed is a Core error and mutates nothing.
        apply_change(
            &mut doc,
            MechanicalChange::RemoveOpeningVertex {
                opening_id: "opening-2".into(),
                vertex_id: "vertex-1".into(),
            },
        )
        .unwrap();
        let before = stored(&doc).clone();
        assert!(
            apply_change(
                &mut doc,
                MechanicalChange::SetOpeningDimension {
                    opening_id: "opening-2".into(),
                    vertex_id: Some("vertex-1".into()),
                    field: MechanicalDimension::OpeningPointX,
                    value: 5.0,
                },
            )
            .is_err(),
            "a removed vertex id is no longer a valid target"
        );
        assert_eq!(stored(&doc), &before);
        assert!(
            apply_change(
                &mut doc,
                MechanicalChange::SetOpeningDimension {
                    opening_id: "opening-1".into(),
                    vertex_id: Some("vertex-1".into()),
                    field: MechanicalDimension::OpeningPointX,
                    value: 5.0,
                },
            )
            .is_err(),
            "a removed opening id is no longer a valid target"
        );
        assert_eq!(stored(&doc), &before);
    }

    /// Case 1: absent closure mounts initialize from the resolved assembly, and an
    /// explicit `Some(vec![])` stays deliberate.
    #[test]
    fn absent_closure_mounts_initialize_from_the_resolved_assembly() {
        let mut doc = document_with_outline();
        assert!(stored(&doc).closure_mounts.is_none());
        let changed = apply_change(&mut doc, MechanicalChange::InitializeClosures).unwrap();
        let mounts = stored(&doc)
            .closure_mounts
            .clone()
            .expect("initialized mounts");
        assert!(!mounts.is_empty(), "the outline suggests closure mounts");
        for mount in &mounts {
            assert!(mount.id.starts_with("auto-closure/"));
            assert_eq!(mount.kind, MountKind::Boss);
            assert_eq!(mount.hole_diameter, 2.2);
            assert_eq!(
                mount.height,
                Some(
                    stored(&doc).plate_to_pcb
                        + stored(&doc).pcb_thickness
                        + stored(&doc)
                            .bottom_foam_thickness
                            .max(stored(&doc).battery_height)
                )
            );
        }
        assert!(changed.contains(&"board".to_string()));

        // Explicit empty is deliberate and never re-initialized.
        let mut deliberate = document_with_outline();
        deliberate.mechanical.as_mut().unwrap().closure_mounts = Some(vec![]);
        assert!(
            apply_change(&mut deliberate, MechanicalChange::InitializeClosures).is_err(),
            "an explicit empty collection is not eligible for initialization"
        );
        assert_eq!(stored(&deliberate).closure_mounts, Some(vec![]));
    }

    /// Case 2 and 4: generated clearance parts, definitions and memberships appear
    /// with the configuration owner and nothing else.
    #[test]
    fn initialization_reports_only_generated_ids_and_the_owner() {
        let mut doc = document_with_outline();
        let changed = apply_change(&mut doc, MechanicalChange::InitializeClosures).unwrap();
        assert_eq!(changed[0], "board", "the configuration owner leads");
        assert!(!doc.parts.is_empty(), "generated clearance parts exist");
        for part in &doc.parts {
            assert!(
                changed.contains(&part.id),
                "generated part {} is reported",
                part.id
            );
        }
        for definition in &doc.definitions {
            assert!(changed.contains(&definition.id));
        }
        assert!(changed.contains(&"board".to_string()));
        for id in &changed {
            assert!(
                id == "board"
                    || id.starts_with("case-closure/")
                    || id.starts_with("assembly-closure/definition/"),
                "unexpected changed id {id}"
            );
        }
    }

    /// Case 5: a required projection without a template rolls back the configuration.
    ///
    /// The stored closure mount makes the projection genuinely create a generated
    /// hole, so the normalized entry is required; a change whose projected hole set is
    /// empty would not need one.
    #[test]
    fn a_missing_required_template_rolls_back_the_configuration() {
        let mut doc = document();
        doc.mechanical.as_mut().unwrap().closure_mounts = Some(vec![mount("closure-1", 4.0, 0.0)]);
        let before = doc.clone();
        let error = apply(
            &mut doc,
            &[],
            &MechanicalEdit {
                target: board_target(),
                change: MechanicalChange::AddOpening,
                mounting_hole: None,
            },
        )
        .unwrap_err();
        assert!(error.contains("mounting-hole"), "{error}");
        assert_eq!(doc, before, "the configuration change rolled back");
    }

    fn mirror_instance(id: &str, mounts: Vec<Mount>) -> PhysicalBoardInstance {
        let mut mechanical = configuration();
        mechanical.mounts = mounts;
        PhysicalBoardInstance {
            id: id.into(),
            name: id.into(),
            board_id: "board".into(),
            half: id.into(),
            role: "controller".into(),
            flipped: false,
            controller_part_id: None,
            mechanical: Some(mechanical),
            construction_ownership: MechanicalConstructionOwnership::Local,
            mechanical_identity_reservation: None,
        }
    }

    fn mount(id: &str, x: f64, y: f64) -> Mount {
        Mount {
            id: id.into(),
            at: Vec2 { x, y },
            kind: MountKind::Hole,
            hole_diameter: 2.2,
            boss_diameter: None,
            height: None,
        }
    }

    /// Two physical halves on one board, each with its own local mount geometry.
    fn mirror_document() -> ProjectDoc {
        let mut doc = ProjectDoc::empty("doc", "Fixture");
        doc.boards.push(board("board", vec![]));
        doc.mechanical = Some(configuration());
        doc.hardware = Some(HardwareConfiguration {
            topology: HardwareTopology::Unibody,
            transport: HardwareTransport::None,
            instances: vec![
                mirror_instance("left", vec![mount("mount-1", 2.0, 3.0)]),
                mirror_instance("right", vec![mount("mount-1", 7.0, 3.0)]),
            ],
            boards: vec![],
            shared_construction: None,
            pairs: vec![],
        });
        doc
    }

    fn correspondence() -> MechanicalFeatureCorrespondence {
        MechanicalFeatureCorrespondence {
            kind: MechanicalFeatureKind::Mount,
            first_feature_id: "mount-1".into(),
            second_feature_id: "mount-1".into(),
            source_instance_id: None,
        }
    }

    fn link_edit(target: MechanicalTarget, pair_id: &str, source: &str) -> MechanicalEdit {
        MechanicalEdit {
            target,
            change: MechanicalChange::LinkMirrorPair {
                pair_id: pair_id.into(),
                members: ["left".into(), "right".into()],
                mapping: Pose2 {
                    at: Vec2 { x: 0.0, y: 0.0 },
                    rotation: 0.0,
                },
                construction_source: source.into(),
                correspondences: vec![correspondence()],
            },
            mounting_hole: Some(Box::new(template())),
        }
    }

    fn lifecycle_edit(change: MechanicalChange) -> MechanicalEdit {
        MechanicalEdit {
            target: instance_target(),
            change,
            mounting_hole: Some(Box::new(template())),
        }
    }

    fn pairs(doc: &ProjectDoc) -> &[MechanicalMirrorPair] {
        &doc.hardware.as_ref().unwrap().pairs
    }

    fn stored_instance<'a>(doc: &'a ProjectDoc, id: &str) -> &'a MechanicalConfiguration {
        doc.hardware
            .as_ref()
            .unwrap()
            .instances
            .iter()
            .find(|instance| instance.id == id)
            .unwrap()
            .mechanical
            .as_ref()
            .unwrap()
    }

    fn ownership(doc: &ProjectDoc, id: &str) -> MechanicalConstructionOwnership {
        doc.hardware
            .as_ref()
            .unwrap()
            .instances
            .iter()
            .find(|instance| instance.id == id)
            .unwrap()
            .construction_ownership
    }

    fn mount_at(configuration: &MechanicalConfiguration, id: &str) -> Vec2 {
        configuration
            .mounts
            .iter()
            .find(|mount| mount.id == id)
            .unwrap()
            .at
    }

    /// The visible geometry a physical half shows for a mount, resolved through the
    /// effective read path (pair-derived when the feature is linked).
    fn visible_mount(doc: &ProjectDoc, instance_id: &str, id: &str) -> Vec2 {
        mount_at(
            &effective_inputs(
                doc,
                &[],
                &MechanicalTarget::Instance {
                    instance_id: instance_id.into(),
                },
            )
            .unwrap()
            .configuration,
            id,
        )
    }

    /// Whether a physical half stores its own copy of a feature.
    ///
    /// A `Linked` feature is owned by the pair, so neither half may store a second
    /// writable copy that could drift from the authored value.
    fn stores_mount(doc: &ProjectDoc, instance_id: &str, id: &str) -> bool {
        stored_instance(doc, instance_id)
            .mounts
            .iter()
            .any(|mount| mount.id == id)
    }

    fn assert_point(actual: Vec2, expected: Vec2) {
        assert!(
            (actual.x - expected.x).abs() < 1e-9 && (actual.y - expected.y).abs() < 1e-9,
            "expected {expected:?}, got {actual:?}"
        );
    }

    fn linked_document() -> ProjectDoc {
        let mut doc = mirror_document();
        apply(
            &mut doc,
            &[],
            &link_edit(board_target(), "pair-left-right", "left"),
        )
        .unwrap();
        doc
    }

    #[test]
    fn linking_a_pair_shares_construction_and_sets_both_ownerships() {
        let mut doc = mirror_document();
        let changed = apply(
            &mut doc,
            &[],
            &link_edit(board_target(), "pair-left-right", "left"),
        )
        .unwrap();
        assert_eq!(changed, vec!["pair-left-right", "left", "right"]);
        let pair = &pairs(&doc)[0];
        assert_eq!(pair.members, ["left".to_string(), "right".to_string()]);
        // The source's shared construction excludes its board-local mounts.
        assert_eq!(
            pair.construction,
            features::shared_construction(&configuration())
        );
        assert_eq!(pair.features.len(), 1);
        assert_eq!(pair.features[0].author, "left");
        assert_eq!(
            features::member_state(pair, "left", "mount-1"),
            Some(MechanicalFeatureState::Linked)
        );
        assert_eq!(
            features::member_state(pair, "right", "mount-1"),
            Some(MechanicalFeatureState::Linked)
        );
        assert_eq!(
            ownership(&doc, "left"),
            MechanicalConstructionOwnership::Pair
        );
        assert_eq!(
            ownership(&doc, "right"),
            MechanicalConstructionOwnership::Pair
        );
        // The pair owns the single authored value. Both halves read it - the author
        // its stored value and the other half the reflected counterpart - and neither
        // keeps a second writable copy.
        assert_point(
            visible_mount(&doc, "left", "mount-1"),
            Vec2 { x: 2.0, y: 3.0 },
        );
        assert_point(
            visible_mount(&doc, "right", "mount-1"),
            Vec2 { x: -2.0, y: 3.0 },
        );
        assert!(!stores_mount(&doc, "left", "mount-1"));
        assert!(!stores_mount(&doc, "right", "mount-1"));
    }

    #[test]
    fn linking_an_already_paired_instance_errors_and_mutates_nothing() {
        let mut doc = linked_document();
        let before = doc.clone();
        let error = apply(
            &mut doc,
            &[],
            &link_edit(board_target(), "pair-second", "left"),
        )
        .unwrap_err();
        assert!(
            error.contains("already belongs to a mechanical pair"),
            "{error}"
        );
        assert_eq!(doc, before);
    }

    #[test]
    fn unlinking_keeps_geometry_and_localizes_ownership() {
        let mut doc = linked_document();
        // A pair-shared construction edit, as a linked settings change produces.
        doc.hardware.as_mut().unwrap().pairs[0]
            .construction
            .plate_thickness = 2.5;
        let before_right = visible_mount(&doc, "right", "mount-1");
        let before_left = visible_mount(&doc, "left", "mount-1");
        let changed = apply(
            &mut doc,
            &[],
            &lifecycle_edit(MechanicalChange::UnlinkMirrorPair {
                instance_id: "right".into(),
            }),
        )
        .unwrap();
        assert_eq!(changed, vec!["pair-left-right", "left", "right"]);
        assert!(pairs(&doc).is_empty());
        assert_eq!(
            ownership(&doc, "left"),
            MechanicalConstructionOwnership::Local
        );
        assert_eq!(
            ownership(&doc, "right"),
            MechanicalConstructionOwnership::Local
        );
        // The pair construction is materialized, and neither mount moved.
        assert_eq!(stored_instance(&doc, "left").plate_thickness, 2.5);
        assert_eq!(stored_instance(&doc, "right").plate_thickness, 2.5);
        assert_point(
            mount_at(stored_instance(&doc, "right"), "mount-1"),
            before_right,
        );
        assert_point(
            mount_at(stored_instance(&doc, "left"), "mount-1"),
            before_left,
        );
    }

    #[test]
    fn make_local_keeps_visible_geometry_and_leaves_the_other_half_linked() {
        let mut doc = linked_document();
        let changed = apply(
            &mut doc,
            &[],
            &lifecycle_edit(MechanicalChange::MakeFeatureLocal {
                feature_id: "mount-1".into(),
                instance_id: "right".into(),
            }),
        )
        .unwrap();
        // The pair and both member instances are affected, not just the edited half.
        assert_eq!(changed, vec!["mount-1", "right", "pair-left-right", "left"]);
        // The reflected counterpart is materialized on the localized half.
        assert_point(
            mount_at(stored_instance(&doc, "right"), "mount-1"),
            Vec2 { x: -2.0, y: 3.0 },
        );
        let pair = &pairs(&doc)[0];
        assert_eq!(
            features::member_state(pair, "right", "mount-1"),
            Some(MechanicalFeatureState::Local)
        );
        assert_eq!(
            features::member_state(pair, "left", "mount-1"),
            Some(MechanicalFeatureState::Linked)
        );
        assert_eq!(pair.features[0].author, "left");
    }

    #[test]
    fn make_local_on_the_author_moves_authorship_to_the_linked_half() {
        let mut doc = linked_document();
        let changed = apply(
            &mut doc,
            &[],
            &lifecycle_edit(MechanicalChange::MakeFeatureLocal {
                feature_id: "mount-1".into(),
                instance_id: "left".into(),
            }),
        )
        .unwrap();
        assert_eq!(changed, vec!["mount-1", "left", "pair-left-right", "right"]);
        // The author keeps its own visible geometry locally.
        assert_point(
            mount_at(stored_instance(&doc, "left"), "mount-1"),
            Vec2 { x: 2.0, y: 3.0 },
        );
        let pair = &pairs(&doc)[0];
        assert_eq!(
            features::member_state(pair, "left", "mount-1"),
            Some(MechanicalFeatureState::Local)
        );
        assert_eq!(
            features::member_state(pair, "right", "mount-1"),
            Some(MechanicalFeatureState::Linked)
        );
        // Authorship moved, and the stored value is now in the other frame:
        // the reflection of the author's (2, 3) about X at the identity map.
        assert_eq!(pair.features[0].author, "right");
        let MechanicalFeatureValue::Mount { mount } = &pair.features[0].value else {
            panic!("a linked mount stores a mount");
        };
        assert_point(mount.at, Vec2 { x: -2.0, y: 3.0 });
        features::validate(&doc).unwrap();
        // The surviving linked half keeps the geometry it showed, derived from the
        // pair rather than stored on the half.
        assert_point(
            visible_mount(&doc, "right", "mount-1"),
            Vec2 { x: -2.0, y: 3.0 },
        );
        assert!(!stores_mount(&doc, "right", "mount-1"));
    }

    #[test]
    fn omit_removes_only_that_half_and_keeps_the_other_linked() {
        let mut doc = linked_document();
        let changed = apply(
            &mut doc,
            &[],
            &lifecycle_edit(MechanicalChange::OmitFeatureMember {
                feature_id: "mount-1".into(),
                instance_id: "right".into(),
            }),
        )
        .unwrap();
        // The pair and both member instances are affected, not just the edited half.
        assert_eq!(changed, vec!["mount-1", "right", "pair-left-right", "left"]);
        assert!(stored_instance(&doc, "right").mounts.is_empty());
        // The omitted half shows nothing; the still-linked author keeps its geometry.
        assert!(
            !effective_inputs(
                &doc,
                &[],
                &MechanicalTarget::Instance {
                    instance_id: "right".into(),
                },
            )
            .unwrap()
            .configuration
            .mounts
            .iter()
            .any(|mount| mount.id == "mount-1"),
            "an omitted half shows no linked geometry"
        );
        assert_point(
            visible_mount(&doc, "left", "mount-1"),
            Vec2 { x: 2.0, y: 3.0 },
        );
        let pair = &pairs(&doc)[0];
        assert_eq!(
            features::member_state(pair, "right", "mount-1"),
            Some(MechanicalFeatureState::Omitted)
        );
        assert_eq!(
            features::member_state(pair, "left", "mount-1"),
            Some(MechanicalFeatureState::Linked)
        );
    }

    #[test]
    fn omit_on_the_author_keeps_the_other_half_deriving_its_geometry() {
        let mut doc = linked_document();
        apply(
            &mut doc,
            &[],
            &lifecycle_edit(MechanicalChange::OmitFeatureMember {
                feature_id: "mount-1".into(),
                instance_id: "left".into(),
            }),
        )
        .unwrap();
        assert!(stored_instance(&doc, "left").mounts.is_empty());
        // The surviving linked half keeps deriving its geometry.
        assert_point(
            visible_mount(&doc, "right", "mount-1"),
            Vec2 { x: -2.0, y: 3.0 },
        );
        let pair = &pairs(&doc)[0];
        assert_eq!(
            features::member_state(pair, "left", "mount-1"),
            Some(MechanicalFeatureState::Omitted)
        );
        assert_eq!(
            features::member_state(pair, "right", "mount-1"),
            Some(MechanicalFeatureState::Linked)
        );
        assert_eq!(pair.features[0].author, "right");
        features::validate(&doc).unwrap();
    }

    #[test]
    fn rejoin_restores_the_link_and_derives_the_reflected_counterpart() {
        let mut doc = linked_document();
        apply(
            &mut doc,
            &[],
            &lifecycle_edit(MechanicalChange::MakeFeatureLocal {
                feature_id: "mount-1".into(),
                instance_id: "right".into(),
            }),
        )
        .unwrap();
        // The localized half has drifted to its own authored position.
        doc.hardware
            .as_mut()
            .unwrap()
            .instances
            .iter_mut()
            .find(|instance| instance.id == "right")
            .unwrap()
            .mechanical
            .as_mut()
            .unwrap()
            .mounts[0]
            .at = Vec2 { x: 100.0, y: 100.0 };

        let changed = apply(
            &mut doc,
            &[],
            &lifecycle_edit(MechanicalChange::RejoinFeatureMember {
                feature_id: "mount-1".into(),
                instance_id: "right".into(),
                source_instance_id: "left".into(),
            }),
        )
        .unwrap();
        assert_eq!(changed, vec!["mount-1", "right", "left", "pair-left-right"]);
        let pair = &pairs(&doc)[0];
        assert_eq!(pair.features[0].author, "left");
        let MechanicalFeatureValue::Mount { mount } = &pair.features[0].value else {
            panic!("a linked mount stores a mount");
        };
        assert_point(mount.at, Vec2 { x: 2.0, y: 3.0 });
        assert_eq!(
            features::member_state(pair, "right", "mount-1"),
            Some(MechanicalFeatureState::Linked)
        );
        // The reflected counterpart replaced the drifted local geometry, and the
        // rejoined half derives it from the pair instead of storing a copy.
        assert_point(
            visible_mount(&doc, "right", "mount-1"),
            Vec2 { x: -2.0, y: 3.0 },
        );
        assert!(!stores_mount(&doc, "right", "mount-1"));
    }

    #[test]
    fn a_no_op_lifecycle_action_returns_an_empty_id_list() {
        let mut doc = linked_document();
        let before = doc.clone();
        assert!(
            apply(
                &mut doc,
                &[],
                &lifecycle_edit(MechanicalChange::RejoinFeatureMember {
                    feature_id: "mount-1".into(),
                    instance_id: "right".into(),
                    source_instance_id: "left".into(),
                }),
            )
            .unwrap()
            .is_empty(),
            "an already linked half is unchanged"
        );
        assert_eq!(doc, before);
        doc.hardware.as_mut().unwrap().pairs[0].features[0].members[1].state =
            MechanicalFeatureState::Omitted;
        assert!(
            apply(
                &mut doc,
                &[],
                &lifecycle_edit(MechanicalChange::OmitFeatureMember {
                    feature_id: "mount-1".into(),
                    instance_id: "right".into(),
                }),
            )
            .unwrap()
            .is_empty(),
            "an already omitted half is unchanged"
        );
    }

    #[test]
    fn unknown_pair_feature_and_instance_ids_error_and_mutate_nothing() {
        let mut doc = linked_document();
        let before = doc.clone();
        for change in [
            MechanicalChange::UnlinkMirrorPair {
                instance_id: "ghost".into(),
            },
            MechanicalChange::MakeFeatureLocal {
                feature_id: "ghost".into(),
                instance_id: "right".into(),
            },
            MechanicalChange::OmitFeatureMember {
                feature_id: "mount-1".into(),
                instance_id: "ghost".into(),
            },
            MechanicalChange::RejoinFeatureMember {
                feature_id: "mount-1".into(),
                instance_id: "ghost".into(),
                source_instance_id: "left".into(),
            },
        ] {
            assert!(
                apply(&mut doc, &[], &lifecycle_edit(change.clone())).is_err(),
                "{change:?} must error"
            );
            assert_eq!(doc, before, "{change:?} must mutate nothing");
        }
        let mut missing_template = link_edit(board_target(), "pair-left-right", "left");
        missing_template.mounting_hole = None;
        assert!(
            apply(&mut doc, &[], &missing_template).is_err(),
            "the projection template is required"
        );
        assert_eq!(doc, before);
    }

    #[test]
    fn an_end_to_end_link_commits_and_one_undo_restores_the_document() {
        use crate::CoreEngine;
        use crate::model::{CoreReply, CoreRequest, EditCommand, EditOperation, EditPhase};
        let mut engine = CoreEngine::new();
        let before = mirror_document();
        let CoreReply::Scene { document, .. } = engine.handle(CoreRequest::Open {
            id: "open".into(),
            document: before.clone(),
        }) else {
            panic!("open must succeed");
        };
        let revision = document.revision;
        let CoreReply::Scene { document, .. } = engine.handle(CoreRequest::Edit {
            id: "link".into(),
            command: EditCommand {
                base_revision: revision,
                transaction_id: "link".into(),
                phase: EditPhase::Commit,
                target_ids: vec![],
                operation: EditOperation::ApplyMechanicalEdit {
                    edit: Box::new(link_edit(board_target(), "pair-left-right", "left")),
                },
            },
        }) else {
            panic!("the link must commit");
        };
        assert_eq!(document.hardware.as_ref().unwrap().pairs.len(), 1);
        let CoreReply::Scene { document, .. } =
            engine.handle(CoreRequest::Undo { id: "undo".into() })
        else {
            panic!("undo must succeed");
        };
        assert_eq!(document.hardware, before.hardware);
        assert_eq!(document.mechanical, before.mechanical);
    }
}

#[cfg(test)]
mod identity_merge_tests {
    use super::*;
    use serde_json::json;

    fn configuration() -> MechanicalConfiguration {
        serde_json::from_value(json!({
            "boardId":"board","method":"printed","mount":"rigid",
            "plateThickness":1.5,"plateFoamThickness":0.5,"pcbThickness":1.6,
            "bottomFoamThickness":0.4,"batteryHeight":4.0,"bottomThickness":3.0,
            "plateToPcb":3.5,"wallThickness":2.0,"clearance":0.3,
            "mounts":[],"closureMounts":[],"profiles":[]
        }))
        .unwrap()
    }

    #[test]
    fn reservation_merging_refuses_an_exhausted_floor() {
        for (label, floor) in [
            (
                "mount",
                IdentityFloor {
                    mount: u64::MAX,
                    ..IdentityFloor::default()
                },
            ),
            (
                "vertex",
                IdentityFloor {
                    vertex: u64::MAX,
                    ..IdentityFloor::default()
                },
            ),
            (
                "critical fit",
                IdentityFloor {
                    critical_fit: u64::MAX,
                    ..IdentityFloor::default()
                },
            ),
            (
                "hardware",
                IdentityFloor {
                    hardware: u64::MAX,
                    ..IdentityFloor::default()
                },
            ),
        ] {
            let mut configuration = configuration();
            if label == "vertex" {
                configuration.openings = Some(vec![MechanicalOpening {
                    id: "opening-1".into(),
                    vertices: vec![],
                    z: 0.0,
                    height: 1.0,
                    next_vertex_identity: 1,
                }]);
            }
            let error = merge_identity_floor(&mut configuration, floor).unwrap_err();
            assert!(error.contains("exhausted"), "{label}: {error}");
        }
    }

    #[test]
    fn merging_an_already_exhausted_counter_is_a_no_op() {
        let mut configuration = configuration();
        configuration.next_mount_identity = u64::MAX;
        merge_identity_floor(
            &mut configuration,
            IdentityFloor {
                mount: u64::MAX,
                ..IdentityFloor::default()
            },
        )
        .unwrap();
        assert_eq!(configuration.next_mount_identity, u64::MAX);
    }
}
