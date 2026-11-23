use super::*;
use crate::support::fixture;

#[test]
fn preview_does_not_commit_and_stale_edit_fails() {
    let mut engine = CoreEngine::new();
    let mut doc = ProjectDoc::empty("p", "Project");
    doc.definitions.push(fixture::part_definition(
        "switch",
        "Switch",
        PartKind::Switch,
    ));
    doc.parts.push(fixture::part("k1", "switch", "SW1"));
    engine.open(doc).unwrap();
    let op = EditOperation::MoveParts {
        positions: vec![Position {
            id: "k1".into(),
            at: Vec2 { x: 10.0, y: 5.0 },
        }],
    };
    let preview = preview(engine.apply_edit(edit(0, EditPhase::Preview, op.clone())));
    assert_eq!(preview.revision, 0);
    assert_eq!(preview.transforms[0].pose.at.x, 10.0);
    let (_, snapshot) = scene(engine.snapshot());
    assert_eq!(snapshot.parts[0].pose.at.x, 0.0);
    assert!(engine.undo().is_err());
    let failed = EditOperation::MoveParts {
        positions: vec![
            Position {
                id: "k1".into(),
                at: Vec2 { x: 20.0, y: 0.0 },
            },
            Position {
                id: "missing".into(),
                at: Vec2::default(),
            },
        ],
    };
    assert!(
        engine
            .apply_edit(edit(0, EditPhase::Preview, failed))
            .is_err()
    );
    let (_, snapshot) = scene(engine.snapshot());
    assert_eq!(snapshot.parts[0].pose.at.x, 0.0);
    let (committed, _) = scene(engine.commit_edit(edit(0, EditPhase::Commit, op.clone())));
    assert_eq!(committed.revision, 1);
    assert!(engine.commit_edit(edit(0, EditPhase::Commit, op)).is_err());
}

#[test]
fn undo_redo_increments_revision() {
    let mut engine = CoreEngine::new();
    // Ticket 37: raw SetOutline is deleted; the typed case write is the live
    // op for this undo/redo accounting.
    let mut doc = ProjectDoc::empty("p", "Project");
    doc.boards.push(fixture::board("board", "Board"));
    scene(engine.open(doc));
    let add = || EditOperation::SetCase {
        body: CaseBody {
            features: None,
            openings: None,
            id: "case".into(),
            name: "Case".into(),
            board_id: "board".into(),
            kind: CaseKind::Plate,
            thickness: 2.0,
            clearance: 0.0,
            material_id: None,
            z: None,
            wall_height: None,
            wall_thickness: None,
            mounts: None,
            pcb_source: None,
            gasket: None,
        },
    };
    scene(engine.commit_edit(edit(0, EditPhase::Commit, add())));
    let (undo, doc) = scene(engine.undo());
    assert_eq!(undo.revision, 2);
    assert!(doc.case_bodies.is_empty());
    let (redo, doc) = scene(engine.redo());
    assert_eq!(redo.revision, 3);
    assert_eq!(doc.case_bodies.len(), 1);
}

#[test]
fn wire_roundtrip_matches_contract() {
    let request = r#"{"id":"snap","kind":"snapshot"}"#;
    let mut engine = CoreEngine::new();
    let reply: CoreReply = serde_json::from_str(&engine.request(request)).unwrap();
    let CoreReply::Scene { document, .. } = reply else {
        panic!("expected scene: {reply:?}")
    };
    let doc = *document;
    assert_eq!(doc.format, "boardstudio/v2");
    assert_eq!(
        serde_json::to_value(&doc).unwrap()["caseBodies"],
        serde_json::json!([])
    );
}

#[test]
fn mechanical_holes_do_not_need_pad_numbers() {
    let mut doc = matrix_doc();
    doc.definitions[0].pads = vec!["center", "left", "right"]
        .into_iter()
        .map(|id| Pad {
            id: id.into(),
            number: String::new(),
            at: Vec2::default(),
            size: Vec2 { x: 2.0, y: 2.0 },
            shape: PadShape::Circle,
            drill: Some(2.0),
            plated: Some(false),
            side: None,
            rotation: None,
            net_id: None,
        })
        .collect();
    doc.parts.push(fixture::part("switch", "switch", "SW1"));
    let mut engine = CoreEngine::new();
    let (scene, _) = scene(engine.open(doc));
    assert!(
        !scene
            .findings
            .iter()
            .any(|finding| finding.id.contains(":number")
                || finding.id.contains(":duplicate-number"))
    );
}

#[test]
fn part_envelope_tracks_committed_move() {
    let mut engine = CoreEngine::new();
    let mut doc = ProjectDoc::empty("p", "Project");
    doc.definitions.push(PartDefinition {
        courtyard: vec![
            Vec2 { x: -1.0, y: -1.0 },
            Vec2 { x: 1.0, y: -1.0 },
            Vec2 { x: 1.0, y: 1.0 },
            Vec2 { x: -1.0, y: 1.0 },
        ],
        ..fixture::part_definition("switch", "Switch", PartKind::Switch)
    });
    doc.parts.push(fixture::part("k1", "switch", "SW1"));
    doc.outline.push(OutlineFeature::PartEnvelope {
        connections: vec![],
        id: "boundary".into(),
        part_ids: vec!["k1".into()],
        settings: Default::default(),
        margin: 1.0,
        operation: Operation::Add,
    });
    let (before, _) = scene(engine.open(doc));
    let op = EditOperation::MoveParts {
        positions: vec![Position {
            id: "k1".into(),
            at: Vec2 { x: 10.0, y: 0.0 },
        }],
    };
    let (after, _) = scene(engine.commit_edit(edit(0, EditPhase::Commit, op)));
    let before_min = before.contours[0]
        .points
        .iter()
        .map(|p| p.x)
        .fold(f64::INFINITY, f64::min);
    let after_min = after.contours[0]
        .points
        .iter()
        .map(|p| p.x)
        .fold(f64::INFINITY, f64::min);
    assert_eq!(after_min - before_min, 10.0);
}

#[test]
fn legacy_scripts_load_without_execution_or_reserialization() {
    let mut json = serde_json::to_value(ProjectDoc::empty("p", "Project")).unwrap();
    json["scripts"] = serde_json::json!([{ "id": "legacy", "name": "Legacy", "enabled": true, "source": "while true {}" }]);
    let doc: ProjectDoc = serde_json::from_value(json).unwrap();
    let mut engine = CoreEngine::new();
    let (_, opened) = scene(engine.open(doc));
    assert!(opened.parts.is_empty());
    assert_eq!(
        serde_json::to_value(opened).unwrap()["scripts"],
        serde_json::json!([])
    );
    let _: ProjectDoc =
        serde_json::from_value(serde_json::to_value(ProjectDoc::empty("p", "Project")).unwrap())
            .unwrap();
}

#[test]
fn add_part_targets_board_and_removal_cleans_both_boards() {
    let mut engine = CoreEngine::new();
    let mut doc = matrix_doc();
    doc.outline.push(OutlineFeature::PartEnvelope {
        connections: vec![],
        id: "right-edge".into(),
        part_ids: vec![],
        settings: Default::default(),
        margin: 3.0,
        operation: Operation::Add,
    });
    doc.boards.push(Board {
        outline_ids: vec!["right-edge".into()],
        ..fixture::board("right", "Right")
    });
    engine.open(doc).unwrap();
    let part = fixture::part("k1", "switch", "SW1");
    let op = EditOperation::AddPart {
        part: part.clone(),
        board_id: Some("right".into()),
        definition: None,
        layout_id: None,
    };
    let preview = preview(engine.apply_edit(edit(0, EditPhase::Preview, op.clone())));
    assert!(preview.board_readiness[1].pcb);
    assert!(!preview.board_readiness[0].pcb);
    assert!(scene(engine.snapshot()).1.parts.is_empty());
    assert!(engine.undo().is_err());
    let (_, added) = scene(engine.commit_edit(edit(0, EditPhase::Commit, op)));
    assert!(added.boards[0].part_ids.is_empty());
    assert_eq!(added.boards[1].part_ids, vec!["k1"]);
    let OutlineFeature::PartEnvelope { part_ids: left, .. } = &added.outline[0] else {
        panic!("expected envelope")
    };
    let OutlineFeature::PartEnvelope {
        part_ids: right, ..
    } = &added.outline[1]
    else {
        panic!("expected envelope")
    };
    assert!(left.is_empty());
    assert_eq!(right, &vec!["k1"]);
    let mut invalid = EditOperation::AddPart {
        part,
        board_id: Some("missing".into()),
        definition: None,
        layout_id: None,
    };
    assert!(
        engine
            .commit_edit(edit(1, EditPhase::Commit, invalid.clone()))
            .is_err()
    );
    if let EditOperation::AddPart { part, .. } = &mut invalid {
        part.id = "k2".into();
    }
    assert!(
        engine
            .commit_edit(edit(1, EditPhase::Commit, invalid))
            .is_err()
    );
    let (_, removed) = scene(engine.commit_edit(edit(
        1,
        EditPhase::Commit,
        EditOperation::RemoveParts {
            ids: vec!["k1".into()],
        },
    )));
    assert!(removed.boards.iter().all(|board| board.part_ids.is_empty()));
    assert!(removed.outline.iter().all(|feature| matches!(feature, OutlineFeature::PartEnvelope { part_ids, .. } if part_ids.is_empty())));
}

#[test]
fn empty_definition_and_pad_ids_are_reported() {
    let mut engine = CoreEngine::new();
    let mut doc = ProjectDoc::empty("p", "Project");
    doc.definitions.push(PartDefinition {
        pads: vec![Pad {
            id: "".into(),
            number: "".into(),
            at: Vec2 { x: 0.0, y: 0.0 },
            size: Vec2 { x: 1.0, y: 1.0 },
            shape: PadShape::Oval,
            drill: Some(f64::INFINITY),
            plated: None,
            side: None,
            rotation: None,
            net_id: None,
        }],
        ..fixture::part_definition("", "Custom", PartKind::Custom)
    });
    doc.parts.push(fixture::part("p", "", "U1"));
    let (scene, _) = scene(engine.open(doc));
    for suffix in ["id", "pad:0:id", "pad:0:drill"] {
        assert!(
            scene
                .findings
                .iter()
                .any(|finding| finding.id == format!("definition::{suffix}")),
            "missing {suffix}"
        );
    }
}

#[test]
fn add_part_refuses_definition_id_mismatch_without_mutation() {
    let (mut engine, document) = linked_pair();
    let mut definition = document.definitions[0].clone();
    definition.id = "catalogue/other-switch".into();
    let mut part = document.parts[0].clone();
    part.id = "board/mismatched-definition".into();
    part.reference = "MISMATCH1".into();
    let reply = engine.commit_edit(edit(
        document.revision,
        EditPhase::Commit,
        EditOperation::AddPart {
            part,
            board_id: Some("board".into()),
            definition: Some(definition),
            layout_id: Some("left-layout".into()),
        },
    ));
    assert!(reply.is_err(), "{reply:?}");
    let (_, after_refusal) = scene(engine.snapshot());
    assert_eq!(after_refusal, document);
}

#[test]
fn matrix_cells_preserve_survivors_and_companions() {
    let mut engine = CoreEngine::new();
    engine.open(matrix_doc()).unwrap();
    let mut value = matrix(2, 2);
    {
        let values = vec![Vec2 { x: 0.0, y: 0.0 }, Vec2 { x: 3.0, y: 0.0 }];
        value
            .row_layout
            .resize_with(values.len().max(value.row_layout.len()), Default::default);
        for record in &mut value.row_layout {
            record.offset = Vec2::default();
        }
        for (record, value) in value.row_layout.iter_mut().zip(values) {
            record.offset = value;
        }
    }
    value.cells = vec![
        MatrixCell {
            shift: None,
            bend: None,
            deleted: false,
            assemblies_local: None,
            row: 0,
            column: 1,
            enabled: false,

            definition_id: None,
            variant: None,
            offset: None,
            rotation: None,
            assemblies: vec![],
        },
        MatrixCell {
            shift: None,
            bend: None,
            deleted: false,
            assemblies_local: None,
            row: 1,
            column: 0,
            enabled: true,

            definition_id: None,
            variant: Some("hotswap".into()),
            offset: Some(Vec2 { x: 2.0, y: 1.0 }),
            rotation: Some(10.0),
            assemblies: vec![MatrixAssembly {
                id: "led".into(),
                definition_id: "switch".into(),
                offset: Vec2 { x: 0.0, y: 4.0 },
                rotation: None,
                side: None,
            }],
        },
    ];
    let (_, doc) = scene(engine.commit_edit(edit(
        0,
        EditPhase::Commit,
        fixture::set_matrix(value.clone()),
    )));
    assert_eq!(doc.matrices[0].part_ids.len(), 4);
    assert!(!doc.parts.iter().any(|part| part.id == "matrix/main/r0c1"));
    let host = doc
        .parts
        .iter()
        .find(|part| part.id == "matrix/main/r1c0")
        .unwrap();
    assert_eq!(host.pose.at, Vec2 { x: 5.0, y: 20.0 });
    assert_eq!(host.pose.rotation, 10.0);
    assert_eq!(host.properties.as_ref().unwrap()["variant"], "hotswap");
    let companion = doc
        .parts
        .iter()
        .find(|part| part.id == "matrix/main/r1c0/led")
        .unwrap();
    assert!((companion.pose.at.x - 4.305407289332279).abs() < 1e-9);
    assert!((companion.pose.at.y - 23.939231012048833).abs() < 1e-9);
    let reference = host.reference.clone();
    value.columns = 3;
    let (_, grown) = scene(engine.commit_edit(edit(
        1,
        EditPhase::Commit,
        fixture::set_matrix(value.clone()),
    )));
    assert_eq!(
        grown
            .parts
            .iter()
            .find(|part| part.id == host.id)
            .unwrap()
            .reference,
        reference
    );
    assert!(grown.parts.iter().any(|part| part.id == "matrix/main/r0c2"));
}

#[test]
fn matrix_registers_definitions_atomically() {
    let mut doc = matrix_doc();
    let definitions = std::mem::take(&mut doc.definitions);
    let mut engine = CoreEngine::new();
    engine.open(doc).unwrap();
    let operation = EditOperation::SetMatrix {
        matrix: matrix(1, 1),
        definitions: Some(definitions),
    };
    let projected = preview(engine.apply_edit(edit(0, EditPhase::Preview, operation.clone())));
    assert_eq!(projected.transforms.len(), 1);
    let (_, snapshot) = scene(engine.snapshot());
    assert!(snapshot.definitions.is_empty());
    assert!(snapshot.parts.is_empty());
    assert!(engine.undo().is_err());
    let (_, placed) = scene(engine.commit_edit(edit(0, EditPhase::Commit, operation)));
    assert_eq!(placed.definitions.len(), 1);
    assert_eq!(placed.parts.len(), 1);
    let mut conflicting = placed.definitions[0].clone();
    conflicting.name = "Changed".into();
    assert!(
        engine
            .commit_edit(edit(
                1,
                EditPhase::Commit,
                EditOperation::SetMatrix {
                    matrix: matrix(1, 1),
                    definitions: Some(vec![conflicting]),
                }
            ))
            .is_err()
    );
    let (_, undone) = scene(engine.undo());
    assert!(undone.definitions.is_empty());
    assert!(undone.parts.is_empty());
}

#[test]
fn matrix_mirror_rotation_and_bad_dimensions() {
    let mut engine = CoreEngine::new();
    engine.open(matrix_doc()).unwrap();
    let mut value = matrix(1, 2);
    value.mirror = Some(Mirror::X);
    value.rotation = Some(90.0);
    let (_, doc) =
        scene(engine.commit_edit(edit(0, EditPhase::Commit, fixture::set_matrix(value))));
    let second = doc
        .parts
        .iter()
        .find(|part| part.id == "matrix/main/r0c1")
        .unwrap();
    assert!((second.pose.at.x).abs() < 1e-9);
    assert!((second.pose.at.y + 19.0).abs() < 1e-9);
    assert_eq!(doc.matrices[0].edge_gap, Some(Vec2 { x: 1.0, y: 1.0 }));
    let mut bad_gap = matrix(1, 1);
    bad_gap.edge_gap = Some(Vec2 { x: 19.0, y: 1.0 });
    assert!(
        engine
            .commit_edit(edit(
                1,
                EditPhase::Commit,
                EditOperation::SetMatrix {
                    matrix: bad_gap,
                    definitions: None,
                }
            ))
            .is_err()
    );
    let bad = matrix(0, 3);
    assert!(
        engine
            .commit_edit(edit(
                1,
                EditPhase::Commit,
                EditOperation::SetMatrix {
                    matrix: bad,
                    definitions: None,
                }
            ))
            .is_err()
    );
}

#[test]
fn deleting_matrix_removes_its_container_and_parts_in_one_edit() {
    let mut engine = CoreEngine::new();
    engine.open(matrix_doc()).unwrap();
    let (_, before) = scene(engine.commit_edit(edit(
        0,
        EditPhase::Commit,
        fixture::set_matrix(matrix(2, 2)),
    )));
    let operation: EditOperation =
        serde_json::from_value(serde_json::json!({"kind":"remove-matrix", "id":"main"})).unwrap();
    let (_, removed) =
        scene(engine.commit_edit(edit(before.revision, EditPhase::Commit, operation)));
    assert!(removed.matrices.is_empty());
    assert!(
        !removed
            .parts
            .iter()
            .any(|part| before.matrices[0].part_ids.contains(&part.id))
    );
    let (_, restored) = scene(engine.undo());
    assert_eq!(restored.matrices, before.matrices);
    assert_eq!(restored.parts, before.parts);
    let (_, deleted_again) = scene(engine.redo());
    assert!(deleted_again.matrices.is_empty());
    let (_, restored) = scene(engine.undo());
    let (_, empty) = scene(engine.commit_edit(edit(
        restored.revision,
        EditPhase::Commit,
        EditOperation::RemoveParts {
            ids: restored.matrices[0].part_ids.clone(),
        },
    )));
    assert!(empty.matrices[0].part_ids.is_empty());
    let (_, removed_empty) = scene(engine.commit_edit(edit(
        empty.revision,
        EditPhase::Commit,
        EditOperation::RemoveMatrix { id: "main".into() },
    )));
    assert!(removed_empty.matrices.is_empty());
    let (_, restored_empty) = scene(engine.undo());
    assert_eq!(restored_empty.matrices, empty.matrices);
}

#[test]
fn deleting_last_matrix_row_removes_matrix_and_key_metadata_and_is_undoable() {
    let mut engine = CoreEngine::new();
    engine.open(matrix_doc()).unwrap();
    let (_, mut doc) = scene(engine.commit_edit(edit(
        0,
        EditPhase::Commit,
        fixture::set_matrix(matrix(1, 1)),
    )));
    doc.keymap = Some(KeymapConfiguration::default());
    doc.keymap.as_mut().unwrap().layers[0]
        .bindings
        .insert("matrix/main/r0c0".into(), KeyBinding::None);
    engine.open(doc.clone()).unwrap();
    let (_, removed) = scene(engine.commit_edit(edit(
        doc.revision,
        EditPhase::Commit,
        EditOperation::RemoveMatrixRow {
            matrix_id: "main".into(),
            row: 0,
        },
    )));
    assert!(removed.matrices.is_empty());
    assert!(removed.parts.is_empty());
    assert!(removed.keymap.unwrap().layers[0].bindings.is_empty());
    let (_, restored) = scene(engine.undo());
    assert_eq!(restored.matrices, doc.matrices);
    assert_eq!(restored.keymap, doc.keymap);
}
