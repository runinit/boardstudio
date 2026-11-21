use super::*;
use crate::open_and_preview::{
    add_linked_independent_part, add_unpaired_mirror_board_part, framed_offset,
};

#[test]
fn open_preparation_restores_missing_linked_part_from_its_correspondence() {
    let mut accepted = pair_document();
    accepted.parts.retain(|part| part.id != "right/U1");
    accepted
        .boards
        .iter_mut()
        .find(|board| board.id == "right")
        .unwrap()
        .part_ids
        .retain(|id| id != "right/U1");
    let accepted_bytes = serde_json::to_vec(&accepted).unwrap();

    let (prepared, findings) = crate::support::prepare_open(&accepted, &[])
        .expect("a missing linked projection is reconstructed from its source");

    assert_eq!(part(&prepared, "right/U1").pose.at.x, 224.0);
    assert_eq!(findings.len(), 1);
    assert!(findings[0].target_ids.contains(&"right/U1".to_owned()));
    assert_eq!(serde_json::to_vec(&accepted).unwrap(), accepted_bytes);

    let (reopened, second_findings) = crate::support::prepare_open(&prepared, &[])
        .expect("the reconstructed document is a valid second Open input");
    assert_eq!(reopened, prepared);
    assert!(second_findings.is_empty());
}

#[test]
fn open_repairs_a_missing_framed_linked_part_before_validating_its_endpoint() {
    let mut accepted = pair_document();
    add_linked_independent_part(&mut accepted);
    let source = part(&accepted, "left/U1").pose;
    let target = part(&accepted, "right/RST").pose;
    accepted.constraints.push(Constraint::Framed {
        id: "constraint/open-missing-mirror".into(),
        source: ConstraintEndpoint::Part {
            part_id: "left/U1".into(),
        },
        target: linked_endpoint("pair", "left/RST", "right/RST"),
        relation: ConstraintRelation::Offset {
            offset: Vec2 {
                x: target.at.x - source.at.x,
                y: target.at.y - source.at.y,
            },
            rotation: target.rotation - source.rotation,
        },
    });
    accepted.parts.retain(|item| item.id != "right/RST");
    accepted
        .boards
        .iter_mut()
        .find(|board| board.id == "right")
        .unwrap()
        .part_ids
        .retain(|id| id != "right/RST");

    let (prepared, findings) = crate::support::prepare_open(&accepted, &[])
        .expect("the narrow independent-Part repair precedes final endpoint validation");

    assert!(prepared.parts.iter().any(|item| item.id == "right/RST"));
    assert_eq!(findings.len(), 1);
    assert!(findings[0].target_ids.contains(&"right/RST".to_owned()));
    let (reopened, second_findings) = crate::support::prepare_open(&prepared, &[])
        .expect("the repaired framed document is stable on a second Open");
    assert_eq!(reopened, prepared);
    assert!(second_findings.is_empty());
}

#[test]
fn open_does_not_repair_a_missing_part_for_a_stale_or_omitted_framed_binding() {
    let mut stale = pair_document();
    add_linked_independent_part(&mut stale);
    stale.constraints.push(framed_offset(
        "constraint/stale-open-binding",
        ConstraintEndpoint::Part {
            part_id: "left/U1".into(),
        },
        ConstraintEndpoint::LinkedMirror {
            pair_id: "wrong-pair".into(),
            source_part_id: "left/RST".into(),
            mirror_part_id: "right/RST".into(),
        },
        Vec2 { x: 236.0, y: 20.0 },
    ));
    stale.parts.retain(|item| item.id != "right/RST");
    stale
        .boards
        .iter_mut()
        .find(|board| board.id == "right")
        .unwrap()
        .part_ids
        .retain(|id| id != "right/RST");
    let stale_error = crate::support::prepare_open(&stale, &[]).unwrap_err();
    assert!(
        stale_error.contains("unknown pair wrong-pair"),
        "{stale_error}"
    );

    let mut omitted = pair_document();
    add_linked_independent_part(&mut omitted);
    omitted.constraints.push(framed_offset(
        "constraint/omitted-open-binding",
        ConstraintEndpoint::Part {
            part_id: "left/U1".into(),
        },
        linked_endpoint("pair", "left/RST", "right/RST"),
        Vec2 { x: 236.0, y: 20.0 },
    ));
    omitted.parts.retain(|item| item.id != "right/RST");
    omitted
        .boards
        .iter_mut()
        .find(|board| board.id == "right")
        .unwrap()
        .part_ids
        .retain(|id| id != "right/RST");
    omitted.hardware.as_mut().unwrap().pairs[0]
        .halves
        .as_mut()
        .unwrap()
        .exceptions
        .push(boardstudio_model::model::MirroredException {
            kind: boardstudio_model::model::MirroredItemKind::Part,
            mirror_id: "right/RST".into(),
            state: boardstudio_model::model::MirroredExceptionState::Omitted,
            companion_mode: None,
            companion_tweak: None,
            removed_copper: None,
        });
    let omitted_error = crate::support::prepare_open(&omitted, &[]).unwrap_err();
    assert!(omitted_error.contains("omitted"), "{omitted_error}");
}

#[test]
fn moving_a_linked_framed_target_persists_its_retuned_binding_and_leaves_legacy_wire_alone() {
    let mut accepted = pair_document();
    add_linked_independent_part(&mut accepted);
    accepted.constraints.push(framed_offset(
        "constraint/retuned",
        ConstraintEndpoint::Part {
            part_id: "left/U1".into(),
        },
        linked_endpoint("pair", "left/RST", "right/RST"),
        Vec2 { x: 236.0, y: 20.0 },
    ));
    add_unpaired_mirror_board_part(&mut accepted);
    let legacy_source = part(&accepted, "left/U1").pose;
    let legacy_target = part(&accepted, "right/FREE").pose;
    accepted.constraints.push(Constraint::Offset {
        id: "constraint/untouched-legacy".into(),
        source_part_id: "left/U1".into(),
        target_part_id: "right/FREE".into(),
        offset: Vec2 {
            x: legacy_target.at.x - legacy_source.at.x,
            y: legacy_target.at.y - legacy_source.at.y,
        },
        rotation: legacy_target.rotation - legacy_source.rotation,
    });
    let untouched_legacy = serde_json::to_value(&accepted.constraints[1]).unwrap();

    let (prepared, _) = prepare_transaction(
        &accepted,
        &[],
        &EditOperation::MoveParts {
            positions: vec![Position {
                id: "right/RST".into(),
                at: Vec2 { x: 220.0, y: 30.0 },
            }],
        },
    )
    .expect("the framed target drag retunes its linked view");

    let Constraint::Framed {
        id,
        target,
        relation: ConstraintRelation::Offset { offset, .. },
        ..
    } = &prepared.constraints[0]
    else {
        panic!("the retuned framed relation remains durable");
    };
    assert_eq!(id, "constraint/retuned");
    assert_eq!(target, &linked_endpoint("pair", "left/RST", "right/RST"));
    assert_eq!(*offset, Vec2 { x: 244.0, y: 25.0 });
    assert_eq!(
        part(&prepared, "right/RST").pose.at,
        Vec2 { x: 220.0, y: 30.0 }
    );
    assert_eq!(
        serde_json::to_value(&prepared.constraints[1]).unwrap(),
        untouched_legacy
    );
}

#[test]
fn linked_companion_target_keeps_the_existing_authored_geometry_refusal() {
    let mut accepted = pair_document();
    let source = part(&accepted, "left/U1").pose;
    let target_id = "matrix/right-keys/r0c0/diode";
    let target = part(&accepted, target_id).pose;
    accepted.constraints.push(Constraint::Framed {
        id: "constraint/deferred-companion".into(),
        source: ConstraintEndpoint::Part {
            part_id: "left/U1".into(),
        },
        target: linked_endpoint("pair", "matrix/left-keys/r0c0/diode", target_id),
        relation: ConstraintRelation::Offset {
            offset: Vec2 {
                x: target.at.x - source.at.x,
                y: target.at.y - source.at.y,
            },
            rotation: target.rotation - source.rotation,
        },
    });
    let desired = Vec2 {
        x: target.at.x + 4.0,
        y: target.at.y - 1.0,
    };
    let refusal = prepare_transaction(
        &accepted,
        &[],
        &EditOperation::MoveParts {
            positions: vec![Position {
                id: target_id.into(),
                at: desired,
            }],
        },
    )
    .unwrap_err();
    assert!(
        refusal.contains("Linked matrix placements cannot retain authored or constrained geometry"),
        "the accepted membership contract continues to refuse a constrained linked companion target: {refusal}"
    );
}

#[test]
fn deferred_companion_source_retune_is_persisted_after_its_live_key_resolves() {
    let mut accepted = pair_document();
    add_unpaired_mirror_board_part(&mut accepted);
    let source_id = "matrix/right-keys/r0c0/diode";
    let source = part(&accepted, source_id).pose;
    let target_id = "right/FREE";
    let target = part(&accepted, target_id).pose;
    accepted.constraints.push(Constraint::Framed {
        id: "constraint/deferred-companion-source".into(),
        source: linked_endpoint("pair", "matrix/left-keys/r0c0/diode", source_id),
        target: ConstraintEndpoint::Part {
            part_id: target_id.into(),
        },
        relation: ConstraintRelation::Offset {
            offset: Vec2 {
                x: target.at.x - source.at.x,
                y: target.at.y - source.at.y,
            },
            rotation: target.rotation - source.rotation,
        },
    });
    let desired = Vec2 {
        x: target.at.x + 4.0,
        y: target.at.y - 1.0,
    };

    let (prepared, _) = prepare_transaction(
        &accepted,
        &[],
        &EditOperation::MoveParts {
            positions: vec![Position {
                id: target_id.into(),
                at: desired,
            }],
        },
    )
    .expect("the deferred companion-source retune binds after its live key resolves");

    let Constraint::Framed {
        source: persisted_source,
        relation: ConstraintRelation::Offset { offset, .. },
        ..
    } = &prepared.constraints[0]
    else {
        panic!("the deferred retune is persisted as Framed");
    };
    assert_eq!(
        persisted_source,
        &linked_endpoint("pair", "matrix/left-keys/r0c0/diode", source_id)
    );
    assert_eq!(
        *offset,
        Vec2 {
            x: target.at.x - source.at.x + 4.0,
            y: target.at.y - source.at.y - 1.0,
        }
    );
    assert_eq!(part(&prepared, target_id).pose.at, desired);
}

#[test]
fn framed_command_part_endpoint_is_normalized_in_the_stored_document() {
    let mut accepted = pair_document();
    add_linked_independent_part(&mut accepted);
    let source = part(&accepted, "left/U1").pose;
    let target = part(&accepted, "right/RST").pose;

    let (prepared, _) = prepare_transaction(
        &accepted,
        &[],
        &EditOperation::SetConstraint {
            constraint: Constraint::Framed {
                id: "constraint/new-framed-command".into(),
                source: ConstraintEndpoint::Part {
                    part_id: "left/U1".into(),
                },
                target: ConstraintEndpoint::Part {
                    part_id: "right/RST".into(),
                },
                relation: ConstraintRelation::Offset {
                    offset: Vec2 {
                        x: target.at.x - source.at.x,
                        y: target.at.y - source.at.y,
                    },
                    rotation: target.rotation - source.rotation,
                },
            },
        },
    )
    .expect("a new command can name the visible linked mirror by Part");

    let Constraint::Framed { target, .. } = &prepared.constraints[0] else {
        panic!("the Framed command remains a durable Framed record");
    };
    assert_eq!(target, &linked_endpoint("pair", "left/RST", "right/RST"));
}

#[test]
fn a_new_framed_command_with_a_wrong_explicit_binding_refuses_atomically() {
    let mut accepted = pair_document();
    add_linked_independent_part(&mut accepted);
    let before = accepted.clone();
    let refusal = prepare_transaction(
        &accepted,
        &[],
        &EditOperation::SetConstraint {
            constraint: framed_offset(
                "constraint/wrong-command-binding",
                ConstraintEndpoint::Part {
                    part_id: "left/U1".into(),
                },
                ConstraintEndpoint::LinkedMirror {
                    pair_id: "wrong-pair".into(),
                    source_part_id: "left/RST".into(),
                    mirror_part_id: "right/RST".into(),
                },
                Vec2 { x: 236.0, y: 20.0 },
            ),
        },
    )
    .unwrap_err();

    assert!(refusal.contains("unknown pair wrong-pair"), "{refusal}");
    assert_eq!(accepted, before);
}

#[test]
fn moving_a_loaded_framed_endpoint_with_a_stale_binding_refuses_without_rebinding() {
    let mut accepted = pair_document();
    add_linked_independent_part(&mut accepted);
    accepted.constraints.push(framed_offset(
        "constraint/stale-move-binding",
        ConstraintEndpoint::Part {
            part_id: "left/U1".into(),
        },
        ConstraintEndpoint::LinkedMirror {
            pair_id: "wrong-pair".into(),
            source_part_id: "left/RST".into(),
            mirror_part_id: "right/RST".into(),
        },
        Vec2 { x: 236.0, y: 20.0 },
    ));
    let before = accepted.clone();

    let refusal = prepare_transaction(
        &accepted,
        &[],
        &EditOperation::MoveParts {
            positions: vec![Position {
                id: "right/RST".into(),
                at: Vec2 { x: 220.0, y: 30.0 },
            }],
        },
    )
    .unwrap_err();

    assert!(refusal.contains("unknown pair wrong-pair"), "{refusal}");
    assert_eq!(accepted, before);
}

#[test]
fn accepted_139_private_final_constraint_validator_rejects_tampered_authority() {
    let mut accepted = pair_document();
    add_linked_independent_part(&mut accepted);
    accepted.constraints.push(framed_offset(
        "constraint/final-postcondition",
        ConstraintEndpoint::Part {
            part_id: "left/U1".into(),
        },
        linked_endpoint("pair", "left/RST", "right/RST"),
        Vec2 { x: 236.0, y: 20.0 },
    ));
    let operation = EditOperation::MoveParts {
        positions: vec![Position {
            id: "right/RST".into(),
            at: Vec2 { x: 212.0, y: 25.0 },
        }],
    };
    let (prepared, _) = prepare_transaction(&accepted, &[], &operation)
        .expect("the complete prepared draft satisfies its framed relation");

    assert!(
        boardstudio_core_mirrored_half::mirrored_half::reconcile::prepare::validate_prepared_draft(
            &accepted, &prepared, &operation
        )
        .is_ok()
    );

    let mut tampered = prepared;
    part_mut(&mut tampered, "left/RST").pose.at.x += 1.0;
    let refusal =
        boardstudio_core_mirrored_half::mirrored_half::reconcile::prepare::validate_prepared_draft(
            &accepted, &tampered, &operation,
        );
    assert!(
        refusal.is_err(),
        "final validation must detect authority drift from its persisted framed relation"
    );
}

#[test]
fn no_op_move_of_a_legacy_constrained_target_does_not_upgrade_its_wire_shape() {
    let mut accepted = pair_document();
    add_unpaired_mirror_board_part(&mut accepted);
    let source = part(&accepted, "left/U1").pose;
    let target = part(&accepted, "right/FREE").pose;
    accepted.constraints.push(Constraint::Offset {
        id: "constraint/legacy-no-op".into(),
        source_part_id: "left/U1".into(),
        target_part_id: "right/FREE".into(),
        offset: Vec2 {
            x: target.at.x - source.at.x,
            y: target.at.y - source.at.y,
        },
        rotation: target.rotation - source.rotation,
    });
    let before = serde_json::to_value(&accepted.constraints).unwrap();
    let target_pose = target.at;

    let (prepared, changed) = prepare_transaction(
        &accepted,
        &[],
        &EditOperation::MoveParts {
            positions: vec![Position {
                id: "right/FREE".into(),
                at: target_pose,
            }],
        },
    )
    .expect("the constrained no-op remains accepted");

    assert_eq!(serde_json::to_value(&prepared.constraints).unwrap(), before);
    assert!(changed.is_empty());
}

#[test]
fn open_preparation_restores_a_missing_part_with_one_expected_board_registration() {
    let mut accepted = pair_document();
    accepted.parts.retain(|part| part.id != "right/U1");
    let accepted_bytes = serde_json::to_vec(&accepted).unwrap();

    let (prepared, findings) = crate::support::prepare_open(&accepted, &[])
        .expect("a unique expected-board registration remains recoverable");

    assert_eq!(part(&prepared, "right/U1").pose.at.x, 224.0);
    assert_eq!(
        prepared
            .boards
            .iter()
            .find(|board| board.id == "right")
            .unwrap()
            .part_ids
            .iter()
            .filter(|id| *id == "right/U1")
            .count(),
        1
    );
    assert_eq!(findings.len(), 1);
    assert!(findings[0].target_ids.contains(&"right/U1".to_owned()));
    assert_eq!(serde_json::to_vec(&accepted).unwrap(), accepted_bytes);
}

#[test]
fn open_preparation_refuses_conflicting_or_repeated_missing_part_registrations() {
    let mut repeated = pair_document();
    repeated.parts.retain(|part| part.id != "right/U1");
    repeated
        .boards
        .iter_mut()
        .find(|board| board.id == "right")
        .unwrap()
        .part_ids
        .push("right/U1".into());
    let repeated_error = crate::support::prepare_open(&repeated, &[]).unwrap_err();
    assert!(repeated_error.contains("conflicting or repeated board registration"));

    let mut misplaced = pair_document();
    misplaced.parts.retain(|part| part.id != "right/U1");
    misplaced
        .boards
        .iter_mut()
        .find(|board| board.id == "right")
        .unwrap()
        .part_ids
        .retain(|id| id != "right/U1");
    misplaced
        .boards
        .iter_mut()
        .find(|board| board.id == "left")
        .unwrap()
        .part_ids
        .push("right/U1".into());
    let misplaced_error = crate::support::prepare_open(&misplaced, &[]).unwrap_err();
    assert!(misplaced_error.contains("conflicting or repeated board registration"));
}

#[test]
fn open_preparation_preserves_a_missing_omitted_matrix() {
    let mut accepted = pair_document();
    let halves = accepted.hardware.as_mut().unwrap().pairs[0]
        .halves
        .as_mut()
        .unwrap();
    let matrix = halves
        .correspondences
        .iter()
        .find(|association| association.kind == boardstudio_model::model::MirroredItemKind::Matrix)
        .unwrap()
        .clone();
    let mirror_members: std::collections::BTreeSet<_> = accepted
        .matrices
        .iter()
        .find(|candidate| candidate.id == matrix.mirror_id)
        .unwrap()
        .part_ids
        .iter()
        .cloned()
        .collect();
    accepted
        .matrices
        .retain(|candidate| candidate.id != matrix.mirror_id);
    accepted
        .parts
        .retain(|part| !mirror_members.contains(&part.id));
    accepted
        .boards
        .iter_mut()
        .find(|board| board.id == "right")
        .unwrap()
        .part_ids
        .retain(|id| !mirror_members.contains(id));
    accepted.hardware.as_mut().unwrap().pairs[0]
        .halves
        .as_mut()
        .unwrap()
        .exceptions
        .push(boardstudio_model::model::MirroredException {
            kind: boardstudio_model::model::MirroredItemKind::Matrix,
            mirror_id: matrix.mirror_id.clone(),
            state: boardstudio_model::model::MirroredExceptionState::Omitted,
            companion_mode: None,
            companion_tweak: None,
            removed_copper: None,
        });

    let (prepared, findings) = crate::support::prepare_open(&accepted, &[])
        .expect("the exact Omitted matrix remains physically absent during Open repair");

    assert!(
        prepared
            .matrices
            .iter()
            .all(|candidate| candidate.id != matrix.mirror_id)
    );
    assert!(findings.is_empty());
}

#[test]
fn open_preparation_repairs_linked_parts_without_matrix_correspondence() {
    let mut accepted = pair_document();
    let halves = accepted.hardware.as_mut().unwrap().pairs[0]
        .halves
        .as_mut()
        .unwrap();
    halves.correspondences.retain(|association| {
        association.kind != boardstudio_model::model::MirroredItemKind::Matrix
    });
    part_mut(&mut accepted, "right/U1").pose.at.x = 999.0;

    let (prepared, findings) = crate::support::prepare_open(&accepted, &[])
        .expect("a matrixless native pair still reconciles independently linked Parts");

    assert_eq!(part(&prepared, "right/U1").pose.at.x, 224.0);
    assert_eq!(findings.len(), 1);
    assert!(findings[0].target_ids.contains(&"right/U1".to_owned()));
}

#[test]
fn open_preserves_unassociated_local_and_omitted_matrix_children() {
    for state in [
        boardstudio_model::model::MirroredExceptionState::Local,
        boardstudio_model::model::MirroredExceptionState::Omitted,
    ] {
        let mut accepted = pair_document();
        let source_id = "matrix/left-keys/r0c1/diode";
        let mirror_id = "matrix/right-keys/r0c1/diode";
        let original_association = accepted.hardware.as_ref().unwrap().pairs[0]
            .halves
            .as_ref()
            .unwrap()
            .correspondences
            .iter()
            .find(|association| {
                association.kind == boardstudio_model::model::MirroredItemKind::Part
                    && association.source_id == source_id
                    && association.mirror_id == mirror_id
            })
            .expect("the fixture starts with an explicit child association")
            .clone();
        let retained_part = part(&accepted, mirror_id).clone();
        {
            let halves = accepted.hardware.as_mut().unwrap().pairs[0]
                .halves
                .as_mut()
                .unwrap();
            halves.correspondences.retain(|association| {
                !(association.kind == boardstudio_model::model::MirroredItemKind::Part
                    && association.source_id == source_id
                    && association.mirror_id == mirror_id)
            });
            halves
                .exceptions
                .push(boardstudio_model::model::MirroredException {
                    kind: boardstudio_model::model::MirroredItemKind::Part,
                    mirror_id: mirror_id.into(),
                    state,
                    companion_mode: original_association.companion_mode,
                    companion_tweak: original_association.companion_tweak,
                    removed_copper: None,
                });
        }
        boardstudio_core_mechanical::mechanical::halves::validate_document(&accepted)
            .expect("the exact matrix slot authenticates the legacy child exception");
        let pair_before =
            serde_json::to_vec(&accepted.hardware.as_ref().unwrap().pairs[0]).unwrap();

        let (prepared, findings) = crate::support::prepare_open(&accepted, &[])
            .unwrap_or_else(|error| panic!("{state:?} unassociated matrix child: {error}"));
        assert!(findings.is_empty(), "{state:?} produced {findings:?}");
        assert_eq!(part(&prepared, mirror_id), &retained_part);
        let prepared_pair = &prepared.hardware.as_ref().unwrap().pairs[0];
        assert_eq!(serde_json::to_vec(prepared_pair).unwrap(), pair_before);
        let halves = prepared_pair.halves.as_ref().unwrap();
        assert!(!halves.correspondences.iter().any(|association| {
            association.kind == boardstudio_model::model::MirroredItemKind::Part
                && (association.source_id == source_id || association.mirror_id == mirror_id)
        }));
        let exception = halves
            .exceptions
            .iter()
            .find(|exception| exception.mirror_id == mirror_id)
            .expect("Open retains the explicit legacy exception");
        assert_eq!(exception.state, state);
        assert_eq!(
            exception.companion_mode,
            original_association.companion_mode
        );
        assert_eq!(
            exception.companion_tweak,
            original_association.companion_tweak
        );

        let prepared_bytes = serde_json::to_vec(&prepared).unwrap();
        let (reopened, second_findings) = crate::support::prepare_open(&prepared, &[])
            .unwrap_or_else(|error| panic!("repeated {state:?} Open failed: {error}"));
        assert!(second_findings.is_empty());
        assert_eq!(reopened, prepared);
        assert_eq!(serde_json::to_vec(&reopened).unwrap(), prepared_bytes);
        assert_eq!(part(&reopened, mirror_id), &retained_part);
        assert_eq!(
            serde_json::to_vec(&reopened.hardware.as_ref().unwrap().pairs[0]).unwrap(),
            pair_before
        );
    }
}

#[test]
fn open_preparation_preserves_local_and_omitted_exceptions() {
    let mut local = pair_document();
    part_mut(&mut local, "right/U1").pose.at.x = 260.0;
    let halves = local.hardware.as_mut().unwrap().pairs[0]
        .halves
        .as_mut()
        .unwrap();
    halves
        .exceptions
        .push(boardstudio_model::model::MirroredException {
            kind: boardstudio_model::model::MirroredItemKind::Part,
            mirror_id: "right/U1".into(),
            state: boardstudio_model::model::MirroredExceptionState::Local,
            companion_mode: None,
            companion_tweak: None,
            removed_copper: None,
        });
    let (prepared_local, local_findings) = crate::support::prepare_open(&local, &[])
        .expect("Local geometry remains independently authored");
    assert_eq!(part(&prepared_local, "right/U1").pose.at.x, 260.0);
    assert!(local_findings.is_empty());

    let mut omitted = pair_document();
    let right = omitted
        .boards
        .iter_mut()
        .find(|board| board.id == "right")
        .unwrap();
    right.part_ids.retain(|id| id != "right/U1");
    omitted.parts.retain(|part| part.id != "right/U1");
    omitted.hardware.as_mut().unwrap().pairs[0]
        .halves
        .as_mut()
        .unwrap()
        .exceptions
        .push(boardstudio_model::model::MirroredException {
            kind: boardstudio_model::model::MirroredItemKind::Part,
            mirror_id: "right/U1".into(),
            state: boardstudio_model::model::MirroredExceptionState::Omitted,
            companion_mode: None,
            companion_tweak: None,
            removed_copper: None,
        });
    let (prepared_omitted, omitted_findings) =
        crate::support::prepare_open(&omitted, &[]).expect("Omitted geometry stays absent");
    assert!(
        prepared_omitted
            .parts
            .iter()
            .all(|part| part.id != "right/U1")
    );
    assert_eq!(omitted_findings.len(), 1);
    assert_eq!(omitted_findings[0].target_ids, vec!["right"]);
    let (reopened_omitted, second_omitted_findings) =
        crate::support::prepare_open(&prepared_omitted, &[]).expect("the Omitted result is stable");
    assert_eq!(reopened_omitted, prepared_omitted);
    assert!(second_omitted_findings.is_empty());
}

#[test]
fn open_preparation_preserves_unpaired_legacy_data_and_refuses_ambiguous_associations() {
    let unpaired = crate::matrix_fixtures::source_half();
    let (prepared, findings) = crate::support::prepare_open(&unpaired, &[])
        .expect("an unpaired legacy document opens without inferred links");
    assert_eq!(prepared, unpaired);
    assert!(findings.is_empty());

    let mut ambiguous = pair_document();
    let halves = ambiguous.hardware.as_mut().unwrap().pairs[0]
        .halves
        .as_mut()
        .unwrap();
    let association = halves
        .correspondences
        .iter()
        .find(|association| association.kind == boardstudio_model::model::MirroredItemKind::Part)
        .unwrap()
        .clone();
    halves.correspondences.push(association);
    let accepted_bytes = serde_json::to_vec(&ambiguous).unwrap();

    let error = crate::support::prepare_open(&ambiguous, &[]).unwrap_err();

    assert!(error.contains("repeats source part"), "{error}");
    assert_eq!(serde_json::to_vec(&ambiguous).unwrap(), accepted_bytes);
}

#[test]
fn ambiguous_mirror_board_target_is_refused_before_prepared_addition() {
    let mut accepted = pair_document();
    let mut second_pair = accepted.hardware.as_ref().unwrap().pairs[0].clone();
    second_pair.id = "ambiguous-pair".into();
    accepted.hardware.as_mut().unwrap().pairs.push(second_pair);
    let before = serde_json::to_vec(&accepted).unwrap();
    let halves = accepted.hardware.as_ref().unwrap().pairs[0]
        .halves
        .as_ref()
        .unwrap();
    let mut part = accepted.parts[0].clone();
    part.id = "right/ambiguous-target-addition".into();
    part.reference = "AMB1".into();
    let operation = EditOperation::AddMirrorOnlyPart {
        part,
        board_id: halves.mirror_board_id.clone(),
        definition: None,
        layout_id: None,
    };
    let canonical = crate::support::canonicalize(&accepted, &operation)
        .expect("the Add request canonicalizes before target selection");
    let mut draft = accepted.clone();
    let refusal =
        boardstudio_core_mirrored_half::mirrored_half::reconcile::prepare::apply_canonical(
            &mut draft,
            &accepted,
            &[],
            &canonical,
            &Default::default(),
            crate::support::test_dispatch(),
        )
        .expect_err("two pairs with this target board are ambiguous");
    assert!(refusal.contains("matched 2 pairs"), "{refusal}");
    assert_eq!(draft, accepted, "the private prepared stage is atomic");
    assert_eq!(serde_json::to_vec(&accepted).unwrap(), before);
}

#[test]
fn open_preparation_refuses_missing_part_identity_and_reference_collisions() {
    let mut id_collision = pair_document();
    id_collision.parts.retain(|part| part.id != "right/U1");
    id_collision
        .boards
        .iter_mut()
        .find(|board| board.id == "right")
        .unwrap()
        .part_ids
        .retain(|id| id != "right/U1");
    let mut colliding_board = id_collision
        .boards
        .iter()
        .find(|board| board.id == "right")
        .unwrap()
        .clone();
    colliding_board.id = "right/U1".into();
    id_collision.boards.push(colliding_board);
    let identity_error = crate::support::prepare_open(&id_collision, &[]).unwrap_err();
    assert!(
        identity_error.contains("target identity"),
        "{identity_error}"
    );

    let mut reference_collision = pair_document();
    reference_collision
        .parts
        .retain(|part| part.id != "right/U1");
    reference_collision
        .boards
        .iter_mut()
        .find(|board| board.id == "right")
        .unwrap()
        .part_ids
        .retain(|id| id != "right/U1");
    let mut collision = part(&reference_collision, "left/U1").clone();
    collision.id = "right/J2".into();
    collision.reference = "right-U1".into();
    reference_collision.parts.push(collision);
    reference_collision
        .boards
        .iter_mut()
        .find(|board| board.id == "right")
        .unwrap()
        .part_ids
        .push("right/J2".into());
    let reference_error = crate::support::prepare_open(&reference_collision, &[]).unwrap_err();
    assert!(
        reference_error.contains("mirror reference"),
        "{reference_error}"
    );
}

#[test]
fn mirror_side_move_prepares_the_source_and_reconciled_mirror_in_one_draft() {
    let accepted = pair_document();
    let accepted_bytes = serde_json::to_vec(&accepted).unwrap();
    let operation = EditOperation::MoveParts {
        positions: vec![Position {
            id: "right/U1".into(),
            at: Vec2 { x: 216.0, y: 14.0 },
        }],
    };

    let (prepared, changed) = prepare_transaction(&accepted, &[], &operation)
        .expect("the mirror-side intent prepares as one inert transaction");

    assert_eq!(
        part(&prepared, "left/U1").pose.at,
        Vec2 { x: -16.0, y: 14.0 }
    );
    assert_eq!(
        part(&prepared, "right/U1").pose.at,
        Vec2 { x: 216.0, y: 14.0 }
    );
    assert!(changed.iter().any(|id| id == "left/U1"));
    assert!(changed.iter().any(|id| id == "right/U1"));
    assert_eq!(serde_json::to_vec(&accepted).unwrap(), accepted_bytes);
}

#[test]
fn canonical_mirror_move_seeds_downstream_constraint_resolution() {
    let mut accepted = pair_document();
    let mut dependent = part(&accepted, "left/U1").clone();
    dependent.id = "left/J1".into();
    dependent.reference = "J1".into();
    dependent.pose.at = Vec2 { x: 10.0, y: 5.0 };
    accepted.parts.push(dependent);
    accepted
        .boards
        .iter_mut()
        .find(|board| board.id == "left")
        .unwrap()
        .part_ids
        .push("left/J1".into());
    accepted.constraints.push(Constraint::Offset {
        id: "left-u1-to-j1".into(),
        source_part_id: "left/U1".into(),
        target_part_id: "left/J1".into(),
        offset: Vec2 { x: 10.0, y: 5.0 },
        rotation: 0.0,
    });
    let operation = EditOperation::MoveParts {
        positions: vec![Position {
            id: "right/U1".into(),
            at: Vec2 { x: 216.0, y: 14.0 },
        }],
    };

    let (prepared, changed) = prepare_transaction(&accepted, &[], &operation)
        .expect("the canonical position also seeds the downstream constraint graph");

    let source = part(&prepared, "left/U1");
    let dependent = part(&prepared, "left/J1");
    assert!((dependent.pose.at.x - source.pose.at.x - 10.0).abs() < 1e-8);
    assert!((dependent.pose.at.y - source.pose.at.y - 5.0).abs() < 1e-8);
    assert!(changed.contains(&"left/J1".to_owned()));
}

#[test]
fn canonical_mirror_component_move_survives_downstream_assembly_resolution() {
    let accepted = pair_document();
    let accepted_bytes = serde_json::to_vec(&accepted).unwrap();
    let mirror_id = "matrix/right-keys/r0c0/diode";
    let requested = Vec2 { x: 218.0, y: -3.0 };
    let operation = EditOperation::MoveParts {
        positions: vec![Position {
            id: mirror_id.into(),
            at: requested,
        }],
    };

    let error = prepare_transaction(&accepted, &[], &operation)
        .expect_err("forward validation refuses authored geometry on a linked matrix member");

    assert!(
        error.contains("Linked matrix placements cannot retain authored or constrained geometry"),
        "{error}"
    );
    assert_eq!(serde_json::to_vec(&accepted).unwrap(), accepted_bytes);
}

#[test]
fn framed_constraint_edit_reaches_resolution_and_the_mirror_projection() {
    let accepted = pair_document();
    let accepted_bytes = serde_json::to_vec(&accepted).unwrap();
    let operation = EditOperation::SetConstraint {
        constraint: Constraint::Offset {
            id: "right-key-offset".into(),
            source_part_id: "matrix/right-keys/r0c0".into(),
            target_part_id: "right/U1".into(),
            offset: Vec2 { x: 12.0, y: 3.0 },
            rotation: 8.0,
        },
    };

    let (prepared, changed) = prepare_transaction(&accepted, &[], &operation)
        .expect("the framed constraint composes with both resolver passes");
    let source = part(&prepared, "matrix/right-keys/r0c0");
    let target = part(&prepared, "right/U1");

    assert!((target.pose.at.x - source.pose.at.x - 12.0).abs() < 1e-8);
    assert!((target.pose.at.y - source.pose.at.y - 3.0).abs() < 1e-8);
    assert!((target.pose.rotation - source.pose.rotation - 8.0).abs() < 1e-8);
    assert!(changed.contains(&"right-key-offset".to_owned()));
    assert!(changed.contains(&"right/U1".to_owned()));
    assert_eq!(serde_json::to_vec(&accepted).unwrap(), accepted_bytes);
}

#[test]
fn refused_mirror_side_move_keeps_the_accepted_document_unchanged() {
    let mut accepted = pair_document();
    part_mut(&mut accepted, "left/U1").locked = Some(true);
    let accepted_bytes = serde_json::to_vec(&accepted).unwrap();
    let operation = EditOperation::MoveParts {
        positions: vec![Position {
            id: "right/U1".into(),
            at: Vec2 { x: 216.0, y: 14.0 },
        }],
    };

    let error = prepare_transaction(&accepted, &[], &operation).unwrap_err();

    assert!(error.contains("Part left/U1 is locked"), "{error}");
    assert_eq!(serde_json::to_vec(&accepted).unwrap(), accepted_bytes);
}

#[test]
fn replacement_is_validated_as_a_whole_without_old_pair_associations() {
    let accepted = pair_document();
    let replacement = ProjectDoc::empty("replacement", "Replacement");
    let operation = EditOperation::ReplaceDocument {
        document: Box::new(replacement.clone()),
    };

    let (prepared, _) = prepare_transaction(&accepted, &[], &operation)
        .expect("a valid replacement is prepared without retaining old associations");

    assert_eq!(prepared, replacement);
    assert!(prepared.hardware.is_none());
    boardstudio_core_mechanical::mechanical::halves::validate_document(&prepared).unwrap();
}

#[test]
fn final_validation_rejects_a_duplicate_correspondence_from_replacement() {
    let accepted = pair_document();
    let mut replacement = accepted.clone();
    let halves = replacement.hardware.as_mut().unwrap().pairs[0]
        .halves
        .as_mut()
        .unwrap();
    let duplicate = halves
        .correspondences
        .iter()
        .find(|association| {
            association.kind == boardstudio_model::model::MirroredItemKind::Part
                && !association.source_id.starts_with("matrix/")
        })
        .expect("the fixture has an independent Part correspondence")
        .clone();
    halves.correspondences.push(duplicate);
    let operation = EditOperation::ReplaceDocument {
        document: Box::new(replacement.clone()),
    };

    let error =
        boardstudio_core_mirrored_half::mirrored_half::reconcile::prepare::validate_prepared_draft(
            &accepted,
            &replacement,
            &operation,
        )
        .expect_err("the final draft validator rejects duplicate durable identity");

    assert!(error.contains("repeats source part"), "{error}");
}

#[test]
fn independent_add_on_the_mirror_board_creates_one_native_correspondence() {
    let accepted = pair_document();
    let mut added = part(&accepted, "right/U1").clone();
    added.id = "right/J1".into();
    added.reference = "J1".into();
    added.pose.at = Vec2 { x: 240.0, y: 15.0 };
    let operation = EditOperation::AddPart {
        part: added,
        board_id: Some("right".into()),
        definition: None,
        layout_id: None,
    };

    let (prepared, changed) = prepare_transaction(&accepted, &[], &operation)
        .expect("the source addition and target correspondence prepare together");

    let source = part(&prepared, "left/J1");
    let mirror = part(&prepared, "right/J1");
    assert_eq!(mirror.pose.at, Vec2 { x: 240.0, y: 15.0 });
    assert_eq!(
        prepared.hardware.as_ref().unwrap().pairs[0]
            .halves
            .as_ref()
            .unwrap()
            .correspondences
            .iter()
            .filter(|association| {
                association.kind == boardstudio_model::model::MirroredItemKind::Part
                    && association.source_id == source.id
                    && association.mirror_id == mirror.id
            })
            .count(),
        1
    );
    assert!(changed.contains(&"left/J1".to_owned()));
    assert!(changed.contains(&"right/J1".to_owned()));
    assert!(changed.contains(&"pair".to_owned()));

    let removal = EditOperation::RemoveParts {
        ids: vec!["right/J1".into()],
    };
    let (removed, removal_changes) = prepare_transaction(&prepared, &[], &removal)
        .expect("removing the new mirror retires both native Part identities");
    assert!(
        removed
            .parts
            .iter()
            .all(|part| part.id != "left/J1" && part.id != "right/J1")
    );
    assert!(removal_changes.contains(&"left/J1".to_owned()));
    assert!(removal_changes.contains(&"right/J1".to_owned()));
    assert!(removal_changes.contains(&"pair".to_owned()));
    assert!(
        removed.hardware.as_ref().unwrap().pairs[0]
            .halves
            .as_ref()
            .unwrap()
            .correspondences
            .iter()
            .all(|association| association.source_id != "left/J1"
                && association.mirror_id != "right/J1")
    );
    boardstudio_core_mechanical::mechanical::halves::validate_document(&removed).unwrap();
}

#[test]
fn new_matrix_addition_and_its_members_are_composed_before_reconciliation() {
    let accepted = pair_document();
    let operation = serde_json::from_value(crate::matrix_fixtures::new_matrix(
        "right-new",
        "right",
        (30.0, 40.0),
        2,
    ))
    .expect("the matrix addition request is a valid operation");

    let (prepared, changed) = prepare_transaction(&accepted, &[], &operation)
        .expect("a new matrix on the mirror side receives its source and mirror drafts");

    assert!(
        prepared
            .matrices
            .iter()
            .any(|matrix| matrix.id == "left-new")
    );
    assert!(
        prepared
            .matrices
            .iter()
            .any(|matrix| matrix.id == "right-new")
    );
    assert!(changed.contains(&"left-new".to_owned()));
    assert!(changed.contains(&"right-new".to_owned()));
    boardstudio_core_mechanical::mechanical::halves::validate_document(&prepared).unwrap();
}

#[test]
fn matrix_member_addition_updates_both_memberships_and_associations() {
    let accepted = pair_document();
    let mut expanded = accepted
        .matrices
        .iter()
        .find(|matrix| matrix.id == "left-keys")
        .unwrap()
        .clone();
    expanded.columns = 3;
    for row in 0..expanded.rows {
        let mut added = expanded
            .cells
            .iter()
            .find(|cell| cell.row == row && cell.column == 1)
            .unwrap()
            .clone();
        added.column = 2;
        expanded.cells.push(added);
    }
    let operation = EditOperation::SetMatrix {
        matrix: expanded,
        definitions: None,
    };

    let (prepared, _) = prepare_transaction(&accepted, &[], &operation)
        .expect("new source matrix members join the mirror membership in the same draft");

    assert_eq!(
        prepared
            .matrices
            .iter()
            .find(|matrix| matrix.id == "left-keys")
            .unwrap()
            .columns,
        3
    );
    assert_eq!(
        prepared
            .matrices
            .iter()
            .find(|matrix| matrix.id == "right-keys")
            .unwrap()
            .columns,
        3
    );
    assert!(
        prepared
            .parts
            .iter()
            .any(|part| part.id == "matrix/right-keys/r0c2/diode")
    );
    boardstudio_core_mechanical::mechanical::halves::validate_document(&prepared).unwrap();
}

#[test]
fn selected_cell_removal_composes_membership_for_every_changed_matrix_pair() {
    let initial = pair_document();
    let second_matrix = crate::matrix_fixtures::new_matrix("left-extra", "left", (30.0, 40.0), 1);
    let accepted = crate::matrix_fixtures::add_matrix(&initial, &second_matrix);
    let selected_members = |matrix_id: &str| {
        let selected_key = format!("matrix/{matrix_id}/r0c0");
        let selected_child_prefix = format!("{selected_key}/");
        accepted
            .matrices
            .iter()
            .find(|matrix| matrix.id == matrix_id)
            .unwrap()
            .part_ids
            .iter()
            .filter(|id| *id == &selected_key || id.starts_with(&selected_child_prefix))
            .cloned()
            .collect::<std::collections::BTreeSet<_>>()
    };
    let source_members: std::collections::BTreeSet<_> = [
        selected_members("left-keys"),
        selected_members("left-extra"),
    ]
    .into_iter()
    .flatten()
    .collect();
    let mirror_members: std::collections::BTreeSet<_> = [
        selected_members("right-keys"),
        selected_members("right-extra"),
    ]
    .into_iter()
    .flatten()
    .collect();
    let operation = EditOperation::RemoveSelectedMatrixCells {
        cells: std::collections::BTreeMap::from([
            ("left-keys".into(), vec![(0, 0)]),
            ("left-extra".into(), vec![(0, 0)]),
        ]),
    };

    let (prepared, changed) = prepare_transaction(&accepted, &[], &operation)
        .expect("all changed matrix pairs transition within the same prepared transaction");

    assert!(source_members.iter().all(|id| {
        !prepared.parts.iter().any(|part| &part.id == id)
            && !prepared
                .boards
                .iter()
                .any(|board| board.part_ids.iter().any(|member| member == id))
    }));
    assert!(mirror_members.iter().all(|id| {
        !prepared.parts.iter().any(|part| &part.id == id)
            && !prepared
                .boards
                .iter()
                .any(|board| board.part_ids.iter().any(|member| member == id))
    }));
    let halves = prepared.hardware.as_ref().unwrap().pairs[0]
        .halves
        .as_ref()
        .unwrap();
    assert!(halves.correspondences.iter().all(|association| {
        !source_members.contains(&association.source_id)
            && !mirror_members.contains(&association.mirror_id)
    }));
    assert!(source_members.iter().all(|id| changed.contains(id)));
    assert!(mirror_members.iter().all(|id| changed.contains(id)));
    boardstudio_core_mechanical::mechanical::halves::validate_document(&prepared).unwrap();
}

#[test]
fn mirror_side_matrix_removal_retires_its_complete_correspondence_closure() {
    let accepted = pair_document();
    let source_members: std::collections::BTreeSet<_> = accepted
        .matrices
        .iter()
        .find(|matrix| matrix.id == "left-keys")
        .unwrap()
        .part_ids
        .iter()
        .cloned()
        .collect();
    let operation = EditOperation::RemoveMatrix {
        id: "right-keys".into(),
    };

    let (prepared, changed) = prepare_transaction(&accepted, &[], &operation)
        .expect("a mirror-side matrix removal retires the linked source and mirror closure");

    assert!(
        prepared
            .matrices
            .iter()
            .all(|matrix| matrix.id != "left-keys" && matrix.id != "right-keys")
    );
    assert!(
        prepared
            .parts
            .iter()
            .all(|part| !source_members.contains(&part.id))
    );
    assert!(changed.contains(&"left-keys".to_owned()));
    assert!(changed.contains(&"right-keys".to_owned()));
    assert!(source_members.iter().all(|id| changed.contains(id)));
    let halves = prepared.hardware.as_ref().unwrap().pairs[0]
        .halves
        .as_ref()
        .unwrap();
    assert!(halves.correspondences.iter().all(|association| {
        association.source_id != "left-keys"
            && association.mirror_id != "right-keys"
            && !source_members.contains(&association.source_id)
    }));
    boardstudio_core_mechanical::mechanical::halves::validate_document(&prepared).unwrap();
}
