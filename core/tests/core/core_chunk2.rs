use super::*;

#[test]
fn preview_does_not_commit_and_stale_edit_fails() {
    let mut engine = CoreEngine::new();
    let mut doc = ProjectDoc::empty("p", "Project");
    doc.definitions.push(PartDefinition {
        hardware_profile: None,
        input_profile: None,
        mechanical_profile: None,
        id: "switch".into(),
        name: "Switch".into(),
        kind: PartKind::Switch,
        courtyard: vec![],
        pads: vec![],
        models: None,
        keycap: None,
        envelope_source: None,
        kicad_source: None,
        terminals: Default::default(),
        matrix_terminals: None,
        envelope_notice: None,
        generator: None,
    });
    doc.parts.push(Part {
        anchor: None,
        id: "k1".into(),
        definition_id: "switch".into(),
        reference: "SW1".into(),
        pose: Pose2 {
            at: Vec2 { x: 0.0, y: 0.0 },
            rotation: 0.0,
        },
        side: Side::Front,
        locked: None,
        keycap: None,
        outline: None,
        properties: None,
        generator_parameters: None,
    });
    engine.handle(CoreRequest::Open {
        id: "open".into(),
        document: doc,
    });
    let op = EditOperation::MoveParts {
        positions: vec![Position {
            id: "k1".into(),
            at: Vec2 { x: 10.0, y: 5.0 },
        }],
    };
    let preview = preview(engine.handle(edit(0, EditPhase::Preview, op.clone())));
    assert_eq!(preview.revision, 0);
    assert_eq!(preview.transforms[0].pose.at.x, 10.0);
    let (_, snapshot) = scene(engine.handle(CoreRequest::Snapshot { id: "snap".into() }));
    assert_eq!(snapshot.parts[0].pose.at.x, 0.0);
    assert!(matches!(
        engine.handle(CoreRequest::Undo { id: "undo".into() }),
        CoreReply::Error { .. }
    ));
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
    assert!(matches!(
        engine.handle(edit(0, EditPhase::Preview, failed)),
        CoreReply::Error { .. }
    ));
    let (_, snapshot) = scene(engine.handle(CoreRequest::Snapshot { id: "snap".into() }));
    assert_eq!(snapshot.parts[0].pose.at.x, 0.0);
    let (committed, _) = scene(engine.handle(edit(0, EditPhase::Commit, op.clone())));
    assert_eq!(committed.revision, 1);
    assert!(matches!(
        engine.handle(edit(0, EditPhase::Commit, op)),
        CoreReply::Error { revision: 1, .. }
    ));
}

#[test]
fn undo_redo_increments_revision() {
    let mut engine = CoreEngine::new();
    // Ticket 37: raw SetOutline is deleted; the typed case write is the live
    // op for this undo/redo accounting.
    let mut doc = ProjectDoc::empty("p", "Project");
    doc.boards.push(Board {
        id: "board".into(),
        name: "Board".into(),
        outline_ids: vec![],
        part_ids: vec![],
        net_ids: vec![],
        thickness: 1.6,
        traces: vec![],
        vias: vec![],
    });
    scene(engine.handle(CoreRequest::Open {
        id: "open".into(),
        document: doc,
    }));
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
    scene(engine.handle(edit(0, EditPhase::Commit, add())));
    let (undo, doc) = scene(engine.handle(CoreRequest::Undo { id: "undo".into() }));
    assert_eq!(undo.revision, 2);
    assert!(doc.case_bodies.is_empty());
    let (redo, doc) = scene(engine.handle(CoreRequest::Redo { id: "redo".into() }));
    assert_eq!(redo.revision, 3);
    assert_eq!(doc.case_bodies.len(), 1);
}

#[test]
fn wire_roundtrip_matches_contract() {
    let request = r#"{"id":"snap","kind":"snapshot"}"#;
    let mut engine = CoreEngine::new();
    let reply: CoreReply = serde_json::from_str(&engine.request(request)).unwrap();
    let (_, doc) = scene(reply);
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
    doc.parts.push(Part {
        anchor: None,
        id: "switch".into(),
        definition_id: "switch".into(),
        reference: "SW1".into(),
        pose: Pose2 {
            at: Vec2::default(),
            rotation: 0.0,
        },
        side: Side::Front,
        locked: None,
        keycap: None,
        outline: None,
        properties: None,
        generator_parameters: None,
    });
    let mut engine = CoreEngine::new();
    let (scene, _) = scene(engine.handle(CoreRequest::Open {
        id: "open".into(),
        document: doc,
    }));
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
        hardware_profile: None,
        input_profile: None,
        mechanical_profile: None,
        id: "switch".into(),
        name: "Switch".into(),
        kind: PartKind::Switch,
        courtyard: vec![
            Vec2 { x: -1.0, y: -1.0 },
            Vec2 { x: 1.0, y: -1.0 },
            Vec2 { x: 1.0, y: 1.0 },
            Vec2 { x: -1.0, y: 1.0 },
        ],
        pads: vec![],
        models: None,
        keycap: None,
        envelope_source: None,
        kicad_source: None,
        terminals: Default::default(),
        matrix_terminals: None,
        envelope_notice: None,
        generator: None,
    });
    doc.parts.push(Part {
        anchor: None,
        id: "k1".into(),
        definition_id: "switch".into(),
        reference: "SW1".into(),
        pose: Pose2 {
            at: Vec2 { x: 0.0, y: 0.0 },
            rotation: 0.0,
        },
        side: Side::Front,
        locked: None,
        keycap: None,
        outline: None,
        properties: None,
        generator_parameters: None,
    });
    doc.outline.push(OutlineFeature::PartEnvelope {
        connections: vec![],
        id: "boundary".into(),
        part_ids: vec!["k1".into()],
        settings: Default::default(),
        margin: 1.0,
        operation: Operation::Add,
    });
    let (before, _) = scene(engine.handle(CoreRequest::Open {
        id: "open".into(),
        document: doc,
    }));
    let op = EditOperation::MoveParts {
        positions: vec![Position {
            id: "k1".into(),
            at: Vec2 { x: 10.0, y: 0.0 },
        }],
    };
    let (after, _) = scene(engine.handle(edit(0, EditPhase::Commit, op)));
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
    let (_, opened) = scene(engine.handle(CoreRequest::Open {
        id: "open".into(),
        document: doc,
    }));
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
        id: "right".into(),
        name: "Right".into(),
        outline_ids: vec!["right-edge".into()],
        part_ids: vec![],
        net_ids: vec![],
        thickness: 1.6,
        traces: vec![],
        vias: vec![],
    });
    engine.handle(CoreRequest::Open {
        id: "open".into(),
        document: doc,
    });
    let part = Part {
        anchor: None,
        id: "k1".into(),
        definition_id: "switch".into(),
        reference: "SW1".into(),
        pose: Pose2 {
            at: Vec2 { x: 0.0, y: 0.0 },
            rotation: 0.0,
        },
        side: Side::Front,
        locked: None,
        keycap: None,
        outline: None,
        properties: None,
        generator_parameters: None,
    };
    let op = EditOperation::AddPart {
        part: part.clone(),
        board_id: Some("right".into()),
        definition: None,
        layout_id: None,
    };
    let preview = preview(engine.handle(edit(0, EditPhase::Preview, op.clone())));
    assert!(preview.board_readiness[1].pcb);
    assert!(!preview.board_readiness[0].pcb);
    assert!(
        scene(engine.handle(CoreRequest::Snapshot {
            id: "snapshot".into()
        }))
        .1
        .parts
        .is_empty()
    );
    assert!(matches!(
        engine.handle(CoreRequest::Undo { id: "undo".into() }),
        CoreReply::Error { .. }
    ));
    let (_, added) = scene(engine.handle(edit(0, EditPhase::Commit, op)));
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
    assert!(matches!(
        engine.handle(edit(1, EditPhase::Commit, invalid.clone())),
        CoreReply::Error { .. }
    ));
    if let EditOperation::AddPart { part, .. } = &mut invalid {
        part.id = "k2".into();
    }
    assert!(matches!(
        engine.handle(edit(1, EditPhase::Commit, invalid)),
        CoreReply::Error { .. }
    ));
    let (_, removed) = scene(engine.handle(edit(
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
        hardware_profile: None,
        input_profile: None,
        mechanical_profile: None,
        id: "".into(),
        name: "Custom".into(),
        kind: PartKind::Custom,
        courtyard: vec![],
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
        models: None,
        keycap: None,
        envelope_source: None,
        kicad_source: None,
        terminals: Default::default(),
        matrix_terminals: None,
        envelope_notice: None,
        generator: None,
    });
    doc.parts.push(Part {
        anchor: None,
        id: "p".into(),
        definition_id: "".into(),
        reference: "U1".into(),
        pose: Pose2 {
            at: Vec2 { x: 0.0, y: 0.0 },
            rotation: 0.0,
        },
        side: Side::Front,
        locked: None,
        keycap: None,
        outline: None,
        properties: None,
        generator_parameters: None,
    });
    let (scene, _) = scene(engine.handle(CoreRequest::Open {
        id: "open".into(),
        document: doc,
    }));
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
fn public_add_part_from_source_board_creates_one_mirror_correspondence() {
    let (mut engine, document) = public_native_pair();
    let halves = document.hardware.as_ref().unwrap().pairs[0]
        .halves
        .as_ref()
        .unwrap();
    let response = engine.request(&serde_json::json!({
        "kind": "edit",
        "id": "source-add",
        "command": {
            "baseRevision": document.revision,
            "transactionId": "source-add",
            "phase": "commit",
            "targetIds": [],
            "operation": {
                "kind": "add-part",
                "boardId": halves.source_board_id,
                "part": {
                    "id": "source/J1", "definitionId": "generator:ceoloide/switch_mx", "reference": "J1",
                    "pose": {"at": {"x": 8.0, "y": 12.0}, "rotation": 0.0}, "side": "front"
                }
            }
        }
    }).to_string());
    let (_, added) = scene(serde_json::from_str(&response).unwrap());
    let halves = added.hardware.as_ref().unwrap().pairs[0]
        .halves
        .as_ref()
        .unwrap();
    assert_eq!(
        added
            .parts
            .iter()
            .filter(|part| part.id == "source/J1")
            .count(),
        1
    );
    assert_eq!(
        halves
            .correspondences
            .iter()
            .filter(|item| { item.kind == MirroredItemKind::Part && item.source_id == "source/J1" })
            .count(),
        1,
        "source-side AddPart must create one durable mirror correspondence"
    );
    let association = halves
        .correspondences
        .iter()
        .find(|item| item.kind == MirroredItemKind::Part && item.source_id == "source/J1")
        .unwrap();
    assert!(
        added
            .boards
            .iter()
            .find(|board| board.id == halves.mirror_board_id)
            .unwrap()
            .part_ids
            .contains(&association.mirror_id)
    );
    for (outline_id, part_id) in [
        ("left/synthetic-envelope", &association.source_id),
        ("mirror/synthetic-envelope", &association.mirror_id),
    ] {
        let OutlineFeature::PartEnvelope { part_ids, .. } = added
            .outline
            .iter()
            .find(|feature| feature.id() == outline_id)
            .unwrap_or_else(|| {
                panic!(
                    "missing {outline_id}; present {:?}",
                    added
                        .outline
                        .iter()
                        .map(OutlineFeature::id)
                        .collect::<Vec<_>>()
                )
            })
        else {
            panic!("expected {outline_id}")
        };
        assert_eq!(
            part_ids.iter().filter(|id| *id == part_id).count(),
            1,
            "{outline_id} membership"
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
    let reply = engine.handle(edit(
        document.revision,
        EditPhase::Commit,
        EditOperation::AddPart {
            part,
            board_id: Some("board".into()),
            definition: Some(definition),
            layout_id: Some("left-layout".into()),
        },
    ));
    assert!(matches!(reply, CoreReply::Error { .. }), "{reply:?}");
    let (_, after_refusal) = scene(engine.handle(CoreRequest::Snapshot {
        id: "snapshot-after-definition-id-mismatch".into(),
    }));
    assert_eq!(after_refusal, document);
}

#[test]
fn mirrored_pair_links_geometry_bidirectionally_and_keeps_hardware_local() {
    let (mut engine, mut doc) = linked_pair();
    let mut right = doc.matrices[1].clone();
    right.cells.push(MatrixCell {
        shift: None,
        bend: None,
        deleted: false,
        assemblies_local: Some(true),
        row: 0,
        column: 1,
        enabled: true,
        variant: Some("right-only".into()),

        definition_id: None,
        offset: None,
        rotation: None,
        assemblies: vec![MatrixAssembly {
            id: "encoder".into(),
            definition_id: "switch".into(),
            offset: Vec2 { x: 3.0, y: -11.0 },
            rotation: Some(22.0),
            side: None,
        }],
    });
    doc = scene(engine.handle(edit(
        doc.revision,
        EditPhase::Commit,
        EditOperation::SetMatrix {
            matrix: right,
            definitions: None,
        },
    )))
    .1;
    for side in [0, 1] {
        let mut matrix = doc.matrices[side].clone();
        matrix.rows = 3;
        matrix.pitch = Vec2 { x: 20.0, y: 18.0 };
        {
            let values = vec![2.0, -3.0, 7.0];
            matrix.column_layout.resize_with(
                values.len().max(matrix.column_layout.len()),
                Default::default,
            );
            for record in &mut matrix.column_layout {
                record.stagger = 0.0;
            }
            for (record, value) in matrix.column_layout.iter_mut().zip(values) {
                record.stagger = value;
            }
        }
        {
            let values = vec![5.0, 12.0, -8.0];
            matrix.column_layout.resize_with(
                values.len().max(matrix.column_layout.len()),
                Default::default,
            );
            for record in &mut matrix.column_layout {
                record.splay = 0.0;
            }
            for (record, value) in matrix.column_layout.iter_mut().zip(values) {
                record.splay = value;
            }
        }
        {
            let values = (vec![
                Some(Vec2 { x: -9.0, y: 17.0 }),
                None,
                Some(Vec2 { x: 25.0, y: -12.0 }),
            ])
            .into_iter()
            .map(|value| value.unwrap_or_default())
            .collect::<Vec<_>>();
            matrix.column_layout.resize_with(
                values.len().max(matrix.column_layout.len()),
                Default::default,
            );
            for record in &mut matrix.column_layout {
                record.splay_origin = Vec2::default();
            }
            for (record, value) in matrix.column_layout.iter_mut().zip(values) {
                record.splay_origin = value;
            }
        }
        matrix.rotation = Some(13.0);
        matrix
            .cells
            .retain(|cell| cell.row != 1 || cell.column != 2);
        matrix.cells.push(MatrixCell {
            shift: None,
            bend: None,
            deleted: false,
            assemblies_local: None,
            row: 1,
            column: 2,
            enabled: false,

            definition_id: None,
            variant: None,
            offset: None,
            rotation: None,
            assemblies: vec![],
        });
        let cell = matrix
            .cells
            .iter_mut()
            .find(|cell| cell.row == 0 && cell.column == 1)
            .unwrap();
        cell.offset = Some(Vec2 { x: 2.0, y: 4.0 });
        cell.rotation = Some(7.0);
        doc = scene(engine.handle(edit(
            doc.revision,
            EditPhase::Commit,
            EditOperation::SetMatrix {
                matrix,
                definitions: None,
            },
        )))
        .1;
        assert_reflected_keys(&doc);
        assert_eq!(doc.matrices[0].rows, 3);
        assert_eq!(doc.matrices[1].rows, 3);
        assert!(
            !doc.parts
                .iter()
                .any(|part| part.id == "matrix/left-matrix/r1c2"
                    || part.id == "matrix/right-matrix/r1c2")
        );
        assert!(
            !doc.parts
                .iter()
                .any(|part| part.id == "matrix/left-matrix/r0c1/encoder")
        );
        let companion = doc.matrices[1]
            .cells
            .iter()
            .find(|cell| cell.row == 0 && cell.column == 1)
            .unwrap();
        assert_eq!(companion.variant.as_deref(), Some("right-only"));
        assert_eq!(companion.assemblies[0].offset.x, 3.0);
    }
}

#[test]
fn matrix_cells_preserve_survivors_and_companions() {
    let mut engine = CoreEngine::new();
    engine.handle(CoreRequest::Open {
        id: "open".into(),
        document: matrix_doc(),
    });
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
    let (_, doc) = scene(engine.handle(edit(
        0,
        EditPhase::Commit,
        EditOperation::SetMatrix {
            matrix: value.clone(),
            definitions: None,
        },
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
    let (_, grown) = scene(engine.handle(edit(
        1,
        EditPhase::Commit,
        EditOperation::SetMatrix {
            matrix: value.clone(),
            definitions: None,
        },
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
    engine.handle(CoreRequest::Open {
        id: "open".into(),
        document: doc,
    });
    let operation = EditOperation::SetMatrix {
        matrix: matrix(1, 1),
        definitions: Some(definitions),
    };
    let projected = preview(engine.handle(edit(0, EditPhase::Preview, operation.clone())));
    assert_eq!(projected.transforms.len(), 1);
    let (_, snapshot) = scene(engine.handle(CoreRequest::Snapshot {
        id: "snapshot".into(),
    }));
    assert!(snapshot.definitions.is_empty());
    assert!(snapshot.parts.is_empty());
    assert!(matches!(
        engine.handle(CoreRequest::Undo { id: "undo".into() }),
        CoreReply::Error { .. }
    ));
    let (_, placed) = scene(engine.handle(edit(0, EditPhase::Commit, operation)));
    assert_eq!(placed.definitions.len(), 1);
    assert_eq!(placed.parts.len(), 1);
    let mut conflicting = placed.definitions[0].clone();
    conflicting.name = "Changed".into();
    assert!(matches!(
        engine.handle(edit(
            1,
            EditPhase::Commit,
            EditOperation::SetMatrix {
                matrix: matrix(1, 1),
                definitions: Some(vec![conflicting]),
            }
        )),
        CoreReply::Error { .. }
    ));
    let (_, undone) = scene(engine.handle(CoreRequest::Undo { id: "undo".into() }));
    assert!(undone.definitions.is_empty());
    assert!(undone.parts.is_empty());
}

#[test]
fn matrix_mirror_rotation_and_bad_dimensions() {
    let mut engine = CoreEngine::new();
    engine.handle(CoreRequest::Open {
        id: "open".into(),
        document: matrix_doc(),
    });
    let mut value = matrix(1, 2);
    value.mirror = Some(Mirror::X);
    value.rotation = Some(90.0);
    let (_, doc) = scene(engine.handle(edit(
        0,
        EditPhase::Commit,
        EditOperation::SetMatrix {
            matrix: value,
            definitions: None,
        },
    )));
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
    assert!(matches!(
        engine.handle(edit(
            1,
            EditPhase::Commit,
            EditOperation::SetMatrix {
                matrix: bad_gap,
                definitions: None,
            }
        )),
        CoreReply::Error { .. }
    ));
    let bad = matrix(0, 3);
    assert!(matches!(
        engine.handle(edit(
            1,
            EditPhase::Commit,
            EditOperation::SetMatrix {
                matrix: bad,
                definitions: None,
            }
        )),
        CoreReply::Error { revision: 1, .. }
    ));
}

#[test]
fn deleting_matrix_removes_its_container_and_parts_in_one_edit() {
    let mut engine = CoreEngine::new();
    engine.handle(CoreRequest::Open {
        id: "open".into(),
        document: matrix_doc(),
    });
    let (_, before) = scene(engine.handle(edit(
        0,
        EditPhase::Commit,
        EditOperation::SetMatrix {
            matrix: matrix(2, 2),
            definitions: None,
        },
    )));
    let operation: EditOperation =
        serde_json::from_value(serde_json::json!({"kind":"remove-matrix", "id":"main"})).unwrap();
    let (_, removed) = scene(engine.handle(edit(before.revision, EditPhase::Commit, operation)));
    assert!(removed.matrices.is_empty());
    assert!(
        !removed
            .parts
            .iter()
            .any(|part| before.matrices[0].part_ids.contains(&part.id))
    );
    let (_, restored) = scene(engine.handle(CoreRequest::Undo { id: "undo".into() }));
    assert_eq!(restored.matrices, before.matrices);
    assert_eq!(restored.parts, before.parts);
    let (_, deleted_again) = scene(engine.handle(CoreRequest::Redo { id: "redo".into() }));
    assert!(deleted_again.matrices.is_empty());
    let (_, restored) = scene(engine.handle(CoreRequest::Undo {
        id: "restore".into(),
    }));
    let (_, empty) = scene(engine.handle(edit(
        restored.revision,
        EditPhase::Commit,
        EditOperation::RemoveParts {
            ids: restored.matrices[0].part_ids.clone(),
        },
    )));
    assert!(empty.matrices[0].part_ids.is_empty());
    let (_, removed_empty) = scene(engine.handle(edit(
        empty.revision,
        EditPhase::Commit,
        EditOperation::RemoveMatrix { id: "main".into() },
    )));
    assert!(removed_empty.matrices.is_empty());
    let (_, restored_empty) = scene(engine.handle(CoreRequest::Undo {
        id: "undo-empty".into(),
    }));
    assert_eq!(restored_empty.matrices, empty.matrices);
}

#[test]
fn mirrored_resize_keeps_each_halves_local_preset_companions() {
    for changed_side in [0, 1] {
        let (mut engine, mut doc) = linked_pair();
        for side in [0, 1] {
            let mut matrix = doc.matrices[side].clone();
            matrix.cells = (0..matrix.rows)
                .flat_map(|row| {
                    (0..matrix.columns).map(move |column| MatrixCell {
                        shift: None,
                        bend: None,
                        deleted: false,
                        row,
                        column,
                        enabled: true,

                        definition_id: Some("switch".into()),
                        variant: Some("preset/mx-rgb/south".into()),
                        offset: None,
                        rotation: None,
                        assemblies_local: Some(true),
                        assemblies: vec![MatrixAssembly {
                            id: format!("local-{side}"),
                            definition_id: "switch".into(),
                            offset: Vec2 { x: 2.0, y: 3.0 },
                            rotation: None,
                            side: None,
                        }],
                    })
                })
                .collect();
            doc = scene(engine.handle(edit(
                doc.revision,
                EditPhase::Commit,
                EditOperation::SetMatrix {
                    matrix,
                    definitions: None,
                },
            )))
            .1;
        }
        let mut matrix = doc.matrices[changed_side].clone();
        matrix.rows = 3;
        for column in 0..matrix.columns {
            let mut cell = matrix.cells[0].clone();
            cell.row = 2;
            cell.column = column;
            matrix.cells.push(cell);
        }
        doc = scene(engine.handle(edit(
            doc.revision,
            EditPhase::Commit,
            EditOperation::SetMatrix {
                matrix,
                definitions: None,
            },
        )))
        .1;
        for side in [0, 1] {
            let cells = &doc.matrices[side].cells;
            assert_eq!(cells.len(), 9);
            for cell in cells {
                assert_eq!(cell.assemblies.len(), 1, "side {side}, row {}", cell.row);
                assert_eq!(cell.assemblies[0].id, format!("local-{side}"));
            }
        }
    }
}

#[test]
fn deleting_matrix_column_compacts_both_halves_and_preserves_local_assemblies() {
    let (mut engine, mut doc) = linked_pair();
    let mut right = doc.matrices[1].clone();
    right.cells = serde_json::from_value(serde_json::json!([
        {"row":0,"column":2,"enabled":true,"variant":"right-only","assembliesLocal":true,
        "assemblies":[{"id":"led","definitionId":"switch","offset":{"x":3,"y":4}}]}
    ]))
    .unwrap();
    doc = scene(engine.handle(edit(
        doc.revision,
        EditPhase::Commit,
        EditOperation::SetMatrix {
            matrix: right,
            definitions: None,
        },
    )))
    .1;
    let reference = doc
        .parts
        .iter()
        .find(|part| part.id == "matrix/right-matrix/r0c2/led")
        .unwrap()
        .reference
        .clone();
    let before = doc.clone();
    let (_, compacted) = scene(engine.handle(edit(
        doc.revision,
        EditPhase::Commit,
        EditOperation::RemoveMatrixColumn {
            matrix_id: "right-matrix".into(),
            column: 1,
        },
    )));
    assert!(compacted.matrices.iter().all(|matrix| matrix.columns == 2));
    let companion = compacted
        .parts
        .iter()
        .find(|part| part.id == "matrix/right-matrix/r0c1/led")
        .unwrap();
    assert_eq!(companion.reference, reference);
    assert!(
        !compacted
            .parts
            .iter()
            .any(|part| part.id == "matrix/left-matrix/r0c1/led")
    );
    assert!(
        compacted.matrices[1]
            .cells
            .iter()
            .any(|cell| cell.column == 1 && cell.variant.as_deref() == Some("right-only"))
    );
    let link = compacted.layouts[1].mirror_link.as_ref().unwrap();
    let mut pairs: Vec<_> = link
        .correspondences
        .iter()
        .filter(|record| record.kind == MirroredItemKind::Part)
        .map(|record| (record.source_id.clone(), record.mirror_id.clone()))
        .collect();
    pairs.sort();
    let mut expected: Vec<_> = (0..compacted.matrices[0].rows)
        .flat_map(|row| {
            (0..compacted.matrices[0].columns).map(move |column| {
                (
                    format!("matrix/left-matrix/r{row}c{column}"),
                    format!("matrix/right-matrix/r{row}c{column}"),
                )
            })
        })
        .collect();
    expected.sort();
    assert_eq!(pairs, expected, "only the surviving pairs may remain");
    let (_, undone) = scene(engine.handle(CoreRequest::Undo { id: "undo".into() }));
    assert_eq!(undone.parts, before.parts);
    assert_eq!(undone.matrices, before.matrices);
    let (_, redone) = scene(engine.handle(CoreRequest::Redo { id: "redo".into() }));
    assert_eq!(redone.parts, compacted.parts);
    assert_eq!(redone.matrices, compacted.matrices);
}

#[test]
fn deleting_last_matrix_row_removes_matrix_and_key_metadata_and_is_undoable() {
    let mut engine = CoreEngine::new();
    engine.handle(CoreRequest::Open {
        id: "open".into(),
        document: matrix_doc(),
    });
    let (_, mut doc) = scene(engine.handle(edit(
        0,
        EditPhase::Commit,
        EditOperation::SetMatrix {
            matrix: matrix(1, 1),
            definitions: None,
        },
    )));
    doc.keymap = Some(KeymapConfiguration::default());
    doc.keymap.as_mut().unwrap().layers[0]
        .bindings
        .insert("matrix/main/r0c0".into(), KeyBinding::None);
    engine.handle(CoreRequest::Open {
        id: "reopen".into(),
        document: doc.clone(),
    });
    let (_, removed) = scene(engine.handle(edit(
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
    let (_, restored) = scene(engine.handle(CoreRequest::Undo { id: "undo".into() }));
    assert_eq!(restored.matrices, doc.matrices);
    assert_eq!(restored.keymap, doc.keymap);
}
