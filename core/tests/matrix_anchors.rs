//! Ticket 19: Core-owned single-key anchor evaluation and typed Add/change/detach
//! edits. All behavior runs through public Core routes (`Open` / `Edit`), never
//! graph helpers alone.
use boardstudio_core::{CoreEngine, model::*};

fn scene(reply: CoreReply) -> (SceneDelta, ProjectDoc) {
    match reply {
        CoreReply::Scene {
            scene, document, ..
        } => (scene, *document),
        other => panic!("expected scene: {other:?}"),
    }
}

fn open(doc: ProjectDoc) -> (CoreEngine, SceneDelta, ProjectDoc) {
    let mut engine = CoreEngine::new();
    let (scene, doc) = scene(engine.handle(CoreRequest::Open {
        id: "open".into(),
        document: doc,
    }));
    (engine, scene, doc)
}

fn edit(
    engine: &mut CoreEngine,
    doc: &ProjectDoc,
    operation: EditOperation,
) -> (SceneDelta, ProjectDoc) {
    scene(engine.handle(CoreRequest::Edit {
        id: "edit".into(),
        command: EditCommand {
            base_revision: doc.revision,
            transaction_id: "edit".into(),
            phase: EditPhase::Commit,
            target_ids: vec![],
            operation,
        },
    }))
}

fn preview(engine: &mut CoreEngine, doc: &ProjectDoc, operation: EditOperation) -> SceneDelta {
    match engine.handle(CoreRequest::Edit {
        id: "preview".into(),
        command: EditCommand {
            base_revision: doc.revision,
            transaction_id: "preview".into(),
            phase: EditPhase::Preview,
            target_ids: vec![],
            operation,
        },
    }) {
        CoreReply::Preview { scene, .. } => scene,
        other => panic!("expected preview: {other:?}"),
    }
}

fn error(engine: &mut CoreEngine, doc: &ProjectDoc, operation: EditOperation) -> String {
    match engine.handle(CoreRequest::Edit {
        id: "edit".into(),
        command: EditCommand {
            base_revision: doc.revision,
            transaction_id: "edit".into(),
            phase: EditPhase::Commit,
            target_ids: vec![],
            operation,
        },
    }) {
        CoreReply::Error { message, .. } => message,
        other => panic!("expected error: {other:?}"),
    }
}

fn definitions() -> Vec<PartDefinition> {
    serde_json::from_value(serde_json::json!([
        {"id":"switch","name":"Switch","kind":"switch","courtyard":[],"pads":[]}
    ]))
    .unwrap()
}

fn key_part(id: &str, x: f64, y: f64) -> Part {
    Part {
        id: id.into(),
        definition_id: "switch".into(),
        reference: id.into(),
        pose: Pose2 {
            at: Vec2 { x, y },
            rotation: 0.0,
        },
        side: Side::Front,
        keycap: None,
        outline: None,
        locked: None,
        properties: None,
        generator_parameters: None,
    }
}

fn main_matrix() -> Matrix {
    Matrix {
        id: "main".into(),
        name: None,
        rows: 1,
        columns: 3,
        pitch: Vec2 { x: 19.0, y: 19.0 },
        origin: Vec2::default(),
        definition_id: "switch".into(),
        part_ids: vec![
            member("main", 0, 0),
            member("main", 0, 1),
            member("main", 0, 2),
        ],
        board_id: Some("board".into()),
        mirror: None,
        rotation: None,
        edge_gap: None,
        diode_direction: None,
        row_layout: vec![],
        column_layout: vec![],
        anchor: None,
        cells: vec![],
    }
}

fn fixture() -> (CoreEngine, ProjectDoc) {
    let mut doc = ProjectDoc::empty("anchors", "Anchors");
    doc.definitions = definitions();
    doc.parts = vec![
        key_part(&member("main", 0, 0), 0.0, 0.0),
        key_part(&member("main", 0, 1), 19.0, 0.0),
        key_part(&member("main", 0, 2), 38.0, 0.0),
    ];
    doc.boards.push(Board {
        id: "board".into(),
        name: "Board".into(),
        outline_ids: vec![],
        part_ids: vec![
            member("main", 0, 0),
            member("main", 0, 1),
            member("main", 0, 2),
        ],
        net_ids: vec![],
        thickness: 1.6,
        traces: vec![],
        vias: vec![],
    });
    doc.matrices.push(main_matrix());
    doc.layouts.push(Layout {
        id: "layout".into(),
        name: "Layout".into(),
        board_id: "board".into(),
        matrix_ids: vec!["main".into()],
        part_ids: vec![],
        mirror_link: None,
    });
    let (engine, _, doc) = open(doc);
    (engine, doc)
}

fn at(doc: &ProjectDoc, id: &str) -> Vec2 {
    doc.parts
        .iter()
        .find(|part| part.id == id)
        .unwrap_or_else(|| panic!("missing part {id}"))
        .pose
        .at
}

fn member(matrix: &str, row: u32, column: u32) -> String {
    format!("matrix/{matrix}/r{row}c{column}")
}

fn anchored_fixture() -> (CoreEngine, ProjectDoc) {
    let (mut engine, doc) = fixture();
    let doc = add_thumb(&mut engine, &doc);
    (engine, doc)
}

fn anchor_of(doc: &ProjectDoc, id: &str) -> Option<MatrixAnchor> {
    doc.matrices
        .iter()
        .find(|matrix| matrix.id == id)
        .and_then(|matrix| matrix.anchor.clone())
}

fn undone(engine: &mut CoreEngine) -> ProjectDoc {
    match engine.handle(CoreRequest::Undo { id: "undo".into() }) {
        CoreReply::Scene { document, .. } => *document,
        other => panic!("expected scene: {other:?}"),
    }
}

fn redone(engine: &mut CoreEngine) -> ProjectDoc {
    match engine.handle(CoreRequest::Redo { id: "redo".into() }) {
        CoreReply::Scene { document, .. } => *document,
        other => panic!("expected scene: {other:?}"),
    }
}

#[test]
fn change_anchor_retargets_dependants_with_preview_cancel_and_undo() {
    let (mut engine, doc) = anchored_fixture();
    let moved = MatrixAnchor {
        matrix_id: "main".into(),
        row: 0,
        column: 0,
        follow: Follow {
            x: true,
            y: true,
            rotation: true,
        },
        aggregate: None,
    };
    let operation = EditOperation::SetMatrixAnchor {
        matrix_id: "thumb".into(),
        anchor: moved.clone(),
    };
    let before = at(&doc, &member("thumb", 0, 0));
    assert_eq!(before, Vec2 { x: 38.0, y: -19.0 });
    let previewed = preview(&mut engine, &doc, operation.clone());
    let previewed_pose = previewed
        .matrix_scenes
        .iter()
        .find(|scene| scene.matrix_id == "thumb")
        .expect("thumb scene")
        .cells
        .iter()
        .find(|cell| cell.row == 0 && cell.column == 0)
        .expect("thumb cell")
        .pose
        .at;
    assert_eq!(previewed_pose, Vec2 { x: 0.0, y: -19.0 });
    // Preview leaves accepted state, revision and history unchanged: the whole
    // document (not just the matrix list) is byte-identical.
    assert_eq!(snapshot(&mut engine), doc);
    let (_, committed) = edit(&mut engine, &doc, operation);
    assert_eq!(anchor_of(&committed, "thumb"), Some(moved));
    assert_eq!(
        at(&committed, &member("thumb", 0, 0)),
        Vec2 { x: 0.0, y: -19.0 }
    );
    let undone_doc = undone(&mut engine);
    assert_eq!(anchor_of(&undone_doc, "thumb"), anchor_of(&doc, "thumb"));
    assert_eq!(redone(&mut engine).matrices, committed.matrices);
}

fn snapshot(engine: &mut CoreEngine) -> ProjectDoc {
    scene(engine.handle(CoreRequest::Snapshot { id: "snap".into() })).1
}

fn add_thumb(engine: &mut CoreEngine, doc: &ProjectDoc) -> ProjectDoc {
    edit(
        engine,
        doc,
        EditOperation::AddAnchoredMatrix {
            id: "thumb".into(),
            anchor: MatrixAnchor {
                matrix_id: "main".into(),
                row: 0,
                column: 2,
                follow: Follow {
                    x: true,
                    y: true,
                    rotation: true,
                },
                aggregate: None,
            },
        },
    )
    .1
}

#[test]
fn add_anchored_matrix_creates_1x3_in_same_layout_and_follows_its_anchor_key() {
    let (mut engine, doc) = fixture();
    let anchor_key = member("main", 0, 2);
    let before = at(&doc, &anchor_key);
    assert_eq!(before, Vec2 { x: 38.0, y: 0.0 });

    let (_, doc) = edit(
        &mut engine,
        &doc,
        EditOperation::AddAnchoredMatrix {
            id: "thumb".into(),
            anchor: MatrixAnchor {
                matrix_id: "main".into(),
                row: 0,
                column: 2,
                follow: Follow {
                    x: true,
                    y: true,
                    rotation: true,
                },
                aggregate: None,
            },
        },
    );
    let thumb = doc
        .matrices
        .iter()
        .find(|matrix| matrix.id == "thumb")
        .expect("added matrix is stored");
    assert_eq!((thumb.rows, thumb.columns), (1, 3));
    assert_eq!(thumb.origin, Vec2 { x: 0.0, y: -19.0 });
    assert_eq!(thumb.board_id.as_deref(), Some("board"));
    assert!(
        doc.layouts
            .iter()
            .find(|layout| layout.id == "layout")
            .is_some_and(|layout| layout.matrix_ids.contains(&"thumb".to_string()))
    );
    // Dependent members hang off the anchor key's final pose plus the relative origin.
    assert_eq!(at(&doc, &member("thumb", 0, 0)), Vec2 { x: 38.0, y: -19.0 });
    assert_eq!(at(&doc, &member("thumb", 0, 2)), Vec2 { x: 76.0, y: -19.0 });

    // Moving the anchor key moves the anchored members with it.
    let (_, doc) = edit(
        &mut engine,
        &doc,
        EditOperation::MoveParts {
            positions: vec![Position {
                id: anchor_key.clone(),
                at: Vec2 { x: 43.0, y: 7.0 },
            }],
        },
    );
    assert_eq!(at(&doc, &anchor_key), Vec2 { x: 43.0, y: 7.0 });
    assert_eq!(at(&doc, &member("thumb", 0, 0)), Vec2 { x: 43.0, y: -12.0 });
    assert_eq!(at(&doc, &member("thumb", 0, 2)), Vec2 { x: 81.0, y: -12.0 });
}

#[test]
fn add_anchored_matrix_previews_without_mutation_and_undoes_in_one_entry() {
    let (mut engine, doc) = fixture();
    let operation = EditOperation::AddAnchoredMatrix {
        id: "thumb".into(),
        anchor: MatrixAnchor {
            matrix_id: "main".into(),
            row: 0,
            column: 2,
            follow: Follow {
                x: true,
                y: true,
                rotation: true,
            },
            aggregate: None,
        },
    };
    // Preview evaluates the new members but stores nothing.
    let previewed = preview(&mut engine, &doc, operation.clone());
    assert!(
        previewed
            .matrix_scenes
            .iter()
            .any(|scene| scene.matrix_id == "thumb"),
        "preview must evaluate the added matrix"
    );
    assert_eq!(snapshot(&mut engine), doc);
    let (_, committed) = edit(&mut engine, &doc, operation);
    assert!(committed.matrices.iter().any(|matrix| matrix.id == "thumb"));
    assert!(
        committed
            .layouts
            .iter()
            .find(|layout| layout.id == "layout")
            .is_some_and(|layout| layout.matrix_ids.contains(&"thumb".to_string()))
    );
    // One Undo restores pre-Add membership: matrix, Layout entry and members.
    let undone_doc = undone(&mut engine);
    assert_eq!(undone_doc.matrices, doc.matrices);
    assert_eq!(undone_doc.layouts, doc.layouts);
    assert_eq!(undone_doc.parts, doc.parts);
    // The Add was a single history entry: nothing remains to undo.
    match engine.handle(CoreRequest::Undo {
        id: "undo-empty".into(),
    }) {
        CoreReply::Error { message, .. } => assert!(
            message.contains("History is empty"),
            "unexpected message: {message:?}"
        ),
        other => panic!("expected empty history, got: {other:?}"),
    }
    assert_eq!(redone(&mut engine).matrices, committed.matrices);
}

#[test]
fn reordered_matrix_lists_evaluate_to_the_same_poses() {
    let (mut engine, doc) = fixture();
    let doc = add_thumb(&mut engine, &doc);
    let forward: Vec<(String, Vec2)> = doc
        .parts
        .iter()
        .map(|part| (part.id.clone(), part.pose.at))
        .collect();
    let mut reordered = doc.clone();
    reordered.matrices.reverse();
    let (_, _, opened) = open(reordered);
    let backward: Vec<(String, Vec2)> = opened
        .parts
        .iter()
        .map(|part| (part.id.clone(), part.pose.at))
        .collect();
    assert_eq!(forward, backward);
}

fn pose_of(doc: &ProjectDoc, id: &str) -> Pose2 {
    doc.parts
        .iter()
        .find(|part| part.id == id)
        .unwrap_or_else(|| panic!("missing part {id}"))
        .pose
}

fn near(actual: Vec2, x: f64, y: f64) {
    assert!(
        (actual.x - x).abs() < 1e-9 && (actual.y - y).abs() < 1e-9,
        "expected ({x}, {y}), got ({}, {})",
        actual.x,
        actual.y
    );
}

#[test]
fn detach_keeps_world_frame_and_clears_the_anchor() {
    let (mut engine, doc) = anchored_fixture();
    // Turn the whole group so the kept frame carries a nonzero rotation.
    let mut turned = doc
        .matrices
        .iter()
        .find(|matrix| matrix.id == "main")
        .expect("main matrix")
        .clone();
    turned.rotation = Some(30.0);
    let (_, doc) = edit(
        &mut engine,
        &doc,
        EditOperation::SetMatrix {
            matrix: turned,
            definitions: None,
        },
    );
    let poses_before: Vec<(String, Pose2)> = doc
        .parts
        .iter()
        .map(|part| (part.id.clone(), part.pose))
        .collect();
    let (_, detached) = edit(
        &mut engine,
        &doc,
        EditOperation::DetachMatrixAnchor {
            matrix_id: "thumb".into(),
        },
    );
    assert_eq!(anchor_of(&detached, "thumb"), None);
    for (id, pose) in &poses_before {
        assert_eq!(pose_of(&detached, id), *pose, "{id}");
    }
    let thumb = detached
        .matrices
        .iter()
        .find(|matrix| matrix.id == "thumb")
        .expect("thumb matrix");
    // The stored frame is the actual evaluated frame, not world zero.
    let cos30 = 30f64.to_radians().cos();
    let sin30 = 30f64.to_radians().sin();
    near(
        thumb.origin,
        38.0 * cos30 + 19.0 * sin30,
        38.0 * sin30 - 19.0 * cos30,
    );
    assert_eq!(thumb.rotation, Some(30.0));
    // Moving the old anchor key no longer moves the detached members.
    let (_, moved) = edit(
        &mut engine,
        &detached,
        EditOperation::MoveParts {
            positions: vec![Position {
                id: member("main", 0, 2),
                at: Vec2 { x: 100.0, y: 100.0 },
            }],
        },
    );
    for (id, pose) in &poses_before {
        if id.starts_with("matrix/thumb/") {
            assert_eq!(pose_of(&moved, id), *pose, "{id}");
        }
    }
    near(pose_of(&moved, &member("main", 0, 2)).at, 100.0, 100.0);
    let undone_move = undone(&mut engine);
    assert_eq!(anchor_of(&undone_move, "thumb"), None);
    let undone_detach = undone(&mut engine);
    assert_eq!(anchor_of(&undone_detach, "thumb"), anchor_of(&doc, "thumb"));
    assert_eq!(redone(&mut engine).matrices, detached.matrices);
    assert_eq!(redone(&mut engine).matrices, moved.matrices);
}

fn linked_anchored_fixture() -> (CoreEngine, ProjectDoc) {
    let all = Follow {
        x: true,
        y: true,
        rotation: true,
    };
    let mut doc = ProjectDoc::empty("anchors-linked", "Anchors linked");
    doc.definitions = definitions();
    let mut main = main_matrix();
    let mut thumb = Matrix {
        id: "thumb".into(),
        name: None,
        rows: 1,
        columns: 3,
        pitch: Vec2 { x: 19.0, y: 19.0 },
        origin: Vec2 { x: 0.0, y: -19.0 },
        definition_id: "switch".into(),
        part_ids: vec![
            member("thumb", 0, 0),
            member("thumb", 0, 1),
            member("thumb", 0, 2),
        ],
        board_id: Some("board".into()),
        mirror: None,
        rotation: None,
        edge_gap: None,
        diode_direction: None,
        row_layout: vec![],
        column_layout: vec![],
        anchor: Some(MatrixAnchor {
            matrix_id: "main".into(),
            row: 0,
            column: 2,
            follow: all,
            aggregate: None,
        }),
        cells: vec![],
    };
    let mut main2 = main.clone();
    main2.id = "main2".into();
    main2.origin = Vec2 { x: 20.0, y: 0.0 };
    main2.mirror = Some(Mirror::X);
    main2.part_ids = vec![
        member("main2", 0, 0),
        member("main2", 0, 1),
        member("main2", 0, 2),
    ];
    let mut thumb2 = thumb.clone();
    thumb2.id = "thumb2".into();
    thumb2.mirror = Some(Mirror::X);
    thumb2.part_ids = vec![
        member("thumb2", 0, 0),
        member("thumb2", 0, 1),
        member("thumb2", 0, 2),
    ];
    if let Some(anchor) = thumb2.anchor.as_mut() {
        anchor.matrix_id = "main2".into();
    }
    main.part_ids = vec![
        member("main", 0, 0),
        member("main", 0, 1),
        member("main", 0, 2),
    ];
    thumb.part_ids = vec![
        member("thumb", 0, 0),
        member("thumb", 0, 1),
        member("thumb", 0, 2),
    ];
    doc.parts = vec![
        key_part(&member("main", 0, 0), 0.0, 0.0),
        key_part(&member("main", 0, 1), 19.0, 0.0),
        key_part(&member("main", 0, 2), 38.0, 0.0),
        key_part(&member("thumb", 0, 0), 38.0, -19.0),
        key_part(&member("thumb", 0, 1), 57.0, -19.0),
        key_part(&member("thumb", 0, 2), 76.0, -19.0),
        key_part(&member("main2", 0, 0), 20.0, 0.0),
        key_part(&member("main2", 0, 1), 1.0, 0.0),
        key_part(&member("main2", 0, 2), -18.0, 0.0),
        key_part(&member("thumb2", 0, 0), -18.0, -19.0),
        key_part(&member("thumb2", 0, 1), 1.0, -19.0),
        key_part(&member("thumb2", 0, 2), 20.0, -19.0),
    ];
    doc.boards.push(Board {
        id: "board".into(),
        name: "Board".into(),
        outline_ids: vec![],
        part_ids: doc.parts.iter().map(|part| part.id.clone()).collect(),
        net_ids: vec![],
        thickness: 1.6,
        traces: vec![],
        vias: vec![],
    });
    doc.matrices.push(main);
    doc.matrices.push(thumb);
    doc.matrices.push(main2);
    doc.matrices.push(thumb2);
    doc.layouts.push(Layout {
        id: "layout".into(),
        name: "Layout".into(),
        board_id: "board".into(),
        matrix_ids: vec!["main".into(), "thumb".into()],
        part_ids: vec![],
        mirror_link: None,
    });
    doc.layouts.push(Layout {
        id: "mirror-layout".into(),
        name: "Mirror".into(),
        board_id: "board".into(),
        matrix_ids: vec!["main2".into(), "thumb2".into()],
        part_ids: vec![],
        mirror_link: Some(LayoutMirrorLink {
            source_id: "layout".into(),
            axis_x: 10.0,
            split_gap: None,
            spacing: None,
            correspondences: vec![
                MirroredCorrespondence {
                    companion_mode: None,
                    companion_tweak: None,
                    kind: MirroredItemKind::Matrix,
                    source_id: "main".into(),
                    mirror_id: "main2".into(),
                },
                MirroredCorrespondence {
                    companion_mode: None,
                    companion_tweak: None,
                    kind: MirroredItemKind::Matrix,
                    source_id: "thumb".into(),
                    mirror_id: "thumb2".into(),
                },
            ],
            exceptions: vec![],
        }),
    });
    let (engine, _, doc) = open(doc);
    (engine, doc)
}

#[test]
fn linked_detach_clears_both_anchors_and_keeps_both_poses() {
    let (mut engine, doc) = linked_anchored_fixture();
    assert!(anchor_of(&doc, "thumb").is_some());
    assert!(anchor_of(&doc, "thumb2").is_some());
    // Mirror sanity: the partner group is the x=10 reflection of the source.
    assert_eq!(
        pose_of(&doc, &member("thumb2", 0, 0)).at,
        Vec2 { x: -18.0, y: -19.0 }
    );
    let poses_before: Vec<(String, Pose2)> = doc
        .parts
        .iter()
        .map(|part| (part.id.clone(), part.pose))
        .collect();
    let (_, detached) = edit(
        &mut engine,
        &doc,
        EditOperation::DetachMatrixAnchor {
            matrix_id: "thumb".into(),
        },
    );
    // Group-level detach rides the normal pair sync: both anchors clear.
    assert_eq!(anchor_of(&detached, "thumb"), None);
    assert_eq!(anchor_of(&detached, "thumb2"), None);
    // Every materialized pose on both halves is unchanged.
    for (id, pose) in &poses_before {
        assert_eq!(pose_of(&detached, id), *pose, "{id}");
    }
    // One transaction: Undo restores both anchors, Redo clears both again.
    let undone_doc = undone(&mut engine);
    assert_eq!(anchor_of(&undone_doc, "thumb"), anchor_of(&doc, "thumb"));
    assert_eq!(anchor_of(&undone_doc, "thumb2"), anchor_of(&doc, "thumb2"));
    let redone_doc = redone(&mut engine);
    assert_eq!(anchor_of(&redone_doc, "thumb"), None);
    assert_eq!(anchor_of(&redone_doc, "thumb2"), None);
}

fn single_matrix(id: &str, board: &str) -> Matrix {
    Matrix {
        id: id.into(),
        name: None,
        rows: 1,
        columns: 1,
        pitch: Vec2 { x: 19.0, y: 19.0 },
        origin: Vec2::default(),
        definition_id: "switch".into(),
        part_ids: vec![member(id, 0, 0)],
        board_id: Some(board.into()),
        mirror: None,
        rotation: None,
        edge_gap: None,
        diode_direction: None,
        row_layout: vec![],
        column_layout: vec![],
        anchor: None,
        cells: vec![],
    }
}

fn rich_fixture() -> (CoreEngine, ProjectDoc) {
    let mut doc = ProjectDoc::empty("anchors-rich", "Anchors rich");
    doc.definitions = definitions();
    doc.parts = vec![
        key_part(&member("main", 0, 0), 0.0, 0.0),
        key_part(&member("main", 0, 1), 19.0, 0.0),
        key_part(&member("main", 0, 2), 38.0, 0.0),
        key_part(&member("aux", 0, 0), 0.0, 0.0),
        key_part(&member("far", 0, 0), 0.0, 0.0),
    ];
    for (board, ids) in [
        (
            "board",
            vec![
                member("main", 0, 0),
                member("main", 0, 1),
                member("main", 0, 2),
                member("aux", 0, 0),
            ],
        ),
        ("board2", vec![member("far", 0, 0)]),
    ] {
        doc.boards.push(Board {
            id: board.into(),
            name: board.into(),
            outline_ids: vec![],
            part_ids: ids,
            net_ids: vec![],
            thickness: 1.6,
            traces: vec![],
            vias: vec![],
        });
    }
    let mut main = main_matrix();
    main.part_ids = vec![
        member("main", 0, 0),
        member("main", 0, 1),
        member("main", 0, 2),
    ];
    main.board_id = Some("board".into());
    doc.matrices.push(main);
    doc.matrices.push(single_matrix("aux", "board"));
    doc.matrices.push(single_matrix("far", "board2"));
    for (id, board, matrices) in [
        ("layout", "board", vec!["main"]),
        ("other", "board", vec!["aux"]),
        ("far-layout", "board2", vec!["far"]),
    ] {
        doc.layouts.push(Layout {
            id: id.into(),
            name: id.into(),
            board_id: board.into(),
            matrix_ids: matrices.into_iter().map(str::to_string).collect(),
            part_ids: vec![],
            mirror_link: None,
        });
    }
    let (engine, _, doc) = open(doc);
    (engine, doc)
}

fn anchored_rich_fixture() -> (CoreEngine, ProjectDoc) {
    let (mut engine, doc) = rich_fixture();
    let doc = add_thumb(&mut engine, &doc);
    (engine, doc)
}

#[test]
fn anchor_graph_refusals_leave_state_unchanged() {
    let (mut engine, doc) = anchored_rich_fixture();
    let all = || Follow {
        x: true,
        y: true,
        rotation: true,
    };
    let anchor = |matrix: &str, row: u32, column: u32| MatrixAnchor {
        matrix_id: matrix.into(),
        row,
        column,
        follow: all(),
        aggregate: None,
    };
    let cases: Vec<(&str, EditOperation, &str)> = vec![
        (
            "self",
            EditOperation::SetMatrixAnchor {
                matrix_id: "thumb".into(),
                anchor: anchor("thumb", 0, 0),
            },
            "cannot anchor to itself",
        ),
        (
            "cycle",
            EditOperation::SetMatrixAnchor {
                matrix_id: "main".into(),
                anchor: anchor("thumb", 0, 0),
            },
            "anchor cycle",
        ),
        (
            "missing-target",
            EditOperation::SetMatrixAnchor {
                matrix_id: "thumb".into(),
                anchor: anchor("ghost", 0, 0),
            },
            "anchor target ghost is missing",
        ),
        (
            "out-of-range",
            EditOperation::SetMatrixAnchor {
                matrix_id: "thumb".into(),
                anchor: anchor("main", 0, 9),
            },
            "anchor key matrix/main/r0c9 is missing",
        ),
        (
            "cross-layout",
            EditOperation::SetMatrixAnchor {
                matrix_id: "thumb".into(),
                anchor: anchor("aux", 0, 0),
            },
            "same board and Layout",
        ),
        (
            "cross-board",
            EditOperation::SetMatrixAnchor {
                matrix_id: "thumb".into(),
                anchor: anchor("far", 0, 0),
            },
            "same board and Layout",
        ),
        (
            "duplicate-id",
            EditOperation::AddAnchoredMatrix {
                id: "thumb".into(),
                anchor: anchor("main", 0, 0),
            },
            "Duplicate matrix identity",
        ),
        (
            "bad-id",
            EditOperation::AddAnchoredMatrix {
                id: "bad id".into(),
                anchor: anchor("main", 0, 0),
            },
            "letters, digits",
        ),
        (
            "missing-matrix",
            EditOperation::SetMatrixAnchor {
                matrix_id: "ghost".into(),
                anchor: anchor("main", 0, 0),
            },
            "Matrix not found",
        ),
        (
            "detach-unanchored",
            EditOperation::DetachMatrixAnchor {
                matrix_id: "main".into(),
            },
            "has no anchor",
        ),
        (
            "constrain-anchor-key",
            EditOperation::SetConstraint {
                constraint: Constraint::Offset {
                    id: "drive".into(),
                    source_part_id: member("main", 0, 0),
                    target_part_id: member("main", 0, 2),
                    offset: Vec2 { x: 19.0, y: 0.0 },
                    rotation: 0.0,
                },
            },
            "anchors to it",
        ),
    ];
    for (name, operation, reason) in cases {
        let message = error(&mut engine, &doc, operation);
        assert!(
            message.contains(reason),
            "{name}: expected refusal containing {reason:?}, got {message:?}"
        );
        assert_eq!(snapshot(&mut engine), doc, "{name}: refusal changed state");
    }
}

#[test]
fn anchoring_to_a_constraint_driven_key_is_refused() {
    let (mut engine, doc) = rich_fixture();
    let (_, doc) = edit(
        &mut engine,
        &doc,
        EditOperation::SetConstraint {
            constraint: Constraint::Offset {
                id: "drive".into(),
                source_part_id: member("main", 0, 1),
                target_part_id: member("main", 0, 0),
                offset: Vec2 { x: -19.0, y: 0.0 },
                rotation: 0.0,
            },
        },
    );
    let message = error(
        &mut engine,
        &doc,
        EditOperation::AddAnchoredMatrix {
            id: "thumb".into(),
            anchor: MatrixAnchor {
                matrix_id: "main".into(),
                row: 0,
                column: 0,
                follow: Follow {
                    x: true,
                    y: true,
                    rotation: true,
                },
                aggregate: None,
            },
        },
    );
    assert!(
        message.contains("constraint-driven"),
        "unexpected message: {message:?}"
    );
    assert_eq!(snapshot(&mut engine), doc);
}

#[test]
// Ticket 20: Adding to a linked group now creates the mirrored counterpart
// in the same edit. The name is retained for the native baseline so the new
// `matrix_anchors` names stay purely additive.
fn adding_to_a_linked_layout_is_refused_until_ticket_20() {
    let (mut engine, doc) = fixture();
    let (_, doc) = edit(
        &mut engine,
        &doc,
        EditOperation::CreateMirroredPair {
            left: Layout {
                id: "left-layout".into(),
                name: "Left half".into(),
                board_id: "board".into(),
                matrix_ids: vec!["left-matrix".into()],
                part_ids: vec![],
                mirror_link: None,
            },
            right: Layout {
                id: "right-layout".into(),
                name: "Right half".into(),
                board_id: "board".into(),
                matrix_ids: vec!["right-matrix".into()],
                part_ids: vec![],
                mirror_link: Some(LayoutMirrorLink {
                    source_id: "left-layout".into(),
                    axis_x: 10.0,
                    split_gap: None,
                    spacing: None,
                    correspondences: vec![],
                    exceptions: vec![],
                }),
            },
            matrix: Box::new(Matrix {
                id: "left-matrix".into(),
                name: None,
                rows: 1,
                columns: 1,
                pitch: Vec2 { x: 19.0, y: 19.0 },
                origin: Vec2::default(),
                definition_id: "switch".into(),
                part_ids: vec![],
                board_id: Some("board".into()),
                mirror: None,
                rotation: None,
                edge_gap: None,
                diode_direction: None,
                row_layout: vec![],
                column_layout: vec![],
                anchor: None,
                cells: vec![],
            }),
            definitions: None,
        },
    );
    let (_, doc) = edit(
        &mut engine,
        &doc,
        EditOperation::AddAnchoredMatrix {
            id: "claw".into(),
            anchor: MatrixAnchor {
                matrix_id: "left-matrix".into(),
                row: 0,
                column: 0,
                follow: Follow {
                    x: true,
                    y: true,
                    rotation: true,
                },
                aggregate: None,
            },
        },
    );
    // Both halves gain a 1x3 thumb; the mirror anchors to the counterpart.
    let claw = doc
        .matrices
        .iter()
        .find(|matrix| matrix.id == "claw")
        .expect("claw matrix");
    let mirror = doc
        .matrices
        .iter()
        .find(|matrix| matrix.id == "claw-mirror")
        .expect("claw mirror");
    assert_eq!(
        claw.anchor.as_ref().expect("claw anchor").matrix_id,
        "left-matrix"
    );
    assert_eq!(
        mirror.anchor.as_ref().expect("mirror anchor").matrix_id,
        "right-matrix"
    );
    // Relative Origin X negates while Y stays; the default (0,-19) is fixed.
    assert_eq!(claw.origin, Vec2 { x: 0.0, y: -19.0 });
    assert_eq!(mirror.origin, Vec2 { x: -0.0, y: -19.0 });
    // Partner poses are the x=10 reflection of the source.
    assert_eq!(
        pose_of(&doc, &member("claw", 0, 0)).at,
        Vec2 { x: 0.0, y: -19.0 }
    );
    assert_eq!(
        pose_of(&doc, &member("claw-mirror", 0, 0)).at,
        Vec2 { x: 20.0, y: -19.0 }
    );
    // Layout membership and matrix correspondences cover both.
    let left = doc
        .layouts
        .iter()
        .find(|layout| layout.id == "left-layout")
        .unwrap();
    let right = doc
        .layouts
        .iter()
        .find(|layout| layout.id == "right-layout")
        .unwrap();
    assert!(left.matrix_ids.contains(&"claw".to_string()));
    assert!(right.matrix_ids.contains(&"claw-mirror".to_string()));
    let link = right.mirror_link.as_ref().expect("mirror link");
    assert!(link.correspondences.iter().any(|record| {
        record.kind == MirroredItemKind::Matrix
            && record.source_id == "claw"
            && record.mirror_id == "claw-mirror"
    }));
    // One Undo removes both, Redo restores both.
    let undone_doc = undone(&mut engine);
    assert!(undone_doc.matrices.iter().all(|matrix| matrix.id != "claw"));
    assert!(
        undone_doc
            .matrices
            .iter()
            .all(|matrix| matrix.id != "claw-mirror")
    );
    let redone_doc = redone(&mut engine);
    assert!(redone_doc.matrices.iter().any(|matrix| matrix.id == "claw"));
    assert!(
        redone_doc
            .matrices
            .iter()
            .any(|matrix| matrix.id == "claw-mirror")
    );
}

fn set_follow(
    engine: &mut CoreEngine,
    doc: &ProjectDoc,
    x: bool,
    y: bool,
    rotation: bool,
) -> ProjectDoc {
    edit(
        engine,
        doc,
        EditOperation::SetMatrixAnchor {
            matrix_id: "thumb".into(),
            anchor: MatrixAnchor {
                matrix_id: "main".into(),
                row: 0,
                column: 2,
                follow: Follow { x, y, rotation },
                aggregate: None,
            },
        },
    )
    .1
}

#[test]
fn anchor_follow_covers_every_subset_with_zeroing() {
    // Each subset keeps exactly its followed components of the anchor motion;
    // unfollowed components read world zero, even for the relative origin.
    for (x, y, rotation) in [
        (false, false, false),
        (true, false, false),
        (false, true, false),
        (false, false, true),
        (true, true, false),
        (true, false, true),
        (false, true, true),
        (true, true, true),
    ] {
        let (mut engine, doc) = anchored_fixture();
        let doc = set_follow(&mut engine, &doc, x, y, rotation);
        let (_, moved) = edit(
            &mut engine,
            &doc,
            EditOperation::MoveParts {
                positions: vec![Position {
                    id: member("main", 0, 2),
                    at: Vec2 { x: 43.0, y: 7.0 },
                }],
            },
        );
        let expected = Vec2 {
            x: if x { 43.0 } else { 0.0 },
            y: if y { 7.0 - 19.0 } else { 0.0 },
        };
        assert_eq!(
            pose_of(&moved, &member("thumb", 0, 0)).at,
            expected,
            "follow x={x} y={y} rotation={rotation}"
        );
        assert_eq!(
            pose_of(&moved, &member("thumb", 0, 0)).rotation,
            0.0,
            "follow x={x} y={y} rotation={rotation}"
        );
    }
}

#[test]
fn anchor_follow_applies_origin_and_rotation_before_filtering() {
    let (mut engine, doc) = anchored_fixture();
    // Widen the relative origin so the order is observable.
    let mut widened = doc
        .matrices
        .iter()
        .find(|matrix| matrix.id == "thumb")
        .expect("thumb matrix")
        .clone();
    widened.origin = Vec2 { x: 10.0, y: -19.0 };
    let (_, doc) = edit(
        &mut engine,
        &doc,
        EditOperation::SetMatrix {
            matrix: widened,
            definitions: None,
        },
    );
    assert_eq!(
        pose_of(&doc, &member("thumb", 0, 0)).at,
        Vec2 { x: 48.0, y: -19.0 }
    );
    // Dropping Follow X zeroes the whole candidate X, including the relative
    // origin: Origin runs before Follow, not after it.
    let doc = set_follow(&mut engine, &doc, false, true, true);
    assert_eq!(
        pose_of(&doc, &member("thumb", 0, 0)).at,
        Vec2 { x: 0.0, y: -19.0 }
    );
    assert_eq!(
        pose_of(&doc, &member("thumb", 0, 2)).at,
        Vec2 { x: 38.0, y: -19.0 }
    );
}

#[test]
fn anchor_follow_rotation_filters_the_completed_heading() {
    let (mut engine, doc) = anchored_fixture();
    let mut turned = doc
        .matrices
        .iter()
        .find(|matrix| matrix.id == "main")
        .expect("main matrix")
        .clone();
    turned.rotation = Some(30.0);
    let (_, doc) = edit(
        &mut engine,
        &doc,
        EditOperation::SetMatrix {
            matrix: turned,
            definitions: None,
        },
    );
    let followed = pose_of(&doc, &member("thumb", 0, 0));
    assert_eq!(followed.rotation, 30.0);
    let cos30 = 30f64.to_radians().cos();
    let sin30 = 30f64.to_radians().sin();
    near(
        followed.at,
        38.0 * cos30 + 19.0 * sin30,
        38.0 * sin30 - 19.0 * cos30,
    );
    // Without Follow rotation the heading zeroes but the rotated candidate
    // position stands.
    let doc = set_follow(&mut engine, &doc, true, true, false);
    let unfollowed = pose_of(&doc, &member("thumb", 0, 0));
    assert_eq!(unfollowed.rotation, 0.0);
    assert_eq!(unfollowed.at, followed.at);
}

#[test]
fn anchored_matrix_translation_respects_follow_and_refuses_impossible_drags() {
    let (mut engine, doc) = anchored_fixture();
    let doc = set_follow(&mut engine, &doc, false, true, true);
    // A drag along the unfollowed X cannot be represented: refuse atomically.
    let message = error(
        &mut engine,
        &doc,
        EditOperation::SetMatrixPlacement {
            matrix_id: "thumb".into(),
            change: MatrixPlacementChange::MatrixTranslation {
                delta: Vec2 { x: 5.0, y: 0.0 },
            },
        },
    );
    assert!(
        message.contains("Follow"),
        "unexpected message: {message:?}"
    );
    assert_eq!(snapshot(&mut engine), doc);
    // A drag along the followed Y succeeds and moves every member.
    let (_, moved) = edit(
        &mut engine,
        &doc,
        EditOperation::SetMatrixPlacement {
            matrix_id: "thumb".into(),
            change: MatrixPlacementChange::MatrixTranslation {
                delta: Vec2 { x: 0.0, y: 5.0 },
            },
        },
    );
    assert_eq!(
        pose_of(&moved, &member("thumb", 0, 0)).at,
        Vec2 { x: 0.0, y: -14.0 }
    );
    assert_eq!(
        pose_of(&moved, &member("thumb", 0, 2)).at,
        Vec2 { x: 38.0, y: -14.0 }
    );
}

#[test]
fn anchored_frame_edits_refuse_unfollowed_components() {
    let (mut engine, doc) = anchored_fixture();
    let doc = set_follow(&mut engine, &doc, true, true, false);
    let mut rotated = doc
        .matrices
        .iter()
        .find(|matrix| matrix.id == "thumb")
        .expect("thumb matrix")
        .clone();
    rotated.rotation = Some(15.0);
    let message = error(
        &mut engine,
        &doc,
        EditOperation::SetMatrix {
            matrix: rotated,
            definitions: None,
        },
    );
    assert!(
        message.contains("not followed"),
        "unexpected message: {message:?}"
    );
    assert_eq!(snapshot(&mut engine), doc);
}

#[test]
fn compaction_remaps_anchors_onto_the_surviving_key() {
    let (mut engine, doc) = anchored_fixture();
    let (_, compacted) = edit(
        &mut engine,
        &doc,
        EditOperation::RemoveMatrixColumn {
            matrix_id: "main".into(),
            column: 0,
        },
    );
    // The anchor followed its physical key from (0,2) to (0,1), never
    // retargeting the new key at the old coordinate.
    assert_eq!(
        anchor_of(&compacted, "thumb"),
        Some(MatrixAnchor {
            matrix_id: "main".into(),
            row: 0,
            column: 1,
            follow: Follow {
                x: true,
                y: true,
                rotation: true,
            },
            aggregate: None,
        })
    );
    assert_eq!(
        pose_of(&compacted, &member("thumb", 0, 0)).at,
        Vec2 { x: 19.0, y: -19.0 }
    );
    assert_eq!(
        pose_of(&compacted, &member("thumb", 0, 2)).at,
        Vec2 { x: 57.0, y: -19.0 }
    );
}

#[test]
fn removing_the_anchor_slot_detaches_dependants_with_their_frame() {
    let (mut engine, doc) = anchored_fixture();
    let poses_before: Vec<(String, Pose2)> = doc
        .parts
        .iter()
        .map(|part| (part.id.clone(), part.pose))
        .collect();
    let (_, detached) = edit(
        &mut engine,
        &doc,
        EditOperation::RemoveMatrixColumn {
            matrix_id: "main".into(),
            column: 2,
        },
    );
    assert_eq!(anchor_of(&detached, "thumb"), None);
    let thumb = detached
        .matrices
        .iter()
        .find(|matrix| matrix.id == "thumb")
        .expect("thumb matrix");
    assert_eq!(thumb.origin, Vec2 { x: 38.0, y: -19.0 });
    for (id, pose) in &poses_before {
        if id.starts_with("matrix/thumb/") {
            assert_eq!(pose_of(&detached, id), *pose, "{id}");
        }
    }
}

#[test]
fn removing_the_anchor_matrix_detaches_dependants() {
    let (mut engine, doc) = anchored_fixture();
    let poses_before: Vec<(String, Pose2)> = doc
        .parts
        .iter()
        .map(|part| (part.id.clone(), part.pose))
        .collect();
    let (_, detached) = edit(
        &mut engine,
        &doc,
        EditOperation::RemoveMatrix { id: "main".into() },
    );
    assert_eq!(anchor_of(&detached, "thumb"), None);
    assert!(
        detached
            .layouts
            .iter()
            .find(|layout| layout.id == "layout")
            .is_some_and(|layout| layout.matrix_ids == vec!["thumb".to_string()])
    );
    for (id, pose) in &poses_before {
        if id.starts_with("matrix/thumb/") {
            assert_eq!(pose_of(&detached, id), *pose, "{id}");
        }
    }
}

#[test]
fn shrinking_through_set_matrix_detaches_out_of_range_anchors() {
    let (mut engine, doc) = anchored_fixture();
    let mut shrunk = doc
        .matrices
        .iter()
        .find(|matrix| matrix.id == "main")
        .expect("main matrix")
        .clone();
    shrunk.columns = 2;
    let (_, detached) = edit(
        &mut engine,
        &doc,
        EditOperation::SetMatrix {
            matrix: shrunk,
            definitions: None,
        },
    );
    assert_eq!(anchor_of(&detached, "thumb"), None);
    assert_eq!(
        pose_of(&detached, &member("thumb", 0, 0)).at,
        Vec2 { x: 38.0, y: -19.0 }
    );
}

#[test]
fn disabled_and_deleted_anchor_keys_keep_anchoring() {
    for (name, operation) in [
        (
            "disabled",
            EditOperation::SetMatrixCellsEnabled {
                cells: [(String::from("main"), vec![(0, 2)])].into_iter().collect(),
                enabled: false,
            },
        ),
        (
            "deleted",
            EditOperation::RemoveMatrixCells {
                matrix_id: "main".into(),
                cells: vec![(0, 2)],
            },
        ),
    ] {
        let (mut engine, doc) = anchored_fixture();
        let (_, edited) = edit(&mut engine, &doc, operation);
        assert!(
            anchor_of(&edited, "thumb").is_some(),
            "{name}: anchor was dropped"
        );
        assert_eq!(
            pose_of(&edited, &member("thumb", 0, 0)).at,
            Vec2 { x: 38.0, y: -19.0 },
            "{name}"
        );
        assert!(
            edited
                .parts
                .iter()
                .all(|part| part.id != member("main", 0, 2)),
            "{name}: invisible key was materialized"
        );
    }
}

#[test]
fn constraints_consume_final_anchored_poses() {
    let (mut engine, doc) = anchored_fixture();
    let (_, doc) = edit(
        &mut engine,
        &doc,
        EditOperation::SetConstraint {
            constraint: Constraint::Offset {
                id: "ride".into(),
                source_part_id: member("main", 0, 0),
                target_part_id: member("thumb", 0, 1),
                offset: Vec2 { x: 5.0, y: 5.0 },
                rotation: 0.0,
            },
        },
    );
    // The anchored member solves from its laid-out pose, not from a stale one.
    assert_eq!(
        pose_of(&doc, &member("thumb", 0, 1)).at,
        Vec2 { x: 5.0, y: 5.0 }
    );
}

#[test]
fn column_align_moves_through_the_actual_anchor_frame() {
    let (mut engine, doc) = anchored_fixture();
    let (_, moved) = edit(
        &mut engine,
        &doc,
        EditOperation::SetMatrixPlacement {
            matrix_id: "thumb".into(),
            change: MatrixPlacementChange::ColumnTranslation {
                column: 1,
                delta: Vec2 { x: 0.0, y: 5.0 },
            },
        },
    );
    assert_eq!(
        pose_of(&moved, &member("thumb", 0, 1)).at,
        Vec2 { x: 57.0, y: -14.0 }
    );
    assert_eq!(
        pose_of(&moved, &member("thumb", 0, 0)).at,
        Vec2 { x: 38.0, y: -19.0 }
    );
    assert_eq!(
        pose_of(&moved, &member("thumb", 0, 2)).at,
        Vec2 { x: 76.0, y: -19.0 }
    );
}

#[test]
fn linked_follow_transports_source_basis_with_non_origin_axis() {
    // Axis x=10 is not the origin: Follow X off keeps source X=0 while the
    // partner keeps the transported X=20, not an independent zero.
    let (mut engine, doc) = linked_anchored_fixture();
    // Drop Follow X on the source thumb; sync carries the subset to the mirror.
    let (_, doc) = edit(
        &mut engine,
        &doc,
        EditOperation::SetMatrixAnchor {
            matrix_id: "thumb".into(),
            anchor: MatrixAnchor {
                matrix_id: "main".into(),
                row: 0,
                column: 2,
                follow: Follow {
                    x: false,
                    y: true,
                    rotation: true,
                },
                aggregate: None,
            },
        },
    );
    assert!(
        !doc.matrices
            .iter()
            .find(|matrix| matrix.id == "thumb2")
            .and_then(|matrix| matrix.anchor.clone())
            .expect("thumb2 anchor")
            .follow
            .x
    );
    let (_, moved) = edit(
        &mut engine,
        &doc,
        EditOperation::MoveParts {
            positions: vec![Position {
                id: member("main", 0, 2),
                at: Vec2 { x: 43.0, y: 7.0 },
            }],
        },
    );
    // Source keeps world-zero X, partner keeps the transported axis image.
    assert_eq!(
        pose_of(&moved, &member("thumb", 0, 0)).at,
        Vec2 { x: 0.0, y: -12.0 }
    );
    assert_eq!(
        pose_of(&moved, &member("thumb2", 0, 0)).at,
        Vec2 { x: 20.0, y: -12.0 }
    );
    // Preview leaves the document unchanged and Undo restores one entry.
    let before = snapshot(&mut engine);
    let _ = preview(
        &mut engine,
        &moved,
        EditOperation::MoveParts {
            positions: vec![Position {
                id: member("main", 0, 2),
                at: Vec2 { x: 44.0, y: 8.0 },
            }],
        },
    );
    assert_eq!(snapshot(&mut engine), before);
}

#[test]
fn linked_anchor_retarget_maps_by_identity_not_list_position() {
    let (_, doc) = linked_anchored_fixture();
    // Reorder both the stored matrices and the Layout membership lists; the
    // group must still resolve by identity and physical key.
    let mut reordered = doc.clone();
    reordered.matrices.reverse();
    for layout in &mut reordered.layouts {
        layout.matrix_ids.reverse();
    }
    let (mut engine, _, doc) = open(reordered);
    let (_, doc) = edit(
        &mut engine,
        &doc,
        EditOperation::SetMatrixAnchor {
            matrix_id: "thumb".into(),
            anchor: MatrixAnchor {
                matrix_id: "main".into(),
                row: 0,
                column: 0,
                follow: Follow {
                    x: true,
                    y: true,
                    rotation: true,
                },
                aggregate: None,
            },
        },
    );
    // Mirror follows via the matrix correspondence, not the stale anchor.
    assert_eq!(
        anchor_of(&doc, "thumb2").expect("thumb2 anchor").matrix_id,
        "main2"
    );
    assert_eq!(anchor_of(&doc, "thumb2").expect("thumb2 anchor").row, 0);
    assert_eq!(anchor_of(&doc, "thumb2").expect("thumb2 anchor").column, 0);
    // Poses track the new anchor keys on both halves.
    assert_eq!(
        pose_of(&doc, &member("thumb", 0, 0)).at,
        Vec2 { x: 0.0, y: -19.0 }
    );
    // main2(0,0) mirrors main(0,0) about x=10: 2*10-0=20.
    assert_eq!(
        pose_of(&doc, &member("thumb2", 0, 0)).at,
        Vec2 { x: 20.0, y: -19.0 }
    );
}

#[test]
fn linked_mirror_originating_shift_bend_reflects_heading_once() {
    // Edit from the mirror half: stored layout values copy unchanged while
    // the kernel negates the whole local heading exactly once.
    let (mut engine, doc) = linked_anchored_fixture();
    let (_, doc) = edit(
        &mut engine,
        &doc,
        EditOperation::SetMatrixPlacement {
            matrix_id: "main2".into(),
            change: MatrixPlacementChange::RowShift {
                row: 0,
                value: Vec2 { x: 5.0, y: 3.0 },
            },
        },
    );
    let source_shift = doc
        .matrices
        .iter()
        .find(|matrix| matrix.id == "main")
        .and_then(|matrix| matrix.row_layout.first().map(|record| record.shift))
        .unwrap_or_default();
    let mirror_shift = doc
        .matrices
        .iter()
        .find(|matrix| matrix.id == "main2")
        .and_then(|matrix| matrix.row_layout.first().map(|record| record.shift))
        .unwrap_or_default();
    // Stored values copy unchanged; the mirror does not negate them twice.
    assert_eq!(source_shift, Vec2 { x: 5.0, y: 3.0 });
    assert_eq!(mirror_shift, Vec2 { x: 5.0, y: 3.0 });
    // World poses are exact reflections about x=10 (sum 20), not copies.
    let source_pose = pose_of(&doc, &member("main", 0, 1));
    let mirror_pose = pose_of(&doc, &member("main2", 0, 1));
    assert!(
        (source_pose.at.x + mirror_pose.at.x - 20.0).abs() < 1e-9
            && (source_pose.at.y - mirror_pose.at.y).abs() < 1e-9,
        "expected reflection, got {source_pose:?} vs {mirror_pose:?}"
    );
    // Key Bend from the mirror half copies too, with single heading negation.
    let (_, doc) = edit(
        &mut engine,
        &doc,
        EditOperation::SetMatrixPlacement {
            matrix_id: "main2".into(),
            change: MatrixPlacementChange::KeyBend {
                row: 0,
                column: 1,
                value: 15.0,
            },
        },
    );
    let source_bend = doc
        .matrices
        .iter()
        .find(|matrix| matrix.id == "main")
        .and_then(|matrix| {
            matrix
                .cells
                .iter()
                .find(|cell| cell.row == 0 && cell.column == 1)
                .and_then(|cell| cell.bend)
        })
        .unwrap_or_default();
    let mirror_bend = doc
        .matrices
        .iter()
        .find(|matrix| matrix.id == "main2")
        .and_then(|matrix| {
            matrix
                .cells
                .iter()
                .find(|cell| cell.row == 0 && cell.column == 1)
                .and_then(|cell| cell.bend)
        })
        .unwrap_or_default();
    assert_eq!(source_bend, 15.0);
    assert_eq!(mirror_bend, 15.0);
    // Headings are opposite-signed (single negation), not equal or doubled.
    let source_pose = pose_of(&doc, &member("main", 0, 1));
    let mirror_pose = pose_of(&doc, &member("main2", 0, 1));
    assert!(
        (source_pose.rotation + mirror_pose.rotation).abs() < 1e-9,
        "expected opposite headings, got {source_pose:?} vs {mirror_pose:?}"
    );
}

#[test]
fn linked_dependent_follows_actual_local_anchor() {
    // The dependent keeps its own linked parameters but follows the actual
    // Local anchor key pose, not a substitute invisible pose.
    let (mut engine, doc) = linked_anchored_fixture();
    // Make the mirror anchor key Local via its Layout link.
    let mut mirror_layout = doc
        .layouts
        .iter()
        .find(|layout| layout.id == "mirror-layout")
        .expect("mirror layout")
        .clone();
    let link = mirror_layout.mirror_link.as_mut().expect("mirror link");
    link.exceptions.push(MirroredException {
        kind: MirroredItemKind::Part,
        mirror_id: member("main2", 0, 2),
        state: MirroredExceptionState::Local,
        companion_mode: None,
        companion_tweak: None,
        removed_copper: None,
    });
    let (_, doc) = edit(
        &mut engine,
        &doc,
        EditOperation::SetLayout {
            layout: mirror_layout,
        },
    );
    assert!(
        doc.layouts
            .iter()
            .find(|layout| layout.id == "mirror-layout")
            .and_then(|layout| layout.mirror_link.as_ref())
            .is_some_and(|link| link
                .exceptions
                .iter()
                .any(|exception| exception.mirror_id == member("main2", 0, 2)
                    && exception.state == MirroredExceptionState::Local))
    );
    // Move the actual Local anchor key independently from the mirror side.
    let (_, moved) = edit(
        &mut engine,
        &doc,
        EditOperation::MoveParts {
            positions: vec![Position {
                id: member("main2", 0, 2),
                at: Vec2 { x: -10.0, y: 5.0 },
            }],
        },
    );
    // Source stays where it was; the mirror anchor moved locally.
    assert_eq!(
        pose_of(&moved, &member("main", 0, 2)).at,
        Vec2 { x: 38.0, y: 0.0 }
    );
    assert_eq!(
        pose_of(&moved, &member("main2", 0, 2)).at,
        Vec2 { x: -10.0, y: 5.0 }
    );
    // The mirror dependent follows its actual Local anchor; its own layout
    // values stay linked (empty, like the source thumb).
    assert_eq!(
        pose_of(&moved, &member("thumb2", 0, 0)).at,
        Vec2 { x: -10.0, y: -14.0 }
    );
    let thumb = moved
        .matrices
        .iter()
        .find(|matrix| matrix.id == "thumb")
        .unwrap();
    let thumb2 = moved
        .matrices
        .iter()
        .find(|matrix| matrix.id == "thumb2")
        .unwrap();
    assert_eq!(thumb.row_layout, thumb2.row_layout);
    assert_eq!(thumb.column_layout, thumb2.column_layout);
}

#[test]
fn linked_column_zero_hold_still_reflects_compensation_once() {
    // Column-0 hold-still writes matrix Origin; the follower gets the
    // reflected Origin once, and both halves hold their keys still.
    let (mut engine, doc) = linked_anchored_fixture();
    // Give column 0 a splay so the origin edit has something to hold still.
    let (_, doc) = edit(
        &mut engine,
        &doc,
        EditOperation::SetMatrixSplay {
            matrix_id: "main".into(),
            column: 1,
            change: MatrixSplayChange::Angle {
                angle: 15.0,
                affect: MatrixSplayAffect::Following,
            },
        },
    );
    let before: Vec<(String, Pose2)> = doc
        .parts
        .iter()
        .map(|part| (part.id.clone(), part.pose))
        .collect();
    // Hold-still origin edit from the mirror half.
    let (_, moved) = edit(
        &mut engine,
        &doc,
        EditOperation::SetMatrixSplay {
            matrix_id: "main2".into(),
            column: 0,
            change: MatrixSplayChange::OriginRelative {
                value: Vec2 { x: 2.0, y: 1.0 },
            },
        },
    );
    // Stored relative origins copy unchanged; Origins are point reflections.
    let source_origin = moved
        .matrices
        .iter()
        .find(|matrix| matrix.id == "main")
        .unwrap()
        .column_layout
        .first()
        .map(|record| record.splay_origin)
        .unwrap_or_default();
    let mirror_origin = moved
        .matrices
        .iter()
        .find(|matrix| matrix.id == "main2")
        .unwrap()
        .column_layout
        .first()
        .map(|record| record.splay_origin)
        .unwrap_or_default();
    assert_eq!(source_origin, Vec2 { x: 2.0, y: 1.0 });
    assert_eq!(mirror_origin, Vec2 { x: 2.0, y: 1.0 });
    // Keys held still: every pre-existing pose is unchanged within tolerance.
    for (id, pose) in &before {
        let after = pose_of(&moved, id);
        assert!(
            (after.at.x - pose.at.x).abs() < 1e-9 && (after.at.y - pose.at.y).abs() < 1e-9,
            "{id} moved: {pose:?} vs {after:?}"
        );
    }
}

#[test]
fn linked_omitted_anchor_keeps_live_logical_pose() {
    // Omitted mirrors suppress inherited parts but keep a live logical pose
    // for anchors, without freezing or detaching. Open with the exception
    // from the start (removal lifecycle belongs to ticket 21).
    let (_, doc) = linked_anchored_fixture();
    let mut omitted = doc.clone();
    // Drop the omitted matrix's materialized parts; Open must not recreate
    // them, but anchors through it stay live.
    omitted
        .parts
        .retain(|part| !part.id.starts_with("matrix/main2/"));
    for board in &mut omitted.boards {
        board.part_ids.retain(|id| !id.starts_with("matrix/main2/"));
    }
    if let Some(layout) = omitted
        .layouts
        .iter_mut()
        .find(|layout| layout.id == "mirror-layout")
        && let Some(link) = layout.mirror_link.as_mut()
    {
        link.exceptions.push(MirroredException {
            kind: MirroredItemKind::Matrix,
            mirror_id: "main2".into(),
            state: MirroredExceptionState::Omitted,
            companion_mode: None,
            companion_tweak: None,
            removed_copper: None,
        });
    }
    let (_, _, doc) = open(omitted);
    // No materialized parts for the omitted anchor target.
    assert!(
        doc.parts
            .iter()
            .all(|part| part.id != member("main2", 0, 2))
    );
    // Both dependants still evaluate through the live logical anchor: source
    // thumb at its anchor key plus relative origin, mirror thumb at the
    // reflected logical pose (axis x=10).
    assert_eq!(
        pose_of(&doc, &member("thumb", 0, 0)).at,
        Vec2 { x: 38.0, y: -19.0 }
    );
    assert_eq!(
        pose_of(&doc, &member("thumb2", 0, 0)).at,
        Vec2 { x: -18.0, y: -19.0 }
    );
}

#[test]
fn linked_y_mirror_change_is_refused() {
    // Ticket 20 preserved refusal: a horizontal local frame needs an
    // independent layout. Proves the `layout.rs::reflected` guard via public
    // Edit; GREEN on current code is characterization (RED without the guard
    // at layout.rs:381-382 is by code-path inspection only: `reflected`
    // would otherwise accept Y and silently map the partner to X).
    let (mut engine, doc) = linked_anchored_fixture();
    for matrix_id in ["thumb", "thumb2"] {
        let mut attempted = doc
            .matrices
            .iter()
            .find(|matrix| matrix.id == matrix_id)
            .expect("linked matrix")
            .clone();
        attempted.mirror = Some(Mirror::Y);
        let message = error(
            &mut engine,
            &doc,
            EditOperation::SetMatrix {
                matrix: attempted,
                definitions: None,
            },
        );
        assert!(
            message.contains("Unlink the halves before changing to a horizontal matrix mirror"),
            "{matrix_id}: unexpected message: {message:?}"
        );
        assert_eq!(
            snapshot(&mut engine),
            doc,
            "{matrix_id}: refusal changed state"
        );
    }
    // Narrow guard: the ordinary None/X toggle still succeeds on the linked
    // source (claw-mirror toggle), so the refusal is Y-specific.
    let mut toggled = doc
        .matrices
        .iter()
        .find(|matrix| matrix.id == "thumb")
        .expect("thumb matrix")
        .clone();
    toggled.mirror = Some(Mirror::X);
    let (_, changed) = edit(
        &mut engine,
        &doc,
        EditOperation::SetMatrix {
            matrix: toggled,
            definitions: None,
        },
    );
    assert_eq!(
        changed
            .matrices
            .iter()
            .find(|matrix| matrix.id == "thumb")
            .and_then(|matrix| matrix.mirror),
        Some(Mirror::X)
    );
}

#[test]
fn linked_resize_keeps_local_partner_dimensions_and_poses() {
    // Ticket 21 first slice: a public source resize must not rewrite a Local
    // partner matrix. Source `main` grows 3 -> 4 columns; Local `main2` keeps
    // its own 1x3 dimensions and every world pose, with valid correspondences.
    let (mut engine, doc) = linked_anchored_fixture();
    // Make the mirror matrix Local via its Layout link (public route).
    let mut mirror_layout = doc
        .layouts
        .iter()
        .find(|layout| layout.id == "mirror-layout")
        .expect("mirror layout")
        .clone();
    mirror_layout
        .mirror_link
        .as_mut()
        .expect("mirror link")
        .exceptions
        .push(MirroredException {
            kind: MirroredItemKind::Matrix,
            mirror_id: "main2".into(),
            state: MirroredExceptionState::Local,
            companion_mode: None,
            companion_tweak: None,
            removed_copper: None,
        });
    let (_, doc) = edit(
        &mut engine,
        &doc,
        EditOperation::SetLayout {
            layout: mirror_layout,
        },
    );
    let poses_before: Vec<(String, Pose2)> = doc
        .parts
        .iter()
        .map(|part| (part.id.clone(), part.pose))
        .collect();
    // Grow the source through the public prepared route.
    let mut grown = doc
        .matrices
        .iter()
        .find(|matrix| matrix.id == "main")
        .expect("main matrix")
        .clone();
    grown.columns = 4;
    let (_, resized) = edit(
        &mut engine,
        &doc,
        EditOperation::SetMatrix {
            matrix: grown,
            definitions: None,
        },
    );
    // The source grew; the Local partner kept its own dimensions.
    assert_eq!(
        resized
            .matrices
            .iter()
            .find(|matrix| matrix.id == "main")
            .expect("grown source")
            .columns,
        4
    );
    let partner = resized
        .matrices
        .iter()
        .find(|matrix| matrix.id == "main2")
        .expect("local partner");
    assert_eq!(partner.rows, 1);
    assert_eq!(partner.columns, 3);
    // Every pre-existing pose is unchanged, including the Local partner's.
    for (id, pose) in &poses_before {
        assert_eq!(pose_of(&resized, id), *pose, "{id}");
    }
    // No mirrored newcomer was fabricated for the Local partner.
    assert!(
        resized
            .parts
            .iter()
            .all(|part| part.id != member("main2", 0, 3))
    );
    // One transaction: Undo restores the pre-resize matrices, Redo restores
    // the resize (revision bumps, so compare content, not whole docs).
    let undone_doc = undone(&mut engine);
    assert_eq!(undone_doc.matrices, doc.matrices);
    assert_eq!(undone_doc.layouts, doc.layouts);
    assert_eq!(undone_doc.parts, doc.parts);
    let redone_doc = redone(&mut engine);
    assert_eq!(redone_doc.matrices, resized.matrices);
    assert_eq!(redone_doc.layouts, resized.layouts);
    assert_eq!(redone_doc.parts, resized.parts);
}

#[test]
fn linked_resize_keeps_omitted_partner_live_and_absent() {
    // Ticket 21: source growth with an Omitted partner succeeds; the partner
    // stays absent (no parts fabricated) while its record follows the source
    // so anchors through it keep a live logical pose.
    let (mut engine, doc) = linked_anchored_fixture();
    let mut mirror_layout = doc
        .layouts
        .iter()
        .find(|layout| layout.id == "mirror-layout")
        .expect("mirror layout")
        .clone();
    mirror_layout
        .mirror_link
        .as_mut()
        .expect("mirror link")
        .exceptions
        .push(MirroredException {
            kind: MirroredItemKind::Matrix,
            mirror_id: "main2".into(),
            state: MirroredExceptionState::Omitted,
            companion_mode: None,
            companion_tweak: None,
            removed_copper: None,
        });
    // The Omit transition itself suppresses the inherited parts.
    let (_, doc) = edit(
        &mut engine,
        &doc,
        EditOperation::SetLayout {
            layout: mirror_layout,
        },
    );
    assert!(
        doc.parts
            .iter()
            .all(|part| !part.id.starts_with("matrix/main2/")),
        "omit must suppress inherited parts"
    );
    assert!(
        doc.matrices.iter().any(|matrix| matrix.id == "main2"),
        "omit keeps the live record"
    );
    // Grow the source through the public prepared route.
    let mut grown = doc
        .matrices
        .iter()
        .find(|matrix| matrix.id == "main")
        .expect("main matrix")
        .clone();
    grown.columns = 4;
    let (_, resized) = edit(
        &mut engine,
        &doc,
        EditOperation::SetMatrix {
            matrix: grown,
            definitions: None,
        },
    );
    // The source grew; the Omitted record followed without materializing.
    assert_eq!(
        resized
            .matrices
            .iter()
            .find(|matrix| matrix.id == "main")
            .expect("grown source")
            .columns,
        4
    );
    let partner = resized
        .matrices
        .iter()
        .find(|matrix| matrix.id == "main2")
        .expect("omitted partner");
    assert_eq!((partner.rows, partner.columns), (1, 4));
    assert!(
        resized
            .parts
            .iter()
            .all(|part| !part.id.starts_with("matrix/main2/")),
        "no mirrored newcomer fabricated for the Omitted partner"
    );
    // Anchors through the Omitted record still evaluate at the live pose.
    assert_eq!(
        pose_of(&resized, &member("thumb2", 0, 0)).at,
        Vec2 { x: -18.0, y: -19.0 }
    );
    // One transaction: Undo restores the pre-resize document.
    let undone_doc = undone(&mut engine);
    assert_eq!(undone_doc.matrices, doc.matrices);
    assert_eq!(undone_doc.layouts, doc.layouts);
    assert_eq!(undone_doc.parts, doc.parts);
    let redone_doc = redone(&mut engine);
    assert_eq!(redone_doc.matrices, resized.matrices);
    assert_eq!(redone_doc.layouts, resized.layouts);
    assert_eq!(redone_doc.parts, resized.parts);
}

#[test]
fn linked_make_local_on_anchored_matrix_detaches_and_keeps_pose() {
    // Ticket 21: making an anchored matrix Local detaches its own anchor and
    // keeps its world pose; its future layout stays its own.
    let (mut engine, doc) = linked_anchored_fixture();
    let poses_before: Vec<(String, Pose2)> = doc
        .parts
        .iter()
        .map(|part| (part.id.clone(), part.pose))
        .collect();
    let mut mirror_layout = doc
        .layouts
        .iter()
        .find(|layout| layout.id == "mirror-layout")
        .expect("mirror layout")
        .clone();
    mirror_layout
        .mirror_link
        .as_mut()
        .expect("mirror link")
        .exceptions
        .push(MirroredException {
            kind: MirroredItemKind::Matrix,
            mirror_id: "thumb2".into(),
            state: MirroredExceptionState::Local,
            companion_mode: None,
            companion_tweak: None,
            removed_copper: None,
        });
    let (_, local) = edit(
        &mut engine,
        &doc,
        EditOperation::SetLayout {
            layout: mirror_layout,
        },
    );
    // Its own anchor cleared, every pose kept.
    assert_eq!(anchor_of(&local, "thumb2"), None);
    assert!(anchor_of(&local, "thumb").is_some());
    for (id, pose) in &poses_before {
        assert_eq!(pose_of(&local, id), *pose, "{id}");
    }
    // One Undo restores the anchor and the exception state together.
    let undone_doc = undone(&mut engine);
    assert_eq!(anchor_of(&undone_doc, "thumb2"), anchor_of(&doc, "thumb2"));
    assert_eq!(undone_doc.layouts, doc.layouts);
    let redone_doc = redone(&mut engine);
    assert_eq!(anchor_of(&redone_doc, "thumb2"), None);
    assert_eq!(redone_doc.layouts, local.layouts);
}

#[test]
fn linked_local_to_omitted_key_returns_to_shared_pose() {
    // Ticket 21: Local->Omitted replaces Local state instead of freezing the
    // former Local placement: the key's logical pose returns to the current
    // shared layout while explicit child exceptions are preserved.
    let (mut engine, doc) = linked_anchored_fixture();
    // Make the mirror anchor key Local, then move it independently.
    let mut mirror_layout = doc
        .layouts
        .iter()
        .find(|layout| layout.id == "mirror-layout")
        .expect("mirror layout")
        .clone();
    mirror_layout
        .mirror_link
        .as_mut()
        .expect("mirror link")
        .exceptions
        .push(MirroredException {
            kind: MirroredItemKind::Part,
            mirror_id: member("main2", 0, 2),
            state: MirroredExceptionState::Local,
            companion_mode: None,
            companion_tweak: None,
            removed_copper: None,
        });
    let (_, doc) = edit(
        &mut engine,
        &doc,
        EditOperation::SetLayout {
            layout: mirror_layout,
        },
    );
    let (_, moved) = edit(
        &mut engine,
        &doc,
        EditOperation::MoveParts {
            positions: vec![Position {
                id: member("main2", 0, 2),
                at: Vec2 { x: -10.0, y: 5.0 },
            }],
        },
    );
    assert_eq!(
        pose_of(&moved, &member("main2", 0, 2)).at,
        Vec2 { x: -10.0, y: 5.0 }
    );
    // Switch the same key from Local to Omitted via the public route.
    let mut omitted_layout = moved
        .layouts
        .iter()
        .find(|layout| layout.id == "mirror-layout")
        .expect("mirror layout")
        .clone();
    if let Some(link) = omitted_layout.mirror_link.as_mut() {
        for exception in &mut link.exceptions {
            if exception.mirror_id == member("main2", 0, 2) {
                exception.state = MirroredExceptionState::Omitted;
            }
        }
    }
    let (_, omitted) = edit(
        &mut engine,
        &moved,
        EditOperation::SetLayout {
            layout: omitted_layout,
        },
    );
    // The Omitted key's part is suppressed, not frozen at its Local pose.
    assert!(
        omitted
            .parts
            .iter()
            .all(|part| part.id != member("main2", 0, 2)),
        "omit must suppress the key part, not freeze its Local placement"
    );
    // The dependent still follows the live shared anchor: the x=10
    // reflection of main(0,2) at (38,0), not the former Local (-10,5).
    assert_eq!(
        pose_of(&omitted, &member("thumb2", 0, 0)).at,
        Vec2 { x: -18.0, y: -19.0 }
    );
    // Rejoin restores the part at the shared pose in the same transaction.
    let mut rejoined_layout = omitted
        .layouts
        .iter()
        .find(|layout| layout.id == "mirror-layout")
        .expect("mirror layout")
        .clone();
    if let Some(link) = rejoined_layout.mirror_link.as_mut() {
        link.exceptions
            .retain(|exception| exception.mirror_id != member("main2", 0, 2));
    }
    let (_, rejoined) = edit(
        &mut engine,
        &omitted,
        EditOperation::SetLayout {
            layout: rejoined_layout,
        },
    );
    assert_eq!(
        pose_of(&rejoined, &member("main2", 0, 2)).at,
        Vec2 { x: -18.0, y: 0.0 }
    );
    assert_eq!(
        pose_of(&rejoined, &member("thumb2", 0, 0)).at,
        Vec2 { x: -18.0, y: -19.0 }
    );
    // Rejoin is one Undo entry: Undo returns to the Omitted state.
    let undone_doc = undone(&mut engine);
    assert_eq!(undone_doc.layouts, omitted.layouts);
    assert_eq!(undone_doc.parts, omitted.parts);
    let redone_doc = redone(&mut engine);
    assert_eq!(redone_doc.parts, rejoined.parts);
}

#[test]
fn linked_rejoin_adopts_current_source_and_keeps_child_exceptions() {
    // Ticket 21: Rejoin adopts the current source dimensions/layout/anchor
    // and preserves explicit child exceptions.
    let (mut engine, doc) = linked_anchored_fixture();
    // Child exception first: the mirror key goes Local and moves.
    let mut mirror_layout = doc
        .layouts
        .iter()
        .find(|layout| layout.id == "mirror-layout")
        .expect("mirror layout")
        .clone();
    mirror_layout
        .mirror_link
        .as_mut()
        .expect("mirror link")
        .exceptions
        .push(MirroredException {
            kind: MirroredItemKind::Part,
            mirror_id: member("main2", 0, 0),
            state: MirroredExceptionState::Local,
            companion_mode: None,
            companion_tweak: None,
            removed_copper: None,
        });
    let (_, doc) = edit(
        &mut engine,
        &doc,
        EditOperation::SetLayout {
            layout: mirror_layout,
        },
    );
    let (_, doc) = edit(
        &mut engine,
        &doc,
        EditOperation::MoveParts {
            positions: vec![Position {
                id: member("main2", 0, 0),
                at: Vec2 { x: 30.0, y: 5.0 },
            }],
        },
    );
    // Omit the whole matrix, grow the source while absent, then Rejoin.
    let mut omitted_layout = doc
        .layouts
        .iter()
        .find(|layout| layout.id == "mirror-layout")
        .expect("mirror layout")
        .clone();
    omitted_layout
        .mirror_link
        .as_mut()
        .expect("mirror link")
        .exceptions
        .push(MirroredException {
            kind: MirroredItemKind::Matrix,
            mirror_id: "main2".into(),
            state: MirroredExceptionState::Omitted,
            companion_mode: None,
            companion_tweak: None,
            removed_copper: None,
        });
    let (_, doc) = edit(
        &mut engine,
        &doc,
        EditOperation::SetLayout {
            layout: omitted_layout,
        },
    );
    // Inherited parts shed; the explicit Local child keeps its own state.
    assert!(
        doc.parts
            .iter()
            .filter(|part| part.id.starts_with("matrix/main2/"))
            .map(|part| part.id.clone())
            .collect::<Vec<_>>()
            == vec![member("main2", 0, 0)],
        "omit sheds inherited parts but keeps explicit child exceptions"
    );
    assert_eq!(
        pose_of(&doc, &member("main2", 0, 0)).at,
        Vec2 { x: 30.0, y: 5.0 }
    );
    let mut grown = doc
        .matrices
        .iter()
        .find(|matrix| matrix.id == "main")
        .expect("main matrix")
        .clone();
    grown.columns = 4;
    let (_, doc) = edit(
        &mut engine,
        &doc,
        EditOperation::SetMatrix {
            matrix: grown,
            definitions: None,
        },
    );
    // Rejoin: remove the matrix overlay, keep the child.
    let mut rejoined_layout = doc
        .layouts
        .iter()
        .find(|layout| layout.id == "mirror-layout")
        .expect("mirror layout")
        .clone();
    if let Some(link) = rejoined_layout.mirror_link.as_mut() {
        link.exceptions.retain(|exception| {
            !(exception.kind == MirroredItemKind::Matrix && exception.mirror_id == "main2")
        });
    }
    let (_, rejoined) = edit(
        &mut engine,
        &doc,
        EditOperation::SetLayout {
            layout: rejoined_layout,
        },
    );
    // Current source dimensions adopted with materialized parts.
    let partner = rejoined
        .matrices
        .iter()
        .find(|matrix| matrix.id == "main2")
        .expect("rejoined partner");
    assert_eq!((partner.rows, partner.columns), (1, 4));
    for column in 0..4 {
        assert!(
            rejoined
                .parts
                .iter()
                .any(|part| part.id == member("main2", 0, column)),
            "rejoined part (0,{column}) materialized"
        );
    }
    // Explicit child exception preserved with its Local pose.
    assert!(
        rejoined
            .layouts
            .iter()
            .find(|layout| layout.id == "mirror-layout")
            .and_then(|layout| layout.mirror_link.as_ref())
            .is_some_and(|link| link.exceptions.iter().any(|exception| {
                exception.kind == MirroredItemKind::Part
                    && exception.mirror_id == member("main2", 0, 0)
                    && exception.state == MirroredExceptionState::Local
            }))
    );
    assert_eq!(
        pose_of(&rejoined, &member("main2", 0, 0)).at,
        Vec2 { x: 30.0, y: 5.0 }
    );
    // Shared members track the source again.
    assert_eq!(
        pose_of(&rejoined, &member("main2", 0, 1)).at,
        Vec2 { x: 1.0, y: 0.0 }
    );
}

#[test]
fn linked_compaction_excludes_local_partner() {
    // Ticket 21: a grouped column removal edits the linked source alone; the
    // Local partner keeps its own dimensions and every world pose.
    let (mut engine, doc) = linked_anchored_fixture();
    let mut mirror_layout = doc
        .layouts
        .iter()
        .find(|layout| layout.id == "mirror-layout")
        .expect("mirror layout")
        .clone();
    mirror_layout
        .mirror_link
        .as_mut()
        .expect("mirror link")
        .exceptions
        .push(MirroredException {
            kind: MirroredItemKind::Matrix,
            mirror_id: "main2".into(),
            state: MirroredExceptionState::Local,
            companion_mode: None,
            companion_tweak: None,
            removed_copper: None,
        });
    let (_, doc) = edit(
        &mut engine,
        &doc,
        EditOperation::SetLayout {
            layout: mirror_layout,
        },
    );
    let poses_before: Vec<(String, Pose2)> = doc
        .parts
        .iter()
        .map(|part| (part.id.clone(), part.pose))
        .collect();
    let (_, compacted) = edit(
        &mut engine,
        &doc,
        EditOperation::RemoveMatrixColumn {
            matrix_id: "main".into(),
            column: 0,
        },
    );
    // Source compacted; Local partner untouched.
    assert_eq!(
        compacted
            .matrices
            .iter()
            .find(|matrix| matrix.id == "main")
            .expect("compacted source")
            .columns,
        2
    );
    let partner = compacted
        .matrices
        .iter()
        .find(|matrix| matrix.id == "main2")
        .expect("local partner");
    assert_eq!((partner.rows, partner.columns), (1, 3));
    // The Local partner keeps every world pose; the linked thumb pair
    // follows the compacted anchor (ticket 19 remap), so only main2 stays.
    for (id, pose) in &poses_before {
        if id.starts_with("matrix/main2/") {
            assert_eq!(pose_of(&compacted, id), *pose, "{id}");
        }
    }
    assert!(
        compacted
            .parts
            .iter()
            .any(|part| part.id == member("main2", 0, 2))
    );
    // The source anchor compacted onto its surviving physical key.
    assert_eq!(
        anchor_of(&compacted, "thumb"),
        Some(MatrixAnchor {
            matrix_id: "main".into(),
            row: 0,
            column: 1,
            follow: Follow {
                x: true,
                y: true,
                rotation: true,
            },
            aggregate: None,
        })
    );
    let undone_doc = undone(&mut engine);
    assert_eq!(undone_doc.matrices, doc.matrices);
}

#[test]
fn omitted_storage_edits_are_refused_with_rejoin_guidance() {
    // Ticket 21: an absent matrix has no independent storage state; direct
    // edits refuse naming Rejoin and change nothing.
    let (mut engine, doc) = linked_anchored_fixture();
    let mut mirror_layout = doc
        .layouts
        .iter()
        .find(|layout| layout.id == "mirror-layout")
        .expect("mirror layout")
        .clone();
    mirror_layout
        .mirror_link
        .as_mut()
        .expect("mirror link")
        .exceptions
        .push(MirroredException {
            kind: MirroredItemKind::Matrix,
            mirror_id: "main2".into(),
            state: MirroredExceptionState::Omitted,
            companion_mode: None,
            companion_tweak: None,
            removed_copper: None,
        });
    let (_, doc) = edit(
        &mut engine,
        &doc,
        EditOperation::SetLayout {
            layout: mirror_layout,
        },
    );
    let mut attempted = doc
        .matrices
        .iter()
        .find(|matrix| matrix.id == "main2")
        .expect("omitted record")
        .clone();
    attempted.columns = 5;
    let message = error(
        &mut engine,
        &doc,
        EditOperation::SetMatrix {
            matrix: attempted,
            definitions: None,
        },
    );
    assert!(
        message.contains("Rejoin"),
        "unexpected message: {message:?}"
    );
    assert_eq!(snapshot(&mut engine), doc, "refusal changed state");
}

#[test]
fn linked_rejoin_refuses_missing_anchor_atomically() {
    // Ticket 21: Rejoin validates the resulting graph atomically. Shrinking
    // the Local mirror parent retires the Omitted dependent's anchor; a
    // later Rejoin adopting the current source anchor refuses naming the
    // missing counterpart without touching the Local parent or other state.
    let (mut engine, doc) = linked_anchored_fixture();
    // Omit the mirror dependent first.
    let mut mirror_layout = doc
        .layouts
        .iter()
        .find(|layout| layout.id == "mirror-layout")
        .expect("mirror layout")
        .clone();
    mirror_layout
        .mirror_link
        .as_mut()
        .expect("mirror link")
        .exceptions
        .push(MirroredException {
            kind: MirroredItemKind::Matrix,
            mirror_id: "thumb2".into(),
            state: MirroredExceptionState::Omitted,
            companion_mode: None,
            companion_tweak: None,
            removed_copper: None,
        });
    // Shrink the Local mirror parent; the Omitted dependent detaches.
    mirror_layout
        .mirror_link
        .as_mut()
        .expect("mirror link")
        .exceptions
        .push(MirroredException {
            kind: MirroredItemKind::Matrix,
            mirror_id: "main2".into(),
            state: MirroredExceptionState::Local,
            companion_mode: None,
            companion_tweak: None,
            removed_copper: None,
        });
    let (_, doc) = edit(
        &mut engine,
        &doc,
        EditOperation::SetLayout {
            layout: mirror_layout,
        },
    );
    let (_, doc) = edit(
        &mut engine,
        &doc,
        EditOperation::RemoveMatrixColumn {
            matrix_id: "main2".into(),
            column: 2,
        },
    );
    assert_eq!(
        doc.matrices
            .iter()
            .find(|matrix| matrix.id == "main2")
            .expect("local parent")
            .columns,
        2
    );
    assert_eq!(anchor_of(&doc, "thumb2"), None);
    // Rejoin adopts the source anchor (main (0,2)) whose counterpart in the
    // smaller Local parent is missing: refuse atomically.
    let mut rejoined_layout = doc
        .layouts
        .iter()
        .find(|layout| layout.id == "mirror-layout")
        .expect("mirror layout")
        .clone();
    if let Some(link) = rejoined_layout.mirror_link.as_mut() {
        link.exceptions.retain(|exception| {
            !(exception.kind == MirroredItemKind::Matrix && exception.mirror_id == "thumb2")
        });
    }
    let message = error(
        &mut engine,
        &doc,
        EditOperation::SetLayout {
            layout: rejoined_layout,
        },
    );
    assert!(
        message.contains("matrix/main2/r0c2"),
        "names the missing counterpart: {message:?}"
    );
    // Nothing changed: the Local parent keeps its size and state, and the
    // rest of the document is byte-identical.
    assert_eq!(snapshot(&mut engine), doc);
    assert_eq!(
        doc.matrices
            .iter()
            .find(|matrix| matrix.id == "main2")
            .expect("local parent")
            .columns,
        2
    );
}

#[test]
fn linked_detach_needing_local_exception_refuses_with_proposal() {
    // Ticket 21: a shared detach that would move the partner refuses naming
    // the required Local exception; cancel (no commit) changes nothing.
    let (mut engine, doc) = linked_anchored_fixture();
    // The mirror anchor key goes Local and moves: the dependent follows its
    // actual Local anchor, so a plain detach would jump the partner.
    let mut mirror_layout = doc
        .layouts
        .iter()
        .find(|layout| layout.id == "mirror-layout")
        .expect("mirror layout")
        .clone();
    mirror_layout
        .mirror_link
        .as_mut()
        .expect("mirror link")
        .exceptions
        .push(MirroredException {
            kind: MirroredItemKind::Part,
            mirror_id: member("main2", 0, 2),
            state: MirroredExceptionState::Local,
            companion_mode: None,
            companion_tweak: None,
            removed_copper: None,
        });
    let (_, doc) = edit(
        &mut engine,
        &doc,
        EditOperation::SetLayout {
            layout: mirror_layout,
        },
    );
    let (_, doc) = edit(
        &mut engine,
        &doc,
        EditOperation::MoveParts {
            positions: vec![Position {
                id: member("main2", 0, 2),
                at: Vec2 { x: -10.0, y: 5.0 },
            }],
        },
    );
    assert_eq!(
        pose_of(&doc, &member("thumb2", 0, 0)).at,
        Vec2 { x: -10.0, y: -14.0 }
    );
    // Plain detach refuses naming the needed Local exception.
    let message = error(
        &mut engine,
        &doc,
        EditOperation::DetachMatrixAnchor {
            matrix_id: "thumb".into(),
        },
    );
    assert!(
        message.contains("thumb2") && message.contains("Local"),
        "names the required exception: {message:?}"
    );
    // Cancel changes nothing: the document is byte-identical.
    assert_eq!(snapshot(&mut engine), doc);
    assert!(anchor_of(&doc, "thumb").is_some());
    assert!(anchor_of(&doc, "thumb2").is_some());
}

#[test]
fn linked_detach_confirmed_applies_detach_and_local_in_one_entry() {
    // Ticket 21: the confirmed detach clears both anchors, keeps both world
    // poses, and records the Local exception in one Undo entry.
    let (mut engine, doc) = linked_anchored_fixture();
    let mut mirror_layout = doc
        .layouts
        .iter()
        .find(|layout| layout.id == "mirror-layout")
        .expect("mirror layout")
        .clone();
    mirror_layout
        .mirror_link
        .as_mut()
        .expect("mirror link")
        .exceptions
        .push(MirroredException {
            kind: MirroredItemKind::Part,
            mirror_id: member("main2", 0, 2),
            state: MirroredExceptionState::Local,
            companion_mode: None,
            companion_tweak: None,
            removed_copper: None,
        });
    let (_, doc) = edit(
        &mut engine,
        &doc,
        EditOperation::SetLayout {
            layout: mirror_layout,
        },
    );
    let (_, doc) = edit(
        &mut engine,
        &doc,
        EditOperation::MoveParts {
            positions: vec![Position {
                id: member("main2", 0, 2),
                at: Vec2 { x: -10.0, y: 5.0 },
            }],
        },
    );
    let poses_before: Vec<(String, Pose2)> = doc
        .parts
        .iter()
        .map(|part| (part.id.clone(), part.pose))
        .collect();
    let (_, detached) = edit(
        &mut engine,
        &doc,
        EditOperation::DetachMatrixAnchorConfirmed {
            matrix_id: "thumb".into(),
            local_matrix_id: "thumb2".into(),
        },
    );
    // Both anchors cleared, every world pose kept, exception recorded.
    assert_eq!(anchor_of(&detached, "thumb"), None);
    assert_eq!(anchor_of(&detached, "thumb2"), None);
    for (id, pose) in &poses_before {
        assert_eq!(pose_of(&detached, id), *pose, "{id}");
    }
    assert!(
        detached
            .layouts
            .iter()
            .find(|layout| layout.id == "mirror-layout")
            .and_then(|layout| layout.mirror_link.as_ref())
            .is_some_and(|link| link.exceptions.iter().any(|exception| {
                exception.kind == MirroredItemKind::Matrix
                    && exception.mirror_id == "thumb2"
                    && exception.state == MirroredExceptionState::Local
            }))
    );
    // One Undo restores anchors and drops the exception; Redo reapplies all.
    let undone_doc = undone(&mut engine);
    assert_eq!(anchor_of(&undone_doc, "thumb"), anchor_of(&doc, "thumb"));
    assert_eq!(anchor_of(&undone_doc, "thumb2"), anchor_of(&doc, "thumb2"));
    assert_eq!(undone_doc.layouts, doc.layouts);
    let redone_doc = redone(&mut engine);
    assert_eq!(anchor_of(&redone_doc, "thumb"), None);
    assert_eq!(anchor_of(&redone_doc, "thumb2"), None);
    assert_eq!(redone_doc.layouts, detached.layouts);
    // A stale confirmation (wrong partner) refuses without changes.
    let current = snapshot(&mut engine);
    let stale = error(
        &mut engine,
        &current,
        EditOperation::DetachMatrixAnchorConfirmed {
            matrix_id: "thumb".into(),
            local_matrix_id: "main2".into(),
        },
    );
    assert!(stale.contains("stale"), "unexpected message: {stale:?}");
    assert_eq!(snapshot(&mut engine), current);
}

#[test]
fn linked_source_removal_retains_local_matrix_and_retires_only_defunct_links() {
    // Ticket 21: deleting a source matrix retains its complete Local
    // counterpart (not loose parts), retires only the defunct records, and
    // preserves surviving ownership, anchors and unrelated links.
    let (mut engine, doc) = linked_anchored_fixture();
    let mut mirror_layout = doc
        .layouts
        .iter()
        .find(|layout| layout.id == "mirror-layout")
        .expect("mirror layout")
        .clone();
    mirror_layout
        .mirror_link
        .as_mut()
        .expect("mirror link")
        .exceptions
        .push(MirroredException {
            kind: MirroredItemKind::Matrix,
            mirror_id: "main2".into(),
            state: MirroredExceptionState::Local,
            companion_mode: None,
            companion_tweak: None,
            removed_copper: None,
        });
    let (_, doc) = edit(
        &mut engine,
        &doc,
        EditOperation::SetLayout {
            layout: mirror_layout,
        },
    );
    let poses_before: Vec<(String, Pose2)> = doc
        .parts
        .iter()
        .map(|part| (part.id.clone(), part.pose))
        .collect();
    let (_, removed) = edit(
        &mut engine,
        &doc,
        EditOperation::RemoveMatrix { id: "main".into() },
    );
    // Source gone with its parts; the Local matrix survives complete.
    assert!(
        removed.matrices.iter().all(|matrix| matrix.id != "main"),
        "source matrix removed"
    );
    assert!(
        removed
            .parts
            .iter()
            .all(|part| !part.id.starts_with("matrix/main/")),
        "source parts removed"
    );
    let survivor = removed
        .matrices
        .iter()
        .find(|matrix| matrix.id == "main2")
        .expect("complete Local survivor");
    assert_eq!((survivor.rows, survivor.columns), (1, 3));
    for column in 0..3 {
        assert!(
            removed
                .parts
                .iter()
                .any(|part| part.id == member("main2", 0, column)),
            "survivor part (0,{column}) retained"
        );
    }
    for (id, pose) in &poses_before {
        if id.starts_with("matrix/main2/") {
            assert_eq!(pose_of(&removed, id), *pose, "{id}");
        }
    }
    // Only the defunct correspondence retired; the unrelated pair intact.
    let link = removed
        .layouts
        .iter()
        .find(|layout| layout.id == "mirror-layout")
        .and_then(|layout| layout.mirror_link.as_ref())
        .expect("surviving link");
    assert!(
        link.correspondences.iter().all(|record| {
            !(record.kind == MirroredItemKind::Matrix
                && (record.source_id == "main" || record.mirror_id == "main2"))
        }),
        "defunct matrix correspondence retired"
    );
    assert!(
        link.correspondences.iter().any(|record| {
            record.kind == MirroredItemKind::Matrix
                && record.source_id == "thumb"
                && record.mirror_id == "thumb2"
        }),
        "unrelated link preserved"
    );
    // The source-anchored dependent detached with its frame; the surviving
    // anchored pair still evaluates.
    assert_eq!(anchor_of(&removed, "thumb"), None);
    assert_eq!(
        pose_of(&removed, &member("thumb2", 0, 0)).at,
        Vec2 { x: -18.0, y: -19.0 }
    );
    // One Undo restores the source; Redo removes it again.
    let undone_doc = undone(&mut engine);
    assert!(undone_doc.matrices.iter().any(|matrix| matrix.id == "main"));
    let redone_doc = redone(&mut engine);
    assert!(redone_doc.matrices.iter().all(|matrix| matrix.id != "main"));
    assert!(
        redone_doc
            .matrices
            .iter()
            .any(|matrix| matrix.id == "main2")
    );
}

#[test]
fn linked_source_removal_deletes_linked_partner_and_keeps_unrelated_pair() {
    // Ticket 21: without an exception the linked counterpart follows the
    // source deletion; unrelated pairs and their poses survive.
    let (mut engine, doc) = linked_anchored_fixture();
    let poses_before: Vec<(String, Pose2)> = doc
        .parts
        .iter()
        .map(|part| (part.id.clone(), part.pose))
        .collect();
    let (_, removed) = edit(
        &mut engine,
        &doc,
        EditOperation::RemoveMatrix { id: "main".into() },
    );
    // Both halves of the removed pair are gone with their parts.
    for gone in ["main", "main2"] {
        assert!(
            removed.matrices.iter().all(|matrix| matrix.id != gone),
            "{gone} removed"
        );
        assert!(
            removed
                .parts
                .iter()
                .all(|part| !part.id.starts_with(&format!("matrix/{gone}/"))),
            "{gone} parts removed"
        );
    }
    // The unrelated pair survives with its poses and correspondence.
    for kept in ["thumb", "thumb2"] {
        assert!(
            removed.matrices.iter().any(|matrix| matrix.id == kept),
            "{kept} survives"
        );
    }
    for (id, pose) in &poses_before {
        if id.starts_with("matrix/thumb") {
            assert_eq!(pose_of(&removed, id), *pose, "{id}");
        }
    }
    let link = removed
        .layouts
        .iter()
        .find(|layout| layout.id == "mirror-layout")
        .and_then(|layout| layout.mirror_link.as_ref())
        .expect("surviving link");
    assert!(
        link.correspondences.iter().any(|record| {
            record.kind == MirroredItemKind::Matrix
                && record.source_id == "thumb"
                && record.mirror_id == "thumb2"
        }),
        "unrelated correspondence preserved"
    );
    assert!(
        link.correspondences
            .iter()
            .all(|record| { record.source_id != "main" && record.mirror_id != "main2" }),
        "defunct records retired"
    );
    let undone_doc = undone(&mut engine);
    assert!(undone_doc.matrices.iter().any(|matrix| matrix.id == "main"));
    assert!(
        undone_doc
            .matrices
            .iter()
            .any(|matrix| matrix.id == "main2")
    );
}

#[test]
fn linked_source_removal_retires_omitted_records_without_resurrection() {
    // Ticket 21: an Omitted counterpart stays absent and its record retires
    // with the source; nothing resurrects it.
    let (mut engine, doc) = linked_anchored_fixture();
    let mut mirror_layout = doc
        .layouts
        .iter()
        .find(|layout| layout.id == "mirror-layout")
        .expect("mirror layout")
        .clone();
    mirror_layout
        .mirror_link
        .as_mut()
        .expect("mirror link")
        .exceptions
        .push(MirroredException {
            kind: MirroredItemKind::Matrix,
            mirror_id: "main2".into(),
            state: MirroredExceptionState::Omitted,
            companion_mode: None,
            companion_tweak: None,
            removed_copper: None,
        });
    let (_, doc) = edit(
        &mut engine,
        &doc,
        EditOperation::SetLayout {
            layout: mirror_layout,
        },
    );
    let (_, removed) = edit(
        &mut engine,
        &doc,
        EditOperation::RemoveMatrix { id: "main".into() },
    );
    assert!(
        removed.matrices.iter().all(|matrix| matrix.id != "main"),
        "source removed"
    );
    assert!(
        removed.matrices.iter().all(|matrix| matrix.id != "main2"),
        "omitted record retired"
    );
    assert!(
        removed
            .parts
            .iter()
            .all(|part| !part.id.starts_with("matrix/main2/")),
        "nothing resurrected"
    );
    // The surviving pair still evaluates through its live anchors.
    assert_eq!(
        pose_of(&removed, &member("thumb2", 0, 0)).at,
        Vec2 { x: -18.0, y: -19.0 }
    );
    let undone_doc = undone(&mut engine);
    assert!(undone_doc.matrices.iter().any(|matrix| matrix.id == "main"));
}
