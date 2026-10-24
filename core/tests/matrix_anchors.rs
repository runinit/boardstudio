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
