//! Forward reconciliation of the mirror half from a validated projection.

use super::changes::changed_document_ids;
use super::hosted::HostedOmissionClosure;
use super::{
    HostedCircuitAddProvenance, HostedTransition, IndependentPartTransition, MatrixStage,
    Projection, circuit_links, outlines, project_part, settings, stage_hosted_circuit_additions,
    stage_hosted_correspondence, stage_matrix, unique_board_owner,
    validate_hosted_omission_baselines, validate_hosted_removal_baselines,
    validate_local_circuit_targets,
};
use crate::model::{
    MirroredCorrespondence, MirroredHalves, MirroredItemKind, OutlineFeature, Part, Pose2,
    ProjectDoc, Vec2,
};
use std::collections::{BTreeMap, BTreeSet};

mod independent;
mod targets;
use independent::{IndependentStage, stage_independent_transitions};
use targets::{CorrespondenceTargets, correspondence_targets};

#[doc(hidden)]
pub fn reconcile(
    document: &mut ProjectDoc,
    pair_id: &str,
    projection: &Projection,
) -> Result<Vec<String>, String> {
    reconcile_projection(document, pair_id, projection, true)
}

pub(super) fn reconcile_canonical(
    document: &mut ProjectDoc,
    pair_id: &str,
    projection: &Projection,
) -> Result<(), String> {
    reconcile_projection(document, pair_id, projection, false).map(|_| ())
}

// The explicit entry point checks caller-supplied membership evidence and returns a report.
// Prepared drafts derive the evidence from their authenticated pair and report at transaction end.
fn reconcile_projection(
    document: &mut ProjectDoc,
    pair_id: &str,
    projection: &Projection,
    explicit: bool,
) -> Result<Vec<String>, String> {
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
    } = stage_matrix(document, halves, projection, pair.mapping, explicit)?;
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
    let circuit_links: Vec<_> = circuit_links(halves).collect();
    let hosted_circuit_members: BTreeSet<String> = circuit_links
        .iter()
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
            super::super::derive::is_controller(&document.definitions, &source.definition_id),
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

    let removed_independent_targets = stage_independent_transitions(
        IndependentStage {
            document,
            halves,
            projection,
            mapping: pair.mapping,
            source_matrix_members: &source_matrix_members,
            mirror_matrix_members: &mirror_matrix_members,
        },
        &mut staged,
        &mut independent_part_targets,
    )?;

    let CorrespondenceTargets {
        part_targets,
        reference_part_targets,
        omitted_part_sources,
        net_targets,
        reference_net_targets,
    } = correspondence_targets(document, halves, &circuit_links, projection)?;
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
    let circuit_pairs: Vec<_> = circuit_links
        .into_iter()
        .filter(|association| {
            !replayed_circuit_additions
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
                super::super::derive::is_controller(
                    &document.definitions,
                    &source_part.definition_id,
                ),
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

    stage_hosted_circuit_additions(
        document,
        pair,
        halves,
        projection,
        &mut staged,
        &mut planned_hosted_operations,
    )?;

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
    let module_context = super::super::ModuleMirror {
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
                let projected = super::super::mirror_module(source, &module_context)?;
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
                    .push(super::super::mirror_module(source, &module_context)?),
                (Some(target), true) => {
                    *target = super::super::mirror_module(source, &module_context)?
                }
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
    outlines::reconcile_draft(&mut staged, pair_id, projection)?;
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

    validate_projection_draft(
        &mut staged,
        pair_id,
        projection,
        &planned_hosted_operations,
        &hosted_omission_closure,
    )?;

    let changed_ids = if explicit {
        changed_document_ids(document, &staged, &[])?
    } else {
        Vec::new()
    };
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
                first_output: document.clone(),
            });
    }
    Ok(changed_ids)
}

fn validate_projection_draft(
    staged: &mut ProjectDoc,
    pair_id: &str,
    projection: &Projection,
    planned_hosted_operations: &BTreeSet<(String, String, String)>,
    hosted_omission_closure: &HostedOmissionClosure,
) -> Result<(), String> {
    fn validate(document: &ProjectDoc) -> Result<(), String> {
        crate::mechanical::features::validate(document)?;
        crate::mechanical::halves::validate_document(document)
            .map_err(|error| format!("The validated hosted draft is inconsistent: {error}"))
    }

    // Removal evidence can prune pair records for validation before the lifecycle
    // layer retires them. Ordinary projection validates the staged draft directly.
    if planned_hosted_operations.is_empty()
        && hosted_omission_closure.parts.is_empty()
        && hosted_omission_closure.nets.is_empty()
        && projection.outline_removal_baselines.borrow().is_empty()
    {
        return validate(staged);
    }

    let pair = staged
        .hardware
        .as_mut()
        .and_then(|hardware| hardware.pairs.iter_mut().find(|pair| pair.id == pair_id))
        .ok_or_else(|| format!("The validated pair {pair_id:?} is unavailable"))?;
    let validated_halves = pair
        .halves
        .as_mut()
        .ok_or_else(|| format!("The validated pair {pair_id:?} has no halves context"))?;
    let original_halves = validated_halves.clone();
    for (operation, source_id, mirror_id) in planned_hosted_operations {
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
    let result =
        outlines::prepare_validation(staged, pair_id, projection).and_then(|()| validate(staged));
    // Validation only prunes this pair's halves. Restore the durable rows before
    // reporting changes or publishing the draft, including when validation refuses.
    staged
        .hardware
        .as_mut()
        .and_then(|hardware| hardware.pairs.iter_mut().find(|pair| pair.id == pair_id))
        .expect("projection validation only modifies the pair's halves")
        .halves = Some(original_halves);
    result
}
