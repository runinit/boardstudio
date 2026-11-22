use boardstudio_core::{CoreEngine, model::*};

fn matrix() -> Matrix {
    Matrix {
        id: "m".into(),
        name: None,
        rows: 1,
        columns: 3,
        pitch: Vec2 { x: 20.0, y: 20.0 },
        // Ticket 11: create at a UI-reachable requested gap (R = 100 mm, so
        // the stored gap is 100 mm with room for the splay to breathe). A
        // negative requested gap stalls the minimum search at the coincident
        // start, which the setup form never produces (gap >= 0).
        origin: Vec2 { x: -59.5, y: 0.0 },
        definition_id: "switch".into(),
        part_ids: vec![],
        board_id: None,
        mirror: None,
        rotation: None,
        edge_gap: None,

        diode_direction: None,
        row_layout: vec![],
        column_layout: vec![
            boardstudio_core::model::ColumnLayout::default(),
            boardstudio_core::model::ColumnLayout::default(),
            boardstudio_core::model::ColumnLayout::default(),
        ],
        anchor: None,
        cells: vec![],
    }
}

fn open() -> CoreEngine {
    let mut doc = ProjectDoc::empty("p", "P");
    doc.definitions.push(PartDefinition {
        hardware_profile: None,
        input_profile: None,
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
        "left":{"id":"left-layout","name":"Left","boardId":"board","matrixIds":["m"],"partIds":[]},
        "right":{"id":"right-layout","name":"Right","boardId":"board","matrixIds":["right"],"partIds":[],"mirrorLink":{"sourceId":"left-layout","axisX":0}}
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
            .all(|matrix| matrix.column_layout[1].splay == 0.0)
    );
    let (committed, doc) = scene(engine.handle(edit(0, EditPhase::Commit, change)));
    assert_eq!(preview.matrix_scenes, committed.matrix_scenes);
    assert_eq!(doc.matrices[0].column_layout[1].splay, 25.0);
    let (_, undone) = scene(engine.handle(CoreRequest::Undo { id: "u".into() }));
    assert_eq!(undone.matrices[0].column_layout[1].splay, 0.0);
    let (_, redone) = scene(engine.handle(CoreRequest::Redo { id: "r".into() }));
    assert_eq!(redone.matrices[0].column_layout[1].splay, 25.0);
}
