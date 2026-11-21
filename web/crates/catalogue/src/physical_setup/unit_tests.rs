use super::*;

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
        pairs: vec![],
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
fn a_fresh_setup_half_owns_its_configuration_instead_of_the_legacy_overlay() {
    // A brand-new project: no physical instances yet.
    let doc = document();
    let proposal = propose_with_mirror_pair(
        &doc,
        SetupIntent::Topology {
            board_id: "board-a".into(),
            selected_instance_id: None,
            split: false,
            new_primary_id: "primary".into(),
            new_secondary_id: "unused".into(),
        },
        keep_definition,
    )
    .unwrap();
    let hardware = proposal.document.hardware.as_ref().expect("hardware");
    assert_eq!(hardware.instances.len(), 1);
    let first_half = &hardware.instances[0];
    assert_eq!(
        first_half.construction_ownership,
        MechanicalConstructionOwnership::Local,
        "a new unpaired half owns its own configuration, not the legacy overlay"
    );
    let own = first_half
        .mechanical
        .clone()
        .expect("the new half carries its configuration");

    // Its effective read is its own configuration: no legacy overlay and no
    // first-instance fallback can change what it shows.
    let resolved = |document: &ProjectDoc| {
        effective_inputs(
            document,
            &[],
            &MechanicalTarget::Instance {
                instance_id: first_half.id.clone(),
            },
        )
        .expect("the new half resolves")
        .configuration
    };
    let before = resolved(&proposal.document);
    assert_eq!(
        before.wall_thickness, own.wall_thickness,
        "the new half reads back its own value"
    );

    // A shared-field write on the legacy source (where a legacy member's edit routes)
    // does not reach the new half.
    let mut legacy_member_edit = proposal.document.clone();
    legacy_member_edit
        .hardware
        .as_mut()
        .expect("hardware")
        .shared_construction
        .as_mut()
        .expect("the setup still writes a shared construction for existing halves")
        .wall_thickness = 9.0;
    assert_eq!(
        resolved(&legacy_member_edit).wall_thickness,
        own.wall_thickness,
        "a shared-field edit on a legacy member cannot reach a local half"
    );

    // A split whose retained primary sits on another board is never pair-linked
    // either, so the half it creates must still own its configuration.
    let mut off_board = document();
    off_board.boards.push(Board {
        id: "board-b".into(),
        name: "Board B".into(),
        outline_ids: vec![],
        part_ids: vec![],
        net_ids: vec![],
        thickness: 1.6,
        traces: vec![],
        vias: vec![],
    });
    let mut foreign = instance(&off_board, "foreign", "left", false);
    foreign.board_id = "board-b".into();
    off_board.hardware = Some(HardwareConfiguration {
        topology: HardwareTopology::Unibody,
        transport: HardwareTransport::None,
        instances: vec![foreign],
        boards: vec![],
        shared_construction: None,
        pairs: vec![],
    });
    let split = propose_with_mirror_pair(
        &off_board,
        SetupIntent::Topology {
            board_id: "board-a".into(),
            selected_instance_id: Some("foreign".into()),
            split: true,
            new_primary_id: "unused".into(),
            new_secondary_id: "secondary-new".into(),
        },
        keep_definition,
    )
    .unwrap();
    assert!(
        split.mirror_pair.is_none(),
        "an off-board primary is never pair-linked"
    );
    let created = split
        .document
        .hardware
        .as_ref()
        .expect("hardware")
        .instances
        .iter()
        .find(|instance| instance.id == "secondary-new")
        .expect("the created half");
    assert_eq!(
        created.construction_ownership,
        MechanicalConstructionOwnership::Local,
        "an unpaired created half owns its configuration even when the primary is off-board"
    );
}

#[test]
fn topology_falls_back_to_board_matching_primary_and_resolved_mechanics_for_shared_construction() {
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
        pairs: vec![],
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
    assert!(hardware.instances[1].flipped);
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
        pairs: vec![],
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

/// The `reversible` flag the proposal writes into a part's parameters is a boolean, as the
/// generator declares it, so Core's parameter check (ADR-0009) accepts the proposed document.
#[test]
fn reversible_writes_a_boolean_that_the_core_parameter_check_accepts() {
    let mut doc = document();
    doc.definitions = vec![definition("supported", "ceoloide/switch_mx")];
    doc.parts = vec![part("explicit", "supported", json!({"reversible": false}))];
    assert_eq!(
        boardstudio_core_kernel::generators::parameter_problems(&doc),
        []
    );
    let result = propose(
        &doc,
        SetupIntent::ReversibleLayout(true),
        |definition, _| Ok((definition.clone(), true)),
    )
    .unwrap();
    assert_eq!(
        boardstudio_core_kernel::generators::parameter_problems(&result),
        []
    );
    assert_eq!(
        result.parts[0].generator_parameters.as_ref().unwrap()["reversible"],
        json!(true)
    );
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

// -- Mechanical mirror-pair setup -------------------------------------------------
