use crate::migrate::CURRENT_VERSION;
use crate::mirrored_half::matrix_addition::tests::pair_document;
use crate::mirrored_half::prepare_transaction;
use crate::model::{
    CaseBody, Constraint, ConstraintEndpoint, ConstraintRelation, CoreReply, CoreRequest,
    EditOperation, Position, ProjectDoc, Vec2,
};
use serde_json::json;

// The shared fixture also serves the integration target, which calls with_construction_features.
#[allow(dead_code)]
#[path = "../../../../tests/support/synthetic_half.rs"]
mod synthetic_half_fixture;

fn part<'a>(document: &'a ProjectDoc, id: &str) -> &'a crate::model::Part {
    document.parts.iter().find(|part| part.id == id).unwrap()
}

fn key_relative_pose(document: &ProjectDoc, key_id: &str, companion_id: &str) -> (Vec2, f64) {
    let key = part(document, key_id).pose;
    let companion = part(document, companion_id).pose;
    let (sin, cos) = (-key.rotation).to_radians().sin_cos();
    let (dx, dy) = (companion.at.x - key.at.x, companion.at.y - key.at.y);
    (
        Vec2 {
            x: dx * cos - dy * sin,
            y: dx * sin + dy * cos,
        },
        companion.rotation - key.rotation,
    )
}

fn associated_local_circuit_document() -> ProjectDoc {
    let mut document = pair_document();
    document.format_version = crate::migrate::CURRENT_VERSION;
    document.module_definitions.push(
        serde_json::from_value(json!({
            "id": "local-circuit-module",
            "name": "Local circuit module",
            "family": "fixture",
            "variant": "fixture",
            "source": {
                "repository": "https://example.invalid/fixture",
                "revision": "fixture",
                "path": "fixture.kicad_pcb",
                "license": "MIT"
            },
            "board": {"contours": []},
            "mounts": [], "volumes": [], "openings": [], "models": [], "gates": [],
            "interfaces": [],
            "electrical": {"protocol": "pass-through", "requiredSignals": [], "logicVoltage": 3.3},
            "constituents": []
        }))
        .unwrap(),
    );
    document.embedded_circuits = serde_json::from_value(json!([
        {"id":"source-circuit","definitionId":"local-circuit-module","hostBoardId":"left",
         "partIds":[],"netIds":[],"ports":{}},
        {"id":"mirror-circuit","definitionId":"local-circuit-module","hostBoardId":"right",
         "partIds":[],"netIds":[],"ports":{}}
    ]))
    .unwrap();
    let halves = document.hardware.as_mut().unwrap().pairs[0]
        .halves
        .as_mut()
        .unwrap();
    halves.correspondences.push(
        serde_json::from_value(json!({
            "kind":"embedded-circuit","sourceId":"source-circuit","mirrorId":"mirror-circuit"
        }))
        .unwrap(),
    );
    halves.exceptions.push(
        serde_json::from_value(json!({
            "kind":"embedded-circuit","mirrorId":"mirror-circuit","state":"local"
        }))
        .unwrap(),
    );
    document
}

fn open_pair_engine(mut document: ProjectDoc) -> crate::CoreEngine {
    document.format_version = crate::migrate::CURRENT_VERSION;
    let mut engine = crate::CoreEngine::default();
    let reply = engine.handle(CoreRequest::Open {
        id: "open-pair".into(),
        document,
    });
    assert!(
        matches!(&reply, CoreReply::Scene { .. }),
        "pair fixture Open reply: {reply:?}"
    );
    engine
}

fn prepare_preview(
    engine: &crate::CoreEngine,
    operation: &EditOperation,
) -> Result<(ProjectDoc, Vec<String>), String> {
    prepare_transaction(&engine.document, &[], operation)
}

#[test]
fn prepared_open_preserves_an_associated_local_circuit_and_validates_its_definition() {
    let accepted = associated_local_circuit_document();
    let (opened, _) = super::prepare_open(&accepted, &[])
        .expect("a valid associated Local circuit remains openable");
    assert_eq!(opened.embedded_circuits, accepted.embedded_circuits);
    assert_eq!(
        opened.hardware.as_ref().unwrap().pairs[0]
            .halves
            .as_ref()
            .unwrap()
            .exceptions,
        accepted.hardware.as_ref().unwrap().pairs[0]
            .halves
            .as_ref()
            .unwrap()
            .exceptions
    );
    let (reopened, _) = super::prepare_open(&opened, &[])
        .expect("the preserved Local circuit is stable on repeat Open");
    assert_eq!(reopened, opened);

    let mut malformed = accepted;
    malformed.module_definitions.clear();
    let error = super::prepare_open(&malformed, &[])
        .expect_err("Open rejects a Local owner with an unavailable module definition");
    assert!(error.contains("module definition"), "{error}");
}

#[test]
fn replace_document_validates_local_owner_when_every_matrix_membership_changed() {
    let accepted = associated_local_circuit_document();
    let mut replacement = accepted.clone();
    replacement.module_definitions.clear();
    for (matrix_id, part_id) in [
        ("left-keys", "matrix/left-keys/r0c0"),
        ("right-keys", "matrix/right-keys/r0c0"),
    ] {
        let mut matrix = replacement
            .matrices
            .iter()
            .find(|matrix| matrix.id == matrix_id)
            .unwrap()
            .clone();
        let cell = matrix
            .cells
            .iter_mut()
            .find(|cell| cell.row == 0 && cell.column == 0)
            .unwrap();
        cell.deleted = true;
        crate::matrix::set_matrix(&mut replacement, &matrix)
            .expect("the replacement removes a valid matrix member");
        replacement.hardware.as_mut().unwrap().pairs[0]
            .halves
            .as_mut()
            .unwrap()
            .correspondences
            .retain(|association| {
                let member_prefix = format!("{part_id}/");
                association.source_id != part_id
                    && association.mirror_id != part_id
                    && !association.source_id.starts_with(&member_prefix)
                    && !association.mirror_id.starts_with(&member_prefix)
            });
    }
    assert!(super::super::matrix_membership_changed(
        &accepted,
        &replacement,
        "left-keys"
    ));
    assert!(super::super::matrix_membership_changed(
        &accepted,
        &replacement,
        "right-keys"
    ));
    let operation = EditOperation::ReplaceDocument {
        document: Box::new(replacement),
    };

    let error = prepare_transaction(&accepted, &[], &operation).expect_err(
        "ReplaceDocument validates the Local closure even when matrix projection skips",
    );

    assert!(error.contains("module definition"), "{error}");
}

#[test]
fn matrix_membership_change_ignores_member_order() {
    let before = pair_document();
    let mut after = before.clone();
    let matrix = after
        .matrices
        .iter_mut()
        .find(|matrix| matrix.id == "left-keys")
        .expect("the source matrix exists");
    matrix.part_ids.reverse();

    assert!(
        !super::super::matrix_membership_changed(&before, &after, "left-keys"),
        "reordering the same matrix members does not change membership"
    );
}

#[test]
fn matrix_membership_change_detects_additions_removals_and_duplicate_multiplicity() {
    let before = pair_document();

    let mut added = before.clone();
    added
        .matrices
        .iter_mut()
        .find(|matrix| matrix.id == "left-keys")
        .unwrap()
        .part_ids
        .push("left/new-member".into());
    assert!(super::super::matrix_membership_changed(
        &before,
        &added,
        "left-keys"
    ));

    let mut removed = before.clone();
    removed
        .matrices
        .iter_mut()
        .find(|matrix| matrix.id == "left-keys")
        .unwrap()
        .part_ids
        .pop();
    assert!(super::super::matrix_membership_changed(
        &before,
        &removed,
        "left-keys"
    ));

    let mut duplicated = before.clone();
    let matrix = duplicated
        .matrices
        .iter_mut()
        .find(|matrix| matrix.id == "left-keys")
        .unwrap();
    matrix.part_ids.push(matrix.part_ids[0].clone());
    assert!(super::super::matrix_membership_changed(
        &before,
        &duplicated,
        "left-keys"
    ));
}

#[test]
fn matrix_membership_change_distinguishes_missing_from_present_empty() {
    let mut empty = pair_document();
    empty
        .matrices
        .iter_mut()
        .find(|matrix| matrix.id == "left-keys")
        .unwrap()
        .part_ids
        .clear();
    let mut missing = empty.clone();
    missing.matrices.retain(|matrix| matrix.id != "left-keys");

    assert!(super::super::matrix_membership_changed(
        &missing,
        &empty,
        "left-keys"
    ));
    assert!(!super::super::matrix_membership_changed(
        &missing,
        &missing.clone(),
        "left-keys"
    ));
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

fn add_unpaired_mirror_board_part(document: &mut ProjectDoc) {
    let mut free = part(document, "left/U1").clone();
    free.id = "right/FREE".into();
    free.reference = "RFREE".into();
    free.pose = crate::model::Pose2 {
        at: Vec2 { x: 260.0, y: 80.0 },
        rotation: -22.0,
    };
    document.parts.push(free);
    document
        .boards
        .iter_mut()
        .find(|board| board.id == "right")
        .unwrap()
        .part_ids
        .push("right/FREE".into());
}

fn linked_endpoint(source_part_id: &str, mirror_part_id: &str) -> ConstraintEndpoint {
    ConstraintEndpoint::LinkedMirror {
        pair_id: "pair".into(),
        source_part_id: source_part_id.into(),
        mirror_part_id: mirror_part_id.into(),
    }
}

fn framed_offset(
    id: &str,
    source: ConstraintEndpoint,
    target: ConstraintEndpoint,
    offset: Vec2,
) -> Constraint {
    Constraint::Framed {
        id: id.into(),
        source,
        target,
        relation: ConstraintRelation::Offset {
            offset,
            rotation: 0.0,
        },
    }
}

fn pair_document_with_unrelated_matrix() -> ProjectDoc {
    let mut document = pair_document();
    let unrelated = crate::mirrored_half::matrix_addition::tests::op(
        crate::mirrored_half::matrix_addition::tests::new_matrix(
            "right-unrelated",
            "right",
            (280.0, 30.0),
            2,
        ),
    );
    crate::apply(&mut document, &[], &unrelated)
        .expect("the unrelated mirror-board matrix materializes");
    document
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
fn prepared_matrix_pose_edits_preserve_authored_companions_from_both_sides() {
    for matrix_id in ["left-keys", "right-keys"] {
        for operation in [
            EditOperation::SetMatrixSplay {
                matrix_id: matrix_id.into(),
                column: 1,
                change: crate::model::MatrixSplayChange::Angle {
                    angle: 12.0,
                    affect: crate::model::MatrixSplayAffect::Following,
                },
            },
            EditOperation::SetMatrixPlacement {
                matrix_id: matrix_id.into(),
                change: crate::model::MatrixPlacementChange::ColumnStagger {
                    column: 1,
                    value: 4.0,
                },
            },
        ] {
            let accepted = pair_document_with_unrelated_matrix();
            let accepted_bytes = serde_json::to_vec(&accepted).unwrap();
            let matrix_before = accepted
                .matrices
                .iter()
                .find(|matrix| matrix.id == matrix_id)
                .unwrap();
            let member_ids = matrix_before.part_ids.clone();
            let companion_fields: Vec<_> = member_ids
                .iter()
                .map(|id| (id.clone(), part(&accepted, id).clone()))
                .collect();
            let unrelated_matrix = accepted
                .matrices
                .iter()
                .find(|matrix| matrix.id == "right-unrelated")
                .unwrap()
                .clone();
            let unrelated_member_ids = unrelated_matrix.part_ids.clone();
            let key_id = format!("matrix/{matrix_id}/r0c1");
            let companion_id = format!("{key_id}/diode");
            let companion_relative_before = key_relative_pose(&accepted, &key_id, &companion_id);

            let (prepared, _) = prepare_transaction(&accepted, &[], &operation)
                .expect("a linked source or mirror matrix pose edit prepares");

            assert_eq!(serde_json::to_vec(&accepted).unwrap(), accepted_bytes);
            let matrix_after = prepared
                .matrices
                .iter()
                .find(|matrix| matrix.id == matrix_id)
                .unwrap();
            assert_eq!(matrix_after.part_ids.len(), member_ids.len());
            assert!(
                !super::super::matrix_membership_changed(&accepted, &prepared, matrix_id),
                "pose edits retain the exact member identities"
            );
            assert_eq!(matrix_after.cells, matrix_before.cells);
            assert_ne!(
                part(&prepared, &key_id).pose,
                part(&accepted, &key_id).pose,
                "the selected matrix edit changes the cell pose"
            );
            let companion_relative_after = key_relative_pose(&prepared, &key_id, &companion_id);
            assert!(
                (companion_relative_before.0.x - companion_relative_after.0.x).abs() < 1e-9
                    && (companion_relative_before.0.y - companion_relative_after.0.y).abs() < 1e-9
                    && (companion_relative_before.1 - companion_relative_after.1).abs() < 1e-9,
                "the authored companion remains at its key-relative pose"
            );
            for (id, original) in &companion_fields {
                let actual = part(&prepared, id);
                let mut expected_fields = original.clone();
                expected_fields.pose = actual.pose;
                assert_eq!(
                    actual, &expected_fields,
                    "authored member fields remain intact for {id}"
                );
            }
            let unrelated_after = prepared
                .matrices
                .iter()
                .find(|matrix| matrix.id == "right-unrelated")
                .unwrap();
            assert_eq!(unrelated_after, &unrelated_matrix);
            for id in &unrelated_member_ids {
                assert_eq!(part(&prepared, id), part(&accepted, id));
            }

            let (repeated, _) = prepare_transaction(&accepted, &[], &operation)
                .expect("repeating the same prepared pose edit is deterministic");
            assert_eq!(repeated, prepared);

            let (opened, _) = super::prepare_open(&prepared, &[])
                .expect("the prepared authored companion state opens");
            let (reopened, findings) = super::prepare_open(&opened, &[])
                .expect("repeated Open remains stable after the prepared pose edit");
            assert_eq!(reopened, opened);
            assert!(findings.is_empty());
        }
    }
}

#[test]
fn prepared_mirror_circuit_embed_reserves_staged_source_references() {
    let mut accepted = pair_document();
    let mut unrelated_board = accepted.boards[0].clone();
    unrelated_board.id = "other".into();
    unrelated_board.name = "Other".into();
    unrelated_board.outline_ids.clear();
    unrelated_board.net_ids.clear();
    unrelated_board.part_ids = vec!["other/C3".into()];
    accepted.boards.push(unrelated_board);
    let mut unrelated_part = accepted.parts[0].clone();
    unrelated_part.id = "other/C3".into();
    unrelated_part.reference = "C3".into();
    accepted.parts.push(unrelated_part);
    let accepted_before = accepted.clone();
    let definition: crate::modules::ModuleDefinition = serde_json::from_value(json!({
        "id": "staged-circuit-definition", "name": "Staged Circuit", "family": "f", "variant": "v",
        "source": {"repository": "https://example.invalid/c", "revision": "r",
                   "path": "c.kicad_pcb", "license": "MIT"},
        "board": {"contours": []}, "mounts": [], "volumes": [], "openings": [], "models": [],
        "gates": [], "interfaces": [],
        "electrical": {"protocol": "pass-through", "requiredSignals": [], "logicVoltage": 3.3},
        "constituents": [],
        "circuit": {
            "definitions": [{"id": "cd", "name": "CD", "kind": "custom", "courtyard": [], "pads": []}],
            "parts": [
                {"id": "Q1", "definitionId": "cd", "reference": "C1", "pose": {"at": {"x": 0, "y": 0}, "rotation": 0}, "side": "front"},
                {"id": "Q2", "definitionId": "cd", "reference": "C1", "pose": {"at": {"x": 2, "y": 0}, "rotation": 0}, "side": "front"}
            ],
            "nets": [], "ports": {}, "adaptations": []
        }
    })).expect("the circuit definition is valid");
    let operation = EditOperation::EmbedModuleCircuit {
        id: "right-staged".into(),
        definition,
        host_board_id: "right".into(),
        pose: crate::model::Pose2 {
            at: Vec2 { x: 180.0, y: 7.0 },
            rotation: 0.0,
        },
        side: crate::model::Side::Front,
        joins: Default::default(),
    };
    let canonical = crate::mirrored_half::inverse::canonicalize(&accepted, &operation)
        .expect("the mirror embed has a canonical source operation");
    let mut source_oracle = accepted.clone();
    crate::apply(&mut source_oracle, &[], &canonical.operation)
        .expect("the ordinary source embed succeeds");

    let (prepared, changed) = prepare_transaction(&accepted, &[], &operation)
        .expect("the prepared mirror embed reserves references staged by the source embed");

    let source_references: Vec<_> = ["embedded/left-staged/Q1", "embedded/left-staged/Q2"]
        .iter()
        .map(|id| part(&prepared, id).reference.as_str())
        .collect();
    let mirror_references: Vec<_> = ["embedded/right-staged/Q1", "embedded/right-staged/Q2"]
        .iter()
        .map(|id| part(&prepared, id).reference.as_str())
        .collect();
    let references_before = [
        "embedded/left-staged/Q1",
        "embedded/left-staged/Q2",
        "embedded/right-staged/Q1",
        "embedded/right-staged/Q2",
    ]
    .map(|id| part(&prepared, id).reference.as_str());
    let oracle_references: Vec<_> = ["embedded/left-staged/Q1", "embedded/left-staged/Q2"]
        .iter()
        .map(|id| part(&source_oracle, id).reference.as_str())
        .collect();
    assert_eq!(source_references, ["C1", "C2"]);
    assert_eq!(
        source_references, oracle_references,
        "the source embed keeps its board-local allocation"
    );
    assert_eq!(mirror_references, ["C4", "C5"]);
    assert_eq!(part(&prepared, "other/C3").reference, "C3");
    assert!(
        source_references
            .iter()
            .all(|reference| !mirror_references.contains(reference))
    );
    assert_eq!(
        accepted, accepted_before,
        "preparation leaves accepted state unchanged"
    );
    assert!(changed.iter().any(|id| id == "left"));
    assert!(changed.iter().any(|id| id == "right"));

    let (repeated, repeated_changes) = prepare_transaction(&accepted, &[], &operation)
        .expect("reconciling the same addition is deterministic");
    assert_eq!(repeated, prepared);
    assert_eq!(repeated_changes, changed);

    let source_move = EditOperation::MoveParts {
        positions: vec![Position {
            id: "embedded/left-staged/Q1".into(),
            at: Vec2 { x: 23.0, y: 11.0 },
        }],
    };
    let (source_moved, _) = prepare_transaction(&prepared, &[], &source_move)
        .expect("a subsequent source member edit reconciles");
    assert_eq!(
        part(&source_moved, "embedded/left-staged/Q1").pose.at,
        Vec2 { x: 23.0, y: 11.0 }
    );
    assert_eq!(
        part(&source_moved, "embedded/right-staged/Q1").pose.at,
        Vec2 { x: 177.0, y: 11.0 }
    );
    let (source_repeated, source_repeat_changes) =
        prepare_transaction(&source_moved, &[], &source_move)
            .expect("reconciling the same result again is idempotent");
    assert_eq!(source_repeated, source_moved);
    assert!(source_repeat_changes.is_empty());
    assert_eq!(
        [
            "embedded/left-staged/Q1",
            "embedded/left-staged/Q2",
            "embedded/right-staged/Q1",
            "embedded/right-staged/Q2"
        ]
        .map(|id| part(&source_repeated, id).reference.as_str()),
        references_before
    );
    assert_eq!(part(&source_repeated, "other/C3").reference, "C3");

    let mirror_move = EditOperation::MoveParts {
        positions: vec![Position {
            id: "embedded/right-staged/Q2".into(),
            at: Vec2 { x: 177.0, y: 12.0 },
        }],
    };
    let (mirror_moved, _) = prepare_transaction(&source_repeated, &[], &mirror_move)
        .expect("a subsequent mirror member edit reconciles");
    assert_eq!(
        part(&mirror_moved, "embedded/right-staged/Q2").pose.at,
        Vec2 { x: 177.0, y: 12.0 }
    );
    assert_eq!(
        part(&mirror_moved, "embedded/left-staged/Q2").pose.at,
        Vec2 { x: 23.0, y: 12.0 }
    );
    assert_eq!(
        [
            "embedded/left-staged/Q1",
            "embedded/left-staged/Q2",
            "embedded/right-staged/Q1",
            "embedded/right-staged/Q2"
        ]
        .map(|id| part(&mirror_moved, id).reference.as_str()),
        references_before
    );
    assert_eq!(part(&mirror_moved, "other/C3").reference, "C3");
}

fn framed_unlink_endpoint_document(linked_source: bool) -> (ProjectDoc, Constraint) {
    let mut document = pair_document();
    add_linked_independent_part(&mut document);
    let visible_id = "right/RST";
    let linked = linked_endpoint("left/RST", visible_id);
    let literal = ConstraintEndpoint::Part {
        part_id: "left/U1".into(),
    };
    let (source, target, source_pose, target_pose) = if linked_source {
        (
            linked,
            literal,
            part(&document, visible_id).pose,
            part(&document, "left/U1").pose,
        )
    } else {
        (
            literal,
            linked,
            part(&document, "left/U1").pose,
            part(&document, visible_id).pose,
        )
    };
    let record = Constraint::Framed {
        id: if linked_source {
            "constraint/unlink-source".into()
        } else {
            "constraint/unlink-target".into()
        },
        source,
        target,
        relation: ConstraintRelation::Offset {
            offset: Vec2 {
                x: target_pose.at.x - source_pose.at.x,
                y: target_pose.at.y - source_pose.at.y,
            },
            rotation: target_pose.rotation - source_pose.rotation,
        },
    };
    document.constraints.push(record.clone());
    (document, record)
}

#[test]
fn unlink_halves_is_the_framed_constraint_lifecycle_control() {
    for linked_source in [false, true] {
        let (accepted, _) = framed_unlink_endpoint_document(linked_source);
        let (prepared, _) = prepare_transaction(
            &accepted,
            &[],
            &EditOperation::UnlinkHalves {
                pair_id: "pair".into(),
            },
        )
        .expect("UnlinkHalves passes its endpoint delta through lifecycle preparation");

        assert!(prepared.hardware.as_ref().unwrap().pairs.is_empty());
        let Constraint::Framed { source, target, .. } = &prepared.constraints[0] else {
            panic!("UnlinkHalves keeps the framed relation");
        };
        let expected = ConstraintEndpoint::Part {
            part_id: "right/RST".into(),
        };
        if linked_source {
            assert_eq!(source, &expected);
        } else {
            assert_eq!(target, &expected);
        }
    }
}

#[test]
fn apply_mechanical_unlink_remains_a_closure_lifecycle_control() {
    let accepted = nonempty_closure_pair_document();
    let operation: EditOperation = serde_json::from_value(serde_json::json!({
        "kind":"apply-mechanical-edit",
        "edit":{
            "target":{"kind":"instance","instanceId":"left"},
            "change":{"unlink-mirror-pair":{"instance_id":"left"}}
        }
    }))
    .expect("the existing ApplyMechanicalEdit UnlinkMirrorPair control parses");

    let (prepared, _) = prepare_transaction(&accepted, &[], &operation)
        .expect("ApplyMechanicalEdit continues to materialize its nonempty closure");
    assert!(prepared.hardware.as_ref().unwrap().pairs.is_empty());
    assert!(
        prepared
            .parts
            .iter()
            .any(|part| part.id.starts_with("case-closure/"))
    );
}

#[test]
fn mirror_side_prepared_preview_uses_a_draft_without_publishing() {
    let engine = open_pair_engine(pair_document());
    let accepted = engine.document.clone();
    let (prepared, changed) = prepare_preview(
        &engine,
        &EditOperation::MoveParts {
            positions: vec![Position {
                id: "right/U1".into(),
                at: Vec2 { x: 216.0, y: 14.0 },
            }],
        },
    )
    .expect("the linked mirror edit prepares as an inert draft");

    assert!(changed.iter().any(|id| id == "left/U1"));
    assert!(changed.iter().any(|id| id == "right/U1"));
    assert_eq!(
        part(&prepared, "left/U1").pose.at,
        Vec2 { x: -16.0, y: 14.0 }
    );
    assert_eq!(
        part(&prepared, "right/U1").pose.at,
        Vec2 { x: 216.0, y: 14.0 }
    );
    assert_eq!(prepared.revision, accepted.revision);
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
    let engine = open_pair_engine(accepted);
    let accepted = engine.document.clone();
    let original_local = part(&engine.document, "right/U1").pose.at;
    let original_source = part(&engine.document, "left/U1").pose.at;

    let (prepared, changed) = prepare_preview(
        &engine,
        &EditOperation::MoveParts {
            positions: vec![
                Position {
                    id: "right/U1".into(),
                    at: Vec2 { x: 260.0, y: 7.0 },
                },
                Position {
                    id: "right/RST".into(),
                    at: Vec2 { x: 216.0, y: 17.0 },
                },
            ],
        },
    )
    .expect("the mixed Local and linked edit prepares as an inert draft");

    assert!(changed.contains(&"right/U1".to_owned()));
    assert!(changed.contains(&"left/RST".to_owned()));
    assert!(changed.contains(&"right/RST".to_owned()));
    assert_eq!(
        part(&prepared, "right/U1").pose.at,
        Vec2 { x: 260.0, y: 7.0 }
    );
    assert_eq!(part(&prepared, "left/U1").pose.at, original_source);
    assert_eq!(part(&engine.document, "right/U1").pose.at, original_local);
    assert_eq!(engine.document, accepted);
    assert!(engine.undo.is_empty());
    assert!(engine.redo.is_empty());
}

#[test]
fn no_op_and_refused_mirror_previews_do_not_publish_history() {
    let engine = open_pair_engine(pair_document());
    let accepted = engine.document.clone();
    let same_pose = part(&accepted, "left/U1").pose.at;

    let (preview, changed) = prepare_preview(
        &engine,
        &EditOperation::MoveParts {
            positions: vec![Position {
                id: "left/U1".into(),
                at: same_pose,
            }],
        },
    )
    .expect("the no-op preview draft is valid");
    assert_eq!(engine.document, accepted);
    assert_eq!(preview, accepted);
    assert!(changed.is_empty());
    assert!(engine.undo.is_empty());
    assert!(engine.redo.is_empty());

    let refused = prepare_preview(
        &engine,
        &EditOperation::MoveParts {
            positions: vec![Position {
                id: "right/not-present".into(),
                at: Vec2 { x: 215.0, y: 9.0 },
            }],
        },
    );
    assert!(refused.is_err());
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
    let engine = open_pair_engine(accepted);
    let accepted = engine.document.clone();

    let reply = prepare_preview(
        &engine,
        &EditOperation::MoveParts {
            positions: vec![Position {
                id: "right/U1".into(),
                at: Vec2 { x: 216.0, y: 14.0 },
            }],
        },
    );

    assert!(reply.is_err());
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

    let engine = open_pair_engine(retired);
    let accepted = engine.document.clone();
    let reply = prepare_preview(
        &engine,
        &EditOperation::MoveParts {
            positions: vec![Position {
                id: "right/RST".into(),
                at: Vec2 { x: 216.0, y: 17.0 },
            }],
        },
    );

    assert!(reply.is_err());
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
        crate::PreparedEdit {
            transaction: "commit-prepared".into(),
            document: prepared,
            changed,
            cache,
            contours,
            findings,
        },
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
fn open_and_prepared_matrix_edits_preserve_explicit_cell_data() {
    let mut accepted = pair_document();
    let mut source_matrix = accepted
        .matrices
        .iter()
        .find(|matrix| matrix.id == "left-keys")
        .unwrap()
        .clone();
    source_matrix.cells[0].definition_id = Some(source_matrix.definition_id.clone());
    source_matrix.cells[0].variant = Some("open-authored-variant".into());
    source_matrix.cells[0].assemblies_local = Some(false);
    crate::matrix::set_matrix(&mut accepted, &source_matrix)
        .expect("the ordinary source matrix materializes authored cell fields");

    let (opened, findings) = super::prepare_open(&accepted, &[])
        .expect("source-authoritative Open accepts authored cell fields");
    assert_eq!(findings.len(), 1);
    assert!(findings[0].target_ids.contains(&"right-keys".to_owned()));
    assert_eq!(
        opened
            .matrices
            .iter()
            .find(|matrix| matrix.id == "left-keys")
            .unwrap(),
        &source_matrix,
        "a complete authored source matrix remains unchanged during Open"
    );
    let open_mirror = opened
        .matrices
        .iter()
        .find(|matrix| matrix.id == "right-keys")
        .unwrap();
    assert_eq!(
        open_mirror.cells[0].definition_id,
        source_matrix.cells[0].definition_id
    );
    assert_eq!(
        open_mirror.cells[0].variant.as_deref(),
        Some("open-authored-variant")
    );
    assert_eq!(open_mirror.cells[0].assemblies_local, Some(false));
    assert_eq!(
        part(&opened, "matrix/right-keys/r0c0")
            .properties
            .as_ref()
            .and_then(|properties| properties.get("variant")),
        Some(&serde_json::json!("open-authored-variant"))
    );

    source_matrix.cells[0].variant = Some("edit-authored-variant".into());
    let (edited, _) = prepare_transaction(
        &opened,
        &[],
        &EditOperation::SetMatrix {
            matrix: source_matrix.clone(),
            definitions: None,
        },
    )
    .expect("prepared SetMatrix keeps the explicit per-cell override");
    let edit_mirror = edited
        .matrices
        .iter()
        .find(|matrix| matrix.id == "right-keys")
        .unwrap();
    assert_eq!(
        edit_mirror.cells[0].definition_id,
        source_matrix.cells[0].definition_id
    );
    assert_eq!(
        edit_mirror.cells[0].variant.as_deref(),
        Some("edit-authored-variant")
    );
    assert_eq!(edit_mirror.cells[0].assemblies_local, Some(false));
    assert_eq!(
        part(&edited, "matrix/right-keys/r0c0")
            .properties
            .as_ref()
            .and_then(|properties| properties.get("variant")),
        Some(&serde_json::json!("edit-authored-variant"))
    );
}

#[test]
fn modern_synthetic_half_private_open_and_preview_preserve_local_cells() {
    // This exercises the private preparation contract on the unchanged ticket-111 fixture.
    // It does not assert that CoreEngine's public Open or edit lifecycle has been activated.
    let source_json = synthetic_half_fixture::document();
    let original_cells: Vec<crate::model::MatrixCell> =
        serde_json::from_value(source_json["matrices"][0]["cells"].clone()).unwrap();
    let mut accepted: ProjectDoc = serde_json::from_value(source_json).unwrap();
    let create: EditOperation = serde_json::from_value(
        synthetic_half_fixture::create_mirrored_half_operation("pair-modern", "right"),
    )
    .unwrap();
    crate::apply(&mut accepted, &[], &create).expect("the unchanged modern fixture pairs");
    let original_source_cells = accepted
        .matrices
        .iter()
        .find(|matrix| matrix.id == "left-synthetic-keys")
        .unwrap()
        .cells
        .clone();
    assert_eq!(
        original_source_cells, original_cells,
        "the shared fixture's authored cell fields are used as-is"
    );

    let (opened, _) = super::prepare_open(&accepted, &[])
        .expect("private Open preparation accepts modern local cell records");
    let matrix_cells_before: Vec<_> = ["left-synthetic-keys", "right-synthetic-keys"]
        .into_iter()
        .map(|matrix_id| {
            opened
                .matrices
                .iter()
                .find(|matrix| matrix.id == matrix_id)
                .unwrap()
                .cells
                .clone()
        })
        .collect();
    assert_eq!(
        opened
            .matrices
            .iter()
            .find(|matrix| matrix.id == "left-synthetic-keys")
            .unwrap()
            .cells,
        original_source_cells,
        "private Open leaves complete source cells unchanged"
    );
    for matrix_id in ["left-synthetic-keys", "right-synthetic-keys"] {
        let matrix = opened
            .matrices
            .iter()
            .find(|matrix| matrix.id == matrix_id)
            .unwrap();
        for cell in &matrix.cells {
            assert_eq!(
                cell.definition_id.as_deref(),
                Some("generator:ceoloide/switch_mx")
            );
            assert_eq!(cell.assemblies_local, Some(true));
            assert_eq!(cell.assemblies.len(), 2);
        }
    }

    let operation = EditOperation::MoveParts {
        positions: vec![Position {
            id: "right/U1".into(),
            at: Vec2 { x: 216.0, y: 14.0 },
        }],
    };
    let engine = crate::CoreEngine {
        document: opened.clone(),
        ..crate::CoreEngine::default()
    };
    let accepted_before_preview = engine.document.clone();
    let (preview, _) = prepare_preview(&engine, &operation)
        .expect("private preview preparation keeps modern local cells");
    assert_eq!(
        engine.document, accepted_before_preview,
        "preview is isolated"
    );
    assert_eq!(
        preview
            .matrices
            .iter()
            .find(|matrix| matrix.id == "left-synthetic-keys")
            .unwrap()
            .cells,
        matrix_cells_before[0],
        "prepared preview preserves the source's modern cell records"
    );
    assert_eq!(
        preview
            .matrices
            .iter()
            .find(|matrix| matrix.id == "right-synthetic-keys")
            .unwrap()
            .cells,
        matrix_cells_before[1],
        "prepared preview preserves the target's local cell records"
    );
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
        target: linked_endpoint("left/RST", "right/RST"),
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

    let (prepared, findings) = super::prepare_open(&accepted, &[])
        .expect("the narrow independent-Part repair precedes final endpoint validation");

    assert!(prepared.parts.iter().any(|item| item.id == "right/RST"));
    assert_eq!(findings.len(), 1);
    assert!(findings[0].target_ids.contains(&"right/RST".to_owned()));
    let (reopened, second_findings) = super::prepare_open(&prepared, &[])
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
    let stale_error = super::prepare_open(&stale, &[]).unwrap_err();
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
        linked_endpoint("left/RST", "right/RST"),
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
        .push(crate::model::MirroredException {
            kind: crate::model::MirroredItemKind::Part,
            mirror_id: "right/RST".into(),
            state: crate::model::MirroredExceptionState::Omitted,
            companion_mode: None,
            companion_tweak: None,
            removed_copper: None,
        });
    let omitted_error = super::prepare_open(&omitted, &[]).unwrap_err();
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
        linked_endpoint("left/RST", "right/RST"),
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
    assert_eq!(target, &linked_endpoint("left/RST", "right/RST"));
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
        target: linked_endpoint("matrix/left-keys/r0c0/diode", target_id),
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
        source: linked_endpoint("matrix/left-keys/r0c0/diode", source_id),
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
        &linked_endpoint("matrix/left-keys/r0c0/diode", source_id)
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
    assert_eq!(target, &linked_endpoint("left/RST", "right/RST"));
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
        linked_endpoint("left/RST", "right/RST"),
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

    assert!(super::validate_prepared_draft(&accepted, &prepared, &operation).is_ok());

    let mut tampered = prepared;
    part_mut(&mut tampered, "left/RST").pose.at.x += 1.0;
    let refusal = super::validate_prepared_draft(&accepted, &tampered, &operation);
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
fn open_preserves_unassociated_local_and_omitted_matrix_children() {
    for state in [
        crate::model::MirroredExceptionState::Local,
        crate::model::MirroredExceptionState::Omitted,
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
                association.kind == crate::model::MirroredItemKind::Part
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
                !(association.kind == crate::model::MirroredItemKind::Part
                    && association.source_id == source_id
                    && association.mirror_id == mirror_id)
            });
            halves.exceptions.push(crate::model::MirroredException {
                kind: crate::model::MirroredItemKind::Part,
                mirror_id: mirror_id.into(),
                state,
                companion_mode: original_association.companion_mode,
                companion_tweak: original_association.companion_tweak,
                removed_copper: None,
            });
        }
        crate::mechanical::halves::validate_document(&accepted)
            .expect("the exact matrix slot authenticates the legacy child exception");
        let pair_before =
            serde_json::to_vec(&accepted.hardware.as_ref().unwrap().pairs[0]).unwrap();

        let (prepared, findings) = super::prepare_open(&accepted, &[])
            .unwrap_or_else(|error| panic!("{state:?} unassociated matrix child: {error}"));
        assert!(findings.is_empty(), "{state:?} produced {findings:?}");
        assert_eq!(part(&prepared, mirror_id), &retained_part);
        let prepared_pair = &prepared.hardware.as_ref().unwrap().pairs[0];
        assert_eq!(serde_json::to_vec(prepared_pair).unwrap(), pair_before);
        let halves = prepared_pair.halves.as_ref().unwrap();
        assert!(!halves.correspondences.iter().any(|association| {
            association.kind == crate::model::MirroredItemKind::Part
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
        let (reopened, second_findings) = super::prepare_open(&prepared, &[])
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
    let canonical = crate::mirrored_half::inverse::canonicalize(&accepted, &operation)
        .expect("the Add request canonicalizes before target selection");
    let mut draft = accepted.clone();
    let refusal =
        super::apply_canonical(&mut draft, &accepted, &[], &canonical, &Default::default())
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

#[test]
fn open_preserves_selected_tombstones_across_later_source_and_mirror_edits() {
    fn cell<'a>(
        document: &'a ProjectDoc,
        matrix_id: &str,
        coordinate: (u32, u32),
    ) -> &'a crate::model::MatrixCell {
        document
            .matrices
            .iter()
            .find(|matrix| matrix.id == matrix_id)
            .unwrap()
            .cells
            .iter()
            .find(|cell| (cell.row, cell.column) == coordinate)
            .unwrap()
    }

    fn assert_tombstone(document: &ProjectDoc) {
        for (matrix_id, board_id) in [("left-keys", "left"), ("right-keys", "right")] {
            let matrix = document
                .matrices
                .iter()
                .find(|matrix| matrix.id == matrix_id)
                .unwrap();
            let cell_id = format!("matrix/{matrix_id}/r0c0");
            assert!(!cell(document, matrix_id, (0, 0)).enabled);
            assert!(cell(document, matrix_id, (0, 0)).deleted);
            assert!(!matrix.part_ids.iter().any(|id| id == &cell_id));
            assert!(
                !document.parts.iter().any(|part| {
                    part.id == cell_id || part.id.starts_with(&format!("{cell_id}/"))
                })
            );
            assert!(
                !document
                    .boards
                    .iter()
                    .find(|board| board.id == board_id)
                    .unwrap()
                    .part_ids
                    .iter()
                    .any(|id| id == &cell_id || id.starts_with(&format!("{cell_id}/")))
            );
        }
    }

    let accepted = pair_document();
    let remove = EditOperation::RemoveSelectedMatrixCells {
        cells: std::collections::BTreeMap::from([("left-keys".into(), vec![(0, 0)])]),
    };
    let (removed, _) = prepare_transaction(&accepted, &[], &remove)
        .expect("selected-cell removal prepares paired tombstones");
    let source_cells = removed
        .matrices
        .iter()
        .find(|matrix| matrix.id == "left-keys")
        .unwrap()
        .cells
        .clone();
    let mirror_cells = removed
        .matrices
        .iter()
        .find(|matrix| matrix.id == "right-keys")
        .unwrap()
        .cells
        .clone();
    assert_tombstone(&removed);

    let (opened, findings) = super::prepare_open(&removed, &[])
        .expect("Open accepts a paired matrix with a selected tombstone");
    assert!(findings.is_empty());
    assert_tombstone(&opened);
    assert_eq!(
        cell(&opened, "left-keys", (0, 0)),
        cell(&removed, "left-keys", (0, 0))
    );
    assert_eq!(
        cell(&opened, "right-keys", (0, 0)),
        cell(&removed, "right-keys", (0, 0))
    );
    assert_eq!(
        opened
            .matrices
            .iter()
            .find(|matrix| matrix.id == "left-keys")
            .unwrap()
            .cells,
        source_cells
    );
    assert_eq!(
        opened
            .matrices
            .iter()
            .find(|matrix| matrix.id == "right-keys")
            .unwrap()
            .cells,
        mirror_cells
    );

    let source_move = EditOperation::MoveParts {
        positions: vec![Position {
            id: "matrix/left-keys/r0c1".into(),
            at: Vec2 { x: 43.0, y: 17.0 },
        }],
    };
    let (source_edited, _) = prepare_transaction(&opened, &[], &source_move)
        .expect("a source-side edit reconciles around the tombstone");
    assert_tombstone(&source_edited);
    let (opened_source_edit, findings) = super::prepare_open(&source_edited, &[])
        .expect("Open preserves the tombstone after a source-side edit");
    assert!(findings.is_empty());
    assert_tombstone(&opened_source_edit);
    assert_eq!(
        cell(&opened_source_edit, "left-keys", (0, 0)),
        cell(&removed, "left-keys", (0, 0))
    );
    assert_eq!(
        cell(&opened_source_edit, "right-keys", (0, 0)),
        cell(&removed, "right-keys", (0, 0))
    );

    let mirror_move = EditOperation::MoveParts {
        positions: vec![Position {
            id: "matrix/right-keys/r0c1".into(),
            at: Vec2 { x: 71.0, y: 83.0 },
        }],
    };
    let (mirror_edited, _) = prepare_transaction(&opened_source_edit, &[], &mirror_move)
        .expect("a mirror-side edit preserves the tombstone");
    assert_tombstone(&mirror_edited);
    let (opened_mirror_edit, findings) = super::prepare_open(&mirror_edited, &[])
        .expect("Open preserves the tombstone after a mirror-side edit");
    assert!(findings.is_empty());
    assert_tombstone(&opened_mirror_edit);
    assert_eq!(
        cell(&opened_mirror_edit, "left-keys", (0, 0)),
        cell(&removed, "left-keys", (0, 0))
    );
    assert_eq!(
        cell(&opened_mirror_edit, "right-keys", (0, 0)),
        cell(&removed, "right-keys", (0, 0))
    );
    let before_repeat = serde_json::to_vec(&opened_mirror_edit).unwrap();
    let (repeated, findings) =
        super::prepare_open(&opened_mirror_edit, &[]).expect("repeated Open remains valid");
    assert!(findings.is_empty());
    assert_eq!(repeated, opened_mirror_edit);
    assert_eq!(serde_json::to_vec(&repeated).unwrap(), before_repeat);
}

fn part_mut<'a>(document: &'a mut ProjectDoc, id: &str) -> &'a mut crate::model::Part {
    document
        .parts
        .iter_mut()
        .find(|part| part.id == id)
        .unwrap()
}
