use super::*;
use std::collections::BTreeSet;

fn allocate_reference(source: &str, used: &mut BTreeSet<String>) -> String {
    if !source.is_empty() && used.insert(source.to_owned()) {
        return source.to_owned();
    }
    let prefix = source.trim_end_matches(|c: char| c.is_ascii_digit());
    let prefix = if prefix.is_empty() { "U" } else { prefix };
    for index in 1.. {
        let candidate = format!("{prefix}{index}");
        if used.insert(candidate.clone()) {
            return candidate;
        }
    }
    unreachable!("A finite circuit cannot exhaust component references")
}

pub(crate) fn embed(
    doc: &mut ProjectDoc,
    id: &str,
    definition: &ModuleDefinition,
    board_id: &str,
    pose: Pose2,
    side: Side,
    joins: &BTreeMap<String, String>,
) -> Result<Vec<String>, String> {
    placement::validate_definition(definition)?;
    if id.trim().is_empty()
        || id.len() > 128
        || !pose.at.x.is_finite()
        || !pose.at.y.is_finite()
        || !pose.rotation.is_finite()
    {
        return Err("Circuit placement requires an identity and finite coordinates".into());
    }
    let board = doc
        .boards
        .iter()
        .find(|b| b.id == board_id)
        .ok_or("Circuit host PCB is missing")?;
    let circuit = definition
        .circuit
        .as_ref()
        .ok_or("No editable circuit source is available for this module")?;
    if doc.embedded_circuits.iter().any(|c| c.id == id)
        || circuit.parts.is_empty()
        || circuit.parts.len() > 2048
    {
        return Err("Choose a new circuit identity and a source with editable members".into());
    }
    let prefix = format!("embedded/{id}/");
    if doc.parts.iter().any(|p| p.id.starts_with(&prefix))
        || doc.definitions.iter().any(|d| d.id.starts_with(&prefix))
        || doc.nets.iter().any(|n| n.id.starts_with(&prefix))
    {
        return Err("The circuit namespace is already in use".into());
    }
    for (port, net) in joins {
        if !circuit.ports.contains_key(port)
            || !board.net_ids.contains(net)
            || !doc.nets.iter().any(|n| &n.id == net)
        {
            return Err(format!(
                "Circuit port {port} must join an existing net on its host PCB"
            ));
        }
    }
    let mut joined = BTreeMap::new();
    for (port, net) in joins {
        let local = &circuit.ports[port];
        if joined
            .insert(local.clone(), net.clone())
            .is_some_and(|old| old != *net)
        {
            return Err(
                "Two names for the same circuit contact cannot join different host nets".into(),
            );
        }
    }
    let net_ids: BTreeMap<_, _> = circuit
        .nets
        .iter()
        .map(|n| {
            (
                n.id.clone(),
                joined
                    .get(&n.id)
                    .cloned()
                    .unwrap_or_else(|| format!("{prefix}{}", n.id)),
            )
        })
        .collect();
    let part_ids: BTreeMap<_, _> = circuit
        .parts
        .iter()
        .map(|p| (p.id.clone(), format!("{prefix}{}", p.id)))
        .collect();
    let definitions: BTreeMap<_, _> = circuit
        .definitions
        .iter()
        .map(|d| (d.id.clone(), format!("{prefix}definition/{}", d.id)))
        .collect();
    if part_ids.len() != circuit.parts.len()
        || definitions.len() != circuit.definitions.len()
        || net_ids.len() != circuit.nets.len()
    {
        return Err("Circuit source contains duplicate identities".into());
    }
    if circuit
        .ports
        .iter()
        .any(|(role, net)| role.trim().is_empty() || !net_ids.contains_key(net))
    {
        return Err("Circuit ports must refer to named source nets".into());
    }
    for signal in &definition.electrical.required_signals {
        let role = serde_json::to_value(signal)
            .ok()
            .and_then(|value| value.as_str().map(str::to_owned))
            .ok_or("Circuit signal role is invalid")?;
        if !circuit.ports.contains_key(&role) {
            return Err(format!(
                "Required electrical port {role} is missing from the circuit source"
            ));
        }
    }
    for part in &circuit.parts {
        if !definitions.contains_key(&part.definition_id) {
            return Err("Circuit member definition is missing".into());
        }
    }
    for net in &circuit.nets {
        for pin in &net.pins {
            let part = circuit
                .parts
                .iter()
                .find(|p| p.id == pin.part_id)
                .ok_or("Circuit net refers to a missing member")?;
            if !circuit
                .definitions
                .iter()
                .any(|d| d.id == part.definition_id && d.pads.iter().any(|p| p.id == pin.pad_id))
            {
                return Err("Circuit net refers to a missing physical pad".into());
            }
        }
    }
    let mut references = doc
        .parts
        .iter()
        .filter(|part| board.part_ids.contains(&part.id))
        .map(|part| part.reference.clone())
        .collect();
    let (sin, cos) = pose.rotation.to_radians().sin_cos();
    for def in &circuit.definitions {
        let mut def = def.clone();
        def.id = definitions[&def.id].clone();
        if let Some(profile) = &mut def.mechanical_profile {
            profile.definition_id = def.id.clone();
        }
        doc.definitions.push(def);
    }
    for member in &circuit.parts {
        let mut part = member.clone();
        part.id = part_ids[&member.id].clone();
        part.definition_id = definitions[&member.definition_id].clone();
        part.reference = allocate_reference(&member.reference, &mut references);
        part.properties.get_or_insert_with(BTreeMap::new).insert(
            "moduleSourceReference".into(),
            serde_json::Value::String(member.reference.clone()),
        );
        let x = if side == Side::Back {
            -member.pose.at.x
        } else {
            member.pose.at.x
        };
        part.pose = Pose2 {
            at: Vec2 {
                x: pose.at.x + x * cos - member.pose.at.y * sin,
                y: pose.at.y + x * sin + member.pose.at.y * cos,
            },
            rotation: pose.rotation
                + if side == Side::Back {
                    -member.pose.rotation
                } else {
                    member.pose.rotation
                },
        };
        if side == Side::Back {
            part.side = if part.side == Side::Back {
                Side::Front
            } else {
                Side::Back
            };
        }
        doc.parts.push(part);
    }
    for local in &circuit.nets {
        let pins: Vec<_> = local
            .pins
            .iter()
            .map(|p| Pin {
                part_id: part_ids[&p.part_id].clone(),
                pad_id: p.pad_id.clone(),
            })
            .collect();
        let net_id = &net_ids[&local.id];
        if let Some(net) = doc.nets.iter_mut().find(|n| n.id == *net_id) {
            net.pins.extend(pins);
        } else {
            doc.nets.push(Net {
                id: net_id.clone(),
                name: local.name.clone(),
                pins,
            });
        }
    }
    let board = doc.boards.iter_mut().find(|b| b.id == board_id).unwrap();
    board.part_ids.extend(part_ids.values().cloned());
    for net in net_ids.values() {
        if !board.net_ids.contains(net) {
            board.net_ids.push(net.clone());
        }
    }
    if !doc.module_definitions.iter().any(|d| d.id == definition.id) {
        doc.module_definitions.push(definition.clone());
    }
    doc.embedded_circuits.push(EmbeddedCircuit {
        id: id.into(),
        definition_id: definition.id.clone(),
        host_board_id: board_id.into(),
        part_ids: part_ids.into_values().collect(),
        net_ids: net_ids.values().cloned().collect(),
        ports: circuit
            .ports
            .iter()
            .filter_map(|(role, local)| net_ids.get(local).map(|id| (role.clone(), id.clone())))
            .collect(),
    });
    Ok(vec![id.into(), board_id.into()])
}

pub(crate) fn remove_circuit(doc: &mut ProjectDoc, id: &str) -> Result<Vec<String>, String> {
    let index = doc
        .embedded_circuits
        .iter()
        .position(|c| c.id == id)
        .ok_or("Embedded circuit is missing")?;
    let circuit = doc.embedded_circuits.remove(index);
    if doc
        .parts
        .iter()
        .any(|p| circuit.part_ids.contains(&p.id) && p.locked == Some(true))
    {
        return Err("Unlock circuit members before removing the circuit".into());
    }
    doc.parts.retain(|p| !circuit.part_ids.contains(&p.id));
    for net in &mut doc.nets {
        net.pins.retain(|p| !circuit.part_ids.contains(&p.part_id));
    }
    let private_net_prefix = format!("embedded/{id}/");
    let removed: BTreeSet<_> = doc
        .nets
        .iter()
        .filter(|n| {
            n.id.starts_with(&private_net_prefix)
                && circuit.net_ids.contains(&n.id)
                && n.pins.is_empty()
        })
        .map(|n| n.id.clone())
        .collect();
    doc.nets.retain(|n| !removed.contains(&n.id));
    for board in &mut doc.boards {
        board.part_ids.retain(|p| !circuit.part_ids.contains(p));
        board.net_ids.retain(|n| !removed.contains(n));
        board
            .traces
            .retain(|t| !t.net_id.as_ref().is_some_and(|n| removed.contains(n)));
        board
            .vias
            .retain(|v| !v.net_id.as_ref().is_some_and(|n| removed.contains(n)));
    }
    let prefix = format!("embedded/{id}/definition/");
    doc.definitions.retain(|d| {
        !d.id.starts_with(&prefix) || doc.parts.iter().any(|p| p.definition_id == d.id)
    });
    Ok(vec![id.into(), circuit.host_board_id])
}

fn circuit_error(scope: Scope, id: String, message: &str, target_ids: Vec<String>) -> Finding {
    Finding {
        scope,
        id,
        message: message.into(),
        target_ids,
        severity: Severity::Error,
    }
}

pub(crate) fn embedded_findings(doc: &ProjectDoc, board_id: &str) -> Vec<Finding> {
    let mut findings = Vec::new();
    for circuit in doc
        .embedded_circuits
        .iter()
        .filter(|circuit| circuit.host_board_id == board_id)
    {
        let Some(definition) = doc
            .module_definitions
            .iter()
            .find(|definition| definition.id == circuit.definition_id)
        else {
            findings.push(circuit_error(
                Scope::Pcb,
                format!("embedded/{}/definition", circuit.id),
                "Embedded circuit source snapshot is missing",
                vec![circuit.id.clone()],
            ));
            continue;
        };
        for gate in definition.gates.iter().filter(|gate| {
            matches!(
                gate.output,
                HardwareOutput::Footprint | HardwareOutput::Electrical | HardwareOutput::Firmware
            )
        }) {
            let mut targets = vec![circuit.id.clone()];
            if gate.code == "pullup-supply" {
                targets.extend(
                    circuit
                        .part_ids
                        .iter()
                        .filter(|id| {
                            ["/R1", "/R2", "/JP1"]
                                .iter()
                                .any(|suffix| id.ends_with(suffix))
                        })
                        .cloned(),
                );
            }
            let (scope, severity) = if gate.output == HardwareOutput::Firmware {
                (Scope::Layout, Severity::Warning)
            } else {
                (Scope::Pcb, Severity::Error)
            };
            findings.push(Finding {
                id: format!("embedded/{}/{}", circuit.id, gate.code),
                scope,
                severity,
                message: gate.message.clone(),
                target_ids: targets,
            });
        }
        let unqualified: Vec<_> = circuit
            .part_ids
            .iter()
            .filter(|id| {
                doc.parts
                    .iter()
                    .find(|part| &part.id == *id)
                    .and_then(|part| {
                        doc.definitions
                            .iter()
                            .find(|definition| definition.id == part.definition_id)
                    })
                    .is_some_and(|definition| {
                        definition.pads.iter().any(|pad| !pad.number.is_empty())
                            && definition
                                .mechanical_profile
                                .as_ref()
                                .is_none_or(|profile| {
                                    profile
                                        .clearance_volumes
                                        .as_ref()
                                        .is_none_or(|volumes| volumes.is_empty())
                                })
                    })
            })
            .cloned()
            .collect();
        if !unqualified.is_empty() {
            let mut targets = vec![circuit.id.clone()];
            targets.extend(unqualified);
            findings.push(circuit_error(Scope::Case,format!("embedded/{}/component-envelopes",circuit.id),"Copied components need measured body heights, mounting and service clearances before exact case output",targets));
        }
    }
    findings
}
