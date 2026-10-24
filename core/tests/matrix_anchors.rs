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
    let message = error(
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
            },
        },
    );
    assert!(
        message.contains("ticket 20"),
        "unexpected message: {message:?}"
    );
    assert_eq!(snapshot(&mut engine), doc);
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
