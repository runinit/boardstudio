use boardstudio_core::CoreEngine;
use boardstudio_core::model::*;

#[path = "core/core_chunk0.rs"]
mod core_chunk0;
#[path = "core/core_chunk1.rs"]
mod core_chunk1;
#[path = "core/core_chunk2.rs"]
mod core_chunk2;
#[path = "core/core_chunk3.rs"]
mod core_chunk3;
fn rect(id: &str, x: f64, y: f64, width: f64, height: f64, operation: Operation) -> OutlineFeature {
    OutlineFeature::Rect {
        rotation: None,
        anchor_part_id: None,
        id: id.into(),
        center: Vec2 { x, y },
        size: Vec2 {
            x: width,
            y: height,
        },
        radius: 0.0,
        operation,
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
fn preview(reply: CoreReply) -> SceneDelta {
    match reply {
        CoreReply::Preview { scene, .. } => scene,
        other => panic!("expected preview: {other:?}"),
    }
}
fn edit(base: u64, phase: EditPhase, op: EditOperation) -> CoreRequest {
    CoreRequest::Edit {
        id: "request".into(),
        command: EditCommand {
            base_revision: base,
            transaction_id: "drag-1".into(),
            phase,
            target_ids: vec![],
            operation: op,
        },
    }
}

fn matrix_doc() -> ProjectDoc {
    let mut doc = ProjectDoc::empty("p", "Project");
    doc.definitions.push(PartDefinition {
        hardware_profile: None,
        input_profile: None,
        mechanical_profile: None,
        id: "switch".into(),
        name: "Switch".into(),
        kind: PartKind::Switch,
        courtyard: vec![
            Vec2 { x: -7.0, y: -7.0 },
            Vec2 { x: 7.0, y: -7.0 },
            Vec2 { x: 7.0, y: 7.0 },
            Vec2 { x: -7.0, y: 7.0 },
        ],
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
    doc.outline.push(OutlineFeature::PartEnvelope {
        connections: vec![],
        id: "edge".into(),
        part_ids: vec![],
        settings: Default::default(),
        margin: 3.0,
        operation: Operation::Add,
    });
    doc.boards.push(Board {
        id: "board".into(),
        name: "Board".into(),
        outline_ids: vec!["edge".into()],
        part_ids: vec![],
        net_ids: vec![],
        thickness: 1.6,
        traces: vec![],
        vias: vec![],
    });
    doc
}

fn mirrored_pair_request(base: u64, phase: &str) -> String {
    serde_json::json!({
        "kind": "edit", "id": "pair",
        "command": {"baseRevision": base, "transactionId": "pair", "phase": phase, "targetIds": [],
            "operation": {"kind": "create-mirrored-pair",
                "matrix": {"id": "left-matrix", "rows": 2, "columns": 3, "pitch": {"x": 19, "y": 19},
                    "origin": {"x": -20, "y": 5}, "mirror": "x", "definitionId": "switch", "boardId": "board", "partIds": []},
                "left": {"id": "left-layout", "name": "Left half", "boardId": "board", "matrixIds": ["left-matrix"], "partIds": []},
                "right": {"id": "right-layout", "name": "Right half", "boardId": "board", "matrixIds": ["right-matrix"], "partIds": [],
                    "mirrorLink": {"sourceId": "left-layout", "axisX": 10}}
            }
        }
    }).to_string()
}

fn public_native_pair() -> (CoreEngine, ProjectDoc) {
    let mut engine = CoreEngine::new();
    let fixture = super::support::synthetic_half::document();
    engine.handle(CoreRequest::Open {
        id: "open".into(),
        document: serde_json::from_value(fixture).unwrap(),
    });
    let pair_response = engine.request(&serde_json::json!({
        "kind": "edit", "id": "pair", "command": {
            "baseRevision": 0, "transactionId": "pair", "phase": "commit", "targetIds": [],
            "operation": {"kind": "create-mirrored-half", "pairId": "pair", "sourceInstanceId": "left",
                "mode": {"kind": "new-board", "boardId": "mirror", "instanceId": "mirror"},
                "mapping": {"at": {"x": 200.0, "y": 0.0}, "rotation": 0.0},
                "idRule": "board-prefix", "mountingHole": null}
        }
    }).to_string());
    let (_, document) = scene(serde_json::from_str(&pair_response).unwrap());
    (engine, document)
}

fn matrixless_native_pair_with_omitted_part() -> (CoreEngine, ProjectDoc) {
    let mut document = ProjectDoc::empty("matrixless", "Matrixless native pair");
    let mut previous_definition = matrix_doc().definitions.remove(0);
    previous_definition.id = "catalog:previous-part".into();
    previous_definition.name = "Previous part".into();
    previous_definition.kind = PartKind::Passive;
    document.definitions.push(previous_definition.clone());
    document.parts.push(Part {
        anchor: None,
        id: "part-previous".into(),
        definition_id: previous_definition.id,
        reference: "J0".into(),
        pose: Pose2 {
            at: Vec2 { x: 0.0, y: 0.0 },
            rotation: 0.0,
        },
        side: Side::Front,
        keycap: None,
        outline: None,
        locked: None,
        properties: None,
        generator_parameters: None,
    });
    document.boards = vec![
        Board {
            id: "board-main".into(),
            name: "Main".into(),
            outline_ids: vec![],
            part_ids: vec!["part-previous".into()],
            net_ids: vec![],
            thickness: 1.6,
            traces: vec![],
            vias: vec![],
        },
        Board {
            id: "board-mirror".into(),
            name: "Mirror".into(),
            outline_ids: vec![],
            part_ids: vec![],
            net_ids: vec![],
            thickness: 1.6,
            traces: vec![],
            vias: vec![],
        },
    ];
    document.hardware = Some(
        serde_json::from_value(serde_json::json!({
            "topology": "split",
            "transport": "wired",
            "instances": [
                {"id":"instance-main", "name":"Main", "boardId":"board-main",
                 "half":"left", "role":"central", "flipped":false,
                 "controllerPartId":null, "mechanical":null},
                {"id":"instance-mirror", "name":"Mirror", "boardId":"board-mirror",
                 "half":"right", "role":"central", "flipped":false,
                 "controllerPartId":null, "mechanical":null}
            ],
            "boards": [],
            "pairs": [{
                "id":"pair-main-mirror", "members":["instance-main", "instance-mirror"],
                "constructionSource":"instance-main",
                "mapping":{"at":{"x":200.0,"y":0.0},"rotation":90.0},
                "construction":{
                    "method":"printed", "mount":"rigid", "integratedPlateFrame":false,
                    "plateThickness":1.5, "plateFoamThickness":0.0, "bottomFoamThickness":0.0,
                    "bottomThickness":2.0, "plateToPcb":3.5, "wallThickness":2.0, "clearance":0.2
                },
                "features":[],
                "halves":{
                    "sourceBoardId":"board-main", "mirrorBoardId":"board-mirror",
                    "correspondences":[{"kind":"part","sourceId":"part-previous","mirrorId":"part-previous-mirror"}],
                    "exceptions":[{"kind":"part","mirrorId":"part-previous-mirror","state":"omitted"}]
                }
            }]
        }))
        .unwrap(),
    );
    let mut engine = CoreEngine::new();
    let (_, opened) = scene(engine.handle(CoreRequest::Open {
        id: "open-matrixless-pair".into(),
        document,
    }));
    (engine, opened)
}

fn linked_pair() -> (CoreEngine, ProjectDoc) {
    let mut engine = CoreEngine::new();
    engine.handle(CoreRequest::Open {
        id: "open".into(),
        document: matrix_doc(),
    });
    let response = engine.request(&mirrored_pair_request(0, "commit"));
    let (_, doc) = scene(serde_json::from_str(&response).unwrap());
    (engine, doc)
}

fn assert_reflected_keys(doc: &ProjectDoc) {
    for left in doc.parts.iter().filter(|part| {
        part.id.starts_with("matrix/left-matrix/") && part.id.matches('/').count() == 2
    }) {
        let right = doc
            .parts
            .iter()
            .find(|part| part.id == left.id.replace("left-matrix", "right-matrix"))
            .unwrap();
        assert!(
            (left.pose.at.x + right.pose.at.x - 20.0).abs() < 1e-8,
            "{left:?} {right:?}"
        );
        assert!((left.pose.at.y - right.pose.at.y).abs() < 1e-8);
        assert!((left.pose.rotation + right.pose.rotation).abs() < 1e-8);
    }
}

/// Board-space pose of a part from the scene transforms.
fn board_pose(scene: &SceneDelta, id: &str) -> Pose2 {
    scene
        .transforms
        .iter()
        .find(|transform| transform.id == id)
        .unwrap_or_else(|| panic!("missing transform {id}"))
        .pose
}

fn matrix(rows: u32, columns: u32) -> Matrix {
    Matrix {
        id: "main".into(),
        name: None,
        rows,
        columns,
        pitch: Vec2 { x: 19.0, y: 19.0 },
        origin: Vec2 { x: 0.0, y: 0.0 },
        definition_id: "switch".into(),
        part_ids: vec![],
        board_id: None,
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
