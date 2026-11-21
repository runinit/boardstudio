use super::*;

fn fixture() -> ProjectDoc {
    let mut doc: ProjectDoc = serde_json::from_str(include_str!(
        "../../../../tests/fixtures/synthetic-outline.json"
    ))
    .unwrap();
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
    let CoreReply::Preview { scene: preview, .. } =
        compare(&mut engine, &mut cold, stagger(6, EditPhase::Preview, 7.25))
    else {
        panic!("expected a preview with a fixed outline");
    };
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
        stagger_outline_changes(&before, &document, &stagger(1)),
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
