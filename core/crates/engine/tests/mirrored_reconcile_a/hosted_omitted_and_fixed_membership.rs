use super::*;

fn omit_circuit_children(document: &mut ProjectDoc) {
    let halves = document.first_halves_mut();
    for (kind, id) in [
        (MirroredItemKind::Part, "embedded/mirror-circuit/U1"),
        (MirroredItemKind::Net, "embedded/mirror-circuit/N$1"),
    ] {
        halves
            .correspondences
            .retain(|link| link.kind != kind || link.mirror_id != id);
        halves.exceptions.push(MirroredException {
            kind,
            mirror_id: id.into(),
            state: MirroredExceptionState::Omitted,
            companion_mode: None,
            companion_tweak: None,
            removed_copper: None,
        });
    }
}

#[test]
fn hosted_omitted_member_and_net_keep_their_unlinked_payload_under_linked_circuit() {
    let (mut document, projection) = hosted_electrical_fixture().unwrap();
    omit_circuit_children(&mut document);
    document
        .parts
        .by_id_mut("embedded/mirror-circuit/U1")
        .reference = "OMITTED-U".into();
    document.nets.by_id_mut("embedded/mirror-circuit/N$1").name = "Omitted private net".into();
    document
        .parts
        .by_id_mut("embedded/source-circuit/U1")
        .reference = "U2".into();
    document.modules.by_id_mut("source-module").at = Vec2 { x: 17.0, y: -8.0 };
    let pair_before = pair_state_bytes(&document);

    reconcile(&mut document, "pair-source-mirror", &projection).unwrap();
    assert_eq!(
        document.parts.by_id("embedded/mirror-circuit/U1").reference,
        "OMITTED-U"
    );
    assert_eq!(
        document.nets.by_id("embedded/mirror-circuit/N$1").name,
        "Omitted private net"
    );
    assert!(
        document
            .embedded_circuits
            .iter()
            .any(|circuit| circuit.id == "mirror-circuit")
    );
    let mirror_circuit = document.embedded_circuits.by_id("mirror-circuit");
    assert!(
        mirror_circuit
            .part_ids
            .contains(&"embedded/mirror-circuit/U1".to_owned())
    );
    assert!(
        mirror_circuit
            .net_ids
            .contains(&"embedded/mirror-circuit/N$1".to_owned())
    );
    assert!((document.modules.by_id("mirror-module").at.x - 108.0).abs() < 0.02);
    assert_eq!(pair_state_bytes(&document), pair_before);
}

#[test]
fn hosted_omitted_member_and_net_can_be_absent_under_linked_circuit() {
    let (mut document, projection) = hosted_electrical_fixture().unwrap();
    reconcile(&mut document, "pair-source-mirror", &projection)
        .expect("the historical linked owner baseline is fully materialized first");
    omit_circuit_children(&mut document);

    // The durable Omitted ownership facts are supplied before this target-side
    // materialization snapshot; the source members and their linked circuit stay
    // present, and the kernel must leave the pair record byte-identical.
    let pre_materialization = document.clone();
    boardstudio_core_mechanical::mechanical::halves::validate_document(&pre_materialization)
        .expect("the durable Omitted owner baseline has real target records");
    document
        .parts
        .retain(|part| part.id != "embedded/mirror-circuit/U1");
    document
        .nets
        .retain(|net| net.id != "embedded/mirror-circuit/N$1");
    let mirror_circuit = document.embedded_circuits.by_id_mut("mirror-circuit");
    mirror_circuit
        .part_ids
        .retain(|id| id != "embedded/mirror-circuit/U1");
    mirror_circuit
        .net_ids
        .retain(|id| id != "embedded/mirror-circuit/N$1");
    let mirror_board = document.boards.by_id_mut("mirror-board");
    mirror_board
        .part_ids
        .retain(|id| id != "embedded/mirror-circuit/U1");
    mirror_board
        .net_ids
        .retain(|id| id != "embedded/mirror-circuit/N$1");
    document
        .nets
        .by_id_mut("host-mirror-net")
        .pins
        .retain(|pin| pin.part_id != "embedded/mirror-circuit/U1");
    let omission_key = (
        "pair-source-mirror".into(),
        "source-circuit".into(),
        "mirror-circuit".into(),
    );
    let historical_pair_bytes = pair_state_bytes(&pre_materialization);
    projection
        .hosted_omission_baselines
        .borrow_mut()
        .insert(omission_key.clone(), pre_materialization.clone());
    let pair_before = pair_state_bytes(&document);
    assert_eq!(pair_before, historical_pair_bytes);
    let applied_before = projection.applied_hosted_operations.borrow().clone();
    let removal_context_before = projection.hosted_removal_baselines.borrow().clone();
    let omission_context_before = projection.hosted_omission_baselines.borrow().clone();
    document.modules.by_id_mut("source-module").at = Vec2 { x: 17.0, y: -8.0 };

    let changed = reconcile(&mut document, "pair-source-mirror", &projection)
        .expect("explicit Omitted ownership permits absent target children under a linked circuit");
    assert_eq!(changed, vec!["mirror-module"]);
    assert!(
        document
            .parts
            .iter()
            .all(|part| part.id != "embedded/mirror-circuit/U1")
    );
    assert!(
        document
            .nets
            .iter()
            .all(|net| net.id != "embedded/mirror-circuit/N$1")
    );
    let mirror_circuit = document
        .embedded_circuits
        .iter()
        .find(|circuit| circuit.id == "mirror-circuit")
        .expect("the linked target circuit remains independently owned");
    assert!(
        !mirror_circuit
            .part_ids
            .contains(&"embedded/mirror-circuit/U1".to_owned())
    );
    assert!(
        !mirror_circuit
            .net_ids
            .contains(&"embedded/mirror-circuit/N$1".to_owned())
    );
    assert!(document.embedded_circuits.iter().any(|circuit| {
        circuit.id == "source-circuit"
            && circuit
                .part_ids
                .contains(&"embedded/source-circuit/U1".to_owned())
            && circuit
                .net_ids
                .contains(&"embedded/source-circuit/N$1".to_owned())
    }));
    let joined_mirror_net = document
        .nets
        .iter()
        .find(|net| net.id == "host-mirror-net")
        .expect("the independently owned joined host net remains present");
    assert!(
        joined_mirror_net
            .pins
            .iter()
            .any(|pin| { pin.part_id == "matrix/mirror-matrix/r0c0" && pin.pad_id == "pad-two" })
    );
    assert!(
        joined_mirror_net
            .pins
            .iter()
            .all(|pin| pin.part_id != "embedded/mirror-circuit/U1")
    );
    assert!((document.modules.by_id("mirror-module").at.x - 108.0).abs() < 0.02);
    assert_eq!(pair_state_bytes(&document), pair_before);
    assert_eq!(
        *projection.applied_hosted_operations.borrow(),
        applied_before,
        "Omitted owner input does not create Add provenance"
    );
    assert_eq!(
        *projection.hosted_removal_baselines.borrow(),
        removal_context_before,
        "Omitted owner input does not create lifecycle removal evidence"
    );
    assert_eq!(
        *projection.hosted_omission_baselines.borrow(),
        omission_context_before,
        "the exact validated Omitted owner context remains unchanged"
    );

    let first_output = document.clone();
    let repeated = reconcile(&mut document, "pair-source-mirror", &projection)
        .expect("the updated omitted-child context is replayable");
    assert!(repeated.is_empty());
    assert_eq!(
        document, first_output,
        "a repeat changes no document identity"
    );
    assert_eq!(pair_state_bytes(&document), pair_before);
    assert_eq!(
        *projection.applied_hosted_operations.borrow(),
        applied_before
    );
    assert_eq!(
        *projection.hosted_removal_baselines.borrow(),
        removal_context_before
    );
    assert_eq!(
        *projection.hosted_omission_baselines.borrow(),
        omission_context_before
    );
    assert_ne!(pre_materialization.parts.len(), document.parts.len());
}

#[test]
fn hosted_module_and_embedded_circuit_edits_follow_established_correspondences() {
    let (mut document, projection) = hosted_electrical_fixture().unwrap();
    reconcile(&mut document, "pair-source-mirror", &projection)
        .expect("the literal source and target records establish one valid baseline");
    let source_reference_before = document.board_references[0].clone();
    let source_copper_before = document.boards.by_id("source-board").traces.clone();
    document.modules.by_id_mut("source-module").at = Vec2 { x: 17.0, y: -8.0 };
    document.modules.by_id_mut("source-module").rotation = 37.0;
    let source_member = document.parts.by_id_mut("embedded/source-circuit/U1");
    source_member.pose = Pose2 {
        at: Vec2 { x: 11.0, y: 13.0 },
        rotation: 23.0,
    };
    source_member
        .properties
        .as_mut()
        .unwrap()
        .insert("edited".into(), serde_json::json!({"revision": 2}));
    document.nets.by_id_mut("embedded/source-circuit/N$1").name = "PRIVATE EDITED".into();
    document
        .nets
        .by_id_mut("embedded/source-circuit/N$1")
        .pins
        .push(Pin {
            part_id: "matrix/source-matrix/r0c0".into(),
            pad_id: "pad-two".into(),
        });
    document
        .embedded_circuits
        .by_id_mut("source-circuit")
        .ports
        .insert("signal".into(), "embedded/source-circuit/N$1".into());
    let pair_before = pair_state_bytes(&document);
    let changed = reconcile(&mut document, "pair-source-mirror", &projection).unwrap();
    assert_eq!(pair_state_bytes(&document), pair_before);

    let mirror_module = document.modules.by_id("mirror-module");
    assert!((mirror_module.at.x - 108.0).abs() < 0.02);
    assert!((mirror_module.at.y + 47.0).abs() < 0.02);
    assert_eq!(mirror_module.rotation, 233.0);
    assert_eq!(
        mirror_module
            .connection
            .as_ref()
            .unwrap()
            .host_connector_part_id,
        "matrix/mirror-matrix/r0c0"
    );
    let mirror_member = document.parts.by_id("embedded/mirror-circuit/U1");
    assert!((mirror_member.pose.at.x - 87.0).abs() < 0.02);
    assert!((mirror_member.pose.at.y + 41.0).abs() < 0.02);
    assert_eq!(mirror_member.pose.rotation, 247.0);
    assert_eq!(
        mirror_member.properties.as_ref().unwrap().get("edited"),
        Some(&serde_json::json!({"revision": 2}))
    );
    let mirror_private_net = document.nets.by_id("embedded/mirror-circuit/N$1");
    assert_eq!(mirror_private_net.name, "PRIVATE EDITED");
    assert_eq!(
        mirror_private_net.pins[0].part_id,
        "embedded/mirror-circuit/U1"
    );
    assert_eq!(
        mirror_private_net.pins[1].part_id,
        "matrix/mirror-matrix/r0c0"
    );
    assert_eq!(mirror_private_net.pins[1].pad_id, "pad-two");
    assert_eq!(
        document.embedded_circuits.by_id("mirror-circuit").ports["signal"],
        "embedded/mirror-circuit/N$1"
    );
    assert_eq!(
        changed,
        vec![
            "embedded/mirror-circuit/N$1",
            "embedded/mirror-circuit/U1",
            "mirror-circuit",
            "mirror-module",
        ]
    );
    let mirror_electrical = document
        .hardware
        .as_ref()
        .unwrap()
        .boards
        .iter()
        .find(|entry| entry.board_id == "mirror-board")
        .unwrap();
    assert_eq!(
        mirror_electrical.controller_part_id.as_deref(),
        Some("embedded/mirror-circuit/U1")
    );
    assert_eq!(
        mirror_electrical
            .locks
            .get("direct/embedded/mirror-circuit/U1")
            .map(String::as_str),
        Some("embedded/mirror-circuit/N$1")
    );
    assert_eq!(
        mirror_electrical
            .assignments
            .get("peripheral/embedded/mirror-circuit/U1/signal")
            .map(String::as_str),
        Some("host-mirror-net")
    );
    assert_eq!(
        mirror_electrical
            .key_bindings
            .get("local-function")
            .map(String::as_str),
        Some("mirror-local-binding")
    );
    assert_eq!(document.board_references, vec![source_reference_before]);
    assert_eq!(
        document.boards.by_id("source-board").traces,
        source_copper_before
    );
    assert!(
        !document
            .boards
            .by_id("mirror-board")
            .traces
            .iter()
            .any(|trace| trace.id == "source-only-copper")
    );
    let first_output = document.clone();
    let repeated_changed = reconcile(&mut document, "pair-source-mirror", &projection).unwrap();
    assert!(
        repeated_changed.is_empty(),
        "the same hosted projection converges"
    );
    assert_eq!(
        document, first_output,
        "whole document and private pair context repeat exactly"
    );
}

#[test]
fn removing_a_hosted_module_keeps_its_independently_owned_connector() {
    let (mut document, mut projection) = hosted_electrical_fixture().unwrap();
    boardstudio_core_mechanical::mechanical::halves::validate_document(&document)
        .expect("the canonical pre-source module baseline validates");
    projection.hosted_removal_baselines.borrow_mut().insert(
        (
            "remove-module".into(),
            "source-module".into(),
            "mirror-module".into(),
        ),
        document.clone(),
    );
    boardstudio_core_mechanical::modules::remove(&mut document, "source-module")
        .expect("the ordinary source operation removes only the module record");
    projection.hosted_transitions = vec![HostedTransition::RemoveModule {
        source_id: "source-module".into(),
        mirror_id: "mirror-module".into(),
    }];
    let pair_before = pair_state_bytes(&document);

    reconcile(&mut document, "pair-source-mirror", &projection)
        .expect("the source module removal has a complete target identity");
    assert_eq!(pair_state_bytes(&document), pair_before);
    assert!(projection.applied_hosted_operations.borrow().contains(&(
        "remove-module".into(),
        "source-module".into(),
        "mirror-module".into()
    )));

    assert!(
        !document
            .modules
            .iter()
            .any(|module| module.id == "mirror-module")
    );
    assert!(
        document
            .parts
            .iter()
            .any(|part| part.id == "matrix/mirror-matrix/r0c0")
    );
    assert!(
        document
            .boards
            .by_id("mirror-board")
            .part_ids
            .contains(&"matrix/mirror-matrix/r0c0".into())
    );
    let first_output = document.clone();
    let first_context = projection.applied_hosted_operations.borrow().clone();
    let repeated = reconcile(&mut document, "pair-source-mirror", &projection);
    assert!(
        repeated.is_ok(),
        "the same hosted removal must accept its full output: {:?}",
        repeated.as_ref().err()
    );
    assert!(repeated.unwrap().is_empty());
    assert_eq!(document, first_output);
    assert_eq!(
        *projection.applied_hosted_operations.borrow(),
        first_context
    );
}

#[test]
fn circuit_removal_preserves_joined_host_nets_copper_and_unrelated_parts() {
    let (mut document, mut projection) = hosted_electrical_fixture().unwrap();
    boardstudio_core_mechanical::mechanical::halves::validate_document(&document)
        .expect("the canonical pre-source circuit baseline validates");
    projection.hosted_removal_baselines.borrow_mut().insert(
        (
            "remove-circuit".into(),
            "source-circuit".into(),
            "mirror-circuit".into(),
        ),
        document.clone(),
    );
    boardstudio_core_mechanical::modules::remove_circuit(&mut document, "source-circuit")
        .expect("the ordinary source operation removes the source circuit");
    document
        .boards
        .by_id_mut("mirror-board")
        .traces
        .push(CopperTrace {
            id: "joined-net-route".into(),
            start: Vec2 { x: 1.0, y: 2.0 },
            end: Vec2 { x: 3.0, y: 4.0 },
            width: 0.25,
            layer: Side::Front,
            net_id: Some("host-mirror-net".into()),
        });
    projection.hosted_transitions = vec![HostedTransition::RemoveCircuit {
        source_id: "source-circuit".into(),
        mirror_id: "mirror-circuit".into(),
    }];
    let pair_before = pair_state_bytes(&document);

    reconcile(&mut document, "pair-source-mirror", &projection)
        .expect("the source circuit removal reconciles through the existing domain closure");
    assert_eq!(pair_state_bytes(&document), pair_before);
    assert!(projection.applied_hosted_operations.borrow().contains(&(
        "remove-circuit".into(),
        "source-circuit".into(),
        "mirror-circuit".into()
    )));

    let mirror_board = document.boards.by_id("mirror-board");
    assert!(
        !document
            .embedded_circuits
            .iter()
            .any(|circuit| circuit.id == "mirror-circuit")
    );
    assert!(
        !document
            .parts
            .iter()
            .any(|part| part.id == "embedded/mirror-circuit/U1")
    );
    assert!(
        document
            .parts
            .iter()
            .any(|part| part.id == "geometry-probe")
    );
    assert!(mirror_board.part_ids.contains(&"geometry-probe".into()));
    assert!(document.nets.iter().any(|net| net.id == "host-mirror-net"));
    assert!(document.nets.iter().any(|net| {
        net.id == "host-mirror-net"
            && net
                .pins
                .iter()
                .any(|pin| pin.part_id == "matrix/mirror-matrix/r0c0")
    }));
    assert!(
        mirror_board
            .traces
            .iter()
            .any(|trace| trace.id == "joined-net-route"
                && trace.net_id.as_deref() == Some("host-mirror-net"))
    );
    assert!(
        !document
            .definitions
            .iter()
            .any(|definition| definition.id == "embedded/mirror-circuit/definition/u")
    );
    let first_output = document.clone();
    let first_context = projection.applied_hosted_operations.borrow().clone();
    let repeated = reconcile(&mut document, "pair-source-mirror", &projection);
    assert!(
        repeated.is_ok(),
        "the same hosted removal must accept its full output: {:?}",
        repeated.as_ref().err()
    );
    assert!(repeated.unwrap().is_empty());
    assert_eq!(document, first_output);
    assert_eq!(
        *projection.applied_hosted_operations.borrow(),
        first_context
    );
}

#[test]
fn locked_circuit_removal_refuses_atomically() {
    let (mut document, mut projection) = hosted_electrical_fixture().unwrap();
    projection.hosted_removal_baselines.borrow_mut().insert(
        (
            "remove-circuit".into(),
            "source-circuit".into(),
            "mirror-circuit".into(),
        ),
        document.clone(),
    );
    boardstudio_core_mechanical::modules::remove_circuit(&mut document, "source-circuit")
        .expect("the ordinary source operation removes the source circuit");
    document
        .parts
        .by_id_mut("embedded/mirror-circuit/U1")
        .locked = Some(true);
    projection.hosted_transitions = vec![HostedTransition::RemoveCircuit {
        source_id: "source-circuit".into(),
        mirror_id: "mirror-circuit".into(),
    }];
    let before = document.clone();

    assert!(reconcile(&mut document, "pair-source-mirror", &projection).is_err());
    assert_eq!(
        document, before,
        "a late domain refusal cannot publish a partial circuit cleanup"
    );
}

#[test]
fn hosted_module_removal_refuses_an_unpaired_foreign_target_atomically() {
    let (mut document, mut projection) = hosted_electrical_fixture().unwrap();
    let mut unrelated_source = document.modules.by_id("source-module").clone();
    unrelated_source.id = "unpaired-source-module".into();
    let mut unrelated_target = document.modules.by_id("mirror-module").clone();
    unrelated_target.id = "unpaired-target-module".into();
    document
        .modules
        .extend([unrelated_source, unrelated_target]);
    boardstudio_core_mechanical::mechanical::halves::validate_document(&document)
        .expect("the unpaired modules are outside valid pair ownership");
    projection.hosted_removal_baselines.borrow_mut().insert(
        (
            "remove-module".into(),
            "unpaired-source-module".into(),
            "unpaired-target-module".into(),
        ),
        document.clone(),
    );
    boardstudio_core_mechanical::modules::remove(&mut document, "unpaired-source-module")
        .expect("the ordinary source operation removes the selected source module");
    projection.hosted_transitions = vec![HostedTransition::RemoveModule {
        source_id: "unpaired-source-module".into(),
        mirror_id: "unpaired-target-module".into(),
    }];
    let before = document.clone();
    let pair_before = pair_state_bytes(&document);
    let applied_before = projection.applied_hosted_operations.borrow().clone();
    let baselines_before = projection.hosted_removal_baselines.borrow().clone();

    assert!(
        reconcile(&mut document, "pair-source-mirror", &projection).is_err(),
        "a valid source removal cannot authorize deleting a target outside its pair"
    );
    assert_eq!(document, before);
    assert_eq!(pair_state_bytes(&document), pair_before);
    assert_eq!(
        *projection.applied_hosted_operations.borrow(),
        applied_before
    );
    assert_eq!(
        *projection.hosted_removal_baselines.borrow(),
        baselines_before
    );
    assert!(
        document
            .modules
            .iter()
            .any(|module| module.id == "unpaired-target-module")
    );
}

#[test]
fn hosted_circuit_removal_refuses_an_unpaired_foreign_target_atomically() {
    let (mut document, mut projection) = hosted_electrical_fixture().unwrap();
    document.embedded_circuits.extend([
        EmbeddedCircuit {
            id: "unpaired-source-circuit".into(),
            definition_id: "module-definition".into(),
            host_board_id: "source-board".into(),
            part_ids: vec![],
            net_ids: vec![],
            ports: BTreeMap::new(),
        },
        EmbeddedCircuit {
            id: "unpaired-target-circuit".into(),
            definition_id: "module-definition".into(),
            host_board_id: "mirror-board".into(),
            part_ids: vec![],
            net_ids: vec![],
            ports: BTreeMap::new(),
        },
    ]);
    boardstudio_core_mechanical::mechanical::halves::validate_document(&document)
        .expect("the unpaired circuits are outside valid pair ownership");
    projection.hosted_removal_baselines.borrow_mut().insert(
        (
            "remove-circuit".into(),
            "unpaired-source-circuit".into(),
            "unpaired-target-circuit".into(),
        ),
        document.clone(),
    );
    boardstudio_core_mechanical::modules::remove_circuit(&mut document, "unpaired-source-circuit")
        .expect("the ordinary source operation removes the selected source circuit");
    projection.hosted_transitions = vec![HostedTransition::RemoveCircuit {
        source_id: "unpaired-source-circuit".into(),
        mirror_id: "unpaired-target-circuit".into(),
    }];
    let before = document.clone();
    let pair_before = pair_state_bytes(&document);
    let applied_before = projection.applied_hosted_operations.borrow().clone();
    let baselines_before = projection.hosted_removal_baselines.borrow().clone();

    assert!(
        reconcile(&mut document, "pair-source-mirror", &projection).is_err(),
        "a valid source removal cannot authorize deleting a circuit outside its pair"
    );
    assert_eq!(document, before);
    assert_eq!(pair_state_bytes(&document), pair_before);
    assert_eq!(
        *projection.applied_hosted_operations.borrow(),
        applied_before
    );
    assert_eq!(
        *projection.hosted_removal_baselines.borrow(),
        baselines_before
    );
    assert!(
        document
            .embedded_circuits
            .iter()
            .any(|circuit| circuit.id == "unpaired-target-circuit")
    );
}

#[test]
fn local_module_exception_keeps_other_hosted_correspondences_linked() {
    let (mut document, projection) = hosted_electrical_fixture().unwrap();
    reconcile(&mut document, "pair-source-mirror", &projection).unwrap();
    let local_module_pose = Pose2 {
        at: Vec2 {
            x: 999.0,
            y: -888.0,
        },
        rotation: 61.0,
    };
    let halves = document.first_halves_mut();
    halves.correspondences.retain(|association| {
        !(association.kind == MirroredItemKind::Module && association.mirror_id == "mirror-module")
    });
    halves.exceptions.push(MirroredException {
        companion_mode: None,
        companion_tweak: None,
        kind: MirroredItemKind::Module,
        removed_copper: None,
        mirror_id: "mirror-module".into(),
        state: MirroredExceptionState::Local,
    });
    let mirror_module = document.modules.by_id_mut("mirror-module");
    mirror_module.at = local_module_pose.at;
    mirror_module.rotation = local_module_pose.rotation;
    document.modules.by_id_mut("source-module").at = Vec2 { x: 21.0, y: 15.0 };
    document
        .parts
        .by_id_mut("embedded/source-circuit/U1")
        .pose
        .at = Vec2 { x: 14.0, y: 12.0 };
    let pair_before = pair_state_bytes(&document);

    reconcile(&mut document, "pair-source-mirror", &projection)
        .expect("a Local module is retained while other items reconcile");
    assert_eq!(pair_state_bytes(&document), pair_before);

    let mirror_module = document.modules.by_id("mirror-module");
    assert_eq!(mirror_module.at, local_module_pose.at);
    assert_eq!(mirror_module.rotation, local_module_pose.rotation);
    assert_eq!(
        mirror_module
            .connection
            .as_ref()
            .unwrap()
            .host_connector_part_id,
        "matrix/mirror-matrix/r0c0"
    );
    let mirror_member = document.parts.by_id("embedded/mirror-circuit/U1");
    assert!((mirror_member.pose.at.x - 88.0).abs() < 0.02);
    assert!((mirror_member.pose.at.y + 44.0).abs() < 0.02);
}

#[test]
fn omitted_embedded_circuit_exception_stays_durable_and_unprojected() {
    let (mut document, projection) = hosted_electrical_fixture().unwrap();
    let halves = document.first_halves_mut();
    halves.correspondences.retain(|association| {
        !((association.kind == MirroredItemKind::EmbeddedCircuit
            && association.mirror_id == "mirror-circuit")
            || (association.kind == MirroredItemKind::Part
                && association.mirror_id == "embedded/mirror-circuit/U1")
            || (association.kind == MirroredItemKind::Net
                && association.mirror_id == "embedded/mirror-circuit/N$1"))
    });
    halves.exceptions.extend([
        MirroredException {
            companion_mode: None,
            companion_tweak: None,
            kind: MirroredItemKind::EmbeddedCircuit,
            removed_copper: None,
            mirror_id: "mirror-circuit".into(),
            state: MirroredExceptionState::Omitted,
        },
        MirroredException {
            companion_mode: None,
            companion_tweak: None,
            kind: MirroredItemKind::Part,
            removed_copper: None,
            mirror_id: "embedded/mirror-circuit/U1".into(),
            state: MirroredExceptionState::Omitted,
        },
        MirroredException {
            companion_mode: None,
            companion_tweak: None,
            kind: MirroredItemKind::Net,
            removed_copper: None,
            mirror_id: "embedded/mirror-circuit/N$1".into(),
            state: MirroredExceptionState::Omitted,
        },
    ]);
    document
        .parts
        .by_id_mut("embedded/source-circuit/U1")
        .pose
        .at = Vec2 { x: 14.0, y: 12.0 };
    let mirror_circuit_before = document.embedded_circuits.by_id("mirror-circuit").clone();
    let mirror_member_before = document.parts.by_id("embedded/mirror-circuit/U1").clone();
    let mirror_net_before = document.nets.by_id("embedded/mirror-circuit/N$1").clone();
    let pair_before = pair_state_bytes(&document);

    reconcile(&mut document, "pair-source-mirror", &projection)
        .expect("an unlinked Omitted circuit remains under exception ownership");
    assert_eq!(pair_state_bytes(&document), pair_before);

    assert_eq!(
        document.embedded_circuits.by_id("mirror-circuit"),
        &mirror_circuit_before
    );
    assert_eq!(
        document.parts.by_id("embedded/mirror-circuit/U1"),
        &mirror_member_before
    );
    assert_eq!(
        document.nets.by_id("embedded/mirror-circuit/N$1"),
        &mirror_net_before
    );
}

#[test]
fn fixed_membership_assembly_fixture_reconciles_existing_companions() {
    let expected_changed = vec![
        "matrix/mirror-matrix/r0c1",
        "matrix/mirror-matrix/r0c1/diode",
        "matrix/mirror-matrix/r0c1/fallback",
        "matrix/mirror-matrix/r0c2",
        "matrix/mirror-matrix/r0c2/diode",
        "matrix/mirror-matrix/r1c0",
        "matrix/mirror-matrix/r1c0/diode",
        "matrix/mirror-matrix/r1c2",
        "matrix/mirror-matrix/r1c2/diode",
        "matrix/mirror-matrix/r1c2/stab",
        "mirror-matrix",
    ];
    let source_keys = [
        ("matrix/source-matrix/r0c0", (7.0, -1.0, 90.0)),
        ("matrix/source-matrix/r0c1", (8.0, 13.0, 0.0)),
        ("matrix/source-matrix/r0c2", (8.0, 5.0, 180.0)),
        ("matrix/source-matrix/r1c0", (-15.0, 0.0, 90.0)),
        ("matrix/source-matrix/r1c1", (9.0, 35.0, 0.0)),
        ("matrix/source-matrix/r1c2", (-17.0, 4.0, 90.0)),
    ];
    let source_companions = [
        ("matrix/source-matrix/r0c0/diode", (4.0, 1.0, 120.0)),
        ("matrix/source-matrix/r0c1/diode", (10.0, 16.0, 30.0)),
        ("matrix/source-matrix/r0c2/diode", (6.0, 2.0, 210.0)),
        ("matrix/source-matrix/r1c0/diode", (-18.0, 2.0, 120.0)),
        ("matrix/source-matrix/r1c1/diode", (11.0, 38.0, 30.0)),
        ("matrix/source-matrix/r1c2/diode", (-20.0, 6.0, 120.0)),
        ("matrix/source-matrix/r1c2/stab", (-17.0, 8.0, 90.0)),
        ("matrix/source-matrix/r0c1/fallback", (13.0, 11.0, 45.0)),
    ];
    let expected_rotated = [
        ("matrix/mirror-matrix/r0c0", (0.0, 0.0, 0.0)),
        ("matrix/mirror-matrix/r0c1", (87.0, -38.0, 90.0)),
        ("matrix/mirror-matrix/r0c2", (95.0, -38.0, -90.0)),
        ("matrix/mirror-matrix/r1c0", (100.0, -15.0, 0.0)),
        ("matrix/mirror-matrix/r1c1", (10.0, 20.0, 0.0)),
        ("matrix/mirror-matrix/r1c2", (96.0, -13.0, 0.0)),
        ("matrix/mirror-matrix/r0c0/diode", (2.0, 3.0, 30.0)),
        ("matrix/mirror-matrix/r0c1/diode", (84.0, -40.0, 60.0)),
        ("matrix/mirror-matrix/r0c2/diode", (98.0, -40.0, -60.0)),
        ("matrix/mirror-matrix/r1c0/diode", (99.0, -14.0, -15.0)),
        ("matrix/mirror-matrix/r1c1/diode", (12.0, 23.0, 30.0)),
        ("matrix/mirror-matrix/r1c2/diode", (94.0, -10.0, -30.0)),
        ("matrix/mirror-matrix/r1c2/stab", (92.0, -13.0, 90.0)),
        ("matrix/mirror-matrix/r0c1/fallback", (89.0, -43.0, 45.0)),
    ];
    let expected_axis = [
        ("matrix/mirror-matrix/r0c0", (0.0, 0.0, 0.0)),
        ("matrix/mirror-matrix/r0c1", (92.0, -17.0, 0.0)),
        ("matrix/mirror-matrix/r0c2", (92.0, -25.0, -180.0)),
        ("matrix/mirror-matrix/r1c0", (115.0, -30.0, -90.0)),
        ("matrix/mirror-matrix/r1c1", (10.0, 20.0, 0.0)),
        ("matrix/mirror-matrix/r1c2", (117.0, -26.0, -90.0)),
        ("matrix/mirror-matrix/r0c0/diode", (2.0, 3.0, 30.0)),
        ("matrix/mirror-matrix/r0c1/diode", (90.0, -14.0, -30.0)),
        ("matrix/mirror-matrix/r0c2/diode", (90.0, -28.0, -150.0)),
        ("matrix/mirror-matrix/r1c0/diode", (116.0, -29.0, -105.0)),
        ("matrix/mirror-matrix/r1c1/diode", (12.0, 23.0, 30.0)),
        ("matrix/mirror-matrix/r1c2/diode", (120.0, -24.0, -120.0)),
        ("matrix/mirror-matrix/r1c2/stab", (117.0, -22.0, 90.0)),
        ("matrix/mirror-matrix/r0c1/fallback", (87.0, -19.0, -45.0)),
    ];
    let assert_pose = |document: &ProjectDoc, id: &str, expected: (f64, f64, f64)| {
        let part = document.parts.by_id(id);
        assert!((part.pose.at.x - expected.0).abs() < 1e-9, "{id} x");
        assert!((part.pose.at.y - expected.1).abs() < 1e-9, "{id} y");
        let heading_delta = (part.pose.rotation - expected.2 + 180.0).rem_euclid(360.0) - 180.0;
        assert!(
            heading_delta.abs() < 1e-9,
            "{id} rotation: {}",
            part.pose.rotation
        );
    };
    let non_pose = |part: &Part| {
        let mut value = serde_json::to_value(part).unwrap();
        value.as_object_mut().unwrap().remove("pose");
        value
    };

    for (mapping_rotation, expected_members, frame, local_cells) in [
        (
            90.0,
            &expected_rotated[..],
            (102.0, -33.0, 90.0),
            [((0, 0), (101.0, 37.0, 0.0)), ((1, 1), (-59.0, 55.0, 90.0))],
        ),
        (
            0.0,
            &expected_axis[..],
            (97.0, -32.0, 0.0),
            [((0, 0), (31.0, -93.0, -90.0)), ((1, 1), (81.0, 15.0, 0.0))],
        ),
    ] {
        let (mut document, projection) =
            fixed_membership_assembly_fixture(mapping_rotation).unwrap();
        for id in ["source-matrix", "mirror-matrix"] {
            let matrix = document.matrices.by_id(id);
            boardstudio_core_kernel::matrix::valid_matrix(matrix, &document).unwrap();
        }
        boardstudio_core_mechanical::mechanical::features::validate(&document).unwrap();
        boardstudio_core_mechanical::mechanical::halves::validate_document(&document).unwrap();
        assert_eq!(projection.parts.len(), 4);
        assert_eq!(projection.locally_protected_mirror_parts.len(), 2);
        assert_eq!(projection.final_owners.len(), 28);
        let member_parts: Vec<_> = document
            .parts
            .iter()
            .filter(|part| projection.final_owners.contains_key(&part.id))
            .collect();
        assert_eq!(member_parts.len(), 28);
        assert_eq!(
            member_parts
                .iter()
                .map(|part| part.reference.as_str())
                .collect::<BTreeSet<_>>()
                .len(),
            28
        );
        assert!(member_parts.iter().all(|part| {
            part.locked == Some(true)
                && part.properties.as_ref().unwrap()["custom"]
                    == serde_json::json!({"part": part.id})
                && part.generator_parameters.as_ref().unwrap()["custom"]
                    == serde_json::json!({"part": part.id})
        }));
        for matrix_id in ["source-matrix", "mirror-matrix"] {
            let matrix = document.matrices.by_id(matrix_id);
            assert_eq!(matrix.part_ids.len(), 14);
        }
        assert_eq!(
            document
                .first_halves()
                .correspondences
                .iter()
                .filter(|association| association.kind == MirroredItemKind::Part)
                .count(),
            14
        );
        for (id, expected) in source_keys.iter().chain(source_companions.iter()) {
            assert_pose(&document, id, *expected);
        }
        for (retained_id, expected_net, expected_constraint) in [
            (
                "matrix/mirror-matrix/r0c0/diode",
                "retained-diode-net-0",
                "retained-diode-constraint-0",
            ),
            (
                "matrix/mirror-matrix/r1c1/diode",
                "retained-diode-net-1",
                "retained-diode-constraint-1",
            ),
        ] {
            assert!(document.nets.iter().any(|net| {
                net.id == expected_net
                    && net
                        .pins
                        .iter()
                        .any(|pin| pin.part_id == retained_id && pin.pad_id == "pad-one")
            }));
            assert!(
                document
                    .constraints
                    .iter()
                    .any(|constraint| constraint.id() == expected_constraint)
            );
            assert!(document.outline.iter().any(|feature| matches!(feature, OutlineFeature::PartEnvelope { part_ids, .. } if part_ids.iter().any(|id| id == retained_id))));
            assert!(document.boards.iter().any(|board| {
                board.id == "mirror-board"
                    && board.part_ids.iter().any(|id| id == retained_id)
                    && board.net_ids.contains(&expected_net.into())
                    && board
                        .outline_ids
                        .contains(&"retained-diodes-envelope".into())
            }));
        }

        let entry = document.clone();
        let source_ids = entry.matrices.by_id("source-matrix").part_ids.clone();
        let retained_ids = [
            "matrix/mirror-matrix/r0c0",
            "matrix/mirror-matrix/r0c0/diode",
            "matrix/mirror-matrix/r1c1",
            "matrix/mirror-matrix/r1c1/diode",
        ];
        let source_parts: Vec<_> = source_ids
            .iter()
            .map(|id| {
                entry
                    .parts
                    .iter()
                    .find(|part| &part.id == id)
                    .unwrap()
                    .clone()
            })
            .collect();
        let retained_parts: Vec<_> = retained_ids
            .iter()
            .map(|id| entry.parts.by_id(id).clone())
            .collect();
        let affected_ids: Vec<_> = entry
            .matrices
            .iter()
            .filter(|matrix| matrix.id == "source-matrix" || matrix.id == "mirror-matrix")
            .flat_map(|matrix| matrix.part_ids.iter())
            .cloned()
            .collect();
        let non_pose_before: Vec<_> = affected_ids
            .iter()
            .map(|id| non_pose(entry.parts.iter().find(|part| &part.id == id).unwrap()))
            .collect();
        let references_before = serde_json::json!({
            "boards": entry.boards,
            "nets": entry.nets,
            "constraints": entry.constraints,
            "boardOutlines": entry.board_outlines,
            "outline": entry.outline,
        });
        let correspondences_before = entry.first_halves().correspondences.clone();

        let changed = reconcile(&mut document, "pair-source-mirror", &projection).unwrap();
        assert_eq!(changed, expected_changed);
        assert_eq!(
            document.first_halves().correspondences,
            correspondences_before
        );
        for (id, expected) in expected_members {
            assert_pose(&document, id, *expected);
        }
        let final_matrix = document.matrices.by_id("mirror-matrix");
        assert_eq!(
            final_matrix.origin,
            Vec2 {
                x: frame.0,
                y: frame.1
            }
        );
        assert!((final_matrix.rotation.unwrap_or(0.0) - frame.2).abs() < 1e-9);
        assert_eq!(final_matrix.mirror, Some(Mirror::X));
        let expected_part_ids = [
            "matrix/mirror-matrix/r0c0",
            "matrix/mirror-matrix/r0c0/diode",
            "matrix/mirror-matrix/r0c1",
            "matrix/mirror-matrix/r0c1/diode",
            "matrix/mirror-matrix/r0c1/fallback",
            "matrix/mirror-matrix/r0c2",
            "matrix/mirror-matrix/r0c2/diode",
            "matrix/mirror-matrix/r1c0",
            "matrix/mirror-matrix/r1c0/diode",
            "matrix/mirror-matrix/r1c1",
            "matrix/mirror-matrix/r1c1/diode",
            "matrix/mirror-matrix/r1c2",
            "matrix/mirror-matrix/r1c2/diode",
            "matrix/mirror-matrix/r1c2/stab",
        ];
        assert_eq!(final_matrix.part_ids, expected_part_ids);
        for ((row, column), (x, y, rotation)) in local_cells {
            let cell = final_matrix
                .cells
                .iter()
                .find(|cell| (cell.row, cell.column) == (row, column))
                .unwrap();
            assert!((cell.offset.unwrap().x - x).abs() < 1e-9);
            assert!((cell.offset.unwrap().y - y).abs() < 1e-9);
            assert!((cell.rotation.unwrap() - rotation).abs() < 1e-9);
        }
        for (id, expected) in source_ids.iter().zip(&source_parts) {
            assert_eq!(
                document.parts.iter().find(|part| &part.id == id).unwrap(),
                expected
            );
        }
        for (id, expected) in retained_ids.iter().zip(&retained_parts) {
            assert_eq!(document.parts.by_id(id), expected);
        }
        let non_pose_after: Vec<_> = affected_ids
            .iter()
            .map(|id| non_pose(document.parts.iter().find(|part| &part.id == id).unwrap()))
            .collect();
        assert_eq!(non_pose_after, non_pose_before);
        assert_eq!(
            serde_json::json!({
                "boards": document.boards,
                "nets": document.nets,
                "constraints": document.constraints,
                "boardOutlines": document.board_outlines,
                "outline": document.outline,
            }),
            references_before
        );
        for (id, owner) in &projection.final_owners {
            assert_eq!(
                document
                    .boards
                    .iter()
                    .filter(|board| board.part_ids.contains(id))
                    .map(|board| board.id.as_str())
                    .collect::<Vec<_>>(),
                vec![owner.as_str()],
                "owner for {id}"
            );
        }

        let mut disposable = document.clone();
        boardstudio_core_kernel::matrix::set_matrix(&mut disposable, final_matrix).unwrap();
        let rematerialized = disposable.matrices.by_id("mirror-matrix");
        assert_eq!(rematerialized, final_matrix);
        for (id, expected) in expected_members {
            assert_pose(&disposable, id, *expected);
        }
        let before_repeat = document.clone();
        assert!(
            reconcile(&mut document, "pair-source-mirror", &projection)
                .unwrap()
                .is_empty()
        );
        assert_eq!(document, before_repeat);
    }
}
