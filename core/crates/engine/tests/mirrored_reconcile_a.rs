#[path = "support/base.rs"]
pub mod base_fixtures;
#[path = "support/reconcile.rs"]
pub mod reconcile_fixtures;
pub mod support;

use base_fixtures::*;
use boardstudio_core_mirrored_half::mirrored_half::reconcile::*;
use boardstudio_model::model::*;
use reconcile_fixtures::*;
use std::collections::{BTreeMap, BTreeSet};
#[test]
fn stage_hosted_correspondence_refuses_conflicts_and_requires_replay_provenance() {
    let (mut document, _) = hosted_electrical_fixture().unwrap();
    let existing = MirroredCorrespondence {
        companion_mode: None,
        companion_tweak: None,
        kind: MirroredItemKind::Module,
        source_id: "source-module".into(),
        mirror_id: "mirror-module".into(),
    };
    let before = document.clone();

    let same_source_conflict = MirroredCorrespondence {
        mirror_id: "mirror-module-other".into(),
        ..existing.clone()
    };
    assert!(
        boardstudio_core_mirrored_half::mirrored_half::reconcile::stage_hosted_correspondence(
            &mut document,
            "pair-source-mirror",
            false,
            same_source_conflict,
        )
        .is_err()
    );
    assert_eq!(document, before);

    let same_mirror_conflict = MirroredCorrespondence {
        source_id: "source-module-other".into(),
        ..existing.clone()
    };
    assert!(
        boardstudio_core_mirrored_half::mirrored_half::reconcile::stage_hosted_correspondence(
            &mut document,
            "pair-source-mirror",
            false,
            same_mirror_conflict,
        )
        .is_err()
    );
    assert_eq!(document, before);

    assert!(
        boardstudio_core_mirrored_half::mirrored_half::reconcile::stage_hosted_correspondence(
            &mut document,
            "pair-source-mirror",
            false,
            existing.clone(),
        )
        .is_err()
    );
    assert_eq!(document, before);
    boardstudio_core_mirrored_half::mirrored_half::reconcile::stage_hosted_correspondence(
        &mut document,
        "pair-source-mirror",
        true,
        existing.clone(),
    )
    .expect("private replay provenance admits the exact existing identity");
    assert_eq!(document, before);

    let fresh = MirroredCorrespondence {
        companion_mode: None,
        companion_tweak: None,
        kind: MirroredItemKind::Module,
        source_id: "source-module-added".into(),
        mirror_id: "mirror-module-added".into(),
    };
    boardstudio_core_mirrored_half::mirrored_half::reconcile::stage_hosted_correspondence(
        &mut document,
        "pair-source-mirror",
        false,
        fresh.clone(),
    )
    .expect("a fresh selected identity is staged without replay provenance");
    let halves = document.hardware.as_ref().unwrap().pairs[0]
        .halves
        .as_ref()
        .unwrap();
    assert_eq!(
        halves
            .correspondences
            .iter()
            .filter(|association| **association == fresh)
            .count(),
        1
    );
    assert_eq!(
        halves.correspondences.len(),
        before.hardware.as_ref().unwrap().pairs[0]
            .halves
            .as_ref()
            .unwrap()
            .correspondences
            .len()
            + 1
    );
}

#[test]
fn hosted_add_module_admits_its_pair_record_and_replays_idempotently() {
    let (mut document, mut projection) = hosted_electrical_fixture().unwrap();
    document
        .modules
        .retain(|module| module.id != "mirror-module");
    document.hardware.as_mut().unwrap().pairs[0]
        .halves
        .as_mut()
        .unwrap()
        .correspondences
        .retain(|association| {
            !(association.kind == MirroredItemKind::Module
                && association.source_id == "source-module")
        });
    projection.hosted_transitions = vec![HostedTransition::AddModule {
        source_id: "source-module".into(),
        mirror_id: "mirror-module-added".into(),
    }];
    let pair_before = pair_state_bytes(&document);

    reconcile(&mut document, "pair-source-mirror", &projection).unwrap();
    let halves = document.hardware.as_ref().unwrap().pairs[0]
        .halves
        .as_ref()
        .unwrap();
    assert_eq!(
        halves
            .correspondences
            .iter()
            .filter(|association| {
                association.kind == MirroredItemKind::Module
                    && association.source_id == "source-module"
                    && association.mirror_id == "mirror-module-added"
            })
            .count(),
        1,
        "the selected hosted module identity becomes a durable pair record"
    );
    let first_document = document.clone();
    let first_context = projection.applied_hosted_operations.borrow().clone();
    let first_circuit_context = projection.applied_hosted_circuit_additions.borrow().clone();
    let repeated = reconcile(&mut document, "pair-source-mirror", &projection);
    assert!(
        repeated.is_ok(),
        "the same hosted addition must accept its own full output"
    );
    assert!(repeated.unwrap().is_empty());
    assert_eq!(document, first_document);
    assert_eq!(
        *projection.applied_hosted_operations.borrow(),
        first_context
    );
    assert_eq!(
        *projection.applied_hosted_circuit_additions.borrow(),
        first_circuit_context
    );
    assert_eq!(
        first_context,
        BTreeSet::from([(
            "module".into(),
            "source-module".into(),
            "mirror-module-added".into()
        )])
    );
    assert_ne!(pair_state_bytes(&document), pair_before);

    let (mut stale, mut stale_projection) = hosted_electrical_fixture().unwrap();
    stale_projection.hosted_transitions = vec![HostedTransition::AddModule {
        source_id: "source-module".into(),
        mirror_id: "mirror-module".into(),
    }];
    let stale_before = stale.clone();
    assert!(reconcile(&mut stale, "pair-source-mirror", &stale_projection).is_err());
    assert_eq!(stale, stale_before);
}

#[test]
fn hosted_add_circuit_admits_selected_pair_records_and_replays_idempotently() {
    let (mut document, mut projection) = hosted_electrical_fixture().unwrap();
    document
        .embedded_circuits
        .retain(|circuit| circuit.id != "mirror-circuit");
    document
        .parts
        .retain(|part| part.id != "embedded/mirror-circuit/U1");
    document
        .nets
        .iter_mut()
        .find(|net| net.id == "host-mirror-net")
        .unwrap()
        .pins
        .retain(|pin| pin.part_id != "embedded/mirror-circuit/U1");
    document
        .definitions
        .retain(|definition| definition.id != "embedded/mirror-circuit/definition/u");
    document
        .nets
        .retain(|net| net.id != "embedded/mirror-circuit/N$1");
    let mirror_board = document
        .boards
        .iter_mut()
        .find(|board| board.id == "mirror-board")
        .unwrap();
    mirror_board
        .part_ids
        .retain(|id| id != "embedded/mirror-circuit/U1");
    mirror_board
        .net_ids
        .retain(|id| id != "embedded/mirror-circuit/N$1");
    document.hardware.as_mut().unwrap().pairs[0]
        .halves
        .as_mut()
        .unwrap()
        .correspondences
        .retain(|association| !match association.kind {
            MirroredItemKind::Part => association.source_id == "embedded/source-circuit/U1",
            MirroredItemKind::EmbeddedCircuit => association.source_id == "source-circuit",
            MirroredItemKind::Net => association.source_id == "embedded/source-circuit/N$1",
            _ => false,
        });
    projection.hosted_transitions = vec![HostedTransition::AddCircuit {
        source_id: "source-circuit".into(),
        mirror_id: "mirror-circuit-added".into(),
        member_ids: BTreeMap::from([(
            "embedded/source-circuit/U1".into(),
            "embedded/mirror-circuit-added/U1".into(),
        )]),
        member_references: BTreeMap::from([("embedded/source-circuit/U1".into(), "U9".into())]),
        definition_ids: BTreeMap::from([(
            "embedded/source-circuit/definition/u".into(),
            "embedded/mirror-circuit-added/definition/u".into(),
        )]),
        net_ids: BTreeMap::from([
            (
                "embedded/source-circuit/N$1".into(),
                "embedded/mirror-circuit-added/N$1".into(),
            ),
            ("host-source-net".into(), "host-mirror-net".into()),
        ]),
        member_poses: BTreeMap::new(),
    }];
    let pair_before = pair_state_bytes(&document);

    reconcile(&mut document, "pair-source-mirror", &projection).unwrap();
    let halves = document.hardware.as_ref().unwrap().pairs[0]
        .halves
        .as_ref()
        .unwrap();
    for (kind, source_id, mirror_id) in [
        (
            MirroredItemKind::EmbeddedCircuit,
            "source-circuit",
            "mirror-circuit-added",
        ),
        (
            MirroredItemKind::Part,
            "embedded/source-circuit/U1",
            "embedded/mirror-circuit-added/U1",
        ),
        (
            MirroredItemKind::Net,
            "embedded/source-circuit/N$1",
            "embedded/mirror-circuit-added/N$1",
        ),
        (MirroredItemKind::Net, "host-source-net", "host-mirror-net"),
    ] {
        assert_eq!(
            halves
                .correspondences
                .iter()
                .filter(|association| {
                    association.kind == kind
                        && association.source_id == source_id
                        && association.mirror_id == mirror_id
                })
                .count(),
            1,
            "selected hosted circuit identities become durable exactly once: {kind:?} {source_id:?} -> {mirror_id:?}"
        );
    }
    assert_eq!(
        document
            .parts
            .iter()
            .find(|part| part.id == "embedded/mirror-circuit-added/U1")
            .unwrap()
            .definition_id,
        "embedded/mirror-circuit-added/definition/u",
        "member definition identity stays in the existing namespace map"
    );
    let first_document = document.clone();
    let first_joined_net = document
        .nets
        .iter()
        .find(|net| net.id == "host-mirror-net")
        .unwrap()
        .clone();
    let first_context = projection.applied_hosted_operations.borrow().clone();
    let first_circuit_context = projection.applied_hosted_circuit_additions.borrow().clone();
    let repeated = reconcile(&mut document, "pair-source-mirror", &projection);
    assert!(
        repeated.is_ok(),
        "the same hosted circuit addition must accept its own full output"
    );
    let repeated_changes = repeated.unwrap();
    let repeated_joined_net = document
        .nets
        .iter()
        .find(|net| net.id == "host-mirror-net")
        .unwrap();
    assert!(
        repeated_changes.is_empty(),
        "authenticated AddCircuit replay must be idempotent: {repeated_changes:?}; before={first_joined_net:?}, after={repeated_joined_net:?}"
    );
    assert_eq!(document, first_document);
    assert_eq!(
        *projection.applied_hosted_operations.borrow(),
        first_context
    );
    assert_eq!(
        *projection.applied_hosted_circuit_additions.borrow(),
        first_circuit_context
    );
    assert_eq!(
        first_context,
        BTreeSet::from([(
            "circuit".into(),
            "source-circuit".into(),
            "mirror-circuit-added".into()
        )])
    );
    assert_ne!(pair_state_bytes(&document), pair_before);

    // The first AddCircuit path appends the member pin to the existing joined host net.
    // Once the addition is admitted, ordinary circuit projection owns subsequent updates and
    // applies its established source-net metadata normalization.
    let first_added_joined_name = first_joined_net.name.clone();
    projection.hosted_transitions.clear();
    projection.applied_hosted_operations.borrow_mut().clear();
    projection
        .applied_hosted_circuit_additions
        .borrow_mut()
        .clear();
    let source_member = document
        .parts
        .iter_mut()
        .find(|part| part.id == "embedded/source-circuit/U1")
        .unwrap();
    source_member.pose.at.x += 2.0;
    document
        .parts
        .iter_mut()
        .find(|part| part.id == "embedded/mirror-circuit-added/U1")
        .unwrap()
        .reference = "MIRROR-LOCAL9".into();
    let after_addition_edit = reconcile(&mut document, "pair-source-mirror", &projection)
        .expect("the admitted member maps participate in ordinary projection");
    assert!(after_addition_edit.contains(&"embedded/mirror-circuit-added/U1".to_owned()));
    let projected_member = document
        .parts
        .iter()
        .find(|part| part.id == "embedded/mirror-circuit-added/U1")
        .unwrap();
    assert_ne!(
        projected_member.pose,
        first_document
            .parts
            .iter()
            .find(|part| part.id == "embedded/mirror-circuit-added/U1")
            .unwrap()
            .pose
    );
    assert_eq!(projected_member.reference, "MIRROR-LOCAL9");
    let ordinary_joined_name = document
        .nets
        .iter()
        .find(|net| net.id == "host-mirror-net")
        .unwrap()
        .name
        .clone();
    assert_eq!(ordinary_joined_name, "HOST");
    assert_ne!(ordinary_joined_name, first_added_joined_name);

    let normalized_document = document.clone();
    let normalized_repeat = reconcile(&mut document, "pair-source-mirror", &projection)
        .expect("the ordinary circuit projection remains idempotent after normalization");
    assert!(normalized_repeat.is_empty());
    assert_eq!(document, normalized_document);

    let serialized = serde_json::to_vec(&document).unwrap();
    let mut reopened: ProjectDoc = serde_json::from_slice(&serialized).unwrap();
    let reopened_repeat = reconcile(&mut reopened, "pair-source-mirror", &projection)
        .expect("durable hosted correspondences survive save/open");
    assert!(reopened_repeat.is_empty());
    assert_eq!(reopened, normalized_document);

    let mut removal_input = reopened.clone();
    removal_input
        .constraints
        .retain(|constraint| constraint.id() != "retained-second-key-offset");
    let (removed, _) = crate::support::prepare_transaction(
        &removal_input,
        &[],
        &boardstudio_model::model::EditOperation::RemoveEmbeddedCircuit {
            id: "source-circuit".into(),
        },
    )
    .expect("the supported source transaction removes an admitted circuit");
    let removed_halves = removed.hardware.as_ref().unwrap().pairs[0]
        .halves
        .as_ref()
        .unwrap();
    assert!(!removed_halves.correspondences.iter().any(|association| {
        matches!(
            association.kind,
            MirroredItemKind::EmbeddedCircuit | MirroredItemKind::Part | MirroredItemKind::Net
        ) && (association.source_id == "source-circuit"
            || association
                .source_id
                .starts_with("embedded/source-circuit/"))
            && (association.mirror_id == "mirror-circuit-added"
                || association
                    .mirror_id
                    .starts_with("embedded/mirror-circuit-added/"))
    }));
    assert!(removed_halves.correspondences.iter().any(|association| {
        association.kind == MirroredItemKind::Net
            && association.source_id == "host-source-net"
            && association.mirror_id == "host-mirror-net"
    }));
    assert!(
        !removed
            .embedded_circuits
            .iter()
            .any(|circuit| circuit.id == "mirror-circuit-added")
    );
    assert!(
        !removed
            .parts
            .iter()
            .any(|part| part.id == "embedded/mirror-circuit-added/U1")
    );
    if let Some(retained_private_net) = removed
        .nets
        .iter()
        .find(|net| net.id == "embedded/mirror-circuit-added/N$1")
    {
        assert!(retained_private_net.pins.is_empty());
    }

    let (mut stale, mut stale_projection) = hosted_electrical_fixture().unwrap();
    stale_projection.hosted_transitions = vec![HostedTransition::AddCircuit {
        source_id: "source-circuit".into(),
        mirror_id: "mirror-circuit".into(),
        member_ids: BTreeMap::from([(
            "embedded/source-circuit/U1".into(),
            "embedded/mirror-circuit/U1".into(),
        )]),
        member_references: BTreeMap::from([("embedded/source-circuit/U1".into(), "U1".into())]),
        definition_ids: BTreeMap::from([(
            "embedded/source-circuit/definition/u".into(),
            "embedded/mirror-circuit/definition/u".into(),
        )]),
        net_ids: BTreeMap::from([
            (
                "embedded/source-circuit/N$1".into(),
                "embedded/mirror-circuit/N$1".into(),
            ),
            ("host-source-net".into(), "host-mirror-net".into()),
        ]),
        member_poses: BTreeMap::new(),
    }];
    let stale_before = stale.clone();
    assert!(reconcile(&mut stale, "pair-source-mirror", &stale_projection).is_err());
    assert_eq!(stale, stale_before);
}

#[test]
fn hosted_module_maps_instance_connector_upstream_and_bus_references() {
    let (mut document, projection) = hosted_electrical_fixture().unwrap();
    let source_module = document
        .modules
        .iter_mut()
        .find(|module| module.id == "source-module")
        .unwrap();
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
    document.hardware.as_mut().unwrap().pairs[0]
        .halves
        .as_mut()
        .unwrap()
        .correspondences
        .push(MirroredCorrespondence {
            companion_mode: None,
            companion_tweak: None,
            kind: MirroredItemKind::Module,
            source_id: "source-upstream".into(),
            mirror_id: "mirror-upstream".into(),
        });

    reconcile(&mut document, "pair-source-mirror", &projection).unwrap();
    let mirror = document
        .modules
        .iter()
        .find(|module| module.id == "mirror-module")
        .unwrap();
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
    let (mut document, projection) = hosted_electrical_fixture().unwrap();
    let mut source_connector = document
        .parts
        .iter()
        .find(|part| part.id == "matrix/source-matrix/r0c0")
        .unwrap()
        .clone();
    source_connector.id = "source-unpaired-connector".into();
    document.parts.push(source_connector);
    document
        .boards
        .iter_mut()
        .find(|board| board.id == "source-board")
        .unwrap()
        .part_ids
        .push("source-unpaired-connector".into());
    document
        .modules
        .iter_mut()
        .find(|module| module.id == "source-module")
        .unwrap()
        .connection
        .as_mut()
        .unwrap()
        .host_connector_part_id = "source-unpaired-connector".into();
    let before = document.clone();
    assert!(reconcile(&mut document, "pair-source-mirror", &projection).is_err());
    assert_eq!(document, before);
}

#[test]
fn hosted_circuit_definition_payload_changes_project_to_the_existing_definition() {
    let (mut document, projection) = hosted_electrical_fixture().unwrap();
    let source_definition = document
        .definitions
        .iter_mut()
        .find(|definition| definition.id == "embedded/source-circuit/definition/u")
        .unwrap();
    source_definition.name = "Updated Source U".into();
    source_definition.pads[0].size = Vec2 { x: 2.5, y: 3.5 };
    source_definition.pads[0].shape = PadShape::Roundrect;
    let changed = reconcile(&mut document, "pair-source-mirror", &projection).unwrap();
    let mirror = document
        .definitions
        .iter()
        .find(|definition| definition.id == "embedded/mirror-circuit/definition/u")
        .unwrap();
    assert_eq!(mirror.name, "Updated Source U");
    assert_eq!(mirror.pads[0].size, Vec2 { x: 2.5, y: 3.5 });
    assert!(changed.contains(&"embedded/mirror-circuit/definition/u".to_owned()));
}

#[test]
fn hosted_joined_net_update_preserves_target_only_host_pins() {
    let (mut document, projection) = hosted_electrical_fixture().unwrap();
    let mut host_part = document
        .parts
        .iter()
        .find(|part| part.id == "matrix/mirror-matrix/r0c1")
        .unwrap()
        .clone();
    host_part.id = "mirror-host-only".into();
    document.parts.push(host_part);
    document
        .boards
        .iter_mut()
        .find(|board| board.id == "mirror-board")
        .unwrap()
        .part_ids
        .push("mirror-host-only".into());
    let host_net = document
        .nets
        .iter_mut()
        .find(|net| net.id == "host-mirror-net")
        .unwrap();
    host_net.pins.push(Pin {
        part_id: "mirror-host-only".into(),
        pad_id: "authored-host-pad".into(),
    });
    let source_net = document
        .nets
        .iter_mut()
        .find(|net| net.id == "host-source-net")
        .unwrap();
    source_net.name = "Edited host net".into();
    source_net.pins.push(Pin {
        part_id: "embedded/source-circuit/U1".into(),
        pad_id: "pad-two".into(),
    });

    reconcile(&mut document, "pair-source-mirror", &projection).unwrap();
    let target = document
        .nets
        .iter()
        .find(|net| net.id == "host-mirror-net")
        .unwrap();
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
    let (mut document, projection) = hosted_electrical_fixture().unwrap();
    let halves = document.hardware.as_mut().unwrap().pairs[0]
        .halves
        .as_mut()
        .unwrap();
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
    let local_part = document
        .parts
        .iter_mut()
        .find(|part| part.id == "embedded/mirror-circuit/U1")
        .unwrap();
    local_part.reference = "LOCAL9".into();
    local_part
        .properties
        .as_mut()
        .unwrap()
        .insert("authoredLocal".into(), serde_json::json!({"v": 7}));
    document
        .nets
        .iter_mut()
        .find(|net| net.id == "embedded/mirror-circuit/N$1")
        .unwrap()
        .name = "Authored local net".into();
    document
        .parts
        .iter_mut()
        .find(|part| part.id == "embedded/source-circuit/U1")
        .unwrap()
        .reference = "U2".into();
    document
        .modules
        .iter_mut()
        .find(|module| module.id == "source-module")
        .unwrap()
        .at = Vec2 { x: 17.0, y: -8.0 };

    reconcile(&mut document, "pair-source-mirror", &projection).unwrap();
    let member = document
        .parts
        .iter()
        .find(|part| part.id == "embedded/mirror-circuit/U1")
        .unwrap();
    assert_eq!(member.reference, "LOCAL9");
    assert_eq!(
        member.properties.as_ref().unwrap()["authoredLocal"],
        serde_json::json!({"v": 7})
    );
    assert_eq!(
        document
            .nets
            .iter()
            .find(|net| net.id == "embedded/mirror-circuit/N$1")
            .unwrap()
            .name,
        "Authored local net"
    );
    let mirror_circuit = document
        .embedded_circuits
        .iter()
        .find(|circuit| circuit.id == "mirror-circuit")
        .unwrap();
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
    assert!(
        (document
            .modules
            .iter()
            .find(|module| module.id == "mirror-module")
            .unwrap()
            .at
            .x
            - 108.0)
            .abs()
            < 0.02
    );
}

#[test]
fn hosted_associated_local_circuit_preserves_owner_and_owned_payload() {
    let (mut document, projection) = hosted_electrical_fixture().unwrap();
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
    let target_circuit = document
        .embedded_circuits
        .iter_mut()
        .find(|circuit| circuit.id == "mirror-circuit")
        .unwrap();
    target_circuit
        .ports
        .insert("local-authored".into(), "host-mirror-net".into());
    target_circuit.net_ids.reverse();
    target_circuit
        .part_ids
        .push("embedded/mirror-circuit/local-extra".into());
    let mut local_extra = document
        .parts
        .iter()
        .find(|part| part.id == "embedded/mirror-circuit/U1")
        .unwrap()
        .clone();
    local_extra.id = "embedded/mirror-circuit/local-extra".into();
    local_extra.definition_id = "embedded/mirror-circuit/definition/local-extra".into();
    local_extra.reference = "LOCAL-EXTRA".into();
    document.parts.push(local_extra);
    let mut local_definition = document
        .definitions
        .iter()
        .find(|definition| definition.id == "embedded/mirror-circuit/definition/u")
        .unwrap()
        .clone();
    local_definition.id = "embedded/mirror-circuit/definition/local-extra".into();
    local_definition.name = "Local extra definition".into();
    document.definitions.push(local_definition);
    document
        .boards
        .iter_mut()
        .find(|board| board.id == "mirror-board")
        .unwrap()
        .part_ids
        .push("embedded/mirror-circuit/local-extra".into());
    document
        .nets
        .iter_mut()
        .find(|net| net.id == "embedded/mirror-circuit/N$1")
        .unwrap()
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
    let target_member = document
        .parts
        .iter_mut()
        .find(|part| part.id == "embedded/mirror-circuit/U1")
        .unwrap();
    target_member.reference = "LOCAL-U".into();
    target_member.pose.at = Vec2 { x: 31.0, y: 42.0 };
    target_member
        .properties
        .as_mut()
        .unwrap()
        .insert("authoredLocal".into(), serde_json::json!({"v": 7}));
    document
        .definitions
        .iter_mut()
        .find(|definition| definition.id == "embedded/mirror-circuit/definition/u")
        .unwrap()
        .name = "Local U definition".into();
    document
        .nets
        .iter_mut()
        .find(|net| net.id == "embedded/mirror-circuit/N$1")
        .unwrap()
        .name = "Local private net".into();
    document
        .modules
        .iter_mut()
        .find(|module| module.id == "source-module")
        .unwrap()
        .at = Vec2 { x: 17.0, y: -8.0 };
    let source_joined_net = document
        .nets
        .iter_mut()
        .find(|net| net.id == "host-source-net")
        .unwrap();
    source_joined_net.name = "HOST SOURCE EDITED".into();
    source_joined_net.pins.push(Pin {
        part_id: "matrix/source-matrix/r0c0".into(),
        pad_id: "pad-one".into(),
    });
    let pair_before = pair_state_bytes(&document);

    reconcile(&mut document, "pair-source-mirror", &projection).unwrap();

    let target_circuit = document
        .embedded_circuits
        .iter()
        .find(|circuit| circuit.id == "mirror-circuit")
        .unwrap();
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
    let target_member = document
        .parts
        .iter()
        .find(|part| part.id == "embedded/mirror-circuit/U1")
        .unwrap();
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
            .iter()
            .find(|definition| definition.id == "embedded/mirror-circuit/definition/u")
            .unwrap()
            .name,
        "Local U definition"
    );
    let private_net = document
        .nets
        .iter()
        .find(|net| net.id == "embedded/mirror-circuit/N$1")
        .unwrap();
    assert_eq!(private_net.name, "Local private net");
    assert!(private_net.pins.contains(&Pin {
        part_id: "embedded/mirror-circuit/local-extra".into(),
        pad_id: "pad-1".into(),
    }));
    assert!(private_net.pins.contains(&Pin {
        part_id: "matrix/mirror-matrix/r0c0".into(),
        pad_id: "pad-one".into(),
    }));
    let joined_host_net = document
        .nets
        .iter()
        .find(|net| net.id == "host-mirror-net")
        .unwrap();
    assert_eq!(joined_host_net.name, "HOST SOURCE EDITED");
    assert!(joined_host_net.pins.contains(&Pin {
        part_id: "matrix/mirror-matrix/r0c0".into(),
        pad_id: "pad-one".into(),
    }));
    assert!(
        (document
            .modules
            .iter()
            .find(|module| module.id == "mirror-module")
            .unwrap()
            .at
            .x
            - 108.0)
            .abs()
            < 0.02
    );
    let stable = document.clone();
    reconcile(&mut document, "pair-source-mirror", &projection).unwrap();
    assert_eq!(document, stable);
}

#[test]
fn local_joined_net_exception_is_independent_of_local_circuit_owner() {
    let (mut document, projection) = hosted_electrical_fixture().unwrap();
    document.hardware.as_mut().unwrap().pairs[0]
        .halves
        .as_mut()
        .unwrap()
        .exceptions
        .extend([
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
    document
        .nets
        .iter_mut()
        .find(|net| net.id == "host-source-net")
        .unwrap()
        .name = "HOST SOURCE EDITED".into();

    reconcile(&mut document, "pair-source-mirror", &projection).unwrap();

    assert_eq!(
        document
            .nets
            .iter()
            .find(|net| net.id == "host-mirror-net")
            .unwrap()
            .name,
        "HOST MIRROR"
    );
}

#[test]
fn inherited_local_circuit_member_settings_and_host_net_pins_survive() {
    let (mut document, projection) = hosted_electrical_fixture().unwrap();
    document.hardware.as_mut().unwrap().pairs[0]
        .halves
        .as_mut()
        .unwrap()
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
        .iter_mut()
        .find(|definition| definition.id == "embedded/mirror-circuit/definition/u")
        .unwrap();
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
        .iter_mut()
        .find(|board| board.id == "source-board")
        .unwrap()
        .net_ids
        .push("source-plain-member-net".into());
    document
        .boards
        .iter_mut()
        .find(|board| board.id == "mirror-board")
        .unwrap()
        .net_ids
        .push("mirror-plain-member-net".into());
    document.hardware.as_mut().unwrap().pairs[0]
        .halves
        .as_mut()
        .unwrap()
        .correspondences
        .push(MirroredCorrespondence {
            companion_mode: None,
            companion_tweak: None,
            kind: MirroredItemKind::Net,
            source_id: "source-plain-member-net".into(),
            mirror_id: "mirror-plain-member-net".into(),
        });

    reconcile(&mut document, "pair-source-mirror", &projection)
        .expect("the Local circuit member remains valid through reconciliation");
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
        document
            .nets
            .iter()
            .find(|net| net.id == "mirror-plain-member-net")
            .unwrap()
            .pins,
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
        let (mut document, projection) = hosted_electrical_fixture().unwrap();
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
        let target = document
            .embedded_circuits
            .iter_mut()
            .find(|circuit| circuit.id == "mirror-circuit")
            .unwrap();
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
                    .iter_mut()
                    .find(|circuit| circuit.id == "source-circuit")
                    .unwrap()
                    .host_board_id = "mirror-board".into();
            }
            "missing-member" => target
                .part_ids
                .push("embedded/mirror-circuit/missing".into()),
            "foreign-member" => target.part_ids.push("embedded/other-circuit/U2".into()),
            "missing-definition" => {
                document
                    .parts
                    .iter_mut()
                    .find(|part| part.id == "embedded/mirror-circuit/U1")
                    .unwrap()
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
                document
                    .nets
                    .iter_mut()
                    .find(|net| net.id == "embedded/mirror-circuit/N$1")
                    .unwrap()
                    .pins[0]
                    .part_id = "embedded/other-circuit/U1".into()
            }
            _ => unreachable!(),
        }
        let before = serde_json::to_vec(&document).unwrap();

        let error = reconcile(&mut document, "pair-source-mirror", &projection)
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
    let (mut document, projection) = hosted_electrical_fixture().unwrap();
    let halves = document.hardware.as_mut().unwrap().pairs[0]
        .halves
        .as_mut()
        .unwrap();
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
        .iter_mut()
        .find(|circuit| circuit.id == "mirror-circuit")
        .unwrap()
        .ports
        .insert("local-authored".into(), "host-mirror-net".into());
    document
        .modules
        .iter_mut()
        .find(|module| module.id == "source-module")
        .unwrap()
        .at = Vec2 { x: 17.0, y: -8.0 };
    let pair_before = pair_state_bytes(&document);

    reconcile(&mut document, "pair-source-mirror", &projection).unwrap();
    let circuit = document
        .embedded_circuits
        .iter()
        .find(|circuit| circuit.id == "mirror-circuit")
        .unwrap();
    assert_eq!(
        circuit.ports.get("local-authored").map(String::as_str),
        Some("host-mirror-net")
    );
    assert!(
        (document
            .modules
            .iter()
            .find(|module| module.id == "mirror-module")
            .unwrap()
            .at
            .x
            - 108.0)
            .abs()
            < 0.02
    );
    assert_eq!(pair_state_bytes(&document), pair_before);
}

#[test]
fn hosted_omitted_member_and_net_keep_their_unlinked_payload_under_linked_circuit() {
    let (mut document, projection) = hosted_electrical_fixture().unwrap();
    let halves = document.hardware.as_mut().unwrap().pairs[0]
        .halves
        .as_mut()
        .unwrap();
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
        .iter_mut()
        .find(|part| part.id == "embedded/mirror-circuit/U1")
        .unwrap()
        .reference = "OMITTED-U".into();
    document
        .nets
        .iter_mut()
        .find(|net| net.id == "embedded/mirror-circuit/N$1")
        .unwrap()
        .name = "Omitted private net".into();
    document
        .parts
        .iter_mut()
        .find(|part| part.id == "embedded/source-circuit/U1")
        .unwrap()
        .reference = "U2".into();
    document
        .modules
        .iter_mut()
        .find(|module| module.id == "source-module")
        .unwrap()
        .at = Vec2 { x: 17.0, y: -8.0 };
    let pair_before = pair_state_bytes(&document);

    reconcile(&mut document, "pair-source-mirror", &projection).unwrap();
    assert_eq!(
        document
            .parts
            .iter()
            .find(|part| part.id == "embedded/mirror-circuit/U1")
            .unwrap()
            .reference,
        "OMITTED-U"
    );
    assert_eq!(
        document
            .nets
            .iter()
            .find(|net| net.id == "embedded/mirror-circuit/N$1")
            .unwrap()
            .name,
        "Omitted private net"
    );
    assert!(
        document
            .embedded_circuits
            .iter()
            .any(|circuit| circuit.id == "mirror-circuit")
    );
    let mirror_circuit = document
        .embedded_circuits
        .iter()
        .find(|circuit| circuit.id == "mirror-circuit")
        .unwrap();
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
    assert!(
        (document
            .modules
            .iter()
            .find(|module| module.id == "mirror-module")
            .unwrap()
            .at
            .x
            - 108.0)
            .abs()
            < 0.02
    );
    assert_eq!(pair_state_bytes(&document), pair_before);
}

#[test]
fn hosted_omitted_member_and_net_can_be_absent_under_linked_circuit() {
    let (mut document, projection) = hosted_electrical_fixture().unwrap();
    reconcile(&mut document, "pair-source-mirror", &projection)
        .expect("the historical linked owner baseline is fully materialized first");
    let halves = document.hardware.as_mut().unwrap().pairs[0]
        .halves
        .as_mut()
        .unwrap();
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
    let mirror_circuit = document
        .embedded_circuits
        .iter_mut()
        .find(|circuit| circuit.id == "mirror-circuit")
        .unwrap();
    mirror_circuit
        .part_ids
        .retain(|id| id != "embedded/mirror-circuit/U1");
    mirror_circuit
        .net_ids
        .retain(|id| id != "embedded/mirror-circuit/N$1");
    let mirror_board = document
        .boards
        .iter_mut()
        .find(|board| board.id == "mirror-board")
        .unwrap();
    mirror_board
        .part_ids
        .retain(|id| id != "embedded/mirror-circuit/U1");
    mirror_board
        .net_ids
        .retain(|id| id != "embedded/mirror-circuit/N$1");
    document
        .nets
        .iter_mut()
        .find(|net| net.id == "host-mirror-net")
        .unwrap()
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
    document
        .modules
        .iter_mut()
        .find(|module| module.id == "source-module")
        .unwrap()
        .at = Vec2 { x: 17.0, y: -8.0 };

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
    assert!(
        (document
            .modules
            .iter()
            .find(|module| module.id == "mirror-module")
            .unwrap()
            .at
            .x
            - 108.0)
            .abs()
            < 0.02
    );
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
    let source_copper_before = document
        .boards
        .iter()
        .find(|board| board.id == "source-board")
        .unwrap()
        .traces
        .clone();
    document
        .modules
        .iter_mut()
        .find(|module| module.id == "source-module")
        .unwrap()
        .at = Vec2 { x: 17.0, y: -8.0 };
    document
        .modules
        .iter_mut()
        .find(|module| module.id == "source-module")
        .unwrap()
        .rotation = 37.0;
    let source_member = document
        .parts
        .iter_mut()
        .find(|part| part.id == "embedded/source-circuit/U1")
        .unwrap();
    source_member.pose = Pose2 {
        at: Vec2 { x: 11.0, y: 13.0 },
        rotation: 23.0,
    };
    source_member
        .properties
        .as_mut()
        .unwrap()
        .insert("edited".into(), serde_json::json!({"revision": 2}));
    document
        .nets
        .iter_mut()
        .find(|net| net.id == "embedded/source-circuit/N$1")
        .unwrap()
        .name = "PRIVATE EDITED".into();
    document
        .nets
        .iter_mut()
        .find(|net| net.id == "embedded/source-circuit/N$1")
        .unwrap()
        .pins
        .push(Pin {
            part_id: "matrix/source-matrix/r0c0".into(),
            pad_id: "pad-two".into(),
        });
    document
        .embedded_circuits
        .iter_mut()
        .find(|circuit| circuit.id == "source-circuit")
        .unwrap()
        .ports
        .insert("signal".into(), "embedded/source-circuit/N$1".into());
    let pair_before = pair_state_bytes(&document);
    let changed = reconcile(&mut document, "pair-source-mirror", &projection).unwrap();
    assert_eq!(pair_state_bytes(&document), pair_before);

    let mirror_module = document
        .modules
        .iter()
        .find(|module| module.id == "mirror-module")
        .unwrap();
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
    let mirror_member = document
        .parts
        .iter()
        .find(|part| part.id == "embedded/mirror-circuit/U1")
        .unwrap();
    assert!((mirror_member.pose.at.x - 87.0).abs() < 0.02);
    assert!((mirror_member.pose.at.y + 41.0).abs() < 0.02);
    assert_eq!(mirror_member.pose.rotation, 247.0);
    assert_eq!(
        mirror_member.properties.as_ref().unwrap().get("edited"),
        Some(&serde_json::json!({"revision": 2}))
    );
    let mirror_private_net = document
        .nets
        .iter()
        .find(|net| net.id == "embedded/mirror-circuit/N$1")
        .unwrap();
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
        document
            .embedded_circuits
            .iter()
            .find(|circuit| circuit.id == "mirror-circuit")
            .unwrap()
            .ports["signal"],
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
        document
            .boards
            .iter()
            .find(|board| board.id == "source-board")
            .unwrap()
            .traces,
        source_copper_before
    );
    assert!(
        !document
            .boards
            .iter()
            .find(|board| board.id == "mirror-board")
            .unwrap()
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
            .iter()
            .find(|board| board.id == "mirror-board")
            .unwrap()
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
        .iter_mut()
        .find(|board| board.id == "mirror-board")
        .unwrap()
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

    let mirror_board = document
        .boards
        .iter()
        .find(|board| board.id == "mirror-board")
        .unwrap();
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
        .iter_mut()
        .find(|part| part.id == "embedded/mirror-circuit/U1")
        .unwrap()
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
    let mut unrelated_source = document
        .modules
        .iter()
        .find(|module| module.id == "source-module")
        .unwrap()
        .clone();
    unrelated_source.id = "unpaired-source-module".into();
    let mut unrelated_target = document
        .modules
        .iter()
        .find(|module| module.id == "mirror-module")
        .unwrap()
        .clone();
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
    let halves = document.hardware.as_mut().unwrap().pairs[0]
        .halves
        .as_mut()
        .unwrap();
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
    let mirror_module = document
        .modules
        .iter_mut()
        .find(|module| module.id == "mirror-module")
        .unwrap();
    mirror_module.at = local_module_pose.at;
    mirror_module.rotation = local_module_pose.rotation;
    document
        .modules
        .iter_mut()
        .find(|module| module.id == "source-module")
        .unwrap()
        .at = Vec2 { x: 21.0, y: 15.0 };
    document
        .parts
        .iter_mut()
        .find(|part| part.id == "embedded/source-circuit/U1")
        .unwrap()
        .pose
        .at = Vec2 { x: 14.0, y: 12.0 };
    let pair_before = pair_state_bytes(&document);

    reconcile(&mut document, "pair-source-mirror", &projection)
        .expect("a Local module is retained while other items reconcile");
    assert_eq!(pair_state_bytes(&document), pair_before);

    let mirror_module = document
        .modules
        .iter()
        .find(|module| module.id == "mirror-module")
        .unwrap();
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
    let mirror_member = document
        .parts
        .iter()
        .find(|part| part.id == "embedded/mirror-circuit/U1")
        .unwrap();
    assert!((mirror_member.pose.at.x - 88.0).abs() < 0.02);
    assert!((mirror_member.pose.at.y + 44.0).abs() < 0.02);
}

#[test]
fn omitted_embedded_circuit_exception_stays_durable_and_unprojected() {
    let (mut document, projection) = hosted_electrical_fixture().unwrap();
    let halves = document.hardware.as_mut().unwrap().pairs[0]
        .halves
        .as_mut()
        .unwrap();
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
        .iter_mut()
        .find(|part| part.id == "embedded/source-circuit/U1")
        .unwrap()
        .pose
        .at = Vec2 { x: 14.0, y: 12.0 };
    let mirror_circuit_before = document
        .embedded_circuits
        .iter()
        .find(|circuit| circuit.id == "mirror-circuit")
        .unwrap()
        .clone();
    let mirror_member_before = document
        .parts
        .iter()
        .find(|part| part.id == "embedded/mirror-circuit/U1")
        .unwrap()
        .clone();
    let mirror_net_before = document
        .nets
        .iter()
        .find(|net| net.id == "embedded/mirror-circuit/N$1")
        .unwrap()
        .clone();
    let pair_before = pair_state_bytes(&document);

    reconcile(&mut document, "pair-source-mirror", &projection)
        .expect("an unlinked Omitted circuit remains under exception ownership");
    assert_eq!(pair_state_bytes(&document), pair_before);

    assert_eq!(
        document
            .embedded_circuits
            .iter()
            .find(|circuit| circuit.id == "mirror-circuit")
            .unwrap(),
        &mirror_circuit_before
    );
    assert_eq!(
        document
            .parts
            .iter()
            .find(|part| part.id == "embedded/mirror-circuit/U1")
            .unwrap(),
        &mirror_member_before
    );
    assert_eq!(
        document
            .nets
            .iter()
            .find(|net| net.id == "embedded/mirror-circuit/N$1")
            .unwrap(),
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
        let part = document.parts.iter().find(|part| part.id == id).unwrap();
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
            let matrix = document
                .matrices
                .iter()
                .find(|matrix| matrix.id == id)
                .unwrap();
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
            let matrix = document
                .matrices
                .iter()
                .find(|matrix| matrix.id == matrix_id)
                .unwrap();
            assert_eq!(matrix.part_ids.len(), 14);
        }
        assert_eq!(
            document.hardware.as_ref().unwrap().pairs[0]
                .halves
                .as_ref()
                .unwrap()
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
        let source_ids = entry
            .matrices
            .iter()
            .find(|matrix| matrix.id == "source-matrix")
            .unwrap()
            .part_ids
            .clone();
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
            .map(|id| {
                entry
                    .parts
                    .iter()
                    .find(|part| part.id == *id)
                    .unwrap()
                    .clone()
            })
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
        let correspondences_before = entry.hardware.as_ref().unwrap().pairs[0]
            .halves
            .as_ref()
            .unwrap()
            .correspondences
            .clone();

        let changed = reconcile(&mut document, "pair-source-mirror", &projection).unwrap();
        assert_eq!(changed, expected_changed);
        assert_eq!(
            document.hardware.as_ref().unwrap().pairs[0]
                .halves
                .as_ref()
                .unwrap()
                .correspondences,
            correspondences_before
        );
        for (id, expected) in expected_members {
            assert_pose(&document, id, *expected);
        }
        let final_matrix = document
            .matrices
            .iter()
            .find(|matrix| matrix.id == "mirror-matrix")
            .unwrap();
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
            assert_eq!(
                document.parts.iter().find(|part| part.id == *id).unwrap(),
                expected
            );
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
        let rematerialized = disposable
            .matrices
            .iter()
            .find(|matrix| matrix.id == "mirror-matrix")
            .unwrap();
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

#[test]
fn fixed_membership_view_preserves_consistent_selected_cell_tombstones() {
    let (mut document, mut projection) = fixed_geometry_fixture().unwrap();
    let source_key = "matrix/source-matrix/r0c1";
    let mirror_key = "matrix/mirror-matrix/r0c1";
    for (matrix_id, key_id) in [("source-matrix", source_key), ("mirror-matrix", mirror_key)] {
        let matrix = document
            .matrices
            .iter_mut()
            .find(|matrix| matrix.id == matrix_id)
            .unwrap();
        let cell = matrix
            .cells
            .iter_mut()
            .find(|cell| (cell.row, cell.column) == (0, 1))
            .unwrap();
        cell.enabled = false;
        cell.deleted = true;
        cell.assemblies.clear();
        matrix.part_ids.retain(|id| id != key_id);
        document.parts.retain(|part| part.id != key_id);
        for board in &mut document.boards {
            board.part_ids.retain(|id| id != key_id);
        }
    }
    let halves = document.hardware.as_mut().unwrap().pairs[0]
        .halves
        .as_mut()
        .unwrap();
    halves.correspondences.retain(|association| {
        !(association.kind == MirroredItemKind::Part
            && association.source_id == source_key
            && association.mirror_id == mirror_key)
    });
    projection.parts.retain(|part| part.source_id != source_key);
    projection.final_owners.remove(source_key);
    projection.final_owners.remove(mirror_key);

    let halves = document.hardware.as_ref().unwrap().pairs[0]
        .halves
        .as_ref()
        .unwrap();
    let source = document
        .matrices
        .iter()
        .find(|matrix| matrix.id == "source-matrix")
        .unwrap();
    let mirror = document
        .matrices
        .iter()
        .find(|matrix| matrix.id == "mirror-matrix")
        .unwrap();
    let matrix_association = halves
        .correspondences
        .iter()
        .find(|association| association.kind == MirroredItemKind::Matrix)
        .unwrap();
    let projected = boardstudio_core_mirrored_half::mirrored_half::reconcile::project_matrix(
        source,
        mirror,
        matrix_association,
        projection.mapping,
    )
    .unwrap();
    let view =
        boardstudio_core_mirrored_half::mirrored_half::reconcile::build_fixed_membership_view(
            &document,
            halves,
            source,
            mirror,
            &projected,
            &projection,
        )
        .expect("matching explicit selected-cell tombstones are valid inactive coordinates");

    for matrix in [&view.normalized_source_matrix, &view.incoming_matrix] {
        let tombstone = matrix
            .cells
            .iter()
            .find(|cell| (cell.row, cell.column) == (0, 1))
            .unwrap();
        assert!(tombstone.deleted);
        assert!(!tombstone.enabled);
    }

    let source = document
        .matrices
        .iter()
        .find(|matrix| matrix.id == "source-matrix")
        .unwrap()
        .clone();
    let mut edited_source = source;
    edited_source
        .cells
        .iter_mut()
        .find(|cell| (cell.row, cell.column) == (1, 0))
        .unwrap()
        .offset = Some(Vec2 { x: 4.0, y: -1.0 });
    boardstudio_core_kernel::matrix::set_matrix(&mut document, &edited_source)
        .expect("the later source edit preserves its existing active membership");
    let expected_source = document
        .matrices
        .iter()
        .find(|matrix| matrix.id == "source-matrix")
        .unwrap()
        .clone();
    boardstudio_core_mirrored_half::mirrored_half::reconcile::reconcile(
        &mut document,
        "pair-source-mirror",
        &projection,
    )
    .expect("ordinary reconciliation keeps the selected slot tombstoned");
    let mirror = document
        .matrices
        .iter()
        .find(|matrix| matrix.id == "mirror-matrix")
        .unwrap();
    let tombstone = mirror
        .cells
        .iter()
        .find(|cell| (cell.row, cell.column) == (0, 1))
        .unwrap();
    assert!(tombstone.deleted);
    assert!(!tombstone.enabled);
    assert!(!mirror.part_ids.contains(&mirror_key.into()));
    assert!(!document.parts.iter().any(|part| part.id == mirror_key));
    assert_eq!(
        document
            .matrices
            .iter()
            .find(|matrix| matrix.id == "source-matrix")
            .unwrap(),
        &expected_source
    );
    let after_first = document.clone();
    assert!(
        boardstudio_core_mirrored_half::mirrored_half::reconcile::reconcile(
            &mut document,
            "pair-source-mirror",
            &projection
        )
        .unwrap()
        .is_empty()
    );
    assert_eq!(document, after_first);
}

#[test]
fn selected_tombstone_survives_prepared_source_and_mirror_edits() {
    fn tombstone(document: &ProjectDoc, matrix_id: &str) -> boardstudio_model::model::MatrixCell {
        document
            .matrices
            .iter()
            .find(|matrix| matrix.id == matrix_id)
            .unwrap()
            .cells
            .iter()
            .find(|cell| (cell.row, cell.column) == (0, 0))
            .unwrap()
            .clone()
    }

    fn assert_tombstone(document: &ProjectDoc, matrix_id: &str) {
        let matrix = document
            .matrices
            .iter()
            .find(|matrix| matrix.id == matrix_id)
            .unwrap();
        let cell = tombstone(document, matrix_id);
        assert!(!cell.enabled && cell.deleted);
        let key_id = format!("matrix/{matrix_id}/r0c0");
        assert!(!matrix.part_ids.iter().any(|id| id == &key_id));
        assert!(
            !document
                .parts
                .iter()
                .any(|part| { part.id == key_id || part.id.starts_with(&format!("{key_id}/")) })
        );
        assert!(!document.boards.iter().any(|board| {
            board
                .part_ids
                .iter()
                .any(|id| id == &key_id || id.starts_with(&format!("{key_id}/")))
        }));
        let halves = document.hardware.as_ref().unwrap().pairs[0]
            .halves
            .as_ref()
            .unwrap();
        assert!(!halves.correspondences.iter().any(|association| {
            association.source_id == key_id
                || association.mirror_id == key_id
                || association.source_id.starts_with(&format!("{key_id}/"))
                || association.mirror_id.starts_with(&format!("{key_id}/"))
        }));
    }

    let accepted = fixture_document().unwrap();
    let (removed, _) = crate::support::prepare_transaction(
        &accepted,
        &[],
        &boardstudio_model::model::EditOperation::RemoveSelectedMatrixCells {
            cells: BTreeMap::from([("source-matrix".into(), vec![(0, 0)])]),
        },
    )
    .expect("selected-cell removal stores a valid matrix tombstone");
    let removed_tombstones = [
        tombstone(&removed, "source-matrix"),
        tombstone(&removed, "mirror-matrix"),
    ];
    assert_tombstone(&removed, "source-matrix");
    assert_tombstone(&removed, "mirror-matrix");

    let source_move = boardstudio_model::model::EditOperation::MoveParts {
        positions: vec![boardstudio_model::model::Position {
            id: "matrix/source-matrix/r0c1".into(),
            at: Vec2 { x: 43.0, y: 17.0 },
        }],
    };
    let (source_edited, _) = crate::support::prepare_transaction(&removed, &[], &source_move)
        .expect("later source edits reconcile around inactive membership");
    assert_tombstone(&source_edited, "source-matrix");
    assert_tombstone(&source_edited, "mirror-matrix");
    assert_eq!(
        tombstone(&source_edited, "source-matrix"),
        removed_tombstones[0]
    );
    assert_eq!(
        tombstone(&source_edited, "mirror-matrix"),
        removed_tombstones[1]
    );

    let mirror_move = boardstudio_model::model::EditOperation::MoveParts {
        positions: vec![boardstudio_model::model::Position {
            id: "matrix/mirror-matrix/r0c1".into(),
            at: Vec2 { x: 71.0, y: 83.0 },
        }],
    };
    let (mirror_edited, _) = crate::support::prepare_transaction(&source_edited, &[], &mirror_move)
        .expect("later mirror edits preserve inactive membership");
    assert_tombstone(&mirror_edited, "source-matrix");
    assert_tombstone(&mirror_edited, "mirror-matrix");
    assert_eq!(
        tombstone(&mirror_edited, "source-matrix"),
        removed_tombstones[0]
    );
    assert_eq!(
        tombstone(&mirror_edited, "mirror-matrix"),
        removed_tombstones[1]
    );
    let (repeated, repeated_changes) =
        crate::support::prepare_transaction(&mirror_edited, &[], &mirror_move)
            .expect("repeated mirror edit remains prepared");
    assert!(repeated_changes.is_empty());
    assert_eq!(repeated, mirror_edited);

    let (all_inactive, _) = crate::support::prepare_transaction(
        &removed,
        &[],
        &boardstudio_model::model::EditOperation::RemoveSelectedMatrixCells {
            cells: BTreeMap::from([("source-matrix".into(), vec![(0, 1)])]),
        },
    )
    .expect("all-slots-inactive paired matrices remain valid");
    for matrix_id in ["source-matrix", "mirror-matrix"] {
        let matrix = all_inactive
            .matrices
            .iter()
            .find(|matrix| matrix.id == matrix_id)
            .unwrap();
        assert_eq!(matrix.part_ids, Vec::<String>::new());
        assert!(
            matrix
                .cells
                .iter()
                .all(|cell| !cell.enabled && cell.deleted)
        );
    }
}

#[test]
fn fixed_membership_view_rejects_tombstone_mismatches_and_stale_members() {
    fn tombstone_slot(document: &mut ProjectDoc, matrix_id: &str) {
        let key_id = format!("matrix/{matrix_id}/r0c1");
        let matrix = document
            .matrices
            .iter_mut()
            .find(|matrix| matrix.id == matrix_id)
            .unwrap();
        let cell = matrix
            .cells
            .iter_mut()
            .find(|cell| (cell.row, cell.column) == (0, 1))
            .unwrap();
        cell.enabled = false;
        cell.deleted = true;
        cell.assemblies.clear();
        matrix.part_ids.retain(|id| id != &key_id);
    }

    fn fixed_view_error(document: &ProjectDoc, projection: &Projection) -> String {
        let halves = document.hardware.as_ref().unwrap().pairs[0]
            .halves
            .as_ref()
            .unwrap();
        let source = document
            .matrices
            .iter()
            .find(|matrix| matrix.id == "source-matrix")
            .unwrap();
        let mirror = document
            .matrices
            .iter()
            .find(|matrix| matrix.id == "mirror-matrix")
            .unwrap();
        let association = halves
            .correspondences
            .iter()
            .find(|association| association.kind == MirroredItemKind::Matrix)
            .unwrap();
        let projected = boardstudio_core_mirrored_half::mirrored_half::reconcile::project_matrix(
            source,
            mirror,
            association,
            projection.mapping,
        )
        .unwrap();
        match boardstudio_core_mirrored_half::mirrored_half::reconcile::build_fixed_membership_view(
            document, halves, source, mirror, &projected, projection,
        ) {
            Err(error) => error,
            Ok(_) => panic!("expected malformed fixed membership to be refused"),
        }
    }

    let (mut document, projection) = fixed_geometry_fixture().unwrap();
    tombstone_slot(&mut document, "source-matrix");
    assert!(fixed_view_error(&document, &projection).contains("inconsistent inactive slots"));

    let (mut document, projection) = fixed_geometry_fixture().unwrap();
    tombstone_slot(&mut document, "mirror-matrix");
    assert!(fixed_view_error(&document, &projection).contains("inconsistent inactive slots"));

    fn clean_tombstone_fixture() -> (ProjectDoc, Projection) {
        let (mut document, mut projection) = fixed_geometry_fixture().unwrap();
        tombstone_slot(&mut document, "source-matrix");
        tombstone_slot(&mut document, "mirror-matrix");
        let source_id = "matrix/source-matrix/r0c1";
        let mirror_id = "matrix/mirror-matrix/r0c1";
        document
            .parts
            .retain(|part| part.id != source_id && part.id != mirror_id);
        for board in &mut document.boards {
            board
                .part_ids
                .retain(|id| id != source_id && id != mirror_id);
        }
        document.hardware.as_mut().unwrap().pairs[0]
            .halves
            .as_mut()
            .unwrap()
            .correspondences
            .retain(|association| {
                !(association.kind == MirroredItemKind::Part
                    && association.source_id == source_id
                    && association.mirror_id == mirror_id)
            });
        projection.parts.retain(|part| part.source_id != source_id);
        projection.final_owners.remove(source_id);
        projection.final_owners.remove(mirror_id);
        (document, projection)
    }

    for stale_kind in ["part", "board owner", "owner input", "correspondence"] {
        let (mut document, mut projection) = clean_tombstone_fixture();
        let source_id = "matrix/source-matrix/r0c1";
        let mirror_id = "matrix/mirror-matrix/r0c1";
        match stale_kind {
            "part" => {
                let mut part = document
                    .parts
                    .iter()
                    .find(|part| part.id == "matrix/mirror-matrix/r0c2")
                    .unwrap()
                    .clone();
                part.id = mirror_id.into();
                document.parts.push(part);
            }
            "board owner" => document
                .boards
                .iter_mut()
                .find(|board| board.id == "mirror-board")
                .unwrap()
                .part_ids
                .push(mirror_id.into()),
            "owner input" => {
                projection
                    .final_owners
                    .insert(mirror_id.into(), "mirror-board".into());
            }
            "correspondence" => document.hardware.as_mut().unwrap().pairs[0]
                .halves
                .as_mut()
                .unwrap()
                .correspondences
                .push(MirroredCorrespondence {
                    companion_mode: None,
                    companion_tweak: None,
                    kind: MirroredItemKind::Part,
                    source_id: source_id.into(),
                    mirror_id: mirror_id.into(),
                }),
            _ => unreachable!(),
        }
        assert!(
            fixed_view_error(&document, &projection).contains("materialized or associated members"),
            "stale {stale_kind} was not refused"
        );
    }

    let (mut document, projection) = fixed_geometry_fixture().unwrap();
    let source_cell = document
        .matrices
        .iter_mut()
        .find(|matrix| matrix.id == "source-matrix")
        .unwrap()
        .cells
        .iter_mut()
        .find(|cell| (cell.row, cell.column) == (0, 1))
        .unwrap();
    source_cell.enabled = false;
    source_cell.deleted = false;
    assert!(fixed_view_error(&document, &projection).contains("fixed active membership"));
}

#[test]
fn fixed_membership_view_accepts_unassociated_local_or_omitted_matrix_children() {
    for state in [
        MirroredExceptionState::Local,
        MirroredExceptionState::Omitted,
    ] {
        let (document, projection) = fixed_membership_assembly_fixture(90.0).unwrap();
        let source_id = "matrix/source-matrix/r0c1/diode";
        let mirror_id = "matrix/mirror-matrix/r0c1/diode";
        let mut document = document;
        let halves = document.hardware.as_mut().unwrap().pairs[0]
            .halves
            .as_mut()
            .unwrap();
        halves.correspondences.retain(|association| {
            !(association.kind == MirroredItemKind::Part
                && association.source_id == source_id
                && association.mirror_id == mirror_id)
        });
        halves.exceptions.push(MirroredException {
            companion_mode: Some(MirroredCompanionMode::LiteralMirrorWithTweak),
            companion_tweak: Some(MirroredCompanionTweak {
                x: 1.0,
                y: -2.0,
                rotation: 15.0,
            }),
            kind: MirroredItemKind::Part,
            mirror_id: mirror_id.into(),
            state,
            removed_copper: None,
        });
        boardstudio_core_mechanical::mechanical::halves::validate_document(&document)
            .expect("the unassociated exception names an existing matrix child");

        let halves = document.hardware.as_ref().unwrap().pairs[0]
            .halves
            .as_ref()
            .unwrap();
        let source = document
            .matrices
            .iter()
            .find(|matrix| matrix.id == "source-matrix")
            .unwrap();
        let mirror = document
            .matrices
            .iter()
            .find(|matrix| matrix.id == "mirror-matrix")
            .unwrap();
        let matrix_association = halves
            .correspondences
            .iter()
            .find(|association| association.kind == MirroredItemKind::Matrix)
            .unwrap();
        let projected = boardstudio_core_mirrored_half::mirrored_half::reconcile::project_matrix(
            source,
            mirror,
            matrix_association,
            projection.mapping,
        )
        .unwrap();
        let view =
            boardstudio_core_mirrored_half::mirrored_half::reconcile::build_fixed_membership_view(
                &document,
                halves,
                source,
                mirror,
                &projected,
                &projection,
            )
            .expect("the exact matrix slot authenticates its unassociated child exception");
        let incoming_cell = view
            .incoming_matrix
            .cells
            .iter()
            .find(|cell| (cell.row, cell.column) == (0, 1))
            .unwrap();
        assert!(
            incoming_cell
                .assemblies
                .iter()
                .any(|assembly| assembly.id == "diode")
        );
        let legacy_part = document
            .parts
            .iter()
            .find(|part| part.id == mirror_id)
            .unwrap()
            .clone();
        let pair_before =
            serde_json::to_vec(&document.hardware.as_ref().unwrap().pairs[0]).unwrap();
        boardstudio_core_mirrored_half::mirrored_half::reconcile::reconcile(
            &mut document,
            "pair-source-mirror",
            &projection,
        )
        .unwrap_or_else(|error| panic!("{state:?} legacy exception: {error}"));
        assert_eq!(
            document.parts.iter().find(|part| part.id == mirror_id),
            Some(&legacy_part)
        );
        let exception = document.hardware.as_ref().unwrap().pairs[0]
            .halves
            .as_ref()
            .unwrap()
            .exceptions
            .iter()
            .find(|exception| exception.mirror_id == mirror_id)
            .unwrap();
        assert_eq!(exception.state, state);
        assert_eq!(
            exception.companion_mode,
            Some(MirroredCompanionMode::LiteralMirrorWithTweak)
        );
        assert_eq!(
            exception.companion_tweak,
            Some(MirroredCompanionTweak {
                x: 1.0,
                y: -2.0,
                rotation: 15.0,
            })
        );
        let pair_after = serde_json::to_vec(&document.hardware.as_ref().unwrap().pairs[0]).unwrap();
        assert_eq!(pair_after, pair_before);
        let first = document.clone();
        assert!(
            boardstudio_core_mirrored_half::mirrored_half::reconcile::reconcile(
                &mut document,
                "pair-source-mirror",
                &projection
            )
            .unwrap()
            .is_empty()
        );
        assert_eq!(document, first);
    }
}

#[test]
fn fixed_membership_view_only_allows_exactly_omitted_matrix_companions_to_be_absent() {
    let (mut document, projection) = fixed_membership_assembly_fixture(90.0).unwrap();
    let source_id = "matrix/source-matrix/r0c1/diode";
    let mirror_id = "matrix/mirror-matrix/r0c1/diode";
    let pair = &mut document.hardware.as_mut().unwrap().pairs[0];
    let halves = pair.halves.as_mut().unwrap();
    halves.exceptions.push(MirroredException {
        companion_mode: None,
        companion_tweak: None,
        kind: MirroredItemKind::Part,
        mirror_id: mirror_id.into(),
        state: MirroredExceptionState::Omitted,
        removed_copper: None,
    });
    document.parts.retain(|part| part.id != mirror_id);
    document
        .boards
        .iter_mut()
        .find(|board| board.id == "mirror-board")
        .unwrap()
        .part_ids
        .retain(|id| id != mirror_id);
    document
        .matrices
        .iter_mut()
        .find(|matrix| matrix.id == "mirror-matrix")
        .unwrap()
        .part_ids
        .retain(|id| id != mirror_id);
    let mut projection = projection;
    projection.final_owners.remove(mirror_id);
    boardstudio_core_mechanical::mechanical::halves::validate_document(&document)
        .expect("Omitted keeps the exact correspondence while its target is absent");
    let pair_before = serde_json::to_vec(&document.hardware.as_ref().unwrap().pairs[0]).unwrap();
    boardstudio_core_mirrored_half::mirrored_half::reconcile::reconcile(
        &mut document,
        "pair-source-mirror",
        &projection,
    )
    .expect("the exact Omitted overlay authenticates the absent child");
    assert!(document.parts.iter().all(|part| part.id != mirror_id));
    let mirror = document
        .matrices
        .iter()
        .find(|matrix| matrix.id == "mirror-matrix")
        .unwrap();
    assert!(!mirror.part_ids.iter().any(|id| id == mirror_id));
    assert!(mirror.cells.iter().any(|cell| {
        (cell.row, cell.column) == (0, 1)
            && cell
                .assemblies
                .iter()
                .any(|assembly| assembly.id == "diode")
    }));
    let pair_after = serde_json::to_vec(&document.hardware.as_ref().unwrap().pairs[0]).unwrap();
    assert_eq!(
        pair_after, pair_before,
        "Omitted retains durable provenance"
    );

    let (mut document, mut projection) = fixed_membership_assembly_fixture(90.0).unwrap();
    document.parts.retain(|part| part.id != mirror_id);
    document
        .boards
        .iter_mut()
        .find(|board| board.id == "mirror-board")
        .unwrap()
        .part_ids
        .retain(|id| id != mirror_id);
    document
        .matrices
        .iter_mut()
        .find(|matrix| matrix.id == "mirror-matrix")
        .unwrap()
        .part_ids
        .retain(|id| id != mirror_id);
    projection.final_owners.remove(mirror_id);
    let halves = document.hardware.as_ref().unwrap().pairs[0]
        .halves
        .as_ref()
        .unwrap();
    let source = document
        .matrices
        .iter()
        .find(|matrix| matrix.id == "source-matrix")
        .unwrap();
    let mirror = document
        .matrices
        .iter()
        .find(|matrix| matrix.id == "mirror-matrix")
        .unwrap();
    let association = halves
        .correspondences
        .iter()
        .find(|item| item.kind == MirroredItemKind::Matrix)
        .unwrap();
    let projected = boardstudio_core_mirrored_half::mirrored_half::reconcile::project_matrix(
        source,
        mirror,
        association,
        projection.mapping,
    )
    .unwrap();
    let error =
        match boardstudio_core_mirrored_half::mirrored_half::reconcile::build_fixed_membership_view(
            &document,
            halves,
            source,
            mirror,
            &projected,
            &projection,
        ) {
            Err(error) => error,
            Ok(_) => panic!("an unrecorded missing assembly must remain a refusal"),
        };
    assert!(error.contains("membership is not canonical"), "{error}");
    assert!(
        halves
            .correspondences
            .iter()
            .any(|item| item.source_id == source_id && item.mirror_id == mirror_id)
    );
}

#[test]
fn fixed_membership_view_preserves_authored_cell_defaults_and_variants() {
    let (mut document, projection) = fixed_membership_assembly_fixture(90.0).unwrap();
    let matrix_default = document
        .matrices
        .iter()
        .find(|matrix| matrix.id == "source-matrix")
        .unwrap()
        .definition_id
        .clone();
    let mut alternate_definition = document.definitions[0].clone();
    alternate_definition.id = "alternate-switch".into();
    document.definitions.push(alternate_definition);
    {
        let source_matrix = document
            .matrices
            .iter_mut()
            .find(|matrix| matrix.id == "source-matrix")
            .unwrap();
        let cell = source_matrix
            .cells
            .iter_mut()
            .find(|cell| (cell.row, cell.column) == (0, 1))
            .unwrap();
        // The definition override is redundant today but remains an authored per-cell
        // choice if the matrix default changes later.
        cell.definition_id = Some(matrix_default.clone());
        cell.variant = Some("preset/authored-red".into());
        cell.assemblies_local = Some(false);
        source_matrix
            .cells
            .iter_mut()
            .find(|cell| (cell.row, cell.column) == (0, 2))
            .unwrap()
            .definition_id = Some("alternate-switch".into());
    }
    let source_matrix = document
        .matrices
        .iter()
        .find(|matrix| matrix.id == "source-matrix")
        .unwrap()
        .clone();
    boardstudio_core_kernel::matrix::set_matrix(&mut document, &source_matrix)
        .expect("the source materializes its explicit and alternate definitions");

    let halves = document.hardware.as_ref().unwrap().pairs[0]
        .halves
        .as_ref()
        .unwrap();
    let source = document
        .matrices
        .iter()
        .find(|matrix| matrix.id == "source-matrix")
        .unwrap();
    let mirror = document
        .matrices
        .iter()
        .find(|matrix| matrix.id == "mirror-matrix")
        .unwrap();
    let correspondence = halves
        .correspondences
        .iter()
        .find(|association| association.kind == MirroredItemKind::Matrix)
        .unwrap();
    let projected = boardstudio_core_mirrored_half::mirrored_half::reconcile::project_matrix(
        source,
        mirror,
        correspondence,
        projection.mapping,
    )
    .unwrap();
    let view =
        boardstudio_core_mirrored_half::mirrored_half::reconcile::build_fixed_membership_view(
            &document,
            halves,
            source,
            mirror,
            &projected,
            &projection,
        )
        .expect("authored cell fields are valid reconciliation input");

    let cell = view
        .incoming_matrix
        .cells
        .iter()
        .find(|cell| (cell.row, cell.column) == (0, 1))
        .unwrap();
    assert_eq!(cell.definition_id.as_deref(), Some(matrix_default.as_str()));
    assert_eq!(cell.variant.as_deref(), Some("preset/authored-red"));
    assert_eq!(cell.assemblies_local, Some(false));
    let alternate = view
        .incoming_matrix
        .cells
        .iter()
        .find(|cell| (cell.row, cell.column) == (0, 2))
        .unwrap();
    assert_eq!(alternate.definition_id.as_deref(), Some("alternate-switch"));

    let source_before_reconcile = document
        .matrices
        .iter()
        .find(|matrix| matrix.id == "source-matrix")
        .unwrap()
        .clone();
    boardstudio_core_mirrored_half::mirrored_half::reconcile::reconcile(
        &mut document,
        "pair-source-mirror",
        &projection,
    )
    .expect("authored cell definitions materialize through reconciliation");
    assert_eq!(
        document
            .matrices
            .iter()
            .find(|matrix| matrix.id == "source-matrix")
            .unwrap(),
        &source_before_reconcile,
        "reconciliation preserves the complete authored source matrix"
    );
    for (matrix_id, coordinate, expected) in [
        ("source-matrix", (0, 1), matrix_default.as_str()),
        ("mirror-matrix", (0, 1), matrix_default.as_str()),
        ("source-matrix", (0, 2), "alternate-switch"),
        ("mirror-matrix", (0, 2), "alternate-switch"),
    ] {
        let key_id =
            boardstudio_core_kernel::matrix::member_id(matrix_id, coordinate.0, coordinate.1);
        let key = document
            .parts
            .iter()
            .find(|part| part.id == key_id)
            .unwrap_or_else(|| panic!("generated key {key_id} exists"));
        assert_eq!(key.definition_id, expected, "{key_id}");
    }
}

#[test]
fn fixed_membership_view_fills_only_missing_active_cells_with_defaults() {
    let (mut document, projection) = fixed_geometry_fixture().unwrap();
    for matrix_id in ["source-matrix", "mirror-matrix"] {
        document
            .matrices
            .iter_mut()
            .find(|matrix| matrix.id == matrix_id)
            .unwrap()
            .cells
            .retain(|cell| (cell.row, cell.column) != (0, 1));
    }

    let halves = document.hardware.as_ref().unwrap().pairs[0]
        .halves
        .as_ref()
        .unwrap();
    let source = document
        .matrices
        .iter()
        .find(|matrix| matrix.id == "source-matrix")
        .unwrap();
    let mirror = document
        .matrices
        .iter()
        .find(|matrix| matrix.id == "mirror-matrix")
        .unwrap();
    let correspondence = halves
        .correspondences
        .iter()
        .find(|association| association.kind == MirroredItemKind::Matrix)
        .unwrap();
    let projected = boardstudio_core_mirrored_half::mirrored_half::reconcile::project_matrix(
        source,
        mirror,
        correspondence,
        projection.mapping,
    )
    .unwrap();
    let view =
        boardstudio_core_mirrored_half::mirrored_half::reconcile::build_fixed_membership_view(
            &document,
            halves,
            source,
            mirror,
            &projected,
            &projection,
        )
        .expect("an absent active cell uses the matrix materializer defaults");

    let normalized_source_cell = view
        .normalized_source_matrix
        .cells
        .iter()
        .find(|cell| (cell.row, cell.column) == (0, 1))
        .unwrap();
    assert!(normalized_source_cell.enabled);
    assert!(!normalized_source_cell.deleted);
    assert_eq!(normalized_source_cell.definition_id, None);
    assert_eq!(view.normalized_source_matrix.cells.len(), 6);

    let cell = view
        .incoming_matrix
        .cells
        .iter()
        .find(|cell| (cell.row, cell.column) == (0, 1))
        .unwrap();
    assert!(cell.enabled);
    assert!(!cell.deleted);
    assert_eq!(cell.definition_id, None);
    assert_eq!(cell.variant, None);
    assert_eq!(cell.offset, None);
    assert_eq!(cell.rotation, None);
    assert!(cell.assemblies.is_empty());
    assert_eq!(cell.assemblies_local, None);
    assert_eq!(view.incoming_matrix.cells.len(), 6);

    boardstudio_core_mirrored_half::mirrored_half::reconcile::reconcile(
        &mut document,
        "pair-source-mirror",
        &projection,
    )
    .expect("both halves materialize the missing active cell");
    let first = document.clone();
    for matrix_id in ["source-matrix", "mirror-matrix"] {
        let matrix = document
            .matrices
            .iter()
            .find(|matrix| matrix.id == matrix_id)
            .unwrap();
        assert_eq!(matrix.cells.len(), 6);
        let filled = matrix
            .cells
            .iter()
            .find(|cell| (cell.row, cell.column) == (0, 1))
            .unwrap();
        assert!(filled.enabled);
        assert!(!filled.deleted);
        assert_eq!(filled.definition_id, None);
        assert_eq!(filled.variant, None);
        assert_eq!(filled.assemblies_local, None);
    }
    let repeated = boardstudio_core_mirrored_half::mirrored_half::reconcile::reconcile(
        &mut document,
        "pair-source-mirror",
        &projection,
    )
    .expect("repeated sparse reconciliation is stable");
    assert_eq!(document, first);
    assert!(
        repeated.is_empty(),
        "repeated reconciliation reported {repeated:?}"
    );
}

#[test]
fn source_and_mirror_local_assemblies_have_no_correspondence_and_survive_reconcile() {
    let (mut document, projection) = fixed_membership_assembly_fixture(90.0).unwrap();
    let mut target_definition = document.definitions[0].clone();
    target_definition.id = "target-local-switch".into();
    document.definitions.push(target_definition);
    for matrix_id in ["source-matrix", "mirror-matrix"] {
        let cell = document
            .matrices
            .iter_mut()
            .find(|matrix| matrix.id == matrix_id)
            .unwrap()
            .cells
            .iter_mut()
            .find(|cell| (cell.row, cell.column) == (0, 0))
            .unwrap();
        cell.assemblies_local = Some(true);
        cell.variant = Some(if matrix_id == "source-matrix" {
            "source-local-variant".into()
        } else {
            "target-local-variant".into()
        });
        if matrix_id == "mirror-matrix" {
            cell.definition_id = Some("target-local-switch".into());
        }
    }
    let mut source_matrix = document
        .matrices
        .iter()
        .find(|matrix| matrix.id == "source-matrix")
        .unwrap()
        .clone();
    source_matrix.cells[0].assemblies.push(MatrixAssembly {
        id: "source-only".into(),
        definition_id: "diode".into(),
        offset: Vec2 { x: 9.0, y: 1.0 },
        rotation: Some(0.0),
        side: Some(Side::Back),
    });
    boardstudio_core_kernel::matrix::set_matrix(&mut document, &source_matrix).unwrap();
    let target_matrix = document
        .matrices
        .iter()
        .find(|matrix| matrix.id == "mirror-matrix")
        .unwrap()
        .clone();
    boardstudio_core_kernel::matrix::set_matrix(&mut document, &target_matrix).unwrap();
    let mut projection = projection;
    projection.final_owners.insert(
        "matrix/source-matrix/r0c0/source-only".into(),
        "source-board".into(),
    );
    document.hardware.as_mut().unwrap().pairs[0]
        .halves
        .as_mut()
        .unwrap()
        .correspondences
        .retain(|association| {
            association.source_id != "matrix/source-matrix/r0c0/diode"
                && association.mirror_id != "matrix/mirror-matrix/r0c0/diode"
        });
    let mirror_assembly = document
        .matrices
        .iter()
        .find(|matrix| matrix.id == "mirror-matrix")
        .unwrap()
        .cells[0]
        .assemblies[0]
        .clone();
    let target_cell = document
        .matrices
        .iter()
        .find(|matrix| matrix.id == "mirror-matrix")
        .unwrap()
        .cells
        .iter()
        .find(|cell| (cell.row, cell.column) == (0, 0))
        .unwrap()
        .clone();
    let source_assemblies = document
        .matrices
        .iter()
        .find(|matrix| matrix.id == "source-matrix")
        .unwrap()
        .cells[0]
        .assemblies
        .clone();
    let mirror_part = document
        .parts
        .iter()
        .find(|part| part.id == "matrix/mirror-matrix/r0c0/diode")
        .unwrap()
        .clone();

    reconcile(&mut document, "pair-source-mirror", &projection)
        .expect("independent local companion lists do not need pair associations");

    assert_eq!(
        document
            .matrices
            .iter()
            .find(|matrix| matrix.id == "source-matrix")
            .unwrap()
            .cells[0]
            .assemblies,
        source_assemblies
    );
    let source_cell = document
        .matrices
        .iter()
        .find(|matrix| matrix.id == "source-matrix")
        .unwrap()
        .cells
        .iter()
        .find(|cell| (cell.row, cell.column) == (0, 0))
        .unwrap();
    assert_eq!(source_cell.definition_id, None);
    assert_eq!(source_cell.variant.as_deref(), Some("source-local-variant"));
    let source_key = document
        .parts
        .iter()
        .find(|part| part.id == "matrix/source-matrix/r0c0")
        .unwrap();
    assert_eq!(source_key.definition_id, "switch");
    assert_eq!(
        source_key
            .properties
            .as_ref()
            .and_then(|properties| properties.get("variant")),
        Some(&serde_json::json!("source-local-variant"))
    );
    assert_eq!(
        document
            .matrices
            .iter()
            .find(|matrix| matrix.id == "mirror-matrix")
            .unwrap()
            .cells[0]
            .assemblies[0],
        mirror_assembly
    );
    let mirrored_cell = document
        .matrices
        .iter()
        .find(|matrix| matrix.id == "mirror-matrix")
        .unwrap()
        .cells
        .iter()
        .find(|cell| (cell.row, cell.column) == (0, 0))
        .unwrap();
    assert_eq!(mirrored_cell.definition_id, target_cell.definition_id);
    assert_eq!(mirrored_cell.variant, target_cell.variant);
    assert_eq!(mirrored_cell.assemblies_local, target_cell.assemblies_local);
    assert_eq!(mirrored_cell.assemblies, target_cell.assemblies);
    assert_eq!(
        mirrored_cell.definition_id.as_deref(),
        Some("target-local-switch")
    );
    assert_eq!(
        mirrored_cell.variant.as_deref(),
        Some("target-local-variant")
    );
    let target_key = document
        .parts
        .iter()
        .find(|part| part.id == "matrix/mirror-matrix/r0c0")
        .unwrap();
    assert_eq!(target_key.definition_id, "target-local-switch");
    assert_eq!(
        target_key
            .properties
            .as_ref()
            .and_then(|properties| properties.get("variant")),
        Some(&serde_json::json!("target-local-variant"))
    );
    assert_eq!(
        document
            .parts
            .iter()
            .find(|part| part.id == mirror_part.id)
            .unwrap(),
        &mirror_part
    );
    let after_first = document.clone();
    assert!(
        reconcile(&mut document, "pair-source-mirror", &projection)
            .unwrap()
            .is_empty()
    );
    assert_eq!(document, after_first);
}

#[test]
fn fixed_membership_assembly_rejections_are_atomic() {
    fn assert_rejected(mut document: ProjectDoc, projection: &Projection, reason: &str) {
        let before = serde_json::to_value(&document).unwrap();
        let Err(error) = reconcile(&mut document, "pair-source-mirror", projection) else {
            panic!("expected atomic rejection containing {reason:?}");
        };
        assert!(error.contains(reason), "expected {reason:?}, got {error:?}");
        assert_eq!(serde_json::to_value(&document).unwrap(), before);
    }

    for (part_id, authored) in [
        ("matrix/mirror-matrix/r0c1", true),
        ("matrix/source-matrix/r0c1", true),
        ("matrix/mirror-matrix/r0c1", false),
        ("matrix/source-matrix/r0c1", false),
    ] {
        let (mut document, projection) = fixed_membership_assembly_fixture(90.0).unwrap();
        if authored {
            document
                .parts
                .iter_mut()
                .find(|part| part.id == part_id)
                .unwrap()
                .properties
                .as_mut()
                .unwrap()
                .insert("layoutOverride".into(), serde_json::json!(1));
        } else {
            document.constraints.push(Constraint::Offset {
                id: "linked-key-constraint".into(),
                source_part_id: "assembly-probe".into(),
                target_part_id: part_id.into(),
                offset: Vec2 { x: 1.0, y: 2.0 },
                rotation: 0.0,
            });
        }
        assert_rejected(
            document,
            &projection,
            "Linked matrix keys cannot have authored or constrained placement",
        );
    }

    let (mut document, projection) = fixed_membership_assembly_fixture(90.0).unwrap();
    document
        .parts
        .retain(|part| part.id != "matrix/source-matrix/r0c0/diode");
    assert_rejected(document, &projection, "unavailable");

    let (mut document, projection) = fixed_membership_assembly_fixture(90.0).unwrap();
    document.hardware.as_mut().unwrap().pairs[0]
        .halves
        .as_mut()
        .unwrap()
        .correspondences
        .retain(|association| association.source_id != "matrix/source-matrix/r0c0/diode");
    assert_rejected(document, &projection, "companion association is missing");

    let (mut document, projection) = fixed_membership_assembly_fixture(90.0).unwrap();
    let association = document.hardware.as_mut().unwrap().pairs[0]
        .halves
        .as_mut()
        .unwrap()
        .correspondences
        .iter_mut()
        .find(|association| association.source_id == "matrix/source-matrix/r0c0/diode")
        .unwrap();
    association.source_id = "matrix/mirror-matrix/r0c0/diode".into();
    association.mirror_id = "matrix/source-matrix/r0c0/diode".into();
    assert_rejected(
        document,
        &projection,
        "companion association is aliased or inconsistent",
    );

    let (mut document, projection) = fixed_membership_assembly_fixture(90.0).unwrap();
    let association = document.hardware.as_mut().unwrap().pairs[0]
        .halves
        .as_mut()
        .unwrap()
        .correspondences
        .iter()
        .find(|association| association.source_id == "matrix/source-matrix/r0c0/diode")
        .unwrap()
        .clone();
    document.hardware.as_mut().unwrap().pairs[0]
        .halves
        .as_mut()
        .unwrap()
        .correspondences
        .push(association);
    assert_rejected(
        document,
        &projection,
        "companion association is aliased or inconsistent",
    );

    let (mut document, projection) = fixed_membership_assembly_fixture(90.0).unwrap();
    let association = document.hardware.as_mut().unwrap().pairs[0]
        .halves
        .as_mut()
        .unwrap()
        .correspondences
        .iter_mut()
        .find(|association| association.source_id == "matrix/source-matrix/r0c0/diode")
        .unwrap();
    association.mirror_id = "matrix/mirror-matrix/r0c1/diode".into();
    assert_rejected(
        document,
        &projection,
        "companion association is aliased or inconsistent",
    );

    let (document, mut projection) = fixed_membership_assembly_fixture(90.0).unwrap();
    projection
        .final_owners
        .remove("matrix/mirror-matrix/r1c1/diode");
    assert_rejected(document, &projection, "final owner input is incomplete");

    let (mut document, projection) = fixed_membership_assembly_fixture(90.0).unwrap();
    document
        .boards
        .iter_mut()
        .find(|board| board.id == "mirror-board")
        .unwrap()
        .part_ids
        .retain(|id| id != "matrix/mirror-matrix/r1c1/diode");
    assert_rejected(document, &projection, "no board owner");

    let (mut document, projection) = fixed_membership_assembly_fixture(90.0).unwrap();
    document
        .boards
        .iter_mut()
        .find(|board| board.id == "source-board")
        .unwrap()
        .part_ids
        .push("matrix/mirror-matrix/r1c1/diode".into());
    assert_rejected(document, &projection, "ambiguous board ownership");

    let (mut document, projection) = fixed_membership_assembly_fixture(90.0).unwrap();
    document
        .boards
        .iter_mut()
        .find(|board| board.id == "mirror-board")
        .unwrap()
        .part_ids
        .retain(|id| id != "matrix/mirror-matrix/r0c1/diode");
    document
        .boards
        .iter_mut()
        .find(|board| board.id == "source-board")
        .unwrap()
        .part_ids
        .push("matrix/mirror-matrix/r0c1/diode".into());
    assert_rejected(
        document,
        &projection,
        "companion has an incomplete or stale owner input",
    );

    let (mut document, mut projection) = fixed_membership_assembly_fixture(90.0).unwrap();
    let matrix = document
        .matrices
        .iter_mut()
        .find(|matrix| matrix.id == "source-matrix")
        .unwrap();
    matrix
        .cells
        .iter_mut()
        .find(|cell| (cell.row, cell.column) == (0, 0))
        .unwrap()
        .assemblies
        .clear();
    matrix
        .part_ids
        .retain(|id| id != "matrix/source-matrix/r0c0/diode");
    document
        .parts
        .retain(|part| part.id != "matrix/source-matrix/r0c0/diode");
    document
        .boards
        .iter_mut()
        .find(|board| board.id == "source-board")
        .unwrap()
        .part_ids
        .retain(|id| id != "matrix/source-matrix/r0c0/diode");
    document.hardware.as_mut().unwrap().pairs[0]
        .halves
        .as_mut()
        .unwrap()
        .correspondences
        .retain(|association| association.source_id != "matrix/source-matrix/r0c0/diode");
    projection
        .final_owners
        .remove("matrix/source-matrix/r0c0/diode");
    assert_rejected(document, &projection, "membership is not canonical");

    let (mut document, projection) = fixed_membership_assembly_fixture(90.0).unwrap();
    document
        .matrices
        .iter_mut()
        .find(|matrix| matrix.id == "source-matrix")
        .unwrap()
        .cells[0]
        .assemblies[0]
        .offset
        .x = f64::NAN;
    assert_rejected(document, &projection, "Matrix assembly is invalid");

    let (mut document, projection) = fixed_membership_assembly_fixture(90.0).unwrap();
    let association = document.hardware.as_mut().unwrap().pairs[0]
        .halves
        .as_mut()
        .unwrap()
        .correspondences
        .iter_mut()
        .find(|association| association.source_id == "matrix/source-matrix/r1c0/diode")
        .unwrap();
    association.companion_tweak.as_mut().unwrap().rotation = f64::NAN;
    assert_rejected(document, &projection, "finite");

    let (mut document, projection) = fixed_membership_assembly_fixture(90.0).unwrap();
    let mut source = document
        .matrices
        .iter()
        .find(|matrix| matrix.id == "source-matrix")
        .unwrap()
        .clone();
    source
        .cells
        .iter_mut()
        .find(|cell| (cell.row, cell.column) == (1, 0))
        .unwrap()
        .assemblies[0]
        .offset
        .x = f64::MAX * 0.9;
    boardstudio_core_kernel::matrix::set_matrix(&mut document, &source).unwrap();
    let association = document.hardware.as_mut().unwrap().pairs[0]
        .halves
        .as_mut()
        .unwrap()
        .correspondences
        .iter_mut()
        .find(|association| association.source_id == "matrix/source-matrix/r1c0/diode")
        .unwrap();
    association.companion_tweak.as_mut().unwrap().x = -f64::MAX * 0.9;
    assert_rejected(document, &projection, "non-finite values");

    let (mut document, projection) = fixed_membership_assembly_fixture(90.0).unwrap();
    document.constraints.push(Constraint::Offset {
        id: "linked-companion-constraint".into(),
        source_part_id: "assembly-probe".into(),
        target_part_id: "matrix/mirror-matrix/r0c1/diode".into(),
        offset: Vec2 { x: 1.0, y: 2.0 },
        rotation: 0.0,
    });
    assert_rejected(document, &projection, "authored or constrained placement");

    let (mut document, projection) = fixed_membership_assembly_fixture(90.0).unwrap();
    let companion = document
        .parts
        .iter_mut()
        .find(|part| part.id == "matrix/mirror-matrix/r0c1/diode")
        .unwrap();
    companion
        .properties
        .get_or_insert_with(BTreeMap::new)
        .insert("layoutOverride".into(), serde_json::json!(1));
    assert_rejected(document, &projection, "authored or constrained placement");
}

#[test]
fn fixed_membership_view_rejects_legacy_exception_aliases_and_foreign_owners() {
    fn legacy_child_fixture() -> (ProjectDoc, Projection, String) {
        let (mut document, projection) = fixed_membership_assembly_fixture(90.0).unwrap();
        let source_id = "matrix/source-matrix/r0c1/diode";
        let mirror_id = "matrix/mirror-matrix/r0c1/diode".to_owned();
        let halves = document.hardware.as_mut().unwrap().pairs[0]
            .halves
            .as_mut()
            .unwrap();
        halves.correspondences.retain(|association| {
            !(association.kind == MirroredItemKind::Part
                && association.source_id == source_id
                && association.mirror_id == mirror_id)
        });
        halves.exceptions.push(MirroredException {
            companion_mode: Some(MirroredCompanionMode::LiteralMirror),
            companion_tweak: None,
            kind: MirroredItemKind::Part,
            mirror_id: mirror_id.clone(),
            state: MirroredExceptionState::Local,
            removed_copper: None,
        });
        (document, projection, mirror_id)
    }

    fn assert_rejected(mut document: ProjectDoc, projection: &Projection, reason: &str) {
        let before = serde_json::to_value(&document).unwrap();
        let Err(error) = reconcile(&mut document, "pair-source-mirror", projection) else {
            panic!("expected atomic rejection containing {reason:?}");
        };
        assert!(error.contains(reason), "expected {reason:?}, got {error:?}");
        assert_eq!(serde_json::to_value(&document).unwrap(), before);
    }

    let (mut document, projection, _) = legacy_child_fixture();
    document.hardware.as_mut().unwrap().pairs[0]
        .halves
        .as_mut()
        .unwrap()
        .exceptions[0]
        .mirror_id = "matrix/mirror-matrix/r0c2/diode".into();
    assert_rejected(document, &projection, "companion association is missing");

    let (mut document, projection, _) = legacy_child_fixture();
    let halves = document.hardware.as_mut().unwrap().pairs[0]
        .halves
        .as_mut()
        .unwrap();
    halves.exceptions[0].kind = MirroredItemKind::Net;
    assert_rejected(
        document,
        &projection,
        "companion exception is aliased or inconsistent",
    );

    let (mut document, projection, _) = legacy_child_fixture();
    let source_id = "matrix/source-matrix/r0c1/diode";
    let mirror_id = "matrix/mirror-matrix/r0c1/diode";
    let halves = document.hardware.as_mut().unwrap().pairs[0]
        .halves
        .as_mut()
        .unwrap();
    halves.correspondences.push(MirroredCorrespondence {
        companion_mode: None,
        companion_tweak: None,
        kind: MirroredItemKind::Part,
        source_id: mirror_id.into(),
        mirror_id: source_id.into(),
    });
    assert_rejected(
        document,
        &projection,
        "companion association is aliased or inconsistent",
    );

    let (mut document, projection, _mirror_id) = legacy_child_fixture();
    let exception = document.hardware.as_mut().unwrap().pairs[0]
        .halves
        .as_mut()
        .unwrap()
        .exceptions[0]
        .clone();
    document.hardware.as_mut().unwrap().pairs[0]
        .halves
        .as_mut()
        .unwrap()
        .exceptions
        .push(exception);
    assert_rejected(
        document,
        &projection,
        "companion exception is aliased or inconsistent",
    );
    let _ = mirror_id;

    let (mut document, projection, mirror_id) = legacy_child_fixture();
    document
        .boards
        .iter_mut()
        .find(|board| board.id == "mirror-board")
        .unwrap()
        .part_ids
        .retain(|id| id != &mirror_id);
    document
        .boards
        .iter_mut()
        .find(|board| board.id == "source-board")
        .unwrap()
        .part_ids
        .push(mirror_id);
    assert_rejected(document, &projection, "incomplete or stale owner input");
}

#[test]
fn fixed_membership_rejections_are_atomic_and_reach_the_intended_guard() {
    fn assert_rejected(mut document: ProjectDoc, projection: &Projection, expected_reason: &str) {
        let before = serde_json::to_value(&document).unwrap();
        let error = reconcile(&mut document, "pair-source-mirror", projection).unwrap_err();
        assert!(
            error.contains(expected_reason),
            "expected {expected_reason:?}, got {error:?}"
        );
        assert_eq!(serde_json::to_value(&document).unwrap(), before);
    }

    let (mut document, projection) = fixed_geometry_fixture().unwrap();
    document.hardware.as_mut().unwrap().pairs[0]
        .halves
        .as_mut()
        .unwrap()
        .correspondences
        .push(MirroredCorrespondence {
            companion_mode: None,
            companion_tweak: None,
            kind: MirroredItemKind::Matrix,
            source_id: "mirror-matrix".into(),
            mirror_id: "source-matrix".into(),
        });
    assert_rejected(document, &projection, "matrix correspondence is ambiguous");

    let (document, mut projection) = fixed_geometry_fixture().unwrap();
    projection.parts.push(PartProjection {
        source_id: "matrix/source-matrix/r0c1".into(),
        mirror_id: "matrix/mirror-matrix/r0c1".into(),
    });
    assert_rejected(
        document,
        &projection,
        "linked matrix associations contain aliases",
    );

    let (mut document, projection) = fixed_geometry_fixture().unwrap();
    document
        .parts
        .retain(|part| part.id != "matrix/source-matrix/r0c0");
    assert_rejected(document, &projection, "unavailable");

    let (document, mut projection) = fixed_geometry_fixture().unwrap();
    projection.final_owners.remove("matrix/mirror-matrix/r1c1");
    assert_rejected(document, &projection, "final owner input is incomplete");

    let (mut document, projection) = fixed_geometry_fixture().unwrap();
    let part_id = "matrix/source-matrix/r0c1";
    document
        .boards
        .iter_mut()
        .find(|board| board.id == "source-board")
        .unwrap()
        .part_ids
        .retain(|id| id != part_id);
    document
        .boards
        .iter_mut()
        .find(|board| board.id == "mirror-board")
        .unwrap()
        .part_ids
        .push(part_id.into());
    assert_rejected(document, &projection, "owner input");

    let (mut document, projection) = fixed_geometry_fixture().unwrap();
    document
        .matrices
        .iter_mut()
        .find(|matrix| matrix.id == "source-matrix")
        .unwrap()
        .cells[1]
        .enabled = false;
    assert_rejected(document, &projection, "fixed active membership");

    let (mut document, projection) = fixed_geometry_fixture().unwrap();
    document
        .matrices
        .iter_mut()
        .find(|matrix| matrix.id == "source-matrix")
        .unwrap()
        .part_ids
        .push("matrix/source-matrix/r0c0".into());
    assert_rejected(document, &projection, "membership is not canonical");

    let (mut document, projection) = fixed_geometry_fixture().unwrap();
    document
        .matrices
        .iter_mut()
        .find(|matrix| matrix.id == "source-matrix")
        .unwrap()
        .column_layout[1]
        .splay = f64::NAN;
    assert_rejected(document, &projection, "finite");

    let (mut document, projection) = fixed_geometry_fixture().unwrap();
    document
        .matrices
        .iter_mut()
        .find(|matrix| matrix.id == "source-matrix")
        .unwrap()
        .column_layout = vec![
        boardstudio_model::model::ColumnLayout {
            splay: f64::MAX,
            ..Default::default()
        },
        boardstudio_model::model::ColumnLayout {
            splay: f64::MAX,
            ..Default::default()
        },
        Default::default(),
    ];
    assert_rejected(
        document,
        &projection,
        "layout values and evaluated results must be finite",
    );

    let (mut document, projection) = fixed_geometry_fixture().unwrap();
    document.hardware.as_mut().unwrap().pairs[0]
        .halves
        .as_mut()
        .unwrap()
        .correspondences
        .retain(|association| association.source_id != "matrix/source-matrix/r1c1");
    assert_rejected(
        document,
        &projection,
        "canonical matrix part association is missing",
    );
}

#[test]
fn changed_source_matrix_projects_linked_neighbor_after_entry_snapshot() {
    let mut document = fixture_document().unwrap();
    let mut moved_source = document
        .matrices
        .iter()
        .find(|matrix| matrix.id == "source-matrix")
        .unwrap()
        .clone();
    moved_source.origin = Vec2 { x: 27.0, y: -14.0 };
    moved_source.rotation = Some(43.0);
    boardstudio_core_kernel::matrix::set_matrix(&mut document, &moved_source).unwrap();

    let definition = document
        .definitions
        .iter_mut()
        .find(|definition| definition.id == "switch")
        .unwrap();
    definition.pads = vec![
        boardstudio_model::model::Pad {
            id: "pad-one".into(),
            number: "1".into(),
            at: Vec2 { x: 0.0, y: 0.0 },
            size: Vec2 { x: 1.0, y: 1.0 },
            shape: boardstudio_model::model::PadShape::Circle,
            drill: None,
            plated: None,
            side: None,
            rotation: None,
            net_id: None,
        },
        boardstudio_model::model::Pad {
            id: "pad-two".into(),
            number: "2".into(),
            at: Vec2 { x: 1.0, y: 0.0 },
            size: Vec2 { x: 1.0, y: 1.0 },
            shape: boardstudio_model::model::PadShape::Circle,
            drill: None,
            plated: None,
            side: None,
            rotation: None,
            net_id: None,
        },
    ];

    let probe_id = "projection-probe";
    let mut probe = document
        .parts
        .iter()
        .find(|part| part.id == "matrix/mirror-matrix/r0c1")
        .unwrap()
        .clone();
    probe.id = probe_id.into();
    probe.reference = "PROBE".into();
    probe.pose.at = Vec2 { x: 4.0, y: 2.0 };
    probe.pose.rotation = 0.0;
    document.parts.push(probe);
    document
        .boards
        .iter_mut()
        .find(|board| board.id == "mirror-board")
        .unwrap()
        .part_ids
        .push(probe_id.into());

    let retained_net_id = "retained-two-pin-net";
    document.nets.push(boardstudio_model::model::Net {
        id: retained_net_id.into(),
        name: "RETAINED".into(),
        pins: vec![
            boardstudio_model::model::Pin {
                part_id: "matrix/mirror-matrix/r0c0".into(),
                pad_id: "pad-one".into(),
            },
            boardstudio_model::model::Pin {
                part_id: probe_id.into(),
                pad_id: "pad-two".into(),
            },
        ],
    });
    document
        .boards
        .iter_mut()
        .find(|board| board.id == "mirror-board")
        .unwrap()
        .net_ids
        .push(retained_net_id.into());

    let retained_offset = boardstudio_model::model::Constraint::Offset {
        id: "retained-stationary-offset".into(),
        source_part_id: probe_id.into(),
        target_part_id: "matrix/mirror-matrix/r0c0".into(),
        offset: Vec2 { x: -4.0, y: -2.0 },
        rotation: 0.0,
    };
    document.constraints.push(retained_offset.clone());

    let retained_envelope_id = "retained-global-envelope";
    document
        .outline
        .push(boardstudio_model::model::OutlineFeature::PartEnvelope {
            connections: vec![],
            settings: Default::default(),
            id: retained_envelope_id.into(),
            part_ids: vec![
                "matrix/mirror-matrix/r0c0".into(),
                "matrix/mirror-matrix/r0c1".into(),
                probe_id.into(),
            ],
            margin: 2.0,
            operation: boardstudio_model::model::Operation::Add,
        });
    document
        .boards
        .iter_mut()
        .find(|board| board.id == "mirror-board")
        .unwrap()
        .outline_ids
        .push(retained_envelope_id.into());

    let assert_retained_net_is_valid = |document: &ProjectDoc| {
        let net = document
            .nets
            .iter()
            .find(|net| net.id == retained_net_id)
            .unwrap();
        assert_eq!(
            net.pins,
            vec![
                boardstudio_model::model::Pin {
                    part_id: "matrix/mirror-matrix/r0c0".into(),
                    pad_id: "pad-one".into(),
                },
                boardstudio_model::model::Pin {
                    part_id: probe_id.into(),
                    pad_id: "pad-two".into(),
                },
            ]
        );
        let owner = document
            .boards
            .iter()
            .find(|board| board.net_ids.contains(&net.id))
            .unwrap();
        assert_eq!(owner.id, "mirror-board");
        assert_eq!(
            document
                .boards
                .iter()
                .filter(|board| board.net_ids.contains(&net.id))
                .count(),
            1
        );
        for pin in &net.pins {
            assert!(owner.part_ids.contains(&pin.part_id));
            let part = document
                .parts
                .iter()
                .find(|part| part.id == pin.part_id)
                .unwrap();
            let definition = document
                .definitions
                .iter()
                .find(|definition| definition.id == part.definition_id)
                .unwrap();
            assert!(
                definition
                    .pads
                    .iter()
                    .any(|pad| pad.id == pin.pad_id && !pad.id.is_empty())
            );
        }

        let retained_mirror = document
            .parts
            .iter()
            .find(|part| part.id == "matrix/mirror-matrix/r0c0")
            .unwrap();
        assert_eq!(
            retained_mirror.pose,
            boardstudio_model::model::Pose2 {
                at: Vec2 { x: 0.0, y: 0.0 },
                rotation: 0.0,
            }
        );
        let probe = document
            .parts
            .iter()
            .find(|part| part.id == probe_id)
            .unwrap();
        assert_eq!(
            probe.pose,
            boardstudio_model::model::Pose2 {
                at: Vec2 { x: 4.0, y: 2.0 },
                rotation: 0.0,
            }
        );

        let envelope = document
            .outline
            .iter()
            .find(|feature| {
                matches!(feature, boardstudio_model::model::OutlineFeature::PartEnvelope { id, .. } if id == retained_envelope_id)
            })
            .unwrap();
        let boardstudio_model::model::OutlineFeature::PartEnvelope { part_ids, .. } = envelope
        else {
            unreachable!("the retained feature was found as a part envelope");
        };
        assert_eq!(
            part_ids,
            &vec![
                "matrix/mirror-matrix/r0c0".to_owned(),
                "matrix/mirror-matrix/r0c1".to_owned(),
                probe_id.to_owned(),
            ]
        );
        assert_eq!(
            document
                .boards
                .iter()
                .filter(|board| board.outline_ids.contains(&retained_envelope_id.to_owned()))
                .map(|board| board.id.as_str())
                .collect::<Vec<_>>(),
            vec!["mirror-board"]
        );
        assert!(
            document
                .boards
                .iter()
                .find(|board| board.id == "source-board")
                .unwrap()
                .net_ids
                .iter()
                .all(|net_id| net_id != retained_net_id)
        );
    };
    assert_retained_net_is_valid(&document);
    boardstudio_core_mechanical::mechanical::halves::validate_document(&document).unwrap();

    // The entry snapshot is deliberately taken after the source's ordinary edit/materialization.
    let entry = document.clone();
    let entry_source_matrix = entry
        .matrices
        .iter()
        .find(|matrix| matrix.id == "source-matrix")
        .unwrap()
        .clone();
    let entry_mirror_matrix = entry
        .matrices
        .iter()
        .find(|matrix| matrix.id == "mirror-matrix")
        .unwrap()
        .clone();
    let matrix_correspondence = MirroredCorrespondence {
        companion_mode: None,
        companion_tweak: None,
        kind: MirroredItemKind::Matrix,
        source_id: entry_source_matrix.id.clone(),
        mirror_id: entry_mirror_matrix.id.clone(),
    };
    let expected_mirror_matrix = project_matrix(
        &entry_source_matrix,
        &entry_mirror_matrix,
        &matrix_correspondence,
        fixture_mapping(),
    )
    .unwrap();
    let mut expected_document = entry.clone();
    boardstudio_core_kernel::matrix::set_matrix(&mut expected_document, &expected_mirror_matrix)
        .unwrap();

    let protected_local_id = "matrix/mirror-matrix/r0c0";
    let linked_source_id = "matrix/source-matrix/r0c1";
    let linked_mirror_id = "matrix/mirror-matrix/r0c1";
    let entry_probe = entry
        .parts
        .iter()
        .find(|part| part.id == probe_id)
        .unwrap()
        .clone();
    let entry_probe_owner = entry
        .boards
        .iter()
        .find(|board| board.part_ids.contains(&probe_id.to_owned()))
        .unwrap()
        .id
        .clone();
    let entry_local = entry
        .parts
        .iter()
        .find(|part| part.id == protected_local_id)
        .unwrap();

    let mut final_owners = BTreeMap::new();
    for id in [
        "matrix/source-matrix/r0c0",
        linked_source_id,
        protected_local_id,
        linked_mirror_id,
    ] {
        let owner = entry
            .boards
            .iter()
            .find(|board| board.part_ids.iter().any(|part_id| part_id == id))
            .map(|board| board.id.clone())
            .unwrap();
        final_owners.insert(id.to_owned(), owner);
    }
    let projection = Projection {
        matrix: Some(MatrixLink {
            source_matrix_id: entry_source_matrix.id.clone(),
            mirror_matrix_id: entry_mirror_matrix.id.clone(),
        }),
        preserve_matrix_geometry: false,
        source_board_id: "source-board".into(),
        mirror_board_id: "mirror-board".into(),
        mapping: fixture_mapping(),
        parts: vec![PartProjection {
            source_id: linked_source_id.into(),
            mirror_id: linked_mirror_id.into(),
        }],
        independent_part_transitions: vec![],
        hosted_transitions: vec![],
        applied_independent_additions: std::cell::RefCell::new(BTreeSet::new()),
        applied_hosted_operations: std::cell::RefCell::new(BTreeSet::new()),
        applied_hosted_circuit_additions: std::cell::RefCell::new(BTreeMap::new()),
        hosted_removal_baselines: std::cell::RefCell::new(BTreeMap::new()),
        hosted_omission_baselines: std::cell::RefCell::new(BTreeMap::new()),
        outline_removal_baselines: std::cell::RefCell::new(BTreeMap::new()),
        locally_protected_mirror_parts: BTreeSet::from([protected_local_id.into()]),
        final_owners: final_owners.clone(),
    };

    let entry_source_matrix_json = serde_json::to_value(&entry_source_matrix).unwrap();
    let entry_source_parts_json: Vec<_> = ["matrix/source-matrix/r0c0", linked_source_id]
        .into_iter()
        .map(|id| {
            serde_json::to_value(entry.parts.iter().find(|part| part.id == id).unwrap()).unwrap()
        })
        .collect();
    let entry_mirror_ids = entry_mirror_matrix.part_ids.clone();
    let entry_local_json = serde_json::to_value(entry_local).unwrap();
    let protected_references = serde_json::json!({
        "boards": &entry.boards,
        "nets": &entry.nets,
        "constraints": &entry.constraints,
        "boardOutlines": &entry.board_outlines,
        "definitions": &entry.definitions,
        "outline": &entry.outline,
    });
    let changed_ids = reconcile(&mut document, "pair-source-mirror", &projection).unwrap();

    assert_retained_net_is_valid(&document);
    boardstudio_core_mechanical::mechanical::halves::validate_document(&document).unwrap();
    assert_eq!(
        document.constraints.iter().find(|constraint| {
            matches!(constraint, boardstudio_model::model::Constraint::Offset { id, .. } if id == "retained-stationary-offset")
        }),
        Some(&retained_offset)
    );
    let actual_probe = document
        .parts
        .iter()
        .find(|part| part.id == probe_id)
        .unwrap();
    assert_eq!(actual_probe, &entry_probe);
    let actual_probe_owner = document
        .boards
        .iter()
        .find(|board| board.part_ids.contains(&probe_id.to_owned()))
        .unwrap()
        .id
        .clone();
    assert_eq!(actual_probe_owner, entry_probe_owner);
    assert!(
        document
            .boards
            .iter()
            .find(|board| board.id == entry_probe_owner)
            .unwrap()
            .part_ids
            .contains(&probe_id.to_owned())
    );

    let actual_neighbor = document
        .parts
        .iter()
        .find(|part| part.id == linked_mirror_id)
        .unwrap();
    assert!(
        (actual_neighbor.pose.at.x - 45.48221151584141).abs() < 1e-9,
        "the linked neighbor must receive the independently calculated reflected x position"
    );
    assert!(
        (actual_neighbor.pose.at.y - -46.956048106863605).abs() < 1e-9,
        "the linked neighbor must receive the independently calculated reflected y position"
    );
    assert!(
        (actual_neighbor.pose.rotation - -12.0).abs() < 1e-9,
        "the linked neighbor must receive the independently calculated reflected heading"
    );

    assert_eq!(
        serde_json::to_value(
            document
                .matrices
                .iter()
                .find(|matrix| matrix.id == "source-matrix")
                .unwrap()
        )
        .unwrap(),
        entry_source_matrix_json
    );
    let actual_source_parts_json: Vec<_> = ["matrix/source-matrix/r0c0", linked_source_id]
        .into_iter()
        .map(|id| {
            serde_json::to_value(document.parts.iter().find(|part| part.id == id).unwrap()).unwrap()
        })
        .collect();
    assert_eq!(actual_source_parts_json, entry_source_parts_json);
    let actual_mirror_matrix = document
        .matrices
        .iter()
        .find(|matrix| matrix.id == "mirror-matrix")
        .unwrap();
    assert_eq!(actual_mirror_matrix.id, entry_mirror_matrix.id);
    assert_eq!(actual_mirror_matrix.part_ids, entry_mirror_ids);
    let mut rematerialized = entry.clone();
    boardstudio_core_kernel::matrix::set_matrix(&mut rematerialized, actual_mirror_matrix).unwrap();
    let rematerialized_matrix = rematerialized
        .matrices
        .iter()
        .find(|matrix| matrix.id == "mirror-matrix")
        .unwrap();
    assert_eq!(
        serde_json::to_value(actual_mirror_matrix).unwrap(),
        serde_json::to_value(rematerialized_matrix).unwrap(),
        "the final active matrix must already be a complete normalized materializer input"
    );
    let actual_retained = document
        .parts
        .iter()
        .find(|part| part.id == protected_local_id)
        .unwrap();
    let rematerialized_retained = rematerialized
        .parts
        .iter()
        .find(|part| part.id == protected_local_id)
        .unwrap();
    for pose in [&actual_retained.pose, &rematerialized_retained.pose] {
        assert!((pose.at.x - 0.0).abs() < 1e-9);
        assert!((pose.at.y - 0.0).abs() < 1e-9);
        assert!((pose.rotation - 0.0).abs() < 1e-9);
    }
    let mut actual_non_pose = serde_json::to_value(actual_retained).unwrap();
    let mut rematerialized_non_pose = serde_json::to_value(rematerialized_retained).unwrap();
    actual_non_pose.as_object_mut().unwrap().remove("pose");
    rematerialized_non_pose
        .as_object_mut()
        .unwrap()
        .remove("pose");
    assert_eq!(
        actual_non_pose, rematerialized_non_pose,
        "the retained member's identity and non-pose data must agree with rematerialization"
    );
    assert_eq!(
        serde_json::to_value(
            document
                .parts
                .iter()
                .find(|part| part.id == protected_local_id)
                .unwrap()
        )
        .unwrap(),
        entry_local_json
    );
    assert_eq!(
        serde_json::json!({
            "boards": &document.boards,
            "nets": &document.nets,
            "constraints": &document.constraints,
            "boardOutlines": &document.board_outlines,
            "definitions": &document.definitions,
            "outline": &document.outline,
        }),
        protected_references
    );

    let actual_final_owners: BTreeMap<_, _> = final_owners
        .keys()
        .map(|id| {
            let owner = document
                .boards
                .iter()
                .find(|board| board.part_ids.iter().any(|part_id| part_id == id))
                .map(|board| board.id.clone())
                .unwrap();
            (id.clone(), owner)
        })
        .collect();
    assert_eq!(actual_final_owners, final_owners);

    let mut expected_changed_ids = BTreeSet::new();
    if serde_json::to_value(&expected_mirror_matrix).unwrap()
        != serde_json::to_value(&entry_mirror_matrix).unwrap()
    {
        expected_changed_ids.insert(entry_mirror_matrix.id.clone());
    }
    let before = entry
        .parts
        .iter()
        .find(|part| part.id == linked_mirror_id)
        .unwrap();
    let after = expected_document
        .parts
        .iter()
        .find(|part| part.id == linked_mirror_id)
        .unwrap();
    if serde_json::to_value(before).unwrap() != serde_json::to_value(after).unwrap() {
        expected_changed_ids.insert(linked_mirror_id.into());
    }
    assert_eq!(
        changed_ids,
        expected_changed_ids.into_iter().collect::<Vec<_>>()
    );
    let before_second_reconcile = document.clone();
    assert!(
        reconcile(&mut document, "pair-source-mirror", &projection)
            .unwrap()
            .is_empty()
    );
    assert_eq!(document, before_second_reconcile);

    let before_rejected = serde_json::to_value(&document).unwrap();
    let unsupported = Projection {
        matrix: projection.matrix.clone(),
        preserve_matrix_geometry: projection.preserve_matrix_geometry,
        source_board_id: projection.source_board_id.clone(),
        mirror_board_id: projection.mirror_board_id.clone(),
        mapping: projection.mapping,
        parts: vec![],
        independent_part_transitions: vec![],
        hosted_transitions: vec![],
        applied_independent_additions: std::cell::RefCell::new(BTreeSet::new()),
        applied_hosted_operations: std::cell::RefCell::new(BTreeSet::new()),
        applied_hosted_circuit_additions: std::cell::RefCell::new(BTreeMap::new()),
        hosted_removal_baselines: std::cell::RefCell::new(BTreeMap::new()),
        hosted_omission_baselines: std::cell::RefCell::new(BTreeMap::new()),
        outline_removal_baselines: std::cell::RefCell::new(BTreeMap::new()),
        locally_protected_mirror_parts: projection.locally_protected_mirror_parts.clone(),
        final_owners: projection.final_owners.clone(),
    };
    assert!(reconcile(&mut document, "pair-source-mirror", &unsupported).is_err());
    assert_eq!(serde_json::to_value(&document).unwrap(), before_rejected);

    let mismatched_pair = Projection {
        matrix: projection.matrix.clone(),
        preserve_matrix_geometry: projection.preserve_matrix_geometry,
        mapping: Pose2 {
            at: projection.mapping.at,
            rotation: projection.mapping.rotation + 1.0,
        },
        source_board_id: projection.source_board_id.clone(),
        mirror_board_id: projection.mirror_board_id.clone(),
        parts: projection
            .parts
            .iter()
            .map(|part| PartProjection {
                source_id: part.source_id.clone(),
                mirror_id: part.mirror_id.clone(),
            })
            .collect(),
        independent_part_transitions: vec![],
        hosted_transitions: vec![],
        applied_independent_additions: std::cell::RefCell::new(BTreeSet::new()),
        applied_hosted_operations: std::cell::RefCell::new(BTreeSet::new()),
        applied_hosted_circuit_additions: std::cell::RefCell::new(BTreeMap::new()),
        hosted_removal_baselines: std::cell::RefCell::new(BTreeMap::new()),
        hosted_omission_baselines: std::cell::RefCell::new(BTreeMap::new()),
        outline_removal_baselines: std::cell::RefCell::new(BTreeMap::new()),
        locally_protected_mirror_parts: projection.locally_protected_mirror_parts.clone(),
        final_owners: projection.final_owners.clone(),
    };
    assert!(reconcile(&mut document, "pair-source-mirror", &mismatched_pair).is_err());
    assert_eq!(serde_json::to_value(&document).unwrap(), before_rejected);
}

#[test]
fn reconcile_projects_associated_independent_parts_and_preserves_fixed_layouts() {
    let (mut document, mut projection) = fixed_geometry_fixture().unwrap();
    let mut source = document
        .parts
        .iter()
        .find(|part| part.id == "matrix/source-matrix/r0c1")
        .unwrap()
        .clone();
    source.id = "independent-source".into();
    source.reference = "J1".into();
    source.pose = Pose2 {
        at: Vec2 { x: 10.0, y: 15.0 },
        rotation: 25.0,
    };
    source.keycap = Some(Vec2 { x: 22.0, y: 18.0 });
    source.outline = Some(PartOutline {
        excluded: true,
        margin: Some(3.5),
        allow_body_overhang: true,
    });
    source.side = Side::Back;
    source.locked = Some(false);
    source.properties = Some(BTreeMap::from([
        (
            "custom".into(),
            serde_json::json!({"board":"source","revision":7}),
        ),
        ("userFlag".into(), serde_json::json!(true)),
    ]));
    source.generator_parameters = Some(BTreeMap::from([
        ("variant".into(), serde_json::json!("non-default")),
        ("width".into(), serde_json::json!(2.25)),
    ]));
    let mut target = source.clone();
    target.id = "independent-mirror".into();
    target.definition_id = "mirror-switch".into();
    target.reference = "J9".into();
    target.pose = Pose2 {
        at: Vec2::default(),
        rotation: 0.0,
    };
    target.keycap = None;
    target.outline = None;
    target.side = Side::Front;
    target.locked = Some(true);
    target.properties = Some(BTreeMap::from([(
        "local".into(),
        serde_json::json!("target"),
    )]));
    target.generator_parameters = Some(BTreeMap::from([("local".into(), serde_json::json!(9))]));
    let mut target_definition = document.definitions[0].clone();
    target_definition.id = "mirror-switch".into();
    target_definition.name = "Mirror switch".into();
    document.definitions.push(target_definition);
    document.parts.extend([source.clone(), target]);
    let mut unrelated_part = document
        .parts
        .iter()
        .find(|part| part.id == "geometry-probe")
        .unwrap()
        .clone();
    unrelated_part.id = "unrelated-part".into();
    unrelated_part.reference = "LOCAL1".into();
    document.parts.push(unrelated_part);
    let mut added_source = source.clone();
    added_source.id = "independent-source-added".into();
    added_source.reference = "J2".into();
    added_source.pose.at = Vec2 { x: -8.0, y: 6.0 };
    document.parts.push(added_source.clone());
    document
        .boards
        .iter_mut()
        .find(|board| board.id == "source-board")
        .unwrap()
        .part_ids
        .push(source.id.clone());
    document
        .boards
        .iter_mut()
        .find(|board| board.id == "source-board")
        .unwrap()
        .part_ids
        .push(added_source.id.clone());
    document
        .boards
        .iter_mut()
        .find(|board| board.id == "mirror-board")
        .unwrap()
        .part_ids
        .push("independent-mirror".into());
    document
        .boards
        .iter_mut()
        .find(|board| board.id == "mirror-board")
        .unwrap()
        .part_ids
        .push("unrelated-part".into());

    let halves = document.hardware.as_mut().unwrap().pairs[0]
        .halves
        .as_mut()
        .unwrap();
    halves.correspondences.extend([
        MirroredCorrespondence {
            companion_mode: None,
            companion_tweak: None,
            kind: MirroredItemKind::Part,
            source_id: "independent-source".into(),
            mirror_id: "independent-mirror".into(),
        },
        MirroredCorrespondence {
            companion_mode: None,
            companion_tweak: None,
            kind: MirroredItemKind::Layout,
            source_id: "source-layout".into(),
            mirror_id: "mirror-layout".into(),
        },
    ]);
    let source_layout = Layout {
        id: "source-layout".into(),
        name: "Source layout".into(),
        board_id: "source-board".into(),
        matrix_ids: vec!["source-matrix".into()],
        part_ids: vec!["independent-source".into()],
        mirror_link: None,
    };
    let mirror_layout = Layout {
        id: "mirror-layout".into(),
        name: "My target layout".into(),
        board_id: "mirror-board".into(),
        matrix_ids: vec!["mirror-matrix".into()],
        part_ids: vec!["geometry-probe".into(), "independent-mirror".into()],
        mirror_link: None,
    };
    projection.independent_part_transitions = vec![IndependentPartTransition::Add {
        source_id: "independent-source-added".into(),
        mirror_id: "independent-mirror-added".into(),
        mirror_definition_id: "mirror-switch".into(),
        mirror_reference: "J99".into(),
    }];
    boardstudio_core_kernel::matrix::set_matrix(
        &mut document,
        &fixture_matrix("unrelated-matrix", "mirror-board"),
    )
    .unwrap();
    let unrelated_layout = Layout {
        id: "unrelated-layout".into(),
        name: "Unrelated local layout".into(),
        board_id: "mirror-board".into(),
        matrix_ids: vec!["unrelated-matrix".into()],
        part_ids: vec!["unrelated-part".into()],
        mirror_link: None,
    };
    document.layouts = vec![
        source_layout.clone(),
        mirror_layout.clone(),
        unrelated_layout.clone(),
    ];

    assert!(
        document
            .parts
            .iter()
            .all(|part| part.id != "independent-mirror-added")
    );
    assert!(document.parts.iter().all(|part| part.reference != "J99"));

    crate::support::apply_for_test(
        &mut document,
        &[],
        &boardstudio_model::model::EditOperation::SetLayout {
            layout: Layout {
                part_ids: vec![
                    "independent-source-added".into(),
                    "independent-source".into(),
                ],
                ..source_layout
            },
        },
    )
    .expect("the ordinary source SetLayout operation records its membership change");

    boardstudio_core_mechanical::mechanical::halves::validate_document(&document).unwrap();
    let source_before = document
        .parts
        .iter()
        .find(|part| part.id == "independent-source")
        .unwrap()
        .clone();
    let unrelated_part_before = document
        .parts
        .iter()
        .find(|part| part.id == "unrelated-part")
        .unwrap()
        .clone();
    let unrelated_owner_ids_before: Vec<_> = document
        .boards
        .iter()
        .filter(|board| board.part_ids.contains(&"unrelated-part".to_owned()))
        .map(|board| board.id.clone())
        .collect();
    let original = document.clone();

    let valid_addition_evidence = projection.independent_part_transitions.clone();
    let mut invalid_addition_outcomes = Vec::new();

    let mut unrelated_projection = projection.clone();
    unrelated_projection.independent_part_transitions[0] = IndependentPartTransition::Add {
        source_id: "independent-source-added".into(),
        mirror_id: "unrelated-part".into(),
        mirror_definition_id: unrelated_part_before.definition_id.clone(),
        mirror_reference: unrelated_part_before.reference.clone(),
    };
    let mut unrelated_target = original.clone();
    let unrelated_target_before = unrelated_target.clone();
    let unrelated_result = reconcile(
        &mut unrelated_target,
        "pair-source-mirror",
        &unrelated_projection,
    );
    invalid_addition_outcomes.push((
        "first add cannot adopt an unrelated mirror-owned target",
        unrelated_result.is_err(),
        unrelated_target == unrelated_target_before,
    ));

    let mut two_sources = original.clone();
    let mut second_source = added_source.clone();
    second_source.id = "independent-source-outside-layout".into();
    second_source.reference = "Q77".into();
    second_source.pose = Pose2 {
        at: Vec2 { x: 31.0, y: -17.0 },
        rotation: 71.0,
    };
    second_source.properties = Some(BTreeMap::from([(
        "sibling".into(),
        serde_json::json!("distinct payload"),
    )]));
    assert_ne!(second_source.pose, added_source.pose);
    assert_ne!(second_source.properties, added_source.properties);
    two_sources.parts.push(second_source.clone());
    two_sources
        .boards
        .iter_mut()
        .find(|board| board.id == "source-board")
        .unwrap()
        .part_ids
        .push(second_source.id.clone());
    boardstudio_core_mechanical::mechanical::halves::validate_document(&two_sources)
        .expect("the sibling source outside the paired Layout is valid");
    assert!(
        two_sources.layouts[0]
            .part_ids
            .iter()
            .all(|id| id != &second_source.id)
    );
    assert!(
        two_sources.parts.iter().all(|part| {
            part.id == "independent-source-outside-layout" || part.reference != "Q91"
        })
    );
    assert!(
        two_sources.parts.iter().all(|part| {
            part.id == "independent-source-outside-layout" || part.reference != "Q92"
        })
    );
    assert!(two_sources.parts.iter().all(|part| {
        !matches!(
            part.id.as_str(),
            "planned-independent-target"
                | "planned-independent-target-a"
                | "planned-independent-target-b"
        )
    }));

    let mut aliased_id_projection = projection.clone();
    aliased_id_projection.independent_part_transitions = vec![
        IndependentPartTransition::Add {
            source_id: "independent-source-added".into(),
            mirror_id: "planned-independent-target".into(),
            mirror_definition_id: "mirror-switch".into(),
            mirror_reference: "Q91".into(),
        },
        IndependentPartTransition::Add {
            source_id: second_source.id.clone(),
            mirror_id: "planned-independent-target".into(),
            mirror_definition_id: "mirror-switch".into(),
            mirror_reference: "Q91".into(),
        },
    ];
    let mut aliased_target_id = two_sources.clone();
    let aliased_target_id_before = aliased_target_id.clone();
    let aliased_id_result = reconcile(
        &mut aliased_target_id,
        "pair-source-mirror",
        &aliased_id_projection,
    );
    invalid_addition_outcomes.push((
        "sibling additions cannot select the same fresh target ID and reference",
        aliased_id_result.is_err(),
        aliased_target_id == aliased_target_id_before,
    ));

    let mut aliased_reference_projection = projection.clone();
    aliased_reference_projection.independent_part_transitions = vec![
        IndependentPartTransition::Add {
            source_id: "independent-source-added".into(),
            mirror_id: "planned-independent-target-a".into(),
            mirror_definition_id: "mirror-switch".into(),
            mirror_reference: "Q92".into(),
        },
        IndependentPartTransition::Add {
            source_id: second_source.id,
            mirror_id: "planned-independent-target-b".into(),
            mirror_definition_id: "mirror-switch".into(),
            mirror_reference: "Q92".into(),
        },
    ];
    let mut aliased_target_reference = two_sources.clone();
    let aliased_target_reference_before = aliased_target_reference.clone();
    let aliased_reference_result = reconcile(
        &mut aliased_target_reference,
        "pair-source-mirror",
        &aliased_reference_projection,
    );
    invalid_addition_outcomes.push((
        "sibling additions cannot select distinct targets with one fresh reference",
        aliased_reference_result.is_err(),
        aliased_target_reference == aliased_target_reference_before,
    ));
    assert!(
        invalid_addition_outcomes
            .iter()
            .all(|(_, refused, unchanged)| *refused && *unchanged),
        "invalid independent Add evidence must refuse atomically: {invalid_addition_outcomes:?}"
    );

    projection.independent_part_transitions = valid_addition_evidence.clone();
    projection.independent_part_transitions[0] = IndependentPartTransition::Add {
        source_id: "independent-source-added".into(),
        mirror_id: "independent-mirror-added".into(),
        mirror_definition_id: "missing-mirror-definition".into(),
        mirror_reference: "J99".into(),
    };
    let mut unavailable_definition = original.clone();
    let refused = reconcile(
        &mut unavailable_definition,
        "pair-source-mirror",
        &projection,
    )
    .unwrap_err();
    assert!(refused.contains("target definition"));
    assert_eq!(unavailable_definition, original);

    projection.independent_part_transitions[0] = IndependentPartTransition::Add {
        source_id: "independent-source-added".into(),
        mirror_id: "independent-mirror-added".into(),
        mirror_definition_id: "mirror-switch".into(),
        mirror_reference: "LOCAL1".into(),
    };
    let mut conflicting_reference = original.clone();
    let refused = reconcile(
        &mut conflicting_reference,
        "pair-source-mirror",
        &projection,
    )
    .unwrap_err();
    assert!(refused.contains("reference") || refused.contains("already in use"));
    assert_eq!(conflicting_reference, original);

    projection.independent_part_transitions = valid_addition_evidence.clone();
    let mut stale_source_owner = original.clone();
    stale_source_owner
        .boards
        .iter_mut()
        .find(|board| board.id == "source-board")
        .unwrap()
        .part_ids
        .retain(|id| id != "independent-source-added");
    stale_source_owner
        .boards
        .iter_mut()
        .find(|board| board.id == "mirror-board")
        .unwrap()
        .part_ids
        .push("independent-source-added".into());
    let stale_source_owner_before = stale_source_owner.clone();
    let refused =
        reconcile(&mut stale_source_owner, "pair-source-mirror", &projection).unwrap_err();
    assert!(refused.contains("stale source ownership"));
    assert_eq!(stale_source_owner, stale_source_owner_before);

    let changed = reconcile(&mut document, "pair-source-mirror", &projection).unwrap();
    assert_eq!(
        changed,
        vec![
            "independent-mirror",
            "independent-mirror-added",
            "matrix/mirror-matrix/r0c1",
            "matrix/mirror-matrix/r0c2",
            "matrix/mirror-matrix/r1c0",
            "matrix/mirror-matrix/r1c2",
            "mirror-board",
            "mirror-layout",
            "mirror-matrix",
        ]
    );
    let mirrored = document
        .parts
        .iter()
        .find(|part| part.id == "independent-mirror")
        .unwrap();
    assert_eq!(mirrored.id, "independent-mirror");
    assert_eq!(mirrored.definition_id, "mirror-switch");
    assert_eq!(mirrored.reference, "J9");
    assert_eq!(mirrored.keycap, source.keycap);
    assert_eq!(mirrored.outline, source.outline);
    assert_eq!(mirrored.side, source.side);
    assert_eq!(mirrored.locked, source.locked);
    assert_eq!(mirrored.properties, source.properties);
    assert_eq!(mirrored.generator_parameters, source.generator_parameters);
    assert_eq!(
        mirrored.pose,
        Pose2 {
            at: Vec2 { x: 85.0, y: -40.0 },
            rotation: 245.0,
        }
    );
    assert!(changed.contains(&"independent-mirror".to_owned()));
    assert!(changed.contains(&"independent-mirror-added".to_owned()));
    let added_mirror = document
        .parts
        .iter()
        .find(|part| part.id == "independent-mirror-added")
        .expect("the explicit independent addition creates its selected target ID");
    assert_eq!(added_mirror.definition_id, "mirror-switch");
    assert_eq!(added_mirror.reference, "J99");
    assert_eq!(added_mirror.keycap, added_source.keycap);
    assert_eq!(added_mirror.outline, added_source.outline);
    assert_eq!(added_mirror.side, added_source.side);
    assert_eq!(added_mirror.locked, added_source.locked);
    assert_eq!(added_mirror.properties, added_source.properties);
    assert_eq!(
        added_mirror.generator_parameters,
        added_source.generator_parameters
    );
    assert_eq!(
        added_mirror.pose,
        Pose2 {
            at: Vec2 { x: 94.0, y: -22.0 },
            rotation: 245.0,
        }
    );
    assert!(
        document
            .boards
            .iter()
            .any(|board| board.id == "mirror-board"
                && board.part_ids.contains(&"independent-mirror-added".into()))
    );
    assert_eq!(
        document
            .parts
            .iter()
            .find(|part| part.id == "independent-source")
            .unwrap(),
        &source_before
    );
    assert_eq!(document.layouts[1].id, mirror_layout.id);
    assert_eq!(document.layouts[1].name, mirror_layout.name);
    assert_eq!(document.layouts[1].board_id, mirror_layout.board_id);
    assert_eq!(document.layouts[1].matrix_ids, mirror_layout.matrix_ids);
    assert_eq!(document.layouts[1].mirror_link, mirror_layout.mirror_link);
    assert_eq!(
        document.layouts[1].part_ids,
        vec![
            "independent-mirror-added",
            "independent-mirror",
            "geometry-probe"
        ]
    );
    assert!(changed.contains(&"mirror-layout".to_owned()));
    assert_eq!(document.layouts[2], unrelated_layout);
    assert_eq!(
        document
            .parts
            .iter()
            .find(|part| part.id == "unrelated-part")
            .unwrap(),
        &unrelated_part_before
    );
    let unrelated_owner_ids_after: Vec<_> = document
        .boards
        .iter()
        .filter(|board| board.part_ids.contains(&"unrelated-part".to_owned()))
        .map(|board| board.id.clone())
        .collect();
    assert_eq!(unrelated_owner_ids_after, unrelated_owner_ids_before);

    let mut repeated_from_same_input = original.clone();
    reconcile(
        &mut repeated_from_same_input,
        "pair-source-mirror",
        &projection,
    )
    .unwrap();
    assert_eq!(repeated_from_same_input, document);
    let first_output = document.clone();
    let applied_additions_before_repeat = projection.applied_independent_additions.borrow().clone();
    let second_changed = reconcile(&mut document, "pair-source-mirror", &projection).unwrap();
    assert!(second_changed.is_empty());
    assert_eq!(document, first_output);
    assert_eq!(
        *projection.applied_independent_additions.borrow(),
        applied_additions_before_repeat,
        "updated private Add provenance converges with the repeated output"
    );

    let mut removal_source = document.clone();
    removal_source.nets.push(boardstudio_model::model::Net {
        id: "independent-removal-net".into(),
        name: "Independent removal control".into(),
        pins: vec![
            boardstudio_model::model::Pin {
                part_id: "independent-mirror-added".into(),
                pad_id: "pad-two".into(),
            },
            boardstudio_model::model::Pin {
                part_id: "geometry-probe".into(),
                pad_id: "pad-two".into(),
            },
        ],
    });
    removal_source
        .boards
        .iter_mut()
        .find(|board| board.id == "mirror-board")
        .unwrap()
        .net_ids
        .push("independent-removal-net".into());
    removal_source
        .constraints
        .push(boardstudio_model::model::Constraint::Offset {
            id: "independent-removal-offset".into(),
            source_part_id: "independent-mirror-added".into(),
            target_part_id: "geometry-probe".into(),
            offset: Vec2 { x: 4.0, y: -2.0 },
            rotation: 5.0,
        });
    let mut expected_retained_keys = removal_source
        .outline
        .iter()
        .find_map(|feature| match feature {
            OutlineFeature::PartEnvelope { id, part_ids, .. } if id == "retained-keys-envelope" => {
                Some(part_ids.clone())
            }
            _ => None,
        })
        .expect("the baseline carries a registered retained-key envelope");
    expected_retained_keys.push("independent-mirror-added".into());
    if let Some(OutlineFeature::PartEnvelope { part_ids, .. }) =
        removal_source.outline.iter_mut().find(|feature| {
            matches!(feature, OutlineFeature::PartEnvelope { id, .. }
            if id == "retained-keys-envelope")
        })
    {
        part_ids.push("independent-mirror-added".into());
    }
    crate::support::apply_for_test(
        &mut removal_source,
        &[],
        &boardstudio_model::model::EditOperation::RemoveParts {
            ids: vec!["independent-source-added".into()],
        },
    )
    .unwrap();
    assert!(
        removal_source
            .parts
            .iter()
            .all(|part| part.id != "independent-source-added")
    );
    assert!(removal_source.outline.iter().any(|feature| {
        matches!(feature, OutlineFeature::PartEnvelope { id, part_ids, .. }
            if id == "retained-keys-envelope"
                && part_ids.contains(&"independent-mirror-added".into()))
    }));
    projection.independent_part_transitions = vec![IndependentPartTransition::Remove {
        source_id: "independent-source-added".into(),
        mirror_id: "independent-mirror-added".into(),
    }];
    let persisted_pair_before = removal_source.hardware.as_ref().unwrap().pairs[0]
        .halves
        .as_ref()
        .unwrap()
        .correspondences
        .clone();
    let mut refused_removal = removal_source.clone();
    refused_removal.layouts[1]
        .part_ids
        .push("independent-mirror".into());
    let refusal_before = refused_removal.clone();
    let refusal = reconcile(&mut refused_removal, "pair-source-mirror", &projection).unwrap_err();
    assert!(refusal.contains("paired layout contains duplicate part membership"));
    assert_eq!(refused_removal, refusal_before);

    projection.independent_part_transitions = vec![IndependentPartTransition::Remove {
        source_id: "independent-source-added".into(),
        mirror_id: "independent-mirror".into(),
    }];
    let mut conflicting_removal_identity = removal_source.clone();
    let conflicting_removal_before = conflicting_removal_identity.clone();
    let refusal = reconcile(
        &mut conflicting_removal_identity,
        "pair-source-mirror",
        &projection,
    )
    .unwrap_err();
    assert!(refusal.contains("conflicting pair provenance"));
    assert_eq!(conflicting_removal_identity, conflicting_removal_before);
    projection.independent_part_transitions = vec![IndependentPartTransition::Remove {
        source_id: "independent-source-added".into(),
        mirror_id: "independent-mirror-added".into(),
    }];

    let removed = reconcile(&mut removal_source, "pair-source-mirror", &projection).unwrap();
    assert!(
        removal_source
            .parts
            .iter()
            .all(|part| part.id != "independent-mirror-added")
    );
    assert!(
        removal_source
            .boards
            .iter()
            .all(|board| !board.part_ids.contains(&"independent-mirror-added".into()))
    );
    assert!(
        removal_source.layouts[1]
            .part_ids
            .iter()
            .all(|id| id != "independent-mirror-added")
    );
    assert_eq!(
        removal_source
            .nets
            .iter()
            .find(|net| net.id == "independent-removal-net")
            .unwrap()
            .pins,
        vec![boardstudio_model::model::Pin {
            part_id: "geometry-probe".into(),
            pad_id: "pad-two".into(),
        }]
    );
    assert!(
        removal_source
            .constraints
            .iter()
            .all(|constraint| constraint.id() != "independent-removal-offset")
    );
    expected_retained_keys.retain(|id| id != "independent-mirror-added");
    assert!(removal_source.outline.iter().any(|feature| {
        matches!(feature, OutlineFeature::PartEnvelope { id, part_ids, .. }
            if id == "retained-keys-envelope" && part_ids == &expected_retained_keys)
    }));
    assert_eq!(
        removed,
        vec![
            "independent-mirror-added",
            "independent-removal-net",
            "independent-removal-offset",
            "mirror-board",
            "mirror-layout",
            "retained-keys-envelope",
        ]
    );
    assert_eq!(
        removal_source.hardware.as_ref().unwrap().pairs[0]
            .halves
            .as_ref()
            .unwrap()
            .correspondences,
        persisted_pair_before
    );
    let first_removal_output = removal_source.clone();
    let repeated_removal =
        reconcile(&mut removal_source, "pair-source-mirror", &projection).unwrap();
    assert!(repeated_removal.is_empty());
    assert_eq!(removal_source, first_removal_output);
}

#[test]
fn source_layout_removal_prunes_linked_projection_and_keeps_local_and_unpaired_members() {
    let (mut document, projection) = fixed_geometry_fixture().unwrap();
    let source_linked = "layout-linked-source";
    let mirror_linked = "layout-linked-mirror";
    let source_local = "layout-local-source";
    let mirror_local = "layout-local-mirror";
    let mut linked_source_part = document
        .parts
        .iter()
        .find(|part| part.id == "matrix/source-matrix/r0c1")
        .unwrap()
        .clone();
    linked_source_part.id = source_linked.into();
    linked_source_part.reference = "LINKED-SOURCE".into();
    let mut linked_mirror_part = document
        .parts
        .iter()
        .find(|part| part.id == "matrix/mirror-matrix/r0c1")
        .unwrap()
        .clone();
    linked_mirror_part.id = mirror_linked.into();
    linked_mirror_part.reference = "LINKED-MIRROR".into();
    let mut local_source_part = linked_source_part.clone();
    local_source_part.id = source_local.into();
    local_source_part.reference = "LOCAL-SOURCE".into();
    let mut local_mirror_part = local_source_part.clone();
    local_mirror_part.id = mirror_local.into();
    local_mirror_part.reference = "LOCAL-MIRROR".into();
    document.parts.extend([
        linked_source_part,
        linked_mirror_part,
        local_source_part,
        local_mirror_part,
    ]);
    document
        .boards
        .iter_mut()
        .find(|board| board.id == "source-board")
        .unwrap()
        .part_ids
        .extend([source_linked.into(), source_local.into()]);
    document
        .boards
        .iter_mut()
        .find(|board| board.id == "mirror-board")
        .unwrap()
        .part_ids
        .extend([mirror_linked.into(), mirror_local.into()]);

    let halves = document.hardware.as_mut().unwrap().pairs[0]
        .halves
        .as_mut()
        .unwrap();
    halves.correspondences.extend([
        MirroredCorrespondence {
            companion_mode: None,
            companion_tweak: None,
            kind: MirroredItemKind::Part,
            source_id: source_linked.into(),
            mirror_id: mirror_linked.into(),
        },
        MirroredCorrespondence {
            companion_mode: None,
            companion_tweak: None,
            kind: MirroredItemKind::Part,
            source_id: source_local.into(),
            mirror_id: mirror_local.into(),
        },
        MirroredCorrespondence {
            companion_mode: None,
            companion_tweak: None,
            kind: MirroredItemKind::Layout,
            source_id: "source-layout-removal".into(),
            mirror_id: "mirror-layout-removal".into(),
        },
    ]);
    halves.exceptions.push(
        serde_json::from_value(serde_json::json!({
            "kind": "part", "mirrorId": mirror_local, "state": "local"
        }))
        .unwrap(),
    );

    let source_layout = Layout {
        id: "source-layout-removal".into(),
        name: "Source layout".into(),
        board_id: "source-board".into(),
        matrix_ids: vec!["source-matrix".into()],
        part_ids: vec![source_linked.into(), source_local.into()],
        mirror_link: None,
    };
    let mirror_layout = Layout {
        id: "mirror-layout-removal".into(),
        name: "Mirror layout".into(),
        board_id: "mirror-board".into(),
        matrix_ids: vec!["mirror-matrix".into()],
        part_ids: vec![
            mirror_linked.into(),
            mirror_local.into(),
            "geometry-probe".into(),
        ],
        mirror_link: None,
    };
    document.layouts = vec![source_layout.clone(), mirror_layout];
    boardstudio_core_mechanical::mechanical::halves::validate_document(&document).unwrap();

    let request = boardstudio_model::model::EditOperation::SetLayout {
        layout: Layout {
            part_ids: vec![mirror_local.into(), "geometry-probe".into()],
            ..document.layouts[1].clone()
        },
    };
    let canonical = crate::support::canonicalize(&document, &request)
        .expect("the mirror request maps into source membership");
    let expected_source = boardstudio_model::model::EditOperation::SetLayout {
        layout: Layout {
            part_ids: vec![source_local.into()],
            ..source_layout
        },
    };
    assert_eq!(canonical.operation, expected_source);
    crate::support::apply_for_test(&mut document, &[], &canonical.operation)
        .expect("the translated source layout removes its linked member");
    reconcile(&mut document, "pair-source-mirror", &projection)
        .expect("the removal and retained target members reconcile");

    assert_eq!(
        document.layouts[1].part_ids,
        vec![mirror_local.to_owned(), "geometry-probe".to_owned()]
    );
}

#[test]
fn mirror_layout_reorder_survives_source_apply_and_reconciliation() {
    let (mut document, projection) = fixed_geometry_fixture().unwrap();
    let source_header = "layout-header-source";
    let source_u1 = "layout-u1-source";
    let mirror_header = "layout-header-mirror";
    let mirror_u1 = "layout-u1-mirror";
    let mut source_header_part = document
        .parts
        .iter()
        .find(|part| part.id == "matrix/source-matrix/r0c0")
        .unwrap()
        .clone();
    source_header_part.id = source_header.into();
    source_header_part.reference = "LAYOUT-HEADER-SOURCE".into();
    let mut source_u1_part = document
        .parts
        .iter()
        .find(|part| part.id == "matrix/source-matrix/r0c1")
        .unwrap()
        .clone();
    source_u1_part.id = source_u1.into();
    source_u1_part.reference = "LAYOUT-U1-SOURCE".into();
    let mut mirror_header_part = document
        .parts
        .iter()
        .find(|part| part.id == "matrix/mirror-matrix/r0c0")
        .unwrap()
        .clone();
    mirror_header_part.id = mirror_header.into();
    mirror_header_part.reference = "LAYOUT-HEADER-MIRROR".into();
    let mut mirror_u1_part = document
        .parts
        .iter()
        .find(|part| part.id == "matrix/mirror-matrix/r0c1")
        .unwrap()
        .clone();
    mirror_u1_part.id = mirror_u1.into();
    mirror_u1_part.reference = "LAYOUT-U1-MIRROR".into();
    document.parts.extend([
        source_header_part,
        source_u1_part,
        mirror_header_part,
        mirror_u1_part,
    ]);
    document
        .boards
        .iter_mut()
        .find(|board| board.id == "source-board")
        .unwrap()
        .part_ids
        .extend([source_header.into(), source_u1.into()]);
    document
        .boards
        .iter_mut()
        .find(|board| board.id == "mirror-board")
        .unwrap()
        .part_ids
        .extend([mirror_header.into(), mirror_u1.into()]);
    document.hardware.as_mut().unwrap().pairs[0]
        .halves
        .as_mut()
        .unwrap()
        .correspondences
        .extend([
            MirroredCorrespondence {
                companion_mode: None,
                companion_tweak: None,
                kind: MirroredItemKind::Part,
                source_id: source_header.into(),
                mirror_id: mirror_header.into(),
            },
            MirroredCorrespondence {
                companion_mode: None,
                companion_tweak: None,
                kind: MirroredItemKind::Part,
                source_id: source_u1.into(),
                mirror_id: mirror_u1.into(),
            },
        ]);
    document.hardware.as_mut().unwrap().pairs[0]
        .halves
        .as_mut()
        .unwrap()
        .exceptions
        .push(
            serde_json::from_value(serde_json::json!({
                "kind": "part", "mirrorId": mirror_u1, "state": "local"
            }))
            .unwrap(),
        );
    let source_layout = Layout {
        id: "source-layout-reorder".into(),
        name: "Source layout".into(),
        board_id: "source-board".into(),
        matrix_ids: vec!["source-matrix".into()],
        part_ids: vec![source_header.into(), source_u1.into()],
        mirror_link: None,
    };
    let mirror_layout = Layout {
        id: "mirror-layout-reorder".into(),
        name: "Mirror layout".into(),
        board_id: "mirror-board".into(),
        matrix_ids: vec!["mirror-matrix".into()],
        part_ids: vec![
            mirror_header.into(),
            mirror_u1.into(),
            "geometry-probe".into(),
        ],
        mirror_link: None,
    };
    document.layouts = vec![source_layout.clone(), mirror_layout.clone()];
    document.hardware.as_mut().unwrap().pairs[0]
        .halves
        .as_mut()
        .unwrap()
        .correspondences
        .push(MirroredCorrespondence {
            companion_mode: None,
            companion_tweak: None,
            kind: MirroredItemKind::Layout,
            source_id: source_layout.id.clone(),
            mirror_id: mirror_layout.id.clone(),
        });

    let requested = Layout {
        part_ids: vec![
            mirror_u1.into(),
            mirror_header.into(),
            "geometry-probe".into(),
        ],
        ..mirror_layout
    };
    let canonical = crate::support::canonicalize(
        &document,
        &EditOperation::SetLayout {
            layout: requested.clone(),
        },
    )
    .expect("the reordered mirror request maps to source member order");
    assert_eq!(
        canonical.operation,
        EditOperation::SetLayout {
            layout: Layout {
                part_ids: vec![source_u1.into(), source_header.into()],
                ..source_layout
            },
        }
    );
    crate::support::apply_for_test(&mut document, &[], &canonical.operation)
        .expect("the canonical source layout applies");
    reconcile(&mut document, "pair-source-mirror", &projection)
        .expect("the reordered source layout reconciles");

    assert_eq!(document.layouts[1].part_ids, requested.part_ids);
}
