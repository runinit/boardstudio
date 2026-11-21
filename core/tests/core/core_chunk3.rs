use super::*;

#[test]
fn pcb_readiness_checks_pads_outlines_and_thickness() {
    let mut engine = CoreEngine::new();
    let mut doc = ProjectDoc::empty("p", "Project");
    doc.outline
        .push(rect("edge", 0.0, 0.0, 20.0, 20.0, Operation::Add));
    doc.definitions.push(PartDefinition {
        hardware_profile: None,
        input_profile: None,
        mechanical_profile: None,
        id: "switch".into(),
        name: "Switch".into(),
        kind: PartKind::Switch,
        courtyard: vec![],
        pads: vec![Pad {
            id: "pad1".into(),
            number: "1".into(),
            at: Vec2 { x: 0.0, y: 0.0 },
            size: Vec2 { x: 1.0, y: 1.0 },
            shape: PadShape::Circle,
            drill: None,
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
    doc.nets.push(Net {
        id: "row".into(),
        name: "ROW".into(),
        pins: vec![Pin {
            part_id: "k1".into(),
            pad_id: "missing".into(),
        }],
    });
    doc.boards.push(Board {
        id: "board".into(),
        name: "Board".into(),
        outline_ids: vec!["wrong".into()],
        part_ids: vec!["k1".into()],
        net_ids: vec!["row".into()],
        thickness: 0.0,
        traces: vec![],
        vias: vec![],
    });
    let (bad, _) = scene(engine.handle(CoreRequest::Open {
        id: "bad".into(),
        document: doc.clone(),
    }));
    assert!(!bad.readiness.pcb);
    assert!(bad.findings.iter().any(|f| f.id == "board:board:thickness"));
    assert!(
        bad.findings
            .iter()
            .any(|f| f.id == "board:board:outline:wrong")
    );
    assert!(
        bad.findings
            .iter()
            .any(|f| f.id == "net:row:pad:k1:missing")
    );
    doc.boards[0].outline_ids = vec!["edge".into()];
    doc.boards[0].thickness = 1.6;
    doc.nets[0].pins[0].pad_id = "pad1".into();
    let (good, _) = scene(engine.handle(CoreRequest::Open {
        id: "good".into(),
        document: doc,
    }));
    assert!(good.readiness.pcb);
}

#[test]
fn empty_board_subset_blocks_pcb() {
    let mut engine = CoreEngine::new();
    let mut doc = ProjectDoc::empty("p", "Project");
    doc.outline
        .push(rect("outer", 0.0, 0.0, 20.0, 20.0, Operation::Add));
    doc.outline
        .push(rect("cut", 0.0, 0.0, 2.0, 2.0, Operation::Subtract));
    doc.boards.push(Board {
        id: "board".into(),
        name: "Board".into(),
        outline_ids: vec!["cut".into()],
        part_ids: vec![],
        net_ids: vec![],
        thickness: 1.6,
        traces: vec![],
        vias: vec![],
    });
    let (scene, _) = scene(engine.handle(CoreRequest::Open {
        id: "open".into(),
        document: doc,
    }));
    assert!(!scene.contours.is_empty());
    assert!(scene.board_contours[0].contours.is_empty());
    assert!(!scene.readiness.pcb);
    assert!(
        scene
            .findings
            .iter()
            .any(|finding| finding.id == "board:board:empty-outline")
    );
}

#[test]
fn authored_case_geometry_ignores_missing_material_metadata() {
    let mut doc = ProjectDoc::empty("authored-case", "Authored Case");
    doc.outline
        .push(rect("edge", 0.0, 0.0, 30.0, 20.0, Operation::Add));
    doc.boards.push(Board {
        id: "left".into(),
        name: "Left PCB".into(),
        outline_ids: vec!["edge".into()],
        part_ids: vec![],
        net_ids: vec![],
        thickness: 1.6,
        traces: vec![],
        vias: vec![],
    });
    doc.case_bodies.push(CaseBody {
        features: None,
        openings: None,
        id: "plate".into(),
        name: "Left plate".into(),
        board_id: "left".into(),
        kind: CaseKind::Plate,
        thickness: 3.0,
        clearance: 0.5,
        material_id: Some("pla".into()),
        z: Some(0.0),
        wall_height: Some(14.0),
        wall_thickness: Some(2.0),
        mounts: Some(vec![]),
        pcb_source: None,
        gasket: None,
    });
    let mut engine = CoreEngine::new();
    let (scene, document) = scene(engine.handle(CoreRequest::Open {
        id: "open".into(),
        document: doc,
    }));
    assert!(!scene.board_readiness[0].case_ready);
    assert!(
        scene
            .findings
            .iter()
            .any(|finding| finding.id == "case:plate:material")
    );
    assert!(boardstudio_core::authored_case_geometry_ready(
        &document, &scene, "left"
    ));

    let reply = engine.handle(CoreRequest::PrepareCase {
        id: "prepare".into(),
        ir: CaseAssemblyIR {
            revision: document.revision,
            bodies: vec![CaseIR {
                revision: document.revision,
                body: document.case_bodies[0].clone(),
                contours: scene.board_contours[0].contours.clone(),
            }],
        },
    });
    assert!(matches!(reply, CoreReply::CasePrepared { .. }));
}

#[test]
fn mirrored_pair_is_atomic_and_survives_history_and_reopen() {
    let mut engine = CoreEngine::new();
    engine.handle(CoreRequest::Open {
        id: "open".into(),
        document: matrix_doc(),
    });
    let response = engine.request(&mirrored_pair_request(0, "commit"));
    let (_, document) = scene(serde_json::from_str(&response).unwrap());
    assert_eq!(document.matrices.len(), 2);
    assert_eq!(document.parts.len(), 12);
    assert_eq!(document.boards[0].part_ids.len(), 12);
    let serialized = serde_json::to_value(&document).unwrap();
    assert_eq!(
        serialized["layouts"][1]["mirrorLink"]["sourceId"],
        "left-layout"
    );
    let (_, undone) = scene(engine.handle(CoreRequest::Undo { id: "undo".into() }));
    assert!(undone.matrices.is_empty());
    assert!(undone.parts.is_empty());
    let (_, redone) = scene(engine.handle(CoreRequest::Redo { id: "redo".into() }));
    assert_eq!(redone.parts, document.parts);
    let (_, reopened) = scene(engine.handle(CoreRequest::Open {
        id: "reopen".into(),
        document: serde_json::from_value(serialized).unwrap(),
    }));
    assert_eq!(reopened.parts, document.parts);
}

#[test]
fn matrixless_linked_add_part_projects_from_either_half_and_preserves_omission() {
    for (board_id, part_id, reference) in [
        ("board-main", "board-main/source-add", "SRC1"),
        ("board-mirror", "board-mirror/reverse-add", "REV1"),
    ] {
        let (mut engine, before) = matrixless_native_pair_with_omitted_part();
        assert!(before.matrices.is_empty());
        assert!(before.layouts.is_empty());
        let mut definition = before.definitions[0].clone();
        definition.id = "catalog:matrixless-addition".into();
        definition.name = "Matrixless addition".into();
        let part = Part {
            anchor: None,
            id: part_id.into(),
            definition_id: definition.id.clone(),
            reference: reference.into(),
            pose: Pose2 {
                at: Vec2 { x: 5.0, y: -2.0 },
                rotation: 0.0,
            },
            side: Side::Front,
            keycap: None,
            outline: None,
            locked: None,
            properties: None,
            generator_parameters: None,
        };
        let (_, added) = scene(engine.handle(edit(
            before.revision,
            EditPhase::Commit,
            EditOperation::AddPart {
                part: part.clone(),
                board_id: Some(board_id.into()),
                definition: Some(definition.clone()),
                layout_id: None,
            },
        )));
        let halves = added.hardware.as_ref().unwrap().pairs[0]
            .halves
            .as_ref()
            .unwrap();
        let new_associations: Vec<_> = halves
            .correspondences
            .iter()
            .filter(|association| {
                association.kind == MirroredItemKind::Part
                    && association.source_id != "part-previous"
            })
            .collect();
        assert_eq!(new_associations.len(), 1, "one new placement links once");
        let association = new_associations[0];
        let source_part = added
            .parts
            .iter()
            .find(|candidate| candidate.id == association.source_id)
            .unwrap();
        let mirror_part = added
            .parts
            .iter()
            .find(|candidate| candidate.id == association.mirror_id)
            .unwrap();
        let (authored, counterpart) = if board_id == "board-main" {
            (source_part, mirror_part)
        } else {
            (mirror_part, source_part)
        };
        assert_eq!(
            authored.pose, part.pose,
            "the requested side keeps its pose"
        );
        let expected_counterpart_at = if board_id == "board-main" {
            Vec2 { x: 202.0, y: -5.0 }
        } else {
            Vec2 { x: 2.0, y: 195.0 }
        };
        assert!(
            (counterpart.pose.at.x - expected_counterpart_at.x).abs() < 1e-9
                && (counterpart.pose.at.y - expected_counterpart_at.y).abs() < 1e-9,
            "rotated map gives counterpart position {:?}, expected {:?}",
            counterpart.pose.at,
            expected_counterpart_at
        );
        assert_eq!(
            counterpart.pose.rotation, 270.0,
            "the rotated reflection maps heading exactly"
        );
        assert!(
            added.boards.iter().any(|board| {
                board.id == halves.source_board_id
                    && board.part_ids.contains(&association.source_id)
            }),
            "source Part has a board owner"
        );
        assert!(
            added.boards.iter().any(|board| {
                board.id == halves.mirror_board_id
                    && board.part_ids.contains(&association.mirror_id)
            }),
            "mirror Part has a board owner"
        );
        assert!(
            halves.exceptions.iter().any(|exception| {
                exception.kind == MirroredItemKind::Part
                    && exception.mirror_id == "part-previous-mirror"
                    && exception.state == MirroredExceptionState::Omitted
            }),
            "the pre-existing Omitted association remains unchanged"
        );
        assert_eq!(
            added
                .definitions
                .iter()
                .filter(|item| item.id == definition.id)
                .count(),
            1
        );
        assert_eq!(added.matrices, before.matrices);
        assert_eq!(added.layouts, before.layouts);
    }
}

#[test]
fn mirror_only_part_refuses_a_source_board_target() {
    let (mut engine, document) = public_native_pair();
    let halves = document.hardware.as_ref().unwrap().pairs[0]
        .halves
        .as_ref()
        .unwrap()
        .clone();
    let mut prior = document.parts[0].clone();
    prior.id = "mirror/prior-redo".into();
    prior.reference = "PRIOR1".into();
    scene(engine.handle(edit(
        document.revision,
        EditPhase::Commit,
        EditOperation::AddMirrorOnlyPart {
            part: prior.clone(),
            board_id: halves.mirror_board_id.clone(),
            definition: None,
            layout_id: None,
        },
    )));
    scene(engine.handle(CoreRequest::Undo {
        id: "undo-before-refusal".into(),
    }));
    let (_, before_refusal) = scene(engine.handle(CoreRequest::Snapshot {
        id: "before-refusal".into(),
    }));

    let mut part = document.parts[0].clone();
    part.id = "wrong-half/only".into();
    part.reference = "WRONG1".into();
    let error = match engine.handle(edit(
        before_refusal.revision,
        EditPhase::Commit,
        EditOperation::AddMirrorOnlyPart {
            part,
            board_id: halves.source_board_id.clone(),
            definition: None,
            layout_id: None,
        },
    )) {
        CoreReply::Error { message, .. } => message,
        other => panic!("expected a mirror target refusal, got {other:?}"),
    };
    assert!(
        error.contains("Mirror-only placement requires one exact native mirror board"),
        "{error}"
    );
    let (_, after_refusal) = scene(engine.handle(CoreRequest::Snapshot {
        id: "after-refusal".into(),
    }));
    assert_eq!(
        after_refusal, before_refusal,
        "refusal preserves the full document"
    );
    let (_, redone) = scene(engine.handle(CoreRequest::Redo {
        id: "redo-after-refusal".into(),
    }));
    assert!(
        redone
            .parts
            .iter()
            .any(|candidate| candidate.id == prior.id),
        "refusal preserves the existing Redo entry"
    );
}

#[test]
fn add_part_accepts_legacy_payload_without_optional_placement_fields() {
    let operation: EditOperation = serde_json::from_value(serde_json::json!({
        "kind": "add-part",
        "part": {
            "id": "board/legacy",
            "definitionId": "switch",
            "reference": "SW88",
            "pose": {"at": {"x": 10.0, "y": 20.0}, "rotation": 0.0},
            "side": "front"
        },
        "boardId": "board"
    }))
    .expect("legacy AddPart JSON remains readable");
    let EditOperation::AddPart {
        definition,
        layout_id,
        ..
    } = &operation
    else {
        panic!("expected AddPart")
    };
    assert!(definition.is_none());
    assert!(layout_id.is_none());
    let encoded = serde_json::to_value(operation).expect("AddPart JSON serializes");
    assert!(encoded.get("definition").is_none());
    assert!(encoded.get("layoutId").is_none());
}

#[test]
fn mirrored_pair_previews_restore_both_halves_and_nudges_stay_parametric() {
    let (mut engine, doc) = linked_pair();
    let mut next = doc.matrices[1].clone();
    next.columns = 4;
    preview(engine.handle(edit(
        doc.revision,
        EditPhase::Preview,
        EditOperation::SetMatrix {
            matrix: next,
            definitions: None,
        },
    )));
    let snapshot = scene(engine.handle(CoreRequest::Snapshot {
        id: "snapshot".into(),
    }))
    .1;
    assert_eq!(snapshot, doc);
    let id = "matrix/right-matrix/r0c0";
    let original = doc.parts.iter().find(|part| part.id == id).unwrap();
    let operation = EditOperation::MoveParts {
        positions: vec![Position {
            id: id.into(),
            at: Vec2 {
                x: original.pose.at.x + 4.0,
                y: original.pose.at.y + 3.0,
            },
        }],
    };
    preview(engine.handle(edit(doc.revision, EditPhase::Preview, operation.clone())));
    assert_eq!(
        scene(engine.handle(CoreRequest::Snapshot {
            id: "snapshot".into()
        }))
        .1,
        doc
    );
    let doc = scene(engine.handle(edit(doc.revision, EditPhase::Commit, operation))).1;
    assert_reflected_keys(&doc);
    assert!(doc.matrices[1].cells[0].offset.is_some());
    let mut grow = doc.matrices[0].clone();
    grow.columns += 1;
    let grown = scene(engine.handle(edit(
        doc.revision,
        EditPhase::Commit,
        EditOperation::SetMatrix {
            matrix: grow,
            definitions: None,
        },
    )))
    .1;
    assert_eq!(
        grown.parts.iter().find(|part| part.id == id).unwrap().pose,
        doc.parts.iter().find(|part| part.id == id).unwrap().pose
    );
    assert_reflected_keys(&grown);
}

#[test]
fn mirrored_pair_links_extra_components_and_unlink_preserves_poses() {
    let (mut engine, doc) = linked_pair();
    let mut local = doc.parts[0].clone();
    local.id = "left-encoder".into();
    local.reference = "ENC1".into();
    let mut doc = scene(engine.handle(edit(
        doc.revision,
        EditPhase::Commit,
        EditOperation::AddPart {
            part: local.clone(),
            board_id: Some("board".into()),
            definition: None,
            layout_id: None,
        },
    )))
    .1;
    let mut left = doc.layouts[0].clone();
    if !left.part_ids.contains(&local.id) {
        left.part_ids.push(local.id.clone());
    }
    let (joined_scene, joined) = scene(engine.handle(edit(
        doc.revision,
        EditPhase::Commit,
        EditOperation::SetLayout { layout: left },
    )));
    doc = joined;
    // Ticket 12: joining the half converts the drop to half-frame storage;
    // the evaluated board pose is unchanged.
    assert_eq!(board_pose(&joined_scene, &local.id), local.pose);
    let mut right = doc.matrices[1].clone();
    {
        let values = vec![7.0];
        let count = values.len().max(right.column_layout.len());
        right.column_layout.resize_with(count, Default::default);
        for record in &mut right.column_layout {
            record.stagger = 0.0;
        }
        for (record, value) in right.column_layout.iter_mut().zip(values) {
            record.stagger = value;
        }
    }
    let (mirrored_scene, mirrored) = scene(engine.handle(edit(
        doc.revision,
        EditPhase::Commit,
        EditOperation::SetMatrix {
            matrix: right,
            definitions: None,
        },
    )));
    doc = mirrored;
    assert_eq!(doc.layouts[1].part_ids.len(), 1);
    // Ticket 12: the linked partner mirrors in the half frame (stored halves
    // negate about the origin) and evaluates to the board reflection.
    let stored = doc
        .parts
        .iter()
        .find(|part| part.id == local.id)
        .unwrap()
        .pose;
    let counterpart = doc
        .parts
        .iter()
        .find(|part| part.id == doc.layouts[1].part_ids[0])
        .unwrap();
    assert!((counterpart.pose.at.x + stored.at.x).abs() < 1e-8);
    assert!((counterpart.pose.at.y - stored.at.y).abs() < 1e-8);
    let placed = board_pose(&mirrored_scene, &local.id);
    let mirrored_board = board_pose(&mirrored_scene, &counterpart.id);
    assert!((mirrored_board.at.x + placed.at.x - 20.0).abs() < 1e-8);
    assert!((mirrored_board.at.y - placed.at.y).abs() < 1e-8);
    let mut independent = doc.layouts[1].clone();
    independent.mirror_link = None;
    let (unlinked_scene, unlinked) = scene(engine.handle(edit(
        doc.revision,
        EditPhase::Commit,
        EditOperation::SetLayout {
            layout: independent,
        },
    )));
    // Unlink preserves every evaluated pose; storage returns board-space.
    assert_eq!(
        unlinked_scene
            .transforms
            .iter()
            .map(|transform| (&transform.id, transform.pose))
            .collect::<Vec<_>>(),
        mirrored_scene
            .transforms
            .iter()
            .map(|transform| (&transform.id, transform.pose))
            .collect::<Vec<_>>()
    );
    let mut change = unlinked.matrices[0].clone();
    change.rows += 1;
    let changed = scene(engine.handle(edit(
        unlinked.revision,
        EditPhase::Commit,
        EditOperation::SetMatrix {
            matrix: change,
            definitions: None,
        },
    )))
    .1;
    assert_eq!(changed.matrices[1], unlinked.matrices[1]);
}

#[test]
fn new_matrix_rejects_out_of_bounds_edits() {
    let mut engine = CoreEngine::new();
    engine.handle(CoreRequest::Open {
        id: "open".into(),
        document: matrix_doc(),
    });
    let mut value = matrix(1, 1);
    {
        let values = vec![Vec2::default(); 2];
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
    assert!(
        matches!(engine.handle(edit(0, EditPhase::Commit, EditOperation::SetMatrix { matrix: value.clone(), definitions: None })), CoreReply::Error { message, .. } if message == "layout list exceeds axis count")
    );
    value.row_layout.clear();
    value.cells = vec![MatrixCell {
        shift: None,
        bend: None,
        deleted: false,
        assemblies_local: None,
        row: 1,
        column: 0,
        enabled: true,

        definition_id: None,
        variant: None,
        offset: None,
        rotation: None,
        assemblies: vec![],
    }];
    assert!(
        matches!(engine.handle(edit(0, EditPhase::Commit, EditOperation::SetMatrix { matrix: value, definitions: None })), CoreReply::Error { message, .. } if message == "invalid or duplicate cell coordinate")
    );
}

#[test]
fn matrix_targets_board_and_preview_is_scoped() {
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
    let mut target = matrix(1, 2);
    target.board_id = Some("right".into());
    let preview = preview(engine.handle(edit(
        0,
        EditPhase::Preview,
        EditOperation::SetMatrix {
            matrix: target.clone(),
            definitions: None,
        },
    )));
    assert_eq!(preview.board_contours[1].contours.len(), 1);
    assert!(preview.board_contours[0].contours.is_empty());
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
    let (_, placed) = scene(engine.handle(edit(
        0,
        EditPhase::Commit,
        EditOperation::SetMatrix {
            matrix: target.clone(),
            definitions: None,
        },
    )));
    assert_eq!(placed.boards[1].part_ids.len(), 2);
    assert!(placed.boards[0].part_ids.is_empty());
    let OutlineFeature::PartEnvelope { part_ids, .. } = &placed.outline[1] else {
        panic!("expected envelope")
    };
    assert_eq!(part_ids.len(), 2);
    let mut edit_matrix = target;
    edit_matrix.board_id = None;
    let (_, updated) = scene(engine.handle(edit(
        1,
        EditPhase::Commit,
        EditOperation::SetMatrix {
            matrix: edit_matrix,
            definitions: None,
        },
    )));
    assert_eq!(updated.matrices[0].board_id.as_deref(), Some("right"));
    assert_eq!(updated.boards[1].part_ids.len(), 2);
    let mut invalid = matrix(1, 1);
    invalid.board_id = Some("missing".into());
    assert!(matches!(
        engine.handle(edit(
            2,
            EditPhase::Commit,
            EditOperation::SetMatrix {
                matrix: invalid,
                definitions: None,
            }
        )),
        CoreReply::Error { .. }
    ));
}

#[test]
fn matrix_generates_stable_parts_and_one_undo_step() {
    let mut engine = CoreEngine::new();
    engine.handle(CoreRequest::Open {
        id: "open".into(),
        document: matrix_doc(),
    });
    let preview = preview(engine.handle(edit(
        0,
        EditPhase::Preview,
        EditOperation::SetMatrix {
            matrix: matrix(2, 2),
            definitions: None,
        },
    )));
    assert_eq!(preview.transforms.len(), 4);
    assert!(
        scene(engine.handle(CoreRequest::Snapshot {
            id: "snapshot".into()
        }))
        .1
        .parts
        .is_empty()
    );
    let (committed_scene, doc) = scene(engine.handle(edit(
        0,
        EditPhase::Commit,
        EditOperation::SetMatrix {
            matrix: matrix(2, 2),
            definitions: None,
        },
    )));
    assert_eq!(committed_scene.revision, 1);
    assert_eq!(
        doc.matrices[0].part_ids,
        vec![
            "matrix/main/r0c0",
            "matrix/main/r0c1",
            "matrix/main/r1c0",
            "matrix/main/r1c1"
        ]
    );
    assert_eq!(doc.boards[0].part_ids.len(), 4);
    let OutlineFeature::PartEnvelope { part_ids, .. } = &doc.outline[0] else {
        panic!("expected envelope")
    };
    assert_eq!(part_ids.len(), 4);
    assert!(committed_scene.readiness.outline);
    let (undone, doc) = scene(engine.handle(CoreRequest::Undo { id: "undo".into() }));
    assert_eq!(undone.revision, 2);
    assert!(doc.parts.is_empty());
}

#[test]
fn replacing_matrix_switch_remaps_existing_terminal_nets_and_undoes() {
    let mut doc = matrix_doc();
    doc.definitions[0].pads = ["one", "two"].iter().enumerate().map(|(i, id)| serde_json::from_value(serde_json::json!({
        "id": id, "number": (i + 1).to_string(), "at": {"x": i, "y": 0}, "size": {"x": 1, "y": 1}, "shape": "circle"
    })).unwrap()).collect();
    let mut engine = CoreEngine::new();
    engine.handle(CoreRequest::Open {
        id: "open".into(),
        document: doc,
    });
    let (_, mut placed) = scene(engine.handle(edit(
        0,
        EditPhase::Commit,
        EditOperation::SetMatrix {
            matrix: matrix(1, 1),
            definitions: None,
        },
    )));
    let id = placed.parts[0].id.clone();
    placed.nets.push(Net {
        id: "external".into(),
        name: "ROW0".into(),
        pins: vec![Pin {
            part_id: id.clone(),
            pad_id: "one".into(),
        }],
    });
    placed.boards[0].net_ids.push("external".into());
    placed.nets.push(Net {
        id: "external-column".into(),
        name: "COL0".into(),
        pins: vec![Pin {
            part_id: id.clone(),
            pad_id: "two".into(),
        }],
    });
    placed.boards[0].net_ids.push("external-column".into());
    engine.handle(CoreRequest::Open {
        id: "reopen".into(),
        document: placed.clone(),
    });
    let mut replacement = placed.definitions[0].clone();
    replacement.id = "new-switch".into();
    replacement.pads[0].id = "new-row".into();
    replacement.pads[1].id = "new-column".into();
    replacement
        .terminals
        .insert("from".into(), vec!["new-row".into()]);
    replacement
        .terminals
        .insert("to".into(), vec!["new-column".into()]);
    replacement.matrix_terminals = Some(MatrixTerminals {
        row: "from".into(),
        column: "to".into(),
    });
    let mut incoming = placed.matrices[0].clone();
    incoming.definition_id = replacement.id.clone();
    let mut conflicting = replacement.clone();
    conflicting.matrix_terminals.as_mut().unwrap().column = "from".into();
    assert!(matches!(
        engine.handle(edit(
            placed.revision,
            EditPhase::Commit,
            EditOperation::SetMatrix {
                matrix: incoming.clone(),
                definitions: Some(vec![conflicting])
            }
        )),
        CoreReply::Error { .. }
    ));
    let (_, changed) = scene(engine.handle(edit(
        placed.revision,
        EditPhase::Commit,
        EditOperation::SetMatrix {
            matrix: incoming,
            definitions: Some(vec![replacement]),
        },
    )));
    assert_eq!(
        changed
            .nets
            .iter()
            .find(|n| n.id == "external")
            .unwrap()
            .pins,
        vec![Pin {
            part_id: id,
            pad_id: "new-row".into()
        }]
    );
    let (_, undone) = scene(engine.handle(CoreRequest::Undo { id: "undo".into() }));
    assert_eq!(undone.nets, placed.nets);
    assert_eq!(undone.parts, placed.parts);
}

#[test]
fn linked_matrix_resize_retires_out_of_bounds_correspondences_and_reopens() {
    let (mut engine, mut doc) = linked_pair();
    for columns in [2, 4] {
        let mut matrix = doc.matrices[0].clone();
        matrix.columns = columns;
        doc = scene(engine.handle(edit(
            doc.revision,
            EditPhase::Commit,
            EditOperation::SetMatrix {
                matrix,
                definitions: None,
            },
        )))
        .1;
        let link = doc.layouts[1].mirror_link.as_ref().unwrap();
        for row in 0..doc.matrices[0].rows {
            for column in 0..columns {
                assert!(
                    link.correspondences.iter().any(|record| {
                        record.source_id == format!("matrix/left-matrix/r{row}c{column}")
                            && record.mirror_id == format!("matrix/right-matrix/r{row}c{column}")
                    }),
                    "missing r{row}c{column} after resizing to {columns} columns"
                );
            }
        }
        if columns == 2 {
            assert!(
                link.correspondences.iter().all(|record| {
                    !record.source_id.ends_with("r0c2") && !record.mirror_id.ends_with("r0c2")
                }),
                "the retired r0c2 pair must stay gone: {link:?}"
            );
        }
        let (_, reopened) = scene(CoreEngine::new().handle(CoreRequest::Open {
            id: "reopen-resized".into(),
            document: doc.clone(),
        }));
        assert_eq!(reopened.layouts, doc.layouts);
    }
}

#[test]
fn grouped_matrix_cell_actions_are_atomic_across_unlinked_matrices() {
    let mut engine = CoreEngine::new();
    engine.handle(CoreRequest::Open {
        id: "open".into(),
        document: matrix_doc(),
    });
    let (_, doc) = scene(engine.handle(edit(
        0,
        EditPhase::Commit,
        EditOperation::SetMatrix {
            matrix: matrix(1, 2),
            definitions: None,
        },
    )));
    let mut other = matrix(1, 2);
    other.id = "other".into();
    other.origin.x = 60.0;
    let (_, before) = scene(engine.handle(edit(
        doc.revision,
        EditPhase::Commit,
        EditOperation::SetMatrix {
            matrix: other,
            definitions: None,
        },
    )));
    let operation = serde_json::from_value(serde_json::json!({"kind":"set-matrix-cells-enabled", "cells":{"main":[[0,0]],"other":[[0,1]]}, "enabled":false})).expect("Core must support grouped matrix enable changes atomically");
    let (geometry, disabled) =
        scene(engine.handle(edit(before.revision, EditPhase::Commit, operation)));
    assert!(
        geometry
            .matrix_scenes
            .iter()
            .all(|matrix| matrix.cells.len() == 2)
    );
    assert!(
        disabled
            .matrices
            .iter()
            .all(|matrix| matrix.part_ids.len() == 1)
    );
    let operation = serde_json::from_value(serde_json::json!({"kind":"set-matrix-cells-enabled", "cells":{"main":[[0,0]],"other":[[0,1]]}, "enabled":true})).unwrap();
    let (_, enabled) = scene(engine.handle(edit(disabled.revision, EditPhase::Commit, operation)));
    assert!(
        enabled
            .matrices
            .iter()
            .all(|matrix| matrix.part_ids.len() == 2)
    );
    let operation = serde_json::from_value(serde_json::json!({"kind":"remove-selected-matrix-cells", "cells":{"main":[[0,0]],"other":[[0,1]]}})).expect("Core must delete a grouped selection atomically");
    let (geometry, deleted) =
        scene(engine.handle(edit(enabled.revision, EditPhase::Commit, operation)));
    assert!(
        geometry
            .matrix_scenes
            .iter()
            .all(|matrix| matrix.cells.len() == 1)
    );
    let (_, undone) = scene(engine.handle(CoreRequest::Undo { id: "undo".into() }));
    assert_eq!(undone.matrices, enabled.matrices);
    assert_eq!(undone.parts, enabled.parts);
    let (_, redone) = scene(engine.handle(CoreRequest::Redo { id: "redo".into() }));
    assert_eq!(redone.matrices, deleted.matrices);
    let document = serde_json::from_value(serde_json::to_value(&deleted).unwrap()).unwrap();
    let (geometry, _) = scene(engine.handle(CoreRequest::Open {
        id: "reopen".into(),
        document,
    }));
    assert!(
        geometry
            .matrix_scenes
            .iter()
            .all(|matrix| matrix.cells.len() == 1)
    );
}

#[test]
fn undo_retains_only_the_latest_hundred_commits() {
    let mut engine = CoreEngine::new();
    let mut seed = ProjectDoc::empty("bounded-history", "0");
    seed.parts.push(Part {
        anchor: None,
        id: "part-0".into(),
        definition_id: "catalog:unused".into(),
        reference: "P0".into(),
        pose: Pose2 {
            at: Vec2 { x: 0.0, y: 0.0 },
            rotation: 0.0,
        },
        side: Side::Front,
        keycap: None,
        outline: None,
        locked: None,
        properties: None,
        generator_parameters: None,
    });
    let (_, mut document) = scene(engine.handle(CoreRequest::Open {
        id: "open".into(),
        document: seed,
    }));
    for index in 1..=101 {
        (_, document) = scene(engine.handle(edit(
            document.revision,
            EditPhase::Commit,
            EditOperation::MoveParts {
                positions: vec![Position {
                    id: "part-0".into(),
                    at: Vec2 {
                        x: index as f64,
                        y: 0.0,
                    },
                }],
            },
        )));
    }
    for _ in 0..100 {
        (_, document) = scene(engine.handle(CoreRequest::Undo { id: "undo".into() }));
    }
    assert_eq!(document.parts[0].pose.at.x, 1.0);
    assert!(matches!(
        engine.handle(CoreRequest::Undo {
            id: "too-old".into()
        }),
        CoreReply::Error { .. }
    ));
    for _ in 0..100 {
        (_, document) = scene(engine.handle(CoreRequest::Redo { id: "redo".into() }));
    }
    assert_eq!(document.parts[0].pose.at.x, 101.0);
    (_, document) = scene(engine.handle(CoreRequest::Undo {
        id: "branch".into(),
    }));
    scene(engine.handle(edit(
        document.revision,
        EditPhase::Commit,
        EditOperation::MoveParts {
            positions: vec![Position {
                id: "part-0".into(),
                at: Vec2 { x: 0.5, y: 0.0 },
            }],
        },
    )));
    assert!(matches!(
        engine.handle(CoreRequest::Redo {
            id: "no-redo".into()
        }),
        CoreReply::Error { .. }
    ));
    scene(engine.handle(CoreRequest::Open {
        id: "fresh".into(),
        document: ProjectDoc::empty("fresh", "Fresh"),
    }));
    assert!(matches!(
        engine.handle(CoreRequest::Undo { id: "empty".into() }),
        CoreReply::Error { .. }
    ));
}

#[test]
fn adding_a_link_never_recreates_a_pair_excused_by_a_local_exception() {
    let (mut engine, mut doc) = linked_pair();
    let mut link = doc.layouts[1].mirror_link.take().unwrap();
    let (_, opened) = scene(engine.handle(CoreRequest::Open {
        id: "open-unlinked".into(),
        document: doc.clone(),
    }));
    link.correspondences.clear();
    link.exceptions.push(MirroredException {
        companion_mode: None,
        companion_tweak: None,
        kind: MirroredItemKind::Part,
        mirror_id: "matrix/right-matrix/r0c1".into(),
        state: MirroredExceptionState::Local,
        removed_copper: None,
    });
    let mut layout = opened.layouts[1].clone();
    layout.mirror_link = Some(link);
    let (_, linked) = scene(engine.handle(edit(
        opened.revision,
        EditPhase::Commit,
        EditOperation::SetLayout { layout },
    )));
    let link = linked.layouts[1].mirror_link.as_ref().unwrap();
    assert!(
        !link.correspondences.iter().any(|record| {
            record.kind == MirroredItemKind::Part
                && (record.source_id.ends_with("left-matrix/r0c1")
                    || record.mirror_id.ends_with("right-matrix/r0c1"))
        }),
        "adding a link must not recreate an excused pair: {link:?}"
    );
}
