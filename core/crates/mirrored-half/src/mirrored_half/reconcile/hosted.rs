//! Hosted embedded-circuit staging and hosted baseline validation.

use super::{HostedTransition, Projection, project_part};
use crate::model::{
    MirroredCorrespondence, MirroredHalves, MirroredItemKind, Part, Pose2, ProjectDoc, Vec2,
};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, PartialEq)]
#[doc(hidden)]
pub struct HostedCircuitAddProvenance {
    pub(super) member_ids: BTreeMap<String, String>,
    pub(super) member_references: BTreeMap<String, String>,
    pub(super) definition_ids: BTreeMap<String, String>,
    pub(super) net_ids: BTreeMap<String, String>,
    pub(super) member_poses: BTreeMap<String, Pose2>,
    pub(super) first_output: ProjectDoc,
}

#[derive(Default)]
pub(super) struct HostedOmissionClosure {
    pub(super) parts: BTreeSet<String>,
    pub(super) nets: BTreeSet<String>,
}

#[doc(hidden)]
pub fn stage_hosted_correspondence(
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

/// Stages every hosted AddCircuit transition onto the mirror draft with its Add provenance.
pub(super) fn stage_hosted_circuit_additions(
    document: &ProjectDoc,
    pair: &crate::model::MechanicalMirrorPair,
    halves: &MirroredHalves,
    projection: &Projection,
    staged: &mut ProjectDoc,
    planned_hosted_operations: &mut BTreeSet<(String, String, String)>,
) -> Result<(), String> {
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
                    super::super::derive::is_controller(
                        &document.definitions,
                        &source_part.definition_id,
                    ),
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
                    anchor: None,
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
                    super::super::derive::is_controller(
                        &document.definitions,
                        &source_part.definition_id,
                    ),
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
    Ok(())
}

pub(super) fn validate_hosted_removal_baselines(
    document: &ProjectDoc,
    pair_id: &str,
    projection: &Projection,
) -> Result<(), String> {
    fn pair_bytes(document: &ProjectDoc, pair_id: &str) -> Result<Vec<u8>, String> {
        let hardware = document
            .hardware
            .as_ref()
            .ok_or_else(|| "The hosted removal hardware configuration is unavailable".to_owned())?;
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

pub(super) fn validate_hosted_omission_baselines(
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
