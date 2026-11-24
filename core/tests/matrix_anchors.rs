//! Ticket 19: Core-owned single-key anchor evaluation and typed Add/change/detach
//! edits. All behavior runs through public Core routes (`Open` / `Edit`), never
//! graph helpers alone.
use crate::support::fixture::{self, key_part, part_pose as pose_of, scene_and_document as scene};
use crate::support::session::{commit_edit, open_engine};
use boardstudio_core::{CommittedScene, CoreEngine, EditOutcome, model::*};

#[path = "matrix_anchors/anchor_chunk0.rs"]
mod anchor_chunk0;
#[path = "matrix_anchors/anchor_chunk1.rs"]
mod anchor_chunk1;
#[path = "matrix_anchors/anchor_chunk2.rs"]
mod anchor_chunk2;

fn preview(engine: &mut CoreEngine, doc: &ProjectDoc, operation: EditOperation) -> SceneDelta {
    match engine.apply_edit(fixture::edit(
        "preview",
        "preview",
        doc.revision,
        EditPhase::Preview,
        operation,
    )) {
        Ok(EditOutcome::Preview(scene)) => scene,
        other => panic!("expected preview: {other:?}"),
    }
}

fn error(engine: &mut CoreEngine, doc: &ProjectDoc, operation: EditOperation) -> String {
    match engine.commit_edit(fixture::commit("edit", "edit", doc.revision, operation)) {
        Err(message) => message,
        other => panic!("expected error: {other:?}"),
    }
}

fn definitions() -> Vec<PartDefinition> {
    serde_json::from_value(serde_json::json!([
        {"id":"switch","name":"Switch","kind":"switch","courtyard":[],"pads":[]}
    ]))
    .unwrap()
}

fn main_matrix() -> Matrix {
    Matrix {
        part_ids: vec![
            member("main", 0, 0),
            member("main", 0, 1),
            member("main", 0, 2),
        ],
        board_id: Some("board".into()),
        ..fixture::matrix("main", 1, 3)
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
        part_ids: vec![
            member("main", 0, 0),
            member("main", 0, 1),
            member("main", 0, 2),
        ],
        ..fixture::board("board", "Board")
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
    let (engine, _, doc) = open_engine(doc);
    (engine, doc)
}

fn at(doc: &ProjectDoc, id: &str) -> Vec2 {
    pose_of(doc, id).at
}

fn assert_at(doc: &ProjectDoc, id: &str, x: f64, y: f64) {
    assert_eq!(at(doc, id), Vec2 { x, y }, "position of {id}");
}

fn member(matrix: &str, row: u32, column: u32) -> String {
    format!("matrix/{matrix}/r{row}c{column}")
}

fn anchored_fixture() -> (CoreEngine, ProjectDoc) {
    let (mut engine, doc) = fixture();
    let doc = add_thumb(&mut engine, &doc);
    (engine, doc)
}

fn matrix<'a>(doc: &'a ProjectDoc, id: &str) -> &'a Matrix {
    doc.matrices
        .iter()
        .find(|matrix| matrix.id == id)
        .unwrap_or_else(|| panic!("missing matrix {id}"))
}

fn layout<'a>(doc: &'a ProjectDoc, id: &str) -> &'a Layout {
    doc.layouts
        .iter()
        .find(|layout| layout.id == id)
        .unwrap_or_else(|| panic!("missing layout {id}"))
}

fn poses(doc: &ProjectDoc) -> impl Iterator<Item = (&str, Pose2)> {
    doc.parts.iter().map(|part| (part.id.as_str(), part.pose))
}

fn change_matrix(doc: &ProjectDoc, id: &str, update: impl FnOnce(&mut Matrix)) -> EditOperation {
    let mut matrix = matrix(doc, id).clone();
    update(&mut matrix);
    fixture::set_matrix(matrix)
}

fn move_part(id: impl Into<String>, at: Vec2) -> EditOperation {
    EditOperation::MoveParts {
        positions: vec![Position { id: id.into(), at }],
    }
}

fn key_anchor(id: &str, row: u32, column: u32) -> MatrixAnchor {
    MatrixAnchor {
        matrix_id: id.into(),
        row,
        column,
        follow: Follow {
            x: true,
            y: true,
            rotation: true,
        },
        aggregate: None,
    }
}

fn add_anchored(id: &str, anchor: MatrixAnchor) -> EditOperation {
    EditOperation::AddAnchoredMatrix {
        id: id.into(),
        anchor,
    }
}

fn reanchor(id: &str, anchor: MatrixAnchor) -> EditOperation {
    EditOperation::SetMatrixAnchor {
        matrix_id: id.into(),
        anchor,
    }
}

fn anchor_of(doc: &ProjectDoc, id: &str) -> Option<MatrixAnchor> {
    doc.matrices
        .iter()
        .find(|matrix| matrix.id == id)
        .and_then(|matrix| matrix.anchor.clone())
}

fn undone(engine: &mut CoreEngine) -> ProjectDoc {
    match engine.undo() {
        Ok(CommittedScene { document, .. }) => document,
        other => panic!("expected scene: {other:?}"),
    }
}

fn redone(engine: &mut CoreEngine) -> ProjectDoc {
    match engine.redo() {
        Ok(CommittedScene { document, .. }) => document,
        other => panic!("expected scene: {other:?}"),
    }
}

fn snapshot(engine: &mut CoreEngine) -> ProjectDoc {
    scene(engine.snapshot()).1
}

fn add_thumb(engine: &mut CoreEngine, doc: &ProjectDoc) -> ProjectDoc {
    commit_edit(engine, doc, add_anchored("thumb", key_anchor("main", 0, 2))).1
}

fn near(actual: Vec2, x: f64, y: f64) {
    assert!(
        (actual.x - x).abs() < 1e-9 && (actual.y - y).abs() < 1e-9,
        "expected ({x}, {y}), got ({}, {})",
        actual.x,
        actual.y
    );
}

fn single_matrix(id: &str, board: &str) -> Matrix {
    Matrix {
        part_ids: vec![member(id, 0, 0)],
        board_id: Some(board.into()),
        ..fixture::matrix(id, 1, 1)
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
            part_ids: ids,
            ..fixture::board(board, board)
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
    let (engine, _, doc) = open_engine(doc);
    (engine, doc)
}

fn anchored_rich_fixture() -> (CoreEngine, ProjectDoc) {
    let (mut engine, doc) = rich_fixture();
    let doc = add_thumb(&mut engine, &doc);
    (engine, doc)
}

fn set_follow(
    engine: &mut CoreEngine,
    doc: &ProjectDoc,
    x: bool,
    y: bool,
    rotation: bool,
) -> ProjectDoc {
    commit_edit(
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
