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
        let fresh = engine.scene(
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
