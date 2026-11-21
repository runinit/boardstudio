//! Hosted embedded-circuit staging and hosted baseline validation.

use super::{HostedTransition, Projection};
use crate::model::{MirroredCorrespondence, MirroredHalves, MirroredItemKind, ProjectDoc};
use std::collections::{BTreeMap, BTreeSet};

pub(super) fn stage_hosted_correspondence(
    document: &mut ProjectDoc,
    pair_id: &str,
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

/// Linked items receive projected values. References also resolve Local items, while
/// Omitted items release references and electrical assignments.
#[derive(Default)]
pub(super) struct HostedTargets {
    pub(super) linked: BTreeMap<String, String>,
    pub(super) references: BTreeMap<String, String>,
    pub(super) omitted: BTreeSet<String>,
}

impl HostedTargets {
    pub(super) fn derive(
        document: &ProjectDoc,
        halves: &MirroredHalves,
        circuits: &[&MirroredCorrespondence],
        projection: &Projection,
    ) -> Result<(Self, Self), String> {
        let (mut parts, mut nets) = (Self::default(), Self::default());
        for link in &halves.correspondences {
            let targets = match link.kind {
                MirroredItemKind::Part => &mut parts,
                MirroredItemKind::Net => &mut nets,
                _ => continue,
            };
            let state = super::effective_circuit_child_state(
                document,
                halves,
                link,
                circuits.iter().copied(),
            );
            if state.is_none() {
                targets
                    .linked
                    .insert(link.source_id.clone(), link.mirror_id.clone());
            }
            if state == Some(crate::model::MirroredExceptionState::Omitted) {
                targets.omitted.insert(link.source_id.clone());
            } else {
                targets
                    .references
                    .insert(link.source_id.clone(), link.mirror_id.clone());
            }
        }
        for transition in &projection.hosted_transitions {
            let HostedTransition::AddCircuit {
                member_ids,
                net_ids,
                ..
            } = transition
            else {
                continue;
            };
            for (kind, targets, additions) in [
                (MirroredItemKind::Part, &mut parts, member_ids),
                (MirroredItemKind::Net, &mut nets, net_ids),
            ] {
                for (source, mirror) in additions {
                    if targets
                        .references
                        .iter()
                        .any(|(s, m)| s != source && m == mirror)
                    {
                        return Err(format!(
                            "The selected hosted {kind:?} reference target {mirror:?} is already in use"
                        ));
                    }
                    if targets
                        .references
                        .insert(source.clone(), mirror.clone())
                        .is_some_and(|old| old != *mirror)
                    {
                        return Err(format!(
                            "The selected hosted {kind:?} reference conflicts with an existing identity"
                        ));
                    }
                }
            }
        }
        Ok((parts, nets))
    }
}

/// Stages every hosted AddCircuit transition onto the mirror draft with its Add provenance.
pub(super) fn stage_hosted_circuit_additions(
    document: &ProjectDoc,
    pair: &crate::model::MechanicalMirrorPair,
    halves: &MirroredHalves,
    projection: &Projection,
    staged: &mut ProjectDoc,
) -> Result<(), String> {
    for transition in &projection.hosted_transitions {
        let HostedTransition::AddCircuit {
            source_id,
            mirror_id,
            member_ids,
            member_references,
            definition_ids,
            net_ids,
            ..
        } = transition
        else {
            continue;
        };
        let source = document
            .embedded_circuits
            .iter()
            .find(|circuit| circuit.id == *source_id)
            .ok_or_else(|| format!("The source embedded circuit {source_id:?} is unavailable"))?;
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
            if existing.is_some() {
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
            let target_part = crate::mirrored_half::derive::part(
                source_part,
                target_member_id,
                target_definition,
                target_reference,
                pair.mapping,
                crate::mirrored_half::derive::is_controller(
                    &document.definitions,
                    &source_part.definition_id,
                ),
            );
            staged.parts.push(target_part);
            let board = staged
                .boards
                .iter_mut()
                .find(|board| board.id == halves.mirror_board_id)
                .ok_or_else(|| "The target circuit board is unavailable".to_owned())?;
            if !board.part_ids.contains(target_member_id) {
                board.part_ids.push(target_member_id.clone());
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
            if staged
                .definitions
                .iter()
                .any(|existing| existing.id == *target_definition_id)
            {
                return Err(format!(
                    "The selected target circuit definition {target_definition_id:?} already exists"
                ));
            }
            staged.definitions.push(definition);
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
                if !is_joined_host_net {
                    return Err(format!(
                        "The selected target circuit net {target_net_id:?} already exists without private Add provenance"
                    ));
                }
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
        if staged
            .embedded_circuits
            .iter()
            .any(|existing| existing.id == *mirror_id)
        {
            return Err(format!(
                "The selected target circuit {mirror_id:?} already exists"
            ));
        }
        staged.embedded_circuits.push(circuit);
    }
    Ok(())
}
