use super::*;
fn fixture() -> ProjectDoc {
    let mut doc = synthetic_outline();
    // Stagger accumulates across columns: edit the last column so one cutout moves while the first stays reusable.
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
fn stagger_document(columns: Option<u32>) -> ProjectDoc {
    // Matches the release Chrome profile's authored 5 × 12 fixture.
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
        assert!(apply_op(&mut engine, &open_req(document)).is_ok());
        let Ok(EditOutcome::Committed(CommittedScene {
            document: resized, ..
        })) = apply_op(
            &mut engine,
            &edit(
                0,
                EditOperation::SetMatrix {
                    matrix,
                    definitions: None,
                },
            ),
        )
        else {
            panic!("could not resize profile fixture");
        };
        document = resized;
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
/// One typed Core operation the warm/cold reuse harness replays on both engines.
#[derive(Clone, Debug)]
enum ReuseOp {
    Open(ProjectDoc),
    Edit(EditCommand),
    Undo,
    Redo,
    Snapshot,
}

fn open_req(document: ProjectDoc) -> ReuseOp {
    ReuseOp::Open(document)
}
fn preview_edit(op: &mut ReuseOp) {
    let ReuseOp::Edit(command) = op else {
        unreachable!()
    };
    command.phase = EditPhase::Preview;
}
fn preview_op(revision: u64, operation: EditOperation) -> ReuseOp {
    let mut req = edit(revision, operation);
    preview_edit(&mut req);
    req
}
fn apply_op(engine: &mut CoreEngine, op: &ReuseOp) -> Result<EditOutcome, String> {
    match op {
        ReuseOp::Open(document) => engine.open(document.clone()).map(EditOutcome::Committed),
        ReuseOp::Edit(command) => engine.apply_edit(command.clone()),
        ReuseOp::Undo => engine.undo().map(EditOutcome::Committed),
        ReuseOp::Redo => engine.redo().map(EditOutcome::Committed),
        ReuseOp::Snapshot => engine.snapshot().map(EditOutcome::Committed),
    }
}
fn assert_warm_matches_cold(
    engine: &mut CoreEngine,
    cold: &mut CoreEngine,
    request: ReuseOp,
) -> Result<EditOutcome, String> {
    let reply = compare(engine, cold, request);
    assert_eq!(engine.document, cold.document);
    assert_eq!(
        serde_json::to_vec(&engine.document).unwrap(),
        serde_json::to_vec(&cold.document).unwrap()
    );
    reply
}
#[test]
fn settled_stagger_matches_cold_wire_and_history() {
    let mut engine = CoreEngine::new();
    let mut cold = CoreEngine::new();
    let mut document = stagger_document(Some(12));
    document.matrices[0].column_layout = vec![ColumnLayout {
        splay: 9.0,
        ..Default::default()
    }];
    document.matrices[0].mirror = Some(Mirror::X);
    let _ = assert_warm_matches_cold(&mut engine, &mut cold, open_req(document));
    let before = serde_json::to_vec(&engine.document).unwrap();
    for operation in [stagger(11, -0.0), stagger(12, 4.0), stagger(11, f64::NAN)] {
        let _ = compare(&mut engine, &mut cold, edit(0, operation));
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
    let preview = preview_op(0, last.clone());
    assert!(matches!(
        compare(&mut engine, &mut cold, preview),
        Ok(EditOutcome::Preview(scene)) if scene.changed_ids == expected
    ));
    assert_eq!(serde_json::to_vec(&engine.document).unwrap(), before);
    assert!(matches!(
        compare(&mut engine, &mut cold, edit(0, last)),
        Ok(EditOutcome::Committed(CommittedScene { scene, .. })) if scene.changed_ids == expected && scene.revision == 1
    ));
    for request in [
        edit(1, stagger(0, 2.0)),
        edit(2, stagger(11, -0.0)),
        edit(3, stagger(11, 0.0)),
        ReuseOp::Undo,
        ReuseOp::Redo,
        ReuseOp::Snapshot,
    ] {
        let _ = compare(&mut engine, &mut cold, request);
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
    let _ = assert_warm_matches_cold(&mut engine, &mut cold, open_req(document));
    let operation = stagger(1, 4.0);
    let mut draft = engine.document.clone();
    assert!(
        !engine
            .apply_layout_edit(&mut draft, &[], &operation)
            .unwrap()
            .1
    );
    let _ = compare(&mut engine, &mut cold, edit(0, operation));
    let OutlineFeature::PartEnvelope { part_ids, .. } = &engine.document.outline[0] else {
        panic!("expected a part envelope");
    };
    assert_eq!(part_ids.last(), Some(&missing));
}

fn edit(revision: u64, operation: EditOperation) -> ReuseOp {
    ReuseOp::Edit(EditCommand {
        base_revision: revision,
        transaction_id: "project edit".into(),
        phase: EditPhase::Commit,
        target_ids: vec![],
        operation,
    })
}

fn compare(
    engine: &mut CoreEngine,
    cold: &mut CoreEngine,
    request: ReuseOp,
) -> Result<EditOutcome, String> {
    cold.reusable_scene = None;
    let expected = apply_op(cold, &request);
    let actual = apply_op(engine, &request);
    assert_eq!(format!("{actual:?}"), format!("{expected:?}"));
    if let Ok(EditOutcome::Committed(committed)) = &actual {
        let (cache, contours, findings) = outlines(&committed.document, None, &[]);
        assert_eq!(
            committed.scene.board_contours,
            geometry::board_contours(&committed.document, &cache).0
        );
        assert_eq!(
            committed.scene.board_outline_scenes,
            geometry::board_outline_scenes(&committed.document, &cache)
        );
        let fresh_scene = cold
            .scene_with_board_geometry(
                SceneInput {
                    transaction: &committed.scene.transaction_id,
                    changed: committed.scene.changed_ids.clone(),
                    document: &committed.document,
                    contours: &contours,
                    geom_findings: &findings,
                    cache: &cache,
                },
                None,
            )
            .unwrap();
        // The reusable scene must reproduce the full recomputation exactly.
        assert_eq!(format!("{fresh_scene:?}"), format!("{:?}", committed.scene));
    }
    actual
}
fn open_preview_fixture(document: ProjectDoc) -> (CoreEngine, CoreEngine) {
    let mut engine = CoreEngine::new();
    let mut cold = CoreEngine::new();
    assert!(assert_warm_matches_cold(&mut engine, &mut cold, open_req(document)).is_ok());
    assert!(engine.reusable_scene.is_some());
    (engine, cold)
}
fn preview_move(positions: Vec<Position>) -> ReuseOp {
    ReuseOp::Edit(EditCommand {
        base_revision: 0,
        transaction_id: "move".into(),
        phase: EditPhase::Preview,
        target_ids: vec![],
        operation: EditOperation::MoveParts { positions },
    })
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
            apply_op(&mut engine, &request),
            Ok(EditOutcome::Preview(_))
        ));
        assert!(engine.move_preview.is_some());
        let ReuseOp::Edit(mut command) = request else {
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
                let ReuseOp::Edit(mut command) = edit(0, rename("draft")) else {
                    unreachable!()
                };
                command.phase = EditPhase::Preview;
                assert!(matches!(
                    engine.apply_edit(command),
                    Ok(EditOutcome::Preview(_))
                ));
                assert!(engine.move_preview.is_none());
            }
            _ => {}
        }
        let projections = engine.full_scene_projections.get();
        let Ok(EditOutcome::Committed(CommittedScene { scene, .. })) =
            compare(&mut engine, &mut cold, ReuseOp::Edit(command))
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
    let open = open_req(document);
    let mut engine = CoreEngine::new();
    let mut cold = CoreEngine::new();
    for session in [&mut engine, &mut cold] {
        assert!(apply_op(session, &open).is_ok());
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
        apply_op(&mut engine, &request),
        Ok(EditOutcome::Preview(_))
    ));
    assert!(engine.move_preview.is_none());
    let projections = engine.full_scene_projections.get();
    let ReuseOp::Edit(mut command) = request else {
        unreachable!()
    };
    command.phase = EditPhase::Commit;
    let commit = ReuseOp::Edit(command);
    let actual = apply_op(&mut engine, &commit);
    assert_eq!(engine.full_scene_projections.get(), projections + 1);
    assert_eq!(
        format!("{actual:?}"),
        format!("{:?}", apply_op(&mut cold, &commit)),
    );
    let Ok(EditOutcome::Committed(CommittedScene { scene, .. })) = actual else {
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
            ReuseOp::Snapshot,
        ),
        Ok(EditOutcome::Committed(CommittedScene { scene, .. }))
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
    let Ok(EditOutcome::Preview(preview)) =
        apply_op(&mut engine, &preview_move(vec![position.clone()]))
    else {
        panic!("anchored drag should preview");
    };
    assert!(
        preview
            .transforms
            .iter()
            .any(|transform| { transform.id == "main/U1" && transform.pose.at == position.at })
    );
    assert!(preview.transforms.iter().any(|transform| {
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
        apply_op(&mut engine, &preview_move(vec![
            position,
            Position { id: "missing".into(), at: Vec2::default() },
        ])),
        Err(message) if message == "Unknown part missing"
    ));
    assert_preview_preserves_snapshot(&mut engine, &mut cold);

    // Drag the anchor key: the cutout must follow U1's evaluated pose.
    let positions = vec![Position {
        id: "matrix/matrix/r0c0".into(),
        at: Vec2 { x: 0.0, y: 5.0 },
    }];
    let Ok(EditOutcome::Preview(preview)) = apply_op(&mut engine, &preview_move(positions.clone()))
    else {
        panic!("anchor key drag should preview");
    };
    assert!(preview.transforms.iter().any(|transform| {
        transform.id == "main/U1" && transform.pose.at == Vec2 { x: 18.0, y: 37.0 }
    }));
    assert_preview_preserves_snapshot(&mut engine, &mut cold);

    let Ok(EditOutcome::Committed(CommittedScene {
        scene: committed,
        document,
    })) = apply_op(&mut cold, &edit(0, EditOperation::MoveParts { positions }))
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
        (stagger_document(Some(12)), true),
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
        let preview = preview_op(0, stagger(column, 4.0));
        let board_projections = engine.independent_board_projections.get();
        let Ok(EditOutcome::Preview(scene)) = compare(&mut engine, &mut cold, preview) else {
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
            Ok(EditOutcome::Committed(CommittedScene { scene: committed, .. })) if {
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
    let mut document = stagger_document(Some(12));
    for part in &mut document.parts {
        if document.matrices[0].part_ids.contains(&part.id) {
            part.properties
                .get_or_insert_with(Default::default)
                .insert(matrix::OVERRIDE.into(), serde_json::json!(1));
        }
    }
    let (mut engine, mut cold) = open_preview_fixture(document);
    let initial_poses = engine.document.parts.clone();
    let preview = preview_op(0, stagger(0, 2.0));
    let _ = compare(&mut engine, &mut cold, preview);
    assert!(engine.stagger_preview.as_ref().unwrap().settled.is_some());
    let Ok(EditOutcome::Committed(CommittedScene { document, .. })) =
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
    let _ = assert_warm_matches_cold(&mut engine, &mut cold, open_req(document));
    let value = -3.125_000_000_000_001;
    let initial = serde_json::to_vec(&engine.document).unwrap();
    let Ok(EditOutcome::Preview(preview)) =
        compare(&mut engine, &mut cold, preview_op(0, stagger(1, value)))
    else {
        panic!("stagger should preview");
    };
    assert_eq!(serde_json::to_vec(&engine.document).unwrap(), initial);
    assert!(engine.undo.is_empty() && engine.redo.is_empty());
    let Ok(EditOutcome::Committed(CommittedScene {
        mut scene,
        document,
        ..
    })) = compare(&mut engine, &mut cold, edit(0, stagger(1, value)))
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
    assert!(compare(&mut engine, &mut cold, preview_op(0, stagger(1, 2.0))).is_err());
    assert!(matches!(
        compare(&mut engine, &mut cold, edit(1, stagger(1, value))),
        Ok(EditOutcome::Committed(CommittedScene { scene, .. })) if scene.revision == 1 && scene.changed_ids.is_empty()
    ));
    assert_eq!(engine.undo.len(), 1);
    let _ = compare(&mut engine, &mut cold, ReuseOp::Undo);
    let _ = compare(&mut engine, &mut cold, ReuseOp::Redo);

    let position = Position {
        id: "main/U1".into(),
        at: Vec2 { x: 19.0, y: 32.0 },
    };
    let _ = compare(
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
    let _ = compare(&mut engine, &mut cold, ReuseOp::Snapshot);
    let _ = compare(&mut engine, &mut cold, preview_op(4, stagger(1, 1.5)));
    let _ = compare(&mut engine, &mut cold, edit(4, stagger(1, 1.5)));
    let Ok(EditOutcome::Committed(CommittedScene { document, .. })) =
        compare(&mut engine, &mut cold, ReuseOp::Snapshot)
    else {
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
    let Ok(EditOutcome::Committed(CommittedScene {
        scene: fixed,
        document: saved,
        ..
    })) = compare(&mut engine, &mut cold, edit(5, copy))
    else {
        panic!("expected a saved fixed outline");
    };
    assert!(engine.reusable_scene.is_none());
    let board_projections = engine.independent_board_projections.get();
    let Ok(EditOutcome::Preview(preview)) =
        compare(&mut engine, &mut cold, preview_op(6, stagger(1, 7.25)))
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
    let _ = compare(&mut engine, &mut cold, ReuseOp::Snapshot);
}
#[test]
fn rename_and_stagger_match_full_scene_recomputation_through_history() {
    let mut engine = CoreEngine::new();
    let mut cold = CoreEngine::new();
    let Ok(EditOutcome::Committed(CommittedScene { scene: initial, .. })) =
        assert_warm_matches_cold(&mut engine, &mut cold, open_req(fixture()))
    else {
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
        Ok(EditOutcome::Committed(CommittedScene { scene, document }))
            if scene.revision == 1
                && document.name == "Renamed"
                && scene.findings == initial.findings
                && !scene.changed_ids.is_empty()
    ));
    assert!(compare(&mut engine, &mut cold, edit(0, rename)).is_err());

    let stagger = |column| EditOperation::SetMatrixPlacement {
        matrix_id: "matrix".into(),
        change: MatrixPlacementChange::ColumnStagger { column, value: 4.0 },
    };
    assert!(compare(&mut engine, &mut cold, edit(1, stagger(10))).is_err());
    let before = engine.document.clone();
    let Ok(EditOutcome::Committed(CommittedScene { scene, document })) =
        compare(&mut engine, &mut cold, edit(1, stagger(1)))
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
    let mut document = document;
    document.name = "Renamed again".into();
    document.outline.pop();
    document.boards[0].outline_ids.pop();
    assert!(!replacement_keeps_scene(&engine.document, &mut document));
    let _ = compare(
        &mut engine,
        &mut cold,
        edit(
            2,
            EditOperation::ReplaceDocument {
                document: Box::new(document),
            },
        ),
    );
    for request in [ReuseOp::Undo, ReuseOp::Undo, ReuseOp::Undo, ReuseOp::Redo] {
        let _ = compare(&mut engine, &mut cold, request);
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
    let snapshot = |_id: &str| ReuseOp::Snapshot;
    let _ = compare(&mut engine, &mut cold, snapshot("before open"));
    assert!(engine.reusable_scene.is_none());
    let _ = compare(&mut engine, &mut cold, open_req(fixture()));
    let _ = compare(&mut engine, &mut cold, edit(0, rename("Renamed")));
    assert!(matches!(
        compare(&mut engine, &mut cold, edit(1, rename("Renamed"))),
        Ok(EditOutcome::Committed(CommittedScene { scene, .. })) if scene.changed_ids.is_empty() && scene.revision == 1
    ));
    assert_eq!(engine.undo.len(), 1);
    let mut invalid = fixture();
    invalid.format.clear();
    assert!(compare(&mut engine, &mut cold, open_req(invalid)).is_err());
    let projections = engine.full_scene_projections.get();
    for _id in ["first snapshot", "second snapshot"] {
        assert!(matches!(
            compare(&mut engine, &mut cold, snapshot(_id)),
            Ok(EditOutcome::Committed(CommittedScene { scene, document }))
                if scene.transaction_id == "snapshot"
                    && scene.changed_ids.is_empty() && scene.revision == 1
                    && document.name == "Renamed"
        ));
    }
    assert_eq!(engine.full_scene_projections.get(), projections);

    for request in [
        ReuseOp::Undo,
        snapshot("after undo"),
        ReuseOp::Redo,
        snapshot("after redo"),
    ] {
        let _ = compare(&mut engine, &mut cold, request);
    }
    // An explicit Open can reuse the former session's revision number.
    let mut reopened = fixture();
    reopened.revision = engine.document.revision;
    reopened.outline.clear();
    reopened.boards[0].outline_ids.clear();
    let _ = compare(&mut engine, &mut cold, open_req(reopened));
    assert!(matches!(
        compare(&mut engine, &mut cold, snapshot("after reopen")),
        Ok(EditOutcome::Committed(CommittedScene { scene, document }))
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
    let Ok(EditOutcome::Committed(CommittedScene { scene: open, .. })) =
        assert_warm_matches_cold(&mut engine, &mut cold, open_req(document))
    else {
        panic!("open should publish its fabrication findings");
    };
    let Ok(EditOutcome::Preview(preview)) =
        compare(&mut engine, &mut cold, preview_op(0, stagger(0, 4.0)))
    else {
        panic!("stagger should preview its fabrication findings");
    };
    let Ok(EditOutcome::Committed(CommittedScene { scene: commit, .. })) =
        compare(&mut engine, &mut cold, edit(0, stagger(0, 4.0)))
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
    let Ok(EditOutcome::Committed(CommittedScene { document, .. })) =
        assert_warm_matches_cold(&mut typed, &mut legacy, open_req(fixture()))
    else {
        panic!("open should publish a scene");
    };
    let mut replacement = document;
    replacement.name = "Élan / 測試".into();
    for phase in [EditPhase::Preview, EditPhase::Commit] {
        let request = |operation| {
            let mut request = edit(0, operation);
            let ReuseOp::Edit(command) = &mut request else {
                unreachable!();
            };
            command.phase = phase;
            request
        };
        let actual = apply_op(&mut typed, &request(rename(&replacement.name)));
        let expected = apply_op(
            &mut legacy,
            &request(EditOperation::ReplaceDocument {
                document: Box::new(replacement.clone()),
            }),
        );
        assert_eq!(format!("{actual:?}"), format!("{expected:?}"));
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
        ReuseOp::Undo,
        ReuseOp::Undo,
        ReuseOp::Redo,
        ReuseOp::Redo,
        ReuseOp::Snapshot,
    ] {
        let _ = compare(&mut typed, &mut legacy, request);
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
    let _ = assert_warm_matches_cold(&mut engine, &mut fresh, open_req(document));
    engine.reusable_scene = None;
    let revision = u64::MAX - 1;
    for (base, name, expected) in [
        (revision - 1, "Valid", "Stale base revision"),
        (revision, " \u{2003}\t", "Project name cannot be empty"),
    ] {
        assert!(matches!(
            apply_op(&mut engine, &edit(base, rename(name))),
            Err(message) if message == expected
        ));
        assert_eq!(engine.document.revision, revision);
        assert!(engine.undo.is_empty());
    }
    let previous_name = engine.document.name.clone();
    assert!(matches!(
        apply_op(&mut engine, &edit(revision, rename(&previous_name))),
        Ok(EditOutcome::Committed(CommittedScene { scene, .. })) if scene.revision == revision && scene.changed_ids.is_empty()
    ));
    assert!(engine.undo.is_empty());
    assert!(matches!(
        compare(&mut engine, &mut fresh, edit(revision, rename("  Renamed  "))),
        Ok(EditOutcome::Committed(CommittedScene { scene, document }))
            if document.name == "  Renamed  " && scene.revision == u64::MAX
    ));
    assert!(matches!(
        apply_op(&mut engine, &edit(u64::MAX, rename("Too late"))),
        Err(message)
            if message == "Document revision is exhausted"
    ));
    assert_eq!(engine.document.name, "  Renamed  ");
    assert_eq!(engine.undo.len(), 1);
}
fn stagger_diagnostics_document(large: bool) -> ProjectDoc {
    let mut doc = stagger_document(large.then_some(12));
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
                requests.push(preview_op(0, stagger(column, value)));
            }
            requests.extend([edit(0, stagger(column, 4.0)), ReuseOp::Undo, ReuseOp::Redo]);
            let mut last_preview = None;
            let mut committed = None;
            for request in requests {
                let reply = compare(&mut engine, &mut cold, request);
                match &reply {
                    Ok(EditOutcome::Preview(scene)) => {
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
                    Ok(EditOutcome::Committed(CommittedScene { scene, document })) => {
                        match scene.revision {
                            1 => {
                                let mut preview = last_preview.take().unwrap();
                                preview.revision = 1;
                                assert_eq!(&preview, scene);
                                committed = Some(document.clone());
                            }
                            2 => {
                                let mut expected = accepted.clone();
                                expected.revision = 2;
                                assert_eq!(document, &expected);
                            }
                            3 => {
                                let mut expected = committed.take().unwrap();
                                expected.revision = 3;
                                assert_eq!(document, &expected);
                            }
                            _ => panic!("unexpected history revision"),
                        }
                    }
                    _ => panic!("expected a scene reply"),
                }
            }
        }
    }
}

mod stagger_geometry;
