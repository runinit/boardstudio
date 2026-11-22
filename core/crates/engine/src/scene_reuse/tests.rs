use super::*;

fn fixture() -> ProjectDoc {
    let mut doc = synthetic_outline();
    // Stagger accumulates across columns. Edit the last column so one
    // cutout moves while the cutout attached to the first key is reusable.
    for (column, id) in ["stationary-cutout", "moved-cutout"]
        .into_iter()
        .enumerate()
    {
        doc.outline.push(OutlineFeature::Rect {
            id: id.into(),
            anchor_part_id: Some(format!("matrix/matrix/r0c{column}")),
            center: Vec2::default(),
            size: Vec2 { x: 2.0, y: 2.0 },
            radius: 0.0,
            rotation: None,
            operation: Operation::Subtract,
        });
        doc.boards[0].outline_ids.push(id.into());
    }
    doc
}

fn synthetic_outline() -> ProjectDoc {
    serde_json::from_str(include_str!(
        "../../../../tests/fixtures/synthetic-outline.json"
    ))
    .unwrap()
}

// Match the release Chrome profile's authored 5 × 12 fixture.
fn stagger_profile_document(large: bool) -> ProjectDoc {
    stagger_document(large.then_some(12))
}

fn stagger_document(columns: Option<u32>) -> ProjectDoc {
    let mut document = synthetic_outline();
    if let Some(columns) = columns {
        let mut matrix = document.matrices[0].clone();
        matrix.rows = 5;
        matrix.columns = columns;
        matrix.row_layout = vec![
            RowLayout::default(),
            RowLayout {
                shift: Vec2 { x: 4.75, y: 0.0 },
                ..Default::default()
            },
        ];
        let mut engine = CoreEngine::new();
        assert!(matches!(
            engine.handle(CoreRequest::Open {
                id: "fixture-open".into(),
                document
            }),
            CoreReply::Scene { .. }
        ));
        let CoreReply::Scene {
            document: resized, ..
        } = engine.handle(edit(
            0,
            EditOperation::SetMatrix {
                matrix,
                definitions: None,
            },
        ))
        else {
            panic!("could not resize profile fixture");
        };
        document = *resized;
        assert_eq!(document.matrices[0].part_ids.len(), 5 * columns as usize);
        let OutlineFeature::PartEnvelope { part_ids, .. } = &mut document.outline[0] else {
            panic!("expected a part envelope");
        };
        *part_ids = document.parts.iter().map(|part| part.id.clone()).collect();
        document.id.push_str(&format!("-{}-keys", 5 * columns));
        document.revision = 0;
    }
    document
}

fn stagger(column: u32, value: f64) -> EditOperation {
    EditOperation::SetMatrixPlacement {
        matrix_id: "matrix".into(),
        change: MatrixPlacementChange::ColumnStagger { column, value },
    }
}

#[test]
fn settled_stagger_matches_cold_wire_and_history() {
    let mut engine = CoreEngine::new();
    let mut cold = CoreEngine::new();
    let mut document = stagger_profile_document(true);
    document.matrices[0].column_layout = vec![ColumnLayout {
        splay: 9.0,
        ..Default::default()
    }];
    document.matrices[0].mirror = Some(Mirror::X);
    compare(
        &mut engine,
        &mut cold,
        CoreRequest::Open {
            id: "open".into(),
            document,
        },
    );
    let before = serde_json::to_vec(&engine.document).unwrap();
    for operation in [stagger(11, -0.0), stagger(12, 4.0), stagger(11, f64::NAN)] {
        compare(&mut engine, &mut cold, edit(0, operation));
        assert_eq!(serde_json::to_vec(&engine.document).unwrap(), before);
        assert!(engine.undo.is_empty());
    }
    let mut draft = engine.document.clone();
    let last = stagger(11, 4.0);
    let expected = ["matrix".to_string()]
        .into_iter()
        .chain((0..5).map(|row| format!("matrix/matrix/r{row}c11")))
        .collect::<Vec<_>>();
    assert_eq!(
        engine.apply_layout_edit(&mut draft, &[], &last).unwrap(),
        (expected.clone(), true),
    );
    let mut preview = edit(0, last.clone());
    let CoreRequest::Edit { command, .. } = &mut preview else {
        unreachable!()
    };
    command.phase = EditPhase::Preview;
    assert!(matches!(
        compare(&mut engine, &mut cold, preview),
        CoreReply::Preview { scene, .. } if scene.changed_ids == expected
    ));
    assert_eq!(serde_json::to_vec(&engine.document).unwrap(), before);
    assert!(matches!(
        compare(&mut engine, &mut cold, edit(0, last)),
        CoreReply::Scene { scene, .. } if scene.changed_ids == expected && scene.revision == 1
    ));
    for request in [
        edit(1, stagger(0, 2.0)),
        edit(2, stagger(11, -0.0)),
        edit(3, stagger(11, 0.0)),
        CoreRequest::Undo { id: "undo".into() },
        CoreRequest::Redo { id: "redo".into() },
        CoreRequest::Snapshot {
            id: "snapshot".into(),
        },
    ] {
        compare(&mut engine, &mut cold, request);
    }
    assert_eq!(
        engine.document.matrices[0].column_layout[11]
            .stagger
            .to_bits(),
        (-0.0f64).to_bits()
    );
}

#[test]
fn stagger_uses_full_writeback_to_repair_saved_envelope_membership() {
    let mut engine = CoreEngine::new();
    let mut cold = CoreEngine::new();
    let mut document = synthetic_outline();
    let OutlineFeature::PartEnvelope { part_ids, .. } = &mut document.outline[0] else {
        panic!("expected a part envelope");
    };
    let missing = part_ids.pop().unwrap();
    compare(
        &mut engine,
        &mut cold,
        CoreRequest::Open {
            id: "open".into(),
            document,
        },
    );
    let operation = stagger(1, 4.0);
    let mut draft = engine.document.clone();
    assert!(
        !engine
            .apply_layout_edit(&mut draft, &[], &operation)
            .unwrap()
            .1
    );
    compare(&mut engine, &mut cold, edit(0, operation));
    let OutlineFeature::PartEnvelope { part_ids, .. } = &engine.document.outline[0] else {
        panic!("expected a part envelope");
    };
    assert_eq!(part_ids.last(), Some(&missing));
}

fn edit(revision: u64, operation: EditOperation) -> CoreRequest {
    CoreRequest::Edit {
        id: "edit".into(),
        command: EditCommand {
            base_revision: revision,
            transaction_id: "project edit".into(),
            phase: EditPhase::Commit,
            target_ids: vec![],
            operation,
        },
    }
}

fn compare(engine: &mut CoreEngine, cold: &mut CoreEngine, request: CoreRequest) -> CoreReply {
    cold.reusable_scene = None;
    let expected = cold.handle(request.clone());
    let actual = engine.handle(request);
    assert_eq!(
        boardstudio_model::wire::encode_core_reply(&actual),
        boardstudio_model::wire::encode_core_reply(&expected),
    );
    if let CoreReply::Scene {
        id,
        scene,
        document,
    } = &actual
    {
        let (cache, contours, findings) = outlines(document, None, &[]);
        assert_eq!(
            scene.board_contours,
            geometry::board_contours(document, &cache).0
        );
        assert_eq!(
            scene.board_outline_scenes,
            geometry::board_outline_scenes(document, &cache)
        );
        let fresh = cold.scene(
            id.clone(),
            SceneInput {
                transaction: &scene.transaction_id,
                changed: scene.changed_ids.clone(),
                document,
                contours: &contours,
                geom_findings: &findings,
                cache: &cache,
                kind: SceneKind::Committed,
            },
        );
        assert_eq!(
            boardstudio_model::wire::encode_core_reply(&actual),
            boardstudio_model::wire::encode_core_reply(&fresh),
        );
    }
    actual
}

fn open_preview_fixture(document: ProjectDoc) -> (CoreEngine, CoreEngine) {
    let mut engine = CoreEngine::new();
    let mut cold = CoreEngine::new();
    assert!(matches!(
        compare(
            &mut engine,
            &mut cold,
            CoreRequest::Open {
                id: "open".into(),
                document,
            },
        ),
        CoreReply::Scene { .. }
    ));
    assert!(engine.reusable_scene.is_some());
    (engine, cold)
}

fn preview_move(positions: Vec<Position>) -> CoreRequest {
    CoreRequest::Edit {
        id: "preview move".into(),
        command: EditCommand {
            base_revision: 0,
            transaction_id: "move".into(),
            phase: EditPhase::Preview,
            target_ids: vec![],
            operation: EditOperation::MoveParts { positions },
        },
    }
}

#[test]
fn plain_move_commit_reuses_only_a_matching_preview() {
    for mismatch in [
        None,
        Some("position"),
        Some("transaction"),
        Some("targets"),
        Some("edit"),
    ] {
        let document = fixture();
        let (mut engine, mut cold) = open_preview_fixture(document);
        let request = preview_move(vec![Position {
            id: "matrix/matrix/r0c1".into(),
            at: Vec2 { x: 22.0, y: 4.0 },
        }]);
        assert!(matches!(
            engine.handle(request.clone()),
            CoreReply::Preview { .. }
        ));
        assert!(engine.move_preview.is_some());
        let CoreRequest::Edit { id, mut command } = request else {
            unreachable!()
        };
        command.phase = EditPhase::Commit;
        match mismatch {
            Some("position") => {
                let EditOperation::MoveParts { positions } = &mut command.operation else {
                    unreachable!()
                };
                positions[0].at.y += 1.0;
            }
            Some("transaction") => command.transaction_id.push_str(" other"),
            Some("targets") => command.target_ids.push("other".into()),
            Some("edit") => {
                let CoreRequest::Edit { mut command, id } = edit(0, rename("draft")) else {
                    unreachable!()
                };
                command.phase = EditPhase::Preview;
                assert!(matches!(
                    engine.handle(CoreRequest::Edit { id, command }),
                    CoreReply::Preview { .. }
                ));
                assert!(engine.move_preview.is_none());
            }
            _ => {}
        }
        let projections = engine.full_scene_projections.get();
        let CoreReply::Scene { scene, .. } =
            compare(&mut engine, &mut cold, CoreRequest::Edit { id, command })
        else {
            panic!("the move should commit");
        };
        assert_eq!(scene.revision, 1);
        assert!(
            scene
                .changed_ids
                .iter()
                .any(|id| id == "matrix/matrix/r0c1")
        );
        assert_eq!(
            engine.full_scene_projections.get() - projections,
            usize::from(mismatch.is_some())
        );
        assert!(engine.move_preview.is_none());
    }
}

#[test]
fn matched_preview_of_a_mirrored_move_reprojects_the_commit() {
    use mirrored_half::{MirrorDestination, MirroredPair};

    let mut document = fixture();
    let source = document.boards[0].id.clone();
    document.boards.push(Board {
        id: "other-half".into(),
        name: "Other half".into(),
        outline_ids: vec![],
        part_ids: vec![],
        net_ids: vec![],
        thickness: 1.6,
        traces: vec![],
        vias: vec![],
    });
    document.mirrored_pairs.push(MirroredPair {
        id: "pair".into(),
        source: MirrorDestination::SecondBoard(source),
        destination: MirrorDestination::SecondBoard("other-half".into()),
        mapping: Pose2 {
            at: Vec2 { x: 100.0, y: 0.0 },
            rotation: 0.0,
        },
        overrides: vec![],
    });
    let open = CoreRequest::Open {
        id: "open".into(),
        document,
    };
    let mut engine = CoreEngine::new();
    let mut cold = CoreEngine::new();
    for session in [&mut engine, &mut cold] {
        assert!(matches!(
            session.handle(open.clone()),
            CoreReply::Scene { .. }
        ));
    }
    let source_id = String::from("matrix/matrix/r0c1");
    let sources = [source_id.clone()];
    let target_id = engine
        .resolved
        .affected_targets(&sources)
        .next()
        .unwrap()
        .clone();
    let request = preview_move(vec![Position {
        id: source_id.clone(),
        at: Vec2 { x: 22.0, y: 4.0 },
    }]);
    assert!(matches!(
        engine.handle(request.clone()),
        CoreReply::Preview { .. }
    ));
    assert!(engine.move_preview.is_none());
    let projections = engine.full_scene_projections.get();
    let CoreRequest::Edit { id, mut command } = request else {
        unreachable!()
    };
    command.phase = EditPhase::Commit;
    let commit = CoreRequest::Edit { id, command };
    let actual = engine.handle(commit.clone());
    assert_eq!(engine.full_scene_projections.get(), projections + 1);
    assert_eq!(
        boardstudio_model::wire::encode_core_reply(&actual),
        boardstudio_model::wire::encode_core_reply(&cold.handle(commit)),
    );
    let CoreReply::Scene { scene, .. } = actual else {
        panic!("expected mirrored move scene")
    };
    let at = |id| {
        scene
            .transforms
            .iter()
            .find(|item| item.id == id)
            .unwrap()
            .pose
            .at
    };
    let source = at(source_id);
    let target = at(target_id);
    assert_eq!(
        target,
        Vec2 {
            x: 100.0 - source.x,
            y: source.y
        }
    );
}

fn assert_preview_preserves_snapshot(engine: &mut CoreEngine, cold: &mut CoreEngine) {
    assert_eq!(engine.document, cold.document);
    assert_eq!(
        serde_json::to_vec(&engine.document).unwrap(),
        serde_json::to_vec(&cold.document).unwrap(),
    );
    assert!(engine.undo.is_empty() && engine.redo.is_empty());
    assert!(matches!(
        compare(
            engine,
            cold,
            CoreRequest::Snapshot { id: "snapshot".into() },
        ),
        CoreReply::Scene { scene, .. }
            if scene.revision == 0 && scene.transaction_id == "snapshot"
                && scene.changed_ids.is_empty()
    ));
}

#[test]
fn anchored_drag_preview_does_not_rewrite_accepted_spread() {
    let mut document = fixture();
    document.layouts[0].part_ids.push("main/U1".into());
    document
        .parts
        .iter_mut()
        .find(|part| part.id == "main/U1")
        .unwrap()
        .anchor = Some(PartAnchor {
        matrix_id: "matrix".into(),
        spread: 18.0,
        stagger: 32.0,
        ..Default::default()
    });
    document.constraints.push(Constraint::Offset {
        id: "controller-dependent".into(),
        source_part_id: "main/U1".into(),
        target_part_id: "main/SW2".into(),
        offset: Vec2 { x: 18.0, y: 0.0 },
        rotation: 0.0,
    });
    document.outline.push(OutlineFeature::Rect {
        id: "controller-cutout".into(),
        anchor_part_id: Some("main/U1".into()),
        center: Vec2::default(),
        size: Vec2 { x: 2.0, y: 2.0 },
        radius: 0.0,
        rotation: None,
        operation: Operation::Subtract,
    });
    document.boards[0]
        .outline_ids
        .push("controller-cutout".into());
    let (mut engine, mut cold) = open_preview_fixture(document);
    let position = Position {
        id: "main/U1".into(),
        at: Vec2 { x: 23.0, y: 34.0 },
    };
    let CoreReply::Preview { scene, .. } = engine.handle(preview_move(vec![position.clone()]))
    else {
        panic!("anchored drag should preview");
    };
    assert!(
        scene
            .transforms
            .iter()
            .any(|transform| { transform.id == "main/U1" && transform.pose.at == position.at })
    );
    assert!(scene.transforms.iter().any(|transform| {
        transform.id == "main/SW2" && transform.pose.at == Vec2 { x: 41.0, y: 34.0 }
    }));
    assert_eq!(
        engine
            .document
            .parts
            .iter()
            .find(|part| part.id == "main/U1")
            .and_then(|part| part.anchor.as_ref())
            .map(|anchor| anchor.spread),
        Some(18.0),
    );
    assert_preview_preserves_snapshot(&mut engine, &mut cold);

    // The first position already writes the draft when the second one fails.
    assert!(matches!(
        engine.handle(preview_move(vec![
            position,
            Position { id: "missing".into(), at: Vec2::default() },
        ])),
        CoreReply::Error { message, revision: 0, .. } if message == "Unknown part missing"
    ));
    assert_preview_preserves_snapshot(&mut engine, &mut cold);

    // Drag the anchor key: the cutout must follow U1's evaluated pose.
    let positions = vec![Position {
        id: "matrix/matrix/r0c0".into(),
        at: Vec2 { x: 0.0, y: 5.0 },
    }];
    let CoreReply::Preview { scene: preview, .. } = engine.handle(preview_move(positions.clone()))
    else {
        panic!("anchor key drag should preview");
    };
    assert!(preview.transforms.iter().any(|transform| {
        transform.id == "main/U1" && transform.pose.at == Vec2 { x: 18.0, y: 37.0 }
    }));
    assert_preview_preserves_snapshot(&mut engine, &mut cold);

    let CoreReply::Scene {
        scene: committed,
        document,
        ..
    } = cold.handle(edit(0, EditOperation::MoveParts { positions }))
    else {
        panic!("anchor key drag should commit");
    };
    let (cache, _, _) = outlines(&document, None, &[]);
    let fresh = geometry::board_scene_geometry(&document, &cache);
    assert_eq!(preview.board_contours, fresh.contours);
    assert_eq!(committed.board_contours, fresh.contours);
}

#[test]
fn stagger_shares_only_the_same_ordered_outline_composition() {
    for (mut document, shares_composition) in [
        (fixture(), true),
        (stagger_profile_document(true), true),
        (fixture(), false),
    ] {
        if !shares_composition {
            // Applying the subtractions before the envelope changes the board
            // but leaves the legacy project contour in its original order.
            document.boards[0].outline_ids.rotate_left(1);
        }
        let (mut engine, mut cold) = open_preview_fixture(document);
        let before = serde_json::to_vec(&engine.document).unwrap();
        let accepted = engine.reusable_scene.as_ref().unwrap().clone();
        let column = engine.document.matrices[0].columns - 1;
        let mut preview = edit(0, stagger(column, 4.0));
        let CoreRequest::Edit { command, .. } = &mut preview else {
            unreachable!()
        };
        command.phase = EditPhase::Preview;
        let board_projections = engine.independent_board_projections.get();
        let CoreReply::Preview { scene, .. } = compare(&mut engine, &mut cold, preview) else {
            panic!("expected a stagger preview");
        };
        assert_ne!(scene.contours, accepted.contours);
        assert_eq!(
            scene.contours == scene.board_contours[0].contours,
            shares_composition
        );
        let expected_extra_projections = usize::from(!shares_composition);
        assert_eq!(
            engine.independent_board_projections.get() - board_projections,
            expected_extra_projections
        );
        assert_eq!(serde_json::to_vec(&engine.document).unwrap(), before);
        assert_eq!(engine.reusable_scene.as_ref(), Some(&accepted));
        assert!(engine.undo.is_empty() && engine.redo.is_empty());

        let cold_projections = cold.independent_board_projections.get();
        let full_projections = engine.full_scene_projections.get();
        assert!(matches!(
            compare(&mut engine, &mut cold, edit(0, stagger(column, 4.0))),
            CoreReply::Scene { scene: committed, .. } if {
                let mut expected = scene;
                expected.revision = 1;
                committed == expected
            }
        ));
        assert_eq!(
            engine.full_scene_projections.get() - full_projections,
            expected_extra_projections
        );
        assert_eq!(
            engine.independent_board_projections.get() - board_projections,
            2 * expected_extra_projections
        );
        // The independently projected cold reply and the fresh scene used by
        // compare both went through separate board composition.
        assert_eq!(
            cold.independent_board_projections.get() - cold_projections,
            2
        );
    }
}

#[test]
fn matching_part_poses_do_not_reuse_a_different_stagger_draft() {
    let mut document = stagger_profile_document(true);
    for part in &mut document.parts {
        if document.matrices[0].part_ids.contains(&part.id) {
            part.properties
                .get_or_insert_with(Default::default)
                .insert(matrix::OVERRIDE.into(), serde_json::json!(1));
        }
    }
    let (mut engine, mut cold) = open_preview_fixture(document);
    let initial_poses = engine.document.parts.clone();
    let mut preview = edit(0, stagger(0, 2.0));
    let CoreRequest::Edit { command, .. } = &mut preview else {
        unreachable!()
    };
    command.phase = EditPhase::Preview;
    compare(&mut engine, &mut cold, preview);
    assert!(engine.stagger_preview.as_ref().unwrap().settled.is_some());

    let CoreReply::Scene { document, .. } =
        compare(&mut engine, &mut cold, edit(0, stagger(0, 4.0)))
    else {
        panic!("expected stagger commit");
    };
    assert_eq!(document.parts, initial_poses);
    assert_eq!(document.matrices[0].column_layout[0].stagger, 4.0);
}

#[test]
fn stagger_preview_commit_and_invalidation_match_cold_projection() {
    let mut engine = CoreEngine::new();
    let mut cold = CoreEngine::new();
    let mut document = fixture();
    document.matrices[0].column_layout = vec![ColumnLayout {
        splay: -17.125,
        ..Default::default()
    }];
    compare(
        &mut engine,
        &mut cold,
        CoreRequest::Open {
            id: "open".into(),
            document,
        },
    );
    let stagger = |base, phase, value| {
        let mut request = edit(
            base,
            EditOperation::SetMatrixPlacement {
                matrix_id: "matrix".into(),
                change: MatrixPlacementChange::ColumnStagger { column: 1, value },
            },
        );
        let CoreRequest::Edit { command, .. } = &mut request else {
            unreachable!()
        };
        command.phase = phase;
        request
    };
    let value = -3.125_000_000_000_001;
    let initial = serde_json::to_vec(&engine.document).unwrap();
    let CoreReply::Preview { scene: preview, .. } = compare(
        &mut engine,
        &mut cold,
        stagger(0, EditPhase::Preview, value),
    ) else {
        panic!("stagger should preview");
    };
    assert_eq!(serde_json::to_vec(&engine.document).unwrap(), initial);
    assert!(engine.undo.is_empty() && engine.redo.is_empty());
    let CoreReply::Scene {
        mut scene,
        document,
        ..
    } = compare(&mut engine, &mut cold, stagger(0, EditPhase::Commit, value))
    else {
        panic!("stagger should commit");
    };
    assert_eq!(
        document.matrices[0].column_layout[1].stagger.to_bits(),
        value.to_bits()
    );
    assert_eq!(scene.revision, 1);
    scene.revision = preview.revision;
    assert_eq!(scene, preview);

    assert!(matches!(
        compare(&mut engine, &mut cold, stagger(0, EditPhase::Preview, 2.0)),
        CoreReply::Error { revision: 1, .. }
    ));
    assert!(matches!(
        compare(&mut engine, &mut cold, stagger(1, EditPhase::Commit, value)),
        CoreReply::Scene { scene, .. } if scene.revision == 1 && scene.changed_ids.is_empty()
    ));
    assert_eq!(engine.undo.len(), 1);
    compare(
        &mut engine,
        &mut cold,
        CoreRequest::Undo { id: "undo".into() },
    );
    compare(
        &mut engine,
        &mut cold,
        CoreRequest::Redo { id: "redo".into() },
    );

    let position = Position {
        id: "main/U1".into(),
        at: Vec2 { x: 19.0, y: 32.0 },
    };
    compare(
        &mut engine,
        &mut cold,
        edit(
            3,
            EditOperation::MoveParts {
                positions: vec![position],
            },
        ),
    );
    assert!(engine.reusable_scene.is_none());
    compare(
        &mut engine,
        &mut cold,
        CoreRequest::Snapshot {
            id: "snapshot after move".into(),
        },
    );
    compare(&mut engine, &mut cold, stagger(4, EditPhase::Preview, 1.5));
    compare(&mut engine, &mut cold, stagger(4, EditPhase::Commit, 1.5));
    let CoreReply::Scene { document, .. } = compare(
        &mut engine,
        &mut cold,
        CoreRequest::Snapshot {
            id: "snapshot".into(),
        },
    ) else {
        panic!("expected saved stagger scene");
    };
    assert_eq!(document.revision, 5);
    assert_eq!(document.matrices[0].column_layout[1].stagger, 1.5);

    let copy: EditOperation = serde_json::from_value(serde_json::json!({
        "kind": "apply-outline-edit", "edit": {
            "target": {"kind": "board", "boardId": "main"},
            "change": {"copy": {"version_id": "fixed", "name": "Fixed outline"}},
            "seed": 1, "reviewed": null
        }
    }))
    .unwrap();
    let CoreReply::Scene {
        scene: fixed,
        document: saved,
        ..
    } = compare(&mut engine, &mut cold, edit(5, copy))
    else {
        panic!("expected a saved fixed outline");
    };
    assert!(engine.reusable_scene.is_none());
    let board_projections = engine.independent_board_projections.get();
    let CoreReply::Preview { scene: preview, .. } =
        compare(&mut engine, &mut cold, stagger(6, EditPhase::Preview, 7.25))
    else {
        panic!("expected a preview with a fixed outline");
    };
    assert_eq!(
        engine.independent_board_projections.get(),
        board_projections + 1
    );
    assert_eq!(preview.board_contours, fixed.board_contours);
    assert_eq!(
        serde_json::to_vec(&engine.document).unwrap(),
        serde_json::to_vec(&saved).unwrap(),
    );
    compare(
        &mut engine,
        &mut cold,
        CoreRequest::Snapshot {
            id: "snapshot with fixed outline".into(),
        },
    );
}

#[test]
fn rename_and_stagger_match_full_scene_recomputation_through_history() {
    let mut engine = CoreEngine::new();
    let mut cold = CoreEngine::new();
    let CoreReply::Scene { scene: initial, .. } = compare(
        &mut engine,
        &mut cold,
        CoreRequest::Open {
            id: "open".into(),
            document: fixture(),
        },
    ) else {
        panic!("open should publish a scene");
    };
    assert!(!initial.findings.is_empty());
    assert!(!initial.finding_markers.is_empty());
    assert!(engine.reusable_scene.is_some());

    let mut document = engine.document.clone();
    document.name = "Renamed".into();
    assert!(replacement_keeps_scene(&engine.document, &mut document));
    let rename = EditOperation::ReplaceDocument {
        document: Box::new(document),
    };
    assert!(matches!(
        compare(&mut engine, &mut cold, edit(0, rename.clone())),
        CoreReply::Scene { scene, document, .. }
            if scene.revision == 1
                && document.name == "Renamed"
                && scene.findings == initial.findings
                && !scene.changed_ids.is_empty()
    ));
    assert!(matches!(
        compare(&mut engine, &mut cold, edit(0, rename)),
        CoreReply::Error { revision: 1, .. }
    ));

    let stagger = |column| EditOperation::SetMatrixPlacement {
        matrix_id: "matrix".into(),
        change: MatrixPlacementChange::ColumnStagger { column, value: 4.0 },
    };
    assert!(matches!(
        compare(&mut engine, &mut cold, edit(1, stagger(10))),
        CoreReply::Error { revision: 1, .. }
    ));
    let before = engine.document.clone();
    let CoreReply::Scene {
        scene, document, ..
    } = compare(&mut engine, &mut cold, edit(1, stagger(1)))
    else {
        panic!("stagger should publish a scene");
    };
    assert_eq!(
        settled_outline_changes(&before, &document, &stagger(1)),
        Some(vec!["matrix/matrix/r0c1".into()]),
    );
    assert!(
        scene
            .changed_ids
            .iter()
            .any(|id| id == "matrix/matrix/r0c1")
    );
    assert_ne!(scene.board_contours, initial.board_contours);

    // A replacement with a new name can also carry a real geometry change.
    let mut document = *document;
    document.name = "Renamed again".into();
    document.outline.pop();
    document.boards[0].outline_ids.pop();
    assert!(!replacement_keeps_scene(&engine.document, &mut document));
    compare(
        &mut engine,
        &mut cold,
        edit(
            2,
            EditOperation::ReplaceDocument {
                document: Box::new(document),
            },
        ),
    );
    for request in [
        CoreRequest::Undo {
            id: "undo replacement".into(),
        },
        CoreRequest::Undo {
            id: "undo stagger".into(),
        },
        CoreRequest::Undo {
            id: "undo rename".into(),
        },
        CoreRequest::Redo {
            id: "redo rename".into(),
        },
    ] {
        compare(&mut engine, &mut cold, request);
    }
    assert_eq!(engine.document.name, "Renamed");
    assert_eq!(engine.document.revision, 7);
}

#[test]
fn replacement_keeps_scene_checks_signed_zero_in_geometry() {
    let accepted = fixture();
    let mut next = accepted.clone();
    next.name = "Renamed".into();
    next.parts[0].pose.rotation = -0.0;
    assert_eq!(accepted.parts[0].pose.rotation, next.parts[0].pose.rotation);
    assert!(!replacement_keeps_scene(&accepted, &mut next));
    assert_eq!(next.name, "Renamed");
}

fn rename(name: &str) -> EditOperation {
    EditOperation::RenameProject { name: name.into() }
}

#[test]
fn typed_snapshot_reuses_the_accepted_scene_and_refreshes_its_receipt() {
    let mut engine = CoreEngine::new();
    let mut cold = CoreEngine::new();
    let snapshot = |id: &str| CoreRequest::Snapshot { id: id.into() };
    compare(&mut engine, &mut cold, snapshot("before open"));
    assert!(engine.reusable_scene.is_none());

    let open = |document| CoreRequest::Open {
        id: "open".into(),
        document,
    };
    compare(&mut engine, &mut cold, open(fixture()));
    compare(&mut engine, &mut cold, edit(0, rename("Renamed")));
    assert!(matches!(
        compare(&mut engine, &mut cold, edit(1, rename("Renamed"))),
        CoreReply::Scene { scene, .. } if scene.changed_ids.is_empty() && scene.revision == 1
    ));
    assert_eq!(engine.undo.len(), 1);

    let mut invalid = fixture();
    invalid.format.clear();
    assert!(matches!(
        compare(&mut engine, &mut cold, open(invalid)),
        CoreReply::Error { revision: 1, .. }
    ));
    let projections = engine.full_scene_projections.get();
    for id in ["first snapshot", "second snapshot"] {
        assert!(matches!(
            compare(&mut engine, &mut cold, snapshot(id)),
            CoreReply::Scene { id: reply_id, scene, document }
                if reply_id == id && scene.transaction_id == "snapshot"
                    && scene.changed_ids.is_empty() && scene.revision == 1
                    && document.name == "Renamed"
        ));
    }
    assert_eq!(engine.full_scene_projections.get(), projections);

    for request in [
        CoreRequest::Undo { id: "undo".into() },
        snapshot("after undo"),
        CoreRequest::Redo { id: "redo".into() },
        snapshot("after redo"),
    ] {
        compare(&mut engine, &mut cold, request);
    }

    // An explicit Open can reuse the former session's revision number.
    let mut reopened = fixture();
    reopened.revision = engine.document.revision;
    reopened.outline.clear();
    reopened.boards[0].outline_ids.clear();
    compare(&mut engine, &mut cold, open(reopened));
    assert!(matches!(
        compare(&mut engine, &mut cold, snapshot("after reopen")),
        CoreReply::Scene { scene, document, .. }
            if scene.contours.is_empty() && document.name == "Synthetic outline contract"
    ));
    assert!(engine.undo.is_empty() && engine.redo.is_empty());
}

#[test]
fn shared_courtyard_findings_keep_part_order_and_follow_staggered_poses() {
    let mut engine = CoreEngine::new();
    let mut cold = CoreEngine::new();
    let mut document = fixture();
    document.definitions[0].courtyard.swap(1, 2);
    // Give fabrication validation material independently of the invalid courtyard.
    document.outline = vec![OutlineFeature::Rect {
        id: "fixed perimeter".into(),
        anchor_part_id: None,
        center: Vec2::default(),
        size: Vec2 { x: 160.0, y: 160.0 },
        radius: 0.0,
        rotation: None,
        operation: Operation::Add,
    }];
    document.boards[0].outline_ids = vec!["fixed perimeter".into()];
    let targets = document
        .parts
        .iter()
        .filter(|part| part.definition_id == document.definitions[0].id)
        .map(|part| part.id.clone())
        .collect::<Vec<_>>();
    let check = |scene: &SceneDelta| {
        let reported = scene
            .findings
            .iter()
            .filter(|finding| finding.id.starts_with("board:main:outline:body-source:"))
            .map(|finding| finding.target_ids[1].clone())
            .collect::<Vec<_>>();
        assert_eq!(reported, targets);
        let id = "board:main:outline:body-source:matrix/matrix/r0c0";
        let marker = scene
            .finding_markers
            .iter()
            .find(|marker| marker.finding_id == id)
            .unwrap();
        marker.contours[0].points[0]
    };
    let CoreReply::Scene { scene: open, .. } = compare(
        &mut engine,
        &mut cold,
        CoreRequest::Open {
            id: "open".into(),
            document,
        },
    ) else {
        panic!("open should publish its fabrication findings");
    };
    let stagger = |phase| {
        let mut request = edit(
            0,
            EditOperation::SetMatrixPlacement {
                matrix_id: "matrix".into(),
                change: MatrixPlacementChange::ColumnStagger {
                    column: 0,
                    value: 4.0,
                },
            },
        );
        let CoreRequest::Edit { command, .. } = &mut request else {
            unreachable!()
        };
        command.phase = phase;
        request
    };
    let CoreReply::Preview { scene: preview, .. } =
        compare(&mut engine, &mut cold, stagger(EditPhase::Preview))
    else {
        panic!("stagger should preview its fabrication findings");
    };
    let CoreReply::Scene { scene: commit, .. } =
        compare(&mut engine, &mut cold, stagger(EditPhase::Commit))
    else {
        panic!("stagger should publish its fabrication findings");
    };
    let before = check(&open);
    let after = Vec2 {
        x: before.x,
        y: before.y + 4.0,
    };
    assert_eq!(check(&preview), after);
    assert_eq!(check(&commit), after);
}

#[test]
fn typed_rename_matches_legacy_preview_commit_and_history() {
    let mut typed = CoreEngine::new();
    let mut legacy = CoreEngine::new();
    let CoreReply::Scene { document, .. } = compare(
        &mut typed,
        &mut legacy,
        CoreRequest::Open {
            id: "open".into(),
            document: fixture(),
        },
    ) else {
        panic!("open should publish a scene");
    };
    let mut replacement = *document;
    replacement.name = "Élan / 測試".into();
    for phase in [EditPhase::Preview, EditPhase::Commit] {
        let request = |operation| {
            let mut request = edit(0, operation);
            let CoreRequest::Edit { command, .. } = &mut request else {
                unreachable!();
            };
            command.phase = phase;
            request
        };
        let actual = typed.handle(request(rename(&replacement.name)));
        let expected = legacy.handle(request(EditOperation::ReplaceDocument {
            document: Box::new(replacement.clone()),
        }));
        assert_eq!(
            boardstudio_model::wire::encode_core_reply(&actual),
            boardstudio_model::wire::encode_core_reply(&expected),
        );
        assert_eq!(typed.document, legacy.document);
        assert_eq!(
            typed.document.revision,
            u64::from(phase == EditPhase::Commit)
        );
    }

    for request in [
        edit(
            1,
            EditOperation::SetMatrixPlacement {
                matrix_id: "matrix".into(),
                change: MatrixPlacementChange::ColumnStagger {
                    column: 1,
                    value: 4.0,
                },
            },
        ),
        CoreRequest::Undo {
            id: "undo stagger".into(),
        },
        CoreRequest::Undo {
            id: "undo rename".into(),
        },
        CoreRequest::Redo {
            id: "redo rename".into(),
        },
        CoreRequest::Redo {
            id: "redo stagger".into(),
        },
        CoreRequest::Snapshot {
            id: "snapshot".into(),
        },
    ] {
        compare(&mut typed, &mut legacy, request);
    }
    assert_eq!(typed.document.name, replacement.name);
    assert_eq!(typed.document.revision, 6);
}

#[test]
fn typed_rename_validates_and_preserves_scene_without_a_reusable_projection() {
    let mut engine = CoreEngine::new();
    let mut fresh = CoreEngine::new();
    let mut document = fixture();
    document.revision = u64::MAX - 1;
    compare(
        &mut engine,
        &mut fresh,
        CoreRequest::Open {
            id: "open".into(),
            document,
        },
    );
    engine.reusable_scene = None;
    let revision = u64::MAX - 1;
    for (base, name, expected) in [
        (revision - 1, "Valid", "Stale base revision"),
        (revision, " \u{2003}\t", "Project name cannot be empty"),
    ] {
        assert!(matches!(
            engine.handle(edit(base, rename(name))),
            CoreReply::Error { message, revision: current, .. }
                if message == expected && current == revision
        ));
        assert!(engine.undo.is_empty());
    }
    let previous_name = engine.document.name.clone();
    assert!(matches!(
        engine.handle(edit(revision, rename(&previous_name))),
        CoreReply::Scene { scene, .. } if scene.revision == revision && scene.changed_ids.is_empty()
    ));
    assert!(engine.undo.is_empty());
    assert!(matches!(
        compare(&mut engine, &mut fresh, edit(revision, rename("  Renamed  "))),
        CoreReply::Scene { scene, document, .. }
            if document.name == "  Renamed  " && scene.revision == u64::MAX
    ));
    assert!(matches!(
        engine.handle(edit(u64::MAX, rename("Too late"))),
        CoreReply::Error { message, revision: u64::MAX, .. }
            if message == "Document revision is exhausted"
    ));
    assert_eq!(engine.document.name, "  Renamed  ");
    assert_eq!(engine.undo.len(), 1);
}

fn stagger_diagnostics_document(large: bool) -> ProjectDoc {
    let mut doc = stagger_profile_document(large);
    doc.definitions[0].pads.push(Pad {
        id: "switch-pin".into(),
        number: "1".into(),
        at: Vec2::default(),
        size: Vec2 { x: 2.8, y: 1.8 },
        shape: PadShape::Oval,
        drill: Some(1.2),
        plated: Some(true),
        side: None,
        rotation: Some(15.0),
        net_id: None,
    });
    doc.keycaps = Some(KeycapConfiguration {
        matrices: BTreeMap::from([(
            "matrix".into(),
            KeycapMatrixSettings {
                profile: Some(KeycapProfile::Dsa),
                mount: Some(KeycapMount::Mx),
                ..Default::default()
            },
        )]),
        boards: BTreeMap::from([(
            "main".into(),
            KeycapBoardSettings {
                clearance: 1.5,
                ..Default::default()
            },
        )]),
        keys: BTreeMap::from([(
            "matrix/matrix/r0c0".into(),
            KeycapKeySettings {
                units: Some(Vec2 { x: 1.5, y: 1.0 }),
                ..Default::default()
            },
        )]),
    });
    let last = doc.matrices[0].columns - 1;
    doc.outline.push(OutlineFeature::Rect {
        id: "switch-cutout".into(),
        anchor_part_id: Some(format!("matrix/matrix/r0c{last}")),
        center: Vec2::default(),
        size: Vec2 { x: 2.0, y: 2.0 },
        radius: 0.0,
        rotation: None,
        operation: Operation::Subtract,
    });
    doc.boards[0].outline_ids.push("switch-cutout".into());
    doc
}

#[test]
fn stagger_diagnostics_match_cold_wire_and_history() {
    for large in [false, true] {
        let document = stagger_diagnostics_document(large);
        for column in [0, document.matrices[0].columns - 1] {
            let (mut engine, mut cold) = open_preview_fixture(document.clone());
            let accepted = engine.document.clone();
            let mut requests = Vec::new();
            for value in [1.25, 2.75, 4.0] {
                let mut request = edit(0, stagger(column, value));
                let CoreRequest::Edit { command, .. } = &mut request else {
                    unreachable!()
                };
                command.phase = EditPhase::Preview;
                requests.push(request);
            }
            requests.extend([
                edit(0, stagger(column, 4.0)),
                CoreRequest::Undo { id: "undo".into() },
                CoreRequest::Redo { id: "redo".into() },
            ]);
            let mut last_preview = None;
            let mut committed = None;
            for request in requests {
                let reply = compare(&mut engine, &mut cold, request);
                match &reply {
                    CoreReply::Preview { scene, .. } => {
                        assert_eq!(engine.document, accepted);
                        assert!(engine.undo.is_empty() && engine.redo.is_empty());
                        for prefix in [
                            "board:main:outline:body:",
                            "board:main:outline:support:",
                            "keycaps/",
                        ] {
                            assert!(scene.finding_markers.iter().any(|marker| {
                                marker.finding_id.starts_with(prefix)
                                    && marker.board_id == "main"
                                    && !marker.contours.is_empty()
                                    && scene
                                        .findings
                                        .iter()
                                        .any(|finding| finding.id == marker.finding_id)
                            }));
                        }
                        last_preview = Some(scene.clone());
                    }
                    CoreReply::Scene {
                        scene, document, ..
                    } => match scene.revision {
                        1 => {
                            let mut preview = last_preview.take().unwrap();
                            preview.revision = 1;
                            assert_eq!(&preview, scene);
                            committed = Some(document.clone());
                        }
                        2 => {
                            let mut expected = accepted.clone();
                            expected.revision = 2;
                            assert_eq!(document.as_ref(), &expected);
                        }
                        3 => {
                            let mut expected = committed.take().unwrap();
                            expected.revision = 3;
                            assert_eq!(document, &expected);
                        }
                        _ => panic!("unexpected history revision"),
                    },
                    _ => panic!("expected a scene reply"),
                }
            }
        }
    }
}
