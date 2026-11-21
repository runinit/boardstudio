use super::*;

pub(crate) fn key_relative_pose(
    document: &ProjectDoc,
    key_id: &str,
    companion_id: &str,
) -> (Vec2, f64) {
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

pub(crate) fn associated_local_circuit_document() -> ProjectDoc {
    let mut document = pair_document();
    document.format_version = boardstudio_core_kernel::migrate::CURRENT_VERSION;
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

pub(crate) fn open_pair_engine(mut document: ProjectDoc) -> boardstudio_core_engine::CoreEngine {
    document.format_version = boardstudio_core_kernel::migrate::CURRENT_VERSION;
    let mut engine = boardstudio_core_engine::CoreEngine::new();
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

pub(crate) fn prepare_preview(
    engine: &boardstudio_core_engine::CoreEngine,
    operation: &EditOperation,
) -> Result<(ProjectDoc, Vec<String>), String> {
    prepare_transaction(engine.document_for_test(), &[], operation)
}

#[test]
fn typed_project_rename_matches_a_name_only_replacement_on_a_native_pair() {
    use boardstudio_model::model::{EditCommand, EditPhase};

    let mut typed = open_pair_engine(pair_document());
    let mut legacy = open_pair_engine(pair_document());
    let mut replacement = legacy.document_for_test().clone();
    replacement.name = "Renamed pair".into();
    let request = |operation| CoreRequest::Edit {
        id: "rename".into(),
        command: EditCommand {
            base_revision: replacement.revision,
            transaction_id: "rename".into(),
            phase: EditPhase::Commit,
            target_ids: vec![replacement.id.clone()],
            operation,
        },
    };
    let expected = legacy.handle(request(EditOperation::ReplaceDocument {
        document: Box::new(replacement.clone()),
    }));
    let actual = typed.handle(request(EditOperation::RenameProject {
        name: replacement.name.clone(),
    }));
    assert!(matches!(
        &actual,
        CoreReply::Scene { scene, document, .. }
            if scene.changed_ids.is_empty() && document.name == replacement.name
    ));
    assert_eq!(
        boardstudio_model::wire::encode_core_reply(&actual),
        boardstudio_model::wire::encode_core_reply(&expected),
    );
}

#[test]
fn prepared_open_preserves_an_associated_local_circuit_and_validates_its_definition() {
    let accepted = associated_local_circuit_document();
    let (opened, _) = crate::support::prepare_open(&accepted, &[])
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
    let (reopened, _) = crate::support::prepare_open(&opened, &[])
        .expect("the preserved Local circuit is stable on repeat Open");
    assert_eq!(reopened, opened);

    let mut malformed = accepted;
    malformed.module_definitions.clear();
    let error = crate::support::prepare_open(&malformed, &[])
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
        boardstudio_core_kernel::matrix::set_matrix(&mut replacement, &matrix)
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
    assert!(
        boardstudio_core_mirrored_half::mirrored_half::reconcile::matrix_membership_changed(
            &accepted,
            &replacement,
            "left-keys"
        )
    );
    assert!(
        boardstudio_core_mirrored_half::mirrored_half::reconcile::matrix_membership_changed(
            &accepted,
            &replacement,
            "right-keys"
        )
    );
    let operation = EditOperation::ReplaceDocument {
        document: Box::new(replacement),
    };

    let error = prepare_transaction(&accepted, &[], &operation).expect_err(
        "ReplaceDocument validates the Local closure even when matrix projection skips",
    );

    assert!(error.contains("module definition"), "{error}");
}

#[test]
fn matrix_membership_change_table() {
    let changed =
        boardstudio_core_mirrored_half::mirrored_half::reconcile::matrix_membership_changed;
    // Table-driven: each row builds a (before, after) pair and the expected
    // membership-change verdict for matrix "left-keys".
    for (name, expected) in [
        ("reordering the same members", false),
        ("adding a member", true),
        ("removing a member", true),
        ("duplicating a member", true),
        ("missing matrix versus present-but-empty matrix", true),
        ("missing matrix versus itself", false),
    ] {
        let (before, after) = match name {
            "reordering the same members" => {
                let before = pair_document();
                let mut after = before.clone();
                after
                    .matrices
                    .iter_mut()
                    .find(|matrix| matrix.id == "left-keys")
                    .expect("the source matrix exists")
                    .part_ids
                    .reverse();
                (before, after)
            }
            "adding a member" => {
                let before = pair_document();
                let mut after = before.clone();
                after
                    .matrices
                    .iter_mut()
                    .find(|matrix| matrix.id == "left-keys")
                    .unwrap()
                    .part_ids
                    .push("left/new-member".into());
                (before, after)
            }
            "removing a member" => {
                let before = pair_document();
                let mut after = before.clone();
                after
                    .matrices
                    .iter_mut()
                    .find(|matrix| matrix.id == "left-keys")
                    .unwrap()
                    .part_ids
                    .pop();
                (before, after)
            }
            "duplicating a member" => {
                let before = pair_document();
                let mut after = before.clone();
                let matrix = after
                    .matrices
                    .iter_mut()
                    .find(|matrix| matrix.id == "left-keys")
                    .unwrap();
                matrix.part_ids.push(matrix.part_ids[0].clone());
                (before, after)
            }
            "missing matrix versus present-but-empty matrix" => {
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
                (missing, empty)
            }
            "missing matrix versus itself" => {
                let mut missing = pair_document();
                missing.matrices.retain(|matrix| matrix.id != "left-keys");
                let after = missing.clone();
                (missing, after)
            }
            _ => unreachable!("unknown membership-change case {name}"),
        };
        assert_eq!(
            changed(&before, &after, "left-keys"),
            expected,
            "membership-change case: {name}"
        );
    }
}

pub(crate) fn add_linked_independent_part(document: &mut ProjectDoc) {
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
        .push(boardstudio_model::model::MirroredCorrespondence {
            companion_mode: None,
            companion_tweak: None,
            kind: boardstudio_model::model::MirroredItemKind::Part,
            source_id: "left/RST".into(),
            mirror_id: "right/RST".into(),
        });
}

pub(crate) fn add_unpaired_mirror_board_part(document: &mut ProjectDoc) {
    let mut free = part(document, "left/U1").clone();
    free.id = "right/FREE".into();
    free.reference = "RFREE".into();
    free.pose = boardstudio_model::model::Pose2 {
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

pub(crate) fn framed_offset(
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

pub(crate) fn pair_document_with_unrelated_matrix() -> ProjectDoc {
    let mut document = pair_document();
    let unrelated = crate::matrix_fixtures::op(crate::matrix_fixtures::new_matrix(
        "right-unrelated",
        "right",
        (280.0, 30.0),
        2,
    ));
    crate::support::apply_for_test(&mut document, &[], &unrelated)
        .expect("the unrelated mirror-board matrix materializes");
    document
}

pub(crate) fn nonempty_closure_pair_document() -> ProjectDoc {
    let mut source = crate::matrix_fixtures::source_half();
    source.hardware.as_mut().unwrap().instances[0]
        .mechanical
        .as_mut()
        .unwrap()
        .closure_mounts = Some(vec![boardstudio_model::model::Mount {
        id: "source-hole".into(),
        at: Vec2 { x: -8.0, y: -16.0 },
        kind: boardstudio_model::model::MountKind::Hole,
        hole_diameter: 2.2,
        boss_diameter: None,
        height: None,
    }]);
    let mounting_hole = boardstudio_core_kernel::generators::catalogue()
        .unwrap()
        .into_iter()
        .find(|definition| {
            definition
                .generator
                .as_ref()
                .is_some_and(|generator| generator.source == "ceoloide/mounting_hole_npth")
        })
        .expect("the bundled mounting-hole definition is available");
    let operation = crate::matrix_fixtures::op(json!({
        "kind":"create-mirrored-half",
        "pairId":"pair",
        "sourceInstanceId":"left",
        "mode":{"kind":"new-board","boardId":"right","instanceId":"right"},
        "mapping":{"at":{"x":200,"y":0},"rotation":0},
        "idRule":"board-prefix",
        "companionMode":"literal-mirror",
        "mountingHole":mounting_hole
    }));
    crate::support::apply_for_test(&mut source, &[], &operation)
        .expect("the half with a closure is created");
    source
}

#[test]
fn prepared_matrix_pose_edits_preserve_authored_companions_from_both_sides() {
    for matrix_id in ["left-keys", "right-keys"] {
        for operation in [
            EditOperation::SetMatrixSplay {
                matrix_id: matrix_id.into(),
                column: 1,
                change: boardstudio_model::model::MatrixSplayChange::Angle {
                    angle: 12.0,
                    affect: boardstudio_model::model::MatrixSplayAffect::Following,
                },
            },
            EditOperation::SetMatrixPlacement {
                matrix_id: matrix_id.into(),
                change: boardstudio_model::model::MatrixPlacementChange::ColumnStagger {
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
                !boardstudio_core_mirrored_half::mirrored_half::reconcile::matrix_membership_changed(&accepted, &prepared, matrix_id),
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

            let (opened, _) = crate::support::prepare_open(&prepared, &[])
                .expect("the prepared authored companion state opens");
            let (reopened, findings) = crate::support::prepare_open(&opened, &[])
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
    let definition: boardstudio_core_mechanical::modules::ModuleDefinition = serde_json::from_value(json!({
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
        pose: boardstudio_model::model::Pose2 {
            at: Vec2 { x: 180.0, y: 7.0 },
            rotation: 0.0,
        },
        side: boardstudio_model::model::Side::Front,
        joins: Default::default(),
    };
    let canonical = crate::support::canonicalize(&accepted, &operation)
        .expect("the mirror embed has a canonical source operation");
    let mut source_oracle = accepted.clone();
    crate::support::apply_for_test(&mut source_oracle, &[], &canonical.operation)
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

pub(crate) fn framed_unlink_endpoint_document(linked_source: bool) -> (ProjectDoc, Constraint) {
    let mut document = pair_document();
    add_linked_independent_part(&mut document);
    let visible_id = "right/RST";
    let linked = linked_endpoint("pair", "left/RST", visible_id);
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
    let accepted = engine.document_for_test().clone();
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
    assert_eq!(engine.document_for_test(), &accepted);
    assert!(engine.undo_for_test().is_empty());
    assert!(engine.redo_for_test().is_empty());
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
        .push(boardstudio_model::model::MirroredException {
            kind: boardstudio_model::model::MirroredItemKind::Part,
            mirror_id: "right/U1".into(),
            state: boardstudio_model::model::MirroredExceptionState::Local,
            companion_mode: None,
            companion_tweak: None,
            removed_copper: None,
        });
    let engine = open_pair_engine(accepted);
    let accepted = engine.document_for_test().clone();
    let original_local = part(engine.document_for_test(), "right/U1").pose.at;
    let original_source = part(engine.document_for_test(), "left/U1").pose.at;

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
    assert_eq!(
        part(engine.document_for_test(), "right/U1").pose.at,
        original_local
    );
    assert_eq!(engine.document_for_test(), &accepted);
    assert!(engine.undo_for_test().is_empty());
    assert!(engine.redo_for_test().is_empty());
}

#[test]
fn no_op_and_refused_mirror_previews_do_not_publish_history() {
    let engine = open_pair_engine(pair_document());
    let accepted = engine.document_for_test().clone();
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
    assert_eq!(engine.document_for_test(), &accepted);
    assert_eq!(preview, accepted);
    assert!(changed.is_empty());
    assert!(engine.undo_for_test().is_empty());
    assert!(engine.redo_for_test().is_empty());

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
    assert_eq!(engine.document_for_test(), &accepted);
    assert!(engine.undo_for_test().is_empty());
    assert!(engine.redo_for_test().is_empty());
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
        .push(boardstudio_model::model::MirroredException {
            kind: boardstudio_model::model::MirroredItemKind::Part,
            mirror_id: "right/U1".into(),
            state: boardstudio_model::model::MirroredExceptionState::Omitted,
            companion_mode: None,
            companion_tweak: None,
            removed_copper: None,
        });
    let engine = open_pair_engine(accepted);
    let accepted = engine.document_for_test().clone();

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
    assert_eq!(engine.document_for_test(), &accepted);
    assert!(engine.undo_for_test().is_empty());
    assert!(engine.redo_for_test().is_empty());
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
    let accepted = engine.document_for_test().clone();
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
    assert_eq!(engine.document_for_test(), &accepted);
    assert!(engine.undo_for_test().is_empty());
    assert!(engine.redo_for_test().is_empty());
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
        .push(boardstudio_model::model::MirroredException {
            kind: boardstudio_model::model::MirroredItemKind::Part,
            mirror_id: "right/RST".into(),
            state: boardstudio_model::model::MirroredExceptionState::Local,
            companion_mode: None,
            companion_tweak: None,
            removed_copper: None,
        });
    let mut engine = open_pair_engine(accepted);
    let accepted = engine.document_for_test().clone();
    let operation = EditOperation::MoveParts {
        positions: vec![Position {
            id: "right/U1".into(),
            at: Vec2 { x: 216.0, y: 14.0 },
        }],
    };
    let (prepared, changed) = prepare_transaction(&accepted, &[], &operation)
        .expect("the paired edit prepares without publishing");
    let (cache, contours, findings) = boardstudio_core_kernel::geometry::outlines(
        &prepared,
        Some(engine.outline_cache_for_test()),
        &changed,
    );
    let reply = engine.publish_prepared_for_test(
        "commit-prepared".into(),
        "commit-prepared".into(),
        prepared,
        changed,
        cache,
        contours,
        findings,
    );
    assert!(matches!(reply, CoreReply::Scene { .. }));

    assert_eq!(engine.undo_for_test().len(), 1);
    assert!(engine.redo_for_test().is_empty());
    assert_eq!(
        part(engine.document_for_test(), "left/U1").pose.at,
        Vec2 { x: -16.0, y: 14.0 }
    );
    assert_eq!(
        part(engine.document_for_test(), "right/U1").pose.at,
        Vec2 { x: 216.0, y: 14.0 }
    );
    assert_eq!(
        part(engine.document_for_test(), "right/RST").pose.at,
        Vec2 { x: 260.0, y: 25.0 }
    );
    assert!(
        boardstudio_core_mechanical::mechanical::halves::validate_document(
            engine.document_for_test()
        )
        .is_ok()
    );

    let undo = engine.handle(CoreRequest::Undo {
        id: "undo-prepared".into(),
    });
    assert!(matches!(undo, CoreReply::Scene { .. }));
    assert_eq!(engine.undo_for_test().len(), 0);
    assert_eq!(engine.redo_for_test().len(), 1);
    let mut expected_undo = accepted.clone();
    expected_undo.revision = engine.document_for_test().revision;
    assert_eq!(engine.document_for_test(), &expected_undo);

    let redo = engine.handle(CoreRequest::Redo {
        id: "redo-prepared".into(),
    });
    assert!(matches!(redo, CoreReply::Scene { .. }));
    assert_eq!(engine.undo_for_test().len(), 1);
    assert!(engine.redo_for_test().is_empty());
    assert_eq!(
        part(engine.document_for_test(), "left/U1").pose.at,
        Vec2 { x: -16.0, y: 14.0 }
    );
    assert_eq!(
        part(engine.document_for_test(), "right/U1").pose.at,
        Vec2 { x: 216.0, y: 14.0 }
    );
    assert_eq!(
        part(engine.document_for_test(), "right/RST").pose.at,
        Vec2 { x: 260.0, y: 25.0 }
    );
    assert_eq!(engine.document_for_test().revision, accepted.revision + 3);
    assert!(
        boardstudio_core_mechanical::mechanical::halves::validate_document(
            engine.document_for_test()
        )
        .is_ok()
    );
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

    let (reopened, findings) = crate::support::prepare_open(&independent, &[])
        .expect("the independent Unlink result prepares as a valid Open input");

    assert_eq!(reopened, independent);
    assert!(findings.is_empty());
}

#[test]
fn open_preparation_repairs_linked_geometry_and_reports_a_located_warning() {
    let mut accepted = pair_document();
    part_mut(&mut accepted, "right/U1").pose.at.x = 999.0;
    let accepted_bytes = serde_json::to_vec(&accepted).unwrap();

    let (prepared, findings) = crate::support::prepare_open(&accepted, &[])
        .expect("the source-authoritative Open draft prepares");

    assert_eq!(part(&prepared, "right/U1").pose.at.x, 224.0);
    assert_eq!(findings.len(), 1);
    assert_eq!(
        findings[0].severity,
        boardstudio_model::model::Severity::Warning
    );
    assert!(findings[0].target_ids.contains(&"right/U1".to_owned()));
    assert_eq!(serde_json::to_vec(&accepted).unwrap(), accepted_bytes);

    let (reopened, second_findings) = crate::support::prepare_open(&prepared, &[])
        .expect("the repaired document is a valid second Open input");
    assert_eq!(reopened, prepared);
    assert!(second_findings.is_empty());

    accepted.format_version = boardstudio_core_kernel::migrate::CURRENT_VERSION;
    let mut engine = boardstudio_core_engine::CoreEngine::new();
    let CoreReply::Scene {
        mut scene,
        document,
        ..
    } = engine.handle(CoreRequest::Open {
        id: "open".into(),
        document: accepted,
    })
    else {
        panic!("expected Core to open and repair the document");
    };
    assert!(scene.findings.contains(&findings[0]));
    "snapshot".clone_into(&mut scene.transaction_id);
    assert_eq!(
        boardstudio_model::wire::encode_core_reply(&engine.handle(CoreRequest::Snapshot {
            id: "snapshot".into(),
        })),
        boardstudio_model::wire::encode_core_reply(&CoreReply::Scene {
            id: "snapshot".into(),
            scene,
            document,
        }),
    );
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
    boardstudio_core_kernel::matrix::set_matrix(&mut accepted, &source_matrix)
        .expect("the ordinary source matrix materializes authored cell fields");

    let (opened, findings) = crate::support::prepare_open(&accepted, &[])
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
    let original_cells: Vec<boardstudio_model::model::MatrixCell> =
        serde_json::from_value(source_json["matrices"][0]["cells"].clone()).unwrap();
    let mut accepted: ProjectDoc = serde_json::from_value(source_json).unwrap();
    let create: EditOperation = serde_json::from_value(
        synthetic_half_fixture::create_mirrored_half_operation("pair-modern", "right"),
    )
    .unwrap();
    crate::support::apply_for_test(&mut accepted, &[], &create)
        .expect("the unchanged modern fixture pairs");
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

    let (opened, _) = crate::support::prepare_open(&accepted, &[])
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
    let engine = boardstudio_core_engine::CoreEngine::with_document_for_test(opened.clone());
    let accepted_before_preview = engine.document_for_test().clone();
    let (preview, _) = prepare_preview(&engine, &operation)
        .expect("private preview preparation keeps modern local cells");
    assert_eq!(
        engine.document_for_test(),
        &accepted_before_preview,
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
            association.kind == boardstudio_model::model::MirroredItemKind::Part
                && association.source_id.starts_with("matrix/")
                && association.source_id.ends_with("/diode")
        })
        .unwrap()
        .mirror_id
        .clone();
    part_mut(&mut accepted, &target_id).pose.at.x += 123.0;
    let accepted_bytes = serde_json::to_vec(&accepted).unwrap();

    let (prepared, findings) = crate::support::prepare_open(&accepted, &[])
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
    let second_matrix = crate::matrix_fixtures::new_matrix("left-extra", "left", (30.0, 40.0), 1);
    let accepted = crate::matrix_fixtures::add_matrix(&initial, &second_matrix);
    let expected: Vec<_> = accepted
        .parts
        .iter()
        .filter(|part| part.id.starts_with("matrix/right-"))
        .cloned()
        .collect();

    let (prepared, findings) = crate::support::prepare_open(&accepted, &[])
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
            change: boardstudio_model::model::KeycapBoardChange::Color {
                value: "#112233".into(),
            },
        },
        EditOperation::SetKeycapBoard {
            board_id: "right".into(),
            change: boardstudio_model::model::KeycapBoardChange::Color {
                value: "#abcdef".into(),
            },
        },
        EditOperation::SetMatrixKeycaps {
            matrix_id: "left-keys".into(),
            change: boardstudio_model::model::KeycapMatrixChange::Profile {
                value: Some(boardstudio_model::model::KeycapProfile::Choc),
            },
        },
        EditOperation::SetKeycapKey {
            key_id: "matrix/left-keys/r0c0".into(),
            change: boardstudio_model::model::KeycapKeyChange::Legend {
                value: Some("Q".into()),
            },
        },
        EditOperation::SetKeycapKey {
            key_id: "matrix/right-keys/r0c0".into(),
            change: boardstudio_model::model::KeycapKeyChange::Legend {
                value: Some("wrong".into()),
            },
        },
    ] {
        crate::support::apply_for_test(&mut accepted, &[], &operation)
            .expect("the keycap edit applies");
    }

    let (prepared, findings) = crate::support::prepare_open(&accepted, &[])
        .expect("linked keycap settings are restored from their source");

    let keycaps = prepared.keycaps.as_ref().unwrap();
    assert_eq!(keycaps.boards["right"].color, "#112233");
    assert_eq!(
        keycaps.matrices["right-keys"].profile,
        Some(boardstudio_model::model::KeycapProfile::Choc)
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
