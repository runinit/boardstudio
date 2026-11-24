//! Edit persistence and mirror-pair ownership: enable, disable, link, unlink and topology repair.

use super::super::change::{MechanicalChange, MechanicalEdit, MechanicalTarget};
use super::super::features;
use super::super::inputs::effective_inputs;
use super::paired::{persist_paired_feature_edit, project_changed, validate_supplied_template};
use super::{
    closure_list_absent, default_configuration, find_feature_value, identity_reservation_of,
    initialize_target_closures, instance_configuration_mut, merge_identity_floor, owner_id,
    pair_at, paired_index, remove_feature, reserved_identity_floor,
};
use crate::model::*;

/// Writes the effective configuration back to the target's stored slot.
///
/// Canonical edits write `doc.mechanical`; instance edits write that instance's
/// `mechanical`. The stored value is the effective configuration the patch
/// produced, matching the adapter's `prepare_document` + `persist_configuration`
/// sequence.
pub(super) fn persist(
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
/// Pair-owned features and shared construction route through
/// [`persist_paired_feature_edit`]; a `Legacy` member's overlay change is written
/// to the shared source so the next effective read returns it, and every other
/// field stays local.
pub(super) fn persist_edit(
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
                persist_paired_feature_edit(
                    doc,
                    instance_id,
                    pair_index,
                    before,
                    &after,
                    &mut local,
                    changed,
                )?;
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
pub(super) fn enable(
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
    validate_supplied_template(edit.mounting_hole.as_deref())?;
    // The fresh configuration carries the Core defaults (battery, stabilizers and the
    // rest), exactly as the effective read would resolve it, so enabling and the
    // initialization that follows persist the same configuration the adapter shows.
    let mut configuration = super::super::inputs::apply_mechanical_defaults(
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
    project_changed(doc, edit.mounting_hole.as_deref(), &mut changed)?;
    Ok(changed)
}

/// Clears the target's stored configuration; absent configuration is a no-op.
///
/// Disabling can remove generated closure geometry, so it is projection-required
/// and must remove the stale generated clearance parts in the same edit.
pub(super) fn disable(
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
    validate_supplied_template(edit.mounting_hole.as_deref())?;
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
    project_changed(doc, edit.mounting_hole.as_deref(), &mut changed)?;
    Ok(changed)
}

/// Initializes absent closure mounts from the resolved assembly and projects them.
///
/// An already-present collection or a gasket mount is not eligible and reports the
/// adapter's message; explicit `Some(vec![])` therefore stays deliberate.
pub(super) fn initialize_closures(
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
    validate_supplied_template(edit.mounting_hole.as_deref())?;
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
    project_changed(doc, edit.mounting_hole.as_deref(), &mut changed)?;
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
pub(super) fn link_mirror_pair(
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
    project_changed(doc, edit.mounting_hole.as_deref(), &mut changed)?;
    Ok(changed)
}

/// Removes one physical half from its pair, preserving the survivor's visible model.
///
/// The target must name the removed half. The survivor's accepted effective model -
/// its pair-derived geometry, shared construction and hardware-specific values - is
/// materialized into its own configuration first, so nothing it showed is lost, and
/// then the pair is cleared and the member removed with its references. Generated
/// closure holes and memberships refresh in the same atomic edit.
pub(super) fn remove_physical_instance(
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
    validate_supplied_template(edit.mounting_hole.as_deref())?;
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
    let pair_id = pair_at(doc, pair_index)?.id.clone();
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
    project_changed(doc, edit.mounting_hole.as_deref(), &mut changed)?;
    Ok(changed)
}

/// The pair that owns `instance_id`, as its index and the other member's id.
pub(super) fn pair_membership(
    doc: &ProjectDoc,
    instance_id: &str,
) -> Result<(usize, String), String> {
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
pub fn reconcile_physical_topology(
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
        let pair = pair_at(doc, index)?.clone();
        doc.hardware
            .as_mut()
            .ok_or("Physical instances are unavailable")?
            .pairs
            .remove(index);
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
pub(super) fn materialize_survivor(
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
pub(super) fn unlink_mirror_pair(
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
    project_changed(doc, edit.mounting_hole.as_deref(), &mut changed)?;
    Ok(changed)
}

/// The index of the mechanical pair that owns `instance_id`, if any.
pub(super) fn pair_index(doc: &ProjectDoc, instance_id: &str) -> Option<usize> {
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
pub(super) fn dissolve_pair_ownership(
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
