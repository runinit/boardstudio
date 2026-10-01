use boardstudio_core::{CoreEngine, model::*};

fn matrix() -> Matrix {
    Matrix {
        id: "m".into(),
        name: None,
        rows: 1,
        columns: 3,
        pitch: Vec2 { x: 20.0, y: 20.0 },
        origin: Vec2 { x: 0.0, y: 0.0 },
        definition_id: "switch".into(),
        part_ids: vec![],
        board_id: None,
        mirror: None,
        rotation: None,
        edge_gap: None,

        diode_direction: None,
        row_offsets: vec![],
        column_offsets: vec![],
        column_staggers: vec![],
        column_splays: vec![0.0, 0.0, 0.0],
        column_origins: vec![],
        cells: vec![],
    }
}

fn open() -> CoreEngine {
    let mut doc = ProjectDoc::empty("p", "P");
    doc.definitions.push(PartDefinition {
        mechanical_profile: None,
        id: "switch".into(),
        name: "Switch".into(),
        kind: PartKind::Switch,
        courtyard: vec![],
        pads: vec![],
        models: None,
        keycap: None,
        envelope_source: None,
        kicad_source: None,
        terminals: Default::default(),
        matrix_terminals: None,
        envelope_notice: None,
        generator: None,
    });
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
    let mut engine = CoreEngine::new();
    scene(engine.handle(CoreRequest::Open {
        id: "open".into(),
        document: doc,
    }));
    let mut source = matrix();
    source.board_id = Some("board".into());
    let request = serde_json::json!({"kind":"edit","id":"pair","command":{
    "baseRevision":0,"transactionId":"pair","phase":"commit","targetIds":[],
    "operation":{"kind":"create-mirrored-pair","matrix":source,
        "left":{"id":"left-layout","name":"Left","boardId":"board","matrixId":"m","partIds":[]},
        "right":{"id":"right-layout","name":"Right","boardId":"board","matrixId":"right","partIds":[],"mirrorLink":{"sourceId":"left-layout","axisX":0}}
    }}});
    let (_, mut paired) =
        scene(serde_json::from_str(&engine.request(&request.to_string())).unwrap());
    paired.revision = 0;
    scene(engine.handle(CoreRequest::Open {
        id: "reopen".into(),
        document: paired,
    }));
    engine
}

fn edit(base: u64, phase: EditPhase, change: MatrixSplayChange) -> CoreRequest {
    CoreRequest::Edit {
        id: "e".into(),
        command: EditCommand {
            base_revision: base,
            transaction_id: "t".into(),
            phase,
            target_ids: vec![],
            operation: EditOperation::SetMatrixSplay {
                matrix_id: "m".into(),
                column: 1,
                change,
            },
        },
    }
}

fn scene(reply: CoreReply) -> (SceneDelta, ProjectDoc) {
    match reply {
        CoreReply::Scene {
            scene, document, ..
        } => (scene, *document),
        other => panic!("expected scene: {other:?}"),
    }
}

#[test]
fn preview_and_commit_have_equal_matrix_projection_and_history_roundtrips() {
    let mut engine = open();
    let change = MatrixSplayChange::Angle {
        angle: 25.0,
        affect: MatrixSplayAffect::Following,
    };
    let preview = match engine.handle(edit(0, EditPhase::Preview, change.clone())) {
        CoreReply::Preview { scene, .. } => scene,
        other => panic!("{other:?}"),
    };
    let (_, snapshot) = scene(engine.handle(CoreRequest::Snapshot {
        id: "snapshot".into(),
    }));
    assert!(
        snapshot
            .matrices
            .iter()
            .all(|matrix| matrix.column_splays[1] == 0.0)
    );
    let (committed, doc) = scene(engine.handle(edit(0, EditPhase::Commit, change)));
    assert_eq!(preview.matrix_scenes, committed.matrix_scenes);
    assert_eq!(doc.matrices[0].column_splays[1], 25.0);
    let (_, undone) = scene(engine.handle(CoreRequest::Undo { id: "u".into() }));
    assert_eq!(undone.matrices[0].column_splays[1], 0.0);
    let (_, redone) = scene(engine.handle(CoreRequest::Redo { id: "r".into() }));
    assert_eq!(redone.matrices[0].column_splays[1], 25.0);
}

#[test]
fn custom_origin_and_linked_following_change_are_preserved() {
    let mut engine = open();
    let preview = match engine.handle(edit(
        0,
        EditPhase::Preview,
        MatrixSplayChange::Angle {
            angle: 12.0,
            affect: MatrixSplayAffect::Following,
        },
    )) {
        CoreReply::Preview { scene, .. } => scene,
        other => panic!("{other:?}"),
    };
    let (_, snapshot) = scene(engine.handle(CoreRequest::Snapshot { id: "s".into() }));
    assert_eq!(snapshot.revision, 0);
    assert_eq!(preview.revision, 0);
    assert!(
        snapshot
            .matrices
            .iter()
            .all(|matrix| matrix.column_splays[1] == 0.0)
    );
    let (committed, doc) = scene(engine.handle(edit(
        0,
        EditPhase::Commit,
        MatrixSplayChange::Origin {
            world: Some(Vec2 { x: 3.0, y: 4.0 }),
        },
    )));
    assert_eq!(
        doc.matrices[0].column_origins[1],
        Some(Vec2 { x: 3.0, y: 4.0 })
    );
    assert_eq!(
        doc.matrices[1].column_origins[1],
        Some(Vec2 { x: 3.0, y: 4.0 })
    );
    let left = committed
        .matrix_scenes
        .iter()
        .find(|scene| scene.matrix_id == "m")
        .unwrap();
    let right = committed
        .matrix_scenes
        .iter()
        .find(|scene| scene.matrix_id == "right")
        .unwrap();
    assert_eq!(left.cells.len(), 3);
    assert_eq!(left.cells.len(), right.cells.len());
    assert!((left.columns[1].splay_origin.x + right.columns[1].splay_origin.x).abs() < 1e-8);
    assert!((left.columns[1].splay_origin.y - right.columns[1].splay_origin.y).abs() < 1e-8);
    for (left_cell, right_cell) in left.cells.iter().zip(&right.cells) {
        assert!((left_cell.pose.at.x + right_cell.pose.at.x).abs() < 1e-8);
        assert!((left_cell.pose.at.y - right_cell.pose.at.y).abs() < 1e-8);
    }
    let (_, reset) = scene(engine.handle(edit(
        1,
        EditPhase::Commit,
        MatrixSplayChange::Origin { world: None },
    )));
    assert_eq!(reset.matrices[0].column_origins[1], None);
    let (_, undone) = scene(engine.handle(CoreRequest::Undo { id: "u".into() }));
    assert_eq!(
        undone.matrices[0].column_origins[1],
        Some(Vec2 { x: 3.0, y: 4.0 })
    );
    let (_, redone) = scene(engine.handle(CoreRequest::Redo { id: "r".into() }));
    assert_eq!(redone.matrices[0].column_origins[1], None);
}

#[test]
fn editing_the_mirrored_half_updates_both_projections() {
    let mut engine = open();
    let mut request = edit(
        0,
        EditPhase::Commit,
        MatrixSplayChange::Origin {
            world: Some(Vec2 { x: -7.0, y: -13.0 }),
        },
    );
    if let CoreRequest::Edit { command, .. } = &mut request
        && let EditOperation::SetMatrixSplay { matrix_id, .. } = &mut command.operation
    {
        *matrix_id = "right".into();
    }
    let (committed, document) = scene(engine.handle(request));
    assert!(
        document
            .matrices
            .iter()
            .all(|matrix| matrix.column_origins[1] == Some(Vec2 { x: 7.0, y: -13.0 }))
    );
    let left = &committed.matrix_scenes[0];
    let right = &committed.matrix_scenes[1];
    assert_eq!(left.columns[1].splay_origin, Vec2 { x: 7.0, y: -13.0 });
    assert_eq!(right.columns[1].splay_origin, Vec2 { x: -7.0, y: -13.0 });
    let (_, undone) = scene(engine.handle(CoreRequest::Undo { id: "undo".into() }));
    assert!(
        undone
            .matrices
            .iter()
            .all(|matrix| matrix.column_origins.is_empty())
    );
    let (redone, _) = scene(engine.handle(CoreRequest::Redo { id: "redo".into() }));
    assert_eq!(redone.matrix_scenes, committed.matrix_scenes);
}
