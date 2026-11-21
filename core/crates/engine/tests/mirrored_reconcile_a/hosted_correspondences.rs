use super::*;

#[test]
fn hosted_module_maps_instance_connector_upstream_and_bus_references() {
    let mut document = hosted_electrical_fixture().unwrap();
    let source_module = document.modules.by_id_mut("source-module");
    source_module.host_instance_id = Some("left".into());
    source_module
        .connection
        .as_mut()
        .unwrap()
        .upstream_module_id = Some("source-upstream".into());
    let mut source_upstream = source_module.clone();
    source_upstream.id = "source-upstream".into();
    source_upstream.host_instance_id = Some("left".into());
    source_upstream
        .connection
        .as_mut()
        .unwrap()
        .upstream_module_id = None;
    let mut mirror_upstream = source_upstream.clone();
    mirror_upstream.id = "mirror-upstream".into();
    mirror_upstream.host_board_id = "mirror-board".into();
    mirror_upstream.host_instance_id = Some("right".into());
    mirror_upstream
        .connection
        .as_mut()
        .unwrap()
        .host_connector_part_id = "matrix/mirror-matrix/r0c0".into();
    mirror_upstream.connection.as_mut().unwrap().bus_id = "mirror-upstream-bus".into();
    mirror_upstream.at = Vec2 { x: 0.0, y: 0.0 };
    document.modules.push(source_upstream);
    document.modules.push(mirror_upstream);
    document
        .first_halves_mut()
        .correspondences
        .push(MirroredCorrespondence {
            companion_mode: None,
            companion_tweak: None,
            kind: MirroredItemKind::Module,
            source_id: "source-upstream".into(),
            mirror_id: "mirror-upstream".into(),
        });

    reopen(&mut document).unwrap();
    let mirror = document.modules.by_id("mirror-module");
    assert_eq!(mirror.host_instance_id.as_deref(), Some("right"));
    let connection = mirror.connection.as_ref().unwrap();
    assert_eq!(
        connection.host_connector_part_id,
        "matrix/mirror-matrix/r0c0"
    );
    assert_eq!(
        connection.upstream_module_id.as_deref(),
        Some("mirror-upstream")
    );
    assert_eq!(connection.bus_id, "mirror-board/source-bus");
}

#[test]
fn hosted_module_refuses_unmapped_nonempty_references_atomically() {
    let mut document = hosted_electrical_fixture().unwrap();
    let mut source_connector = document.parts.by_id("matrix/source-matrix/r0c0").clone();
    source_connector.id = "source-unpaired-connector".into();
    document.parts.push(source_connector);
    document
        .boards
        .by_id_mut("source-board")
        .part_ids
        .push("source-unpaired-connector".into());
    document
        .modules
        .by_id_mut("source-module")
        .connection
        .as_mut()
        .unwrap()
        .host_connector_part_id = "source-unpaired-connector".into();
    let before = document.clone();
    assert!(reopen(&mut document).is_err());
    assert_eq!(document, before);
}

#[test]
fn hosted_circuit_definition_payload_changes_project_to_the_existing_definition() {
    let mut document = hosted_electrical_fixture().unwrap();
    let source_definition = document
        .definitions
        .by_id_mut("embedded/source-circuit/definition/u");
    source_definition.name = "Updated Source U".into();
    source_definition.pads[0].size = Vec2 { x: 2.5, y: 3.5 };
    source_definition.pads[0].shape = PadShape::Roundrect;
    let (prepared, changed) = crate::support::prepare_transaction(
        &document,
        &[],
        &EditOperation::MoveParts { positions: vec![] },
    )
    .unwrap();
    reopen(&mut document).unwrap();
    assert_eq!(prepared.definitions, document.definitions);
    let mirror = document
        .definitions
        .by_id("embedded/mirror-circuit/definition/u");
    assert_eq!(mirror.name, "Updated Source U");
    assert_eq!(mirror.pads[0].shape, PadShape::Roundrect);
    assert_eq!(mirror.pads[0].size, Vec2 { x: 2.5, y: 3.5 });
    assert!(changed.contains(&"embedded/mirror-circuit/definition/u".to_owned()));
}

#[test]
fn hosted_joined_net_update_preserves_target_only_host_pins() {
    let mut document = hosted_electrical_fixture().unwrap();
    let mut host_part = document.parts.by_id("matrix/mirror-matrix/r0c1").clone();
    host_part.id = "mirror-host-only".into();
    document.parts.push(host_part);
    document
        .boards
        .by_id_mut("mirror-board")
        .part_ids
        .push("mirror-host-only".into());
    let host_net = document.nets.by_id_mut("host-mirror-net");
    host_net.pins.push(Pin {
        part_id: "mirror-host-only".into(),
        pad_id: "authored-host-pad".into(),
    });
    let source_net = document.nets.by_id_mut("host-source-net");
    source_net.name = "Edited host net".into();
    source_net.pins.push(Pin {
        part_id: "embedded/source-circuit/U1".into(),
        pad_id: "pad-two".into(),
    });

    reopen(&mut document).unwrap();
    let target = document.nets.by_id("host-mirror-net");
    assert!(target.pins.contains(&Pin {
        part_id: "mirror-host-only".into(),
        pad_id: "authored-host-pad".into()
    }));
    assert!(target.pins.contains(&Pin {
        part_id: "embedded/mirror-circuit/U1".into(),
        pad_id: "pad-two".into()
    }));
    assert_eq!(target.name, "Edited host net");
}

#[test]
fn hosted_circuit_local_member_and_net_exceptions_preserve_authored_payload() {
    let mut document = hosted_electrical_fixture().unwrap();
    let halves = document.first_halves_mut();
    halves.correspondences.retain(|association| {
        !((association.kind == MirroredItemKind::Part
            && association.mirror_id == "embedded/mirror-circuit/U1")
            || (association.kind == MirroredItemKind::Net
                && association.mirror_id == "embedded/mirror-circuit/N$1"))
    });
    halves.exceptions.extend([
        MirroredException {
            companion_mode: None,
            companion_tweak: None,
            kind: MirroredItemKind::Part,
            removed_copper: None,
            mirror_id: "embedded/mirror-circuit/U1".into(),
            state: MirroredExceptionState::Local,
        },
        MirroredException {
            companion_mode: None,
            companion_tweak: None,
            kind: MirroredItemKind::Net,
            removed_copper: None,
            mirror_id: "embedded/mirror-circuit/N$1".into(),
            state: MirroredExceptionState::Local,
        },
    ]);
    let local_part = document.parts.by_id_mut("embedded/mirror-circuit/U1");
    local_part.reference = "LOCAL9".into();
    local_part
        .properties
        .as_mut()
        .unwrap()
        .insert("authoredLocal".into(), serde_json::json!({"v": 7}));
    document.nets.by_id_mut("embedded/mirror-circuit/N$1").name = "Authored local net".into();
    document
        .parts
        .by_id_mut("embedded/source-circuit/U1")
        .reference = "U2".into();
    document.modules.by_id_mut("source-module").at = Vec2 { x: 17.0, y: -8.0 };

    reopen(&mut document).unwrap();
    let member = document.parts.by_id("embedded/mirror-circuit/U1");
    assert_eq!(member.reference, "LOCAL9");
    assert_eq!(
        member.properties.as_ref().unwrap()["authoredLocal"],
        serde_json::json!({"v": 7})
    );
    assert_eq!(
        document.nets.by_id("embedded/mirror-circuit/N$1").name,
        "Authored local net"
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
}

#[test]
fn hosted_associated_local_circuit_preserves_owner_and_owned_payload() {
    let mut document = hosted_electrical_fixture().unwrap();
    let halves = document.first_halves_mut();
    halves.exceptions.push(MirroredException {
        companion_mode: None,
        companion_tweak: None,
        kind: MirroredItemKind::EmbeddedCircuit,
        removed_copper: None,
        mirror_id: "mirror-circuit".into(),
        state: MirroredExceptionState::Local,
    });
    let target_circuit = document.embedded_circuits.by_id_mut("mirror-circuit");
    target_circuit
        .ports
        .insert("local-authored".into(), "host-mirror-net".into());
    target_circuit.net_ids.reverse();
    target_circuit
        .part_ids
        .push("embedded/mirror-circuit/local-extra".into());
    let mut local_extra = document.parts.by_id("embedded/mirror-circuit/U1").clone();
    local_extra.id = "embedded/mirror-circuit/local-extra".into();
    local_extra.definition_id = "embedded/mirror-circuit/definition/local-extra".into();
    local_extra.reference = "LOCAL-EXTRA".into();
    document.parts.push(local_extra);
    let mut local_definition = document
        .definitions
        .by_id("embedded/mirror-circuit/definition/u")
        .clone();
    local_definition.id = "embedded/mirror-circuit/definition/local-extra".into();
    local_definition.name = "Local extra definition".into();
    document.definitions.push(local_definition);
    document
        .boards
        .by_id_mut("mirror-board")
        .part_ids
        .push("embedded/mirror-circuit/local-extra".into());
    document
        .nets
        .by_id_mut("embedded/mirror-circuit/N$1")
        .pins
        .extend([
            Pin {
                part_id: "embedded/mirror-circuit/local-extra".into(),
                pad_id: "pad-1".into(),
            },
            Pin {
                part_id: "matrix/mirror-matrix/r0c0".into(),
                pad_id: "pad-one".into(),
            },
        ]);
    let target_member = document.parts.by_id_mut("embedded/mirror-circuit/U1");
    target_member.reference = "LOCAL-U".into();
    target_member.pose.at = Vec2 { x: 31.0, y: 42.0 };
    target_member
        .properties
        .as_mut()
        .unwrap()
        .insert("authoredLocal".into(), serde_json::json!({"v": 7}));
    document
        .definitions
        .by_id_mut("embedded/mirror-circuit/definition/u")
        .name = "Local U definition".into();
    document.nets.by_id_mut("embedded/mirror-circuit/N$1").name = "Local private net".into();
    document.modules.by_id_mut("source-module").at = Vec2 { x: 17.0, y: -8.0 };
    let source_joined_net = document.nets.by_id_mut("host-source-net");
    source_joined_net.name = "HOST SOURCE EDITED".into();
    source_joined_net.pins.push(Pin {
        part_id: "matrix/source-matrix/r0c0".into(),
        pad_id: "pad-one".into(),
    });
    let pair_before = pair_state_bytes(&document);

    reopen(&mut document).unwrap();

    let target_circuit = document.embedded_circuits.by_id("mirror-circuit");
    assert_eq!(
        target_circuit
            .ports
            .get("local-authored")
            .map(String::as_str),
        Some("host-mirror-net")
    );
    assert_eq!(
        target_circuit.part_ids,
        vec![
            "embedded/mirror-circuit/U1",
            "embedded/mirror-circuit/local-extra"
        ]
    );
    assert_eq!(
        target_circuit.net_ids,
        vec!["host-mirror-net", "embedded/mirror-circuit/N$1"]
    );
    let target_member = document.parts.by_id("embedded/mirror-circuit/U1");
    assert_eq!(target_member.reference, "LOCAL-U");
    assert_eq!(target_member.pose.at, Vec2 { x: 31.0, y: 42.0 });
    assert!(document.parts.iter().any(|part| {
        part.id == "embedded/mirror-circuit/local-extra"
            && part.reference == "LOCAL-EXTRA"
            && part.definition_id == "embedded/mirror-circuit/definition/local-extra"
    }));
    assert_eq!(pair_state_bytes(&document), pair_before);
    assert_eq!(
        target_member.properties.as_ref().unwrap()["authoredLocal"],
        serde_json::json!({"v": 7})
    );
    assert_eq!(
        document
            .definitions
            .by_id("embedded/mirror-circuit/definition/u")
            .name,
        "Local U definition"
    );
    let private_net = document.nets.by_id("embedded/mirror-circuit/N$1");
    assert_eq!(private_net.name, "Local private net");
    assert!(private_net.pins.contains(&Pin {
        part_id: "embedded/mirror-circuit/local-extra".into(),
        pad_id: "pad-1".into(),
    }));
    assert!(private_net.pins.contains(&Pin {
        part_id: "matrix/mirror-matrix/r0c0".into(),
        pad_id: "pad-one".into(),
    }));
    let joined_host_net = document.nets.by_id("host-mirror-net");
    assert_eq!(joined_host_net.name, "HOST SOURCE EDITED");
    assert!(joined_host_net.pins.contains(&Pin {
        part_id: "matrix/mirror-matrix/r0c0".into(),
        pad_id: "pad-one".into(),
    }));
    assert!((document.modules.by_id("mirror-module").at.x - 108.0).abs() < 0.02);
    let stable = document.clone();
    reopen(&mut document).unwrap();
    assert_eq!(document, stable);
}

#[test]
fn local_joined_net_exception_is_independent_of_local_circuit_owner() {
    let mut document = hosted_electrical_fixture().unwrap();
    document.first_halves_mut().exceptions.extend([
        MirroredException {
            companion_mode: None,
            companion_tweak: None,
            kind: MirroredItemKind::EmbeddedCircuit,
            mirror_id: "mirror-circuit".into(),
            state: MirroredExceptionState::Local,
            removed_copper: None,
        },
        MirroredException {
            companion_mode: None,
            companion_tweak: None,
            kind: MirroredItemKind::Net,
            mirror_id: "host-mirror-net".into(),
            state: MirroredExceptionState::Local,
            removed_copper: None,
        },
    ]);
    document.nets.by_id_mut("host-source-net").name = "HOST SOURCE EDITED".into();

    reopen(&mut document).unwrap();

    assert_eq!(document.nets.by_id("host-mirror-net").name, "HOST MIRROR");
}

#[test]
fn inherited_local_circuit_member_settings_and_host_net_pins_survive() {
    let mut document = hosted_electrical_fixture().unwrap();
    document
        .first_halves_mut()
        .exceptions
        .push(MirroredException {
            companion_mode: None,
            companion_tweak: None,
            kind: MirroredItemKind::EmbeddedCircuit,
            mirror_id: "mirror-circuit".into(),
            state: MirroredExceptionState::Local,
            removed_copper: None,
        });
    let mut keycaps = KeycapConfiguration::default();
    keycaps.keys.insert(
        "embedded/source-circuit/U1".into(),
        KeycapKeySettings {
            legend: Some("SOURCE".into()),
            ..Default::default()
        },
    );
    keycaps.keys.insert(
        "embedded/mirror-circuit/U1".into(),
        KeycapKeySettings {
            legend: Some("MIRROR".into()),
            ..Default::default()
        },
    );
    document.keycaps = Some(keycaps);

    let request = EditOperation::SetKeycapKey {
        key_id: "embedded/mirror-circuit/U1".into(),
        change: KeycapKeyChange::Legend {
            value: Some("LOCAL EDIT".into()),
        },
    };
    let canonical = crate::support::canonicalize(&document, &request)
        .expect("a member of a Local circuit edits its keycap setting as written");
    assert_eq!(canonical.operation, request);
    crate::support::apply_for_test(&mut document, &[], &canonical.operation)
        .expect("the mirror-side keycap edit applies");

    let mirror_definition = document
        .definitions
        .by_id_mut("embedded/mirror-circuit/definition/u");
    let mut local_pad = mirror_definition.pads[0].clone();
    local_pad.id = "pad-2".into();
    mirror_definition.pads.push(local_pad);
    document.nets.extend([
        Net {
            id: "source-plain-member-net".into(),
            name: "SOURCE MEMBER NET".into(),
            pins: vec![Pin {
                part_id: "embedded/source-circuit/U1".into(),
                pad_id: "pad-1".into(),
            }],
        },
        Net {
            id: "mirror-plain-member-net".into(),
            name: "LOCAL MEMBER NET".into(),
            pins: vec![Pin {
                part_id: "embedded/mirror-circuit/U1".into(),
                pad_id: "pad-2".into(),
            }],
        },
    ]);
    document
        .boards
        .by_id_mut("source-board")
        .net_ids
        .push("source-plain-member-net".into());
    document
        .boards
        .by_id_mut("mirror-board")
        .net_ids
        .push("mirror-plain-member-net".into());
    document
        .first_halves_mut()
        .correspondences
        .push(MirroredCorrespondence {
            companion_mode: None,
            companion_tweak: None,
            kind: MirroredItemKind::Net,
            source_id: "source-plain-member-net".into(),
            mirror_id: "mirror-plain-member-net".into(),
        });

    reopen(&mut document).expect("the Local circuit member remains valid through reconciliation");
    let keycaps = document.keycaps.as_ref().unwrap();
    assert_eq!(
        keycaps.keys["embedded/source-circuit/U1"].legend.as_deref(),
        Some("SOURCE")
    );
    assert_eq!(
        keycaps.keys["embedded/mirror-circuit/U1"].legend.as_deref(),
        Some("LOCAL EDIT")
    );
    assert_eq!(
        document.nets.by_id("mirror-plain-member-net").pins,
        vec![Pin {
            part_id: "embedded/mirror-circuit/U1".into(),
            pad_id: "pad-2".into(),
        }],
        "a host net keeps its pin on a Local circuit member",
    );
}

#[test]
fn associated_local_circuit_closure_refusals_are_atomic() {
    for malformed in [
        "duplicate-owner",
        "duplicate-target",
        "wrong-board",
        "wrong-source-board",
        "missing-member",
        "foreign-member",
        "missing-definition",
        "missing-net",
        "invalid-port",
        "foreign-private-pin",
    ] {
        let mut document = hosted_electrical_fixture().unwrap();
        let halves = document.hardware.as_mut().unwrap().pairs[0]
            .halves
            .as_mut()
            .unwrap();
        halves.exceptions.push(MirroredException {
            companion_mode: None,
            companion_tweak: None,
            kind: MirroredItemKind::EmbeddedCircuit,
            removed_copper: None,
            mirror_id: "mirror-circuit".into(),
            state: MirroredExceptionState::Local,
        });
        let target = document.embedded_circuits.by_id_mut("mirror-circuit");
        let target_copy = target.clone();
        match malformed {
            "duplicate-owner" => halves.correspondences.push(
                halves
                    .correspondences
                    .iter()
                    .find(|association| {
                        association.kind == MirroredItemKind::EmbeddedCircuit
                            && association.mirror_id == "mirror-circuit"
                    })
                    .unwrap()
                    .clone(),
            ),
            "duplicate-target" => document.embedded_circuits.push(target_copy),
            "wrong-board" => target.host_board_id = "source-board".into(),
            "wrong-source-board" => {
                document
                    .embedded_circuits
                    .by_id_mut("source-circuit")
                    .host_board_id = "mirror-board".into();
            }
            "missing-member" => target
                .part_ids
                .push("embedded/mirror-circuit/missing".into()),
            "foreign-member" => target.part_ids.push("embedded/other-circuit/U2".into()),
            "missing-definition" => {
                document
                    .parts
                    .by_id_mut("embedded/mirror-circuit/U1")
                    .definition_id = "embedded/mirror-circuit/definition/missing".into()
            }
            "missing-net" => target
                .net_ids
                .push("embedded/mirror-circuit/missing".into()),
            "invalid-port" => {
                target
                    .ports
                    .insert("invalid".into(), "embedded/mirror-circuit/missing".into());
            }
            "foreign-private-pin" => {
                document.nets.by_id_mut("embedded/mirror-circuit/N$1").pins[0].part_id =
                    "embedded/other-circuit/U1".into()
            }
            _ => unreachable!(),
        }
        let before = serde_json::to_vec(&document).unwrap();

        let error = reopen(&mut document)
            .expect_err("an invalid associated Local owner closure is refused");

        assert!(!error.is_empty(), "{malformed} produced no reason");
        if malformed == "wrong-source-board" {
            assert!(error.contains("not on board"), "{error}");
        }
        assert_eq!(
            serde_json::to_vec(&document).unwrap(),
            before,
            "{malformed}"
        );
    }
}

#[test]
fn hosted_local_circuit_exception_preserves_authored_circuit_while_linked_neighbors_update() {
    let mut document = hosted_electrical_fixture().unwrap();
    let halves = document.first_halves_mut();
    halves.correspondences.retain(|association| {
        !(association.kind == MirroredItemKind::EmbeddedCircuit
            && association.mirror_id == "mirror-circuit")
    });
    halves.exceptions.push(MirroredException {
        companion_mode: None,
        companion_tweak: None,
        kind: MirroredItemKind::EmbeddedCircuit,
        removed_copper: None,
        mirror_id: "mirror-circuit".into(),
        state: MirroredExceptionState::Local,
    });
    document
        .embedded_circuits
        .by_id_mut("mirror-circuit")
        .ports
        .insert("local-authored".into(), "host-mirror-net".into());
    document.modules.by_id_mut("source-module").at = Vec2 { x: 17.0, y: -8.0 };
    let pair_before = pair_state_bytes(&document);

    reopen(&mut document).unwrap();
    let circuit = document.embedded_circuits.by_id("mirror-circuit");
    assert_eq!(
        circuit.ports.get("local-authored").map(String::as_str),
        Some("host-mirror-net")
    );
    assert!((document.modules.by_id("mirror-module").at.x - 108.0).abs() < 0.02);
    assert_eq!(pair_state_bytes(&document), pair_before);
}
