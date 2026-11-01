//! Ticket 10: spacing adapts on every committed edit.
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

fn matrix_1x1(id: &str, origin_x: f64, mirror: Option<Mirror>) -> Matrix {
    Matrix {
        id: id.into(),
        name: None,
        rows: 1,
        columns: 1,
        pitch: Vec2 { x: 19.0, y: 19.0 },
        origin: Vec2 {
            x: origin_x,
            y: 0.0,
        },
        definition_id: "switch".into(),
        part_ids: vec![member(id, 0, 0)],
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

/// Pre-10 saved board: stored Split gap, no current spacing, key envelopes
/// 2 mm apart edge to edge (below the 12 mm minimum at the default margin).
fn gapped_doc() -> ProjectDoc {
    let mut doc = ProjectDoc::empty("spacing", "Spacing");
    doc.definitions = definitions();
    doc.boards.push(Board {
        id: "board".into(),
        name: "Board".into(),
        outline_ids: vec![],
        part_ids: vec![member("main", 0, 0), member("main-mirror", 0, 0)],
        net_ids: vec![],
        thickness: 1.6,
        traces: vec![],
        vias: vec![],
    });
    // Half-frame origins for a 40 mm stored gap about axis 40: board 11 and 69.
    doc.parts.push(key_part(&member("main", 0, 0), 11.0, 0.0));
    doc.parts
        .push(key_part(&member("main-mirror", 0, 0), 69.0, 0.0));
    doc.matrices.push(matrix_1x1("main", -9.0, None));
    doc.matrices
        .push(matrix_1x1("main-mirror", 9.0, Some(Mirror::X)));
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
            split_gap: Some(40.0),
            spacing: None,
            correspondences: vec![mirror_kind("main", "main-mirror")],
            exceptions: vec![],
        }),
    });
    doc
}

/// Legacy saved board: board-coordinate origins, no stored gap, envelopes
/// touching edge to edge (implied gap 40, minimum 50).
fn legacy_touching_doc() -> ProjectDoc {
    let mut doc = ProjectDoc::empty("legacy-touch", "Legacy touch");
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
    // 1x2 matrices: source keys at board 11 and 30, mirror at 50 and 69.
    // Implied key-edge gap at the origin column: 2 * (40 - 11) - 18 = 40.
    doc.parts.push(key_part(&member("main", 0, 0), 11.0, 0.0));
    doc.parts.push(key_part(&member("main", 0, 1), 30.0, 0.0));
    doc.parts
        .push(key_part(&member("main-mirror", 0, 0), 69.0, 0.0));
    doc.parts
        .push(key_part(&member("main-mirror", 0, 1), 50.0, 0.0));
    let mut source = matrix_1x1("main", 11.0, None);
    source.columns = 2;
    source.part_ids = vec![member("main", 0, 0), member("main", 0, 1)];
    let mut mirror = matrix_1x1("main-mirror", 69.0, Some(Mirror::X));
    mirror.columns = 2;
    mirror.part_ids = vec![member("main-mirror", 0, 0), member("main-mirror", 0, 1)];
    doc.matrices.push(source);
    doc.matrices.push(mirror);
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
            spacing: None,
            correspondences: vec![mirror_kind("main", "main-mirror")],
            exceptions: vec![],
        }),
    });
    doc
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

fn link_of(doc: &ProjectDoc) -> LayoutMirrorLink {
    doc.layouts
        .iter()
        .find(|layout| layout.id == "right")
        .expect("mirror layout")
        .mirror_link
        .clone()
        .expect("mirror link")
}

fn grow_inner_column(doc: &ProjectDoc) -> Matrix {
    let mut grown = doc
        .matrices
        .iter()
        .find(|matrix| matrix.id == "main")
        .expect("source matrix")
        .clone();
    grown.columns = 2;
    grown.part_ids = vec![member("main", 0, 0), member("main", 0, 1)];
    grown
}

#[test]
fn opening_a_saved_board_changes_nothing_until_its_first_edit() {
    let (_, scene, doc) = open(gapped_doc());
    // No migration on open: no current spacing is stored.
    assert_eq!(link_of(&doc).spacing, None);
    assert_eq!(link_of(&doc).split_gap, Some(40.0));
    // Evaluated geometry still follows the stored 40 mm gap.
    assert_eq!(key_poses(&scene, "main"), vec![Vec2 { x: 11.0, y: 0.0 }]);
    assert_eq!(
        key_poses(&scene, "main-mirror"),
        vec![Vec2 { x: 69.0, y: 0.0 }]
    );
}

#[test]
fn growing_an_inner_column_widens_to_the_minimum_in_one_undo() {
    let (mut engine, _, doc) = open(gapped_doc());
    // 1x2 envelopes are 2 mm apart edge to edge; the minimum at the default
    // 3 mm margin is 2 + (12 - 2) = 50 mm, symmetric about the axis.
    let (grown_scene, grown) = edit(
        &mut engine,
        &doc,
        EditOperation::SetMatrix {
            matrix: grow_inner_column(&doc),
            definitions: None,
        },
    );
    assert_eq!(link_of(&grown).split_gap, Some(40.0));
    assert_eq!(link_of(&grown).spacing, Some(50.0));
    assert_eq!(
        key_poses(&grown_scene, "main"),
        vec![Vec2 { x: 6.0, y: 0.0 }, Vec2 { x: 25.0, y: 0.0 }]
    );
    assert_eq!(
        key_poses(&grown_scene, "main-mirror"),
        vec![Vec2 { x: 55.0, y: 0.0 }, Vec2 { x: 74.0, y: 0.0 }]
    );
    // Outlines stay 2 x margin apart: air 12 - 6 = 6 mm, required 6 mm.
    let max_left = 25.0 + 9.0;
    let min_right = 55.0 - 9.0;
    assert_eq!(min_right - max_left, 12.0);
    // One Undo entry restores the edit and the spacing together.
    let restored = undone(&mut engine);
    assert_eq!(link_of(&restored).split_gap, Some(40.0));
    assert_eq!(link_of(&restored).spacing, None);
    let current = scene(engine.handle(CoreRequest::Snapshot {
        id: "snapshot".into(),
    }))
    .0;
    assert_eq!(key_poses(&current, "main"), vec![Vec2 { x: 11.0, y: 0.0 }]);
}

#[test]
fn removing_the_column_narrows_back_to_the_stored_gap() {
    let (mut engine, _, doc) = open(gapped_doc());
    let (_, grown) = edit(
        &mut engine,
        &doc,
        EditOperation::SetMatrix {
            matrix: grow_inner_column(&doc),
            definitions: None,
        },
    );
    assert_eq!(link_of(&grown).spacing, Some(50.0));
    let (shrunk_scene, shrunk) = edit(
        &mut engine,
        &grown,
        EditOperation::RemoveMatrixColumn {
            matrix_id: "main".into(),
            column: 1,
        },
    );
    // The minimum drops back to 12 mm, so spacing returns to the stored gap.
    assert_eq!(link_of(&shrunk).split_gap, Some(40.0));
    assert_eq!(link_of(&shrunk).spacing, Some(40.0));
    assert_eq!(
        key_poses(&shrunk_scene, "main"),
        vec![Vec2 { x: 11.0, y: 0.0 }]
    );
    assert_eq!(
        key_poses(&shrunk_scene, "main-mirror"),
        vec![Vec2 { x: 69.0, y: 0.0 }]
    );
}

#[test]
fn moving_toward_the_axis_widens_and_holds_symmetry() {
    let (mut engine, _, doc) = open(gapped_doc());
    // +15 mm toward the axis: the minimum becomes 42 mm, so spacing follows.
    let (moved_scene, moved) = edit(
        &mut engine,
        &doc,
        EditOperation::SetMatrixPlacement {
            matrix_id: "main".into(),
            change: MatrixPlacementChange::MatrixTranslation {
                delta: Vec2 { x: 15.0, y: 0.0 },
            },
        },
    );
    assert_eq!(link_of(&moved).split_gap, Some(40.0));
    assert_eq!(link_of(&moved).spacing, Some(42.0));
    assert_eq!(
        key_poses(&moved_scene, "main"),
        vec![Vec2 { x: 25.0, y: 0.0 }]
    );
    assert_eq!(
        key_poses(&moved_scene, "main-mirror"),
        vec![Vec2 { x: 55.0, y: 0.0 }]
    );
}

#[test]
fn typing_a_gap_below_the_minimum_keeps_the_stored_value() {
    let (mut engine, _, doc) = open(gapped_doc());
    let (_, grown) = edit(
        &mut engine,
        &doc,
        EditOperation::SetMatrix {
            matrix: grow_inner_column(&doc),
            definitions: None,
        },
    );
    assert_eq!(link_of(&grown).spacing, Some(50.0));
    // Typing 30 mm keeps the stored value; spacing stays at the 50 mm minimum.
    let (typed_scene, typed) = edit(
        &mut engine,
        &grown,
        EditOperation::SetSplitGap {
            layout_id: "right".into(),
            split_gap: 30.0,
        },
    );
    assert_eq!(link_of(&typed).split_gap, Some(30.0));
    assert_eq!(link_of(&typed).spacing, Some(50.0));
    assert_eq!(
        key_poses(&typed_scene, "main"),
        vec![Vec2 { x: 6.0, y: 0.0 }, Vec2 { x: 25.0, y: 0.0 }]
    );
    // One Undo entry restores the typed gap and the spacing together.
    let restored = undone(&mut engine);
    assert_eq!(link_of(&restored).split_gap, Some(40.0));
    assert_eq!(link_of(&restored).spacing, Some(50.0));
}

#[test]
fn first_edit_on_a_touching_legacy_board_adopts_and_widens() {
    let (mut engine, scene, doc) = open(legacy_touching_doc());
    // No migration on open: touching geometry is left alone.
    assert_eq!(link_of(&doc).split_gap, None);
    assert_eq!(link_of(&doc).spacing, None);
    assert_eq!(
        key_poses(&scene, "main"),
        vec![Vec2 { x: 11.0, y: 0.0 }, Vec2 { x: 30.0, y: 0.0 }]
    );
    // The +1 mm move adopts the implied 38 mm gap and widens to the 50 mm
    // minimum in the same Undo entry.
    let (moved_scene, moved) = edit(
        &mut engine,
        &doc,
        EditOperation::SetMatrixPlacement {
            matrix_id: "main".into(),
            change: MatrixPlacementChange::MatrixTranslation {
                delta: Vec2 { x: 1.0, y: 0.0 },
            },
        },
    );
    assert_eq!(link_of(&moved).split_gap, Some(38.0));
    assert_eq!(link_of(&moved).spacing, Some(50.0));
    assert_eq!(
        key_poses(&moved_scene, "main"),
        vec![Vec2 { x: 6.0, y: 0.0 }, Vec2 { x: 25.0, y: 0.0 }]
    );
    assert_eq!(
        key_poses(&moved_scene, "main-mirror"),
        vec![Vec2 { x: 55.0, y: 0.0 }, Vec2 { x: 74.0, y: 0.0 }]
    );
    let restored = undone(&mut engine);
    assert_eq!(link_of(&restored).split_gap, None);
    assert_eq!(link_of(&restored).spacing, None);
}
