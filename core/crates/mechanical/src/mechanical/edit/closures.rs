//! Settings staging, closure-mount initialization and generated closure projection.

use super::super::change::{MechanicalChange, MechanicalEdit, MechanicalTarget};
use super::super::features;
use super::super::inputs::{effective_document, effective_inputs};
use super::{
    AUTO_CLOSURE_PREFIX, GENERATED_DEFINITION_PREFIX, GENERATED_PART_PREFIX,
    MechanicalIdentityLedger, apply_change, instance_configuration_mut, pair_index, pair_owner_ids,
    persist, persist_edit, requires_projection, reserved_identity_floor,
};
use crate::model::*;

pub(super) fn apply_settings(
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
pub(super) fn project(
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
pub(super) fn validate_mounting_hole(template: &PartDefinition) -> Result<(), String> {
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
pub(super) fn initialize_closure_mounts(
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
pub(super) fn initialize_target_closures(
    doc: &mut ProjectDoc,
    board_contours: &[BoardContours],
    target: &MechanicalTarget,
    configuration: &MechanicalConfiguration,
    changed: &mut Vec<String>,
) -> Result<Vec<Mount>, String> {
    if let Some(pair_index) = paired_index(doc, target) {
        if link_pair_closure_proposals(doc, board_contours, pair_index)? {
            changed.extend(pair_owner_ids(doc, pair_index)?);
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
pub(super) fn link_pair_closure_proposals(
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
pub(super) fn paired_index(doc: &ProjectDoc, target: &MechanicalTarget) -> Option<usize> {
    match target {
        MechanicalTarget::Instance { instance_id } => pair_index(doc, instance_id),
        MechanicalTarget::Board { .. } => None,
    }
}

/// Whether an instance's effective stack has no closure list and is not a gasket, so its
/// closure mounts still need initialization.
pub(super) fn closure_list_absent(doc: &ProjectDoc, target: &MechanicalTarget) -> bool {
    effective_inputs(doc, &[], target).is_ok_and(|inputs| {
        inputs.configuration.closure_mounts.is_none()
            && inputs.configuration.mount != MechanicalMount::Gasket
    })
}

/// The accepted contours of the edit target's board, mirrored for a flipped half.
///
/// `None` when the accepted projection has no entry for that board, so initialization
/// falls back to the contours recomputed from the proposed document.
pub(super) fn accepted_target_contours(
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
        super::super::inputs::mirror_contours(declared)
    } else {
        declared
    })
}

/// The generated part ids, generated definition ids and membership owners that
/// differ between two documents.
pub(super) fn generated_change_ids(before: &ProjectDoc, after: &ProjectDoc) -> Vec<String> {
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

pub(super) fn dedup_changed(changed: &mut Vec<String>) {
    let mut seen = std::collections::BTreeSet::new();
    changed.retain(|id| seen.insert(id.clone()));
}

/// Resolves an edit target to its board id and optional physical instance.
pub fn resolve_target<'a>(
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

pub(super) fn target_board_id(
    doc: &ProjectDoc,
    target: &MechanicalTarget,
) -> Result<String, String> {
    resolve_target(doc, target).map(|(board_id, _)| board_id)
}

pub(super) fn owner_id(target: &MechanicalTarget, board_id: &str) -> String {
    match target {
        MechanicalTarget::Board { .. } => board_id.to_string(),
        MechanicalTarget::Instance { instance_id } => instance_id.clone(),
    }
}
