use crate::mirrored_half::matrix_addition::tests::pair_document;
use crate::mirrored_half::prepare_transaction;
use crate::model::{
    CaseBody, Constraint, CoreReply, CoreRequest, EditCommand, EditOperation, EditPhase, Position,
    ProjectDoc, Vec2,
};
use serde_json::json;

fn part<'a>(document: &'a ProjectDoc, id: &str) -> &'a crate::model::Part {
    document.parts.iter().find(|part| part.id == id).unwrap()
}

fn open_pair_engine(document: ProjectDoc) -> crate::CoreEngine {
    let mut engine = crate::CoreEngine::default();
    assert!(matches!(
        engine.handle(CoreRequest::Open {
            id: "open-pair".into(),
            document,
        }),
        CoreReply::Scene { .. }
    ));
    engine
}

fn move_preview(engine: &mut crate::CoreEngine, positions: Vec<Position>) -> CoreReply {
    let base_revision = engine.document.revision;
    engine.handle(CoreRequest::Edit {
        id: "preview-move".into(),
        command: EditCommand {
            base_revision,
            transaction_id: "preview-move".into(),
            phase: EditPhase::Preview,
            target_ids: positions
                .iter()
                .map(|position| position.id.clone())
                .collect(),
            operation: EditOperation::MoveParts { positions },
        },
    })
}

fn add_linked_independent_part(document: &mut ProjectDoc) {
    let source = part(document, "left/U1").clone();
    let mut source = source;
    source.id = "left/RST".into();
    source.reference = "RST".into();
    source.pose.at = Vec2 { x: -12.0, y: 25.0 };
    let mut mirror = source.clone();
    mirror.id = "right/RST".into();
    mirror.pose.at.x = 212.0;

    document.parts.extend([source, mirror]);
    for (board_id, part_id) in [("left", "left/RST"), ("right", "right/RST")] {
        document
            .boards
            .iter_mut()
            .find(|board| board.id == board_id)
            .unwrap()
            .part_ids
            .push(part_id.into());
    }
    document.hardware.as_mut().unwrap().pairs[0]
        .halves
        .as_mut()
        .unwrap()
        .correspondences
        .push(crate::model::MirroredCorrespondence {
            companion_mode: None,
            companion_tweak: None,
            kind: crate::model::MirroredItemKind::Part,
            source_id: "left/RST".into(),
            mirror_id: "right/RST".into(),
        });
}

fn nonempty_closure_pair_document() -> ProjectDoc {
    let mut source = crate::mirrored_half::matrix_addition::tests::source_half();
    source.hardware.as_mut().unwrap().instances[0]
        .mechanical
        .as_mut()
        .unwrap()
        .closure_mounts = Some(vec![crate::model::Mount {
        id: "source-hole".into(),
        at: Vec2 { x: -8.0, y: -16.0 },
        kind: crate::model::MountKind::Hole,
        hole_diameter: 2.2,
        boss_diameter: None,
        height: None,
    }]);
    let mounting_hole = crate::generators::catalogue()
        .unwrap()
        .into_iter()
        .find(|definition| {
            definition
                .generator
                .as_ref()
                .is_some_and(|generator| generator.source == "ceoloide/mounting_hole_npth")
        })
        .expect("the bundled mounting-hole definition is available");
    let operation = crate::mirrored_half::matrix_addition::tests::op(json!({
        "kind":"create-mirrored-half",
        "pairId":"pair",
        "sourceInstanceId":"left",
        "mode":{"kind":"new-board","boardId":"right","instanceId":"right"},
        "mapping":{"at":{"x":200,"y":0},"rotation":0},
        "idRule":"board-prefix",
        "companionMode":"literal-mirror",
        "mountingHole":mounting_hole
    }));
    crate::apply(&mut source, &[], &operation).expect("the half with a closure is created");
    source
}

#[test]
fn mirror_side_preview_uses_a_prepared_draft_without_publishing() {
    let mut engine = open_pair_engine(pair_document());
    let accepted = engine.document.clone();
    let reply = move_preview(
        &mut engine,
        vec![Position {
            id: "right/U1".into(),
            at: Vec2 { x: 216.0, y: 14.0 },
        }],
    );
    let CoreReply::Preview { scene, .. } = reply else {
        panic!("expected a paired preview, got {reply:?}");
    };

    assert!(scene.changed_ids.iter().any(|id| id == "left/U1"));
    assert!(scene.changed_ids.iter().any(|id| id == "right/U1"));
    assert_eq!(engine.document, accepted);
    assert!(engine.undo.is_empty());
    assert!(engine.redo.is_empty());
}

#[test]
fn mixed_preview_keeps_local_geometry_and_projects_linked_mirror_intent() {
    let mut accepted = pair_document();
    add_linked_independent_part(&mut accepted);
    accepted.hardware.as_mut().unwrap().pairs[0]
        .halves
        .as_mut()
        .unwrap()
        .exceptions
        .push(crate::model::MirroredException {
            kind: crate::model::MirroredItemKind::Part,
            mirror_id: "right/U1".into(),
            state: crate::model::MirroredExceptionState::Local,
            companion_mode: None,
            companion_tweak: None,
            removed_copper: None,
        });
    let mut engine = open_pair_engine(accepted);
    let original_local = part(&engine.document, "right/U1").pose.at;

    let reply = move_preview(
        &mut engine,
        vec![
            Position {
                id: "right/U1".into(),
                at: Vec2 { x: 260.0, y: 7.0 },
            },
            Position {
                id: "right/RST".into(),
                at: Vec2 { x: 216.0, y: 17.0 },
            },
        ],
    );
    let CoreReply::Preview { scene, .. } = reply else {
        panic!("expected a mixed paired preview, got {reply:?}");
    };

    assert!(scene.changed_ids.contains(&"right/U1".to_owned()));
    assert!(scene.changed_ids.contains(&"left/RST".to_owned()));
    assert!(scene.changed_ids.contains(&"right/RST".to_owned()));
    assert_eq!(part(&engine.document, "right/U1").pose.at, original_local);
    assert!(engine.undo.is_empty());
    assert!(engine.redo.is_empty());
}

#[test]
fn no_op_and_refused_mirror_previews_do_not_publish_history() {
    let mut engine = open_pair_engine(pair_document());
    let accepted = engine.document.clone();
    let same_pose = part(&accepted, "left/U1").pose.at;

    let reply = move_preview(
        &mut engine,
        vec![Position {
            id: "left/U1".into(),
            at: same_pose,
        }],
    );
    let CoreReply::Preview { scene, .. } = reply else {
        panic!("expected a no-op preview, got {reply:?}");
    };
    assert_eq!(engine.document, accepted);
    assert!(scene.changed_ids.is_empty());
    assert!(engine.undo.is_empty());
    assert!(engine.redo.is_empty());

    let refused = move_preview(
        &mut engine,
        vec![Position {
            id: "right/not-present".into(),
            at: Vec2 { x: 215.0, y: 9.0 },
        }],
    );
    assert!(matches!(refused, CoreReply::Error { .. }));
    assert_eq!(engine.document, accepted);
    assert!(engine.undo.is_empty());
    assert!(engine.redo.is_empty());
}

#[test]
fn omitted_mirror_preview_refusal_preserves_document_and_history() {
    let mut accepted = pair_document();
    accepted.parts.retain(|part| part.id != "right/U1");
    accepted
        .boards
        .iter_mut()
        .find(|board| board.id == "right")
        .unwrap()
        .part_ids
        .retain(|id| id != "right/U1");
    accepted.hardware.as_mut().unwrap().pairs[0]
        .halves
        .as_mut()
        .unwrap()
        .exceptions
        .push(crate::model::MirroredException {
            kind: crate::model::MirroredItemKind::Part,
            mirror_id: "right/U1".into(),
            state: crate::model::MirroredExceptionState::Omitted,
            companion_mode: None,
            companion_tweak: None,
            removed_copper: None,
        });
    let mut engine = open_pair_engine(accepted);
    let accepted = engine.document.clone();

    let reply = move_preview(
        &mut engine,
        vec![Position {
            id: "right/U1".into(),
            at: Vec2 { x: 216.0, y: 14.0 },
        }],
    );

    assert!(matches!(reply, CoreReply::Error { .. }));
    assert_eq!(engine.document, accepted);
    assert!(engine.undo.is_empty());
    assert!(engine.redo.is_empty());
}

#[test]
fn retired_mirror_target_preview_refusal_preserves_document_and_history() {
    let mut accepted = pair_document();
    add_linked_independent_part(&mut accepted);
    let (retired, _) = prepare_transaction(
        &accepted,
        &[],
        &EditOperation::RemoveParts {
            ids: vec!["left/RST".into()],
        },
    )
    .expect("the linked part can be retired as one prepared edit");
    assert!(retired.parts.iter().all(|part| part.id != "right/RST"));

    let mut engine = open_pair_engine(retired);
    let accepted = engine.document.clone();
    let reply = move_preview(
        &mut engine,
        vec![Position {
            id: "right/RST".into(),
            at: Vec2 { x: 216.0, y: 17.0 },
        }],
    );

    assert!(matches!(reply, CoreReply::Error { .. }));
    assert_eq!(engine.document, accepted);
    assert!(engine.undo.is_empty());
    assert!(engine.redo.is_empty());
}

#[test]
fn prepared_publication_and_history_restore_both_halves_and_local_exception_once() {
    let mut accepted = pair_document();
    add_linked_independent_part(&mut accepted);
    part_mut(&mut accepted, "right/RST").pose.at = Vec2 { x: 260.0, y: 25.0 };
    accepted.hardware.as_mut().unwrap().pairs[0]
        .halves
        .as_mut()
        .unwrap()
        .exceptions
        .push(crate::model::MirroredException {
            kind: crate::model::MirroredItemKind::Part,
            mirror_id: "right/RST".into(),
            state: crate::model::MirroredExceptionState::Local,
            companion_mode: None,
            companion_tweak: None,
            removed_copper: None,
        });
    let mut engine = open_pair_engine(accepted);
    let accepted = engine.document.clone();
    let operation = EditOperation::MoveParts {
        positions: vec![Position {
            id: "right/U1".into(),
            at: Vec2 { x: 216.0, y: 14.0 },
        }],
    };
    let (prepared, changed) = prepare_transaction(&accepted, &[], &operation)
        .expect("the paired edit prepares without publishing");
    let (cache, contours, findings) =
        crate::geometry::outlines(&prepared, Some(&engine.outline_cache), &changed);
    let reply = engine.publish_prepared_edit(
        "commit-prepared".into(),
        "commit-prepared".into(),
        prepared,
        changed,
        cache,
        contours,
        findings,
    );
    assert!(matches!(reply, CoreReply::Scene { .. }));

    assert_eq!(engine.undo.len(), 1);
    assert!(engine.redo.is_empty());
    assert_eq!(
        part(&engine.document, "left/U1").pose.at,
        Vec2 { x: -16.0, y: 14.0 }
    );
    assert_eq!(
        part(&engine.document, "right/U1").pose.at,
        Vec2 { x: 216.0, y: 14.0 }
    );
    assert_eq!(
        part(&engine.document, "right/RST").pose.at,
        Vec2 { x: 260.0, y: 25.0 }
    );
    assert!(crate::mechanical::halves::validate_document(&engine.document).is_ok());

    let undo = engine.handle(CoreRequest::Undo {
        id: "undo-prepared".into(),
    });
    assert!(matches!(undo, CoreReply::Scene { .. }));
    assert_eq!(engine.undo.len(), 0);
    assert_eq!(engine.redo.len(), 1);
    let mut expected_undo = accepted.clone();
    expected_undo.revision = engine.document.revision;
    assert_eq!(engine.document, expected_undo);

    let redo = engine.handle(CoreRequest::Redo {
        id: "redo-prepared".into(),
    });
    assert!(matches!(redo, CoreReply::Scene { .. }));
    assert_eq!(engine.undo.len(), 1);
    assert!(engine.redo.is_empty());
    assert_eq!(
        part(&engine.document, "left/U1").pose.at,
        Vec2 { x: -16.0, y: 14.0 }
    );
    assert_eq!(
        part(&engine.document, "right/U1").pose.at,
        Vec2 { x: 216.0, y: 14.0 }
    );
    assert_eq!(
        part(&engine.document, "right/RST").pose.at,
        Vec2 { x: 260.0, y: 25.0 }
    );
    assert_eq!(engine.document.revision, accepted.revision + 3);
    assert!(crate::mechanical::halves::validate_document(&engine.document).is_ok());
}

#[test]
fn prepared_reopen_preserves_a_nonempty_unlinked_result() {
    let accepted = nonempty_closure_pair_document();
    let operation: EditOperation = serde_json::from_value(json!({
        "kind":"apply-mechanical-edit",
        "edit":{
            "target":{"kind":"instance","instanceId":"left"},
            "change":{"unlink-mirror-pair":{"instance_id":"left"}}
        }
    }))
    .expect("the UnlinkMirrorPair operation parses");
    let (independent, _) = prepare_transaction(&accepted, &[], &operation)
        .expect("the nonempty pair closure materializes in the prepared edit");
    assert!(independent.hardware.as_ref().unwrap().pairs.is_empty());
    assert!(
        independent
            .parts
            .iter()
            .any(|part| part.id.starts_with("case-closure/"))
    );

    let (reopened, findings) = super::prepare_open(&independent, &[])
        .expect("the independent Unlink result prepares as a valid Open input");

    assert_eq!(reopened, independent);
    assert!(findings.is_empty());
}

#[test]
fn open_preparation_repairs_linked_geometry_and_reports_a_located_warning() {
    let mut accepted = pair_document();
    part_mut(&mut accepted, "right/U1").pose.at.x = 999.0;
    let accepted_bytes = serde_json::to_vec(&accepted).unwrap();

    let (prepared, findings) =
        super::prepare_open(&accepted, &[]).expect("the source-authoritative Open draft prepares");

    assert_eq!(part(&prepared, "right/U1").pose.at.x, 224.0);
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].severity, crate::model::Severity::Warning);
    assert!(findings[0].target_ids.contains(&"right/U1".to_owned()));
    assert_eq!(serde_json::to_vec(&accepted).unwrap(), accepted_bytes);

    let (reopened, second_findings) = super::prepare_open(&prepared, &[])
        .expect("the repaired document is a valid second Open input");
    assert_eq!(reopened, prepared);
    assert!(second_findings.is_empty());
}

#[test]
fn open_preparation_reports_linked_drift_normalized_before_reconciliation() {
    let mut accepted = pair_document();
    let target_id = accepted.hardware.as_ref().unwrap().pairs[0]
        .halves
        .as_ref()
        .unwrap()
        .correspondences
        .iter()
        .find(|association| {
            association.kind == crate::model::MirroredItemKind::Part
                && association.source_id.starts_with("matrix/")
                && association.source_id.ends_with("/diode")
        })
        .unwrap()
        .mirror_id
        .clone();
    part_mut(&mut accepted, &target_id).pose.at.x += 123.0;
    let accepted_bytes = serde_json::to_vec(&accepted).unwrap();

    let (prepared, findings) = super::prepare_open(&accepted, &[])
        .expect("the existing normalization and projection stages prepare the document");

    assert_ne!(
        part(&prepared, &target_id).pose.at.x,
        part(&accepted, &target_id).pose.at.x
    );
    assert_eq!(findings.len(), 1);
    assert!(findings[0].target_ids.contains(&target_id));
    assert_eq!(serde_json::to_vec(&accepted).unwrap(), accepted_bytes);
}

#[test]
fn open_preparation_preserves_linked_parts_across_multiple_matrix_pairs() {
    let initial = pair_document();
    let second_matrix = crate::mirrored_half::matrix_addition::tests::new_matrix(
        "left-extra",
        "left",
        (30.0, 40.0),
        1,
    );
    let accepted =
        crate::mirrored_half::matrix_addition::tests::add_matrix(&initial, &second_matrix);
    let expected: Vec<_> = accepted
        .parts
        .iter()
        .filter(|part| part.id.starts_with("matrix/right-"))
        .cloned()
        .collect();

    let (prepared, findings) = super::prepare_open(&accepted, &[])
        .expect("each linked matrix keeps its own projection during Open repair");

    for expected_part in expected {
        assert_eq!(part(&prepared, &expected_part.id), &expected_part);
    }
    assert!(findings.is_empty());
}

#[test]
fn open_preparation_reports_repaired_linked_keycap_settings() {
    let mut accepted = pair_document();
    for operation in [
        EditOperation::SetKeycapBoard {
            board_id: "left".into(),
            change: crate::model::KeycapBoardChange::Color {
                value: "#112233".into(),
            },
        },
        EditOperation::SetKeycapBoard {
            board_id: "right".into(),
            change: crate::model::KeycapBoardChange::Color {
                value: "#abcdef".into(),
            },
        },
        EditOperation::SetMatrixKeycaps {
            matrix_id: "left-keys".into(),
            change: crate::model::KeycapMatrixChange::Profile {
                value: Some(crate::model::KeycapProfile::Choc),
            },
        },
        EditOperation::SetKeycapKey {
            key_id: "matrix/left-keys/r0c0".into(),
            change: crate::model::KeycapKeyChange::Legend {
                value: Some("Q".into()),
            },
        },
        EditOperation::SetKeycapKey {
            key_id: "matrix/right-keys/r0c0".into(),
            change: crate::model::KeycapKeyChange::Legend {
                value: Some("wrong".into()),
            },
        },
    ] {
        crate::apply(&mut accepted, &[], &operation).expect("the keycap edit applies");
    }

    let (prepared, findings) = super::prepare_open(&accepted, &[])
        .expect("linked keycap settings are restored from their source");

    let keycaps = prepared.keycaps.as_ref().unwrap();
    assert_eq!(keycaps.boards["right"].color, "#112233");
    assert_eq!(
        keycaps.matrices["right-keys"].profile,
        Some(crate::model::KeycapProfile::Choc)
    );
    assert_eq!(
        keycaps.keys["matrix/right-keys/r0c0"].legend.as_deref(),
        Some("Q")
    );
    assert_eq!(findings.len(), 1);
    for target in ["right", "right-keys", "matrix/right-keys/r0c0"] {
        assert!(findings[0].target_ids.iter().any(|id| id == target));
    }
}

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

    let (prepared, findings) = super::prepare_open(&accepted, &[])
        .expect("a missing linked projection is reconstructed from its source");

    assert_eq!(part(&prepared, "right/U1").pose.at.x, 224.0);
    assert_eq!(findings.len(), 1);
    assert!(findings[0].target_ids.contains(&"right/U1".to_owned()));
    assert_eq!(serde_json::to_vec(&accepted).unwrap(), accepted_bytes);

    let (reopened, second_findings) = super::prepare_open(&prepared, &[])
        .expect("the reconstructed document is a valid second Open input");
    assert_eq!(reopened, prepared);
    assert!(second_findings.is_empty());
}

#[test]
fn open_preparation_restores_a_missing_part_with_one_expected_board_registration() {
    let mut accepted = pair_document();
    accepted.parts.retain(|part| part.id != "right/U1");
    let accepted_bytes = serde_json::to_vec(&accepted).unwrap();

    let (prepared, findings) = super::prepare_open(&accepted, &[])
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
    let repeated_error = super::prepare_open(&repeated, &[]).unwrap_err();
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
    let misplaced_error = super::prepare_open(&misplaced, &[]).unwrap_err();
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
        .find(|association| association.kind == crate::model::MirroredItemKind::Matrix)
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
        .push(crate::model::MirroredException {
            kind: crate::model::MirroredItemKind::Matrix,
            mirror_id: matrix.mirror_id.clone(),
            state: crate::model::MirroredExceptionState::Omitted,
            companion_mode: None,
            companion_tweak: None,
            removed_copper: None,
        });

    let (prepared, findings) = super::prepare_open(&accepted, &[])
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
    halves
        .correspondences
        .retain(|association| association.kind != crate::model::MirroredItemKind::Matrix);
    part_mut(&mut accepted, "right/U1").pose.at.x = 999.0;

    let (prepared, findings) = super::prepare_open(&accepted, &[])
        .expect("a matrixless native pair still reconciles independently linked Parts");

    assert_eq!(part(&prepared, "right/U1").pose.at.x, 224.0);
    assert_eq!(findings.len(), 1);
    assert!(findings[0].target_ids.contains(&"right/U1".to_owned()));
}

#[test]
fn open_preparation_preserves_local_and_omitted_exceptions() {
    let mut local = pair_document();
    part_mut(&mut local, "right/U1").pose.at.x = 260.0;
    let halves = local.hardware.as_mut().unwrap().pairs[0]
        .halves
        .as_mut()
        .unwrap();
    halves.exceptions.push(crate::model::MirroredException {
        kind: crate::model::MirroredItemKind::Part,
        mirror_id: "right/U1".into(),
        state: crate::model::MirroredExceptionState::Local,
        companion_mode: None,
        companion_tweak: None,
        removed_copper: None,
    });
    let (prepared_local, local_findings) =
        super::prepare_open(&local, &[]).expect("Local geometry remains independently authored");
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
        .push(crate::model::MirroredException {
            kind: crate::model::MirroredItemKind::Part,
            mirror_id: "right/U1".into(),
            state: crate::model::MirroredExceptionState::Omitted,
            companion_mode: None,
            companion_tweak: None,
            removed_copper: None,
        });
    let (prepared_omitted, omitted_findings) =
        super::prepare_open(&omitted, &[]).expect("Omitted geometry stays absent");
    assert!(
        prepared_omitted
            .parts
            .iter()
            .all(|part| part.id != "right/U1")
    );
    assert_eq!(omitted_findings.len(), 1);
    assert_eq!(omitted_findings[0].target_ids, vec!["right"]);
    let (reopened_omitted, second_omitted_findings) =
        super::prepare_open(&prepared_omitted, &[]).expect("the Omitted result is stable");
    assert_eq!(reopened_omitted, prepared_omitted);
    assert!(second_omitted_findings.is_empty());
}

#[test]
fn open_preparation_preserves_unpaired_legacy_data_and_refuses_ambiguous_associations() {
    let unpaired = crate::mirrored_half::matrix_addition::tests::source_half();
    let (prepared, findings) = super::prepare_open(&unpaired, &[])
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
        .find(|association| association.kind == crate::model::MirroredItemKind::Part)
        .unwrap()
        .clone();
    halves.correspondences.push(association);
    let accepted_bytes = serde_json::to_vec(&ambiguous).unwrap();

    let error = super::prepare_open(&ambiguous, &[]).unwrap_err();

    assert!(error.contains("repeats source part"), "{error}");
    assert_eq!(serde_json::to_vec(&ambiguous).unwrap(), accepted_bytes);
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
    let identity_error = super::prepare_open(&id_collision, &[]).unwrap_err();
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
    let reference_error = super::prepare_open(&reference_collision, &[]).unwrap_err();
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
    crate::mechanical::halves::validate_document(&prepared).unwrap();
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
            association.kind == crate::model::MirroredItemKind::Part
                && !association.source_id.starts_with("matrix/")
        })
        .expect("the fixture has an independent Part correspondence")
        .clone();
    halves.correspondences.push(duplicate);
    let operation = EditOperation::ReplaceDocument {
        document: Box::new(replacement.clone()),
    };

    let error = super::validate_prepared_draft(&accepted, &replacement, &operation)
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
                association.kind == crate::model::MirroredItemKind::Part
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
    crate::mechanical::halves::validate_document(&removed).unwrap();
}

#[test]
fn new_matrix_addition_and_its_members_are_composed_before_reconciliation() {
    let accepted = pair_document();
    let operation =
        serde_json::from_value(crate::mirrored_half::matrix_addition::tests::new_matrix(
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
    crate::mechanical::halves::validate_document(&prepared).unwrap();
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
    crate::mechanical::halves::validate_document(&prepared).unwrap();
}

#[test]
fn selected_cell_removal_composes_membership_for_every_changed_matrix_pair() {
    let initial = pair_document();
    let second_matrix = crate::mirrored_half::matrix_addition::tests::new_matrix(
        "left-extra",
        "left",
        (30.0, 40.0),
        1,
    );
    let accepted =
        crate::mirrored_half::matrix_addition::tests::add_matrix(&initial, &second_matrix);
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
    crate::mechanical::halves::validate_document(&prepared).unwrap();
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
    crate::mechanical::halves::validate_document(&prepared).unwrap();
}

#[test]
fn set_case_keeps_the_other_boards_custom_body_independent() {
    let mut accepted = pair_document();
    let accepted_correspondences = accepted.hardware.as_ref().unwrap().pairs[0]
        .halves
        .as_ref()
        .unwrap()
        .correspondences
        .clone();
    accepted.case_bodies = [("left-case", "left", 2.0), ("right-case", "right", 3.0)]
        .into_iter()
        .map(|(id, board_id, thickness)| {
            serde_json::from_value::<CaseBody>(json!({
                "id": id,
                "name": id,
                "boardId": board_id,
                "kind": "tray",
                "thickness": thickness,
                "clearance": 0.2
            }))
            .unwrap()
        })
        .collect();
    let right_before = accepted
        .case_bodies
        .iter()
        .find(|body| body.id == "right-case")
        .unwrap()
        .clone();
    let mut left = accepted
        .case_bodies
        .iter()
        .find(|body| body.id == "left-case")
        .unwrap()
        .clone();
    left.thickness = 4.0;

    let (prepared, _) = prepare_transaction(&accepted, &[], &EditOperation::SetCase { body: left })
        .expect("the per-board case edit is preserved through pair reconciliation");

    assert_eq!(
        prepared
            .case_bodies
            .iter()
            .find(|body| body.id == "left-case")
            .unwrap()
            .thickness,
        4.0
    );
    assert_eq!(
        prepared
            .case_bodies
            .iter()
            .find(|body| body.id == "right-case")
            .unwrap(),
        &right_before
    );
    assert_eq!(
        prepared.hardware.as_ref().unwrap().pairs[0]
            .halves
            .as_ref()
            .unwrap()
            .correspondences,
        accepted_correspondences
    );
    crate::mechanical::halves::validate_document(&prepared).unwrap();
}

#[test]
fn no_op_mirror_move_reports_no_changed_id() {
    let accepted = pair_document();
    let current = part(&accepted, "right/U1").pose.at;
    let operation = EditOperation::MoveParts {
        positions: vec![Position {
            id: "right/U1".into(),
            at: current,
        }],
    };

    let (prepared, changed) = prepare_transaction(&accepted, &[], &operation)
        .expect("an unchanged move is a valid inert preparation");

    assert_eq!(prepared, accepted);
    assert!(changed.is_empty());
}

fn part_mut<'a>(document: &'a mut ProjectDoc, id: &str) -> &'a mut crate::model::Part {
    document
        .parts
        .iter_mut()
        .find(|part| part.id == id)
        .unwrap()
}
