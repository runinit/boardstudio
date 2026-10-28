//! Ticket 11: adaptive gap at Mirrored pair creation.
//! Creation stores max(requested gap, 40 mm default, minimum from the
//! ticket-10 formula), so the placed pair already shows the adapted gap.
//! All behavior runs through public Core routes (`Open` / `Edit`).
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

fn definitions() -> Vec<PartDefinition> {
    serde_json::from_value(serde_json::json!([
        {"id":"switch","name":"Switch","kind":"switch","courtyard":[],"pads":[]}
    ]))
    .unwrap()
}

fn board_doc() -> ProjectDoc {
    let mut doc = ProjectDoc::empty("created", "Created");
    doc.definitions = definitions();
    doc.boards.push(Board {
        id: "board".into(),
        name: "Board".into(),
        outline_ids: vec![],
        part_ids: vec![],
        net_ids: vec![],
        thickness: 1.6,
        traces: vec![],
        vias: vec![],
    });
    doc
}

/// Bare source matrix for creation: no member parts yet, every cell enabled.
fn matrix(id: &str, origin_x: f64, rows: u32, columns: u32) -> Matrix {
    Matrix {
        id: id.into(),
        name: None,
        rows,
        columns,
        pitch: Vec2 { x: 19.0, y: 19.0 },
        origin: Vec2 {
            x: origin_x,
            y: 0.0,
        },
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
    }
}

fn create_pair(matrix: Matrix, axis_x: f64) -> EditOperation {
    EditOperation::CreateMirroredPair {
        left: Layout {
            id: "left".into(),
            name: "Left".into(),
            board_id: "board".into(),
            matrix_ids: vec![matrix.id.clone()],
            part_ids: vec![],
            mirror_link: None,
        },
        right: Layout {
            id: "right".into(),
            name: "Right".into(),
            board_id: "board".into(),
            matrix_ids: vec!["main-mirror".into()],
            part_ids: vec![],
            mirror_link: Some(LayoutMirrorLink {
                source_id: "left".into(),
                axis_x,
                split_gap: None,
                spacing: None,
                correspondences: vec![],
                exceptions: vec![],
            }),
        },
        matrix: Box::new(matrix),
        definitions: None,
    }
}

fn link_of(doc: &ProjectDoc) -> &LayoutMirrorLink {
    doc.layouts
        .iter()
        .find(|layout| layout.id == "right")
        .expect("mirror layout")
        .mirror_link
        .as_ref()
        .expect("mirror link")
}

fn origin_of(doc: &ProjectDoc, id: &str) -> Vec2 {
    doc.matrices
        .iter()
        .find(|matrix| matrix.id == id)
        .unwrap_or_else(|| panic!("missing matrix {id}"))
        .origin
}

fn key_centers(scene: &SceneDelta, matrix_id: &str) -> Vec<f64> {
    let mut poses: Vec<f64> = scene
        .matrix_scenes
        .iter()
        .find(|item| item.matrix_id == matrix_id)
        .unwrap_or_else(|| panic!("missing matrix scene {matrix_id}"))
        .cells
        .iter()
        .map(|cell| cell.pose.at.x)
        .collect();
    poses.sort_by(|a, b| a.partial_cmp(b).unwrap());
    poses
}

#[test]
fn small_board_keeps_the_forty_millimeter_default() {
    // 1x1 at a requested 40 mm: the minimum (12 mm) stays below the default.
    let (mut engine, _, doc) = open(board_doc());
    let (_, created) = edit(
        &mut engine,
        &doc,
        create_pair(matrix("main", 71.0, 1, 1), 100.0),
    );
    assert_eq!(link_of(&created).split_gap, Some(40.0));
    assert_eq!(link_of(&created).spacing, None);
    // Half-frame origins convert with the requested placement ...
    assert_eq!(origin_of(&created, "main"), Vec2 { x: -9.0, y: 0.0 });
    assert_eq!(origin_of(&created, "main-mirror"), Vec2 { x: 9.0, y: 0.0 });
    // ... and the placed pair shows the default gap.
    let current = scene(engine.handle(CoreRequest::Snapshot {
        id: "snapshot".into(),
    }))
    .0;
    assert_eq!(key_centers(&current, "main"), vec![71.0]);
    assert_eq!(key_centers(&current, "main-mirror"), vec![129.0]);
}

#[test]
fn requested_gap_below_the_default_rises_to_forty() {
    // 1x1 at a requested 30 mm: still above the 12 mm minimum, but the
    // 40 mm default is the floor, so placement converges to case A.
    let (mut engine, _, doc) = open(board_doc());
    let (_, created) = edit(
        &mut engine,
        &doc,
        create_pair(matrix("main", 76.0, 1, 1), 100.0),
    );
    assert_eq!(link_of(&created).split_gap, Some(40.0));
    let current = scene(engine.handle(CoreRequest::Snapshot {
        id: "snapshot".into(),
    }))
    .0;
    assert_eq!(key_centers(&current, "main"), vec![71.0]);
    assert_eq!(key_centers(&current, "main-mirror"), vec![129.0]);
}

#[test]
fn wide_matrix_adapts_past_the_default_to_its_minimum() {
    // 1x3 at a requested 40 mm: the inner keys overlap (clear -36 mm
    // against a 12 mm need), so the minimum is 88 mm and wins.
    let (mut engine, _, doc) = open(board_doc());
    let (_, created) = edit(
        &mut engine,
        &doc,
        create_pair(matrix("main", 71.0, 1, 3), 100.0),
    );
    assert_eq!(link_of(&created).split_gap, Some(88.0));
    // Stored origins still convert with the requested placement ...
    assert_eq!(origin_of(&created, "main"), Vec2 { x: -9.0, y: 0.0 });
    assert_eq!(origin_of(&created, "main-mirror"), Vec2 { x: 9.0, y: 0.0 });
    // ... while the placed pair shows the adapted gap: 2 * (100 - 47) - 18.
    let current = scene(engine.handle(CoreRequest::Snapshot {
        id: "snapshot".into(),
    }))
    .0;
    assert_eq!(key_centers(&current, "main"), vec![47.0, 66.0, 85.0]);
    assert_eq!(
        key_centers(&current, "main-mirror"),
        vec![115.0, 134.0, 153.0]
    );
}

#[test]
fn requested_gap_below_the_minimum_rises_to_the_minimum() {
    // 1x2 at a requested 24 mm: the minimum is 50 mm, above the default.
    let (mut engine, _, doc) = open(board_doc());
    let (_, created) = edit(
        &mut engine,
        &doc,
        create_pair(matrix("main", 79.0, 1, 2), 100.0),
    );
    assert_eq!(link_of(&created).split_gap, Some(50.0));
    let current = scene(engine.handle(CoreRequest::Snapshot {
        id: "snapshot".into(),
    }))
    .0;
    assert_eq!(key_centers(&current, "main"), vec![66.0, 85.0]);
    assert_eq!(key_centers(&current, "main-mirror"), vec![115.0, 134.0]);
}

#[test]
fn explicit_wider_request_is_kept() {
    // 1x1 at a requested 60 mm: above both the default and the minimum.
    let (mut engine, _, doc) = open(board_doc());
    let (_, created) = edit(
        &mut engine,
        &doc,
        create_pair(matrix("main", 61.0, 1, 1), 100.0),
    );
    assert_eq!(link_of(&created).split_gap, Some(60.0));
    let current = scene(engine.handle(CoreRequest::Snapshot {
        id: "snapshot".into(),
    }))
    .0;
    assert_eq!(key_centers(&current, "main"), vec![61.0]);
    assert_eq!(key_centers(&current, "main-mirror"), vec![139.0]);
}
