//! Private physical-setup proposal rules. This code prepares values only; it never submits edits.

use boardstudio_application::TerminalOutcome;
use boardstudio_core::model::{
    HardwareTopology, HardwareTransport, MechanicalConfiguration, PartDefinition,
    PhysicalBoardInstance, ProjectDoc,
};
use serde_json::Value;
use std::collections::BTreeSet;

/// Presentation-owned event data; generated IDs are captured before proposal work starts.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SetupIntent {
    Topology {
        board_id: String,
        selected_instance_id: Option<String>,
        split: bool,
        new_primary_id: String,
        new_secondary_id: String,
    },
    Transport(HardwareTransport),
    ReversibleLayout(bool),
}

/// A Session acceptance may assign the next revision while retaining the exact proposal payload.
/// Keep this comparison at the owner boundary so unrelated or superseding accepted edits never
/// reconcile the physical-instance preference.
pub fn accepted_matches_proposal(accepted: &ProjectDoc, proposal: &ProjectDoc) -> bool {
    let mut normalized = accepted.clone();
    normalized.revision = proposal.revision;
    normalized == *proposal
}

pub fn can_reconcile_primary(
    outcome: &TerminalOutcome,
    owner_is_current: bool,
    accepted: Option<&ProjectDoc>,
    proposal: &ProjectDoc,
) -> bool {
    matches!(outcome, TerminalOutcome::Completed)
        && owner_is_current
        && accepted.is_some_and(|document| accepted_matches_proposal(document, proposal))
}

/// Build a reference-compatible proposal from an immutable accepted document.
pub fn propose(
    accepted: &ProjectDoc,
    intent: SetupIntent,
    mut normalize: impl FnMut(&PartDefinition, bool) -> Result<(PartDefinition, bool), String>,
) -> Result<ProjectDoc, String> {
    match intent {
        SetupIntent::Topology {
            board_id,
            selected_instance_id,
            split,
            new_primary_id,
            new_secondary_id,
        } => propose_topology(
            accepted,
            &board_id,
            selected_instance_id.as_deref(),
            split,
            &new_primary_id,
            &new_secondary_id,
        ),
        SetupIntent::Transport(transport) => propose_transport(accepted, transport),
        SetupIntent::ReversibleLayout(enabled) => {
            propose_reversible(accepted, enabled, &mut normalize)
        }
    }
}

fn propose_topology(
    accepted: &ProjectDoc,
    board_id: &str,
    selected_instance_id: Option<&str>,
    split: bool,
    new_primary_id: &str,
    new_secondary_id: &str,
) -> Result<ProjectDoc, String> {
    let old_hardware = accepted.hardware.as_ref();
    let current_topology = old_hardware
        .map(|hardware| hardware.topology)
        .unwrap_or_default();
    let target_topology = if split {
        HardwareTopology::Split
    } else {
        HardwareTopology::Unibody
    };
    if old_hardware.is_some() && current_topology == target_topology {
        return Ok(accepted.clone());
    }

    let old_instances = old_hardware
        .map(|hardware| hardware.instances.as_slice())
        .unwrap_or_default();
    let retained = selected_instance_id
        .and_then(|id| old_instances.iter().find(|instance| instance.id == id))
        .or_else(|| {
            old_instances
                .iter()
                .find(|instance| instance.board_id == board_id)
        });
    let mechanical = retained
        .and_then(|instance| instance.mechanical.clone())
        .or_else(|| {
            accepted
                .mechanical
                .as_ref()
                .filter(|configuration| configuration.board_id == board_id)
                .cloned()
        });
    let mechanical = match mechanical {
        Some(configuration) => configuration,
        None => boardstudio_web_host::case_settings::initial_settings(accepted, board_id)?,
    };

    let mut hardware = old_hardware.cloned().unwrap_or_default();
    hardware.topology = target_topology;
    hardware.transport = if split {
        HardwareTransport::Wireless
    } else {
        HardwareTransport::None
    };
    hardware.shared_construction = Some(
        hardware
            .shared_construction
            .clone()
            .unwrap_or_else(|| mechanical.clone()),
    );

    let primary = if let Some(retained) = retained {
        let mut primary = retained.clone();
        primary.half = if split { "left" } else { "unibody" }.into();
        primary.role = if split { "central" } else { "standalone" }.into();
        if matches!(
            primary.name.as_str(),
            "Keyboard" | "Left half" | "Right half"
        ) {
            primary.name = if split { "Left half" } else { "Keyboard" }.into();
        }
        primary
    } else {
        ensure_fresh_id(new_primary_id, old_instances)?;
        make_instance(
            new_primary_id,
            board_id,
            if split { "left" } else { "unibody" },
            if split { "central" } else { "standalone" },
            if split { "Left half" } else { "Keyboard" },
            false,
            mechanical.clone(),
        )
    };

    let mut instances = vec![primary];
    if split {
        if retained.is_none() && new_primary_id == new_secondary_id {
            return Err("Physical instance IDs must be distinct".into());
        }
        ensure_fresh_id(new_secondary_id, old_instances)?;
        let flipped = effective_reversible(accepted);
        let mut secondary_mechanical = mechanical;
        if flipped {
            secondary_mechanical.openings = Some(vec![]);
            secondary_mechanical.mounts.clear();
        }
        instances.push(make_instance(
            new_secondary_id,
            board_id,
            "right",
            "peripheral",
            "Right half",
            flipped,
            secondary_mechanical,
        ));
    }
    hardware.instances = instances;

    let mut proposed = accepted.clone();
    proposed.hardware = Some(hardware);
    Ok(proposed)
}

fn propose_transport(
    accepted: &ProjectDoc,
    transport: HardwareTransport,
) -> Result<ProjectDoc, String> {
    if !matches!(
        transport,
        HardwareTransport::Wired | HardwareTransport::Wireless
    ) {
        return Err("Split transport must be wired or wireless".into());
    }
    let mut proposed = accepted.clone();
    let hardware = proposed
        .hardware
        .as_mut()
        .filter(|hardware| hardware.topology == HardwareTopology::Split)
        .ok_or_else(|| "Split physical setup is unavailable".to_string())?;
    hardware.transport = transport;
    Ok(proposed)
}

fn propose_reversible(
    accepted: &ProjectDoc,
    enabled: bool,
    normalize: &mut impl FnMut(&PartDefinition, bool) -> Result<(PartDefinition, bool), String>,
) -> Result<ProjectDoc, String> {
    let mut proposed = accepted.clone();
    proposed
        .parameters
        .insert("reversibleLayout".into(), Value::Bool(enabled));
    let mut eligible = BTreeSet::new();
    proposed.definitions = accepted
        .definitions
        .iter()
        .map(|definition| {
            let (normalized, supports_reversible) = normalize(definition, enabled)?;
            if supports_reversible {
                eligible.insert(definition.id.clone());
            }
            Ok(normalized)
        })
        .collect::<Result<_, String>>()?;
    for part in &mut proposed.parts {
        if eligible.contains(&part.definition_id)
            && let Some(parameters) = part.generator_parameters.as_mut()
            && parameters.contains_key("reversible")
        {
            parameters.insert("reversible".into(), Value::Bool(enabled));
        }
    }
    if let Some(hardware) = proposed.hardware.as_mut() {
        for instance in &mut hardware.instances {
            instance.flipped = enabled && instance.half == "right";
        }
    }
    Ok(proposed)
}

fn effective_reversible(document: &ProjectDoc) -> bool {
    document
        .parameters
        .get("reversibleLayout")
        .and_then(Value::as_bool)
        .unwrap_or_else(|| {
            document
                .hardware
                .as_ref()
                .is_some_and(|hardware| hardware.instances.iter().any(|instance| instance.flipped))
        })
}

fn ensure_fresh_id(id: &str, existing: &[PhysicalBoardInstance]) -> Result<(), String> {
    if id.is_empty() || existing.iter().any(|instance| instance.id == id) {
        return Err("Physical instance ID must be non-empty and unique".into());
    }
    Ok(())
}

fn make_instance(
    id: &str,
    board_id: &str,
    half: &str,
    role: &str,
    name: &str,
    flipped: bool,
    mechanical: MechanicalConfiguration,
) -> PhysicalBoardInstance {
    PhysicalBoardInstance {
        id: id.into(),
        name: name.into(),
        board_id: board_id.into(),
        half: half.into(),
        role: role.into(),
        flipped,
        controller_part_id: None,
        mechanical: Some(mechanical),
        construction_linked: true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use boardstudio_core::model::{
        Board, ElectricalBoardConfiguration, HardwareConfiguration, HardwareTopology,
        HardwareTransport, MechanicalConfiguration, PartDefinition, PhysicalBoardInstance,
        ProjectDoc,
    };
    use serde_json::{Value, json};
    use std::collections::BTreeMap;

    fn document() -> ProjectDoc {
        let mut doc = ProjectDoc::empty("project", "Test project");
        doc.boards.push(Board {
            id: "board-a".into(),
            name: "Board A".into(),
            outline_ids: vec![],
            part_ids: vec![],
            net_ids: vec![],
            thickness: 1.6,
            traces: vec![],
            vias: vec![],
        });
        doc
    }

    fn mechanical(doc: &ProjectDoc, thickness: f64) -> MechanicalConfiguration {
        let mut value =
            boardstudio_web_host::case_settings::initial_settings(doc, "board-a").unwrap();
        value.plate_thickness = thickness;
        value
    }

    fn instance(doc: &ProjectDoc, id: &str, half: &str, flipped: bool) -> PhysicalBoardInstance {
        PhysicalBoardInstance {
            id: id.into(),
            name: format!("{half} assembly"),
            board_id: "board-a".into(),
            half: half.into(),
            role: "custom-role".into(),
            flipped,
            controller_part_id: Some(format!("{id}-controller")),
            mechanical: Some(mechanical(doc, 4.2)),
            construction_linked: false,
        }
    }

    fn keep_definition(
        definition: &PartDefinition,
        _enabled: bool,
    ) -> Result<(PartDefinition, bool), String> {
        Ok((definition.clone(), false))
    }

    #[test]
    fn topology_preserves_flipped_primary_and_shared_construction_and_adds_reversible_secondary() {
        let mut doc = document();
        let primary = instance(&doc, "primary", "right", true);
        let primary_mechanical = primary.mechanical.clone();
        let shared = mechanical(&doc, 6.0);
        let electrical = ElectricalBoardConfiguration {
            board_id: "board-a".into(),
            controller_part_id: Some("electrical-u1".into()),
            ..Default::default()
        };
        doc.hardware = Some(HardwareConfiguration {
            topology: HardwareTopology::Unibody,
            transport: HardwareTransport::None,
            instances: vec![primary.clone()],
            boards: vec![electrical.clone()],
            shared_construction: Some(shared.clone()),
        });
        doc.parameters.insert("unrelated".into(), json!("keep"));
        let accepted = doc.clone();

        let result = propose(
            &doc,
            SetupIntent::Topology {
                board_id: "board-a".into(),
                selected_instance_id: Some("primary".into()),
                split: true,
                new_primary_id: "unused".into(),
                new_secondary_id: "right-new".into(),
            },
            keep_definition,
        )
        .unwrap();

        assert_eq!(doc, accepted, "accepted input remains immutable");
        let hardware = result.hardware.unwrap();
        assert_eq!(hardware.topology, HardwareTopology::Split);
        assert_eq!(hardware.transport, HardwareTransport::Wireless);
        assert_eq!(hardware.boards, vec![electrical]);
        assert_eq!(hardware.shared_construction, Some(shared));
        assert_eq!(hardware.instances[0].id, primary.id);
        assert!(
            hardware.instances[0].flipped,
            "topology preserves retained orientation"
        );
        assert_eq!(hardware.instances[0].mechanical, primary_mechanical);
        assert_eq!(
            hardware.instances[0].controller_part_id,
            primary.controller_part_id
        );
        assert_eq!(hardware.instances[1].id, "right-new");
        assert_eq!(hardware.instances[1].half, "right");
        assert!(
            hardware.instances[1].flipped,
            "new secondary follows effective reversible mode"
        );
        let new_mechanical = hardware.instances[1].mechanical.as_ref().unwrap();
        assert!(new_mechanical.openings.as_ref().is_none_or(Vec::is_empty));
        assert!(new_mechanical.mounts.is_empty());
        assert_eq!(result.parameters.get("unrelated"), Some(&json!("keep")));
    }

    #[test]
    fn topology_falls_back_to_board_matching_primary_and_resolved_mechanics_for_shared_construction()
     {
        let mut doc = document();
        let primary = instance(&doc, "board-primary", "left", false);
        let primary_mechanical = primary.mechanical.clone();
        let mut another_board = instance(&doc, "other", "unibody", false);
        another_board.board_id = "another-board".into();
        doc.hardware = Some(HardwareConfiguration {
            topology: HardwareTopology::Split,
            transport: HardwareTransport::Wired,
            instances: vec![another_board, primary.clone()],
            boards: vec![],
            shared_construction: None,
        });
        let result = propose(
            &doc,
            SetupIntent::Topology {
                board_id: "board-a".into(),
                selected_instance_id: None,
                split: false,
                new_primary_id: "unused".into(),
                new_secondary_id: "unused-right".into(),
            },
            keep_definition,
        )
        .unwrap();
        let hardware = result.hardware.unwrap();
        assert_eq!(hardware.instances.len(), 1);
        assert_eq!(hardware.instances[0].id, primary.id);
        assert_eq!(hardware.shared_construction, primary_mechanical);
        assert_eq!(hardware.transport, HardwareTransport::None);
    }

    #[test]
    fn new_physical_instances_use_the_existing_mechanical_defaults_factory() {
        let doc = document();
        let result = propose(
            &doc,
            SetupIntent::Topology {
                board_id: "board-a".into(),
                selected_instance_id: None,
                split: true,
                new_primary_id: "primary-new".into(),
                new_secondary_id: "secondary-new".into(),
            },
            keep_definition,
        )
        .unwrap();
        let hardware = result.hardware.unwrap();
        let shared = hardware.shared_construction.as_ref().unwrap();
        assert_eq!(shared.board_id, "board-a");
        assert_eq!(shared.plate_thickness, 1.5);
        assert_eq!(hardware.instances[0].mechanical.as_ref(), Some(shared));
        assert_eq!(hardware.instances[1].mechanical.as_ref(), Some(shared));
        assert!(!hardware.instances[1].flipped);
    }

    #[test]
    fn topology_uses_matching_canonical_board_mechanics_before_the_defaults_factory() {
        let mut doc = document();
        doc.mechanical = Some(mechanical(&doc, 7.25));
        let canonical = doc.mechanical.clone().unwrap();
        let result = propose(
            &doc,
            SetupIntent::Topology {
                board_id: "board-a".into(),
                selected_instance_id: None,
                split: true,
                new_primary_id: "primary-new".into(),
                new_secondary_id: "secondary-new".into(),
            },
            keep_definition,
        )
        .unwrap();
        let hardware = result.hardware.unwrap();
        assert_eq!(hardware.shared_construction, Some(canonical.clone()));
        assert_eq!(hardware.instances[0].mechanical, Some(canonical.clone()));
        assert_eq!(hardware.instances[1].mechanical, Some(canonical));
    }

    #[test]
    fn transport_changes_only_the_transport_field() {
        let mut doc = document();
        doc.hardware = Some(HardwareConfiguration {
            topology: HardwareTopology::Split,
            transport: HardwareTransport::Wireless,
            instances: vec![instance(&doc, "primary", "left", true)],
            boards: vec![ElectricalBoardConfiguration {
                board_id: "board-a".into(),
                ..Default::default()
            }],
            shared_construction: Some(mechanical(&doc, 5.0)),
        });
        let accepted = doc.clone();
        let result = propose(
            &doc,
            SetupIntent::Transport(HardwareTransport::Wired),
            keep_definition,
        )
        .unwrap();
        let mut expected = accepted.hardware.clone().unwrap();
        expected.transport = HardwareTransport::Wired;
        assert_eq!(result.hardware, Some(expected));
        assert_eq!(doc, accepted);
    }

    #[test]
    fn selection_reconciliation_requires_the_exact_accepted_proposal() {
        let original = document();
        let mut proposal = original.clone();
        proposal
            .parameters
            .insert("reversibleLayout".into(), json!(true));
        proposal.revision = original.revision;

        let mut accepted = proposal.clone();
        accepted.revision += 1;
        assert!(accepted_matches_proposal(&accepted, &proposal));

        accepted.name.push_str(" changed after save");
        assert!(!accepted_matches_proposal(&accepted, &proposal));

        let mut matching = proposal.clone();
        matching.revision += 1;
        assert!(can_reconcile_primary(
            &TerminalOutcome::Completed,
            true,
            Some(&matching),
            &proposal,
        ));
        assert!(!can_reconcile_primary(
            &TerminalOutcome::PersistenceFailed("disk full".into()),
            true,
            Some(&matching),
            &proposal,
        ));
        assert!(!can_reconcile_primary(
            &TerminalOutcome::Completed,
            false,
            Some(&matching),
            &proposal,
        ));
    }

    #[test]
    fn reversible_changes_document_flag_and_only_eligible_existing_part_overrides() {
        let mut doc = document();
        doc.definitions = vec![
            definition("supported", "ceoloide/switch_mx"),
            definition("custom", "vendor/custom"),
        ];
        doc.parts = vec![
            part(
                "explicit",
                "supported",
                json!({"reversible":false,"custom":9}),
            ),
            part("implicit", "supported", json!({"custom":7})),
            part("custom", "custom", json!({"reversible":false})),
        ];
        doc.hardware = Some(HardwareConfiguration {
            instances: vec![
                instance(&doc, "left", "left", true),
                instance(&doc, "right", "right", false),
            ],
            ..Default::default()
        });
        let accepted = doc.clone();
        let result = propose(
            &doc,
            SetupIntent::ReversibleLayout(true),
            |definition, enabled| {
                if definition.id == "supported" {
                    let mut result = definition.clone();
                    result
                        .generator
                        .as_mut()
                        .unwrap()
                        .parameters
                        .insert("reversible".into(), json!(enabled));
                    Ok((result, true))
                } else {
                    Ok((definition.clone(), false))
                }
            },
        )
        .unwrap();
        assert_eq!(doc, accepted, "accepted input remains immutable");
        assert_eq!(
            result.parameters.get("reversibleLayout"),
            Some(&Value::Bool(true))
        );
        assert_eq!(result.definitions[1], doc.definitions[1]);
        assert_eq!(
            result.parts[0]
                .generator_parameters
                .as_ref()
                .unwrap()
                .get("reversible"),
            Some(&json!(true))
        );
        assert_eq!(
            result.parts[0]
                .generator_parameters
                .as_ref()
                .unwrap()
                .get("custom"),
            Some(&json!(9))
        );
        assert_eq!(
            result.parts[1]
                .generator_parameters
                .as_ref()
                .unwrap()
                .get("reversible"),
            None
        );
        assert_eq!(
            result.parts[2]
                .generator_parameters
                .as_ref()
                .unwrap()
                .get("reversible"),
            Some(&json!(false))
        );
        assert!(!result.hardware.as_ref().unwrap().instances[0].flipped);
        assert!(result.hardware.as_ref().unwrap().instances[1].flipped);
    }

    fn definition(id: &str, source: &str) -> PartDefinition {
        serde_json::from_value(json!({
            "id":id,"name":id,"kind":"switch","courtyard":[],"pads":[],
            "generator":{"source":source,"version":"1","parameters":{}}
        }))
        .unwrap()
    }

    fn part(id: &str, definition_id: &str, params: Value) -> boardstudio_core::model::Part {
        serde_json::from_value(json!({
            "id":id,"definitionId":definition_id,"reference":id,
            "pose":{"at":{"x":0,"y":0},"rotation":0},"side":"front",
            "generatorParameters":params
        }))
        .unwrap()
    }

    #[test]
    fn topology_noop_and_invalid_transport_are_rejected_without_changing_input() {
        let mut doc = document();
        doc.hardware = Some(HardwareConfiguration::default());
        let accepted = doc.clone();
        let same = propose(
            &doc,
            SetupIntent::Topology {
                board_id: "board-a".into(),
                selected_instance_id: None,
                split: false,
                new_primary_id: "primary".into(),
                new_secondary_id: "right".into(),
            },
            keep_definition,
        )
        .unwrap();
        assert_eq!(same, doc);
        assert!(
            propose(
                &doc,
                SetupIntent::Transport(HardwareTransport::Wired),
                keep_definition
            )
            .is_err()
        );
        assert_eq!(doc, accepted);
    }

    fn _stable_signature(_: &BTreeMap<String, Value>) {}
}
