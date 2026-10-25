//! Ticket 09: per-half frame and stored Split gap (saved-document contract).
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

fn undone(engine: &mut CoreEngine) -> ProjectDoc {
    match engine.handle(CoreRequest::Undo { id: "undo".into() }) {
        CoreReply::Scene { document, .. } => *document,
        other => panic!("expected scene: {other:?}"),
    }
}

fn definitions() -> Vec<PartDefinition> {
    serde_json::from_value(serde_json::json!([
        {"id":"switch","name":"Switch","kind":"switch","courtyard":[],"pads":[]}
    ]))
    .unwrap()
}

fn member(matrix: &str, row: u32, column: u32) -> String {
    format!("matrix/{matrix}/r{row}c{column}")
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

fn matrix(id: &str, origin_x: f64, mirror: Option<Mirror>) -> Matrix {
    Matrix {
        id: id.into(),
        name: None,
        rows: 1,
        columns: 2,
        pitch: Vec2 { x: 19.0, y: 19.0 },
        origin: Vec2 {
            x: origin_x,
            y: 0.0,
        },
        definition_id: "switch".into(),
        part_ids: vec![member(id, 0, 0), member(id, 0, 1)],
        board_id: Some("board".into()),
        mirror,
        rotation: None,
        edge_gap: None,
        diode_direction: None,
        row_layout: vec![],
        column_layout: vec![],
        anchor: None,
        cells: vec![],
    }
}

fn mirror_kind(source_id: &str, mirror_id: &str) -> MirroredCorrespondence {
    MirroredCorrespondence {
        kind: MirroredItemKind::Matrix,
        source_id: source_id.into(),
        mirror_id: mirror_id.into(),
        companion_mode: None,
        companion_tweak: None,
    }
}

/// Legacy saved document: board-coordinate origins, no stored gap.
fn legacy_doc() -> ProjectDoc {
    let mut doc = ProjectDoc::empty("half-frame", "Half frame");
    doc.definitions = definitions();
    doc.boards.push(Board {
        id: "board".into(),
        name: "Board".into(),
        outline_ids: vec![],
        part_ids: vec![
            member("main", 0, 0),
            member("main", 0, 1),
            member("main-mirror", 0, 0),
            member("main-mirror", 0, 1),
        ],
        net_ids: vec![],
        thickness: 1.6,
        traces: vec![],
        vias: vec![],
    });
    doc.parts.push(key_part(&member("main", 0, 0), 0.0, 0.0));
    doc.parts.push(key_part(&member("main", 0, 1), 19.0, 0.0));
    doc.parts
        .push(key_part(&member("main-mirror", 0, 0), 80.0, 0.0));
    doc.parts
        .push(key_part(&member("main-mirror", 0, 1), 61.0, 0.0));
    doc.matrices.push(matrix("main", 0.0, None));
    doc.matrices
        .push(matrix("main-mirror", 80.0, Some(Mirror::X)));
    doc.layouts.push(Layout {
        id: "left".into(),
        name: "Left".into(),
        board_id: "board".into(),
        matrix_ids: vec!["main".into()],
        part_ids: vec![],
        mirror_link: None,
    });
    doc.layouts.push(Layout {
        id: "right".into(),
        name: "Right".into(),
        board_id: "board".into(),
        matrix_ids: vec!["main-mirror".into()],
        part_ids: vec![],
        mirror_link: Some(LayoutMirrorLink {
            source_id: "left".into(),
            axis_x: 40.0,
            split_gap: None,
            correspondences: vec![mirror_kind("main", "main-mirror")],
            exceptions: vec![],
        }),
    });
    doc
}

fn part_poses(doc: &ProjectDoc, matrix_id: &str) -> Vec<Vec2> {
    let mut poses: Vec<Vec2> = doc
        .parts
        .iter()
        .filter(|part| part.id == member(matrix_id, 0, 0) || part.id == member(matrix_id, 0, 1))
        .map(|part| part.pose.at)
        .collect();
    poses.sort_by(|a, b| a.x.partial_cmp(&b.x).unwrap());
    poses
}

fn key_poses(scene: &SceneDelta, matrix_id: &str) -> Vec<Vec2> {
    let mut poses: Vec<Vec2> = scene
        .matrix_scenes
        .iter()
        .find(|item| item.matrix_id == matrix_id)
        .unwrap_or_else(|| panic!("missing matrix scene {matrix_id}"))
        .cells
        .iter()
        .map(|cell| cell.pose.at)
        .collect();
    poses.sort_by(|a, b| a.x.partial_cmp(&b.x).unwrap());
    poses
}

fn origin_of(doc: &ProjectDoc, id: &str) -> Vec2 {
    doc.matrices
        .iter()
        .find(|matrix| matrix.id == id)
        .unwrap_or_else(|| panic!("missing matrix {id}"))
        .origin
}

fn gap_of(doc: &ProjectDoc) -> Option<f64> {
    doc.layouts
        .iter()
        .find(|layout| layout.id == "right")
        .expect("mirror layout")
        .mirror_link
        .as_ref()
        .expect("mirror link")
        .split_gap
}

#[test]
fn legacy_board_opens_with_identical_geometry_and_no_stored_gap() {
    let (_, scene, doc) = open(legacy_doc());
    // No migration on open: the link carries no stored gap.
    assert_eq!(gap_of(&doc), None);
    // Byte-identical evaluated geometry: hand-computed board positions.
    assert_eq!(
        key_poses(&scene, "main"),
        vec![Vec2 { x: 0.0, y: 0.0 }, Vec2 { x: 19.0, y: 0.0 }]
    );
    assert_eq!(
        key_poses(&scene, "main-mirror"),
        vec![Vec2 { x: 61.0, y: 0.0 }, Vec2 { x: 80.0, y: 0.0 }]
    );
}

#[test]
fn first_edit_adopts_half_frame_without_moving_anything() {
    let (mut engine, _, doc) = open(legacy_doc());
    let (_, moved) = edit(
        &mut engine,
        &doc,
        EditOperation::SetMatrixPlacement {
            matrix_id: "main".into(),
            change: MatrixPlacementChange::MatrixTranslation {
                delta: Vec2 { x: 1.0, y: 0.0 },
            },
        },
    );
    // The stored gap is adopted from the moved geometry: 2 * (40 - 1) - 18 = 60.
    assert_eq!(gap_of(&moved), Some(60.0));
    // The edit's world effect is exactly the requested move; adoption moves nothing.
    let current = scene(engine.handle(CoreRequest::Snapshot {
        id: "snapshot".into(),
    }))
    .0;
    assert_eq!(
        key_poses(&current, "main"),
        vec![Vec2 { x: 1.0, y: 0.0 }, Vec2 { x: 20.0, y: 0.0 }]
    );
    assert_eq!(
        key_poses(&current, "main-mirror"),
        vec![Vec2 { x: 60.0, y: 0.0 }, Vec2 { x: 79.0, y: 0.0 }]
    );
    // Stored origins are half-frame values now: source -9, mirror +9.
    assert_eq!(origin_of(&moved, "main"), Vec2 { x: -9.0, y: 0.0 });
    assert_eq!(origin_of(&moved, "main-mirror"), Vec2 { x: 9.0, y: 0.0 });
}

#[test]
fn changing_split_gap_moves_halves_symmetrically_in_one_undo() {
    let (mut engine, _, doc) = open(legacy_doc());
    let (_, moved) = edit(
        &mut engine,
        &doc,
        EditOperation::SetMatrixPlacement {
            matrix_id: "main".into(),
            change: MatrixPlacementChange::MatrixTranslation {
                delta: Vec2 { x: 1.0, y: 0.0 },
            },
        },
    );
    let source_before = origin_of(&moved, "main");
    let mirror_before = origin_of(&moved, "main-mirror");
    let (_, gapped) = edit(
        &mut engine,
        &moved,
        EditOperation::SetSplitGap {
            layout_id: "right".into(),
            split_gap: 70.0,
        },
    );
    assert_eq!(gap_of(&gapped), Some(70.0));
    // Half-frame values are untouched by the gap change.
    assert_eq!(origin_of(&gapped, "main"), source_before);
    assert_eq!(origin_of(&gapped, "main-mirror"), mirror_before);
    // Both halves moved apart symmetrically: exactly 5 mm each about the axis.
    let current = scene(engine.handle(CoreRequest::Snapshot {
        id: "snapshot".into(),
    }))
    .0;
    assert_eq!(
        key_poses(&current, "main"),
        vec![Vec2 { x: -4.0, y: 0.0 }, Vec2 { x: 15.0, y: 0.0 }]
    );
    assert_eq!(
        key_poses(&current, "main-mirror"),
        vec![Vec2 { x: 65.0, y: 0.0 }, Vec2 { x: 84.0, y: 0.0 }]
    );
    // The partner still reflects exactly about the split axis.
    for (left, right) in key_poses(&current, "main")
        .iter()
        .zip(key_poses(&current, "main-mirror").iter().rev())
    {
        assert_eq!(left.x + right.x, 80.0);
        assert_eq!(left.y, right.y);
    }
    // Member part poses follow their halves through the refresh.
    let gapped = scene(engine.handle(CoreRequest::Snapshot {
        id: "snapshot".into(),
    }))
    .1;
    assert_eq!(
        part_poses(&gapped, "main"),
        vec![Vec2 { x: -4.0, y: 0.0 }, Vec2 { x: 15.0, y: 0.0 }]
    );
    assert_eq!(
        part_poses(&gapped, "main-mirror"),
        vec![Vec2 { x: 65.0, y: 0.0 }, Vec2 { x: 84.0, y: 0.0 }]
    );
    // One Undo entry restores the gap and the geometry together.
    let restored = undone(&mut engine);
    assert_eq!(gap_of(&restored), Some(60.0));
    assert_eq!(origin_of(&restored, "main"), source_before);
    let current = scene(engine.handle(CoreRequest::Snapshot {
        id: "snapshot".into(),
    }))
    .0;
    assert_eq!(
        key_poses(&current, "main"),
        vec![Vec2 { x: 1.0, y: 0.0 }, Vec2 { x: 20.0, y: 0.0 }]
    );
}

#[test]
fn split_gap_change_is_validated() {
    let (mut engine, _, doc) = open(legacy_doc());
    let (_, moved) = edit(
        &mut engine,
        &doc,
        EditOperation::SetMatrixPlacement {
            matrix_id: "main".into(),
            change: MatrixPlacementChange::MatrixTranslation {
                delta: Vec2 { x: 1.0, y: 0.0 },
            },
        },
    );
    assert!(
        error(
            &mut engine,
            &moved,
            EditOperation::SetSplitGap {
                layout_id: "right".into(),
                split_gap: -1.0,
            }
        )
        .contains("gap")
    );
    assert!(
        error(
            &mut engine,
            &moved,
            EditOperation::SetSplitGap {
                layout_id: "missing".into(),
                split_gap: 70.0,
            }
        )
        .contains("mirror")
    );
}

#[test]
fn created_pair_stores_gap_with_half_frame_origins() {
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
    let (mut engine, _, doc) = open(doc);
    // Creation takes a bare matrix; members materialize through the pair.
    let mut fresh = matrix("main", 16.0, None);
    fresh.part_ids.clear();
    let (_, created) = edit(
        &mut engine,
        &doc,
        EditOperation::CreateMirroredPair {
            left: Layout {
                id: "left".into(),
                name: "Left".into(),
                board_id: "board".into(),
                matrix_ids: vec!["main".into()],
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
                    axis_x: 40.0,
                    split_gap: None,
                    correspondences: vec![],
                    exceptions: vec![],
                }),
            },
            matrix: Box::new(fresh),
            definitions: None,
        },
    );
    // Requested board placement was center 40, inset 24: gap 2 * (40 - 16) - 18 = 30.
    assert_eq!(gap_of(&created), Some(30.0));
    assert_eq!(origin_of(&created, "main"), Vec2 { x: -9.0, y: 0.0 });
    assert_eq!(origin_of(&created, "main-mirror"), Vec2 { x: 9.0, y: 0.0 });
    let current = scene(engine.handle(CoreRequest::Snapshot {
        id: "snapshot".into(),
    }))
    .0;
    assert_eq!(
        key_poses(&current, "main"),
        vec![Vec2 { x: 16.0, y: 0.0 }, Vec2 { x: 35.0, y: 0.0 }]
    );
    assert_eq!(
        key_poses(&current, "main-mirror"),
        vec![Vec2 { x: 45.0, y: 0.0 }, Vec2 { x: 64.0, y: 0.0 }]
    );
}
