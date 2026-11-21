//! Ticket 19: Core-owned single-key anchor evaluation and typed Add/change/detach
//! edits. All behavior runs through public Core routes (`Open` / `Edit`), never
//! graph helpers alone.
use boardstudio_core::{CoreEngine, model::*};

#[path = "matrix_anchors/anchor_chunk0.rs"]
mod anchor_chunk0;
#[path = "matrix_anchors/anchor_chunk1.rs"]
mod anchor_chunk1;
#[path = "matrix_anchors/anchor_chunk2.rs"]
mod anchor_chunk2;
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
        anchor: None,
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
